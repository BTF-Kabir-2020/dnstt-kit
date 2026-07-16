//! ساخت شناسه و پوشهٔ run زیر `runs/`.

use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub fn new_run_id(prefix: &str) -> String {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    format!("{prefix}_{ts}")
}

pub fn new_run_dir(work_dir: &Path, prefix: &str) -> PathBuf {
    let id = new_run_id(prefix);
    let dir = work_dir.join("runs").join(&id);
    let _ = fs::create_dir_all(&dir);
    dir
}
