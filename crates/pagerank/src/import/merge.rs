/*
  Import system libraries
*/
use std::{
    fs::{File, read_to_string, remove_file, rename},
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

/*
  Import own libraries
*/
use crate::compute::{BUFFER_SIZE, IdMap, MERGED_DIR, NODES_DIR, TOTAL_NODES, read_u64_pair};
use prieco_core::ID_MAP_FILE;

/*
  Description: Classical call, split like this so that the tests could call it with custom paths

  Input: paths of shard files
  Output: Total number of nodes after merge
*/
pub fn run(hash_shards: &[String]) -> Result<u64, Box<dyn std::error::Error>> {
    run_with(hash_shards, ID_MAP_FILE, NODES_DIR, MERGED_DIR, TOTAL_NODES)
}

/*
  Description: Merge new hash shards into a globalx ID map

  Input: paths of shard files
  Output: Total number of nodes after merge
*/
pub fn run_with(
    hash_shards: &[String],
    id_map: &str,
    nodes_dir: &str,
    merged_dir: &str,
    total_nodes_path: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut new_node_shards: Vec<String> = Vec::with_capacity(hash_shards.len());

    for (id, sp) in hash_shards.iter().enumerate() {
        let mut hashes: Vec<u64> = Vec::with_capacity(BUFFER_SIZE * 2);
        let mut r = BufReader::with_capacity(1 << 20, File::open(sp)?);
        loop {
            match read_u64_pair(&mut r) {
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

        let out_path = format!("{}/inc_node_shard_{}.bin", nodes_dir, id);
        let mut w = BufWriter::with_capacity(1 << 20, File::create(&out_path)?);
        for h in &hashes {
            w.write_all(&h.to_le_bytes())?;
        }
        new_node_shards.push(out_path);
    }

    let old_node_count: u64 = if Path::new(total_nodes_path).exists() {
        read_to_string(total_nodes_path)?
            .trim()
            .parse()
            .unwrap_or(0)
    } else {
        0
    };

    let merged_id_map = format!("{}/id_map_merged.bin", merged_dir);

    /*
      Merge old and new nodes
    */
    let total_nodes: u64 = {
        struct SR {
            r: BufReader<File>,
            cur: Option<u64>,
        }

        // Open new node shard readers
        let mut new_readers = Vec::with_capacity(new_node_shards.len());
        for p in new_node_shards.iter() {
            let mut r = BufReader::with_capacity(1 << 20, File::open(p)?);
            let cur = read_u64(&mut r);
            new_readers.push(SR { r, cur });
        }

        // Open old ID map via mmap for O(log n) binary search lookups
        let old_map = if Path::new(id_map).exists() {
            Some(IdMap::open(id_map)?)
        } else {
            None
        };

        let mut next_new_id: u64 = old_node_count;

        /*
          K-way merge new node shards.
        */
        let mut new_entries: Vec<(u64, u64)> = Vec::new();
        let mut last_seen: Option<u64> = None;

        loop {
            let new_best = new_readers
                .iter()
                .enumerate()
                .filter_map(|(i, r)| r.cur.map(|h| (i, h)))
                .min_by_key(|&(_, h)| h);

            let (idx, nh) = match new_best {
                Some(x) => x,
                None => break,
            };

            if last_seen != Some(nh) {
                if old_map
                    .as_ref()
                    .and_then(|m: &IdMap| m.lookup(nh))
                    .is_none()
                {
                    new_entries.push((nh, next_new_id));
                    next_new_id += 1;
                }
                last_seen = Some(nh);
            }
            new_readers[idx].cur = read_u64(&mut new_readers[idx].r);
        }

        /*
          Merge-write old mmap pairs + new_entries
        */
        let mut out = BufWriter::with_capacity(1 << 20, File::create(&merged_id_map)?);
        let old_pairs = old_map.as_ref().map(|m: &IdMap| m.pairs()).unwrap_or(&[]);
        let mut oi = 0;
        let mut ni = 0;

        while oi < old_pairs.len() || ni < new_entries.len() {
            let take_old = match (old_pairs.get(oi), new_entries.get(ni)) {
                (Some(&(oh, _)), Some(&(nh, _))) => oh <= nh,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };

            let (h, id) = if take_old {
                let v = old_pairs[oi];
                oi += 1;
                v
            } else {
                let v = new_entries[ni];
                ni += 1;
                v
            };

            out.write_all(&h.to_le_bytes())?;
            out.write_all(&id.to_le_bytes())?;
        }
        out.flush()?;

        next_new_id
    };

    for p in &new_node_shards {
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
fn read_u64(r: &mut BufReader<File>) -> Option<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf).ok().map(|_| u64::from_le_bytes(buf))
}
