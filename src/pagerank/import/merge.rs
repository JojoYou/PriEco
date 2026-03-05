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
use crate::pagerank::compute::{
    BUFFER_SIZE, ID_MAP_FILE, MERGED_DIR, NODES_DIR, read_u64_pair_zstd, zstd_reader, zstd_writer,
};

/*
  Description: Classical call, split like this so that the tests could call it with custom paths

  Input: paths of shard files
  Output: Total number of nodes after merge
*/
pub fn run(hash_shards: &[String]) -> Result<u64, Box<dyn std::error::Error>> {
    run_with(hash_shards, ID_MAP_FILE, NODES_DIR, MERGED_DIR)
}

/*
  Description: Merge new hash shards into a global ID map

  Input: paths of shard files
  Output: Total number of nodes after merge
*/
pub fn run_with(
    hash_shards: &[String],
    id_map: &str,
    nodes_dir: &str,
    merged_dir: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut new_hash_shards: Vec<String> = Vec::with_capacity(hash_shards.len());

    /*
      Read ids from shard
    */
    for (id, sp) in hash_shards.iter().enumerate() {
        let mut hashes: Vec<u64> = Vec::with_capacity(BUFFER_SIZE * 2);
        let mut dec = zstd_reader(sp)?;

        loop {
            match read_u64_pair_zstd(&mut dec) {
                Ok(Some((a, b))) => {
                    hashes.push(a);
                    hashes.push(b);
                }
                Ok(None) => break,
                Err(e) => return Err(e),
            }
        }

        hashes.sort_unstable();
        hashes.dedup();

        // Write the nodes to a shard
        let out_path = format!("{}/inc_node_shard_{}.bin.zst", nodes_dir, id);
        let mut enc = zstd_writer(&out_path)?;
        for h in &hashes {
            enc.write_all(&h.to_le_bytes())?;
        }
        enc.finish()?;
        new_hash_shards.push(out_path);
    }

    /*
      K-way merge
    */
    let merged_id_map = format!("{}/id_map_merged.bin.zst", merged_dir);
    let old_node_count: u64 = if Path::new(id_map).exists() {
        let mut max_id: u64 = 0;
        let mut found_any = false;
        let mut dec = zstd_reader(id_map)?;
        let mut buf = [0u8; 16];
        while dec.read_exact(&mut buf).is_ok() {
            let id = u64::from_le_bytes(buf[8..16].try_into()?);
            if id >= max_id {
                max_id = id;
                found_any = true;
            }
        }
        if found_any { max_id + 1 } else { 0 }
    } else {
        0
    };

    /*
      Merge old and new nodes
    */
    let total_nodes: u64 = {
        // Shard reader and it's current hash
        struct SR<'a> {
            dec: Decoder<'a, BufReader<File>>,
            cur: Option<u64>,
        }

        // Initialize readers
        let mut new_readers = Vec::with_capacity(new_hash_shards.len());
        for p in new_hash_shards.iter() {
            let mut dec = zstd_reader(p.as_str())?;
            let cur = read_u64_zstd(&mut dec);
            new_readers.push(SR { dec, cur });
        }

        // Initialize old ID map reader
        let mut old_dec: Option<Decoder<BufReader<File>>> = if Path::new(id_map).exists() {
            Some(zstd_reader(id_map)?)
        } else {
            None
        };
        let mut old_buf = [0u8; 16];
        let mut old_valid = false;
        let mut old_hash: u64 = 0;
        let mut old_id: u64 = 0;
        let adv_old = |dec: &mut Decoder<BufReader<File>>,
                       buf: &mut [u8; 16],
                       h: &mut u64,
                       id: &mut u64,
                       v: &mut bool| {
            *v = dec.read_exact(buf).is_ok();
            if *v {
                *h = u64::from_le_bytes(buf[0..8].try_into().unwrap());
                *id = u64::from_le_bytes(buf[8..16].try_into().unwrap());
            }
        };
        if let Some(ref mut dec) = old_dec {
            adv_old(
                dec,
                &mut old_buf,
                &mut old_hash,
                &mut old_id,
                &mut old_valid,
            );
        }

        let mut out = zstd_writer(&merged_id_map)?;
        let mut next_new_id: u64 = old_node_count;
        let mut last_written: Option<u64> = None;

        // K-way merge
        loop {
            // Find smallest hash among new shards
            let new_best = new_readers
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.cur.map(|h| (i, h)))
                .min_by_key(|&(_, h)| h);

            match (old_valid, new_best) {
                // Both old and new exhausted
                (false, None) => break,

                // Only old remains
                (true, None) => {
                    if last_written != Some(old_hash) {
                        out.write_all(&old_hash.to_le_bytes()).unwrap();
                        out.write_all(&old_id.to_le_bytes()).unwrap();
                        last_written = Some(old_hash);
                    }
                    if let Some(ref mut dec) = old_dec {
                        adv_old(
                            dec,
                            &mut old_buf,
                            &mut old_hash,
                            &mut old_id,
                            &mut old_valid,
                        );
                    }
                }

                // Only new remains
                (false, Some((idx, nh))) => {
                    if last_written != Some(nh) {
                        out.write_all(&nh.to_le_bytes()).unwrap();
                        out.write_all(&next_new_id.to_le_bytes()).unwrap();
                        next_new_id += 1;
                        last_written = Some(nh);
                    }
                    new_readers[idx].cur = read_u64_zstd(&mut new_readers[idx].dec);
                }

                // Both old and new remain
                (true, Some((idx, nh))) => {
                    if old_hash <= nh {
                        if last_written != Some(old_hash) {
                            out.write_all(&old_hash.to_le_bytes()).unwrap();
                            out.write_all(&old_id.to_le_bytes()).unwrap();
                            last_written = Some(old_hash);
                        }
                        if old_hash == nh {
                            new_readers[idx].cur = read_u64_zstd(&mut new_readers[idx].dec);
                        }
                        if let Some(ref mut dec) = old_dec {
                            adv_old(
                                dec,
                                &mut old_buf,
                                &mut old_hash,
                                &mut old_id,
                                &mut old_valid,
                            );
                        }
                    } else {
                        // new hash is smaller — assign fresh id
                        if last_written != Some(nh) {
                            out.write_all(&nh.to_le_bytes()).unwrap();
                            out.write_all(&next_new_id.to_le_bytes()).unwrap();
                            next_new_id += 1;
                            last_written = Some(nh);
                        }
                        new_readers[idx].cur = read_u64_zstd(&mut new_readers[idx].dec);
                    }
                }
            }
        }

        out.finish()?;
        next_new_id
    };

    for p in &new_hash_shards {
        let _ = remove_file(p);
    }

    rename(&merged_id_map, id_map)?;

    Ok(total_nodes)
}

/* Helper functions */
/*
  Description: Read a single u64 from a Zstd-decoded file

  Input: Zstd decoder
  Output: Option id
*/
fn read_u64_zstd(r: &mut Decoder<BufReader<File>>) -> Option<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf).ok().map(|_| u64::from_le_bytes(buf))
}
