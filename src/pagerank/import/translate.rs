/*
  File: pagerank/import/translate.rs
  Description:

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created:: 2026-02-26
  Last Modified: 2026-03-31

  Usage:
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    fs::{File, remove_file, rename},
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    time::Instant,
};

/*
  Import own libraries
*/
use crate::{
    globals::icons,
    pagerank::compute::{BUFFER_SIZE, EDGES_DIR, EDGES_SORTED, ID_MAP_FILE, read_u64_pair},
};

pub fn run(hash_shards: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    run_with(hash_shards, ID_MAP_FILE, EDGES_SORTED, EDGES_DIR)
}

pub fn run_with(
    hash_shards: Vec<String>,
    id_map_file: &str,
    edges_sorted: &str,
    edges_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}: translate Pass A: sorting hash pairs...",
        icons::PAGERANK_ICON
    );
    let t = Instant::now();

    let mut sorted_hash_shards: Vec<String> = Vec::new();
    let mut chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);

    for sp in &hash_shards {
        let mut r = BufReader::with_capacity(1 << 23, File::open(sp)?);
        while let Some((fh, th)) = read_u64_pair(&mut r)? {
            chunk.push((fh, th));
            if chunk.len() >= BUFFER_SIZE {
                flush_sorted(&mut chunk, &mut sorted_hash_shards, edges_dir)?;
            }
        }
    }
    if !chunk.is_empty() {
        flush_sorted(&mut chunk, &mut sorted_hash_shards, edges_dir)?;
    }
    println!(
        "{}: Pass A done in {:.3}s, {} shards",
        icons::PAGERANK_ICON,
        t.elapsed().as_secs_f64(),
        sorted_hash_shards.len()
    );

    for p in &hash_shards {
        let _ = remove_file(p);
    }

    println!(
        "{}: translate Pass A2: exploding to hash shards...",
        icons::PAGERANK_ICON
    );
    let t = Instant::now();
    let mut hash_only_shards: Vec<String> = Vec::new();
    let mut hash_chunk: Vec<u64> = Vec::with_capacity(BUFFER_SIZE * 2);

    for p in &sorted_hash_shards {
        let mut r = BufReader::with_capacity(1 << 23, File::open(p)?);
        while let Some((fh, th)) = read_u64_pair(&mut r)? {
            hash_chunk.push(fh);
            hash_chunk.push(th);
            if hash_chunk.len() >= BUFFER_SIZE * 2 {
                flush_hashes(&mut hash_chunk, &mut hash_only_shards, edges_dir)?;
            }
        }
    }
    if !hash_chunk.is_empty() {
        flush_hashes(&mut hash_chunk, &mut hash_only_shards, edges_dir)?;
    }
    println!(
        "{}: Pass A2 done in {:.3}s, {} hash shards",
        icons::PAGERANK_ICON,
        t.elapsed().as_secs_f64(),
        hash_only_shards.len()
    );

    // Pass B: k-way merge hash shards against ID map sequentially — zero random access
    println!(
        "{}: translate Pass B: k-way merge + ID map scan...",
        icons::PAGERANK_ICON
    );
    let t = Instant::now();

    let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();
    let mut hash_readers: Vec<BufReader<File>> = hash_only_shards
        .iter()
        .map(|p| BufReader::with_capacity(1 << 23, File::open(p).unwrap()))
        .collect();

    for (i, r) in hash_readers.iter_mut().enumerate() {
        let mut buf = [0u8; 8];
        if r.read_exact(&mut buf).is_ok() {
            heap.push((Reverse(u64::from_le_bytes(buf)), i));
        }
    }

    let mut translations: Vec<(u64, u64)> = Vec::new();
    let mut last_hash: Option<u64> = None;
    let mut map_buf = [0u8; 16];
    let mut id_map_reader = BufReader::with_capacity(1 << 23, File::open(id_map_file)?);
    let mut map_hash = 0u64;
    let mut map_id = 0u64;
    let mut map_exhausted = false;

    // Prime the ID map with first entry
    match id_map_reader.read_exact(&mut map_buf) {
        Ok(_) => {
            map_hash = u64::from_le_bytes(map_buf[0..8].try_into().unwrap());
            map_id = u64::from_le_bytes(map_buf[8..16].try_into().unwrap());
        }
        Err(_) => map_exhausted = true,
    }

    while let Some((Reverse(hash), idx)) = heap.pop() {
        // Advance the shard reader first
        let mut buf = [0u8; 8];
        if hash_readers[idx].read_exact(&mut buf).is_ok() {
            heap.push((Reverse(u64::from_le_bytes(buf)), idx));
        }

        // Skip duplicates
        if last_hash == Some(hash) {
            continue;
        }
        last_hash = Some(hash);

        if !map_exhausted {
            // Advance ID map until map_hash >= hash
            while map_hash < hash {
                match id_map_reader.read_exact(&mut map_buf) {
                    Ok(_) => {
                        map_hash = u64::from_le_bytes(map_buf[0..8].try_into().unwrap());
                        map_id = u64::from_le_bytes(map_buf[8..16].try_into().unwrap());
                    }
                    Err(_) => {
                        map_exhausted = true;
                        break;
                    }
                }
            }
            if !map_exhausted && map_hash == hash {
                translations.push((hash, map_id));
            }
        }
    }

    for p in &hash_only_shards {
        let _ = remove_file(p);
    }

    println!(
        "{}: resolved {} translations in {:.3}s",
        icons::PAGERANK_ICON,
        translations.len(),
        t.elapsed().as_secs_f64()
    );

    // Step 3: translate pairs using resolved translations (binary search on small vec)
    println!(
        "{}: translate Pass B step 3: translating pairs...",
        icons::PAGERANK_ICON
    );
    let t = Instant::now();
    let mut new_edge_shards: Vec<String> = Vec::new();
    let mut edge_chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut total_read: u64 = 0;
    let mut total_kept: u64 = 0;

    for p in &sorted_hash_shards {
        let mut r = BufReader::with_capacity(1 << 23, File::open(p)?);
        while let Some((fh, th)) = read_u64_pair(&mut r)? {
            total_read += 1;
            let src = translations
                .binary_search_by_key(&fh, |&(h, _)| h)
                .ok()
                .map(|i| translations[i].1);
            let dst = translations
                .binary_search_by_key(&th, |&(h, _)| h)
                .ok()
                .map(|i| translations[i].1);
            if let (Some(src_id), Some(dst_id)) = (src, dst) {
                total_kept += 1;
                edge_chunk.push((src_id, dst_id));
                if edge_chunk.len() >= BUFFER_SIZE {
                    flush_chunk_raw(&mut edge_chunk, &mut new_edge_shards, edges_dir)?;
                }
            }
        }
    }
    if !edge_chunk.is_empty() {
        flush_chunk_raw(&mut edge_chunk, &mut new_edge_shards, edges_dir)?;
    }
    println!(
        "{}: translation phase: {:.3}s (read={}, kept={}, shards={})",
        icons::PAGERANK_ICON,
        t.elapsed().as_secs_f64(),
        total_read,
        total_kept,
        new_edge_shards.len()
    );

    for p in &sorted_hash_shards {
        let _ = remove_file(p);
    }

    /*
      Merge new edge shards with existing edges_sorted
    */
    println!(
        "{}: Merging {} shards into final output...",
        icons::PAGERANK_ICON,
        new_edge_shards.len()
    );
    let t = Instant::now();

    let mut all_shards = new_edge_shards.clone();
    if Path::new(edges_sorted).exists() {
        all_shards.push(edges_sorted.to_string());
    }

    let merged_edges = format!("{}/edges_merged.bin.zst", edges_dir);
    merge_sort_edges(&all_shards, &merged_edges)?;
    println!(
        "{}: Merge: {:.3}s",
        icons::PAGERANK_ICON,
        t.elapsed().as_secs_f64()
    );

    for p in &new_edge_shards {
        let _ = remove_file(p);
    }

    rename(&merged_edges, edges_sorted).map_err(|e| {
        let _ = remove_file(&merged_edges);
        e
    })?;

    Ok(())
}

