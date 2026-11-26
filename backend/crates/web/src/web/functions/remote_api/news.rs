//! # News
//!
//! PriEco news results from AlterLab news API.
//!
//! * Results are cached in RAM for 15 minutes.
//! * 2s timeout, errors give no news.
//! * Images go through the proxy.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-04-14
//! * Last Modified: 2026-11-24
//!
//! ## Planned Improvements
//!
//! - [ ] None

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
/// News API.
const BASE_URL: &str = "https://api.alterlab.io/api/v1/news";
/// How long news are cached.
const CACHE_TTL: Duration = Duration::from_secs(900);

/// 'query_lang_count' → (time, articles). Cleared over 5000 entries.
static NEWS_CACHE: Lazy<RwLock<HashMap<String, (Instant, Vec<Article>)>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(1_000)));

/*
  Structures
*/
/// News result.
#[derive(Serialize, Clone)]
pub struct Article {
    pub title: String,
    pub url: String,
    /// Source name, otherwise domain.
    pub domain: String,
    pub favicon: String,
    /// Proxied image.
    pub image: String,
    /// '5 min ago', '3 h ago', '2 days ago'.
    pub published: String,
    pub description: String,
}

/// API response.
#[derive(Deserialize)]
pub struct NewsResponse {
    #[serde(default)]
    pub items: Vec<NewsItem>,
}

/// API news item.
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

/// News for query.
///
/// # Arguments
///
/// * `query` - Query.
/// * `lang` - Language, 'all' for any.
/// * `count` - Max articles.
///
/// # Returns
///
/// Articles, empty without `NEWS_API_KEY` or on error.
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

/// API response to [Article]s, `now` is used for 'time ago'.
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

/// Time since RFC 3339 date, empty if it can't be parsed.
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
