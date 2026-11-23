//! All prieco_pagerank integration tests in ONE binary (linking every
//! tests/*.rs separately took most of the test build time).
//! compute_run.rs stays its own binary: it changes the process cwd and env.

mod pagerank;
mod pagerank_units;