// Sort and flush a chunk of hash pairs to disk
fn flush_sorted(
    buffer: &mut Vec<(u64, u64)>,
    shards: &mut Vec<String>,
    dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable();
    buffer.dedup();
    let path = format!("{}/sorted_hash_shard_{}.bin", dir, shards.len());
    let mut w = BufWriter::with_capacity(1 << 23, File::create(&path)?);
    for (a, b) in buffer.iter() {
        w.write_all(&a.to_le_bytes())?;
        w.write_all(&b.to_le_bytes())?;
    }
    w.flush()?;
    shards.push(path);
    buffer.clear();
    Ok(())
}

// Flush translated (src_id, dst_id) pairs to disk
fn flush_chunk_raw(
    buffer: &mut Vec<(u64, u64)>,
    shards: &mut Vec<String>,
    dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable();
    buffer.dedup();
    let path = format!("{}/inc_edge_shard_{}.bin", dir, shards.len());
    let mut w = BufWriter::with_capacity(1 << 23, File::create(&path)?);
    for (a, b) in buffer.iter() {
        w.write_all(&a.to_le_bytes())?;
        w.write_all(&b.to_le_bytes())?;
    }
    w.flush()?;
    shards.push(path);
    buffer.clear();
    Ok(())
}

fn flush_hashes(
    buffer: &mut Vec<u64>,
    shards: &mut Vec<String>,
    dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable();
    buffer.dedup();
    let path = format!("{}/hash_only_shard_{}.bin", dir, shards.len());
    let mut w = BufWriter::with_capacity(1 << 23, File::create(&path)?);
    for h in buffer.iter() {
        w.write_all(&h.to_le_bytes())?;
    }
    w.flush()?;
    shards.push(path);
    buffer.clear();
    Ok(())
}

