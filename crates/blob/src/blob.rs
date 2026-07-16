/*
  File: blob/blob.rs
  Description:

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-02-07
  Last Modified: 2026-07-10

  Usage: Run() to take archived htmls and insert them into Blob storage
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{self, File, OpenOptions, create_dir_all, read_dir, remove_file},
    hash::{BuildHasher, DefaultHasher, Hash, Hasher},
    io::{BufWriter, Cursor, Read, Write},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use ahash::AHashMap;
/*
  Import external libraries
*/
use fjall::{Guard, PersistMode};
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use rand::{Rng, rng};
use serde::Deserialize;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, BLOB_STORAGE, META_DICTIONARY, PRIECO_FJALL, WebDocument,
    globals::{colors, icons},
    url_to_id,
};
use rocksdb::{DB, DBCompressionType, Options};
use std::sync::Arc;
pub static ORPHAN_STORAGE: Lazy<Arc<DB>> = Lazy::new(|| {
    Arc::new({
        let mut options = Options::default();
        options.create_if_missing(true);
        options.set_compression_type(DBCompressionType::Lz4);

        options.set_max_background_jobs(2);
        options.set_write_buffer_size(64 * 1024 * 1024);

        DB::open(&options, Path::new("/mnt/hdd/orphan_blobs_triage")).unwrap()
    })
});

#[derive(serde::Serialize, serde::Deserialize)]
pub struct OrphanPayload {
    pub html_id: String,
    pub recovered_url: String,
    pub tag_data: HashMap<String, Vec<String>>,
}

pub fn run() {
    //feed_blobs_for_reembedding();
    /*let directories = find_all_directories();

    if directories.is_empty() {
        return;
    }

    for dir_path in directories {
        println!(
            "{}{}: Processing: {:?}{}",
            icons::BLOB,
            colors::GREEN,
            dir_path,
            colors::RESET,
        );

        if let Err(e) = process_directory(&dir_path, &known_blobs) {
            println!(
                "{}{}: Processing directory: {:?} Error: {}{}",
                icons::BLOB,
                colors::RED,
                dir_path,
                colors::RESET,
                e
            );

            return;
        } else {
            if let Err(e) = remove_dir_all(&dir_path) {
                println!(
                    "{}{}: Removing directory: {:?} Error: {}{}",
                    icons::BLOB,
                    colors::RED,
                    dir_path,
                    colors::RESET,
                    e
                );
            } else {
                println!(
                    "{}{}: Successfully processed and removed: {:?}{}",
                    icons::BLOB,
                    colors::GREEN,
                    dir_path,
                    colors::RESET,
                );
            }
        }
    }*/
}
const BLOB_V2_PROGRESS_FILE: &str = "blob_v2_migration_progress.bin";
const META_INDEX_CACHE_FILE: &str = "meta_known_blobs_cache.bin";
const ORPHAN_RAM_DIR: &str = "/mnt/ramdisk/prieco_orphans";

const MAX_RAM_FILES: usize = 100_000;
const CHECK_INTERVAL: u64 = 5_000;
fn process_directory(
    dir_path: &Path,
    known_blobs: &[[u8; 8]],
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = read_dir(dir_path)?;
    let tar_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext == "gz")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    println!(
        "{}: Found {} tar.gz files to process",
        icons::BLOB,
        tar_files.len()
    );

    let mut buffer: Vec<u8> = Vec::with_capacity(10 * 1024 * 1024);

    for tar_path in tar_files {
        let mut batch = PRIECO_FJALL.blob_db.batch();

        println!("{}: Processing: {:?}", icons::BLOB, tar_path);

        let tar_file = File::open(&tar_path)?;
        let decompressor = GzDecoder::new(tar_file);
        let mut archive = Archive::new(decompressor);

        let mut files_inserted = 0;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;

            let path = entry.path()?;
            let file_name = path.to_str().ok_or("{}: Invalid filename")?;

            if entry.header().entry_type().is_dir() {
                continue;
            }

            let has_valid_ext = Path::new(file_name)
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|s| Path::new(s).extension())
                .and_then(|s| s.to_str())
                .map(|ext| ext == "zst" || ext == "txt")
                .unwrap_or(false);

            if !has_valid_ext {
                continue;
            }

            let name: u64 = Path::new(file_name)
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|s| Path::new(s).file_stem())
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok())
                .ok_or("{}: Invalid blob ID in filename")?;

            let flag: u8 = match Path::new(file_name)
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|s| Path::new(s).extension())
                .and_then(|s| s.to_str())
            {
                Some("zst") => 1,
                Some("txt") => 0,
                _ => 0,
            };

            buffer.clear();
            buffer.push(flag);
            entry.read_to_end(&mut buffer)?;

            batch.insert(&PRIECO_FJALL.blobs_ks, name.to_le_bytes(), buffer.clone());
            files_inserted += 1;

            if files_inserted % 1000 == 0 {
                let _ = batch.commit();
                batch = PRIECO_FJALL.blob_db.batch();

                println!(
                    "{}: Inserted {} files from {:?}",
                    icons::BLOB,
                    files_inserted,
                    tar_path.file_name().ok_or(format!(
                        "{}: Invalid filename: {:?} ",
                        icons::BLOB,
                        tar_path
                    ))
                );

                let key_arr = name.to_le_bytes();
                if known_blobs.binary_search(&key_arr).is_err() {
                    println!(
                        "{}: {}INTEGRITY CHECK FAILED for key {}!{}",
                        icons::BLOB,
                        colors::RED,
                        name,
                        colors::RESET
                    );
                    return Err(format!("Integrity check failed for blob {}", name).into());
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        println!(
            "{}: {}Completed {:?}: {} files inserted{}",
            icons::BLOB,
            colors::GREEN,
            tar_path.file_name().ok_or(format!(
                "{}: Invalid filename: {:?} ",
                icons::BLOB,
                tar_path
            )),
            files_inserted,
            colors::RESET
        );

        println!("{}: Flushing!", icons::BLOB);
        let _ = batch.commit();
        let _ = PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll);

        println!(
            "{}: {}Flushed!{}",
            icons::BLOB,
            colors::GREEN,
            colors::RESET
        );

        remove_file(&tar_path)?;
        println!(
            "{}: Removed {:?}",
            icons::BLOB,
            tar_path.file_name().ok_or(format!(
                "{}: Invalid filename: {:?} ",
                icons::BLOB,
                tar_path
            ))
        );
    }

    Ok(())
}

