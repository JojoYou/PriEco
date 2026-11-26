//! # Globals
//!
//! Objects shared by the whole project. Most are [Lazy], created on first use.
//!
//! ## Parts
//!
//! * **Types:** [SearchResult], [ImgResult], [WebDocument], [PriEcoConfig].
//! * **Config:** [PRIECO_CONFIG], loaded or asked for by [set_up].
//! * **HTTP:** [CLIENT], [PROXY_CLIENT], [PING_CLIENT].
//! * **Storage:** [PRIECO_META] meta storage, [PRIECO_BLOBS] blob storage.
//! * **Index:** [PRIECO_INDEX] = Tantivy ([TANTIVY_INDEX]) + meta + IVF vectors ([VECTOR_CENTROPOIDS]).
//! * **Models:** [EmbeddingService] for query vectors, [RERANKER] (BGE).
//! * **Query understanding:** [get_store] synonyms, [MATCHERS] intent, [QU_PIPELINE] entities,
//!   [SPELL_CHECKER], [BANGS].
//! * **Ranking:** [PAGERANK].
//! * **Analytics:** [ANALYTICS], anonymous daily counters, see [AnalyticsDb].
//! * **Terminal:** [colors], [icons].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2025-09-20
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] Spell checker in more languages than English
//! - [ ] New PageRank

/*
  Import system libraries
*/
#[cfg(feature = "cuda")]
use std::io::{Cursor, Read};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    error::Error,
    fs::{File, create_dir_all, write},
    hash::{Hash, Hasher},
    ops::Range,
    path::Path,
    str::from_utf8,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration as stdDuration,
};

/*
  Import external libraries
*/
use ahash::{AHashMap, AHashSet};
use aho_corasick::{AhoCorasick, MatchKind};
use charabia::{Tokenizer as CHARABIA_TOKENIZER, TokenizerBuilder};
#[cfg(feature = "cuda")]
use cudarc::{
    cublas::{CudaBlas, Gemm, GemmConfig, StridedBatchedConfig, sys::cublasOperation_t},
    driver::{CudaContext, CudaSlice, CudaStream, DevicePtr, LaunchConfig, PushKernelArg},
    nvrtc::compile_ptx,
};
use dashmap::DashSet;
use fjall::{
    CompressionType, Database as FJALL_DATABASE, Keyspace, KeyspaceCreateOptions,
    KvSeparationOptions, config::CompressionPolicy,
};
use fst::Map as FstMap;
use memmap2::{Mmap, MmapOptions};
use ndarray::{Array, Array2, CowArray, IxDyn};
use once_cell::sync::Lazy;
use ort::{
    Environment, ExecutionProvider, GraphOptimizationLevel, Session, SessionBuilder, Value,
    tensor::OrtOwnedTensor,
};
use parking_lot::{Condvar, Mutex, RwLock};
use primp::{Client as PIMP_CLIENT, Impersonate, ImpersonateOS, redirect::Policy};
use rand::TryRng;
#[cfg(feature = "cuda")]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rkyv::{Archive, Archived, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize, access};
use rocket::{
    Request,
    request::{FromRequest, Outcome},
};
use serde::{Deserialize, Serialize};
use symspell::{SymSpell, UnicodeStringStrategy};
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{
        FAST, INDEXED, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
    },
    tokenizer::{TextAnalyzer, Token, TokenStream, Tokenizer as TANTIVY_TOKENIZER},
};
use time::{Date, Duration, OffsetDateTime, macros::format_description};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};
use tokio::task;
use twox_hash::XxHash3_64;
#[cfg(feature = "cuda")]
use zstd::decode_all;
use zstd::dict::DecoderDictionary;

#[cfg(feature = "cuda")]
use crate::{ID_SIZE, RECORD_SIZE};

/*
  Import own libraries
*/
use crate::{
    BGE_MODEL, DAILY_SALT_FILE, STATIC_DIR,
    helpers::{normalize_url, url_to_id},
};
use crate::{ZSTD_DICT, set_up};
use crate::{
    constants::{FINAL_SCORES, ID_MAP_FILE},
    read_file,
};

/*
  Constants
*/
/// Added to CSS URLs, change it to refresh browser caches.
pub const CSS_VERSION: &str = "0.2.0";
/// Added to JS URLs, change it to refresh browser caches.
pub const JS_VERSION: &str = "0.2.0";

// Terminal print colors
/// ANSI colors for terminal prints.
pub mod colors {
    /// Must be used after every colored print.
    pub const RESET: &str = "\x1b[0m";
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
}

/// Prefixes of terminal prints, one per service.
pub mod icons {
    /// Blob storage.
    pub const BLOB: &str = "🪼";
    /// Inserter.
    pub const DB_INSERT: &str = "💾";
    /// PageRank.
    pub const PAGERANK_ICON: &str = "📋";
    /// Mini crawler.
    pub const MINI_CRAWLER_ICON: &str = "👾";
    /// Index updater.
    pub const INDEX_UPDATER: &str = "📚";
    /// Benchmark.
    pub const BENCHMARK: &str = "📏";
}

/*
  PriEco structures
*/
/// Sub page shown under a result.
#[derive(Serialize)]
pub struct Sitelink {
    pub title: String,
    pub url: String,
}

/// One web result, as the search template gets it.
#[derive(Serialize)]
pub struct SearchResult {
    pub url: String,
    /// URL shortened for showing.
    pub display_url: String,
    pub domain: String,

    pub title: String,
    pub description: String,

    pub image: String,
    pub favicon: String,

    /// Blob id, when the page has cached html.
    pub html_id: Option<String>,

    /// Page text start.
    pub content: String,
    /// Label from Flesch Reading Ease.
    pub reading_level: String,
    /// Loading time for showing.
    pub formatted_load: String,
    /// Page intent label.
    pub intent: String,
    /// Where the result came from, e.g. 'PriEco Index'.
    pub source_engine: String,

    pub sitelinks: Vec<Sitelink>,
}

/// One image result.
#[derive(Serialize, Clone)]
pub struct ImgResult {
    /// Small image.
    pub thumbnail: String,
    pub image: String,

    pub title: String,

    /// Page with the image.
    pub site_url: String,
    pub site_domain: String,
    pub favicon: String,
}
/// Product card result.
#[derive(Serialize)]
pub struct WebScrollResult {
    pub url: String,
    pub domain: String,
    pub image: String,
    pub favicon: String,
    pub title: String,
    pub price: String,
}
// Index documents
/// [QueryIntent::Unknown] as u8, for documents saved before intent existed.
fn default_unknown_intent() -> u8 {
    5
}
/// One indexed page, stored as zstd compressed JSON in meta storage.
///
/// Key is [url_to_id] of `url`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebDocument {
    pub url: String,

    /// Max 60 characters.
    pub title: String,
    /// Max 160 characters.
    pub description: String,
    /// First 500 characters of page text.
    pub content: String,
    pub favicon: String,
    pub image: String,
    /// Meta keywords.
    pub keywords: String,

    /// True if page is NSFW.
    pub safe_s: bool,
    /// ISO 639-1 language code.
    pub lang: String,
    /// Country code of server.
    pub loc: String,

    /// Html blob id.
    pub html: String,

    /// Not counted yet.
    pub impressions: u32,
    /// Not counted yet.
    pub clicks: u32,
    /// Flesch Reading Ease and text to image ratio.
    pub confidence: f32,
    /// Text effort.
    pub effort: f32,
    /// Quality score.
    pub qna: f32,
    /// Fourth crawler page score.
    pub sts: f32,

    /// Loading time in seconds.
    pub load: f64,
    /// Date of crawling.
    pub date: i64,

    /// Page intent, [QueryIntent] as u8.
    #[serde(default = "default_unknown_intent")]
    pub intent: u8,

    /// Page has mobile viewport.
    #[serde(default)]
    pub is_mobile: bool,
    /// Page has at least 500 words.
    #[serde(default)]
    pub has_500_words: bool,

    // Dynamic data
    /// Score during search, not stored.
    #[serde(default)]
    pub search_score: f32,
    /// Search engines that found it during search, not stored.
    #[serde(default)]
    pub source: String,
}

/// Paths being written, see [crate::acquire_file_lock].
pub struct FileLocks {
    /// Locked paths.
    pub set: Mutex<DashSet<String>>,
    /// Wakes threads waiting for a path.
    pub condvar: Condvar,
}
/// Process wide [FileLocks].
pub static FILE_LOCKS: Lazy<Arc<FileLocks>> = Lazy::new(|| {
    Arc::new(FileLocks {
        set: Mutex::new(DashSet::new()),
        condvar: Condvar::new(),
    })
});

/// Request guard with the User-Agent header, 'unknown' when missing.
pub struct UserAgent<'r>(pub &'r str);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAgent<'r> {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        Outcome::Success(UserAgent(
            req.headers().get_one("User-Agent").unwrap_or("unknown"),
        ))
    }
}

/*
PriEco config
*/
/// Node config, see [set_up::set_up_wizard].
pub static PRIECO_CONFIG: Lazy<PriEcoConfig> = Lazy::new(set_up::set_up_wizard);
/// Default of `enable_*` fields missing in old configs.
fn default_true() -> bool {
    true
}
/// Node config saved in [crate::CONFIG_FILE].
#[derive(Serialize, Deserialize)]
pub struct PriEcoConfig {
    /// Address the web server binds to.
    pub ip: String,
    /// Port of the web server.
    pub port: i32,

    /// Tantivy index directory.
    pub tantivy_path: String,
    /// Meta storage directory.
    pub meta_path: String,
    /// IVF buckets directory.
    pub vector_path: String,
    /// Blob storage directory.
    pub blob_path: String,

    /// Parallel downloads of the mini crawler.
    pub worker_concurrent: u32,

    /// Run blob importer.
    #[serde(default = "default_true")]
    pub enable_blob_storage: bool,
    /// Run inserter.
    #[serde(default = "default_true")]
    pub enable_db_inserter: bool,
    /// Run PageRank.
    #[serde(default = "default_true")]
    pub enable_pagerank: bool,
    /// Run mini crawler.
    #[serde(default = "default_true")]
    pub enable_mini_crawler: bool,
    /// Run index updater.
    #[serde(default = "default_true")]
    pub enable_index_updater: bool,
    /// Run website and search.
    #[serde(default = "default_true")]
    pub enable_web_server: bool,
}

/*
 Request Client
*/
/// HTTP client that looks like Chrome on Windows. 15s timeout.
pub static CLIENT: Lazy<PIMP_CLIENT> = Lazy::new(|| {
    println!("Created primp client!");

    PIMP_CLIENT::builder()
        .impersonate(Impersonate::ChromeV148)
        .impersonate_os(ImpersonateOS::Windows)
        .timeout(stdDuration::from_secs(15))
        .connect_timeout(stdDuration::from_secs(5))
        .build()
        .expect("Failed to create primp client")
});

/// Same as [CLIENT] but doesn't follow redirects, for the image proxy.
pub static PROXY_CLIENT: Lazy<PIMP_CLIENT> = Lazy::new(|| {
    println!("Created primp proxy client!");

    PIMP_CLIENT::builder()
        .impersonate(Impersonate::ChromeV148)
        .impersonate_os(ImpersonateOS::Windows)
        .redirect(Policy::none())
        .timeout(stdDuration::from_secs(15))
        .connect_timeout(stdDuration::from_secs(5))
        .build()
        .expect("Failed to create primp proxy client")
});

/// Plain HTTP client with 5s timeout, for checking if pages are alive.
pub static PING_CLIENT: Lazy<PIMP_CLIENT> = Lazy::new(|| {
    println!("Created simple primp ping client!");

    PIMP_CLIENT::builder()
        .timeout(stdDuration::from_secs(5))
        .build()
        .expect("Failed to build ping client")
});

