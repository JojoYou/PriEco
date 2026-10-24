//! Tests for src/web/functions/search_api/currency.rs
//!
//! Parsing failures return before any storage/network access. Successful parses are
//! served from a pre-filled widget cache, so no request ever leaves the machine.

use std::time::{SystemTime, UNIX_EPOCH};

use prieco_core::PRIECO_META;
use serde_json::json;

use crate::support::init;
use crate::web::functions::search_api::currency::{FxWidget, get_fx_widget};

fn widget(from: &str, to: &str, amount: f64, rate: f64) -> serde_json::Value {
    json!({
        "amount": amount, "from": from, "to": to, "rate": rate,
        "converted": (amount * rate * 100.0).round() / 100.0,
        "date": "2026-01-01", "svg": "M 0,40 L 260,40",
        "min_label": "1.0000", "max_label": "1.0000", "mid_label": "1.0000",
        "start_date": "2025-12-01", "history_json": "[]"
    })
}

/// Pre-fills the widget cache so get_fx_widget answers without a network call
fn cache(from: &str, to: &str, amount: f64, rate: f64) {
    init();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let key = format!("fx:{}:{}:{}", from, to, amount);
    let v = json!({ "timestamp": now, "data": widget(from, to, amount, rate) });
    PRIECO_META
        .widgets_ks
        .insert(key.as_bytes(), serde_json::to_vec(&v).unwrap())
        .unwrap();
}

async fn fx(q: &str) -> Option<FxWidget> {
    init();
    get_fx_widget(q).await
}

/* ───────────────────────────── not a conversion ───────────────────────────── */

#[tokio::test]
async fn non_currency_queries_are_none() {
    for q in [
        "",
        "   ",
        "hello",
        "usd to eur",
        "10 to eur",
        "10 usd",
        "10 usd eur",
        "ten usd to eur",
        "1,000 usd to eur",
        "-5 usd to eur",
        "1e3 usd to eur",
        "10.5.3 usd to eur",
        "10 usd to eur please",
        "how much is 10 usd to eur",
        "10 usd into eur",
        "10 usd to eur2",
    ] {
        assert!(fx(q).await.is_none(), "{:?}", q);
    }
}

#[tokio::test]
async fn unknown_currencies_are_none() {
    for q in ["10 abc to eur", "10 usd to xyz", "10 btc to usd", "10 usdt to eur", "10 dollarz to eur", "10 e to usd"] {
        assert!(fx(q).await.is_none(), "{:?}", q);
    }
}

#[tokio::test]
async fn same_currency_is_none() {
    for q in ["10 usd to usd", "10 dollars to $", "5 € to euro", "1 EUR = eur", "3 yuan to rmb", "2 pounds to gbp"] {
        assert!(fx(q).await.is_none(), "{:?}", q);
    }
}

/* ───────────────────────────── cached conversions ───────────────────────────── */

#[tokio::test]
async fn cached_conversion_is_returned() {
    cache("USD", "EUR", 11.0, 0.9);
    let w = fx("11 usd to eur").await.unwrap();
    assert_eq!(w.from, "USD");
    assert_eq!(w.to, "EUR");
    assert_eq!(w.amount, 11.0);
    assert_eq!(w.rate, 0.9);
    assert_eq!(w.converted, 9.9);
}

#[tokio::test]
async fn query_syntax_variants_hit_the_same_cache_entry() {
    cache("USD", "EUR", 12.0, 0.5);
    for q in [
        "12 usd to eur",
        "12 USD TO EUR",
        "convert 12 usd to eur",
        "CONVERT 12 usd in eur",
        "12usd=eur",
        "12 $ to €",
        "12 dollars in euros",
        "12.0 usd to eur",
        "  12 usd to eur  ",
    ] {
        let w = fx(q).await.unwrap_or_else(|| panic!("{:?} not parsed", q));
        assert_eq!(w.rate, 0.5, "{}", q);
    }
}

#[tokio::test]
async fn currency_aliases() {
    let pairs = [
        ("13 pounds to yen", "GBP", "JPY"),
        ("13 sterling to rupees", "GBP", "INR"),
        ("13 won to lira", "KRW", "TRY"),
        ("13 hryvnia to shekel", "UAH", "ILS"),
        ("13 nis to baht", "ILS", "THB"),
        ("13 ₱ to dong", "PHP", "VND"),
        ("13 rouble to r$", "RUB", "BRL"),
        ("13 swissfranc to rand", "CHF", "ZAR"),
        ("13 ringgit to zloty", "MYR", "PLN"),
        ("13 us$ to rmb", "USD", "CNY"),
        ("13 ₽ to ₹", "RUB", "INR"),
        ("13 ₩ to ₺", "KRW", "TRY"),
        ("13 ₴ to ₪", "UAH", "ILS"),
        ("13 ฿ to £", "THB", "GBP"),
        ("13 ¥ to ₫", "JPY", "VND"),
    ];
    for (q, from, to) in pairs {
        cache(from, to, 13.0, 2.0);
        let w = fx(q).await.unwrap_or_else(|| panic!("{:?} not parsed", q));
        assert_eq!((w.from.as_str(), w.to.as_str()), (from, to), "{}", q);
    }
}

#[tokio::test]
async fn iso_codes_without_alias_are_accepted() {
    for (q, from, to) in [("14 czk to huf", "CZK", "HUF"), ("14 xau to aed", "XAU", "AED"), ("14 sek to nok", "SEK", "NOK")] {
        cache(from, to, 14.0, 3.0);
        let w = fx(q).await.unwrap_or_else(|| panic!("{:?}", q));
        assert_eq!(w.from, from);
        assert_eq!(w.to, to);
    }
}

#[tokio::test]
async fn fractional_amounts_use_their_own_cache_key() {
    cache("EUR", "USD", 2.5, 1.1);
    let w = fx("2.5 eur to usd").await.unwrap();
    assert_eq!(w.amount, 2.5);
    assert_eq!(w.rate, 1.1);
}

#[test]
fn fx_widget_serde_roundtrip() {
    let w: FxWidget = serde_json::from_value(widget("USD", "EUR", 1.0, 0.9)).unwrap();
    let back: FxWidget = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
    assert_eq!(back.from, "USD");
    assert_eq!(back.converted, 0.9);
    let c = back.clone();
    assert_eq!(c.svg, "M 0,40 L 260,40");
}
