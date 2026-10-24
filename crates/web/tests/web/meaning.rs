//! Tests for src/web/functions/ranking/meaning/{intent, entities, optimize, call}.rs

use prieco_core::{EntityType, QU_PIPELINE, QueryIntent, TaggedEntity};

use crate::support::init;
use crate::web::functions::ranking::meaning::{
    call::process_query, entities::scan_entities, intent::intent,
    optimize::synynyms_and_optimize,
};

/* ───────────────────────────── intent ───────────────────────────── */

#[test]
fn intent_keywords_english() {
    init();
    assert_eq!(intent("how to bake bread", "en"), QueryIntent::Informational);
    assert_eq!(intent("what is rust", "en"), QueryIntent::Informational);
    assert_eq!(intent("buy iphone", "en"), QueryIntent::Transactional);
    assert_eq!(intent("cheap flights", "en"), QueryIntent::CommercialInvestigation);
    assert_eq!(intent("best laptop", "en"), QueryIntent::CommercialInvestigation);
    assert_eq!(intent("pizza near me", "en"), QueryIntent::Local);
    assert_eq!(intent("near", "en"), QueryIntent::Local);
    assert_eq!(intent("gmail login", "en"), QueryIntent::Navigational);
    assert_eq!(intent("facebook", "en"), QueryIntent::Navigational);
}

#[test]
fn intent_is_case_insensitive() {
    init();
    assert_eq!(intent("HOW TO COOK", "en"), QueryIntent::Informational);
    assert_eq!(intent("Buy Shoes", "en"), QueryIntent::Transactional);
}

#[test]
fn intent_unknown_language_falls_back_to_english() {
    init();
    assert_eq!(intent("how to cook", "xx"), QueryIntent::Informational);
    assert_eq!(intent("how to cook", ""), QueryIntent::Informational);
    assert_eq!(intent("how to cook", "all"), QueryIntent::Informational);
}

#[test]
fn intent_every_supported_language_works() {
    init();
    for lang in ["ar", "bg", "ca", "cs", "da", "de", "el", "en", "es", "et", "fi", "fr", "he",
                 "hr", "hu", "id", "is", "it", "ja", "ko", "lt", "lv", "nl", "no", "pl", "pt",
                 "ro", "ru", "sk", "sl", "sr", "sv", "tr"] {
        let _ = intent("some query words here", lang);
        assert_eq!(intent("", lang), QueryIntent::Unknown, "{}", lang);
    }
}

#[test]
fn intent_heuristics_without_keywords() {
    init();
    assert_eq!(intent("", "en"), QueryIntent::Unknown);
    assert_eq!(intent("rust", "en"), QueryIntent::Navigational);
    assert_eq!(intent("rust lang", "en"), QueryIntent::Navigational);
    assert_eq!(intent("rust borrow checker", "en"), QueryIntent::Unknown);
    assert_eq!(intent("qwrt zxcv pqmn", "en"), QueryIntent::Unknown);
    assert_eq!(intent("a b c d", "en"), QueryIntent::Unknown);
    assert_eq!(
        intent("zebra xylophone quartz jungle vortex", "en"),
        QueryIntent::Informational
    );
}

/// BUG: a whitespace-only query has 0 tokens but is not `is_empty()`, so it is
/// classified as Navigational instead of Unknown.
#[test]
fn intent_whitespace_only_query_is_unknown() {
    init();
    assert_eq!(intent("   ", "en"), QueryIntent::Unknown);
}

#[test]
fn intent_leftmost_keyword_wins() {
    init();
    // "buy" (transactional) is left of "how to" (informational)
    assert_eq!(intent("buy how to", "en"), QueryIntent::Transactional);
    assert_eq!(intent("how to buy", "en"), QueryIntent::Informational);
}

#[test]
fn intent_handles_unicode_queries() {
    init();
    let _ = intent("Žltý kôň 🐴 日本語", "sk");
    let _ = intent("مرحبا", "ar");
}

/// BUG: keywords are matched as raw substrings without word boundaries:
/// "pirates" contains "rates" (transactional).
#[test]
fn intent_pirates_is_not_a_purchase() {
    init();
    assert_ne!(intent("pirates of the caribbean", "en"), QueryIntent::Transactional);
}

