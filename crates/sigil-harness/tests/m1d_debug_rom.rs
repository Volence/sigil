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

use sigil_harness::assemble_full_rom_debug;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The only offsets at which Sigil's assembled debug ROM legitimately differs
/// from `s4.debug.bin`: the checksum (`$18E/$18F`) and the ROM-end pointer at
/// `$1A4`, both rewritten by the out-of-scope `convsym -a`/`fixheader` post-
/// steps. `convsym -a` appends the (larger, under `__DEBUG__`) deb2 symbol table
/// and rewrites `EndOfRom-1` to the POST-append size ($0700E5 in the reference
/// vs Sigil's assembled $0673A1); at the current debug build that pointer now
/// differs in THREE bytes ($1A5/$1A6/$1A7 — the append pushed it over a byte
/// boundary), where the earlier smaller pin differed in only $1A6/$1A7.
const CONVSYM_REWRITTEN: &[usize] = &[0x18E, 0x18F, 0x1A5, 0x1A6, 0x1A7];

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
    const DEBUG_ASSEMBLED_LEN: usize = 0x673A2;
    assert_eq!(
        rom.len(),
        DEBUG_ASSEMBLED_LEN,
        "sigil debug ROM length changed (dropped/added section?); expected {DEBUG_ASSEMBLED_LEN:#x}"
    );
    assert!(rom.len() <= refrom.len(), "sigil debug ROM {} longer than ref {}", rom.len(), refrom.len());

    let unexpected: Vec<String> = (0..rom.len())
        .filter(|&i| rom[i] != refrom[i] && !CONVSYM_REWRITTEN.contains(&i))
        .map(|i| format!("{i:#x} (sigil {:#04x} != ref {:#04x})", rom[i], refrom[i]))
        .collect();
    assert!(
        unexpected.is_empty(),
        "sigil debug ROM diverges from the assembled reference at {} unexpected offset(s): {}",
        unexpected.len(),
        unexpected.join(", ")
    );
    for &i in CONVSYM_REWRITTEN {
        assert!(
            i < rom.len() && rom[i] != refrom[i],
            "expected convsym-rewritten byte at {i:#x} to differ, but it matched"
        );
    }
}
