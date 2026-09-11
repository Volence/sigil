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
    reads: BTreeMap<PathBuf, BTreeSet<(u32, u64)>>,
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
    let observation = (crc32(bytes), bytes.len() as u64);
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
            let (crc, size) = *seen.iter().next().expect("one observation");
            out.reads.push(FileRead {
                path: path.clone(),
                crc,
                size,
                generated: log.written.contains(path),
                tool: log.tools.contains(path),
            });
        } else {
            out.conflicts.push(Conflict::Read {
                path: path.clone(),
                observations: seen.iter().copied().collect(),
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
                FileRead { path: p, crc: crc32(b"\x01\x02\x03"), size: 3, generated: false, tool: false },
                FileRead { path: text, crc: crc32(b"hello"), size: 5, generated: false, tool: false },
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
