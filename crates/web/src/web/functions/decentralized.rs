/*
  Import system libraries
*/
use std::{
    cmp::Reverse,
    collections::HashMap,
    fmt::{self, Display, Formatter},
    str::{FromStr, from_utf8},
    sync::{Arc, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

/*
  Import external libraries
*/
use data_encoding::BASE32_NOPAD;
use iroh::{
    Endpoint, EndpointAddr, PublicKey, SecretKey,
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
};
use iroh_gossip::{Gossip, TopicId, api::Event};
use n0_future::StreamExt;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prieco_core::{
    PRIECO_CONFIG, TANTIVY_INDEX, TANTIVY_READER, WebDocument, file_exists, url_to_id, write_file,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::interval;

/*
  Import own libraries
*/
use crate::web::functions::{
    ranking::{self},
    search_db::run_core_search,
};

/*
  Constants
*/
pub static IROH_ENDPOINT: OnceLock<Endpoint> = OnceLock::new();

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
        let mut text = BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = BASE32_NOPAD
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
pub struct KeywordBloom {
    pub bits: Vec<u8>,
}

impl KeywordBloom {
    pub fn new() -> Self {
        Self { bits: vec![0; 512] }
    }

    pub fn insert(&mut self, keyword_hash: u64) {
        let (h1, h2, h3) = Self::derive_hashes(keyword_hash);
        self.set_bit(h1);
        self.set_bit(h2);
        self.set_bit(h3);
    }

    pub fn contains(&self, keyword_hash: u64) -> bool {
        let (h1, h2, h3) = Self::derive_hashes(keyword_hash);
        self.get_bit(h1) && self.get_bit(h2) && self.get_bit(h3)
    }

    fn derive_hashes(h: u64) -> (usize, usize, usize) {
        let h1 = (h & 0xFFF) as usize;
        let h2 = ((h >> 12) & 0xFFF) as usize;
        let h3 = ((h >> 24) & 0xFFF) as usize;
        (h1, h2, h3)
    }

    fn set_bit(&mut self, index: usize) {
        self.bits[index / 8] |= 1 << (index % 8);
    }

    fn get_bit(&self, index: usize) -> bool {
        (self.bits[index / 8] & (1 << (index % 8))) != 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    pub node_id: [u8; 32],
    pub top_centroids: Vec<usize>,
    pub keyword: KeywordBloom,
}

pub fn build_node_profile(node_id: [u8; 32]) -> NodeProfile {
    // Vector index
    let mut centroid_sizes: Vec<(usize, u64)> = std::fs::read_dir(&PRIECO_CONFIG.vector_path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let id = name
                .strip_prefix("bucket_")?
                .strip_suffix(".bin.zst")?
                .parse::<usize>()
                .ok()?;
            let meta = e.metadata().ok()?;
            Some((id, meta.len()))
        })
        .collect();

    centroid_sizes.sort_unstable_by_key(|&(_, size)| Reverse(size));

    let mut top_centroids: Vec<usize> = centroid_sizes
        .into_iter()
        .take(20)
        .map(|(id, _)| id)
        .collect();

    top_centroids.sort_unstable();

    // FTS index
    let mut keyword = KeywordBloom::new();
    let schema = TANTIVY_INDEX.schema();

    if let Ok(content_field) = schema.get_field("content") {
        if let Some(reader) = TANTIVY_READER.searcher().segment_readers().first() {
            if let Ok(inv) = reader.inverted_index(content_field) {
                let mut terms: Vec<(String, u64)> = Vec::new();

                if let Ok(mut stream) = inv.terms().stream() {
                    while let Some((bytes, info)) = stream.next() {
                        if let Ok(s) = from_utf8(bytes) {
                            if info.doc_freq >= 5 && info.doc_freq <= 50 {
                                terms.push((s.to_string(), info.doc_freq as u64));
                            }
                        }
                    }
                }

                terms.sort_by_key(|(_, df)| *df);

                for (t, _) in terms.into_iter().take(5000) {
                    keyword.insert(url_to_id(&t));
                }
            }
        }
    }

    NodeProfile {
        node_id,
        top_centroids,
        keyword,
    }
}

pub static PROFILE_CACHE: Lazy<Arc<RwLock<HashMap<PublicKey, NodeProfile>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::with_capacity(1_000))));

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FedQuery {
    pub query: String,
    pub lang: String,
    pub loc: String,
    pub depth: u8,
    pub embed: Vec<f32>,
}

