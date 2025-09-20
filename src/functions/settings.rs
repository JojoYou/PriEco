use rocket::http::CookieJar;
use serde_json::json;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

use crate::{
    globals::{COUNTRY_TO_LANG, IP_TO_LOC},
    set_cookie,
};

pub fn run(
    context: &mut HashMap<String, serde_json::Value>,
    maybe_ip: &Option<IpAddr>,
    cookie_jar: &CookieJar<'_>,
) {
    ////
    // Language & Location
    ////
    let ip_addr = maybe_ip.unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

    if cookie_jar.get("lang").is_none()
        || cookie_jar.get("loc").is_none() && ip_addr != IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
    {
        let loc = match IP_TO_LOC.lookup_country(&ip_addr.to_string()) {
            Ok(Some(country)) => country,
            Ok(None) => {
                println!("IP {} not found in database", ip_addr);
                String::new()
            }
            Err(e) => {
                println!("Error looking up {}: {}", ip_addr, e);
                String::new()
            }
        };

        if !loc.is_empty() {
            set_cookie(cookie_jar, String::from("loc"), loc.clone(), false, true);

            if let Some(lang) = COUNTRY_TO_LANG.get(loc.as_str()) {
                set_cookie(
                    cookie_jar,
                    String::from("lang"),
                    lang.to_string(),
                    false,
                    true,
                );
            }
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
            "static/css/mobile/"
        } else {
            "static/css/desktop/"
        }),
    );

    context.insert(
        String::from("css_theme"),
        json!(if theme == "light" {
            "static/css/light/"
        } else if theme == "dark" {
            "static/css/dark/"
        } else {
            "static/css/system/"
        }),
    );
}
