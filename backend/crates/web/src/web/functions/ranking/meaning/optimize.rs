//! # Synonyms and Optimization
//!
//! Rewrites query into a Tantivy query.
//!
//! ## Architecture
//!
//! 1. Queries with operators ('"', ':', '-', '|', OR, AND) or 'def' stay as they are.
//! 2. Stop words are removed.
//! 3. Every word: `"word"^2.0` OR 2 synonyms with `^1.0`. User's words are worth more.
//!     * Local words like 'near' get no synonyms.
//! 4. Entities: exact phrase in title, description or keywords. Business `^3.0`, place and name `^2.5`.
//! 5. More groups: (all groups required)^3.0 OR (any group).
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

use ahash::AHashSet;
use once_cell::sync::Lazy;
use prieco_core::{EntityType, LOCAL_NO_EXPAND, QueryIntent, TaggedEntity, get_store};

/// English stop words, removed from query.
static STOPWORDS: Lazy<AHashSet<&'static str>> = Lazy::new(|| {
    let mut set = AHashSet::new();
    let words = [
        "a", "an", "the", "in", "on", "of", "to", "for", "is", "are", "with", "and", "or", "what",
        "how", "why", "when", "at", "by",
    ];
    for w in words.iter() {
        set.insert(*w);
    }
    set
});

/// Adds synonyms and optimizes query for faster full text search.
///
/// # Arguments
///
/// * `query` - Query, replaced by the Tantivy query.
/// * `lang` - Language of synonyms.
/// * `intent` - Query intent.
/// * `tags` - Entities in query.
///
/// # Returns
///
/// None
pub fn synynyms_and_optimize(
    query: &mut String,
    lang: &str,
    intent: &QueryIntent,
    tags: &[TaggedEntity],
) {
    if query.contains('"')
        || query.contains(':')
        || query.contains('-')
        || query.contains('|')
        || query.contains(" OR ")
        || query.contains(" AND ")
        || query
            .split_whitespace()
            .any(|w| matches!(w.to_lowercase().as_str(), "def" | "definition"))
    {
        return;
    }

    let store = get_store(lang);
    let mut groups: Vec<String> = Vec::new();
    let mut last_idx = 0;

    let process_standard_text = |text: &str, target_groups: &mut Vec<String>| {
        for term in text.split_whitespace() {
            let term_lower = term.to_lowercase();

            if STOPWORDS.contains(term_lower.as_str()) {
                continue;
            }

            if *intent == QueryIntent::Local && LOCAL_NO_EXPAND.contains(term_lower.as_str()) {
                target_groups.push(format!("\"{}\"^2.0", term));
                continue;
            }

            match store.as_ref().and_then(|s| s.lookup(&term_lower)) {
                Some(synonyms) if !synonyms.is_empty() => {
                    let mut parts = vec![format!("\"{}\"^2.0", term)];
                    parts.extend(synonyms.iter().take(2).map(|s| format!("\"{}\"^1.0", s)));
                    target_groups.push(format!("({})", parts.join(" OR ")));
                }
                _ => {
                    target_groups.push(format!("\"{}\"^2.0", term));
                }
            }
        }
    };

    let mut sorted_tags = tags.to_vec();
    sorted_tags.sort_by_key(|t| t.range.start);

    for tag in sorted_tags {
        if tag.range.start > last_idx {
            process_standard_text(&query[last_idx..tag.range.start], &mut groups);
        }

        let entity_text = &query[tag.range.clone()];
        let exact_phrase = format!("\"{}\"", entity_text);
        let fast_entity_query = format!(
            "(title:{exact} OR description:{exact} OR keywords:{exact})",
            exact = exact_phrase
        );

        match tag.entity_type {
            EntityType::Business => groups.push(format!("{}^3.0", fast_entity_query)),
            EntityType::Place => groups.push(format!("{}^2.5", fast_entity_query)),
            EntityType::PersonName => groups.push(format!("{}^2.5", fast_entity_query)),
        }

        last_idx = tag.range.end;
    }

    if last_idx < query.len() {
        process_standard_text(&query[last_idx..], &mut groups);
    }

    if !groups.is_empty() {
        if groups.len() > 1 {
            let strict_and = groups
                .iter()
                .map(|g| format!("+({})", g))
                .collect::<Vec<_>>()
                .join(" ");

            let broad_or = groups.join(" ");

            *query = format!("({})^3.0 OR ({})", strict_and, broad_or);
        } else {
            *query = groups.join(" ");
        }
    }
}
