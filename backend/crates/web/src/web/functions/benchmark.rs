//! # Benchmark
//!
//! Runs the offline benchmarks of [prieco_benchmark] through the normal search pipeline.
//! Started by `make benchmark`, which sends SIGUSR1 to the running PriEco.
//!
//! ## Architecture
//!
//! 1. [**Prepare**:][msmarco::prepare] Download MS MARCO, build its own index once.
//! 2. [**Search**:][search_db::run_json] Every judged TREC DL query on that index.
//!     * No cache, peers or discovery, the official index is never touched.
//! 3. [**Run file**:][msmarco::write_run] TREC run file per query set for scoring.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-23
//! * Last Modified: 2026-11-23
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::time::Instant;

/*
  Import own libraries
*/
use crate::web::functions::search_db;
use prieco_benchmark::msmarco;
use prieco_core::{EmbeddingService, colors, globals::icons};

/*
  Constants
*/
const LANG: &str = "en";
const LOC: &str = "";

/// Runs every benchmark once.
///
/// # Arguments
///
/// * `embedding_service` - Query embedder, same as for users.
///
/// # Returns
///
/// None. Progress, errors and run file paths are printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn run(embedding_service: &EmbeddingService) {
    let start = Instant::now();
    println!("{}: Benchmark started!", icons::BENCHMARK);

    /* MS MARCO */
    let index = match msmarco::prepare().await {
        Some(index) => index,
        None => return,
    };
    let docids = msmarco::load_docids();

    for (name, queries_file, qrels_file) in msmarco::QUERY_SETS {
        let set_start = Instant::now();
        let queries = msmarco::load_queries(queries_file, qrels_file);

        // Search every query
        let mut results: Vec<(String, Vec<(String, f32)>)> = Vec::with_capacity(queries.len());
        for (qid, query) in queries {
            let docs = search_db::run_json(
                &query,
                LANG,
                LOC,
                embedding_service,
                Vec::new(),
                false,
                false,
                index.clone(),
            )
            .await;

            // Ranked (url, score)
            let ranked: Vec<(String, f32)> = docs
                .iter()
                .filter_map(|doc| {
                    let url = doc.get("url")?.as_str()?.to_string();
                    let score = doc.get("search_score")?.as_f64()? as f32;
                    Some((url, score))
                })
                .collect();

            results.push((qid, ranked));
        }

        msmarco::write_run(name, &results, &docids);
        println!(
            "{}: {} took {:.3}s for {} queries",
            icons::BENCHMARK,
            name,
            set_start.elapsed().as_secs_f64(),
            results.len(),
        );
    }

    println!(
        "{}{}: Benchmark finished in {:.3}s!{}",
        icons::BENCHMARK,
        colors::GREEN,
        start.elapsed().as_secs_f64(),
        colors::RESET,
    );
}
