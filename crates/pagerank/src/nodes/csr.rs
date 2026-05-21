/*
  File: pagerank/nodes/csr.rs
  Description:

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created:: 2026-02-26
  Last Modified: 2026-02-31

  Usage:
  TODO:
*/

/*
  Import system libraries
*/
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    fs::{File, remove_file},
    io::{BufReader, BufWriter, Read, Write},
    sync::mpsc::{SyncSender, sync_channel},
    thread,
    time::Instant,
};

use zstd::{Decoder, Encoder};

/*
  Import own libraries
*/
use crate::compute::{
    BUFFER_SIZE, CSR_EDGES, CSR_OFFSETS, EDGES_DIR, EDGES_SORTED, OUT_DEGREE, read_u64_pair_zstd,
    zstd_reader,
};
use prieco_core::globals::icons;

/*
  Tuning constants
*/
const GROUP_SIZE: usize = 20;
const PREFETCH_BUFFER: usize = 32_768;
const WRITE_BATCH: usize = 1 << 23; // 8 MB

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

pub fn run_with(
    total_nodes_usize: usize,
    edges_sorted: &str,
    csr_offsets: &str,
    csr_edges: &str,
    out_degree_path: &str,
    scratch_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();

    /*
      Stream edges_sorted → degree arrays + dst-sorted shards
    */
    let reuse_shards = std::env::var("REUSE_SHARDS").is_ok();

    let mut in_degree: Vec<u32> = vec![0u32; total_nodes_usize];
    let mut out_degree: Vec<u32> = vec![0u32; total_nodes_usize];
    let mut chunk: Vec<(u64, u64)> = Vec::with_capacity(BUFFER_SIZE);
    let mut dst_shards: Vec<String> = Vec::new();

    let degrees_cache = format!("{}/csr_degrees.bin", scratch_dir);

    if reuse_shards {
        // Load existing shards
        let mut paths: Vec<String> = std::fs::read_dir(scratch_dir)?
            .filter_map(|e| {
                let p = e.ok()?.path();
                let name = p.file_name()?.to_str()?;
                name.starts_with("csr_shard_")
                    .then(|| p.to_string_lossy().to_string())
            })
            .collect();
        paths.sort();
        println!(
            "{}: REUSE_SHARDS: found {} existing shards",
            icons::PAGERANK_ICON,
            paths.len()
        );
        dst_shards = paths;

        if std::path::Path::new(&degrees_cache).exists() {
            // Load degree arrays from cache
            println!(
                "{}: REUSE_SHARDS: loading degree arrays from cache...",
                icons::PAGERANK_ICON
            );
            let t_deg = Instant::now();
            let mut f = BufReader::with_capacity(1 << 23, File::open(&degrees_cache)?);
            let mut buf4 = [0u8; 4];
            for i in 0..total_nodes_usize {
                f.read_exact(&mut buf4)?;
                out_degree[i] = u32::from_le_bytes(buf4);
            }
            for i in 0..total_nodes_usize {
                f.read_exact(&mut buf4)?;
                in_degree[i] = u32::from_le_bytes(buf4);
            }
            println!(
                "{}: REUSE_SHARDS: degree arrays loaded in {:.1}s",
                icons::PAGERANK_ICON,
                t_deg.elapsed().as_secs_f64()
            );
        } else {
            // Re-stream edges_sorted
            println!(
                "{}: REUSE_SHARDS: no cache found, re-streaming edges",
                icons::PAGERANK_ICON
            );
            let t_deg = Instant::now();
            let mut dec = zstd_reader(edges_sorted)?;
            while let Some((src, dst)) = read_u64_pair_zstd(&mut dec)? {
                out_degree[src as usize] += 1;
                in_degree[dst as usize] += 1;
            }
            println!(
                "{}: REUSE_SHARDS: degree arrays rebuilt in {:.1}s",
                icons::PAGERANK_ICON,
                t_deg.elapsed().as_secs_f64()
            );
            save_degrees(&out_degree, &in_degree, &degrees_cache)?;
        }
    } else {
        println!(
            "{}: Phase 0: streaming edges, building degree arrays + shards...",
            icons::PAGERANK_ICON
        );
        let t_p0 = Instant::now();
        let mut dec = zstd_reader(edges_sorted)?;
        while let Some((src, dst)) = read_u64_pair_zstd(&mut dec)? {
            out_degree[src as usize] += 1;
            in_degree[dst as usize] += 1;
            chunk.push((dst, src));
            if chunk.len() >= BUFFER_SIZE {
                flush(&mut chunk, &mut dst_shards, scratch_dir)?;
            }
        }
        if !chunk.is_empty() {
            flush(&mut chunk, &mut dst_shards, scratch_dir)?;
        }
        println!(
            "{}: Phase 0 done in {:.1}s — {} shards written",
            icons::PAGERANK_ICON,
            t_p0.elapsed().as_secs_f64(),
            dst_shards.len()
        );

        // Save degree arrays so a future REUSE_SHARDS run
        save_degrees(&out_degree, &in_degree, &degrees_cache)?;
    }

    /*
      Write out_degree
    */
    let t1 = Instant::now();
    println!("{}: Phase 1: writing out_degree...", icons::PAGERANK_ICON);
    {
        let mut enc = zstd_writer_fast(out_degree_path)?;
        let mut batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);
        for &od in &out_degree {
            batch.extend_from_slice(&od.to_le_bytes());
            if batch.len() >= WRITE_BATCH {
                enc.write_all(&batch)?;
                batch.clear();
            }
        }
        if !batch.is_empty() {
            enc.write_all(&batch)?;
        }
        enc.finish()?;
    }
    drop(out_degree);
    println!(
        "{}: Phase 1 done in {:.1}s",
        icons::PAGERANK_ICON,
        t1.elapsed().as_secs_f64()
    );

    /*
      Write csr_offsets directly from in_degree — no offsets vec
    */
    let t2 = Instant::now();
    println!(
        "{}: Phase 2: building + writing CSR offsets...",
        icons::PAGERANK_ICON
    );
    let mut acc: u64 = 0;
    {
        let f = File::create(csr_offsets)?;
        let mut w = BufWriter::with_capacity(WRITE_BATCH, f);
        let mut batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);
        for &deg in &in_degree {
            batch.extend_from_slice(&acc.to_le_bytes());
            if batch.len() >= WRITE_BATCH {
                w.write_all(&batch)?;
                batch.clear();
            }
            acc += deg as u64 * 8;
        }

        batch.extend_from_slice(&acc.to_le_bytes());
        if !batch.is_empty() {
            w.write_all(&batch)?;
        }
        w.flush()?;
    }
    drop(in_degree);
    println!(
        "{}: Phase 2 done in {:.1}s — expected csr_edges size: {:.2} GB",
        icons::PAGERANK_ICON,
        t2.elapsed().as_secs_f64(),
        acc as f64 / 1e9
    );

    /*
      Merge groups of shards in parallel → intermediate files
    */
    let t3 = Instant::now();
    let n_shards = dst_shards.len();
    let groups: Vec<Vec<String>> = dst_shards.chunks(GROUP_SIZE).map(|c| c.to_vec()).collect();
    let n_groups = groups.len();
    println!(
        "{}: Phase 3A: merging {} shards in {} groups of ~{}...",
        icons::PAGERANK_ICON,
        n_shards,
        n_groups,
        GROUP_SIZE
    );

    let intermediate_paths: Vec<String> = (0..n_groups)
        .map(|i| format!("{}/csr_intermediate_{}.bin.zst", scratch_dir, i))
        .collect();

    let mut handles = Vec::with_capacity(n_groups);
    for (g_idx, group) in groups.into_iter().enumerate() {
        let out_path = intermediate_paths[g_idx].clone();
        let t_group = Instant::now();

        let handle = thread::spawn(move || -> Result<(), String> {
            let mut receivers = Vec::with_capacity(group.len());
            for shard_path in &group {
                let (tx, rx): (SyncSender<Option<(u64, u64)>>, _) = sync_channel(PREFETCH_BUFFER);
                let path = shard_path.clone();
                thread::spawn(move || {
                    let file = File::open(&path).unwrap();
                    let mut r = Decoder::new(BufReader::with_capacity(1 << 20, file)).unwrap();
                    let mut buf = [0u8; 16];
                    loop {
                        match r.read_exact(&mut buf) {
                            Ok(_) => {
                                let a = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                                let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());
                                if tx.send(Some((a, b))).is_err() {
                                    break;
                                }
                            }
                            Err(_) => {
                                let _ = tx.send(None);
                                break;
                            }
                        }
                    }
                });
                receivers.push(rx);
            }

            struct SR {
                rx: std::sync::mpsc::Receiver<Option<(u64, u64)>>,
                cur: Option<(u64, u64)>,
            }
            let mut readers: Vec<SR> = receivers
                .into_iter()
                .map(|rx| {
                    let cur = rx.recv().ok().flatten();
                    SR { rx, cur }
                })
                .collect();

            let mut heap: BinaryHeap<(Reverse<(u64, u64)>, usize)> = BinaryHeap::new();
            for (i, r) in readers.iter().enumerate() {
                if let Some(v) = r.cur {
                    heap.push((Reverse(v), i));
                }
            }

            let f = File::create(&out_path).map_err(|e| e.to_string())?;
            let mut enc =
                Encoder::new(BufWriter::with_capacity(1 << 23, f), 3).map_err(|e| e.to_string())?;
            let mut write_batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);

            while let Some((Reverse(val), idx)) = heap.pop() {
                write_batch.extend_from_slice(&val.0.to_le_bytes());
                write_batch.extend_from_slice(&val.1.to_le_bytes());
                if write_batch.len() >= WRITE_BATCH {
                    enc.write_all(&write_batch).map_err(|e| e.to_string())?;
                    write_batch.clear();
                }
                readers[idx].cur = readers[idx].rx.recv().ok().flatten();
                if let Some(v) = readers[idx].cur {
                    heap.push((Reverse(v), idx));
                }
            }
            if !write_batch.is_empty() {
                enc.write_all(&write_batch).map_err(|e| e.to_string())?;
            }
            enc.finish().map_err(|e| e.to_string())?;

            println!(
                "{}:   group {} done in {:.1}s",
                icons::PAGERANK_ICON,
                g_idx,
                t_group.elapsed().as_secs_f64()
            );
            Ok(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle
            .join()
            .unwrap()
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    for p in &dst_shards {
        let _ = remove_file(p);
    }
    println!(
        "{}: Phase 3A done in {:.1}s",
        icons::PAGERANK_ICON,
        t3.elapsed().as_secs_f64()
    );

    /*
      Final merge of intermediate files → csr_edges
    */
    let t3b = Instant::now();
    println!(
        "{}: Phase 3B: final merge of {} intermediate files...",
        icons::PAGERANK_ICON,
        n_groups
    );

    {
        let mut receivers = Vec::with_capacity(n_groups);
        for int_path in &intermediate_paths {
            let (tx, rx): (SyncSender<Option<(u64, u64)>>, _) = sync_channel(PREFETCH_BUFFER);
            let path = int_path.clone();
            thread::spawn(move || {
                let file = File::open(&path).unwrap();
                let mut r = Decoder::new(BufReader::with_capacity(1 << 23, file)).unwrap();
                let mut buf = [0u8; 16];
                loop {
                    match r.read_exact(&mut buf) {
                        Ok(_) => {
                            let a = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                            let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());
                            if tx.send(Some((a, b))).is_err() {
                                break;
                            }
                        }
                        Err(_) => {
                            let _ = tx.send(None);
                            break;
                        }
                    }
                }
            });
            receivers.push(rx);
        }

        struct SR {
            rx: std::sync::mpsc::Receiver<Option<(u64, u64)>>,
            cur: Option<(u64, u64)>,
        }
        let mut readers: Vec<SR> = receivers
            .into_iter()
            .map(|rx| {
                let cur = rx.recv().ok().flatten();
                SR { rx, cur }
            })
            .collect();

        let mut heap: BinaryHeap<(Reverse<(u64, u64)>, usize)> = BinaryHeap::new();
        for (i, r) in readers.iter().enumerate() {
            if let Some(v) = r.cur {
                heap.push((Reverse(v), i));
            }
        }

        let f = File::create(csr_edges)?;
        let mut enc = Encoder::new(BufWriter::with_capacity(1 << 23, f), 3)?;
        let mut write_batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);
        let mut edges_written: u64 = 0;
        let mut last_report = Instant::now();

        while let Some((Reverse((_dst, src)), idx)) = heap.pop() {
            write_batch.extend_from_slice(&src.to_le_bytes());
            edges_written += 1;
            if write_batch.len() >= WRITE_BATCH {
                enc.write_all(&write_batch)?;
                write_batch.clear();
            }
            if last_report.elapsed().as_secs_f64() >= 30.0 {
                println!(
                    "{}:   3B progress: {:.0}M edges written, {:.1}s elapsed",
                    icons::PAGERANK_ICON,
                    edges_written as f64 / 1e6,
                    t3b.elapsed().as_secs_f64()
                );
                last_report = Instant::now();
            }
            readers[idx].cur = readers[idx].rx.recv().ok().flatten();
            if let Some(v) = readers[idx].cur {
                heap.push((Reverse(v), idx));
            }
        }
        if !write_batch.is_empty() {
            enc.write_all(&write_batch)?;
        }
        enc.finish()?;

        println!(
            "{}: Phase 3B done in {:.1}s — {:.0}M edges written",
            icons::PAGERANK_ICON,
            t3b.elapsed().as_secs_f64(),
            edges_written as f64 / 1e6
        );
    }

    for p in &intermediate_paths {
        let _ = remove_file(p);
    }

    println!(
        "{}: Expected uncompressed csr_edges size: {:.2} GB",
        icons::PAGERANK_ICON,
        acc as f64 / 1e9
    );
    println!(
        "{}: Total wall time: {:.1}s",
        icons::PAGERANK_ICON,
        t0.elapsed().as_secs_f64()
    );

    Ok(())
}

