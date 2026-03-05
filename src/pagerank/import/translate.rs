/*
  Import system libraries
*/
use std::{
    fs::{File, remove_file, rename},
    io::{BufReader, Read, Write},
    path::Path,
};

/*
  Import external libraries
*/
use zstd::Decoder;

/*
  Import own libraries
*/
use crate::{
    globals::icons,
    pagerank::compute::{
        BUFFER_SIZE, EDGES_DIR, EDGES_SORTED, ID_MAP_FILE, read_u64_pair_zstd, zstd_reader,
        zstd_writer,
    },
};

/*
  Description: Classical call, split like this so that the tests could call it with custom paths

  Input: Shard file paths, Merged ID map file path
  Output: None
*/
pub fn run(hash_shards: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    run_with(hash_shards, ID_MAP_FILE, EDGES_SORTED, EDGES_DIR)
}

/*
  Description: Translates hash-based edges into ID-based edges using merged ID map

  Input: Shard file paths, Merged ID map file path
  Output: None
*/
pub fn run_with(
    hash_shards: Vec<String>,
    id_map_file: &str,
    edges_sorted: &str,
    edges_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}: Phase 1: Pass C: Translating edges!", icons::PAGERANK);

    /*
      Read from-hash pairs from shards and flush them in sorted chunks
    */
    let mut from_chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut from_shards: Vec<String> = Vec::new();
    {
        for sp in &hash_shards {
            let mut dec = zstd_reader(sp)?;
            while let Some(pair) = read_u64_pair_zstd(&mut dec)? {
                from_chunk.push(pair);
                if from_chunk.len() >= BUFFER_SIZE {
                    flush_chunk(
                        &mut from_chunk,
                        &mut from_shards,
                        edges_dir,
                        "inc_from_shard",
                        Some(|p| p.0),
                        false,
                    )?;
                }
            }
        }
        if !from_chunk.is_empty() {
            flush_chunk(
                &mut from_chunk,
                &mut from_shards,
                edges_dir,
                "inc_from_shard",
                Some(|p| p.0),
                false,
            )?;
        }
    }

    /*
      Merge-join from_shards with merged_id_map
      translate `from_hash` -> `to_hash` into `to_hash` -> `src_id` using ID map
    */
    let mut to_chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut to_shards: Vec<String> = Vec::new();
    {
        // Shard reader with a peeked value
        struct SR<'a> {
            dec: Decoder<'a, BufReader<File>>,
            cur: Option<(u64, u64)>, // peeked pair from shard
        }
        let fp: Vec<String> = from_shards.clone();
        let mut readers: Vec<SR> = Vec::new();
        for p in &fp {
            let mut dec = zstd_reader(p)?;
            let cur = read_u64_pair_zstd(&mut dec)?;
            readers.push(SR { dec, cur });
        }

        // Open merged ID map
        let mut id_dec = zstd_reader(&id_map_file)?;
        let mut id_buf = [0u8; 16];
        let mut id_hash = 0u64;
        let mut id_val = 0u64;
        let mut id_valid = false;
        let adv = |dec: &mut Decoder<BufReader<File>>,
                   buf: &mut [u8; 16],
                   h: &mut u64,
                   v2: &mut u64,
                   valid: &mut bool| {
            *valid = dec.read_exact(buf).is_ok();
            if *valid {
                *h = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                *v2 = u64::from_le_bytes(buf[8..16].try_into().unwrap());
            }
        };
        adv(
            &mut id_dec,
            &mut id_buf,
            &mut id_hash,
            &mut id_val,
            &mut id_valid,
        );

        // Merge loop: match from_shard hash with ID map hash
        loop {
            // Pick smallest from_hash
            let best = readers
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.cur.map(|v| (i, v)))
                .min_by_key(|&(_, v)| v.0);
            let (idx, (fh, th)) = match best {
                Some(x) => x,
                None => break,
            };

            // Go through ID map until it matches from_hash
            while id_valid && id_hash < fh {
                adv(
                    &mut id_dec,
                    &mut id_buf,
                    &mut id_hash,
                    &mut id_val,
                    &mut id_valid,
                );
            }

            // Push (to_hash, src_id) to chunk
            if id_valid && id_hash == fh {
                to_chunk.push((th, id_val));
                if to_chunk.len() >= BUFFER_SIZE {
                    flush_chunk(
                        &mut to_chunk,
                        &mut to_shards,
                        edges_dir,
                        "to_shard",
                        Some(|p| p.0),
                        false,
                    )?;
                }
            }
            readers[idx].cur = read_u64_pair_zstd(&mut readers[idx].dec)?;
        }
        if !to_chunk.is_empty() {
            flush_chunk(
                &mut to_chunk,
                &mut to_shards,
                edges_dir,
                "to_shard",
                Some(|p| p.0),
                false,
            )?;
        }
    }
    for p in &from_shards {
        let _ = remove_file(p);
    }

    /*
      Merge-join to_shards with merged_id_map
      translate (to_hash, src_id) → (src_id, dst_id) using merged ID map
    */
    let mut edge_chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut new_edge_shards: Vec<String> = Vec::new();
    {
        struct SR2<'a> {
            dec: Decoder<'a, BufReader<File>>,
            cur: Option<(u64, u64)>,
        }
        let tp: Vec<String> = to_shards.clone();
        let mut readers: Vec<SR2> = Vec::new();

        for p in &tp {
            let mut dec = zstd_reader(p.as_str())?;

            let cur = read_u64_pair_zstd(&mut dec)?;

            readers.push(SR2 { dec, cur });
        }
        let mut id_dec = zstd_reader(&id_map_file)?;
        let mut id_buf = [0u8; 16];
        let mut id_hash = 0u64;
        let mut id_val = 0u64;
        let mut id_valid = false;
        let adv = |dec: &mut Decoder<BufReader<File>>,
                   buf: &mut [u8; 16],
                   h: &mut u64,
                   v2: &mut u64,
                   valid: &mut bool| {
            *valid = dec.read_exact(buf).is_ok();
            if *valid {
                *h = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                *v2 = u64::from_le_bytes(buf[8..16].try_into().unwrap());
            }
        };
        adv(
            &mut id_dec,
            &mut id_buf,
            &mut id_hash,
            &mut id_val,
            &mut id_valid,
        );
        loop {
            let best = readers
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.cur.map(|v| (i, v)))
                .min_by_key(|&(_, v)| v.0);
            let (idx, (th, src_id)) = match best {
                Some(x) => x,
                None => break,
            };
            while id_valid && id_hash < th {
                adv(
                    &mut id_dec,
                    &mut id_buf,
                    &mut id_hash,
                    &mut id_val,
                    &mut id_valid,
                );
            }
            if id_valid && id_hash == th {
                edge_chunk.push((src_id, id_val));
                if edge_chunk.len() >= BUFFER_SIZE {
                    flush_chunk(
                        &mut edge_chunk,
                        &mut new_edge_shards,
                        edges_dir,
                        "inc_edge_shard",
                        None,
                        true,
                    )?;
                }
            }
            readers[idx].cur = read_u64_pair_zstd(&mut readers[idx].dec)?;
        }
        if !edge_chunk.is_empty() {
            flush_chunk(
                &mut edge_chunk,
                &mut new_edge_shards,
                edges_dir,
                "inc_edge_shard",
                None,
                true,
            )?;
        }
    }
    for p in &to_shards {
        let _ = remove_file(p);
    }
    for p in &hash_shards {
        let _ = remove_file(p);
    }

    /*
      Merge all new edge shards with existing sorted edges
    */
    let merged_edges = format!("{}/edges_merged.bin.zst", edges_dir);
    let mut all_shards = new_edge_shards.clone();
    if Path::new(edges_sorted).exists() {
        all_shards.push(edges_sorted.to_string());
    }
    merge_sort_edges_zstd(&all_shards, &merged_edges)?;
    for p in &new_edge_shards {
        let _ = remove_file(p);
    }

    // Atomically replace id_map and edges_sorted
    rename(&merged_edges, edges_sorted)?;

    Ok(())
}

