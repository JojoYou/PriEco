//! # PageRank Compute
//!
//! Adds new links to the link graph and computes PageRank of all pages.
//! Works on disk, the graph doesn't have to fit in RAM.
//!
//! ## Architecture
//!
//! 1. [**Import**:][import] When [CONNECTIONS_DIR] has new 'url->url' files.
//!     * [**Hashing**:][hashing::run] URLs to ids, sorted shards.
//!     * [**Merge**:][merge::run] New ids to id map, every page gets a node id.
//!     * [**Translate**:][translate::run] Edges to node ids, merged into [EDGES_SORTED].
//!     * `REBUILD_CSR` env skips import and uses existing graph.
//!     * Stops here when [IMPORT_ONLY_FILE] exists.
//! 2. [**CSR**:][crate::nodes::csr::run] Incoming edges per node and out degrees.
//! 3. [**Iterate**:][crate::iter::iterate::run] Power iteration.
//! 4. **Save:** Scores to [FINAL_SCORES], [PAGERANK] is reloaded.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-02-26
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] New non-flawed implementation

/*
  Import system libraries
*/
use std::{
    fs::{File, create_dir_all, read_dir, remove_file, rename},
    io::{BufReader, BufWriter, ErrorKind, Read, Write, copy},
    path::Path,
    sync::Arc,
};

/*
  Import external libraries
*/
use zstd::{Decoder, Encoder};

/*
  Import own libraries
*/
use crate::import::{hashing, merge, translate};
use prieco_core::{
    FINAL_SCORES, ID_MAP_FILE,
    globals::{PAGERANK, PageRank, colors, icons},
    read_file, write_file,
};

/*
  Constants
*/
/// Root of PageRank files.
pub const PAGERANK_DIR: &str = "runtime/pagerank";
/// When this file exists, links are only imported, PageRank isn't computed.
pub const IMPORT_ONLY_FILE: &str = "runtime/pagerank/skip.txt";

/// New link files, one 'from->to' per line. Removed after import.
pub const CONNECTIONS_DIR: &str = "runtime/pagerank/connections";
/// Pairs held in RAM before a shard is written (~500 MB).
pub const BUFFER_SIZE: usize = 31_250_000;

/// Temporary edge shards.
pub const EDGES_DIR: &str = "runtime/pagerank/edges";
/// Temporary node shards.
pub const NODES_DIR: &str = "runtime/pagerank/nodes";
/// Temporary merged id map.
pub const MERGED_DIR: &str = "runtime/pagerank/merged";

/// Whole graph, sorted (from, to) node id pairs.
pub const EDGES_SORTED: &str = "runtime/pagerank/edges_sorted.bin.zst";
/// CSR source nodes, grouped by destination.
pub const CSR_EDGES: &str = "runtime/pagerank/csr_edges.bin.zst";
/// CSR start of every node in [CSR_EDGES].
pub const CSR_OFFSETS: &str = "runtime/pagerank/csr_offsets.bin";
/// Scores, iterations switch between A and B.
pub const SCORES_A: &str = "runtime/pagerank/scores_a.bin";
/// Scores, iterations switch between A and B.
pub const SCORES_B: &str = "runtime/pagerank/scores_b.bin";
/// Number of nodes.
pub const TOTAL_NODES: &str = "runtime/pagerank/total_nodes.txt";
/// Outgoing links per node, u32.
pub const OUT_DEGREE: &str = "runtime/pagerank/out_degree.bin.zst";

/// Directories created on start.
pub const DIRS: [&str; 5] = [
    PAGERANK_DIR,
    CONNECTIONS_DIR,
    EDGES_DIR,
    NODES_DIR,
    MERGED_DIR,
];

/// [ID_MAP_FILE] loaded to RAM: sorted (url id, node id) pairs.
pub struct IdMap {
    /// Pairs sorted by url id.
    pairs: Vec<(u64, u64)>,
}

impl IdMap {
    /// Reads id map file.
    ///
    /// # Errors
    ///
    /// If file can't be opened.
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut f = BufReader::with_capacity(1 << 20, File::open(path)?);
        let mut pairs = Vec::new();
        let mut buf = [0u8; 16];

        while f.read_exact(&mut buf).is_ok() {
            let hash = u64::from_le_bytes(buf[0..8].try_into().unwrap());
            let id = u64::from_le_bytes(buf[8..16].try_into().unwrap());
            pairs.push((hash, id));
        }

        Ok(Self { pairs })
    }

    /// All pairs.
    pub fn pairs(&self) -> &[(u64, u64)] {
        &self.pairs
    }

    /// Node id of url id, binary search.
    pub fn lookup(&self, hash: u64) -> Option<u64> {
        self.pairs
            .binary_search_by_key(&hash, |&(h, _)| h)
            .ok()
            .map(|i| self.pairs[i].1)
    }
}

