//! Tests for src/web/routes/pages.rs (pure helpers + cookie only routes through
//! Rocket's local client; template rendering routes are out of scope)

use rocket::{
    get,
    http::{ContentType, Cookie, Header, SameSite, Status},
    local::blocking::{Client, LocalResponse},
    routes,
};

use crate::support::client;
use crate::web::functions::ranking::goggles::{domain_str_to_id, parse_goggle};
use crate::web::routes::pages::{
    self, ClientIp, apply_goggles, country_info, export_quick_tune, goggles_tint,
    handle_shortcuts, index_head, set_preferences, settings_update, update_qt,
};

fn location(r: &LocalResponse<'_>) -> String {
    r.headers().get_one("Location").unwrap_or_default().to_string()
}

fn cookie<'a>(r: &'a LocalResponse<'_>, name: &str) -> Option<Cookie<'static>> {
    r.cookies().get(name).cloned()
}

fn is_removal(c: &Cookie<'_>) -> bool {
    c.value().is_empty() && c.max_age().map_or(true, |a| a.is_zero() || a.is_negative())
}

/* ───────────────────────────── country_info ───────────────────────────── */

#[test]
fn country_info_known_codes() {
    assert_eq!(country_info("sk"), ("Slovakia", "🇸🇰"));
    assert_eq!(country_info("de"), ("Germany", "🇩🇪"));
    assert_eq!(country_info("us").1, "🇺🇸");
    assert_eq!(country_info("ar"), ("Argentina", "🇦🇷"));
    assert_eq!(country_info("zw"), ("Zimbabwe", "🇿🇼"));
}

#[test]
fn country_info_is_case_insensitive() {
    assert_eq!(country_info("SK"), country_info("sk"));
    assert_eq!(country_info("Sk"), country_info("sk"));
}

#[test]
fn country_info_unknown_codes() {
    for c in ["", "zz", "xx", "all", "slovakia", "s", "sk ", "🇸🇰"] {
        assert_eq!(country_info(c), ("Unknown", "🏳"), "{:?}", c);
    }
}

/// Every arm of the big match is parsed from the source and checked for consistency:
/// unique codes, non-empty names, and the flag must be the regional indicator pair
/// of its own code.
#[test]
fn country_info_table_is_consistent() {
    let src = include_str!("../../src/web/routes/pages.rs");
    let start = src.find("pub fn country_info").unwrap();
    let body = &src[start..];
    let body = &body[..body.find("_ => (\"Unknown\"").unwrap()];

    let mut seen = std::collections::HashSet::new();
    let mut bad_flags = Vec::new();
    let mut count = 0;
    for line in body.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('"') else { continue };
        let Some((code, _)) = rest.split_once('"') else { continue };
        count += 1;
        assert!(seen.insert(code.to_string()), "duplicate code {}", code);
        let (name, flag) = country_info(code);
        assert_ne!(name, "Unknown", "{}", code);
        assert!(!name.trim().is_empty());
        let expected: String = code
            .chars()
            .map(|c| char::from_u32(0x1F1E6 + (c as u32 - 'a' as u32)).unwrap())
            .collect();
        if flag != expected {
            bad_flags.push(format!("{} -> {}", code, flag));
        }
    }
    assert!(count > 200, "parsed only {} entries", count);
    assert!(bad_flags.is_empty(), "flags not matching their code: {:?}", bad_flags);
}

/// Every country that the language auto detection knows should have a display name
#[test]
fn country_info_covers_language_detection_countries() {
    let missing: Vec<&str> = prieco_core::COUNTRY_TO_LANG
        .keys()
        .copied()
        .filter(|c| country_info(c).0 == "Unknown")
        .collect();
    assert!(missing.is_empty(), "codes without a country: {:?}", missing);
}

/* ───────────────────────────── goggles_tint ───────────────────────────── */

#[test]
fn tint_defaults() {
    let (ct, css) = goggles_tint(None, None);
    assert_eq!(ct, ContentType::CSS);
    assert_eq!(css, ". { background-color: #14141E66; }");
}

