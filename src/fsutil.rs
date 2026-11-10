//! Filesystem helpers: fast parallel sizing and guarded deletion.

use rayon::prelude::*;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    pub bytes: u64,
    pub files: u64,
    pub failed: u64,
}

impl Stats {
    fn add(self, o: Stats) -> Stats {
        Stats {
            bytes: self.bytes + o.bytes,
            files: self.files + o.files,
            failed: self.failed + o.failed,
        }
    }
}

fn old_enough(meta: &fs::Metadata, cutoff: Option<SystemTime>) -> bool {
    match cutoff {
        None => true,
        Some(c) => meta.modified().map(|m| m <= c).unwrap_or(false),
    }
}

/// Total size of a directory tree. Symlinks and junctions are not followed.
pub fn dir_size(path: &Path) -> u64 {
    scan_tree(path, None).bytes
}

/// Size of a file or directory tree.
pub fn path_size(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => dir_size(path),
        Ok(m) if !m.file_type().is_symlink() => m.len(),
        _ => 0,
    }
}

/// Count bytes/files under `path`, only counting files last modified before `cutoff`.
pub fn scan_tree(path: &Path, cutoff: Option<SystemTime>) -> Stats {
    let Ok(rd) = fs::read_dir(path) else {
        return Stats::default();
    };
    let entries: Vec<fs::DirEntry> = rd.flatten().collect();
    entries
        .par_iter()
        .map(|e| {
            let Ok(ft) = e.file_type() else {
                return Stats::default();
            };
            if ft.is_symlink() {
                Stats::default()
            } else if ft.is_dir() {
                scan_tree(&e.path(), cutoff)
            } else {
                match e.metadata() {
                    Ok(m) if old_enough(&m, cutoff) => Stats { bytes: m.len(), files: 1, failed: 0 },
                    _ => Stats::default(),
                }
            }
        })
        .reduce(Stats::default, Stats::add)
}

fn remove_file_forced(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // Read-only attribute blocks deletion on Windows; clear it and retry.
            if let Ok(meta) = fs::metadata(path) {
                let mut p = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                p.set_readonly(false);
                let _ = fs::set_permissions(path, p);
            }
            fs::remove_file(path)
        }
        Err(e) => Err(e),
    }
}

/// Delete everything inside `dir` (but not `dir` itself) older than `cutoff`.
/// Files that are locked or otherwise undeletable are skipped and counted as failed.
pub fn delete_contents(dir: &Path, cutoff: Option<SystemTime>, dry_run: bool) -> Stats {
    if !is_safe_target(dir) {
        return Stats { failed: 1, ..Default::default() };
    }
    delete_inner(dir, cutoff, dry_run)
}

fn delete_inner(dir: &Path, cutoff: Option<SystemTime>, dry_run: bool) -> Stats {
    let Ok(rd) = fs::read_dir(dir) else {
        return Stats::default();
    };
    let entries: Vec<fs::DirEntry> = rd.flatten().collect();
    entries
        .par_iter()
        .map(|e| {
            let path = e.path();
            let Ok(ft) = e.file_type() else {
                return Stats { failed: 1, ..Default::default() };
            };
            if ft.is_symlink() {
                // Remove the link only, never its target.
                if !dry_run && fs::remove_file(&path).is_err() && fs::remove_dir(&path).is_err() {
                    return Stats { failed: 1, ..Default::default() };
                }
                Stats::default()
            } else if ft.is_dir() {
                let s = delete_inner(&path, cutoff, dry_run);
                if !dry_run {
                    let _ = fs::remove_dir(&path); // only succeeds if now empty
                }
                s
            } else {
                let Ok(meta) = e.metadata() else {
                    return Stats { failed: 1, ..Default::default() };
                };
                if !old_enough(&meta, cutoff) {
                    return Stats::default();
                }
                if dry_run || remove_file_forced(&path).is_ok() {
                    Stats { bytes: meta.len(), files: 1, failed: 0 }
                } else {
                    Stats { failed: 1, ..Default::default() }
                }
            }
        })
        .reduce(Stats::default, Stats::add)
}

/// Delete a whole directory tree (used by purge). Returns stats of what was removed.
pub fn delete_tree(dir: &Path, dry_run: bool) -> Stats {
    if !is_safe_target(dir) {
        return Stats { failed: 1, ..Default::default() };
    }
    let s = delete_inner(dir, None, dry_run);
    if !dry_run {
        let _ = fs::remove_dir(dir);
    }
    s
}

/// Delete a single file.
pub fn delete_file(path: &Path, dry_run: bool) -> Stats {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Stats::default();
    };
    if dry_run || remove_file_forced(path).is_ok() {
        Stats { bytes: meta.len(), files: 1, failed: 0 }
    } else {
        Stats { failed: 1, ..Default::default() }
    }
}

/// Refuse to operate on drive roots, the user profile, system folders and other
/// top-level locations. Every deletion goes through this guard.
pub fn is_safe_target(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let normal = path.components().filter(|c| matches!(c, Component::Normal(_))).count();
    if normal < 2 {
        return false;
    }
    let deny = [
        "USERPROFILE", "HOME", "SystemRoot", "windir", "ProgramFiles", "ProgramFiles(x86)",
        "ProgramData", "LOCALAPPDATA", "APPDATA", "SystemDrive", "PUBLIC",
    ];
    let canon = |p: &Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    let target = canon(path);
    for var in deny {
        if let Some(v) = std::env::var_os(var)
            && canon(Path::new(&v)) == target {
                return false;
            }
    }
    true
}

pub fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from)
}

pub fn home() -> Option<PathBuf> {
    env_path("USERPROFILE").or_else(|| env_path("HOME"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_rejects_roots() {
        // Build paths from the temp dir's root so the test works on C:\ as well as /.
        let tmp = std::env::temp_dir();
        let root: PathBuf = tmp.ancestors().last().unwrap().to_path_buf();
        assert!(!is_safe_target(&root));
        assert!(!is_safe_target(&root.join("top")));
        assert!(!is_safe_target(Path::new("relative/path")));
        assert!(is_safe_target(&root.join("some").join("cache")));
        if let Some(h) = home() {
            assert!(!is_safe_target(&h));
        }
    }

    #[test]
    fn delete_and_size() {
        let base = std::env::temp_dir().join(format!("kazma-test-{}", std::process::id()));
        let d = base.join("cache");
        fs::create_dir_all(d.join("sub")).unwrap();
        fs::write(d.join("a.bin"), vec![0u8; 1000]).unwrap();
        fs::write(d.join("sub/b.bin"), vec![0u8; 500]).unwrap();
        assert_eq!(dir_size(&d), 1500);
        let dry = delete_contents(&d, None, true);
        assert_eq!((dry.bytes, dry.files), (1500, 2));
        assert_eq!(dir_size(&d), 1500);
        let real = delete_contents(&d, None, false);
        assert_eq!((real.bytes, real.files, real.failed), (1500, 2, 0));
        assert!(d.exists());
        assert_eq!(fs::read_dir(&d).unwrap().count(), 0);
        fs::remove_dir_all(&base).unwrap();
    }
}
