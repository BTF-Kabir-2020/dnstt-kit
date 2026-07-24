//! بارگذاری پروفایل‌های DNSTT/NetMod از `config/profiles.json`.
//!
//! هر پروفایل دامنهٔ تونل، pubkey و (اختیاری) SSH را نگه می‌دارد.
//! فایل نمونه: `config/profiles.example.json` — کپی به `profiles.json` و ویرایش کن.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("profiles file not found: {0}")]
    Missing(PathBuf),
    #[error("parse profiles: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown profile: {0}")]
    UnknownProfile(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// دامنهٔ تست TXT / تونل (مثلاً demo.example.com)
    pub tunnel_domain: String,
    /// دامنهٔ پایدار برای پرسش A (پیش‌فرض cloudflare.com)
    #[serde(default = "default_a_domain")]
    pub a_domain: String,
    #[serde(default)]
    pub extra_domains: Vec<String>,
    pub pubkey: String,
    #[serde(default = "default_ps")]
    pub profile_name: String,
    #[serde(default = "default_remark")]
    pub remark: String,
    #[serde(default = "default_ssh_user")]
    pub ssh_user: String,
    #[serde(default)]
    pub ssh_pass: String,
    #[serde(default = "default_true")]
    pub include_ssh: bool,
    /// مود DNSTT: 0 فقط DNSTT، 1 +SOCKS، 2 +SSH
    #[serde(default = "default_mode_ssh")]
    pub dnstt_mode: i32,
}

fn default_a_domain() -> String {
    "cloudflare.com".into()
}
fn default_ps() -> String {
    crate::names::BTF_NAME.into()
}
fn default_remark() -> String {
    format!("{} DNSTT+SSH", crate::names::BTF_NAME)
}
fn default_ssh_user() -> String {
    "root".into()
}
fn default_true() -> bool {
    true
}
fn default_mode_ssh() -> i32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub profiles: HashMap<String, Profile>,
}

impl ProfilesFile {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.is_file() {
            return Err(ConfigError::Missing(path.to_path_buf()));
        }
        let mut text = fs::read_to_string(path)?;
        // PowerShell Set-Content -Encoding utf8 writes a UTF-8 BOM; serde_json rejects it.
        if text.starts_with('\u{feff}') {
            text = text.trim_start_matches('\u{feff}').to_string();
        }
        let mut file: Self = serde_json::from_str(&text)?;
        file.normalize_display_names();
        Ok(file)
    }

    /// Keys: normalize wrong-case person-name letters only (no invented prefix).
    /// remark / profile_name: always carry uppercase [`crate::names::BTF_NAME`].
    pub fn normalize_display_names(&mut self) {
        use crate::names::{ensure_person_name, normalize_display_text, uppercase_person_name};
        let old = std::mem::take(&mut self.profiles);
        let mut next = HashMap::with_capacity(old.len());
        for (key, mut pr) in old {
            // Person name first (may prefix), then DMVPN label casing.
            pr.profile_name = normalize_display_text(&ensure_person_name(&pr.profile_name));
            pr.remark = normalize_display_text(&ensure_person_name(&pr.remark));
            let key = uppercase_person_name(&key);
            next.insert(key, pr);
        }
        self.profiles = next;
    }

    pub fn get(&self, name: &str) -> Result<&Profile, ConfigError> {
        let canon = crate::names::uppercase_person_name(name);
        if let Some(p) = self
            .profiles
            .get(name)
            .or_else(|| self.profiles.get(canon.as_str()))
        {
            return Ok(p);
        }
        let want = canon.to_ascii_lowercase();
        for (k, v) in &self.profiles {
            if k.to_ascii_lowercase() == want {
                return Ok(v);
            }
        }
        Err(ConfigError::UnknownProfile(name.to_string()))
    }
}

/// مسیر پیش‌فرض profiles: `work_dir/config/profiles.json` سپس example.
pub fn resolve_profiles_path(work_dir: &Path) -> PathBuf {
    let primary = work_dir.join("config").join("profiles.json");
    if primary.is_file() {
        return primary;
    }
    work_dir.join("config").join("profiles.example.json")
}

pub fn load_profiles(work_dir: &Path) -> Result<ProfilesFile, ConfigError> {
    ProfilesFile::load(&resolve_profiles_path(work_dir))
}
