/*
  File: globals.rs
  Description: Manages global variables

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-02-06

  Usage:
  TODO:
*/

/*
  Import system libraries
*/
#[cfg(feature = "cuda")]
use std::io::{Cursor, Read};
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, read},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, Ipv6Addr},
    ops::Range,
    path::Path,
    process::exit,
    str::FromStr,
    sync::Arc,
    time::Duration as stdDuration,
};

/*
  Import external libraries
*/
use ahash::{AHashMap, AHashSet};
use aho_corasick::{AhoCorasick, MatchKind};
use charabia::{Tokenizer as CHARABIA_TOKENIZER, TokenizerBuilder};
use chrono::{Duration, NaiveDate, Utc};
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
    Environment, ExecutionProvider, GraphOptimizationLevel, InMemorySession, SessionBuilder, Value,
    tensor::OrtOwnedTensor,
};
use parking_lot::{Condvar, Mutex, RwLock};
#[cfg(feature = "cuda")]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use redb::{Database, ReadableDatabase, TableDefinition};
use reqwest::Client;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use rocket::{
    Request,
    request::{FromRequest, Outcome},
};
use rocksdb::{DB, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use symspell::{SymSpell, UnicodeStringStrategy};
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    directory::MmapDirectory,
    schema::{
        FAST, INDEXED, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
    },
    tokenizer::{TextAnalyzer, Token, TokenStream, Tokenizer as TANTIVY_TOKENIZER},
};
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
use crate::constants::{FINAL_SCORES, ID_MAP_FILE};
use crate::helpers::{normalize_url, url_to_id};
use crate::set_up;

/*
  Constants
*/
pub const CSS_VERSION: &str = "0.2.0";
pub const JS_VERSION: &str = "0.2.0";

// Terminal print colors
pub mod colors {
    pub const RESET: &str = "\x1b[0m"; // Must be used after every print message
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
}

pub mod icons {
    pub const BLOB: &str = "🪼";
    pub const DB_INSERT: &str = "💾";
    pub const PAGERANK_ICON: &str = "📋";
    pub const MINI_CRAWLER_ICON: &str = "👾";
}

/*
  PriEco structures
*/
#[derive(Serialize)]
pub struct SearchResult {
    pub url: String,
    pub display_url: String,
    pub domain: String,

    pub title: String,
    pub description: String,

    pub image: String,
    pub favicon: String,

    pub html_id: Option<String>,
    pub url_enc: String,
}
#[derive(Serialize, Clone)]
pub struct ImgResult {
    pub thumbnail: String,
    pub image: String,

    pub title: String,

    pub site_url: String,
    pub site_domain: String,
    pub favicon: String,
}
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebDocument {
    pub url: String,

    pub title: String,
    pub description: String,
    pub content: String,
    pub favicon: String,
    pub image: String,
    pub keywords: String,

    pub safe_s: bool,
    pub lang: String,
    pub loc: String,

    pub html: String, // Html blob id

    pub impressions: u32,
    pub clicks: u32,
    pub confidence: f32, // Flesch Reading Ease and Text Image Ratio
    pub effort: f32,     // Text effort
    pub qna: f32,        // Quality score
    pub sts: f32,

    pub load: f64, // Loading time
    pub date: i64, // Date of crawling

    #[serde(default)]
    pub search_score: f32,
}

pub struct FileLocks {
    pub set: Mutex<DashSet<String>>,
    pub condvar: Condvar,
}
pub static FILE_LOCKS: Lazy<Arc<FileLocks>> = Lazy::new(|| {
    Arc::new(FileLocks {
        set: Mutex::new(DashSet::new()),
        condvar: Condvar::new(),
    })
});

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
pub static PRIECO_CONFIG: Lazy<PriEcoConfig> = Lazy::new(|| set_up::set_up_wizard());
#[derive(Serialize, Deserialize)]
pub struct PriEcoConfig {
    pub ip: String,
    pub port: i32,
    pub tantivy_path: String,
    pub meta_path: String,
    pub vector_path: String,
    pub worker_id: String,
    pub worker_concurrent: u32,
}

