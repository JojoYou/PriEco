//! Tests for src/web/functions/ranking/goggles/* (parse, storage, resolve, fetch safety)

use std::sync::Arc;

use prieco_core::{url_to_domain_id, url_to_id};
use rocket::{get, http::{Cookie, CookieJar}, routes};

use crate::support::{client, http, init, serve, serve_on, unique_u64};
use crate::web::functions::ranking::goggles::{
    Goggle, GoggleRules, delete, domain_str_to_id, fetch_and_store, get, get_goggle_ids,
    list_all, list_public, load_goggles, parse_goggle, put, touch_fetched_at,
};

/* ───────────────────────────── domain_str_to_id ───────────────────────────── */

#[test]
fn domain_str_to_id_matches_url_domain_id() {
    assert_eq!(domain_str_to_id("example.com"), url_to_domain_id("https://example.com/"));
    assert_eq!(domain_str_to_id("example.com"), url_to_id("example.com"));
    assert_eq!(domain_str_to_id("EXAMPLE.com"), domain_str_to_id("example.com"));
}

#[test]
fn domain_str_to_id_www_is_distinct() {
    assert_ne!(domain_str_to_id("www.example.com"), domain_str_to_id("example.com"));
}

#[test]
fn domain_str_to_id_invalid_is_zero() {
    assert_eq!(domain_str_to_id(""), 0);
    assert_eq!(domain_str_to_id("exa mple.com"), 0);
    assert_eq!(domain_str_to_id("127.0.0.1"), 0);
}

/* ───────────────────────────── parse_goggle: meta ───────────────────────────── */

#[test]
fn parse_empty_goggle() {
    let m = parse_goggle("");
    assert_eq!(m.name, "");
    assert_eq!(m.description, "");
    assert_eq!(m.author, "");
    assert_eq!(m.avatar, "");
    assert!(!m.public);
    assert!(m.rules.boost.is_empty());
    assert!(m.rules.downrank.is_empty());
    assert!(m.rules.discard.is_empty());
    assert!(m.rules.important.is_empty());
    assert!(m.rules.path.is_empty());
    assert!(!m.rules.discard_by_default);
}

#[test]
fn parse_meta_lines() {
    let m = parse_goggle(
        "! name: Tech News\n!description: Only tech\n!  author :  Roman \n! public: true\n! avatar: #ff00ff\n",
    );
    assert_eq!(m.name, "Tech News");
    assert_eq!(m.description, "Only tech");
    assert_eq!(m.author, "Roman");
    assert!(m.public);
    assert_eq!(m.avatar, "#ff00ff");
}

#[test]
fn parse_meta_value_may_contain_colons() {
    let m = parse_goggle("! description: see: https://example.com");
    assert_eq!(m.description, "see: https://example.com");
}

#[test]
fn parse_public_is_exactly_true() {
    for v in ["false", "TRUE", "True", "yes", "1", ""] {
        assert!(!parse_goggle(&format!("! public: {}", v)).public, "{:?}", v);
    }
}

#[test]
fn parse_unknown_and_malformed_meta_is_ignored() {
    let m = parse_goggle("! homepage: x\n! no colon here\n!\n! : value");
    assert_eq!(m.name, "");
}

#[test]
fn parse_later_meta_overrides_earlier() {
    let m = parse_goggle("! name: A\n! name: B");
    assert_eq!(m.name, "B");
}

#[test]
fn parse_handles_crlf_and_indentation() {
    let m = parse_goggle("   ! name: X\r\n\t$boost=2,site=a.com\r\n");
    assert_eq!(m.name, "X");
    assert_eq!(m.rules.boost.get(&domain_str_to_id("a.com")), Some(&2.0));
}

/* ───────────────────────────── parse_goggle: rules ───────────────────────────── */

#[test]
fn parse_discard_by_default() {
    assert!(parse_goggle("$discard").rules.discard_by_default);
    assert!(parse_goggle("  $discard  ").rules.discard_by_default);
    assert!(!parse_goggle("$discard,site=a.com").rules.discard_by_default);
}

#[test]
fn parse_boost_rule() {
    let r = parse_goggle("$boost=2,site=example.com").rules;
    assert_eq!(r.boost.get(&domain_str_to_id("example.com")), Some(&2.0));
    assert!(r.downrank.is_empty() && r.discard.is_empty());
}

#[test]
fn parse_modifier_order_does_not_matter() {
    let a = parse_goggle("$site=example.com,boost=3").rules;
    let b = parse_goggle("$boost=3,site=example.com").rules;
    assert_eq!(a.boost, b.boost);
}

