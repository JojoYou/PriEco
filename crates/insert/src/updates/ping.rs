use std::{thread::sleep, time::Duration};

use dashmap::DashSet;
use once_cell::sync::Lazy;
use prieco_core::{PING_CLIENT, WebDocument};
use url::Url;

use crate::update::{Update, UpdateAction};

static ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("Failed to build Tokio runtime"));

pub struct PingDeadLinksUpdate {
    dead_domains: DashSet<String>,
}

impl PingDeadLinksUpdate {
    pub fn new() -> Self {
        Self {
            dead_domains: DashSet::new(),
        }
    }

    fn ping_with_retries(&self, url: &str, retries: u8) -> bool {
        for _ in 0..retries {
            if let Ok(response) = ASYNC_RUNTIME.block_on(PING_CLIENT.get(url).send()) {
                if response.status().is_success() {
                    return true;
                }
            }

            sleep(Duration::from_millis(500));
        }
        false
    }
}

impl Update for PingDeadLinksUpdate {
    fn id(&self) -> &'static str {
        "ping_dead_links_v1"
    }

    fn apply(&self, doc: &mut WebDocument) -> UpdateAction {
        let domain = match Url::parse(&doc.url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
        {
            Some(d) => d,
            None => return UpdateAction::Deleted,
        };

        // Domain already in dead cache
        if self.dead_domains.contains(&domain) {
            return UpdateAction::Deleted;
        }

        let alive = self.ping_with_retries(&doc.url, 3);
        if alive {
            return UpdateAction::Unchanged;
        }

        // Dead URL
        // Check domain
        let alive_domain = self.ping_with_retries(&format!("https://{}", domain), 2);
        if !alive_domain {
            println!("Dead Domain: {}", domain);
            self.dead_domains.insert(domain);
        }

        UpdateAction::Deleted
    }
}
