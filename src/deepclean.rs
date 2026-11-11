//! `kz deepclean`: the big, rarely-cleaned space hogs (needs administrator).
//!
//! - Windows.old and upgrade leftovers are removed by Windows' own Disk Cleanup
//!   (cleanmgr /sagerun), which handles the TrustedInstaller permissions the
//!   official way; kazma only switches on the handlers you picked.
//! - Memory dumps are deleted directly.
//! - The hibernation file is shrunk with `powercfg /h /type reduced`
//!   (Fast Startup keeps working). `kz power hibernate off` removes it fully.
//! - Old restore points are deleted, always keeping the newest per drive.

use crate::fsutil::{self, Stats};
use crate::{oplog, ps, sys, tr, ui};
use serde::Deserialize;
use std::path::PathBuf;

/// Our private cleanmgr profile number; flags are removed again after the run.
const SAGE: &str = "4242";

const INFO: &str = r#"
$h = Join-Path $env:SystemDrive 'hiberfil.sys'
$size = 0
try { $size = (Get-Item -LiteralPath $h -Force -ErrorAction Stop).Length } catch {}
$type = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Power' -Name HiberFileType -ErrorAction SilentlyContinue).HiberFileType
$shadows = @(Get-CimInstance Win32_ShadowCopy -ErrorAction SilentlyContinue)
$extra = 0
$shadows | Group-Object VolumeName | ForEach-Object { $extra += [math]::Max(0, $_.Count - 1) }
$used = 0
Get-CimInstance Win32_ShadowStorage -ErrorAction SilentlyContinue | ForEach-Object { $used += [int64]$_.UsedSpace }
[pscustomobject]@{ HiberSize = [int64]$size; HiberType = [int]$type; ShadowExtra = [int]$extra; ShadowUsed = [int64]$used } | ConvertTo-Json -Compress
"#;

const CLEANMGR: &str = r#"
$base = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\VolumeCaches'
$flag = "StateFlags$($env:KAZMA_SAGE)"
$want = $env:KAZMA_HANDLERS -split ';'
Get-ChildItem $base | ForEach-Object { Remove-ItemProperty -Path $_.PSPath -Name $flag -ErrorAction SilentlyContinue }
foreach ($h in $want) {
  $k = Join-Path $base $h
  if (Test-Path $k) { New-ItemProperty -Path $k -Name $flag -Value 2 -PropertyType DWord -Force | Out-Null }
}
Start-Process -FilePath cleanmgr.exe -ArgumentList "/sagerun:$($env:KAZMA_SAGE)" -Wait
Get-ChildItem $base | ForEach-Object { Remove-ItemProperty -Path $_.PSPath -Name $flag -ErrorAction SilentlyContinue }
"#;

const OLD_SHADOWS: &str = r#"
$n = 0
Get-CimInstance Win32_ShadowCopy | Group-Object VolumeName | ForEach-Object {
  $_.Group | Sort-Object InstallDate | Select-Object -SkipLast 1 | ForEach-Object { Remove-CimInstance -InputObject $_; $n++ }
}
$n
"#;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct Info {
    hiber_size: i64,
    hiber_type: i32,
    shadow_extra: i32,
    shadow_used: i64,
}

#[derive(Clone)]
enum Action {
    /// Disk Cleanup handler names (VolumeCaches subkeys) and the folders they empty.
    Cleanmgr(Vec<&'static str>, Vec<PathBuf>),
    /// Delete these files, and the contents of these folders.
    Delete(Vec<PathBuf>, Vec<PathBuf>),
    HiberReduce,
    OldRestorePoints,
}

struct Item {
    name: &'static str,
    detail: String,
    size: u64,
    default_on: bool,
    action: Action,
}

fn sys_root() -> PathBuf {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    PathBuf::from(format!("{drive}\\"))
}

fn win_dir() -> PathBuf {
    fsutil::env_path("SystemRoot").unwrap_or_else(|| sys_root().join("Windows"))
}

fn size_of(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|p| fsutil::path_size(p)).sum()
}