#[test]
fn parse_fractional_and_negative_boost() {
    let r = parse_goggle("$boost=1.5,site=a.com\n$boost=-2,site=b.com").rules;
    assert_eq!(r.boost[&domain_str_to_id("a.com")], 1.5);
    assert_eq!(r.boost[&domain_str_to_id("b.com")], -2.0);
}

#[test]
fn parse_boost_without_or_with_invalid_value_is_ignored() {
    let r = parse_goggle("$boost,site=a.com\n$boost=abc,site=b.com\n$boost=,site=c.com").rules;
    assert!(r.boost.is_empty());
}

#[test]
fn parse_downrank_with_and_without_value() {
    let r = parse_goggle("$downrank=4,site=a.com\n$downrank,site=b.com\n$downrank=x,site=c.com").rules;
    assert_eq!(r.downrank[&domain_str_to_id("a.com")], 4.0);
    assert_eq!(r.downrank[&domain_str_to_id("b.com")], 1.0);
    assert_eq!(r.downrank[&domain_str_to_id("c.com")], 1.0);
    assert!(r.boost.is_empty());
}

#[test]
fn parse_discard_rule() {
    let r = parse_goggle("$discard,site=spam.com").rules;
    assert!(r.discard.contains(&domain_str_to_id("spam.com")));
    assert!(r.boost.is_empty() && r.downrank.is_empty());
}

#[test]
fn parse_discard_wins_over_downrank_and_boost() {
    let r = parse_goggle("$discard,downrank=2,boost=3,site=spam.com").rules;
    let id = domain_str_to_id("spam.com");
    assert!(r.discard.contains(&id));
    assert!(!r.downrank.contains_key(&id));
    assert!(!r.boost.contains_key(&id));
}

#[test]
fn parse_downrank_wins_over_boost() {
    let r = parse_goggle("$boost=3,downrank=2,site=a.com").rules;
    let id = domain_str_to_id("a.com");
    assert_eq!(r.downrank.get(&id), Some(&2.0));
    assert!(!r.boost.contains_key(&id));
}

#[test]
fn parse_important_flag() {
    let r = parse_goggle("$important,boost=2,site=a.com\n$important,site=b.com").rules;
    assert!(r.important.contains(&domain_str_to_id("a.com")));
    assert!(r.important.contains(&domain_str_to_id("b.com")));
    assert_eq!(r.boost.len(), 1);
}

#[test]
fn parse_rules_without_site_are_ignored() {
    let r = parse_goggle("$boost=2\n$discard,important\n/blog/$boost=2\nno dollar sign").rules;
    assert!(r.boost.is_empty());
    assert!(r.discard.is_empty());
    assert!(r.important.is_empty());
    assert!(r.path.is_empty());
}

#[test]
fn parse_last_rule_for_same_site_wins() {
    let r = parse_goggle("$boost=1,site=a.com\n$boost=3,site=a.com").rules;
    assert_eq!(r.boost[&domain_str_to_id("a.com")], 3.0);
}

#[test]
fn parse_many_sites() {
    let text: String = (0..1000).map(|i| format!("$boost=1,site=s{}.com\n", i)).collect();
    assert_eq!(parse_goggle(&text).rules.boost.len(), 1000);
}

#[test]
fn parse_unicode_does_not_panic() {
    let m = parse_goggle("! name: Žltý 🐴\n$boost=2,site=bücher.de\n$💥\n!:");
    assert_eq!(m.name, "Žltý 🐴");
    assert!(m.rules.boost.contains_key(&domain_str_to_id("xn--bcher-kva.de")));
}

/// GAP: Brave-style path rules (`/blog/$boost=2`) are documented in GoggleRules.path
/// and used by hand ranking, but the parser never fills `rules.path`.
#[test]
fn parse_path_rules_are_supported() {
    let r = parse_goggle("/blog/$boost=2").rules;
    assert_eq!(r.path.len(), 1, "path rules are never parsed");
}

/// The quick tune export format must round trip through the parser
#[test]
fn parse_quick_tune_export_format() {
    let text = "! name: My PriEco Goggle\n! description: Exported Goggle\n! public: false\n\n\
                $boost=3,site=a.com\n$downrank=3,site=b.com\n$discard,site=c.com\n";
    let m = parse_goggle(text);
    assert_eq!(m.name, "My PriEco Goggle");
    assert!(!m.public);
    assert_eq!(m.rules.boost[&domain_str_to_id("a.com")], 3.0);
    assert_eq!(m.rules.downrank[&domain_str_to_id("b.com")], 3.0);
    assert!(m.rules.discard.contains(&domain_str_to_id("c.com")));
}

/* ───────────────────────────── types ───────────────────────────── */

