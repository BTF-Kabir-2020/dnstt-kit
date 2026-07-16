//! خط لولهٔ کامل: scan → resolvers → [slipnet e2e] → generate.

use crate::archive;
use crate::backup::{self, BackupMode, BackupOpts};
use crate::config;
use crate::db;
use crate::generate;
use crate::output;
use crate::presets::{PresetName, ScanPreset};
use crate::resolvers;
use crate::scan_cmd::{self, ScanArgs};
use crate::slipnet;
use clap::Args;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Args, Debug)]
pub struct PipelineArgs {
    #[arg(long, default_value = "testdata/dns_sample.txt")]
    pub input: PathBuf,

    #[arg(long, default_value = "mame")]
    pub profile: String,

    #[arg(long, default_value = "low")]
    pub preset: String,

    #[arg(long, env = "SLIPNET_PATH")]
    pub slipnet: Option<PathBuf>,

    /// اگر slipnet نبود خطا بده (پیش‌فرض: skip با هشدار)
    #[arg(long)]
    pub require_slipnet: bool,

    #[arg(long)]
    pub skip_slipnet: bool,

    #[arg(long)]
    pub skip_scan: bool,

    #[arg(long)]
    pub skip_generate: bool,

    /// کانفیگ slipnet (slipnet://...). اگر خالی باشد از SLIPNET_CONFIG / .env خوانده می‌شود.
    #[arg(long, env = "SLIPNET_CONFIG")]
    pub slipnet_config: Option<String>,

    #[arg(long)]
    pub run_id: Option<String>,

    /// اگر محلی نبود از GitHub دانلود کن (نیاز به نت)
    #[arg(long)]
    pub fetch_slipnet: bool,

    /// حتی اگر slipnet محلی بود دوباره دانلود کن
    #[arg(long)]
    pub force_fetch_slipnet: bool,

    /// فقط N IP اول از ورودی اسکن
    #[arg(long)]
    pub limit: Option<usize>,

    /// workers اسکن (override preset)
    #[arg(short = 'j', long)]
    pub workers: Option<usize>,

    /// dry-run: فقط مسیر/دستورات را نشان بده
    #[arg(long)]
    pub dry_run: bool,

    /// خروجی DMVPN نساز
    #[arg(long)]
    pub no_dmvpn: bool,

    /// انواع generate: netmod,dnstt,slipnet یا all (پیش‌فرض)
    #[arg(long, default_value = "all")]
    pub generate_kinds: String,

    #[arg(short, long)]
    pub quiet: bool,

    /// بعد از موفقیت، run را zip کن
    #[arg(long)]
    pub auto_archive: bool,

    /// بعد از موفقیت، بکاپ kit بساز
    #[arg(long)]
    pub auto_backup: bool,

    /// فقط probe باینری slipnet (--help) به‌جای e2e کامل
    #[arg(long)]
    pub slipnet_probe: bool,
}

pub type AppResult = Result<(), String>;

