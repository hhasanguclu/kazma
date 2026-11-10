//! `kz installer`: find leftover installer files (setup .exe, .msi, .msix, .iso...)
//! in Downloads and on the Desktop. Removed files go to the Recycle Bin.

use crate::fsutil::{self, home};
use crate::{oplog, tr, ui};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RECENT_DAYS: u64 = 7;

struct Found {
    path: PathBuf,
    size: u64,
    recent: bool,
    age_days: u64,
}

fn ext(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Packages that are always installers, wherever they are.
fn is_package(p: &Path) -> bool {
    matches!(
        ext(p).as_str(),
        "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" | "iso"
    )
}

/// An .exe whose name says it is a setup program.
fn looks_like_setup(p: &Path) -> bool {
    let n = p
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    ext(p) == "exe"
        && [
            "setup",
            "install",
            "installer",
            "update",
            "x64",
            "x86",
            "win64",
            "win32",
            "amd64",
            "_inst",
        ]
        .iter()
        .any(|k| n.contains(k))
}

/// In Downloads every .exe is treated as an installer; elsewhere only setup-looking ones.
fn wanted(p: &Path, downloads: bool) -> bool {
    is_package(p) || (ext(p) == "exe" && (downloads || looks_like_setup(p)))
}

fn collect(dir: &Path, depth: usize, downloads: bool, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        let p = e.path();
        if ft.is_dir() && !ft.is_symlink() && depth > 0 {
            collect(&p, depth - 1, downloads, out);
        } else if ft.is_file() && wanted(&p, downloads) {
            out.push(p);
        }
    }
}

pub fn run(paths: Vec<PathBuf>, dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t(
            "installer :: dry run, nothing will be deleted",
            "installer :: deneme, hiçbir şey silinmeyecek",
        )
    } else {
        ui::t(
            "installer :: remove leftover installer files",
            "installer :: eski kurulum dosyalarını temizle",
        )
    });
    let mut files = vec![];
    if paths.is_empty() {
        let Some(h) = home() else {
            ui::warn(ui::t(
                "cannot find your user folder",
                "kullanıcı klasörü bulunamadı",
            ));
            return Ok(());
        };
        for (d, depth, dl) in [
            (h.join("Downloads"), 2, true),
            (h.join("Desktop"), 0, false),
            (h.join("OneDrive").join("Desktop"), 0, false),
        ] {
            if d.is_dir() {
                println!("{}", tr!("  scanning {}", "  taranıyor {}", d.display()));
                collect(&d, depth, dl, &mut files);
            }
        }
    } else {
        for d in &paths {
            println!("{}", tr!("  scanning {}", "  taranıyor {}", d.display()));
            collect(d, 2, true, &mut files);
        }
    }

    let now = SystemTime::now();
    let mut found: Vec<Found> = files
        .into_iter()
        .filter(|p| fsutil::can_remove(p))
        .filter_map(|path| {
            let m = fs::metadata(&path).ok()?;
            let age = m
                .modified()
                .ok()
                .and_then(|t| now.duration_since(t).ok())
                .unwrap_or(Duration::ZERO);
            Some(Found {
                size: m.len(),
                recent: age < Duration::from_secs(RECENT_DAYS * 86_400),
                age_days: age.as_secs() / 86_400,
                path,
            })
        })
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.size));

    if found.is_empty() {
        ui::ok(ui::t(
            "No installer files found.",
            "Kurulum dosyası bulunamadı.",
        ));
        return Ok(());
    }
    if !dry && !yes && !ui::interactive() {
        ui::warn(ui::t(
            "not an interactive terminal: re-run with --yes to delete, or --dry-run to preview",
            "etkileşimli terminal değil: silmek için --yes, önizlemek için --dry-run ile çalıştırın",
        ));
        return Ok(());
    }

    let selection: Vec<bool> = if dry || yes {
        found.iter().map(|f| yes || !f.recent).collect()
    } else {
        let items = found
            .iter()
            .map(|f| ui::CheckItem {
                label: f
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                detail: tr!(
                    "{}d old  {}",
                    "{} gün önce  {}",
                    f.age_days,
                    f.path
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                ),
                size: f.size,
                checked: !f.recent,
            })
            .collect();
        match ui::checklist(
            ui::t(
                "kazma installer :: select files to send to the Recycle Bin",
                "kazma installer :: Geri Dönüşüm Kutusu'na gidecekleri seç",
            ),
            items,
        )? {
            Some(s) => s,
            None => {
                println!("  {}", ui::t("cancelled.", "iptal edildi."));
                return Ok(());
            }
        }
    };

    ui::section(if dry {
        ui::t(
            "Would move to Recycle Bin",
            "Geri Dönüşüm Kutusu'na taşınacaklar",
        )
    } else {
        ui::t("Moving to Recycle Bin", "Geri Dönüşüm Kutusu'na taşınıyor")
    });
    let mut total = 0u64;
    for (f, on) in found.iter().zip(selection) {
        let line = format!("{:>10}  {}", ui::human(f.size), f.path.display());
        if !on {
            let why = if f.recent {
                tr!(
                    "newer than {RECENT_DAYS}d, kept",
                    "{RECENT_DAYS} günden yeni, tutuldu"
                )
            } else {
                ui::t("not selected", "seçilmedi").into()
            };
            ui::skip(&format!("{line}  ({why})"));
            continue;
        }
        if dry {
            ui::ok(&line);
            total += f.size;
            continue;
        }
        match fsutil::trash(&f.path) {
            Ok(()) => {
                ui::ok(&line);
                total += f.size;
                oplog::record(
                    "installer",
                    &format!("trashed {} size={}", f.path.display(), f.size),
                );
            }
            Err(e) => ui::warn(&format!("{line}  ({e})")),
        }
    }
    println!(
        "\n  {} {}\n",
        if dry {
            ui::t("would free", "kazanılacak")
        } else {
            ui::t(
                "freed (after emptying the Recycle Bin)",
                "kazanılan (Geri Dönüşüm Kutusu boşaltılınca)",
            )
        },
        ui::human(total)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert!(wanted(Path::new("/d/Firefox Setup 132.0.msi"), false));
        assert!(wanted(Path::new("/d/ubuntu-24.04.iso"), false));
        assert!(wanted(Path::new("/d/random-tool.exe"), true));
        assert!(!wanted(Path::new("/d/random-tool.exe"), false));
        assert!(wanted(Path::new("/d/VSCodeUserSetup-x64.exe"), false));
        assert!(!wanted(Path::new("/d/notes.txt"), true));
    }
}
