//! `kz purge`: find and remove regenerable project build artifacts.

use crate::fsutil::{self, Stats, home};
use crate::{oplog, tr, ui};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_DEPTH: usize = 10;
const RECENT_DAYS: u64 = 7;

struct Found {
    path: PathBuf,
    kind: &'static str,
    size: u64,
    recent: bool,
}

fn has_file(dir: &Path, name: &str) -> bool {
    dir.join(name).is_file()
}

fn has_ext(dir: &Path, exts: &[&str]) -> bool {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_lowercase();
                exts.iter().any(|x| n.ends_with(x))
            })
        })
        .unwrap_or(false)
}

/// Decide whether `dir` (named `name`, inside `parent`) is a build artifact.
/// Every rule requires a project marker next to it, so random folders called
/// "build" or "bin" are never touched.
fn artifact_kind(dir: &Path, name: &str, parent: &Path) -> Option<&'static str> {
    let gradle = || {
        has_file(parent, "build.gradle")
            || has_file(parent, "build.gradle.kts")
            || has_file(parent, "settings.gradle")
            || has_file(parent, "settings.gradle.kts")
    };
    match name {
        "node_modules" if has_file(parent, "package.json") => Some("node"),
        ".next" | ".nuxt" | ".turbo" | ".parcel-cache" | ".svelte-kit" | ".angular"
            if has_file(parent, "package.json") =>
        {
            Some("js")
        }
        "target" if has_file(parent, "Cargo.toml") => Some("rust"),
        "target" if has_file(parent, "pom.xml") => Some("maven"),
        "bin" | "obj" if has_ext(parent, &[".csproj", ".fsproj", ".vbproj"]) => Some(".net"),
        ".venv" | "venv" | "env" if has_file(dir, "pyvenv.cfg") => Some("python"),
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox" => Some("python"),
        "build" | ".gradle" if gradle() => Some("gradle"),
        ".dart_tool" | "build" if has_file(parent, "pubspec.yaml") => Some("flutter"),
        "vendor" if has_file(parent, "composer.json") => Some("php"),
        n if n.starts_with("cmake-build-") => Some("cmake"),
        "zig-cache" | ".zig-cache" | "zig-out" if has_file(parent, "build.zig") => Some("zig"),
        _ => None,
    }
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | "AppData"
            | "$Recycle.Bin"
            | "Windows"
            | ".cargo"
            | ".rustup"
            | ".nuget"
    )
}

