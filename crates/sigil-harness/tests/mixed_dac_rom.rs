//! Sound-migration T1 acceptance — the MIXED `.asm`+`.emp` full-ROM harness.
//!
//! This is the tranche's acceptance bar (DSM.9): assemble aeon's REAL
//! `games/sonic4/main.asm` with the `SIGIL_EMP_DAC` gate ON (so its
//! `gameSoundDataIncludes` macro SKIPS `dac_samples.asm` and `org $60000`
//! resumes placement for the Moving-Trucks bank), compile the REAL
//! `dac_samples.emp` from aeon's tree (pinning `dac_blip_bank` @ $50000 and
//! `dac_shared_bank` @ $58000), COMPOSE the two into ONE linked image, and prove
//! the full ROM is BYTE-IDENTICAL to the assembled reference — the same target
//! as the all-`.asm` `m1d_rom` / `m1d_debug_rom` gates.
//!
//! Two variants (plain + `__DEBUG__`), mirroring the two m1d tests, prove BOTH
//! build shapes compose. The all-`.asm` m1d gates build WITHOUT the define; this
//! one builds WITH it — the two coexist.
//!
//! ## Composition (the T1 technique, from ports.rs + dac_port.rs)
//!
//! The `.emp` side is placed into a two-bank map BY SECTION NAME (dac_port.rs:
//! `dac_blip_bank` @ $50000, `dac_shared_bank` @ $58000; the top-level `SND_*`
//! equs land in a zero-byte `text` carrier given a benign home at LMA 0). The
//! `.asm` side's sections are org-pinned by AS itself. The two `Vec<Section>` are
//! concatenated and run through ONE `resolve_layout` + `link` (ports.rs T4
//! technique) so every cross-seam symbol resolves through a single shared table.
//! No new link infra.
//!
//! The zero-byte `text` carrier at LMA 0 aliases the AS reset section's LMA, but
//! `resolve_layout`'s R7p.4 overlap check filters out zero-image-byte sections
//! (`overlap_diag` keys on `image_final_size`), and `flatten` skips empty
//! sections — so the carrier is benign, contributing nothing and colliding with
//! nothing.
//!
//! ## Gap-fill (Task 9 §3 — inspected in the reference before pinning)
//!
//! In the all-`.asm` ROM the bytes between the pre-DAC content and $50000,
//! between the blip bank's end ($50B40) and $58000, and between the drums' end
//! ($5F8BC) and $60000 are produced by asl `align $8000`. In the mixed build
//! those become INTER-SECTION gaps produced by the flatten fill. `xxd` of the
//! reference `aeon/s4.bin` at all three ranges (0x4FFF0, 0x57FF0, 0x5FFF0, and
//! the two bank tails 0x50B40 / 0x5F8C0) shows the pad byte is `0x00`
//! throughout — matching `sigil.map.toml`'s `fill = 0x00` (which `emit_rom` uses
//! for every gap). The pre-DAC content ends at $4867A (Art_Sonic's `align 2`
//! tail, per s4.lst); the blip bank REALLY starts at $50000 in the reference
//! (verified: 0x4FFFx is all-zero, 0x50000 is the first blip byte `80 A6 …`), so
//! nothing lives in $4867A..$50000 except align pad — exactly the gap the
//! flatten fill reproduces. The `org` skip drops ONLY the two BINCLUDE banks +
//! comments + equates from `dac_samples.asm`; the byte-identity assertion below
//! is what proves nothing else was lost.
//!
//! ## STOP RULE (DSM.9)
//!
//! Expected divergences from the reference: NONE beyond the four
//! `convsym`/`fixheader`-rewritten header bytes (identical to `m1d_rom`, since
//! the composed ROM content is byte-identical to the all-`.asm` build). Any
//! other differing offset is a REAL divergence — this test reports it (offset +
//! 16 bytes context each side) and FAILS. It does NOT allowlist new offsets.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (and `aeon/s4.debug.bin`
//! for the debug variant). Absent it SKIPS green unless `SIGIL_STRICT_GATE=1`.
//! Mirrors `m1d_rom` / `m1d_debug_rom`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test mixed_dac_rom
//! ```

use std::path::{Path, PathBuf};

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::assemble_mixed_dac_as_side;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The only offsets at which the assembled ROM legitimately differs from
/// `s4.bin`: the header checksum and the low half of the ROM-end pointer, both
/// rewritten by the out-of-scope `convsym -a`/`fixheader` post-steps. IDENTICAL
/// to `m1d_rom`'s set — the mixed build's ROM content is byte-identical to the
/// all-`.asm` build, so `convsym` rewrites the same four bytes to the same
/// values relative to the same assembled length.
const CONVSYM_REWRITTEN: &[usize] = &[0x18E, 0x18F, 0x1A6, 0x1A7];
/// The debug reference's convsym/fixheader-rewritten set (from `m1d_debug_rom`):
/// the larger `__DEBUG__` deb2 append pushes the ROM-end pointer over a byte
/// boundary, so three bytes ($1A5/$1A6/$1A7) differ instead of two.
const CONVSYM_REWRITTEN_DEBUG: &[usize] = &[0x18E, 0x18F, 0x1A5, 0x1A6, 0x1A7];

/// The assembled (pre-convsym-append) ROM length pins, from `m1d_rom` /
/// `m1d_debug_rom` — `EndOfRom` of each build shape. The mixed build reproduces
/// the same `EndOfRom` (identical content), so these pins double as a
/// dropped-section guard here too.
const ASSEMBLED_LEN: usize = 0x658B4;
const DEBUG_ASSEMBLED_LEN: usize = 0x673A2;

