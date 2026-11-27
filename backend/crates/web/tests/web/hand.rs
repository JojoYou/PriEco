//! Tests for src/web/functions/ranking/hand.rs (hand tuned ranking boosts)
//!
//! A "baseline" document gets a boost of exactly 1.0; every test changes one
//! property and checks the raw quality multiplier against RankingWeights::default().
//! `run` bounds quality by QUALITY_BOUND, goggles are checked unbounded.

use std::sync::Arc;

use prieco_core::{QueryIntent, WebDocument, url_to_domain_id};

use crate::support::{doc, init};
use crate::web::functions::ranking::{goggles::GoggleRules, hand};

const W_LOC: f64 = 1.242;
const W_TLD_LOC: f64 = 1.2;
const W_WIKI: f64 = 1.3905070941466917;
const W_HTTPS: f64 = 1.3521680592049155;
const W_DEV_COM: f64 = 1.01;
const W_ORG_NET: f64 = 1.0343461125773576;
const W_BAD_URL: f64 = 0.99;
const W_DEPTH: f64 = 0.7579354778664352;
const W_EFFORT: f64 = 0.26516713994470764;
const W_500: f64 = 1.15;
const W_SHOP_PRODUCT: f64 = 1.2;
const W_SHOP_DOMAIN: f64 = 1.1151867372193276;
const W_INFO_WIKI: f64 = 1.3623035283335967;
const W_INFO_GOV: f64 = 1.2588360222339798;
const W_INFO_SHOP: f64 = 0.6423278191323988;
const W_TITLE: f64 = 1.3818951856263455;

/// Baseline: no https, no .com/.org/.net/.dev, no tld/loc match, foreign language,
/// 0.5s load (neutral), shallow url, nothing matching the query.
fn base(url: &str) -> WebDocument {
    let mut d = doc(url);
    d.search_score = 1.0;
    d.lang = "xx".into();
    d.loc = "xx".into();
    d.load = 0.5;
    d
}

fn rank(
    docs: Vec<WebDocument>,
    query: &str,
    lang: &str,
    loc: &str,
    intent: QueryIntent,
    goggles: &[Arc<GoggleRules>],
    mobile: bool,
) -> Vec<WebDocument> {
    init();
    let mut v = docs;
    hand::run(&mut v, query, lang, loc, &intent, goggles, mobile);
    v
}

/// Raw quality multiplier of a single document, before bounding
fn factor_with(d: WebDocument, query: &str, lang: &str, loc: &str, intent: QueryIntent) -> f64 {
    hand::quality(
        &d,
        query,
        lang,
        loc,
        &intent,
        &hand::RankingWeights::default(),
        false,
    )
}

fn factor(d: WebDocument, intent: QueryIntent) -> f64 {
    factor_with(d, "zzz", "en", "zz", intent)
}

/// Score after `run` of a single document (initial score 1.0)
fn ranked(d: WebDocument, query: &str, intent: QueryIntent, goggles: &[Arc<GoggleRules>]) -> f64 {
    rank(vec![d], query, "en", "zz", intent, goggles, false)[0].search_score as f64
}

fn goggle(d: WebDocument, goggles: &[Arc<GoggleRules>]) -> f64 {
    hand::goggle_factor(&d, goggles)
}

fn assert_factor(got: f64, expected: f64) {
    assert!(
        ((got - expected) / expected).abs() < 1e-6,
        "got {} expected {}",
        got,
        expected
    );
}

/* ───────────────────────────── basics ───────────────────────────── */

#[test]
fn empty_results_are_fine() {
    let out = rank(vec![], "q", "en", "us", QueryIntent::Unknown, &[], false);
    assert!(out.is_empty());
}

#[test]
fn baseline_document_has_boost_one() {
    assert_factor(factor(base("http://a.xyz"), QueryIntent::Unknown), 1.0);
}

#[test]
fn zero_score_stays_zero() {
    let mut d = base("https://a.com/");
    d.search_score = 0.0;
    assert_eq!(ranked(d, "a", QueryIntent::Navigational, &[]), 0.0);
}

#[test]
fn results_are_sorted_by_boosted_score() {
    let slow = {
        let mut d = base("http://slow.xyz");
        d.load = 10.0;
        d
    };
    let fast = {
        let mut d = base("http://fast.xyz");
        d.load = 0.1;
        d.search_score = 0.8;
        d
    };
    let out = rank(
        vec![slow, fast],
        "q",
        "en",
        "zz",
        QueryIntent::Unknown,
        &[],
        false,
    );
    assert_eq!(out[0].url, "http://fast.xyz");
    assert_eq!(out[1].url, "http://slow.xyz");
}

