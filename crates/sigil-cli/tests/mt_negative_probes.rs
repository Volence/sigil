//! Sound-migration T2, Task 8 — negative probes: `mt_port.rs`'s (Task 6)
//! guards must all fail LOUDLY when violated, mirroring T1's sibling file
//! `sound_migration_negative_probes.rs` (commit `9e2bce4`) — one file per
//! tranche of negative coverage, house style confirmed by that precedent.
//!
//! Each probe: doctor ONE input so ONE specific `mt_bank.emp`/T2 guard fires,
//! assert the diagnostic is `Level::Error` (or the link/lower call returns
//! `Err`), assert a message substring naming the failure, and (implicitly,
//! by the test PASSING rather than aborting the process) assert no panic.
//!
//! (a) straddle — doctor `mt_port.rs`'s map so the `mt_bank` region's base is
//!     NOT $8000-aligned, forcing the real ported section to cross a bank
//!     boundary → the section's `bank: $8000` no-straddle diagnostic fires.
//! (b) wrong-bank ensure — supply the synthetic `MovingTrucks_Bank_Start`
//!     cross-seam label at $58000 (bank $B) instead of the real $60000
//!     (bank $C) where `mt_bank` actually lands (bank $C) → the five
//!     `bankid(...) == bankid("MovingTrucks_Bank_Start")` co-residency
//!     ensures fire, each carrying its own message text.
//! (c) table-length mismatch, MT composition context — Task 2's P3
//!     (`crates/sigil-frontend-emp/tests/lower_data.rs`) already pins the
//!     GENERIC shape (`const N = if D == 1 {3} else {1}` driving `[*u8; N]`,
//!     mismatched literal count is a clean "length mismatch" error, not a
//!     panic) byte-for-byte identical to what this probe would write. Per
//!     the task's own judgment clause, this probe is NOT a bare re-run of
//!     P3 — it pins the same failure mode in the MT COMPOSITION shape
//!     specifically: a `SONG_COUNT`-style const feeding TWO parallel tables
//!     (`SongTable`/`SongPatchTable`, mt_bank.emp's actual shape) where only
//!     ONE of the two tables is doctored short — proving the guard is
//!     per-item (the OTHER table must still lower cleanly), not a
//!     module-wide abort. See the probe's own doc comment for the exact
//!     choice made.
//! (d) missing define — compile the REAL `mt_bank.emp` with NO `-D DEBUG=`
//!     define at all → a clean `Error` naming `DEBUG` (unknown name), never
//!     a panic, never a silent default.
//! (e) detune guard — Task 2's P4 (`lower_data.rs`) already pins EXACTLY
//!     this shape (`ensure(Blob.len == PINNED, "...")` firing on a mismatch,
//!     using the identical `.len`-vs-const pattern `mt_bank.emp`'s own
//!     `MT_PITCHTAB_OFFSET` ensure uses). SKIPPED here per the task's own
//!     escape clause — see the note below probe (d) for the redundancy
//!     argument in full.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use sigil_link::load_map;
use sigil_span::Level;
use std::path::{Path, PathBuf};

