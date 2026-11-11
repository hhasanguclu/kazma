//! `kz backup`: move-to-a-new-PC kit. Saves the winget app list, third-party
//! drivers, a full list of installed programs and kazma's own settings into one
//! folder. `kz backup restore <folder>` reinstalls the apps and drivers.

use crate::fsutil::home;
use crate::{oplog, ps, sys, tr, ui, uninstall};
use std::path::{Path, PathBuf};
use std::process::Command;

const KAZMA_FILES: [&str; 3] = ["config.txt", "whitelist.txt", "clean-rules.txt"];

fn default_dir() -> PathBuf {
    let date = &oplog::now_utc()[..10];
    home()
        .map(|h| h.join("Documents"))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!("kazma-backup-{date}"))
}

/// Run a long command with its output shown live; returns success.
fn run_live(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn readme() -> &'static str {
    ui::t(
        "kazma backup\r\n\r\napps.json      programs that winget can reinstall\r\nprograms.txt   every installed program (install the ones not in winget by hand)\r\ndrivers\\       third-party drivers\r\nkazma\\         kazma settings\r\n\r\nTo restore on the new PC (an elevated terminal is recommended):\r\n    kz backup restore \"this folder\"\r\n",
        "kazma yedeği\r\n\r\napps.json      winget ile kurulabilen programlar\r\nprograms.txt   tüm yüklü programlar (winget'te olmayanları elle kurun)\r\ndrivers\\       üçüncü taraf sürücüler\r\nkazma\\         kazma ayarları\r\n\r\nGeri yüklemek için yeni bilgisayarda (yönetici terminali önerilir):\r\n    kz backup restore \"bu klasör\"\r\n",
    )
}

pub fn create(dir: Option<PathBuf>, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "backup :: move-to-a-new-PC kit",
        "backup :: yeni bilgisayara taşıma paketi",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz backup works on Windows only.",
            "kz backup yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let dir = dir.unwrap_or_else(default_dir);
    let admin = sys::is_admin();
    println!("{}\n", tr!("  folder: {}", "  klasör: {}", dir.display()));
    if dry {
        ui::ok(ui::t(
            "apps.json      winget app list",
            "apps.json      winget program listesi",
        ));
        ui::ok(ui::t(
            "programs.txt   all installed programs",
            "programs.txt   tüm yüklü programlar",
        ));
        if admin {
            ui::ok(ui::t(
                "drivers\\       third-party drivers",
                "drivers\\       üçüncü taraf sürücüler",
            ));
        } else {
            ui::skip(ui::t(
                "drivers\\       (needs administrator)",
                "drivers\\       (yönetici gerekli)",
            ));
        }
        ui::ok(ui::t(
            "kazma\\         kazma settings",
            "kazma\\         kazma ayarları",
        ));
        println!();
        return Ok(());
    }
    std::fs::create_dir_all(&dir)?;

    ui::section(ui::t("Programs", "Programlar"));
    let apps = dir.join("apps.json");
    let (ok, out) = ps::exec(
        "winget",
        &[
            "export",
            "-o",
            &apps.to_string_lossy(),
            "--accept-source-agreements",
            "--disable-interactivity",
        ],
    );
    if apps.exists() {
        ui::ok(&tr!("apps.json  (winget)", "apps.json  (winget)"));
    } else {
        ui::warn(&tr!(
            "winget export failed: {}",
            "winget dışa aktarma başarısız: {}",
            if ok { "?".into() } else { ps::last_line(&out) }
        ));
    }
    match ps::powershell_json::<uninstall::Program>(uninstall::LIST, &[]) {
        Ok(mut list) => {
            list.sort_by_key(|p| p.name.to_lowercase());
            list.dedup_by(|a, b| a.name == b.name && a.version == b.version);
            let body: String = list
                .iter()
                .map(|p| format!("{}\t{}\t{}\r\n", p.name, p.version, p.publisher))
                .collect();
            std::fs::write(
                dir.join("programs.txt"),
                format!("name\tversion\tpublisher\r\n{body}"),
            )?;
            ui::ok(&tr!(
                "programs.txt  ({} programs)",
                "programs.txt  ({} program)",
                list.len()
            ));
        }
        Err(e) => ui::warn(&format!("programs.txt: {e}")),
    }

    ui::section(ui::t("Drivers", "Sürücüler"));
    if admin {
        let drivers = dir.join("drivers");
        std::fs::create_dir_all(&drivers)?;
        ps::working(ui::t(
            "exporting drivers (1-3 min)...",
            "sürücüler dışa aktarılıyor (1-3 dk)...",
        ));
        match ps::text(
            "Export-WindowsDriver -Online -Destination $env:KAZMA_DIR | Out-Null",
            &[("KAZMA_DIR", &drivers.to_string_lossy())],
        ) {
            Ok(_) => {
                let n = std::fs::read_dir(&drivers).map(|r| r.count()).unwrap_or(0);
                ui::ok(&tr!(
                    "drivers\\  ({n} packages)          ",
                    "drivers\\  ({n} paket)          "
                ));
            }
            Err(e) => ui::warn(&format!("drivers: {e}")),
        }
    } else {
        ui::skip(ui::t(
            "drivers skipped: run from an elevated terminal to include them",
            "sürücüler atlandı: eklemek için yönetici terminalinde çalıştırın",
        ));
    }

    ui::section("kazma");
    if let Some(data) = oplog::data_dir() {
        let target = dir.join("kazma");
        std::fs::create_dir_all(&target)?;
        let mut n = 0;
        for f in KAZMA_FILES {
            if std::fs::copy(data.join(f), target.join(f)).is_ok() {
                n += 1;
            }
        }
        ui::ok(&tr!("{n} settings files", "{n} ayar dosyası"));
    }
    std::fs::write(dir.join("README.txt"), readme())?;
    oplog::record("backup", &dir.display().to_string());
    println!(
        "\n  {}\n  kz backup restore \"{}\"\n",
        ui::t(
            "done. on the new PC run:",
            "bitti. yeni bilgisayarda şunu çalıştırın:"
        ),
        dir.display()
    );
    sys::reveal(&dir.join("README.txt"));
    Ok(())
}

