use rocket::{
    http::{Cookie, CookieJar, SameSite},
    time::{Duration, OffsetDateTime},
};
use serde_json::Value;
use std::{fs::read_to_string, hash::Hasher};
use twox_hash::XxHash64;
use url::Url;

// Import local libraries
pub mod globals;

pub mod functions {
    pub mod search_db;
    pub mod search_endpoint;
    pub mod settings;

    pub mod search_api {
        pub mod all;
        pub mod img;
        pub mod yadore;
    }
}

/*
  Description: Gets domain from URL

  Input: URL, if remove_www
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
  Description: Reads file

  Input: file path
  Output: file contents as a string
*/
pub fn read_file(file_path: &str) -> String {
    match read_to_string(file_path) {
        Ok(contents) => contents,
        Err(_) => String::new(),
    }
}
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
pub fn hash_node(node: &str) -> u64 {
    let mut hasher = XxHash64::default();
    hasher.write(node.as_bytes());
    hasher.finish()
}

pub async fn call_api_future_json(req: reqwest::RequestBuilder) -> Option<Value> {
    match req.send().await {
        Ok(resp) => match resp.text().await {
            Ok(text) => serde_json::from_str::<Value>(&text).ok(),
            Err(err) => {
                eprintln!("Failed to read response text: {}", err);
                None
            }
        },
        Err(err) => {
            eprintln!("Request failed: {}", err);
            None
        }
    }
}
