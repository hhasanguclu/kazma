//! `kz services`: Windows services you may not need, with a line on what each
//! does and when to keep it. `[x]` = allowed to run. Turning one off sets it to
//! Disabled; its earlier start type is saved and put back when it is turned on.
//! `kz services --all` lists every service, third-party ones first.

use crate::{oplog, ps, sys, tr, ui};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct Known {
    pub name: &'static str,
    pub en: &'static str,
    pub tr: &'static str,
}

/// Services that are safe to turn off for most people, and what you lose.
pub const KNOWN: &[Known] = &[
    Known {
        name: "DiagTrack",
        en: "Telemetry: sends usage and diagnostic data to Microsoft",
        tr: "Telemetri: kullanım ve tanılama verisini Microsoft'a gönderir",
    },
    Known {
        name: "dmwappushservice",
        en: "Routes push messages for telemetry and device management",
        tr: "Telemetri ve cihaz yönetimi için anlık iletileri yönlendirir",
    },
    Known {
        name: "SysMain",
        en: "Preloads often used apps into memory; off can help on HDDs, little gain on SSDs",
        tr: "Sık kullanılan uygulamaları belleğe önceden yükler; HDD'de kapatmak yardımcı olabilir",
    },
    Known {
        name: "WSearch",
        en: "Indexes files for fast search; off makes Start and Explorer search slower",
        tr: "Hızlı arama için dosyaları dizinler; kapatılırsa Başlat ve Gezgin araması yavaşlar",
    },
    Known {
        name: "Spooler",
        en: "Print queue; keep it if you print or use Print to PDF",
        tr: "Yazdırma kuyruğu; yazdırıyor ya da PDF'ye Yazdır kullanıyorsanız açık kalsın",
    },
    Known {
        name: "Fax",
        en: "Sending and receiving faxes",
        tr: "Faks gönderme ve alma",
    },
    Known {
        name: "MapsBroker",
        en: "Downloads offline maps for the Maps app",
        tr: "Haritalar uygulaması için çevrimdışı harita indirir",
    },
    Known {
        name: "lfsvc",
        en: "Location for apps; off turns location off everywhere",
        tr: "Uygulamalar için konum; kapatılırsa her yerde konum kapanır",
    },
    Known {
        name: "RetailDemo",
        en: "Store demo mode for display PCs",
        tr: "Mağazadaki teşhir bilgisayarları için demo modu",
    },
    Known {
        name: "WbioSrvc",
        en: "Fingerprint and face sign-in (Windows Hello); keep it if you use them",
        tr: "Parmak izi ve yüz ile oturum açma (Windows Hello); kullanıyorsanız açık kalsın",
    },
    Known {
        name: "XblAuthManager",
        en: "Xbox Live sign-in for games",
        tr: "Oyunlar için Xbox Live oturumu",
    },
    Known {
        name: "XblGameSave",
        en: "Xbox cloud saves for games",
        tr: "Oyunlar için Xbox bulut kayıtları",
    },
    Known {
        name: "XboxNetApiSvc",
        en: "Xbox Live networking for games",
        tr: "Oyunlar için Xbox Live ağ hizmeti",
    },
    Known {
        name: "XboxGipSvc",
        en: "Xbox accessories (controller firmware and settings)",
        tr: "Xbox aksesuarları (kumanda yazılımı ve ayarları)",
    },
    Known {
        name: "bthserv",
        en: "Bluetooth; off disconnects Bluetooth mice, headsets and keyboards",
        tr: "Bluetooth; kapatılırsa Bluetooth fare, kulaklık ve klavyeler kopar",
    },
    Known {
        name: "PhoneSvc",
        en: "Phone state for calling apps (Phone Link calls)",
        tr: "Arama uygulamaları için telefon durumu (Phone Link aramaları)",
    },
    Known {
        name: "WMPNetworkSvc",
        en: "Shares the Windows Media Player library on the network",
        tr: "Windows Media Player kitaplığını ağda paylaşır",
    },
    Known {
        name: "SharedAccess",
        en: "Internet connection sharing / mobile hotspot",
        tr: "İnternet bağlantısı paylaşımı / mobil erişim noktası",
    },
    Known {
        name: "WerSvc",
        en: "Windows Error Reporting: sends crash reports",
        tr: "Windows Hata Bildirimi: çökme raporlarını gönderir",
    },
    Known {
        name: "PcaSvc",
        en: "Program Compatibility Assistant: offers fixes for old programs",
        tr: "Program Uyumluluk Yardımcısı: eski programlar için düzeltme önerir",
    },
    Known {
        name: "wisvc",
        en: "Windows Insider program",
        tr: "Windows Insider programı",
    },
    Known {
        name: "TrkWks",
        en: "Keeps shortcuts working across network drives",
        tr: "Kısayolların ağ sürücüleri arasında çalışmasını sağlar",
    },
    Known {
        name: "SCardSvr",
        en: "Smart card readers (ID cards, some company logins)",
        tr: "Akıllı kart okuyucular (kimlik kartı, bazı şirket girişleri)",
    },
    Known {
        name: "RemoteRegistry",
        en: "Lets other PCs edit this registry; off by default, keep it off",
        tr: "Başka bilgisayarların bu kayıt defterini düzenlemesine izin verir; kapalı kalsın",
    },
    Known {
        name: "edgeupdate",
        en: "Microsoft Edge updater (also runs as a scheduled task)",
        tr: "Microsoft Edge güncelleyicisi (zamanlanmış görev olarak da çalışır)",
    },
    Known {
        name: "edgeupdatem",
        en: "Microsoft Edge updater (on demand)",
        tr: "Microsoft Edge güncelleyicisi (gerektiğinde)",
    },
    Known {
        name: "gupdate",
        en: "Google updater for Chrome and other Google apps",
        tr: "Chrome ve diğer Google uygulamaları için Google güncelleyicisi",
    },
    Known {
        name: "gupdatem",
        en: "Google updater (on demand)",
        tr: "Google güncelleyicisi (gerektiğinde)",
    },
    Known {
        name: "AdobeARMservice",
        en: "Adobe Acrobat updater",
        tr: "Adobe Acrobat güncelleyicisi",
    },
];

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Svc {
    pub name: String,
    pub display: String,
    pub state: String,
    /// Auto, Manual, Disabled, Boot, System
    pub start: String,
    #[serde(default)]
    pub delayed: bool,
    #[serde(default)]
    pub path: String,
}

