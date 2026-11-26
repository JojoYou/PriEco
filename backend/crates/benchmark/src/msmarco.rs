//! # MS MARCO Benchmark
//!
//! Builds an own [PriecoIndex] from MS MARCO Document (~3.2M web pages)
//! with the normal PriEco inserter. Searching is done by the web crate,
//! this file prepares the index, the queries and writes TREC run files.
//!
//! ## Architecture
//!
//! 1. [**Prepare**:][prepare] Download and index, returns the opened index.
//!     * [**Download**:][download::run] Corpus, queries and qrels into [MSMARCO_DIR].
//!     * [**Index**:][build_index] Once, until [MSMARCO_INDEXED_FILE] exists.
//!     * [**Convert**:][convert] Streams 'msmarco-docs.tsv.gz' into inserter '.zip' files.
//! 2. [**Queries**:][load_queries] Judged queries of each [QUERY_SETS] entry.
//! 3. [**Docids**:][load_docids] Result url → MS MARCO docid.
//! 4. [**Run file**:][write_run] Ranked docids in TREC format for scoring.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-22
//! * Last Modified: 2026-11-23
//!
//! ## Planned Improvements
//!
//! - [ ] Document vectors (now Tantivy only, vectors are zero and skipped)
//! - [ ] nDCG@10, MRR@10, Recall
//! - [ ] MS MARCO Passage

/*
  Import system libraries
*/
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{File, create_dir_all, read_to_string, remove_dir_all, write},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

/*
  Import external libraries
*/
use flate2::read::GzDecoder;
use parking_lot::Mutex;
use tokio::task::spawn_blocking;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/*
  Import own libraries
*/
use crate::{
    constants::{
        MSMARCO_DIR, MSMARCO_DOCIDS_FILE, MSMARCO_IMPORT_DIR, MSMARCO_INDEX_DIR,
        MSMARCO_INDEXED_FILE,
    },
    download,
};
use prieco_core::{
    PriecoIndex, TANTIVY_HEAP_SIZE, TANTIVY_WRITER_THREADS, VECTOR_DIM, colors, file_exists,
    globals::icons, url_to_id,
};
use prieco_insert::db_insert;

/*
  Constants
*/
/// Everything downloaded into [MSMARCO_DIR].
const FILES: &[&str] = &[
    // Corpus: docid \t url \t title \t body
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docs.tsv.gz",
    // Dev: sparse qrels, for MRR@10
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docdev-queries.tsv.gz",
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docdev-qrels.tsv.gz",
    // TREC DL: graded 0-3 qrels, for nDCG@10. Queries are shared by document and passage
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2019-queries.tsv.gz",
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2020-queries.tsv.gz",
    "https://trec.nist.gov/data/deep/2019qrels-docs.txt",
    "https://trec.nist.gov/data/deep/2020qrels-docs.txt",
    // Passage: corpus + queries + dev qrels (~1 GB tar.gz)
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/collectionandqueries.tar.gz",
    "https://trec.nist.gov/data/deep/2019qrels-pass.txt",
    "https://trec.nist.gov/data/deep/2020qrels-pass.txt",
];
/// Document corpus: docid, url, title, body.
const CORPUS_FILE: &str = "msmarco-docs.tsv.gz";

/// Documents per inserter '.zip'.
const DOCS_PER_ZIP: usize = 100_000;
/// Body characters used as description.
const DESCRIPTION_CHARS: usize = 300;

/// Run name in TREC run files.
const RUN_TAG: &str = "prieco";

/// TREC DL query: name, queries file, qrels file
pub const QUERY_SETS: &[(&str, &str, &str)] = &[
    (
        "trec_dl_2019",
        "msmarco-test2019-queries.tsv.gz",
        "2019qrels-docs.txt",
    ),
    (
        "trec_dl_2020",
        "msmarco-test2020-queries.tsv.gz",
        "2020qrels-docs.txt",
    ),
];

