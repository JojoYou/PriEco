//! # Assets
//!
//! Files that have to be on the root or are built per request.
//!
//! * [sw_js], [unduck_js] - Service worker and unduck, had to be moved to root.
//! * [security], [robots] - '.well-known/security.txt' and 'robots.txt'.
//! * [osd] - OpenSearch description, for Firefox default search engine.
//! * [script] - JavaScript built from Handlebars templates.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-01-31
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{io::Cursor, path::Path};

use prieco_core::STATIC_DIR;
/*
  Import external libraries
*/
use rocket::{
    Request, Response,
    fs::NamedFile,
    get,
    http::{ContentType, uri::Host},
    response::{
        self, Responder,
        content::{RawJavaScript, RawText, RawXml},
    },
};
use rocket_dyn_templates::handlebars::Handlebars;
use urlencoding::encode;

/*
Structures
*/
/// Image response from memory.
pub struct DecompressedImage {
    /// Image bytes.
    data: Vec<u8>,
    /// Image type.
    content_type: ContentType,
}

impl<'r> Responder<'r, 'static> for DecompressedImage {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(self.content_type)
            .sized_body(self.data.len(), Cursor::new(self.data))
            .ok()
    }
}

/// PriEco service worker, `GET /sw.js`. Had to be moved to root.
#[get("/sw.js")]
pub async fn sw_js() -> Option<NamedFile> {
    NamedFile::open(Path::new(&format!("{}/js/unduck/sw.js", STATIC_DIR)))
        .await
        .ok()
}

/// PriEco unduck, `GET /unduck.js`. Had to be moved to root.
#[get("/unduck.js")]
pub async fn unduck_js() -> Option<NamedFile> {
    NamedFile::open(Path::new(&format!("{}/js/unduck/unduck.js", STATIC_DIR)))
        .await
        .ok()
}

/// `GET /.well-known/security.txt`
///
/// Send here any vulnerabilities you find. I don't pay for now but appreciate your help.
#[get("/.well-known/security.txt")]
pub async fn security() -> RawText<&'static str> {
    RawText(
        "Contact: mailto:support@prieco.net\nExpires: 2027-04-16T12:00:00.000Z\nPreferred-Languages: en,sk,cs",
    )
}

/// `GET /robots.txt`. Bots can access everything except '/search'.
#[get("/robots.txt")]
pub async fn robots() -> RawText<&'static str> {
    RawText("User-agent: *\nDisallow: /search")
}

/// OpenSearch description, `GET /osd.xml`.
///
/// File required for Firefox to set PriEco as default search engine.
/// On '.onion' host it points to the onion address.
#[get("/osd.xml")]
pub fn osd(host: &Host) -> RawXml<String> {
    let (short_name, base_url, search_url) = if host.domain().as_str().ends_with(".onion") {
        (
            "PriEco (Onion)",
            "http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/",
            "http://priecovk7jsuh3tvkh62c6j4oep3l5bldigpzmay26rdpqz357t5dmad.onion/search?t=all&amp;q={searchTerms}",
        )
    } else {
        (
            "PriEco",
            "https://prieco.net/",
            "https://prieco.net/search?t=all&amp;q={searchTerms}",
        )
    };

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>{short}</ShortName>
  <Description>Search Privately, Securely and EcoFriendly</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <Url type="text/html" template="{search}"/>
  <Url type="application/x-suggestions+json" template="{base}api/om_sug?q={{searchTerms}}"/>
  <SearchForm>{base}</SearchForm>
</OpenSearchDescription>"#,
        short = short_name,
        search = search_url,
        base = base_url
    );

    RawXml(xml)
}

/// JavaScript from template 'static/js/hbs/`script_name`'. `GET /static/js/hbs/<script_name>`
///
/// Names with '..' or '/' give empty script.
///
/// # Arguments
///
/// * `script_name` - Template file.
/// * `t` - Search type.
/// * `lang` - Language.
/// * `loc` - Location.
/// * `q` - Query, URL encoded in output.
/// * `dec` - Decentralized search, default true.
///
/// # Returns
///
/// Rendered script, or `console.error` with the reason.
#[get("/static/js/hbs/<script_name>?<t>&<lang>&<loc>&<q>&<dec>")]
pub fn script(
    script_name: &str,
    t: Option<&str>,
    lang: Option<&str>,
    loc: Option<&str>,
    q: Option<&str>,
    dec: Option<bool>,
) -> RawJavaScript<String> {
    if script_name.contains("..") || script_name.contains('/') {
        return RawJavaScript(String::new());
    }

    let mut hbs = Handlebars::new();

    let template_path = format!("{}/js/hbs/{}", STATIC_DIR, script_name);
    let template_content = std::fs::read_to_string(&template_path).unwrap_or_else(|_| {
        format!(
            "console.error({});",
            serde_json::to_string(&format!("Template {} not found", template_path)).unwrap()
        )
    });

    if let Err(e) = hbs.register_template_string("js_template", &template_content) {
        return RawJavaScript(format!(
            "console.error({});",
            serde_json::to_string(&format!("Failed to register template: {}", e)).unwrap()
        ));
    }

    RawJavaScript(
        hbs.render(
            "js_template",
            &serde_json::json!({
                "q": encode(q.unwrap_or_default()).into_owned(),
                "t": t.unwrap_or_default(),
                "lang": lang.unwrap_or_default(),
                "loc": loc.unwrap_or_default(),
                "dec": dec.unwrap_or(true),
            }),
        )
        .unwrap_or_else(|_| format!("console.error('Failed to render {}');", script_name)),
    )
}
