//! Tests for src/web/functions/ranking/rrf.rs (reciprocal rank fusion)

use prieco_core::{QueryIntent, WebDocument};

use crate::support::{doc, init};
use crate::web::functions::ranking::rrf;

const K: f32 = 60.0;

fn lang_doc(url: &str, lang: &str) -> WebDocument {
    let mut d = doc(url);
    d.lang = lang.to_string();
    d
}

fn docs(urls: &[&str]) -> Vec<WebDocument> {
    urls.iter().map(|u| doc(u)).collect()
}

fn run(
    query: &str,
    dir: Vec<WebDocument>,
    fts: Vec<WebDocument>,
    ivf: Vec<WebDocument>,
    dis: Vec<WebDocument>,
    dec: Vec<WebDocument>,
) -> Vec<WebDocument> {
    init();
    rrf::run(query, "all", &QueryIntent::Unknown, dir, fts, ivf, dis, dec, K)
}

fn urls(v: &[WebDocument]) -> Vec<&str> {
    v.iter().map(|d| d.url.as_str()).collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

/// "a b c": 3 tokens, no question word, no quotes, no long tokens -> weights 1.0 / 1.0
const NEUTRAL: &str = "a b c";

/* ───────────────────────────── basics ───────────────────────────── */

#[test]
fn no_results_in_no_results_out() {
    assert!(run(NEUTRAL, vec![], vec![], vec![], vec![], vec![]).is_empty());
}

#[test]
fn single_source_keeps_rank_order() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["https://1.com", "https://2.com", "https://3.com"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(urls(&out), vec!["https://1.com", "https://2.com", "https://3.com"]);
}

#[test]
fn scores_follow_rrf_formula() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["https://1.com", "https://2.com"]),
        vec![],
        vec![],
        vec![],
    );
    assert!(close(out[0].search_score, 1.0 / (K + 1.0)));
    assert!(close(out[1].search_score, 1.0 / (K + 2.0)));
}

#[test]
fn source_is_recorded() {
    let out = run(
        NEUTRAL,
        docs(&["https://dir.com"]),
        docs(&["https://fts.com"]),
        docs(&["https://ivf.com"]),
        docs(&["https://dis.com"]),
        docs(&["https://dec.com"]),
    );
    let src = |u: &str| out.iter().find(|d| d.url == u).unwrap().source.clone();
    assert_eq!(src("https://dir.com"), "DIR");
    assert_eq!(src("https://fts.com"), "FTS");
    assert_eq!(src("https://ivf.com"), "IVF");
    assert_eq!(src("https://dis.com"), "DIS");
    assert_eq!(src("https://dec.com"), "DEC");
}

#[test]
fn incoming_source_field_is_overwritten() {
    let mut d = doc("https://x.com");
    d.source = "SPOOFED".into();
    let out = run(NEUTRAL, vec![], vec![d], vec![], vec![], vec![]);
    assert_eq!(out[0].source, "FTS");
}

#[test]
fn source_weights_order_first_ranked_results() {
    // DIR 3.0 > FTS 1.0 == IVF 1.0 > FED 0.9 > DIS 0.5 for rank 0
    let out = run(
        NEUTRAL,
        docs(&["https://dir.com"]),
        docs(&["https://fts.com"]),
        vec![],
        docs(&["https://dis.com"]),
        docs(&["https://dec.com"]),
    );
    assert_eq!(
        urls(&out),
        vec!["https://dir.com", "https://fts.com", "https://dec.com", "https://dis.com"]
    );
    assert!(close(out[0].search_score, 3.0 / (K + 1.0)));
    assert!(close(out[2].search_score, 0.9 / (K + 1.0)));
    assert!(close(out[3].search_score, 0.5 / (K + 1.0)));
}

#[test]
fn same_url_from_several_sources_is_merged_and_summed() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["https://x.com"]),
        docs(&["https://y.com", "https://x.com"]),
        vec![],
        vec![],
    );
    assert_eq!(out.len(), 2);
    let x = out.iter().find(|d| d.url == "https://x.com").unwrap();
    assert!(close(x.search_score, 1.0 / (K + 1.0) + 1.0 / (K + 2.0)));
    assert_eq!(x.source, "FTS + IVF");
    assert_eq!(out[0].url, "https://x.com");
}

#[test]
fn source_chain_lists_each_engine_once_in_order() {
    let all = || docs(&["https://x.com"]);
    let out = run(NEUTRAL, all(), all(), all(), all(), all());
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].source, "DIR + FTS + IVF + DIS + DEC");
}

