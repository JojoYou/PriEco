/*
  File: pagerank/iter/iterate.rs
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
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    mem::swap,
    time::Instant,
};

use memmap2::MmapOptions;
use zstd::Decoder;

/*
  Import own libraries
*/
use crate::compute::{CSR_EDGES, CSR_OFFSETS, OUT_DEGREE, SCORES_A, SCORES_B, zstd_reader};
use prieco_core::globals::icons;

/*
  Constants
*/
const DAMPING: f32 = 0.85;
const EPSILON: f32 = 1e-6;
const MAX_ITER: usize = 100;

pub fn run(total_nodes_usize: usize) -> Result<String, Box<dyn std::error::Error>> {
    run_with(
        total_nodes_usize,
        CSR_OFFSETS,
        CSR_EDGES,
        OUT_DEGREE,
        SCORES_A,
        SCORES_B,
    )
}

pub fn run_with(
    total_nodes_usize: usize,
    csr_offsets_path: &str,
    csr_edges_path: &str,
    out_degree_path: &str,
    scores_a_path: &str,
    scores_b_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let teleport = (1.0 - DAMPING) / total_nodes_usize as f32;

    // Load out_degree into RAM
    println!("{}: Loading out_degree into RAM...", icons::PAGERANK_ICON);
    let t_load = Instant::now();
    let out_degree: Vec<u32> = {
        let mut v = vec![0u32; total_nodes_usize];
        let mut dec = zstd_reader(out_degree_path)?;
        let mut buf = [0u8; 4];
        for i in 0..total_nodes_usize {
            dec.read_exact(&mut buf).unwrap();
            v[i] = u32::from_le_bytes(buf);
        }
        v
    };
    println!(
        "{}: out_degree loaded in {:.1}s",
        icons::PAGERANK_ICON,
        t_load.elapsed().as_secs_f64()
    );

    // Mmap offsets
    println!("{}: mmapping offsets...", icons::PAGERANK_ICON);
    let offsets_file = File::open(csr_offsets_path)?;
    let offsets_mmap = unsafe { MmapOptions::new().map(&offsets_file)? };
    let offsets: &[u64] = unsafe {
        std::slice::from_raw_parts(offsets_mmap.as_ptr() as *const u64, total_nodes_usize + 1)
    };
    println!(
        "{}: offsets mmap ready ({:.2} GB)",
        icons::PAGERANK_ICON,
        offsets_mmap.len() as f64 / 1e9
    );

    let mut read_file = scores_a_path.to_string();
    let mut write_file = scores_b_path.to_string();

    for iteration in 0..MAX_ITER {
        println!("{}: Running iter: {}", icons::PAGERANK_ICON, iteration);
        let s = Instant::now();

        // Load scores_old
        let mut scores_old = vec![0.0f32; total_nodes_usize];
        println!("{}: Allocated scores_old", icons::PAGERANK_ICON);
        let mut dangling_sum = 0.0f64;
        {
            let mut f = BufReader::with_capacity(1 << 24, File::open(&read_file).unwrap());
            let mut buf = [0u8; 4];
            println!("{}: Reading scores a", icons::PAGERANK_ICON);
            for i in 0..total_nodes_usize {
                f.read_exact(&mut buf).unwrap();
                let sc = f32::from_le_bytes(buf);
                scores_old[i] = sc;
                if out_degree[i] == 0 {
                    dangling_sum += sc as f64;
                }
            }
        }
        let dangling_contrib = (DAMPING as f64 * dangling_sum / total_nodes_usize as f64) as f32;

        let mut scores_new = BufWriter::with_capacity(1 << 24, File::create(&write_file).unwrap());
        let mut csr_f = Decoder::new(BufReader::with_capacity(
            1 << 24,
            File::open(csr_edges_path).unwrap(),
        ))
        .unwrap();

        let mut max_delta = 0.0f32;
        let mut buf8 = [0u8; 8];

        println!("{}: Node work: {}", icons::PAGERANK_ICON, total_nodes_usize);
        let mut z = Instant::now();
        let mut cz: u64 = 0;
        let report_every: u64 = 10_000_000;

        for node in 0..total_nodes_usize {
            let old_score = scores_old[node];
            let start = offsets[node];
            let end = offsets[node + 1];
            let num_in = ((end - start) / 8) as usize;
            let mut score = teleport + dangling_contrib;
            for _ in 0..num_in {
                csr_f.read_exact(&mut buf8).unwrap();
                let src = u64::from_le_bytes(buf8) as usize;
                score += DAMPING * scores_old[src] / out_degree[src] as f32;
            }
            let delta = (score - old_score).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            scores_new.write_all(&score.to_le_bytes()).unwrap();
            cz += 1;

            if cz >= report_every {
                cz = 0;
                let done = node + 1;
                let pct = done as f64 / total_nodes_usize as f64 * 100.0;
                let elapsed = z.elapsed().as_secs_f32();
                let rate = report_every as f32 / elapsed;
                let remaining = (total_nodes_usize - done) as f32 / rate;
                println!(
                    "{}: {}/{} ({:.1}%) — {:.0}k nodes/s — ~{:.0}s remaining",
                    icons::PAGERANK_ICON,
                    done,
                    total_nodes_usize,
                    pct,
                    rate / 1000.0,
                    remaining
                );
                z = Instant::now();
            }
        }

        drop(scores_new);

        println!(
            "{}: Iter {:3} — max_delta: {:.2e}  dangling: {:.2e}  ({:.1}s)",
            icons::PAGERANK_ICON,
            iteration,
            max_delta,
            dangling_contrib,
            s.elapsed().as_secs_f64()
        );

        swap(&mut read_file, &mut write_file);
        if max_delta < EPSILON {
            break;
        }
    }

    Ok(read_file)
}
