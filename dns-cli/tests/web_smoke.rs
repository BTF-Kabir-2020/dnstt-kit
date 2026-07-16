//! تست دود پنل وب: همهٔ اکشن‌های UI باید واقعاً کار کنند.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dns-cli"))
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn http(method: &str, host: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(host).expect("connect");
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    let body = body.unwrap_or("");
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn wait_http(host: &str, path: &str, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if let Ok(mut s) = TcpStream::connect(host) {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
            if s.write_all(req.as_bytes()).is_ok() {
                let mut buf = [0u8; 64];
                if s.read(&mut buf).is_ok() {
                    return true;
                }
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn wait_job_done(host: &str, job_id: &str, secs: u64) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        let (_, body) = http("GET", host, "/api/jobs", None);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(arr) = v.as_array() {
                if let Some(j) = arr.iter().find(|j| j["id"] == job_id) {
                    let st = j["status"].as_str().unwrap_or("");
                    if st == "ok" || st == "error" {
                        return j.clone();
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(150));
    }
    panic!("job {job_id} timeout");
}

#[test]
fn web_ui_actions_work_from_kit_root() {
    let root = root();
    let port = free_port();
    let host = format!("127.0.0.1:{port}");
    let bind = host.clone();

    let mut child = bin()
        .current_dir(&root)
        .args([
            "--work-dir",
            root.to_str().unwrap(),
            "serve",
            "--bind",
            &bind,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn serve");

    assert!(wait_http(&host, "/", 8), "serve did not start");

    let (st, html) = http("GET", &host, "/", None);
    assert_eq!(st, 200);
    assert!(html.contains("dnstt-kit panel"), "friendly UI missing");
    assert!(html.contains("Options"), "options tab missing");
    assert!(
        html.contains("/tailwindcss.js"),
        "offline Tailwind CDN script missing"
    );
    assert!(
        !html.contains("https://cdn.tailwindcss.com"),
        "must not load online CDN"
    );

    let (st, js) = http("GET", &host, "/tailwindcss.js", None);
    assert_eq!(st, 200);
    assert!(js.len() > 10_000, "tailwindcss.js too small");
    assert!(
        js.contains("tailwind") || js.starts_with("(()=>{"),
        "not a tailwind bundle"
    );

    let (st, health) = http("GET", &host, "/api/health", None);
    assert_eq!(st, 200);
    let h: serde_json::Value = serde_json::from_str(&health).unwrap();
    assert_eq!(h["ready"], true, "health={health}");
    let wd = h["work_dir"].as_str().unwrap_or("");
    assert!(
        wd.contains("dnstt-kit") || wd.starts_with('…'),
        "masked work_dir={wd}"
    );
    assert!(
        !wd.contains("Users\\"),
        "must not leak full Windows path: {wd}"
    );
    assert!(h.get("work_dir_full").is_none() || h["work_dir_full"].is_null());

    // path traversal rejected
    let (st, body) = http(
        "POST",
        &host,
        "/api/scan",
        Some(r#"{"input":"../Cargo.toml","preset":"low","limit":"1"}"#),
    );
    assert_eq!(st, 409, "{body}");
    assert!(
        body.contains("not allowed") || body.contains(".."),
        "{body}"
    );
    let (st, body) = http("POST", &host, "/api/exec", Some(r#"{"argv":["doctor"]}"#));
    assert_eq!(st, 200, "{body}");
    let j: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(j["ok"], true);
    let job = wait_job_done(&host, j["job_id"].as_str().unwrap(), 20);
    assert_eq!(job["status"], "ok", "doctor job={job}");

    // scan
    let (st, body) = http(
        "POST",
        &host,
        "/api/scan",
        Some(r#"{"input":"testdata/dns_sample.txt","preset":"low","limit":"3","quiet":true}"#),
    );
    assert_eq!(st, 200, "{body}");
    let j: serde_json::Value = serde_json::from_str(&body).unwrap();
    let job = wait_job_done(&host, j["job_id"].as_str().unwrap(), 30);
    assert_eq!(job["status"], "ok", "scan job={job}");

    // pipeline with full options payload
    let (st, body) = http(
        "POST",
        &host,
        "/api/pipeline",
        Some(
            r#"{"input":"testdata/dns_sample.txt","profile":"mame","preset":"low","limit":"3","skip_slipnet":true,"no_dmvpn":true,"quiet":true,"generate_kinds":"all"}"#,
        ),
    );
    assert_eq!(st, 200, "{body}");
    let j: serde_json::Value = serde_json::from_str(&body).unwrap();
    let job = wait_job_done(&host, j["job_id"].as_str().unwrap(), 45);
    assert_eq!(job["status"], "ok", "pipeline job={job}");

    // backup
    let (st, body) = http(
        "POST",
        &host,
        "/api/exec",
        Some(r#"{"argv":["backup","create","--mode","kit","--keep","5","--label","websmoke"]}"#),
    );
    assert_eq!(st, 200, "{body}");
    let j: serde_json::Value = serde_json::from_str(&body).unwrap();
    let job = wait_job_done(&host, j["job_id"].as_str().unwrap(), 60);
    assert_eq!(job["status"], "ok", "backup job={job}");

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn serve_from_target_release_auto_resolves_kit_root() {
    let root = root();
    let release = root.join("target").join("release");
    if !release.is_dir() {
        std::fs::create_dir_all(&release).ok();
    }
    let port = free_port();
    let host = format!("127.0.0.1:{port}");

    // بدون --work-dir، فقط cwd=target/release — باید auto-detect شود
    let mut child = bin()
        .current_dir(&release)
        .args(["serve", "--bind", &host])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    assert!(wait_http(&host, "/api/health", 8), "serve start");
    let (st, health) = http("GET", &host, "/api/health", None);
    assert_eq!(st, 200);
    let h: serde_json::Value = serde_json::from_str(&health).unwrap();
    assert_eq!(
        h["ready"], true,
        "expected auto work_dir to kit root, got {health}"
    );
    let wd = h["work_dir"].as_str().unwrap_or("");
    assert!(!wd.contains(":\\"), "API must mask absolute path, got {wd}");

    let (st, body) = http(
        "POST",
        &host,
        "/api/scan",
        Some(r#"{"input":"testdata/dns_sample.txt","preset":"low","limit":"2","quiet":true}"#),
    );
    assert_eq!(st, 200, "{body}");
    let j: serde_json::Value = serde_json::from_str(&body).unwrap();
    let job = wait_job_done(&host, j["job_id"].as_str().unwrap(), 30);
    assert_eq!(
        job["status"], "ok",
        "scan from target/release cwd must work via auto work_dir; job={job}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
