// tests/pagerank_units.rs
//
// Unit-level tests for every public pagerank building block:
// binary readers/writers, IdMap, hashing, merge, translate, csr and iterate edge cases.
// Complements tests/pagerank.rs (which covers end to end behaviour).

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, Read, Write},
    sync::atomic::{AtomicU32, Ordering},
};

use prieco_core::{normalize_url, url_to_id};
use prieco_pagerank::{
    compute::{
        self, IdMap, read_u64_pair, read_u64_pair_zstd, zstd_reader, zstd_writer,
    },
    import::{hashing, merge, translate},
    iter::iterate,
    nodes::csr,
};

/* ───────────────────────────── helpers ───────────────────────────── */

struct Tmp(String);
static N: AtomicU32 = AtomicU32::new(0);

impl Tmp {
    fn new() -> Self {
        let p = std::env::temp_dir()
            .join(format!(
                "prieco_pr_units_{}_{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ))
            .to_string_lossy()
            .into_owned();
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
    fn p(&self, name: &str) -> String {
        format!("{}/{}", self.0, name)
    }
    fn dir(&self, name: &str) -> String {
        let d = self.p(name);
        fs::create_dir_all(&d).unwrap();
        d
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_pairs_raw(path: &str, pairs: &[(u64, u64)]) {
    let mut f = File::create(path).unwrap();
    for (a, b) in pairs {
        f.write_all(&a.to_le_bytes()).unwrap();
        f.write_all(&b.to_le_bytes()).unwrap();
    }
}

fn read_pairs_raw(path: &str) -> Vec<(u64, u64)> {
    let mut r = BufReader::new(File::open(path).unwrap());
    let mut out = Vec::new();
    while let Some(p) = read_u64_pair(&mut r).unwrap() {
        out.push(p);
    }
    out
}

fn write_pairs_zstd(path: &str, pairs: &[(u64, u64)]) {
    let mut enc = zstd_writer(path).unwrap();
    for (a, b) in pairs {
        enc.write_all(&a.to_le_bytes()).unwrap();
        enc.write_all(&b.to_le_bytes()).unwrap();
    }
}

fn read_pairs_zstd(path: &str) -> Vec<(u64, u64)> {
    let mut dec = zstd_reader(path).unwrap();
    let mut out = Vec::new();
    while let Some(p) = read_u64_pair_zstd(&mut dec).unwrap() {
        out.push(p);
    }
    out
}

fn read_u64s(path: &str) -> Vec<u64> {
    fs::read(path)
        .unwrap()
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn read_u32s_zstd(path: &str) -> Vec<u32> {
    let mut dec = zstd_reader(path).unwrap();
    let mut buf = Vec::new();
    dec.read_to_end(&mut buf).unwrap();
    buf.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn read_f32s(path: &str) -> Vec<f32> {
    fs::read(path)
        .unwrap()
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn h(url: &str) -> u64 {
    url_to_id(&normalize_url(url))
}

fn write_conn(dir: &str, name: &str, content: &str) {
    fs::write(format!("{}/{}", dir, name), content).unwrap();
}

/* ───────────────────────────── constants ───────────────────────────── */

#[test]
fn all_working_paths_live_inside_pagerank_dir() {
    for p in [
        compute::IMPORT_ONLY_FILE,
        compute::CONNECTIONS_DIR,
        compute::EDGES_DIR,
        compute::NODES_DIR,
        compute::MERGED_DIR,
        compute::EDGES_SORTED,
        compute::CSR_EDGES,
        compute::CSR_OFFSETS,
        compute::SCORES_A,
        compute::SCORES_B,
        compute::TOTAL_NODES,
        compute::OUT_DEGREE,
    ] {
        assert!(
            p.starts_with(&format!("{}/", compute::PAGERANK_DIR)),
            "{}",
            p
        );
    }
}

#[test]
fn dirs_constant_lists_every_directory_once() {
    let set: HashSet<&str> = compute::DIRS.iter().copied().collect();
    assert_eq!(set.len(), compute::DIRS.len());
    for d in [
        compute::PAGERANK_DIR,
        compute::CONNECTIONS_DIR,
        compute::EDGES_DIR,
        compute::NODES_DIR,
        compute::MERGED_DIR,
    ] {
        assert!(set.contains(d), "{}", d);
    }
    // The root must be created first
    assert_eq!(compute::DIRS[0], compute::PAGERANK_DIR);
}

#[test]
fn score_files_are_distinct() {
    assert_ne!(compute::SCORES_A, compute::SCORES_B);
    assert_ne!(compute::CSR_EDGES, compute::CSR_OFFSETS);
}

#[test]
fn buffer_size_is_positive() {
    assert!(compute::BUFFER_SIZE > 0);
}

/* ───────────────────────────── read_u64_pair ───────────────────────────── */

#[test]
fn read_u64_pair_reads_all_pairs_then_none() {
    let t = Tmp::new();
    let p = t.p("pairs.bin");
    write_pairs_raw(&p, &[(1, 2), (u64::MAX, 0), (42, 42)]);
    assert_eq!(read_pairs_raw(&p), vec![(1, 2), (u64::MAX, 0), (42, 42)]);
}

#[test]
fn read_u64_pair_empty_file_is_none() {
    let t = Tmp::new();
    let p = t.p("empty.bin");
    File::create(&p).unwrap();
    let mut r = BufReader::new(File::open(&p).unwrap());
    assert_eq!(read_u64_pair(&mut r).unwrap(), None);
    // repeated reads at EOF stay None
    assert_eq!(read_u64_pair(&mut r).unwrap(), None);
}

#[test]
fn read_u64_pair_truncated_tail_is_treated_as_eof() {
    let t = Tmp::new();
    let p = t.p("trunc.bin");
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&7u64.to_le_bytes());
    bytes.extend_from_slice(&8u64.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9]); // 9 of 16 bytes
    fs::write(&p, bytes).unwrap();
    assert_eq!(read_pairs_raw(&p), vec![(7, 8)]);
}

#[test]
fn read_u64_pair_is_little_endian() {
    let t = Tmp::new();
    let p = t.p("le.bin");
    let mut bytes = vec![0u8; 16];
    bytes[0] = 1; // a = 1
    bytes[15] = 0x80; // b = 1 << 63
    fs::write(&p, bytes).unwrap();
    assert_eq!(read_pairs_raw(&p), vec![(1, 1u64 << 63)]);
}

/* ───────────────────────────── zstd helpers ───────────────────────────── */

#[test]
fn zstd_writer_reader_roundtrip() {
    let t = Tmp::new();
    let p = t.p("pairs.zst");
    let pairs: Vec<(u64, u64)> = (0..10_000u64).map(|i| (i * 3, u64::MAX - i)).collect();
    write_pairs_zstd(&p, &pairs);
    assert_eq!(read_pairs_zstd(&p), pairs);
}

#[test]
fn zstd_writer_actually_compresses() {
    let t = Tmp::new();
    let p = t.p("zeros.zst");
    write_pairs_zstd(&p, &vec![(0u64, 0u64); 100_000]);
    assert!(fs::metadata(&p).unwrap().len() < 100_000 * 16 / 100);
}

#[test]
fn zstd_empty_stream_reads_none() {
    let t = Tmp::new();
    let p = t.p("empty.zst");
    write_pairs_zstd(&p, &[]);
    assert!(read_pairs_zstd(&p).is_empty());
}

#[test]
fn zstd_reader_missing_file_is_error() {
    let t = Tmp::new();
    assert!(zstd_reader(&t.p("missing.zst")).is_err());
}

#[test]
fn zstd_writer_into_missing_dir_is_error() {
    let t = Tmp::new();
    assert!(zstd_writer(&t.p("no/such/dir/x.zst")).is_err());
}

#[test]
fn zstd_reader_on_non_zstd_data_errors_instead_of_eof() {
    let t = Tmp::new();
    let p = t.p("plain.bin");
    write_pairs_raw(&p, &[(1, 2), (3, 4)]);
    let mut dec = zstd_reader(&p).unwrap();
    assert!(read_u64_pair_zstd(&mut dec).is_err());
}

#[test]
fn zstd_truncated_last_pair_is_treated_as_eof() {
    let t = Tmp::new();
    let p = t.p("odd.zst");
    {
        let mut enc = zstd_writer(&p).unwrap();
        enc.write_all(&5u64.to_le_bytes()).unwrap();
        enc.write_all(&6u64.to_le_bytes()).unwrap();
        enc.write_all(&[9u8; 5]).unwrap();
    }
    assert_eq!(read_pairs_zstd(&p), vec![(5, 6)]);
}

/* ───────────────────────────── IdMap ───────────────────────────── */

#[test]
fn id_map_open_and_lookup() {
    let t = Tmp::new();
    let p = t.p("id_map.bin");
    write_pairs_raw(&p, &[(10, 0), (20, 1), (30, 2)]);
    let m = IdMap::open(&p).unwrap();
    assert_eq!(m.pairs(), &[(10, 0), (20, 1), (30, 2)]);
    assert_eq!(m.lookup(10), Some(0));
    assert_eq!(m.lookup(20), Some(1));
    assert_eq!(m.lookup(30), Some(2));
    assert_eq!(m.lookup(15), None);
    assert_eq!(m.lookup(0), None);
    assert_eq!(m.lookup(u64::MAX), None);
}

#[test]
fn id_map_missing_file_is_error() {
    let t = Tmp::new();
    assert!(IdMap::open(&t.p("missing")).is_err());
}

#[test]
fn id_map_empty_file_is_empty() {
    let t = Tmp::new();
    let p = t.p("empty");
    File::create(&p).unwrap();
    let m = IdMap::open(&p).unwrap();
    assert!(m.pairs().is_empty());
    assert_eq!(m.lookup(0), None);
}

#[test]
fn id_map_ignores_trailing_partial_record() {
    let t = Tmp::new();
    let p = t.p("partial");
    let mut b = Vec::new();
    b.extend_from_slice(&1u64.to_le_bytes());
    b.extend_from_slice(&2u64.to_le_bytes());
    b.extend_from_slice(&[0xFF; 12]);
    fs::write(&p, b).unwrap();
    let m = IdMap::open(&p).unwrap();
    assert_eq!(m.pairs(), &[(1, 2)]);
}

#[test]
fn id_map_extreme_values() {
    let t = Tmp::new();
    let p = t.p("extreme");
    write_pairs_raw(&p, &[(0, u64::MAX), (u64::MAX, 0)]);
    let m = IdMap::open(&p).unwrap();
    assert_eq!(m.lookup(0), Some(u64::MAX));
    assert_eq!(m.lookup(u64::MAX), Some(0));
}

#[test]
fn id_map_large() {
    let t = Tmp::new();
    let p = t.p("large");
    let pairs: Vec<(u64, u64)> = (0..200_000u64).map(|i| (i * 7 + 1, i)).collect();
    write_pairs_raw(&p, &pairs);
    let m = IdMap::open(&p).unwrap();
    assert_eq!(m.pairs().len(), 200_000);
    for i in (0..200_000u64).step_by(997) {
        assert_eq!(m.lookup(i * 7 + 1), Some(i));
        assert_eq!(m.lookup(i * 7 + 2), None);
    }
}

/* ───────────────────────────── hashing::run_in ───────────────────────────── */

#[test]
fn hashing_missing_connection_dir_is_error() {
    let t = Tmp::new();
    let edges = t.dir("edges");
    assert!(hashing::run_in(&t.p("nope"), &edges).is_err());
}

#[test]
fn hashing_dir_with_only_subdirectories_is_error() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    fs::create_dir_all(format!("{}/sub", conn)).unwrap();
    let edges = t.dir("edges");
    assert!(hashing::run_in(&conn, &edges).is_err());
}

#[test]
fn hashing_ignores_lines_without_arrow_and_empty_lines() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(
        &conn,
        "c.txt",
        "\n\nhttps://a.com\njust text\nhttps://a.com -> https://b.com\n=>\n- >\n",
    );
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(shards.len(), 1);
    assert_eq!(
        read_pairs_raw(&shards[0]),
        vec![(h("https://a.com"), h("https://b.com"))]
    );
}

#[test]
fn hashing_only_invalid_lines_produces_no_shard() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "c.txt", "garbage\nmore garbage\n");
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert!(shards.is_empty());
}

#[test]
fn hashing_trims_whitespace_around_urls() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "c.txt", "   https://a.com/x   ->\thttps://b.com/y \t\n");
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(
        read_pairs_raw(&shards[0]),
        vec![(h("https://a.com/x"), h("https://b.com/y"))]
    );
}

