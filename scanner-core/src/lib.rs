//! هستهٔ اسکنر DNS برای پیدا کردن رزولور مناسب DNSTT.
//!
//! دو API اصلی:
//! - [`run_scan`]: همهٔ نتایج را در حافظه جمع می‌کند (مناسب FFI / گزارش کامل)
//! - [`run_scan_stream`]: هر نتیجه را از کانال می‌فرستد (مناسب RAM کم / 512MB)
//!
//! بدون وابستگی به CLI؛ قابل استفاده از Rust و از طریق FFI در Java/Kotlin (اندروید).

pub mod ffi;

use chrono::Local;
use rand::Rng;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::time::timeout;

const TCP_PORTS: &[u16] = &[853];

static LOG_FILE: OnceLock<StdMutex<Option<File>>> = OnceLock::new();
static LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);
static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// اگر true باشد، لاگ DEBUG روی stdout چاپ نمی‌شود (فایل لاگ اگر باز باشد همچنان می‌نویسد).
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

pub fn init_logger(base_name: &str) {
    let idx = LOG_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let now = Local::now();
    let ts = now.format("%Y%m%d_%H%M%S");
    let logs_dir = PathBuf::from("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_name = format!("{}_{}_{:03}.log", base_name, ts, idx);
    let log_path = logs_dir.join(log_name);

    match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => {
            let mutex = LOG_FILE.get_or_init(|| StdMutex::new(None));
            let mut guard = mutex.lock().unwrap();
            *guard = Some(f);
            let _ = guard
                .as_mut()
                .unwrap()
                .write_all(format!("# LOG FILE: {}\n", log_path.display()).as_bytes());
        }
        Err(e) => {
            println!("log init failed ({}): {}", log_path.display(), e);
        }
    }
}

fn log_line(msg: &str) {
    if !QUIET.load(Ordering::Relaxed) {
        println!("{}", msg);
    }
    let mutex = LOG_FILE.get_or_init(|| StdMutex::new(None));
    if let Ok(mut guard) = mutex.lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(msg.as_bytes());
            let _ = f.write_all(b"\n");
            let _ = f.flush();
        }
    }
}

macro_rules! log_println {
    ($($arg:tt)*) => {{
        let s = format!($($arg)*);
        log_line(&s);
    }};
}

fn is_tcp_port(port: u16) -> bool {
    TCP_PORTS.contains(&port)
}

fn build_dns_query(domain: &str) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let tx_id: u16 = rng.gen();
    let mut packet = Vec::with_capacity(64);
    packet.extend_from_slice(&tx_id.to_be_bytes());
    packet.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in domain.split('.') {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);
    packet
}

// --- ترجمهٔ خوانای DNS (برای لاگ و کنسول) ---

fn dns_rcode_explain(rcode: u16) -> &'static str {
    match rcode {
        0 => "NOERROR — بدون خطای پروتکل؛ ممکن است پاسخ خالی (NODATA) هم باشد",
        1 => "FORMERR — قالب بسته نامعتبر است",
        2 => "SERVFAIL — سرور نتوانست پاسخ بدهد",
        3 => "NXDOMAIN — این نام دامنه از نظر این رزولور وجود ندارد (یا مسدود/فیلتر)",
        4 => "NOTIMP — نوع پرسش پشتیبانی نمی‌شود",
        5 => "REFUSED — سرور از پاسخ دادن امتناع کرد",
        6 => "YXDOMAIN — نام وجود ندارد (رزرو)",
        7 => "YXRRSET — رکورد وجود دارد (رزرو)",
        8 => "NXRRSET — رکورد وجود ندارد (رزرو)",
        9 => "NOTAUTH — غیرمجاز",
        10 => "NOTZONE — خارج از زون",
        _ => "کد رزرو/ناشناخته",
    }
}

fn dns_type_explain(qtype: u16) -> &'static str {
    match qtype {
        1 => "A (آدرس IPv4)",
        2 => "NS",
        5 => "CNAME",
        6 => "SOA",
        12 => "PTR",
        15 => "MX",
        16 => "TXT (متن)",
        28 => "AAAA (آدرس IPv6)",
        33 => "SRV",
        255 => "ANY",
        _ => "سایر/نامشخص",
    }
}

