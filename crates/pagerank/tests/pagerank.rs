// tests/pagerank.rs
//
// Integration tests for the pagerank pipeline.
// Run with: cargo test
//
// Replace `prieco_rs` below with the [package] name from your Cargo.toml
// if it differs from the lib name.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    sync::atomic::{AtomicU32, Ordering},
};

use prieco_core::{normalize_url, url_to_id};
use prieco_pagerank::{
    compute::{read_u64_pair_zstd, zstd_reader, zstd_writer},
    import::{hashing, merge, translate},
    iter::iterate,
    nodes::csr,
};

// ── RAII cleanup guard ────────────────────────────────────────────────────────
// Deletes the temp directory when dropped, even if the test panics.

struct TmpDir(String);

static COUNTER: AtomicU32 = AtomicU32::new(1);

impl TmpDir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        // Put temp dirs inside target/ so they don't pollute the project root
        // and are ignored by version control.
        let path = format!("target/test_tmp_{}", n);
        fs::create_dir_all(&path).unwrap();
        TmpDir(path)
    }

    fn path(&self) -> &str {
        &self.0
    }

    // Create a subdirectory and return its path.
    fn sub(&self, name: &str) -> String {
        let p = format!("{}/{}", self.0, name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ── Binary I/O helpers ────────────────────────────────────────────────────────

fn write_pairs_zstd(path: &str, pairs: &[(u64, u64)]) {
    let mut enc = zstd_writer(path).unwrap();
    for (a, b) in pairs {
        enc.write_all(&a.to_le_bytes()).unwrap();
        enc.write_all(&b.to_le_bytes()).unwrap();
    }
}

fn read_pairs_zstd(path: &str) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut dec = zstd_reader(path).unwrap();
    loop {
        match read_u64_pair_zstd(&mut dec).unwrap() {
            Some(p) => out.push(p),
            None => break,
        }
    }
    out
}

fn read_id_map(path: &str) -> HashMap<u64, u64> {
    let mut map = HashMap::new();
    let mut f = BufReader::new(File::open(path).unwrap());
    let mut buf = [0u8; 16];
    while f.read_exact(&mut buf).is_ok() {
        let hash = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let id = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        map.insert(hash, id);
    }
    map
}

fn write_connections(dir: &str, filename: &str, edges: &[(&str, &str)]) {
    fs::create_dir_all(dir).unwrap();
    let mut f = File::create(format!("{}/{}", dir, filename)).unwrap();
    for (a, b) in edges {
        writeln!(f, "{}->{}", a, b).unwrap();
    }
}

fn read_pairs_raw(path: &str) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut f = BufReader::new(File::open(path).unwrap());
    let mut buf = [0u8; 16];
    while f.read_exact(&mut buf).is_ok() {
        out.push((
            u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            u64::from_le_bytes(buf[8..16].try_into().unwrap()),
        ));
    }
    out
}

// ── Score helpers ─────────────────────────────────────────────────────────────

fn init_scores(n: usize, path: &str) {
    let init = 1.0f32 / n as f32;
    let mut f = BufWriter::with_capacity(1 << 20, File::create(path).unwrap());
    for _ in 0..n {
        f.write_all(&init.to_le_bytes()).unwrap();
    }
}

fn compress_scores(src: &str, n: u64, dst: &str) {
    let mut rdr = BufReader::new(File::open(src).unwrap());
    let mut enc = zstd_writer(dst).unwrap();
    let mut buf = [0u8; 4];
    for _ in 0..n {
        rdr.read_exact(&mut buf).unwrap();
        enc.write_all(&buf).unwrap();
    }
}

fn lookup_score(url: &str, id_map: &str, scores: &str) -> Option<f32> {
    let target = url_to_id(&normalize_url(url));
    let map = read_id_map(id_map);
    let id = *map.get(&target)?;
    let mut dec = zstd_reader(scores).unwrap();
    let mut buf = [0u8; 4];
    for _ in 0..id {
        dec.read_exact(&mut buf).ok()?;
    }
    dec.read_exact(&mut buf).ok()?;
    Some(f32::from_le_bytes(buf))
}

