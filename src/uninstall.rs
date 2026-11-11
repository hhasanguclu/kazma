//! `kz uninstall`: remove installed programs with their own uninstaller, then
//! offer to send leftover folders (AppData, ProgramData, install dir) to the Recycle Bin.

use crate::fsutil::{self, env_path};
use crate::{oplog, ps, tr, ui};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

pub const LIST: &str = r#"
$roots = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
         'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
         'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
@(Get-ItemProperty $roots -ErrorAction SilentlyContinue |
  Where-Object { $_.DisplayName -and $_.UninstallString -and $_.SystemComponent -ne 1 -and -not $_.ParentKeyName -and -not $_.ReleaseType } |
  ForEach-Object {
    [pscustomobject]@{
      Name      = [string]$_.DisplayName
      Version   = [string]$_.DisplayVersion
      Publisher = [string]$_.Publisher
      SizeKb    = [int64]$_.EstimatedSize
      Location  = [string]$_.InstallLocation
      Uninstall = [string]$_.UninstallString
      Quiet     = [string]$_.QuietUninstallString
      Key       = [string]$_.PSChildName
    }
  }) | ConvertTo-Json -Compress
"#;

const STILL_INSTALLED: &str = r#"
$k = $env:KAZMA_KEY
$hit = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\',
       'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\',
       'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' | Where-Object { Test-Path ($_ + $k) }
if ($hit) { 'yes' } else { 'no' }
"#;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub size_kb: i64,
    #[serde(default)]
    pub location: String,
    pub uninstall: String,
    #[serde(default)]
    pub quiet: String,
    pub key: String,
}

/// "7-Zip 23.01 (x64)" -> "7-Zip": drop (...) parts and version-like words.
pub fn base_name(name: &str) -> String {
    let mut s = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = i32::max(depth - 1, 0),
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            let v = w.trim_start_matches(['v', 'V']);
            let version = !v.is_empty()
                && v.chars().all(|c| c.is_ascii_digit() || c == '.')
                && v.chars().any(|c| c.is_ascii_digit());
            !version
                && !matches!(
                    w.to_lowercase().as_str(),
                    "x64" | "x86" | "64-bit" | "32-bit"
                )
        })
        .collect();
    words
        .join(" ")
        .trim_matches(|c: char| c == '-' || c.is_whitespace())
        .to_string()
}

/// Names too generic to delete as "leftovers" because other programs share them.
fn generic(name: &str) -> bool {
    let n = name.to_lowercase();
    n.len() < 3
        || [
            "microsoft",
            "windows",
            "google",
            "intel",
            "nvidia",
            "amd",
            "packages",
            "programs",
            "temp",
            "common files",
            "mozilla",
            "apple",
            "adobe",
            "steam",
            "microsoft corporation",
            "jetbrains",
            "oracle",
        ]
        .contains(&n.as_str())
}

/// Find leftover folders after an uninstall. Only exact folder-name matches are
/// considered, and everything goes through the same guards as other deletions.
pub fn leftovers(p: &Program) -> Vec<PathBuf> {
    let name = base_name(&p.name);
    let publisher = base_name(&p.publisher);
    let bases: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA", "ProgramData"]
        .iter()
        .filter_map(|v| env_path(v))
        .chain(env_path("LOCALAPPDATA").map(|l| l.join("Programs")))
        .collect();
    let mut out = vec![];
    if !p.location.trim().is_empty() {
        out.push(PathBuf::from(p.location.trim().trim_matches('"')));
    }
    if !generic(&name) {
        for b in &bases {
            out.push(b.join(&name));
            if !publisher.is_empty() && !publisher.eq_ignore_ascii_case(&name) {
                out.push(b.join(&publisher).join(&name));
            }
        }
    }
    out.sort();
    out.dedup();
    out.into_iter()
        .filter(|d| d.is_dir() && fsutil::can_remove(d))
        .collect()
}

fn still_installed(key: &str) -> bool {
    ps::powershell(STILL_INSTALLED, &[("KAZMA_KEY", key)])
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "yes")
        .unwrap_or(false)
}