#[test]
fn nan_scores_do_not_panic_while_sorting() {
    let mut a = base("http://a.xyz");
    a.search_score = f32::NAN;
    let b = base("http://b.xyz");
    let out = rank(
        vec![a, b],
        "q",
        "en",
        "zz",
        QueryIntent::Unknown,
        &[],
        false,
    );
    assert_eq!(out.len(), 2);
}

/* ───────────────────────────── bound ───────────────────────────── */

#[test]
fn stacked_boosts_are_capped_at_bound() {
    let mut d = base("https://a.com");
    d.title = "rust".into();
    assert!(
        factor_with(d.clone(), "rust", "en", "zz", QueryIntent::Unknown)
            > 1.0 + hand::QUALITY_BOUND
    );
    assert_factor(
        ranked(d, "rust", QueryIntent::Unknown, &[]),
        1.0 + hand::QUALITY_BOUND,
    );
}

#[test]
fn stacked_penalties_are_capped_at_bound() {
    let mut d = base("http://a.xyz/b/c/d/e");
    d.load = 10.0;
    assert_factor(
        ranked(d, "q", QueryIntent::Unknown, &[]),
        1.0 / (1.0 + hand::QUALITY_BOUND),
    );
}

#[test]
fn small_factors_are_not_changed_by_bound() {
    let mut d = base("http://a.xyz");
    d.load = 0.1;
    assert_factor(ranked(d, "q", QueryIntent::Unknown, &[]), 1.08);
}

#[test]
fn goggles_are_not_bounded() {
    let id = url_to_domain_id("https://a.xyz");
    let mut g = rules();
    g.downrank.insert(id, 3.0);
    assert_factor(
        ranked(
            base("https://a.xyz"),
            "q",
            QueryIntent::Unknown,
            &[Arc::new(g)],
        ),
        (1.0 + hand::QUALITY_BOUND) * 0.25,
    );
}

/* ───────────────────────────── url signals ───────────────────────────── */

#[test]
fn https_boost() {
    assert_factor(factor(base("https://a.xyz"), QueryIntent::Unknown), W_HTTPS);
}

#[test]
fn com_and_dev_boost() {
    assert_factor(
        factor(base("http://a.com"), QueryIntent::Unknown),
        W_DEV_COM,
    );
    assert_factor(
        factor(base("http://a.dev"), QueryIntent::Unknown),
        W_DEV_COM,
    );
}

#[test]
fn org_and_net_boost() {
    assert_factor(
        factor(base("http://a.org"), QueryIntent::Unknown),
        W_ORG_NET,
    );
    assert_factor(
        factor(base("http://a.net"), QueryIntent::Unknown),
        W_ORG_NET,
    );
}

#[test]
fn tld_matching_location_boost() {
    let d = base("http://a.xyz");
    assert_factor(
        factor_with(d, "zzz", "en", "XYZ", QueryIntent::Unknown),
        W_TLD_LOC,
    );
}

#[test]
fn language_is_not_a_factor() {
    let mut d = base("http://a.xyz");
    d.lang = "en".into();
    assert_factor(factor(d, QueryIntent::Unknown), 1.0);
}

#[test]
fn location_boost_only_for_local_intent() {
    let mut d = base("http://a.xyz");
    d.loc = "zz".into();
    assert_factor(factor(d.clone(), QueryIntent::Unknown), 1.0);
    // Local intent itself has no other multiplier except the `_` relevance branch (neutral here)
    assert_factor(factor(d, QueryIntent::Local), W_LOC);
}

#[test]
fn bad_url_patterns_penalty() {
    for u in ["http://a_b.xyz", "http://a.xyz:8080", "http://a,,b.xyz"] {
        assert_factor(factor(base(u), QueryIntent::Unknown), W_BAD_URL);
    }
}

#[test]
fn deep_urls_are_penalised_per_extra_slash() {
    assert_factor(
        factor(base("http://a.xyz/b"), QueryIntent::Unknown),
        W_DEPTH,
    );
    assert_factor(
        factor(base("http://a.xyz/b/c"), QueryIntent::Unknown),
        W_DEPTH.powi(2),
    );
    assert_factor(
        factor(base("http://a.xyz/b/c/d/e"), QueryIntent::Unknown),
        W_DEPTH.powi(4),
    );
}

