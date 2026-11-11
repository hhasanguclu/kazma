//! `kz startup`: programs that start with Windows. Enables/disables them the same
//! way Task Manager does (StartupApproved registry flags), so nothing is deleted
//! and every change can be undone from here or from Task Manager.

use crate::{oplog, ps, tr, ui};
use serde::Deserialize;

pub const LIST: &str = r#"
function Approved([string]$hive, [string]$sub, [string]$name) {
  $p = "${hive}:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\$sub"
  try {
    $v = (Get-ItemProperty -Path $p -Name $name -ErrorAction Stop).$name
    return -not ($v[0] -band 1)
  } catch { return $true }
}
$out = New-Object System.Collections.ArrayList
$runs = @(
  @{ Hive = 'HKCU'; Path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'; Sub = 'Run' },
  @{ Hive = 'HKLM'; Path = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'; Sub = 'Run' },
  @{ Hive = 'HKLM'; Path = 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; Sub = 'Run32' }
)
foreach ($r in $runs) {
  $key = Get-Item -Path $r.Path -ErrorAction SilentlyContinue
  if (-not $key) { continue }
  foreach ($n in $key.GetValueNames()) {
    if (-not $n) { continue }
    [void]$out.Add([pscustomobject]@{
      Kind = 'run'; Hive = $r.Hive; Sub = $r.Sub; Name = $n; TaskPath = ''
      Command = [string]$key.GetValue($n); Enabled = (Approved $r.Hive $r.Sub $n)
    })
  }
}
$folders = @(
  @{ Hive = 'HKCU'; Path = [Environment]::GetFolderPath('Startup') },
  @{ Hive = 'HKLM'; Path = [Environment]::GetFolderPath('CommonStartup') }
)
foreach ($f in $folders) {
  if (-not $f.Path -or -not (Test-Path $f.Path)) { continue }
  Get-ChildItem -Path $f.Path -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne 'desktop.ini' } | ForEach-Object {
    [void]$out.Add([pscustomobject]@{
      Kind = 'folder'; Hive = $f.Hive; Sub = 'StartupFolder'; Name = $_.Name; TaskPath = ''
      Command = $_.FullName; Enabled = (Approved $f.Hive 'StartupFolder' $_.Name)
    })
  }
}
try {
  Get-ScheduledTask -ErrorAction Stop | Where-Object {
    $_.TaskPath -notlike '\Microsoft\*' -and
    ($_.Triggers | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' })
  } | ForEach-Object {
    $exe = ''
    if ($_.Actions) { $exe = [string]$_.Actions[0].Execute }
    [void]$out.Add([pscustomobject]@{
      Kind = 'task'; Hive = ''; Sub = ''; Name = $_.TaskName; TaskPath = $_.TaskPath
      Command = $exe; Enabled = ($_.State -ne 'Disabled')
    })
  }
} catch {}
ConvertTo-Json -InputObject @($out) -Compress
"#;

const SET: &str = r#"
$enable = $env:KAZMA_ENABLE -eq '1'
if ($env:KAZMA_KIND -eq 'task') {
  if ($enable) { Enable-ScheduledTask -TaskPath $env:KAZMA_TASKPATH -TaskName $env:KAZMA_NAME | Out-Null }
  else { Disable-ScheduledTask -TaskPath $env:KAZMA_TASKPATH -TaskName $env:KAZMA_NAME | Out-Null }
  return
}
$p = "$($env:KAZMA_HIVE):\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\$($env:KAZMA_SUB)"
if (-not (Test-Path $p)) { New-Item -Path $p -Force | Out-Null }
$bytes = New-Object byte[] 12
if ($enable) { $bytes[0] = 2 } else {
  $bytes[0] = 3
  [BitConverter]::GetBytes([DateTime]::Now.ToFileTime()).CopyTo($bytes, 4)
}
New-ItemProperty -Path $p -Name $env:KAZMA_NAME -Value $bytes -PropertyType Binary -Force | Out-Null
"#;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub kind: String,
    #[serde(default)]
    pub hive: String,
    #[serde(default)]
    pub sub: String,
    pub name: String,
    #[serde(default)]
    pub task_path: String,
    #[serde(default)]
    pub command: String,
    pub enabled: bool,
}

impl Entry {
    fn source(&self) -> String {
        match self.kind.as_str() {
            "run" => format!("{} Run", self.hive),
            "folder" => tr!("Startup folder", "Başlangıç klasörü"),
            _ => tr!("Task", "Görev"),
        }
    }
}

pub fn list() -> anyhow::Result<Vec<Entry>> {
    let mut v: Vec<Entry> = ps::powershell_json(LIST, &[])?;
    v.sort_by_key(|e| (!e.enabled, e.name.to_lowercase()));
    Ok(v)
}

fn set(e: &Entry, enable: bool) -> Result<(), String> {
    let env = [
        ("KAZMA_KIND", e.kind.as_str()),
        ("KAZMA_HIVE", e.hive.as_str()),
        ("KAZMA_SUB", e.sub.as_str()),
        ("KAZMA_NAME", e.name.as_str()),
        ("KAZMA_TASKPATH", e.task_path.as_str()),
        ("KAZMA_ENABLE", if enable { "1" } else { "0" }),
    ];
    match ps::powershell(SET, &env) {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).to_lowercase();
            if err.contains("access") || err.contains("erişim") || err.contains("denied") {
                Err(tr!("needs administrator", "yönetici gerekli"))
            } else {
                Err(String::from_utf8_lossy(&o.stderr)
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("failed")
                    .trim()
                    .to_string())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn run(list_only: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "startup :: programs that start with Windows",
        "startup :: Windows ile başlayan programlar"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz startup works on Windows only.",
            "kz startup yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    ps::working(&tr!(
        "reading startup entries...",
        "başlangıç kayıtları okunuyor..."
    ));
    let entries = list()?;
    println!("{:60}", "");
    if entries.is_empty() {
        ui::ok(&tr!(
            "Nothing starts with Windows.",
            "Windows ile başlayan program yok."
        ));
        return Ok(());
    }

    if list_only || !ui::interactive() {
        ui::section(&tr!(
            "{} startup entries",
            "{} başlangıç kaydı",
            entries.len()
        ));
        for e in &entries {
            let state = if e.enabled {
                tr!("on ", "açık")
            } else {
                tr!("off", "kapalı")
            };
            let line = format!(
                "{state:<6} {:<32} {:<18} {}",
                ui::truncate(&e.name, 32),
                e.source(),
                ui::truncate(&e.command, 60)
            );
            if e.enabled {
                ui::ok(&line)
            } else {
                ui::skip(&line)
            }
        }
        println!();
        return Ok(());
    }

    let items = entries
        .iter()
        .map(|e| ui::CheckItem {
            label: e.name.clone(),
            detail: format!("{}  {}", e.source(), e.command),
            size: 0,
            checked: e.enabled,
        })
        .collect();
    let title = tr!(
        "kazma startup :: [x] = starts with Windows",
        "kazma startup :: [x] = Windows ile başlar"
    );
    let Some(sel) = ui::checklist_opts(&title, items, false)? else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };

    let changes: Vec<(&Entry, bool)> = entries
        .iter()
        .zip(sel)
        .filter(|(e, on)| e.enabled != *on)
        .collect();
    if changes.is_empty() {
        println!("  {}", tr!("no changes.", "değişiklik yok."));
        return Ok(());
    }
    ui::section(&tr!("Applying", "Uygulanıyor"));
    for (e, on) in changes {
        let what = if on {
            tr!("enabled ", "açıldı  ")
        } else {
            tr!("disabled", "kapatıldı")
        };
        match set(e, on) {
            Ok(()) => {
                ui::ok(&format!("{what}  {}", e.name));
                oplog::record(
                    "startup",
                    &format!("{} {}", if on { "enabled" } else { "disabled" }, e.name),
                );
            }
            Err(err) => ui::warn(&format!("{}  ({err})", e.name)),
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "takes effect at next sign-in.",
            "bir sonraki oturum açışta geçerli olur."
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let json = r#"[{"Kind":"run","Hive":"HKCU","Sub":"Run","Name":"Discord","TaskPath":"","Command":"C:\\x\\Update.exe","Enabled":true},
                       {"Kind":"task","Hive":"","Sub":"","Name":"Updater","TaskPath":"\\","Command":"u.exe","Enabled":false}]"#;
        let v: Vec<Entry> = ps::parse_json_list(json).unwrap();
        assert_eq!(v.len(), 2);
        assert!(v[0].enabled && !v[1].enabled);
        assert_eq!(v[1].task_path, "\\");
    }
}
