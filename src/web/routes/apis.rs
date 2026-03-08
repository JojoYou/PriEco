/*
  File: web/routes/apis.rs
  Description: Opens up API endpoints

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created: 2026-01-31
  Last Modified: 2026-02-01

  Usage: Call these APIs in browser, your app or culr...
  TODO:
*/

/*
  Import system libraries
*/
use std::{io::Cursor, path::Path};

/*
  Import external libraries
*/
use image::GenericImageView;
use reqwest::Client;
use rocket::{
    State, get,
    http::{ContentType, CookieJar, Status, uri::Host},
    post,
    serde::json::{Json, Value as RocketValue},
};

/*
  Import own libraries
*/
use crate::{
    globals::{ANALYTICS, EmbeddingService, UserAgent},
    web::{
        functions::{general::is_valid_url, search_db},
        routes::pages::ClientIp,
    },
};

/*
  Description: Opens up API that calls PriEco index and returns results in JSON

  Input: API key, language, location, query
  Output: JSON
*/
#[get("/api?<a>&<lang>&<loc>&<q>")]
pub async fn api(
    a: &str,
    lang: &str,
    loc: &str,
    q: &str,
    embedding_service: &State<EmbeddingService>,
) -> Json<Vec<RocketValue>> {
    if !["IWaebywkZHaQikH9YfznSanMS9c2H8dHvAtlDWWzKSfWOu83DdVfidb5khjn"].contains(&a) {
        return Json(vec![]);
    }

    ANALYTICS.record_api_request();

    let (_, results) = search_db::run_json(q, lang, loc, embedding_service).await;
    Json(results)
}

/*
  Description: Proxy remote content, handles GET

  Input: URL, Optional width, Optional height
  Output: Content
*/
#[get("/proxy?<u>&<width>&<height>")]
pub async fn proxy_get(
    u: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    proxy_request(u, None, "GET", width, height).await
}

/*
  Description: Proxy remote content, handles POST

  Input: URL, Optional width, Optional height
  Output: Content
*/
#[post("/proxy?<u>&<width>&<height>", data = "<body>")]
pub async fn proxy_post(
    u: &str,
    body: Vec<u8>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    proxy_request(u, Some(body), "POST", width, height).await
}

/* Helper functions */

/*
  Description: Proxies the contenct based on request type from route

  Input: URL, Optional body (JSON headers in the request), method (GET or POST), Optional width, Optional height
  Output: Content
*/
async fn proxy_request(
    url: &str,
    body: Option<Vec<u8>>,
    method: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    let decoded_url = match urlencoding::decode(url) {
        Ok(url) => url.to_string(),
        Err(_) => return Err(Status::BadRequest),
    };

    if !is_valid_url(&decoded_url) {
        return Err(Status::BadRequest);
    }

    let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36")
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .map_err(|_| Status::InternalServerError)?;

    let request_builder = match method {
        "GET" => client
            .get(&decoded_url)
            .header("Referer", "https://dev.prieco.net/"),
        "POST" => {
            let mut req = client
                .post(&decoded_url)
                .header("Referer", "https://dev.prieco.net/");
            if let Some(body_data) = body {
                req = req
                    .body(body_data)
                    .header("Content-Type", "application/json");
            }
            req
        }
        _ => return Err(Status::MethodNotAllowed),
    };

    let resp = match request_builder.send().await {
        Ok(response) => response,
        Err(e) => {
            println!("Request failed for URL: {}", decoded_url);
            println!("Error: {:?}", e);
            return Err(Status::BadGateway);
        }
    };

    if !resp.status().is_success() {
        println!("Fetched URL: {}", decoded_url);
        println!("Status: {}", resp.status());
        if let Some(ct) = resp.headers().get("content-type") {
            println!("Content-Type: {:?}", ct);
        }
        return Err(Status::NotFound);
    }

    let content_type = if let Some(ct) = resp.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            match ct_str {
                s if s.starts_with("text/javascript") => ContentType::JavaScript,
                s if s.starts_with("application/javascript") => ContentType::JavaScript,
                s if s.starts_with("text/css") => ContentType::CSS,
                s if s.starts_with("text/html") => ContentType::HTML,
                s if s.starts_with("application/json") => ContentType::JSON,
                s if s.starts_with("image/") => {
                    if s.contains("png") {
                        ContentType::PNG
                    } else if s.contains("jpeg") || s.contains("jpg") {
                        ContentType::JPEG
                    } else if s.contains("gif") {
                        ContentType::GIF
                    } else if s.contains("webp") {
                        ContentType::WEBP
                    } else if s.contains("svg") {
                        ContentType::SVG
                    } else {
                        ContentType::Binary
                    }
                }
                _ => ContentType::Binary,
            }
        } else {
            detect_content_type_from_url(&decoded_url)
        }
    } else {
        detect_content_type_from_url(&decoded_url)
    };

    let mut body = match resp.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return Err(Status::InternalServerError),
    };

    if (width.is_some() || height.is_some())
        && (content_type == ContentType::PNG
            || content_type == ContentType::JPEG
            || content_type == ContentType::GIF
            || content_type == ContentType::WEBP
            || content_type == ContentType::SVG)
    {
        body = match resize_image(&body, width, height) {
            Ok(resized) => resized,
            Err(_) => return Err(Status::InternalServerError),
        };
    }

    Ok((content_type, body))
}

