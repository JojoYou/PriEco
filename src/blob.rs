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
    fs::{File, create_dir_all, read_dir, remove_dir_all, remove_file},
    io::Read,
    path::{Path, PathBuf},
};

/*
  Import external libraries
*/
use flate2::read::GzDecoder;
use tar::Archive;

/*
  Import own libraries
*/
use crate::globals::{BLOB_IMPORT_DIR, BLOB_STORAGE, colors, icons};

pub fn run() {
    match find_next_directory() {
        Some(dir_path) => {
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
            } else {
                // Successfully processed - remove the directory
                if let Err(e) = remove_dir_all(&dir_path) {
                    println!(
                        "{}{}: Rmoving directory: {:?} Error: {}{}",
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
        None => return,
    }
}

fn process_directory(dir_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Find all .tar.gz files in the directory
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
        println!("{}: Processing: {:?}", icons::BLOB, tar_path);

        // Open and decompress the tar.gz file
        let tar_file = File::open(&tar_path)?;
        let decompressor = GzDecoder::new(tar_file);
        let mut archive = Archive::new(decompressor);

        let mut files_inserted = 0;

        // Process each entry in the tar archive
        for entry_result in archive.entries()? {
            let mut entry = entry_result?;

            // Get the filename from the tar entry
            let path = entry.path()?;
            let file_name = path.to_str().ok_or("{}: Invalid filename")?;

            // Skip if it's a directory entry
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
                continue; // Skip files that aren't .zst or .txt
            }

            // Parse the blob ID from filename (stem without extension)
            let name: u64 = Path::new(file_name)
                .file_name()
                .and_then(|f| f.to_str())
                .and_then(|s| Path::new(s).file_stem())
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse().ok())
                .ok_or("{}: Invalid blob ID in filename")?;

            // Determine compression flag from extension
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

            // Read file content from tar entry
            buffer.clear();
            buffer.push(flag); // Prepend flag byte
            entry.read_to_end(&mut buffer)?;

            // Insert into RocksDB
            BLOB_STORAGE.put(name.to_le_bytes(), &buffer)?;
            files_inserted += 1;

            if files_inserted % 1000 == 0 {
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

        // Flush after each tar file
        println!("{}: Flushing!", icons::BLOB,);
        BLOB_STORAGE.flush()?;
        println!(
            "{}: {}Flushed!{}",
            icons::BLOB,
            colors::GREEN,
            colors::RESET
        );

        // Remove the processed tar.gz file
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
fn find_next_directory() -> Option<PathBuf> {
    let watch_path = Path::new(BLOB_IMPORT_DIR);
    if !watch_path.exists() {
        let _ = create_dir_all(watch_path);
        return None;
    }
    if let Ok(entries) = read_dir(watch_path) {
        return entries
            .filter_map(|e| e.ok())
            .find(|e| e.path().is_dir())
            .map(|e| e.path());
    }
    None
}
