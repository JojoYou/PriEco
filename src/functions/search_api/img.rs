use dotenv_codegen::dotenv;
use redb::ReadableDatabase;
use reqwest::Client;
use serde_json::Value;
use std::{
    fs::{metadata, write},
    time::Duration,
};

use crate::{
    get_domain,
    globals::{
        ARTISTS_DB, ARTISTS_TABLE, ImgResult,
        colors::{self},
    },
    read_file,
};

pub async fn run(query: &str) -> Vec<ImgResult> {
    ////
    // Cache
    ////
    let bing_file = format!("cache/img/bing/{}.json", query.replace("/", "_"),);
    let unsplash_file = format!("cache/img/unsplash/{}.json", query.replace("/", "_"),);
    let fanart_file = format!("cache/img/fanart/{}.json", query.replace("/", "_"),);
    if metadata(&bing_file).is_ok()
        && metadata(&unsplash_file).is_ok()
        && metadata(&fanart_file).is_ok()
    {
        println!("Cache!");

        let bing_json: Value = match serde_json::from_str(&read_file(&bing_file)) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse Bing cache {}: {}", bing_file, e);
                Value::Null
            }
        };

        let unsplash_json = match serde_json::from_str(&read_file(&unsplash_file)) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse Unsplash cache {}: {}", unsplash_file, e);
                Value::Null
            }
        };

        let fanart_json = match serde_json::from_str(&read_file(&fanart_file)) {
            Ok(json) => json,
            Err(e) => {
                println!("Failed to parse Fanart cache {}: {}", fanart_file, e);
                Value::Null
            }
        };

        return merge_images(
            format_bing(bing_json),
            format_unsplash(unsplash_json),
            format_fanart(fanart_json),
        );
    }

    ////
    // API
    ////
    println!("Call!");

    // Get Artist ID for FanArt from musicbrainz.org database
    let mbid: String = if let Ok(read_txn) = ARTISTS_DB.begin_read() {
        if let Ok(table) = read_txn.open_table(ARTISTS_TABLE) {
            table
                .get(query.replace("+", " ").as_str())
                .ok()
                .flatten()
                .map(|entry| entry.value().to_string())
                .unwrap_or_else(|| {
                    println!("{}Artist not found{}", colors::RED, colors::RESET);
                    String::new()
                })
        } else {
            println!("{}Failed to open table{}", colors::RED, colors::RESET);
            String::new()
        }
    } else {
        println!("{}Failed to open DB{}", colors::RED, colors::RESET);
        String::new()
    };

    let client = match Client::builder()
        .user_agent("PriEco/1.0.0 ( support@jojoyou.org )")
        .timeout(Duration::from_secs(2))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("Img: Failed to build HTTP client: {}", e);
            return Vec::new();
        }
    };

    // Await the results of both requests
    let (bing_result, unsplash_result, fanart_result) = tokio::join!(
        // Bing
        client
            .get(&format!(
                "http://0.0.0.0:8040/?s=b&img=true&api={}&q={}",
                dotenv!("BB2_API_KEY"),
                query,
            ))
            .send(),
        // Unsplash
        client
            .get(&format!(
                "https://api.unsplash.com/search/photos?client_id={}&page=1&query={}",
                dotenv!("UNSPLASH_API_KEY"),
                query
            ))
            .send(),
        // FanArt
        client
            .get(&format!(
                "https://webservice.fanart.tv/v3/music/{}?api_key={}",
                mbid,
                dotenv!("FANART_API_KEY")
            ))
            .send(),
    );

    // Convert Results into Options
    let bing_resp = bing_result.ok();
    let unsplash_resp = unsplash_result.ok();
    let fanart_resp = fanart_result.ok();

    // Parse JSON, falling back to Value::Null if missing or parsing fails
    let bing_json: Value = match bing_resp {
        Some(resp) => resp.json().await.unwrap_or(Value::Null),
        None => Value::Null,
    };

    let unsplash_json: Value = match unsplash_resp {
        Some(resp) => resp.json().await.unwrap_or(Value::Null),
        None => Value::Null,
    };

    let fanart_json: Value = if !mbid.is_empty() {
        match fanart_resp {
            Some(resp) => resp.json().await.unwrap_or(Value::Null),
            None => Value::Null,
        }
    } else {
        Value::Null
    };

    ////
    // Cache results
    ////
    // Bing
    let path = format!("cache/img/bing/{}.json", query.replace("/", "_"));
    match serde_json::to_string_pretty(&bing_json) {
        Ok(json_str) => {
            if let Err(e) = write(&path, json_str) {
                println!("Failed to write Bing cache {}: {}", path, e);
            }
        }
        Err(e) => {
            println!("Failed to serialize Bing JSON for caching: {}", e);
        }
    }

    // Unsplash
    let path = format!("cache/img/unsplash/{}.json", query.replace("/", "_"));
    match serde_json::to_string_pretty(&unsplash_json) {
        Ok(json_str) => {
            if let Err(e) = write(&path, json_str) {
                println!("Failed to write Unsplash cache {}: {}", path, e);
            }
        }
        Err(e) => {
            println!("Failed to serialize Bing JSON for caching: {}", e);
        }
    }

    // FanART
    let path = format!("cache/img/fanart/{}.json", query.replace("/", "_"));
    match serde_json::to_string_pretty(&fanart_json) {
        Ok(json_str) => {
            if let Err(e) = write(&path, json_str) {
                println!("Failed to write Unsplash cache {}: {}", path, e);
            }
        }
        Err(e) => {
            println!("Failed to serialize FanArt JSON for caching: {}", e);
        }
    }

    merge_images(
        format_bing(bing_json),
        format_unsplash(unsplash_json),
        format_fanart(fanart_json),
    )
}

