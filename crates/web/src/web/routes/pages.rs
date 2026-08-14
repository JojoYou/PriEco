//!  File: web/routes/pages.rs
//!  Description: PriEco web pages
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-01-31
//!  Last Modified: 2026-02-01
//!
//!  Usage: Visit them in a browser
//!  TODO:

/*
  Import system libraries
*/
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    io::Cursor,
    net::IpAddr,
};

/*
  Import external libraries
*/
use chrono::Utc;
use dotenv_codegen::dotenv;
use prieco_blob::decode::decode_blob_to_html_rendered;
use rocket::{
    Request, Response, State,
    form::{Form, FromForm},
    get, head,
    http::{
        ContentType, Cookie, CookieJar, Header, SameSite, Status,
        uri::{Host, Origin},
    },
    post,
    request::{FromRequest, Outcome},
    response::{Redirect, Responder, Result as RocketResult, content::RawHtml, status::NotFound},
    serde::json::{Json, Value as RocketValue},
    time::Duration,
    uri,
};
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;
use urlencoding::{decode, encode};

/*
  Import own libraries
*/
use crate::web::{
    functions::{
        general::{get_domain, set_cookie},
        ranking::goggles::{fetch_and_store, get_goggle_ids, load_goggles, refresh_stale_goggles},
        search_endpoint::{self, UserQtPrefs, get_user_qt_prefs},
    },
    modules::settings,
};
use prieco_core::{
    BANGS, CLIENT, PRIECO_FJALL, TANTIVY_INDEX,
    globals::{ANALYTICS, CSS_VERSION, EmbeddingService, JS_VERSION, UserAgent},
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

/// Description: Responds if PriEco is alive
///
/// Input:
/// Output: OK
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
    let ip_addr = client_ip.0;

    let mut context: HashMap<String, RocketValue> = HashMap::new();

    context.insert(String::from("css_version"), json!(CSS_VERSION));
    context.insert(String::from("js_version"), json!(JS_VERSION));

    settings::run(&mut context, &Some(ip_addr), cookie_jar, host);

    let no_js = cookie_jar.get("js").is_some();
    context.insert(String::from("no_js"), json!(no_js));

    if no_js {
        let mut settings_ctx: HashMap<String, Value> = HashMap::new();
        settings_ctx.insert("css_version".into(), json!(CSS_VERSION));
        settings_ctx.insert("js_version".into(), json!(JS_VERSION));
        settings_ctx.insert(String::from("no_js"), json!(no_js));

        settings::run(&mut settings_ctx, &Some(client_ip.0), cookie_jar, host);
        context.insert(String::from("settings"), json!(settings_ctx));
    }

    // Shortcuts
    let mut shortcuts = Vec::new();
    if let Some(cookie) = cookie_jar.get("shortcuts") {
        let decoded = decode(cookie.value()).unwrap_or_default().into_owned();
        let items: Vec<&str> = decoded.split(',').collect();

        for (i, item) in items.iter().enumerate() {
            if i == 0 {
                continue;
            }

            let parts: Vec<&str> = item.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let icon = format!(
                    "/proxy?u={}",
                    urlencoding::encode(&format!(
                        "https://fav.prieco.net/icon?url={}&size=32",
                        urlencoding::encode(&get_domain(&url, false))
                    ))
                );

                let display_name = if name.len() > 10 {
                    format!("{}...", &name[..7])
                } else {
                    name.clone()
                };

                shortcuts.push(ShortcutView {
                    id: i,
                    name,
                    display_name,
                    url,
                    icon,
                });
            }
        }
    }
    context.insert(String::from("shortcuts"), json!(shortcuts));

    Template::render("home", &context)
}

#[derive(Serialize)]
pub struct ShortcutView {
    pub id: usize,
    pub name: String,
    pub display_name: String,
    pub url: String,
    pub icon: String,
}

#[derive(FromForm)]
pub struct ShortcutAction<'r> {
    action: &'r str,
    shortcutID: Option<usize>,
    shortcutName: Option<&'r str>,
    shortcutURL: Option<&'r str>,
}
#[post("/", data = "<form>")]
pub fn handle_shortcuts(form: Form<ShortcutAction<'_>>, cookie_jar: &CookieJar<'_>) -> Redirect {
    let mut items: Vec<String> = Vec::new();

    if let Some(cookie) = cookie_jar.get("shortcuts") {
        let decoded = decode(cookie.value()).unwrap_or_default().into_owned();
        items = decoded.split(',').map(|s| s.to_string()).collect();
    }

    match form.action {
        "add" => {
            if let (Some(name), Some(url)) = (form.shortcutName, form.shortcutURL) {
                if items.is_empty() {
                    items.push("dummy=dummy".to_string());
                }

                let final_url = if url.starts_with("http://") || url.starts_with("https://") {
                    url.to_string()
                } else {
                    format!("https://{}", url)
                };

                items.push(format!("{}={}", name, final_url));
            }
        }
        "edit" => {
            if let (Some(id), Some(name), Some(url)) =
                (form.shortcutID, form.shortcutName, form.shortcutURL)
            {
                if id < items.len() {
                    let final_url = if url.starts_with("http://") || url.starts_with("https://") {
                        url.to_string()
                    } else {
                        format!("https://{}", url)
                    };

                    items[id] = format!("{}={}", name, final_url);
                }
            }
        }
        "delete" => {
            if let Some(id) = form.shortcutID {
                if id < items.len() {
                    items.remove(id);
                }
            }
        }
        _ => {}
    }

    set_cookie(
        cookie_jar,
        String::from("shortcuts"),
        encode(&items.join(",")).into_owned(),
        true,
        true,
    );
    Redirect::to(uri!(index))
}

