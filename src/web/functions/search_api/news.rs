/*
  File: web/modules/search_api/news.rs
  Description: Manages PriEco News

  Author: Roman Lancos <support@jojoyou.org>
  License: AGPL v3.0

  Date Created: 2026-04-14
  Last Modified: 2026-04-14

  Usage Call this to get news results
  TODO:
*/

/*
  Import system libraries
*/

/*
  Import external libraries
*/
use dotenv_codegen::dotenv;
use reqwest::Client;
use rocket::form::validate::Len;
use serde::{Deserialize, Serialize};

/*
  Import own libraries
*/

/*
  Constants
*/
const BASE_URL: &str = "https://api.currentsapi.services/v1/search";

/*
  Structures
*/
#[derive(Debug, Deserialize, Serialize)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub author: String,
    pub image: String,
    pub language: String,
    pub category: Vec<String>,
    pub published: String,
}

#[derive(Debug, Deserialize)]
pub struct NewsResponse {
    pub status: String,
    pub page: Option<u32>,
    pub news: Vec<Article>,
}

pub async fn run(
    query: &str,
    lang: &str,
    loc: &str,
    count: u32,
) -> Result<Vec<Article>, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();

    let final_lang = if lang == "all" { "en" } else { lang };
    let final_loc = if loc == "all" {
        String::from("US")
    } else {
        loc.to_uppercase()
    };

    let resp = client
        .get(BASE_URL)
        .query(&[
            ("keywords", query),
            ("language", final_lang),
            ("country", final_loc.as_str()), // .as_str() keeps the array types uniform
            ("page_number", "1"),
            ("page_size", &count.to_string()),
            ("apiKey", dotenv!("NEWS_API_KEY")),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<NewsResponse>()
        .await?;

    if resp.status != "ok" {
        return Err(format!("News API returned non-ok status: {}", resp.status).into());
    }

    println!("News: {}", resp.news.len());

    Ok(resp.news)
}