fn format_bing(json: Value) -> Vec<ImgResult> {
    let mut remote_results: Vec<ImgResult> = Vec::with_capacity(50);

    if let Some(results) = json.get("data").and_then(|d| {
        d.get("result")
            .and_then(|r| r.get("items").and_then(|i| i.as_array()))
    }) {
        for item in results {
            let url = match item.get("url").and_then(|v| v.as_str()) {
                Some(u) => u,
                None => continue,
            };

            remote_results.push(ImgResult {
                thumbnail: format!(
                    "https://proxy.prieco.net/proxy?height=200&u={}",
                    urlencoding::encode(
                        item.get("media")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                    )
                ),
                image: format!(
                    "https://proxy.prieco.net/proxy?u={}",
                    urlencoding::encode(
                        item.get("media")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                    )
                ),

                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                site_url: url.to_string(),
                site_domain: get_domain(url, true),

                favicon: item
                    .get("favicon")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }

    remote_results
}
fn format_unsplash(json: serde_json::Value) -> Vec<ImgResult> {
    let mut remote_results: Vec<ImgResult> = Vec::with_capacity(20);

    if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
        for item in results {
            // The page URL
            let site_url = item
                .get("links")
                .and_then(|l| l.get("html"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // The image URLs
            let urls = item.get("urls");
            remote_results.push(ImgResult {
                thumbnail: format!(
                    "https://proxy.prieco.net/proxy?height=200&u={}",
                    urls.and_then(|u| u.get("thumb"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                ),
                image: format!(
                    "https://proxy.prieco.net/proxy?u={}",
                    urls.and_then(|u| u.get("regular"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                ),

                title: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),

                site_url: site_url.to_string(),
                site_domain: get_domain(site_url, true),

                favicon: format!("https://{}/favicon.ico", get_domain(site_url, false)),
            });
        }
    }

    remote_results
}
fn format_fanart(json: serde_json::Value) -> Vec<ImgResult> {
    let mut remote_results: Vec<ImgResult> = Vec::with_capacity(100);

    // Helper closure to push images from an array
    let mut push_images = |arr_opt: Option<&serde_json::Value>, category: &str| {
        if let Some(arr) = arr_opt.and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                    remote_results.push(ImgResult {
                        thumbnail: format!("https://proxy.prieco.net/proxy?height=200&u={}", url),
                        image: format!("https://proxy.prieco.net/proxy?u={}", url),
                        title: format!("Category: {}", category),
                        site_url: String::from("https://fanart.tv/"),
                        site_domain: String::from("fanart.tv"),
                        favicon: String::from("https://fanart.tv/favicon.ico"),
                    });
                }
            }
        }
    };

    // Artist-level artwork
    push_images(json.get("artistbackground"), "artistbackground");
    push_images(json.get("artistthumb"), "artistthumb");
    // Album-level artwork
    if let Some(albums) = json.get("albums").and_then(|v| v.as_object()) {
        for (album_id, album_data) in albums {
            push_images(
                album_data.get("albumcover"),
                &format!("albumcover:{}", album_id),
            );
            push_images(album_data.get("cdart"), &format!("cdart:{}", album_id));
        }
    }
    push_images(json.get("musicbanner"), "musicbanner");

    remote_results
}

fn merge_images(
    karma: Vec<ImgResult>,
    unsplash: Vec<ImgResult>,
    fanart: Vec<ImgResult>,
) -> Vec<ImgResult> {
    let mut merged = Vec::with_capacity(karma.len() + unsplash.len());

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    while i < karma.len() || j < unsplash.len() || k < fanart.len() {
        // Take up to 2 from karma
        for _ in 0..2 {
            if i < karma.len() {
                merged.push(karma[i].clone());
                i += 1;
            }
        }

        // Take 1 from unsplash
        if j < unsplash.len() {
            merged.push(unsplash[j].clone());
            j += 1;
        }

        // Take up to 3 from FanArt
        for _ in 0..3 {
            if k < fanart.len() {
                merged.push(fanart[k].clone());
                k += 1;
            }
        }
    }

    merged
}