#[test]
fn duplicate_inside_one_source_adds_score_but_not_source() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["https://x.com", "https://x.com"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].source, "FTS");
    assert!(close(out[0].search_score, 1.0 / (K + 1.0) + 1.0 / (K + 2.0)));
}

#[test]
fn k_zero_gives_inverse_rank() {
    init();
    let out = rrf::run(
        NEUTRAL,
        "all",
        &QueryIntent::Unknown,
        vec![],
        docs(&["https://1.com", "https://2.com", "https://3.com"]),
        vec![],
        vec![],
        vec![],
        0.0,
    );
    assert!(close(out[0].search_score, 1.0));
    assert!(close(out[1].search_score, 0.5));
    assert!(close(out[2].search_score, 1.0 / 3.0));
}

#[test]
fn result_is_sorted_descending() {
    let out = run(
        NEUTRAL,
        docs(&["https://d1.com", "https://d2.com"]),
        docs(&["https://f1.com", "https://f2.com", "https://f3.com"]),
        docs(&["https://i1.com", "https://f3.com"]),
        docs(&["https://s1.com"]),
        docs(&["https://e1.com"]),
    );
    for w in out.windows(2) {
        assert!(w[0].search_score >= w[1].search_score);
    }
}

#[test]
fn many_results() {
    let fts: Vec<WebDocument> = (0..500).map(|i| doc(&format!("https://f{}.com", i))).collect();
    let ivf: Vec<WebDocument> = (0..500).map(|i| doc(&format!("https://f{}.com", 499 - i))).collect();
    let out = run(NEUTRAL, vec![], fts, ivf, vec![], vec![]);
    assert_eq!(out.len(), 500);
    assert!(out.iter().all(|d| d.source == "FTS + IVF"));
}

/* ───────────────────────────── weight heuristics ───────────────────────────── */

fn fts_ivf_scores(query: &str) -> (f32, f32) {
    let out = run(
        query,
        vec![],
        docs(&["https://fts.com"]),
        docs(&["https://ivf.com"]),
        vec![],
        vec![],
    );
    let get = |u: &str| out.iter().find(|d| d.url == u).unwrap().search_score;
    (get("https://fts.com") * (K + 1.0), get("https://ivf.com") * (K + 1.0))
}

#[test]
fn neutral_query_has_equal_weights() {
    let (f, i) = fts_ivf_scores(NEUTRAL);
    assert!(close(f, 1.0) && close(i, 1.0), "{} {}", f, i);
}

#[test]
fn short_query_boosts_full_text() {
    let (f, i) = fts_ivf_scores("rust");
    assert!(close(f, 1.7) && close(i, 1.0), "{} {}", f, i);
    let (f, _) = fts_ivf_scores("rust lang");
    assert!(close(f, 1.7));
}

#[test]
fn long_query_boosts_vector() {
    let (f, i) = fts_ivf_scores("a b c d e f");
    assert!(close(f, 1.0) && close(i, 1.7), "{} {}", f, i);
}

#[test]
fn question_word_boosts_vector() {
    let (f, i) = fts_ivf_scores("what rust does");
    assert!(close(f, 1.0) && close(i, 1.6), "{} {}", f, i);
    let (_, i) = fts_ivf_scores("WHY rust is");
    assert!(close(i, 1.6), "question words are case insensitive");
}

#[test]
fn quotes_boost_full_text() {
    let (f, _) = fts_ivf_scores("\"exact phrase\" here");
    assert!(close(f, 1.8), "{}", f);
}

#[test]
fn rare_long_tokens_boost_full_text() {
    let (f, _) = fts_ivf_scores("abcdefgh ijklmnop x");
    assert!(close(f, 1.4), "{}", f);
}

#[test]
fn weights_are_clamped_to_two() {
    // 1 token (+0.7), quotes (+0.8), rare (+0.4) = 2.9 -> 2.0
    let (f, _) = fts_ivf_scores("\"supercalifragilistic\"");
    assert!(close(f, 2.0), "{}", f);
    // >= 6 tokens (+0.7) + question (+0.6) = 2.3 -> 2.0
    let (_, i) = fts_ivf_scores("what is the difference between these two");
    assert!(close(i, 2.0), "{}", i);
}

#[test]
fn empty_query_counts_as_short() {
    let (f, i) = fts_ivf_scores("");
    assert!(close(f, 1.7) && close(i, 1.0), "{} {}", f, i);
}

/* ───────────────────────────── vector language enforcement ───────────────────────────── */

