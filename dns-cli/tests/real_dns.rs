//! تست واقعی DNS روی رزالورهای عمومی (نیاز به شبکه UDP/53).

use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dns-cli"))
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn real_scan_public_resolvers() {
    let root = root();
    let out = bin()
        .current_dir(&root)
        .args([
            "scan",
            "testdata/dns_sample.txt",
            "--preset",
            "low",
            "--workers",
            "2",
            "--timeout",
            "5",
            "--no-ping",
            "--run-id",
            "test_real_dns",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let ips = root.join("runs/test_real_dns/scan/dns_ok_and_dnsonly_ips.json");
    assert!(ips.is_file(), "missing {}", ips.display());
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ips).unwrap()).unwrap();
    let n = v.as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        n >= 1,
        "expected at least 1 working public resolver, got {n}"
    );
}

#[test]
fn pipeline_skip_slipnet_end_to_end() {
    let root = root();
    let out = bin()
        .current_dir(&root)
        .args([
            "pipeline",
            "run",
            "--input",
            "testdata/dns_sample.txt",
            "--profile",
            "mame",
            "--preset",
            "low",
            "--skip-slipnet",
            "--run-id",
            "test_pipeline_noslip",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let dnstt = root.join("runs/test_pipeline_noslip/configs/dnstt/dnstt_all_dns.txt");
    assert!(dnstt.is_file());
    let link = std::fs::read_to_string(dnstt).unwrap();
    assert!(link.starts_with("sn://dnstt?"));
}