/*
  Description: PriEco results page, just static parts and JS that loads results

  Input: Search type, Search query
  Output: Privacy Policy page html
*/
// Handle POST request
#[derive(FromForm)]
pub struct SearchForm<'r> {
    t: &'r str,
    q: &'r str,
    sxprmedia: Option<&'r str>,
    sxprsearchsugg: Option<&'r str>,
}
#[post("/search", data = "<form>")]
pub async fn search_post(
    form: Form<SearchForm<'_>>,
    client_ip: ClientIp,
    user_agent: UserAgent<'_>,
    cookie_jar: &CookieJar<'_>,
    host: &Host<'_>,
    uri: &Origin<'_>,
    embedding_service: &State<EmbeddingService>,
) -> Result<(ContentType, Template), Redirect> {
    search(
        form.t,
        form.q,
        form.sxprmedia,
        form.sxprsearchsugg,
        client_ip,
        user_agent,
        cookie_jar,
        host,
        uri,
        embedding_service,
    )
    .await
}

#[get("/search?<t>&<q>&<sxprmedia>&<sxprsearchsugg>")]
pub async fn search(
    t: &str,
    q: &str,
    #[allow(unused_variables)] sxprmedia: Option<&str>, // Search Expander data, route needs to accept it
    #[allow(unused_variables)] sxprsearchsugg: Option<&str>, // Search Expander data, route needs to accept it
    client_ip: ClientIp,
    user_agent: UserAgent<'_>,
    cookie_jar: &CookieJar<'_>,
    host: &Host<'_>,
    uri: &Origin<'_>,
    embedding_service: &State<EmbeddingService>,
) -> Result<(ContentType, Template), Redirect> {
    let raw_qt_cookie = cookie_jar
        .get("prieco_qt_prefs")
        .map(|c| c.value())
        .unwrap_or("{}");

    let active_goggle_ids = get_goggle_ids(None, Some(cookie_jar));

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
        (
            String::from("current_path"),
            json!(encode(&uri.to_string()).into_owned()),
        ),
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
        (
            String::from("qt_prefs_enc"),
            json!(encode(raw_qt_cookie).into_owned()),
        ),
        (
            String::from("active_goggle_count"),
            json!(active_goggle_ids.len()),
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

    // No JS enabled
    let no_js = cookie_jar.get("js").is_some();
    context.insert(String::from("no_js"), json!(no_js));

    if no_js {
        if let Some((_clean_query, redirects)) = multibangs(q) {
            if redirects.len() == 1 {
                return Err(Redirect::to(redirects[0].url.clone()));
            } else if redirects.len() > 1 {
                let mut multibang_ctx = HashMap::new();
                multibang_ctx.insert("query", json!(q));
                multibang_ctx.insert("redirects", json!(redirects));
                multibang_ctx.insert("css_version", json!(CSS_VERSION));

                return Ok((
                    ContentType::HTML,
                    Template::render("multibang", &multibang_ctx),
                ));
            }
        }

        let lang = cookie_jar.get("lang").map_or("all", |c| c.value());
        let loc = cookie_jar.get("loc").map_or("all", |c| c.value());
        let active_goggles = load_goggles(&get_goggle_ids(None, Some(cookie_jar)));

        let user_qt_prefs: UserQtPrefs = serde_json::from_str(raw_qt_cookie).unwrap_or_default();

        // Is user mobile
        let ua = user_agent.0.to_lowercase();
        let user_is_mobile = ua.contains("mobi") || ua.contains("android") || ua.contains("iphone");

        // Results
        let results_ctx = search_endpoint::run(
            t,
            q,
            lang,
            loc,
            embedding_service,
            active_goggles,
            &user_qt_prefs,
            user_is_mobile,
        )
        .await;

        context.insert(String::from("search_results"), json!(results_ctx));
        ANALYTICS.record_query();

        // Settings
        let mut settings_ctx: HashMap<String, Value> = HashMap::new();
        settings_ctx.insert("css_version".into(), json!(CSS_VERSION));
        settings_ctx.insert("js_version".into(), json!(JS_VERSION));
        settings_ctx.insert(String::from("no_js"), json!(no_js));

        settings::run(&mut settings_ctx, &Some(client_ip.0), cookie_jar, host);
        context.insert(String::from("settings"), json!(settings_ctx));
    }

    // Aplly cookies' settings to context
    settings::run(&mut context, &Some(client_ip.0), cookie_jar, host);

    // Analytics
    ANALYTICS.record_visitor(
        &client_ip.0.to_string(),
        user_agent.0,
        &host.to_string(),
        cookie_jar.get("loc").map(|c| c.value()),
    );

    Ok((ContentType::HTML, Template::render("search", &context)))
}

// Bangs
#[derive(Serialize, Clone)]
pub struct BangRedirect {
    pub title: String,
    pub domain: String,
    pub url: String,
}

fn multibangs(query: &str) -> Option<(String, Vec<BangRedirect>)> {
    let words: Vec<&str> = query.split_whitespace().collect();
    let bang_candidates: Vec<&str> = words
        .iter()
        .filter(|w| w.starts_with('!'))
        .cloned()
        .collect();

    if bang_candidates.is_empty() {
        return None;
    }

    let mut bangs = Vec::new();
    for token in &bang_candidates {
        let candidate = token[1..].to_lowercase();
        if let Some(bang) = BANGS.get(&candidate) {
            bangs.push(bang);
        }
    }

    if bangs.is_empty() {
        return None;
    }

    let clean_query = words
        .into_iter()
        .filter(|w| !bang_candidates.contains(w))
        .collect::<Vec<_>>()
        .join(" ");

    let encoded_query = urlencoding::encode(&clean_query)
        .into_owned()
        .replace("%2F", "/");

    let mut redirects = Vec::new();
    for bang in bangs {
        let target_url = if clean_query.is_empty() {
            format!("https://{}", bang.d)
        } else {
            bang.u.replace("{{{s}}}", &encoded_query)
        };

        redirects.push(BangRedirect {
            title: bang.t.clone(),
            domain: bang.d.clone(),
            url: target_url,
        });
    }

    Some((clean_query, redirects))
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
    user_agent: UserAgent<'_>,
) -> Template {
    ANALYTICS.record_query();

    let active_goggles = load_goggles(&get_goggle_ids(None, Some(cookie_jar)));
    let user_qt_prefs = get_user_qt_prefs(cookie_jar);

    // Is user mobile
    let ua = user_agent.0.to_lowercase();
    let user_is_mobile = ua.contains("mobi") || ua.contains("android") || ua.contains("iphone");

    let mut ctx = search_endpoint::run(
        t,
        q,
        lang,
        loc,
        embedding_service,
        active_goggles,
        &user_qt_prefs,
        user_is_mobile,
    )
    .await;

    let goggle_count = get_goggle_ids(None, Some(cookie_jar)).len();
    ctx.insert(String::from("active_goggle_count"), json!(goggle_count));
    ctx.insert(
        String::from("query_enc"),
        json!(urlencoding::encode(q).into_owned()),
    );
    ctx.insert(String::from("type"), json!(t));
    ctx.insert(String::from("lang"), json!(lang));
    ctx.insert(String::from("loc"), json!(loc));
    ctx.insert(String::from("no_js"), json!(false));

    Template::render("search/results", ctx)
}

#[get("/archive/<id>")]
pub async fn view_blob(id: u64) -> Option<RawHtml<String>> {
    let raw_blob = match PRIECO_FJALL.blobs_ks.get(&id.to_le_bytes()) {
        Ok(Some(blob)) => blob,
        _ => return None,
    };

    let proxy_prefix = "/proxy?u=";
    let body_html = decode_blob_to_html_rendered(&raw_blob, proxy_prefix);

    let full_page = format!(
        r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>PriEco Document: {}</title>
        </head>
        <body>
            {}
        </body>
        </html>"#,
        id, body_html
    );

    Some(RawHtml(full_page))
}

/*
  Description: PriEco Analytics

  Input:
  Output: Analytics page html
*/
#[get("/stats")]
pub async fn stats(client_ip: ClientIp, cookie_jar: &CookieJar<'_>, host: &Host<'_>) -> Template {
    // Index
    let index_size_raw: u64 = match TANTIVY_INDEX.reader() {
        Ok(reader) => {
            let searcher = reader.searcher();
            searcher
                .segment_readers()
                .iter()
                .map(|s| s.num_docs() as u64)
                .sum()
        }
        Err(_) => 0,
    };

    let index_display = if index_size_raw >= 1_000_000_000 {
        (
            format!("{:.2}", index_size_raw as f64 / 1_000_000_000.0),
            "B results",
        )
    } else if index_size_raw >= 1_000_000 {
        (
            format!("{:.2}", index_size_raw as f64 / 1_000_000.0),
            "M results",
        )
    } else if index_size_raw >= 1_000 {
        (
            format!("{:.1}", index_size_raw as f64 / 1_000.0),
            "K results",
        )
    } else {
        (index_size_raw.to_string(), "results")
    };

    // Search volume chart
    let raw_days = ANALYTICS.daily_queries(30);
    let max_count = raw_days.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let search_days: Vec<Value> = raw_days
        .iter()
        .map(|(date, count)| {
            let height_pct = (*count as f64 / max_count as f64 * 100.0) as u64;
            json!({ "date": date, "count": count.to_string(), "height_pct": height_pct.max(2) })
        })
        .collect();
    let search_today = raw_days.last().map(|(_, c)| *c).unwrap_or(0);
    let len = raw_days.len();
    let search_x_labels: Vec<String> = raw_days
        .iter()
        .enumerate()
        .filter_map(|(i, (date, _))| {
            let step = (len / 5).max(1);
            if i % step == 0 || i == len - 1 {
                Some(date[5..].to_string())
            } else {
                None
            }
        })
        .collect();

    // Visitors + pageviews
    let (visitors_today, visitors_yesterday, pageviews_today, _) = ANALYTICS.visitor_stats();
    let visitors_delta = if visitors_yesterday == 0 {
        0.0
    } else {
        (visitors_today as f64 - visitors_yesterday as f64) / visitors_yesterday as f64 * 100.0
    };

    // Countries
    let countries: Vec<Value> = ANALYTICS
        .top_countries()
        .into_iter()
        .map(|(cc, count)| {
            let (name, flag) = country_info(&cc);
            let pct = if visitors_today == 0 {
                0.0
            } else {
                count as f64 / visitors_today as f64 * 100.0
            };
            json!({ "flag": flag, "name": name, "pct": format!("{:.1}", pct) })
        })
        .collect();

    // API
    let (api_today, api_yesterday) = ANALYTICS.api_stats_today_yesterday();
    let api_delta = if api_yesterday == 0 {
        0.0
    } else {
        (api_today as f64 - api_yesterday as f64) / api_yesterday as f64 * 100.0
    };

    let mut context: HashMap<String, Value> = HashMap::from([
        (
            String::from("generated_at"),
            json!(Utc::now().format("updated %H:%M UTC").to_string()),
        ),
        (String::from("index_size_display"), json!(index_display.0)),
        (String::from("index_size_unit"), json!(index_display.1)),
        (String::from("search_days"), json!(search_days)),
        (
            String::from("search_today"),
            json!(search_today.to_string()),
        ),
        (String::from("search_x_labels"), json!(search_x_labels)),
        (
            String::from("visitors_24h"),
            json!(visitors_today.to_string()),
        ),
        (
            String::from("visitors_delta"),
            json!(format!("{:.1}", visitors_delta.abs())),
        ),
        (String::from("visitors_up"), json!(visitors_delta >= 0.0)),
        (
            String::from("pageviews_24h"),
            json!(pageviews_today.to_string()),
        ),
        (String::from("countries"), json!(countries)),
        (String::from("api_today"), json!(api_today.to_string())),
        (
            String::from("api_delta"),
            json!(format!("{:.1}", api_delta.abs())),
        ),
        (String::from("api_up"), json!(api_delta >= 0.0)),
    ]);

    settings::run(&mut context, &Some(client_ip.0), cookie_jar, host);
    Template::render("analytics", context)
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

// No JS settings
#[derive(FromForm)]
pub struct SettingsForm<'r> {
    newtab: Option<&'r str>,
    index: Option<&'r str>,
    theme: Option<&'r str>,
    lang: Option<&'r str>,
    loc: Option<&'r str>,
    js: Option<&'r str>,
    post: Option<&'r str>,
}
#[post("/settings_update", data = "<form>")]
pub fn settings_update(form: Form<SettingsForm<'_>>, cookie_jar: &CookieJar<'_>) -> Redirect {
    // Helpers
    let remove_cookie = |name: &str| {
        let mut cookie = Cookie::from(name.to_string());
        cookie.set_path("/");
        cookie_jar.remove(cookie);
    };

    let add_cookie = |name: &str, value: &str| {
        set_cookie(cookie_jar, name.to_string(), value.to_string(), true, true);
    };

    if form.newtab.is_some() {
        add_cookie("newtab", "1");
    } else {
        remove_cookie("newtab");
    }

    if form.index.is_some() {
        add_cookie("index", "1");
    } else {
        remove_cookie("index");
    }

    if let Some(theme) = form.theme {
        if theme == "system" {
            remove_cookie("theme");
        } else {
            add_cookie("theme", theme);
        }
    }

    if let Some(lang) = form.lang {
        add_cookie("lang", lang);
    }

    if let Some(loc) = form.loc {
        add_cookie("loc", loc);
    }

    if form.js.is_some() {
        add_cookie("js", "1");
    } else {
        remove_cookie("js");
    }

    if form.post.is_some() {
        add_cookie("post", "1");
    } else {
        remove_cookie("post");
    }

    Redirect::to(uri!("/"))
}

#[get("/set?<lang>&<loc>&<theme>&<newtab>&<js>&<index>&<post>&<return_to>")]
pub fn set_preferences(
    lang: Option<String>,
    loc: Option<String>,
    theme: Option<String>,
    newtab: Option<u8>,
    js: Option<u8>,
    index: Option<u8>,
    post: Option<u8>,
    return_to: Option<String>,
    cookie_jar: &CookieJar<'_>,
) -> Redirect {
    let apply_cookie = |name: &str, value: String| {
        cookie_jar.add(
            Cookie::build((name.to_string(), value))
                .path("/")
                .same_site(SameSite::Strict)
                .secure(true)
                .max_age(Duration::days(365))
                .build(),
        );
    };
    let remove_cookie = |name: &str| {
        let mut cookie = Cookie::from(name.to_string());
        cookie.set_path("/");
        cookie_jar.remove(cookie);
    };

    if let Some(v) = lang {
        apply_cookie("lang", v);
    }
    if let Some(v) = loc {
        apply_cookie("loc", v);
    }
    if let Some(v) = theme {
        apply_cookie("theme", v);
    }

    if let Some(v) = newtab {
        if v == 1 {
            apply_cookie("newtab", "1".to_string());
        } else {
            remove_cookie("newtab");
        }
    }

    if let Some(v) = js {
        if v == 1 {
            apply_cookie("js", "1".to_string());
        } else {
            remove_cookie("js");
        }
    }

    if let Some(v) = index {
        if v == 1 {
            apply_cookie("index", "1".to_string());
        } else {
            remove_cookie("index");
        }
    }

    if let Some(v) = post {
        if v == 1 {
            apply_cookie("post", "1".to_string());
        } else {
            remove_cookie("post");
        }
    }

    let redirect_url = match return_to {
        Some(url) if url.starts_with('/') => url,
        _ => String::from("/"),
    };

    Redirect::to(redirect_url)
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

/*
  Description: PriEco URL submit

  Input:
  Output: URL submit page html
*/
#[get("/submit")]
pub fn submit(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Submit | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("search/submit", context)
}

#[derive(FromForm)]
pub struct SubmitForm {
    message: String,
}

#[derive(Serialize)]
struct PageDataPayload {
    page_url: String,
    links: Vec<String>,
}

#[post("/submit", data = "<form>")]
pub async fn submit_post(form: Form<SubmitForm>) -> String {
    let links: Vec<String> = form
        .message
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    let payload = PageDataPayload {
        page_url: "user-submission".to_string(),
        links,
    };

    let client = reqwest::Client::new();
    let res = client
        .post("https://crawler.prieco.net/web-discovery")
        .json(&payload)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            "Successfully sent to crawler!".to_string()
        }
        Ok(response) => {
            format!("Crawler rejected the payload: {}", response.status())
        }
        Err(e) => {
            format!("Failed to reach crawler: {}", e)
        }
    }
}