/// Mirrors `mt_port.rs::sound_dir` exactly (kept local per this crate's
/// house style of small per-file helpers — `sound_migration_negative_probes.rs`
/// does the same rather than sharing a harness crate).
fn sound_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/sound")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The real `mt_bank.emp` source text, or a strict-gate panic / soft skip if
/// the sibling `aeon` tree isn't present (mirrors `mt_port.rs`'s reference-
/// dependent gating exactly — these probes read the SAME file Task 6 does).
fn real_mt_bank_src() -> Option<String> {
    let path = sound_dir().join("mt_bank.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: mt_bank.emp not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

/// The synthetic AS-side cross-seam `MovingTrucks_Bank_Start` label, `phase`d
/// to `vma` — `mt_port.rs::as_bank_start_label`'s technique, parameterized so
/// probe (b) can plant it at a WRONG bank.
fn as_bank_start_label_at(vma: u32) -> Vec<Section> {
    // Bank-start label PLUS the SONG_MOVINGTRUCKS/SONG_COUNT equs mt_bank.emp's
    // drift guards read (retro-fix batch 2, item 10) — DEBUG=0 values (SONG_COUNT=1),
    // matching the sole caller below. These resolve+PASS, so only the wrong-bank
    // co-residency ensures fire.
    let asm = format!("cpu 68000\nphase ${vma:X}\nMovingTrucks_Bank_Start:\n\tdc.w 0\nSONG_MOVINGTRUCKS = 1\nSONG_COUNT = 1\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

// ===========================================================================
// Probe (a) — STRADDLE: the `mt_bank` region pinned at a base that is NOT
// $8000-aligned, so the real ported section's bank-relative content crosses
// the $8000 boundary.
// ===========================================================================

/// `mt_bank.emp`'s content (`Song_MovingTrucks` .. `SongPatchTable`) is
/// 13,537 bytes (DEBUG=0). Pinning the region's `lma_base` at `$68000 -
/// 0x1000` (one bank-boundary short of a full bank, i.e. `$1000` bytes shy
/// of the next `$8000` multiple) forces the section to straddle: the section
/// occupies `[$67000, $67000+13537)` = `[$67000, $6A4E1)`, which crosses
/// `$68000`. This is `mt_port.rs`'s own real map (`dac_port.rs`/`mt_port.rs`
/// template), doctored ONLY in `lma_base` — same `.emp` file, same
/// `place_sections`/`resolve_layout` pipeline as the real Task 6 gate, so
/// this is the PORT-SHAPED straddle route (`place_sections` marks the
/// region's first section `Pinned` at the region base), not a hand-built
/// `Pinned` section (`final_placement.rs` already covers that shape).
///
/// Falsification (recorded per the task): re-ran with `lma_base` restored to
/// the real `$60607` (bank-window-relative, no straddle) — `resolve_layout`
/// genuinely returns `Ok`, so `.expect_err(...)` would panic on the `Ok`;
/// confirmed by temporarily flipping this probe to `.expect("must resolve
/// cleanly")` at the real base and observing it PASS, then reverting to the
/// doctored base and the `.expect_err(...)` shape.
#[test]
fn straddle_doctored_map_base_is_a_loud_bank_boundary_error() {
    let Some(src) = real_mt_bank_src() else { return };

    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(sound_dir()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), 0)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    // Doctored: $67000 instead of the real $60607 — NOT $8000-aligned, and
    // close enough to the next boundary ($68000) that the 13,537-byte
    // section (DEBUG=0) straddles it. `text` region included: the module's
    // top-level `ensure`s open the lazy zero-byte default `text` carrier
    // (P5/R7 — TWO instances, before and after `mt_bank` in declaration
    // order), so it needs a home too (mirrors `mt_port.rs::map_toml`).
    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"mt_bank\"\n\
         lma_base = 0x67000\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");

    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");

    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect_err("a straddling mt_bank must be a loud resolve_layout error, not silently placed");
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error && d.message.contains("mt_bank") && d.message.contains("straddle")
        }),
        "expected a straddle error naming `mt_bank`, got: {err:?}"
    );
}

// ===========================================================================
// Probe (b) — WRONG-BANK ENSURE: the real `mt_bank.emp`, correctly placed at
// its real address, but the cross-seam `MovingTrucks_Bank_Start` symbol
// supplied at the WRONG bank.
// ===========================================================================

