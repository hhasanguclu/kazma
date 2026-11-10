//! Thin platform layer. Windows APIs live here; other platforms get harmless fallbacks
//! so the project can still be built and tested on Linux/macOS.

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::UI::Shell::{
        IsUserAnAdmin, SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI,
        SHERB_NOSOUND, SHQUERYRBINFO,
    };

    pub fn is_admin() -> bool {
        unsafe { IsUserAnAdmin() != 0 }
    }

    pub fn recycle_bin_size() -> Option<(u64, u64)> {
        let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
        let hr = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) };
        (hr >= 0).then(|| (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
    }

    pub fn empty_recycle_bin() -> bool {
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), flags) };
        // E_UNEXPECTED is returned when the bin is already empty.
        hr >= 0 || hr == 0x8000_FFFFu32 as i32
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn is_admin() -> bool {
        std::env::var("USER").map(|u| u == "root").unwrap_or(false)
    }
    pub fn recycle_bin_size() -> Option<(u64, u64)> {
        None
    }
    pub fn empty_recycle_bin() -> bool {
        false
    }
}

pub use imp::*;

/// Free space on the system drive (C:\ on Windows, / elsewhere).
pub fn system_drive_free() -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    let want = std::env::var("SystemDrive").map(|d| format!("{d}\\")).unwrap_or_else(|_| "/".into());
    disks
        .iter()
        .find(|d| d.mount_point().to_string_lossy().eq_ignore_ascii_case(&want))
        .map(|d| d.available_space())
}

/// Reveal a path in Explorer (or the platform file manager).
pub fn reveal(path: &std::path::Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(path)).spawn();
}