/*
 Vector embeder
*/
/// Tokenizer of [crate::EMBED_MODEL].
pub static VECTOR_EMBEDDING_TOKENIZER: &[u8] = include_bytes!("../../../data/tokenizer.json");
/// Turns text into a vector with [crate::EMBED_MODEL].
#[derive(Clone)]
pub struct EmbeddingService {
    /// Text tokenizer.
    pub tokenizer: Arc<tokio::sync::Mutex<Tokenizer>>,
    /// ONNX session.
    pub model: Arc<tokio::sync::Mutex<Session>>,
}
impl EmbeddingService {
    /// Embeds query.
    ///
    /// Runs on a blocking thread. Vector is mean of token vectors, not normalized.
    ///
    /// # Arguments
    ///
    /// * `query` - Text.
    ///
    /// # Returns
    ///
    /// Vector of [crate::VECTOR_DIM] values.
    ///
    /// # Errors
    ///
    /// If tokenization or model fails.
    pub async fn embed_query(
        &self,
        query: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let query = query.to_string();
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();

        task::spawn_blocking(
            move || -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
                let tokenizer_guard = tokio::runtime::Handle::current().block_on(tokenizer.lock());

                let encoding = tokenizer_guard
                    .encode(query, true)
                    .map_err(|e| format!("Tokenization failed: {}", e))?;

                let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
                let attention_mask: Vec<i64> = encoding
                    .get_attention_mask()
                    .iter()
                    .map(|&v| v as i64)
                    .collect();

                let type_ids: Vec<i64> =
                    encoding.get_type_ids().iter().map(|&v| v as i64).collect();

                let shape = [1, token_ids.len()];
                let cow_input_ids = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), token_ids)
                        .map_err(|e| format!("Failed to create input_ids array: {}", e))?,
                );
                let cow_attention_masks = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), attention_mask.clone())
                        .map_err(|e| format!("Failed to create attention_mask array: {}", e))?,
                );

                let cow_type_ids = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), type_ids)
                        .map_err(|e| format!("Failed to create type_ids array: {}", e))?,
                );

                let embedder = tokio::runtime::Handle::current().block_on(model.lock());

                let input_tensor_ids = ort::Value::from_array(embedder.allocator(), &cow_input_ids)
                    .map_err(|e| format!("Failed to create input tensor: {}", e))?;
                let input_tensor_attention_mask =
                    ort::Value::from_array(embedder.allocator(), &cow_attention_masks)
                        .map_err(|e| format!("Failed to create attention mask tensor: {}", e))?;

                let input_tensor_type_ids =
                    ort::Value::from_array(embedder.allocator(), &cow_type_ids)
                        .map_err(|e| format!("Failed to create type_ids tensor: {}", e))?;

                let outputs: Vec<ort::Value<'static>> = embedder
                    .run(vec![
                        input_tensor_ids,
                        input_tensor_attention_mask,
                        input_tensor_type_ids,
                    ])
                    .map_err(|e| format!("Model inference failed: {}", e))?;

                let tensor: OrtOwnedTensor<f32, _> = outputs[0]
                    .try_extract()
                    .map_err(|e| format!("Failed to extract output tensor: {}", e))?;

                let view = tensor.view();
                let shape = view.shape();
                let hidden_size = shape[2];

                let batch_row = view.outer_iter().next().ok_or("No embedding row found")?;

                let mut sum_vec = vec![0.0f32; hidden_size];
                let mut token_count = 0.0f32;

                for (j, token_vec) in batch_row.outer_iter().enumerate() {
                    if attention_mask[j] == 1 {
                        for k in 0..hidden_size {
                            sum_vec[k] += token_vec[k];
                        }
                        token_count += 1.0;
                    }
                }

                // Average
                if token_count > 0.0 {
                    for val in &mut sum_vec[..hidden_size] {
                        *val /= token_count;
                    }
                }

                Ok(sum_vec)
            },
        )
        .await?
    }
}

/// Country code → main language code. Default language for a location.
pub static COUNTRY_TO_LANG: Lazy<Arc<AHashMap<&'static str, &'static str>>> = Lazy::new(|| {
    let mut map = AHashMap::with_capacity(200);

    // Direct mappings
    map.insert("de", "de");
    map.insert("at", "de");
    map.insert("ch", "de");
    map.insert("fr", "fr");
    map.insert("es", "es");
    map.insert("mx", "es");
    map.insert("ar", "es");
    map.insert("cl", "es");
    map.insert("co", "es");
    map.insert("pe", "es");
    map.insert("pt", "pt");
    map.insert("br", "pt");
    map.insert("us", "en");
    map.insert("gb", "en");
    map.insert("au", "en");
    map.insert("nz", "en");
    map.insert("ie", "en");
    map.insert("ca", "all"); // fallback to All
    map.insert("it", "it");
    map.insert("is", "is");
    map.insert("id", "id");
    map.insert("lv", "lv");
    map.insert("lt", "lt");
    map.insert("hu", "hu");
    map.insert("nl", "nl");
    map.insert("no", "no");
    map.insert("pl", "pl");
    map.insert("ro", "ro");
    map.insert("sk", "sk");
    map.insert("fi", "fi");
    map.insert("tr", "tr");
    map.insert("bg", "bg");
    map.insert("ru", "ru");
    map.insert("rs", "sr");
    map.insert("si", "sl");
    map.insert("se", "sv");
    map.insert("gr", "el");
    map.insert("cz", "cs");
    map.insert("dk", "da");
    map.insert("jp", "ja");
    map.insert("kr", "ko");

    // === Closest language mappings ===
    map.insert("cy", "el"); // Cyprus → Greek
    map.insert("il", "he"); // Israel → Hebrew

    // Arabic world
    for c in [
        "sa", "ae", "eg", "dz", "ma", "tn", "ye", "om", "iq", "jo", "ps", "kw", "qa", "bh", "ly",
        "sd", "sy",
    ] {
        map.insert(c, "ar");
    }

    // Former USSR → Russian
    for c in ["uz", "kz", "kg", "tj", "by", "ua", "am", "az", "ge"] {
        map.insert(c, "ru");
    }

    // Chinese-speaking (no zh support) → All fallback
    for c in ["sg", "hk", "mo", "tw", "cn"] {
        map.insert(c, "all");
    }

    // South Asia → English
    for c in ["in", "pk", "bd", "np", "lk"] {
        map.insert(c, "en");
    }

    // Africa & rest → English
    for c in ["ng", "za", "ke", "gh", "tz", "ug", "zm", "zw", "mw"] {
        map.insert(c, "en");
    }

    Arc::new(map)
});

/// zstd dictionary for [WebDocument] compression.
///
/// Path is `PRIECO_ZSTD_DICT` or [crate::ZSTD_DICT]. `None` uses plain zstd.
pub static META_DICTIONARY: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    let path = std::env::var("PRIECO_ZSTD_DICT").unwrap_or_else(|_| ZSTD_DICT.to_string());

    match std::fs::read(&path) {
        Ok(bytes) => {
            println!("Loaded ZSTD dictionary from {}", path);
            Some(bytes)
        }
        Err(_) => {
            eprintln!("Warning: ZSTD dictionary not found. Proceeding with standard compression.");
            None
        }
    }
});

/// Prepared decoder of [META_DICTIONARY].
pub static META_DECODER: Lazy<Option<DecoderDictionary<'static>>> = Lazy::new(|| {
    META_DICTIONARY
        .as_ref()
        .map(|dict_bytes| DecoderDictionary::copy(dict_bytes))
});

/// Directory with '.tar.gz' files for [blob storage][PRIECO_BLOBS].
pub const BLOB_IMPORT_DIR: &str = "/mnt/hdd/imp_blob/";

/// Fjall database with result meta data and small site data.
pub struct PriecoMetaStorage {
    // Meta data: Titles, Descriptions, URLs...
    /// Database.
    pub meta_db: FJALL_DATABASE,
    /// Page id → zstd [WebDocument].
    pub meta_ks: Keyspace,

    // Goggles
    /// Goggles.
    pub goggles_ks: Keyspace,

    // Widgets
    /// Widget caches.
    pub widgets_ks: Keyspace,

    // Analytics
    /// Analytics counters, see [AnalyticsDb].
    pub analytics_ks: Keyspace,

    // API
    /// API keys and usage.
    pub api_ks: Keyspace,
}

/// Fjall database with compressed page text and links.
pub struct PriecoBlobStorage {
    // Blob storage: Compressed web page data
    /// Database.
    pub blob_db: FJALL_DATABASE,
    /// Blob id → blob, see [prieco_blob](../prieco_blob/index.html).
    pub blobs_ks: Keyspace,
}

/// Meta storage of this node.
pub static PRIECO_META: Lazy<Arc<PriecoMetaStorage>> = Lazy::new(|| {
    let meta_db = load_meta_db(&PRIECO_CONFIG.meta_path);
    let meta_ks = load_meta_ks(&meta_db);

    let goggles_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));
    let goggles_ks = meta_db.keyspace("goggles", || goggles_opts).unwrap();

    let widgets_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));
    let widgets_ks = meta_db.keyspace("widgets", || widgets_opts).unwrap();

    let analytics_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));
    let analytics_ks = meta_db.keyspace("analytics", || analytics_opts).unwrap();

    let api_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));
    let api_ks = meta_db.keyspace("api", || api_opts).unwrap();

    Arc::new(PriecoMetaStorage {
        meta_db,
        meta_ks,

        goggles_ks,

        widgets_ks,

        analytics_ks,

        api_ks,
    })
});

/// Opens or creates the Fjall database that holds result meta data.
///
/// Shared by [PRIECO_META] and [PriecoIndex::open].
///
/// # Arguments
///
/// * `path` - Directory of the database.
///
/// # Returns
///
/// Opened database.
///
/// # Panics
///
/// If the database can't be opened. Start-up only.
fn load_meta_db(path: &str) -> FJALL_DATABASE {
    FJALL_DATABASE::builder(Path::new(path))
        .worker_threads(4)
        .cache_size(2 * 1024 * 1024 * 1024)
        .open()
        .expect("Failed to open Meta Fjall DB")
}

/// Opens or creates the `meta` keyspace with one [WebDocument] per id.
///
/// Data blocks aren't compressed, documents are already zstd compressed.
///
/// # Arguments
///
/// * `meta_db` - Database from [load_meta_db].
///
/// # Returns
///
/// The `meta` keyspace.
///
/// # Panics
///
/// If the keyspace can't be opened. Start-up only.
fn load_meta_ks(meta_db: &FJALL_DATABASE) -> Keyspace {
    let meta_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::disabled())
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));

    meta_db
        .keyspace("meta", || meta_opts)
        .expect("Failed to open Meta keyspace")
}

/// Blob storage of this node. Large values are stored apart from the tree.
pub static PRIECO_BLOBS: Lazy<Arc<PriecoBlobStorage>> = Lazy::new(|| {
    let blob_db = FJALL_DATABASE::builder(Path::new(&PRIECO_CONFIG.blob_path))
        .worker_threads(2)
        .cache_size(1024 * 1024 * 1024)
        .open()
        .expect("Failed to open Blob Fjall DB");

    let blob_opts = KeyspaceCreateOptions::default()
        .max_memtable_size(2048 * 1024 * 1024)
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .with_kv_separation(Some(KvSeparationOptions {
            compression: CompressionType::None,
            file_target_size: 2048 * 1024 * 1024,
            separation_threshold: 100,
            staleness_threshold: 0.9,
            age_cutoff: 0.0,
        }));

    let blobs_ks = blob_db.keyspace("blobs", || blob_opts).unwrap();

    Arc::new(PriecoBlobStorage { blob_db, blobs_ks })
});

