//! Tests for src/web/functions/ranking/hand.rs (hand tuned ranking boosts)
//!
//! A "baseline" document gets a boost of exactly 1.0; every test changes one
//! property and checks the resulting multiplier against RankingWeights::default().

use std::sync::Arc;

use prieco_core::{QueryIntent, WebDocument, url_to_domain_id};

use crate::support::{doc, init};
use crate::web::functions::ranking::{goggles::GoggleRules, hand};

const W_DOMAIN: f64 = 4.164;
const W_HOME: f64 = 1.8207127570559796;
const W_LANG: f64 = 1.01;
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

/// Multiplier applied to a single document (initial score 1.0)
fn factor_with(d: WebDocument, query: &str, lang: &str, loc: &str, intent: QueryIntent, goggles: &[Arc<GoggleRules>]) -> f64 {
    rank(vec![d], query, lang, loc, intent, goggles, false)[0].search_score as f64
}

fn factor(d: WebDocument, intent: QueryIntent) -> f64 {
    factor_with(d, "zzz", "en", "zz", intent, &[])
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
    assert_eq!(factor(d, QueryIntent::Navigational), 0.0);
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
        d.search_score = 0.5;
        d
    };
    let out = rank(vec![slow, fast], "q", "en", "zz", QueryIntent::Unknown, &[], false);
    assert_eq!(out[0].url, "http://fast.xyz");
    assert_eq!(out[1].url, "http://slow.xyz");
}

#[test]
fn nan_scores_do_not_panic_while_sorting() {
    let mut a = base("http://a.xyz");
    a.search_score = f32::NAN;
    let b = base("http://b.xyz");
    let out = rank(vec![a, b], "q", "en", "zz", QueryIntent::Unknown, &[], false);
    assert_eq!(out.len(), 2);
}

/* ───────────────────────────── url signals ───────────────────────────── */

#[test]
fn https_boost() {
    assert_factor(factor(base("https://a.xyz"), QueryIntent::Unknown), W_HTTPS);
}

#[test]
fn com_and_dev_boost() {
    assert_factor(factor(base("http://a.com"), QueryIntent::Unknown), W_DEV_COM);
    assert_factor(factor(base("http://a.dev"), QueryIntent::Unknown), W_DEV_COM);
}

#[test]
fn org_and_net_boost() {
    assert_factor(factor(base("http://a.org"), QueryIntent::Unknown), W_ORG_NET);
    assert_factor(factor(base("http://a.net"), QueryIntent::Unknown), W_ORG_NET);
}

#[test]
fn tld_matching_location_boost() {
    let d = base("http://a.xyz");
    assert_factor(factor_with(d, "zzz", "en", "XYZ", QueryIntent::Unknown, &[]), W_TLD_LOC);
}

#[test]
fn language_match_boost() {
    let mut d = base("http://a.xyz");
    d.lang = "en".into();
    assert_factor(factor(d, QueryIntent::Unknown), W_LANG);
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
    assert_factor(factor(base("http://a.xyz/b"), QueryIntent::Unknown), W_DEPTH);
    assert_factor(factor(base("http://a.xyz/b/c"), QueryIntent::Unknown), W_DEPTH.powi(2));
    assert_factor(factor(base("http://a.xyz/b/c/d/e"), QueryIntent::Unknown), W_DEPTH.powi(4));
}

#[test]
fn query_string_and_fragment_are_ignored() {
    assert_factor(factor(base("http://a.xyz?x=1/2/3"), QueryIntent::Unknown), 1.0);
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
    assert_factor(factor_with(d, "Rust", "en", "zz", QueryIntent::Unknown, &[]), W_TITLE);
}

