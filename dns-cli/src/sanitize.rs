//! پاکسازی و مرتب‌سازی فایل ورودی IP (dnsir.txt / tokhmi.txt).
//!
//! حذف خطوط خالی / کامنت / فرمت نامعتبر، حذف تکراری، مرتب‌سازی عددی.

use clap::Args;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub type AppResult = Result<(), String>;

#[derive(Args, Debug)]
pub struct SanitizeArgs {
    /// فایل ورودی (مثل dnsir.txt)
    pub input: PathBuf,

    /// فایل خروجی. اگر خالی باشد: <input>.clean.txt
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// فقط بر اساس IP (بدون پورت) تکراری حذف شود
    #[arg(long)]
    pub dedup_by_ip: bool,

    /// فایل exclude (هر خط یک IP یا IP:port — حذف می‌شوند)
    #[arg(long)]
    pub exclude: Option<PathBuf>,

    /// فقط چاپ آمار بدون نوشتن فایل
    #[arg(long)]
    pub dry_run: bool,

    /// جایگزینی فایل اصلی (به‌جای نوشتن فایل جداگانه)
    #[arg(long)]
    pub inplace: bool,
}

pub fn run(work_dir: &Path, args: SanitizeArgs) -> AppResult {
    let input = if args.input.is_absolute() {
        args.input.clone()
    } else {
        work_dir.join(&args.input)
    };
    if !input.is_file() {
        return Err(format!("input not found: {}", input.display()));
    }

    let output = if args.inplace {
        input.clone()
    } else {
        args.output.clone().unwrap_or_else(|| {
            let stem = input
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "cleaned".into());
            let parent = input.parent().unwrap_or(work_dir);
            parent.join(format!("{stem}.clean.txt"))
        })
    };

    // --- load exclude list ---
    let mut exclude_set: HashSet<String> = HashSet::new();
    if let Some(ref excl_path) = args.exclude {
        let excl = if excl_path.is_absolute() {
            excl_path.clone()
        } else {
            work_dir.join(excl_path)
        };
        if excl.is_file() {
            let data = fs::read_to_string(&excl).map_err(|e| e.to_string())?;
            for line in data.lines() {
                let l = line.trim().to_string();
                if !l.is_empty() && !l.starts_with('#') {
                    exclude_set.insert(l);
                }
            }
            println!("ℹ️  exclude: {} entry loaded", exclude_set.len());
        }
    }

    // --- read & parse ---
    let raw = fs::read_to_string(&input).map_err(|e| e.to_string())?;
    let raw_lines: usize = raw.lines().count();

    struct Entry {
        ip: [u8; 4],
        port: u16,
        original: String,
    }

    let mut entries: Vec<Entry> = Vec::new();
    let mut skipped_blank = 0usize;
    let mut skipped_comment = 0usize;
    let mut skipped_bad = 0usize;
    let mut skipped_excluded = 0usize;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            skipped_blank += 1;
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            skipped_comment += 1;
            continue;
        }

        // support both "IP:port" and bare "IP" (default port 53)
        let (ip_str, port_str) = if let Some(colon_pos) = trimmed.rfind(':') {
            let ip_part = &trimmed[..colon_pos];
            let port_part = &trimmed[colon_pos + 1..];
            (ip_part, Some(port_part))
        } else {
            (trimmed, None)
        };

        let octets: Vec<&str> = ip_str.split('.').collect();
        if octets.len() != 4 {
            skipped_bad += 1;
            continue;
        }
        let ip: [u8; 4] = match octets.iter().map(|o| o.parse::<u8>()).collect::<Result<Vec<_>, _>>() {
            Ok(v) => [v[0], v[1], v[2], v[3]],
            Err(_) => {
                skipped_bad += 1;
                continue;
            }
        };
        let port: u16 = match port_str {
            Some(p) => match p.parse::<u16>() {
                Ok(v) => v,
                Err(_) => {
                    skipped_bad += 1;
                    continue;
                }
            },
            None => 53,
        };

        // exclude check
        let key_full = format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port);
        let key_ip = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
        if exclude_set.contains(&key_full) || exclude_set.contains(&key_ip) {
            skipped_excluded += 1;
            continue;
        }

        entries.push(Entry {
            ip,
            port,
            original: format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port),
        });
    }

    let before_dedup = entries.len();

    // --- dedup ---
    let mut seen_full: HashSet<String> = HashSet::new();
    let mut seen_ip: HashSet<String> = HashSet::new();
    let mut unique: Vec<Entry> = Vec::new();
    let mut skipped_dup = 0usize;

    for e in entries {
        let key_full = format!("{}.{}.{}.{}:{}", e.ip[0], e.ip[1], e.ip[2], e.ip[3], e.port);
        let key_ip = format!("{}.{}.{}.{}", e.ip[0], e.ip[1], e.ip[2], e.ip[3]);

        if args.dedup_by_ip {
            if seen_ip.contains(&key_ip) {
                skipped_dup += 1;
                continue;
            }
            seen_ip.insert(key_ip);
        } else {
            if seen_full.contains(&key_full) {
                skipped_dup += 1;
                continue;
            }
            seen_full.insert(key_full);
        }
        unique.push(e);
    }

    // --- sort numerically: IP then port ---
    unique.sort_by(|a, b| a.ip.cmp(&b.ip).then(a.port.cmp(&b.port)));

    let after_count = unique.len();

    // --- write ---
    if !args.dry_run {
        let mut out_str = String::with_capacity(after_count * 24);
        for e in &unique {
            out_str.push_str(&e.original);
            out_str.push('\n');
        }
        fs::write(&output, &out_str).map_err(|e| e.to_string())?;
    }

    // --- report ---
    println!("📊 sanitize results:");
    println!("   raw lines       : {raw_lines}");
    println!("   blank removed   : {skipped_blank}");
    println!("   comments removed: {skipped_comment}");
    println!("   invalid removed : {skipped_bad}");
    println!("   excluded removed: {skipped_excluded}");
    println!("   before dedup    : {before_dedup}");
    println!("   duplicates      : {skipped_dup}");
    println!("   final unique    : {after_count}");
    if args.dry_run {
        println!("   (dry-run — no file written)");
    } else {
        println!("   output          : {}", output.display());
    }

    Ok(())
}