/*
  Description: Detects content type from URL

  Input: URL
  Output: ContentType
*/
fn detect_content_type_from_url(url: &str) -> ContentType {
    let path = Path::new(url);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("js") => ContentType::JavaScript,
        Some("css") => ContentType::CSS,
        Some("html") | Some("htm") => ContentType::HTML,
        Some("json") => ContentType::JSON,
        Some("png") => ContentType::PNG,
        Some("jpg") | Some("jpeg") => ContentType::JPEG,
        Some("gif") => ContentType::GIF,
        Some("webp") => ContentType::WEBP,
        Some("svg") => ContentType::SVG,
        Some("pdf") => ContentType::PDF,
        Some("txt") => ContentType::Text,
        _ => ContentType::Binary,
    }
}

/*
  Description: Resizes image based on target width and height

  Input: Image bytes, Optional target width, Optional target height
  Output: Image data
*/
fn resize_image(
    data: &[u8],
    target_width: Option<u32>,
    target_height: Option<u32>,
) -> Result<Vec<u8>, image::ImageError> {
    let img = image::load_from_memory(data)?;

    let (orig_width, orig_height) = img.dimensions();

    // Decide target size while keeping aspect ratio
    let (new_width, new_height) = match (target_width, target_height) {
        (Some(w), None) => {
            let h = (orig_height as f32 * (w as f32 / orig_width as f32)).round() as u32;
            (w, h)
        }
        (None, Some(h)) => {
            let w = (orig_width as f32 * (h as f32 / orig_height as f32)).round() as u32;
            (w, h)
        }
        (Some(w), Some(h)) => (w, h), // caller forces both, aspect ratio may break
        _ => (orig_width, orig_height),
    };

    let resized = img.resize_exact(new_width, new_height, image::imageops::Lanczos3);

    // Encode back to the same format (e.g. JPEG if original was JPEG)
    let mut out = Cursor::new(Vec::new());
    match image::guess_format(data)? {
        image::ImageFormat::Jpeg => resized.write_to(&mut out, image::ImageFormat::Jpeg)?,
        image::ImageFormat::Png => resized.write_to(&mut out, image::ImageFormat::Png)?,
        image::ImageFormat::Gif => resized.write_to(&mut out, image::ImageFormat::Gif)?,
        image::ImageFormat::WebP => resized.write_to(&mut out, image::ImageFormat::WebP)?,
        _ => resized.write_to(&mut out, image::ImageFormat::Png)?,
    }

    Ok(out.into_inner())
}

/*
  Description: PriEco SW Cache version

  Input:
  Output: PriEco SW cache version
*/
#[get("/cache-ver")]
pub fn cache_ver() -> String {
    String::from("0.1.5")
}

/*
  Description: Increment pageview

  Input:
  Output: ok
*/
#[get("/pv")]
pub fn pageview(
    client_ip: ClientIp,
    user_agent: UserAgent<'_>,
    cookie_jar: &CookieJar<'_>,
    host: &Host<'_>,
) -> &'static str {
    ANALYTICS.record_visitor(
        &client_ip.0.to_string(),
        user_agent.0,
        &host.to_string(),
        cookie_jar.get("loc").map(|c| c.value()),
    );
    "ok"
}