#[test]
fn hashing_normalizes_urls_so_variants_dedup() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(
        &conn,
        "c.txt",
        "https://a.com/->https://b.com\nhttps://A.com?x=1->https://b.com/#frag\nhttps://a.com->https://b.com/\n",
    );
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(read_pairs_raw(&shards[0]).len(), 1);
}

#[test]
fn hashing_drops_self_loops_after_normalization() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(
        &conn,
        "c.txt",
        "https://a.com/->https://a.com\nhttps://a.com/?q=1->https://a.com/#x\nhttps://a.com->https://b.com\n",
    );
    let shards = hashing::run_in(&conn, &edges).unwrap();
    let pairs = read_pairs_raw(&shards[0]);
    assert_eq!(pairs, vec![(h("https://a.com"), h("https://b.com"))]);
}

#[test]
fn hashing_splits_on_first_arrow_only() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "c.txt", "https://a.com->https://b.com/x->y\n");
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(
        read_pairs_raw(&shards[0]),
        vec![(h("https://a.com"), h("https://b.com/x->y"))]
    );
}

#[test]
fn hashing_output_is_sorted_and_deduplicated() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    let mut content = String::new();
    for i in 0..50 {
        for j in 0..5 {
            content.push_str(&format!("https://s{}.com->https://d{}.com\n", i % 10, j));
        }
    }
    write_conn(&conn, "c.txt", &content);
    let shards = hashing::run_in(&conn, &edges).unwrap();
    let pairs = read_pairs_raw(&shards[0]);
    assert_eq!(pairs.len(), 50);
    let mut sorted = pairs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(pairs, sorted);
}