pub fn run(work_dir: &Path, args: PipelineArgs) -> AppResult {
    let run_id = args
        .run_id
        .clone()
        .unwrap_or_else(|| output::new_run_id("pipeline"));
    let run_root = work_dir.join("runs").join(&run_id);
    fs::create_dir_all(&run_root).map_err(|e| e.to_string())?;
    println!("📁 run: {}", run_root.display());

    if args.dry_run {
        println!(
            "[dry-run] input={:?} profile={} preset={} limit={:?} kinds={} fetch={}",
            args.input,
            args.profile,
            args.preset,
            args.limit,
            args.generate_kinds,
            args.fetch_slipnet
        );
        println!(
            "[dry-run] skip_scan={} skip_slipnet={} skip_generate={}",
            args.skip_scan, args.skip_slipnet, args.skip_generate
        );
        return Ok(());
    }

    if args.fetch_slipnet || args.force_fetch_slipnet {
        match slipnet::fetch_to_vendor(
            work_dir,
            slipnet::DEFAULT_SLIPNET_TAG,
            args.force_fetch_slipnet,
        ) {
            Ok(p) => println!("slipnet ready: {}", p.display()),
            Err(e) => {
                if args.require_slipnet {
                    return Err(e);
                }
                println!("⚠️  fetch slipnet: {e}");
            }
        }
    }

    let profiles = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
    let profile = profiles.get(&args.profile).map_err(|e| e.to_string())?;

    // --- scan ---
    if !args.skip_scan {
        let scan_args = ScanArgs {
            input: args.input.clone(),
            preset: args.preset.clone(),
            workers: args.workers,
            timeout: None,
            no_ping: false,
            domain: Some(profile.tunnel_domain.clone()),
            a_domain: Some(profile.a_domain.clone()),
            extra_domains: if profile.extra_domains.is_empty() {
                Some(vec![profile.tunnel_domain.clone()])
            } else {
                Some(profile.extra_domains.clone())
            },
            udp_attempts: ScanPreset::named(
                PresetName::parse(&args.preset).unwrap_or(PresetName::Low),
            )
            .udp_attempts,
            udp_backoff_ms: 150,
            stream: false,
            legacy_out: true,
            no_legacy_out: false,
            limit: args.limit,
            ok_only: false,
            enable_tcp: false,
            quiet: args.quiet,
            run_id: Some(run_id.clone()),
        };
        scan_cmd::run(work_dir, scan_args)?;
    } else {
        println!("⏭️  skip-scan");
    }

    // --- resolvers from scan ---
    let scan_json = run_root.join("scan").join("dns_ok_and_dnsonly_ips.json");
    let legacy_json = work_dir.join("out/json/dns_ok_and_dnsonly_ips.json");
    let from_json = if scan_json.is_file() {
        Some(scan_json)
    } else if legacy_json.is_file() {
        Some(legacy_json)
    } else {
        None
    };
    let resolvers_path = run_root.join("resolvers.json");
    resolvers::sync(
        work_dir,
        from_json,
        Some(work_dir.join("out/txt/dns_ok_and_dnsonly_ips.txt")),
        resolvers_path.clone(),
        args.limit,
    )?;
    // also copy to work_dir/resolvers.json for generators convenience
    if resolvers_path.is_file() {
        let _ = fs::copy(&resolvers_path, work_dir.join("resolvers.json"));
    }

    // --- slipnet e2e / probe ---
    let mut e2e_ok = false;
    if args.skip_slipnet && !args.slipnet_probe {
        println!("⏭️  skip-slipnet");
    } else if args.slipnet_probe {
        match slipnet::probe(work_dir, args.slipnet.clone()) {
            Ok(msg) => {
                println!("✅ slipnet probe: {msg}");
                e2e_ok = true;
            }
            Err(e) => {
                if args.require_slipnet {
                    return Err(e);
                }
                println!("⚠️  slipnet probe: {e}");
            }
        }
    } else {
        match slipnet::find_or_fetch(
            work_dir,
            args.slipnet.clone(),
            args.fetch_slipnet,
            args.force_fetch_slipnet,
            slipnet::DEFAULT_SLIPNET_TAG,
        ) {
            Ok(bin) => {
                let cfg = args
                    .slipnet_config
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| {
                        std::env::var("SLIPNET_CONFIG")
                            .ok()
                            .filter(|s| !s.trim().is_empty())
                    })
                    .unwrap_or_default();
                if cfg.trim().is_empty() {
                    let msg = "slipnet پیدا شد ولی --slipnet-config / SLIPNET_CONFIG خالی است؛ e2e رد شد. (docs/ENV.md)";
                    if args.require_slipnet {
                        return Err(msg.into());
                    }
                    println!("⚠️  {msg}");
                } else {
                    let ips = work_dir.join("dns_ok_and_dnsonly_ips.txt");
                    if !ips.is_file() {
                        let list =
                            resolvers::load_resolvers_json(&resolvers_path).unwrap_or_default();
                        resolvers::write_ips_txt(&ips, &list)?;
                    }
                    let e2e_out = run_root.join("e2e_passed.txt");
                    println!("🚀 slipnet e2e → {}", e2e_out.display());
                    match run_slipnet_scan(&bin, &cfg, &ips, &e2e_out, work_dir) {
                        Ok(()) => {
                            e2e_ok = true;
                            if e2e_out.is_file() {
                                let text = fs::read_to_string(&e2e_out).unwrap_or_default();
                                let e2e_ips: Vec<String> = text
                                    .lines()
                                    .map(|l| l.trim())
                                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                                    .map(|l| l.split_whitespace().next().unwrap_or("").to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let e2e_ips = resolvers::normalize_list(e2e_ips);
                                if !e2e_ips.is_empty() {
                                    resolvers::write_resolvers_json(&resolvers_path, &e2e_ips)?;
                                    let _ =
                                        fs::copy(&resolvers_path, work_dir.join("resolvers.json"));
                                    println!("✅ e2e → {} resolver", e2e_ips.len());
                                } else {
                                    println!("ℹ️  e2e خالی؛ resolvers همان خروجی scan ماند.");
                                }
                            }
                        }
                        Err(e) => {
                            if args.require_slipnet {
                                return Err(e);
                            }
                            println!("⚠️  slipnet failed: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                if args.require_slipnet {
                    return Err(e);
                }
                println!("⚠️  {e}");
            }
        }
    }

    // --- generate ---
    let mut working_count = 0i64;
    if let Ok(list) = resolvers::load_resolvers_json(&resolvers_path) {
        working_count = list.len() as i64;
    }
    if !args.skip_generate {
        let gen_dir = run_root.join("configs");
        fs::create_dir_all(gen_dir.join("netmod")).map_err(|e| e.to_string())?;
        fs::create_dir_all(gen_dir.join("dnstt")).map_err(|e| e.to_string())?;
        fs::create_dir_all(gen_dir.join("slipnet")).map_err(|e| e.to_string())?;
        let kinds = args.generate_kinds.to_ascii_lowercase();
        let want_all = kinds == "all";
        let want_netmod = want_all || kinds.contains("netmod");
        let want_dnstt = want_all || kinds.contains("dnstt");
        let want_slip = want_all || kinds.contains("slipnet");
        let gen_opts = generate::GenOpts {
            limit: None,
            no_dmvpn: args.no_dmvpn,
            shuffle: true,
            ns: None,
            pubkey: None,
            remark: None,
        };
        if want_netmod {
            generate::netmod_cmd(
                work_dir,
                &args.profile,
                resolvers_path.clone(),
                Some(gen_dir.join("netmod")),
                &gen_opts,
            )?;
        }
        if want_dnstt {
            generate::dnstt_cmd(
                work_dir,
                &args.profile,
                resolvers_path.clone(),
                Some(gen_dir.join("dnstt")),
                "both",
                &gen_opts,
            )?;
        }
        if want_slip {
            generate::slipnet_cmd(
                work_dir,
                &args.profile,
                resolvers_path.clone(),
                Some(gen_dir.join("slipnet")),
                &gen_opts,
            )?;
        }
        let _ = db::add_artifact(
            work_dir,
            &run_id,
            "configs",
            &gen_dir.display().to_string(),
            None,
        );
    }

    let summary = serde_json::json!({
        "run_id": run_id,
        "profile": args.profile,
        "preset": args.preset,
        "e2e_ok": e2e_ok,
        "resolvers": resolvers_path.display().to_string(),
        "working_count": working_count,
    });
    fs::write(
        run_root.join("summary.json"),
        serde_json::to_string_pretty(&summary).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    let _ = db::insert_run(
        work_dir,
        &run_id,
        "pipeline",
        Some(&args.profile),
        Some(&args.preset),
        "ok",
        e2e_ok,
        working_count,
        &run_root.display().to_string(),
    );
    println!("✅ pipeline تمام شد. e2e_ok={e2e_ok}");

    if args.auto_archive {
        match archive::pack(work_dir, &run_id, 30, false) {
            Ok(()) => {}
            Err(e) => println!("⚠️  auto-archive: {e}"),
        }
    }
    if args.auto_backup {
        let opts = BackupOpts {
            mode: BackupMode::Kit,
            keep: 20,
            include_secrets: false,
            include_runs: false,
            include_vendor: false,
            label: Some(format!("after_{run_id}")),
        };
        match backup::create(work_dir, &opts) {
            Ok(()) => {}
            Err(e) => println!("⚠️  auto-backup: {e}"),
        }
    }
    Ok(())
}

fn run_slipnet_scan(
    bin: &Path,
    config: &str,
    ips: &Path,
    output: &Path,
    cwd: &Path,
) -> Result<(), String> {
    let mut child = Command::new(bin)
        .arg("scan")
        .arg("--config")
        .arg(config)
        .arg("--ips")
        .arg(ips)
        .arg("--output")
        .arg(output)
        .arg("--e2e")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn slipnet: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            println!("{line}");
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("slipnet exit {:?}", status.code()));
    }
    Ok(())
}
