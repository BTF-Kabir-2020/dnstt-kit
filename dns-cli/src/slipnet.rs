//! کشف و (اختیاری) دانلود باینری slipnet از GitHub Releases.
//!
//! پیش‌فرض: فقط محلی/vendor. دانلود فقط با `slipnet fetch` یا `--fetch-slipnet`.

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub type AppResult = Result<(), String>;

/// نسخهٔ CLI پایدار که assetهای دسکتاپ دارد (نه فقط APK).
pub const DEFAULT_SLIPNET_TAG: &str = "v2.5.3";
pub const SLIPNET_REPO: &str = "anonvector/SlipNet";

pub fn platform_triple() -> &'static str {
    if cfg!(all(windows, target_arch = "x86_64")) {
        "windows-x86_64"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x86_64"
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "linux-aarch64"
    } else {
        "unknown"
    }
}

/// نام asset در GitHub release
pub fn github_asset_name() -> Result<&'static str, String> {
    match platform_triple() {
        "windows-x86_64" => Ok("slipnet-windows-amd64.exe"),
        "linux-x86_64" => Ok("slipnet-linux-amd64"),
        "linux-aarch64" => Ok("slipnet-linux-arm64"),
        other => Err(format!("no slipnet asset mapping for platform {other}")),
    }
}

pub fn vendor_bin_path(work_dir: &Path) -> PathBuf {
    let triple = platform_triple();
    let dir = work_dir.join("vendor").join("slipnet").join(triple);
    if cfg!(windows) {
        dir.join("slipnet.exe")
    } else {
        dir.join("slipnet")
    }
}

pub fn find_slipnet(work_dir: &Path, override_path: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(p) = override_path {
        let p = if p.is_absolute() { p } else { work_dir.join(p) };
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!("slipnet not found at {}", p.display()));
    }

    if let Ok(p) = env::var("SLIPNET_PATH") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Ok(p);
        }
    }

    let exe_names: &[&str] = if cfg!(windows) {
        &["slipnet.exe", "slipnet", "slipnet-windows-amd64.exe"]
    } else {
        &["slipnet", "slipnet-linux-amd64", "slipnet-linux-arm64"]
    };

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in exe_names {
                let c = dir.join(name);
                if c.is_file() {
                    return Ok(c);
                }
            }
        }
    }

    let vendor = vendor_bin_path(work_dir);
    if vendor.is_file() {
        return Ok(vendor);
    }
    let triple = platform_triple();
    let vendor_dir = work_dir.join("vendor").join("slipnet").join(triple);
    for name in exe_names {
        let c = vendor_dir.join(name);
        if c.is_file() {
            return Ok(c);
        }
    }

    for name in exe_names {
        let c = work_dir.join(name);
        if c.is_file() {
            return Ok(c);
        }
    }

    Err(format!(
        "slipnet پیدا نشد. گزینه‌ها:\n\
         • dns-cli slipnet fetch   (دانلود از GitHub — نیاز به نت)\n\
         • کپی دستی به {}\n\
         • --slipnet PATH / SLIPNET_PATH",
        vendor.display()
    ))
}

/// اگر محلی نبود و allow_fetch، از GitHub می‌گیرد.
pub fn find_or_fetch(
    work_dir: &Path,
    override_path: Option<PathBuf>,
    allow_fetch: bool,
    force: bool,
    tag: &str,
) -> Result<PathBuf, String> {
    if !force {
        if let Ok(p) = find_slipnet(work_dir, override_path.clone()) {
            return Ok(p);
        }
    }
    if allow_fetch {
        return fetch_to_vendor(work_dir, tag, force);
    }
    find_slipnet(work_dir, override_path)
}

pub fn fetch_to_vendor(work_dir: &Path, tag: &str, force: bool) -> Result<PathBuf, String> {
    let dest = vendor_bin_path(work_dir);
    if dest.is_file() && !force {
        println!("ℹ️  slipnet از قبل هست: {}", dest.display());
        return Ok(dest);
    }
    let asset = github_asset_name()?;
    let url = format!("https://github.com/{SLIPNET_REPO}/releases/download/{tag}/{asset}");
    println!("⬇️  دانلود slipnet از GitHub:\n   {url}");
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("download");
    download_file(&url, &tmp)?;
    // rename
    let _ = fs::remove_file(&dest);
    fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&dest, perms);
    }
    // VERSION note
    if let Some(parent) = dest.parent() {
        let _ = fs::write(parent.join("VERSION"), format!("{tag}\n{asset}\n{url}\n"));
    }
    println!("✅ ذخیره شد: {}", dest.display());
    Ok(dest)
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let resp = ureq::get(url)
        .set("User-Agent", "dnstt-kit/0.1")
        .call()
        .map_err(|e| format!("دانلود ناموفق (نت/فیلتر؟): {e}"))?;
    if !(200..300).contains(&resp.status()) {
        return Err(format!("HTTP {}", resp.status()));
    }
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        total += n as u64;
    }
    file.flush().map_err(|e| e.to_string())?;
    if total < 100_000 {
        let _ = fs::remove_file(dest);
        return Err(format!(
            "فایل دانلود شده خیلی کوچک است ({total} bytes) — احتمالاً HTML خطا"
        ));
    }
    println!("   دریافت شد: {total} bytes");
    Ok(())
}

pub fn which_cmd(work_dir: &Path, path: Option<PathBuf>) -> AppResult {
    match find_slipnet(work_dir, path) {
        Ok(p) => {
            println!("{}", p.display());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn fetch_cmd(work_dir: &Path, tag: String, force: bool) -> AppResult {
    let p = fetch_to_vendor(work_dir, &tag, force)?;
    println!("{}", p.display());
    Ok(())
}

/// اجرای سبک باینری (مثلاً --help) برای اطمینان از وجود و قابل‌اجرا بودن
pub fn probe(work_dir: &Path, override_path: Option<PathBuf>) -> Result<String, String> {
    let bin = find_slipnet(work_dir, override_path)?;
    let out = Command::new(&bin)
        .arg("--help")
        .output()
        .or_else(|_| Command::new(&bin).arg("-h").output())
        .or_else(|_| Command::new(&bin).output())
        .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
    // بعضی باینری‌ها --help را با exit!=0 برمی‌گردانند؛ مهم اجرا شدن است
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}{stderr}");
    if combined.is_empty() && !out.status.success() {
        return Err(format!(
            "slipnet exited {:?} with empty output",
            out.status.code()
        ));
    }
    Ok(format!(
        "{} (exit={:?}, out_len={})",
        bin.display(),
        out.status.code(),
        combined.len()
    ))
}

pub fn probe_cmd(work_dir: &Path, path: Option<PathBuf>) -> AppResult {
    let msg = probe(work_dir, path)?;
    println!("✅ {msg}");
    Ok(())
}
