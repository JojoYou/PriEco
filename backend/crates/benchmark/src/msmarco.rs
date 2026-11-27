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
//!     * [**Embed**:][embed_batch] Document vectors, same model and recipe.
//!     * [**Merge**:][merge_index] Tantivy segments and IVF buckets, ignores 'dont_merge.txt'.
//! 2. [**Queries**:][load_queries] Judged queries of each [QUERY_SETS] entry.
//! 3. [**Docids**:][load_docids] Result url → MS MARCO docid.
//! 4. [**Run file**:][write_run] Ranked docids in TREC format for scoring.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-22
//! * Last Modified: 2026-11-27
//!
//! ## Planned Improvements
//!
//! - [ ] nDCG@10, MRR@10, Recall
//! - [ ] MS MARCO Passage

/*
  Import system libraries
*/
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{File, create_dir_all, read_dir, read_to_string, remove_dir_all, write},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

/*
  Import external libraries
*/
use flate2::read::GzDecoder;
use ndarray::{Array, CowArray, IxDyn, s};
use ort::{Session, Value, tensor::OrtOwnedTensor};
use parking_lot::Mutex;
use tantivy::merge_policy::NoMergePolicy;
use tokenizers::{Encoding, Tokenizer, TruncationParams};
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
    EmbeddingService, PriecoIndex, TANTIVY_HEAP_SIZE, TANTIVY_WRITER_THREADS, VECTOR_DIM, colors,
    file_exists, globals::icons, url_to_id,
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

/// Documents embedded in one model run.
const EMBED_BATCH: usize = 128;
/// Characters of 'title body' that are embedded.
const EMBED_CHARS: usize = 500;
/// Tokens the embedded text is truncated to.
const EMBED_TOKENS: usize = 64;

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
    (
        "msmarco_dev",
        "msmarco-docdev-queries.tsv.gz",
        "msmarco-docdev-qrels.tsv.gz",
    ),
];