/// Imports new links and computes PageRank.
///
/// Does nothing without new links, unless `REBUILD_CSR` is set.
/// New links remove old CSR and scores, they are rebuilt.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// None. Errors are printed.
///
/// # Panics
///
/// If score files can't be created, copied or renamed.
pub fn run() {
    // Create required directories
    for dir in DIRS {
        if let Err(e) = create_dir_all(dir) {
            println!(
                "{}: {}{}{}",
                icons::PAGERANK_ICON,
                colors::RED,
                e,
                colors::RESET
            );
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
        icons::PAGERANK_ICON,
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
                println!("{}: {}", icons::PAGERANK_ICON, e);
                return;
            }
        };

        // Invalidate stale CSR and scores
        let _ = remove_file(CSR_EDGES);
        let _ = remove_file(SCORES_A);
        let _ = remove_file(SCORES_B);

        write_file(TOTAL_NODES, total_nodes.to_string(), false);
    } else if has_new_connections && !has_existing_edges {
        total_nodes = match import() {
            Ok(c) => c,
            Err(e) => {
                println!("{}: {}", icons::PAGERANK_ICON, e);
                return;
            }
        };
        write_file(TOTAL_NODES, total_nodes.to_string(), false);
    } else if std::env::var("REBUILD_CSR").is_ok() {
        total_nodes = match read_file(TOTAL_NODES).trim().parse() {
            Ok(t) => t,
            Err(e) => {
                println!(
                    "{}Failed to read total nodes:{} {}",
                    colors::RED,
                    colors::RESET,
                    e
                );
                return;
            }
        };
    } else {
        println!("No new work!");
        return;
    }
    println!(
        "{}: {}Import done!{}",
        icons::PAGERANK_ICON,
        colors::GREEN,
        colors::RESET
    );

    // Import-only more = we are done, import was enough
    if import_only {
        println!(
            "{}: Pagerank ran in import-only mode. Delete {} to compute PageRank.",
            icons::PAGERANK_ICON,
            IMPORT_ONLY_FILE
        );
        return;
    }

    /*
      2: Build graph
    */
    println!(
        "{}: Phase 2: building CSR ({} nodes)…",
        icons::PAGERANK_ICON,
        total_nodes
    );
    let total_nodes_usize = total_nodes as usize;
    if !Path::new(CSR_EDGES).exists() {
        match crate::nodes::csr::run(total_nodes_usize) {
            Ok(_) => (),
            Err(e) => {
                println!("{}: {}", icons::PAGERANK_ICON, e);
                return;
            }
        };
    } else {
        println!("{}: Skipping Phase 2", icons::PAGERANK_ICON);
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
    println!(
        "{}: Phase 3: streaming power iteration…",
        icons::PAGERANK_ICON
    );
    let final_file: String = match crate::iter::iterate::run(total_nodes_usize) {
        Ok(file) => file,
        Err(e) => {
            println!("{}: {}", icons::PAGERANK_ICON, e);
            return;
        }
    };

    // Save
    println!("{}: Saving {} scores…", icons::PAGERANK_ICON, total_nodes);
    const TMP: &str = "runtime/pagerank/pageranks.bin.tmp";
    let mut rdr = BufReader::with_capacity(1_048_576, File::open(final_file).unwrap());
    let mut out = BufWriter::with_capacity(1_048_576, File::create(TMP).unwrap());
    copy(&mut rdr, &mut out).unwrap();
    drop(out);
    rename(TMP, FINAL_SCORES).unwrap();
    println!(
        "{}: {}Saved!{}",
        icons::PAGERANK_ICON,
        colors::GREEN,
        colors::RESET
    );

    *PAGERANK.write() = PageRank::open(ID_MAP_FILE, FINAL_SCORES).ok().map(Arc::new);

    let google_score = PAGERANK
        .read()
        .as_ref()
        .map(|pr| pr.get_score("https://www.google.com/"))
        .unwrap_or(0.0);

    println!("{}: Google: {}", icons::PAGERANK_ICON, google_score);
}

/* Caller functions */
/// Runs the 3 import passes in order.
///
/// # Returns
///
/// Total number of nodes.
///
/// # Errors
///
/// If a pass fails.
fn import() -> Result<u64, Box<dyn std::error::Error>> {
    println!("{}: Phase 1: Pass A: Hashing!", icons::PAGERANK_ICON);
    let hash_shards = hashing::run()?;

    println!("{}: Phase 1: Pass B: Merging!", icons::PAGERANK_ICON);
    let total_nodes = merge::run(&hash_shards)?;

    println!(
        "{}: Phase 1: Pass C: Translating edges!",
        icons::PAGERANK_ICON
    );
    translate::run(hash_shards)?;

    Ok(total_nodes)
}

/* Helper functions */
/// Creates zstd level 19 writer. PageRank files are zstd wherever possible to save storage.
///
/// # Errors
///
/// If file can't be created.
pub fn zstd_writer(
    path: &str,
) -> Result<
    zstd::stream::write::AutoFinishEncoder<'static, BufWriter<File>>,
    Box<dyn std::error::Error>,
> {
    let f = File::create(path)?;
    let writer = BufWriter::with_capacity(1 << 20, f);

    let encoder = Encoder::new(writer, 19)?.auto_finish(); // Lvl 19 compression

    Ok(encoder)
}

/// Creates zstd reader.
///
/// # Errors
///
/// If file can't be opened.
pub fn zstd_reader(
    path: &str,
) -> Result<Decoder<'static, BufReader<File>>, Box<dyn std::error::Error>> {
    let f = File::open(path)?;
    let decoder = Decoder::new(f)?;

    Ok(decoder)
}

/// Reads one (u64, u64) pair from zstd stream.
///
/// # Returns
///
/// `None` at end of stream.
///
/// # Errors
///
/// On read error other than end of file.
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

/// Reads one (u64, u64) little endian pair.
///
/// # Returns
///
/// `None` at end of file.
///
/// # Errors
///
/// On read error other than end of file.
pub fn read_u64_pair(
    r: &mut BufReader<File>,
) -> Result<Option<(u64, u64)>, Box<dyn std::error::Error>> {
    let mut buf = [0u8; 16];
    match r.read_exact(&mut buf) {
        Ok(_) => Ok(Some((
            u64::from_le_bytes(buf[0..8].try_into()?),
            u64::from_le_bytes(buf[8..16].try_into()?),
        ))),
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}