#[test]
fn tint_valid_hex_with_and_without_hash() {
    assert_eq!(goggles_tint(Some("ff00AA"), Some("card")).1, ".card { background-color: #ff00AA66; }");
    assert_eq!(goggles_tint(Some("#123456"), Some("x")).1, ".x { background-color: #12345666; }");
    assert_eq!(goggles_tint(Some("##123456"), Some("x")).1, ".x { background-color: #12345666; }");
}

#[test]
fn tint_invalid_hex_falls_back() {
    for t in ["", "fff", "1234567", "gggggg", "12345z", "#12 456", "ŽŽŽ", "red"] {
        assert!(goggles_tint(Some(t), Some("x")).1.contains("#14141E66"), "{:?}", t);
    }
}

/// SECURITY: `target` is copied into the stylesheet unescaped -> CSS injection
/// (e.g. hide or restyle the whole page, exfiltrate via url()).
#[test]
fn tint_target_cannot_inject_css() {
    let (_, css) = goggles_tint(Some("ffffff"), Some("x{} body{display:none} .y"));
    assert!(!css.contains("body{display:none}"), "{}", css);
}

#[get("/tint?<tint>&<target>")]
fn tint_route(tint: Option<&str>, target: Option<&str>) -> (ContentType, String) {
    goggles_tint(tint, target)
}

#[test]
fn tint_route_serves_css() {
    let c = client(routes![tint_route]);
    let r = c.get("/tint?tint=00ff00&target=g1").dispatch();
    assert_eq!(r.content_type(), Some(ContentType::CSS));
    assert_eq!(r.into_string().unwrap(), ".g1 { background-color: #00ff0066; }");
}

/* ───────────────────────────── index_head ───────────────────────────── */

#[test]
fn index_head_is_empty_body() {
    assert_eq!(index_head(), "");
}

#[test]
fn index_head_route_answers_head() {
    let c = client(routes![pages::index_head]);
    let r = c.head("/").dispatch();
    assert_eq!(r.status(), Status::Ok);
}

/* ───────────────────────────── ClientIp ───────────────────────────── */

#[get("/ip")]
fn ip_route(ip: ClientIp) -> String {
    ip.0.to_string()
}

fn ip_client() -> Client {
    client(routes![ip_route])
}

