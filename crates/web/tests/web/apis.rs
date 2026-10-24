//! Tests for src/web/routes/apis.rs: proxy SSRF guard, cache version, suggestions,
//! feedback redirect. Everything here is offline (rejections happen before any request).

use std::fs;

use rocket::http::{ContentType, Status};

use crate::support::{client, init, serve, http};
use crate::web::routes::apis::{self, cache_ver, om_sug, proxy_get, proxy_post};

async fn get_status(u: &str) -> Result<(ContentType, Vec<u8>), Status> {
    init();
    proxy_get(u, None, None).await
}

async fn post_status(u: &str) -> Result<(ContentType, Vec<u8>), Status> {
    init();
    proxy_post(u, b"{}".to_vec(), None, None).await
}

/* ───────────────────────────── proxy: malformed input ───────────────────────────── */

#[tokio::test]
async fn proxy_unparsable_urls_are_bad_request() {
    for u in ["", "not a url", "/relative.png", "//example.com/a.png", "http://", "http://exa mple.com/"] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::BadRequest, "{:?}", u);
        assert_eq!(post_status(u).await.unwrap_err(), Status::BadRequest, "{:?}", u);
    }
}

#[tokio::test]
async fn proxy_invalid_percent_encoding_is_bad_request() {
    // decodes to invalid UTF-8
    assert_eq!(get_status("%FF%FE").await.unwrap_err(), Status::BadRequest);
}

#[tokio::test]
async fn proxy_accepts_percent_encoded_target() {
    // encoded loopback url is decoded first and then rejected by the SSRF guard
    assert_eq!(
        get_status("http%3A%2F%2F127.0.0.1%2Fa.png").await.unwrap_err(),
        Status::Forbidden
    );
}

/* ───────────────────────────── proxy: SSRF guard ───────────────────────────── */

#[tokio::test]
async fn proxy_rejects_non_http_schemes() {
    for u in ["ftp://example.com/a.png", "file:///etc/passwd", "gopher://example.com/", "javascript:alert(1)", "data:image/png;base64,AAAA"] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[tokio::test]
async fn proxy_rejects_credentials_in_url() {
    for u in ["http://user:pw@example.com/a.png", "https://user@example.com/a.png", "https://:pw@example.com/"] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[tokio::test]
async fn proxy_rejects_internal_ipv4() {
    for u in [
        "http://127.0.0.1/a.png",
        "http://127.255.255.254/",
        "http://127.1/",
        "http://2130706433/",
        "http://0x7f000001/",
        "http://10.0.0.1/",
        "http://172.16.0.1/",
        "http://172.31.255.255/",
        "http://192.168.1.1/",
        "http://169.254.169.254/latest/meta-data/",
        "http://0.0.0.0/",
        "http://255.255.255.255/",
        "http://192.0.2.1/",
        "http://198.51.100.1/",
        "http://203.0.113.1/",
    ] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
        assert_eq!(post_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[tokio::test]
async fn proxy_rejects_internal_ipv6() {
    for u in [
        "http://[::1]/",
        "http://[::]/",
        "http://[::ffff:127.0.0.1]/",
        "http://[::ffff:10.0.0.1]/",
        "http://[fc00::1]/",
        "http://[fd12:3456::1]/",
        "http://[fe80::1]/",
        "http://[febf::1]/",
    ] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[tokio::test]
async fn proxy_rejects_localhost_names() {
    for u in ["http://localhost/", "http://LOCALHOST:8080/x.png", "http://localhost./"] {
        assert_eq!(get_status(u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[tokio::test]
async fn proxy_rejects_explicit_port_on_internal_host() {
    assert_eq!(get_status("http://127.0.0.1:22/").await.unwrap_err(), Status::Forbidden);
    assert_eq!(get_status("https://10.0.0.5:8443/a.js").await.unwrap_err(), Status::Forbidden);
}

#[tokio::test]
async fn proxy_never_reaches_a_local_server() {
    let port = serve(1, |_| http("200 OK", "image/png", b"\x89PNG\r\n\x1a\n"));
    for host in ["127.0.0.1", "localhost", "[::ffff:127.0.0.1]"] {
        let u = format!("http://{}:{}/a.png", host, port);
        assert_eq!(get_status(&u).await.unwrap_err(), Status::Forbidden, "{}", u);
    }
}

#[test]
fn proxy_route_rejects_through_rocket() {
    let c = client(rocket::routes![apis::proxy_get, apis::proxy_post]);
    let r = c.get("/proxy?u=http%3A%2F%2F127.0.0.1%2Fx.png").dispatch();
    assert_eq!(r.status(), Status::Forbidden);
    let r = c.post("/proxy?u=http%3A%2F%2F10.0.0.1%2F").body("{}").dispatch();
    assert_eq!(r.status(), Status::Forbidden);
    let r = c.get("/proxy?u=nonsense").dispatch();
    assert_eq!(r.status(), Status::BadRequest);
    // width/height must be numbers
    let r = c.get("/proxy?u=http%3A%2F%2F127.0.0.1%2F&width=abc").dispatch();
    assert_ne!(r.status(), Status::Ok);
    // `u` is mandatory
    let r = c.get("/proxy").dispatch();
    assert_ne!(r.status(), Status::Ok);
}

/* ───────────────────────────── cache-ver ───────────────────────────── */

#[test]
fn cache_ver_unknown_then_trimmed_file_content() {
    init();
    let _ = fs::remove_file("cache_version.txt");
    assert_eq!(cache_ver(), "unknown");
    fs::write("cache_version.txt", "  1.2.3 \n").unwrap();
    assert_eq!(cache_ver(), "1.2.3");
    fs::write("cache_version.txt", "").unwrap();
    assert_eq!(cache_ver(), "");
    fs::remove_file("cache_version.txt").unwrap();
    assert_eq!(cache_ver(), "unknown");
}

/* ───────────────────────────── om_sug ───────────────────────────── */

#[tokio::test]
async fn suggestions_for_empty_query_are_empty_without_network() {
    init();
    for q in ["", "   ", "\t"] {
        let v = om_sug(q).await.into_inner();
        assert_eq!(v, serde_json::json!([q, []]), "{:?}", q);
    }
}

/* ───────────────────────────── feedback (send_signal) ───────────────────────────── */

fn feedback(return_path: &str) -> (Status, String) {
    let c = client(rocket::routes![apis::send_signal]);
    let r = c
        .post("/submit_msg")
        .header(ContentType::Form)
        .body(format!("message=hello&return_path={}", urlencoding::encode(return_path)))
        .dispatch();
    (r.status(), r.headers().get_one("Location").unwrap_or_default().to_string())
}

#[test]
fn feedback_without_signal_config_redirects_back() {
    let (s, loc) = feedback("/roadmap");
    assert_eq!(s, Status::SeeOther);
    assert_eq!(loc, "/roadmap");
}

#[test]
fn feedback_requires_both_fields() {
    let c = client(rocket::routes![apis::send_signal]);
    let r = c.post("/submit_msg").header(ContentType::Form).body("message=hi").dispatch();
    assert_eq!(r.status(), Status::UnprocessableEntity);
}

/// SECURITY (open redirect): return_path is used verbatim.
#[test]
fn feedback_refuses_off_site_redirects() {
    for target in ["https://evil.example/phish", "//evil.example"] {
        let (_, loc) = feedback(target);
        assert!(loc.starts_with('/') && !loc.starts_with("//"), "redirected to {}", loc);
    }
}
