/*
  File: pagerank/compute.rs
  Description: Import & Calculate pagerank on a new connections

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created:: 2026-02-26
  Last Modified: 2026-02-26

  Usage: Call to process connections in
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    fs::{File, create_dir_all, read_dir, remove_file},
    io::{BufReader, BufWriter, ErrorKind, Read, Write},
    path::Path,
};

/*
  Import external libraries
*/
use zstd::{Decoder, Encoder};

/*
  Import own libraries
*/
use crate::{
    globals::{colors, icons},
    pagerank::{import, iter, nodes},
    write_file,
};

/*
  Constants
*/
pub const PAGERANK_DIR: &str = "pagerank";
pub const IMPORT_ONLY_FILE: &str = "pagerank/skip.txt";

pub const CONNECTIONS_DIR: &str = "pagerank/connections";
pub const BUFFER_SIZE: usize = 8_000_000;

pub const EDGES_DIR: &str = "pagerank/edges";
pub const NODES_DIR: &str = "pagerank/nodes";
pub const MERGED_DIR: &str = "pagerank/merged";

pub const EDGES_SORTED: &str = "pagerank/edges_sorted.bin.zst"; // Sorted deduplicated (u64 src_id, u64 dst_id) edge list.
pub const CSR_EDGES: &str = "pagerank/csr_edges.bin";
pub const CSR_OFFSETS: &str = "pagerank/csr_offsets.bin.zst"; // u64 per node: byte offset into csr_edges where in-neighbors start.
pub const ID_MAP_FILE: &str = "pagerank/id_map.bin.zst"; // Sorted (u64 hash, u64 id) pairs — binary-searchable URL→ID lookup.
pub const SCORES_A: &str = "pagerank/scores_a.bin";
pub const SCORES_B: &str = "pagerank/scores_b.bin";
const TOTAL_NODES: &str = "pagerank/total_nodes.txt";
pub const OUT_DEGREE: &str = "pagerank/out_degree.bin.zst";
pub const FINAL_SCORES: &str = "pagerank/pageranks.bin.zst";
pub const TMP_SCORES: &str = "pagerank/pageranks.bin.tmp_lookup";

pub const DIRS: [&str; 5] = [
    PAGERANK_DIR,
    CONNECTIONS_DIR,
    EDGES_DIR,
    NODES_DIR,
    MERGED_DIR,
];

/*
  Description: Decide how to proceed and call responsible functions to import connections to the graph and compute pagerank

  Input: None
  Output: None
*/
pub fn run() {
    // Create required directories
    for dir in DIRS {
        if let Err(e) = create_dir_all(dir) {
            println!("{}{}{}", colors::RED, e, colors::RESET);
            return;
        }
    }

    // Rules
    // Do you want to just import new connections to the graph without calculating new pageranks score?
    // Make sure the file exists when this fn is called
    let import_only = Path::new(IMPORT_ONLY_FILE).exists();
    // Check if connection dir has new connections to process
    let has_new_connections = Path::new(CONNECTIONS_DIR).exists()
        && read_dir(CONNECTIONS_DIR)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
    // Check if graph already exists
    let has_existing_edges = Path::new(EDGES_SORTED).exists();

    println!(
        "{}: Starting\nImport-only mode: {}",
        icons::PAGERANK,
        import_only
    );

    /*
      1: Import connections
    */
    let total_nodes: u64;
    // Decide which path to take
    if has_new_connections && has_existing_edges {
        total_nodes = match import() {
            Ok(c) => c,
            Err(e) => {
                println!("{}: {}", icons::PAGERANK, e);
                return;
            }
        };

        // Invalidate stale CSR and scores
        let _ = remove_file(CSR_EDGES);
        let _ = remove_file(SCORES_A);
        let _ = remove_file(SCORES_B);

        write_file(TOTAL_NODES, &total_nodes.to_string(), false);
    } else if has_new_connections && !has_existing_edges {
        total_nodes = match import() {
            Ok(c) => c,
            Err(e) => {
                println!("{}: {}", icons::PAGERANK, e);
                return;
            }
        };
        write_file(TOTAL_NODES, &total_nodes.to_string(), false);
    } else {
        println!(
            "{}: {}No new work!{}",
            icons::PAGERANK,
            colors::YELLOW,
            colors::RESET
        );
        return;
    }
    println!(
        "{}: {}Import done!{}",
        icons::PAGERANK,
        colors::GREEN,
        colors::RESET
    );

    // Import-only more = we are done, import was enough
    if import_only {
        println!(
            "{}: Pagerank ran in import-only mode. Delete {} to compute PageRank.",
            icons::PAGERANK,
            IMPORT_ONLY_FILE
        );
        return;
    }

    /*
      2: Build graph
    */
    println!(
        "{}: Phase 2: building CSR ({} nodes)…",
        icons::PAGERANK,
        total_nodes
    );
    let total_nodes_usize = total_nodes as usize;
    if !Path::new(CSR_EDGES).exists() {
        match nodes::csr::run(total_nodes_usize) {
            Ok(_) => (),
            Err(e) => {
                println!("{}: {}", icons::PAGERANK, e);
                return;
            }
        };
    } else {
        println!("{}: Skipping Phase 2", icons::PAGERANK);
    }

    // Initial scores
    if !Path::new(SCORES_A).exists() {
        let init = 1.0f32 / total_nodes_usize as f32;
        let mut f = BufWriter::with_capacity(1 << 20, File::create(SCORES_A).unwrap());
        for _ in 0..total_nodes_usize {
            f.write_all(&init.to_le_bytes()).unwrap();
        }
    }

    /*
      3: Iterate
    */
    println!("{}: Phase 3: streaming power iteration…", icons::PAGERANK);
    let final_file: String = match iter::iterate::run(total_nodes_usize) {
        Ok(file) => file,
        Err(e) => {
            println!("{}: {}", icons::PAGERANK, e);
            return;
        }
    };

    // Save
    println!("{}: Compressing {} scores…", icons::PAGERANK, total_nodes);
    let mut rdr = BufReader::with_capacity(1_048_576, File::open(final_file).unwrap());
    let mut enc = match zstd_writer(FINAL_SCORES) {
        Ok(e) => e,
        Err(e) => {
            println!("{}: {}", icons::PAGERANK, e);
            return;
        }
    };
    let mut buf = [0u8; 4];
    for _ in 0..total_nodes {
        rdr.read_exact(&mut buf).unwrap();
        enc.write_all(&buf).unwrap();
    }
    if let Err(e) = enc.finish() {
        println!("{}: {}", icons::PAGERANK, e);
        return;
    };
    println!(
        "{}: {}Saved!{}",
        icons::PAGERANK,
        colors::GREEN,
        colors::RESET
    );
}

