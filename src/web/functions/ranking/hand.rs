use prieco_core::WebDocument;

pub fn run(results: &mut Vec<WebDocument>, query: &str, lang: &str, loc: &str) {
    // Calculate custom ranking boost
    for doc in results.iter_mut() {
        let clean_url = strip_url_noise(&doc.url);

        let mut boost: f64 = 1.0; // Multiplicative base

        // (https://www/)query(.tld/)
        let domain_root = clean_url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|d| d.strip_prefix("www.").or(Some(d)))
            .and_then(|d| d.split('.').next())
            .unwrap_or("");
        if domain_root.eq_ignore_ascii_case(query) && clean_url.matches('/').count() <= 3 {
            boost *= 2.2;
        }

        // Homepage
        if is_effectively_homepage(clean_url) {
            boost *= 1.4;
        }

        // Language
        if doc.lang == lang {
            boost *= 1.4;
        }

        // Location
        if doc.loc == loc {
            boost *= 1.2;
        }
        let tld_matches_loc = clean_url
            .split("://")
            .nth(1)
            .and_then(|s| s.trim_end_matches('/').split('.').last())
            .map_or(false, |tld| tld.eq_ignore_ascii_case(loc));
        if tld_matches_loc {
            boost *= 1.2;
        }

        // Wikipedia authority signal
        if clean_url.contains(".wikipedia.org/wiki/") {
            boost *= 1.3;
        }

        // SSL
        if clean_url.starts_with("https://") {
            boost *= 1.05;
        }

        // TLD quality
        if clean_url.contains(".dev") || clean_url.contains(".com") {
            boost *= 1.04;
        } else if clean_url.contains(".org") || clean_url.contains(".net") {
            boost *= 1.02;
        }

        // Load speed
        boost *= match doc.load {
            l if l < 0.3 => 1.08,
            l if l < 0.5 => 1.05,
            l if l < 2.0 => 1.02,
            l if l < 3.0 => 1.01,
            _ => 1.0,
        };

        // Bad URL patterns
        if clean_url.contains('_') || clean_url.contains(",,") || clean_url.contains(':') {
            boost *= 0.9;
        }

        boost *= 1.0 + sigmoid(doc.confidence as f64 / 200.0) * 0.15;
        boost *= 1.0 + sigmoid(doc.effort as f64 / 200.0) * 0.08;

        doc.search_score = (doc.search_score as f64 * boost) as f32;
    }

    results.sort_by(|a, b| {
        b.search_score
            .partial_cmp(&a.search_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/* Helper functions */
fn strip_url_noise(url: &str) -> &str {
    let url = url.split('?').next().unwrap_or(url);
    let url = url.split('#').next().unwrap_or(url);
    url
}
fn is_effectively_homepage(url: &str) -> bool {
    let path = url
        .split("://")
        .nth(1)
        .unwrap_or("")
        .trim_end_matches('/')
        .splitn(2, '/')
        .nth(1) // everything after the domain
        .unwrap_or("");

    if path.is_empty() {
        return true;
    }

    // Match locale-only paths: en, en-CA, fr-FR, zh-Hans, pt-BR etc.
    let is_locale = |seg: &str| -> bool {
        let seg = seg.trim_end_matches('/');
        matches!(seg.len(), 2..=7)
            && seg.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
            && seg.contains(|c: char| c.is_ascii_alphabetic()) // not just dashes
    };

    // Single locale segment: /en-CA/ or /en/
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    matches!(segments.as_slice(), [seg] if is_locale(seg))
}
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
