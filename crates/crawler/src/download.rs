use once_cell::sync::Lazy;
use reqwest::Client;
use reqwest::header::{
    ACCEPT_LANGUAGE, CACHE_CONTROL, HeaderMap, HeaderName, HeaderValue, USER_AGENT,
};

use std::time::Instant;

pub static DEFAULT_HEADERS: Lazy<HeaderMap> = Lazy::new(|| {
    let mut request_headers = HeaderMap::new();
    request_headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (compatible; PriEcoBot/1.0.0; +https://prieco.net)"),
    );

    request_headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));

    request_headers.insert(
        HeaderName::from_static("accept-content-language"),
        HeaderValue::from_static("en"),
    );
    request_headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    request_headers
});

/*
  Description: Download remote file

  Input: Client, URL
  Output: body, status_code, downloading_time, final_url
*/
pub async fn download(client: &Client, url: &str) -> (String, u16, f64, String) {
    // Measure time it takes to download
    let start_downloading_time = Instant::now();

    // Download file
    let response = match client
        .get(url)
        .headers(DEFAULT_HEADERS.clone())
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return (String::new(), 0, 0.0, String::new()),
    };

    let downloading_time = start_downloading_time.elapsed().as_secs_f64();
    let status_code = response.status().as_u16();
    let final_url = response.url().to_string();
    let body = match response.bytes().await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => return (String::new(), 0, 0.0, String::new()),
    };

    (body, status_code, downloading_time, final_url)
}
