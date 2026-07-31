//! `derive_offcanon` — P4a: re-derive the four off-canonical size tables from SIGIL'S
//! OWN resolved layout, retiring the asl-`.lst` parse (`capture_offcanon`, kill-list row
//! 34). Reads each boundary label's ROM address = `section.lma + label.offset` off a
//! native frozen-chain resolve (LMA-correct for the phased z80 idle; section-END markers
//! synthesized from the owning section's geometry) — no asl, no listing file.
//!
//! ```text
//! SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon derive_offcanon [<out_dir>]
//! ```
//! `<out_dir>` defaults to `crates/sigil-harness/golden/offcanonical_sizes`. The written
//! tables carry a sigil-native provenance header (no listing_stamp / no asl); the numbers
//! are the same boundary addresses (proven == the committed tables by
//! `native_offcanonical_placement::*_size_table_rederives_native`) — the SOURCE is what
//! changes: sigil's resolve, not an asl listing. These are golden provenance; the sizes
//! re-derive from sigil on any ruled post-flip golden re-baseline (see the handoff note).

use std::path::PathBuf;

use sigil_harness::native::{self, GameProfile};

fn main() -> std::process::ExitCode {
    let aeon = match std::env::var("AEON_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => {
            eprintln!("ERROR: set AEON_DIR to the aeon tree.");
            return std::process::ExitCode::from(2);
        }
    };
    if std::env::var("SIGIL_EMIT").map(|v| v.is_empty()).unwrap_or(true) {
        eprintln!("ERROR: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (sound-on builds need it).");
        return std::process::ExitCode::from(2);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("golden/offcanonical_sizes"));
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("ERROR: create {}: {e}", out_dir.display());
        return std::process::ExitCode::from(2);
    }

    // (target file stem, golden blob under golden/, the profile).
    let targets: Vec<(&str, &str, GameProfile)> = vec![
        ("demo", "demo.bin", native::demo_profile(false)),
        ("demo_debug", "demo.debug.bin", native::demo_profile(true)),
        ("config_a", "config_a.bin", native::config_a_profile()),
        ("config_b", "config_b.bin", native::config_b_profile()),
    ];

    for (stem, golden_name, profile) in &targets {
        let table = match native::derive_frozen_table(&aeon, profile) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("ERROR: derive {stem}: {e}");
                return std::process::ExitCode::from(1);
            }
        };
        // Provenance: tie to the committed golden blob (full-file CRC + the header-neutral
        // assembled anchor over [0, EndOfRom) — the invariant that never moves).
        let golden_path = manifest.join("golden").join(golden_name);
        let golden = match std::fs::read(&golden_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("ERROR: read golden {}: {e}", golden_path.display());
                return std::process::ExitCode::from(2);
            }
        };
        let full_crc = native::crc32(&golden);
        let eor = profile.assembled_len;
        let anchor = native::assembled_anchor_crc(&golden, eor);

        let mut out = String::new();
        out.push_str("# GENERATED — derive_offcanon. SIGIL-NATIVE region-boundary addresses (Stage-3 P4a).\n");
        out.push_str("# Derived from sigil's OWN resolved layout (each label's ROM LMA), NOT an asl\n");
        out.push_str("# listing — THE LAST asl-derived constants retire here (kill-list rows 34/95).\n");
        out.push_str("# Section-END markers (`*_End`) are synthesized from the owning section's resolved\n");
        out.push_str("# geometry (lma + image_len); the phased z80 idle reports its ROM LMA. The declared\n");
        out.push_str("# per-region sizes (addr[end]-addr[start]) reproduce the golden below. On any ruled\n");
        out.push_str("# post-flip golden re-baseline these re-derive from sigil (nothing requires asl).\n");
        out.push_str(&format!("# target={stem}\n"));
        out.push_str(&format!("# reproduces_golden={golden_name}\n"));
        out.push_str(&format!("# golden_crc32={full_crc:08x}\n"));
        out.push_str(&format!("# assembled_anchor={anchor:08x}\n"));
        out.push_str(&format!("# assembled_end={eor:#x}\n"));
        out.push_str(&format!("# labels={}\n", table.len()));
        for (name, addr) in &table {
            out.push_str(&format!("{name} {addr:#x}\n"));
        }
        let out_path = out_dir.join(format!("{stem}.txt"));
        if let Err(e) = std::fs::write(&out_path, &out) {
            eprintln!("ERROR: write {}: {e}", out_path.display());
            return std::process::ExitCode::from(2);
        }
        println!(
            "   {stem}: {} boundary labels, end={eor:#x}, golden {golden_name} full={full_crc:08x} anchor={anchor:08x} -> {}",
            table.len(),
            out_path.display()
        );
    }
    println!("== done — sigil-native size tables derived (asl-free); commit them =");
    std::process::ExitCode::SUCCESS
}
