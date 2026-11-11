//! `kz ports`: which program listens on which port. Optional `--kill PORT`.

use crate::{oplog, ps, tr, ui};
use serde::Deserialize;

const LIST: &str = r#"
$procs = @{}
Get-Process | ForEach-Object { $procs[[int]$_.Id] = $_.ProcessName }
$out = New-Object System.Collections.ArrayList
Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | ForEach-Object {
  [void]$out.Add([pscustomobject]@{ Proto = 'TCP'; Address = [string]$_.LocalAddress; Port = [int]$_.LocalPort; Pid = [int]$_.OwningProcess; Name = [string]$procs[[int]$_.OwningProcess] })
}
Get-NetUDPEndpoint -ErrorAction SilentlyContinue | ForEach-Object {
  [void]$out.Add([pscustomobject]@{ Proto = 'UDP'; Address = [string]$_.LocalAddress; Port = [int]$_.LocalPort; Pid = [int]$_.OwningProcess; Name = [string]$procs[[int]$_.OwningProcess] })
}
ConvertTo-Json -InputObject @($out) -Compress
"#;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Listener {
    pub proto: String,
    pub address: String,
    pub port: u16,
    pub pid: u32,
    #[serde(default)]
    pub name: String,
}

/// Keep one row per (proto, port, pid) and drop the noisy ephemeral UDP ports.
pub fn tidy(mut v: Vec<Listener>) -> Vec<Listener> {
    v.retain(|l| l.proto == "TCP" || l.port < 49152);
    v.sort_by(|a, b| (a.port, &a.proto, a.pid).cmp(&(b.port, &b.proto, b.pid)));
    v.dedup_by(|a, b| a.port == b.port && a.proto == b.proto && a.pid == b.pid);
    v
}

fn matches(l: &Listener, filter: &str) -> bool {
    filter.is_empty()
        || l.port.to_string() == filter
        || l.name.to_lowercase().contains(&filter.to_lowercase())
}

fn kill(pid: u32) -> bool {
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    );
    match sys {
        Ok(s) => s
            .process(sysinfo::Pid::from_u32(pid))
            .is_some_and(|p| p.kill()),
        Err(_) => false,
    }
}

pub fn run(filter: Option<String>, kill_port: Option<u16>) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "ports :: who is listening on which port",
        "ports :: hangi port hangi programda",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz ports works on Windows only.",
            "kz ports yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    ps::working(ui::t("reading connections...", "bağlantılar okunuyor..."));
    let list = tidy(ps::powershell_json::<Listener>(LIST, &[])?);
    println!("{:60}", "");

    if let Some(port) = kill_port {
        let owners: Vec<&Listener> = list
            .iter()
            .filter(|l| l.port == port && l.pid > 4)
            .collect();
        if owners.is_empty() {
            ui::warn(&tr!(
                "nothing is listening on port {port}",
                "{port} portunu dinleyen yok"
            ));
            return Ok(());
        }
        for l in owners {
            let q = tr!(
                "Close {} (pid {}) that uses port {port}?",
                "{port} portunu kullanan {} (pid {}) kapatılsın mı?",
                l.name,
                l.pid
            );
            if !ui::interactive() || !crate::remove::ask(&q) {
                continue;
            }
            if kill(l.pid) {
                ui::ok(&tr!(
                    "closed {} (pid {})",
                    "{} kapatıldı (pid {})",
                    l.name,
                    l.pid
                ));
                oplog::record(
                    "ports",
                    &format!("killed {} pid={} port={port}", l.name, l.pid),
                );
            } else {
                ui::warn(&tr!(
                    "could not close {} (try an elevated terminal)",
                    "{} kapatılamadı (yönetici terminalini deneyin)",
                    l.name
                ));
            }
        }
        return Ok(());
    }

    let f = filter.unwrap_or_default();
    let shown: Vec<&Listener> = list.iter().filter(|l| matches(l, &f)).collect();
    println!(
        "  {:<5} {:>6}  {:<22} {:>7}  {}",
        ui::t("PROTO", "PROTO"),
        "PORT",
        ui::t("ADDRESS", "ADRES"),
        "PID",
        ui::t("PROGRAM", "PROGRAM")
    );
    println!("  {}", "-".repeat(70));
    for l in &shown {
        let line = format!(
            "{:<5} {:>6}  {:<22} {:>7}  {}",
            l.proto,
            l.port,
            ui::truncate(&l.address, 22),
            l.pid,
            l.name
        );
        if l.address == "0.0.0.0" || l.address == "::" {
            println!("  {line}");
        } else {
            ui::skip(&line);
        }
    }
    println!(
        "\n  {}\n  {}\n",
        tr!(
            "{} listeners. dim rows only accept local connections.",
            "{} dinleyici. soluk satırlar yalnızca yerel bağlantı kabul eder.",
            shown.len()
        ),
        ui::t(
            "close the program on a port: kz ports --kill 8080",
            "bir porttaki programı kapat: kz ports --kill 8080"
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(proto: &str, port: u16, pid: u32, name: &str) -> Listener {
        Listener {
            proto: proto.into(),
            address: "0.0.0.0".into(),
            port,
            pid,
            name: name.into(),
        }
    }

    #[test]
    fn tidy_and_filter() {
        let v = tidy(vec![
            l("TCP", 8080, 10, "node"),
            l("TCP", 8080, 10, "node"),
            l("UDP", 60000, 3, "x"),
            l("TCP", 443, 5, "nginx"),
        ]);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].port, 443);
        assert!(matches(&v[1], "8080"));
        assert!(matches(&v[1], "NODE"));
        assert!(!matches(&v[1], "80"));
    }
}
