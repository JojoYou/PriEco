use std::{collections::HashMap, hash::DefaultHasher};

use prieco_core::{PRIECO_FJALL, url_to_domain_id, url_to_id};
use rocket::http::CookieJar;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ParsedGoggle {
    pub discard_by_default: bool,
    pub site_boost: HashMap<u64, f64>,
    pub site_downrank: HashMap<u64, f64>,
    pub path_boost: Vec<(String, f64)>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StoredGoggle {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub author: String,
    pub public: bool,
    pub avatar: String,
    pub source_url: String,
    pub raw_text: String,
    pub fetched_at: i64,
    pub content_hash: u64,
    pub rules: ParsedGoggle,
}

pub fn put(goggle: &StoredGoggle) {
    let key = goggle.id.to_be_bytes();
    let value = serde_json::to_vec(goggle).expect("Failed to serialize StoredGoggle");
    PRIECO_FJALL
        .goggles_ks
        .insert(&key, &value)
        .expect("Failed to write goggle");
}

pub fn get(id: u64) -> Option<StoredGoggle> {
    let key = id.to_be_bytes();
    let raw = PRIECO_FJALL.goggles_ks.get(&key).ok()??;
    serde_json::from_slice(&raw).ok()
}

pub fn list_public() -> Vec<StoredGoggle> {
    PRIECO_FJALL
        .goggles_ks
        .iter()
        .filter_map(|guard| {
            let value = guard.value().ok()?;
            serde_json::from_slice::<StoredGoggle>(&value).ok()
        })
        .filter(|g| g.public)
        .collect()
}

pub fn parse_goggle(raw: &str) -> (String, String, String, bool, String, ParsedGoggle) {
    let mut name = String::new();
    let mut description = String::new();
    let mut author = String::new();
    let mut public = false;
    let mut avatar = String::new();
    let mut parsed = ParsedGoggle::default();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('!') {
            let rest = rest.trim();
            if let Some((k, v)) = rest.split_once(':') {
                match k.trim() {
                    "name" => name = v.trim().to_string(),
                    "description" => description = v.trim().to_string(),
                    "author" => author = v.trim().to_string(),
                    "public" => public = v.trim() == "true",
                    "avatar" => avatar = v.trim().to_string(),
                    _ => {}
                }
            }
            continue;
        }
        if line == "$discard" {
            parsed.discard_by_default = true;
            continue;
        }
        let (pattern, modifiers) = match line.split_once('$') {
            Some((p, m)) => (p.to_string(), m),
            None => continue,
        };
        let mut boost_val: Option<f64> = None;
        let mut is_downrank = false;
        let mut site: Option<String> = None;
        for part in modifiers.split(',') {
            if let Some(n) = part.strip_prefix("boost=") {
                boost_val = n.parse().ok();
            } else if part.starts_with("downrank") {
                is_downrank = true;
                if let Some(n) = part.strip_prefix("downrank=") {
                    boost_val = n.parse().ok();
                }
            } else if let Some(s) = part.strip_prefix("site=") {
                site = Some(s.to_string());
            }
        }
        if let Some(s) = site {
            let id = domain_str_to_id(&s);
            if is_downrank {
                parsed.site_downrank.insert(id, boost_val.unwrap_or(1.0));
            } else if let Some(b) = boost_val {
                parsed.site_boost.insert(id, b);
            }
        } else if !pattern.is_empty() {
            if let Some(b) = boost_val {
                parsed.path_boost.push((pattern, b));
            }
        }
    }
    (name, description, author, public, avatar, parsed)
}

pub fn store_from_raw(raw_text: String, source_url: String) -> StoredGoggle {
    let id = url_to_id(&source_url);
    let (name, description, author, public, avatar, rules) = parse_goggle(&raw_text);
    let content_hash = url_to_id(&raw_text);

    let goggle = StoredGoggle {
        id,
        name,
        description,
        author,
        public,
        avatar,
        source_url,
        raw_text,
        fetched_at: chrono::Utc::now().timestamp(),
        content_hash,
        rules,
    };

    put(&goggle);
    goggle
}
#[derive(Debug)]
pub enum FetchError {
    Network(reqwest::Error),
    EmptyBody,
}

pub async fn fetch_and_store(url: String) -> Result<StoredGoggle, FetchError> {
    let response = reqwest::get(&url).await.map_err(FetchError::Network)?;
    let text = response.text().await.map_err(FetchError::Network)?;
    if text.trim().is_empty() {
        return Err(FetchError::EmptyBody);
    }
    Ok(store_from_raw(text, url))
}
pub fn resolve_active(cookie_jar: &CookieJar<'_>) -> Option<ParsedGoggle> {
    let id: u64 = cookie_jar.get("active_goggle")?.value().parse().ok()?;
    get(id).map(|stored| stored.rules)
}

pub fn domain_str_to_id(domain: &str) -> u64 {
    let fake_url = format!("https://{}/", domain);
    url_to_domain_id(&fake_url)
}
