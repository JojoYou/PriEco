//! Shared helpers: sandbox, rocket test clients, documents, tiny local HTTP server.

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Once,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use prieco_core::WebDocument;
use rocket::local::blocking::Client;

static INIT: Once = Once::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Removes temp dirs left behind by earlier test runs: a sandbox can't be deleted by the
/// run that created it (global DB handles stay open until exit) and a crash skips Drop.
/// Only dirs whose process is gone are touched, so parallel `cargo test` runs are safe.
fn sweep_stale() {
    // Without /proc there is no way to tell whether a pid is still alive
    if !Path::new("/proc/self").exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    let own_pid = std::process::id().to_string();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("prieco_") {
            continue;
        }
        // Every PriEco test dir is named prieco_<tag>_<pid>[_...]
        let Some(pid) = name
            .split('_')
            .find(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        else {
            continue;
        };
        if pid != own_pid && !Path::new("/proc").join(pid).exists() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Moves the test process into a private temp directory and answers the PriEco set up
/// wizard through env vars, so PRIECO_CONFIG / PRIECO_META / TANTIVY are all
/// created inside the sandbox. Call it first in every test.
pub fn init() -> PathBuf {
    let root = std::env::temp_dir().join(format!("prieco_web_sandbox_{}", std::process::id()));
    INIT.call_once(|| {
        sweep_stale();
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
            // Never bill or validate against the real Polar.sh
            std::env::remove_var("POLAR_API_KEY");
            std::env::remove_var("POLAR_ORG_ID");
        }
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
