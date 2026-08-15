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
use std::{
    collections::{HashMap, HashSet},
    io::{Write, stdout},
    sync::Arc,
    time::Instant,
};

/*
  Import external libraries
*/
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};
use rocket::{State, serde::json::Json};
use serde_json::Value as Json_Value;
use tantivy::{
    Term,
    collector::TopDocs,
    query::{BooleanQuery, BoostQuery, Occur, Query, QueryParser, TermQuery, TermSetQuery},
    schema::{IndexRecordOption, Value},
};
use zstd::bulk::Decompressor;

/*
  Import own libraries
*/
use crate::web::functions::{
    additional::discover::discover_and_ping_domains,
    general::get_domain,
    ranking::{self, goggles::GoggleRules},
};
use prieco_core::{
    META_DECODER, PRIECO_FJALL, QueryIntent,
    globals::{
        EmbeddingService, PAGERANK, RERANKER, SearchResult, TANTIVY_INDEX, TANTIVY_READER,
        VECTOR_CENTROPOIDS, WebDocument, colors,
    },
    url_to_domain_id, url_to_id,
};

/*
  Constants
*/
const MAX_FTS: usize = 80;
const MAX_IVF: usize = 200;
const NPROBS: usize = 4;
const PAGERANK_CUTOFF: usize = 100;
const RERANK_CUTOFF: usize = 30;
const MAX_PER_DOMAIN: usize = 5;

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
    embedding_service: &State<EmbeddingService>,
    goggles: Vec<Arc<GoggleRules>>,
    mobile: bool,
) {
    let local_results = run_json(query, lang, loc, embedding_service, goggles, mobile).await;

    // Create final results
    if let Some(arr) = Json(Json_Value::from(local_results)).as_array() {
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

/// # Search Index Pipeline
///
/// This function checks if results for the same query + lang + loc are already in RAM cache.
/// It's unsed internally and for API calls.
///
/// Performs [Intent classification][ranking::meaning::call::process_query],
/// which also returns coordinates if the query contains a place.
/// And synonym expansion for full-text search.
///
/// Embeds query.
///
/// Performs 4 differnt styles of searches at the same time.
/// * Direct search
///     * Checks for domain results of .com, .net, .org where domain is trimmed query.
///     * Helpful for root domain searches such as YouTube.
/// * Discovery search
///     * Pings a lot of domains matching query. And waits (max 1s) for 200 response code.
///     * Useful for root domain searches that PriEco doesn't yet know.
///     * Successful findings are sent to PriEco web crawler.
/// * Full-text search
///     * PriEco uses Tantivy(https://github.com/quickwit-oss/tantivy).
///     * Tantivy checks for keywords in web page `title`, `description`, `content` (first 500 page characters), `keywords` (manually set keywords by page).
///     * Many rules are applied during this search, especially with query intent and preferred language.
/// * Vector search
///     * IVF index made by me. I needed a simple vector search that embeds query, finds closest centropoids, mmap sequencially reads those buckets and gets the closest vectors to the query in cosine similarity.
///     * Vectors are normalized to size of 1 for faster math.
///
/// * Reciprocal Rank Fusion merge & Deduplication.
///
/// * Goggle discard filter.
///
/// * [Hand ranking][ranking::hand::run] (Weights calculated using genetic algorith on NDCG test).
///
/// * Reranker + PageRank.
///
/// * Caps results from single domain.
///
/// # Arguments
///
/// * `query` - Search query.
/// * `lang` - Prefered language.
/// * `loc` - Prefered location.
/// * `embedding_manager` - Query embedder.
/// * `goggles` - Filters.
/// * `mobile` - Is user using mobile.
///
/// # Returns
///
/// Vector of JSONs that are made form [WebDocument] objects.
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn run_json(
    query: &str,
    lang: &str,
    loc: &str,
    embedding_manager: &State<EmbeddingService>,
    goggles: Vec<Arc<GoggleRules>>,
    mobile: bool,
) -> Vec<Json_Value> {
    // Cache
    let cache_key = format!("{}_{}_{}", query, lang, loc);
    let cached_data = {
        let cache = QUERY_CACHE.read();
        cache.get(&cache_key).cloned()
    };

    // Clone query to pass to parallel index calls
    let q_clone = query.to_string();
    let mut fts_query = q_clone.clone();
    let fts_original_query = q_clone.clone();
    let q_clone3 = q_clone.clone();
    let q_clone4 = q_clone.to_string();

    // Clasify query intent
    let (intent, coords) = ranking::meaning::call::process_query(&mut fts_query, lang, loc);

    let mut results = if let Some(cached) = cached_data {
        cached
    } else {
        // Context
        // Local query, add loc to embed for better locality
        let contextual_query = if intent == QueryIntent::Local
            || (query.split_whitespace().count() <= 2 && !loc.is_empty())
        {
            format!("{} {}", query, loc)
        } else {
            query.to_string()
        };

        // Query to embed (vector)
        let embed: Vec<f32> = match embedding_manager.embed_query(&contextual_query).await {
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

                return id_score_vec;
            }

            for tld in &[".com", ".net", ".org"] {
                for prefix in &[
                    "",
                    "www.",
                    "http://",
                    "http://www.",
                    "https://",
                    "https://www.",
                ] {
                    let url = format!("{}{}{}/", prefix, trimmed, tld);
                    let id = url_to_id(&url);
                    id_score_vec.push((id, 0.0));
                }
            }

            let elapsed = start.elapsed().as_secs_f32();
            println!("DIR lookup took {elapsed:.3}s");
            stdout().flush().ok();

            id_score_vec
        });

        let lang_clone = lang.to_string();
        let loc_clone = loc.to_string();
        let goggles_clone = goggles.clone();
        let tantivy_task = tokio::task::spawn_blocking(move || {
            if (matches!(lang_clone.as_str(), "zh" | "ja" | "ko" | "th")
                && fts_original_query.chars().count() >= 15)
                || fts_original_query.split_whitespace().count() >= 8
            {
                return Vec::new();
            }

            let start = Instant::now();

            // Filter: site:
            if fts_query.starts_with("site:") {
                fts_query = fts_query.replace("site:", "");

                fts_query = fts_query
                    .rsplit_once('.')
                    .map(|(left, _)| left.to_string())
                    .unwrap_or(fts_query);
                fts_query = fts_query
                    .rsplit_once('.')
                    .map(|(_, right)| right.to_string())
                    .unwrap_or(fts_query);
            }

            let res = search_tantivy(
                &fts_query,
                &lang_clone,
                &loc_clone,
                &intent,
                MAX_FTS,
                &goggles_clone,
            )
            .unwrap_or_default();

            let elapsed = start.elapsed().as_secs_f32();
            println!(
                "Tantivy took {elapsed:.3}s, {} segments",
                TANTIVY_INDEX
                    .searchable_segments()
                    .unwrap_or_default()
                    .len()
            );
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
            let s = Instant::now();
            let res: Vec<(u64, f32)> = VECTOR_CENTROPOIDS
                .search(&embed, 0, NPROBS)
                .unwrap_or_default();
            println!("Nprobs: {}", s.elapsed().as_secs_f32());

            let vector_ids: Vec<(u64, f32)> = res.iter().take(MAX_IVF).cloned().collect();

            let elapsed = start.elapsed().as_secs_f32();
            println!("Vector search took {elapsed:.3}s");
            stdout().flush().ok();

            vector_ids
        });

        // Web discovery
        let discovery_task =
            tokio::spawn(async move { discover_and_ping_domains(&q_clone4).await });

        let (tantivy_ids, vector_ids, dir_ids, discovery_results) =
            tokio::join!(tantivy_task, vector_task, dir_task, discovery_task);

        let total_elapsed = total_start.elapsed().as_secs_f32();
        println!("Total concurrent time {total_elapsed:.3}s");

        // Fetch documents
        let mut engines_map: HashMap<&'static str, Vec<(u64, f32)>> = HashMap::new();
        engines_map.insert("FTS", tantivy_ids.unwrap_or_default());
        engines_map.insert("IVF", vector_ids.unwrap_or_default());
        engines_map.insert("DIR", dir_ids.unwrap_or_default());

        let z = Instant::now();
        let mut fetched_results = fetch_documents(engines_map);
        println!("Fetch took: {}s", z.elapsed().as_secs_f32());

        // Split
        let mut dir_results: Vec<WebDocument> = fetched_results.remove("DIR").unwrap_or_default();
        let mut tantivy_results: Vec<WebDocument> =
            fetched_results.remove("FTS").unwrap_or_default();
        let mut vector_results: Vec<WebDocument> =
            fetched_results.remove("IVF").unwrap_or_default();
        let mut discovery_results: Vec<WebDocument> = discovery_results.unwrap_or_default();

        // Sort each result vector in-place by search_score descending
        dir_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        tantivy_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        vector_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
        discovery_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

        println!("DIR: {}", dir_results.len());
        println!("Tantivy: {}", tantivy_results.len());
        println!("IVF: {}", vector_results.len());
        println!("DIS: {}", discovery_results.len());

        // RRF Merge & Deduplicate
        let mut results: Vec<WebDocument> = ranking::rrf::run(
            query,
            lang,
            &intent,
            dir_results,
            tantivy_results,
            vector_results,
            discovery_results,
            60.0,
        );

        // Temp remove blocked terms
        let blocked_terms: &[&str] = &["porn", "sex", "dick", "fap"];
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

    // Goggle Discard filter
    results.retain(|doc| {
        let domain_id = url_to_domain_id(&doc.url);

        if goggles.iter().any(|g| g.discard.contains(&domain_id)) {
            return false;
        }

        let discard_active = goggles.iter().any(|g| g.discard_by_default);
        if discard_active {
            return goggles.iter().any(|g| {
                g.boost.contains_key(&domain_id)
                    || g.path.iter().any(|(p, _)| path_matches(&doc.url, p))
            });
        }

        true
    });

    if results.is_empty() {
        println!("No results → confidence = 0.0 (force fallback)");
        return Vec::new();
    }

    // Hand ranking
    ranking::hand::run(&mut results, query, lang, loc, &intent, &goggles, mobile);
    results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

    // PageRank
    let pr_candidates = results.len().min(PAGERANK_CUTOFF);

    let mut pagerank_time_total: f32 = 0.0;
    if pr_candidates > 0 {
        let pr_start = Instant::now();

        for doc in results[..pr_candidates].iter_mut() {
            let pr_score = PAGERANK.read().get_score(&doc.url);
            doc.search_score *= 1.0 + pr_score;
        }
        pagerank_time_total = pr_start.elapsed().as_secs_f32();

        results[..pr_candidates]
            .sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    }

    // RERANKER
    let mut rerank_time_total: f32 = 0.0;
    let nn_candidates = results.len().min(RERANK_CUTOFF);
    if nn_candidates > 0 {
        let rerank_start = Instant::now();
        let passages: Vec<String> = results[..nn_candidates]
            .iter()
            .map(|d| format!("{} {} {}", d.title, d.description, d.content))
            .collect();

        let scores = RERANKER
            .score_batch(query, &passages)
            .await
            .unwrap_or_else(|_| vec![0.0; nn_candidates]);
        rerank_time_total = rerank_start.elapsed().as_secs_f32();

        for (i, doc) in results[..nn_candidates].iter_mut().enumerate() {
            let neural_relevance = 1.0 / (1.0 + (-scores[i]).exp());
            doc.search_score *= neural_relevance;
        }

        results[..nn_candidates]
            .sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    }
    println!(
        "PageRank: {}s\nRerank: {}s",
        pagerank_time_total, rerank_time_total
    );

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
    let shown_results: Vec<WebDocument> = results.iter().take(20).cloned().collect();
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

/// # Performs FTS index search.
///
/// This funtion prepares a Tantivy call and performs it.
/// FOr curation uses [QueryIntent], prefered lang and goggles (discard filter).
///
/// # Arguments
///
/// * `query_text` - Search query.
/// * `lang` - Prefered language.
/// * `loc` - Prefered location.
/// * `intent` - [QueryIntent].
/// * `limit` - How many results to return.
/// * `goggles` - Filters.
///
/// # Returns
///
/// Option, vector of [WebDocument] IDs and their BM25 scores.
///
/// # Panics
///
/// Only if system runs out of memory.
fn search_tantivy(
    query_text: &str,
    lang: &str,
    loc: &str,
    intent: &QueryIntent,
    limit: usize,
    goggles: &[Arc<GoggleRules>],
) -> Option<Vec<(u64, f32)>> {
    let schema = TANTIVY_INDEX.schema();
    let doc_id = schema.get_field("doc_id").ok()?;
    let domain_id_field = schema.get_field("domain_id").ok()?;

    let title_field = schema.get_field("title").ok()?;
    let description_field = schema.get_field("description").ok()?;
    let content_field = schema.get_field("content").ok()?;
    let keywords_field = schema.get_field("keywords").ok()?;

    let lang_field = schema.get_field("lang").ok()?;
    let loc_field = schema.get_field("loc").ok()?;

    let intent_field = schema.get_field("intent").ok()?;

    let query_parser = QueryParser::for_index(
        &TANTIVY_INDEX,
        vec![
            title_field,
            description_field,
            content_field,
            keywords_field,
        ],
    );

    let parsed_query = query_parser.parse_query(query_text).ok()?;

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, parsed_query)];

    let intent_val: u64 = match intent {
        QueryIntent::Informational => 0,
        QueryIntent::Transactional => 1,
        QueryIntent::CommercialInvestigation => 2,
        QueryIntent::Navigational => 3,
        QueryIntent::Local => 4,
        QueryIntent::Unknown => 5,
    };
    if intent != &QueryIntent::Unknown {
        let intent_term = Term::from_field_u64(intent_field, intent_val);
        let intent_term_query = Box::new(TermQuery::new(intent_term, IndexRecordOption::Basic));

        let boosted_intent_query = Box::new(BoostQuery::new(intent_term_query, 2.0));

        clauses.push((Occur::Should, boosted_intent_query));
    }

    if lang != "all" && !lang.is_empty() {
        let lang_term = Term::from_field_text(lang_field, lang);
        let lang_query = Box::new(TermQuery::new(lang_term, IndexRecordOption::Basic));

        if intent == &QueryIntent::Navigational {
            let boosted_lang = Box::new(BoostQuery::new(lang_query, 2.0));
            clauses.push((Occur::Should, boosted_lang));
        } else {
            clauses.push((Occur::Must, lang_query));
        }
    }

    if intent != &QueryIntent::Navigational && lang != "all" && !lang.is_empty() {
        let lang_term = Term::from_field_text(lang_field, lang);
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(lang_term, IndexRecordOption::Basic)),
        ));
    }

    if intent == &QueryIntent::Local && loc != "all" && !loc.is_empty() {
        let loc_term = Term::from_field_text(loc_field, loc);
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(loc_term, IndexRecordOption::Basic)),
        ));
    }

    let discard_goggles: Vec<&GoggleRules> = goggles
        .iter()
        .map(|g| g.as_ref())
        .filter(|g| g.discard_by_default)
        .collect();
    if !discard_goggles.is_empty() {
        let terms: Vec<Term> = discard_goggles
            .iter()
            .flat_map(|g| g.boost.keys())
            .map(|id| Term::from_field_u64(domain_id_field, *id))
            .collect();
        if !terms.is_empty() {
            clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
        }
    }

    let final_query: Box<dyn Query> = if clauses.len() == 1 {
        clauses.pop().unwrap().1
    } else {
        Box::new(BooleanQuery::new(clauses))
    };

    let searcher = TANTIVY_READER.searcher();
    let top_docs = searcher
        .search(&final_query, &TopDocs::with_limit(limit))
        .ok()?;

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

    Some(id_score_vec)
}

