//! `kz types`: what kind of files fill a folder or drive (video, images, archives...).
//! Uses the raw MFT when possible (administrator + NTFS), else a parallel walk.

use crate::{mft, tr, ui};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// (category key, english, turkish, extensions)
const CATEGORIES: &[(&str, &str, &str, &[&str])] = &[
    (
        "video",
        "Video",
        "Video",
        &[
            "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "m2ts", "vob",
            "3gp",
        ],
    ),
    (
        "image",
        "Images",
        "Resimler",
        &[
            "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tif", "tiff", "raw",
            "cr2", "cr3", "nef", "arw", "dng", "svg", "ico", "psd", "xcf",
        ],
    ),
    (
        "audio",
        "Audio",
        "Ses",
        &[
            "mp3", "flac", "wav", "aac", "ogg", "m4a", "wma", "opus", "aiff", "mid",
        ],
    ),
    (
        "archive",
        "Archives & disk images",
        "Arşivler ve disk kalıpları",
        &[
            "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "iso", "img", "cab", "vhd",
            "vhdx", "vmdk", "wim", "esd",
        ],
    ),
    (
        "doc",
        "Documents",
        "Belgeler",
        &[
            "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "txt", "rtf",
            "md", "csv", "epub", "one",
        ],
    ),
    (
        "program",
        "Programs & libraries",
        "Programlar ve kütüphaneler",
        &[
            "exe", "dll", "msi", "msix", "appx", "sys", "so", "ocx", "mui", "cpl", "drv",
        ],
    ),
    (
        "dev",
        "Code & build output",
        "Kod ve derleme çıktısı",
        &[
            "rs", "js", "ts", "py", "java", "c", "cpp", "h", "hpp", "cs", "go", "json", "xml",
            "html", "css", "pdb", "obj", "o", "lib", "a", "rlib", "rmeta", "class", "jar", "node",
            "wasm", "pyc",
        ],
    ),
    (
        "data",
        "Data, caches & logs",
        "Veri, önbellek ve loglar",
        &[
            "dat", "db", "sqlite", "log", "tmp", "bak", "cache", "etl", "evtx", "dmp", "pak",
            "bin", "blob", "ldb", "idx",
        ],
    ),
];

pub fn category_of(ext: &str) -> usize {
    CATEGORIES
        .iter()
        .position(|(_, _, _, list)| list.contains(&ext))
        .unwrap_or(CATEGORIES.len())
}

fn category_name(i: usize) -> &'static str {
    match CATEGORIES.get(i) {
        Some((_, en, trn, _)) => ui::t(en, trn),
        None => ui::t("Other", "Diğer"),
    }
}

/// (extension, bytes, files) by walking the folder in parallel.
pub fn walk(dir: &Path) -> Vec<(String, u64, u64)> {
    fn go(dir: &Path) -> HashMap<String, (u64, u64)> {
        let Ok(rd) = fs::read_dir(dir) else {
            return HashMap::new();
        };
        let entries: Vec<fs::DirEntry> = rd.flatten().collect();
        entries
            .par_iter()
            .map(|e| {
                let Ok(ft) = e.file_type() else {
                    return HashMap::new();
                };
                if ft.is_symlink() {
                    HashMap::new()
                } else if ft.is_dir() {
                    go(&e.path())
                } else {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    HashMap::from([(mft::ext_of(&e.file_name().to_string_lossy()), (size, 1))])
                }
            })
            .reduce(HashMap::new, |mut a, b| {
                for (k, (s, c)) in b {
                    let e = a.entry(k).or_default();
                    e.0 += s;
                    e.1 += c;
                }
                a
            })
    }
    go(dir).into_iter().map(|(k, (b, c))| (k, b, c)).collect()
}

fn via_mft(dir: &Path) -> Option<Vec<(String, u64, u64)>> {
    mft::scan_drive(dir).ok()?.by_extension(dir)
}

