//! winapp2.ini support: the community database of cleaning rules for thousands of
//! programs (https://github.com/MoscaDotTo/Winapp2, CC BY-SA 4.0).
//!
//! kazma does not ship the file. `kz winapp2 update` downloads it into the data
//! folder and `kz clean` then lists the entries whose program is on this PC.
//!
//! Only file rules are used (`FileKey`, `ExcludeKey FILE|PATH`); registry rules
//! (`RegKey`) are left alone. On top of the file's own rules kazma refuses a rule
//! that would empty a top-level folder (a drive, Windows, System32, the profile,
//! Documents, AppData...) and never touches whitelisted paths.

use crate::fsutil::{self, Stats, env_path, home};
use crate::{oplog, ps, tr, ui, whitelist};
use std::path::{Path, PathBuf};

/// The non-CCleaner flavor: it also has the entries CCleaner covers natively.
pub const URL: &str =
    "https://raw.githubusercontent.com/MoscaDotTo/Winapp2/master/Non-CCleaner/Winapp2.ini";

/// One `FileKey`: files matching `patterns` in `dir`.
#[derive(Clone, Debug, PartialEq)]
pub struct FileKey {
    pub path: String,
    pub patterns: Vec<String>,
    pub recurse: bool,
    pub remove_self: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Exclude {
    /// FILE: only files directly in `path`; PATH: the whole tree
    pub tree: bool,
    pub path: String,
    pub pattern: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub name: String,
    pub detect: Vec<String>,
    pub detect_file: Vec<String>,
    pub special: Vec<String>,
    pub detect_os: Option<String>,
    pub warning: Option<String>,
    pub files: Vec<FileKey>,
    pub excludes: Vec<Exclude>,
    /// `RegKeyN` values: `HIVE\\key` or `HIVE\\key|value`
    pub reg_keys: Vec<String>,
}

/// A FileKey resolved to real folders on this PC.
#[derive(Clone, Debug)]
pub struct Rule {
    pub dir: PathBuf,
    pub patterns: Vec<String>,
    pub recurse: bool,
    pub remove_self: bool,
}

#[derive(Clone, Debug)]
pub struct Excl {
    pub tree: bool,
    pub dir: String,
    pub pattern: Option<String>,
}

/// An entry ready to measure and clean.
#[derive(Clone, Debug)]
pub struct Found {
    pub name: String,
    pub warning: Option<String>,
    pub rules: Vec<Rule>,
    pub excludes: Vec<Excl>,
    /// some folder is under Windows / Program Files
    pub admin: bool,
    /// registry rules that passed the safety checks and exist here
    pub regs: Vec<String>,
}

// ---- parsing ------------------------------------------------------------------------

/// Key name without its number: `FileKey12` -> `filekey`.
fn key_kind(k: &str) -> String {
    k.trim_end_matches(|c: char| c.is_ascii_digit())
        .to_ascii_lowercase()
}

fn parse_filekey(v: &str) -> Option<FileKey> {
    let mut parts = v.split('|').map(str::trim);
    let path = parts.next().filter(|p| !p.is_empty())?.to_string();
    let mut patterns = vec![];
    let (mut recurse, mut remove_self) = (false, false);
    for p in parts {
        match p.to_ascii_uppercase().as_str() {
            "RECURSE" => recurse = true,
            "REMOVESELF" => remove_self = true,
            "" => {}
            _ => patterns.extend(
                p.split(';')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_lowercase),
            ),
        }
    }
    if patterns.is_empty() {
        patterns.push("*".into());
    }
    Some(FileKey {
        path,
        patterns,
        recurse: recurse || remove_self,
        remove_self,
    })
}

fn parse_exclude(v: &str) -> Option<Exclude> {
    let mut parts = v.splitn(3, '|');
    let tree = match parts.next()?.trim().to_ascii_uppercase().as_str() {
        "FILE" => false,
        "PATH" => true,
        _ => return None, // REG excludes do not matter: registry rules are not used
    };
    let path = parts.next()?.trim().trim_end_matches('\\').to_string();
    let pattern = parts
        .next()
        .map(|p| p.trim().to_lowercase())
        .filter(|p| !p.is_empty() && (tree || p != "*"));
    if path.is_empty() {
        return None;
    }
    Some(Exclude {
        tree,
        path,
        // FILE|dir\| with nothing after it means every file in dir
        pattern: if tree {
            pattern
        } else {
            pattern.or(Some("*".into()))
        },
    })
}

