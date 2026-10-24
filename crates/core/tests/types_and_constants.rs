//! Tests for constants, plain data structures and their (de)serialization in prieco_core

use std::collections::{HashMap, HashSet};

use prieco_core::{
    BLOB_IMPORT_DIR, Bang, CSS_VERSION, EntityRegistry, EntityType, FINAL_SCORES, GeoCoords,
    ID_MAP_FILE, ID_SIZE, INSERTER_IMPORT_DIR, ImgResult, JS_VERSION, PriEcoConfig, QueryIntent,
    RECORD_SIZE, SEARCHED_K1, SEARCHED_K2, SearchResult, TANTIVY_HEAP_SIZE, TaggedEntity,
    UserAgent, VECTOR_DIM, WebDocument, WebScrollResult, colors, icons,
};
use rocket::{get, http::Header, local::blocking::Client, routes};
use serde_json::{Value, json};

/* ───────────────────────────── constants ───────────────────────────── */

#[test]
fn inserter_record_layout_constants() {
    assert_eq!(ID_SIZE, 8);
    assert_eq!(ID_SIZE, std::mem::size_of::<u64>());
    assert_eq!(VECTOR_DIM, 384);
    assert_eq!(RECORD_SIZE, ID_SIZE + VECTOR_DIM * std::mem::size_of::<f32>());
    assert_eq!(RECORD_SIZE, 1544);
}

#[test]
fn pagerank_paths() {
    assert_eq!(ID_MAP_FILE, "pagerank/id_map.bin");
    assert_eq!(FINAL_SCORES, "pagerank/pageranks.bin");
    assert_ne!(ID_MAP_FILE, FINAL_SCORES);
}

#[test]
fn searched_hash_keys_are_fixed_and_distinct() {
    assert_eq!(SEARCHED_K1, 0xC70F6907A1C9566B);
    assert_eq!(SEARCHED_K2, 0x85E4C66FC71D33EF);
    assert_ne!(SEARCHED_K1, SEARCHED_K2);
}

#[test]
fn import_dirs_are_absolute() {
    assert!(BLOB_IMPORT_DIR.starts_with('/'));
    assert!(BLOB_IMPORT_DIR.ends_with('/'));
    assert!(INSERTER_IMPORT_DIR.starts_with('/'));
}

#[test]
fn tantivy_heap_is_above_tantivy_minimum() {
    // Tantivy refuses budgets below 15MB per indexing thread
    assert!(TANTIVY_HEAP_SIZE >= 15_000_000);
    // ... and above 4GB per thread
    assert!(TANTIVY_HEAP_SIZE < 4_000_000_000);
}

#[test]
fn asset_versions_are_semver_like() {
    for v in [CSS_VERSION, JS_VERSION] {
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3, "{}", v);
        for p in parts {
            p.parse::<u32>().unwrap();
        }
    }
}

#[test]
fn terminal_colors_are_ansi_escape_codes() {
    let all = [
        colors::RESET,
        colors::BLACK,
        colors::RED,
        colors::GREEN,
        colors::YELLOW,
        colors::BLUE,
        colors::MAGENTA,
        colors::CYAN,
        colors::WHITE,
    ];
    for c in all {
        assert!(c.starts_with("\x1b["), "{:?}", c);
        assert!(c.ends_with('m'), "{:?}", c);
    }
    assert_eq!(colors::RESET, "\x1b[0m");
    assert_eq!(colors::RED, "\x1b[31m");
    assert_eq!(colors::GREEN, "\x1b[32m");
    let unique: HashSet<&str> = all.iter().copied().collect();
    assert_eq!(unique.len(), all.len());
}

#[test]
fn icons_are_non_empty_and_distinct() {
    let all = [
        icons::BLOB,
        icons::DB_INSERT,
        icons::PAGERANK_ICON,
        icons::MINI_CRAWLER_ICON,
        icons::INDEX_UPDATER,
    ];
    for i in all {
        assert!(!i.is_empty());
    }
    let unique: HashSet<&str> = all.iter().copied().collect();
    assert_eq!(unique.len(), all.len());
}

/* ───────────────────────────── WebDocument ───────────────────────────── */

fn minimal_doc_json() -> Value {
    json!({
        "url": "https://example.com/",
        "title": "Example",
        "description": "Desc",
        "content": "Content",
        "favicon": "",
        "image": "",
        "keywords": "a,b",
        "safe_s": true,
        "lang": "en",
        "loc": "us",
        "html": "123",
        "impressions": 1,
        "clicks": 2,
        "confidence": 50.5,
        "effort": 1.25,
        "qna": 3.0,
        "sts": 0.5,
        "load": 0.33,
        "date": 260101
    })
}

