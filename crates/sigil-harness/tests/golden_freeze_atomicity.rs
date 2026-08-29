//! THE GOLDEN FREEZE'S WRITE PATH, MEASURED UNDER A KILL.
//!
//! `golden/capture_goldens.sh --write` runs longer than an agent can hold a foreground
//! command open, so a kill part-way through a capture is an ordinary event. What that
//! kill leaves on disk is the whole subject here: a write path that copies each fresh ROM
//! straight onto its committed golden leaves a MIXTURE of two captures behind, and — since
//! `cp` is interruptible mid-file — can leave one golden TRUNCATED. Neither state
//! announces itself, and in a shared checkout both sit under whatever runs next.
//!
//! `golden/atomic_freeze.sh` is the staged commit that replaces it. These gates drive it
//! as a shell library against stand-in blobs in a scratch directory, because the real
//! ritual is a seven-target ROM build and a gate that could only run inside one would
//! never run.
//!
//! THE CONTROL IS THE POINT. [`Writer::Direct`] reproduces the bare-`cp` write path, and
//! two gates assert that the hazards ARE observed through it — a truncated blob, and a
//! mixed set. Without them the staged gates are assertions that have never been shown
//! capable of failing, which is the shape of a gate that passed from the day it was
//! written and would keep passing with the mechanism deleted.
//!
//! WHAT IS ASSERTED OF THE STAGED PATH, and it is deliberately narrower than "atomic":
//!
//!   1. Nothing is committed until the whole set is captured — a kill in the multi-minute
//!      staging stretch leaves the complete old set, untouched.
//!   2. No committed golden is open for writing during staging, so none can be truncated;
//!      a half-arrived blob lands in the staging area instead.
//!   3. A commit that stops part-way is DETECTABLE: the staging area survives holding
//!      exactly the blobs that did not land, carrying the marker that distinguishes it
//!      from abandoned capture output.
//!   4. The next run refuses over that state instead of capturing on top of it, and
//!      names the blobs that did not land.
//!   5. Abandoned capture output — a staging area with no commit marker — is discarded
//!      rather than refused, since the committed set is provably the complete old one.
//!
//! THE MIXED-SET WINDOW IS NOT CLOSED, and gate 3 is written to say so out loud: the
//! commit loop is one rename per target, and a kill between two of them leaves a mixture.
//! It is a window of milliseconds instead of minutes, and it is loud instead of silent.
//! A gate asserting the mixture is unreachable would be asserting something false.
//!
//! HOW THE PART-WAY COMMIT IS PRODUCED. Gate 3 does not race a kill against the rename
//! loop; it makes one rename FAIL, by staging a directory over an existing regular file.
//! That drives the loop's own abort path, so the state examined afterwards is the state
//! `freeze_commit` really leaves, not one this file constructed to look like it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The stand-in set, in the lexicographic order the commit loop's glob visits.
const TARGETS: &[&str] = &[
    "config_a.bin",
    "demo.bin",
    "demo.debug.bin",
    "s4.bin",
    "s4.debug.bin",
];

/// The blob whose rename is made to fail in the part-way-commit gate. Third of five, so
/// the loop has landed two and has two left — a mixture with both halves non-empty.
const STUMBLE: &str = "demo.debug.bin";

/// Long enough that `cp` flushes a partial destination before it blocks: the copy buffer
/// is a small multiple of the block size, so a source that stalls after a quarter of a
/// megabyte has already put bytes on the other side.
const BLOB_LEN: usize = 1 << 20;
/// How much of a blob the truncation gates push through before stalling the source.
const PARTIAL_LEN: usize = 1 << 18;

fn lib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/atomic_freeze.sh")
}

fn old_bytes(name: &str) -> Vec<u8> {
    pattern(&format!("OLD {name} "), BLOB_LEN)
}

fn new_bytes(name: &str) -> Vec<u8> {
    pattern(&format!("NEW {name} "), BLOB_LEN)
}

