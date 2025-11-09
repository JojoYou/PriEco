use chrono::{Datelike, Duration as chDuration, Local, TimeZone, offset::LocalResult};
use dotenv_codegen::dotenv;
use rand::Rng;
use reqwest::Client;
use serde_json::Value;
use std::{
    fs::{File, metadata, remove_file, write},
    io::Write,
    time::Duration,
};

use crate::{
    call_api_future_json, get_domain,
    globals::{SearchResult, colors},
    read_file,
};

pub async fn run(query: &str, lang: &str, loc: &str) -> Option<Vec<SearchResult>> {
    ////
    // Cache
    ////
    for dir in &["google", "bing", "bing2", "brave", "brave2"] {
        let cache_file = format!(
            "cache/all/{}/{}_{}_{}.json",
            dir,
            query.replace(" ", "_").replace("/", "_"),
            loc,
            lang
        );
        if metadata(&cache_file).is_ok() {
            println!("Cache!");
            match *dir {
                "google" => {
                    if let Ok(json) = serde_json::from_str(&read_file(&cache_file)) {
                        return Some(format_google(json));
                    } else {
                        println!("All: Failed to parse JSON from cache: {}", cache_file);
                        return Some(Vec::new());
                    }
                }
                "bing" => {
                    if let Ok(json) = serde_json::from_str(&read_file(&cache_file)) {
                        return Some(format_bing(json));
                    } else {
                        println!("All: Failed to parse JSON from cache: {}", cache_file);
                        return Some(Vec::new());
                    }
                }
                "bing2" => {
                    if let Ok(json) = serde_json::from_str(&read_file(&cache_file)) {
                        return Some(format_bing(json));
                    } else {
                        println!("All: Failed to parse JSON from cache: {}", cache_file);
                        return Some(Vec::new());
                    }
                }
                "brave" => {
                    if let Ok(json) = serde_json::from_str(&read_file(&cache_file)) {
                        return Some(format_brave(json));
                    } else {
                        println!("All: Failed to parse JSON from cache: {}", cache_file);
                        return Some(Vec::new());
                    }
                }
                "brave2" => {
                    if let Ok(json) = serde_json::from_str(&read_file(&cache_file)) {
                        return Some(format_brave2(json));
                    } else {
                        println!("All: Failed to parse JSON from cache: {}", cache_file);
                        return Some(Vec::new());
                    }
                }
                _ => continue,
            }
        }
    }

    ////
    // API
    ////
    println!("Call!");

    // Parameters
    let random_boost: bool = rand::rng().random_bool(0.2); // 20% chance to use more powerful API
    let is_likely_english = query.chars().all(|c| c.is_ascii() || c.is_whitespace());
    let is_hard_query: bool = query.len() > 20 // Long
        || query.split_whitespace().count() > 3 // Many words
        || query.contains('"') // Quoted
        || query.chars().any(|c| "!@#$%^&*()[]{}<>".contains(c)); // Special characters

    // API selection priority
    // Check rate limit files
    let google_blocked = check_dis_remote("disGoogle.txt");
    let bing_blocked = check_dis_remote("disBing.txt");
    let bing2_blocked = check_dis_remote("disBing2.txt");
    let brave_blocked = check_dis_remote("disBrave.txt");
    let brave2_blocked = check_dis_remote("disBrave2.txt");

    if google_blocked && bing_blocked && bing2_blocked && brave_blocked && brave2_blocked {
        return Some(Vec::new()); // No APIs available, don't retry
    }

    let selected_api = if is_likely_english {
            if is_hard_query || random_boost {
                // Prefer powerful APIs for hard queries
                if !google_blocked {
                    Some("google")
                } else if !bing_blocked {
                    Some("bing")
                } else if !bing2_blocked {
                    Some("bing2")
                } else if !brave2_blocked {
                    Some("brave2")
                } else if !brave_blocked {
                    Some("brave")
                } else {
                    None
                }
            } else {
                // Prefer cheaper APIs for easy queries
                if !brave2_blocked {
                    Some("brave2")
                } else if !brave_blocked {
                    Some("brave")
                } else if !bing_blocked {
                    Some("bing")
                } else if !bing2_blocked {
                    Some("bing2")
                } else if !google_blocked {
                    Some("google")
                } else {
                    None
                }
            }
        } else {
            // Non-English: prefer Google
            if !google_blocked {
                Some("google")
            } else if !bing_blocked {
                Some("bing")
            } else if !bing2_blocked {
                Some("bing2")
            } else if !brave2_blocked {
                Some("brave2")
            } else if !brave_blocked {
                Some("brave")
            } else {
                None
            }
        };

        let selected_api = match selected_api {
            Some(api) => api,
            None => {
                println!("No available APIs (all blocked)");
                return Some(Vec::new());
            }
        };


    println!("Using API: {}", selected_api);

    let client = match Client::builder()
        .user_agent("PriEco/1.0.0 ( support@jojoyou.org )")
        .timeout(Duration::from_secs(2))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            println!(
                "{}Call ALL: Failed to build HTTP client: {}{}",
                colors::RED,
                colors::RESET,
                err
            );
            return None;
        }
    };

    match selected_api {
        "brave" => {
            let brave_option = call_api_future_json(  client
                .get(&format!(
                    "https://api.search.brave.com/res/v1/web/search?spellcheck=0&result_filter=web&country={}&search_lang={}&safesearch=moderate&q={}",
                    loc, lang, query
                ))
                .header("X-Subscription-Token", dotenv!("BRAVE_API_KEY"))).await;

            let brave_json = match brave_option {
                Some(json) => json,
                None => {
                    dis_remote(
                        "Brave",
                        (Local::now() + chDuration::hours(1)).timestamp() as u64,
                    );

                    return None; // Retry search
                }
            };

            // Check for errors
            if brave_json.get("error").is_some() {
                // Disable until 1st of next month
                let now = Local::now();
                let next_month = if now.month() == 12 {
                    1
                } else {
                    now.month() + 1
                };
                let year = now.year() + if now.month() == 12 { 1 } else { 0 };

                match Local.with_ymd_and_hms(year, next_month, 1, 0, 0, 0) {
                    LocalResult::Single(disable_dt) => {
                        dis_remote("Brave", disable_dt.timestamp() as u64);
                    }
                    LocalResult::Ambiguous(dt1, _dt2) => {
                        // pick the first in case of ambiguity
                        dis_remote("Brave", dt1.timestamp() as u64);
                    }
                    LocalResult::None => {
                        println!(
                            "All: Failed to compute first day of next month for Brave disable"
                        );
                    }
                }

                return None; // Send signal to retry search
            }

            let path = format!(
                "cache/all/brave/{}_{}_{}.json",
                query.replace(" ", "_").replace("/", "_"),
                loc,
                lang
            );
            match serde_json::to_string_pretty(&brave_json) {
                Ok(json_str) => {
                    if let Err(e) = write(&path, json_str) {
                        println!("All: Failed to write cache file {}: {}", path, e);
                    }
                }
                Err(e) => {
                    println!("All: Failed to serialize brave_json for caching: {}", e);
                }
            }

            return Some(format_brave(brave_json));
        }
        "brave2" => {
            let brave2_option = call_api_future_json(client.get(&format!(
                "{}/?&s=k&api={}&q={}",
                dotenv!("BB2_URL"),
                dotenv!("BB2_API_KEY"),
                query
            )))
            .await;
            let brave2_json = match brave2_option {
                Some(json) => json,
                None => {
                    dis_remote(
                        "Brave2",
                        (Local::now() + chDuration::hours(24)).timestamp() as u64,
                    );

                    return None; // Retry search
                }
            };

            let path = format!(
                "cache/all/brave2/{}_{}_{}.json",
                query.replace(" ", "_").replace("/", "_"),
                loc,
                lang
            );
            match serde_json::to_string_pretty(&brave2_json) {
                Ok(json_str) => {
                    if let Err(e) = write(&path, json_str) {
                        println!("All: Failed to write cache file {}: {}", path, e);
                    }
                }
                Err(e) => {
                    println!("All: Failed to serialize brave2_json for caching: {}", e);
                }
            }

            return Some(format_brave2(brave2_json));
        }
        "bing" => {
            let bing_option = call_api_future_json(client.get(&format!(
                "http://0.0.0.0:8040/?&s=b&api={}&q={}",
                dotenv!("BB2_API_KEY"),
                query
            )))
            .await;
            let bing_json = match bing_option {
                Some(json) => json,
                None => {
                    dis_remote(
                        "Bing",
                        (Local::now() + chDuration::hours(24)).timestamp() as u64,
                    );

                    return None; // Retry search
                }
            };

            let path = format!(
                "cache/all/bing/{}_{}_{}.json",
                query.replace(" ", "_").replace("/", "_"),
                loc,
                lang
            );
            match serde_json::to_string_pretty(&bing_json) {
                Ok(json_str) => {
                    if let Err(e) = write(&path, json_str) {
                        println!("All: Failed to write cache file {}: {}", path, e);
                    }
                }
                Err(e) => {
                    println!("All: Failed to serialize bing_json for caching: {}", e);
                }
            }

            return Some(format_bing(bing_json));
        }
        "bing2" => {
            let bing2_option = call_api_future_json(client.get(&format!(
                "{}/?&s=b&api={}&q={}",
                dotenv!("BB2_URL"),
                dotenv!("BB2_API_KEY"),
                query
            )))
            .await;
            let bing2_json = match bing2_option {
                Some(json) => json,
                None => {
                    dis_remote(
                        "Bing2",
                        (Local::now() + chDuration::hours(24)).timestamp() as u64,
                    );

                    return None; // Retry search
                }
            };

            let path = format!(
                "cache/all/bing2/{}_{}_{}.json",
                query.replace(" ", "_").replace("/", "_"),
                loc,
                lang
            );
            match serde_json::to_string_pretty(&bing2_json) {
                Ok(json_str) => {
                    if let Err(e) = write(&path, json_str) {
                        println!("All: Failed to write cache file {}: {}", path, e);
                    }
                }
                Err(e) => {
                    println!("All: Failed to serialize bing2_json for caching: {}", e);
                }
            }

            return Some(format_bing(bing2_json));
        }
        _ => {
            let google_option = call_api_future_json(client.get(&format!(
                "https://www.googleapis.com/customsearch/v1?key={}&cx={}&hl={}&q={}",
                dotenv!("GOOGLE_API_KEY"),
                dotenv!("GOOGLE_CX_KEY"),
                loc,
                query
            )))
            .await;
            let google_json = match google_option {
                Some(json) => json,
                None => {
                    dis_remote(
                        "Google",
                        (Local::now() + chDuration::hours(1)).timestamp() as u64,
                    );

                    return None; // Retry search
                }
            };

            // Check for errors
            if google_json.get("error").is_some() {
                // Disable until 8am (today/tomorrow)
                let now = Local::now();

                match Local.with_ymd_and_hms(now.year(), now.month(), now.day(), 8, 0, 0) {
                    LocalResult::Single(today_8am) => {
                        let disable_until = if now >= today_8am {
                            today_8am + chDuration::days(1)
                        } else {
                            today_8am
                        };
                        dis_remote("Google", disable_until.timestamp() as u64);
                    }
                    LocalResult::Ambiguous(dt1, _dt2) => {
                        // pick the first option if ambiguous
                        let disable_until = if now >= dt1 {
                            dt1 + chDuration::days(1)
                        } else {
                            dt1
                        };
                        dis_remote("Google", disable_until.timestamp() as u64);
                    }
                    LocalResult::None => {
                        println!("Failed to compute 8 AM for Google disable");
                    }
                }

                return None; // Send signal to retry search
            }

            let path = format!(
                "cache/all/google/{}_{}_{}.json",
                query.replace(" ", "_").replace("/", "_"),
                loc,
                lang
            );
            match serde_json::to_string_pretty(&google_json) {
                Ok(json_str) => {
                    if let Err(e) = write(&path, json_str) {
                        println!("All: Failed to write cache file {}: {}", path, e);
                    }
                }
                Err(e) => {
                    println!("All: Failed to serialize google_json for caching: {}", e);
                }
            }

            return Some(format_google(google_json));
        }
    }
}