// ── Full pipeline helper ──────────────────────────────────────────────────────

fn run_pipeline(tmp: &TmpDir, edges: &[(&str, &str)]) -> (u64, HashMap<String, f32>) {
    // Each phase gets its own scratch subdirectory so parallel tests can't
    // collide on intermediate shard filenames.
    let conn_dir = tmp.sub("connections");
    let edges_dir = tmp.sub("edges");
    let nodes_dir = tmp.sub("nodes");
    let merged_dir = tmp.sub("merged");
    let csr_dir = tmp.sub("csr_scratch");

    write_connections(&conn_dir, "c.txt", edges);

    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    let csr_off = format!("{}/csr_offsets.bin", tmp.path());
    let csr_e = format!("{}/csr_edges.bin", tmp.path());
    let out_deg = format!("{}/out_degree.bin.zst", tmp.path());
    let scores_a = format!("{}/scores_a.bin", tmp.path());
    let scores_b = format!("{}/scores_b.bin", tmp.path());
    let final_s = format!("{}/pageranks.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    // Phase 1
    let hash_shards = hashing::run_in(&conn_dir, &edges_dir).unwrap();
    let n = merge::run_with(&hash_shards, &id_map, &nodes_dir, &merged_dir, &total_nodes).unwrap();
    fs::write(&total_nodes, n.to_string()).unwrap();
    translate::run_with(hash_shards, &id_map, &edges_s, &edges_dir).unwrap();

    // Phase 2 — pass csr_dir so flush() writes shards there, not into csr_edges path
    csr::run_with(n as usize, &edges_s, &csr_off, &csr_e, &out_deg, &csr_dir).unwrap();

    // Phase 3
    init_scores(n as usize, &scores_a);

    // Phase 4
    let final_file =
        iterate::run_with(n as usize, &csr_off, &csr_e, &out_deg, &scores_a, &scores_b).unwrap();

    // Phase 5
    compress_scores(&final_file, n, &final_s);

    let urls: Vec<&str> = {
        let mut v: Vec<&str> = edges.iter().flat_map(|(a, b)| [*a, *b]).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let scores = urls
        .iter()
        .filter_map(|&url| Some((url.to_string(), lookup_score(url, &id_map, &final_s)?)))
        .collect();

    (n, scores)
}

// ── Incremental pipeline helper ───────────────────────────────────────────────

fn run_incremental(
    tmp: &TmpDir,
    batch1: &[(&str, &str)],
    batch2: &[(&str, &str)],
) -> (u64, HashMap<String, f32>) {
    let edges_dir = tmp.sub("edges");
    let csr_dir = tmp.sub("csr_scratch");

    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    // Batch 1
    let conn1 = tmp.sub("conn1");
    write_connections(&conn1, "c.txt", batch1);
    let sh1 = hashing::run_in(&conn1, &edges_dir).unwrap();
    let _ = merge::run_with(&sh1, &id_map, &tmp.sub("n1"), &tmp.sub("m1"), &total_nodes).unwrap();
    let n1 = read_id_map(&id_map).len();
    fs::write(&total_nodes, n1.to_string()).unwrap();
    translate::run_with(sh1, &id_map, &edges_s, &edges_dir).unwrap();

    // Batch 2
    let conn2 = tmp.sub("conn2");
    write_connections(&conn2, "c.txt", batch2);
    let sh2 = hashing::run_in(&conn2, &edges_dir).unwrap();
    let n = merge::run_with(&sh2, &id_map, &tmp.sub("n2"), &tmp.sub("m2"), &total_nodes).unwrap();
    fs::write(&total_nodes, n.to_string()).unwrap();
    translate::run_with(sh2, &id_map, &edges_s, &edges_dir).unwrap();

    // Build and score
    let csr_off = format!("{}/csr_offsets.bin", tmp.path());
    let csr_e = format!("{}/csr_edges.bin", tmp.path());
    let out_deg = format!("{}/out_degree.bin.zst", tmp.path());
    let scores_a = format!("{}/scores_a.bin", tmp.path());
    let scores_b = format!("{}/scores_b.bin", tmp.path());
    let final_s = format!("{}/pageranks.bin.zst", tmp.path());

    csr::run_with(n as usize, &edges_s, &csr_off, &csr_e, &out_deg, &csr_dir).unwrap();
    init_scores(n as usize, &scores_a);
    let last =
        iterate::run_with(n as usize, &csr_off, &csr_e, &out_deg, &scores_a, &scores_b).unwrap();
    compress_scores(&last, n, &final_s);

    let all_urls: Vec<&str> = {
        let mut v: Vec<&str> = batch1
            .iter()
            .chain(batch2.iter())
            .flat_map(|(a, b)| [*a, *b])
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let scores = all_urls
        .iter()
        .filter_map(|&url| Some((url.to_string(), lookup_score(url, &id_map, &final_s)?)))
        .collect();

    (n, scores)
}

// ── Reference PageRank ────────────────────────────────────────────────────────

fn reference_pagerank(edges: &[(&str, &str)]) -> HashMap<String, f64> {
    const D: f64 = 0.85;
    let mut nodes: Vec<String> = Vec::new();
    for (a, b) in edges {
        for u in [a, b] {
            if !nodes.contains(&u.to_string()) {
                nodes.push(u.to_string());
            }
        }
    }
    let n = nodes.len();
    let idx: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, b) in edges {
        let ai = idx[*a];
        let bi = idx[*b];
        if ai != bi {
            incoming[bi].push(ai);
        }
    }
    for inc in &mut incoming {
        inc.sort_unstable();
        inc.dedup();
    }
    let mut od = vec![0usize; n];
    for dst in 0..n {
        for &src in &incoming[dst] {
            od[src] += 1;
        }
    }
    let mut pr = vec![1.0 / n as f64; n];
    for _ in 0..500 {
        let dangling: f64 = (0..n).filter(|&i| od[i] == 0).map(|i| pr[i]).sum();
        let mut new_pr = vec![(1.0 - D) / n as f64 + D * dangling / n as f64; n];
        for node in 0..n {
            for &src in &incoming[node] {
                new_pr[node] += D * pr[src] / od[src] as f64;
            }
        }
        let delta = new_pr
            .iter()
            .zip(&pr)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        pr = new_pr;
        if delta < 1e-12 {
            break;
        }
    }
    let sum: f64 = pr.iter().sum();
    nodes
        .iter()
        .enumerate()
        .map(|(i, u)| (u.clone(), pr[i] / sum))
        .collect()
}

fn assert_close(got: f32, exp: f32, label: &str) {
    let tol = (exp.abs() * 0.02_f32).max(5e-4);
    assert!(
        (got - exp).abs() < tol,
        "{label}: got {got:.6}, expected {exp:.6}"
    );
}

fn compare_to_reference(tmp: &TmpDir, edges: &[(&str, &str)]) {
    let (_, scores) = run_pipeline(tmp, edges);
    let ref_pr = reference_pagerank(edges);
    for (url, &exp) in &ref_pr {
        assert_close(*scores.get(url).unwrap_or(&0.0), exp as f32, url);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// HASHING
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn hash_self_loops_not_included() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(&conn, "c.txt", &[("A", "A"), ("A", "B"), ("B", "B")]);
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let mut pairs: Vec<(u64, u64)> = shards.iter().flat_map(|s| read_pairs_raw(s)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), 1, "only A->B should survive");
}

#[test]
fn hash_duplicate_edges_deduped_within_shard() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    let edges: Vec<(&str, &str)> = vec![("X", "Y"); 20];
    write_connections(&conn, "c.txt", &edges);
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let mut pairs: Vec<(u64, u64)> = shards.iter().flat_map(|s| read_pairs_raw(s)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), 1);
}