/*
  Index
*/
/// Memory of Tantivy writer, in bytes.
pub const TANTIVY_HEAP_SIZE: usize = 512_000_000;
/// Threads of Tantivy writer.
pub const TANTIVY_WRITER_THREADS: usize = 2;
/// Tantivy index of this node.
pub static TANTIVY_INDEX: Lazy<Arc<Index>> =
    Lazy::new(|| Arc::new(load_tantivy(&PRIECO_CONFIG.tantivy_path)));

/// Opens or creates a Tantivy index with the PriEco schema.
///
/// Shared by [TANTIVY_INDEX] and [PriecoIndex::open], so every index has the same schema.
///
/// # Arguments
///
/// * `path` - Directory of the index. Must exist.
///
/// # Returns
///
/// Index with the multilingual tokenizer registered.
///
/// # Panics
///
/// If the directory can't be opened or the index is corrupted. Start-up only.
fn load_tantivy(path: &str) -> Index {
    // Build schema
    let mut builder = Schema::builder();

    builder.add_u64_field("doc_id", STORED | INDEXED);
    builder.add_u64_field("domain_id", INDEXED | FAST);

    let multilingual = TextFieldIndexing::default()
        .set_tokenizer("multilingual")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_opts = TextOptions::default().set_indexing_options(multilingual);

    builder.add_text_field("title", text_opts.clone());
    builder.add_text_field("description", text_opts.clone());
    builder.add_text_field("content", text_opts.clone());
    builder.add_text_field("keywords", text_opts);

    builder.add_text_field("lang", STRING | FAST);
    builder.add_text_field("loc", STRING | FAST);
    builder.add_i64_field("date", INDEXED | FAST);
    builder.add_bool_field("safe_s", INDEXED | FAST);

    builder.add_u64_field("intent", INDEXED | FAST);

    let schema = builder.build();

    // Open index
    let dir = MmapDirectory::open(Path::new(path)).expect("Failed to open Tantivy V2 directory");
    let mut index =
        Index::open_or_create(dir, schema.clone()).expect("Failed to open Tantivy index");
    index
        .set_multithread_executor(6)
        .expect("Failed to set Tantivy executor");

    index
        .tokenizers()
        .register("multilingual", TextAnalyzer::from(Multilingual::new()));

    index
}

/// Reader of [TANTIVY_INDEX].
pub static TANTIVY_READER: Lazy<Arc<IndexReader>> =
    Lazy::new(|| Arc::new(load_tantivy_reader(&TANTIVY_INDEX)));

/// Creates a reader that reloads shortly after every commit.
///
/// # Arguments
///
/// * `index` - Index to read.
///
/// # Returns
///
/// Reader of the index.
///
/// # Panics
///
/// If Tantivy can't create the reader. Start-up only.
fn load_tantivy_reader(index: &Index) -> IndexReader {
    index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .expect("Failed to create Tantivy reader")
}

/// The only writer of [TANTIVY_INDEX].
pub static TANTIVY_WRITER: Lazy<Arc<Mutex<IndexWriter>>> = Lazy::new(|| {
    Arc::new(Mutex::new(
        TANTIVY_INDEX
            .writer_with_num_threads(TANTIVY_WRITER_THREADS, TANTIVY_HEAP_SIZE)
            .expect("Failed to create Tantivy V2 writer"),
    ))
});

/// Query parser of [TANTIVY_INDEX].
pub static TANTIVY_QUERY_PARSER: Lazy<QueryParser> =
    Lazy::new(|| load_query_parser(&TANTIVY_INDEX));

/// Creates the query parser over title, description, content and keywords.
///
/// All terms must match by default.
///
/// # Arguments
///
/// * `index` - Index the parser is for.
///
/// # Returns
///
/// Query parser of the index.
///
/// # Panics
///
/// If the index has no PriEco schema. Start-up only.
fn load_query_parser(index: &Index) -> QueryParser {
    let schema = index.schema();
    let title_field = schema.get_field("title").unwrap();
    let description_field = schema.get_field("description").unwrap();
    let content_field = schema.get_field("content").unwrap();
    let keywords_field = schema.get_field("keywords").unwrap();

    let mut parser = QueryParser::for_index(
        index,
        vec![
            title_field,
            description_field,
            content_field,
            keywords_field,
        ],
    );

    parser.set_conjunction_by_default();

    parser
}

/// One searchable index: Tantivy, result meta data and vector buckets.
///
/// Search and the inserter take it as an argument, so they can run
/// on the official index ([PRIECO_INDEX]) or on a benchmark index.
pub struct PriecoIndex {
    /// Tantivy index.
    pub tantivy: Arc<Index>,
    /// Reader of `tantivy`.
    pub reader: Arc<IndexReader>,
    /// Query parser of `tantivy`.
    pub query_parser: QueryParser,

    /// Meta database.
    pub meta_db: FJALL_DATABASE,
    /// Page id → zstd [WebDocument].
    pub meta_ks: Keyspace,

    /// IVF buckets directory.
    pub vector_path: String,
}
impl PriecoIndex {
    /// Opens or creates an index in its own directory.
    ///
    /// Uses `dir/tantivy`, `dir/meta` and `dir/vectors`, with the same
    /// schema and settings as the official index.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory of the index, created when missing.
    ///
    /// # Returns
    ///
    /// Opened index.
    ///
    /// # Panics
    ///
    /// If a directory or database can't be opened. Start-up only.
    pub fn open(dir: &str) -> Self {
        let tantivy_path = format!("{}/tantivy", dir);
        let meta_path = format!("{}/meta", dir);
        let vector_path = format!("{}/vectors", dir);

        create_dir_all(&tantivy_path).expect("Failed to create Tantivy directory");
        create_dir_all(&vector_path).expect("Failed to create vector directory");

        // Tantivy
        let tantivy = Arc::new(load_tantivy(&tantivy_path));
        let reader = Arc::new(load_tantivy_reader(&tantivy));
        let query_parser = load_query_parser(&tantivy);

        // Meta
        let meta_db = load_meta_db(&meta_path);
        let meta_ks = load_meta_ks(&meta_db);

        PriecoIndex {
            tantivy,
            reader,
            query_parser,

            meta_db,
            meta_ks,

            vector_path,
        }
    }
}

/// Official index of this node, made of the globals above.
pub static PRIECO_INDEX: Lazy<Arc<PriecoIndex>> = Lazy::new(|| {
    Arc::new(PriecoIndex {
        tantivy: TANTIVY_INDEX.clone(),
        reader: TANTIVY_READER.clone(),
        query_parser: TANTIVY_QUERY_PARSER.clone(),

        meta_db: PRIECO_META.meta_db.clone(),
        meta_ks: PRIECO_META.meta_ks.clone(),

        vector_path: PRIECO_CONFIG.vector_path.clone(),
    })
});

