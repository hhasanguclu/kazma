//! `kz drivers`: find and install driver updates from Windows Update, and list
//! installed third-party drivers by age.
//!
//! Drivers only come from Microsoft's Windows Update catalog (signed, tested for
//! the exact hardware IDs on this machine), never from third-party download sites.

use crate::{oplog, ps, sys, ui};
use serde::Deserialize;

const SEARCH: &str = r#"
$s = New-Object -ComObject Microsoft.Update.Session
$s.ClientApplicationID = 'kazma'
$q = $s.CreateUpdateSearcher()
$q.Online = $true
try { $q.ServerSelection = 2 } catch {}   # 2 = Windows Update, even on WSUS-managed PCs
$r = $q.Search("IsInstalled=0 and Type='Driver' and IsHidden=0")
"#;

const SCAN: &str = r#"
@($r.Updates | ForEach-Object {
  [pscustomobject]@{
    Id       = $_.Identity.UpdateID
    Title    = [string]$_.Title
    Size     = [int64]$_.MaxDownloadSize
    Class    = [string]$_.DriverClass
    Provider = [string]$_.DriverProvider
    Date     = $(if ($_.DriverVerDate) { $_.DriverVerDate.ToString('yyyy-MM-dd') } else { '' })
  }
}) | ConvertTo-Json -Compress
"#;

const INSTALL: &str = r#"
$ids = $env:KAZMA_IDS -split ','
$c = New-Object -ComObject Microsoft.Update.UpdateColl
foreach ($u in $r.Updates) {
  if ($ids -contains $u.Identity.UpdateID) {
    if (-not $u.EulaAccepted) { $u.AcceptEula() }
    [void]$c.Add($u)
  }
}
if ($c.Count -eq 0) { return }
$d = $s.CreateUpdateDownloader(); $d.Updates = $c; [void]$d.Download()
$i = $s.CreateUpdateInstaller();  $i.Updates = $c; $res = $i.Install()
@(for ($k = 0; $k -lt $c.Count; $k++) {
  $ur = $res.GetUpdateResult($k)
  [pscustomobject]@{
    Id      = $c.Item($k).Identity.UpdateID
    Code    = [int]$ur.ResultCode
    HResult = [int64]$ur.HResult
    Reboot  = [bool]$ur.RebootRequired
  }
}) | ConvertTo-Json -Compress
"#;

const RESTORE_POINT: &str = r#"
Checkpoint-Computer -Description 'kazma: before driver update' -RestorePointType DEVICE_DRIVER_INSTALL
"#;

