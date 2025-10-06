use ahash::{AHashMap, AHashSet};
use ndarray::{Array, CowArray, IxDyn};
use once_cell::sync::Lazy;
use ort::{InMemorySession, tensor::OrtOwnedTensor};
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::Serialize;
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    path::Path,
    str::FromStr,
    sync::Arc,
};
use tokenizers::Tokenizer;
use tokio::task;

pub const CSS_VERSION: &str = "0.1.1";
pub const JS_VERSION: &str = "0.1.1";

// Constants for all possible terminal colors
pub mod colors {
    pub const RESET: &str = "\x1b[0m"; // Reset to default color, must be used after every print message
    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
}

#[derive(Serialize)]
pub struct SearchResult {
    pub url: String,
    pub display_url: String,
    pub domain: String,

    pub title: String,
    pub description: String,

    pub image: String,
    pub favicon: String,
}

#[derive(Serialize, Clone)]
pub struct ImgResult {
    pub thumbnail: String,
    pub image: String,

    pub title: String,

    pub site_url: String,
    pub site_domain: String,
    pub favicon: String,
}

#[derive(Serialize)]
pub struct WebScrollResult {
    pub url: String,
    pub domain: String,
    pub image: String,
    pub favicon: String,
    pub title: String,
    pub price: String,
}

pub static VECTOR_EMBEDDING_TOKENIZER: &[u8] = include_bytes!("data/tokenizer.json");
pub static VECTOR_EMBEDDING_MODEL: &[u8] = include_bytes!("data/model_int8.onnx");
#[derive(Clone)]
pub struct EmbeddingService {
    pub tokenizer: Arc<tokio::sync::Mutex<Tokenizer>>,
    pub model: Arc<tokio::sync::Mutex<InMemorySession<'static>>>,
}

impl EmbeddingService {
    pub async fn embed_query(
        &self,
        query: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let query = query.to_string();
        let tokenizer = self.tokenizer.clone();
        let model = self.model.clone();

        // Move the actual embedding work to a blocking task
        let embeddings = task::spawn_blocking(
            move || -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
                // Tokenize the query (single item, not batch)
                let tokenizer_guard = tokio::runtime::Handle::current().block_on(tokenizer.lock());

                let encoding = tokenizer_guard
                    .encode(query, true)
                    .map_err(|e| format!("Tokenization failed: {}", e))?;

                // Get token data (similar to your batch logic but for single query)
                let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
                let attention_mask: Vec<i64> = encoding
                    .get_attention_mask()
                    .iter()
                    .map(|&v| v as i64)
                    .collect();

                // Create arrays with shape [1, sequence_length] for single query
                let shape = [1, token_ids.len()];
                let cow_input_ids = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), token_ids)
                        .map_err(|e| format!("Failed to create input_ids array: {}", e))?,
                );
                let cow_attention_masks = CowArray::from(
                    Array::from_shape_vec(IxDyn(&shape), attention_mask)
                        .map_err(|e| format!("Failed to create attention_mask array: {}", e))?,
                );

                // Get model lock and create input tensors (matching your working code)
                let embedder = tokio::runtime::Handle::current().block_on(model.lock());

                let input_tensor_ids = ort::Value::from_array(embedder.allocator(), &cow_input_ids)
                    .map_err(|e| format!("Failed to create input tensor: {}", e))?;
                let input_tensor_attention_mask =
                    ort::Value::from_array(embedder.allocator(), &cow_attention_masks)
                        .map_err(|e| format!("Failed to create attention mask tensor: {}", e))?;

                // Run inference (exactly like your batch code)
                let outputs: Vec<ort::Value<'static>> = embedder
                    .run(vec![input_tensor_ids, input_tensor_attention_mask])
                    .map_err(|e| format!("Model inference failed: {}", e))?;

                // Extract embeddings (adapted from your batch logic)
                let tensor: OrtOwnedTensor<f32, _> = outputs[0]
                    .try_extract()
                    .map_err(|e| format!("Failed to extract output tensor: {}", e))?;

                // Since we have only one query, take the first (and only) row
                // and limit to 384 dimensions like your batch code
                let binding = tensor.view();
                let embed_row = binding
                    .outer_iter()
                    .next()
                    .ok_or("No embedding row found")?;
                let full_embed: Vec<f32> = embed_row.iter().copied().collect();
                let embedding = full_embed[..384.min(full_embed.len())].to_vec();

                Ok(embedding)
            },
        )
        .await?;

        embeddings
    }
}

pub static TOP_DOMAINS: Lazy<AHashSet<&'static str>> =
    Lazy::new(|| include_str!("data/domains.txt").lines().collect());

pub static PAGERANK: Lazy<Arc<Database>> =
    Lazy::new(|| Arc::new(Database::create(Path::new("kv/pageranks.redb")).unwrap()));
pub const PAGERANKS_TABLE: TableDefinition<&str, f64> = TableDefinition::new("pageranks");

pub static IP_TO_LOC: Lazy<Arc<IpGeoDatabase>> =
    Lazy::new(|| Arc::new(IpGeoDatabase::open("kv/ip.redb").unwrap()));
