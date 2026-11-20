//! sigil-harness — reference-build helpers shared by the strict gates and the CLI.
//!
//! The native whole-ROM build lives in [`native`] (the all-gates-ON driver + the
//! off-canonical profiles); the seam-1/seam-2 sound composition lives in [`seam1`]
//! / [`seam2`]; layout pins in [`pins`]. This module retains the small shared
//! comparison helpers the surviving golden gates use: [`region_at_lma`] (fetch a
//! linked section by LMA) and the convsym allowlist ([`derive_convsym_rewritten`]
//! / [`assert_rom_matches_convsym`] / [`assert_rom_matches`]) that confines the
//! two semantic header fields the out-of-scope `convsym`/`fixheader` post-steps
//! rewrite — in every shape that carries a deb2 appendix, which since the
//! crash-report ruling (owner-ruled 2026-08-04) is BOTH canonical shapes, release
//! included. Only the opt-in `lean` profile ships no appendix; its exact-identity
//! bar is [`assert_rom_matches_release`].

/// Shared test-support: the AS-truth equ blob for the `engine.constants` twin
/// and the drift-guard filter, consolidated out of ~9 hand-copied port/probe
/// test files. Reachable by both the CLI tests and this crate's own tests
/// (both depend on `sigil-harness`).
pub mod test_support;

/// WHICH test binaries ask the reference tree a question, derived from source so no
/// consumer types a number. Read by the gate that makes a run say how much of itself it
/// did not measure, and by the bare-run refusal that names the same population.
pub mod reference_dependence;

/// WHICH harness tree a tool writes into — derived from the invoking tree and
/// handed from a parent tool to its children, never taken from link time. Shared
/// by `refreeze` and `repin` so a parent and its child cannot resolve to
/// different checkouts.
pub mod harness_root;

/// The `repin` pin generator (tranche-10 step 0): listing parsing, the
/// `repin.toml` manifest, and the `pins.rs` renderer. Driven by the `repin`
/// bin and the `repin_pins` staleness test.
pub mod repin;

/// The contract-closure baseline diff: one shape's frozen rows (read from its
/// descriptions' `baseline.toml` files) and the multiset diff the build gate and the CI
/// gates both run, so the two cannot grow two meanings of "moved against the baseline".
pub mod contract_baseline;

/// GENERATED layout pins (regions/symbols/offsets in both build shapes) —
/// the single source the port tests import. Regenerate with
/// `cargo run -p sigil-harness --bin repin`; never edit by hand.
pub mod pins;

/// The seam-1 resident-sound-blob native link: the five sound `.emp` files linked
/// as ONE Z80 module set, shared by the whole-ROM gates and the `emit_sound_blob`
/// bin (Option A — sigil emits the canonical build inputs asl packs).
pub mod seam1;

pub mod seam2;

/// The one renderer every error diagnostic the native build reports goes through:
/// one `path:line:col: [Error] message` line per diagnostic, for the comptime
/// guards, the deferred link asserts, and the sound emitters' co-link guards alike.
pub use sigil_build::diag::render as diag_render;
/// The import rule for the seams that lower `.emp` files with no resolve pass.
pub mod import_check;

/// The strict-gate CENSUS: the population of `if !strict_gate()` consultations the
/// test tree declares, derived from source at attest time, plus the set comparison
/// `refreeze --attest` runs it against the run's witness. Replaces the old
/// `strict_bodies == 0` floor, which a gate going dark could satisfy.
pub mod strict_census;

/// The golden provenance chain (`golden/provenance.toml`) — the §17 optimization arc's
/// re-freeze discipline: tip-match + anchor-move-needs-A/B. Driven by the `refreeze`
/// bin and the `provenance_chain` gate test.
pub mod provenance;

/// Which of `refreeze --freeze`'s four regeneration steps completed, so that a run
/// killed in one of the three joints BETWEEN them is nameable afterwards instead of
/// leaving fresh artifacts beside stale ones that all parse. Driven by the `refreeze`
/// bin and the `freeze_step_gap` gate test.
pub mod freeze_journal;

/// Whether the revisions `provenance.toml` records still EXIST in the histories they
/// name. [`provenance`] checks their SHAPE; this checks that a branch still reaches
/// them, and distinguishes an object this clone has not fetched from one a rebase
/// orphaned. Driven by `refreeze --reachability` and the `rev_reachability` gate test.
pub mod rev_reachability;

/// What is left of the profile-driven driver after G0b Task 11: the sound-artifact
/// emitters a build's seam and the standalone port gates call, the fold check, and the
/// `native::` re-exports of the game-independent driver code `sigil-build` holds.
pub mod native;

/// Parcel-K1 placement map reader (`games/<g>/map.toml`) — the declared anchors /
/// hole / budget / order-validation facts the chainer checks its layout against.
/// Moved, unchanged, to `sigil_build::map_placement` (G0b Task 4).
pub use sigil_build::map_placement;

