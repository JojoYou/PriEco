//! # Intent
//!
//! Heuristic query intent from keywords in 'intent.json'.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-07-07
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use prieco_core::{MATCHERS, QueryIntent};

/// Classifies query intent.
///
/// First whole word keyword of the language wins, unsupported languages use English.
/// Without keyword: 5+ words Informational, 1-2 words Navigational, otherwise Unknown.
///
/// # Arguments
///
/// * `query` - Query.
/// * `lang` - Language code.
///
/// # Returns
///
/// Intent.
pub fn intent(query: &str, lang: &str) -> QueryIntent {
    let lower_query = query.to_lowercase();
    let byte_lower = lower_query.as_bytes();

    let target_lang = if MATCHERS.contains_key(lang) {
        lang
    } else {
        "en"
    };

    let whole_word = |m: &aho_corasick::Match| {
        (m.start() == 0 || !byte_lower[m.start() - 1].is_ascii_alphanumeric())
            && (m.end() == byte_lower.len() || !byte_lower[m.end()].is_ascii_alphanumeric())
    };

    if let Some(matcher) = MATCHERS.get(target_lang)
        && let Some(mat) = matcher.automaton.find_iter(&lower_query).find(whole_word)
    {
        return matcher.intent_map[mat.pattern().as_usize()];
    }

    let token_count = lower_query.split_whitespace().count();
    if token_count >= 5 {
        QueryIntent::Informational
    } else if token_count <= 2 && token_count > 0 {
        QueryIntent::Navigational
    } else {
        QueryIntent::Unknown
    }
}
