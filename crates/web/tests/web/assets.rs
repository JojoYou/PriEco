//! Tests for src/web/routes/assets.rs
//!
//! Template / favicon files are created inside the test sandbox (cwd), never in the repo.

use std::{fs, io::Write};

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
    let date = chrono::DateTime::parse_from_rfc3339(&exp["Expires: ".len()..]).unwrap();
    assert!(date > chrono::Utc::now(), "security.txt is expired");
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
    let x = osd(&Host::parse("priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion").unwrap()).0;
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
    fs::create_dir_all("static/js/hbs").unwrap();
    let name = format!("{}.js", unique("tpl"));
    fs::write(format!("static/js/hbs/{}", name), content).unwrap();
    name
}

#[test]
fn script_renders_all_variables() {
    let name = template("q={{q}}|t={{t}}|lang={{lang}}|loc={{loc}}|dec={{dec}}");
    let out = script(&name, Some("all"), Some("sk"), Some("sk"), Some("rust lang"), Some(false)).0;
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
    assert_eq!(out, "console.error('Template static/js/hbs/does_not_exist.js not found');");
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
    let out = script("x');alert(document.domain);(", None, None, None, None, None).0;
    assert!(!out.contains("');alert(document.domain);("), "{}", out);
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

    fs::create_dir_all("static/js/unduck").unwrap();
    fs::write("static/js/unduck/sw.js", "self.x=1;").unwrap();
    fs::write("static/js/unduck/unduck.js", "var u=2;").unwrap();
    let r = c.get("/sw.js").dispatch();
    assert_eq!(r.content_type(), Some(ContentType::JavaScript));
    assert_eq!(r.into_string().unwrap(), "self.x=1;");
    assert_eq!(c.get("/unduck.js").dispatch().into_string().unwrap(), "var u=2;");
}

/* ───────────────────────────── favicons ───────────────────────────── */

fn brotli(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut w = ::brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
        w.write_all(data).unwrap();
    }
    out
}

fn favicon_file(raw: &[u8]) -> String {
    init();
    fs::create_dir_all("static/prieco_favicons").unwrap();
    let name = unique("fav");
    fs::write(format!("static/prieco_favicons/{}.br", name), brotli(raw)).unwrap();
    name
}

fn fetch_favicon(name: &str) -> (Status, Option<ContentType>, Vec<u8>) {
    let c = client(rocket::routes![assets::favicon]);
    let r = c.get(format!("/static/prieco_favicons/{}", name)).dispatch();
    (r.status(), r.content_type(), r.into_bytes().unwrap_or_default())
}

#[test]
fn favicon_is_decompressed() {
    let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
    let (s, ct, body) = fetch_favicon(&favicon_file(&png));
    assert_eq!(s, Status::Ok);
    assert_eq!(ct, Some(ContentType::PNG));
    assert_eq!(body, png);
}

#[test]
fn favicon_type_detection() {
    let mut webp = b"RIFF\x10\x00\x00\x00WEBPVP8 ".to_vec();
    webp.extend_from_slice(&[0; 8]);
    let cases: Vec<(Vec<u8>, ContentType)> = vec![
        (vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0], ContentType::JPEG),
        (b"GIF87a\x01\x00\x01\x00".to_vec(), ContentType::GIF),
        (b"GIF89a\x01\x00\x01\x00".to_vec(), ContentType::GIF),
        (b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec(), ContentType::SVG),
        (b"<SVG></SVG>....".to_vec(), ContentType::SVG),
        (vec![0x00, 0x00, 0x01, 0x00, 1, 0, 16, 16], ContentType::Icon),
        (webp, ContentType::new("image", "webp")),
        (b"<html><body>hi</body></html>".to_vec(), ContentType::Binary),
        (b"RIFF\x10\x00\x00\x00WAVEfmt ".to_vec(), ContentType::Binary),
        (vec![1, 2, 3, 4, 5, 6, 7, 8, 9], ContentType::Binary),
        (vec![0x89, 0x50, 0x4E], ContentType::Binary), // shorter than 8 bytes
    ];
    for (raw, expected) in cases {
        let (s, ct, _) = fetch_favicon(&favicon_file(&raw));
        assert_eq!(s, Status::Ok);
        assert_eq!(ct, Some(expected.clone()), "{:?}", &raw[..raw.len().min(8)]);
    }
}

#[test]
fn favicon_svg_marker_must_be_in_first_100_bytes() {
    let mut late = b"<?xml version=\"1.0\"?><!--".to_vec();
    late.extend(std::iter::repeat(b'x').take(200));
    late.extend_from_slice(b"--><svg></svg>");
    let (_, ct, _) = fetch_favicon(&favicon_file(&late));
    assert_eq!(ct, Some(ContentType::Binary));
}

#[test]
fn favicon_svg_with_multibyte_char_at_byte_100_is_not_misdetected() {
    // byte 100 cuts a 2-byte char -> from_utf8 fails -> Binary (not a panic)
    let mut data = b"<svg".to_vec();
    data.extend(std::iter::repeat(b'a').take(95));
    data.extend("ž".as_bytes());
    let (s, _, _) = fetch_favicon(&favicon_file(&data));
    assert_eq!(s, Status::Ok);
}

#[test]
fn favicon_missing_is_not_found() {
    init();
    assert_eq!(fetch_favicon("definitely_missing_icon").0, Status::NotFound);
}

#[test]
fn favicon_traversal_is_bad_request() {
    init();
    assert_eq!(fetch_favicon("a..b").0, Status::BadRequest);
    assert_eq!(fetch_favicon("%2E%2E%2Fsecret").0, Status::BadRequest);
}

#[test]
fn favicon_corrupt_brotli_is_server_error() {
    init();
    fs::create_dir_all("static/prieco_favicons").unwrap();
    let name = unique("corrupt");
    fs::write(format!("static/prieco_favicons/{}.br", name), b"\xff\xff\xff\xffnot brotli").unwrap();
    assert_eq!(fetch_favicon(&name).0, Status::InternalServerError);
}

#[test]
fn favicon_empty_file_is_binary() {
    init();
    fs::create_dir_all("static/prieco_favicons").unwrap();
    let name = unique("empty");
    fs::write(format!("static/prieco_favicons/{}.br", name), brotli(b"")).unwrap();
    let (s, ct, body) = fetch_favicon(&name);
    assert_eq!(s, Status::Ok);
    assert_eq!(ct, Some(ContentType::Binary));
    assert!(body.is_empty());
}
