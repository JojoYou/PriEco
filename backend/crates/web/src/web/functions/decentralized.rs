//! # Decentralization
//!
//! PriEco nodes find each other and search each other's index over Iroh.
//!
//! ## Architecture
//!
//! 1. [**Identity**:][get_iroh_secret] Every node keeps one Iroh secret key.
//! 2. [**Join**:][Ticket] A ticket holds the gossip topic and known nodes.
//! 3. [**Profile**:][build_node_profile] What a node has in its index.
//!     * 20 biggest IVF buckets.
//!     * [Bloom filter][KeywordBloom] of 5000 rare words (in 5-50 pages).
//! 4. [**Gossip**:][gossip_sync] Profiles are sent on join, to new peers and every 2 hours.
//!     * Profiles are kept in [PROFILE_CACHE]. 'offline' message removes a node.
//! 5. [**Search**:][SearchProtocol] Peers send a [FedQuery], node answers with its top 20 results.
//!     * Peers are picked by their profile, so not every node is asked.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-09-08
//! * Last Modified: 2026-11-24
//!
//! ## Planned Improvements
//!
//! - [ ] None

/*
  Import system libraries
*/
use std::{
    cmp::Reverse,
    collections::HashMap,
    fmt::{self, Display, Formatter},
    str::{FromStr, from_utf8},
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    ANALYTICS, IROH_SECRET_FILE, PRIECO_CONFIG, PRIECO_INDEX, Stage, TANTIVY_INDEX, TANTIVY_READER,
    WebDocument, file_exists, url_to_id, write_file,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tokio::time::interval;

/*
  Import own libraries
*/
use crate::web::functions::{
    ranking::{self, rrf::RRFParams},
    search_db::run_core_search,
};

/*
  Constants
*/
/// Iroh endpoint of this node, set on start.
pub static IROH_ENDPOINT: OnceLock<Endpoint> = OnceLock::new();
/// Size of [KeywordBloom], 4096 bits.
const BLOOM_BYTES: usize = 512;

// Ticket
/// Invitation to the PriEco network.
///
/// As text it is lowercase base32 of JSON, set with `PRIECO_TICKET`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Ticket {
    /// Gossip topic.
    pub topic: TopicId,
    /// Nodes to connect to first.
    pub endpoints: Vec<EndpointAddr>,
}

impl Ticket {
    /// Ticket from JSON.
    fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Ticket as JSON.
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
/// Secret key from [IROH_SECRET_FILE], new one is generated and saved when missing or broken.
pub fn get_iroh_secret() -> SecretKey {
    if !file_exists(IROH_SECRET_FILE) {
        return gen_iroh_secret();
    }

    if let Ok(bytes) = std::fs::read(IROH_SECRET_FILE)
        && bytes.len() == 32
    {
        let mut array = [0u8; 32];

        array.copy_from_slice(&bytes);

        return SecretKey::from_bytes(&array);
    }

    gen_iroh_secret()
}

/// Generates and saves new secret key.
fn gen_iroh_secret() -> SecretKey {
    let secret = SecretKey::generate();

    write_file(IROH_SECRET_FILE, secret.to_bytes(), false);

    secret
}

// Profile
/// Bloom filter of word hashes, 3 bits per word.
///
/// Bits are taken from the 12 bit parts of [url_to_id] of the word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordBloom {
    /// Exactly [BLOOM_BYTES].
    #[serde(deserialize_with = "exact_bloom_bits")]
    pub bits: Vec<u8>,
}
/// Rejects filters of wrong size from peers.
fn exact_bloom_bits<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let bits = Vec::<u8>::deserialize(d)?;
    if bits.len() != BLOOM_BYTES {
        return Err(serde::de::Error::invalid_length(
            bits.len(),
            &"512 bloom bytes",
        ));
    }
    Ok(bits)
}

impl KeywordBloom {
    /// Empty filter.
    pub fn new() -> Self {
        Self {
            bits: vec![0; BLOOM_BYTES],
        }
    }

