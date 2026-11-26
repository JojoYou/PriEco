//! # General
//!
//! Small functions used by many pages: URLs, cookies, hashing, remote JSON.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::hash::Hasher;

/*
  Import external libraries
*/
use rocket::{
    http::{Cookie, CookieJar, SameSite},
    time::{Duration, OffsetDateTime},
};
use serde_json::Value;
use twox_hash::XxHash64;
use url::Url;

use prieco_core::globals::colors;

/// Checks if URL is valid http or https URL.
///
/// # Returns
///
/// True if valid, false otherwise.
pub fn is_valid_url(input: &str) -> bool {
    match Url::parse(input) {
        Ok(url) => matches!(url.scheme(), "http" | "https"),
        Err(_) => false,
    }
}

/// Gets domain from URL.
///
/// # Arguments
///
/// * `url` - URL.
/// * `remove_www` - Strip 'www.'.
///
/// # Returns
///
/// Domain or empty string, error is printed.
pub fn get_domain(url: &str, remove_www: bool) -> String {
    if url.is_empty() {
        return String::new();
    }
    let parsed_url = Url::parse(url);
    match parsed_url {
        Ok(parsed) => {
            if let Some(domain) = parsed.domain() {
                if remove_www && domain.starts_with("www.") {
                    return domain[4..].to_string();
                }
                return domain.to_string();
            }
            println!("URL has no domain part: {}", url);
        }
        Err(err) => {
            println!("Failed to parse URL {}: {}", url, err);
        }
    }
    String::new()
}

/// Simple call to create a browser cookie. SameSite Lax, path '/'.
///
/// # Arguments
///
/// * `cookie_jar` - Cookies.
/// * `cookie_name` - Name.
/// * `cookie_value` - Value.
/// * `cookie_long_life` - 1 year, otherwise 7 days.
/// * `js` - JavaScript can read it, otherwise HTTP only.
/// * `secure` - Only over HTTPS.
pub fn set_cookie(
    cookie_jar: &CookieJar<'_>,
    cookie_name: String,
    cookie_value: String,
    cookie_long_life: bool,
    js: bool,
    secure: bool,
) {
    let mut cookie = Cookie::new(cookie_name, cookie_value);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(secure);
    cookie.set_path("/");

    if cookie_long_life {
        cookie.set_max_age(Duration::days(365));
        cookie.set_expires(OffsetDateTime::now_utc() + Duration::days(365));
    } else {
        cookie.set_max_age(Duration::days(7));
        cookie.set_expires(OffsetDateTime::now_utc() + Duration::days(7));
    }

    if !js {
        cookie.set_http_only(true);
    }

    cookie_jar.add(cookie);
}

/// Hashes text to a number, xxHash64.
pub fn hash_node(node: &str) -> u64 {
    let mut hasher = XxHash64::default();
    hasher.write(node.as_bytes());
    hasher.finish()
}

/// Safely calls external URL for JSON.
///
/// # Returns
///
/// Parsed JSON, `None` on error, the error is printed.
pub async fn call_api_future_json(req: reqwest::RequestBuilder) -> Option<Value> {
    match req.send().await {
        Ok(resp) => match resp.text().await {
            Ok(text) => serde_json::from_str::<Value>(&text).ok(),
            Err(err) => {
                println!(
                    "{}Failed to read response text{}: {}",
                    colors::RED,
                    colors::RESET,
                    err
                );
                None
            }
        },
        Err(err) => {
            println!("{}Request failed:{} {}", colors::RED, colors::RESET, err);
            None
        }
    }
}

/// Proxied DuckDuckGo favicon of URL's domain. Empty string without domain.
pub fn favicon_url(url: &str) -> String {
    let domain = get_domain(url, false);
    if domain.is_empty() {
        return String::new();
    }
    format!(
        "/proxy?u={}",
        urlencoding::encode(&format!("https://icons.duckduckgo.com/ip3/{}.ico", domain))
    )
}