/*
 Request Client
*/
pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    println!("Created client!");
    Client::builder()
        .use_rustls_tls()
        .connect_timeout(stdDuration::from_secs(3))
        .timeout(stdDuration::from_secs(15))
        .pool_max_idle_per_host(50)
        .pool_idle_timeout(stdDuration::from_secs(90))
        .tcp_keepalive(stdDuration::from_secs(60))
        .tcp_keepalive_interval(stdDuration::from_secs(30))
        .http2_keep_alive_timeout(stdDuration::from_secs(20))
        .build()
        .expect("Failed to create client")
});

/*
 Vector embeder
*/
pub static VECTOR_EMBEDDING_TOKENIZER: &[u8] = include_bytes!("../../../data/tokenizer.json");
pub static VECTOR_EMBEDDING_MODEL: &[u8] =
    include_bytes!("../../../data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx");
#[derive(Clone)]
pub struct EmbeddingService {
    pub tokenizer: Arc<tokio::sync::Mutex<Tokenizer>>,
    pub model: Arc<tokio::sync::Mutex<InMemorySession<'static>>>,
}
impl EmbeddingService {
    pub async fn embed_query(
        &self,
        query: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let query = query.to_string();
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();

        let embeddings = task::spawn_blocking(
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
                    for k in 0..hidden_size {
                        sum_vec[k] /= token_count;
                    }
                }

                Ok(sum_vec)
            },
        )
        .await?;

        embeddings
    }
}

/*
 IP to location
*/
pub static IP_TO_LOC: Lazy<Arc<IpGeoDatabase>> =
    Lazy::new(|| Arc::new(IpGeoDatabase::open("kv/ip.redb").unwrap()));
const IP_RANGES: TableDefinition<u128, (u128, String)> = TableDefinition::new("ip_ranges");

pub struct IpGeoDatabase {
    db: Database,
}
impl IpGeoDatabase {
    pub fn open(db_path: &str) -> Result<Self, redb::Error> {
        let db = Database::create(db_path)?;
        Ok(Self { db })
    }

    pub fn lookup_country(
        &self,
        ip_str: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let ip_num = self.parse_ip_to_u128(ip_str)?;

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(IP_RANGES)?;

        // Find the range that contains this IP
        // We need to find the largest start IP that is <= our target IP
        let mut range_iter = table.range(..=ip_num)?;

        // Get the last (highest) start IP that's <= our target
        if let Some(result) = range_iter.next_back() {
            let (start_ip, value) = result?;
            let (end_ip, country_code) = value.value();

            // Check if our IP falls within this range
            if ip_num >= start_ip.value() && ip_num <= end_ip {
                return Ok(Some(country_code.clone()));
            }
        }

        Ok(None)
    }

    fn parse_ip_to_u128(&self, ip_str: &str) -> Result<u128, Box<dyn std::error::Error>> {
        // Try IPv4 first
        if let Ok(ipv4) = Ipv4Addr::from_str(ip_str) {
            let octets = ipv4.octets();
            let ip_u32 = ((octets[0] as u32) << 24)
                | ((octets[1] as u32) << 16)
                | ((octets[2] as u32) << 8)
                | (octets[3] as u32);
            return Ok(ip_u32 as u128);
        }

        // Try IPv6
        if let Ok(ipv6) = Ipv6Addr::from_str(ip_str) {
            let segments = ipv6.segments();
            let mut ip_u128 = 0u128;
            for (i, segment) in segments.iter().enumerate() {
                ip_u128 |= (*segment as u128) << (112 - (i * 16));
            }
            return Ok(ip_u128);
        }

        Err(format!("Invalid IP address: {}", ip_str).into())
    }
}

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
    map.insert("sr", "sr");
    map.insert("sk", "sk");
    map.insert("sl", "sl");
    map.insert("fi", "fi");
    map.insert("sv", "sv");
    map.insert("tr", "tr");
    map.insert("el", "el");
    map.insert("bg", "bg");
    map.insert("cs", "cs");
    map.insert("da", "da");
    map.insert("he", "he");
    map.insert("ja", "ja");
    map.insert("ko", "ko");
    map.insert("ru", "ru");
    map.insert("ar", "ar");
    map.insert("se", "nl");

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