#[test]
fn web_document_minimal_json_uses_defaults() {
    let doc: WebDocument = serde_json::from_value(minimal_doc_json()).unwrap();
    assert_eq!(doc.url, "https://example.com/");
    assert_eq!(doc.intent, 5, "missing intent must default to Unknown (5)");
    assert!(!doc.is_mobile);
    assert!(!doc.has_500_words);
    assert_eq!(doc.search_score, 0.0);
    assert_eq!(doc.source, "");
    assert_eq!(doc.date, 260101);
    assert_eq!(doc.impressions, 1);
    assert_eq!(doc.clicks, 2);
}

#[test]
fn web_document_roundtrip_preserves_all_fields() {
    let mut v = minimal_doc_json();
    v["intent"] = json!(3);
    v["is_mobile"] = json!(true);
    v["has_500_words"] = json!(true);
    v["search_score"] = json!(1.5);
    v["source"] = json!("FTS + IVF");
    let doc: WebDocument = serde_json::from_value(v).unwrap();

    let back: WebDocument = serde_json::from_slice(&serde_json::to_vec(&doc).unwrap()).unwrap();
    assert_eq!(back.url, doc.url);
    assert_eq!(back.title, doc.title);
    assert_eq!(back.description, doc.description);
    assert_eq!(back.content, doc.content);
    assert_eq!(back.keywords, doc.keywords);
    assert_eq!(back.safe_s, doc.safe_s);
    assert_eq!(back.lang, doc.lang);
    assert_eq!(back.loc, doc.loc);
    assert_eq!(back.html, doc.html);
    assert_eq!(back.confidence, doc.confidence);
    assert_eq!(back.effort, doc.effort);
    assert_eq!(back.qna, doc.qna);
    assert_eq!(back.sts, doc.sts);
    assert_eq!(back.load, doc.load);
    assert_eq!(back.date, doc.date);
    assert_eq!(back.intent, 3);
    assert!(back.is_mobile);
    assert!(back.has_500_words);
    assert_eq!(back.search_score, 1.5);
    assert_eq!(back.source, "FTS + IVF");
}

#[test]
fn web_document_missing_required_field_is_error() {
    for field in [
        "url",
        "title",
        "description",
        "content",
        "favicon",
        "image",
        "keywords",
        "safe_s",
        "lang",
        "loc",
        "html",
        "impressions",
        "clicks",
        "confidence",
        "effort",
        "qna",
        "sts",
        "load",
        "date",
    ] {
        let mut v = minimal_doc_json();
        v.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<WebDocument>(v).is_err(),
            "field {} should be required",
            field
        );
    }
}

#[test]
fn web_document_rejects_wrong_types() {
    let mut v = minimal_doc_json();
    v["impressions"] = json!(-1);
    assert!(serde_json::from_value::<WebDocument>(v).is_err());

    let mut v = minimal_doc_json();
    v["intent"] = json!(300);
    assert!(serde_json::from_value::<WebDocument>(v).is_err());

    let mut v = minimal_doc_json();
    v["safe_s"] = json!("true");
    assert!(serde_json::from_value::<WebDocument>(v).is_err());
}

#[test]
fn web_document_ignores_unknown_fields() {
    let mut v = minimal_doc_json();
    v["something_new"] = json!({"x": 1});
    assert!(serde_json::from_value::<WebDocument>(v).is_ok());
}

#[test]
fn web_document_accepts_integer_floats_and_negative_date() {
    let mut v = minimal_doc_json();
    v["load"] = json!(2);
    v["confidence"] = json!(-10);
    v["date"] = json!(-5);
    let doc: WebDocument = serde_json::from_value(v).unwrap();
    assert_eq!(doc.load, 2.0);
    assert_eq!(doc.confidence, -10.0);
    assert_eq!(doc.date, -5);
}

#[test]
fn web_document_unicode_content_survives_roundtrip() {
    let mut v = minimal_doc_json();
    v["title"] = json!("Žltý kôň 🐴 \"quoted\" \\ back");
    let doc: WebDocument = serde_json::from_value(v).unwrap();
    let back: WebDocument = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.title, "Žltý kôň 🐴 \"quoted\" \\ back");
}

