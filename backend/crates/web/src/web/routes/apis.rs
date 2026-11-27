//! # APIs
//!
//! JSON and helper endpoints. Call them in browser, your app or curl.
//!
//! * [**Search API**:][api] `GET /api`, results from PriEco index.
//!     * Free: 25 searches a day per IP.
//!     * Paid: API key with credits, bought on Polar, see [manage][crate::web::functions::api::manage].
//! * [**Proxy**:][proxy_get] `GET`/`POST /proxy`, images and Search Expander files without leaking user IP.
//! * [**Suggestions**:][om_sug] `GET /api/om_sug`, browser omnibox suggestions.
//! * [**Analytics**:][pageview] `GET /pv`, anonymous page view.
//! * [**Feedback**:][send_signal] `POST /submit_msg`, roadmap message to Signal.
//! * [**Cache version**:][cache_ver] `GET /cache-ver`.
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
use std::{
    fs::read_to_string,
    net::{IpAddr, ToSocketAddrs},
    path::Path,
};

/*
  Import external libraries
*/
use rocket::{
    FromForm, State,
    form::Form,
    get,
    http::{ContentType, CookieJar, Status, uri::Host},
    post,
    response::Redirect,
    serde::json::{Json, Value as RocketValue},
};
use serde::Deserialize;
use serde_json::json;
use url::Url;

/*
  Import own libraries
*/
use crate::web::{
    functions::{
        api::manage::{API_BILLING_QUEUE, free, paid},
        general::is_valid_url,
        ranking::goggles::{get_goggle_ids, load_goggles},
        search_db,
    },
    routes::pages::ClientIp,
};
use prieco_core::{
    CACHE_VERSION_FILE, CLIENT, PRIECO_INDEX, PROXY_CLIENT, STATIC_DIR, colors,
    globals::{ANALYTICS, EmbeddingService, UserAgent},
    local_redirect,
};

/// Where API credits can be bought.
const POLAR_URL: &str = "https://buy.polar.sh/polar_cl_oj0yTj0z4isxyAzUEg33xBjerkhrJpIhclrHU484jSE";

/// Query of [api].
#[derive(FromForm)]
pub struct APIParams<'r> {
    /// API key. Missing or empty is a free request.
    pub a: Option<&'r str>,
    /// Language code or 'all'.
    pub lang: &'r str,
    /// Country code or 'all'.
    pub loc: &'r str,
    /// Query.
    pub q: &'r str,
    /// Goggle ids.
    pub goggles: Option<&'r str>,
    /// Ask other nodes too, default true.
    pub dec: Option<bool>,
    /// Add page content (first 500 characters), default false.
    pub content: Option<bool>,
}
/// Searches PriEco index and returns results in JSON. `GET /api?q=&lang=&loc=`
///
/// Every result has url, title, description, lang, loc, safe_s, image and optional content.
/// Paid requests are billed only when there are results.
///
/// # Arguments
///
/// * `params` - [APIParams].
///
/// # Returns
///
/// Max 20 results. Empty when API env keys are missing.
///
/// # Errors
///
/// * 429 when free limit is reached.
/// * 402 when API key is invalid or has no credits.
///
/// Both with `buy` link.
#[get("/api?<params..>")]
pub async fn api(
    params: APIParams<'_>,
    client_ip: ClientIp,
    embedding_service: &State<EmbeddingService>,
) -> Result<Json<Vec<RocketValue>>, (Status, Json<RocketValue>)> {
    // Get env data
    let polar_api_key = match std::env::var("POLAR_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: POLAR_API_KEY is missing!");
            return Ok(Json(vec![]));
        }
    };
    let resulti_api_key = match std::env::var("RESULTI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: RESULTI_API_KEY is missing!");
            return Ok(Json(vec![]));
        }
    };
    let uruky_api_key = match std::env::var("URUKY_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: URUKY_API_KEY is missing!");
            return Ok(Json(vec![]));
        }
    };
    let uruky_id = match std::env::var("URUKY_ID") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: URUKY_ID is missing!");
            return Ok(Json(vec![]));
        }
    };

    // Check legibility of the request
    let polar_id: Option<String> = match params.a {
        // Resulti is unlimited AND Uruky is old (upcomming update)
        Some(key) if key == resulti_api_key || key == uruky_api_key => None,
        // Request doesn't have an API key, it's a free request
        // Check legibility
        None | Some("") => {
            if !free(client_ip.0) {
                return Err((
                    Status::TooManyRequests,
                    Json(json!({
                        "error": "Free limit of 25 searches a day reached",
                        "buy": POLAR_URL
                    })),
                ));
            }

            None
        }
        // Request has other API key
        Some(key) => match paid(key).await {
            Some(id) => Some(id),
            None => {
                return Err((
                    Status::PaymentRequired,
                    Json(json!({
                        "error": "Invalid API key or no credits left",
                        "buy": POLAR_URL
                    })),
                ));
            }
        },
    };
    ANALYTICS.record_api_request();

    let decentralized = params.dec.unwrap_or(true);

    let active_goggles = load_goggles(&get_goggle_ids(params.goggles, None));

    let full_results = search_db::run_json(
        params.q,
        params.lang,
        params.loc,
        embedding_service,
        active_goggles,
        false,
        decentralized,
        true,
        PRIECO_INDEX.clone(),
        None,
        20,
        None,
    )
    .await;

    let content_bool = params.content.unwrap_or(false);
    let results: Vec<serde_json::Value> = full_results
        .into_iter()
        .map(|res| {
            let mut r = json!({
                "url": res["url"],
                "title": res["title"],
                "description": res["description"],
                "lang": res["lang"],
                "loc": res["loc"],
                "safe_s": res["safe_s"],
                "image": res["image"]
            });
            if content_bool {
                r["content"] = res["content"].clone();
            }
            r
        })
        .collect();

    if !results.is_empty()
        && params.a == Some(uruky_api_key.as_str())
        && let Err(e) = CLIENT
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

    // Bill customer
    if let Some(id) = polar_id.filter(|_| !results.is_empty()) {
        API_BILLING_QUEUE.lock().push(id);
    }

    Ok(Json(results))
}