#[test]
fn goggle_default_and_serde_roundtrip() {
    let d = Goggle::default();
    assert_eq!(d.id, 0);
    assert!(d.url.is_empty());

    let mut g = Goggle::default();
    g.id = 7;
    g.name = "n".into();
    g.rules.boost.insert(1, 2.0);
    g.rules.path.push(("/x".into(), 1.5));
    g.rules.discard_by_default = true;
    let back: Goggle = serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
    assert_eq!(back.id, 7);
    assert_eq!(back.name, "n");
    assert_eq!(back.rules.boost[&1], 2.0);
    assert_eq!(back.rules.path[0], ("/x".to_string(), 1.5));
    assert!(back.rules.discard_by_default);
}

/* ───────────────────────────── storage ───────────────────────────── */

fn stored(public: bool) -> Goggle {
    let mut g = Goggle::default();
    g.id = unique_u64();
    g.name = format!("goggle {}", g.id);
    g.public = public;
    g.url = format!("https://goggles.example/{}.goggle", g.id);
    g.rules.boost.insert(42, 2.0);
    g
}

#[test]
fn storage_put_get_roundtrip() {
    init();
    let g = stored(true);
    put(&g);
    let back = get(g.id).unwrap();
    assert_eq!(back.name, g.name);
    assert_eq!(back.url, g.url);
    assert_eq!(back.rules.boost[&42], 2.0);
}

#[test]
fn storage_get_missing_is_none() {
    init();
    assert!(get(unique_u64()).is_none());
}

#[test]
fn storage_put_overwrites() {
    init();
    let mut g = stored(false);
    put(&g);
    g.name = "renamed".into();
    put(&g);
    assert_eq!(get(g.id).unwrap().name, "renamed");
}

#[test]
fn storage_delete() {
    init();
    let g = stored(true);
    put(&g);
    delete(g.id);
    assert!(get(g.id).is_none());
    delete(g.id); // deleting twice is harmless
    delete(unique_u64());
}

#[test]
fn storage_touch_updates_fetched_at_only() {
    init();
    let mut g = stored(true);
    g.fetched_at = 0;
    put(&g);
    let before = chrono::Utc::now().timestamp();
    touch_fetched_at(g.id);
    let back = get(g.id).unwrap();
    assert!(back.fetched_at >= before);
    assert_eq!(back.name, g.name);
    assert_eq!(back.rules.boost.len(), 1);
}

#[test]
fn storage_touch_missing_does_not_create() {
    init();
    let id = unique_u64();
    touch_fetched_at(id);
    assert!(get(id).is_none());
}

#[test]
fn storage_list_all_and_public() {
    init();
    let pub_g = stored(true);
    let priv_g = stored(false);
    put(&pub_g);
    put(&priv_g);
    let all: Vec<u64> = list_all().iter().map(|g| g.id).collect();
    assert!(all.contains(&pub_g.id) && all.contains(&priv_g.id));
    let public = list_public();
    assert!(public.iter().any(|g| g.id == pub_g.id));
    assert!(!public.iter().any(|g| g.id == priv_g.id));
    assert!(public.iter().all(|g| g.public));
}

#[test]
fn storage_corrupt_entries_are_skipped() {
    init();
    let id = unique_u64();
    prieco_core::PRIECO_META
        .goggles_ks
        .insert(id.to_be_bytes(), b"{not json".as_slice())
        .unwrap();
    assert!(get(id).is_none());
    let _ = list_all(); // must not panic
    delete(id);
}

#[test]
fn storage_extreme_ids() {
    init();
    for id in [0u64, u64::MAX] {
        let mut g = Goggle::default();
        g.id = id;
        g.name = format!("extreme {}", id);
        put(&g);
        assert_eq!(get(id).unwrap().name, g.name);
        delete(id);
    }
}

/* ───────────────────────────── resolve ───────────────────────────── */

#[test]
fn goggle_ids_from_param() {
    assert_eq!(get_goggle_ids(Some("1,2,3"), None), vec![1, 2, 3]);
    assert_eq!(get_goggle_ids(Some(" 1 , 2 ,x,,3 "), None), vec![1, 2, 3]);
    assert_eq!(get_goggle_ids(Some(""), None), Vec::<u64>::new());
    assert_eq!(get_goggle_ids(Some("-1,18446744073709551616"), None), Vec::<u64>::new());
    assert_eq!(get_goggle_ids(Some("18446744073709551615"), None), vec![u64::MAX]);
    assert_eq!(get_goggle_ids(None, None), Vec::<u64>::new());
}

#[get("/ids?<p>")]
fn ids_route(p: Option<&str>, jar: &CookieJar<'_>) -> String {
    format!("{:?}", get_goggle_ids(p, Some(jar)))
}

