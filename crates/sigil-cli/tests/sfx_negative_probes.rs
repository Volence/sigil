//! Sound-migration T3, Task 6 — negative probes: `sfx_port.rs`'s (Task 4) and
//! Task 5's guards must all fail LOUDLY when violated, mirroring T2's sibling
//! file `mt_negative_probes.rs` — one file per tranche of negative coverage,
//! house style confirmed by that precedent.
//!
//! Each probe: doctor ONE input so ONE specific `sfx_bank.emp`/T3 guard fires,
//! assert the diagnostic is `Level::Error` (or the link/lower/resolve call
//! returns `Err`), assert a message substring naming the failure, and
//! (implicitly, by the test PASSING rather than aborting the process) assert
//! no panic.
//!
//! ## Keep-copies convention (per `mt_negative_probes.rs` :52-54)
//!
//! Each probe is self-contained: the small per-file helpers (`sound_dir`,
//! `sfx_dir`, `strict_gate`, `real_sfx_bank_src`, `as_bank_start_label_at`)
//! are kept LOCAL rather than shared through a harness crate — the
//! per-file-self-contained gate convention is explicit house style
//! (`mt_negative_probes.rs` documents it; `sfx_port.rs`'s Task-4 review
//! ratified the same duplication ruling). The truly-shared surface is a
//! handful of lines; hoisting would trade the gates' read-in-one-file
//! credibility for indirection.
//!
//! (a) straddle — doctor the map so the `sfx_bank` region's base is `0x67C00`;
//!     the real 1864-byte ported section then ends at `0x68348`, crossing the
//!     `$68000` bank boundary → the section's `bank: $8000` no-straddle
//!     diagnostic fires (`resolve_layout` errors naming `sfx_bank` +
//!     "straddle").
//! (b) wrong-bank ensure — supply the synthetic `MovingTrucks_Bank_Start`
//!     cross-seam label at `0x58000` (bank $B) instead of the real `0x60000`
//!     (bank $C, where the engine table actually lives) → resolve/link succeed
//!     but the ONE `bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start")`
//!     co-residency ensure fires as a `Level::Error`; `len() == 1` and the
//!     "not co-located with the engine-table bank" message substring pinned.
//! (c) table-length mismatch — a doctored INLINE module reproducing the
//!     `SfxTable` shape (`[*u8; SFX_TABLE_LEN]` with `const SFX_TABLE_LEN =
//!     135`) but supplying only 134 elements → a clean "array length mismatch"
//!     `Error`, never a panic.
//! (d) wrong-sym-for-null — a JUDGED probe (T2 probe-(e) precedent). WRITTEN.
//!     See the note above probe (d) for the coverage argument: it adds a
//!     LEGAL-lowering-different-bytes demonstration distinct from Task 4's
//!     falsification (which corrupted the EXPECTATION, not the input) — it
//!     pins that a wrong sym where a `0` null belongs LOWERS CLEANLY (no
//!     spurious diagnostic to mask the drift) yet emits DIFFERENT bytes, so
//!     the Task-4 byte gate is the sole guard against that transcription
//!     class.
//! (e) grown-mt overlap — compose the REAL `mt_bank.emp` (DEBUG=0, at its real
//!     region base `0x60607`) AND the REAL `sfx_bank.emp` (plain, at `0x63AE8`)
//!     in ONE `resolve_layout`, then doctor the composition so the `mt_bank`
//!     section's placed extent GROWS past the sfx base → the two-section
//!     overlap diagnostic (`overlap_diag`, relax.rs R7p.4) fires LOUD, naming
//!     BOTH sections + both hex extents. Pins that the map's benign
//!     region-level overlap can never become silent section interleaving.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Fragment, Section, SectionPlacement, SymbolTable};
use sigil_link::load_map;
use sigil_span::{Level, SourceId, Span};
use std::path::{Path, PathBuf};

/// The aeon `games/sonic4/data/sound` dir (honors `AEON_DIR`) — the include
/// root for `mt_bank.emp` (probe (e)). Mirrors `mt_negative_probes.rs`.
fn sound_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/sound")
}

