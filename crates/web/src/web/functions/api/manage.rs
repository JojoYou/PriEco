//!  File: web/functions/api/manage.rs
//!  Description: Manages API access free tier per IP, Polar.sh usage billing
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-11-01
//!  Last Modified: 2026-11-01
//!
//!  Usage: Call free() or paid() before search, push customer ID to API_BILLING_QUEUE after it
//!  TODO:

/*
  Import system libraries
*/
use std::{collections::HashMap, net::IpAddr, time::Duration};

/*
  Import external libraries
*/
use chrono::{Datelike, Utc};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
/*
  Import own libraries
*/
use prieco_core::{CLIENT, PRIECO_META, colors};
use serde_json::{Value, json};
use tokio::time::sleep;

/*
  Constants
*/
const MAX_IP_CALLS: u8 = 25;
const CACHE_EXPIRE: i64 = 300;
const SUBSCRIPTION_ID: &str = "cd67ac3a-ad1c-4dd0-ab89-984698787e57";

// Manages free requests
pub fn free(ip: IpAddr) -> bool {
    // Generate FJALL key
    let mut key = b"f:".to_vec();
    match ip {
        IpAddr::V4(v4) => key.extend_from_slice(&v4.octets()),
        IpAddr::V6(v6) => key.extend_from_slice(&v6.octets()[..8]),
    }

    let today = (Utc::now().date_naive().num_days_from_ce() as u32).to_le_bytes();

    // Get value
    let value = PRIECO_META.api_ks.get(&key).unwrap_or(None);
    let mut count: u8 = 0;
    if let Some(v) = value {
        if v.len() == 5 && v[0..4] == today {
            count = v[4];
        }
    }

    // Don't allow more
    if count >= MAX_IP_CALLS {
        return false;
    }

    // Insert new value
    let mut new_value = Vec::with_capacity(5);
    new_value.extend_from_slice(&today);
    new_value.push(count + 1);
    if let Err(e) = PRIECO_META.api_ks.insert(&key, new_value) {
        eprintln!("{}Error inserting key:{} {}", colors::RED, colors::RESET, e);
    }

    // Allow free call
    true
}

