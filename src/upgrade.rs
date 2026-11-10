//! `kz upgrade`: update installed programs through winget.

use crate::{oplog, ps, ui};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct Pkg {
    pub name: String,
    pub id: String,
    pub version: String,
    pub available: String,
    pub source: String,
}

/// Clean one raw output line: keep only what follows the last carriage return
/// (winget redraws its spinner with `\r`) and drop control characters.
fn clean_line(raw: &str) -> String {
    let after_cr = raw.rsplit('\r').next().unwrap_or(raw);
    after_cr.chars().filter(|c| !c.is_control()).collect::<String>().trim_end().to_string()
}

/// Parse the table printed by `winget upgrade`. Column boundaries are taken from
/// the header line, so it works whatever language winget prints its headers in.
pub fn parse_winget_table(output: &str) -> Vec<Pkg> {
    let lines: Vec<String> = output.lines().map(clean_line).collect();
    let Some(dash) = lines.iter().position(|l| l.len() >= 10 && l.chars().all(|c| c == '-')) else {
        return vec![];
    };
    if dash == 0 {
        return vec![];
    }
    let header: Vec<char> = lines[dash - 1].chars().collect();
    let starts: Vec<usize> =
        (0..header.len()).filter(|&i| header[i] != ' ' && (i == 0 || header[i - 1] == ' ')).collect();
    if starts.len() < 4 {
        return vec![];
    }
    let col = |chars: &[char], i: usize| -> String {
        let from = starts[i].min(chars.len());
        let to = starts.get(i + 1).copied().unwrap_or(chars.len()).min(chars.len());
        chars[from..to].iter().collect::<String>().trim().to_string()
    };
    let mut out = vec![];
    for line in &lines[dash + 1..] {
        if line.trim().is_empty() {
            break;
        }
        let chars: Vec<char> = line.chars().collect();
        if chars.len() <= starts[3] {
            break; // summary line such as "5 upgrades available."
        }
        let pkg = Pkg {
            name: col(&chars, 0),
            id: col(&chars, 1),
            version: col(&chars, 2),
            available: col(&chars, 3),
            source: if starts.len() > 4 { col(&chars, 4) } else { String::new() },
        };
        if pkg.id.is_empty() || pkg.id.contains(' ') || pkg.available.is_empty() {
            continue;
        }
        out.push(pkg);
    }
    out
}

fn winget() -> Command {
    let mut c = Command::new("winget");
    ps::hide_window(&mut c);
    c
}

