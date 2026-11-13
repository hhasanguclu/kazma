//! `kz restore`: list / create System Restore points, open the restore wizard.
//! Rolling back is left to Windows' own wizard (rstrui), which shows exactly
//! what will change before it does anything.

use crate::{oplog, ps, sys, tr, ui};
use serde::Deserialize;

const LIST: &str = r#"
@(Get-ComputerRestorePoint | ForEach-Object {
  [pscustomobject]@{
    Seq  = [int]$_.SequenceNumber
    Desc = [string]$_.Description
    When = [Management.ManagementDateTimeConverter]::ToDateTime($_.CreationTime).ToString('yyyy-MM-dd HH:mm')
    Type = [int]$_.RestorePointType
  }
}) | ConvertTo-Json -Compress
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Point {
    seq: i64,
    desc: String,
    when: String,
    #[serde(rename = "Type")]
    kind: i64,
}

fn kind_name(k: i64) -> &'static str {
    match k {
        0 => ui::t("app install", "uygulama kurulumu"),
        1 => ui::t("app removal", "uygulama kaldırma"),
        10 => ui::t("driver install", "sürücü kurulumu"),
        12 => ui::t("settings", "ayarlar"),
        13 => ui::t("cancelled", "iptal"),
        _ => ui::t("other", "diğer"),
    }
}

fn list() -> anyhow::Result<Vec<Point>> {
    ps::powershell_json(LIST, &[]).map_err(|e| {
        if e.to_string().contains("not supported") {
            anyhow::anyhow!(
                "{}",
                ui::t(
                    "System Restore is not available on this edition of Windows (e.g. Windows Server)",
                    "Sistem Geri Yükleme bu Windows sürümünde yok (örneğin Windows Server)"
                )
            )
        } else {
            e
        }
    })
}

fn need_admin() -> anyhow::Result<()> {
    if !sys::is_admin() {
        anyhow::bail!(
            "{}",
            ui::t(
                "System Restore needs an elevated terminal",
                "Sistem Geri Yükleme için yönetici terminali gerekir"
            )
        );
    }
    Ok(())
}

pub fn run(action: Option<String>, desc: Option<String>) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "restore :: System Restore points",
        "restore :: sistem geri yükleme noktaları",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz restore works on Windows only.",
            "kz restore yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    match action.as_deref().unwrap_or("list") {
        "list" => {
            need_admin()?;
            let mut points = list()?;
            points.sort_by_key(|p| -p.seq);
            if points.is_empty() {
                ui::warn(ui::t(
                    "no restore points (System Restore may be off: kz restore enable)",
                    "geri yükleme noktası yok (kapalı olabilir: kz restore enable)",
                ));
            } else {
                ui::section(&tr!(
                    "{} restore points",
                    "{} geri yükleme noktası",
                    points.len()
                ));
                for p in &points {
                    println!(
                        "  {:>4}  {}  {} {}",
                        p.seq,
                        p.when,
                        ui::pad(kind_name(p.kind), 18),
                        p.desc
                    );
                }
            }
            println!(
                "\n  {}",
                ui::t(
                    "create: kz restore create \"before update\"",
                    "oluştur: kz restore create \"güncelleme öncesi\""
                )
            );
            println!(
                "  {}\n",
                ui::t(
                    "roll back: kz restore open  (Windows' own wizard)",
                    "geri dön: kz restore open  (Windows'un kendi sihirbazı)"
                )
            );
            Ok(())
        }
        "create" => {
            need_admin()?;
            let desc = desc.unwrap_or_else(|| format!("kazma {}", &oplog::now_utc()[..16]));
            let before = list().map(|v| v.len()).unwrap_or(0);
            ps::working(ui::t(
                "creating restore point (up to a minute)...",
                "geri yükleme noktası oluşturuluyor (bir dakikaya kadar)...",
            ));
            ps::text(
                "Checkpoint-Computer -Description $env:KAZMA_DESC -RestorePointType MODIFY_SETTINGS -WarningAction SilentlyContinue",
                &[("KAZMA_DESC", &desc)],
            )?;
            println!("{:70}", "");
            if list().map(|v| v.len()).unwrap_or(0) > before {
                ui::ok(&tr!(
                    "restore point created: {desc}",
                    "geri yükleme noktası oluşturuldu: {desc}"
                ));
                oplog::record("restore", &format!("created {desc}"));
            } else {
                ui::warn(ui::t(
                    "Windows skipped it: only one restore point per 24 hours is allowed by default.",
                    "Windows atladı: varsayılan olarak 24 saatte yalnızca bir geri yükleme noktası oluşturulur.",
                ));
            }
            Ok(())
        }
        "enable" => {
            need_admin()?;
            let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
            ps::text(
                "Enable-ComputerRestore -Drive \"$($env:KAZMA_DRIVE)\\\"",
                &[("KAZMA_DRIVE", &drive)],
            )?;
            ui::ok(&tr!(
                "System Restore enabled on {drive}",
                "{drive} için Sistem Geri Yükleme açıldı"
            ));
            oplog::record("restore", &format!("enabled on {drive}"));
            Ok(())
        }
        "open" => {
            // GUI wizard: start it and return, don't wait for it to close
            std::process::Command::new("rstrui.exe").spawn()?;
            ui::ok(ui::t(
                "System Restore wizard opened",
                "Sistem Geri Yükleme sihirbazı açıldı",
            ));
            Ok(())
        }
        other => anyhow::bail!(
            "{}",
            tr!(
                "unknown: {other}. use list | create | enable | open",
                "bilinmeyen: {other}. list | create | enable | open kullanın"
            )
        ),
    }
}
