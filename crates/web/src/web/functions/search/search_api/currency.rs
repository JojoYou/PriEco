use chrono::{Duration, Utc};
use prieco_core::{CLIENT, PRIECO_META};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashSet,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

static CURRENCY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:convert\s+)?(\d+(?:\.\d+)?)\s*([a-z\$€£¥₹₩₺₴₪฿₱₫₽]+)\s*(?:to|in|=)\s*([a-z\$€£¥₹₩₺₴₪฿₱₫₽]+)$",
    )
    .unwrap()
});

static VALID_ISO_CODES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT",
        "BHD", "BIF", "BMD", "BND", "BOB", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD", "CDF",
        "CHF", "CLP", "CNH", "CNY", "COP", "CRC", "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD",
        "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP", "GEL", "GGP", "GHS", "GIP", "GMD", "GNF",
        "GTQ", "GYD", "HKD", "HNL", "HTG", "HUF", "IDR", "ILS", "IMP", "INR", "IQD", "IRR", "ISK",
        "JEP", "JMD", "JOD", "JPY", "KES", "KGS", "KHR", "KMF", "KPW", "KRW", "KWD", "KYD", "KZT",
        "LAK", "LBP", "LKR", "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP",
        "MRO", "MRU", "MUR", "MVR", "MWK", "MXN", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK", "NPR",
        "NZD", "OMR", "PAB", "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB",
        "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP", "SLE", "SOS", "SRD", "SSP", "STN",
        "SVC", "SYP", "SZL", "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TWD", "TZS", "UAH",
        "UGX", "USD", "UYU", "UZS", "VES", "VND", "VUV", "WST", "XAF", "XAG", "XAU", "XCD", "XCG",
        "XDR", "XOF", "XPD", "XPF", "XPT", "YER", "ZAR", "ZMW", "ZWG",
    ]
    .into_iter()
    .collect()
});

#[derive(Serialize, Deserialize, Clone)]
pub struct FxWidget {
    pub amount: f64,
    pub from: String,
    pub to: String,
    pub rate: f64,
    pub converted: f64,
    pub date: String,
    pub svg: String,
    pub min_label: String,
    pub max_label: String,
    pub mid_label: String,
    pub start_date: String,
    pub history_json: String,
}

#[derive(Serialize, Deserialize)]
struct CachedFx {
    timestamp: u64,
    data: FxWidget,
}

pub async fn get_fx_widget(q: &str) -> Option<FxWidget> {
    let caps = CURRENCY_REGEX.captures(q.trim())?;
    let amount: f64 = caps.get(1)?.as_str().parse().ok()?;
    let from = normalize_fx(caps.get(2)?.as_str())?;
    let to = normalize_fx(caps.get(3)?.as_str())?;

    if from == to {
        return None;
    }

    let cache_key = format!("fx:{}:{}:{}", from, to, amount);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Ok(Some(bytes)) = PRIECO_META.widgets_ks.get(cache_key.as_bytes()) {
        if let Ok(c) = serde_json::from_slice::<CachedFx>(&bytes) {
            if now.saturating_sub(c.timestamp) < 21600 {
                return Some(c.data);
            }
        }
    }

    let start = (Utc::now() - Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let url = format!(
        "https://api.frankfurter.dev/v2/rates?from={}&base={}&quotes={}",
        start, from, to
    );

    let res = CLIENT.get(&url).send().await.ok()?;
    let rates: Vec<serde_json::Value> = res.json().await.ok()?;
    let latest = rates.last()?;

    let rate = latest["rate"].as_f64()?;
    let rate_vals: Vec<f64> = rates.iter().filter_map(|r| r["rate"].as_f64()).collect();

    let min = rate_vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = rate_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mid = (min + max) / 2.0;

    let start_date = rates
        .first()
        .and_then(|r| r["date"].as_str())
        .unwrap_or("")
        .to_string();

    let history_array: Vec<serde_json::Value> = rates
        .iter()
        .filter_map(|r| {
            Some(json!({
                "date": r["date"].as_str()?,
                "rate": r["rate"].as_f64()?
            }))
        })
        .collect();
    let history_json = serde_json::to_string(&history_array).unwrap_or_default();

    let widget = FxWidget {
        amount,
        from: from.to_string(),
        to: to.to_string(),
        rate,
        converted: (amount * rate * 100.0).round() / 100.0,
        date: latest["date"].as_str()?.to_string(),
        svg: gen_svg(&rate_vals, min, max, 260.0, 80.0),
        min_label: format!("{:.4}", min),
        max_label: format!("{:.4}", max),
        mid_label: format!("{:.4}", mid),
        start_date,
        history_json,
    };

    if let Ok(bytes) = serde_json::to_vec(&CachedFx {
        timestamp: now,
        data: widget.clone(),
    }) {
        let _ = PRIECO_META.widgets_ks.insert(cache_key.as_bytes(), bytes);
    }

    Some(widget)
}

fn normalize_fx(s: &str) -> Option<&'static str> {
    let upper = s.to_uppercase();

    let aliased = match upper.as_str() {
        "$" | "USD" | "DOLLAR" | "DOLLARS" | "US$" => Some("USD"),
        "€" | "EUR" | "EURO" | "EUROS" => Some("EUR"),
        "£" | "GBP" | "POUND" | "POUNDS" | "STERLING" => Some("GBP"),
        "¥" | "JPY" | "YEN" => Some("JPY"),
        "₹" | "INR" | "RUPEE" | "RUPEES" => Some("INR"),
        "₩" | "KRW" | "WON" => Some("KRW"),
        "₺" | "TRY" | "LIRA" => Some("TRY"),
        "₴" | "UAH" | "HRYVNIA" => Some("UAH"),
        "₪" | "ILS" | "SHEKEL" | "NIS" => Some("ILS"),
        "฿" | "THB" | "BAHT" => Some("THB"),
        "₱" | "PHP" => Some("PHP"),
        "₫" | "VND" | "DONG" => Some("VND"),
        "₽" | "RUB" | "RUBLE" | "ROUBLE" => Some("RUB"),
        "R$" | "BRL" | "REAL" => Some("BRL"),
        "CHF" | "SWISSFRANC" => Some("CHF"),
        "ZAR" | "RAND" => Some("ZAR"),
        "MYR" | "RINGGIT" => Some("MYR"),
        "YUAN" | "RMB" | "CNY" => Some("CNY"),
        "ZLOTY" | "PLN" => Some("PLN"),
        _ => None,
    };

    if let Some(code) = aliased {
        return Some(code);
    }

    if upper.len() == 3 {
        if let Some(&code) = VALID_ISO_CODES.get(upper.as_str()) {
            return Some(code);
        }
    }

    None
}

fn gen_svg(rates: &[f64], min: f64, max: f64, w: f64, h: f64) -> String {
    if rates.len() < 2 || max <= min {
        return format!("M 0,{} L {},{}", h / 2.0, w, h / 2.0);
    }
    let (step, range) = (w / (rates.len() - 1) as f64, max - min);
    rates
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let y = h - ((r - min) / range * (h - 10.0) + 5.0);
            format!(
                "{} {:.1},{:.1}",
                if i == 0 { "M" } else { "L" },
                i as f64 * step,
                y
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}
