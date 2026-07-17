//! presetهای منابع: `low` برای تک‌هسته / ~512MB (استریم واقعی)، `normal` برای ماشین معمولی.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetName {
    Low,
    Normal,
    Fast,
}

impl PresetName {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "normal" | "default" => Some(Self::Normal),
            "fast" => Some(Self::Fast),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPreset {
    pub workers: usize,
    pub timeout: f64,
    pub no_ping: bool,
    pub udp_attempts: u32,
    pub udp_backoff_ms: u64,
    /// اگر true از run_scan_stream + نوشتن افزایشی دیسک استفاده شود (کم‌رم / لیست بزرگ)
    pub stream: bool,
}

impl ScanPreset {
    pub fn named(name: PresetName) -> Self {
        match name {
            // چند worker هم‌زمان با سقف in-flight؛ RAM ≈ O(workers) نه O(خطوط فایل)
            PresetName::Low => Self {
                workers: 16,
                timeout: 5.0,
                no_ping: true,
                udp_attempts: 2,
                udp_backoff_ms: 200,
                stream: true,
            },
            PresetName::Normal => Self {
                workers: 64,
                timeout: 5.0,
                no_ping: false,
                udp_attempts: 2,
                udp_backoff_ms: 150,
                stream: false,
            },
            PresetName::Fast => Self {
                workers: 128,
                timeout: 2.0,
                no_ping: true,
                udp_attempts: 1,
                udp_backoff_ms: 100,
                stream: true,
            },
        }
    }
}