/*
  PriEco Storage
  Description: FJALL key-value storage. Stores results meta data and html blobs
*/
pub static META_DICTIONARY: Lazy<Vec<u8>> =
    Lazy::new(|| read("idx/prieco_zstd.dict").expect("Failed to load zstd dictionary into memory"));
pub static META_DECODER: Lazy<DecoderDictionary<'static>> =
    Lazy::new(|| DecoderDictionary::copy(&*META_DICTIONARY));

pub struct PriecoStorage {
    // Meta data: Titles, Descriptions, URLs...
    pub meta_db: FJALL_DATABASE,
    pub meta_ks: Keyspace,

    // Goggles
    pub goggles_ks: Keyspace,

    // Blob storage: Compressed web page data
    pub blob_db: FJALL_DATABASE,
    pub blobs_ks: Keyspace,
}

pub static PRIECO_FJALL: Lazy<Arc<PriecoStorage>> = Lazy::new(|| {
    // Meta storage
    let meta_db = FJALL_DATABASE::builder(Path::new(&PRIECO_CONFIG.meta_path))
        .worker_threads(4)
        .cache_size(2 * 1024 * 1024 * 1024)
        .open()
        .expect("Failed to open Meta Fjall DB");

    let meta_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::disabled())
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));

    let meta_ks = meta_db.keyspace("meta", || meta_opts).unwrap();

    // Goggles storage
    let goggles_opts = KeyspaceCreateOptions::default()
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4));
    let goggles_ks = meta_db.keyspace("goggles", || goggles_opts).unwrap();

    // Blob storage
    let blob_db = FJALL_DATABASE::builder(Path::new("/mnt/ssd/blobs"))
        .worker_threads(2)
        .cache_size(1 * 1024 * 1024 * 1024)
        .open()
        .expect("Failed to open Blob Fjall DB");

    let blob_opts = KeyspaceCreateOptions::default()
        .max_memtable_size(2048 * 1024 * 1024)
        .data_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .index_block_compression_policy(CompressionPolicy::all(CompressionType::Lz4))
        .with_kv_separation(Some(KvSeparationOptions {
            compression: CompressionType::None,
            file_target_size: 2048 * 1024 * 1024, // 2GB blobs
            separation_threshold: 100,
            staleness_threshold: 0.5,
            age_cutoff: 0.0,
        }));

    let blobs_ks = blob_db.keyspace("blobs", || blob_opts).unwrap();

    Arc::new(PriecoStorage {
        meta_db,
        meta_ks,
        goggles_ks,
        blob_db,
        blobs_ks,
    })
});

/*
  Index
*/
pub const TANTIVY_HEAP_SIZE: usize = 1_240_000_000;
pub static TANTIVY_INDEX: Lazy<Arc<Index>> = Lazy::new(|| {
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

    let schema = builder.build();

    // Open index
    let dir = MmapDirectory::open(Path::new(&PRIECO_CONFIG.tantivy_path))
        .expect("Failed to open Tantivy V2 directory");
    let index = Index::open_or_create(dir, schema.clone()).expect("Failed to open Tantivy index");

    index
        .tokenizers()
        .register("multilingual", TextAnalyzer::from(Multilingual::new()));

    Arc::new(index)
});

pub static TANTIVY_READER: Lazy<Arc<IndexReader>> = Lazy::new(|| {
    Arc::new(
        TANTIVY_INDEX
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .expect("Failed to create Tantivy reader"),
    )
});