fn show(rows: &[(String, u64, u64)], method: &str, secs: f64) {
    let total: u64 = rows.iter().map(|r| r.1).sum();
    let files: u64 = rows.iter().map(|r| r.2).sum();
    let mut cats = vec![(0u64, 0u64); CATEGORIES.len() + 1];
    for (ext, b, c) in rows {
        let i = category_of(ext);
        cats[i].0 += b;
        cats[i].1 += c;
    }
    let mut order: Vec<usize> = (0..cats.len()).filter(|&i| cats[i].1 > 0).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(cats[i].0));
    let top = order.first().map(|&i| cats[i].0).unwrap_or(1).max(1);

    println!(
        "{}\n",
        tr!(
            "  {} in {} files  ({method}, {:.1}s)",
            "  {1} dosyada {0}  ({method}, {2:.1} sn)",
            ui::human(total),
            files,
            secs
        )
    );
    ui::section(ui::t("By type", "Türe göre"));
    for i in order {
        let (b, c) = cats[i];
        let pct = if total > 0 {
            b as f64 * 100.0 / total as f64
        } else {
            0.0
        };
        println!(
            "  {} {:>10} {:>5.1}%  {}  {}",
            ui::pad(category_name(i), 28),
            ui::human(b),
            pct,
            ui::cbar(b as f64 / top as f64, 24),
            tr!("{c} files", "{c} dosya")
        );
    }
    let mut exts: Vec<&(String, u64, u64)> = rows.iter().collect();
    exts.sort_by_key(|r| std::cmp::Reverse(r.1));
    ui::section(ui::t("Top extensions", "En çok yer kaplayan uzantılar"));
    for (ext, b, c) in exts.into_iter().take(15) {
        let name = if ext.is_empty() {
            ui::t("(none)", "(yok)").to_string()
        } else {
            format!(".{ext}")
        };
        println!(
            "  {:<12} {:>10}  {:>8}  {}",
            name,
            ui::human(*b),
            c,
            ui::cbar(*b as f64 / top as f64, 24)
        );
    }
    println!();
}

pub fn run(path: Option<PathBuf>, method: String, verify: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "types :: what kind of files take the space",
        "types :: hangi tür dosyalar yer kaplıyor",
    ));
    let dir = path
        .or_else(crate::fsutil::home)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = fs::canonicalize(&dir).unwrap_or(dir);
    let dir = PathBuf::from(dir.to_string_lossy().trim_start_matches(r"\\?\"));
    println!("{}", tr!("  folder: {}", "  klasör: {}", dir.display()));

    if verify {
        let t = Instant::now();
        let a = via_mft(&dir).ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                ui::t(
                    "MFT scan not possible here (admin + NTFS needed)",
                    "MFT taraması burada yapılamıyor (yönetici + NTFS gerekli)"
                )
            )
        })?;
        let mft_secs = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let b = walk(&dir);
        let walk_secs = t.elapsed().as_secs_f64();
        let sum = |v: &[(String, u64, u64)]| {
            (
                v.iter().map(|r| r.1).sum::<u64>(),
                v.iter().map(|r| r.2).sum::<u64>(),
            )
        };
        let (ma, mb) = (sum(&a), sum(&b));
        println!("  MFT : {} / {} files  {:.2}s", ma.0, ma.1, mft_secs);
        println!("  walk: {} / {} files  {:.2}s", mb.0, mb.1, walk_secs);
        let mut x = a.clone();
        let mut y = b.clone();
        x.sort();
        y.sort();
        if x != y {
            anyhow::bail!("MFT and walk disagree");
        }
        ui::ok(ui::t(
            "MFT and walk agree",
            "MFT ve normal tarama aynı sonucu verdi",
        ));
        return Ok(());
    }

    let t = Instant::now();
    let (rows, used) = match method.as_str() {
        "walk" => (walk(&dir), "walk"),
        "mft" => (
            via_mft(&dir).ok_or_else(|| {
                anyhow::anyhow!(
                    "{}",
                    ui::t(
                        "MFT scan not possible here (admin + NTFS needed)",
                        "MFT taraması burada yapılamıyor (yönetici + NTFS gerekli)"
                    )
                )
            })?,
            "MFT",
        ),
        _ => match via_mft(&dir) {
            Some(r) => (r, "MFT"),
            None => (walk(&dir), "walk"),
        },
    };
    show(&rows, used, t.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories() {
        assert_eq!(CATEGORIES[category_of("mkv")].0, "video");
        assert_eq!(CATEGORIES[category_of("jpg")].0, "image");
        assert_eq!(category_of("weird"), CATEGORIES.len());
        let mut all: Vec<&str> = CATEGORIES
            .iter()
            .flat_map(|c| c.3.iter().copied())
            .collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "an extension is in two categories");
    }

    #[test]
    fn walk_counts() {
        let base = std::env::temp_dir().join(format!("kazma-types-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("x")).unwrap();
        fs::write(base.join("a.MP4"), vec![0u8; 100]).unwrap();
        fs::write(base.join("x/b.mp4"), vec![0u8; 50]).unwrap();
        fs::write(base.join("x/noext"), vec![0u8; 7]).unwrap();
        let mut r = walk(&base);
        r.sort();
        assert_eq!(r, vec![("".into(), 7, 1), ("mp4".into(), 150, 2)]);
        fs::remove_dir_all(&base).unwrap();
    }
}
