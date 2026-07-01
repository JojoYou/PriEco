/*
  File: blob/blob.rs
  Description:

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-02-07
  Last Modified: 2026-02-07

  Usage: Run() to take archived htmls and insert them into RocksDB
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    fs::{File, OpenOptions, create_dir_all, read_dir, remove_dir_all, remove_file},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/*
  Import external libraries
*/
use flate2::read::GzDecoder;
use rocksdb::{Direction, IteratorMode};
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, BLOB_STORAGE, LMDB_BLOB_STORAGE, META_STORAGE, ROCKSDB_INDEX,
    globals::{colors, icons},
};
use zstd::bulk::Compressor;

const MIGRATION_CHECKPOINT: &str = "/mnt/hdd/blobs/.migration_checkpoint";

pub fn run() {
    println!("Migrating!");
    if let Err(e) = migrate_rocksdb_to_lmdb_meta() {
        println!("{}Failed to migrate!{} {}", colors::RED, colors::RESET, e);
    };
    println!(
        "{}Migration to LMDB completed!{}",
        colors::GREEN,
        colors::RESET
    );

    let directories = find_all_directories();

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

        if let Err(e) = process_directory(&dir_path) {
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
    }
}

fn process_directory(dir_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut write_opts = rocksdb::WriteOptions::default();
    write_opts.disable_wal(true);

    for tar_path in tar_files {
        println!("{}: Processing: {:?}", icons::BLOB, tar_path);

        let tar_file = File::open(&tar_path)?;
        let decompressor = GzDecoder::new(tar_file);
        let mut archive = Archive::new(decompressor);

        let mut files_inserted = 0;
        let mut batch = rocksdb::WriteBatch::default();

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
            buffer.push(flag); // Prepend flag byte
            entry.read_to_end(&mut buffer)?;

            batch.put(name.to_le_bytes(), &buffer);
            files_inserted += 1;

            if files_inserted % 1000 == 0 {
                BLOB_STORAGE.write_opt(batch, &write_opts)?;
                batch = rocksdb::WriteBatch::default();

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

                if let Err(e) = BLOB_STORAGE.get(name.to_le_bytes()) {
                    println!(
                        "{}: {}INTEGRITY CHECK FAILED for key {}! Error:{} {}",
                        icons::BLOB,
                        colors::RED,
                        name,
                        e,
                        colors::RESET
                    );
                    return Err(Box::new(e));
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        if !batch.is_empty() {
            BLOB_STORAGE.write_opt(batch, &write_opts)?;
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
        BLOB_STORAGE.flush()?;
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

/* Temp */
pub fn migrate_rocksdb_to_lmdb_blob() {
    let checkpoint = std::fs::read(MIGRATION_CHECKPOINT).ok();

    let iter = match &checkpoint {
        Some(last_key) => {
            println!(
                "{}: Resuming migration from checkpoint ({} bytes key)",
                icons::BLOB,
                last_key.len()
            );

            BLOB_STORAGE.iterator(IteratorMode::From(last_key, rocksdb::Direction::Forward))
        }
        None => {
            println!("{}: Starting fresh migration", icons::BLOB);
            BLOB_STORAGE.iterator(IteratorMode::Start)
        }
    };

    let mut count = 0u64;
    let mut skipped = 0u64;
    let mut last_key: Option<Box<[u8]>> = None;
    let mut wtxn = LMDB_BLOB_STORAGE.env.write_txn().unwrap();
    let mut first = checkpoint.is_some();

    for item in iter {
        let (key, value) = item.unwrap();

        if first {
            first = false;
            skipped += 1;
            continue;
        }

        LMDB_BLOB_STORAGE.db.put(&mut wtxn, &*key, &*value).unwrap();

        last_key = Some(key);
        count += 1;

        if count % 10_000 == 0 {
            wtxn.commit().unwrap();

            if let Some(ref k) = last_key {
                std::fs::write(MIGRATION_CHECKPOINT, k.as_ref()).unwrap();
            }

            wtxn = LMDB_BLOB_STORAGE.env.write_txn().unwrap();

            println!(
                "{}: Migrated {} entries (skipped {} on resume)",
                icons::BLOB,
                count,
                skipped
            );
        }
    }

    wtxn.commit().unwrap();
    if let Some(ref k) = last_key {
        std::fs::write(MIGRATION_CHECKPOINT, k.as_ref()).unwrap();
    }

    println!(
        "{}: {}Migration complete: {} entries migrated{}",
        icons::BLOB,
        colors::GREEN,
        count,
        colors::RESET
    );
}

const PROGRESS_FILE: &str = "migration_progress.bin";
const DICT_FILE: &str = "prieco_zstd.dict";
const DICT_SIZE: usize = 112_640;
const SAMPLES_TO_TRAIN: usize = 100_000;
const ZSTD_LEVEL: i32 = 3;

pub fn migrate_rocksdb_to_lmdb_meta() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "{}: Starting migration from RocksDB to LMDB (with Zstd Dictionary)...",
        icons::DB_INSERT
    );

    let dictionary = if Path::new(DICT_FILE).exists() {
        println!(
            "{}: Loading existing Zstd dictionary from disk...",
            icons::DB_INSERT
        );
        std::fs::read(DICT_FILE)?
    } else {
        println!(
            "{}: Training new Zstd dictionary on {} samples. This takes a moment...",
            icons::DB_INSERT,
            SAMPLES_TO_TRAIN
        );

        let mut samples_data = Vec::new();
        let mut sample_sizes = Vec::new();
        let iter = ROCKSDB_INDEX.iterator(IteratorMode::Start);

        for item in iter.take(SAMPLES_TO_TRAIN) {
            let (_, value) = item?;
            samples_data.extend_from_slice(&value);
            sample_sizes.push(value.len());
        }

        let dict = zstd::dict::from_continuous(&samples_data, &sample_sizes, DICT_SIZE)?;
        let mut file = File::create(DICT_FILE)?;
        file.write_all(&dict)?;
        println!(
            "{}: Dictionary trained and saved to {}",
            icons::DB_INSERT,
            DICT_FILE
        );
        dict
    };

    let mut compressor = Compressor::with_dictionary(ZSTD_LEVEL, &dictionary)?;

    let mut last_committed_key: Option<Vec<u8>> = None;
    if Path::new(PROGRESS_FILE).exists() {
        let mut file = File::open(PROGRESS_FILE)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        if !buf.is_empty() {
            last_committed_key = Some(buf);
            println!(
                "{}: Resuming migration from last saved checkpoint.",
                icons::DB_INSERT
            );
        }
    }

    let iter_mode = match &last_committed_key {
        Some(key) => IteratorMode::From(key, Direction::Forward),
        None => IteratorMode::Start,
    };

    let mut wtxn = META_STORAGE.env.write_txn()?;
    let mut count = 0;
    let mut batch_last_key: Option<Vec<u8>> = None;

    let iter = ROCKSDB_INDEX.iterator(iter_mode);

    for item in iter {
        let (key, value) = item?;

        if count == 0 {
            if let Some(ref last_key) = last_committed_key {
                if key.as_ref() == last_key.as_slice() {
                    continue;
                }
            }
        }

        let compressed_value = compressor.compress(value.as_ref())?;

        META_STORAGE
            .db
            .put(&mut wtxn, key.as_ref(), &compressed_value)?;

        batch_last_key = Some(key.to_vec());
        count += 1;

        if count % 100_000 == 0 {
            wtxn.commit()?;

            if let Some(ref exact_key) = batch_last_key {
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(PROGRESS_FILE)?;
                file.write_all(exact_key)?;
                file.flush()?;
            }

            println!(
                "{}: Migrated, compressed, and committed {} records...",
                icons::DB_INSERT,
                count
            );
            wtxn = META_STORAGE.env.write_txn()?;
        }
    }

    wtxn.commit()?;
    META_STORAGE.env.force_sync()?;

    if Path::new(PROGRESS_FILE).exists() {
        remove_file(PROGRESS_FILE)?;
    }

    println!(
        "{}: Migration complete! Total records added in this run: {}",
        icons::DB_INSERT,
        count
    );
    Ok(())
}
