use ahash::AHashMap;
use dotenv_codegen::dotenv;
use once_cell::sync::Lazy;
use prieco_core::{CLIENT, PRIECO_CONFIG, icons};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;
use zstd::stream::Encoder;

use crate::download::download;
use crate::meta::extract_metadata;
use crate::tag_data::extract_tag_data;

#[derive(Serialize, Deserialize, Debug)]
pub struct HTMLResultData {
    pub url: String,
    pub title: String,
    pub description: String,
    pub text: String, // Header text of the html
    pub favicon: String,

    pub img: String,
    pub keywords: String,
    pub safe_search: bool, // Safe search required. Requires to turn off safe search in PriEco to show this result
    pub html: String,      // Name of stored html file

    pub language: String,
    pub location: String,

    pub points: Vec<f32>,
    pub loading_time: f64,        // Loading time of the result
    pub date_of_crawling: String, // When we crawled this result

    pub vector: String,

    pub tag_data: AHashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerBatchResponse {
    pub batch_id: String,
    pub urls: Vec<String>,
}

const MASTER_URL: &str = "https://crawler.prieco.net";

pub async fn run() {
    // Fetch URLs
    let urls_req = CLIENT
        .get(format!(
            "{}/api/worker/urls?id={}",
            MASTER_URL, PRIECO_CONFIG.worker_id,
        ))
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
            return;
        }
    };

    if response_data.urls.is_empty() {
        println!(
            "{}: Queue empty, waiting for 10s...",
            icons::MINI_CRAWLER_ICON
        );
        tokio::time::sleep(Duration::from_secs(10)).await;
        return;
    }

    println!(
        "{}: Received {} URLs to process.",
        icons::MINI_CRAWLER_ICON,
        response_data.urls.len()
    );
    let mut results_batch: Vec<HTMLResultData> = Vec::with_capacity(response_data.urls.len());

    // Process URLs
    let total_urls = response_data.urls.len();
    let mut download_handles = Vec::with_capacity(total_urls);
    let semaphore = Arc::new(Semaphore::new(PRIECO_CONFIG.worker_concurrent as usize));

    for (i, url) in response_data.urls.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        download_handles.push(tokio::spawn(async move {
            println!("Downloading ({}/{}): {}", i + 1, total_urls, url);
            let response = download(&*CLIENT, &url).await;

            drop(permit);
            response
        }));
    }

    // Process
    for handle in download_handles {
        /*
          From now use these variables:
          response.0: body
          response.1: status code
          response.2: downloading time
          response.3: final url
        */
        let response = match handle.await {
            Ok(res) => res,
            Err(_) => continue,
        };

        if response.1 != 200 || response.0.trim().is_empty() {
            continue;
        }

        let document = Html::parse_document(&response.0);
        let tag_data: AHashMap<String, Vec<String>> = extract_tag_data(&document);
        let result: HTMLResultData =
            extract_metadata(&document, &response.3, response.2, &tag_data);

        if !result.title.is_empty() {
            results_batch.push(result);
        }
    }

    if results_batch.is_empty() {
        return;
    }

    // Serialize and Compress
    println!(
        "{}: Serializing and compressing {} results...",
        icons::MINI_CRAWLER_ICON,
        results_batch.len()
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
                return;
            }
        };

        if let Err(e) = encoder.write_all(&serialized_bytes) {
            println!(
                "{}: Failed to compress batch: {}",
                icons::MINI_CRAWLER_ICON,
                e
            );
            return;
        }

        if let Err(e) = encoder.finish() {
            println!(
                "{}: Failed to finish compression: {}",
                icons::MINI_CRAWLER_ICON,
                e
            );
            return;
        }

        // Upload
        println!(
            "{}: Uploading {} bytes to main crawler...",
            icons::MINI_CRAWLER_ICON,
            compressed_buffer.len()
        );

        let upload_res = CLIENT
            .post(&format!(
                "{}/api/worker/submit?id={}&batch_id={}",
                MASTER_URL, PRIECO_CONFIG.worker_id, response_data.batch_id
            ))
            .body(compressed_buffer)
            .send()
            .await;

        match upload_res {
            Ok(res) if res.status().is_success() => println!(
                "{}: Batch accepted by main crawler!",
                icons::MINI_CRAWLER_ICON
            ),
            Ok(res) => {
                let status = res.status();
                let error_body = res
                    .text()
                    .await
                    .unwrap_or_else(|_| "Could not read error body".to_string());
                println!(
                    "{}: Failed to upload batch. Server responded with Status: {} - {}",
                    icons::MINI_CRAWLER_ICON,
                    status,
                    error_body
                );
            }
            Err(e) => {
                println!(
                    "{}: Network error during upload: {}",
                    icons::MINI_CRAWLER_ICON,
                    e
                );
            }
        }
    }
}

/* Helper functions */
/*
  Description: Checks if URL is valid

  Input: URL
  Output: true if valid, false otherwise
*/
pub fn is_valid_url(input: &str) -> bool {
    match Url::parse(input) {
        Ok(url) => match url.scheme() {
            "http" | "https" => true,
            _ => false,
        },
        Err(_) => false,
    }
}

pub static PARSER_SELECTORS: Lazy<Arc<CompiledSelectors>> =
    Lazy::new(|| Arc::new(CompiledSelectors::new()));
pub struct CompiledSelectors {
    // Your existing selectors
    pub h_selectors: Vec<Selector>,
    pub text_selectors: Vec<Selector>,
    pub meta_selector: Selector,
    pub img_selector: Selector,
    pub a_selector: Selector,

    pub title_selector: Selector,
    pub p_selector: Selector,
    pub meta_keywords_selector: Selector,
    pub meta_description_selector: Selector,
    pub og_site_name_selector: Selector,
    pub og_description_selector: Selector,
    pub content_selectors: Vec<Selector>, // h2-h6, p combined

    pub link_icon_selector: Selector,
    pub link_shortcut_selector: Selector,
    pub link_apple_selector: Selector,
}

impl CompiledSelectors {
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

            // Add new selectors
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