pub async fn gossip_sync(
    gossip: Gossip,
    topic: TopicId,
    bootstrap_peers: Vec<PublicKey>,
    my_profile: NodeProfile,
    cache: Arc<RwLock<HashMap<PublicKey, NodeProfile>>>,
) -> Result<(), String> {
    let (sender, mut receiver) = gossip
        .subscribe(topic, bootstrap_peers)
        .await
        .map_err(|e| e.to_string())?
        .split();

    receiver.joined().await.map_err(|e| e.to_string())?;

    let make_msg = || -> Vec<u8> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        serde_json::to_vec(&serde_json::json!({ "profile": my_profile, "nonce": nonce }))
            .unwrap_or_default()
    };

    if let Err(e) = sender.broadcast(make_msg().into()).await {
        println!("Initial broadcast failed: {}", e);
    }

    let mut heartbeat = interval(std::time::Duration::from_secs(30));
    heartbeat.tick().await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if let Err(e) = sender.broadcast(make_msg().into()).await {
                    println!("Broadcast failed: {}", e);
                }               }
            event = receiver.next() => {
                let Some(event) = event else { break };

                if let Ok(Event::Received(msg)) = event {
                    if let Ok(parsed) = serde_json::from_slice::<Value>(&msg.content) {

                        if let Some(offline_node_id) = parsed.get("offline") {
                            if let Ok(node_id_bytes) = serde_json::from_value::<[u8; 32]>(offline_node_id.clone()) {
                                if let Ok(pub_key) = PublicKey::from_bytes(&node_id_bytes) {
                                    let mut c = cache.write();
                                    if c.remove(&pub_key).is_some() {
                                        println!("👋 Peer {} went offline and was removed from cache.", pub_key);
                                    }
                                }
                            }
                            continue;
                        }

                        let profile_opt = if let Some(profile_val) = parsed.get("profile") {
                            serde_json::from_value::<NodeProfile>(profile_val.clone()).ok()
                        } else {
                            serde_json::from_slice::<NodeProfile>(&msg.content).ok()
                        };

                        if let Some(profile) = profile_opt {
                            if let Ok(pub_key) = PublicKey::from_bytes(&profile.node_id) {
                                let mut is_new_peer = false;
                                {
                                    let mut c = cache.write();
                                    if !c.contains_key(&pub_key) {
                                        println!("🤝 Discovered peer {} via Gossip!", pub_key);
                                        c.insert(pub_key, profile);
                                        is_new_peer = true;
                                    }
                                }

                                if is_new_peer {
                                    let _ = sender.broadcast(make_msg().into()).await;
                                }
                            }
                        }
                    } else {
                        if let Ok(profile) = serde_json::from_slice::<NodeProfile>(&msg.content) {
                            if let Ok(pub_key) = PublicKey::from_bytes(&profile.node_id) {
                                let mut is_new_peer = false;
                                {
                                    let mut c = cache.write();
                                    if !c.contains_key(&pub_key) {
                                        println!("🤝 Discovered peer {} via Gossip!", pub_key);
                                        c.insert(pub_key, profile);
                                        is_new_peer = true;
                                    }
                                }
                                if is_new_peer {
                                    let _ = sender.broadcast(make_msg().into()).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SearchProtocol {
    pub profile: NodeProfile,
}

impl ProtocolHandler for SearchProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await?;

        if let Ok(data) = recv.read_to_end(1024 * 1024).await {
            if let Ok(fed_query) = serde_json::from_slice::<FedQuery>(&data) {
                println!("📞 Received peer query!",);

                let mut results: Vec<WebDocument> = Vec::new();

                if fed_query.depth > 0 {
                    let mut fts_query = fed_query.query.clone();
                    let (intent, _) = ranking::meaning::call::process_query(
                        &mut fts_query,
                        &fed_query.lang,
                        &fed_query.loc,
                    );

                    let embed = fed_query.embed.clone();

                    let (dir_results, tantivy_results, vector_results, dis_results) =
                        run_core_search(
                            &fed_query.query,
                            &fed_query.lang,
                            &fed_query.loc,
                            &intent,
                            embed,
                            false,
                            Vec::new(),
                        )
                        .await;

                    results = ranking::rrf::run(
                        &fed_query.query,
                        &fed_query.lang,
                        &intent,
                        dir_results,
                        tantivy_results,
                        vector_results,
                        dis_results,
                        Vec::new(),
                        60.0,
                    );

                    results.truncate(20);
                }

                if let Ok(response_bytes) = serde_json::to_vec(&results) {
                    let _ = send.write_all(&response_bytes).await;
                }
            }
        }

        let _ = send.finish();
        connection.closed().await;
        Ok(())
    }
}
