/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions, create_dir_all, metadata, read_dir, remove_file},
    io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

/*
  Import external libraries
*/
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use zip::ZipArchive;
use zstd::stream::Encoder as ZstdEncoder;

/*
  Import own libraries
*/
use prieco_core::{
    ID_SIZE, INSERTER_IMPORT_DIR, META_DICTIONARY, PRIECO_CONFIG, PRIECO_META, RECORD_SIZE,
    TANTIVY_INDEX, TANTIVY_WRITER, VECTOR_CENTROPOIDS, VECTOR_DIM, WebDocument, file_exists,
    globals::icons, url_to_domain_id, url_to_id,
};

/*
  Constants
*/
const SKIP_MERGE_FILE: &str = "dont_merge.txt";
pub const MAX_VECTORS_IN_VRAM: usize = 1_500_000;
const BATCH_SIZE_FOR_GPU: usize = 1_500;
const ZSTD_LEVEL: i32 = 3;

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
                if s.contains(".zip") {
                    files.push(s.to_string());
                }
            }
        }
    }
    if files.is_empty() {
        println!("{}: No files found. Sleeping...", icons::DB_INSERT,);
        return Ok(());
    }

    let schema = TANTIVY_INDEX.schema();
    let doc_id_field = schema.get_field("doc_id").unwrap();
    let domain_id_field = schema.get_field("domain_id").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let description_field = schema.get_field("description").unwrap();
    let content_field = schema.get_field("content").unwrap();
    let keywords_field = schema.get_field("keywords").unwrap();
    let lang_field = schema.get_field("lang").unwrap();
    let loc_field = schema.get_field("loc").unwrap();
    let date_field = schema.get_field("date").unwrap();
    let safe_s_field = schema.get_field("safe_s").unwrap();
    let intent_field = schema.get_field("intent").unwrap();

    let mut compressor = match &*META_DICTIONARY {
        Some(dict) => zstd::bulk::Compressor::with_dictionary(3, dict)?,
        None => zstd::bulk::Compressor::new(3)?,
    };

    // Process files
    let mut vector_idx_buffer: HashMap<u64, Vec<f32>> = HashMap::with_capacity(1_000_000);
    for file_name in &files {
        let zip_file = File::open(file_name)?;
        let mut archive = ZipArchive::new(zip_file)?;

        for entry_idx in 0..archive.len() {
            let entry = archive.by_index(entry_idx)?;
            let entry_name = entry.name().to_string();
            if !entry_name.ends_with(".txt") {
                continue;
            }

            let reader = BufReader::with_capacity(1 << 20, entry);
            let mut inserted = 0;

            for line in reader.lines() {
                let line = line?;

                let parts: Vec<&str> = line.split("<-->").collect();

                if parts.len() != 18 {
                    continue;
                }

                let url = sanitize_string(parts[0]);
                let id = url_to_id(&url);

                // Preserve uniqness
                if PRIECO_META
                    .meta_ks
                    .get(&id.to_be_bytes())
                    .unwrap()
                    .is_some()
                {
                    println!("Uniqnuess: {}", &url);
                    continue;
                }

                let vector: Vec<f32> = parts[17]
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
                    println!("Point len: {}", points.len());
                    continue;
                }

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

                    intent: parts[14].parse().unwrap_or(5),
                    is_mobile: parts[15] == "1" || parts[15] == "true",
                    has_500_words: parts[16] == "1" || parts[16] == "true",

                    search_score: 0.0,
                    source: String::new(),
                };

                /* FJALL */
                let doc_bytes = serde_json::to_vec(&doc)?;
                let compressed_doc = compressor.compress(&doc_bytes)?;
                PRIECO_META
                    .meta_ks
                    .insert(&id.to_be_bytes(), &compressed_doc)?;

                /* Tantivy */
                TANTIVY_WRITER.lock().add_document(tantivy::doc!(
                    doc_id_field => id,
                    domain_id_field => url_to_domain_id(&url),
                    title_field => doc.title.clone(),
                    description_field => doc.description.clone(),
                    content_field => doc.content.clone(),
                    keywords_field => doc.keywords.clone(),
                    lang_field => doc.lang.clone(),
                    loc_field => doc.loc.clone(),
                    date_field => doc.date,
                    safe_s_field => doc.safe_s,
                    intent_field => doc.intent as u64
                ))?;

                vector_idx_buffer.insert(id, vector);

                inserted += 1;
                if inserted % 1_000 == 0 {
                    let segment_count = TANTIVY_INDEX
                        .searchable_segment_ids()
                        .map(|segments| segments.len())
                        .unwrap_or(0);

                    println!(
                        "{}: Inserted {} | Tantivy Segments: {}",
                        icons::DB_INSERT,
                        inserted,
                        segment_count
                    );
                }

                if vector_idx_buffer.len() >= MAX_VECTORS_IN_VRAM {
                    println!(
                        "{}: Commiting {} vectors",
                        icons::DB_INSERT,
                        vector_idx_buffer.len()
                    );

                    vector_process(&mut vector_idx_buffer)?;
                    println!("{}: Vector idx commited", icons::DB_INSERT);
                }

                println!("Inserted: {}", &doc.url);
            }

            vector_process(&mut vector_idx_buffer)?;
            println!("{}: Vector idx commited", icons::DB_INSERT);
        }

        drop(archive);
        remove_file(file_name)?;
        println!("{}: Removed {}", icons::DB_INSERT, file_name);
    }

    if !file_exists(SKIP_MERGE_FILE) {
        merge_tantivy();

        println!("Compacting Meta...");
        PRIECO_META
            .meta_ks
            .major_compact()
            .expect("Failed to run major compaction");

        println!(
            "{}: Merging staging files into buckets...",
            icons::DB_INSERT
        );

        let staging_files: Vec<usize> = read_dir(&PRIECO_CONFIG.vector_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name();
                let s = name.to_string_lossy();
                s.strip_prefix("staging_")
                    .and_then(|r| r.strip_suffix(".bin"))
                    .and_then(|id_str| id_str.parse::<usize>().ok())
            })
            .collect();

        let total_buckets = staging_files.len();
        let merged_count = AtomicUsize::new(0);

        staging_files.par_iter().for_each(|bucket_id| {
            if let Err(e) = merge_bucket(*bucket_id) {
                println!("Failed to merge bucket {}! {}", bucket_id, e);
            } else {
                let current = merged_count.fetch_add(1, Ordering::Relaxed) + 1;

                if current % 100 == 0 || current == total_buckets {
                    println!(
                        "{}: Merge progress: {}/{} ({:.1}%)",
                        icons::DB_INSERT,
                        current,
                        total_buckets,
                        current as f64 / total_buckets as f64 * 100.0
                    );
                }
            }
        });

        println!("{}: Merge complete", icons::DB_INSERT);
    }

    Ok(())
}

