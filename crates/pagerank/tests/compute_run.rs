// tests/compute_run.rs
//
// End to end test of prieco_pagerank::run() with its real relative paths
// ("pagerank/..."), executed inside a private temp working directory.
// Single test: cwd and env are process wide.

use std::{fs, path::Path};

use prieco_core::{FINAL_SCORES, ID_MAP_FILE, PAGERANK};
use prieco_pagerank::compute::{
    CONNECTIONS_DIR, CSR_EDGES, CSR_OFFSETS, DIRS, EDGES_SORTED, IMPORT_ONLY_FILE, IdMap,
    OUT_DEGREE, SCORES_A, TOTAL_NODES,
};

fn scores() -> Vec<f32> {
    fs::read(FINAL_SCORES)
        .unwrap()
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn total_nodes() -> u64 {
    fs::read_to_string(TOTAL_NODES)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn connections(name: &str, links: &[(&str, &str)]) {
    let text: String = links
        .iter()
        .map(|(a, b)| format!("{}->{}\n", a, b))
        .collect();
    fs::write(format!("{}/{}", CONNECTIONS_DIR, name), text).unwrap();
}

fn score(url: &str) -> f32 {
    PAGERANK
        .read()
        .as_ref()
        .expect("PAGERANK not loaded")
        .get_score(url)
}

#[test]
fn run_full_incremental_import_only_and_rebuild_cycle() {
    let root = std::env::temp_dir().join(format!("prieco_pagerank_run_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    std::env::set_current_dir(&root).unwrap();
    // SAFETY: single test in this binary
    unsafe { std::env::remove_var("REBUILD_CSR") };
    unsafe { std::env::remove_var("REUSE_SHARDS") };

    /* 0: nothing to do creates the directory layout only */
    prieco_pagerank::run();
    for d in DIRS {
        assert!(Path::new(d).is_dir(), "{} not created", d);
    }
    assert!(!Path::new(FINAL_SCORES).exists());

    /* 1: first import computes scores */
    connections(
        "batch1.txt",
        &[
            ("https://a.com", "https://b.com"),
            ("https://b.com", "https://c.com"),
            ("https://c.com", "https://a.com"),
            ("https://d.com", "https://a.com"),
        ],
    );
    prieco_pagerank::run();
    assert_eq!(total_nodes(), 4);
    assert_eq!(IdMap::open(ID_MAP_FILE).unwrap().pairs().len(), 4);
    for f in [
        EDGES_SORTED,
        CSR_EDGES,
        CSR_OFFSETS,
        OUT_DEGREE,
        SCORES_A,
        FINAL_SCORES,
    ] {
        assert!(Path::new(f).exists(), "{} missing", f);
    }
    let s = scores();
    assert_eq!(s.len(), 4);
    let sum: f32 = s.iter().sum();
    assert!((sum - 1.0).abs() < 1e-3, "sum {}", sum);
    assert_eq!(
        fs::read_dir(CONNECTIONS_DIR).unwrap().count(),
        0,
        "connections consumed"
    );

    // the global reader was reloaded and answers lookups
    assert!(score("https://a.com") > score("https://d.com"));
    assert!(score("https://a.com/") > 0.0, "normalised lookup");
    assert_eq!(score("https://unknown.example"), 0.0);
    let before = fs::read(FINAL_SCORES).unwrap();

    /* 2: no new connections -> no work, nothing changes */
    prieco_pagerank::run();
    assert_eq!(fs::read(FINAL_SCORES).unwrap(), before);
    assert_eq!(total_nodes(), 4);

    /* 3: incremental batch adds nodes and recomputes */
    connections(
        "batch2.txt",
        &[
            ("https://e.com", "https://a.com"),
            ("https://a.com", "https://e.com"),
        ],
    );
    prieco_pagerank::run();
    assert_eq!(total_nodes(), 5);
    assert_eq!(scores().len(), 5);
    let sum: f32 = scores().iter().sum();
    assert!((sum - 1.0).abs() < 1e-3, "sum {}", sum);
    assert!(score("https://e.com") > 0.0);
    // ids of existing nodes are stable
    let m = IdMap::open(ID_MAP_FILE).unwrap();
    let ids: std::collections::HashSet<u64> = m.pairs().iter().map(|p| p.1).collect();
    assert_eq!(ids, (0..5).collect::<std::collections::HashSet<u64>>());

    /* 4: import-only mode imports but does not recompute */
    fs::write(IMPORT_ONLY_FILE, "").unwrap();
    let scores_before = fs::read(FINAL_SCORES).unwrap();
    connections("batch3.txt", &[("https://f.com", "https://a.com")]);
    prieco_pagerank::run();
    assert_eq!(total_nodes(), 6);
    assert_eq!(
        fs::read(FINAL_SCORES).unwrap(),
        scores_before,
        "scores must not be recomputed"
    );
    assert!(
        !Path::new(CSR_EDGES).exists(),
        "stale CSR must be invalidated"
    );

    /* 5: REBUILD_CSR recomputes from the imported graph */
    fs::remove_file(IMPORT_ONLY_FILE).unwrap();
    unsafe { std::env::set_var("REBUILD_CSR", "1") };
    prieco_pagerank::run();
    unsafe { std::env::remove_var("REBUILD_CSR") };
    assert_eq!(scores().len(), 6);
    assert!(score("https://f.com") > 0.0);
    let sum: f32 = scores().iter().sum();
    assert!((sum - 1.0).abs() < 1e-3, "sum {}", sum);

    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = fs::remove_dir_all(&root);
}
