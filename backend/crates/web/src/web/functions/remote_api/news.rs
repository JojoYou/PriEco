//!  File: web/modules/search_api/news.rs
//!  Description: Manages PriEco News
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-04-14
//!  Last Modified: 2026-11-24
//!
//!  Usage Call this to get news results
//!  TODO:

/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/*
  Import external libraries
*/
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/*
  Import own libraries
*/
use crate::web::functions::general::{favicon_url, get_domain};
use prieco_core::CLIENT;

/*
  Constants
*/
const BASE_URL: &str = "https://api.alterlab.io/api/v1/news";
const CACHE_TTL: Duration = Duration::from_secs(900);

static NEWS_CACHE: Lazy<RwLock<HashMap<String, (Instant, Vec<Article>)>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(1_000)));

/*
  Structures
*/
#[derive(Serialize, Clone)]
pub struct Article {
    pub title: String,
    pub url: String,
    pub domain: String,
    pub favicon: String,
    pub image: String,
    pub published: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct NewsResponse {
    #[serde(default)]
    pub items: Vec<NewsItem>,
}

#[derive(Deserialize)]
pub struct NewsItem {
    pub headline: String,
    pub url: String,
    #[serde(default)]
    pub source_name: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub snippet: Option<String>,
}

pub async fn run(query: &str, lang: &str, count: usize) -> Vec<Article> {
    let cache_key = format!("{}_{}_{}", query, lang, count);
    if let Some((at, cached)) = NEWS_CACHE.read().get(&cache_key)
        && at.elapsed() < CACHE_TTL
    {
        return cached.clone();
    }

    let Ok(api_key) = std::env::var("NEWS_API_KEY") else {
        eprintln!("Warning: NEWS_API_KEY is missing!");
        return Vec::new();
    };

    let mut url = format!(
        "{}?q={}&limit={}",
        BASE_URL,
        urlencoding::encode(query),
        count
    );
    if lang != "all" {
        url.push_str(&format!("&language={}", urlencoding::encode(lang)));
    }

    let Ok(resp) = CLIENT
        .get(&url)
        .header("X-API-Key", api_key)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    else {
        return Vec::new();
    };

    if !resp.status().is_success() {
        return Vec::new();
    }

    let Ok(text) = resp.text().await else {
        return Vec::new();
    };

    let Ok(parsed) = serde_json::from_str::<NewsResponse>(&text) else {
        return Vec::new();
    };

    let news = format_news(parsed, OffsetDateTime::now_utc());

    let mut cache = NEWS_CACHE.write();
    if cache.len() > 5_000 {
        cache.clear();
    }
    cache.insert(cache_key, (Instant::now(), news.clone()));

    news
}

pub fn format_news(response: NewsResponse, now: OffsetDateTime) -> Vec<Article> {
    response
        .items
        .into_iter()
        .map(|item| Article {
            domain: if item.source_name.is_empty() {
                get_domain(&item.url, true)
            } else {
                item.source_name
            },
            favicon: favicon_url(&item.url),
            image: item
                .image
                .filter(|s| !s.is_empty())
                .map(|s| format!("/proxy?u={}", urlencoding::encode(&s)))
                .unwrap_or_default(),
            published: time_ago(&item.published_at, now),
            description: item.snippet.unwrap_or_default(),
            title: item.headline,
            url: item.url,
        })
        .collect()
}

fn time_ago(published: &str, now: OffsetDateTime) -> String {
    let Ok(date) = OffsetDateTime::parse(published, &Rfc3339) else {
        return String::new();
    };

    let minutes = (now - date).whole_minutes().max(0);
    match minutes {
        0..60 => format!("{} min ago", minutes),
        60..1440 => format!("{} h ago", minutes / 60),
        _ => format!("{} days ago", minutes / 1440),
    }
}
