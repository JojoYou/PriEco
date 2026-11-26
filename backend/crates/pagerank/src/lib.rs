//! # PriEco PageRank
//!
//! Computes PageRank from links found by the crawler. Reading scores is in [prieco_core::PageRank].
//!
//! * [compute] - Runs everything.
//! * [import] - Adds new links to the graph.
//! * [nodes] - Builds CSR.
//! * [iter] - Power iteration.

pub mod compute;
pub mod import;
pub mod iter;
pub mod nodes;

pub use compute::run;
