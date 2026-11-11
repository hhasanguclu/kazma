//! `kz empty`: empty folders, broken shortcuts and zero-byte files.
//! Everything found goes to the Recycle Bin.

use crate::fsutil::{self, env_path, home};
use crate::{oplog, ps, tr, ui};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Zero-byte files that are empty on purpose.
const KEEP_EMPTY: &[&str] = &[
    ".gitkeep",
    ".keep",
    "__init__.py",
    "py.typed",
    ".nomedia",
    "desktop.ini",
    ".gitignore",
    "lock",
    ".lock",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    EmptyDir,
    ZeroFile,
    BrokenLink,
}

#[derive(Default)]
struct Scan {
    empty_dirs: Vec<PathBuf>,
    zero_files: Vec<PathBuf>,
    links: Vec<PathBuf>,
}

impl Scan {
    fn merge(mut self, o: Scan) -> Scan {
        self.empty_dirs.extend(o.empty_dirs);
        self.zero_files.extend(o.zero_files);
        self.links.extend(o.links);
        self
    }
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "AppData" | "$Recycle.Bin" | "System Volume Information"
        )
}

/// Walk `dir`; returns (dir is completely empty, findings). A fully empty tree is
/// reported once, at its top, by the caller.
fn walk(dir: &Path) -> (bool, Scan) {
    let Ok(rd) = fs::read_dir(dir) else {
        return (false, Scan::default());
    };
    let entries: Vec<fs::DirEntry> = rd.flatten().collect();
    let results: Vec<(bool, Scan)> = entries
        .par_iter()
        .map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(ft) = e.file_type() else {
                return (false, Scan::default());
            };
            if ft.is_symlink() {
                return (false, Scan::default());
            }
            if ft.is_dir() {
                if skip_dir(&name) {
                    return (false, Scan::default());
                }
                let (empty, mut s) = walk(&path);
                if empty {
                    s.empty_dirs.push(path);
                }
                return (empty, s);
            }
            let mut s = Scan::default();
            if name.to_lowercase().ends_with(".lnk") {
                s.links.push(path.clone());
            }
            if e.metadata().map(|m| m.len() == 0).unwrap_or(false)
                && !KEEP_EMPTY.contains(&name.to_lowercase().as_str())
            {
                s.zero_files.push(path);
            }
            (false, s)
        })
        .collect();
    let all_empty = results.iter().all(|(e, _)| *e);
    let mut scan = results
        .into_iter()
        .fold(Scan::default(), |a, (_, s)| a.merge(s));
    if all_empty {
        // only the top of an empty tree is kept; the caller adds `dir` itself
        scan.empty_dirs.retain(|p| p.parent() != Some(dir));
    }
    (all_empty, scan)
}

/// Shortcuts whose local target no longer exists (checked by Windows itself).
fn broken_links(links: &[PathBuf]) -> Vec<PathBuf> {
    if links.is_empty() || !cfg!(windows) {
        return vec![];
    }
    let list = std::env::temp_dir().join(format!("kazma-links-{}.txt", std::process::id()));
    let body: String = links.iter().map(|p| format!("{}\n", p.display())).collect();
    if fs::write(&list, body).is_err() {
        return vec![];
    }
    let script = r#"
$sh = New-Object -ComObject WScript.Shell
Get-Content -LiteralPath $env:KAZMA_LIST -Encoding UTF8 | ForEach-Object {
  try {
    $t = $sh.CreateShortcut($_).TargetPath
    if ($t -and $t -notmatch '^(\\\\|[a-zA-Z]+://)' -and -not (Test-Path -LiteralPath $t)) { $_ }
  } catch {}
}
"#;
    let out = ps::text(script, &[("KAZMA_LIST", &list.to_string_lossy())]).unwrap_or_default();
    let _ = fs::remove_file(&list);
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub fn find(roots: &[PathBuf]) -> Vec<(Kind, PathBuf)> {
    let scan = roots
        .par_iter()
        .map(|r| walk(r).1)
        .reduce(Scan::default, Scan::merge);
    let mut out: Vec<(Kind, PathBuf)> = vec![];
    out.extend(
        scan.empty_dirs
            .into_iter()
            .filter(|p| !roots.contains(p))
            .map(|p| (Kind::EmptyDir, p)),
    );
    out.extend(
        broken_links(&scan.links)
            .into_iter()
            .map(|p| (Kind::BrokenLink, p)),
    );
    out.extend(scan.zero_files.into_iter().map(|p| (Kind::ZeroFile, p)));
    out.retain(|(_, p)| fsutil::can_remove(p));
    out.sort_by(|a, b| (a.0 as u8, &a.1).cmp(&(b.0 as u8, &b.1)));
    out
}

fn default_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = home()
        .map(|h| {
            [
                "Desktop",
                "Documents",
                "Downloads",
                "Pictures",
                "Videos",
                "Music",
            ]
            .iter()
            .map(|d| h.join(d))
            .collect()
        })
        .unwrap_or_default();
    for (var, rest) in [
        ("APPDATA", "Microsoft\\Windows\\Start Menu\\Programs"),
        ("ProgramData", "Microsoft\\Windows\\Start Menu\\Programs"),
    ] {
        if let Some(b) = env_path(var) {
            v.push(b.join(rest));
        }
    }
    v.into_iter().filter(|p| p.is_dir()).collect()
}

