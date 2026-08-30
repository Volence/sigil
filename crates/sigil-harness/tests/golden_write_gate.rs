//! THE GOLDEN WRITE GATE — that a hand-run `--write` is refused, and that the journalled
//! one is not.
//!
//! `golden/capture_goldens.sh --write` replaces the seven committed golden blobs, which
//! are the measuring instrument every byte gate and every paired freeze reads. Through
//! `refreeze --freeze` that write leaves a completion journal and a provenance entry;
//! run by hand it moves the same bytes and leaves nothing, and the two forms are one flag
//! apart. The script's WRITE GATE closes that, and these gates hold it to both halves.
//!
//! BOTH HALVES, because a gate proven only on its refusal is indistinguishable from one
//! that refuses everything — including the ritual whose landing it would then block. So
//! the refusal is asserted, and so is a COMPLETE seven-target write running through to
//! `freeze_commit` with the journalled caller's environment.
//!
//! WITHOUT A REAL CAPTURE. A real one needs a provisioned aeon tree, twenty minutes, and
//! it rewrites the committed blobs — a gate that could only run inside one would never
//! run. These drive the REAL script against a stand-in bed: a golden directory of
//! recognisable stand-in blobs, and a stub aeon whose `build.sh` and stub `sigil` write
//! recognisable stand-in ROMs. The script is the shipped one, copied; only what it builds
//! is fake.
//!
//! THE PLUMBING IS MEASURED, NOT ASSUMED. `refreeze_hands_the_script_a_token_it_accepts`
//! runs the real `refreeze` binary far enough to spawn the capture step, captures the
//! environment it hands its child, and then runs the REAL script under that environment
//! and requires it not to be refused. Nothing here names the variable: what is asserted is
//! that whatever `refreeze` exports is enough, so a rename on either side that breaks the
//! journalled path fails this gate rather than passing it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The seven goldens the capture writes, in the script's own order.
const TARGETS: &[&str] = &[
    "s4.bin",
    "s4.debug.bin",
    "demo.bin",
    "demo.debug.bin",
    "config_a.bin",
    "config_b.bin",
    "lean.bin",
];

/// The phrase this rule and nothing else prints. Asserted verbatim, so a run that failed
/// for any other reason — a missing aeon tree, a missing emitter, a syntax error — cannot
/// be mistaken for the gate firing.
const REFUSAL: &str = "refusing an unjournalled golden write";

/// The trace an acknowledged hand write appends to, beside the blobs.
const TRACE: &str = ".unjournalled-write";

fn golden_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
}

fn pattern(seed: &str) -> Vec<u8> {
    seed.as_bytes().iter().copied().cycle().take(1024).collect()
}

/// What the bed's goldens hold before a capture.
fn old_bytes(name: &str) -> Vec<u8> {
    pattern(&format!(" OLD {name}"))
}

/// What the bed's stub builders produce, and therefore what a completed write leaves in
/// each committed golden.
fn new_bytes(name: &str) -> Vec<u8> {
    pattern(&format!(" NEW {name}"))
}

/// A stand-in capture bed: the shipped scripts over stand-in blobs, and a stub aeon.
struct Bed {
    tmp: tempfile::TempDir,
}

impl Bed {
    fn plant() -> Bed {
        let bed = Bed { tmp: tempfile::tempdir().expect("tempdir") };
        std::fs::create_dir_all(bed.golden()).expect("mkdir golden");
        std::fs::create_dir_all(bed.aeon()).expect("mkdir aeon");
        std::fs::create_dir_all(bed.bin()).expect("mkdir bin");

        // The SHIPPED scripts, copied so the bed's own directory is what `$HERE` resolves
        // to and no committed golden is reachable from the run.
        for s in ["capture_goldens.sh", "atomic_freeze.sh"] {
            std::fs::copy(golden_src().join(s), bed.golden().join(s))
                .unwrap_or_else(|e| panic!("copy {s}: {e}"));
        }
        make_executable(&bed.golden().join("capture_goldens.sh"));

        for t in TARGETS {
            std::fs::write(bed.golden().join(t), old_bytes(t)).expect("plant golden");
        }

        // The canonical four go through aeon's build.sh, whose game argument is positional
        // and whose debug shape is selected by DEBUG=1 — the same two facts the real
        // script's capture() depends on.
        write_script(
            &bed.aeon().join("build.sh"),
            r#"#!/bin/bash
set -euo pipefail
game="${1:-sonic4}"
base=s4; [[ "$game" == demo ]] && base=demo
name="$base"; [[ "${DEBUG:-}" == "1" ]] && name="$base.debug"
python3 -c "import sys; sys.stdout.buffer.write(((' NEW %s.bin' % sys.argv[1]).encode()*200)[:1024])" "$name" > "$name.bin"
printf '     0/  00000200 : EndOfRom:\n' > "$name.lst"
"#,
        );
        // The three off-canonical shapes go through `sigil build --config-*`/`--lean`.
        write_script(
            &bed.bin().join("sigil"),
            r#"#!/bin/bash
set -euo pipefail
shape=""; rom=""; lst=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --config-a) shape=config_a;; --config-b) shape=config_b;; --lean) shape=lean;;
        -o) rom="$2"; shift;; --emit-lst) lst="$2"; shift;;
    esac
    shift
