//!  File: set_up.rs
//!  Description: Loads config or questions user
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2026-01-31
//!  Last Modified: 2026-02-06
//!
//!  Usage: Run PriEco and complete set up questions
//!  TODO:

use std::{
    fs::{create_dir_all, read_to_string, write},
    io::{Write, stdin, stdout},
    net::IpAddr,
    process::exit,
};

use crate::globals::{PriEcoConfig, colors};
use crate::helpers::file_exists;

const CONFIG_FILE: &str = "data/settings.conf";
const TOTAL_QUESTIONS: usize = 7;

pub fn set_up_wizard() -> PriEcoConfig {
    let mut conf = PriEcoConfig {
        ip: String::from("0.0.0.0"),
        port: 80,

        tantivy_path: String::from("idx/tantivy"),
        meta_path: String::from("idx/meta"),
        vector_path: String::from("idx/vectors"),
        blob_path: String::from("idx/blob"),

        worker_id: String::new(),
        worker_concurrent: 1,
    };

    if file_exists(CONFIG_FILE) {
        match load_config() {
            Some(config) => return config,
            None => {}
        }
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
            TOTAL_QUESTIONS, &conf.tantivy_path
        ),
        &conf.tantivy_path,
    );

    conf.meta_path = check_env(
        "PRIECO_META",
        &format!(
            "\n🗨 4/{}: Path for LMDB database (default {}):",
            TOTAL_QUESTIONS, &conf.meta_path
        ),
        &conf.meta_path,
    );

    conf.vector_path = check_env(
        "PRIECO_VECTOR",
        &format!(
            "\n🗨 5/{}: Path for Vector index (default {}):",
            TOTAL_QUESTIONS, &conf.vector_path
        ),
        &conf.vector_path,
    );

    conf.blob_path = check_env(
        "PRIECO_BLOB",
        &format!(
            "\n🗨 6/{}: Path for Blob storage (default {}):",
            TOTAL_QUESTIONS, &conf.blob_path
        ),
        &conf.blob_path,
    );

    conf.worker_id = check_env(
        "PRIECO_ID",
        &format!(
            "\n🗨 7/{}: Worker ID (leave empty if you dont have one):",
            TOTAL_QUESTIONS
        ),
        &conf.worker_id,
    );

    conf.worker_concurrent = match check_env(
        "PRIECO_WORKERS",
        &format!(
            "\n🗨 8/{}: Worker Condurent Website downloads (default: 1):",
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

    create_dirs(&conf);

    conf
}

fn check_env(env_key: &str, prompt: &str, default: &str) -> String {
    if let Ok(val) = std::env::var(env_key) {
        println!("Loaded {} from environment.", env_key);
        return val;
    }

    ask(prompt, default)
}

/* Helper functions */
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
fn save_config(conf: &PriEcoConfig) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&conf)?;
    write(CONFIG_FILE, json)?;
    Ok(())
}
fn load_config() -> Option<PriEcoConfig> {
    let data = read_to_string(CONFIG_FILE).ok()?;
    let config: PriEcoConfig = serde_json::from_str(&data).ok()?;
    Some(config)
}

fn create_dirs(conf: &PriEcoConfig) {
    let paths = [
        &conf.tantivy_path,
        &conf.meta_path,
        &conf.vector_path,
        &conf.blob_path,
    ];

    for path in paths {
        if let Err(e) = create_dir_all(path) {
            eprintln!(
                "{}Warning: Failed to create directory '{}': {}{}",
                colors::YELLOW,
                path,
                e,
                colors::RESET
            );
        }
    }
}
