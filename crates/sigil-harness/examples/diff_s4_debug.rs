//! M1.D T5 exploration: assemble the `__DEBUG__` ROM in Sigil and diff it against
//! the deliberately-built debug reference (`aeon/s4.debug.bin`, produced by
//! `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4`). Reports the assembled
//! length, the four expected convsym/fixheader header bytes, and the first
//! divergences beyond those — the first-diff triage input.
//!
//! ```text
//! AEON_DIR=/path/to/aeon cargo run -q -p sigil-harness --example diff_s4_debug
//! ```
use std::path::PathBuf;

use sigil_harness::{assemble_full_rom_debug, CONVSYM_REWRITTEN};

// convsym rewrites the checksum (0x18E) and the low half of the ROM-end pointer
// (0x1A6); fixheader re-checksums. These are the only *legitimate* diffs (A1/A2
// scope decision). NOTE: this example diffs against the plain (non-debug) 4-byte
// set for a quick triage pass; `m1d_debug_rom`'s gate uses the debug-specific
// 5-byte `CONVSYM_REWRITTEN_DEBUG` set (the deb2 append is larger and pushes the
// ROM-end pointer over a byte boundary), so a few extra "unexpected" diffs at
// $1A5 are expected noise here — this tool is a triage aid, not the gate.

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    );
    let refrom = std::fs::read(aeon.join("s4.debug.bin"))
        .expect("read aeon/s4.debug.bin (build it: DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4)");

    let linked = assemble_full_rom_debug(&aeon).expect("assemble debug");
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).expect("read map"))
        .expect("load map");
    let rom = sigil_link::emit_rom(&linked, &map).expect("emit_rom");

    println!("sigil debug ROM: {} bytes ({:#x})", rom.len(), rom.len());
    println!("ref   debug ROM: {} bytes ({:#x})", refrom.len(), refrom.len());

    // WINDOW=0x2320,0x40 dumps a hex+ascii window of both ROMs and exits.
    if let Ok(w) = std::env::var("WINDOW") {
        let (s, n) = w.split_once(',').unwrap();
        let s = usize::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
        let n = usize::from_str_radix(n.trim_start_matches("0x"), 16).unwrap();
        let show = |label: &str, b: &[u8]| {
            let hex: Vec<String> = b[s..(s + n).min(b.len())].iter().map(|x| format!("{x:02x}")).collect();
            let asc: String = b[s..(s + n).min(b.len())].iter().map(|&x| if (32..127).contains(&x) { x as char } else { '.' }).collect();
            println!("{label} {s:#x}: {}", hex.join(" "));
            println!("{label} chars : {asc}");
        };
        show("SIG", &rom);
        show("REF", &refrom);
        return;
    }

    let n = rom.len().min(refrom.len());
    let diffs: Vec<usize> = (0..n).filter(|&i| rom[i] != refrom[i]).collect();
    let unexpected: Vec<usize> =
        diffs.iter().copied().filter(|i| !CONVSYM_REWRITTEN.contains(i)).collect();

    println!("total diffs over [0,{n:#x}): {}", diffs.len());
    for &i in CONVSYM_REWRITTEN {
        let m = if i < n && rom[i] != refrom[i] { "DIFF" } else { "same" };
        println!("  convsym byte {i:#x}: sigil {:#04x} ref {:#04x} [{m}]", rom.get(i).copied().unwrap_or(0), refrom.get(i).copied().unwrap_or(0));
    }
    println!("UNEXPECTED diffs (beyond the 4 convsym bytes): {}", unexpected.len());
    for &i in unexpected.iter().take(40) {
        println!("  {i:#x}: sigil {:#04x} != ref {:#04x}", rom[i], refrom[i]);
    }
    if unexpected.len() > 40 {
        println!("  … and {} more", unexpected.len() - 40);
    }
}