fn dns_class_explain(qclass: u16) -> &'static str {
    match qclass {
        1 => "IN (اینترنت)",
        2 => "CSNET (قدیمی)",
        3 => "CHAOS",
        4 => "Hesiod",
        _ => "سایر",
    }
}

/// خواندن نام دامنهٔ DNS از بایت `pos` (با پشتیبانی pointer فشرده‌سازی).
fn read_dns_labels(packet: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut pos_after = pos;
    let mut saw_jump = false;
    let mut finished = false;
    let mut budget = 32usize;
    while budget > 0 {
        budget -= 1;
        if pos >= packet.len() {
            return None;
        }
        let b = packet[pos];
        if b == 0 {
            pos += 1;
            if !saw_jump {
                pos_after = pos;
            }
            finished = true;
            break;
        }
        if (b & 0xC0) == 0xC0 {
            if pos + 1 >= packet.len() {
                return None;
            }
            if !saw_jump {
                pos_after = pos + 2;
                saw_jump = true;
            }
            pos = (((b as usize) & 0x3F) << 8) | packet[pos + 1] as usize;
            continue;
        }
        let lab = b as usize;
        pos += 1;
        if pos + lab > packet.len() {
            return None;
        }
        if !out.is_empty() {
            out.push('.');
        }
        out.push_str(std::str::from_utf8(&packet[pos..pos + lab]).unwrap_or("?"));
        pos += lab;
    }
    if !finished {
        return None;
    }
    Some((out, pos_after))
}

fn parse_dns_question(packet: &[u8], pos: usize) -> Option<(String, u16, u16, usize)> {
    let (name, after_name) = read_dns_labels(packet, pos)?;
    if after_name + 4 > packet.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([packet[after_name], packet[after_name + 1]]);
    let qclass = u16::from_be_bytes([packet[after_name + 2], packet[after_name + 3]]);
    Some((name, qtype, qclass, after_name + 4))
}

/// یک خط توضیح فارسی + فنی برای بستهٔ DNS (پرسش یا پاسخ).
fn format_dns_readable(packet: &[u8], context: &str) -> String {
    if packet.len() < 12 {
        return format!(
            "{} | بسته خیلی کوتاه ({} بایت) — هدر DNS کامل نیست",
            context,
            packet.len()
        );
    }
    let id = u16::from_be_bytes([packet[0], packet[1]]);
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    let qr = (flags >> 15) & 1;
    let opcode = (flags >> 11) & 0x0F;
    let rd = (flags >> 8) & 1;
    let ra = (flags >> 7) & 1;
    let rcode = flags & 0x0F;
    let qd = u16::from_be_bytes([packet[4], packet[5]]);
    let an = u16::from_be_bytes([packet[6], packet[7]]);
    let ns = u16::from_be_bytes([packet[8], packet[9]]);
    let ar = u16::from_be_bytes([packet[10], packet[11]]);

    let role = if qr == 1 {
        "پاسخ سرور (QR=1)"
    } else {
        "پرسش کلاینت (QR=0)"
    };
    let opcode_s = match opcode {
        0 => "QUERY استاندارد",
        1 => "IQUERY",
        2 => "STATUS",
        _ => "سایر",
    };

    let mut s = format!(
        "{} | TXID=0x{:04X} | {} | {} | RD={} RA={} | سوال‌ها={} پاسخ‌ها={} authority={} additional={}",
        context, id, role, opcode_s, rd, ra, qd, an, ns, ar
    );

    if qr == 1 {
        s.push_str(&format!(
            " | rcode={} ({})",
            rcode,
            dns_rcode_explain(rcode)
        ));
    }

    if qd > 0 {
        if let Some((name, qtype, qclass, _)) = parse_dns_question(packet, 12) {
            s.push_str(&format!(
                " | نام_پرسش='{}' | نوع={} ({}) | کلاس={} ({})",
                name,
                qtype,
                dns_type_explain(qtype),
                qclass,
                dns_class_explain(qclass)
            ));
        } else {
            s.push_str(" | (نام پرسش قابل‌خواندن نیست — بسته ناقص یا فشرده‌سازی غیرعادی)");
        }
    }

    s
}