/// The `sfx_bank.emp` module dir (`sound/sfx`) — its own `include_root`, under
/// which the eighteen `embed("sfx_*.bin")` fixtures resolve (`sfx_port.rs`).
fn sfx_dir() -> PathBuf {
    sound_dir().join("sfx")
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The real `sfx_bank.emp` source text, or a strict-gate panic / soft skip if
/// the sibling `aeon` tree isn't present (mirrors `sfx_port.rs`'s reference-
/// dependent gating exactly — these probes read the SAME file Task 4 does).
fn real_sfx_bank_src() -> Option<String> {
    let path = sfx_dir().join("sfx_bank.emp");
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(_) if strict_gate() => panic!("SIGIL_STRICT_GATE set but missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: sfx_bank.emp not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

/// The real `mt_bank.emp` source text (probe (e) composes it with sfx_bank).
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
/// to `vma` — `sfx_port.rs::as_bank_start_label`'s technique, parameterized so
/// probe (b) can plant it at a WRONG bank.
fn as_bank_start_label_at(vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\nMovingTrucks_Bank_Start:\n\tdc.w 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

fn span() -> Span {
    Span { source: SourceId(0), start: 0, end: 0 }
}

// ===========================================================================
// Probe (a) — STRADDLE: the `sfx_bank` region pinned at a base near the top of
// a bank so the real 1864-byte section crosses the `$68000` boundary.
// ===========================================================================

/// `sfx_bank.emp`'s content is 1864 bytes (`$748`, shape-invariant). Pinning
/// the region's `lma_base` at `0x67C00` puts the section at `[0x67C00,
/// 0x67C00+0x748)` = `[0x67C00, 0x68348)`, which crosses `$68000` — a
/// straddle. This is `sfx_port.rs`'s own real map, doctored ONLY in the region
/// base (same `.emp` file, same `place_sections`/`resolve_layout` pipeline as
/// the real Task 4 gate — `place_sections` marks the region's first section
/// `Pinned` at the region base), so this is the PORT-SHAPED straddle route.
///
/// FALSIFIED (restore-real-value): re-ran with `lma_base` restored to the real
/// plain base `0x63AE8` (no straddle) — `resolve_layout` genuinely returns
/// `Ok`, so `.expect_err(...)` panics on the `Ok`; confirmed by temporarily
/// flipping the base back and observing the `.expect_err` trip, then reverting
/// to the doctored `0x67C00`.
#[test]
fn straddle_doctored_map_base_is_a_loud_bank_boundary_error() {
    let Some(src) = real_sfx_bank_src() else { return };

    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(sfx_dir()),
        defines: vec![], // shape-invariant (R4)
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    // Doctored: base $67C00 — the 1864-byte ($748) section ends at $68348,
    // crossing $68000. `text` region included: the module's top-level `ensure`
    // opens the lazy zero-byte default `text` carrier (R-T0.3), so it needs a
    // home too (mirrors `sfx_port.rs::map_toml`). Region sized $500 so the
    // straddling section still fits the region window (the straddle is the
    // section's, not a region overflow).
    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = 0x67C00\n\
         size = 0x800\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");

    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");

    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect_err("a straddling sfx_bank must be a loud resolve_layout error, not silently placed");
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("sfx_bank")
                && d.message.contains("straddle")
        }),
        "expected a straddle error naming `sfx_bank`, got: {err:?}"
    );
}

// ===========================================================================
// Probe (b) — WRONG-BANK ENSURE: the real `sfx_bank.emp`, correctly placed,
// but the cross-seam `MovingTrucks_Bank_Start` symbol supplied at the WRONG
// bank.
// ===========================================================================

/// `sfx_bank` genuinely lands at `0x63AE8` (bank $C, `bankid = (0x63AE8 &
/// $7F8000) >> 15 = 0xC`). Supplying the synthetic `MovingTrucks_Bank_Start`
/// label at `0x58000` (bank $B) instead of the real `0x60000` (also bank $C)
/// means the ONE `ensure(bankid("Sfx_33") == bankid("MovingTrucks_Bank_
/// Start"), "...")` co-residency guard compares bank $C against bank $B — a
/// genuine mismatch, not a vacuous always-pass. `resolve_layout` + `link` both
/// succeed (nothing is mis-placed); `check_link_asserts` reports the ONE
/// ensure as a loud `Error` carrying its "not co-located with the engine-table
/// bank" message.
///
/// FALSIFIED (restore-real-value): re-ran with the label restored to the real
/// `0x60000` — `check_link_asserts` returns an EMPTY diagnostic list (the one
/// ensure passes); confirmed by temporarily asserting `.is_empty()` at the
/// real address and observing it hold, then reverting to the wrong address and
/// the `len() == 1` assertion below.
#[test]
fn wrong_bank_cross_seam_label_fires_the_co_residency_ensure() {
    let Some(src) = real_sfx_bank_src() else { return };

    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "parse errors: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(sfx_dir()),
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "lower errors: {ldiags:?}");

    // Real plain base ($63AE8, bank $C) — sized to the $68000 bank top.
    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = 0x63AE8\n\
         size = 0x4518\n\
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
        .unwrap_or_else(|d| panic!("resolve_layout must still succeed (only the ensure should fail): {d:?}"));
    sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link must still succeed (only the ensure should fail): {d:?}"));

    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    assert_eq!(
        assert_diags.len(),
        1,
        "expected exactly the one co-residency ensure to fire (wrong bank), got: {assert_diags:?}"
    );
    assert!(
        assert_diags[0].level == Level::Error,
        "the firing ensure must be Level::Error: {assert_diags:?}"
    );
    assert!(
        assert_diags[0]
            .message
            .contains("not co-located with the engine-table bank"),
        "expected the co-residency ensure's exact message substring, got: {assert_diags:?}"
    );
}

