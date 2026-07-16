//! اعتبارسنجی خروجی لینک‌ها (decode بدون نیاز به شبکه).

use crate::generate::kryo;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use flate2::read::ZlibDecoder;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path, path: PathBuf) -> AppResult {
    let path = if path.is_absolute() {
        path
    } else {
        work_dir.join(path)
    };
    if !path.is_file() {
        return Err(format!("file not found: {}", path.display()));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut checked = 0usize;
    let mut bad = 0usize;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        checked += 1;
        match verify_one(line) {
            Ok(kind) => println!("✅ L{} {kind}", i + 1),
            Err(e) => {
                bad += 1;
                println!("❌ L{} {e}", i + 1);
            }
        }
    }
    println!("verify: checked={checked} bad={bad}");
    if bad > 0 {
        Err(format!("{bad} invalid link(s)"))
    } else if checked == 0 {
        Err("no links found in file".into())
    } else {
        Ok(())
    }
}

fn verify_one(line: &str) -> Result<&'static str, String> {
    if let Some(rest) = line.strip_prefix("dns://") {
        let raw = STANDARD
            .decode(rest.trim())
            .map_err(|e| format!("netmod base64: {e}"))?;
        let v: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|e| format!("netmod json: {e}"))?;
        if v.get("ns")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .is_empty()
        {
            return Err("netmod missing ns".into());
        }
        if v.get("addr")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .is_empty()
        {
            return Err("netmod missing addr".into());
        }
        return Ok("netmod/dns://");
    }
    if let Some(rest) = line.strip_prefix("sn://dnstt?") {
        let b64 = rest.split('#').next().unwrap_or(rest).trim();
        let compressed = URL_SAFE_NO_PAD
            .decode(b64)
            .or_else(|_| {
                let mut s = b64.to_string();
                while s.len() % 4 != 0 {
                    s.push('=');
                }
                URL_SAFE_NO_PAD.decode(s)
            })
            .map_err(|e| format!("dnstt b64: {e}"))?;
        let mut dec = ZlibDecoder::new(&compressed[..]);
        let mut raw = Vec::new();
        dec.read_to_end(&mut raw)
            .map_err(|e| format!("dnstt zlib: {e}"))?;
        if raw.len() < 8 {
            return Err("dnstt payload too short".into());
        }
        // version int LE = 3
        let ver = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        if ver != 3 {
            return Err(format!("dnstt unexpected version {ver}"));
        }
        let _ = kryo::b64_url_safe_no_pad; // keep module linked
        return Ok("nekobox/sn://dnstt");
    }
    if let Some(rest) = line.strip_prefix("slipnet://") {
        let raw = STANDARD
            .decode(rest.trim())
            .or_else(|_| {
                let mut s = rest.trim().to_string();
                while s.len() % 4 != 0 {
                    s.push('=');
                }
                STANDARD.decode(s)
            })
            .map_err(|e| format!("slipnet b64: {e}"))?;
        let s = String::from_utf8(raw).map_err(|e| format!("slipnet utf8: {e}"))?;
        let fields: Vec<_> = s.split('|').collect();
        if fields.len() < 12 {
            return Err(format!("slipnet fields={} need>=12", fields.len()));
        }
        if fields[3].is_empty() || fields[11].is_empty() {
            return Err("slipnet missing domain/pubkey".into());
        }
        return Ok("slipnet://");
    }
    Err("unknown scheme (expected dns:// | sn://dnstt? | slipnet://)".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use crate::generate::netmod;
    use crate::generate::slipnet_uri;

    fn profile() -> Profile {
        Profile {
            tunnel_domain: "example.com".into(),
            a_domain: "cloudflare.com".into(),
            extra_domains: vec![],
            pubkey: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899".into(),
            profile_name: "t".into(),
            remark: "r".into(),
            ssh_user: "root".into(),
            ssh_pass: "x".into(),
            include_ssh: true,
            dnstt_mode: 2,
        }
    }

    #[test]
    fn verify_netmod_and_slipnet() {
        let p = profile();
        let nm = netmod::netmod_link(&p, "1.1.1.1");
        assert!(verify_one(&nm).is_ok());
        let sl = slipnet_uri::build_uri(&p, "1.1.1.1:53:0", "r");
        assert!(verify_one(&sl).is_ok());
    }
}
