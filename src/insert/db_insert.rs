/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::Path,
};

/*
  Import external libraries
*/
use tantivy::Term;

/*
  Import own libraries
*/
use crate::{
    globals::{
        INSERTER_IMPORT_DIR, PRIECO_CONFIG, ROCKSDB_INDEX, TANTIVY_INDEX, TANTIVY_WRITER,
        VECTOR_CENTROPOIDS, VECTOR_DIM, WebDocument, icons,
    },
    url_to_id,
};

/*
  Constants
*/
const MAX_VECTORS_IN_VRAM: usize = 1_500_000;
const BATCH_SIZE_FOR_GPU: usize = 4_000;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    create_dir_all(INSERTER_IMPORT_DIR)?;

    // Get *results.txt
    let mut files = Vec::with_capacity(100);
    for entry in read_dir(INSERTER_IMPORT_DIR)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if let Some(s) = path.to_str() {
                if s.contains(".txt") {
                    files.push(s.to_string());
                }
            }
        }
    }
    if files.is_empty() {
        println!("{}: No files found. Sleeping...", icons::DB_INSERT,);
        return Ok(());
    }

    let mut vector_idx_buffer: HashMap<u64, Vec<f32>> = HashMap::with_capacity(1_000_000);

    // Process files
    for file_name in files {
        let file = File::open(&file_name)?;
        let reader = BufReader::new(file);
        let mut inserted = 0;

        for line in reader.lines() {
            let line = line?;

            // Create a web document with checks
            let parts: Vec<&str> = line.split("<-->").collect();
            if parts.len() != 15 {
                continue;
            }

            let vector: Vec<f32> = parts[14]
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if vector.len() != VECTOR_DIM {
                continue;
            }

            let points: Vec<f32> = parts[11]
                .replace(['[', ']'], "")
                .split(", ")
                .filter_map(|s| s.parse().ok())
                .collect();
            if points.len() != 4 {
                continue;
            }

            let url = sanitize_string(parts[0]);
            let id = url_to_id(&url);

            let doc = WebDocument {
                url: url.clone(),
                title: sanitize_string(parts[1]),
                description: sanitize_string(parts[2]),
                content: sanitize_string(parts[3]),
                favicon: sanitize_string(parts[4]),
                image: sanitize_string(parts[5]),
                keywords: sanitize_string(parts[6]),
                safe_s: if parts[7] == "true" { true } else { false },
                html: parts[8].to_string(),
                lang: parts[9].to_string(),
                loc: parts[10].to_string(),
                impressions: 0,
                clicks: 0,
                confidence: points[0],
                effort: points[1],
                qna: points[2],
                sts: points[3],
                load: parts[12].parse().unwrap_or_default(),
                date: parts[13].parse().unwrap_or(0),
                search_score: 0.0,
            };

            ROCKSDB_INDEX.put(id.to_be_bytes(), serde_json::to_vec(&doc)?)?;

            /* Tantivy */
            let doc_id_field = TANTIVY_INDEX.schema().get_field("doc_id").unwrap();
            let title_field = TANTIVY_INDEX.schema().get_field("title").unwrap();
            let description_field = TANTIVY_INDEX.schema().get_field("description").unwrap();
            let content_field = TANTIVY_INDEX.schema().get_field("content").unwrap();
            let keywords_field = TANTIVY_INDEX.schema().get_field("keywords").unwrap();
            let safe_s_field = TANTIVY_INDEX.schema().get_field("safe_s").unwrap();

            TANTIVY_WRITER
                .lock()
                .delete_term(Term::from_field_u64(doc_id_field, id)); // Ensure uniqness
            TANTIVY_WRITER.lock().add_document(tantivy::doc!(
                doc_id_field => id ,
                title_field => doc.title.clone(),
                description_field => doc.description.clone(),
                content_field => doc.content.clone(),
                keywords_field => doc.keywords.clone(),
                safe_s_field => doc.safe_s
            ))?;

            vector_idx_buffer.insert(id, vector);

            inserted += 1;
            if inserted % 1_000 == 0 {
                println!("{}: Inserted {}", icons::DB_INSERT, inserted);
            }

            if vector_idx_buffer.len() >= MAX_VECTORS_IN_VRAM {
                println!(
                    "{}: Commiting {} vectors",
                    icons::DB_INSERT,
                    vector_idx_buffer.len()
                );
                TANTIVY_WRITER.lock().commit()?;
                println!(
                    "{}: Tantivy commited {} vectors",
                    icons::DB_INSERT,
                    vector_idx_buffer.len()
                );
                vector_commit(&mut vector_idx_buffer)?;
                println!("{}: Vector idx commited", icons::DB_INSERT,);
            }
        }

        // Commit after file
        println!(
            "{}: Commiting {} vectors",
            icons::DB_INSERT,
            vector_idx_buffer.len()
        );
        TANTIVY_WRITER.lock().commit()?;
        println!(
            "{}: Tantivy commited {} vectors",
            icons::DB_INSERT,
            vector_idx_buffer.len()
        );
        vector_commit(&mut vector_idx_buffer)?;
        println!("{}: Vector idx commited", icons::DB_INSERT,);

        let _ = remove_file(&file_name);
    }

    Ok(())
}

