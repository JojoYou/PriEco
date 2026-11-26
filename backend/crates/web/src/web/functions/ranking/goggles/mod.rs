//! # Goggles
//!
//! Rule lists that change ranking: raise, lower or discard domains and paths.
//!
//! ## Format
//!
//! ```text
//! ! name: My Goggle
//! ! description: What it does
//! ! author: Me
//! ! public: true
//! ! avatar: #14141E
//! $discard
//! $boost=3,site=example.com
//! $downrank=2,site=example.org
//! $discard,site=example.net
//! $boost=2,important,site=example.edu
//! /docs/$boost=1
//! ```
//!
//! * `$discard` alone discards every domain the goggle doesn't mention.
//! * `important` makes this goggle's rule the only one for the domain.
//! * Line without `site=` boosts URLs matching the path pattern.
//!
//! ## Parts
//!
//! * [fetch] - Download, refresh.
//! * [parse] - Text to [GoggleRules].
//! * [storage] - Goggles keyspace.
//! * [resolve] - Active goggles of a request.
//! * [types] - [Goggle], [GoggleRules].
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-07-12
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

mod fetch;
mod parse;
mod resolve;
mod storage;
mod types;

pub use fetch::{fetch_and_store, refresh_stale_goggles};
pub use parse::{domain_str_to_id, parse_goggle};
pub use resolve::{get_goggle_ids, load_goggles};
pub use storage::{delete, get, list_all, list_public, put, touch_fetched_at};
pub use types::{Goggle, GoggleRules};
