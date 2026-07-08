use prieco_core::{MATCHERS, QueryIntent};

pub fn intent(query: &str, lang: &str) -> QueryIntent {
    let lower_query = query.to_lowercase();

    let target_lang = if MATCHERS.contains_key(lang) {
        lang
    } else {
        "en"
    };

    if let Some(matcher) = MATCHERS.get(target_lang) {
        if let Some(mat) = matcher.automaton.find(&lower_query) {
            return matcher.intent_map[mat.pattern().as_usize()];
        }
    }

    let token_count = lower_query.split_whitespace().count();
    if token_count >= 5 {
        QueryIntent::Informational
    } else if token_count <= 2 && !lower_query.is_empty() {
        QueryIntent::Navigational
    } else {
        QueryIntent::Unknown
    }
}
