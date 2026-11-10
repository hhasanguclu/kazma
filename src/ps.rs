//! Helpers for running PowerShell and external tools on Windows.

use std::process::{Command, Output};

/// Run a PowerShell script (stdout forced to UTF-8). `env` values are passed as
/// environment variables so no user data ever needs to be quoted into the script.
pub fn powershell(script: &str, env: &[(&str, &str)]) -> anyhow::Result<Output> {
    // A silenced non-terminating error (-ErrorAction SilentlyContinue) still makes
    // powershell.exe exit with 1 when it was the last command, so finish with an
    // explicit `exit 0`. Real errors are terminating ($ErrorActionPreference =
    // 'Stop') and never reach it, so they still exit with 1.
    let full = format!(
        "$ProgressPreference='SilentlyContinue';$ErrorActionPreference='Stop';\
         [Console]::OutputEncoding=[Text.Encoding]::UTF8;{script}\nexit 0"
    );
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &full,
    ]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    hide_window(&mut cmd);
    Ok(cmd.output()?)
}

/// Run a PowerShell script that prints JSON and parse it. A single object is
/// wrapped into a one-element array so callers always get a list.
pub fn powershell_json<T: serde::de::DeserializeOwned>(
    script: &str,
    env: &[(&str, &str)],
) -> anyhow::Result<Vec<T>> {
    let out = powershell(script, env)?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!(
            "{}",
            err.lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("powershell failed")
                .trim()
        );
    }
    parse_json_list(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_json_list<T: serde::de::DeserializeOwned>(text: &str) -> anyhow::Result<Vec<T>> {
    let t = text.trim().trim_start_matches('\u{feff}');
    if t.is_empty() {
        return Ok(vec![]);
    }
    let v: serde_json::Value = serde_json::from_str(t)?;
    Ok(match v {
        serde_json::Value::Array(_) => serde_json::from_value(v)?,
        other => vec![serde_json::from_value(other)?],
    })
}

#[cfg(windows)]
pub fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
pub fn hide_window(_: &mut Command) {}

/// Simple spinner-less "working..." line that is overwritten when done.
pub fn working(msg: &str) {
    use std::io::Write;
    print!("  .. {msg}\r");
    let _ = std::io::stdout().flush();
}