/// Downloads MS MARCO and opens its index, building it on the first run.
///
/// Indexing is heavy disk work, so it runs on a blocking thread.
/// Later runs find everything cached and only open the index.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// The opened benchmark index.
///
/// `None` when the download or indexing failed, the error is printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub async fn prepare() -> Option<Arc<PriecoIndex>> {
    // Download dataset
    // May be already cached
    if !download::run(MSMARCO_DIR, FILES).await {
        return None;
    }

    // Open index
    let opened = spawn_blocking(|| -> Result<PriecoIndex, String> {
        let index = PriecoIndex::open(MSMARCO_INDEX_DIR);
        if !file_exists(MSMARCO_INDEXED_FILE) {
            build_index(&index).map_err(|e| e.to_string())?;
        }
        Ok(index)
    })
    .await;

    match opened {
        Ok(Ok(index)) => Some(Arc::new(index)),
        Ok(Err(e)) => {
            eprintln!(
                "{}{}: Failed to build MS MARCO index: {}{}",
                icons::BENCHMARK,
                colors::RED,
                e,
                colors::RESET,
            );
            None
        }
        Err(e) => {
            eprintln!(
                "{}{}: MS MARCO index task crashed: {}{}",
                icons::BENCHMARK,
                colors::RED,
                e,
                colors::RESET,
            );
            None
        }
    }
}

/// Loads the queries of a query set that have relevance judgments.
///
/// TREC (Text REtrieval Conference) is NIST's yearly search evaluation.
/// Its Deep Learning (DL) track reuses MS MARCO with human graded judgments.
///
/// TREC DL has 200 test queries per year, but only ~45 are judged.
/// Unjudged queries can't be scored, so they are skipped.
///
/// # Arguments
///
/// * `queries_file` - 'qid \t query' '.tsv.gz' in [MSMARCO_DIR].
/// * `qrels_file` - 'qid Q0 docid relevance' file in [MSMARCO_DIR].
///
/// # Returns
///
/// Vector of (qid, query).
///
/// An empty vector when a file can't be read, the error is printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub fn load_queries(queries_file: &str, qrels_file: &str) -> Vec<(String, String)> {
    match read_queries(queries_file, qrels_file) {
        Ok(queries) => queries,
        Err(e) => {
            eprintln!(
                "{}{}: Failed to load {}: {}{}",
                icons::BENCHMARK,
                colors::RED,
                queries_file,
                e,
                colors::RESET,
            );
            Vec::new()
        }
    }
}

/// Loads the url → docid map written by [convert].
///
/// A docid is the MS MARCO id of a document, e.g. 'D1555982'.
/// Qrels and run files name documents only by docid, never by url.
///
/// Keys are [url_to_id] hashes, smaller than 3.2M url strings.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// Hashmap of url id and MS MARCO docid.
///
/// An empty map when [MSMARCO_DOCIDS_FILE] can't be read, the error is printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub fn load_docids() -> HashMap<u64, String> {
    let file = match File::open(MSMARCO_DOCIDS_FILE) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "{}{}: Failed to open {}: {}{}",
                icons::BENCHMARK,
                colors::RED,
                MSMARCO_DOCIDS_FILE,
                e,
                colors::RESET,
            );
            return HashMap::new();
        }
    };

    let mut docids: HashMap<u64, String> = HashMap::with_capacity(3_300_000);
    for line in BufReader::with_capacity(1 << 20, file)
        .lines()
        .map_while(Result::ok)
    {
        if let Some((url, docid)) = line.split_once('\t') {
            docids.insert(url_to_id(url), docid.to_string());
        }
    }

    docids
}

/// Writes ranked results as a TREC run file.
///
/// A run file is the standard results format of TREC: the ranking of a system per query.
/// Tools like 'trec_eval' compare it against qrels to compute nDCG@10 or MRR@10.
///
/// One line per result: 'qid Q0 docid rank score prieco'.
/// Results without a docid are skipped, a docid only counts once per query.
///
/// # Arguments
///
/// * `name` - Query set name, the file is 'run_<name>.trec' in [MSMARCO_DIR].
/// * `results` - (qid, ranked (url, score)) per query.
/// * `docids` - Map from [load_docids].
///
/// # Returns
///
/// None. Errors are printed.
///
/// # Panics
///
/// Only if system runs out of memory.
pub fn write_run(
    name: &str,
    results: &[(String, Vec<(String, f32)>)],
    docids: &HashMap<u64, String>,
) {
    let path = format!("{}/run_{}.trec", MSMARCO_DIR, name);

    if let Err(e) = write_run_file(&path, results, docids) {
        eprintln!(
            "{}{}: Failed to write {}: {}{}",
            icons::BENCHMARK,
            colors::RED,
            path,
            e,
            colors::RESET,
        );
        return;
    }

    println!(
        "{}{}: Wrote {}!{}",
        icons::BENCHMARK,
        colors::GREEN,
        path,
        colors::RESET,
    );
}