pub fn restore(dir: &Path, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "backup restore :: reinstall apps and drivers",
        "backup restore :: programları ve sürücüleri geri kur",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz backup works on Windows only.",
            "kz backup yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let apps = dir.join("apps.json");
    let drivers = dir.join("drivers");
    let kazma = dir.join("kazma");
    if !apps.exists() && !drivers.is_dir() {
        anyhow::bail!(
            "{}",
            tr!(
                "not a kazma backup folder: {}",
                "kazma yedek klasörü değil: {}",
                dir.display()
            )
        );
    }
    let admin = sys::is_admin();

    ui::section(ui::t("Plan", "Plan"));
    if apps.exists() {
        ui::ok(ui::t(
            "reinstall programs with winget import",
            "programları winget import ile kur",
        ));
    }
    if drivers.is_dir() {
        if admin {
            ui::ok(ui::t(
                "install drivers with pnputil",
                "sürücüleri pnputil ile kur",
            ));
        } else {
            ui::skip(ui::t(
                "drivers: needs administrator",
                "sürücüler: yönetici gerekli",
            ));
        }
    }
    if kazma.is_dir() {
        ui::ok(ui::t(
            "restore kazma settings (existing ones are kept)",
            "kazma ayarlarını geri yükle (var olanlar korunur)",
        ));
    }
    if dry {
        println!();
        return Ok(());
    }
    if !ui::interactive() || !crate::remove::ask(ui::t("Start?", "Başlansın mı?")) {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    }

    if apps.exists() {
        ui::section(ui::t(
            "Programs (this can take a while)",
            "Programlar (uzun sürebilir)",
        ));
        let ok = run_live(
            "winget",
            &[
                "import",
                "-i",
                &apps.to_string_lossy(),
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--ignore-unavailable",
                "--ignore-versions",
            ],
        );
        if ok {
            ui::ok(ui::t("programs installed", "programlar kuruldu"))
        } else {
            ui::warn(ui::t(
                "some programs could not be installed, see above",
                "bazı programlar kurulamadı, yukarıya bakın",
            ))
        }
    }
    if drivers.is_dir() && admin {
        ui::section(ui::t("Drivers", "Sürücüler"));
        let pattern = drivers.join("*.inf");
        let ok = run_live(
            "pnputil",
            &[
                "/add-driver",
                &pattern.to_string_lossy(),
                "/subdirs",
                "/install",
            ],
        );
        if ok {
            ui::ok(ui::t("drivers added", "sürücüler eklendi"))
        } else {
            ui::warn(ui::t(
                "pnputil reported problems, see above",
                "pnputil sorun bildirdi, yukarıya bakın",
            ))
        }
    }
    if kazma.is_dir()
        && let Some(data) = oplog::data_dir()
    {
        std::fs::create_dir_all(&data)?;
        for f in KAZMA_FILES {
            let (src, dst) = (kazma.join(f), data.join(f));
            if src.exists() && !dst.exists() {
                let _ = std::fs::copy(src, dst);
            }
        }
        ui::ok(ui::t(
            "kazma settings restored",
            "kazma ayarları geri yüklendi",
        ));
    }
    oplog::record("backup", &format!("restored from {}", dir.display()));
    println!(
        "\n  {}\n",
        ui::t(
            "see programs.txt for programs winget could not install.",
            "winget'in kuramadığı programlar için programs.txt'ye bakın."
        )
    );
    Ok(())
}
