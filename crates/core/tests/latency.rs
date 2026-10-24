//! Tests for latency histogram math and the Stage enum (crates/core/src/globals.rs)

use std::collections::{BTreeMap, HashSet};

use prieco_core::{LatencySummary, Stage, lat_bucket_ms, percentile, summarize};

fn hist(pairs: &[(u16, u64)]) -> BTreeMap<u16, u64> {
    pairs.iter().copied().collect()
}

/* ───────────────────────────── Stage ───────────────────────────── */

#[test]
fn stage_all_contains_every_stage_once() {
    assert_eq!(Stage::ALL.len(), 7);
    for (i, a) in Stage::ALL.iter().enumerate() {
        for (j, b) in Stage::ALL.iter().enumerate() {
            assert_eq!(i == j, a == b);
        }
    }
}

#[test]
fn stage_keys_are_unique_non_empty_and_colon_free() {
    // Keys are embedded into "lat:{date}:{key}:{bucket}" analytics keys,
    // a ':' inside a key would corrupt the prefix scans.
    let keys: HashSet<&str> = Stage::ALL.iter().map(|s| s.key()).collect();
    assert_eq!(keys.len(), Stage::ALL.len());
    for s in Stage::ALL {
        assert!(!s.key().is_empty());
        assert!(!s.key().contains(':'), "{:?}", s);
        assert!(!s.key().contains(char::is_whitespace), "{:?}", s);
    }
}

#[test]
fn stage_specific_keys() {
    assert_eq!(Stage::Core.key(), "core");
    assert_eq!(Stage::Fts.key(), "fts");
    assert_eq!(Stage::Vector.key(), "ivf");
    assert_eq!(Stage::Dir.key(), "dir");
    assert_eq!(Stage::Fetch.key(), "fetch");
    assert_eq!(Stage::PeerServe.key(), "peer_serve");
    assert_eq!(Stage::PeerRtt.key(), "peer_rtt");
}

#[test]
fn stage_no_key_is_prefix_of_another_with_colon() {
    // day_hist scans prefix "lat:{date}:{key}:" so "peer" vs "peer_serve" must not clash
    for a in Stage::ALL {
        for b in Stage::ALL {
            if a != b {
                assert!(!format!("{}:", b.key()).starts_with(&format!("{}:", a.key())));
            }
        }
    }
}

#[test]
fn stage_labels_and_descriptions_are_unique_and_non_empty() {
    let labels: HashSet<&str> = Stage::ALL.iter().map(|s| s.label()).collect();
    let descs: HashSet<&str> = Stage::ALL.iter().map(|s| s.desc()).collect();
    assert_eq!(labels.len(), 7);
    assert_eq!(descs.len(), 7);
    for s in Stage::ALL {
        assert!(!s.label().trim().is_empty());
        assert!(!s.desc().trim().is_empty());
    }
    assert_eq!(Stage::Core.label(), "End to end");
}

#[test]
fn stage_is_copy() {
    let a = Stage::Fetch;
    let b = a;
    assert_eq!(a, b);
}

/* ───────────────────────────── lat_bucket_ms ───────────────────────────── */

#[test]
fn bucket_ms_zero_is_geometric_middle_of_first_bucket() {
    let expected = (0.5f64 / 16.0).exp() / 1000.0;
    assert!((lat_bucket_ms(0) - expected).abs() < 1e-15);
}

#[test]
fn bucket_ms_is_strictly_increasing() {
    let mut prev = lat_bucket_ms(0);
    for b in 1..=400u16 {
        let cur = lat_bucket_ms(b);
        assert!(cur > prev, "bucket {} not increasing", b);
        prev = cur;
    }
}

#[test]
fn bucket_ms_ratio_between_neighbours_is_constant() {
    let r = (1.0f64 / 16.0).exp();
    for b in 0..400u16 {
        let ratio = lat_bucket_ms(b + 1) / lat_bucket_ms(b);
        assert!((ratio - r).abs() < 1e-9);
    }
}

#[test]
fn bucket_ms_one_millisecond_is_around_bucket_110() {
    // ln(1000us) * 16 = 110.52 -> bucket 110
    let ms = lat_bucket_ms(110);
    assert!((ms - 1.0).abs() < 0.07, "{}", ms);
}

#[test]
fn bucket_ms_max_bucket_is_finite() {
    let v = lat_bucket_ms(400);
    assert!(v.is_finite());
    assert!(v > 1_000_000.0, "bucket 400 should be > 1000s, got {}", v);
}

/* ───────────────────────────── percentile ───────────────────────────── */

