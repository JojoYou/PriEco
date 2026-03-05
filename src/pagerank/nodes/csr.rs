/*
  File: pagerank/nodes/csr.rs
  Description:

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created:: 2026-02-26
  Last Modified: 2026-02-27

  Usage:
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    fs::{File, metadata, remove_file},
    io::{BufReader, BufWriter, Write},
};

/*
  Import external libraries
*/
use zstd::Decoder;

/*
  Import own libraries
*/
use crate::pagerank::compute::{
    BUFFER_SIZE, CSR_EDGES, CSR_OFFSETS, EDGES_DIR, EDGES_SORTED, OUT_DEGREE, read_u64_pair_zstd,
    zstd_reader, zstd_writer,
};

/*
  Description: Classical call, split like this so that the tests could call it with custom paths

  Input: Total nodes count
  Output: Result shards paths
*/
pub fn run(total_nodes_usize: usize) -> Result<(), Box<dyn std::error::Error>> {
    run_with(
        total_nodes_usize,
        EDGES_SORTED,
        CSR_OFFSETS,
        CSR_EDGES,
        OUT_DEGREE,
        EDGES_DIR,
    )
}

/*
  Description: Build the CSR representation from the sorted edge list

  Input: Total nodes count
  Output: Result shards paths
*/
pub fn run_with(
    total_nodes_usize: usize,
    edges_sorted: &str,
    csr_offsets: &str,
    csr_edges: &str,
    out_degree_path: &str,
    scratch_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Count degrees
    let mut in_degree: Vec<u32> = vec![0u32; total_nodes_usize];
    let mut out_degree: Vec<u32> = vec![0u32; total_nodes_usize];
    {
        let mut dec = zstd_reader(edges_sorted)?;
        while let Some((src, dst)) = read_u64_pair_zstd(&mut dec)? {
            out_degree[src as usize] += 1;
            in_degree[dst as usize] += 1;
        }
    }

    // Write out_degree
    {
        let mut enc = zstd_writer(out_degree_path)?;
        for &od in &out_degree {
            enc.write_all(&od.to_le_bytes()).unwrap();
        }
        enc.finish()?;
    }

    // Compute CSR byte offsets (prefix sum of in_degree × 8 bytes per u64 edge)
    let mut offsets: Vec<u64> = Vec::with_capacity(total_nodes_usize + 1);
    let mut acc: u64 = 0;
    for &deg in &in_degree {
        offsets.push(acc);
        acc += deg as u64 * 8;
    }
    offsets.push(acc);
    drop(in_degree); // free RAM — no longer needed

    {
        let mut enc = zstd_writer(csr_offsets)?;
        for &off in &offsets {
            enc.write_all(&off.to_le_bytes()).unwrap();
        }
        enc.finish()?;
    }

    // Sort edges by dst: write (dst, src) chunk shards sorted by dst, merge them.
    let mut chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE); // (dst, src)
    let mut dst_shards: Vec<String> = Vec::new();

    {
        let mut dec = zstd_reader(edges_sorted)?;
        while let Some((src, dst)) = read_u64_pair_zstd(&mut dec)? {
            chunk.push((dst, src)); // flip: group by destination
            if chunk.len() >= BUFFER_SIZE {
                flush(&mut chunk, &mut dst_shards, scratch_dir)?;
            }
        }
        if !chunk.is_empty() {
            flush(&mut chunk, &mut dst_shards, scratch_dir)?;
        }
    }

    // Merge dst_shards in order, write csr_edges.bin sequentially
    {
        struct SR<'a> {
            dec: Decoder<'a, BufReader<File>>,
            cur: Option<(u64, u64)>,
        }
        let mut readers: Vec<SR> = Vec::new();
        for p in &dst_shards {
            let mut dec = zstd_reader(p.as_str())?;
            let cur = read_u64_pair_zstd(&mut dec)?;

            readers.push(SR { dec, cur });
        }

        let mut out = BufWriter::with_capacity(1 << 20, File::create(csr_edges).unwrap());

        // Verify we write in the order offsets expects: all in-neighbors of node 0,
        // then node 1, etc. Because edges are sorted by dst this falls out naturally.
        loop {
            let best = readers
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.cur.map(|v| (i, v)))
                .min_by_key(|&(_, v)| v.0); // min by dst
            let (idx, (_dst, src)) = match best {
                Some(x) => x,
                None => break,
            };
            out.write_all(&src.to_le_bytes()).unwrap();
            readers[idx].cur = read_u64_pair_zstd(&mut readers[idx].dec)?;
        }
    }

    for p in &dst_shards {
        let _ = remove_file(p);
    }

    // Verify file size matches what offsets promised
    let csr_file_size = metadata(csr_edges).unwrap().len();
    assert_eq!(
        csr_file_size, acc,
        "csr_edges.bin size mismatch: wrote {} bytes, expected {}",
        csr_file_size, acc
    );

    Ok(())
}

/* Helper functions */
/*
  Description: Flush a buffer of (dst, src) pairs to a compressed shard

  Input: Total nodes count
  Output: None
*/
fn flush(
    buffer: &mut Vec<(u64, u64)>,
    shards: &mut Vec<String>,
    scratch_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable_by_key(|p| p.0);

    let path = format!("{}/csr_shard_{}.bin.zst", scratch_dir, shards.len());

    let mut enc = zstd_writer(&path)?;

    // Write all pairs
    for (a, b) in buffer.iter() {
        enc.write_all(&a.to_le_bytes())?;
        enc.write_all(&b.to_le_bytes())?;
    }

    enc.finish()?;

    shards.push(path);
    buffer.clear();

    Ok(())
}
