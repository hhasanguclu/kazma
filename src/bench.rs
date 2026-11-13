//! `kz bench`: quick disk speed test. Sequential read/write with 1 MiB blocks and
//! random 4 KiB read/write at queue depth 1. The OS cache is bypassed
//! (FILE_FLAG_NO_BUFFERING on Windows, O_DIRECT on Linux) so the numbers are the
//! disk's, not RAM's. The test file is always deleted afterwards.

use crate::{oplog, tr, ui};
use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const BLOCK: usize = 1 << 20;
const SMALL: usize = 4096;
const ALIGN: usize = 4096;
const RANDOM_SECS: u64 = 3;

/// A page-aligned buffer, required for unbuffered I/O.
struct AlignedBuf {
    ptr: *mut u8,
    len: usize,
}

impl AlignedBuf {
    fn new(len: usize) -> Self {
        let layout = Layout::from_size_align(len, ALIGN).expect("layout");
        // SAFETY: layout has non-zero size; the pointer is checked below.
        let ptr = unsafe { alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "out of memory");
        let mut b = AlignedBuf { ptr, len };
        // incompressible content so SSD controllers can't cheat
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for chunk in b.as_mut().chunks_mut(8) {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let bytes = x.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        b
    }
    fn as_mut(&mut self) -> &mut [u8] {
        // SAFETY: ptr is valid for len bytes for the lifetime of self.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
    fn as_ref(&self) -> &[u8] {
        // SAFETY: as above.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        // SAFETY: allocated with the same layout in `new`.
        unsafe {
            dealloc(
                self.ptr,
                Layout::from_size_align(self.len, ALIGN).expect("layout"),
            )
        }
    }
}

/// Deletes the test file however the benchmark ends.
struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(windows)]
fn direct_flags(write: bool) -> u32 {
    const NO_BUFFERING: u32 = 0x2000_0000;
    const WRITE_THROUGH: u32 = 0x8000_0000;
    if write {
        NO_BUFFERING | WRITE_THROUGH
    } else {
        NO_BUFFERING
    }
}

#[cfg(all(unix, target_arch = "x86_64"))]
fn direct_flags(_write: bool) -> i32 {
    0o40000 // O_DIRECT
}

#[cfg(all(unix, not(target_arch = "x86_64")))]
fn direct_flags(_write: bool) -> i32 {
    0o200000 // O_DIRECT on aarch64 / arm
}

/// Open for unbuffered I/O; falls back to normal I/O when the filesystem refuses.
fn open(path: &Path, write: bool, create: bool) -> std::io::Result<(File, bool)> {
    let mut o = OpenOptions::new();
    o.read(true).write(write).create(create).truncate(create);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let mut d = o.clone();
        d.custom_flags(direct_flags(write));
        if let Ok(f) = d.open(path) {
            return Ok((f, true));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut d = o.clone();
        d.custom_flags(direct_flags(write));
        if let Ok(f) = d.open(path) {
            return Ok((f, true));
        }
    }
    Ok((o.open(path)?, false))
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Width of the label column: the longest test name in the current language.
fn label_w() -> usize {
    [
        ui::t("sequential read", "sıralı okuma"),
        ui::t("sequential write", "sıralı yazma"),
        ui::t("4K random read", "4K rastgele okuma"),
        ui::t("4K random write", "4K rastgele yazma"),
    ]
    .iter()
    .map(|l| ui::width(l))
    .max()
    .unwrap_or(16)
    .max(16)
}

fn progress(label: &str, done: u64, total: u64, t: Instant) {
    let mbs = done as f64 / 1_048_576.0 / t.elapsed().as_secs_f64().max(0.001);
    print!(
        "\r  {} {} {:>8.1} MB/s ",
        ui::pad(label, label_w()),
        ui::cbar(done as f64 / total.max(1) as f64, 24),
        mbs
    );
    let _ = std::io::stdout().flush();
}

pub struct Result4 {
    pub seq_write: f64,
    pub seq_read: f64,
    pub rnd_read_iops: f64,
    pub rnd_write_iops: f64,
    pub direct: bool,
}

pub fn measure(file: &Path, size_mb: u64, show: bool) -> std::io::Result<Result4> {
    let _guard = TempFile(file.to_path_buf());
    let total = size_mb * BLOCK as u64;
    let mut buf = AlignedBuf::new(BLOCK);

    // sequential write
    let (mut f, direct) = open(file, true, true)?;
    let t = Instant::now();
    let mut done = 0u64;
    while done < total {
        f.write_all(buf.as_ref())?;
        done += BLOCK as u64;
        if show && done.is_multiple_of(32 * BLOCK as u64) {
            progress(ui::t("seq write", "sıralı yazma"), done, total, t);
        }
    }
    f.sync_all()?;
    let seq_write = total as f64 / 1_048_576.0 / t.elapsed().as_secs_f64();
    drop(f);
    if show {
        println!();
    }

    // sequential read
    let (mut f, _) = open(file, false, false)?;
    let t = Instant::now();
    let mut done = 0u64;
    while done < total {
        let n = f.read(buf.as_mut())?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if show && done.is_multiple_of(32 * BLOCK as u64) {
            progress(ui::t("seq read", "sıralı okuma"), done, total, t);
        }
    }
    let seq_read = done as f64 / 1_048_576.0 / t.elapsed().as_secs_f64();
    if show {
        println!();
    }

    // random 4K, queue depth 1
    let blocks = total / SMALL as u64;
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let mut small = AlignedBuf::new(SMALL);
    let deadline = Duration::from_secs(RANDOM_SECS);
    let t = Instant::now();
    let mut ops = 0u64;
    while t.elapsed() < deadline {
        f.seek(SeekFrom::Start(rng.next() % blocks * SMALL as u64))?;
        f.read_exact(small.as_mut())?;
        ops += 1;
    }
    let rnd_read_iops = ops as f64 / t.elapsed().as_secs_f64();
    drop(f);
    if show {
        println!(
            "  {} {:>8.0} IOPS",
            ui::pad(ui::t("4K random read", "4K rastgele okuma"), label_w()),
            rnd_read_iops
        );
    }

    let (mut f, _) = open(file, true, false)?;
    let t = Instant::now();
    let mut ops = 0u64;
    while t.elapsed() < deadline {
        f.seek(SeekFrom::Start(rng.next() % blocks * SMALL as u64))?;
        f.write_all(small.as_ref())?;
        if !direct {
            f.sync_data()?;
        }
        ops += 1;
    }
    let rnd_write_iops = ops as f64 / t.elapsed().as_secs_f64();
    if show {
        println!(
            "  {} {:>8.0} IOPS",
            ui::pad(ui::t("4K random write", "4K rastgele yazma"), label_w()),
            rnd_write_iops
        );
    }
    Ok(Result4 {
        seq_write,
        seq_read,
        rnd_read_iops,
        rnd_write_iops,
        direct,
    })
}

fn free_space(path: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    let p = path.to_string_lossy().to_lowercase();
    disks
        .iter()
        .filter(|d| p.starts_with(&d.mount_point().to_string_lossy().to_lowercase()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

pub fn run(dir: Option<PathBuf>, size_mb: u64) -> anyhow::Result<()> {
    ui::print_banner(ui::t("bench :: disk speed test", "bench :: disk hız testi"));
    let dir = dir.unwrap_or_else(std::env::temp_dir);
    let size_mb = size_mb.clamp(64, 16384);
    if let Some(free) = free_space(&dir)
        && free < size_mb * BLOCK as u64 * 2
    {
        anyhow::bail!(
            "{}",
            tr!(
                "not enough free space on {} for a {size_mb} MB test",
                "{} üzerinde {size_mb} MB test için yeterli boş alan yok",
                dir.display()
            )
        );
    }
    let file = dir.join(format!("kazma-bench-{}.tmp", std::process::id()));
    println!(
        "{}\n",
        tr!(
            "  file: {}  ({size_mb} MB, deleted afterwards)",
            "  dosya: {}  ({size_mb} MB, sonra silinir)",
            file.display()
        )
    );
    let r = measure(&file, size_mb, true)?;
    let to_mbs = |iops: f64| iops * SMALL as f64 / 1_048_576.0;
    let rows = [
        (
            ui::t("sequential read", "sıralı okuma"),
            r.seq_read,
            String::new(),
        ),
        (
            ui::t("sequential write", "sıralı yazma"),
            r.seq_write,
            String::new(),
        ),
        (
            ui::t("4K random read", "4K rastgele okuma"),
            to_mbs(r.rnd_read_iops),
            format!("{:.0} IOPS", r.rnd_read_iops),
        ),
        (
            ui::t("4K random write", "4K rastgele yazma"),
            to_mbs(r.rnd_write_iops),
            format!("{:.0} IOPS", r.rnd_write_iops),
        ),
    ];
    let top = rows.iter().map(|r| r.1).fold(1.0, f64::max);
    ui::section(ui::t("Result", "Sonuç"));
    for (name, mbs, extra) in &rows {
        println!(
            "  {} {:>9.1} MB/s  {}  {extra}",
            ui::pad(name, label_w() + 4),
            mbs,
            ui::cbar(mbs / top, 30)
        );
    }
    if !r.direct {
        ui::warn(ui::t(
            "this filesystem refused unbuffered I/O; the OS cache may inflate the numbers",
            "bu dosya sistemi önbelleksiz G/Ç'yi reddetti; işletim sistemi önbelleği sonuçları şişirebilir",
        ));
    }
    oplog::record(
        "bench",
        &format!(
            "{} read={:.0} write={:.0} MB/s",
            dir.display(),
            r.seq_read,
            r.seq_write
        ),
    );
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_run_cleans_up() {
        let f = std::env::temp_dir().join(format!("kazma-bench-test-{}.tmp", std::process::id()));
        let r = measure(&f, 8, false).unwrap();
        assert!(
            r.seq_write > 0.0
                && r.seq_read > 0.0
                && r.rnd_read_iops > 0.0
                && r.rnd_write_iops > 0.0
        );
        assert!(!f.exists(), "test file must be deleted");
    }
}