async fn recv_exact_tcp(
    stream: &mut TcpStream,
    n: usize,
    dur: Duration,
) -> std::io::Result<Vec<u8>> {
    let mut out = vec![0u8; n];
    let mut read = 0;
    while read < n {
        let chunk = timeout(dur, stream.read(&mut out[read..])).await??;
        if chunk == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "short read",
            ));
        }
        read += chunk;
    }
    Ok(out)
}

/// بین تلاش‌های پیاپی UDP بعد از شکست، backoff نمایی (میلی‌ثانیه) با سقف ۵s.
fn udp_retry_delay_ms(attempt: u32, backoff_base_ms: u64) -> u64 {
    if attempt <= 1 {
        return 0;
    }
    let shift = (attempt - 2).min(10);
    let mult = 1u64 << shift;
    backoff_base_ms.saturating_mul(mult).min(5000)
}

async fn check_dns_udp(
    host: &str,
    port: u16,
    query: &[u8],
    dur: Duration,
    max_attempts: u32,
    backoff_base_ms: u64,
) -> (bool, f64, String) {
    let max_attempts = max_attempts.clamp(1, 32);
    let addr: SocketAddr = match format!("{}:{}", host, port).parse() {
        Ok(a) => a,
        Err(e) => return (false, -1.0, e.to_string()),
    };

    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => return (false, -1.0, e.to_string()),
    };

    log_println!(
        "[DEBUG][UDP-SEND] {}:{} | query_len={} | query_hex={:02X?}",
        host,
        port,
        query.len(),
        query
    );
    log_println!(
        "[DEBUG][DNS-QUERY-READABLE] {}",
        format_dns_readable(query, "ترجمهٔ پرسش ارسالی")
    );

    let mut buf = [0u8; 4096];

    for attempt in 1..=max_attempts {
        let wait_ms = udp_retry_delay_ms(attempt, backoff_base_ms);
        if wait_ms > 0 {
            log_println!(
                "[DEBUG][UDP-BACKOFF] {}:{} | before_attempt={} | sleep_ms={}",
                host,
                port,
                attempt,
                wait_ms
            );
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }

        log_println!(
            "[DEBUG][UDP-ATTEMPT] {}:{} attempt={}/{}",
            host,
            port,
            attempt,
            max_attempts
        );
        let start = Instant::now();

        if socket.send_to(query, addr).await.is_err() {
            return (false, -1.0, "send_failed".into());
        }

        match timeout(dur, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                let resp = &buf[..n];

                log_println!(
                    "[DEBUG][UDP-RECV] {}:{} | bytes={} | latency={:.2}ms",
                    host,
                    port,
                    n,
                    latency
                );
                log_println!("[DEBUG][DNS-RAW] {:02X?}", resp);
                log_println!(
                    "[DEBUG][DNS-RESP-READABLE] {}",
                    format_dns_readable(resp, "ترجمهٔ پاسخ دریافتی")
                );

                if n < 12 {
                    log_println!("[DEBUG][DNS-PARSE] short packet");
                    return (false, latency, "short_packet".into());
                }

                let flags = u16::from_be_bytes([resp[2], resp[3]]);
                let rcode = flags & 0x000F;
                let an = u16::from_be_bytes([resp[6], resp[7]]);
                let ns = u16::from_be_bytes([resp[8], resp[9]]);

                log_println!(
                    "[DEBUG][DNS-HEADER] flags=0x{:04X} rcode={} answers={} authority={}",
                    flags,
                    rcode,
                    an,
                    ns
                );

                if rcode == 0 && (an > 0 || ns > 0) {
                    log_println!(
                        "[DEBUG][DNS-OK] {}:{} | answers={} | authority={}",
                        host,
                        port,
                        an,
                        ns
                    );
                    return (true, latency, String::new());
                } else {
                    log_println!(
                        "[DEBUG][DNS-FAIL] rcode={} answers={} authority={}",
                        rcode,
                        an,
                        ns
                    );
                    return (false, latency, "bad_dns_rcode".into());
                }
            }
            Ok(Err(e)) => log_println!("[DEBUG][UDP-ERR] {}", e),
            Err(_) => log_println!("[DEBUG][UDP-TIMEOUT] {}:{} attempt={}", host, port, attempt),
        }
    }

    (false, -1.0, "timeout".into())
}