fn items(info: &Info) -> Vec<Item> {
    let root = sys_root();
    let win = win_dir();
    let mut v = vec![];

    let old = root.join("Windows.old");
    if old.is_dir() {
        v.push(Item {
            name: ui::t(
                "Previous Windows installation (Windows.old)",
                "Önceki Windows kurulumu (Windows.old)",
            ),
            detail: ui::t(
                "you can no longer roll back to it",
                "artık ona geri dönülemez",
            )
            .into(),
            size: fsutil::dir_size(&old),
            default_on: true,
            action: Action::Cleanmgr(vec!["Previous Installations"], vec![old]),
        });
    }
    let setup: Vec<PathBuf> = ["$WINDOWS.~BT", "$WINDOWS.~WS", "$GetCurrent"]
        .iter()
        .map(|d| root.join(d))
        .filter(|p| p.is_dir())
        .collect();
    if !setup.is_empty() {
        v.push(Item {
            name: ui::t(
                "Windows upgrade setup files",
                "Windows yükseltme kurulum dosyaları",
            ),
            detail: setup
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            size: size_of(&setup),
            default_on: true,
            action: Action::Cleanmgr(
                vec![
                    "Temporary Setup Files",
                    "Windows Upgrade Log Files",
                    "Setup Log Files",
                ],
                setup,
            ),
        });
    }
    let esd = root.join("ESD");
    if esd.is_dir() {
        v.push(Item {
            name: ui::t(
                "Windows ESD installation files",
                "Windows ESD kurulum dosyaları",
            ),
            detail: ui::t(
                "needed only for 'Reset this PC' without download",
                "yalnızca indirmesiz 'Bu bilgisayarı sıfırla' için gerekir",
            )
            .into(),
            size: fsutil::dir_size(&esd),
            default_on: false,
            action: Action::Cleanmgr(vec!["Windows ESD installation files"], vec![esd]),
        });
    }
    let dump_files: Vec<PathBuf> = [win.join("MEMORY.DMP")]
        .into_iter()
        .filter(|p| p.is_file())
        .collect();
    let dump_dirs: Vec<PathBuf> = [win.join("Minidump"), win.join("LiveKernelReports")]
        .into_iter()
        .filter(|p| p.is_dir())
        .collect();
    let dump_size =
        size_of(&dump_files) + dump_dirs.iter().map(|d| fsutil::dir_size(d)).sum::<u64>();
    if dump_size > 0 {
        v.push(Item {
            name: ui::t("System memory dumps", "Sistem bellek dökümleri"),
            detail: ui::t(
                "only needed to debug old crashes",
                "yalnızca eski çökmeleri incelemek için gerekir",
            )
            .into(),
            size: dump_size,
            default_on: true,
            action: Action::Delete(dump_files, dump_dirs),
        });
    }
    // HiberFileType: 2 = full, 1 = reduced. Reducing roughly halves the file.
    if info.hiber_size > 0 && info.hiber_type != 1 {
        v.push(Item {
            name: ui::t(
                "Shrink the hibernation file",
                "Hazırda bekletme dosyasını küçült",
            ),
            detail: ui::t(
                "Fast Startup keeps working; full hibernate is turned off",
                "Hızlı Başlangıç çalışmaya devam eder; tam hazırda bekletme kapanır",
            )
            .into(),
            size: info.hiber_size as u64 / 2,
            default_on: true,
            action: Action::HiberReduce,
        });
    }
    if info.shadow_extra > 0 {
        v.push(Item {
            name: ui::t(
                "Older restore points (newest is kept)",
                "Eski geri yükleme noktaları (en yenisi kalır)",
            ),
            detail: tr!("{} older points", "{} eski nokta", info.shadow_extra),
            size: info.shadow_used.max(0) as u64,
            default_on: false,
            action: Action::OldRestorePoints,
        });
    }
    v
}

fn exists_any(paths: &[PathBuf]) -> bool {
    paths.iter().any(|p| p.exists())
}

