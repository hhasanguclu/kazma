//! `kz startup`: programs that start with Windows. Enables/disables them the same
//! way Task Manager does (StartupApproved registry flags), so nothing is deleted
//! and every change can be undone from here or from Task Manager.

use crate::{oplog, ps, tr, ui};
use serde::Deserialize;
use std::path::PathBuf;

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
$sh = New-Object -ComObject WScript.Shell
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
      Target = $(try { [string]$sh.CreateShortcut($_.FullName).TargetPath } catch { '' })
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
    /// For Startup-folder shortcuts: the program the shortcut points to.
    #[serde(default)]
    pub target: String,
}

/// The program an entry starts: the shortcut target, or the first path in the command.
pub fn target_path(e: &Entry) -> Option<PathBuf> {
    let raw = if e.kind == "folder" {
        e.target.trim()
    } else {
        e.command.trim()
    };
    if raw.is_empty() {
        return None;
    }
    let path = if let Some(rest) = raw.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").to_string()
    } else {
        let lower = raw.to_lowercase();
        match lower.find(".exe") {
            Some(i) => raw[..i + 4].to_string(),
            None => raw.split_whitespace().next().unwrap_or("").to_string(),
        }
    };
    let p = crate::whitelist::expand(&path);
    let s = p.to_string_lossy();
    // only full paths can be checked; bare names are found through PATH
    (s.contains(":\\") || s.starts_with("\\\\") || s.starts_with('/')).then_some(p)
}

/// The entry points at a program that no longer exists.
pub fn is_broken(e: &Entry) -> bool {
    target_path(e).is_some_and(|p| !p.exists())
}

fn backup_dir() -> Result<PathBuf, String> {
    let d = crate::oplog::data_dir()
        .ok_or("no data folder")?
        .join("backups")
        .join("startup");
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

/// Remove one broken entry, keeping a backup: Run values are exported to .reg,
/// tasks to .xml, shortcuts go to the Recycle Bin.
fn remove_entry(e: &Entry) -> Result<(), String> {
    let stamp = crate::oplog::now_utc().replace([' ', ':'], "-");
    match e.kind.as_str() {
        "run" => {
            let key = match (e.hive.as_str(), e.sub.as_str()) {
                ("HKCU", _) => r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                (_, "Run32") => r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
                _ => r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run",
            };
            let file = backup_dir()?.join(format!("{stamp}-{}-{}.reg", e.hive, e.sub));
            let (ok, out) = ps::exec("reg", &["export", key, &file.to_string_lossy(), "/y"]);
            if !ok {
                return Err(ps::last_line(&out));
            }
            ps::text(
                "Remove-ItemProperty -LiteralPath (\"Registry::\" + $env:KAZMA_KEY) -Name $env:KAZMA_NAME",
                &[("KAZMA_KEY", key), ("KAZMA_NAME", &e.name)],
            )
            .map(|_| ())
            .map_err(|x| x.to_string())
        }
        "folder" => {
            crate::fsutil::trash(std::path::Path::new(&e.command)).map_err(|x| x.to_string())
        }
        _ => {
            let file = backup_dir()?.join(format!("{stamp}-task.xml"));
            ps::text(
                "Export-ScheduledTask -TaskPath $env:KAZMA_TP -TaskName $env:KAZMA_NAME | Set-Content -LiteralPath $env:KAZMA_FILE -Encoding UTF8; Unregister-ScheduledTask -TaskPath $env:KAZMA_TP -TaskName $env:KAZMA_NAME -Confirm:$false",
                &[("KAZMA_TP", &e.task_path), ("KAZMA_NAME", &e.name), ("KAZMA_FILE", &file.to_string_lossy())],
            )
            .map(|_| ())
            .map_err(|x| x.to_string())
        }
    }
}

/// Remove every broken startup entry (with backups). Returns how many were removed.
pub fn remove_broken() -> Result<usize, String> {
    let entries = list().map_err(|e| e.to_string())?;
    let mut n = 0;
    let mut errors = vec![];
    for e in entries.iter().filter(|e| is_broken(e)) {
        match remove_entry(e) {
            Ok(()) => {
                n += 1;
                oplog::record("startup", &format!("removed broken entry {}", e.name));
            }
            Err(x) => errors.push(format!("{}: {x}", e.name)),
        }
    }
    if n == 0 && !errors.is_empty() {
        Err(errors.join("; "))
    } else {
        Ok(n)
    }
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
        let sw = ui::width(&tr!("on ", "açık"))
            .max(ui::width(&tr!("off", "kapalı")))
            .max(6);
        let srcw = entries
            .iter()
            .map(|e| ui::width(&e.source()))
            .max()
            .unwrap_or(0)
            .max(18);
        for e in &entries {
            let state = if e.enabled {
                tr!("on ", "açık")
            } else {
                tr!("off", "kapalı")
            };
            let broken = if is_broken(e) {
                ui::t("  (broken: program is gone)", "  (bozuk: program silinmiş)")
            } else {
                ""
            };
            let line = format!(
                "{} {} {} {}{broken}",
                ui::pad(&state, sw),
                ui::pad(&ui::truncate(&e.name, 32), 32),
                ui::pad(&e.source(), srcw),
                // the command gets what is left of the terminal width
                ui::truncate(
                    &e.command,
                    ui::term_width()
                        .saturating_sub(7 + sw + 1 + 32 + 1 + srcw + 1 + ui::width(broken))
                        .max(20)
                )
            );
            if !broken.is_empty() {
                ui::warn(&line)
            } else if e.enabled {
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

    fn entry(kind: &str, command: &str, target: &str) -> Entry {
        Entry {
            kind: kind.into(),
            hive: "HKCU".into(),
            sub: "Run".into(),
            name: "x".into(),
            task_path: String::new(),
            command: command.into(),
            enabled: true,
            target: target.into(),
        }
    }

    #[test]
    fn finds_the_program_in_a_command() {
        let p =
            |c: &str| target_path(&entry("run", c, "")).map(|p| p.to_string_lossy().into_owned());
        assert_eq!(
            p(r#""C:\Program Files\App\app.exe" --minimized"#).as_deref(),
            Some(r"C:\Program Files\App\app.exe")
        );
        assert_eq!(
            p(r"C:\Tools\tray.EXE -silent").as_deref(),
            Some(r"C:\Tools\tray.EXE")
        );
        assert_eq!(
            p("rundll32.exe shell32.dll,Foo"),
            None,
            "bare names resolve through PATH"
        );
        assert_eq!(p(""), None);
        // a shortcut is judged by its target, not by the .lnk file
        let lnk = entry("folder", r"C:\Users\me\Startup\x.lnk", r"Z:\gone\prog.exe");
        assert!(is_broken(&lnk));
        assert!(!is_broken(&entry("folder", "x.lnk", "")));
    }

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