#[test]
fn hash_multiple_connection_files() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(&conn, "c1.txt", &[("A", "B")]);
    write_connections(&conn, "c2.txt", &[("B", "C"), ("C", "A")]);
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let mut pairs: Vec<(u64, u64)> = shards.iter().flat_map(|s| read_pairs_raw(s)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(pairs.len(), 3);
}

#[test]
fn hash_empty_connections_dir_returns_error() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    let result = hashing::run_in(&conn, &tmp.sub("edges"));
    assert!(result.is_err());
}

#[test]
fn hash_url_to_id_is_deterministic() {
    assert_eq!(
        url_to_id("https://example.com"),
        url_to_id("https://example.com")
    );
}

#[test]
fn hash_url_to_id_different_urls_differ() {
    assert_ne!(
        url_to_id("https://alpha.com"),
        url_to_id("https://beta.com")
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// MERGE
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn merge_first_run_no_existing_id_map() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(&conn, "c.txt", &[("A", "B"), ("B", "C")]);
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());
    let n = merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();
    assert_eq!(n, 3);
}

#[test]
fn merge_ids_are_unique() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(
        &conn,
        "c.txt",
        &[("P", "Q"), ("Q", "R"), ("R", "S"), ("S", "P")],
    );
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();
    let map = read_id_map(&id_map);
    assert_eq!(map.len(), 4);
    let mut ids: Vec<u64> = map.values().cloned().collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 4);
}

