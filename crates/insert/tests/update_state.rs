//! Tests for crates/insert/src/update.rs (UpdateState, UpdateEngine) and
//! crates/insert/src/updates/ping.rs (offline parts of PingDeadLinksUpdate)

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use prieco_core::WebDocument;
use prieco_insert::{
    update::{Range, Update, UpdateAction, UpdateEngine, UpdateState},
    updates::ping::PingDeadLinksUpdate,
};

struct Tmp(PathBuf);
static N: AtomicU32 = AtomicU32::new(0);
impl Tmp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "prieco_insert_state_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ranges(s: &UpdateState, id: &str) -> Vec<(u64, u64)> {
    s.finished_ranges
        .get(id)
        .map(|v| v.iter().map(|r| (r.start, r.end)).collect())
        .unwrap_or_default()
}

fn doc(url: &str) -> WebDocument {
    serde_json::from_value(serde_json::json!({
        "url": url, "title": "", "description": "", "content": "", "favicon": "",
        "image": "", "keywords": "", "safe_s": false, "lang": "", "loc": "", "html": "",
        "impressions": 0, "clicks": 0, "confidence": 0.0, "effort": 0.0, "qna": 0.0,
        "sts": 0.0, "load": 0.0, "date": 0
    }))
    .unwrap()
}

/* ───────────────────────────── add / merge ───────────────────────────── */

#[test]
fn add_first_range() {
    let mut s = UpdateState::default();
    s.add("u", 5, 10);
    assert_eq!(ranges(&s, "u"), vec![(5, 10)]);
}

#[test]
fn add_disjoint_ranges_stay_separate_and_sorted() {
    let mut s = UpdateState::default();
    s.add("u", 50, 60);
    s.add("u", 0, 10);
    s.add("u", 20, 30);
    assert_eq!(ranges(&s, "u"), vec![(0, 10), (20, 30), (50, 60)]);
}

#[test]
fn add_overlapping_ranges_merge() {
    let mut s = UpdateState::default();
    s.add("u", 0, 10);
    s.add("u", 5, 20);
    assert_eq!(ranges(&s, "u"), vec![(0, 20)]);
}

#[test]
fn add_adjacent_ranges_merge() {
    let mut s = UpdateState::default();
    s.add("u", 0, 10);
    s.add("u", 11, 20);
    assert_eq!(ranges(&s, "u"), vec![(0, 20)]);
}

#[test]
fn add_ranges_with_gap_of_one_do_not_merge() {
    let mut s = UpdateState::default();
    s.add("u", 0, 10);
    s.add("u", 12, 20);
    assert_eq!(ranges(&s, "u"), vec![(0, 10), (12, 20)]);
}

#[test]
fn add_contained_range_is_absorbed() {
    let mut s = UpdateState::default();
    s.add("u", 0, 100);
    s.add("u", 10, 20);
    assert_eq!(ranges(&s, "u"), vec![(0, 100)]);
}

#[test]
fn add_bridging_range_merges_three_into_one() {
    let mut s = UpdateState::default();
    s.add("u", 0, 10);
    s.add("u", 20, 30);
    s.add("u", 40, 50);
    s.add("u", 9, 41);
    assert_eq!(ranges(&s, "u"), vec![(0, 50)]);
}

#[test]
fn add_same_range_twice_is_idempotent() {
    let mut s = UpdateState::default();
    s.add("u", 3, 7);
    s.add("u", 3, 7);
    assert_eq!(ranges(&s, "u"), vec![(3, 7)]);
}

#[test]
fn add_single_point_ranges() {
    let mut s = UpdateState::default();
    s.add("u", 1, 1);
    s.add("u", 2, 2);
    s.add("u", 4, 4);
    assert_eq!(ranges(&s, "u"), vec![(1, 2), (4, 4)]);
}