#[test]
fn hashing_deletes_processed_connection_files() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "a.txt", "https://a.com->https://b.com\n");
    write_conn(&conn, "b.txt", "garbage only\n");
    hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(fs::read_dir(&conn).unwrap().count(), 0);
}

#[test]
fn hashing_keeps_subdirectories_untouched() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    fs::create_dir_all(format!("{}/keep", conn)).unwrap();
    write_conn(&conn, "a.txt", "https://a.com->https://b.com\n");
    hashing::run_in(&conn, &edges).unwrap();
    assert!(std::path::Path::new(&format!("{}/keep", conn)).is_dir());
}

#[test]
fn hashing_merges_multiple_files_into_one_shard() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "1.txt", "https://a.com->https://b.com\n");
    write_conn(&conn, "2.txt", "https://c.com->https://d.com\nhttps://a.com->https://b.com\n");
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(shards.len(), 1);
    assert_eq!(read_pairs_raw(&shards[0]).len(), 2);
    assert!(shards[0].starts_with(&edges));
    assert!(shards[0].ends_with("inc_hash_shard_0.bin"));
}

#[test]
fn hashing_handles_crlf_line_endings() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    write_conn(&conn, "c.txt", "https://a.com->https://b.com\r\nhttps://c.com->https://d.com\r\n");
    let shards = hashing::run_in(&conn, &edges).unwrap();
    let pairs: HashSet<(u64, u64)> = read_pairs_raw(&shards[0]).into_iter().collect();
    assert!(pairs.contains(&(h("https://a.com"), h("https://b.com"))));
    assert!(pairs.contains(&(h("https://c.com"), h("https://d.com"))));
}

