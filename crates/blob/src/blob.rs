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
    fs::{self, File, read_dir, remove_file},
    io::{Cursor, Read, Write},
    os::{fd::AsRawFd, unix::fs::FileExt},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

/*
  Import external libraries
*/
use fjall::PersistMode;
use flate2::read::GzDecoder;
use io_uring::{IoUring, opcode, types};
use moka::sync::Cache;
use once_cell::sync::Lazy;
use rand::{Rng, rng};
use rayon::prelude::*;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    META_DICTIONARY, PRIECO_FJALL, WebDocument,
    globals::{colors, icons},
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
    feed_blobs_for_reembedding();
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
/*fn find_all_directories() -> Vec<PathBuf> {
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
*/

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

    let mut batch = Vec::with_capacity(BATCH_SIZE + 100);
    let mut last_meta_id = 0u64;

    let io_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(128)
        .build()
        .expect("Failed to build custom thread pool");

    loop {
        let mut paused_for_space = false;
        while full_dir("/mnt/vec/") {
            if !paused_for_space {
                println!("Paused: /mnt/vec/");
                paused_for_space = true;
            }
            thread::sleep(Duration::from_secs(2));
        }

        if paused_for_space {
            println!("Resuming: Space freed up in /mnt/vec/");
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

        let chunk_size = 50;

        let chunk_results: Vec<(u64, [u64; 6], [u128; 9], [u64; 2])> = io_pool.install(|| {
            batch
                .par_chunks(chunk_size)
                .map_init(
                    || {
                        zstd::bulk::Decompressor::with_dictionary(&META_DICTIONARY)
                            .expect("Failed to init zstd")
                    },
                    |decompressor, chunk| {
                        let mut l_fed = 0;
                        let mut l_skips = [0u64; 6];
                        let mut l_times = [0u128; 9];
                        let mut l_cache_hits = 0u64;
                        let mut l_cache_misses = 0u64;

                        struct BlobPayload {
                            meta_id: u64,
                            doc: WebDocument,
                            decompressed_blob: Vec<u8>,
                        }

                        let mut blob_payloads: Vec<BlobPayload> = Vec::with_capacity(chunk.len());
                        let mut chunk_missing_dict_ids: Vec<usize> = Vec::new();

                        for (key, val) in chunk {
                            let t = Instant::now();
                            let decompressed =
                                match decompressor.decompress(val.as_ref(), 10_000_000) {
                                    Ok(d) => d,
                                    Err(_) => {
                                        l_skips[0] += 1;
                                        continue;
                                    }
                                };
                            l_times[0] += t.elapsed().as_micros();

                            let t = Instant::now();
                            let doc: WebDocument = match serde_json::from_slice(&decompressed) {
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

                            let t_ssd = Instant::now();
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
                            l_times[2] += t_ssd.elapsed().as_micros();

                            let is_compressed = raw_blob[0] == 1;
                            let payload = &raw_blob[1..];
                            let t_zstd = Instant::now();
                            let decompressed_blob = if is_compressed {
                                match zstd::stream::decode_all(std::io::Cursor::new(payload)) {
                                    Ok(data) => data,
                                    Err(_) => continue,
                                }
                            } else {
                                payload.to_vec()
                            };
                            l_times[3] += t_zstd.elapsed().as_micros();

                            let t_collect = Instant::now();
                            collect_missing_ids(
                                &decompressed_blob,
                                &mut chunk_missing_dict_ids,
                                &mut l_cache_hits,
                                &mut l_cache_misses,
                            );
                            l_times[4] += t_collect.elapsed().as_micros();

                            blob_payloads.push(BlobPayload {
                                meta_id,
                                doc,
                                decompressed_blob,
                            });
                        }

                        let t_dict = Instant::now();
                        bulk_prefetch_missing_words(chunk_missing_dict_ids);
                        l_times[5] += t_dict.elapsed().as_micros();

                        for payload in blob_payloads {
                            let (embed_text, links_text, has_500_words, is_mobile, b_str, _, _) =
                                decode_decompressed_to_embed_text(&payload.decompressed_blob);
                            l_times[6] += b_str;

                            let t_format = Instant::now();
                            let result_str = format!(
                                "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                                payload.doc.url,
                                payload.doc.title,
                                payload.meta_id,
                                payload.doc.lang,
                                embed_text,
                                links_text,
                                has_500_words,
                                is_mobile
                            );
                            l_times[7] += t_format.elapsed().as_micros();

                            let t_write = Instant::now();
                            write_to_random_file("/mnt/vec/", &result_str);
                            l_times[8] += t_write.elapsed().as_micros();

                            l_fed += 1;
                        }

                        (l_fed, l_skips, l_times, [l_cache_hits, l_cache_misses])
                    },
                )
                .collect()
        });

        let mut batch_fed = 0;
        let mut skips = [0u64; 6];
        let mut times = [0u128; 9];
        let mut cache_stats = [0u64; 2];

        for (f, sk, tm, cs) in chunk_results {
            batch_fed += f;
            for i in 0..6 {
                skips[i] += sk[i];
            }
            for i in 0..9 {
                times[i] += tm[i];
            }
            cache_stats[0] += cs[0];
            cache_stats[1] += cs[1];
        }

        fed += batch_fed;
        total_skipped += skips.iter().sum::<u64>();

        if let Some((last_key, _)) = batch.last() {
            if let Ok(id_bytes) = <[u8; 8]>::try_from(last_key.as_ref()) {
                last_meta_id = u64::from_be_bytes(id_bytes);
                let _ = fs::write(RESUME_FILE, last_meta_id.to_string());
            }
        }

        let total_lookups = cache_stats[0] + cache_stats[1];
        let hit_rate = if total_lookups > 0 {
            (cache_stats[0] as f64 / total_lookups as f64) * 100.0
        } else {
            0.0
        };

        batch.clear();
        println!(
            "✅ Fed: {} | ❌ Skipped: {} | Last ID: {}",
            fed, total_skipped, last_meta_id
        );
        println!(
            "⚠️ Skips -> Meta:{} JSON:{} BadID:{} DBMiss:{} Empty:{}",
            skips[0], skips[1], skips[2], skips[3], skips[4]
        );

        println!(
            "⏱️ CPU(ms) -> MetaZstd: {:.0} | JSON: {:.0} | BlobZstd: {:.0} | CollectIDs: {:.0} | DecodeText: {:.0} | FormatStr: {:.0}",
            times[0] as f64 / 1000.0,
            times[1] as f64 / 1000.0,
            times[3] as f64 / 1000.0,
            times[4] as f64 / 1000.0,
            times[6] as f64 / 1000.0,
            times[7] as f64 / 1000.0
        );
        println!(
            "⏱️ I/O(ms) -> FjallFetch: {:.0} | DictFetch: {:.0} | Write: {:.0}",
            times[2] as f64 / 1000.0,
            times[5] as f64 / 1000.0,
            times[8] as f64 / 1000.0
        );
        println!(
            "🧠 Cache -> Hits: {} | Misses: {} | Hit Rate: {:.2}%",
            cache_stats[0], cache_stats[1], hit_rate
        );
    }
}
fn collect_missing_ids(
    decompressed: &[u8],
    missing_ids: &mut Vec<usize>,
    cache_hits: &mut u64,
    cache_misses: &mut u64,
) {
    let mut i = 0;
    let mut simulated_embed_len = 0;
    let mut p_word_count = 0;

    'outer: while i < decompressed.len() {
        if i > 30_000 {
            break 'outer;
        }

        let tag_byte = decompressed[i];
        i += 1;
        let tag_name = tag_byte_to_name(tag_byte);

        let skip_tag = matches!(tag_name, "meta" | "img_src" | "a_href");
        let is_p = tag_name == "p";
        let is_meta = tag_name == "meta";

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

            if is_p {
                p_word_count += 1;
            }

            if simulated_embed_len >= 600 && p_word_count >= 500 {
                break 'outer;
            }

            if tag_name == "img_src" || tag_name == "a_href" {
                i += len;
                continue;
            }

            if !is_meta && simulated_embed_len >= 600 {
                i += len;
                continue;
            }

            if slice.len() > 3
                || !slice.iter().all(|&b| {
                    b.is_ascii_alphanumeric()
                        || b == b'.'
                        || b == b','
                        || b == b'-'
                        || b == b'!'
                        || b == b'?'
                        || b == b'\''
                })
            {
                let mut id = 0u64;
                for (j, b) in slice.iter().enumerate() {
                    id |= (*b as u64) << (8 * j);
                }
                let id_usize = id as usize;

                if id_usize >= 256 {
                    if DICT_CACHE.contains_key(&id_usize) {
                        *cache_hits += 1;
                    } else {
                        *cache_misses += 1;
                        missing_ids.push(id_usize);
                    }
                }
            }

            if !skip_tag {
                simulated_embed_len += len + 1;
            }

            i += len;
        }
    }
}

