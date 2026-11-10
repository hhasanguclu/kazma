//! `kz health`: read-only system health report. All checks run in one PowerShell
//! script; each check is isolated with try/catch so one failure never hides the rest.

use crate::{ps, tr, ui};
use serde::Deserialize;

const CHECKS: &str = r#"
$tr = $env:KAZMA_TR -eq '1'
function T([string]$en, [string]$t) { if ($tr) { $t } else { $en } }
$out = New-Object System.Collections.ArrayList
function Add([string]$sec, [string]$item, [string]$val, [string]$st) {
  [void]$out.Add([pscustomobject]@{ Section = $sec; Item = $item; Value = $val; Status = $st })
}
$S = (T 'System' 'Sistem'); $D = (T 'Disks' 'Diskler'); $P = (T 'Power' 'Güç')
$C = (T 'Stability' 'Kararlılık'); $X = (T 'Security' 'Güvenlik')

try {
  $os = Get-CimInstance Win32_OperatingSystem
  Add $S 'Windows' ("{0} ({1})" -f $os.Caption, $os.BuildNumber) 'info'
  $up = (Get-Date) - $os.LastBootUpTime
  $st = if ($up.TotalDays -gt 14) { 'warn' } else { 'ok' }
  Add $S (T 'Uptime' 'Açık kalma süresi') ("{0}d {1}h" -f [int][math]::Floor($up.TotalDays), $up.Hours) $st
  $total = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1); $free = [math]::Round($os.FreePhysicalMemory / 1MB, 1)
  $st = if ($free / $total -lt 0.1) { 'warn' } else { 'ok' }
  Add $S (T 'Memory free' 'Boş bellek') ("{0} / {1} GB" -f $free, $total) $st
} catch { Add $S 'Windows' $_.Exception.Message 'warn' }

try {
  $lic = Get-CimInstance SoftwareLicensingProduct -Filter "PartialProductKey IS NOT NULL AND Name LIKE 'Windows%'" | Select-Object -First 1
  if ($lic.LicenseStatus -eq 1) { Add $S (T 'Activation' 'Etkinleştirme') (T 'activated' 'etkin') 'ok' }
  else { Add $S (T 'Activation' 'Etkinleştirme') (T 'not activated' 'etkin değil') 'bad' }
} catch {}

try {
  $reboot = (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired') -or
            (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending')
  if ($reboot) { Add $S (T 'Pending restart' 'Bekleyen yeniden başlatma') (T 'yes, restart to finish updates' 'evet, güncellemeler için yeniden başlatın') 'warn' }
  else { Add $S (T 'Pending restart' 'Bekleyen yeniden başlatma') (T 'no' 'yok') 'ok' }
} catch {}

try {
  $hf = Get-HotFix | Where-Object { $_.InstalledOn } | Sort-Object InstalledOn -Descending | Select-Object -First 1
  if ($hf) {
    $age = ((Get-Date) - $hf.InstalledOn).Days
    $st = if ($age -gt 60) { 'bad' } elseif ($age -gt 35) { 'warn' } else { 'ok' }
    Add $S (T 'Last update' 'Son güncelleme') ("{0} ({1}, {2} {3})" -f $hf.HotFixID, $hf.InstalledOn.ToString('yyyy-MM-dd'), $age, (T 'days ago' 'gün önce')) $st
  }
} catch {}

try {
  Get-PhysicalDisk | ForEach-Object {
    $name = "{0} ({1}, {2:N0} GB)" -f $_.FriendlyName, $_.MediaType, ($_.Size / 1GB)
    $st = if ($_.HealthStatus -eq 'Healthy') { 'ok' } elseif ($_.HealthStatus -eq 'Warning') { 'warn' } else { 'bad' }
    $val = [string]$_.HealthStatus
    try {
      $r = $_ | Get-StorageReliabilityCounter -ErrorAction Stop
      if ($r.Temperature) { $val += ", {0} C" -f $r.Temperature }
      if ($r.Wear) { $val += (T ', wear {0}%' ', aşınma %{0}') -f $r.Wear; if ($r.Wear -ge 80) { $st = 'warn' } }
      if ($r.PowerOnHours) { $val += (T ', {0} h on' ', {0} saat açık') -f $r.PowerOnHours }
      if ($r.ReadErrorsUncorrected -gt 0) { $val += (T ', {0} read errors' ', {0} okuma hatası') -f $r.ReadErrorsUncorrected; $st = 'bad' }
    } catch {}
    Add $D $name $val $st
  }
} catch {}

try {
  Get-Volume | Where-Object { $_.DriveLetter -and $_.DriveType -eq 'Fixed' -and $_.Size -gt 0 } | Sort-Object DriveLetter | ForEach-Object {
    $pct = [math]::Round($_.SizeRemaining / $_.Size * 100)
    $st = if ($pct -lt 5) { 'bad' } elseif ($pct -lt 12) { 'warn' } else { 'ok' }
    Add $D ("{0}:" -f $_.DriveLetter) ("{0:N1} GB {1} ({2}%)" -f ($_.SizeRemaining / 1GB), (T 'free' 'boş'), $pct) $st
  }
} catch {}

try {
  $b = Get-CimInstance Win32_Battery -ErrorAction Stop | Select-Object -First 1
  if ($b) {
    Add $P (T 'Battery charge' 'Pil şarjı') ("{0}%" -f $b.EstimatedChargeRemaining) 'info'
    try {
      $design = (Get-CimInstance -Namespace root\wmi -ClassName BatteryStaticData -ErrorAction Stop | Select-Object -First 1).DesignedCapacity
      $full = (Get-CimInstance -Namespace root\wmi -ClassName BatteryFullChargedCapacity -ErrorAction Stop | Select-Object -First 1).FullChargedCapacity
      if ($design -gt 0 -and $full -gt 0) {
        $h = [math]::Round($full / $design * 100)
        $st = if ($h -lt 50) { 'bad' } elseif ($h -lt 75) { 'warn' } else { 'ok' }
        Add $P (T 'Battery health' 'Pil sağlığı') ((T '{0}% of original capacity' 'orijinal kapasitenin %{0}''i') -f $h) $st
      }
    } catch {}
  } else { Add $P (T 'Battery' 'Pil') (T 'none (desktop)' 'yok (masaüstü)') 'info' }
} catch {}

$since = (Get-Date).AddDays(-30)
try {
  $bsod = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; Id = 1001; ProviderName = 'Microsoft-Windows-WER-SystemErrorReporting'; StartTime = $since } -ErrorAction Stop).Count
} catch { $bsod = 0 }
$st = if ($bsod -gt 0) { 'bad' } else { 'ok' }
Add $C (T 'Blue screens (30 days)' 'Mavi ekran (30 gün)') ([string]$bsod) $st
try {
  $kp = @(Get-WinEvent -FilterHashtable @{ LogName = 'System'; Id = 41; StartTime = $since } -ErrorAction Stop).Count
} catch { $kp = 0 }
$st = if ($kp -gt 2) { 'warn' } else { 'ok' }
Add $C (T 'Unexpected shutdowns (30 days)' 'Beklenmedik kapanma (30 gün)') ([string]$kp) $st
try {
  $errs = Get-WinEvent -FilterHashtable @{ LogName = 'System'; Level = 1, 2; StartTime = (Get-Date).AddDays(-7) } -MaxEvents 2000 -ErrorAction Stop
  $n = @($errs).Count
  $st = if ($n -gt 100) { 'warn' } else { 'ok' }
  $top = ($errs | Group-Object ProviderName | Sort-Object Count -Descending | Select-Object -First 3 | ForEach-Object { "{0} x{1}" -f $_.Name, $_.Count }) -join ', '
  Add $C (T 'System errors (7 days)' 'Sistem hataları (7 gün)') ("{0}  {1}" -f $n, $top) $st
} catch { Add $C (T 'System errors (7 days)' 'Sistem hataları (7 gün)') '0' 'ok' }