pub fn vector_process(
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
    for (bucket_id, items) in bucket_data.iter() {
        append_to_bucket(*bucket_id, items)
            .map_err(|e| format!("Failed on bucket {}: {}", bucket_id, e))?;
    }

    chunk_buffer.clear();
    Ok(())
}

fn append_to_bucket(
    bucket_id: usize,
    items: &[(u64, Vec<f32>)],
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let staging_path = format!(
        "{}/staging_{:06}.bin",
        &PRIECO_CONFIG.vector_path, bucket_id
    );
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

/* Merge functions */
pub fn merge_tantivy() {
    let mut writer = TANTIVY_WRITER.lock();

    println!("Tantivy Merge: Commiting...");
    writer.commit().expect("Failed to commit pending documents");

    println!("Tantivy Merge: Fetching segments...");
    let segment_ids = TANTIVY_INDEX
        .searchable_segment_ids()
        .expect("Failed to get searchable segment IDs");

    println!(
        "Tantivy Merge: Found {} segments. Merging...",
        segment_ids.len()
    );
    match writer.merge(&segment_ids).wait() {
        Ok(_) => println!("Merge completed successfully!"),
        Err(e) => eprintln!("Merge failed: {}", e),
    }
}

pub fn merge_bucket(bucket_id: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
    let staging_path = format!(
        "{}/staging_{:06}.bin",
        &PRIECO_CONFIG.vector_path, bucket_id
    );
    let zst_path = format!(
        "{}/bucket_{:06}.bin.zst",
        &PRIECO_CONFIG.vector_path, bucket_id
    );

    if !Path::new(&staging_path).exists() {
        return Ok(());
    }

    let file_size = metadata(&staging_path)?.len() as usize;
    let clean_bytes = (file_size / RECORD_SIZE) * RECORD_SIZE;
    if clean_bytes < file_size {
        let file = OpenOptions::new().write(true).open(&staging_path)?;
        file.set_len(clean_bytes as u64)?;
    }

    let mut staging: HashMap<u64, u64> = HashMap::new();
    {
        let mut f = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut id_buf = [0u8; ID_SIZE];
        let mut current_offset = 0u64;
        let skip_bytes = (VECTOR_DIM * 4) as i64;

        while f.read_exact(&mut id_buf).is_ok() {
            let id = u64::from_le_bytes(id_buf);
            staging.insert(id, current_offset);
            f.seek(SeekFrom::Current(skip_bytes))?;
            current_offset += RECORD_SIZE as u64;
        }
    }

    let out = AtomicFile::new(&zst_path)?;
    let mut encoder = ZstdEncoder::new(out, ZSTD_LEVEL)?;

    if Path::new(&zst_path).exists() {
        let zst_file = File::open(&zst_path)?;
        let mut decoder = match zstd::stream::Decoder::new(zst_file) {
            Ok(dec) => dec,
            Err(e) => {
                println!(
                    "{}: Bucket {} zst corrupted ({e}), rebuilding from staging only",
                    icons::DB_INSERT,
                    bucket_id
                );
                return Err(e.into());
            }
        };

        let mut record_buf = vec![0u8; RECORD_SIZE];

        while decoder.read_exact(&mut record_buf).is_ok() {
            let id = u64::from_le_bytes(record_buf[0..ID_SIZE].try_into().unwrap());

            if !staging.contains_key(&id) {
                encoder.write_all(&record_buf)?;
            }
        }
    }

    {
        let mut f = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut record_buf = vec![0u8; RECORD_SIZE];
        let mut current_offset = 0u64;

        while f.read_exact(&mut record_buf).is_ok() {
            let id = u64::from_le_bytes(record_buf[0..ID_SIZE].try_into().unwrap());

            if staging.get(&id) == Some(&current_offset) {
                encoder.write_all(&record_buf)?;
            }

            current_offset += RECORD_SIZE as u64;
        }
    }

    let atomic = encoder.finish()?;
    atomic.commit()?;
    remove_file(&staging_path)?;

    Ok(())
}

/* Helper functions */
fn sanitize_string(s: &str) -> String {
    s.replace('"', "").replace('\'', "")
}