fn pattern(seed: &str, len: usize) -> Vec<u8> {
    seed.as_bytes().iter().copied().cycle().take(len).collect()
}

/// Which write path a fixture runs. `Direct` is the bare `cp` onto the committed golden;
/// `Staged` is `atomic_freeze.sh`.
#[derive(Clone, Copy, PartialEq)]
enum Writer {
    Direct,
    Staged,
}

impl Writer {
    /// The shell prologue defining `open_set`, `write_one` and `commit_set`.
    fn prologue(self) -> String {
        match self {
            Writer::Direct => "\
open_set()   { :; }
write_one()  { cp \"$SRC/$1\" \"$GOLDEN/$1\"; }
commit_set() { :; }
"
            .to_string(),
            Writer::Staged => format!(
                "\
source {lib}
open_set()   {{ freeze_open \"$GOLDEN\"; }}
write_one()  {{ freeze_stage \"$SRC/$1\" \"$1\"; }}
commit_set() {{ freeze_commit; }}
",
                lib = shell_quote(&lib())
            ),
        }
    }
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', r"'\''"))
}

/// A scratch capture: a `golden/` holding the committed old set, a `src/` holding the
/// fresh capture, and a run directory for the fixture's sentinels.
struct Bed {
    tmp: tempfile::TempDir,
}

impl Bed {
    fn plant() -> Bed {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bed = Bed { tmp };
        std::fs::create_dir_all(bed.golden()).expect("mkdir golden");
        std::fs::create_dir_all(bed.src()).expect("mkdir src");
        std::fs::create_dir_all(bed.run()).expect("mkdir run");
        for t in TARGETS {
            std::fs::write(bed.golden().join(t), old_bytes(t)).expect("plant golden");
            std::fs::write(bed.src().join(t), new_bytes(t)).expect("plant source");
        }
        bed
    }

    fn golden(&self) -> PathBuf {
        self.tmp.path().join("golden")
    }
    fn src(&self) -> PathBuf {
        self.tmp.path().join("src")
    }
    fn run(&self) -> PathBuf {
        self.tmp.path().join("run")
    }
    fn stage(&self) -> PathBuf {
        self.golden().join(".staging")
    }

    fn committed(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.golden().join(name))
            .unwrap_or_else(|e| panic!("read committed {name}: {e}"))
    }

    /// Which targets currently hold the fresh capture's bytes.
    fn fresh_set(&self) -> Vec<&'static str> {
        TARGETS
            .iter()
            .copied()
            .filter(|t| self.committed(t) == new_bytes(t))
            .collect()
    }

    /// Which targets still hold the committed old bytes, byte for byte.
    fn old_set(&self) -> Vec<&'static str> {
        TARGETS
            .iter()
            .copied()
            .filter(|t| self.committed(t) == old_bytes(t))
            .collect()
    }

    /// Targets holding neither set — a truncated or otherwise partial blob.
    fn partial_set(&self) -> Vec<&'static str> {
        TARGETS
            .iter()
            .copied()
            .filter(|t| {
                let got = self.committed(t);
                got != old_bytes(t) && got != new_bytes(t)
            })
            .collect()
    }

    fn write_fixture(&self, body: &str) -> PathBuf {
        let path = self.run().join("fixture.sh");
        std::fs::write(&path, body).expect("write fixture");
        path
    }
}

/// Run a shell fragment with the bed's paths in the environment, and return its output.
fn run_shell(bed: &Bed, writer: Writer, body: &str) -> std::process::Output {
    let script = format!(
        "set -euo pipefail\nSRC={src}\nGOLDEN={golden}\nRUN={run}\n{prologue}\n{body}\n",
        src = shell_quote(&bed.src()),
        golden = shell_quote(&bed.golden()),
        run = shell_quote(&bed.run()),
        prologue = writer.prologue(),
    );
    let path = bed.write_fixture(&script);
    Command::new("bash")
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("could not run the fixture: {e}"))
}

