//!  File: web/functions/additional/ip_widget.rs
//!  Description: Public IP address widget
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-11-25
//!  Last Modified: 2026-11-25

use std::net::IpAddr;

use maxminddb::geoip2::Country;
use prieco_core::globals::COUNTRY_TO_LANG;
use prieco_mini_crawler::meta::GEOIP_READER;
use serde_json::{Value, json};

const TRIGGERS: [&str; 8] = [
    "ip",
    "my ip",
    "ip address",
    "my ip address",
    "what is my ip",
    "what's my ip",
    "whats my ip",
    "what is my ip address",
];

pub fn get_ip_widget(q: &str, ip: IpAddr) -> Option<Value> {
    let q = q.trim().to_lowercase();
    if !TRIGGERS.contains(&q.as_str()) {
        return None;
    }

    let public = match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified())
        }
        IpAddr::V6(v6) => {
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local())
        }
    };
    if !public {
        return None;
    }

    let record: Option<Country> = GEOIP_READER
        .lookup(ip)
        .and_then(|r| r.decode())
        .ok()
        .flatten();
    let name = record
        .as_ref()
        .and_then(|rec| rec.country.names.english.map(str::to_string));
    let code = record
        .as_ref()
        .and_then(|rec| rec.country.iso_code.map(str::to_string));
    let lang = code
        .as_deref()
        .and_then(|c| COUNTRY_TO_LANG.get(c.to_ascii_lowercase().as_str()))
        .map(|l| l.to_string());

    Some(json!({
        "ip": ip.to_string(),
        "country": name,
        "code": code,
        "lang": lang,
    }))
}
