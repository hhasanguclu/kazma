//! `kz debloat`: remove preinstalled promo apps (only from a fixed, known list) and
//! turn off ads / suggestions in Windows. Everything is per-user and reversible:
//! apps can be reinstalled from the Microsoft Store, tweaks undone with --restore.

use crate::{oplog, ps, tr, ui};
use serde::Deserialize;

/// (package name pattern, friendly name, selected by default). `*` matches anything.
const APPS: &[(&str, &str, bool)] = &[
    ("king.com.*", "Candy Crush & King games", true),
    ("*TikTok*", "TikTok", true),
    ("*Disney*", "Disney+", true),
    ("*Facebook*", "Facebook", true),
    ("*Instagram*", "Instagram", true),
    ("*Twitter*", "Twitter / X", true),
    ("*LinkedIn*", "LinkedIn", true),
    ("*AmazonPrimeVideo*", "Prime Video", true),
    ("Microsoft.BingNews", "Microsoft News", true),
    ("Microsoft.BingSearch", "Bing Search", true),
    ("Microsoft.GetHelp", "Get Help", true),
    ("Microsoft.Getstarted", "Tips", true),
    (
        "Microsoft.MicrosoftOfficeHub",
        "Office hub (Microsoft 365 ads)",
        true,
    ),
    (
        "Microsoft.MicrosoftSolitaireCollection",
        "Solitaire Collection",
        true,
    ),
    ("Microsoft.People", "People", true),
    ("Microsoft.WindowsFeedbackHub", "Feedback Hub", true),
    ("Microsoft.SkypeApp", "Skype", true),
    (
        "Microsoft.MixedReality.Portal",
        "Mixed Reality Portal",
        true,
    ),
    ("Microsoft.Microsoft3DViewer", "3D Viewer", true),
    ("Microsoft.Print3D", "Print 3D", true),
    ("Microsoft.3DBuilder", "3D Builder", true),
    ("Microsoft.549981C3F5F10", "Cortana", true),
    ("Microsoft.Wallet", "Wallet", true),
    ("Microsoft.OneConnect", "Mobile Plans", true),
    ("Microsoft.Messaging", "Messaging", true),
    ("Microsoft.BingWeather", "Weather", false),
    ("Microsoft.WindowsMaps", "Maps", false),
    ("Microsoft.ZuneVideo", "Movies & TV", false),
    ("Microsoft.Todos", "Microsoft To Do", false),
    ("Microsoft.YourPhone", "Phone Link", false),
    ("Microsoft.GamingApp", "Xbox app", false),
    ("Microsoft.PowerAutomateDesktop", "Power Automate", false),
    ("Clipchamp.Clipchamp", "Clipchamp", false),
    ("MicrosoftTeams", "Teams (personal)", false),
    ("SpotifyAB.SpotifyMusic", "Spotify", false),
    ("*Netflix*", "Netflix", false),
];

/// A registry value to set. `on: None` means "delete the value" on restore.
struct Val {
    path: &'static str,
    name: &'static str,
    off: u32,
    on: Option<u32>,
}

struct Tweak {
    en: &'static str,
    tr: &'static str,
    values: &'static [Val],
}

const CDM: &str = r"HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const ADV: &str = r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";