#[test]
fn web_document_clone_is_deep() {
    let doc: WebDocument = serde_json::from_value(minimal_doc_json()).unwrap();
    let mut c = doc.clone();
    c.title.push_str(" changed");
    assert_eq!(doc.title, "Example");
}

/* ───────────────────────────── PriEcoConfig ───────────────────────────── */

fn minimal_config_json() -> Value {
    json!({
        "ip": "0.0.0.0",
        "port": 80,
        "tantivy_path": "idx/tantivy",
        "meta_path": "idx/meta",
        "vector_path": "idx/vectors",
        "blob_path": "idx/blob",
        "worker_concurrent": 1
    })
}

#[test]
fn config_enable_flags_default_to_true() {
    let c: PriEcoConfig = serde_json::from_value(minimal_config_json()).unwrap();
    assert!(c.enable_blob_storage);
    assert!(c.enable_db_inserter);
    assert!(c.enable_pagerank);
    assert!(c.enable_mini_crawler);
    assert!(c.enable_index_updater);
    assert!(c.enable_web_server);
}

#[test]
fn config_explicit_false_flags_are_respected() {
    let mut v = minimal_config_json();
    for k in [
        "enable_blob_storage",
        "enable_db_inserter",
        "enable_pagerank",
        "enable_mini_crawler",
        "enable_index_updater",
        "enable_web_server",
    ] {
        v[k] = json!(false);
    }
    let c: PriEcoConfig = serde_json::from_value(v).unwrap();
    assert!(!c.enable_blob_storage);
    assert!(!c.enable_db_inserter);
    assert!(!c.enable_pagerank);
    assert!(!c.enable_mini_crawler);
    assert!(!c.enable_index_updater);
    assert!(!c.enable_web_server);
}

#[test]
fn config_missing_required_fields_fail() {
    for field in [
        "ip",
        "port",
        "tantivy_path",
        "meta_path",
        "vector_path",
        "blob_path",
        "worker_concurrent",
    ] {
        let mut v = minimal_config_json();
        v.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<PriEcoConfig>(v).is_err(),
            "{} should be required",
            field
        );
    }
}

#[test]
fn config_roundtrip() {
    let c: PriEcoConfig = serde_json::from_value(minimal_config_json()).unwrap();
    let s = serde_json::to_string_pretty(&c).unwrap();
    let back: PriEcoConfig = serde_json::from_str(&s).unwrap();
    assert_eq!(back.ip, "0.0.0.0");
    assert_eq!(back.port, 80);
    assert_eq!(back.tantivy_path, "idx/tantivy");
    assert_eq!(back.meta_path, "idx/meta");
    assert_eq!(back.vector_path, "idx/vectors");
    assert_eq!(back.blob_path, "idx/blob");
    assert_eq!(back.worker_concurrent, 1);
}

#[test]
fn config_rejects_negative_worker_count() {
    let mut v = minimal_config_json();
    v["worker_concurrent"] = json!(-1);
    assert!(serde_json::from_value::<PriEcoConfig>(v).is_err());
}

/* ───────────────────────────── result structs ───────────────────────────── */

#[test]
fn search_result_serializes_all_fields() {
    let r = SearchResult {
        url: "u".into(),
        display_url: "d".into(),
        domain: "dom".into(),
        title: "t".into(),
        description: "desc".into(),
        image: "i".into(),
        favicon: "f".into(),
        html_id: None,
        content: "c".into(),
        reading_level: "r".into(),
        formatted_load: "l".into(),
        intent: "in".into(),
        source_engine: "s".into(),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["url"], "u");
    assert_eq!(v["display_url"], "d");
    assert_eq!(v["domain"], "dom");
    assert_eq!(v["html_id"], Value::Null);
    assert_eq!(v["source_engine"], "s");
    assert_eq!(v.as_object().unwrap().len(), 13);
}

#[test]
fn img_and_scroll_results_serialize() {
    let i = ImgResult {
        thumbnail: "t".into(),
        image: "i".into(),
        title: "ti".into(),
        site_url: "s".into(),
        site_domain: "sd".into(),
        favicon: "f".into(),
    };
    let v = serde_json::to_value(i.clone()).unwrap();
    assert_eq!(v["site_domain"], "sd");
    assert_eq!(v.as_object().unwrap().len(), 6);

    let w = WebScrollResult {
        url: "u".into(),
        domain: "d".into(),
        image: "i".into(),
        favicon: "f".into(),
        title: "t".into(),
        price: "9 EUR".into(),
    };
    let v = serde_json::to_value(&w).unwrap();
    assert_eq!(v["price"], "9 EUR");
}

