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

        return merge_images(
            format_bing(serde_json::from_str(&read_file(&bing_file)).unwrap()),
            format_unsplash(serde_json::from_str(&read_file(&unsplash_file)).unwrap()),
            format_fanart(serde_json::from_str(&read_file(&fanart_file)).unwrap()),
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

    let client = Client::builder()
        .user_agent("PriEco/1.0.0 ( support@jojoyou.org )")
        .timeout(Duration::from_secs(2))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .unwrap();

    // Await the results of both requests
    let (bing_results, unsplash_results, fanart_results) = tokio::try_join!(
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
    )
    .unwrap();

    // Parse the responses as JSON
    let bing_json: Value = bing_results.json().await.unwrap();
    let unsplash_json: Value = unsplash_results.json().await.unwrap();
    let fanart_json: Value = if !mbid.is_empty() {
        fanart_results.json().await.unwrap()
    } else {
        Value::Null
    };

    ////
    // Cache results
    ////
    // Bing
    write(
        &format!("cache/img/bing/{}.json", query.replace("/", "_"),),
        serde_json::to_string_pretty(&bing_json).unwrap(),
    )
    .unwrap();
    // Unsplash
    write(
        &format!("cache/img/unsplash/{}.json", query.replace("/", "_"),),
        serde_json::to_string_pretty(&unsplash_json).unwrap(),
    )
    .unwrap();
    // FanART
    write(
        &format!("cache/img/fanart/{}.json", query.replace("/", "_"),),
        serde_json::to_string_pretty(&fanart_json).unwrap(),
    )
    .unwrap();

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
