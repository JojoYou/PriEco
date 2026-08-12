//! # Blob Storage Inserter
//!
//! Automatically inserts new htmls from '.tar.gz' files to [PRIECO_FJALL] blob storage.
//!
//! ## Architecture
//!
//! 1. [**Scan**:][find_all_directories] Finds all subdirectories in [BLOB_IMPORT_DIR].
//! 2. [**Get**:][get_dir_files] Get all '.tar.gz' files in directory.
//! 3. [**Process**:][process_file] Process individual .
//!     * Streams files from '.tar.gz'.
//!     * Accepts only '.zst' and '.txt' file streams.
//!     * Inserts the data to [PRIECO_FJALL] blob storage.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-02-07
//! * Last Modified: 2026-08-11
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{
    fs::{File, create_dir_all, metadata, read_dir, remove_dir_all, remove_file},
    io::{self, Error, ErrorKind, Read},
    path::{Path, PathBuf},
};

/*
  Import external libraries
*/
use fjall::PersistMode;
use flate2::read::GzDecoder;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, PRIECO_FJALL, file_exists,
    globals::{colors, icons},
};

/// Calls and coordinates functions to insert blobs.
///
/// This function calls in order [find_all_directories] to get all subdirs to import.
///
/// [get_dir_files] to get all '.tar.gz' files in individual subdirs (1 by 1).
///
/// [process_file] on each '.tar.gz' file to stream its content, create blobs and insert them to storage.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// None
///
/// # Failure Handling
///
/// This function doesn't error out.
/// Rather it manages out functions and if they 'Error', this function 'return' to
/// prevent data loss.
///
/// # Panics
///
/// Only if system runs out of memory.
pub fn run() {
    // Get all import dirs
    let directories = find_all_directories();
    if directories.is_empty() {
        return;
    }

    // Process dirs
    for dir_path in directories {
        println!(
            "{}{}: Processing: {:?}{}",
            icons::BLOB,
            colors::GREEN,
            dir_path,
            colors::RESET,
        );

        // Get files
        let tar_files = get_dir_files(&dir_path);
        println!(
            "{}: Found {} tar.gz files to process",
            icons::BLOB,
            tar_files.len()
        );
        if tar_files.is_empty() {
            return;
        }

        // Reusable memory to hold a single blob while we are creating it
        let mut single_blob_buffer: Vec<u8> = Vec::with_capacity(10 * 1024 * 1024);

        // Process tar files
        for tar_path in tar_files {
            println!("{}: Processing: {:?}", icons::BLOB, tar_path);

            if let Err(e) = process_file(&tar_path, &mut single_blob_buffer) {
                eprintln!(
                    "{}{}: Processing {:?} failed!{} {}",
                    icons::BLOB,
                    colors::RED,
                    tar_path,
                    colors::RESET,
                    e
                );

                return;
            };
        }

        if let Err(e) = remove_dir_all(&dir_path) {
            eprintln!(
                "{}{}: Removing directory: {:?} Error: {}{}",
                icons::BLOB,
                colors::RED,
                dir_path,
                colors::RESET,
                e
            );
        }

        println!(
            "{}{}: Successfully processed and removed: {:?}{}",
            icons::BLOB,
            colors::GREEN,
            dir_path,
            colors::RESET,
        );
    }
}

/* Helper functions */

/// Finds all subdirectories in [BLOB_IMPORT_DIR].
///
/// This function reads [BLOB_IMPORT_DIR] and filters entries to only directories.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// A vector of dir paths.
///
/// If there is none or the [BLOB_IMPORT_DIR] doesn't even exists,
/// it cretes it, and returns an empty vector.
///
/// # Panics
///
/// Only if system runs out of memory.
fn find_all_directories() -> Vec<PathBuf> {
    let path = Path::new(BLOB_IMPORT_DIR);
    if !path.exists() {
        let _ = create_dir_all(path);
        return Vec::new();
    }

    read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_else(|_| Vec::new())
}

