//! A WRITE INTO THE REFERENCE TREE REFUSES UNLESS `AEON_DIR` NAMED IT.
//!
//! `test_support::aeon_dir` falls back to a hardcoded path — the owner's LIVE aeon
//! working checkout — when `AEON_DIR` is unset, and every sound emitter
//! `native::ensure_generated` drives writes INSIDE the tree it resolves. The hazard
//! is invisibility, not damage: aeon's build regenerates those artifacts
//! unconditionally, so it heals itself, but `engine/sound/generated/` is gitignored
//! there, so a foreign write leaves no trace in `git status` and nothing records
//! which process produced the bytes a later read picks up. A fallback to somebody's
//! live tree is structurally incapable of announcing its own failure. A refusal is
//! loud at the caller's own site and costs one exported variable.
//!
//! Reads keep the fallback. This gate is about the write side only.
//!
//! WHAT THE EXPECTATION IS DERIVED FROM — never a pin, never one measurement.
//!
//!   * the emitter set is PARSED OUT OF `ensure_generated`'s own body (shared with
//!     `reference_tree_write_guard` via `common`), so an eighth emitter added there
//!     and not here fails by name rather than shrinking the coverage silently;
//!   * the path the refusal must name is `test_support::LIVE_TREE_FALLBACK`, the
//!     same constant `aeon_dir` falls back to, so the two cannot drift apart;
//!   * the ORDERING — naming checked before the tree's contents are probed — is
//!     read off `seam2::SOUND_PLACEMENT_MAP_REL`: the unset-env refusal must NOT
//!     mention it (the content probe never ran), and the named-env refusal MUST
//!     (the content probe did). Reversed, an unset `AEON_DIR` would have the content
//!     probe consult the live checkout, find a complete tree, and let the write
//!     through.
//!
//! HOW IT TELLS "THE REFUSAL FIRED" FROM "NOTHING RAN" — the two are identical from
//! outside, and the second is the cheaper accident. The property needs `AEON_DIR`
//! absent from the environment, which a landing run always sets, so it is asserted
//! in a CHILD process spawned from this binary with the variable removed. The parent
//! then refuses to pass unless it can account for the child:
//!
//!   * the child must exit successfully AND its output must carry libtest's own
//!     `test result: ok.` line — a child that never started exits non-zero and
//!     prints nothing;
//!   * the child emits one `WITNESS refused` line per emitter and one
//!     `WITNESS content-refusal` line per emitter, and the parent reconciles both
//!     counts against the set parsed out of `ensure_generated`;
//!   * an emitter that reports SUCCESS is a failure, not an absence of evidence:
//!     against an empty directory nothing could have been read.
//!
//! Every count that cannot be established panics with `UNMEASURABLE`, carrying the
//! child's stdout and stderr, rather than rendering as a zero or a green.
//!
//! It needs no reference tree of its own and therefore never skips: it runs in every
//! `cargo test --workspace`, which is what `scripts/landing-run.sh` invokes.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

/// The signature every artifact emitter shares: read `aeon`, write into `out_dir`.
type Emitter = fn(&Path, &Path) -> Result<(), String>;

/// The emitters this gate drives, keyed by the name they are called under in
/// `ensure_generated`. The KEYS are reconciled against the set parsed out of that
/// function, so this table cannot fall behind it silently.
const EXERCISED: &[(&str, Emitter)] = &[
    ("emit_sound_blob", sigil_harness::seam1::emit_sound_blob),
    ("emit_dac_artifacts", sigil_harness::seam2::emit_dac_artifacts),
    ("emit_mt_artifacts", sigil_harness::seam2::emit_mt_artifacts),
    ("emit_sfx_artifacts", sigil_harness::seam2::emit_sfx_artifacts),
    ("emit_seq_opcode_artifacts", sigil_harness::seam2::emit_seq_opcode_artifacts),
    ("emit_sound_tables_artifacts", sigil_harness::seam2::emit_sound_tables_artifacts),
    ("emit_pitchtable_artifacts", sigil_harness::seam2::emit_pitchtable_artifacts),
];

/// Set in the child process to select the assertion body over the spawn.
const CHILD_VAR: &str = "SIGIL_NAMED_WRITE_CHILD";
/// The scratch tree the child aims its writes at, handed down by the parent so the
/// parent can inspect it afterwards.
const DIR_VAR: &str = "SIGIL_NAMED_WRITE_DIR";

/// A directory this process created and no concurrent run can collide with. It is a
/// stand-in reference tree: it EXISTS and is EMPTY, so a write that is not refused
/// lands in it visibly.
fn scratch_tree() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sigil-named-write-{}-{nanos}", std::process::id()))
}

