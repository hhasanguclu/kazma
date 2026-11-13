//! `kz clean`: known-safe caches, temp files, logs and the Recycle Bin.

use crate::fsutil::{self, Stats, env_path, home};
use crate::{oplog, sys, tr, ui};
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
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

/// Cache folders of a Chromium-based browser (Chrome, Edge, Brave, Opera...), across all profiles.
fn chromium(user_data: Option<PathBuf>) -> Vec<PathBuf> {
    let Some(ud) = user_data.filter(|p| p.is_dir()) else {
        return vec![];
    };
    let mut out = vec![Some(ud.join("ShaderCache")), Some(ud.join("GrShaderCache"))];
    for prof in subdirs(&ud) {
        for c in [
            "Cache",
            "Code Cache",
            "GPUCache",
            "DawnCache",
            "DawnGraphiteCache",
        ] {
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

// ---- custom rules ---------------------------------------------------------------

const RULES_TEMPLATE: &str =
    "# kazma custom clean rules: extra folders for `kz clean` (group \"Custom\").
# One rule per line:   name | folder | minimum age in hours (optional)
# The folder's contents are deleted, not the folder itself. %VARS% and ~ work.
# The usual protections still apply (system folders, whitelist).
# Examples:
#   Old app logs   | D:\\MyApp\\logs          | 168
#   Screenshots    | ~\\Pictures\\Screenshots   | 720
";

pub fn rules_file() -> Option<PathBuf> {
    crate::oplog::data_dir().map(|d| d.join("clean-rules.txt"))
}

/// Parse `name | folder | hours` lines; bad lines are skipped.
pub fn parse_rules(text: &str) -> Vec<(String, PathBuf, Option<u64>)> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let mut parts = l.split('|').map(str::trim);
            let name = parts.next().filter(|n| !n.is_empty())?.to_string();
            let path = crate::whitelist::expand(parts.next().filter(|p| !p.is_empty())?);
            let hours = parts.next().and_then(|h| h.parse::<u64>().ok());
            Some((name, path, hours))
        })
        .collect()
}

/// Rules from the app / launcher / developer / AI catalog.
fn catalog_targets() -> Vec<Target> {
    crate::catalog::RULES
        .iter()
        .map(|rule| {
            let found: Vec<PathBuf> = rule
                .paths
                .iter()
                .flat_map(|p| crate::catalog::expand(p))
                .collect();
            let kind = if !found.is_empty() && found.iter().all(|p| p.is_file()) {
                Kind::Files(found)
            } else {
                Kind::Dirs(found.into_iter().filter(|p| p.is_dir()).collect())
            };
            Target {
                group: rule.group,
                name: ui::t(rule.en, rule.tr),
                kind,
                min_age_h: rule.min_age_h,
                admin: rule.admin,
                default_on: rule.on,
            }
        })
        .collect()
}

/// A folder may be named by several rules (e.g. a shader cache); keep it in the
/// first rule only so nothing is counted or cleaned twice.
fn dedupe(mut targets: Vec<Target>) -> Vec<Target> {
    let mut seen = std::collections::HashSet::new();
    for t in &mut targets {
        if let Kind::Dirs(v) | Kind::Files(v) = &mut t.kind {
            v.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
        }
    }
    targets
}

fn custom_targets() -> Vec<Target> {
    let Some(text) = rules_file().and_then(|f| std::fs::read_to_string(f).ok()) else {
        return vec![];
    };
    parse_rules(&text)
        .into_iter()
        .filter(|(_, p, _)| p.is_dir() && fsutil::is_safe_target(p))
        .map(|(name, path, hours)| Target {
            group: "Custom",
            // targets live for the whole run; leaking a few short strings is fine
            name: Box::leak(name.into_boxed_str()),
            kind: Kind::Dirs(vec![path]),
            min_age_h: hours,
            admin: false,
            default_on: true,
        })
        .collect()
}