#[test]
fn query_string_and_fragment_are_ignored() {
    assert_factor(
        factor(base("http://a.xyz?x=1/2/3"), QueryIntent::Unknown),
        1.0,
    );
    assert_factor(factor(base("http://a.xyz#/x/y"), QueryIntent::Unknown), 1.0);
}

/// BUG: the depth penalty counts raw slashes, so "http://a.xyz/" (3 slashes) is
/// penalised against "http://a.xyz" (2 slashes) although both are the same homepage.
#[test]
fn trailing_slash_does_not_change_score() {
    let a = factor(base("http://a.xyz"), QueryIntent::Unknown);
    let b = factor(base("http://a.xyz/"), QueryIntent::Unknown);
    assert_factor(b, a);
}

/* ───────────────────────────── load time ───────────────────────────── */

#[test]
fn load_time_buckets() {
    for (load, f) in [
        (0.1, 1.08),
        (0.25, 1.05),
        (0.4, 1.02),
        (0.5, 1.0),
        (1.19, 1.0),
        (1.5, 0.9),
        (3.0, 0.7),
        (4.9, 0.4),
        (5.0, 0.1),
        (60.0, 0.1),
    ] {
        let mut d = base("http://a.xyz");
        d.load = load;
        assert_factor(factor(d, QueryIntent::Unknown), f);
    }
}

#[test]
fn unknown_load_time_counts_as_fastest() {
    // Documents current behaviour: load 0.0 (unknown, e.g. discovered pages) and
    // negative values get the "very fast" boost
    for load in [0.0, -1.0] {
        let mut d = base("http://a.xyz");
        d.load = load;
        assert_factor(factor(d, QueryIntent::Unknown), 1.08);
    }
}

#[test]
fn nan_load_gets_the_slowest_penalty() {
    let mut d = base("http://a.xyz");
    d.load = f64::NAN;
    assert_factor(factor(d, QueryIntent::Unknown), 0.1);
}

/* ───────────────────────────── content signals ───────────────────────────── */

#[test]
fn query_in_title_boost_case_insensitive() {
    let mut d = base("http://a.xyz");
    d.title = "The RUST Programming Language".into();
    assert_factor(
        factor_with(d, "Rust", "en", "zz", QueryIntent::Unknown),
        W_TITLE,
    );
}

#[test]
fn quality_score_boost_is_clamped() {
    for (qna, f) in [
        (600.0f32, 1.25),
        (1200.0, 1.25),
        (240.0, 1.1),
        (-240.0, 0.9),
        (-10_000.0, 0.8),
    ] {
        let mut d = base("http://a.xyz");
        d.qna = qna;
        assert_factor(factor(d, QueryIntent::Unknown), f);
    }
}

#[test]
fn unknown_intent_uses_confidence_and_effort() {
    let mut d = base("http://a.xyz");
    d.confidence = 50.0;
    assert_factor(factor(d, QueryIntent::Unknown), 1.0 + 0.5 * 0.001 * 10.0);

    let mut d = base("http://a.xyz");
    d.effort = 500.0;
    assert_factor(factor(d, QueryIntent::Unknown), 1.0 + 0.5 * W_EFFORT);

    let mut d = base("http://a.xyz");
    d.effort = 100_000.0;
    assert_factor(factor(d, QueryIntent::Unknown), 1.0 + 0.5 * W_EFFORT);
}

#[test]
fn has_500_words_boost_depends_on_intent() {
    let mut d = base("http://a.xyz");
    d.has_500_words = true;
    assert_factor(factor(d.clone(), QueryIntent::Unknown), W_500);
    assert_factor(factor(d.clone(), QueryIntent::Navigational), W_500);
    assert_factor(factor(d.clone(), QueryIntent::Informational), W_500 * 1.1);
}

#[test]
fn short_pages_penalised_for_informational_queries() {
    assert_factor(
        factor(base("http://a.xyz"), QueryIntent::Informational),
        0.9,
    );
}

#[test]
fn mobile_user_has_neutral_weights_by_default() {
    let mut d = base("http://a.xyz");
    d.is_mobile = false;
    let out = rank(vec![d], "q", "en", "zz", QueryIntent::Unknown, &[], true);
    assert_factor(out[0].search_score as f64, 1.0);
}

/* ───────────────────────────── intents ───────────────────────────── */

#[test]
fn navigational_is_not_a_factor() {
    let d = base("http://github.xyz");
    assert_factor(
        factor_with(d, "github", "en", "zz", QueryIntent::Navigational),
        1.0,
    );
}

