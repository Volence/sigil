//! Emit the full Sonic 4 ROM assembled entirely by Sigil, ready to run in an
//! emulator. This is the **assembled** ROM (the `convsym` debug-symbol append
//! that `aeon/build.sh` adds is out of scope — it is not executed), with Sigil's
//! own valid header checksum. Byte-identical to `aeon/s4.bin` (non-debug) /
//! `aeon/s4.debug.bin` (debug) over the code/data.
//!
//! `DEBUG=1` assembles the `__DEBUG__` build (the assert / KDebug / `%<…>`
//! debugger machinery) instead of the default non-debug build.
//!
//! ```text
//! # non-debug (default → sigil_s4.bin):
//! AEON_DIR=/path/to/aeon cargo run -q -p sigil-harness --example emit_s4_rom
//! # __DEBUG__ (default → sigil_s4_debug.bin):
//! DEBUG=1 AEON_DIR=/path/to/aeon cargo run -q -p sigil-harness --example emit_s4_rom
//! # override the output path:
//! SIGIL_OUT=/tmp/rom.bin DEBUG=1 AEON_DIR=… cargo run -q -p sigil-harness --example emit_s4_rom
//! ```
use std::path::PathBuf;

use sigil_harness::{assemble_full_rom, assemble_full_rom_debug};

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    );
    let debug = std::env::var("DEBUG").is_ok_and(|v| v == "1");
    let out = PathBuf::from(std::env::var("SIGIL_OUT").unwrap_or_else(|_| {
        let name = if debug { "sigil_s4_debug.bin" } else { "sigil_s4.bin" };
        format!("/home/volence/sonic_hacks/{name}")
    }));

    let linked = if debug {
        assemble_full_rom_debug(&aeon)
    } else {
        assemble_full_rom(&aeon)
    }
    .expect("assemble");

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).expect("read map"))
        .expect("load map");
    let rom = sigil_link::emit_rom(&linked, &map).expect("emit_rom");

    std::fs::write(&out, &rom).expect("write ROM");
    println!(
        "wrote {} bytes ({}) to {}",
        rom.len(),
        if debug { "__DEBUG__" } else { "non-debug" },
        out.display()
    );
}
