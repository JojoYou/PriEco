/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions, create_dir_all, metadata, read_dir, remove_file},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/*
  Import external libraries
*/
use ahash::AHashSet;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rocksdb::{WriteBatch, WriteOptions};
use tantivy::{IndexWriter, Term, index::SegmentId};
use zip::ZipArchive;
use zstd::stream::{Encoder as ZstdEncoder, decode_all};

/*
  Import own libraries
*/
use prieco_core::{
    ID_SIZE, INSERTER_IMPORT_DIR, PRIECO_CONFIG, RECORD_SIZE, ROCKSDB_INDEX, TANTIVY_HEAP_SIZE,
    TANTIVY_INDEX, TANTIVY_WRITER, VECTOR_CENTROPOIDS, VECTOR_DIM, WebDocument, file_exists,
    globals::{colors, icons},
    url_to_id,
};

/*
  Constants
*/
const SKIP_MERGE_FILE: &str = "dont_merge.txt";
const MAX_VECTORS_IN_VRAM: usize = 1_500_000;
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

    let doc_id_field = TANTIVY_INDEX.schema().get_field("doc_id").unwrap();
    let title_field = TANTIVY_INDEX.schema().get_field("title").unwrap();
    let description_field = TANTIVY_INDEX.schema().get_field("description").unwrap();
    let content_field = TANTIVY_INDEX.schema().get_field("content").unwrap();
    let keywords_field = TANTIVY_INDEX.schema().get_field("keywords").unwrap();
    let safe_s_field = TANTIVY_INDEX.schema().get_field("safe_s").unwrap();

    // Create local RocksDB writer
    let mut rocksdb_write_opts = WriteOptions::default();
    rocksdb_write_opts.disable_wal(true);
    let mut rocksdb_batch = WriteBatch::default();
    let mut batch_ids: AHashSet<u64> = AHashSet::with_capacity(2_000); // Monitoring batch

    // Process files
    let mut vector_idx_buffer: HashMap<u64, Vec<f32>> = HashMap::with_capacity(1_000_000);
    for file_name in &files {
        println!("{}: Opening {}", icons::DB_INSERT, file_name);
        let zip_file = File::open(file_name)?;
        let mut archive = ZipArchive::new(zip_file)?;

        for entry_idx in 0..archive.len() {
            let entry = archive.by_index(entry_idx)?;
            let entry_name = entry.name().to_string();
            if !entry_name.ends_with(".txt") {
                continue;
            }
            println!("{}: Processing entry {}", icons::DB_INSERT, entry_name);

            let reader = BufReader::with_capacity(1 << 20, entry);
            let mut inserted = 0;

            for line in reader.lines() {
                let line = line?;

                // Create a web document with checks
                let parts: Vec<&str> = line.split("<-->").collect();
                if parts.len() != 15 {
                    continue;
                }

                let url = sanitize_string(parts[0]);
                let id = url_to_id(&url);

                // Preserve uniqness
                if batch_ids.contains(&id) || ROCKSDB_INDEX.get(id.to_be_bytes())?.is_some() {
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

                batch_ids.insert(id);

                rocksdb_batch.put(id.to_be_bytes(), serde_json::to_vec(&doc)?);

                /* Tantivy */
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
                    ROCKSDB_INDEX.write_opt(rocksdb_batch, &rocksdb_write_opts)?;
                    rocksdb_batch = WriteBatch::default();
                    batch_ids.clear();
                    println!("{}: Inserted {}", icons::DB_INSERT, inserted);
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
            }

            // Commit after file
            batch_ids.clear();

            if !rocksdb_batch.is_empty() {
                ROCKSDB_INDEX.write_opt(rocksdb_batch, &rocksdb_write_opts)?;
                rocksdb_batch = WriteBatch::default();
            }
            println!("{}: RocksDB commited", icons::DB_INSERT);

            TANTIVY_WRITER.lock().commit()?;
            println!(
                "{}: Tantivy commited {} vectors",
                icons::DB_INSERT,
                vector_idx_buffer.len()
            );

            vector_process(&mut vector_idx_buffer)?;
            println!("{}: Vector idx commited", icons::DB_INSERT);
        }

        drop(archive);
        remove_file(file_name)?;
        println!("{}: Removed {}", icons::DB_INSERT, file_name);
    }

    println!("{}: Merging tantivy...", icons::DB_INSERT);
    let segments = TANTIVY_INDEX.searchable_segments()?;
    let segment_ids: Vec<SegmentId> = segments.iter().map(|s| s.id()).collect();
    let mut writer = TANTIVY_WRITER.lock();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(writer.merge(&segment_ids))?;

    if !file_exists(SKIP_MERGE_FILE) {
        println!(
            "{}: Merging staging files into buckets...",
            icons::DB_INSERT
        );
        let staging_files: Vec<usize> = read_dir(&PRIECO_CONFIG.vector_path)?
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
        let mut merged_count = 0;
        for bucket_id in &staging_files {
            merge_bucket(*bucket_id)?;
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
        println!("{}: Merge complete", icons::DB_INSERT);
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

fn merge_bucket(bucket_id: usize) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    // Collect staging IDs
    let mut staging_ids: HashMap<u64, ()> = HashMap::new();
    {
        let mut f = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut id_buf = [0u8; ID_SIZE];
        let mut skip = vec![0u8; VECTOR_DIM * 4];
        while f.read_exact(&mut id_buf).is_ok() {
            staging_ids.insert(u64::from_le_bytes(id_buf), ());
            f.read_exact(&mut skip)?;
        }
    }

    // Read existing zst
    let existing: Option<Vec<u8>> = if Path::new(&zst_path).exists() {
        let compressed = std::fs::read(&zst_path)?;
        match decode_all(compressed.as_slice()) {
            Ok(data) => Some(data),
            Err(e) => {
                println!(
                    "{}: Bucket {} zst corrupted ({e}), rebuilding from staging only",
                    icons::DB_INSERT,
                    bucket_id
                );
                let _ = remove_file(&zst_path);
                None
            }
        }
    } else {
        None
    };

    let out = AtomicFile::new(&zst_path)?;
    let mut encoder = ZstdEncoder::new(out, ZSTD_LEVEL)?;

    // Stream existing data
    if let Some(data) = existing {
        let count = data.len() / RECORD_SIZE;
        for i in 0..count {
            let base = i * RECORD_SIZE;
            let id = u64::from_le_bytes(data[base..base + ID_SIZE].try_into().unwrap());
            if !staging_ids.contains_key(&id) {
                encoder.write_all(&data[base..base + RECORD_SIZE])?;
            }
        }
    }

    // Append staging into encoder
    {
        let mut f = BufReader::with_capacity(1 << 20, File::open(&staging_path)?);
        let mut buf = vec![0u8; RECORD_SIZE];
        while f.read_exact(&mut buf).is_ok() {
            encoder.write_all(&buf)?;
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
