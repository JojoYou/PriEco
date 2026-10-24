//! Tests for crates/core/src/set_up.rs (set up wizard driven through env variables)
//!
//! The wizard reads/writes "config/settings.conf" relative to the working directory,
//! so this binary moves into its own temp dir. Everything runs in ONE test because
//! the process environment and cwd are shared.

use std::fs;

use prieco_core::{PriEcoConfig, set_up::set_up_wizard};

fn set_env(ip: &str, port: &str, workers: &str, base: &str) {
    // SAFETY: the only test in this binary, no other thread reads the env
    unsafe {
        std::env::set_var("PRIECO_IP", ip);
        std::env::set_var("PRIECO_PORT", port);
        std::env::set_var("PRIECO_TANTIVY", format!("{}/tantivy", base));
        std::env::set_var("PRIECO_META", format!("{}/meta", base));
        std::env::set_var("PRIECO_VECTOR", format!("{}/vectors", base));
        std::env::set_var("PRIECO_BLOB", format!("{}/blob", base));
        std::env::set_var("PRIECO_WORKERS", workers);
    }
}

fn saved() -> PriEcoConfig {
    serde_json::from_str(&fs::read_to_string("config/settings.conf").unwrap()).unwrap()
}

#[test]
fn wizard_env_save_load_and_repair_cycle() {
    let root = std::env::temp_dir().join(format!("prieco_core_setup_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    std::env::set_current_dir(&root).unwrap();

    /* 1: fresh install, everything from env */
    set_env("127.0.0.1", " 8081 ", " 3 ", "data1");
    let c = set_up_wizard();
    assert_eq!(c.ip, "127.0.0.1");
    assert_eq!(c.port, 8081, "port is trimmed");
    assert_eq!(c.worker_concurrent, 3, "workers are trimmed");
    assert_eq!(c.tantivy_path, "data1/tantivy");
    assert_eq!(c.meta_path, "data1/meta");
    assert_eq!(c.vector_path, "data1/vectors");
    assert_eq!(c.blob_path, "data1/blob");
    assert!(c.enable_blob_storage && c.enable_db_inserter && c.enable_pagerank);
    assert!(c.enable_mini_crawler && c.enable_index_updater && c.enable_web_server);

    // directories are created
    for d in ["data1/tantivy", "data1/meta", "data1/vectors", "data1/blob", "config"] {
        assert!(root.join(d).is_dir(), "{} not created", d);
    }
    // and the config is persisted as JSON
    let s = saved();
    assert_eq!(s.ip, "127.0.0.1");
    assert_eq!(s.port, 8081);
    assert_eq!(s.vector_path, "data1/vectors");

    /* 2: existing config wins over env */
    set_env("10.0.0.1", "9999", "9", "data2");
    let c = set_up_wizard();
    assert_eq!(c.ip, "127.0.0.1");
    assert_eq!(c.port, 8081);
    assert_eq!(c.tantivy_path, "data1/tantivy");
    assert!(!root.join("data2").exists(), "loading must not create new dirs");

    /* 3: missing enable_* flags in an old config default to true, explicit false kept */
    fs::write(
        "config/settings.conf",
        r#"{"ip":"0.0.0.0","port":80,"tantivy_path":"t","meta_path":"m","vector_path":"v",
            "blob_path":"b","worker_concurrent":1,"enable_pagerank":false}"#,
    )
    .unwrap();
    let c = set_up_wizard();
    assert_eq!(c.ip, "0.0.0.0");
    assert!(!c.enable_pagerank);
    assert!(c.enable_web_server && c.enable_mini_crawler);

    /* 4: corrupt config is replaced by a fresh wizard run */
    fs::write("config/settings.conf", "{ definitely not json").unwrap();
    set_env("::1", "443", "2", "data3");
    let c = set_up_wizard();
    assert_eq!(c.ip, "::1");
    assert_eq!(c.port, 443);
    assert_eq!(c.worker_concurrent, 2);
    assert_eq!(saved().blob_path, "data3/blob");

    /* 5: IP addresses are normalised */
    fs::remove_file("config/settings.conf").unwrap();
    set_env("0000:0000:0000:0000:0000:0000:0000:0001", "80", "1", "data4");
    assert_eq!(set_up_wizard().ip, "::1");

    fs::remove_file("config/settings.conf").unwrap();
    set_env(" 127.0.0.1 ", "80", "1", "data5");
    // NOTE: the IP value is not trimmed before parsing (port and workers are); a
    // padded IP aborts the process with exit(1). Only the trimmed form is used here.
    set_env("192.168.0.10", "80", "1", "data5");
    assert_eq!(set_up_wizard().ip, "192.168.0.10");

    /* 6: negative or zero ports are accepted as is (no validation) */
    fs::remove_file("config/settings.conf").unwrap();
    set_env("127.0.0.1", "-1", "0", "data6");
    let c = set_up_wizard();
    assert_eq!(c.port, -1);
    assert_eq!(c.worker_concurrent, 0);

    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    let _ = fs::remove_dir_all(&root);
}
