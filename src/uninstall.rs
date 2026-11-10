//! `kz uninstall`: remove installed programs with their own uninstaller, then
//! offer to send leftover folders (AppData, ProgramData, install dir) to the Recycle Bin.

use crate::fsutil::{self, env_path};
use crate::{oplog, ps, ui};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

const LIST: &str = r#"
$roots = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
         'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
         'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
@(Get-ItemProperty $roots -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -and $_.UninstallString -and $_.SystemComponent -ne 1 -and -not $_.ParentKeyName -and -not $_.ReleaseType } |
  ForEach-Object {
    [pscustomobject]@{
      Name      = [string]$_.DisplayName
      Version   = [string]$_.DisplayVersion
      Publisher = [string]$_.Publisher
      SizeKb    = [int64]$_.EstimatedSize
      Location  = [string]$_.InstallLocation
      Uninstall = [string]$_.UninstallString
      Quiet     = [string]$_.QuietUninstallString
      Key       = [string]$_.PSChildName
    }
  }) | ConvertTo-Json -Compress
"#;

const STILL_INSTALLED: &str = r#"
$k = $env:KAZMA_KEY
$hit = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\',
       'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\',
       'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' | Where-Object { Test-Path ($_ + $k) }
if ($hit) { 'yes' } else { 'no' }
"#;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub size_kb: i64,
    #[serde(default)]
    pub location: String,
    pub uninstall: String,
    #[serde(default)]
    pub quiet: String,
    pub key: String,
}

/// "7-Zip 23.01 (x64)" -> "7-Zip": drop (...) parts and version-like words.
pub fn base_name(name: &str) -> String {
    let mut s = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = i32::max(depth - 1, 0),
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            let v = w.trim_start_matches(['v', 'V']);
            let version = !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.') && v.chars().any(|c| c.is_ascii_digit());
            !version && !matches!(w.to_lowercase().as_str(), "x64" | "x86" | "64-bit" | "32-bit")
        })
        .collect();
    words.join(" ").trim_matches(|c: char| c == '-' || c.is_whitespace()).to_string()
}

/// Names too generic to delete as "leftovers" because other programs share them.
fn generic(name: &str) -> bool {
    let n = name.to_lowercase();
    n.len() < 3
        || [
            "microsoft", "windows", "google", "intel", "nvidia", "amd", "packages", "programs", "temp",
            "common files", "mozilla", "apple", "adobe", "steam", "microsoft corporation", "jetbrains", "oracle",
        ]
        .contains(&n.as_str())
}

/// Find leftover folders after an uninstall. Only exact folder-name matches are
/// considered, and everything goes through the same guards as other deletions.
pub fn leftovers(p: &Program) -> Vec<PathBuf> {
    let name = base_name(&p.name);
    let publisher = base_name(&p.publisher);
    let bases: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA", "ProgramData"]
        .iter()
        .filter_map(|v| env_path(v))
        .chain(env_path("LOCALAPPDATA").map(|l| l.join("Programs")))
        .collect();
    let mut out = vec![];
    if !p.location.trim().is_empty() {
        out.push(PathBuf::from(p.location.trim().trim_matches('"')));
    }
    if !generic(&name) {
        for b in &bases {
            out.push(b.join(&name));
            if !publisher.is_empty() && !publisher.eq_ignore_ascii_case(&name) {
                out.push(b.join(&publisher).join(&name));
            }
        }
    }
    out.sort();
    out.dedup();
    out.into_iter().filter(|d| d.is_dir() && fsutil::can_remove(d)).collect()
}

fn still_installed(key: &str) -> bool {
    ps::powershell(STILL_INSTALLED, &[("KAZMA_KEY", key)])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "yes")
        .unwrap_or(false)
}

/// The command line to run. MSI products are removed with msiexec /x so the
/// standard Windows Installer progress dialog is shown.
fn command_line(p: &Program, quiet: bool) -> String {
    let u = p.uninstall.trim();
    let lower = u.to_lowercase();
    if let (true, Some(a), Some(b)) = (lower.contains("msiexec"), u.find('{'), u.find('}'))
        && b > a
    {
        let mode = if quiet { "/qn" } else { "/passive" };
        return format!("msiexec.exe /x {} {mode} /norestart", &u[a..=b]);
    }
    if quiet && !p.quiet.trim().is_empty() { p.quiet.trim().to_string() } else { u.to_string() }
}

#[cfg(windows)]
fn run_command_line(line: &str) -> std::io::Result<std::process::ExitStatus> {
    use std::os::windows::process::CommandExt;
    // `/s /c "<line>"`: cmd strips exactly the outer quotes, leaving the vendor's
    // own quoting intact.
    Command::new("cmd").raw_arg(format!("/d /s /c \"{line}\"")).status()
}

#[cfg(not(windows))]
fn run_command_line(line: &str) -> std::io::Result<std::process::ExitStatus> {
    Command::new("sh").arg("-c").arg(line).status()
}

fn pause(msg: &str) {
    use std::io::Write;
    print!("  .. {msg}");
    let _ = std::io::stdout().flush();
    let _ = std::io::stdin().read_line(&mut String::new());
}

