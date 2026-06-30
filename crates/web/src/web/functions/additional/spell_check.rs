use prieco_core::SPELL_CHECKER;
use symspell::{Suggestion, Verbosity};

pub fn spell_check_query(q: &str) -> Option<String> {
    let mut changed = false;
    let corrected: Vec<String> = q
        .split_whitespace()
        .map(|word| match correct_word(word) {
            Some(c) => {
                changed = true;
                c
            }
            None => word.to_string(),
        })
        .collect();

    changed.then(|| corrected.join(" "))
}

fn correct_word(word: &str) -> Option<String> {
    let best = best_correction(word)?;

    if !should_suggest(word, &best) {
        return None;
    }

    (best.term.to_lowercase() != word.to_lowercase()).then_some(best.term)
}

fn best_correction(word: &str) -> Option<Suggestion> {
    let max_dist = max_edit_distance_for(word);
    let candidates: Vec<Suggestion> = SPELL_CHECKER.lookup(word, Verbosity::All, max_dist);
    let word_lower = word.to_lowercase();

    candidates.into_iter().max_by_key(|c| {
        let cand_lower = c.term.to_lowercase();
        (
            -c.distance,
            word_lower
                .chars()
                .zip(cand_lower.chars())
                .take_while(|(x, y)| x == y)
                .count(),
            c.count,
        )
    })
}

/* Helper functions */
fn max_edit_distance_for(word: &str) -> i64 {
    match word.chars().count() {
        0..=4 => 1,
        5..=8 => 2,
        _ => 2,
    }
}

fn should_suggest(original: &str, candidate: &Suggestion) -> bool {
    if candidate.term.to_lowercase() == original.to_lowercase() {
        return false;
    }

    if original.chars().count() <= 4 && candidate.distance > 1 {
        return false;
    }
    true
}
