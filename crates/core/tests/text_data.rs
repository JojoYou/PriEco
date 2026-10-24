//! Tests for embedded text data and text processing globals:
//! tokenizer, synonyms, intent matchers, entity pipeline, domains, spell checker, model files.

use std::sync::Arc;

use prieco_core::{
    BGE_TOKENIZER, CENTROPOIDS_BIN, EntityType, LOCAL_NO_EXPAND, MATCHERS, Multilingual,
    QU_PIPELINE, QueryIntent, SPELL_CHECKER, TOP_DOMAINS, VECTOR_DIM, VECTOR_EMBEDDING_TOKENIZER,
    get_store,
};
use symspell::Verbosity;
use tantivy::tokenizer::{TokenStream, Tokenizer as _};

/* ───────────────────────────── Multilingual tokenizer ───────────────────────────── */

struct Tok {
    text: String,
    from: usize,
    to: usize,
    pos: usize,
}

fn tokens(text: &str) -> Vec<Tok> {
    let mut t = Multilingual::new();
    let mut stream = t.token_stream(text);
    let mut out = Vec::new();
    while stream.advance() {
        let tok = stream.token();
        out.push(Tok {
            text: tok.text.clone(),
            from: tok.offset_from,
            to: tok.offset_to,
            pos: tok.position,
        });
    }
    out
}

#[test]
fn tokenizer_empty_and_whitespace_produce_no_tokens() {
    assert!(tokens("").is_empty());
    assert!(tokens("   \t\n ").is_empty());
}

#[test]
fn tokenizer_drops_punctuation_and_lowercases() {
    let t = tokens("Hello, World!");
    let words: Vec<&str> = t.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(words, vec!["hello", "world"]);
}

#[test]
fn tokenizer_byte_offsets_point_into_original_text() {
    let text = "Hello, World!";
    let t = tokens(text);
    assert_eq!((t[0].from, t[0].to), (0, 5));
    assert_eq!((t[1].from, t[1].to), (7, 12));
    assert_eq!(&text[t[1].from..t[1].to], "World");
}

#[test]
fn tokenizer_positions_are_sequential_over_words_only() {
    let t = tokens("one, two; three... four");
    let pos: Vec<usize> = t.iter().map(|t| t.pos).collect();
    assert_eq!(pos, vec![0, 1, 2, 3]);
}

#[test]
fn tokenizer_offsets_are_valid_for_multibyte_text() {
    for text in ["Žltý kôň úpel ďábelské ódy", "日本語のテキストです", "مرحبا بالعالم", "🐴 horse 🐴"] {
        let t = tokens(text);
        assert!(!t.is_empty(), "{}", text);
        for tok in &t {
            assert!(tok.from < tok.to);
            assert!(tok.to <= text.len());
            assert!(text.is_char_boundary(tok.from));
            assert!(text.is_char_boundary(tok.to));
        }
    }
}

#[test]
fn tokenizer_segments_chinese_without_spaces() {
    let t = tokens("我喜欢搜索引擎");
    assert!(t.len() >= 2, "expected word segmentation, got {} tokens", t.len());
}

#[test]
fn tokenizer_clones_share_behaviour() {
    let a = Multilingual::new();
    let mut b = a.clone();
    let mut s = b.token_stream("Clone works");
    let mut n = 0;
    while s.advance() {
        n += 1;
    }
    assert_eq!(n, 2);
}

/* ───────────────────────────── Synonyms ───────────────────────────── */

const SYNONYM_LANGS: &[&str] = &[
    "ar", "bg", "ca", "da", "en", "et", "fi", "he", "hr", "hu", "is", "lt", "lv", "nb", "nn", "no",
    "ro", "sk", "sl", "sr", "sv",
];

#[test]
fn synonym_store_loads_for_every_embedded_language() {
    for lang in SYNONYM_LANGS {
        assert!(get_store(lang).is_some(), "synonyms for {} failed to decode", lang);
    }
}

#[test]
fn synonym_store_none_for_unknown_language() {
    for lang in ["", "xx", "EN", "english", "zh"] {
        assert!(get_store(lang).is_none(), "{:?}", lang);
    }
}

#[test]
fn synonym_store_missing_for_languages_without_data() {
    // These languages are supported by PriEco but have no synonym files embedded
    for lang in ["de", "fr", "es", "it", "pt", "pl", "nl", "ru", "cs", "tr"] {
        assert!(get_store(lang).is_none(), "{}", lang);
    }
}

