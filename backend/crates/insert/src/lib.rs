//! # PriEco Inserter
//!
//! Inserts crawled pages to the index and keeps the index up to date.
//!
//! * [db_insert] - Inserts '.zip' files to meta storage, Tantivy and vector index.
//! * [update] - Applies rules to already indexed pages.
//! * [updates] - The rules.

pub mod db_insert;
pub use db_insert::run;
pub mod update;
/// Rules for [update::UpdateEngine].
pub mod updates {
    pub mod ping;
}