pub static TANTIVY_WRITER: Lazy<Arc<Mutex<IndexWriter>>> = Lazy::new(|| {
    Arc::new(Mutex::new(
        TANTIVY_INDEX
            .writer(TANTIVY_HEAP_SIZE)
            .expect("Failed to create Tantivy V2 writer"),
    ))
});

// Multilang tokenization
#[derive(Clone)]
pub struct Multilingual(Arc<CHARABIA_TOKENIZER<'static>>);

impl Multilingual {
    pub fn new() -> Self {
        let builder: &'static mut TokenizerBuilder<Vec<u8>> =
            Box::leak(Box::new(TokenizerBuilder::default()));
        Self(Arc::new(builder.build()))
    }
}

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

    fn token_stream<'a>(&mut self, text: &'a str) -> MultiStream {
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
pub const INSERTER_IMPORT_DIR: &str = "/mnt/ssd/results/imp";
pub static CENTROPOIDS_BIN: &[u8] = include_bytes!("../../../data/ivf/centroids.bin");

pub static VECTOR_CENTROPOIDS: Lazy<Arc<CentroidIndex>> =
    Lazy::new(|| Arc::new(CentroidIndex::new(CENTROPOIDS_BIN).unwrap()));
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

    pub fn search(
        &self,
        query: &[f32],
        n: usize,
        t: usize,
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
                let zst_path = format!("{}/bucket_{:06}.bin.zst", &PRIECO_CONFIG.vector_path, cid);
                if !Path::new(&zst_path).exists() || cid == 080159 {
                    return None;
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
                    let vnorm: f32 = slice.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-10);
                    results.push((id, dot / vnorm));
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
#[cfg(not(feature = "cuda"))]
pub struct CentroidIndex;

#[cfg(not(feature = "cuda"))]
impl CentroidIndex {
    pub fn new(_: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        println!("CUDA feature not enabled");
        Ok(Self)
    }

    pub fn assign_batch(&self, _: &[Vec<f32>]) -> Result<Vec<usize>, Box<dyn Error + Send + Sync>> {
        println!("CUDA feature not enabled");
        Ok(Vec::new())
    }

    pub fn search(
        &self,
        _: &[f32],
        _: usize,
        _: usize,
    ) -> Result<Vec<(u64, f32)>, Box<dyn std::error::Error>> {
        Err("CUDA feature not enabled".into())
    }
}

/*
  PageRank
*/
pub static PAGERANK: Lazy<RwLock<Arc<PageRank>>> =
    Lazy::new(|| RwLock::new(Arc::new(PageRank::open(ID_MAP_FILE, FINAL_SCORES).unwrap())));

pub struct PageRank {
    _id_mmap: Mmap,
    ptr: *const (u64, u64),
    len: usize,
    _scores_mmap: Mmap,
    scores_ptr: *const f32,
}

unsafe impl Send for PageRank {}
unsafe impl Sync for PageRank {}

impl PageRank {
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

    pub fn pairs(&self) -> &[(u64, u64)] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn lookup(&self, hash: u64) -> Option<u64> {
        let pairs = self.pairs();
        pairs
            .binary_search_by_key(&hash, |&(h, _)| h)
            .ok()
            .map(|i| pairs[i].1)
    }

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
pub static BGE_MODEL: &[u8] = include_bytes!("../../../data/bge/model.onnx");
pub static BGE_TOKENIZER: &[u8] = include_bytes!("../../../data/bge/tokenizer.json");
pub static RERANKER: Lazy<Reranker> = Lazy::new(Reranker::new);

pub struct Reranker {
    session: InMemorySession<'static>,
    tokenizer: Tokenizer,
}

impl Reranker {
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
                    arena_extend_strategy:
                        ort::execution_providers::ArenaExtendStrategy::SameAsRequested,
                    ..Default::default()
                },
            )])
            .expect("Failed to attach CUDA provider")
            .with_model_from_memory(BGE_MODEL)
            .expect("Failed to load BGE model");

        let mut tokenizer = Tokenizer::from_bytes(BGE_TOKENIZER).unwrap();

        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        tokenizer.with_truncation(Some(TruncationParams {
            max_length: 512,
            ..Default::default()
        }));

        Self { session, tokenizer }
    }

    pub fn score_batch(&self, query: &str, passages: &[String]) -> Vec<f32> {
        if passages.is_empty() {
            return Vec::new();
        }

        let batch_size = passages.len();

        let input_pairs: Vec<(&str, &str)> = passages.iter().map(|p| (query, p.as_str())).collect();
        let encodings = self.tokenizer.encode_batch(input_pairs, true).unwrap();
        let seq_len = encodings[0].get_ids().len();

        let mut ids = Vec::with_capacity(batch_size * seq_len);
        let mut mask = Vec::with_capacity(batch_size * seq_len);

        for enc in &encodings {
            ids.extend(enc.get_ids().iter().map(|&x| x as i64));
            mask.extend(enc.get_attention_mask().iter().map(|&x| x as i64));
        }

        let input_ids =
            CowArray::from(Array2::from_shape_vec((batch_size, seq_len), ids).unwrap()).into_dyn();
        let attention_mask =
            CowArray::from(Array2::from_shape_vec((batch_size, seq_len), mask).unwrap()).into_dyn();

        let mut io_binding = self.session.bind().unwrap();

        io_binding
            .bind_input(
                "input_ids",
                Value::from_array(self.session.allocator(), &input_ids).unwrap(),
            )
            .unwrap();

        io_binding
            .bind_input(
                "attention_mask",
                Value::from_array(self.session.allocator(), &attention_mask).unwrap(),
            )
            .unwrap();

        let mem_info = ort::MemoryInfo::new(
            ort::AllocationDevice::CPU,
            0,
            ort::AllocatorType::Arena,
            ort::MemType::Default,
        )
        .unwrap();

        io_binding.bind_output("logits", mem_info).unwrap();

        self.session.run_with_binding(&io_binding).unwrap();

        let outputs = io_binding.outputs().unwrap();
        let scores = outputs["logits"].try_extract::<f32>().unwrap();
        let view = scores.view();

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

        results
    }
}

