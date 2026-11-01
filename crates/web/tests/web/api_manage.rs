//! Tests for src/web/functions/api/manage.rs: free tier counter and billing queue.
//! Everything here is offline: support::init removes the POLAR_* env vars, so
//! paid() and flush() return before any request to Polar.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use chrono::{Datelike, Utc};
use prieco_core::PRIECO_META;

use crate::support::{init, unique, unique_u64};
use crate::web::functions::api::manage::{API_BILLING_QUEUE, flush, free, paid};

const LIMIT: usize = 25;

fn today() -> u32 {
    Utc::now().date_naive().num_days_from_ce() as u32
}

/// Fresh 10.x.x.x address, so parallel tests never share a counter
fn fresh_v4() -> IpAddr {
    let n = unique_u64() as u32;
    IpAddr::V4(Ipv4Addr::from(0x0A00_0000 | (n & 0x00FF_FFFF)))
}

/// Fresh /64 (2001:db8:n::/64) with the given interface part
fn fresh_v6_net() -> (u16, u16) {
    let n = unique_u64() as u32;
    ((n >> 16) as u16, n as u16)
}

fn v6(net: (u16, u16), host: u16) -> IpAddr {
    IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, net.0, net.1, 0, 0, 0, host))
}

/// Same key layout as free(): "f:" + IPv4 octets or first 8 IPv6 octets
fn key(ip: IpAddr) -> Vec<u8> {
    let mut key = b"f:".to_vec();
    match ip {
        IpAddr::V4(v4) => key.extend_from_slice(&v4.octets()),
        IpAddr::V6(v6) => key.extend_from_slice(&v6.octets()[..8]),
    }
    key
}

fn stored(ip: IpAddr) -> Option<Vec<u8>> {
    PRIECO_META
        .api_ks
        .get(key(ip))
        .unwrap()
        .map(|v| v.to_vec())
}

fn use_up(ip: IpAddr) {
    for i in 0..LIMIT {
        assert!(free(ip), "call {} of {} refused", i + 1, LIMIT);
    }
}

/* ───────────────────────────── free tier ───────────────────────────── */

#[test]
fn allows_25_then_refuses() {
    init();
    let ip = fresh_v4();

    use_up(ip);
    assert!(!free(ip));
    assert!(!free(ip));
}

#[test]
fn stores_today_and_count() {
    init();
    let ip = fresh_v4();

    assert!(free(ip));
    assert!(free(ip));

    let mut expected = today().to_le_bytes().to_vec();
    expected.push(2);
    assert_eq!(stored(ip), Some(expected));
}

#[test]
fn refused_calls_do_not_increase_count() {
    init();
    let ip = fresh_v4();

    use_up(ip);
    for _ in 0..10 {
        assert!(!free(ip));
    }

    let v = stored(ip).unwrap();
    assert_eq!(v.len(), 5);
    assert_eq!(v[4] as usize, LIMIT);
}

#[test]
fn new_day_resets_count() {
    init();
    let ip = fresh_v4();

    // Limit used up yesterday
    let mut yesterday = (today() - 1).to_le_bytes().to_vec();
    yesterday.push(LIMIT as u8);
    PRIECO_META.api_ks.insert(key(ip), yesterday).unwrap();

    assert!(free(ip));

    let mut expected = today().to_le_bytes().to_vec();
    expected.push(1);
    assert_eq!(stored(ip), Some(expected));
}

#[test]
fn malformed_value_counts_as_zero() {
    init();
    let ip = fresh_v4();

    PRIECO_META.api_ks.insert(key(ip), vec![1u8, 2, 3]).unwrap();

    use_up(ip);
    assert!(!free(ip));
}

#[test]
fn ipv4_addresses_are_counted_separately() {
    init();
    let a = fresh_v4();
    let b = fresh_v4();

    use_up(a);
    assert!(!free(a));
    assert!(free(b));
}

#[test]
fn ipv6_same_64_shares_counter() {
    init();
    let net = fresh_v6_net();

    use_up(v6(net, 1));
    // Different address, same /64
    assert!(!free(v6(net, 2)));
    assert!(!free(v6(net, 0xffff)));
}

#[test]
fn ipv6_different_64_has_own_counter() {
    init();
    let a = fresh_v6_net();
    let b = fresh_v6_net();

    use_up(v6(a, 1));
    assert!(!free(v6(a, 1)));
    assert!(free(v6(b, 1)));
}

#[test]
fn ipv4_and_ipv6_do_not_collide() {
    init();
    let ip4 = fresh_v4();
    let IpAddr::V4(v4) = ip4 else { unreachable!() };
    let o = v4.octets();

    // IPv6 whose first 4 bytes equal the IPv4 address
    let ip6 = IpAddr::V6(Ipv6Addr::new(
        u16::from_be_bytes([o[0], o[1]]),
        u16::from_be_bytes([o[2], o[3]]),
        0,
        0,
        0,
        0,
        0,
        1,
    ));

    use_up(ip4);
    assert!(!free(ip4));
    assert!(free(ip6));
}

/* ───────────────────────────── paid ───────────────────────────── */

#[tokio::test]
async fn paid_without_polar_env_refuses() {
    init();

    assert_eq!(paid(&unique("prieco_test")).await, None);
}

/* ───────────────────────────── billing queue ───────────────────────────── */

#[tokio::test]
async fn flush_without_api_key_keeps_queue() {
    init();
    let id = unique("customer");

    API_BILLING_QUEUE.lock().push(id.clone());
    flush().await;

    // Nothing was sent, so nothing may be lost
    assert!(API_BILLING_QUEUE.lock().contains(&id));
}

#[tokio::test]
async fn flush_keeps_duplicates() {
    init();
    let id = unique("customer");

    for _ in 0..3 {
        API_BILLING_QUEUE.lock().push(id.clone());
    }
    flush().await;

    // Every call is one billed event, duplicates must stay
    let count = API_BILLING_QUEUE.lock().iter().filter(|q| **q == id).count();
    assert_eq!(count, 3);
}
