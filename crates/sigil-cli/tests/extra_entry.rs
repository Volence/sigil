//! `sigil build --extra-entry <module>` — THE NEGATIVE-BUILD LANE'S ENTRY POINT.
//!
//! A module-level `ensure` is evaluated iff its module is in the build's `use`
//! closure, so a guard written in a module nothing imports cannot fail — which is
//! exactly the position aeon's poison modules (`games/sonic4/test/poison/*.emp`) are
//! in. `--extra-entry` adds one `use` edge from the synthetic entry, evaluating the
//! named module INSIDE the real build profile: the same manifest rewrites (helper
//! publication + glob normalization), the same comptime `-D` set. Its guards then
//! run, and a false one fails the build with its own message.
//!
//! THE CONTRACT THE AEON LANE DEPENDS ON (`tools/emp_expect_fail.py`), asserted here:
//!   - nonzero exit,
//!   - the guard's message on the normal `[Error]` surface,
//!   - one `[Error]` per firing guard and no plumbing-minted extras,
//!   - and ZERO effect on emitted bytes when the module's guards hold.
//!
//! The poisons are written in the AMBIENT AUTHORING SPELLING — `module` line first,
//! no imports, helper vocabulary glob-injected by the build — and this gate takes
//! them unchanged. That is the point: if the flag needed a poison rewritten, the flag
//! would be describing something other than what an author's module does.
//!
//! Reference tree: defaults to the sibling aeon checkout (override with `AEON_DIR`);
//! under `SIGIL_STRICT_GATE` a missing tree HARD-FAILS, the house pattern.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon \
//!   cargo test --release -p sigil-cli --test extra_entry
//! ```

use sigil_harness::native;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A poison whose guard is the band walk's per-CRAM-entry nesting refusal, named by
/// PATH (the on-disk spelling a lane or an author reaches for) — with the fragment
/// aeon's `CASES` table pins for it.
///
/// Chosen against three properties, none of which is "it fails", because a fixture
/// picked only for failing is the vacuous shape this file exists to avoid:
///
/// 1. **Exactly one error.** Two of the cases below assert `[Error]` count == 1, so a
///    COUNT-2 poison would redden them for a reason unrelated to the flag. That rules
///    out `poison_band_orphan_restore` / `poison_band_span_mismatch` / `poison_band_h2_sh`.
/// 2. **A fragment unique to ONE guard.** `"carries no band id"` appears in three
///    separate poisons, so it cannot witness which rule fired — the matcher-collision
///    the poison bar names. This fragment appears in exactly two files: this poison and
///    the guard that emits it (`engine/effects/raster_dsl.emp`).
/// 3. **Load-bearing for the OTHER lane.** aeon's own header calls this "the fixture
///    that matters most to P2a" — a two-band program that must still be refused while
///    two others now build. A fixture its owner depends on is the one least likely to
///    be quietly re-aimed.
const POISON_NESTED_PATH: &str = "games/sonic4/test/poison/poison_band_nested.emp";
const POISON_NESTED_FRAGMENT: &str = "two bands are live on CRAM entry";

/// A second poison, named by dotted MODULE ID (the other accepted spelling), whose
/// guard is the direct-`SetReg($8Axx)` refusal.
const POISON_DIRECT_8A_ID: &str = "games.sonic4.test.poison.poison_direct_8a";
const POISON_DIRECT_8A_FRAGMENT: &str = "detonates the relative-arm chain";

/// Names that must resolve to NOTHING, one in each accepted spelling. Constants rather
/// than literals so `every_aeon_fixture_this_file_names_still_resolves` can assert their
/// ABSENCE against the same strings this file's cases pass in — a second copy could drift
/// from the first and the drift would be invisible.
const MISSING_EXTRA_ENTRY_ID: &str = "games.sonic4.test.poison.no_such_poison";
const MISSING_EXTRA_ENTRY_PATH: &str = "games/sonic4/test/poison/gone.emp";

/// A PURE-COMPTIME module (consts + `ensure`s, no `data`/`proc`/`section`) that is
/// outside the sonic4 profile's `use` closure and whose guards HOLD. The passing
/// half of the flag's contract: bringing it in must move no byte.
const PASSING_EXTRA_ENTRY: &str = "games.demo.constants";

/// Modules that CONTRIBUTE to the artifact, with the declaration kind the refusal
/// must name: `engine.math` emits code, `engine.ram` declares the RAM regions. Both
/// directions of the byte-neutrality contract, since a RAM allocator moves every
/// address after it and so changes emitted operands without emitting a byte itself.
/// (The per-kind verdicts, region-form `vars` included, are pinned tree-free in
/// `native.rs`'s `extra_entry_tests`.)
const CONTRIBUTING_EXTRA_ENTRIES: &[(&str, &str)] =
    &[("engine.math", "`proc`"), ("engine.ram", "`region`")];