#[test]
fn percentile_of_empty_hist_is_zero() {
    let h = BTreeMap::new();
    for q in [0.0, 0.5, 0.99, 1.0] {
        assert_eq!(percentile(&h, q), 0.0);
    }
}

#[test]
fn percentile_of_all_zero_counts_is_zero() {
    let h = hist(&[(1, 0), (2, 0)]);
    assert_eq!(percentile(&h, 0.5), 0.0);
}

#[test]
fn percentile_single_bucket_is_that_bucket() {
    let h = hist(&[(7, 3)]);
    for q in [0.0, 0.01, 0.5, 0.95, 0.99, 1.0] {
        assert_eq!(percentile(&h, q), lat_bucket_ms(7));
    }
}

#[test]
fn percentile_nearest_rank_method() {
    let h = hist(&[(1, 50), (2, 45), (3, 4), (4, 1)]);
    assert_eq!(percentile(&h, 0.50), lat_bucket_ms(1));
    assert_eq!(percentile(&h, 0.51), lat_bucket_ms(2));
    assert_eq!(percentile(&h, 0.95), lat_bucket_ms(2));
    assert_eq!(percentile(&h, 0.96), lat_bucket_ms(3));
    assert_eq!(percentile(&h, 0.99), lat_bucket_ms(3));
    assert_eq!(percentile(&h, 1.00), lat_bucket_ms(4));
}

#[test]
fn percentile_q_zero_returns_first_non_empty_bucket() {
    let h = hist(&[(1, 0), (5, 2), (9, 1)]);
    assert_eq!(percentile(&h, 0.0), lat_bucket_ms(5));
}

#[test]
fn percentile_skips_zero_count_buckets() {
    let h = hist(&[(1, 0), (5, 2)]);
    assert_eq!(percentile(&h, 0.5), lat_bucket_ms(5));
}

#[test]
fn percentile_out_of_range_q_is_clamped() {
    let h = hist(&[(10, 1), (20, 1)]);
    assert_eq!(percentile(&h, 5.0), lat_bucket_ms(20));
    assert_eq!(percentile(&h, -3.0), lat_bucket_ms(10));
}

#[test]
fn percentile_nan_q_does_not_panic() {
    let h = hist(&[(10, 1), (20, 1)]);
    let v = percentile(&h, f64::NAN);
    assert!(v == lat_bucket_ms(10) || v == lat_bucket_ms(20));
}

#[test]
fn percentile_is_monotonic_in_q() {
    let h = hist(&[(3, 10), (8, 7), (40, 2), (100, 1), (300, 5)]);
    let mut prev = 0.0;
    for i in 0..=100 {
        let v = percentile(&h, i as f64 / 100.0);
        assert!(v >= prev, "q={} -> {} < {}", i, v, prev);
        prev = v;
    }
}

#[test]
fn percentile_result_is_always_a_bucket_value() {
    let h = hist(&[(3, 10), (8, 7), (40, 2)]);
    let allowed = [lat_bucket_ms(3), lat_bucket_ms(8), lat_bucket_ms(40)];
    for i in 0..=20 {
        let v = percentile(&h, i as f64 / 20.0);
        assert!(allowed.contains(&v));
    }
}

/* ───────────────────────────── summarize ───────────────────────────── */

#[test]
fn summarize_empty() {
    let s = summarize(&BTreeMap::new());
    assert_eq!(s.count, 0);
    assert_eq!(s.p50, 0.0);
    assert_eq!(s.p95, 0.0);
    assert_eq!(s.p99, 0.0);
}

#[test]
fn summarize_counts_and_percentiles() {
    let h = hist(&[(1, 50), (2, 45), (3, 4), (4, 1)]);
    let s = summarize(&h);
    assert_eq!(s.count, 100);
    assert_eq!(s.p50, lat_bucket_ms(1));
    assert_eq!(s.p95, lat_bucket_ms(2));
    assert_eq!(s.p99, lat_bucket_ms(3));
}

#[test]
fn summarize_percentiles_are_ordered() {
    let h = hist(&[(5, 1), (50, 1000), (60, 30), (90, 5), (200, 1)]);
    let s = summarize(&h);
    assert!(s.p50 <= s.p95);
    assert!(s.p95 <= s.p99);
    assert_eq!(s.count, 1037);
}

#[test]
fn latency_summary_default_is_zeroed_and_copy() {
    let s = LatencySummary::default();
    let c = s;
    assert_eq!(c.count, 0);
    assert_eq!(s.p50, 0.0);
    assert_eq!(s.p95, 0.0);
    assert_eq!(s.p99, 0.0);
}
