//! # Blob Decoder
//!
//! Turns a stored blob back into text or html.
//!
//! ## Blob Format
//!
//! 1. First byte: 1 if the rest is zstd compressed.
//! 2. Then tags, each one is:
//!     * Tag byte, see [tag_byte_to_name].
//!     * Tokens: length byte + bytes.
//!     * End marker `[255, 255, 255, 255]`.
//! 3. [Token][resolve_token] is a short word written as is, or a word id.
//!
//! ## Modes
//!
//! * [**Text**:][decode_blob_to_text] `<tag>words</tag>` per line.
//! * [**Html**:][decode_blob_to_html_rendered] Readable html, images go through a proxy.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-08-12
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] Fill [DICT_CACHE], now nothing inserts to it so word ids decode to empty strings

/*
  Import external libraries
*/
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Output of [decode_blob].
pub enum DecodeMode<'a> {
    /// Plain `<tag>words</tag>` lines.
    Text,
    /// Html for viewing, `proxy_prefix` goes before every image URL.
    Html { proxy_prefix: &'a str },
}

/* APIs */
/// Decodes blob to text.
///
/// # Arguments
///
/// * `raw_db_value` - Blob from storage.
///
/// # Returns
///
/// `<tag>words</tag>` per line, empty string if blob is empty or broken.
pub fn decode_blob_to_text(raw_db_value: &[u8]) -> String {
    decode_blob(raw_db_value, DecodeMode::Text)
}

/// Decodes blob to html for viewing.
///
/// Every tag is a div with `[tag]` label. Images are loaded through `proxy_prefix`,
/// links open in new tab. Empty tags are skipped.
///
/// # Arguments
///
/// * `raw_db_value` - Blob from storage.
/// * `proxy_prefix` - Image proxy, URL encoded image URL is added after it.
///
/// # Returns
///
/// Html, empty string if blob is empty or broken.
pub fn decode_blob_to_html_rendered(raw_db_value: &[u8], proxy_prefix: &str) -> String {
    decode_blob(raw_db_value, DecodeMode::Html { proxy_prefix })
}

/* Main function */

/// Decodes blob in given mode.
///
/// Broken zstd data gives empty string. A token longer than the rest of data ends the tag.
///
/// # Arguments
///
/// * `raw_db_value` - Blob from storage.
/// * `mode` - Output mode.
///
/// # Returns
///
/// Decoded blob.
///
/// # Panics
///
/// Only if system runs out of memory.
fn decode_blob(raw_db_value: &[u8], mode: DecodeMode) -> String {
    if raw_db_value.is_empty() {
        return String::new();
    }

    let is_compressed = raw_db_value[0] == 1;
    let payload = &raw_db_value[1..];

    let decompressed = if is_compressed {
        zstd::stream::decode_all(std::io::Cursor::new(payload)).unwrap_or_default()
    } else {
        payload.to_vec()
    };

    if decompressed.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(decompressed.len() * 4);
    let mut i = 0;

    while i < decompressed.len() {
        let tag_byte = decompressed[i];
        i += 1;

        let tag_name = tag_byte_to_name(tag_byte);
        let is_url = tag_byte == b'i' || tag_byte == b'h';
        let mut inner_content = String::new();

        while i < decompressed.len() {
            if i + 3 < decompressed.len() && decompressed[i..i + 4] == [255, 255, 255, 255] {
                i += 4;
                break;
            }

            let len = decompressed[i] as usize;
            i += 1;

            if len == 0 {
                continue;
            }
            if i + len > decompressed.len() {
                break;
            }

            let slice = &decompressed[i..i + len];
            let (resolved_word, _) = resolve_token(slice);

            let word_str: &str = match &resolved_word {
                ResolvedToken::Raw(s) => s,
                ResolvedToken::Cached(arc) => arc.as_str(),
            };

            inner_content.push_str(word_str);

            match mode {
                DecodeMode::Text => {
                    inner_content.push(' ');
                }
                DecodeMode::Html { .. } => {
                    if !is_url && i + len < decompressed.len() && decompressed[i + len] != 255 {
                        inner_content.push(' ');
                    }
                }
            }

            i += len;
        }

        match mode {
            DecodeMode::Text => {
                output.push('<');
                output.push_str(tag_name);
                output.push('>');
                output.push_str(&inner_content);
                output.push_str("</");
                output.push_str(tag_name);
                output.push_str(">\n");
            }
            DecodeMode::Html { proxy_prefix } => {
                let inner = inner_content.trim();
                if inner.is_empty() {
                    continue;
                }

                let tag_label = format!("<strong>[{}]</strong>", tag_name);

                match tag_byte {
                    b'i' => {
                        let encoded_url = urlencoding::encode(inner);
                        output.push_str(&format!(
                            "<div>{}<br><img src=\"{}{}\" /></div>\n",
                            tag_label, proxy_prefix, encoded_url
                        ));
                    }
                    b'h' => {
                        output.push_str(&format!(
                            "<div>{}<a href=\"{}\" target=\"_blank\">{}</a></div>\n",
                            tag_label, inner, inner
                        ));
                    }
                    b'm' => {
                        output.push_str(&format!("<div>{} <i>{}</i></div>\n", tag_label, inner));
                    }
                    _ => {
                        output.push_str(&format!(
                            "<div>{}<br><span>{}</span></div>\n",
                            tag_label, inner
                        ));
                    }
                }
            }
        }
    }

    output
}