/// `kz rules`: show the custom rules, or open them in Notepad with --edit.
pub fn rules(edit: bool) -> anyhow::Result<()> {
    let f = rules_file().ok_or_else(|| anyhow::anyhow!("no data folder"))?;
    if !f.exists() {
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&f, RULES_TEMPLATE)?;
    }
    if edit {
        let editor = if cfg!(windows) {
            "notepad".to_string()
        } else {
            std::env::var("EDITOR").unwrap_or_else(|_| "vi".into())
        };
        std::process::Command::new(editor).arg(&f).status()?;
        return Ok(());
    }
    ui::print_banner(ui::t(
        "rules :: your own clean rules",
        "rules :: kendi temizlik kuralların",
    ));
    println!(
        "{}
",
        tr!("  file: {}", "  dosya: {}", f.display())
    );
    let list = parse_rules(&std::fs::read_to_string(&f)?);
    if list.is_empty() {
        println!(
            "  {}",
            ui::t(
                "no rules yet. add some with: kz rules --edit",
                "henüz kural yok. eklemek için: kz rules --edit"
            )
        );
    }
    for (name, path, hours) in list {
        let age = hours
            .map(|h| tr!("  older than {h} h", "  {h} saatten eski"))
            .unwrap_or_default();
        let line = format!("{name:<24} {}{age}", path.display());
        if !path.is_dir() {
            ui::warn(&tr!("{line}  (folder not found)", "{line}  (klasör yok)"));
        } else if !fsutil::is_safe_target(&path) {
            ui::warn(&tr!(
                "{line}  (protected, ignored)",
                "{line}  (korumalı, yok sayıldı)"
            ));
        } else {
            ui::ok(&line);
        }
    }
    println!();
    Ok(())
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
                            let n = p
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase();
                            (n.starts_with("thumbcache_") || n.starts_with("iconcache_"))
                                && n.ends_with(".db")
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let t = |group, name, kind, min_age_h, admin, default_on| Target {
        group,
        name,
        kind,
        min_age_h,
        admin,
        default_on,
    };
    use Kind::*;
    let builtin = vec![
        // --- user essentials
        t(
            "User",
            ui::t(
                "User temp files (>24h)",
                "Kullanıcı geçici dosyaları (>24s)",
            ),
            Dirs(existing([env_path("TEMP").or_else(|| l("Temp"))])),
            Some(24),
            false,
            true,
        ),
        t(
            "User",
            ui::t("Crash dumps", "Çökme dökümleri"),
            Dirs(existing([l("CrashDumps")])),
            None,
            false,
            true,
        ),
        t(
            "User",
            ui::t("Error reports (WER)", "Hata raporları (WER)"),
            Dirs(existing([
                l("Microsoft\\Windows\\WER\\ReportArchive"),
                l("Microsoft\\Windows\\WER\\ReportQueue"),
            ])),
            None,
            false,
            true,
        ),
        t(
            "User",
            ui::t("Thumbnail & icon cache", "Küçük resim ve ikon önbelleği"),
            Files(thumbcache),
            None,
            false,
            false,
        ),
        t(
            "User",
            ui::t("Recycle Bin", "Geri Dönüşüm Kutusu"),
            RecycleBin,
            None,
            false,
            false,
        ),
        // --- browsers
        t(
            "Browser",
            ui::t("Google Chrome cache", "Google Chrome önbelleği"),
            Dirs(chromium(l("Google\\Chrome\\User Data"))),
            None,
            false,
            true,
        ),
        t(
            "Browser",
            ui::t("Microsoft Edge cache", "Microsoft Edge önbelleği"),
            Dirs(chromium(l("Microsoft\\Edge\\User Data"))),
            None,
            false,
            true,
        ),
        t(
            "Browser",
            ui::t("Brave cache", "Brave önbelleği"),
            Dirs(chromium(l("BraveSoftware\\Brave-Browser\\User Data"))),
            None,
            false,
            true,
        ),
        t(
            "Browser",
            ui::t("Vivaldi cache", "Vivaldi önbelleği"),
            Dirs(chromium(l("Vivaldi\\User Data"))),
            None,
            false,
            true,
        ),
        t(
            "Browser",
            ui::t("Opera cache", "Opera önbelleği"),
            Dirs(existing([
                l("Opera Software\\Opera Stable\\Cache"),
                l("Opera Software\\Opera GX Stable\\Cache"),
            ])),
            None,
            false,
            true,
        ),
        t(
            "Browser",
            ui::t("Firefox cache", "Firefox önbelleği"),
            Dirs(existing(firefox)),
            None,
            false,
            true,
        ),
        // --- apps
        t(
            "Apps",
            ui::t("Discord cache", "Discord önbelleği"),
            Dirs(electron(r("discord"))),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("Slack cache", "Slack önbelleği"),
            Dirs(electron(r("Slack"))),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("VS Code cache", "VS Code önbelleği"),
            Dirs(existing([
                r("Code\\Cache"),
                r("Code\\CachedData"),
                r("Code\\Code Cache"),
                r("Code\\GPUCache"),
                r("Code\\logs"),
            ])),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("Microsoft Teams cache", "Microsoft Teams önbelleği"),
            Dirs(electron(l(
                "Packages\\MSTeams_8wekyb3d8bbwe\\LocalCache\\Microsoft\\MSTeams\\EBWebView\\Default",
            ))),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("Spotify cache", "Spotify önbelleği"),
            Dirs(existing([l("Spotify\\Data")])),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("Steam html cache", "Steam html önbelleği"),
            Dirs(existing([l("Steam\\htmlcache")])),
            None,
            false,
            true,
        ),
        t(
            "Apps",
            ui::t("DirectX shader cache", "DirectX gölgelendirici önbelleği"),
            Dirs(existing([
                l("D3DSCache"),
                l("NVIDIA\\DXCache"),
                l("NVIDIA\\GLCache"),
                l("AMD\\DxCache"),
                l("AMD\\DxcCache"),
            ])),
            None,
            false,
            true,
        ),
        // --- developer
        t(
            "Dev",
            ui::t("npm cache", "npm önbelleği"),
            Dirs(existing([l("npm-cache")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("Yarn cache", "Yarn önbelleği"),
            Dirs(existing([l("Yarn\\Cache")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("pnpm store", "pnpm deposu"),
            Dirs(existing([l("pnpm\\store")])),
            None,
            false,
            false,
        ),
        t(
            "Dev",
            ui::t("pip cache", "pip önbelleği"),
            Dirs(existing([l("pip\\Cache")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("NuGet HTTP cache", "NuGet HTTP önbelleği"),
            Dirs(existing([l("NuGet\\v3-cache"), l("NuGet\\plugins-cache")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("Cargo registry cache", "Cargo kayıt önbelleği"),
            Dirs(existing([h(".cargo\\registry\\cache")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("Go build cache", "Go derleme önbelleği"),
            Dirs(existing([l("go-build")])),
            None,
            false,
            true,
        ),
        t(
            "Dev",
            ui::t("Gradle caches", "Gradle önbellekleri"),
            Dirs(existing([h(".gradle\\caches")])),
            None,
            false,
            false,
        ),
        // --- system (needs admin)
        t(
            "System",
            ui::t("Windows temp (>24h)", "Windows geçici dosyaları (>24s)"),
            Dirs(existing([w("Temp")])),
            Some(24),
            true,
            true,
        ),
        t(
            "System",
            ui::t("Windows Update downloads", "Windows Update indirmeleri"),
            Dirs(existing([w("SoftwareDistribution\\Download")])),
            None,
            true,
            true,
        ),
        t(
            "System",
            ui::t(
                "Delivery Optimization cache",
                "Teslim İyileştirme önbelleği",
            ),
            Dirs(existing([w(
                "ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache",
            )])),
            None,
            true,
            true,
        ),
        t(
            "System",
            ui::t("System error reports (WER)", "Sistem hata raporları (WER)"),
            Dirs(existing([
                pd("Microsoft\\Windows\\WER\\ReportArchive"),
                pd("Microsoft\\Windows\\WER\\ReportQueue"),
            ])),
            None,
            true,
            true,
        ),
        t(
            "System",
            ui::t("Old CBS / DISM logs", "Eski CBS / DISM logları"),
            Dirs(existing([w("Logs\\CBS"), w("Logs\\DISM")])),
            Some(24 * 7),
            true,
            false,
        ),
    ];
    let all: Vec<Target> = builtin
        .into_iter()
        .chain(catalog_targets())
        .chain(custom_targets())
        .collect();
    dedupe(all)
        .into_iter()
        .filter(|t| match &t.kind {
            Dirs(v) | Files(v) => !v.is_empty(),
            RecycleBin => cfg!(windows),
        })
        .collect()
}

fn cutoff(t: &Target) -> Option<SystemTime> {
    t.min_age_h
        .map(|h| SystemTime::now() - Duration::from_secs(h * 3600))
}

fn measure(t: &Target) -> Stats {
    match &t.kind {
        Kind::Dirs(v) => v
            .par_iter()
            .map(|d| fsutil::scan_tree(d, cutoff(t)))
            .reduce(Stats::default, |a, b| Stats {
                bytes: a.bytes + b.bytes,
                files: a.files + b.files,
                failed: 0,
            }),
        Kind::Files(v) => Stats {
            bytes: v.iter().map(|p| fsutil::path_size(p)).sum(),
            files: v.len() as u64,
            failed: 0,
        },
        Kind::RecycleBin => sys::recycle_bin_size()
            .map(|(b, n)| Stats {
                bytes: b,
                files: n,
                failed: 0,
            })
            .unwrap_or_default(),
    }
}

fn run_target(t: &Target, dry: bool) -> Stats {
    match &t.kind {
        Kind::Dirs(v) => v
            .iter()
            .map(|d| fsutil::delete_contents(d, cutoff(t), dry))
            .fold(Stats::default(), |a, b| Stats {
                bytes: a.bytes + b.bytes,
                files: a.files + b.files,
                failed: a.failed + b.failed,
            }),
        Kind::Files(v) => {
            v.iter()
                .map(|p| fsutil::delete_file(p, dry))
                .fold(Stats::default(), |a, b| Stats {
                    bytes: a.bytes + b.bytes,
                    files: a.files + b.files,
                    failed: a.failed + b.failed,
                })
        }
        Kind::RecycleBin => {
            let before = measure(t);
            if !dry && !sys::empty_recycle_bin() {
                return Stats {
                    failed: 1,
                    ..Default::default()
                };
            }
            before
        }
    }
}

fn group_name(group: &str) -> &'static str {
    match group {
        "User" => ui::t("User", "Kullanıcı"),
        "Browser" => ui::t("Browser", "Tarayıcı"),
        "Apps" => ui::t("Apps", "Uygulamalar"),
        "Dev" => ui::t("Dev", "Geliştirici"),
        "Custom" => ui::t("Custom", "Özel"),
        "AI" => ui::t("AI tools", "Yapay zekâ araçları"),
        "Games" => ui::t("Games", "Oyunlar"),
        "GPU" => ui::t("Graphics drivers", "Ekran kartı sürücüleri"),
        _ => ui::t("System", "Sistem"),
    }
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    let admin = sys::is_admin();
    ui::print_banner(if dry {
        ui::t(
            "clean :: dry run, nothing will be deleted",
            "clean :: deneme, hiçbir şey silinmeyecek",
        )
    } else {
        ui::t("clean :: reclaim disk space", "clean :: disk alanı kazan")
    });
    println!(
        "{}",
        tr!(
            "  free on system drive: {}   admin: {}",
            "  sistem diskinde boş: {}   yönetici: {}",
            sys::system_drive_free()
                .map(ui::human)
                .unwrap_or_else(|| "?".into()),
            if admin {
                ui::t("yes", "evet")
            } else {
                ui::t(
                    "no (system items skipped)",
                    "hayır (sistem öğeleri atlandı)",
                )
            }
        )
    );
    println!("  {}", ui::t("scanning...", "taranıyor..."));

    let all = targets();
    let (runnable, needs_admin): (Vec<Target>, Vec<Target>) =
        all.into_iter().partition(|t| admin || !t.admin);
    let mut measured: Vec<(Target, Stats)> = runnable
        .into_par_iter()
        .map(|t| {
            let s = measure(&t);
            (t, s)
        })
        .filter(|(_, s)| s.bytes > 0 || s.files > 0)
        .collect();
    measured.sort_by_key(|(t, _)| {
        [
            "User", "Browser", "Apps", "AI", "Games", "GPU", "Dev", "Custom", "System",
        ]
        .iter()
        .position(|g| *g == t.group)
    });

    if measured.is_empty() {
        ui::ok(ui::t(
            "Nothing to clean. Already spotless.",
            "Temizlenecek bir şey yok. Tertemiz.",
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
    let selection: Vec<bool> = if yes || dry || !ui::interactive() {
        measured.iter().map(|(t, _)| t.default_on || yes).collect()
    } else {
        // the group column fits the longest group name in this language
        let gw = measured
            .iter()
            .map(|(t, _)| ui::width(group_name(t.group)))
            .max()
            .unwrap_or(10)
            .clamp(10, 24)
            + 2;
        let items = measured
            .iter()
            .map(|(t, s)| ui::CheckItem {
                label: format!("{}{}", ui::pad(group_name(t.group), gw), t.name),
                detail: tr!("{} files", "{} dosya", s.files),
                size: s.bytes,
                checked: t.default_on,
            })
            .collect();
        match ui::checklist(
            ui::t(
                "kazma clean :: select what to remove",
                "kazma clean :: silinecekleri seç",
            ),
            items,
        )? {
            Some(sel) => sel,
            None => {
                println!("  {}", ui::t("cancelled.", "iptal edildi."));
                return Ok(());
            }
        }
    };

    let mut total = Stats::default();
    let mut group = "";
    for ((t, s), on) in measured.iter().zip(selection) {
        if t.group != group {
            group = t.group;
            ui::section(group_name(group));
        }
        if !on {
            ui::skip(&tr!(
                "{:<34} {:>10}  (skipped)",
                "{:<34} {:>10}  (atlandı)",
                t.name,
                ui::human(s.bytes)
            ));
            continue;
        }
        let r = run_target(t, dry);
        let note = if r.failed > 0 {
            tr!(
                "  ({} in use, left alone)",
                "  ({} kullanımda, dokunulmadı)",
                r.failed
            )
        } else {
            String::new()
        };
        ui::ok(&format!(
            "{} {:>10}{note}",
            ui::pad(t.name, 34),
            ui::human(r.bytes)
        ));
        total.bytes += r.bytes;
        total.files += r.files;
        total.failed += r.failed;
        if !dry {
            oplog::record(
                "clean",
                &format!(
                    "{} freed={} files={} failed={}",
                    t.name, r.bytes, r.files, r.failed
                ),
            );
        }
    }

    if !needs_admin.is_empty() {
        ui::section(ui::t("Needs administrator", "Yönetici gerekli"));
        for t in &needs_admin {
            ui::skip(&tr!(
                "{}  (run kz from an elevated terminal)",
                "{}  (kz'yi yönetici terminalinde çalıştırın)",
                t.name
            ));
        }
    }

    println!("\n  .{}.", "-".repeat(44));
    println!(
        "  | {:<20}{:>22} |",
        if dry {
            ui::t("would free", "kazanılacak")
        } else {
            ui::t("freed", "kazanılan")
        },
        ui::human(total.bytes)
    );
    println!(
        "  | {}{:>22} |",
        ui::pad(ui::t("files", "dosya"), 20),
        total.files
    );
    println!("  '{}'\n", "-".repeat(44));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_parse() {
        let r = parse_rules("# c\nLogs | /var/log/app | 168\nbad line\n | /x\nShots|/tmp/s\n");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "Logs");
        assert_eq!(r[0].1, PathBuf::from("/var/log/app"));
        assert_eq!(r[0].2, Some(168));
        assert_eq!(r[1].2, None);
    }
}
