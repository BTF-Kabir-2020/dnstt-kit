//! `dns-cli` — نقطهٔ ورود واحد برای اسکن DNS، pipeline، تولید لینک، slipnet، وب، بکاپ و بیلد.

mod archive;
mod backup;
mod clean;
mod config;
mod db;
mod decode;
mod doctor;
mod env_file;
mod generate;
mod init_cmd;
mod jobs;
mod menu;
mod output;
mod pipeline;
mod presets;
mod resolvers;
mod scan_cmd;
mod slipnet;
mod verify;
mod web;
mod web_security;
mod workdir;

use clap::{CommandFactory, Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "dns-cli",
    version,
    about = "dnstt-kit — DNS scan, config generate, slipnet, web panel, backup",
    long_about = "dnstt-kit (dns-cli): one toolkit for DNSTT.\n\
Beginners: run with NO args → starter guide + menu.\n\
Web panel: dns-cli serve   →  http://127.0.0.1:8787\n\
Offline-first slipnet (vendor/). Fetch only with `slipnet fetch`."
)]
struct Cli {
    /// work_dir (profiles, testdata, runs…). Default: cwd or DNS_CLI_WORK_DIR
    #[arg(long, global = true, env = "DNS_CLI_WORK_DIR")]
    work_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// init folders + profiles.json from example
    Init {
        #[arg(long)]
        force_profiles: bool,
    },
    /// DNS resolver scan
    Scan(scan_cmd::ScanArgs),
    Resolvers {
        #[command(subcommand)]
        action: ResolversCmd,
    },
    /// generate NetMod / DNSTT / SlipNet links
    Generate {
        #[command(subcommand)]
        kind: GenerateCmd,
    },
    /// scan + generate pipeline
    Pipeline {
        #[command(subcommand)]
        action: PipelineCmd,
    },
    Slipnet {
        #[command(subcommand)]
        action: SlipnetCmd,
    },
    Archive {
        #[command(subcommand)]
        action: ArchiveCmd,
    },
    Backup {
        #[command(subcommand)]
        action: BackupCmd,
    },
    Clean {
        #[arg(long)]
        runs_keep: Option<usize>,
        #[arg(long)]
        archives_keep: Option<usize>,
        #[arg(long)]
        backups_keep: Option<usize>,
        #[arg(long)]
        logs: bool,
        #[arg(long)]
        out: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Profiles {
        #[command(subcommand)]
        action: ProfilesCmd,
    },
    /// health check (doctor)
    Doctor {
        #[arg(long)]
        fetch_hint: bool,
    },
    Verify {
        path: PathBuf,
    },
    /// decode dns:// / slipnet:// / sn://dnstt? (optionally save local profile)
    Decode {
        /// URI string (quote it in PowerShell)
        uri: Option<String>,
        /// read first non-empty line from file
        #[arg(long)]
        file: Option<PathBuf>,
        /// write into config/profiles.json (gitignored)
        #[arg(long)]
        save_profile: Option<String>,
        /// print ssh password in clear (default: masked)
        #[arg(long)]
        show_secrets: bool,
        #[arg(long)]
        json: bool,
    },
    /// web panel (localhost)
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787", env = "DNS_CLI_BIND")]
        bind: String,
    },
    Status,
    /// shell completion
    Completion {
        #[arg(value_enum)]
        shell: ShellKind,
    },
    /// build / path info
    Info,
    /// interactive MENU (recommended for beginners)
    Menu,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum ShellKind {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

#[derive(Subcommand, Debug)]
enum ResolversCmd {
    Sync {
        #[arg(long)]
        from_json: Option<PathBuf>,
        #[arg(long)]
        from_txt: Option<PathBuf>,
        #[arg(long, default_value = "resolvers.json")]
        out: PathBuf,
        #[arg(long)]
        limit: Option<usize>,
    },
    Normalize {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Sort {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Take {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long)]
        n: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Shuffle {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Merge {
        #[arg(long, required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, default_value = "resolvers.json")]
        out: PathBuf,
    },
    /// حذف IPهای فایل exclude از لیست
    Exclude {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long)]
        exclude: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// export one-IP-per-line (SlipNet / MasterDnsVPN client_resolvers.txt)
    ExportTxt {
        #[arg(long, default_value = "resolvers.json")]
        input: PathBuf,
        #[arg(long, default_value = "client_resolvers.txt")]
        out: PathBuf,
    },
}

#[derive(Debug, Clone, Default)]
struct GenCliOpts {
    limit: Option<usize>,
    no_dmvpn: bool,
    shuffle: bool,
    ns: Option<String>,
    pubkey: Option<String>,
    remark: Option<String>,
}

impl GenCliOpts {
    fn to_opts(&self) -> generate::GenOpts {
        generate::GenOpts {
            limit: self.limit,
            no_dmvpn: self.no_dmvpn,
            shuffle: self.shuffle,
            ns: self.ns.clone(),
            pubkey: self.pubkey.clone(),
            remark: self.remark.clone(),
        }
    }
}

#[derive(Subcommand, Debug)]
enum GenerateCmd {
    Netmod {
        #[arg(long, default_value = "demo")]
        profile: String,
        #[arg(long, default_value = "resolvers.json")]
        resolvers: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        shuffle: bool,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        ns: Option<String>,
        #[arg(long)]
        pubkey: Option<String>,
        #[arg(long)]
        remark: Option<String>,
    },
    Dnstt {
        #[arg(long, default_value = "demo")]
        profile: String,
        #[arg(long, default_value = "resolvers.json")]
        resolvers: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long, default_value = "both")]
        mode: String,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        no_dmvpn: bool,
        #[arg(long)]
        ns: Option<String>,
        #[arg(long)]
        pubkey: Option<String>,
        #[arg(long)]
        remark: Option<String>,
    },
    SlipnetUri {
        #[arg(long, default_value = "demo")]
        profile: String,
        #[arg(long, default_value = "resolvers.json")]
        resolvers: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        ns: Option<String>,
        #[arg(long)]
        pubkey: Option<String>,
        #[arg(long)]
        remark: Option<String>,
    },
    All {
        #[arg(long, default_value = "demo")]
        profile: String,
        #[arg(long, default_value = "resolvers.json")]
        resolvers: PathBuf,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        no_dmvpn: bool,
        #[arg(long)]
        ns: Option<String>,
        #[arg(long)]
        pubkey: Option<String>,
        #[arg(long)]
        remark: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum PipelineCmd {
    Run(pipeline::PipelineArgs),
}

#[derive(Subcommand, Debug)]
enum SlipnetCmd {
    Which {
        #[arg(long)]
        path: Option<PathBuf>,
    },
    Fetch {
        #[arg(long, default_value = "v2.5.3")]
        tag: String,
        #[arg(long)]
        force: bool,
    },
    Probe {
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum ArchiveCmd {
    Pack {
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value_t = 15)]
        keep: usize,
        #[arg(long)]
        allow_delete_after_archive: bool,
    },
    Restore {
        #[arg(long)]
        run_id: String,
    },
    List,
}

#[derive(Subcommand, Debug)]
enum BackupCmd {
    /// ساخت بکاپ zip
    Create {
        #[arg(long, default_value = "kit")]
        mode: String,
        #[arg(long, default_value_t = 20)]
        keep: usize,
        #[arg(long)]
        include_secrets: bool,
        #[arg(long)]
        include_runs: bool,
        #[arg(long)]
        include_vendor: bool,
        #[arg(long)]
        label: Option<String>,
    },
    List,
    Restore {
        /// نام فایل در backups/ یا مسیر کامل zip
        zip: String,
        #[arg(long)]
        force: bool,
    },
    Prune {
        #[arg(long, default_value_t = 20)]
        keep: usize,
    },
    /// بکاپ دوره‌ای (Ctrl+C)
    Watch {
        #[arg(long, default_value = "kit")]
        mode: String,
        #[arg(long, default_value_t = 3600)]
        interval: u64,
        #[arg(long, default_value_t = 20)]
        keep: usize,
        #[arg(long)]
        include_secrets: bool,
        #[arg(long)]
        include_runs: bool,
        #[arg(long)]
        include_vendor: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ProfilesCmd {
    List,
    Show { name: String },
}

fn main() -> ExitCode {
    enable_utf8_console();
    // اول .env کنار cwd را بخوان تا DNS_CLI_WORK_DIR برای clap در دسترس باشد
    let _ = env_file::load_dotenv_files(Path::new("."));

    let cli = Cli::parse();
    let (work_dir, work_note) = workdir::resolve(cli.work_dir.clone());
    if let Some(note) = work_note {
        eprintln!("ℹ️  {note}");
        eprintln!("   work_dir={}", work_dir.display());
    }

    // سپس .env داخل work_dir (اگر جداست)
    let loaded = env_file::load_dotenv_files(&work_dir);
    if std::env::var_os("DNS_CLI_ENV_DEBUG").is_some() {
        for p in &loaded {
            eprintln!("ℹ️  dotenv loaded: {}", p.display());
        }
    }

    let result = match cli.command {
        None => starter_flow(&work_dir),
        Some(Commands::Init { force_profiles }) => init_cmd::run(&work_dir, force_profiles),
        Some(Commands::Scan(args)) => scan_cmd::run(&work_dir, args),
        Some(Commands::Resolvers { action }) => match action {
            ResolversCmd::Sync {
                from_json,
                from_txt,
                out,
                limit,
            } => resolvers::sync(&work_dir, from_json, from_txt, out, limit),
            ResolversCmd::Normalize { input, out } => {
                resolvers::normalize_cmd(&work_dir, input, out)
            }
            ResolversCmd::Sort { input, out } => resolvers::sort_cmd(&work_dir, input, out),
            ResolversCmd::Take { input, n, out } => resolvers::take_cmd(&work_dir, input, n, out),
            ResolversCmd::Shuffle { input, out } => resolvers::shuffle_cmd(&work_dir, input, out),
            ResolversCmd::Merge { inputs, out } => resolvers::merge_cmd(&work_dir, inputs, out),
            ResolversCmd::Exclude {
                input,
                exclude,
                out,
            } => resolvers::exclude_cmd(&work_dir, input, exclude, out),
            ResolversCmd::ExportTxt { input, out } => {
                resolvers::export_txt_cmd(&work_dir, input, out)
            }
        },
        Some(Commands::Generate { kind }) => match kind {
            GenerateCmd::Netmod {
                profile,
                resolvers,
                out_dir,
                shuffle,
                limit,
                ns,
                pubkey,
                remark,
            } => {
                let opts = GenCliOpts {
                    limit,
                    shuffle,
                    ns,
                    pubkey,
                    remark,
                    ..Default::default()
                };
                generate::netmod_cmd(&work_dir, &profile, resolvers, out_dir, &opts.to_opts())
            }
            GenerateCmd::Dnstt {
                profile,
                resolvers,
                out_dir,
                mode,
                limit,
                no_dmvpn,
                ns,
                pubkey,
                remark,
            } => {
                let opts = GenCliOpts {
                    limit,
                    no_dmvpn,
                    ns,
                    pubkey,
                    remark,
                    ..Default::default()
                };
                generate::dnstt_cmd(
                    &work_dir,
                    &profile,
                    resolvers,
                    out_dir,
                    &mode,
                    &opts.to_opts(),
                )
            }
            GenerateCmd::SlipnetUri {
                profile,
                resolvers,
                out_dir,
                limit,
                ns,
                pubkey,
                remark,
            } => {
                let opts = GenCliOpts {
                    limit,
                    ns,
                    pubkey,
                    remark,
                    ..Default::default()
                };
                generate::slipnet_cmd(&work_dir, &profile, resolvers, out_dir, &opts.to_opts())
            }
            GenerateCmd::All {
                profile,
                resolvers,
                out_dir,
                limit,
                no_dmvpn,
                ns,
                pubkey,
                remark,
            } => {
                let opts = GenCliOpts {
                    limit,
                    no_dmvpn,
                    shuffle: true,
                    ns,
                    pubkey,
                    remark,
                };
                generate::all_cmd(&work_dir, &profile, resolvers, out_dir, &opts.to_opts())
            }
        },
        Some(Commands::Pipeline { action }) => match action {
            PipelineCmd::Run(args) => pipeline::run(&work_dir, args),
        },
        Some(Commands::Slipnet { action }) => match action {
            SlipnetCmd::Which { path } => slipnet::which_cmd(&work_dir, path),
            SlipnetCmd::Fetch { tag, force } => slipnet::fetch_cmd(&work_dir, tag, force),
            SlipnetCmd::Probe { path } => slipnet::probe_cmd(&work_dir, path),
        },
        Some(Commands::Archive { action }) => match action {
            ArchiveCmd::Pack {
                run_id,
                keep,
                allow_delete_after_archive,
            } => archive::pack(&work_dir, &run_id, keep, allow_delete_after_archive),
            ArchiveCmd::Restore { run_id } => archive::restore(&work_dir, &run_id),
            ArchiveCmd::List => archive::list(&work_dir),
        },
        Some(Commands::Backup { action }) => match action {
            BackupCmd::Create {
                mode,
                keep,
                include_secrets,
                include_runs,
                include_vendor,
                label,
            } => match backup::BackupMode::parse(&mode) {
                Ok(mode) => backup::create(
                    &work_dir,
                    &backup::BackupOpts {
                        mode,
                        keep,
                        include_secrets,
                        include_runs,
                        include_vendor,
                        label,
                    },
                ),
                Err(e) => Err(e),
            },
            BackupCmd::List => backup::list(&work_dir),
            BackupCmd::Restore { zip, force } => backup::restore(&work_dir, &zip, force),
            BackupCmd::Prune { keep } => backup::prune(&work_dir, keep),
            BackupCmd::Watch {
                mode,
                interval,
                keep,
                include_secrets,
                include_runs,
                include_vendor,
            } => match backup::BackupMode::parse(&mode) {
                Ok(mode) => backup::watch(
                    &work_dir,
                    &backup::BackupOpts {
                        mode,
                        keep,
                        include_secrets,
                        include_runs,
                        include_vendor,
                        label: Some("watch".into()),
                    },
                    interval,
                ),
                Err(e) => Err(e),
            },
        },
        Some(Commands::Clean {
            runs_keep,
            archives_keep,
            backups_keep,
            logs,
            out,
            dry_run,
        }) => clean::run(
            &work_dir,
            &clean::CleanOpts {
                runs_keep,
                archives_keep,
                backups_keep,
                logs,
                out,
                dry_run,
            },
        ),
        Some(Commands::Profiles { action }) => match action {
            ProfilesCmd::List => profiles_list(&work_dir),
            ProfilesCmd::Show { name } => profiles_show(&work_dir, &name),
        },
        Some(Commands::Doctor { fetch_hint }) => doctor::run(&work_dir, fetch_hint),
        Some(Commands::Verify { path }) => verify::run(&work_dir, path),
        Some(Commands::Decode {
            uri,
            file,
            save_profile,
            show_secrets,
            json,
        }) => decode::run(&work_dir, uri, file, save_profile, show_secrets, json),
        Some(Commands::Serve { bind }) => web::serve(&work_dir, &bind),
        Some(Commands::Status) => status_cmd(&work_dir),
        Some(Commands::Completion { shell }) => {
            emit_completion(shell);
            Ok(())
        }
        Some(Commands::Info) => info_cmd(&work_dir),
        Some(Commands::Menu) => menu::run(&work_dir),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("❌ {e}");
            ExitCode::FAILURE
        }
    }
}

fn emit_completion(shell: ShellKind) {
    use clap_complete::{generate, shells};
    let mut cmd = Cli::command();
    let name = "dns-cli";
    match shell {
        ShellKind::Bash => generate(shells::Bash, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Zsh => generate(shells::Zsh, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Fish => generate(shells::Fish, &mut cmd, name, &mut std::io::stdout()),
        ShellKind::Powershell => {
            generate(shells::PowerShell, &mut cmd, name, &mut std::io::stdout())
        }
        ShellKind::Elvish => generate(shells::Elvish, &mut cmd, name, &mut std::io::stdout()),
    }
}

fn info_cmd(work_dir: &std::path::Path) -> Result<(), String> {
    println!("dns-cli {}", env!("CARGO_PKG_VERSION"));
    println!("work_dir={}", work_dir.display());
    println!("os={}", std::env::consts::OS);
    println!("arch={}", std::env::consts::ARCH);
    println!("sqlite={}", db::db_path(work_dir).display());
    println!("slipnet_triple={}", slipnet::platform_triple());
    match slipnet::find_slipnet(work_dir, None) {
        Ok(p) => println!("slipnet={}", p.display()),
        Err(e) => println!("slipnet=(missing) {e}"),
    }
    println!(
        "profiles={}",
        crate::config::resolve_profiles_path(work_dir).display()
    );
    let env_path = work_dir.join(".env");
    println!(
        ".env={}",
        if env_path.is_file() {
            format!("present ({})", env_path.display())
        } else {
            "missing — copy .env.example → .env (see docs/ENV.md)".to_string()
        }
    );
    println!("--- env vars (masked) ---");
    println!("{}", env_file::summarize_for_user());
    Ok(())
}

fn status_cmd(work_dir: &std::path::Path) -> Result<(), String> {
    let (runs, ok, arts) = db::stats(work_dir)?;
    println!("SQLite: {}", db::db_path(work_dir).display());
    println!("runs={runs} ok={ok} artifacts={arts}");
    for r in db::list_runs(work_dir, 10)? {
        println!(
            "  {} | {} | {} | working={} e2e={}",
            r.created_at, r.id, r.status, r.working_count, r.e2e_ok
        );
    }
    Ok(())
}

fn profiles_list(work_dir: &std::path::Path) -> Result<(), String> {
    let p = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
    let mut names: Vec<_> = p.profiles.keys().cloned().collect();
    names.sort();
    for n in names {
        let pr = &p.profiles[&n];
        println!("{n}\tns={}\tssh={}", pr.tunnel_domain, pr.include_ssh);
    }
    Ok(())
}

fn profiles_show(work_dir: &std::path::Path, name: &str) -> Result<(), String> {
    let p = config::load_profiles(work_dir).map_err(|e| e.to_string())?;
    let pr = p.get(name).map_err(|e| e.to_string())?;
    println!("name={name}");
    println!("tunnel_domain={}", pr.tunnel_domain);
    println!("a_domain={}", pr.a_domain);
    println!("pubkey_len={}", pr.pubkey.len());
    println!("remark={}", pr.remark);
    println!("ssh_user={}", pr.ssh_user);
    println!("ssh_pass_len={}", pr.ssh_pass.len());
    println!("include_ssh={}", pr.include_ssh);
    println!("dnstt_mode={}", pr.dnstt_mode);
    Ok(())
}

/// کنسول ویندوز را روی UTF-8 (65001) می‌گذارد تا فارسی/یونیکد نشکند.
fn starter_flow(work_dir: &Path) -> Result<(), String> {
    use std::io::IsTerminal;

    println!("════════════════════════════════════════");
    println!("  dnstt-kit  (dns-cli)  v{}", env!("CARGO_PKG_VERSION"));
    println!("════════════════════════════════════════");
    println!();
    println!("work_dir = {}", work_dir.display());
    if !workdir::looks_like_kit(work_dir) {
        println!();
        println!("WARNING: inja folder-e kit nist (profiles / testdata nadare).");
        println!("  1) File .exe ro bezar too folder dnstt-kit");
        println!("     YA az Release source zip ro extract kon");
        println!("  2) Bad:  .\\dnstt-kit-windows-x64.exe init");
        println!("  3) Bad:  .\\dnstt-kit-windows-x64.exe menu");
        println!();
    }
    println!("Asan-tarin rah (beginner):");
    println!("  menu     → interactive menu (recommended)");
    println!("  serve    → web panel  http://127.0.0.1:8787");
    println!("  init     → sakht folder + profiles.json");
    println!("  doctor   → health check");
    println!();
    println!("Mesal:");
    println!("  dns-cli menu");
    println!("  dns-cli serve");
    println!("  dns-cli doctor");
    println!("  dns-cli --help");
    println!();

    if std::io::stdin().is_terminal() {
        println!("Terminal interactive → menu baz mishe. Ctrl+C = exit.");
        println!();
        menu::run(work_dir)
    } else {
        Ok(())
    }
}

fn enable_utf8_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
            fn SetConsoleCP(wCodePageID: u32) -> i32;
        }
        unsafe {
            SetConsoleOutputCP(65001);
            SetConsoleCP(65001);
        }
    }
    // لینوکس/مک معمولاً UTF-8 هستند؛ فقط اطمینان از متغیرهای رایج:
    if std::env::var_os("LANG").is_none() {
        // no-op — از تغییر اجباری LANG خودداری می‌کنیم
    }
}
