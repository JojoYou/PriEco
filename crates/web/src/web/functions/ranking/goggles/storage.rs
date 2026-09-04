use prieco_core::PRIECO_META;

use super::types::Goggle;

pub fn put(goggle: &Goggle) {
    let key = goggle.id.to_be_bytes();
    let value = serde_json::to_vec(goggle).expect("Failed to serialize StoredGoggle");
    PRIECO_META
        .goggles_ks
        .insert(&key, &value)
        .expect("Failed to write goggle");
}

pub fn get(id: u64) -> Option<Goggle> {
    let raw = PRIECO_META.goggles_ks.get(&id.to_be_bytes()).ok()??;
    serde_json::from_slice(&raw).ok()
}

pub fn delete(id: u64) {
    let _ = PRIECO_META.goggles_ks.remove(&id.to_be_bytes());
}

pub fn touch_fetched_at(id: u64) {
    if let Some(mut g) = get(id) {
        g.fetched_at = chrono::Utc::now().timestamp();
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
    list_all().into_iter().filter(|g| g.public).collect()
}