/// Proxies remote content, `GET /proxy?u=<URL encoded URL>`.
///
/// See [proxy_request].
///
/// # Errors
///
/// * 400 for invalid URL.
/// * 403 for [local addresses][bad_url].
/// * 415 for [not allowed type][allowed_type].
#[get("/proxy?<u>")]
pub async fn proxy_get(u: &str) -> Result<(ContentType, Vec<u8>), Status> {
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
    let (content_type, body) = proxy_request(&url, None, "GET").await?;

    if allowed_type(&url, &content_type) {
        Ok((content_type, body))
    } else {
        Err(Status::UnsupportedMediaType)
    }
}

/// Proxies remote content with POST body, `POST /proxy?u=<URL encoded URL>`.
///
/// Same as [proxy_get].
#[post("/proxy?<u>", data = "<body>")]
pub async fn proxy_post(u: &str, body: Vec<u8>) -> Result<(ContentType, Vec<u8>), Status> {
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
    let (content_type, body) = proxy_request(&url, Some(body), "POST").await?;

    if allowed_type(&url, &content_type) {
        Ok((content_type, body))
    } else {
        Err(Status::UnsupportedMediaType)
    }
}

/* Helper functions */
/// Checks if URL is unsafe to proxy, for security reasons.
///
/// Bad are: invalid URLs, URLs with user or password, hosts that don't resolve,
/// loopback, private, link local and other local IPs.
///
/// # Returns
///
/// True if URL must not be proxied.
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

/// Checks proxied content type.
///
/// All images are allowed. CSS, JS and JSON only from Search Expander and DuckDuckGo.
///
/// # Returns
///
/// True if allowed.
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
    // CSS and JS only Search Expander
    if top == "image" || (allowed_data && allowed_domain) {
        true
    }
    // Reject the rest
    else {
        false
    }
}

