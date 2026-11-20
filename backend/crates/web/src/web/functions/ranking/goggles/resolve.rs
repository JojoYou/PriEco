use super::storage::get;
use super::types::GoggleRules;
use rocket::http::CookieJar;
use std::sync::Arc;

pub fn get_goggle_ids(param: Option<&str>, cookie_jar: Option<&CookieJar<'_>>) -> Vec<u64> {
    let raw = param.map(str::to_string).or_else(|| {
        cookie_jar
            .and_then(|jar| jar.get("active_goggles"))
            .map(|c| c.value().to_string())
    });
    raw.map(|s| s.split(',').filter_map(|p| p.trim().parse().ok()).collect())
        .unwrap_or_default()
}

pub fn load_goggles(ids: &[u64]) -> Vec<Arc<GoggleRules>> {
    ids.iter()
        .filter_map(|&id| get(id))
        .map(|g| Arc::new(g.rules))
        .collect()
}
