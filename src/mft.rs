//! Raw NTFS Master File Table reader: the trick that makes WizTree fast.
//!
//! Instead of asking Windows for every folder one by one, read the volume's
//! `$MFT` sequentially in large chunks and rebuild the tree in memory: every
//! file record carries its name, parent folder and size. A whole drive with a
//! million files takes seconds.
//!
//! Needs administrator rights (raw volume access) and an NTFS volume. Every
//! problem returns an error so callers can fall back to the normal walk.
//! Sizes are logical file sizes; hard links are counted once; alternate data
//! streams are ignored (like Explorer).

#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const ROOT: u32 = 5;
/// `$Extend` (quota, object ids, USN journal...): NTFS metadata, not user data.
const EXTEND: u32 = 11;
/// Records below this are reserved NTFS metafiles ($MFT, $LogFile, $Bitmap...).
const FIRST_USER: u32 = 24;
const CHUNK: u64 = 4 << 20;

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}
fn u64_at(b: &[u8], o: usize) -> Option<u64> {
    b.get(o..o + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}

/// One MFT record, reduced to what a disk-usage tool needs.
#[derive(Clone, Default)]
struct Rec {
    parent: u32,
    size: u64,
    name: Option<Box<str>>,
    /// Namespace of the chosen name (2 = DOS 8.3 short name, least preferred).
    ns: u8,
    dir: bool,
    used: bool,
}

pub struct Index {
    recs: Vec<Rec>,
    /// Volume root as a path, e.g. `C:\`.
    root: PathBuf,
}

struct Boot {
    cluster: u64,
    record: usize,
    mft_offset: u64,
}

fn parse_boot(b: &[u8]) -> io::Result<Boot> {
    if b.get(3..11) != Some(b"NTFS    ") {
        return Err(bad("not an NTFS volume"));
    }
    let bps = u16_at(b, 0x0B).ok_or_else(|| bad("boot"))? as u64;
    let spc_raw = b[0x0D];
    let spc = if spc_raw > 0x80 {
        1u64 << (256 - spc_raw as u32)
    } else {
        spc_raw as u64
    };
    let cluster = bps * spc;
    let mft_lcn = u64_at(b, 0x30).ok_or_else(|| bad("boot"))?;
    let cpr = b[0x40] as i8;
    let record = if cpr > 0 {
        cpr as u64 * cluster
    } else {
        1u64 << (-(cpr as i64))
    } as usize;
    if cluster == 0 || !(256..=65536).contains(&record) || bps < 512 {
        return Err(bad("unexpected boot sector values"));
    }
    Ok(Boot {
        cluster,
        record,
        mft_offset: mft_lcn * cluster,
    })
}

/// Apply the update sequence array (sector fixups). Returns false on a torn record.
fn fixup(rec: &mut [u8]) -> bool {
    let (Some(ofs), Some(count)) = (u16_at(rec, 4), u16_at(rec, 6)) else {
        return false;
    };
    let (ofs, count) = (ofs as usize, count as usize);
    if count == 0 || ofs + count * 2 > rec.len() {
        return false;
    }
    let usn = [rec[ofs], rec[ofs + 1]];
    for i in 1..count {
        let end = i * 512;
        if end > rec.len() || rec[end - 2..end] != usn {
            return false;
        }
        rec[end - 2] = rec[ofs + i * 2];
        rec[end - 1] = rec[ofs + i * 2 + 1];
    }
    true
}

/// Decode a non-resident attribute's mapping pairs into (lcn, clusters) runs.
fn data_runs(b: &[u8]) -> Vec<(i64, u64)> {
    let mut out = vec![];
    let mut i = 0;
    let mut lcn: i64 = 0;
    while i < b.len() && b[i] != 0 {
        let h = b[i];
        let (ln, on) = ((h & 0x0F) as usize, (h >> 4) as usize);
        i += 1;
        if ln == 0 || ln > 8 || on > 8 || i + ln + on > b.len() {
            break;
        }
        let mut len: u64 = 0;
        for k in 0..ln {
            len |= (b[i + k] as u64) << (8 * k);
        }
        i += ln;
        if on == 0 {
            // sparse run: no clusters on disk
            out.push((-1, len));
            continue;
        }
        let mut off: i64 = 0;
        for k in 0..on {
            off |= (b[i + k] as i64) << (8 * k);
        }
        if b[i + on - 1] & 0x80 != 0 && on < 8 {
            off |= -1i64 << (8 * on);
        }
        i += on;
        lcn += off;
        out.push((lcn, len));
    }
    out
}

struct Parsed {
    rec: Rec,
    /// For extension records: the base record they belong to.
    base: u32,
    /// Unnamed $DATA size, if this record holds its first (VCN 0) piece.
    data_size: Option<u64>,
    /// $DATA mapping pairs of the unnamed stream (used for $MFT itself).
    runs: Vec<(i64, u64)>,
}

fn parse_record(rec: &mut [u8]) -> Option<Parsed> {
    if rec.get(0..4)? != b"FILE" || !fixup(rec) {
        return None;
    }
    let flags = u16_at(rec, 0x16)?;
    let base = (u64_at(rec, 0x20)? & 0xFFFF_FFFF_FFFF) as u32;
    let mut p = Parsed {
        rec: Rec {
            used: flags & 1 != 0,
            dir: flags & 2 != 0,
            ns: 255,
            ..Default::default()
        },
        base,
        data_size: None,
        runs: vec![],
    };
    let mut off = u16_at(rec, 0x14)? as usize;
    while off + 16 <= rec.len() {
        let kind = u32_at(rec, off)?;
        if kind == 0xFFFF_FFFF {
            break;
        }
        let len = u32_at(rec, off + 4)? as usize;
        if len < 16 || off + len > rec.len() {
            break;
        }
        let a = &rec[off..off + len];
        let nonres = a[8] != 0;
        let name_len = a[9];
        match kind {
            0x30 if !nonres => {
                let vo = u16_at(a, 0x14)? as usize;
                let v = a.get(vo..)?;
                let parent = (u64_at(v, 0)? & 0xFFFF_FFFF_FFFF) as u32;
                let n = *v.get(0x40)? as usize;
                let ns = *v.get(0x41)?;
                let units: Vec<u16> = v
                    .get(0x42..0x42 + n * 2)?
                    .chunks(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                // prefer Win32 / POSIX names over DOS short names
                let better = p.rec.name.is_none() || (p.rec.ns == 2 && ns != 2);
                if better {
                    p.rec.parent = parent;
                    p.rec.name = Some(String::from_utf16_lossy(&units).into_boxed_str());
                    p.rec.ns = ns;
                }
            }
            0x80 if name_len == 0 => {
                if nonres {
                    let lowest_vcn = u64_at(a, 0x10)?;
                    if lowest_vcn == 0 {
                        p.data_size = Some(u64_at(a, 0x30)?);
                    }
                    let ro = u16_at(a, 0x20)? as usize;
                    p.runs = data_runs(a.get(ro..)?);
                } else {
                    p.data_size = Some(u32_at(a, 0x10)? as u64);
                }
            }
            _ => {}
        }
        off += len;
    }
    Some(p)
}

/// Read a volume (a `\\.\C:` handle or an image file) and index its MFT.
pub fn scan<R: Read + Seek>(mut vol: R, root: PathBuf) -> io::Result<Index> {
    let mut boot = vec![0u8; 4096];
    vol.read_exact(&mut boot)?;
    let b = parse_boot(&boot)?;

    // record 0 is $MFT itself: its $DATA runs say where the rest of the table is
    let mut first = vec![0u8; b.record.max(4096)];
    vol.seek(SeekFrom::Start(b.mft_offset))?;
    vol.read_exact(&mut first)?;
    let mut r0 = first[..b.record].to_vec();
    let mft = parse_record(&mut r0).ok_or_else(|| bad("unreadable $MFT record"))?;
    let total_bytes = mft.data_size.ok_or_else(|| bad("$MFT has no data size"))?;
    let run_bytes: u64 = mft.runs.iter().map(|r| r.1 * b.cluster).sum();
    if mft.runs.is_empty() || run_bytes < total_bytes {
        // the $MFT is so fragmented that its run list continues in an attribute
        // list record; not supported, the caller falls back to a normal walk
        return Err(bad("$MFT run list is incomplete"));
    }
    let count = (total_bytes / b.record as u64) as usize;
    let mut recs: Vec<Rec> = vec![Rec::default(); count];
    let mut ext_sizes: Vec<(u32, u64)> = vec![];
    let mut ext_names: Vec<(u32, Rec)> = vec![];

    let mut index = 0usize;
    let mut buf = vec![0u8; CHUNK as usize];
    'runs: for (lcn, clusters) in mft.runs {
        let mut left = clusters * b.cluster;
        if lcn < 0 {
            // sparse part of the table: records there are unused
            index += (left / b.record as u64) as usize;
            continue;
        }
        let mut pos = lcn as u64 * b.cluster;
        while left > 0 && index < count {
            let n = left.min(CHUNK);
            vol.seek(SeekFrom::Start(pos))?;
            vol.read_exact(&mut buf[..n as usize])?;
            for chunk in buf[..n as usize].chunks_mut(b.record) {
                if index >= count {
                    break 'runs;
                }
                if let Some(p) = parse_record(chunk)
                    && p.rec.used
                {
                    if p.base != 0 && p.base as usize != index {
                        if let Some(s) = p.data_size {
                            ext_sizes.push((p.base, s));
                        }
                        if p.rec.name.is_some() {
                            ext_names.push((p.base, p.rec));
                        }
                    } else {
                        let mut r = p.rec;
                        r.size = p.data_size.unwrap_or(0);
                        recs[index] = r;
                    }
                }
                index += 1;
            }
            pos += n;
            left -= n;
        }
    }
    // pieces that live in extension records (files with many attributes)
    for (base, size) in ext_sizes {
        if let Some(r) = recs.get_mut(base as usize) {
            r.size = size;
        }
    }
    for (base, ext) in ext_names {
        if let Some(r) = recs.get_mut(base as usize)
            && r.name.is_none()
        {
            r.name = ext.name;
            r.parent = ext.parent;
        }
    }
    if recs.get(ROOT as usize).is_none_or(|r| !r.used || !r.dir) {
        return Err(bad("no root directory record"));
    }
    Ok(Index { recs, root })
}

/// Open a drive letter's volume (`C:`) and scan it. Windows + administrator only.
#[cfg(windows)]
pub fn scan_drive(path: &Path) -> io::Result<Index> {
    let s = path.to_string_lossy();
    let s = s.trim_start_matches(r"\\?\");
    let letter = s
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic() && s.get(1..2) == Some(":"))
        .ok_or_else(|| bad("not a drive path"))?;
    let vol = std::fs::File::open(format!(r"\\.\{letter}:"))?;
    scan(
        vol,
        PathBuf::from(format!(r"{}:\", letter.to_ascii_uppercase())),
    )
}

#[cfg(not(windows))]
pub fn scan_drive(_path: &Path) -> io::Result<Index> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "MFT scan is Windows-only",
    ))
}

