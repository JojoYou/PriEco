/*
  Import system libraries
*/
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    time::Instant,
};

/*
  Import own libraries
*/
use crate::{
    globals::icons,
    pagerank::compute::{CSR_EDGES, CSR_OFFSETS, OUT_DEGREE, SCORES_A, SCORES_B, zstd_reader},
};

/*
  Constants
*/
const DAMPING: f32 = 0.85;
const EPSILON: f32 = 1e-6;
const MAX_ITER: usize = 100;
const AVAILABLE_RAM: usize = 6 * 1024 * 1024 * 1024;

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

/*
  Description: Calculates pagerank scores from the graph

  Input: Count of the nodes
  Output: Final score file path
*/
pub fn run_with(
    total_nodes_usize: usize,
    csr_offsets_path: &str,
    csr_edges_path: &str,
    out_degree_path: &str,
    scores_a_path: &str,
    scores_b_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let teleport = (1.0 - DAMPING) / total_nodes_usize as f32;

    // Does data fit into RAM?
    let od_bytes = total_nodes_usize as u64 * 4;
    let off_bytes = (total_nodes_usize as u64 + 1) * 8;
    let meta_bytes = od_bytes + off_bytes;
    let fits_in_ram = meta_bytes <= AVAILABLE_RAM as u64;

    println!("{}: Fits into ram: {}", icons::PAGERANK, fits_in_ram);

    let out_degree_ram: Option<Vec<u32>> = if fits_in_ram {
        let mut v = vec![0u32; total_nodes_usize];
        let mut dec = zstd_reader(out_degree_path)?;
        let mut buf = [0u8; 4];
        for i in 0..total_nodes_usize {
            dec.read_exact(&mut buf).unwrap();
            v[i] = u32::from_le_bytes(buf);
        }
        Some(v)
    } else {
        None
    };

    let offsets_ram: Option<Vec<u64>> = if fits_in_ram {
        let mut v = vec![0u64; total_nodes_usize + 1];
        let mut dec = zstd_reader(csr_offsets_path)?;
        let mut buf = [0u8; 8];
        for i in 0..=total_nodes_usize {
            dec.read_exact(&mut buf).unwrap();
            v[i] = u64::from_le_bytes(buf);
        }
        Some(v)
    } else {
        None
    };

    let mut read_file = scores_a_path.to_string();
    let mut write_file = scores_b_path.to_string();

    for iteration in 0..MAX_ITER {
        let s = Instant::now();

        let od_iter_storage: Vec<u32>;
        let off_iter_storage: Vec<u64>;
        let out_degree: &[u32] = if let Some(ref v) = out_degree_ram {
            v
        } else {
            od_iter_storage = {
                let mut v = vec![0u32; total_nodes_usize];
                let mut dec = zstd_reader(out_degree_path)?;
                let mut buf = [0u8; 4];
                for i in 0..total_nodes_usize {
                    dec.read_exact(&mut buf).unwrap();
                    v[i] = u32::from_le_bytes(buf);
                }
                v
            };
            &od_iter_storage
        };
        let offsets: &[u64] = if let Some(ref v) = offsets_ram {
            v
        } else {
            off_iter_storage = {
                let mut v = vec![0u64; total_nodes_usize + 1];
                let mut dec = zstd_reader(csr_offsets_path)?;
                let mut buf = [0u8; 8];
                for i in 0..=total_nodes_usize {
                    dec.read_exact(&mut buf).unwrap();
                    v[i] = u64::from_le_bytes(buf);
                }
                v
            };
            &off_iter_storage
        };

        // Pass 1: dangling mass
        let mut dangling_sum = 0.0f64;
        {
            let mut f = BufReader::with_capacity(1 << 20, File::open(&read_file).unwrap());
            let mut buf = [0u8; 4];
            for i in 0..total_nodes_usize {
                f.read_exact(&mut buf).unwrap();
                if out_degree[i] == 0 {
                    dangling_sum += f32::from_le_bytes(buf) as f64;
                }
            }
        }
        let dangling_contrib = (DAMPING as f64 * dangling_sum / total_nodes_usize as f64) as f32;

        // Pass 2: new scores
        let mut scores_old = File::open(&read_file).unwrap();
        let mut scores_new = BufWriter::with_capacity(1 << 20, File::create(&write_file).unwrap());
        let mut csr_f = BufReader::with_capacity(1 << 20, File::open(csr_edges_path).unwrap());
        let mut old_scores_seq = BufReader::with_capacity(1 << 20, File::open(&read_file).unwrap());

        let mut max_delta = 0.0f32;
        let mut csr_pos: u64 = 0;
        let mut buf4 = [0u8; 4];
        let mut buf8 = [0u8; 8];

        for node in 0..total_nodes_usize {
            old_scores_seq.read_exact(&mut buf4).unwrap();
            let old_score = f32::from_le_bytes(buf4);

            let start = offsets[node];
            let end = offsets[node + 1];
            let num_in = ((end - start) / 8) as usize;

            if start != csr_pos {
                drop(csr_f);
                let mut f = File::open(csr_edges_path).unwrap();
                f.seek(SeekFrom::Start(start)).unwrap();
                csr_f = BufReader::with_capacity(1 << 16, f);
                csr_pos = start;
            }

            let mut score = teleport + dangling_contrib;
            for _ in 0..num_in {
                csr_f.read_exact(&mut buf8).unwrap();
                csr_pos += 8;
                let src = u64::from_le_bytes(buf8) as usize;
                scores_old.seek(SeekFrom::Start(src as u64 * 4)).unwrap();
                scores_old.read_exact(&mut buf4).unwrap();
                let src_score = f32::from_le_bytes(buf4);
                let src_od = out_degree[src] as f32;
                score += DAMPING * src_score / src_od;
            }

            let delta = (score - old_score).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            scores_new.write_all(&score.to_le_bytes()).unwrap();
        }

        drop(scores_new);

        println!(
            "  Iter {:3} — max_delta: {:.2e}  dangling: {:.2e}  ({:.1}s)",
            iteration,
            max_delta,
            dangling_contrib,
            s.elapsed().as_secs_f64()
        );

        std::mem::swap(&mut read_file, &mut write_file);
        if max_delta < EPSILON {
            break;
        }
    }

    Ok(read_file)
}
