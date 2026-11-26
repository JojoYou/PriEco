//! # Reciprocal Rank Fusion
//!
//! Merges sorted results of all searches into one list.
//!
//! ## Architecture
//!
//! 1. Every result gets weight / (k + rank + 1), scores of the same URL are added.
//!     * Weights: direct 3.0, discovery 0.5, other nodes 0.9, full text and vector by [adjust_weights].
//!     * Full text and vector results in a different language: ×0.7 for Navigational, otherwise ×0.1.
//!     * Page intent same as query intent: ×1.3.
//! 2. `site:` or `domain:` in query keeps only that domain and its subdomains.
//! 3. HTTP results with an HTTPS twin are removed.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-03-05
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use std::collections::{HashMap, HashSet};

use crate::web::functions::general::get_domain;
use prieco_core::{QueryIntent, WebDocument};

/// Input of [run]. Every result list is sorted, best first.
pub struct RRFParams<'r> {
    /// User query.
    pub query: &'r str,
    /// User language.
    pub lang: &'r str,
    /// Query intent.
    pub intent: &'r QueryIntent,
    /// Direct search.
    pub dir_results: Vec<WebDocument>,
    /// Full text search.
    pub fts_results: Vec<WebDocument>,
    /// Vector search.
    pub ivf_results: Vec<WebDocument>,
    /// Discovery search.
    pub dis_results: Vec<WebDocument>,
    /// Results from other nodes.
    pub dec_results: Vec<WebDocument>,
    /// RRF constant, higher makes rank matter less.
    pub k: f32,
}
/// Merges results.
///
/// `search_score` is the RRF score, `source` lists searches that found it, e.g. 'FTS + IVF'.
///
/// # Returns
///
/// Merged results, best first.
pub fn run(params: RRFParams<'_>) -> Vec<WebDocument> {
    // Custom searching WEIGHTs
    /// Direct search weight.
    const DIR_WEIGHT: f32 = 3.0;
    /// Discovery search weight.
    const DIS_WEIGHT: f32 = 0.5;
    /// Other nodes weight.
    const FED_WEIGHT: f32 = 0.9;

    let (fts_weight, ivf_weight) = adjust_weights(params.query);

    let mut scores: HashMap<String, (f32, WebDocument)> = HashMap::with_capacity(
        params.dir_results.len()
            + params.fts_results.len()
            + params.ivf_results.len()
            + params.dis_results.len()
            + params.dec_results.len(),
    );

    let sources = [
        (params.dir_results, DIR_WEIGHT, "DIR"),
        (params.fts_results, fts_weight, "FTS"),
        (params.ivf_results, ivf_weight, "IVF"),
        (params.dis_results, DIS_WEIGHT, "DIS"),
        (params.dec_results, FED_WEIGHT, "DEC"),
    ];

    for (results, weight, source_name) in sources {
        for (rank, mut doc) in results.into_iter().enumerate() {
            let mut rrf_score = weight * (1.0 / (params.k + rank as f32 + 1.0));

            if params.lang != "all"
                && !params.lang.is_empty()
                && !doc.lang.is_empty()
                && doc.lang != params.lang
                && (source_name == "IVF" || source_name == "FTS")
            {
                if params.intent == &QueryIntent::Navigational {
                    rrf_score *= 0.7;
                } else {
                    rrf_score *= 0.1;
                }
            }

            let inte = match params.intent {
                QueryIntent::Informational => 0,
                QueryIntent::Transactional => 1,
                QueryIntent::CommercialInvestigation => 2,
                QueryIntent::Navigational => 3,
                QueryIntent::Local => 4,
                QueryIntent::Unknown => 5,
            };

            if params.intent != &QueryIntent::Unknown && doc.intent == inte {
                rrf_score *= 1.3;
            }

            scores
                .entry(doc.url.clone())
                .and_modify(|(score, existing_doc)| {
                    *score += rrf_score;

                    if !existing_doc.source.contains(source_name) {
                        existing_doc.source = format!("{} + {}", existing_doc.source, source_name);
                    }
                })
                .or_insert_with(|| {
                    doc.source = source_name.to_string();
                    (rrf_score, doc)
                });
        }
    }

    let mut merged: Vec<(f32, WebDocument)> = scores
        .into_values()
        .map(|(score, mut doc)| {
            doc.search_score = score;
            (score, doc)
        })
        .collect();

    merged.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut result: Vec<WebDocument> = merged.into_iter().map(|(_, doc)| doc).collect();

    /// `site:` and `domain:` operators.
    static RRF_DOMAIN_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"(?i)(site|domain):([^\s\(\)]+)").unwrap()
    });

    let mut target_domains = Vec::new();
    for caps in RRF_DOMAIN_RE.captures_iter(params.query) {
        let mut target = caps[2].to_lowercase();
        target = target.replace("https://", "").replace("http://", "");
        target = target
            .strip_prefix("www.")
            .unwrap_or(&target)
            .trim_end_matches('/')
            .to_string();
        target_domains.push(target);
    }

    if !target_domains.is_empty() {
        result.retain(|doc| {
            let doc_domain = get_domain(&doc.url, true);
            target_domains.iter().any(|target| {
                doc_domain == *target || doc_domain.ends_with(&format!(".{}", target))
            })
        });
    }

    // Remove HTTP duplicates of HTTPS URLs
    let https_urls: HashSet<String> = result
        .iter()
        .filter(|s| s.url.starts_with("https://"))
        .map(|s| s.url.trim_end_matches('/').to_string())
        .collect();

    result.retain(|s| {
        if s.url.starts_with("http://") {
            let https_version = s.url.replacen("http://", "https://", 1);
            !https_urls.contains(https_version.trim_end_matches('/'))
        } else {
            true
        }
    });

    result
}

