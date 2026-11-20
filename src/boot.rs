//! `kz boot`: how long Windows takes to start, and what slows it down.
//!
//! Windows measures every boot itself and writes the result to the
//! Diagnostics-Performance event log: event 100 is the boot time, 101-103 name
//! the apps, drivers and services that made a boot slower than it should be.

use crate::{ps, tr, ui};
use serde::{Deserialize, Serialize};

const READ: &str = r#"
$ev = Get-WinEvent -LogName 'Microsoft-Windows-Diagnostics-Performance/Operational' -FilterXPath '*[System[(EventID=100 or EventID=101 or EventID=102 or EventID=103)]]' -MaxEvents 400 -ErrorAction Stop
$boots = New-Object System.Collections.ArrayList
$slow = New-Object System.Collections.ArrayList
foreach ($e in $ev) {
  $d = @{}
  foreach ($x in ([xml]$e.ToXml()).Event.EventData.Data) { $d[$x.Name] = $x.'#text' }
  $days = [int]((Get-Date) - $e.TimeCreated).TotalDays
  if ($e.Id -eq 100) {
    [void]$boots.Add([pscustomobject]@{
      When = $e.TimeCreated.ToString('yyyy-MM-dd HH:mm'); DaysAgo = $days
      Total = [int64]$d['BootTime']; Main = [int64]$d['MainPathBootTime']; Post = [int64]$d['BootPostBootTime'] })
  } elseif ($days -le 60) {
    $kind = switch ($e.Id) { 101 { 'app' } 102 { 'driver' } default { 'service' } }
    $name = if ($d['FriendlyName']) { $d['FriendlyName'] } else { $d['Name'] }
    [void]$slow.Add([pscustomobject]@{ Kind = $kind; Name = [string]$name; File = [string]$d['Name']; Delay = [int64]$d['DegradationTime'] })
  }
}
ConvertTo-Json -InputObject @{ boots = @($boots | Select-Object -First 10); slow = @($slow) } -Compress -Depth 4
"#;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Boot {
    pub when: String,
    pub days_ago: i64,
    /// milliseconds
    pub total: i64,
    pub main: i64,
    pub post: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Slow {
    kind: String,
    name: String,
    #[serde(default)]
    file: String,
    delay: i64,
}

#[derive(Debug, Deserialize)]
struct Raw {
    #[serde(default)]
    boots: Vec<Boot>,
    #[serde(default)]
    slow: Vec<Slow>,
}

/// One program / driver / service that slowed boots down.
#[derive(Debug, Serialize, PartialEq)]
pub struct Culprit {
    pub kind: String,
    pub name: String,
    pub times: usize,
    /// average extra milliseconds per slow boot
    pub avg_delay: i64,
}

#[derive(Serialize)]
struct Report<'a> {
    boots: &'a [Boot],
    slowing: &'a [Culprit],
}

/// Group the slow-boot events by name, worst first.
pub fn culprits(slow: Vec<(String, String, i64)>) -> Vec<Culprit> {
    let mut map: Vec<Culprit> = vec![];
    for (kind, name, delay) in slow {
        let name = name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        match map.iter_mut().find(|c| c.kind == kind && c.name == name) {
            Some(c) => {
                c.avg_delay = (c.avg_delay * c.times as i64 + delay) / (c.times as i64 + 1);
                c.times += 1;
            }
            None => map.push(Culprit {
                kind,
                name,
                times: 1,
                avg_delay: delay,
            }),
        }
    }
    map.sort_by_key(|c| std::cmp::Reverse(c.avg_delay * c.times as i64));
    map
}

#[derive(Debug, PartialEq)]
enum ReadError {
    /// no boot recorded, or no such log on this edition (Windows Server, some VMs)
    NoBoots,
    Denied,
    Other,
}

fn read_error(stderr: &str) -> ReadError {
    let e = stderr.to_lowercase();
    if e.contains("no events")
        || e.contains("there is not an event log")
        || e.contains("could not be found")
        || e.contains("specified channel")
    {
        ReadError::NoBoots
    } else if e.contains("unauthorized") || e.contains("access is denied") {
        ReadError::Denied
    } else {
        ReadError::Other
    }
}

fn secs(ms: i64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0)
}