#[test]
fn quality_score_boost_is_clamped() {
    for (qna, f) in [(600.0f32, 1.25), (1200.0, 1.25), (240.0, 1.1), (-240.0, 0.9), (-10_000.0, 0.8)] {
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
    // "http://a.xyz" is a homepage for navigational queries
    assert_factor(factor(d.clone(), QueryIntent::Navigational), W_500 * W_HOME);
    assert_factor(factor(d.clone(), QueryIntent::Informational), W_500 * 1.1);
}

#[test]
fn short_pages_penalised_for_informational_queries() {
    assert_factor(factor(base("http://a.xyz"), QueryIntent::Informational), 0.9);
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
fn navigational_domain_match_and_homepage() {
    let d = base("http://github.xyz");
    let f = factor_with(d, "GitHub", "en", "zz", QueryIntent::Navigational, &[]);
    assert_factor(f, W_DOMAIN * W_HOME);
}

#[test]
fn navigational_domain_match_ignores_www() {
    let d = base("http://www.github.xyz");
    let f = factor_with(d, "github", "en", "zz", QueryIntent::Navigational, &[]);
    assert_factor(f, W_DOMAIN * W_HOME);
}

#[test]
fn navigational_locale_paths_count_as_homepage() {
    for path in ["en", "en-ca", "pt-BR", "zh-Hans"] {
        let d = base(&format!("http://x.xyz/{}", path));
        let f = factor_with(d, "nomatch", "en", "zz", QueryIntent::Navigational, &[]);
        assert_factor(f, W_HOME * W_DEPTH);
    }
}

#[test]
fn navigational_non_locale_paths_are_not_homepage() {
    for path in ["a", "toolongsegment", "en/us", "e1"] {
        let d = base(&format!("http://x.xyz/{}", path));
        let f = factor_with(d, "nomatch", "en", "zz", QueryIntent::Navigational, &[]);
        let slashes = format!("http://x.xyz/{}", path).matches('/').count() as i32 - 2;
        assert_factor(f, W_DEPTH.powi(slashes));
    }
}

/// BUG: the "locale path" heuristic accepts any 2-7 letter segment, so ordinary
/// pages like /login, /about, /blog or /pricing get the homepage boost.
#[test]
fn navigational_regular_short_paths_are_not_homepages() {
    for path in ["login", "about", "blog", "pricing"] {
        let d = base(&format!("http://x.xyz/{}", path));
        let f = factor_with(d, "nomatch", "en", "zz", QueryIntent::Navigational, &[]);
        assert_factor(f, W_DEPTH);
    }
}

#[test]
fn navigational_homepage_beats_deep_page() {
    let home = base("https://github.com");
    let deep = {
        let mut d = base("https://github.com/some/deep/page");
        d.search_score = 3.0;
        d
    };
    let out = rank(vec![deep, home], "github", "en", "us", QueryIntent::Navigational, &[], false);
    assert_eq!(out[0].url, "https://github.com");
}

#[test]
fn commercial_product_and_shop_domains() {
    assert_factor(
        factor(base("http://a.xyz/product/1"), QueryIntent::CommercialInvestigation),
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
    let same = factor_with(base(url), "zzz", "en", "zz", QueryIntent::Unknown, &[]);
    assert_factor(same, W_WIKI * W_ORG_NET * depth);
    let other = factor_with(base(url), "zzz", "de", "zz", QueryIntent::Unknown, &[]);
    assert_factor(other, 0.1 * W_ORG_NET * depth);
    let all = factor_with(base(url), "zzz", "all", "zz", QueryIntent::Unknown, &[]);
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
    assert_factor(
        factor_with(base("http://a.xyz"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]),
        1.3,
    );

    let mut g = rules();
    g.downrank.insert(id, 1.0);
    assert_factor(
        factor_with(base("http://a.xyz"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]),
        0.5,
    );
}

#[test]
fn goggle_values_are_capped_at_three() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g = rules();
    g.boost.insert(id, 1000.0);
    assert_factor(
        factor_with(base("http://a.xyz"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]),
        1.45,
    );
    let mut g = rules();
    g.downrank.insert(id, 1000.0);
    assert_factor(
        factor_with(base("http://a.xyz"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]),
        0.25,
    );
}

#[test]
fn several_goggles_multiply() {
    let id = url_to_domain_id("http://a.xyz");
    let mut g1 = rules();
    g1.boost.insert(id, 2.0);
    let mut g2 = rules();
    g2.boost.insert(id, 2.0);
    assert_factor(
        factor_with(
            base("http://a.xyz"),
            "q",
            "en",
            "zz",
            QueryIntent::Unknown,
            &[Arc::new(g1), Arc::new(g2)],
        ),
        1.3 * 1.3,
    );
}

#[test]
fn important_goggle_replaces_all_other_signals() {
    let url = "https://a.com/x/y";
    let id = url_to_domain_id(url);
    let mut g = rules();
    g.important.insert(id);
    g.boost.insert(id, 2.0);
    let f = factor_with(base(url), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]);
    assert_factor(f, 1.3);
}

#[test]
fn important_goggle_downrank_replaces_signals() {
    let url = "https://a.com";
    let id = url_to_domain_id(url);
    let mut g = rules();
    g.important.insert(id);
    g.downrank.insert(id, 3.0);
    let f = factor_with(base(url), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]);
    assert_factor(f, 0.25);
}

#[test]
fn goggles_for_other_domains_have_no_effect() {
    let mut g = rules();
    g.boost.insert(url_to_domain_id("https://other.example"), 3.0);
    g.downrank.insert(url_to_domain_id("https://other2.example"), 3.0);
    assert_factor(
        factor_with(base("http://a.xyz"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]),
        1.0,
    );
}

#[test]
fn goggle_path_rules_boost_matching_urls() {
    let mut g = rules();
    g.path.push(("a.xyz/blog".into(), 2.0));
    let f = factor_with(base("http://a.xyz/blog"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g.clone())]);
    assert_factor(f, 1.3 * W_DEPTH);
    let f = factor_with(base("http://a.xyz/shop"), "q", "en", "zz", QueryIntent::Unknown, &[Arc::new(g)]);
    assert_factor(f, W_DEPTH);
}