#[test]
fn navigational_match_domain_homepage() {
    assert!(hand::navigational_match("http://github.xyz", "GitHub"));
    assert!(hand::navigational_match("https://github.com/", "github"));
    assert!(hand::navigational_match(
        "https://github.com/?tab=1",
        " github "
    ));
}

#[test]
fn navigational_match_ignores_www() {
    assert!(hand::navigational_match("http://www.github.xyz", "github"));
}

#[test]
fn navigational_match_other_domain_is_false() {
    assert!(!hand::navigational_match("https://gitlab.com", "github"));
    assert!(!hand::navigational_match("https://notgithub.com", "github"));
}

#[test]
fn navigational_locale_paths_count_as_homepage() {
    for path in ["en", "en-ca", "pt-BR", "zh-Hans"] {
        assert!(hand::navigational_match(
            &format!("http://x.xyz/{}", path),
            "x"
        ));
    }
}

#[test]
fn navigational_non_locale_paths_are_not_homepage() {
    for path in ["a", "toolongsegment", "en/us", "e1"] {
        assert!(!hand::navigational_match(
            &format!("http://x.xyz/{}", path),
            "x"
        ));
    }
}

/// Ordinary short pages like /login, /about, /blog or /pricing aren't homepages.
#[test]
fn navigational_regular_short_paths_are_not_homepages() {
    for path in ["login", "about", "blog", "pricing"] {
        assert!(!hand::navigational_match(
            &format!("http://x.xyz/{}", path),
            "x"
        ));
    }
}

#[test]
fn navigational_deep_page_is_not_a_match() {
    assert!(hand::navigational_match("https://github.com", "github"));
    assert!(!hand::navigational_match(
        "https://github.com/some/deep/page",
        "github"
    ));
}

#[test]
fn commercial_product_and_shop_domains() {
    assert_factor(
        factor(
            base("http://a.xyz/product/1"),
            QueryIntent::CommercialInvestigation,
        ),
        W_SHOP_PRODUCT * W_DEPTH.powi(2),
    );
    assert_factor(
        factor(base("http://a.shop"), QueryIntent::CommercialInvestigation),
        W_SHOP_DOMAIN,
    );
}

#[test]
fn commercial_quality_and_long_page_penalty() {
    let mut d = base("http://a.xyz");
    d.qna = 120.0;
    // general qna (1 + 120/2400) and commercial qna (1 + 120/1200)
    assert_factor(factor(d, QueryIntent::Transactional), 1.05 * 1.1);

    let mut d = base("http://a.xyz");
    d.effort = 501.0;
    assert_factor(factor(d, QueryIntent::CommercialInvestigation), 0.85);
}

#[test]
fn informational_prefers_wikipedia_and_gov_edu_and_penalises_shops() {
    let wiki = base("http://en.wikipedia.org");
    assert_factor(
        factor(wiki, QueryIntent::Informational),
        W_INFO_WIKI * W_INFO_GOV * W_ORG_NET * 0.9,
    );
    assert_factor(
        factor(base("http://a.edu"), QueryIntent::Informational),
        W_INFO_GOV * 0.9,
    );
    assert_factor(
        factor(base("http://a.shop"), QueryIntent::Informational),
        W_INFO_SHOP * 0.9,
    );
}

#[test]
fn wikipedia_article_language_signal() {
    let url = "http://en.wikipedia.org/wiki/Rust";
    let depth = W_DEPTH.powi(2);
    let same = factor_with(base(url), "zzz", "en", "zz", QueryIntent::Unknown);
    assert_factor(same, W_WIKI * W_ORG_NET * depth);
    let other = factor_with(base(url), "zzz", "de", "zz", QueryIntent::Unknown);
    assert_factor(other, W_ORG_NET * depth);
    let all = factor_with(base(url), "zzz", "all", "zz", QueryIntent::Unknown);
    assert_factor(all, W_ORG_NET * depth);
}

/* ───────────────────────────── goggles ───────────────────────────── */

fn rules() -> GoggleRules {
    GoggleRules::default()
}

#[test]
fn goggle_boost_and_downrank() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g = rules();
    g.boost.insert(id, 2.0);
    assert_factor(goggle(base("http://a.xyz"), &[Arc::new(g)]), 1.3);

    let mut g = rules();
    g.downrank.insert(id, 1.0);
    assert_factor(goggle(base("http://a.xyz"), &[Arc::new(g)]), 0.5);
}

