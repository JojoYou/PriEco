//! Tests for src/web/modules/settings.rs (language/location auto detection, page context)
//!
//! The sandbox geo database maps 1.0.0.x -> "SK", 2.0.0.x -> "" and 3.0.0.x -> "US".

use std::{collections::HashMap, net::IpAddr};

use rocket::{
    get,
    http::{Cookie, CookieJar, uri::Host},
    local::blocking::Client,
    routes,
    serde::json::Json,
};
use serde_json::Value;

use crate::support::{client, unique_u64};
use crate::web::functions::ranking::goggles::{Goggle, put};
use crate::web::modules::settings;

#[get("/probe?<ip>&<host>")]
fn probe(ip: Option<String>, host: Option<String>, jar: &CookieJar<'_>) -> Json<HashMap<String, Value>> {
    let mut ctx = HashMap::new();
    let ip: Option<IpAddr> = ip.and_then(|s| s.parse().ok());
    let host = host.unwrap_or_else(|| "prieco.net".to_string());
    let h = Host::parse(&host).unwrap();
    settings::run(&mut ctx, &ip, jar, &h);
    Json(ctx)
}

struct Probe {
    ctx: HashMap<String, Value>,
    set: HashMap<String, Cookie<'static>>,
}

fn run(query: &str, cookies: &[(&str, &str)]) -> Probe {
    let c: &'static Client = Box::leak(Box::new(client(routes![probe])));
    let mut req = c.get(format!("/probe?{}", query));
    for (k, v) in cookies {
        req = req.cookie(Cookie::new(k.to_string(), v.to_string()));
    }
    let r = req.dispatch();
    let set = r
        .cookies()
        .iter()
        .map(|c| (c.name().to_string(), c.clone()))
        .collect();
    let ctx = r.into_json::<HashMap<String, Value>>().unwrap();
    Probe { ctx, set }
}

impl Probe {
    fn s(&self, k: &str) -> &str {
        self.ctx.get(k).and_then(|v| v.as_str()).unwrap_or_default()
    }
    fn cookie(&self, k: &str) -> Option<&str> {
        self.set.get(k).map(|c| c.value())
    }
}

/* ───────────────────────────── defaults ───────────────────────────── */

#[test]
fn defaults_without_ip_or_cookies() {
    let p = run("", &[]);
    assert_eq!(p.s("selected_loc"), "all");
    assert_eq!(p.s("selected_lang"), "all");
    assert_eq!(p.s("selected_theme"), "system");
    assert_eq!(p.s("css_path"), "/static/css/desktop/");
    assert_eq!(p.s("css_theme"), "/static/css/system/");
    assert_eq!(p.s("goggles_ids"), "");
    assert!(p.ctx.get("goggles").unwrap().is_array());
    for k in ["osd_title", "check_newtab", "newtab", "check_js", "check_post", "check_dec"] {
        assert!(!p.ctx.contains_key(k), "{}", k);
    }
    assert!(p.cookie("loc").is_none(), "no ip -> no location cookie");
    assert!(p.cookie("lang").is_none());
}

/* ───────────────────────────── location detection ───────────────────────────── */

#[test]
fn ip_location_is_detected_and_lowercased() {
    let p = run("ip=1.0.0.7", &[]);
    assert_eq!(p.cookie("loc"), Some("sk"));
    let p = run("ip=3.0.0.200", &[]);
    assert_eq!(p.cookie("loc"), Some("us"));
}

#[test]
fn unknown_or_empty_country_falls_back_to_all() {
    for ip in ["2.0.0.1", "9.9.9.9", "255.255.255.254", "::1", "2001:db8::1"] {
        let p = run(&format!("ip={}", ip), &[]);
        assert_eq!(p.cookie("loc"), Some("all"), "{}", ip);
    }
}

#[test]
fn unspecified_ip_is_treated_as_missing() {
    let p = run("ip=0.0.0.0", &[]);
    assert!(p.cookie("loc").is_none());
}

#[test]
fn location_cookie_attributes() {
    let p = run("ip=1.0.0.7", &[]);
    let c = &p.set["loc"];
    assert_eq!(c.max_age(), Some(rocket::time::Duration::days(7)));
    assert_eq!(c.secure(), Some(true));
    assert_ne!(c.http_only(), Some(true), "readable by JS");
}

#[test]
fn existing_location_cookie_is_respected() {
    let p = run("ip=1.0.0.7", &[("loc", "de")]);
    assert!(p.cookie("loc").is_none(), "must not overwrite the user's choice");
    assert_eq!(p.s("selected_loc"), "de");
}

#[test]
fn onion_host_never_geolocates() {
    let p = run("ip=1.0.0.7&host=abc.onion", &[]);
    assert_eq!(p.cookie("loc"), Some("all"));
    assert_eq!(p.s("osd_title"), " (Onion)");
}

/* ───────────────────────────── language detection ───────────────────────────── */

