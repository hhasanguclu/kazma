//! `kz features`: turn Windows optional features on and off (WSL, Hyper-V,
//! Windows Sandbox, .NET 3.5...), from a short list with a line on each.

use crate::{oplog, ps, sys, tr, ui};
use serde::Deserialize;

pub struct Feature {
    /// the name DISM / PowerShell uses
    pub name: &'static str,
    pub en: &'static str,
    pub tr: &'static str,
    pub detail_en: &'static str,
    pub detail_tr: &'static str,
}

pub const FEATURES: &[Feature] = &[
    Feature {
        name: "Microsoft-Windows-Subsystem-Linux",
        en: "Windows Subsystem for Linux",
        tr: "Linux için Windows Alt Sistemi",
        detail_en: "run Linux tools and distributions (WSL); also turn on the virtual machine platform",
        detail_tr: "Linux araçları ve dağıtımları çalıştırın (WSL); sanal makine platformunu da açın",
    },
    Feature {
        name: "VirtualMachinePlatform",
        en: "Virtual Machine Platform",
        tr: "Sanal Makine Platformu",
        detail_en: "needed by WSL 2 and the Android subsystem",
        detail_tr: "WSL 2 ve Android alt sistemi için gerekli",
    },
    Feature {
        name: "Microsoft-Hyper-V-All",
        en: "Hyper-V",
        tr: "Hyper-V",
        detail_en: "Microsoft's virtual machines (Pro, Enterprise, Education)",
        detail_tr: "Microsoft'un sanal makineleri (Pro, Enterprise, Education)",
    },
    Feature {
        name: "Containers-DisposableClientVM",
        en: "Windows Sandbox",
        tr: "Windows Korumalı Alanı",
        detail_en: "a throwaway desktop for trying unknown programs safely",
        detail_tr: "bilinmeyen programları güvenle denemek için kullan-at bir masaüstü",
    },
    Feature {
        name: "NetFx3",
        en: ".NET Framework 3.5",
        tr: ".NET Framework 3.5",
        detail_en: "older programs and games ask for it (downloaded from Windows Update)",
        detail_tr: "eski programlar ve oyunlar ister (Windows Update'ten indirilir)",
    },
    Feature {
        name: "TelnetClient",
        en: "Telnet client",
        tr: "Telnet istemcisi",
        detail_en: "the telnet command, handy to test whether a port answers",
        detail_tr: "telnet komutu; bir portun yanıt verip vermediğini denemek için",
    },
    Feature {
        name: "TFTP",
        en: "TFTP client",
        tr: "TFTP istemcisi",
        detail_en: "for flashing routers and network devices",
        detail_tr: "modem ve ağ cihazlarına yazılım yüklemek için",
    },
    Feature {
        name: "SMB1Protocol",
        en: "SMB 1.0 file sharing (old, insecure)",
        tr: "SMB 1.0 dosya paylaşımı (eski, güvensiz)",
        detail_en: "only for very old NAS boxes; keep it off otherwise",
        detail_tr: "yalnızca çok eski NAS cihazları için; değilse kapalı tutun",
    },
    Feature {
        name: "MicrosoftWindowsPowerShellV2Root",
        en: "PowerShell 2.0 (old)",
        tr: "PowerShell 2.0 (eski)",
        detail_en: "an old engine attackers use to dodge logging; safe to turn off",
        detail_tr: "saldırganların günlüklemeden kaçmak için kullandığı eski motor; kapatılabilir",
    },
    Feature {
        name: "Printing-PrintToPDFServices-Features",
        en: "Microsoft Print to PDF",
        tr: "Microsoft PDF'ye Yazdır",
        detail_en: "the PDF printer in every print dialog",
        detail_tr: "her yazdırma penceresindeki PDF yazıcısı",
    },
    Feature {
        name: "Microsoft-Windows-Printing-XPSServices-Features",
        en: "XPS printer",
        tr: "XPS yazıcısı",
        detail_en: "the old XPS document printer; rarely needed",
        detail_tr: "eski XPS belge yazıcısı; nadiren gerekir",
    },
    Feature {
        name: "WorkFolders-Client",
        en: "Work Folders",
        tr: "İş Klasörleri",
        detail_en: "file sync with a company server; not needed at home",
        detail_tr: "şirket sunucusuyla dosya eşitleme; evde gerekmez",
    },
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    name: String,
    on: bool,
}

const READ: &str = r#"
$names = $env:KAZMA_FEATURES -split ';'
@(Get-WindowsOptionalFeature -Online -ErrorAction Stop | Where-Object { $names -contains $_.FeatureName } | ForEach-Object {
  [pscustomobject]@{ Name = $_.FeatureName; On = ([string]$_.State -match '^Enabled') }
}) | ConvertTo-Json -Compress
"#;