#[test]
fn merge_id_map_sorted_by_hash() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    let edges: Vec<(String, String)> = (0..20u32)
        .map(|i| (format!("url{i}"), format!("url{}", i + 1)))
        .collect();
    let edge_refs: Vec<(&str, &str)> = edges
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    write_connections(&conn, "c.txt", &edge_refs);
    let shards = hashing::run_in(&conn, &tmp.sub("edges")).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();

    let mut prev = 0u64;
    let mut dec = zstd_reader(&id_map).unwrap();
    let mut buf = [0u8; 16];
    while dec.read_exact(&mut buf).is_ok() {
        let hash = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        assert!(hash >= prev, "id_map not sorted: {} < {}", hash, prev);
        prev = hash;
    }
}

#[test]
fn merge_incremental_preserves_existing_ids() {
    let tmp = TmpDir::new();
    let conn1 = tmp.sub("conn1");
    write_connections(&conn1, "c.txt", &[("A", "B")]);
    let sh1 = hashing::run_in(&conn1, &tmp.sub("e1")).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    let n1 = merge::run_with(&sh1, &id_map, &tmp.sub("n1"), &tmp.sub("m1"), &total_nodes).unwrap();
    std::fs::write(&total_nodes, n1.to_string()).unwrap();

    let map_before = read_id_map(&id_map);

    let conn2 = tmp.sub("conn2");
    write_connections(&conn2, "c.txt", &[("C", "D")]);
    let sh2 = hashing::run_in(&conn2, &tmp.sub("e2")).unwrap();

    let n2 = merge::run_with(&sh2, &id_map, &tmp.sub("n2"), &tmp.sub("m2"), &total_nodes).unwrap();
    std::fs::write(&total_nodes, n2.to_string()).unwrap();

    let map_after = read_id_map(&id_map);

    for (hash, old_id) in &map_before {
        assert_eq!(
            map_after[hash], *old_id,
            "existing ID changed after incremental merge"
        );
    }
    assert_eq!(map_after.len(), 4);
}

