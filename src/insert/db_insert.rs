/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
const BATCH_SIZE_FOR_GPU: usize = 1_500;

/*
  Structs
*/
struct AtomicFile {
    tmp_path: PathBuf,
    final_path: PathBuf,
    writer: BufWriter<File>,
}
impl AtomicFile {
    fn new(final_path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let final_path = final_path.as_ref().to_path_buf();
        let tmp_path = final_path.with_extension("tmp");
        let writer = BufWriter::with_capacity(1 << 20, File::create(&tmp_path)?);
        Ok(Self {
            tmp_path,
            final_path,
            writer,
        })
    }
    fn commit(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.writer.flush()?;
        std::fs::rename(&self.tmp_path, &self.final_path)?;
        Ok(())
    }
}
impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
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

    // Process files
    let mut vector_idx_buffer: HashMap<u64, Vec<f32>> = HashMap::with_capacity(1_000_000);
    for file_name in &files {
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

                vector_process(&mut vector_idx_buffer)?;
                println!("{}: Vector idx commited", icons::DB_INSERT);
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

        vector_process(&mut vector_idx_buffer)?;
        println!("{}: Vector idx commited", icons::DB_INSERT);
    }

    println!(
        "{}: Merging staging files into buckets...",
        icons::DB_INSERT
    );
    let bucket_dirs: Vec<_> = read_dir(&PRIECO_CONFIG.vector_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("staging.bin").exists())
        .collect();
    let total_buckets = bucket_dirs.len();

    let mut merged_count = 0;
    for entry in &bucket_dirs {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Some(id_str) = name_str.strip_prefix("bucket_") {
            if let Ok(bucket_id) = id_str.parse::<usize>() {
                merge_bucket(bucket_id)?;
                merged_count += 1;
                if merged_count % 100 == 0 || merged_count == total_buckets {
                    println!(
                        "{}: Merge progress: {}/{} ({:.1}%)",
                        icons::DB_INSERT,
                        merged_count,
                        total_buckets,
                        merged_count as f64 / total_buckets as f64 * 100.0
                    );
                }
            }
        }
    }
    println!("{}: Merge complete", icons::DB_INSERT);

    for file_name in &files {
        let _ = remove_file(file_name);
    }

    Ok(())
}

fn vector_process(
    chunk_buffer: &mut HashMap<u64, Vec<f32>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let ids: Vec<u64> = chunk_buffer.keys().copied().collect();
    let mut bucket_data: HashMap<usize, Vec<(u64, Vec<f32>)>> =
        HashMap::with_capacity(chunk_buffer.len() + 1_000);

    println!("{}: GPU assigning!", icons::DB_INSERT);
    let mut inserted_counter = 0;
    for chunk in ids.chunks(BATCH_SIZE_FOR_GPU) {
        let vectors: Vec<Vec<f32>> = chunk
            .iter()
            .map(|id| chunk_buffer.get(id).unwrap().clone())
            .collect();

        let bucket_ids = VECTOR_CENTROPOIDS.assign_batch(&vectors)?;

        for (i, &id) in chunk.iter().enumerate() {
            let bucket_id = bucket_ids[i];
            let vector = chunk_buffer.get(&id).unwrap().clone();
            bucket_data
                .entry(bucket_id)
                .or_insert_with(Vec::new)
                .push((id, vector));
        }

        inserted_counter += 1;
        if inserted_counter % 200_000 == 0 {
            println!(
                "{}: Assigned: {}/{}",
                icons::DB_INSERT,
                inserted_counter,
                bucket_data.len()
            );
        }
    }

    println!(
        "{}: Writing chunks to disk! {}",
        icons::DB_INSERT,
        bucket_data.len()
    );
    bucket_data.par_iter().try_for_each(|(bucket_id, items)| {
        append_to_bucket(*bucket_id, items)
            .map_err(|e| format!("Failed on bucket {}: {}", bucket_id, e))
    })?;

    chunk_buffer.clear();
    Ok(())
}

