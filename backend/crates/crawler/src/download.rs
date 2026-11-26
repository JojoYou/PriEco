//! # Page Download
//!
//! Downloads a single page for [crate::mini_crawler].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-06-16
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use primp::Client;
use url::Url;

use std::time::Instant;

/// Downloads remote page.
///
/// When the request fails and host is an apex domain ('example.com'),
/// it tries once more with 'www.'. Redirects are followed by `client`.
///
/// # Arguments
///
/// * `client` - HTTP client.
/// * `url` - Page URL.
///
/// # Returns
///
/// (body, status code, download time in seconds, final URL after redirects).
///
/// All empty and 0 when request fails. Only body is empty when it can't be read.
pub async fn download(client: &Client, url: &str) -> (String, u16, f64, String) {
    let start_downloading_time = Instant::now();
    let mut current_url = url.to_string();

    let mut response_result = client.get(&current_url).send().await;

    // Try adding www.
    if response_result.is_err()
        && let Ok(mut parsed_url) = Url::parse(&current_url)
        && let Some(host) = parsed_url.host_str()
        && !host.starts_with("www.")
        && host.matches('.').count() == 1
    {
        let new_host = format!("www.{}", host);
        if parsed_url.set_host(Some(&new_host)).is_ok() {
            current_url = parsed_url.to_string();
            println!(
                "🚀 Connection dropped! Retrying with apex fallback: {}",
                current_url
            );

            response_result = client.get(&current_url).send().await;
        }
    }

    let response = match response_result {
        Ok(response) => response,
        Err(_) => {
            return (String::new(), 0, 0.0, String::new());
        }
    };

    let downloading_time = start_downloading_time.elapsed().as_secs_f64();
    let status_code = response.status().as_u16();
    let final_url = response.url().to_string();

    let body = match response.bytes().await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => {
            return (String::new(), status_code, downloading_time, final_url);
        }
    };

    (body, status_code, downloading_time, final_url)
}
