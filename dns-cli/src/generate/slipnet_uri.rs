//! تولید URI کلاینت SlipNet: `slipnet://` + base64(فیلدهای | جدا).
//! سازگار با parser رسمی SlipNet CLI (حداقل ۱۲ فیلد؛ ما تا ۶۲ پد می‌کنیم).

use crate::config::Profile;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::json;
use std::fs;
use std::path::Path;

/// ساخت payload خام (pipe-separated) برای یک resolver (یا چندتایی با ویرگول).
pub fn build_payload(profile: &Profile, resolvers_field: &str, name: &str) -> String {
    let tunnel = if profile.include_ssh && profile.dnstt_mode == 2 {
        "dnstt_ssh"
    } else {
        "dnstt"
    };
    let ssh_en = if profile.include_ssh { "1" } else { "0" };

    // ایندکس‌ها مطابق parseURI در SlipNet CLI
    let mut fields = vec![
        "18".to_string(),              // 0 version
        tunnel.to_string(),            // 1 type
        name.to_string(),              // 2 name
        profile.tunnel_domain.clone(), // 3 domain
        resolvers_field.to_string(),   // 4 resolvers
        "0".into(),                    // 5 auth
        "5000".into(),                 // 6 keepalive
        "bbr".into(),                  // 7 cc
        "1080".into(),                 // 8 socks port
        "127.0.0.1".into(),            // 9 host
        "0".into(),                    // 10 gso
        profile.pubkey.clone(),        // 11 pubkey
        "".into(),                     // 12 socks user
        "".into(),                     // 13 socks pass
        ssh_en.into(),                 // 14 ssh enabled
        profile.ssh_user.clone(),      // 15
        profile.ssh_pass.clone(),      // 16
        "22".into(),                   // 17 ssh port
        "0".into(),                    // 18
        "127.0.0.1".into(),            // 19 ssh host
    ];
    // پد تا حداقل ۶۲ برای فیلدهای جدیدتر CLI
    while fields.len() < 62 {
        fields.push(String::new());
    }
    // چند پیش‌فرض مفید
    if fields.len() > 22 {
        fields[22] = "udp".into(); // DNSTransport
    }
    fields.join("|")
}

pub fn build_uri(profile: &Profile, resolvers_field: &str, name: &str) -> String {
    let raw = build_payload(profile, resolvers_field, name);
    format!("slipnet://{}", STANDARD.encode(raw.as_bytes()))
}

/// نرمال‌سازی resolver برای فیلد SlipNet: اغلب `ip:53:0`
pub fn slipnet_resolver_token(r: &str) -> String {
    let r = r.trim();
    if r.contains(':') {
        // اگر فقط ip:port → ip:port:0
        let parts: Vec<_> = r.split(':').collect();
        if parts.len() == 2 {
            format!("{}:{}:0", parts[0], parts[1])
        } else {
            r.to_string()
        }
    } else {
        format!("{r}:53:0")
    }
}

pub fn generate(profile: &Profile, resolvers: &[String], out_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let all_tokens: Vec<_> = resolvers
        .iter()
        .map(|r| slipnet_resolver_token(r))
        .collect();
    let all_field = all_tokens.join(",");
    let all_uri = build_uri(profile, &all_field, &profile.remark);
    fs::write(out_dir.join("slipnet_all.txt"), format!("{all_uri}\n"))
        .map_err(|e| e.to_string())?;

    let mut per = Vec::new();
    let mut map = serde_json::Map::new();
    for r in resolvers {
        let tok = slipnet_resolver_token(r);
        let uri = build_uri(profile, &tok, &format!("{} ({r})", profile.remark));
        per.push(uri.clone());
        map.insert(r.clone(), json!({"resolver": r, "link": uri}));
    }
    fs::write(out_dir.join("slipnet_per.txt"), per.join("\n") + "\n").map_err(|e| e.to_string())?;
    let info = json!({
        "format": "slipnet:// + base64(pipe fields)",
        "tunnel_domain": profile.tunnel_domain,
        "total": resolvers.len(),
        "all": all_uri,
        "per": map,
        "generated_at": chrono::Local::now().to_rfc3339(),
    });
    fs::write(
        out_dir.join("slipnet_links.json"),
        serde_json::to_string_pretty(&info).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(resolvers.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;

    fn p() -> Profile {
        Profile {
            tunnel_domain: "example.com".into(),
            a_domain: "cloudflare.com".into(),
            extra_domains: vec![],
            pubkey: "aabb".into(),
            profile_name: "t".into(),
            remark: "test".into(),
            ssh_user: "root".into(),
            ssh_pass: "x".into(),
            include_ssh: true,
            dnstt_mode: 2,
        }
    }

    #[test]
    fn uri_scheme_and_decodable() {
        let uri = build_uri(&p(), "8.8.8.8:53:0", "test");
        assert!(uri.starts_with("slipnet://"));
        let b64 = &uri["slipnet://".len()..];
        let raw = STANDARD.decode(b64).unwrap();
        let s = String::from_utf8(raw).unwrap();
        let fields: Vec<_> = s.split('|').collect();
        assert!(fields.len() >= 12);
        assert_eq!(fields[1], "dnstt_ssh");
        assert_eq!(fields[3], "example.com");
        assert_eq!(fields[11], "aabb");
    }
}
