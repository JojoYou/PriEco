//! Tests for AnalyticsDb (crates/core/src/globals.rs)
//!
//! The whole binary runs inside a temp sandbox (see common::sandbox) so the global
//! ANALYTICS / PRIECO_META storages are created in a throw away directory.
//! Every test uses its own counter names / stages / days so tests can run in parallel.

mod common;

use std::{
    collections::BTreeMap,
    fs,
    sync::{Arc, Mutex},
    thread,
    time::Duration as StdDuration,
};

use chrono::{Duration, NaiveDate, Utc};
use common::{sandbox, unique};
use prieco_core::{ANALYTICS, PRIECO_META, Stage, lat_bucket_ms};

/// Both query tests touch the same "queries:{today}" key
static QUERY_TESTS: Mutex<()> = Mutex::new(());

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

fn day(offset_days: i64) -> String {
    (today() - Duration::days(offset_days)).to_string()
}

fn put_u64(key: &str, v: u64) {
    PRIECO_META
        .analytics_ks
        .insert(key.as_bytes(), v.to_le_bytes())
        .unwrap();
}

fn put_raw(key: &str, v: &[u8]) {
    PRIECO_META.analytics_ks.insert(key.as_bytes(), v).unwrap();
}

fn raw(key: &str) -> Option<Vec<u8>> {
    PRIECO_META
        .analytics_ks
        .get(key.as_bytes())
        .unwrap()
        .map(|s| s.to_vec())
}

fn series_values(s: &[(String, u64)]) -> Vec<u64> {
    s.iter().map(|(_, v)| *v).collect()
}

fn assert_consecutive_dates_ending_today(s: &[(String, u64)]) {
    let n = s.len() as i64;
    for (i, (date, _)) in s.iter().enumerate() {
        assert_eq!(*date, day(n - 1 - i as i64));
    }
}

/* ───────────────────────────── daily series shape ───────────────────────────── */

#[test]
fn series_with_zero_days_are_empty() {
    sandbox();
    assert!(ANALYTICS.daily_queries(0).is_empty());
    assert!(ANALYTICS.daily_visitors(0).is_empty());
    assert!(ANALYTICS.daily_api_requests(0).is_empty());
    assert!(ANALYTICS.counter_series(&unique("zero"), 0).is_empty());
    assert!(ANALYTICS.latency_daily(Stage::PeerRtt, 0).is_empty());
    assert!(ANALYTICS.latency_hist(Stage::PeerRtt, 0).is_empty());
}

#[test]
fn series_are_oldest_first_and_end_today() {
    sandbox();
    for s in [
        ANALYTICS.daily_queries(7),
        ANALYTICS.daily_visitors(7),
        ANALYTICS.daily_api_requests(7),
        ANALYTICS.counter_series(&unique("shape"), 7),
    ] {
        assert_eq!(s.len(), 7);
        assert_consecutive_dates_ending_today(&s);
    }
    let lat = ANALYTICS.latency_daily(Stage::PeerRtt, 5);
    assert_eq!(lat.len(), 5);
    assert_eq!(lat[4].0, day(0));
    assert_eq!(lat[0].0, day(4));
}

#[test]
fn unknown_counter_is_all_zeros() {
    sandbox();
    let s = ANALYTICS.counter_series(&unique("never_used"), 10);
    assert!(series_values(&s).iter().all(|v| *v == 0));
}

/* ───────────────────────────── queries ───────────────────────────── */

#[test]
fn record_query_increments_today_and_reads_past_days() {
    sandbox();
    let _guard = QUERY_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    put_u64(&format!("queries:{}", day(3)), 42);

    let before = ANALYTICS.daily_queries(1)[0].1;
    ANALYTICS.record_query();
    ANALYTICS.record_query();
    let s = ANALYTICS.daily_queries(5);
    assert_eq!(s[4].1, before + 2);
    assert_eq!(s[1].1, 42, "value injected 3 days ago");
    assert_eq!(s[1].0, day(3));
}