done
python3 -c "import sys; sys.stdout.buffer.write(((' NEW %s.bin' % sys.argv[1]).encode()*200)[:1024])" "$shape" > "$rom"
printf '     0/  00000200 : EndOfRom:\n' > "$lst"
"#,
        );
        write_script(&bed.bin().join("emit_sound_blob"), "#!/bin/bash\nexit 0\n");
        bed
    }

    fn golden(&self) -> PathBuf {
        self.tmp.path().join("golden")
    }
    fn aeon(&self) -> PathBuf {
        self.tmp.path().join("aeon")
    }
    fn bin(&self) -> PathBuf {
        self.tmp.path().join("bin")
    }
    fn trace(&self) -> PathBuf {
        self.golden().join(TRACE)
    }

    /// Run the bed's capture script the way an operator runs it, with `extra` folded into
    /// the environment.
    fn capture(&self, args: &[&str], extra: &[(&str, &str)]) -> Output {
        let mut cmd = Command::new("bash");
        cmd.arg(self.golden().join("capture_goldens.sh"))
            .args(args)
            .current_dir(self.golden())
            .env("AEON_DIR", self.aeon())
            .env("SIGIL_EMIT", self.bin().join("emit_sound_blob"))
            .env("SIGIL_BUILD", self.bin().join("sigil"))
            .env_remove("SIGIL_GOLDEN_WRITE");
        for (k, v) in extra {
            cmd.env(k, v);
        }
        cmd.output().expect("run the capture script")
    }

    /// Which goldens hold the stand-in capture's bytes, and which still hold the old set.
    fn fresh_set(&self) -> Vec<&'static str> {
        TARGETS.iter().copied().filter(|t| self.committed(t) == new_bytes(t)).collect()
    }
    fn old_set(&self) -> Vec<&'static str> {
        TARGETS.iter().copied().filter(|t| self.committed(t) == old_bytes(t)).collect()
    }
    fn committed(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.golden().join(name))
            .unwrap_or_else(|e| panic!("read committed {name}: {e}"))
    }
}

fn make_executable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(p).expect("stat").permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(p, perm).expect("chmod");
}

fn write_script(p: &Path, body: &str) {
    std::fs::write(p, body).unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
    make_executable(p);
}

fn merged(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── the refusal ─────────────────────────────────────────────────────────────

/// The hand form is refused, and refused BEFORE the run costs anything: no staging area
/// is opened, no golden is touched, and the environment checks are never reached — which
/// is what makes the refusal reachable in a tree with no aeon provisioned at all.
#[test]
fn an_unacknowledged_hand_write_is_refused_before_any_build() {
    let bed = Bed::plant();
    // A deliberately absent aeon: if the gate ever stops firing, this run stops at the
    // AEON_DIR check instead of capturing, so the gate's own failure cannot damage a bed.
    let out = bed.capture(&["--write"], &[("AEON_DIR", "/nonexistent-aeon-for-this-gate")]);
    let text = merged(&out);

    assert!(!out.status.success(), "the hand write was NOT refused:\n{text}");
    assert!(text.contains(REFUSAL), "the refusal did not name this rule:\n{text}");
    assert!(
        !text.contains("AEON_DIR not a dir"),
        "the gate must be consulted BEFORE the environment checks, so that a refusal \
         costs nothing and is provable without a provisioned tree:\n{text}"
    );
    // A refusal that does not name the safe command gets bypassed rather than obeyed.
    assert!(
        text.contains("--freeze") && text.contains("refreeze"),
        "the refusal must name the journalled command to run instead:\n{text}"
    );
    assert!(
        text.contains("SIGIL_GOLDEN_WRITE=unjournalled"),
        "the refusal must name the deliberate override, or the only way past it is to \
         edit the script:\n{text}"
    );

    assert_eq!(bed.old_set(), TARGETS, "a refused write moved a golden");
    assert!(!bed.golden().join(".staging").exists(), "a refused write opened a staging area");
    assert!(!bed.trace().exists(), "a refused write left a hand-write record");
}

/// The acknowledgement is allowed BECAUSE it is recorded, so a write whose record cannot
/// be written does not happen.
#[test]
fn a_hand_write_that_cannot_be_recorded_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let bed = Bed::plant();
    let mut perm = std::fs::metadata(bed.golden()).expect("stat golden").permissions();
    perm.set_mode(0o555);
    std::fs::set_permissions(bed.golden(), perm).expect("chmod golden");

    // LOUD ON UNMEASURABLE: root ignores the mode bits, and a gate that cannot arm its
    // own precondition must say so rather than pass.
    let armed = std::fs::write(bed.golden().join(".writability-probe"), b"x").is_err();
    if !armed {
        let mut perm = std::fs::metadata(bed.golden()).expect("stat").permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(bed.golden(), perm).expect("chmod back");
        panic!(
            "could not make the golden directory unwritable, so this gate could not be \
             armed (running as root?). It is NOT reported as a pass."
        );
    }

    let out = bed.capture(&["--write"], &[("SIGIL_GOLDEN_WRITE", "unjournalled")]);
    let text = merged(&out);

    let mut perm = std::fs::metadata(bed.golden()).expect("stat").permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(bed.golden(), perm).expect("chmod back");

    assert!(!out.status.success(), "an unrecordable hand write was performed:\n{text}");
    assert!(
        text.contains("cannot record this hand write"),
        "the refusal did not name the missing record:\n{text}"
    );
    assert_eq!(bed.old_set(), TARGETS, "an unrecordable hand write moved a golden");
}

