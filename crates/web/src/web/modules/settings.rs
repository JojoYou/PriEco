//!  File: web/modules/settings.rs
//!  Description: PriEco settings module
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2025-09-20
//!  Last Modified: 2026-02-06
//!
//!  Usage: Run run() on any page to integrate settings to the page
//!  TODO:

/*
  Import system libraries
*/
use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr},
};

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
use prieco_core::globals::{COUNTRY_TO_LANG, IP_TO_LOC};

#[derive(Serialize)]
struct GoggleView {
    id: u64,
    name: String,
    author: String,
    description: String,
    source_url: String,
    avatar: String,
    checked: bool,
}

/// Description: Integrates settings to the page
///
/// Input: Shared context beteween functions, Optional IP address, CookieJar, Host (PriEco URL)
/// Output: None
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

    if (cookie_jar.get("lang").is_none() || cookie_jar.get("loc").is_none())
        && ip_addr != IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    {
        let loc: String = if !prieco_url.domain().as_str().ends_with(".onion") {
            match IP_TO_LOC.lookup_country(&ip_addr.to_string()) {
                Ok(Some(country)) => country,
                Ok(None) => String::new(),
                Err(_) => String::new(),
            }
        } else {
            String::from("all")
        };

        if !loc.is_empty() {
            set_cookie(cookie_jar, String::from("loc"), loc.clone(), false, true);

            if !prieco_url.domain().as_str().ends_with(".onion") {
                if let Some(lang) = COUNTRY_TO_LANG.get(loc.as_str()) {
                    set_cookie(
                        cookie_jar,
                        String::from("lang"),
                        lang.to_string(),
                        false,
                        true,
                    );
                } else {
                    set_cookie(
                        cookie_jar,
                        String::from("lang"),
                        String::from("all"),
                        false,
                        true,
                    );
                }
            } else {
                set_cookie(
                    cookie_jar,
                    String::from("lang"),
                    String::from("all"),
                    false,
                    true,
                );
            }
        } else {
            set_cookie(
                cookie_jar,
                String::from("loc"),
                String::from("all"),
                false,
                true,
            );
        }
    }
    context.insert(
        String::from("selected_loc"),
        json!(
            cookie_jar
                .get("loc")
                .map(|c| c.value().to_string())
                .unwrap_or_else(|| "all".to_string())
        ),
    );
    context.insert(
        String::from("selected_lang"),
        json!(
            cookie_jar
                .get("lang")
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

    /*let goggles_list: Vec<GoggleView> = list_public()
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
    );*/
}
