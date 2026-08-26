//! Sound-migration T3 — the REAL `sfx_bank.emp` port, region-level byte gate.
//!
//! `mt_port.rs`'s sibling (Task 4): compiles the ACTUAL ported file from aeon's
//! tree — `games/sonic4/data/sound/sfx/sfx_bank.emp` — through the production
//! parse -> lower -> place -> resolve -> link pipeline, with `include_root` set
//! to the module's OWN directory (so the eighteen `embed(...)` blobs resolve),
//! and asserts the `sfx_bank` section's flattened bytes equal the reference ROM
//! window at the pinned addresses, in BOTH build shapes.
//!
//! ## No shape define
//!
//! Unlike `mt_bank.emp`, this module carries NO `DEBUG` member: the SFX block's
//! CONTENT is byte-identical plain and debug (1864 bytes = `$748` in both) — only
//! its BASE address shifts, because it sits after the shape-dependent song tables
//! (plain `$5BB20` / debug `$5D570`). So the SHAPE lives entirely in the MAP
//! (per-shape `map_toml(debug)` region base, R7), not in the module: `lower` runs
//! with an EMPTY `defines` vec for both shapes.
//!
//! ## The cross-seam symbol
//!
//! `sfx_bank.emp` carries ONE link-time `ensure` of the shape
//! `ensure(bankid("Sfx_33") == bankid("MovingTrucks_Bank_Start"), "...")` (R5 —
//! the :260 co-residency fatal's successor). It reads the LABEL rather than the
//! `SND_ENGINE_TABLE_BANK` equ directly for the same reason `mt_bank.emp` does
//! (the bankid-label idiom, T2 Deviation 2). So the ONLY external symbol this
//! test must supply is `MovingTrucks_Bank_Start` — via the `mt_port` `phase`-label
//! technique verbatim: a synthetic AS unit that `phase`s a label to the exact VMA
//! the real `.asm` head pins it at ($58000, main.asm's `align $8000`), placed at a
//! harness-private LMA that cannot collide with the `sfx_bank`/`text` map regions,
//! then concatenated with the `.emp` sections before ONE `resolve_layout` + `link`
//! + `check_link_asserts` pass.
//!
//! ## Reference windows
//!
//! Plain (map base `$5BB20`): `s4.bin[0x5BB20..0x5C31E]` (2046 bytes).
//! Debug (map base `$5D570`): `s4.debug.bin[0x5D570..0x5DD6E]` (2046 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure (mirrors the
//! `mt_port.rs` gate idiom).
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test sfx_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::seam2::{sound_layout, SoundLayout};
use sigil_harness::test_support::{reference_tree, strict_gate};
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// The live map's sound layout, read from the aeon root five levels above the
/// `sound/sfx` dir — every bank address this port uses is DERIVED from it.
fn layout(sfx_dir: &Path) -> SoundLayout {
    let aeon = sfx_dir.ancestors().nth(5).expect("games/sonic4/data/sound/sfx has an aeon root");
    sound_layout(aeon).expect("sound_layout derives the SFX block bases from map.toml")
}

/// REFERENCE-DEPENDENT: the module's own directory in aeon's tree — the
/// `include_root` under which the eighteen `embed("sfx_*.bin")` fixtures
/// resolve. `Some(dir)` when the tree carries the module and its fixtures (the
/// first pair stands for the set — they ship together in this one directory);
/// `None` — both tests SKIP green — when it does not, unless
/// `SIGIL_STRICT_GATE=1` makes absence a hard failure.
fn sound_dir() -> Option<PathBuf> {
    reference_tree(&[
        "games/sonic4/data/sound/sfx/sfx_bank.emp",
        "games/sonic4/data/sound/sfx/sfx_33.bin",
        "games/sonic4/data/sound/sfx/sfx_33_patches.bin",
    ])
    .map(|aeon| aeon.join("games/sonic4/data/sound/sfx"))
}

/// The FROZEN golden slice comparand (the asl-witnessed reference), NOT the live
/// tree ROM — post-flip `aeon/s4.bin` is itself sigil-built, so composing `.emp`
/// and comparing to it would be circular; the committed golden is the independent
/// row-91 witness (bar b). Mirrors `native_offcanonical_rom::golden`.
fn golden(name: &str) -> Option<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) if strict_gate() => panic!("golden missing: {}", path.display()),
        Err(_) => {
            eprintln!("skip: golden not at {} (set SIGIL_STRICT_GATE)", path.display());
            None
        }
    }
}

/// The map: a `text` region for the module's zero-byte default-section carrier
/// (opened by its top-level `ensure` — R-T0.3's carrier contract) and the real
/// `sfx_bank` region pinned at the PER-SHAPE LMA `seam2::sound_layout` derives
/// from the live map, sized to the sound bank's top (anchor + $8000). The ONLY
/// structural difference from `mt_port.rs`'s map: the region base is
/// shape-dependent, so this is a `fn of debug`. Nothing is retyped: a re-layout
/// (e.g. 2026-08-26, `$5BB20`/`$5D570` → `$A3B20`/`$A5570`) moves this with it.
fn map_toml(sfx_dir: &Path, debug: bool) -> String {
    let l = layout(sfx_dir);
    let base = if debug { l.sfx_bank_lma_debug } else { l.sfx_bank_lma_plain };
    let top = l.sound_tables_z80_lma + 0x8000;
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"sfx_bank\"\n\
         lma_base = 0x{base:X}\n\
         size = 0x{:X}\n\
         kind = \"rom\"\n",
        top - base
    )
}

