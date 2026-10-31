//! Tests for search helpers: search_db::path_matches / QUERY_CACHE,
//! search_endpoint::UserQtPrefs, discover, news and yadore API guards.

use prieco_core::url_to_domain_id;
use rocket::{get, http::{Cookie, CookieJar}, routes};

use crate::support::{client, doc, init};
use crate::web::functions::{
    additional::discover::discover_and_ping_domains,
    search_api::{news, yadore},
    search_db::{QUERY_CACHE, path_matches},
    search_endpoint::{UserQtPrefs, get_user_qt_prefs},
};

/* ───────────────────────────── path_matches ───────────────────────────── */

#[test]
fn path_matches_contains_by_default() {
    assert!(path_matches("https://a.com/blog/post", "/blog/"));
    assert!(path_matches("https://a.com/blog/post", "a.com"));
    assert!(!path_matches("https://a.com/news", "/blog/"));
}

#[test]
fn path_matches_anchored_start() {
    assert!(path_matches("https://a.com/x", "^https://a.com"));
    assert!(!path_matches("http://a.com/x", "^https://a.com"));
    assert!(!path_matches("https://b.com/https://a.com", "^https://a.com"));
}

#[test]
fn path_matches_anchored_end() {
    assert!(path_matches("https://a.com/file.pdf", ".pdf$"));
    assert!(!path_matches("https://a.com/file.pdf?x=1", ".pdf$"));
}

#[test]
fn path_matches_exact() {
    assert!(path_matches("https://a.com/", "^https://a.com/$"));
    assert!(!path_matches("https://a.com/x", "^https://a.com/$"));
}

#[test]
fn path_matches_edge_patterns() {
    // empty pattern matches everything, "^" and "$" alone too
    assert!(path_matches("anything", ""));
    assert!(path_matches("anything", "^"));
    assert!(path_matches("anything", "$"));
    // "^$" only matches the empty url
    assert!(path_matches("", "^$"));
    assert!(!path_matches("x", "^$"));
    assert!(!path_matches("", "x"));
}

#[test]
fn path_matches_is_case_sensitive_and_literal() {
    assert!(!path_matches("https://a.com/Blog", "/blog"));
    // no regex semantics: "." is literal
    assert!(!path_matches("https://a.com/xpdf", ".pdf$"));
    assert!(path_matches("https://a.com/a*b", "a*b"));
}

#[test]
fn path_matches_unicode() {
    assert!(path_matches("https://a.com/žltý", "žltý"));
    assert!(path_matches("https://a.com/žltý", "žltý$"));
}

/* ───────────────────────────── QUERY_CACHE ───────────────────────────── */

#[test]
fn query_cache_is_shared_and_writable() {
    let key = format!("unit_test_query_{}_all_all", std::process::id());
    QUERY_CACHE.write().insert(key.clone(), vec![doc("https://cached.example")]);
    assert_eq!(QUERY_CACHE.read().get(&key).unwrap()[0].url, "https://cached.example");
    QUERY_CACHE.write().remove(&key);
    assert!(QUERY_CACHE.read().get(&key).is_none());
}

/* ───────────────────────────── UserQtPrefs ───────────────────────────── */

fn prefs(boost: &[&str], downrank: &[&str], discard: &[&str]) -> UserQtPrefs {
    UserQtPrefs {
        boost: boost.iter().map(|s| s.to_string()).collect(),
        downrank: downrank.iter().map(|s| s.to_string()).collect(),
        discard: discard.iter().map(|s| s.to_string()).collect(),
    }
}

fn did(d: &str) -> u64 {
    url_to_domain_id(&format!("https://{}/", d))
}

#[test]
fn qt_empty_prefs_give_empty_rules() {
    let r = UserQtPrefs::default().into_goggle_rules();
    assert!(r.boost.is_empty() && r.downrank.is_empty() && r.discard.is_empty());
    assert!(r.important.is_empty() && r.path.is_empty());
    assert!(!r.discard_by_default);
}

#[test]
fn qt_boost_adds_apex_and_www() {
    let r = prefs(&["example.com"], &[], &[]).into_goggle_rules();
    assert_eq!(r.boost.len(), 2);
    assert_eq!(r.boost[&did("example.com")], 3.0);
    assert_eq!(r.boost[&did("www.example.com")], 3.0);
}

#[test]
fn qt_www_domain_is_not_doubled() {
    let r = prefs(&["www.example.com"], &[], &[]).into_goggle_rules();
    assert_eq!(r.boost.len(), 1);
    assert!(r.boost.contains_key(&did("www.example.com")));
}

