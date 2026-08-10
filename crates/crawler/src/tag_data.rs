use ahash::{AHashMap, AHashSet};
use scraper::{Html, Selector};

use crate::mini_crawler::{PARSER_SELECTORS, is_valid_url};

/// Description: extract data from html documnet and format them to tag_data
/// 
/// Input: document
/// Output: tag_data
pub fn extract_tag_data(document: &Html) -> AHashMap<String, Vec<String>> {
    /*
      Description: tag_data is a very important variable. It provides organized, extracted access to important data from the html.
      Keys:
        Text: h1-h6, span, p, a, li, label
        Meta: meta_name, meta_property
        Link: a_href, img_src
    */
    let mut tag_data: AHashMap<String, Vec<String>> = AHashMap::new();
    let selectors = &**PARSER_SELECTORS;

    // Extract h1..h6 tags
    for (i, h_selector) in selectors.h_selectors.iter().enumerate() {
        let tag_name = format!("h{}", i + 1);
        extract_data(
            document,
            &mut tag_data,
            h_selector,
            &tag_name,
            "text",
            false,
        );
    }

    // Extract text from these tags
    let tags = ["span", "p", "a", "li", "label"];
    for (selector, &tag_name) in selectors.text_selectors.iter().zip(tags.iter()) {
        extract_data(document, &mut tag_data, selector, tag_name, "text", false);
    }

    // Extract meta data
    extract_data(
        document,
        &mut tag_data,
        &selectors.meta_selector,
        "meta",
        "meta_name",
        false,
    );
    extract_data(
        document,
        &mut tag_data,
        &selectors.meta_selector,
        "meta",
        "meta_property",
        false,
    );

    // Extract links such as <img> and <a>
    extract_data(
        document,
        &mut tag_data,
        &selectors.img_selector,
        "img",
        "src",
        true,
    );
    extract_data(
        document,
        &mut tag_data,
        &selectors.a_selector,
        "a",
        "href",
        true,
    );

    // Limit to first 20 unique <a> URLs
    if let Some(urls) = tag_data.get_mut("a") {
        let mut seen = AHashSet::with_capacity(100);
        let mut limited = Vec::with_capacity(25);

        for url in urls.iter() {
            if seen.insert(url) {
                // insert returns false if already in set
                limited.push(url.clone());
            }
            if limited.len() >= 20 {
                break;
            }
        }

        *urls = limited;
    }

    tag_data
}

/*
Description: Function to finally extract data to tag_data. Called by extract_tag_data for each tag. Modifies tag_data dirrectly

Input: document, &mut tag_data, tag (tag of element to extract data from),
keyword (text to extract inner text, meta_name, meta_property, inner tag [href, link] to extract data from it), add_slash
Output: None
*/
fn extract_data(
    document: &Html,
    tag_data: &mut AHashMap<String, Vec<String>>,
    selector: &Selector,
    tag: &str,
    keyword: &str,
    add_slash: bool,
) {
    let mut seen_url = AHashSet::new(); // Filter duplicate urls from <a> and <img> tags

    //Extract data by keyword
    let data: Vec<String> = document
        .select(selector)
        .filter_map(|element| match keyword {
            "text" => element
                .text()
                .next()
                .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|cleaned| !cleaned.is_empty()), // Extract inner text from element
            "meta_name" => {
                if let Some(content) = element.value().attr("content") {
                    if let Some(name) = element.value().attr("name") {
                        return Some(format!("{}={}", name, content));
                    }
                }
                None
            } // Extract content from meta tags
            "meta_property" => {
                if let Some(content) = element.value().attr("content") {
                    if let Some(property) = element.value().attr("property") {
                        return Some(format!("{}={}", property, content));
                    }
                }
                None
            } // Extract property from meta tags
            _ => element
                .value()
                .attr(keyword)
                .map(|attr| {
                    if !is_valid_url(&attr) {
                        return String::new();
                    }
                    // Find the position of http:// or https://
                    let start_pos = if let Some(pos) = attr.find("http://") {
                        pos
                    } else if let Some(pos) = attr.find("https://") {
                        pos
                    } else {
                        0
                    };
                    // Get the substring from the protocol onwards
                    let cleaned_url = &attr[start_pos..];

                    if add_slash && !cleaned_url.ends_with('/') {
                        format!("{}/", cleaned_url)
                    } else {
                        cleaned_url.to_string()
                    }
                })
                .filter(|attr| {
                    if !attr.is_empty() {
                        seen_url.insert(attr.clone())
                    } else {
                        false
                    }
                }), // Extract data from different value in tag. Used for a href and img src
        })
        .collect();

    // Insert data to tag_data, this checks if keyword is any (the last extraction) or predefined one
    if !data.is_empty() {
        if !keyword.is_empty()
            && keyword != "text"
            && keyword != "meta_name"
            && keyword != "meta_property"
        {
            tag_data.insert(format!("{}_{}", tag, keyword), data);
        } else {
            tag_data.insert(tag.to_string(), data);
        }
    }
}
