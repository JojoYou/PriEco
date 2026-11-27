//! # Index search
//!
//! Performs an index search (JSON) and creates search results out of it.
//!
//! ## Architecture
//!
//! 1. [**run()**:][run] Calls [run_json]() and creates search results.
//! 2. [**run_json()**:][run_json] Performs search indexes pipeline, optionally records every stage.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-11-27
//!
//! ## Planned Improvements
//!
//! - [ ] Learn to rank
//! - [ ] Better vector index recall

/*
  Import system libraries
*/
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

/*
  Import external libraries
*/
use iroh::{EndpointAddr, PublicKey};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prieco_insert::db_insert::ingest_peer_documents;
use rayon::{current_num_threads, iter::ParallelIterator, slice::ParallelSlice};
use regex::Regex;
use rocket::{State, serde::json::Json};
use serde_json::Value as Json_Value;
use tantivy::{
    Term,
    collector::TopDocs,
    query::{BooleanQuery, Occur, Query, TermQuery, TermSetQuery},
    schema::{IndexRecordOption, Value},
};
use time::{Date, macros::format_description};
use zstd::bulk::Decompressor;

/*
  Import own libraries
*/
use crate::web::functions::{
    decentralized::{FedQuery, IROH_ENDPOINT, PROFILE_CACHE},
    general::{favicon_url, get_domain},
    ranking::{self, goggles::GoggleRules, rrf::RRFParams},
};
use prieco_core::{
    ANALYTICS, META_DECODER, PRIECO_INDEX, PriecoIndex, QueryIntent, Stage,
    globals::{
        EmbeddingService, PAGERANK, RERANKER, SearchResult, VECTOR_CENTROPOIDS, WebDocument, colors,
    },
    url_to_domain_id, url_to_id,
};

/*
  Constants
*/
/// Max full text results.
const MAX_FTS: usize = 160;
/// Max vector results.
const MAX_IVF: usize = 20;
/// IVF buckets searched per query.
const NPROBS: usize = 4;
/// Top results that get PageRank.
const PAGERANK_CUTOFF: usize = 100;
/// Top results that go through reranker.
const RERANK_CUTOFF: usize = 50;
/// Characters of a passage given to the reranker, more than its 512 tokens hold.
const RERANK_CHARS: usize = 2_500;
/// Reranked score is this + reranker logit, so it's always above RRF scores, which are under 1.
const RERANK_TIER: f32 = 1_000.0;
/// Max results from one domain.
const MAX_PER_DOMAIN: usize = 5;

/*
  Structures
*/
/// 'query_lang_loc...' → ranked results, only for [PRIECO_INDEX].
pub static QUERY_CACHE: Lazy<RwLock<HashMap<String, Vec<WebDocument>>>> =
    Lazy::new(|| RwLock::new(HashMap::with_capacity(1_000)));

/// Input of [run].
pub struct SearchDBParams<'r> {
    /// Where results are pushed.
    pub results: &'r mut Vec<SearchResult>,
    /// Search query.
    pub query: &'r str,
    /// Prefered language.
    pub lang: &'r str,
    /// Prefered location.
    pub loc: &'r str,
    /// Query embedder.
    pub embedding_service: &'r State<EmbeddingService>,
    /// Filters.
    pub goggles: Vec<Arc<GoggleRules>>,
    /// Is user using mobile.
    pub mobile: bool,
    /// Ask peers too.
    pub decentralize: bool,
    /// Hide NSFW results.
    pub safe_search: bool,
    /// Query intent, from [query_intent].
    pub intent: QueryIntent,
    /// How many results to return.
    pub limit: usize,
}

/// # Calls [run_json]() and creates [SearchResult]
///
/// This function calls [run_json]() for JSON results.
/// Generates result info, the 3 dots next to each result.
/// Formats JSON as a [SearchResult] object and pushes them to `results`.
///
/// # Arguments
///
/// * `params` - [SearchDBParams].
///
/// # Returns
///
/// None
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn run(params: SearchDBParams<'_>) {
    let local_results = run_json(
        params.query,
        params.lang,
        params.loc,
        params.embedding_service,
        params.goggles,
        params.mobile,
        params.decentralize,
        params.safe_search,
        PRIECO_INDEX.clone(),
        Some(params.intent),
        params.limit,
        None,
    )
    .await;

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

            params.results.push(SearchResult {
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
                favicon: favicon_url(&url),
                html_id,

                reading_level,
                formatted_load: formatted_load.to_string(),
                source_engine,
                content: content.to_string(),
                intent,
                sitelinks: Vec::new(),
            });
        }
    }
}