    /// Adds word hash.
    pub fn insert(&mut self, keyword_hash: u64) {
        let (h1, h2, h3) = Self::derive_hashes(keyword_hash);
        self.set_bit(h1);
        self.set_bit(h2);
        self.set_bit(h3);
    }

    /// True if word hash may be in, false if surely not.
    pub fn contains(&self, keyword_hash: u64) -> bool {
        let (h1, h2, h3) = Self::derive_hashes(keyword_hash);
        self.get_bit(h1) && self.get_bit(h2) && self.get_bit(h3)
    }

    /// 3 bit positions from a hash.
    fn derive_hashes(h: u64) -> (usize, usize, usize) {
        let h1 = (h & 0xFFF) as usize;
        let h2 = ((h >> 12) & 0xFFF) as usize;
        let h3 = ((h >> 24) & 0xFFF) as usize;
        (h1, h2, h3)
    }

    /// Sets bit `i`.
    fn set_bit(&mut self, i: usize) {
        if let Some(b) = self.bits.get_mut(i / 8) {
            *b |= 1 << (i % 8);
        }
    }

    /// Reads bit `i`.
    fn get_bit(&self, i: usize) -> bool {
        self.bits
            .get(i / 8)
            .is_some_and(|b| b & (1 << (i % 8)) != 0)
    }
}

impl Default for KeywordBloom {
    fn default() -> Self {
        Self::new()
    }
}

/// What a node has in its index, shared over gossip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProfile {
    /// Iroh public key.
    pub node_id: [u8; 32],
    /// 20 biggest IVF buckets, sorted.
    pub top_centroids: Vec<usize>,
    /// Rare words in the index.
    pub keyword: KeywordBloom,
}

/// Builds profile of this node.
///
/// Words come from the first Tantivy segment only.
///
/// # Arguments
///
/// * `node_id` - Iroh public key.
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

    if let Ok(content_field) = schema.get_field("content")
        && let Some(reader) = TANTIVY_READER.searcher().segment_readers().first()
        && let Ok(inv) = reader.inverted_index(content_field)
    {
        let mut terms: Vec<(String, u64)> = Vec::new();

        if let Ok(mut stream) = inv.terms().stream() {
            while let Some((bytes, info)) = stream.next() {
                if let Ok(s) = from_utf8(bytes)
                    && info.doc_freq >= 5
                    && info.doc_freq <= 50
                {
                    terms.push((s.to_string(), info.doc_freq as u64));
                }
            }
        }

        terms.sort_by_key(|(_, df)| *df);

        for (t, _) in terms.into_iter().take(5000) {
            keyword.insert(url_to_id(&t));
        }
    }

    NodeProfile {
        node_id,
        top_centroids,
        keyword,
    }
}

/// Known peers and their profiles.
pub static PROFILE_CACHE: Lazy<Arc<RwLock<HashMap<PublicKey, NodeProfile>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::with_capacity(1_000))));

/// Search request sent to a peer.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FedQuery {
    /// User query.
    pub query: String,
    /// Language.
    pub lang: String,
    /// Location.
    pub loc: String,
    /// 0 means don't search.
    pub depth: u8,
    /// Query vector, peer doesn't embed it again.
    pub embed: Vec<f32>,
}