fn format_google(json: Value) -> Vec<SearchResult> {
    let mut remote_results: Vec<SearchResult> = Vec::with_capacity(20);
    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
        for item in items {
            let url = item
                .get("link")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            remote_results.push(SearchResult {
                url: url.to_string(),
                display_url: url
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("www.", "")
                    .trim_end_matches('/')
                    .replace("/", " › "),
                domain: get_domain(url, true),
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("htmlSnippet")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: item
                    .get("pagemap")
                    .and_then(|pm| pm.get("cse_image"))
                    .and_then(|imgs| imgs.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|img| img.get("src"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        format!(
                            "<img loading='lazy' alt='‎' src='/proxy?u={}'>",
                            urlencoding::encode(s)
                        )
                    })
                    .unwrap_or_default(),
                favicon: format!(
                    "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                    urlencoding::encode(&get_domain(url, false))
                ),
            });
        }
    }
    remote_results
}
fn format_bing(json: Value) -> Vec<SearchResult> {
    let mut remote_results: Vec<SearchResult> = Vec::with_capacity(20);

    if let Some(mainline_items) = json
        .get("data")
        .and_then(|v| v.get("result"))
        .and_then(|r| r.get("items"))
        .and_then(|items| items.get("mainline"))
        .and_then(|ml| ml.as_array())
    {
        for block in mainline_items {
            if let Some(items) = block.get("items").and_then(|v| v.as_array()) {
                for item in items {
                    let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                    remote_results.push(SearchResult {
                        url: url.to_string(),
                        display_url: url
                            .replace("https://", "")
                            .replace("http://", "")
                            .replace("www.", "")
                            .trim_end_matches('/')
                            .replace("/", " › "),
                        domain: get_domain(url, true),
                        title: item
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        description: item
                            .get("desc")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        image: item
                            .get("thumbnailUrl")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| {
                                format!(
                                    "<img loading='lazy' alt='‎' src='/proxy?u={}'>",
                                    urlencoding::encode(s)
                                )
                            })
                            .unwrap_or_default(),
                        favicon: item
                            .get("favicon")
                            .and_then(|v| v.as_str())
                            .map(|s| format!("/proxy?u={}", urlencoding::encode(s)))
                            .unwrap_or_else(|| {
                                format!(
                                    "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                                    urlencoding::encode(&get_domain(url, false))
                                )
                            }),
                    });
                }
            }
        }
    }

    remote_results
}
fn format_brave(json: Value) -> Vec<SearchResult> {
    let mut remote_results: Vec<SearchResult> = Vec::with_capacity(20);

    if let Some(results) = json
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(|r| r.as_array())
    {
        for item in results {
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or_default();

            let favicon = item
                .get("meta_url")
                .and_then(|m| m.get("favicon"))
                .and_then(|v| v.as_str())
                .map(|s| format!("/proxy?u={}", s))
                .unwrap_or_else(|| {
                    format!(
                        "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                        get_domain(url, false)
                    )
                });

            let image = item
                .get("thumbnail")
                .and_then(|t| t.get("src"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| format!("<img loading='lazy' alt='‎' src='/proxy?u={}'>", s))
                .unwrap_or_default();

            remote_results.push(SearchResult {
                url: url.to_string(),
                display_url: url
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("www.", "")
                    .trim_end_matches('/')
                    .replace("/", " › "),
                domain: get_domain(url, true),
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image,
                favicon,
            });
        }
    }

    remote_results
}
fn format_brave2(json: Value) -> Vec<SearchResult> {
    let mut remote_results: Vec<SearchResult> = Vec::with_capacity(20);

    if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
        for item in results {
            let url = if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                url
            } else {
                // Skip if result isn't web result
                continue;
            };

            remote_results.push(SearchResult {
                url: url.to_string(),
                display_url: url
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("www.", "")
                    .trim_end_matches('/')
                    .replace("/", " › "),
                domain: get_domain(url, true),
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: String::new(),
                favicon: item
                    .get("meta_url")
                    .and_then(|m| m.get("favicon"))
                    .and_then(|v| v.as_str())
                    .map(|s| format!("/proxy?u={}", s))
                    .unwrap_or_else(|| {
                        format!(
                            "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                            get_domain(url, false)
                        )
                    }),
            });
        }
    }

    remote_results
}