// ===========================================================================
// Probe (c) — TABLE-LENGTH MISMATCH: an inline module reproducing the SfxTable
// shape (`[*u8; SFX_TABLE_LEN]`, `const SFX_TABLE_LEN = 135`) doctored short.
// ===========================================================================

/// `sfx_bank.emp`'s `SfxTable` is `data SfxTable: [*u8; SFX_TABLE_LEN]` with
/// `const SFX_TABLE_LEN = 135` — a hand-owned sparse table (R1/R5). The
/// realistic hand-edit mistake is dropping (or adding) a row while leaving the
/// const at 135. This inline module reproduces that shape at small scale
/// (`SFX_TABLE_LEN = 135` const, but only 134 elements supplied) and pins that
/// the array-length guard fires a CLEAN "array length mismatch" `Error` naming
/// the two counts, never a panic. (The real file's 135-element literal is
/// covered by Task 4's byte gate + the type check; this probe pins the
/// diagnostic SHAPE of the check that backs the type, on a stand-in the test
/// can doctor without touching the committed file.)
#[test]
fn table_length_mismatch_against_sfx_table_len_is_a_clean_error() {
    // One real sym target + a 133-element zero run = 134 elements, one short
    // of SFX_TABLE_LEN = 135.
    let zeros = std::iter::repeat_n("0", 133).collect::<Vec<_>>().join(", ");
    let src = format!(
        "module probe.sfx_table_len\n\
         data Sfx_33: [u8;1] = [1]\n\
         const SFX_TABLE_LEN = 135\n\
         data SfxTable: [*u8; SFX_TABLE_LEN] = [\"Sfx_33\", {zeros}]\n"
    );
    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.is_empty(), "expected a clean parse, got: {pdiags:?}");
    let (_module, ldiags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] },
    );
    assert!(
        ldiags.iter().any(|d| d.level == Level::Error
            && d.message.contains("array length mismatch")
            && d.message.contains("135")
            && d.message.contains("134")),
        "expected a clean array-length-mismatch error naming 135 and 134, got: {ldiags:?}"
    );
    // No panic reaching this line is itself part of the assertion.
}

// ===========================================================================
// Probe (d) — WRONG-SYM-FOR-NULL (JUDGED: WRITTEN).
// ===========================================================================
//
// The task offers a choice: write a wrong-sym-for-null demonstration OR skip
// with a rationale (T2 probe-(e) precedent). JUDGMENT — WRITTEN, because it
// covers a class Task 4's falsification does NOT:
//
//   * Task 4's byte-gate falsification XOR'd the EXPECTED window (corrupting
//     the reference side of the assert) — it proves the gate is non-vacuous,
//     but says nothing about how a WRONG *input* behaves.
//   * This probe corrupts the INPUT (a wrong sym where a `0` null belongs) and
//     proves two things at once: (1) the wrong sym LOWERS CLEANLY — no
//     spurious diagnostic that would mask the drift and give false comfort;
//     (2) it emits DIFFERENT bytes than a correct `0` null. Together those
//     pin that the byte gate (Task 4) is the SOLE guard against this
//     transcription class — a legal lowering with wrong bytes, which is
//     exactly the failure a sparse hand-owned table invites.
//
// Done on a MINIMAL inline `[*u8; N]` module (not the 135-element real file):
// the class is width-independent, and a small module keeps the "legal, but
// different bytes" contrast readable in one screen.