const IP_RANGES: TableDefinition<u128, (u128, String)> = TableDefinition::new("ip_ranges");

pub struct IpGeoDatabase {
    db: Database,
}
impl IpGeoDatabase {
    pub fn open(db_path: &str) -> Result<Self, redb::Error> {
        let db = Database::create(db_path)?;
        Ok(Self { db })
    }

    pub fn lookup_country(
        &self,
        ip_str: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let ip_num = self.parse_ip_to_u128(ip_str)?;

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(IP_RANGES)?;

        // Find the range that contains this IP
        // We need to find the largest start IP that is <= our target IP
        let mut range_iter = table.range(..=ip_num)?;

        // Get the last (highest) start IP that's <= our target
        if let Some(result) = range_iter.next_back() {
            let (start_ip, value) = result?;
            let (end_ip, country_code) = value.value();

            // Check if our IP falls within this range
            if ip_num >= start_ip.value() && ip_num <= end_ip {
                return Ok(Some(country_code.clone()));
            }
        }

        Ok(None)
    }

    fn parse_ip_to_u128(&self, ip_str: &str) -> Result<u128, Box<dyn std::error::Error>> {
        // Try IPv4 first
        if let Ok(ipv4) = Ipv4Addr::from_str(ip_str) {
            let octets = ipv4.octets();
            let ip_u32 = ((octets[0] as u32) << 24)
                | ((octets[1] as u32) << 16)
                | ((octets[2] as u32) << 8)
                | (octets[3] as u32);
            return Ok(ip_u32 as u128);
        }

        // Try IPv6
        if let Ok(ipv6) = Ipv6Addr::from_str(ip_str) {
            let segments = ipv6.segments();
            let mut ip_u128 = 0u128;
            for (i, segment) in segments.iter().enumerate() {
                ip_u128 |= (*segment as u128) << (112 - (i * 16));
            }
            return Ok(ip_u128);
        }

        Err(format!("Invalid IP address: {}", ip_str).into())
    }
}

pub static COUNTRY_TO_LANG: Lazy<Arc<AHashMap<&'static str, &'static str>>> = Lazy::new(|| {
    let mut map = AHashMap::with_capacity(200);

    // === Direct mappings ===
    map.insert("de", "de");
    map.insert("at", "de");
    map.insert("ch", "de");
    map.insert("fr", "fr");
    map.insert("es", "es");
    map.insert("mx", "es");
    map.insert("ar", "es");
    map.insert("cl", "es");
    map.insert("co", "es");
    map.insert("pe", "es");
    map.insert("pt", "pt");
    map.insert("br", "pt");
    map.insert("us", "en");
    map.insert("gb", "en");
    map.insert("au", "en");
    map.insert("nz", "en");
    map.insert("ie", "en");
    map.insert("ca", "all"); // fallback to All
    map.insert("it", "it");
    map.insert("is", "is");
    map.insert("id", "id");
    map.insert("lv", "lv");
    map.insert("lt", "lt");
    map.insert("hu", "hu");
    map.insert("nl", "nl");
    map.insert("no", "no");
    map.insert("pl", "pl");
    map.insert("ro", "ro");
    map.insert("sr", "sr");
    map.insert("sk", "sk");
    map.insert("sl", "sl");
    map.insert("fi", "fi");
    map.insert("sv", "sv");
    map.insert("tr", "tr");
    map.insert("el", "el");
    map.insert("bg", "bg");
    map.insert("cs", "cs");
    map.insert("da", "da");
    map.insert("he", "he");
    map.insert("ja", "ja");
    map.insert("ko", "ko");
    map.insert("ru", "ru");
    map.insert("ar", "ar");
    map.insert("se", "nl");

    // === Closest language mappings ===
    map.insert("cy", "el"); // Cyprus → Greek
    map.insert("il", "he"); // Israel → Hebrew

    // Arabic world
    for c in [
        "sa", "ae", "eg", "dz", "ma", "tn", "ye", "om", "iq", "jo", "ps", "kw", "qa", "bh", "ly",
        "sd", "sy",
    ] {
        map.insert(c, "ar");
    }

    // Former USSR → Russian
    for c in ["uz", "kz", "kg", "tj", "by", "ua", "am", "az", "ge"] {
        map.insert(c, "ru");
    }

    // Chinese-speaking (no zh support) → All fallback
    for c in ["sg", "hk", "mo", "tw", "cn"] {
        map.insert(c, "all");
    }

    // South Asia → English
    for c in ["in", "pk", "bd", "np", "lk"] {
        map.insert(c, "en");
    }

    // Africa & rest → English
    for c in ["ng", "za", "ke", "gh", "tz", "ug", "zm", "zw", "mw"] {
        map.insert(c, "en");
    }

    Arc::new(map)
});

pub static ARTISTS_DB: Lazy<Arc<Database>> =
    Lazy::new(|| Arc::new(Database::open("kv/artists.redb").unwrap()));
pub const ARTISTS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("artists");
