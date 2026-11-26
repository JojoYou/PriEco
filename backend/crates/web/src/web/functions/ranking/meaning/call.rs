//! # Query Understanding
//!
//! Runs query understanding in order.
//!
//! 1. [**Intent**:][intent] Keywords decide what user wants. Also decides if we show map, products...
//! 2. [**Entities**:][scan_entities] Places, businesses and names clarify intent, places give coordinates.
//! 3. [**Synonyms**:][synynyms_and_optimize] Adds synonyms, strips stop words, builds Tantivy query.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-07-07
//! * Last Modified: 2026-11-21
//!
//! ## Planned Improvements
//!
//! - [ ] None

use prieco_core::QueryIntent;

use crate::web::functions::ranking::meaning::{
    entities::scan_entities, intent::intent, optimize::synynyms_and_optimize,
};

/// Understands query and rewrites it for full text search.
///
/// # Arguments
///
/// * `query` - Query, replaced by the Tantivy query.
/// * `lang` - Language.
/// * `loc` - Location, picks the right place when more have the same name.
///
/// # Returns
///
/// (intent, (lat, lon) of a place in query).
pub fn process_query(
    query: &mut String,
    lang: &str,
    loc: &str,
) -> (QueryIntent, Option<(f32, f32)>) {
    // Clasifies intent of the query
    // It's also used to decide if we want to show map, products...
    let possible_intent = intent(query, lang);

    // Entities scan
    // Clarifies intent
    // If query contains a place extract its coords
    let (intent, coords, tags) = scan_entities(query, possible_intent, loc);

    // Adds synonyms to the query for better coverage
    // Optimizes query for faster FTS performance
    // If stops words are unnecessary it strips them
    synynyms_and_optimize(query, lang, &intent, &tags);

    (intent, coords)
}
