//! G0a Task 6, gate 2: every ROM byte is covered by exactly one line row.
//!
//! The rows are what Oracle steps on and what Aurora's go-to-source jumps through, so the
//! gate reads them as a consumer does, from the sidecar beside the ROM file, and checks
//! them against the ROM FILE and the listing, never against the exporter's own idea of
//! the image:
//!
//!  * every byte of `[0, len(rom file))` is in exactly one row: no gap, no overlap,
//!    nothing past the end;
//!  * the deb2 appendix rows are exactly `[EndOfRom, len)`, where `EndOfRom` is the
//!    listing's address for the label the appendix is appended at (the assembled image's
//!    length), and the ROM holds the deb2 magic there; a shape without the appendix has
//!    `EndOfRom == len` and no deb2 row;
//!  * the checksum row is exactly the field `sigil_link::apply_header_checksum` writes,
//!    and the ROM's word there IS the header checksum of the file;
//!  * a fill or org-gap row covers uniform bytes (padding, not code mislabelled);
//!  * a source row names a `files` row and a line from 1; a no-source row names neither.
//!
//! Loud on unmeasurable: a sidecar with no rows, a listing without `EndOfRom`, or a ROM
//! too short for a header fails the gate rather than passing it.
//!
//! Real `sigil build` runs over a COPY of the reference tree (`AEON_DIR`), because
//! `sigil build` rewrites files in the tree it builds.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

static LOCK: Mutex<()> = Mutex::new(());
static TREE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The seven shapes `scripts/step0_shapes_cmp.sh` proves byte-identical, by name and flags.
const SHAPES: [(&str, &[&str]); 7] = [
    ("plain", &[]),
    ("debug", &["--debug"]),
    ("demo", &["--game", "demo"]),
    ("demo_debug", &["--game", "demo", "--debug"]),
    ("config_a", &["--config-a"]),
    ("config_b", &["--config-b"]),
    ("lean", &["--lean"]),
];

/// The deb2 appendix's first two bytes (`native::DEB2_MAGIC`).
const DEB2_MAGIC: [u8; 2] = [0xDE, 0xB2];

/// The line rows the G0a spike measured at label granularity on the s4 plain shape; the
/// fragment walk must state more than that.
const SPIKE_LABEL_ROWS: usize = 2285;