#[test]
fn client_ip_prefers_cloudflare_header() {
    let c = ip_client();
    let r = c
        .get("/ip")
        .header(Header::new("CF-Connecting-IP", "1.2.3.4"))
        .header(Header::new("X-Forwarded-For", "5.6.7.8"))
        .header(Header::new("X-Real-IP", "9.9.9.9"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "1.2.3.4");
}

#[test]
fn client_ip_uses_first_forwarded_for_entry() {
    let c = ip_client();
    let r = c
        .get("/ip")
        .header(Header::new("X-Forwarded-For", " 5.6.7.8 , 10.0.0.1"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "5.6.7.8");
}

#[test]
fn client_ip_real_ip_header() {
    let c = ip_client();
    let r = c.get("/ip").header(Header::new("X-Real-IP", "9.9.9.9")).dispatch();
    assert_eq!(r.into_string().unwrap(), "9.9.9.9");
}

#[test]
fn client_ip_ipv6() {
    let c = ip_client();
    let r = c
        .get("/ip")
        .header(Header::new("CF-Connecting-IP", "2001:db8::1"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "2001:db8::1");
}

#[test]
fn client_ip_invalid_headers_fall_through() {
    let c = ip_client();
    let r = c
        .get("/ip")
        .header(Header::new("CF-Connecting-IP", "not-an-ip"))
        .header(Header::new("X-Forwarded-For", "garbage, 1.1.1.1"))
        .header(Header::new("X-Real-IP", "8.8.4.4"))
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "8.8.4.4");
}

#[test]
fn client_ip_falls_back_to_socket_address() {
    let c = ip_client();
    let r = c
        .get("/ip")
        .remote("7.7.7.7:1234".parse().unwrap())
        .dispatch();
    assert_eq!(r.into_string().unwrap(), "7.7.7.7");
}

#[test]
fn client_ip_without_any_source_is_bad_request() {
    let c = ip_client();
    let r = c.get("/ip").dispatch();
    assert_eq!(r.status(), Status::BadRequest);
}

/* ───────────────────────────── shortcuts ───────────────────────────── */

fn shortcuts_client() -> Client {
    client(routes![handle_shortcuts])
}

fn post_shortcut(c: &Client, form: &str, existing: Option<&str>) -> (Status, String, Vec<String>) {
    let mut req = c.post("/").header(ContentType::Form).body(form.to_string());
    if let Some(v) = existing {
        req = req.cookie(Cookie::new("shortcuts", urlencoding::encode(v).into_owned()));
    }
    let r = req.dispatch();
    let status = r.status();
    let loc = location(&r);
    let items = cookie(&r, "shortcuts")
        .map(|c| {
            urlencoding::decode(c.value())
                .unwrap()
                .split(',')
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    (status, loc, items)
}

#[test]
fn shortcut_add_first_inserts_dummy_and_adds_https() {
    let c = shortcuts_client();
    let (status, loc, items) = post_shortcut(&c, "action=add&shortcut_name=Git&shortcut_url=github.com", None);
    assert_eq!(status, Status::SeeOther);
    assert_eq!(loc, "/");
    assert_eq!(items, vec!["dummy=dummy", "Git=https://github.com"]);
}

#[test]
fn shortcut_add_keeps_explicit_scheme() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(
        &c,
        "action=add&shortcut_name=Old&shortcut_url=http%3A%2F%2Fold.example",
        Some("dummy=dummy"),
    );
    assert_eq!(items, vec!["dummy=dummy", "Old=http://old.example"]);
}

#[test]
fn shortcut_add_without_name_or_url_changes_nothing() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(&c, "action=add&shortcut_name=X", Some("dummy=dummy,A=https://a.com"));
    assert_eq!(items, vec!["dummy=dummy", "A=https://a.com"]);
}

#[test]
fn shortcut_edit() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(
        &c,
        "action=edit&shortcut_id=1&shortcut_name=B&shortcut_url=b.com",
        Some("dummy=dummy,A=https://a.com"),
    );
    assert_eq!(items, vec!["dummy=dummy", "B=https://b.com"]);
}

#[test]
fn shortcut_edit_out_of_range_is_ignored() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(
        &c,
        "action=edit&shortcut_id=9&shortcut_name=B&shortcut_url=b.com",
        Some("dummy=dummy,A=https://a.com"),
    );
    assert_eq!(items, vec!["dummy=dummy", "A=https://a.com"]);
}

#[test]
fn shortcut_delete() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(&c, "action=delete&shortcut_id=1", Some("dummy=dummy,A=https://a.com,B=https://b.com"));
    assert_eq!(items, vec!["dummy=dummy", "B=https://b.com"]);
}

#[test]
fn shortcut_delete_out_of_range_and_missing_id() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(&c, "action=delete&shortcut_id=5", Some("dummy=dummy,A=https://a.com"));
    assert_eq!(items, vec!["dummy=dummy", "A=https://a.com"]);
    let (_, _, items) = post_shortcut(&c, "action=delete", Some("dummy=dummy,A=https://a.com"));
    assert_eq!(items, vec!["dummy=dummy", "A=https://a.com"]);
}

#[test]
fn shortcut_unknown_action_rewrites_same_cookie() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(&c, "action=explode", Some("dummy=dummy,A=https://a.com"));
    assert_eq!(items, vec!["dummy=dummy", "A=https://a.com"]);
}

#[test]
fn shortcut_cookie_attributes() {
    let c = shortcuts_client();
    let r = c
        .post("/")
        .header(ContentType::Form)
        .body("action=add&shortcut_name=A&shortcut_url=a.com")
        .dispatch();
    let ck = cookie(&r, "shortcuts").unwrap();
    assert_eq!(ck.max_age(), Some(rocket::time::Duration::days(365)));
    assert_eq!(ck.secure(), Some(true));
    assert_ne!(ck.http_only(), Some(true), "readable by JS");
}

