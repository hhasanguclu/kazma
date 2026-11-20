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

/// One log line as JSON: {"time": "...", "command": "...", "message": "..."}.
pub fn parse_line(l: &str) -> Option<serde_json::Value> {
    let (time, rest) = l.split_once(" UTC  ")?;
    let rest = rest.trim_start();
    let (command, message) = rest.split_once(' ').unwrap_or((rest, ""));
    Some(serde_json::json!({
        "time": format!("{time} UTC"),
        "command": command,
        "message": message.trim(),
    }))
}

/// `kz history --json`: the last `limit` entries as a JSON array.
pub fn history_json(limit: usize) {
    let lines: Vec<serde_json::Value> = log_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().filter_map(parse_line).collect())
        .unwrap_or_default();
    let from = lines.len().saturating_sub(limit);
    println!(
        "{}",
        serde_json::to_string_pretty(&lines[from..]).unwrap_or_else(|_| "[]".into())
    );
}

/// Days since the last `kz clean` that deleted something, from the log.
pub fn days_since_clean() -> Option<i64> {
    let text = std::fs::read_to_string(log_path()?).ok()?;
    let last = text.lines().rev().find(|l| {
        l.split_once(" UTC  ")
            .is_some_and(|(_, r)| r.starts_with("clean "))
    })?;
    let then = day_number(&last[..10])?;
    let now = day_number(&now_utc()[..10])?;
    Some(now - then)
}

/// Days since 1970-01-01 for "YYYY-MM-DD".
fn day_number(date: &str) -> Option<i64> {
    let mut p = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, d) = (p.next()??, p.next()??, p.next()??);
    // days from civil, Howard Hinnant's algorithm
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(
            day_number("2026-11-20").unwrap() - day_number("2026-11-13").unwrap(),
            7
        );
        assert_eq!(
            day_number("2024-03-01").unwrap() - day_number("2024-02-28").unwrap(),
            2
        );
    }

    #[test]
    fn parses_lines() {
        let v = parse_line("2026-11-19 15:01:44 UTC  tweak    on menu-delay").unwrap();
        assert_eq!(v["command"], "tweak");
        assert_eq!(v["message"], "on menu-delay");
        assert_eq!(v["time"], "2026-11-19 15:01:44 UTC");
        assert!(parse_line("garbage").is_none());
    }
}
