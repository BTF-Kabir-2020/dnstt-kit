//! صف کار پس‌زمینه برای پنل وب — scan/pipeline اختصاصی + هر argv از CLI.

use crate::pipeline::{self, PipelineArgs};
use crate::scan_cmd::{self, ScanArgs};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub message: String,
    pub log: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Default)]
struct Inner {
    jobs: Vec<JobInfo>,
    running: bool,
}

#[derive(Clone, Default)]
pub struct JobQueue {
    inner: Arc<Mutex<Inner>>,
}

impl JobQueue {
    pub fn list(&self) -> Vec<JobInfo> {
        self.inner.lock().unwrap().jobs.clone()
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().running
    }

    pub fn start_pipeline(&self, work_dir: PathBuf, args: PipelineArgs) -> Result<String, String> {
        self.spawn("pipeline", work_dir, move |wd| {
            pipeline::run(&wd, args).map(|_| "pipeline تمام شد".into())
        })
    }

    pub fn start_scan(&self, work_dir: PathBuf, args: ScanArgs) -> Result<String, String> {
        self.spawn("scan", work_dir, move |wd| {
            scan_cmd::run(&wd, args).map(|_| "scan تمام شد".into())
        })
    }

    /// اجرای هر زیر‌دستور dns-cli به‌صورت subprocess (هم‌ترازی کامل با CLI)
    pub fn start_argv(&self, work_dir: PathBuf, argv: Vec<String>) -> Result<String, String> {
        validate_argv(&argv)?;
        let kind = argv.first().cloned().unwrap_or_else(|| "exec".into());
        self.spawn(&kind, work_dir, move |wd| run_argv(&wd, &argv))
    }

    fn spawn<F>(&self, kind: &str, work_dir: PathBuf, f: F) -> Result<String, String>
    where
        F: FnOnce(PathBuf) -> Result<String, String> + Send + 'static,
    {
        let mut g = self.inner.lock().unwrap();
        if g.running {
            return Err("یک کار دیگر در حال اجراست — صبر کن تا تمام شود، بعد دوباره بزن.".into());
        }
        let id = format!("job_{}", chrono::Local::now().format("%Y%m%d_%H%M%S_%f"));
        let info = JobInfo {
            id: id.clone(),
            kind: kind.into(),
            status: "running".into(),
            message: format!("شروع {kind}"),
            log: String::new(),
            started_at: chrono::Local::now().to_rfc3339(),
            finished_at: None,
        };
        g.jobs.insert(0, info);
        g.jobs.truncate(50);
        g.running = true;
        drop(g);

        let q = self.clone();
        let id2 = id.clone();
        thread::spawn(move || {
            let result = f(work_dir);
            let mut g = q.inner.lock().unwrap();
            g.running = false;
            if let Some(j) = g.jobs.iter_mut().find(|j| j.id == id2) {
                match result {
                    Ok(msg) => {
                        j.status = "ok".into();
                        j.message = msg.chars().take(400).collect();
                        j.log = j.message.clone();
                    }
                    Err(e) => {
                        j.status = "error".into();
                        j.message = e.chars().take(800).collect();
                        j.log = j.message.clone();
                    }
                }
                j.finished_at = Some(chrono::Local::now().to_rfc3339());
            }
        });
        Ok(id)
    }
}

fn validate_argv(argv: &[String]) -> Result<(), String> {
    if argv.is_empty() {
        return Err("argv empty".into());
    }
    let cmd = argv[0].as_str();
    if matches!(cmd, "serve" | "menu" | "completion") {
        return Err(format!("{cmd} از وب مجاز نیست"));
    }
    if cmd == "backup" && argv.get(1).map(|s| s.as_str()) == Some("watch") {
        return Err("backup watch از وب مجاز نیست".into());
    }
    const ALLOWED: &[&str] = &[
        "init",
        "scan",
        "resolvers",
        "generate",
        "pipeline",
        "slipnet",
        "archive",
        "backup",
        "clean",
        "profiles",
        "doctor",
        "verify",
        "status",
        "info",
    ];
    if !ALLOWED.contains(&cmd) {
        return Err(format!("command not allowed from web: {cmd}"));
    }
    Ok(())
}

