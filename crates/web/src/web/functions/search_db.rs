/*
  File: web/functions/search_db.rs
  Description: Searches own index for results

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-03-31

  Usage: Run run() with parameters
  TODO: Pull up HTMLs for more context; do bm25f on htmls
*/

/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    io::{Write, stdout},
    time::Instant,
};

/*
  Import external libraries
*/
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rocket::{State, serde::json::Json};
use serde_json::Value as Json_Value;
use tantivy::{collector::TopDocs, query::QueryParser, schema::Value};

/*
  Import own libraries
*/
use crate::web::functions::{
    additional::discover::discover_and_ping_domains,
    general::get_domain,
    ranking::{self},
};
use prieco_core::{
    globals::{
        EmbeddingService, PAGERANK, RERANKER, ROCKSDB_INDEX, SearchResult, TANTIVY_INDEX,
        TANTIVY_READER, VECTOR_CENTROPOIDS, WebDocument, colors,
    },
    url_to_id,
};

/*
  Constants
*/
const MAX_FTS: usize = 80;
const MAX_IVF: usize = 200;
const NPROBS: usize = 4;
const RERANK_CUTOFF: usize = 30;
const MAX_PER_DOMAIN: usize = 5;

/*
  Structures
*/
pub static QUERY_CACHE: Lazy<RwLock<HashMap<String, Vec<WebDocument>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/*
  Description: Gets database results and confidence score

  Input: Results to add to, Query, Language, Location, Embedding service
  Output: Confidence score of the results, decides if call external APIs too & Modified results
*/
pub async fn run(
    results: &mut Vec<SearchResult>,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) {
    let local_results = run_json(q, lang, loc, embedding_service).await; // Get results from database

    // Create final results
    if let Some(arr) = Json(Json_Value::from(local_results)).as_array() {
        for item in arr {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
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
                        format!(
                            "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                            get_domain(&url, false)
                        )
                    }),
            });
        }
    }
}