/// BUG: `inc` is a read-modify-write (`get_u64` then `insert`) without any lock,
/// so concurrent `record_query` calls lose increments.
#[test]
fn record_query_is_safe_under_concurrency() {
    sandbox();
    let _guard = QUERY_TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let before = ANALYTICS.daily_queries(1)[0].1;
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..250 {
                    ANALYTICS.record_query();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let after = ANALYTICS.daily_queries(1)[0].1;
    assert!(
        after >= before + 2000,
        "lost {} of 2000 increments",
        (before + 2000).saturating_sub(after)
    );
}

/* ───────────────────────────── visitors ───────────────────────────── */

#[test]
fn visitors_pageviews_countries_and_dedup() {
    sandbox();
    put_u64(&format!("visitors:{}", day(1)), 7);
    put_u64(&format!("pageviews:{}", day(1)), 9);

    let (v0, vy, p0, py) = ANALYTICS.visitor_stats();
    assert_eq!((vy, py), (7, 9));

    // first visit
    ANALYTICS.record_visitor("203.0.113.1", "UA-1", "prieco.net", Some("SK"));
    let (v, _, p, _) = ANALYTICS.visitor_stats();
    assert_eq!((v, p), (v0 + 1, p0 + 1));

    // same person again: only a pageview
    ANALYTICS.record_visitor("203.0.113.1", "UA-1", "prieco.net", Some("SK"));
    let (v, _, p, _) = ANALYTICS.visitor_stats();
    assert_eq!((v, p), (v0 + 1, p0 + 2));

    // same ip, different user agent -> new visitor
    ANALYTICS.record_visitor("203.0.113.1", "UA-2", "prieco.net", Some("us"));
    // different ip -> new visitor
    ANALYTICS.record_visitor("203.0.113.2", "UA-1", "prieco.net", Some("us"));
    // different entity (host) -> new visitor
    ANALYTICS.record_visitor("203.0.113.1", "UA-1", "other.onion", None);
    // "all" is not a country
    ANALYTICS.record_visitor("203.0.113.3", "UA-1", "prieco.net", Some("all"));

    let (v, _, p, _) = ANALYTICS.visitor_stats();
    assert_eq!(v, v0 + 5);
    assert_eq!(p, p0 + 6);
    assert_eq!(ANALYTICS.daily_visitors(1)[0].1, v);

    let countries = ANALYTICS.top_countries();
    assert_eq!(countries[0], ("us".to_string(), 2));
    assert_eq!(countries[1], ("sk".to_string(), 1), "codes are lowercased");
    assert!(!countries.iter().any(|(c, _)| c == "all"));
    assert_eq!(countries.len(), 2);

    // The daily salt was persisted in the sandbox
    let salt = fs::read_to_string("config/analytics_daily_salt.txt").unwrap();
    let (date, hex) = salt.split_once(':').unwrap();
    assert_eq!(date, day(0));
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));

    // and fingerprints are stored hashed, never as raw ips
    let seen: Vec<String> = PRIECO_META
        .analytics_ks
        .prefix(format!("seen:{}:", day(0)).as_bytes())
        .filter_map(|g| g.key().ok())
        .map(|k| String::from_utf8_lossy(&k).into_owned())
        .collect();
    assert!(seen.len() >= 5);
    for k in seen {
        assert!(!k.contains("203.0.113"), "raw ip leaked into analytics key {}", k);
    }
}

/* ───────────────────────────── api requests ───────────────────────────── */

#[test]
fn api_requests_today_and_yesterday() {
    sandbox();
    put_u64(&format!("api:total:{}", day(1)), 5);

    let (t0, y) = ANALYTICS.api_stats_today_yesterday();
    assert_eq!(y, 5);
    ANALYTICS.record_api_request();
    ANALYTICS.record_api_request();
    ANALYTICS.record_api_request();
    let (t, y) = ANALYTICS.api_stats_today_yesterday();
    assert_eq!(t, t0 + 3);
    assert_eq!(y, 5);

    let s = ANALYTICS.daily_api_requests(2);
    assert_eq!(series_values(&s), vec![5, t]);
}

/* ───────────────────────────── generic counters ───────────────────────────── */

#[test]
fn count_and_count_by_accumulate() {
    sandbox();
    let name = unique("ctr");
    ANALYTICS.count(&name);
    ANALYTICS.count(&name);
    ANALYTICS.count(&name);
    ANALYTICS.count_by(&name, 10);
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, 13);

    // Flushed values keep accumulating
    ANALYTICS.count_by(&name, 7);
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, 20);
}

#[test]
fn count_by_zero_is_a_no_op_and_writes_nothing() {
    sandbox();
    let name = unique("ctr_zero");
    ANALYTICS.count_by(&name, 0);
    ANALYTICS.flush();
    assert_eq!(raw(&format!("ctr:{}:{}", day(0), name)), None);
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, 0);
}

#[test]
fn count_by_large_values() {
    sandbox();
    let name = unique("ctr_big");
    ANALYTICS.count_by(&name, u64::MAX / 4);
    ANALYTICS.count_by(&name, u64::MAX / 4);
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, (u64::MAX / 4) * 2);
}