/*
  Description: PriEco roadmap

  Input:
  Output: Roadmap page html
*/
#[get("/roadmap")]
pub fn roadmap(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Roadmap | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("roadmap", context)
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RoadmapVote {
    pub feature: String,
    pub is_like: bool,
}
#[post("/roadmap/vote", format = "json", data = "<vote>")]
pub async fn submit_roadmap_vote(vote: Json<RoadmapVote>) -> Status {
    let vote_emoji = if vote.is_like { "👍" } else { "👎" };
    let signal_message = format!(
        "**PriEco Roadmap Vote**\n**Feature:** {}\n**Vote:** {}",
        vote.feature, vote_emoji
    );

    let payload = serde_json::json!({
        "message": signal_message,
        "number": dotenv!("SIGNAL_BOT_NUMBER"),
        "recipients": [dotenv!("SIGNAL_RECIPIENT_NUMBER")],
        "text_mode": "styled",
    });

    match CLIENT
        .post("http://localhost:8071/v2/send")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                println!("Vote for '{}' successfully sent to Signal!", vote.feature);
                Status::Ok
            } else {
                let error_text = response.text().await.unwrap_or_default();
                println!("Signal API Error ({}): {}", status, error_text);
                Status::InternalServerError
            }
        }
        Err(e) => {
            println!("Failed to send vote to Signal (Timeout/Network): {}", e);
            Status::InternalServerError
        }
    }
}

