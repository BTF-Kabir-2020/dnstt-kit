//! راه‌اندازی اولیهٔ workspace (پوشه‌ها + profiles از نمونه + .env.example).

use crate::env_file;
use std::fs;
use std::path::Path;

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path, force_profiles: bool) -> AppResult {
    for d in [
        "config",
        "runs",
        "archives",
        "backups",
        "data",
        "logs",
        "vendor/slipnet",
        "dist",
        "out/txt",
        "out/json",
    ] {
        let p = work_dir.join(d);
        fs::create_dir_all(&p).map_err(|e| e.to_string())?;
        println!("📁 {}", p.display());
    }

    let example = work_dir.join("config/profiles.example.json");
    let profiles = work_dir.join("config/profiles.json");
    if example.is_file() && (!profiles.is_file() || force_profiles) {
        fs::copy(&example, &profiles).map_err(|e| e.to_string())?;
        println!("✅ wrote {}", profiles.display());
        println!("   ⚠️  ssh_pass را از CHANGE_ME عوض کن");
    } else if profiles.is_file() {
        println!("ℹ️  profiles.json موجود است (برای بازنویسی: --force-profiles)");
    } else {
        println!("⚠️  profiles.example.json پیدا نشد");
    }

    let env_ex = env_file::ensure_env_example(work_dir)?;
    println!("✅ .env.example → {}", env_ex.display());
    let env_path = work_dir.join(".env");
    if !env_path.is_file() {
        println!("➡️  قدم بعد: Copy-Item .env.example .env   سپس مقادیر را ویرایش کن");
        println!("   راهنما: docs/ENV.md");
    } else {
        println!("ℹ️  .env از قبل هست → {}", env_path.display());
    }

    let _ = crate::db::open(work_dir)?;
    println!(
        "✅ sqlite ready: {}",
        crate::db::db_path(work_dir).display()
    );
    println!("✅ init کامل");
    Ok(())
}
