//! # PriEco Mini Crawler
//!
//! Node side of crawling. Gets URLs from main crawler, downloads and parses them
//! and sends the results back.
//!
//! * [mini_crawler] - Worker loop, fetch and upload.
//! * [download] - Page download.
//! * [meta] - Title, description, language, country, NSFW.
//! * [tag_data] - Text, meta tags and links from html.

pub mod download;
pub mod meta;
pub mod mini_crawler;
pub mod tag_data;
