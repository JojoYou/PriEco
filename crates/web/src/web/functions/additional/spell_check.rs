use prieco_core::SPELL_CHECKER;

pub fn spell_check_query(query: &str) -> String {
    let suggestions = SPELL_CHECKER.lookup_compound(query, 2);

    suggestions
        .first()
        .map(|s| s.term.clone())
        .unwrap_or_else(|| query.to_string())
}
