//! Just enough SQLite to edit browser cookie databases, through `winsqlite3.dll`,
//! the copy of SQLite that ships with Windows 10 and 11. It is loaded at run
//! time, so kazma needs no C compiler and no extra DLL.

#[cfg(windows)]
mod imp {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};
    use std::path::Path;
    use std::sync::OnceLock;

    type Open =
        unsafe extern "system" fn(*const c_char, *mut *mut c_void, c_int, *const c_char) -> c_int;
    type Close = unsafe extern "system" fn(*mut c_void) -> c_int;
    type Prepare = unsafe extern "system" fn(
        *mut c_void,
        *const c_char,
        c_int,
        *mut *mut c_void,
        *mut *const c_char,
    ) -> c_int;
    type BindText =
        unsafe extern "system" fn(*mut c_void, c_int, *const c_char, c_int, isize) -> c_int;
    type Stmt = unsafe extern "system" fn(*mut c_void) -> c_int;
    type ColText = unsafe extern "system" fn(*mut c_void, c_int) -> *const u8;
    type ColInt = unsafe extern "system" fn(*mut c_void, c_int) -> i64;
    type ErrMsg = unsafe extern "system" fn(*mut c_void) -> *const c_char;
    type Busy = unsafe extern "system" fn(*mut c_void, c_int) -> c_int;

    struct Api {
        open: Open,
        close: Close,
        prepare: Prepare,
        bind_text: BindText,
        step: Stmt,
        reset: Stmt,
        finalize: Stmt,
        col_text: ColText,
        col_int: ColInt,
        changes: Stmt,
        errmsg: ErrMsg,
        busy: Busy,
    }

    const ROW: c_int = 100;
    const DONE: c_int = 101;
    const OPEN_READONLY: c_int = 0x1;
    const OPEN_READWRITE: c_int = 0x2;
    /// SQLITE_TRANSIENT: SQLite copies the bound text
    const TRANSIENT: isize = -1;

    fn api() -> Option<&'static Api> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(|| unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
            let name: Vec<u16> = "winsqlite3.dll".encode_utf16().chain([0]).collect();
            let lib = LoadLibraryW(name.as_ptr());
            if lib.is_null() {
                return None;
            }
            macro_rules! f {
                ($t:ty, $n:literal) => {
                    std::mem::transmute::<unsafe extern "system" fn() -> isize, $t>(GetProcAddress(
                        lib,
                        concat!($n, "\0").as_ptr(),
                    )?)
                };
            }
            Some(Api {
                open: f!(Open, "sqlite3_open_v2"),
                close: f!(Close, "sqlite3_close_v2"),
                prepare: f!(Prepare, "sqlite3_prepare_v2"),
                bind_text: f!(BindText, "sqlite3_bind_text"),
                step: f!(Stmt, "sqlite3_step"),
                reset: f!(Stmt, "sqlite3_reset"),
                finalize: f!(Stmt, "sqlite3_finalize"),
                col_text: f!(ColText, "sqlite3_column_text"),
                col_int: f!(ColInt, "sqlite3_column_int64"),
                changes: f!(Stmt, "sqlite3_changes"),
                errmsg: f!(ErrMsg, "sqlite3_errmsg"),
                busy: f!(Busy, "sqlite3_busy_timeout"),
            })
        })
        .as_ref()
    }

    pub struct Db {
        h: *mut c_void,
        a: &'static Api,
    }

    impl Drop for Db {
        fn drop(&mut self) {
            unsafe { (self.a.close)(self.h) };
        }
    }

    impl Db {
        pub fn open(path: &Path, write: bool) -> anyhow::Result<Db> {
            let a = api().ok_or_else(|| anyhow::anyhow!("winsqlite3.dll is not available"))?;
            let p = CString::new(path.to_string_lossy().as_bytes())?;
            let mut h = std::ptr::null_mut();
            let flags = if write { OPEN_READWRITE } else { OPEN_READONLY };
            let rc = unsafe { (a.open)(p.as_ptr(), &mut h, flags, std::ptr::null()) };
            let db = Db { h, a };
            if rc != 0 {
                anyhow::bail!("{}", db.err());
            }
            unsafe { (a.busy)(h, 2000) };
            Ok(db)
        }

        fn err(&self) -> String {
            if self.h.is_null() {
                return "cannot open database".into();
            }
            unsafe { CStr::from_ptr((self.a.errmsg)(self.h)) }
                .to_string_lossy()
                .into_owned()
        }

        fn prepare(&self, sql: &str) -> anyhow::Result<*mut c_void> {
            let s = CString::new(sql)?;
            let mut st = std::ptr::null_mut();
            let rc =
                unsafe { (self.a.prepare)(self.h, s.as_ptr(), -1, &mut st, std::ptr::null_mut()) };
            if rc != 0 {
                anyhow::bail!("{}", self.err());
            }
            Ok(st)
        }

        /// Run a statement that returns no rows.
        pub fn exec(&self, sql: &str) -> anyhow::Result<()> {
            let st = self.prepare(sql)?;
            let rc = unsafe { (self.a.step)(st) };
            unsafe { (self.a.finalize)(st) };
            if rc != DONE && rc != ROW {
                anyhow::bail!("{}", self.err());
            }
            Ok(())
        }

        /// Rows of (text, integer) from a two-column query.
        pub fn pairs(&self, sql: &str) -> anyhow::Result<Vec<(String, i64)>> {
            let st = self.prepare(sql)?;
            let mut out = vec![];
            loop {
                match unsafe { (self.a.step)(st) } {
                    ROW => {
                        let t = unsafe { (self.a.col_text)(st, 0) };
                        let text = if t.is_null() {
                            String::new()
                        } else {
                            unsafe { CStr::from_ptr(t as *const c_char) }
                                .to_string_lossy()
                                .into_owned()
                        };
                        out.push((text, unsafe { (self.a.col_int)(st, 1) }));
                    }
                    DONE => break,
                    _ => {
                        unsafe { (self.a.finalize)(st) };
                        anyhow::bail!("{}", self.err());
                    }
                }
            }
            unsafe { (self.a.finalize)(st) };
            Ok(out)
        }

        /// Run `sql` (one `?` text parameter) once per value; returns rows changed.
        pub fn each(&self, sql: &str, values: &[String]) -> anyhow::Result<u64> {
            let st = self.prepare(sql)?;
            let mut n = 0u64;
            for v in values {
                let c = CString::new(v.as_bytes())?;
                unsafe {
                    (self.a.bind_text)(st, 1, c.as_ptr(), -1, TRANSIENT);
                    let rc = (self.a.step)(st);
                    (self.a.reset)(st);
                    if rc != DONE {
                        (self.a.finalize)(st);
                        anyhow::bail!("{}", self.err());
                    }
                    n += (self.a.changes)(self.h) as u64;
                }
            }
            unsafe { (self.a.finalize)(st) };
            Ok(n)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;

    pub struct Db;

    impl Db {
        pub fn open(_: &Path, _: bool) -> anyhow::Result<Db> {
            anyhow::bail!("cookie databases are only read on Windows")
        }
        pub fn exec(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        pub fn pairs(&self, _: &str) -> anyhow::Result<Vec<(String, i64)>> {
            Ok(vec![])
        }
        pub fn each(&self, _: &str, _: &[String]) -> anyhow::Result<u64> {
            Ok(0)
        }
    }
}

pub use imp::Db;

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = std::env::temp_dir().join(format!("kazma-sqlite-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        std::fs::write(&p, b"").unwrap();
        {
            let db = Db::open(&p, true).unwrap();
            db.exec("CREATE TABLE cookies (host_key TEXT, name TEXT)")
                .unwrap();
            db.exec("INSERT INTO cookies VALUES ('.a.com','x'),('.a.com','y'),('b.com','z')")
                .unwrap();
            let hosts = db
                .pairs("SELECT host_key, COUNT(*) FROM cookies GROUP BY host_key ORDER BY host_key")
                .unwrap();
            assert_eq!(hosts, vec![(".a.com".into(), 2), ("b.com".into(), 1)]);
            let n = db
                .each("DELETE FROM cookies WHERE host_key = ?", &[".a.com".into()])
                .unwrap();
            assert_eq!(n, 2);
        }
        let db = Db::open(&p, false).unwrap();
        let left = db
            .pairs("SELECT host_key, COUNT(*) FROM cookies GROUP BY host_key")
            .unwrap();
        assert_eq!(left, vec![("b.com".into(), 1)]);
        drop(db);
        let _ = std::fs::remove_file(&p);
    }
}
