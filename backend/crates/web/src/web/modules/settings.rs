//! # Settings
//!
//! Adds settings to the context of any page. Call [run] on every page.
//!
//! ## Architecture
//!
//! 1. **Location:** 'loc' cookie, otherwise country of user IP by GeoLite2. Onion users get 'all'.
//! 2. **Language:** 'lang' cookie, otherwise main language of the location.
//! 3. **Options:** New tab, theme, no JS, POST, decentralized search, safe search from cookies.
//! 4. **Goggles:** Public goggles, active ones are checked.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-11-26
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr},
};

use maxminddb::geoip2::Country;
use prieco_mini_crawler::meta::GEOIP_READER;
/*
  Import external libraries
*/
use rocket::http::{CookieJar, uri::Host};
use serde::Serialize;
use serde_json::json;

/*
  Import own libraries
*/
use crate::web::functions::{general::set_cookie, ranking::goggles::list_public};
use prieco_core::globals::COUNTRY_TO_LANG;

/// Goggle in settings list.
#[derive(Serialize)]
struct GoggleView {
    id: u64,
    name: String,
    author: String,
    description: String,
    source_url: String,
    avatar: String,
    /// Goggle is active.
    checked: bool,
}

/// Adds settings to the page context.
///
/// Detected location and language are saved to cookies.
///
/// # Arguments
///
/// * `context` - Template context, shared between functions.
/// * `maybe_ip` - User IP, for location detection.
/// * `cookie_jar` - Cookies.
/// * `prieco_url` - Host, '.onion' turns off location detection.
///
/// # Returns
///
/// None
pub fn run(
    context: &mut HashMap<String, serde_json::Value>,
    maybe_ip: &Option<IpAddr>,
    cookie_jar: &CookieJar<'_>,
    prieco_url: &Host,
) {
    // OSD
    if prieco_url.domain().as_str().ends_with(".onion") {
        context.insert(String::from("osd_title"), json!(" (Onion)"));
    }

    // Language & Location
    let ip_addr = maybe_ip.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
    let onion = prieco_url.domain().as_str().ends_with(".onion");

    // Location
    if cookie_jar.get("loc").is_none() && ip_addr != IpAddr::V4(Ipv4Addr::UNSPECIFIED) {
        let loc = if onion {
            "all".to_string()
        } else {
            let record: Option<Country> = GEOIP_READER
                .lookup(ip_addr)
                .and_then(|r| r.decode())
                .ok()
                .flatten();
            record
                .and_then(|rec| rec.country.iso_code)
                .map_or("all".to_string(), |cou| cou.to_ascii_lowercase())
        };

        set_cookie(cookie_jar, "loc".into(), loc, false, true, true);
    }

    // Language
    if cookie_jar.get("lang").is_none()
        && let Some(loc) = cookie_jar.get_pending("loc").map(|c| c.value().to_string())
    {
        let lang = COUNTRY_TO_LANG
            .get(loc.as_str())
            .filter(|_| !onion)
            .map_or("all".to_string(), |l| l.to_string());
        set_cookie(cookie_jar, "lang".into(), lang, false, true, true);
    }

    context.insert(
        String::from("selected_loc"),
        json!(
            cookie_jar
                .get_pending("loc")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| "all".to_string())
        ),
    );
    context.insert(
        String::from("selected_lang"),
        json!(
            cookie_jar
                .get_pending("lang")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| "all".to_string())
        ),
    );

    if cookie_jar.get("newtab").is_some() {
        context.insert(String::from("check_newtab"), json!(1));
        context.insert(String::from("newtab"), json!("target='_blank'"));
    }

    let theme = cookie_jar
        .get("theme")
        .map(|c| c.value().to_string())
        .unwrap_or_else(|| "system".to_string());
    context.insert(String::from("selected_theme"), json!(theme));

    context.insert(
        String::from("css_path"),
        json!(if cookie_jar
            .get("screen_width")
            .and_then(|c| c.value().parse::<u32>().ok())
            .unwrap_or(1024)
            < 890
        {
            "/static/css/mobile/"
        } else {
            "/static/css/desktop/"
        }),
    );

    context.insert(
        String::from("css_theme"),
        json!(if theme == "light" {
            "/static/css/light/"
        } else if theme == "dark" {
            "/static/css/dark/"
        } else {
            "/static/css/system/"
        }),
    );

    if cookie_jar.get("js").is_some() {
        context.insert(String::from("check_js"), json!(1));
    }

    if cookie_jar.get("post").is_some() {
        context.insert(String::from("check_post"), json!(1));
    }

    if cookie_jar.get("dec").map(|c| c.value()) == Some("0") {
        context.insert(String::from("check_dec"), json!(1));
    }

    if cookie_jar.get("safe").map(|c| c.value()) == Some("0") {
        context.insert(String::from("check_safe_off"), json!(1));
    }

    // Goggles
    let active_ids: HashSet<u64> = cookie_jar
        .get("active_goggles")
        .map(|c| {
            c.value()
                .split(',')
                .filter_map(|p| p.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let goggles_list: Vec<GoggleView> = list_public()
        .into_iter()
        .map(|g| GoggleView {
            checked: active_ids.contains(&g.id),
            id: g.id,
            name: g.name,
            author: g.author,
            description: g.description,
            source_url: g.url,
            avatar: g.avatar.trim_start_matches('#').to_string(),
        })
        .collect();

    context.insert(String::from("goggles"), json!(goggles_list));
    context.insert(
        String::from("goggles_ids"),
        json!(
            active_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
}
