//! Emit the full non-debug Sonic 4 ROM assembled entirely by Sigil, ready to run
//! in an emulator. This is the assembled ROM (the convsym debug-symbol append
//! that `aeon/build.sh` adds is out of scope — it is not executed), with Sigil's
//! own valid header checksum. Byte-identical to `aeon/s4.bin` over the code/data.
//!
//! ```text
//! AEON_DIR=/path/to/aeon SIGIL_OUT=/path/to/sigil_s4.bin \
//!   cargo run -q -p sigil-harness --example emit_s4_rom
//! ```
use std::path::PathBuf;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, SymbolTable};

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    );
    let out = PathBuf::from(
        std::env::var("SIGIL_OUT")
            .unwrap_or_else(|_| "/home/volence/sonic_hacks/sigil_s4.bin".into()),
    );
    let root = aeon.join("games/sonic4/main.asm");
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");

    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines: vec![("SOUND_DRIVER_ENABLED".to_string(), 1)],
        include_root: Some(aeon.clone()),
    };

    let module = assemble_root(&root, &opts).expect("assemble");
    let stubs = SymbolTable::new();
    let resolved =
        sigil_link::resolve_layout(&module.sections, &stubs, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &stubs).expect("link");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).expect("read map"))
        .expect("load map");
    let rom = sigil_link::emit_rom(&linked, &map).expect("emit_rom");

    std::fs::write(&out, &rom).expect("write ROM");
    println!("wrote {} bytes to {}", rom.len(), out.display());
}
