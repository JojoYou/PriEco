use ahash::AHashMap;
use chrono::Local;
use dashmap::DashMap;
use once_cell::sync::Lazy;

use scraper::{Html, Selector};
use std::collections::HashSet;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use url::Url;

use regex::Regex;
use whatlang::{Lang, detect_lang};

use crate::mini_crawler::{HTMLResultData, PARSER_SELECTORS, is_valid_url};

pub static DOMAIN_COUNTRY_CACHE: Lazy<Arc<DashMap<String, String>>> =
    Lazy::new(|| Arc::new(DashMap::with_capacity(500_000)));

pub static GEOIP_DB: &[u8] = include_bytes!("data/GeoLite2-Country.mmdb");
pub static GEOIP_READER: Lazy<maxminddb::Reader<&'static [u8]>> =
    Lazy::new(|| maxminddb::Reader::from_source(GEOIP_DB).expect("Failed to load GeoIP DB"));

pub fn extract_metadata(
    document: &Html,
    final_url: &str,
    loading_time: f64,
    tag_data: &AHashMap<String, Vec<String>>,
) -> HTMLResultData {
    let selectors = &**PARSER_SELECTORS;
    let mut result = HTMLResultData {
        url: final_url.to_string(),
        title: String::new(),
        description: String::new(),
        text: String::new(),
        favicon: String::new(),
        img: String::new(),
        keywords: String::new(),
        safe_search: false,
        html: String::new(),
        language: String::new(),
        location: String::new(),
        points: Vec::new(),
        loading_time,
        date_of_crawling: Local::now().format("%y%m%d").to_string(),
        vector: String::new(),
        tag_data: tag_data.clone(),
    };

    //  Title & Description
    result.title = extract_title(
        document,
        &selectors.og_site_name_selector,
        &selectors.title_selector,
        &selectors.h_selectors,
    );

    result.description = extract_description(
        document,
        &selectors.og_description_selector,
        &selectors.meta_selector,
        &selectors.p_selector,
        &result.title,
    );

    // Truncate length
    if result.title.len() > 60 {
        let end = result
            .title
            .char_indices()
            .nth(57)
            .map(|(i, _)| i)
            .unwrap_or(result.title.len());
        result.title = format!("{}...", &result.title[..end]);
    }
    if result.description.len() > 160 {
        let end = result
            .description
            .char_indices()
            .nth(157)
            .map(|(i, _)| i)
            .unwrap_or(result.description.len());
        result.description = format!("{}...", &result.description[..end]);
    }

    //  Text Extraction
    let mut text = String::with_capacity(2048);
    let mut char_count = 0;
    let limit = 500;
    for selector in selectors.content_selectors.iter() {
        for element in document.select(selector) {
            let element_text = element.text().collect::<Vec<_>>().join(" ");
            for ch in element_text.chars() {
                text.push(ch);
                char_count += 1;
                if char_count >= limit {
                    break;
                }
            }
            text.push(' ');
            char_count += 1;
            if char_count >= limit {
                break;
            }
        }
        if char_count >= limit {
            break;
        }
    }
    let regex = Regex::new(r"\s+").unwrap();
    result.text = regex.replace_all(&text, " ").trim().to_string();

    //  Keywords
    result.keywords = document
        .select(&selectors.meta_keywords_selector)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(String::from)
        .unwrap_or_default();

    // Favicon URL
    result.favicon = document
        .select(&selectors.link_icon_selector)
        .next()
        .or_else(|| document.select(&selectors.link_shortcut_selector).next())
        .or_else(|| document.select(&selectors.link_apple_selector).next())
        .and_then(|e| e.value().attr("href"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    //  First Image
    result.img = result
        .tag_data
        .get("img_src")
        .and_then(|imgs| {
            imgs.iter().filter(|img| is_valid_url(img)).find_map(|img| {
                if img.starts_with("http") {
                    Some(img.to_string())
                } else if img.starts_with('/') {
                    let domain = get_domain(&result.url, false);
                    if !domain.is_empty() {
                        let protocol = if result.url.starts_with("https") {
                            "https"
                        } else {
                            "http"
                        };
                        Some(format!("{}://{}{}", protocol, domain, img))
                    } else {
                        None
                    }
                } else {
                    let base_url = result.url.trim_end_matches('/');
                    Some(format!("{}/{}", base_url, img))
                }
            })
        })
        .unwrap_or_default();

    // Language Detection
    let mut combined_text = String::with_capacity(200);
    'outer: for tag in [
        "h1", "h2", "h3", "h4", "h5", "h6", "p", "span", "a", "li", "label",
    ] {
        if let Some(texts) = result.tag_data.get(tag) {
            for text in texts {
                if !text.is_empty() {
                    if combined_text.len() + text.len() > 199 {
                        break 'outer;
                    }
                    combined_text.push_str(text);
                    combined_text.push(' ');
                }
            }
        }
    }
    result.language = get_language_code(detect_lang(&combined_text));

    result.location = get_website_country(&result.url);

    //  NSFW Check
    result.safe_search = nsfw_check(&result);

    result
}

/*
  Description: Extract title text from html

  Input: document
  Output: title as String
*/
fn extract_title(
    document: &Html,
    og_selector: &Selector,
    title_selector: &Selector,
    h_selectors: &Vec<Selector>,
) -> String {
    // Site specified title
    if let Some(og_site_name_element) = document.select(og_selector).next() {
        if let Some(content) = og_site_name_element.value().attr("content") {
            return content.to_string();
        }
    }

    // <title> tag
    if let Some(title_element) = document.select(title_selector).next() {
        return title_element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
    }

    // <h1..6> tags
    for i in 1..6 {
        if let Some(h_selector) = h_selectors.get(i) {
            if let Some(h_element) = document.select(h_selector).next() {
                return h_element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
            }
        }
    }

    String::new()
}

/*
  Description: Extract description text from html

  Input: document, title
  Output: description as String
*/
fn extract_description(
    document: &Html,
    og_selector: &Selector,
    meta_selector: &Selector,
    p_selector: &Selector,
    title: &str,
) -> String {
    // Site specified description
    if let Some(og_description_element) = document.select(og_selector).next() {
        if let Some(content) = og_description_element.value().attr("content") {
            if content != title {
                return content.to_string();
            }
        }
    }

    if let Some(description_element) = document.select(meta_selector).next() {
        if let Some(content) = description_element.value().attr("content") {
            if content != title {
                return content.to_string();
            }
        }
    }

    // Get description from <p> tag
    if let Some(p_element) = document.select(&p_selector).next() {
        let content = p_element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if content != title {
            return content;
        }
    }

    String::new()
}

/*
  Description: Get country from URL
  Input: URL
  Output: Country name as String
*/
fn get_website_country(url: &str) -> String {
    // Get domain from URL
    let domain: String = get_domain(url, true);
    if domain.is_empty() {
        return String::new();
    }

    // Cache lookup
    if let Some(country) = DOMAIN_COUNTRY_CACHE.get(&domain) {
        return country.clone();
    }

    // If not found in cache get the country
    // Domain to IP address
    let mut ip = None;
    for port in [443, 80] {
        if let Ok(addrs) = format!("{}:{}", domain, port).to_socket_addrs() {
            if let Some(addr) = addrs.into_iter().next() {
                ip = Some(addr.ip());
                break;
            }
        }
    }
    let ip = match ip {
        Some(ip) => ip,
        None => {
            return String::new();
        }
    };

    // GeoLite2 database
    let record: maxminddb::geoip2::Country = match GEOIP_READER.lookup(ip) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };

    // Extract country name
    let country_code = match record
        .country
        .and_then(|c| c.iso_code)
        .map(|s| s.to_lowercase())
    {
        Some(country_code) => country_code,
        None => String::new(),
    };
    if country_code.is_empty() {
        return String::new();
    }

    // Save country code to cache
    if !country_code.is_empty() {
        if DOMAIN_COUNTRY_CACHE.len() > 1_000_000 {
            DOMAIN_COUNTRY_CACHE.clear();
            println!("🧹 Cleared Domain Country Cache");
        }

        DOMAIN_COUNTRY_CACHE.insert(domain, country_code.clone());
    }

    return country_code;
}

