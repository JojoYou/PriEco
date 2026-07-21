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
use prieco_blob::blob::decode_blob_to_text;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};
use rocket::{State, serde::json::Json};
use serde_json::Value as Json_Value;
use tantivy::{
    Term,
    collector::TopDocs,
    query::{BooleanQuery, Occur, Query, QueryParser, TermQuery, TermSetQuery},
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
    goggles: Vec<Arc<GoggleRules>>,
) {
    let local_results = run_json(q, lang, loc, embedding_service, goggles).await;

    // Create final results
    if let Some(arr) = Json(Json_Value::from(local_results)).as_array() {
        for item in arr {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let url_enc = urlencoding::encode(&url).into_owned();

            let html_id = item
                .get("html")
                .and_then(|v| v.as_str())
                .and_then(|html_str| html_str.rsplit('/').next())
                .and_then(|f| f.strip_suffix(".zst").or_else(|| f.strip_suffix(".txt")))
                .map(|id| id.to_string());

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
                html_id,
                url_enc,
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
    goggles: Vec<Arc<GoggleRules>>,
) -> Vec<Json_Value> {
    // Cache
    let cache_key = format!("{}_{}_{}", query, lang, loc);
    let cached_data = {
        let cache = QUERY_CACHE.read();
        cache.get(&cache_key).cloned()
    };

    let q_clone = query.to_string();
    let mut fts_query = q_clone.clone();
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
        /*let embed: Vec<f32> = match embedding_service.embed_query(&contextual_query).await {
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
        };*/
        let embed: Vec<f32> = Vec::new();

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
            println!("Tantivy took {elapsed:.3}s");
            stdout().flush().ok();
            res
        });

        let vector_task = tokio::task::spawn_blocking(move || {
            return Vec::new();
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

    // Stage: 2
    // Hand ranking
    ranking::hand::run(&mut results, query, lang, loc, &intent, &goggles);

    // Stage: 3
    // Reranker + PageRank
    /*let mut pagerank_time_total: f32 = 0.0;
    let mut rerank_time_total: f32 = 0.0;
    let candidates = results.len().min(RERANK_CUTOFF);
    if candidates > 0 {
        // Rerank
        let rerank_start = Instant::now();
        let passages: Vec<String> = results[..candidates]
            .iter()
            .map(|d| format!("{} {} {}", d.title, d.description, d.content))
            .collect();

        let q = query.to_string();
        let scores = tokio::task::spawn_blocking(move || RERANKER.score_batch(&q, &passages))
            .await
            .unwrap_or_else(|_| vec![0.0; candidates]);
        rerank_time_total = rerank_start.elapsed().as_secs_f32();

        // PageRank
        let pr_start = Instant::now();
        for (i, doc) in results[..candidates].iter_mut().enumerate() {
            doc.search_score *= 1.0 + PAGERANK.read().get_score(&doc.url);
            doc.search_score *= 0.7 + (1.0 / (1.0 + (-scores[i]).exp())) * 0.6;
        }
        pagerank_time_total = pr_start.elapsed().as_secs_f32();
    }
    results[..candidates].sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    println!(
        "PageRank: {}s\nRerank: {}s",
        pagerank_time_total, rerank_time_total
    );*/

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
    let shown_results: Vec<WebDocument> = results.iter().take(20).cloned().collect();
    /*let html_ids: Vec<u64> = shown_results
    .iter()
    .filter_map(|doc| {
        doc.html
            .split('/')
            .nth(1)?
            .trim_end_matches(".zst")
            .trim_end_matches(".txt")
            .parse::<u64>()
            .ok()
    })
    .collect();*/

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

    /*benchmark_concurrent_fetch(html_ids).await;*/

    serialized_sites
}

/* Index search functions */
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

    let mut query_parser = QueryParser::for_index(
        &TANTIVY_INDEX,
        vec![
            title_field,
            description_field,
            content_field,
            keywords_field,
        ],
    );

    query_parser.set_field_boost(title_field, 3.0);
    query_parser.set_field_boost(keywords_field, 2.0);
    query_parser.set_field_boost(description_field, 1.5);
    query_parser.set_field_boost(content_field, 1.0);

    let parsed_query = query_parser.parse_query(query_text).ok()?;

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, parsed_query)];

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

pub async fn benchmark_concurrent_fetch(mut top_30_ids: Vec<u64>) {
    println!(
        "{}Starting Sorted I/O + Parallel CPU Benchmark...{}",
        colors::BLUE,
        colors::RESET
    );

    let total_start = Instant::now();

    top_30_ids.sort_unstable();

    let target_set: HashSet<u64> = top_30_ids.iter().copied().collect();
    let total_targets = target_set.len();

    let mut tasks = Vec::new();
    let mut found_count = 0;

    let io_start = Instant::now();

    if let Some(&first_id) = top_30_ids.first() {
        let start_key = first_id.to_le_bytes();

        let iter = PRIECO_FJALL.blobs_ks.range(start_key..);

        for guard in iter {
            if found_count >= total_targets {
                break;
            }

            let check_result = guard.into_inner_if(|k| {
                let key_arr: [u8; 8] = k.as_ref().try_into().unwrap_or([0; 8]);
                let current_id = u64::from_le_bytes(key_arr);
                target_set.contains(&current_id)
            });

            if let Ok((key_bytes, Some(raw_blob))) = check_result {
                let key_arr: [u8; 8] = key_bytes.as_ref().try_into().unwrap_or([0; 8]);
                let current_id = u64::from_le_bytes(key_arr);

                found_count += 1;

                let blob_owned = raw_blob.to_vec();

                let task = tokio::task::spawn_blocking(move || {
                    let decode_start = Instant::now();
                    let html_text = decode_blob_to_text(&blob_owned);
                    let decode_ms = decode_start.elapsed().as_millis();

                    (current_id, decode_ms, html_text.len())
                });

                tasks.push(task);
            }
        }
    }

    let io_elapsed = io_start.elapsed().as_millis();
    println!(
        "{}HDD Sequential Scan Completed in {}ms{}",
        colors::YELLOW,
        io_elapsed,
        colors::RESET
    );

    let mut total_bytes = 0;
    for task in tasks {
        match task.await {
            Ok((id, decode_ms, size)) => {
                total_bytes += size;
                println!(
                    "Blob {} | CPU Decode: {:>3}ms | Output Size: {}",
                    id, decode_ms, size
                );
            }
            Err(e) => println!("Task panicked: {}", e),
        }
    }

    let elapsed = total_start.elapsed();
    println!(
        "{}Benchmark Complete!{} Fetched & Decoded {} blobs in {:.2?} (Total HTML size: {} bytes)",
        colors::GREEN,
        colors::RESET,
        found_count,
        elapsed,
        total_bytes
    );
}