fn aeon_dir() -> Option<PathBuf> {
    let aeon = sigil_harness::test_support::aeon_dir();
    if !aeon.exists() {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference tree missing: {}", aeon.display());
        }
        eprintln!("skip: aeon tree not at {} (set AEON_DIR)", aeon.display());
        return None;
    }
    Some(aeon)
}

/// EVERY aeon path this file names still resolves — and the one that must NOT resolve
/// still does not.
///
/// This is a source-DRIFT check, and it exists because of a measured two-parcel latency.
/// A zero-byte aeon parcel renamed `poison_two_restores.emp` out from under three cases
/// here; nothing noticed, because the only thing that reads these paths is a full
/// build, and a full build only runs when a freeze moves bytes. The break surfaced two
/// parcels later inside an unrelated byte-mover's attestation, where it read as that
/// parcel's fault and was not.
///
/// This case costs no build: it is a `Path::exists` sweep. It cannot tell you a fixture
/// still fires the guard it used to — only the cases above do that — but it turns a
/// rename from "red inside someone else's landing, attributed wrongly" into "red here,
/// naming the file, the morning after".
///
/// BOTH DIRECTIONS, because a one-sided version decays. The absent-by-design argument
/// to `a_missing_extra_entry_errors_loudly` is asserted ABSENT: if someone ever creates
/// `poison/gone.emp`, that case silently stops testing the unresolvable-name path while
/// continuing to pass, which is the exact vacuity this file is built to refuse.
// The two loops below each walk a ONE-ELEMENT array today, and that is deliberate: each
// array is this file's declared SET of aeon fixtures in that direction, present and
// absent. Collapsing either to a `let` would turn the next fixture added from one more
// row into a restructure of the assertion around it, and the header above is explicitly
// about keeping both directions as sets.
#[allow(clippy::single_element_loop)]
#[test]
fn every_aeon_fixture_this_file_names_still_resolves() {
    let Some(aeon) = aeon_dir() else { return };

    for path in [POISON_NESTED_PATH] {
        assert!(
            aeon.join(path).is_file(),
            "`{path}` is named by this file but does not exist under {}. An aeon parcel \
             has renamed, moved or deleted it. Do NOT simply re-point at the new name: \
             check the replacement's own header for its EXPECTED FRAGMENT and its error \
             COUNT first, because a renamed poison is often also a re-aimed one.",
            aeon.display()
        );
    }

    for absent in [MISSING_EXTRA_ENTRY_PATH] {
        assert!(
            !aeon.join(absent).exists(),
            "`{absent}` is asserted ABSENT by this file — `a_missing_extra_entry_errors_loudly` \
             uses it to prove an unresolvable name is a loud error. It now EXISTS under {}, so \
             that case has stopped testing what it claims while still passing. Pick another \
             name that does not exist.",
            aeon.display()
        );
    }
}

/// The native builds touch the shared `engine/sound/generated` dir — serialize.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The canonical target the aeon lane builds.
const SONIC4: &[&str] = &["--game", "sonic4"];

/// Run `sigil build` for `target` over the reference tree with `extra`
/// `--extra-entry` arguments, discarding the ROM. Returns the process result and its
/// combined output — the aeon lane reads `stdout + stderr` as one stream and counts
/// `[Error]` over it, so this gate must too.
fn build_target_with(aeon: &PathBuf, target: &[&str], extra: &[&str]) -> (Output, String) {
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let out_bin = std::env::temp_dir().join(format!("sigil_extra_entry_{}.bin", std::process::id()));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
    cmd.args(["build", "--aeon"]).arg(aeon).args(["--native"]).args(target).arg("-o").arg(&out_bin);
    for e in extra {
        cmd.args(["--extra-entry", e]);
    }
    let out = cmd.output().expect("run sigil build");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&out_bin);
    (out, text)
}

/// The canonical-target shorthand every case below the config-b one uses.
fn build_with(aeon: &PathBuf, extra: &[&str]) -> (Output, String) {
    build_target_with(aeon, SONIC4, extra)
}