fn walk(dir: &Path, depth: usize) -> Vec<(PathBuf, &'static str)> {
    if depth > MAX_DEPTH {
        return vec![];
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let subs: Vec<(PathBuf, String)> = rd
        .flatten()
        .filter(|e| {
            e.file_type()
                .map(|t| t.is_dir() && !t.is_symlink())
                .unwrap_or(false)
        })
        .map(|e| (e.path(), e.file_name().to_string_lossy().into_owned()))
        .collect();
    subs.par_iter()
        .flat_map(|(path, name)| {
            if let Some(kind) = artifact_kind(path, name, dir) {
                vec![(path.clone(), kind)]
            } else if skip_dir(name) {
                vec![]
            } else {
                walk(path, depth + 1)
            }
        })
        .collect()
}

fn default_roots() -> Vec<PathBuf> {
    let Some(h) = home() else { return vec![] };
    let mut roots: Vec<PathBuf> = [
        "source",
        "src",
        "dev",
        "Dev",
        "Projects",
        "projects",
        "code",
        "Code",
        "repos",
        "workspace",
        "Documents\\GitHub",
        "Documents/GitHub",
        "Desktop",
    ]
    .iter()
    .map(|p| h.join(p))
    .filter(|p| p.is_dir())
    .filter_map(|p| fs::canonicalize(&p).ok())
    .collect();
    roots.sort();
    roots.dedup();
    // drop roots nested inside another root
    let all = roots.clone();
    roots.retain(|r| !all.iter().any(|o| o != r && r.starts_with(o)));
    roots
}

pub fn run(paths: Vec<PathBuf>, dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t(
            "purge :: dry run, nothing will be deleted",
            "purge :: deneme, hiçbir şey silinmeyecek",
        )
    } else {
        ui::t(
            "purge :: clean project build artifacts",
            "purge :: proje derleme çıktılarını temizle",
        )
    });
    let roots = if paths.is_empty() {
        default_roots()
    } else {
        paths
    };
    if roots.is_empty() {
        ui::warn(ui::t(
            "No project folders found. Pass one explicitly: kz purge D:\\code",
            "Proje klasörü bulunamadı. Klasörü belirtin: kz purge D:\\code",
        ));
        return Ok(());
    }
    for r in &roots {
        println!("{}", tr!("  scanning {}", "  taranıyor {}", r.display()));
    }

    let recent_cut = SystemTime::now() - Duration::from_secs(RECENT_DAYS * 86_400);
    let mut hits: Vec<(PathBuf, &'static str)> =
        roots.par_iter().flat_map(|r| walk(r, 0)).collect();
    hits.sort();
    hits.dedup();
    let mut found: Vec<Found> = hits
        .into_par_iter()
        .map(|(path, kind)| {
            let size = fsutil::dir_size(&path);
            let recent = fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(|m| m > recent_cut)
                .unwrap_or(false);
            Found {
                path,
                kind,
                size,
                recent,
            }
        })
        .filter(|f| f.size > 0)
        .collect();
    found.sort_by_key(|f| std::cmp::Reverse(f.size));

    if found.is_empty() {
        ui::ok(ui::t(
            "No build artifacts found.",
            "Derleme çıktısı bulunamadı.",
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
    let selection: Vec<bool> = if dry || yes || !ui::interactive() {
        found.iter().map(|f| yes || !f.recent).collect()
    } else {
        let items = found
            .iter()
            .map(|f| ui::CheckItem {
                label: format!("{:<7}{}", f.kind, f.path.display()),
                detail: if f.recent {
                    tr!(
                        "touched < {RECENT_DAYS}d ago",
                        "< {RECENT_DAYS} gün önce değişti"
                    )
                } else {
                    String::new()
                },
                size: f.size,
                checked: !f.recent,
            })
            .collect();
        match ui::checklist(
            ui::t(
                "kazma purge :: select artifacts to delete",
                "kazma purge :: silinecek çıktıları seç",
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

    let mut total = Stats::default();
    ui::section(if dry {
        ui::t("Would delete", "Silinecekler")
    } else {
        ui::t("Deleting", "Siliniyor")
    });
    for (f, on) in found.iter().zip(&selection) {
        if !*on {
            let why = if f.recent {
                tr!(
                    "recent (< {RECENT_DAYS}d), kept",
                    "yeni (< {RECENT_DAYS} gün), tutuldu"
                )
            } else {
                ui::t("not selected", "seçilmedi").into()
            };
            ui::skip(&format!(
                "{:>10}  {}  ({why})",
                ui::human(f.size),
                f.path.display()
            ));
            continue;
        }
        let r = fsutil::delete_tree(&f.path, dry);
        let note = if r.failed > 0 {
            tr!("  ({} locked)", "  ({} kilitli)", r.failed)
        } else {
            String::new()
        };
        ui::ok(&format!(
            "{:>10}  {}{note}",
            ui::human(r.bytes),
            f.path.display()
        ));
        total.bytes += r.bytes;
        total.files += r.files;
        if !dry {
            oplog::record("purge", &format!("{} freed={}", f.path.display(), r.bytes));
        }
    }
    println!(
        "\n  {} {}\n",
        if dry {
            ui::t("would free", "kazanılacak")
        } else {
            ui::t("freed", "kazanılan")
        },
        ui::human(total.bytes)
    );
    Ok(())
}