async fn check_dns_tcp(host: &str, port: u16, query: &[u8], dur: Duration) -> (bool, f64, String) {
    let addr: SocketAddr = match format!("{}:{}", host, port).parse() {
        Ok(a) => a,
        Err(e) => return (false, -1.0, e.to_string()),
    };
    let mut len_buf = Vec::with_capacity(2 + query.len());
    len_buf.extend_from_slice(&(query.len() as u16).to_be_bytes());
    len_buf.extend_from_slice(query);

    let start = Instant::now();
    let mut stream = match timeout(dur, TcpStream::connect(addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return (false, -1.0, e.to_string()),
        Err(_) => return (false, -1.0, "timeout".into()),
    };
    if stream.write_all(&len_buf).await.is_err() {
        return (false, -1.0, "send failed".into());
    }
    let len_bytes = match recv_exact_tcp(&mut stream, 2, dur).await {
        Ok(b) => b,
        Err(e) => return (false, -1.0, e.to_string()),
    };
    let resp_len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
    if resp_len == 0 || resp_len > 65535 {
        return (false, -1.0, "tcp_bad_length".into());
    }
    let to_read = resp_len.min(4096);
    match recv_exact_tcp(&mut stream, to_read, dur).await {
        Ok(_) => {
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            (true, ms, String::new())
        }
        Err(e) => (false, -1.0, e.to_string()),
    }
}

async fn check_dns_resolver(
    host: &str,
    port: u16,
    timeout_secs: f64,
    test_domain: &str,
    udp_attempts: u32,
    udp_backoff_ms: u64,
) -> (bool, f64, String) {
    let query = build_dns_query(test_domain);
    let dur = Duration::from_secs_f64(timeout_secs.max(0.1));
    if is_tcp_port(port) {
        check_dns_tcp(host, port, &query, dur).await
    } else {
        check_dns_udp(host, port, &query, dur, udp_attempts, udp_backoff_ms).await
    }
}

fn ping_host(host: &str, timeout_secs: f64) -> f64 {
    let timeout_ms = (timeout_secs * 1000.0) as u32;
    let output = if cfg!(windows) {
        std::process::Command::new("ping")
            .args(["-n", "1", "-w", &timeout_ms.to_string(), host])
            .output()
    } else {
        std::process::Command::new("ping")
            .args([
                "-c",
                "1",
                "-W",
                &timeout_secs.max(1.0).ceil().to_string(),
                host,
            ])
            .output()
    };

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return -1.0,
    };

    if cfg!(windows) {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"(?i)time[=<](\d+)ms").unwrap());
        if let Some(caps) = re.captures(&output) {
            if let Some(m) = caps.get(1) {
                if let Ok(v) = m.as_str().parse() {
                    return v;
                }
            }
        }
    } else {
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"time=(\d+\.?\d*)\s*ms").unwrap());
        if let Some(caps) = re.captures(&output) {
            if let Some(m) = caps.get(1) {
                if let Ok(v) = m.as_str().parse() {
                    return v;
                }
            }
        }
    }

    -1.0
}

#[derive(Clone)]
pub struct Target {
    pub original: String,
    pub host: String,
    pub port: u16,
}

pub fn parse_target(line: &str) -> Option<Target> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"^(?:(?:udp|tcp)://)?([a-zA-Z0-9.-]+)(?::(\d+))?$").unwrap());
    let caps = re.captures(line)?;
    let host = caps.get(1)?.as_str().to_string();
    let port = caps
        .get(2)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(53);
    if ![53, 853, 5353].contains(&port) {
        return None;
    }
    Some(Target {
        original: line.to_string(),
        host,
        port,
    })
}

pub fn load_targets(path: &PathBuf) -> std::io::Result<Vec<Target>> {
    let content = std::fs::read_to_string(path)?;
    let mut targets = Vec::new();
    for line in content.lines() {
        if let Some(t) = parse_target(line) {
            targets.push(t);
        }
    }
    Ok(targets)
}

