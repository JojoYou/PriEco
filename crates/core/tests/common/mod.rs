//! Shared helpers for prieco_core integration tests.
//!
//! Everything written by tests lives under the OS temp dir and is removed on drop,
//! so the repository is never touched at test runtime.
#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Once,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TmpDir {
    path: PathBuf,
}

impl TmpDir {
    pub fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "prieco_core_test_{}_{}_{}",
            std::process::id(),
            tag,
            n
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TmpDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, p: &str) -> PathBuf {
        self.path.join(p)
    }

    pub fn join_str(&self, p: &str) -> String {
        self.join(p).to_string_lossy().into_owned()
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Unique string per call, handy for global keys (lock paths, counter names...)
pub fn unique(tag: &str) -> String {
    format!(
        "{}_{}_{}",
        tag,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Runs `f` on another thread and returns `Some(result)` if it finished within `timeout`.
/// A hung thread is leaked (the test process still exits once all tests finish).
pub fn run_with_timeout<T: Send + 'static>(
    timeout: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(timeout).ok()
}

static SANDBOX: Once = Once::new();

/// Moves the whole test process into a private temp directory and feeds the
/// PriEco set up wizard through environment variables, so global `Lazy` storages
/// (PRIECO_CONFIG, PRIECO_META, ANALYTICS...) can be used without stdin and
/// without writing anything into the repository.
///
/// Must be called at the start of every test in a file that touches those globals.
pub fn sandbox() -> PathBuf {
    let root = std::env::temp_dir().join(format!("prieco_core_sandbox_{}", std::process::id()));
    SANDBOX.call_once(|| {
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        std::env::set_current_dir(&root).unwrap();

        // SAFETY: called once, before any other thread of this test binary reads the env
        // through PriEco (all tests call `sandbox()` first, `Once` serialises them).
        unsafe {
            std::env::set_var("PRIECO_IP", "127.0.0.1");
            std::env::set_var("PRIECO_PORT", "8080");
            std::env::set_var("PRIECO_TANTIVY", root.join("idx/tantivy"));
            std::env::set_var("PRIECO_META", root.join("idx/meta"));
            std::env::set_var("PRIECO_VECTOR", root.join("idx/vectors"));
            std::env::set_var("PRIECO_BLOB", root.join("idx/blob"));
            std::env::set_var("PRIECO_WORKERS", "1");
            std::env::set_var("PRIECO_ZSTD_DICT", root.join("idx/no_dictionary.dict"));
        }
    });
    root
}
