//! Tests for crates/blob/src/decode.rs
//!
//! Blob format:   [flag: 1 = zstd, anything else = raw] [payload]
//! Payload:       repeated { [tag byte] { [len][token bytes] }* [FF FF FF FF] }
//! Tokens of 1..=3 "plain" bytes are inline text, everything else is a dictionary id.

use prieco_blob::decode::{
    DecodeMode, ResolvedToken, decode_blob_to_html_rendered, decode_blob_to_text,
    search_word_by_id,
};

const END: [u8; 4] = [255, 255, 255, 255];

/// Builds an uncompressed payload (without flag)
fn payload(tags: &[(u8, &[&[u8]])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (tag, tokens) in tags {
        out.push(*tag);
        for t in *tokens {
            out.push(t.len() as u8);
            out.extend_from_slice(t);
        }
        out.extend_from_slice(&END);
    }
    out
}

fn raw_blob(tags: &[(u8, &[&[u8]])]) -> Vec<u8> {
    let mut b = vec![0u8];
    b.extend(payload(tags));
    b
}

fn zstd_blob(tags: &[(u8, &[&[u8]])]) -> Vec<u8> {
    let mut b = vec![1u8];
    b.extend(zstd::stream::encode_all(&payload(tags)[..], 3).unwrap());
    b
}

/* ───────────────────────────── empty / degenerate input ───────────────────────────── */

#[test]
fn empty_input_is_empty_output() {
    assert_eq!(decode_blob_to_text(&[]), "");
    assert_eq!(decode_blob_to_html_rendered(&[], "/p?u="), "");
}

#[test]
fn flag_only_is_empty_output() {
    assert_eq!(decode_blob_to_text(&[0]), "");
    assert_eq!(decode_blob_to_text(&[1]), "");
    assert_eq!(decode_blob_to_html_rendered(&[0], ""), "");
}

#[test]
fn invalid_zstd_payload_is_empty_output() {
    assert_eq!(decode_blob_to_text(&[1, 9, 9, 9, 9, 9]), "");
    assert_eq!(decode_blob_to_html_rendered(&[1, 0xde, 0xad], ""), "");
}

#[test]
fn zstd_of_empty_payload_is_empty_output() {
    let mut b = vec![1u8];
    b.extend(zstd::stream::encode_all(&[][..], 3).unwrap());
    assert_eq!(decode_blob_to_text(&b), "");
}

#[test]
fn any_non_one_flag_means_raw() {
    let p = payload(&[(b'p', &[b"hi"])]);
    for flag in [0u8, 2, 7, 255] {
        let mut b = vec![flag];
        b.extend_from_slice(&p);
        assert_eq!(decode_blob_to_text(&b), "<p>hi </p>\n", "flag {}", flag);
    }
}

/* ───────────────────────────── text mode ───────────────────────────── */

#[test]
fn text_single_tag_single_word() {
    assert_eq!(decode_blob_to_text(&raw_blob(&[(b'p', &[b"hi"])])), "<p>hi </p>\n");
}

#[test]
fn text_words_are_space_separated_with_trailing_space() {
    let b = raw_blob(&[(b'p', &[b"hi", b"you", b"a"])]);
    assert_eq!(decode_blob_to_text(&b), "<p>hi you a </p>\n");
}

#[test]
fn text_multiple_tags_in_order() {
    let b = raw_blob(&[(b'1', &[b"Big"]), (b'p', &[b"txt"]), (b'l', &[b"li"])]);
    assert_eq!(
        decode_blob_to_text(&b),
        "<h1>Big </h1>\n<p>txt </p>\n<li>li </li>\n"
    );
}

#[test]
fn text_empty_tag_is_still_emitted() {
    let b = raw_blob(&[(b'p', &[])]);
    assert_eq!(decode_blob_to_text(&b), "<p></p>\n");
}

#[test]
fn text_tag_without_terminator_at_end_of_data() {
    assert_eq!(decode_blob_to_text(&[0, b'p']), "<p></p>\n");
    assert_eq!(decode_blob_to_text(&[0, b'p', 2, b'o', b'k']), "<p>ok </p>\n");
}

#[test]
fn text_zero_length_tokens_are_skipped() {
    let b = [0, b'p', 0, 0, 2, b'h', b'i', 0, 255, 255, 255, 255];
    assert_eq!(decode_blob_to_text(&b), "<p>hi </p>\n");
}

#[test]
fn text_allowed_punctuation_in_short_tokens() {
    let b = raw_blob(&[(b'p', &[b"a.b", b"c,", b"-!", b"?", b"'s", b"A1"])]);
    assert_eq!(decode_blob_to_text(&b), "<p>a.b c, -! ? 's A1 </p>\n");
}

#[test]
fn text_short_tokens_with_other_bytes_resolve_through_dictionary() {
    // ':' '/' ' ' and non ascii are not "plain", they become dictionary ids.
    // The dictionary cache is empty, so they decode to "" (but still add a space).
    let b = raw_blob(&[(b'p', &[b"a:b", b"/", b" ", "é".as_bytes(), b"ok"])]);
    assert_eq!(decode_blob_to_text(&b), "<p>    ok </p>\n");
}

#[test]
fn text_long_tokens_resolve_through_dictionary() {
    // tokens longer than 3 bytes are always dictionary ids
    let b = raw_blob(&[(b'p', &[b"word", &[1, 2, 3, 4, 5, 6, 7, 8], b"hi"])]);
    assert_eq!(decode_blob_to_text(&b), "<p>  hi </p>\n");
}

#[test]
fn text_all_tag_names() {
    let cases: &[(u8, &str)] = &[
        (b'1', "h1"),
        (b'2', "h2"),
        (b'3', "h3"),
        (b'4', "h4"),
        (b'5', "h5"),
        (b'6', "h6"),
        (b's', "span"),
        (b'p', "p"),
        (b'a', "a"),
        (b'l', "li"),
        (b'b', "label"),
        (b'm', "meta"),
        (b'i', "img_src"),
        (b'h', "a_href"),
        (b'z', "div"),
        (b'0', "div"),
        (b'7', "div"),
        (0, "div"),
    ];
    for (byte, name) in cases {
        let b = raw_blob(&[(*byte, &[b"x"])]);
        assert_eq!(
            decode_blob_to_text(&b),
            format!("<{0}>x </{0}>\n", name),
            "tag byte {}",
            byte
        );
    }
}

#[test]
fn text_url_tags_still_get_spaces_in_text_mode() {
    let b = raw_blob(&[(b'i', &[b"ab", b"c.d"])]);
    assert_eq!(decode_blob_to_text(&b), "<img_src>ab c.d </img_src>\n");
}

#[test]
fn text_compressed_and_raw_decode_identically() {
    let tags: &[(u8, &[&[u8]])] = &[
        (b'1', &[b"Hi", b"all"]),
        (b'p', &[b"ok", b"a.b", b"zz"]),
        (b'h', &[b"x.y"]),
        (b'm', &[b"k=v"]),
    ];
    assert_eq!(decode_blob_to_text(&raw_blob(tags)), decode_blob_to_text(&zstd_blob(tags)));
    assert_eq!(
        decode_blob_to_html_rendered(&raw_blob(tags), "/x?u="),
        decode_blob_to_html_rendered(&zstd_blob(tags), "/x?u=")
    );
}

#[test]
fn text_large_document() {
    let words: Vec<&[u8]> = (0..5000).map(|_| &b"ab"[..]).collect();
    let b = raw_blob(&[(b'p', &words)]);
    let out = decode_blob_to_text(&b);
    assert_eq!(out.matches("ab ").count(), 5000);
    assert!(out.starts_with("<p>") && out.ends_with("</p>\n"));
}

/* ───────────────────────────── malformed input robustness ───────────────────────────── */

#[test]
fn truncated_token_does_not_panic_and_rest_is_parsed_as_tags() {
    // len 5 but only 1 byte left: the token is abandoned and the remaining byte
    // is re-read as a new tag byte ('a' -> <a>)
    assert_eq!(decode_blob_to_text(&[0, b'p', 5, b'a']), "<p></p>\n<a></a>\n");
}

#[test]
fn short_terminator_at_end_does_not_panic() {
    assert_eq!(
        decode_blob_to_text(&[0, b'p', 255, 255, 255]),
        "<p></p>\n<div></div>\n"
    );
}

/// BUG: `resolve_token` builds the dictionary id with `b << (8 * j)` for every byte of the
/// token. For tokens longer than 8 bytes the shift is >= 64 bits, which panics in debug
/// builds ("attempt to shift left with overflow") and silently wraps in release.
#[test]
fn token_longer_than_8_bytes_does_not_panic() {
    let long = [b'x'; 12];
    let b = raw_blob(&[(b'p', &[&long[..], b"ok"])]);
    let r = std::panic::catch_unwind(|| decode_blob_to_text(&b));
    assert!(r.is_ok(), "decoding a 12 byte token panicked");
}

#[test]
fn max_length_token_does_not_panic() {
    let long = [7u8; 254];
    let b = raw_blob(&[(b'p', &[&long[..]])]);
    let r = std::panic::catch_unwind(|| decode_blob_to_html_rendered(&b, ""));
    assert!(r.is_ok(), "decoding a 254 byte token panicked");
}

#[test]
fn random_garbage_never_panics() {
    // simple deterministic LCG, no extra deps
    let mut x: u64 = 0x1234_5678_9abc_def0;
    for len in 0..300usize {
        let mut b = Vec::with_capacity(len);
        for _ in 0..len {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            b.push((x >> 56) as u8);
        }
        let _ = decode_blob_to_text(&b);
        let _ = decode_blob_to_html_rendered(&b, "/proxy?u=");
        if !b.is_empty() {
            b[0] = 0;
            let _ = decode_blob_to_text(&b);
            let _ = decode_blob_to_html_rendered(&b, "/proxy?u=");
        }
    }
}

#[test]
fn every_single_and_double_byte_payload_never_panics() {
    for a in 0..=255u8 {
        let _ = decode_blob_to_text(&[0, a]);
        for b in [0u8, 1, 2, 3, 4, 254, 255] {
            let _ = decode_blob_to_text(&[0, a, b]);
            let _ = decode_blob_to_html_rendered(&[0, a, b], "");
        }
    }
}

#[test]
fn output_is_always_valid_utf8_string() {
    // String return type guarantees it, but make sure invalid bytes don't leak as raw text
    let b = raw_blob(&[(b'p', &[&[0xff, 0xfe], &[0xc3]])]);
    let out = decode_blob_to_text(&b);
    assert!(!out.contains('\u{FFFD}'));
    assert_eq!(out, "<p>  </p>\n");
}

/* ───────────────────────────── html mode ───────────────────────────── */

#[test]
fn html_text_tag_is_wrapped_in_span() {
    let b = raw_blob(&[(b'p', &[b"hi", b"you"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, "/proxy?u="),
        "<div><strong>[p]</strong><br><span>hi you</span></div>\n"
    );
}

#[test]
fn html_no_trailing_space_before_terminator() {
    let b = raw_blob(&[(b'2', &[b"a", b"b", b"c"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[h2]</strong><br><span>a b c</span></div>\n"
    );
}

#[test]
fn html_empty_tags_are_skipped() {
    let b = raw_blob(&[(b'p', &[]), (b'1', &[b"ok"]), (b's', &[b"word"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[h1]</strong><br><span>ok</span></div>\n"
    );
}

#[test]
fn html_image_uses_proxy_prefix_and_url_encoding() {
    let b = raw_blob(&[(b'i', &[b"ab", b"c.d", b"e,f"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, "/proxy?u="),
        "<div><strong>[img_src]</strong><br><img src=\"/proxy?u=abc.de%2Cf\" /></div>\n"
    );
}

#[test]
fn html_url_tags_are_joined_without_spaces() {
    let b = raw_blob(&[(b'h', &[b"ex", b"a.c", b"om"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[a_href]</strong><a href=\"exa.com\" target=\"_blank\">exa.com</a></div>\n"
    );
}

#[test]
fn html_meta_tag_is_italic() {
    let b = raw_blob(&[(b'm', &[b"k", b"v"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[meta]</strong> <i>k v</i></div>\n"
    );
}

#[test]
fn html_unknown_tag_renders_as_div_span() {
    let b = raw_blob(&[(b'q', &[b"x"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[div]</strong><br><span>x</span></div>\n"
    );
}

#[test]
fn html_proxy_prefix_is_used_verbatim() {
    let b = raw_blob(&[(b'i', &[b"a.b"])]);
    let out = decode_blob_to_html_rendered(&b, "https://cdn.example/?img=");
    assert!(out.contains("src=\"https://cdn.example/?img=a.b\""));
}

#[test]
fn html_inner_is_trimmed() {
    // a dictionary token resolves to "" and leaves a dangling space that must be trimmed
    let b = raw_blob(&[(b'p', &[b"word", b"ok", b"more"])]);
    assert_eq!(
        decode_blob_to_html_rendered(&b, ""),
        "<div><strong>[p]</strong><br><span>ok</span></div>\n"
    );
}

#[test]
fn html_multiple_sections() {
    let b = raw_blob(&[(b'1', &[b"T"]), (b'p', &[b"b"]), (b'm', &[b"x"])]);
    let out = decode_blob_to_html_rendered(&b, "");
    assert_eq!(out.lines().count(), 3);
    assert!(out.starts_with("<div><strong>[h1]</strong>"));
    assert!(out.ends_with("<i>x</i></div>\n"));
}

/// SECURITY: in html mode a_href is emitted verbatim into `href="..."`.
/// Short plain tokens can only contain [A-Za-z0-9.,-!?'] so quotes/brackets can't be
/// injected today, but a single quote survives and `javascript` style payloads are
/// not filtered. This test pins the current escaping guarantee.
#[test]
fn html_href_never_contains_double_quote_or_angle_brackets() {
    for tok in [&b"\"x"[..], b"<a", b">", b"'"] {
        let b = raw_blob(&[(b'h', &[tok])]);
        let out = decode_blob_to_html_rendered(&b, "");
        let inner = out
            .split("href=\"")
            .nth(1)
            .map(|s| s.split('"').next().unwrap_or(""))
            .unwrap_or("");
        assert!(!inner.contains('<') && !inner.contains('>'), "{:?}", out);
    }
}

/* ───────────────────────────── DecodeMode / dictionary ───────────────────────────── */

#[test]
fn decode_mode_variants_construct() {
    let a = DecodeMode::Text;
    let b = DecodeMode::Html { proxy_prefix: "/p" };
    assert!(matches!(a, DecodeMode::Text));
    assert!(matches!(b, DecodeMode::Html { proxy_prefix: "/p" }));
}

#[test]
fn resolved_token_variants_construct() {
    let r = ResolvedToken::Raw("x");
    let c = ResolvedToken::Cached(std::sync::Arc::new("y".to_string()));
    assert!(matches!(r, ResolvedToken::Raw("x")));
    assert!(matches!(c, ResolvedToken::Cached(ref s) if s.as_str() == "y"));
}

#[test]
fn small_dictionary_ids_are_hits_with_empty_word() {
    for id in [0usize, 1, 100, 255] {
        let (w, hit) = search_word_by_id(id);
        assert!(hit, "{}", id);
        assert_eq!(w.as_str(), "");
    }
}

#[test]
fn unknown_dictionary_ids_are_misses() {
    for id in [256usize, 1000, 1 << 32, usize::MAX] {
        let (w, hit) = search_word_by_id(id);
        assert!(!hit, "{}", id);
        assert_eq!(w.as_str(), "");
    }
}

/// The dictionary cache is never populated anywhere in the code base, so every word
/// longer than 3 bytes is lost when decoding. A real document must keep its words.
#[test]
fn long_words_survive_decoding() {
    let b = raw_blob(&[(b'p', &[b"search"])]);
    assert!(
        decode_blob_to_text(&b).contains("search"),
        "dictionary words are dropped: {:?}",
        decode_blob_to_text(&b)
    );
}