#[test]
fn add_is_per_update_id() {
    let mut s = UpdateState::default();
    s.add("a", 0, 10);
    s.add("b", 100, 200);
    assert_eq!(ranges(&s, "a"), vec![(0, 10)]);
    assert_eq!(ranges(&s, "b"), vec![(100, 200)]);
    assert!(ranges(&s, "c").is_empty());
}

#[test]
fn add_many_random_order_ranges_converge() {
    let mut s = UpdateState::default();
    let mut order: Vec<u64> = (0..100).collect();
    // deterministic shuffle
    for i in 0..order.len() {
        let j = (i * 37 + 11) % order.len();
        order.swap(i, j);
    }
    for k in order {
        s.add("u", k * 10, k * 10 + 9);
    }
    assert_eq!(ranges(&s, "u"), vec![(0, 999)]);
}

#[test]
fn add_up_to_u64_max_does_not_overflow() {
    let mut s = UpdateState::default();
    s.add("u", 0, u64::MAX - 1);
    s.add("u", 5, 6);
    assert_eq!(ranges(&s, "u"), vec![(0, u64::MAX - 1)]);
}

/// BUG: merging computes `last.end + 1`, which overflows (panics in debug builds)
/// when a stored range ends at u64::MAX.
#[test]
fn add_after_range_ending_at_u64_max_does_not_panic() {
    let r = std::panic::catch_unwind(|| {
        let mut s = UpdateState::default();
        s.add("u", 0, u64::MAX);
        s.add("u", 5, 6);
        ranges(&s, "u")
    });
    assert_eq!(r.ok(), Some(vec![(0, u64::MAX)]));
}

#[test]
fn add_reversed_range_is_stored_as_given() {
    // Documents current behaviour: start > end is not validated
    let mut s = UpdateState::default();
    s.add("u", 10, 5);
    assert_eq!(ranges(&s, "u"), vec![(10, 5)]);
    assert!(!s.processed("u", 7));
}

/* ───────────────────────────── processed / range_end ───────────────────────────── */

#[test]
fn processed_checks_inclusive_bounds() {
    let mut s = UpdateState::default();
    s.add("u", 10, 20);
    assert!(!s.processed("u", 9));
    assert!(s.processed("u", 10));
    assert!(s.processed("u", 15));
    assert!(s.processed("u", 20));
    assert!(!s.processed("u", 21));
}

#[test]
fn processed_unknown_update_is_false() {
    let s = UpdateState::default();
    assert!(!s.processed("nope", 0));
}

#[test]
fn processed_with_multiple_ranges() {
    let mut s = UpdateState::default();
    s.add("u", 0, 5);
    s.add("u", 10, 15);
    assert!(s.processed("u", 3));
    assert!(!s.processed("u", 7));
    assert!(s.processed("u", 12));
}

#[test]
fn processed_range_end_returns_end_of_containing_range() {
    let mut s = UpdateState::default();
    s.add("u", 0, 5);
    s.add("u", 10, 15);
    assert_eq!(s.processed_range_end("u", 0), Some(5));
    assert_eq!(s.processed_range_end("u", 5), Some(5));
    assert_eq!(s.processed_range_end("u", 6), None);
    assert_eq!(s.processed_range_end("u", 12), Some(15));
    assert_eq!(s.processed_range_end("u", 16), None);
    assert_eq!(s.processed_range_end("other", 1), None);
}

/* ───────────────────────────── unprocessed_id ───────────────────────────── */

#[test]
fn unprocessed_id_without_ranges_is_zero() {
    let s = UpdateState::default();
    assert_eq!(s.unprocessed_id("u"), 0);
}

#[test]
fn unprocessed_id_after_prefix_range() {
    let mut s = UpdateState::default();
    s.add("u", 0, 99);
    assert_eq!(s.unprocessed_id("u"), 100);
}

#[test]
fn unprocessed_id_when_first_range_does_not_start_at_zero() {
    let mut s = UpdateState::default();
    s.add("u", 5, 99);
    assert_eq!(s.unprocessed_id("u"), 0);
}

