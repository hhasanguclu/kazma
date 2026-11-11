//! `kz privacy`: clear usage traces. Browser history and cookies are opt-in
//! (cookies sign you out of websites) and skipped while the browser is running.

use crate::fsutil::{self, Stats, env_path};
use crate::{oplog, ps, tr, ui};
use std::path::PathBuf;

enum Action {
    /// Delete these files.
    Files(Vec<PathBuf>),
    /// Delete everything inside these folders.
    Dirs(Vec<PathBuf>),
    /// Run this PowerShell.
    Ps(&'static str),
}

struct Item {
    name: String,
    detail: String,
    default_on: bool,
    /// Process names (lowercase, no .exe) that must not be running.
    blocked_by: &'static [&'static str],
    action: Action,
}

const CLEAR_KEYS: &str = r#"
foreach ($k in $env:KAZMA_KEYS -split ';') {
  if (Test-Path $k) {
    (Get-Item $k).GetValueNames() | Where-Object { $_ } | ForEach-Object { Remove-ItemProperty -Path $k -Name $_ -ErrorAction SilentlyContinue }
  }
}
"#;

const RUN_MRU: &str =
    r#"$env:KAZMA_KEYS = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\RunMRU'"#;
const TYPED: &str =
    r#"$env:KAZMA_KEYS = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\TypedPaths'"#;
const WORDWHEEL: &str = r#"$env:KAZMA_KEYS = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\WordWheelQuery'"#;
const CLIPBOARD: &str =
    "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Clipboard]::Clear()";

fn chromium_profiles(user_data: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(ud) = user_data.filter(|p| p.is_dir()) else {
        return vec![];
    };
    std::fs::read_dir(&ud)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("Preferences").is_file())
                .collect()
        })
        .unwrap_or_default()
}

fn existing(v: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    v.into_iter().filter(|p| p.exists()).collect()
}