/// پارامترهای اجرای اسکن (بدون مسیر خروجی؛ برای استفاده در lib و FFI).
#[derive(Clone)]
pub struct ScanConfig {
    pub input_file: PathBuf,
    pub timeout: f64,
    /// دامنه برای مرحلهٔ اول (پرسش A). اگر `None` باشد همان `domain` استفاده می‌شود.
    pub a_probe_domain: Option<String>,
    pub domain: String,
    pub extra_domains: Vec<String>,
    pub enable_tcp: bool,
    pub include_dns_only: bool,
    pub workers: usize,
    pub no_ping: bool,
    pub ping_timeout: Option<f64>,
    /// تعداد تلاش UDP برای هر پرسش DNS روی ۵۳/۵۳۵۳ (۱…۳۲).
    pub udp_attempts: u32,
    /// مبنای backoff نمایی بین تلاش‌های UDP (ms); سقف تأخیر بین تلاش‌ها ۵s.
    pub udp_backoff_ms: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            input_file: PathBuf::new(),
            timeout: 5.0,
            a_probe_domain: None,
            domain: "cloudflare.com".to_string(),
            extra_domains: vec!["cloudflare.com".to_string(), "example.com".to_string()],
            enable_tcp: false,
            include_dns_only: true,
            workers: 64,
            no_ping: false,
            ping_timeout: None,
            udp_attempts: 2,
            udp_backoff_ms: 150,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanResult {
    pub original: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub latency_dns_ms: f64,
    pub latency_txt_ms: f64,
    pub latency_ping_ms: f64,
    pub recursive: bool,
    pub txt_ok: bool,
    pub error: String,
}

fn build_dns_query_txt(domain: &str) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let tx_id: u16 = rng.gen();
    let mut packet = Vec::with_capacity(64);
    packet.extend_from_slice(&tx_id.to_be_bytes());
    packet.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in domain.split('.') {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&[0x00, 0x10, 0x00, 0x01]);
    packet
}

async fn check_recursive_txt(
    host: &str,
    port: u16,
    timeout_secs: f64,
    domain: &str,
    udp_attempts: u32,
    udp_backoff_ms: u64,
) -> (bool, f64, String) {
    let query = build_dns_query_txt(domain);
    let dur = Duration::from_secs_f64(timeout_secs.max(0.1));
    if is_tcp_port(port) {
        check_dns_tcp(host, port, &query, dur).await
    } else {
        check_dns_udp(host, port, &query, dur, udp_attempts, udp_backoff_ms).await
    }
}

/// خروجی در حافظهٔ اسکن؛ قابل سریال به JSON برای FFI/Android.
#[derive(Debug, Serialize)]
pub struct ScanOutput {
    pub elapsed_secs: f64,
    pub total_count: usize,
    pub ok_count: usize,
    pub fail_count: usize,
    pub all_results: Vec<ScanResult>,
    pub working: Vec<ScanResult>,
    pub working_sorted_filtered: Vec<ScanResult>,
    pub working_ips: Vec<String>,
    pub working_ip_ports: Vec<String>,
    pub dns_only: Vec<ScanResult>,
    pub dns_only_ips: Vec<String>,
    pub dns_only_ip_ports: Vec<String>,
    pub ok_and_dnsonly_ips: Vec<String>,
}

