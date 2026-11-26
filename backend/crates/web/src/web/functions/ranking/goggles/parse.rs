//! # Goggle Parse
//!
//! Turns goggle text into [GoggleRules]. Unknown lines are ignored.
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

use super::types::GoggleRules;
use prieco_core::url_to_domain_id;

/// Domain id of a bare domain like 'example.com'.
pub fn domain_str_to_id(domain: &str) -> u64 {
    url_to_domain_id(&format!("https://{}/", domain))
}

/// Parsed goggle file.
pub struct GoggleMeta {
    pub name: String,
    pub description: String,
    pub author: String,
    pub public: bool,
    pub avatar: String,
    pub rules: GoggleRules,
}

/// Parses goggle text, see [format][super].
pub fn parse_goggle(raw: &str) -> GoggleMeta {
    let mut meta = GoggleMeta {
        name: String::new(),
        description: String::new(),
        author: String::new(),
        public: false,
        avatar: String::new(),
        rules: GoggleRules::default(),
    };

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix('!') {
            parse_meta_line(rest.trim(), &mut meta);
            continue;
        }
        if line == "$discard" {
            meta.rules.discard_by_default = true;
            continue;
        }

        parse_rule_line(line, &mut meta.rules);
    }

    meta
}

/// Parses '! key: value' line, '!' already removed.
fn parse_meta_line(line: &str, meta: &mut GoggleMeta) {
    let Some((k, v)) = line.split_once(':') else {
        return;
    };
    let v = v.trim().to_string();
    match k.trim() {
        "name" => meta.name = v,
        "description" => meta.description = v,
        "author" => meta.author = v,
        "public" => meta.public = v == "true",
        "avatar" => meta.avatar = v,
        _ => {}
    }
}

/// Parses 'pattern$modifiers' line.
///
/// Modifiers: `boost=N`, `downrank` or `downrank=N` (default 1), `discard`, `important`, `site=domain`.
fn parse_rule_line(line: &str, rules: &mut GoggleRules) {
    let Some((pattern, modifiers)) = line.split_once('$') else {
        return;
    };

    let mut boost_val: Option<f64> = None;
    let mut is_downrank = false;
    let mut is_discard = false;
    let mut is_important = false;
    let mut site: Option<&str> = None;

    for part in modifiers.split(',') {
        if let Some(n) = part.strip_prefix("boost=") {
            boost_val = n.parse().ok();
        } else if let Some(n) = part.strip_prefix("downrank=") {
            is_downrank = true;
            boost_val = n.parse().ok();
        } else if part == "downrank" {
            is_downrank = true;
        } else if part == "discard" {
            is_discard = true;
        } else if part == "important" {
            is_important = true;
        } else if let Some(s) = part.strip_prefix("site=") {
            site = Some(s);
        }
    }
    if let Some(s) = site {
        let id = domain_str_to_id(s);
        if is_important {
            rules.important.insert(id);
        }
        if is_discard {
            rules.discard.insert(id);
        } else if is_downrank {
            rules.downrank.insert(id, boost_val.unwrap_or(1.0));
        } else if let Some(b) = boost_val {
            rules.boost.insert(id, b);
        }
    } else if let Some(b) = boost_val
        && !is_downrank
        && !is_discard
    {
        let pattern = pattern.trim();
        if !pattern.is_empty() {
            rules.path.push((pattern.to_string(), b));
        }
    }
}
