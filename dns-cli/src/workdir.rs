//! تشخیص پوشهٔ کاری kit — حتی اگر از `target/release` اجرا شود.

use std::path::{Path, PathBuf};

/// نشانه‌های ریشهٔ dnstt-kit
pub fn looks_like_kit(dir: &Path) -> bool {
    if dir.join("testdata").join("dns_sample.txt").is_file() {
        return true;
    }
    if dir.join("dns-cli").join("Cargo.toml").is_file()
        && dir.join("scanner-core").join("Cargo.toml").is_file()
    {
        return true;
    }
    dir.join(".env.example").is_file() && dir.join("docs").join("WEB.md").is_file()
}

/// از مسیر فعلی به بالا برو تا kit پیدا شود (حداکثر ۶ سطح).
pub fn climb_to_kit(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    for _ in 0..6 {
        if looks_like_kit(&cur) {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

/// اولویت: آرگومان صریح → تشخیص از cwd → تشخیص از محل باینری → cwd خام.
/// اگر cwd فقط `target/release|debug` باشد، ریشهٔ kit را برمی‌گرداند.
pub fn resolve(explicit: Option<PathBuf>) -> (PathBuf, Option<&'static str>) {
    if let Some(p) = explicit {
        return (p, None);
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if looks_like_kit(&cwd) {
        return (cwd, None);
    }
    if let Some(root) = climb_to_kit(&cwd) {
        return (
            root,
            Some("work_dir از cwd به ریشهٔ dnstt-kit ارتقا یافت (مثلاً target/release نبود)"),
        );
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Some(root) = climb_to_kit(parent) {
                return (root, Some("work_dir از محل باینری dns-cli تشخیص داده شد"));
            }
        }
    }

    (
        cwd,
        Some("هشدار: ریشهٔ kit پیدا نشد — testdata/profiles ممکن است نباشند"),
    )
}

/// خلاصهٔ سلامت work_dir برای API وب
pub fn health_json(work_dir: &Path) -> serde_json::Value {
    let testdata = work_dir.join("testdata").join("dns_sample.txt").is_file();
    let profiles = work_dir.join("config").join("profiles.json").is_file()
        || work_dir
            .join("config")
            .join("profiles.example.json")
            .is_file();
    let env_example = work_dir.join(".env.example").is_file();
    let ready = testdata && profiles;
    serde_json::json!({
        "work_dir": work_dir.display().to_string(),
        "looks_like_kit": looks_like_kit(work_dir),
        "testdata": testdata,
        "profiles": profiles,
        "env_example": env_example,
        "ready": ready,
        "hint": if ready {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(
                "پوشهٔ کاری اشتباه است. از ریشهٔ dnstt-kit با .\\dns-cli.cmd serve اجرا کن یا --work-dir را بگذار."
                    .into(),
            )
        }
    })
}
