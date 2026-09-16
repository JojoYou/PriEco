use std::collections::HashMap;
use std::sync::RwLock;
use tokio::runtime::Runtime;
use url::Url;

use crate::update::{Update, UpdateAction};
use prieco_core::{PING_CLIENT, WebDocument};

pub struct PruneDeadLinks {
    domain_cache: RwLock<HashMap<String, bool>>,
    rt: Runtime,
}

impl PruneDeadLinks {
    pub fn new() -> Self {
        Self {
            domain_cache: RwLock::new(HashMap::new()),
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
        }
    }

    fn extract_root(&self, url_str: &str) -> Option<String> {
        let parsed = Url::parse(url_str).ok()?;
        Some(format!("{}://{}", parsed.scheme(), parsed.host_str()?))
    }
}

impl Update for PruneDeadLinks {
    fn id(&self) -> &'static str {
        "v1_prune_dead_links_dry_run"
    }

    fn apply(&self, doc: &mut WebDocument) -> UpdateAction {
        let _guard = self.rt.enter();

        let root_url = match self.extract_root(&doc.url) {
            Some(url) => url,
            None => {
                println!("🗑️ [DRY RUN] Would delete garbage URL format: {}", doc.url);
                return UpdateAction::Unchanged;
            }
        };

        {
            let cache = self.domain_cache.read().unwrap();
            if let Some(&is_alive) = cache.get(&root_url) {
                if !is_alive {
                    println!(
                        "🗑️ [DRY RUN] Would delete due to cached dead root {}: {}",
                        root_url, doc.url
                    );
                    return UpdateAction::Unchanged;
                }
            }
        }

        let domain_known = self.domain_cache.read().unwrap().contains_key(&root_url);
        if !domain_known {
            let is_alive = match self.rt.block_on(PING_CLIENT.head(&root_url).send()) {
                Ok(resp) => {
                    let status = resp.status().as_u16();

                    if doc.url.contains("youtube.com") {
                        println!("ℹ️ Pinging root YouTube {}: status {}", root_url, status);
                    }

                    status < 400 || status == 403
                }
                Err(_) => false,
            };

            self.domain_cache
                .write()
                .unwrap()
                .insert(root_url.clone(), is_alive);

            if !is_alive {
                println!(
                    "🗑️ [DRY RUN] Would delete due to unreachable root {}: {}",
                    root_url, doc.url
                );
                return UpdateAction::Unchanged;
            }
        }

        match self.rt.block_on(PING_CLIENT.head(&doc.url).send()) {
            Ok(resp) => {
                let status = resp.status().as_u16();

                if doc.url.contains("youtube.com") {
                    println!(
                        "ℹ️ Pinging actual YouTube URL {}: status {}",
                        doc.url, status
                    );
                }

                if status == 404 || status == 410 {
                    println!("🗑️ [DRY RUN] Would delete 404/410 URL: {}", doc.url);
                    UpdateAction::Unchanged
                } else {
                    UpdateAction::Unchanged
                }
            }
            Err(_) => {
                println!(
                    "🗑️ [DRY RUN] Would delete due to exact URL timeout: {}",
                    doc.url
                );
                UpdateAction::Unchanged
            }
        }
    }
}
