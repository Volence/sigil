//! M1.D T5 (A2): prove `sigil` assembles the REAL Aeon `main.asm` under
//! `__DEBUG__` and emits a full ROM BYTE-EXACT to the assembled debug reference.
//! The debug build pulls in `debugger.asm`'s assertion / KDebug / `__FSTRING`
//! error-message machinery (`switch`/`lowstring`/`substr`/`val`/`.ATTRIBUTE`,
//! `cmp`→`cmpa`, string-`.`-local macro args, `%<…>` decoded strings) — none of
//! which the non-debug ROM (`m1d_rom`) exercises.
//!
//! ## The debug reference (A2 scope inherits T4's decision)
//!
//! The reference is built DELIBERATELY (it is NOT the shipped `s4.bin`):
//! ```text
//! cd aeon && DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4
//! cp s4.bin s4.debug.bin && cp s4.lst s4.debug.lst   # capture
//! # then restore the non-debug reference so the other gates stay green:
//! ./build.sh sonic4        # (or restore s4.bin/s4.lst from a backup)
//! ```
//! See `golden/PROVENANCE.md`. `build.sh` post-processes the assembled ROM with
//! `convsym … -a` (appends the MD-Debugger `deb2` symbol table — larger under
//! `__DEBUG__` — and rewrites two header fields) then `fixheader`; that append is
//! out of scope (M1.B models `convsym` as a no-op), so the target of
//! byte-exactness is the ASSEMBLED debug ROM. Sigil's ROM and `s4.debug.bin` are
//! therefore identical over `[0, emit_len)` EXCEPT the four `convsym`/`fixheader`-
//! rewritten header bytes — the same shape as `m1d_rom`, adjusted for the debug
//! build's larger `EndOfRom`.
//!
//! REFERENCE-DEPENDENT: needs `aeon/s4.debug.bin`. Absent it SKIPS green unless
//! `SIGIL_STRICT_GATE=1`. (Note: the non-debug gates need `aeon/s4.bin`, so both
//! references must be present on disk simultaneously — the deliberate capture
//! above keeps `s4.debug.bin` alongside the restored non-debug `s4.bin`.)
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-harness --test m1d_debug_rom
//! ```

use std::path::PathBuf;

use sigil_harness::{assemble_full_rom_debug, assert_rom_matches_convsym};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

// The only offsets at which Sigil's assembled debug ROM legitimately differs
// from `s4.debug.bin` are the checksum (`$18E..$190`) and ROM-end pointer
// (`$1A4..$1A8`) header fields, rewritten by the out-of-scope
// `convsym -a`/`fixheader` post-steps (`convsym -a` appends the larger
// `__DEBUG__` deb2 symbol table and bumps `EndOfRom-1` to the POST-append
// size). WHICH pointer bytes differ shifts with the append size (historically
// $1A5/$1A6/$1A7, four bytes since the tranche-9 PerFrame deletion), so
// `assert_rom_matches_convsym` DERIVES the allowlist per comparison, confined
// to the two semantic fields — the tranche-10 D-T10.6 replacement for the
// pinned `CONVSYM_REWRITTEN_DEBUG` array that re-pinned on every append-size
// change.

#[test]
fn full_debug_rom_matches_assembled_reference() {
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

    let linked = assemble_full_rom_debug(&aeon).unwrap_or_else(|e| panic!("{e}"));

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));

    // Pin the assembled debug length (EndOfRom of the DEBUG build at the T5 pin),
    // so a regression that drops a trailing section can't silently pass the diff
    // check. Larger than the non-debug `0x658B4` — the debugger code adds bytes.
    // Sourced from `sigil_harness::pins` (regenerate via `repin`).
    let debug_assembled_len = sigil_harness::pins::DEBUG_ASSEMBLED_LEN;
    assert_rom_matches_convsym(&rom, &refrom, debug_assembled_len, "sigil debug");
}
