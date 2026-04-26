/*
  File: web/functions/general.rs
  Description: PriEco settings module

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-02-06

  Usage: Call these usually used functions for website
  TODO:
*/

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

use crate::globals::colors;

/*
  Description: Checks if URL is valid

  Input: URL
  Output: true if valid, false otherwise
*/
pub fn is_valid_url(input: &str) -> bool {
    match Url::parse(input) {
        Ok(url) => match url.scheme() {
            "http" | "https" => true,
            _ => false,
        },
        Err(_) => false,
    }
}

/*
  Description: Gets domain from URL

  Input: URL, if remove www
  Output: domain
*/
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

/*
  Description: Simple call to create a browser cookie

  Input: Name, Value, 7 days or 1 year, HTTP-ONLY
  Output:
*/
pub fn set_cookie(
    cookie_jar: &CookieJar<'_>,
    cookie_name: String,
    cookie_value: String,
    cookie_long_life: bool,
    js: bool,
) {
    let mut cookie = Cookie::new(cookie_name, cookie_value);
    cookie.set_same_site(SameSite::Strict);
    cookie.set_secure(true);

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

/*
  Description: Hash URL to a number

  Input: URL
  Output: Hash
*/
pub fn hash_node(node: &str) -> u64 {
    let mut hasher = XxHash64::default();
    hasher.write(node.as_bytes());
    hasher.finish()
}

/*
  Description:  Safely calls external URL for data

  Input: Request
  Output: JSON
*/
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