/// Intent of query, same as [run_json] gets it.
pub fn query_intent(query: &str, lang: &str, loc: &str) -> QueryIntent {
    let mut fts_query = normalize_search_operators(query);
    ranking::meaning::call::process_query(&mut fts_query, lang, loc).0
}

/// Ranking of every pipeline stage: (stage name, urls best first).
pub type StageTrace = Vec<(&'static str, Vec<String>)>;

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
///     * Pings a lot of domains matching query. And waits (max 1s) for 20 response code.
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
/// * Results in user language first.
///
/// * Reranker on the top [RERANK_CUTOFF], score becomes [RERANK_TIER] + reranker logit.
///
/// * Hand ranking, bounded quality factors and goggles, reranked always stay on top.
///     * Reranked: [ranking::hand::run_logits] adds logarithm of the boost to the logit.
///     * Others: [ranking::hand::run] multiplies the RRF score by the boost.
///
/// * PageRank, added as logarithm for reranked results, multiplied for others.
///
/// * Results in user language first again, homepage of the queried domain first for navigational queries.
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
/// * `decentralize` - Ask peers too.
/// * `index` - Index to search, [PRIECO_INDEX] or a benchmark index.
/// * `trace` - Ranking after every stage for the benchmark, `None` records nothing.
///     * Stages: 'tantivy', 'ivf', 'rrf', 'hand', 'pagerank', 'rerank'.
///
/// Any other index than [PRIECO_INDEX] skips [QUERY_CACHE], peers and discovery,
/// so benchmark results never reach users, peers or the official index.
///
/// # Returns
///
/// Vector of JSONs that are made form [WebDocument] objects.
///
/// # Panics
///
/// Only if system runs out of memory.
#[allow(clippy::too_many_arguments)]
pub async fn run_json(
    query: &str,
    lang: &str,
    loc: &str,
    embedding_manager: &EmbeddingService,
    goggles: Vec<Arc<GoggleRules>>,
    mobile: bool,
    decentralize: bool,
    safe_search: bool,
    index: Arc<PriecoIndex>,
    intent: Option<QueryIntent>,
    limit: usize,
    mut trace: Option<&mut StageTrace>,
) -> Vec<Json_Value> {
    let official_index = Arc::ptr_eq(&index, &PRIECO_INDEX);
    let decentralize = decentralize && official_index;

    let cache_key = format!("{}_{}_{}_{}", query, lang, loc, safe_search);
    let cached_data = if official_index {
        let cache = QUERY_CACHE.read();
        cache.get(&cache_key).cloned()
    } else {
        None
    };

    let intent = intent.unwrap_or_else(|| query_intent(query, lang, loc));

    let mut results = if let Some(cached) = cached_data {
        cached
    } else {
        let contextual_query = if intent == QueryIntent::Local
            || (query.split_whitespace().count() <= 2 && !loc.is_empty())
        {
            format!("{} {}", query, loc)
        } else {
            query.to_string()
        };

        let embed: Vec<f32> = embedding_manager
            .embed_query(&contextual_query)
            .await
            .unwrap_or_default();

        let q_clone = query.to_string();
        let q_clone2 = query.to_string();
        let l_clone = lang.to_string();
        let lang_clone = lang.to_string();
        let loc_clone = loc.to_string();
        let loc_clone2 = loc.to_string();
        let int_clone = intent;
        let emb_clone = embed.clone();
        let g_clone = goggles.clone();

        let index_clone = index.clone();

        let local_task = tokio::spawn(async move {
            run_core_search(
                &q_clone,
                &l_clone,
                &loc_clone,
                &int_clone,
                emb_clone,
                official_index,
                g_clone,
                index_clone,
            )
            .await
        });

        let experts_task = tokio::spawn(async move {
            if !decentralize {
                return Vec::new();
            }

            let search_future = async {
                let t = Instant::now();

                let mut decentralized_results = Vec::new();
                let query_words: Vec<u64> = q_clone2
                    .split_whitespace()
                    .map(|w| url_to_id(&w.to_lowercase()))
                    .collect();

                let mut scored_nodes: Vec<(PublicKey, usize)> = {
                    let cache = PROFILE_CACHE.read();
                    cache
                        .iter()
                        .map(|(pub_key, profile)| {
                            let mut score = 0;
                            for w in &query_words {
                                if profile.keyword.contains(*w) {
                                    score += 10;
                                }
                            }
                            (*pub_key, score)
                        })
                        .collect()
                };

                scored_nodes.sort_unstable_by_key(|&(_, score)| Reverse(score));
                let best_nodes: Vec<PublicKey> =
                    scored_nodes.into_iter().take(4).map(|(pk, _)| pk).collect();

                let fed_query = FedQuery {
                    query: q_clone2,
                    lang: lang_clone,
                    loc: loc_clone2,
                    depth: 1,
                    embed: embed.clone(),
                };

                ANALYTICS.count("p2p_sent");

                if let Ok(payload) = serde_json::to_vec(&fed_query)
                    && let Some(endpoint) = IROH_ENDPOINT.get()
                {
                    let mut handles = Vec::new();
                    for pub_key in best_nodes {
                        let ep = endpoint.clone();
                        let pl = payload.clone();

                        handles.push(tokio::spawn(async move {
                            let node_addr = EndpointAddr::from(pub_key);

                            let fetch_future = async {
                                let conn = ep.connect(node_addr, b"prieco-search/0").await.ok()?;
                                let (mut send, mut recv) = conn.open_bi().await.ok()?;
                                send.write_all(&pl).await.ok()?;
                                send.finish().ok()?;
                                let data = recv.read_to_end(5 * 1024 * 1024).await.ok()?;
                                serde_json::from_slice::<Vec<WebDocument>>(&data).ok()
                            };

                            match tokio::time::timeout(Duration::from_secs(9), fetch_future).await {
                                Ok(Some(res)) => Some(res),
                                _ => {
                                    let mut cache = PROFILE_CACHE.write();
                                    cache.remove(&pub_key);
                                    None
                                }
                            }
                        }));
                    }
                    for handle in handles {
                        if let Ok(Some(res)) = handle.await {
                            decentralized_results.extend(res);
                        }
                    }
                }
                decentralized_results
                    .sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

                ANALYTICS.count("p2p_sent_ok");
                ANALYTICS.record_latency(Stage::PeerRtt, t.elapsed());

                decentralized_results
            };

            match tokio::time::timeout(Duration::from_secs(10), search_future).await {
                Ok(results) => results,
                Err(_) => {
                    println!("Decentralized search timed out!");
                    Vec::new()
                }
            }
        });

        let (local_res, dec_res) = tokio::join!(local_task, experts_task);

        let (dir_results, tantivy_results, vector_results, discovery_results) = local_res.unwrap();
        let decentralized_results = dec_res.unwrap();

        if let Some(stages) = trace.as_mut() {
            stages.push((
                "tantivy",
                tantivy_results.iter().map(|doc| doc.url.clone()).collect(),
            ));
            stages.push((
                "ivf",
                vector_results.iter().map(|doc| doc.url.clone()).collect(),
            ));
        }

        // Local save
        if !decentralized_results.is_empty() {
            let docs_to_ingest = decentralized_results.clone();
            tokio::spawn(async move {
                ingest_peer_documents(docs_to_ingest).await;
            });
        }

        let mut rrf_results: Vec<WebDocument> = ranking::rrf::run(RRFParams {
            query,
            lang,
            intent: &intent,
            dir_results,
            fts_results: tantivy_results,
            ivf_results: vector_results,
            dis_results: discovery_results,
            dec_results: decentralized_results,
            k: 10.0,
        });

        if let Some(stages) = trace.as_mut() {
            stages.push((
                "rrf",
                rrf_results.iter().map(|doc| doc.url.clone()).collect(),
            ));
        }

        // Temp remove blocked terms
        if safe_search {
            let blocked_prefixes: &[&str] = &["https://mail.proton.me/u/"];
            rrf_results.retain(|doc| {
                !blocked_prefixes
                    .iter()
                    .any(|prefix| doc.url.starts_with(prefix))
            });

            let blocked_terms: &[&str] = &["porn", "sex", "dick", "fap"];
            rrf_results.retain(|doc| {
                if doc.safe_s {
                    return false;
                }
                let haystack = format!(
                    "{} {} {} {} {}",
                    doc.title.to_lowercase(),
                    doc.description.to_lowercase(),
                    doc.url.to_lowercase(),
                    doc.content.to_lowercase(),
                    doc.keywords.to_lowercase()
                );
                !blocked_terms.iter().any(|term| haystack.contains(term))
            });
        }

        if official_index {
            let mut cache = QUERY_CACHE.write();
            if cache.len() > 5_000 {
                cache.clear();
            }
            cache.insert(cache_key, rrf_results.clone());
        }

        rrf_results
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
        return Vec::new();
    }

    // User language
    let other_lang = |doc: &WebDocument| {
        lang != "all" && !lang.is_empty() && !doc.lang.is_empty() && doc.lang != lang
    };
    results.sort_by_key(other_lang);

    // RERANKER
    let nn_candidates = results.len().min(RERANK_CUTOFF);
    if nn_candidates > 0 {
        let rerank_start = Instant::now();
        let passages: Vec<String> = results[..nn_candidates]
            .iter()
            .map(|d| {
                let passage = if d.content.starts_with(&d.description) {
                    format!("{} {}", d.title, d.content)
                } else {
                    format!("{} {} {}", d.title, d.description, d.content)
                };
                passage.chars().take(RERANK_CHARS).collect()
            })
            .collect();
        let scores = RERANKER
            .score_batch(query, &passages)
            .await
            .unwrap_or_else(|e| {
                println!("{}Reranker failed: {}{}", colors::RED, e, colors::RESET);
                vec![0.0; nn_candidates]
            });
        for (doc, logit) in results[..nn_candidates].iter_mut().zip(scores) {
            doc.search_score = RERANK_TIER + logit;
        }
        results[..nn_candidates]
            .sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

        if official_index {
            ANALYTICS.record_latency(Stage::Rerank, rerank_start.elapsed());
        }
    }

    if let Some(stages) = trace.as_mut() {
        stages.push((
            "rerank",
            results.iter().map(|doc| doc.url.clone()).collect(),
        ));
    }

    let (reranked, rest) = results.split_at_mut(nn_candidates);
    ranking::hand::run_logits(reranked, query, lang, loc, &intent, &goggles, mobile);
    ranking::hand::run(rest, query, lang, loc, &intent, &goggles, mobile);

    if let Some(stages) = trace.as_mut() {
        stages.push(("hand", results.iter().map(|doc| doc.url.clone()).collect()));
    }

    // PageRank
    let pr_candidates = results.len().min(PAGERANK_CUTOFF);
    if pr_candidates > 0 {
        let pr_guard = PAGERANK.read();
        for (rank, doc) in results[..pr_candidates].iter_mut().enumerate() {
            let pr_score = if let Some(pr) = pr_guard.as_ref() {
                pr.get_score(&doc.url)
            } else {
                0.0
            };
            if rank < nn_candidates {
                doc.search_score += (1.0 + pr_score).ln();
            } else {
                doc.search_score *= 1.0 + pr_score;
            }
        }
        drop(pr_guard);
        results[..pr_candidates]
            .sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    }

    if let Some(stages) = trace.as_mut() {
        stages.push((
            "pagerank",
            results.iter().map(|doc| doc.url.clone()).collect(),
        ));
    }

    results.sort_by_key(other_lang);

    if intent == QueryIntent::Navigational
        && let Some(pos) = results
            .iter()
            .position(|doc| ranking::hand::navigational_match(&doc.url, query))
    {
        let homepage = results.remove(pos);
        results.insert(0, homepage);
    }

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

    let shown_results: Vec<WebDocument> = results.iter().take(limit).cloned().collect();
    shown_results
        .into_iter()
        .filter_map(|s| serde_json::to_value(s).ok())
        .collect()
}
/// # Runs all index searches at the same time
///
/// Used by [run_json] and by [peers][crate::web::functions::decentralized::SearchProtocol].
///
/// # Arguments
///
/// * `query` - Search query.
/// * `lang` - Prefered language.
/// * `loc` - Prefered location.
/// * `intent` - [QueryIntent].
/// * `embed` - Query vector.
/// * `_run_discovery` - Discovery search, currently off.
/// * `goggles` - Filters.
/// * `index` - Index to search.
///
/// # Returns
///
/// (direct, full text, vector, discovery) results, each sorted.
///
/// # Panics
///
/// Only if system runs out of memory.
#[allow(clippy::too_many_arguments)]
pub async fn run_core_search(
    query: &str,
    lang: &str,
    loc: &str,
    intent: &QueryIntent,
    embed: Vec<f32>,
    _run_discovery: bool,
    goggles: Vec<Arc<GoggleRules>>,
    index: Arc<PriecoIndex>,
) -> (
    Vec<WebDocument>,
    Vec<WebDocument>,
    Vec<WebDocument>,
    Vec<WebDocument>,
) {
    let total_start = std::time::Instant::now();
    let record = Arc::ptr_eq(&index, &PRIECO_INDEX);
    let q_clone = query.to_string();
    let q_clone3 = q_clone.clone();
    let _q_clone4 = q_clone.clone();

    let fts_query = normalize_search_operators(&q_clone);
    let fts_original_query = fts_query.clone();

    let lang_clone = lang.to_string();
    let loc_clone = loc.to_string();

    let dir_task = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let trimmed = sanitize_string(q_clone.trim()).to_lowercase();
        let mut id_score_vec: Vec<(u64, f32)> = Vec::new();

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
                id_score_vec.push((url_to_id(&format!("{}{}/", prefix, domain)), 0.0));
            }
            println!("DIR lookup took {:.3}s", start.elapsed().as_secs_f32());
            if record {
                ANALYTICS.record_latency(Stage::Dir, start.elapsed());
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
                id_score_vec.push((url_to_id(&format!("{}{}{}/", prefix, trimmed, tld)), 0.0));
            }
        }

        println!("DIR lookup took {:.3}s", start.elapsed().as_secs_f32());
        if record {
            ANALYTICS.record_latency(Stage::Dir, start.elapsed());
        }

        id_score_vec
    });

    let intent_clone = *intent;
    let goggles_clone = goggles.clone();
    let index_clone = index.clone();
    let tantivy_task = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        if (matches!(lang_clone.as_str(), "zh" | "ja" | "ko" | "th")
            && fts_original_query.chars().count() >= 15)
        //|| fts_original_query.split_whitespace().count() >= 8
        {
            return Vec::new();
        }

        let res = search_tantivy(
            &index_clone,
            &fts_query,
            &lang_clone,
            &loc_clone,
            &intent_clone,
            MAX_FTS,
            &goggles_clone,
        )
        .unwrap_or_default();

        println!(
            "Tantivy took {:.3}s with {} segments",
            start.elapsed().as_secs_f32(),
            index_clone.reader.searcher().segment_readers().len()
        );
        if record {
            ANALYTICS.record_latency(Stage::Fts, start.elapsed());
        }

        res
    });

    let vector_index = index.clone();
    let vector_task = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        if q_clone3.contains('"')
            || q_clone3.contains(':')
            || q_clone3.contains('-')
            || q_clone3.contains('|')
            || q_clone3.contains(" OR ")
        {
            return Vec::new();
        }
        let res: Vec<(u64, f32)> = VECTOR_CENTROPOIDS
            .search(&embed, 0, NPROBS, &vector_index.vector_path)
            .unwrap_or_default();

        let final_res: Vec<(u64, f32)> = res.into_iter().take(MAX_IVF).collect();
        println!("Vector search took {:.3}s", start.elapsed().as_secs_f32());
        if record {
            ANALYTICS.record_latency(Stage::Vector, start.elapsed());
        }

        final_res
    });

    let discovery_task = tokio::spawn(async move {
        /*if run_discovery {
            discover_and_ping_domains(&q_clone4).await
        } else {
            Vec::new()
        }*/
        Vec::new()
    });

    let (tantivy_ids, vector_ids, dir_ids, discovery_results) =
        tokio::join!(tantivy_task, vector_task, dir_task, discovery_task);

    println!(
        "Total concurrent query time {:.3}s",
        total_start.elapsed().as_secs_f32()
    );

    let mut engines_map: HashMap<&'static str, Vec<(u64, f32)>> = HashMap::new();
    engines_map.insert("FTS", tantivy_ids.unwrap_or_default());
    engines_map.insert("IVF", vector_ids.unwrap_or_default());
    engines_map.insert("DIR", dir_ids.unwrap_or_default());

    let fetch_start = std::time::Instant::now();
    let mut fetched_results = fetch_documents(&index, engines_map);
    println!("Fetch took {:.3}s", fetch_start.elapsed().as_secs_f32());
    if record {
        ANALYTICS.record_latency(Stage::Fetch, fetch_start.elapsed());
    }

    let mut dir_results: Vec<WebDocument> = fetched_results.remove("DIR").unwrap_or_default();
    let mut tantivy_results: Vec<WebDocument> = fetched_results.remove("FTS").unwrap_or_default();
    let mut vector_results: Vec<WebDocument> = fetched_results.remove("IVF").unwrap_or_default();
    let mut dis_results: Vec<WebDocument> = discovery_results.unwrap_or_default();

    dir_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    tantivy_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    vector_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());
    dis_results.sort_by(|a, b| b.search_score.partial_cmp(&a.search_score).unwrap());

    println!("DIR: {}", dir_results.len());
    println!("Tantivy: {}", tantivy_results.len());
    println!("IVF: {}", vector_results.len());
    println!("DIS: {}", dis_results.len());

    if record {
        ANALYTICS.record_latency(Stage::Core, total_start.elapsed());
    }

    (dir_results, tantivy_results, vector_results, dis_results)
}