/// Lower a two-element `[*u8; 2]` table twice — once with the correct `0` null
/// in slot 1, once with a WRONG sym (`"Sfx_33"`) there — and prove BOTH lower
/// cleanly (no Error) yet produce DIFFERENT bytes for that slot. This is the
/// LEGAL-lowering-different-bytes demonstration: only Task 4's byte gate
/// distinguishes them.
#[test]
fn wrong_sym_where_null_belongs_lowers_clean_but_emits_different_bytes() {
    fn table_bytes(slot1: &str) -> Vec<u8> {
        let src = format!(
            "module probe.wrong_sym\n\
             data Sfx_33: [u8;1] = [1]\n\
             data T: [*u8; 2] = [\"Sfx_33\", {slot1}]\n"
        );
        let (file, pdiags) = parse_str(&src);
        assert!(pdiags.is_empty(), "expected a clean parse, got: {pdiags:?}");
        let (module, ldiags) = lower_module(
            &file,
            &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] },
        );
        // The KEY assertion: a wrong sym where a null belongs is a LEGAL
        // lowering — no Error masks the byte drift.
        assert!(
            ldiags.iter().all(|d| d.level != Level::Error),
            "table with slot1=`{slot1}` must lower cleanly (Error would mask the drift): {ldiags:?}"
        );
        // Place + link so the `T` section's SymRef cells resolve to real bytes.
        let map = load_map(
            "fill = 0x00\n\
             [[region]]\n\
             name = \"text\"\n\
             lma_base = 0x1000\n\
             size = 0x100\n\
             kind = \"rom\"\n",
        )
        .expect("map must load");
        let mut sections = module.sections;
        let pdiags = place_sections(&mut sections, &map);
        assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");
        let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
            .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
        let linked = sigil_link::link(&resolved, &SymbolTable::new())
            .unwrap_or_else(|d| panic!("link: {d:?}"));
        linked.section("text").expect("linked image must carry the T section's region").bytes.clone()
    }

    let correct = table_bytes("0"); // slot1 = null (the production shape)
    let wrong = table_bytes("\"Sfx_33\""); // slot1 = a WRONG sym

    // Both lowered cleanly (asserted inside `table_bytes`). The bytes DIFFER —
    // slot1 is `00 00 00 00` (null) in `correct` vs the `Sfx_33` address in
    // `wrong` — so only the byte gate (Task 4) catches this transcription
    // class; the front-end never rejects it.
    assert_ne!(
        correct, wrong,
        "a wrong sym where a null belongs must produce DIFFERENT bytes (else the byte gate could not catch it)"
    );
    // Pin the specific slot: correct slot1 (bytes [4..8)) is the zero null.
    assert_eq!(&correct[4..8], &[0, 0, 0, 0], "correct slot1 must be the null cell");
    assert_ne!(&wrong[4..8], &[0, 0, 0, 0], "wrong slot1 must carry a non-null address");
}

// ===========================================================================
// Probe (e) — GROWN-MT OVERLAP: the REAL mt_bank.emp AND the REAL sfx_bank.emp
// composed in ONE resolve_layout, with mt's placed extent grown past the sfx
// base so the two sections interleave (R7p.4 `overlap_diag`).
// ===========================================================================