#[test]
fn hashing_skips_non_utf8_lines_but_keeps_the_rest() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges = t.dir("edges");
    let mut bytes = b"https://a.com->https://b.com\n".to_vec();
    bytes.extend_from_slice(&[0xff, 0xfe, b'-', b'>', 0xc3, b'\n']);
    bytes.extend_from_slice(b"https://c.com->https://d.com\n");
    fs::write(format!("{}/bin.txt", conn), bytes).unwrap();
    let shards = hashing::run_in(&conn, &edges).unwrap();
    assert_eq!(read_pairs_raw(&shards[0]).len(), 2);
}

/* ───────────────────────────── merge::run_with ───────────────────────────── */

fn merge_dirs(t: &Tmp) -> (String, String, String, String) {
    (
        t.p("id_map.bin"),
        t.dir("nodes"),
        t.dir("merged"),
        t.p("total_nodes.txt"),
    )
}

#[test]
fn merge_no_shards_and_no_map_creates_empty_map() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let n = merge::run_with(&[], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n, 0);
    assert_eq!(fs::metadata(&id_map).unwrap().len(), 0);
}

#[test]
fn merge_missing_shard_is_error() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    assert!(merge::run_with(&[t.p("missing.bin")], &id_map, &nodes, &merged, &total).is_err());
}