const MAX_WORD_LEN: u32 = 256;

fn bulk_prefetch_missing_words(mut missing_ids: Vec<usize>) {
    if missing_ids.is_empty() {
        return;
    }

    missing_ids.sort_unstable();
    missing_ids.dedup();

    let db_path = "/root/crawler/prieco_crawler/dictionary/offset.db";

    DICT_FILE.with(|file_cell| {
        let mut borrow = file_cell.borrow_mut();

        if borrow.is_none() {
            if let Ok(file) = File::open(db_path) {
                unsafe {
                    libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_RANDOM);
                }
                *borrow = Some(file);
            }
        }

        let file = borrow.as_ref().unwrap();
        let fd = types::Fd(file.as_raw_fd());

        RING.with(|ring_cell| {
            let mut ring = ring_cell.borrow_mut();

            for chunk in missing_ids.chunks(256) {
                let mut buffers = vec![[0u8; MAX_WORD_LEN as usize]; chunk.len()];

                {
                    let mut sq = ring.submission();
                    for (idx, &id) in chunk.iter().enumerate() {
                        let offset = (id - 256) as u64;
                        let buf_ptr = buffers[idx].as_mut_ptr();

                        let read_e = opcode::Read::new(fd, buf_ptr, MAX_WORD_LEN)
                            .offset(offset)
                            .build()
                            .user_data(idx as u64);

                        unsafe {
                            sq.push(&read_e).unwrap();
                        }
                    }
                    sq.sync();
                }

                ring.submit_and_wait(chunk.len()).unwrap();

                {
                    let cq = ring.completion();
                    for cqe in cq {
                        let idx = cqe.user_data() as usize;
                        let id = chunk[idx];
                        let buffer = &buffers[idx];
                        let bytes_read = cqe.result();

                        if bytes_read > 0 {
                            let bytes_read = bytes_read as usize;
                            let word = if let Some(null_pos) =
                                buffer[..bytes_read].iter().position(|&b| b == 0)
                            {
                                String::from_utf8_lossy(&buffer[..null_pos]).into_owned()
                            } else {
                                String::from_utf8_lossy(&buffer[..bytes_read]).into_owned()
                            };
                            DICT_CACHE.insert(id, word);
                        }
                    }
                }
            }
        });
    });
}

