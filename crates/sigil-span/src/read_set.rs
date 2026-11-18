//! The native build's READ SET: every file the build reads, with the CRC-32 and size
//! of the bytes it consumed; which of those files the build wrote itself first; which
//! executables it ran; and the membership of every module-directory scan.
//!
//! # One recorder, so the set is complete by construction
//!
//! The `.lst` source digest vouches that an artifact is a function of the files it
//! names. A file the build reads that the digest omits makes a stale artifact read as
//! fresh, and nothing about the artifact shows it. So the set is never assembled from a
//! list of the file kinds a build is expected to read. Every build read goes through
//! [`read`] or [`read_to_string`], which hand back the bytes AND record them, so a read
//! that reached the build reached this log. Two nets hold that rule:
//!
//!  * a source gate (`crates/sigil-span/tests/read_set_gate.rs`) refuses a raw
//!    `std::fs` read, `File::open` or `Command::new` anywhere a `sigil build` can run
//!    it, unless the site states why it is not a build input;
//!  * a runtime witness (`crates/sigil-cli/tests/lst_source_digest.rs`) runs real builds
//!    under an `open(2)` interposer and requires the files the kernel was asked to open
//!    to equal the digest's rows, in both directions.
//!
//! # What a record is
//!
//! A read's CRC-32 and size are computed from the bytes the caller receives, at the
//! moment it receives them, never from a later re-read: the digest names what the
//! build consumed even when the file changes afterwards. A path read twice with
//! different contents is kept as a [`Conflict`] rather than resolved, because the build
//! consumed two versions and no single row could say so honestly. Paths are keyed by
//! their canonical form (symlinks resolved), so one file reached through two spellings
//! is one record.
//!
//! # Process-wide, by design
//!
//! The log is a process global rather than a thread-local: the comptime evaluator runs
//! on a thread of its own, and a thread-local log would miss every `embed` it performs.
//! `sigil build` is one build per process, so the log is exactly that build's reads. A
//! test process that runs several builds accumulates their union, and [`reset`] clears
//! it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

/// Everything recorded since the last [`reset`].
struct Log {
    /// Canonical path to every distinct `(crc32, size)` observed reading it.
    reads: BTreeMap<PathBuf, BTreeSet<(u32, u64, [u8; 32])>>,
    /// Canonical paths this process wrote through [`write_generated`].
    written: BTreeSet<PathBuf>,
    /// Canonical paths of the executables this process ran through [`tool_command`].
    tools: BTreeSet<PathBuf>,
    /// Canonical scan root to every distinct sorted membership a scan of it found.
    scans: BTreeMap<PathBuf, BTreeSet<Vec<PathBuf>>>,
}

static LOG: Mutex<Log> = Mutex::new(Log {
    reads: BTreeMap::new(),
    written: BTreeSet::new(),
    tools: BTreeSet::new(),
    scans: BTreeMap::new(),
});

fn log() -> MutexGuard<'static, Log> {
    // A panic while holding the lock leaves the log exactly as consistent as the
    // insert it interrupted; every insert is a single set operation.
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The key a path is recorded under: its canonical form when the file exists, else
/// its absolute lexical form, so a file removed right after its read still keys the
/// same way every other spelling of it would have.
pub fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn record_bytes(path: &Path, bytes: &[u8]) {
    let key = canonical(path);
    // The SHA-256 is taken from the same bytes, at the same moment, as the CRC: the
    // build-data export names each file by it, and a later re-read could name bytes the
    // build never consumed.
    let observation = (crc32(bytes), bytes.len() as u64, sha256(bytes));
    log().reads.entry(key).or_default().insert(observation);
}

/// [`std::fs::read`], recording the bytes it returns.
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    record_bytes(path, &bytes);
    Ok(bytes)
}

/// [`std::fs::read_to_string`], recording the bytes it returns. Errors are the
/// standard library's own, unchanged: a file that is not UTF-8 was never consumed, so
/// it records nothing.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)?;
    record_bytes(path, text.as_bytes());
    Ok(text)
}

/// [`std::fs::write`] for a file the build GENERATES and later reads back. Recording
/// the write is what lets the digest mark the later read `origin=generated` from what
/// happened, rather than from where the file lives.
pub fn write_generated<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref();
    std::fs::write(path, contents)?;
    log().written.insert(canonical(path));
    Ok(())
}

/// A [`Command`] for an executable the build runs, recording the executable's bytes
/// as a read first. The tool's output reaches the artifact, so the tool is an input
/// exactly as a source file is; an unreadable tool is the same error it would be at
/// spawn.
pub fn tool_command<P: AsRef<Path>>(program: P) -> io::Result<Command> {
    let program = program.as_ref();
    let bytes = std::fs::read(program)?;
    record_bytes(program, &bytes);
    log().tools.insert(canonical(program));
    Ok(Command::new(program))
}