fn ivf_score(lang: &str, doc_lang: &str, intent: QueryIntent, source: &str) -> f32 {
    init();
    let d = vec![lang_doc("https://x.com", doc_lang)];
    let (fts, ivf) = if source == "IVF" { (vec![], d) } else { (d, vec![]) };
    let out = rrf::run(NEUTRAL, lang, &intent, vec![], fts, ivf, vec![], vec![], K);
    out[0].search_score * (K + 1.0)
}

#[test]
fn foreign_vector_results_are_penalised() {
    assert!(close(ivf_score("en", "de", QueryIntent::Unknown, "IVF"), 0.1));
    assert!(close(ivf_score("en", "de", QueryIntent::Informational, "IVF"), 0.1));
}

#[test]
fn foreign_vector_results_penalised_less_for_navigational() {
    assert!(close(ivf_score("en", "de", QueryIntent::Navigational, "IVF"), 0.7));
}

#[test]
fn matching_language_is_not_penalised() {
    assert!(close(ivf_score("en", "en", QueryIntent::Unknown, "IVF"), 1.0));
}

#[test]
fn all_or_empty_language_disables_penalty() {
    assert!(close(ivf_score("all", "de", QueryIntent::Unknown, "IVF"), 1.0));
    assert!(close(ivf_score("", "de", QueryIntent::Unknown, "IVF"), 1.0));
}

#[test]
fn language_penalty_only_applies_to_vector_source() {
    assert!(close(ivf_score("en", "de", QueryIntent::Unknown, "FTS"), 1.0));
}

/* ───────────────────────────── site: / domain: filters ───────────────────────────── */

fn site_filter(query: &str, urls_in: &[&str]) -> Vec<String> {
    run(query, vec![], docs(urls_in), vec![], vec![], vec![])
        .into_iter()
        .map(|d| d.url)
        .collect()
}

const SITES: &[&str] = &[
    "https://rust-lang.org/learn",
    "https://www.rust-lang.org/",
    "https://doc.rust-lang.org/std",
    "https://evilrust-lang.org/",
    "https://rust-lang.org.evil.com/",
    "https://example.com/rust-lang.org",
];

#[test]
fn site_filter_keeps_domain_and_subdomains_only() {
    let out = site_filter("rust site:rust-lang.org", SITES);
    assert_eq!(
        out,
        vec![
            "https://rust-lang.org/learn",
            "https://www.rust-lang.org/",
            "https://doc.rust-lang.org/std",
        ]
    );
}

#[test]
fn domain_operator_is_an_alias() {
    let out = site_filter("rust domain:rust-lang.org", SITES);
    assert_eq!(out.len(), 3);
}

#[test]
fn site_operator_is_case_insensitive_and_normalised() {
    for q in [
        "x SITE:rust-lang.org",
        "x site:RUST-LANG.ORG",
        "x site:https://www.rust-lang.org/",
        "x site:http://rust-lang.org",
    ] {
        assert_eq!(site_filter(q, SITES).len(), 3, "{}", q);
    }
}

#[test]
fn multiple_site_operators_are_a_union() {
    let out = site_filter("x site:rust-lang.org site:example.com", SITES);
    assert_eq!(out.len(), 4);
    assert!(out.contains(&"https://example.com/rust-lang.org".to_string()));
}

#[test]
fn site_filter_inside_parentheses() {
    let out = site_filter("x (site:example.com)", SITES);
    assert_eq!(out, vec!["https://example.com/rust-lang.org"]);
}

#[test]
fn site_filter_with_no_match_returns_nothing() {
    assert!(site_filter("x site:nothing.test", SITES).is_empty());
}

#[test]
fn ip_urls_are_removed_by_site_filter() {
    assert!(site_filter("x site:127.0.0.1", &["http://127.0.0.1/"]).is_empty());
}

/* ───────────────────────────── http/https duplicates ───────────────────────────── */

#[test]
fn http_duplicate_of_https_is_removed() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["http://a.com/x", "https://a.com/x"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(urls(&out), vec!["https://a.com/x"]);
}

#[test]
fn http_duplicate_with_trailing_slash_difference_is_removed() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["http://a.com/x/", "https://a.com/x"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(urls(&out), vec!["https://a.com/x"]);
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["https://a.com/x/", "http://a.com/x"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(urls(&out), vec!["https://a.com/x/"]);
}

#[test]
fn http_without_https_twin_is_kept() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["http://a.com/x", "https://a.com/y"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(out.len(), 2);
}

#[test]
fn https_root_and_http_root_dedup() {
    let out = run(
        NEUTRAL,
        vec![],
        docs(&["http://a.com", "https://a.com/"]),
        vec![],
        vec![],
        vec![],
    );
    assert_eq!(urls(&out), vec!["https://a.com/"]);
}
