//!  File: web/routes/apis.rs
//!  Description: Opens up API endpoints
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-01-31
//!  Last Modified: 2026-02-01
//!
//!  Usage: Call these APIs in browser, your app or culr...
//!  TODO:

/*
  Import system libraries
*/
use std::{
    fs::read_to_string,
    io::Cursor,
    net::{IpAddr, ToSocketAddrs},
    path::Path,
};

/*
  Import external libraries
*/
use image::GenericImageView;
use rocket::{
    FromForm, State,
    form::Form,
    get,
    http::{ContentType, CookieJar, Status, uri::Host},
    post,
    response::Redirect,
    serde::json::{Json, Value as RocketValue},
};
use serde_json::json;
use url::Url;

/*
  Import own libraries
*/
use crate::web::{
    functions::{
        general::is_valid_url,
        ranking::goggles::{get_goggle_ids, load_goggles},
        search_db,
    },
    routes::pages::ClientIp,
};
use prieco_core::{
    CLIENT, PROXY_CLIENT, colors,
    globals::{ANALYTICS, EmbeddingService, UserAgent},
};

/// Description: Opens up API that calls PriEco index and returns results in JSON
///
/// Input: API key, language, location, query
/// Output: JSON
#[get("/api?<a>&<lang>&<loc>&<q>&<goggles>")]
pub async fn api(
    a: &str,
    lang: &str,
    loc: &str,
    q: &str,
    goggles: Option<&str>,

    embedding_service: &State<EmbeddingService>,
) -> Json<Vec<RocketValue>> {
    let resulti_api_key = match std::env::var("RESULTI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: RESULTI_API_KEY is missing!");

            return Json(vec![]);
        }
    };

    let polar_api_key = match std::env::var("POLAR_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: POLAR_API_KEY is missing!");

            return Json(vec![]);
        }
    };
    // Uruky
    let uruky_api_key = match std::env::var("URUKY_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: URUKY_API_KEY is missing!");

            return Json(vec![]);
        }
    };
    let uruky_id = match std::env::var("URUKY_ID") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: URUKY_ID is missing!");

            return Json(vec![]);
        }
    };

    if ![&resulti_api_key, &uruky_api_key].contains(&&a.to_string()) {
        return Json(vec![]);
    }

    ANALYTICS.record_api_request();

    let active_goggles = load_goggles(&get_goggle_ids(goggles, None));

    let full_results =
        search_db::run_json(q, lang, loc, embedding_service, active_goggles, false).await;

    let results: Vec<serde_json::Value> = full_results
        .into_iter()
        .map(|res| {
            json!({
                "url": res["url"],
                "title": res["title"],
                "description": res["description"],
                "lang": res["lang"],
                "loc": res["loc"],
                "safe_s": res["safe_s"],
                "image": res["image"]
            })
        })
        .collect();

    if !results.is_empty() && a == uruky_api_key {
        if let Err(e) = CLIENT
            .post("https://api.polar.sh/v1/events/ingest")
            .header("Authorization", format!("Bearer {}", polar_api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "events": [
                    {
                        "name": "api_call",
                        "external_customer_id": uruky_id
                    }
                ]
            }))
            .send()
            .await
        {
            println!("{}API charging failed!{} {}", colors::RED, colors::RESET, e);
        }
    }

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
    let decoded_url = match urlencoding::decode(u) {
        Ok(dec) => dec.into_owned(),
        Err(_) => return Err(Status::BadRequest),
    };

    let url = match Url::parse(&decoded_url) {
        Ok(ur) => ur,
        Err(_) => return Err(Status::BadRequest),
    };

    // URL check
    if bad_url(&url) {
        return Err(Status::Forbidden);
    }

    // File type
    let (content_type, body) = proxy_request(&url, None, "GET", width, height).await?;

    if allowed_type(&url, &content_type) {
        Ok((content_type, body))
    } else {
        Err(Status::UnsupportedMediaType)
    }
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
    let decoded_url = match urlencoding::decode(u) {
        Ok(dec) => dec.into_owned(),
        Err(_) => return Err(Status::BadRequest),
    };

    let url = match Url::parse(&decoded_url) {
        Ok(ur) => ur,
        Err(_) => return Err(Status::BadRequest),
    };

    // URL check
    if bad_url(&url) {
        return Err(Status::Forbidden);
    }

    // File type
    let (content_type, body) = proxy_request(&url, Some(body), "POST", width, height).await?;

    if allowed_type(&url, &content_type) {
        Ok((content_type, body))
    } else {
        Err(Status::UnsupportedMediaType)
    }
}