pub fn parse(text: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = vec![];
    let mut cur: Option<Entry> = None;
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            out.extend(cur.take());
            let name = line[1..line.len() - 1].trim();
            let name = name.strip_suffix('*').unwrap_or(name).trim();
            cur = Some(Entry {
                name: name.to_string(),
                ..Default::default()
            });
            continue;
        }
        let (Some(e), Some((k, v))) = (cur.as_mut(), line.split_once('=')) else {
            continue;
        };
        let v = v.trim();
        match key_kind(k.trim()).as_str() {
            "detect" => e.detect.push(v.to_string()),
            "detectfile" => e.detect_file.push(v.to_string()),
            "specialdetect" => e.special.push(v.to_string()),
            "detectos" => e.detect_os = Some(v.to_string()),
            "warning" => e.warning = Some(v.to_string()),
            "filekey" => e.files.extend(parse_filekey(v)),
            "excludekey" => e.excludes.extend(parse_exclude(v)),
            "regkey" => e.reg_keys.push(v.to_string()),
            _ => {}
        }
    }
    out.extend(cur);
    out
}

/// `; Version: 260915` from the header.
pub fn version(text: &str) -> Option<String> {
    text.lines()
        .take(20)
        .find_map(|l| l.trim().strip_prefix("; Version:"))
        .map(|v| v.trim().to_string())
}

// ---- paths --------------------------------------------------------------------------