/// The `.emp` module's own directory in aeon's tree — the `include_root` under
/// which `embed("temp_blip.bin")` / `embed("dac/*.pcm")` resolve (dac_port.rs).
fn sound_dir(aeon: &Path) -> PathBuf {
    aeon.join("games/sonic4/data/sound")
}

/// The two-bank map for placing the `.emp` sections BY NAME, at the aeon-f828406
/// pins (dac_port.rs verbatim): `dac_blip_bank` @ $50000, `dac_shared_bank` @
/// $58000. The top-level `SND_*` equs land in the default `text` section — a
/// ZERO-byte carrier here (all equs, no data cells) — which `place_sections`
/// still requires a home for; a nominal `text` region at LMA 0 is benign (the
/// R7p.4 overlap check and `flatten` both skip zero-image-byte sections, so it
/// never collides with the AS reset section that also anchors at LMA 0).
fn emp_bank_map() -> &'static str {
    "fill = 0x00\n\
     \n\
     [[region]]\n\
     name = \"text\"\n\
     lma_base = 0x0000\n\
     size = 0x10\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_blip_bank\"\n\
     lma_base = 0x50000\n\
     size = 0x8000\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_shared_bank\"\n\
     lma_base = 0x58000\n\
     size = 0x8000\n\
     kind = \"rom\"\n"
}

/// Compile the REAL `dac_samples.emp` and PLACE its sections into the two-bank
/// map (dac_port.rs pipeline). Returns the placed sections ready to concat with
/// the AS side. Placement runs against `emp_bank_map`, NOT the whole-ROM
/// `sigil.map.toml`, because `place_sections` matches BY NAME and errors on any
/// section without a region — so the AS sections (org-pinned already) are never
/// fed through it; only the emp sections are placed here, then the union is
/// resolved+linked once.
fn placed_emp_sections(aeon: &Path) -> Vec<Section> {
    let dir = sound_dir(aeon);
    let emp_path = dir.join("dac_samples.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: Some(dir.clone()) };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "emp lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(emp_bank_map()).expect("emp bank map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );
    sections
}

/// Assert two ROMs are byte-identical modulo the convsym allowlist, reporting the
/// first UNEXPECTED differing offset with 16 bytes of context from each side (the
/// DSM.9 STOP-RULE evidence format), then confirming the allowlisted bytes
/// genuinely differ (guards against the reference silently changing shape).
fn assert_rom_matches(rom: &[u8], refrom: &[u8], allow: &[usize], expected_len: usize) {
    assert_eq!(
        rom.len(),
        expected_len,
        "mixed ROM length changed (dropped/added section, or the org skip lost content?); \
         expected EndOfRom {expected_len:#x}"
    );
    assert!(
        rom.len() <= refrom.len(),
        "mixed ROM {} longer than reference {}",
        rom.len(),
        refrom.len()
    );

    let unexpected: Vec<usize> =
        (0..rom.len()).filter(|&i| rom[i] != refrom[i] && !allow.contains(&i)).collect();
    if let Some(&i) = unexpected.first() {
        let ctx = |b: &[u8]| {
            let lo = i.saturating_sub(0);
            let hi = (i + 16).min(b.len());
            b[lo..hi].to_vec()
        };
        panic!(
            "DSM.9 STOP: mixed ROM diverges from the reference at {} unexpected offset(s); \
             FIRST at {i:#x} (mixed {:#04x} != ref {:#04x})\n\
             mixed[{i:#x}..] = {:02X?}\n  ref[{i:#x}..] = {:02X?}\n\
             (all unexpected offsets: {:#X?})",
            unexpected.len(),
            rom[i],
            refrom[i],
            ctx(rom),
            ctx(refrom),
            unexpected,
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

/// Shared body: assemble the AS side (gate ON, `debug` toggling `__DEBUG__`),
/// compose with the placed emp sections, resolve+link ONCE, and emit the full
/// ROM through the whole-ROM `sigil.map.toml`.
fn build_mixed_rom(aeon: &Path, debug: bool) -> Vec<u8> {
    let as_module = assemble_mixed_dac_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));

    // Concat: AS sections (org-pinned by AS) + placed emp sections. ONE joint
    // resolve_layout + link over the union — the ports.rs T4 mixed-seam shape.
    let mut sections = as_module.sections;
    sections.extend(placed_emp_sections(aeon));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed): {d:?}"));

    // The whole-ROM map: `region_for` covers [0, 0x400000) so every section
    // (AS + emp banks) validates by LMA, and `fill = 0x00` matches the reference
    // align-pad byte inspected above.
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed): {e}"))
}

/// Plain (non-debug) mixed build == `aeon/s4.bin`, modulo the four convsym bytes.
#[test]
fn mixed_dac_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let rom = build_mixed_rom(&aeon, false);
    assert_rom_matches(&rom, &refrom, CONVSYM_REWRITTEN, ASSEMBLED_LEN);
}

/// `__DEBUG__` mixed build == `aeon/s4.debug.bin`, modulo the five convsym bytes.
#[test]
fn mixed_dac_debug_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!(
                "SIGIL_STRICT_GATE set but debug reference missing: aeon/s4.debug.bin \
                 (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4; see PROVENANCE.md)"
            );
        }
        eprintln!("skip: debug reference not at {} (build per PROVENANCE.md)", rom_path.display());
        return;
    };
    let rom = build_mixed_rom(&aeon, true);
    assert_rom_matches(&rom, &refrom, CONVSYM_REWRITTEN_DEBUG, DEBUG_ASSEMBLED_LEN);
}
