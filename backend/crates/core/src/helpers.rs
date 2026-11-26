//! # Helpers
//!
//! Small functions used across the whole project.
//!
//! * **Files:** [file_exists], [read_file], [write_file], [get_dir_size].
//! * **Locks:** [acquire_file_lock], [release_file_lock] for writes from more threads.
//! * **URLs:** [url_to_id], [url_to_domain_id], [normalize_url], [local_redirect].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{
    fs::{OpenOptions, create_dir_all, read_dir, read_to_string},
    hash::Hasher,
    io::{BufWriter, Write},
    path::Path,
};

use twox_hash::XxHash3_64;
use url::Url;

use crate::globals::{FILE_LOCKS, colors};

/// Checks if file or directory exists.
///
/// I find it more intuitive than `Path::exists`.
///
/// # Arguments
///
/// * `file_path` - Path.
///
/// # Returns
///
/// True if it exists, false otherwise.
pub fn file_exists<P: AsRef<Path>>(file_path: P) -> bool {
    file_path.as_ref().exists()
}

/// Reads whole file to string.
///
/// # Arguments
///
/// * `file_path` - Path.
///
/// # Returns
///
/// File content, empty string if it can't be read.
pub fn read_file(file_path: &str) -> String {
    read_to_string(file_path).unwrap_or_default()
}

/// Writes to file, safe when more threads write the same file.
///
/// Creates parent directories. Holds a [file lock][acquire_file_lock] while writing.
/// Errors are printed.
///
/// # Arguments
///
/// * `file_path` - Path.
/// * `content` - Data to write.
/// * `append` - Append to file, otherwise old content is removed.
///
/// # Returns
///
/// None
pub fn write_file<C: AsRef<[u8]>>(file_path: &str, content: C, append: bool) {
    acquire_file_lock(file_path);

    // Create parent directories if needed
    let path = Path::new(file_path);
    if let Err(e) = create_dir_all(
        path.parent()
            .map(|p| p.to_str().unwrap_or(""))
            .unwrap_or(""),
    ) {
        println!("{}{}{}", colors::RED, e, colors::RESET);
    }

    /*
      Create, open, write and flush the data
    */
    // Open file with buffered writer
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .append(append)
        .truncate(!append)
        .open(file_path)
        .map(BufWriter::new)
    {
        Ok(f) => f,
        Err(e) => {
            println!(
                "{}Bulk write function{}: Failed to open file {} Because of: {}",
                colors::RED,
                colors::RESET,
                file_path,
                e
            );

            release_file_lock(file_path);
            return;
        }
    };

    // Write and flush
    let _ = file.write_all(content.as_ref()).and_then(|_| file.flush());

    release_file_lock(file_path);
}

/// Blocks until no other thread holds the lock for `path`, then takes it.
///
/// Lock is per path string, only in this process. Release with [release_file_lock].
pub fn acquire_file_lock(path: &str) {
    let mut set = FILE_LOCKS.set.lock();

    while set.contains(path) {
        FILE_LOCKS.condvar.wait(&mut set);
    }

    set.insert(path.to_string());
}

/// Releases lock taken by [acquire_file_lock] and wakes waiting threads.
pub fn release_file_lock(path: &str) {
    let set = FILE_LOCKS.set.lock();
    set.remove(path);
    FILE_LOCKS.condvar.notify_all();
}

/// Page id of `url`. xxHash3 64 with seed 0 of the whole URL.
///
/// Used as key in meta storage and Tantivy `doc_id`.
pub fn url_to_id(url: &str) -> u64 {
    let mut h = XxHash3_64::with_seed(0);
    h.write(url.as_bytes());
    h.finish()
}

/// Domain id of `url`. xxHash3 64 with seed 0 of the domain.
///
/// # Returns
///
/// Domain id, 0 if URL can't be parsed or has no domain (e.g. IP address).
pub fn url_to_domain_id(url: &str) -> u64 {
    let pased_url = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            println!(
                "{}Failed to generate domain from URL{}: {} Error: {}",
                colors::RED,
                colors::RESET,
                url,
                e
            );
            return 0;
        }
    };

    let domain = match pased_url.domain() {
        Some(d) => d,
        None => return 0,
    };

    let mut h = XxHash3_64::with_seed(0);
    h.write(domain.as_bytes());
    h.finish()
}

/// Normalizes URL for comparing.
///
/// Adds 'http://' when scheme is missing, removes query, fragment and trailing '/'.
/// Host gets lowercase by URL parser.
///
/// # Returns
///
/// Normalized URL, `raw` if it can't be parsed.
pub fn normalize_url(raw: &str) -> String {
    let url_str = if raw.to_ascii_lowercase().starts_with("http://")
        || raw.to_ascii_lowercase().starts_with("https://")
    {
        raw.to_owned()
    } else {
        format!("http://{}", raw)
    };

    Url::parse(&url_str)
        .ok()
        .map(|mut url| {
            url.set_query(None);
            url.set_fragment(None);
            let mut normalized = url.to_string();
            if normalized.ends_with('/') {
                normalized.pop();
            }
            normalized
        })
        .unwrap_or_else(|| raw.to_string())
}

/// Size of all files in directory and subdirectories, in bytes.
///
/// Unreadable entries are skipped.
pub fn get_dir_size(path: impl AsRef<Path>) -> u64 {
    let mut size = 0;
    if let Ok(entries) = read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size += get_dir_size(entry.path());
                } else {
                    size += metadata.len();
                }
            }
        }
    }
    size
}

/// Allows only local redirects.
///
/// `target` must start with a single '/' and have no control characters,
/// so it can't redirect to a different site.
///
/// # Returns
///
/// `target` if it is local, '/' otherwise.
pub fn local_redirect(target: &str) -> String {
    if target.starts_with('/')
        && !target.starts_with("//")
        && !target.starts_with("/\\")
        && !target.chars().any(|c| c.is_control())
    {
        target.to_string()
    } else {
        "/".to_string()
    }
}