/// Downloads content for the proxy.
///
/// Follows max 2 redirects, each one is checked by [bad_url].
/// 'width' and 'height' are removed from Search Expander URLs.
/// Failed responses return the image placeholder SVG.
///
/// # Arguments
///
/// * `url` - Remote URL.
/// * `body` - JSON body for POST.
/// * `method` - 'GET' or 'POST'.
///
/// # Returns
///
/// (content type, body).
///
/// # Errors
///
/// * 502 on network error or too many redirects.
/// * 413 when over 10 MB.
async fn proxy_request(
    url: &Url,
    body: Option<Vec<u8>>,
    method: &str,
) -> Result<(ContentType, Vec<u8>), Status> {
    let mut target_url = url.clone();
    if target_url
        .domain()
        .is_some_and(|d| d.contains("searchexpander.com"))
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
            if let Some(loc) = response.headers().get("location")
                && let Ok(loc_str) = loc.to_str()
            {
                current_url = match current_url.join(loc_str) {
                    Ok(u) => u,
                    Err(_) => return Err(Status::BadGateway),
                };

                if bad_url(&current_url) {
                    return Err(Status::Forbidden);
                }

                if response.status().as_u16() == 303
                    || ((response.status().as_u16() == 301 || response.status().as_u16() == 302)
                        && current_method == "POST")
                {
                    current_method = "GET".to_string();
                    current_body = None;
                }

                redirects_left -= 1;
                continue;
            }

            // Broken redirect
            break;
        }

        // 200, 404...
        break;
    }

    if !response.status().is_success() {
        let fallback_svg =
            std::fs::read(format!("{}/img/icon/image.svg", STATIC_DIR)).unwrap_or_default();
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

    let body = match response.bytes().await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => return Err(Status::InternalServerError),
    };

    Ok((content_type, body))
}

/// Content type from file extension, Binary when unknown.
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

/// Service worker cache version, `GET /cache-ver`. 'unknown' when file is missing.
#[get("/cache-ver")]
pub fn cache_ver() -> String {
    read_to_string(CACHE_VERSION_FILE)
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|_| String::from("unknown"))
}

/// Counts anonymous page view, `GET /pv`.
///
/// See [prieco_core::AnalyticsDb::record_visitor], host is the counted entity.
///
/// # Returns
///
/// 'ok'
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

/// Roadmap feedback form.
#[derive(FromForm)]
pub struct RoadmapFeedback<'r> {
    /// Message.
    pub message: &'r str,
    /// Local page to go back to.
    pub return_path: &'r str,
}

/// Sends roadmap feedback to Roman Láncoš as a Signal message. `POST /submit_msg`
///
/// Uses signal-cli REST API on localhost:8071. Errors are only printed.
///
/// # Returns
///
/// Redirect to `return_path`, only [local][prieco_core::local_redirect].
#[post("/submit_msg", data = "<feedback>")]
pub async fn send_signal(feedback: Form<RoadmapFeedback<'_>>) -> Redirect {
    let signal_bot_number = match std::env::var("SIGNAL_BOT_NUMBER") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: SIGNAL_BOT_NUMBER is missing!");

            return Redirect::to(local_redirect(feedback.return_path));
        }
    };

    let signal_recipient_number = match std::env::var("SIGNAL_RECIPIENT_NUMBER") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: SIGNAL_RECIPIENT_NUMBER is missing!");

            return Redirect::to(local_redirect(feedback.return_path));
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

    Redirect::to(local_redirect(feedback.return_path))
}

/// One suggestion from DuckDuckGo.
#[derive(Deserialize)]
struct DdgSuggestion {
    phrase: String,
}

/// Search suggestions for browser omnibox, `GET /api/om_sug?q=`.
///
/// From DuckDuckGo, empty on error.
///
/// # Returns
///
/// OpenSearch suggestions JSON: `[query, [suggestions]]`.
#[get("/api/om_sug?<q>")]
pub async fn om_sug(q: &str) -> Json<RocketValue> {
    if q.trim().is_empty() {
        return Json(json!([q, Vec::<String>::new()]));
    }

    let mut phrases: Vec<String> = Vec::new();

    if let Ok(response) = reqwest::get(&format!(
        "https://duckduckgo.com/ac/?q={}&kl=wt-wt",
        urlencoding::encode(q)
    ))
    .await
        && let Ok(ddg_results) = response.json::<Vec<DdgSuggestion>>().await
    {
        phrases = ddg_results.into_iter().map(|item| item.phrase).collect();
    }

    Json(json!([q, phrases]))
}
