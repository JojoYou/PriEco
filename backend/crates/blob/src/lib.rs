//! # PriEco Blob Storage
//!
//! Stores compressed text, tags and links of crawled pages.
//!
//! * [insert] - Imports '.tar.gz' files to blob storage.
//! * [decode] - Reads a blob as text or html.

pub mod insert;
pub use insert::run;
pub mod decode;
