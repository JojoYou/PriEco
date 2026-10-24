//! Tests for crates/crawler/src/download.rs against a tiny local HTTP server
//! (127.0.0.1 only, nothing leaves the machine).

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use prieco_mini_crawler::download::download;

/// Serves `n` connections; `respond(path) -> raw HTTP response bytes`
fn serve(n: usize, respond: fn(&str, u16) -> Vec<u8>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().take(n) {
            let Ok(mut s) = stream else { continue };
            let path = read_request_path(&mut s);
            let _ = s.write_all(&respond(&path, port));
            let _ = s.flush();
        }
    });
    port
}

fn read_request_path(s: &mut TcpStream) -> String {
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&buf);
    text.split_whitespace().nth(1).unwrap_or("/").to_string()
}

fn http(status: &str, extra_headers: &str, body: &[u8]) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
        status,
        body.len(),
        extra_headers
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

fn client() -> primp::Client {
    primp::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap()
}

fn closed_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let p = l.local_addr().unwrap().port();
    drop(l);
    p
}

#[tokio::test]
async fn ok_response_returns_body_status_time_and_url() {
    let port = serve(1, |_, _| http("200 OK", "Content-Type: text/html\r\n", b"<p>hello</p>"));
    let url = format!("http://127.0.0.1:{}/page", port);
    let (body, status, time, final_url) = download(&client(), &url).await;
    assert_eq!(body, "<p>hello</p>");
    assert_eq!(status, 200);
    assert!(time >= 0.0 && time < 10.0);
    assert_eq!(final_url, url);
}

#[tokio::test]
async fn error_status_is_returned_with_body() {
    let port = serve(1, |_, _| http("404 Not Found", "", b"missing"));
    let (body, status, _, final_url) =
        download(&client(), &format!("http://127.0.0.1:{}/x", port)).await;
    assert_eq!(status, 404);
    assert_eq!(body, "missing");
    assert!(final_url.ends_with("/x"));
}

#[tokio::test]
async fn server_error_status_is_returned() {
    let port = serve(1, |_, _| http("500 Internal Server Error", "", b""));
    let (body, status, _, _) = download(&client(), &format!("http://127.0.0.1:{}/", port)).await;
    assert_eq!(status, 500);
    assert_eq!(body, "");
}

#[tokio::test]
async fn redirects_are_followed_and_final_url_reported() {
    let port = serve(2, |path, port| {
        if path == "/start" {
            http(
                "302 Found",
                &format!("Location: http://127.0.0.1:{}/final\r\n", port),
                b"",
            )
        } else {
            http("200 OK", "", b"done")
        }
    });
    let (body, status, _, final_url) =
        download(&client(), &format!("http://127.0.0.1:{}/start", port)).await;
    assert_eq!(status, 200);
    assert_eq!(body, "done");
    assert_eq!(final_url, format!("http://127.0.0.1:{}/final", port));
}

#[tokio::test]
async fn invalid_utf8_body_is_decoded_lossily() {
    let port = serve(1, |_, _| http("200 OK", "", &[b'o', b'k', 0xff, 0xfe, b'!']));
    let (body, status, _, _) = download(&client(), &format!("http://127.0.0.1:{}/", port)).await;
    assert_eq!(status, 200);
    assert!(body.starts_with("ok"));
    assert!(body.ends_with('!'));
    assert!(body.contains('\u{FFFD}'));
}

#[tokio::test]
async fn large_body_is_fully_read() {
    let port = serve(1, |_, _| http("200 OK", "", &vec![b'a'; 2 * 1024 * 1024]));
    let (body, _, _, _) = download(&client(), &format!("http://127.0.0.1:{}/", port)).await;
    assert_eq!(body.len(), 2 * 1024 * 1024);
}

#[tokio::test]
async fn connection_refused_returns_empty_tuple() {
    let url = format!("http://127.0.0.1:{}/", closed_port());
    let (body, status, time, final_url) = download(&client(), &url).await;
    assert_eq!((body.as_str(), status, time, final_url.as_str()), ("", 0, 0.0, ""));
}

#[tokio::test]
async fn invalid_url_returns_empty_tuple() {
    for url in ["", "not a url", "ftp://127.0.0.1/", "http://"] {
        let (body, status, time, final_url) = download(&client(), url).await;
        assert_eq!(
            (body.as_str(), status, time, final_url.as_str()),
            ("", 0, 0.0, ""),
            "{:?}",
            url
        );
    }
}

#[tokio::test]
async fn server_closing_without_response_returns_empty_tuple() {
    let port = serve(2, |_, _| Vec::new());
    let (body, status, _, _) = download(&client(), &format!("http://127.0.0.1:{}/", port)).await;
    assert_eq!(status, 0);
    assert_eq!(body, "");
}
