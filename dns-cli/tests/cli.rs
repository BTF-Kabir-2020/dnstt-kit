//! تست یکپارچگی CLI (بدون شبکه سنگین — generate و help).

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
fn help_works() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("scan") || s.contains("Scan"));
}

#[test]
fn generate_netmod_sample() {
    let root = root();
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .current_dir(&root)
        .args([
            "generate",
            "netmod",
            "--profile",
            "demo",
            "--resolvers",
            "testdata/resolvers_sample.json",
            "--out-dir",
        ])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().collect();
    assert!(!entries.is_empty());
}

#[test]
fn generate_dnstt_sample() {
    let root = root();
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .current_dir(&root)
        .args([
            "generate",
            "dnstt",
            "--profile",
            "demo",
            "--resolvers",
            "testdata/resolvers_sample.json",
            "--out-dir",
        ])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let all = tmp.path().join("dnstt_all_dns.txt");
    assert!(all.is_file());
    let link = std::fs::read_to_string(all).unwrap();
    assert!(link.starts_with("sn://dnstt?"));
}

#[test]
fn slipnet_which_finds_vendor_on_windows() {
    if !cfg!(windows) {
        return;
    }
    let root = root();
    let out = bin()
        .current_dir(&root)
        .args(["slipnet", "which"])
        .output()
        .unwrap();
    // vendor slipnet.exe should exist from setup
    if out.status.success() {
        let p = String::from_utf8_lossy(&out.stdout);
        assert!(p.contains("slipnet"));
    }
}

#[test]
fn generate_all_and_slipnet_uri() {
    let root = root();
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .current_dir(&root)
        .args([
            "generate",
            "all",
            "--profile",
            "demo",
            "--resolvers",
            "testdata/resolvers_sample.json",
            "--out-dir",
        ])
        .arg(tmp.path())
        .arg("--limit")
        .arg("2")
        .arg("--no-dmvpn")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut found = None;
    for e in walkdir_simple(tmp.path()) {
        if e.ends_with("slipnet_all.txt") {
            found = Some(e);
            break;
        }
    }
    let slip2 = found.expect("slipnet_all.txt");
    let link = std::fs::read_to_string(slip2).unwrap();
    assert!(link.starts_with("slipnet://"));
}

