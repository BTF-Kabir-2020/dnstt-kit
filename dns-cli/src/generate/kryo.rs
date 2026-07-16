//! سریال‌سازی Kryo 5.x (زیرمجموعهٔ لازم برای DNSTTBean v3).
//! باید با `dnstt_ssh_share_link.py` بایت‌به‌بایت یکی باشد.

use std::io::Write;

/// بافر خروجی شبیه KryoOutput در پایتون.
pub struct KryoOutput {
    buf: Vec<u8>,
}

impl KryoOutput {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    #[allow(dead_code)]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// int ثابت ۴ بایت little-endian.
    pub fn write_int(&mut self, value: i32) {
        let v = value as u32;
        self.buf.extend_from_slice(&[
            (v & 0xff) as u8,
            ((v >> 8) & 0xff) as u8,
            ((v >> 16) & 0xff) as u8,
            ((v >> 24) & 0xff) as u8,
        ]);
    }

    /// varint با فلگ (optimize_positive برای string همیشه true).
    pub fn write_var_int_flag(&mut self, flag: bool, mut value: u32, optimize_positive: bool) {
        if !optimize_positive {
            // zigzag
            let signed = value as i32;
            value = ((signed << 1) ^ (signed >> 31)) as u32;
        }
        let mut first = (value & 0x3f) | if flag { 0x80 } else { 0 };
        if value >> 6 == 0 {
            self.buf.push(first as u8);
            return;
        }
        if value >> 13 == 0 {
            first |= 0x40;
            self.buf
                .extend_from_slice(&[first as u8, ((value >> 6) & 0xff) as u8]);
            return;
        }
        if value >> 20 == 0 {
            first |= 0x40;
            self.buf.extend_from_slice(&[
                first as u8,
                (((value >> 6) | 0x80) & 0xff) as u8,
                ((value >> 13) & 0xff) as u8,
            ]);
            return;
        }
        if value >> 27 == 0 {
            first |= 0x40;
            self.buf.extend_from_slice(&[
                first as u8,
                (((value >> 6) | 0x80) & 0xff) as u8,
                (((value >> 13) | 0x80) & 0xff) as u8,
                ((value >> 20) & 0xff) as u8,
            ]);
            return;
        }
        first |= 0x40;
        self.buf.extend_from_slice(&[
            first as u8,
            (((value >> 6) | 0x80) & 0xff) as u8,
            (((value >> 13) | 0x80) & 0xff) as u8,
            (((value >> 20) | 0x80) & 0xff) as u8,
            ((value >> 27) & 0xff) as u8,
        ]);
    }

    pub fn write_string(&mut self, value: Option<&str>) {
        let Some(value) = value else {
            self.buf.push(0x80);
            return;
        };
        let char_count = value.chars().count();
        if char_count == 0 {
            self.buf.push(0x81); // 1 | 0x80
            return;
        }

        // کوتاه ASCII: بایت‌ها + بیت بالای آخرین بایت
        if (2..=32).contains(&char_count) && value.is_ascii() {
            let mut raw = value.as_bytes().to_vec();
            if let Some(last) = raw.last_mut() {
                *last |= 0x80;
            }
            self.buf.extend_from_slice(&raw);
            return;
        }

        self.write_var_int_flag(true, (char_count + 1) as u32, true);
        let chars: Vec<char> = value.chars().collect();
        let mut char_index = 0;
        while char_index < char_count {
            let c = chars[char_index] as u32;
            if c > 127 {
                break;
            }
            self.buf.push(c as u8);
            char_index += 1;
        }
        while char_index < char_count {
            let c = chars[char_index] as u32;
            if c <= 0x007f {
                self.buf.push(c as u8);
            } else if c > 0x07ff {
                self.buf.extend_from_slice(&[
                    (0xe0 | ((c >> 12) & 0x0f)) as u8,
                    (0x80 | ((c >> 6) & 0x3f)) as u8,
                    (0x80 | (c & 0x3f)) as u8,
                ]);
            } else {
                self.buf.extend_from_slice(&[
                    (0xc0 | ((c >> 6) & 0x1f)) as u8,
                    (0x80 | (c & 0x3f)) as u8,
                ]);
            }
            char_index += 1;
        }
    }
}

impl Default for KryoOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// پارامترهای DNSTTBean v3 مطابق اپ NekoBox.
#[derive(Debug, Clone)]
pub struct DnsttBean {
    pub server_address: String,
    pub server_port: i32,
    pub dns_resolver: String,
    pub tunnel_domain: String,
    pub public_key: String,
    pub mode: i32,
    pub socks_username: String,
    pub socks_password: String,
    pub ssh_username: String,
    pub ssh_auth_type: i32,
    pub ssh_password: String,
    pub ssh_port: i32,
    pub ssh_private_key: String,
    pub ssh_key_passphrase: String,
    pub name: String,
    pub custom_outbound_json: String,
    pub custom_config_json: String,
}

