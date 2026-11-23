/*
Paths
*/
// Frontend
pub const STATIC_DIR: &str = "frontend/static";
pub const TEMPLATES_DIR: &str = "frontend/templates";
pub const BLOG_POSTS_DIR: &str = "frontend/templates/blog/post";
pub const DOCS_DIR: &str = "frontend/site";

// Data
pub const BGE_MODEL: &str = "backend/data/bge/model.onnx";
pub const EMBED_MODEL: &str = "backend/data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx";

// Runtime
pub const CONFIG_FILE: &str = "runtime/config/settings.conf";
pub const IROH_SECRET_FILE: &str = "runtime/config/iroh/iroh_secret.bin";
pub const DAILY_SALT_FILE: &str = "runtime/config/analytics_daily_salt.txt";
pub const INDEX_UPDATES_FILE: &str = "runtime/config/index_updates.json";
pub const ZSTD_DICT: &str = "runtime/prieco_zstd.dict";
pub const STOP_FILE: &str = "runtime/stop.txt";
pub const SKIP_MERGE_FILE: &str = "runtime/dont_merge.txt";
pub const WEIGHTS_READY: &str = "runtime/weights.ready";
pub const WEIGHTS_JSON: &str = "runtime/weights.json";
pub const CACHE_VERSION_FILE: &str = "runtime/cache_version.txt";

// Storage
pub const DEFAULT_TANTIVY: &str = "runtime/idx/tantivy";
pub const DEFAULT_VECTORS: &str = "runtime/idx/vectors";

pub const DEFAULT_META: &str = "runtime/idx/meta";
pub const DEFAULT_BLOB: &str = "runtime/idx/blob";

// Pagerank
pub const ID_MAP_FILE: &str = "runtime/pagerank/id_map.bin";
pub const FINAL_SCORES: &str = "runtime/pagerank/pageranks.bin";

pub const IMG_CACHE_DIR: &str = "runtime/cache/img/bing";

// Benchmarks
pub const MSMARCO_DOCUMENT_DIR: &str = "runtime/benchmarks/msmarco/document";
pub const MSMARCO_PASSAGE_DIR: &str = "runtime/benchmarks/msmarco/passage";

// Insterter
pub const ID_SIZE: usize = 8;
pub const VECTOR_DIM: usize = 384;
pub const RECORD_SIZE: usize = ID_SIZE + (VECTOR_DIM * 4);

// Blob storage
pub const SEARCHED_K1: u64 = 0xC70F6907A1C9566B;
pub const SEARCHED_K2: u64 = 0x85E4C66FC71D33EF;
