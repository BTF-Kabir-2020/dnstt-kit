//! Interactive MENU — common tasks without remembering flags.

use crate::backup::{self, BackupMode, BackupOpts};
use crate::config;
use crate::db;
use crate::doctor;
use crate::jobs;
use crate::pipeline;
use crate::workdir;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path) -> AppResult {
    println!("═══ dnstt-kit MENU ═══");
    println!("work_dir={}", work_dir.display());
    if !workdir::looks_like_kit(work_dir) {
        println!("WARNING: inja mesle rishe-ye kit nist.");
        println!("Az folder dnstt-kit ejra kon, ya aval `init` bezan.");
    }
    loop {
        println!();
        println!("  1) doctor     — health check (profile / slipnet / db)");
        println!("  2) scan       — DNS scan-e sari (preset low)");
        println!("  3) pipeline   — scan + generate config (NetMod/DNSTT/SlipNet)");
        println!("  4) backup     — backup az kit");
        println!("  5) profiles   — list profile-ha");
        println!("  6) status     — SQLite status");
        println!("  7) serve      — web panel  http://127.0.0.1:8787");
        println!("  8) init       — folder + profiles.json");
        println!("  9) info       — build / path info");
        println!("  s) sanitize   — clean/dedup/sort input IP list (dnsir.txt)");
        println!("  d) generate dns — IP list → dns.txt (flat / Python-style)");
        println!("  0) exit");
        print!("Select (0-9, s, d): ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        match line.trim() {
            "0" | "q" | "quit" | "exit" => {
                println!("bye.");
                return Ok(());
            }
            "1" => {
                let _ = doctor::run(work_dir, true);
            }
            "2" => {
                let args = jobs::default_scan_args(
                    PathBuf::from("testdata/dns_sample.txt"),
                    "low".into(),
                    Some(6),
                );
                report("scan", crate::scan_cmd::run(work_dir, args));
            }
            "3" => {
                let args = jobs::default_pipeline_args(
                    PathBuf::from("testdata/dns_sample.txt"),
                    "demo".into(),
                    "low".into(),
                    Some(6),
                );
                report("pipeline", pipeline::run(work_dir, args));
            }
            "4" => {
                let opts = BackupOpts {
                    mode: BackupMode::Kit,
                    keep: 15,
                    include_runs: false,
                    include_vendor: false,
                    include_secrets: false,
                    label: Some("menu".into()),
                };
                report("backup", backup::create(work_dir, &opts));
            }
            "5" => report("profiles", list_profiles(work_dir)),
            "6" => report("status", show_status(work_dir)),
            "7" => {
                println!("Web panel: http://127.0.0.1:8787  — Ctrl+C to stop");
                return crate::web::serve(work_dir, "127.0.0.1:8787");
            }
            "8" => report("init", crate::init_cmd::run(work_dir, false)),
            "9" => {
                println!("dns-cli {}", env!("CARGO_PKG_VERSION"));
                println!("work_dir={}", work_dir.display());
                println!(
                    "os={} arch={}",
                    std::env::consts::OS,
                    std::env::consts::ARCH
                );
            }
            "s" => {
                print!("Input file [dnsir.txt]: ");
                let _ = io::stdout().flush();
                let mut path = String::new();
                io::stdin()
                    .read_line(&mut path)
                    .map_err(|e| e.to_string())?;
                let path = path.trim();
                let path = if path.is_empty() {
                    PathBuf::from("dnsir.txt")
                } else {
                    PathBuf::from(path)
                };
                let args = crate::sanitize::SanitizeArgs {
                    input: path,
                    output: None,
                    dedup_by_ip: false,
                    exclude: None,
                    dry_run: false,
                    inplace: true,
                };
                report("sanitize", crate::sanitize::run(work_dir, args));
            }
            "d" => report("generate dns", run_flat_dns(work_dir)),
            _ => println!("invalid option — 0..9 / s / d"),
        }
    }
}

fn prompt_line(label: &str, default: &str) -> Result<String, String> {
    if default.is_empty() {
        print!("{label}: ");
    } else {
        print!("{label} [{default}]: ");
    }
    let _ = io::stdout().flush();
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    let t = line.trim();
    if t.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(t.to_string())
    }
}

fn run_flat_dns(work_dir: &Path) -> AppResult {
    let input = prompt_line("IP list file", "testdata/iran_dns_ips.txt")?;
    let out = prompt_line("Output file", "dns.txt")?;
    let profile = prompt_line("Profile (empty = use --ns/--pubkey flags)", "")?;
    let ps = prompt_line("ps (display name)", "NEWJJ")?;
    if profile.is_empty() {
        let ns = prompt_line("ns (tunnel domain)", "")?;
        let pubkey = prompt_line("pubkey (hex)", "")?;
        let user = prompt_line("user", "root")?;
        let pass = prompt_line("pass", "")?;
        crate::generate::flat_dns_cmd(
            work_dir,
            PathBuf::from(input),
            PathBuf::from(out),
            None,
            Some(ps),
            Some(ns),
            Some(pubkey),
            Some(user),
            Some(pass),
            53,
            true,
            None,
        )
    } else {
        crate::generate::flat_dns_cmd(
            work_dir,
            PathBuf::from(input),
            PathBuf::from(out),
            Some(profile.as_str()),
            Some(ps),
            None,
            None,
            None,
            None,
            53,
            true,
            None,
        )
    }
}

fn report(name: &str, r: AppResult) {
    match r {
        Ok(()) => println!("OK  {name} done"),
        Err(e) => println!("ERR {name}: {e}"),
    }
}

fn list_profiles(work_dir: &Path) -> AppResult {
    let p = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
    let mut names: Vec<_> = p.profiles.keys().cloned().collect();
    names.sort();
    if names.is_empty() {
        println!("(empty) — aval `init` ya profiles.json check kon");
    }
    for n in names {
        let pr = &p.profiles[&n];
        println!("{n}\tns={}\tssh={}", pr.tunnel_domain, pr.include_ssh);
    }
    Ok(())
}

fn show_status(work_dir: &Path) -> AppResult {
    let rows = db::list_runs(work_dir, 12).map_err(|e| e.to_string())?;
    if rows.is_empty() {
        println!("(no runs yet)");
        return Ok(());
    }
    for r in rows {
        println!("{}\t{}\t{}\t{}", r.created_at, r.id, r.kind, r.status);
    }
    Ok(())
}
