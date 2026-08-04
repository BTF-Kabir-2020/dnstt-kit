//! Client link generators: NetMod (`dns://`), DMVPN (`sn://dnstt?`), SlipNet (`slipnet://`).

pub mod dnstt;
pub mod flat_dns;
pub mod kryo;
pub mod netmod;
pub mod slipnet_uri;

use crate::config;
use crate::db;
use crate::names;
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
    /// Shared batch tag for one generate run; if `None`, a new tag is minted.
    pub batch: Option<String>,
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
        let r = names::ensure_person_name(r);
        profile.remark = r.clone();
        profile.profile_name = r;
    }
    profile
}

fn resolve_batch(opts: &GenOpts) -> String {
    opts.batch
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(names::new_batch_tag)
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
    let batch = resolve_batch(opts);
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "netmod"));
    std::fs::create_dir_all(&run_dir).map_err(|e| e.to_string())?;
    let summary = netmod::generate(&profile, &ips, &run_dir, opts.shuffle, &batch)?;
    println!(
        "✅ NetMod: {} لینک (batch {batch}) → {}",
        summary.total,
        run_dir.display()
    );
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

/// Emit DMVPN import links (`sn://dnstt?…`) into `out_dir` and optionally the root `DMVPN/` bundle.
pub fn dmvpn_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    mode: &str,
    opts: &GenOpts,
) -> AppResult {
    let (profile, ips) = load_profile_ips(work_dir, profile_name, resolvers_path, opts)?;
    let batch = resolve_batch(opts);
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "dmvpn"));
    std::fs::create_dir_all(&run_dir).map_err(|e| e.to_string())?;
    let summary = dnstt::generate(&profile, &ips, &run_dir, mode, &batch)?;
    if !opts.no_dmvpn {
        let dmvpn = dnstt::write_dmvpn_bundle(work_dir, &profile, &summary, &batch)?;
        println!("📁 {} bundle: {}", names::DMVPN_LABEL, dmvpn.display());
    }
    println!(
        "✅ {}: all={} per_dns={} (batch {batch}) → {}",
        names::DMVPN_LABEL,
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
    let batch = resolve_batch(opts);
    let run_dir = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "slipnet_uri"));
    let n = slipnet_uri::generate(&profile, &ips, &run_dir, &batch)?;
    println!(
        "✅ SlipNet URI: {n} (batch {batch}) → {}",
        run_dir.display()
    );
    Ok(())
}

/// Flat `dns.txt` from an IP list (Python `generate_dns.py` style).
/// Prefer `--profile` **or** pass `--ns` + `--pubkey` (+ optional user/pass/ps).
#[derive(Debug, Clone)]
pub struct FlatDnsCmd {
    pub input: PathBuf,
    pub out: PathBuf,
    pub profile_name: Option<String>,
    pub ps: Option<String>,
    pub ns: Option<String>,
    pub pubkey: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub port: u16,
    pub dedup: bool,
    pub limit: Option<usize>,
}

pub fn flat_dns_cmd(work_dir: &Path, cmd: FlatDnsCmd) -> AppResult {
    let input = work(work_dir, cmd.input);
    let out = work(work_dir, cmd.out);
    let profile_name = cmd.profile_name.as_deref();

    let n = if let Some(name) = profile_name {
        let profiles = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
        let profile = profiles.get(name).map_err(|e| e.to_string())?;
        let mut pr = profile.clone();
        if let Some(ns) = cmd.ns {
            pr.tunnel_domain = ns;
        }
        if let Some(pk) = cmd.pubkey {
            pr.pubkey = pk;
        }
        if let Some(u) = cmd.user {
            pr.ssh_user = u;
            pr.include_ssh = true;
        }
        if let Some(p) = cmd.pass {
            pr.ssh_pass = p;
            pr.include_ssh = true;
        }
        flat_dns::from_profile(
            &pr,
            &input,
            &out,
            cmd.ps.as_deref(),
            cmd.port,
            cmd.dedup,
            cmd.limit,
        )?
    } else {
        let ns = cmd.ns.ok_or_else(|| {
            "need --profile NAME  or  --ns + --pubkey (and usually --ps/--user/--pass)".to_string()
        })?;
        let pubkey = cmd
            .pubkey
            .ok_or_else(|| "--pubkey is required without --profile".to_string())?;
        let ps = cmd.ps.unwrap_or_else(|| "dnstt".into());
        flat_dns::run(flat_dns::FlatDnsArgs {
            input: input.clone(),
            out: out.clone(),
            ps,
            ns,
            pubkey,
            user: cmd.user.unwrap_or_default(),
            pass: cmd.pass.unwrap_or_default(),
            port: cmd.port,
            dedup: cmd.dedup,
            limit: cmd.limit,
        })?
    };

    println!("✅ flat dns: {n} configs → {}", out.display());
    let _ = db::insert_run(
        work_dir,
        &format!(
            "gen_flat_dns_{}",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ),
        "generate_dns",
        profile_name,
        None,
        "ok",
        false,
        n as i64,
        &out.display().to_string(),
    );
    Ok(())
}

pub fn all_cmd(
    work_dir: &Path,
    profile_name: &str,
    resolvers_path: PathBuf,
    out_dir: Option<PathBuf>,
    opts: &GenOpts,
) -> AppResult {
    let batch = resolve_batch(opts);
    let mut shared = opts.clone();
    shared.batch = Some(batch.clone());
    let base = out_dir
        .map(|p| work(work_dir, p))
        .unwrap_or_else(|| output::new_run_dir(work_dir, "generate_all"));
    std::fs::create_dir_all(base.join("netmod")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(base.join("dmvpn")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(base.join("slipnet")).map_err(|e| e.to_string())?;
    netmod_cmd(
        work_dir,
        profile_name,
        resolvers_path.clone(),
        Some(base.join("netmod")),
        &shared,
    )?;
    dmvpn_cmd(
        work_dir,
        profile_name,
        resolvers_path.clone(),
        Some(base.join("dmvpn")),
        "both",
        &shared,
    )?;
    slipnet_cmd(
        work_dir,
        profile_name,
        resolvers_path,
        Some(base.join("slipnet")),
        &shared,
    )?;
    println!("✅ generate all (batch {batch}) → {}", base.display());
    Ok(())
}
