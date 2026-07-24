//! هستهٔ اسکنر DNS برای پیدا کردن رزولور مناسب DNSTT.
//!
//! دو API اصلی:
//! - [`run_scan`]: نتایج را در حافظه جمع می‌کند (مناسب FFI / گزارش کامل روی لیست کوچک)
//! - [`run_scan_stream`]: ورودی خط‌به‌خط + ارسال نتیجه روی کانال (مناسب لیست بزرگ / RAM کم)
//!
//! بدون وابستگی به CLI؛ قابل استفاده از Rust و از طریق FFI در Java/Kotlin (اندروید).

pub mod ffi;

use chrono::Local;
use rand::Rng;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::timeout;

const TCP_PORTS: &[u16] = &[853];

static LOG_FILE: OnceLock<StdMutex<Option<File>>> = OnceLock::new();
static LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);
static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// اگر true باشد، لاگ DEBUG نه روی stdout و نه در فایل نوشته می‌شود (مناسب اسکن بزرگ / RAM کم).
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
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
    if is_quiet() {
        return;
    }
    println!("{}", msg);
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
        if !$crate::is_quiet() {
            let s = format!($($arg)*);
            $crate::log_line(&s);
        }
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
    if query.len() < 2 {
        return (false, -1.0, "query_too_short".into());
    }
    let expect_txid = u16::from_be_bytes([query[0], query[1]]);

    // connect() so only peer `addr` can deliver datagrams (drops injected/wrong-source replies).
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => return (false, -1.0, e.to_string()),
    };
    if let Err(e) = socket.connect(addr).await {
        return (false, -1.0, format!("connect_failed: {e}"));
    }

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

        if socket.send(query).await.is_err() {
            return (false, -1.0, "send_failed".into());
        }

        match timeout(dur, socket.recv(&mut buf)).await {
            Ok(Ok(n)) => {
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
                    continue;
                }

                let got_txid = u16::from_be_bytes([resp[0], resp[1]]);
                if got_txid != expect_txid {
                    log_println!(
                        "[DEBUG][DNS-TXID-MISMATCH] expect=0x{:04X} got=0x{:04X} — ignore",
                        expect_txid,
                        got_txid
                    );
                    continue;
                }

                let flags = u16::from_be_bytes([resp[2], resp[3]]);
                let qr = (flags >> 15) & 1;
                if qr != 1 {
                    log_println!("[DEBUG][DNS-PARSE] not a response (QR=0) — ignore");
                    continue;
                }
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

/// خواندن همهٔ هدف‌ها به `Vec` (فقط برای لیست‌های کوچک / تست).
pub fn load_targets(path: &Path) -> std::io::Result<Vec<Target>> {
    load_targets_limited(path, None)
}

/// مثل [`load_targets`] ولی بعد از `max` هدف معتبر متوقف می‌شود (`None` = بدون سقف).
pub fn load_targets_limited(path: &Path, max: Option<usize>) -> std::io::Result<Vec<Target>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut targets = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if let Some(t) = parse_target(&line) {
            targets.push(t);
            if max.is_some_and(|m| targets.len() >= m) {
                break;
            }
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
    /// سقف تعداد هدف معتبر از فایل (`None` = همه). خط‌به‌خط خوانده می‌شود؛ کل فایل در RAM نیست.
    pub max_targets: Option<usize>,
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
            max_targets: None,
        }
    }
}

#[derive(Clone)]
struct ScanTaskParams {
    timeout_secs: f64,
    domain: String,
    extra_domains: Vec<String>,
    a_domain: String,
    do_ping: bool,
    ping_timeout: f64,
    include_dns_only: bool,
    udp_attempts: u32,
    udp_backoff_ms: u64,
}

fn scan_task_params(config: &ScanConfig) -> ScanTaskParams {
    let domain = config.domain.clone();
    let a_domain = config
        .a_probe_domain
        .clone()
        .unwrap_or_else(|| domain.clone());
    ScanTaskParams {
        timeout_secs: config.timeout,
        domain,
        extra_domains: config.extra_domains.clone(),
        a_domain,
        do_ping: !config.no_ping,
        ping_timeout: config.ping_timeout.unwrap_or(config.timeout),
        include_dns_only: config.include_dns_only,
        udp_attempts: config.udp_attempts.clamp(1, 32),
        udp_backoff_ms: config.udp_backoff_ms.min(10_000),
    }
}

