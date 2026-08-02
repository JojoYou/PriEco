use primp::Client;
use url::Url;

use std::time::Instant;

/*
  Description: Download remote file

  Input: Client, URL
  Output: body, status_code, downloading_time, final_url
*/
pub async fn download(client: &Client, url: &str) -> (String, u16, f64, String) {
    let start_downloading_time = Instant::now();
    let mut current_url = url.to_string();

    let mut response_result = client.get(&current_url).send().await;

    // Try adding www.
    if response_result.is_err() {
        if let Ok(mut parsed_url) = Url::parse(&current_url) {
            if let Some(host) = parsed_url.host_str() {
                if !host.starts_with("www.") && host.matches('.').count() == 1 {
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
            }
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
