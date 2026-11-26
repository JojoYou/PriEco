//! # Spell Checker
//!
//! Suggests corrected query. The original query is still searched.
//!
//! ## Architecture
//!
//! 1. Every word is looked up in [SPELL_CHECKER], English only.
//! 2. Words that are entities (places, names, businesses) stay,
//!    unless the best correction is 1 edit away and very common.
//! 3. Best correction: lowest edit distance, then most common, then longest same start.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-06-29
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] More languages

use prieco_core::{QU_PIPELINE, SPELL_CHECKER};
use symspell::{Suggestion, Verbosity};

/// Corrects query.
///
/// # Arguments
///
/// * `q` - Query.
///
/// # Returns
///
/// Corrected query, `None` when nothing changed.
pub fn spell_check_query(q: &str) -> Option<String> {
    let tags = QU_PIPELINE.get_tags(q);

    let mut changed = false;
    let mut corrected_words = Vec::new();

    for word in q.split_whitespace() {
        let start_idx = word.as_ptr() as usize - q.as_ptr() as usize;

        let protected = tags.iter().any(|tag| tag.range.contains(&start_idx))
            && !best_correction(word)
                .is_some_and(|best| best.distance == 1 && best.count >= 1_000_000_000);

        if protected {
            corrected_words.push(word.to_string());
        } else {
            match correct_word(word) {
                Some(c) => {
                    changed = true;
                    corrected_words.push(c);
                }
                None => corrected_words.push(word.to_string()),
            }
        }
    }

    changed.then(|| corrected_words.join(" "))
}

/// Correction of one word, `None` when it is fine or the correction isn't good enough.
fn correct_word(word: &str) -> Option<String> {
    let best = best_correction(word)?;

    if !should_suggest(word, &best) {
        return None;
    }

    (best.term.to_lowercase() != word.to_lowercase()).then_some(best.term)
}

/// Best dictionary match of a word.
fn best_correction(word: &str) -> Option<Suggestion> {
    let max_dist = max_edit_distance_for(word);
    let candidates: Vec<Suggestion> = SPELL_CHECKER.lookup(word, Verbosity::All, max_dist);
    let word_lower = word.to_lowercase();

    candidates.into_iter().max_by_key(|c| {
        let prefix = word_lower
            .chars()
            .zip(c.term.to_lowercase().chars())
            .take_while(|(x, y)| x == y)
            .count();
        (-c.distance, c.count, prefix)
    })
}

/* Helper functions */
/// Max edits: 1 for words up to 4 characters, otherwise 2.
fn max_edit_distance_for(word: &str) -> i64 {
    match word.chars().count() {
        0..=4 => 1,
        5..=8 => 2,
        _ => 2,
    }
}

/// False for same word or more than 1 edit on short words.
fn should_suggest(original: &str, candidate: &Suggestion) -> bool {
    if candidate.term.to_lowercase() == original.to_lowercase() {
        return false;
    }

    if original.chars().count() <= 4 && candidate.distance > 1 {
        return false;
    }
    true
}