/*
  Description: PriEco Goggles store

  Input:
  Output: Goggle store html
*/
#[get("/goggles")]
pub fn goggles(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    tokio::spawn(async {
        refresh_stale_goggles().await;
    });

    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Goggles | ")),
    ]);

    context.insert(String::from("no_js"), json!(cookie_jar.get("js").is_some()));

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("search/goggles", context)
}

#[get("/goggles/load?<url>")]
pub async fn load_goggle(url: String, cookie_jar: &CookieJar<'_>) -> Redirect {
    let goggle = fetch_and_store(url).await;

    if goggle.id != 0 {
        let mut ids: Vec<u64> = cookie_jar
            .get("active_goggles")
            .map(|c| {
                c.value()
                    .split(',')
                    .filter_map(|p| p.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default();

        if !ids.contains(&goggle.id) {
            ids.push(goggle.id);
        }

        let joined = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let mut cookie = Cookie::new("active_goggles", joined);
        cookie.set_path("/");
        cookie_jar.add(cookie);
    }

    Redirect::to("/goggles")
}

#[get("/goggles/apply?<ids>")]
pub fn apply_goggles(ids: Option<Vec<u64>>, cookie_jar: &CookieJar<'_>) -> Redirect {
    let ids = ids.unwrap_or_default();

    if ids.is_empty() {
        let mut cookie = Cookie::from("active_goggles");
        cookie.set_path("/");
        cookie_jar.remove(cookie);
    } else {
        let joined = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let mut cookie = Cookie::new("active_goggles", joined);
        cookie.set_path("/");
        cookie_jar.add(cookie);
    }

    Redirect::to("/goggles")
}

#[get("/static/css/goggles_tint.css.hbs?<tint>&<target>")]
pub fn goggles_tint(tint: Option<&str>, target: Option<&str>) -> (ContentType, String) {
    let hex = validate_hex(tint.unwrap_or("14141E")).unwrap_or_else(|| String::from("14141E"));
    let target = target.unwrap_or_default();

    let css = format!(".{target} {{ background-color: #{hex}66; }}");

    (ContentType::CSS, css)
}

fn validate_hex(hex: &str) -> Option<String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_string())
    } else {
        None
    }
}

