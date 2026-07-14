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
    hash::{BuildHasher, Hash, Hasher},
    io::{BufWriter, Cursor, Read, Write},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

/*
  Import external libraries
*/
use fjall::PersistMode;
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use serde::Deserialize;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, BLOB_STORAGE, META_DICTIONARY, PRIECO_FJALL,
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
    let mut known_blobs = get_known_blob_ids();
    migrate_blob_to_fjall(&mut known_blobs);

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

        reconstructed_text.push_str(&format!("<{}>", tag_name));

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

        reconstructed_text.push_str(&format!("</{}>\n", tag_name));
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

const SEARCHED_K1: u64 = 0xC70F6907A1C9566B;
const SEARCHED_K2: u64 = 0x85E4C66FC71D33EF;

pub fn attempt_url_recovery(
    target_blob_id: u64,
    tag_data: &HashMap<String, Vec<String>>,
) -> Option<String> {
    let build_hasher = ahash::RandomState::with_seeds(SEARCHED_K1, SEARCHED_K2, 0, 0);

    let mut check_url = |candidate: &str| -> Option<String> {
        let clean = candidate.trim();
        if clean.is_empty() || !clean.starts_with("http") {
            return None;
        }

        let variations = [
            clean.to_string(),
            clean.trim_end_matches('/').to_string(),
            format!("{}/", clean.trim_end_matches('/')),
            clean.replace("http://", "https://"),
            clean.replace("https://", "http://"),
        ];

        for var in variations {
            let mut hasher = build_hasher.build_hasher();
            var.hash(&mut hasher);

            if hasher.finish() == target_blob_id {
                return Some(var);
            }
        }
        None
    };

    if let Some(metas) = tag_data.get("meta") {
        for meta in metas {
            if let Some(pos) = meta.find('=') {
                let url_candidate = &meta[pos + 1..];
                if let Some(matched) = check_url(url_candidate) {
                    return Some(matched);
                }
            }
        }
    }

    if let Some(links) = tag_data.get("a_href") {
        for link in links {
            if let Some(matched) = check_url(link) {
                return Some(matched);
            }
        }
    }

    None
}
thread_local! {
    static DICT_FILE: RefCell<Option<File>> = RefCell::new(None);
}