/// Downloads MS MARCO and opens its index, building it on the first run.
///
/// Indexing is heavy disk work, so it runs on a blocking thread.
/// Later runs find everything cached and only open the index.
///
/// # Arguments
///
/// * `service` - Embedder of the running PriEco, used for document vectors.
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
pub async fn prepare(service: &EmbeddingService) -> Option<Arc<PriecoIndex>> {
    // Download dataset
    // May be already cached
    if !download::run(MSMARCO_DIR, FILES).await {
        return None;
    }

    // Open index
    let service = service.clone();
    let opened = spawn_blocking(move || -> Result<PriecoIndex, String> {
        let index = PriecoIndex::open(MSMARCO_INDEX_DIR);
        if !file_exists(MSMARCO_INDEXED_FILE) {
            build_index(&index, &service).map_err(|e| e.to_string())?;
        }
        merge_index(&index).map_err(|e| e.to_string())?;
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
    let qrels_path = Path::new(MSMARCO_DIR).join(qrels_file);
    let qrels = if qrels_file.ends_with(".gz") {
        let mut text = String::new();
        GzDecoder::new(File::open(&qrels_path)?).read_to_string(&mut text)?;
        text
    } else {
        read_to_string(&qrels_path)?
    };

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
/// * `service` - Embedder for document vectors.
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
fn build_index(
    index: &PriecoIndex,
    service: &EmbeddingService,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = Instant::now();

    // Convert corpus to inserter format
    let converted = convert(service)?;

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

/// Merges the benchmark index, even when the official index skips merging.
///
/// The inserter only merges when 'dont_merge.txt' is missing, that switch is for the official index.
/// IVF search reads only merged 'bucket_*.bin.zst', so 'staging_*.bin' left unmerged are never searched.
/// Runs on every [prepare], does nothing when there is one segment and no staging file.
///
/// # Arguments
///
/// * `index` - Opened benchmark index.
///
/// # Returns
///
/// `Ok(())` when Tantivy has one segment and every staging file is merged.
///
/// # Errors
///
/// * Vector dir can't be read.
/// * Tantivy writer can't be created or segments can't be read.
/// * A bucket merge failed.
///
/// # Panics
///
/// If the Tantivy commit fails.
fn merge_index(index: &PriecoIndex) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = Instant::now();

    // Tantivy
    if index.tantivy.searchable_segment_ids()?.len() > 1 {
        let writer = index
            .tantivy
            .writer_with_num_threads(TANTIVY_WRITER_THREADS, TANTIVY_HEAP_SIZE)?;
        writer.set_merge_policy(Box::new(NoMergePolicy));

        let writer = Mutex::new(writer);
        db_insert::merge_tantivy(index, &writer);
        writer.into_inner().wait_merging_threads()?;
    }

    // Vector buckets
    let mut staging: Vec<usize> = Vec::new();
    for entry in read_dir(&index.vector_path)? {
        let name = entry?.file_name();
        if let Some(bucket_id) = name
            .to_string_lossy()
            .strip_prefix("staging_")
            .and_then(|rest| rest.strip_suffix(".bin"))
            .and_then(|id| id.parse::<usize>().ok())
        {
            staging.push(bucket_id);
        }
    }

    for (done, bucket_id) in staging.iter().enumerate() {
        db_insert::merge_bucket(*bucket_id, &index.vector_path)?;
        if (done + 1).is_multiple_of(100) || done + 1 == staging.len() {
            println!(
                "{}: Merged {}/{} buckets",
                icons::BENCHMARK,
                done + 1,
                staging.len()
            );
        }
    }

    println!(
        "{}{}: MS MARCO index merged in {:.3}s!{}",
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
/// Documents are buffered and embedded per [EMBED_BATCH], see [write_batch].
///
/// # Arguments
///
/// * `service` - Embedder for document vectors.
///
/// # Returns
///
/// Number of converted documents.
///
/// # Errors
///
/// * Corpus is missing or not valid gzip.
/// * Import dir or a '.zip' can't be written.
/// * Tokenizer truncation can't be set or embedding failed.
///
/// # Panics
///
/// Only if system runs out of memory.
fn convert(service: &EmbeddingService) -> Result<usize, Box<dyn Error + Send + Sync>> {
    if Path::new(MSMARCO_IMPORT_DIR).exists() {
        remove_dir_all(MSMARCO_IMPORT_DIR)?;
    }
    create_dir_all(MSMARCO_IMPORT_DIR)?;

    // Open corpus
    let corpus = File::open(Path::new(MSMARCO_DIR).join(CORPUS_FILE))?;
    let reader = BufReader::with_capacity(1 << 20, GzDecoder::new(corpus));
    let mut docids = BufWriter::new(File::create(MSMARCO_DOCIDS_FILE)?);

    let mut tokenizer: Tokenizer = service.tokenizer.blocking_lock().clone();
    tokenizer.with_truncation(Some(TruncationParams {
        max_length: EMBED_TOKENS,
        ..Default::default()
    }))?;

    let mut batch: Vec<(String, String, String, String)> = Vec::with_capacity(EMBED_BATCH);
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

        batch.push((url.to_string(), docid.to_string(), title, body));
        if batch.len() == EMBED_BATCH {
            write_batch(
                service,
                &tokenizer,
                &mut batch,
                &mut zip,
                &mut docids,
                &mut converted,
            )?;
        }
    }

    if !batch.is_empty() {
        write_batch(
            service,
            &tokenizer,
            &mut batch,
            &mut zip,
            &mut docids,
            &mut converted,
        )?;
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

/// Embeds buffered documents and writes them as inserter records.
///
/// A new '.zip' is started every [DOCS_PER_ZIP] documents.
/// Embedded text is 'title body', cut to [EMBED_CHARS] characters.
/// The model is locked only for one batch, so user queries can still be embedded.
///
/// # Arguments
///
/// * `service` - Embedder, only its model is used.
/// * `tokenizer` - Tokenizer truncated to [EMBED_TOKENS].
/// * `batch` - (url, docid, title, body) per document, empty afterwards.
/// * `zip` - Current '.zip', replaced when full.
/// * `docids` - Writer of the url → docid map.
/// * `converted` - Documents written so far, increased per document.
///
/// # Returns
///
/// `Ok(())` when every document of the batch is written.
///
/// # Errors
///
/// * Embedding failed.
/// * A '.zip' or the docid map can't be written.
fn write_batch(
    service: &EmbeddingService,
    tokenizer: &Tokenizer,
    batch: &mut Vec<(String, String, String, String)>,
    zip: &mut Option<ZipWriter<File>>,
    docids: &mut BufWriter<File>,
    converted: &mut usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let texts: Vec<String> = batch
        .iter()
        .map(|(_, _, title, body)| {
            format!("{} {}", title, body)
                .chars()
                .take(EMBED_CHARS)
                .collect()
        })
        .collect();

    let vectors = {
        let model = service.model.blocking_lock();
        embed_batch(tokenizer, &model, texts)?
    };

    for ((url, docid, title, body), vector) in batch.drain(..).zip(vectors) {
        if converted.is_multiple_of(DOCS_PER_ZIP) {
            if let Some(full_zip) = zip.take() {
                full_zip.finish()?;
            }

            let zip_path = format!(
                "{}/msmarco_{:04}.zip",
                MSMARCO_IMPORT_DIR,
                *converted / DOCS_PER_ZIP
            );
            let mut new_zip = ZipWriter::new(File::create(zip_path)?);
            new_zip.start_file(
                "docs.txt",
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .large_file(true),
            )?;
            *zip = Some(new_zip);
        }

        let description: String = body.chars().take(DESCRIPTION_CHARS).collect();
        let has_500_words = body.split_whitespace().count() >= 500;
        let record = format!(
            "{}<-->{}<-->{}<-->{}<--><--><--><-->false<--><-->en<--><-->[0, 0, 0, 0]<-->0<-->0<-->5<-->false<-->{}<-->{}\n",
            url, title, description, body, has_500_words, vector
        );

        // Write
        if let Some(current_zip) = zip.as_mut() {
            current_zip.write_all(record.as_bytes())?;
        }
        writeln!(docids, "{}\t{}", url, docid)?;

        *converted += 1;
        if converted.is_multiple_of(100_000) {
            println!("{}: Converted {}", icons::BENCHMARK, converted);
        }
    }

    Ok(())
}

/// Embeds texts in one model run.
///
/// Vector is the mean of unpadded token vectors, L2 normalized for cosine search.
///
/// # Arguments
///
/// * `tokenizer` - Tokenizer of [prieco_core::EMBED_MODEL], with padding and truncation.
/// * `model` - Loaded [prieco_core::EMBED_MODEL].
/// * `texts` - Texts to embed, not empty.
///
/// # Returns
///
/// One vector of [VECTOR_DIM] values per text, as space separated string.
///
/// # Errors
///
/// If tokenization, tensor creation or the model fails.
fn embed_batch(
    tokenizer: &Tokenizer,
    model: &Session,
    texts: Vec<String>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let encodings = tokenizer.encode_batch(texts, true)?;
    let (batch, seq) = (encodings.len(), encodings[0].len());
    let flat = |field: fn(&Encoding) -> &[u32]| -> Vec<i64> {
        encodings
            .iter()
            .flat_map(|encoding| field(encoding).iter().map(|&v| v as i64))
            .collect()
    };

    let mask = flat(Encoding::get_attention_mask);
    let ids = CowArray::from(Array::from_shape_vec(
        IxDyn(&[batch, seq]),
        flat(Encoding::get_ids),
    )?);
    let mask_array = CowArray::from(Array::from_shape_vec(IxDyn(&[batch, seq]), mask.clone())?);
    let type_ids = CowArray::from(Array::from_shape_vec(
        IxDyn(&[batch, seq]),
        flat(Encoding::get_type_ids),
    )?);

    let outputs = model.run(vec![
        Value::from_array(model.allocator(), &ids)?,
        Value::from_array(model.allocator(), &mask_array)?,
        Value::from_array(model.allocator(), &type_ids)?,
    ])?;
    let tensor: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;
    let view = tensor.view();

    let mut vectors: Vec<String> = Vec::with_capacity(batch);
    for i in 0..batch {
        // Mean pooling
        let mut sum = vec![0f32; VECTOR_DIM];
        let mut count = 0f32;
        for j in 0..seq {
            if mask[i * seq + j] == 1 {
                for (s, &t) in sum.iter_mut().zip(view.slice(s![i, j, ..]).iter()) {
                    *s += t;
                }
                count += 1.0;
            }
        }
        let mean: Vec<f32> = sum.iter().map(|v| v / count.max(1.0)).collect();

        // L2 normalize
        let norm = mean.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
        vectors.push(
            mean.iter()
                .map(|v| (v / norm).to_string())
                .collect::<Vec<String>>()
                .join(" "),
        );
    }

    Ok(vectors)
}