/* Helper functions */
fn find_all_directories() -> Vec<PathBuf> {
    let watch_path = Path::new(BLOB_IMPORT_DIR);
    if !watch_path.exists() {
        let _ = create_dir_all(watch_path);
        return Vec::new();
    }

    read_dir(watch_path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_else(|_| Vec::new())
}

#[derive(Deserialize)]
struct HtmlOnly {
    html: String,
}

/*
  Description: Feeds old blob+meta data back through the embedder pipeline.
  Reconstructs embed-input text from blob storage, writes it in the legacy
  VECTORS pipe-format so the existing embedder run() picks it up unchanged.
  The meta_ks key (doc id) is stashed in the `html` field so it survives
  through to results.txt for the collector step later.
*/
pub fn vector_dir_bytes() -> u64 {
    read_dir("/mnt/vec/")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}
pub const VECTOR_DIR_MAX: u64 = 1_717_986_918;

pub fn write_to_random_file(dir: &str, content: &str) {
    let mut rng = rng();
    let mut file_path = PathBuf::from(dir);

    loop {
        let file_name = format!("{}.txt", rng.random_range(0..10_000_000_000_000_i64));
        file_path.push(&file_name);

        if !file_path.exists() {
            let mut file = match File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    println!(
                        "{}Write to random file function{}: Failed to create file {} Because of: {}",
                        colors::RED,
                        colors::RESET,
                        file_path.to_str().unwrap_or(""),
                        e
                    );
                    return;
                }
            };
            let _ = file.write_all(content.as_bytes());
            return;
        }

        file_path.pop();
    }
}
pub fn test_blob_relevance_data_extraction() {
    println!("Starting Direct Blob Storage Extraction Test...");

    // Iterate DIRECTLY over the blob keyspace
    let mut iter = PRIECO_FJALL.blobs_ks.iter();

    let mut blobs_checked = 0;
    let mut found_og_type = 0;
    let mut found_pub_date = 0;

    // Test the first 10,000 blobs
    while blobs_checked < 100_000 {
        let Some(item) = iter.next() else {
            break;
        };

        // 2. Extract key and value. Continue to the next item if this specific read fails.
        let Ok((_key, val)) = item.into_inner() else {
            continue;
        };

        if val.is_empty() {
            continue;
        }

        // Pass the raw Fjall bytes directly into your decoder
        let embed_text = decode_blob_to_embed_text(val.as_ref());
        if embed_text.trim().is_empty() {
            continue;
        }

        blobs_checked += 1;
        let mut has_og = false;
        let mut has_date = false;

        // Hunt for the properties
        // Note: converting to lowercase just in case decode_blob_to_embed_text doesn't normalize
        for token in embed_text.split_whitespace() {
            let lower_token = token.to_lowercase();

            if lower_token.starts_with("og:type=") {
                has_og = true;
                println!("Found OG: {}", token); // Uncomment to debug actual values
            }

            if lower_token.starts_with("article:published_time=")
                || lower_token.starts_with("date=")
                || lower_token.starts_with("pubdate=")
            {
                has_date = true;
                println!("Found Date: {}", token); // Uncomment to debug actual values
            }
        }

        if has_og {
            found_og_type += 1;
        }
        if has_date {
            found_pub_date += 1;
        }
    }

    println!("--------------------------------------------------");
    println!("✅ Blob Extraction Test Results");
    println!("Blobs Checked: {}", blobs_checked);
    if blobs_checked > 0 {
        println!(
            "Contains OpenGraph (og:type): {} ({:.2}%)",
            found_og_type,
            (found_og_type as f64 / blobs_checked as f64) * 100.0
        );
        println!(
            "Contains Publish Date: {} ({:.2}%)",
            found_pub_date,
            (found_pub_date as f64 / blobs_checked as f64) * 100.0
        );
    }
    println!("--------------------------------------------------");
}