/// A FAILING extra entry fails the build with its own message, on the normal
/// `[Error]` surface, with a nonzero exit — the three things the aeon lane checks.
///
/// NOT VACUOUS: the same build WITHOUT the flag succeeds, so the failure is the
/// flag's doing and not a broken tree. That control is what distinguishes "the poison
/// was evaluated" from "this build was going to fail anyway".
#[test]
fn a_failing_extra_entry_fails_the_build_with_its_message() {
    let Some(aeon) = aeon_dir() else { return };

    let (clean, clean_text) = build_with(&aeon, &[]);
    assert!(
        clean.status.success(),
        "the reference tree does not build clean without the flag, so nothing below \
         means anything:\n{clean_text}"
    );

    let (out, text) = build_with(&aeon, &[POISON_NESTED_PATH]);
    assert!(!out.status.success(), "expected a nonzero exit; output:\n{text}");
    assert!(
        text.contains(POISON_NESTED_FRAGMENT),
        "the guard's own message must reach the report; output:\n{text}"
    );
    assert_eq!(
        text.matches("[Error]").count(),
        1,
        "one firing guard must render exactly one `[Error]` — an extra one means the \
         plumbing minted a diagnostic of its own; output:\n{text}"
    );
}

/// The flag reaches a NON-CANONICAL target too. `--extra-entry` rides the profile, so
/// every target honours it through one spelling — but canonical sonic4 is the only one
/// whose driver the flag re-routes, and it is the only one the cases around this test
/// exercise. `--config-b` (a different profile, a different `-D` set, sound off) holds
/// the other side: the guard still runs and still fails the build.
#[test]
fn an_off_canonical_target_honours_the_flag() {
    let Some(aeon) = aeon_dir() else { return };
    let (out, text) = build_target_with(&aeon, &["--config-b"], &[POISON_NESTED_PATH]);
    assert!(!out.status.success(), "expected a nonzero exit; output:\n{text}");
    assert!(
        text.contains(POISON_NESTED_FRAGMENT),
        "the guard's own message must reach the report; output:\n{text}"
    );
    assert_eq!(text.matches("[Error]").count(), 1, "output:\n{text}");
}

/// Two `--extra-entry` flags COMPOSE: both modules are evaluated, both guards
/// fire, and each contributes exactly one `[Error]`. Also proves both accepted
/// spellings work in one invocation — a path and a dotted module id.
#[test]
fn two_extra_entries_compose() {
    let Some(aeon) = aeon_dir() else { return };
    let (out, text) = build_with(&aeon, &[POISON_NESTED_PATH, POISON_DIRECT_8A_ID]);
    assert!(!out.status.success(), "expected a nonzero exit; output:\n{text}");
    assert!(text.contains(POISON_NESTED_FRAGMENT), "output:\n{text}");
    assert!(text.contains(POISON_DIRECT_8A_FRAGMENT), "output:\n{text}");
    assert_eq!(
        text.matches("[Error]").count(),
        2,
        "two firing guards, two `[Error]` rows; output:\n{text}"
    );
}

/// A name that resolves to NOTHING is a loud error, never a silent skip: a lane
/// whose subject was renamed, moved or deleted must fail rather than pass vacuously.
/// The message must name the argument, which is the whole difference between "fix
/// this string" and "something is wrong somewhere".
#[test]
fn a_missing_extra_entry_errors_loudly() {
    let Some(aeon) = aeon_dir() else { return };

    for arg in [MISSING_EXTRA_ENTRY_ID, MISSING_EXTRA_ENTRY_PATH] {
        let (out, text) = build_with(&aeon, &[arg]);
        assert!(!out.status.success(), "`{arg}`: expected a nonzero exit; output:\n{text}");
        assert!(text.contains(arg), "`{arg}`: the error must name the argument; output:\n{text}");
        assert!(
            text.contains("no such module under the scan root"),
            "`{arg}`: expected the unresolvable-name error; output:\n{text}"
        );
    }
}

/// An extra entry that would CONTRIBUTE to the artifact is REFUSED by name.
/// `--extra-entry` runs comptime guards and is byte-neutral by contract; silently
/// packing a smuggled module's bytes into a shipping region — or chaining its `vars`
/// onto the RAM map and sliding every address after it — is the one outcome that must
/// not be possible. The refusal names the module AND the disqualifying declaration,
/// so the reader knows why their module does not qualify.
#[test]
fn a_contributing_extra_entry_is_refused() {
    let Some(aeon) = aeon_dir() else { return };
    for (module, kind) in CONTRIBUTING_EXTRA_ENTRIES {
        let (out, text) = build_with(&aeon, &[module]);
        assert!(!out.status.success(), "`{module}`: expected a nonzero exit; output:\n{text}");
        assert!(text.contains(module), "`{module}`: output:\n{text}");
        assert!(
            text.contains("byte-neutral by contract"),
            "`{module}`: expected the byte-neutrality refusal; output:\n{text}"
        );
        assert!(
            text.contains(kind),
            "`{module}`: the refusal must name the disqualifying declaration {kind}; \
             output:\n{text}"
        );
    }
}

