//! Tests for crates/core/src/helpers.rs

mod common;

use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use common::{TmpDir, run_with_timeout, unique};
use prieco_core::{
    acquire_file_lock, file_exists, get_dir_size, normalize_url, read_file, release_file_lock,
    url_to_domain_id, url_to_id, write_file,
};

/* ───────────────────────────── file_exists ───────────────────────────── */

#[test]
fn file_exists_true_for_existing_file() {
    let tmp = TmpDir::new("fe1");
    let p = tmp.join("a.txt");
    fs::write(&p, "x").unwrap();
    assert!(file_exists(&p));
}

#[test]
fn file_exists_false_for_missing_file() {
    let tmp = TmpDir::new("fe2");
    assert!(!file_exists(tmp.join("missing.txt")));
}

#[test]
fn file_exists_true_for_directory() {
    let tmp = TmpDir::new("fe3");
    assert!(file_exists(tmp.path()));
}

#[test]
fn file_exists_false_for_empty_path() {
    assert!(!file_exists(""));
}

#[test]
fn file_exists_accepts_str_string_and_pathbuf() {
    let tmp = TmpDir::new("fe4");
    let p = tmp.join_str("a.txt");
    fs::write(&p, "x").unwrap();
    assert!(file_exists(p.as_str()));
    assert!(file_exists(p.clone()));
    assert!(file_exists(PathBuf::from(&p)));
}

#[test]
fn file_exists_false_for_file_inside_missing_dir() {
    let tmp = TmpDir::new("fe5");
    assert!(!file_exists(tmp.join("no/such/dir/file.txt")));
}

/* ───────────────────────────── read_file ───────────────────────────── */

#[test]
fn read_file_returns_contents() {
    let tmp = TmpDir::new("rf1");
    let p = tmp.join_str("a.txt");
    fs::write(&p, "hello\nworld").unwrap();
    assert_eq!(read_file(&p), "hello\nworld");
}

#[test]
fn read_file_missing_returns_empty_string() {
    let tmp = TmpDir::new("rf2");
    assert_eq!(read_file(&tmp.join_str("missing")), "");
}

#[test]
fn read_file_directory_returns_empty_string() {
    let tmp = TmpDir::new("rf3");
    assert_eq!(read_file(&tmp.path().to_string_lossy()), "");
}

#[test]
fn read_file_empty_file_returns_empty_string() {
    let tmp = TmpDir::new("rf4");
    let p = tmp.join_str("e.txt");
    fs::write(&p, "").unwrap();
    assert_eq!(read_file(&p), "");
}

#[test]
fn read_file_invalid_utf8_returns_empty_string() {
    let tmp = TmpDir::new("rf5");
    let p = tmp.join_str("bin");
    fs::write(&p, [0xff, 0xfe, 0x00, 0xc3]).unwrap();
    assert_eq!(read_file(&p), "");
}

#[test]
fn read_file_preserves_unicode() {
    let tmp = TmpDir::new("rf6");
    let p = tmp.join_str("u.txt");
    let text = "Žltý kôň 🐴 日本語 العربية";
    fs::write(&p, text).unwrap();
    assert_eq!(read_file(&p), text);
}

#[test]
fn read_file_empty_path_returns_empty_string() {
    assert_eq!(read_file(""), "");
}

/* ───────────────────────────── write_file ───────────────────────────── */

#[test]
fn write_file_creates_file_with_content() {
    let tmp = TmpDir::new("wf1");
    let p = tmp.join_str("out.txt");
    write_file(&p, "content", false);
    assert_eq!(fs::read_to_string(&p).unwrap(), "content");
}

#[test]
fn write_file_creates_missing_parent_directories() {
    let tmp = TmpDir::new("wf2");
    let p = tmp.join_str("a/b/c/d/out.txt");
    write_file(&p, "deep", false);
    assert_eq!(fs::read_to_string(&p).unwrap(), "deep");
}

#[test]
fn write_file_overwrite_truncates_previous_longer_content() {
    let tmp = TmpDir::new("wf3");
    let p = tmp.join_str("out.txt");
    write_file(&p, "a very long first content", false);
    write_file(&p, "short", false);
    assert_eq!(fs::read_to_string(&p).unwrap(), "short");
}

#[test]
fn write_file_append_appends() {
    let tmp = TmpDir::new("wf4");
    let p = tmp.join_str("out.txt");
    write_file(&p, "one,", true);
    write_file(&p, "two,", true);
    write_file(&p, "three", true);
    assert_eq!(fs::read_to_string(&p).unwrap(), "one,two,three");
}