#[test]
fn shortcut_missing_action_is_unprocessable() {
    let c = shortcuts_client();
    let r = c.post("/").header(ContentType::Form).body("shortcut_name=A").dispatch();
    assert_eq!(r.status(), Status::UnprocessableEntity);
}

/// BUG: shortcuts are stored as "name=url" joined by ",". A name containing ","
/// splits into two entries, so it can never be deleted as one shortcut.
#[test]
fn shortcut_name_with_comma_stays_one_shortcut() {
    let c = shortcuts_client();
    let (_, _, items) = post_shortcut(&c, "action=add&shortcut_name=a%2Cb&shortcut_url=x.com", None);
    let stored = items.join(",");
    let (_, _, after) = post_shortcut(&c, "action=delete&shortcut_id=1", Some(&stored));
    assert_eq!(after, vec!["dummy=dummy"], "left over: {:?}", after);
}

/* ───────────────────────────── settings_update ───────────────────────────── */

fn post_settings(form: &str, cookies: &[(&str, &str)]) -> LocalResponse<'static> {
    // Leak a client per call: LocalResponse borrows it
    let c: &'static Client = Box::leak(Box::new(client(routes![settings_update])));
    let mut req = c.post("/settings_update").header(ContentType::Form).body(form.to_string());
    for (k, v) in cookies {
        req = req.cookie(Cookie::new(k.to_string(), v.to_string()));
    }
    req.dispatch()
}

#[test]
fn settings_update_sets_cookies_and_redirects_home() {
    let r = post_settings("newtab=on&index=on&theme=dark&lang=sk&loc=sk&js=on&post=on", &[]);
    assert_eq!(r.status(), Status::SeeOther);
    assert_eq!(location(&r), "/");
    for (k, v) in [("newtab", "1"), ("index", "1"), ("theme", "dark"), ("lang", "sk"), ("loc", "sk"), ("js", "1"), ("post", "1")] {
        let ck = cookie(&r, k).unwrap_or_else(|| panic!("{} not set", k));
        assert_eq!(ck.value(), v, "{}", k);
        assert_eq!(ck.max_age(), Some(rocket::time::Duration::days(365)));
    }
}

#[test]
fn settings_update_removes_unchecked_flags() {
    let r = post_settings("", &[("newtab", "1"), ("index", "1"), ("js", "1"), ("post", "1")]);
    for k in ["newtab", "index", "js", "post"] {
        let ck = cookie(&r, k).unwrap_or_else(|| panic!("no removal cookie for {}", k));
        assert!(is_removal(&ck), "{} not removed: {:?}", k, ck);
    }
}

#[test]
fn settings_update_system_theme_removes_cookie() {
    let r = post_settings("theme=system", &[("theme", "dark")]);
    assert!(is_removal(&cookie(&r, "theme").unwrap()));
}

#[test]
fn settings_update_missing_lang_loc_theme_are_untouched() {
    let r = post_settings("", &[("lang", "de"), ("loc", "at"), ("theme", "light")]);
    assert!(cookie(&r, "lang").is_none());
    assert!(cookie(&r, "loc").is_none());
    assert!(cookie(&r, "theme").is_none());
}

/* ───────────────────────────── set_preferences ───────────────────────────── */

fn get_set(query: &str, cookies: &[(&str, &str)]) -> LocalResponse<'static> {
    let c: &'static Client = Box::leak(Box::new(client(routes![set_preferences])));
    let mut req = c.get(format!("/set?{}", query));
    for (k, v) in cookies {
        req = req.cookie(Cookie::new(k.to_string(), v.to_string()));
    }
    req.dispatch()
}

