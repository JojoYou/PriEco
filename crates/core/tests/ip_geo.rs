//! Tests for IpGeoDatabase (IP range -> country lookups backed by redb)

mod common;

use std::net::{Ipv4Addr, Ipv6Addr};

use common::TmpDir;
use prieco_core::IpGeoDatabase;
use redb::{Database, TableDefinition};

// Must mirror the private table definition in globals.rs
const IP_RANGES: TableDefinition<u128, (u128, String)> = TableDefinition::new("ip_ranges");

fn v4(s: &str) -> u128 {
    u32::from(s.parse::<Ipv4Addr>().unwrap()) as u128
}

fn v6(s: &str) -> u128 {
    u128::from(s.parse::<Ipv6Addr>().unwrap())
}

/// Builds a redb file with the given (start, end, country) ranges and opens it with IpGeoDatabase
fn geo_db(tmp: &TmpDir, ranges: &[(u128, u128, &str)]) -> IpGeoDatabase {
    let path = tmp.join_str("ip.redb");
    {
        let db = Database::create(&path).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut t = tx.open_table(IP_RANGES).unwrap();
            for (start, end, cc) in ranges {
                t.insert(*start, (*end, cc.to_string())).unwrap();
            }
        }
        tx.commit().unwrap();
    }
    IpGeoDatabase::open(&path).unwrap()
}

fn sample(tmp: &TmpDir) -> IpGeoDatabase {
    geo_db(
        tmp,
        &[
            (v4("1.0.0.0"), v4("1.0.0.255"), "au"),
            (v4("10.0.0.0"), v4("10.255.255.255"), "zz"),
            (v4("100.0.0.0"), v4("100.0.0.0"), "one"),
            (v6("2001:db8::"), v6("2001:db8::ffff"), "v6"),
        ],
    )
}

#[test]
fn ipv4_inside_range() {
    let tmp = TmpDir::new("geo1");
    let db = sample(&tmp);
    assert_eq!(db.lookup_country("1.0.0.5").unwrap().as_deref(), Some("au"));
    assert_eq!(db.lookup_country("10.20.30.40").unwrap().as_deref(), Some("zz"));
}

#[test]
fn ipv4_range_boundaries_are_inclusive() {
    let tmp = TmpDir::new("geo2");
    let db = sample(&tmp);
    assert_eq!(db.lookup_country("1.0.0.0").unwrap().as_deref(), Some("au"));
    assert_eq!(db.lookup_country("1.0.0.255").unwrap().as_deref(), Some("au"));
    assert_eq!(db.lookup_country("10.0.0.0").unwrap().as_deref(), Some("zz"));
    assert_eq!(db.lookup_country("10.255.255.255").unwrap().as_deref(), Some("zz"));
}

#[test]
fn ipv4_just_outside_range_is_none() {
    let tmp = TmpDir::new("geo3");
    let db = sample(&tmp);
    assert_eq!(db.lookup_country("1.0.1.0").unwrap(), None);
    assert_eq!(db.lookup_country("0.255.255.255").unwrap(), None);
    assert_eq!(db.lookup_country("11.0.0.0").unwrap(), None);
    assert_eq!(db.lookup_country("9.255.255.255").unwrap(), None);
}

#[test]
fn single_address_range() {
    let tmp = TmpDir::new("geo4");
    let db = sample(&tmp);
    assert_eq!(db.lookup_country("100.0.0.0").unwrap().as_deref(), Some("one"));
    assert_eq!(db.lookup_country("100.0.0.1").unwrap(), None);
}

#[test]
fn lowest_and_highest_ipv4() {
    let tmp = TmpDir::new("geo5");
    let db = geo_db(
        &tmp,
        &[
            (v4("0.0.0.0"), v4("0.0.0.10"), "lo"),
            (v4("255.255.255.0"), v4("255.255.255.255"), "hi"),
        ],
    );
    assert_eq!(db.lookup_country("0.0.0.0").unwrap().as_deref(), Some("lo"));
    assert_eq!(db.lookup_country("255.255.255.255").unwrap().as_deref(), Some("hi"));
    assert_eq!(db.lookup_country("128.0.0.0").unwrap(), None);
}

#[test]
fn ipv6_inside_and_outside_range() {
    let tmp = TmpDir::new("geo6");
    let db = sample(&tmp);
    assert_eq!(db.lookup_country("2001:db8::1").unwrap().as_deref(), Some("v6"));
    assert_eq!(db.lookup_country("2001:db8::ffff").unwrap().as_deref(), Some("v6"));
    assert_eq!(db.lookup_country("2001:db8::1:0").unwrap(), None);
    assert_eq!(db.lookup_country("::1").unwrap(), None);
}