/// The CLI's own extras path writes the SAME ROM.
///
/// Canonical sonic4 has two drivers — its own entry point when no extra entry is
/// given, and the declared-order chainer (which that entry point delegates to for a
/// frozen size source) when one has to reach `build_emp`. This is the gate on that
/// fork: the two must be byte-identical, or `--extra-entry` would silently mean "and
/// also build it differently".
#[test]
fn the_cli_writes_the_same_rom_with_a_passing_extra_entry() {
    let Some(aeon) = aeon_dir() else { return };

    let dir = std::env::temp_dir().join(format!("sigil_extra_entry_fork_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let run = |name: &str, extra: &[&str]| -> Vec<u8> {
        let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let out_bin = dir.join(name);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sigil"));
        cmd.args(["build", "--aeon"])
            .arg(&aeon)
            .args(["--native", "--game", "sonic4", "-o"])
            .arg(&out_bin);
        for e in extra {
            cmd.args(["--extra-entry", e]);
        }
        let out = cmd.output().expect("run sigil build");
        assert!(
            out.status.success(),
            "`{name}` build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read(&out_bin).expect("read the written ROM")
    };

    let flagless = run("flagless.bin", &[]);
    let with_extra = run("with_extra.bin", &[PASSING_EXTRA_ENTRY]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        with_extra.len(),
        flagless.len(),
        "the two sonic4 drivers wrote different-length ROMs"
    );
    assert_eq!(
        with_extra.iter().zip(&flagless).position(|(a, b)| a != b),
        None,
        "`--extra-entry` routed sonic4 through a driver that emits different bytes"
    );
}

/// A PASSING extra entry moves NO BYTE: the full file is identical to the
/// flagless build's, and its CRC/size still equal the frozen golden's.
///
/// NOT VACUOUS — the reachability half is proven in the same measurement: without
/// the flag the module is reported `[module.unreachable]` (its guards never
/// evaluate), with the flag that row is GONE. So this asserts "the module really was
/// pulled into the closure and still cost nothing", not "an argument was ignored".
#[test]
fn a_passing_extra_entry_moves_no_bytes() {
    let Some(aeon) = aeon_dir() else { return };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let base = native::sonic4_profile(false);
    let with = native::sonic4_profile(false).with_extra_entries([PASSING_EXTRA_ENTRY]);

    let unreachable = |p: &native::GameProfile| -> bool {
        native::build_emp(&aeon, p)
            .unwrap_or_else(|e| panic!("build_emp: {e}"))
            .warnings
            .iter()
            .any(|w| w.id == "module.unreachable" && w.message.contains(PASSING_EXTRA_ENTRY))
    };
    assert!(
        unreachable(&base),
        "`{PASSING_EXTRA_ENTRY}` is already inside the sonic4 closure, so this gate \
         proves nothing about the flag — pick a module the profile does not reach."
    );
    assert!(
        !unreachable(&with),
        "`--extra-entry {PASSING_EXTRA_ENTRY}` did not bring the module into the \
         closure, so its guards still never ran"
    );

    let base_rom =
        native::build_full_file_chained(&aeon, &base).unwrap_or_else(|e| panic!("base: {e}"));
    let with_rom =
        native::build_full_file_chained(&aeon, &with).unwrap_or_else(|e| panic!("with extra: {e}"));
    assert_eq!(
        with_rom.len(),
        base_rom.len(),
        "`--extra-entry` changed the full-file LENGTH — the plumbing leaked into emission"
    );
    let diff = base_rom.iter().zip(&with_rom).position(|(a, b)| a != b);
    assert_eq!(
        diff, None,
        "`--extra-entry` changed an emitted byte (first at {diff:?}) — it must be \
         byte-neutral"
    );

    // And the shared bytes are still the FROZEN golden's, so "identical to the other
    // build" cannot be two identically-wrong builds.
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sigil-harness/golden");
    let tip = sigil_harness::provenance::tip_target(&golden, "s4")
        .unwrap_or_else(|e| panic!("provenance tip: {e}"));
    let want_crc = sigil_harness::provenance::hex_u32(&tip.full_crc).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(with_rom.len(), tip.full_size, "full-file size vs the frozen golden");
    assert_eq!(native::crc32(&with_rom), want_crc, "full-file CRC vs the frozen golden");
}
