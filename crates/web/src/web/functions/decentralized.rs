use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use iroh::{EndpointAddr, SecretKey};
use iroh_gossip::TopicId;
use prieco_core::{
    PRIECO_CONFIG, TANTIVY_INDEX, TANTIVY_READER, file_exists, read_file, url_to_id, write_file,
};
use serde::{Deserialize, Serialize};
use tokio::fs::write;

// Ticket
#[derive(Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub topic: TopicId,
    pub endpoints: Vec<EndpointAddr>,
}

impl Ticket {
    fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

impl Display for Ticket {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD
            .decode(s.to_ascii_uppercase().as_bytes())
            .map_err(|e| e.to_string())?;

        Self::from_bytes(&bytes).map_err(|e| e.to_string())
    }
}

// Secret
const IROH_SECRET_FILE: &str = "config/iroh/iroh_secret.bin";
pub fn get_iroh_secret() -> SecretKey {
    if !file_exists(IROH_SECRET_FILE) {
        return gen_iroh_secret();
    }

    if let Ok(bytes) = std::fs::read(IROH_SECRET_FILE) {
        if bytes.len() == 32 {
            let mut array = [0u8; 32];

            array.copy_from_slice(&bytes);

            return SecretKey::from_bytes(&array);
        }
    }

    gen_iroh_secret()
}

fn gen_iroh_secret() -> SecretKey {
    let secret = SecretKey::generate();

    write_file(IROH_SECRET_FILE, secret.to_bytes(), false);

    secret
}

// Profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    pub node_id: [u8; 32],
    pub top_centroids: Vec<usize>,
    pub rare_keywords: Vec<u64>,
}

pub fn build_node_profile(node_id: [u8; 32]) -> NodeProfile {
    let mut top_centroids: Vec<usize> = std::fs::read_dir(&PRIECO_CONFIG.vector_path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|name| {
            name.strip_prefix("bucket_")?
                .strip_suffix(".bin.zst")?
                .parse::<usize>()
                .ok()
        })
        .collect();

    top_centroids.sort_unstable();

    let mut rare_keywords = Vec::new();
    let schema = TANTIVY_INDEX.schema();

    if let Ok(content_field) = schema.get_field("content") {
        if let Some(reader) = TANTIVY_READER.searcher().segment_readers().first() {
            if let Ok(inv) = reader.inverted_index(content_field) {
                let mut terms: Vec<(String, u64)> = Vec::new();

                if let Ok(mut stream) = inv.terms().stream() {
                    while let Some((bytes, info)) = stream.next() {
                        if let Ok(s) = std::str::from_utf8(bytes) {
                            if info.doc_freq >= 5 && info.doc_freq <= 50 {
                                terms.push((s.to_string(), info.doc_freq as u64));
                            }
                        }
                    }
                }

                terms.sort_by_key(|(_, df)| *df);

                rare_keywords = terms
                    .into_iter()
                    .take(5_000)
                    .map(|(t, _)| url_to_id(&t))
                    .collect();
            }
        }
    }

    rare_keywords.sort_unstable();

    NodeProfile {
        node_id,
        top_centroids,
        rare_keywords,
    }
}
