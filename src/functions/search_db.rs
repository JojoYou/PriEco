use ahash::AHashSet;
use manticoresearch::{
    apis::{SearchApi, SearchApiClient, UtilsApi, UtilsApiClient, configuration::Configuration},
    models::{HitsHits, KnnQuery, SearchQuery, SearchRequest, SqlResponse},
};
use rocket::{State, serde::json::Json};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

use crate::{
    get_domain,
    globals::{EmbeddingService, SearchResult, TOP_DOMAINS},
};

pub async fn run(
    results: &mut Vec<SearchResult>,
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) -> f64 {
    let (confidence_score, local_results) = run_json(q, lang, loc, embedding_service).await;

    ////
    // Create final results
    ////
    if let Some(arr) = Json(Value::from(local_results)).as_array() {
        for item in arr {
            let url = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            results.push(SearchResult {
                url: url.clone(),
                display_url: item
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
                    .replace("https://", "")
                    .replace("http://", "")
                    .replace("www.", "")
                    .trim_end_matches('/')
                    .replace("/", " › "),
                domain: format!(
                    "{} 🍃",
                    get_domain(
                        &item
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        true,
                    )
                ),
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                image: item
                    .get("image")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        format!(
                            "<img loading='lazy' alt='‎' src='/proxy?u={}'>",
                            urlencoding::encode(s)
                        )
                    })
                    .unwrap_or_default(),
                favicon: item
                    .get("favicon")
                    .and_then(|v| v.as_str()) // ensure it's a string
                    .filter(|s| !s.is_empty()) // only keep non-empty values
                    .map(|s| format!("static/prieco_favicons/{}", s)) // local path
                    .unwrap_or_else(|| {
                        format!(
                            "/proxy?u=https://www.google.com/s2/favicons?domain={}&sz=512",
                            get_domain(&url, false)
                        )
                    }),
            });
        }
    }

    confidence_score // Return confidence score
}

