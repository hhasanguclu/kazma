//! `kz optimize`: safe Windows maintenance tasks.

use crate::{oplog, ps, sys, ui};
use std::process::Command;

enum Action {
    /// Program and arguments.
    Exec(&'static str, &'static [&'static str]),
    /// PowerShell script.
    Ps(&'static str),
}

struct Task {
    name: &'static str,
    detail: &'static str,
    admin: bool,
    default_on: bool,
    action: Action,
}

const TRIM: &str = r#"
$ok = 0
Get-Volume | Where-Object { $_.DriveLetter -and $_.DriveType -eq 'Fixed' -and $_.FileSystemType -eq 'NTFS' } | ForEach-Object {
  try { Optimize-Volume -DriveLetter $_.DriveLetter -ReTrim -ErrorAction Stop; $ok++ } catch {}
}
if ($ok -eq 0) { throw 'no SSD volumes could be trimmed' }
"#;

const RESTART_EXPLORER: &str = r#"
Stop-Process -Name explorer -Force
Start-Sleep -Seconds 1
if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process explorer.exe }
"#;

fn tasks() -> Vec<Task> {
    use Action::*;
    vec![
        Task { name: "Flush DNS cache", detail: "fixes stale or wrong website addresses", admin: false, default_on: true, action: Exec("ipconfig", &["/flushdns"]) },
        Task { name: "Refresh icon cache", detail: "fixes blank or wrong icons", admin: false, default_on: true, action: Exec("ie4uinit.exe", &["-show"]) },
        Task { name: "Restart Windows Explorer", detail: "taskbar / desktop glitches; windows will flicker", admin: false, default_on: false, action: Ps(RESTART_EXPLORER) },
        Task { name: "TRIM SSD drives", detail: "keeps SSD write speed up", admin: true, default_on: true, action: Ps(TRIM) },
        Task { name: "Clean up component store (DISM)", detail: "removes superseded Windows updates, 5-15 min", admin: true, default_on: false, action: Exec("Dism.exe", &["/Online", "/Cleanup-Image", "/StartComponentCleanup"]) },
        Task { name: "Check system files (SFC)", detail: "repairs corrupted Windows files, 10-20 min", admin: true, default_on: false, action: Exec("sfc", &["/scannow"]) },
    ]
}

fn execute(a: &Action) -> Result<(), String> {
    let out = match a {
        Action::Exec(prog, args) => {
            let mut c = Command::new(prog);
            c.args(*args);
            ps::hide_window(&mut c);
            c.output()
        }
        Action::Ps(script) => ps::powershell(script, &[]).map_err(std::io::Error::other),
    };
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let text = format!("{}{}", String::from_utf8_lossy(&o.stderr), String::from_utf8_lossy(&o.stdout));
            // sfc/dism print UTF-16; strip NULs so the message is readable.
            let text: String = text.chars().filter(|c| *c != '\0').collect();
            let msg = text.lines().map(str::trim).rfind(|l| !l.is_empty()).unwrap_or("failed").to_string();
            Err(format!("exit {}: {}", o.status.code().unwrap_or(-1), ui::truncate(&msg, 70)))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry { "optimize :: dry run, nothing will be changed" } else { "optimize :: windows maintenance" });
    if !cfg!(windows) {
        ui::warn("kz optimize works on Windows only.");
        return Ok(());
    }
    let admin = sys::is_admin();
    let (runnable, need_admin): (Vec<Task>, Vec<Task>) = tasks().into_iter().partition(|t| admin || !t.admin);

    if !dry && !yes && !ui::interactive() {
        ui::warn("not an interactive terminal: re-run with --yes to run the default tasks, or --dry-run to preview");
        return Ok(());
    }
    let selection: Vec<bool> = if dry || yes {
        runnable.iter().map(|t| t.default_on).collect()
    } else {
        let items = runnable
            .iter()
            .map(|t| ui::CheckItem { label: t.name.to_string(), detail: t.detail.to_string(), size: 0, checked: t.default_on })
            .collect();
        match ui::checklist_opts("kazma optimize :: select maintenance tasks", items, false)? {
            Some(s) => s,
            None => {
                println!("  cancelled.");
                return Ok(());
            }
        }
    };

    ui::section(if dry { "Would run" } else { "Running" });
    for (t, on) in runnable.iter().zip(selection) {
        if !on {
            ui::skip(&format!("{:<34} (skipped)", t.name));
            continue;
        }
        if dry {
            ui::ok(&format!("{:<34} {}", t.name, t.detail));
            continue;
        }
        ps::working(&format!("{:<34} running...", t.name));
        match execute(&t.action) {
            Ok(()) => {
                ui::ok(&format!("{:<34} done      ", t.name));
                oplog::record("optimize", t.name);
            }
            Err(e) => {
                ui::warn(&format!("{:<34} {e}", t.name));
                oplog::record("optimize", &format!("{} FAILED {e}", t.name));
            }
        }
    }
    if !need_admin.is_empty() {
        ui::section("Needs administrator");
        for t in &need_admin {
            ui::skip(&format!("{:<34} (run kz from an elevated terminal)", t.name));
        }
    }
    println!();
    Ok(())
}
