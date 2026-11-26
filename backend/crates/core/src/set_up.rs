//! # Set Up
//!
//! Loads node config, or asks the user on first run.
//!
//! ## Architecture
//!
//! 1. [**Load**:][load_config] [CONFIG_FILE] if it exists.
//! 2. [**Ask**:][check_env] Otherwise 7 questions, every one can be answered by env variable.
//!     * `PRIECO_IP`, `PRIECO_PORT`, `PRIECO_TANTIVY`, `PRIECO_META`, `PRIECO_VECTOR`,
//!       `PRIECO_BLOB`, `PRIECO_WORKERS`.
//! 3. [**Dirs**:][create_dirs] Creates index directories.
//! 4. [**Save**:][save_config] Writes [CONFIG_FILE].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-01-31
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] Ask for `enable_*` services, now all are on

use std::{
    fs::{create_dir_all, read_to_string, write},
    io::{Write, stdin, stdout},
    net::IpAddr,
    path::Path,
    process::exit,
};

use crate::{CONFIG_FILE, DEFAULT_BLOB, DEFAULT_META, DEFAULT_VECTORS, helpers::file_exists};
use crate::{
    DEFAULT_TANTIVY,
    globals::{PriEcoConfig, colors},
};

/// Number of set up questions.
const TOTAL_QUESTIONS: usize = 7;

/// Returns node config, asks for it on first run.
///
/// All `enable_*` services are on in a new config.
///
/// # Arguments
///
/// None
///
/// # Returns
///
/// Loaded or new config.
///
/// # Panics
///
/// Exits the process on invalid IP, port or number of workers.
/// Panics if stdin or stdout fails.
pub fn set_up_wizard() -> PriEcoConfig {
    let mut conf = PriEcoConfig {
        ip: String::from("0.0.0.0"),
        port: 80,

        tantivy_path: DEFAULT_TANTIVY.to_string(),
        vector_path: DEFAULT_VECTORS.to_string(),

        meta_path: DEFAULT_META.to_string(),
        blob_path: DEFAULT_BLOB.to_string(),

        worker_concurrent: 1,

        enable_blob_storage: true,
        enable_db_inserter: true,
        enable_pagerank: true,
        enable_mini_crawler: true,
        enable_index_updater: true,
        enable_web_server: true,
    };

    if file_exists(CONFIG_FILE)
        && let Some(config) = load_config()
    {
        return config;
    }

    println!("Hi beautiful human being 👋");
    println!(
        "Let's get PriEco set up. I am going to ask you {} questions. I promise it's simple.",
        TOTAL_QUESTIONS
    );
    conf.ip=  match check_env("PRIECO_IP",&format!(
 "\n🗨 1/{}: Tell me my IP address. You can use 0.0.0.0 or 127.0.0.1 or something else.\n0.0.0.0 (Default if you just press enter) (Let's anyone on your locale network connect to this instance)\n127.0.0.1 (Connection can be established only from current device)\nYou will want to link public domain to this regardless\n\nWhat is my IP:",TOTAL_QUESTIONS),
 &conf.ip,
).parse::<IpAddr
>() {
               Ok(ip) =>  ip.to_string(), // valid IP, exit loop
               Err(_) => {println!("{}Error: Invalid IP address. Please enter a valid IPv4 or IPv6 address.{}", colors::RED, colors::RESET); exit(1)},
           }

           ;

    conf.port=match check_env("PRIECO_PORT",&format!(
        "\n🗨 2/{}: Tell me my PORT. This can be anything, usually 80 (default) or 443 or 8080 is used\nWhat is my PORT:",TOTAL_QUESTIONS),
        &conf.port.to_string(),
    )  .trim()
    .parse::<i32>()
    {
        Ok(num) => num,
        Err(_) =>{println!("{}Error: Invalid PORT. Please enter a valid NUMBER.{}", colors::RED, colors::RESET); exit(1)},
    };

    conf.tantivy_path = check_env(
        "PRIECO_TANTIVY",
        &format!(
            "\n🗨 3/{}: Path for Tantivy index (default {}):",
            TOTAL_QUESTIONS, conf.tantivy_path
        ),
        &conf.tantivy_path,
    );

    conf.meta_path = check_env(
        "PRIECO_META",
        &format!(
            "\n🗨 4/{}: Path for LMDB database (default {}):",
            TOTAL_QUESTIONS, conf.meta_path
        ),
        &conf.meta_path,
    );

    conf.vector_path = check_env(
        "PRIECO_VECTOR",
        &format!(
            "\n🗨 5/{}: Path for Vector index (default {}):",
            TOTAL_QUESTIONS, conf.vector_path
        ),
        &conf.vector_path,
    );

    conf.blob_path = check_env(
        "PRIECO_BLOB",
        &format!(
            "\n🗨 6/{}: Path for Blob storage (default {}):",
            TOTAL_QUESTIONS, conf.blob_path
        ),
        &conf.blob_path,
    );

    conf.worker_concurrent = match check_env(
        "PRIECO_WORKERS",
        &format!(
            "\n🗨 7/{}: Worker Condurent Website downloads (default: 1):",
            TOTAL_QUESTIONS
        ),
        &conf.worker_concurrent.to_string(),
    )
    .trim()
    .parse::<u32>()
    {
        Ok(num) => num,
        Err(_) => {
            println!(
                "{}Error: Invalid number of concurrent downloads. Please enter a valid NUMBER.{}",
                colors::RED,
                colors::RESET
            );
            exit(1)
        }
    };

    create_dirs(&conf);

    match save_config(&conf) {
        Ok(_) => {
            println!("{}Configuration was saved!{}", colors::GREEN, colors::RESET);
        }
        Err(e) => {
            println!(
                "{}Warning: Failed to save configuration file:{}{}",
                colors::YELLOW,
                colors::RESET,
                e
            );
        }
    }

    conf
}