#[test]
fn counter_series_includes_past_days() {
    sandbox();
    let name = unique("ctr_past");
    put_u64(&format!("ctr:{}:{}", day(1), name), 4);
    put_u64(&format!("ctr:{}:{}", day(2), name), 1);
    ANALYTICS.count_by(&name, 13);
    assert_eq!(
        series_values(&ANALYTICS.counter_series(&name, 3)),
        vec![1, 4, 13]
    );
}

#[test]
fn counter_names_with_colons_and_unicode() {
    sandbox();
    let a = format!("{}:with:colons", unique("c"));
    let b = format!("{}_žltý_🐴", unique("c"));
    ANALYTICS.count(&a);
    ANALYTICS.count_by(&b, 2);
    assert_eq!(ANALYTICS.counter_series(&a, 1)[0].1, 1);
    assert_eq!(ANALYTICS.counter_series(&b, 1)[0].1, 2);
}

#[test]
fn corrupted_counter_value_reads_as_zero() {
    sandbox();
    let name = unique("ctr_corrupt");
    put_raw(&format!("ctr:{}:{}", day(0), name), &[1, 2, 3]);
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, 0);
}

#[test]
fn counters_are_exact_under_concurrency() {
    sandbox();
    let name = Arc::new(unique("ctr_conc"));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let name = name.clone();
            thread::spawn(move || {
                for i in 0..200 {
                    ANALYTICS.count(&name);
                    if i % 50 == 0 {
                        ANALYTICS.flush();
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(ANALYTICS.counter_series(&name, 1)[0].1, 1600);
}

#[test]
fn flush_twice_is_harmless() {
    sandbox();
    let name = unique("ctr_flush");
    ANALYTICS.count(&name);
    ANALYTICS.flush();
    ANALYTICS.flush();
    assert_eq!(raw(&format!("ctr:{}:{}", day(0), name)), Some(1u64.to_le_bytes().to_vec()));
}

/* ───────────────────────────── worker batches ───────────────────────────── */

#[test]
fn worker_batches_count_pages_bytes_and_unique_workers() {
    sandbox();
    let get = |n: &str| ANALYTICS.counter_series(n, 1)[0].1;
    let (b0, p0, by0, w0) = (
        get("worker_batches"),
        get("worker_pages"),
        get("worker_bytes"),
        get("workers"),
    );

    ANALYTICS.record_worker_batch("198.51.100.1", 10, 1000);
    assert_eq!(get("worker_batches"), b0 + 1);
    assert_eq!(get("worker_pages"), p0 + 10);
    assert_eq!(get("worker_bytes"), by0 + 1000);
    assert_eq!(get("workers"), w0 + 1);

    // same worker again: not a new contributor
    ANALYTICS.record_worker_batch("198.51.100.1", 5, 0);
    assert_eq!(get("worker_batches"), b0 + 2);
    assert_eq!(get("worker_pages"), p0 + 15);
    assert_eq!(get("worker_bytes"), by0 + 1000);
    assert_eq!(get("workers"), w0 + 1);

    // a new worker
    ANALYTICS.record_worker_batch("198.51.100.2", 0, 0);
    assert_eq!(get("worker_batches"), b0 + 3);
    assert_eq!(get("worker_pages"), p0 + 15);
    assert_eq!(get("workers"), w0 + 2);
}

/* ───────────────────────────── latency ───────────────────────────── */

#[test]
fn latency_one_millisecond_goes_to_bucket_110() {
    sandbox();
    for _ in 0..5 {
        ANALYTICS.record_latency(Stage::Fts, StdDuration::from_millis(1));
    }
    let h = ANALYTICS.latency_hist(Stage::Fts, 1);
    assert_eq!(h, BTreeMap::from([(110u16, 5u64)]));
}

#[test]
fn latency_zero_duration_goes_to_bucket_zero() {
    sandbox();
    ANALYTICS.record_latency(Stage::Vector, StdDuration::ZERO);
    ANALYTICS.record_latency(Stage::Vector, StdDuration::from_nanos(999));
    let h = ANALYTICS.latency_hist(Stage::Vector, 1);
    assert_eq!(h, BTreeMap::from([(0u16, 2u64)]));
}

#[test]
fn latency_huge_durations_are_capped_at_bucket_400() {
    sandbox();
    ANALYTICS.record_latency(Stage::Dir, StdDuration::from_secs(10_000_000));
    ANALYTICS.record_latency(Stage::Dir, StdDuration::from_secs(u32::MAX as u64));
    let h = ANALYTICS.latency_hist(Stage::Dir, 1);
    assert_eq!(h, BTreeMap::from([(400u16, 2u64)]));
}

#[test]
fn latency_summary_percentiles() {
    sandbox();
    for _ in 0..99 {
        ANALYTICS.record_latency(Stage::Fetch, StdDuration::from_millis(1));
    }
    ANALYTICS.record_latency(Stage::Fetch, StdDuration::from_secs(1));
    let s = ANALYTICS.latency_summary(Stage::Fetch, 1);
    assert_eq!(s.count, 100);
    assert_eq!(s.p50, lat_bucket_ms(110));
    assert_eq!(s.p95, lat_bucket_ms(110));
    assert_eq!(s.p99, lat_bucket_ms(110));
    let h = ANALYTICS.latency_hist(Stage::Fetch, 1);
    assert_eq!(h.get(&221), Some(&1), "1s = ln(1e6)*16 = bucket 221");
}

#[test]
fn latency_hist_merges_days_and_daily_splits_them() {
    sandbox();
    put_u64(&format!("lat:{}:{}:050", day(2), Stage::PeerServe.key()), 3);
    ANALYTICS.record_latency(Stage::PeerServe, StdDuration::from_millis(1));

    assert_eq!(
        ANALYTICS.latency_hist(Stage::PeerServe, 3),
        BTreeMap::from([(50u16, 3u64), (110u16, 1u64)])
    );
    assert_eq!(
        ANALYTICS.latency_hist(Stage::PeerServe, 1),
        BTreeMap::from([(110u16, 1u64)])
    );

    let daily = ANALYTICS.latency_daily(Stage::PeerServe, 3);
    assert_eq!(daily[0].1.count, 3);
    assert_eq!(daily[0].1.p50, lat_bucket_ms(50));
    assert_eq!(daily[1].1.count, 0);
    assert_eq!(daily[2].1.count, 1);
}

#[test]
fn latency_ignores_unparsable_buckets_and_reads_corrupt_counts_as_zero() {
    sandbox();
    let k = Stage::Core.key();
    put_u64(&format!("lat:{}:{}:abc", day(0), k), 5);
    put_u64(&format!("lat:{}:{}:007", day(0), k), 2);
    put_raw(&format!("lat:{}:{}:008", day(0), k), &[9, 9]);
    let h = ANALYTICS.latency_hist(Stage::Core, 1);
    assert_eq!(h, BTreeMap::from([(7u16, 2u64), (8u16, 0u64)]));
}

#[test]
fn latency_stages_do_not_leak_into_each_other() {
    sandbox();
    // PeerRtt is never recorded by any test in this file
    assert!(ANALYTICS.latency_hist(Stage::PeerRtt, 30).is_empty());
    assert_eq!(ANALYTICS.latency_summary(Stage::PeerRtt, 30).count, 0);
}

/* ───────────────────────────── purge ───────────────────────────── */

#[test]
fn purge_removes_only_entries_older_than_30_days() {
    sandbox();
    let tag = unique("purge");
    let old = [
        "queries:2000-01-01".to_string(),
        "visitors:2000-01-01".to_string(),
        "pageviews:2000-01-01".to_string(),
        "country:2000-01-01:sk".to_string(),
        format!("seen:2000-01-01:{}", tag),
        "api:total:2000-01-01".to_string(),
        "lat:2000-01-01:core:010".to_string(),
        format!("ctr:2000-01-01:{}", tag),
        format!("wseen:2000-01-01:{}", tag),
        format!("ctr:{}:{}", day(31), tag),
    ];
    let keep = [
        format!("ctr:{}:{}", day(30), tag),
        format!("ctr:{}:{}", day(29), tag),
        format!("ctr:{}:{}", day(0), tag),
        format!("ctr:notadate:{}", tag),
        format!("unrelated:2000-01-01:{}", tag),
        format!("ctr:{}:{}", (today() + Duration::days(5)), tag),
    ];
    for k in old.iter().chain(keep.iter()) {
        put_u64(k, 1);
    }

    ANALYTICS.purge_expired();

    for k in &old {
        assert_eq!(raw(k), None, "{} should be purged", k);
    }
    for k in &keep {
        assert!(raw(k).is_some(), "{} should be kept", k);
    }
}

#[test]
fn purge_on_clean_database_is_harmless() {
    sandbox();
    ANALYTICS.purge_expired();
    ANALYTICS.purge_expired();
}