async fn scan_one_target(
    t: Target,
    p: Arc<ScanTaskParams>,
    ok_count: Arc<AtomicUsize>,
    fail_count: Arc<AtomicUsize>,
) -> ScanResult {
    log_println!(
        "[SCAN-START] {} (host={}, port={})",
        t.original,
        t.host,
        t.port
    );

    let (dns_ok, latency_dns_ms, error_dns) = check_dns_resolver(
        &t.host,
        t.port,
        p.timeout_secs,
        &p.a_domain,
        p.udp_attempts,
        p.udp_backoff_ms,
    )
    .await;

    let mut latency_ping_ms = -1.0;
    if dns_ok && p.do_ping {
        let host = t.host.clone();
        let ping_timeout = p.ping_timeout;
        latency_ping_ms = tokio::task::spawn_blocking(move || ping_host(&host, ping_timeout))
            .await
            .unwrap_or(-1.0);
    }

    let mut latency_txt_ms = -1.0;
    let mut recursive_ok = false;
    let mut txt_ok = false;
    let mut error = error_dns.clone();

    if dns_ok {
        let mut last_err = error_dns.clone();
        for d in std::iter::once(&p.domain).chain(p.extra_domains.iter()) {
            let (txt_res, txt_latency, txt_err) = check_recursive_txt(
                &t.host,
                t.port,
                p.timeout_secs,
                d,
                p.udp_attempts,
                p.udp_backoff_ms,
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

    let is_working = dns_ok && (txt_ok || p.include_dns_only);
    if is_working {
        ok_count.fetch_add(1, Ordering::Relaxed);
    } else {
        fail_count.fetch_add(1, Ordering::Relaxed);
    }

    let scan_result = ScanResult {
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

    log_println!(
        "[SCAN-END] {} (status={}, dns_ms={:.2}, txt_ms={:.2}, ping_ms={:.2})",
        scan_result.original,
        scan_result.status,
        scan_result.latency_dns_ms,
        scan_result.latency_txt_ms,
        scan_result.latency_ping_ms
    );

    scan_result
}

/// خواندن خط‌به‌خط فایل هدف و spawn با سقف in-flight = `workers` (بدون `Vec` از همهٔ handleها).
async fn spawn_targets_bounded<F, Fut>(
    config: &ScanConfig,
    workers: usize,
    mut spawn_one: F,
) -> Result<usize, String>
where
    F: FnMut(Target) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let file = File::open(&config.input_file).map_err(|e| format!("read input error: {}", e))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut scheduled = 0usize;
    let mut join_set: JoinSet<()> = JoinSet::new();
    let enable_tcp = config.enable_tcp;
    let max_targets = config.max_targets;

    loop {
        if max_targets.is_some_and(|m| scheduled >= m) {
            break;
        }
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| format!("read input error: {}", e))?;
        if n == 0 {
            break;
        }
        let Some(t) = parse_target(&line) else {
            continue;
        };
        if is_tcp_port(t.port) && !enable_tcp {
            continue;
        }

        while join_set.len() >= workers {
            let _ = join_set.join_next().await;
        }

        let fut = spawn_one(t);
        join_set.spawn(fut);
        scheduled += 1;
    }

    while join_set.join_next().await.is_some() {}

    if scheduled == 0 {
        return Err("هیچ هدف معتبری در فایل پیدا نشد.".to_string());
    }
    Ok(scheduled)
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

fn aggregate_scan_results(
    all_results: Vec<ScanResult>,
    elapsed: f64,
    include_dns_only: bool,
    ok_count: usize,
    fail_count: usize,
) -> ScanOutput {
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
        ok_and_dnsonly_ips: combined_ips,
    }
}

/// اجرای اسکن بر اساس تنظیمات؛ بدون نوشتن فایل. خروجی را برمی‌گرداند.
///
/// ورودی خط‌به‌خط خوانده می‌شود؛ ولی **همهٔ نتایج در RAM جمع می‌شوند** —
/// برای لیست‌های میلیون‌تایی از [`run_scan_stream`] + نوشتن افزایشی روی دیسک استفاده کنید.
pub async fn run_scan(config: ScanConfig) -> Result<ScanOutput, String> {
    if !is_quiet() {
        init_logger("scanner_debug");
    }

    let workers = config.workers.clamp(1, 512);
    let ok_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));
    let params = Arc::new(scan_task_params(&config));
    let include_dns_only = config.include_dns_only;

    let t0 = Instant::now();
    let (tx, mut rx) = mpsc::channel::<ScanResult>(workers.saturating_mul(2).max(32));
    let collector = tokio::spawn(async move {
        let mut all = Vec::new();
        while let Some(r) = rx.recv().await {
            all.push(r);
        }
        all
    });

    let tx_spawn = tx.clone();
    let ok_spawn = ok_count.clone();
    let fail_spawn = fail_count.clone();
    let params_spawn = params.clone();
    spawn_targets_bounded(&config, workers, move |t| {
        let tx = tx_spawn.clone();
        let ok_count = ok_spawn.clone();
        let fail_count = fail_spawn.clone();
        let params = params_spawn.clone();
        async move {
            let result = scan_one_target(t, params, ok_count, fail_count).await;
            let _ = tx.send(result).await;
        }
    })
    .await?;
    drop(tx);

    let all_results = collector.await.map_err(|e| e.to_string())?;
    let elapsed = t0.elapsed().as_secs_f64();
    Ok(aggregate_scan_results(
        all_results,
        elapsed,
        include_dns_only,
        ok_count.load(Ordering::Relaxed),
        fail_count.load(Ordering::Relaxed),
    ))
}