/// Full text and vector weights from query shape. Both start at 1.0, max 2.0.
///
/// * 1-2 words: full text +0.7. 6+ words: vector +0.7.
/// * Quotes: full text +0.8.
/// * Question word: vector +0.6.
/// * Over 30% words longer than 7 characters: full text +0.4.
///
/// # Returns
///
/// (full text weight, vector weight).
fn adjust_weights(query: &str) -> (f32, f32) {
    /// Words that make a query a question.
    const QUESTION_WORDS: [&str; 21] = [
        "what",
        "why",
        "how",
        "when",
        "where",
        "who",
        "which",
        "explain",
        "meaning",
        "is",
        "are",
        "does",
        "can",
        "should",
        "will",
        "would",
        "could",
        "difference",
        "between",
        "versus",
        "vs",
    ];

    //query analysis
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let len = tokens.len() as f32;
    let has_quotes = query.contains('"');

    let has_question = tokens
        .iter()
        .any(|t| QUESTION_WORDS.contains(&t.to_lowercase().as_str()));

    // rare term heuristic = long tokens
    let rare_ratio = tokens.iter().filter(|t| t.len() > 7).count() as f32 / len.max(1.0);

    //base weights
    let mut fts_weight: f32 = 1.0;
    let mut ivf_weight: f32 = 1.0;

    //heuristics

    // query length
    if len <= 2.0 {
        fts_weight += 0.7;
    } else if len >= 6.0 {
        ivf_weight += 0.7;
    }

    // quotes → lexical intent
    if has_quotes {
        fts_weight += 0.8;
    }

    // questions → semantic intent
    if has_question {
        ivf_weight += 0.6;
    }

    // rare tokens → exact match matters
    if rare_ratio > 0.3 {
        fts_weight += 0.4;
    }

    // clamp to max 2.0
    fts_weight = fts_weight.min(2.0);
    ivf_weight = ivf_weight.min(2.0);

    (fts_weight, ivf_weight)
}
