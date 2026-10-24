//! Tests for crates/crawler/src/tag_data.rs and the helpers in mini_crawler.rs

use prieco_mini_crawler::{
    meta::get_domain,
    mini_crawler::{CompiledSelectors, PARSER_SELECTORS, is_valid_url},
    tag_data::extract_tag_data,
};
use scraper::Html;

fn tags(html: &str) -> ahash::AHashMap<String, Vec<String>> {
    extract_tag_data(&Html::parse_document(html))
}

fn get<'a>(m: &'a ahash::AHashMap<String, Vec<String>>, k: &str) -> Vec<&'a str> {
    m.get(k)
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default()
}

/* ───────────────────────────── is_valid_url ───────────────────────────── */

#[test]
fn valid_http_and_https_urls() {
    for u in [
        "http://example.com",
        "https://example.com/a?b=c#d",
        "https://sub.example.co.uk:8443/path",
        "http://127.0.0.1/",
        "http://localhost",
        "HTTPS://EXAMPLE.COM",
        "https://bücher.de",
        "https://[::1]/",
        "  https://example.com  ",
    ] {
        assert!(is_valid_url(u), "{:?}", u);
    }
}

#[test]
fn invalid_or_non_http_urls() {
    for u in [
        "",
        "example.com",
        "/relative/path",
        "//example.com/path",
        "ftp://example.com",
        "mailto:a@b.c",
        "javascript:alert(1)",
        "data:text/html,hi",
        "file:///etc/passwd",
        "https://",
        "http://exa mple.com",
        "ws://example.com",
    ] {
        assert!(!is_valid_url(u), "{:?}", u);
    }
}

/* ───────────────────────────── get_domain ───────────────────────────── */

#[test]
fn get_domain_keeps_www_when_not_removing() {
    assert_eq!(get_domain("https://www.example.com/a", false), "www.example.com");
}

#[test]
fn get_domain_removes_www() {
    assert_eq!(get_domain("https://www.example.com/a", true), "example.com");
}

#[test]
fn get_domain_only_removes_leading_www_label() {
    assert_eq!(get_domain("https://wwwexample.com", true), "wwwexample.com");
    assert_eq!(get_domain("https://a.www.example.com", true), "a.www.example.com");
    assert_eq!(get_domain("https://www.www.example.com", true), "www.example.com");
}

#[test]
fn get_domain_lowercases_and_drops_port_and_path() {
    assert_eq!(get_domain("https://WWW.Example.COM:8080/x?y#z", true), "example.com");
}

#[test]
fn get_domain_idn_is_punycode() {
    assert_eq!(get_domain("https://bücher.de/", true), "xn--bcher-kva.de");
}

#[test]
fn get_domain_ip_and_invalid_inputs_are_empty() {
    for u in ["", "not a url", "http://127.0.0.1/", "http://[::1]/", "mailto:a@b.c", "example.com"] {
        assert_eq!(get_domain(u, true), "", "{:?}", u);
        assert_eq!(get_domain(u, false), "", "{:?}", u);
    }
}

/* ───────────────────────────── selectors ───────────────────────────── */

#[test]
fn compiled_selectors_shape() {
    let s = CompiledSelectors::new();
    assert_eq!(s.h_selectors.len(), 6);
    assert_eq!(s.text_selectors.len(), 5);
    assert_eq!(s.content_selectors.len(), 6);
    let global = &**PARSER_SELECTORS;
    assert_eq!(global.h_selectors.len(), 6);
}

#[test]
fn compiled_selectors_match_expected_elements() {
    let s = CompiledSelectors::new();
    let doc = Html::parse_document(
        r#"<html><head>
        <meta name="keywords" content="k"><meta name="description" content="d">
        <meta property="og:site_name" content="S"><meta property="og:description" content="OD">
        <link rel="icon" href="/i.ico"><link rel="shortcut icon" href="/s.ico">
        <link rel="apple-touch-icon" href="/a.png"><title>T</title></head>
        <body><h1>1</h1><h6>6</h6><p>p</p><img src="x"><a href="y">a</a></body></html>"#,
    );
    assert_eq!(doc.select(&s.meta_keywords_selector).count(), 1);
    assert_eq!(doc.select(&s.meta_description_selector).count(), 1);
    assert_eq!(doc.select(&s.og_site_name_selector).count(), 1);
    assert_eq!(doc.select(&s.og_description_selector).count(), 1);
    assert_eq!(doc.select(&s.link_icon_selector).count(), 1);
    assert_eq!(doc.select(&s.link_shortcut_selector).count(), 1);
    assert_eq!(doc.select(&s.link_apple_selector).count(), 1);
    assert_eq!(doc.select(&s.title_selector).count(), 1);
    assert_eq!(doc.select(&s.meta_selector).count(), 4);
    assert_eq!(doc.select(&s.h_selectors[0]).count(), 1);
    assert_eq!(doc.select(&s.h_selectors[5]).count(), 1);
    assert_eq!(doc.select(&s.img_selector).count(), 1);
    assert_eq!(doc.select(&s.a_selector).count(), 1);
}

