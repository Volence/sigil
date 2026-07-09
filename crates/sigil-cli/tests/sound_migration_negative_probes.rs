//! Sound-migration T0+T1, Task 10 — NEGATIVE probes: the four failure modes
//! of the DAC-port world must all fail LOUDLY, with diagnostics naming the
//! offending section/symbol, never silently corrupt or misplace bytes.
//!
//! Three of the four failure-mode CLASSES already have coverage elsewhere in
//! the suite (see each probe's doc comment for exactly what pre-existed and
//! why it doesn't fully cover the port-shaped/cross-source case this task
//! asks for); this file adds the specific shape each was missing:
//!
//!   1. straddle  — `final_placement.rs::pinned_bank_section_straddling_is_a_
//!      loud_error_not_moved` already proves a HAND-BUILT `Pinned` bank
//!      section straddling is a loud link error. Missing: the PORT-SHAPED
//!      route to a pin — a `--map` region (`place_sections`, the exact
//!      mechanism `dac_bank_acceptance.rs`/`dac_port.rs` use) landing a
//!      `bank:` section as the region's first (Pinned) occupant, through the
//!      real emp front-end. Added below.
//!   2. length guard — `dac_samples.emp` itself carries ten
//!      `ensure(0 < B.len && B.len < $8000, ...)` guards; nothing exercises
//!      the FAILING half (a guard that actually fires) in that exact shape.
//!      Added below, with a zero-length blob built from an empty array
//!      literal (`[]`), not `embed` — see the probe's doc comment for why.
//!   3. dup equ — `relax.rs::duplicate_equ_name_is_dup_symbol_error` already
//!      proves equ-vs-equ (two synthetic IR sections, hand-built, never
//!      touching a real front-end) collides. Missing: the shape the actual
//!      migration produces — an emp `equ` colliding with a still-.asm
//!      **label** of the same name, through the real parse/lower/assemble →
//!      place_sequential → resolve_layout → link pipeline (mirrors
//!      `ports.rs`'s `mixed_build_cross_seam_name_collision_errors`, which
//!      covers emp-DATA-label vs AS-label, not emp-EQU vs AS-label). Added
//!      below.
//!   4. overlap — AS org-pinned content colliding with an emp region-pinned
//!      `bank:` section. `final_placement.rs::colliding_pins_are_a_loud_
//!      link_error` proves overlap for two hand-built Pinned sections;
//!      `ports.rs`'s T4 tests prove the emp<->AS symbol seam but never drive
//!      two colliding PINS across that seam. Added below, ports.rs-style.
//!
//! Each probe asserts only the diagnostic's ESSENTIAL content (the section or
//! symbol name, the failure class), never a full string — brittle exact-text
//! assertions are exactly what this suite avoids elsewhere (see `ports.rs`).

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_link::load_map;
use sigil_span::Level;

/// The `Vec<Section>` an emp module lowers to (no sandbox root, 68k initial),
/// mirroring `ports.rs::emp_sections` — kept local per this file's own
/// house style (each `tests/*.rs` in this crate keeps its own small helpers
/// rather than sharing a harness crate; see `dac_bank_acceptance.rs`).
fn emp_sections(emp: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse: {pdiags:?}");
    let (module, ldiags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "emp lower: {ldiags:?}");
    module.sections
}

/// The `Vec<Section>` an AS source assembles to (68k, mirroring `ports.rs`).
fn as_sections(asm: &str) -> Vec<Section> {
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
}

// ===========================================================================
// Probe 1 — STRADDLE, port-shaped: a `bank:` section placed by a `--map`
// region (the real port mechanism) whose content crosses the 32KB boundary
// measured from the region's pinned base.
// ===========================================================================

