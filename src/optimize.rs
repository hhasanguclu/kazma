//! `kz optimize`: safe Windows maintenance tasks.

use crate::{oplog, ps, sys, tr, ui};
use std::process::Command;

enum Action {
    /// Program and arguments.
    Exec(&'static str, &'static [&'static str]),
    /// PowerShell script.
    Ps(&'static str),
    /// Rust function.
    Native(fn() -> Result<(), String>),
}

struct Task {
    name: &'static str,
    detail: &'static str,
    admin: bool,
    default_on: bool,
    action: Action,
}

/// Tasks that only need to run now and then: (state key, days between runs).
fn due_key(a: &Action) -> Option<(&'static str, i64)> {
    match a {
        Action::Ps(s) if *s == TRIM => Some(("trim", 7)),
        Action::Exec(p, _) if p.eq_ignore_ascii_case("dism.exe") => Some(("dism", 30)),
        Action::Exec(p, _) if p.eq_ignore_ascii_case("sfc") => Some(("sfc", 90)),
        _ => None,
    }
}

const RESTART_SHELL: &str = r#"
foreach ($n in 'StartMenuExperienceHost', 'SearchHost', 'SearchApp', 'SearchUI', 'ShellExperienceHost') {
  Stop-Process -Name $n -Force -ErrorAction SilentlyContinue
}
"#;

fn state_file() -> Option<std::path::PathBuf> {
    oplog::data_dir().map(|d| d.join("optimize-state.txt"))
}

/// Days since 1970-01-01 for "YYYY-MM-DD" (Howard Hinnant's days_from_civil).
pub fn day_number(date: &str) -> Option<i64> {
    let mut it = date.get(..10)?.split('-').map(|x| x.parse::<i64>());
    let (y, m, d) = (it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Days since the task last ran successfully, or None if never.
fn days_since(key: &str) -> Option<i64> {
    let text = std::fs::read_to_string(state_file()?).ok()?;
    let last = text
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{key}=")))?;
    Some(day_number(&oplog::now_utc())? - day_number(last)?)
}

fn mark_done(key: &str) {
    let Some(f) = state_file() else { return };
    let today = &oplog::now_utc()[..10];
    let mut lines: Vec<String> = std::fs::read_to_string(&f)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with(&format!("{key}=")))
        .map(String::from)
        .collect();
    lines.push(format!("{key}={today}"));
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(f, lines.join("\n") + "\n");
}

fn broken_startup() -> Result<(), String> {
    crate::startup::remove_broken().map(|_| ())
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
    let broken = if cfg!(windows) {
        crate::startup::list()
            .map(|v| v.iter().filter(|e| crate::startup::is_broken(e)).count())
            .unwrap_or(0)
    } else {
        0
    };
    let mut v = vec![
        Task {
            name: ui::t(
                "Remove broken startup entries",
                "Bozuk başlangıç kayıtlarını kaldır",
            ),
            detail: if broken > 0 {
                ui::t(
                    "programs that no longer exist; backed up first",
                    "artık var olmayan programlar; önce yedeklenir",
                )
            } else {
                ui::t("none found", "bulunamadı")
            },
            admin: false,
            default_on: broken > 0,
            action: Native(broken_startup),
        },
        Task {
            name: ui::t(
                "Restart Start menu and Search",
                "Başlat menüsünü ve Aramayı yeniden başlat",
            ),
            detail: ui::t(
                "Start or search not opening or stuck",
                "Başlat ya da arama açılmıyor veya takılıyor",
            ),
            admin: false,
            default_on: false,
            action: Ps(RESTART_SHELL),
        },
    ];
    v.extend(vec![
        Task {
            name: ui::t("Flush DNS cache", "DNS önbelleğini temizle"),
            detail: ui::t(
                "fixes stale or wrong website addresses",
                "eski ya da hatalı site adreslerini düzeltir",
            ),
            admin: false,
            default_on: true,
            action: Exec("ipconfig", &["/flushdns"]),
        },
        Task {
            name: ui::t("Refresh icon cache", "İkon önbelleğini yenile"),
            detail: ui::t(
                "fixes blank or wrong icons",
                "boş ya da yanlış ikonları düzeltir",
            ),
            admin: false,
            default_on: true,
            action: Exec("ie4uinit.exe", &["-show"]),
        },
        Task {
            name: ui::t(
                "Restart Windows Explorer",
                "Windows Gezgini'ni yeniden başlat",
            ),
            detail: ui::t(
                "taskbar / desktop glitches; windows will flicker",
                "görev çubuğu / masaüstü hataları; ekran titrer",
            ),
            admin: false,
            default_on: false,
            action: Ps(RESTART_EXPLORER),
        },
        Task {
            name: ui::t("TRIM SSD drives", "SSD'lere TRIM uygula"),
            detail: ui::t("keeps SSD write speed up", "SSD yazma hızını korur"),
            admin: true,
            default_on: true,
            action: Ps(TRIM),
        },
        Task {
            name: ui::t(
                "Clean up component store (DISM)",
                "Bileşen deposunu temizle (DISM)",
            ),
            detail: ui::t(
                "removes superseded Windows updates, 5-15 min",
                "eski Windows güncellemelerini siler, 5-15 dk",
            ),
            admin: true,
            default_on: false,
            action: Exec(
                "Dism.exe",
                &["/Online", "/Cleanup-Image", "/StartComponentCleanup"],
            ),
        },
        Task {
            name: ui::t(
                "Check system files (SFC)",
                "Sistem dosyalarını denetle (SFC)",
            ),
            detail: ui::t(
                "repairs corrupted Windows files, 10-20 min",
                "bozuk Windows dosyalarını onarır, 10-20 dk",
            ),
            admin: true,
            default_on: false,
            action: Exec("sfc", &["/scannow"]),
        },
    ]);
    // TRIM / DISM / SFC: pre-select only when they are due, and say when they last ran
    for t in v.iter_mut() {
        if let Some((key, every)) = due_key(&t.action) {
            let since = days_since(key);
            t.default_on = since.is_none_or(|d| d >= every);
            let when = match since {
                Some(0) => ui::t("ran today", "bugün çalıştı").to_string(),
                Some(d) => tr!("last run {d} days ago", "son çalışma {d} gün önce"),
                None => ui::t("never run by kazma", "kazma ile hiç çalışmadı").to_string(),
            };
            // tasks live for this run only; a short leaked string is fine
            t.detail = Box::leak(format!("{}  ({when})", t.detail).into_boxed_str());
        }
    }
    v
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
        Action::Native(f) => return f(),
    };
    match out {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stderr),
                String::from_utf8_lossy(&o.stdout)
            );
            // sfc/dism print UTF-16; strip NULs so the message is readable.
            let text: String = text.chars().filter(|c| *c != '\0').collect();
            let msg = text
                .lines()
                .map(str::trim)
                .rfind(|l| !l.is_empty())
                .unwrap_or("failed")
                .to_string();
            Err(format!(
                "exit {}: {}",
                o.status.code().unwrap_or(-1),
                ui::truncate(&msg, 70)
            ))
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t(
            "optimize :: dry run, nothing will be changed",
            "optimize :: deneme, hiçbir şey değişmeyecek",
        )
    } else {
        ui::t(
            "optimize :: windows maintenance",
            "optimize :: windows bakımı",
        )
    });
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz optimize works on Windows only.",
            "kz optimize yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let admin = sys::is_admin();
    let (runnable, need_admin): (Vec<Task>, Vec<Task>) =
        tasks().into_iter().partition(|t| admin || !t.admin);

