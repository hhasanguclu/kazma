//! `kz optimize`: safe Windows maintenance tasks.

use crate::{oplog, ps, sys, tr, ui};
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

    ui::section(if dry {
        ui::t("Would run", "Çalıştırılacaklar")
    } else {
        ui::t("Running", "Çalıştırılıyor")
    });
    for (t, on) in runnable.iter().zip(selection) {
        if !on {
            ui::skip(&tr!("{:<34} (skipped)", "{:<34} (atlandı)", t.name));
            continue;
        }
        if dry {
            ui::ok(&format!("{:<34} {}", t.name, t.detail));
            continue;
        }
        ps::working(&tr!("{:<34} running...", "{:<34} çalışıyor...", t.name));
        match execute(&t.action) {
            Ok(()) => {
                ui::ok(&tr!("{:<34} done      ", "{:<34} tamam     ", t.name));
                oplog::record("optimize", t.name);
            }
            Err(e) => {
                ui::warn(&format!("{:<34} {e}", t.name));
                oplog::record("optimize", &format!("{} FAILED {e}", t.name));
            }
        }
    }
    if !need_admin.is_empty() {
        ui::section(ui::t("Needs administrator", "Yönetici gerekli"));
        for t in &need_admin {
            ui::skip(&tr!(
                "{:<34} (run kz from an elevated terminal)",
                "{:<34} (kz'yi yönetici terminalinde çalıştırın)",
                t.name
            ));
        }
    }
    println!();
    Ok(())
}
