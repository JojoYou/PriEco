/*
  File: globals.rs
  Description: Manages global variables

  Author: Roman Lancos <support@jojoyou.org>
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
use std::{fs::read, io::Cursor};
use std::{
    fs::{File, metadata},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    net::{Ipv4Addr, Ipv6Addr},
    path::Path,
    str::FromStr,
    sync::Arc,
};

/*
  Import external libraries
*/
use ahash::{AHashMap, AHashSet};
#[cfg(feature = "cuda")]
use cudarc::{
    cublas::{CudaBlas, Gemm, GemmConfig, StridedBatchedConfig, sys::cublasOperation_t},
    driver::{CudaContext, CudaSlice, CudaStream, DevicePtr, LaunchConfig, PushKernelArg},
    nvrtc::compile_ptx,
};
use dashmap::DashSet;
use ndarray::{Array, Array2, CowArray, IxDyn};
use once_cell::sync::Lazy;
use ort::{
    Environment, ExecutionProvider, GraphOptimizationLevel, InMemorySession, SessionBuilder, Value,
    tensor::OrtOwnedTensor,
};
use parking_lot::{Condvar, Mutex};
use redb::{Database, ReadableDatabase, TableDefinition};
use rocksdb::{DB, DBCompressionType, Options};
use serde::{Deserialize, Serialize};
use tantivy::{
    Index, IndexReader, IndexWriter, ReloadPolicy,
    directory::MmapDirectory,
    schema::{FAST, INDEXED, STORED, STRING, Schema, TEXT},
};
use tokenizers::Tokenizer;
use tokio::task;

/*
  Import own libraries
*/
use crate::{
    pagerank::compute::{FINAL_SCORES, ID_MAP_FILE, TMP_SCORES, zstd_reader},
    set_up, url_to_id,
};

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
    pub const PAGERANK: &str = "📋";
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
    pub html: String,
    pub lang: String,
    pub loc: String,
    pub impressions: u32,
    pub clicks: u32,
    pub confidence: f32,
    pub effort: f32,
    pub qna: f32,
    pub sts: f32,
    pub load: f64,
    pub date: i64,

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

/*
PriEco config
*/
pub static PRIECO_CONFIG: Lazy<PriEcoConfig> = Lazy::new(|| set_up::set_up_wizard());
#[derive(Serialize, Deserialize)]
pub struct PriEcoConfig {
    pub ip: String,
    pub port: i32,
    pub tantivy_path: String,
    pub rocksdb_path: String,
    pub vector_path: String,
}

/*
 Vector embeder
*/
pub static VECTOR_EMBEDDING_TOKENIZER: &[u8] = include_bytes!("../data/tokenizer.json");
pub static VECTOR_EMBEDDING_MODEL: &[u8] = include_bytes!("../data/model_int8.onnx");
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

        // Move the actual embedding work to a blocking task
        let embeddings = task::spawn_blocking(
            move || -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
                // Tokenize the query (single item, not batch)
                let tokenizer_guard = tokio::runtime::Handle::current().block_on(tokenizer.lock());

                let encoding = tokenizer_guard
                    .encode(query, true)
                    .map_err(|e| format!("Tokenization failed: {}", e))?;

                // Get token data (similar to your batch logic but for single query)
                let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
                let attention_mask: Vec<i64> = encoding
                    .get_attention_mask()
                    .iter()
                    .map(|&v| v as i64)
                    .collect();

                // Create arrays with shape [1, sequence_length] for single query
                let shape = [1, token_ids.len()];
                let cow_input_ids = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), token_ids)
                        .map_err(|e| format!("Failed to create input_ids array: {}", e))?,
                );
                let cow_attention_masks = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), attention_mask)
                        .map_err(|e| format!("Failed to create attention_mask array: {}", e))?,
                );

                // Get model lock and create input tensors (matching your working code)
                let embedder = tokio::runtime::Handle::current().block_on(model.lock());

                let input_tensor_ids = ort::Value::from_array(embedder.allocator(), &cow_input_ids)
                    .map_err(|e| format!("Failed to create input tensor: {}", e))?;
                let input_tensor_attention_mask =
                    ort::Value::from_array(embedder.allocator(), &cow_attention_masks)
                        .map_err(|e| format!("Failed to create attention mask tensor: {}", e))?;

                // Run inference (exactly like your batch code)
                let outputs: Vec<ort::Value<'static>> = embedder
                    .run(vec![input_tensor_ids, input_tensor_attention_mask])
                    .map_err(|e| format!("Model inference failed: {}", e))?;

                // Extract embeddings (adapted from your batch logic)
                let tensor: OrtOwnedTensor<f32, _> = outputs[0]
                    .try_extract()
                    .map_err(|e| format!("Failed to extract output tensor: {}", e))?;

                // Since we have only one query, take the first (and only) row
                // and limit to 384 dimensions like your batch code
                let binding = tensor.view();
                let embed_row = binding
                    .outer_iter()
                    .next()
                    .ok_or("No embedding row found")?;
                let full_embed: Vec<f32> = embed_row.iter().copied().collect();
                let embedding = full_embed[..384.min(full_embed.len())].to_vec();

                Ok(embedding)
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

    // === Direct mappings ===
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
  Blob storage
  Description: RocksDB for html blobs storage. Designed for high capacity HDD, usage of SSD is beneficial too