fn process(
    source: &Value,
    hit: Option<&HitsHits>, // Made optional since SQL rows don't have hit data
    fields_to_show: &[&str],
    query: &str,
    lang: &str,
    loc: &str,
) -> Value {
    let mut filtered = serde_json::Map::new();

    // Extract the specified fields from source
    for field in fields_to_show {
        if let Some(value) = source.get(field) {
            let processed_value = if field == &"image" && value.is_string() {
                json!(value.as_str().unwrap().trim_end_matches('/'))
            } else {
                value.clone()
            };
            filtered.insert(field.to_string(), processed_value);
        }
    }

    // Calculate custom ranking boost
    let ranking_boost = match source.get("lang").and_then(|v| v.as_str()) {
        // Language
        Some(value) if value == lang => 3000.0,
        _ => 0.0,
    } + match source.get("loc").and_then(|v| v.as_str()) {
        // Location
        Some(value) if value == loc => 2000.0,
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // (https://www/)query(.tld/)
        Some(url)
            if url
                .split("://")
                .nth(1)
                .and_then(|s| s.split('/').next())
                .and_then(|d| d.strip_prefix("www.").or(Some(d)))
                .and_then(|d| d.split('.').next())
                .map_or(false, |domain| {
                    domain.eq_ignore_ascii_case(query) && url.matches('/').count() <= 3
                }) =>
        {
            5000.0
        }
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // Homepage
        Some(url)
            if url
                .split("://")
                .nth(1)
                .map_or(false, |s| !s.trim_end_matches('/').contains('/')) =>
        {
            1000.0
        }
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // Wikipedia
        Some(url)
            if url
              .contains(".wikipedia.org/wiki/") =>
        {
            1000.0
        }
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // Domain tld = loc
        Some(url)
            if url
                .split("://")
                .nth(1)
                .and_then(|s| s.trim_end_matches('/').split('.').last())
                .map_or(false, |tld| tld.eq_ignore_ascii_case(loc)) =>
        {
            2000.0
        }
        _ => 0.0,
    } /*+
        // Pagerank: 13%
    {
      (|| {
          let read_txn = PAGERANK.begin_read().ok()?;
          let table = read_txn.open_table(PAGERANKS_TABLE).ok()?;

          let url = source.get("url")?.as_str()?;
          let node_hash = hash_node(url).to_string();

          let rank = table.get(&*node_hash).ok()??;

          Some(rank.value() * 1_000_000_000.0 * 260.0)
      })().unwrap_or(0.0)
    }*/ + match source.get("confidence").and_then(|v| v.as_str()) {
        // Confidence 10%
        Some(confidence) => match confidence.parse::<f64>() {
            Ok(num) => num * 200.0,
            Err(_) => 0.0,
        },
        _ => 0.0,
    } + match source.get("effort").and_then(|v| v.as_str()) {
        // Effort 5%
        Some(effort) => match effort.parse::<f64>() {
            Ok(num) => num * 100.0,
            Err(_) => 0.0,
        },
        _ => 0.0,
    } + match source.get("load").and_then(|v| v.as_str()) {
        // Load speed 3%
        Some(load) => {
            let load = match load.parse::<f64>() {
                Ok(num) => num,
                Err(_) => 10.0,
            };
            if load < 0.3 {
                60.0
            } else if load < 0.5 {
                30.0
            } else if load < 2.0 {
                15.0
            } else if load < 3.0 {
                5.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // SSL 2%
        Some(url) if url.starts_with("https://") => 40.0,
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // Well respected TLDs 2%
        Some(url) => {
            if url.contains(".dev") {
                40.0
            } else if url.contains(".com") {
                30.0
            } else if url.contains(".org") || url.contains(".net") {
                20.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    } + match source.get("url").and_then(|v| v.as_str()) {
        // Not recommended characters 1%
        Some(url) if url.contains("_") || url.contains(":") || url.contains(",,") => -20.0,
        _ => 0.0,
    };

    // Calculate final score
    let original_score = hit.and_then(|h| h._score).unwrap_or(0);
    let final_score = original_score as f64 + ranking_boost;

    // Add scoring fields
    filtered.insert("_score".to_string(), json!(final_score));
    filtered.insert("_original_score".to_string(), json!(original_score));

    // Only add KNN distance if we have hit data
    if let Some(hit) = hit {
        if let Some(knn_distance) = &hit._knn_dist {
            filtered.insert("_knn_distance".to_string(), json!(knn_distance));
        }
    }

    Value::Object(filtered)
}

pub async fn run_json(
    q: &str,
    lang: &str,
    loc: &str,
    embedding_service: &State<EmbeddingService>,
) -> (f64, Vec<Value>) {
    // Data
    let fields_to_show = [
        "title",
        "description",
        "url",
        "favicon",
        "image",
        "lang",
        "loc",
        "safe_s",
    ];

    ////
    // Manticore Search
    ////
    // Exact root domain
    let exact_future = async {
        let mut exact_results = Vec::new();
        let mut urls = Vec::new();

        for tld in vec![
            "com", "org", "net", "co.uk", "de", "fr", "ca", "au", "in", "br", "mx", "es", "it",
            "ru", "jp", "cn",
        ] {
            urls.push(format!("'https://{}.{}/'", &q, tld));
            urls.push(format!("'http://{}.{}/'", &q, tld));
            urls.push(format!("'https://www.{}.{}/'", &q, tld));
            urls.push(format!("'http://www.{}.{}/'", &q, tld));
        }

        if let Ok(response) = UtilsApiClient::new(Arc::new(Configuration::new()))
            .sql(
                &format!("SELECT * FROM web WHERE url IN ({});", urls.join(", ")),
                Some(true),
            )
            .await
        {
            // Process response...
            match response {
                SqlResponse::SqlRawResponse(raw_response) => {
                    for result_set in raw_response {
                        if let Some(data_array) = result_set.get("data").and_then(|d| d.as_array())
                        {
                            for row in data_array {
                                if let Some(url) = row.get("url").and_then(|v| v.as_str()) {
                                    exact_results.push((
                                        url.to_string(),
                                        process(row, None, &fields_to_show, &q, lang, loc),
                                    ));
                                }
                            }
                        }
                    }
                }
                SqlResponse::SqlObjResponse(_) => {}
            }
        }
        exact_results
    };

    // Text search
    let text_future = async {
        let mut text_results = Vec::new();

        let txt_search = SearchApiClient::new(Arc::new(Configuration::new()))
            .search(SearchRequest {
                table: "web".to_string(),
                query: Some(Box::new(SearchQuery {
                    query_string: Some(q.to_lowercase()),
                    ..Default::default()
                })),
                options: Some(json!(HashMap::from([(
                    "ranker".to_string(),
                    json!("expr('sum(lcs) + sum(bm25)')")
                )]))),
                limit: Some(30),
                ..Default::default()
            })
            .await;
        if let Ok(response) = txt_search {
            for hit in response.hits.and_then(|h| h.hits).unwrap_or_default() {
                if let Some(source) = hit._source.as_ref() {
                    if let Some(url) = source.get("url").and_then(|v| v.as_str()) {
                        text_results.push((
                            url.to_string(),
                            process(&source, Some(&hit), &fields_to_show, &q, lang, loc),
                        ));
                    }
                }
            }
        }
        text_results
    };

    // Vector search
    let vector_future = async {
        let mut vector_results = Vec::new();

        let embed = match embedding_service.embed_query(&q).await {
            Ok(embed) => embed.iter().map(|&x| x as f64).collect(),
            Err(e) => {
                println!("Failed to embed query: {}", e);
                return vector_results;
            }
        };

        if let Ok(response) = SearchApiClient::new(Arc::new(Configuration::new()))
            .search(SearchRequest {
                table: "web".to_string(),
                knn: Some(Box::new(KnnQuery {
                    field: "vector".to_string(),
                    query_vector: Some(embed),
                    k: 20,
                    ..Default::default()
                })),
                limit: Some(20),
                ..Default::default()
            })
            .await
        {
            for hit in response.hits.and_then(|h| h.hits).unwrap_or_default() {
                if let Some(source) = hit._source.as_ref().cloned() {
                    if let Some(url) = source.get("url").and_then(|v| v.as_str()) {
                        vector_results.push((
                            url.to_string(),
                            process(&source, Some(&hit), &fields_to_show, &q, lang, loc),
                            hit._score,
                        ));
                    }
                }
            }
        }
        vector_results
    };

    let (exact_results, text_results, vector_results) =
        tokio::join!(exact_future, text_future, vector_future); // Combined call

    ////
    // Merge and Deduplicate results
    ////
    let mut seen_urls = AHashSet::with_capacity(100);
    let mut local_results = Vec::with_capacity(100);

    // Add exact results
    for (url, result) in exact_results {
        if seen_urls.insert(url) {
            local_results.push(result);
        }
    }

    // Add text results
    for (url, result) in text_results {
        if seen_urls.insert(url) {
            local_results.push(result);
        }
    }

    // Add vector results
    // Boost if url already seen: exact or text search
    for (url, result, _score) in vector_results {
        if seen_urls.insert(url.clone()) {
            local_results.push(result);
        } else {
            if let Some(index) = local_results
                .iter()
                .position(|r| r.get("url").and_then(|v| v.as_str()) == Some(&url))
            {
                if let Some(current_score) =
                    local_results[index].get("_score").and_then(|v| v.as_i64())
                {
                    local_results[index]
                        .as_object_mut()
                        .unwrap()
                        .insert("_score".to_string(), json!(current_score + 1000));
                    local_results[index]
                        .as_object_mut()
                        .unwrap()
                        .insert("_dual_search_boost".to_string(), json!(1000));
                }
            }
        }
    }

    // Remove HTTP duplicates of HTTPS URLs
    let mut to_remove = Vec::new();
    for (i, result) in local_results.iter().enumerate() {
        if let Some(url) = result.get("url").and_then(|v| v.as_str()) {
            if url.starts_with("http://") {
                let https_version = url.replacen("http://", "https://", 1);
                if local_results
                    .iter()
                    .any(|r| r.get("url").and_then(|v| v.as_str()) == Some(&https_version))
                {
                    to_remove.push(i);
                }
            }
        }
    }

    // Remove in reverse order to maintain indices
    for &i in to_remove.iter().rev() {
        local_results.remove(i);
    }

    // Sort by score descending
    local_results.sort_by(|a, b| {
        let score_a = a.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0);
        let score_b = b.get("_score").and_then(|s| s.as_f64()).unwrap_or(0.0);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ////
    // Calculate confidence
    ////
    if local_results.is_empty() {
        println!("No results → confidence = 0.0 (force fallback)");
        return (0.0, Vec::new());
    }

    let mut confidence_score = 0.0;
    // -------------- Language ---------------
    let mut incorrect_lang = 0;
    // ---------- Domain Diversity ----------
    let mut unique_domains: AHashSet<String> = AHashSet::with_capacity(local_results.len());
    for result in local_results.iter() {
        if lang != "all"
            && lang
                != result
                    .get("lang")
                    .unwrap_or_default()
                    .as_str()
                    .unwrap_or_default()
        {
            incorrect_lang += 1;
        }
        let domain = get_domain(
            result
                .get("url")
                .unwrap_or_default()
                .as_str()
                .unwrap_or_default(),
            true,
        );
        unique_domains.insert(domain);
    }
    let domain_ratio = unique_domains.len() as f64 / local_results.len() as f64;

    confidence_score += (domain_ratio.min(1.0)) * 0.4; // Weight: 0.4

    let correct_lang = local_results.len() - incorrect_lang;
    let lang_ratio = (correct_lang.min(10) as f64) / 10.0;
    confidence_score += lang_ratio.min(1.0) * 0.4;

    // ---------- Authority (Top Domains) ----------
    let mut top_domains_count = 0;
    for domain in &unique_domains {
        if TOP_DOMAINS.contains(domain.as_str()) {
            top_domains_count += 1;
        }
    }
    let authority_ratio = top_domains_count as f64 / local_results.len() as f64;
    // Weight: 0.3
    confidence_score += (authority_ratio.min(1.0)) * 0.3;

    // ---------- Score Drop Ratio ----------
    let scores: Vec<f64> = local_results
        .iter()
        .map(|r| r.get("_score").unwrap().as_f64().unwrap())
        .collect();

    let mut drop_score = 0.0;
    if scores.len() >= 4 {
        let mid = scores.len() / 2;
        let first_half_avg = scores[..mid].iter().sum::<f64>() / mid as f64;
        let second_half_avg = scores[mid..].iter().sum::<f64>() / (scores.len() - mid) as f64;
        let ratio = first_half_avg / second_half_avg;
        // Cap at 3.0 so extreme values don’t dominate
        drop_score = ratio.min(3.0) / 3.0;
    }
    // Weight: 0.3
    confidence_score += drop_score * 0.3;

    (confidence_score, local_results)
}
