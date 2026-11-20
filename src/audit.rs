//! `kz privacy audit`: which apps used the camera, the microphone and your
//! location, and when. Windows keeps this in the registry (the same data the
//! Settings privacy pages show as "recent activity"); kazma only reads it.

use crate::{ps, tr, ui};
use serde::{Deserialize, Serialize};

const READ: &str = r#"
$out = New-Object System.Collections.ArrayList
$now = Get-Date
foreach ($cap in 'webcam', 'microphone', 'location') {
  $base = "HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\$cap"
  if (-not (Test-Path $base)) { continue }
  $keys = @(Get-ChildItem $base -ErrorAction SilentlyContinue | Where-Object { $_.PSChildName -ne 'NonPackaged' })
  $keys += @(Get-ChildItem "$base\NonPackaged" -ErrorAction SilentlyContinue)
  foreach ($k in $keys) {
    $start = [int64]$k.GetValue('LastUsedTimeStart', 0)
    $stop = [int64]$k.GetValue('LastUsedTimeStop', 0)
    if ($start -le 0) { continue }
    $when = [DateTime]::FromFileTime($start)
    [void]$out.Add([pscustomobject]@{
      Cap = $cap; Key = $k.PSChildName; Packaged = ($k.PSParentPath -notmatch 'NonPackaged$')
      When = $when.ToString('yyyy-MM-dd HH:mm'); MinutesAgo = [int64]($now - $when).TotalMinutes
      InUse = ($start -gt $stop) })
  }
}
ConvertTo-Json -InputObject @($out) -Compress
"#;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Use {
    pub cap: String,
    pub key: String,
    pub packaged: bool,
    pub when: String,
    pub minutes_ago: i64,
    pub in_use: bool,
    #[serde(default)]
    pub app: String,
}

/// A readable app name from a ConsentStore key: "Microsoft.WindowsCamera_8wekyb3d8bbwe"
/// -> "Microsoft.WindowsCamera", "C:#Program Files#Zoom#bin#Zoom.exe" -> "Zoom.exe".
pub fn app_name(key: &str, packaged: bool) -> String {
    if packaged {
        key.rsplit_once('_')
            .map(|(a, _)| a)
            .unwrap_or(key)
            .to_string()
    } else {
        key.rsplit('#').next().unwrap_or(key).to_string()
    }
}

pub fn ago(minutes: i64) -> String {
    match minutes {
        m if m < 1 => tr!("just now", "az önce"),
        m if m < 60 => tr!("{m} min ago", "{m} dk önce"),
        m if m < 60 * 48 => tr!("{} h ago", "{} sa önce", m / 60),
        m => tr!("{} days ago", "{} gün önce", m / (60 * 24)),
    }
}

pub fn run(json: bool) -> anyhow::Result<()> {
    if !json {
        ui::print_banner(&tr!(
            "privacy :: who used the camera, microphone and location",
            "privacy :: kamera, mikrofon ve konumu kim kullandı"
        ));
    }
    if !cfg!(windows) {
        anyhow::bail!(
            "{}",
            tr!(
                "kz privacy audit works on Windows only.",
                "kz privacy audit yalnızca Windows'ta çalışır."
            )
        );
    }
    let mut uses: Vec<Use> = ps::powershell_json(READ, &[])?;
    for u in &mut uses {
        u.app = app_name(&u.key, u.packaged);
    }
    uses.sort_by_key(|u| (!u.in_use, u.minutes_ago));
    if json {
        println!("{}", serde_json::to_string_pretty(&uses)?);
        return Ok(());
    }
    for (cap, en, trn) in [
        ("webcam", "Camera", "Kamera"),
        ("microphone", "Microphone", "Mikrofon"),
        ("location", "Location", "Konum"),
    ] {
        ui::section(ui::t(en, trn));
        let list: Vec<&Use> = uses.iter().filter(|u| u.cap == cap).collect();
        if list.is_empty() {
            ui::skip(&tr!("no app has used it", "hiçbir uygulama kullanmamış"));
            continue;
        }
        let nw = list
            .iter()
            .take(10)
            .map(|u| ui::width(&u.app))
            .max()
            .unwrap_or(0)
            .clamp(20, 44);
        for u in list.iter().take(10) {
            let line = format!("{} {}  {}", ui::fit(&u.app, nw), u.when, ago(u.minutes_ago));
            if u.in_use {
                ui::warn(&format!(
                    "{line}  {}",
                    tr!("(in use now)", "(şu an kullanımda)")
                ));
            } else {
                ui::ok(&line);
            }
        }
        if list.len() > 10 {
            ui::skip(&tr!(
                "... and {} more",
                "... ve {} tane daha",
                list.len() - 10
            ));
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "change which apps may use them: Settings > Privacy & security",
            "hangi uygulamaların kullanabileceğini değiştirmek için: Ayarlar > Gizlilik ve güvenlik"
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe", true),
            "Microsoft.WindowsCamera"
        );
        assert_eq!(
            app_name("C:#Program Files#Zoom#bin#Zoom.exe", false),
            "Zoom.exe"
        );
        assert_eq!(ago(0), "just now");
        assert_eq!(ago(5), "5 min ago");
        assert_eq!(ago(180), "3 h ago");
        assert_eq!(ago(60 * 24 * 3), "3 days ago");
    }
}