#[test]
fn bang_deserializes() {
    let b: Bang = serde_json::from_str(r#"{"d":"github.com","t":"gh","u":"https://github.com/search?q={{{s}}}"}"#)
        .unwrap();
    assert_eq!(b.t, "gh");
    assert_eq!(b.d, "github.com");
    assert!(b.u.contains("{{{s}}}"));
    assert!(serde_json::from_str::<Bang>(r#"{"d":"x","t":"y"}"#).is_err());
}

/* ───────────────────────────── enums ───────────────────────────── */

#[test]
fn query_intents_are_distinct_values() {
    let all = [
        QueryIntent::Informational,
        QueryIntent::Transactional,
        QueryIntent::CommercialInvestigation,
        QueryIntent::Navigational,
        QueryIntent::Local,
        QueryIntent::Unknown,
    ];
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            assert_eq!(i == j, a == b);
        }
    }
}

#[test]
fn entity_types_are_distinct() {
    assert_ne!(EntityType::PersonName, EntityType::Business);
    assert_ne!(EntityType::Business, EntityType::Place);
    assert_ne!(EntityType::PersonName, EntityType::Place);
}

#[test]
fn tagged_entity_clone() {
    let t = TaggedEntity {
        range: 3..9,
        entity_type: EntityType::Place,
        matched_text: "london".into(),
    };
    let c = t.clone();
    assert_eq!(c.range, 3..9);
    assert_eq!(c.entity_type, EntityType::Place);
    assert_eq!(c.matched_text, "london");
}

/* ───────────────────────────── entity registry ───────────────────────────── */

fn registry() -> EntityRegistry {
    let mut places = HashMap::new();
    places.insert(
        "Bratislava".to_string(),
        vec![GeoCoords {
            country: "sk".into(),
            lon: 17.1077,
            lat: 48.1486,
        }],
    );
    places.insert(
        "Springfield".to_string(),
        vec![
            GeoCoords {
                country: "us".into(),
                lon: -89.65,
                lat: 39.78,
            },
            GeoCoords {
                country: "au".into(),
                lon: 152.9,
                lat: -27.7,
            },
        ],
    );
    EntityRegistry { places }
}

#[test]
fn entity_registry_json_roundtrip() {
    let reg = registry();
    let back: EntityRegistry =
        serde_json::from_str(&serde_json::to_string(&reg).unwrap()).unwrap();
    assert_eq!(back.places.len(), 2);
    assert_eq!(back.places["Springfield"].len(), 2);
    assert_eq!(back.places["Bratislava"][0].country, "sk");
}

#[test]
fn entity_registry_rkyv_roundtrip_and_validation() {
    let reg = registry();
    let bytes = rkyv::to_bytes::<_, 256>(&reg).unwrap();
    let archived = rkyv::check_archived_root::<EntityRegistry>(&bytes).unwrap();
    assert_eq!(archived.places.len(), 2);
    let b = archived.places.get("Bratislava").unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].country.as_str(), "sk");
    assert!((b[0].lat - 48.1486).abs() < 1e-4);
    assert!(archived.places.get("bratislava").is_none(), "keys are case sensitive");
}

#[test]
fn entity_registry_rkyv_rejects_garbage() {
    let garbage = vec![0xAAu8; 64];
    assert!(rkyv::check_archived_root::<EntityRegistry>(&garbage).is_err());
}

/* ───────────────────────────── UserAgent request guard ───────────────────────────── */

#[get("/ua")]
fn ua_route(agent: UserAgent<'_>) -> String {
    agent.0.to_string()
}

fn ua_client() -> Client {
    Client::tracked(rocket::build().mount("/", routes![ua_route])).unwrap()
}

#[test]
fn user_agent_guard_reads_header() {
    let client = ua_client();
    let res = client
        .get("/ua")
        .header(Header::new("User-Agent", "Mozilla/5.0 Test"))
        .dispatch();
    assert_eq!(res.into_string().unwrap(), "Mozilla/5.0 Test");
}

#[test]
fn user_agent_guard_defaults_to_unknown() {
    let client = ua_client();
    let res = client.get("/ua").dispatch();
    assert_eq!(res.into_string().unwrap(), "unknown");
}

#[test]
fn user_agent_guard_keeps_empty_header_value() {
    let client = ua_client();
    let res = client.get("/ua").header(Header::new("User-Agent", "")).dispatch();
    assert_eq!(res.into_string().unwrap(), "");
}
