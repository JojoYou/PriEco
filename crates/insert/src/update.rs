use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    hash::{BuildHasher, Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use ahash::RandomState;
use serde::{Deserialize, Serialize};

use prieco_core::{
    META_DECODER, META_DICTIONARY, PRIECO_BLOBS, PRIECO_META, SEARCHED_K1, SEARCHED_K2,
    TANTIVY_INDEX, TANTIVY_WRITER, WebDocument, icons, url_to_domain_id,
};
use tantivy::Term;

// State
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UpdateState {
    pub finished_ranges: HashMap<String, Vec<Range>>,
    pub absolute_ceilings: HashMap<String, u64>,
}

impl UpdateState {
    pub fn load(path: &Path) -> Self {
        read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = write(path, data);
        }
    }

    pub fn add(&mut self, id: &str, start: u64, end: u64) {
        let ranges = self.finished_ranges.entry(id.to_string()).or_default();
        ranges.push(Range { start, end });
        ranges.sort_by_key(|r| r.start);

        let mut merged: Vec<Range> = Vec::new();
        for current in ranges.drain(..) {
            if let Some(last) = merged.last_mut() {
                if current.start <= last.end + 1 {
                    last.end = last.end.max(current.end);
                    continue;
                }
            }
            merged.push(current);
        }

        *ranges = merged;
    }

    pub fn processed(&self, id: &str, doc_id: u64) -> bool {
        self.finished_ranges.get(id).map_or(false, |ranges| {
            ranges.iter().any(|r| doc_id >= r.start && doc_id <= r.end)
        })
    }

    pub fn finished(&self, id: &str) -> bool {
        let target = match self.absolute_ceilings.get(id) {
            Some(&t) => t,
            None => return false,
        };

        if let Some(ranges) = self.finished_ranges.get(id) {
            if let Some(first_range) = ranges.first() {
                return first_range.start == 0 && first_range.end >= target;
            }
        }
        false
    }

    pub fn unprocessed_id(&self, update_id: &str) -> u64 {
        if let Some(ranges) = self.finished_ranges.get(update_id) {
            if let Some(first_range) = ranges.first() {
                if first_range.start == 0 {
                    return first_range.end + 1;
                }
            }
        }

        0
    }

    pub fn processed_range_end(&self, id: &str, doc_id: u64) -> Option<u64> {
        self.finished_ranges.get(id).and_then(|ranges| {
            for r in ranges {
                if doc_id >= r.start && doc_id <= r.end {
                    return Some(r.end);
                }
            }
            None
        })
    }
}

// Engine
pub enum UpdateAction {
    Unchanged,
    Modified,
    Deleted,
}

pub trait Update: Send + Sync {
    fn id(&self) -> &'static str;
    fn apply(&self, doc: &mut WebDocument) -> UpdateAction;
}

pub struct UpdateEngine {
    updates: Vec<Box<dyn Update>>,
    state_file: PathBuf,
}