// Multilang tokenization
/// Tantivy tokenizer for all languages, uses charabia. Tokens are lemmas of words.
#[derive(Clone)]
pub struct Multilingual(Arc<CHARABIA_TOKENIZER<'static>>);

impl Multilingual {
    /// Creates tokenizer. Leaks one builder, call it once per index.
    pub fn new() -> Self {
        let builder: &'static mut TokenizerBuilder<Vec<u8>> =
            Box::leak(Box::new(TokenizerBuilder::default()));
        Self(Arc::new(builder.build()))
    }
}

impl Default for Multilingual {
    fn default() -> Self {
        Self::new()
    }
}

/// Token stream of [Multilingual]: tokens and the current one.
pub struct MultiStream(std::vec::IntoIter<Token>, Option<Token>);

impl TokenStream for MultiStream {
    fn advance(&mut self) -> bool {
        self.1 = self.0.next();
        self.1.is_some()
    }
    fn token(&self) -> &Token {
        self.1.as_ref().unwrap()
    }
    fn token_mut(&mut self) -> &mut Token {
        self.1.as_mut().unwrap()
    }
}

impl TANTIVY_TOKENIZER for Multilingual {
    type TokenStream<'a> = MultiStream;

    fn token_stream(&mut self, text: &str) -> MultiStream {
        let tokens: Vec<Token> = self
            .0
            .tokenize(text)
            .filter(|t| t.is_word())
            .enumerate()
            .map(|(i, t)| Token {
                offset_from: t.byte_start,
                offset_to: t.byte_end,
                position: i,
                text: t.lemma().to_string(),
                position_length: 1,
            })
            .collect();
        MultiStream(tokens.into_iter(), None)
    }
}

/*
  Inserter
*/
/// Directory with '.zip' files for the inserter.
pub const INSERTER_IMPORT_DIR: &str = "/tmp/res_imp";
/// IVF centroids: count (u32), dims (u32), then f32 values.
pub static CENTROPOIDS_BIN: &[u8] = include_bytes!("../../../data/ivf/centroids.bin");

/// IVF centroids on GPU.
pub static VECTOR_CENTROPOIDS: Lazy<Arc<CentroidIndex>> =
    Lazy::new(|| Arc::new(CentroidIndex::new(CENTROPOIDS_BIN).unwrap()));
/// IVF centroids loaded on GPU, with CUDA kernels.
///
/// Assigns vectors to buckets and searches buckets.
#[cfg(feature = "cuda")]
pub struct CentroidIndex {
    ctx: Arc<CudaContext>,
    stream: Arc<CudaStream>,
    blas: CudaBlas,
    centroids_gpu: CudaSlice<f32>,
    num_centroids: usize,
    dims: usize,
    normalize_fn: cudarc::driver::CudaFunction,
    argmax_fn: cudarc::driver::CudaFunction,
    topk_fn: cudarc::driver::CudaFunction,
}

#[cfg(feature = "cuda")]
impl CentroidIndex {
    /// Loads centroids to GPU and compiles kernels.
    ///
    /// # Errors
    ///
    /// If data is short or CUDA fails.
    fn new(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut cursor = Cursor::new(data);
        let mut buf = [0u8; 4];

        cursor.read_exact(&mut buf)?;
        let num_centroids = u32::from_le_bytes(buf) as usize;
        cursor.read_exact(&mut buf)?;
        let dims = u32::from_le_bytes(buf) as usize;

        let mut floats = vec![0f32; num_centroids * dims];
        for val in floats.iter_mut() {
            let mut b = [0u8; 4];
            cursor.read_exact(&mut b)?;
            *val = f32::from_le_bytes(b);
        }

        let ctx = CudaContext::new(0)?;
        let stream = ctx.default_stream();
        let mut centroids_gpu = unsafe { stream.alloc::<f32>(floats.len())? };
        stream.memcpy_htod(&floats, &mut centroids_gpu)?;
        let blas = CudaBlas::new(stream.clone())?;

        let kernel_src = r#"
extern "C" __global__ void normalize_rows(
    float* data,
    int rows,
    int cols
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= rows) return;

    float* row = data + (long long)i * cols;
    float norm = 0.0f;
    for (int j = 0; j < cols; j++) {
        norm += row[j] * row[j];
    }
    norm = sqrtf(norm);
    if (norm < 1e-10f) norm = 1e-10f;
    for (int j = 0; j < cols; j++) {
        row[j] /= norm;
    }
}

extern "C" __global__ void row_argmax(
    const float* sims,
    int* argmax,
    int num_centroids,
    int batch_size
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= batch_size) return;

    const float* row = sims + (long long)i * num_centroids;
    float best_val = -1e38f;
    int best_idx = 0;
    for (int j = 0; j < num_centroids; j++) {
        float v = row[j];
        if (v > best_val) {
            best_val = v;
            best_idx = j;
        }
    }
    argmax[i] = best_idx;
}
extern "C" __global__ void topk_argmax(
    const float* sims,
    int* top_k_indices,
    int num_centroids,
    int num_queries,
    int T
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= num_queries) return;

    const float* row = sims + (long long)i * num_centroids;
    int   best_idx[32];

    for (int t = 0; t < T; t++) {
        float bv = -1e38f;
        int   bi = 0;
        for (int j = 0; j < num_centroids; j++) {
            bool already = false;
            for (int p = 0; p < t; p++) if (best_idx[p] == j) { already = true; break; }
            if (!already && row[j] > bv) { bv = row[j]; bi = j; }
        }
        best_idx[t] = bi;
    }
    for (int t = 0; t < T; t++) {
        top_k_indices[(long long)i * T + t] = best_idx[t];
    }
}
"#;

        let ptx = compile_ptx(kernel_src)?;
        let module = ctx.load_module(ptx)?;
        let normalize_fn = module.load_function("normalize_rows")?;
        let argmax_fn = module.load_function("row_argmax")?;
        let topk_fn = module.load_function("topk_argmax")?;

        println!("✅ Centroids loaded to GPU: {}x{}", num_centroids, dims);
        Ok(CentroidIndex {
            ctx,
            stream,
            blas,
            centroids_gpu,
            num_centroids,
            dims,
            normalize_fn,
            argmax_fn,
            topk_fn,
        })
    }

    /// Bucket of every vector: the centroid with highest cosine similarity.
    ///
    /// # Arguments
    ///
    /// * `vectors` - Vectors of [crate::VECTOR_DIM].
    ///
    /// # Returns
    ///
    /// Bucket id per vector, same order.
    ///
    /// # Errors
    ///
    /// If CUDA fails.
    pub fn assign_batch(
        &self,
        vectors: &[Vec<f32>],
    ) -> Result<Vec<usize>, Box<dyn Error + Send + Sync>> {
        let batch_size = vectors.len();
        let dims = self.dims;

        // FLATTEN (CPU)
        let flat: Vec<f32> = vectors.iter().flatten().copied().collect();

        // COPY H→D
        let mut queries_gpu = unsafe { self.stream.alloc::<f32>(flat.len())? };
        self.stream.memcpy_htod(&flat, &mut queries_gpu)?;

        // NORMALIZE ON GPU
        let threads = 256u32;
        let blocks = ((batch_size as u32) + threads - 1) / threads;
        let cfg = LaunchConfig {
            grid_dim: (blocks, 1, 1),
            block_dim: (threads, 1, 1),
            shared_mem_bytes: 0,
        };
        let rows_i32 = batch_size as i32;
        let cols_i32 = dims as i32;
        unsafe {
            let mut builder = self.stream.launch_builder(&self.normalize_fn);
            builder.arg(&mut queries_gpu);
            builder.arg(&rows_i32);
            builder.arg(&cols_i32);
            builder.launch(cfg)?;
        }
        self.stream.synchronize()?;

        // ALLOC SIM BUFFER
        let mut sims_gpu = self
            .stream
            .alloc_zeros::<f32>(batch_size * self.num_centroids)?;

        // GEMM
        unsafe {
            self.blas.gemm(
                GemmConfig {
                    transa: cublasOperation_t::CUBLAS_OP_T,
                    transb: cublasOperation_t::CUBLAS_OP_N,
                    m: self.num_centroids as i32,
                    n: batch_size as i32,
                    k: dims as i32,
                    alpha: 1.0f32,
                    lda: dims as i32,
                    ldb: dims as i32,
                    beta: 0.0f32,
                    ldc: self.num_centroids as i32,
                },
                &self.centroids_gpu,
                &queries_gpu,
                &mut sims_gpu,
            )?;
        }
        self.stream.synchronize()?;

        // ARGMAX ON GPU
        let mut argmax_gpu = self.stream.alloc_zeros::<i32>(batch_size)?;
        let num_centroids_i32 = self.num_centroids as i32;
        let batch_size_i32 = batch_size as i32;
        unsafe {
            let mut builder = self.stream.launch_builder(&self.argmax_fn);
            builder.arg(&sims_gpu);
            builder.arg(&mut argmax_gpu);
            builder.arg(&num_centroids_i32);
            builder.arg(&batch_size_i32);
            builder.launch(cfg)?;
        }
        self.stream.synchronize()?;

        // COPY ARGMAX D→H
        let mut argmax_cpu = vec![0i32; batch_size];
        self.stream.memcpy_dtoh(&argmax_gpu, &mut argmax_cpu)?;

        let result: Vec<usize> = argmax_cpu.into_iter().map(|x| x as usize).collect();

        Ok(result)
    }

    /// Searches vector index.
    ///
    /// Finds `t` closest centroids on GPU, then reads their '.bin.zst' buckets
    /// in parallel and scores every vector by cosine similarity on CPU.
    /// Same id in more buckets keeps its best score.
    ///
    /// # Arguments
    ///
    /// * `query` - Query vector.
    /// * `n` - Max results, 0 means all.
    /// * `t` - Buckets to read, max 32.
    /// * `vector_path` - Directory with buckets.
    ///
    /// # Returns
    ///
    /// (page id, similarity), best first.
    ///
    /// # Errors
    ///
    /// If CUDA fails.
    ///
    /// # Panics
    ///
    /// If `t` is over 32.
    pub fn search(
        &self,
        query: &[f32],
        n: usize,
        t: usize,
        vector_path: &str,
    ) -> Result<Vec<(u64, f32)>, Box<dyn std::error::Error>> {
        assert!(t <= 32, "t must be <=32");
        let dims = self.dims;

        // Upload + normalize query on GPU
        let mut q_gpu = unsafe { self.stream.alloc::<f32>(dims)? };
        self.stream.memcpy_htod(query, &mut q_gpu)?;

        let rows_i32 = 1i32;
        let cols_i32 = dims as i32;
        let cfg1 = LaunchConfig {
            grid_dim: (1, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        unsafe {
            let mut b = self.stream.launch_builder(&self.normalize_fn);
            b.arg(&mut q_gpu);
            b.arg(&rows_i32);
            b.arg(&cols_i32);
            b.launch(cfg1)?;
        }
        self.stream.synchronize()?;

        // GEMM: sims = centroids^T · query → (num_centroids × 1)
        let mut sims_gpu = self.stream.alloc_zeros::<f32>(self.num_centroids)?;
        unsafe {
            self.blas.gemm(
                GemmConfig {
                    transa: cublasOperation_t::CUBLAS_OP_T,
                    transb: cublasOperation_t::CUBLAS_OP_N,
                    m: self.num_centroids as i32,
                    n: 1i32,
                    k: dims as i32,
                    alpha: 1.0f32,
                    lda: dims as i32,
                    ldb: dims as i32,
                    beta: 0.0f32,
                    ldc: self.num_centroids as i32,
                },
                &self.centroids_gpu,
                &q_gpu,
                &mut sims_gpu,
            )?;
        }
        self.stream.synchronize()?;

        // Top-T argmax on GPU
        let mut topk_gpu = self.stream.alloc_zeros::<i32>(t)?;
        let nc_i32 = self.num_centroids as i32;
        let one_i32 = 1i32;
        let t_i32 = t as i32;
        unsafe {
            let mut b = self.stream.launch_builder(&self.topk_fn);
            b.arg(&sims_gpu);
            b.arg(&mut topk_gpu);
            b.arg(&nc_i32);
            b.arg(&one_i32);
            b.arg(&t_i32);
            b.launch(LaunchConfig {
                grid_dim: (1, 1, 1),
                block_dim: (1, 1, 1),
                shared_mem_bytes: 0,
            })?;
        }
        self.stream.synchronize()?;

        let mut topk_cpu = vec![0i32; t];
        self.stream.memcpy_dtoh(&topk_gpu, &mut topk_cpu)?;
        let centroid_ids: Vec<usize> = topk_cpu.into_iter().map(|x| x as usize).collect();

        // Normalize query on CPU for cosine scan
        let norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
        let query_norm: Vec<f32> = query.iter().map(|x| x / norm).collect();

        // Scan probed buckets in parallel, cosine similarity
        let all: Vec<Vec<(u64, f32)>> = centroid_ids
            .par_iter()
            .filter_map(|&cid| {
                let zst_path = format!("{}/bucket_{:06}.bin.zst", vector_path, cid);
                if !Path::new(&zst_path).exists() {
                    return None;
                }

                if let Ok(meta) = std::fs::metadata(&zst_path) {
                    if meta.len() > 1_073_741_824 {
                        return None;
                    }
                } else {
                    return None; // Skip if we can't read file metadata
                }

                let compressed = std::fs::read(&zst_path).ok()?;
                let data = decode_all(compressed.as_slice()).ok()?;
                let count = data.len() / RECORD_SIZE;
                let mut results = Vec::with_capacity(count);

                for i in 0..count {
                    let base = i * RECORD_SIZE;
                    let id = u64::from_le_bytes(data[base..base + ID_SIZE].try_into().unwrap());
                    let slice: &[f32] =
                        bytemuck::cast_slice(&data[base + ID_SIZE..base + RECORD_SIZE]);

                    let dot: f32 = slice
                        .iter()
                        .zip(query_norm.iter())
                        .map(|(v, q)| v * q)
                        .sum();

                    results.push((id, dot));
                }

                Some(results)
            })
            .collect();

        // Flatten, dedup, sort, truncate
        let mut candidates: Vec<(u64, f32)> = all.into_iter().flatten().collect();
        candidates.sort_unstable_by_key(|(id, _)| *id);
        candidates.dedup_by(|a, b| {
            if a.0 == b.0 {
                if a.1 > b.1 {
                    b.1 = a.1;
                }
                true
            } else {
                false
            }
        });
        candidates.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        if n > 0 {
            candidates.truncate(n);
        }

        Ok(candidates)
    }
}
/// Placeholder without `cuda` feature. Vector search is off.
#[cfg(not(feature = "cuda"))]
pub struct CentroidIndex;

#[cfg(not(feature = "cuda"))]
impl CentroidIndex {
    /// Prints that CUDA is off.
    pub fn new(_: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        println!("CUDA feature not enabled");
        Ok(Self)
    }

    /// Returns no buckets.
    pub fn assign_batch(&self, _: &[Vec<f32>]) -> Result<Vec<usize>, Box<dyn Error + Send + Sync>> {
        println!("CUDA feature not enabled");
        Ok(Vec::new())
    }

    /// Always an error.
    pub fn search(
        &self,
        _: &[f32],
        _: usize,
        _: usize,
        _: &str,
    ) -> Result<Vec<(u64, f32)>, Box<dyn std::error::Error>> {
        Err("CUDA feature not enabled".into())
    }
}

/*
  PageRank
*/
/// PageRank scores, `None` when files are missing. Can be swapped at runtime.
pub static PAGERANK: Lazy<RwLock<Option<Arc<PageRank>>>> =
    Lazy::new(|| RwLock::new(PageRank::open(ID_MAP_FILE, FINAL_SCORES).ok().map(Arc::new)));

/// Memory mapped PageRank files.
///
/// * [crate::ID_MAP_FILE] - Sorted (url id, node id) pairs.
/// * [crate::FINAL_SCORES] - f32 score per node id.
pub struct PageRank {
    /// Keeps id map mapped.
    _id_mmap: Mmap,
    /// Start of id map.
    ptr: *const (u64, u64),
    /// Number of pairs.
    len: usize,
    /// Keeps scores mapped.
    _scores_mmap: Mmap,
    /// Start of scores.
    scores_ptr: *const f32,
}

unsafe impl Send for PageRank {}
unsafe impl Sync for PageRank {}

impl PageRank {
    /// Maps both files.
    ///
    /// # Errors
    ///
    /// If a file can't be opened or mapped.
    pub fn open(id_map_path: &str, scores_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let id_file = File::open(id_map_path)?;
        let id_mmap = unsafe { MmapOptions::new().map(&id_file)? };
        id_mmap.advise(memmap2::Advice::Random)?;

        let len = id_mmap.len() / 16;
        let ptr = id_mmap.as_ptr() as *const (u64, u64);

        let scores_file = File::open(scores_path)?;
        let scores_mmap = unsafe { MmapOptions::new().map(&scores_file)? };
        scores_mmap.advise(memmap2::Advice::Random)?;

        let scores_ptr = scores_mmap.as_ptr() as *const f32;

        Ok(Self {
            _id_mmap: id_mmap,
            ptr,
            len,
            _scores_mmap: scores_mmap,
            scores_ptr,
        })
    }

    /// All (url id, node id) pairs.
    pub fn pairs(&self) -> &[(u64, u64)] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Node id of url id, binary search.
    pub fn lookup(&self, hash: u64) -> Option<u64> {
        let pairs = self.pairs();
        pairs
            .binary_search_by_key(&hash, |&(h, _)| h)
            .ok()
            .map(|i| pairs[i].1)
    }

    /// PageRank of URL, normalized by [crate::normalize_url]. 0 when unknown.
    pub fn get_score(&self, url: &str) -> f32 {
        let hash = url_to_id(&normalize_url(url.trim()));
        let node_id = match self.lookup(hash) {
            Some(id) => id,
            None => return 0.0,
        };

        unsafe { *self.scores_ptr.add(node_id as usize) }
    }
}

/*
  Reranker
*/
/// Tokenizer of [crate::BGE_MODEL].
pub static BGE_TOKENIZER: &[u8] = include_bytes!("../../../data/bge/tokenizer.json");
/// BGE reranker.
pub static RERANKER: Lazy<Reranker> = Lazy::new(Reranker::new);

/// Cross encoder scoring how well a passage answers a query.
pub struct Reranker {
    /// ONNX session on CUDA.
    pub session: Arc<Session>,
    /// Tokenizer, max 512 tokens.
    pub tokenizer: Arc<Tokenizer>,
}

impl Reranker {
    /// Loads [crate::BGE_MODEL] on CUDA.
    ///
    /// # Panics
    ///
    /// If model or tokenizer can't be loaded.
    pub fn new() -> Self {
        let environment = Environment::builder()
            .with_name("reranker")
            .with_log_level(ort::LoggingLevel::Verbose)
            .build()
            .unwrap()
            .into_arc();

        let session = SessionBuilder::new(&environment)
            .expect("Failed to create SessionBuilder")
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .expect("Failed to set optimization level")
            .with_execution_providers([ExecutionProvider::CUDA(
                ort::execution_providers::CUDAExecutionProviderOptions {
                    enable_cuda_graph: false,
                    ..Default::default()
                },
            )])
            .expect("Failed to attach CUDA provider")
            .with_model_from_file(BGE_MODEL)
            .expect("Failed to load BGE model");

        let mut tokenizer = Tokenizer::from_bytes(BGE_TOKENIZER).unwrap();

        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_to_multiple_of: Some(64),
            ..Default::default()
        }));

        if let Err(e) = tokenizer.with_truncation(Some(TruncationParams {
            max_length: 512,
            ..Default::default()
        })) {
            println!(
                "{}Failed to set tokenizer truncation!{} {}",
                colors::RED,
                colors::RESET,
                e
            );
        };

        Self {
            session: Arc::new(session),
            tokenizer: Arc::new(tokenizer),
        }
    }

    /// Scores passages for a query in one batch.
    ///
    /// # Arguments
    ///
    /// * `query` - Query.
    /// * `passages` - Texts to score.
    ///
    /// # Returns
    ///
    /// Score per passage, same order. Higher is better.
    ///
    /// # Errors
    ///
    /// If model fails.
    pub async fn score_batch(
        &self,
        query: &str,
        passages: &[String],
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        if passages.is_empty() {
            return Ok(Vec::new());
        }

        let query = query.to_string();
        let passages = passages.to_vec();

        let session = self.session.clone();
        let tokenizer = self.tokenizer.clone();

        let scores = tokio::task::spawn_blocking(
            move || -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
                let batch_size = passages.len();

                let input_pairs: Vec<(&str, &str)> = passages
                    .iter()
                    .map(|p| (query.as_str(), p.as_str()))
                    .collect();

                let encodings = tokenizer
                    .encode_batch(input_pairs, true)
                    .map_err(|e| e.to_string())?;

                let seq_len = encodings[0].get_ids().len();

                let mut ids = Vec::with_capacity(batch_size * seq_len);
                let mut mask = Vec::with_capacity(batch_size * seq_len);

                for enc in &encodings {
                    ids.extend(enc.get_ids().iter().map(|&x| x as i32));
                    mask.extend(enc.get_attention_mask().iter().map(|&x| x as i32));
                }

                let input_ids_arr = Array2::from_shape_vec((batch_size, seq_len), ids).unwrap();
                let attention_mask_arr =
                    Array2::from_shape_vec((batch_size, seq_len), mask).unwrap();

                let input_ids = CowArray::from(input_ids_arr).into_dyn();
                let attention_mask = CowArray::from(attention_mask_arr).into_dyn();

                let input_ids_val = Value::from_array(session.allocator(), &input_ids).unwrap();
                let attention_mask_val =
                    Value::from_array(session.allocator(), &attention_mask).unwrap();

                let outputs = session.run(vec![input_ids_val, attention_mask_val])?;

                let scores_tensor = outputs[0].try_extract::<f32>()?;
                let view = scores_tensor.view();

                let mut results = Vec::with_capacity(batch_size);
                if view.ndim() == 2 {
                    for i in 0..batch_size {
                        results.push(view[[i, 0]]);
                    }
                } else {
                    for i in 0..batch_size {
                        results.push(view[[i]]);
                    }
                }

                Ok(results)
            },
        )
        .await??;

        Ok(scores)
    }
}

