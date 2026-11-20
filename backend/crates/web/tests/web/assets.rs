//! Tests for src/web/routes/assets.rs
//!
//! Template / favicon files are created inside the test sandbox (cwd), never in the repo.

use std::{fs, io::Write};

use prieco_core::STATIC_DIR;
use rocket::http::{ContentType, Status, uri::Host};

use crate::support::{client, init, unique};
use crate::web::routes::assets::{self, osd, robots, script, security};

/* ───────────────────────────── static texts ───────────────────────────── */

#[tokio::test]
async fn security_txt_has_contact_and_expiry() {
    let s = security().await.0;
    assert!(s.contains("Contact: mailto:support@prieco.net"));
    assert!(s.contains("Expires: "));
    let exp = s.lines().find(|l| l.starts_with("Expires: ")).unwrap();
    let date = time::OffsetDateTime::parse(
        &exp["Expires: ".len()..],
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    assert!(
        date > time::OffsetDateTime::now_utc(),
        "security.txt is expired"
    );
}

#[tokio::test]
async fn robots_disallows_search() {
    let r = robots().await.0;
    assert!(r.contains("User-agent: *"));
    assert!(r.contains("Disallow: /search"));
}

#[test]
fn security_and_robots_routes() {
    let c = client(rocket::routes![assets::security, assets::robots]);
    let r = c.get("/.well-known/security.txt").dispatch();
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::Text));
    let r = c.get("/robots.txt").dispatch();
    assert_eq!(r.content_type(), Some(ContentType::Text));
}

/* ───────────────────────────── OpenSearch ───────────────────────────── */