#[test]
fn ipv6_compressed_and_expanded_forms_are_equal() {
    let tmp = TmpDir::new("geo7");
    let db = sample(&tmp);
    assert_eq!(
        db.lookup_country("2001:0db8:0000:0000:0000:0000:0000:0001").unwrap(),
        db.lookup_country("2001:db8::1").unwrap()
    );
}

#[test]
fn ipv4_mapped_ipv6_is_not_treated_as_ipv4() {
    let tmp = TmpDir::new("geo8");
    let db = sample(&tmp);
    // ::ffff:1.0.0.5 is a different number than 1.0.0.5 in the shared u128 space
    assert_eq!(db.lookup_country("::ffff:1.0.0.5").unwrap(), None);
}

#[test]
fn invalid_ip_strings_are_errors() {
    let tmp = TmpDir::new("geo9");
    let db = sample(&tmp);
    for bad in [
        "",
        "garbage",
        "1.2.3",
        "1.2.3.4.5",
        "256.0.0.1",
        "1.0.0.5 ",
        " 1.0.0.5",
        "1.0.0.5:80",
        "[2001:db8::1]",
        "2001:db8::g",
        "localhost",
    ] {
        assert!(db.lookup_country(bad).is_err(), "{:?} should be an error", bad);
    }
}

#[test]
fn database_without_table_returns_none() {
    let tmp = TmpDir::new("geo10");
    let db = IpGeoDatabase::open(&tmp.join_str("empty.redb")).unwrap();
    assert_eq!(db.lookup_country("1.2.3.4").unwrap(), None);
    assert_eq!(db.lookup_country("::1").unwrap(), None);
}

#[test]
fn database_with_empty_table_returns_none() {
    let tmp = TmpDir::new("geo11");
    let db = geo_db(&tmp, &[]);
    assert_eq!(db.lookup_country("1.2.3.4").unwrap(), None);
}

#[test]
fn open_creates_missing_parent_directories() {
    let tmp = TmpDir::new("geo12");
    let path = tmp.join_str("a/b/c/ip.redb");
    let db = IpGeoDatabase::open(&path).unwrap();
    assert!(std::path::Path::new(&path).exists());
    assert_eq!(db.lookup_country("8.8.8.8").unwrap(), None);
}

#[test]
fn open_existing_database_keeps_data() {
    let tmp = TmpDir::new("geo13");
    drop(sample(&tmp));
    let reopened = IpGeoDatabase::open(&tmp.join_str("ip.redb")).unwrap();
    assert_eq!(reopened.lookup_country("1.0.0.1").unwrap().as_deref(), Some("au"));
}

#[test]
fn country_code_is_returned_verbatim() {
    let tmp = TmpDir::new("geo14");
    let db = geo_db(&tmp, &[(v4("5.5.5.0"), v4("5.5.5.255"), "SK")]);
    assert_eq!(db.lookup_country("5.5.5.5").unwrap().as_deref(), Some("SK"));
}

#[test]
fn adjacent_ranges_pick_the_right_one() {
    let tmp = TmpDir::new("geo15");
    let db = geo_db(
        &tmp,
        &[
            (v4("20.0.0.0"), v4("20.0.0.127"), "a"),
            (v4("20.0.0.128"), v4("20.0.0.255"), "b"),
        ],
    );
    assert_eq!(db.lookup_country("20.0.0.127").unwrap().as_deref(), Some("a"));
    assert_eq!(db.lookup_country("20.0.0.128").unwrap().as_deref(), Some("b"));
}

#[test]
fn gap_between_ranges_is_none() {
    let tmp = TmpDir::new("geo16");
    let db = geo_db(
        &tmp,
        &[
            (v4("30.0.0.0"), v4("30.0.0.9"), "a"),
            (v4("30.0.0.20"), v4("30.0.0.29"), "b"),
        ],
    );
    assert_eq!(db.lookup_country("30.0.0.15").unwrap(), None);
}

#[test]
fn many_ranges_lookup_is_consistent() {
    let tmp = TmpDir::new("geo17");
    let ranges: Vec<(u128, u128, String)> = (0..200u32)
        .map(|i| {
            let start = (i as u128) << 16;
            (start, start + 0xFF, format!("c{}", i))
        })
        .collect();
    let refs: Vec<(u128, u128, &str)> = ranges.iter().map(|(a, b, c)| (*a, *b, c.as_str())).collect();
    let db = geo_db(&tmp, &refs);
    for i in (0..200u32).step_by(17) {
        let ip = Ipv4Addr::from(((i as u32) << 16) + 0x10);
        assert_eq!(
            db.lookup_country(&ip.to_string()).unwrap(),
            Some(format!("c{}", i))
        );
        let gap = Ipv4Addr::from(((i as u32) << 16) + 0x1000);
        assert_eq!(db.lookup_country(&gap.to_string()).unwrap(), None);
    }
}
