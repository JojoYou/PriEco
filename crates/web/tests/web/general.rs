//! Tests for src/web/functions/general.rs

use rocket::{get, http::{CookieJar, SameSite}, routes, time::Duration};

use crate::support::{client, closed_port, http, init, serve};
use crate::web::functions::general::{
    call_api_future_json, get_domain, hash_node, is_valid_url, set_cookie,
};

/* ───────────────────────────── is_valid_url ───────────────────────────── */

#[test]
fn valid_urls() {
    for u in [
        "http://a.com",
        "https://a.com/x?y=z#w",
        "HTTPS://A.COM",
        "http://127.0.0.1:8080",
        "https://[2001:db8::1]/",
        "https://bücher.de",
    ] {
        assert!(is_valid_url(u), "{}", u);
    }
}

#[test]
fn invalid_urls() {
    for u in [
        "",
        " ",
        "a.com",
        "/x",
        "ftp://a.com",
        "javascript:alert(1)",
        "data:text/html,<b>x</b>",
        "file:///etc/passwd",
        "https://",
        "mailto:x@y.z",
    ] {
        assert!(!is_valid_url(u), "{:?}", u);
    }
}

/* ───────────────────────────── get_domain ───────────────────────────── */

#[test]
fn get_domain_basic() {
    assert_eq!(get_domain("https://www.example.com/path", false), "www.example.com");
    assert_eq!(get_domain("https://www.example.com/path", true), "example.com");
    assert_eq!(get_domain("https://example.com", true), "example.com");
    assert_eq!(get_domain("http://sub.example.org:81/", true), "sub.example.org");
}

#[test]
fn get_domain_case_and_idn() {
    assert_eq!(get_domain("https://WWW.EXAMPLE.COM", true), "example.com");
    assert_eq!(get_domain("https://www.bücher.de", true), "xn--bcher-kva.de");
}

#[test]
fn get_domain_empty_for_ip_invalid_and_hostless() {
    for u in ["", "garbage", "http://10.0.0.1/", "http://[::1]/", "mailto:a@b.c", "example.com"] {
        assert_eq!(get_domain(u, true), "", "{:?}", u);
    }
}

#[test]
fn get_domain_matches_crawler_implementation() {
    for u in [
        "https://www.a.com/x",
        "https://a.b.c.d/",
        "http://www.www.a.com",
        "",
        "nope",
        "http://1.2.3.4/",
    ] {
        for www in [true, false] {
            assert_eq!(
                get_domain(u, www),
                prieco_mini_crawler::meta::get_domain(u, www),
                "{} {}",
                u,
                www
            );
        }
    }
}

/* ───────────────────────────── hash_node ───────────────────────────── */

#[test]
fn hash_node_matches_xxh64_reference() {
    // XXH64("") with seed 0
    assert_eq!(hash_node(""), 0xEF46_DB37_51D8_E999);
}

#[test]
fn hash_node_is_deterministic_and_distinguishes_inputs() {
    assert_eq!(hash_node("https://a.com"), hash_node("https://a.com"));
    assert_ne!(hash_node("https://a.com"), hash_node("https://a.com/"));
    assert_ne!(hash_node("a"), hash_node("A"));
}

#[test]
fn hash_node_differs_from_url_to_id() {
    // different algorithm (XXH64 vs XXH3) -> callers must not mix them
    assert_ne!(hash_node("https://a.com"), prieco_core::url_to_id("https://a.com"));
}

/* ───────────────────────────── set_cookie ───────────────────────────── */

#[get("/set?<name>&<value>&<long>&<js>&<secure>")]
fn set_route(
    jar: &CookieJar<'_>,
    name: String,
    value: String,
    long: bool,
    js: bool,
    secure: bool,
) -> &'static str {
    set_cookie(jar, name, value, long, js, secure);
    "ok"
}

