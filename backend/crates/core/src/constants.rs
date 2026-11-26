//! # Constants
//!
//! Paths and sizes shared by all crates. Paths are relative to the repo root.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-21
//! * Last Modified: 2026-11-24
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
Paths
*/
// Frontend
/// CSS, JS, images and fonts.
pub const STATIC_DIR: &str = "frontend/static";
/// Handlebars templates.
pub const TEMPLATES_DIR: &str = "frontend/templates";
/// Blog post templates.
pub const BLOG_POSTS_DIR: &str = "frontend/templates/blog/post";
/// Built MkDocs and API docs, served on '/docs'.
pub const DOCS_DIR: &str = "frontend/site";

// Data
/// BGE reranker ONNX model.
pub const BGE_MODEL: &str = "backend/data/bge/model.onnx";
/// Multilingual Mini2026-11-24 ONNX model for query and page vectors.
pub const EMBED_MODEL: &str = "backend/data/paraphrase-multilingual-MiniLM-L12-v2_O3.onnx";

// Runtime
/// Node settings, see [crate::set_up].
pub const CONFIG_FILE: &str = "runtime/config/settings.conf";
/// Iroh secret key of this node.
pub const IROH_SECRET_FILE: &str = "runtime/config/iroh/iroh_secret.bin";
/// Salt of anonymous analytics, changes every day.
pub const DAILY_SALT_FILE: &str = "runtime/config/analytics_daily_salt.txt";
/// Progress of index updates.
pub const INDEX_UPDATES_FILE: &str = "runtime/config/index_updates.json";
/// zstd dictionary of meta storage. Can be changed with `PRIECO_ZSTD_DICT`.
pub const ZSTD_DICT: &str = "runtime/prieco_zstd.dict";
/// When this file exists, background work stops.
pub const STOP_FILE: &str = "runtime/stop.txt";
/// When this file exists, inserter doesn't merge after insertion.
pub const SKIP_MERGE_FILE: &str = "runtime/dont_merge.txt";
/// When this file exists, ranking weights are reloaded from [WEIGHTS_JSON].
pub const WEIGHTS_READY: &str = "runtime/weights.ready";
/// Hand ranking weights.
pub const WEIGHTS_JSON: &str = "runtime/weights.json";
/// Service worker cache version.
pub const CACHE_VERSION_FILE: &str = "runtime/cache_version.txt";

// Storage
/// Tantivy index.
pub const DEFAULT_TANTIVY: &str = "runtime/idx/tantivy";
/// IVF vector buckets.
pub const DEFAULT_VECTORS: &str = "runtime/idx/vectors";

/// Meta storage.
pub const DEFAULT_META: &str = "runtime/idx/meta";
/// Blob storage.
pub const DEFAULT_BLOB: &str = "runtime/idx/blob";

// Pagerank
/// PageRank url id → node id map.
pub const ID_MAP_FILE: &str = "runtime/pagerank/id_map.bin";
/// Computed PageRank scores.
pub const FINAL_SCORES: &str = "runtime/pagerank/pageranks.bin";

/// Cached image search results.
pub const IMG_CACHE_DIR: &str = "runtime/cache/img/bing";

// Insterter
/// Bytes of page id in vector records.
pub const ID_SIZE: usize = 8;
/// Dimensions of page and query vectors.
pub const VECTOR_DIM: usize = 384;
/// Bytes of one vector record: id + f32 vector.
pub const RECORD_SIZE: usize = ID_SIZE + (VECTOR_DIM * 4);

// Blob storage
/// First hash seed of blob ids.
pub const SEARCHED_K1: u64 = 0xC70F6907A1C9566B;
/// Second hash seed of blob ids.
pub const SEARCHED_K2: u64 = 0x85E4C66FC71D33EF;
