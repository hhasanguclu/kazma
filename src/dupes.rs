//! `kz dupes`: fast duplicate file finder.
//!
//! Group by size (free), then hash the first and last 64 KB, then a full BLAKE3
//! hash only for files that still collide. Every stage runs in parallel. Copies
//! go to the Recycle Bin and at least one file of every group is always kept.

use crate::fsutil::{self, home};
use crate::{oplog, tr, ui};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const EDGE: u64 = 64 * 1024;

#[derive(Clone)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn walk(dir: &Path) -> Vec<FileInfo> {
    let Ok(rd) = fs::read_dir(dir) else {
        return vec![];
    };
    let entries: Vec<fs::DirEntry> = rd.flatten().collect();
    entries
        .par_iter()
        .flat_map(|e| {
            let Ok(ft) = e.file_type() else { return vec![] };
            if ft.is_symlink() {
                return vec![];
            }
            if ft.is_dir() {
                let n = e.file_name();
                let n = n.to_string_lossy();
                if n.starts_with('.')
                    || n == "node_modules"
                    || n == "AppData"
                    || n == "$Recycle.Bin"
                {
                    return vec![];
                }
                return walk(&e.path());
            }
            match e.metadata() {
                Ok(m) => vec![FileInfo {
                    path: e.path(),
                    size: m.len(),
                    modified: m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                }],
                Err(_) => vec![],
            }
        })
        .collect()
}

fn partial_hash(f: &FileInfo) -> Option<[u8; 32]> {
    let mut file = File::open(&f.path).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; EDGE as usize];
    let n = file.read(&mut buf).ok()?;
    h.update(&buf[..n]);
    if f.size > 2 * EDGE {
        file.seek(SeekFrom::End(-(EDGE as i64))).ok()?;
        let n = file.read(&mut buf).ok()?;
        h.update(&buf[..n]);
    }
    Some(*h.finalize().as_bytes())
}

fn full_hash(f: &FileInfo) -> Option<[u8; 32]> {
    let mut file = File::open(&f.path).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Some(*h.finalize().as_bytes())
}

/// Split each group by `key`, keeping only sub-groups with 2+ files.
fn refine<K, F>(groups: Vec<Vec<FileInfo>>, key: F) -> Vec<Vec<FileInfo>>
where
    K: Eq + std::hash::Hash + Send,
    F: Fn(&FileInfo) -> Option<K> + Sync,
{
    groups
        .into_par_iter()
        .flat_map(|g| {
            let keyed: Vec<(K, FileInfo)> = g
                .into_par_iter()
                .filter_map(|f| key(&f).map(|k| (k, f)))
                .collect();
            let mut m: HashMap<K, Vec<FileInfo>> = HashMap::new();
            for (k, f) in keyed {
                m.entry(k).or_default().push(f);
            }
            m.into_values().filter(|v| v.len() > 1).collect::<Vec<_>>()
        })
        .collect()
}

pub fn find(roots: &[PathBuf], min_size: u64) -> Vec<Vec<PathBuf>> {
    let files: Vec<FileInfo> = roots
        .par_iter()
        .flat_map(|r| walk(r))
        .filter(|f| f.size >= min_size)
        .collect();
    let mut by_size: HashMap<u64, Vec<FileInfo>> = HashMap::new();
    for f in files {
        by_size.entry(f.size).or_default().push(f);
    }
    let groups: Vec<Vec<FileInfo>> = by_size.into_values().filter(|v| v.len() > 1).collect();
    let groups = refine(groups, partial_hash);
    let groups = refine(groups, |f| {
        if f.size <= 2 * EDGE {
            Some([0u8; 32])
        } else {
            full_hash(f)
        }
    });
    let mut out: Vec<Vec<FileInfo>> = groups
        .into_iter()
        .map(|mut g| {
            // keep the oldest copy first: it is most likely the original
            g.sort_by_key(|f| (f.modified, f.path.as_os_str().len()));
            g
        })
        .collect();
    out.sort_by_key(|g| std::cmp::Reverse(g[0].size * (g.len() as u64 - 1)));
    out.into_iter()
        .map(|g| g.into_iter().map(|f| f.path).collect())
        .collect()
}

fn default_roots() -> Vec<PathBuf> {
    let Some(h) = home() else { return vec![] };
    [
        "Documents",
        "Downloads",
        "Desktop",
        "Pictures",
        "Videos",
        "Music",
        "OneDrive",
    ]
    .iter()
    .map(|d| h.join(d))
    .filter(|p| p.is_dir())
    .collect()
}

