//! # Dead Links Update
//!
//! [Update] that removes pages which don't respond anymore.
//!
//! ## Architecture
//!
//! 1. [**Page**:][PingDeadLinksUpdate::ping_with_retries] Pings page URL 3 times.
//!     * 2xx, 401 and 403 count as alive.
//! 2. [**Domain**:][PingDeadLinksUpdate::ping_with_retries] If page is dead, pings 'https://domain' 2 times.
//!     * Dead domain is cached, its other pages are removed without ping.
//! 3. Dead page is [deleted][UpdateAction::Deleted].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-09-16
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use std::{thread::sleep, time::Duration};

use dashmap::DashSet;
use once_cell::sync::Lazy;
use prieco_core::{PING_CLIENT, WebDocument, icons};
use url::Url;

use crate::update::{Update, UpdateAction};

/// Tokio runtime for blocking pings from the sync updater.
static ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("Failed to build Tokio runtime"));

/// Removes pages and domains that are offline.
pub struct PingDeadLinksUpdate {
    /// Domains found dead during this run.
    dead_domains: DashSet<String>,
}

impl PingDeadLinksUpdate {
    /// Update with empty dead domains cache.
    pub fn new() -> Self {
        Self {
            dead_domains: DashSet::new(),
        }
    }

    /// Pings `url` up to `retries` times, 500 ms apart.
    ///
    /// # Returns
    ///
    /// True on 2xx, 401 or 403 response.
    fn ping_with_retries(&self, url: &str, retries: u8) -> bool {
        for _ in 0..retries {
            if let Ok(response) =
                ASYNC_RUNTIME.block_on(async { PING_CLIENT.get(url).send().await })
                && (response.status().is_success()
                    || response.status().as_u16() == 403
                    || response.status().as_u16() == 401)
            {
                return true;
            }

            sleep(Duration::from_millis(500));
        }
        false
    }
}

impl Default for PingDeadLinksUpdate {
    fn default() -> Self {
        Self::new()
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
            println!("{}: Dead Domain: {}", icons::INDEX_UPDATER, domain);
            self.dead_domains.insert(domain);
        }

        UpdateAction::Deleted
    }
}