*/
pub const BLOB_IMPORT_DIR: &str = "blob/import";
pub static BLOB_STORAGE: Lazy<Arc<DB>> = Lazy::new(|| {
    Arc::new({
        let mut options = Options::default();
        options.create_if_missing(true);

        // Disable compression. Crawler is responsible for this
        options.set_compression_type(DBCompressionType::None);

        // HDD tuning - reduce compaction frequency
        options.set_level_zero_file_num_compaction_trigger(10);
        options.set_max_bytes_for_level_base(256 * 1024 * 1024);

        // Single background job - avoid random seeks from parallel compaction
        options.set_max_background_jobs(1);

        // Large readahead for sequential HDD reads during compaction
        options.set_compaction_readahead_size(2 * 1024 * 1024);

        // Large write buffers - reduce flush frequency
        options.set_write_buffer_size(256 * 1024 * 1024);
        options.set_max_write_buffer_number(4);
        options.set_min_write_buffer_number_to_merge(2);

        // Larger SST files for HDD sequential writes
        options.set_target_file_size_base(128 * 1024 * 1024);

        DB::open(&options, Path::new("blob/blobs")).unwrap()
    })
});

/*
  Pagerank
*/

/*
  Index
*/
pub static ROCKSDB_INDEX: Lazy<Arc<DB>> = Lazy::new(|| {
    Arc::new({
        let mut rocksdb_opts = Options::default();
        rocksdb_opts.create_if_missing(true);
        DB::open(&rocksdb_opts, PRIECO_CONFIG.rocksdb_path.clone()).expect("Faile to open RocksDB")
    })
});

pub static TANTIVY_INDEX: Lazy<Arc<Index>> = Lazy::new(|| {
    // Build schema
    let mut builder = Schema::builder();
    builder.add_u64_field("doc_id", STORED | INDEXED);
    builder.add_text_field("url", STRING);
    builder.add_text_field("title", TEXT);
    builder.add_text_field("description", TEXT);
    builder.add_text_field("content", TEXT);
    builder.add_text_field("keywords", TEXT);
    builder.add_bool_field("safe_s", INDEXED | FAST);
    let schema = builder.build();

    // Open index
    let dir = MmapDirectory::open(PRIECO_CONFIG.tantivy_path.clone())
        .expect("Failed to open Tantivy directory");
    let index = Index::open_or_create(dir, schema.clone()).expect("Failed to open Tantivy index");

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
            .writer_with_num_threads(num_cpus::get(), TANTIVY_HEAP_SIZE)
            .expect("Failed to create Tantivy writer"),
    ))
});

/*
  Inserter
*/
pub const INSERTER_IMPORT_DIR: &str = "results_import";
pub const TANTIVY_HEAP_SIZE: usize = 1_240_000_000;
pub const VECTOR_DIM: usize = 384;

