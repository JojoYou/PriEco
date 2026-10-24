//! Tests for src/web/functions/decentralized.rs (offline parts: bloom filter, tickets,
//! secrets, node profile, wire structs)

use std::{collections::HashSet, fs, str::FromStr};

use iroh::{EndpointAddr, SecretKey};
use iroh_gossip::TopicId;
use prieco_core::{PRIECO_CONFIG, url_to_id};

use crate::support::init;
use crate::web::functions::decentralized::{
    FedQuery, KeywordBloom, NodeProfile, Ticket, build_node_profile, get_iroh_secret,
};

/* ───────────────────────────── KeywordBloom ───────────────────────────── */

#[test]
fn bloom_new_is_512_zero_bytes() {
    let b = KeywordBloom::new();
    assert_eq!(b.bits.len(), 512);
    assert!(b.bits.iter().all(|x| *x == 0));
}

#[test]
fn bloom_empty_contains_nothing() {
    let b = KeywordBloom::new();
    for h in [0u64, 1, 42, u64::MAX, url_to_id("rust")] {
        assert!(!b.contains(h));
    }
}

#[test]
fn bloom_contains_inserted_values() {
    let mut b = KeywordBloom::new();
    let words = ["rust", "search", "privacy", "engine", "ž", ""];
    for w in words {
        b.insert(url_to_id(w));
    }
    for w in words {
        assert!(b.contains(url_to_id(w)), "{:?}", w);
    }
}

#[test]
fn bloom_sets_at_most_three_bits_per_insert() {
    let mut b = KeywordBloom::new();
    b.insert(url_to_id("rust"));
    let ones: u32 = b.bits.iter().map(|x| x.count_ones()).sum();
    assert!((1..=3).contains(&ones), "{}", ones);
}

#[test]
fn bloom_zero_and_max_hashes_do_not_panic() {
    let mut b = KeywordBloom::new();
    b.insert(0);
    b.insert(u64::MAX);
    assert!(b.contains(0));
    assert!(b.contains(u64::MAX));
    // u64::MAX uses index 4095 -> last bit of the last byte
    assert_eq!(b.bits[511] & 0x80, 0x80);
    assert_eq!(b.bits[0] & 1, 1);
}

#[test]
fn bloom_insert_is_idempotent() {
    let mut a = KeywordBloom::new();
    a.insert(12345);
    let snapshot = a.bits.clone();
    a.insert(12345);
    assert_eq!(a.bits, snapshot);
}

#[test]
fn bloom_only_uses_low_36_bits_of_the_hash() {
    // Documents the design: bits above 36 never influence the filter
    let mut b = KeywordBloom::new();
    b.insert(0x0000_000A_BCDE_F123);
    assert!(b.contains(0xFFFF_FFFA_BCDE_F123));
}

#[test]
fn bloom_false_positive_rate_is_reasonable_for_small_sets() {
    let mut b = KeywordBloom::new();
    for i in 0..100 {
        b.insert(url_to_id(&format!("word{}", i)));
    }
    let fp = (0..10_000)
        .filter(|i| b.contains(url_to_id(&format!("other{}", i))))
        .count();
    assert!(fp < 500, "false positive rate {}/10000", fp);
}

#[test]
fn bloom_full_filter_contains_everything() {
    let mut b = KeywordBloom::new();
    b.bits = vec![0xFF; 512];
    assert!(b.contains(987654321));
}

#[test]
fn bloom_serde_roundtrip() {
    let mut b = KeywordBloom::new();
    b.insert(777);
    let back: KeywordBloom = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
    assert_eq!(back.bits, b.bits);
    assert!(back.contains(777));
}

