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

/// The `repin` pin generator (tranche-10 step 0): listing parsing, the
/// `repin.toml` manifest, and the `pins.rs` renderer. Driven by the `repin`
/// bin and the `repin_pins` staleness test.
pub mod repin;

/// The FROZEN contract-closure baselines — the single copy the CI gates and the
/// build-integrated closure gate both read, so a pin cannot fork.
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

/// The golden provenance chain (`golden/provenance.toml`) — the §17 optimization arc's
/// re-freeze discipline: tip-match + anchor-move-needs-A/B. Driven by the `refreeze`
/// bin and the `provenance_chain` gate test.
pub mod provenance;

/// Flip Stage 1 · S1.1 — the all-gates-ON native whole-ROM driver (the registry
/// + `build_native_rom`), the seam-2 whole-ROM template generalized to all 53
/// gates. Consumed by the `native_rom` gate.
pub mod native;

/// Parcel-K1 placement map reader (`games/<g>/map.toml`) — the declared anchors /
/// hole / budget / order-validation facts the chainer checks its layout against.
pub mod map_placement;

use sigil_link::LinkedImage;

/// Region A base LMA in the assembled ROM: the resident phase-0 Z80 driver.
/// Provenance: the retired `golden/windows.toml`, `regen`-derived from the
/// bracketing 68k anchor label `Z80_Sound_Start`. Slid `0x3EA → 0x3E0` in
/// phase2.5 c5 (the dead Cold_Boot CROSS_RESET store removed upstream −0xA),
/// then `0x3E0 → 0x3DE` in the t23 boot clr.w wave (−2 upstream; the Z80
/// blob content is byte-identical, only its ROM position shifts).
pub const REGION_A_LMA: u32 = 0x3DE;
// `REGION_B_LMA` (the phase-`08000h` Moving-Trucks / SFX engine-table bank base)
// used to live here as a pinned literal. It had no remaining consumers, and the
// bank base's sole authority is now the `sound_bank` anchor in
// `games/<g>/map.toml` (read by `seam2::bank_anchors` / `seam2::sound_layout`), so
// the const was DELETED rather than left as a second, drifting truth.

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}

/// The Genesis header's checksum word (`$18E..$190`) — a SEMANTIC header
/// fact, not a layout pin. One of the only two fields the out-of-scope
/// `convsym -a`/`fixheader` post-steps rewrite (`convsym -a` appends the
/// MD-Debugger `deb2` symbol table; `fixheader` re-checksums the appended
/// image — M1.B models `convsym` as a no-op, so Sigil's `emit_rom` target is
/// the pre-append ASSEMBLED ROM).
pub const CHECKSUM_FIELD_RANGE: std::ops::Range<usize> = 0x18E..0x190;
/// The header's `dc.l EndOfRom-1` ROM-end pointer (`$1A4..$1A8`) — the other
/// convsym-rewritten field (bumped to the POST-append end). Which of its four
/// bytes actually differ shifts whenever the deb2 append changes size (the
/// tranche-9 PerFrame deletion flipped `$1A5` back), which is why the
/// rewritten set is DERIVED per comparison rather than pinned — see
/// [`derive_convsym_rewritten`] (tranche-10 step 0, D-T10.6).
pub const ROM_END_FIELD_RANGE: std::ops::Range<usize> = 0x1A4..0x1A8;

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
        "{label}: the release reference is {} bytes but EndOfRom is {expected_len:#x} — a \
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
             field ({range:#X?}), but assembled and final ROMs match there — did the reference \
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