/// اسکن استریم‌شده: ورودی خط‌به‌خط؛ هر [`ScanResult`] روی کانال؛ سقف in-flight = workers.
///
/// بازگشت: `(ok_count, fail_count, elapsed_secs)`.
/// بعد از اتمام، `tx` drop می‌شود تا گیرنده EOF ببیند.
/// مصرف RAM تقریباً O(workers) است نه O(تعداد خطوط فایل) — به شرطی که گیرنده نتایج را در RAM جمع نکند.
pub async fn run_scan_stream(
    config: ScanConfig,
    tx: mpsc::Sender<ScanResult>,
) -> Result<(usize, usize, f64), String> {
    if !is_quiet() {
        init_logger("scanner_debug");
    }

    let workers = config.workers.clamp(1, 512);
    let ok_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));
    let params = Arc::new(scan_task_params(&config));

    let t0 = Instant::now();
    let tx_spawn = tx.clone();
    let ok_spawn = ok_count.clone();
    let fail_spawn = fail_count.clone();
    let params_spawn = params.clone();
    spawn_targets_bounded(&config, workers, move |t| {
        let tx = tx_spawn.clone();
        let ok_count = ok_spawn.clone();
        let fail_count = fail_spawn.clone();
        let params = params_spawn.clone();
        async move {
            let result = scan_one_target(t, params, ok_count, fail_count).await;
            let _ = tx.send(result).await;
        }
    })
    .await?;
    drop(tx);

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
    fn test_load_targets_limited_stops_early() {
        let dir = std::env::temp_dir();
        let path = dir.join("scanner_core_test_targets_limit.txt");
        let mut content = String::new();
        for i in 0..100 {
            content.push_str(&format!("10.0.0.{i}\n"));
        }
        std::fs::write(&path, content).unwrap();
        let targets = load_targets_limited(&path, Some(5)).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(targets.len(), 5);
        assert_eq!(targets[0].host, "10.0.0.0");
        assert_eq!(targets[4].host, "10.0.0.4");
    }

    #[test]
    fn test_format_dns_readable_nxdomain_like_log() {
        // QNAME = b.example.com (NXDOMAIN sample)
        let pkt: Vec<u8> = vec![
            0x96, 0x45, 0x81, 0x83, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x62,
            0x07, 0x65, 0x78, 0x61, 0x6d, 0x70, 0x6c, 0x65, 0x03, 0x63, 0x6f, 0x6d, 0x00, 0x00,
            0x01, 0x00, 0x01,
        ];
        let s = format_dns_readable(&pkt, "تست");
        assert!(s.contains("b.example.com"), "{}", s);
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
        assert!(c.max_targets.is_none());
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

    #[tokio::test]
    async fn test_run_scan_stream_respects_max_targets() {
        set_quiet(true);
        let dir = std::env::temp_dir();
        let path = dir.join("scanner_core_test_stream_max.txt");
        let mut content = String::new();
        // TEST-NET-1 — should fail fast / timeout, no real internet needed for count check
        for i in 0..50 {
            content.push_str(&format!("192.0.2.{i}\n"));
        }
        std::fs::write(&path, &content).unwrap();

        let config = ScanConfig {
            input_file: path.clone(),
            timeout: 0.05,
            no_ping: true,
            workers: 4,
            udp_attempts: 1,
            max_targets: Some(7),
            ..ScanConfig::default()
        };
        let (tx, mut rx) = mpsc::channel::<ScanResult>(32);
        let scan_fut = run_scan_stream(config, tx);
        let collect_fut = async {
            let mut n = 0usize;
            while rx.recv().await.is_some() {
                n += 1;
            }
            n
        };
        let (scan_res, n) = tokio::join!(scan_fut, collect_fut);
        std::fs::remove_file(&path).ok();
        set_quiet(false);
        let (ok, fail, _) = scan_res.expect("stream scan");
        assert_eq!(ok + fail, 7, "ok={ok} fail={fail}");
        assert_eq!(n, 7);
    }

    #[tokio::test]
    async fn test_run_scan_stream_large_file_line_by_line() {
        set_quiet(true);
        let dir = std::env::temp_dir();
        let path = dir.join("scanner_core_test_stream_large.txt");
        // ~20k lines — must not load whole file into Vec<Target> before scan
        let mut content = String::with_capacity(20_000 * 16);
        for i in 0..20_000 {
            content.push_str(&format!("198.51.100.{}\n", i % 250));
        }
        std::fs::write(&path, &content).unwrap();

        let config = ScanConfig {
            input_file: path.clone(),
            timeout: 0.02,
            no_ping: true,
            workers: 8,
            udp_attempts: 1,
            max_targets: Some(200),
            ..ScanConfig::default()
        };
        let (tx, mut rx) = mpsc::channel::<ScanResult>(64);
        let scan_fut = run_scan_stream(config, tx);
        let collect_fut = async {
            let mut n = 0usize;
            while rx.recv().await.is_some() {
                n += 1;
            }
            n
        };
        let (scan_res, n) = tokio::join!(scan_fut, collect_fut);
        std::fs::remove_file(&path).ok();
        set_quiet(false);
        let (ok, fail, _) = scan_res.expect("large stream");
        assert_eq!(ok + fail, 200);
        assert_eq!(n, 200);
    }
}
