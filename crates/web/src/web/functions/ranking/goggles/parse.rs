use super::types::GoggleRules;
use prieco_core::url_to_domain_id;

pub fn domain_str_to_id(domain: &str) -> u64 {
    url_to_domain_id(&format!("https://{}/", domain))
}

pub struct GoggleMeta {
    pub name: String,
    pub description: String,
    pub author: String,
    pub public: bool,
    pub avatar: String,
    pub rules: GoggleRules,
}

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
    }
}