pub fn decode_decompressed_to_embed_text(
    decompressed: &[u8],
) -> (String, String, u8, u8, u128, u64, u64) {
    if decompressed.is_empty() {
        return (String::new(), String::new(), 0, 0, 0, 0, 0);
    }

    let mut embed_text = String::with_capacity(1024);
    let mut p_word_count = 0;
    let mut is_mobile = 0;

    let mut cache_hits = 0u64;
    let mut cache_misses = 0u64;

    let t_loop = std::time::Instant::now();
    let mut i = 0;

    'outer: while i < decompressed.len() {
        if i > 30_000 {
            break 'outer;
        }

        let tag_byte = decompressed[i];
        i += 1;
        let tag_name = tag_byte_to_name(tag_byte);

        let skip_tag = matches!(tag_name, "meta" | "img_src" | "a_href");
        let is_p = tag_name == "p";
        let is_meta = tag_name == "meta";

        let mut current_meta_content = String::new();

        while i < decompressed.len() {
            if i + 3 < decompressed.len()
                && decompressed[i] == 255
                && decompressed[i + 1] == 255
                && decompressed[i + 2] == 255
                && decompressed[i + 3] == 255
            {
                i += 4;
                if is_meta {
                    let clean_meta = current_meta_content
                        .to_lowercase()
                        .replace(' ', "")
                        .replace('"', "")
                        .replace('\'', "");

                    if clean_meta.contains("width=device-width")
                        || clean_meta.contains("device-width")
                    {
                        is_mobile = 1;
                    }
                }
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

            if is_p {
                p_word_count += 1;
            }

            if embed_text.len() >= 600 && p_word_count >= 500 {
                break 'outer;
            }

            if tag_name == "img_src" || tag_name == "a_href" {
                i += len;
                continue;
            }

            if is_meta && is_mobile == 1 {
                i += len;
                continue;
            }

            if !is_meta && embed_text.len() >= 600 {
                i += len;
                continue;
            }

            let (word, hit_cache) = resolve_token(slice);

            if hit_cache {
                cache_hits += 1;
            } else {
                let mut id = 0u64;
                for (j, b) in slice.iter().enumerate() {
                    id |= (*b as u64) << (8 * j);
                }
                if id >= 256 {
                    cache_misses += 1;
                }
            }

            if is_meta {
                current_meta_content.push_str(&word);
                current_meta_content.push(' ');
            }

            if !skip_tag {
                embed_text.push_str(&word);
                embed_text.push(' ');
            }

            i += len;
        }
    }

    let string_time_us = t_loop.elapsed().as_micros();
    let has_500_words = if p_word_count >= 500 { 1 } else { 0 };

    (
        embed_text,
        String::new(),
        has_500_words,
        is_mobile,
        string_time_us,
        cache_hits,
        cache_misses,
    )
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
            let (word, hit_cache) = resolve_token(slice);
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
fn resolve_token(slice: &[u8]) -> (String, bool) {
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
        (std::str::from_utf8(slice).unwrap().to_string(), true)
    } else {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        search_word_by_id(id as usize)
    }
}