#[test]
fn goggle_values_are_capped_at_three() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g = rules();
    g.boost.insert(id, 1000.0);
    assert_factor(goggle(base("http://a.xyz"), &[Arc::new(g)]), 1.45);
    let mut g = rules();
    g.downrank.insert(id, 1000.0);
    assert_factor(goggle(base("http://a.xyz"), &[Arc::new(g)]), 0.25);
}

#[test]
fn several_goggles_multiply() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g1 = rules();
    g1.boost.insert(id, 2.0);
    let mut g2 = rules();
    g2.boost.insert(id, 2.0);
    assert_factor(
        goggle(base("http://a.xyz"), &[Arc::new(g1), Arc::new(g2)]),
        1.3 * 1.3,
    );
}

#[test]
fn important_goggle_replaces_other_goggles() {
    let url = "https://a.com/x/y";
    let id = url_to_domain_id(url);
    let mut g = rules();
    g.important.insert(id);
    g.boost.insert(id, 2.0);
    let mut other = rules();
    other.boost.insert(id, 3.0);
    let f = goggle(base(url), &[Arc::new(g), Arc::new(other)]);
    assert_factor(f, 1.3);
}

#[test]
fn important_goggle_downrank_replaces_other_goggles() {
    let url = "https://a.com";
    let id = url_to_domain_id(url);
    let mut g = rules();
    g.important.insert(id);
    g.downrank.insert(id, 3.0);
    let f = goggle(base(url), &[Arc::new(g)]);
    assert_factor(f, 0.25);
}

#[test]
fn goggles_for_other_domains_have_no_effect() {
    let mut g = rules();
    g.boost
        .insert(url_to_domain_id("https://other.example"), 3.0);
    g.downrank
        .insert(url_to_domain_id("https://other2.example"), 3.0);
    assert_factor(goggle(base("http://a.xyz"), &[Arc::new(g)]), 1.0);
}

#[test]
fn goggle_path_rules_boost_matching_urls() {
    let mut g = rules();
    g.path.push(("a.xyz/blog".into(), 2.0));
    let f = goggle(base("http://a.xyz/blog"), &[Arc::new(g.clone())]);
    assert_factor(f, 1.3);
    let f = goggle(base("http://a.xyz/shop"), &[Arc::new(g)]);
    assert_factor(f, 1.0);
}

/* ───────────────────────────── logits ───────────────────────────── */

fn logit_score(d: WebDocument, goggles: &[Arc<GoggleRules>]) -> f64 {
    init();
    let mut v = vec![d];
    hand::run_logits(
        &mut v,
        "q",
        "en",
        "zz",
        &QueryIntent::Unknown,
        goggles,
        false,
    );
    v[0].search_score as f64
}

#[test]
fn logits_baseline_is_unchanged() {
    let mut d = base("http://a.xyz");
    d.search_score = 3.0;
    assert!((logit_score(d, &[]) - 3.0).abs() < 1e-6);
}

#[test]
fn logits_add_log_of_bounded_quality() {
    let mut d = base("https://a.com/");
    d.search_score = 3.0;
    let expected = 3.0 + (1.0 + hand::QUALITY_BOUND).ln();
    assert!((logit_score(d, &[]) - expected).abs() < 1e-5);
}

#[test]
fn logits_work_for_negative_scores() {
    let mut d = base("http://a.xyz");
    d.search_score = -4.0;
    d.load = 10.0;
    let expected = -4.0 - (1.0 + hand::QUALITY_BOUND).ln();
    assert!((logit_score(d, &[]) - expected).abs() < 1e-5);
}

#[test]
fn logits_add_log_of_goggles() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g = rules();
    g.downrank.insert(id, 3.0);
    let mut d = base("http://a.xyz");
    d.search_score = 2.0;
    let expected = 2.0 + 0.25f64.ln();
    assert!((logit_score(d, &[Arc::new(g)]) - expected).abs() < 1e-5);
}

#[test]
fn logits_quality_only_breaks_near_ties() {
    init();
    let good_site = {
        let mut d = base("https://a.com");
        d.search_score = 1.0;
        d
    };
    let relevant = {
        let mut d = base("http://b.xyz/x/y/z");
        d.search_score = 2.0;
        d
    };
    let mut v = vec![good_site, relevant];
    hand::run_logits(&mut v, "q", "en", "zz", &QueryIntent::Unknown, &[], false);
    assert_eq!(v[0].url, "http://b.xyz/x/y/z");
}