    if !dry && !yes && !ui::interactive() {
        ui::warn(ui::t(
            "not an interactive terminal: re-run with --yes to run the default tasks, or --dry-run to preview",
            "etkileşimli terminal değil: varsayılan görevler için --yes, önizleme için --dry-run kullanın",
        ));
        return Ok(());
    }
    let selection: Vec<bool> = if dry || yes {
        runnable.iter().map(|t| t.default_on).collect()
    } else {
        let items = runnable
            .iter()
            .map(|t| ui::CheckItem {
                label: t.name.to_string(),
                detail: t.detail.to_string(),
                size: 0,
                checked: t.default_on,
            })
            .collect();
        match ui::checklist_opts(
            ui::t(
                "kazma optimize :: select maintenance tasks",
                "kazma optimize :: bakım görevlerini seç",
            ),
            items,
            false,
        )? {
            Some(s) => s,
            None => {
                println!("  {}", ui::t("cancelled.", "iptal edildi."));
                return Ok(());
            }
        }
    };

    // the name column fits the longest task name in this language
    let nw = runnable
        .iter()
        .chain(need_admin.iter())
        .map(|t| ui::width(t.name))
        .max()
        .unwrap_or(0)
        .max(34);
    ui::section(if dry {
        ui::t("Would run", "Çalıştırılacaklar")
    } else {
        ui::t("Running", "Çalıştırılıyor")
    });
    for (t, on) in runnable.iter().zip(selection) {
        if !on {
            ui::skip(&tr!("{} (skipped)", "{} (atlandı)", ui::pad(t.name, nw)));
            continue;
        }
        if dry {
            ui::ok(&format!(
                "{} {}",
                ui::pad(t.name, nw),
                ui::truncate_end(
                    t.detail,
                    ui::term_width().saturating_sub(7 + nw + 1).max(20)
                )
            ));
            continue;
        }
        ps::working(&tr!(
            "{} running...",
            "{} çalışıyor...",
            ui::pad(t.name, nw)
        ));
        match execute(&t.action) {
            Ok(()) => {
                if let Some((key, _)) = due_key(&t.action) {
                    mark_done(key);
                }
                ui::ok(&tr!("{} done      ", "{} tamam     ", ui::pad(t.name, nw)));
                oplog::record("optimize", t.name);
            }
            Err(e) => {
                ui::warn(&format!("{} {e}", ui::pad(t.name, nw)));
                oplog::record("optimize", &format!("{} FAILED {e}", t.name));
            }
        }
    }
    if !need_admin.is_empty() {
        ui::section(ui::t("Needs administrator", "Yönetici gerekli"));
        for t in &need_admin {
            ui::skip(&tr!(
                "{} (run kz from an elevated terminal)",
                "{} (kz'yi yönetici terminalinde çalıştırın)",
                ui::pad(t.name, nw)
            ));
        }
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(
            day_number("2026-11-11").unwrap() - day_number("2026-11-01").unwrap(),
            10
        );
        assert_eq!(
            day_number("2024-03-01").unwrap() - day_number("2024-02-28").unwrap(),
            2
        );
        assert!(day_number("bad").is_none());
        assert_eq!(due_key(&Action::Ps(TRIM)), Some(("trim", 7)));
        assert_eq!(
            due_key(&Action::Exec("sfc", &["/scannow"])),
            Some(("sfc", 90))
        );
    }
}
