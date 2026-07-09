//! Sound-migration T1+T2 acceptance — the MIXED `.asm`+`.emp` full-ROM harness.
//!
//! This is each tranche's acceptance bar (DSM.9): assemble aeon's REAL
//! `games/sonic4/main.asm` with one or both sound gates ON (so `main.asm`'s
//! `gameSoundDataIncludes` macro skips the matching `.asm` block and resumes
//! placement by `org`), compile the matching REAL `.emp` module(s) from aeon's
//! tree, COMPOSE everything into ONE linked image, and prove the full ROM is
//! BYTE-IDENTICAL to the assembled reference — the same target as the
//! all-`.asm` `m1d_rom` / `m1d_debug_rom` gates.
//!
//! Two variants per tranche (plain + `__DEBUG__`), mirroring the two m1d tests,
//! prove BOTH build shapes compose. The all-`.asm` m1d gates build WITHOUT
//! either define; the mixed tests build WITH them — all coexist.
//!
//! ## Composition (the T1 technique, from ports.rs + dac_port.rs)
//!
//! The `.emp` side is placed into a bank map BY SECTION NAME (dac_port.rs:
//! `dac_blip_bank` @ $50000, `dac_shared_bank` @ $58000; T2 adds `mt_bank` @
//! $60607, size $79F9 — mt_port.rs's region, R7). The top-level `SND_*`/`equ`
//! carriers land in zero-byte `text` sections given a benign home at LMA 0 —
//! T2 lowers TWO `.emp` modules (`dac_samples.emp` + `mt_bank.emp`), each
//! opening its OWN `text` carrier, and P5/R7 already proved a same-named-`text`
//! pair chains cleanly through one region (both zero bytes, cumulative
//! per-region cursor). The `.asm` side's sections are org-pinned by AS itself.
//! Every `Vec<Section>` (AS + both `.emp` modules) is concatenated and run
//! through ONE `resolve_layout` + `link` (ports.rs T4 technique) so every
//! cross-seam symbol resolves through a single shared table. No new link infra.
//!
//! The zero-byte `text` carrier(s) at LMA 0 alias the AS reset section's LMA,
//! but `resolve_layout`'s R7p.4 overlap check filters out zero-image-byte
//! sections (`overlap_diag` keys on `image_final_size`), and `flatten` skips
//! empty sections — so the carrier(s) are benign, contributing nothing and
//! colliding with nothing (proven for the pair in Task 2's P5 probe).
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
//! **T2 adds NO new gap.** The MT block's `.asm` else-arm resumes placement
//! EXACTLY at `mt_bank`'s section end (`$63AE8` plain / `$6553A` debug — the
//! fact base's tail addresses): `mt_bank.emp`'s items emit contiguously
//! (§4.3 no-auto-pad) all the way to `SongPatchTable_End`, and the SFX block
//! that follows in `.asm` picks up at that exact address with no `align`
//! between — so there is no inter-section pad to reason about here, unlike the
//! DAC banks' bank-aligned boundaries above.
//!
//! ## Cross-seam resolution (T2 — the imm32 deferral proving out end-to-end)
//!
//! `engine/sound/sound_api.asm`'s `movea.l #SongTable, a0` / `movea.l
//! #SongPatchTable, a0` are UNCONDITIONAL engine code (not gated by
//! `SIGIL_EMP_MT`) that reference labels `mt_bank.emp`'s `mt_bank` section
//! defines (`SongTable`/`SongPatchTable`, at `$63AE0`/`$63AE4` plain,
//! `$65522`/`$6552E` debug). Since the AS side assembles these two operands
//! before the `.emp` module is even lowered, they are unresolved AT AS-TIME —
//! Task 3's `Value32Be` imm32 deferral (R3) is what lets `main.asm` assemble at
//! all here instead of hard-erroring; the deferred fixups are then satisfied by
//! the ONE joint `resolve_layout` + `link` pass below, exactly like every other
//! cross-seam symbol. `MovingTrucks_Bank_Start` (main.asm:138, read by
//! `mt_bank.emp`'s five `bankid(...)` co-residency ensures) is a real `.asm`
//! label defined UNCONDITIONALLY (outside both gates) — so unlike `mt_port.rs`,
//! no synthetic cross-seam symbol injection is needed here: the real AS module
//! supplies it for real, through the same shared symbol table.
//!
//! ## STOP RULE (DSM.9)
//!
//! Expected divergences from the reference: NONE beyond the
//! `convsym`/`fixheader`-rewritten header bytes (identical sets to `m1d_rom` /
//! the T1 mixed tests, since the composed ROM content is byte-identical to the
//! all-`.asm` build). Any other differing offset is a REAL divergence — this
//! test reports it (offset + 16 bytes context each side) and FAILS. It does NOT
//! allowlist new offsets.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (and `aeon/s4.debug.bin`
//! for the debug variants). Absent it SKIPS green unless `SIGIL_STRICT_GATE=1`.
//! Mirrors `m1d_rom` / `m1d_debug_rom`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test mixed_dac_rom
//! ```

