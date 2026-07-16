//! بکاپ دستی و چرخشی از سورس kit، داده، و حالت full.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub type AppResult = Result<(), String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    /// سورس + docs + scripts + testdata + config نمونه (بدون target/runs)
    Kit,
    /// sqlite + archives (+ runs اگر include_runs)
    Data,
    /// kit + data
    Full,
}

impl BackupMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "kit" => Ok(Self::Kit),
            "data" => Ok(Self::Data),
            "full" => Ok(Self::Full),
            other => Err(format!("unknown backup mode `{other}` (kit|data|full)")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kit => "kit",
            Self::Data => "data",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupOpts {
    pub mode: BackupMode,
    pub keep: usize,
    pub include_secrets: bool,
    pub include_runs: bool,
    pub include_vendor: bool,
    pub label: Option<String>,
}

pub fn backups_dir(work_dir: &Path) -> PathBuf {
    work_dir.join("backups")
}

pub fn create(work_dir: &Path, opts: &BackupOpts) -> AppResult {
    let dir = backups_dir(work_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let label = opts
        .label
        .as_deref()
        .map(|s| format!("_{}", sanitize(s)))
        .unwrap_or_default();
    let name = format!("dnstt_{}_{}{}.zip", opts.mode.as_str(), ts, label);
    let zip_path = dir.join(&name);

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut manifest_paths: Vec<String> = Vec::new();

    match opts.mode {
        BackupMode::Kit => {
            add_kit(work_dir, &mut zip, options, opts, &mut manifest_paths)?;
        }
        BackupMode::Data => {
            add_data(work_dir, &mut zip, options, opts, &mut manifest_paths)?;
        }
        BackupMode::Full => {
            add_kit(work_dir, &mut zip, options, opts, &mut manifest_paths)?;
            add_data(work_dir, &mut zip, options, opts, &mut manifest_paths)?;
        }
    }

    let meta = serde_json::json!({
        "mode": opts.mode.as_str(),
        "created_at": chrono::Local::now().to_rfc3339(),
        "work_dir": work_dir.display().to_string(),
        "include_secrets": opts.include_secrets,
        "include_runs": opts.include_runs,
        "include_vendor": opts.include_vendor,
        "files": manifest_paths.len(),
    });
    zip.start_file("_backup_manifest.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(serde_json::to_string_pretty(&meta).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    let sha = file_sha256(&zip_path)?;
    let side = zip_path.with_extension("sha256");
    fs::write(&side, format!("{sha}  {name}\n")).map_err(|e| e.to_string())?;
    println!("✅ backup → {}", zip_path.display());
    println!("   sha256={sha} files≈{}", manifest_paths.len());

    prune(work_dir, opts.keep)?;
    Ok(())
}

pub fn list(work_dir: &Path) -> AppResult {
    let dir = backups_dir(work_dir);
    if !dir.is_dir() {
        println!("(no backups/)");
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let meta = e.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        println!("{}\t{} bytes", e.file_name().to_string_lossy(), size);
    }
    Ok(())
}

pub fn restore(work_dir: &Path, zip_name: &str, force: bool) -> AppResult {
    let zip_path = {
        let p = PathBuf::from(zip_name);
        if p.is_file() {
            p
        } else {
            backups_dir(work_dir).join(zip_name)
        }
    };
    if !zip_path.is_file() {
        return Err(format!("backup not found: {}", zip_path.display()));
    }
    let dest = work_dir.join("_restore_tmp");
    if dest.exists() {
        if force {
            fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            return Err("_restore_tmp exists — use --force".into());
        }
    }
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let file = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file
            .enclosed_name()
            .ok_or_else(|| "bad zip entry".to_string())?
            .to_path_buf();
        let outpath = dest.join(&name);
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    println!(
        "✅ extracted to {} — فایل‌ها را دستی به مسیر دلخواه کپی کن",
        dest.display()
    );
    println!("   (برای جلوگیری از overwrite ناخواسته، merge خودکار انجام نمی‌شود)");
    Ok(())
}

pub fn prune(work_dir: &Path, keep: usize) -> AppResult {
    let dir = backups_dir(work_dir);
    if !dir.is_dir() || keep == 0 {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("zip"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for e in entries.into_iter().skip(keep) {
        let p = e.path();
        let _ = fs::remove_file(&p);
        let _ = fs::remove_file(p.with_extension("sha256"));
        println!("🗑️  pruned {}", e.file_name().to_string_lossy());
    }
    Ok(())
}

/// بکاپ دوره‌ای در حلقه (Ctrl+C برای توقف)
pub fn watch(work_dir: &Path, opts: &BackupOpts, interval_secs: u64) -> AppResult {
    if interval_secs < 60 {
        return Err("interval must be >= 60 seconds".into());
    }
    println!(
        "⏱️  backup watch every {interval_secs}s mode={} keep={} (Ctrl+C to stop)",
        opts.mode.as_str(),
        opts.keep
    );
    loop {
        if let Err(e) = create(work_dir, opts) {
            eprintln!("⚠️  backup failed: {e}");
        }
        std::thread::sleep(Duration::from_secs(interval_secs));
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect()
}

fn add_kit(
    work_dir: &Path,
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    opts: &BackupOpts,
    manifest: &mut Vec<String>,
) -> AppResult {
    let roots = [
        "Cargo.toml",
        "Cargo.lock",
        "README.md",
        "run.py",
        "dns-cli",
        "scanner-core",
        "docs",
        "scripts",
        "testdata",
        "config",
        ".github",
        "vendor/slipnet/README.md",
    ];
    for r in roots {
        let p = work_dir.join(r);
        if p.is_file() {
            add_file(work_dir, &p, zip, options, manifest)?;
        } else if p.is_dir() {
            add_dir_filtered(work_dir, &p, zip, options, opts, manifest)?;
        }
    }
    Ok(())
}

fn add_data(
    work_dir: &Path,
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    opts: &BackupOpts,
    manifest: &mut Vec<String>,
) -> AppResult {
    for name in ["data", "archives"] {
        let p = work_dir.join(name);
        if p.is_dir() {
            add_dir_filtered(work_dir, &p, zip, options, opts, manifest)?;
        }
    }
    if opts.include_runs {
        let p = work_dir.join("runs");
        if p.is_dir() {
            add_dir_filtered(work_dir, &p, zip, options, opts, manifest)?;
        }
    }
    if opts.include_vendor {
        let p = work_dir.join("vendor");
        if p.is_dir() {
            add_dir_filtered(work_dir, &p, zip, options, opts, manifest)?;
        }
    }
    Ok(())
}

fn add_dir_filtered(
    work_dir: &Path,
    dir: &Path,
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    opts: &BackupOpts,
    manifest: &mut Vec<String>,
) -> AppResult {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(cur) = stack.pop() {
        let rd = match fs::read_dir(&cur) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if should_skip(&name, &p, opts) {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                add_file(work_dir, &p, zip, options, manifest)?;
            }
        }
    }
    Ok(())
}

fn should_skip(name: &str, path: &Path, opts: &BackupOpts) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "target" | ".git" | "node_modules" | "__pycache__" | ".cargo"
    ) {
        return true;
    }
    if lower.ends_with(".pdb") || lower.ends_with(".o") || lower.ends_with(".d") {
        return true;
    }
    if !opts.include_secrets && (lower == "profiles.json" || lower.ends_with(".env")) {
        return true;
    }
    if !opts.include_vendor && (lower == "slipnet" || lower == "slipnet.exe") {
        return true;
    }
    // skip huge accidental dumps
    if let Ok(meta) = path.metadata() {
        if meta.len() > 80 * 1024 * 1024 {
            return true;
        }
    }
    false
}

fn add_file(
    work_dir: &Path,
    path: &Path,
    zip: &mut ZipWriter<File>,
    options: SimpleFileOptions,
    manifest: &mut Vec<String>,
) -> AppResult {
    let rel = path
        .strip_prefix(work_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let mut f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    zip.start_file(&rel, options).map_err(|e| e.to_string())?;
    zip.write_all(&buf).map_err(|e| e.to_string())?;
    manifest.push(rel);
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
