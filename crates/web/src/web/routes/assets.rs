//!  File: web/routes/assets.rs
//!  Description: Handles PriEco's static assets
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-01-31
//!  Last Modified: 2026-02-01
//!
//!  Usage: Call routes to get PriEco's static assets
//!  TODO:

/*
  Import system libraries
*/
use std::{
    io::{Cursor, Read},
    path::Path,
};

/*
  Import external libraries
*/
use rocket::{
    Request, Response,
    fs::NamedFile,
    get,
    http::{ContentType, Status, uri::Host},
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
pub struct DecompressedImage {
    data: Vec<u8>,
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

/// Description: PriEco service worker. Had to be moved to root
/// 
/// Input:
/// Output: Service worker JS
#[get("/sw.js")]
pub async fn sw_js() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/js/unduck/sw.js"))
        .await
        .ok()
}

/*
  Description: PriEco unduck. Had to be moved to root

  Input:
  Output: Unduck JS
*/
#[get("/unduck.js")]
pub async fn unduck_js() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/js/unduck/unduck.js"))
        .await
        .ok()
}

/*
  Description: Send here any vulnerabilities you find. I don't pay for now but appreciate your help

  Input:
  Output: Information
*/
#[get("/.well-known/security.txt")]
pub async fn security() -> RawText<&'static str> {
    RawText(
        "Contact: mailto:support@prieco.net\nExpires: 2027-04-16T12:00:00.000Z\nPreferred-Languages: en,sk,cs",
    )
}

/*
  Description: Communicate to bots which pages they can access

  Input:
  Output: Information
*/
#[get("/robots.txt")]
pub async fn robots() -> RawText<&'static str> {
    RawText("User-agent: *\nDisallow: /search")
}

/*
  Description: File required for Firefox to set PriEco as default search engine

  Input:
  Output: PriEco as your default search engine
*/
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
  <SearchForm>{base}</SearchForm>
</OpenSearchDescription>"#,
        short = short_name,
        search = search_url,
        base = base_url
    );

    RawXml(xml)
}

/*
  Description: Assemble JavaScript templates

  Input: JS template name, Optional type, Optional language, Optional location, Optional query
  Output:
*/
#[get("/static/js/hbs/<script_name>?<t>&<lang>&<loc>&<q>")]
pub fn script(
    script_name: &str,
    t: Option<&str>,
    lang: Option<&str>,
    loc: Option<&str>,
    q: Option<&str>,
) -> RawJavaScript<String> {
    if script_name.contains("..") || script_name.contains('/') {
        return RawJavaScript(String::new());
    }

    let mut hbs = Handlebars::new();

    let template_path = format!("static/js/hbs/{}", script_name);
    let template_content = std::fs::read_to_string(&template_path)
        .unwrap_or_else(|_| format!("console.error('Template {} not found');", template_path));

    if let Err(e) = hbs.register_template_string("js_template", &template_content) {
        return RawJavaScript(format!(
            "console.error('Failed to register template: {}');",
            e
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
            }),
        )
        .unwrap_or_else(|_| format!("console.error('Failed to render {}');", script_name)),
    )
}

/*
  Description: Serves own PriEco result favicons

  Input: Favicon id
  Output: Favicon as image
*/
#[rocket::get("/static/prieco_favicons/<filename>")]
pub async fn favicon(filename: String) -> Result<DecompressedImage, Status> {
    if filename.contains("..") || filename.contains("/") {
        return Err(Status::BadRequest);
    }

    let file_path = format!("static/prieco_favicons/{}.br", filename);
    let path = Path::new(&file_path);

    if !path.exists() {
        return Err(Status::NotFound);
    }

    let compressed_data = match std::fs::read(path) {
        Ok(data) => data,
        Err(_) => return Err(Status::InternalServerError),
    };

    let mut decompressed = Vec::new();
    let mut decoder = brotli::Decompressor::new(compressed_data.as_slice(), 4096);

    match decoder.read_to_end(&mut decompressed) {
        Ok(_) => {
            let content_type = detect_image_type(&decompressed);
            Ok(DecompressedImage {
                data: decompressed,
                content_type,
            })
        }
        Err(_) => Err(Status::InternalServerError),
    }
}

/* Helper functions */

/*
  Description: Use image bytes to detect image type

  Input: Image bytes
  Output: Possible image type
*/
fn detect_image_type(data: &[u8]) -> ContentType {
    if data.len() < 8 {
        return ContentType::Binary;
    }

    // Check file signatures
    match &data[0..8] {
        // PNG: 89 50 4E 47 0D 0A 1A 0A
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => ContentType::PNG,
        // JPEG: FF D8 FF
        [0xFF, 0xD8, 0xFF, ..] => ContentType::JPEG,
        // GIF87a: 47 49 46 38 37 61
        [0x47, 0x49, 0x46, 0x38, 0x37, 0x61, ..] => ContentType::GIF,
        // GIF89a: 47 49 46 38 39 61
        [0x47, 0x49, 0x46, 0x38, 0x39, 0x61, ..] => ContentType::GIF,
        // SVG: Check if it starts with < and contains "svg"
        [0x3C, ..] => {
            if let Ok(text) = std::str::from_utf8(&data[0..100.min(data.len())]) {
                if text.to_lowercase().contains("svg") {
                    return ContentType::SVG;
                }
            }
            ContentType::Binary
        }
        // ICO: 00 00 01 00
        [0x00, 0x00, 0x01, 0x00, ..] => ContentType::Icon,
        // WebP: RIFF....WEBP
        [0x52, 0x49, 0x46, 0x46, _, _, _, _] if data.len() >= 12 && &data[8..12] == b"WEBP" => {
            ContentType::new("image", "webp")
        }
        _ => ContentType::Binary,
    }
}