/// Value of env variable `env_key`, otherwise [asks][ask] the user.
fn check_env(env_key: &str, prompt: &str, default: &str) -> String {
    if let Ok(val) = std::env::var(env_key) {
        println!("Loaded {} from environment.", env_key);
        return val;
    }

    ask(prompt, default)
}

/* Helper functions */
/// Prints `prompt` and reads a line from stdin.
///
/// # Returns
///
/// Trimmed answer, `default` when empty.
///
/// # Panics
///
/// If stdin or stdout fails.
fn ask(prompt: &str, default: &str) -> String {
    print!("{}", prompt);
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    let value = input.trim();
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}
/// Saves config as pretty JSON to [CONFIG_FILE].
fn save_config(conf: &PriEcoConfig) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&conf)?;
    write(CONFIG_FILE, json)?;
    Ok(())
}
/// Loads [CONFIG_FILE], `None` if it is missing or broken.
fn load_config() -> Option<PriEcoConfig> {
    let data = read_to_string(CONFIG_FILE).ok()?;
    let config: PriEcoConfig = serde_json::from_str(&data).ok()?;
    Some(config)
}

/// Creates index directories and the config directory. Errors are printed.
fn create_dirs(conf: &PriEcoConfig) {
    let mut paths: Vec<&Path> = vec![
        std::path::Path::new(&conf.tantivy_path),
        std::path::Path::new(&conf.meta_path),
        std::path::Path::new(&conf.vector_path),
        std::path::Path::new(&conf.blob_path),
    ];

    if let Some(parent) = std::path::Path::new(CONFIG_FILE).parent()
        && !parent.as_os_str().is_empty()
    {
        paths.push(parent);
    }

    for path in paths {
        if let Err(e) = create_dir_all(path) {
            eprintln!(
                "{}Warning: Failed to create directory '{}': {}{}",
                colors::YELLOW,
                path.display(),
                e,
                colors::RESET
            );
        }
    }
}