fn winget_available() -> bool {
    winget().arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn list_upgrades() -> anyhow::Result<Vec<Pkg>> {
    let out = winget()
        .args(["upgrade", "--accept-source-agreements", "--disable-interactivity"])
        .output()?;
    Ok(parse_winget_table(&String::from_utf8_lossy(&out.stdout)))
}

fn upgrade_one(p: &Pkg) -> (bool, String) {
    let mut args = vec![
        "upgrade", "--id", &p.id, "--exact", "--silent", "--accept-package-agreements",
        "--accept-source-agreements", "--disable-interactivity",
    ];
    if !p.source.is_empty() {
        args.extend(["--source", &p.source]);
    }
    match winget().args(&args).output() {
        Ok(o) if o.status.success() => (true, String::new()),
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let last = text.lines().map(clean_line).rfind(|l| !l.trim().is_empty()).unwrap_or_default();
            (false, format!("exit {:#x}: {}", o.status.code().unwrap_or(-1) as u32, last.trim()))
        }
        Err(e) => (false, e.to_string()),
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry { "upgrade :: dry run, nothing will be installed" } else { "upgrade :: update installed programs (winget)" });
    if !cfg!(windows) {
        ui::warn("kz upgrade works on Windows only.");
        return Ok(());
    }
    if !winget_available() {
        ui::warn("winget not found. Install 'App Installer' from the Microsoft Store, then try again.");
        return Ok(());
    }
    ps::working("asking winget for updates (this can take a few seconds)...");
    let pkgs = list_upgrades()?;
    println!("{:60}", "");
    if pkgs.is_empty() {
        ui::ok("Everything is up to date.");
        return Ok(());
    }
    if !dry && !yes && !ui::interactive() {
        ui::warn("not an interactive terminal: re-run with --yes to upgrade, or --dry-run to preview");
        return Ok(());
    }

    let selection: Vec<bool> = if dry || yes {
        vec![true; pkgs.len()]
    } else {
        let items = pkgs
            .iter()
            .map(|p| ui::CheckItem {
                label: p.name.clone(),
                detail: format!("{} -> {}   ({})", p.version, p.available, p.id),
                size: 0,
                checked: true,
            })
            .collect();
        match ui::checklist_opts("kazma upgrade :: select programs to update", items, false)? {
            Some(s) => s,
            None => {
                println!("  cancelled.");
                return Ok(());
            }
        }
    };

    ui::section(if dry { "Available updates" } else { "Upgrading" });
    let (mut ok, mut fail) = (0, 0);
    for (p, on) in pkgs.iter().zip(selection) {
        let line = format!("{:<36} {:>14} -> {:<14}", ui::truncate(&p.name, 36), ui::truncate(&p.version, 14), ui::truncate(&p.available, 14));
        if !on {
            ui::skip(&format!("{line}  (skipped)"));
            continue;
        }
        if dry {
            ui::ok(&line);
            continue;
        }
        ps::working(&format!("{line}  installing..."));
        let (success, err) = upgrade_one(p);
        if success {
            ok += 1;
            ui::ok(&format!("{line}           "));
            oplog::record("upgrade", &format!("{} {} -> {}", p.id, p.version, p.available));
        } else {
            fail += 1;
            ui::warn(&format!("{line}  {err}"));
            oplog::record("upgrade", &format!("{} FAILED {err}", p.id));
        }
    }
    if !dry {
        println!("\n  upgraded {ok}, failed {fail}");
        if fail > 0 {
            println!("  tip: some installers need an elevated terminal, or the program to be closed first.");
        }
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str = "   - \r   \\ \r\
Name                                 Id                           Version      Available    Source\n\
---------------------------------------------------------------------------------------------------\n\
Microsoft Visual Studio Code         Microsoft.VisualStudioCode   1.94.0       1.95.2       winget\n\
7-Zip 23.01 (x64)                    7zip.7zip                    23.01        24.08        winget\n\
Git                                  Git.Git                      < 2.47.0     2.47.0       winget\n\
3 upgrades available.\n\
\n\
The following packages have an upgrade available, but require explicit targeting for upgrade:\n";

    /// Build a winget-style table with real column alignment (widths in chars).
    fn table(rows: &[[&str; 5]]) -> String {
        let mut t = String::new();
        for (i, r) in rows.iter().enumerate() {
            t += &format!("{:<22}{:<20}{:<11}{:<15}{}\n", r[0], r[1], r[2], r[3], r[4]);
            if i == 0 {
                t += &format!("{}\n", "-".repeat(74));
            }
        }
        t + "2 y\u{fc}kseltme kullan\u{131}labilir.\n"
    }

    #[test]
    fn parses_english() {
        let p = parse_winget_table(EN);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].id, "Microsoft.VisualStudioCode");
        assert_eq!(p[1].name, "7-Zip 23.01 (x64)");
        assert_eq!(p[1].available, "24.08");
        assert_eq!(p[2].version, "< 2.47.0");
        assert_eq!(p[2].source, "winget");
    }

    #[test]
    fn parses_turkish_headers_and_unicode_names() {
        let tr = table(&[
            ["Ad", "Kimlik", "S\u{fc}r\u{fc}m", "Kullan\u{131}labilir", "Kaynak"],
            ["Mozilla Firefox (x64)", "Mozilla.Firefox", "131.0", "132.0.1", "winget"],
            ["Discord \u{2026}", "Discord.Discord", "1.0.9163", "1.0.9171", "winget"],
        ]);
        let p = parse_winget_table(&tr);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].id, "Mozilla.Firefox");
        assert_eq!(p[1].name, "Discord \u{2026}");
        assert_eq!(p[1].available, "1.0.9171");
    }

    #[test]
    fn empty_when_up_to_date() {
        assert!(parse_winget_table("No installed package found matching input criteria.\n").is_empty());
    }
}