fn set(query: &str) -> rocket::http::Cookie<'static> {
    let c = client(routes![set_route]);
    let res = c.get(format!("/set?{}", query)).dispatch();
    assert_eq!(res.status(), rocket::http::Status::Ok);
    res.cookies().iter().next().cloned().expect("no cookie set")
}

#[test]
fn set_cookie_short_life_is_seven_days() {
    let c = set("name=a&value=b&long=false&js=true&secure=false");
    assert_eq!(c.name(), "a");
    assert_eq!(c.value(), "b");
    assert_eq!(c.max_age(), Some(Duration::days(7)));
}

#[test]
fn set_cookie_long_life_is_one_year() {
    let c = set("name=a&value=b&long=true&js=true&secure=false");
    assert_eq!(c.max_age(), Some(Duration::days(365)));
    let expires = c.expires_datetime().expect("expires must be set");
    let now = rocket::time::OffsetDateTime::now_utc();
    assert!(expires > now + Duration::days(364));
    assert!(expires < now + Duration::days(366));
}

#[test]
fn set_cookie_common_attributes() {
    let c = set("name=a&value=b&long=false&js=true&secure=false");
    assert_eq!(c.same_site(), Some(SameSite::Lax));
    assert_eq!(c.path(), Some("/"));
}

#[test]
fn set_cookie_http_only_unless_js_needs_it() {
    let hidden = set("name=a&value=b&long=false&js=false&secure=false");
    assert_eq!(hidden.http_only(), Some(true));
    let visible = set("name=a&value=b&long=false&js=true&secure=false");
    assert_ne!(visible.http_only(), Some(true));
}

#[test]
fn set_cookie_secure_flag() {
    assert_eq!(set("name=a&value=b&long=false&js=true&secure=true").secure(), Some(true));
    assert_ne!(set("name=a&value=b&long=false&js=true&secure=false").secure(), Some(true));
}

#[test]
fn set_cookie_keeps_unicode_and_encoded_values() {
    let c = set("name=lang&value=sk%2Cen&long=true&js=true&secure=true");
    assert_eq!(c.value(), "sk,en");
}

/* ───────────────────────────── call_api_future_json ───────────────────────────── */

fn http_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

#[tokio::test]
async fn call_api_parses_json_object() {
    init();
    let port = serve(1, |_| http("200 OK", "application/json", br#"{"a":1,"b":[true,null]}"#));
    let v = call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/", port)))
        .await
        .unwrap();
    assert_eq!(v["a"], 1);
    assert_eq!(v["b"][0], true);
}

#[tokio::test]
async fn call_api_parses_json_even_on_error_status() {
    init();
    let port = serve(1, |_| http("500 Internal Server Error", "application/json", br#"{"error":"x"}"#));
    let v = call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/", port))).await;
    assert_eq!(v.unwrap()["error"], "x");
}

#[tokio::test]
async fn call_api_non_json_body_is_none() {
    init();
    let port = serve(1, |_| http("200 OK", "text/html", b"<html>not json</html>"));
    assert!(
        call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/", port)))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn call_api_empty_body_is_none() {
    init();
    let port = serve(1, |_| http("204 No Content", "application/json", b""));
    assert!(
        call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/", port)))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn call_api_connection_error_is_none() {
    init();
    let url = format!("http://127.0.0.1:{}/", closed_port());
    assert!(call_api_future_json(http_client().get(url)).await.is_none());
}

#[tokio::test]
async fn call_api_json_scalar_and_array() {
    init();
    let port = serve(2, |p| {
        if p == "/arr" {
            http("200 OK", "application/json", b"[1,2,3]")
        } else {
            http("200 OK", "application/json", b"42")
        }
    });
    let a = call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/arr", port)))
        .await
        .unwrap();
    assert_eq!(a.as_array().unwrap().len(), 3);
    let n = call_api_future_json(http_client().get(format!("http://127.0.0.1:{}/n", port)))
        .await
        .unwrap();
    assert_eq!(n, 42);
}
