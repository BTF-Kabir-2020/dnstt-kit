//! پنل وب: داشبورد + API امن‌شده (توکن اختیاری، مسیر نسبی، بدون leak مسیر کامل).

use crate::db;
use crate::jobs::{self, JobQueue};
use crate::web_security::{self, mask_work_dir, safe_rel_path};
use crate::workdir;
use std::path::Path;
use std::sync::Arc;
use tiny_http::{Header, Method, Request, Response, Server};

pub type AppResult = Result<(), String>;

pub fn serve(work_dir: &Path, bind: &str) -> AppResult {
    let _ = db::open(work_dir)?;
    let jobs = Arc::new(JobQueue::default());
    let work_dir = work_dir.to_path_buf();
    web_security::warn_if_non_loopback(bind);
    let server = Server::http(bind).map_err(|e| e.to_string())?;
    println!("🌐 web UI: http://{bind}/  (db under work_dir, path masked in API)");
    println!("   work_dir (local console only)={}", work_dir.display());
    println!(
        "   API: GET /api/{{health,runs,stats,jobs,commands}}  POST /api/{{pipeline,scan,exec}}"
    );
    println!("   Ctrl+C برای توقف");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();
        let (status, body, ctype) = route(&work_dir, &jobs, &method, &url, &mut request);
        let mut response = Response::from_string(body)
            .with_status_code(tiny_http::StatusCode(status as u16))
            .with_header(
                Header::from_bytes("Content-Type", ctype.as_bytes())
                    .unwrap_or_else(|_| Header::from_bytes("Content-Type", "text/plain").unwrap()),
            );
        // basic hardening headers
        if let Ok(h) = Header::from_bytes("X-Content-Type-Options", "nosniff") {
            response = response.with_header(h);
        }
        if let Ok(h) = Header::from_bytes("X-Frame-Options", "DENY") {
            response = response.with_header(h);
        }
        if let Ok(h) = Header::from_bytes("Referrer-Policy", "no-referrer") {
            response = response.with_header(h);
        }
        if let Ok(h) = Header::from_bytes("Cache-Control", "no-store") {
            response = response.with_header(h);
        }
        let _ = request.respond(response);
    }
    Ok(())
}

fn route(
    work_dir: &Path,
    jobs: &JobQueue,
    method: &Method,
    url: &str,
    request: &mut Request,
) -> (i32, String, String) {
    let path = url.split('?').next().unwrap_or(url);

    // static assets (no auth — no secrets)
    // Offline copy of https://cdn.tailwindcss.com
    if matches!(method, Method::Get) && path == "/tailwindcss.js" {
        return (
            200,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/static/tailwindcss.js"
            ))
            .to_string(),
            "application/javascript; charset=utf-8".into(),
        );
    }
    if matches!(method, Method::Get) && path == "/site.css" {
        return (
            200,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/site.css")).to_string(),
            "text/css; charset=utf-8".into(),
        );
    }
    if matches!(method, Method::Get) && (path == "/" || path == "/index.html") {
        return (200, html_page(), "text/html; charset=utf-8".into());
    }

    // API auth when token configured
    if path.starts_with("/api/") {
        if let Err(e) = web_security::authorize(request) {
            return json_status(
                401,
                &serde_json::json!({"ok": false, "error": e}).to_string(),
            );
        }
    }

    match (method, path) {
        (Method::Get, "/api/health") => json_ok(&public_health(work_dir).to_string()),
        (Method::Get, "/api/runs") => json_ok(&runs_json(work_dir)),
        (Method::Get, "/api/stats") => {
            let (runs, ok, arts) = db::stats(work_dir).unwrap_or((0, 0, 0));
            let mut v = public_health(work_dir);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("runs".into(), runs.into());
                obj.insert("ok".into(), ok.into());
                obj.insert("artifacts".into(), arts.into());
                obj.insert("job_running".into(), jobs.is_running().into());
            }
            json_ok(&v.to_string())
        }
        (Method::Get, "/api/jobs") => {
            json_ok(&serde_json::to_string_pretty(&jobs.list()).unwrap_or_else(|_| "[]".into()))
        }
        (Method::Get, "/api/commands") => json_ok(COMMANDS_CATALOG),
        (Method::Post, "/api/pipeline") => post_job(request, |body| {
            let v = parse_body(&body);
            let mut args = jobs::pipeline_args_from_json(&v);
            args.input = safe_rel_path(args.input.to_string_lossy().as_ref())?;
            jobs.start_pipeline(work_dir.to_path_buf(), args)
        }),
        (Method::Post, "/api/scan") => post_job(request, |body| {
            let v = parse_body(&body);
            let mut args = jobs::scan_args_from_json(&v);
            args.input = safe_rel_path(args.input.to_string_lossy().as_ref())?;
            jobs.start_scan(work_dir.to_path_buf(), args)
        }),
        (Method::Post, "/api/exec") => post_job(request, |body| {
            let v = parse_body(&body);
            let argv = parse_argv(&v)?;
            jobs.start_argv(work_dir.to_path_buf(), argv)
        }),
        _ => (
            404,
            serde_json::json!({"error": "not found"}).to_string(),
            "application/json; charset=utf-8".into(),
        ),
    }
}