fn vector_commit(
    vector_idx_buffer: &mut HashMap<u64, Vec<f32>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ids: Vec<u64> = vector_idx_buffer.keys().copied().collect();
    let mut bucket_data: HashMap<usize, Vec<(u64, Vec<f32>)>> =
        HashMap::with_capacity(vector_idx_buffer.len());

    println!("{}: Assigning vectors!", icons::DB_INSERT,);
    for chunk in ids.chunks(BATCH_SIZE_FOR_GPU) {
        let vectors: Vec<Vec<f32>> = chunk
            .iter()
            .map(|id| vector_idx_buffer.get(id).unwrap().clone())
            .collect();

        let bucket_ids = VECTOR_CENTROPOIDS.assign_batch(&vectors)?;

        for (i, &id) in chunk.iter().enumerate() {
            let bucket_id = bucket_ids[i];
            let vector = vector_idx_buffer.get(&id).unwrap().clone();
            bucket_data
                .entry(bucket_id)
                .or_insert_with(Vec::new)
                .push((id, vector));
        }
    }

    println!("{}: Writing chunks to disk!", icons::DB_INSERT,);
    for (bucket_id, items) in bucket_data.iter() {
        update_bucket(*bucket_id, items)?;
    }

    vector_idx_buffer.clear();

    Ok(())
}

/* Helper functions */
fn sanitize_string(s: &str) -> String {
    s.replace('"', "").replace('\'', "")
}

fn update_bucket(
    bucket_id: usize,
    new_items: &[(u64, Vec<f32>)],
) -> Result<(), Box<dyn std::error::Error>> {
    let bucket_dir = format!("{}/bucket_{:06}", &PRIECO_CONFIG.vector_path, bucket_id);
    create_dir_all(&bucket_dir)?;

    let existing_ids_with_positions = load_existing_bucket_ids(bucket_id)?;

    let ids_path = format!("{}/ids.bin", bucket_dir);
    let vecs_path = format!("{}/vectors.bin", bucket_dir);

    let mut merged: HashMap<u64, Vec<f32>> =
        HashMap::with_capacity(existing_ids_with_positions.len() + new_items.len());

    if Path::new(&vecs_path).exists() {
        let mut vecs_file = File::open(&vecs_path)?;
        for (&id, &pos) in existing_ids_with_positions.iter() {
            let mut vector = vec![0f32; 384];
            vecs_file.seek(SeekFrom::Start((pos * 384 * 4) as u64))?;
            for i in 0..384 {
                let mut buf = [0u8; 4];
                vecs_file.read_exact(&mut buf)?;
                vector[i] = f32::from_le_bytes(buf);
            }
            merged.insert(id, vector);
        }
    }

    // In case of same id, wins the new vector
    for (id, vector) in new_items {
        merged.insert(*id, vector.clone());
    }

    // Write the files to disk
    let mut ids_file = File::create(ids_path)?;
    let mut vecs_file = File::create(vecs_path)?;
    for (id, vector) in merged.iter() {
        ids_file.write_all(&id.to_le_bytes())?;
        for &val in vector {
            vecs_file.write_all(&val.to_le_bytes())?;
        }
    }

    Ok(())
}

fn load_existing_bucket_ids(
    bucket_id: usize,
) -> Result<HashMap<u64, usize>, Box<dyn std::error::Error>> {
    let ids_path = format!(
        "{}/bucket_{:06}/ids.bin",
        &PRIECO_CONFIG.vector_path, bucket_id
    );
    let mut id_positions: HashMap<u64, usize> = HashMap::with_capacity(15_000);

    if Path::new(&ids_path).exists() {
        let mut file = File::open(&ids_path)?;
        let mut buffer = [0u8; 8];
        let mut position = 0;

        while file.read_exact(&mut buffer).is_ok() {
            let id = u64::from_le_bytes(buffer);
            id_positions.insert(id, position);
            position += 1;
        }
    }

    Ok(id_positions)
}
