//! پاکسازی runs / archives / logs / out با سیاست keep-N.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub type AppResult = Result<(), String>;

#[derive(Debug, Clone)]
pub struct CleanOpts {
    pub runs_keep: Option<usize>,
    pub archives_keep: Option<usize>,
    pub backups_keep: Option<usize>,
    pub logs: bool,
    pub out: bool,
    pub dry_run: bool,
}

pub fn run(work_dir: &Path, opts: &CleanOpts) -> AppResult {
    if let Some(k) = opts.runs_keep {
        prune_dirs(work_dir.join("runs"), k, opts.dry_run, "runs")?;
    }
    if let Some(k) = opts.archives_keep {
        prune_files(
            work_dir.join("archives"),
            k,
            opts.dry_run,
            &["zip", "json"],
            "archives",
        )?;
    }
    if let Some(k) = opts.backups_keep {
        prune_files(
            work_dir.join("backups"),
            k,
            opts.dry_run,
            &["zip", "sha256"],
            "backups",
        )?;
    }
    if opts.logs {
        wipe_dir(work_dir.join("logs"), opts.dry_run, "logs")?;
    }
    if opts.out {
        wipe_dir(work_dir.join("out"), opts.dry_run, "out")?;
    }
    println!("✅ clean done");
    Ok(())
}

fn prune_dirs(dir: std::path::PathBuf, keep: usize, dry: bool, label: &str) -> AppResult {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    entries.reverse();
    for e in entries.into_iter().skip(keep) {
        let p = e.path();
        println!("🗑️  {label}: {}", p.display());
        if !dry {
            let _ = fs::remove_dir_all(&p);
        }
    }
    Ok(())
}

fn prune_files(
    dir: std::path::PathBuf,
    keep: usize,
    dry: bool,
    exts: &[&str],
    label: &str,
) -> AppResult {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| exts.iter().any(|x| s.eq_ignore_ascii_case(x)))
                .unwrap_or(false)
        })
        .collect();
    // keep newest zip pairs roughly by name
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    let zips: Vec<std::fs::DirEntry> = entries
        .into_iter()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        })
        .collect();
    for e in zips.into_iter().skip(keep) {
        let p = e.path();
        println!("🗑️  {label}: {}", p.display());
        if !dry {
            let _ = fs::remove_file(&p);
            let _ = fs::remove_file(p.with_extension("sha256"));
            if let Some(stem) = p.file_stem() {
                let mut man = p.clone();
                man.set_file_name(format!("{}.manifest.json", stem.to_string_lossy()));
                let _ = fs::remove_file(man);
            }
        }
    }
    Ok(())
}

fn wipe_dir(dir: std::path::PathBuf, dry: bool, label: &str) -> AppResult {
    if !dir.is_dir() {
        return Ok(());
    }
    for e in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        println!("🗑️  {label}: {}", p.display());
        if !dry {
            if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            } else {
                let _ = fs::remove_file(&p);
            }
        }
    }
    Ok(())
}
