//! sigil-harness — reference-build helpers shared by the strict gates and the CLI.
//!
//! ## History (M1.D T6)
//!
//! This crate once drove an M0 "bounded harness": it assembled the Z80 sound
//! driver's Region A + Region B *in isolation* (`harness_root.asm`), stubbing the
//! ~42 68k leaf symbols the driver referenced but that the isolated build did not
//! define (`golden/stub-syms.toml`, re-derived by the `regen` bin). That
//! scaffolding existed only because Sigil could not yet assemble the whole 68k
//! ROM.
//!
//! It now can. The `m1d_rom` gate proves the full non-debug `main.asm` assembles
//! BYTE-EXACT to the reference with **zero stubs**, and `m0_regions` proves the
//! sound driver's Region A + Region B fall out of that full build byte-exact. So
//! the bounded harness, its stub table, and `regen` were all retired, leaving a
//! single reference-build entry point: "assemble the full non-debug ROM".

use std::path::Path;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, Module, SymbolTable};
use sigil_link::LinkedImage;

/// Region A base LMA in the assembled ROM: the resident phase-0 Z80 driver.
/// Provenance: the retired `golden/windows.toml`, `regen`-derived from the
/// bracketing 68k anchor label `Z80_Sound_Start`.
pub const REGION_A_LMA: u32 = 0x3EA;
/// Region B base LMA: the phase-`08000h` Moving-Trucks / SFX engine-table bank.
/// Provenance: `MovingTrucks_Bank_Start`.
pub const REGION_B_LMA: u32 = 0x60000;

/// Assemble the full non-debug Aeon ROM from `<aeon>/games/sonic4/main.asm` and
/// link it, with **no stubs** — the full include tree defines everything. Mirrors
/// `build.sh`'s default ASFLAGS: `SOUND_DRIVER_ENABLED` on, `__DEBUG__` off.
///
/// Returns the linked image (each section carries name / LMA / bytes); call
/// [`sigil_link::emit_rom`] on it for a flat ROM. This is the one reference-build
/// entry point shared by the CLI and the region gates.
pub fn assemble_full_rom(aeon: &Path) -> Result<LinkedImage, String> {
    assemble_full_rom_with(aeon, false)
}

/// Assemble the full **`__DEBUG__`** Aeon ROM (`DEBUG=1 ./build.sh`): everything
/// `assemble_full_rom` does, plus `__DEBUG__` defined, which pulls in
/// `debugger.asm`'s assertion / KDebug / `__FSTRING` error-message code. Used by
/// the `m1d_debug_rom` gate (A2).
pub fn assemble_full_rom_debug(aeon: &Path) -> Result<LinkedImage, String> {
    assemble_full_rom_with(aeon, true)
}