/* ───────────────────────────── text extraction ───────────────────────────── */

#[test]
fn empty_document_has_no_tag_data() {
    assert!(tags("").is_empty());
    assert!(tags("<html><body></body></html>").is_empty());
}

#[test]
fn headings_are_extracted_per_level() {
    let t = tags("<h1>One</h1><h2>Two</h2><h3>Three</h3><h4>Four</h4><h5>Five</h5><h6>Six</h6>");
    assert_eq!(get(&t, "h1"), vec!["One"]);
    assert_eq!(get(&t, "h2"), vec!["Two"]);
    assert_eq!(get(&t, "h3"), vec!["Three"]);
    assert_eq!(get(&t, "h4"), vec!["Four"]);
    assert_eq!(get(&t, "h5"), vec!["Five"]);
    assert_eq!(get(&t, "h6"), vec!["Six"]);
}

#[test]
fn multiple_elements_keep_document_order() {
    let t = tags("<p>first</p><div><p>second</p></div><p>third</p>");
    assert_eq!(get(&t, "p"), vec!["first", "second", "third"]);
}

#[test]
fn text_whitespace_is_collapsed() {
    let t = tags("<p>  lots \n\t of    space  </p>");
    assert_eq!(get(&t, "p"), vec!["lots of space"]);
}

#[test]
fn only_first_text_node_is_taken() {
    let t = tags("<p>Hello <b>World</b> again</p>");
    assert_eq!(get(&t, "p"), vec!["Hello"]);
}

#[test]
fn element_starting_with_child_takes_childs_text() {
    let t = tags("<li><a href='https://x.com'>Link</a> tail</li>");
    assert_eq!(get(&t, "li"), vec!["Link"]);
}

#[test]
fn empty_and_whitespace_only_elements_are_skipped() {
    let t = tags("<p></p><p>   </p><p>ok</p><span>\n</span>");
    assert_eq!(get(&t, "p"), vec!["ok"]);
    assert!(t.get("span").is_none());
}

#[test]
fn span_li_label_text() {
    let t = tags("<span>s</span><ul><li>l1</li><li>l2</li></ul><label>lab</label>");
    assert_eq!(get(&t, "span"), vec!["s"]);
    assert_eq!(get(&t, "li"), vec!["l1", "l2"]);
    assert_eq!(get(&t, "label"), vec!["lab"]);
}

#[test]
fn text_entities_are_decoded() {
    let t = tags("<p>Tom &amp; Jerry &lt;3</p>");
    assert_eq!(get(&t, "p"), vec!["Tom & Jerry <3"]);
}

#[test]
fn script_and_style_content_is_not_text_of_p() {
    let t = tags("<script>var p = 1;</script><style>p{}</style><p>real</p>");
    assert_eq!(get(&t, "p"), vec!["real"]);
}

#[test]
fn anchor_texts_are_deduplicated() {
    let t = tags("<a href='https://a.com'>Same</a><a href='https://b.com'>Same</a><a>Other</a>");
    assert_eq!(get(&t, "a"), vec!["Same", "Other"]);
}

#[test]
fn anchor_texts_are_limited_to_20() {
    let html: String = (0..30)
        .map(|i| format!("<a href='https://s{0}.com'>text{0}</a>", i))
        .collect();
    let t = tags(&html);
    assert_eq!(get(&t, "a").len(), 20);
    assert_eq!(get(&t, "a")[0], "text0");
    assert_eq!(get(&t, "a")[19], "text19");
}

/* ───────────────────────────── links ───────────────────────────── */

#[test]
fn absolute_links_get_trailing_slash() {
    let t = tags("<a href='https://example.com'>x</a><a href='https://example.com/page/'>y</a>");
    assert_eq!(
        get(&t, "a_href"),
        vec!["https://example.com/", "https://example.com/page/"]
    );
}

