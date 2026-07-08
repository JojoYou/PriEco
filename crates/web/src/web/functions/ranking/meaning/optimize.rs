use ahash::AHashSet;
use once_cell::sync::Lazy;
use prieco_core::{LOCAL_NO_EXPAND, QueryIntent, get_store};

static STOPWORDS: Lazy<AHashSet<&'static str>> = Lazy::new(|| {
    let mut set = AHashSet::new();
    let words = [
        "a", "an", "the", "in", "on", "of", "to", "for", "is", "are", "with", "and", "or", "what",
        "how", "why", "when", "at", "by",
    ];
    for w in words.iter() {
        set.insert(*w);
    }
    set
});

pub fn synynyms_and_optimize(query: &mut String, lang: &str, intent: &QueryIntent) {
    if query.contains('"')
        || query.contains("def")
        || query.contains("definition")
        || query.contains(" AND ")
        || query.contains(" OR ")
        || query.starts_with('-')
    {
        return;
    }

    let store = get_store(lang);

    let lower = query.to_lowercase();
    let mut groups: Vec<String> = Vec::new();

    for term in lower.split_whitespace() {
        if STOPWORDS.contains(term) {
            continue;
        }

        // Local intent Bypass
        if intent == &QueryIntent::Local && LOCAL_NO_EXPAND.contains(term) {
            groups.push(format!("\"{}\"^2.0", term));
            continue;
        }

        // Synonym Expansion
        match store.as_ref().and_then(|s| s.lookup(term)) {
            Some(synonyms) if !synonyms.is_empty() => {
                let mut parts = vec![format!("\"{}\"^2.0", term)];
                parts.extend(synonyms.iter().take(2).map(|s| format!("\"{}\"^1.0", s)));
                groups.push(format!("({})", parts.join(" OR ")));
            }
            _ => {
                groups.push(format!("\"{}\"^2.0", term));
            }
        }
    }

    if !groups.is_empty() {
        *query = groups.join(" ");
    }
}