pub fn run(paths: Vec<PathBuf>, min_mb: u64, dry: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "dupes :: find duplicate files",
        "dupes :: yinelenen dosyaları bul"
    ));
    let roots = if paths.is_empty() {
        default_roots()
    } else {
        paths
    };
    for r in &roots {
        println!("  {} {}", tr!("scanning", "taranıyor"), r.display());
    }
    let t = std::time::Instant::now();
    let groups = find(&roots, min_mb * 1024 * 1024);
    let sizes: Vec<u64> = groups.iter().map(|g| fsutil::path_size(&g[0])).collect();
    let wasted: u64 = groups
        .iter()
        .zip(&sizes)
        .map(|(g, s)| s * (g.len() as u64 - 1))
        .sum();
    println!(
        "  {}",
        tr!(
            "done in {:.1}s",
            "{:.1} sn'de bitti",
            t.elapsed().as_secs_f64()
        )
    );

    if groups.is_empty() {
        ui::ok(&tr!(
            "No duplicates of {min_mb} MB or more.",
            "{min_mb} MB ve üstü yinelenen dosya yok."
        ));
        return Ok(());
    }
    if dry || !ui::interactive() {
        for (i, (g, s)) in groups.iter().zip(&sizes).enumerate() {
            ui::section(&tr!(
                "#{} {} x {}",
                "#{} {} x {}",
                i + 1,
                g.len(),
                ui::human(*s)
            ));
            for (j, p) in g.iter().enumerate() {
                let tag = if j == 0 {
                    tr!("keep", "tut ")
                } else {
                    tr!("dup ", "kopya")
                };
                println!("  {tag}  {}", p.display());
            }
        }
        println!(
            "\n  {}\n",
            tr!(
                "{} groups, {} reclaimable",
                "{} grup, {} kazanılabilir",
                groups.len(),
                ui::human(wasted)
            )
        );
        return Ok(());
    }

    let mut items = vec![];
    let mut index = vec![];
    for (gi, (g, s)) in groups.iter().zip(&sizes).enumerate() {
        for (j, p) in g.iter().enumerate() {
            items.push(ui::CheckItem {
                label: p.display().to_string(),
                detail: if j == 0 {
                    tr!("#{} keep (oldest)", "#{} tut (en eski)", gi + 1)
                } else {
                    format!("#{}", gi + 1)
                },
                size: *s,
                checked: j > 0,
            });
            index.push((gi, p.clone()));
        }
    }
    let title = tr!(
        "kazma dupes :: [x] = send to Recycle Bin",
        "kazma dupes :: [x] = Geri Dönüşüm Kutusu'na gönder"
    );
    let Some(sel) = ui::checklist(&title, items)? else {
        println!("  {}", tr!("cancelled.", "iptal edildi."));
        return Ok(());
    };

    ui::section(&tr!(
        "Moving to Recycle Bin",
        "Geri Dönüşüm Kutusu'na taşınıyor"
    ));
    let mut freed = 0u64;
    for (gi, g) in groups.iter().enumerate() {
        let chosen: Vec<&PathBuf> = index
            .iter()
            .zip(&sel)
            .filter(|((i, _), on)| *i == gi && **on)
            .map(|((_, p), _)| p)
            .collect();
        let chosen: Vec<&PathBuf> = if chosen.len() == g.len() {
            ui::warn(&tr!(
                "group #{}: all copies selected, keeping {}",
                "grup #{}: hepsi seçili, {} tutuluyor",
                gi + 1,
                g[0].display()
            ));
            chosen.into_iter().filter(|p| **p != g[0]).collect()
        } else {
            chosen
        };
        for p in chosen {
            match fsutil::trash(p) {
                Ok(()) => {
                    freed += sizes[gi];
                    ui::ok(&format!("{:>10}  {}", ui::human(sizes[gi]), p.display()));
                    oplog::record("dupes", &format!("trashed {}", p.display()));
                }
                Err(e) => ui::warn(&format!("{}  ({e})", p.display())),
            }
        }
    }
    println!(
        "\n  {}\n",
        tr!(
            "freed {} (after emptying the Recycle Bin)",
            "{} boşaltıldı (Geri Dönüşüm Kutusu boşaltılınca)",
            ui::human(freed)
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_true_duplicates_only() {
        let base = std::env::temp_dir().join(format!("kazma-dupes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("a/b")).unwrap();
        let big: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let mut other = big.clone();
        other[150_000] ^= 1; // same size, same edges, different middle
        fs::write(base.join("one.bin"), &big).unwrap();
        fs::write(base.join("a/two.bin"), &big).unwrap();
        fs::write(base.join("a/b/three.bin"), &big).unwrap();
        fs::write(base.join("near.bin"), &other).unwrap();
        fs::write(base.join("small1.txt"), b"hello").unwrap();
        fs::write(base.join("small2.txt"), b"hello").unwrap();
        let g = find(std::slice::from_ref(&base), 1000);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].len(), 3);
        assert!(!g[0].iter().any(|p| p.ends_with("near.bin")));
        let g = find(std::slice::from_ref(&base), 0);
        assert_eq!(g.len(), 2);
        fs::remove_dir_all(&base).unwrap();
    }
}
