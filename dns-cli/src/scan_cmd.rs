//! زیر‌دستور `scan` — نوشتن خروجی در `runs/<id>/scan/` و `out/` سازگار با pipeline قدیمی.

use crate::output;
use crate::presets::{PresetName, ScanPreset};
use clap::Args;
use scanner_core::{run_scan, run_scan_stream, ScanConfig, ScanOutput, ScanResult};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

#[derive(Args, Debug)]
pub struct ScanArgs {
    /// فایل لیست IP (مثل tokhmi.txt)
    pub input: PathBuf,

    #[arg(long, default_value = "normal")]
    pub preset: String,

    #[arg(short = 'j', long)]
    pub workers: Option<usize>,

    #[arg(long)]
    pub timeout: Option<f64>,

    #[arg(long)]
    pub no_ping: bool,

    #[arg(long)]
    pub domain: Option<String>,

    #[arg(long)]
    pub a_domain: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub extra_domains: Option<Vec<String>>,

    #[arg(long, default_value_t = 2)]
    pub udp_attempts: u32,

    #[arg(long, default_value_t = 150)]
    pub udp_backoff_ms: u64,

    /// اجبار استریم (کم‌رم) حتی اگر preset نگوید
    #[arg(long)]
    pub stream: bool,

    /// نوشتن سازگار با مسیر قدیمی out/txt و out/json
    #[arg(long, default_value_t = true)]
    pub legacy_out: bool,

    /// غیرفعال کردن out/txt و out/json
    #[arg(long)]
    pub no_legacy_out: bool,

    /// فقط N هدف معتبر اول (خط‌به‌خط؛ کل فایل در RAM بار نمی‌شود)
    #[arg(long)]
    pub limit: Option<usize>,

    /// فقط status=OK (بدون DNS_ONLY)
    #[arg(long)]
    pub ok_only: bool,

    /// تست TCP 853 هم
    #[arg(long)]
    pub enable_tcp: bool,

    #[arg(short, long)]
    pub quiet: bool,

    #[arg(long)]
    pub run_id: Option<String>,
}

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path, args: ScanArgs) -> AppResult {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(run_async(work_dir, args))
}

async fn run_async(work_dir: &Path, args: ScanArgs) -> AppResult {
    struct QuietReset;
    impl Drop for QuietReset {
        fn drop(&mut self) {
            scanner_core::set_quiet(false);
        }
    }
    scanner_core::set_quiet(args.quiet);
    let _quiet_reset = QuietReset;

    let preset = PresetName::parse(&args.preset)
        .map(ScanPreset::named)
        .unwrap_or_else(|| ScanPreset::named(PresetName::Normal));

    let input = if args.input.is_absolute() {
        args.input.clone()
    } else {
        work_dir.join(&args.input)
    };
    if !input.is_file() {
        return Err(format!("input not found: {}", input.display()));
    }

    let run_dir = if let Some(id) = &args.run_id {
        let d = work_dir.join("runs").join(id).join("scan");
        fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        d
    } else {
        let d = output::new_run_dir(work_dir, "scan");
        fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        d
    };

    if let Some(n) = args.limit {
        if !args.quiet {
            println!("ℹ️  --limit {n} (line-by-line, no full-file load)");
        }
    }

    let mut config = ScanConfig {
        input_file: input,
        timeout: args.timeout.unwrap_or(preset.timeout),
        a_probe_domain: args.a_domain.clone(),
        domain: args
            .domain
            .clone()
            .unwrap_or_else(|| "cloudflare.com".into()),
        extra_domains: args.extra_domains.clone().unwrap_or(preset_extra()),
        enable_tcp: args.enable_tcp,
        include_dns_only: !args.ok_only,
        workers: args.workers.unwrap_or(preset.workers),
        no_ping: args.no_ping || preset.no_ping,
        ping_timeout: None,
        udp_attempts: args.udp_attempts.max(1),
        udp_backoff_ms: args.udp_backoff_ms,
        max_targets: args.limit,
    };

    // اگر domain پروفایل داده نشده، extra پیش‌فرض نگه دار
    if config.extra_domains.is_empty() {
        config.extra_domains = vec!["cloudflare.com".into(), "example.com".into()];
    }

    let use_stream = args.stream || preset.stream;
    if !args.quiet {
        println!(
            "🔍 scan workers={} timeout={} stream={} ok_only={} → {}",
            config.workers,
            config.timeout,
            use_stream,
            args.ok_only,
            run_dir.display()
        );
    }

    let out = if use_stream {
        run_stream_to_disk(&run_dir, config, args.quiet).await?
    } else {
        run_scan(config).await?
    };

    if !use_stream {
        write_scan_outputs(&run_dir, &out)?;
    }
    let legacy = args.legacy_out && !args.no_legacy_out;
    if legacy {
        write_legacy_out(work_dir, &out)?;
    }

    if !args.quiet {
        println!(
            "✅ scan done: total={} ok/working≈{} ok_dnsonly_ips={}",
            out.total_count,
            out.ok_count,
            out.ok_and_dnsonly_ips.len()
        );
        println!("   run: {}", run_dir.display());
    }
    let run_key = run_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("scan");
    let _ = crate::db::insert_run(
        work_dir,
        run_key,
        "scan",
        None,
        Some(&args.preset),
        "ok",
        false,
        out.ok_and_dnsonly_ips.len() as i64,
        &run_dir.display().to_string(),
    );
    Ok(())
}