static DICT_CACHE: Lazy<Cache<usize, String>> =
    Lazy::new(|| Cache::builder().max_capacity(50_000_000).build());

thread_local! {
    static DICT_FILE: RefCell<Option<File>> = RefCell::new(None);
    static RING: RefCell<IoUring> = RefCell::new(IoUring::new(256).expect("Failed to init io_uring"));
}

pub fn search_word_by_id(id: usize) -> (String, bool) {
    if id < 256 {
        return (String::new(), true);
    }

    if let Some(cached_word) = DICT_CACHE.get(&id) {
        return (cached_word, true);
    }

    let mut current_offset = (id - 256) as u64;
    let db_path = "/root/crawler/prieco_crawler/dictionary/offset.db";

    let word = DICT_FILE.with(|file_cell| {
        let mut borrow = file_cell.borrow_mut();

        if borrow.is_none() {
            match File::open(db_path) {
                Ok(f) => *borrow = Some(f),
                Err(_) => return String::new(),
            }
        }

        let file = borrow.as_ref().unwrap();
        let mut result_bytes = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            if result_bytes.len() > 8192 {
                break;
            }

            match file.read_at(&mut buffer, current_offset) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    if let Some(null_pos) = buffer[..bytes_read].iter().position(|&b| b == 0) {
                        result_bytes.extend_from_slice(&buffer[..null_pos]);
                        break;
                    } else {
                        result_bytes.extend_from_slice(&buffer[..bytes_read]);
                        current_offset += bytes_read as u64;
                    }
                }
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&result_bytes).into_owned()
    });

    DICT_CACHE.insert(id, word.clone());

    (word, false)
}

/* Helper functions */
fn full_dir(path: &str) -> bool {
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let c_path = std::ffi::CString::new(path).unwrap();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let total_blocks = stat.f_blocks as f64;
            let available_blocks = stat.f_bavail as f64;
            let used_blocks = total_blocks - available_blocks;
            let percent_used = used_blocks / total_blocks;

            if percent_used >= 0.95 {
                true
            } else if percent_used >= 0.9 {
                rand::rng().random_bool(0.1)
            } else {
                false
            }
        } else {
            false
        }
    }
}

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
