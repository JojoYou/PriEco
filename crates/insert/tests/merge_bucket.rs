//! Tests for vector bucket merging in crates/insert/src/db_insert.rs
//!
//! Runs inside a temp sandbox: PRIECO_CONFIG.vector_path points to a throw away dir.
//! Each test uses its own bucket ids.

mod common;

use std::{
    collections::HashMap,
    fs,
    io::Read,
    panic::{AssertUnwindSafe, catch_unwind},
};

use common::sandbox;
use prieco_core::{ID_SIZE, PRIECO_CONFIG, RECORD_SIZE, VECTOR_DIM};
use prieco_insert::db_insert::{MAX_VECTORS_IN_VRAM, merge_bucket, vector_process};

fn record(id: u64, fill: f32) -> Vec<u8> {
    let mut r = Vec::with_capacity(RECORD_SIZE);
    r.extend_from_slice(&id.to_le_bytes());
    for i in 0..VECTOR_DIM {
        r.extend_from_slice(&(fill + i as f32 * 0.001).to_le_bytes());
    }
    r
}

fn staging_path(bucket: usize) -> String {
    format!("{}/staging_{:06}.bin", PRIECO_CONFIG.vector_path, bucket)
}

fn bucket_path(bucket: usize) -> String {
    format!("{}/bucket_{:06}.bin.zst", PRIECO_CONFIG.vector_path, bucket)
}

fn write_staging(bucket: usize, records: &[Vec<u8>]) {
    fs::write(staging_path(bucket), records.concat()).unwrap();
}

fn write_bucket(bucket: usize, records: &[Vec<u8>]) {
    let data = zstd::stream::encode_all(&records.concat()[..], 3).unwrap();
    fs::write(bucket_path(bucket), data).unwrap();
}

/// (id, first float) of every record in the bucket
fn read_bucket(bucket: usize) -> Vec<(u64, f32)> {
    let mut dec = zstd::stream::Decoder::new(fs::File::open(bucket_path(bucket)).unwrap()).unwrap();
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw).unwrap();
    assert_eq!(raw.len() % RECORD_SIZE, 0, "bucket contains a partial record");
    raw.chunks_exact(RECORD_SIZE)
        .map(|r| {
            (
                u64::from_le_bytes(r[..ID_SIZE].try_into().unwrap()),
                f32::from_le_bytes(r[ID_SIZE..ID_SIZE + 4].try_into().unwrap()),
            )
        })
        .collect()
}

fn no_tmp_files(bucket: usize) -> bool {
    !std::path::Path::new(&format!(
        "{}/bucket_{:06}.bin.tmp",
        PRIECO_CONFIG.vector_path, bucket
    ))
    .exists()
}

#[test]
fn constants() {
    assert_eq!(MAX_VECTORS_IN_VRAM, 1_500_000);
    assert_eq!(record(1, 0.0).len(), RECORD_SIZE);
}

#[test]
fn merge_without_staging_is_a_no_op() {
    sandbox();
    merge_bucket(1).unwrap();
    assert!(!std::path::Path::new(&bucket_path(1)).exists());
}

#[test]
fn merge_without_staging_keeps_existing_bucket() {
    sandbox();
    write_bucket(2, &[record(10, 1.0)]);
    merge_bucket(2).unwrap();
    assert_eq!(read_bucket(2), vec![(10, 1.0)]);
}

#[test]
fn merge_creates_bucket_from_staging() {
    sandbox();
    write_staging(3, &[record(1, 1.0), record(2, 2.0), record(3, 3.0)]);
    merge_bucket(3).unwrap();
    assert_eq!(read_bucket(3), vec![(1, 1.0), (2, 2.0), (3, 3.0)]);
    assert!(!std::path::Path::new(&staging_path(3)).exists(), "staging must be removed");
    assert!(no_tmp_files(3));
}

#[test]
fn merge_preserves_full_vectors_bit_exact() {
    sandbox();
    let rec = record(77, -12.5);
    write_staging(4, &[rec.clone()]);
    merge_bucket(4).unwrap();
    let mut dec = zstd::stream::Decoder::new(fs::File::open(bucket_path(4)).unwrap()).unwrap();
    let mut raw = Vec::new();
    dec.read_to_end(&mut raw).unwrap();
    assert_eq!(raw, rec);
}

#[test]
fn merge_duplicate_ids_in_staging_keep_last_write() {
    sandbox();
    write_staging(5, &[record(1, 1.0), record(2, 2.0), record(1, 9.0)]);
    merge_bucket(5).unwrap();
    assert_eq!(read_bucket(5), vec![(2, 2.0), (1, 9.0)]);
}

