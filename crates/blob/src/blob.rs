//!  File: blob/blob.rs
//!  Description:
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2025-02-07
//!  Last Modified: 2026-07-10
//!
//!  Usage: Run() to take archived htmls and insert them into Blob storage
//!  TODO:

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
use dashmap::DashMap;
use fjall::PersistMode;
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tar::Archive;

/*
  Import own libraries
*/
use prieco_core::{
    BLOB_IMPORT_DIR, PRIECO_FJALL,
    globals::{colors, icons},
};

pub fn run() {
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
                if let Err(e) = batch.commit() {
                    println!(
                        "{}Failed to commit blob batch!{} {}",
                        colors::RED,
                        colors::RESET,
                        e
                    );
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
        if let Err(e) = batch.commit() {
            println!(
                "{}Failed to commit blob batch!{} {}",
                colors::RED,
                colors::RESET,
                e
            );
        };

        if let Err(e) = PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll) {
            println!(
                "{}Failed to make blobdb persistant!{} {}",
                colors::RED,
                colors::RESET,
                e
            );
        };

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

/*
  Decode blob
*/
pub enum DecodeMode<'a> {
    Text,
    Html { proxy_prefix: &'a str },
}

pub fn decode_blob_to_text(raw_db_value: &[u8]) -> String {
    decode_blob(raw_db_value, DecodeMode::Text)
}

pub fn decode_blob_to_html_rendered(raw_db_value: &[u8], proxy_prefix: &str) -> String {
    decode_blob(raw_db_value, DecodeMode::Html { proxy_prefix })
}

fn decode_blob(raw_db_value: &[u8], mode: DecodeMode) -> String {
    if raw_db_value.is_empty() {
        return String::new();
    }

    let is_compressed = raw_db_value[0] == 1;
    let payload = &raw_db_value[1..];

    let decompressed = if is_compressed {
        zstd::stream::decode_all(std::io::Cursor::new(payload)).unwrap_or_default()
    } else {
        payload.to_vec()
    };

    if decompressed.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(decompressed.len() * 4);
    let mut i = 0;

    while i < decompressed.len() {
        let tag_byte = decompressed[i];
        i += 1;

        let tag_name = tag_byte_to_name(tag_byte);
        let is_url = tag_byte == b'i' || tag_byte == b'h';
        let mut inner_content = String::new();

        while i < decompressed.len() {
            if i + 3 < decompressed.len() && decompressed[i..i + 4] == [255, 255, 255, 255] {
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
            let (resolved_word, _) = resolve_token(slice);

            let word_str: &str = match &resolved_word {
                ResolvedToken::Raw(s) => s,
                ResolvedToken::Cached(arc) => arc.as_str(),
            };

            inner_content.push_str(word_str);

            match mode {
                DecodeMode::Text => {
                    inner_content.push(' ');
                }
                DecodeMode::Html { .. } => {
                    if !is_url && i + len < decompressed.len() && decompressed[i + len] != 255 {
                        inner_content.push(' ');
                    }
                }
            }

            i += len;
        }

        match mode {
            DecodeMode::Text => {
                output.push('<');
                output.push_str(tag_name);
                output.push('>');
                output.push_str(&inner_content);
                output.push_str("</");
                output.push_str(tag_name);
                output.push_str(">\n");
            }
            DecodeMode::Html { proxy_prefix } => {
                let inner = inner_content.trim();
                if inner.is_empty() {
                    continue;
                }

                let tag_label = format!("<strong>[{}]</strong>", tag_name);

                match tag_byte {
                    b'i' => {
                        let encoded_url = urlencoding::encode(inner);
                        output.push_str(&format!(
                            "<div>{}<br><img src=\"{}{}\" /></div>\n",
                            tag_label, proxy_prefix, encoded_url
                        ));
                    }
                    b'h' => {
                        output.push_str(&format!(
                            "<div>{}<a href=\"{}\" target=\"_blank\">{}</a></div>\n",
                            tag_label, inner, inner
                        ));
                    }
                    b'm' => {
                        output.push_str(&format!("<div>{} <i>{}</i></div>\n", tag_label, inner));
                    }
                    _ => {
                        output.push_str(&format!(
                            "<div>{}<br><span>{}</span></div>\n",
                            tag_label, inner
                        ));
                    }
                }
            }
        }
    }

    output
}

/* Helper functions */
/* Importer */
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

/* Decoder */
pub enum ResolvedToken<'a> {
    Raw(&'a str),
    Cached(Arc<String>),
}

fn resolve_token<'a>(slice: &'a [u8]) -> (ResolvedToken<'a>, bool) {
    if slice.len() > 3 {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        let (word, hit) = search_word_by_id(id as usize);
        return (ResolvedToken::Cached(word), hit);
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
        (
            ResolvedToken::Raw(std::str::from_utf8(slice).unwrap()),
            true,
        )
    } else {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        let (word, hit) = search_word_by_id(id as usize);
        (ResolvedToken::Cached(word), hit)
    }
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

static DICT_CACHE: Lazy<DashMap<usize, Arc<String>>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_ITEMS + 1_000));
const MAX_CACHE_ITEMS: usize = 1_000_000;

pub fn search_word_by_id(id: usize) -> (Arc<String>, bool) {
    if id < 256 {
        return (Arc::new(String::new()), true);
    }

    if let Some(cached_word) = DICT_CACHE.get(&id) {
        return (cached_word.value().clone(), true);
    }

    (Arc::new(String::new()), false)
}
