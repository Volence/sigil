//! `capture_offcanon` — freeze one off-canonical target's asl-derived region-boundary
//! addresses (Flip Stage 2 · S1.2, overseer condition 1). Reuses the authoritative
//! `repin::parse_listing` so the capture can never drift from how the pins are read.
//!
//! ```text
//! capture_offcanon <target> <lst_path> <golden_bin_path> <out_path>
//! ```
//!
//! Emits a committed, dependency-free text table (`# provenance…` header + `LABEL
//! 0xADDR` rows) of every region-boundary label named in `repin.toml` (+ the
//! off-canonical extras) that EXISTS in this target's listing. A region's declared
//! size is `addr[end] − addr[start]`, computed by the driver at build time; absent
//! labels (e.g. sound-off `sound_api`) are simply omitted. The header records the
//! golden CRC these addresses reproduce — the provenance tie condition 1 requires.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sigil_harness::repin::{load_manifest, parse_listing};

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        eprintln!("usage: capture_offcanon <target> <lst_path> <golden_bin_path> <out_path>");
        return std::process::ExitCode::from(2);
    }
    let (target, lst_path, golden_path, out_path) =
        (&args[0], PathBuf::from(&args[1]), PathBuf::from(&args[2]), PathBuf::from(&args[3]));

    let lst_txt = match std::fs::read_to_string(&lst_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {e}", lst_path.display());
            return std::process::ExitCode::from(2);
        }
    };
    let listing = match parse_listing(&lst_txt) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("parse {}: {e}", lst_path.display());
            return std::process::ExitCode::from(2);
        }
    };
    let golden = match std::fs::read(&golden_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cannot read golden {}: {e}", golden_path.display());
            return std::process::ExitCode::from(2);
        }
    };
    let crc = crc32(&golden);
    let golden_name = golden_path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

    // The complete region-boundary label set from repin.toml (start + end), plus the
    // off-canonical extras + spine anchors.
    let manifest_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/repin.toml"));
    let manifest = match load_manifest(manifest_src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("load repin.toml: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let mut labels: BTreeMap<String, ()> = BTreeMap::new();
    for r in &manifest.regions {
        labels.insert(r.start.clone(), ());
        if let Some(e) = &r.end {
            labels.insert(e.clone(), ());
        }
    }
    for extra in [
        // Config-A extras (game_debug, sound_debug); Config-B extra (z80_init).
        "Debug_MusicToggle",
        "Section_Init",
        "Sound_DebugMirror",
        "CarrierMaskTableZ",
        "Z80_IdleProgram",
        "Z80_IdleProgram_End",
        // the label-less AS data blob (collision heightmaps + level data tail): a
        // real listing label the chainer needs as a frozen anchor to ORDER it after
        // the emp anim regions (its baked residual address mis-sorts it into them).
        "HeightMaps",
        // spine anchors the chainer keys on
        "Vectors",
        "Checksum",
        "EntryPoint",
        "BootData",
        "ObjCodeBase",
        "EndOfRom",
        "NullInterrupt",
    ] {
        labels.insert(extra.to_string(), ());
    }

    let mut present: Vec<(String, u32)> = Vec::new();
    for name in labels.keys() {
        if let Ok(addr) = listing.get(name) {
            present.push((name.clone(), addr));
        }
    }
    present.sort();

    let mut out = String::new();
    out.push_str("# GENERATED — capture_offcanon. FROZEN asl-derived region-boundary addresses.\n");
    out.push_str("# These are PERISHABLE (no asl after the flip) and are GOLDEN PROVENANCE: the\n");
    out.push_str("# declared per-region sizes (addr[end]-addr[start]) reproduce the golden below.\n");
    out.push_str(&format!("# target={target}\n"));
    out.push_str(&format!("# reproduces_golden={golden_name}\n"));
    out.push_str(&format!("# golden_crc32={crc:08x}\n"));
    out.push_str(&format!("# assembled_end={:#x}\n", listing.end_addr));
    out.push_str(&format!("# listing_stamp={}\n", listing.stamp));
    out.push_str(&format!("# labels={}\n", present.len()));
    for (name, addr) in &present {
        out.push_str(&format!("{name} {addr:#x}\n"));
    }
    if let Err(e) = std::fs::write(&out_path, &out) {
        eprintln!("cannot write {}: {e}", out_path.display());
        return std::process::ExitCode::from(2);
    }
    println!(
        "   {target}: {} boundary labels, end={:#x}, golden {golden_name} crc {crc:08x} -> {}",
        present.len(),
        listing.end_addr,
        out_path.display()
    );
    std::process::ExitCode::SUCCESS
}