fn get_language_code(lang_info: Option<Lang>) -> String {
    match lang_info {
        Some(info) => match info.code() {
            "ara" => "ar".to_string(),
            "bul" => "bg".to_string(),
            "cat" => "ca".to_string(),
            "ces" => "cs".to_string(),
            "dan" => "da".to_string(),
            "deu" => "de".to_string(),
            "ell" => "el".to_string(),
            "eng" => "en".to_string(),
            "spa" => "es".to_string(),
            "est" => "et".to_string(),
            "fin" => "fi".to_string(),
            "fra" => "fr".to_string(),
            "hrv" => "hr".to_string(),
            "hun" => "hu".to_string(),
            "ind" => "id".to_string(),
            "isl" => "is".to_string(),
            "ita" => "it".to_string(),
            "heb" => "iw".to_string(),
            "jpn" => "ja".to_string(),
            "kor" => "ko".to_string(),
            "lit" => "lt".to_string(),
            "lav" => "lv".to_string(),
            "nld" => "nl".to_string(),
            "nor" => "no".to_string(),
            "pol" => "pl".to_string(),
            "por" => "pt".to_string(),
            "ron" => "ro".to_string(),
            "rus" => "ru".to_string(),
            "slk" => "sk".to_string(),
            "slv" => "sl".to_string(),
            "srp" => "sr".to_string(),
            "swe" => "sv".to_string(),
            "tur" => "tr".to_string(),
            _ => String::new(),
        },
        None => String::new(),
    }
}