/// Joins gossip topic and keeps [PROFILE_CACHE] up to date. Runs until gossip stops.
///
/// # Arguments
///
/// * `gossip` - Iroh gossip.
/// * `topic` - Topic from ticket.
/// * `bootstrap_peers` - Nodes from ticket.
/// * `my_profile` - Profile of this node.
/// * `cache` - Where peer profiles are kept.
///
/// # Errors
///
/// If topic can't be joined.
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

    let mut heartbeat = interval(Duration::from_hours(2));
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
                            if let Ok(node_id_bytes) = serde_json::from_value::<[u8; 32]>(offline_node_id.clone())
                                && let Ok(pub_key) = PublicKey::from_bytes(&node_id_bytes) {
                                    let mut c = cache.write();
                                    if c.remove(&pub_key).is_some() {
                                        println!("👋 Peer {} went offline and was removed from cache.", pub_key);ANALYTICS.count("p2p_peer_left");

                                    }
                                }
                            continue;
                        }

                        let profile_opt = if let Some(profile_val) = parsed.get("profile") {
                            serde_json::from_value::<NodeProfile>(profile_val.clone()).ok()
                        } else {
                            serde_json::from_slice::<NodeProfile>(&msg.content).ok()
                        };

                        if let Some(profile) = profile_opt
                            && let Ok(pub_key) = PublicKey::from_bytes(&profile.node_id) {
                                let mut is_new_peer = false;
                                {
                                    let mut c = cache.write();
                                    if let std::collections::hash_map::Entry::Vacant(e) = c.entry(pub_key) {
                                        println!("🤝 Discovered peer {} via Gossip!", pub_key);
                                        e.insert(profile);ANALYTICS.count("p2p_peer_joined");

                                        is_new_peer = true;
                                    }
                                }

                                if is_new_peer {
                                    let _ = sender.broadcast(make_msg().into()).await;
                                }
                            }
                    } else {
                        if let Ok(profile) = serde_json::from_slice::<NodeProfile>(&msg.content)
                            && let Ok(pub_key) = PublicKey::from_bytes(&profile.node_id) {
                                let mut is_new_peer = false;
                                {
                                    let mut c = cache.write();
                                    if let std::collections::hash_map::Entry::Vacant(e) = c.entry(pub_key) {
                                        println!("🤝 Discovered peer {} via Gossip!", pub_key);
                                        e.insert(profile);ANALYTICS.count("p2p_peer_joined");

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
    Ok(())
}

/// Answers [FedQuery] of other nodes.
///
/// Request max 1 MB. Search runs on own index without discovery and other nodes,
/// results are merged with [RRF][ranking::rrf] (k = 60) and cut to 20.
#[derive(Debug, Clone)]
pub struct SearchProtocol {
    /// Profile of this node.
    pub profile: NodeProfile,
}

impl ProtocolHandler for SearchProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let started = std::time::Instant::now(); // first line of the function

        let (mut send, mut recv) = connection.accept_bi().await?;

        if let Ok(data) = recv.read_to_end(1024 * 1024).await
            && let Ok(fed_query) = serde_json::from_slice::<FedQuery>(&data)
        {
            println!("📞 Received peer query!",);
            ANALYTICS.count("p2p_served");

            let mut results: Vec<WebDocument> = Vec::new();

            if fed_query.depth > 0 {
                let mut fts_query = fed_query.query.clone();
                let (intent, _) = ranking::meaning::call::process_query(
                    &mut fts_query,
                    &fed_query.lang,
                    &fed_query.loc,
                );

                let embed = fed_query.embed.clone();

                let (dir_results, tantivy_results, vector_results, dis_results) = run_core_search(
                    &fed_query.query,
                    &fed_query.lang,
                    &fed_query.loc,
                    &intent,
                    embed,
                    false,
                    Vec::new(),
                    PRIECO_INDEX.clone(),
                )
                .await;

                results = ranking::rrf::run(RRFParams {
                    query: &fed_query.query,
                    lang: &fed_query.lang,
                    intent: &intent,
                    dir_results,
                    fts_results: tantivy_results,
                    ivf_results: vector_results,
                    dis_results,
                    dec_results: Vec::new(),
                    k: 60.0,
                });

                results.truncate(20);
            }

            if let Ok(response_bytes) = serde_json::to_vec(&results) {
                let _ = send.write_all(&response_bytes).await;
            }
            ANALYTICS.record_latency(Stage::PeerServe, started.elapsed());
        }

        let _ = send.finish();
        connection.closed().await;
        Ok(())
    }
}
