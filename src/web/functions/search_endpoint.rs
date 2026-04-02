/*
  File: web/modules/search_endpoint.rs
  Description: Decided what to do with the search query

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-02-06

  Usage Call this to get results in Rocket template format (context that gets inserted to the template)
  TODO:
*/

/*
  Import system libraries
*/
use std::collections::HashMap;

/*
  Import external libraries
*/
use rocket::State;
use serde_json::{Value, json};

/*
  Import own libraries
*/
use crate::{
    globals::{EmbeddingService, SearchResult},
    web::functions::{search_api::img, search_db},
};

/*
  Description: Decides what kind of search to perform
  Input: Search type, Search query, Language, Location, Embedding service, Cookie jar
  Output: Search results in Rocket template format
*/
pub async fn run(
    t: &str,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) -> HashMap<String, Value> {
    // Don't perform a search on bang
    if q.contains("!") {
        return HashMap::new();
    }

    let mut context: HashMap<String, Value> = HashMap::with_capacity(100);

    match t {
        "img" => {
            context.insert(String::from("img_results"), json!(true));
            context.insert(
                String::from("images"),
                json!(&img::run(&q.to_lowercase().replace(" ", "+")).await),
            );
        }
        _ => {
            all_search(&mut context, q, lang, loc, embedding_service).await;
        }
    }

    context
}

async fn all_search(
    context: &mut HashMap<String, Value>,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) {
    context.insert(String::from("all_results"), json!(true)); // Set btn search type

    let mut results_vec: Vec<SearchResult> = Vec::with_capacity(100);

    let _ = search_db::run(&mut results_vec, q, lang, loc, &embedding_service).await; // Search database: Modify results + return confidence score

    // If PriEco confidence is too low, use other indexes too
    /*if !cookie_jar.get("index").is_some() && index_confidence < 0.95 {
        let mut mixed_results = Vec::with_capacity(200);
        let mut seen_urls: AHashSet<String> = AHashSet::with_capacity(100);

        let prieco_urls: AHashSet<String> = results_vec
            .iter()
            .map(|result| result.url.clone())
            .collect(); // Extract urls from PriEco index to show them when PriEco + Remote is a duplicated result

        // Call external result provider
        let remote_results: Vec<SearchResult> = loop {
            if let Some(res) = all::run(&q.to_lowercase().replace(" ", "+"), &lang, "us").await {
                break res;
            }
        };

        let mut db_iter = results_vec.into_iter();
        let mut remote_iter = remote_results.into_iter();

        // Interleave results: 2 Remote, 1 PriEco, repeat
        loop {
            let mut added_any = false;
            for _ in 0..2 {
                if let Some(google_result) = remote_iter.next() {
                    if !prieco_urls.contains(&google_result.url)
                        && seen_urls.insert(google_result.url.clone())
                    {
                        mixed_results.push(google_result);
                        added_any = true;
                    }
                }
            }
            if let Some(db_result) = db_iter.next() {
                if seen_urls.insert(db_result.url.clone()) {
                    mixed_results.push(db_result);
                    added_any = true;
                }
            }
            if !added_any {
                break;
            }
        }

        results_vec = mixed_results; // Rewrite results
    }
    // Confidence is enought + pure PriEco isn't selected
    else if !cookie_jar.get("index").is_some() {
        set_cookie(
            cookie_jar,
            String::from("prieco_searches"),
            (cookie_jar
                .get("prieco_searches")
                .and_then(|c| c.value().parse::<u64>().ok())
                .unwrap_or(0)
                + 1)
            .to_string(),
            true,
            false,
        );
    }

    // Web search was made, increment cookie counter
    if !cookie_jar.get("index").is_some() {
        set_cookie(
            cookie_jar,
            String::from("all_searches"),
            (cookie_jar
                .get("all_searches")
                .and_then(|c| c.value().parse::<u64>().ok())
                .unwrap_or(0)
                + 1)
            .to_string(),
            true,
            false,
        );
    }*/

    context.insert(String::from("results"), json!(&results_vec));

    // Yadore Ads
    //context.insert(String::from("yadore"), json!(&yadore::run(q, loc).await));
}