/* Helper functions */
/*
  Description: Flushes a chunk of edges to a shard file

  Input: Buffer to write, shard paths to insert new shards paths
  Output: None
*/
fn flush_chunk(
    buffer: &mut Vec<(u64, u64)>,
    shards: &mut Vec<String>,
    dir: &str,
    prefix: &str,
    sort_key: Option<fn(&(u64, u64)) -> u64>,
    dedup_edges: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Sort edges
    match sort_key {
        Some(key_fn) => buffer.sort_unstable_by_key(key_fn),
        None => buffer.sort_unstable(), // sort by whole tuple
    }

    // Deduplicate if needed
    if dedup_edges {
        buffer.dedup();
    }

    // Create file path
    let path = format!("{}/{}_{}.bin.zst", dir, prefix, shards.len());

    // Open zstd writer
    let mut enc = zstd_writer(&path)?;

    // Write all edges
    for (a, b) in buffer.iter() {
        enc.write_all(&a.to_le_bytes())?;
        enc.write_all(&b.to_le_bytes())?;
    }
    enc.finish()?;

    // Track shard and clear buffer
    shards.push(path);
    buffer.clear();

    Ok(())
}

/*
  Description: Merge multiple sorted edge shards into a single sorted output file

  Input: Shard file paths, Output file path
  Output: None
*/
pub fn merge_sort_edges_zstd(
    shards: &[String],
    out_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    struct SR<'a> {
        dec: Decoder<'a, BufReader<File>>,
        cur: Option<(u64, u64)>,
    }
    let paths: Vec<String> = shards.to_vec();
    let mut readers: Vec<SR> = Vec::new();

    for p in &paths {
        let mut dec = zstd_reader(p.as_str())?;

        let cur = read_u64_pair_zstd(&mut dec)?;

        readers.push(SR { dec, cur });
    }

    let mut out = zstd_writer(out_path)?;
    let mut last: Option<(u64, u64)> = None;

    loop {
        let best = readers
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.cur.map(|v| (i, v)))
            .min_by_key(|&(_, v)| v);
        let (idx, val) = match best {
            Some(x) => x,
            None => break,
        };
        if last != Some(val) {
            out.write_all(&val.0.to_le_bytes()).unwrap();
            out.write_all(&val.1.to_le_bytes()).unwrap();
            last = Some(val);
        }
        readers[idx].cur = read_u64_pair_zstd(&mut readers[idx].dec)?;
    }
    out.finish()?;
    Ok(())
}
