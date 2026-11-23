use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prieco_core::PRIECO_META;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use time::OffsetDateTime;

use super::types::{Goggle, GoggleRules};

static PUBLIC_CACHE: Lazy<RwLock<Option<Arc<Vec<Goggle>>>>> = Lazy::new(|| RwLock::new(None));
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn put(goggle: &Goggle) {
    let key = goggle.id.to_be_bytes();
    let value = serde_json::to_vec(goggle).expect("Failed to serialize StoredGoggle");
    PRIECO_META
        .goggles_ks
        .insert(key, &value)
        .expect("Failed to write goggle");
    invalidate_public();
}

pub fn get(id: u64) -> Option<Goggle> {
    let raw = PRIECO_META.goggles_ks.get(id.to_be_bytes()).ok()??;
    serde_json::from_slice(&raw).ok()
}

pub fn delete(id: u64) {
    let _ = PRIECO_META.goggles_ks.remove(id.to_be_bytes());
    invalidate_public();
}

pub fn touch_fetched_at(id: u64) {
    if let Some(mut g) = get(id) {
        g.fetched_at = OffsetDateTime::now_utc().unix_timestamp();
        put(&g);
    }
}

pub fn list_all() -> Vec<Goggle> {
    PRIECO_META
        .goggles_ks
        .iter()
        .filter_map(|guard| serde_json::from_slice(&guard.value().ok()?).ok())
        .collect()
}

pub fn list_public() -> Vec<Goggle> {
    if let Some(cached) = PUBLIC_CACHE.read().as_ref() {
        return cached.as_ref().clone();
    }

    let gen_before = GENERATION.load(Ordering::SeqCst);
    let fresh: Arc<Vec<Goggle>> = Arc::new(
        list_all()
            .into_iter()
            .filter(|g| g.public)
            .map(|mut g| {
                g.rules = GoggleRules::default();
                g
            })
            .collect(),
    );

    let mut slot = PUBLIC_CACHE.write();
    if GENERATION.load(Ordering::SeqCst) == gen_before {
        *slot = Some(Arc::clone(&fresh));
    }
    fresh.as_ref().clone()
}

/* Helper function */
fn invalidate_public() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    *PUBLIC_CACHE.write() = None;
}
