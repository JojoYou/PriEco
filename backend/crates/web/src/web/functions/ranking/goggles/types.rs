//! # Goggle Types
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-07-12
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Stored goggle, JSON in goggles keyspace.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Goggle {
    /// [url_to_id][prieco_core::url_to_id] of source URL.
    pub id: u64,

    /// '! name:'
    pub name: String,
    /// '! description:'
    pub description: String,
    /// Shown in the store, '! public: true'.
    pub public: bool,
    /// '! author:'
    pub author: String,
    /// '! avatar:', color of results tint.
    pub avatar: String,

    /// Parsed rules.
    pub rules: GoggleRules,

    /// Source URL.
    pub url: String,
    /// Unix time of last download.
    pub fetched_at: i64,
    /// Hash of source text, for refresh.
    pub content_hash: u64,
}

/// Ranking rules of one goggle. Domains are [domain ids][prieco_core::url_to_domain_id].
///
/// Boost value is capped at 3, boost multiplies score by 1 + 0.15 × value, downrank divides it by 1 + value.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GoggleRules {
    /// URL pattern → boost.
    pub path: Vec<(String, f64)>,

    /// Raised domains.
    pub boost: HashMap<u64, f64>,
    /// Lowered domains.
    pub downrank: HashMap<u64, f64>,
    /// Removed domains.
    pub discard: HashSet<u64>,
    /// Domains where only this goggle's rule counts.
    pub important: HashSet<u64>,

    /// Removes every domain the goggle doesn't mention.
    pub discard_by_default: bool,
}
