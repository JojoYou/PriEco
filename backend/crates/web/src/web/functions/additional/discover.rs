//! # Discovery Search
//!
//! Finds root domains PriEco doesn't know yet.
//!
//! ## Architecture
//!
//! 1. Query without spaces and quotes + every TLD from the list, e.g. 'prieco' → 'prieco.com', 'prieco.net'...
//!     * Domains already in the index are skipped.
//! 2. HEAD request to all of them at once, 400 ms timeout each, 450 ms in total.
//! 3. Domains with 2xx are returned as results and sent to the crawler.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-06-27
//! * Last Modified: 2026-11-26
//!
//! ## Planned Improvements
//!
//! - [ ] None

use prieco_core::{CLIENT, PRIECO_META, WebDocument, url_to_id};

use crate::web::functions::general::favicon_url;

/// Pings domains made from query.
///
/// # Arguments
///
/// * `query` - User query.
///
/// # Returns
///
/// Placeholder documents with domain as title and 'Adding to PriEco index...' as description.
pub async fn discover_and_ping_domains(query: &str) -> Vec<WebDocument> {
    let trimmed = query.trim();

    let possible_domain = trimmed
        .replace(['"', '\''], "")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();

    if possible_domain.is_empty() {
        return Vec::new();
    }

    let mut discovery_results = Vec::new();
    let mut ping_tasks = tokio::task::JoinSet::new();

    let urls_to_ping: Vec<(String, String)> = {
        let mut pending = Vec::new();

        for tld in &[
            ".com",
            ".net",
            ".org",
            ".info",
            ".biz",
            ".co",
            ".io",
            ".dev",
            ".app",
            ".ai",
            ".tech",
            ".xyz",
            ".cloud",
            ".software",
            ".network",
            ".digital",
            ".me",
            ".tv",
            ".blog",
            ".design",
            ".art",
            ".media",
            ".video",
            ".news",
            ".shop",
            ".store",
            ".agency",
            ".global",
            ".online",
            ".site",
            ".pro",
            ".company",
            ".uk",
            ".de",
            ".fr",
            ".nl",
            ".eu",
            ".it",
            ".es",
            ".pl",
            ".ch",
            ".se",
            ".no",
            ".dk",
            ".fi",
            ".us",
            ".ca",
            ".mx",
            ".br",
            ".jp",
            ".cn",
            ".in",
            ".au",
            ".nz",
            ".sg",
        ] {
            let domain = format!("{}{}", possible_domain, tld);
            let canonical_url = format!("https://{}/", domain);

            let already_indexed = [
                canonical_url.clone(),
                format!("https://www.{}/", domain),
                format!("http://{}/", domain),
                format!("http://www.{}/", domain),
            ]
            .iter()
            .any(|url| {
                matches!(
                    PRIECO_META.meta_ks.get(url_to_id(url).to_be_bytes()),
                    Ok(Some(_))
                )
            });

            if !already_indexed {
                pending.push((canonical_url, domain));
            }
        }
        pending
    };

    for (canonical_url, domain_clone) in urls_to_ping {
        ping_tasks.spawn(async move {
            if let Ok(res) = CLIENT
                .head(&canonical_url)
                .timeout(std::time::Duration::from_millis(400))
                .send()
                .await
                && res.status().is_success()
            {
                return Some((canonical_url, domain_clone));
            }
            None
        });
    }

    let _ = tokio::time::timeout(std::time::Duration::from_millis(450), async {
        while let Some(task_res) = ping_tasks.join_next().await {
            if let Ok(Some((url, domain))) = task_res {
                let mock_doc = WebDocument {
                    url: url.clone(),
                    title: domain.clone(),
                    description: String::from("Adding to PriEco index..."),
                    content: String::new(),
                    favicon: favicon_url(&url),
                    image: String::new(),
                    keywords: String::new(),
                    safe_s: false,
                    html: String::new(),
                    lang: String::new(),
                    loc: String::new(),
                    impressions: 0,
                    clicks: 0,
                    confidence: 1.0,
                    effort: 0.0,
                    qna: 0.0,
                    sts: 0.0,
                    load: 0.0,
                    date: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,

                    has_500_words: false,
                    intent: 0,
                    is_mobile: false,

                    search_score: 0.0,
                    source: String::from(""),
                };
                discovery_results.push(mock_doc);
            }
        }
    })
    .await;

    // Send URLs to crawler
    if !discovery_results.is_empty() {
        let page_url = discovery_results[0].url.clone();
        let links: Vec<String> = discovery_results.iter().map(|d| d.url.clone()).collect();

        tokio::spawn(async move {
            let payload = serde_json::json!({
                "page_url": page_url,
                "links": links
            });

            if let Err(e) = CLIENT
                .post("http://0.0.0.0:8090/user-submission")
                .json(&payload)
                .send()
                .await
            {
                println!("Failed to batch send discovery to crawler queue: {}", e);
            }
        });
    }

    discovery_results
}
