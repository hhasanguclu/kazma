//! `kz net`: network info, latency, DNS benchmark / switch, speed test, saved Wi-Fi.

use crate::{oplog, ps, sys, tr, ui};
use serde::Deserialize;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// (key, name, servers)
pub const DNS: &[(&str, &str, &str)] = &[
    ("cloudflare", "Cloudflare", "1.1.1.1,1.0.0.1"),
    ("google", "Google", "8.8.8.8,8.8.4.4"),
    ("quad9", "Quad9", "9.9.9.9,149.112.112.112"),
    (
        "adguard",
        "AdGuard (blocks ads)",
        "94.140.14.14,94.140.15.15",
    ),
    ("opendns", "OpenDNS", "208.67.222.222,208.67.220.220"),
];

const INFO: &str = r#"
@(Get-NetIPConfiguration | Where-Object { $_.NetAdapter.Status -eq 'Up' -and $_.IPv4DefaultGateway } | ForEach-Object {
  $dns = (Get-DnsClientServerAddress -InterfaceIndex $_.InterfaceIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses
  [pscustomobject]@{
    Name    = [string]$_.InterfaceAlias
    Desc    = [string]$_.NetAdapter.InterfaceDescription
    Ip      = [string](@($_.IPv4Address)[0].IPAddress)
    Gateway = [string](@($_.IPv4DefaultGateway)[0].NextHop)
    Dns     = (@($dns) -join ', ')
    Speed   = [string]$_.NetAdapter.LinkSpeed
  }
}) | ConvertTo-Json -Compress
"#;

const PUBLIC_IP: &str = r#"
try { (Invoke-RestMethod -UseBasicParsing -TimeoutSec 6 -Uri 'https://api.ipify.org').ToString() } catch { '' }
"#;

const DNS_BENCH: &str = r#"
$domains = 'google.com','github.com','wikipedia.org','microsoft.com','cloudflare.com','amazon.com','youtube.com','reddit.com'
$res = foreach ($s in ($env:KAZMA_SERVERS -split ';')) {
  $n, $ip = $s -split '='
  $times = @(); $fail = 0
  foreach ($d in $domains) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    try {
      Resolve-DnsName -Name $d -Server $ip -Type A -DnsOnly -NoHostsFile -QuickTimeout -ErrorAction Stop | Out-Null
      $times += $sw.Elapsed.TotalMilliseconds
    } catch { $fail++ }
  }
  $avg = -1
  if ($times.Count) { $avg = [math]::Round(($times | Measure-Object -Average).Average, 1) }
  [pscustomobject]@{ Name = $n; Server = $ip; Ms = $avg; Fail = $fail }
}
ConvertTo-Json -InputObject @($res) -Compress
"#;

const DNS_SET: &str = r#"
$up = Get-NetIPConfiguration | Where-Object { $_.NetAdapter.Status -eq 'Up' -and $_.IPv4DefaultGateway }
foreach ($a in $up) {
  if ($env:KAZMA_DNS -eq 'auto') { Set-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -ResetServerAddresses }
  else { Set-DnsClientServerAddress -InterfaceIndex $a.InterfaceIndex -ServerAddresses ($env:KAZMA_DNS -split ',') }
  Write-Output $a.InterfaceAlias
}
Clear-DnsClientCache
"#;