#[derive(FromForm)]
pub struct QtUpdateForm {
    return_url: String,
    prefs: HashMap<String, String>,
}

#[post("/quick_tune_update", data = "<form>")]
pub fn update_qt(form: Form<QtUpdateForm>, cookie_jar: &CookieJar<'_>) -> Redirect {
    let mut user_prefs = cookie_jar
        .get("prieco_qt_prefs")
        .and_then(|c| serde_json::from_str::<UserQtPrefs>(c.value()).ok())
        .unwrap_or_default();

    for (domain, action) in form.prefs.iter() {
        user_prefs.boost.retain(|d| d != domain);
        user_prefs.downrank.retain(|d| d != domain);
        user_prefs.discard.retain(|d| d != domain);

        match action.as_str() {
            "boost" => user_prefs.boost.push(domain.clone()),
            "downrank" => user_prefs.downrank.push(domain.clone()),
            "discard" => user_prefs.discard.push(domain.clone()),
            _ => {}
        }
    }

    let cookie_str = serde_json::to_string(&user_prefs).unwrap();
    let mut cookie = Cookie::new("prieco_qt_prefs", cookie_str);
    cookie.set_path("/");
    cookie.set_max_age(rocket::time::Duration::days(365));
    cookie_jar.add(cookie);

    Redirect::to(form.return_url.clone())
}

pub struct QuickTuneExport(String);

impl<'r, 'o: 'r> Responder<'r, 'o> for QuickTuneExport {
    fn respond_to(self, _: &'r Request<'_>) -> RocketResult<'o> {
        Response::build()
            .header(ContentType::Plain)
            .header(Header::new(
                "Content-Disposition",
                "attachment; filename=\"my_prieco.goggle\"",
            ))
            .sized_body(self.0.len(), Cursor::new(self.0))
            .ok()
    }
}

#[get("/quick_tune/export")]
pub fn export_quick_tune(cookie_jar: &CookieJar<'_>) -> QuickTuneExport {
    let user_qt_prefs = get_user_qt_prefs(cookie_jar);

    let mut out = String::new();
    out.push_str("! name: My PriEco Goggle\n");
    out.push_str("! description: Exported Goggle\n");
    out.push_str("! public: false\n\n");

    for domain in &user_qt_prefs.boost {
        out.push_str(&format!("$boost=3,site={}\n", domain));
    }
    for domain in &user_qt_prefs.downrank {
        out.push_str(&format!("$downrank=3,site={}\n", domain));
    }
    for domain in &user_qt_prefs.discard {
        out.push_str(&format!("$discard,site={}\n", domain));
    }

    QuickTuneExport(out)
}

/*
  Description: PriEco big thank you for money tranfer, usually donation

  Input:
  Output: Thank you html
*/
#[get("/thanks")]
pub fn thanks(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Thank You! | ")),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Template::render("legal/thanks", context)
}

