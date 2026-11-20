//! `kz tasks`: scheduled tasks that programs added (updaters, helpers,
//! telemetry of other apps). `[x]` = enabled; toggling uses Windows' own
//! enable / disable, so nothing is deleted and everything can be turned back on.
//! `kz tasks --all` also lists Windows' own tasks (read-only).

use crate::{oplog, ps, sys, tr, ui};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Task {
    pub name: String,
    pub path: String,
    pub state: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub action: String,
}

const READ: &str = r#"
@(Get-ScheduledTask | ForEach-Object {
  $a = @($_.Actions | Where-Object { $_.Execute } | ForEach-Object { ($_.Execute + ' ' + $_.Arguments).Trim() }) -join ' ; '
  [pscustomobject]@{ Name = $_.TaskName; Path = $_.TaskPath; State = [string]$_.State
    Author = [string]$_.Author; Description = [string]$_.Description; Action = [string]$a }
}) | ConvertTo-Json -Compress
"#;

const SET: &str = r#"
$t = Get-ScheduledTask -TaskPath $env:KAZMA_PATH -TaskName $env:KAZMA_NAME -ErrorAction Stop
if ($env:KAZMA_ON -eq '1') { $null = $t | Enable-ScheduledTask -ErrorAction Stop }
else { $null = $t | Disable-ScheduledTask -ErrorAction Stop }
"#;

/// Windows' own tasks live under \Microsoft\.
pub fn is_windows(t: &Task) -> bool {
    t.path.to_lowercase().starts_with(r"\microsoft\")
}

fn enabled(t: &Task) -> bool {
    t.state != "Disabled"
}

fn what(t: &Task) -> String {
    let d = t.description.lines().next().unwrap_or("").trim();
    if !d.is_empty() {
        d.to_string()
    } else {
        t.action.clone()
    }
}

pub fn run(all: bool, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "tasks :: scheduled tasks that programs added",
        "tasks :: programların eklediği zamanlanmış görevler"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz tasks works on Windows only.",
            "kz tasks yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    ps::working(&tr!(
        "reading scheduled tasks...",
        "zamanlanmış görevler okunuyor..."
    ));
    let mut tasks: Vec<Task> = ps::powershell_json(READ, &[])?;
    println!("{:60}", "");
    tasks.sort_by_key(|t| (is_windows(t), t.path.to_lowercase(), t.name.to_lowercase()));
    let mine: Vec<&Task> = tasks.iter().filter(|t| !is_windows(t)).collect();

    if all || dry || !ui::interactive() {
        let shown: Vec<&Task> = if all {
            tasks.iter().collect()
        } else {
            mine.clone()
        };
        if shown.is_empty() {
            ui::ok(&tr!(
                "No scheduled tasks from other programs.",
                "Başka programlardan zamanlanmış görev yok."
            ));
            return Ok(());
        }
        let nw = 40;
        let mut header = None;
        for t in shown {
            let w = is_windows(t);
            if header != Some(w) {
                header = Some(w);
                ui::section(&if w {
                    tr!("Windows' own tasks", "Windows'un kendi görevleri")
                } else {
                    tr!("Added by programs", "Programların eklediği")
                });
            }
            let room = ui::term_width().saturating_sub(7 + nw + 2);
            let line = format!(
                "{}  {}",
                ui::fit(
                    &format!("{}{}", t.path.trim_start_matches('\\'), t.name),
                    nw
                ),
                ui::truncate_end(&what(t), room.max(10))
            );
            if enabled(t) {
                ui::ok(&line)
            } else {
                ui::skip(&line)
            }
        }
        println!(
            "\n  {}\n",
            tr!(
                "[ok] enabled   [--] disabled   turn the programs' tasks on / off with: kz tasks",
                "[ok] açık   [--] kapalı   programların görevlerini açıp kapatmak için: kz tasks"
            )
        );
        return Ok(());
    }

    if mine.is_empty() {
        ui::ok(&tr!(
            "No scheduled tasks from other programs.",
            "Başka programlardan zamanlanmış görev yok."
        ));
        return Ok(());
    }
    let items = mine
        .iter()
        .map(|t| ui::CheckItem {
            label: t.name.clone(),
            detail: t.path.clone(),
            size: 0,
            checked: enabled(t),
        })
        .collect();
    let notes: Vec<String> = mine
        .iter()
        .map(|t| {
            let by = if t.author.is_empty() {
                String::new()
            } else {
                format!("{} :: ", t.author)
            };
            format!("{by}{}", what(t))
        })
        .collect();
    let Some(sel) = ui::checklist_explained(
        &tr!(
            "kazma tasks :: [x] = enabled, enter applies the changes",
            "kazma tasks :: [x] = açık, enter değişiklikleri uygular"
        ),
        items,
        &notes,
    )?
    else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };
    let changes: Vec<(&Task, bool)> = mine
        .iter()
        .zip(sel)
        .filter(|(t, on)| enabled(t) != *on)
        .map(|(t, on)| (*t, on))
        .collect();
    if changes.is_empty() {
        ui::ok(&tr!("Nothing to change.", "Değiştirilecek bir şey yok."));
        return Ok(());
    }
    ui::section(&tr!("Applying", "Uygulanıyor"));
    let mut denied = false;
    for (t, on) in changes {
        let label = format!(
            "{} {}{}",
            if on {
                tr!("[on] ", "[aç] ")
            } else {
                tr!("[off]", "[kapat]")
            },
            t.path,
            t.name
        );
        match ps::powershell(
            SET,
            &[
                ("KAZMA_PATH", &t.path),
                ("KAZMA_NAME", &t.name),
                ("KAZMA_ON", if on { "1" } else { "0" }),
            ],
        ) {
            Ok(o) if o.status.success() => {
                ui::ok(&label);
                oplog::record(
                    "tasks",
                    &format!("{} {}{}", if on { "on" } else { "off" }, t.path, t.name),
                );
            }
            Ok(o) => {
                let e = ps::first_line(&String::from_utf8_lossy(&o.stderr));
                denied |= e.to_lowercase().contains("access is denied");
                ui::warn(&format!("{label}  ({e})"));
            }
            Err(e) => ui::warn(&format!("{label}  ({e})")),
        }
    }
    if denied && !sys::is_admin() {
        println!(
            "\n  {}",
            tr!(
                "some tasks belong to the system: run kz from an elevated terminal for those.",
                "bazı görevler sisteme ait: bunlar için kz'yi yönetici terminalinde çalıştırın."
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
    fn windows_tasks() {
        let t = |p: &str| Task {
            name: "x".into(),
            path: p.into(),
            state: "Ready".into(),
            author: String::new(),
            description: String::new(),
            action: "a.exe".into(),
        };
        assert!(is_windows(&t(r"\Microsoft\Windows\Defrag\")));
        assert!(!is_windows(&t(r"\")));
        assert!(!is_windows(&t(r"\GoogleSystem\")));
        assert_eq!(what(&t(r"\")), "a.exe");
    }
}