pub fn run(dry: bool, quiet: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry { "uninstall :: dry run, nothing will be removed" } else { "uninstall :: remove programs and their leftovers" });
    if !cfg!(windows) {
        ui::warn("kz uninstall works on Windows only.");
        return Ok(());
    }
    if !dry && !ui::interactive() {
        ui::warn("kz uninstall needs an interactive terminal (or use --dry-run to list programs).");
        return Ok(());
    }
    ps::working("reading installed programs...");
    let mut progs: Vec<Program> = ps::powershell_json(LIST, &[])?;
    println!("{:60}", "");
    progs.sort_by_key(|p| p.name.to_lowercase());
    progs.dedup_by(|a, b| a.name == b.name && a.version == b.version);

    if dry {
        ui::section(&format!("{} installed programs", progs.len()));
        for p in &progs {
            println!("  {:<44} {:<16} {:>10}  {}", ui::truncate(&p.name, 44), ui::truncate(&p.version, 16),
                if p.size_kb > 0 { ui::human(p.size_kb as u64 * 1024) } else { "-".into() }, ui::truncate(&p.publisher, 24));
        }
        println!();
        return Ok(());
    }

    let items = progs
        .iter()
        .map(|p| ui::CheckItem {
            label: p.name.clone(),
            detail: format!("{}  {}", p.version, p.publisher),
            size: p.size_kb.max(0) as u64 * 1024,
            checked: false,
        })
        .collect();
    let Some(sel) = ui::checklist("kazma uninstall :: select programs to remove", items)? else {
        println!("  cancelled.");
        return Ok(());
    };
    let chosen: Vec<&Program> = progs.iter().zip(&sel).filter(|(_, on)| **on).map(|(p, _)| p).collect();
    if chosen.is_empty() {
        println!("  nothing selected.");
        return Ok(());
    }

    ui::section("Will uninstall");
    for p in &chosen {
        println!("  - {} {}", p.name, p.version);
    }
    println!();
    if !crate::remove::ask(&format!("Uninstall {} program(s)?", chosen.len())) {
        println!("  cancelled.");
        return Ok(());
    }

    let mut removed: Vec<&Program> = vec![];
    for p in &chosen {
        ui::section(&p.name);
        let line = command_line(p, quiet);
        println!("  running: {}", ui::truncate(&line, 90));
        match run_command_line(&line) {
            Ok(_) => {}
            Err(e) => {
                ui::warn(&format!("could not start the uninstaller: {e}"));
                continue;
            }
        }
        if still_installed(&p.key) {
            // Many uninstallers (NSIS, Inno) hand off to a copy of themselves and exit at once.
            pause("if the uninstaller window is still open, finish it, then press Enter...");
        }
        if still_installed(&p.key) {
            ui::warn("still installed (uninstall cancelled or failed), leftovers not touched");
            oplog::record("uninstall", &format!("{} NOT removed", p.name));
        } else {
            ui::ok("uninstalled");
            oplog::record("uninstall", &format!("{} {}", p.name, p.version));
            removed.push(p);
        }
    }

    // Leftovers of everything that was really removed.
    let found: Vec<(PathBuf, u64)> = removed
        .iter()
        .flat_map(|p| leftovers(p))
        .map(|d| {
            let s = fsutil::dir_size(&d);
            (d, s)
        })
        .collect();
    if found.is_empty() {
        println!("\n  no leftovers found.\n");
        return Ok(());
    }
    let items = found
        .iter()
        .map(|(d, s)| ui::CheckItem { label: d.display().to_string(), detail: String::new(), size: *s, checked: true })
        .collect();
    let Some(sel) = ui::checklist("kazma uninstall :: leftover folders -> Recycle Bin", items)? else {
        println!("  leftovers kept.");
        return Ok(());
    };
    ui::section("Leftovers");
    for ((d, s), on) in found.iter().zip(sel) {
        if !on {
            ui::skip(&format!("{} (kept)", d.display()));
            continue;
        }
        match fsutil::trash(d) {
            Ok(()) => {
                ui::ok(&format!("{:>10}  {}", ui::human(*s), d.display()));
                oplog::record("uninstall", &format!("trashed leftover {}", d.display()));
            }
            Err(e) => ui::warn(&format!("{}  ({e})", d.display())),
        }
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(uninstall: &str, quiet: &str) -> Program {
        Program {
            name: "X".into(),
            version: String::new(),
            publisher: String::new(),
            size_kb: 0,
            location: String::new(),
            uninstall: uninstall.into(),
            quiet: quiet.into(),
            key: "k".into(),
        }
    }

    #[test]
    fn names() {
        assert_eq!(base_name("7-Zip 23.01 (x64)"), "7-Zip");
        assert_eq!(base_name("Mozilla Firefox (x64 en-US)"), "Mozilla Firefox");
        assert_eq!(base_name("Notepad++ v8.6.9"), "Notepad++");
        assert_eq!(base_name("VLC media player"), "VLC media player");
        assert!(generic("Microsoft"));
        assert!(!generic("Notepad++"));
    }

    #[test]
    fn msi_command() {
        let p = prog("MsiExec.exe /I{23170F69-40C1-2702-2301-000001000000}", "");
        assert_eq!(command_line(&p, false), "msiexec.exe /x {23170F69-40C1-2702-2301-000001000000} /passive /norestart");
        let q = prog("\"C:\\Program Files\\App\\uninst.exe\"", "\"C:\\Program Files\\App\\uninst.exe\" /S");
        assert_eq!(command_line(&q, false), "\"C:\\Program Files\\App\\uninst.exe\"");
        assert_eq!(command_line(&q, true), "\"C:\\Program Files\\App\\uninst.exe\" /S");
    }
}
