//! `kz clean`: known-safe caches, temp files, logs and the Recycle Bin.

use crate::fsutil::{self, env_path, home, Stats};
use crate::{oplog, sys, ui};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Clone)]
enum Kind {
    /// Delete the contents of these directories.
    Dirs(Vec<PathBuf>),
    /// Delete these individual files.
    Files(Vec<PathBuf>),
    RecycleBin,
}

#[derive(Clone)]
struct Target {
    group: &'static str,
    name: &'static str,
    kind: Kind,
    /// Only delete files older than this many hours.
    min_age_h: Option<u64>,
    admin: bool,
    /// Pre-selected in the checklist.
    default_on: bool,
}

fn existing(paths: impl IntoIterator<Item = Option<PathBuf>>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = paths.into_iter().flatten().filter(|p| p.is_dir()).collect();
    v.sort();
    v.dedup();
    v
}

fn subdirs(p: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(p)
        .map(|rd| rd.flatten().filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).map(|e| e.path()).collect())
        .unwrap_or_default()
}

/// Cache folders of a Chromium-based browser (Chrome, Edge, Brave, Opera...), across all profiles.
fn chromium(user_data: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(ud) = user_data.filter(|p| p.is_dir()) else { return vec![] };
    let mut out = vec![Some(ud.join("ShaderCache")), Some(ud.join("GrShaderCache"))];
    for prof in subdirs(&ud) {
        for c in ["Cache", "Code Cache", "GPUCache", "DawnCache", "DawnGraphiteCache"] {
            out.push(Some(prof.join(c)));
        }
        out.push(Some(prof.join("Service Worker").join("CacheStorage")));
    }
    existing(out)
}

/// Electron apps keep the same cache layout as Chromium profiles.
fn electron(root: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(r) = root else { return vec![] };
    existing(["Cache", "Code Cache", "GPUCache", "DawnCache", "logs"].map(|c| Some(r.join(c))))
}

