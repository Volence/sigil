//! M1.D T6 (A3): the FULL build reproduces the Z80 sound driver's Region A +
//! Region B byte-exact, with **zero stubs** — the full `main.asm` include tree
//! defines every symbol the driver references (DAC bank/ptr/len, the MT/SFX bank
//! id, the `Sfx_XX` blob labels) itself.
//!
//! This re-expresses the retired M0 bounded-harness acceptance gate. The old gate
//! (`harness_root.asm` + `golden/stub-syms.toml` + `build_harness`) assembled
//! Regions A+B *in isolation*, stubbing the ~42 leaf symbols defined by the 68k
//! side it did not assemble. That scaffolding existed only because Sigil could not
//! yet assemble the whole 68k ROM. It now can (see `m1d_rom.rs`), so M0 acceptance
//! is a strict subset of the full-ROM match — but a *localized* one: a first-diff
//! here points straight at the sound driver, not at some offset in a 420 KB image.
//!
//! Region A is the resident phase-0 driver (LMA `0x3EA`); Region B is the
//! phase-`08000h` Moving-Trucks / SFX engine-table bank (LMA `0x60000`). The
//! region *lengths* are not pinned — they come from the live linked sections, so
//! the gate tracks driver growth automatically.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (both its source, to
//! assemble, and `aeon/s4.bin`, to compare). Absent it SKIPS green unless
//! `SIGIL_STRICT_GATE=1`. Mirrors `m1b_gate.rs` / `m1c_vector_table.rs` /
//! `m1d_rom.rs`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test m0_regions
//! ```

use std::path::PathBuf;

use sigil_harness::{assemble_full_rom, region_at_lma, REGION_A_LMA, REGION_B_LMA};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

#[test]
fn full_build_reproduces_sound_driver_regions() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    // Full non-debug build, NO stubs.
    let linked = assemble_full_rom(&aeon).unwrap_or_else(|e| panic!("{e}"));

    for (label, lma) in [("Region A", REGION_A_LMA), ("Region B", REGION_B_LMA)] {
        let bytes = region_at_lma(&linked, lma).unwrap_or_else(|| {
            let present: Vec<String> = linked
                .sections
                .iter()
                .map(|s| format!("{}@{:#x}({}B)", s.name, s.lma, s.bytes.len()))
                .collect();
            panic!("{label}: no linked section at LMA {lma:#x}; present: {present:?}");
        });
        // A zero-byte section would make the compare loop below pass vacuously.
        // The driver regions are always non-empty (Region A ≈ 5.9 KB); guard so a
        // regression that emptied one fails here instead of silently passing.
        assert!(!bytes.is_empty(), "{label}: linked section at {lma:#x} is empty");
        let start = lma as usize;
        let end = start + bytes.len();
        assert!(
            end <= refrom.len(),
            "{label}: window [{start:#x}..{end:#x}) exceeds reference ROM ({} B)",
            refrom.len()
        );
        let win = &refrom[start..end];
        if let Some(i) = (0..bytes.len()).find(|&i| bytes[i] != win[i]) {
            panic!(
                "{label} ({} B @ {lma:#x}) diverged at region offset {i:#x} \
                 (ROM {:#x}): sigil {:#04x} != ref {:#04x}",
                bytes.len(),
                start + i,
                bytes[i],
                win[i]
            );
        }
    }
}
