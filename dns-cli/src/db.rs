//! پایگاه SQLite برای ثبت runها، خلاصهٔ اسکن و خروجی‌های generate.
//! مسیر پیش‌فرض: `data/dnstt_kit.sqlite`

use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};

pub fn db_path(work_dir: &Path) -> PathBuf {
    work_dir.join("data").join("dnstt_kit.sqlite")
}

pub fn open(work_dir: &Path) -> Result<Connection, String> {
    let path = db_path(work_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            profile TEXT,
            preset TEXT,
            status TEXT NOT NULL,
            e2e_ok INTEGER DEFAULT 0,
            working_count INTEGER DEFAULT 0,
            notes TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            path TEXT NOT NULL,
            meta_json TEXT,
            FOREIGN KEY(run_id) REFERENCES runs(id)
        );
        CREATE INDEX IF NOT EXISTS idx_artifacts_run ON artifacts(run_id);
        "#,
    )
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_run(
    work_dir: &Path,
    id: &str,
    kind: &str,
    profile: Option<&str>,
    preset: Option<&str>,
    status: &str,
    e2e_ok: bool,
    working_count: i64,
    notes: &str,
) -> Result<(), String> {
    let conn = open(work_dir)?;
    conn.execute(
        "INSERT OR REPLACE INTO runs (id, kind, profile, preset, status, e2e_ok, working_count, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            kind,
            profile,
            preset,
            status,
            if e2e_ok { 1 } else { 0 },
            working_count,
            notes,
            chrono::Local::now().to_rfc3339(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn add_artifact(
    work_dir: &Path,
    run_id: &str,
    kind: &str,
    path: &str,
    meta_json: Option<&str>,
) -> Result<(), String> {
    let conn = open(work_dir)?;
    conn.execute(
        "INSERT INTO artifacts (run_id, kind, path, meta_json) VALUES (?1, ?2, ?3, ?4)",
        params![run_id, kind, path, meta_json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: String,
    pub kind: String,
    pub profile: String,
    pub preset: String,
    pub status: String,
    pub e2e_ok: bool,
    pub working_count: i64,
    pub notes: String,
    pub created_at: String,
}

pub fn list_runs(work_dir: &Path, limit: usize) -> Result<Vec<RunRow>, String> {
    let conn = open(work_dir)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, IFNULL(profile,''), IFNULL(preset,''), status, e2e_ok, working_count,
                    IFNULL(notes,''), created_at
             FROM runs ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit as i64], |row| {
            Ok(RunRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                profile: row.get(2)?,
                preset: row.get(3)?,
                status: row.get(4)?,
                e2e_ok: row.get::<_, i64>(5)? != 0,
                working_count: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn stats(work_dir: &Path) -> Result<(i64, i64, i64), String> {
    let conn = open(work_dir)?;
    let runs: i64 = conn
        .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0))
        .unwrap_or(0);
    let ok: i64 = conn
        .query_row("SELECT COUNT(*) FROM runs WHERE status='ok'", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    let arts: i64 = conn
        .query_row("SELECT COUNT(*) FROM artifacts", [], |r| r.get(0))
        .unwrap_or(0);
    Ok((runs, ok, arts))
}