////
// Helper functions
////
fn dis_remote(remote_name: &str, time: u64) {
    let filename = format!("dis{}.txt", remote_name);
    println!("Creating dis file: {} with timestamp: {}", filename, time);

    match File::create(&filename) {
        Ok(mut file) => {
            if let Err(e) = write!(file, "{}", time) {
                println!(
                    "{}Failed to write to {}:{} {}",
                    colors::RED,
                    filename,
                    colors::RESET,
                    e
                );
            } else if let Err(e) = file.sync_all() {
                println!(
                    "{}Failed to sync {}:{} {}",
                    colors::RED,
                    filename,
                    colors::RESET,
                    e
                );
            } else {
                println!("Successfully created {} - {} blocked until timestamp {}",
                         filename, remote_name, time);
            }
        }
        Err(e) => {
            println!(
                "{}Failed to create {}:{} {}",
                colors::RED,
                filename,
                colors::RESET,
                e
            );
        }
    }
}

fn check_dis_remote(filename: &str) -> bool {

    // Check if the file exists
    if metadata(&filename).is_err() {
        return false;
    }

    if let Ok(block_time) = read_file(&filename).trim().parse::<u64>() {
        let now = Local::now().timestamp() as u64;
        if block_time <= now {
            // time expired, remove the file
            let _ = remove_file(&filename);
            return false; // not blocked anymore
        } else {
            return true; // still blocked
        }
    }

    false // file missing or unreadable → treat as not blocked
}