const SPEED: &str = r#"
Add-Type -AssemblyName System.Net.Http
$c = New-Object System.Net.Http.HttpClient
$c.Timeout = [TimeSpan]::FromSeconds(60)
$c.DefaultRequestHeaders.Add('User-Agent', 'kazma')
$down = [int]$env:KAZMA_DOWN; $upN = [int]$env:KAZMA_UP
[void]$c.GetByteArrayAsync('https://speed.cloudflare.com/__down?bytes=100').Result
$sw = [Diagnostics.Stopwatch]::StartNew()
$b = $c.GetByteArrayAsync("https://speed.cloudflare.com/__down?bytes=$down").Result
$dms = $sw.Elapsed.TotalMilliseconds
$payload = New-Object System.Net.Http.ByteArrayContent (,(New-Object byte[] $upN))
$sw.Restart()
$r = $c.PostAsync('https://speed.cloudflare.com/__up', $payload).Result
$ums = $sw.Elapsed.TotalMilliseconds
[pscustomobject]@{ DownBytes = $b.Length; DownMs = $dms; UpBytes = $upN; UpMs = $ums; UpOk = [bool]$r.IsSuccessStatusCode } | ConvertTo-Json -Compress
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Adapter {
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    gateway: String,
    #[serde(default)]
    dns: String,
    #[serde(default)]
    speed: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DnsResult {
    name: String,
    server: String,
    ms: f64,
    fail: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Speed {
    down_bytes: u64,
    down_ms: f64,
    up_bytes: u64,
    up_ms: f64,
    up_ok: bool,
}

/// TCP connect time to host:443, the best of `n` tries, in ms.
pub fn tcp_latency(ip: &str, n: usize) -> Option<f64> {
    let addr: SocketAddr = format!("{ip}:443").parse().ok()?;
    (0..n)
        .filter_map(|_| {
            let t = Instant::now();
            TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
            Some(t.elapsed().as_secs_f64() * 1000.0)
        })
        .min_by(f64::total_cmp)
}

pub fn mbps(bytes: u64, ms: f64) -> f64 {
    if ms <= 0.0 {
        0.0
    } else {
        bytes as f64 * 8.0 / (ms / 1000.0) / 1_000_000.0
    }
}

fn windows_only() -> bool {
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz net works on Windows only.",
            "kz net yalnızca Windows'ta çalışır.",
        ));
    }
    cfg!(windows)
}

fn info() -> anyhow::Result<()> {
    ps::working(ui::t(
        "reading network adapters...",
        "ağ bağdaştırıcıları okunuyor...",
    ));
    let adapters: Vec<Adapter> = ps::powershell_json(INFO, &[])?;
    let public = ps::text(PUBLIC_IP, &[]).unwrap_or_default();
    println!("{:60}", "");
    for a in &adapters {
        ui::section(&format!("{}  ({})", a.name, a.desc));
        println!("  {:<12} {}", "IP", a.ip);
        println!("  {:<12} {}", ui::t("gateway", "ağ geçidi"), a.gateway);
        println!("  {:<12} {}", "DNS", a.dns);
        println!("  {:<12} {}", ui::t("link speed", "bağlantı"), a.speed);
    }
    if adapters.is_empty() {
        ui::warn(ui::t(
            "no connected network adapter",
            "bağlı ağ bağdaştırıcısı yok",
        ));
    }
    ui::section(ui::t("Internet", "İnternet"));
    println!(
        "  {:<12} {}",
        ui::t("public IP", "genel IP"),
        if public.trim().is_empty() {
            "?"
        } else {
            public.trim()
        }
    );
    for (name, ip) in [("Cloudflare", "1.1.1.1"), ("Google", "8.8.8.8")] {
        match tcp_latency(ip, 3) {
            Some(ms) => println!(
                "  {:<12} {:>6.1} ms  {}",
                name,
                ms,
                ui::bar((ms / 200.0).min(1.0), 20)
            ),
            None => ui::warn(&tr!("{name}: unreachable", "{name}: ulaşılamıyor")),
        }
    }
    println!(
        "\n  {}\n",
        ui::t(
            "more: kz net dns | kz net speed | kz net wifi",
            "daha fazlası: kz net dns | kz net speed | kz net wifi"
        )
    );
    Ok(())
}

