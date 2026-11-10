//! `kz remove`: uninstall kazma itself (exe, PATH entry, log/data folder).

use crate::{oplog, tr, ui};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn ask(question: &str) -> bool {
    print!("  ?? {question} [y/N] ");
    let _ = std::io::stdout().flush();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).is_ok()
        && matches!(s.trim().to_lowercase().as_str(), "y" | "yes" | "e" | "evet")
}

/// Is `dir` in the user's PATH? Compared case-insensitively, ignoring trailing slashes.
fn same_dir(a: &str, b: &Path) -> bool {
    let norm = |s: &str| s.trim().trim_end_matches(['\\', '/']).to_lowercase();
    norm(a) == norm(&b.to_string_lossy())
}

#[cfg(windows)]
fn user_path() -> Option<String> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','User')",
        ])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(windows)]
fn set_user_path(value: &str) -> bool {
    // Pass the value through an env var so no quoting/escaping of paths is needed.
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::SetEnvironmentVariable('Path',$env:KAZMA_NEW_PATH,'User')",
        ])
        .env("KAZMA_NEW_PATH", value)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A running exe cannot delete itself on Windows, so hand the job to a detached cmd
/// that waits for us to exit first.
#[cfg(windows)]
fn schedule_self_delete(exe: &Path, dir: Option<&Path>) -> bool {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut script = format!("ping -n 3 127.0.0.1 >nul & del /f /q \"{}\"", exe.display());
    if let Some(d) = dir {
        script.push_str(&format!(" & rmdir \"{}\"", d.display()));
    }
    std::process::Command::new("cmd")
        .arg("/c")
        .raw_arg(&script)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}

#[cfg(not(windows))]
fn user_path() -> Option<String> {
    None
}
#[cfg(not(windows))]
fn set_user_path(_: &str) -> bool {
    false
}
#[cfg(not(windows))]
fn schedule_self_delete(exe: &Path, _dir: Option<&Path>) -> bool {
    std::fs::remove_file(exe).is_ok()
}

pub fn run(dry: bool, yes: bool) -> anyhow::Result<()> {
    ui::print_banner(if dry {
        ui::t("remove :: dry run", "remove :: deneme")
    } else {
        ui::t(
            "remove :: uninstall kazma from this computer",
            "remove :: kazma'yı bu bilgisayardan kaldır",
        )
    });
    let exe = std::env::current_exe()?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let exe = PathBuf::from(exe.to_string_lossy().trim_start_matches(r"\\?\"));
    let dir = exe.parent().map(Path::to_path_buf);
    // Only remove the folder itself if it is kazma's own install folder.
    let own_dir = dir.clone().filter(|d| {
        let n = d
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        n == "kazma"
    });
    let data = oplog::data_dir().filter(|d| d.exists());
    let path_entry = match (&dir, user_path()) {
        (Some(d), Some(p)) => p.split(';').any(|e| same_dir(e, d)).then(|| (d.clone(), p)),
        _ => None,
    };

    ui::section(ui::t("Will remove", "Kaldırılacaklar"));
    println!(
        "{}",
        tr!("  program     {}", "  program     {}", exe.display())
    );
    if let Some(d) = &own_dir {
        println!(
            "{}",
            tr!("  folder      {}", "  klasör      {}", d.display())
        );
    }
    if let Some((d, _)) = &path_entry {
        println!(
            "{}",
            tr!("  PATH entry  {}", "  PATH kaydı  {}", d.display())
        );
    }
    if let Some(d) = &data {
        println!(
            "{}",
            tr!("  data & log  {}", "  veri ve log {}", d.display())
        );
    }
    println!();

    if dry {
        ui::ok(ui::t(
            "dry run, nothing removed.",
            "deneme, hiçbir şey kaldırılmadı.",
        ));
        return Ok(());
    }
    if !yes {
        if !ui::interactive() {
            ui::warn(ui::t(
                "not an interactive terminal: re-run with --yes to uninstall",
                "etkileşimli terminal değil: kaldırmak için --yes ile çalıştırın",
            ));
            return Ok(());
        }
        if !ask(ui::t("Uninstall kazma?", "kazma kaldırılsın mı?")) {
            println!("  {}", ui::t("cancelled.", "iptal edildi."));
            return Ok(());
        }
    }

    if let Some((d, p)) = path_entry {
        let kept: Vec<&str> = p
            .split(';')
            .filter(|e| !e.is_empty() && !same_dir(e, &d))
            .collect();
        if set_user_path(&kept.join(";")) {
            ui::ok(ui::t("PATH entry removed", "PATH kaydı kaldırıldı"))
        } else {
            ui::warn(ui::t("could not update PATH", "PATH güncellenemedi"))
        }
    }
    if let Some(d) = data {
        if std::fs::remove_dir_all(&d).is_ok() {
            ui::ok(ui::t("data & log removed", "veri ve log kaldırıldı"))
        } else {
            ui::warn(ui::t(
                "could not remove data folder",
                "veri klasörü kaldırılamadı",
            ))
        }
    }
    if schedule_self_delete(&exe, own_dir.as_deref()) {
        ui::ok(ui::t(
            "program removed (finishes when this window returns)",
            "program kaldırıldı (bu pencere dönünce tamamlanır)",
        ));
    } else {
        ui::warn(&format!("delete {} manually", exe.display()));
    }
    println!(
        "\n  {}\n",
        ui::t(
            "bye. thanks for using kazma.",
            "hoşça kal. kazma'yı kullandığın için teşekkürler."
        )
    );
    Ok(())
}