fn targets() -> Vec<Target> {
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    let pdata = env_path("ProgramData");
    let win = env_path("SystemRoot").or_else(|| env_path("windir"));
    let home = home();
    let l = |p: &str| local.as_ref().map(|b| b.join(p));
    let r = |p: &str| roaming.as_ref().map(|b| b.join(p));
    let w = |p: &str| win.as_ref().map(|b| b.join(p));
    let pd = |p: &str| pdata.as_ref().map(|b| b.join(p));
    let h = |p: &str| home.as_ref().map(|b| b.join(p));

    let firefox = local
        .as_ref()
        .map(|b| subdirs(&b.join("Mozilla").join("Firefox").join("Profiles")))
        .unwrap_or_default()
        .into_iter()
        .map(|p| Some(p.join("cache2")));

    let thumbcache: Vec<PathBuf> = l("Microsoft\\Windows\\Explorer")
        .map(|d| {
            std::fs::read_dir(d)
                .map(|rd| {
                    rd.flatten()
                        .map(|e| e.path())
                        .filter(|p| {
                            let n = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                            (n.starts_with("thumbcache_") || n.starts_with("iconcache_")) && n.ends_with(".db")
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let t = |group, name, kind, min_age_h, admin, default_on| Target { group, name, kind, min_age_h, admin, default_on };
    use Kind::*;
    vec![
        // --- user essentials
        t("User", "User temp files (>24h)", Dirs(existing([env_path("TEMP").or_else(|| l("Temp"))])), Some(24), false, true),
        t("User", "Crash dumps", Dirs(existing([l("CrashDumps")])), None, false, true),
        t("User", "Error reports (WER)", Dirs(existing([l("Microsoft\\Windows\\WER\\ReportArchive"), l("Microsoft\\Windows\\WER\\ReportQueue")])), None, false, true),
        t("User", "Thumbnail & icon cache", Files(thumbcache), None, false, false),
        t("User", "Recycle Bin", RecycleBin, None, false, false),
        // --- browsers
        t("Browser", "Google Chrome cache", Dirs(chromium(l("Google\\Chrome\\User Data"))), None, false, true),
        t("Browser", "Microsoft Edge cache", Dirs(chromium(l("Microsoft\\Edge\\User Data"))), None, false, true),
        t("Browser", "Brave cache", Dirs(chromium(l("BraveSoftware\\Brave-Browser\\User Data"))), None, false, true),
        t("Browser", "Vivaldi cache", Dirs(chromium(l("Vivaldi\\User Data"))), None, false, true),
        t("Browser", "Opera cache", Dirs(existing([l("Opera Software\\Opera Stable\\Cache"), l("Opera Software\\Opera GX Stable\\Cache")])), None, false, true),
        t("Browser", "Firefox cache", Dirs(existing(firefox)), None, false, true),
        // --- apps
        t("Apps", "Discord cache", Dirs(electron(r("discord"))), None, false, true),
        t("Apps", "Slack cache", Dirs(electron(r("Slack"))), None, false, true),
        t("Apps", "VS Code cache", Dirs(existing([r("Code\\Cache"), r("Code\\CachedData"), r("Code\\Code Cache"), r("Code\\GPUCache"), r("Code\\logs")])), None, false, true),
        t("Apps", "Microsoft Teams cache", Dirs(electron(l("Packages\\MSTeams_8wekyb3d8bbwe\\LocalCache\\Microsoft\\MSTeams\\EBWebView\\Default"))), None, false, true),
        t("Apps", "Spotify cache", Dirs(existing([l("Spotify\\Data")])), None, false, true),
        t("Apps", "Steam html cache", Dirs(existing([l("Steam\\htmlcache")])), None, false, true),
        t("Apps", "DirectX shader cache", Dirs(existing([l("D3DSCache"), l("NVIDIA\\DXCache"), l("NVIDIA\\GLCache"), l("AMD\\DxCache"), l("AMD\\DxcCache")])), None, false, true),
        // --- developer
        t("Dev", "npm cache", Dirs(existing([l("npm-cache")])), None, false, true),
        t("Dev", "Yarn cache", Dirs(existing([l("Yarn\\Cache")])), None, false, true),
        t("Dev", "pnpm store", Dirs(existing([l("pnpm\\store")])), None, false, false),
        t("Dev", "pip cache", Dirs(existing([l("pip\\Cache")])), None, false, true),
        t("Dev", "NuGet HTTP cache", Dirs(existing([l("NuGet\\v3-cache"), l("NuGet\\plugins-cache")])), None, false, true),
        t("Dev", "Cargo registry cache", Dirs(existing([h(".cargo\\registry\\cache")])), None, false, true),
        t("Dev", "Go build cache", Dirs(existing([l("go-build")])), None, false, true),
        t("Dev", "Gradle caches", Dirs(existing([h(".gradle\\caches")])), None, false, false),
        // --- system (needs admin)
        t("System", "Windows temp (>24h)", Dirs(existing([w("Temp")])), Some(24), true, true),
        t("System", "Windows Update downloads", Dirs(existing([w("SoftwareDistribution\\Download")])), None, true, true),
        t("System", "Delivery Optimization cache", Dirs(existing([w("ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache")])), None, true, true),
        t("System", "System error reports (WER)", Dirs(existing([pd("Microsoft\\Windows\\WER\\ReportArchive"), pd("Microsoft\\Windows\\WER\\ReportQueue")])), None, true, true),
        t("System", "Old CBS / DISM logs", Dirs(existing([w("Logs\\CBS"), w("Logs\\DISM")])), Some(24 * 7), true, false),
    ]
    .into_iter()
    .filter(|t| match &t.kind {
        Dirs(v) | Files(v) => !v.is_empty(),
        RecycleBin => cfg!(windows),
    })
    .collect()
}

fn cutoff(t: &Target) -> Option<SystemTime> {
    t.min_age_h.map(|h| SystemTime::now() - Duration::from_secs(h * 3600))
}

fn measure(t: &Target) -> Stats {
    match &t.kind {
        Kind::Dirs(v) => v.par_iter().map(|d| fsutil::scan_tree(d, cutoff(t))).reduce(Stats::default, |a, b| Stats {
            bytes: a.bytes + b.bytes,
            files: a.files + b.files,
            failed: 0,
        }),
        Kind::Files(v) => Stats { bytes: v.iter().map(|p| fsutil::path_size(p)).sum(), files: v.len() as u64, failed: 0 },
        Kind::RecycleBin => sys::recycle_bin_size().map(|(b, n)| Stats { bytes: b, files: n, failed: 0 }).unwrap_or_default(),
    }
}

fn run_target(t: &Target, dry: bool) -> Stats {
    match &t.kind {
        Kind::Dirs(v) => v.iter().map(|d| fsutil::delete_contents(d, cutoff(t), dry)).fold(Stats::default(), |a, b| Stats {
            bytes: a.bytes + b.bytes,
            files: a.files + b.files,
            failed: a.failed + b.failed,
        }),
        Kind::Files(v) => v.iter().map(|p| fsutil::delete_file(p, dry)).fold(Stats::default(), |a, b| Stats {
            bytes: a.bytes + b.bytes,
            files: a.files + b.files,
            failed: a.failed + b.failed,
        }),
        Kind::RecycleBin => {
            let before = measure(t);
            if !dry && !sys::empty_recycle_bin() {
                return Stats { failed: 1, ..Default::default() };
            }
            before
        }
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    let admin = sys::is_admin();
    ui::print_banner(if dry { "clean :: dry run, nothing will be deleted" } else { "clean :: reclaim disk space" });
    println!(
        "  free on system drive: {}   admin: {}",
        sys::system_drive_free().map(ui::human).unwrap_or_else(|| "?".into()),
        if admin { "yes" } else { "no (system items skipped)" }
    );
    println!("  scanning...");

    let all = targets();
    let (runnable, needs_admin): (Vec<Target>, Vec<Target>) = all.into_iter().partition(|t| admin || !t.admin);
    let mut measured: Vec<(Target, Stats)> =
        runnable.into_par_iter().map(|t| { let s = measure(&t); (t, s) }).filter(|(_, s)| s.bytes > 0 || s.files > 0).collect();
    measured.sort_by_key(|(t, _)| ["User", "Browser", "Apps", "Dev", "System"].iter().position(|g| *g == t.group));

    if measured.is_empty() {
        ui::ok("Nothing to clean. Already spotless.");
        return Ok(());
    }

    if !dry && !yes && !ui::interactive() {
        ui::warn("not an interactive terminal: re-run with --yes to delete, or --dry-run to preview");
        return Ok(());
    }
    let selection: Vec<bool> = if yes || dry || !ui::interactive() {
        measured.iter().map(|(t, _)| t.default_on || yes).collect()
    } else {
        let items = measured
            .iter()
            .map(|(t, s)| ui::CheckItem {
                label: format!("{:<8}{}", t.group, t.name),
                detail: format!("{} files", s.files),
                size: s.bytes,
                checked: t.default_on,
            })
            .collect();
        match ui::checklist("kazma clean :: select what to remove", items)? {
            Some(sel) => sel,
            None => {
                println!("  cancelled.");
                return Ok(());
            }
        }
    };

    let mut total = Stats::default();
    let mut group = "";
    for ((t, s), on) in measured.iter().zip(selection) {
        if t.group != group {
            group = t.group;
            ui::section(group);
        }
        if !on {
            ui::skip(&format!("{:<34} {:>10}  (skipped)", t.name, ui::human(s.bytes)));
            continue;
        }
        let r = run_target(t, dry);
        let note = if r.failed > 0 { format!("  ({} in use, left alone)", r.failed) } else { String::new() };
        ui::ok(&format!("{:<34} {:>10}{note}", t.name, ui::human(r.bytes)));
        total.bytes += r.bytes;
        total.files += r.files;
        total.failed += r.failed;
        if !dry {
            oplog::record("clean", &format!("{} freed={} files={} failed={}", t.name, r.bytes, r.files, r.failed));
        }
    }

    if !needs_admin.is_empty() {
        ui::section("Needs administrator");
        for t in &needs_admin {
            ui::skip(&format!("{}  (run kz from an elevated terminal)", t.name));
        }
    }

    println!("\n  +{}+", "-".repeat(44));
    println!(
        "  | {:<20}{:>22} |",
        if dry { "would free" } else { "freed" },
        ui::human(total.bytes)
    );
    println!("  | {:<20}{:>22} |", "files", total.files);
    println!("  +{}+\n", "-".repeat(44));
    Ok(())
}
