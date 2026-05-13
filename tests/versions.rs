mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use common::{code, stdout, Harness};

/// A tiny single-thread HTTP server that responds to two paths:
///   GET /token  → {"token":"fake-token"}
///   GET /tags   → {"tags":[...]} or anything supplied
struct MockGhcr {
    addr: String,
    _handle: thread::JoinHandle<()>,
    _stop: Arc<Mutex<bool>>,
}

impl MockGhcr {
    fn start(tags_body: &'static str, token_body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(Mutex::new(false));
        let stop_c = stop.clone();
        let h = thread::spawn(move || {
            listener.set_nonblocking(false).unwrap();
            // Serve up to ~10 requests, plenty for a single rusta versions call.
            for _ in 0..10 {
                if *stop_c.lock().unwrap() {
                    break;
                }
                let (mut sock, _) = match listener.accept() {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let mut buf = [0u8; 4096];
                let n = sock.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = if req.contains("/token") {
                    token_body
                } else {
                    tags_body
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes());
            }
        });
        Self { addr, _handle: h, _stop: stop }
    }

    fn token_url(&self) -> String {
        format!("{}/token", self.addr)
    }

    fn tags_url(&self) -> String {
        format!("{}/tags", self.addr)
    }
}

#[test]
fn versions_lists_and_marks_default() {
    let h = Harness::new();
    let mock = MockGhcr::start(
        r#"{"tags":["20.04","22.04","24.04","latest","99.x"]}"#,
        r#"{"token":"fake-token"}"#,
    );
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 0, "stderr: {}", common::stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("20.04"));
    assert!(s.contains("22.04"));
    assert!(s.contains("24.04 (default)"));
    assert!(!s.contains("latest"));
    assert!(!s.contains("99.x"));
}

#[test]
fn versions_token_request_failure_is_fatal() {
    let h = Harness::new();
    let mut cmd = h.cmd(&["versions"]);
    // Point at an address nothing is listening on.
    cmd.env("RUSTA_GHCR_TOKEN_URL", "http://127.0.0.1:1/token");
    cmd.env("RUSTA_GHCR_TAGS_URL", "http://127.0.0.1:1/tags");
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn versions_missing_token_field_is_fatal() {
    let h = Harness::new();
    let mock = MockGhcr::start(r#"{"tags":["24.04"]}"#, r#"{}"#);
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}

#[test]
fn versions_missing_tags_array_is_fatal() {
    let h = Harness::new();
    let mock = MockGhcr::start(r#"{}"#, r#"{"token":"t"}"#);
    let mut cmd = h.cmd(&["versions"]);
    cmd.env("RUSTA_GHCR_TOKEN_URL", mock.token_url());
    cmd.env("RUSTA_GHCR_TAGS_URL", mock.tags_url());
    let out = cmd.output().unwrap();
    assert_eq!(code(&out), 1);
}
