/*
  File: web/routes/pages.rs
  Description: PriEco web pages

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created: 2026-01-31
  Last Modified: 2026-02-01

  Usage: Visit them in a browser
  TODO:
*/

/*
  Import system libraries
*/
use std::{collections::HashMap, net::IpAddr};

/*
  Import external libraries
*/
use rocket::{
    Request, State, get, head,
    http::{CookieJar, uri::Host},
    request::{FromRequest, Outcome},
    serde::json::Value as RocketValue,
};
use rocket_dyn_templates::Template;
use serde_json::{Value, json};
use urlencoding::encode;

/*
  Import own libraries
*/
use crate::{
    globals::{CSS_VERSION, EmbeddingService, JS_VERSION, ROCKSDB_INDEX},
    web::{functions::search_endpoint, modules::settings},
};

/*
Know user IP
Used for location and language auto detection
*/
pub struct ClientIp(pub IpAddr);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(cf_ip) = req.headers().get_one("CF-Connecting-IP") {
            if let Ok(ip) = cf_ip.parse::<IpAddr>() {
                return Outcome::Success(ClientIp(ip));
            }
        }
        if let Some(xff) = req.headers().get_one("X-Forwarded-For") {
            if let Some(ip_str) = xff.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                    return Outcome::Success(ClientIp(ip));
                }
            }
        }
        if let Some(real_ip) = req.headers().get_one("X-Real-IP") {
            if let Ok(ip) = real_ip.parse::<IpAddr>() {
                return Outcome::Success(ClientIp(ip));
            }
        }
        if let Some(socket_addr) = req.remote() {
            return Outcome::Success(ClientIp(socket_addr.ip()));
        }
        Outcome::Error((rocket::http::Status::BadRequest, ()))
    }
}

/*
  Description: Responds if PriEco is alive

  Input:
  Output: OK
*/
#[head("/")]
pub fn index_head() -> &'static str {
    ""
}

/*
  Description: PriEco home page

  Input:
  Output: Home page html
*/
#[get("/")]
pub fn index(client_ip: ClientIp, cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let ip_addr = client_ip.0; // Extract IP address

    let mut context: HashMap<String, RocketValue> = HashMap::new();

    context.insert(String::from("css_version"), json!(CSS_VERSION));
    context.insert(String::from("js_version"), json!(JS_VERSION));

    settings::run(&mut context, &Some(ip_addr), cookie_jar, host);

    Template::render("home", &context)
}

/*
  Description: PriEco results page, just static parts and JS that loads results

  Input: Search type, Search query
  Output: Privacy Policy page html
*/
#[get("/search?<t>&<q>&<sxprmedia>&<sxprsearchsugg>")]
pub async fn search(
    t: &str,
    q: &str,
    #[allow(unused_variables)] sxprmedia: Option<&str>, // Search Expander data, route needs to accept it
    #[allow(unused_variables)] sxprsearchsugg: Option<&str>, // Search Expander data, route needs to accept it
    client_ip: ClientIp,
    cookie_jar: &CookieJar<'_>,
    host: &Host<'_>,
) -> Template {
    let mut context: HashMap<String, Value> = HashMap::from([
        // File versions
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        // Page title
        (String::from("title_query"), json!(format!("{} | ", q))),
        // Search data
        (String::from("query"), json!(q)),
        (String::from("query_enc"), json!(encode(q).into_owned())),
        (String::from("type"), json!(t)),
        // Data from cookies
        (
            String::from("lang"),
            json!(cookie_jar.get("lang").map_or("all", |c| c.value())),
        ),
        (
            String::from("loc"),
            json!(cookie_jar.get("loc").map_or("all", |c| c.value())),
        ),
        // Count of result placeholders while loading
        (
            String::from("placeholder_number"),
            json!(vec![json!(()); 10]),
        ),
    ]);

    // Search type
    if let Some(key) = match t {
        "all" => Some("all_active"),
        "img" => Some("img_active"),
        "vid" => Some("vid_active"),
        "new" => Some("new_active"),
        "shop" => Some("shop_active"),
        "map" => Some("map_active"),
        _ => None,
    } {
        context.insert(String::from(key), json!("type_active"));
    }

    // Detect bang query
    context.insert("bang".to_string(), json!(q.contains('!')));

    // Aplly cookies' settings to context
    settings::run(&mut context, &Some(client_ip.0), cookie_jar, host);

    Template::render("search", &context)
}

/*
  Description: PriEco results html, endpoint to get results as html

  Input: Search type, Search query, Location, Language
  Output: Results html
*/
#[get("/results_html?<t>&<q>&<loc>&<lang>")]
pub async fn results_htmls(
    t: &str,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
    cookie_jar: &CookieJar<'_>,
) -> Template {
    Template::render(
        "search/results",
        search_endpoint::run(t, q, lang, loc, embedding_service, cookie_jar).await,
    )
}

/*
  Description: PriEco Index size

  Input:
  Output: Index size
*/
#[get("/size")]
pub fn index_size() -> String {
    match ROCKSDB_INDEX.property_int_value("rocksdb.estimate-num-keys") {
        Ok(Some(value)) => value.to_string(),
        Ok(None) => "0".to_string(),
        Err(_) => "0".to_string(),
    }
}

/*
  Description: PriEco SW Cache version

  Input:
  Output: PriEco SW cache version
*/
#[get("/cache-ver")]
pub fn cache_ver() -> String {
    String::from("0.1.3")
}

/*
  Description: PriEco page settings html

  Input:
  Output: Settings html
*/
#[get("/settings_html")]
pub fn settings_htmls(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::new();

    context.insert(String::from("css_version"), json!(CSS_VERSION));
    context.insert(String::from("js_version"), json!(JS_VERSION));

    if !cookie_jar.get("index").is_some() {
        context.insert(
            String::from("prieco_user_stats"),
            json!(
                (format!(
                    "{:.2}",
                    cookie_jar
                        .get("prieco_searches")
                        .and_then(|c| c.value().parse::<u64>().ok())
                        .unwrap_or(1) as f64
                        / cookie_jar
                            .get("all_searches")
                            .and_then(|c| c.value().parse::<u64>().ok())
                            .unwrap_or(1) as f64
                        * 100.0
                ))
            ),
        );
    } else {
        context.insert(String::from("prieco_user_stats"), json!(100.0));
    }

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("settings", &context)
}

/*
  Description: PriEco privacy policy

  Input:
  Output: Privacy Policy page html
*/
#[get("/privacy")]
pub fn privacy(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Privacy Policy | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("legal/privacy", context)
}

/*
  Description: PriEco Web Discovery privacy policy

  Input:
  Output: Web Discovery Privacy Policy html
*/
#[get("/privacy-ext")]
pub fn ext_privacy(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Privacy Policy | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("ext/privacy", context)
}
