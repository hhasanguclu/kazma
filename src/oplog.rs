//! Append-only operation log, readable with `kz history`.

use crate::fsutil::{env_path, home};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn data_dir() -> Option<PathBuf> {
    env_path("LOCALAPPDATA")
        .map(|p| p.join("kazma"))
        .or_else(|| home().map(|h| h.join(".local").join("share").join("kazma")))
}

pub fn log_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("operations.log"))
}

/// UTC timestamp "YYYY-MM-DD HH:MM:SS" without pulling in a date crate.
pub fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

pub fn record(command: &str, message: &str) {
    if std::env::var_os("KAZMA_NO_OPLOG").is_some() {
        return;
    }
    let Some(path) = log_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{} UTC  {command:<8} {message}", now_utc());
    }
}

pub fn show_history(limit: usize) {
    crate::ui::print_banner(&crate::tr!(
        "history :: what kazma did",
        "history :: kazma'nın yaptıkları"
    ));
    let Some(path) = log_path() else { return };
    match std::fs::read_to_string(&path) {
        Ok(s) => {
            let lines: Vec<&str> = s.lines().collect();
            for l in &lines[lines.len().saturating_sub(limit)..] {
                println!("  {l}");
            }
            println!("\n  {}\n", path.display());
        }
        Err(_) => println!(
            "  {}\n  {}\n",
            crate::tr!("No history yet.", "Henüz geçmiş yok."),
            path.display()
        ),
    }
}
