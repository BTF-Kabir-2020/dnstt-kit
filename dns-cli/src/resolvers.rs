//! همگام‌سازی و نرمال‌سازی لیست رزالور.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub type AppResult = Result<(), String>;

fn work(p: &Path, rel: PathBuf) -> PathBuf {
    if rel.is_absolute() {
        rel
    } else {
        p.join(rel)
    }
}

/// استخراج IP / host:port از JSON خروجی اسکنر.
pub fn extract_ips_from_json(raw: &Value) -> Vec<String> {
    let mut out = Vec::new();

    fn add(out: &mut Vec<String>, s: &str) {
        let s = s.trim();
        if !s.is_empty() {
            out.push(s.to_string());
        }
    }

    match raw {
        Value::Array(arr) => {
            for item in arr {
                match item {
                    Value::String(s) => add(&mut out, s),
                    Value::Object(map) => {
                        for key in ["ip", "host", "original", "address"] {
                            if let Some(Value::String(v)) = map.get(key) {
                                let host = v.trim();
                                if let Some(Value::Number(port)) = map.get("port") {
                                    if let Some(p) = port.as_u64() {
                                        if p != 0 && p != 53 && !host.contains(':') {
                                            add(&mut out, &format!("{host}:{p}"));
                                            break;
                                        }
                                    }
                                }
                                add(&mut out, host);
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::Object(map) => {
            for key in ["ips", "ip_port", "results", "data", "ok_and_dnsonly_ips"] {
                if let Some(v) = map.get(key) {
                    return extract_ips_from_json(v);
                }
            }
        }
        _ => {}
    }
    out
}

pub fn load_txt_ips(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    Ok(text
        .lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn write_resolvers_json(path: &Path, ips: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(ips).map_err(|e| e.to_string())?;
    fs::write(path, body + "\n").map_err(|e| e.to_string())
}

/// برای slipnet: یک IP در هر خط، بدون `:53` و بدون BOM.
pub fn write_ips_txt(path: &Path, ips: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut lines = Vec::new();
    for s in ips {
        let mut s = s.trim().trim_start_matches('\u{feff}').to_string();
        if s.ends_with(":53") {
            s.truncate(s.len() - 3);
        }
        let s = s.trim();
        if !s.is_empty() {
            lines.push(s.to_string());
        }
    }
    fs::write(
        path,
        lines.join("\n") + if lines.is_empty() { "" } else { "\n" },
    )
    .map_err(|e| e.to_string())
}

pub fn normalize_list(ips: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for s in ips {
        let mut s = s.trim().to_string();
        if s.is_empty() {
            continue;
        }
        if !s.contains(':') && !s.starts_with("https://") && !s.starts_with("tls://") {
            // برای resolvers.json معمولاً ip یا ip:53؛ نرمال با :53
            s = format!("{s}:53");
        }
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

/// مرتب‌سازی عددی IPv4 (ساده؛ بقیه lexicographic).
pub fn sort_ips(mut ips: Vec<String>) -> Vec<String> {
    ips.sort_by(|a, b| {
        let ka = ipv4_key(a);
        let kb = ipv4_key(b);
        ka.cmp(&kb).then_with(|| a.cmp(b))
    });
    ips
}

fn ipv4_key(s: &str) -> (u8, [u8; 4]) {
    let host = s.split(':').next().unwrap_or(s);
    let parts: Vec<_> = host.split('.').collect();
    if parts.len() == 4 {
        if let (Ok(a), Ok(b), Ok(c), Ok(d)) = (
            parts[0].parse::<u8>(),
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
            parts[3].parse::<u8>(),
        ) {
            return (0, [a, b, c, d]);
        }
    }
    (1, [0, 0, 0, 0])
}

pub fn sync(
    work_dir: &Path,
    from_json: Option<PathBuf>,
    from_txt: Option<PathBuf>,
    out: PathBuf,
    limit: Option<usize>,
) -> AppResult {
    let json_path = from_json
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| work_dir.join("out/json/dns_ok_and_dnsonly_ips.json"));
    let txt_path = from_txt
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| work_dir.join("out/txt/dns_ok_and_dnsonly_ips.txt"));
    let out_path = work(work_dir, out);

    let mut ips = Vec::new();
    if json_path.is_file() {
        let text = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
        let text = text.trim_start_matches('\u{feff}');
        let raw: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        ips = extract_ips_from_json(&raw);
        println!("منبع JSON: {} → {} مورد", json_path.display(), ips.len());
    }
    if ips.is_empty() && txt_path.is_file() {
        ips = load_txt_ips(&txt_path)?;
        println!("fallback TXT: {} → {} مورد", txt_path.display(), ips.len());
    }
    let mut ips = normalize_list(ips);
    if let Some(n) = limit {
        ips.truncate(n);
        println!("limit → {} IP", ips.len());
    }
    write_resolvers_json(&out_path, &ips)?;
    let slip_txt = out_path.with_extension("txt");
    let slip_path = if out_path.file_name().and_then(|s| s.to_str()) == Some("resolvers.json") {
        work_dir.join("dns_ok_and_dnsonly_ips.txt")
    } else {
        slip_txt
    };
    write_ips_txt(&slip_path, &ips)?;
    println!(
        "✅ {} و {} به‌روز شدند.",
        out_path.display(),
        slip_path.display()
    );
    Ok(())
}

pub fn normalize_cmd(work_dir: &Path, input: PathBuf, out: Option<PathBuf>) -> AppResult {
    let input = work(work_dir, input);
    let text = fs::read_to_string(&input).map_err(|e| e.to_string())?;
    let text = text.trim_start_matches('\u{feff}');
    let ips: Vec<String> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let ips = normalize_list(ips);
    let out = out.map(|p| work(work_dir, p)).unwrap_or(input);
    write_resolvers_json(&out, &ips)?;
    println!("✅ normalize → {} ({} IP)", out.display(), ips.len());
    Ok(())
}

pub fn sort_cmd(work_dir: &Path, input: PathBuf, out: Option<PathBuf>) -> AppResult {
    let input = work(work_dir, input);
    let mut ips = load_resolvers_json(&input)?;
    ips = sort_ips(ips);
    let out = out.map(|p| work(work_dir, p)).unwrap_or(input);
    write_resolvers_json(&out, &ips)?;
    println!("✅ sort → {} ({} IP)", out.display(), ips.len());
    Ok(())
}

pub fn take_cmd(work_dir: &Path, input: PathBuf, n: usize, out: Option<PathBuf>) -> AppResult {
    let input = work(work_dir, input);
    let mut ips = load_resolvers_json(&input)?;
    ips.truncate(n);
    let out = out.map(|p| work(work_dir, p)).unwrap_or(input);
    write_resolvers_json(&out, &ips)?;
    println!("✅ take {n} → {} ({} IP)", out.display(), ips.len());
    Ok(())
}

pub fn shuffle_cmd(work_dir: &Path, input: PathBuf, out: Option<PathBuf>) -> AppResult {
    use rand::seq::SliceRandom;
    let input = work(work_dir, input);
    let mut ips = load_resolvers_json(&input)?;
    ips.shuffle(&mut rand::thread_rng());
    let out = out.map(|p| work(work_dir, p)).unwrap_or(input);
    write_resolvers_json(&out, &ips)?;
    println!("✅ shuffle → {} ({} IP)", out.display(), ips.len());
    Ok(())
}

pub fn merge_cmd(work_dir: &Path, inputs: Vec<PathBuf>, out: PathBuf) -> AppResult {
    let mut all = Vec::new();
    for p in inputs {
        let p = work(work_dir, p);
        let mut ips = load_resolvers_json(&p)?;
        all.append(&mut ips);
    }
    let ips = normalize_list(all);
    let out = work(work_dir, out);
    write_resolvers_json(&out, &ips)?;
    println!("✅ merge → {} ({} IP unique)", out.display(), ips.len());
    Ok(())
}

/// حذف IPهای موجود در فایل exclude از لیست اصلی (مثل remove_ips.py رقیب)
pub fn exclude_cmd(
    work_dir: &Path,
    input: PathBuf,
    exclude: PathBuf,
    out: Option<PathBuf>,
) -> AppResult {
    let input = work(work_dir, input);
    let exclude = work(work_dir, exclude);
    let mut ips = load_resolvers_json(&input)?;
    let bad = if exclude
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        load_resolvers_json(&exclude)?
    } else {
        load_txt_ips(&exclude)?
    };
    let bad_set: std::collections::HashSet<_> = normalize_list(bad).into_iter().collect();
    let before = ips.len();
    ips.retain(|ip| !bad_set.contains(ip));
    let out = out.map(|p| work(work_dir, p)).unwrap_or(input);
    write_resolvers_json(&out, &ips)?;
    println!(
        "✅ exclude → {} ({} → {} IP)",
        out.display(),
        before,
        ips.len()
    );
    Ok(())
}

/// خروجی یک IP در هر خط — مناسب SlipNet / MasterDnsVPN `client_resolvers.txt`.
pub fn export_txt_cmd(work_dir: &Path, input: PathBuf, out: PathBuf) -> AppResult {
    let input = work(work_dir, input);
    let out = work(work_dir, out);
    let ips = if input
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        load_resolvers_json(&input)?
    } else {
        normalize_list(load_txt_ips(&input)?)
    };
    write_ips_txt(&out, &ips)?;
    println!(
        "✅ export-txt → {} ({} IP) — drop into MasterDnsVPN as client_resolvers.txt",
        out.display(),
        ips.len()
    );
    Ok(())
}

pub fn load_resolvers_json(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let text = text.trim_start_matches('\u{feff}');
    let ips: Vec<String> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(normalize_list(ips))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_ipv4_numeric() {
        let v = sort_ips(vec![
            "10.0.0.2:53".into(),
            "10.0.0.10:53".into(),
            "8.8.8.8:53".into(),
        ]);
        assert_eq!(v[0], "8.8.8.8:53");
        assert_eq!(v[1], "10.0.0.2:53");
        assert_eq!(v[2], "10.0.0.10:53");
    }
}
