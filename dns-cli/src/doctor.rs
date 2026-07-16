//! خودآزمایی محیط: باینری‌ها، پروفایل، SQLite، vendor، فضای دیسک تقریبی.

use crate::config;
use crate::db;
use crate::slipnet;
use std::fs;
use std::path::Path;

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path, fetch_hint: bool) -> AppResult {
    let mut ok = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;

    println!("🩺 doctor — work_dir={}", work_dir.display());

    // profiles
    match config::load_profiles(work_dir) {
        Ok(p) => {
            let names: Vec<_> = p.profiles.keys().cloned().collect();
            println!("✅ profiles: {} → {:?}", names.len(), names);
            ok += 1;
            for (k, v) in &p.profiles {
                if v.ssh_pass == "CHANGE_ME" || v.ssh_pass.is_empty() {
                    println!("⚠️  profile `{k}`: ssh_pass هنوز CHANGE_ME/خالی است");
                    warn += 1;
                }
                if v.pubkey.len() < 16 {
                    println!("⚠️  profile `{k}`: pubkey کوتاه به نظر می‌رسد");
                    warn += 1;
                }
            }
        }
        Err(e) => {
            println!("❌ profiles: {e}");
            fail += 1;
        }
    }

    // slipnet
    match slipnet::find_slipnet(work_dir, None) {
        Ok(p) => {
            let len = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            println!("✅ slipnet: {} ({len} bytes)", p.display());
            ok += 1;
        }
        Err(e) => {
            println!("⚠️  slipnet: {e}");
            if fetch_hint {
                println!("   → dns-cli slipnet fetch");
            }
            warn += 1;
        }
    }

    // sqlite
    match db::open(work_dir) {
        Ok(_) => {
            let (runs, oks, arts) = db::stats(work_dir).unwrap_or((0, 0, 0));
            println!(
                "✅ sqlite: {} (runs={runs} ok={oks} artifacts={arts})",
                db::db_path(work_dir).display()
            );
            ok += 1;
        }
        Err(e) => {
            println!("❌ sqlite: {e}");
            fail += 1;
        }
    }

    // testdata
    for name in ["testdata/dns_sample.txt", "testdata/resolvers_sample.json"] {
        let p = work_dir.join(name);
        if p.is_file() {
            println!("✅ {name}");
            ok += 1;
        } else {
            println!("⚠️  missing {name}");
            warn += 1;
        }
    }

    // .env
    let env_path = work_dir.join(".env");
    let example = work_dir.join(".env.example");
    if env_path.is_file() {
        println!("✅ .env present → {}", env_path.display());
        ok += 1;
        let summary = crate::env_file::summarize_for_user();
        for line in summary.lines() {
            println!("   {line}");
        }
    } else {
        println!("⚠️  .env نیست — از .env.example کپی کن (docs/ENV.md)");
        warn += 1;
        if example.is_file() {
            println!("   نمونه: {}", example.display());
        }
    }

    // dirs
    for name in [
        "config",
        "runs",
        "vendor/slipnet",
        "backups",
        "archives",
        "data",
    ] {
        let p = work_dir.join(name);
        if p.is_dir() {
            println!("✅ dir {name}/");
            ok += 1;
        } else {
            println!("⚠️  dir missing: {name}/ (dns-cli init)");
            warn += 1;
        }
    }

    // backups present?
    let bdir = work_dir.join("backups");
    if bdir.is_dir() {
        let n = fs::read_dir(&bdir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .map(|s| s.eq_ignore_ascii_case("zip"))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0);
        println!("ℹ️  backups zip count={n}");
    }

    println!("———");
    println!("summary: ok={ok} warn={warn} fail={fail}");
    if fail > 0 {
        Err(format!("doctor failed ({fail} errors)"))
    } else {
        Ok(())
    }
}
