/*
  File: lib.rs
  Description: Joins whole project & Holds universal functions

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-02-06

  Usage: Call these functions to develop faster
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    fs::{OpenOptions, create_dir_all, read_to_string},
    hash::Hasher,
    io::{BufWriter, Write},
    path::Path,
};

use twox_hash::XxHash3_64;
use url::Url;

use crate::globals::{FILE_LOCKS, colors};

/*
  Description: I find it more intuitive than Path exists

  Input: file path
  Output: true if file exists, false otherwise
*/
pub fn file_exists(file_path: &str) -> bool {
    Path::new(file_path).exists()
}

/*
  Description: Simpler function with error handling, if failed to load file I expect empty string

  Input: file path
  Output: file contents as a string
*/
pub fn read_file(file_path: &str) -> String {
    match read_to_string(file_path) {
        Ok(contents) => contents,
        Err(_) => String::new(),
    }
}

/*
  Description: Safe, thread resistent write to a file

  Input: file path, content, should append the file (if false it removes all already written content)
  Output: None
*/
pub fn write_file(file_path: &str, content: &str, append: bool) {
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
            return;
        }
    };

    // Write and flush
    let _ = file
        .write_all(content.as_bytes())
        .and_then(|_| file.flush());

    release_file_lock(file_path);
}

pub fn acquire_file_lock(path: &str) {
    let mut set = FILE_LOCKS.set.lock();

    while set.contains(path) {
        FILE_LOCKS.condvar.wait(&mut set);
    }

    set.insert(path.to_string());
}

pub fn release_file_lock(path: &str) {
    let set = FILE_LOCKS.set.lock();
    set.remove(path);
    FILE_LOCKS.condvar.notify_one();
}

pub fn url_to_id(url: &str) -> u64 {
    let mut h = XxHash3_64::with_seed(0);
    h.write(url.as_bytes());
    h.finish()
}

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

pub fn normalize_url(raw: &str) -> String {
    let url_str = if !raw.starts_with("http") {
        format!("http://{}", raw)
    } else {
        raw.to_owned()
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
