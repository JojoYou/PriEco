//! # Mini Crawler
//!
//! Worker loop of a node. Crawls URLs given by main crawler at [MASTER_URL].
//!
//! ## Architecture
//!
//! 1. [**Fetch**:][run] Asks main crawler for a [WorkerBatchResponse].
//!     * Waits 5s on error, 10s on empty queue.
//! 2. [**Download**:][crate::download::download] In parallel, `worker_concurrent` downloads at once.
//!     * Only 200 responses with body continue.
//! 3. [**Parse**:][crate::meta::extract_metadata] On blocking threads, 2 per CPU core.
//!     * Pages without title are dropped.
//! 4. [**Compress**:][run] Batch is bincode encoded and zstd compressed.
//! 5. [**Upload**:][UploadTask] Background task sends batch to main crawler.
//!     * Retries every 5s until accepted.
//!     * Max 2 batches wait, then crawling waits too.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-06-16
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use ahash::AHashMap;
use once_cell::sync::Lazy;
use prieco_core::{ANALYTICS, CLIENT, PRIECO_CONFIG, icons};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc};
use url::Url;
use zstd::stream::Encoder;

use crate::download::download;
use crate::meta::extract_metadata;
use crate::tag_data::extract_tag_data;

/// Parsed page, sent to main crawler.
#[derive(Serialize, Deserialize, Debug)]
pub struct HTMLResultData {
    /// Final URL after redirects.
    pub url: String,
    /// Max 60 characters.
    pub title: String,
    /// Max 160 characters.
    pub description: String,
    /// First 500 characters of h2-h6 and p.
    pub text: String,
    /// Href of icon link, as written in html.
    pub favicon: String,
    /// First valid image URL.
    pub img: String,
    /// Meta keywords.
    pub keywords: String,
    /// True if page is NSFW, see [crate::meta].
    pub safe_search: bool,
    /// Empty, filled by main crawler.
    pub html: String,
    /// ISO 639-1 code, empty when unknown.
    pub language: String,
    /// Lowercase country code of server IP.
    pub location: String,
    /// Empty, page scores are computed by main crawler.
    pub points: Vec<f32>,
    /// Download time in seconds.
    pub loading_time: f64,
    /// 'YYMMDD'.
    pub date_of_crawling: String,
    /// Empty, embedding is made by main crawler.
    pub vector: String,
    /// Output of [crate::tag_data::extract_tag_data].
    pub tag_data: AHashMap<String, Vec<String>>,
}

/// URLs to crawl, from main crawler.
#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerBatchResponse {
    /// Id the results are uploaded under.
    pub batch_id: String,
    /// URLs to crawl.
    pub urls: Vec<String>,
}

/// Main crawler.
const MASTER_URL: &str = "https://crawler.prieco.net";

/// Compressed batch waiting for upload.
struct UploadTask {
    /// Id from [WorkerBatchResponse].
    batch_id: String,
    /// zstd compressed bincode of `Vec<HTMLResultData>`.
    compressed_buffer: Vec<u8>,
    /// Pages in batch, for analytics.
    successful: usize,
}