fn read_u64_pair_raw(
    r: &mut (impl Read + ?Sized),
) -> Result<Option<(u64, u64)>, Box<dyn std::error::Error>> {
    let mut buf = [0u8; 16];
    match r.read_exact(&mut buf) {
        Ok(()) => {
            let a = u64::from_le_bytes(buf[0..8].try_into().unwrap());
            let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());
            Ok(Some((a, b)))
        }
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn merge_sort_edges(shards: &[String], out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let t_total = Instant::now();
    let t = Instant::now();
    let mut heap: BinaryHeap<(Reverse<(u64, u64)>, usize)> = BinaryHeap::new();
    let mut readers: Vec<Box<dyn Read>> = Vec::new();

    for (i, path) in shards.iter().enumerate() {
        let mut reader: Box<dyn Read> = if path.ends_with(".zst") {
            Box::new(zstd::Decoder::new(BufReader::with_capacity(
                1 << 23,
                File::open(path)?,
            ))?)
        } else {
            Box::new(BufReader::with_capacity(1 << 23, File::open(path)?))
        };

        if let Some(first) = read_u64_pair_raw(&mut *reader)? {
            heap.push((Reverse(first), i));
        }
        readers.push(reader);
    }
    println!(
        "{}: merge: opened {} readers in {:.3}s",
        icons::PAGERANK_ICON,
        shards.len(),
        t.elapsed().as_secs_f64()
    );

    let f = File::create(out_path)?;
    let buf_writer = BufWriter::with_capacity(1 << 23, f);
    let mut out = zstd::Encoder::new(buf_writer, 3)?;

    const WRITE_BUF_EDGES: usize = 65_536;
    let mut write_buf: Vec<u8> = Vec::with_capacity(WRITE_BUF_EDGES * 16);
    let mut last: Option<(u64, u64)> = None;
    let mut edges_written: u64 = 0;
    let mut last_report = Instant::now();
    let t_loop = Instant::now();

    while let Some((Reverse(val), idx)) = heap.pop() {
        if last != Some(val) {
            write_buf.extend_from_slice(&val.0.to_le_bytes());
            write_buf.extend_from_slice(&val.1.to_le_bytes());
            last = Some(val);
            edges_written += 1;
            if write_buf.len() >= WRITE_BUF_EDGES * 16 {
                out.write_all(&write_buf)?;
                write_buf.clear();
            }
        }
        if let Some(next) = read_u64_pair_raw(&mut *readers[idx])? {
            heap.push((Reverse(next), idx));
        }
        if edges_written % 5_000_000 == 0 && last_report.elapsed().as_secs_f64() > 5.0 {
            let elapsed = t_loop.elapsed().as_secs_f64();
            let rate = edges_written as f64 / elapsed / 1_000_000.0;
            println!(
                "{}: merge: {:.0}M edges written, {:.2}M/s, {:.1}s elapsed",
                icons::PAGERANK_ICON,
                edges_written as f64 / 1_000_000.0,
                rate,
                elapsed
            );
            last_report = Instant::now();
        }
    }

    if !write_buf.is_empty() {
        out.write_all(&write_buf)?;
    }
    out.finish()?;

    println!(
        "{}: merge: DONE — {:.0}M edges written, loop {:.3}s, total {:.3}s",
        icons::PAGERANK_ICON,
        edges_written as f64 / 1_000_000.0,
        t_loop.elapsed().as_secs_f64(),
        t_total.elapsed().as_secs_f64()
    );
    Ok(())
}
