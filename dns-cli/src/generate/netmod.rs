//! سازندهٔ لینک NetMod: `dns://` + base64(JSON فشرده).

use crate::config::Profile;
use crate::names::{batch_item_label, ensure_person_name};
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct NetmodSummary {
    pub total: usize,
    pub batch: String,
    pub txt_path: String,
    pub nm_dir: String,
    pub info_path: String,
}

pub fn normalize_resolver(resolver: &str) -> String {
    let r = resolver.trim();
    if r.contains(':') {
        r.to_string()
    } else {
        format!("{r}:53")
    }
}

pub fn netmod_link(profile: &Profile, resolver: &str, display_ps: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let mut obj = json!({
        "ps": ensure_person_name(display_ps),
        "addr": normalize_resolver(resolver),
        "ns": profile.tunnel_domain,
        "pubkey": profile.pubkey,
    });
    if profile.include_ssh {
        obj["user"] = json!(profile.ssh_user);
        obj["pass"] = json!(profile.ssh_pass);
    } else {
        obj["user"] = json!("");
        obj["pass"] = json!("");
    }
    let j = serde_json::to_string(&obj).expect("json");
    let j = serde_json::to_vec(&obj)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or(j);
    format!("dns://{}", STANDARD.encode(j.as_bytes()))
}

pub fn generate(
    profile: &Profile,
    resolvers: &[String],
    out_dir: &Path,
    shuffle: bool,
    batch: &str,
) -> Result<NetmodSummary, String> {
    let mut list = resolvers.to_vec();
    if shuffle {
        use rand::seq::SliceRandom;
        list.shuffle(&mut rand::thread_rng());
    }

    let total = list.len();
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let txt_name = format!("netmod_dns_{total}_{batch}_{ts}.txt");
    let nm_dir_name = format!("netmod_dns_{total}_{batch}_{ts}_nm");
    let info_name = format!("netmod_dns_{total}_{batch}_{ts}_info.json");

    let txt_path = out_dir.join(&txt_name);
    let nm_dir = out_dir.join(&nm_dir_name);
    let info_path = out_dir.join(&info_name);
    fs::create_dir_all(&nm_dir).map_err(|e| e.to_string())?;

    let base_name = if profile.remark.trim().is_empty() {
        profile.profile_name.as_str()
    } else {
        profile.remark.as_str()
    };

    let mut lines = Vec::new();
    let mut configs = Vec::new();
    for (i, r) in list.iter().enumerate() {
        let idx = i + 1;
        let ps = batch_item_label(base_name, batch, idx);
        let link = netmod_link(profile, r, &ps);
        lines.push(link.clone());
        let nm_path = nm_dir.join(format!("{batch}_{idx:03}.nm"));
        fs::write(&nm_path, format!("{}\n", link.trim())).map_err(|e| e.to_string())?;
        configs.push(json!({
            "id": idx,
            "batch": batch,
            "ps": ps,
            "resolver": r,
            "link": link,
        }));
    }

    fs::write(&txt_path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    let info = json!({
        "generated_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "format": "dns:// + base64(json)",
        "batch": batch,
        "ns": profile.tunnel_domain,
        "include_ssh_credentials": profile.include_ssh,
        "total": total,
        "configs": configs,
    });
    fs::write(
        &info_path,
        serde_json::to_string_pretty(&info).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;

    Ok(NetmodSummary {
        total,
        batch: batch.to_string(),
        txt_path: txt_path.display().to_string(),
        nm_dir: nm_dir.display().to_string(),
        info_path: info_path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

    fn sample_profile() -> Profile {
        Profile {
            tunnel_domain: "demo.example.com".into(),
            a_domain: "cloudflare.com".into(),
            extra_domains: vec![],
            pubkey: "eae29baf5db2573e1e2f0769eb91faf8c9f14fd60f7fdfd270b1e304d448d21f".into(),
            profile_name: "dnstt".into(),
            remark: "t".into(),
            ssh_user: "root".into(),
            ssh_pass: "x".into(),
            include_ssh: true,
            dnstt_mode: 2,
        }
    }

    #[test]
    fn link_has_dns_scheme() {
        let link = netmod_link(&sample_profile(), "8.8.8.8", "BTF-test-K7HM-01");
        assert!(link.starts_with("dns://"));
        use base64::Engine;
        let b64 = &link["dns://".len()..];
        let raw = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(v["addr"], "8.8.8.8:53");
        assert_eq!(v["ns"], "demo.example.com");
        assert_eq!(v["ps"], "BTF-test-K7HM-01");
    }
}
