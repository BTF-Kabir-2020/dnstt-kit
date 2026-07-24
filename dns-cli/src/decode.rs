//! Decode client URIs (`dns://`, `slipnet://`, `sn://dnstt?`) into profile fields.
//!
//! Secrets are masked on stdout. Use `--save-profile` to write `config/profiles.json`
//! (gitignored) for local scan/generate.

use crate::config::{self, Profile, ProfilesFile};
use crate::generate::kryo;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use flate2::read::ZlibDecoder;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::path::Path;

pub type AppResult = Result<(), String>;

#[derive(Debug, Clone)]
pub struct DecodedLink {
    pub kind: &'static str,
    pub remark: String,
    pub tunnel_domain: String,
    pub pubkey: String,
    pub resolver: String,
    pub ssh_user: String,
    pub ssh_pass: String,
    pub include_ssh: bool,
    pub dnstt_mode: i32,
    pub tunnel_type: String,
}

impl DecodedLink {
    pub fn to_profile(&self) -> Profile {
        Profile {
            tunnel_domain: self.tunnel_domain.clone(),
            a_domain: "cloudflare.com".into(),
            extra_domains: vec![self.tunnel_domain.clone()],
            pubkey: self.pubkey.clone(),
            profile_name: if self.remark.is_empty() {
                "dnstt".into()
            } else {
                self.remark.clone()
            },
            remark: if self.remark.is_empty() {
                "imported".into()
            } else {
                self.remark.clone()
            },
            ssh_user: if self.ssh_user.is_empty() {
                "root".into()
            } else {
                self.ssh_user.clone()
            },
            ssh_pass: self.ssh_pass.clone(),
            include_ssh: self.include_ssh,
            dnstt_mode: self.dnstt_mode,
        }
    }
}

pub fn decode_uri(uri: &str) -> Result<DecodedLink, String> {
    let uri = uri.trim();
    if let Some(rest) = uri.strip_prefix("dns://") {
        return decode_netmod(rest);
    }
    if let Some(rest) = uri.strip_prefix("slipnet://") {
        return decode_slipnet(rest);
    }
    if let Some(rest) = uri.strip_prefix("sn://dnstt?") {
        return decode_nekobox(rest);
    }
    Err("unknown scheme (expected dns:// | slipnet:// | sn://dnstt?)".into())
}