#[test]
fn merge_staging_replaces_old_bucket_entries() {
    sandbox();
    write_bucket(6, &[record(1, 1.0), record(3, 3.0)]);
    write_staging(6, &[record(1, 100.0), record(4, 4.0)]);
    merge_bucket(6).unwrap();
    assert_eq!(read_bucket(6), vec![(3, 3.0), (1, 100.0), (4, 4.0)]);
}

#[test]
fn merge_truncates_partial_trailing_record() {
    sandbox();
    let mut data = [record(1, 1.0), record(2, 2.0)].concat();
    data.extend_from_slice(&[0xAB; 100]);
    fs::write(staging_path(7), data).unwrap();
    merge_bucket(7).unwrap();
    assert_eq!(read_bucket(7), vec![(1, 1.0), (2, 2.0)]);
}

#[test]
fn merge_staging_smaller_than_one_record() {
    sandbox();
    write_bucket(8, &[record(5, 5.0)]);
    fs::write(staging_path(8), [1u8; 10]).unwrap();
    merge_bucket(8).unwrap();
    assert_eq!(read_bucket(8), vec![(5, 5.0)]);
}

#[test]
fn merge_empty_staging_keeps_old_bucket() {
    sandbox();
    write_bucket(9, &[record(1, 1.0), record(2, 2.0)]);
    fs::write(staging_path(9), b"").unwrap();
    merge_bucket(9).unwrap();
    assert_eq!(read_bucket(9), vec![(1, 1.0), (2, 2.0)]);
    assert!(!std::path::Path::new(&staging_path(9)).exists());
}

#[test]
fn merge_corrupt_bucket_is_rebuilt_from_staging() {
    sandbox();
    fs::write(bucket_path(10), b"this is not zstd at all").unwrap();
    write_staging(10, &[record(1, 1.0)]);
    merge_bucket(10).unwrap();
    assert_eq!(read_bucket(10), vec![(1, 1.0)]);
}

#[test]
fn merge_twice_accumulates() {
    sandbox();
    write_staging(11, &[record(1, 1.0)]);
    merge_bucket(11).unwrap();
    write_staging(11, &[record(2, 2.0)]);
    merge_bucket(11).unwrap();
    write_staging(11, &[record(1, 3.0)]);
    merge_bucket(11).unwrap();
    assert_eq!(read_bucket(11), vec![(2, 2.0), (1, 3.0)]);
}

#[test]
fn merge_buckets_are_independent() {
    sandbox();
    write_staging(12, &[record(1, 1.0)]);
    write_staging(13, &[record(2, 2.0)]);
    merge_bucket(12).unwrap();
    assert!(std::path::Path::new(&staging_path(13)).exists());
    merge_bucket(13).unwrap();
    assert_eq!(read_bucket(12), vec![(1, 1.0)]);
    assert_eq!(read_bucket(13), vec![(2, 2.0)]);
}

#[test]
fn merge_large_bucket() {
    sandbox();
    let recs: Vec<Vec<u8>> = (0..2_000u64).map(|i| record(i, i as f32)).collect();
    write_staging(14, &recs);
    merge_bucket(14).unwrap();
    let got = read_bucket(14);
    assert_eq!(got.len(), 2_000);
    assert_eq!(got[1999], (1999, 1999.0));
}

#[test]
fn bucket_ids_wider_than_six_digits() {
    sandbox();
    let id = 1_234_567;
    write_staging(id, &[record(1, 1.0)]);
    merge_bucket(id).unwrap();
    assert!(bucket_path(id).ends_with("bucket_1234567.bin.zst"));
    assert_eq!(read_bucket(id), vec![(1, 1.0)]);
}

#[test]
fn merge_extreme_ids() {
    sandbox();
    write_staging(15, &[record(0, 0.5), record(u64::MAX, 1.5)]);
    merge_bucket(15).unwrap();
    assert_eq!(read_bucket(15), vec![(0, 0.5), (u64::MAX, 1.5)]);
}

/* ───────────────────────────── vector_process ───────────────────────────── */

#[test]
fn vector_process_empty_buffer_is_ok() {
    sandbox();
    let mut buf: HashMap<u64, Vec<f32>> = HashMap::new();
    vector_process(&mut buf).unwrap();
    assert!(buf.is_empty());
}

/// BUG: without the `cuda` feature `CentroidIndex::assign_batch` returns an empty Vec,
/// and `vector_process` indexes `bucket_ids[i]`, panicking with index out of bounds
/// instead of returning an error. The whole inserter thread dies.
#[cfg(not(feature = "cuda"))]
#[test]
fn vector_process_without_cuda_returns_error_instead_of_panicking() {
    sandbox();
    let mut buf: HashMap<u64, Vec<f32>> = HashMap::new();
    buf.insert(42, vec![0.1; VECTOR_DIM]);
    let r = catch_unwind(AssertUnwindSafe(|| vector_process(&mut buf)));
    assert!(r.is_ok(), "vector_process panicked");
}