pub static CENTROPOIDS_BIN: &[u8] = include_bytes!("../data/ivf/centroids.bin");

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
    ) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        use std::time::Instant;

        let total_start = Instant::now();
        let batch_size = vectors.len();
        let dims = self.dims;

        // ---------------- FLATTEN (CPU) ----------------
        let t = Instant::now();
        let flat: Vec<f32> = vectors.iter().flatten().copied().collect();
        println!("flatten: {:?}", t.elapsed());

        // ---------------- COPY H→D ----------------
        let t = Instant::now();
        let mut queries_gpu = unsafe { self.stream.alloc::<f32>(flat.len())? };
        self.stream.memcpy_htod(&flat, &mut queries_gpu)?;
        println!("copy h2d: {:?}", t.elapsed());

        // ---------------- NORMALIZE ON GPU ----------------
        let t = Instant::now();
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
        println!("normalize gpu: {:?}", t.elapsed());

        // ---------------- ALLOC SIM BUFFER ----------------
        let mut sims_gpu = self
            .stream
            .alloc_zeros::<f32>(batch_size * self.num_centroids)?;

        // ---------------- GEMM ----------------
        let t = Instant::now();
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
        println!("gemm: {:?}", t.elapsed());

        // ---------------- ARGMAX ON GPU ----------------
        let t = Instant::now();
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
        println!("gpu argmax kernel: {:?}", t.elapsed());

        // ---------------- COPY ARGMAX D→H ----------------
        let t = Instant::now();
        let mut argmax_cpu = vec![0i32; batch_size];
        self.stream.memcpy_dtoh(&argmax_gpu, &mut argmax_cpu)?;
        println!("copy argmax d2h: {:?}", t.elapsed());

        let result: Vec<usize> = argmax_cpu.into_iter().map(|x| x as usize).collect();

        println!("TOTAL assign_batch: {:?}", total_start.elapsed());
        println!("--------------------------------");

        Ok(result)
    }

    pub fn search(
        &self,
        query: &[f32], // 384D, need not be normalized
        n: usize,      // how many results to return (0 = all from probs: t)
        t: usize,      // how many centroid buckets to probe
    ) -> Result<Vec<(u64, f32)>, Box<dyn std::error::Error>> {
        assert!(t <= 32, "t must be <=32");
        let dims = self.dims;

        // ── 1. Upload + normalize query on GPU ──────────────────────────────────
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

        // ── 2. GEMM: sims = centroids^T · query  →  (num_centroids × 1) ────────
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

        // ── 3. Top-T argmax on GPU ───────────────────────────────────────────────
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

        // ── 4. Normalize query on CPU for the brute-force cosine scan ───────────
        let norm = query.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
        let query_norm: Vec<f32> = query.iter().map(|x| x / norm).collect();

        // ── 5. Scan buckets, compute cosine sim, collect candidates ─────────────
        let mut seen = std::collections::HashSet::<u64>::new();
        let mut candidates: Vec<(u64, f32)> = Vec::new();

        for cid in centroid_ids {
            let bucket_dir = format!("{}/bucket_{:06}", &PRIECO_CONFIG.vector_path, cid);
            let ids_path = format!("{}/ids.bin", bucket_dir);
            let vecs_path = format!("{}/vectors.bin", bucket_dir);

            if !Path::new(&ids_path).exists() || cid == 080159 {
                continue;
            }

            let ids_bytes = read(&ids_path)?;
            let count = ids_bytes.len() / 8;

            let mut vecs_file = File::open(&vecs_path)?;
            let mut vec_buf = vec![0u8; count * dims * 4];
            vecs_file.read_exact(&mut vec_buf)?;

            for i in 0..count {
                let id = u64::from_le_bytes(ids_bytes[i * 8..(i + 1) * 8].try_into().unwrap());
                if !seen.insert(id) {
                    continue;
                } // dedup across probed buckets

                let base = i * dims * 4;
                let mut dot = 0f32;
                let mut vnorm = 0f32;
                for j in 0..dims {
                    let off = base + j * 4;
                    let v = f32::from_le_bytes(vec_buf[off..off + 4].try_into().unwrap());
                    dot += v * query_norm[j];
                    vnorm += v * v;
                }
                let sim = dot / vnorm.sqrt().max(1e-10);
                candidates.push((id, sim));
            }
        }

        // ── 6. Sort, truncate ────────────────────────────────────────────────────
        candidates.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        if n > 0 {
            candidates.truncate(n);
        }

        Ok(candidates) // (id, cosine_similarity) sorted best → worst
    }
}

#[cfg(not(feature = "cuda"))]
pub struct CentroidIndex;

#[cfg(not(feature = "cuda"))]
impl CentroidIndex {
    pub fn new(_: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Err("CUDA feature not enabled".into())
    }