#[test]
fn merge_assigns_dense_ids_in_hash_order() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let shard = t.p("s.bin");
    write_pairs_raw(&shard, &[(30, 10), (20, 30)]);
    let n = merge::run_with(&[shard], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n, 3);
    assert_eq!(read_pairs_raw(&id_map), vec![(10, 0), (20, 1), (30, 2)]);
}

#[test]
fn merge_removes_temporary_node_shards() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let shard = t.p("s.bin");
    write_pairs_raw(&shard, &[(1, 2)]);
    merge::run_with(&[shard], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(fs::read_dir(&nodes).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&merged).unwrap().count(), 0, "merged file must be renamed away");
}

#[test]
fn merge_dedups_hashes_across_shards() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let s1 = t.p("s1.bin");
    let s2 = t.p("s2.bin");
    write_pairs_raw(&s1, &[(1, 2), (2, 3)]);
    write_pairs_raw(&s2, &[(3, 1), (4, 2)]);
    let n = merge::run_with(&[s1, s2], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n, 4);
    let ids: Vec<u64> = read_pairs_raw(&id_map).iter().map(|p| p.1).collect();
    assert_eq!(ids, vec![0, 1, 2, 3]);
}

#[test]
fn merge_incremental_continues_ids_from_total_nodes() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let s1 = t.p("s1.bin");
    write_pairs_raw(&s1, &[(100, 200)]);
    let n1 = merge::run_with(&[s1], &id_map, &nodes, &merged, &total).unwrap();
    fs::write(&total, n1.to_string()).unwrap();

    let s2 = t.p("s2.bin");
    write_pairs_raw(&s2, &[(50, 100), (300, 200)]);
    let n2 = merge::run_with(&[s2], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n2, 4);
    let m = IdMap::open(&id_map).unwrap();
    assert_eq!(m.lookup(100), Some(0));
    assert_eq!(m.lookup(200), Some(1));
    // new hashes get ids after the old ones, in hash order
    assert_eq!(m.lookup(50), Some(2));
    assert_eq!(m.lookup(300), Some(3));
    // map stays sorted by hash
    let hashes: Vec<u64> = m.pairs().iter().map(|p| p.0).collect();
    assert_eq!(hashes, vec![50, 100, 200, 300]);
}

#[test]
fn merge_total_nodes_with_whitespace_is_parsed() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    write_pairs_raw(&id_map, &[(5, 0), (6, 1), (7, 2)]);
    fs::write(&total, "  3 \n").unwrap();
    let s = t.p("s.bin");
    write_pairs_raw(&s, &[(8, 5)]);
    let n = merge::run_with(&[s], &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n, 4);
    assert_eq!(IdMap::open(&id_map).unwrap().lookup(8), Some(3));
}

/// BUG: a corrupted total_nodes file is silently parsed as 0 (`unwrap_or(0)`),
/// so new nodes reuse ids that are already taken in the existing id map.
#[test]
fn merge_corrupt_total_nodes_must_not_reuse_existing_ids() {
    let t = Tmp::new();
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    write_pairs_raw(&id_map, &[(5, 0), (6, 1), (7, 2)]);
    fs::write(&total, "not a number").unwrap();
    let s = t.p("s.bin");
    write_pairs_raw(&s, &[(8, 9)]);
    let result = merge::run_with(&[s], &id_map, &nodes, &merged, &total);
    if let Ok(_) = result {
        let ids: Vec<u64> = IdMap::open(&id_map).unwrap().pairs().iter().map(|p| p.1).collect();
        let unique: HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate ids assigned: {:?}", ids);
    }
}

/* ───────────────────────────── translate::run_with ───────────────────────────── */

#[test]
fn translate_maps_hashes_to_ids_and_drops_unknown_nodes() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    let edges_sorted = t.p("edges_sorted.bin.zst");
    write_pairs_raw(&id_map, &[(10, 0), (20, 1), (30, 2)]);
    let shard = format!("{}/inc_hash_shard_0.bin", edges_dir);
    // (20,99) has an unknown destination and must be dropped
    write_pairs_raw(&shard, &[(10, 20), (20, 30), (30, 10), (20, 99)]);

    translate::run_with(vec![shard.clone()], &id_map, &edges_sorted, &edges_dir).unwrap();

    assert_eq!(read_pairs_zstd(&edges_sorted), vec![(0, 1), (1, 2), (2, 0)]);
    assert!(!std::path::Path::new(&shard).exists(), "input shard must be removed");
}