#[test]
fn write_file_append_on_missing_file_creates_it() {
    let tmp = TmpDir::new("wf5");
    let p = tmp.join_str("new.txt");
    write_file(&p, "x", true);
    assert_eq!(fs::read_to_string(&p).unwrap(), "x");
}

#[test]
fn write_file_overwrite_after_append_replaces_everything() {
    let tmp = TmpDir::new("wf6");
    let p = tmp.join_str("out.txt");
    write_file(&p, "aaa", true);
    write_file(&p, "bbb", true);
    write_file(&p, "c", false);
    assert_eq!(fs::read_to_string(&p).unwrap(), "c");
}

#[test]
fn write_file_accepts_bytes_vec_and_string() {
    let tmp = TmpDir::new("wf7");
    let p = tmp.join_str("out.bin");
    write_file(&p, vec![1u8, 2, 3], false);
    write_file(&p, &[4u8, 5][..], true);
    write_file(&p, String::from("6"), true);
    assert_eq!(fs::read(&p).unwrap(), vec![1, 2, 3, 4, 5, b'6']);
}

#[test]
fn write_file_empty_content_creates_empty_file() {
    let tmp = TmpDir::new("wf8");
    let p = tmp.join_str("empty.txt");
    write_file(&p, "", false);
    assert!(file_exists(&p));
    assert_eq!(fs::metadata(&p).unwrap().len(), 0);
}

#[test]
fn write_file_binary_roundtrip_all_byte_values() {
    let tmp = TmpDir::new("wf9");
    let p = tmp.join_str("all.bin");
    let data: Vec<u8> = (0..=255u8).collect();
    write_file(&p, &data, false);
    assert_eq!(fs::read(&p).unwrap(), data);
}

#[test]
fn write_file_large_content() {
    let tmp = TmpDir::new("wf10");
    let p = tmp.join_str("big.bin");
    let data = vec![7u8; 5 * 1024 * 1024];
    write_file(&p, &data, false);
    assert_eq!(fs::metadata(&p).unwrap().len(), data.len() as u64);
}