fn decode_netmod(b64: &str) -> Result<DecodedLink, String> {
    let raw = STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("netmod base64: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|e| format!("netmod json: {e}"))?;
    let ns = v
        .get("ns")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if ns.is_empty() {
        return Err("netmod missing ns".into());
    }
    let addr = v
        .get("addr")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let user = v
        .get("user")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let pass = v
        .get("pass")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let include_ssh = !user.is_empty() || !pass.is_empty();
    Ok(DecodedLink {
        kind: "netmod/dns://",
        remark: v
            .get("ps")
            .and_then(|x| x.as_str())
            .unwrap_or("imported")
            .to_string(),
        tunnel_domain: ns,
        pubkey: v
            .get("pubkey")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        resolver: addr,
        ssh_user: user,
        ssh_pass: pass,
        include_ssh,
        dnstt_mode: if include_ssh { 2 } else { 0 },
        tunnel_type: if include_ssh {
            "dnstt_ssh".into()
        } else {
            "dnstt".into()
        },
    })
}

fn decode_slipnet(b64: &str) -> Result<DecodedLink, String> {
    let raw = STANDARD
        .decode(b64.trim())
        .or_else(|_| {
            let mut s = b64.trim().to_string();
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
    let tunnel = fields[1].to_string();
    let include_ssh = fields.get(14).map(|x| *x == "1").unwrap_or(false)
        || tunnel.contains("ssh");
    Ok(DecodedLink {
        kind: "slipnet://",
        remark: fields[2].to_string(),
        tunnel_domain: fields[3].to_string(),
        pubkey: fields[11].to_string(),
        resolver: fields[4].to_string(),
        ssh_user: fields.get(15).unwrap_or(&"").to_string(),
        ssh_pass: fields.get(16).unwrap_or(&"").to_string(),
        include_ssh,
        dnstt_mode: if include_ssh { 2 } else { 0 },
        tunnel_type: tunnel,
    })
}

fn decode_nekobox(rest: &str) -> Result<DecodedLink, String> {
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
    let ver = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    if ver != 3 {
        return Err(format!("dnstt unexpected version {ver}"));
    }
    let _ = kryo::b64_url_safe_no_pad;
    // Full Kryo field walk is heavy; link is valid but use dns:// / slipnet:// for --save-profile.
    Ok(DecodedLink {
        kind: "nekobox/sn://dnstt",
        remark: "nekobox".into(),
        tunnel_domain: String::new(),
        pubkey: String::new(),
        resolver: String::new(),
        ssh_user: String::new(),
        ssh_pass: String::new(),
        include_ssh: false,
        dnstt_mode: 0,
        tunnel_type: "dnstt".into(),
    })
}

fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return "(empty)".into();
    }
    if s.len() <= 4 {
        return "****".into();
    }
    format!("{}…****", &s[..2.min(s.len())])
}

pub fn run(
    work_dir: &Path,
    uri: Option<String>,
    file: Option<std::path::PathBuf>,
    save_profile: Option<String>,
    show_secrets: bool,
    json_out: bool,
) -> AppResult {
    let uri = if let Some(u) = uri {
        u
    } else if let Some(p) = file {
        let path = if p.is_absolute() {
            p
        } else {
            work_dir.join(p)
        };
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        text.lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .ok_or_else(|| "no URI line in file".to_string())?
            .to_string()
    } else {
        return Err("pass a URI or --file".into());
    };

    let decoded = decode_uri(&uri)?;
    if json_out {
        let obj = json!({
            "kind": decoded.kind,
            "remark": decoded.remark,
            "ns": decoded.tunnel_domain,
            "pubkey": decoded.pubkey,
            "addr": decoded.resolver,
            "user": decoded.ssh_user,
            "pass": if show_secrets { decoded.ssh_pass.clone() } else { mask_secret(&decoded.ssh_pass) },
            "include_ssh": decoded.include_ssh,
            "tunnel_type": decoded.tunnel_type,
        });
        println!("{}", serde_json::to_string_pretty(&obj).map_err(|e| e.to_string())?);
    } else {
        println!("kind:          {}", decoded.kind);
        println!("remark:        {}", decoded.remark);
        println!("nameserver:    {}", decoded.tunnel_domain);
        println!("public_key:    {}", decoded.pubkey);
        println!("dns_address:   {}", decoded.resolver);
        println!("tunnel_type:   {}", decoded.tunnel_type);
        println!("ssh_user:      {}", decoded.ssh_user);
        println!(
            "ssh_pass:      {}",
            if show_secrets {
                decoded.ssh_pass.clone()
            } else {
                mask_secret(&decoded.ssh_pass)
            }
        );
        println!();
        println!("Next (scan for working UDP resolvers against this tunnel domain):");
        println!(
            "  dns-cli scan local/lists/tokhmi.txt --preset low --domain {} --a-domain cloudflare.com --quiet",
            decoded.tunnel_domain
        );
        println!("  # or smaller sample:");
        println!(
            "  dns-cli scan testdata/dns_sample.txt --preset low --domain {} --a-domain cloudflare.com --limit 20",
            decoded.tunnel_domain
        );
        println!();
        println!("Then generate client links for OK resolvers:");
        println!("  dns-cli resolvers sync --from-txt <ok_list.txt>");
        if save_profile.is_none() {
            println!("  dns-cli decode \"…\" --save-profile mytunnel");
            println!("  dns-cli generate all --profile mytunnel --resolvers resolvers.json --limit 50");
        } else {
            println!(
                "  dns-cli generate all --profile {} --resolvers resolvers.json --limit 50",
                save_profile.as_ref().unwrap()
            );
        }
        println!();
        println!("Phone test: import the same dns:// / slipnet:// in NetMod or SlipNet and Connect.");
        println!("This kit does not dial the tunnel itself — it finds resolvers + builds configs.");
    }

    if let Some(name) = save_profile {
        if decoded.tunnel_domain.is_empty() || decoded.pubkey.is_empty() {
            return Err(
                "--save-profile needs dns:// or slipnet:// (sn://dnstt is verify-only here)".into(),
            );
        }
        save_decoded_profile(work_dir, &name, &decoded)?;
        println!("✅ saved profile `{name}` → config/profiles.json (local / gitignored)");
    }
    Ok(())
}

fn save_decoded_profile(work_dir: &Path, name: &str, decoded: &DecodedLink) -> AppResult {
    let path = work_dir.join("config").join("profiles.json");
    let mut file = if path.is_file() {
        ProfilesFile::load(&path).map_err(|e| e.to_string())?
    } else {
        let example = work_dir.join("config").join("profiles.example.json");
        if example.is_file() {
            ProfilesFile::load(&example).map_err(|e| e.to_string())?
        } else {
            ProfilesFile {
                profiles: Default::default(),
            }
        }
    };
    file.profiles.insert(name.to_string(), decoded.to_profile());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(&path, text + "\n").map_err(|e| e.to_string())?;
    let _ = config::load_profiles(work_dir);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::netmod;
    use crate::generate::slipnet_uri;

    fn sample() -> Profile {
        Profile {
            tunnel_domain: "demo.example.com".into(),
            a_domain: "cloudflare.com".into(),
            extra_domains: vec![],
            pubkey: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899".into(),
            profile_name: "test dnstt 1".into(),
            remark: "test dnstt 1".into(),
            ssh_user: "root".into(),
            ssh_pass: "secret".into(),
            include_ssh: true,
            dnstt_mode: 2,
        }
    }

    #[test]
    fn decode_roundtrip_netmod() {
        let link = netmod::netmod_link(&sample(), "1.1.1.1");
        let d = decode_uri(&link).unwrap();
        assert_eq!(d.tunnel_domain, "demo.example.com");
        assert_eq!(d.resolver, "1.1.1.1:53");
        assert_eq!(d.ssh_user, "root");
        assert_eq!(d.ssh_pass, "secret");
    }

    #[test]
    fn decode_roundtrip_slipnet() {
        let link = slipnet_uri::build_uri(&sample(), "1.1.1.1:53:0", "r");
        let d = decode_uri(&link).unwrap();
        assert_eq!(d.tunnel_domain, "demo.example.com");
        assert!(d.include_ssh);
    }
}
