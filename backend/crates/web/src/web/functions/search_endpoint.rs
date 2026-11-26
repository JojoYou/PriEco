//!  File: web/modules/search_endpoint.rs
//!  Description: Decided what to do with the search query
//!
//!  Author: Roman Lancos <support@prieco.net>
//!  License: AGPL v3.0
//!
//!  Date Created: 2025-09-20
//!  Last Modified: 2026-02-06
//!
//!  Usage Call this to get results in Rocket template format (context that gets inserted to the template)
//!  TODO:

/*
  Import system libraries
*/
use std::{collections::HashMap, sync::Arc};

/*
  Import external libraries
*/
use rocket::{State, http::CookieJar};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use urlencoding::decode;

/*
  Import own libraries
*/
use crate::web::functions::{
    additional::spell_check::spell_check_query,
    general::get_domain,
    ranking::goggles::GoggleRules,
    remote_api::{currency::get_fx_widget, img, news, video},
    search_db::{self, SearchDBParams},
};
use prieco_core::{EmbeddingService, QueryIntent, SearchResult, Sitelink, url_to_domain_id};

/// Description: Decides what kind of search to perform
/// Input: Search type, Search query, Language, Location, Embedding service, Cookie jar
/// Output: Search results in Rocket template format
pub struct EndParams<'r> {
    pub t: &'r str,
    pub q: &'r str,
    pub lang: &'r str,
    pub loc: &'r str,
    pub embedding_service: &'r State<EmbeddingService>,
    pub goggles: Vec<Arc<GoggleRules>>,
    pub user_qt_prefs: &'r UserQtPrefs,
    pub mobile: bool,
    pub decentralize: bool,
    pub safe_search: bool,
}
pub async fn run(params: EndParams<'_>) -> HashMap<String, Value> {
    // Don't perform a search on bang
    if params.q.contains("!") {
        return HashMap::new();
    }

    let mut context: HashMap<String, Value> = HashMap::with_capacity(100);

    println!("Type: {}", params.t);
    context.insert(String::from("type"), json!(params.t));

    match params.t {
        "img" => {
            context.insert(String::from("img_results"), json!(true));
            context.insert(
                String::from("images"),
                json!(&img::run(&params.q.to_lowercase().replace(" ", "+")).await),
            );
        }
        "new" => {
            context.insert(String::from("new_results"), json!(true));
            let n = news::run(params.q, params.lang, 50).await;
            match n.is_empty() {
                false => context.insert(String::from("news"), json!(&n)),
                true => context.insert(
                    String::from("news"),
                    json!([{
                        "title": "No news found",
                        "description":format!("We know nothing new about {}",params. q),
                        "url": "",
                        "domain": "PriEco",
                        "favicon": "",
                        "image": ""
                    }]),
                ),
            };
        }
        "map" => {}
        _ => {
            all_search(AllParams {
                context: &mut context,
                q: params.q,
                lang: params.lang,
                loc: params.loc,
                embedding_service: params.embedding_service,
                goggles: params.goggles,
                user_qt_prefs: params.user_qt_prefs,
                mobile: params.mobile,
                decentralize: params.decentralize,
                safe_search: params.safe_search,
            })
            .await;
        }
    }

    context
}

#[derive(Deserialize, Serialize, Default)]
pub struct UserQtPrefs {
    #[serde(default)]
    pub boost: Vec<String>,
    #[serde(default)]
    pub downrank: Vec<String>,
    #[serde(default)]
    pub discard: Vec<String>,
}

#[derive(Serialize)]
pub struct QtDomainDisplay {
    pub domain: String,
    pub is_boost: bool,
    pub is_downrank: bool,
    pub is_discard: bool,
}
impl UserQtPrefs {
    pub fn into_goggle_rules(&self) -> GoggleRules {
        let mut rules = GoggleRules::default();

        for d in &self.boost {
            rules
                .boost
                .insert(url_to_domain_id(&format!("https://{}/", d)), 3.0);
            if !d.starts_with("www.") {
                rules
                    .boost
                    .insert(url_to_domain_id(&format!("https://www.{}/", d)), 3.0);
            }
        }

        for d in &self.downrank {
            rules
                .downrank
                .insert(url_to_domain_id(&format!("https://{}/", d)), 3.0);
            if !d.starts_with("www.") {
                rules
                    .downrank
                    .insert(url_to_domain_id(&format!("https://www.{}/", d)), 3.0);
            }
        }

        for d in &self.discard {
            rules
                .discard
                .insert(url_to_domain_id(&format!("https://{}/", d)));
            if !d.starts_with("www.") {
                rules
                    .discard
                    .insert(url_to_domain_id(&format!("https://www.{}/", d)));
            }
        }

        rules.discard_by_default = false;

        rules
    }
}