#[test]
fn goggle_ids_from_cookie_and_param_precedence() {
    let c = client(routes![ids_route]);
    let r = c
        .get("/ids")
        .cookie(Cookie::new("active_goggles", "5,6"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "[5, 6]");

    // an explicit parameter wins over the cookie, even when empty
    let r = c
        .get("/ids?p=7")
        .cookie(Cookie::new("active_goggles", "5,6"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "[7]");
    let r = c
        .get("/ids?p=")
        .cookie(Cookie::new("active_goggles", "5,6"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "[]");

    let r = c.get("/ids").dispatch();
    assert_eq!(r.into_string().unwrap(), "[]");
}

#[test]
fn load_goggles_returns_rules_of_existing_ids_in_order() {
    init();
    let a = stored(true);
    let mut b = stored(false);
    b.rules.boost.clear();
    b.rules.discard.insert(9);
    put(&a);
    put(&b);
    let loaded = load_goggles(&[b.id, unique_u64(), a.id]);
    assert_eq!(loaded.len(), 2);
    assert!(loaded[0].discard.contains(&9));
    assert_eq!(loaded[1].boost[&42], 2.0);
    assert!(load_goggles(&[]).is_empty());
}

/* ───────────────────────────── fetch_and_store safety ───────────────────────────── */

fn is_default(g: &Goggle) -> bool {
    g.id == 0 && g.url.is_empty() && g.name.is_empty()
}

#[tokio::test]
async fn fetch_rejects_invalid_and_non_http_urls() {
    init();
    for u in ["", "not a url", "ftp://example.com/x.goggle", "file:///etc/passwd", "javascript:alert(1)", "data:text/plain,$discard"] {
        assert!(is_default(&fetch_and_store(u.to_string()).await), "{}", u);
    }
}

#[tokio::test]
async fn fetch_rejects_local_network_hosts() {
    init();
    for u in [
        "http://localhost/x",
        "http://LOCALHOST/x",
        "http://127.0.0.1/x",
        "http://127.1/x",
        "http://2130706433/x",
        "http://10.1.2.3/x",
        "http://192.168.1.1/x",
        "http://169.254.169.254/latest/meta-data/",
    ] {
        assert!(is_default(&fetch_and_store(u.to_string()).await), "{}", u);
    }
}

fn goggle_body(_: &str) -> Vec<u8> {
    http("200 OK", "text/plain", b"! name: Internal\n$boost=2,site=a.com\n")
}

/// SECURITY (SSRF): the blocklist misses IPv6 loopback, so a goggle URL pointing at
/// [::1] is fetched from inside the server.
#[tokio::test]
async fn fetch_rejects_ipv6_loopback() {
    init();
    let Some(port) = serve_on("[::1]:0", 1, goggle_body) else {
        return; // no IPv6 on this machine
    };
    let g = fetch_and_store(format!("http://[::1]:{}/x.goggle", port)).await;
    assert!(is_default(&g), "internal [::1] service was fetched");
}

/// SECURITY (SSRF): 0.0.0.0 reaches local services on Linux and is not blocked.
#[tokio::test]
async fn fetch_rejects_unspecified_address() {
    init();
    let port = serve(1, goggle_body);
    let g = fetch_and_store(format!("http://0.0.0.0:{}/x.goggle", port)).await;
    assert!(is_default(&g), "internal 0.0.0.0 service was fetched");
}

#[tokio::test]
async fn fetch_from_public_style_server_parses_and_stores() {
    init();
    // Unblocked loopback form (see bug above) lets us exercise the success path
    let port = serve(1, goggle_body);
    let url = format!("http://0.0.0.0:{}/ok.goggle", port);
    let g = fetch_and_store(url.clone()).await;
    if is_default(&g) {
        return; // blocked (bug fixed) -> nothing more to check here
    }
    assert_eq!(g.id, url_to_id(&url));
    assert_eq!(g.name, "Internal");
    assert_eq!(g.url, url);
    assert_eq!(g.content_hash, url_to_id("! name: Internal\n$boost=2,site=a.com\n"));
    assert!(g.fetched_at > 0);
    assert_eq!(get(g.id).unwrap().name, "Internal");
}

#[tokio::test]
async fn fetch_empty_body_returns_default() {
    init();
    let port = serve(1, |_| http("200 OK", "text/plain", b"   \n  "));
    let g = fetch_and_store(format!("http://0.0.0.0:{}/empty.goggle", port)).await;
    assert!(is_default(&g));
}

#[test]
fn goggle_rules_can_be_shared() {
    let r = Arc::new(GoggleRules::default());
    let c = r.clone();
    assert!(c.boost.is_empty());
}
