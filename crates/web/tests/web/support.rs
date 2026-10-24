//! Shared helpers: sandbox, rocket test clients, documents, tiny local HTTP server.

use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Once,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use prieco_core::WebDocument;
use redb::{Database, TableDefinition};
use rocket::local::blocking::Client;

static INIT: Once = Once::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Mirrors the private IP table of prieco_core::IpGeoDatabase
const IP_RANGES: TableDefinition<u128, (u128, String)> = TableDefinition::new("ip_ranges");

fn v4(s: &str) -> u128 {
    u32::from(s.parse::<Ipv4Addr>().unwrap()) as u128
}

/// Moves the test process into a private temp directory and answers the PriEco set up
/// wizard through env vars, so PRIECO_CONFIG / PRIECO_META / TANTIVY / IP_TO_LOC are all
/// created inside the sandbox. Call it first in every test.
///
/// IP ranges in the sandbox geo database:
///   1.0.0.0 - 1.0.0.255 -> "SK"
///   2.0.0.0 - 2.0.0.255 -> "" (empty country)
///   3.0.0.0 - 3.0.0.255 -> "US"
pub fn init() -> PathBuf {
    let root = std::env::temp_dir().join(format!("prieco_web_sandbox_{}", std::process::id()));
    INIT.call_once(|| {
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        std::env::set_current_dir(&root).unwrap();

        // SAFETY: executed once before any PriEco global reads the environment
        unsafe {
            std::env::set_var("PRIECO_IP", "127.0.0.1");
            std::env::set_var("PRIECO_PORT", "8080");
            std::env::set_var("PRIECO_TANTIVY", root.join("idx/tantivy"));
            std::env::set_var("PRIECO_META", root.join("idx/meta"));
            std::env::set_var("PRIECO_VECTOR", root.join("idx/vectors"));
            std::env::set_var("PRIECO_BLOB", root.join("idx/blob"));
            std::env::set_var("PRIECO_WORKERS", "1");
            std::env::set_var("PRIECO_ZSTD_DICT", root.join("idx/no_dictionary.dict"));
            std::env::remove_var("NEWS_API_KEY");
            std::env::remove_var("YADORE_API_KEY");
            std::env::remove_var("SIGNAL_BOT_NUMBER");
            std::env::remove_var("SIGNAL_RECIPIENT_NUMBER");
        }

        fs::create_dir_all("kv").unwrap();
        let db = Database::create("kv/ip.redb").unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut t = tx.open_table(IP_RANGES).unwrap();
            t.insert(v4("1.0.0.0"), (v4("1.0.0.255"), "SK".to_string()))
                .unwrap();
            t.insert(v4("2.0.0.0"), (v4("2.0.0.255"), String::new()))
                .unwrap();
            t.insert(v4("3.0.0.0"), (v4("3.0.0.255"), "US".to_string()))
                .unwrap();
        }
        tx.commit().unwrap();
        drop(db);
    });
    root
}

pub fn unique(tag: &str) -> String {
    format!("{}_{}", tag, COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn unique_u64() -> u64 {
    0xC0FF_EE00_0000_0000 + COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Local rocket client with the given routes mounted at "/"
pub fn client(routes: Vec<rocket::Route>) -> Client {
    init();
    Client::tracked(rocket::build().mount("/", routes)).unwrap()
}

pub fn doc(url: &str) -> WebDocument {
    WebDocument {
        url: url.to_string(),
        title: String::new(),
        description: String::new(),
        content: String::new(),
        favicon: String::new(),
        image: String::new(),
        keywords: String::new(),
        safe_s: true,
        lang: String::new(),
        loc: String::new(),
        html: String::new(),
        impressions: 0,
        clicks: 0,
        confidence: 0.0,
        effort: 0.0,
        qna: 0.0,
        sts: 0.0,
        load: 0.0,
        date: 0,
        intent: 5,
        is_mobile: false,
        has_500_words: false,
        search_score: 0.0,
        source: String::new(),
    }
}

pub fn scored(url: &str, score: f32) -> WebDocument {
    let mut d = doc(url);
    d.search_score = score;
    d
}

/* ───────────────────────────── local HTTP server ───────────────────────────── */

fn read_request_path(s: &mut TcpStream) -> String {
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    String::from_utf8_lossy(&buf)
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string()
}

pub fn http(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

/// Serves up to `n` connections on `bind` ("127.0.0.1:0", "[::1]:0"...).
/// Returns the port, or None when the address family is unavailable.
pub fn serve_on(bind: &str, n: usize, respond: fn(&str) -> Vec<u8>) -> Option<u16> {
    let listener = TcpListener::bind(bind).ok()?;
    let port = listener.local_addr().ok()?.port();
    thread::spawn(move || {
        for stream in listener.incoming().take(n) {
            let Ok(mut s) = stream else { continue };
            let path = read_request_path(&mut s);
            let _ = s.write_all(&respond(&path));
            let _ = s.flush();
        }
    });
    Some(port)
}

pub fn serve(n: usize, respond: fn(&str) -> Vec<u8>) -> u16 {
    serve_on("127.0.0.1:0", n, respond).unwrap()
}

pub fn closed_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}