fn dns_bench() -> anyhow::Result<()> {
    let current: Vec<Adapter> = ps::powershell_json(INFO, &[]).unwrap_or_default();
    let mut servers: Vec<String> = DNS
        .iter()
        .map(|(_, n, s)| format!("{n}={}", s.split(',').next().unwrap_or("")))
        .collect();
    if let Some(cur) = current
        .first()
        .and_then(|a| a.dns.split(", ").next())
        .filter(|s| !s.is_empty())
    {
        servers.insert(0, format!("{}={cur}", ui::t("current", "şu anki")));
    }
    ps::working(ui::t(
        "testing DNS servers (8 lookups each)...",
        "DNS sunucuları deneniyor (her biri 8 sorgu)...",
    ));
    let mut res: Vec<DnsResult> =
        ps::powershell_json(DNS_BENCH, &[("KAZMA_SERVERS", &servers.join(";"))])?;
    println!("{:60}", "");
    res.sort_by(|a, b| {
        let k = |r: &DnsResult| {
            if r.ms < 0.0 {
                f64::MAX
            } else {
                r.ms + r.fail as f64 * 1000.0
            }
        };
        k(a).total_cmp(&k(b))
    });
    let worst = res
        .iter()
        .filter(|r| r.ms > 0.0)
        .map(|r| r.ms)
        .fold(1.0, f64::max);
    ui::section(ui::t(
        "DNS response time (lower is better)",
        "DNS yanıt süresi (düşük olan iyi)",
    ));
    for (i, r) in res.iter().enumerate() {
        let ms = if r.ms < 0.0 {
            "  fail".to_string()
        } else {
            format!("{:>6.1} ms", r.ms)
        };
        let fails = if r.fail > 0 {
            tr!("  {} failed", "  {} başarısız", r.fail)
        } else {
            String::new()
        };
        let line = format!(
            "{:<22} {:<16} {ms}  {}{fails}",
            r.name,
            r.server,
            ui::bar(r.ms.max(0.0) / worst, 20)
        );
        if i == 0 {
            ui::ok(&line)
        } else {
            println!("       {line}")
        }
    }
    println!(
        "\n  {}",
        ui::t(
            "switch (needs administrator):",
            "değiştir (yönetici gerekli):"
        )
    );
    println!("  kz net dns set cloudflare | google | quad9 | adguard | opendns | auto\n");
    Ok(())
}

fn dns_set(name: &str) -> anyhow::Result<()> {
    let servers = if name == "auto" {
        "auto".to_string()
    } else {
        DNS.iter()
            .find(|(k, _, _)| *k == name)
            .map(|(_, _, s)| s.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("{}", tr!("unknown DNS: {name}", "bilinmeyen DNS: {name}"))
            })?
    };
    if !sys::is_admin() {
        anyhow::bail!(
            "{}",
            ui::t(
                "changing DNS needs an elevated terminal",
                "DNS değiştirmek için yönetici terminali gerekir"
            )
        );
    }
    let out = ps::text(DNS_SET, &[("KAZMA_DNS", &servers)])?;
    for a in out.lines().filter(|l| !l.trim().is_empty()) {
        ui::ok(&tr!("{}: DNS = {servers}", "{}: DNS = {servers}", a.trim()));
    }
    oplog::record("net", &format!("dns {name} ({servers})"));
    Ok(())
}

fn speed() -> anyhow::Result<()> {
    ps::working(ui::t(
        "measuring speed with speed.cloudflare.com (about 10 s)...",
        "speed.cloudflare.com ile hız ölçülüyor (yaklaşık 10 sn)...",
    ));
    let latency = tcp_latency("1.1.1.1", 5);
    let s: Vec<Speed> = ps::powershell_json(
        SPEED,
        &[("KAZMA_DOWN", "25000000"), ("KAZMA_UP", "10000000")],
    )?;
    println!("{:70}", "");
    let Some(s) = s.first() else {
        anyhow::bail!("no result")
    };
    let (d, u) = (mbps(s.down_bytes, s.down_ms), mbps(s.up_bytes, s.up_ms));
    let top = d.max(u).max(1.0);
    ui::section(ui::t("Speed", "Hız"));
    if let Some(ms) = latency {
        println!("  {:<10} {:>8.1} ms", ui::t("ping", "gecikme"), ms);
    }
    println!(
        "  {:<10} {:>8.1} Mbps  {}",
        ui::t("download", "indirme"),
        d,
        ui::bar(d / top, 30)
    );
    if s.up_ok {
        println!(
            "  {:<10} {:>8.1} Mbps  {}",
            ui::t("upload", "yükleme"),
            u,
            ui::bar(u / top, 30)
        );
    } else {
        ui::warn(ui::t("upload test failed", "yükleme testi başarısız"));
    }
    println!();
    Ok(())
}

