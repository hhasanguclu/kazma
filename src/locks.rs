//! "Which program has this file open?" via the Windows Restart Manager, the same
//! API Windows uses for its own "file is in use" dialogs. Works without admin
//! rights for the user's own processes.

use crate::{tr, ui};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Locker {
    pub pid: u32,
    pub name: String,
    /// Service short name, when the locker is a Windows service.
    pub service: String,
}

/// Files to check for `path`: the file itself, or up to `limit` files inside a folder.
fn files_of(path: &Path, limit: usize) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut out = vec![];
    let mut stack = vec![path.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() && !ft.is_symlink() {
                stack.push(e.path());
            } else if ft.is_file() {
                out.push(e.path());
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(windows)]
pub fn lockers(path: &Path) -> Vec<Locker> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::RestartManager::{
        CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
        RmStartSession,
    };
    let files = files_of(path, 1000);
    if files.is_empty() {
        return vec![];
    }
    let wide: Vec<Vec<u16>> = files
        .iter()
        .map(|f| f.as_os_str().encode_wide().chain([0]).collect())
        .collect();
    let ptrs: Vec<*const u16> = wide.iter().map(|w| w.as_ptr()).collect();
    let utf16 = |s: &[u16]| {
        String::from_utf16_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())])
    };
    let mut out: Vec<Locker> = vec![];
    // SAFETY: plain Win32 calls; every buffer outlives the session, which is always ended.
    unsafe {
        let mut session: u32 = 0;
        let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
        if RmStartSession(&mut session, 0, key.as_mut_ptr()) != 0 {
            return vec![];
        }
        if RmRegisterResources(
            session,
            ptrs.len() as u32,
            ptrs.as_ptr(),
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
        ) == 0
        {
            let mut needed: u32 = 0;
            let mut count: u32 = 0;
            let mut reasons: u32 = 0;
            // first call asks for the size (ERROR_MORE_DATA = 234), second fetches
            let _ = RmGetList(
                session,
                &mut needed,
                &mut count,
                std::ptr::null_mut(),
                &mut reasons,
            );
            if needed > 0 {
                let mut info: Vec<RM_PROCESS_INFO> = vec![std::mem::zeroed(); needed as usize];
                count = needed;
                if RmGetList(
                    session,
                    &mut needed,
                    &mut count,
                    info.as_mut_ptr(),
                    &mut reasons,
                ) == 0
                {
                    for i in &info[..count as usize] {
                        out.push(Locker {
                            pid: i.Process.dwProcessId,
                            name: utf16(&i.strAppName),
                            service: utf16(&i.strServiceShortName),
                        });
                    }
                }
            }
        }
        RmEndSession(session);
    }
    out.sort_by_key(|l| l.pid);
    out.dedup_by_key(|l| l.pid);
    out
}

#[cfg(not(windows))]
pub fn lockers(path: &Path) -> Vec<Locker> {
    let _ = files_of(path, 0);
    vec![]
}

/// One line for the UI: "in use by chrome (1234), OneDrive (88)" or "not in use".
pub fn summary(list: &[Locker]) -> String {
    if list.is_empty() {
        return ui::t("not in use by any program", "hiçbir program kullanmıyor").into();
    }
    let names: Vec<String> = list
        .iter()
        .take(4)
        .map(|l| {
            if l.service.is_empty() {
                format!("{} ({})", l.name, l.pid)
            } else {
                format!("{} [{}] ({})", l.name, l.service, l.pid)
            }
        })
        .collect();
    let more = if list.len() > 4 {
        tr!(" and {} more", " ve {} tane daha", list.len() - 4)
    } else {
        String::new()
    };
    tr!("in use by {}{more}", "kullanan: {}{more}", names.join(", "))
}

/// `kz lock <path>`
pub fn run(path: PathBuf) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "lock :: which program has this open",
        "lock :: bunu hangi program açık tutuyor",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz lock works on Windows only.",
            "kz lock yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    if !path.exists() {
        anyhow::bail!("{}", tr!("not found: {}", "bulunamadı: {}", path.display()));
    }
    let list = lockers(&path);
    println!("{}\n", tr!("  {}", "  {}", path.display()));
    if list.is_empty() {
        ui::ok(&summary(&list));
    } else {
        ui::section(ui::t("Programs using it", "Kullanan programlar"));
        for l in &list {
            let svc = if l.service.is_empty() {
                String::new()
            } else {
                tr!("  service: {}", "  servis: {}", l.service)
            };
            ui::warn(&format!("{:>7}  {}{svc}", l.pid, l.name));
        }
        println!(
            "\n  {}\n",
            ui::t(
                "close them, or end them in kz status (x).",
                "bunları kapatın ya da kz status içinde (x) sonlandırın."
            )
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_text() {
        let l = |pid, n: &str| Locker {
            pid,
            name: n.into(),
            service: String::new(),
        };
        assert!(summary(&[]).contains("not") || summary(&[]).contains("hiçbir"));
        let s = summary(&[l(1, "a"), l(2, "b"), l(3, "c"), l(4, "d"), l(5, "e")]);
        assert!(s.contains("a (1)") && (s.contains("1 more") || s.contains("1 tane")));
    }

    #[test]
    fn collects_files_from_folders() {
        let base = std::env::temp_dir().join(format!("kazma-locks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("a/b")).unwrap();
        for f in ["x", "a/y", "a/b/z"] {
            std::fs::write(base.join(f), b"1").unwrap();
        }
        assert_eq!(files_of(&base, 10).len(), 3);
        assert_eq!(files_of(&base, 2).len(), 2);
        assert_eq!(files_of(&base.join("x"), 10).len(), 1);
        std::fs::remove_dir_all(&base).unwrap();
    }
}
