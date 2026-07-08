use prieco_core::QueryIntent;

use crate::web::functions::ranking::meaning::{
    entities::scan_entities, intent::intent, optimize::synynyms_and_optimize,
};

pub fn process_query(query: &mut String, lang: &str) -> (QueryIntent, Option<(f32, f32)>) {
    // Clasifies intent of the query
    // It's also used to decide if we want to show map, products...
    let possible_intent = intent(&query, lang);

    // Entities scan
    // Clarifies intent
    // If query contains a place extract its coords
    let (intent, coords, tags) = scan_entities(query, possible_intent);

    // Adds synonyms to the query for better coverage
    // Optimizes query for faster FTS performance
    // If stops words are unnecessary it strips them
    synynyms_and_optimize(query, lang, &intent, &tags);

    (intent, coords)
}