#[test]
fn merge_overlapping_batches_no_duplicate_ids() {
    let tmp = TmpDir::new();
    let conn1 = tmp.sub("conn1");
    write_connections(&conn1, "c.txt", &[("A", "B"), ("B", "C")]);
    let sh1 = hashing::run_in(&conn1, &tmp.sub("e1")).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    let n1 = merge::run_with(&sh1, &id_map, &tmp.sub("n1"), &tmp.sub("m1"), &total_nodes).unwrap();
    std::fs::write(&total_nodes, n1.to_string()).unwrap();

    let conn2 = tmp.sub("conn2");
    write_connections(&conn2, "c.txt", &[("B", "C"), ("C", "D")]);
    let sh2 = hashing::run_in(&conn2, &tmp.sub("e2")).unwrap();

    let n2 = merge::run_with(&sh2, &id_map, &tmp.sub("n2"), &tmp.sub("m2"), &total_nodes).unwrap();
    std::fs::write(&total_nodes, n2.to_string()).unwrap();

    assert_eq!(n1, 3);
    assert_eq!(n2, 4);
    let map = read_id_map(&id_map);
    let mut ids: Vec<u64> = map.values().cloned().collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 4, "duplicate IDs detected");
}

// ═════════════════════════════════════════════════════════════════════════════
// TRANSLATE
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn translate_first_run_no_existing_edges() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(&conn, "c.txt", &[("A", "B"), ("B", "C"), ("C", "A")]);
    let edges_dir = tmp.sub("edges");
    let shards = hashing::run_in(&conn, &edges_dir).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    let _ = merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    translate::run_with(shards, &id_map, &edges_s, &edges_dir).unwrap();
    assert_eq!(read_pairs_zstd(&edges_s).len(), 3);
}

#[test]
fn translate_edges_are_sorted_and_deduped() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(
        &conn,
        "c.txt",
        &[("A", "B"), ("A", "B"), ("B", "C"), ("B", "C"), ("C", "A")],
    );
    let edges_dir = tmp.sub("edges");
    let shards = hashing::run_in(&conn, &edges_dir).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    let _ = merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    translate::run_with(shards, &id_map, &edges_s, &edges_dir).unwrap();

    let edges = read_pairs_zstd(&edges_s);
    assert_eq!(edges.len(), 3, "duplicates not removed");
    for w in edges.windows(2) {
        assert!(w[0] <= w[1], "edges not sorted");
    }
}

