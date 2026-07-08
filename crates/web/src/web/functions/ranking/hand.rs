use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::RwLock;

use prieco_core::{QueryIntent, WebDocument};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct RankingWeights {
    pub domain_match_boost: f64,
    pub homepage_boost: f64,
    pub lang_boost: f64,
    pub loc_boost: f64,
    pub tld_loc_boost: f64,
    pub wiki_boost: f64,
    pub https_boost: f64,
    pub dev_com_boost: f64,
    pub org_net_boost: f64,
    pub bad_url_penalty: f64,
    pub path_depth_penalty: f64,
    pub confidence_multi: f64,
    pub effort_multi: f64,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            domain_match_boost: 2.164824041987222,
            homepage_boost: 1.6227153067582822,
            lang_boost: 1.2422396900328991,
            loc_boost: 0.9794162380413227,
            tld_loc_boost: 1.2,
            wiki_boost: 1.3977276102163492,
            https_boost: 1.2874976244929701,
            dev_com_boost: 1.04,
            org_net_boost: 1.02,
            bad_url_penalty: 1.1601573817712056,
            path_depth_penalty: 1.096907068787565,
            confidence_multi: 0.01,
            effort_multi: 0.08,
        }
    }
}

static ACTIVE_WEIGHTS: Lazy<RwLock<RankingWeights>> =
    Lazy::new(|| RwLock::new(RankingWeights::default()));

pub fn run(
    results: &mut Vec<WebDocument>,
    query: &str,
    lang: &str,
    loc: &str,
    intent: &QueryIntent,
) {
    // Training
    check_for_updated_weights(); // PHP might have generated new weights to be tested
    let weights = {
        let w = ACTIVE_WEIGHTS.read().unwrap();
        *w
    };

    // Calculate custom ranking boost
    for doc in results.iter_mut() {
        let clean_url = strip_url_noise(&doc.url);

        let mut boost: f64 = 1.0; // Multiplicative base

        // (https://www/)query(.tld/)
        if *intent == QueryIntent::Navigational {
            // Extract the core domain name (e.g., "facebook" from "https://www.facebook.com/login")
            let domain_root = clean_url
                .split("://")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .and_then(|d| d.strip_prefix("www.").or(Some(d)))
                .and_then(|d| d.split('.').next())
                .unwrap_or("");

            if domain_root.eq_ignore_ascii_case(query) {
                boost *= weights.domain_match_boost;
            }

            // Homepage
            if is_effectively_homepage(clean_url) {
                boost *= weights.homepage_boost;
            }
        }

        // Shopping intent
        if *intent == QueryIntent::CommercialInvestigation {
            if clean_url.contains("/product/")
                || clean_url.contains("/item/")
                || clean_url.contains("review")
            {
                boost *= 1.2;
            }

            if clean_url.contains(".com")
                || clean_url.contains(".shop")
                || clean_url.contains(".store")
            {
                boost *= 1.1;
            }
        }

        // Informational: learning
        if *intent == QueryIntent::Informational {
            if clean_url.contains(".wikipedia.org") {
                boost *= 1.4;
            }

            if clean_url.contains(".org")
                || clean_url.contains(".edu")
                || clean_url.contains(".gov")
            {
                boost *= 1.25;
            }

            if clean_url.contains(".shop") || clean_url.contains("/product/") {
                boost *= 0.8;
            }
        }

        // Language
        if doc.lang == lang {
            boost *= weights.lang_boost;
        }

        // Location
        if *intent == QueryIntent::Local && doc.loc == loc {
            boost *= weights.loc_boost;
        }

        let tld_matches_loc = clean_url
            .split("://")
            .nth(1)
            .and_then(|s| s.trim_end_matches('/').split('.').last())
            .map_or(false, |tld| tld.eq_ignore_ascii_case(loc));
        if tld_matches_loc {
            boost *= weights.tld_loc_boost;
        }

        // Wikipedia authority signal
        if clean_url.contains(".wikipedia.org/wiki/") {
            boost *= weights.wiki_boost;
        }

        // SSL
        if clean_url.starts_with("https://") {
            boost *= weights.https_boost;
        }

        // TLD quality
        if clean_url.contains(".dev") || clean_url.contains(".com") {
            boost *= weights.dev_com_boost;
        } else if clean_url.contains(".org") || clean_url.contains(".net") {
            boost *= weights.org_net_boost;
        }

        // Load speed
        boost *= match doc.load {
            l if l < 0.3 => 1.08,
            l if l < 0.5 => 1.05,
            l if l < 2.0 => 1.02,
            l if l < 3.0 => 1.01,
            _ => 1.0,
        };

        // Bad URL patterns
        let url_body = clean_url.split("://").nth(1).unwrap_or(clean_url);
        if url_body.contains('_') || url_body.contains(",,") || url_body.contains(':') {
            boost *= weights.bad_url_penalty;
        }

        // Deep url
        let path_depth = clean_url.matches('/').count();
        if path_depth > 2 {
            let extra_slashes = (path_depth - 2) as i32;
            boost *= weights.path_depth_penalty.powi(extra_slashes);
        }

        boost *= 1.0 + sigmoid(doc.confidence as f64 / 200.0) * weights.confidence_multi;
        boost *= 1.0 + sigmoid(doc.effort as f64 / 200.0) * weights.effort_multi;

        doc.search_score = (doc.search_score as f64 * boost) as f32;
    }

    results.sort_by(|a, b| {
        b.search_score
            .partial_cmp(&a.search_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/* Training */
fn check_for_updated_weights() {
    let ready_path = Path::new("weights.ready");
    let json_path = Path::new("weights.json");

    if ready_path.exists() {
        // PHP generated new weights to be tested
        if let Ok(json_data) = fs::read_to_string(json_path) {
            if let Ok(new_weights) = serde_json::from_str::<RankingWeights>(&json_data) {
                if let Ok(mut current_weights) = ACTIVE_WEIGHTS.write() {
                    *current_weights = new_weights;
                    println!("⚡ PriEco swapped ranking weights.");
                }
            } else {
                eprintln!("❌ Failed to parse weights.json format.");
            }
        }
        let _ = fs::remove_file(ready_path); // Remove the trigger file to signal PHP to proceed
    }
}
/* Helper functions */
fn strip_url_noise(url: &str) -> &str {
    let url = url.split('?').next().unwrap_or(url);
    let url = url.split('#').next().unwrap_or(url);
    url
}
fn is_effectively_homepage(url: &str) -> bool {
    let path = url
        .split("://")
        .nth(1)
        .unwrap_or("")
        .trim_end_matches('/')
        .splitn(2, '/')
        .nth(1) // everything after the domain
        .unwrap_or("");

    if path.is_empty() {
        return true;
    }

    // Match locale-only paths: en, en-CA, fr-FR, zh-Hans, pt-BR etc.
    let is_locale = |seg: &str| -> bool {
        let seg = seg.trim_end_matches('/');
        matches!(seg.len(), 2..=7)
            && seg.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
            && seg.contains(|c: char| c.is_ascii_alphabetic()) // not just dashes
    };

    // Single locale segment: /en-CA/ or /en/
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    matches!(segments.as_slice(), [seg] if is_locale(seg))
}
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