impl Default for Reranker {
    fn default() -> Self {
        Self::new()
    }
}

/*
  Synonyms
*/
/// Synonyms of one language.
pub struct LangSynonyms {
    /// Word → row.
    index: FstMap<&'static [u8]>,
    /// Synonyms per row.
    rows: Vec<Box<[Box<str>]>>,
}

/// Built in synonym files of a language: (fst, rows).
fn get_synonym_bytes(lang: &str) -> Option<(&'static [u8], &'static [u8])> {
    match lang {
        "ar" => Some((
            include_bytes!("../../../data/synonyms/ar.fst"),
            include_bytes!("../../../data/synonyms/ar.rows.bin"),
        )),
        "bg" => Some((
            include_bytes!("../../../data/synonyms/bg.fst"),
            include_bytes!("../../../data/synonyms/bg.rows.bin"),
        )),
        "ca" => Some((
            include_bytes!("../../../data/synonyms/ca.fst"),
            include_bytes!("../../../data/synonyms/ca.rows.bin"),
        )),
        "da" => Some((
            include_bytes!("../../../data/synonyms/da.fst"),
            include_bytes!("../../../data/synonyms/da.rows.bin"),
        )),
        "en" => Some((
            include_bytes!("../../../data/synonyms/en.fst"),
            include_bytes!("../../../data/synonyms/en.rows.bin"),
        )),
        "et" => Some((
            include_bytes!("../../../data/synonyms/et.fst"),
            include_bytes!("../../../data/synonyms/et.rows.bin"),
        )),
        "fi" => Some((
            include_bytes!("../../../data/synonyms/fi.fst"),
            include_bytes!("../../../data/synonyms/fi.rows.bin"),
        )),
        "he" => Some((
            include_bytes!("../../../data/synonyms/he.fst"),
            include_bytes!("../../../data/synonyms/he.rows.bin"),
        )),
        "hr" => Some((
            include_bytes!("../../../data/synonyms/hr.fst"),
            include_bytes!("../../../data/synonyms/hr.rows.bin"),
        )),
        "hu" => Some((
            include_bytes!("../../../data/synonyms/hu.fst"),
            include_bytes!("../../../data/synonyms/hu.rows.bin"),
        )),
        "is" => Some((
            include_bytes!("../../../data/synonyms/is.fst"),
            include_bytes!("../../../data/synonyms/is.rows.bin"),
        )),
        "lt" => Some((
            include_bytes!("../../../data/synonyms/lt.fst"),
            include_bytes!("../../../data/synonyms/lt.rows.bin"),
        )),
        "lv" => Some((
            include_bytes!("../../../data/synonyms/lv.fst"),
            include_bytes!("../../../data/synonyms/lv.rows.bin"),
        )),
        "nb" => Some((
            include_bytes!("../../../data/synonyms/nb.fst"),
            include_bytes!("../../../data/synonyms/nb.rows.bin"),
        )),
        "nn" => Some((
            include_bytes!("../../../data/synonyms/nn.fst"),
            include_bytes!("../../../data/synonyms/nn.rows.bin"),
        )),
        "no" => Some((
            include_bytes!("../../../data/synonyms/no.fst"),
            include_bytes!("../../../data/synonyms/no.rows.bin"),
        )),
        "ro" => Some((
            include_bytes!("../../../data/synonyms/ro.fst"),
            include_bytes!("../../../data/synonyms/ro.rows.bin"),
        )),
        "sk" => Some((
            include_bytes!("../../../data/synonyms/sk.fst"),
            include_bytes!("../../../data/synonyms/sk.rows.bin"),
        )),
        "sl" => Some((
            include_bytes!("../../../data/synonyms/sl.fst"),
            include_bytes!("../../../data/synonyms/sl.rows.bin"),
        )),
        "sr" => Some((
            include_bytes!("../../../data/synonyms/sr.fst"),
            include_bytes!("../../../data/synonyms/sr.rows.bin"),
        )),
        "sv" => Some((
            include_bytes!("../../../data/synonyms/sv.fst"),
            include_bytes!("../../../data/synonyms/sv.rows.bin"),
        )),
        _ => None,
    }
}