/* Helper functions */
/*
  Description: Validates URL for Proxy for security reasons

  Input: URL
  Output: If URL is internal
*/
fn bad_url(url: &Url) -> bool {
    if !is_valid_url(url.as_str()) {
        return true;
    }

    if url.username() != "" || url.password().is_some() {
        return true;
    }

    let host_str = match url.host_str() {
        Some(h) => h,
        None => return true,
    };
    let port = url.port_or_known_default().unwrap_or(80);

    // DNS
    let addrs = format!("{}:{}", host_str, port).to_socket_addrs();
    let mut addrs_iter = match addrs {
        Ok(a) => a,
        Err(_) => return true,
    };

    // Get first IP
    let ip = match addrs_iter.next() {
        Some(socket) => socket.ip(),
        None => return true,
    };

    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_documentation()
                || ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.to_ipv4_mapped().is_some()
                || (ipv6.segments()[0] & 0xfe00) == 0xfc00
                || (ipv6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/*
  Description: Validates Proxied content, allowed are all images and (CSS,JS) only from SearchExpander

  Input: URL, Content type
  Output: If type is allowed
*/
fn allowed_type(url: &Url, content_type: &ContentType) -> bool {
    let top = content_type.top().as_str(); // "image", "text", "application"
    let sub = content_type.sub().as_str(); // "png", "css", "javascript"

    let allowed_data = (top == "text" && sub == "css")
        || (top == "text" && sub == "javascript")
        || (top == "application" && sub == "javascript")
        || (top == "application" && sub == "x-javascript")
        || (top == "application" && sub == "json");

    let allowed_domain = match url.domain() {
        Some(d) => {
            d == "searchexpander.com"
                || d.ends_with(".searchexpander.com")
                || d == "duckduckgo.com"
                || d.ends_with(".duckduckgo.com")
        }
        None => false,
    };

    // Images are allowed
    if top == "image" {
        true
    }
    // CSS and JS only Search Expander
    else if allowed_data && allowed_domain {
        true
    }
    // Reject the rest
    else {
        false
    }
}

/*
  Description: Proxies the contenct based on request type from route

  Input: URL, Optional body (JSON headers in the request), method (GET or POST), Optional width, Optional height
  Output: Content
*/
async fn proxy_request(
    url: &Url,
    body: Option<Vec<u8>>,
    method: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(ContentType, Vec<u8>), Status> {
    let mut target_url = url.clone();
    if target_url
        .domain()
        .map_or(false, |d| d.contains("searchexpander.com"))
    {
        target_url.query_pairs_mut().clear().extend_pairs(
            url.query_pairs()
                .filter(|(k, _)| k != "width" && k != "height"),
        );
    }

    let mut current_url = target_url;
    let mut current_method = method.to_string();
    let mut current_body = body.clone();
    let mut redirects_left = 2;

    let mut response;

    loop {
        let request_builder = match current_method.as_str() {
            "GET" => PROXY_CLIENT
                .get(current_url.as_str())
                .header("Referer", "https://dev.prieco.net/"),
            "POST" => {
                let mut req = PROXY_CLIENT
                    .post(current_url.as_str())
                    .header("Referer", "https://dev.prieco.net/");

                if let Some(ref body_data) = current_body {
                    req = req
                        .body(body_data.clone())
                        .header("Content-Type", "application/json");
                }
                req
            }
            _ => return Err(Status::MethodNotAllowed),
        };

        response = match request_builder.send().await {
            Ok(response) => response,
            Err(_) => {
                return Err(Status::BadGateway);
            }
        };

        if response.status().is_redirection() {
            if redirects_left == 0 {
                return Err(Status::BadGateway);
            }
            if let Some(loc) = response.headers().get("location") {
                if let Ok(loc_str) = loc.to_str() {
                    current_url = match current_url.join(loc_str) {
                        Ok(u) => u,
                        Err(_) => return Err(Status::BadGateway),
                    };

                    if bad_url(&current_url) {
                        return Err(Status::Forbidden);
                    }

                    if response.status().as_u16() == 303
                        || ((response.status().as_u16() == 301
                            || response.status().as_u16() == 302)
                            && current_method == "POST")
                    {
                        current_method = "GET".to_string();
                        current_body = None;
                    }

                    redirects_left -= 1;
                    continue;
                }
            }

            // Broken redirect
            break;
        }

        // 200, 404...
        break;
    }

    if !response.status().is_success() {
        let fallback_svg = std::fs::read("static/img/icon/image.svg").unwrap_or_default();
        return Ok((ContentType::SVG, fallback_svg));
    }

    let content_type = if let Some(ct) = response.headers().get("content-type") {
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
                    } else if s.contains("icon") || s.contains("ico") {
                        ContentType::Icon
                    } else {
                        ContentType::Binary
                    }
                }
                _ => ContentType::Binary,
            }
        } else {
            detect_content_type_from_url(url)
        }
    } else {
        detect_content_type_from_url(url)
    };

    if response.content_length().unwrap_or(0) > 10 * 1024 * 1024 {
        return Err(Status::PayloadTooLarge);
    }

    let mut body = match response.bytes().await {
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
fn detect_content_type_from_url(url: &Url) -> ContentType {
    let path = Path::new(url.as_str());
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
    read_to_string("cache_version.txt")
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|_| String::from("unknown"))
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

// Sends to Roman Láncoš a signal message with message
#[derive(FromForm)]
pub struct RoadmapFeedback<'r> {
    pub message: &'r str,
    pub return_path: &'r str,
}

#[post("/submit_msg", data = "<feedback>")]
pub async fn send_signal(feedback: Form<RoadmapFeedback<'_>>) -> Redirect {
    let signal_bot_number = match std::env::var("SIGNAL_BOT_NUMBER") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: SIGNAL_BOT_NUMBER is missing!");

            return Redirect::to(feedback.return_path.to_string());
        }
    };

    let signal_recipient_number = match std::env::var("SIGNAL_RECIPIENT_NUMBER") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: SIGNAL_RECIPIENT_NUMBER is missing!");

            return Redirect::to(feedback.return_path.to_string());
        }
    };

    let payload = serde_json::json!({
        "message": feedback.message,
        "number": signal_bot_number,
        "recipients": [signal_recipient_number],
        "text_mode": "styled",
    });

    match CLIENT
        .post("http://localhost:8071/v2/send")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            println!("Message successfully forwarded to Signal!");
        }
        Ok(response) => {
            let status_code = response.status();
            let error_text = response.text().await.unwrap_or_default();
            println!("Signal API Error ({}): {}", status_code, error_text);
        }
        Err(e) => {
            println!("Failed to send message to Signal (Timeout/Network): {}", e);
        }
    }

    Redirect::to(feedback.return_path.to_string())
}