// Manages paid requests
pub struct Customer {
    pub id: Option<String>,
    pub subscribed: bool,
    pub balance: i64,
    pub checked: i64,
}
static CACHE: Lazy<Mutex<HashMap<String, Customer>>> = Lazy::new(|| Mutex::new(HashMap::new()));
// If POLAR is down, keep serving Cached keys
fn stale(key: &str) -> Option<String> {
    let mut cache = CACHE.lock();
    if let Some(customer) = cache.get_mut(key) {
        if let Some(id) = &customer.id {
            if customer.subscribed || customer.balance > 0 {
                customer.balance -= 1;
                customer.checked = Utc::now().timestamp() - CACHE_EXPIRE + 60;
                return Some(id.clone());
            }
        }
    }
    None
}
pub async fn paid(key: &str) -> Option<String> {
    let now = Utc::now().timestamp();

    // Check cache
    {
        let mut cache = CACHE.lock();
        if let Some(customer) = cache.get_mut(key) {
            // Invalidate key after 5 min to recheck with POLAR.sh
            if now - customer.checked < CACHE_EXPIRE {
                if let Some(id) = &customer.id {
                    if customer.subscribed || customer.balance > 0 {
                        // Changed and given access
                        customer.balance -= 1;
                        return Some(id.clone());
                    }
                }

                return None; // Not given access
            }
        }
    }

    // Validate key
    let org_id = match std::env::var("POLAR_ORG_ID") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: POLAR_ORG_ID is missing!");
            return None;
        }
    };

    let validate = CLIENT
        .post("https://api.polar.sh/v1/customer-portal/license-keys/validate")
        .json(&json!({
            "key": key,
            "organization_id":  org_id
        }))
        .send()
        .await;
    let result = match validate {
        Ok(r) => r,
        Err(_) => return stale(key),
    };

    let status = result.status().as_u16();

    // Invalid key: Cache to stop calling POLAR.sh
    if status >= 400 && status <= 499 && status != 429 {
        let mut cache = CACHE.lock();
        if cache.len() > 100_000 {
            cache.clear();
        }

        // Insert fake key
        cache.insert(
            key.to_string(),
            Customer {
                id: None,
                subscribed: false,
                balance: 0,
                checked: Utc::now().timestamp(),
            },
        );

        return None;
    }
    // Something else went wrong
    else if status < 200 || status >= 300 {
        return stale(key);
    }

    // Get data
    let body = match result.text().await {
        Ok(b) => b,
        Err(_) => return stale(key),
    };
    let value_json: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return stale(key),
    };

    let status: Option<&str> = value_json.get("status").and_then(|v| v.as_str());
    let possible_id: Option<&str> = value_json.get("customer_id").and_then(|v| v.as_str());

    // Get granted access
    let customer_id = if status == Some("granted") {
        possible_id.map(|s| s.to_string())
    } else {
        None
    };

    let id = match customer_id {
        Some(i) => i,
        None => {
            // Not granted access, fake cache API key
            let mut cache = CACHE.lock();
            if cache.len() > 100_000 {
                cache.clear();
            }

            // Insert fake data
            cache.insert(
                key.to_string(),
                Customer {
                    id: None,
                    subscribed: false,
                    balance: 0,
                    checked: Utc::now().timestamp(),
                },
            );

            return None;
        }
    };

    // Get customer state
    let api_key = match std::env::var("POLAR_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: POLAR_API_KEY is missing!");
            return None;
        }
    };
    let state_request = CLIENT
        .get(format!("https://api.polar.sh/v1/customers/{}/state", id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;

    let result = match state_request {
        Ok(r) if r.status().is_success() => r,
        _ => return stale(key),
    };

    // Get data
    let state_body = match result.text().await {
        Ok(b) => b,
        Err(_) => return stale(key),
    };

    let state_json: Value = match serde_json::from_str(&state_body) {
        Ok(v) => v,
        Err(_) => return stale(key),
    };

    // Check if customer is subscribed
    let mut subscribed = false;
    if let Some(subs) = state_json
        .get("active_subscriptions")
        .and_then(|v| v.as_array())
    {
        for sub in subs {
            if sub.get("product_id").and_then(|v| v.as_str()) == Some(SUBSCRIPTION_ID) {
                subscribed = true;
                break;
            }
        }
    }

    // Check balance
    let mut balance: i64 = 0;
    if let Some(meters) = state_json.get("active_meters").and_then(|v| v.as_array()) {
        for meter in meters {
            if let Some(bal) = meter.get("balance").and_then(|v| v.as_f64()) {
                balance += bal as i64;
            }
        }
    }

    // Cache API key
    let mut cache = CACHE.lock();
    if cache.len() > 100_000 {
        cache.clear();
    }

    let allowed = subscribed || balance > 0;
    let final_balance = if allowed { balance - 1 } else { balance };

    cache.insert(
        key.to_string(),
        Customer {
            id: Some(id.clone()),
            subscribed,
            balance: final_balance,
            checked: Utc::now().timestamp(),
        },
    );

    if allowed { Some(id) } else { None }
}

// Reports billing to POLAR.sh
pub static API_BILLING_QUEUE: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub async fn flush_loop() {
    loop {
        sleep(Duration::from_secs(10)).await;
        flush().await;
    }
}
pub async fn flush() {
    let api_key = match std::env::var("POLAR_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Warning: POLAR_API_KEY is missing!");
            return;
        }
    };

    // Get ids to bill usage
    let ids: Vec<String> = {
        let mut queue = API_BILLING_QUEUE.lock();
        std::mem::take(&mut *queue)
    };
    if ids.is_empty() {
        return;
    }

    // Process max 1000 ids at a time
    // Boy I wish I had so many customers :)
    // But let's limit it regardless
    for chunk in ids.chunks(1000) {
        let events: Vec<_> = chunk
            .iter()
            .map(|id| {
                json!({
                    "name": "api_call",
                    "customer_id": id
                })
            })
            .collect();

        let payload = json!({ "events": events });

        let result = CLIENT
            .post("https://api.polar.sh/v1/events/ingest")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await;

        // Remove incorrect requests
        let mut requeue = false;
        match result {
            Ok(res) => {
                let status = res.status();
                if status.is_success() {
                    // Success
                } else if status.is_server_error() || matches!(status.as_u16(), 401 | 403 | 429) {
                    // 5xx or 429 Retry later
                    eprintln!(
                        "Polar flush transient error ({}). Requeuing {} events.",
                        status,
                        chunk.len()
                    );
                    requeue = true;
                } else {
                    // Drop the chunk
                    let body = res
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read body".to_string());

                    eprintln!(
                        "{}Polar rejected chunk{} with {}. Dropping {} events. Reason: {}",
                        colors::RED,
                        colors::RESET,
                        status,
                        chunk.len(),
                        body
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Polar network error: {}. Requeuing {} events.",
                    e,
                    chunk.len()
                );
                requeue = true;
            }
        }

        if requeue {
            API_BILLING_QUEUE.lock().extend_from_slice(chunk);
        }
    }
}