pub fn get_user_qt_prefs(cookie_jar: &CookieJar<'_>) -> UserQtPrefs {
    cookie_jar
        .get("prieco_qt_prefs")
        .and_then(|c| {
            let decoded = decode(c.value()).ok()?;
            serde_json::from_str(&decoded).ok()
        })
        .unwrap_or_default()
}

pub struct AllParams<'r> {
    pub context: &'r mut HashMap<String, Value>,
    pub q: &'r str,
    pub lang: &'r str,
    pub loc: &'r str,
    pub embedding_service: &'r State<EmbeddingService>,
    pub goggles: Vec<Arc<GoggleRules>>,
    pub user_qt_prefs: &'r UserQtPrefs,
    pub mobile: bool,
    pub decentralize: bool,
    pub safe_search: bool,
}
async fn all_search(mut params: AllParams<'_>) {
    // Spell check
    if let Some(suggestion) = spell_check_query(params.q) {
        params
            .context
            .insert(String::from("did_you_mean"), json!(suggestion));
    }

    params
        .context
        .insert(String::from("all_results"), json!(true)); // Set btn search type

    let mut results_vec: Vec<SearchResult> = Vec::with_capacity(100);

    let user_rules = params.user_qt_prefs.into_goggle_rules();
    params.goggles.push(Arc::new(user_rules));

    let query_intent = search_db::query_intent(params.q, params.lang, params.loc);
    let videos_enable = matches!(
        query_intent,
        QueryIntent::Informational
            | QueryIntent::CommercialInvestigation
            | QueryIntent::Navigational
            | QueryIntent::Unknown
    );
    let news_enable = matches!(
        query_intent,
        QueryIntent::Informational | QueryIntent::Navigational | QueryIntent::Unknown
    );
    let (q, lang, loc) = (params.q, params.lang, params.loc);

    let (_, videos, news) = tokio::join!(
        search_db::run(SearchDBParams {
            results: &mut results_vec,
            query: params.q,
            lang: params.lang,
            loc: params.loc,
            embedding_service: params.embedding_service,
            goggles: params.goggles,
            mobile: params.mobile,
            decentralize: params.decentralize,
            safe_search: params.safe_search,
            intent: query_intent,
            limit: 40,
        }), // Search database: Modify results + return confidence score
        async move {
            if videos_enable {
                video::run(q, lang, loc, 10).await
            } else {
                Vec::new()
            }
        },
        async move {
            if news_enable {
                news::run(q, lang, 10).await
            } else {
                Vec::new()
            }
        }
    );

    let mut results_vec = merge_sitelinks(results_vec);
    results_vec.truncate(20);

    if !videos.is_empty() {
        params.context.insert(String::from("videos"), json!(videos));
        params.context.insert(
            String::from("videos_at"),
            json!(
                results_vec
                    .len()
                    .clamp(1, if news.is_empty() { 2 } else { 4 })
                    - 1
            ),
        );
    }

    if !news.is_empty() {
        params.context.insert(String::from("news"), json!(news));
        params.context.insert(
            String::from("news_at"),
            json!(results_vec.len().clamp(1, 2) - 1),
        );
    }

    // QUICK TUNE DOMAIN EXTRACTION
    let mut unique_domains = std::collections::HashSet::new();
    let mut qt_domains: Vec<QtDomainDisplay> = Vec::new();

    for res in &results_vec {
        let domain = get_domain(&res.url, true);

        if unique_domains.insert(domain.clone()) {
            qt_domains.push(QtDomainDisplay {
                domain: domain.clone(),
                is_boost: params.user_qt_prefs.boost.contains(&domain),
                is_downrank: params.user_qt_prefs.downrank.contains(&domain),
                is_discard: params.user_qt_prefs.discard.contains(&domain),
            });
        }
    }

    let mut saved_qt_domains: Vec<QtDomainDisplay> = Vec::new();

    let mut add_saved = |domain: &String, is_boost: bool, is_downrank: bool, is_discard: bool| {
        if !unique_domains.contains(domain) {
            saved_qt_domains.push(QtDomainDisplay {
                domain: domain.clone(),
                is_boost,
                is_downrank,
                is_discard,
            });
            unique_domains.insert(domain.clone());
        }
    };

    for d in &params.user_qt_prefs.boost {
        add_saved(d, true, false, false);
    }
    for d in &params.user_qt_prefs.downrank {
        add_saved(d, false, true, false);
    }
    for d in &params.user_qt_prefs.discard {
        add_saved(d, false, false, true);
    }

    params
        .context
        .insert(String::from("qt_domains_count"), json!(qt_domains.len()));
    params
        .context
        .insert(String::from("qt_domains"), json!(qt_domains));
    params
        .context
        .insert(String::from("saved_qt_domains"), json!(saved_qt_domains));

    if let Some(fx_widget) = get_fx_widget(params.q).await {
        params
            .context
            .insert(String::from("currency_widget"), json!(fx_widget));
    }

    // If PriEco confidence is too low, use other indexes too
    /*if !cookie_jar.get("index").is_some() && index_confidence < 0.95 {
        let mut mixed_results = Vec::with_capacity(200);
        let mut seen_urls: AHashSet<String> = AHashSet::with_capacity(100);

        let prieco_urls: AHashSet<String> = results_vec
            .iter()
            .map(|result| result.url.clone())
            .collect(); // Extract urls from PriEco index to show them when PriEco + Remote is a duplicated result

        // Call external result provider
        let remote_results: Vec<SearchResult> = loop {
            if let Some(res) = all::run(&q.to_lowercase().replace(" ", "+"), &lang, "us").await {
                break res;
            }
        };

        let mut db_iter = results_vec.into_iter();
        let mut remote_iter = remote_results.into_iter();

        // Interleave results: 2 Remote, 1 PriEco, repeat
        loop {
            let mut added_any = false;
            for _ in 0..2 {
                if let Some(google_result) = remote_iter.next() {
                    if !prieco_urls.contains(&google_result.url)
                        && seen_urls.insert(google_result.url.clone())
                    {
                        mixed_results.push(google_result);
                        added_any = true;
                    }
                }
            }
            if let Some(db_result) = db_iter.next() {
                if seen_urls.insert(db_result.url.clone()) {
                    mixed_results.push(db_result);
                    added_any = true;
                }
            }
            if !added_any {
                break;
            }
        }

        results_vec = mixed_results; // Rewrite results
    }
    // Confidence is enought + pure PriEco isn't selected
    else if !cookie_jar.get("index").is_some() {
        set_cookie(
            cookie_jar,
            String::from("prieco_searches"),
            (cookie_jar
                .get("prieco_searches")
                .and_then(|c| c.value().parse::<u64>().ok())
                .unwrap_or(0)
                + 1)
            .to_string(),
            true,
            false,
        );
    }

    // Web search was made, increment cookie counter
    if !cookie_jar.get("index").is_some() {
        set_cookie(
            cookie_jar,
            String::from("all_searches"),
            (cookie_jar
                .get("all_searches")
                .and_then(|c| c.value().parse::<u64>().ok())
                .unwrap_or(0)
                + 1)
            .to_string(),
            true,
            false,
        );
    }*/

    params
        .context
        .insert(String::from("results"), json!(&results_vec));

    /*
      External APIs
    */
    /*let (yadore_result, news_result) =
        tokio::join!(yadore::run(q, loc), news::run(q, lang, loc, 20));

    // Yadore Ads
    if let Ok(yadore_data) = yadore_result {
        context.insert(String::from("yadore"), json!(&yadore_data));
    }
    */
}

fn merge_sitelinks(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut merged: Vec<SearchResult> = Vec::with_capacity(results.len());
    let mut by_domain: HashMap<String, usize> = HashMap::new();

    for res in results {
        let parent = if res.domain.is_empty() {
            None
        } else {
            by_domain.get(&res.domain).copied()
        };

        match parent {
            Some(i) => {
                let title = if res.title.is_empty() || res.title == merged[i].title {
                    res.display_url
                        .rsplit('›')
                        .next()
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                } else {
                    res.title
                };
                merged[i].sitelinks.push(Sitelink {
                    title,
                    url: res.url,
                });
            }
            None => {
                if !res.domain.is_empty() {
                    by_domain.insert(res.domain.clone(), merged.len());
                }
                merged.push(res);
            }
        }
    }

    merged
}