/*
  Description: Queries indexes for results and returns them in a format that is convertable to SERP or JSON

  Input:
  Output:
*/
pub async fn run_json(
    query: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) -> Vec<Json_Value> {
    // Cache
    let cache_key = format!("{}_{}_{}", query, lang, loc);
    let cached_data = {
        let cache = QUERY_CACHE.read();
        cache.get(&cache_key).cloned()
    };

    let mut results = if let Some(cached) = cached_data {
        cached
    } else {
        // Query to embed (vector)
        let embed: Vec<f32> = match embedding_service.embed_query(&query).await {
            Ok(embed) => embed,
            Err(e) => {
                println!(
                    "{}Failed to embed query!{}  {}",
                    colors::YELLOW,
                    colors::RESET,
                    e
                );
                return Vec::new();
            }
        };

        let q_clone = query.to_string();
        let mut q_clone2 = q_clone.clone();
        let q_clone3 = q_clone.clone();
        let q_clone4 = q_clone.to_string();

        let total_start = Instant::now();

        let dir_task = tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let trimmed = sanitize_string(q_clone.trim()).to_lowercase();
            let mut id_score_vec: Vec<(u64, f32)> = Vec::new();

            // Filter: site:
            if trimmed.starts_with("site:") {
                let domain = trimmed.to_lowercase().replace("site:", "");
                let prefixes = [
                    "",
                    "www.",
                    "http://",
                    "http://www.",
                    "https://",
                    "https://www.",
                ];
                for prefix in &prefixes {
                    let url = format!("{}{}/", prefix, domain);
                    let id = url_to_id(&url);
                    id_score_vec.push((id, 0.0));
                }
                return fetch_documents(id_score_vec, "DIR");
            }

            let tlds = [
                ".com", ".net", ".org", ".io", ".co", ".dev", ".app", ".ai", ".info", ".biz",
                ".me", ".tv", ".us", ".uk", ".ca", ".de", ".fr", ".nl", ".eu", ".xyz", ".tech",
                ".online", ".site",
            ];
            let prefixes = [
                "",
                "www.",
                "http://",
                "http://www.",
                "https://",
                "https://www.",
            ];
            for tld in &tlds {
                for prefix in &prefixes {
                    let url = format!("{}{}{}/", prefix, trimmed, tld);
                    let id = url_to_id(&url);
                    id_score_vec.push((id, 0.0));
                }
            }
            let results: Vec<WebDocument> = fetch_documents(id_score_vec, "DIR");
            let elapsed = start.elapsed().as_secs_f32();
            println!("DIR lookup took {elapsed:.3}s");
            stdout().flush().ok();

            results
        });

        let tantivy_task = tokio::task::spawn_blocking(move || {
            let start = Instant::now();

            // Filter: site:
            if q_clone2.starts_with("site:") {
                q_clone2 = q_clone2.replace("site:", "");

                q_clone2 = q_clone2
                    .rsplit_once('.')
                    .map(|(left, _)| left.to_string())
                    .unwrap_or(q_clone2);
                q_clone2 = q_clone2
                    .rsplit_once('.')
                    .map(|(_, right)| right.to_string())
                    .unwrap_or(q_clone2);
            }

            let res = search_tantivy(&q_clone2, MAX_FTS).unwrap_or_default();

            let elapsed = start.elapsed().as_secs_f32();
            println!("Tantivy took {elapsed:.3}s");
            stdout().flush().ok();
            res
        });

        let vector_task = tokio::task::spawn_blocking(move || {
            if q_clone3.contains('"')
                || q_clone3.contains("site:")
                || q_clone3.contains("filetype:")
                || q_clone3.contains("inurl:")
                || q_clone3.contains("intitle:")
                || q_clone3.starts_with('-')
            {
                return Vec::new();
            }
            let start = Instant::now();

            // search vector DB
            let mut s = Instant::now();
            let res: Vec<(u64, f32)> = VECTOR_CENTROPOIDS
                .search(&embed, 0, NPROBS)
                .unwrap_or_default();
            println!("Nprobs: {}", s.elapsed().as_secs_f32());
            s = Instant::now();
            let res_trimmed: Vec<(u64, f32)> = res.iter().take(MAX_IVF).cloned().collect();

            let docs: Vec<WebDocument> = fetch_documents(res_trimmed, "IVF");
            println!("Fetch docs: {}", s.elapsed().as_secs_f32());

            let elapsed = start.elapsed().as_secs_f32();
            println!("Vector search took {elapsed:.3}s");
            stdout().flush().ok();

            docs
        });

        // Web discovery
        let discovery_task =
            tokio::spawn(async move { discover_and_ping_domains(&q_clone4).await });

        let (tantivy_results, vector_id_similarity, dir_results, discovery_results) =
            tokio::join!(tantivy_task, vector_task, dir_task, discovery_task);

        let total_elapsed = total_start.elapsed().as_secs_f32();
        println!("Total concurrent time {total_elapsed:.3}s");

        let mut dir_results: Vec<WebDocument> = dir_results.unwrap();
        let mut tantivy_results: Vec<WebDocument> = tantivy_results.unwrap();
        let mut vector_results: Vec<WebDocument> = vector_id_similarity.unwrap();
        let mut discovery_results: Vec<WebDocument> = discovery_results.unwrap();

        // Sort each result vector in-place by search_score descending
        dir_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        tantivy_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        vector_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        discovery_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

        println!("DIR: {}", dir_results.len());
        println!("Tantivy: {}", tantivy_results.len());
        println!("IVF: {}", vector_results.len());
        println!("DIS: {}", discovery_results.len());

        // Stage: 1
        // RRF Merge & Deduplicate
        let mut results: Vec<WebDocument> = ranking::rrf::run(
            query,
            dir_results,
            tantivy_results,
            vector_results,
            discovery_results,
            60.0,
        );

        // Temp remove blocked terms
        let blocked_terms: &[&str] = &["porn", "sex"];
        results.retain(|doc| {
            let haystack = format!(
                "{} {} {} {} {}",
                doc.title.to_lowercase(),
                doc.description.to_lowercase(),
                doc.url.to_lowercase(),
                doc.content.to_lowercase(),
                doc.keywords.to_lowercase(),
            );
            !blocked_terms.iter().any(|term| haystack.contains(term))
        });
        {
            let mut cache = QUERY_CACHE.write();
            if cache.len() > 5_000 {
                cache.clear();
            }
            cache.insert(cache_key, results.clone());
        }

        results
    };

    if results.is_empty() {
        println!("No results → confidence = 0.0 (force fallback)");
        return Vec::new();
    }

    // Stage: 2
    // Hand ranking
    ranking::hand::run(&mut results, query, lang, loc);

    // Stage: 3
    // Reranker + PageRank
    let mut pagerank_time_total: f32 = 0.0;
    let mut rerank_time_total: f32 = 0.0;
    let candidates = results.len().min(RERANK_CUTOFF);
    for doc in &mut results[..candidates] {
        // Pagerank
        let pagerank_time = Instant::now();
        let page_rank_score: f32 = PAGERANK.read().get_score(&doc.url);
        let page_rank_boost = 1.0 + page_rank_score;
        doc.search_score *= page_rank_boost;
        pagerank_time_total += pagerank_time.elapsed().as_secs_f32();

        // Rerank
        let reranking_time = Instant::now();
        let passage = format!("{} {}", doc.title, doc.description);
        let raw = RERANKER.score(query, &passage);
        let reranker_prob = 1.0 / (1.0 + (-raw).exp());

        doc.search_score = doc.search_score * (0.7 + reranker_prob * 0.6);
        rerank_time_total += reranking_time.elapsed().as_secs_f32();
    }
    results[..candidates].sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    println!(
        "PageRank: {}s\nRerank: {}s",
        pagerank_time_total, rerank_time_total
    );

    // Stage: 4
    // Cap result count from a single domain
    if !query.contains("site:") {
        let mut domain_counts: HashMap<String, usize> = HashMap::new();
        let mut deduped: Vec<WebDocument> = Vec::with_capacity(results.len());
        for doc in results {
            let domain = get_domain(&doc.url, true);
            let count = domain_counts.entry(domain).or_insert(0);
            if *count < MAX_PER_DOMAIN {
                *count += 1;
                deduped.push(doc);
            }
        }
        results = deduped;
    }

    // Trim results
    let shown_results: Vec<_> = results.iter().take(20).cloned().collect();
    let serialized_sites: Vec<_> = shown_results
        .into_iter()
        .filter_map(|s| match serde_json::to_value(s) {
            Ok(value) => Some(value),
            Err(e) => {
                println!(
                    "{}Serialization error: {}{}",
                    colors::YELLOW,
                    e,
                    colors::RESET
                );
                None
            }
        })
        .collect();

    serialized_sites
}

