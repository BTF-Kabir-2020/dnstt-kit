//! بارگذاری فایل `.env` از work_dir / cwd.
//!
//! **چرا؟** تا کاربر مجبور نباشد هر بار در PowerShell/Bash متغیر ست کند.
//! مقادیر داخل `.env` فقط اگر هنوز در محیط OS ست نشده باشند اعمال می‌شوند
//! (پس `export` دستی اولویت دارد).

use std::fs;
use std::path::{Path, PathBuf};

/// متن کامل `.env.example` — هم‌تراز با فایل ریشهٔ repo (برای `dns-cli init`).
/// اگر فایل repo را ویرایش کردی، این ثابت را هم به‌روز کن یا از include استفاده کن.
pub const ENV_EXAMPLE_TEXT: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../.env.example"));

/// تلاش برای لود `.env` از چند مسیر رایج. خطا نادیده گرفته می‌شود (فایل اختیاری است).
pub fn load_dotenv_files(work_dir: &Path) -> Vec<PathBuf> {
    let mut loaded = Vec::new();
    let candidates = [
        work_dir.join(".env"),
        PathBuf::from(".env"),
        // اگر از پوشهٔ بالاتر اجرا شد
        work_dir.join("dnstt-kit").join(".env"),
    ];
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        // از مسیر تکراری جلوگیری
        if loaded.iter().any(|p: &PathBuf| p == &path) {
            continue;
        }
        match dotenvy::from_path(&path) {
            Ok(_) => loaded.push(path),
            Err(e) => eprintln!("⚠️  خواندن {} ناموفق: {e}", path.display()),
        }
    }
    loaded
}

/// خلاصهٔ وضعیت متغیرهای محیطی مرتبط (بدون چاپ رمز کامل).
pub fn summarize_for_user() -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "DNS_CLI_WORK_DIR={}",
        mask_or_show(std::env::var("DNS_CLI_WORK_DIR").ok(), false)
    ));
    lines.push(format!(
        "SLIPNET_PATH={}",
        mask_or_show(std::env::var("SLIPNET_PATH").ok(), false)
    ));
    lines.push(format!(
        "SLIPNET_CONFIG={}",
        mask_or_show(std::env::var("SLIPNET_CONFIG").ok(), true)
    ));
    lines.push(format!(
        "DNS_CLI_BIND={}",
        mask_or_show(std::env::var("DNS_CLI_BIND").ok(), false)
    ));
    lines.join("\n")
}

fn mask_or_show(v: Option<String>, secretish: bool) -> String {
    match v {
        None => "(unset)".into(),
        Some(s) if s.trim().is_empty() => "(empty)".into(),
        Some(s) if secretish => {
            let t = s.trim();
            if t.len() <= 24 {
                format!("set(len={}, prefix={:?}…)", t.len(), &t[..t.len().min(12)])
            } else {
                format!(
                    "set(len={}, starts_with={:?})",
                    t.len(),
                    &t[..16.min(t.len())]
                )
            }
        }
        Some(s) => s,
    }
}

/// نوشتن `.env.example` اگر نبود (برای init).
pub fn ensure_env_example(work_dir: &Path) -> Result<PathBuf, String> {
    let path = work_dir.join(".env.example");
    // همیشه از نسخهٔ داخل باینری هم‌تراز نگه دار اگر فایل خیلی کوتاه/قدیمی است
    let need_write = match fs::read_to_string(&path) {
        Ok(existing) => existing.lines().count() < 10,
        Err(_) => true,
    };
    if need_write {
        fs::write(&path, ENV_EXAMPLE_TEXT).map_err(|e| e.to_string())?;
    }
    Ok(path)
}