/// BUG: "show tomorrow" contains "how to".
#[test]
fn intent_show_tomorrow_is_not_a_how_to() {
    init();
    assert_ne!(intent("show tomorrow", "en"), QueryIntent::Informational);
}

/// BUG: "stop" contains "top" (commercial), "homework" contains "home" (navigational),
/// "playlist" contains "list" (informational).
#[test]
fn intent_keywords_inside_words_do_not_match() {
    init();
    assert_ne!(intent("stop smoking", "en"), QueryIntent::CommercialInvestigation);
    assert_eq!(intent("homework", "en"), QueryIntent::Navigational); // 1 token heuristic agrees
    assert_ne!(intent("playlist", "en"), QueryIntent::Informational);
    assert_ne!(intent("macbook", "en"), QueryIntent::Transactional);
}

/* ───────────────────────────── scan_entities ───────────────────────────── */

const BIZ: &str = "abn amro bank n.v.";
const PLACE: &str = "trnava pri laborci";

#[test]
fn entities_none_found_keeps_intent() {
    init();
    let mut q = String::from("zzqqxxyy");
    for i in [
        QueryIntent::Unknown,
        QueryIntent::Informational,
        QueryIntent::Transactional,
        QueryIntent::Local,
    ] {
        let (out, coords, tags) = scan_entities(&mut q, i, "sk");
        assert_eq!(out, i);
        assert!(coords.is_none());
        assert!(tags.is_empty());
    }
}

#[test]
fn entities_business_upgrades_unknown_to_navigational() {
    init();
    let mut q = BIZ.to_string();
    let (out, _, tags) = scan_entities(&mut q, QueryIntent::Unknown, "nl");
    assert_eq!(out, QueryIntent::Navigational);
    assert!(tags.iter().any(|t| t.entity_type == EntityType::Business));
}

#[test]
fn entities_business_keeps_known_intents() {
    init();
    for i in [
        QueryIntent::Informational,
        QueryIntent::Transactional,
        QueryIntent::CommercialInvestigation,
        QueryIntent::Navigational,
        QueryIntent::Local,
    ] {
        let mut q = BIZ.to_string();
        assert_eq!(scan_entities(&mut q, i, "nl").0, i);
    }
}

#[test]
fn entities_person_upgrades_unknown_and_navigational_to_informational() {
    init();
    for i in [QueryIntent::Unknown, QueryIntent::Navigational] {
        let mut q = String::from("einstein");
        assert_eq!(scan_entities(&mut q, i, "de").0, QueryIntent::Informational);
    }
    let mut q = String::from("einstein");
    assert_eq!(
        scan_entities(&mut q, QueryIntent::Transactional, "de").0,
        QueryIntent::Transactional
    );
}

#[test]
fn entities_place_upgrades_unknown_and_navigational_to_local() {
    init();
    for i in [QueryIntent::Unknown, QueryIntent::Navigational] {
        let mut q = PLACE.to_string();
        assert_eq!(scan_entities(&mut q, i, "sk").0, QueryIntent::Local);
    }
    let mut q = PLACE.to_string();
    assert_eq!(
        scan_entities(&mut q, QueryIntent::Informational, "sk").0,
        QueryIntent::Informational
    );
}

#[test]
fn entities_returns_same_tags_as_pipeline() {
    init();
    let mut q = format!("{} {}", BIZ, PLACE);
    let expected: Vec<(std::ops::Range<usize>, EntityType)> = QU_PIPELINE
        .get_tags(&q)
        .into_iter()
        .map(|t| (t.range, t.entity_type))
        .collect();
    let (_, _, tags) = scan_entities(&mut q, QueryIntent::Unknown, "sk");
    let got: Vec<_> = tags.into_iter().map(|t| (t.range, t.entity_type)).collect();
    assert_eq!(got, expected);
}

#[test]
fn entities_do_not_modify_query() {
    init();
    let mut q = format!("{} hotels", PLACE);
    let before = q.clone();
    scan_entities(&mut q, QueryIntent::Unknown, "sk");
    assert_eq!(q, before);
}