use std::path::{Path, PathBuf};

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::{
    assemble_mixed_dac_as_side, assemble_mixed_mt_as_side, assert_rom_matches, CONVSYM_REWRITTEN,
    CONVSYM_REWRITTEN_DEBUG,
};
use sigil_ir::backend::Cpu;
use sigil_ir::{LinkAssert, Section, SymbolTable};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

// `CONVSYM_REWRITTEN` / `CONVSYM_REWRITTEN_DEBUG` (imported from `sigil_harness`):
// IDENTICAL to `m1d_rom` / `m1d_debug_rom`'s sets — the mixed build's ROM content
// is byte-identical to the all-`.asm` build, so `convsym` rewrites the same
// bytes to the same values relative to the same assembled length.

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

/// The two-bank map for placing `dac_samples.emp`'s sections BY NAME, at the
/// aeon-f828406 pins (dac_port.rs verbatim): `dac_blip_bank` @ $50000,
/// `dac_shared_bank` @ $58000. The top-level `SND_*` equs land in the default
/// `text` section — a ZERO-byte carrier here (all equs, no data cells) —
/// which `place_sections` still requires a home for; a nominal `text` region
/// at LMA 0 is benign (the R7p.4 overlap check and `flatten` both skip
/// zero-image-byte sections, so it never collides with the AS reset section
/// that also anchors at LMA 0).
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

/// T2's map: `emp_bank_map`'s three regions PLUS `mt_bank` @ `0x60607` size
/// `0x79F9` (mt_port.rs's R7 region, to the bank top `$68000`). Both
/// `dac_samples.emp` and `mt_bank.emp` each open their own zero-byte `text`
/// carrier — Task 2's P5 probe proved a same-named `text` pair chains fine
/// through one region (cumulative per-region cursor) — so a single `text`
/// region still covers both modules' carriers here.
fn emp_bank_map_with_mt() -> &'static str {
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
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"mt_bank\"\n\
     lma_base = 0x60607\n\
     size = 0x79F9\n\
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
    placed_module_sections(aeon, "dac_samples.emp", &[], emp_bank_map())
}

/// Compile the REAL `dac_samples.emp` (no defines) and `mt_bank.emp` (`DEBUG`
/// matching the shape), each PLACED into `emp_bank_map_with_mt`'s regions by
/// name, returning BOTH modules' placed sections concatenated (dac first, mt
/// second — declaration order only, `resolve_layout` doesn't care).
fn placed_emp_sections_with_mt(aeon: &Path, debug_val: i128) -> Vec<Section> {
    let mut sections = placed_module_sections(aeon, "dac_samples.emp", &[], emp_bank_map_with_mt());
    sections.extend(placed_module_sections(
        aeon,
        "mt_bank.emp",
        &[("DEBUG".to_string(), debug_val)],
        emp_bank_map_with_mt(),
    ));
    sections
}