/*
  Description: Check if web page's content is NSFW

  Input: result
  Output: answer (True if web page is NSFW, False otherwise)
*/
fn nsfw_check(result: &HTMLResultData) -> bool {
    // List of bad words for URL check
    let bad_words_url = vec![
        "porn",
        "nude",
        "naked",
        "fuck",
        "dick",
        "ass",
        "asshole",
        "cunt",
        "cock",
        "pussy",
        "cocks",
        "cocksucker",
        "dicks",
        "dicksucker",
        "cocksucking",
        "cunts",
        "cocksuck",
        "dicksuck",
        "cocksucked",
        "dicksucked",
    ];
    // Check if URL contains any of the bad words
    if bad_words_url.iter().any(|ext| result.url.contains(ext)) {
        return true;
    }

    // List of science terms for possibility the page is scientific
    let science_terms = vec![
        "anatomy",
        "reproductive",
        "gynecology",
        "urology",
        "medical",
        "clinical",
        "biology",
        "chemistry",
        "physics",
        "science",
        "scientific",
        "hypothesis",
        "theory",
        "experiment",
        "laboratory",
        "research",
        "molecule",
        "atom",
        "element",
        "compound",
        "reaction",
        "cell",
        "organism",
        "species",
        "evolution",
        "genetics",
        "quantum",
        "thermal",
        "nuclear",
        "organic",
        "inorganic",
        "biochemistry",
        "neuroscience",
        "biotechnology",
        "ecology",
        "astronomy",
        "geology",
        "meteorology",
        "oceanography",
        "mathematics",
        "algorithm",
        "computation",
        "data",
        "engineering",
        "technology",
        "innovation",
        "development",
        "physician",
        "doctor",
        "hospital",
        "examination",
        "diagnosis",
        "treatment",
        "patient",
        "health",
        "consultation",
        "symptoms",
        "condition",
        "procedure",
        "surgery",
        "medicine",
        "healthcare",
        "clinic",
        "specialist",
        "medical condition",
        "anatomical",
        "biological",
        "physiology",
        "medical exam",
        "textbook",
        "study",
        "education",
        "academic",
        "journal",
        "paper",
        "article",
        "curriculum",
        "educational",
        "learning",
        "university",
        "college",
        "school",
        "lecture",
        "course",
        "class",
        "teaching",
        "phd",
        "thesis",
        "dissertation",
        "professor",
        "student",
    ];
    // Count number of unique scientific words in the content
    let containing_science_words: HashSet<&str> = result
        .tag_data
        .values()
        .flat_map(|values| values.iter())
        .flat_map(|value| value.split_whitespace())
        .filter(|&word| science_terms.contains(&word.to_lowercase().as_str()))
        .collect();
    // If more than 5 scientific words are found, the page is not NSFW but scientific
    if containing_science_words.len() >= 5 {
        return false;
    }

    // List of bad words for page content check
    let bad_word = vec![
        "sex",
        "fuck",
        "dick",
        "ass",
        "asshole",
        "cunt",
        "cock",
        "pussy",
        "cocks",
        "cocksucker",
        "dicks",
        "dicksucker",
        "cocksucking",
        "cunts",
        "cocksuck",
        "dicksuck",
        "cocksucked",
        "dicksucked",
        "adult",
        "erotica",
        "pornography",
        "fetish",
        "kink",
        "BDSM",
        "foreplay",
        "intimacy",
        "arousal",
        "orgasm",
        "libido",
        "seduction",
        "fantasy",
        "swinging",
        "voyeurism",
        "exhibitionism",
        "roleplay",
        "consent",
        "safe sex",
        "sexual health",
        "aphrodisiac",
        "carnal",
        "sensual",
        "naughty",
        "intimate",
        "passion",
        "desire",
        "seductive",
        "tease",
        "hookup",
        "one-night stand",
        "polyamory",
        "threesome",
        "fetishism",
        "dominance",
        "submission",
        "aftercare",
        "dirty talk",
        "sex toys",
        "adult films",
        "striptease",
        "lap dance",
        "porn star",
        "sexuality",
        "masturbation",
        "climax",
        "petting",
        "making love",
        "pornographic",
        "cuckold",
        "swinger",
        "chemistry",
        "flirting",
        "kinky",
        "fetish party",
        "bondage",
        "sadism",
        "masochism",
    ];
    // Count number of unique bad words in the content
    let containing_bad_words: HashSet<&str> = result
        .tag_data
        .values()
        .flat_map(|values| values.iter())
        .flat_map(|value| value.split_whitespace())
        .filter(|&word| bad_word.contains(&word.to_lowercase().as_str()))
        .collect();
    // If more than 3 bad words are found, the page is NSFW
    if containing_bad_words.len() >= 3 {
        return true;
    }

    // Otherwise, the page is not NSFW
    false
}

/*
  Description: Gets domain from URL

  Input: URL, if remove_www
  Output: domain
*/
pub fn get_domain(url: &str, remove_www: bool) -> String {
    if url.is_empty() {
        return String::new();
    }
    let parsed_url = Url::parse(url);
    match parsed_url {
        Ok(parsed) => {
            if let Some(domain) = parsed.domain() {
                if remove_www && domain.starts_with("www.") {
                    return domain[4..].to_string();
                }
                return domain.to_string();
            }
            println!("URL has no domain part: {}", url);
        }
        Err(err) => {
            println!("Failed to parse URL {}: {}", url, err);
        }
    }
    String::new()
}