#[test]
fn relative_and_non_http_links_are_dropped() {
    let t = tags(
        "<a href='/relative'>1</a><a href='page.html'>2</a><a href='#top'>3</a>\
         <a href='javascript:void(0)'>4</a><a href='mailto:a@b.c'>5</a><a href=''>6</a><a>7</a>",
    );
    assert!(t.get("a_href").is_none());
}

#[test]
fn duplicate_links_are_removed() {
    let t = tags(
        "<a href='https://a.com/x'>1</a><a href='https://a.com/x'>2</a><a href='https://a.com/x/'>3</a>",
    );
    assert_eq!(get(&t, "a_href"), vec!["https://a.com/x/"]);
}

#[test]
fn links_with_surrounding_spaces_are_cleaned() {
    let t = tags("<a href='   https://a.com/x'>1</a>");
    assert_eq!(get(&t, "a_href"), vec!["https://a.com/x/"]);
}

#[test]
fn link_with_query_keeps_query() {
    let t = tags("<a href='https://a.com/s?q=rust'>1</a>");
    assert_eq!(get(&t, "a_href"), vec!["https://a.com/s?q=rust/"]);
}

/// BUG: the link cleaner looks for "http://" *anywhere* first, so an https link that
/// carries an http url in its query string is cut down to the embedded url.
#[test]
fn https_link_with_embedded_http_url_is_kept_whole() {
    let t = tags("<a href='https://a.com/redirect?to=http://b.com'>x</a>");
    assert_eq!(
        get(&t, "a_href"),
        vec!["https://a.com/redirect?to=http://b.com/"]
    );
}

/// BUG: "Limit to first 20 unique <a> URLs" is applied to the key "a" (anchor
/// *texts*), the real URL list "a_href" is never limited.
#[test]
fn link_urls_are_limited_to_20() {
    let html: String = (0..30)
        .map(|i| format!("<a href='https://s{0}.com/'>t{0}</a>", i))
        .collect();
    let t = tags(&html);
    assert!(get(&t, "a_href").len() <= 20, "got {}", get(&t, "a_href").len());
}

#[test]
fn image_sources_are_extracted() {
    let t = tags("<img src='https://cdn.example.com/a.png'><img src='/local.png'><img>");
    let imgs = get(&t, "img_src");
    assert_eq!(imgs.len(), 1);
    assert!(imgs[0].starts_with("https://cdn.example.com/a.png"));
}

/// BUG: `add_slash = true` is also used for <img src>, so every image URL gets a
/// trailing "/" ("a.png/") which points to a different (usually 404) resource.
#[test]
fn image_urls_are_not_modified() {
    let t = tags("<img src='https://cdn.example.com/a.png'>");
    assert_eq!(get(&t, "img_src"), vec!["https://cdn.example.com/a.png"]);
}

/* ───────────────────────────── meta ───────────────────────────── */

#[test]
fn meta_name_content_pairs() {
    let t = tags("<meta name='description' content='Desc'><meta name='author' content='Me'>");
    assert_eq!(get(&t, "meta"), vec!["description=Desc", "author=Me"]);
}

#[test]
fn meta_property_content_pairs() {
    let t = tags("<meta property='og:title' content='T'>");
    assert_eq!(get(&t, "meta"), vec!["og:title=T"]);
}

#[test]
fn meta_without_content_or_name_is_ignored() {
    let t = tags("<meta charset='utf-8'><meta name='x'><meta content='orphan'>");
    assert!(t.get("meta").is_none());
}

/// BUG: name metas and property metas are both stored under the key "meta";
/// the property pass overwrites the name pass, losing description/keywords/etc.
#[test]
fn meta_names_survive_when_properties_exist() {
    let t = tags("<meta name='description' content='Desc'><meta property='og:title' content='T'>");
    let m = get(&t, "meta");
    assert!(m.contains(&"description=Desc"), "{:?}", m);
    assert!(m.contains(&"og:title=T"), "{:?}", m);
}

#[test]
fn malformed_html_does_not_panic() {
    for html in [
        "<p>unclosed",
        "<<<>>>",
        "<a href=",
        "</p></p></div>",
        "<img src='https://x.com/a.png'",
        "\0\0\0",
        "<table><tr><td>cell</table>",
    ] {
        let _ = tags(html);
    }
}