// ── the write that must still work ──────────────────────────────────────────

/// THE HALF THAT GETS SKIPPED. With the journalled caller's token the whole seven-target
/// write runs to `freeze_commit` and every committed golden holds the fresh capture.
#[test]
fn the_journalled_caller_writes_the_whole_set() {
    let bed = Bed::plant();
    let out = bed.capture(&["--write"], &[("SIGIL_GOLDEN_WRITE", "refreeze")]);
    let text = merged(&out);

    assert!(out.status.success(), "the journalled write did not complete:\n{text}");
    assert!(!text.contains(REFUSAL), "the journalled write was refused:\n{text}");
    assert_eq!(bed.fresh_set(), TARGETS, "the committed set is not this capture's:\n{text}");
    assert!(!bed.golden().join(".staging").exists(), "the staging area outlived the commit");
    assert!(
        !bed.trace().exists(),
        "a journalled write left a hand-write record, which would report the ritual as a \
         hand run"
    );
}

/// Without `--write` nothing is gated and nothing is replaced: the read-only capture is
/// what most hand runs actually want, and the refusal names it.
#[test]
fn a_capture_without_write_is_neither_gated_nor_writing() {
    let bed = Bed::plant();
    let out = bed.capture(&[], &[]);
    let text = merged(&out);

    assert!(out.status.success(), "the read-only capture failed:\n{text}");
    assert!(!text.contains(REFUSAL), "the read-only capture was gated:\n{text}");
    assert_eq!(bed.old_set(), TARGETS, "a run without --write replaced a golden");
}

/// The acknowledged hand write proceeds AND is recorded — both halves of the record: that
/// one was attempted, and that one landed.
#[test]
fn an_acknowledged_hand_write_is_recorded_and_announced() {
    let bed = Bed::plant();
    let out = bed.capture(&["--write"], &[("SIGIL_GOLDEN_WRITE", "unjournalled")]);
    let text = merged(&out);

    assert!(out.status.success(), "the acknowledged hand write did not complete:\n{text}");
    assert_eq!(bed.fresh_set(), TARGETS, "the committed set is not this capture's:\n{text}");

    let record = std::fs::read_to_string(bed.trace()).expect("the hand write left no record");
    assert!(record.contains("started"), "the record does not say a hand write began:\n{record}");
    assert!(
        record.contains("committed"),
        "the record does not say the goldens moved, so a landed write reads like an \
         abandoned one:\n{record}"
    );

    // The NEXT write run reports what the goldens it is about to replace were carrying —
    // including a journalled one, which is the run whose report matters.
    let next = bed.capture(&["--write"], &[("SIGIL_GOLDEN_WRITE", "refreeze")]);
    let next_text = merged(&next);
    assert!(next.status.success(), "the follow-on journalled write failed:\n{next_text}");
    assert!(
        next_text.contains(TRACE) && next_text.contains("carry hand writes"),
        "a later freeze did not report the hand writes these goldens carry:\n{next_text}"
    );
    assert!(
        bed.trace().exists(),
        "a journalled freeze removed the record of a hand write; the record outlives the \
         blobs it wrote"
    );
}

// ── the plumbing, measured ──────────────────────────────────────────────────