/// Shared body of the two entry points above. `debug` toggles the `__DEBUG__`
/// define; `SOUND_DRIVER_ENABLED` is always on (build.sh's default), no stubs.
fn assemble_full_rom_with(aeon: &Path, debug: bool) -> Result<LinkedImage, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    let module = assemble_root(&root, &opts)
        .map_err(|d| format!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &stubs, true)
        .map_err(|d| format!("resolve_layout: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("link: {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Assemble the AS side of the MIXED `.asm`+`.emp` build: everything
/// `assemble_full_rom` does (SOUND_DRIVER_ENABLED on, no stubs), PLUS
/// `SIGIL_EMP_DAC` defined so `main.asm`'s `gameSoundDataIncludes` macro SKIPS
/// `dac_samples.asm` and `org $60000` resumes placement for the Moving-Trucks
/// bank (leaving the $50000/$58000 DAC banks for the `.emp` side to supply).
/// `debug` toggles `__DEBUG__` exactly as the two `assemble_full_rom*` entry
/// points do — the mixed harness proves BOTH debug shapes compose.
///
/// Returns the UNLINKED [`Module`] (raw sections), not a `LinkedImage`: the
/// mixed harness concatenates these with the `.emp` module's placed sections and
/// runs ONE `resolve_layout` + `link` over the union, so the cross-seam symbols
/// (`SND_*_BANK/PTR/LEN` etc.) resolve through a single shared symbol table.
pub fn assemble_mixed_dac_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        // `asl`'s `ifndef` tests symbol EXISTENCE, so any value works; 1 mirrors
        // the other `-D` defines. This is the gate that flips main.asm's
        // dac_samples.asm include to `org $60000`.
        ("SIGIL_EMP_DAC".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts)
        .map_err(|d| format!("assemble (mixed AS side): {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// Assemble the AS side of the T2 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_dac_as_side` does, PLUS `SIGIL_EMP_MT` defined so
/// `main.asm`'s Moving-Trucks block (lines 150-208: the six streaming-bank
/// includes + the pitch-contiguity fatal) is REPLACED by an `org` resume — per
/// shape, `$6553A` (`__DEBUG__`) or `$63AE8` (plain) — leaving the whole
/// `$60607..end` window for the `.emp` side's `mt_bank` section to supply.
/// Both `SIGIL_EMP_DAC` and `SIGIL_EMP_MT` are independent gates (R6); T2's
/// mixed build exercises both ON together, DAC-only stays covered by the
/// unchanged `assemble_mixed_dac_as_side` T1 tests.
///
/// Returns the UNLINKED [`Module`], exactly like `assemble_mixed_dac_as_side`:
/// the T2 mixed harness concatenates these sections with BOTH `.emp` modules'
/// placed sections (`dac_samples.emp` + `mt_bank.emp`) and runs ONE
/// `resolve_layout` + `link` over the union, so every cross-seam symbol
/// (including `movea.l #SongTable`/`#SongPatchTable` in `sound_api.asm`,
/// deferred by Task 3's imm32 fixup) resolves through a single shared table.
pub fn assemble_mixed_mt_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed MT AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// Assemble the AS side of the T3 MIXED `.asm`+`.emp` build: everything
/// `assemble_mixed_mt_as_side` does, PLUS `SIGIL_EMP_SFX` defined so
/// `main.asm`'s SFX block (the 19 blob/patch/table includes + the two SFX
/// fatals, R6) is REPLACED by an `org` resume — per shape, `$65C82`
/// (`__DEBUG__`) or `$64230` (plain), i.e. `SfxTable_End` — leaving the whole
/// `$63AE8..SfxTable_End` window for the `.emp` side's `sfx_bank` section to
/// supply. All three gates (`SIGIL_EMP_DAC`, `SIGIL_EMP_MT`, `SIGIL_EMP_SFX`)
/// are independent (R6); T3's mixed build exercises all three ON together.
///
/// Returns the UNLINKED [`Module`], exactly like the two sibling helpers: the
/// T3 mixed harness concatenates these sections with all THREE `.emp` modules'
/// placed sections (`dac_samples.emp` + `mt_bank.emp` + `sfx_bank.emp`) and
/// runs ONE `resolve_layout` + `link` over the union. The cross-seam reads
/// unique to this tranche are the `soundBankHead` win-tab's nine
/// `dw sfx_winptr(Sfx_NN)` entries (a compound `(Sfx_NN & $7FFF) | $8000` in a
/// Z80 `phase 08000h` LE `dw`): with `SIGIL_EMP_SFX` on the `Sfx_NN` labels are
/// `.emp`-side, so those entries assemble here with the target UNRESOLVED (T0's
/// dw deferral, P1-proven) and are satisfied by the joint link against
/// `sfx_bank.emp`'s labels through the same shared symbol table everything else
/// uses.
pub fn assemble_mixed_sfx_as_side(aeon: &Path, debug: bool) -> Result<Module, String> {
    let root = aeon.join("games/sonic4/main.asm");
    let mut defines = vec![
        ("SOUND_DRIVER_ENABLED".to_string(), 1),
        ("SIGIL_EMP_DAC".to_string(), 1),
        ("SIGIL_EMP_MT".to_string(), 1),
        ("SIGIL_EMP_SFX".to_string(), 1),
    ];
    if debug {
        defines.push(("__DEBUG__".to_string(), 1));
    }
    let opts = Options { initial_cpu: Cpu::M68000, defines, include_root: Some(aeon.to_path_buf()) };
    assemble_root(&root, &opts).map_err(|d| {
        format!("assemble (mixed SFX AS side): {} diagnostics; first: {:?}", d.len(), d.first())
    })
}

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}

/// The only offsets at which Sigil's assembled (non-debug) ROM legitimately
/// differs from the pinned `s4.bin`: the header checksum and the low half of the
/// `dc.l EndOfRom-1` ROM-end pointer, both rewritten by the out-of-scope
/// `convsym -a`/`fixheader` post-steps (`convsym -a` appends the MD-Debugger
/// `deb2` symbol table and rewrites two header fields; `fixheader` re-checksums
/// the appended image — M1.B models `convsym` as a no-op, so Sigil's `emit_rom`
/// target is the pre-append ASSEMBLED ROM). See `m1d_rom`/`m1d_debug_rom`/
/// `mixed_dac_rom` for the full provenance.
pub const CONVSYM_REWRITTEN: &[usize] = &[0x18E, 0x18F, 0x1A6, 0x1A7];
/// The debug reference's convsym/fixheader-rewritten set: the larger `__DEBUG__`
/// deb2 append pushes the ROM-end pointer over a byte boundary, so three bytes
/// (`$1A5`/`$1A6`/`$1A7`) differ instead of two.
pub const CONVSYM_REWRITTEN_DEBUG: &[usize] = &[0x18E, 0x18F, 0x1A5, 0x1A6, 0x1A7];

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