/// Words like 'near' in all languages. They are not expanded with synonyms.
pub static LOCAL_NO_EXPAND: Lazy<AHashSet<&'static str>> = Lazy::new(|| {
    let words = vec![
        // 🇬🇧English (en)
        "near",
        "nearby",
        "close",
        "closest",
        "around",
        "by",
        "at",
        "vicinity",
        "me",
        "my",
        "here",
        "myself",
        "area",
        "neighborhood",
        "town",
        "city",
        "location",
        "place",
        "district",
        "street",
        "zip",
        "open",
        "now",
        "today",
        "tonight",
        "hours",
        "map",
        "directions",
        "distance",
        "walking",
        "driving",
        "route",
        "miles",
        "km",
        // 🇸🇰 Slovak (sk) & 🇨🇿 Czech (cs - similar behavior)
        "blízko",
        "blizko",
        "najbližšie",
        "najblizsie",
        "pri",
        "okolo",
        "mne",
        "mi",
        "tu",
        "oblasť",
        "oblast",
        "mesto",
        "miesto",
        "ulica",
        "otvorené",
        "otvorene",
        "teraz",
        "dnes",
        "večer",
        "hodiny",
        "mapa",
        "trasa",
        "vzdialenosť",
        // 🇭🇷 Croatian (hr) & 🇷🇸 Serbian (sr)
        "blizu",
        "najbliže",
        "oko",
        "kod",
        "meni",
        "mi",
        "ovdje",
        "ovde",
        "područje",
        "grad",
        "mjesto",
        "mesto",
        "otvoreno",
        "sada",
        "danas",
        "večeras",
        "mapa",
        "karta",
        "ruta",
        "udaljenost",
        // 🇸🇮 Slovenian (sl)
        "blizu",
        "najbližje",
        "okoli",
        "pri",
        "meni",
        "mi",
        "tukaj",
        "območje",
        "mesto",
        "kraj",
        "odprto",
        "zdaj",
        "danes",
        "zemljevid",
        // 🇧🇬 Bulgarian (bg)
        "близо",
        "най-близо",
        "около",
        "при",
        "мен",
        "ми",
        "тук",
        "район",
        "град",
        "място",
        "отворено",
        "сега",
        "днес",
        "карта",
        // 🇷🇴 Romanian (ro)
        "aproape",
        "lângă",
        "langa",
        "în jur",
        "mine",
        "aici",
        "zonă",
        "zona",
        "oraș",
        "oras",
        "loc",
        "stradă",
        "deschis",
        "acum",
        "astăzi",
        "hartă",
        "harta",
        // 🇭🇺 Hungarian (hu)
        "közel",
        "legközelebb",
        "körül",
        "mellett",
        "nekem",
        "itt",
        "terület",
        "város",
        "hely",
        "utca",
        "nyitva",
        "most",
        "ma",
        "térkép",
        // 🇸🇪 Swedish (sv), 🇳🇴 Norwegian (no/nb/nn), 🇩🇰 Danish (da)
        "nära",
        "nær",
        "närmaste",
        "nærmeste",
        "runt",
        "rundt",
        "mig",
        "meg",
        "här",
        "her",
        "område",
        "stad",
        "by",
        "plats",
        "sted",
        "öppen",
        "åpen",
        "åben",
        "nu",
        "idag",
        "i dag",
        "karta",
        "kart",
        "kort",
        "väg",
        "vei",
        // 🇫🇮 Finnish (fi)
        "lähellä",
        "lähin",
        "ympärillä",
        "minua",
        "minun",
        "täällä",
        "alue",
        "kaupunki",
        "paikka",
        "katu",
        "auki",
        "avoinna",
        "nyt",
        "tänään",
        "kartta",
        // 🇪🇪 Estonian (et)
        "lähedal",
        "lähim",
        "ümber",
        "minu",
        "siin",
        "piirkond",
        "linn",
        "koht",
        "avatud",
        "praegu",
        "täna",
        "kaart",
        // 🇱🇻 Latvian (lv) & 🇱🇹 Lithuanian (lt)
        "tuvu",
        "tuvākais",
        "arti",
        "arčiausiai",
        "aplink",
        "man",
        "šeit",
        "čia",
        "rajons",
        "rajonas",
        "pilsēta",
        "miestas",
        "vieta",
        "atvērts",
        "atidaryta",
        "tagad",
        "dabar",
        "šodien",
        "šiandien",
        "karte",
        "žemėlapis",
        // 🇮🇸 Icelandic (is)
        "nálægt",
        "næst",
        "kringum",
        "mig",
        "hér",
        "svæði",
        "borg",
        "staður",
        "opið",
        "núna",
        "í dag",
        "kort",
        // 🇦🇩 Catalan (ca)
        "prop",
        "a prop",
        "més proper",
        "voltant",
        "mi",
        "aquí",
        "àrea",
        "ciutat",
        "lloc",
        "carrer",
        "obert",
        "ara",
        "avui",
        "mapa",
        // 🇮🇱 Hebrew (he)
        "קרוב",
        "הכי קרוב",
        "סביב",
        "לי",
        "שלי",
        "כאן",
        "אזור",
        "עיר",
        "מקום",
        "רחוב",
        "פתוח",
        "עכשיו",
        "היום",
        "מפה",
        "כיוונים",
        // 🇸🇦 Arabic (ar)
        "قريب",
        "الأقرب",
        "حولي",
        "بجانبي",
        "لي",
        "هنا",
        "منطقة",
        "مدينة",
        "مكان",
        "شارع",
        "مفتوح",
        "الآن",
        "اليوم",
        "خريطة",
        "اتجاهات",
    ];

    let mut set = AHashSet::with_capacity(words.len());
    for w in words {
        set.insert(w);
    }
    set
});

impl LangSynonyms {
    /// Loads synonyms of `lang`, `None` if there are none.
    fn load(lang: &str) -> Option<Arc<LangSynonyms>> {
        let (fst_bytes, rows_bytes) = get_synonym_bytes(lang)?;

        let index = FstMap::new(fst_bytes).ok()?;

        let (rows, _bytes_read): (Vec<Box<[Box<str>]>>, usize) =
            bincode_next::decode_from_slice(rows_bytes, bincode_next::config::standard()).ok()?;

        Some(Arc::new(LangSynonyms { index, rows }))
    }

    /// Synonyms of `term`.
    pub fn lookup(&self, term: &str) -> Option<&[Box<str>]> {
        let id = self.index.get(term)?;
        self.rows.get(id as usize).map(|r| r.as_ref())
    }
}

/// Loaded synonyms per language.
static SYNONYM_STORES: Lazy<RwLock<std::collections::HashMap<String, Option<Arc<LangSynonyms>>>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));
/// Synonyms of `lang`, loaded on first use. `None` for unsupported languages.
pub fn get_store(lang: &str) -> Option<Arc<LangSynonyms>> {
    if let Some(entry) = SYNONYM_STORES.read().get(lang) {
        return entry.clone();
    }
    let mut w = SYNONYM_STORES.write();
    if let Some(entry) = w.get(lang) {
        return entry.clone();
    }
    let loaded = LangSynonyms::load(lang)?;
    w.insert(lang.to_string(), Some(loaded.clone()));
    Some(loaded)
}

/*
  Intent
*/
/// Query intent, the same categories as Google uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    /// Learn information.
    Informational,
    /// Action: purchase, download, sign up.
    Transactional,
    /// Research, comparing options.
    CommercialInvestigation,
    /// Reach a known domain or page.
    Navigational,
    /// Near me businesses.
    Local,
    /// Failed to determine.
    Unknown,
}

/// Intent keywords of one language.
pub struct LangMatcher {
    /// Keyword matcher.
    pub automaton: AhoCorasick,
    /// Intent per keyword.
    pub intent_map: Vec<QueryIntent>,
}

/// Keywords of one language in 'intent.json'.
#[derive(Deserialize)]
struct LanguageKeywords {
    #[serde(default)]
    informational_keywords: Vec<String>,
    #[serde(default)]
    transactional_keywords: Vec<String>,
    #[serde(default)]
    commercial_keywords: Vec<String>,
    #[serde(default)]
    local_keywords: Vec<String>,
    #[serde(default)]
    navigational_keywords: Vec<String>,
}

/// 'intent.json', language code → keywords.
#[derive(Deserialize)]
struct IntentConfig {
    #[serde(flatten)]
    languages: AHashMap<String, LanguageKeywords>,
}

/// Language code → [LangMatcher], from 'intent.json'.
pub static MATCHERS: Lazy<AHashMap<String, LangMatcher>> = Lazy::new(|| {
    let mut matchers = AHashMap::new();

    let json_data = include_str!("../../../data/intent.json");
    let config: IntentConfig = serde_json::from_str(json_data)
        .expect("Error: intents.json format does not match required schema.");

    for (lang_code, keywords_struct) in config.languages {
        let mut patterns = Vec::new();
        let mut intent_map = Vec::new();

        let mut add_patterns = |keywords: Vec<String>, intent: QueryIntent| {
            for kw in keywords {
                let trimmed = kw.trim().to_lowercase();
                if !trimmed.is_empty() {
                    patterns.push(trimmed);
                    intent_map.push(intent);
                }
            }
        };

        add_patterns(
            keywords_struct.informational_keywords,
            QueryIntent::Informational,
        );
        add_patterns(
            keywords_struct.transactional_keywords,
            QueryIntent::Transactional,
        );
        add_patterns(
            keywords_struct.commercial_keywords,
            QueryIntent::CommercialInvestigation,
        );
        add_patterns(keywords_struct.local_keywords, QueryIntent::Local);
        add_patterns(
            keywords_struct.navigational_keywords,
            QueryIntent::Navigational,
        );

        if !patterns.is_empty() {
            let automaton = AhoCorasick::builder()
                .ascii_case_insensitive(true)
                .match_kind(MatchKind::LeftmostFirst)
                .build(&patterns)
                .expect("Failed to build AhoCorasick state machine");

            matchers.insert(
                lang_code,
                LangMatcher {
                    automaton,
                    intent_map,
                },
            );
        }
    }

    matchers
});

/*
  Entities
*/
/// Kind of entity found in query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    PersonName,
    Business,
    Place,
}

/// Entity found in query.
#[derive(Debug, Clone)]
pub struct TaggedEntity {
    /// Byte range in lowercase query.
    pub range: Range<usize>,
    /// Kind.
    pub entity_type: EntityType,
    /// Matched name.
    pub matched_text: String,
}

/// Place location.
#[derive(Archive, Deserialize, Serialize, RkyvSerialize, RkyvDeserialize, Debug, Clone)]
pub struct GeoCoords {
    pub country: String,
    pub lon: f32,
    pub lat: f32,
}

/// Place name → locations, built in as rkyv.
#[derive(Archive, Deserialize, Serialize, RkyvSerialize, RkyvDeserialize, Debug, Clone)]
pub struct EntityRegistry {
    pub places: HashMap<String, Vec<GeoCoords>>,
}

/// Finds places, businesses and person names in query.
pub struct QueryUnderstandingPipeline {
    /// Matcher of all names.
    pub automaton: AhoCorasick,
    /// Kind and name per pattern.
    pub metadata_map: Vec<(EntityType, String)>,
    /// Places with coordinates.
    pub archived_places: &'static rkyv::Archived<EntityRegistry>,
}

