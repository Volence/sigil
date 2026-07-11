//! M1.D T4 (A1): prove `sigil` assembles the REAL Aeon `games/sonic4/main.asm`
//! and emits a full ROM BYTE-EXACT to the assembled reference — the entire
//! non-debug pipeline (assemble → resolve_layout → link → load_map → emit_rom),
//! with NO stubs (the full build defines everything itself).
//!
//! ## What "assembled reference" means (A1 scope, decided 2026-07-05)
//!
//! `aeon/build.sh` post-processes the assembled ROM: after `asl`+`p2bin` it runs
//! `convsym … -a`, which APPENDS the MD-Debugger `deb2` symbol table (~34 KB) and
//! rewrites two header fields, then `fixheader` re-checksums the appended image.
//! That appended symbol table is debug tooling (not executed, not game content —
//! the MD-Debugger analogue of an ELF `.symtab`), and Sigil's `emit_rom`
//! deliberately models `convsym` as a no-op (an M1.B decision). So the target of
//! byte-exactness is the ASSEMBLED ROM, which Sigil reproduces exactly.
//!
//! Consequently Sigil's ROM and the pinned `s4.bin` are identical over
//! `[0, emit_len)` EXCEPT the two fields `convsym`/`fixheader` rewrote:
//! `0x18E..0x190` (header checksum, computed over the post-append image, not
//! ours) and `0x1A6..0x1A8` (low half of the `dc.l EndOfRom-1` ROM-end pointer,
//! which `convsym` bumps to the post-append end `0x6E13D` vs our `0x658B3`).
//! This test asserts the diff set is EXACTLY those four bytes: any assembler
//! regression introduces a new differing offset and fails. (Full-pipeline
//! equivalence — `sigil emit_rom` + real `convsym` + `fixheader` == `s4.bin`,
//! sha256 `605631da…` — was verified out-of-band during T4 bring-up.)
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree. Absent (e.g. GitHub CI) it
//! SKIPS green — unless SIGIL_STRICT_GATE=1 makes a missing reference a hard
//! failure. Mirrors `m1b_gate.rs` / `m1c_vector_table.rs`.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test m1d_rom
//! ```

use std::path::PathBuf;

use sigil_harness::{assemble_full_rom, assert_rom_matches_convsym};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

// The only offsets at which Sigil's assembled ROM legitimately differs from the
// pinned `s4.bin` are the checksum + ROM-end header fields rewritten by the
// out-of-scope `convsym`/`fixheader` post-steps (see module header above).
// `assert_rom_matches_convsym` DERIVES that allowlist per comparison, confined
// to the two semantic fields (`CHECKSUM_FIELD_RANGE`/`ROM_END_FIELD_RANGE`) —
// the tranche-10 D-T10.6 replacement for the pinned `CONVSYM_REWRITTEN` array.

#[test]
fn full_rom_matches_assembled_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: aeon/s4.bin");
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    // Non-debug config, mirroring build.sh's default ASFLAGS: SOUND_DRIVER_ENABLED
    // on, __DEBUG__ off. No stubs — the full build defines everything.
    let linked = assemble_full_rom(&aeon).unwrap_or_else(|e| panic!("{e}"));

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    // Pin the assembled length. `emit_rom` ends the image at the last non-empty
    // section byte with NO trailing padding, so a regression that DROPS a trailing
    // section would shrink the ROM while leaving the (header-adjacent) prefix — and
    // the four expected diffs — byte-identical, silently passing the diff check
    // below. `EndOfRom` at the T0.0 clean-tree pin (aeon 9bacc93) is `0x658B4`;
    // this is the assembled (pre-convsym-append) ROM length. Pinned like the
    // `m1c_vector_table` stub addresses.
    const ASSEMBLED_LEN: usize = 0x658B4;
    assert_rom_matches_convsym(&rom, &refrom, ASSEMBLED_LEN, "sigil");
}