/// What `refreeze` hands its capture child is enough to pass the gate.
///
/// The variable is never named here. `refreeze` is run far enough to spawn the capture
/// step against a stand-in harness root whose `capture_goldens.sh` is a stub that dumps
/// its environment; the REAL script is then run under exactly the environment `refreeze`
/// supplied, and must not be refused. A rename on either side breaks this gate instead of
/// breaking a landing.
#[test]
fn refreeze_hands_the_script_a_token_it_accepts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("harness");
    let golden = root.join("golden");
    std::fs::create_dir_all(&golden).expect("mkdir golden");

    // The two markers that identify a harness root, and nothing else: this tree exists to
    // be resolved, not to be frozen.
    std::fs::write(golden.join("provenance.toml"), "").expect("write provenance marker");
    std::fs::write(root.join("repin.toml"), "").expect("write repin marker");

    // The capture step, replaced by a stub that records its environment and stops the run
    // there. Nothing after step 1 is exercised and nothing is built.
    let envfile = tmp.path().join("child-env");
    write_script(
        &golden.join("capture_goldens.sh"),
        &format!(
            "#!/bin/bash\nexport -p > {}\nexit 1\n",
            shell_quote(&envfile)
        ),
    );

    // `--freeze` refuses unless it can name the aeon revision honestly: a real repository,
    // a resolvable HEAD, and a clean tree.
    let aeon = tmp.path().join("aeon");
    std::fs::create_dir_all(&aeon).expect("mkdir aeon");
    git(&aeon, &["init", "-q"]);
    std::fs::write(aeon.join("f"), "x").expect("write aeon file");
    git(&aeon, &["add", "f"]);
    git(&aeon, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "seed"]);

    let out = Command::new(env!("CARGO_BIN_EXE_refreeze"))
        .args(["--freeze", "gate-plumbing", "--ab", "none"])
        .current_dir(tmp.path())
        .env("SIGIL_HARNESS_ROOT", &root)
        .env("AEON_DIR", &aeon)
        .env_remove("SIGIL_GOLDEN_WRITE")
        .output()
        .expect("run refreeze");
    let text = merged(&out);

    let dump = std::fs::read_to_string(&envfile).unwrap_or_else(|e| {
        panic!("refreeze never reached the capture step ({e}); its output was:\n{text}")
    });
    let child_env = parse_export(&dump);

    // What refreeze ADDED, measured against a CONTROL child spawned from the same
    // directory with the same environment REFREEZE was given. Diffing against the test's
    // own environment instead would count the shell's bookkeeping, and the variables
    // refreeze was handed, as messages refreeze wrote.
    let control = Command::new("bash")
        .args(["-c", "export -p"])
        .current_dir(tmp.path())
        .env("SIGIL_HARNESS_ROOT", &root)
        .env("AEON_DIR", &aeon)
        .env_remove("SIGIL_GOLDEN_WRITE")
        .output()
        .expect("run the control shell");
    let control_env = parse_export(&String::from_utf8_lossy(&control.stdout));
    // `_` is rewritten by bash for every command it runs and says nothing about the caller.
    let added: BTreeMap<&String, &String> = child_env
        .iter()
        .filter(|(k, v)| k.as_str() != "_" && control_env.get(*k) != Some(*v))
        .collect();
    assert!(
        !added.is_empty(),
        "refreeze handed its capture child nothing of its own, so the write gate would \
         refuse the journalled path:\n{text}"
    );

    // The real script, under exactly that environment, must not be refused.
    let bed = Bed::plant();
    let extra: Vec<(&str, &str)> =
        added.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let run = bed.capture(&["--write"], &extra);
    let run_text = merged(&run);
    assert!(
        !run_text.contains(REFUSAL),
        "the write gate refused the environment refreeze hands its capture child, so the \
         journalled path is broken. refreeze added: {added:?}\n{run_text}"
    );
    assert!(run.status.success(), "the write refreeze would drive did not complete:\n{run_text}");
    assert_eq!(bed.fresh_set(), TARGETS, "the write refreeze would drive moved nothing");
}

fn git(dir: &Path, args: &[&str]) {
    let st = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(st.status.success(), "git {args:?} failed: {}", String::from_utf8_lossy(&st.stderr));
}

fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', r"'\''"))
}

/// Parse `export -p` output into name -> value. Only the simple `declare -x K="V"` form is
/// read; a variable whose value spans lines is skipped rather than guessed at.
fn parse_export(dump: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in dump.lines() {
        let Some(rest) = line.strip_prefix("declare -x ") else { continue };
        let Some((k, v)) = rest.split_once('=') else { continue };
        let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
        map.insert(k.to_string(), v.replace("\\\"", "\"").replace("\\\\", "\\"));
    }
    map
}