/// Finds all '.tar.gz' files in specified directory.
///
/// This function reads specified directory and filters entries to only .gz files.
///
/// # Arguments
///
/// directory path
///
/// # Returns
///
/// A vector of '.tar.gz' files..
///
/// If there is none or the directory doesn't even exists,
/// it returns an empty vector.
///
/// # Panics
///
/// Only if system runs out of memory.
fn get_dir_files(dir_path: &Path) -> Vec<PathBuf> {
    let entries = match read_dir(dir_path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext == "gz")
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect()
}

/// Inserts `.tar.gz` files data to [PRIECO_FJALL] blob storage.
///
/// This function opens the archives, streams their entries, creates blobs and inserts them to [PRIECO_FJALL] blob storage.
///
/// # Arguments
///
/// * `tar_path` - Path of archive.
/// * `single_blob_buffer` - Reusable single-blob buffer.
///
/// # Returns
///
/// A `Result` of the processing.
///
/// A success status
///
/// # Errors
///
/// This function will return an [`std::io::Error`] in the following situations:
/// * If `.tar.gz` file failes to get remove.
/// * If `tar_path` does not already exist.
/// * If archive doesn't have entries.
/// * If entry doesn't have path.
/// * If entry contains invalid blob ID.
/// * If entry fails to stream to 'single_blob_buffer'.
/// * If [PRIECO_FJALL] blob storage batch commit fails.
/// * If continual simple data integrity check fails.
/// * If it failes to make [PRIECO_FJALL] blob storage persistant.
///
/// # Panics
///
/// Only if system runs out of memory.
fn process_file(tar_path: &Path, single_blob_buffer: &mut Vec<u8>) -> io::Result<()> {
    // Archive exists
    if !file_exists(tar_path) {
        return Ok(()); // Pretend we processed it
    }

    // Archive is empty (less than 100B)
    if metadata(tar_path)?.len() < 100 {
        remove_file(tar_path)?;
        return Ok(());
    }

    // Get a FJALL batch to buffer writes to disk
    let mut batch = PRIECO_FJALL.blob_db.batch();

    // Get archive handler
    let file = File::open(tar_path)?;
    let mut archive = Archive::new(GzDecoder::new(file));

    // Stream individual .txt files in .tar.gz
    let mut files_inserted = 0;
    for entry_result in archive.entries()? {
        // Extract entry
        let mut entry = entry_result?;
        let path = entry.path()?;
        if path.is_dir() {
            continue;
        }

        // Reject other than .zst and .txt file streams
        let file_extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let flag: u8 = match file_extension {
            "zst" => 1,
            "txt" => 0,
            _ => continue,
        };

        let name: u64 = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid blob ID in filename: {:?}", path),
                )
            })?;

        // Insert to buffers
        single_blob_buffer.clear();

        single_blob_buffer.push(flag);
        entry.read_to_end(single_blob_buffer)?;

        batch.insert(
            &PRIECO_FJALL.blobs_ks,
            name.to_le_bytes(),
            single_blob_buffer.clone(),
        );

        // Logging & Commit
        files_inserted += 1;
        if files_inserted % 1000 == 0 {
            if let Err(e) = batch.commit() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "{}: {}Failed to commit FJALL blob batch!{} {}",
                        icons::BLOB,
                        colors::RED,
                        colors::RESET,
                        e
                    ),
                ));
            };

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

            // Simple data integrity check
            let missing = match PRIECO_FJALL.meta_ks.get(&name.to_le_bytes()) {
                Ok(Some(_)) => false,
                Ok(None) => true,
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Database error during integrity check: {}", e),
                    ));
                }
            };
            if missing {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Integrity check failed for blob {}", name),
                ));
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    println!("{}: Final Flush!", icons::BLOB);
    if let Err(e) = batch.commit() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "{}: {}Failed to commit FJALL blob batch!{} {}",
                icons::BLOB,
                colors::RED,
                colors::RESET,
                e
            ),
        ));
    };

    if let Err(e) = PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "{}: {}Failed to make FJALL blob persistant!{} {}",
                icons::BLOB,
                colors::RED,
                colors::RESET,
                e
            ),
        ));
    };

    println!(
        "{}: {}Flushed!{}",
        icons::BLOB,
        colors::GREEN,
        colors::RESET
    );

    remove_file(&tar_path)?;

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

    Ok(())
}
