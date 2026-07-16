//! آرشیو ZIP برای پوشه‌های runs/ با سیاست keep-N.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub type AppResult = Result<(), String>;

pub fn pack(work_dir: &Path, run_id: &str, keep: usize, allow_delete: bool) -> AppResult {
    let run_dir = work_dir.join("runs").join(run_id);
    if !run_dir.is_dir() {
        return Err(format!("run not found: {}", run_dir.display()));
    }
    let archives = work_dir.join("archives");
    fs::create_dir_all(&archives).map_err(|e| e.to_string())?;
    let zip_path = archives.join(format!("{run_id}.zip"));
    zip_dir(&run_dir, &zip_path)?;
    let sha = file_sha256(&zip_path)?;
    let manifest = serde_json::json!({
        "run_id": run_id,
        "zip": zip_path.display().to_string(),
        "sha256": sha,
        "created_at": chrono::Local::now().to_rfc3339(),
    });
    fs::write(
        archives.join(format!("{run_id}.manifest.json")),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    println!("✅ archived {}", zip_path.display());

    prune_old_runs(work_dir, keep, allow_delete)?;
    Ok(())
}

pub fn restore(work_dir: &Path, run_id: &str) -> AppResult {
    let zip_path = work_dir.join("archives").join(format!("{run_id}.zip"));
    if !zip_path.is_file() {
        return Err(format!("archive not found: {}", zip_path.display()));
    }
    let dest = work_dir.join("runs").join(run_id);
    if dest.exists() {
        return Err(format!(
            "run dir already exists: {} (حذف دستی یا نام دیگر)",
            dest.display()
        ));
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
    println!("✅ restored → {}", dest.display());
    Ok(())
}

pub fn list(work_dir: &Path) -> AppResult {
    let runs = work_dir.join("runs");
    if !runs.is_dir() {
        println!("(no runs)");
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(&runs)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        println!("{}", e.file_name().to_string_lossy());
    }
    Ok(())
}

fn prune_old_runs(work_dir: &Path, keep: usize, allow_delete: bool) -> AppResult {
    let runs = work_dir.join("runs");
    if !runs.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(&runs)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();
    if entries.len() <= keep {
        return Ok(());
    }
    let to_drop = entries.len() - keep;
    for p in entries.into_iter().take(to_drop) {
        let id = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let zip = work_dir.join("archives").join(format!("{id}.zip"));
        if zip.is_file() {
            if allow_delete {
                let _ = fs::remove_dir_all(&p);
                println!("🗑️  removed {} (zip ok)", p.display());
            } else {
                println!(
                    "ℹ️  would remove {} (pass --allow-delete-after-archive)",
                    p.display()
                );
            }
        }
    }
    Ok(())
}

fn zip_dir(src: &Path, dst: &Path) -> AppResult {
    let file = File::create(dst).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir(&mut zip, src, src, opts)?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn add_dir(
    zip: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    opts: SimpleFileOptions,
) -> AppResult {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = path
            .strip_prefix(base)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), opts)
                .map_err(|e| e.to_string())?;
            add_dir(zip, base, &path, opts)?;
        } else {
            zip.start_file(&name, opts).map_err(|e| e.to_string())?;
            let mut f = File::open(&path).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            zip.write_all(&buf).map_err(|e| e.to_string())?;
        }
    }
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