try {
  $mp = Get-MpComputerStatus -ErrorAction Stop
  $st = if ($mp.RealTimeProtectionEnabled) { 'ok' } else { 'bad' }
  Add $X (T 'Defender real-time protection' 'Defender gerçek zamanlı koruma') ([string]$mp.RealTimeProtectionEnabled) $st
  $age = $mp.AntivirusSignatureAge
  $st = if ($age -gt 7) { 'warn' } else { 'ok' }
  Add $X (T 'Virus definitions age' 'Virüs tanımlarının yaşı') ("{0} {1}" -f $age, (T 'days' 'gün')) $st
} catch { Add $X 'Defender' (T 'not available (third-party antivirus?)' 'erişilemedi (başka antivirüs?)') 'info' }
try {
  $fw = Get-NetFirewallProfile -ErrorAction Stop | Where-Object { -not $_.Enabled }
  if ($fw) { Add $X 'Firewall' ((T 'off for: {0}' 'kapalı: {0}') -f (($fw.Name) -join ', ')) 'bad' }
  else { Add $X 'Firewall' (T 'on' 'açık') 'ok' }
} catch {}

ConvertTo-Json -InputObject @($out) -Compress
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Check {
    pub section: String,
    pub item: String,
    pub value: String,
    pub status: String,
}

pub fn run() -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "health :: system health report",
        "health :: sistem sağlık raporu"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz health works on Windows only.",
            "kz health yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    ps::working(&tr!(
        "checking (10-30 s)...",
        "kontrol ediliyor (10-30 sn)..."
    ));
    let checks: Vec<Check> =
        ps::powershell_json(CHECKS, &[("KAZMA_TR", if ui::is_tr() { "1" } else { "0" })])?;
    println!("{:60}", "");

    let mut section = String::new();
    for c in &checks {
        if c.section != section {
            section = c.section.clone();
            ui::section(&section);
        }
        let line = format!("{:<34} {}", ui::truncate(&c.item, 34), c.value);
        match c.status.as_str() {
            "ok" => ui::ok(&line),
            "warn" => ui::warn(&line),
            "bad" => ui::bad(&line),
            _ => ui::info(&line),
        }
    }
    let bad = checks.iter().filter(|c| c.status == "bad").count();
    let warn = checks.iter().filter(|c| c.status == "warn").count();
    println!("\n  +{}+", "-".repeat(44));
    let verdict = match (bad, warn) {
        (0, 0) => tr!("all good", "her şey yolunda"),
        (0, w) => tr!("{w} thing(s) to look at", "{w} konuya göz atın"),
        (b, w) => tr!("{b} problem(s), {w} warning(s)", "{b} sorun, {w} uyarı"),
    };
    println!("  | {:<42} |", verdict);
    println!("  +{}+\n", "-".repeat(44));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_checks() {
        let v: Vec<Check> = ps::parse_json_list(
            r#"{"Section":"Disks","Item":"C:","Value":"10 GB free","Status":"warn"}"#,
        )
        .unwrap();
        assert_eq!(v[0].status, "warn");
    }
}