/// A `(bank: $8000)` section is the FIRST (and only) occupant of a region
/// pinned at `lma_base = $57000` — NOT a 32KB-aligned base. `place_sections`
/// marks the first section in a region `Pinned` at that exact base (the real
/// mechanism `dac_bank_acceptance.rs`/`dac_port.rs` drive), so a section large
/// enough that `[$57000, $57000+len)` crosses the next `$8000` multiple
/// ($58000) straddles — and because it's PINNED (never bumped), the always-on
/// post-placement check must fire, naming the section and its extent, rather
/// than silently emitting bytes that would compute a WRONG `bankid()`/
/// `winptr()` on hardware.
///
/// $57000 to the next $8000-boundary ($58000) is $1000 (4096) bytes; a
/// 4096-byte blob would fit EXACTLY (no straddle — the boundary check is
/// half-open), so the blob is sized $1001 (4097) bytes, one byte past exact
/// fit, to force a genuine straddle without being a boundary off-by-one
/// artifact.
///
/// Falsification (TDD-loose, recorded per the task): re-run with the blob
/// shrunk to exactly 4096 bytes (fits before the boundary, no straddle) —
/// `resolve_layout` genuinely returns `Ok`, so `.expect_err(...)` panics on
/// the `Ok` value, proving this probe is not vacuously true at any size.
#[test]
fn port_shaped_pinned_region_straddle_is_a_loud_link_error() {
    // 4097-byte blob via the `Range.map` idiom (there is no array-repeat
    // literal syntax — mirrors `ports.rs::offsets_overflow_is_a_compile_error`'s
    // identical construction for its own oversized-blob probe).
    let emp = "module probe.straddle\n\
               const BlobRange = 0..4097\n\
               const Blob = BlobRange.map(|_| 0)\n\
               section dac_bank (bank: $8000) {\n\
                 data Payload: [u8; 4097] = Blob\n\
               }\n";
    let mut sections = emp_sections(emp);

    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"dac_bank\"\n\
         lma_base = 0x57000\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");

    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect_err("a pinned-region bank straddle must be a loud resolve_layout error");
    assert!(
        err.iter().any(|d| {
            d.level == Level::Error
                && d.message.contains("dac_bank")
                && d.message.contains("straddle")
        }),
        "expected a straddle error naming `dac_bank`, got: {err:?}"
    );
}

// ===========================================================================
// Probe 2 — LENGTH GUARD, comptime fire: the `dac_samples.emp` shape
// (`ensure(0 < B.len && B.len < $8000, "...")`) with a zero-length blob.
// ===========================================================================

/// Why an EMPTY ARRAY LITERAL (`[]`), not `embed(...)` of an empty file: the
/// task asks to "check which the language allows" — `embed` on an empty file
/// reads zero bytes cleanly (`eval_embed` has no zero-length special case; it
/// just yields an empty `Value::Data`), so an empty-file embed would NOT
/// error earlier for a different reason and IS a valid route to this probe.
/// But it requires a real file on disk (a sandboxed `include_root` fixture),
/// which is unnecessary ceremony for a probe whose entire point is the
/// `ensure` firing. `const B = []` is a plain, already-proven construct
/// (`eval_builtins.rs`'s `E.map`/`E.fold` tests use the identical `[]`
/// empty-array-literal shape) with `.len == 0` — the guard is exercised
/// exactly the same way, with zero filesystem dependency. This is a
/// same-semantics substitute, not a different mechanism: `dac_samples.emp`'s
/// guard reads `B.len` off any array-typed value, embed or literal alike.
///
/// Mirrors the real file's exact guard text shape
/// (`"DAC sample length must be > 0 and < $8000"`) so this probe reads as
/// the literal failing twin of a passing `dac_samples.emp` line, not an
/// invented condition.
#[test]
fn length_guard_zero_length_blob_fires_at_comptime() {
    let src = "module probe.length_guard\n\
               const B = []\n\
               ensure(0 < B.len && B.len < $8000, \"DAC sample length must be > 0 and < $8000\")\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "expected a clean parse, got: {pdiags:?}");
    let (_module, ldiags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        ldiags.iter().any(|d| d.level == Level::Error
            && d.message.contains("DAC sample length must be > 0 and < $8000")),
        "expected the length guard to fire AT COMPTIME (lower-time) with its own message, got: {ldiags:?}"
    );
}

/// The mirror-image positive: the SAME guard over a non-empty blob (`.len ==
/// 3`) is silent — proving probe 2 isn't vacuously failing on some unrelated
/// parse/lower defect (e.g. `[]`'s type) and that the guard genuinely keys on
/// the length, not on the array-literal construction itself.
#[test]
fn length_guard_nonzero_length_blob_is_silent() {
    let src = "module probe.length_guard\n\
               const B = [1, 2, 3]\n\
               ensure(0 < B.len && B.len < $8000, \"DAC sample length must be > 0 and < $8000\")\n";
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "expected a clean parse, got: {pdiags:?}");
    let (_module, ldiags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "a non-empty blob must pass the length guard silently, got: {ldiags:?}"
    );
}

// ===========================================================================
// Probe 3 — DUPLICATE EQU CROSS-SEAM: an emp `equ` colliding with a
// still-.asm LABEL of the same name (the realistic migration collision — a
// migrated constant name colliding with a not-yet-migrated .asm label).
// ===========================================================================