/// Entity finder, from built in places and 'keywords_automaton.txt'.
pub static QU_PIPELINE: Lazy<QueryUnderstandingPipeline> = Lazy::new(|| {
    macro_rules! include_bytes_aligned {
        ($path:expr) => {{
            #[repr(C, align(16))]
            struct Aligned([u8; include_bytes!($path).len()]);
            static ALIGNED: Aligned = Aligned(*include_bytes!($path));
            &ALIGNED.0
        }};
    }

    let places_bytes = include_bytes_aligned!("../../../data/entities/places.rkyv");
    let archived_places =
        access::<Archived<EntityRegistry>, rkyv::rancor::Error>(&places_bytes[..]).unwrap();

    let keywords_str = include_str!("../../../data/entities/keywords_automaton.txt");

    let mut patterns = Vec::new();
    let mut metadata_map = Vec::new();

    for place_name in archived_places.places.keys() {
        let name_str = place_name.as_str().to_lowercase();
        if name_str.len() > 1 {
            patterns.push(name_str.clone());
            metadata_map.push((EntityType::Place, name_str));
        }
    }

    for line in keywords_str.lines() {
        if let Some((prefix, value)) = line.split_once('|') {
            if value.trim().len() < 2 {
                continue;
            }

            let etype = match prefix {
                "BIZ" => EntityType::Business,
                "NAME" => EntityType::PersonName,
                _ => continue,
            };
            patterns.push(value.to_string());
            metadata_map.push((etype, value.to_string()));
        }
    }

    let automaton = AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::LeftmostLongest)
        .build(&patterns)
        .unwrap();

    QueryUnderstandingPipeline {
        automaton,
        metadata_map,
        archived_places,
    }
});
impl QueryUnderstandingPipeline {
    /// Finds entities in query. Matches must be whole words.
    ///
    /// Longest match wins when names overlap.
    pub fn get_tags(&self, query: &str) -> Vec<TaggedEntity> {
        let query_lower = query.to_lowercase();
        let query_bytes = query_lower.as_bytes();

        self.automaton
            .find_iter(&query_lower)
            .filter_map(|mat| {
                let start = mat.start();
                let end = mat.end();

                let is_start_boundary =
                    start == 0 || !query_bytes[start - 1].is_ascii_alphanumeric();
                let is_end_boundary =
                    end == query_bytes.len() || !query_bytes[end].is_ascii_alphanumeric();

                if is_start_boundary && is_end_boundary {
                    let (etype, text) = &self.metadata_map[mat.pattern().as_usize()];
                    Some(TaggedEntity {
                        range: mat.range(),
                        entity_type: *etype,
                        matched_text: text.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

/*
  Server-side bangs
*/
/// Bang, read from unduck 'bangs.js'.
#[derive(Deserialize, Debug, Clone)]
pub struct Bang {
    /// Domain.
    pub d: String,
    /// Trigger, e.g. 'g' for !g.
    pub t: String,
    /// URL template, query goes to '{{{s}}}'.
    pub u: String,
}
/// Trigger → [Bang], for redirects without JavaScript.
pub static BANGS: Lazy<AHashMap<String, Bang>> = Lazy::new(|| {
    let content = read_file(&format!("{}/js/unduck/bangs.js", STATIC_DIR));
    let mut map = AHashMap::new();

    if content.is_empty() {
        return map;
    }

    let blocks = content.split("},{");

    for block in blocks {
        let t = extract_val(block, "t:\"");
        let d = extract_val(block, "d:\"");
        let u = extract_val(block, "u:\"");

        if let (Some(t_val), Some(d_val), Some(u_val)) = (t, d, u) {
            map.insert(
                t_val.clone(),
                Bang {
                    t: t_val,
                    d: d_val,
                    u: u_val,
                },
            );
        }
    }

    map
});
/// Value of `key"..."` in a 'bangs.js' block.
fn extract_val(block: &str, key: &str) -> Option<String> {
    let start = block.find(key)? + key.len();
    let end = block[start..].find('"')?;
    Some(block[start..start + end].to_string())
}

/// Known popular domains from 'domains.txt'.
pub static TOP_DOMAINS: Lazy<AHashSet<&'static str>> =
    Lazy::new(|| include_str!("../../../data/domains.txt").lines().collect());

/*
  Spell checker
*/
/// SymSpell with English words and bigrams.
pub static SPELL_CHECKER: Lazy<Arc<SymSpell<UnicodeStringStrategy>>> = Lazy::new(|| {
    let mut symspell: SymSpell<UnicodeStringStrategy> = SymSpell::default();

    let en_dict = include_str!("../../../data/spell_check/frequency_dictionary_en_82_765.txt")
        .trim_start_matches('\u{feff}');
    let mut word_count = 0;
    for line in en_dict.lines() {
        if symspell.load_dictionary_line(line, 0, 1, " ") {
            word_count += 1;
        }
    }
    println!(
        "{}Loaded English dictionary ({} words){}",
        colors::GREEN,
        word_count,
        colors::RESET
    );

    let en_bigrams =
        include_str!("../../../data/spell_check/frequency_bigramdictionary_en_243_342.txt")
            .trim_start_matches('\u{feff}');
    let mut bigram_count = 0;
    for line in en_bigrams.lines() {
        if symspell.load_bigram_dictionary_line(line, 0, 2, " ") {
            bigram_count += 1;
        }
    }
    println!(
        "{}Loaded English bigrams ({} entries){}",
        colors::GREEN,
        bigram_count,
        colors::RESET
    );

    Arc::new(symspell)
});

/*
  Analytics (only PriEco purely vibe coded part)
  They don't matter enough, and AI can do a good job in this
*/
/// Anonymous analytics of this node.
pub static ANALYTICS: Lazy<AnalyticsDb> = Lazy::new(|| {
    let date = OffsetDateTime::now_utc().date();

    AnalyticsDb {
        db: PRIECO_META.meta_db.clone(),
        ks: PRIECO_META.analytics_ks.clone(),

        salt: RwLock::new((date, generate_analytics_salt(date))),

        pending: Mutex::new(HashMap::new()),
        flush_lock: Mutex::new(()),
        last_flush: AtomicI64::new(OffsetDateTime::now_utc().unix_timestamp()),

        inc_lock: Mutex::new(()),
    }
});

/*
  Latency stages
*/
/// Measured part of search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Whole run_core_search (concurrent engines + fetch + sort)
    Core,
    /// Tantivy full text search
    Fts,
    /// Vector / IVF search
    Vector,
    /// Direct domain lookup
    Dir,
    /// Document fetch from the KV store
    Fetch,
    /// Handling an inbound federated query from a peer
    PeerServe,
    /// Round trip of an outbound federated query
    PeerRtt,
}

impl Stage {
    /// All stages, in showing order.
    pub const ALL: [Stage; 7] = [
        Stage::Core,
        Stage::Fts,
        Stage::Vector,
        Stage::Dir,
        Stage::Fetch,
        Stage::PeerServe,
        Stage::PeerRtt,
    ];

    /// Key used in storage.
    pub fn key(self) -> &'static str {
        match self {
            Stage::Core => "core",
            Stage::Fts => "fts",
            Stage::Vector => "ivf",
            Stage::Dir => "dir",
            Stage::Fetch => "fetch",
            Stage::PeerServe => "peer_serve",
            Stage::PeerRtt => "peer_rtt",
        }
    }

    /// Name for stats page.
    pub fn label(self) -> &'static str {
        match self {
            Stage::Core => "End to end",
            Stage::Fts => "Full text",
            Stage::Vector => "Vector",
            Stage::Dir => "Direct lookup",
            Stage::Fetch => "Doc fetch",
            Stage::PeerServe => "Peer serve",
            Stage::PeerRtt => "Peer round trip",
        }
    }

    /// Description for stats page.
    pub fn desc(self) -> &'static str {
        match self {
            Stage::Core => "All engines, fetch and sort",
            Stage::Fts => "Tantivy search",
            Stage::Vector => "IVF centroid search",
            Stage::Dir => "Domain guess lookup",
            Stage::Fetch => "Metadata read from storage",
            Stage::PeerServe => "Answering other nodes",
            Stage::PeerRtt => "Asking other nodes",
        }
    }
}

/*
  Log-scale histogram: 16 buckets per e-fold -> ~6.5% relative error on every percentile.
  Bucket b covers [e^(b/16), e^((b+1)/16)) microseconds.
*/
/// Buckets per e-fold of latency histogram, ~6.5% error.
const LAT_RES: f64 = 16.0;
/// Last bucket.
const LAT_MAX_BUCKET: u16 = 400;

/// Histogram bucket of latency in microseconds.
fn lat_bucket(micros: u64) -> u16 {
    ((micros.max(1) as f64).ln() * LAT_RES)
        .floor()
        .min(LAT_MAX_BUCKET as f64) as u16
}

/// Geometric middle of a bucket, in milliseconds
pub fn lat_bucket_ms(bucket: u16) -> f64 {
    (((bucket as f64) + 0.5) / LAT_RES).exp() / 1000.0
}

/// Percentiles in milliseconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct LatencySummary {
    /// Samples.
    pub count: u64,
    /// Median.
    pub p50: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
}

/// Percentile `q` (0..1) of histogram in ms, nearest rank. 0 when empty.
pub fn percentile(hist: &BTreeMap<u16, u64>, q: f64) -> f64 {
    let total: u64 = hist.values().sum();
    if total == 0 {
        return 0.0;
    }

    // Nearest-rank method
    let rank = ((q * total as f64).ceil() as u64).clamp(1, total);
    let mut cumulative = 0u64;

    for (bucket, count) in hist {
        cumulative += count;
        if cumulative >= rank {
            return lat_bucket_ms(*bucket);
        }
    }

    hist.keys()
        .next_back()
        .map(|b| lat_bucket_ms(*b))
        .unwrap_or(0.0)
}

/// [LatencySummary] of histogram.
pub fn summarize(hist: &BTreeMap<u16, u64>) -> LatencySummary {
    LatencySummary {
        count: hist.values().sum(),
        p50: percentile(hist, 0.50),
        p95: percentile(hist, 0.95),
        p99: percentile(hist, 0.99),
    }
}

/// Daily counters in the analytics keyspace.
///
/// No IPs or user agents are stored. Unique visitors are counted by a hash with a salt
/// that changes every day, so they can't be linked between days.
/// Everything older than 30 days is removed.
pub struct AnalyticsDb {
    /// Meta database.
    db: FJALL_DATABASE,
    /// Analytics keyspace.
    ks: Keyspace,
    /// Salt of today.
    salt: RwLock<(Date, [u8; 32])>,

    /// Buffered counter increments.
    pending: Mutex<HashMap<String, u64>>,
    /// One flush at a time.
    flush_lock: Mutex<()>,
    /// Unix time of last flush.
    last_flush: AtomicI64,

    /// One direct increment at a time.
    inc_lock: Mutex<()>,
}

impl AnalyticsDb {
    // Write
    /// Counts one search.
    pub fn record_query(&self) {
        self.inc(&format!("queries:{}", self.today()), 1);
    }

    /// Counts a page view and, once per day, a unique visitor and their country.
    ///
    /// # Arguments
    ///
    /// * `ip` - Visitor IP, only hashed.
    /// * `user_agent` - Only hashed.
    /// * `entity_id` - Counted thing, visitor is unique per entity.
    /// * `country_code` - Country from settings, 'all' is skipped.
    ///
    /// # Panics
    ///
    /// If write fails.
    pub fn record_visitor(
        &self,
        ip: &str,
        user_agent: &str,
        entity_id: &str,
        country_code: Option<&str>,
    ) {
        let date = self.today();

        let pvk = format!("pageviews:{}", date);
        let mut batch = self.db.batch();
        batch.insert(
            &self.ks,
            pvk.as_bytes(),
            (self.get_u64(&pvk) + 1).to_le_bytes(),
        );

        let seen_key = format!(
            "seen:{}:{}",
            date,
            self.fingerprint(ip, user_agent, entity_id, date)
        );

        if self.ks.get(seen_key.as_bytes()).ok().flatten().is_none() {
            batch.insert(&self.ks, seen_key.as_bytes(), [1u8]);

            let vk = format!("visitors:{}", date);
            batch.insert(
                &self.ks,
                vk.as_bytes(),
                (self.get_u64(&vk) + 1).to_le_bytes(),
            );

            if let Some(cc) = country_code
                && cc != "all"
            {
                let ck = format!("country:{}:{}", date, cc.to_lowercase());
                batch.insert(
                    &self.ks,
                    ck.as_bytes(),
                    (self.get_u64(&ck) + 1).to_le_bytes(),
                );
            }
        }

        batch.commit().expect("Analytics visitor write failed");
    }

    /// Counts one API request.
    pub fn record_api_request(&self) {
        let date = self.today();
        let mut batch = self.db.batch();
        let total_key = format!("api:total:{}", date);
        batch.insert(
            &self.ks,
            total_key.as_bytes(),
            (self.get_u64(&total_key) + 1).to_le_bytes(),
        );
        batch.commit().expect("API request tracking write failed");
    }

    /// Record one latency sample. Cheap: one mutex + hashmap increment.
    pub fn record_latency(&self, stage: Stage, elapsed: std::time::Duration) {
        let key = format!(
            "lat:{}:{}:{:03}",
            self.today(),
            stage.key(),
            lat_bucket(elapsed.as_micros() as u64)
        );
        *self.pending.lock().entry(key).or_insert(0) += 1;
        self.maybe_flush();
    }

    /// Generic daily counter, e.g. count("p2p_served")
    pub fn count(&self, name: &str) {
        self.count_by(name, 1);
    }

    /// Adds `n` to daily counter `name`. Buffered.
    pub fn count_by(&self, name: &str, n: u64) {
        if n == 0 {
            return;
        }
        let key = format!("ctr:{}:{}", self.today(), name);
        *self.pending.lock().entry(key).or_insert(0) += n;
        self.maybe_flush();
    }

    /// One mini crawler batch was accepted by the main crawler.
    /// Unique contributors are counted with the same daily-salted fingerprint as visitors.
    pub fn record_worker_batch(&self, ip: &str, pages: u64, bytes: u64) {
        let date = self.today();

        self.count("worker_batches");
        self.count_by("worker_pages", pages);
        self.count_by("worker_bytes", bytes);

        let seen_key = format!(
            "wseen:{}:{}",
            date,
            self.fingerprint(ip, "mini-crawler", "worker", date)
        );

        if self.ks.get(seen_key.as_bytes()).ok().flatten().is_none()
            && self.ks.insert(seen_key.as_bytes(), [1u8]).is_ok()
        {
            self.count("workers");
        }
    }

    // Read
    /// Searches per day, oldest first.
    pub fn daily_queries(&self, days: u32) -> Vec<(String, u64)> {
        let today = self.today();
        (0..days)
            .rev()
            .map(|i| {
                let date = today - Duration::days(i as i64);
                (date.to_string(), self.get_u64(&format!("queries:{}", date)))
            })
            .collect()
    }

    /// Unique visitors per day, oldest first.
    pub fn daily_visitors(&self, days: u32) -> Vec<(String, u64)> {
        let today = self.today();
        (0..days)
            .rev()
            .map(|i| {
                let date = today - Duration::days(i as i64);
                (
                    date.to_string(),
                    self.get_u64(&format!("visitors:{}", date)),
                )
            })
            .collect()
    }

    /// (visitors today, visitors yesterday, page views today, page views yesterday).
    pub fn visitor_stats(&self) -> (u64, u64, u64, u64) {
        let today = self.today();
        let yesterday = today - Duration::days(1);
        (
            self.get_u64(&format!("visitors:{}", today)),
            self.get_u64(&format!("visitors:{}", yesterday)),
            self.get_u64(&format!("pageviews:{}", today)),
            self.get_u64(&format!("pageviews:{}", yesterday)),
        )
    }

    /// Visitors per country today, most first.
    pub fn top_countries(&self) -> Vec<(String, u64)> {
        let prefix = format!("country:{}:", self.today());
        let mut results: Vec<(String, u64)> = self
            .ks
            .prefix(prefix.as_bytes())
            .filter_map(|item| {
                let (k, v) = match item.into_inner() {
                    Ok(kv) => kv,
                    Err(_) => return None,
                };

                let key_str = std::str::from_utf8(&k).ok()?;

                if !key_str.starts_with(&prefix) {
                    return None;
                }

                let cc = key_str.rsplit(':').next()?.to_string();
                let count = u64::from_le_bytes((&v[..]).try_into().unwrap_or([0; 8]));
                Some((cc, count))
            })
            .collect();
        results.sort_by_key(|a| Reverse(a.1));
        results
    }

    /// API requests per day, oldest first.
    pub fn daily_api_requests(&self, days: u32) -> Vec<(String, u64)> {
        let today = self.today();
        (0..days)
            .rev()
            .map(|i| {
                let date = today - Duration::days(i as i64);
                (
                    date.to_string(),
                    self.get_u64(&format!("api:{}:{}", "total", date)),
                )
            })
            .collect()
    }

    /// (API requests today, yesterday).
    pub fn api_stats_today_yesterday(&self) -> (u64, u64) {
        let today = self.today();
        let yesterday = today - Duration::days(1);
        (
            self.get_u64(&format!("api:total:{}", today)),
            self.get_u64(&format!("api:total:{}", yesterday)),
        )
    }

    /// Daily values of a generic counter, oldest first (last item = today)
    pub fn counter_series(&self, name: &str, days: u32) -> Vec<(String, u64)> {
        self.flush();
        let today = self.today();
        (0..days)
            .rev()
            .map(|i| {
                let date = today - Duration::days(i as i64);
                (
                    date.to_string(),
                    self.get_u64(&format!("ctr:{}:{}", date, name)),
                )
            })
            .collect()
    }

    /// Merged histogram of the last `days` days (1 = today only)
    pub fn latency_hist(&self, stage: Stage, days: u32) -> BTreeMap<u16, u64> {
        self.flush();
        let today = self.today();
        let mut merged: BTreeMap<u16, u64> = BTreeMap::new();

        for i in 0..days {
            let date = today - Duration::days(i as i64);
            for (bucket, count) in self.day_hist(date, stage) {
                *merged.entry(bucket).or_insert(0) += count;
            }
        }

        merged
    }

    /// [LatencySummary] of last `days` days.
    pub fn latency_summary(&self, stage: Stage, days: u32) -> LatencySummary {
        summarize(&self.latency_hist(stage, days))
    }

    /// Per-day percentiles, oldest first
    pub fn latency_daily(&self, stage: Stage, days: u32) -> Vec<(String, LatencySummary)> {
        self.flush();
        let today = self.today();
        (0..days)
            .rev()
            .map(|i| {
                let date = today - Duration::days(i as i64);
                (date.to_string(), summarize(&self.day_hist(date, stage)))
            })
            .collect()
    }

    // Purge
    /// Removes counters older than 30 days.
    ///
    /// # Panics
    ///
    /// If write fails.
    pub fn purge_expired(&self) {
        let cutoff = self.today() - Duration::days(30);
        let mut batch = self.db.batch();
        for prefix in [
            "queries:",
            "visitors:",
            "pageviews:",
            "country:",
            "seen:",
            "api:",
            "lat:",
            "ctr:",
            "wseen:",
        ] {
            for item in self.ks.prefix(prefix.as_bytes()) {
                let k = match item.key() {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let Ok(key_str) = std::str::from_utf8(&k) else {
                    continue;
                };

                // The date is not always the first segment ("api:total:2026-01-01"),
                // so take the first segment that parses as a date.
                let date = key_str[prefix.len()..].split(':').find_map(|part| {
                    Date::parse(part, format_description!("[year]-[month]-[day]")).ok()
                });

                if let Some(date) = date
                    && date < cutoff
                {
                    batch.remove(&self.ks, k);
                }
            }
        }
        batch.commit().expect("Analytics purge failed");
    }

    /// Purges now and every midnight UTC. Never returns.
    pub async fn background_purge_task(&self) {
        self.purge_expired();
        loop {
            let now = OffsetDateTime::now_utc();
            let secs_until_midnight = {
                let tomorrow = (now.date() + Duration::days(1)).midnight().assume_utc();
                (tomorrow - now).whole_seconds().max(0) as u64
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(secs_until_midnight)).await;
            self.purge_expired();
        }
    }

    // Buffered writes
    /// Write all buffered increments to the database. Safe to call from anywhere, any time.
    pub fn flush(&self) {
        let _guard = self.flush_lock.lock();

        let drained: HashMap<String, u64> = std::mem::take(&mut *self.pending.lock());
        self.last_flush.store(
            OffsetDateTime::now_utc().unix_timestamp(),
            Ordering::Relaxed,
        );

        if drained.is_empty() {
            return;
        }

        let mut batch = self.db.batch();
        for (key, delta) in drained {
            batch.insert(
                &self.ks,
                key.as_bytes(),
                (self.get_u64(&key) + delta).to_le_bytes(),
            );
        }

        if let Err(e) = batch.commit() {
            eprintln!("Analytics flush failed: {}", e);
        }
    }

    /// Flushes when last flush is 30s old.
    fn maybe_flush(&self) {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let last = self.last_flush.load(Ordering::Relaxed);

        if now - last >= 30
            && self
                .last_flush
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            self.flush();
        }
    }

    /// Latency histogram of one day.
    fn day_hist(&self, date: Date, stage: Stage) -> BTreeMap<u16, u64> {
        let prefix = format!("lat:{}:{}:", date, stage.key());
        let mut hist = BTreeMap::new();

        for item in self.ks.prefix(prefix.as_bytes()) {
            let Ok((k, v)) = item.into_inner() else {
                continue;
            };
            let Some(bucket) = std::str::from_utf8(&k)
                .ok()
                .and_then(|s| s.rsplit(':').next())
                .and_then(|s| s.parse::<u16>().ok())
            else {
                continue;
            };
            let count = u64::from_le_bytes((&v[..]).try_into().unwrap_or([0; 8]));
            hist.insert(bucket, count);
        }

        hist
    }

    // Helpers
    /// Today in UTC.
    fn today(&self) -> Date {
        OffsetDateTime::now_utc().date()
    }

    /// Counter value, 0 when missing.
    fn get_u64(&self, key: &str) -> u64 {
        self.ks
            .get(key.as_bytes())
            .ok()
            .flatten()
            .map(|v| u64::from_le_bytes((&v[..]).try_into().unwrap_or([0; 8])))
            .unwrap_or(0)
    }

    /// Adds to counter directly, without buffer.
    ///
    /// # Panics
    ///
    /// If write fails.
    fn inc(&self, key: &str, delta: u64) {
        let _guard = self.inc_lock.lock();
        let current = self.get_u64(key);
        self.ks
            .insert(key.as_bytes(), current.saturating_add(delta).to_le_bytes())
            .expect("Analytics write failed");
    }

    /// Salt of `today`, new one when day changed.
    fn get_daily_salt(&self, today: Date) -> [u8; 32] {
        {
            let guard = self.salt.read();
            if guard.0 == today {
                return guard.1;
            }
        }

        let mut guard = self.salt.write();

        if guard.0 == today {
            return guard.1;
        }

        let new_salt = generate_analytics_salt(today);

        *guard = (today, new_salt);

        new_salt
    }

    /// Hash of salt, IP, user agent, date and entity. 16 hex characters.
    fn fingerprint(&self, ip: &str, user_agent: &str, entity_id: &str, date: Date) -> String {
        let mut hasher = XxHash3_64::default();
        self.get_daily_salt(date).hash(&mut hasher);
        ip.hash(&mut hasher);
        user_agent.hash(&mut hasher);
        date.to_string().hash(&mut hasher);
        entity_id.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

/// Salt of `date` from [crate::DAILY_SALT_FILE], new random one if the file is from a different day.
///
/// # Panics
///
/// If the file can't be written.
fn generate_analytics_salt(date: Date) -> [u8; 32] {
    let data = read_file(DAILY_SALT_FILE);

    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() == 2 && parts[0] == date.to_string() {
        let mut salt = [0u8; 32];
        let mut valid = true;

        for (i, chunk) in parts[1].as_bytes().chunks(2).enumerate() {
            if i >= 32 {
                break;
            }

            if let Ok(str_chunk) = from_utf8(chunk) {
                if let Ok(byte) = u8::from_str_radix(str_chunk, 16) {
                    salt[i] = byte;
                } else {
                    valid = false;
                }
            } else {
                valid = false;
            }
        }

        if valid {
            return salt;
        }
    }

    // Generate new salt
    let mut new_salt = [0u8; 32];
    rand::rng().try_fill_bytes(&mut new_salt);

    let salt_hex: String = new_salt.iter().map(|b| format!("{:02x}", b)).collect();
    let content = format!("{}:{}", date, salt_hex);

    write(DAILY_SALT_FILE, content).expect("Failed to write daily salt file");

    new_salt
}
