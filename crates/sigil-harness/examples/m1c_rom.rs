//! M1.C/M1.D full-ROM emit: assemble the REAL aeon `games/sonic4/main.asm`,
//! resolve jmp/jsr widths, link, place via the canonical `sigil.map.toml`, apply
//! the header checksum, and diff the result against `aeon/s4.bin`.
//!
//! Reports the first differing byte offset (mapped to the covering section) or a
//! sha256 match. Run:
//!   AEON_DIR=/home/volence/sonic_hacks/aeon cargo run -q -p sigil-harness --example m1c_rom
use std::path::PathBuf;

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::{Cpu, SymbolTable};

fn main() {
    let aeon = PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".into()),
    );
    let root = aeon.join("games/sonic4/main.asm");
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");

    let defines: Vec<(String, i64)> = vec![("SOUND_DRIVER_ENABLED".to_string(), 1)];
    let opts = Options {
        initial_cpu: Cpu::M68000,
        defines,
        include_root: Some(aeon.clone()),
    };

    let module = match assemble_root(&root, &opts) {
        Ok(m) => m,
        Err(d) => {
            println!("ASSEMBLE FAILED: {} diagnostics", d.len());
            for x in d.iter().take(10) {
                println!("  {:?}", x.message);
            }
            return;
        }
    };
    println!("assembled {} sections", module.sections.len());

    let stubs = SymbolTable::new();
    let resolved = match sigil_link::resolve_layout(&module.sections, &stubs, true) {
        Ok(r) => r,
        Err(d) => {
            println!("RESOLVE_LAYOUT FAILED: {} diagnostics", d.len());
            for x in d.iter().take(10) {
                println!("  {:?}", x.message);
            }
            return;
        }
    };

    let linked = match sigil_link::link(&resolved, &stubs) {
        Ok(l) => l,
        Err(d) => {
            println!("LINK FAILED: {} diagnostics", d.len());
            for x in d.iter().take(10) {
                println!("  {:?}", x.message);
            }
            return;
        }
    };

    let map_src = match std::fs::read_to_string(&map_path) {
        Ok(s) => s,
        Err(e) => {
            println!("cannot read map {}: {e}", map_path.display());
            return;
        }
    };
    let map = match sigil_link::load_map(&map_src) {
        Ok(m) => m,
        Err(e) => {
            println!("MAP LOAD FAILED: {e}");
            return;
        }
    };

    let rom = match sigil_link::emit_rom(&linked, &map) {
        Ok(r) => r,
        Err(e) => {
            println!("EMIT_ROM FAILED: {e}");
            return;
        }
    };
    println!("emitted ROM: {} bytes", rom.len());

    let refrom = match std::fs::read(aeon.join("s4.bin")) {
        Ok(r) => r,
        Err(e) => {
            println!("cannot read reference s4.bin: {e}");
            return;
        }
    };
    println!("reference ROM: {} bytes", refrom.len());

    if rom == refrom {
        println!("*** BYTE-IDENTICAL to aeon/s4.bin ***");
        return;
    }

    let n = rom.len().min(refrom.len());
    if let Some(i) = (0..n).find(|&i| rom[i] != refrom[i]) {
        // Map the offset back to the covering linked section.
        let sec = linked
            .sections
            .iter()
            .filter(|s| (s.lma as usize) <= i && i < s.lma as usize + s.bytes.len())
            .min_by_key(|s| i - s.lma as usize);
        let where_ = match sec {
            Some(s) => format!("section `{}` (lma {:#x}, +{:#x})", s.name, s.lma, i - s.lma as usize),
            None => "no section covers this offset (gap-fill byte)".to_string(),
        };
        println!(
            "FIRST DIFF at {i:#x}: sigil {:#04x} != ref {:#04x} — {where_}",
            rom[i], refrom[i]
        );
    }
    if rom.len() != refrom.len() {
        println!("LENGTH differs: sigil {} vs ref {}", rom.len(), refrom.len());
    }
}