const TWEAKS: &[Tweak] = &[
    Tweak {
        en: "Start menu suggestions & ads",
        tr: "Başlat menüsü önerileri ve reklamları",
        values: &[
            Val {
                path: CDM,
                name: "SystemPaneSuggestionsEnabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "SubscribedContent-338388Enabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: ADV,
                name: "Start_IrisRecommendations",
                off: 0,
                on: Some(1),
            },
        ],
    },
    Tweak {
        en: "Silent install of promoted apps",
        tr: "Önerilen uygulamaların sessiz kurulumu",
        values: &[
            Val {
                path: CDM,
                name: "SilentInstalledAppsEnabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "OemPreInstalledAppsEnabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "PreInstalledAppsEnabled",
                off: 0,
                on: Some(1),
            },
        ],
    },
    Tweak {
        en: "Tips, tricks & suggestions notifications",
        tr: "İpucu ve öneri bildirimleri",
        values: &[
            Val {
                path: CDM,
                name: "SoftLandingEnabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "SubscribedContent-338389Enabled",
                off: 0,
                on: Some(1),
            },
        ],
    },
    Tweak {
        en: "Lock screen fun facts & ads",
        tr: "Kilit ekranı bilgi ve reklamları",
        values: &[
            Val {
                path: CDM,
                name: "RotatingLockScreenOverlayEnabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "SubscribedContent-338387Enabled",
                off: 0,
                on: Some(1),
            },
        ],
    },
    Tweak {
        en: "Suggested content in Settings",
        tr: "Ayarlar'daki önerilen içerik",
        values: &[
            Val {
                path: CDM,
                name: "SubscribedContent-338393Enabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "SubscribedContent-353694Enabled",
                off: 0,
                on: Some(1),
            },
            Val {
                path: CDM,
                name: "SubscribedContent-353696Enabled",
                off: 0,
                on: Some(1),
            },
        ],
    },
    Tweak {
        en: "OneDrive / Microsoft 365 ads in Explorer",
        tr: "Gezgin'deki OneDrive / Microsoft 365 reklamları",
        values: &[Val {
            path: ADV,
            name: "ShowSyncProviderNotifications",
            off: 0,
            on: Some(1),
        }],
    },
    Tweak {
        en: "Advertising ID for apps",
        tr: "Uygulamalar için reklam kimliği",
        values: &[Val {
            path: r"HKCU:\Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
            name: "Enabled",
            off: 0,
            on: Some(1),
        }],
    },
    Tweak {
        en: "Tailored ads from diagnostic data",
        tr: "Tanılama verisine göre kişisel reklam",
        values: &[Val {
            path: r"HKCU:\Software\Microsoft\Windows\CurrentVersion\Privacy",
            name: "TailoredExperiencesWithDiagnosticDataEnabled",
            off: 0,
            on: Some(1),
        }],
    },
    Tweak {
        en: "Bing web results in Start search",
        tr: "Başlat aramasında Bing web sonuçları",
        values: &[Val {
            path: r"HKCU:\Software\Policies\Microsoft\Windows\Explorer",
            name: "DisableSearchBoxSuggestions",
            off: 1,
            on: None,
        }],
    },
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Pkg {
    name: String,
    full: String,
}

fn glob(pattern: &str, name: &str) -> bool {
    let (p, n) = (pattern.to_lowercase(), name.to_lowercase());
    match (p.strip_prefix('*'), p.ends_with('*')) {
        (Some(mid), true) => n.contains(mid.trim_end_matches('*')),
        (Some(suffix), false) => n.ends_with(suffix),
        (None, true) => n.starts_with(p.trim_end_matches('*')),
        (None, false) => n == p,
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// PowerShell that prints "1" per tweak whose values are all at their "off" value.
fn read_tweaks_script() -> String {
    let mut s = String::from("$r = @()\n");
    for t in TWEAKS {
        let conds: Vec<String> = t
            .values
            .iter()
            .map(|v| format!("((Get-ItemProperty -Path {} -Name {} -ErrorAction SilentlyContinue).{} -eq {})", ps_quote(v.path), ps_quote(v.name), ps_quote(v.name), v.off))
            .collect();
        s.push_str(&format!("$r += [int]({})\n", conds.join(" -and ")));
    }
    s.push_str("ConvertTo-Json -InputObject @($r) -Compress\n");
    s
}

fn write_tweak_script(t: &Tweak, restore: bool) -> String {
    let mut s = String::new();
    for v in t.values {
        let (p, n) = (ps_quote(v.path), ps_quote(v.name));
        match (restore, v.on) {
            (true, None) => s.push_str(&format!("Remove-ItemProperty -Path {p} -Name {n} -ErrorAction SilentlyContinue\n")),
            (true, Some(on)) => s.push_str(&format!("if (-not (Test-Path {p})) {{ New-Item -Path {p} -Force | Out-Null }}\nNew-ItemProperty -Path {p} -Name {n} -Value {on} -PropertyType DWord -Force | Out-Null\n")),
            (false, _) => s.push_str(&format!("if (-not (Test-Path {p})) {{ New-Item -Path {p} -Force | Out-Null }}\nNew-ItemProperty -Path {p} -Name {n} -Value {} -PropertyType DWord -Force | Out-Null\n", v.off)),
        }
    }
    s
}

fn tweak_name(t: &Tweak) -> &'static str {
    ui::t(t.en, t.tr)
}

pub fn run(dry: bool, restore: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "debloat :: remove promo apps, turn off ads",
        "debloat :: tanıtım uygulamalarını kaldır, reklamları kapat"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz debloat works on Windows only.",
            "kz debloat yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }

    if restore {
        ui::section(&tr!(
            "Restoring Windows defaults",
            "Windows varsayılanları geri yükleniyor"
        ));
        for t in TWEAKS {
            match ps::powershell(&write_tweak_script(t, true), &[]) {
                Ok(o) if o.status.success() => ui::ok(tweak_name(t)),
                _ => ui::warn(tweak_name(t)),
            }
        }
        oplog::record("debloat", "restored tweaks");
        println!(
            "\n  {}\n",
            tr!(
                "removed apps can be reinstalled from the Microsoft Store.",
                "kaldırılan uygulamalar Microsoft Store'dan yeniden kurulabilir."
            )
        );
        return Ok(());
    }

    ps::working(&tr!(
        "reading installed apps and settings...",
        "yüklü uygulamalar ve ayarlar okunuyor..."
    ));
    let pkgs: Vec<Pkg> = ps::powershell_json(
        "@(Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.NonRemovable } | ForEach-Object { [pscustomobject]@{ Name = $_.Name; Full = $_.PackageFullName } }) | ConvertTo-Json -Compress",
        &[],
    )?;
    let applied: Vec<i64> = ps::powershell_json(&read_tweaks_script(), &[]).unwrap_or_default();
    println!("{:60}", "");

    // (kind, index, label, detail, default)
    let mut rows: Vec<(bool, usize, String, String, bool)> = vec![];
    for (i, t) in TWEAKS.iter().enumerate() {
        if applied.get(i).copied().unwrap_or(0) == 1 {
            continue; // already off
        }
        rows.push((
            false,
            i,
            format!("{} {}", tr!("[off]", "[kapat]"), tweak_name(t)),
            tr!("setting", "ayar"),
            true,
        ));
    }
    for (i, p) in pkgs.iter().enumerate() {
        if let Some((_, friendly, def)) = APPS.iter().find(|(pat, _, _)| glob(pat, &p.name)) {
            rows.push((
                true,
                i,
                format!("{} {friendly}", tr!("[app]", "[uyg]")),
                p.name.clone(),
                *def,
            ));
        }
    }
    if rows.is_empty() {
        ui::ok(&tr!(
            "Nothing to remove, ads already off.",
            "Kaldırılacak bir şey yok, reklamlar zaten kapalı."
        ));
        return Ok(());
    }

    if dry || !ui::interactive() {
        ui::section(&tr!("Would change", "Değiştirilecekler"));
        for (_, _, label, detail, def) in &rows {
            let line = format!("{label:<52} {detail}");
            if *def { ui::ok(&line) } else { ui::skip(&line) }
        }
        println!();
        return Ok(());
    }

    let items = rows
        .iter()
        .map(|(_, _, l, d, def)| ui::CheckItem {
            label: l.clone(),
            detail: d.clone(),
            size: 0,
            checked: *def,
        })
        .collect();
    let Some(sel) = ui::checklist_opts(
        &tr!(
            "kazma debloat :: select what to remove / turn off",
            "kazma debloat :: kaldırılacak / kapatılacakları seç"
        ),
        items,
        false,
    )?
    else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };

    ui::section(&tr!("Applying", "Uygulanıyor"));
    for ((is_app, i, label, _, _), on) in rows.iter().zip(sel) {
        if !on {
            continue;
        }
        let result = if *is_app {
            ps::powershell(
                "Remove-AppxPackage -Package $env:KAZMA_PKG",
                &[("KAZMA_PKG", &pkgs[*i].full)],
            )
        } else {
            ps::powershell(&write_tweak_script(&TWEAKS[*i], false), &[])
        };
        match result {
            Ok(o) if o.status.success() => {
                ui::ok(label);
                oplog::record("debloat", label);
            }
            Ok(o) => ui::warn(&format!(
                "{label}  ({})",
                String::from_utf8_lossy(&o.stderr)
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("failed")
                    .trim()
            )),
            Err(e) => ui::warn(&format!("{label}  ({e})")),
        }
    }
    println!(
        "\n  {}",
        tr!(
            "undo the settings any time with: kz debloat --restore",
            "ayarları geri almak için: kz debloat --restore"
        )
    );
    println!(
        "  {}\n",
        tr!(
            "sign out and back in for Start menu changes to show.",
            "Başlat menüsü değişiklikleri için oturumu kapatıp açın."
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(glob("king.com.*", "king.com.CandyCrushSaga"));
        assert!(glob("*TikTok*", "BytedancePte.Ltd.TikTok"));
        assert!(glob("Microsoft.BingNews", "microsoft.bingnews"));
        assert!(!glob("Microsoft.BingNews", "Microsoft.BingNewsExtra"));
        assert!(!glob("king.com.*", "Microsoft.WindowsCalculator"));
        // nothing on the list may match core apps
        for core in [
            "Microsoft.WindowsStore",
            "Microsoft.Windows.Photos",
            "Microsoft.WindowsCalculator",
            "Microsoft.WindowsTerminal",
            "Microsoft.WindowsNotepad",
        ] {
            assert!(
                !APPS.iter().any(|(p, _, _)| glob(p, core)),
                "{core} would be removed"
            );
        }
    }

    #[test]
    fn scripts_quote_safely() {
        assert_eq!(ps_quote("a'b"), "'a''b'");
        assert!(read_tweaks_script().contains("ConvertTo-Json"));
        assert!(write_tweak_script(&TWEAKS[8], true).contains("Remove-ItemProperty"));
    }
}