/// اجرای اسکن بر اساس تنظیمات؛ بدون نوشتن فایل. خروجی را برمی‌گرداند.
pub async fn run_scan(config: ScanConfig) -> Result<ScanOutput, String> {
    init_logger("scanner_debug");

    let ping_timeout = config.ping_timeout.unwrap_or(config.timeout);
    let do_ping = !config.no_ping;

    let targets =
        load_targets(&config.input_file).map_err(|e| format!("read input error: {}", e))?;

    if targets.is_empty() {
        return Err("هیچ هدف معتبری در فایل پیدا نشد.".to_string());
    }

    let workers = config.workers.clamp(1, 512);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));
    let ok_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));

    let timeout_secs = config.timeout;
    let domain = config.domain.clone();
    let extra_domains = config.extra_domains.clone();
    let a_domain = config
        .a_probe_domain
        .clone()
        .unwrap_or_else(|| domain.clone());
    let enable_tcp = config.enable_tcp;
    let include_dns_only = config.include_dns_only;
    let udp_attempts = config.udp_attempts.clamp(1, 32);
    let udp_backoff_ms = config.udp_backoff_ms.min(10_000);

    let t0 = Instant::now();
    let mut handles = Vec::new();

    for t in targets {
        if is_tcp_port(t.port) && !enable_tcp {
            continue;
        }
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let ok_count = ok_count.clone();
        let fail_count = fail_count.clone();
        let domain = domain.clone();
        let extra_domains = extra_domains.clone();
        let a_domain = a_domain.clone();

        let h = tokio::spawn(async move {
            let _permit = permit;

            log_println!(
                "[SCAN-START] {} (host={}, port={})",
                t.original,
                t.host,
                t.port
            );

            let (dns_ok, latency_dns_ms, error_dns) = check_dns_resolver(
                &t.host,
                t.port,
                timeout_secs,
                &a_domain,
                udp_attempts,
                udp_backoff_ms,
            )
            .await;

            let mut latency_ping_ms = -1.0;
            if dns_ok && do_ping {
                let host = t.host.clone();
                latency_ping_ms =
                    tokio::task::spawn_blocking(move || ping_host(&host, ping_timeout))
                        .await
                        .unwrap_or(-1.0);
            }

            let mut latency_txt_ms = -1.0;
            let mut recursive_ok = false;
            let mut txt_ok = false;
            let mut error = error_dns.clone();

            if dns_ok {
                let mut last_err = error_dns.clone();
                for d in std::iter::once(&domain).chain(extra_domains.iter()) {
                    let (txt_res, txt_latency, txt_err) = check_recursive_txt(
                        &t.host,
                        t.port,
                        timeout_secs,
                        d,
                        udp_attempts,
                        udp_backoff_ms,
                    )
                    .await;
                    latency_txt_ms = txt_latency;
                    if txt_res {
                        txt_ok = true;
                        recursive_ok = true;
                        error = String::new();
                        break;
                    } else {
                        last_err = txt_err;
                    }
                }
                if !txt_ok {
                    error = last_err;
                }
            }

            let status = if dns_ok && txt_ok {
                "OK".to_string()
            } else if dns_ok {
                "DNS_ONLY".to_string()
            } else {
                "FAIL".to_string()
            };

            let is_working = dns_ok && (txt_ok || include_dns_only);
            if is_working {
                ok_count.fetch_add(1, Ordering::Relaxed);
            } else {
                fail_count.fetch_add(1, Ordering::Relaxed);
            }

            let scan_result = ScanResult {
                original: t.original.clone(),
                host: t.host.clone(),
                port: t.port,
                status: status.clone(),
                latency_dns_ms,
                latency_txt_ms,
                latency_ping_ms,
                recursive: recursive_ok,
                txt_ok,
                error: error.clone(),
            };

            log_println!(
                "[SCAN-END] {} (status={}, dns_ms={:.2}, txt_ms={:.2}, ping_ms={:.2})",
                scan_result.original,
                scan_result.status,
                scan_result.latency_dns_ms,
                scan_result.latency_txt_ms,
                scan_result.latency_ping_ms
            );

            scan_result
        });
        handles.push(h);
    }

    let mut all_results = Vec::new();
    for h in handles {
        if let Ok(r) = h.await {
            all_results.push(r);
        }
    }

    let elapsed = t0.elapsed().as_secs_f64();

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

    let mut combined_ips: Vec<String> = working
        .iter()
        .map(|r| r.host.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    combined_ips.sort();

    let total = ok_count.load(Ordering::Relaxed) + fail_count.load(Ordering::Relaxed);

    Ok(ScanOutput {
        elapsed_secs: elapsed,
        total_count: total,
        ok_count: ok_count.load(Ordering::Relaxed),
        fail_count: fail_count.load(Ordering::Relaxed),
        all_results,
        working,
        working_sorted_filtered: working_filtered,
        working_ips: hosts_unique,
        working_ip_ports,
        dns_only: dns_only_all,
        dns_only_ips,
        dns_only_ip_ports,
        ok_and_dnsonly_ips: combined_ips,
    })
}