#[test]
fn translate_merges_with_existing_sorted_edges() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    let edges_sorted = t.p("edges_sorted.bin.zst");
    write_pairs_raw(&id_map, &[(10, 0), (20, 1), (30, 2)]);
    write_pairs_zstd(&edges_sorted, &[(0, 2), (1, 0)]);
    let shard = format!("{}/s.bin", edges_dir);
    write_pairs_raw(&shard, &[(10, 20), (20, 10)]); // (0,1) new, (1,0) duplicate

    translate::run_with(vec![shard], &id_map, &edges_sorted, &edges_dir).unwrap();
    assert_eq!(read_pairs_zstd(&edges_sorted), vec![(0, 1), (0, 2), (1, 0)]);
}

#[test]
fn translate_leaves_no_scratch_files_behind() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    let edges_sorted = t.p("edges_sorted.bin.zst");
    write_pairs_raw(&id_map, &[(10, 0), (20, 1)]);
    let shard = format!("{}/s.bin", edges_dir);
    write_pairs_raw(&shard, &[(10, 20)]);
    translate::run_with(vec![shard], &id_map, &edges_sorted, &edges_dir).unwrap();
    let left: Vec<_> = fs::read_dir(&edges_dir).unwrap().collect();
    assert!(left.is_empty(), "scratch files left: {:?}", left);
}

#[test]
fn translate_with_all_unknown_nodes_produces_empty_graph() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    let edges_sorted = t.p("edges_sorted.bin.zst");
    write_pairs_raw(&id_map, &[(10, 0)]);
    let shard = format!("{}/s.bin", edges_dir);
    write_pairs_raw(&shard, &[(1, 2), (3, 4)]);
    translate::run_with(vec![shard], &id_map, &edges_sorted, &edges_dir).unwrap();
    assert!(read_pairs_zstd(&edges_sorted).is_empty());
}

#[test]
fn translate_missing_id_map_is_error() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let shard = format!("{}/s.bin", edges_dir);
    write_pairs_raw(&shard, &[(1, 2)]);
    assert!(
        translate::run_with(vec![shard], &t.p("missing"), &t.p("out.zst"), &edges_dir).is_err()
    );
}

#[test]
fn translate_missing_shard_is_error() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    write_pairs_raw(&id_map, &[(1, 0)]);
    assert!(
        translate::run_with(vec![t.p("missing")], &id_map, &t.p("out.zst"), &edges_dir).is_err()
    );
}

#[test]
fn translate_handles_extreme_hash_values() {
    let t = Tmp::new();
    let edges_dir = t.dir("edges");
    let id_map = t.p("id_map.bin");
    let edges_sorted = t.p("edges_sorted.bin.zst");
    write_pairs_raw(&id_map, &[(0, 0), (u64::MAX, 1)]);
    let shard = format!("{}/s.bin", edges_dir);
    write_pairs_raw(&shard, &[(0, u64::MAX), (u64::MAX, 0)]);
    translate::run_with(vec![shard], &id_map, &edges_sorted, &edges_dir).unwrap();
    assert_eq!(read_pairs_zstd(&edges_sorted), vec![(0, 1), (1, 0)]);
}

/* ───────────────────────────── csr::run_with ───────────────────────────── */

struct Csr {
    offsets: Vec<u64>,
    edges: Vec<u64>,
    out_degree: Vec<u32>,
}

fn build_csr(t: &Tmp, n: usize, edges: &[(u64, u64)]) -> Csr {
    let mut sorted = edges.to_vec();
    sorted.sort_unstable();
    let es = t.p("edges_sorted.bin.zst");
    write_pairs_zstd(&es, &sorted);
    let off = t.p("csr_offsets.bin");
    let ce = t.p("csr_edges.bin.zst");
    let od = t.p("out_degree.bin.zst");
    let scratch = t.dir("csr_scratch");
    csr::run_with(n, &es, &off, &ce, &od, &scratch).unwrap();

    let mut dec = zstd_reader(&ce).unwrap();
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw).unwrap();
    Csr {
        offsets: read_u64s(&off),
        edges: raw
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
        out_degree: read_u32s_zstd(&od),
    }
}

#[test]
fn csr_empty_graph() {
    let t = Tmp::new();
    let c = build_csr(&t, 3, &[]);
    assert_eq!(c.offsets, vec![0, 0, 0, 0]);
    assert!(c.edges.is_empty());
    assert_eq!(c.out_degree, vec![0, 0, 0]);
}