fn preset_extra() -> Vec<String> {
    vec!["cloudflare.com".into(), "example.com".into()]
}

/// استریم واقعی: نتایج را هم‌زمان روی دیسک می‌نویسد؛ فقط IPهای working در RAM می‌مانند.
async fn run_stream_to_disk(
    run_dir: &Path,
    config: ScanConfig,
    quiet: bool,
) -> Result<ScanOutput, String> {
    let include_dns_only = config.include_dns_only;
    let workers = config.workers.clamp(1, 512);
    let chan = workers.saturating_mul(2).clamp(32, 256);
    let (tx, mut rx) = mpsc::channel::<ScanResult>(chan);

    let all_path = run_dir.join("dns_all_results.txt");
    let mut all_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&all_path)
        .map_err(|e| e.to_string())?;
    all_file
        .write_all(
            b"original\thost\tport\tstatus\tlatency_dns_ms\tlatency_txt_ms\tlatency_ping_ms\trecursive\ttxt_ok\terror\n",
        )
        .map_err(|e| e.to_string())?;

    let scan_fut = run_scan_stream(config, tx);
    let writer = async {
        let mut working_ok: HashSet<String> = HashSet::new();
        let mut dns_only: HashSet<String> = HashSet::new();
        let mut working_ip_ports: HashSet<String> = HashSet::new();
        let mut dns_only_ip_ports: HashSet<String> = HashSet::new();
        let mut done = 0usize;
        let mut buf = Vec::with_capacity(512);

        while let Some(r) = rx.recv().await {
            buf.clear();
            write_line_bytes(&r, &mut buf);
            all_file.write_all(&buf).map_err(|e| e.to_string())?;

            if r.status == "OK" {
                working_ok.insert(r.host.clone());
                working_ip_ports.insert(format!("{}:{}", r.host, r.port));
            } else if include_dns_only && r.status == "DNS_ONLY" {
                dns_only.insert(r.host.clone());
                dns_only_ip_ports.insert(format!("{}:{}", r.host, r.port));
            }

            done += 1;
            if !quiet && done % 1000 == 0 {
                println!("… scanned {done}");
            }
        }
        all_file.flush().map_err(|e| e.to_string())?;

        Ok::<_, String>((working_ok, dns_only, working_ip_ports, dns_only_ip_ports))
    };

    let (scan_res, writer_res) = tokio::join!(scan_fut, writer);
    let (ok, fail, elapsed) = scan_res?;
    let (working_ok, dns_only, working_ip_ports, dns_only_ip_ports) = writer_res?;

    if !quiet {
        println!("زمان: {elapsed:.2}s | OK/working≈{ok} | FAIL≈{fail} (stream→disk)");
    }

    let mut hosts_unique: Vec<String> = working_ok.into_iter().collect();
    hosts_unique.sort();
    let mut working_ip_ports: Vec<String> = working_ip_ports.into_iter().collect();
    working_ip_ports.sort();
    let mut dns_only_ips: Vec<String> = dns_only.into_iter().collect();
    dns_only_ips.sort();
    let mut dns_only_ip_ports: Vec<String> = dns_only_ip_ports.into_iter().collect();
    dns_only_ip_ports.sort();

    let mut combined: Vec<String> = hosts_unique
        .iter()
        .chain(dns_only_ips.iter())
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    combined.sort();

    let out = ScanOutput {
        elapsed_secs: elapsed,
        total_count: ok + fail,
        ok_count: ok,
        fail_count: fail,
        // در حالت استریم، نتایج کامل فقط روی دیسک است (نه در RAM)
        all_results: Vec::new(),
        working: Vec::new(),
        working_sorted_filtered: Vec::new(),
        working_ips: hosts_unique,
        working_ip_ports,
        dns_only: Vec::new(),
        dns_only_ips,
        dns_only_ip_ports,
        ok_and_dnsonly_ips: combined,
    };

    write_stream_summaries(run_dir, &out)?;
    Ok(out)
}

