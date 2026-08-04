//! Flat NetMod generator — like `generate_dns.py`:
//! one IP list → one `dns.txt` of `dns://` lines (same `ps` for every row).

use crate::config::Profile;
use crate::resolvers;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct FlatDnsArgs {
    pub input: PathBuf,
    pub out: PathBuf,
    /// Display name (`ps`) — used as-is (no BTF prefix rewrite).
    pub ps: String,
    pub ns: String,
    pub pubkey: String,
    pub user: String,
    pub pass: String,
    pub port: u16,
    pub dedup: bool,
    pub limit: Option<usize>,
}

fn with_port(host: &str, port: u16) -> String {
    let h = host.trim();
    if h.contains(':') {
        h.to_string()
    } else {
        format!("{h}:{port}")
    }
}

fn link(ps: &str, addr: &str, ns: &str, pubkey: &str, user: &str, pass: &str) -> String {
    let obj = json!({
        "ps": ps,
        "addr": addr,
        "ns": ns,
        "pubkey": pubkey,
        "user": user,
        "pass": pass,
    });
    // Compact JSON (no spaces) — same as Python separators=(",", ":")
    let j = serde_json::to_string(&obj).expect("json");
    format!("dns://{}", STANDARD.encode(j.as_bytes()))
}

/// Build from an explicit profile (SSH fields + ns/pubkey).
pub fn from_profile(
    profile: &Profile,
    input: &Path,
    out: &Path,
    ps: Option<&str>,
    port: u16,
    dedup: bool,
    limit: Option<usize>,
) -> Result<usize, String> {
    let ps = ps
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if !profile.remark.trim().is_empty() {
                profile.remark.clone()
            } else {
                profile.profile_name.clone()
            }
        });
    let user = if profile.include_ssh {
        profile.ssh_user.clone()
    } else {
        String::new()
    };
    let pass = if profile.include_ssh {
        profile.ssh_pass.clone()
    } else {
        String::new()
    };
    run(FlatDnsArgs {
        input: input.to_path_buf(),
        out: out.to_path_buf(),
        ps,
        ns: profile.tunnel_domain.clone(),
        pubkey: profile.pubkey.clone(),
        user,
        pass,
        port,
        dedup,
        limit,
    })
}

pub fn run(args: FlatDnsArgs) -> Result<usize, String> {
    if args.ns.trim().is_empty() {
        return Err("--ns (tunnel domain) is required".into());
    }
    if args.pubkey.trim().is_empty() {
        return Err("--pubkey is required".into());
    }
    if args.ps.trim().is_empty() {
        return Err("--ps is required".into());
    }

    let mut ips = resolvers::load_txt_ips(&args.input)?;
    if ips.is_empty() {
        return Err(format!("no IPs in {}", args.input.display()));
    }
    if args.dedup {
        let mut seen = std::collections::HashSet::new();
        ips.retain(|ip| seen.insert(ip.clone()));
    }
    if let Some(n) = args.limit {
        ips.truncate(n);
    }

    let mut lines = Vec::with_capacity(ips.len());
    for ip in &ips {
        let addr = with_port(ip, args.port);
        lines.push(link(
            &args.ps,
            &addr,
            &args.ns,
            &args.pubkey,
            &args.user,
            &args.pass,
        ));
    }

    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(&args.out, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    Ok(lines.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn flat_link_roundtrip() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "1.2.3.4").unwrap();
        writeln!(f, "5.6.7.8").unwrap();
        writeln!(f, "1.2.3.4").unwrap(); // dup
        let out = NamedTempFile::new().unwrap();
        let n = run(FlatDnsArgs {
            input: f.path().to_path_buf(),
            out: out.path().to_path_buf(),
            ps: "NEWJJ".into(),
            ns: "wide.darkmous.ir".into(),
            pubkey: "aabb".into(),
            user: "root".into(),
            pass: "secret".into(),
            port: 53,
            dedup: true,
            limit: None,
        })
        .unwrap();
        assert_eq!(n, 2);
        let text = fs::read_to_string(out.path()).unwrap();
        let lines: Vec<_> = text.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("dns://"));
        let raw = STANDARD.decode(&lines[0]["dns://".len()..]).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(v["ps"], "NEWJJ");
        assert_eq!(v["addr"], "1.2.3.4:53");
        assert_eq!(v["ns"], "wide.darkmous.ir");
        assert_eq!(v["user"], "root");
        assert_eq!(v["pass"], "secret");
    }
}
