//! Tests for the mmapped PageRank reader (prieco_core::PageRank)

mod common;

use std::fs;

use common::TmpDir;
use prieco_core::{PageRank, normalize_url, url_to_id};

fn write_id_map(path: &str, pairs: &[(u64, u64)]) {
    let mut bytes = Vec::with_capacity(pairs.len() * 16);
    for (h, id) in pairs {
        bytes.extend_from_slice(&h.to_le_bytes());
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn write_scores(path: &str, scores: &[f32]) {
    let bytes: Vec<u8> = scores.iter().flat_map(|s| s.to_le_bytes()).collect();
    fs::write(path, bytes).unwrap();
}

/// id map + scores for a few urls, ids given in url order
fn build(tmp: &TmpDir, urls: &[&str], scores: &[f32]) -> PageRank {
    let mut pairs: Vec<(u64, u64)> = urls
        .iter()
        .enumerate()
        .map(|(i, u)| (url_to_id(&normalize_url(u)), i as u64))
        .collect();
    pairs.sort_unstable();
    let id_map = tmp.join_str("id_map.bin");
    let sc = tmp.join_str("scores.bin");
    write_id_map(&id_map, &pairs);
    write_scores(&sc, scores);
    PageRank::open(&id_map, &sc).unwrap()
}

#[test]
fn open_missing_id_map_is_error() {
    let tmp = TmpDir::new("pr1");
    let sc = tmp.join_str("scores.bin");
    write_scores(&sc, &[1.0]);
    assert!(PageRank::open(&tmp.join_str("missing.bin"), &sc).is_err());
}

#[test]
fn open_missing_scores_is_error() {
    let tmp = TmpDir::new("pr2");
    let id = tmp.join_str("id_map.bin");
    write_id_map(&id, &[(1, 0)]);
    assert!(PageRank::open(&id, &tmp.join_str("missing.bin")).is_err());
}

#[test]
fn pairs_exposes_whole_map() {
    let tmp = TmpDir::new("pr3");
    let id = tmp.join_str("id_map.bin");
    let sc = tmp.join_str("scores.bin");
    write_id_map(&id, &[(1, 10), (5, 11), (9, 12)]);
    write_scores(&sc, &[0.0; 13]);
    let pr = PageRank::open(&id, &sc).unwrap();
    assert_eq!(pr.pairs(), &[(1, 10), (5, 11), (9, 12)]);
}

#[test]
fn lookup_finds_existing_hashes() {
    let tmp = TmpDir::new("pr4");
    let id = tmp.join_str("id_map.bin");
    let sc = tmp.join_str("scores.bin");
    write_id_map(&id, &[(1, 10), (5, 11), (9, 12), (u64::MAX, 13)]);
    write_scores(&sc, &[0.0; 14]);
    let pr = PageRank::open(&id, &sc).unwrap();
    assert_eq!(pr.lookup(1), Some(10));
    assert_eq!(pr.lookup(5), Some(11));
    assert_eq!(pr.lookup(9), Some(12));
    assert_eq!(pr.lookup(u64::MAX), Some(13));
}

#[test]
fn lookup_missing_hashes_is_none() {
    let tmp = TmpDir::new("pr5");
    let id = tmp.join_str("id_map.bin");
    let sc = tmp.join_str("scores.bin");
    write_id_map(&id, &[(10, 0), (20, 1)]);
    write_scores(&sc, &[0.0; 2]);
    let pr = PageRank::open(&id, &sc).unwrap();
    for h in [0, 9, 11, 19, 21, u64::MAX] {
        assert_eq!(pr.lookup(h), None, "{}", h);
    }
}

#[test]
fn trailing_partial_record_is_ignored() {
    let tmp = TmpDir::new("pr6");
    let id = tmp.join_str("id_map.bin");
    let sc = tmp.join_str("scores.bin");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7u64.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&[0xAB; 9]); // garbage tail, not a full record
    fs::write(&id, bytes).unwrap();
    write_scores(&sc, &[0.25]);
    let pr = PageRank::open(&id, &sc).unwrap();
    assert_eq!(pr.pairs().len(), 1);
    assert_eq!(pr.lookup(7), Some(0));
}

#[test]
fn get_score_returns_score_of_url() {
    let tmp = TmpDir::new("pr7");
    let pr = build(
        &tmp,
        &["https://a.com", "https://b.com/x", "https://c.org"],
        &[0.5, 0.3, 0.2],
    );
    assert_eq!(pr.get_score("https://a.com"), 0.5);
    assert_eq!(pr.get_score("https://b.com/x"), 0.3);
    assert_eq!(pr.get_score("https://c.org"), 0.2);
}

#[test]
fn get_score_unknown_url_is_zero() {
    let tmp = TmpDir::new("pr8");
    let pr = build(&tmp, &["https://a.com"], &[0.9]);
    assert_eq!(pr.get_score("https://unknown.example"), 0.0);
    assert_eq!(pr.get_score(""), 0.0);
}

#[test]
fn get_score_normalizes_and_trims_url() {
    let tmp = TmpDir::new("pr9");
    let pr = build(&tmp, &["https://a.com/page"], &[0.75]);
    for variant in [
        "https://a.com/page",
        "https://a.com/page/",
        "  https://a.com/page  ",
        "https://A.COM/page",
        "https://a.com/page?utm=1",
        "https://a.com/page#frag",
        "https://a.com:443/page",
    ] {
        assert_eq!(pr.get_score(variant), 0.75, "{}", variant);
    }
}

#[test]
fn get_score_path_is_case_sensitive() {
    let tmp = TmpDir::new("pr10");
    let pr = build(&tmp, &["https://a.com/page"], &[0.75]);
    assert_eq!(pr.get_score("https://a.com/PAGE"), 0.0);
}

#[test]
fn get_score_scheme_matters() {
    let tmp = TmpDir::new("pr11");
    let pr = build(&tmp, &["https://a.com"], &[0.75]);
    assert_eq!(pr.get_score("http://a.com"), 0.0);
}

#[test]
fn get_score_bare_domain_maps_to_http() {
    let tmp = TmpDir::new("pr12");
    let pr = build(&tmp, &["http://a.com"], &[0.6]);
    assert_eq!(pr.get_score("a.com"), 0.6);
}

#[test]
fn pagerank_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PageRank>();
}

#[test]
fn pagerank_can_be_read_from_many_threads() {
    let tmp = TmpDir::new("pr13");
    let urls: Vec<String> = (0..500).map(|i| format!("https://site{}.com", i)).collect();
    let refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
    let scores: Vec<f32> = (0..500).map(|i| i as f32 / 500.0).collect();
    let pr = std::sync::Arc::new(build(&tmp, &refs, &scores));

    let handles: Vec<_> = (0..8)
        .map(|t| {
            let pr = pr.clone();
            std::thread::spawn(move || {
                for i in (t..500).step_by(8) {
                    let s = pr.get_score(&format!("https://site{}.com", i));
                    assert_eq!(s, i as f32 / 500.0);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}