fn write_stream_summaries(dir: &Path, out: &ScanOutput) -> AppResult {
    fs::write(
        dir.join("dns_ok_and_dnsonly_ips.txt"),
        out.ok_and_dnsonly_ips.join("\n") + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("dns_ok_and_dnsonly_ips.json"),
        serde_json::to_string_pretty(&out.ok_and_dnsonly_ips).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "elapsed_secs": out.elapsed_secs,
            "total_count": out.total_count,
            "ok_count": out.ok_count,
            "fail_count": out.fail_count,
            "working_ips": out.working_ips.len(),
            "ok_and_dnsonly_ips": out.ok_and_dnsonly_ips.len(),
            "stream": true,
            "all_results_file": "dns_all_results.txt",
        }))
        .map_err(|e| e.to_string())?
            + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_line_bytes(r: &ScanResult, buf: &mut Vec<u8>) {
    use std::io::Write as _;
    let _ = writeln!(
        buf,
        "{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}",
        r.original,
        r.host,
        r.port,
        r.status,
        r.latency_dns_ms,
        r.latency_txt_ms,
        r.latency_ping_ms,
        r.recursive,
        r.txt_ok,
        r.error
    );
}

fn write_line(r: &ScanResult) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}\n",
        r.original,
        r.host,
        r.port,
        r.status,
        r.latency_dns_ms,
        r.latency_txt_ms,
        r.latency_ping_ms,
        r.recursive,
        r.txt_ok,
        r.error
    )
}

fn write_scan_outputs(dir: &Path, out: &ScanOutput) -> AppResult {
    let header = "original\thost\tport\tstatus\tlatency_dns_ms\tlatency_txt_ms\tlatency_ping_ms\trecursive\ttxt_ok\terror\n";
    let mut all = String::from(header);
    for r in &out.all_results {
        all.push_str(&write_line(r));
    }
    fs::write(dir.join("dns_all_results.txt"), all).map_err(|e| e.to_string())?;
    fs::write(
        dir.join("dns_ok_and_dnsonly_ips.txt"),
        out.ok_and_dnsonly_ips.join("\n") + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("dns_ok_and_dnsonly_ips.json"),
        serde_json::to_string_pretty(&out.ok_and_dnsonly_ips).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "elapsed_secs": out.elapsed_secs,
            "total_count": out.total_count,
            "ok_count": out.ok_count,
            "fail_count": out.fail_count,
            "working": out.working.len(),
            "ok_and_dnsonly_ips": out.ok_and_dnsonly_ips.len(),
        }))
        .map_err(|e| e.to_string())?
            + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_legacy_out(work_dir: &Path, out: &ScanOutput) -> AppResult {
    let txt = work_dir.join("out/txt");
    let json = work_dir.join("out/json");
    fs::create_dir_all(&txt).map_err(|e| e.to_string())?;
    fs::create_dir_all(&json).map_err(|e| e.to_string())?;
    fs::write(
        txt.join("dns_ok_and_dnsonly_ips.txt"),
        out.ok_and_dnsonly_ips.join("\n") + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        json.join("dns_ok_and_dnsonly_ips.json"),
        serde_json::to_string_pretty(&out.ok_and_dnsonly_ips).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