fn kind_label(k: Kind) -> String {
    match k {
        Kind::EmptyDir => ui::t("empty folder", "boş klasör").into(),
        Kind::BrokenLink => ui::t("broken shortcut", "bozuk kısayol").into(),
        Kind::ZeroFile => ui::t("0-byte file", "0 baytlık dosya").into(),
    }
}

pub fn run(paths: Vec<PathBuf>, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "empty :: empty folders, broken shortcuts, 0-byte files",
        "empty :: boş klasörler, bozuk kısayollar, 0 baytlık dosyalar",
    ));
    let roots = if paths.is_empty() {
        default_roots()
    } else {
        paths
    };
    for r in &roots {
        println!("{}", tr!("  scanning {}", "  taranıyor {}", r.display()));
    }
    let found = find(&roots);
    if found.is_empty() {
        ui::ok(ui::t("Nothing found.", "Bir şey bulunamadı."));
        return Ok(());
    }
    if dry || !ui::interactive() {
        let mut last = None;
        for (k, p) in &found {
            if last != Some(*k) {
                ui::section(&kind_label(*k));
                last = Some(*k);
            }
            println!("  {}", p.display());
        }
        println!("\n  {}\n", tr!("{} items", "{} öğe", found.len()));
        return Ok(());
    }
    let items = found
        .iter()
        .map(|(k, p)| ui::CheckItem {
            label: p.display().to_string(),
            detail: kind_label(*k),
            size: 0,
            checked: *k != Kind::ZeroFile,
        })
        .collect();
    let Some(sel) = ui::checklist_opts(
        ui::t(
            "kazma empty :: [x] = send to Recycle Bin",
            "kazma empty :: [x] = Geri Dönüşüm Kutusu'na gönder",
        ),
        items,
        false,
    )?
    else {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    };
    ui::section(ui::t(
        "Moving to Recycle Bin",
        "Geri Dönüşüm Kutusu'na taşınıyor",
    ));
    let mut n = 0;
    for ((_, p), on) in found.iter().zip(sel) {
        if !on {
            continue;
        }
        match fsutil::trash(p) {
            Ok(()) => {
                n += 1;
                ui::ok(&p.display().to_string());
                oplog::record("empty", &format!("trashed {}", p.display()));
            }
            Err(e) => ui::warn(&format!("{}  ({e})", p.display())),
        }
    }
    println!("\n  {}\n", tr!("{n} items removed", "{n} öğe kaldırıldı"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_empty_trees_and_zero_files() {
        let base = std::env::temp_dir().join(format!("kazma-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("a/b/c")).unwrap(); // fully empty tree -> report "a" only
        fs::create_dir_all(base.join("full/empty_child")).unwrap();
        fs::write(base.join("full/data.txt"), b"x").unwrap();
        fs::write(base.join("full/zero.bin"), b"").unwrap();
        fs::write(base.join("full/__init__.py"), b"").unwrap();
        fs::create_dir_all(base.join(".git/objects")).unwrap();
        let found = find(std::slice::from_ref(&base));
        let names: Vec<(Kind, String)> = found
            .iter()
            .map(|(k, p)| (*k, p.strip_prefix(&base).unwrap().display().to_string()))
            .collect();
        assert!(names.contains(&(Kind::EmptyDir, "a".into())), "{names:?}");
        assert!(
            !names
                .iter()
                .any(|(_, n)| n.starts_with("a/") || n.starts_with("a\\")),
            "{names:?}"
        );
        assert!(
            names
                .iter()
                .any(|(k, n)| *k == Kind::EmptyDir && n.ends_with("empty_child"))
        );
        assert!(
            names
                .iter()
                .any(|(k, n)| *k == Kind::ZeroFile && n.ends_with("zero.bin"))
        );
        assert!(
            !names
                .iter()
                .any(|(_, n)| n.contains("__init__") || n.contains(".git"))
        );
        fs::remove_dir_all(&base).unwrap();
    }
}