const SET: &str = r#"
if ($env:KAZMA_ON -eq '1') {
  $r = Enable-WindowsOptionalFeature -Online -FeatureName $env:KAZMA_FEATURE -All -NoRestart -ErrorAction Stop
} else {
  $r = Disable-WindowsOptionalFeature -Online -FeatureName $env:KAZMA_FEATURE -NoRestart -ErrorAction Stop
}
if ($r.RestartNeeded) { 'RESTART' }
"#;

fn name(f: &Feature) -> &'static str {
    ui::t(f.en, f.tr)
}

pub fn run(list: bool, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "features :: Windows optional features",
        "features :: Windows isteğe bağlı özellikleri"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz features works on Windows only.",
            "kz features yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    if !sys::is_admin() {
        ui::warn(&tr!(
            "Windows only shows its optional features to an elevated terminal (Run as administrator).",
            "Windows isteğe bağlı özelliklerini yalnızca yönetici terminaline gösterir (Yönetici olarak çalıştır)."
        ));
        println!();
        return Ok(());
    }
    ps::working(&tr!(
        "reading optional features (10-20 s)...",
        "isteğe bağlı özellikler okunuyor (10-20 sn)..."
    ));
    let names: Vec<&str> = FEATURES.iter().map(|f| f.name).collect();
    let states: Vec<State> = ps::powershell_json(READ, &[("KAZMA_FEATURES", &names.join(";"))])?;
    println!("{:60}", "");
    // only what this edition of Windows has (Hyper-V is missing on Home)
    let here: Vec<(&Feature, bool)> = FEATURES
        .iter()
        .filter_map(|f| {
            states
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(f.name))
                .map(|s| (f, s.on))
        })
        .collect();

    if list || dry || !ui::interactive() {
        let nw = here
            .iter()
            .map(|(f, _)| ui::width(name(f)))
            .max()
            .unwrap_or(0)
            .max(30);
        for (f, on) in &here {
            let line = format!(
                "{} {}",
                ui::pad(name(f), nw),
                ui::truncate_end(
                    ui::t(f.detail_en, f.detail_tr),
                    ui::term_width().saturating_sub(nw + 9).max(20)
                )
            );
            if *on { ui::ok(&line) } else { ui::skip(&line) }
        }
        println!(
            "\n  {}\n",
            tr!(
                "[ok] on   [--] off   change them with: kz features",
                "[ok] açık   [--] kapalı   değiştirmek için: kz features"
            )
        );
        return Ok(());
    }

    let items = here
        .iter()
        .map(|(f, on)| ui::CheckItem {
            label: name(f).to_string(),
            detail: f.name.to_string(),
            size: 0,
            checked: *on,
        })
        .collect();
    let notes: Vec<String> = here
        .iter()
        .map(|(f, _)| ui::t(f.detail_en, f.detail_tr).to_string())
        .collect();
    let Some(sel) = ui::checklist_explained(
        &tr!(
            "kazma features :: [x] = on, enter applies the changes",
            "kazma features :: [x] = açık, enter değişiklikleri uygular"
        ),
        items,
        &notes,
    )?
    else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };
    let changes: Vec<(&Feature, bool)> = here
        .iter()
        .zip(sel)
        .filter(|((_, was), now)| was != now)
        .map(|((f, _), now)| (*f, now))
        .collect();
    if changes.is_empty() {
        ui::ok(&tr!("Nothing to change.", "Değiştirilecek bir şey yok."));
        return Ok(());
    }
    ui::section(&tr!(
        "Applying (this can take a few minutes)",
        "Uygulanıyor (birkaç dakika sürebilir)"
    ));
    let mut restart = false;
    for (f, on) in changes {
        let label = format!(
            "{} {}",
            if on {
                tr!("[on] ", "[aç] ")
            } else {
                tr!("[off]", "[kapat]")
            },
            name(f)
        );
        ps::working(&format!("{label} ..."));
        match ps::powershell(
            SET,
            &[
                ("KAZMA_FEATURE", f.name),
                ("KAZMA_ON", if on { "1" } else { "0" }),
            ],
        ) {
            Ok(o) if o.status.success() => {
                restart |= String::from_utf8_lossy(&o.stdout).contains("RESTART");
                ui::ok(&format!("{label}          "));
                oplog::record(
                    "features",
                    &format!("{} {}", if on { "on" } else { "off" }, f.name),
                );
            }
            Ok(o) => ui::warn(&format!(
                "{label}  ({})",
                ps::first_line(&String::from_utf8_lossy(&o.stderr))
            )),
            Err(e) => ui::warn(&format!("{label}  ({e})")),
        }
    }
    if restart {
        println!(
            "\n  {}",
            tr!(
                "restart Windows to finish the changes.",
                "değişikliklerin tamamlanması için Windows'u yeniden başlatın."
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
    fn names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for f in FEATURES {
            assert!(seen.insert(f.name), "{}", f.name);
            assert!(!f.name.contains(' '));
        }
    }
}
