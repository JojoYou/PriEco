/*
  File: blob/blob.rs
  Description:

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-02-07
  Last Modified: 2026-02-07

  Usage: Run() to take archived htmls and insert them into Blob storage
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    error::Error,
    fs::{File, create_dir_all, read_dir, remove_dir_all, remove_file},
    io::Read,
    path::{Path, PathBuf},
};

use fjall::{Keyspace, PersistMode};
/*
  Import external libraries
*/
use flate2::read::GzDecoder;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, BLOB_STORAGE, META_DECODER, PRIECO_FJALL, TANTIVY_INDEX, TANTIVY_INDEX2,
    TANTIVY_WRITER, TANTIVY_WRITER2, WebDocument,
    globals::{colors, icons},
    url_to_domain_id,
};

pub fn run() {
    /*println!("Migrating!");
    migrate_blob_to_fjall();
    println!(
        "{}Migration to BLOB completed!{}",
        colors::GREEN,
        colors::RESET
    );
    println!("Compacting blobs");
    PRIECO_FJALL
        .blobs
        .major_compact()
        .expect("Failed to run major compaction");

    println!("Migrating tantivy");
    if let Err(e) = rebuild_tantivy_index_v2() {
        println!("Tantivy rebuild: {}", e);
    };*/
    println!("Merging tantivy");
    force_merge_index();
    println!("All blob operations are done!");

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

    for tar_path in tar_files {
        // Create batch
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
            buffer.push(flag); // Prepend flag byte
            entry.read_to_end(&mut buffer)?;

            batch.insert(&PRIECO_FJALL.blobs_ks, name.to_le_bytes(), buffer.clone());
            files_inserted += 1;

            if files_inserted % 1000 == 0 {
                batch.commit();
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

                if PRIECO_FJALL
                    .meta_ks
                    .get(&name.to_le_bytes())
                    .unwrap()
                    .is_none()
                {
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
        batch.commit();
        PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll);

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
const BLOB_V2_PROGRESS_FILE: &str = "blob_v2_migration_progress.bin";

pub fn migrate_blob_to_fjall() {
    let last_migrated_key: Option<[u8; 8]> = std::fs::read(BLOB_V2_PROGRESS_FILE)
        .ok()
        .and_then(|buf| buf.try_into().ok());

    let iter_mode = match &last_migrated_key {
        Some(key) => rocksdb::IteratorMode::From(key, rocksdb::Direction::Forward),
        None => rocksdb::IteratorMode::Start,
    };

    println!(
        "{}Starting RocksDB -> Fjall (Blob) migration!{}",
        colors::BLUE,
        colors::RESET
    );

    let mut count: u64 = 0;
    let mut last_key: Option<[u8; 8]> = None;
    let mut skip_first = last_migrated_key.is_some();

    for item in BLOB_STORAGE.iterator(iter_mode) {
        let (key, value) = item.unwrap();
        count += 1;

        if skip_first {
            skip_first = false;
            continue;
        }

        let key_arr: [u8; 8] = key.as_ref().try_into().unwrap();

        PRIECO_FJALL.blobs_ks.insert(key_arr, &*value).unwrap();

        last_key = Some(key_arr);

        if count % 10_000 == 0 {
            if let Some(k) = last_key {
                std::fs::write(BLOB_V2_PROGRESS_FILE, k).unwrap();
            }

            println!("{}Written!{} {}", colors::BLUE, colors::RESET, count);
        }
    }

    if let Some(k) = last_key {
        std::fs::write(BLOB_V2_PROGRESS_FILE, k).unwrap();
    }

    PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll).unwrap();

    println!(
        "{}: {}Migration complete: {} entries migrated{}",
        icons::BLOB,
        colors::GREEN,
        count,
        colors::RESET
    );
}

pub fn rebuild_tantivy_index_v2() -> Result<(), Box<dyn std::error::Error>> {
    let schema = TANTIVY_INDEX2.schema();
    let doc_id_field = schema.get_field("doc_id")?;
    let domain_id_field = schema.get_field("domain_id")?;
    let title_field = schema.get_field("title")?;
    let description_field = schema.get_field("description")?;
    let content_field = schema.get_field("content")?;
    let keywords_field = schema.get_field("keywords")?;
    let lang_field = schema.get_field("lang")?;
    let loc_field = schema.get_field("loc")?;
    let date_field = schema.get_field("date")?;
    let safe_s_field = schema.get_field("safe_s")?;

    let mut writer = TANTIVY_WRITER2.lock();
    let mut count: u64 = 0;

    for guard in PRIECO_FJALL.meta_ks.iter() {
        let (key, compressed) = guard.into_inner()?;
        let id = u64::from_be_bytes(key.as_ref().try_into().expect("meta key is not 8 bytes"));

        let mut decoder = zstd::stream::read::Decoder::with_prepared_dictionary(
            compressed.as_ref(),
            &META_DECODER,
        )?;
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        let doc: WebDocument = serde_json::from_slice(&raw)?;
        let domain_id = url_to_domain_id(&doc.url);

        writer.add_document(tantivy::doc!(
            doc_id_field => id,
            domain_id_field => domain_id,
            title_field => doc.title.clone(),
            description_field => doc.description.clone(),
            content_field => doc.content.clone(),
            keywords_field => doc.keywords.clone(),
            lang_field => doc.lang.clone(),
            loc_field => doc.loc.clone(),
            date_field => doc.date,
            safe_s_field => doc.safe_s
        ))?;

        count += 1;
        if count % 250_000 == 0 {
            writer.commit()?;
            println!("Indexed {count} documents...");
        }
    }

    writer.commit()?;
    println!("Done. Indexed {count} documents into Tantivy v2.");
    Ok(())
}

pub fn force_merge_index() {
    println!("Preparing to merge index...");

    // 1. Lock the writer
    let mut writer = TANTIVY_WRITER2.lock();

    // 2. Commit any pending uncommitted documents first
    writer.commit().expect("Failed to commit pending documents");

    // 3. Fetch all current segment IDs from the index
    let segment_ids = TANTIVY_INDEX2
        .searchable_segment_ids()
        .expect("Failed to get searchable segment IDs");

    println!("Found {} segments. Starting merge...", segment_ids.len());

    // 4. Execute the merge and wait for it to finish
    // Note: .wait() is required as merge() returns a Future in Tantivy
    match writer.merge(&segment_ids).wait() {
        Ok(_) => println!("Merge completed successfully!"),
        Err(e) => eprintln!("Merge failed: {}", e),
    }
}