#[test]
fn write_file_concurrent_appends_lose_nothing() {
    let tmp = TmpDir::new("wf11");
    let p = Arc::new(tmp.join_str("concurrent.txt"));
    let threads: Vec<_> = (0..16)
        .map(|t| {
            let p = p.clone();
            thread::spawn(move || {
                for i in 0..50 {
                    write_file(&p, format!("{}-{}\n", t, i), true);
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let content = fs::read_to_string(&*p).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 16 * 50);
    // Every line must be intact (no interleaving inside a write)
    for line in lines {
        let (a, b) = line.split_once('-').expect("corrupted line");
        assert!(a.parse::<u32>().unwrap() < 16);
        assert!(b.parse::<u32>().unwrap() < 50);
    }
}

#[test]
fn write_file_same_path_twice_does_not_deadlock() {
    let tmp = TmpDir::new("wf12");
    let p = tmp.join_str("twice.txt");
    let done = run_with_timeout(Duration::from_secs(5), move || {
        write_file(&p, "1", false);
        write_file(&p, "2", false);
        fs::read_to_string(&p).unwrap()
    });
    assert_eq!(done.as_deref(), Some("2"));
}

/// BUG: when `OpenOptions::open` fails, `write_file` returns early WITHOUT calling
/// `release_file_lock`, so the path stays locked forever and the next writer hangs.
#[test]
fn write_file_failure_must_release_the_lock() {
    let tmp = TmpDir::new("wf13");
    // A directory can't be opened for writing -> open() fails
    let dir_path = tmp.join_str("i_am_a_directory");
    fs::create_dir_all(&dir_path).unwrap();

    write_file(&dir_path, "x", false); // fails internally, must not keep the lock

    let p = dir_path.clone();
    let finished = run_with_timeout(Duration::from_secs(5), move || {
        write_file(&p, "y", false);
    });
    assert!(
        finished.is_some(),
        "second write_file on the same path hung: lock leaked after a failed open"
    );
}

#[test]
fn write_file_does_not_touch_other_files_in_directory() {
    let tmp = TmpDir::new("wf14");
    let other = tmp.join_str("other.txt");
    fs::write(&other, "keep").unwrap();
    write_file(&tmp.join_str("target.txt"), "x", false);
    assert_eq!(fs::read_to_string(&other).unwrap(), "keep");
}

/* ───────────────────────────── file locks ───────────────────────────── */

#[test]
fn lock_acquire_release_acquire_again() {
    let path = unique("lock_a");
    let p = path.clone();
    let ok = run_with_timeout(Duration::from_secs(5), move || {
        acquire_file_lock(&p);
        release_file_lock(&p);
        acquire_file_lock(&p);
        release_file_lock(&p);
    });
    assert!(ok.is_some());
}

#[test]
fn lock_on_different_paths_do_not_block_each_other() {
    let a = unique("lock_b1");
    let b = unique("lock_b2");
    let ok = run_with_timeout(Duration::from_secs(5), move || {
        acquire_file_lock(&a);
        acquire_file_lock(&b);
        release_file_lock(&b);
        release_file_lock(&a);
    });
    assert!(ok.is_some());
}

#[test]
fn lock_blocks_second_acquirer_until_released() {
    let path = unique("lock_c");
    acquire_file_lock(&path);

    let acquired = Arc::new(AtomicBool::new(false));
    let acquired2 = acquired.clone();
    let p = path.clone();
    let handle = thread::spawn(move || {
        acquire_file_lock(&p);
        acquired2.store(true, Ordering::SeqCst);
        release_file_lock(&p);
    });

    thread::sleep(Duration::from_millis(300));
    assert!(
        !acquired.load(Ordering::SeqCst),
        "second thread acquired a held lock"
    );

    release_file_lock(&path);
    handle.join().unwrap();
    assert!(acquired.load(Ordering::SeqCst));
}

#[test]
fn lock_release_of_never_acquired_path_is_harmless() {
    let path = unique("lock_d");
    release_file_lock(&path);
    let p = path.clone();
    let ok = run_with_timeout(Duration::from_secs(5), move || {
        acquire_file_lock(&p);
        release_file_lock(&p);
    });
    assert!(ok.is_some());
}

#[test]
fn lock_provides_mutual_exclusion() {
    let path = Arc::new(unique("lock_e"));
    let inside = Arc::new(AtomicUsize::new(0));
    let violations = Arc::new(AtomicUsize::new(0));

    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (path, inside, violations) = (path.clone(), inside.clone(), violations.clone());
            thread::spawn(move || {
                for _ in 0..100 {
                    acquire_file_lock(&path);
                    if inside.fetch_add(1, Ordering::SeqCst) != 0 {
                        violations.fetch_add(1, Ordering::SeqCst);
                    }
                    thread::yield_now();
                    inside.fetch_sub(1, Ordering::SeqCst);
                    release_file_lock(&path);
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    assert_eq!(violations.load(Ordering::SeqCst), 0);
}

/// BUG (lost wake up): `release_file_lock` uses `Condvar::notify_one` while waiters for
/// *different* paths share one condvar. The single notification can wake a waiter whose
/// path is still held, and the waiter of the released path keeps sleeping forever.
#[test]
fn lock_many_waiters_on_different_paths_all_make_progress() {
    let paths = Arc::new([unique("lock_f1"), unique("lock_f2"), unique("lock_f3")]);
    let finished = run_with_timeout(Duration::from_secs(30), move || {
        let threads: Vec<_> = (0..12)
            .map(|t| {
                let paths = paths.clone();
                thread::spawn(move || {
                    for i in 0..300 {
                        let p = &paths[(t + i) % paths.len()];
                        acquire_file_lock(p);
                        thread::yield_now();
                        release_file_lock(p);
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
    });
    assert!(
        finished.is_some(),
        "threads stuck waiting: notify_one lost a wake up"
    );
}

/* ───────────────────────────── url_to_id ───────────────────────────── */

#[test]
fn url_to_id_is_deterministic() {
    for u in ["", "a", "https://example.com", "https://ŽLTÝ.sk/ä?b=c#d"] {
        assert_eq!(url_to_id(u), url_to_id(u));
    }
}

#[test]
fn url_to_id_empty_string_matches_xxh3_reference_value() {
    // XXH3_64bits("") with seed 0
    assert_eq!(url_to_id(""), 0x2D06_8005_38D3_94C2);
}

#[test]
fn url_to_id_different_inputs_differ() {
    let inputs = [
        "https://example.com",
        "https://example.com/",
        "http://example.com",
        "https://Example.com",
        "https://example.org",
        "https://example.com/a",
        "https://example.com/b",
    ];
    let ids: std::collections::HashSet<u64> = inputs.iter().map(|u| url_to_id(u)).collect();
    assert_eq!(ids.len(), inputs.len());
}

#[test]
fn url_to_id_is_case_sensitive_and_slash_sensitive() {
    assert_ne!(url_to_id("A"), url_to_id("a"));
    assert_ne!(url_to_id("https://a.com"), url_to_id("https://a.com/"));
}

#[test]
fn url_to_id_has_no_collisions_on_100k_generated_urls() {
    let mut seen = std::collections::HashSet::with_capacity(100_000);
    for i in 0..100_000u32 {
        assert!(seen.insert(url_to_id(&format!("https://site{}.com/page/{}", i % 997, i))));
    }
}

#[test]
fn url_to_id_handles_very_long_input() {
    let long = "a".repeat(1_000_000);
    assert_eq!(url_to_id(&long), url_to_id(&long.clone()));
    assert_ne!(url_to_id(&long), url_to_id(&long[1..]));
}

/* ───────────────────────────── url_to_domain_id ───────────────────────────── */

#[test]
fn domain_id_equals_hash_of_domain() {
    assert_eq!(
        url_to_domain_id("https://example.com/some/path?q=1"),
        url_to_id("example.com")
    );
}

#[test]
fn domain_id_same_for_different_paths_queries_and_schemes() {
    let base = url_to_domain_id("https://example.com/");
    for u in [
        "https://example.com",
        "http://example.com/a/b/c",
        "https://example.com/?x=1#frag",
        "https://example.com:8443/",
        "https://user:pw@example.com/",
    ] {
        assert_eq!(url_to_domain_id(u), base, "{}", u);
    }
}

#[test]
fn domain_id_host_is_case_insensitive() {
    assert_eq!(
        url_to_domain_id("https://EXAMPLE.Com/"),
        url_to_domain_id("https://example.com/")
    );
}

#[test]
fn domain_id_www_is_a_different_domain() {
    assert_ne!(
        url_to_domain_id("https://www.example.com/"),
        url_to_domain_id("https://example.com/")
    );
}

#[test]
fn domain_id_subdomains_differ() {
    assert_ne!(
        url_to_domain_id("https://a.example.com/"),
        url_to_domain_id("https://b.example.com/")
    );
}

#[test]
fn domain_id_ip_hosts_return_zero() {
    assert_eq!(url_to_domain_id("http://127.0.0.1/"), 0);
    assert_eq!(url_to_domain_id("http://[::1]/"), 0);
}

#[test]
fn domain_id_invalid_urls_return_zero() {
    for u in ["", "not a url", "example.com", "/relative/path", "http://"] {
        assert_eq!(url_to_domain_id(u), 0, "{:?}", u);
    }
}

#[test]
fn domain_id_urls_without_host_return_zero() {
    assert_eq!(url_to_domain_id("mailto:someone@example.com"), 0);
    assert_eq!(url_to_domain_id("file:///etc/hosts"), 0);
    assert_eq!(url_to_domain_id("data:text/plain,hello"), 0);
}

#[test]
fn domain_id_idn_uses_punycode() {
    assert_eq!(
        url_to_domain_id("https://bücher.de/"),
        url_to_id("xn--bcher-kva.de")
    );
}

/* ───────────────────────────── normalize_url ───────────────────────────── */

#[test]
fn normalize_adds_http_scheme_when_missing() {
    assert_eq!(normalize_url("example.com"), "http://example.com");
    assert_eq!(normalize_url("example.com/a/b"), "http://example.com/a/b");
}

#[test]
fn normalize_keeps_https_scheme() {
    assert_eq!(normalize_url("https://example.com"), "https://example.com");
}

#[test]
fn normalize_strips_query() {
    assert_eq!(
        normalize_url("https://example.com/a?b=c&d=e"),
        "https://example.com/a"
    );
}

#[test]
fn normalize_strips_fragment() {
    assert_eq!(
        normalize_url("https://example.com/a#section"),
        "https://example.com/a"
    );
}

#[test]
fn normalize_strips_query_and_fragment_on_root() {
    assert_eq!(
        normalize_url("https://example.com/?q=1#top"),
        "https://example.com"
    );
}

#[test]
fn normalize_strips_single_trailing_slash() {
    assert_eq!(normalize_url("https://example.com/"), "https://example.com");
    assert_eq!(normalize_url("https://example.com/a/"), "https://example.com/a");
}

#[test]
fn normalize_slash_and_no_slash_are_equal() {
    assert_eq!(
        normalize_url("https://example.com/path/"),
        normalize_url("https://example.com/path")
    );
}

#[test]
fn normalize_removes_only_one_trailing_slash() {
    assert_eq!(
        normalize_url("https://example.com/a//"),
        "https://example.com/a/"
    );
}

#[test]
fn normalize_lowercases_host_but_keeps_path_case() {
    assert_eq!(
        normalize_url("https://EXAMPLE.COM/Some/Path"),
        "https://example.com/Some/Path"
    );
}

#[test]
fn normalize_removes_default_ports() {
    assert_eq!(normalize_url("https://example.com:443/"), "https://example.com");
    assert_eq!(normalize_url("http://example.com:80/x"), "http://example.com/x");
}

#[test]
fn normalize_keeps_non_default_port() {
    assert_eq!(
        normalize_url("http://example.com:8080/x"),
        "http://example.com:8080/x"
    );
}

#[test]
fn normalize_resolves_dot_segments() {
    assert_eq!(
        normalize_url("https://example.com/a/../b/./c"),
        "https://example.com/b/c"
    );
}

#[test]
fn normalize_percent_encodes_spaces_in_path() {
    assert_eq!(
        normalize_url("https://example.com/a b"),
        "https://example.com/a%20b"
    );
}

#[test]
fn normalize_converts_idn_to_punycode() {
    assert_eq!(normalize_url("https://bücher.de/"), "https://xn--bcher-kva.de");
}

#[test]
fn normalize_handles_ipv6_hosts() {
    assert_eq!(
        normalize_url("http://[::1]:8080/x/"),
        "http://[::1]:8080/x"
    );
}

#[test]
fn normalize_keeps_userinfo() {
    assert_eq!(
        normalize_url("https://user:pw@example.com/"),
        "https://user:pw@example.com"
    );
}

#[test]
fn normalize_empty_string_returns_empty_string() {
    assert_eq!(normalize_url(""), "");
}

#[test]
fn normalize_unparsable_http_url_is_returned_unchanged() {
    assert_eq!(normalize_url("http://"), "http://");
    assert_eq!(normalize_url("https://exa mple.com"), "https://exa mple.com");
}

#[test]
fn normalize_is_idempotent() {
    for u in [
        "example.com",
        "https://example.com/",
        "https://EXAMPLE.com/A/?q#f",
        "http://example.com:8080/a/b/",
        "https://bücher.de/x",
    ] {
        let once = normalize_url(u);
        assert_eq!(normalize_url(&once), once, "not idempotent for {}", u);
    }
}

/// BUG: the scheme check is `starts_with("http")`, so every bare domain that starts
/// with the letters "http" (httpbin.org, httpie.io...) is never prefixed and fails
/// to parse. The same site then gets two different ids.
#[test]
fn normalize_bare_domain_starting_with_http_gets_a_scheme() {
    assert_eq!(normalize_url("httpbin.org/get"), "http://httpbin.org/get");
    assert_eq!(
        normalize_url("httpbin.org"),
        normalize_url("http://httpbin.org/")
    );
}

/// BUG: the scheme check is case sensitive, "HTTPS://" gets a second "http://" prepended.
#[test]
fn normalize_uppercase_scheme() {
    assert_eq!(normalize_url("HTTPS://EXAMPLE.COM/"), "https://example.com");
}

/* ───────────────────────────── get_dir_size ───────────────────────────── */

#[test]
fn dir_size_missing_dir_is_zero() {
    let tmp = TmpDir::new("ds1");
    assert_eq!(get_dir_size(tmp.join("nope")), 0);
}

#[test]
fn dir_size_empty_dir_is_zero() {
    let tmp = TmpDir::new("ds2");
    assert_eq!(get_dir_size(tmp.path()), 0);
}

#[test]
fn dir_size_sums_files() {
    let tmp = TmpDir::new("ds3");
    fs::write(tmp.join("a"), vec![0u8; 100]).unwrap();
    fs::write(tmp.join("b"), vec![0u8; 23]).unwrap();
    assert_eq!(get_dir_size(tmp.path()), 123);
}

#[test]
fn dir_size_is_recursive() {
    let tmp = TmpDir::new("ds4");
    fs::create_dir_all(tmp.join("x/y/z")).unwrap();
    fs::write(tmp.join("top"), vec![0u8; 10]).unwrap();
    fs::write(tmp.join("x/mid"), vec![0u8; 20]).unwrap();
    fs::write(tmp.join("x/y/z/deep"), vec![0u8; 30]).unwrap();
    assert_eq!(get_dir_size(tmp.path()), 60);
    assert_eq!(get_dir_size(tmp.join("x")), 50);
}

#[test]
fn dir_size_of_a_file_path_is_zero() {
    let tmp = TmpDir::new("ds5");
    fs::write(tmp.join("f"), vec![0u8; 10]).unwrap();
    assert_eq!(get_dir_size(tmp.join("f")), 0);
}

#[test]
fn dir_size_ignores_empty_subdirectories() {
    let tmp = TmpDir::new("ds6");
    fs::create_dir_all(tmp.join("e1/e2")).unwrap();
    fs::write(tmp.join("f"), vec![0u8; 7]).unwrap();
    assert_eq!(get_dir_size(tmp.path()), 7);
}
