use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use prieco_core::{
    META_DECODER, META_DICTIONARY, PRIECO_BLOBS, PRIECO_META, TANTIVY_INDEX, TANTIVY_WRITER,
    WebDocument, url_to_domain_id,
};

// State
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UpdateState {
    pub trackers: HashMap<String, Vec<Range>>,
    pub targets: HashMap<String, u64>,
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
        let ranges = self.trackers.entry(id.to_string()).or_default();
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
        self.trackers.get(id).map_or(false, |ranges| {
            ranges.iter().any(|r| doc_id >= r.start && doc_id <= r.end)
        })
    }

    pub fn finished(&self, id: &str) -> bool {
        let target = match self.targets.get(id) {
            Some(&t) => t,
            None => return false,
        };

        if let Some(ranges) = self.trackers.get(id) {
            if let Some(first_range) = ranges.first() {
                return first_range.start == 0 && first_range.end >= target;
            }
        }
        false
    }

    pub fn unprocessed_id(&self, update_id: &str) -> u64 {
        if let Some(ranges) = self.trackers.get(update_id) {
            if let Some(first_range) = ranges.first() {
                if first_range.start == 0 {
                    return first_range.end + 1;
                }
            }
        }

        0
    }

    pub fn processed_range_end(&self, id: &str, doc_id: u64) -> Option<u64> {
        self.trackers.get(id).and_then(|ranges| {
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
        if self.updates.is_empty() {
            return;
        }

        let mut state = UpdateState::load(&self.state_file);
        let meta_ks = &PRIECO_META.meta_ks;

        let current_max_db_id = meta_ks
            .iter()
            .next_back()
            .and_then(|guard| guard.into_inner().ok())
            .map(|(k, _)| u64::from_be_bytes(k[0..8].try_into().unwrap()))
            .unwrap_or(0);

        println!("DEBUG: Found max DB ID: {}", current_max_db_id);

        // Get maximum id the updater needs to update to
        // Prevents newly inserted items from being updated
        // as they already follow the new rule
        let mut state_changed = false;
        for update in &self.updates {
            if !state.targets.contains_key(update.id()) {
                state
                    .targets
                    .insert(update.id().to_string(), current_max_db_id);
                state_changed = true;
            }
        }
        if state_changed {
            state.save(&self.state_file);
        }

        let driving_update = match self
            .updates
            .iter()
            .filter(|u| !state.finished(u.id()))
            .max_by_key(|u| state.unprocessed_id(u.id()))
        {
            Some(u) => u,
            None => {
                println!("DEBUG: No pending updates found.");
                return;
            }
        };

        let target_ceiling = *state.targets.get(driving_update.id()).unwrap();
        let mut current_pos = state.unprocessed_id(driving_update.id());

        println!(
            "DEBUG: Starting cursor at {} with target ceiling {}",
            current_pos, target_ceiling
        );

        let mut batch_modified = Vec::new();
        let mut batch_deleted = Vec::new();
        let mut batch_start_id = None;
        let mut items_scanned = 0;

        while current_pos <= target_ceiling {
            if let Some(range_end) = state.processed_range_end(driving_update.id(), current_pos) {
                current_pos = range_end + 1;
                continue;
            }

            let mut iter = meta_ks.range(current_pos.to_be_bytes()..);
            let mut processed_in_iter = 0;

            while let Some(guard) = iter.next() {
                let (key, value) = match guard.into_inner() {
                    Ok(kv) => kv,
                    Err(_) => continue,
                };

                let doc_id = u64::from_be_bytes(key[0..8].try_into().unwrap());
                processed_in_iter += 1;

                if state
                    .processed_range_end(driving_update.id(), doc_id)
                    .is_some()
                {
                    current_pos = doc_id;
                    break;
                }

                if doc_id > target_ceiling {
                    current_pos = doc_id;
                    break;
                }

                if batch_start_id.is_none() {
                    batch_start_id = Some(doc_id);
                }
                current_pos = doc_id;
                items_scanned += 1;

                let decode_result = if let Some(dict) = META_DECODER.as_ref() {
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

                let mut doc = None;
                match decode_result {
                    Ok(decompressed) => {
                        match serde_json::from_slice::<WebDocument>(&decompressed) {
                            Ok(d) => doc = Some(d),
                            Err(e) => {
                                if items_scanned <= 5 {
                                    println!(
                                        "❌ DEBUG: JSON decode failed for doc_id {}: {}",
                                        doc_id, e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if items_scanned <= 5 {
                            println!("❌ DEBUG: ZSTD decode failed for doc_id {}: {}", doc_id, e);
                        }
                    }
                }

                if let Some(mut document) = doc {
                    if items_scanned <= 5 {
                        println!(
                            "✅ DEBUG: Successfully decoded doc_id {}. Checking updates...",
                            doc_id
                        );
                    }

                    let mut save = false;
                    let mut tombstoned = false;

                    for update in &self.updates {
                        let u_target = state.targets.get(update.id()).copied().unwrap_or(0);
                        let is_processed = state.processed(update.id(), doc_id);

                        if items_scanned <= 5 {
                            println!(
                                "   -> Update {}: u_target={}, doc_id={}, processed={}",
                                update.id(),
                                u_target,
                                doc_id,
                                is_processed
                            );
                        }

                        if doc_id <= u_target && !is_processed {
                            match update.apply(&mut document) {
                                UpdateAction::Modified => save = true,
                                UpdateAction::Deleted => {
                                    tombstoned = true;
                                    break;
                                }
                                UpdateAction::Unchanged => {}
                            }
                        }
                    }

                    if tombstoned {
                        batch_deleted.push(doc_id);
                    } else if save {
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

            current_pos += 1;
        }

        if items_scanned > 0 {
            if !batch_modified.is_empty() || !batch_deleted.is_empty() {
                self.flush_batch(&batch_modified, &batch_deleted);
            }
            self.update_state(&mut state, batch_start_id.unwrap(), current_pos - 1);
        }

        if !batch_modified.is_empty() || !batch_deleted.is_empty() {
            self.flush_batch(&batch_modified, &batch_deleted);
            self.update_state(&mut state, batch_start_id.unwrap(), current_pos - 1);
        }
    }

    fn update_state(&self, state: &mut UpdateState, start: u64, end: u64) {
        for update in &self.updates {
            state.add(update.id(), start, end);
        }
        state.save(&self.state_file);
    }

    fn flush_batch(&self, modified: &[(u64, WebDocument)], deleted: &[u64]) {
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

            let _ = PRIECO_META.meta_ks.insert(&id.to_be_bytes(), &compressed);

            tantivy_writer.delete_term(tantivy::Term::from_field_u64(doc_id_field, *id));
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

        for id in deleted {
            let key = id.to_be_bytes();
            let _ = PRIECO_META.meta_ks.remove(&key);
            let _ = PRIECO_BLOBS.blobs_ks.remove(&key);

            tantivy_writer.delete_term(tantivy::Term::from_field_u64(doc_id_field, *id));
        }
    }
}