/* Index search functions */
fn search_tantivy(query_text: &str, limit: usize) -> Option<Vec<WebDocument>> {
    let schema = TANTIVY_INDEX.schema();
    let title_field = schema.get_field("title").ok()?;
    let description_field = schema.get_field("description").ok()?;
    let content_field = schema.get_field("content").ok()?;
    let keywords_field = schema.get_field("keywords").ok()?;
    let safe_s = schema.get_field("safe_s").ok()?;
    let doc_id = schema.get_field("doc_id").ok()?;

    let query_parser = QueryParser::for_index(
        &TANTIVY_INDEX,
        vec![
            title_field,
            description_field,
            content_field,
            keywords_field,
        ],
    );

    let query = query_parser.parse_query(query_text).ok()?;

    let searcher = TANTIVY_READER.searcher();
    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit)).ok()?;

    let mut id_score_vec: Vec<(u64, f32)> = Vec::with_capacity(top_docs.len());

    for (score, doc_address) in top_docs {
        let retrieved_doc: tantivy::TantivyDocument = match searcher.doc(doc_address) {
            Ok(doc) => doc,
            Err(e) => {
                println!(
                    "{}Tantivy: Failed to fetch document with ID {:?}: {}{}",
                    colors::YELLOW,
                    doc_address,
                    colors::RESET,
                    e
                );
                continue;
            }
        };

        if let Some(doc_id_value) = retrieved_doc.get_first(doc_id) {
            if let Some(id) = doc_id_value.as_u64() {
                id_score_vec.push((id, score));
            }
        }
    }

    // Single call to fetch_documents
    let results: Vec<WebDocument> = fetch_documents(id_score_vec, "FTS");

    Some(results)
}

/* Helper functions*/
fn fetch_documents(id_score: Vec<(u64, f32)>, idx_type: &str) -> Vec<WebDocument> {
    let (ids, scores): (Vec<u64>, Vec<f32>) = id_score.into_iter().unzip();
    let keys: Vec<[u8; 8]> = ids.iter().map(|id| id.to_be_bytes()).collect();

    let results = ROCKSDB_INDEX.multi_get(&keys);

    let mut documents = Vec::with_capacity(ids.len());

    for ((id, score), result) in ids.iter().zip(scores.iter()).zip(results) {
        let data = match result {
            Ok(Some(data)) => data,
            Ok(None) => {
                continue;
            }
            Err(_) => continue,
        };

        let mut doc: WebDocument = match serde_json::from_slice(&data) {
            Ok(doc) => doc,
            Err(e) => {
                println!(
                    "{}RocksDB: Failed to deserialize document with ID {}{}: {}",
                    colors::YELLOW,
                    id,
                    colors::RESET,
                    e
                );
                continue;
            }
        };

        // Normalize score
        doc.search_score = match idx_type {
            "FTS" => score / (score + 40.0),
            "IVF" => ((score - 0.75) / 0.25).clamp(0.0, 1.0),
            "DIR" => 0.8,
            _ => *score,
        };

        documents.push(doc);
    }

    documents
}

fn sanitize_string(s: &str) -> String {
    s.replace('"', "").replace('\'', "")
}