/// `*` / `?` wildcard match; both sides already lowercase.
pub fn glob(pat: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pat.chars().collect(), name.chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// "*.*" means every file, also names without a dot.
fn file_match(pat: &str, name: &str) -> bool {
    pat == "*.*" || glob(pat, name)
}

/// What a winapp2 variable stands for (Program Files also means its x86 twin).
fn var(name: &str) -> Vec<String> {
    let s = |p: Option<PathBuf>| p.map(|p| p.to_string_lossy().into_owned());
    let h = |sub: &str| s(home().map(|h| h.join(sub)));
    let v: Vec<Option<String>> = match name.to_ascii_lowercase().as_str() {
        "appdata" => vec![s(env_path("APPDATA"))],
        "localappdata" => vec![s(env_path("LOCALAPPDATA"))],
        "locallowappdata" => vec![h("AppData\\LocalLow")],
        "programdata" | "commonappdata" | "allusersprofile" => vec![s(env_path("ProgramData"))],
        "programfiles" => vec![
            s(env_path("ProgramFiles")),
            s(env_path("ProgramFiles(x86)")),
        ],
        "programfiles(x86)" | "programfilesx86" => vec![s(env_path("ProgramFiles(x86)"))],
        "commonprogramfiles" => vec![
            s(env_path("CommonProgramFiles")),
            s(env_path("CommonProgramFiles(x86)")),
        ],
        "userprofile" => vec![s(home())],
        "windir" | "systemroot" => vec![s(env_path("SystemRoot").or_else(|| env_path("windir")))],
        "system" => vec![s(env_path("SystemRoot").map(|w| w.join("System32")))],
        "temp" | "tmp" => vec![s(env_path("TEMP"))],
        "documents" => vec![h("Documents")],
        "desktop" => vec![h("Desktop")],
        "music" => vec![h("Music")],
        "pictures" => vec![h("Pictures")],
        "videos" => vec![h("Videos")],
        "downloads" => vec![h("Downloads")],
        other => vec![std::env::var(other).ok()],
    };
    let mut out: Vec<String> = v.into_iter().flatten().filter(|x| !x.is_empty()).collect();
    out.dedup();
    out
}

/// Replace every `%VAR%`; one result per combination.
fn expand_vars(s: &str) -> Vec<String> {
    let Some(start) = s.find('%') else {
        return vec![s.to_string()];
    };
    let Some(len) = s[start + 1..].find('%') else {
        return vec![s.to_string()];
    };
    let name = &s[start + 1..start + 1 + len];
    let rest = &s[start + 2 + len..];
    var(name)
        .into_iter()
        .flat_map(|v| {
            expand_vars(rest)
                .into_iter()
                .map(move |r| format!("{}{v}{r}", &s[..start]))
        })
        .collect()
}

/// Every existing path a winapp2 path names (variables and `*` segments expanded).
pub fn resolve(spec: &str) -> Vec<PathBuf> {
    let mut out = vec![];
    for full in expand_vars(spec) {
        let mut segs = full.split(['\\', '/']).filter(|s| !s.is_empty());
        let Some(first) = segs.next() else { continue };
        // "C:" -> "C:\", a leading "/" on other systems stays a root
        let root = if first.ends_with(':') {
            PathBuf::from(format!("{first}\\"))
        } else if full.starts_with('/') {
            PathBuf::from(format!("/{first}"))
        } else {
            PathBuf::from(first)
        };
        let mut cur = vec![root];
        for seg in segs {
            let seg_l = seg.to_lowercase();
            cur = if seg.contains(['*', '?']) {
                cur.iter()
                    .flat_map(|d| {
                        std::fs::read_dir(d)
                            .map(|rd| {
                                rd.flatten()
                                    .filter(|e| {
                                        glob(
                                            &seg_l,
                                            &e.file_name().to_string_lossy().to_lowercase(),
                                        )
                                    })
                                    .map(|e| e.path())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            } else {
                cur.iter().map(|d| d.join(seg)).collect()
            };
            if cur.is_empty() {
                break;
            }
        }
        out.extend(cur.into_iter().filter(|p| p.exists()));
    }
    out.sort();
    out.dedup();
    out
}

fn lower(p: &Path) -> String {
    p.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

/// Folders a rule may never empty wholesale: drives, Windows, the profile and its
/// main folders. A rule there may only name specific files, without recursion.
fn top_level(dir: &Path) -> bool {
    if !fsutil::is_safe_target(dir) {
        return true;
    }
    let d = lower(dir);
    let win = env_path("SystemRoot").or_else(|| env_path("windir"));
    let mut tops: Vec<PathBuf> = vec![];
    if let Some(w) = &win {
        tops.extend([w.join("System32"), w.join("SysWOW64")]);
    }
    if let Some(h) = home() {
        for sub in [
            "AppData",
            "AppData\\LocalLow",
            "Documents",
            "Desktop",
            "Downloads",
            "Pictures",
            "Music",
            "Videos",
            "OneDrive",
        ] {
            tops.push(h.join(sub));
        }
    }
    for v in ["CommonProgramFiles", "CommonProgramFiles(x86)"] {
        tops.extend(env_path(v));
    }
    tops.iter().any(|t| lower(t) == d)
}

fn broad(patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|p| p.chars().all(|c| c == '*' || c == '.'))
}

fn under_admin_folder(dir: &Path) -> bool {
    let d = lower(dir);
    ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|v| env_path(v))
        .any(|base| {
            let b = lower(&base);
            d == b || d.starts_with(&format!("{b}\\"))
        })
}

// ---- detection ----------------------------------------------------------------------

#[cfg(windows)]
fn reg_exists(spec: &str) -> bool {
    use windows_sys::Win32::System::Registry::*;
    let (key, value) = match spec.split_once('|') {
        Some((k, v)) => (k, Some(v)),
        None => (spec, None),
    };
    let (hive, path) = key.split_once('\\').unwrap_or((key, ""));
    let root = match hive.to_ascii_uppercase().as_str() {
        "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
        "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
        "HKCR" | "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
        "HKU" | "HKEY_USERS" => HKEY_USERS,
        "HKCC" | "HKEY_CURRENT_CONFIG" => HKEY_CURRENT_CONFIG,
        _ => return false,
    };
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let wpath = wide(path);
    [KEY_WOW64_64KEY, KEY_WOW64_32KEY]
        .iter()
        .any(|view| unsafe {
            let mut h: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(root, wpath.as_ptr(), 0, KEY_READ | view, &mut h) != 0 {
                return false;
            }
            let ok = match value {
                None => true,
                Some(v) => {
                    let wv = wide(v);
                    RegQueryValueExW(
                        h,
                        wv.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    ) == 0
                }
            };
            RegCloseKey(h);
            ok
        })
}

#[cfg(not(windows))]
fn reg_exists(_: &str) -> bool {
    false
}

fn special(code: &str) -> Option<&'static str> {
    Some(match code.trim().to_ascii_uppercase().as_str() {
        "DET_CHROME" => "%LocalAppData%\\Google\\Chrome\\User Data",
        "DET_FIREFOX" => "%AppData%\\Mozilla\\Firefox",
        "DET_EDGE" => "%LocalAppData%\\Microsoft\\Edge\\User Data",
        "DET_OPERA" => "%AppData%\\Opera Software\\Opera Stable",
        "DET_THUNDERBIRD" => "%AppData%\\Thunderbird",
        "DET_WINSTORE" => "%LocalAppData%\\Packages",
        _ => return None,
    })
}

/// Windows 10 and 11 both report 10.0; `DetectOS=min|max`.
fn os_ok(spec: &str) -> bool {
    let num = |s: &str| s.trim().parse::<f64>().ok();
    let (min, max) = spec.split_once('|').unwrap_or((spec, ""));
    num(min).is_none_or(|m| m <= 10.0) && num(max).is_none_or(|m| m >= 10.0)
}

pub fn detected(e: &Entry) -> bool {
    if e.detect_os.as_deref().is_some_and(|d| !os_ok(d)) {
        return false;
    }
    e.detect.iter().any(|d| reg_exists(d))
        || e.detect_file.iter().any(|d| !resolve(d).is_empty())
        || e.special
            .iter()
            .filter_map(|c| special(c))
            .any(|p| !resolve(p).is_empty())
}

/// Resolve an entry's rules on this PC; None when nothing of it exists here.
pub fn find(e: &Entry) -> Option<Found> {
    let mut rules = vec![];
    for fk in &e.files {
        for dir in resolve(&fk.path) {
            if !dir.is_dir() || whitelist::is_protected(&dir) {
                continue;
            }
            if top_level(&dir) && (fk.recurse || broad(&fk.patterns)) {
                continue;
            }
            rules.push(Rule {
                dir,
                patterns: fk.patterns.clone(),
                recurse: fk.recurse,
                remove_self: fk.remove_self,
            });
        }
    }
    let regs: Vec<String> = e
        .reg_keys
        .iter()
        .filter(|r| reg_allowed(r) && reg_exists(r))
        .cloned()
        .collect();
    if rules.is_empty() && regs.is_empty() {
        return None;
    }
    let excludes = e
        .excludes
        .iter()
        .flat_map(|x| {
            let pattern = x.pattern.clone();
            let tree = x.tree;
            resolve(&x.path).into_iter().map(move |d| Excl {
                tree,
                dir: lower(&d),
                pattern: pattern.clone(),
            })
        })
        .collect();
    let admin = rules.iter().any(|r| under_admin_folder(&r.dir))
        || (rules.is_empty() && regs.iter().all(|r| r.to_uppercase().starts_with("HKLM")));
    Some(Found {
        name: e.name.clone(),
        warning: e.warning.clone(),
        rules,
        excludes,
        admin,
        regs,
    })
}

/// A registry rule kazma will carry out: HKCU or HKLM, deep enough to belong
/// to one program, and never in places Windows itself depends on.
pub fn reg_allowed(spec: &str) -> bool {
    let key = spec.split('|').next().unwrap_or("").trim();
    let parts: Vec<String> = key
        .split('\\')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_lowercase())
        .collect();
    let Some(hive) = parts.first() else {
        return false;
    };
    if !matches!(
        hive.as_str(),
        "hkcu" | "hkey_current_user" | "hklm" | "hkey_local_machine"
    ) {
        return false;
    }
    let rest: Vec<&str> = parts[1..].iter().map(String::as_str).collect();
    // only below Software, and not in file associations or policies
    if rest.first() != Some(&"software")
        || matches!(rest.get(1), Some(&"classes") | Some(&"policies"))
    {
        return false;
    }
    // keys Windows needs whole: startup programs, uninstall entries, shell folders
    let joined = rest.join("\\");
    if [
        "\\run",
        "\\runonce",
        "\\uninstall",
        "shell folders",
        "\\app paths",
    ]
    .iter()
    .any(|k| joined.ends_with(k) || joined.contains(&format!("{k}\\")))
    {
        return false;
    }
    // Software\Vendor\Product at least; Microsoft / WOW6432Node keys one deeper
    let deep = match rest.get(1) {
        Some(&"microsoft") | Some(&"wow6432node") => 5,
        _ => 3,
    };
    rest.len() >= deep
}

#[cfg(windows)]
fn reg_delete(spec: &str, backup: &Path) -> bool {
    let (key, value) = match spec.split_once('|') {
        Some((k, v)) => (k.trim(), Some(v.trim())),
        None => (spec.trim(), None),
    };
    // a .reg copy of the whole key first: double-click it to undo
    let saved = std::process::Command::new("reg")
        .args(["export", key, &backup.to_string_lossy(), "/y"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !saved {
        return false;
    }
    let mut cmd = std::process::Command::new("reg");
    cmd.args(["delete", key]);
    if let Some(v) = value {
        cmd.args(["/v", v]);
    }
    cmd.arg("/f");
    cmd.output().is_ok_and(|o| o.status.success())
}

#[cfg(not(windows))]
fn reg_delete(_: &str, _: &Path) -> bool {
    false
}

fn reg_backup_path(entry: &str, n: usize) -> Option<PathBuf> {
    let dir = oplog::data_dir()?.join("backups");
    std::fs::create_dir_all(&dir).ok()?;
    let stamp: String = oplog::now_utc()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let safe: String = entry
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    Some(dir.join(format!("winapp2-{stamp}-{safe}-{n}.reg")))
}

// ---- measuring and deleting -----------------------------------------------------------

fn excluded(path: &Path, is_dir: bool, ex: &[Excl]) -> bool {
    let p = lower(path);
    let (parent, name) = match p.rsplit_once(['\\', '/']) {
        Some((a, b)) => (a, b),
        None => ("", p.as_str()),
    };
    ex.iter().any(|x| {
        let inside =
            p.starts_with(&format!("{}\\", x.dir)) || p.starts_with(&format!("{}/", x.dir));
        match (x.tree, &x.pattern) {
            // a whole folder tree
            (true, None) => inside || p == x.dir,
            (true, Some(pat)) => !is_dir && inside && file_match(pat, name),
            (false, Some(pat)) => !is_dir && parent == x.dir && file_match(pat, name),
            (false, None) => false,
        }
    })
}

fn walk(dir: &Path, r: &Rule, ex: &[Excl], dry: bool) -> Stats {
    let mut st = Stats::default();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return st;
    };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() || whitelist::is_protected(&path) {
            continue; // never follow links, never touch protected paths
        }
        if ft.is_dir() {
            if r.recurse && !excluded(&path, true, ex) {
                let s = walk(&path, r, ex, dry);
                st.bytes += s.bytes;
                st.files += s.files;
                st.failed += s.failed;
                if r.remove_self && !dry {
                    let _ = std::fs::remove_dir(&path); // only when empty
                }
            }
            continue;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        if r.patterns.iter().any(|p| file_match(p, &name)) && !excluded(&path, false, ex) {
            let s = fsutil::delete_file(&path, dry);
            st.bytes += s.bytes;
            st.files += s.files;
            st.failed += s.failed;
        }
    }
    st
}

/// Measure (dry) or delete what an entry names.
pub fn run(f: &Found, dry: bool) -> Stats {
    let mut st = Stats::default();
    for r in &f.rules {
        if !fsutil::is_safe_target(&r.dir) && (r.recurse || broad(&r.patterns)) {
            continue;
        }
        let s = walk(&r.dir, r, &f.excludes, dry);
        st.bytes += s.bytes;
        st.files += s.files;
        st.failed += s.failed;
        if r.remove_self && !dry && fsutil::is_safe_target(&r.dir) && !top_level(&r.dir) {
            let _ = std::fs::remove_dir(&r.dir); // only when empty
        }
    }
    // registry rules count as one item each; HKLM ones only when elevated
    let admin = crate::sys::is_admin();
    for (n, spec) in f.regs.iter().enumerate() {
        if spec.to_uppercase().starts_with("HKLM") && !admin {
            continue;
        }
        if dry {
            st.files += 1;
        } else if let Some(b) = reg_backup_path(&f.name, n) {
            if reg_delete(spec, &b) {
                st.files += 1;
            } else {
                st.failed += 1;
            }
        }
    }
    st
}

// ---- the installed file -----------------------------------------------------------------

pub fn file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("winapp2.ini"))
}

fn selected_file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("winapp2-selected.txt"))
}

