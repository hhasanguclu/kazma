//! `kz schedule`: run `kz clean --yes` automatically with Task Scheduler.
//! The task runs as the current user (no admin needed) and only cleans the
//! default-selected, known-safe items.

use crate::{oplog, ps, tr, ui};

const REGISTER: &str = r#"
$a = New-ScheduledTaskAction -Execute $env:KAZMA_EXE -Argument 'clean --yes'
$at = [datetime]::ParseExact($env:KAZMA_TIME, 'HH:mm', $null)
$t = switch ($env:KAZMA_FREQ) {
  'daily'   { New-ScheduledTaskTrigger -Daily -At $at }
  'weekly'  { New-ScheduledTaskTrigger -Weekly -DaysOfWeek Sunday -At $at }
  'monthly' { New-ScheduledTaskTrigger -Weekly -WeeksInterval 4 -DaysOfWeek Sunday -At $at }
}
$s = New-ScheduledTaskSettingsSet -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Hours 1)
$p = New-ScheduledTaskPrincipal -UserId ([Security.Principal.WindowsIdentity]::GetCurrent().Name) -LogonType Interactive -RunLevel Limited
Register-ScheduledTask -TaskPath '\kazma\' -TaskName 'kazma clean' -Action $a -Trigger $t -Settings $s -Principal $p -Force | Out-Null
"#;

const STATUS: &str = r#"
$t = $null
try { $t = Get-ScheduledTask -TaskPath '\kazma\' -TaskName 'kazma clean' -ErrorAction Stop } catch {}
if ($t) {
$i = $t | Get-ScheduledTaskInfo
[pscustomobject]@{
  State = [string]$t.State
  Next  = $(if ($i.NextRunTime) { $i.NextRunTime.ToString('yyyy-MM-dd HH:mm') } else { '' })
  Last  = $(if ($i.LastRunTime -and $i.LastRunTime.Year -gt 2000) { $i.LastRunTime.ToString('yyyy-MM-dd HH:mm') } else { '' })
  Exe   = [string]$t.Actions[0].Execute
} | ConvertTo-Json -Compress
}
"#;

const REMOVE: &str = r#"
try { Unregister-ScheduledTask -TaskPath '\kazma\' -TaskName 'kazma clean' -Confirm:$false -ErrorAction Stop } catch {}
"#;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    state: String,
    next: String,
    last: String,
    exe: String,
}

fn valid_time(t: &str) -> bool {
    let Some((h, m)) = t.split_once(':') else {
        return false;
    };
    h.len() == 2
        && m.len() == 2
        && h.parse::<u8>().is_ok_and(|h| h < 24)
        && m.parse::<u8>().is_ok_and(|m| m < 60)
}

fn show_status() -> anyhow::Result<()> {
    let s: Vec<Status> = ps::powershell_json(STATUS, &[])?;
    match s.first() {
        Some(s) => {
            ui::ok(&tr!(
                "scheduled cleaning is ON ({})",
                "zamanlanmış temizlik AÇIK ({})",
                s.state
            ));
            println!(
                "  {:<10} {}",
                tr!("next run", "sonraki"),
                if s.next.is_empty() { "-" } else { &s.next }
            );
            println!(
                "  {:<10} {}",
                tr!("last run", "son"),
                if s.last.is_empty() { "-" } else { &s.last }
            );
            println!("  {:<10} {}", "exe", s.exe);
        }
        None => ui::skip(&tr!(
            "scheduled cleaning is off.  turn on: kz schedule weekly",
            "zamanlanmış temizlik kapalı.  açmak için: kz schedule weekly"
        )),
    }
    println!();
    Ok(())
}

pub fn run(freq: Option<String>, time: String) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "schedule :: automatic cleaning",
        "schedule :: otomatik temizlik"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz schedule works on Windows only.",
            "kz schedule yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    let freq = freq.unwrap_or_else(|| "status".into()).to_lowercase();
    match freq.as_str() {
        "status" => show_status(),
        "off" => {
            ps::powershell(REMOVE, &[])?;
            oplog::record("schedule", "off");
            ui::ok(&tr!(
                "scheduled cleaning turned off.",
                "zamanlanmış temizlik kapatıldı."
            ));
            Ok(())
        }
        "daily" | "weekly" | "monthly" => {
            if !valid_time(&time) {
                anyhow::bail!(
                    "{}",
                    tr!(
                        "--time must look like 11:00",
                        "--time 11:00 biçiminde olmalı"
                    )
                );
            }
            let exe = std::env::current_exe()?;
            let lower = exe.to_string_lossy().to_lowercase();
            if lower.contains("\\downloads\\") || lower.contains("\\temp\\") {
                ui::warn(&tr!(
                    "kz.exe is in a temporary place ({}). Install it first so the task keeps working:",
                    "kz.exe geçici bir yerde ({}). Görevin çalışmaya devam etmesi için önce kurun:",
                    exe.display()
                ));
                println!(
                    "  iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex\n"
                );
            }
            let out = ps::powershell(
                REGISTER,
                &[
                    ("KAZMA_EXE", &exe.to_string_lossy()),
                    ("KAZMA_FREQ", &freq),
                    ("KAZMA_TIME", &time),
                ],
            )?;
            if !out.status.success() {
                anyhow::bail!(
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("Register-ScheduledTask failed")
                        .trim()
                );
            }
            oplog::record("schedule", &format!("{freq} {time}"));
            show_status()
        }
        other => anyhow::bail!(
            "{}",
            tr!(
                "unknown: {other}. use daily | weekly | monthly | off | status",
                "bilinmeyen: {other}. daily | weekly | monthly | off | status kullanın"
            )
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert!(valid_time("11:00"));
        assert!(valid_time("23:59"));
        assert!(!valid_time("24:00"));
        assert!(!valid_time("9:00"));
        assert!(!valid_time("noon"));
    }
}