/* Caller functions */
/*
  Description: Import needed to be sharded to 3 files for readability. They are called in this order always together, created a fn for it

  Input: None
  Output: Result total_nodes
*/
fn import() -> Result<u64, Box<dyn std::error::Error>> {
    println!("{}: Phase 1: Pass A: Hashing!", icons::PAGERANK);
    let hash_shards = import::hashing::run()?;

    println!("{}: Phase 1: Pass B: Merging!", icons::PAGERANK);
    let total_nodes = import::merge::run(&hash_shards)?;

    println!("{}: Phase 1: Pass C: Translating edges!", icons::PAGERANK);
    import::translate::run(hash_shards)?;

    Ok(total_nodes)
}

/* Helper functions */
/*
  Description: Create zstd writer, we use zstd pagerank files whereever possible to save storage

  Input: None
  Output: Result zstd encoder/writer
*/
pub fn zstd_writer(
    path: &str,
) -> Result<Encoder<'static, BufWriter<File>>, Box<dyn std::error::Error>> {
    let f = File::create(path)?;
    let writer = BufWriter::with_capacity(1 << 20, f);
    let encoder = Encoder::new(writer, 19)?;

    Ok(encoder)
}

/*
  Description: Create zstd reader, we use zstd pagerank files whereever possible to save storage

  Input: None
  Output: Result zstd decoder/reader
*/
pub fn zstd_reader(
    path: &str,
) -> Result<Decoder<'static, BufReader<File>>, Box<dyn std::error::Error>> {
    let f = File::open(path)?;
    let decoder = Decoder::new(f)?;

    Ok(decoder)
}

/*
  Description: Read url->url pairs from the zstd graph

  Input: None
  Output: Result Option hashed url-hashed url pair
*/
pub fn read_u64_pair_zstd(
    r: &mut Decoder<BufReader<File>>,
) -> Result<Option<(u64, u64)>, Box<dyn std::error::Error>> {
    let mut buf = [0u8; 16];

    match r.read_exact(&mut buf) {
        Ok(_) => {
            let a = u64::from_le_bytes(buf[0..8].try_into()?);
            let b = u64::from_le_bytes(buf[8..16].try_into()?);
            Ok(Some((a, b)))
        }
        Err(e) => {
            if e.kind() == ErrorKind::UnexpectedEof {
                Ok(None)
            } else {
                Err(Box::new(e))
            }
        }
    }
}