#[test]
fn set_preferences_sets_strict_secure_cookies() {
    let r = get_set("lang=de&loc=at&theme=light&newtab=1&js=1&index=1&post=1", &[]);
    for (k, v) in [("lang", "de"), ("loc", "at"), ("theme", "light"), ("newtab", "1"), ("js", "1"), ("index", "1"), ("post", "1")] {
        let ck = cookie(&r, k).unwrap_or_else(|| panic!("{}", k));
        assert_eq!(ck.value(), v);
        assert_eq!(ck.same_site(), Some(SameSite::Strict));
        assert_eq!(ck.secure(), Some(true));
        assert_eq!(ck.path(), Some("/"));
    }
}

#[test]
fn set_preferences_zero_removes_flags() {
    let r = get_set("newtab=0&js=0&index=0&post=0", &[("newtab", "1"), ("js", "1"), ("index", "1"), ("post", "1")]);
    for k in ["newtab", "js", "index", "post"] {
        assert!(is_removal(&cookie(&r, k).unwrap()), "{}", k);
    }
}

#[test]
fn set_preferences_without_params_changes_nothing() {
    let r = get_set("", &[("lang", "sk")]);
    assert_eq!(location(&r), "/");
    assert!(r.cookies().iter().next().is_none());
}

#[test]
fn set_preferences_redirects_to_local_paths() {
    assert_eq!(location(&get_set("return_to=%2Fsearch%3Fq%3Drust", &[])), "/search?q=rust");
    assert_eq!(location(&get_set("return_to=%2Fsettings", &[])), "/settings");
}

#[test]
fn set_preferences_refuses_absolute_redirects() {
    for target in ["https%3A%2F%2Fevil.example", "evil.example", "javascript%3Aalert(1)"] {
        assert_eq!(location(&get_set(&format!("return_to={}", target), &[])), "/", "{}", target);
    }
}

/// SECURITY (open redirect): "//evil.example" starts with '/' and is accepted,
/// browsers treat it as a protocol relative absolute URL.
#[test]
fn set_preferences_refuses_protocol_relative_redirects() {
    let loc = location(&get_set("return_to=%2F%2Fevil.example%2Fx", &[]));
    assert!(!loc.starts_with("//"), "open redirect to {}", loc);
    let loc = location(&get_set("return_to=%2F%5Cevil.example", &[]));
    assert!(!loc.starts_with("/\\"), "open redirect to {}", loc);
}

/* ───────────────────────────── apply_goggles ───────────────────────────── */

fn apply(query: &str, cookies: &[(&str, &str)]) -> LocalResponse<'static> {
    let c: &'static Client = Box::leak(Box::new(client(routes![apply_goggles])));
    let mut req = c.get(format!("/goggles/apply{}", query));
    for (k, v) in cookies {
        req = req.cookie(Cookie::new(k.to_string(), v.to_string()));
    }
    req.dispatch()
}

#[test]
fn apply_goggles_sets_joined_ids() {
    let r = apply("?ids=1&ids=22&ids=18446744073709551615", &[]);
    assert_eq!(location(&r), "/goggles");
    assert_eq!(cookie(&r, "active_goggles").unwrap().value(), "1,22,18446744073709551615");
}

#[test]
fn apply_goggles_without_ids_removes_cookie() {
    let r = apply("", &[("active_goggles", "1,2")]);
    assert_eq!(location(&r), "/goggles");
    assert!(is_removal(&cookie(&r, "active_goggles").unwrap()));
}

#[test]
fn apply_goggles_rejects_non_numeric_ids() {
    let r = apply("?ids=abc", &[]);
    assert!(cookie(&r, "active_goggles").map_or(true, |c| c.value() != "abc"));
}

/* ───────────────────────────── quick tune ───────────────────────────── */

fn qt_post(form: &str, existing: Option<&str>) -> LocalResponse<'static> {
    let c: &'static Client = Box::leak(Box::new(client(routes![update_qt])));
    let mut req = c.post("/quick_tune_update").header(ContentType::Form).body(form.to_string());
    if let Some(v) = existing {
        req = req.cookie(Cookie::new("prieco_qt_prefs", v.to_string()));
    }
    req.dispatch()
}

