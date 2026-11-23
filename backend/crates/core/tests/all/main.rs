//! All prieco_core integration tests in ONE binary (linking every tests/*.rs
//! separately took most of the test build time).
//! set_up.rs stays its own binary: it sets process wide env/cwd for the wizard.

mod common;

mod analytics;
mod country_to_lang;
mod helpers;
mod latency;
mod pagerank_mmap;
mod text_data;
mod types_and_constants;