const INSTALLED: &str = r#"
@(Get-CimInstance Win32_PnPSignedDriver |
  Where-Object { $_.DeviceName -and $_.DriverProviderName -and $_.DriverProviderName -notmatch '^Microsoft' } |
  ForEach-Object {
    [pscustomobject]@{
      Device   = [string]$_.DeviceName
      Provider = [string]$_.DriverProviderName
      Version  = [string]$_.DriverVersion
      Class    = [string]$_.DeviceClass
      Date     = $(if ($_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { '' })
    }
  }) | ConvertTo-Json -Compress
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DriverUpdate {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InstallResult {
    id: String,
    code: i32,
    #[serde(default)]
    h_result: i64,
    #[serde(default)]
    reboot: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InstalledDriver {
    pub device: String,
    pub provider: String,
    pub version: String,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub date: String,
}

fn result_text(code: i32) -> &'static str {
    match code {
        2 => "installed",
        3 => "installed with errors",
        4 => "failed",
        5 => "aborted",
        _ => "unknown result",
    }
}

fn current_year() -> i32 {
    oplog::now_utc()[..4].parse().unwrap_or(2026)
}

fn age_years(date: &str) -> Option<i32> {
    date.get(..4)?.parse::<i32>().ok().map(|y| current_year() - y)
}

fn gpu_tip(drivers: &[InstalledDriver]) -> Option<&'static str> {
    let display = drivers.iter().filter(|d| d.class.eq_ignore_ascii_case("DISPLAY"));
    for d in display {
        let p = d.provider.to_lowercase();
        if p.contains("nvidia") {
            return Some("NVIDIA GPU: newest game-ready drivers come from the NVIDIA App (nvidia.com).");
        }
        if p.contains("amd") || p.contains("advanced micro") {
            return Some("AMD GPU: newest drivers come from AMD Software: Adrenalin Edition (amd.com).");
        }
        if p.contains("intel") {
            return Some("Intel GPU: newest drivers come from Intel Driver & Support Assistant (intel.com).");
        }
    }
    None
}

fn list_installed() -> anyhow::Result<()> {
    ps::working("reading installed drivers...");
    let mut list: Vec<InstalledDriver> = ps::powershell_json(INSTALLED, &[])?;
    println!("{:60}", "");
    list.sort_by(|a, b| a.date.cmp(&b.date).then(a.device.cmp(&b.device)));
    list.dedup_by(|a, b| a.device == b.device && a.version == b.version);

    ui::section(&format!("Installed third-party drivers ({}), oldest first", list.len()));
    println!("  {:<10} {:<5} {:<38} {:<18} PROVIDER", "DATE", "AGE", "DEVICE", "VERSION");
    println!("  {}", "-".repeat(96));
    for d in &list {
        let age = age_years(&d.date);
        let age_s = age.map(|a| format!("{a}y")).unwrap_or_else(|| "?".into());
        let line = format!(
            "{:<10} {:<5} {:<38} {:<18} {}",
            d.date,
            age_s,
            ui::truncate(&d.device, 38),
            ui::truncate(&d.version, 18),
            ui::truncate(&d.provider, 24)
        );
        if age.unwrap_or(0) >= 4 { ui::warn(&line) } else { println!("  {line}") }
    }
    println!("\n  [!!] = 4+ years old. An old date is not always a problem: many devices simply");
    println!("  never get newer drivers. Run `kz drivers` to check Windows Update for newer ones.");
    if let Some(t) = gpu_tip(&list) {
        println!("\n  tip: {t}");
    }
    println!();
    Ok(())
}

pub fn run(installed: bool, dry: bool, yes: bool, no_restore_point: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry { "drivers :: dry run, nothing will be installed" } else { "drivers :: driver updates from Windows Update" });
    if !cfg!(windows) {
        ui::warn("kz drivers works on Windows only.");
        return Ok(());
    }
    if installed {
        return list_installed();
    }

    ps::working("searching Windows Update for driver updates (can take up to a minute)...");
    let updates: Vec<DriverUpdate> = ps::powershell_json(&format!("{SEARCH}{SCAN}"), &[])?;
    println!("{:72}", "");
    if updates.is_empty() {
        ui::ok("No driver updates available. Your drivers match the latest Windows Update offers.");
        println!("  (see what is installed with: kz drivers --installed)\n");
        return Ok(());
    }

    let admin = sys::is_admin();
    if dry || !admin {
        ui::section(&format!("{} driver update(s) available", updates.len()));
        for u in &updates {
            ui::ok(&format!("{:<10} {:>10}  {}", u.date, ui::human(u.size.max(0) as u64), u.title));
        }
        if !admin && !dry {
            println!();
            ui::warn("Installing drivers needs administrator rights.");
            println!("  Open Terminal as administrator and run: kz drivers\n");
        }
        return Ok(());
    }
    if !yes && !ui::interactive() {
        ui::warn("not an interactive terminal: re-run with --yes to install, or --dry-run to preview");
        return Ok(());
    }

    let selection: Vec<bool> = if yes {
        vec![true; updates.len()]
    } else {
        let items = updates
            .iter()
            .map(|u| ui::CheckItem {
                label: u.title.clone(),
                detail: format!("{} {} {}", u.provider, u.class, u.date),
                size: u.size.max(0) as u64,
                checked: true,
            })
            .collect();
        match ui::checklist("kazma drivers :: select drivers to install", items)? {
            Some(s) => s,
            None => {
                println!("  cancelled.");
                return Ok(());
            }
        }
    };
    let chosen: Vec<&DriverUpdate> = updates.iter().zip(&selection).filter(|(_, on)| **on).map(|(u, _)| u).collect();
    if chosen.is_empty() {
        println!("  nothing selected.");
        return Ok(());
    }

    if !no_restore_point {
        ps::working("creating a system restore point...");
        match ps::powershell(RESTORE_POINT, &[]) {
            Ok(o) if o.status.success() => ui::ok("restore point created                        "),
            _ => {
                ui::warn("could not create a restore point (System Restore may be off,          ");
                println!("       or one was already made in the last 24h).");
                if !yes && !crate::remove::ask("Continue without a new restore point?") {
                    println!("  cancelled.");
                    return Ok(());
                }
            }
        }
    }

    let ids = chosen.iter().map(|u| u.id.as_str()).collect::<Vec<_>>().join(",");
    ps::working(&format!("downloading and installing {} driver(s), please wait...", chosen.len()));
    let results: Vec<InstallResult> = ps::powershell_json(&format!("{SEARCH}{INSTALL}"), &[("KAZMA_IDS", &ids)])?;
    println!("{:72}", "");

    ui::section("Result");
    let mut reboot = false;
    for u in &chosen {
        match results.iter().find(|r| r.id == u.id) {
            Some(r) if r.code == 2 || r.code == 3 => {
                reboot |= r.reboot;
                ui::ok(&format!("{}  ({})", u.title, result_text(r.code)));
                oplog::record("drivers", &format!("installed {}", u.title));
            }
            Some(r) => {
                ui::warn(&format!("{}  ({}, {:#x})", u.title, result_text(r.code), r.h_result as u32));
                oplog::record("drivers", &format!("FAILED {} code={}", u.title, r.code));
            }
            None => ui::warn(&format!("{}  (no longer offered)", u.title)),
        }
    }
    if reboot {
        println!("\n  ** restart Windows to finish installing the drivers **");
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powershell_json() {
        let one = r#"{"Id":"a-b","Title":"Intel - Display - 31.0.101.5590","Size":123456,"Class":"Display","Provider":"Intel","Date":"2024-06-01"}"#;
        let v: Vec<DriverUpdate> = ps::parse_json_list(one).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].size, 123456);
        let many = format!("\u{feff}[{one},{one}]\r\n");
        assert_eq!(ps::parse_json_list::<DriverUpdate>(&many).unwrap().len(), 2);
        assert!(ps::parse_json_list::<DriverUpdate>("  ").unwrap().is_empty());
    }

    #[test]
    fn age() {
        assert_eq!(age_years(&format!("{}-01-01", current_year() - 5)), Some(5));
        assert_eq!(age_years(""), None);
    }
}