fn qt_prefs(r: &LocalResponse<'_>) -> serde_json::Value {
    serde_json::from_str(cookie(r, "prieco_qt_prefs").unwrap().value()).unwrap()
}

#[test]
fn quick_tune_adds_preferences() {
    let r = qt_post("return_url=%2Fsearch&prefs[alpha]=boost&prefs[beta]=downrank&prefs[gamma]=discard", None);
    assert_eq!(location(&r), "/search");
    let v = qt_prefs(&r);
    assert_eq!(v["boost"], serde_json::json!(["alpha"]));
    assert_eq!(v["downrank"], serde_json::json!(["beta"]));
    assert_eq!(v["discard"], serde_json::json!(["gamma"]));
}

#[test]
fn quick_tune_moves_domain_between_lists_without_duplicates() {
    let existing = r#"{"boost":["alpha","keep"],"downrank":[],"discard":["alpha"]}"#;
    let r = qt_post("return_url=%2F&prefs[alpha]=downrank", Some(existing));
    let v = qt_prefs(&r);
    assert_eq!(v["boost"], serde_json::json!(["keep"]));
    assert_eq!(v["downrank"], serde_json::json!(["alpha"]));
    assert_eq!(v["discard"], serde_json::json!([]));
}

#[test]
fn quick_tune_unknown_action_clears_domain() {
    let existing = r#"{"boost":["alpha"],"downrank":[],"discard":[]}"#;
    let v = qt_prefs(&qt_post("return_url=%2F&prefs[alpha]=none", Some(existing)));
    assert_eq!(v["boost"], serde_json::json!([]));
}

#[test]
fn quick_tune_invalid_existing_cookie_starts_fresh() {
    let v = qt_prefs(&qt_post("return_url=%2F&prefs[alpha]=boost", Some("not json")));
    assert_eq!(v["boost"], serde_json::json!(["alpha"]));
}

/// SECURITY (open redirect): return_url is used verbatim as redirect target.
#[test]
fn quick_tune_refuses_off_site_redirects() {
    let r = qt_post("return_url=https%3A%2F%2Fevil.example%2Fphish&prefs[a]=boost", None);
    let loc = location(&r);
    assert!(loc.starts_with('/') && !loc.starts_with("//"), "redirects to {}", loc);
}

#[get("/export")]
fn export_route(jar: &rocket::http::CookieJar<'_>) -> pages::QuickTuneExport {
    export_quick_tune(jar)
}

#[test]
fn quick_tune_export_is_a_valid_goggle_download() {
    let c = client(routes![export_route]);
    let prefs = r#"{"boost":["a.com"],"downrank":["b.com"],"discard":["c.com"]}"#;
    let r = c
        .get("/export")
        .cookie(Cookie::new("prieco_qt_prefs", urlencoding::encode(prefs).into_owned()))
        .dispatch();
    assert_eq!(r.content_type(), Some(ContentType::Plain));
    assert_eq!(
        r.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"my_prieco.goggle\"")
    );
    let text = r.into_string().unwrap();
    assert!(text.contains("$boost=3,site=a.com"));
    assert!(text.contains("$downrank=3,site=b.com"));
    assert!(text.contains("$discard,site=c.com"));

    let parsed = parse_goggle(&text);
    assert_eq!(parsed.name, "My PriEco Goggle");
    assert!(!parsed.public);
    assert_eq!(parsed.rules.boost[&domain_str_to_id("a.com")], 3.0);
    assert_eq!(parsed.rules.downrank[&domain_str_to_id("b.com")], 3.0);
    assert!(parsed.rules.discard.contains(&domain_str_to_id("c.com")));
}

#[test]
fn quick_tune_export_without_cookie_has_only_header() {
    let c = client(routes![export_route]);
    let text = c.get("/export").dispatch().into_string().unwrap();
    assert_eq!(text, "! name: My PriEco Goggle\n! description: Exported Goggle\n! public: false\n\n");
}