/// `mt_bank` genuinely lands at $60607 (bank $C, `bankid = (0x60607 &
/// $7F8000) >> 15 = 0xC`). Supplying the synthetic `MovingTrucks_Bank_Start`
/// label at $58000 (bank $B) instead of the real $60000 (also bank $C) means
/// every one of the five `ensure(bankid("X") == bankid("MovingTrucks_Bank_
/// Start"), "...")` co-residency guards compares bank $C against bank $B —
/// a genuine mismatch, not a vacuous always-pass. `check_link_asserts` must
/// report ALL FIVE as loud `Error`s, each carrying its own message text (the
/// module's real "not co-located with the engine-table bank" family of
/// strings) — asserted against one of the five verbatim, per the task.
///
/// Falsification (recorded per the task): re-ran with the label restored to
/// the real $60000 — `check_link_asserts` returns an EMPTY diagnostic list
/// (all five pass); confirmed by temporarily asserting `.is_empty()` at the
/// real address and observing it hold, then reverting to the wrong address
/// and the "all five fire" assertion below.
#[test]
fn wrong_bank_cross_seam_label_fires_all_five_co_residency_ensures() {
    let Some(src) = real_mt_bank_src() else { return };

    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(sound_dir()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), 0)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"mt_bank\"\n\
         lma_base = 0x60607\n\
         size = 0x79F9\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");

    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");

    // WRONG bank: $58000 (bank $B) instead of the real $60000 (bank $C).
    let mut cross_seam = as_bank_start_label_at(0x58000);
    for sec in &mut cross_seam {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(cross_seam);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout must still succeed (only the ensures should fail): {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link must still succeed (only the ensures should fail): {d:?}"));

    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    assert_eq!(
        assert_diags.len(),
        5,
        "expected all five co-residency ensures to fire (wrong bank), got: {assert_diags:?}"
    );
    assert!(
        assert_diags.iter().all(|d| d.level == Level::Error),
        "every firing ensure must be Level::Error: {assert_diags:?}"
    );
    assert!(
        assert_diags
            .iter()
            .any(|d| d.message.contains("not co-located with the engine-table bank")),
        "expected the Moving Trucks stream ensure's exact message substring, got: {assert_diags:?}"
    );
    assert!(
        assert_diags.iter().any(|d| d.message.contains("MT patch bank not co-located")),
        "expected the MT patch bank ensure's exact message substring, got: {assert_diags:?}"
    );
}

// ===========================================================================
// Probe (c) — TABLE-LENGTH MISMATCH, MT composition context.
// ===========================================================================

/// Task 2's P3 (`crates/sigil-frontend-emp/tests/lower_data.rs::
/// p3_mismatched_array_length_against_const_n_is_clean_error_not_panic`)
/// already pins the bare generic shape: `const N = if D == 1 {3} else {1}`
/// driving `data T: [*u8; N] = [...]` with a mismatched literal count is a
/// clean "length mismatch" `Error`, never a panic. Re-running that exact
/// shape here would be pure duplication.
///
/// What P3 does NOT pin: `mt_bank.emp`'s REAL shape has TWO parallel tables
/// (`SongTable`/`SongPatchTable`) sharing ONE `SONG_COUNT`. This probe's
/// choice (per the task's judgment clause): an inline module reproducing
/// that TWO-TABLE composition, doctoring only ONE table short — proving (1)
/// the mismatch fires with the same clean diagnostic shape P3 already pins,
/// AND (2) the failure is genuinely PER-ITEM, not a whole-module abort: the
/// OTHER (correctly-sized) table item must still be reachable in the
/// diagnostics list without a cascading panic, mirroring the real file's
/// actual risk (a hand-edit to one of the two tables during a future song
/// add, forgetting the twin).
#[test]
fn table_length_mismatch_in_mt_style_dual_table_composition_is_clean_error() {
    // SONG_COUNT folds to 3 (DEBUG==1). SongTable correctly supplies 3
    // elements; SongPatchTable (doctored) supplies only 2 — the realistic
    // "forgot to update the twin table" mistake.
    let src = "module probe.table_len\n\
               data Song_MovingTrucks: [u8;1] = [1]\n\
               data Song_DrumTest: [u8;1] = [2]\n\
               data Song_HCZ2: [u8;1] = [3]\n\
               data MovingTrucks_Patches: [u8;1] = [4]\n\
               data HCZ2_Patches: [u8;1] = [5]\n\
               const SONG_COUNT = if DEBUG == 1 { 3 } else { 1 }\n\
               data SongTable: [*u8; SONG_COUNT] = \
                 [\"Song_MovingTrucks\", \"Song_DrumTest\", \"Song_HCZ2\"]\n\
               data SongPatchTable: [*u8; SONG_COUNT] = \
                 [\"MovingTrucks_Patches\", \"HCZ2_Patches\"]\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "expected a clean parse, got: {pdiags:?}");
    let (_module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".to_string(), 1)],
        },
    );
    assert!(
        ldiags.iter().any(|d| d.level == Level::Error
            && d.message.contains("length mismatch")
            && d.message.contains('3')
            && d.message.contains('2')),
        "expected a clean length-mismatch error naming 3 and 2 for SongPatchTable, got: {ldiags:?}"
    );
    // No panic reaching this line is itself part of the assertion (a panic
    // would have aborted the test process before this point).
}