fn key(p: &Path) -> String {
    p.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

impl Index {
    #[cfg(test)]
    pub fn file_count(&self) -> usize {
        self.recs.iter().filter(|r| r.used && !r.dir).count()
    }

    fn parent_of(&self, i: u32) -> Option<u32> {
        let r = self.recs.get(i as usize)?;
        (i != ROOT && r.used).then_some(r.parent)
    }

    /// NTFS's own bookkeeping files, which Explorer does not count either.
    fn is_meta(&self, i: u32) -> bool {
        i < FIRST_USER || self.under(i, EXTEND)
    }

    /// Total size below every directory (files counted once, cycles ignored).
    pub fn dir_totals(&self) -> Vec<u64> {
        let mut tot = vec![0u64; self.recs.len()];
        for (i, r) in self.recs.iter().enumerate() {
            if !r.used || r.dir || r.size == 0 || self.is_meta(i as u32) {
                continue;
            }
            let mut cur = i as u32;
            for _ in 0..512 {
                let Some(p) = self.parent_of(cur) else { break };
                if p as usize >= tot.len() {
                    break;
                }
                tot[p as usize] += r.size;
                if p == ROOT {
                    break;
                }
                cur = p;
            }
        }
        tot
    }

    /// Full paths of all directories (index = record number).
    fn dir_paths(&self) -> Vec<Option<PathBuf>> {
        let mut out: Vec<Option<PathBuf>> = vec![None; self.recs.len()];
        out[ROOT as usize] = Some(self.root.clone());
        fn resolve(
            ix: &Index,
            i: u32,
            out: &mut Vec<Option<PathBuf>>,
            depth: u32,
        ) -> Option<PathBuf> {
            if let Some(p) = &out[i as usize] {
                return Some(p.clone());
            }
            if depth > 512 {
                return None;
            }
            let r = ix.recs.get(i as usize)?;
            if !r.used || !r.dir {
                return None;
            }
            let name = r.name.as_deref()?;
            let parent = resolve(ix, r.parent, out, depth + 1)?;
            let p = parent.join(name);
            out[i as usize] = Some(p.clone());
            Some(p)
        }
        for i in 0..self.recs.len() as u32 {
            if self.recs[i as usize].dir {
                resolve(self, i, &mut out, 0);
            }
        }
        out
    }

    /// Size of every directory on the volume, keyed by full path (for `analyze`).
    pub fn dir_sizes(&self) -> HashMap<PathBuf, u64> {
        let tot = self.dir_totals();
        self.dir_paths()
            .into_iter()
            .enumerate()
            .filter_map(|(i, p)| p.map(|p| (p, tot[i])))
            .collect()
    }

    fn record_of(&self, dir: &Path) -> Option<u32> {
        let want = key(dir);
        if want == key(&self.root) {
            return Some(ROOT);
        }
        self.dir_paths()
            .into_iter()
            .position(|p| p.is_some_and(|p| key(&p) == want))
            .map(|i| i as u32)
    }

    fn under(&self, mut i: u32, dir: u32) -> bool {
        for _ in 0..512 {
            if i == dir {
                return true;
            }
            match self.parent_of(i) {
                Some(p) if p != i => i = p,
                _ => return i == dir,
            }
        }
        false
    }

    fn files_under(&self, dir: &Path) -> Option<Vec<u32>> {
        let d = self.record_of(dir)?;
        Some(
            (0..self.recs.len() as u32)
                .filter(|&i| {
                    let r = &self.recs[i as usize];
                    r.used
                        && !r.dir
                        && r.name.is_some()
                        && !self.is_meta(i)
                        && self.under(r.parent, d)
                })
                .collect(),
        )
    }

    fn file_path(&self, i: u32, dirs: &[Option<PathBuf>]) -> Option<PathBuf> {
        let r = self.recs.get(i as usize)?;
        Some(
            dirs.get(r.parent as usize)?
                .as_ref()?
                .join(r.name.as_deref()?),
        )
    }

    /// The `n` biggest files below `dir`.
    pub fn largest(&self, dir: &Path, n: usize) -> Option<Vec<(PathBuf, u64)>> {
        let mut files = self.files_under(dir)?;
        files.sort_by_key(|&i| std::cmp::Reverse(self.recs[i as usize].size));
        files.truncate(n);
        let dirs = self.dir_paths();
        Some(
            files
                .into_iter()
                .filter_map(|i| Some((self.file_path(i, &dirs)?, self.recs[i as usize].size)))
                .collect(),
        )
    }

    /// (extension, bytes, files) below `dir`; extension is lowercase without the dot.
    pub fn by_extension(&self, dir: &Path) -> Option<Vec<(String, u64, u64)>> {
        let mut m: HashMap<String, (u64, u64)> = HashMap::new();
        for i in self.files_under(dir)? {
            let r = &self.recs[i as usize];
            let e = m
                .entry(ext_of(r.name.as_deref().unwrap_or("")))
                .or_default();
            e.0 += r.size;
            e.1 += 1;
        }
        Some(m.into_iter().map(|(k, (b, c))| (k, b, c)).collect())
    }

    /// Total bytes and file count below `dir`.
    #[cfg(test)]
    pub fn total(&self, dir: &Path) -> Option<(u64, u64)> {
        let files = self.files_under(dir)?;
        Some((
            files.iter().map(|&i| self.recs[i as usize].size).sum(),
            files.len() as u64,
        ))
    }
}

pub fn ext_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, e))
            if !stem.is_empty() && !e.is_empty() && e.len() <= 12 && !e.contains(' ') =>
        {
            e.to_lowercase()
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_lists() {
        // 0x21: 1-byte length, 2-byte offset; then a negative relative offset
        let runs = data_runs(&[0x21, 0x10, 0x00, 0x01, 0x11, 0x08, 0xF0, 0x01, 0x05, 0x00]);
        assert_eq!(runs, vec![(256, 16), (240, 8), (-1, 5)]);
    }

    #[test]
    fn fixups_detect_torn_records() {
        let mut r = vec![0u8; 1024];
        r[0..4].copy_from_slice(b"FILE");
        r[4..6].copy_from_slice(&0x30u16.to_le_bytes());
        r[6..8].copy_from_slice(&3u16.to_le_bytes());
        r[0x30..0x32].copy_from_slice(&[0xAB, 0xCD]);
        r[0x32..0x34].copy_from_slice(&[1, 2]);
        r[0x34..0x36].copy_from_slice(&[3, 4]);
        r[510..512].copy_from_slice(&[0xAB, 0xCD]);
        r[1022..1024].copy_from_slice(&[0xAB, 0xCD]);
        let mut ok = r.clone();
        assert!(fixup(&mut ok));
        assert_eq!(&ok[510..512], &[1, 2]);
        assert_eq!(&ok[1022..1024], &[3, 4]);
        r[1023] = 0;
        assert!(!fixup(&mut r));
    }

    #[test]
    fn extensions() {
        assert_eq!(ext_of("Movie.MKV"), "mkv");
        assert_eq!(ext_of(".gitignore"), "");
        assert_eq!(ext_of("README"), "");
        assert_eq!(ext_of("a.tar.gz"), "gz");
    }

    /// Full check against a real NTFS image: set KAZMA_NTFS_TEST=image.img|expected.txt
    /// where each expected line is "relative/dir<TAB>bytes".
    #[test]
    fn real_ntfs_image() {
        let Ok(spec) = std::env::var("KAZMA_NTFS_TEST") else {
            return;
        };
        let (img, expected) = spec.split_once('|').unwrap();
        let ix = scan(std::fs::File::open(img).unwrap(), PathBuf::from("/vol")).unwrap();
        let sizes = ix.dir_sizes();
        let mut checked = 0;
        for line in std::fs::read_to_string(expected).unwrap().lines() {
            let (rel, bytes) = line.split_once('\t').unwrap();
            let p = if rel == "." {
                PathBuf::from("/vol")
            } else {
                Path::new("/vol").join(rel)
            };
            let got = if rel == "." {
                ix.total(&p).unwrap().0
            } else {
                sizes[&p]
            };
            assert_eq!(got, bytes.parse::<u64>().unwrap(), "{rel}");
            checked += 1;
        }
        assert!(checked > 0);
        eprintln!("checked {checked} directories, {} files", ix.file_count());
    }
}