    pub fn assign_batch(&self, _: &[Vec<f32>]) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
        Err("CUDA feature not enabled".into())
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
pub static PAGERANK: Lazy<Arc<Database>> =
    Lazy::new(|| Arc::new(Database::create(Path::new("kv/pageranks.redb")).unwrap()));
pub const PAGERANKS_TABLE: TableDefinition<&str, f64> = TableDefinition::new("pageranks");

pub fn pagerank_warmup_lookup_cache() {
    let tmp_map = format!("{}.tmp_lookup", ID_MAP_FILE);
    let tmp_scores = TMP_SCORES;

    if Path::new(&tmp_map).exists() && Path::new(tmp_scores).exists() {
        return;
    }

    println!("Building lookup cache...");

    if !Path::new(&tmp_map).exists() {
        let mut dec = match zstd_reader(ID_MAP_FILE) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to open id_map for warmup: {e}");
                return;
            }
        };
        let mut out = BufWriter::new(match File::create(&tmp_map) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create tmp lookup file: {e}");
                return;
            }
        });
        let mut buf = [0u8; 16];
        while dec.read_exact(&mut buf).is_ok() {
            out.write_all(&buf).unwrap();
        }
    }

    if !Path::new(tmp_scores).exists() {
        let mut dec = match zstd_reader(FINAL_SCORES) {
            Ok(d) => d,
            Err(e) => {
                println!("Failed to open scores for warmup: {e}");
                return;
            }
        };
        let mut out = BufWriter::with_capacity(
            1 << 20,
            match File::create(tmp_scores) {
                Ok(f) => f,
                Err(e) => {
                    println!("Failed to create tmp scores file: {e}");
                    return;
                }
            },
        );
        let mut buf = [0u8; 4];
        while dec.read_exact(&mut buf).is_ok() {
            out.write_all(&buf).unwrap();
        }
    }

    println!("Lookup cache ready.");
}
pub fn lookup_in(url: &str) -> f32 {
    let target = url_to_id(url);
    let tmp_map = format!("{}.tmp_lookup", ID_MAP_FILE);

    let num_entries = match metadata(&tmp_map).ok() {
        Some(m) => m.len() / 16,
        None => return 0.0,
    };
    let mut f = match File::open(&tmp_map) {
        Ok(f) => f,
        Err(_) => return 0.0,
    };
    let mut buf = [0u8; 16];
    let mut lo = 0u64;
    let mut hi = num_entries;
    let id = loop {
        if lo >= hi {
            return 0.0;
        }
        let mid = (lo + hi) / 2;
        if f.seek(SeekFrom::Start(mid * 16)).is_err() {
            return 0.0;
        }
        if f.read_exact(&mut buf).is_err() {
            return 0.0;
        }
        let hash = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let id = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        match hash.cmp(&target) {
            std::cmp::Ordering::Equal => break id,
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
        }
    };

    let mut scores_f = match File::open(TMP_SCORES) {
        Ok(f) => f,
        Err(_) => return 0.0,
    };
    let mut buf4 = [0u8; 4];
    if scores_f.seek(SeekFrom::Start(id * 4)).is_err() {
        return 0.0;
    }
    if scores_f.read_exact(&mut buf4).is_err() {
        return 0.0;
    }
    f32::from_le_bytes(buf4)
}

/*
  Reranker
*/
pub static BGE_MODEL: &[u8] = include_bytes!("../data/bge/model.onnx");
pub static BGE_TOKENIZER: &[u8] = include_bytes!("../data/bge/tokenizer.json");
pub static RERANKER: Lazy<Reranker> = Lazy::new(Reranker::new);

pub struct Reranker {
    session: InMemorySession<'static>,
    tokenizer: Tokenizer,
}

impl Reranker {
    pub fn new() -> Self {
        let environment = Environment::builder()
            .with_name("reranker")
            .build()
            .unwrap()
            .into_arc();

        let session = SessionBuilder::new(&environment)
            .unwrap()
            .with_execution_providers([ExecutionProvider::CUDA(Default::default())])
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .unwrap()
            .with_model_from_memory(BGE_MODEL)
            .unwrap();

        let tokenizer = Tokenizer::from_bytes(BGE_TOKENIZER).unwrap();
        Self { session, tokenizer }
    }

    pub fn score(&self, query: &str, passage: &str) -> f32 {
        let encoding = self.tokenizer.encode((query, passage), true).unwrap();
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();
        let seq_len = ids.len();
        let input_ids =
            CowArray::from(Array2::from_shape_vec((1, seq_len), ids).unwrap()).into_dyn();
        let attention_mask =
            CowArray::from(Array2::from_shape_vec((1, seq_len), mask).unwrap()).into_dyn();
        let inputs = vec![
            Value::from_array(self.session.allocator(), &input_ids).unwrap(),
            Value::from_array(self.session.allocator(), &attention_mask).unwrap(),
        ];
        let outputs = self.session.run(inputs).unwrap();
        let scores = outputs[0].try_extract::<f32>().unwrap();
        scores.view()[[0, 0]]
    }

    /// Returns sigmoid(raw_score) as a 0-1 relevance probability
    pub fn score_normalized(&self, query: &str, passage: &str) -> f32 {
        let raw = self.score(query, passage);
        1.0 / (1.0 + (-raw).exp())
    }
}

pub static ARTISTS_DB: Lazy<Arc<Database>> =
    Lazy::new(|| Arc::new(Database::open("kv/artists.redb").unwrap()));
pub const ARTISTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("artists");

pub static TOP_DOMAINS: Lazy<AHashSet<&'static str>> =
    Lazy::new(|| include_str!("../data/domains.txt").lines().collect());