fn run_argv(work_dir: &std::path::Path, argv: &[String]) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(exe);
    cmd.arg("--work-dir").arg(work_dir);
    for a in argv {
        cmd.arg(a);
    }
    let out = cmd.output().map_err(|e| format!("spawn: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    let tail: String = combined
        .chars()
        .rev()
        .take(1200)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if !stdout.trim().is_empty() {
        print!("{stdout}");
    }
    if !stderr.trim().is_empty() {
        eprint!("{stderr}");
    }
    if out.status.success() {
        Ok(if tail.trim().is_empty() {
            "تمام شد".into()
        } else {
            tail
        })
    } else {
        Err(format!(
            "exit {:?} | {}",
            out.status.code(),
            if tail.trim().is_empty() {
                "(بدون خروجی)".into()
            } else {
                tail
            }
        ))
    }
}

pub fn pipeline_args_from_json(v: &serde_json::Value) -> PipelineArgs {
    let input = PathBuf::from(str_field(v, "input", "testdata/dns_sample.txt"));
    let profile = str_field(v, "profile", "mame").to_string();
    let preset = str_field(v, "preset", "low").to_string();
    let limit = opt_usize(v, "limit");
    let mut args = default_pipeline_args(input, profile, preset, limit);
    if let Some(b) = opt_bool(v, "skip_slipnet") {
        args.skip_slipnet = b;
    }
    if let Some(b) = opt_bool(v, "auto_archive") {
        args.auto_archive = b;
    }
    if let Some(b) = opt_bool(v, "auto_backup") {
        args.auto_backup = b;
    }
    if let Some(b) = opt_bool(v, "no_dmvpn") {
        args.no_dmvpn = b;
    }
    if let Some(b) = opt_bool(v, "quiet") {
        args.quiet = b;
    }
    if let Some(b) = opt_bool(v, "slipnet_probe") {
        args.slipnet_probe = b;
    }
    if let Some(b) = opt_bool(v, "dry_run") {
        args.dry_run = b;
    }
    if let Some(s) = v.get("generate_kinds").and_then(|x| x.as_str()) {
        args.generate_kinds = s.to_string();
    }
    args
}

pub fn scan_args_from_json(v: &serde_json::Value) -> ScanArgs {
    let input = PathBuf::from(str_field(v, "input", "testdata/dns_sample.txt"));
    let preset = str_field(v, "preset", "low").to_string();
    let limit = opt_usize(v, "limit");
    let mut args = default_scan_args(input, preset, limit);
    if let Some(b) = opt_bool(v, "quiet") {
        args.quiet = b;
    }
    if let Some(b) = opt_bool(v, "enable_tcp") {
        args.enable_tcp = b;
    }
    if let Some(b) = opt_bool(v, "ok_only") {
        args.ok_only = b;
    }
    if let Some(b) = opt_bool(v, "no_ping") {
        args.no_ping = b;
    }
    args
}

fn str_field<'a>(v: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or(default)
}

fn opt_usize(v: &serde_json::Value, key: &str) -> Option<usize> {
    v.get(key).and_then(|x| {
        x.as_u64()
            .map(|n| n as usize)
            .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
    })
}

fn opt_bool(v: &serde_json::Value, key: &str) -> Option<bool> {
    v.get(key).and_then(|x| {
        x.as_bool().or_else(|| {
            x.as_str()
                .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        })
    })
}

pub fn default_pipeline_args(
    input: PathBuf,
    profile: String,
    preset: String,
    limit: Option<usize>,
) -> PipelineArgs {
    PipelineArgs {
        input,
        profile,
        preset,
        slipnet: None,
        require_slipnet: false,
        skip_slipnet: true,
        skip_scan: false,
        skip_generate: false,
        slipnet_config: None,
        run_id: None,
        fetch_slipnet: false,
        force_fetch_slipnet: false,
        limit,
        workers: None,
        dry_run: false,
        no_dmvpn: true,
        generate_kinds: "all".into(),
        quiet: true,
        auto_archive: false,
        auto_backup: false,
        slipnet_probe: false,
    }
}

pub fn default_scan_args(input: PathBuf, preset: String, limit: Option<usize>) -> ScanArgs {
    ScanArgs {
        input,
        preset,
        workers: None,
        timeout: None,
        no_ping: false,
        domain: None,
        a_domain: None,
        extra_domains: None,
        udp_attempts: 2,
        udp_backoff_ms: 150,
        stream: false,
        legacy_out: true,
        no_legacy_out: false,
        limit,
        ok_only: false,
        enable_tcp: false,
        quiet: true,
        run_id: None,
    }
}
