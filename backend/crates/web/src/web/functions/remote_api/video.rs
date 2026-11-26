//! # Videos
//!
//! Video results from YouTube search, without API key.
//!
//! * Results are cached in RAM, cleared over 5000 queries.
//! * 2s timeout, errors give no videos.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-04-15
//! * Last Modified: 2026-11-24
//!
//! ## Planned Improvements
//!
//! - [ ] Cache expiry

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};

use prieco_core::CLIENT;

/// YouTube internal search API.
const SEARCH_URL: &str = "https://www.youtube.com/youtubei/v1/search?prettyPrint=false";

/// 'query_lang_loc' → videos.
static VIDEO_CACHE: Lazy<RwLock<HashMap<String, Vec<Video>>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(1_000)));

/// Video result.
#[derive(Serialize, Clone)]
pub struct Video {
    /// YouTube video id.
    pub id: String,
    pub title: String,
    /// Channel name.
    pub channel: String,
    /// As YouTube shows it, e.g. '2 years ago'.
    pub published: String,
    /// Duration, e.g. '12:34'.
    pub length: String,
    /// Short view count, e.g. '1.2M views'.
    pub views: String,
    pub description: String,
}

/// Videos for query.
///
/// # Arguments
///
/// * `query` - Query.
/// * `lang` - Language, 'all' is 'en'.
/// * `loc` - Country, 'all' is 'US'.
/// * `count` - Max videos.
///
/// # Returns
///
/// Videos, empty on error.
pub async fn run(query: &str, lang: &str, loc: &str, count: usize) -> Vec<Video> {
    let cache_key = format!("{}_{}_{}", query, lang, loc);
    if let Some(cached) = VIDEO_CACHE.read().get(&cache_key) {
        return cached.clone();
    }

    let hl = if lang == "all" { "en" } else { lang };
    let gl = if loc == "all" {
        String::from("US")
    } else {
        loc.to_uppercase()
    };

    let body = json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": "2.20261001.00.00",
                "hl": hl,
                "gl": gl
            }
        },
        "query": query
    });

    let Ok(resp) = CLIENT
        .post(SEARCH_URL)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .timeout(Duration::from_secs(2))
        .send()
        .await
    else {
        return Vec::new();
    };

    let Ok(text) = resp.text().await else {
        return Vec::new();
    };

    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };

    let videos = format_youtube(&json, count);

    if !videos.is_empty() {
        let mut cache = VIDEO_CACHE.write();
        if cache.len() > 5_000 {
            cache.clear();
        }
        cache.insert(cache_key, videos.clone());
    }

    videos
}

/// Videos from YouTube search JSON, other items are skipped.
fn format_youtube(json: &Value, count: usize) -> Vec<Video> {
    let Some(sections) = json
        .pointer(
            "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents",
        )
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    sections
        .iter()
        .filter_map(|s| {
            s.pointer("/itemSectionRenderer/contents")
                .and_then(Value::as_array)
        })
        .flatten()
        .filter_map(|item| item.get("videoRenderer"))
        .filter_map(|v| {
            Some(Video {
                id: v.get("videoId")?.as_str()?.to_string(),
                title: v.pointer("/title/runs/0/text")?.as_str()?.to_string(),
                channel: text_at(v, "/ownerText/runs/0/text"),
                published: text_at(v, "/publishedTimeText/simpleText"),
                length: text_at(v, "/lengthText/simpleText"),
                views: text_at(v, "/shortViewCountText/simpleText"),
                description: v
                    .pointer("/detailedMetadataSnippets/0/snippetText/runs")
                    .and_then(Value::as_array)
                    .map(|runs| {
                        runs.iter()
                            .filter_map(|r| r.get("text").and_then(Value::as_str))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .take(count)
        .collect()
}

/// String at JSON pointer, empty when missing.
fn text_at(v: &Value, pointer: &str) -> String {
    v.pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}
