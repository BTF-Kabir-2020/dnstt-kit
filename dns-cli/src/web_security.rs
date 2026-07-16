//! سخت‌سازی پنل وب: توکن، ماسک مسیر، جلوگیری از path traversal.

use std::path::{Component, Path, PathBuf};

/// نمایش امن مسیر برای UI/API — بدون مسیر کامل دیسک.
/// Separators `/` and `\` both count so Windows paths mask correctly on Unix CI too.
pub fn mask_work_dir(work_dir: &Path) -> String {
    let raw = work_dir.to_string_lossy();
    let name = raw
        .rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or("kit");
    format!("…/{name}")
}

/// فقط مسیر نسبی امن داخل work_dir — رد absolute / .. / UNC.
pub fn safe_rel_path(raw: &str) -> Result<PathBuf, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("path empty".into());
    }
    if s.contains('\0') {
        return Err("path invalid".into());
    }
    // Windows drive / UNC / Unix absolute
    if Path::new(s).is_absolute()
        || s.starts_with('/')
        || s.starts_with('\\')
        || s.chars().nth(1) == Some(':')
        || s.starts_with("\\\\")
    {
        return Err("absolute paths not allowed from web API".into());
    }
    let p = PathBuf::from(s);
    for c in p.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err("path '..' not allowed".into()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths not allowed from web API".into());
            }
        }
    }
    Ok(p)
}

pub fn web_token() -> Option<String> {
    std::env::var("DNS_CLI_WEB_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// اگر توکن ست شده باشد، هدر باید مطابقت کند.
pub fn authorize(request: &tiny_http::Request) -> Result<(), String> {
    let Some(expected) = web_token() else {
        return Ok(());
    };
    for h in request.headers() {
        let name = h.field.as_str().as_str();
        let val = h.value.as_str();
        if name.eq_ignore_ascii_case("Authorization") {
            let prefix = "Bearer ";
            if let Some(rest) = val.strip_prefix(prefix) {
                if rest == expected {
                    return Ok(());
                }
            }
            if val == expected {
                return Ok(());
            }
        }
        if name.eq_ignore_ascii_case("X-DNS-CLI-Token") && val == expected {
            return Ok(());
        }
    }
    // query ?token= for simple browser testing (still localhost)
    if let Some(q) = request.url().split('?').nth(1) {
        for pair in q.split('&') {
            let mut it = pair.splitn(2, '=');
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                if k == "token" && v == expected {
                    return Ok(());
                }
            }
        }
    }
    Err("unauthorized: set Authorization Bearer or X-DNS-CLI-Token".into())
}

pub fn warn_if_non_loopback(bind: &str) {
    let lower = bind.to_ascii_lowercase();
    if !(lower.starts_with("127.0.0.1")
        || lower.starts_with("localhost")
        || lower.starts_with("[::1]"))
    {
        eprintln!("⚠️  SECURITY: bind={bind} is not loopback.");
        eprintln!("   Prefer 127.0.0.1 and a reverse proxy. Set DNS_CLI_WEB_TOKEN.");
    }
    if web_token().is_none() {
        eprintln!("ℹ️  Tip: set DNS_CLI_WEB_TOKEN in .env to require API auth.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert!(safe_rel_path("../etc/passwd").is_err());
        assert!(safe_rel_path("C:\\\\Windows").is_err());
        assert!(safe_rel_path("/etc/passwd").is_err());
        assert!(safe_rel_path("testdata/dns_sample.txt").is_ok());
    }

    #[test]
    fn masks_dir() {
        let win = mask_work_dir(Path::new(r"C:\Users\x\dnstt-kit"));
        assert_eq!(win, "…/dnstt-kit");
        assert!(!win.contains("Users"));

        let unix = mask_work_dir(Path::new("/home/x/dnstt-kit"));
        assert_eq!(unix, "…/dnstt-kit");
        assert!(!unix.contains("home"));
    }
}
