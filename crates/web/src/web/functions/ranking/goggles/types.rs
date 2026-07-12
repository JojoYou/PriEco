use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Goggle {
    pub id: u64,

    // Goggle
    pub name: String,
    pub description: String,
    pub public: bool,
    pub author: String,
    pub avatar: String,

    pub rules: GoggleRules,

    // Meta data
    pub url: String,
    pub fetched_at: i64,
    pub content_hash: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GoggleRules {
    pub path: Vec<(String, f64)>,

    pub boost: HashMap<u64, f64>,
    pub downrank: HashMap<u64, f64>,
    pub discard: HashSet<u64>,
    pub important: HashSet<u64>,

    pub discard_by_default: bool,
}