fn apply(a: &Action) -> Result<String, String> {
    match a {
        Action::Cleanmgr(handlers, folders) => {
            let before = size_of(folders);
            ps::text(
                CLEANMGR,
                &[
                    ("KAZMA_SAGE", SAGE),
                    ("KAZMA_HANDLERS", &handlers.join(";")),
                ],
            )
            .map_err(|e| e.to_string())?;
            let after = if exists_any(folders) {
                size_of(folders)
            } else {
                0
            };
            Ok(ui::human(before.saturating_sub(after)))
        }
        Action::Delete(files, dirs) => {
            let s = files
                .iter()
                .map(|f| fsutil::delete_file(f, false))
                .chain(dirs.iter().map(|d| fsutil::delete_contents(d, None, false)))
                .fold(Stats::default(), |a, b| Stats {
                    bytes: a.bytes + b.bytes,
                    files: a.files + b.files,
                    failed: a.failed + b.failed,
                });
            Ok(ui::human(s.bytes))
        }
        Action::HiberReduce => {
            let (ok, out) = ps::exec("powercfg", &["/h", "/type", "reduced"]);
            if ok {
                Ok(ui::t("reduced", "küçültüldü").into())
            } else {
                Err(ps::last_line(&out))
            }
        }
        Action::OldRestorePoints => {
            let n = ps::text(OLD_SHADOWS, &[]).map_err(|e| e.to_string())?;
            Ok(tr!("{} removed", "{} kaldırıldı", n.trim()))
        }
    }
}

pub fn run(dry: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "deepclean :: the big space hogs",
        "deepclean :: büyük yer kaplayanlar",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz deepclean works on Windows only.",
            "kz deepclean yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    ps::working(ui::t(
        "measuring (Windows.old can take a minute)...",
        "ölçülüyor (Windows.old bir dakika sürebilir)...",
    ));
    let info: Info = ps::powershell_json::<Info>(INFO, &[])
        .ok()
        .and_then(|v| v.into_iter().next())
        .unwrap_or_default();
    let list = items(&info);
    println!("{:70}", "");
    if list.is_empty() {
        ui::ok(ui::t(
            "Nothing big to clean. Nice.",
            "Temizlenecek büyük bir şey yok. Güzel.",
        ));
        return Ok(());
    }
    let admin = sys::is_admin();
    if dry || !admin || !ui::interactive() {
        ui::section(ui::t("Found", "Bulunanlar"));
        let mut total = 0;
        for i in &list {
            total += i.size;
            let line = format!("{:<46} {:>10}  {}", i.name, ui::human(i.size), i.detail);
            if i.default_on {
                ui::ok(&line)
            } else {
                ui::skip(&line)
            }
        }
        println!(
            "\n  {}",
            tr!(
                "up to {} can be freed",
                "en fazla {} kazanılabilir",
                ui::human(total)
            )
        );
        if !admin {
            ui::warn(ui::t(
                "cleaning these needs an elevated terminal: kz deepclean",
                "temizlemek için yönetici terminali gerekir: kz deepclean",
            ));
        }
        println!();
        return Ok(());
    }
    let items_ui = list
        .iter()
        .map(|i| ui::CheckItem {
            label: i.name.to_string(),
            detail: i.detail.clone(),
            size: i.size,
            checked: i.default_on,
        })
        .collect();
    let Some(sel) = ui::checklist(
        ui::t(
            "kazma deepclean :: select what to remove",
            "kazma deepclean :: kaldırılacakları seç",
        ),
        items_ui,
    )?
    else {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    };
    ui::section(ui::t(
        "Cleaning (Disk Cleanup may open a progress window)",
        "Temizleniyor (Disk Temizleme ilerleme penceresi açabilir)",
    ));
    for (i, on) in list.iter().zip(sel) {
        if !on {
            continue;
        }
        ps::working(&tr!("{} ...", "{} ...", i.name));
        match apply(&i.action) {
            Ok(r) => {
                ui::ok(&format!("{} {r}          ", ui::pad(i.name, 46)));
                oplog::record("deepclean", &format!("{} ({r})", i.name));
            }
            Err(e) => ui::warn(&format!("{} {e}", ui::pad(i.name, 46))),
        }
    }
    println!();
    Ok(())
}