#[test]
fn unprocessed_id_ignores_later_ranges() {
    let mut s = UpdateState::default();
    s.add("u", 0, 9);
    s.add("u", 20, 30);
    assert_eq!(s.unprocessed_id("u"), 10);
}

/// BUG: `first_range.end + 1` overflows when everything up to u64::MAX is processed.
#[test]
fn unprocessed_id_with_full_range_does_not_panic() {
    let r = std::panic::catch_unwind(|| {
        let mut s = UpdateState::default();
        s.add("u", 0, u64::MAX);
        s.unprocessed_id("u")
    });
    assert!(r.is_ok(), "unprocessed_id overflowed");
}

/* ───────────────────────────── finished ───────────────────────────── */

#[test]
fn finished_requires_a_ceiling() {
    let mut s = UpdateState::default();
    s.add("u", 0, 1000);
    assert!(!s.finished("u"));
}

#[test]
fn finished_when_prefix_reaches_ceiling() {
    let mut s = UpdateState::default();
    s.absolute_ceilings.insert("u".into(), 100);
    s.add("u", 0, 99);
    assert!(!s.finished("u"));
    s.add("u", 100, 100);
    assert!(s.finished("u"));
}

#[test]
fn finished_when_prefix_exceeds_ceiling() {
    let mut s = UpdateState::default();
    s.absolute_ceilings.insert("u".into(), 100);
    s.add("u", 0, 5000);
    assert!(s.finished("u"));
}

#[test]
fn not_finished_if_first_range_does_not_start_at_zero() {
    let mut s = UpdateState::default();
    s.absolute_ceilings.insert("u".into(), 100);
    s.add("u", 1, 1000);
    assert!(!s.finished("u"));
}

#[test]
fn not_finished_without_ranges() {
    let mut s = UpdateState::default();
    s.absolute_ceilings.insert("u".into(), 0);
    assert!(!s.finished("u"));
}

#[test]
fn finished_with_zero_ceiling_after_first_item() {
    let mut s = UpdateState::default();
    s.absolute_ceilings.insert("u".into(), 0);
    s.add("u", 0, 0);
    assert!(s.finished("u"));
}

/* ───────────────────────────── load / save ───────────────────────────── */

#[test]
fn load_missing_file_is_default() {
    let t = Tmp::new();
    let s = UpdateState::load(&t.0.join("missing.json"));
    assert!(s.finished_ranges.is_empty());
    assert!(s.absolute_ceilings.is_empty());
}

#[test]
fn load_corrupt_file_is_default() {
    let t = Tmp::new();
    let p = t.0.join("bad.json");
    fs::write(&p, "{not json").unwrap();
    let s = UpdateState::load(&p);
    assert!(s.finished_ranges.is_empty());
}

#[test]
fn load_wrong_shape_is_default() {
    let t = Tmp::new();
    let p = t.0.join("shape.json");
    fs::write(&p, r#"{"finished_ranges": 5, "absolute_ceilings": {}}"#).unwrap();
    assert!(UpdateState::load(&p).absolute_ceilings.is_empty());
}

#[test]
fn save_and_load_roundtrip() {
    let t = Tmp::new();
    let p = t.0.join("state.json");
    let mut s = UpdateState::default();
    s.add("a", 0, 10);
    s.add("a", 20, 30);
    s.add("b", 5, 5);
    s.absolute_ceilings.insert("a".into(), 30);
    s.absolute_ceilings.insert("b".into(), u64::MAX);
    s.save(&p);

    let l = UpdateState::load(&p);
    assert_eq!(ranges(&l, "a"), vec![(0, 10), (20, 30)]);
    assert_eq!(ranges(&l, "b"), vec![(5, 5)]);
    assert_eq!(l.absolute_ceilings["a"], 30);
    assert_eq!(l.absolute_ceilings["b"], u64::MAX);
}

#[test]
fn save_overwrites_previous_state() {
    let t = Tmp::new();
    let p = t.0.join("state.json");
    let mut s = UpdateState::default();
    s.add("a", 0, 10);
    s.save(&p);
    let s2 = UpdateState::default();
    s2.save(&p);
    assert!(UpdateState::load(&p).finished_ranges.is_empty());
}

#[test]
fn save_into_missing_directory_is_silently_ignored() {
    let t = Tmp::new();
    let p = t.0.join("no/such/dir/state.json");
    UpdateState::default().save(&p);
    assert!(!p.exists());
}

#[test]
fn saved_file_is_pretty_json() {
    let t = Tmp::new();
    let p = t.0.join("state.json");
    let mut s = UpdateState::default();
    s.add("a", 1, 2);
    s.save(&p);
    let text = fs::read_to_string(&p).unwrap();
    assert!(text.contains('\n'));
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["finished_ranges"]["a"][0]["start"], 1);
    assert_eq!(v["finished_ranges"]["a"][0]["end"], 2);
}