/* Helper functions*/

/// # Fetches documents from [PRIECO_FJALL] meta storage.
///
/// This funtion takes in batch all IDs needed for fetching.
/// Deduplicates IDs.
/// Sorts them for slightly better sequencial locality.
/// Spins up multithreated disk seeks.
/// [META_DICTIONARY] decompresses data.
///
/// # Arguments
///
/// * `engines` - Hashmap of index names and result IDs + scores they returned.
///
/// # Returns
///
/// Hashmap of index names and vector of [WebDocument].
///
/// # Panics
///
/// Only if system runs out of memory.
fn fetch_documents(
    engines: HashMap<&'static str, Vec<(u64, f32)>>,
) -> HashMap<&'static str, Vec<WebDocument>> {
    let mut unique_ids = HashSet::new();
    for id_scores in engines.values() {
        for &(id, _) in id_scores {
            unique_ids.insert(id);
        }
    }

    let mut ids_to_fetch: Vec<u64> = unique_ids.into_iter().collect();
    ids_to_fetch.sort_unstable(); // Optimistic attempt to increase locality

    let chunk_size = 100;
    let fetched_docs: HashMap<u64, WebDocument> = ids_to_fetch
        .par_chunks(chunk_size)
        .map_init(
            || Decompressor::with_prepared_dictionary(&*META_DECODER).unwrap(),
            |decompressor, chunk| {
                let mut local_docs = Vec::with_capacity(chunk.len());

                for &id in chunk {
                    let key = id.to_be_bytes();

                    // Get data
                    let compressed_data = match PRIECO_FJALL.meta_ks.get(&key) {
                        Ok(Some(data)) => data,
                        _ => {
                            continue;
                        }
                    };

                    // Decompress
                    let decompressed_buf =
                        match decompressor.decompress(compressed_data.as_ref(), 1024 * 1024) {
                            Ok(buf) => buf,
                            Err(e) => {
                                println!(
                                    "{}ZSTD DECODE ERROR for ID {}: {}{}",
                                    colors::RED,
                                    id,
                                    e,
                                    colors::RESET
                                );
                                continue;
                            }
                        };

                    // Deserialize
                    match serde_json::from_slice::<WebDocument>(&decompressed_buf) {
                        Ok(doc) => local_docs.push((id, doc)),
                        Err(e) => println!(
                            "{}JSON ERROR for ID {}: {}{}",
                            colors::RED,
                            id,
                            e,
                            colors::RESET
                        ),
                    }
                }

                local_docs
            },
        )
        .flatten()
        .collect();

    let mut final_results: HashMap<&'static str, Vec<WebDocument>> = HashMap::new();
    for (engine_name, id_scores) in engines {
        let mut docs = Vec::with_capacity(id_scores.len());
        for (id, score) in id_scores {
            if let Some(mut doc) = fetched_docs.get(&id).cloned() {
                doc.search_score = match engine_name {
                    "FTS" => score / (score + 40.0),
                    "IVF" => ((score - 0.75) / 0.25).clamp(0.0, 1.0),
                    "DIR" => 0.8,
                    _ => score,
                };
                docs.push(doc);
            }
        }
        final_results.insert(engine_name, docs);
    }

    final_results
}

fn sanitize_string(s: &str) -> String {
    s.replace('"', "").replace('\'', "")
}

/// Helper function for discarting results whose path matches discard Goggle filter
pub fn path_matches(url: &str, pattern: &str) -> bool {
    if let Some(rest) = pattern.strip_prefix('^') {
        if let Some(core) = rest.strip_suffix('$') {
            return url == core; // Exact match
        }
        return url.starts_with(rest); // Anchored start
    }
    if let Some(core) = pattern.strip_suffix('$') {
        return url.ends_with(core); // Anchored end
    }
    url.contains(pattern)
}