/// An emp section defines `equ SND_DUP = 1`; a Z80 AS source defines a LABEL
/// `SND_DUP:` — a genuine name collision a partial migration could produce
/// (one module's constant is ported to an emp `equ` while a sibling `.asm`
/// file still declares a same-named label, e.g. both meaning to be the same
/// symbolic constant during a staged migration). Composed and linked ONCE
/// through the shared symbol table (mirrors `ports.rs`'s
/// `mixed_build_cross_seam_name_collision_errors`, which collides emp DATA
/// labels with AS labels — this is the EQU-specific variant, which is a
/// different lowering path: `Section.equ_syms`, not `Section.labels`, per
/// `sigil-link/src/lib.rs`'s two separate `defined_here` population passes).
///
/// Falsification (TDD-loose, recorded per the task): re-run with the AS
/// label renamed to `SND_NOT_DUP` (no collision) — `link` genuinely returns
/// `Ok` (a real `LinkedImage` with both sections' bytes present), so
/// `.expect_err(...)` panics on the `Ok`, proving the assertion is exercising
/// the real dup-symbol channel rather than a tautology.
#[test]
fn dup_equ_cross_seam_emp_equ_vs_as_label_is_a_loud_link_error() {
    let emp = "module probe.dup_equ\n\
               section blob (cpu: m68000, vma: $8000) {\n\
                 data Anchor: u8 = 0\n\
                 equ SND_DUP = 1\n\
               }\n";
    // The AS side independently defines a LABEL of the identical name — the
    // realistic partial-migration collision, not an equ-vs-equ collision.
    let asm = "SND_DUP:\n\tdc.b $00\n";

    let mut sections = emp_sections(emp);
    sections.extend(as_sections(asm));
    // Mirror the T4 no-map tail: two independently-lowered/assembled modules
    // each stamp their first section Pinned at lma 0 — place sequentially
    // before resolve_layout (R7p.4's colliding-pins guard would otherwise
    // fire first, masking the intended dup-symbol error).
    sigil_frontend_emp::resolve::place_sequential(&mut sections, 0);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("an emp equ colliding with an AS label of the same name must be a loud link error");
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("SND_DUP")
            && d.message.to_lowercase().contains("redefin")),
        "expected a `SND_DUP redefined` dup-symbol error, got: {err:?}"
    );
}

// ===========================================================================
// Probe 4 — OVERLAP: AS org-pinned content colliding with an emp
// region-pinned `bank:` section, ports.rs-style construction.
// ===========================================================================

/// An AS source `org`-pinned at $58000 with real bytes, and an emp
/// `(bank: $8000)` section placed by a `--map` region ALSO at
/// `lma_base = $58000` — the classic dac-style co-residency mistake (an
/// AS-side sample or table not yet migrated off the exact address an emp
/// `bank:` section's region now claims). Composed through
/// `resolve_layout`'s R7p.4 overlap check (the SAME channel
/// `final_placement.rs::colliding_pins_are_a_loud_link_error` proves for two
/// hand-built sections) — but here BOTH sides are the real front-ends
/// (AS `org` + emp `place_sections`), so this is truthful placement, never
/// silent corruption: the two colliding pins must be a loud `resolve_layout`
/// `Err` naming both sections' extents, not a silently-overwritten image.
///
/// Falsification (TDD-loose, recorded per the task): re-run with the AS
/// `org` moved to $10000 (no collision — outside the $58000 bank region) —
/// `resolve_layout` genuinely returns `Ok`, so `.expect_err(...)` panics on
/// the `Ok`, proving the assertion depends on the real address overlap.
#[test]
fn overlap_as_org_pin_collides_with_emp_bank_region_pin_is_loud() {
    // AS side: 4 bytes physically placed at $58000 via `org` (an LMA pin —
    // unlike `phase` (VMA-only, `probe_b`'s mechanism above), a leading `org`
    // with no section yet open sets BOTH `vma_base` and `lma`, mirroring
    // `sigil-frontend-as::eval::tests::org_with_no_section_open_yet_just_sets_
    // the_phase_base`).
    let asm = "cpu 68000\norg $58000\nAsLegacyTable:\n\tdc.l $11223344\n";

    // emp side: a `bank:` section, region-placed at the SAME $58000 base — the
    // first (and only) section in its region lands Pinned there (R7p mechanism).
    let emp = "module probe.overlap\n\
               section dac_bank (bank: $8000) {\n\
                 data Payload: [u8; 4] = [$AA, $BB, $CC, $DD]\n\
               }\n";

    let mut emp_secs = emp_sections(emp);
    let map = load_map(
        "fill = 0x00\n\
         [[region]]\n\
         name = \"dac_bank\"\n\
         lma_base = 0x58000\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");
    let pdiags = place_sections(&mut emp_secs, &map);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "place_sections: {pdiags:?}");

    let mut sections = as_sections(asm);
    sections.extend(emp_secs);

    let err = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect_err("an AS org-pin colliding with an emp bank-region pin must be a loud overlap error");
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("overlap")
            && d.message.contains("dac_bank")),
        "expected a truthful-placement overlap error naming `dac_bank`, got: {err:?}"
    );
}