/// Per-game `[defines]` table reader (`games/<g>/map.toml`) — the game-declared
/// comptime define rows the game-as-data driver merges with the shape's description
/// rows (conflict = loud error, neither source wins).
pub use sigil_build::game_defines;

/// `[sound.bank-id-vs-placement]`: the sound bank ids the emit bakes into the resident
/// Z80 blob and the DAC descriptor head, checked against the banks as placed. Run by
/// every build's sound seam (`driver_seams::HarnessSeams::validate_sound`).
pub mod sound_bank_ids;

/// Installing a file by rename rather than by truncation, so a consumer reading an
/// artifact while sigil writes it sees the whole previous file or the whole new one
/// and never a prefix. Every artifact this workspace hands to a consumer goes
/// through here; the module doc also states what the guarantee does NOT cover,
/// including the file-mode question a rename answers differently from a truncation.
pub mod atomic_write;

/// The stdout writes of every sigil binary, through `print!`/`println!` macros that
/// shadow std's. When stdout's reader has gone, the writes stop and the run ends with
/// its own status, silently, rather than std's exit-101 panic; the module doc states
/// the rule, why it is not exit 0 or SIGPIPE's 141, and what it leaves alone.
pub mod stdout;

/// The `.lst` source digest of one native build: the files it read (from the
/// `sigil_span::read_set` recorder), the module scan's membership, the build
/// configuration and the identity of the ROM it wrote.
pub mod source_digest;
pub mod export_files;
/// The build export's symbol table, from the front ends' side tables (G0a Task 5).
pub mod export_layouts;
pub mod export_symbols;
pub mod export_z80;
/// The new driver's harness seams (G0b Task 5).
pub mod driver_seams;

/// A described shape built through the game-as-data driver, for the harness's tests and
/// tools (G0b Task 11).
pub mod shapes;

use sigil_link::LinkedImage;

// The region base LMAs are NOT constants here. Each has exactly one authority, and a
// pinned literal beside it is a second, drifting truth — both `REGION_A_LMA` and
// `REGION_B_LMA` were deleted for that reason, each having drifted off its authority
// while no consumer remained to notice:
//   * Region A (the resident phase-0 Z80 driver) — `seam1::blob_lma(debug)`, i.e.
//     `pins::Z80_SOUND_START.<shape>`, the label's own pin, so a boot-size shift
//     cannot rot it and the shape-dependence stays explicit.
//   * Region B (the phase-`08000h` Moving-Trucks / SFX engine-table bank) — the
//     `sound_bank` anchor in `games/<g>/map.toml`, read by `seam2::bank_anchors` /
//     `seam2::sound_layout`.

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}

/// The Genesis header's checksum word and ROM-end pointer: the two semantic header
/// fields the `convsym -a` / `fixheader` post-steps rewrite. Moved, unchanged, to
/// `sigil_build::emit` (G0b Task 4).
pub use sigil_build::emit::{CHECKSUM_FIELD_RANGE, ROM_END_FIELD_RANGE};

/// The offsets at which `rom` (assembled) and `refrom` (final, post-convsym)
/// differ WITHIN the two semantic header fields above — the derived
/// convsym/fixheader allowlist. Confinement is NOT checked here: pass the
/// result to [`assert_rom_matches`] (or use [`assert_rom_matches_convsym`]),
/// whose unexpected-offset check then asserts the FULL diff set ⊆ these two
/// fields with the DSM.9 evidence format on failure.
pub fn derive_convsym_rewritten(rom: &[u8], refrom: &[u8]) -> Vec<usize> {
    let n = rom.len().min(refrom.len());
    (0..n)
        .filter(|&i| CHECKSUM_FIELD_RANGE.contains(&i) || ROM_END_FIELD_RANGE.contains(&i))
        .filter(|&i| rom[i] != refrom[i])
        .collect()
}

/// The NO-APPENDIX whole-ROM bar: a build that ships no deb2 appendix, so the
/// reference file IS the assembled image — same length, and identical in every byte
/// INCLUDING the two header fields the appendix would otherwise perturb (`emit_rom`
/// folds the checksum over exactly these bytes and nothing re-fixes it afterwards).
/// The length check is the standing regression guard against the appendix leaking
/// into such an artifact; the empty allowlist is the point, not an omission — the
/// convsym/fixheader post-steps do not run in this shape, so there is nothing to allow.
///
/// SCOPE (owner-ruled 2026-08-04): this WAS the release-shape bar under review item
/// 29. The crash-report ruling put the deb2 symbol table back into release, so the
/// only shape it describes now is the opt-in `lean` profile. The release shapes moved
/// to [`assert_rom_matches_convsym`]. Retained as the canonical statement of the bar
/// (and reusable by any future no-appendix shape); `lean`'s own gate today is the
/// anchor comparison in `native_offcanonical_rom`, plus the appendix-length assertion
/// in `native_offcanonical_full`.
pub fn assert_rom_matches_release(rom: &[u8], refrom: &[u8], expected_len: usize, label: &str) {
    assert_eq!(
        refrom.len(),
        expected_len,
        "{label}: the release reference is {} bytes but EndOfRom is {expected_len:#x}, a \
         non-DEBUG build must ship the assembled image with NOTHING appended (has the deb2 \
         symbol appendix leaked back into release?)",
        refrom.len()
    );
    assert_rom_matches(rom, refrom, expected_len, &[], label);
}