/* Helper functions */

/* Decoder */
/// Word from a token.
pub enum ResolvedToken<'a> {
    /// Short word stored as is.
    Raw(&'a str),
    /// Word found by id in [DICT_CACHE].
    Cached(Arc<String>),
}

/// Turns token bytes into a word.
///
/// * Longer than 8 bytes: empty.
/// * 4 to 8 bytes: little endian word id.
/// * Up to 3 bytes of letters, digits or `.,-!?'`: the word itself.
/// * Other 1 to 3 bytes: word id.
///
/// # Returns
///
/// (word, true if found).
fn resolve_token<'a>(slice: &'a [u8]) -> (ResolvedToken<'a>, bool) {
    if slice.len() > 8 {
        return (ResolvedToken::Raw(""), false);
    }

    if slice.len() > 3 {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        let (word, hit) = search_word_by_id(id as usize);
        return (ResolvedToken::Cached(word), hit);
    }

    let is_normal_short_word = slice.iter().all(|&b| {
        b.is_ascii_alphanumeric()
            || b == b'.'
            || b == b','
            || b == b'-'
            || b == b'!'
            || b == b'?'
            || b == b'\''
    }) && std::str::from_utf8(slice).is_ok();

    if is_normal_short_word {
        (
            ResolvedToken::Raw(std::str::from_utf8(slice).unwrap()),
            true,
        )
    } else {
        let mut id = 0u64;
        for (j, b) in slice.iter().enumerate() {
            id |= (*b as u64) << (8 * j);
        }
        let (word, hit) = search_word_by_id(id as usize);
        (ResolvedToken::Cached(word), hit)
    }
}

/// Tag byte → tag name. Unknown bytes are 'div'.
fn tag_byte_to_name(tag_byte: u8) -> &'static str {
    match tag_byte {
        b'1' => "h1",
        b'2' => "h2",
        b'3' => "h3",
        b'4' => "h4",
        b'5' => "h5",
        b'6' => "h6",
        b's' => "span",
        b'p' => "p",
        b'a' => "a",
        b'l' => "li",
        b'b' => "label",
        b'm' => "meta",
        b'i' => "img_src",
        b'h' => "a_href",
        _ => "div",
    }
}

/// Word id → word.
static DICT_CACHE: Lazy<DashMap<usize, Arc<String>>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_ITEMS + 1_000));
/// Max words in [DICT_CACHE].
const MAX_CACHE_ITEMS: usize = 1_000_000;

/// Finds word by id.
///
/// Ids under 256 are empty words.
///
/// # Returns
///
/// (word, true if found). Not found gives empty word.
pub fn search_word_by_id(id: usize) -> (Arc<String>, bool) {
    if id < 256 {
        return (Arc::new(String::new()), true);
    }

    if let Some(cached_word) = DICT_CACHE.get(&id) {
        return (cached_word.value().clone(), true);
    }

    (Arc::new(String::new()), false)
}