fn flush(
    buffer: &mut Vec<(u64, u64)>,
    shards: &mut Vec<String>,
    scratch_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    buffer.sort_unstable_by_key(|p| p.0);
    let path = format!("{}/csr_shard_{}.bin.zst", scratch_dir, shards.len());
    let f = File::create(&path)?;
    let mut enc = Encoder::new(BufWriter::with_capacity(1 << 23, f), 3)?;
    let mut batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);
    for (a, b) in buffer.iter() {
        batch.extend_from_slice(&a.to_le_bytes());
        batch.extend_from_slice(&b.to_le_bytes());
        if batch.len() >= WRITE_BATCH {
            enc.write_all(&batch)?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        enc.write_all(&batch)?;
    }
    enc.finish()?;
    shards.push(path);
    buffer.clear();
    Ok(())
}

fn save_degrees(
    out_degree: &[u32],
    in_degree: &[u32],
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let t = Instant::now();
    println!("{}: saving degree arrays to cache...", icons::PAGERANK_ICON);
    let f = File::create(path)?;
    let mut w = BufWriter::with_capacity(1 << 23, f);
    let mut batch: Vec<u8> = Vec::with_capacity(WRITE_BATCH);
    for &v in out_degree.iter().chain(in_degree.iter()) {
        batch.extend_from_slice(&v.to_le_bytes());
        if batch.len() >= WRITE_BATCH {
            w.write_all(&batch)?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        w.write_all(&batch)?;
    }
    w.flush()?;
    println!(
        "{}: degree cache saved in {:.1}s",
        icons::PAGERANK_ICON,
        t.elapsed().as_secs_f64()
    );
    Ok(())
}

fn zstd_writer_fast(
    path: &str,
) -> Result<Encoder<'static, BufWriter<File>>, Box<dyn std::error::Error>> {
    let f = File::create(path)?;
    let writer = BufWriter::with_capacity(1 << 23, f);
    let encoder = Encoder::new(writer, 1)?;
    Ok(encoder)
}
