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
use sigil_ir::{Cpu, SymbolTable};
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
    let root = aeon.join("games/sonic4/main.asm");
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![("SOUND_DRIVER_ENABLED".to_string(), 1)],
        include_root: Some(aeon.to_path_buf()),
    };
    let module = assemble_root(&root, &opts)
        .map_err(|d| format!("assemble: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&module.sections, &stubs, true)
        .map_err(|d| format!("resolve_layout: {} diagnostics; first: {:?}", d.len(), d.first()))?;
    sigil_link::link(&resolved, &stubs)
        .map_err(|d| format!("link: {} diagnostics; first: {:?}", d.len(), d.first()))
}

/// The bytes of the linked section whose LMA equals `lma`. Regions are keyed by
/// their ROM base address, not by section name — the front-end's auto-section
/// names (`sec{vma}`) are disambiguated on collision and so are not stable
/// identifiers (the Z80 driver's `phase 0` region and the 68k reset section both
/// base at vma 0).
pub fn region_at_lma(img: &LinkedImage, lma: u32) -> Option<&[u8]> {
    img.sections.iter().find(|s| s.lma == lma).map(|s| s.bytes.as_slice())
}