#[test]
fn language_is_derived_from_location_cookie() {
    for (loc, lang) in [("at", "de"), ("sk", "sk"), ("us", "en"), ("cn", "all"), ("zz", "all"), ("all", "all")] {
        let p = run("", &[("loc", loc)]);
        assert_eq!(p.cookie("lang"), Some(lang), "{}", loc);
    }
}

#[test]
fn onion_language_is_all() {
    let p = run("host=abc.onion", &[("loc", "sk")]);
    assert_eq!(p.cookie("lang"), Some("all"));
}

#[test]
fn existing_language_cookie_is_respected() {
    let p = run("", &[("loc", "at"), ("lang", "en")]);
    assert!(p.cookie("lang").is_none());
    assert_eq!(p.s("selected_lang"), "en");
    assert_eq!(p.s("selected_loc"), "at");
}

/// BUG: `CookieJar::get` only sees cookies sent by the browser, not ones added in the
/// same request. On the very first visit the detected location is written, but the
/// language is not derived from it and the page still renders "all"/"all".
#[test]
fn first_visit_applies_detected_location_and_language() {
    let p = run("ip=1.0.0.7", &[]);
    assert_eq!(p.cookie("loc"), Some("sk"));
    assert_eq!(p.s("selected_loc"), "sk", "page rendered with stale location");
    assert_eq!(p.cookie("lang"), Some("sk"), "language not detected on first visit");
}

/* ───────────────────────────── appearance flags ───────────────────────────── */

#[test]
fn newtab_js_post_flags() {
    let p = run("", &[("newtab", "1"), ("js", "1"), ("post", "1")]);
    assert_eq!(p.ctx["check_newtab"], 1);
    assert_eq!(p.s("newtab"), "target='_blank'");
    assert_eq!(p.ctx["check_js"], 1);
    assert_eq!(p.ctx["check_post"], 1);
}

#[test]
fn decentralization_flag_only_when_zero() {
    assert_eq!(run("", &[("dec", "0")]).ctx["check_dec"], 1);
    assert!(!run("", &[("dec", "1")]).ctx.contains_key("check_dec"));
    assert!(!run("", &[("dec", "")]).ctx.contains_key("check_dec"));
}

#[test]
fn themes() {
    for (theme, css) in [
        ("light", "/static/css/light/"),
        ("dark", "/static/css/dark/"),
        ("system", "/static/css/system/"),
        ("neon", "/static/css/system/"),
    ] {
        let p = run("", &[("theme", theme)]);
        assert_eq!(p.s("selected_theme"), theme);
        assert_eq!(p.s("css_theme"), css, "{}", theme);
    }
}

#[test]
fn screen_width_selects_mobile_or_desktop_css() {
    for (w, css) in [
        ("320", "/static/css/mobile/"),
        ("889", "/static/css/mobile/"),
        ("0", "/static/css/mobile/"),
        ("890", "/static/css/desktop/"),
        ("1920", "/static/css/desktop/"),
        ("abc", "/static/css/desktop/"),
        ("-5", "/static/css/desktop/"),
        ("99999999999", "/static/css/desktop/"),
    ] {
        let p = run("", &[("screen_width", w)]);
        assert_eq!(p.s("css_path"), css, "{}", w);
    }
}

/* ───────────────────────────── goggles ───────────────────────────── */

#[test]
fn active_goggle_ids_are_parsed() {
    let p = run("", &[("active_goggles", "3, 1,x,,2")]);
    let mut ids: Vec<&str> = p.s("goggles_ids").split(',').collect();
    ids.sort();
    assert_eq!(ids, vec!["1", "2", "3"]);
}

#[test]
fn public_goggles_are_listed_and_marked_checked() {
    let mut g = Goggle::default();
    g.id = unique_u64();
    g.name = "Listed".into();
    g.author = "Tester".into();
    g.public = true;
    g.avatar = "#00ff00".into();
    g.url = "https://example.com/x.goggle".into();
    put(&g);

    let mut hidden = Goggle::default();
    hidden.id = unique_u64();
    hidden.public = false;
    put(&hidden);

    let p = run("", &[("active_goggles", &g.id.to_string())]);
    let list = p.ctx["goggles"].as_array().unwrap();
    let entry = list.iter().find(|e| e["id"] == g.id).expect("public goggle missing");
    assert_eq!(entry["name"], "Listed");
    assert_eq!(entry["author"], "Tester");
    assert_eq!(entry["source_url"], "https://example.com/x.goggle");
    assert_eq!(entry["avatar"], "00ff00");
    assert_eq!(entry["checked"], true);
    assert!(!list.iter().any(|e| e["id"] == hidden.id));

    let p = run("", &[]);
    let list = p.ctx["goggles"].as_array().unwrap();
    let entry = list.iter().find(|e| e["id"] == g.id).unwrap();
    assert_eq!(entry["checked"], false);
}