/// The command line to run. MSI products are removed with msiexec /x so the
/// standard Windows Installer progress dialog is shown.
fn command_line(p: &Program, quiet: bool) -> String {
    let u = p.uninstall.trim();
    let lower = u.to_lowercase();
    if let (true, Some(a), Some(b)) = (lower.contains("msiexec"), u.find('{'), u.find('}'))
        && b > a
    {
        let mode = if quiet { "/qn" } else { "/passive" };
        return format!("msiexec.exe /x {} {mode} /norestart", &u[a..=b]);
    }
    if quiet && !p.quiet.trim().is_empty() {
        p.quiet.trim().to_string()
    } else {
        u.to_string()
    }
}

#[cfg(windows)]
fn run_command_line(line: &str) -> std::io::Result<std::process::ExitStatus> {
    use std::os::windows::process::CommandExt;
    // `/s /c "<line>"`: cmd strips exactly the outer quotes, leaving the vendor's
    // own quoting intact.
    Command::new("cmd")
        .raw_arg(format!("/d /s /c \"{line}\""))
        .status()
}

#[cfg(not(windows))]
fn run_command_line(line: &str) -> std::io::Result<std::process::ExitStatus> {
    Command::new("sh").arg("-c").arg(line).status()
}

fn pause(msg: &str) {
    use std::io::Write;
    print!("  .. {msg}");
    let _ = std::io::stdout().flush();
    let _ = std::io::stdin().read_line(&mut String::new());
}

// ---- Microsoft Store apps and Scoop packages -------------------------------------------

const STORE_LIST: &str = r#"
@(Get-AppxPackage | Where-Object { -not $_.IsFramework -and -not $_.NonRemovable -and $_.SignatureKind -eq 'Store' -and $_.Name -ne 'Microsoft.WindowsStore' } | ForEach-Object {
  $n = $_.Name
  try { $d = (Get-AppxPackageManifest $_).Package.Properties.DisplayName; if ($d -and $d -notlike 'ms-resource*') { $n = $d } } catch {}
  [pscustomobject]@{
    Name = [string]$n; Version = [string]$_.Version
    Publisher = [string]($_.Publisher -replace '^CN=([^,]+).*', '$1')
    SizeKb = 0; Location = ''; Uninstall = 'appx'; Quiet = ''; Key = 'appx:' + $_.PackageFullName
  }
}) | ConvertTo-Json -Compress
"#;

fn store_apps() -> Vec<Program> {
    ps::powershell_json(STORE_LIST, &[]).unwrap_or_default()
}

fn scoop_root() -> Option<PathBuf> {
    env_path("SCOOP")
        .or_else(|| crate::fsutil::home().map(|h| h.join("scoop")))
        .filter(|p| p.join("apps").is_dir())
}

