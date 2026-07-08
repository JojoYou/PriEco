use ahash::AHashSet;
use once_cell::sync::Lazy;
use prieco_core::{EntityType, LOCAL_NO_EXPAND, QueryIntent, TaggedEntity, get_store};

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

pub fn synynyms_and_optimize(
    query: &mut String,
    lang: &str,
    intent: &QueryIntent,
    tags: &[TaggedEntity],
) {
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
    let mut groups: Vec<String> = Vec::new();
    let mut last_idx = 0;

    let process_standard_text = |text: &str, target_groups: &mut Vec<String>| {
        for term in text.split_whitespace() {
            let term_lower = term.to_lowercase();

            if STOPWORDS.contains(term_lower.as_str()) {
                continue;
            }

            if *intent == QueryIntent::Local && LOCAL_NO_EXPAND.contains(term_lower.as_str()) {
                target_groups.push(format!("\"{}\"^2.0", term));
                continue;
            }

            match store.as_ref().and_then(|s| s.lookup(&term_lower)) {
                Some(synonyms) if !synonyms.is_empty() => {
                    let mut parts = vec![format!("\"{}\"^2.0", term)];
                    parts.extend(synonyms.iter().take(2).map(|s| format!("\"{}\"^1.0", s)));
                    target_groups.push(format!("({})", parts.join(" OR ")));
                }
                _ => {
                    target_groups.push(format!("\"{}\"^2.0", term));
                }
            }
        }
    };

    let mut sorted_tags = tags.to_vec();
    sorted_tags.sort_by_key(|t| t.range.start);

    for tag in sorted_tags {
        if tag.range.start > last_idx {
            process_standard_text(&query[last_idx..tag.range.start], &mut groups);
        }

        let entity_text = &query[tag.range.clone()];
        match tag.entity_type {
            EntityType::Business => groups.push(format!("\"{}\"^3.0", entity_text)),
            EntityType::Place => groups.push(format!("\"{}\"^2.5", entity_text)),
            EntityType::PersonName => groups.push(format!("\"{}\"^2.5", entity_text)),
        }

        last_idx = tag.range.end;
    }

    if last_idx < query.len() {
        process_standard_text(&query[last_idx..], &mut groups);
    }

    if !groups.is_empty() {
        *query = groups.join(" ");
    }
}