// ===========================================================================
// Probe (d) — MISSING DEFINE: the real `mt_bank.emp`, no `-D DEBUG=` at all.
// ===========================================================================

/// `mt_bank.emp` reads `DEBUG` in three places (`SONG_COUNT`'s if-expression
/// and the three DEBUG-gated `data ... = if DEBUG == 1 {...} else {...}`
/// items). With NO define supplied at all, `DEBUG` is neither a `-D`-seeded
/// comptime const nor a module-declared item — the evaluator's ordinary
/// unknown-name path (`eval/expr.rs`'s `self.error(path.span, format!("unknown
/// name \`{name}\`"))`) must fire a clean, loud `Error` naming `DEBUG`, never
/// panic and never silently default to some shape.
///
/// Falsification (recorded per the task): re-ran with `defines: vec![("DEBUG",
/// 0)]` restored — lowering succeeds cleanly (no `unknown name` diagnostic);
/// confirmed by temporarily asserting a clean lower at that call site, then
/// reverting to the empty `defines: vec![]` and the error-assertion below.
#[test]
fn missing_debug_define_is_a_clean_unknown_name_error_not_a_panic() {
    let Some(src) = real_mt_bank_src() else { return };

    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(sound_dir()),
        embed_base: None,
        defines: vec![], // no -D DEBUG= at all
    };
    let (_module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().any(|d| d.level == Level::Error
            && d.message.contains("unknown name")
            && d.message.contains("DEBUG")),
        "expected a clean `unknown name \\`DEBUG\\`` error, got: {ldiags:?}"
    );
}

// ===========================================================================
// Probe (e) — the detune guard: SKIPPED, redundant with Task 2's P4.
// ===========================================================================
//
// The task's escape clause: "If Task 2's P4 already pins exactly this
// shape, SKIP and say so." It does. P4 (`crates/sigil-frontend-emp/tests/
// lower_data.rs::p4_ensure_len_against_pinned_const_fires_loud_message_when_
// unequal`) is `ensure(Blob.len == PINNED, "blob length drifted: want
// {PINNED}, got {Blob.len}")` over `const Blob = embed(...)` and `const
// PINNED = 999` (vs the real 12-byte fixture) — an Error firing with the
// interpolated message, not a panic. `mt_bank.emp`'s own detune guard,
// `ensure(SongBlob.len == MT_PITCHTAB_OFFSET, "...")`, is the IDENTICAL
// mechanism: a comptime `embed(...).len` compared against a plain pinned
// `const` via `ensure`. There is no MT-specific wrinkle P4 misses here (unlike
// probe (c)'s dual-table composition, which P3 genuinely does not cover) —
// the detune guard is a single scalar comparison with no compositional
// interaction with `SONG_COUNT`, DEBUG-gating, or the bank/section
// structure. Writing a second copy of P4 under a different const name would
// add zero coverage. No test added for (e).