/// Spawn a fixture in its own process group and kill the whole group once `sentinel`
/// appears. Descendants matter: the copy itself is a `cp` child, and killing only the
/// shell would leave it writing.
fn run_until_sentinel_then_kill(bed: &Bed, writer: Writer, body: &str, sentinel: &str) {
    let script = format!(
        "set -euo pipefail\nSRC={src}\nGOLDEN={golden}\nRUN={run}\necho $$ > \"$RUN/pgid\"\n{prologue}\n{body}\n",
        src = shell_quote(&bed.src()),
        golden = shell_quote(&bed.golden()),
        run = shell_quote(&bed.run()),
        prologue = writer.prologue(),
    );
    let path = bed.write_fixture(&script);
    let mut child = Command::new("setsid")
        .arg("bash")
        .arg(&path)
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn the fixture under setsid: {e}"));

    await_path(&bed.run().join(sentinel), "the fixture never reached its sentinel");
    let pgid = std::fs::read_to_string(bed.run().join("pgid"))
        .expect("the fixture never recorded its process group")
        .trim()
        .to_string();
    let killed = Command::new("bash")
        .arg("-c")
        .arg(format!("kill -9 -- -{pgid}"))
        .status()
        .unwrap_or_else(|e| panic!("could not kill the fixture group: {e}"));
    assert!(killed.success(), "kill -9 on process group {pgid} failed");
    let _ = child.wait();
}

/// Block until `p` exists. Panics rather than returning: a gate that proceeded without
/// its precondition would measure a race, not a mechanism.
fn await_path(p: &Path, what: &str) {
    await_true(|| p.exists(), what);
}

fn await_true(mut cond: impl FnMut() -> bool, what: &str) {
    for _ in 0..2000 {
        if cond() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("{what} (waited 10s)");
}

/// A fixture body that writes every target, pausing after `TARGETS[1]` on a fifo the test
/// never opens — so the kill lands squarely between two targets, deterministically.
fn body_pause_midway() -> String {
    format!(
        "mkfifo \"$RUN/gate\"\nopen_set\nfor t in {targets}; do\n  write_one \"$t\"\n  if [[ \"$t\" == \"{pause}\" ]]; then touch \"$RUN/paused\"; read -r _ < \"$RUN/gate\"; fi\ndone\ncommit_set\ntouch \"$RUN/done\"\n",
        targets = TARGETS.join(" "),
        pause = TARGETS[1],
    )
}

/// A fixture body that writes exactly one target whose source is a fifo. The test opens
/// the write end, pushes part of the blob and holds it, so the copy is stalled mid-file
/// at a point the test chooses.
fn body_stall_mid_file(name: &str) -> String {
    format!(
        "rm -f \"$SRC/{name}\"\nmkfifo \"$SRC/{name}\"\nopen_set\ntouch \"$RUN/armed\"\nwrite_one \"{name}\"\ntouch \"$RUN/copied\"\n"
    )
}

/// Feed `PARTIAL_LEN` bytes into the fifo the fixture is copying from, and hold it open.
/// Returns the still-open write end: dropping it would let the copy finish.
fn stall_source(bed: &Bed, name: &str) -> std::fs::File {
    await_path(&bed.run().join("armed"), "the fixture never armed its fifo source");
    // Opening a fifo for writing blocks until the copy opens the read end, so this
    // returns only once the copy is genuinely under way — and never returns at all if the
    // fixture died first. It runs on its own thread against a deadline so that failure is
    // a named red rather than a suite that hangs, which is strictly worse than a red.
    let path = bed.src().join(name);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string());
        let _ = tx.send(opened);
    });
    let mut w = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .unwrap_or_else(|_| {
            panic!("nothing opened the fifo source for reading within 20s — the fixture never reached its copy")
        })
        .unwrap_or_else(|e| panic!("open fifo source for write: {e}"));
    let blob = new_bytes(name);
    w.write_all(&blob[..PARTIAL_LEN]).expect("push the partial blob");
    w.flush().expect("flush the partial blob");
    w
}