/// The DEBUG-shape whole-ROM bar: [`assert_rom_matches`] with the convsym
/// allowlist DERIVED-and-CONFINED (D-T10.6): the allowlist is computed from the
/// actual header diff instead of pinned, killing that re-pin row; confinement to
/// the two semantic fields is enforced by the unexpected-offset check (any diff
/// outside them is unlisted and fails with full evidence). Each field must
/// genuinely differ somewhere — a reference rebuilt WITHOUT the convsym append
/// would otherwise silently change shape under us (the guard the pinned arrays'
/// "allowlisted bytes must differ" assert used to provide).
///
/// DEBUG SHAPES ONLY since item 29 gated the appendix: a release reference has
/// no appendix and therefore no rewritten header fields at all — that shape's
/// bar is [`assert_rom_matches_release`] (exact identity).
pub fn assert_rom_matches_convsym(rom: &[u8], refrom: &[u8], expected_len: usize, label: &str) {
    let allow = derive_convsym_rewritten(rom, refrom);
    for (field, range) in
        [("checksum", CHECKSUM_FIELD_RANGE), ("ROM-end pointer", ROM_END_FIELD_RANGE)]
    {
        assert!(
            allow.iter().any(|i| range.contains(i)),
            "{label}: expected the convsym/fixheader post-steps to rewrite the header {field} \
             field ({range:#X?}), but assembled and final ROMs match there, did the reference \
             lose its deb2 append?"
        );
    }
    assert_rom_matches(rom, refrom, expected_len, &allow, label);
}

/// Assert `rom` is byte-identical to `refrom` modulo the `allow`-listed offsets,
/// after pinning `rom`'s length to `expected_len` (guards against a regression
/// that drops/adds a trailing section while leaving the header-adjacent prefix —
/// and the allowlisted diffs — byte-identical, which would otherwise silently
/// pass the diff check below).
///
/// On mismatch, reports the FIRST unexpected differing offset with 16 bytes of
/// context from each side (the DSM.9 STOP-RULE evidence format), plus every
/// unexpected offset's sigil/ref byte values, then panics. Finally confirms the
/// allowlisted bytes genuinely differ — this guards against the reference
/// silently changing shape under us (e.g. a rebuild without the convsym append
/// would make these match, and this assertion would catch it).
///
/// `label` names the ROM under test in panic messages (e.g. `"mixed"`,
/// `"sigil"`, `"sigil debug"`) so failures from different gates are
/// distinguishable.
pub fn assert_rom_matches(
    rom: &[u8],
    refrom: &[u8],
    expected_len: usize,
    allow: &[usize],
    label: &str,
) {
    assert_eq!(
        rom.len(),
        expected_len,
        "{label} ROM length changed (dropped/added section, or an org skip lost content?); \
         expected EndOfRom {expected_len:#x}"
    );
    assert!(
        rom.len() <= refrom.len(),
        "{label} ROM {} longer than reference {}",
        rom.len(),
        refrom.len()
    );

    let unexpected: Vec<usize> =
        (0..rom.len()).filter(|&i| rom[i] != refrom[i] && !allow.contains(&i)).collect();
    if let Some(&i) = unexpected.first() {
        let ctx = |b: &[u8]| {
            let hi = (i + 16).min(b.len());
            b[i..hi].to_vec()
        };
        let detail: Vec<String> = unexpected
            .iter()
            .map(|&j| format!("{j:#x} ({label} {:#04x} != ref {:#04x})", rom[j], refrom[j]))
            .collect();
        panic!(
            "{label} ROM diverges from the reference at {} unexpected offset(s); \
             FIRST at {i:#x} ({label} {:#04x} != ref {:#04x})\n\
             {label}[{i:#x}..] = {:02X?}\n  ref[{i:#x}..] = {:02X?}\n\
             (all unexpected offsets: {})",
            unexpected.len(),
            rom[i],
            refrom[i],
            ctx(rom),
            ctx(refrom),
            detail.join(", "),
        );
    }
    // The allowlisted bytes MUST genuinely differ — else the reference changed
    // shape under us (e.g. a rebuild without the convsym append).
    for &i in allow {
        assert!(
            i < rom.len() && rom[i] != refrom[i],
            "expected convsym-rewritten byte at {i:#x} to differ, but it matched"
        );
    }
}