fn tree() -> Option<(MutexGuard<'static, ()>, PathBuf)> {
    let guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let path = TREE.get_or_init(|| {
        let profile = sigil_harness::native::sonic4_profile(false);
        let aeon = sigil_harness::test_support::reference_tree_for_profile(&profile)?;
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("build_export_lines_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tree = dir.join("tree");
        let st = Command::new("cp").arg("-a").arg(&aeon).arg(&tree).status().expect("cp");
        assert!(st.success());
        Some(tree.canonicalize().unwrap())
    });
    path.clone().map(|p| (guard, p))
}

fn skip() {
    eprintln!("{}no aeon reference tree", sigil_harness::test_support::SKIP_MARKER);
}

/// One built shape: the ROM file, its listing and its parsed sidecar.
struct Built {
    rom: Vec<u8>,
    listing: String,
    export: serde_json::Value,
    sidecar_len: u64,
}

fn build_shape(tree: &Path, name: &str, flags: &[&str]) -> Built {
    let dir = tree.parent().unwrap().join("out");
    std::fs::create_dir_all(&dir).unwrap();
    let rom = dir.join(format!("{name}.bin"));
    let lst = dir.join(format!("{name}.lst"));
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["build", "--aeon"])
        .arg(tree)
        .args(flags)
        .arg("-o")
        .arg(&rom)
        .arg("--emit-lst")
        .arg(&lst)
        .output()
        .expect("spawn sigil");
    assert!(out.status.success(), "shape {name}: build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let sidecar = rom.with_extension("build.json");
    let json = std::fs::read_to_string(&sidecar).expect("sidecar");
    Built {
        rom: std::fs::read(&rom).expect("rom"),
        listing: std::fs::read_to_string(&lst).expect("listing"),
        export: serde_json::from_str(&json).expect("valid json"),
        sidecar_len: json.len() as u64,
    }
}

/// `EndOfRom`'s address from the listing's symbol table (`  EndOfRom : C3E7C C |`).
fn end_of_rom(listing: &str) -> Result<u64, String> {
    let row = listing
        .lines()
        .find_map(|l| l.strip_prefix(" EndOfRom : ").and_then(|r| r.strip_suffix(" C |")))
        .ok_or("the listing has no EndOfRom symbol row: the appendix start is unmeasurable")?;
    u64::from_str_radix(row.trim(), 16).map_err(|e| format!("EndOfRom `{row}`: {e}"))
}

/// One row as the gate reads it.
#[derive(Clone, Debug)]
struct Row {
    section: String,
    lma: u64,
    len: u64,
    file: u64,
    line: u64,
    kind: String,
}

fn rows_of(export: &serde_json::Value) -> Result<Vec<Row>, String> {
    let cols = export["lines"]["columns"].as_array().ok_or("no lines.columns")?;
    let want = ["vma", "lma", "len", "file", "line", "kind"];
    if cols.len() < 6 || cols.iter().zip(want).any(|(c, w)| c != w) {
        return Err(format!("unexpected columns {cols:?}"));
    }
    let secs = export["lines"]["sections"].as_object().ok_or("no lines.sections")?;
    let mut rows = Vec::new();
    for (name, s) in secs {
        for r in s["rows"].as_array().ok_or_else(|| format!("section {name} has no rows array"))? {
            let n = |i: usize| r[i].as_u64().ok_or_else(|| format!("section {name}: row {r} column {i} is not an integer"));
            rows.push(Row {
                section: name.clone(),
                lma: n(1)?,
                len: n(2)?,
                file: n(3)?,
                line: n(4)?,
                kind: r[5].as_str().ok_or_else(|| format!("row {r}: kind is not a string"))?.to_string(),
            });
        }
    }
    if rows.is_empty() {
        return Err("the sidecar has no line rows: nothing to measure".into());
    }
    rows.sort_by_key(|r| (r.lma, r.len));
    Ok(rows)
}

/// Gate 2 over one shape. Returns the problems found (empty = green).
fn gate2(rom: &[u8], end_of_rom: u64, export: &serde_json::Value, rows: &[Row]) -> Vec<String> {
    let mut bad = Vec::new();
    let len = rom.len() as u64;
    let n_files = export["files"].as_array().map_or(0, |f| f.len()) as u64;

    // Exactly once: walk the sorted rows.
    let mut at = 0u64;
    for r in rows {
        if r.len == 0 {
            bad.push(format!("empty row {r:?}"));
        }
        if r.lma > at {
            bad.push(format!("gap: [{at:#X}, {:#X}) has no row", r.lma));
        } else if r.lma < at {
            bad.push(format!("overlap: {r:?} starts before {at:#X}, where the previous row ends"));
        }
        at = at.max(r.lma + r.len);
    }
    if at < len {
        bad.push(format!("gap: [{at:#X}, {len:#X}) at the end of the ROM has no row"));
    }
    if at > len {
        bad.push(format!("rows run to {at:#X}, past the {len:#X}-byte ROM"));
    }

    // Each row's own claims.
    for r in rows {
        let bytes = &rom[(r.lma.min(len)) as usize..((r.lma + r.len).min(len)) as usize];
        match r.kind.as_str() {
            "code" | "data" => {
                if r.file >= n_files || r.line == 0 {
                    bad.push(format!("source row names no file/line: {r:?} ({n_files} files)"));
                }
            }
            "fill" | "org-gap" => {
                if (r.file, r.line) != (0, 0) {
                    bad.push(format!("no-source row carries a file/line: {r:?}"));
                }
                if bytes.iter().any(|b| *b != bytes[0]) {
                    bad.push(format!("{} row over non-uniform bytes: {r:?}", r.kind));
                }
            }
            "deb2" | "checksum" => {
                if (r.file, r.line) != (0, 0) {
                    bad.push(format!("no-source row carries a file/line: {r:?}"));
                }
            }
            other => bad.push(format!("unknown kind `{other}`: {r:?}")),
        }
    }

    // deb2: exactly [EndOfRom, len).
    let deb2: Vec<&Row> = rows.iter().filter(|r| r.kind == "deb2").collect();
    if end_of_rom > len {
        bad.push(format!("EndOfRom {end_of_rom:#X} lies past the {len:#X}-byte ROM"));
    } else if end_of_rom == len {
        if !deb2.is_empty() {
            bad.push(format!("no appendix (EndOfRom == len) but deb2 rows {deb2:?}"));
        }
    } else {
        if rom[end_of_rom as usize..].get(..2) != Some(&DEB2_MAGIC[..]) {
            bad.push(format!("no deb2 magic at EndOfRom {end_of_rom:#X}"));
        }
        let covered: u64 = deb2.iter().map(|r| r.len).sum();
        let first = deb2.first().map(|r| r.lma);
        if first != Some(end_of_rom) || covered != len - end_of_rom {
            bad.push(format!("deb2 rows {deb2:?} are not exactly [{end_of_rom:#X}, {len:#X})"));
        }
    }

    // checksum: exactly the field the fold writes, holding the file's checksum.
    let ck: Vec<&Row> = rows.iter().filter(|r| r.kind == "checksum").collect();
    match sigil_link::header_checksum_field(rom.len()) {
        None => bad.push(format!("a {len}-byte ROM has no header checksum field: unmeasurable")),
        Some(field) => {
            let want = (field.start as u64, field.len() as u64);
            if ck.len() != 1 || (ck[0].lma, ck[0].len) != want {
                bad.push(format!("checksum rows {ck:?}, want one row at {want:?}"));
            }
            let word = u16::from_be_bytes([rom[field.start], rom[field.start + 1]]);
            if word != sigil_link::header_checksum(rom) {
                bad.push(format!("the ROM's word at {:#X} is not its header checksum", field.start));
            }
        }
    }
    bad
}

#[test]
fn every_rom_byte_is_covered_once() {
    let Some((_g, tree)) = tree() else { return skip() };
    let mut failures = Vec::new();
    for (name, flags) in SHAPES {
        let b = build_shape(&tree, name, flags);
        let eor = end_of_rom(&b.listing).unwrap_or_else(|e| panic!("shape {name}: {e}"));
        let rows = rows_of(&b.export).unwrap_or_else(|e| panic!("shape {name}: {e}"));
        let bad = gate2(&b.rom, eor, &b.export, &rows);
        let count = |k: &str| rows.iter().filter(|r| r.kind == k).count();
        eprintln!(
            "GATE2 shape={name} rom={} end_of_rom={eor} rows={} code={} data={} fill={} org-gap={} deb2={} checksum={} sections={} sidecar_bytes={} bad={}",
            b.rom.len(),
            rows.len(),
            count("code"),
            count("data"),
            count("fill"),
            count("org-gap"),
            count("deb2"),
            count("checksum"),
            b.export["lines"]["sections"].as_object().map_or(0, |s| s.len()),
            b.sidecar_len,
            bad.len()
        );
        if !bad.is_empty() {
            failures.push(format!("shape {name}: {} problems; first: {:?}", bad.len(), &bad[..bad.len().min(5)]));
        }

        if name == "plain" {
            // The fragment walk states far more than the spike's label-granularity rows.
            assert!(
                rows.len() > SPIKE_LABEL_ROWS,
                "s4 plain has {} line rows, not more than the spike's {SPIKE_LABEL_ROWS}",
                rows.len()
            );
            // Controls: the gate is not blind to the two defects it exists to catch.
            // (1) Drop one row.
            let mut dropped = rows.clone();
            let gone = dropped.remove(dropped.len() / 2);
            let red = gate2(&b.rom, eor, &b.export, &dropped);
            assert!(red.iter().any(|m| m.contains("gap")), "a dropped row {gone:?} passed gate 2: {red:?}");
            // (2) A source row duplicated over the header back-patch: the checksum bytes
            // claimed again by a data row (the source line that declared them).
            let ck = rows.iter().find(|r| r.kind == "checksum").expect("a checksum row").clone();
            let mut dup = rows.clone();
            dup.push(Row { section: ck.section.clone(), kind: "data".into(), file: 0, line: 1, ..ck });
            dup.sort_by_key(|r| (r.lma, r.len));
            let red = gate2(&b.rom, eor, &b.export, &dup);
            assert!(red.iter().any(|m| m.contains("overlap")), "a duplicated header row passed gate 2: {red:?}");
        }
    }
    assert!(failures.is_empty(), "gate 2 failed:\n{}", failures.join("\n"));
}