// ---------------------------------------------------------------------------
// THE CONTROLS. The bare-`cp` write path, and the two states it really leaves.
// ---------------------------------------------------------------------------

/// A kill between two targets leaves a MIXTURE of two captures. This is the defect, held
/// in the suite so the staged gates below are known to be capable of failing.
#[test]
fn the_direct_write_path_leaves_a_mixed_set_when_killed_between_targets() {
    let bed = Bed::plant();
    run_until_sentinel_then_kill(&bed, Writer::Direct, &body_pause_midway(), "paused");

    let fresh = bed.fresh_set();
    let old = bed.old_set();
    assert_eq!(
        fresh,
        TARGETS[..2].to_vec(),
        "the control did not reproduce the defect: fresh={fresh:?} old={old:?}"
    );
    assert_eq!(
        old,
        TARGETS[2..].to_vec(),
        "the control did not reproduce the defect: fresh={fresh:?} old={old:?}"
    );
    assert!(
        !fresh.is_empty() && !old.is_empty(),
        "a mixture needs both halves non-empty"
    );
}

/// A kill mid-file leaves a committed golden TRUNCATED — neither capture's bytes, and
/// nothing on disk saying so.
#[test]
fn the_direct_write_path_leaves_a_truncated_golden_when_killed_mid_file() {
    let bed = Bed::plant();
    let name = TARGETS[0];
    let script = format!(
        "set -euo pipefail\nSRC={src}\nGOLDEN={golden}\nRUN={run}\necho $$ > \"$RUN/pgid\"\n{prologue}\n{body}\n",
        src = shell_quote(&bed.src()),
        golden = shell_quote(&bed.golden()),
        run = shell_quote(&bed.run()),
        prologue = Writer::Direct.prologue(),
        body = body_stall_mid_file(name),
    );
    let path = bed.write_fixture(&script);
    let mut child = Command::new("setsid")
        .arg("bash")
        .arg(&path)
        .spawn()
        .expect("spawn the fixture");
    let w = stall_source(&bed, name);

    let dest = bed.golden().join(name);
    await_true(
        || {
            std::fs::metadata(&dest)
                .map(|m| m.len() as usize)
                .map(|n| n > 0 && n < BLOB_LEN)
                .unwrap_or(false)
        },
        "the committed golden never held a partial length",
    );
    let seen = std::fs::read(&dest).expect("read the committed golden mid-copy");

    let pgid = std::fs::read_to_string(bed.run().join("pgid")).expect("pgid");
    let _ = Command::new("bash")
        .arg("-c")
        .arg(format!("kill -9 -- -{}", pgid.trim()))
        .status();
    drop(w);
    let _ = child.wait();

    assert!(
        seen.len() < BLOB_LEN && !seen.is_empty(),
        "the control did not reproduce a truncated golden: {} bytes of {BLOB_LEN}",
        seen.len()
    );
    assert_ne!(seen, old_bytes(name), "a truncated blob is not the old blob");
    assert_ne!(seen, new_bytes(name), "a truncated blob is not the new blob");
}

// ---------------------------------------------------------------------------
// THE STAGED PATH.
// ---------------------------------------------------------------------------