const RESUME_FILE: &str = "/mnt/ssd/feed_resume.txt";
const BATCH_SIZE: usize = 5000;
pub fn feed_blobs_for_reembedding() {
    let start_key = fs::read_to_string(RESUME_FILE)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok());

    let mut fed = 0u64;
    let mut total_skipped = 0u64;

    let mut iter = if let Some(k) = start_key {
        println!("Resuming from meta_id: {}", k);
        PRIECO_FJALL.meta_ks.range(k.to_be_bytes()..)
    } else {
        println!("Starting from the beginning...");
        PRIECO_FJALL.meta_ks.iter()
    };

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut last_meta_id = 0u64;

    loop {
        while vector_dir_bytes() >= VECTOR_DIR_MAX {
            thread::sleep(Duration::from_millis(200));
        }

        for _ in 0..BATCH_SIZE {
            if let Some(item) = iter.next() {
                if let Ok((key, val)) = item.into_inner() {
                    batch.push((key, val));
                } else {
                    total_skipped += 1;
                }
            } else {
                break;
            }
        }

        if batch.is_empty() {
            break;
        }

        let chunk_size = (batch.len() / 8).max(1);

        let mut batch_fed = 0;
        let mut skips = [0u64; 6];
        let mut times = [0u128; 5];

        thread::scope(|s| {
            let mut handles = Vec::new();

            for chunk in batch.chunks(chunk_size) {
                let handle = s.spawn(move || {
                    let mut decompressor =
                        zstd::bulk::Decompressor::with_dictionary(&META_DICTIONARY)
                            .expect("Failed to init zstd");

                    let mut l_fed = 0;
                    let mut l_skips = [0u64; 6];
                    let mut l_times = [0u128; 5];

                    for (key, val) in chunk {
                        let t = Instant::now();
                        let decompressed = match decompressor.decompress(val.as_ref(), 10_000_000) {
                            Ok(d) => d,
                            Err(_) => {
                                l_skips[0] += 1;
                                continue;
                            }
                        };
                        l_times[0] += t.elapsed().as_micros();

                        let t = Instant::now();
                        let doc = match serde_json::from_slice::<WebDocument>(&decompressed) {
                            Ok(d) => d,
                            Err(_) => {
                                l_skips[1] += 1;
                                continue;
                            }
                        };
                        l_times[1] += t.elapsed().as_micros();

                        let Ok(id_bytes): Result<[u8; 8], _> = key.as_ref().try_into() else {
                            l_skips[2] += 1;
                            continue;
                        };
                        let meta_id = u64::from_be_bytes(id_bytes);

                        let stem = doc.html.rsplit('/').next().and_then(|f| {
                            f.strip_suffix(".zst").or_else(|| f.strip_suffix(".txt"))
                        });
                        let Some(blob_id) = stem.and_then(|s| s.parse::<u64>().ok()) else {
                            l_skips[2] += 1;
                            continue;
                        };

                        let t = Instant::now();
                        let raw_blob = match PRIECO_FJALL.blobs_ks.get(&blob_id.to_le_bytes()) {
                            Ok(Some(b)) => b,
                            _ => {
                                l_skips[3] += 1;
                                continue;
                            }
                        };
                        if raw_blob.is_empty() {
                            l_skips[4] += 1;
                            continue;
                        }
                        l_times[2] += t.elapsed().as_micros();

                        let t = Instant::now();
                        let embed_text = decode_blob_to_embed_text(&raw_blob);
                        if embed_text.trim().is_empty() {
                            l_skips[5] += 1;
                            continue;
                        }
                        l_times[3] += t.elapsed().as_micros();

                        let t = Instant::now();
                        let result_str = format!(
                            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{:?}\n{}\n{}\n{}",
                            doc.url,
                            doc.title,
                            doc.description,
                            doc.content,
                            doc.favicon,
                            doc.image,
                            doc.keywords,
                            doc.safe_s,
                            meta_id,
                            doc.lang,
                            doc.loc,
                            vec![doc.confidence, doc.effort, doc.qna, doc.sts],
                            doc.load,
                            doc.date,
                            embed_text
                        );
                        write_to_random_file("/mnt/vec/", &result_str);
                        l_times[4] += t.elapsed().as_micros();

                        l_fed += 1;
                    }
                    (l_fed, l_skips, l_times)
                });
                handles.push(handle);
            }

            for handle in handles {
                let (f, sk, tm) = handle.join().unwrap();
                batch_fed += f;
                for i in 0..6 {
                    skips[i] += sk[i];
                }
                for i in 0..5 {
                    times[i] += tm[i];
                }
            }
        });

        fed += batch_fed;
        let batch_total_skips: u64 = skips.iter().sum();
        total_skipped += batch_total_skips;

        if let Some((last_key, _)) = batch.last() {
            if let Ok(id_bytes) = <[u8; 8]>::try_from(last_key.as_ref()) {
                last_meta_id = u64::from_be_bytes(id_bytes);
                let _ = fs::write(RESUME_FILE, last_meta_id.to_string());
            }
        }

        let t_meta = times[0] as f64 / 1000.0;
        let t_json = times[1] as f64 / 1000.0;
        let t_hdd = times[2] as f64 / 1000.0;
        let t_dec = times[3] as f64 / 1000.0;
        let t_writ = times[4] as f64 / 1000.0;

        batch.clear();
        println!("--------------------------------------------------");
        println!(
            "✅ Total Fed: {} | ❌ Total Skipped: {} | Last ID: {}",
            fed, total_skipped, last_meta_id
        );
        println!(
            "⚠️ Skip Reasons: MetaZstd:{} | JSON:{} | Bad ID:{} | DB Miss:{} | EmptyBlob:{} | EmptyText:{}",
            skips[0], skips[1], skips[2], skips[3], skips[4], skips[5]
        );
        println!(
            "⏱️ Cumulative Thread Time (ms) -> MetaZstd: {:.0} | JSON: {:.0} | HDD Fetch: {:.0} | Blob Decode: {:.0} | Write: {:.0}",
            t_meta, t_json, t_hdd, t_dec, t_writ
        );
    }
}