/* Helper functions */
pub fn country_info(code: &str) -> (&'static str, &'static str) {
    match code.to_lowercase().as_str() {
        "af" => ("Afghanistan", "🇦🇫"),
        "za" => ("South Africa", "🇿🇦"),
        "al" => ("Albania", "🇦🇱"),
        "dz" => ("Algeria", "🇩🇿"),
        "ad" => ("Andorra", "🇦🇩"),
        "ao" => ("Angola", "🇦🇴"),
        "ai" => ("Anguilla", "🇦🇮"),
        "aq" => ("Antarctica", "🇦🇶"),
        "ag" => ("Antigua and Barbuda", "🇦🇬"),
        "ar" => ("Argentina", "🇦🇷"),
        "am" => ("Armenia", "🇦🇲"),
        "aw" => ("Aruba", "🇦🇼"),
        "au" => ("Australia", "🇦🇺"),
        "az" => ("Azerbaijan", "🇦🇿"),
        "bs" => ("Bahamas", "🇧🇸"),
        "bh" => ("Bahrain", "🇧🇭"),
        "bd" => ("Bangladesh", "🇧🇩"),
        "bb" => ("Barbados", "🇧🇧"),
        "by" => ("Belarus", "🇧🇾"),
        "be" => ("Belgium", "🇧🇪"),
        "bz" => ("Belize", "🇧🇿"),
        "bj" => ("Benin", "🇧🇯"),
        "bm" => ("Bermuda", "🇧🇲"),
        "bt" => ("Bhutan", "🇧🇹"),
        "bo" => ("Bolivia", "🇧🇴"),
        "ba" => ("Bosnia and Herzegovina", "🇧🇦"),
        "bw" => ("Botswana", "🇧🇼"),
        "bv" => ("Bouvet Island", "🇧🇻"),
        "br" => ("Brazil", "🇧🇷"),
        "io" => ("British Indian Ocean Territory", "🇮🇴"),
        "bn" => ("Brunei", "🇧🇳"),
        "bg" => ("Bulgaria", "🇧🇬"),
        "bf" => ("Burkina Faso", "🇧🇫"),
        "bi" => ("Burundi", "🇧🇮"),
        "kh" => ("Cambodia", "🇰🇭"),
        "cm" => ("Cameroon", "🇨🇲"),
        "ca" => ("Canada", "🇨🇦"),
        "cv" => ("Cape Verde", "🇨🇻"),
        "ky" => ("Cayman Islands", "🇰🇾"),
        "cf" => ("Central African Republic", "🇨🇫"),
        "td" => ("Chad", "🇹🇩"),
        "cl" => ("Chile", "🇨🇱"),
        "cn" => ("China", "🇨🇳"),
        "cx" => ("Christmas Island", "🇨🇽"),
        "cc" => ("Cocos (Keeling) Islands", "🇨🇨"),
        "co" => ("Colombia", "🇨🇴"),
        "km" => ("Comoros", "🇰🇲"),
        "cg" => ("Republic of the Congo", "🇨🇬"),
        "cd" => ("Democratic Republic of the Congo", "🇨🇩"),
        "ck" => ("Cook Islands", "🇨🇰"),
        "cr" => ("Costa Rica", "🇨🇷"),
        "ci" => ("Ivory Coast", "🇨🇮"),
        "hr" => ("Croatia", "🇭🇷"),
        "cu" => ("Cuba", "🇨🇺"),
        "cy" => ("Cyprus", "🇨🇾"),
        "cz" => ("Czech Republic", "🇨🇿"),
        "dk" => ("Denmark", "🇩🇰"),
        "at" => ("Austria", "🇦🇹"),
        "de" => ("Germany", "🇩🇪"),
        "dj" => ("Djibouti", "🇩🇯"),
        "dm" => ("Dominica", "🇩🇲"),
        "do" => ("Dominican Republic", "🇩🇴"),
        "ec" => ("Ecuador", "🇪🇨"),
        "eg" => ("Egypt", "🇪🇬"),
        "sv" => ("El Salvador", "🇸🇻"),
        "gq" => ("Equatorial Guinea", "🇬🇶"),
        "er" => ("Eritrea", "🇪🇷"),
        "ee" => ("Estonia", "🇪🇪"),
        "et" => ("Ethiopia", "🇪🇹"),
        "fk" => ("Falkland Islands", "🇫🇰"),
        "fo" => ("Faroe Islands", "🇫🇴"),
        "fj" => ("Fiji", "🇫🇯"),
        "fi" => ("Finland", "🇫🇮"),
        "fr" => ("France", "🇫🇷"),
        "gf" => ("French Guiana", "🇬🇫"),
        "pf" => ("French Polynesia", "🇵🇫"),
        "tf" => ("French Southern Territories", "🇹🇫"),
        "ga" => ("Gabon", "🇬🇦"),
        "gm" => ("Gambia", "🇬🇲"),
        "ge" => ("Georgia", "🇬🇪"),
        "gh" => ("Ghana", "🇬🇭"),
        "gi" => ("Gibraltar", "🇬🇮"),
        "gr" => ("Greece", "🇬🇷"),
        "gl" => ("Greenland", "🇬🇱"),
        "gd" => ("Grenada", "🇬🇩"),
        "gp" => ("Guadeloupe", "🇬🇵"),
        "gu" => ("Guam", "🇬🇺"),
        "gt" => ("Guatemala", "🇬🇹"),
        "gn" => ("Guinea", "🇬🇳"),
        "gw" => ("Guinea-Bissau", "🇬🇼"),
        "gy" => ("Guyana", "🇬🇾"),
        "ht" => ("Haiti", "🇭🇹"),
        "hm" => ("Heard Island and McDonald Islands", "🇭🇲"),
        "hn" => ("Honduras", "🇭🇳"),
        "hk" => ("Hong Kong", "🇭🇰"),
        "hu" => ("Hungary", "🇭🇺"),
        "is" => ("Iceland", "🇮🇸"),
        "in" => ("India", "🇮🇳"),
        "id" => ("Indonesia", "🇮🇩"),
        "ir" => ("Iran", "🇮🇷"),
        "iq" => ("Iraq", "🇮🇶"),
        "ie" => ("Ireland", "🇮🇪"),
        "il" => ("Israel", "🇮🇱"),
        "it" => ("Italy", "🇮🇹"),
        "jm" => ("Jamaica", "🇯🇲"),
        "jp" => ("Japan", "🇯🇵"),
        "jo" => ("Jordan", "🇯🇴"),
        "kz" => ("Kazakhstan", "🇰🇿"),
        "ke" => ("Kenya", "🇰🇪"),
        "ki" => ("Kiribati", "🇰🇮"),
        "kp" => ("North Korea", "🇰🇵"),
        "kr" => ("South Korea", "🇰🇷"),
        "kw" => ("Kuwait", "🇰🇼"),
        "kg" => ("Kyrgyzstan", "🇰🇬"),
        "la" => ("Laos", "🇱🇦"),
        "lv" => ("Latvia", "🇱🇻"),
        "lb" => ("Lebanon", "🇱🇧"),
        "ls" => ("Lesotho", "🇱🇸"),
        "lr" => ("Liberia", "🇱🇷"),
        "ly" => ("Libya", "🇱🇾"),
        "li" => ("Liechtenstein", "🇱🇮"),
        "lt" => ("Lithuania", "🇱🇹"),
        "lu" => ("Luxembourg", "🇱🇺"),
        "mo" => ("Macau", "🇲🇴"),
        "mk" => ("North Macedonia", "🇲🇰"),
        "mg" => ("Madagascar", "🇲🇬"),
        "mw" => ("Malawi", "🇲🇼"),
        "my" => ("Malaysia", "🇲🇾"),
        "mv" => ("Maldives", "🇲🇻"),
        "ml" => ("Mali", "🇲🇱"),
        "mt" => ("Malta", "🇲🇹"),
        "mh" => ("Marshall Islands", "🇲🇭"),
        "mq" => ("Martinique", "🇲🇶"),
        "mr" => ("Mauritania", "🇲🇷"),
        "mu" => ("Mauritius", "🇲🇺"),
        "yt" => ("Mayotte", "🇾🇹"),
        "mx" => ("Mexico", "🇲🇽"),
        "fm" => ("Micronesia", "🇫🇲"),
        "md" => ("Moldova", "🇲🇩"),
        "mc" => ("Monaco", "🇲🇨"),
        "mn" => ("Mongolia", "🇲🇳"),
        "ms" => ("Montserrat", "🇲🇸"),
        "ma" => ("Morocco", "🇲🇦"),
        "mz" => ("Mozambique", "🇲🇿"),
        "mm" => ("Myanmar", "🇲🇲"),
        "na" => ("Namibia", "🇳🇦"),
        "nr" => ("Nauru", "🇳🇷"),
        "np" => ("Nepal", "🇳🇵"),
        "nl" => ("Netherlands", "🇳🇱"),
        "an" => ("Netherlands Antilles", "🇧🇶"),
        "nc" => ("New Caledonia", "🇳🇨"),
        "nz" => ("New Zealand", "🇳🇿"),
        "ni" => ("Nicaragua", "🇳🇮"),
        "ne" => ("Niger", "🇳🇪"),
        "ng" => ("Nigeria", "🇳🇬"),
        "nu" => ("Niue", "🇳🇺"),
        "nf" => ("Norfolk Island", "🇳🇫"),
        "mp" => ("Northern Mariana Islands", "🇲🇵"),
        "no" => ("Norway", "🇳🇴"),
        "om" => ("Oman", "🇴🇲"),
        "pk" => ("Pakistan", "🇵🇰"),
        "pw" => ("Palau", "🇵🇼"),
        "ps" => ("Palestine", "🇵🇸"),
        "pa" => ("Panama", "🇵🇦"),
        "pg" => ("Papua New Guinea", "🇵🇬"),
        "py" => ("Paraguay", "🇵🇾"),
        "pe" => ("Peru", "🇵🇪"),
        "ph" => ("Philippines", "🇵🇭"),
        "pn" => ("Pitcairn Islands", "🇵🇳"),
        "pl" => ("Poland", "🇵🇱"),
        "pt" => ("Portugal", "🇵🇹"),
        "pr" => ("Puerto Rico", "🇵🇷"),
        "qa" => ("Qatar", "🇶🇦"),
        "re" => ("Reunion", "🇷🇪"),
        "ro" => ("Romania", "🇷🇴"),
        "ru" => ("Russia", "🇷🇺"),
        "rw" => ("Rwanda", "🇷🇼"),
        "sh" => ("Saint Helena", "🇸🇭"),
        "kn" => ("Saint Kitts and Nevis", "🇰🇳"),
        "lc" => ("Saint Lucia", "🇱🇨"),
        "pm" => ("Saint Pierre and Miquelon", "🇵🇲"),
        "vc" => ("Saint Vincent and the Grenadines", "🇻🇨"),
        "ws" => ("Samoa", "🇼🇸"),
        "sm" => ("San Marino", "🇸🇲"),
        "st" => ("Sao Tome and Principe", "🇸🇹"),
        "sa" => ("Saudi Arabia", "🇸🇦"),
        "sn" => ("Senegal", "🇸🇳"),
        "cs" => ("Serbia and Montenegro", "🇷🇸"),
        "sc" => ("Seychelles", "🇸🇨"),
        "sl" => ("Sierra Leone", "🇸🇱"),
        "sg" => ("Singapore", "🇸🇬"),
        "sk" => ("Slovakia", "🇸🇰"),
        "si" => ("Slovenia", "🇸🇮"),
        "sb" => ("Solomon Islands", "🇸🇧"),
        "so" => ("Somalia", "🇸🇴"),
        "gs" => ("South Georgia and the South Sandwich Islands", "🇬🇸"),
        "es" => ("Spain", "🇪🇸"),
        "lk" => ("Sri Lanka", "🇱🇰"),
        "sd" => ("Sudan", "🇸🇩"),
        "sr" => ("Suriname", "🇸🇷"),
        "sj" => ("Svalbard and Jan Mayen", "🇸🇯"),
        "sz" => ("Eswatini", "🇸🇿"),
        "se" => ("Sweden", "🇸🇪"),
        "ch" => ("Switzerland", "🇨🇭"),
        "sy" => ("Syria", "🇸🇾"),
        "tw" => ("Taiwan", "🇹🇼"),
        "tj" => ("Tajikistan", "🇹🇯"),
        "tz" => ("Tanzania", "🇹🇿"),
        "th" => ("Thailand", "🇹🇭"),
        "tl" => ("Timor-Leste", "🇹🇱"),
        "tg" => ("Togo", "🇹🇬"),
        "tk" => ("Tokelau", "🇹🇰"),
        "to" => ("Tonga", "🇹🇴"),
        "tt" => ("Trinidad and Tobago", "🇹🇹"),
        "tn" => ("Tunisia", "🇹🇳"),
        "tr" => ("Turkey", "🇹🇷"),
        "tm" => ("Turkmenistan", "🇹🇲"),
        "tc" => ("Turks and Caicos Islands", "🇹🇨"),
        "tv" => ("Tuvalu", "🇹🇻"),
        "ug" => ("Uganda", "🇺🇬"),
        "ua" => ("Ukraine", "🇺🇦"),
        "ae" => ("United Arab Emirates", "🇦🇪"),
        "uk" => ("United Kingdom", "🇬🇧"),
        "us" => ("United States", "🇺🇸"),
        "um" => ("U.S. Minor Outlying Islands", "🇺🇲"),
        "uy" => ("Uruguay", "🇺🇾"),
        "uz" => ("Uzbekistan", "🇺🇿"),
        "vu" => ("Vanuatu", "🇻🇺"),
        "va" => ("Vatican City", "🇻🇦"),
        "ve" => ("Venezuela", "🇻🇪"),
        "vn" => ("Vietnam", "🇻🇳"),
        "vg" => ("British Virgin Islands", "🇻🇬"),
        "vi" => ("U.S. Virgin Islands", "🇻🇮"),
        "wf" => ("Wallis and Futuna", "🇼🇫"),
        "eh" => ("Western Sahara", "🇪🇭"),
        "ye" => ("Yemen", "🇾🇪"),
        "zm" => ("Zambia", "🇿🇲"),
        "zw" => ("Zimbabwe", "🇿🇼"),
        _ => ("Unknown", "🏳"),
    }
}

