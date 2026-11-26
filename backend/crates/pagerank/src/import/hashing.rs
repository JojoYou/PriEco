//! # Hashing
//!
//! Import pass A. Reads link files and turns URLs into ids.
//!
//! ## Architecture
//!
//! 1. Every line 'from->to' is two [url ids][url_to_id] of [normalized][normalize_url] URLs.
//!     * Self links are skipped.
//! 2. Every [BUFFER_SIZE] pairs, a sorted unique shard is [written][flush_hash].
//! 3. Read files are removed.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-02-26
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{
    fs::{File, read_dir, remove_file},
    io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Write},
};

/*
  Import own libraries
*/
use crate::compute::{BUFFER_SIZE, CONNECTIONS_DIR, EDGES_DIR};
use prieco_core::{
    globals::{colors, icons},
    normalize_url, url_to_id,
};

/// Hashes [CONNECTIONS_DIR] into [EDGES_DIR].
///
/// Split from [run_in] so tests can use their own paths.
///
/// # Returns
///
/// Paths of shards.
///
/// # Errors
///
/// Same as [run_in].
pub fn run() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    run_in(CONNECTIONS_DIR, EDGES_DIR)
}

/// Hashes link files of `conn_dir` into shards in `edges_dir`.
///
/// Files that can't be opened are skipped.
///
/// # Arguments
///
/// * `conn_dir` - Link files.
/// * `edges_dir` - Output directory.
///
/// # Returns
///
/// Paths of shards.
///
/// # Errors
///
/// * If `conn_dir` can't be read or is empty.
/// * If shard can't be written.
pub fn run_in(conn_dir: &str, edges_dir: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file_buffer: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut hash_shards_paths: Vec<String> = Vec::with_capacity(1_000);

    // Get files from connection dir
    let files: Vec<_> = read_dir(conn_dir)?
        .filter_map(|e| {
            let p = e.ok()?.path();
            p.is_file().then(|| p.to_string_lossy().to_string())
        })
        .collect();
    if files.is_empty() {
        println!(
            "{}: {}No files in {}{}",
            icons::PAGERANK_ICON,
            colors::RED,
            conn_dir,
            colors::RESET
        );
        return Err(Error::new(ErrorKind::NotFound, format!("No files in {}", conn_dir)).into());
    }

    // Get data from files
    for path in files {
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                println!(
                    "{}: {}Skip {}:{} {}",
                    icons::PAGERANK_ICON,
                    colors::YELLOW,
                    path,
                    colors::RESET,
                    e
                );
                continue;
            }
        };

        // Line by line: url->url to 2 hashes (ids)
        for line in BufReader::with_capacity(1_048_576, file).lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            if let Some((from, to)) = line.split_once("->") {
                let fh = url_to_id(&normalize_url(from.trim()));
                let th = url_to_id(&normalize_url(to.trim()));
                if fh == th {
                    continue;
                }
                file_buffer.push((fh, th));
                if file_buffer.len() >= BUFFER_SIZE {
                    flush_hash(&mut file_buffer, &mut hash_shards_paths, edges_dir)?;
                }
            }
        }
        let _ = remove_file(path);
    }
    if !file_buffer.is_empty() {
        flush_hash(&mut file_buffer, &mut hash_shards_paths, edges_dir)?;
    }

    Ok(hash_shards_paths)
}

/* Helper functions */
/// Sorts, dedups and writes `buffer` as 'inc_hash_shard_N.bin', then clears it.
///
/// # Errors
///
/// If shard can't be written.
fn flush_hash(
    buffer: &mut Vec<(u64, u64)>,
    shards_paths: &mut Vec<String>,
    edges_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable();
    buffer.dedup();

    let path = format!("{}/inc_hash_shard_{}.bin", edges_dir, shards_paths.len());
    let mut writer = BufWriter::with_capacity(1 << 20, File::create(&path)?);

    for (a, b) in buffer.iter() {
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
    }

    shards_paths.push(path);
    buffer.clear();

    Ok(())
}