/* Index search functions */

/// `site:` operator.
static SITE_EXTRACT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(site|domain):([^\s\(\)]+)").unwrap());

/// # Performs FTS index search.
///
/// This function prepares a Tantivy call and performs it.
/// For curation uses [QueryIntent], prefered lang and goggles (discard filter).
///
/// # Arguments
///
/// * `index` - Index to search.
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
    index: &PriecoIndex,
    query_text: &str,
    _lang: &str,
    loc: &str,
    intent: &QueryIntent,
    limit: usize,
    goggles: &[Arc<GoggleRules>],
) -> Option<Vec<(u64, f32)>> {
    let schema = index.tantivy.schema();
    let doc_id = schema.get_field("doc_id").ok()?;
    let domain_id_field = schema.get_field("domain_id").ok()?;
    let _lang_field = schema.get_field("lang").ok()?;
    let loc_field = schema.get_field("loc").ok()?;
    let _intent_field = schema.get_field("intent").ok()?;

    let mut clean_query_text = query_text.to_string();
    let mut site_hashes = Vec::new();

    for caps in SITE_EXTRACT_RE.captures_iter(&clean_query_text.clone()) {
        let mut raw_domain = caps[2].to_string();
        raw_domain = raw_domain.replace("https://", "").replace("http://", "");
        raw_domain = raw_domain
            .strip_prefix("www.")
            .unwrap_or(&raw_domain)
            .trim_end_matches('/')
            .to_string();

        for prefix in ["", "www.", "en.", "m."] {
            let url = format!("http://{}{}", prefix, raw_domain);
            let hash = url_to_domain_id(&url);
            if hash != 0 {
                site_hashes.push(hash);
            }
        }
    }

    clean_query_text = SITE_EXTRACT_RE
        .replace_all(&clean_query_text, "")
        .to_string();

    let (parsed_query, _) = index.query_parser.parse_query_lenient(&clean_query_text);

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, parsed_query)];

    if !site_hashes.is_empty() {
        let terms: Vec<Term> = site_hashes
            .into_iter()
            .map(|id| Term::from_field_u64(domain_id_field, id))
            .collect();
        clauses.push((Occur::Must, Box::new(TermSetQuery::new(terms))));
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

    let searcher = index.reader.searcher();
    let top_docs = searcher
        .search(&final_query, &TopDocs::with_limit(limit).order_by_score())
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

        if let Some(doc_id_value) = retrieved_doc.get_first(doc_id)
            && let Some(id) = doc_id_value.as_u64()
        {
            id_score_vec.push((id, score));
        }
    }

    Some(id_score_vec)
}