impl UpdateEngine {
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            state_file: PathBuf::from("config/index_updates.json"),
        }
    }

    pub fn add(&mut self, update: Box<dyn Update>) {
        self.updates.push(update);
    }

    pub fn run(&self) {
        // No updates to process
        if self.updates.is_empty() {
            return;
        }

        // Load states of updates
        let mut state = UpdateState::load(&self.state_file);
        let meta_ks = &PRIECO_META.meta_ks;

        // Get maximum id the updater needs to update to
        // Prevents newly inserted items from being updated
        // as they already follow the new rule
        let current_max_db_id = meta_ks
            .iter()
            .next_back()
            .and_then(|guard| guard.into_inner().ok())
            .map(|(k, _)| u64::from_be_bytes(k[0..8].try_into().unwrap()))
            .unwrap_or(0);
        println!(
            "{}: Found max DB ID: {}",
            icons::INDEX_UPDATER,
            current_max_db_id
        );
        let mut state_changed = false;
        for update in &self.updates {
            if !state.absolute_ceilings.contains_key(update.id()) {
                state
                    .absolute_ceilings
                    .insert(update.id().to_string(), current_max_db_id);
                state_changed = true;
            }
        }
        if state_changed {
            state.save(&self.state_file);
        }

        // Select the oldest unfinished updater to take the cursor
        // Only the oldest unfinished updater has the cursor
        // While this one runs the other apply their filters on items too
        // Saving disk I/O
        // As oldest hasn't updated certain items, the newer updaters haven't updated them too
        let driving_update = match self
            .updates
            .iter()
            .filter(|u| !state.finished(u.id()))
            .max_by_key(|u| state.unprocessed_id(u.id()))
        {
            Some(u) => u,
            None => {
                println!("{}: No pending updates found.", icons::INDEX_UPDATER);
                return;
            }
        };

        // Max item the updater is allowed to update
        // Newer items than the updater already follow new rules
        // thus don't need to be updated
        let absolute_ceiling_item = *state.absolute_ceilings.get(driving_update.id()).unwrap();

        // Next item the updater must update
        let mut current_item = state.unprocessed_id(driving_update.id());
        println!(
            "{}: Starting cursor at {} with target ceiling {}",
            icons::INDEX_UPDATER,
            current_item,
            absolute_ceiling_item
        );

        // Process from next item [current_item] to [absolute_ceiling_item]
        let mut batch_modified = Vec::with_capacity(1_000);
        let mut batch_deleted: Vec<(u64, String)> = Vec::with_capacity(1_000);
        let mut batch_start_id = None;
        let mut items_scanned = 0;
        while current_item <= absolute_ceiling_item {
            // If current id is in already processed range, skip the range
            if let Some(range_end) = state.processed_range_end(driving_update.id(), current_item) {
                current_item = range_end + 1;
                continue;
            }

            // Get items
            let mut iter = meta_ks.range(current_item.to_be_bytes()..);
            let mut processed_in_iter = 0;
            while let Some(guard) = iter.next() {
                let (key, value) = match guard.into_inner() {
                    Ok(kv) => kv,
                    Err(e) => {
                        println!(
                            "{}: Failed to read DB key/value! Error: {}",
                            icons::INDEX_UPDATER,
                            e
                        );

                        continue;
                    }
                };
                let doc_id = u64::from_be_bytes(key[0..8].try_into().unwrap());

                processed_in_iter += 1;

                // Check for processed range and end of ceiling
                // Without this we would be reading every item
                // In front of us may be ranges that were already processed by
                // older (already finished) updater
                if state
                    .processed_range_end(driving_update.id(), doc_id)
                    .is_some()
                    || doc_id > absolute_ceiling_item
                {
                    current_item = doc_id;
                    break;
                }

                // Keep progress on disk
                // This is "counter"
                if batch_start_id.is_none() {
                    batch_start_id = Some(doc_id);
                }
                current_item = doc_id;
                items_scanned += 1;

                // Decode item and serialize it to [WebDocument]
                let decoded_result = if let Some(dict) = META_DECODER.as_ref() {
                    zstd::stream::Decoder::with_prepared_dictionary(value.as_ref(), dict).and_then(
                        |mut decoder| {
                            let mut buf = Vec::new();
                            decoder.read_to_end(&mut buf)?;
                            Ok(buf)
                        },
                    )
                } else {
                    zstd::decode_all(value.as_ref())
                };

                let mut final_document: Option<WebDocument> = None;
                match decoded_result {
                    Ok(decompressed) => {
                        match serde_json::from_slice::<WebDocument>(&decompressed) {
                            Ok(d) => final_document = Some(d),
                            Err(e) => {
                                println!(
                                    "{}: JSON decode failed for doc_id {}: {}",
                                    icons::INDEX_UPDATER,
                                    doc_id,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "{}: ZSTD decode failed for doc_id {}: {}",
                            icons::INDEX_UPDATER,
                            doc_id,
                            e
                        );
                    }
                }

                if let Some(mut document) = final_document {
                    println!(
                        "{}: Successfully decoded doc_id {}. Checking updates...",
                        icons::INDEX_UPDATER,
                        doc_id
                    );

                    // Check update viability
                    let mut save_to_disk = false; // Update changed the item
                    let mut tombstoned = false; // Mark for deletion
                    for update in &self.updates {
                        let update_max_ceiling: u64 = state
                            .absolute_ceilings
                            .get(update.id())
                            .copied()
                            .unwrap_or(0);
                        let processed_item: bool = state.processed(update.id(), doc_id);

                        println!(
                            "   -> Update {}: u_target={}, doc_id={}, processed={}",
                            update.id(),
                            update_max_ceiling,
                            doc_id,
                            processed_item
                        );

                        // Apply updates
                        if !processed_item && doc_id <= update_max_ceiling {
                            match update.apply(&mut document) {
                                UpdateAction::Modified => save_to_disk = true,
                                UpdateAction::Deleted => {
                                    tombstoned = true;
                                    break;
                                }
                                UpdateAction::Unchanged => {}
                            }
                        }
                    }

                    if tombstoned {
                        batch_deleted.push((doc_id, document.url.clone()));
                    } else if save_to_disk {
                        batch_modified.push((doc_id, document));
                    }
                }

                if items_scanned >= 1000 {
                    if !batch_modified.is_empty() || !batch_deleted.is_empty() {
                        self.flush_batch(&batch_modified, &batch_deleted);
                        batch_modified.clear();
                        batch_deleted.clear();
                    }
                    self.update_state(&mut state, batch_start_id.unwrap(), doc_id);
                    batch_start_id = None;
                    items_scanned = 0;
                }
            }

            if processed_in_iter == 0 {
                break;
            }

            current_item += 1;
        }

        if items_scanned > 0 {
            if !batch_modified.is_empty() || !batch_deleted.is_empty() {
                self.flush_batch(&batch_modified, &batch_deleted);
            }
            self.update_state(&mut state, batch_start_id.unwrap(), current_item - 1);
        }
    }

    fn update_state(&self, state: &mut UpdateState, start: u64, end: u64) {
        for update in &self.updates {
            state.add(update.id(), start, end);
        }
        state.save(&self.state_file);
    }

    fn flush_batch(&self, modified: &[(u64, WebDocument)], deleted: &[(u64, String)]) {
        let schema = TANTIVY_INDEX.schema();
        let doc_id_field = schema.get_field("doc_id").unwrap();
        let domain_id_field = schema.get_field("domain_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let description_field = schema.get_field("description").unwrap();
        let content_field = schema.get_field("content").unwrap();
        let keywords_field = schema.get_field("keywords").unwrap();
        let lang_field = schema.get_field("lang").unwrap();
        let loc_field = schema.get_field("loc").unwrap();
        let date_field = schema.get_field("date").unwrap();
        let safe_s_field = schema.get_field("safe_s").unwrap();
        let intent_field = schema.get_field("intent").unwrap();

        let tantivy_writer = TANTIVY_WRITER.lock();

        // Update
        for (id, doc) in modified {
            let doc_bytes = serde_json::to_vec(doc).unwrap();
            let compressed = if let Some(dict_bytes) = META_DICTIONARY.as_ref() {
                let mut encoder =
                    zstd::stream::Encoder::with_dictionary(Vec::new(), 3, dict_bytes).unwrap();
                encoder.write_all(&doc_bytes).unwrap();
                encoder.finish().unwrap()
            } else {
                zstd::encode_all(doc_bytes.as_slice(), 3).unwrap()
            };

            // Meta
            let _ = PRIECO_META.meta_ks.insert(&id.to_be_bytes(), &compressed);

            // Tantivy
            tantivy_writer.delete_term(Term::from_field_u64(doc_id_field, *id));
            let _ = tantivy_writer.add_document(tantivy::doc!(
                doc_id_field => *id,
                domain_id_field => url_to_domain_id(&doc.url),
                title_field => doc.title.clone(),
                description_field => doc.description.clone(),
                content_field => doc.content.clone(),
                keywords_field => doc.keywords.clone(),
                lang_field => doc.lang.clone(),
                loc_field => doc.loc.clone(),
                date_field => doc.date,
                safe_s_field => doc.safe_s,
                intent_field => doc.intent as u64
            ));
        }

        for (id, url) in deleted {
            let meta_key = id.to_be_bytes();
            let _ = PRIECO_META.meta_ks.remove(&meta_key);

            let build_hasher = RandomState::with_seeds(SEARCHED_K1, SEARCHED_K2, 0, 0);
            let mut hasher = build_hasher.build_hasher();
            url.hash(&mut hasher);
            let blob_id: u64 = hasher.finish();

            let blob_key = blob_id.to_le_bytes();
            let _ = PRIECO_BLOBS.blobs_ks.remove(&blob_key);

            tantivy_writer.delete_term(Term::from_field_u64(doc_id_field, *id));
        }
    }
}