fn is_empty(dir: &Path) -> bool {
    std::fs::read_dir(dir).map(|mut d| d.next().is_none()).unwrap_or(false)
}

/// A write into a tree `AEON_DIR` does not name refuses, before it probes the tree
/// and before it creates anything.
#[test]
fn a_write_into_the_reference_tree_refuses_unless_aeon_dir_named_it() {
    match std::env::var(CHILD_VAR).as_deref() {
        Ok("unset") => child_with_aeon_dir_removed(),
        Ok("named") => child_with_aeon_dir_set(),
        Ok(other) => panic!("UNMEASURABLE: unknown {CHILD_VAR} mode `{other}`"),
        Err(_) => parent(),
    }
}

/// `AEON_DIR` removed from the environment: every emitter refuses by naming the
/// variable and the fallback it would otherwise have written into, the tree is
/// untouched, and the refusal precedes the content probe.
fn child_with_aeon_dir_removed() {
    let declared = common::emitters_named_by_ensure_generated();
    let names: Vec<&str> = EXERCISED.iter().map(|(n, _)| *n).collect();
    common::reconcile_arms(&declared, &names);

    assert!(
        std::env::var_os("AEON_DIR").is_none(),
        "UNMEASURABLE: the child was started with AEON_DIR still set, so the property it exists \
         to assert was never in force."
    );

    let aeon = PathBuf::from(std::env::var(DIR_VAR).expect("parent hands the child its scratch tree"));
    let out_dir = aeon.join("engine/sound/generated");
    let fallback = sigil_harness::test_support::LIVE_TREE_FALLBACK;

    for (name, emit) in EXERCISED {
        let err = match emit(&aeon, &out_dir) {
            Err(e) => e,
            Ok(()) => panic!(
                "UNMEASURABLE: `{name}` reported success writing into {} with AEON_DIR unset and \
                 nothing in the tree to read. Nothing could have been read; the gate cannot tell \
                 what it just measured.",
                aeon.display()
            ),
        };
        assert!(
            err.contains("AEON_DIR"),
            "`{name}` refused without naming AEON_DIR, so the reader is not told what to set. \
             Error: {err}"
        );
        assert!(
            err.contains(fallback),
            "`{name}` refused without naming the fallback tree `{fallback}` its write would have \
             gone to. The refusal must say what it is protecting. Error: {err}"
        );
        assert!(
            err.contains(&aeon.display().to_string()),
            "`{name}` refused without naming the tree it was aimed at, so the reader cannot tell \
             which call site refused. Error: {err}"
        );
        assert!(
            !err.contains(sigil_harness::seam2::SOUND_PLACEMENT_MAP_REL),
            "`{name}` refused with the CONTENT error, so the naming check ran after the content \
             probe. In that order an unset AEON_DIR sends the probe to the live checkout, where \
             it finds a complete tree and passes, and the write proceeds into exactly the tree \
             the check exists to keep it out of. Error: {err}"
        );
        assert!(
            is_empty(&aeon),
            "`{name}` created something under {} while refusing. Nothing may be written before \
             the tree is named.",
            aeon.display()
        );
        println!("WITNESS refused {name}");
    }

    let panicked = std::panic::catch_unwind(|| sigil_harness::native::ensure_generated(&aeon));
    let msg = match panicked {
        Err(p) => p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
        Ok(()) => panic!(
            "UNMEASURABLE: `ensure_generated` returned normally against {} with AEON_DIR unset. \
             It writes seven artifacts; a normal return means it did not run or did not write.",
            aeon.display()
        ),
    };
    assert!(
        msg.contains("AEON_DIR"),
        "`ensure_generated` refused at its entry without naming AEON_DIR. Panic: {msg}"
    );
    assert!(
        is_empty(&aeon),
        "`ensure_generated` created something under {} while refusing.",
        aeon.display()
    );
    println!("WITNESS refused ensure_generated");
}

