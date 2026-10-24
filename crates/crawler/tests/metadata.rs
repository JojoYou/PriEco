//! Tests for crates/crawler/src/meta.rs (extract_metadata and its helpers)
//!
//! URLs use IP hosts (no domain -> no DNS lookup) or domains pre-seeded in
//! DOMAIN_COUNTRY_CACHE, so these tests never touch the network.

use prieco_mini_crawler::{
    meta::{DOMAIN_COUNTRY_CACHE, GEOIP_DB, GEOIP_READER, extract_metadata},
    mini_crawler::{HTMLResultData, WorkerBatchResponse},
    tag_data::extract_tag_data,
};
use scraper::Html;

const URL: &str = "http://127.0.0.1/page";

fn meta_at(html: &str, url: &str) -> HTMLResultData {
    let doc = Html::parse_document(html);
    let tags = extract_tag_data(&doc);
    extract_metadata(&doc, url, 0.42, &tags)
}

fn meta(html: &str) -> HTMLResultData {
    meta_at(html, URL)
}

fn page(head: &str, body: &str) -> String {
    format!("<html><head>{}</head><body>{}</body></html>", head, body)
}

/* ───────────────────────────── pass-through fields ───────────────────────────── */

#[test]
fn url_and_loading_time_are_passed_through() {
    let m = meta(&page("<title>T</title>", ""));
    assert_eq!(m.url, URL);
    assert_eq!(m.loading_time, 0.42);
}

#[test]
fn unused_fields_are_empty() {
    let m = meta(&page("<title>T</title>", ""));
    assert!(m.html.is_empty());
    assert!(m.vector.is_empty());
    assert!(m.points.is_empty());
}