#[test]
fn synonym_store_is_cached() {
    let a = get_store("en").unwrap();
    let b = get_store("en").unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn synonym_store_cache_is_safe_across_threads() {
    let handles: Vec<_> = (0..8)
        .map(|i| std::thread::spawn(move || get_store(SYNONYM_LANGS[i % SYNONYM_LANGS.len()]).is_some()))
        .collect();
    for h in handles {
        assert!(h.join().unwrap());
    }
}

#[test]
fn synonym_lookup_unknown_term_is_none() {
    let store = get_store("en").unwrap();
    assert!(store.lookup("zzqqxxnotaword").is_none());
    assert!(store.lookup("").is_none());
}

#[test]
fn synonym_lookup_common_english_words_have_synonyms() {
    let store = get_store("en").unwrap();
    let found = ["car", "big", "happy", "house", "fast", "good", "small", "buy"]
        .iter()
        .filter(|w| store.lookup(w).map_or(false, |s| !s.is_empty()))
        .count();
    assert!(found > 0, "no synonyms for any common English word");
}

#[test]
fn synonym_lookup_never_returns_empty_strings() {
    let store = get_store("en").unwrap();
    for w in ["car", "big", "happy", "house", "fast", "good", "small", "buy", "run", "go"] {
        if let Some(syns) = store.lookup(w) {
            for s in syns {
                assert!(!s.is_empty(), "empty synonym for {}", w);
            }
        }
    }
}

#[test]
fn local_no_expand_contains_location_words() {
    for w in ["near", "nearby", "closest", "me", "here", "city"] {
        assert!(LOCAL_NO_EXPAND.contains(w), "{}", w);
    }
    assert!(LOCAL_NO_EXPAND.contains("خريطة"));
    assert!(!LOCAL_NO_EXPAND.contains("pizza"));
    assert!(!LOCAL_NO_EXPAND.contains("NEAR"), "set is lowercase only");
}

/* ───────────────────────────── Intent matchers ───────────────────────────── */

const INTENT_LANGS: &[&str] = &[
    "ar", "bg", "ca", "cs", "da", "de", "el", "en", "es", "et", "fi", "fr", "he", "hr", "hu", "id",
    "is", "it", "ja", "ko", "lt", "lv", "nl", "no", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sv",
    "tr",
];

#[test]
fn matchers_exist_for_every_language_in_intent_json() {
    for lang in INTENT_LANGS {
        assert!(MATCHERS.contains_key(*lang), "{}", lang);
    }
    assert_eq!(MATCHERS.len(), INTENT_LANGS.len());
}

#[test]
fn matchers_intent_map_matches_pattern_count() {
    for (lang, m) in MATCHERS.iter() {
        assert_eq!(m.intent_map.len(), m.automaton.patterns_len(), "{}", lang);
        assert!(m.automaton.patterns_len() > 0, "{}", lang);
    }
}

fn en_intent(q: &str) -> Option<QueryIntent> {
    let m = MATCHERS.get("en").unwrap();
    m.automaton
        .find(q)
        .map(|mat| m.intent_map[mat.pattern().as_usize()])
}

#[test]
fn matchers_english_keywords_map_to_their_intent() {
    assert_eq!(en_intent("how to cook rice"), Some(QueryIntent::Informational));
    assert_eq!(en_intent("buy shoes"), Some(QueryIntent::Transactional));
    assert_eq!(en_intent("cheap flights"), Some(QueryIntent::CommercialInvestigation));
    assert_eq!(en_intent("pizza near me"), Some(QueryIntent::Local));
    assert_eq!(en_intent("gmail login"), Some(QueryIntent::Navigational));
}

#[test]
fn matchers_are_ascii_case_insensitive() {
    assert_eq!(en_intent("HOW TO COOK"), Some(QueryIntent::Informational));
    assert_eq!(en_intent("Buy Now"), Some(QueryIntent::Transactional));
}

#[test]
fn matchers_no_keyword_no_match() {
    assert_eq!(en_intent("zzqqxx"), None);
    assert_eq!(en_intent(""), None);
}

/* ───────────────────────────── Entity pipeline ───────────────────────────── */

#[test]
fn entity_pipeline_metadata_matches_patterns() {
    assert_eq!(
        QU_PIPELINE.metadata_map.len(),
        QU_PIPELINE.automaton.patterns_len()
    );
    assert!(QU_PIPELINE.metadata_map.len() > 100_000);
}

#[test]
fn entity_pipeline_no_pattern_shorter_than_two_bytes() {
    for (_, text) in QU_PIPELINE.metadata_map.iter() {
        assert!(text.trim().len() >= 2, "pattern {:?} too short", text);
    }
}

#[test]
fn entity_tags_empty_query() {
    assert!(QU_PIPELINE.get_tags("").is_empty());
    assert!(QU_PIPELINE.get_tags("     ").is_empty());
}

#[test]
fn entity_tags_detect_business() {
    let tags = QU_PIPELINE.get_tags("abn amro bank n.v. login");
    let biz = tags
        .iter()
        .find(|t| t.entity_type == EntityType::Business)
        .expect("business not detected");
    assert_eq!(biz.matched_text, "abn amro bank n.v.");
    assert_eq!(biz.range, 0..18);
}

#[test]
fn entity_tags_are_case_insensitive() {
    let tags = QU_PIPELINE.get_tags("ABN AMRO BANK N.V.");
    assert!(tags.iter().any(|t| t.entity_type == EntityType::Business));
}

#[test]
fn entity_tags_detect_multi_word_place_longest_match() {
    let tags = QU_PIPELINE.get_tags("hotels trnava pri laborci");
    let place = tags
        .iter()
        .find(|t| t.entity_type == EntityType::Place)
        .expect("place not detected");
    assert_eq!(place.matched_text, "trnava pri laborci");
    assert_eq!(place.range, 7..25);
}

#[test]
fn entity_tags_respect_word_boundaries() {
    // "abn amro bank n.v." glued to other letters must not match
    let tags = QU_PIPELINE.get_tags("xabn amro bank n.v.");
    assert!(!tags.iter().any(|t| t.matched_text == "abn amro bank n.v."));
}

#[test]
fn entity_tag_ranges_are_valid_and_match_query_text() {
    for q in [
        "hotels trnava pri laborci",
        "abn amro bank n.v. login",
        "einstein relativity paris london",
        "Žltý kôň paris",
    ] {
        let lower = q.to_lowercase();
        for t in QU_PIPELINE.get_tags(q) {
            assert!(t.range.end <= lower.len());
            assert!(lower.is_char_boundary(t.range.start));
            assert!(lower.is_char_boundary(t.range.end));
            assert!(lower[t.range.clone()].eq_ignore_ascii_case(&t.matched_text));
        }
    }
}

#[test]
fn entity_tags_are_non_overlapping_and_ordered() {
    let tags = QU_PIPELINE.get_tags("einstein paris london shakespeare abn amro bank n.v.");
    for w in tags.windows(2) {
        assert!(w[0].range.end <= w[1].range.start);
    }
}

#[test]
fn entity_places_registry_contains_capitalised_keys() {
    let coords = QU_PIPELINE
        .archived_places
        .places
        .get("Trnava pri Laborci")
        .expect("place missing in registry");
    assert!(!coords.is_empty());
    assert!(coords.iter().any(|c| c.country.as_str().eq_ignore_ascii_case("sk")));
}

/// BUG (data): the NAME list contains plain English words ("the", "how", "to",
/// "show", "car", "cat", "red", "tomorrow"...). Every query containing them is tagged
/// as a person, which flips intents and blocks spell checking.
#[test]
fn common_words_are_not_person_names() {
    for w in ["the", "how", "to", "show", "car", "cat", "red", "tomorrow", "bake"] {
        let tags = QU_PIPELINE.get_tags(w);
        assert!(
            !tags.iter().any(|t| t.entity_type == EntityType::PersonName),
            "{:?} tagged as a person name",
            w
        );
    }
}

/// BUG (data): big cities are also in the NAME list; with LeftmostLongest and equal
/// length the NAME pattern (added first) wins, so "paris"/"london" are never Places.
#[test]
fn big_cities_are_tagged_as_places() {
    for city in ["paris", "london", "berlin"] {
        let tags = QU_PIPELINE.get_tags(city);
        assert!(
            tags.iter().any(|t| t.entity_type == EntityType::Place),
            "{} not tagged as a place: {:?}",
            city,
            tags.iter().map(|t| t.entity_type).collect::<Vec<_>>()
        );
    }
}

/* ───────────────────────────── Top domains ───────────────────────────── */

#[test]
fn top_domains_loaded() {
    assert!(TOP_DOMAINS.len() > 500_000, "{}", TOP_DOMAINS.len());
    assert!(TOP_DOMAINS.contains("google.com"));
    assert!(TOP_DOMAINS.contains("microsoft.com"));
}

#[test]
fn top_domains_have_no_empty_or_padded_entries() {
    assert!(!TOP_DOMAINS.contains(""));
    for d in TOP_DOMAINS.iter().take(10_000) {
        assert_eq!(d.trim(), *d, "{:?}", d);
        assert!(!d.contains('/'), "{:?}", d);
    }
}

/* ───────────────────────────── Spell checker ───────────────────────────── */

#[test]
fn spell_checker_knows_correct_words() {
    let s = SPELL_CHECKER.lookup("hello", Verbosity::Top, 0);
    assert_eq!(s.first().map(|s| s.term.as_str()), Some("hello"));
}

#[test]
fn spell_checker_corrects_common_typos() {
    for (typo, fix) in [
        ("recieve", "receive"),
        ("definately", "definitely"),
        ("goverment", "government"),
        ("beleive", "believe"),
    ] {
        let s = SPELL_CHECKER.lookup(typo, Verbosity::Top, 2);
        assert_eq!(s.first().map(|s| s.term.as_str()), Some(fix), "{}", typo);
    }
}

#[test]
fn spell_checker_unknown_word_with_zero_distance_has_no_suggestion() {
    assert!(SPELL_CHECKER.lookup("zzqqxxyy", Verbosity::Top, 0).is_empty());
}

/* ───────────────────────────── Embedded model files ───────────────────────────── */

#[test]
fn embedding_tokenizer_json_parses_and_encodes() {
    let tok = tokenizers::Tokenizer::from_bytes(VECTOR_EMBEDDING_TOKENIZER).unwrap();
    let enc = tok.encode("hello world", true).unwrap();
    assert!(!enc.get_ids().is_empty());
    assert_eq!(enc.get_ids().len(), enc.get_attention_mask().len());
}

#[test]
fn reranker_tokenizer_json_parses_and_encodes_pairs() {
    let tok = tokenizers::Tokenizer::from_bytes(BGE_TOKENIZER).unwrap();
    let enc = tok.encode(("query", "passage text"), true).unwrap();
    assert!(enc.get_ids().len() > 3);
}

#[test]
fn centroids_file_header_matches_vector_dim() {
    assert!(CENTROPOIDS_BIN.len() > 8);
    let n = u32::from_le_bytes(CENTROPOIDS_BIN[0..4].try_into().unwrap()) as usize;
    let dims = u32::from_le_bytes(CENTROPOIDS_BIN[4..8].try_into().unwrap()) as usize;
    assert_eq!(dims, VECTOR_DIM);
    assert!(n > 0);
    assert_eq!(CENTROPOIDS_BIN.len(), 8 + n * dims * 4);
}

#[test]
fn centroids_contain_only_finite_values() {
    for chunk in CENTROPOIDS_BIN[8..].chunks_exact(4).step_by(9973) {
        let v = f32::from_le_bytes(chunk.try_into().unwrap());
        assert!(v.is_finite());
    }
}

#[cfg(not(feature = "cuda"))]
mod no_cuda {
    use prieco_core::CentroidIndex;

    #[test]
    fn centroid_index_without_cuda_constructs() {
        assert!(CentroidIndex::new(&[]).is_ok());
    }

    #[test]
    fn centroid_index_without_cuda_search_is_error() {
        let idx = CentroidIndex::new(&[]).unwrap();
        let err = idx.search(&[0.0; 384], 4, 10).unwrap_err();
        assert!(err.to_string().contains("CUDA"));
    }

    #[test]
    fn centroid_index_without_cuda_assigns_nothing() {
        let idx = CentroidIndex::new(&[]).unwrap();
        let out = idx.assign_batch(&[vec![0.0; 384], vec![1.0; 384]]).unwrap();
        assert!(out.is_empty(), "non cuda build cannot assign buckets");
    }
}
