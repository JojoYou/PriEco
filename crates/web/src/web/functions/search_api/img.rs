use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::Rng;
use serde_json::Value;
use std::{
    fs::{metadata, read, write},
    hash::Hasher,
};
use twox_hash::XxHash64;

use crate::web::functions::general::get_domain;
use prieco_core::{CLIENT, ImgResult, colors, read_file, url_to_id};

/*
  Description: 3rd party image search with 0 knowledge cache
               Query is hashed: used to find JSON blob
               Original query is used to decrypt the blob
               Disk stores encrypted files with hashed names
  Input: Query
  Output: Image results
*/
pub async fn run(query: &str) -> Vec<ImgResult> {
    ////
    // Cache
    ////
    let cache_id = url_to_id(query); // In this case query to id
    let bing_file = format!("cache/img/bing/{}.bin", cache_id);
    if metadata(&bing_file).is_ok() {
        println!("Cache!");

        if let Ok(encrypted_bytes) = read(&bing_file) {
            if let Ok(decrypted_str) = decrypt_cache(query, &encrypted_bytes) {
                let bing_json: Value = match serde_json::from_str(&decrypted_str) {
                    Ok(json) => json,
                    Err(e) => {
                        println!("Failed to parse decrypted Bing cache {}: {}", bing_file, e);
                        Value::Null
                    }
                };

                if !bing_json.is_null() {
                    return format_bing(bing_json);
                }
            } else {
                println!("Failed to decrypt cache file: {}", bing_file);
            }
        }
    }

    ////
    // API
    ////
    println!("Call!");

    let bing_result = CLIENT
        .get(&format!(
            "https://proxy.prieco.net/images/search?q={}",
            query,
        ))
        .send()
        .await;

    let bing_json: Value = match bing_result {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            if !status.is_success() {
                println!(
                    "{}Bing request failed with status {}{}: {}",
                    colors::RED,
                    status,
                    colors::RESET,
                    body
                );
                Value::Null
            } else {
                match serde_json::from_str::<Value>(&body) {
                    Ok(json) => json,
                    Err(e) => {
                        println!(
                            "{}Failed to parse Bing JSON: {}{} — body was: {}",
                            colors::RED,
                            e,
                            colors::RESET,
                            body
                        );
                        Value::Null
                    }
                }
            }
        }
        Err(e) => {
            println!("{}Bing request error: {}{}", colors::RED, e, colors::RESET);
            Value::Null
        }
    };

    ////
    // Cache results
    ////
    if !bing_json.is_null() {
        match serde_json::to_string(&bing_json) {
            Ok(json_str) => match encrypt_cache(query, &json_str) {
                Ok(encrypted_data) => {
                    if let Err(e) = write(&bing_file, encrypted_data) {
                        println!("Failed to write encrypted Bing cache {}: {}", bing_file, e);
                    }
                }
                Err(e) => println!("Failed to encrypt Bing cache: {}", e),
            },
            Err(e) => {
                println!("Failed to serialize Bing JSON for caching: {}", e);
            }
        }
    }
    format_bing(bing_json)
}

fn format_bing(json: Value) -> Vec<ImgResult> {
    let mut remote_results: Vec<ImgResult> = Vec::with_capacity(50);

    if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
        for item in results {
            let url = match item.get("url").and_then(|v| v.as_str()) {
                Some(u) => u,
                None => continue,
            };

            remote_results.push(ImgResult {
                thumbnail: replace_resulti(
                    item.get("thumbnail")
                        .and_then(|t| t.get("src"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default(),
                ),

                image: replace_resulti(
                    item.get("properties")
                        .and_then(|p| p.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default(),
                ),

                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                site_url: url.to_string(),
                site_domain: get_domain(url, true),

                favicon: replace_resulti(
                    item.get("meta_url")
                        .and_then(|m| m.get("favicon"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default(),
                ),
            });
        }
    }

    remote_results
}

/* Helper functions */
fn replace_resulti(url: &str) -> String {
    url.replace("https://api.resulti.org", "https://proxy.prieco.net")
}

fn encrypt_cache(query: &str, plaintext: &str) -> Result<Vec<u8>, aes_gcm::Error> {
    let key_bytes = derive_key(query);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())?;

    let mut final_data = nonce.to_vec();
    final_data.extend_from_slice(&ciphertext);

    Ok(final_data)
}

fn decrypt_cache(query: &str, encrypted_data: &[u8]) -> Result<String, aes_gcm::Error> {
    if encrypted_data.len() < 12 {
        return Err(aes_gcm::Error);
    }

    let key_bytes = derive_key(query);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher.decrypt(nonce, ciphertext)?;

    Ok(String::from_utf8_lossy(&plaintext_bytes).into_owned())
}

fn derive_key(query: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let bytes = query.as_bytes();

    for i in 0..4 {
        let mut h = XxHash64::with_seed(i as u64);
        h.write(bytes);
        let part = h.finish().to_le_bytes();

        let start = i * 8;
        let end = start + 8;
        key[start..end].copy_from_slice(&part);
    }

    key
}