/// BUG: patterns are lowercased place names, but the registry keys keep their
/// original case ("Trnava pri Laborci"), so `places.get(matched_text)` never finds
/// coordinates for any capitalised place.
#[test]
fn entities_place_returns_coordinates() {
    init();
    let mut q = PLACE.to_string();
    let (_, coords, _) = scan_entities(&mut q, QueryIntent::Unknown, "sk");
    let (lat, lon) = coords.expect("no coordinates for a known place");
    // Trnava pri Laborci, Slovakia
    assert!((lat - 48.8).abs() < 1.0, "{}", lat);
    assert!((lon - 21.9).abs() < 1.0, "{}", lon);
}

/* ───────────────────────────── synynyms_and_optimize ───────────────────────────── */

fn opt(q: &str, intent: QueryIntent, tags: &[TaggedEntity]) -> String {
    init();
    let mut s = q.to_string();
    // "xx" has no synonym store -> deterministic output
    synynyms_and_optimize(&mut s, "xx", &intent, tags);
    s
}

#[test]
fn optimize_skips_queries_with_operators() {
    for q in [
        "\"exact phrase\"",
        "site:example.com rust",
        "rust -java",
        "a | b",
        "a OR b",
        "a AND b",
        "title:rust",
    ] {
        assert_eq!(opt(q, QueryIntent::Unknown, &[]), q, "{}", q);
    }
}

#[test]
fn optimize_skips_definition_queries() {
    assert_eq!(opt("definition rust", QueryIntent::Unknown, &[]), "definition rust");
    assert_eq!(opt("def rust", QueryIntent::Unknown, &[]), "def rust");
}

/// BUG: the "def" check is a substring test, so every query containing "def"
/// (default, defense, undefined...) skips stop word removal and boosting.
#[test]
fn optimize_does_not_skip_words_containing_def() {
    assert_ne!(opt("the default", QueryIntent::Unknown, &[]), "the default");
}

#[test]
fn optimize_single_term() {
    assert_eq!(opt("cat", QueryIntent::Unknown, &[]), "\"cat\"^2.0");
}

#[test]
fn optimize_preserves_original_case_of_terms() {
    assert_eq!(opt("Cat", QueryIntent::Unknown, &[]), "\"Cat\"^2.0");
}

#[test]
fn optimize_removes_stop_words() {
    assert_eq!(opt("the cat", QueryIntent::Unknown, &[]), "\"cat\"^2.0");
    assert_eq!(opt("THE Cat", QueryIntent::Unknown, &[]), "\"Cat\"^2.0");
}

#[test]
fn optimize_only_stop_words_leaves_query_untouched() {
    assert_eq!(opt("the a of", QueryIntent::Unknown, &[]), "the a of");
    assert_eq!(opt("what is", QueryIntent::Unknown, &[]), "what is");
}

#[test]
fn optimize_empty_query() {
    assert_eq!(opt("", QueryIntent::Unknown, &[]), "");
    assert_eq!(opt("   ", QueryIntent::Unknown, &[]), "   ");
}

#[test]
fn optimize_two_terms_build_strict_and_broad_parts() {
    assert_eq!(
        opt("red car", QueryIntent::Unknown, &[]),
        "(+(\"red\"^2.0) +(\"car\"^2.0))^3.0 OR (\"red\"^2.0 \"car\"^2.0)"
    );
}

#[test]
fn optimize_three_terms() {
    assert_eq!(
        opt("fast red car", QueryIntent::Unknown, &[]),
        "(+(\"fast\"^2.0) +(\"red\"^2.0) +(\"car\"^2.0))^3.0 OR (\"fast\"^2.0 \"red\"^2.0 \"car\"^2.0)"
    );
}

#[test]
fn optimize_collapses_whitespace_between_terms() {
    assert_eq!(opt("  cat \t ", QueryIntent::Unknown, &[]), "\"cat\"^2.0");
}

fn tag(range: std::ops::Range<usize>, t: EntityType, text: &str) -> TaggedEntity {
    TaggedEntity {
        range,
        entity_type: t,
        matched_text: text.to_string(),
    }
}