pub fn run(json: bool) -> anyhow::Result<()> {
    if !json {
        ui::print_banner(&tr!(
            "boot :: how long Windows takes to start",
            "boot :: Windows'un açılış süresi"
        ));
    }
    if !cfg!(windows) {
        anyhow::bail!(
            "{}",
            tr!(
                "kz boot works on Windows only.",
                "kz boot yalnızca Windows'ta çalışır."
            )
        );
    }
    if !json {
        ps::working(&tr!("reading the boot log...", "açılış kaydı okunuyor..."));
    }
    let out = ps::powershell(READ, &[])?;
    if !json {
        println!("{:60}", "");
    }
    let none = !out.status.success()
        && match read_error(&String::from_utf8_lossy(&out.stderr)) {
            ReadError::NoBoots => true,
            ReadError::Denied if !crate::sys::is_admin() => anyhow::bail!(
                "{}",
                tr!(
                    "the boot log can only be read from an elevated terminal (Run as administrator)",
                    "açılış kaydı yalnızca yönetici terminalinden okunabilir (Yönetici olarak çalıştır)"
                )
            ),
            // anything else: the first line of PowerShell's own message says what it was
            _ => anyhow::bail!(
                "{}",
                String::from_utf8_lossy(&out.stderr)
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("Get-WinEvent failed")
            ),
        };
    let raw: Raw = if none {
        Raw {
            boots: vec![],
            slow: vec![],
        }
    } else {
        serde_json::from_str(
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .trim_start_matches('\u{feff}'),
        )?
    };
    let slow = culprits(
        raw.slow
            .into_iter()
            .map(|s| {
                (
                    s.kind,
                    if s.name.is_empty() { s.file } else { s.name },
                    s.delay,
                )
            })
            .collect(),
    );
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&Report {
                boots: &raw.boots,
                slowing: &slow
            })?
        );
        return Ok(());
    }

    if raw.boots.is_empty() {
        ui::warn(&tr!(
            "Windows has not recorded any boot yet.",
            "Windows henüz bir açılış kaydetmemiş."
        ));
        return Ok(());
    }
    ui::section(&tr!("Recent boots", "Son açılışlar"));
    let max = raw.boots.iter().map(|b| b.total).max().unwrap_or(1).max(1);
    for b in &raw.boots {
        let line = format!(
            "{}  {} {:>8}   {}",
            b.when,
            ui::cbar(b.total as f64 / max as f64, 24),
            secs(b.total),
            tr!(
                "desktop ready after {}",
                "masaüstü {} sonra hazır",
                secs(b.main)
            )
        );
        // over a minute to a usable desktop is slow on any PC with an SSD
        if b.main > 60_000 {
            ui::warn(&line);
        } else {
            ui::ok(&line);
        }
    }
    let avg = raw.boots.iter().map(|b| b.main).sum::<i64>() / raw.boots.len() as i64;
    println!(
        "\n  {}",
        tr!(
            "average time to a usable desktop: {}",
            "kullanılabilir masaüstüne ortalama süre: {}",
            secs(avg)
        )
    );

    ui::section(&tr!(
        "Slowing down startup (last 60 days)",
        "Açılışı yavaşlatanlar (son 60 gün)"
    ));
    if slow.is_empty() {
        ui::ok(&tr!(
            "nothing: Windows reported no slow apps, drivers or services",
            "yok: Windows yavaş uygulama, sürücü ya da hizmet bildirmedi"
        ));
    } else {
        let nw = slow
            .iter()
            .take(12)
            .map(|c| ui::width(&c.name))
            .max()
            .unwrap_or(0)
            .clamp(20, 44);
        for c in slow.iter().take(12) {
            let kind = match c.kind.as_str() {
                "app" => tr!("app", "uygulama"),
                "driver" => tr!("driver", "sürücü"),
                _ => tr!("service", "hizmet"),
            };
            ui::warn(&format!(
                "{} {}  {}",
                ui::fit(&c.name, nw),
                ui::pad(&kind, 10),
                tr!(
                    "+{} on average, {}x",
                    "ortalama +{}, {} kez",
                    secs(c.avg_delay),
                    c.times
                )
            ));
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "turn startup apps off with: kz startup",
            "başlangıç uygulamalarını kapatmak için: kz startup"
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_culprits() {
        let c = culprits(vec![
            ("app".into(), "Teams".into(), 4000),
            ("app".into(), "Teams".into(), 2000),
            ("service".into(), "Spooler".into(), 1000),
            ("app".into(), " ".into(), 9000),
        ]);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].name, "Teams");
        assert_eq!((c[0].times, c[0].avg_delay), (2, 3000));
        assert_eq!(secs(1500), "1.5 s");
    }

    #[test]
    fn read_errors() {
        assert_eq!(
            read_error(
                "Get-WinEvent : No events were found that match the specified selection criteria."
            ),
            ReadError::NoBoots
        );
        assert_eq!(
            read_error(
                "Get-WinEvent : There is not an event log on the localhost computer that matches \"Microsoft-Windows-Diagnostics-Performance/Operational\"."
            ),
            ReadError::NoBoots
        );
        assert_eq!(
            read_error("Get-WinEvent : Attempted to perform an unauthorized operation."),
            ReadError::Denied
        );
        assert_eq!(read_error("something else"), ReadError::Other);
    }
}
