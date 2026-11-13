//! `kz drivers`: find and install driver updates from Windows Update, and list
//! installed third-party drivers by age.
//!
//! Drivers only come from Microsoft's Windows Update catalog (signed, tested for
//! the exact hardware IDs on this machine), never from third-party download sites.

use crate::{oplog, ps, sys, tr, ui};
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

pub const RESTORE_POINT: &str = r#"
Checkpoint-Computer -Description 'kazma: before driver update' -RestorePointType DEVICE_DRIVER_INSTALL
"#;

pub const INSTALLED: &str = r#"
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
        2 => ui::t("installed", "kuruldu"),
        3 => ui::t("installed with errors", "hatalarla kuruldu"),
        4 => ui::t("failed", "başarısız"),
        5 => ui::t("aborted", "iptal edildi"),
        _ => ui::t("unknown result", "bilinmeyen sonuç"),
    }
}

fn current_year() -> i32 {
    oplog::now_utc()[..4].parse().unwrap_or(2026)
}

fn age_years(date: &str) -> Option<i32> {
    date.get(..4)?
        .parse::<i32>()
        .ok()
        .map(|y| current_year() - y)
}

fn gpu_tip(drivers: &[InstalledDriver]) -> Option<&'static str> {
    let display = drivers
        .iter()
        .filter(|d| d.class.eq_ignore_ascii_case("DISPLAY"));
    for d in display {
        let p = d.provider.to_lowercase();
        if p.contains("nvidia") {
            return Some(ui::t(
                "NVIDIA GPU: newest game-ready drivers come from the NVIDIA App (nvidia.com).",
                "NVIDIA GPU: en yeni oyun sürücüleri NVIDIA App'ten gelir (nvidia.com).",
            ));
        }
        if p.contains("amd") || p.contains("advanced micro") {
            return Some(ui::t(
                "AMD GPU: newest drivers come from AMD Software: Adrenalin Edition (amd.com).",
                "AMD GPU: en yeni sürücüler AMD Software: Adrenalin Edition'dan gelir (amd.com).",
            ));
        }
        if p.contains("intel") {
            return Some(ui::t(
                "Intel GPU: newest drivers come from Intel Driver & Support Assistant (intel.com).",
                "Intel GPU: en yeni sürücüler Intel Driver & Support Assistant'tan gelir (intel.com).",
            ));
        }
    }
    None
}

fn list_installed() -> anyhow::Result<()> {
    ps::working(ui::t(
        "reading installed drivers...",
        "yüklü sürücüler okunuyor...",
    ));
    let mut list: Vec<InstalledDriver> = ps::powershell_json(INSTALLED, &[])?;
    println!("{:60}", "");
    list.sort_by(|a, b| a.date.cmp(&b.date).then(a.device.cmp(&b.device)));
    list.dedup_by(|a, b| a.device == b.device && a.version == b.version);

    ui::section(&tr!(
        "Installed third-party drivers ({}), oldest first",
        "Yüklü üçüncü taraf sürücüler ({}), en eskiden yeniye",
        list.len()
    ));
    println!(
        "  {:<10} {:<5} {:<38} {:<18} PROVIDER",
        "DATE", "AGE", "DEVICE", "VERSION"
    );
    println!("  {}", "-".repeat(96));
    for d in &list {
        let age = age_years(&d.date);
        let age_s = age.map(|a| format!("{a}y")).unwrap_or_else(|| "?".into());
        let line = format!(
            "{:<10} {:<5} {} {} {}",
            d.date,
            age_s,
            ui::pad(&ui::truncate(&d.device, 38), 38),
            ui::pad(&ui::truncate(&d.version, 18), 18),
            ui::truncate(&d.provider, 24)
        );
        if age.unwrap_or(0) >= 4 {
            ui::warn(&line)
        } else {
            println!("  {line}")
        }
    }
    println!(
        "\n  {}",
        ui::t(
            "[!!] = 4+ years old. An old date is not always a problem: many devices simply\n  never get newer drivers. Run `kz drivers` to check Windows Update for newer ones.",
            "[!!] = 4+ yaşında. Eski tarih her zaman sorun değildir: birçok cihaza hiç yeni\n  sürücü çıkmaz. Windows Update'te daha yenisi var mı diye bakmak için: kz drivers"
        )
    );
    if let Some(t) = gpu_tip(&list) {
        println!("\n  {} {t}", ui::t("tip:", "ipucu:"));
    }
    println!();
    Ok(())
}