/* Helper functions*/

/// # Fetches documents from meta storage.
///
/// This funtion takes in batch all IDs needed for fetching.
/// Deduplicates IDs.
/// Sorts them for slightly better sequencial locality.
/// Spins up multithreated disk seeks.
/// [META_DICTIONARY] decompresses data.
///
/// # Arguments
///
/// * `index` - Index whose meta storage is read.
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
    index: &PriecoIndex,
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

    let chunk_size = ids_to_fetch
        .len()
        .div_ceil(current_num_threads() * 3)
        .max(1);
    let fetched_docs: HashMap<u64, WebDocument> = ids_to_fetch
        .par_chunks(chunk_size)
        .map_init(
            || match &*META_DECODER {
                Some(decoder) => Decompressor::with_prepared_dictionary(decoder).unwrap(),
                None => Decompressor::new().unwrap(),
            },
            |decompressor, chunk| {
                let mut local_docs = Vec::with_capacity(chunk.len());

                for &id in chunk {
                    let key = id.to_be_bytes();

                    // Get data
                    let compressed_data = match index.meta_ks.get(key) {
                        Ok(Some(data)) => data,
                        _ => {
                            continue;
                        }
                    };

                    // Decompress
                    let capacity =
                        zstd::zstd_safe::get_frame_content_size(compressed_data.as_ref())
                            .ok()
                            .flatten()
                            .map_or(5 * 1024 * 1024, |size| size as usize)
                            .min(64 * 1024 * 1024);
                    let decompressed_buf =
                        match decompressor.decompress(compressed_data.as_ref(), capacity) {
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

/// Removes `"` and `'` from text.
fn sanitize_string(s: &str) -> String {
    s.replace(['"', '\''], "")
}

/// Checks if URL matches a goggle path pattern.
///
/// * `^/path$` - Exact.
/// * `^/path` - Starts with.
/// * `/path$` - Ends with.
/// * `/path` - Contains.
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

/// `before:` and `after:` with YYYY-MM-DD.
static DATE_OPERATOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(before|after):(\d{4}-\d{2}-\d{2})").unwrap());

/// Turns user operators into Tantivy syntax.
///
/// * `before:2026-01-02` → `date:{* TO 260102}`, `after:` the other way.
/// * `intitle:` → `title:`.
/// * ` | ` → ` OR `.
fn normalize_search_operators(query: &str) -> String {
    let mut normalized = query.to_string();

    normalized = DATE_OPERATOR_RE
        .replace_all(&normalized, |caps: &regex::Captures| {
            let operator = caps[1].to_lowercase();
            let date_str = &caps[2];

            if let Ok(date) = Date::parse(
                date_str,
                format_description!("[year]-[month padding:none]-[day padding:none]"),
            ) {
                let formatted_date = date
                    .format(format_description!("[year repr:last_two][month][day]"))
                    .unwrap_or_default();
                if let Ok(date_int) = formatted_date.parse::<i64>() {
                    match operator.as_str() {
                        "before" => format!("date:{{* TO {}}}", date_int),
                        "after" => format!("date:{{{} TO *}}", date_int),
                        _ => caps[0].to_string(),
                    }
                } else {
                    caps[0].to_string()
                }
            } else {
                caps[0].to_string()
            }
        })
        .to_string();

    normalized = normalized.replace("intitle:", "title:");
    normalized = normalized.replace(" | ", " OR ");

    normalized
}
