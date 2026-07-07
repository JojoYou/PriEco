use crate::web::functions::ranking::meaning::{intent::intent, optimize::optimize};

pub fn process_query(query: &mut String, lang: &str, is_dir_hit: bool) {
    // Adds synonyms to the query for better coverage

    // Clasifies intent of the query
    // It's also used to decide if we want to show map, products...
    let intent = intent(&query, lang, is_dir_hit);

    // Optimizes query for faster FTS performance
    // If stops words are unnecessary it strips them
    optimize(query);
}
