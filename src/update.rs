//! `kz update`: update kazma itself from the latest GitHub release.
//! The new exe is checked against the release's SHA256SUMS.txt before it replaces
//! the running one (a running exe can be renamed on Windows, not overwritten).

use crate::{oplog, ps, tr, ui};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const REPO: &str = "hhasanguclu/kazma";

const LATEST: &str = r#"
$r = Invoke-RestMethod -UseBasicParsing -Headers @{ 'User-Agent' = 'kazma-updater' } -Uri "https://api.github.com/repos/$($env:KAZMA_REPO)/releases/latest"
$exe = $r.assets | Where-Object { $_.name -eq 'kz.exe' } | Select-Object -First 1
$sums = $r.assets | Where-Object { $_.name -eq 'SHA256SUMS.txt' } | Select-Object -First 1
[pscustomobject]@{ Tag = $r.tag_name; Url = [string]$exe.browser_download_url; Sums = [string]$sums.browser_download_url; Page = $r.html_url } | ConvertTo-Json -Compress
"#;

const DOWNLOAD: &str = r#"
Invoke-WebRequest -UseBasicParsing -Headers @{ 'User-Agent' = 'kazma-updater' } -Uri $env:KAZMA_URL -OutFile $env:KAZMA_OUT
$want = ((Invoke-WebRequest -UseBasicParsing -Headers @{ 'User-Agent' = 'kazma-updater' } -Uri $env:KAZMA_SUMS).Content -split "`n" |
  Where-Object { $_ -match '\skz\.exe\s*$' } | Select-Object -First 1) -split '\s+' | Select-Object -First 1
$got = (Get-FileHash -Algorithm SHA256 -Path $env:KAZMA_OUT).Hash
if (-not $want -or $got.ToLower() -ne $want.Trim().ToLower()) {
  Remove-Item $env:KAZMA_OUT -Force -ErrorAction SilentlyContinue
  throw "checksum mismatch"
}
Unblock-File $env:KAZMA_OUT -ErrorAction SilentlyContinue
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Latest {
    tag: String,
    url: String,
    sums: String,
    page: String,
}

/// "v0.10.2" -> (0, 10, 2)
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches(['v', 'V']).split('.').map(|p| {
        p.split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("")
            .parse::<u64>()
    });
    Some((
        it.next()?.ok()?,
        it.next().and_then(Result::ok).unwrap_or(0),
        it.next().and_then(Result::ok).unwrap_or(0),
    ))
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    match (parse_version(remote), parse_version(local)) {
        (Some(r), Some(l)) => r > l,
        _ => false,
    }
}

fn old_path(exe: &Path) -> PathBuf {
    exe.with_extension("exe.old")
}

/// Remove the previous exe left behind by an update (called at every start).
pub fn cleanup_old() {
    if let Ok(exe) = std::env::current_exe() {
        let old = old_path(&exe);
        if old.exists() {
            let _ = std::fs::remove_file(old);
        }
    }
}

pub fn run(check_only: bool) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "update :: update kazma itself",
        "update :: kazma'yı güncelle"
    ));
    let current = env!("CARGO_PKG_VERSION");
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz update works on Windows only. Current version: {current}",
            "kz update yalnızca Windows'ta çalışır. Sürüm: {current}"
        ));
        return Ok(());
    }
    ps::working(&tr!(
        "checking GitHub for the latest release...",
        "GitHub'da son sürüm kontrol ediliyor..."
    ));
    let latest: Vec<Latest> = ps::powershell_json(LATEST, &[("KAZMA_REPO", REPO)])?;
    println!("{:60}", "");
    let Some(l) = latest.into_iter().next() else {
        anyhow::bail!("{}", tr!("no release found", "release bulunamadı"));
    };
    println!("  {:<12} v{current}", tr!("installed", "yüklü"));
    println!("  {:<12} {}\n", tr!("latest", "en son"), l.tag);
    if !is_newer(&l.tag, current) {
        ui::ok(&tr!("kazma is up to date.", "kazma güncel."));
        return Ok(());
    }
    if check_only {
        ui::warn(&tr!(
            "a new version is available: kz update",
            "yeni sürüm var: kz update"
        ));
        println!("  {}\n", l.page);
        return Ok(());
    }
    if l.url.is_empty() || l.sums.is_empty() {
        anyhow::bail!(
            "{}",
            tr!(
                "release has no kz.exe / SHA256SUMS.txt",
                "release'te kz.exe / SHA256SUMS.txt yok"
            )
        );
    }

    let exe = std::env::current_exe()?;
    let new = exe.with_extension("exe.new");
    ps::working(&tr!(
        "downloading {} and verifying checksum...",
        "{} indiriliyor, sağlama toplamı doğrulanıyor...",
        l.tag
    ));
    let out = ps::powershell(
        DOWNLOAD,
        &[
            ("KAZMA_URL", &l.url),
            ("KAZMA_SUMS", &l.sums),
            ("KAZMA_OUT", &new.to_string_lossy()),
        ],
    )?;
    println!("{:70}", "");
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!(
            "{}: {}",
            tr!("download failed", "indirme başarısız"),
            err.lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .trim()
        );
    }
    ui::ok(&tr!(
        "downloaded and verified (SHA-256)",
        "indirildi ve doğrulandı (SHA-256)"
    ));

    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old)?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe); // roll back
        anyhow::bail!(
            "{}: {e}",
            tr!("could not replace kz.exe", "kz.exe değiştirilemedi")
        );
    }
    oplog::record("update", &format!("v{current} -> {}", l.tag));
    ui::ok(&tr!(
        "updated to {}. Restart kz to use it.",
        "{} sürümüne güncellendi. Kullanmak için kz'yi yeniden başlatın.",
        l.tag
    ));
    println!();
    Ok(())
}

/// Newest release tag, checked at most once a day and cached in the data folder.
/// Returns None when offline, on non-Windows, or when KAZMA_NO_UPDATE_CHECK is set.
pub fn latest_cached() -> Option<String> {
    if std::env::var_os("KAZMA_NO_UPDATE_CHECK").is_some() || !cfg!(windows) {
        return None;
    }
    let today = oplog::now_utc()[..10].to_string();
    let cache = oplog::data_dir()?.join("update-check.txt");
    if let Ok(s) = std::fs::read_to_string(&cache)
        && let Some((day, tag)) = s.trim().split_once(' ')
        && day == today
    {
        return Some(tag.to_string());
    }
    let latest: Vec<Latest> = ps::powershell_json(LATEST, &[("KAZMA_REPO", REPO)]).ok()?;
    let tag = latest.into_iter().next()?.tag;
    if let Some(d) = cache.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&cache, format!("{today} {tag}"));
    Some(tag)
}

/// Check in the background; the receiver gets a tag only if it is newer.
pub fn spawn_check() -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Some(tag) = latest_cached()
            && is_newer(&tag, env!("CARGO_PKG_VERSION"))
        {
            let _ = tx.send(tag);
        }
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.10.2"), Some((0, 10, 2)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert!(is_newer("v0.3.0", "0.2.9"));
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("garbage", "0.2.0"));
    }
}