#[test]
fn range_serde_and_clone() {
    let r = Range { start: 1, end: 9 };
    let c = r.clone();
    let back: Range = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!((back.start, back.end), (1, 9));
}

#[test]
fn state_clone_is_independent() {
    let mut a = UpdateState::default();
    a.add("u", 0, 1);
    let mut b = a.clone();
    b.add("u", 2, 10);
    assert_eq!(ranges(&a, "u"), vec![(0, 1)]);
    assert_eq!(ranges(&b, "u"), vec![(0, 10)]);
}

/* ───────────────────────────── UpdateEngine ───────────────────────────── */

struct Noop;
impl Update for Noop {
    fn id(&self) -> &'static str {
        "noop_v1"
    }
    fn apply(&self, _: &mut WebDocument) -> UpdateAction {
        UpdateAction::Unchanged
    }
}

#[test]
fn engine_without_updates_returns_immediately() {
    // Must not touch any global storage (would start the interactive set up wizard)
    let engine = UpdateEngine::new();
    engine.run();
}

#[test]
fn engine_accepts_boxed_updates() {
    let mut engine = UpdateEngine::new();
    engine.add(Box::new(Noop));
    engine.add(Box::new(PingDeadLinksUpdate::new()));
}

#[test]
fn update_trait_object_dispatch() {
    let updates: Vec<Box<dyn Update>> = vec![Box::new(Noop), Box::new(PingDeadLinksUpdate::new())];
    let ids: Vec<&str> = updates.iter().map(|u| u.id()).collect();
    assert_eq!(ids, vec!["noop_v1", "ping_dead_links_v1"]);
    let mut d = doc("https://example.com");
    assert!(matches!(updates[0].apply(&mut d), UpdateAction::Unchanged));
}

/* ───────────────────────────── PingDeadLinksUpdate (offline) ───────────────────────────── */

#[test]
fn ping_update_id_is_stable() {
    // The id is persisted in config/index_updates.json, changing it restarts the update
    assert_eq!(PingDeadLinksUpdate::new().id(), "ping_dead_links_v1");
}

#[test]
fn ping_update_deletes_unparsable_urls_without_network() {
    let u = PingDeadLinksUpdate::new();
    for url in ["", "not a url", "example.com", "/relative"] {
        let mut d = doc(url);
        assert!(matches!(u.apply(&mut d), UpdateAction::Deleted), "{:?}", url);
    }
}

#[test]
fn ping_update_deletes_urls_without_host_without_network() {
    let u = PingDeadLinksUpdate::new();
    for url in ["mailto:someone@example.com", "data:text/plain,hi", "file:///etc/hosts"] {
        let mut d = doc(url);
        assert!(matches!(u.apply(&mut d), UpdateAction::Deleted), "{:?}", url);
    }
}

#[test]
fn ping_update_does_not_modify_document() {
    let u = PingDeadLinksUpdate::new();
    let mut d = doc("not a url");
    let _ = u.apply(&mut d);
    assert_eq!(d.url, "not a url");
}