/// Record that a directory walk from `root` found exactly `files` (paths the walk
/// built under `root`). The walk's membership is an input of its own: a file that
/// APPEARS under the root after the build has no read record, yet the next build's
/// walk would find it. Members are kept as the walk spelled them, relative to the
/// root, so the record is what re-running the walk's rule reproduces.
pub fn record_scan(root: &Path, files: &[PathBuf]) {
    let mut members: Vec<PathBuf> = files
        .iter()
        .map(|f| f.strip_prefix(root).map(Path::to_path_buf).unwrap_or_else(|_| f.clone()))
        .collect();
    members.sort();
    log().scans.entry(canonical(root)).or_default().insert(members);
}

/// Forget everything recorded so far.
pub fn reset() {
    let mut log = log();
    log.reads.clear();
    log.written.clear();
    log.tools.clear();
    log.scans.clear();
}

/// One recorded read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRead {
    /// The canonical path read.
    pub path: PathBuf,
    /// CRC-32 (IEEE) of the bytes read.
    pub crc: u32,
    /// Byte length of the bytes read.
    pub size: u64,
    /// SHA-256 of the bytes read, computed from the same bytes as [`FileRead::crc`] at
    /// read time, never from a later re-read.
    pub sha256: [u8; 32],
    /// This process wrote the file through [`write_generated`].
    pub generated: bool,
    /// This process ran the file through [`tool_command`].
    pub tool: bool,
}

/// One recorded directory scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scan {
    /// The canonical scan root.
    pub root: PathBuf,
    /// The paths the walk found, relative to the root as the walk spelled them, sorted.
    pub files: Vec<PathBuf>,
}

/// A record the log cannot state as one row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Conflict {
    /// One path read with more than one distinct content.
    Read { path: PathBuf, observations: Vec<(u32, u64)> },
    /// One root scanned with more than one distinct membership.
    Scan { root: PathBuf, memberships: usize },
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Conflict::Read { path, observations } => {
                let seen: Vec<String> =
                    observations.iter().map(|(c, s)| format!("crc={c:08x} size={s}")).collect();
                write!(
                    f,
                    "{} was read {} times with different contents ({}), so it changed while the build was reading it",
                    path.display(),
                    observations.len(),
                    seen.join(", ")
                )
            }
            Conflict::Scan { root, memberships } => write!(
                f,
                "{} was scanned {memberships} times and the walks found different files, so the tree changed while the build was reading it",
                root.display()
            ),
        }
    }
}

/// The log's contents at one moment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Snapshot {
    /// Every path read with exactly one content, sorted by path.
    pub reads: Vec<FileRead>,
    /// Every root scanned with exactly one membership, sorted by root.
    pub scans: Vec<Scan>,
    /// Every path or root that was not.
    pub conflicts: Vec<Conflict>,
}

/// A copy of everything recorded since the last [`reset`].
pub fn snapshot() -> Snapshot {
    let log = log();
    let mut out = Snapshot::default();
    for (path, seen) in &log.reads {
        if seen.len() == 1 {
            let (crc, size, sha256) = *seen.iter().next().expect("one observation");
            out.reads.push(FileRead {
                path: path.clone(),
                crc,
                size,
                sha256,
                generated: log.written.contains(path),
                tool: log.tools.contains(path),
            });
        } else {
            out.conflicts.push(Conflict::Read {
                path: path.clone(),
                observations: seen.iter().map(|&(c, s, _)| (c, s)).collect(),
            });
        }
    }
    for (root, memberships) in &log.scans {
        if memberships.len() == 1 {
            let files = memberships.iter().next().expect("one membership").clone();
            out.scans.push(Scan { root: root.clone(), files });
        } else {
            out.conflicts.push(Conflict::Scan { root: root.clone(), memberships: memberships.len() });
        }
    }
    out
}