fn items() -> Vec<Item> {
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    let recent = roaming
        .as_ref()
        .map(|r| r.join("Microsoft").join("Windows").join("Recent"));
    let browsers: [(&str, Option<PathBuf>, &'static [&'static str]); 4] = [
        (
            "Chrome",
            local
                .as_ref()
                .map(|l| l.join("Google").join("Chrome").join("User Data")),
            &["chrome"],
        ),
        (
            "Edge",
            local
                .as_ref()
                .map(|l| l.join("Microsoft").join("Edge").join("User Data")),
            &["msedge"],
        ),
        (
            "Brave",
            local.as_ref().map(|l| {
                l.join("BraveSoftware")
                    .join("Brave-Browser")
                    .join("User Data")
            }),
            &["brave"],
        ),
        (
            "Vivaldi",
            local.as_ref().map(|l| l.join("Vivaldi").join("User Data")),
            &["vivaldi"],
        ),
    ];
    let firefox_profiles: Vec<PathBuf> = roaming
        .as_ref()
        .map(|r| r.join("Mozilla").join("Firefox").join("Profiles"))
        .and_then(|p| std::fs::read_dir(p).ok())
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();

    let mut v = vec![
        Item {
            name: tr!(
                "Recent files list & jump lists",
                "Son kullanılan dosyalar ve atlama listeleri"
            ),
            detail: tr!(
                "shortcuts only, your files stay",
                "yalnızca kısayollar, dosyalarınız kalır"
            ),
            default_on: true,
            blocked_by: &[],
            action: Action::Dirs(existing(recent.iter().flat_map(|r| {
                [
                    r.join("AutomaticDestinations"),
                    r.join("CustomDestinations"),
                ]
            }))),
        },
        Item {
            name: tr!("Run dialog history (Win+R)", "Çalıştır geçmişi (Win+R)"),
            detail: String::new(),
            default_on: true,
            blocked_by: &[],
            action: Action::Ps(RUN_MRU),
        },
        Item {
            name: tr!(
                "Explorer address bar history",
                "Gezgin adres çubuğu geçmişi"
            ),
            detail: String::new(),
            default_on: true,
            blocked_by: &[],
            action: Action::Ps(TYPED),
        },
        Item {
            name: tr!("Explorer search history", "Gezgin arama geçmişi"),
            detail: String::new(),
            default_on: true,
            blocked_by: &[],
            action: Action::Ps(WORDWHEEL),
        },
        Item {
            name: tr!("Clipboard", "Pano"),
            detail: String::new(),
            default_on: true,
            blocked_by: &[],
            action: Action::Ps(CLIPBOARD),
        },
    ];
    // Recent .lnk files directly in Recent (not the folders inside it)
    if let Some(r) = &recent {
        let lnks: Vec<PathBuf> = std::fs::read_dir(r)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")))
                    .collect()
            })
            .unwrap_or_default();
        v.insert(
            1,
            Item {
                name: tr!("Recent files shortcuts", "Son dosyalar kısayolları"),
                detail: format!("{} .lnk", lnks.len()),
                default_on: true,
                blocked_by: &[],
                action: Action::Files(lnks),
            },
        );
    }
    for (name, ud, procs) in browsers {
        let profiles = chromium_profiles(ud);
        if profiles.is_empty() {
            continue;
        }
        let history = existing(profiles.iter().flat_map(|p| {
            [
                "History",
                "History-journal",
                "Visited Links",
                "Top Sites",
                "Top Sites-journal",
                "Shortcuts",
            ]
            .map(|f| p.join(f))
        }));
        let cookies = existing(profiles.iter().flat_map(|p| {
            [
                p.join("Cookies"),
                p.join("Cookies-journal"),
                p.join("Network").join("Cookies"),
                p.join("Network").join("Cookies-journal"),
            ]
        }));
        v.push(Item {
            name: tr!("{name} history", "{name} geçmişi"),
            detail: tr!(
                "bookmarks and passwords stay",
                "yer imleri ve şifreler kalır"
            ),
            default_on: false,
            blocked_by: procs,
            action: Action::Files(history),
        });
        v.push(Item {
            name: tr!("{name} cookies", "{name} çerezleri"),
            detail: tr!("signs you out of websites", "sitelerden çıkış yaparsınız"),
            default_on: false,
            blocked_by: procs,
            action: Action::Files(cookies),
        });
    }
    if !firefox_profiles.is_empty() {
        let cookies = existing(
            firefox_profiles
                .iter()
                .flat_map(|p| [p.join("cookies.sqlite"), p.join("cookies.sqlite-wal")]),
        );
        let history = existing(
            firefox_profiles
                .iter()
                .flat_map(|p| [p.join("formhistory.sqlite"), p.join("sessionstore.jsonlz4")]),
        );
        v.push(Item {
            name: tr!(
                "Firefox form & session history",
                "Firefox form ve oturum geçmişi"
            ),
            detail: tr!(
                "bookmarks and passwords stay",
                "yer imleri ve şifreler kalır"
            ),
            default_on: false,
            blocked_by: &["firefox"],
            action: Action::Files(history),
        });
        v.push(Item {
            name: tr!("Firefox cookies", "Firefox çerezleri"),
            detail: tr!("signs you out of websites", "sitelerden çıkış yaparsınız"),
            default_on: false,
            blocked_by: &["firefox"],
            action: Action::Files(cookies),
        });
    }
    v.into_iter()
        .filter(|i| match &i.action {
            Action::Files(f) | Action::Dirs(f) => !f.is_empty(),
            Action::Ps(_) => cfg!(windows),
        })
        .collect()
}

fn running(names: &[&str]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_processes(sysinfo::ProcessRefreshKind::nothing()),
    )
    .ok()?;
    sys.processes().values().find_map(|p| {
        let n = p.name().to_string_lossy().to_lowercase();
        let n = n.trim_end_matches(".exe");
        names.contains(&n).then(|| n.to_string())
    })
}