fn append_to_bucket(
    bucket_id: usize,
    items: &[(u64, Vec<f32>)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bucket_dir = format!("{}/bucket_{:06}", &PRIECO_CONFIG.vector_path, bucket_id);
    create_dir_all(&bucket_dir)?;

    let staging_path = format!("{}/staging.bin", bucket_dir);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&staging_path)?;

    let mut writer = BufWriter::with_capacity(1 << 16, file);
    for (id, vector) in items {
        writer.write_all(&id.to_le_bytes())?;
        let bytes: &[u8] = bytemuck::cast_slice(vector.as_slice());
        writer.write_all(bytes)?;
    }
    writer.flush()?;
    Ok(())
}

fn merge_bucket(bucket_id: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bucket_dir = format!("{}/bucket_{:06}", &PRIECO_CONFIG.vector_path, bucket_id);
    let staging_path = format!("{}/staging.bin", bucket_dir);
    if !Path::new(&staging_path).exists() {
        return Ok(());
    }

    let ids_path = format!("{}/ids.bin", bucket_dir);
    let vecs_path = format!("{}/vectors.bin", bucket_dir);

    // First pass: collect all IDs that exist in staging (these will overwrite existing)
    let mut staging_ids: HashMap<u64, ()> = HashMap::new();
    {
        let mut staging_file = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut id_buf = [0u8; 8];
        while staging_file.read_exact(&mut id_buf).is_ok() {
            staging_ids.insert(u64::from_le_bytes(id_buf), ());
            // Skip the vector bytes
            let mut skip = vec![0u8; 384 * 4];
            staging_file.read_exact(&mut skip)?;
        }
    }

    let mut out_ids = AtomicFile::new(format!("{}/ids.bin", bucket_dir))?;
    let mut out_vecs = AtomicFile::new(format!("{}/vectors.bin", bucket_dir))?;

    // Stream existing canonical files, skipping any IDs that staging will overwrite
    if Path::new(&ids_path).exists() && Path::new(&vecs_path).exists() {
        let ids_len = std::fs::metadata(&ids_path)?.len() as usize;
        let vecs_len = std::fs::metadata(&vecs_path)?.len() as usize;
        let count = ids_len / 8;
        if vecs_len == count * 384 * 4 {
            let mut ids_file = BufReader::with_capacity(1 << 20, File::open(&ids_path)?);
            let mut vecs_file = BufReader::with_capacity(1 << 20, File::open(&vecs_path)?);
            let mut id_buf = [0u8; 8];
            let mut vec_buf = vec![0u8; 384 * 4];
            for _ in 0..count {
                ids_file.read_exact(&mut id_buf)?;
                vecs_file.read_exact(&mut vec_buf)?;
                let id = u64::from_le_bytes(id_buf);
                // Only write if staging doesn't have a newer version
                if !staging_ids.contains_key(&id) {
                    out_ids.write_all(&id_buf)?;
                    out_vecs.write_all(&vec_buf)?;
                }
            }
        } else {
            println!(
                "{}: Warning: bucket {} corrupted, discarding",
                icons::DB_INSERT,
                bucket_id
            );
        }
    }

    // Stream staging into output
    {
        let mut staging_file = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut id_buf = [0u8; 8];
        let mut vec_buf = vec![0u8; 384 * 4];
        while staging_file.read_exact(&mut id_buf).is_ok() {
            staging_file.read_exact(&mut vec_buf)?;
            out_ids.write_all(&id_buf)?;
            out_vecs.write_all(&vec_buf)?;
        }
    }

    out_ids.commit()?;
    out_vecs.commit()?;

    remove_file(&staging_path)?;
    Ok(())
}

/* Helper functions */
fn sanitize_string(s: &str) -> String {
    s.replace('"', "").replace('\'', "")
}
