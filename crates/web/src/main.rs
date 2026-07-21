/*
  File: main.rs
  Description: Sets up configuration for PriEco, starts web server and threads

  Author: Roman Lancos <support@prieco.net>
  License: AGPL v3.0

  Date Created: 2025-09-20
  Last Modified: 2026-01-31

  Usage: Run PriEco and complete set up questions
  TODO:
*/

/*
  Set global allovator
  Reason: Default was insufficient for deallocating RAM from crawler HTTP connections
  This one is modern
*/
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::runtime::Runtime;
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/*
  Import system libraries
*/
use std::{
    path::Path,
    process::exit,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{sleep, spawn},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/*
  Import external libraries
*/
use env_logger::Env;
use fjall::PersistMode;
use ort::{Environment, GraphOptimizationLevel, InMemorySession, LoggingLevel, SessionBuilder};
use rocket::{
    Request, Response,
    fairing::{AdHoc, Fairing, Info, Kind},
    fs::FileServer,
    http::{Header, Status},
    launch, routes,
};
use rocket_dyn_templates::Template;
use tokenizers::{PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer};

/*
  Import own libraries
*/
pub mod web;
use crate::web::routes::{apis::*, assets::*, pages::*};
use prieco_blob as blob;
use prieco_core::{
    ANALYTICS, EmbeddingService, META_DECODER, PAGERANK, PRIECO_CONFIG, PRIECO_FJALL,
    TANTIVY_READER, TANTIVY_WRITER, VECTOR_CENTROPOIDS, VECTOR_EMBEDDING_MODEL,
    VECTOR_EMBEDDING_TOKENIZER, colors, icons,
};
use prieco_insert::db_insert;
use prieco_mini_crawler::mini_crawler;
use prieco_pagerank as pagerank;

/*
  Set PriEco web server headers
*/
pub struct GlobalHeaders;
#[rocket::async_trait]
impl Fairing for GlobalHeaders {
    fn info(&self) -> Info {
        Info {
            name: "CORS + Security Headers + Block Bots",
            kind: Kind::Response | Kind::Request,
        }
    }
    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // --- Block curl/wget ---
        if let Some(agent) = req.headers().get_one("User-Agent") {
            let agent_lower = agent.to_ascii_lowercase();
            if agent_lower.starts_with("wget") || agent_lower.starts_with("curl") {
                res.set_status(Status::Forbidden);
                res.set_sized_body(0, std::io::Cursor::new("")); // empty body
                return; // skip adding headers
            }
        }
        // --- CORS headers ---
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new("Access-Control-Allow-Methods", "GET"));
        res.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));
        // --- Security headers ---
        res.set_header(Header::new(
            "Content-Security-Policy",
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self'; \
             img-src 'self' data: https://proxy.prieco.net; \
             connect-src 'self'; \
             frame-src 'self' https://cartes.app; \
             frame-ancestors 'self'; \
             form-action 'self'; \
             object-src 'none'; \
             base-uri 'self';",
        ));
        res.set_header(Header::new("X-Frame-Options", "SAMEORIGIN"));
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("Referrer-Policy", "no-referrer"));
        res.set_header(Header::new("Cache-Control", "no-store"));
    }
}

/*
  Description: Creates embeder + launches thread manager + launches Rocket web server

  Input: None
  Output: None
*/
#[launch]
async fn rocket() -> _ {
    // Disable foster parenting warning
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .filter_module("html5ever", log::LevelFilter::Error)
        .filter_module("markup5ever", log::LevelFilter::Error)
        .init();

    // Print banner
    println!(
        "{}{}{}",
        colors::GREEN,
        r#"
     ____  ____  _  _____ ____ ____
    /  __\/  __\/ \/  __//   _Y  _ \
    |  \/||  \/|| ||  \  |  / | / \|
    |  __/|    /| ||  /_ |  \_| \_/|
    \_/   \_/\_\\_/\____\\____|____/
    "#,
        colors::RESET
    );
    println!("Info:");
    println!(
        "{}: Blob storage\n{}: Database inserter\n{}: Pagerank\n{}: Mini crawler\n",
        icons::BLOB,
        icons::DB_INSERT,
        icons::PAGERANK_ICON,
        icons::MINI_CRAWLER_ICON
    );

    // Load config
    let _ = PRIECO_CONFIG;

    // Vector Embeding
    let embedding_service = EmbeddingService {
        tokenizer: Arc::new(tokio::sync::Mutex::new(create_tokenizer())),
        model: Arc::new(tokio::sync::Mutex::new(create_embeder())),
    };

    // Spawn Thread Manager
    spawn(move || {
        thread_manager();
    });

    // Analytics
    tokio::spawn(async { ANALYTICS.background_purge_task().await });

    // Launch Rocket web server
    rocket::build()
        .configure(
            rocket::Config::figment()
                .merge(("address", &PRIECO_CONFIG.ip))
                .merge(("port", PRIECO_CONFIG.port))
                .merge(("workers", num_cpus::get() * 2)),
        )
        .manage(embedding_service)
        .attach(GlobalHeaders)
        .attach(Template::fairing())
        .attach(AdHoc::on_shutdown("Flush DBs", |_| {
            Box::pin(async move {
                println!("Flushing Fjall to disk...");
                let _ = PRIECO_FJALL.meta_db.persist(PersistMode::SyncAll);
                let _ = PRIECO_FJALL.blob_db.persist(PersistMode::SyncAll);
                println!("{}Shutdown!{}", colors::GREEN, colors::RESET);
            })
        }))
        .mount(
            "/",
            routes![
                // Assets
                set_preferences, // Set cookie preferences
                sw_js,           // Service worker (Browser cache + unduck)
                unduck_js,
                security, // Security.txt
                robots,   // Robots.txt
                osd,
                script,
                favicon,
                privacy, // Privacy Policy
                // Landing page
                index,
                index_head,
                // Search
                search,
                search_post,
                results_htmls,
                api,
                stats,
                cache_ver,
                pageview,
                // Settings
                settings_htmls,
                settings_update,
                // Proxy
                proxy_get,
                proxy_post,
                // Extension
                ext_privacy,
                // Roadmap
                roadmap,
                submit_roadmap_feedback,
                submit_roadmap_vote,
                // Goggles
                goggles,
                load_goggle,
                apply_goggles,
                goggles_tint,
                update_qt,
                export_quick_tune,
                // Thanks page
                thanks,
                // Blob storage
                view_blob
            ],
        )
        .mount("/static", FileServer::from("./static"))
}