fn get_blob_filename(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}.txt", hasher.finish())
}

pub fn feed_blob_to_crawler(
    vectors_dir: &str,
    url: &str,
    tag_data: &AHashMap<String, Vec<String>>,
) -> std::io::Result<()> {
    let mut tags = String::with_capacity(2048);
    for (key, value) in tag_data.iter() {
        if key == "a_href" || key == "img_src" || key == "meta" {
            continue;
        }
        if !tags.is_empty() {
            tags.push_str(" ");
        }
        tags.push_str(&value.join(" "));
    }

    let payload = format!("{}\n{}", url, tags);

    let filename = get_blob_filename(url);
    let file_path = Path::new(vectors_dir).join(filename);

    fs::write(file_path, payload)?;
    Ok(())
}

fn get_known_blob_ids() -> Vec<u64> {
    if Path::new(META_INDEX_CACHE_FILE).exists() {
        println!(
            "{}Found meta index cache on disk! Loading...{}",
            colors::GREEN,
            colors::RESET
        );

        let contents =
            std::fs::read_to_string(META_INDEX_CACHE_FILE).expect("Failed to read cache");
        let known_ids: Vec<u64> = contents
            .lines()
            .filter_map(|line| line.parse::<u64>().ok())
            .collect();

        return known_ids;
    }

    let mut known_ids: Vec<u64> = Vec::with_capacity(420_000_000);
    let mut j = 0;
    let mut x = 0;

    let mut decompressor = zstd::bulk::Decompressor::with_dictionary(&META_DICTIONARY)
        .expect("Failed to initialize zstd decompressor");
    for item in PRIECO_FJALL.meta_ks.iter() {
        j += 1;
        if let Ok(val) = item.value() {
            x += 1;
            if x % 100_000 == 0 {
                println!("Found: {} of {}", x, j);
            }

            if let Ok(decompressed_vec) = decompressor.decompress(&val, 10_000_000) {
                if let Ok(doc) = serde_json::from_slice::<HtmlOnly>(&decompressed_vec) {
                    let path = std::path::Path::new(&doc.html);

                    if let Some(stem) = path.file_stem() {
                        if let Some(stem_str) = stem.to_str() {
                            if let Ok(blob_id) = stem_str.parse::<u64>() {
                                known_ids.push(blob_id);
                            }
                        }
                    }
                }
            }
        }
    }

    known_ids.sort_unstable();
    known_ids.dedup();

    write_known_blob_cache(&known_ids);

    known_ids
}
pub fn diagnostic_self_referential_recovery() {
    println!(
        "{}--- STARTING SELF-REFERENTIAL URL RECOVERY ---{}",
        colors::BLUE,
        colors::RESET
    );

    let mut recovered_count = 0;
    let mut total_scanned = 0;

    for item in ORPHAN_STORAGE.iterator(rocksdb::IteratorMode::Start) {
        if let Ok((key_bytes, raw_blob)) = item {
            let key_arr: [u8; 8] = match key_bytes.as_ref().try_into() {
                Ok(arr) => arr,
                Err(_) => continue,
            };

            let target_blob_id = u64::from_le_bytes(key_arr);
            if target_blob_id == 0 {
                continue;
            }

            total_scanned += 1;

            let html_text = decode_blob_to_text(&raw_blob);

            let potential_urls = extract_all_urls_from_text(&html_text);

            for url in potential_urls {
                let clean = url.trim();

                let variations = vec![clean.to_string(), format!("{}/", clean)];

                let mut found = false;
                for var in variations {
                    if url_to_id(&var) == target_blob_id {
                        println!(
                            "{}SUCCESS:{} Match found for ID {}! \n  Recovered URL via self-link: {}",
                            colors::GREEN,
                            colors::RESET,
                            target_blob_id,
                            var
                        );
                        recovered_count += 1;
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }

            if total_scanned % 5000 == 0 {
                println!(
                    "Scanned: {} | Recovered: {}",
                    total_scanned, recovered_count
                );
            }

            if total_scanned >= 20000 {
                break;
            }
        }
    }

    println!(
        "\n{}Diagnostic Complete!{} Scanned: {} | Recovered: {}",
        colors::GREEN,
        colors::RESET,
        total_scanned,
        recovered_count
    );
}
fn write_known_blob_cache(known_ids: &[u64]) {
    use std::io::{BufWriter, Write};
    let file = std::fs::File::create(META_INDEX_CACHE_FILE).expect("Failed to create cache");
    let mut writer = BufWriter::new(file);

    for id in known_ids {
        writeln!(writer, "{id}").expect("Failed to write cache");
    }
    writer.flush().expect("Failed to flush");
}
pub fn migrate_blob_to_fjall(known_blobs: &[u64]) {
    let last_migrated_key: Option<[u8; 8]> = std::fs::read(BLOB_V2_PROGRESS_FILE)
        .ok()
        .and_then(|buf| buf.try_into().ok());

    let iter_mode = match &last_migrated_key {
        Some(key) => rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
        None => rocksdb::IteratorMode::Start,
    };

    println!(
        "{}Starting Triage Migration: Known -> Fjall (SSD) | Unknown -> RocksDB (HDD){}",
        colors::BLUE,
        colors::RESET
    );

    let mut count: u64 = 0;
    let mut known_count: u64 = 0;
    let mut orphan_count: u64 = 0;
    let mut last_key: Option<[u8; 8]> = None;
    let mut skip_first = last_migrated_key.is_some();

    let mut start_time = std::time::Instant::now();

    for item in BLOB_STORAGE.iterator(iter_mode) {
        let (key, value) = item.unwrap();

        if skip_first {
            skip_first = false;
            continue;
        }

        count += 1;
        let key_arr: [u8; 8] = key.as_ref().try_into().unwrap();
        let blob_id = u64::from_le_bytes(key_arr);

        if known_blobs.binary_search(&blob_id).is_ok() {
            PRIECO_FJALL.blobs_ks.insert(key_arr, &*value).unwrap();
            known_count += 1;
        } else {
            ORPHAN_STORAGE.put(key, &*value).unwrap();
            orphan_count += 1;
        }

        last_key = Some(key_arr);

        if count % 5_000 == 0 {
            if let Some(k) = last_key {
                std::fs::write(BLOB_V2_PROGRESS_FILE, k).unwrap();
            }

            let elapsed = start_time.elapsed();
            let items_per_sec = 5_000.0 / elapsed.as_secs_f64();

            println!(
                "{}Progress:{} {} scanned in {:.2?} ({:.0} ops/sec) | {} Saved to Fjall | {} Deferred to HDD",
                colors::BLUE,
                colors::RESET,
                count,
                elapsed,
                items_per_sec,
                known_count,
                orphan_count
            );

            start_time = std::time::Instant::now();
        }
    }

    if let Some(k) = last_key {
        std::fs::write(BLOB_V2_PROGRESS_FILE, k).unwrap();
    }

    PRIECO_FJALL
        .blob_db
        .persist(fjall::PersistMode::SyncAll)
        .unwrap();

    let mut flush_opts = rocksdb::FlushOptions::default();
    flush_opts.set_wait(true);
    ORPHAN_STORAGE.flush_opt(&flush_opts).unwrap();

    println!(
        "{}: {}Triage Migration Complete!{}\nTotal Scanned: {}\nMigrated to Fjall: {}\nDeferred to HDD: {}",
        icons::BLOB,
        colors::GREEN,
        colors::RESET,
        count,
        known_count,
        orphan_count
    );
}

fn append_to_cache(new_id: u64) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(mut file) = OpenOptions::new().append(true).open(META_INDEX_CACHE_FILE) {
        let _ = writeln!(file, "{new_id}");
    }
}

pub fn decode_blob_to_text(raw_db_value: &[u8]) -> String {
    if raw_db_value.is_empty() {
        return String::new();
    }

    let is_compressed = raw_db_value[0] == 1;
    let payload = &raw_db_value[1..];

    let decompressed_data = if is_compressed {
        match zstd::stream::decode_all(Cursor::new(payload)) {
            Ok(data) => data,
            Err(_) => return String::new(),
        }
    } else {
        payload.to_vec()
    };

    let decompressed = &decompressed_data;
    let mut reconstructed_text = String::with_capacity(decompressed.len() * 3);
    let mut i = 0;

    while i < decompressed.len() {
        let tag_byte = decompressed[i];
        i += 1;

        let tag_name = tag_byte_to_name(tag_byte);

        reconstructed_text.push('<');
        reconstructed_text.push_str(tag_name);
        reconstructed_text.push('>');

        while i < decompressed.len() {
            if i + 3 < decompressed.len()
                && decompressed[i] == 255
                && decompressed[i + 1] == 255
                && decompressed[i + 2] == 255
                && decompressed[i + 3] == 255
            {
                i += 4;
                break;
            }

            let len = decompressed[i] as usize;
            i += 1;

            if len == 0 {
                continue;
            }
            if i + len > decompressed.len() {
                break;
            }

            let slice = &decompressed[i..i + len];
            let word = resolve_token(slice);
            reconstructed_text.push_str(&word);
            reconstructed_text.push(' ');
            i += len;
        }

        reconstructed_text.push_str("</");
        reconstructed_text.push_str(tag_name);
        reconstructed_text.push_str(">\n");
    }

    reconstructed_text
}

pub fn decode_blob_to_tag_data(raw_db_value: &[u8]) -> HashMap<String, Vec<String>> {
    let mut tag_data: HashMap<String, Vec<String>> = HashMap::new();

    if raw_db_value.is_empty() {
        return tag_data;
    }

    let is_compressed = raw_db_value[0] == 1;
    let payload = &raw_db_value[1..];

    let decompressed_data = if is_compressed {
        match zstd::stream::decode_all(Cursor::new(payload)) {
            Ok(data) => data,
            Err(_) => return tag_data,
        }
    } else {
        payload.to_vec()
    };

    let decompressed = &decompressed_data;
    let mut i = 0;

    while i < decompressed.len() {
        let tag_byte = decompressed[i];
        i += 1;
        let tag_name = tag_byte_to_name(tag_byte).to_string();
        let mut current_word = String::new();

        while i < decompressed.len() {
            if i + 3 < decompressed.len()
                && decompressed[i] == 255
                && decompressed[i + 1] == 255
                && decompressed[i + 2] == 255
                && decompressed[i + 3] == 255
            {
                i += 4;
                break;
            }

            let len = decompressed[i] as usize;
            i += 1;

            if len == 0 {
                continue;
            }
            if i + len > decompressed.len() {
                break;
            }

            let slice = &decompressed[i..i + len];
            let word = resolve_token(slice);

            match tag_name.as_str() {
                "meta" | "a_href" | "img_src" => {
                    tag_data.entry(tag_name.clone()).or_default().push(word);
                }
                _ => {
                    if !current_word.is_empty() {
                        current_word.push(' ');
                    }
                    current_word.push_str(&word);
                }
            }

            i += len;
        }

        if !current_word.is_empty() {
            tag_data.entry(tag_name).or_default().push(current_word);
        }
    }

    tag_data
}
pub fn decode_blob_to_embed_text(raw_db_value: &[u8]) -> String {
    if raw_db_value.is_empty() {
        return String::new();
    }
    let is_compressed = raw_db_value[0] == 1;
    let payload = &raw_db_value[1..];
    let decompressed_data = if is_compressed {
        match zstd::stream::decode_all(Cursor::new(payload)) {
            Ok(data) => data,
            Err(_) => return String::new(),
        }
    } else {
        payload.to_vec()
    };
    let decompressed = &decompressed_data;
    let mut embed_text = String::with_capacity(decompressed.len() * 3);
    let mut i = 0;
    while i < decompressed.len() {
        let tag_byte = decompressed[i];
        i += 1;
        let tag_name = tag_byte_to_name(tag_byte);
        let skip_tag = matches!(tag_name, "meta" | "a_href" | "img_src");

        while i < decompressed.len() {
            if i + 3 < decompressed.len()
                && decompressed[i] == 255
                && decompressed[i + 1] == 255
                && decompressed[i + 2] == 255
                && decompressed[i + 3] == 255
            {
                i += 4;
                break;
            }
            let len = decompressed[i] as usize;
            i += 1;
            if len == 0 {
                continue;
            }
            if i + len > decompressed.len() {
                break;
            }
            let slice = &decompressed[i..i + len];
            if !skip_tag {
                let word = resolve_token(slice);
                embed_text.push_str(&word);
                embed_text.push(' ');
            }
            i += len;
        }
    }
    embed_text
}
fn tag_byte_to_name(tag_byte: u8) -> &'static str {
    match tag_byte {
        b'1' => "h1",
        b'2' => "h2",
        b'3' => "h3",
        b'4' => "h4",
        b'5' => "h5",
        b'6' => "h6",
        b's' => "span",
        b'p' => "p",
        b'a' => "a",
        b'l' => "li",
        b'b' => "label",
        b'm' => "meta",
        b'i' => "img_src",
        b'h' => "a_href",
        _ => "div",
    }
}