fn apply(a: &Action) -> Result<Stats, String> {
    match a {
        Action::Files(f) => {
            Ok(f.iter()
                .map(|p| fsutil::delete_file(p, false))
                .fold(Stats::default(), |a, b| Stats {
                    bytes: a.bytes + b.bytes,
                    files: a.files + b.files,
                    failed: a.failed + b.failed,
                }))
        }
        Action::Dirs(d) => Ok(d
            .iter()
            .map(|p| fsutil::delete_contents(p, None, false))
            .fold(Stats::default(), |a, b| Stats {
                bytes: a.bytes + b.bytes,
                files: a.files + b.files,
                failed: a.failed + b.failed,
            })),
        Action::Ps(script) => {
            let full = if script.starts_with("$env:KAZMA_KEYS") {
                format!("{script}\n{CLEAR_KEYS}")
            } else {
                script.to_string()
            };
            match ps::powershell(&full, &[]) {
                Ok(o) if o.status.success() => Ok(Stats::default()),
                Ok(o) => Err(String::from_utf8_lossy(&o.stderr)
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("failed")
                    .trim()
                    .to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

fn size_of(a: &Action) -> u64 {
    match a {
        Action::Files(f) => f.iter().map(|p| fsutil::path_size(p)).sum(),
        Action::Dirs(d) => d.iter().map(|p| fsutil::dir_size(p)).sum(),
        Action::Ps(_) => 0,
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "privacy :: clear usage traces",
        "privacy :: kullanım izlerini temizle"
    ));
    let all = items();
    if all.is_empty() {
        ui::ok(&tr!("Nothing to clear.", "Temizlenecek bir şey yok."));
        return Ok(());
    }
    if !dry && !yes && !ui::interactive() {
        ui::warn(&tr!(
            "not an interactive terminal: re-run with --yes, or --dry-run to preview",
            "etkileşimli terminal değil: --yes ile çalıştırın ya da --dry-run ile önizleyin"
        ));
        return Ok(());
    }
    let selection: Vec<bool> = if dry || yes {
        all.iter().map(|i| i.default_on).collect()
    } else {
        let items = all
            .iter()
            .map(|i| ui::CheckItem {
                label: i.name.clone(),
                detail: i.detail.clone(),
                size: size_of(&i.action),
                checked: i.default_on,
            })
            .collect();
        match ui::checklist(
            &tr!(
                "kazma privacy :: select traces to clear",
                "kazma privacy :: temizlenecek izleri seç"
            ),
            items,
        )? {
            Some(s) => s,
            None => {
                println!("  {}", tr!("cancelled.", "iptal edildi."));
                return Ok(());
            }
        }
    };

    ui::section(
        if dry {
            tr!("Would clear", "Temizlenecekler")
        } else {
            tr!("Clearing", "Temizleniyor")
        }
        .as_str(),
    );
    for (i, on) in all.iter().zip(selection) {
        if !on {
            ui::skip(&format!("{:<44} {}", i.name, tr!("(skipped)", "(atlandı)")));
            continue;
        }
        if let Some(p) = running(i.blocked_by) {
            ui::warn(&format!(
                "{:<44} {}",
                i.name,
                tr!(
                    "({p} is running, close it first)",
                    "({p} açık, önce kapatın)"
                )
            ));
            continue;
        }
        if dry {
            ui::ok(&i.name);
            continue;
        }
        match apply(&i.action) {
            Ok(s) => {
                let note = if s.failed > 0 {
                    tr!("  ({} in use)", "  ({} kullanımda)", s.failed)
                } else {
                    String::new()
                };
                ui::ok(&format!("{}{note}", ui::pad(&i.name, 44)));
                oplog::record("privacy", &i.name);
            }
            Err(e) => ui::warn(&format!("{} {e}", ui::pad(&i.name, 44))),
        }
    }
    println!();
    Ok(())
}