#[test]
fn csr_offsets_are_byte_offsets_of_in_neighbours() {
    let t = Tmp::new();
    // 0->1, 0->2, 1->2, 2->0
    let c = build_csr(&t, 3, &[(0, 1), (0, 2), (1, 2), (2, 0)]);
    // in-degree: node0=1, node1=1, node2=2 ; offsets in bytes (8 per edge)
    assert_eq!(c.offsets, vec![0, 8, 16, 32]);
    assert_eq!(c.out_degree, vec![2, 1, 1]);
    // edges grouped by destination, sources ascending inside a group
    assert_eq!(c.edges, vec![2, 0, 0, 1]);
}

#[test]
fn csr_offsets_are_monotonic_and_last_matches_edge_count() {
    let t = Tmp::new();
    let n = 50usize;
    let edges: Vec<(u64, u64)> = (0..n as u64)
        .flat_map(|i| [(i, (i * 7 + 1) % n as u64), (i, (i * 3 + 2) % n as u64)])
        .filter(|(a, b)| a != b)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let c = build_csr(&t, n, &edges);
    assert_eq!(c.offsets.len(), n + 1);
    for w in c.offsets.windows(2) {
        assert!(w[0] <= w[1]);
    }
    assert_eq!(*c.offsets.last().unwrap(), edges.len() as u64 * 8);
    assert_eq!(c.edges.len(), edges.len());
    assert_eq!(
        c.out_degree.iter().map(|d| *d as usize).sum::<usize>(),
        edges.len()
    );
}

#[test]
fn csr_isolated_trailing_nodes_have_zero_degrees() {
    let t = Tmp::new();
    let c = build_csr(&t, 6, &[(0, 1)]);
    assert_eq!(c.out_degree, vec![1, 0, 0, 0, 0, 0]);
    assert_eq!(c.offsets, vec![0, 0, 8, 8, 8, 8, 8]);
}

#[test]
fn csr_removes_scratch_shards() {
    let t = Tmp::new();
    build_csr(&t, 3, &[(0, 1), (1, 2)]);
    let left: Vec<String> = fs::read_dir(t.p("csr_scratch"))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("csr_shard_") || n.starts_with("csr_intermediate_"))
        .collect();
    assert!(left.is_empty(), "{:?}", left);
}

#[test]
fn csr_writes_degree_cache_for_reuse() {
    let t = Tmp::new();
    build_csr(&t, 3, &[(0, 1), (0, 2), (2, 1)]);
    let cache = fs::read(format!("{}/csr_degrees.bin", t.p("csr_scratch"))).unwrap();
    let vals: Vec<u32> = cache
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    // out degrees then in degrees
    assert_eq!(vals, vec![2, 0, 1, 0, 2, 1]);
}

#[test]
fn csr_missing_edges_file_is_error() {
    let t = Tmp::new();
    let scratch = t.dir("s");
    assert!(
        csr::run_with(
            2,
            &t.p("missing.zst"),
            &t.p("o"),
            &t.p("e"),
            &t.p("d"),
            &scratch
        )
        .is_err()
    );
}

#[test]
#[should_panic]
fn csr_node_id_out_of_range_panics() {
    // Documents current behaviour: ids >= total_nodes index out of bounds instead of Err
    let t = Tmp::new();
    build_csr(&t, 2, &[(0, 5)]);
}

/* ───────────────────────────── iterate::run_with ───────────────────────────── */

fn run_pagerank(t: &Tmp, n: usize, edges: &[(u64, u64)]) -> Vec<f32> {
    build_csr(t, n, edges);
    let sa = t.p("scores_a.bin");
    let sb = t.p("scores_b.bin");
    let init: Vec<u8> = (0..n).flat_map(|_| (1.0f32 / n as f32).to_le_bytes()).collect();
    fs::write(&sa, init).unwrap();
    let out = iterate::run_with(
        n,
        &t.p("csr_offsets.bin"),
        &t.p("csr_edges.bin.zst"),
        &t.p("out_degree.bin.zst"),
        &sa,
        &sb,
    )
    .unwrap();
    assert!(out == sa || out == sb);
    read_f32s(&out)
}

#[test]
fn iterate_single_dangling_node_has_score_one() {
    let t = Tmp::new();
    let s = run_pagerank(&t, 1, &[]);
    assert_eq!(s.len(), 1);
    assert!((s[0] - 1.0).abs() < 1e-6, "{}", s[0]);
}