/// Runs the mini crawler forever.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// Only if uploader stops.
///
/// # Panics
///
/// If a semaphore gets closed.
pub async fn run() {
    // Holds max 2 batches to upload, then blocks
    let (upload_tx, mut upload_rx) = mpsc::channel::<UploadTask>(2);

    // Background uploader
    tokio::spawn(async move {
        // Upload new batch
        while let Some(task) = upload_rx.recv().await {
            println!(
                "{}: Uploading batch {} ({} bytes)...",
                icons::MINI_CRAWLER_ICON,
                task.batch_id,
                task.compressed_buffer.len()
            );

            let upload_url = format!(
                "{}/api/worker/submit?batch_id={}",
                MASTER_URL, task.batch_id
            );

            loop {
                let payload = task.compressed_buffer.clone();

                let upload_res = CLIENT.post(&upload_url).body(payload).send().await;

                match upload_res {
                    Ok(res) if res.status().is_success() => {
                        ANALYTICS.record_worker_batch(
                            "worker",
                            task.successful as u64,
                            task.compressed_buffer.len() as u64,
                        );
                        println!(
                            "{}: Batch {} accepted by main crawler!",
                            icons::MINI_CRAWLER_ICON,
                            task.batch_id
                        );
                        break;
                    }
                    Ok(res) => {
                        let status = res.status();
                        let error_body = res
                            .text()
                            .await
                            .unwrap_or_else(|_| "Could not read error body".to_string());
                        println!(
                            "{}: Failed to upload batch {}. Status: {} - {}. Retrying in 5s...",
                            icons::MINI_CRAWLER_ICON,
                            task.batch_id,
                            status,
                            error_body
                        );
                    }
                    Err(e) => {
                        println!(
                            "{}: Network error uploading batch {}: {}. Retrying in 5s...",
                            icons::MINI_CRAWLER_ICON,
                            task.batch_id,
                            e
                        );
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    });

    let dl_semaphore = Arc::new(Semaphore::new(PRIECO_CONFIG.worker_concurrent as usize));
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let parse_semaphore = Arc::new(Semaphore::new(cpu_cores * 2));

    loop {
        // Fetch URLs
        let urls_req = CLIENT
            .get(format!("{}/api/worker/urls", MASTER_URL,))
            .send()
            .await;

        let response_data: WorkerBatchResponse = match urls_req {
            Ok(res) if res.status().is_success() => {
                res.json().await.unwrap_or_else(|_| WorkerBatchResponse {
                    batch_id: String::new(),
                    urls: vec![],
                })
            }
            _ => {
                println!(
                    "{}: Failed to fetch URLs, retrying in 5s...",
                    icons::MINI_CRAWLER_ICON
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        if response_data.urls.is_empty() {
            println!(
                "{}: Queue empty, waiting for 10s...",
                icons::MINI_CRAWLER_ICON
            );
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }

        let total_urls = response_data.urls.len();
        println!(
            "{}: Received {} URLs to process.",
            icons::MINI_CRAWLER_ICON,
            total_urls
        );

        ANALYTICS.count_by("worker_urls_issued", total_urls as u64);

        let mut results_batch: Vec<HTMLResultData> = Vec::with_capacity(total_urls);
        let mut join_handles = Vec::with_capacity(total_urls);

        let download_counter = Arc::new(AtomicUsize::new(1));
        for url in response_data.urls.into_iter() {
            let dl_sem_clone = dl_semaphore.clone();
            let parse_sem_clone = parse_semaphore.clone();

            let counter_clone = download_counter.clone();
            join_handles.push(tokio::spawn(async move {
                let dl_permit = dl_sem_clone.acquire_owned().await.unwrap();

                let current_count = counter_clone.fetch_add(1, Ordering::SeqCst);
                if current_count.is_multiple_of(100) {
                    println!("Downloaded {}/{}", current_count, total_urls);
                }

                let response = download(&CLIENT, &url).await;

                drop(dl_permit);

                if response.1 != 200 || response.0.trim().is_empty() {
                    return None;
                }

                let parse_permit = parse_sem_clone.acquire_owned().await.unwrap();

                let parse_result = tokio::task::spawn_blocking(move || {
                    let document = Html::parse_document(&response.0);
                    let tag_data: AHashMap<String, Vec<String>> = extract_tag_data(&document);
                    let result: HTMLResultData =
                        extract_metadata(&document, &response.3, response.2, &tag_data);

                    drop(parse_permit);

                    if !result.title.is_empty() {
                        Some(result)
                    } else {
                        None
                    }
                })
                .await;

                parse_result.unwrap_or(None)
            }));
        }

        for handle in join_handles {
            if let Ok(Some(result)) = handle.await {
                results_batch.push(result);
            }
        }

        let successful = results_batch.len();
        if successful == 0 {
            println!(
                "{}: All URLs in batch failed, moving to next...",
                icons::MINI_CRAWLER_ICON
            );
            continue;
        }

        println!(
            "{}: Serializing and compressing {} results...",
            icons::MINI_CRAWLER_ICON,
            successful
        );

        let serialized_bytes_result =
            bincode_next::serde::encode_to_vec(&results_batch, bincode_next::config::standard());

        if let Ok(serialized_bytes) = serialized_bytes_result {
            let mut compressed_buffer = Vec::new();
            let mut encoder = match Encoder::new(&mut compressed_buffer, 3) {
                Ok(enc) => enc,
                Err(e) => {
                    println!(
                        "{}: Failed to create zstd encoder: {}",
                        icons::MINI_CRAWLER_ICON,
                        e
                    );
                    continue;
                }
            };

            if let Err(e) = encoder.write_all(&serialized_bytes) {
                println!(
                    "{}: Failed to compress batch: {}",
                    icons::MINI_CRAWLER_ICON,
                    e
                );
                continue;
            }

            if let Err(e) = encoder.finish() {
                println!(
                    "{}: Failed to finish compression: {}",
                    icons::MINI_CRAWLER_ICON,
                    e
                );
                continue;
            }

            // Send to Uploader
            if upload_tx
                .send(UploadTask {
                    batch_id: response_data.batch_id,
                    compressed_buffer,
                    successful,
                })
                .await
                .is_err()
            {
                println!(
                    "{}: Uploader channel closed unexpectedly.",
                    icons::MINI_CRAWLER_ICON
                );
                break;
            }
        }
    }
}

/* Helper functions */
/// Checks if `input` is an http or https URL.
///
/// # Arguments
///
/// * `input` - URL.
///
/// # Returns
///
/// True if valid, false otherwise.
pub fn is_valid_url(input: &str) -> bool {
    match Url::parse(input) {
        Ok(url) => matches!(url.scheme(), "http" | "https"),
        Err(_) => false,
    }
}

/// Selectors compiled once and shared by all parsers.
pub static PARSER_SELECTORS: Lazy<Arc<CompiledSelectors>> =
    Lazy::new(|| Arc::new(CompiledSelectors::new()));
/// CSS selectors used by [crate::meta] and [crate::tag_data].
pub struct CompiledSelectors {
    // Your existing selectors
    /// h1 to h6, in order.
    pub h_selectors: Vec<Selector>,
    /// span, p, a, li, label, in order.
    pub text_selectors: Vec<Selector>,
    /// meta
    pub meta_selector: Selector,
    /// img
    pub img_selector: Selector,
    /// a
    pub a_selector: Selector,

    /// title
    pub title_selector: Selector,
    /// p
    pub p_selector: Selector,
    /// meta name='keywords'
    pub meta_keywords_selector: Selector,
    /// meta name='description'
    pub meta_description_selector: Selector,
    /// meta property='og:site_name'
    pub og_site_name_selector: Selector,
    /// meta property='og:description'
    pub og_description_selector: Selector,
    /// h2 to h6 and p, for page text.
    pub content_selectors: Vec<Selector>, // h2-h6, p combined

    /// link rel='icon'
    pub link_icon_selector: Selector,
    /// link rel='shortcut icon'
    pub link_shortcut_selector: Selector,
    /// link rel='apple-touch-icon'
    pub link_apple_selector: Selector,
}

impl CompiledSelectors {
    /// Compiles all selectors.
    ///
    /// # Panics
    ///
    /// Never, selectors are constant.
    pub fn new() -> Self {
        Self {
            h_selectors: (1..=6)
                .map(|i| Selector::parse(&format!("h{}", i)).unwrap())
                .collect(),
            text_selectors: ["span", "p", "a", "li", "label"]
                .iter()
                .map(|tag| Selector::parse(tag).unwrap())
                .collect(),
            meta_selector: Selector::parse("meta").unwrap(),
            img_selector: Selector::parse("img").unwrap(),
            a_selector: Selector::parse("a").unwrap(),

            title_selector: Selector::parse("title").unwrap(),
            p_selector: Selector::parse("p").unwrap(),
            meta_keywords_selector: Selector::parse("meta[name='keywords']").unwrap(),
            meta_description_selector: Selector::parse("meta[name='description']").unwrap(),
            og_site_name_selector: Selector::parse("meta[property='og:site_name']").unwrap(),
            og_description_selector: Selector::parse("meta[property='og:description']").unwrap(),
            content_selectors: ["h2", "h3", "h4", "h5", "h6", "p"]
                .iter()
                .map(|tag| Selector::parse(tag).unwrap())
                .collect(),

            link_icon_selector: Selector::parse("link[rel='icon']").unwrap(),
            link_shortcut_selector: Selector::parse("link[rel='shortcut icon']").unwrap(),
            link_apple_selector: Selector::parse("link[rel='apple-touch-icon']").unwrap(),
        }
    }
}

impl Default for CompiledSelectors {
    fn default() -> Self {
        Self::new()
    }
}