/// The map places `mt_bank` @ `0x60607` and `sfx_bank` @ `0x63AE8` — the real
/// per-shape bases (Task 5's `emp_bank_map_with_mt`). In the reference build
/// `mt_bank`'s content ends EXACTLY at `0x63AE8` (the two blocks are
/// contiguous — sfx resumes right where mt ends), so the map's regions abut
/// benignly. This probe models "an mt_bank regen GREW past the sfx base": it
/// places mt at its REAL base with its REAL content, then APPENDS synthetic
/// padding bytes to the `mt_bank` section (a `Fill` fragment) so its placed
/// image extent crosses `0x63AE8` into `sfx_bank`'s window. That is the most
/// faithful model of the actual risk — a real regen (an added song) that
/// stayed at the same base but produced MORE bytes than the seam budgeted —
/// without fabricating a section from whole cloth.
///
/// `resolve_layout` must fail LOUD with the two-section overlap diagnostic
/// (`overlap_diag`, relax.rs R7p.4): an Error naming BOTH `mt_bank` and
/// `sfx_bank` plus both hex extents. This pins that the map's benign
/// region-level abutment (name-keyed per-region cursors) can NEVER degrade
/// into silent section interleaving — a grown section trips the check.
///
/// FALSIFIED (restore-real-value): re-ran WITHOUT the appended padding (mt at
/// its real 13,537-byte extent, ending exactly at `0x63AE8`) — `resolve_layout`
/// returns `Ok` (the sections abut, they do not overlap), so `.expect_err(...)`
/// panics on the `Ok`; confirmed by temporarily skipping the `push(Fill)` and
/// observing the `.expect_err` trip, then restoring the padding.
#[test]
fn grown_mt_section_past_the_sfx_base_is_a_loud_overlap_error() {
    let (Some(mt_src), Some(sfx_src)) = (real_mt_bank_src(), real_sfx_bank_src()) else {
        return;
    };

    // The joint map: mt_bank @ $60607 (bank window to $68000) and sfx_bank @
    // $63AE8 — the real Task-5 abutment. `text` carriers for both modules'
    // zero-byte default sections.
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
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = 0x63AE8\n\
         size = 0x4518\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");

    // Lower + place mt_bank (DEBUG=0, its real region 0x60607).
    let (mt_file, mt_pdiags) = parse_str(&mt_src);
    assert!(mt_pdiags.iter().all(|d| d.level != Level::Error), "mt parse: {mt_pdiags:?}");
    let (mt_module, mt_ldiags) = lower_module(
        &mt_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(sound_dir()),
            defines: vec![("DEBUG".to_string(), 0)],
        },
    );
    assert!(mt_ldiags.iter().all(|d| d.level != Level::Error), "mt lower: {mt_ldiags:?}");
    let mut mt_sections = mt_module.sections;
    let pdiags = place_sections(&mut mt_sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "mt place_sections: {pdiags:?}");

    // DOCTOR: grow the `mt_bank` section past the sfx base. Its real DEBUG=0
    // content ends EXACTLY at $63AE8 (== the sfx base), so ANY positive
    // padding crosses into sfx_bank's window. $200 bytes puts mt's end at
    // $63CE8, well inside [$63AE8, $64230) — an unambiguous overlap. This
    // models a regen that stayed at $60607 but emitted more bytes.
    let mt_bank = mt_sections
        .iter_mut()
        .find(|s| s.name == "mt_bank")
        .expect("composed sections must contain the mt_bank section");
    mt_bank.fragments.push(Fragment::Fill { value: 0x00, count: 0x200, span: span() });

    // Lower + place sfx_bank (plain, its real region 0x63AE8) — UNMODIFIED.
    let (sfx_file, sfx_pdiags) = parse_str(&sfx_src);
    assert!(sfx_pdiags.iter().all(|d| d.level != Level::Error), "sfx parse: {sfx_pdiags:?}");
    let (sfx_module, sfx_ldiags) = lower_module(
        &sfx_file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: Some(sfx_dir()),
            defines: vec![],
        },
    );
    assert!(sfx_ldiags.iter().all(|d| d.level != Level::Error), "sfx lower: {sfx_ldiags:?}");
    let mut sfx_sections = sfx_module.sections;
    let pdiags = place_sections(&mut sfx_sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "sfx place_sections: {pdiags:?}");

    // ONE resolve over BOTH modules' sections. The grown mt_bank overlaps
    // sfx_bank → loud R7p.4 overlap error.
    let mut sections = mt_sections;
    sections.extend(sfx_sections);

    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true).expect_err(
        "a grown mt_bank straddling into sfx_bank's window must be a loud overlap error, not silent interleaving",
    );
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("overlap")
                && d.message.contains("mt_bank")
                && d.message.contains("sfx_bank")
        }),
        "expected a two-section overlap error naming BOTH mt_bank and sfx_bank, got: {err:?}"
    );
}