/// Pull `<tag>value</tag>` out of a small XML document.
pub fn xml_tag<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&format!("</{tag}>"))? + start;
    Some(&xml[start..end])
}

fn wifi(show: bool) -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("kazma-wifi-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let mut args = vec![
        "wlan".to_string(),
        "export".into(),
        "profile".into(),
        format!("folder={}", dir.display()),
    ];
    if show {
        args.push("key=clear".into());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, out) = ps::exec("netsh", &refs);
    let mut nets: Vec<(String, String, String)> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| std::fs::read_to_string(e.path()).ok())
                .filter_map(|x| {
                    let name = xml_tag(&x, "name")?.to_string();
                    let auth = xml_tag(&x, "authentication").unwrap_or("open").to_string();
                    let key = xml_tag(&x, "keyMaterial").unwrap_or("").to_string();
                    Some((name, auth, key))
                })
                .collect()
        })
        .unwrap_or_default();
    // the folder only holds the files netsh just wrote
    let _ = std::fs::remove_dir_all(&dir);
    if nets.is_empty() {
        ui::warn(&tr!(
            "no saved Wi-Fi networks ({})",
            "kayıtlı Wi-Fi ağı yok ({})",
            ps::last_line(&out)
        ));
        return Ok(());
    }
    nets.sort_by_key(|n| n.0.to_lowercase());
    ui::section(&tr!(
        "{} saved Wi-Fi networks",
        "{} kayıtlı Wi-Fi ağı",
        nets.len()
    ));
    for (name, auth, key) in &nets {
        let pw = if show {
            if key.is_empty() {
                "-".to_string()
            } else {
                key.clone()
            }
        } else {
            "********".into()
        };
        println!("  {:<32} {:<14} {pw}", ui::truncate(name, 32), auth);
    }
    if !show {
        println!(
            "\n  {}",
            ui::t(
                "show passwords: kz net wifi --show",
                "şifreleri göster: kz net wifi --show"
            )
        );
    }
    println!();
    Ok(())
}

pub fn run(
    action: Option<String>,
    arg: Option<String>,
    arg2: Option<String>,
    show: bool,
) -> anyhow::Result<()> {
    ui::print_banner(ui::t("net :: network tools", "net :: ağ araçları"));
    if !windows_only() {
        return Ok(());
    }
    match (
        action.as_deref().unwrap_or("info"),
        arg.as_deref(),
        arg2.as_deref(),
    ) {
        ("info", _, _) => info(),
        ("dns", None, _) => dns_bench(),
        ("dns", Some("set"), Some(name)) => dns_set(&name.to_lowercase()),
        ("speed", _, _) => speed(),
        ("wifi", _, _) => wifi(show),
        _ => anyhow::bail!(
            "{}",
            ui::t(
                "usage: kz net [info | dns | dns set NAME | speed | wifi [--show]]",
                "kullanım: kz net [info | dns | dns set AD | speed | wifi [--show]]"
            )
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert!((mbps(25_000_000, 2000.0) - 100.0).abs() < 0.01);
        assert_eq!(mbps(1, 0.0), 0.0);
        let x = "<WLANProfile><name>Ev Ağı</name><authentication>WPA2PSK</authentication><keyMaterial>p@ss</keyMaterial></WLANProfile>";
        assert_eq!(xml_tag(x, "name"), Some("Ev Ağı"));
        assert_eq!(xml_tag(x, "keyMaterial"), Some("p@ss"));
        assert_eq!(xml_tag(x, "missing"), None);
        assert!(DNS.iter().all(|(_, _, s)| s.split(',').count() == 2));
    }
}