#[test]
fn qt_downrank_and_discard() {
    let r = prefs(&[], &["slow.com"], &["spam.com"]).into_goggle_rules();
    assert_eq!(r.downrank[&did("slow.com")], 3.0);
    assert_eq!(r.downrank[&did("www.slow.com")], 3.0);
    assert!(r.discard.contains(&did("spam.com")));
    assert!(r.discard.contains(&did("www.spam.com")));
    assert!(r.boost.is_empty());
}

#[test]
fn qt_invalid_domains_map_to_zero_id() {
    // Documents current behaviour: garbage entries collapse onto domain id 0
    let r = prefs(&["not a domain"], &[], &[]).into_goggle_rules();
    assert!(r.boost.contains_key(&0));
}

#[test]
fn qt_serde_defaults() {
    let p: UserQtPrefs = serde_json::from_str("{}").unwrap();
    assert!(p.boost.is_empty() && p.downrank.is_empty() && p.discard.is_empty());
    let p: UserQtPrefs = serde_json::from_str(r#"{"boost":["a.com"]}"#).unwrap();
    assert_eq!(p.boost, vec!["a.com"]);
    let s = serde_json::to_string(&prefs(&["a"], &["b"], &["c"])).unwrap();
    let back: UserQtPrefs = serde_json::from_str(&s).unwrap();
    assert_eq!((back.boost, back.downrank, back.discard), (vec!["a".to_string()], vec!["b".to_string()], vec!["c".to_string()]));
}

#[get("/qt")]
fn qt_route(jar: &CookieJar<'_>) -> String {
    let p = get_user_qt_prefs(jar);
    format!("{:?}|{:?}|{:?}", p.boost, p.downrank, p.discard)
}

#[test]
fn qt_prefs_from_cookie() {
    let c = client(routes![qt_route]);
    let json = r#"{"boost":["a.com"],"downrank":["b.com"],"discard":["c.com"]}"#;

    // raw json cookie (as written by /quick_tune_update)
    let r = c.get("/qt").cookie(Cookie::new("prieco_qt_prefs", json)).dispatch();
    assert_eq!(r.into_string().unwrap(), r#"["a.com"]|["b.com"]|["c.com"]"#);

    // url encoded json cookie
    let enc = urlencoding::encode(json).into_owned();
    let r = c.get("/qt").cookie(Cookie::new("prieco_qt_prefs", enc)).dispatch();
    assert_eq!(r.into_string().unwrap(), r#"["a.com"]|["b.com"]|["c.com"]"#);
}

#[test]
fn qt_prefs_missing_or_invalid_cookie_is_default() {
    let c = client(routes![qt_route]);
    assert_eq!(c.get("/qt").dispatch().into_string().unwrap(), "[]|[]|[]");
    for bad in ["garbage", "%ZZ", "%FF%FE", "[1,2]", r#"{"boost":"x"}"#] {
        let r = c.get("/qt").cookie(Cookie::new("prieco_qt_prefs", bad)).dispatch();
        assert_eq!(r.into_string().unwrap(), "[]|[]|[]", "{:?}", bad);
    }
}

/* ───────────────────────────── discover ───────────────────────────── */

#[tokio::test]
async fn discover_empty_queries_do_nothing() {
    init();
    for q in ["", "   ", "\"\"", "''", " \" ' \t"] {
        assert!(discover_and_ping_domains(q).await.is_empty(), "{:?}", q);
    }
}

/* ───────────────────────────── external API guards ───────────────────────────── */

#[tokio::test]
async fn news_without_api_key_is_an_error() {
    init();
    let err = news::run("rust", "en", "us", 10).await.unwrap_err();
    assert!(err.to_string().contains("NEWS_API_KEY"));
}

#[tokio::test]
async fn yadore_without_api_key_is_an_error() {
    init();
    assert!(yadore::run("shoes", "de").await.is_err());
}

/// BUG (copy paste): the yadore error message talks about NEWS_API_KEY.
#[tokio::test]
async fn yadore_error_names_the_right_variable() {
    init();
    let err = yadore::run("shoes", "de").await.err().expect("expected an error");
    assert!(err.to_string().contains("YADORE_API_KEY"), "{}", err);
}

#[test]
fn news_article_serde() {
    let a: news::Article = serde_json::from_str(
        r#"{"id":"1","title":"t","description":"d","url":"u","author":"a","image":"i",
            "language":"en","category":["tech"],"published":"2026-01-01"}"#,
    )
    .unwrap();
    assert_eq!(a.category, vec!["tech"]);
    let r: news::NewsResponse =
        serde_json::from_str(r#"{"status":"ok","news":[]}"#).unwrap();
    assert_eq!(r.status, "ok");
    assert!(r.page.is_none());
    assert!(serde_json::from_str::<news::NewsResponse>(r#"{"status":"ok"}"#).is_err());
}
