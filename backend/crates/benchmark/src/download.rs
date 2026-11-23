//! # Dataset Downloader
//!
//! Downloads benchmark datasets with the global [CLIENT] into a runtime directory.
//!
//! ## Architecture
//!
//! 1. [**Run**:][run] Creates the directory and downloads every url.
//! 2. [**Fetch**:][fetch] Streams one file to '.part' and renames it when done.
//!     * Skips files that already exist.
//!     * Keeps '.gz' files compressed.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-22
//! * Last Modified: 2026-11-22
//!
//! ## Planned Improvements
//!
//! - [ ] Resume '.part' files with a Range header

/*
  Import system libraries
*/
use std::{
    error::Error,
    fs::{File, create_dir_all, rename},
    io::{BufWriter, Write},
    path::Path,
    time::{Duration, Instant},
};

/*
  Import own libraries
*/
use prieco_core::{CLIENT, colors, file_exists, globals::icons};

/*
  Constants
*/
const DOWNLOAD_TIMEOUT: Duration = Duration::from_hours(12);

/// Downloads every url into a directory.
///
/// Files that are already in the directory are skipped,
/// so only the first run downloads anything.
///
/// # Arguments
///
/// * `dir` - Directory to download into, created when missing.
/// * `urls` - Urls to download. The file name is the last part of the url.
///
/// # Returns
///
/// `true` when every file is in the directory.
///
/// `false` when any download failed, the error is printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn run(dir: &str, urls: &[&str]) -> bool {
    // Create dir
    if let Err(e) = create_dir_all(dir) {
        eprintln!(
            "{}{}: Failed to create {}: {}{}",
            icons::BENCHMARK,
            colors::RED,
            dir,
            e,
            colors::RESET,
        );
        return false;
    }

    // Download files
    for url in urls {
        let name = url.rsplit('/').next().unwrap_or_default();
        let path = Path::new(dir).join(name);

        if let Err(e) = fetch(url, &path).await {
            eprintln!(
                "{}{}: Failed to download {}: {}{}",
                icons::BENCHMARK,
                colors::RED,
                url,
                e,
                colors::RESET,
            );
            return false;
        }
    }

    true
}

/* Helper functions */

/// Streams one url into a file.
///
/// The body is written to '<name>.part' and renamed when finished,
/// so an aborted download is never seen as cached.
///
/// # Arguments
///
/// * `url` - Url to download.
/// * `path` - Final file path.
///
/// # Returns
///
/// `Ok(())` when the file is downloaded or already exists.
///
/// # Errors
///
/// * Request failed or timed out.
/// * Server answered with an error status (404, 500, ...).
/// * File could not be written or renamed.
///
/// # Panics
///
/// Only if system runs out of memory.
async fn fetch(url: &str, path: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Cached
    if file_exists(path) {
        println!("{}: Cached: {}", icons::BENCHMARK, path.display());
        return Ok(());
    }

    let start = Instant::now();
    println!("{}: Downloading: {}", icons::BENCHMARK, url);

    let mut response = CLIENT
        .get(url)
        .timeout(DOWNLOAD_TIMEOUT)
        .header("Accept-Encoding", "identity")
        .send()
        .await?
        .error_for_status()?;

    // Partial download
    let part = path.with_file_name(format!(
        "{}.part",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
    ));

    // Write
    let mut out = BufWriter::new(File::create(&part)?);
    while let Some(chunk) = response.chunk().await? {
        out.write_all(&chunk)?;
    }
    out.into_inner().map_err(|e| e.into_error())?.sync_all()?;

    rename(&part, path)?;

    println!(
        "{}{}: Downloaded {} in {:.3}s!{}",
        icons::BENCHMARK,
        colors::GREEN,
        path.display(),
        start.elapsed().as_secs_f64(),
        colors::RESET,
    );

    Ok(())
}