/// اسکن استریم‌شده: هر [`ScanResult`] را روی کانال می‌فرستد تا مصرف RAM ثابت بماند.
///
/// بازگشت: `(ok_count, fail_count, elapsed_secs)`.
/// بعد از اتمام حلقهٔ spawn، `tx` داخل این تابع drop می‌شود تا گیرنده EOF ببیند.
pub async fn run_scan_stream(
    config: ScanConfig,
    tx: mpsc::Sender<ScanResult>,
) -> Result<(usize, usize, f64), String> {
    init_logger("scanner_debug");

    let ping_timeout = config.ping_timeout.unwrap_or(config.timeout);
    let do_ping = !config.no_ping;

    let targets =
        load_targets(&config.input_file).map_err(|e| format!("read input error: {}", e))?;

    if targets.is_empty() {
        return Err("هیچ هدف معتبری در فایل پیدا نشد.".to_string());
    }

    let workers = config.workers.clamp(1, 512);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));
    let ok_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));

    let timeout_secs = config.timeout;
    let domain = config.domain.clone();
    let extra_domains = config.extra_domains.clone();
    let a_domain = config
        .a_probe_domain
        .clone()
        .unwrap_or_else(|| domain.clone());
    let enable_tcp = config.enable_tcp;
    let include_dns_only = config.include_dns_only;
    let udp_attempts = config.udp_attempts.clamp(1, 32);
    let udp_backoff_ms = config.udp_backoff_ms.min(10_000);

    let t0 = Instant::now();
    let mut handles = Vec::new();

    for t in targets {
        if is_tcp_port(t.port) && !enable_tcp {
            continue;
        }

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let tx = tx.clone();
        let ok_count = ok_count.clone();
        let fail_count = fail_count.clone();
        let domain = domain.clone();
        let extra_domains = extra_domains.clone();
        let a_domain = a_domain.clone();

        let h = tokio::spawn(async move {
            let _permit = permit;

            let (dns_ok, latency_dns_ms, error_dns) = check_dns_resolver(
                &t.host,
                t.port,
                timeout_secs,
                &a_domain,
                udp_attempts,
                udp_backoff_ms,
            )
            .await;

            let mut latency_ping_ms = -1.0;
            if dns_ok && do_ping {
                let host = t.host.clone();
                latency_ping_ms =
                    tokio::task::spawn_blocking(move || ping_host(&host, ping_timeout))
                        .await
                        .unwrap_or(-1.0);
            }

            let mut latency_txt_ms = -1.0;
            let mut recursive_ok = false;
            let mut txt_ok = false;
            let mut error = error_dns.clone();

            if dns_ok {
                let mut last_err = error_dns.clone();
                for d in std::iter::once(&domain).chain(extra_domains.iter()) {
                    let (txt_res, txt_latency, txt_err) = check_recursive_txt(
                        &t.host,
                        t.port,
                        timeout_secs,
                        d,
                        udp_attempts,
                        udp_backoff_ms,
                    )
                    .await;
                    latency_txt_ms = txt_latency;
                    if txt_res {
                        txt_ok = true;
                        recursive_ok = true;
                        error = String::new();
                        break;
                    } else {
                        last_err = txt_err;
                    }
                }
                if !txt_ok {
                    error = last_err;
                }
            }

            let status = if dns_ok && txt_ok {
                "OK".to_string()
            } else if dns_ok {
                "DNS_ONLY".to_string()
            } else {
                "FAIL".to_string()
            };

            let is_working = dns_ok && (txt_ok || include_dns_only);
            if is_working {
                ok_count.fetch_add(1, Ordering::Relaxed);
            } else {
                fail_count.fetch_add(1, Ordering::Relaxed);
            }

            let result = ScanResult {
                original: t.original,
                host: t.host,
                port: t.port,
                status,
                latency_dns_ms,
                latency_txt_ms,
                latency_ping_ms,
                recursive: recursive_ok,
                txt_ok,
                error,
            };
            let _ = tx.send(result).await;
        });
        handles.push(h);
    }

    drop(tx);

    for h in handles {
        let _ = h.await;
    }

    let elapsed = t0.elapsed().as_secs_f64();
    Ok((
        ok_count.load(Ordering::Relaxed),
        fail_count.load(Ordering::Relaxed),
        elapsed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_empty_and_comment() {
        assert!(parse_target("").is_none());
        assert!(parse_target("   ").is_none());
        assert!(parse_target("# comment").is_none());
        assert!(parse_target("  # foo").is_none());
    }

    #[test]
    fn test_parse_target_ip_only() {
        let t = parse_target("1.2.3.4").unwrap();
        assert_eq!(t.host, "1.2.3.4");
        assert_eq!(t.port, 53);
        assert_eq!(t.original, "1.2.3.4");
    }

    #[test]
    fn test_parse_target_ip_port_53() {
        let t = parse_target("195.62.4.28:53").unwrap();
        assert_eq!(t.host, "195.62.4.28");
        assert_eq!(t.port, 53);
    }

    #[test]
    fn test_parse_target_udp_scheme() {
        let t = parse_target("udp://1.2.3.4:53").unwrap();
        assert_eq!(t.host, "1.2.3.4");
        assert_eq!(t.port, 53);
    }

    #[test]
    fn test_parse_target_tcp_853() {
        let t = parse_target("tcp://1.2.3.4:853").unwrap();
        assert_eq!(t.host, "1.2.3.4");
        assert_eq!(t.port, 853);
    }

    #[test]
    fn test_parse_target_port_5353() {
        let t = parse_target("example.resolver:5353").unwrap();
        assert_eq!(t.host, "example.resolver");
        assert_eq!(t.port, 5353);
    }

    #[test]
    fn test_parse_target_invalid_port_rejected() {
        assert!(parse_target("1.2.3.4:80").is_none());
        assert!(parse_target("1.2.3.4:443").is_none());
        assert!(parse_target("1.2.3.4:9999").is_none());
    }

    #[test]
    fn test_load_targets_from_temp_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("scanner_core_test_targets.txt");
        let content = "1.2.3.4\n# comment\n195.62.4.28:53\n  \nudp://10.0.0.1:53\n";
        std::fs::write(&path, content).unwrap();
        let targets = load_targets(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].host, "1.2.3.4");
        assert_eq!(targets[1].host, "195.62.4.28");
        assert_eq!(targets[2].host, "10.0.0.1");
    }

    #[test]
    fn test_format_dns_readable_nxdomain_like_log() {
        let pkt: Vec<u8> = vec![
            0x96, 0x45, 0x81, 0x83, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x62,
            0x08, 0x64, 0x61, 0x72, 0x6b, 0x6d, 0x6f, 0x75, 0x73, 0x02, 0x69, 0x72, 0x00, 0x00,
            0x01, 0x00, 0x01,
        ];
        let s = format_dns_readable(&pkt, "تست");
        assert!(s.contains("b.darkmous.ir"), "{}", s);
        assert!(s.contains("rcode=3"), "{}", s);
        assert!(s.contains("NXDOMAIN"), "{}", s);
    }

    #[test]
    fn test_udp_retry_delay_ms() {
        assert_eq!(udp_retry_delay_ms(1, 150), 0);
        assert_eq!(udp_retry_delay_ms(2, 150), 150);
        assert_eq!(udp_retry_delay_ms(3, 150), 300);
        assert_eq!(udp_retry_delay_ms(4, 100), 400);
        assert_eq!(udp_retry_delay_ms(32, 10_000), 5000);
    }

    #[test]
    fn test_scan_config_default() {
        let c = ScanConfig::default();
        assert_eq!(c.timeout, 5.0);
        assert_eq!(c.domain, "cloudflare.com");
        assert!(c.a_probe_domain.is_none());
        assert_eq!(c.udp_attempts, 2);
        assert_eq!(c.udp_backoff_ms, 150);
        assert_eq!(c.workers, 64);
        assert!(!c.enable_tcp);
        assert!(c.include_dns_only);
    }

    #[tokio::test]
    async fn test_run_scan_empty_targets_returns_err() {
        let dir = std::env::temp_dir();
        let path = dir.join("scanner_core_test_empty.txt");
        std::fs::write(&path, "# only comments\n\n  \n").unwrap();
        let config = ScanConfig {
            input_file: path.clone(),
            no_ping: true,
            ..ScanConfig::default()
        };
        let result = run_scan(config).await;
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("هیچ هدف معتبری"));
    }
}