pub fn run(installed: bool, dry: bool, yes: bool, no_restore_point: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t(
            "drivers :: dry run, nothing will be installed",
            "drivers :: deneme, hiçbir şey kurulmayacak",
        )
    } else {
        ui::t(
            "drivers :: driver updates from Windows Update",
            "drivers :: Windows Update'ten sürücü güncellemeleri",
        )
    });
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz drivers works on Windows only.",
            "kz drivers yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    if installed {
        return list_installed();
    }

    ps::working(ui::t(
        "searching Windows Update for driver updates (can take up to a minute)...",
        "Windows Update'te sürücü güncellemesi aranıyor (bir dakika sürebilir)...",
    ));
    let updates: Vec<DriverUpdate> = ps::powershell_json(&format!("{SEARCH}{SCAN}"), &[])?;
    println!("{:72}", "");
    if updates.is_empty() {
        ui::ok(ui::t(
            "No driver updates available. Your drivers match the latest Windows Update offers.",
            "Sürücü güncellemesi yok. Sürücüleriniz Windows Update'teki en son sürümlerle aynı.",
        ));
        println!(
            "  {}\n",
            ui::t(
                "(see what is installed with: kz drivers --installed)",
                "(yüklü olanları görmek için: kz drivers --installed)"
            )
        );
        return Ok(());
    }

    let admin = sys::is_admin();
    if dry || !admin {
        ui::section(&tr!(
            "{} driver update(s) available",
            "{} sürücü güncellemesi var",
            updates.len()
        ));
        for u in &updates {
            ui::ok(&format!(
                "{:<10} {:>10}  {}",
                u.date,
                ui::human(u.size.max(0) as u64),
                u.title
            ));
        }
        if !admin && !dry {
            println!();
            ui::warn(ui::t(
                "Installing drivers needs administrator rights.",
                "Sürücü kurmak için yönetici yetkisi gerekir.",
            ));
            println!(
                "  {}\n",
                ui::t(
                    "Open Terminal as administrator and run: kz drivers",
                    "Terminali yönetici olarak açıp şunu çalıştırın: kz drivers"
                )
            );
        }
        return Ok(());
    }
    if !yes && !ui::interactive() {
        ui::warn(ui::t(
            "not an interactive terminal: re-run with --yes to install, or --dry-run to preview",
            "etkileşimli terminal değil: kurmak için --yes, önizlemek için --dry-run kullanın",
        ));
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
        match ui::checklist(
            ui::t(
                "kazma drivers :: select drivers to install",
                "kazma drivers :: kurulacak sürücüleri seç",
            ),
            items,
        )? {
            Some(s) => s,
            None => {
                println!("  {}", ui::t("cancelled.", "iptal edildi."));
                return Ok(());
            }
        }
    };
    let chosen: Vec<&DriverUpdate> = updates
        .iter()
        .zip(&selection)
        .filter(|(_, on)| **on)
        .map(|(u, _)| u)
        .collect();
    if chosen.is_empty() {
        println!("  {}", ui::t("nothing selected.", "hiçbir şey seçilmedi."));
        return Ok(());
    }

    if !no_restore_point {
        ps::working(ui::t(
            "creating a system restore point...",
            "sistem geri yükleme noktası oluşturuluyor...",
        ));
        match ps::powershell(RESTORE_POINT, &[]) {
            Ok(o) if o.status.success() => ui::ok(ui::t(
                "restore point created                        ",
                "geri yükleme noktası oluşturuldu            ",
            )),
            _ => {
                ui::warn(ui::t(
                    "could not create a restore point (System Restore may be off,          ",
                    "geri yükleme noktası oluşturulamadı (Sistem Geri Yükleme kapalı olabilir  ",
                ));
                println!(
                    "{}",
                    ui::t(
                        "       or one was already made in the last 24h).",
                        "       ya da son 24 saatte zaten bir tane oluşturuldu)."
                    )
                );
                if !yes
                    && !crate::remove::ask(ui::t(
                        "Continue without a new restore point?",
                        "Yeni geri yükleme noktası olmadan devam edilsin mi?",
                    ))
                {
                    println!("  {}", ui::t("cancelled.", "iptal edildi."));
                    return Ok(());
                }
            }
        }
    }

    let ids = chosen
        .iter()
        .map(|u| u.id.as_str())
        .collect::<Vec<_>>()
        .join(",");
    ps::working(&tr!(
        "downloading and installing {} driver(s), please wait...",
        "{} sürücü indirilip kuruluyor, lütfen bekleyin...",
        chosen.len()
    ));
    let results: Vec<InstallResult> =
        ps::powershell_json(&format!("{SEARCH}{INSTALL}"), &[("KAZMA_IDS", &ids)])?;
    println!("{:72}", "");

    ui::section(ui::t("Result", "Sonuç"));
    let mut reboot = false;
    for u in &chosen {
        match results.iter().find(|r| r.id == u.id) {
            Some(r) if r.code == 2 || r.code == 3 => {
                reboot |= r.reboot;
                ui::ok(&format!("{}  ({})", u.title, result_text(r.code)));
                oplog::record("drivers", &format!("installed {}", u.title));
            }
            Some(r) => {
                ui::warn(&format!(
                    "{}  ({}, {:#x})",
                    u.title,
                    result_text(r.code),
                    r.h_result as u32
                ));
                oplog::record("drivers", &format!("FAILED {} code={}", u.title, r.code));
            }
            None => ui::warn(&tr!(
                "{}  (no longer offered)",
                "{}  (artık sunulmuyor)",
                u.title
            )),
        }
    }
    if reboot {
        println!(
            "\n  ** {} **",
            ui::t(
                "restart Windows to finish installing the drivers",
                "sürücü kurulumunu bitirmek için Windows'u yeniden başlatın"
            )
        );
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
        assert!(
            ps::parse_json_list::<DriverUpdate>("  ")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn age() {
        assert_eq!(age_years(&format!("{}-01-01", current_year() - 5)), Some(5));
        assert_eq!(age_years(""), None);
    }
}