#[test]
fn optimize_business_entity_is_searched_in_title_description_keywords() {
    let out = opt(
        "google maps",
        QueryIntent::Navigational,
        &[tag(0..6, EntityType::Business, "google")],
    );
    let ent = "(title:\"google\" OR description:\"google\" OR keywords:\"google\")^3.0";
    assert_eq!(
        out,
        format!("(+({}) +(\"maps\"^2.0))^3.0 OR ({} \"maps\"^2.0)", ent, ent)
    );
}

#[test]
fn optimize_entity_boosts_by_type() {
    for (t, boost) in [
        (EntityType::Business, "3.0"),
        (EntityType::Place, "2.5"),
        (EntityType::PersonName, "2.5"),
    ] {
        let out = opt("london", QueryIntent::Unknown, &[tag(0..6, t, "london")]);
        assert_eq!(
            out,
            format!(
                "(title:\"london\" OR description:\"london\" OR keywords:\"london\")^{}",
                boost
            )
        );
    }
}

#[test]
fn optimize_entity_uses_original_query_text() {
    let out = opt("London", QueryIntent::Unknown, &[tag(0..6, EntityType::Place, "london")]);
    assert!(out.contains("title:\"London\""), "{}", out);
}

#[test]
fn optimize_entity_in_middle_of_query() {
    let out = opt(
        "hotels london cheap",
        QueryIntent::Unknown,
        &[tag(7..13, EntityType::Place, "london")],
    );
    let ent = "(title:\"london\" OR description:\"london\" OR keywords:\"london\")^2.5";
    assert_eq!(
        out,
        format!(
            "(+(\"hotels\"^2.0) +({}) +(\"cheap\"^2.0))^3.0 OR (\"hotels\"^2.0 {} \"cheap\"^2.0)",
            ent, ent
        )
    );
}

#[test]
fn optimize_tags_are_processed_in_position_order() {
    let unsorted = [
        tag(7..13, EntityType::Place, "london"),
        tag(0..6, EntityType::PersonName, "connor"),
    ];
    let out = opt("connor london", QueryIntent::Unknown, &unsorted);
    let first = out.find("connor").unwrap();
    let second = out.find("london").unwrap();
    assert!(first < second, "{}", out);
}

#[test]
fn optimize_local_intent_never_expands_location_words() {
    init();
    let mut q = String::from("near");
    // even with a real synonym store, local words stay literal
    synynyms_and_optimize(&mut q, "en", &QueryIntent::Local, &[]);
    assert_eq!(q, "\"near\"^2.0");
}

#[test]
fn optimize_with_english_synonyms_produces_or_groups() {
    init();
    let mut found = false;
    for w in ["car", "big", "happy", "house", "fast", "buy", "good"] {
        let mut q = w.to_string();
        synynyms_and_optimize(&mut q, "en", &QueryIntent::Unknown, &[]);
        if q.starts_with('(') {
            found = true;
            assert!(q.contains(&format!("\"{}\"^2.0", w)), "{}", q);
            assert!(q.contains(" OR "), "{}", q);
            // at most the term + 2 synonyms
            assert!(q.matches("^1.0").count() <= 2, "{}", q);
        }
    }
    assert!(found, "no English word expanded with synonyms");
}

/* ───────────────────────────── process_query ───────────────────────────── */

#[test]
fn process_query_informational() {
    init();
    let mut q = String::from("how to bake bread");
    let (i, coords) = process_query(&mut q, "en", "us");
    assert_eq!(i, QueryIntent::Informational);
    assert!(coords.is_none());
    assert_ne!(q, "how to bake bread", "query should be rewritten for the index");
}

#[test]
fn process_query_with_operator_is_left_alone() {
    init();
    let mut q = String::from("\"exact words\"");
    process_query(&mut q, "en", "us");
    assert_eq!(q, "\"exact words\"");
}

#[test]
fn process_query_place_becomes_local() {
    init();
    let mut q = PLACE.to_string();
    let (i, _) = process_query(&mut q, "sk", "sk");
    assert_eq!(i, QueryIntent::Local);
}

#[test]
fn process_query_never_panics_on_odd_input() {
    init();
    for s in ["", " ", "!!!", "Žltý kôň", "İstanbul hotels", "ß straße", "日本 東京", "a\u{0}b"] {
        let mut q = s.to_string();
        let _ = process_query(&mut q, "en", "us");
    }
}