/// The whole set moves, and nothing is left behind.
#[test]
fn a_completed_staged_capture_commits_the_whole_set_and_leaves_no_staging_area() {
    let bed = Bed::plant();
    let body = format!(
        "open_set\nfor t in {}; do write_one \"$t\"; done\ncommit_set\n",
        TARGETS.join(" ")
    );
    let out = run_shell(&bed, Writer::Staged, &body);
    assert!(
        out.status.success(),
        "the staged capture failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(bed.fresh_set(), TARGETS.to_vec(), "not every golden moved");
    assert!(
        !bed.stage().exists(),
        "the staging area survived a successful commit: {}",
        bed.stage().display()
    );
}

/// THE LONG WINDOW. A kill anywhere in the multi-minute staging stretch leaves the
/// committed set complete and byte-identical — the case the direct path gets wrong.
#[test]
fn a_kill_during_staging_leaves_the_complete_old_committed_set() {
    let bed = Bed::plant();
    run_until_sentinel_then_kill(&bed, Writer::Staged, &body_pause_midway(), "paused");

    assert_eq!(
        bed.old_set(),
        TARGETS.to_vec(),
        "a kill during staging altered committed goldens: fresh={:?} partial={:?}",
        bed.fresh_set(),
        bed.partial_set()
    );
    assert!(
        bed.stage().join(TARGETS[0]).exists(),
        "the capture that had happened was not held in the staging area"
    );
    assert!(
        !bed.stage().join(".committing").exists(),
        "a staging area that never reached the commit loop carries the commit marker"
    );
}

/// NO TRUNCATED COMMITTED BLOB. A copy stalled mid-file writes into the staging area;
/// the committed golden is not open for writing at all, and reads back byte-identical to
/// the old set while the partial blob is visibly accumulating beside it.
#[test]
fn a_copy_stalled_mid_file_cannot_disturb_the_committed_golden() {
    let bed = Bed::plant();
    let name = TARGETS[0];
    let script = format!(
        "set -euo pipefail\nSRC={src}\nGOLDEN={golden}\nRUN={run}\necho $$ > \"$RUN/pgid\"\n{prologue}\n{body}\n",
        src = shell_quote(&bed.src()),
        golden = shell_quote(&bed.golden()),
        run = shell_quote(&bed.run()),
        prologue = Writer::Staged.prologue(),
        body = body_stall_mid_file(name),
    );
    let path = bed.write_fixture(&script);
    let mut child = Command::new("setsid")
        .arg("bash")
        .arg(&path)
        .spawn()
        .expect("spawn the fixture");
    let w = stall_source(&bed, name);

    let staged = bed.stage().join(name);
    await_true(
        || {
            std::fs::metadata(&staged)
                .map(|m| m.len() as usize)
                .map(|n| n > 0 && n < BLOB_LEN)
                .unwrap_or(false)
        },
        "the partial blob never appeared in the staging area",
    );
    let committed = bed.committed(name);

    let pgid = std::fs::read_to_string(bed.run().join("pgid")).expect("pgid");
    let _ = Command::new("bash")
        .arg("-c")
        .arg(format!("kill -9 -- -{}", pgid.trim()))
        .status();
    drop(w);
    let _ = child.wait();

    assert_eq!(
        committed,
        old_bytes(name),
        "the committed golden changed while a copy was mid-file — {} bytes",
        committed.len()
    );
    assert_eq!(
        bed.old_set(),
        TARGETS.to_vec(),
        "a stalled copy disturbed the committed set: partial={:?}",
        bed.partial_set()
    );
}

/// THE RESIDUAL WINDOW, STATED AS A MEASUREMENT. A commit that stops part-way DOES leave
/// a mixture — the loop is one rename per target — and what the mechanism guarantees is
/// that no blob is truncated and that the leftover names exactly what did not land.
///
/// The part-way stop is produced by making one rename fail rather than by racing a kill,
/// so this is the loop's own abort path leaving its own state.
#[test]
fn a_commit_that_stops_part_way_leaves_a_mixture_that_names_what_did_not_land() {
    let bed = Bed::plant();
    let body = format!(
        "open_set\nfor t in {targets}; do write_one \"$t\"; done\nrm -f \"$GOLDEN/.staging/{stumble}\"\nmkdir \"$GOLDEN/.staging/{stumble}\"\ncommit_set\n",
        targets = TARGETS.join(" "),
        stumble = STUMBLE,
    );
    let out = run_shell(&bed, Writer::Staged, &body);
    assert!(
        !out.status.success(),
        "the commit loop reported success after a rename it could not perform"
    );

    // No blob is truncated: every committed golden is exactly one capture's bytes.
    assert!(
        bed.partial_set().is_empty(),
        "a part-way commit truncated a golden: {:?}",
        bed.partial_set()
    );
    // And a mixture IS reachable — asserted, not wished away.
    let landed = bed.fresh_set();
    assert_eq!(
        landed,
        TARGETS[..2].to_vec(),
        "the loop did not stop where the failing rename is"
    );

    // The leftover holds exactly the blobs that did not land, under the commit marker.
    assert!(
        bed.stage().join(".committing").exists(),
        "the leftover does not record that the commit loop had begun"
    );
    for t in &TARGETS[3..] {
        assert!(
            bed.stage().join(t).exists(),
            "{t} did not land and is not in the staging area either"
        );
    }
    for t in &TARGETS[..2] {
        assert!(
            !bed.stage().join(t).exists(),
            "{t} landed but is still staged"
        );
    }
}

/// The next run refuses over that state and names the blobs that did not land, rather
/// than capturing a second set on top of a mixture.
#[test]
fn a_leftover_from_a_part_way_commit_refuses_the_next_run() {
    let bed = Bed::plant();
    std::fs::create_dir_all(bed.stage()).expect("mkdir staging");
    std::fs::write(bed.stage().join(".committing"), "").expect("marker");
    std::fs::write(bed.stage().join(STUMBLE), new_bytes(STUMBLE)).expect("stranded blob");

    let out = run_shell(&bed, Writer::Staged, "open_set\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "freeze_open accepted a staging area left by a part-way commit: {err}"
    );
    assert!(
        err.contains(STUMBLE),
        "the refusal did not name the blob that did not land: {err}"
    );
    assert!(
        bed.stage().join(STUMBLE).exists(),
        "the refusal destroyed the evidence it refused over"
    );
}

/// Abandoned capture output is discarded, not refused: with no commit marker the
/// committed set is provably the complete old one, so there is nothing to adjudicate.
#[test]
fn a_leftover_from_an_abandoned_capture_is_discarded_and_the_run_proceeds() {
    let bed = Bed::plant();
    std::fs::create_dir_all(bed.stage()).expect("mkdir staging");
    std::fs::write(bed.stage().join(TARGETS[0]), b"stale capture output").expect("stale blob");

    let body = format!(
        "open_set\nfor t in {}; do write_one \"$t\"; done\ncommit_set\n",
        TARGETS.join(" ")
    );
    let out = run_shell(&bed, Writer::Staged, &body);
    assert!(
        out.status.success(),
        "an abandoned staging area blocked a fresh capture: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        bed.fresh_set(),
        TARGETS.to_vec(),
        "the stale blob survived into the committed set"
    );
    assert!(!bed.stage().exists(), "the staging area survived the commit");
}

/// `freeze_abandon` clears capture output but declines once the commit loop has begun:
/// past that point the staged blobs are the only copy of what the loop did not install,
/// and the leftover is the only record that the committed set may be mixed. This is what
/// keeps the EXIT trap from erasing the evidence on the way out.
#[test]
fn abandon_clears_capture_output_and_declines_to_erase_a_part_way_commit() {
    let bed = Bed::plant();
    let out = run_shell(
        &bed,
        Writer::Staged,
        &format!("open_set\nwrite_one \"{}\"\nfreeze_abandon\n", TARGETS[0]),
    );
    assert!(
        out.status.success(),
        "abandon refused ordinary capture output: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!bed.stage().exists(), "abandon left the staging area behind");

    let bed = Bed::plant();
    let out = run_shell(
        &bed,
        Writer::Staged,
        &format!(
            "open_set\nwrite_one \"{}\"\n: > \"$GOLDEN/.staging/.committing\"\nfreeze_abandon\n",
            TARGETS[0]
        ),
    );
    assert!(
        !out.status.success(),
        "abandon erased a staging area whose commit loop had begun"
    );
    assert!(
        bed.stage().join(TARGETS[0]).exists(),
        "abandon destroyed a blob the commit loop had not installed"
    );
}
