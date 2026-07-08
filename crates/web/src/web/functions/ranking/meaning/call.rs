use prieco_core::QueryIntent;

use crate::web::functions::ranking::meaning::{intent::intent, optimize::synynyms_and_optimize};

pub fn process_query(query: &mut String, lang: &str) -> QueryIntent {
    // Clasifies intent of the query
    // It's also used to decide if we want to show map, products...
    let intent = intent(&query, lang);

    // Adds synonyms to the query for better coverage
    // Optimizes query for faster FTS performance
    // If stops words are unnecessary it strips them
    synynyms_and_optimize(query, lang, &intent);

    intent
}
