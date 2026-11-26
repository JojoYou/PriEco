//! # Constants
//!
//! Paths of benchmark data, relative to the repo root.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-11-24
//! * Last Modified: 2026-11-24
//!
//! ## Planned Improvements
//!
//! - [ ] None

// MS MARCO
/// Downloads, queries, qrels and run files.
pub const MSMARCO_DIR: &str = "runtime/benchmarks/msmarco";
/// Own [prieco_core::PriecoIndex] of MS MARCO.
pub const MSMARCO_INDEX_DIR: &str = "runtime/benchmarks/msmarco/index";
/// Inserter '.zip' files made from the corpus.
pub const MSMARCO_IMPORT_DIR: &str = "runtime/benchmarks/msmarco/import";
/// Url id → MS MARCO docid.
pub const MSMARCO_DOCIDS_FILE: &str = "runtime/benchmarks/msmarco/docids.tsv";
/// Exists when indexing finished.
pub const MSMARCO_INDEXED_FILE: &str = "runtime/benchmarks/msmarco/indexed.txt";
