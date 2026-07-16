//! زیر‌دستور `scan` — نوشتن خروجی در `runs/<id>/scan/` و `out/` سازگار با pipeline قدیمی.

use crate::output;
use crate::presets::{PresetName, ScanPreset};
use clap::Args;
use scanner_core::{run_scan, run_scan_stream, ScanConfig, ScanOutput, ScanResult};
use std::fs;
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

    /// فقط N خط اول فایل ورودی
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

    let mut input = if args.input.is_absolute() {
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

    // --limit: فایل موقت با N خط اول داخل همان run
    if let Some(n) = args.limit {
        let text = fs::read_to_string(&input).map_err(|e| e.to_string())?;
        let lines: Vec<_> = text
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with('#')
            })
            .take(n)
            .collect();
        let tmp = run_dir.join("_input_limit.txt");
        fs::write(&tmp, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        if !args.quiet {
            println!("ℹ️  --limit {n} → {} خط", lines.len());
        }
        input = tmp;
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
        let (tx, mut rx) = mpsc::channel::<ScanResult>(256);
        let scan_fut = run_scan_stream(config.clone(), tx);
        let mut collected = Vec::new();
        let collector = async {
            while let Some(r) = rx.recv().await {
                collected.push(r);
            }
            collected
        };
        let (scan_res, collected) = tokio::join!(scan_fut, collector);
        let (ok, fail, elapsed) = scan_res?;
        if !args.quiet {
            println!("زمان: {elapsed:.2}s | OK/working≈{ok} | FAIL≈{fail} (stream)");
        }
        aggregate_from_results(collected, elapsed, config.include_dns_only, ok, fail)
    } else {
        run_scan(config).await?
    };

    write_scan_outputs(&run_dir, &out)?;
    let legacy = args.legacy_out && !args.no_legacy_out;
    if legacy {
        write_legacy_out(work_dir, &out)?;
    }

    if !args.quiet {
        println!(
            "✅ scan done: total={} working={} ok_dnsonly_ips={}",
            out.total_count,
            out.working.len(),
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

fn aggregate_from_results(
    all_results: Vec<ScanResult>,
    elapsed: f64,
    include_dns_only: bool,
    ok_count: usize,
    fail_count: usize,
) -> ScanOutput {
    use std::collections::HashSet;
    let working: Vec<_> = all_results
        .iter()
        .filter(|r| r.status == "OK" || (include_dns_only && r.status == "DNS_ONLY"))
        .cloned()
        .collect();
    let mut working_sorted = working.clone();
    working_sorted.sort_by(|a, b| {
        let la = if a.latency_dns_ms >= 0.0 {
            a.latency_dns_ms
        } else {
            1e9
        };
        let lb = if b.latency_dns_ms >= 0.0 {
            b.latency_dns_ms
        } else {
            1e9
        };
        la.partial_cmp(&lb).unwrap()
    });
    let working_filtered: Vec<_> = working_sorted
        .into_iter()
        .filter(|r| r.latency_dns_ms >= 50.0)
        .collect();
    let working_ok_all: Vec<_> = working
        .iter()
        .filter(|r| r.status == "OK")
        .cloned()
        .collect();
    let dns_only_all: Vec<_> = working
        .iter()
        .filter(|r| r.status == "DNS_ONLY")
        .cloned()
        .collect();
    let mut hosts_unique: Vec<String> = working_ok_all
        .iter()
        .map(|r| r.host.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    hosts_unique.sort();
    let mut working_ip_ports: Vec<String> = working_ok_all
        .iter()
        .map(|r| format!("{}:{}", r.host, r.port))
        .collect();
    working_ip_ports.sort();
    working_ip_ports.dedup();
    let mut dns_only_ips: Vec<String> = dns_only_all.iter().map(|r| r.host.clone()).collect();
    dns_only_ips.sort();
    dns_only_ips.dedup();
    let dns_only_ip_ports: Vec<String> = dns_only_all
        .iter()
        .map(|r| format!("{}:{}", r.host, r.port))
        .collect();
    let mut combined: Vec<String> = working
        .iter()
        .map(|r| r.host.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    combined.sort();

    ScanOutput {
        elapsed_secs: elapsed,
        total_count: ok_count + fail_count,
        ok_count,
        fail_count,
        all_results,
        working,
        working_sorted_filtered: working_filtered,
        working_ips: hosts_unique,
        working_ip_ports,
        dns_only: dns_only_all,
        dns_only_ips,
        dns_only_ip_ports,
        ok_and_dnsonly_ips: combined,
    }
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