/* Helper functions */

/// Reads queries that have judgements.
///
/// Qrels (query relevance judgments) are human ratings, 'qid Q0 docid relevance' per line.
/// TREC DL grades 0 (irrelevant) to 3 (perfect), MS MARCO dev only marks relevant ones.
///
/// # Arguments
///
/// * `queries_file` - 'qid \t query' '.tsv.gz' in [MSMARCO_DIR].
/// * `qrels_file` - Qrels in [MSMARCO_DIR], first column is qid.
///
/// # Returns
///
/// Vector of (qid, query).
///
/// # Errors
///
/// If a file can't be read.
fn read_queries(
    queries_file: &str,
    qrels_file: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error + Send + Sync>> {
    // Judged query ids
    let qrels = read_to_string(Path::new(MSMARCO_DIR).join(qrels_file))?;
    let judged: HashSet<&str> = qrels
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();

    // Queries
    let file = File::open(Path::new(MSMARCO_DIR).join(queries_file))?;
    let mut queries: Vec<(String, String)> = Vec::with_capacity(judged.len());
    for line in BufReader::new(GzDecoder::new(file)).lines() {
        let line = line?;
        if let Some((qid, query)) = line.split_once('\t')
            && judged.contains(qid)
        {
            queries.push((qid.to_string(), query.to_string()));
        }
    }

    Ok(queries)
}

/// Writes TREC run file, 'qid Q0 docid rank score prieco' per line.
///
/// 'Q0' is a fixed legacy column, ignored by scorers.
/// Scorers sort by score, not rank, so higher score must mean more relevant.
///
/// Results without MS MARCO docid are skipped, every docid once per query.
/// Written to '.tmp' first, then renamed.
///
/// # Errors
///
/// If file can't be written.
fn write_run_file(
    path: &str,
    results: &[(String, Vec<(String, f32)>)],
    docids: &HashMap<u64, String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let tmp_path = format!("{}.tmp", path);
    let mut out = BufWriter::new(File::create(&tmp_path)?);

    for (qid, ranked) in results {
        let mut seen: HashSet<&str> = HashSet::with_capacity(ranked.len());
        let mut rank: usize = 0;

        for (url, score) in ranked {
            let Some(docid) = docids.get(&url_to_id(url)) else {
                continue;
            };
            if !seen.insert(docid.as_str()) {
                continue;
            }

            rank += 1;
            writeln!(out, "{} Q0 {} {} {} {}", qid, docid, rank, score, RUN_TAG)?;
        }
    }

    out.flush()?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Converts the corpus and inserts it into the benchmark index.
///
/// The index is local, the official index and its globals are never touched.
/// When done, [MSMARCO_INDEXED_FILE] is created so it runs only once.
///
/// # Arguments
///
/// * `index` - Opened benchmark index.
///
/// # Returns
///
/// `Ok(())` when the index is complete.
///
/// # Errors
///
/// * Corpus can't be read or converted.
/// * Tantivy writer can't be created.
/// * Inserter failed.
///
/// # Panics
///
/// If the index can't be opened. Start-up only.
fn build_index(index: &PriecoIndex) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = Instant::now();

    // Convert corpus to inserter format
    let converted = convert()?;

    if converted == 0 {
        return Err(format!("No documents converted from {}", CORPUS_FILE).into());
    }

    println!(
        "{}: Converted {} docs in {:.3}s",
        icons::BENCHMARK,
        converted,
        start.elapsed().as_secs_f64(),
    );

    // Own writer
    let writer = Mutex::new(
        index
            .tantivy
            .writer_with_num_threads(TANTIVY_WRITER_THREADS, TANTIVY_HEAP_SIZE)?,
    );

    // Fill index
    db_insert::run_in(MSMARCO_IMPORT_DIR, index, &writer)?;

    // Mark as done
    write(MSMARCO_INDEXED_FILE, "")?;

    println!(
        "{}{}: MS MARCO indexed in {:.3}s!{}",
        icons::BENCHMARK,
        colors::GREEN,
        start.elapsed().as_secs_f64(),
        colors::RESET,
    );

    Ok(())
}

/// Streams 'msmarco-docs.tsv.gz' into inserter '.zip' files.
///
/// Writes one '.zip' per [DOCS_PER_ZIP] documents into [MSMARCO_IMPORT_DIR]
/// and a url → docid map into [MSMARCO_DOCIDS_FILE].
/// PriEco indexes urls, the map translates results back to docids the qrels know.
/// Fields MS MARCO doesn't have get neutral values, so only text relevance is measured.
/// A url seen before is skipped, the inserter would drop it anyway.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// Number of converted documents.
///
/// # Errors
///
/// * Corpus is missing or not valid gzip.
/// * Import dir or a '.zip' can't be written.
///
/// # Panics
///
/// Only if system runs out of memory.
fn convert() -> Result<usize, Box<dyn Error + Send + Sync>> {
    if Path::new(MSMARCO_IMPORT_DIR).exists() {
        remove_dir_all(MSMARCO_IMPORT_DIR)?;
    }
    create_dir_all(MSMARCO_IMPORT_DIR)?;

    // Open corpus
    let corpus = File::open(Path::new(MSMARCO_DIR).join(CORPUS_FILE))?;
    let reader = BufReader::with_capacity(1 << 20, GzDecoder::new(corpus));
    let mut docids = BufWriter::new(File::create(MSMARCO_DOCIDS_FILE)?);

    // No embeddings yet, the inserter skips zero vectors
    let vector = vec!["0"; VECTOR_DIM].join(" ");

    let mut seen_urls: HashSet<u64> = HashSet::with_capacity(3_300_000);
    let mut zip: Option<ZipWriter<File>> = None;
    let mut converted: usize = 0;
    let mut duplicates: usize = 0;

    for line in reader.lines() {
        let line = line?;

        let parts: Vec<&str> = line.splitn(4, '\t').collect();
        if parts.len() != 4 {
            continue;
        }
        let docid = parts[0];
        let url = parts[1];
        let title = parts[2].replace("<-->", " ");
        let body = parts[3].replace("<-->", " ");

        if !seen_urls.insert(url_to_id(url)) {
            duplicates += 1;
            continue;
        }

        if converted.is_multiple_of(DOCS_PER_ZIP) {
            if let Some(full_zip) = zip.take() {
                full_zip.finish()?;
            }

            let zip_path = format!(
                "{}/msmarco_{:04}.zip",
                MSMARCO_IMPORT_DIR,
                converted / DOCS_PER_ZIP
            );
            let mut new_zip = ZipWriter::new(File::create(zip_path)?);
            new_zip.start_file(
                "docs.txt",
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .large_file(true),
            )?;
            zip = Some(new_zip);
        }

        let description: String = body.chars().take(DESCRIPTION_CHARS).collect();
        let has_500_words = body.split_whitespace().count() >= 500;
        let record = format!(
            "{}<-->{}<-->{}<-->{}<--><--><--><-->true<--><-->en<--><-->[0, 0, 0, 0]<-->0<-->0<-->5<-->false<-->{}<-->{}\n",
            url, title, description, body, has_500_words, vector
        );

        // Write
        if let Some(current_zip) = zip.as_mut() {
            current_zip.write_all(record.as_bytes())?;
        }
        writeln!(docids, "{}\t{}", url, docid)?;

        converted += 1;
        if converted.is_multiple_of(100_000) {
            println!("{}: Converted {}", icons::BENCHMARK, converted);
        }
    }

    if let Some(last_zip) = zip.take() {
        last_zip.finish()?;
    }
    docids.flush()?;

    if duplicates > 0 {
        println!(
            "{}{}: Skipped {} duplicate urls{}",
            icons::BENCHMARK,
            colors::YELLOW,
            duplicates,
            colors::RESET,
        );
    }

    Ok(converted)
}
