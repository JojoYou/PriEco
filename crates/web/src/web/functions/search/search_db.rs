//! # Index search
//!
//! Performs an index search (JSON) and creates search results out of it.
//!
//! ## Architecture
//!
//! 1. [**run()**:][run] Calls [run_json]() and creates search results.
//! 2. [**run_json()**:][run_json] Performs search indexes pipeline.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-08-11
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{collections::HashMap, sync::Arc};

/*
  Import external libraries
*/
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde_json::Value as Json_Value;

/*
  Import own libraries
*/
use crate::web::functions::{general::get_domain, ranking::goggles::GoggleRules, search::pipeline};
use prieco_core::globals::{EmbeddingService, SearchResult, WebDocument};

/*
  Structures
*/
pub static QUERY_CACHE: Lazy<RwLock<HashMap<String, Vec<WebDocument>>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(1_000)));

/// # Calls [run_json]() and creates [SearchResult]
///
/// This funtion calls [run_json]() for JSON results.
/// Generates result info, the 3 dots next to each result.
/// Formats JSON as a [SearchResult] object and pushes them to a vector.
///
/// # Arguments
///
/// * `results` - Mutable vector of [SearchResult].
/// * `query` - Search query.
/// * `lang` - Prefered language.
/// * `loc` - Prefered location.
/// * `embedding_manager` - Query embedder.
/// * `goggles` - Filters.
/// * `mobile` - Is user using mobile.
///
/// # Returns
///
/// None
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn run(
    results: &mut Vec<SearchResult>,
    query: &str,
    lang: &str,
    loc: &str,
    embedding_service: &EmbeddingService,
    iroh_endpoint: &Endpoint,
    goggles: Vec<Arc<GoggleRules>>,
    mobile: bool,
) {
    let local_results = pipeline::run(
        query,
        lang,
        loc,
        embedding_service,
        iroh_endpoint,
        goggles,
        mobile,
        false,
    )
    .await;

    let json_results = serde_json::to_value(local_results).unwrap_or(Json_Value::Null);

    // Create final results
    if let Some(arr) = json_results.as_array() {
        for item in arr {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let html_id = item
                .get("html")
                .and_then(|v| v.as_str())
                .and_then(|html_str| html_str.rsplit('/').next())
                .and_then(|f| f.strip_suffix(".zst").or_else(|| f.strip_suffix(".txt")))
                .map(|id| id.to_string());

            let confidence = item
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let reading_level = match confidence {
                c if c >= 70.0 => "📖 Easy Read",
                c if c >= 40.0 => "🎓 Intermediate Read",
                c if c > 0.0 => "🔬 Dense / Academic Read",
                _ => "📄 Unknown Read",
            }
            .to_string();

            let load_time = item.get("load").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let formatted_load = if load_time <= 0.0 {
                "❓ Unknown speed"
            } else if load_time < 1.0 {
                "⚡⚡⚡"
            } else if load_time < 2.5 {
                "⚡⚡"
            } else {
                "⚡"
            };

            let raw_intent = item.get("intent").and_then(|v| v.as_u64()).unwrap_or(5);
            let intent = match raw_intent {
                0 => "🧠 Informational",
                1 => "💳 Transactional",
                2 => "🛍️ Commercial Investigation",
                3 => "🧭 Navigational",
                4 => "📍 Local",
                _ => "🎯 Unknown Intent",
            }
            .to_string();

            let raw_source = item.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let mut source_engine = raw_source.to_string();
            if source_engine.is_empty() {
                source_engine = "🔍 PriEco Index".to_string();
            } else {
                source_engine = source_engine.replace("FTS", "🔍 Keyword");
                source_engine = source_engine.replace("IVF", "🤖 Semantic");
                source_engine = source_engine.replace("DIR", "🗂️ Directory");
                source_engine = source_engine.replace("DIS", "🌐 Discovered");
            }

            let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");

            results.push(SearchResult {
                url: url.clone(),
                display_url: url
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("www.", "")
                    .trim_end_matches('/')
                    .replace("/", " › "),
                domain: get_domain(&url, true),
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: item
                    .get("image")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        format!(
                            "<img loading='lazy' alt='‎' src='/proxy?u={}'>",
                            urlencoding::encode(s)
                        )
                    })
                    .unwrap_or_default(),
                favicon: item
                    .get("favicon")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("static/prieco_favicons/{}", s))
                    .unwrap_or_else(|| {
                        let icon = format!(
                            "https://fav.prieco.net/icon?url={}&size=32",
                            urlencoding::encode(&get_domain(&url, false))
                        );
                        format!("/proxy?u={}", urlencoding::encode(&icon))
                    }),
                html_id,

                reading_level,
                formatted_load: formatted_load.to_string(),
                source_engine,
                content: content.to_string(),
                intent,
            });
        }
    }
}

pub async fn run_json(
    query: &str,
    lang: &str,
    loc: &str,
    embedding_manager: &EmbeddingService,
    iroh_endpoint: &Endpoint,
    goggles: Vec<Arc<GoggleRules>>,
    mobile: bool,
    decentralized: bool,
) -> Vec<Json_Value> {
    let docs = pipeline::run(
        query,
        lang,
        loc,
        embedding_manager,
        iroh_endpoint,
        goggles,
        mobile,
        decentralized,
    )
    .await;

    docs.into_iter()
        .take(20)
        .filter_map(|s| serde_json::to_value(s).ok())
        .collect()
}

pub async fn run_json_docs(
    query: &str,
    lang: &str,
    loc: &str,
    mobile: bool,
    embedding_manager: &EmbeddingService,
) -> Vec<WebDocument> {
    pipeline::run(
        query,
        lang,
        loc,
        embedding_manager,
        iroh_endpoint,
        vec![],
        mobile,
        false,
    )
    .await
}