const SHA_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 (FIPS 180-4) of `data`. This crate has no dependencies (a gate holds it to
/// that), so the digest the read log needs is written out here; `sigil-link`'s export
/// hashes with the `sha2` crate, and a test there pins the two to each other.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let (blocks, rest) = data.as_chunks::<64>();
    let mut tail = rest.to_vec();
    tail.push(0x80);
    while tail.len() % 64 != 56 {
        tail.push(0);
    }
    tail.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    let full = blocks.iter().chain(tail.as_chunks::<64>().0);
    for block in full {
        let mut w = [0u32; 64];
        for (i, c) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*c);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA_K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

/// CRC-32 (IEEE 802.3, reflected, init and final XOR `0xFFFFFFFF`): the campaign's
/// provenance hash alongside byte size, and the one `zlib.crc32` computes.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    // The log is process-wide; tests that read it take this lock and look only at
    // paths under their own directory.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn scratch(case: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sigil_read_set_{case}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        canonical(&dir)
    }

    fn reads_under(snap: &Snapshot, dir: &Path) -> Vec<FileRead> {
        snap.reads.iter().filter(|r| r.path.starts_with(dir)).cloned().collect()
    }

    #[test]
    fn crc32_matches_the_standard_check_value() {
        // The CRC-32/ISO-HDLC check value, the one every reference implementation
        // (zlib.crc32 included) publishes for the ASCII string "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    fn hex(b: [u8; 32]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha256_matches_the_fips_vectors() {
        assert_eq!(hex(sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        // Two-block message, and the 55/56/64-byte padding boundaries.
        assert_eq!(
            hex(sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            hex(sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        for n in [55usize, 56, 63, 64, 65] {
            let _ = sha256(&vec![7u8; n]);
        }
    }

    #[test]
    fn a_read_records_the_bytes_it_returned() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = scratch("read");
        let p = dir.join("a.bin");
        std::fs::write(&p, b"\x01\x02\x03").unwrap();
        assert_eq!(read(&p).unwrap(), b"\x01\x02\x03");
        let text = dir.join("b.txt");
        std::fs::write(&text, "hello").unwrap();
        assert_eq!(read_to_string(&text).unwrap(), "hello");
        let got = reads_under(&snapshot(), &dir);
        assert_eq!(
            got,
            vec![
                FileRead { path: p, crc: crc32(b"\x01\x02\x03"), size: 3, sha256: sha256(b"\x01\x02\x03"), generated: false, tool: false },
                FileRead { path: text, crc: crc32(b"hello"), size: 5, sha256: sha256(b"hello"), generated: false, tool: false },
            ]
        );
    }

    #[test]
    fn two_spellings_of_one_file_are_one_record() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = scratch("spelling");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let p = dir.join("sub/x.emp");
        std::fs::write(&p, "module x\n").unwrap();
        read_to_string(&p).unwrap();
        read_to_string(dir.join("sub/../sub/./x.emp")).unwrap();
        assert_eq!(reads_under(&snapshot(), &dir).len(), 1);
    }

    #[test]
    fn a_file_read_with_two_contents_is_a_conflict_not_a_row() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = scratch("conflict");
        let p = dir.join("moving.emp");
        std::fs::write(&p, "one").unwrap();
        read(&p).unwrap();
        std::fs::write(&p, "two").unwrap();
        read(&p).unwrap();
        let snap = snapshot();
        assert!(reads_under(&snap, &dir).is_empty(), "a conflicting path must not yield a row");
        assert!(
            snap.conflicts.iter().any(|c| matches!(c, Conflict::Read { path, observations } if *path == p && observations.len() == 2)),
            "no conflict recorded: {:?}",
            snap.conflicts
        );
    }

    #[test]
    fn a_generated_write_and_a_tool_mark_their_reads() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = scratch("marks");
        let generated = dir.join("gen.bin");
        write_generated(&generated, b"G").unwrap();
        read(&generated).unwrap();
        let tool = dir.join("tool.sh");
        std::fs::write(&tool, "#!/bin/sh\n").unwrap();
        let _cmd = tool_command(&tool).unwrap();
        let got = reads_under(&snapshot(), &dir);
        assert!(got.iter().any(|r| r.path == generated && r.generated && !r.tool), "{got:?}");
        assert!(got.iter().any(|r| r.path == tool && r.tool && !r.generated), "{got:?}");
    }

    #[test]
    fn a_scan_records_its_membership_and_a_changed_walk_conflicts() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = scratch("scan");
        let a = dir.join("a.emp");
        let b = dir.join("b.emp");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        record_scan(&dir, &[b.clone(), a.clone()]);
        record_scan(&dir, &[a.clone(), b.clone()]);
        let snap = snapshot();
        let scan = snap.scans.iter().find(|s| s.root == dir).expect("scan recorded");
        assert_eq!(
            scan.files,
            vec![PathBuf::from("a.emp"), PathBuf::from("b.emp")],
            "membership is root-relative, sorted and deduplicated"
        );
        record_scan(&dir, &[a]);
        let snap = snapshot();
        assert!(!snap.scans.iter().any(|s| s.root == dir));
        assert!(snap.conflicts.iter().any(|c| matches!(c, Conflict::Scan { root, memberships: 2 } if *root == dir)));
    }
}
