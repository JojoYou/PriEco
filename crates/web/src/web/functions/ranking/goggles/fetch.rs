use crate::web::functions::ranking::goggles::parse_goggle;

use super::storage::{delete, list_all, put, touch_fetched_at};
use super::types::Goggle;
use chrono::Utc;
use prieco_core::{CLIENT, colors, url_to_id};

pub async fn fetch_and_store(url: String) -> Goggle {
    if !is_safe_goggle_url(&url) {
        println!(
            "{}We got passed unsafe goggle!{} {}",
            colors::RED,
            colors::RESET,
            &url
        );
        return Goggle::default();
    }

    // Fetch
    let Ok(response) = CLIENT.get(&url).send().await else {
        println!(
            "{}Network error fetching Goggle!{} {}",
            colors::RED,
            colors::RESET,
            &url
        );
        return Goggle::default();
    };

    let Ok(text) = response.text().await else {
        println!(
            "{}Failed to read Goggle response body!{} {}",
            colors::RED,
            colors::RESET,
            &url
        );
        return Goggle::default();
    };
    if text.trim().is_empty() {
        println!(
            "{}Goggle text is empty!{} {}",
            colors::RED,
            colors::RESET,
            &url
        );
        return Goggle::default();
    }

    store_goggle(text, url)
}

/// Description: Cache Goggle for 1 day and then recheck if it's still online
/// 
/// Input: None
/// Output: None
pub async fn refresh_stale_goggles() {
    const STALE_AFTER_SECS: i64 = 86_400; // 1 day

    for stored in list_all() {
        if stored.url.is_empty() {
            continue;
        }

        // Time hasnt passed
        if Utc::now().timestamp() - stored.fetched_at < STALE_AFTER_SECS {
            continue;
        }

        refresh_one(stored).await;
    }
}

async fn refresh_one(stored: Goggle) {
    match CLIENT.get(&stored.url).send().await {
        Ok(resp)
            if matches!(
                resp.status(),
                reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::GONE
            ) =>
        {
            println!("Goggle {} no longer exists at source, removing", stored.id);
            delete(stored.id);
        }
        Ok(resp) if resp.status().is_success() => {
            let Ok(text) = resp.text().await else { return };
            if url_to_id(&text) != stored.content_hash {
                store_goggle(text.to_string(), stored.url);
            } else {
                touch_fetched_at(stored.id);
            }
        }
        _ => {
            println!(
                "Goggle {} refresh failed (transient), leaving as-is",
                stored.id
            );
        }
    }
}

/* Helper functions */
/*
  Description: Checks if Goggle URL is safe and not suspicious

  Input: url
  Output: is safe
*/
fn is_safe_goggle_url(url: &str) -> bool {
    // Is url valid?
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };

    // Is url HTTP protocol?
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return false;
    }

    // Has URL a host?
    let Some(host) = parsed.host_str() else {
        return false;
    };

    // Is it trying to call something on local network?
    !matches!(host, "localhost")
        && !host.starts_with("127.")
        && !host.starts_with("169.254.")
        && !host.starts_with("10.")
        && !host.starts_with("192.168.")
}

fn store_goggle(text: String, url: String) -> Goggle {
    let meta = parse_goggle(&text);
    let goggle = Goggle {
        id: url_to_id(&url),
        name: meta.name,
        description: meta.description,
        author: meta.author,
        public: meta.public,
        avatar: meta.avatar,
        content_hash: url_to_id(&text),
        fetched_at: Utc::now().timestamp(),
        url,
        rules: meta.rules,
    };
    put(&goggle);
    goggle
}