/*
  Blog
*/
#[derive(Serialize)]
pub struct BlogPost {
    pub title: String,
    pub desc: String,
    pub slug: String,
    pub date: String,
}

#[get("/blog")]
pub fn blog(cookie_jar: &CookieJar<'_>, host: &Host) -> Template {
    // Get posts
    let mut posts: Vec<BlogPost> = read_dir("templates/blog/post")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;

            let slug = file_name.strip_suffix(".html.hbs")?;
            if slug.is_empty() || !path.is_file() {
                return None;
            }

            let content = read_to_string(&path).ok()?;
            let (title, date, desc) = extract_metadata(&content);

            Some(BlogPost {
                title,
                slug: slug.to_string(),
                date,
                desc,
            })
        })
        .collect();

    // Sort posts
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // Render page
    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!("Blog | ")),
        (String::from("posts"), json!(posts)),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);
    Template::render("blog/index", context)
}

#[get("/blog/<slug>")]
pub fn blog_post(
    slug: &str,
    cookie_jar: &CookieJar<'_>,
    host: &Host,
) -> Result<Template, NotFound<String>> {
    let file_path = format!("templates/blog/post/{}.html.hbs", slug);

    let content =
        read_to_string(&file_path).map_err(|_| NotFound("Blog post not found".to_string()))?;

    let (title, date, desc) = extract_metadata(&content);

    let mut context: HashMap<String, RocketValue> = HashMap::from([
        (String::from("css_version"), json!(CSS_VERSION)),
        (String::from("js_version"), json!(JS_VERSION)),
        (String::from("title_query"), json!(format!("{} | ", title))), // Much better page title!
        (String::from("post_title"), json!(title)),
        (String::from("post_date"), json!(date)),
        (String::from("post_desc"), json!(desc)),
    ]);

    settings::run(&mut context, &None, cookie_jar, host);

    Ok(Template::render(format!("blog/post/{}", slug), context))
}

fn extract_metadata(content: &str) -> (String, String, String) {
    let mut title = String::from("Untitled");
    let mut desc = String::from("Empty description");
    let mut date = String::from("No date");

    // Load comment with metadata
    let mut in_metadata = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "{{!--" {
            in_metadata = true;
            continue;
        }
        if trimmed == "--}}" {
            break;
        }

        // Extract them
        if in_metadata {
            if let Some(rest) = trimmed.strip_prefix("title: ") {
                title = rest.to_string();
            } else if let Some(rest) = trimmed.strip_prefix("date: ") {
                date = rest.to_string();
            } else if let Some(rest) = trimmed.strip_prefix("desc: ") {
                desc = rest.to_string();
            }
        }
    }

    (title, date, desc)
}