/// Entries of the installed winapp2.ini that are on this PC.
pub fn installed() -> Vec<Found> {
    let Some(text) = file().and_then(|f| std::fs::read_to_string(f).ok()) else {
        return vec![];
    };
    use rayon::prelude::*;
    parse(&text)
        .par_iter()
        .filter(|e| detected(e))
        .filter_map(find)
        .collect()
}

/// Entry names picked in an earlier `kz clean`; they start selected (and `--yes` cleans them).
pub fn remembered() -> Vec<String> {
    selected_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn remember(names: &[String]) {
    let Some(f) = selected_file() else { return };
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let mut body = String::from("# winapp2 entries selected in kz clean\n");
    for n in names {
        body.push_str(n);
        body.push('\n');
    }
    let _ = std::fs::write(f, body);
}

const DOWNLOAD: &str = r#"
Invoke-WebRequest -UseBasicParsing -Headers @{ 'User-Agent' = 'kazma' } -Uri $env:KAZMA_URL -OutFile $env:KAZMA_OUT
"#;

/// Check a downloaded / given file and install it.
fn install(text: &str) -> anyhow::Result<(usize, Option<String>)> {
    let entries = parse(text);
    let rules = entries.iter().filter(|e| !e.files.is_empty()).count();
    if rules < 10 {
        anyhow::bail!(
            "{}",
            tr!(
                "this does not look like a winapp2.ini file",
                "bu bir winapp2.ini dosyasına benzemiyor"
            )
        );
    }
    let f = file().ok_or_else(|| anyhow::anyhow!("no data folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = f.with_extension("ini.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &f)?;
    Ok((entries.len(), version(text)))
}

fn status() {
    let Some(f) = file() else { return };
    let Ok(text) = std::fs::read_to_string(&f) else {
        ui::warn(&tr!(
            "winapp2.ini is not installed.",
            "winapp2.ini kurulu değil."
        ));
        println!(
            "\n  {}",
            tr!(
                "kz winapp2 update   downloads it (about 2 MB) from the winapp2 project",
                "kz winapp2 update   winapp2 projesinden indirir (yaklaşık 2 MB)"
            )
        );
        println!(
            "  {}\n",
            tr!(
                "kz winapp2 use FILE uses a winapp2.ini you already have",
                "kz winapp2 use FILE elinizdeki bir winapp2.ini dosyasını kullanır"
            )
        );
        return;
    };
    ps::working(&tr!(
        "looking for the programs on this PC...",
        "bu bilgisayardaki programlar aranıyor..."
    ));
    let all = parse(&text);
    let found = installed();
    println!("{:60}", "");
    let w = 16;
    println!("  {} {}", ui::pad(&tr!("file", "dosya"), w), f.display());
    println!(
        "  {} {}",
        ui::pad(&tr!("version", "sürüm"), w),
        version(&text).unwrap_or_else(|| "?".into())
    );
    println!("  {} {}", ui::pad(&tr!("entries", "kayıt"), w), all.len());
    println!(
        "  {} {}",
        ui::pad(&tr!("on this PC", "bu bilgisayarda"), w),
        found.len()
    );
    println!(
        "  {} {}",
        ui::pad(&tr!("selected", "seçili"), w),
        remembered().len()
    );
    println!(
        "\n  {}",
        tr!(
            "kz clean lists these entries; the ones you pick are remembered.",
            "kz clean bu kayıtları listeler; seçtikleriniz hatırlanır."
        )
    );
    println!(
        "  {}",
        tr!(
            "only file rules are used, registry rules are skipped.",
            "yalnızca dosya kuralları kullanılır, kayıt defteri kuralları atlanır."
        )
    );
    println!(
        "  {}\n",
        tr!(
            "winapp2.ini: github.com/MoscaDotTo/Winapp2 (CC BY-SA 4.0)",
            "winapp2.ini: github.com/MoscaDotTo/Winapp2 (CC BY-SA 4.0)"
        )
    );
}

/// `kz winapp2 [status | list | update [URL] | use FILE | remove]`
pub fn command(action: Option<String>, arg: Option<String>) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "winapp2 :: cleaning rules for thousands of programs",
        "winapp2 :: binlerce program için temizlik kuralları"
    ));
    match action.as_deref() {
        None | Some("status") => status(),
        Some("list") => {
            ps::working(&tr!(
                "looking for the programs on this PC...",
                "bu bilgisayardaki programlar aranıyor..."
            ));
            let mut found = installed();
            println!("{:60}", "");
            if found.is_empty() {
                ui::warn(&tr!(
                    "no entries for this PC (install with: kz winapp2 update)",
                    "bu bilgisayar için kayıt yok (kurmak için: kz winapp2 update)"
                ));
                return Ok(());
            }
            found.sort_by_key(|f| f.name.to_lowercase());
            let picked = remembered();
            for f in &found {
                let line = format!("{}{}", f.name, if f.admin { "  (admin)" } else { "" });
                if picked.contains(&f.name) {
                    ui::ok(&line);
                } else {
                    ui::skip(&line);
                }
            }
            println!();
        }
        Some("update") => {
            let url = arg.unwrap_or_else(|| URL.to_string());
            if !url.starts_with("https://") {
                anyhow::bail!(
                    "{}",
                    tr!("use an https:// address", "https:// adresi kullanın")
                );
            }
            let tmp =
                std::env::temp_dir().join(format!("kazma-winapp2-{}.ini", std::process::id()));
            ps::working(&tr!(
                "downloading winapp2.ini...",
                "winapp2.ini indiriliyor..."
            ));
            let out = ps::powershell(
                DOWNLOAD,
                &[("KAZMA_URL", &url), ("KAZMA_OUT", &tmp.to_string_lossy())],
            )?;
            println!("{:60}", "");
            if !out.status.success() {
                let _ = std::fs::remove_file(&tmp);
                anyhow::bail!(
                    "{}: {}",
                    tr!("download failed", "indirme başarısız"),
                    ps::last_line(&String::from_utf8_lossy(&out.stderr))
                );
            }
            let text = std::fs::read_to_string(&tmp);
            let _ = std::fs::remove_file(&tmp);
            let (n, ver) = install(&text?)?;
            oplog::record(
                "winapp2",
                &format!("update {}", ver.as_deref().unwrap_or("?")),
            );
            ui::ok(&tr!(
                "winapp2.ini {} installed: {} entries",
                "winapp2.ini {} kuruldu: {} kayıt",
                ver.unwrap_or_default(),
                n
            ));
            println!(
                "  {}\n",
                tr!(
                    "kz clean now lists the programs found on this PC.",
                    "kz clean artık bu bilgisayarda bulunan programları listeler."
                )
            );
        }
        Some("use") => {
            let path = arg.ok_or_else(|| anyhow::anyhow!("kz winapp2 use FILE"))?;
            let text = std::fs::read_to_string(whitelist::expand(&path))?;
            let (n, ver) = install(&text)?;
            oplog::record("winapp2", &format!("use {path}"));
            ui::ok(&tr!(
                "winapp2.ini {} installed: {} entries",
                "winapp2.ini {} kuruldu: {} kayıt",
                ver.unwrap_or_default(),
                n
            ));
            println!();
        }
        Some("remove") => {
            for f in [file(), selected_file()].into_iter().flatten() {
                let _ = std::fs::remove_file(f);
            }
            oplog::record("winapp2", "remove");
            ui::ok(&tr!("winapp2.ini removed.", "winapp2.ini kaldırıldı."));
            println!();
        }
        Some(other) => anyhow::bail!(
            "{}",
            tr!(
                "unknown action: {other}. use status | list | update | use FILE | remove",
                "bilinmeyen işlem: {other}. status | list | update | use FILE | remove kullanın"
            )
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; Version: 260915\n; # of entries: 2\n\n\
[Some App Cache *]\nLangSecRef=3021\nDetectFile=%AppData%\\Some App\nDetect1=HKCU\\Software\\Some\n\
FileKey1=%AppData%\\Some App\\Cache|*|REMOVESELF\nFileKey2=%AppData%\\Some App|*.log;*.tmp\n\
FileKey3=%AppData%\\Some App\\Logs|*.txt|RECURSE\n\
ExcludeKey1=FILE|%AppData%\\Some App\\|keep.log\nExcludeKey2=PATH|%AppData%\\Some App\\Cache\\Keep\\\n\
RegKey1=HKCU\\Software\\Some\\MRU\n\n[Other *]\nDetectOS=|6.1\nWarning=Careful\nFileKey1=%Temp%\\x|REMOVESELF\n";

    #[test]
    fn parses() {
        let e = parse(SAMPLE);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].name, "Some App Cache");
        assert_eq!(e[0].detect, vec!["HKCU\\Software\\Some"]);
        assert_eq!(e[0].files.len(), 3);
        assert!(e[0].files[0].remove_self && e[0].files[0].recurse);
        assert_eq!(e[0].files[1].patterns, vec!["*.log", "*.tmp"]);
        assert!(!e[0].files[1].recurse);
        assert_eq!(e[0].excludes.len(), 2);
        assert_eq!(e[0].excludes[0].pattern.as_deref(), Some("keep.log"));
        assert!(e[0].excludes[1].tree && e[0].excludes[1].pattern.is_none());
        assert_eq!(e[0].reg_keys, vec!["HKCU\\Software\\Some\\MRU"]);
        assert_eq!(e[1].files[0].patterns, vec!["*"]);
        assert_eq!(e[1].warning.as_deref(), Some("Careful"));
        assert_eq!(version(SAMPLE).as_deref(), Some("260915"));
        assert!(!os_ok("|6.1"));
        assert!(os_ok("6.2|"));
        assert!(os_ok("10.0|10.0"));
    }

    #[test]
    fn globs() {
        assert!(glob("*.log", "a.log"));
        assert!(!glob("*.log", "a.log1"));
        assert!(glob("*.log*", "a.log1"));
        assert!(glob("*log.txt", "speedtestlog.txt"));
        assert!(glob("chrome*", "chrome sxs"));
        assert!(glob("a?c", "abc"));
        assert!(!glob("a?c", "ac"));
        assert!(glob("*", ""));
        assert!(file_match("*.*", "noext"));
        assert!(broad(&["*".into()]) && broad(&["*.*".into()]) && !broad(&["*.log".into()]));
    }

    #[test]
    fn cleans_by_pattern_with_excludes() {
        let root = std::env::temp_dir().join(format!("kazma-w2-{}", std::process::id()));
        let app = root.join("Some App");
        let cache = app.join("Cache");
        std::fs::create_dir_all(cache.join("Keep")).unwrap();
        std::fs::create_dir_all(cache.join("sub")).unwrap();
        for (p, body) in [
            (app.join("a.log"), "1234"),
            (app.join("keep.log"), "12"),
            (app.join("settings.ini"), "x"),
            (cache.join("c1"), "123"),
            (cache.join("sub").join("c2"), "1"),
            (cache.join("Keep").join("k"), "1"),
        ] {
            std::fs::write(p, body).unwrap();
        }
        let ex = vec![
            Excl {
                tree: false,
                dir: lower(&app),
                pattern: Some("keep.log".into()),
            },
            Excl {
                tree: true,
                dir: lower(&cache.join("Keep")),
                pattern: None,
            },
        ];
        let f = Found {
            name: "t".into(),
            warning: None,
            rules: vec![
                Rule {
                    dir: app.clone(),
                    patterns: vec!["*.log".into()],
                    recurse: false,
                    remove_self: false,
                },
                Rule {
                    dir: cache.clone(),
                    patterns: vec!["*".into()],
                    recurse: true,
                    remove_self: true,
                },
            ],
            excludes: ex,
            admin: false,
            regs: vec![],
        };
        let m = run(&f, true);
        assert_eq!((m.files, m.bytes), (3, 4 + 3 + 1));
        assert!(app.join("a.log").exists(), "dry run deletes nothing");
        let d = run(&f, false);
        assert_eq!(d.files, 3);
        assert!(!app.join("a.log").exists());
        assert!(app.join("keep.log").exists());
        assert!(app.join("settings.ini").exists());
        assert!(!cache.join("sub").exists(), "emptied folder removed");
        assert!(cache.join("Keep").join("k").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn resolves_wildcard_folders() {
        let root = std::env::temp_dir().join(format!("kazma-w2r-{}", std::process::id()));
        std::fs::create_dir_all(root.join("Chrome Beta").join("Cache")).unwrap();
        std::fs::create_dir_all(root.join("Chrome").join("Cache")).unwrap();
        std::fs::create_dir_all(root.join("Other").join("Cache")).unwrap();
        let spec = format!("{}\\Chrome*\\Cache", root.display());
        let got = resolve(&spec);
        assert_eq!(got.len(), 2, "{got:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn registry_rules() {
        assert!(reg_allowed(r"HKCU\Software\7-Zip\FM"));
        assert!(reg_allowed(r"HKCU\Software\Vendor\App|LastPath"));
        assert!(reg_allowed(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\RecentDocs"
        ));
        assert!(!reg_allowed(r"HKCU\Software\Vendor"));
        assert!(!reg_allowed(r"HKCU\Software\Microsoft\Windows"));
        assert!(!reg_allowed(r"HKCU\Software\Classes\.txt\OpenWithList"));
        assert!(!reg_allowed(r"HKLM\SOFTWARE\Policies\Vendor\App"));
        assert!(!reg_allowed(r"HKLM\SYSTEM\CurrentControlSet\Services\X"));
        assert!(!reg_allowed(r"HKCR\Something\Deep\Enough"));
        assert!(!reg_allowed(r"HKU\S-1-5\Software\X\Y"));
        assert!(!reg_allowed(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
        ));
        assert!(!reg_allowed(
            r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\App"
        ));
    }

    #[test]
    fn refuses_top_level_folders() {
        if let Some(h) = home() {
            assert!(top_level(&h));
            assert!(top_level(&h.join("Documents")));
            assert!(!top_level(&h.join("Documents").join("My Games").join("X")));
        }
    }
}
