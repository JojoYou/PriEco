use dotenv_codegen::dotenv;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use crate::{
    globals::{WebScrollResult, colors},
    web::functions::general::call_api_future_json,
};

pub async fn run(
    query: &str,
    loc: &str,
) -> Result<Vec<WebScrollResult>, Box<dyn std::error::Error + Send + Sync>> {
    let client = match Client::builder()
        .user_agent("PriEco/1.0.0 ( support@jojoyou.org )")
        .timeout(Duration::from_secs(2))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            println!(
                "{}Call Yadore: Failed to build HTTP client: {}{}",
                colors::RED,
                colors::RESET,
                err
            );
            return Ok(Vec::new());
        }
    };
    let yadore_option = call_api_future_json(        client
    .get(&format!(
        "https://api.yadore.com/v2/offer?market={}&keyword={}&precision=fuzzy&sort=rel_desc&limit=20",
        loc, query
    ))   .header("API-Key", dotenv!("YADORE_API_KEY")))
    .await;
    let yadore_json = match yadore_option {
        Some(json) => json,
        None => {
            return Ok(Vec::new()); // Failed
        }
    };

    if yadore_json.get("error").is_some() {
        return Ok(Vec::new()); // Failed
    }

    Ok(format_yadore(yadore_json))
}

fn format_yadore(json: Value) -> Vec<WebScrollResult> {
    let mut results = Vec::with_capacity(20);

    if let Some(offers) = json.get("offers").and_then(|v| v.as_array()) {
        for offer in offers {
            results.push(WebScrollResult {
                url: offer
                    .get("clickUrl")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                domain: offer
                    .get("merchant")
                    .and_then(|m| m.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: offer
                    .get("image")
                    .and_then(|img| img.get("url"))
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        format!(
                            "<img loading='lazy' alt='‎' src='/proxy?u={}'>",
                            urlencoding::encode(s)
                        )
                    })
                    .unwrap_or_default(),
                favicon: offer
                    .get("merchant")
                    .and_then(|m| m.get("logo"))
                    .and_then(|l| l.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                title: offer
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                price: offer
                    .get("price")
                    .and_then(|p| {
                        let amount = p.get("amount").and_then(|v| v.as_str());
                        let currency = p.get("currency").and_then(|v| v.as_str());
                        match (amount, currency) {
                            (Some(a), Some(c)) => Some(format!("{} {}", a, c)),
                            (Some(a), None) => Some(a.to_string()),
                            _ => None,
                        }
                    })
                    .unwrap_or_default(),
            });
        }
    }
    results
}