/// A malformed profile from the network (short bit vector) must not crash the node
#[test]
fn bloom_from_peer_with_short_bits_does_not_panic() {
    let b: KeywordBloom = serde_json::from_str(r#"{"bits":[1,2,3]}"#).unwrap();
    let r = std::panic::catch_unwind(|| b.contains(u64::MAX));
    assert!(r.is_ok(), "peer controlled bloom size caused an out of bounds panic");
}

/* ───────────────────────────── Ticket ───────────────────────────── */

fn ticket(seed: u8, endpoints: usize) -> Ticket {
    Ticket {
        topic: TopicId::from_bytes([seed; 32]),
        endpoints: (0..endpoints)
            .map(|_| EndpointAddr::new(SecretKey::generate().public()))
            .collect(),
    }
}

#[test]
fn ticket_display_is_lowercase_base32() {
    let s = ticket(1, 0).to_string();
    assert!(!s.is_empty());
    assert!(s.chars().all(|c| c.is_ascii_lowercase() || ('2'..='7').contains(&c)), "{}", s);
    assert!(!s.contains('='), "no padding");
}

#[test]
fn ticket_roundtrip_without_endpoints() {
    let t = ticket(7, 0);
    let back = Ticket::from_str(&t.to_string()).unwrap();
    assert_eq!(back.to_string(), t.to_string());
    assert!(back.endpoints.is_empty());
}

#[test]
fn ticket_roundtrip_with_endpoints() {
    let t = ticket(9, 3);
    let back: Ticket = t.to_string().parse().unwrap();
    assert_eq!(back.endpoints, t.endpoints);
    assert_eq!(back.to_bytes(), t.to_bytes());
}

#[test]
fn ticket_parse_accepts_uppercase() {
    let t = ticket(3, 1);
    let upper = t.to_string().to_uppercase();
    assert!(Ticket::from_str(&upper).is_ok());
}

#[test]
fn ticket_different_topics_differ() {
    assert_ne!(ticket(1, 0).to_string(), ticket(2, 0).to_string());
}

#[test]
fn ticket_parse_rejects_garbage() {
    for s in ["", "!!!!", "abc1", "0189", "aaaaaaaa"] {
        assert!(Ticket::from_str(s).is_err(), "{:?}", s);
    }
}

#[test]
fn ticket_parse_rejects_valid_base32_with_invalid_json() {
    let mut s = data_encoding::BASE32_NOPAD.encode(b"{\"not\":\"a ticket\"}");
    s.make_ascii_lowercase();
    assert!(Ticket::from_str(&s).is_err());
}

#[test]
fn ticket_to_bytes_is_json() {
    let v: serde_json::Value = serde_json::from_slice(&ticket(5, 1).to_bytes()).unwrap();
    assert!(v.get("topic").is_some());
    assert_eq!(v["endpoints"].as_array().unwrap().len(), 1);
}

/* ───────────────────────────── wire structs ───────────────────────────── */

#[test]
fn fed_query_serde_roundtrip() {
    let q = FedQuery {
        query: "rust \"quoted\" ž".into(),
        lang: "sk".into(),
        loc: "sk".into(),
        depth: 255,
        embed: vec![0.5, -1.0, f32::MIN_POSITIVE],
    };
    let back: FedQuery = serde_json::from_slice(&serde_json::to_vec(&q).unwrap()).unwrap();
    assert_eq!(back.query, q.query);
    assert_eq!(back.depth, 255);
    assert_eq!(back.embed, q.embed);
}

#[test]
fn fed_query_rejects_depth_overflow() {
    let r = serde_json::from_str::<FedQuery>(
        r#"{"query":"q","lang":"en","loc":"us","depth":256,"embed":[]}"#,
    );
    assert!(r.is_err());
}

#[test]
fn node_profile_serde_roundtrip() {
    let mut kw = KeywordBloom::new();
    kw.insert(99);
    let p = NodeProfile {
        node_id: [4; 32],
        top_centroids: vec![1, 5, 9],
        keyword: kw,
    };
    let back: NodeProfile = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    assert_eq!(back.node_id, [4; 32]);
    assert_eq!(back.top_centroids, vec![1, 5, 9]);
    assert!(back.keyword.contains(99));
}

#[test]
fn node_profile_rejects_wrong_node_id_length() {
    let r = serde_json::from_str::<NodeProfile>(
        r#"{"node_id":[1,2,3],"top_centroids":[],"keyword":{"bits":[]}}"#,
    );
    assert!(r.is_err());
}

/* ───────────────────────────── secrets ───────────────────────────── */

#[test]
fn iroh_secret_is_generated_persisted_reused_and_repaired() {
    init();
    let path = "config/iroh/iroh_secret.bin";
    let _ = fs::remove_file(path);

    let k1 = get_iroh_secret();
    assert_eq!(fs::read(path).unwrap(), k1.to_bytes().to_vec());

    let k2 = get_iroh_secret();
    assert_eq!(k2.to_bytes(), k1.to_bytes(), "secret must be stable across restarts");
    assert_eq!(k2.public(), k1.public());

    fs::write(path, [1u8; 10]).unwrap();
    let k3 = get_iroh_secret();
    assert_ne!(k3.to_bytes(), k1.to_bytes());
    assert_eq!(fs::read(path).unwrap(), k3.to_bytes().to_vec());

    fs::write(path, [7u8; 32]).unwrap();
    let k4 = get_iroh_secret();
    assert_eq!(k4.to_bytes(), [7u8; 32]);
}

/* ───────────────────────────── node profile ───────────────────────────── */

#[test]
fn node_profile_picks_20_largest_buckets_sorted_by_id() {
    init();
    let dir = &PRIECO_CONFIG.vector_path;
    fs::create_dir_all(dir).unwrap();
    // 25 buckets, bucket i has size i+1 -> the 20 largest are ids 5..=24
    for i in 0..25usize {
        fs::write(format!("{}/bucket_{:06}.bin.zst", dir, i), vec![0u8; i + 1]).unwrap();
    }
    // noise that must be ignored
    fs::write(format!("{}/staging_000001.bin", dir), vec![0u8; 999]).unwrap();
    fs::write(format!("{}/bucket_abc.bin.zst", dir), vec![0u8; 999]).unwrap();
    fs::write(format!("{}/bucket_000099.bin", dir), vec![0u8; 999]).unwrap();

    let p = build_node_profile([9; 32]);
    assert_eq!(p.node_id, [9; 32]);
    assert_eq!(p.top_centroids, (5..25).collect::<Vec<usize>>());
    let unique: HashSet<usize> = p.top_centroids.iter().copied().collect();
    assert_eq!(unique.len(), 20);
    // empty full text index -> empty keyword filter
    assert_eq!(p.keyword.bits.len(), 512);
    assert!(p.keyword.bits.iter().all(|b| *b == 0));
}