/*
  Synonyms
*/
pub struct LangSynonyms {
    index: FstMap<&'static [u8]>,
    rows: Vec<Box<[Box<str>]>>,
}

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
    fn load(lang: &str) -> Option<Arc<LangSynonyms>> {
        let (fst_bytes, rows_bytes) = get_synonym_bytes(lang)?;

        let index = FstMap::new(fst_bytes).ok()?;

        let (rows, _bytes_read): (Vec<Box<[Box<str>]>>, usize) =
            bincode_next::decode_from_slice(rows_bytes, bincode_next::config::standard()).ok()?;

        Some(Arc::new(LangSynonyms { index, rows }))
    }

    pub fn lookup(&self, term: &str) -> Option<&[Box<str>]> {
        let id = self.index.get(term)?;
        self.rows.get(id as usize).map(|r| r.as_ref())
    }
}

static SYNONYM_STORES: Lazy<RwLock<std::collections::HashMap<String, Option<Arc<LangSynonyms>>>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));
pub fn get_store(lang: &str) -> Option<Arc<LangSynonyms>> {
    {
        let cache = SYNONYM_STORES.read();
        if let Some(entry) = cache.get(lang) {
            return entry.clone();
        }
    }
    let loaded = LangSynonyms::load(lang);
    SYNONYM_STORES
        .write()
        .insert(lang.to_string(), loaded.clone());
    loaded
}

