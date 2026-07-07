use ahash::AHashSet;
use once_cell::sync::Lazy;

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

pub fn optimize(query: &mut String) {
    if query.contains('"')
        || query.contains("def")
        || query.contains("definition")
        || query.contains(" AND ")
        || query.contains(" OR ")
        || query.starts_with('-')
    {
        return;
    }

    let terms: Vec<&str> = query.split_whitespace().collect();
    let mut filtered_terms: Vec<&str> = Vec::with_capacity(terms.len());

    for term in &terms {
        if !STOPWORDS.contains(term.to_lowercase().as_str()) {
            filtered_terms.push(*term);
        }
    }

    if filtered_terms.is_empty() {
        return;
    }

    *query = filtered_terms.join(" ");
}
