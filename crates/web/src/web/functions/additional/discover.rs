use serde_json::Value as Json_Value;
use tokio::task::JoinSet;

use prieco_core::{CLIENT, ROCKSDB_INDEX, url_to_id};

pub async fn discover_and_ping_domains(query: &str) -> Vec<Json_Value> {
    let trimmed = query.trim();

    // Combine & Sanitize
    let possible_domain = trimmed
        .replace('"', "")
        .replace('\'', "")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();

    if possible_domain.is_empty() {
        return Vec::new();
    }

    let mut discovery_results = Vec::new();
    let mut ping_tasks = JoinSet::new();

    for tld in &[
        ".com", ".net", ".org", ".io", ".co", ".dev", ".app", ".ai", ".info", ".biz", ".me", ".tv",
        ".us", ".uk", ".ca", ".de", ".fr", ".nl", ".eu", ".xyz", ".tech", ".online", ".site",
    ] {
        let domain = format!("{}{}", possible_domain, tld);
        let possible_url = format!("https://{}/", domain);

        // RocksDB check (if in, then index could have brought the result)
        match ROCKSDB_INDEX.get(&url_to_id(&possible_url).to_be_bytes()) {
            Ok(Some(_)) => continue,
            _ => {
                let url_clone = possible_url.clone();
                let domain_clone = domain.clone();

                ping_tasks.spawn(async move {
                    // HEAD request
                    if let Ok(res) = CLIENT.head(&url_clone).send().await {
                        if res.status().is_success() {
                            return Some((url_clone, domain_clone));
                        }
                    }

                    // Fallback GET
                    if let Ok(res) = CLIENT.get(&url_clone).send().await {
                        if res.status().is_success() {
                            return Some((url_clone, domain_clone));
                        }
                    }

                    None
                });
            }
        }
    }

    // Create results
    let mut discovered_urls = Vec::new();
    while let Some(task_res) = ping_tasks.join_next().await {
        if let Ok(Some((url, domain))) = task_res {
            discovered_urls.push(url.clone());

            // Construct results
            let mock_doc = serde_json::json!({
                "url": url,
                "title": format!("Discovery: {}", domain),
                "description": "Adding to PriEco index...",
                "image": "",
                "favicon": format!("https://www.google.com/s2/favicons?domain={}&sz=512", domain),
                "search_score": 0.95
            });

            discovery_results.push(mock_doc);
        }
    }

    // Send URLs to crawler
    if !discovered_urls.is_empty() {
        let mut urls_iter = discovered_urls.into_iter();
        let page_url = urls_iter.next().unwrap();
        let links: Vec<String> = urls_iter.collect();

        tokio::spawn(async move {
            let payload = serde_json::json!({
                "page_url": page_url,
                "links": links
            });

            let res = CLIENT
                .post("http://0.0.0.0:8090/web-discovery")
                .json(&payload)
                .send()
                .await;

            if let Err(e) = res {
                println!("Failed to batch send discovery to crawler queue: {}", e);
            }
        });
    }

    discovery_results
}
