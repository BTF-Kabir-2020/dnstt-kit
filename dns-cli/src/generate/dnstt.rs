//! تولید لینک `sn://dnstt` از پروفایل + لیست رزالور.

use super::kryo::{
    build_sn_dnstt_link, normalize_dns_resolvers, serialize_dnstt_bean_v3, DnsttBean,
};
use crate::config::Profile;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct DnsttSummary {
    pub all_link: Option<String>,
    pub per_dns: BTreeMap<String, String>,
}

fn bean_from_profile(profile: &Profile, dns_resolver: &str, name: &str) -> DnsttBean {
    DnsttBean {
        server_address: "127.0.0.1".into(),
        server_port: 53,
        dns_resolver: dns_resolver.to_string(),
        tunnel_domain: profile.tunnel_domain.clone(),
        public_key: profile.pubkey.clone(),
        mode: profile.dnstt_mode,
        socks_username: String::new(),
        socks_password: String::new(),
        ssh_username: profile.ssh_user.clone(),
        ssh_auth_type: if profile.include_ssh { 1 } else { 0 },
        ssh_password: profile.ssh_pass.clone(),
        ssh_port: 22,
        ssh_private_key: String::new(),
        ssh_key_passphrase: String::new(),
        name: name.to_string(),
        custom_outbound_json: String::new(),
        custom_config_json: String::new(),
    }
}

pub fn generate(
    profile: &Profile,
    resolvers: &[String],
    out_dir: &Path,
    mode: &str,
) -> Result<DnsttSummary, String> {
    let mode = mode.to_ascii_lowercase();
    let want_all = mode == "all" || mode == "both";
    let want_each = mode == "each" || mode == "both" || mode == "per";

    let mut summary = DnsttSummary {
        all_link: None,
        per_dns: BTreeMap::new(),
    };

    if want_all {
        let dns_for_bean = normalize_dns_resolvers(resolvers);
        let bean = bean_from_profile(profile, &dns_for_bean, &profile.remark);
        let raw = serialize_dnstt_bean_v3(&bean);
        let link = build_sn_dnstt_link(&raw);
        fs::write(out_dir.join("dnstt_all_dns.txt"), format!("{link}\n"))
            .map_err(|e| e.to_string())?;
        summary.all_link = Some(link);
    }

    if want_each {
        let mut lines = Vec::new();
        for dns in resolvers {
            let dns_norm = normalize_dns_resolvers(std::slice::from_ref(dns));
            let name = format!("{} ({dns})", profile.remark);
            let bean = bean_from_profile(profile, &dns_norm, &name);
            let raw = serialize_dnstt_bean_v3(&bean);
            let link = build_sn_dnstt_link(&raw);
            lines.push(link.clone());
            summary.per_dns.insert(dns.clone(), link);
        }
        fs::write(out_dir.join("dnstt_per_dns.txt"), lines.join("\n") + "\n")
            .map_err(|e| e.to_string())?;
    }

    let json_data = json!({
        "all_dns": summary.all_link.as_ref().map(|l| json!({"ip": "all", "link": l})),
        "per_dns": summary.per_dns.iter().map(|(k,v)| (k.clone(), json!({"ip": k, "link": v}))).collect::<BTreeMap<_,_>>(),
        "generated_at": chrono::Local::now().to_rfc3339(),
        "profile": profile.tunnel_domain,
    });
    fs::write(
        out_dir.join("dnstt_links.json"),
        serde_json::to_string_pretty(&json_data).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;

    Ok(summary)
}

/// خروجی شبیه اسکریپت قدیمی: `DMVPN/<ts>_<remark>/`
pub fn write_dmvpn_bundle(
    work_dir: &Path,
    profile: &Profile,
    summary: &DnsttSummary,
) -> Result<std::path::PathBuf, String> {
    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let safe: String = profile
        .remark
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let folder = work_dir.join("DMVPN").join(format!("{ts}_{safe}"));
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    if let Some(link) = &summary.all_link {
        fs::write(folder.join("dnstt_all_dns.txt"), format!("{link}\n"))
            .map_err(|e| e.to_string())?;
    }
    let mut lines = Vec::new();
    for link in summary.per_dns.values() {
        lines.push(link.clone());
    }
    fs::write(folder.join("dnstt_per_dns.txt"), lines.join("\n") + "\n")
        .map_err(|e| e.to_string())?;
    let json_data = json!({
        "all_dns": summary.all_link.as_ref().map(|l| json!({"ip": "all", "link": l})),
        "per_dns": summary.per_dns.iter().map(|(k,v)| (k.clone(), json!({"ip": k, "link": v}))).collect::<BTreeMap<_,_>>(),
        "generated_at": chrono::Local::now().to_rfc3339(),
        "profile": profile.tunnel_domain,
        "remark": profile.remark,
    });
    fs::write(
        folder.join("dnstt_links.json"),
        serde_json::to_string_pretty(&json_data).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(folder)
}