fn scoop_apps() -> Vec<Program> {
    let Some(root) = scoop_root() else {
        return vec![];
    };
    std::fs::read_dir(root.join("apps"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n != "scoop")
                .map(|n| {
                    let version = std::fs::read_dir(root.join("apps").join(&n))
                        .map(|rd| {
                            rd.flatten()
                                .map(|e| e.file_name().to_string_lossy().into_owned())
                                .filter(|v| v != "current")
                                .last()
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    Program {
                        name: n.clone(),
                        version,
                        publisher: "Scoop".into(),
                        size_kb: 0,
                        location: String::new(),
                        uninstall: "scoop".into(),
                        quiet: String::new(),
                        key: format!("scoop:{n}"),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn source_tag(p: &Program) -> &'static str {
    if p.key.starts_with("appx:") {
        "[Store] "
    } else if p.key.starts_with("scoop:") {
        "[Scoop] "
    } else {
        ""
    }
}

/// Run the right uninstaller for each kind of program.
fn uninstall_one(p: &Program, quiet: bool) -> Result<(), String> {
    if let Some(full) = p.key.strip_prefix("appx:") {
        return ps::text(
            "Remove-AppxPackage -Package $env:KAZMA_PKG",
            &[("KAZMA_PKG", full)],
        )
        .map(|_| ())
        .map_err(|e| e.to_string());
    }
    if let Some(name) = p.key.strip_prefix("scoop:") {
        return ps::text("scoop uninstall $env:KAZMA_APP", &[("KAZMA_APP", name)])
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    let line = command_line(p, quiet);
    println!(
        "{}",
        tr!(
            "  running: {}",
            "  çalıştırılıyor: {}",
            ui::truncate(&line, 90)
        )
    );
    run_command_line(&line)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn is_installed(p: &Program) -> bool {
    if let Some(full) = p.key.strip_prefix("appx:") {
        return ps::text(
            "if (Get-AppxPackage | Where-Object PackageFullName -eq $env:KAZMA_PKG) { 'yes' }",
            &[("KAZMA_PKG", full)],
        )
        .map(|o| o.trim() == "yes")
        .unwrap_or(false);
    }
    if let Some(name) = p.key.strip_prefix("scoop:") {
        return scoop_root().is_some_and(|r| r.join("apps").join(name).is_dir());
    }
    still_installed(&p.key)
}

// ---- registry leftovers ---------------------------------------------------------------------

/// Registry keys named exactly after the program (or publisher\program) that still exist.
pub fn registry_leftovers(p: &Program) -> Vec<String> {
    if !cfg!(windows) || p.key.starts_with("appx:") || p.key.starts_with("scoop:") {
        return vec![];
    }
    let name = base_name(&p.name);
    let publisher = base_name(&p.publisher);
    if generic(&name) {
        return vec![];
    }
    let mut cands = vec![];
    for root in [
        r"HKCU\Software",
        r"HKLM\Software",
        r"HKLM\Software\WOW6432Node",
    ] {
        cands.push(format!(r"{root}\{name}"));
        if !publisher.is_empty() && !generic(&publisher) && !publisher.eq_ignore_ascii_case(&name) {
            cands.push(format!(r"{root}\{publisher}\{name}"));
        }
    }
    let script =
        "$env:KAZMA_KEYS -split ';' | Where-Object { Test-Path -LiteralPath (\"Registry::$_\") }";
    ps::text(script, &[("KAZMA_KEYS", &cands.join(";"))])
        .map(|o| {
            o.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Export the key to a .reg file in kazma's backup folder, then delete it.
fn remove_registry_key(key: &str) -> Result<PathBuf, String> {
    let dir = crate::oplog::data_dir()
        .ok_or("no data folder")?
        .join("backups")
        .join("registry");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let file = dir.join(format!(
        "{}-{safe}.reg",
        crate::oplog::now_utc().replace([' ', ':'], "-")
    ));
    let (ok, out) = ps::exec("reg", &["export", key, &file.to_string_lossy(), "/y"]);
    if !ok {
        return Err(ps::last_line(&out));
    }
    let (ok, out) = ps::exec("reg", &["delete", key, "/f"]);
    if !ok {
        return Err(ps::last_line(&out));
    }
    Ok(file)
}

// ---- orphaned leftovers (apps removed some other way) ----------------------------------------

/// Folder names that belong to Windows or to many programs, never orphans.
const KEEP: &[&str] = &[
    "microsoft",
    "packages",
    "temp",
    "programs",
    "comms",
    "connecteddevicesplatform",
    "d3dscache",
    "crashdumps",
    "publishers",
    "virtualstore",
    "peerdistrepub",
    "kazma",
    "npm",
    "npm-cache",
    "pip",
    "nuget",
    "package cache",
    "nvidia",
    "nvidia corporation",
    "amd",
    "intel",
    "google",
    "mozilla",
    "apple",
    "adobe",
    "windows",
    "ssh",
    "softwaredistribution",
    "usoshared",
    "usoprivate",
    "packagemanagement",
    "regid.1991-06.com.microsoft",
    "microsoft help",
    "desktop",
    "documents",
    "start menu",
    "sun",
    "oracle",
    "history",
    "identities",
    "media center programs",
];

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Does folder `name` look like it belongs to one of the installed programs?
pub fn belongs(name: &str, installed: &[String]) -> bool {
    let n = normalize(name);
    if n.len() < 3 {
        return true;
    }
    installed
        .iter()
        .any(|i| i.len() >= 3 && (i.contains(&n) || (n.len() >= 4 && n.contains(i.as_str()))))
}

pub fn orphans(installed: &[Program]) -> Vec<(PathBuf, u64)> {
    let names: Vec<String> = installed
        .iter()
        .flat_map(|p| {
            [
                normalize(&base_name(&p.name)),
                normalize(&base_name(&p.publisher)),
                normalize(&p.name),
            ]
        })
        .filter(|n| n.len() >= 3)
        .collect();
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(90 * 86_400);
    let bases: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA", "ProgramData"]
        .iter()
        .filter_map(|v| env_path(v))
        .chain(env_path("LOCALAPPDATA").map(|l| l.join("Programs")))
        .collect();
    let mut out = vec![];
    for b in bases {
        let Ok(rd) = std::fs::read_dir(&b) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if !ft.is_dir()
                || ft.is_symlink()
                || name.starts_with('.')
                || name.starts_with('{')
                || KEEP.contains(&lower.as_str())
            {
                continue;
            }
            if belongs(&name, &names) {
                continue;
            }
            let recent = e
                .metadata()
                .and_then(|m| m.modified())
                .map(|m| m > cutoff)
                .unwrap_or(true);
            if recent || !fsutil::can_remove(&e.path()) {
                continue;
            }
            out.push((e.path(), fsutil::dir_size(&e.path())));
        }
    }
    out.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
    out
}

/// `kz uninstall --leftovers`: folders left behind by programs that are gone.
pub fn run_orphans() -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "uninstall :: leftovers of removed programs",
        "uninstall :: kaldırılmış programların kalıntıları",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz uninstall works on Windows only.",
            "kz uninstall yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    ps::working(ui::t(
        "comparing folders with installed programs...",
        "klasörler yüklü programlarla karşılaştırılıyor...",
    ));
    let mut installed: Vec<Program> = ps::powershell_json(LIST, &[]).unwrap_or_default();
    installed.extend(store_apps());
    installed.extend(scoop_apps());
    let found = orphans(&installed);
    println!("{:70}", "");
    if found.is_empty() {
        ui::ok(ui::t(
            "No leftovers of removed programs found.",
            "Kaldırılmış programlardan kalıntı bulunamadı.",
        ));
        return Ok(());
    }
    if !ui::interactive() {
        for (d, sz) in &found {
            println!("  {:>10}  {}", ui::human(*sz), d.display());
        }
        return Ok(());
    }
    let items = found
        .iter()
        .map(|(d, sz)| ui::CheckItem {
            label: d.display().to_string(),
            detail: ui::t(
                "no installed program owns it; untouched for 90+ days",
                "hiçbir yüklü programa ait değil; 90+ gündür dokunulmadı",
            )
            .into(),
            size: *sz,
            checked: false,
        })
        .collect();
    let title = ui::t(
        "kazma uninstall :: check before removing -> Recycle Bin",
        "kazma uninstall :: silmeden önce kontrol edin -> Geri Dönüşüm Kutusu",
    );
    let Some(sel) = ui::checklist(title, items)? else {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    };
    for ((d, sz), on) in found.iter().zip(sel) {
        if !on {
            continue;
        }
        match fsutil::trash(d) {
            Ok(()) => {
                ui::ok(&format!("{:>10}  {}", ui::human(*sz), d.display()));
                oplog::record("uninstall", &format!("trashed orphan {}", d.display()));
            }
            Err(e) => ui::warn(&format!("{}  ({e})", d.display())),
        }
    }
    println!();
    Ok(())
}

pub fn run(dry: bool, quiet: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t(
            "uninstall :: dry run, nothing will be removed",
            "uninstall :: deneme, hiçbir şey kaldırılmayacak",
        )
    } else {
        ui::t(
            "uninstall :: remove programs and their leftovers",
            "uninstall :: programları ve kalıntılarını kaldır",
        )
    });
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz uninstall works on Windows only.",
            "kz uninstall yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    if !dry && !ui::interactive() {
        ui::warn(ui::t(
            "kz uninstall needs an interactive terminal (or use --dry-run to list programs).",
            "kz uninstall etkileşimli terminal ister (listelemek için --dry-run kullanın).",
        ));
        return Ok(());
    }
    ps::working(ui::t(
        "reading installed programs...",
        "yüklü programlar okunuyor...",
    ));
    let mut progs: Vec<Program> = ps::powershell_json(LIST, &[])?;
    progs.extend(store_apps());
    progs.extend(scoop_apps());
    println!("{:60}", "");
    progs.sort_by_key(|p| p.name.to_lowercase());
    progs.dedup_by(|a, b| a.name == b.name && a.version == b.version);

    if dry {
        ui::section(&tr!(
            "{} installed programs",
            "{} yüklü program",
            progs.len()
        ));
        for p in &progs {
            println!(
                "  {:<44} {:<16} {:>10}  {}",
                ui::truncate(&p.name, 44),
                ui::truncate(&p.version, 16),
                if p.size_kb > 0 {
                    ui::human(p.size_kb as u64 * 1024)
                } else {
                    "-".into()
                },
                ui::truncate(&p.publisher, 24)
            );
        }
        println!();
        return Ok(());
    }

    let items = progs
        .iter()
        .map(|p| ui::CheckItem {
            label: format!("{}{}", source_tag(p), p.name),
            detail: format!("{}  {}", p.version, p.publisher),
            size: p.size_kb.max(0) as u64 * 1024,
            checked: false,
        })
        .collect();
    let Some(sel) = ui::checklist(
        ui::t(
            "kazma uninstall :: select programs to remove",
            "kazma uninstall :: kaldırılacak programları seç",
        ),
        items,
    )?
    else {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    };
    let chosen: Vec<&Program> = progs
        .iter()
        .zip(&sel)
        .filter(|(_, on)| **on)
        .map(|(p, _)| p)
        .collect();
    if chosen.is_empty() {
        println!("  {}", ui::t("nothing selected.", "hiçbir şey seçilmedi."));
        return Ok(());
    }

    ui::section(ui::t("Will uninstall", "Kaldırılacaklar"));
    for p in &chosen {
        println!("  - {} {}", p.name, p.version);
    }
    println!();
    if !crate::remove::ask(&tr!(
        "Uninstall {} program(s)?",
        "{} program kaldırılsın mı?",
        chosen.len()
    )) {
        println!("  {}", ui::t("cancelled.", "iptal edildi."));
        return Ok(());
    }

    let mut removed: Vec<&Program> = vec![];
    for p in &chosen {
        ui::section(&p.name);
        if let Err(e) = uninstall_one(p, quiet) {
            ui::warn(&tr!(
                "could not start the uninstaller: {e}",
                "kaldırma programı başlatılamadı: {e}"
            ));
            continue;
        }
        if is_installed(p) {
            // Many uninstallers (NSIS, Inno) hand off to a copy of themselves and exit at once.
            pause(ui::t(
                "if the uninstaller window is still open, finish it, then press Enter...",
                "kaldırma penceresi hâlâ açıksa bitirin, sonra Enter'a basın...",
            ));
        }
        if is_installed(p) {
            ui::warn(ui::t(
                "still installed (uninstall cancelled or failed), leftovers not touched",
                "hâlâ yüklü (kaldırma iptal edildi ya da başarısız), kalıntılara dokunulmadı",
            ));
            oplog::record("uninstall", &format!("{} NOT removed", p.name));
        } else {
            ui::ok(ui::t("uninstalled", "kaldırıldı"));
            oplog::record("uninstall", &format!("{} {}", p.name, p.version));
            removed.push(p);
        }
    }

    // Leftovers of everything that was really removed: folders and registry keys.
    enum Left {
        Folder(PathBuf, u64),
        Key(String),
    }
    let mut found: Vec<Left> = vec![];
    for p in &removed {
        if p.key.starts_with("appx:") {
            continue; // Windows removes a Store app's data itself
        }
        for d in leftovers(p) {
            let s = fsutil::dir_size(&d);
            found.push(Left::Folder(d, s));
        }
        if let Some(name) = p.key.strip_prefix("scoop:")
            && let Some(persist) = scoop_root()
                .map(|r| r.join("persist").join(name))
                .filter(|d| d.is_dir())
        {
            let s = fsutil::dir_size(&persist);
            found.push(Left::Folder(persist, s));
        }
        for k in registry_leftovers(p) {
            found.push(Left::Key(k));
        }
    }
    if found.is_empty() {
        println!(
            "\n  {}\n",
            ui::t("no leftovers found.", "kalıntı bulunamadı.")
        );
        return Ok(());
    }
    let items = found
        .iter()
        .map(|l| match l {
            Left::Folder(d, s) => ui::CheckItem {
                label: d.display().to_string(),
                detail: ui::t("folder -> Recycle Bin", "klasör -> Geri Dönüşüm Kutusu").into(),
                size: *s,
                checked: true,
            },
            Left::Key(k) => ui::CheckItem {
                label: k.clone(),
                detail: ui::t(
                    "registry key, backed up first",
                    "kayıt defteri anahtarı, önce yedeklenir",
                )
                .into(),
                size: 0,
                checked: true,
            },
        })
        .collect();
    let Some(sel) = ui::checklist(
        ui::t(
            "kazma uninstall :: leftovers",
            "kazma uninstall :: kalıntılar",
        ),
        items,
    )?
    else {
        println!("  {}", ui::t("leftovers kept.", "kalıntılar tutuldu."));
        return Ok(());
    };
    ui::section(ui::t("Leftovers", "Kalıntılar"));
    for (l, on) in found.iter().zip(sel) {
        match (l, on) {
            (Left::Folder(d, _), false) => ui::skip(&tr!("{} (kept)", "{} (tutuldu)", d.display())),
            (Left::Key(k), false) => ui::skip(&tr!("{} (kept)", "{} (tutuldu)", k)),
            (Left::Folder(d, s), true) => match fsutil::trash(d) {
                Ok(()) => {
                    ui::ok(&format!("{:>10}  {}", ui::human(*s), d.display()));
                    oplog::record("uninstall", &format!("trashed leftover {}", d.display()));
                }
                Err(e) => ui::warn(&format!("{}  ({e})", d.display())),
            },
            (Left::Key(k), true) => match remove_registry_key(k) {
                Ok(backup) => {
                    ui::ok(&tr!(
                        "{k}  (backup: {})",
                        "{k}  (yedek: {})",
                        backup.display()
                    ));
                    oplog::record(
                        "uninstall",
                        &format!("removed registry {k} backup={}", backup.display()),
                    );
                }
                Err(e) => ui::warn(&format!("{k}  ({e})")),
            },
        }
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(uninstall: &str, quiet: &str) -> Program {
        Program {
            name: "X".into(),
            version: String::new(),
            publisher: String::new(),
            size_kb: 0,
            location: String::new(),
            uninstall: uninstall.into(),
            quiet: quiet.into(),
            key: "k".into(),
        }
    }

    #[test]
    fn orphan_matching() {
        let installed: Vec<String> = ["Mozilla Firefox", "Notepad++", "Discord Inc"]
            .iter()
            .map(|s| normalize(s))
            .collect();
        assert!(belongs("Notepad++", &installed));
        assert!(belongs("discord", &installed));
        assert!(belongs("Firefox", &installed));
        assert!(!belongs("OldGameStudio", &installed));
        assert!(
            belongs("ab", &installed),
            "very short names are never orphans"
        );
    }

    #[test]
    fn names() {
        assert_eq!(base_name("7-Zip 23.01 (x64)"), "7-Zip");
        assert_eq!(base_name("Mozilla Firefox (x64 en-US)"), "Mozilla Firefox");
        assert_eq!(base_name("Notepad++ v8.6.9"), "Notepad++");
        assert_eq!(base_name("VLC media player"), "VLC media player");
        assert!(generic("Microsoft"));
        assert!(!generic("Notepad++"));
    }

    #[test]
    fn msi_command() {
        let p = prog("MsiExec.exe /I{23170F69-40C1-2702-2301-000001000000}", "");
        assert_eq!(
            command_line(&p, false),
            "msiexec.exe /x {23170F69-40C1-2702-2301-000001000000} /passive /norestart"
        );
        let q = prog(
            "\"C:\\Program Files\\App\\uninst.exe\"",
            "\"C:\\Program Files\\App\\uninst.exe\" /S",
        );
        assert_eq!(
            command_line(&q, false),
            "\"C:\\Program Files\\App\\uninst.exe\""
        );
        assert_eq!(
            command_line(&q, true),
            "\"C:\\Program Files\\App\\uninst.exe\" /S"
        );
    }
}