/// The synthetic AS-side cross-seam unit: a label `phase`d to the exact VMA the
/// harness carriers pin `MovingTrucks_Bank_Start` at — the `sound_bank` anchor
/// (`seam2::sound_layout().sound_tables_z80_lma`, the head of the bank the SFX
/// block shares) — the `mt_port`/T0 `probe_b` idiom, which proved a
/// `bankid("Name")` ensure resolves against a label defined this way exactly as
/// it would against the real cross-source symbol.
fn as_bank_start_label(sfx_dir: &Path) -> Vec<Section> {
    // Just the bank-start label the ONE surviving ensure (bankid co-residency)
    // reads cross-seam. Parcel F2 retired sfx_bank.emp's SFX_ID_BASE/SFX_COUNT/
    // SFX_TABLE_LEN drift guards (they are now the SOLE authority, derived from the
    // SfxTable rows — nothing external to cross-check), so no id-count carrier is
    // needed here.
    let bank = layout(sfx_dir).sound_tables_z80_lma;
    let asm = format!("cpu 68000\nphase ${bank:X}\nMovingTrucks_Bank_Start:\n\tdc.w 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (cross-seam label): {d:?}")).sections
}

/// Parse -> lower (with the sfx-dir include-root, NO defines) -> place the `.emp`
/// sections into the per-shape map -> append the synthetic cross-seam label
/// section at a harness-private LMA (clear of both map regions) -> ONE
/// `resolve_layout` -> `link` -> `check_link_asserts`. Returns the placed+resolved
/// `.emp` sections, the linked image, the link-assert diagnostics (expected empty
/// — the ONE ensure passes), and the module's captured link asserts.
fn compile_real_file(
    dir: &Path,
    debug: bool,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_span::Diagnostic>, Vec<LinkAssert>) {
    let emp_path = dir.join("sfx_bank.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    // NO defines: the SFX block is shape-invariant; the shape lives in the map.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.to_path_buf()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(dir, debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append the cross-seam label section at a harness-private LMA — well clear
    // of both `text` ($0..$10) and `sfx_bank` — so it cannot collide with either
    // map region. Its VMA ($58000, from `phase`) is what the `bankid()` ensure
    // actually reads; its LMA placement here is inert harness bookkeeping.
    let mut cross_seam = as_bank_start_label(dir);
    for sec in &mut cross_seam {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(cross_seam);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed (bank straddle / ensure?): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &module.link_asserts);
    (resolved, linked, assert_diags, module.link_asserts)
}

/// On mismatch, report the first differing offset plus 8 bytes of context on each
/// side (`mt_port.rs` style byte-diff reporting): the window starts 8 bytes BEFORE
/// the first-diff offset (not at it) so the panic message shows bytes on both
/// sides of the diff, not just after it.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// (plain) The `sfx_bank` section's linked bytes equal `s4.bin[0x5BB20..0x5C31E]`.
#[test]
fn sfx_bank_region_matches_reference() {
    let Some(dir) = sound_dir() else { return };
    let Some(refrom) = golden("s4.bin") else { return };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(&dir, false);
    assert_eq!(
        guard_assert_count(&link_asserts),
        1,
        "sfx_bank.emp's ONE surviving ensure must be captured (the bankid co-residency; the SFX_ID_BASE/SFX_COUNT/SFX_TABLE_LEN drift guards retired at F2 — sfx_bank is the derived authority)"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the cross-seam co-residency ensure must PASS (link succeeded): {assert_diags:?}"
    );

    let base = layout(&dir).sfx_bank_lma_plain as usize;
    let section = linked.section("sfx_bank").expect("linked image must carry sfx_bank");
    let expected = &refrom[base..base + section.bytes.len()];
    assert_region_matches(&section.bytes, expected, &format!("sfx_bank (plain) vs s4.bin[{base:#X}..{:#X}]", base + section.bytes.len()));
}

/// (debug) The `sfx_bank` section's linked bytes equal
/// `s4.debug.bin[0x5D570..0x5DD6E]`.
#[test]
fn sfx_bank_debug_region_matches_reference() {
    let Some(dir) = sound_dir() else { return };
    let Some(refrom) = golden("s4.debug.bin") else { return };

    let (_resolved, linked, assert_diags, link_asserts) = compile_real_file(&dir, true);
    assert_eq!(
        guard_assert_count(&link_asserts),
        1,
        "sfx_bank.emp's ONE surviving ensure must be captured (the bankid co-residency; the SFX_ID_BASE/SFX_COUNT/SFX_TABLE_LEN drift guards retired at F2 — sfx_bank is the derived authority)"
    );
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the cross-seam co-residency ensure must PASS (link succeeded): {assert_diags:?}"
    );

    let base = layout(&dir).sfx_bank_lma_debug as usize;
    let section = linked.section("sfx_bank").expect("linked image must carry sfx_bank");
    let expected = &refrom[base..base + section.bytes.len()];
    assert_region_matches(&section.bytes, expected, &format!("sfx_bank (debug) vs s4.debug.bin[{base:#X}..{:#X}]", base + section.bytes.len()));
}

/// Count the deferred GUARD asserts, excluding the D2.29 [layout.odd-item]
/// parity asserts that now also ride module.link_asserts. Shared idiom in
/// `sigil_harness::test_support`.
use sigil_harness::test_support::guard_assert_count;