#[test]
fn iterate_converged_input_returns_after_first_iteration() {
    // A 2-cycle starting at the uniform distribution is already stationary:
    // the first iteration writes scores_b with delta 0 and returns it.
    let t = Tmp::new();
    build_csr(&t, 2, &[(0, 1), (1, 0)]);
    let sa = t.p("scores_a.bin");
    let sb = t.p("scores_b.bin");
    let init: Vec<u8> = [0.5f32, 0.5].iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(&sa, init).unwrap();
    let out = iterate::run_with(
        2,
        &t.p("csr_offsets.bin"),
        &t.p("csr_edges.bin.zst"),
        &t.p("out_degree.bin.zst"),
        &sa,
        &sb,
    )
    .unwrap();
    assert_eq!(out, sb);
    let s = read_f32s(&out);
    assert!((s[0] - 0.5).abs() < 1e-6 && (s[1] - 0.5).abs() < 1e-6);
}

#[test]
fn iterate_scores_sum_to_one() {
    let t = Tmp::new();
    let s = run_pagerank(&t, 5, &[(0, 1), (1, 2), (2, 0), (3, 0), (4, 4 - 1)]);
    let sum: f32 = s.iter().sum();
    assert!((sum - 1.0).abs() < 1e-4, "{}", sum);
}

#[test]
fn iterate_all_scores_positive_and_finite() {
    let t = Tmp::new();
    let s = run_pagerank(&t, 6, &[(0, 1), (2, 1), (3, 1), (4, 5)]);
    for v in s {
        assert!(v.is_finite() && v > 0.0, "{}", v);
    }
}

#[test]
fn iterate_more_in_links_more_rank() {
    let t = Tmp::new();
    let s = run_pagerank(&t, 4, &[(0, 3), (1, 3), (2, 3), (3, 0)]);
    assert!(s[3] > s[0] && s[3] > s[1] && s[3] > s[2]);
    assert!(s[0] > s[1], "node 0 receives the hub's vote");
    assert!((s[1] - s[2]).abs() < 1e-6, "symmetric leaves");
}

#[test]
fn iterate_missing_out_degree_is_error() {
    let t = Tmp::new();
    build_csr(&t, 2, &[(0, 1)]);
    assert!(
        iterate::run_with(
            2,
            &t.p("csr_offsets.bin"),
            &t.p("csr_edges.bin.zst"),
            &t.p("missing.zst"),
            &t.p("a"),
            &t.p("b"),
        )
        .is_err()
    );
}

#[test]
fn iterate_missing_offsets_is_error() {
    let t = Tmp::new();
    build_csr(&t, 2, &[(0, 1)]);
    assert!(
        iterate::run_with(
            2,
            &t.p("missing_offsets"),
            &t.p("csr_edges.bin.zst"),
            &t.p("out_degree.bin.zst"),
            &t.p("a"),
            &t.p("b"),
        )
        .is_err()
    );
}

/* ───────────────────────────── full chain sanity ───────────────────────────── */

#[test]
fn hashing_merge_translate_chain_preserves_edge_set() {
    let t = Tmp::new();
    let conn = t.dir("conn");
    let edges_dir = t.dir("edges");
    let (id_map, nodes, merged, total) = merge_dirs(&t);
    let es = t.p("edges_sorted.bin.zst");
    let links = [
        ("https://a.com", "https://b.com"),
        ("https://b.com", "https://c.com"),
        ("https://c.com", "https://a.com"),
        ("https://d.com", "https://a.com"),
    ];
    let content: String = links.iter().map(|(a, b)| format!("{}->{}\n", a, b)).collect();
    write_conn(&conn, "c.txt", &content);

    let shards = hashing::run_in(&conn, &edges_dir).unwrap();
    let n = merge::run_with(&shards, &id_map, &nodes, &merged, &total).unwrap();
    assert_eq!(n, 4);
    translate::run_with(shards, &id_map, &es, &edges_dir).unwrap();

    let m = IdMap::open(&id_map).unwrap();
    let expected: HashSet<(u64, u64)> = links
        .iter()
        .map(|(a, b)| (m.lookup(h(a)).unwrap(), m.lookup(h(b)).unwrap()))
        .collect();
    let got: HashSet<(u64, u64)> = read_pairs_zstd(&es).into_iter().collect();
    assert_eq!(got, expected);
}
