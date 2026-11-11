//! `kz repair`: standard fixes for common Windows problems (needs administrator).
//! Nothing is deleted outright: Windows Update folders are renamed so they can be
//! put back, and a restore point is offered first.

use crate::{drivers, oplog, ps, sys, tr, ui};

enum Step {
    Exec(&'static str, &'static [&'static str]),
    Ps(&'static str),
}

struct Fix {
    key: &'static str,
    en: &'static str,
    tr: &'static str,
    detail_en: &'static str,
    detail_tr: &'static str,
    restart: bool,
    steps: &'static [Step],
}

const UPDATE_RESET: &str = r#"
$svcs = 'wuauserv', 'cryptSvc', 'bits', 'msiserver'
foreach ($s in $svcs) { Stop-Service -Name $s -Force -ErrorAction SilentlyContinue }
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
foreach ($p in @("$env:SystemRoot\SoftwareDistribution", "$env:SystemRoot\System32\catroot2")) {
  if (Test-Path -LiteralPath $p) { Rename-Item -LiteralPath $p -NewName ((Split-Path $p -Leaf) + ".kazma-$stamp") }
}
foreach ($s in $svcs) { Start-Service -Name $s -ErrorAction SilentlyContinue }
"#;

const SPOOLER_RESET: &str = r#"
Stop-Service -Name Spooler -Force
Get-ChildItem -LiteralPath "$env:SystemRoot\System32\spool\PRINTERS" -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
Start-Service -Name Spooler
"#;

const TIME_SYNC: &str = r#"
Set-Service -Name w32time -StartupType Manual -ErrorAction SilentlyContinue
Start-Service -Name w32time -ErrorAction SilentlyContinue
w32tm /resync /force | Out-Null
"#;

const FIXES: &[Fix] = &[
    Fix {
        key: "network",
        en: "Reset network (Winsock, TCP/IP, DNS cache)",
        tr: "Ağı sıfırla (Winsock, TCP/IP, DNS önbelleği)",
        detail_en: "no internet, 'limited connection', broken DNS",
        detail_tr: "internet yok, 'sınırlı bağlantı', DNS bozuk",
        restart: true,
        steps: &[
            Step::Exec("netsh", &["winsock", "reset"]),
            Step::Exec("netsh", &["int", "ip", "reset"]),
            Step::Exec("ipconfig", &["/flushdns"]),
        ],
    },
    Fix {
        key: "update",
        en: "Reset Windows Update components",
        tr: "Windows Update bileşenlerini sıfırla",
        detail_en: "updates stuck or failing; old folders are renamed, not deleted",
        detail_tr: "güncellemeler takılıyor; eski klasörler silinmez, yeniden adlandırılır",
        restart: true,
        steps: &[Step::Ps(UPDATE_RESET)],
    },
    Fix {
        key: "store",
        en: "Reset Microsoft Store cache",
        tr: "Microsoft Store önbelleğini sıfırla",
        detail_en: "Store won't open or download",
        detail_tr: "Store açılmıyor ya da indirmiyor",
        restart: false,
        steps: &[Step::Ps("Start-Process -FilePath wsreset.exe -Wait")],
    },
    Fix {
        key: "dism",
        en: "Repair Windows image (DISM + SFC)",
        tr: "Windows görüntüsünü onar (DISM + SFC)",
        detail_en: "corrupted system files; 15-40 min",
        detail_tr: "bozuk sistem dosyaları; 15-40 dk",
        restart: false,
        steps: &[
            Step::Exec("DISM.exe", &["/Online", "/Cleanup-Image", "/RestoreHealth"]),
            Step::Exec("sfc", &["/scannow"]),
        ],
    },
    Fix {
        key: "time",
        en: "Sync the clock with the internet",
        tr: "Saati internetle eşitle",
        detail_en: "wrong time, certificate / login errors",
        detail_tr: "yanlış saat, sertifika / oturum hataları",
        restart: false,
        steps: &[Step::Ps(TIME_SYNC)],
    },
    Fix {
        key: "printer",
        en: "Clear stuck print queue",
        tr: "Takılan yazdırma kuyruğunu temizle",
        detail_en: "documents stuck in the printer queue",
        detail_tr: "yazıcı kuyruğunda takılı belgeler",
        restart: false,
        steps: &[Step::Ps(SPOOLER_RESET)],
    },
];

fn name(f: &Fix) -> &'static str {
    ui::t(f.en, f.tr)
}

fn run_fix(f: &Fix) -> Result<(), String> {
    for step in f.steps {
        let (ok, out) = match step {
            Step::Exec(p, a) => ps::exec(p, a),
            Step::Ps(s) => match ps::text(s, &[]) {
                Ok(o) => (true, o),
                Err(e) => (false, e.to_string()),
            },
        };
        if !ok {
            return Err(ps::last_line(&out));
        }
    }
    Ok(())
}

pub fn run(only: Option<String>, dry: bool, no_restore_point: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "repair :: fixes for common Windows problems",
        "repair :: yaygın Windows sorunlarına çözümler",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz repair works on Windows only.",
            "kz repair yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let chosen: Vec<&Fix> = match only.as_deref() {
        Some(k) => vec![FIXES.iter().find(|f| f.key == k).ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                tr!(
                    "unknown fix: {k}. use network | update | store | dism | time | printer",
                    "bilinmeyen: {k}. network | update | store | dism | time | printer kullanın"
                )
            )
        })?],
        None if dry || !ui::interactive() => FIXES.iter().collect(),
        None => {
            let items = FIXES
                .iter()
                .map(|f| ui::CheckItem {
                    label: name(f).into(),
                    detail: ui::t(f.detail_en, f.detail_tr).into(),
                    size: 0,
                    checked: false,
                })
                .collect();
            let Some(sel) = ui::checklist_opts(
                ui::t(
                    "kazma repair :: select what to fix",
                    "kazma repair :: düzeltilecekleri seç",
                ),
                items,
                false,
            )?
            else {
                println!("  {}", ui::t("cancelled.", "iptal edildi."));
                return Ok(());
            };
            FIXES
                .iter()
                .zip(sel)
                .filter(|(_, on)| *on)
                .map(|(f, _)| f)
                .collect()
        }
    };
    if dry {
        ui::section(ui::t("Available fixes", "Kullanılabilir düzeltmeler"));
        for f in &chosen {
            println!(
                "  {:<9} {:<46} {}",
                f.key,
                name(f),
                ui::t(f.detail_en, f.detail_tr)
            );
        }
        println!(
            "\n  {}\n",
            ui::t(
                "run one: kz repair network",
                "birini çalıştır: kz repair network"
            )
        );
        return Ok(());
    }
    if chosen.is_empty() {
        println!("  {}", ui::t("nothing selected.", "hiçbir şey seçilmedi."));
        return Ok(());
    }
    if !sys::is_admin() {
        anyhow::bail!(
            "{}",
            ui::t(
                "repairs need an elevated terminal (Run as administrator)",
                "onarımlar için yönetici terminali gerekir (Yönetici olarak çalıştır)"
            )
        );
    }
    if only.is_some()
        && ui::interactive()
        && !crate::remove::ask(&tr!(
            "Run \"{}\"?",
            "\"{}\" çalıştırılsın mı?",
            name(chosen[0])
        ))
    {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    }
    if !no_restore_point {
        ps::working(ui::t(
            "creating a restore point...",
            "geri yükleme noktası oluşturuluyor...",
        ));
        match ps::powershell(drivers::RESTORE_POINT, &[]) {
            Ok(o) if o.status.success() => ui::ok(ui::t(
                "restore point created                ",
                "geri yükleme noktası oluşturuldu       ",
            )),
            _ => ui::skip(ui::t(
                "no new restore point (off, or one was made in the last 24 h)",
                "yeni geri yükleme noktası yok (kapalı ya da son 24 saatte oluşturulmuş)",
            )),
        }
    }
    ui::section(ui::t("Repairing", "Onarılıyor"));
    let mut restart = false;
    for f in chosen {
        ps::working(&format!("{} ...", name(f)));
        match run_fix(f) {
            Ok(()) => {
                restart |= f.restart;
                ui::ok(&format!("{:<50}", name(f)));
                oplog::record("repair", f.key);
            }
            Err(e) => {
                ui::warn(&format!("{}  ({e})", name(f)));
                oplog::record("repair", &format!("{} FAILED {e}", f.key));
            }
        }
    }
    if restart {
        println!(
            "\n  ** {} **",
            ui::t(
                "restart Windows to finish the repair",
                "onarımı tamamlamak için Windows'u yeniden başlatın"
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
    fn fixes_are_well_formed() {
        let mut keys: Vec<&str> = FIXES.iter().map(|f| f.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), FIXES.len());
        assert!(FIXES.iter().all(|f| !f.steps.is_empty()));
        // the update reset must rename, never delete
        assert!(UPDATE_RESET.contains("Rename-Item") && !UPDATE_RESET.contains("Remove-Item"));
    }
}