pub fn serialize_dnstt_bean_v3(bean: &DnsttBean) -> Vec<u8> {
    let mut out = KryoOutput::new();
    out.write_int(3);
    out.write_string(Some(&bean.server_address));
    out.write_int(bean.server_port);
    out.write_string(Some(&bean.dns_resolver));
    out.write_string(Some(&bean.tunnel_domain));
    out.write_string(Some(&bean.public_key));
    out.write_int(bean.mode);
    out.write_string(Some(&bean.socks_username));
    out.write_string(Some(&bean.socks_password));
    out.write_string(Some(&bean.ssh_username));
    out.write_int(bean.ssh_auth_type);
    out.write_string(Some(&bean.ssh_password));
    out.write_int(bean.ssh_port);
    out.write_string(Some(&bean.ssh_private_key));
    out.write_string(Some(&bean.ssh_key_passphrase));
    out.write_int(1);
    out.write_string(Some(&bean.name));
    out.write_string(Some(&bean.custom_outbound_json));
    out.write_string(Some(&bean.custom_config_json));
    out.into_bytes()
}

pub fn normalize_dns_resolvers(tokens: &[String]) -> String {
    let mut out = Vec::new();
    for t in tokens {
        for part in t.replace("\r\n", "\n").split('\n') {
            for p in part.split(',') {
                let p = p.trim();
                if p.is_empty() {
                    continue;
                }
                let n = if p.eq_ignore_ascii_case("local") {
                    "127.0.0.1:53".to_string()
                } else if p.starts_with("https://") || p.starts_with("tls://") || p.contains(':') {
                    p.to_string()
                } else {
                    format!("{p}:53")
                };
                out.push(n);
            }
        }
    }
    if out.is_empty() {
        out.push("8.8.8.8:53".into());
    }
    out.join(",")
}

pub fn b64_url_safe_no_pad(data: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    URL_SAFE_NO_PAD.encode(data)
}

pub fn build_sn_dnstt_link(payload: &[u8]) -> String {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(9));
    enc.write_all(payload).expect("zlib write");
    let compressed = enc.finish().expect("zlib finish");
    format!("sn://dnstt?{}", b64_url_safe_no_pad(&compressed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn write_int_le() {
        let mut k = KryoOutput::new();
        k.write_int(3);
        assert_eq!(k.as_slice(), &[3, 0, 0, 0]);
    }

    #[test]
    fn write_string_none_and_empty() {
        let mut k = KryoOutput::new();
        k.write_string(None);
        assert_eq!(k.as_slice(), &[0x80]);
        let mut k = KryoOutput::new();
        k.write_string(Some(""));
        assert_eq!(k.as_slice(), &[0x81]);
    }

    #[test]
    fn write_string_short_ascii() {
        let mut k = KryoOutput::new();
        k.write_string(Some("ab"));
        // 'a', 'b'|0x80
        assert_eq!(k.as_slice(), &[b'a', b'b' | 0x80]);
    }

    #[test]
    fn normalize_adds_port() {
        let s = normalize_dns_resolvers(&["8.8.8.8".into(), "1.1.1.1:53".into()]);
        assert_eq!(s, "8.8.8.8:53,1.1.1.1:53");
    }

    #[test]
    fn sn_link_prefix() {
        let bean = DnsttBean {
            server_address: "127.0.0.1".into(),
            server_port: 53,
            dns_resolver: "8.8.8.8:53".into(),
            tunnel_domain: "example.com".into(),
            public_key: "aa".into(),
            mode: 0,
            socks_username: String::new(),
            socks_password: String::new(),
            ssh_username: String::new(),
            ssh_auth_type: 0,
            ssh_password: String::new(),
            ssh_port: 22,
            ssh_private_key: String::new(),
            ssh_key_passphrase: String::new(),
            name: "t".into(),
            custom_outbound_json: String::new(),
            custom_config_json: String::new(),
        };
        let raw = serialize_dnstt_bean_v3(&bean);
        let link = build_sn_dnstt_link(&raw);
        assert!(link.starts_with("sn://dnstt?"));
        assert!(!link.contains('='));
    }

    #[test]
    fn golden_matches_python_hex_file() {
        let hex_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("testdata/kryo_golden_raw.hex");
        if !hex_path.is_file() {
            eprintln!("skip golden: missing {hex_path:?}");
            return;
        }
        let hex = std::fs::read_to_string(&hex_path).unwrap();
        let expected = hex::decode(hex.trim()).unwrap();
        let bean = DnsttBean {
            server_address: "127.0.0.1".into(),
            server_port: 53,
            dns_resolver: "8.8.8.8:53".into(),
            tunnel_domain: "example.com".into(),
            public_key: "aa".into(),
            mode: 0,
            socks_username: String::new(),
            socks_password: String::new(),
            ssh_username: String::new(),
            ssh_auth_type: 0,
            ssh_password: String::new(),
            ssh_port: 22,
            ssh_private_key: String::new(),
            ssh_key_passphrase: String::new(),
            name: "t".into(),
            custom_outbound_json: String::new(),
            custom_config_json: String::new(),
        };
        let raw = serialize_dnstt_bean_v3(&bean);
        assert_eq!(raw, expected, "Kryo bytes must match Python golden");
    }
}