fn public_health(work_dir: &Path) -> serde_json::Value {
    let mut v = workdir::health_json(work_dir);
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "work_dir".into(),
            serde_json::Value::String(mask_work_dir(work_dir)),
        );
        obj.insert("work_dir_full".into(), serde_json::Value::Null);
    }
    v
}

fn post_job<F>(request: &mut Request, f: F) -> (i32, String, String)
where
    F: FnOnce(String) -> Result<String, String>,
{
    let body = read_body(request);
    match f(body) {
        Ok(id) => json_status(
            200,
            &serde_json::json!({"ok": true, "job_id": id}).to_string(),
        ),
        Err(e) => {
            let code = if e.starts_with("unauthorized") {
                401
            } else {
                409
            };
            json_status(
                code,
                &serde_json::json!({"ok": false, "error": e}).to_string(),
            )
        }
    }
}

fn json_ok(s: &str) -> (i32, String, String) {
    json_status(200, s)
}

fn json_status(code: i32, s: &str) -> (i32, String, String) {
    (
        code,
        s.to_string(),
        "application/json; charset=utf-8".into(),
    )
}

fn runs_json(work_dir: &Path) -> String {
    let runs = db::list_runs(work_dir, 100).unwrap_or_default();
    let json: Vec<_> = runs
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id, "kind": r.kind, "profile": r.profile, "preset": r.preset,
                "status": r.status, "e2e_ok": r.e2e_ok, "working_count": r.working_count,
                "notes": r.notes, "created_at": r.created_at,
            })
        })
        .collect();
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "[]".into())
}

fn read_body(request: &mut Request) -> String {
    let mut buf = Vec::new();
    let _ = std::io::Read::read_to_end(&mut request.as_reader(), &mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

fn parse_body(body: &str) -> serde_json::Value {
    if body.trim_start().starts_with('{') {
        return serde_json::from_str(body).unwrap_or(serde_json::json!({}));
    }
    let mut map = serde_json::Map::new();
    for pair in body.split('&') {
        let mut it = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (it.next(), it.next()) {
            map.insert(url_decode(k), serde_json::Value::String(url_decode(v)));
        }
    }
    serde_json::Value::Object(map)
}

fn url_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_argv(v: &serde_json::Value) -> Result<Vec<String>, String> {
    if let Some(arr) = v.get("argv").and_then(|x| x.as_array()) {
        let out: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect();
        if out.is_empty() {
            return Err("argv empty".into());
        }
        return Ok(out);
    }
    if let Some(line) = v.get("cmdline").and_then(|x| x.as_str()) {
        let parts = shell_split(line);
        if parts.is_empty() {
            return Err("cmdline empty".into());
        }
        return Ok(parts);
    }
    Err("need argv[] or cmdline".into())
}

fn shell_split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in line.chars() {
        match c {
            '"' => in_q = !in_q,
            ' ' | '\t' if !in_q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

const COMMANDS_CATALOG: &str = r#"{
  "note": "Auth: DNS_CLI_WEB_TOKEN via Bearer / X-DNS-CLI-Token when set.",
  "blocked": ["serve", "menu", "backup watch", "completion"],
  "security": "relative paths only; work_dir masked in API"
}"#;

fn html_page() -> String {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/index.html")).to_string()
}
