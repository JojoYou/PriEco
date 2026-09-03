// reciprocal_rank_fusion
use std::collections::{HashMap, HashSet};

use crate::web::functions::general::get_domain;
use prieco_core::{QueryIntent, WebDocument};

pub fn run(
    query: &str,
    lang: &str,
    intent: &QueryIntent,
    dir_results: Vec<WebDocument>,
    fts_results: Vec<WebDocument>,
    ivf_results: Vec<WebDocument>,
    dis_results: Vec<WebDocument>,
    k: f32,
) -> Vec<WebDocument> {
    // Custom searching WEIGHTs
    const DIR_WEIGHT: f32 = 3.0;
    const DIS_WEIGHT: f32 = 0.5;

    let (fts_weight, ivf_weight) = adjust_weights(query);

    let mut scores: HashMap<String, (f32, WebDocument)> = HashMap::with_capacity(
        dir_results.len() + fts_results.len() + ivf_results.len() + dis_results.len(),
    );

    let sources = [
        (dir_results, DIR_WEIGHT, "DIR"),
        (fts_results, fts_weight, "FTS"),
        (ivf_results, ivf_weight, "IVF"),
        (dis_results, DIS_WEIGHT, "DIS"),
    ];

    for (results, weight, source_name) in sources {
        for (rank, mut doc) in results.into_iter().enumerate() {
            let mut rrf_score = weight * (1.0 / (k + rank as f32 + 1.0));

            // Enforce Vector index lang
            if lang != "all" && !lang.is_empty() && doc.lang != lang && source_name == "IVF" {
                if intent == &QueryIntent::Navigational {
                    rrf_score *= 0.7;
                } else {
                    rrf_score *= 0.1;
                }
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

    static RRF_DOMAIN_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"(?i)(site|domain):([^\s\(\)]+)").unwrap()
    });

    let mut target_domains = Vec::new();
    for caps in RRF_DOMAIN_RE.captures_iter(query) {
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

fn adjust_weights(query: &str) -> (f32, f32) {
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