#[test]
fn date_of_crawling_is_today_yymmdd() {
    let m = meta(&page("<title>T</title>", ""));
    assert_eq!(m.date_of_crawling.len(), 6);
    assert!(m.date_of_crawling.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(m.date_of_crawling, chrono::Local::now().format("%y%m%d").to_string());
}

#[test]
fn tag_data_is_copied_into_result() {
    let m = meta(&page("", "<h1>Head</h1><p>Para</p>"));
    assert_eq!(m.tag_data.get("h1").unwrap(), &vec!["Head".to_string()]);
    assert_eq!(m.tag_data.get("p").unwrap(), &vec!["Para".to_string()]);
}

/* ───────────────────────────── title ───────────────────────────── */

#[test]
fn title_prefers_og_site_name() {
    let m = meta(&page(
        "<meta property='og:site_name' content='Site Name'><title>Page Title</title>",
        "",
    ));
    assert_eq!(m.title, "Site Name");
}

#[test]
fn title_og_site_name_without_content_falls_back() {
    let m = meta(&page("<meta property='og:site_name'><title>Page Title</title>", ""));
    assert_eq!(m.title, "Page Title");
}

#[test]
fn title_from_title_tag_is_trimmed() {
    let m = meta(&page("<title>  Hello World  </title>", ""));
    assert_eq!(m.title, "Hello World");
}

#[test]
fn title_falls_back_to_h2() {
    let m = meta(&page("", "<h2>Second</h2><h3>Third</h3>"));
    assert_eq!(m.title, "Second");
}

#[test]
fn title_falls_back_to_lower_headings_in_order() {
    let m = meta(&page("", "<h6>Six</h6><h4>Four</h4>"));
    assert_eq!(m.title, "Four");
}

/// BUG: the heading fallback loops `for i in 1..6` over `h_selectors`, which are
/// h1..h6 at indices 0..5, so <h1> (index 0) is skipped entirely.
#[test]
fn title_falls_back_to_h1() {
    let m = meta(&page("", "<h1>Main heading</h1>"));
    assert_eq!(m.title, "Main heading");
}

#[test]
fn title_empty_when_nothing_available() {
    let m = meta(&page("", "<p>text</p>"));
    assert_eq!(m.title, "");
}

#[test]
fn title_of_exactly_60_chars_is_kept() {
    let t = "a".repeat(60);
    let m = meta(&page(&format!("<title>{}</title>", t), ""));
    assert_eq!(m.title, t);
}

#[test]
fn title_longer_than_60_is_truncated_to_57_plus_ellipsis() {
    let t = "b".repeat(61);
    let m = meta(&page(&format!("<title>{}</title>", t), ""));
    assert_eq!(m.title, format!("{}...", "b".repeat(57)));
    assert_eq!(m.title.chars().count(), 60);
}

#[test]
fn title_truncation_is_utf8_safe() {
    let t = "é".repeat(80);
    let m = meta(&page(&format!("<title>{}</title>", t), ""));
    assert_eq!(m.title, format!("{}...", "é".repeat(57)));
}

/// BUG: the length check uses bytes (`len() > 60`) but the cut uses chars (`nth(57)`).
/// A 31 char title of 2-byte letters (62 bytes) is not cut at all, but still gets "...".
#[test]
fn short_multibyte_title_gets_no_ellipsis() {
    let t = "é".repeat(31);
    let m = meta(&page(&format!("<title>{}</title>", t), ""));
    assert_eq!(m.title, t);
}

/* ───────────────────────────── description ───────────────────────────── */

#[test]
fn description_prefers_og_description() {
    let m = meta(&page(
        "<meta property='og:description' content='OG desc'><meta name='description' content='Meta desc'>",
        "<p>Para</p>",
    ));
    assert_eq!(m.description, "OG desc");
}

#[test]
fn description_from_meta_description() {
    let m = meta(&page("<meta name='description' content='Meta desc'>", "<p>Para</p>"));
    assert_eq!(m.description, "Meta desc");
}

#[test]
fn description_falls_back_to_first_paragraph() {
    let m = meta(&page("<title>T</title>", "<p>  First para  </p><p>Second</p>"));
    assert_eq!(m.description, "First para");
}

#[test]
fn description_equal_to_title_is_skipped() {
    let m = meta(&page(
        "<title>Same</title><meta property='og:description' content='Same'>",
        "<p>Other</p>",
    ));
    assert_eq!(m.description, "Other");
}

#[test]
fn description_empty_when_only_title_like_paragraph() {
    let m = meta(&page("<title>Same</title>", "<p>Same</p>"));
    assert_eq!(m.description, "");
}

#[test]
fn description_longer_than_160_is_truncated() {
    let d = "d".repeat(200);
    let m = meta(&page(&format!("<meta name='description' content='{}'>", d), ""));
    assert_eq!(m.description, format!("{}...", "d".repeat(157)));
}

#[test]
fn description_of_exactly_160_is_kept() {
    let d = "d".repeat(160);
    let m = meta(&page(&format!("<meta name='description' content='{}'>", d), ""));
    assert_eq!(m.description, d);
}

/// BUG: the fallback uses `meta_selector` ("meta") instead of
/// `meta_description_selector`, so the *first* meta tag with any content wins:
/// on almost every real page that is `<meta name="viewport" ...>`.
#[test]
fn description_ignores_viewport_meta() {
    let m = meta(&page(
        "<meta name='viewport' content='width=device-width'><meta name='description' content='Real'>",
        "",
    ));
    assert_eq!(m.description, "Real");
}

/// BUG: same root cause: a leading `<meta charset>` (no content) stops the meta
/// lookup and the real description is never read.
#[test]
fn description_found_after_charset_meta() {
    let m = meta(&page(
        "<meta charset='utf-8'><meta name='description' content='Real'>",
        "<p>Fallback</p>",
    ));
    assert_eq!(m.description, "Real");
}

/* ───────────────────────────── content text ───────────────────────────── */

#[test]
fn text_comes_from_headings_and_paragraphs() {
    let m = meta(&page("", "<h2>Head</h2><p>Body text</p><span>not content</span>"));
    assert!(m.text.contains("Head"));
    assert!(m.text.contains("Body text"));
    assert!(!m.text.contains("not content"));
}

#[test]
fn text_excludes_h1() {
    let m = meta(&page("", "<h1>Only h1</h1>"));
    assert_eq!(m.text, "");
}

#[test]
fn text_whitespace_is_collapsed_and_trimmed() {
    let m = meta(&page("", "<p>  a \n\n b\t\tc  </p>"));
    assert_eq!(m.text, "a b c");
}

#[test]
fn text_is_limited_to_about_500_chars() {
    let long = "word ".repeat(1000);
    let m = meta(&page("", &format!("<p>{}</p><p>{}</p>", long, long)));
    assert!(m.text.chars().count() <= 501, "{}", m.text.chars().count());
    assert!(m.text.chars().count() >= 400);
}

#[test]
fn text_limit_is_char_based_for_multibyte() {
    let long = "ž".repeat(2000);
    let m = meta(&page("", &format!("<p>{}</p>", long)));
    assert!(m.text.chars().count() <= 501);
}

/* ───────────────────────────── keywords / favicon / image ───────────────────────────── */

#[test]
fn keywords_from_meta_keywords() {
    let m = meta(&page("<meta name='keywords' content='rust, search, privacy'>", ""));
    assert_eq!(m.keywords, "rust, search, privacy");
}

#[test]
fn keywords_empty_when_missing() {
    assert_eq!(meta(&page("", "")).keywords, "");
}

#[test]
fn favicon_from_link_icon_trimmed() {
    let m = meta(&page("<link rel='icon' href='  /favicon.ico  '>", ""));
    assert_eq!(m.favicon, "/favicon.ico");
}

#[test]
fn favicon_priority_icon_then_shortcut_then_apple() {
    let all = "<link rel='apple-touch-icon' href='/apple.png'>\
               <link rel='shortcut icon' href='/short.ico'>\
               <link rel='icon' href='/icon.ico'>";
    assert_eq!(meta(&page(all, "")).favicon, "/icon.ico");
    let two = "<link rel='apple-touch-icon' href='/apple.png'><link rel='shortcut icon' href='/short.ico'>";
    assert_eq!(meta(&page(two, "")).favicon, "/short.ico");
    let one = "<link rel='apple-touch-icon' href='/apple.png'>";
    assert_eq!(meta(&page(one, "")).favicon, "/apple.png");
}

#[test]
fn favicon_empty_when_missing_or_without_href() {
    assert_eq!(meta(&page("", "")).favicon, "");
    assert_eq!(meta(&page("<link rel='icon'>", "")).favicon, "");
}

#[test]
fn image_is_first_absolute_image() {
    let m = meta(&page(
        "",
        "<img src='/relative.png'><img src='https://cdn.example.com/first.png'><img src='https://cdn.example.com/second.png'>",
    ));
    assert!(m.img.starts_with("https://cdn.example.com/first.png"), "{}", m.img);
}

#[test]
fn image_empty_when_no_absolute_images() {
    let m = meta(&page("", "<img src='/a.png'><img src='b.png'>"));
    assert_eq!(m.img, "");
}

/* ───────────────────────────── language ───────────────────────────── */

#[test]
fn english_page_is_detected() {
    let m = meta(&page(
        "",
        "<p>This is a long English sentence about search engines and privacy on the internet.</p>",
    ));
    assert_eq!(m.language, "en");
}

#[test]
fn german_page_is_detected() {
    let m = meta(&page(
        "",
        "<p>Dies ist ein langer deutscher Satz über Suchmaschinen und den Datenschutz im Internet.</p>",
    ));
    assert_eq!(m.language, "de");
}

#[test]
fn page_without_text_has_no_language() {
    assert_eq!(meta(&page("", "")).language, "");
}

#[test]
fn unsupported_language_maps_to_empty() {
    // Vietnamese is detected by whatlang but not mapped by PriEco
    let m = meta(&page(
        "",
        "<p>Đây là một câu tiếng Việt khá dài về công cụ tìm kiếm và quyền riêng tư trên mạng.</p>",
    ));
    assert_eq!(m.language, "");
}

/// BUG: Hebrew is mapped to the legacy code "iw", while the rest of PriEco
/// (COUNTRY_TO_LANG, synonyms, intents) uses "he".
#[test]
fn hebrew_page_uses_he_code() {
    let m = meta(&page(
        "",
        "<p>זהו משפט ארוך בעברית על מנועי חיפוש ועל פרטיות באינטרנט ועל הרבה דברים אחרים.</p>",
    ));
    assert_eq!(m.language, "he");
}

/* ───────────────────────────── location ───────────────────────────── */

#[test]
fn ip_hosts_have_no_location() {
    assert_eq!(meta(&page("<title>T</title>", "")).location, "");
}

#[test]
fn location_uses_domain_cache_without_www() {
    DOMAIN_COUNTRY_CACHE.insert("prieco-cache-test.example".to_string(), "sk".to_string());
    let m = meta_at(
        &page("<title>T</title>", ""),
        "https://www.prieco-cache-test.example/x",
    );
    assert_eq!(m.location, "sk");
}

#[test]
fn geoip_database_is_embedded_and_parses() {
    assert!(!GEOIP_DB.is_empty());
    let r = &*GEOIP_READER;
    let ip: std::net::IpAddr = "8.8.8.8".parse().unwrap();
    let c: maxminddb::geoip2::Country = r.lookup(ip).unwrap();
    assert_eq!(c.country.and_then(|c| c.iso_code), Some("US"));
}

/* ───────────────────────────── NSFW ───────────────────────────── */

#[test]
fn clean_page_is_safe() {
    let m = meta(&page("<title>Cooking</title>", "<p>How to bake bread at home</p>"));
    assert!(!m.safe_search);
}

#[test]
fn bad_word_in_url_flags_page() {
    let m = meta_at(&page("<title>T</title>", ""), "http://127.0.0.1/porn/");
    assert!(m.safe_search);
}

#[test]
fn three_bad_words_in_content_flag_page() {
    let m = meta(&page("", "<p>sex fetish bondage</p>"));
    assert!(m.safe_search);
}

#[test]
fn two_bad_words_in_content_are_not_enough() {
    let m = meta(&page("", "<p>sex fetish</p>"));
    assert!(!m.safe_search);
}

#[test]
fn repeated_bad_word_counts_once() {
    let m = meta(&page("", "<p>sex sex sex sex</p>"));
    assert!(!m.safe_search);
}

#[test]
fn bad_words_are_case_insensitive() {
    let m = meta(&page("", "<p>SEX Fetish BONDAGE</p>"));
    assert!(m.safe_search);
}

#[test]
fn scientific_context_overrides_bad_words() {
    let m = meta(&page(
        "",
        "<p>sex fetish bondage anatomy biology medical research university</p>",
    ));
    assert!(!m.safe_search);
}

/// BUG: the URL check is a plain substring test, so innocent URLs containing
/// "ass" (class, pass, assets, glasses, massachusetts...) are marked NSFW.
#[test]
fn innocent_urls_are_not_flagged() {
    for url in [
        "http://127.0.0.1/classes/",
        "http://127.0.0.1/assets/main",
        "http://127.0.0.1/passport",
        "http://127.0.0.1/massachusetts",
    ] {
        let m = meta_at(&page("<title>T</title>", ""), url);
        assert!(!m.safe_search, "{} flagged as NSFW", url);
    }
}

/// BUG: "BDSM" is stored uppercase in the bad word list but compared against
/// lowercased words, so it can never match.
#[test]
fn uppercase_entries_in_bad_word_list_can_match() {
    // sadism + masochism = 2 bad words, "bdsm" should be the 3rd
    let m = meta(&page("", "<p>bdsm sadism masochism</p>"));
    assert!(m.safe_search, "\"bdsm\" never counts as a bad word");
}

/* ───────────────────────────── serde of crawler payloads ───────────────────────────── */

#[test]
fn html_result_bincode_roundtrip() {
    let m = meta(&page("<title>T</title><meta name='keywords' content='k'>", "<p>Body</p>"));
    let bytes = bincode_next::serde::encode_to_vec(&vec![m], bincode_next::config::standard()).unwrap();
    let (back, _): (Vec<HTMLResultData>, usize) =
        bincode_next::serde::decode_from_slice(&bytes, bincode_next::config::standard()).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].title, "T");
    assert_eq!(back[0].keywords, "k");
    assert_eq!(back[0].tag_data.get("p").unwrap()[0], "Body");
}

#[test]
fn worker_batch_response_bincode_roundtrip() {
    let r = WorkerBatchResponse {
        batch_id: "b1".into(),
        urls: vec!["https://a.com".into(), "https://b.com".into()],
    };
    let bytes = bincode_next::serde::encode_to_vec(&r, bincode_next::config::standard()).unwrap();
    let (back, read): (WorkerBatchResponse, usize) =
        bincode_next::serde::decode_from_slice(&bytes, bincode_next::config::standard()).unwrap();
    assert_eq!(read, bytes.len());
    assert_eq!(back.batch_id, "b1");
    assert_eq!(back.urls, r.urls);
}

#[test]
fn html_result_bincode_rejects_truncated_input() {
    let m = meta(&page("<title>T</title>", "<p>Body</p>"));
    let bytes = bincode_next::serde::encode_to_vec(&m, bincode_next::config::standard()).unwrap();
    let r: Result<(HTMLResultData, usize), _> =
        bincode_next::serde::decode_from_slice(&bytes[..bytes.len() / 2], bincode_next::config::standard());
    assert!(r.is_err());
}
