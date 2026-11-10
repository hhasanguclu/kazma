//! Protected paths. Anything listed in `%LOCALAPPDATA%\kazma\whitelist.txt` (and
//! everything below it) is never deleted by clean, purge, installer, analyze or
//! uninstall. One path per line, `#` starts a comment, `%VARS%` and `~` are expanded.

use crate::{fsutil, oplog, ui};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::OnceLock;

static ENTRIES: OnceLock<Vec<String>> = OnceLock::new();

pub fn file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("whitelist.txt"))
}

/// Comparable form: unified separators, no trailing separator, case-folded on Windows.
fn norm(p: &Path) -> String {
    let s: String = p.to_string_lossy().chars().map(|c| if c == '/' || c == '\\' { MAIN_SEPARATOR } else { c }).collect();
    let s = s.trim_end_matches(MAIN_SEPARATOR).to_string();
    if cfg!(windows) { s.to_lowercase() } else { s }
}

/// Expand `%NAME%` environment variables and a leading `~`.
pub fn expand(line: &str) -> PathBuf {
    let mut out = String::new();
    let mut rest = line.trim();
    if let Some(r) = rest.strip_prefix('~') {
        if let Some(h) = fsutil::home() {
            out.push_str(&h.to_string_lossy());
        }
        rest = r;
    }
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => out.push_str(&format!("%{name}%")),
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    PathBuf::from(out)
}

fn parse(text: &str) -> Vec<PathBuf> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(expand)
        .collect()
}

/// Read the whitelist file. A missing file means "no entries"; a file that exists
/// but cannot be read is an error, so nothing runs without its protections.
pub fn load() -> anyhow::Result<Vec<PathBuf>> {
    let Some(f) = file() else { return Ok(vec![]) };
    if !f.exists() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&f)
        .map_err(|e| anyhow::anyhow!("cannot read whitelist {}: {e}. Refusing to run without it.", f.display()))?;
    Ok(parse(&text))
}

pub fn init() -> anyhow::Result<()> {
    let entries = load()?.iter().map(|p| norm(p)).filter(|s| !s.is_empty()).collect();
    let _ = ENTRIES.set(entries);
    Ok(())
}

fn entries() -> &'static [String] {
    ENTRIES.get().map(Vec::as_slice).unwrap_or(&[])
}

fn under(child: &str, parent: &str) -> bool {
    child == parent || (child.starts_with(parent) && child[parent.len()..].starts_with(MAIN_SEPARATOR))
}

/// `path` is whitelisted or inside a whitelisted folder.
pub fn is_protected(path: &Path) -> bool {
    let e = entries();
    if e.is_empty() {
        return false;
    }
    let n = norm(path);
    e.iter().any(|w| under(&n, w))
}

/// Deleting `path` as a whole would also delete a whitelisted item below it.
pub fn contains_protected(path: &Path) -> bool {
    let e = entries();
    if e.is_empty() {
        return false;
    }
    let n = norm(path);
    e.iter().any(|w| under(w, &n))
}

const TEMPLATE: &str = "# kazma whitelist: these paths (and everything inside them) are never deleted.
# One path per line. %LOCALAPPDATA%, %APPDATA%, %USERPROFILE% and ~ are expanded.
# Examples:
#   %LOCALAPPDATA%\\Google\\Chrome\\User Data\\Default\\Cache
#   ~\\source\\big-project\\node_modules
#   D:\\Downloads\\keep
";

fn write(entries: &[String]) -> anyhow::Result<PathBuf> {
    let f = file().ok_or_else(|| anyhow::anyhow!("cannot find a folder for the whitelist"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut text = if f.exists() { std::fs::read_to_string(&f)? } else { TEMPLATE.to_string() };
    for e in entries {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(e);
        text.push('\n');
    }
    std::fs::write(&f, text)?;
    Ok(f)
}

pub fn run(add: Vec<String>, remove: Vec<String>, edit: bool) -> anyhow::Result<()> {
    if !add.is_empty() {
        let f = write(&add)?;
        for a in &add {
            ui::ok(&format!("protected: {}", expand(a).display()));
        }
        println!("  file: {}", f.display());
        return Ok(());
    }
    if !remove.is_empty() {
        let f = file().ok_or_else(|| anyhow::anyhow!("no whitelist file"))?;
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let drop: Vec<String> = remove.iter().map(|r| norm(&expand(r))).collect();
        let kept: Vec<&str> = text
            .lines()
            .filter(|l| {
                let body = l.split('#').next().unwrap_or("").trim();
                body.is_empty() || !drop.contains(&norm(&expand(body)))
            })
            .collect();
        std::fs::write(&f, kept.join("\n") + "\n")?;
        ui::ok(&format!("removed {} entr{}", remove.len(), if remove.len() == 1 { "y" } else { "ies" }));
        return Ok(());
    }
    if edit {
        let f = write(&[])?;
        let editor = if cfg!(windows) { "notepad".to_string() } else { std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()) };
        std::process::Command::new(editor).arg(&f).status()?;
        return Ok(());
    }
    ui::print_banner("whitelist :: paths kazma will never delete");
    let list = load()?;
    match file() {
        Some(f) => println!("  file: {}\n", f.display()),
        None => println!("  (no data folder)\n"),
    }
    if list.is_empty() {
        println!("  no protected paths yet.");
        println!("  add one:  kz whitelist --add \"%LOCALAPPDATA%\\Spotify\\Data\"");
        println!("  or edit:  kz whitelist --edit\n");
    } else {
        for p in list {
            let mark = if p.exists() { "[ok]" } else { "[??]" };
            println!("  {mark} {}", p.display());
        }
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_and_parse() {
        // SAFETY: tests in this module are the only users of this variable.
        unsafe { std::env::set_var("KAZMA_TEST_DIR", "/data") };
        let v = parse("# comment\n%KAZMA_TEST_DIR%/keep   # trailing\n\n  /x/y  \n%NOPE_NOT_SET%/z\n");
        assert_eq!(v, vec![PathBuf::from("/data/keep"), PathBuf::from("/x/y"), PathBuf::from("%NOPE_NOT_SET%/z")]);
    }

    #[test]
    fn prefix_matching_respects_boundaries() {
        // Go through norm() so separators match the platform (\ on Windows).
        let n = |s: &str| norm(Path::new(s));
        assert!(under(&n("/a/b/c"), &n("/a/b")));
        assert!(under(&n("/a/b"), &n("/a/b/")));
        assert!(!under(&n("/a/bc"), &n("/a/b")));
        assert!(!under(&n("/a"), &n("/a/b")));
        if cfg!(windows) {
            assert!(under(&n("C:/Users/X/AppData"), &n("c:\\users\\x")));
        }
    }
}
