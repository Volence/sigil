//! `derive_offcanon` — P4a: re-derive the off-canonical size tables from SIGIL'S
//! OWN resolved layout, retiring the asl-`.lst` parse (`capture_offcanon`, kill-list row
//! 34). Reads each boundary label's ROM address = `section.lma + label.offset` off the
//! resolved layout of a described shape (`sigil build --project`; LMA-correct for the
//! phased z80 idle; section-END markers synthesized from the owning section's geometry,
//! [`sigil_harness::repin::boundary_table_of`]) — no asl, no listing file.
//!
//! ```text
//! SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon derive_offcanon <out_dir> <target>=<game>:<shape>:<golden>...
//! ```
//! Each `<target>` names a table (`<out_dir>/<target>.txt`), the described shape whose
//! layout it is read off, and the golden blob under `golden/` it is tied to. The label set
//! of each table is the committed table's own (`golden/offcanonical_sizes/<target>.txt`):
//! a re-derivation measures the boundaries the freeze recorded, it does not choose them.
//! The target-to-shape table lives in the caller (`golden/derive_offcanonical_sizes.sh`). The written
//! tables carry a sigil-native provenance header (no listing_stamp / no asl); the numbers
//! are the same boundary addresses (proven == the committed tables by
//! `native_offcanonical_placement::*_size_table_rederives_native`) — the SOURCE is what
//! changes: sigil's resolve, not an asl listing. These are golden provenance; the sizes
//! re-derive from sigil on any ruled post-flip golden re-baseline (see the handoff note).

use std::collections::HashSet;
use std::path::PathBuf;

use sigil_harness::native;
use sigil_harness::repin::{boundary_table_of, parse_size_table};
use sigil_harness::shapes::{self, Shape};
use sigil_harness::stdout::println;

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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((out_dir, specs)) = args.split_first() else {
        eprintln!("usage: derive_offcanon <out_dir> <target>=<game>:<shape>:<golden>...");
        return std::process::ExitCode::from(2);
    };
    let out_dir = PathBuf::from(out_dir);
    if specs.is_empty() {
        eprintln!("ERROR: no <target>=<game>:<shape>:<golden> rows: nothing to derive");
        return std::process::ExitCode::from(2);
    }
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("ERROR: create {}: {e}", out_dir.display());
        return std::process::ExitCode::from(2);
    }
    for spec in specs {
        let parsed = spec
            .split_once('=')
            .and_then(|(t, rest)| {
                let mut it = rest.splitn(3, ':');
                Some((t, it.next()?, it.next()?, it.next()?))
            })
            .filter(|(t, g, s, b)| ![t, g, s, b].iter().any(|x| x.is_empty()));
        let Some((stem, game, row, golden_name)) = parsed else {
            eprintln!("ERROR: `{spec}` is not <target>=<game>:<shape>:<golden>");
            return std::process::ExitCode::from(2);
        };
        let committed_path = manifest.join("golden/offcanonical_sizes").join(format!("{stem}.txt"));
        let want: HashSet<String> = match std::fs::read_to_string(&committed_path) {
            Ok(t) => parse_size_table(&t).into_keys().collect(),
            Err(e) => {
                eprintln!("ERROR: read the committed table {}: {e}", committed_path.display());
                return std::process::ExitCode::from(2);
            }
        };
        let shape = Shape::new(game, row);
        let table = match shapes::resolved_layout(&aeon, &shape).and_then(|r| boundary_table_of(&r, &want)) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("ERROR: derive {stem} ({}): {e}", shape.name());
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
        // EOR from the freshly-derived layout itself (`EndOfRom` is a label of every
        // committed table, so `boundary_table_of` has already refused a layout without it).
        let Some(eor) = table.get("EndOfRom").map(|a| *a as usize) else {
            eprintln!("ERROR: derive {stem}: the committed table names no `EndOfRom` boundary");
            return std::process::ExitCode::from(1);
        };
        let anchor = native::assembled_anchor_crc(&golden, eor);
        let mut out = String::new();
        out.push_str("# GENERATED, derive_offcanon. SIGIL-NATIVE region-boundary addresses (Stage-3 P4a).\n");
        out.push_str("# Derived from sigil's OWN resolved layout (each label's ROM LMA), NOT an asl\n");
        out.push_str("# listing, THE LAST asl-derived constants retire here (kill-list rows 34/95).\n");
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
    println!("== done, sigil-native size tables derived (asl-free); commit them =");
    std::process::ExitCode::SUCCESS
}