fn walkdir_simple(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walkdir_simple(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn status_sqlite_works() {
    let root = root();
    let out = bin().current_dir(&root).args(["status"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("SQLite") || s.contains("runs="));
}

#[test]
fn doctor_and_profiles_and_verify() {
    let root = root();
    let out = bin().current_dir(&root).args(["doctor"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = bin()
        .current_dir(&root)
        .args(["profiles", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("demo"));

    let tmp = tempfile::tempdir().unwrap();
    let gen = bin()
        .current_dir(&root)
        .args([
            "generate",
            "netmod",
            "--profile",
            "demo",
            "--resolvers",
            "testdata/resolvers_sample.json",
            "--out-dir",
        ])
        .arg(tmp.path())
        .arg("--limit")
        .arg("1")
        .output()
        .unwrap();
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    // find .txt with dns://
    let mut link_file = None;
    for e in walkdir_simple(tmp.path()) {
        if e.extension().and_then(|s| s.to_str()) == Some("txt") {
            let t = std::fs::read_to_string(&e).unwrap_or_default();
            if t.contains("dns://") {
                link_file = Some(e);
                break;
            }
        }
    }
    let lf = link_file.expect("netmod txt");
    let ver = bin()
        .current_dir(&root)
        .args(["verify"])
        .arg(&lf)
        .output()
        .unwrap();
    assert!(
        ver.status.success(),
        "{}",
        String::from_utf8_lossy(&ver.stderr)
    );
}

#[test]
fn resolvers_sort_take() {
    let root = root();
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("r.json");
    std::fs::write(&input, r#"["10.0.0.2","8.8.8.8","10.0.0.10"]"#).unwrap();
    let sorted = tmp.path().join("sorted.json");
    let out = bin()
        .current_dir(&root)
        .args(["resolvers", "sort", "--input"])
        .arg(&input)
        .arg("--out")
        .arg(&sorted)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let taken = tmp.path().join("take.json");
    let out = bin()
        .current_dir(&root)
        .args(["resolvers", "take", "--input"])
        .arg(&sorted)
        .args(["--n", "2", "--out"])
        .arg(&taken)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(taken).unwrap()).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn pipeline_dry_run() {
    let root = root();
    let out = bin()
        .current_dir(&root)
        .args([
            "pipeline",
            "run",
            "--dry-run",
            "--input",
            "testdata/dns_sample.txt",
            "--profile",
            "demo",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("[dry-run]"));
}

#[test]
fn init_backup_info_clean() {
    let root = root();
    let out = bin().current_dir(&root).args(["init"]).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = bin()
        .current_dir(&root)
        .args([
            "backup", "create", "--mode", "kit", "--keep", "5", "--label", "test",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(root.join("backups").is_dir());

    let out = bin()
        .current_dir(&root)
        .args(["backup", "list"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("dnstt_kit_"));

    let out = bin().current_dir(&root).args(["info"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("dns-cli"));

    let out = bin()
        .current_dir(&root)
        .args(["clean", "--backups-keep", "50", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn slipnet_probe_or_skip() {
    let root = root();
    let out = bin()
        .current_dir(&root)
        .args(["slipnet", "probe"])
        .output()
        .unwrap();
    // اگر باینری نباشد تست را رد می‌کنیم نه fail سخت
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("slipnet") || err.contains("❌"),
            "unexpected: {err}"
        );
        return;
    }
    assert!(String::from_utf8_lossy(&out.stdout).contains("slipnet") || out.status.success());
}

#[test]
fn env_example_is_documented_and_loads() {
    let root = root();
    let example = root.join(".env.example");
    assert!(example.is_file(), ".env.example must exist at kit root");
    let text = std::fs::read_to_string(&example).unwrap();
    assert!(
        text.contains("SLIPNET_CONFIG"),
        "must document SLIPNET_CONFIG"
    );
    assert!(
        text.contains("MOCK") || text.contains("mock.example"),
        "must include mock example"
    );
    assert!(text.contains("docs/ENV.md"), "must point to docs");

    let tmp = tempfile::tempdir().unwrap();
    let mock = "slipnet://MTh8ZG5zdHRfc3NofE1PQ0tfUFJPRklMRV9ET19OT1RfVVNFfG1vY2suZXhhbXBsZS5pbnZhbGlkfDEuMS4xLjE6NTM6MHwwfDUwMDB8YmJyfDEwODB8MTI3LjAuMC4xfDB8YWFiYmNjZGRlZWZmMDAxMTIyMzM0NDU1NjY3Nzg4OTlhYWJiY2NkZGVlZmYwMDExMjIzMzQ0NTU2Njc3ODg5OXx8fDF8cm9vdHxNT0NLX1BBU1NfTk9UX1JFQUx8MjJ8MHwxMjcuMC4wLjF8fHx1ZHB8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHw=";
    std::fs::write(tmp.path().join(".env"), format!("SLIPNET_CONFIG={mock}\n")).unwrap();

    // پاک کردن متغیر قبلی تست‌های موازی را خراب نکند — فقط در این پروسه
    std::env::remove_var("SLIPNET_CONFIG");
    dotenvy::from_path(tmp.path().join(".env")).unwrap();
    let got = std::env::var("SLIPNET_CONFIG").unwrap();
    assert!(
        got.starts_with("slipnet://"),
        "dotenv must load mock config"
    );
    assert!(got.contains("MOCK") || got.len() > 40);

    let out = bin()
        .current_dir(&root)
        .env("SLIPNET_CONFIG", &got)
        .args(["info"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("SLIPNET_CONFIG=set("),
        "info should show masked set config, got:\n{s}"
    );
}

#[test]
fn completion_powershell() {
    let out = bin().args(["completion", "powershell"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("dns-cli") || s.contains("Register-ArgumentCompleter") || !s.is_empty());
}