/// `AEON_DIR` set: the naming precondition passes and the emitters go on to the
/// content probe. Without this direction the gate could not tell a working refusal
/// from an emitter set that fails for any reason at all.
fn child_with_aeon_dir_set() {
    let declared = common::emitters_named_by_ensure_generated();
    let aeon = PathBuf::from(std::env::var(DIR_VAR).expect("parent hands the child its scratch tree"));
    let out_dir = aeon.join("engine/sound/generated");

    assert_eq!(
        std::env::var_os("AEON_DIR").map(PathBuf::from),
        Some(aeon.clone()),
        "UNMEASURABLE: the child's AEON_DIR does not name the tree under test, so a refusal here \
         would not distinguish the two checks."
    );

    for (name, emit) in EXERCISED {
        let err = match emit(&aeon, &out_dir) {
            Err(e) => e,
            Ok(()) => panic!(
                "UNMEASURABLE: `{name}` reported success against the empty tree {}. Nothing could \
                 have been read.",
                aeon.display()
            ),
        };
        assert!(
            err.contains(sigil_harness::seam2::SOUND_PLACEMENT_MAP_REL),
            "with AEON_DIR naming the tree, `{name}` must reach the CONTENT probe and refuse for \
             the absent `{}`. It refused for something else, so this gate's unset-env direction \
             proves nothing about the naming check specifically. Error: {err}",
            sigil_harness::seam2::SOUND_PLACEMENT_MAP_REL
        );
        assert!(
            is_empty(&aeon),
            "`{name}` created something under {} while refusing on content.",
            aeon.display()
        );
        println!("WITNESS content-refusal {name}");
    }
    assert_eq!(
        EXERCISED.len(),
        declared.len(),
        "UNMEASURABLE: exercised {} emitters but `ensure_generated` drives {}.",
        EXERCISED.len(),
        declared.len()
    );
}

/// Spawns both children with the environment the property needs, and refuses to
/// report green on anything it cannot account for.
fn parent() {
    let declared = common::emitters_named_by_ensure_generated();
    let names: Vec<&str> = EXERCISED.iter().map(|(n, _)| *n).collect();
    common::reconcile_arms(&declared, &names);

    // Direction 1: AEON_DIR removed. One refusal per emitter plus ensure_generated.
    let unset = scratch_tree();
    std::fs::create_dir_all(&unset).expect("create the child's scratch tree");
    let out = run_child("unset", &unset);
    let left_behind = !is_empty(&unset);
    let _ = std::fs::remove_dir_all(&unset);
    assert!(
        !left_behind,
        "the AEON_DIR-unset child left something under {}. The parent checks the tree \
         independently of the child's own assertions.\n{out}",
        unset.display()
    );
    let refusals = out.matches("WITNESS refused ").count();
    let expected = declared.len() + 1; // every emitter, plus ensure_generated's entry
    assert_eq!(
        refusals, expected,
        "UNMEASURABLE: the AEON_DIR-unset child reported {refusals} refusals; \
         `ensure_generated` drives {} emitters and refuses at its own entry, so {expected} were \
         expected. A count that does not reconcile is a run that measured something other than \
         the property.\n{out}",
        declared.len()
    );

    // Direction 2: AEON_DIR set. The naming check passes and the content probe runs,
    // so a refusal in direction 1 is attributable to the naming check alone.
    let named = scratch_tree();
    std::fs::create_dir_all(&named).expect("create the child's scratch tree");
    let out2 = run_child("named", &named);
    let _ = std::fs::remove_dir_all(&named);
    let contents = out2.matches("WITNESS content-refusal ").count();
    assert_eq!(
        contents,
        declared.len(),
        "UNMEASURABLE: the AEON_DIR-set child reported {contents} content refusals; \
         `ensure_generated` drives {}.\n{out2}",
        declared.len()
    );
}

/// Runs this test binary again in `mode`, and returns its combined output. Anything
/// that would leave the parent unable to say the child RAN is an `UNMEASURABLE`
/// panic rather than a silent zero.
fn run_child(mode: &str, dir: &Path) -> String {
    let exe = std::env::current_exe().expect("the test binary knows its own path");
    let mut cmd = Command::new(&exe);
    cmd.arg("--nocapture").arg("--test-threads=1").env(CHILD_VAR, mode).env(DIR_VAR, dir);
    if mode == "named" {
        cmd.env("AEON_DIR", dir);
    } else {
        cmd.env_remove("AEON_DIR");
    }
    let out = cmd.output().unwrap_or_else(|e| {
        panic!("UNMEASURABLE: could not run {} in `{mode}` mode: {e}", exe.display())
    });
    let text = format!(
        "--- child `{mode}` stdout ---\n{}--- child `{mode}` stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "the `{mode}` child failed ({}). Its assertions are the property; read them below.\n{text}",
        out.status
    );
    assert!(
        text.contains("test result: ok."),
        "UNMEASURABLE: the `{mode}` child exited 0 without libtest's own `test result: ok.` line, \
         so it cannot be established that it ran any test at all. A zero from a run that did not \
         happen looks exactly like a zero from a run that did.\n{text}"
    );
    text
}