#[test]
fn translate_self_loops_absent_from_output() {
    let tmp = TmpDir::new();
    let conn = tmp.sub("conn");
    write_connections(&conn, "c.txt", &[("A", "A"), ("A", "B"), ("B", "A")]);
    let edges_dir = tmp.sub("edges");
    let shards = hashing::run_in(&conn, &edges_dir).unwrap();
    let id_map = format!("{}/id_map.bin.zst", tmp.path());
    let total_nodes = format!("{}/total_nodes.txt", tmp.path());

    let _ = merge::run_with(
        &shards,
        &id_map,
        &tmp.sub("nodes"),
        &tmp.sub("merged"),
        &total_nodes,
    )
    .unwrap();
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    translate::run_with(shards, &id_map, &edges_s, &edges_dir).unwrap();

    for (src, dst) in read_pairs_zstd(&edges_s) {
        assert_ne!(src, dst, "self-loop found in translated edges");
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// CSR
// ═════════════════════════════════════════════════════════════════════════════

fn build_csr_for_test(
    tmp: &TmpDir,
    n: u64,
    edges: &[(u64, u64)],
) -> (Vec<u64>, Vec<u64>, Vec<u32>) {
    let edges_s = format!("{}/edges.bin.zst", tmp.path());
    let csr_off = format!("{}/offsets.bin.zst", tmp.path());
    let csr_e = format!("{}/csr_edges.bin", tmp.path());
    let out_deg = format!("{}/out_degree.bin.zst", tmp.path());
    let scratch = tmp.sub("csr_scratch");

    let mut sorted = edges.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    write_pairs_zstd(&edges_s, &sorted);

    csr::run_with(n as usize, &edges_s, &csr_off, &csr_e, &out_deg, &scratch).unwrap();

    let offsets: Vec<u64> = {
        let mut v = Vec::new();
        let mut f = BufReader::new(File::open(&csr_off).unwrap());
        let mut buf = [0u8; 8];
        while f.read_exact(&mut buf).is_ok() {
            v.push(u64::from_le_bytes(buf));
        }
        v
    };

    let csr_edges: Vec<u64> = {
        let mut v = Vec::new();
        // Change from BufReader to zstd_reader
        let mut dec = zstd_reader(&csr_e).unwrap();
        let mut buf = [0u8; 8];
        while dec.read_exact(&mut buf).is_ok() {
            v.push(u64::from_le_bytes(buf));
        }
        v
    };

    let out_degree: Vec<u32> = {
        let mut v = Vec::new();
        let mut dec = zstd_reader(&out_deg).unwrap();
        let mut buf = [0u8; 4];
        while dec.read_exact(&mut buf).is_ok() {
            v.push(u32::from_le_bytes(buf));
        }
        v
    };
    (offsets, csr_edges, out_degree)
}

#[test]
fn csr_out_degree_correct() {
    let tmp = TmpDir::new();
    let (_, _, od) = build_csr_for_test(&tmp, 3, &[(0, 1), (0, 2), (1, 2)]);
    assert_eq!(od[0], 2);
    assert_eq!(od[1], 1);
    assert_eq!(od[2], 0);
}

#[test]
fn csr_in_neighbors_correct() {
    let tmp = TmpDir::new();
    let (offsets, csr_edges, _) = build_csr_for_test(&tmp, 3, &[(0, 2), (1, 2)]);
    let start = (offsets[2] / 8) as usize;
    let end = (offsets[3] / 8) as usize;
    let mut nbrs = csr_edges[start..end].to_vec();
    nbrs.sort_unstable();
    assert_eq!(nbrs, vec![0u64, 1u64]);
}

#[test]
fn csr_offsets_length_is_n_plus_one() {
    let tmp = TmpDir::new();
    let (offsets, _, _) = build_csr_for_test(&tmp, 5, &[(0, 1), (1, 2), (2, 3)]);
    assert_eq!(offsets.len(), 6);
}

#[test]
fn csr_total_edge_bytes_matches_last_offset() {
    let tmp = TmpDir::new();
    let edges = vec![(0u64, 1), (0, 2), (1, 2), (2, 0)];
    let (offsets, _, _) = build_csr_for_test(&tmp, 3, &edges);
    assert_eq!(offsets[3] / 8, edges.len() as u64);
}

#[test]
fn csr_dangling_node_has_zero_out_degree() {
    let tmp = TmpDir::new();
    let (_, _, od) = build_csr_for_test(&tmp, 3, &[(0, 2), (1, 2)]);
    assert_eq!(od[2], 0);
}

#[test]
fn csr_file_size_matches_offset_promise() {
    let tmp = TmpDir::new();
    // The assert inside csr::run_with will panic if sizes don't match.
    build_csr_for_test(&tmp, 3, &[(0, 1), (1, 2), (2, 0), (0, 2)]);
}

// ═════════════════════════════════════════════════════════════════════════════
// PAGERANK CORRECTNESS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn pagerank_mass_conservation() {
    let tmp = TmpDir::new();
    let (_, scores) = run_pipeline(
        &tmp,
        &[("A", "B"), ("B", "C"), ("C", "A"), ("A", "D"), ("D", "B")],
    );
    let sum: f32 = scores.values().sum();
    assert!((sum - 1.0).abs() < 1e-3, "mass not conserved: sum={sum}");
}

#[test]
fn pagerank_symmetric_mutual_links_equal() {
    let tmp = TmpDir::new();
    let (_, scores) = run_pipeline(&tmp, &[("A", "B"), ("B", "A")]);
    assert!((scores["A"] - scores["B"]).abs() < 5e-4);
}

#[test]
fn pagerank_star_leaves_equal() {
    let tmp = TmpDir::new();
    let (_, scores) = run_pipeline(&tmp, &[("H", "A"), ("H", "B"), ("H", "C"), ("H", "D")]);
    assert!((scores["A"] - scores["B"]).abs() < 5e-4);
    assert!((scores["A"] - scores["C"]).abs() < 5e-4);
    assert!((scores["A"] - scores["D"]).abs() < 5e-4);
}

#[test]
fn pagerank_high_indegree_node_ranks_highest() {
    let tmp = TmpDir::new();
    let edges = [
        ("A", "Z"),
        ("B", "Z"),
        ("C", "Z"),
        ("D", "Z"),
        ("E", "Z"),
        ("A", "B"),
        ("B", "C"),
    ];
    let (_, scores) = run_pipeline(&tmp, &edges);
    let z = scores["Z"];
    for (url, &sc) in &scores {
        if url != "Z" {
            assert!(z >= sc, "Z should rank highest but {url}={sc} > Z={z}");
        }
    }
}

#[test]
fn pagerank_self_loops_do_not_change_scores() {
    let tmp1 = TmpDir::new();
    let tmp2 = TmpDir::new();
    let (_, s1) = run_pipeline(&tmp1, &[("A", "B"), ("B", "C"), ("C", "A")]);
    let (_, s2) = run_pipeline(
        &tmp2,
        &[("A", "B"), ("B", "C"), ("C", "A"), ("A", "A"), ("B", "B")],
    );
    for url in ["A", "B", "C"] {
        assert_close(s1[url], s2[url], url);
    }
}

#[test]
fn pagerank_duplicate_edges_same_as_single() {
    let tmp1 = TmpDir::new();
    let tmp2 = TmpDir::new();
    let (_, s1) = run_pipeline(&tmp1, &[("A", "B"), ("B", "C"), ("C", "A")]);
    let (_, s2) = run_pipeline(
        &tmp2,
        &[
            ("A", "B"),
            ("A", "B"),
            ("B", "C"),
            ("B", "C"),
            ("C", "A"),
            ("C", "A"),
        ],
    );
    for url in ["A", "B", "C"] {
        assert_close(s1[url], s2[url], url);
    }
}

#[test]
fn pagerank_disconnected_components_all_positive() {
    let tmp = TmpDir::new();
    let (_, scores) = run_pipeline(&tmp, &[("A", "B"), ("B", "A"), ("C", "D"), ("D", "C")]);
    for (u, &s) in &scores {
        assert!(s > 0.0, "{u} should have positive score");
    }
}

#[test]
fn pagerank_disconnected_symmetric_components_equal() {
    let tmp = TmpDir::new();
    let (_, scores) = run_pipeline(&tmp, &[("A", "B"), ("B", "A"), ("C", "D"), ("D", "C")]);
    assert!((scores["A"] - scores["B"]).abs() < 5e-4);
    assert!((scores["C"] - scores["D"]).abs() < 5e-4);
}

#[test]
fn pagerank_all_dangling_still_sums_to_one() {
    let tmp = TmpDir::new();
    let (n, scores) = run_pipeline(&tmp, &[("SRC", "A"), ("SRC", "B"), ("SRC", "C")]);
    assert_eq!(n, 4);
    let sum: f32 = scores.values().sum();
    assert!((sum - 1.0).abs() < 1e-3, "sum={sum}");
}

#[test]
fn pagerank_single_edge() {
    let tmp = TmpDir::new();
    let (n, scores) = run_pipeline(&tmp, &[("A", "B")]);
    assert_eq!(n, 2);
    assert!(scores["A"] > 0.0 && scores["B"] > 0.0);
}

#[test]
fn pagerank_matches_reference_cycle() {
    let tmp = TmpDir::new();
    compare_to_reference(&tmp, &[("A", "B"), ("B", "C"), ("C", "A")]);
}

#[test]
fn pagerank_matches_reference_with_dangling() {
    let tmp = TmpDir::new();
    compare_to_reference(&tmp, &[("A", "B"), ("B", "C"), ("C", "A"), ("A", "D")]);
}

#[test]
fn pagerank_matches_reference_dense() {
    let tmp = TmpDir::new();
    compare_to_reference(
        &tmp,
        &[("A", "B"), ("A", "C"), ("B", "A"), ("B", "C"), ("C", "A")],
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// INCREMENTAL PIPELINE
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn incremental_node_count_disjoint_batches() {
    let tmp = TmpDir::new();
    let (n, _) = run_incremental(&tmp, &[("A", "B"), ("B", "C")], &[("D", "E")]);
    assert_eq!(n, 5);
}

#[test]
fn incremental_node_count_overlapping_batches() {
    let tmp = TmpDir::new();
    let (n, _) = run_incremental(&tmp, &[("A", "B")], &[("B", "A"), ("A", "C")]);
    assert_eq!(n, 3);
}

#[test]
fn incremental_duplicate_edges_across_batches_deduped() {
    let tmp = TmpDir::new();
    run_incremental(
        &tmp,
        &[("A", "B"), ("B", "C"), ("C", "A")],
        &[("A", "B"), ("B", "C"), ("C", "A")],
    );
    let edges_s = format!("{}/edges_sorted.bin.zst", tmp.path());
    assert_eq!(read_pairs_zstd(&edges_s).len(), 3);
}

#[test]
fn incremental_mass_conservation() {
    let tmp = TmpDir::new();
    let (_, scores) = run_incremental(
        &tmp,
        &[("A", "B"), ("B", "C"), ("C", "A"), ("A", "D")],
        &[("E", "A"), ("F", "B"), ("G", "C"), ("H", "D")],
    );
    let sum: f32 = scores.values().sum();
    assert!((sum - 1.0).abs() < 1e-3, "sum={sum}");
}

#[test]
fn incremental_all_urls_have_positive_scores() {
    let tmp = TmpDir::new();
    let (_, scores) = run_incremental(&tmp, &[("A", "B"), ("B", "C")], &[("D", "E"), ("E", "A")]);
    for url in ["A", "B", "C", "D", "E"] {
        assert!(scores.contains_key(url), "{url} missing");
        assert!(scores[url] > 0.0, "{url} score is zero");
    }
}

#[test]
fn incremental_scores_match_full_one_shot_run() {
    let tmp_inc = TmpDir::new();
    let tmp_full = TmpDir::new();
    let b1 = [("A", "B"), ("B", "C"), ("C", "A")];
    let b2 = [("A", "D"), ("D", "B"), ("E", "A")];
    let all: Vec<(&str, &str)> = b1.iter().chain(b2.iter()).copied().collect();

    let (_, scores_inc) = run_incremental(&tmp_inc, &b1, &b2);
    let (_, scores_full) = run_pipeline(&tmp_full, &all);

    for (url, &s_full) in &scores_full {
        let s_inc = *scores_inc.get(url).unwrap_or(&0.0);
        assert_close(s_inc, s_full, url);
    }
}

#[test]
fn incremental_id_collision_stress() {
    let tmp = TmpDir::new();
    let b1: Vec<(String, String)> = (0..50)
        .map(|i| (format!("alpha_{i}"), format!("alpha_{}", (i + 1) % 50)))
        .collect();
    let b2: Vec<(String, String)> = (0..50)
        .map(|i| (format!("beta_{i}"), format!("alpha_{}", i % 50)))
        .collect();
    let b1r: Vec<(&str, &str)> = b1.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let b2r: Vec<(&str, &str)> = b2.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let (_, scores) = run_incremental(&tmp, &b1r, &b2r);
    let sum: f32 = scores.values().sum();
    assert!(
        (sum - 1.0).abs() < 1e-3,
        "ID collision suspected — mass={sum}"
    );
}