const READ: &str = r#"
@(Get-CimInstance Win32_Service | ForEach-Object {
  [pscustomobject]@{ Name = $_.Name; Display = [string]$_.DisplayName; State = [string]$_.State
    Start = [string]$_.StartMode; Delayed = [bool]$_.DelayedAutoStart; Path = [string]$_.PathName }
}) | ConvertTo-Json -Compress
"#;

/// Set a start type: Auto, AutoDelayed, Manual, Disabled. Stops it when disabling.
const SET: &str = r#"
$n = $env:KAZMA_SVC
switch ($env:KAZMA_START) {
  'AutoDelayed' { $o = & sc.exe config $n start= delayed-auto }
  'Auto'        { $o = & sc.exe config $n start= auto }
  'Manual'      { $o = & sc.exe config $n start= demand }
  'Disabled'    { $o = & sc.exe config $n start= disabled }
}
if ($LASTEXITCODE -ne 0) { throw (($o | Where-Object { $_ }) -join ' ') }
if ($env:KAZMA_START -eq 'Disabled') { Stop-Service -Name $n -Force -NoWait -ErrorAction SilentlyContinue }
"#;

fn backup_file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("services-backup.json"))
}

fn load_backup() -> BTreeMap<String, String> {
    backup_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_backup(m: &BTreeMap<String, String>) -> anyhow::Result<()> {
    let f = backup_file().ok_or_else(|| anyhow::anyhow!("no data folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(f, serde_json::to_string_pretty(m)?)?;
    Ok(())
}

/// The start type as `SET` takes it.
pub fn start_kind(s: &Svc) -> &'static str {
    match (s.start.as_str(), s.delayed) {
        ("Auto", true) => "AutoDelayed",
        ("Auto", false) => "Auto",
        ("Disabled", _) => "Disabled",
        _ => "Manual",
    }
}

fn start_label(kind: &str) -> String {
    match kind {
        "Auto" => tr!("automatic", "otomatik"),
        "AutoDelayed" => tr!("automatic (delayed)", "otomatik (gecikmeli)"),
        "Disabled" => tr!("disabled", "devre dışı"),
        _ => tr!("manual", "elle"),
    }
}

/// Not from Windows: its program is outside the Windows folder.
pub fn third_party(s: &Svc) -> bool {
    let p = s.path.to_lowercase();
    !p.is_empty()
        && !p.contains(r"\windows\")
        && !p.contains("%systemroot%")
        && !p.starts_with(r"\systemroot")
}

pub fn explain(name: &str) -> Option<&'static str> {
    KNOWN
        .iter()
        .find(|k| k.name.eq_ignore_ascii_case(name))
        .map(|k| ui::t(k.en, k.tr))
}

fn read() -> anyhow::Result<Vec<Svc>> {
    ps::powershell_json(READ, &[])
}

fn print_all(all: &[Svc]) {
    let mut v: Vec<&Svc> = all.iter().collect();
    v.sort_by_key(|s| {
        (
            !third_party(s),
            s.state != "Running",
            s.display.to_lowercase(),
        )
    });
    let nw = 40;
    let sw = ["Auto", "AutoDelayed", "Manual", "Disabled"]
        .iter()
        .map(|k| ui::width(&start_label(k)))
        .max()
        .unwrap_or(10);
    let mut header = None;
    for s in v {
        let tp = third_party(s);
        if header != Some(tp) {
            header = Some(tp);
            ui::section(&if tp {
                tr!("Third-party services", "Üçüncü taraf hizmetler")
            } else {
                tr!("Windows services", "Windows hizmetleri")
            });
        }
        let room = ui::term_width().saturating_sub(7 + nw + 1 + sw + 2);
        let what = explain(&s.name)
            .map(str::to_string)
            .unwrap_or_else(|| s.name.clone());
        let line = format!(
            "{} {}  {}",
            ui::fit(&s.display, nw),
            ui::pad(&start_label(start_kind(s)), sw),
            ui::truncate_end(&what, room.max(10))
        );
        if s.state == "Running" {
            ui::ok(&line);
        } else {
            ui::skip(&line);
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "[ok] running   [--] stopped   change the optional ones with: kz services",
            "[ok] çalışıyor   [--] durmuş   isteğe bağlı olanları değiştirmek için: kz services"
        )
    );
}

pub fn run(all: bool, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "services :: Windows services you may not need",
        "services :: gerekmeyebilecek Windows hizmetleri"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz services works on Windows only.",
            "kz services yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    ps::working(&tr!("reading services...", "hizmetler okunuyor..."));
    let services = read()?;
    println!("{:60}", "");
    if all {
        print_all(&services);
        return Ok(());
    }
    // the known optional services present on this PC
    let here: Vec<(&Known, &Svc)> = KNOWN
        .iter()
        .filter_map(|k| {
            services
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(k.name))
                .map(|s| (k, s))
        })
        .collect();
    let backup = load_backup();

    if dry || !ui::interactive() {
        let nw = here
            .iter()
            .map(|(_, s)| ui::width(&s.display))
            .max()
            .unwrap_or(0)
            .clamp(24, 44);
        for (k, s) in &here {
            let room = ui::term_width().saturating_sub(7 + nw + 2);
            let line = format!(
                "{}  {}",
                ui::fit(&s.display, nw),
                ui::truncate_end(ui::t(k.en, k.tr), room.max(10))
            );
            if start_kind(s) == "Disabled" {
                ui::skip(&line)
            } else {
                ui::ok(&line)
            }
        }
        println!(
            "\n  {}\n",
            tr!(
                "[ok] allowed to run   [--] disabled   kz services --all lists every service",
                "[ok] çalışabilir   [--] devre dışı   tüm hizmetler için: kz services --all"
            )
        );
        return Ok(());
    }

    let items = here
        .iter()
        .map(|(_, s)| ui::CheckItem {
            label: s.display.clone(),
            detail: format!(
                "{}  {}",
                start_label(start_kind(s)),
                if s.state == "Running" {
                    tr!("running", "çalışıyor")
                } else {
                    String::new()
                }
            ),
            size: 0,
            checked: start_kind(s) != "Disabled",
        })
        .collect();
    let notes: Vec<String> = here
        .iter()
        .map(|(k, s)| format!("{}: {}", s.name, ui::t(k.en, k.tr)))
        .collect();
    let Some(sel) = ui::checklist_explained(
        &tr!(
            "kazma services :: [x] = allowed to run, enter applies the changes",
            "kazma services :: [x] = çalışabilir, enter değişiklikleri uygular"
        ),
        items,
        &notes,
    )?
    else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };
    let changes: Vec<(&Svc, bool)> = here
        .iter()
        .zip(sel)
        .filter(|((_, s), on)| (start_kind(s) != "Disabled") != *on)
        .map(|((_, s), on)| (*s, on))
        .collect();
    if changes.is_empty() {
        ui::ok(&tr!("Nothing to change.", "Değiştirilecek bir şey yok."));
        return Ok(());
    }
    if !sys::is_admin() {
        anyhow::bail!(
            "{}",
            tr!(
                "changing services needs an elevated terminal (Run as administrator)",
                "hizmetleri değiştirmek için yönetici terminali gerekir (Yönetici olarak çalıştır)"
            )
        );
    }
    ui::section(&tr!("Applying", "Uygulanıyor"));
    let mut backup = backup;
    for (s, on) in changes {
        // off: remember how it started; on: put that back (Manual if unknown)
        let target = if on {
            backup
                .get(&s.name)
                .cloned()
                .unwrap_or_else(|| "Manual".into())
        } else {
            backup
                .entry(s.name.clone())
                .or_insert_with(|| start_kind(s).to_string());
            save_backup(&backup)?;
            "Disabled".into()
        };
        let label = format!("{}  ->  {}", s.display, start_label(&target));
        match ps::powershell(SET, &[("KAZMA_SVC", &s.name), ("KAZMA_START", &target)]) {
            Ok(o) if o.status.success() => {
                ui::ok(&label);
                oplog::record("services", &format!("{} {}", s.name, target));
                if on {
                    backup.remove(&s.name);
                    save_backup(&backup)?;
                }
            }
            Ok(o) => ui::warn(&format!(
                "{label}  ({})",
                ps::first_line(&String::from_utf8_lossy(&o.stderr))
            )),
            Err(e) => ui::warn(&format!("{label}  ({e})")),
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "turned-off services keep their old start type in services-backup.json; turn them on again here.",
            "kapatılan hizmetlerin eski başlangıç türü services-backup.json'da saklanır; buradan yeniden açabilirsiniz."
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(start: &str, delayed: bool, path: &str) -> Svc {
        Svc {
            name: "x".into(),
            display: "X".into(),
            state: "Running".into(),
            start: start.into(),
            delayed,
            path: path.into(),
        }
    }

    #[test]
    fn start_kinds() {
        assert_eq!(start_kind(&svc("Auto", true, "")), "AutoDelayed");
        assert_eq!(start_kind(&svc("Auto", false, "")), "Auto");
        assert_eq!(start_kind(&svc("Manual", false, "")), "Manual");
        assert_eq!(start_kind(&svc("Disabled", false, "")), "Disabled");
    }

    #[test]
    fn third_party_by_path() {
        assert!(!third_party(&svc(
            "Auto",
            false,
            r"C:\WINDOWS\system32\svchost.exe -k netsvcs"
        )));
        assert!(third_party(&svc(
            "Auto",
            false,
            r#""C:\Program Files\Steam\steamservice.exe""#
        )));
        assert!(!third_party(&svc(
            "Auto",
            false,
            r"\SystemRoot\System32\drivers\x.sys"
        )));
    }

    #[test]
    fn known_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for k in KNOWN {
            assert!(seen.insert(k.name.to_lowercase()), "{}", k.name);
        }
    }
}
