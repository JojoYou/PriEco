//! Tests for COUNTRY_TO_LANG (ISO 3166 country code -> PriEco language code)

use std::collections::HashSet;

use prieco_core::COUNTRY_TO_LANG;

fn lang(cc: &str) -> Option<&'static str> {
    COUNTRY_TO_LANG.get(cc).copied()
}

/// Languages PriEco supports in the language selector / index
const SUPPORTED: &[&str] = &[
    "all", "ar", "bg", "cs", "da", "de", "el", "en", "es", "fi", "fr", "he", "hu", "id", "is",
    "it", "ja", "ko", "lt", "lv", "nl", "no", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sv", "tr",
];

#[test]
fn every_value_is_a_supported_language() {
    let supported: HashSet<&str> = SUPPORTED.iter().copied().collect();
    for (cc, l) in COUNTRY_TO_LANG.iter() {
        assert!(supported.contains(l), "{} -> {} is not supported", cc, l);
    }
}

#[test]
fn every_key_is_two_lowercase_ascii_letters() {
    for cc in COUNTRY_TO_LANG.keys() {
        assert_eq!(cc.len(), 2, "{}", cc);
        assert!(cc.chars().all(|c| c.is_ascii_lowercase()), "{}", cc);
    }
}

#[test]
fn map_is_reasonably_large() {
    assert!(COUNTRY_TO_LANG.len() > 80, "{}", COUNTRY_TO_LANG.len());
}

#[test]
fn german_speaking_countries() {
    for cc in ["de", "at", "ch"] {
        assert_eq!(lang(cc), Some("de"), "{}", cc);
    }
}

#[test]
fn english_speaking_countries() {
    for cc in ["us", "gb", "au", "nz", "ie"] {
        assert_eq!(lang(cc), Some("en"), "{}", cc);
    }
}

#[test]
fn south_asia_maps_to_english() {
    for cc in ["in", "pk", "bd", "np", "lk"] {
        assert_eq!(lang(cc), Some("en"), "{}", cc);
    }
}

#[test]
fn african_countries_map_to_english() {
    for cc in ["ng", "za", "ke", "gh", "tz", "ug", "zm", "zw", "mw"] {
        assert_eq!(lang(cc), Some("en"), "{}", cc);
    }
}

#[test]
fn spanish_speaking_countries() {
    for cc in ["es", "mx", "cl", "co", "pe"] {
        assert_eq!(lang(cc), Some("es"), "{}", cc);
    }
}

#[test]
fn portuguese_speaking_countries() {
    for cc in ["pt", "br"] {
        assert_eq!(lang(cc), Some("pt"), "{}", cc);
    }
}

#[test]
fn arabic_world() {
    for cc in [
        "sa", "ae", "eg", "dz", "ma", "tn", "ye", "om", "iq", "jo", "ps", "kw", "qa", "bh", "ly",
        "sd", "sy",
    ] {
        assert_eq!(lang(cc), Some("ar"), "{}", cc);
    }
}

#[test]
fn former_ussr_maps_to_russian() {
    for cc in ["ru", "uz", "kz", "kg", "tj", "by", "ua", "am", "az", "ge"] {
        assert_eq!(lang(cc), Some("ru"), "{}", cc);
    }
}

#[test]
fn chinese_speaking_regions_fall_back_to_all() {
    for cc in ["sg", "hk", "mo", "tw", "cn"] {
        assert_eq!(lang(cc), Some("all"), "{}", cc);
    }
}

#[test]
fn canada_falls_back_to_all() {
    assert_eq!(lang("ca"), Some("all"));
}

#[test]
fn closest_language_mappings() {
    assert_eq!(lang("cy"), Some("el"));
    assert_eq!(lang("il"), Some("he"));
}

#[test]
fn simple_one_to_one_mappings() {
    for (cc, l) in [
        ("fr", "fr"),
        ("it", "it"),
        ("is", "is"),
        ("id", "id"),
        ("lv", "lv"),
        ("lt", "lt"),
        ("hu", "hu"),
        ("nl", "nl"),
        ("no", "no"),
        ("pl", "pl"),
        ("ro", "ro"),
        ("sk", "sk"),
        ("fi", "fi"),
        ("tr", "tr"),
        ("bg", "bg"),
    ] {
        assert_eq!(lang(cc), Some(l), "{}", cc);
    }
}

#[test]
fn unknown_codes_are_none() {
    for cc in ["", "zz", "xx", "usa", "e", "123"] {
        assert_eq!(lang(cc), None, "{:?}", cc);
    }
}

#[test]
fn lookup_is_case_sensitive() {
    assert_eq!(lang("DE"), None);
    assert_eq!(lang("De"), None);
}

/// BUG: "ar" is inserted twice: first as Argentina -> "es", later as Arabic -> "ar".
/// The second insert silently overwrites Argentina, so Argentinians get Arabic results.
#[test]
fn argentina_maps_to_spanish() {
    assert_eq!(lang("ar"), Some("es"));
}

/// BUG: Sweden ("se") is mapped to Dutch ("nl") although Swedish ("sv") is supported.
#[test]
fn sweden_maps_to_swedish() {
    assert_eq!(lang("se"), Some("sv"));
}

/// BUG: several keys are *language* codes instead of ISO 3166 *country* codes, so the
/// real countries are missing: Czechia is "cz" (not "cs"), Greece "gr" (not "el"),
/// Slovenia "si" (not "sl"), Denmark "dk" (not "da"), Japan "jp" (not "ja"),
/// South Korea "kr" (not "ko"), Serbia "rs" (not "sr", which is Suriname),
/// Sweden "se" (not "sv", which is El Salvador).
#[test]
fn real_iso_country_codes_are_mapped() {
    for (cc, l) in [
        ("cz", "cs"),
        ("gr", "el"),
        ("si", "sl"),
        ("dk", "da"),
        ("jp", "ja"),
        ("kr", "ko"),
        ("rs", "sr"),
    ] {
        assert_eq!(lang(cc), Some(l), "country {} should map to {}", cc, l);
    }
}

/// BUG: consequences of the language/country mix up above: these are real
/// countries whose official language is NOT what the map returns.
#[test]
fn language_codes_used_as_country_codes_map_to_wrong_countries() {
    assert_ne!(lang("sv"), Some("sv"), "\"sv\" is El Salvador (Spanish), not Sweden");
    assert_ne!(lang("sr"), Some("sr"), "\"sr\" is Suriname (Dutch), not Serbia");
    assert_ne!(lang("sl"), Some("sl"), "\"sl\" is Sierra Leone (English), not Slovenia");
}