/*
  Description: Manages different PriEco threads like blob storage, database insertion and mini crawler

  Input: None
  Output: None
*/
fn thread_manager() {
    // Initialize data
    let _ = &*PRIECO_FJALL;
    let _ = &*META_DECODER;

    let _ = TANTIVY_READER;
    let _ = TANTIVY_WRITER;
    let _ = PAGERANK;

    println!("Starting GPU!");
    let _ = VECTOR_CENTROPOIDS.search(&vec![0.0; 384], 1, 1);

    // Blob storage
    let blob_thread = {
        spawn(move || {
            while !stop_requested() {
                blob::run();
                sleep(Duration::from_mins(1));
                break;
            }
        })
    };

    // Result database inserter
    let insert_thread = {
        spawn(move || {
            unsafe {
                libc::syscall(libc::SYS_ioprio_set, 1, 0, (3 << 13) | 7);
            }
            while !stop_requested() {
                if let Err(e) = db_insert::run() {
                    println!(
                        "{}Error inserting results: {}{}",
                        colors::RED,
                        e,
                        colors::RESET
                    );
                };
                sleep(Duration::from_mins(1));
            }
        })
    };

    // Pagerank
    let pagerank_thread = {
        spawn(move || {
            while !stop_requested() {
                pagerank::compute::run();
                sleep(Duration::from_hours(3));
            }
        })
    };

    // Mini crawler
    let mini_crawler_thread = if PRIECO_CONFIG.worker_id.is_empty() {
        None
    } else {
        Some(spawn(move || {
            let rt = Runtime::new().expect("Failed to create Tokio runtime for mini crawler");

            while !stop_requested() {
                rt.block_on(async {
                    crate::mini_crawler::run().await;
                });
            }
        }))
    };

    let _ = blob_thread.join();
    let _ = insert_thread.join();
    let _ = pagerank_thread.join();
    if let Some(thread) = mini_crawler_thread {
        let _ = thread.join();
    }

    println!("{}Threads finished!{}", colors::GREEN, colors::RESET);
}

/* Helper functions */
// Vector embeder creation
fn create_tokenizer() -> Tokenizer {
    let mut tokenizer: Tokenizer = match Tokenizer::from_bytes(VECTOR_EMBEDDING_TOKENIZER) {
        Ok(tokenizer) => tokenizer,
        Err(e) => {
            println!(
                "{}Main: Failed to create tokenizer: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        direction: PaddingDirection::Right,
        pad_to_multiple_of: None,
        pad_id: 0,
        pad_type_id: 0,
        pad_token: "[PAD]".into(),
    }));

    tokenizer
}
fn create_embeder() -> InMemorySession<'static> {
    let environment: Arc<Environment> = match Environment::builder()
        .with_name("embedder")
        .with_log_level(LoggingLevel::Warning)
        .build()
    {
        Ok(env) => Arc::new(env),
        Err(e) => {
            println!(
                "{}Main: Failed to create vector embedding environment: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    let mut session_builder = match SessionBuilder::new(&environment) {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}Main: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };
    session_builder = match session_builder.with_optimization_level(GraphOptimizationLevel::Level3)
    {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}Main: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    session_builder = match session_builder.with_parallel_execution(true) {
        Ok(builder) => builder,
        Err(e) => {
            println!(
                "{}Main: Failed to create vector embedding session builder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    };

    match session_builder.with_model_from_memory(VECTOR_EMBEDDING_MODEL) {
        Ok(emb) => emb,
        Err(e) => {
            println!(
                "{}Main: Failed to create embeder: {}{}",
                colors::RED,
                e,
                colors::RESET
            );
            exit(1);
        }
    }
}

/*
  Description: Checks if stop.txt exists, used for stopping threads

  Input: None
  Output: true if stop.txt exists, false otherwise
*/
static LAST_CHECK: AtomicU64 = AtomicU64::new(0);
static STOP_FLAG: AtomicBool = AtomicBool::new(false);

pub fn stop_requested() -> bool {
    if STOP_FLAG.load(Ordering::Relaxed) {
        return true;
    }

    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            println!(
                "{}Error getting system time: {}{}",
                colors::YELLOW,
                e,
                colors::RESET
            );
            return false;
        }
    };

    let last = LAST_CHECK.load(Ordering::Relaxed);
    if now - last >= 1 {
        let exists = Path::new("stop.txt").exists();
        STOP_FLAG.store(exists, Ordering::Relaxed);
        LAST_CHECK.store(now, Ordering::Relaxed);
    }

    STOP_FLAG.load(Ordering::Relaxed)
}
