//! تولید لینک کلاینت: NetMod، NekoBox (sn://)، SlipNet (slipnet://)، پوشهٔ DMVPN.

pub mod dnstt;
pub mod kryo;
pub mod netmod;
pub mod slipnet_uri;

use crate::config;
use crate::db;
use crate::output;
use crate::resolvers;
use std::path::{Path, PathBuf};

pub type AppResult = Result<(), String>;

#[derive(Debug, Clone, Default)]
pub struct GenOpts {
    pub limit: Option<usize>,
    pub no_dmvpn: bool,
    pub shuffle: bool,
    pub ns: Option<String>,
    pub pubkey: Option<String>,
    pub remark: Option<String>,
}

fn work(base: &Path, rel: PathBuf) -> PathBuf {
    if rel.is_absolute() {
        rel
    } else {
        base.join(rel)
    }
}

fn apply_overrides(mut profile: config::Profile, opts: &GenOpts) -> config::Profile {
    if let Some(ns) = &opts.ns {
        profile.tunnel_domain = ns.clone();
    }
    if let Some(pk) = &opts.pubkey {
        profile.pubkey = pk.clone();
    }
    if let Some(r) = &opts.remark {
        profile.remark = r.clone();
        profile.profile_name = r.clone();
    }
    profile
}

fn load_profile_ips(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    opts: &GenOpts,
) -> Result<(config::Profile, Vec<String>), String> {
    let profiles = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
    let profile = apply_overrides(
        profiles
            .get(profile_name)
            .map_err(|e| e.to_string())?
            .clone(),
        opts,
    );
    let resolvers_path = work(work_dir, resolvers_path);
    let mut ips = resolvers::load_resolvers_json(&resolvers_path)?;
    if ips.is_empty() {
        return Err("resolvers list is empty".into());
    }
    if let Some(n) = opts.limit {
        ips.truncate(n);
    }
    Ok((profile, ips))
}

pub fn netmod_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    opts: &GenOpts,
) -> AppResult {
    let (profile, ips) = load_profile_ips(work_dir, profile_name, resolvers_path, opts)?;
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "netmod"));
    std::fs::create_dir_all(&run_dir).map_err(|e| e.to_string())?;
    let summary = netmod::generate(&profile, &ips, &run_dir, opts.shuffle)?;
    println!("✅ NetMod: {} لینک → {}", summary.total, run_dir.display());
    let _ = db::insert_run(
        work_dir,
        &format!(
            "gen_netmod_{}",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ),
        "generate_netmod",
        Some(profile_name),
        None,
        "ok",
        false,
        summary.total as i64,
        &run_dir.display().to_string(),
    );
    Ok(())
}

pub fn dnstt_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    mode: &str,
    opts: &GenOpts,
) -> AppResult {
    let (profile, ips) = load_profile_ips(work_dir, profile_name, resolvers_path, opts)?;
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "dnstt"));
    std::fs::create_dir_all(&run_dir).map_err(|e| e.to_string())?;
    let summary = dnstt::generate(&profile, &ips, &run_dir, mode)?;
    if !opts.no_dmvpn {
        let dmvpn = dnstt::write_dmvpn_bundle(work_dir, &profile, &summary)?;
        println!("📁 DMVPN bundle: {}", dmvpn.display());
    }
    println!(
        "✅ DNSTT/NekoBox: all={} per_dns={} → {}",
        summary.all_link.is_some(),
        summary.per_dns.len(),
        run_dir.display()
    );
    Ok(())
}

pub fn slipnet_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    opts: &GenOpts,
) -> AppResult {
    let (profile, ips) = load_profile_ips(work_dir, profile_name, resolvers_path, opts)?;
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "slipnet_uri"));
    let n = slipnet_uri::generate(&profile, &ips, &run_dir)?;
    println!("✅ SlipNet URI: {n} → {}", run_dir.display());
    Ok(())
}

pub fn all_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    opts: &GenOpts,
) -> AppResult {
    let base = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "generate_all"));
    std::fs::create_dir_all(base.join("netmod")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(base.join("dnstt")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(base.join("slipnet")).map_err(|e| e.to_string())?;
    netmod_cmd(
        work_dir,
        profile_name,
        resolvers_path.clone(),
        Some(base.join("netmod")),
        opts,
    )?;
    dnstt_cmd(
        work_dir,
        profile_name,
        resolvers_path.clone(),
        Some(base.join("dnstt")),
        "both",
        opts,
    )?;
    slipnet_cmd(
        work_dir,
        profile_name,
        resolvers_path,
        Some(base.join("slipnet")),
        opts,
    )?;
    println!("✅ generate all → {}", base.display());
    Ok(())
}