#[test]
fn osd_clearnet() {
    let x = osd(&Host::parse("prieco.net").unwrap()).0;
    assert!(x.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(x.contains("<ShortName>PriEco</ShortName>"));
    assert!(x.contains("template=\"https://prieco.net/search?t=all&amp;q={searchTerms}\""));
    assert!(x.contains("template=\"https://prieco.net/api/om_sug?q={searchTerms}\""));
    assert!(x.contains("<SearchForm>https://prieco.net/</SearchForm>"));
    assert!(!x.contains("onion"));
}

#[test]
fn osd_onion() {
    let x = osd(
        &Host::parse("priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion").unwrap(),
    )
    .0;
    assert!(x.contains("<ShortName>PriEco (Onion)</ShortName>"));
    assert!(x.contains("http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/search?t=all&amp;q={searchTerms}"));
    assert!(!x.contains("https://prieco.net"));
}

#[test]
fn osd_any_other_host_points_to_prieco_net() {
    for h in ["localhost:8000", "127.0.0.1", "my.instance.example"] {
        let x = osd(&Host::parse(h).unwrap()).0;
        assert!(x.contains("https://prieco.net/"), "{}", h);
    }
}

#[test]
fn osd_xml_has_no_unescaped_ampersands() {
    let x = osd(&Host::parse("prieco.net").unwrap()).0;
    for (i, _) in x.match_indices('&') {
        assert!(x[i..].starts_with("&amp;"), "raw & at {}", i);
    }
}

/* ───────────────────────────── script templates ───────────────────────────── */

fn template(content: &str) -> String {
    init();
    fs::create_dir_all(format!("{}/js/hbs", STATIC_DIR)).unwrap();
    let name = format!("{}.js", unique("tpl"));
    fs::write(format!("{}/js/hbs/{}", STATIC_DIR, name), content).unwrap();
    name
}

#[test]
fn script_renders_all_variables() {
    let name = template("q={{q}}|t={{t}}|lang={{lang}}|loc={{loc}}|dec={{dec}}");
    let out = script(
        &name,
        Some("all"),
        Some("sk"),
        Some("sk"),
        Some("rust lang"),
        Some(false),
    )
    .0;
    assert_eq!(out, "q=rust%20lang|t=all|lang=sk|loc=sk|dec=false");
}

#[test]
fn script_defaults() {
    let name = template("[{{q}}][{{t}}][{{lang}}][{{loc}}][{{dec}}]");
    let out = script(&name, None, None, None, None, None).0;
    assert_eq!(out, "[][][][][true]");
}

#[test]
fn script_query_is_url_encoded_so_it_cannot_break_out_of_strings() {
    let name = template("var q = '{{q}}';");
    let out = script(&name, None, None, None, Some("'</script><b>\"&"), None).0;
    assert_eq!(out, "var q = '%27%3C%2Fscript%3E%3Cb%3E%22%26';");
}

#[test]
fn script_other_values_are_html_escaped() {
    let name = template("{{t}}|{{lang}}");
    let out = script(&name, Some("<img>"), Some("\"x\""), None, None, None).0;
    assert!(!out.contains('<') && !out.contains('"'), "{}", out);
}

#[test]
fn script_rejects_path_traversal() {
    init();
    for n in ["../secret.js", "..", "a/b.js", "/etc/passwd", "x..y"] {
        assert_eq!(script(n, None, None, None, None, None).0, "", "{:?}", n);
    }
}

#[test]
fn script_missing_template_reports_error() {
    init();
    let out = script("does_not_exist.js", None, None, None, None, None).0;
    assert_eq!(
        out,
        format!(
            "console.error(\"Template {}/js/hbs/does_not_exist.js not found\");",
            STATIC_DIR
        )
    );
}

#[test]
fn script_invalid_template_reports_error() {
    let name = template("{{#if}}unterminated");
    let out = script(&name, None, None, None, None, None).0;
    assert!(out.starts_with("console.error("), "{}", out);
}

/// SECURITY (reflected JS injection): the requested file name is pasted into the
/// "not found" JavaScript. A name like `x');alert(1);(` produces executable code
/// served as application/javascript from PriEco's origin.
#[test]
fn script_name_cannot_inject_javascript() {
    init();
    let name = "x');alert(document.domain);(\"+alert(1)+\"";
    let out = script(name, None, None, None, None, None).0;
    // The whole message must be ONE string literal: console.error(<json string>);
    let arg = out
        .strip_prefix("console.error(")
        .and_then(|s| s.strip_suffix(");"))
        .unwrap_or_else(|| panic!("unexpected output {}", out));
    let msg: String = serde_json::from_str(arg)
        .unwrap_or_else(|_| panic!("argument is not a single string literal: {}", out));
    assert!(msg.contains(name), "{}", msg);
}

#[test]
fn script_route_content_type() {
    let name = template("var a = 1;");
    let c = client(rocket::routes![assets::script]);
    let r = c.get(format!("/static/js/hbs/{}?t=all", name)).dispatch();
    assert_eq!(r.status(), Status::Ok);
    assert_eq!(r.content_type(), Some(ContentType::JavaScript));
    assert_eq!(r.into_string().unwrap(), "var a = 1;");
}

/* ───────────────────────────── service worker / unduck ───────────────────────────── */

#[test]
fn sw_and_unduck_are_served_when_present() {
    init();
    let c = client(rocket::routes![assets::sw_js, assets::unduck_js]);
    // the files never exist before this test in the sandbox
    assert_eq!(c.get("/sw.js").dispatch().status(), Status::NotFound);
    assert_eq!(c.get("/unduck.js").dispatch().status(), Status::NotFound);

    fs::create_dir_all(format!("{}/js/unduck", STATIC_DIR)).unwrap();
    fs::write(format!("{}/js/unduck/sw.js", STATIC_DIR), "self.x=1;").unwrap();
    fs::write(format!("{}/js/unduck/unduck.js", STATIC_DIR), "var u=2;").unwrap();
    let r = c.get("/sw.js").dispatch();
    assert_eq!(r.content_type(), Some(ContentType::JavaScript));
    assert_eq!(r.into_string().unwrap(), "self.x=1;");
    assert_eq!(
        c.get("/unduck.js").dispatch().into_string().unwrap(),
        "var u=2;"
    );
}