/// Shared body of `placed_emp_sections`/`placed_emp_sections_with_mt`: parse +
/// lower the named `.emp` file (from `sound_dir`, with the given comptime
/// `defines`) and place its sections into `map_src` by name.
fn placed_module_sections(
    aeon: &Path,
    module_file: &str,
    defines: &[(String, i128)],
    map_src: &str,
) -> Vec<Section> {
    let dir = sound_dir(aeon);
    let emp_path = dir.join(module_file);
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} parse errors: {pdiags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        defines: defines.to_vec(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(map_src).expect("emp bank map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{module_file} place_sections errors (region-per-section): {pdiags:?}"
    );

    // The `text` carrier MUST stay zero-byte — the whole "benign at LMA 0"
    // argument (module doc) rests on it contributing no image bytes.
    assert!(
        sections.iter().filter(|s| s.name == "text").all(|s| s.image_bytes().is_empty()),
        "{module_file} `text` carrier gained image bytes — the zero-byte-carrier invariant is broken"
    );

    sections
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

/// T2's shared body: assemble the AS side with BOTH `SIGIL_EMP_DAC` +
/// `SIGIL_EMP_MT` on, compose with BOTH placed `.emp` modules' sections,
/// resolve+link ONCE (so `sound_api.asm`'s deferred `movea.l
/// #SongTable`/`#SongPatchTable` fixups resolve against `mt_bank.emp`'s
/// labels through the SAME shared table everything else uses), check the five
/// cross-seam `ensure`s actually ran and passed, and emit the full ROM.
/// Returns `(rom_bytes, link_assert_diags)` — the caller asserts the diags are
/// all non-Error, mirroring `mt_port.rs`'s explicit `check_link_asserts` call.
fn build_mixed_mt_rom(aeon: &Path, debug: bool) -> (Vec<u8>, Vec<sigil_span::Diagnostic>) {
    let as_module = assemble_mixed_mt_as_side(aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let debug_val: i128 = if debug { 1 } else { 0 };

    // The MT `.emp` module's link_asserts (the five `bankid(...)` co-residency
    // ensures) — captured from a fresh lower pass mirroring
    // `placed_module_sections`'s own parse+lower, since `place_sections`
    // consumes `module.sections` but the LinkAssert list lives on `module`
    // itself, not on any `Section`. mt_port.rs's `compile_real_file` captures
    // this the same way (before place_sections).
    let dir = sound_dir(aeon);
    let mt_src = std::fs::read_to_string(dir.join("mt_bank.emp"))
        .unwrap_or_else(|e| panic!("cannot read mt_bank.emp: {e}"));
    let (mt_file, pdiags) = parse_str(&mt_src);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "mt_bank.emp parse: {pdiags:?}");
    let mt_opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        defines: vec![("DEBUG".to_string(), debug_val)],
    };
    let (mt_module, ldiags) = lower_module(&mt_file, &mt_opts);
    assert!(ldiags.iter().all(|d| d.level != sigil_span::Level::Error), "mt_bank.emp lower: {ldiags:?}");
    let link_asserts: Vec<LinkAssert> = mt_module.link_asserts;

    // Concat: AS sections + BOTH placed emp modules' sections. ONE joint
    // resolve_layout + link over the union.
    let mut sections = as_module.sections;
    sections.extend(placed_emp_sections_with_mt(aeon, debug_val));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed MT): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed MT): {d:?}"));
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed MT): {e}"));
    (rom, assert_diags)
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
    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed");
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
    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed debug",
    );
}

/// T2 acceptance — plain (non-debug) DAC+MT mixed build == `aeon/s4.bin`,
/// modulo the four convsym bytes. Both `SIGIL_EMP_DAC` and `SIGIL_EMP_MT` are
/// ON; both `.emp` modules are lowered and composed; the five `mt_bank.emp`
/// cross-seam ensures must genuinely run (via `check_link_asserts`) and pass.
#[test]
fn mixed_mt_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };
    let (rom, assert_diags) = build_mixed_mt_rom(&aeon, false);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );
    assert_rom_matches(&rom, &refrom, ASSEMBLED_LEN, CONVSYM_REWRITTEN, "DSM.9 STOP: mixed MT");
}

/// T2 acceptance — `__DEBUG__` DAC+MT mixed build == `aeon/s4.debug.bin`,
/// modulo the five convsym bytes. Same composition as the plain variant, with
/// `DEBUG=1` driving both `mt_bank.emp`'s if-expressions and `__DEBUG__` on
/// the AS side.
#[test]
fn mixed_mt_debug_rom_matches_assembled_reference() {
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
    let (rom, assert_diags) = build_mixed_mt_rom(&aeon, true);
    assert!(
        assert_diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "mt_bank.emp's five cross-seam co-residency ensures must all PASS (link succeeded): {assert_diags:?}"
    );
    assert_rom_matches(
        &rom,
        &refrom,
        DEBUG_ASSEMBLED_LEN,
        CONVSYM_REWRITTEN_DEBUG,
        "DSM.9 STOP: mixed MT debug",
    );
}
