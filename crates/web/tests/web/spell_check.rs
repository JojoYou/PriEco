//! Tests for src/web/functions/additional/spell_check.rs

use crate::support::init;
use crate::web::functions::additional::spell_check::spell_check_query;

fn check(q: &str) -> Option<String> {
    init();
    spell_check_query(q)
}

#[test]
fn correct_queries_return_none() {
    for q in ["hello world", "receive", "the government", "search engine privacy"] {
        assert_eq!(check(q), None, "{}", q);
    }
}

#[test]
fn empty_and_whitespace_return_none() {
    assert_eq!(check(""), None);
    assert_eq!(check("    "), None);
    assert_eq!(check("\t\n"), None);
}

#[test]
fn single_typo_is_corrected() {
    assert_eq!(check("recieve").as_deref(), Some("receive"));
    assert_eq!(check("definately").as_deref(), Some("definitely"));
    assert_eq!(check("acommodate").as_deref(), Some("accommodate"));
    assert_eq!(check("seperate").as_deref(), Some("separate"));
    assert_eq!(check("goverment").as_deref(), Some("government"));
}

#[test]
fn short_typo_is_corrected_with_distance_one() {
    assert_eq!(check("wrld").as_deref(), Some("world"));
}

#[test]
fn several_typos_are_all_corrected() {
    assert_eq!(
        check("recieve definately").as_deref(),
        Some("receive definitely")
    );
}

#[test]
fn correct_words_are_kept_next_to_corrected_ones() {
    assert_eq!(
        check("please recieve this").as_deref(),
        Some("please receive this")
    );
}

#[test]
fn whitespace_is_normalised_in_corrections() {
    assert_eq!(
        check("  recieve    definately ").as_deref(),
        Some("receive definitely")
    );
}

#[test]
fn unknown_gibberish_is_left_alone() {
    assert_eq!(check("zzqqxxyyww"), None);
}

#[test]
fn protected_entities_are_not_corrected() {
    // "helo" is in the entity name list, so it is protected from correction
    assert_eq!(check("helo"), None);
}

#[test]
fn unicode_and_symbols_do_not_panic() {
    for q in ["Žltý kôň", "日本語", "🐴🐴", "c++ vs c#", "a\u{0}b", "İstanbul", "ß"] {
        let _ = check(q);
    }
}

#[test]
fn very_long_query_does_not_panic() {
    let q = "recieve ".repeat(200);
    let out = check(&q).unwrap();
    assert_eq!(out.split(' ').count(), 200);
    assert!(out.split(' ').all(|w| w == "receive"));
}

/// BUG: candidates are ranked by (distance, common prefix length, frequency).
/// Prefix length beats frequency, so "teh" becomes "tech" ("te" prefix) instead of the
/// far more common "the" (and "teh" is additionally protected as a person name).
#[test]
fn classic_teh_typo_becomes_the() {
    assert_eq!(check("teh").as_deref(), Some("the"));
}