/*
  Intent
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    Informational,           // Learn information
    Transactional,           // Action purchase, download, sign up
    CommercialInvestigation, // Research, comparing options
    Navigational,            // Reach a known domain, page
    Local,                   // Near me businesses
    Unknown,                 // Failed to determine
}

pub struct LangMatcher {
    pub automaton: AhoCorasick,
    pub intent_map: Vec<QueryIntent>,
}

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

#[derive(Deserialize)]
struct IntentConfig {
    #[serde(flatten)]
    languages: AHashMap<String, LanguageKeywords>,
}

pub static MATCHERS: Lazy<AHashMap<String, LangMatcher>> = Lazy::new(|| {
    let mut matchers = AHashMap::new();

    let json_data = include_str!("../../../data/intent.json");
    let config: IntentConfig = serde_json::from_str(&json_data)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    PersonName,
    Business,
    Place,
}

#[derive(Debug, Clone)]
pub struct TaggedEntity {
    pub range: Range<usize>,
    pub entity_type: EntityType,
    pub matched_text: String,
}

#[derive(Archive, Deserialize, Serialize, RkyvSerialize, RkyvDeserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct GeoCoords {
    pub country: String,
    pub lon: f32,
    pub lat: f32,
}

#[derive(Archive, Deserialize, Serialize, RkyvSerialize, RkyvDeserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct EntityRegistry {
    pub places: HashMap<String, Vec<GeoCoords>>,
}

pub struct QueryUnderstandingPipeline {
    pub automaton: AhoCorasick,
    pub metadata_map: Vec<(EntityType, String)>,
    pub archived_places: &'static rkyv::Archived<EntityRegistry>,
}

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
    let archived_places = rkyv::check_archived_root::<EntityRegistry>(places_bytes).unwrap();

    let keywords_str = include_str!("../../../data/entities/keywords_automaton.txt");

    let mut patterns = Vec::new();
    let mut metadata_map = Vec::new();

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

    for place_name in archived_places.places.keys() {
        let name_str = place_name.as_str().to_lowercase();
        if name_str.len() > 1 {
            patterns.push(name_str.clone());
            metadata_map.push((EntityType::Place, name_str));
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

pub static ARTISTS_DB: Lazy<Arc<Database>> =
    Lazy::new(|| Arc::new(Database::open("kv/artists.redb").unwrap()));
pub const ARTISTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("artists");

pub static TOP_DOMAINS: Lazy<AHashSet<&'static str>> =
    Lazy::new(|| include_str!("../../../data/domains.txt").lines().collect());

/*
  Spell checker
*/
pub static SPELL_CHECKER: Lazy<Arc<SymSpell<UnicodeStringStrategy>>> = Lazy::new(|| {
    let mut symspell: SymSpell<UnicodeStringStrategy> = SymSpell::default();

    let en_dict = include_str!("../../../data/spell_check/frequency_dictionary_en_82_765.txt");
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
        include_str!("../../../data/spell_check/frequency_bigramdictionary_en_243_342.txt");
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
  Analytics
*/
pub static ANALYTICS: Lazy<AnalyticsDb> = Lazy::new(|| AnalyticsDb::open("kv/analytics"));

pub struct AnalyticsDb {
    db: Arc<DB>,
}

impl AnalyticsDb {
    fn open(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).expect("Failed to open analytics RocksDB");
        Self { db: Arc::new(db) }
    }

    // Write
    pub fn record_query(&self) {
        self.inc(&format!("queries:{}", self.today()), 1);
    }

    pub fn record_visitor(
        &self,
        ip: &str,
        user_agent: &str,
        entity_id: &str,
        country_code: Option<&str>,
    ) {
        let date = self.today();

        // Always count the raw pageview
        let pvk = format!("pageviews:{}", date);
        let mut batch = WriteBatch::default();
        batch.put(pvk.as_bytes(), (self.get_u64(&pvk) + 1).to_le_bytes());

        // Unique visitor dedup
        let seen_key = format!(
            "seen:{}:{}",
            date,
            self.fingerprint(ip, user_agent, entity_id, date)
        );
        if self.db.get(seen_key.as_bytes()).ok().flatten().is_none() {
            batch.put(seen_key.as_bytes(), &[1u8]);

            let vk = format!("visitors:{}", date);
            batch.put(vk.as_bytes(), (self.get_u64(&vk) + 1).to_le_bytes());

            if let Some(cc) = country_code {
                // Skip "all" — that's the default no-selection value
                if cc != "all" {
                    let ck = format!("country:{}:{}", date, cc.to_lowercase());
                    batch.put(ck.as_bytes(), (self.get_u64(&ck) + 1).to_le_bytes());
                }
            }
        }

        self.db
            .write(batch)
            .expect("Analytics visitor write failed");
    }

    pub fn record_api_request(&self) {
        let date = self.today();
        let mut batch = WriteBatch::default();
        let total_key = format!("api:total:{}", date);
        batch.put(
            total_key.as_bytes(),
            (self.get_u64(&total_key) + 1).to_le_bytes(),
        );
        self.db
            .write(batch)
            .expect("API request tracking write failed");
    }

    // Read
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

    // (today_visitors, yesterday_visitors, today_pageviews, yesterday_pageviews)
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

    pub fn top_countries(&self) -> Vec<(String, u64)> {
        let prefix = format!("country:{}:", self.today());
        let mut results: Vec<(String, u64)> = self
            .db
            .prefix_iterator(prefix.as_bytes())
            .filter_map(|item| {
                let (k, v) = item.ok()?;
                let key_str = std::str::from_utf8(&k).ok()?;
                if !key_str.starts_with(&prefix) {
                    return None;
                }
                let cc = key_str.rsplit(':').next()?.to_string();
                let count = u64::from_le_bytes((&v[..]).try_into().unwrap_or([0; 8]));
                Some((cc, count))
            })
            .collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

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

    pub fn api_stats_today_yesterday(&self) -> (u64, u64) {
        let today = self.today();
        let yesterday = today - Duration::days(1);
        (
            self.get_u64(&format!("api:total:{}", today)),
            self.get_u64(&format!("api:total:{}", yesterday)),
        )
    }

    // Purge
    pub fn purge_expired(&self) {
        let cutoff = self.today() - Duration::days(30);
        let mut batch = WriteBatch::default();
        for prefix in [
            "queries:",
            "visitors:",
            "pageviews:",
            "country:",
            "seen:",
            "api:",
        ] {
            for item in self.db.prefix_iterator(prefix.as_bytes()) {
                let Ok((k, _)) = item else { continue };
                let Ok(key_str) = std::str::from_utf8(&k) else {
                    continue;
                };
                if !key_str.starts_with(prefix) {
                    break;
                }
                let date_part = key_str[prefix.len()..].splitn(2, ':').next().unwrap_or("");
                if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                    if date < cutoff {
                        batch.delete(&k);
                    }
                }
            }
        }
        self.db.write(batch).expect("Analytics purge failed");
    }

    pub async fn background_purge_task(&self) {
        self.purge_expired();
        loop {
            let now = Utc::now();
            let secs_until_midnight = {
                let tomorrow = (now + Duration::days(1))
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap();
                (tomorrow.and_utc() - now).num_seconds().max(0) as u64
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(secs_until_midnight)).await;
            self.purge_expired();
        }
    }

    // Helpers
    fn today(&self) -> NaiveDate {
        Utc::now().date_naive()
    }

    fn get_u64(&self, key: &str) -> u64 {
        self.db
            .get(key.as_bytes())
            .ok()
            .flatten()
            .map(|v| u64::from_le_bytes((&v[..]).try_into().unwrap_or([0; 8])))
            .unwrap_or(0)
    }

    fn inc(&self, key: &str, delta: u64) {
        let current = self.get_u64(key);
        self.db
            .put(key.as_bytes(), (current + delta).to_le_bytes())
            .expect("Analytics write failed");
    }

    fn fingerprint(&self, ip: &str, user_agent: &str, entity_id: &str, date: NaiveDate) -> String {
        let mut hasher = XxHash3_64::default();
        ip.hash(&mut hasher);
        user_agent.hash(&mut hasher);
        date.to_string().hash(&mut hasher);
        entity_id.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