fn resolve_token(slice: &[u8]) -> String {
    if slice.len() > 3 {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        return search_word_by_id(id as usize);
    }

    let is_normal_short_word = slice.iter().all(|&b| {
        b.is_ascii_alphanumeric()
            || b == b'.'
            || b == b','
            || b == b'-'
            || b == b'!'
            || b == b'?'
            || b == b'\''
    }) && std::str::from_utf8(slice).is_ok();

    if is_normal_short_word {
        std::str::from_utf8(slice).unwrap().to_string()
    } else {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        search_word_by_id(id as usize)
    }
}
pub fn search_word_by_id(id: usize) -> String {
    if id < 256 {
        return String::new();
    }

    let start_offset = (id - 256) as u64;
    let db_path = "/root/crawler/prieco_crawler/dictionary/offset.db";

    DICT_FILE.with(|file_cell| {
        let mut borrow = file_cell.borrow_mut();

        if borrow.is_none() {
            match File::open(db_path) {
                Ok(f) => *borrow = Some(f),
                Err(e) => {
                    println!("Could not open dictionary! {}", e);
                    return String::new();
                }
            }
        }

        let file = borrow.as_ref().unwrap();
        let mut buffer = Vec::with_capacity(32);
        let mut current_offset = start_offset;
        let mut byte = [0u8; 1];

        loop {
            match file.read_at(&mut byte, current_offset) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == 0 {
                        break;
                    }
                    buffer.push(byte[0]);
                    current_offset += 1;
                }
                Err(e) => {
                    println!(
                        "Could not read dictionary at offset {}! {}",
                        current_offset, e
                    );
                    return String::new();
                }
            }
        }

        String::from_utf8(buffer).unwrap_or_default()
    })
}
fn extract_all_urls_from_text(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut current_idx = 0;

    while let Some(start_offset) = text[current_idx..].find("http") {
        let start = current_idx + start_offset;
        let mut raw_url = String::new();
        let mut end = start;

        for c in text[start..].chars() {
            if c == '"' || c == '\'' || c == '<' || c == '>' || c == '\n' || c == ']' {
                break;
            }
            if !c.is_whitespace() {
                raw_url.push(c);
            }
            end += c.len_utf8();
        }

        if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            urls.push(raw_url);
        }

        current_idx = end;
        if current_idx >= text.len() {
            break;
        }
    }

    urls
}
pub async fn attempt_url_recovery_parallel_dry_run() {
    println!(
        "{}Starting Parallel URL Recovery (DRY RUN)...{}",
        colors::BLUE,
        colors::RESET
    );
    let start_time = Instant::now();

    let mut tasks = Vec::new();
    let mut read_count = 0;

    for item in ORPHAN_STORAGE.iterator(rocksdb::IteratorMode::Start) {
        if let Ok((key_bytes, raw_blob)) = item {
            let key_arr: [u8; 8] = match key_bytes.as_ref().try_into() {
                Ok(arr) => arr,
                Err(_) => continue,
            };

            let blob_id = u64::from_le_bytes(key_arr);
            let blob_owned = raw_blob.to_vec();

            let task = tokio::task::spawn_blocking(move || {
                let html_text = decode_blob_to_text(&blob_owned);
                let potential_urls = extract_all_urls_from_text(&html_text);

                for url in potential_urls {
                    let clean = url.trim();

                    let variations = vec![
                        clean.to_string(),
                        format!("{}/", clean),
                        clean.trim_end_matches('/').to_string(),
                    ];

                    for var in variations {
                        if url_to_id(&var) == blob_id {
                            return Some((blob_id, var));
                        }
                    }
                }
                None
            });

            tasks.push(task);
            read_count += 1;

            if read_count >= 1000 {
                break;
            }
        }
    }

    let mut recovered_count = 0;

    for task in tasks {
        if let Ok(Some((id, found_url))) = task.await {
            recovered_count += 1;

            if recovered_count <= 5 {
                println!(
                    "{}SUCCESS:{} Match found for ID {}! \n  Recovered URL: {}",
                    colors::GREEN,
                    colors::RESET,
                    id,
                    found_url
                );
            }
        }
    }

    println!(
        "\n{}Dry Run Complete in {:.2?}!{}\nTotal Scanned: {}\nSuccessfully Recovered: {}",
        colors::GREEN,
        start_time.elapsed(),
        colors::RESET,
        read_count,
        recovered_count
    );
}
pub fn diagnostic_url_hash_mismatch() {
    println!(
        "{}--- STARTING MULTI-BLOB FORENSIC DIAGNOSTIC ---{}",
        colors::BLUE,
        colors::RESET
    );

    let mut tested_count = 0;

    for item in ORPHAN_STORAGE.iterator(rocksdb::IteratorMode::Start) {
        if let Ok((key_bytes, raw_blob)) = item {
            let key_arr: [u8; 8] = match key_bytes.as_ref().try_into() {
                Ok(arr) => arr,
                Err(_) => continue,
            };

            let target_blob_id = u64::from_le_bytes(key_arr);

            if target_blob_id == 0 {
                continue;
            }

            println!(
                "\n{}=================================================={}",
                colors::YELLOW,
                colors::RESET
            );
            println!(
                "{}TARGET BLOB ID TO MATCH: {}{}",
                colors::YELLOW,
                target_blob_id,
                colors::RESET
            );

            let html_text = decode_blob_to_text(&raw_blob);
            let potential_urls = extract_all_urls_from_text(&html_text);

            println!(
                "Extracted {} potential HTTP strings from this blob.",
                potential_urls.len()
            );

            if potential_urls.is_empty() {
                println!("No URLs found in this blob, skipping to the next one...");
                continue;
            }

            for (i, url) in potential_urls.iter().take(5).enumerate() {
                let clean = url.trim();
                println!(
                    "\n{}--- Testing Extracted String #{} ---{}",
                    colors::BLUE,
                    i,
                    colors::RESET
                );
                println!("Raw String: '{}'", clean);

                let variations = vec![
                    clean.to_string(),
                    format!("{}/", clean),
                    clean.trim_end_matches('/').to_string(),
                    clean.replace("https://", "http://"),
                    clean.replace("http://", "https://"),
                    clean
                        .replace("%20", " ")
                        .replace("%3A", ":")
                        .replace("%2F", "/"),
                ];

                let mut matched = false;
                for (v_idx, var) in variations.iter().enumerate() {
                    let generated_hash = url_to_id(var);

                    println!("  Variation {}: '{}'", v_idx, var);
                    println!("    Generated Hash: {:>20}", generated_hash);
                    println!("    Target Hash:    {:>20}", target_blob_id);

                    if generated_hash == target_blob_id {
                        println!(
                            "    {}*** PERFECT MATCH FOUND! ***{}",
                            colors::GREEN,
                            colors::RESET
                        );
                        matched = true;
                        break;
                    }
                }

                if matched {
                    break;
                }
            }

            tested_count += 1;
            if tested_count >= 5 {
                break;
            }
        }
    }

    println!(
        "\n{}--- FORENSIC DIAGNOSTIC COMPLETE ---{}",
        colors::BLUE,
        colors::RESET
    );
}
pub fn diagnostic_raw_decode() {
    println!(
        "{}--- STARTING RAW DECODE VERIFIER ---{}",
        colors::BLUE,
        colors::RESET
    );

    let mut iterator = ORPHAN_STORAGE.iterator(rocksdb::IteratorMode::Start);

    while let Some(Ok((key_bytes, raw_blob))) = iterator.next() {
        let key_arr: [u8; 8] = match key_bytes.as_ref().try_into() {
            Ok(arr) => arr,
            Err(_) => continue,
        };
        let target_blob_id = u64::from_le_bytes(key_arr);

        if target_blob_id == 0 {
            continue;
        }

        println!(
            "\n{}--- TESTING BLOB ID: {} ---{}",
            colors::YELLOW,
            target_blob_id,
            colors::RESET
        );

        println!("Raw payload size from RocksDB: {} bytes", raw_blob.len());
        if raw_blob.len() < 10 {
            println!("Payload is suspiciously small. Here are the raw bytes:");
            println!("{:?}", raw_blob.as_ref());
        }

        let html_text = decode_blob_to_text(&raw_blob);

        println!("Decoded string length: {} characters", html_text.len());

        if html_text.is_empty() {
            println!(
                "{}FATAL: decode_blob_to_text returned an empty string!{}",
                colors::RED,
                colors::RESET
            );
        } else {
            let preview_len = html_text.len().min(500);
            println!(
                "\n{}--- PREVIEW OF DECODED TEXT (First {} chars) ---{}",
                colors::BLUE,
                preview_len,
                colors::RESET
            );
            println!("{}", &html_text[..preview_len]);
            println!(
                "{}--------------------------------------------------{}",
                colors::BLUE,
                colors::RESET
            );
        }

        break;
    }
}
const SEARCHED_K1: u64 = 0xC70F6907A1C9566B;
const SEARCHED_K2: u64 = 0x85E4C66FC71D33EF;

thread_local! {
    static DICT_FILE: RefCell<Option<File>> = RefCell::new(None);
}
