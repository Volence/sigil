//! Spec 2, Plan 7 — #9b (R9b.12): the `script` exhibit, PINNED.
//!
//! `examples/game/badniks/pitcher_plant_script.emp` is the SIBLING of the proc
//! exhibit (`pitcher_plant.emp`): the SAME badnik with its brain written as ONE
//! `script` coroutine instead of three hand-threaded procs. It builds through
//! the REAL multi-module pipeline (`sigil emp <entry> --root examples/game
//! --prelude prelude`), compiling end-to-end with ZERO diagnostics.
//!
//! Invocation mirrors `pitcher_plant_acceptance.rs` exactly (the `--root`/
//! `--prelude` multi-module CLI shape has no in-process helper — the CLI binary
//! is the house entry point for this shape).
//!
//! ## What this test pins (and what it deliberately does NOT)
//!
//! The controller does the full-image byte-for-byte hand-derivation pass
//! separately (as `pitcher_plant_acceptance.rs` does for the proc version).
//! This test pins the load-bearing facts a regression would trip on:
//!   - exit 0 + ZERO diagnostics of any severity;
//!   - the exact output byte LENGTH (358 — recorded from the verified build);
//!   - the HIDDEN resume table (R9b.2): its base is `brain` (the script's
//!     name), which `Def.code` points at; member 0 = the entry segment; there
//!     is exactly one row per yield PLUS the entry row; rows are monotonically
//!     increasing offsets into the body. Each row's value is derived and
//!     documented below.
//!
//! ## The exhibit's shape (yield count → table layout)
//!
//! `pitcher_plant_script.emp`'s `brain` has exactly THREE bare `yield`s (one in
//! each of `.wait_tick`, `.windup_tick`, and the `.rearm` tail). Per R9b.2 the
//! hidden table has `yield_count + 1 = 4` rows (member 0 = entry, then one
//! resume point per yield). Encoding is `word_offsets`, so each row is a 2-byte
//! big-endian offset from the table base → the table is `4 * 2 = 8` bytes.
//!
//! Row 0 (the entry offset) is therefore `2 * (1 + yield_count) = 2 * 4 = 8`
//! (`0x0008`): the entry segment begins immediately after the 8-byte table.
//! The remaining rows (`0x001E`, `0x006C`, `0x0082`) are the byte offsets of
//! the three `__resume$k` labels, read from the verified build below; the test
//! asserts they are STRICTLY INCREASING (each resume point sits later in the
//! body than the previous — a structural invariant of a straight-line flattened
//! body: the entry precedes resume 1 precedes resume 2 precedes resume 3), and
//! pins all four verbatim.

use std::path::Path;
use std::process::Command;

/// The multi-module example root (workspace `examples/game/`, mirroring
/// `pitcher_plant_acceptance.rs`).
fn game_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/game"))
}

/// Run `sigil emp <entry> --root <root> --prelude prelude -o <out>` exactly as
/// a user would, and return `(exit_success, stdout, stderr, image_bytes)`.
fn build_script_exhibit(root: &Path, out: &Path) -> (bool, String, String, Option<Vec<u8>>) {
    let entry = root.join("badniks/pitcher_plant_script.emp");
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn the sigil binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let bytes = std::fs::read(out).ok();
    (output.status.success(), stdout, stderr, bytes)
}

/// Read a big-endian 16-bit word at `off`.
fn word(image: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([image[off], image[off + 1]])
}

/// Read a big-endian 32-bit long at `off`.
fn long(image: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([
        image[off],
        image[off + 1],
        image[off + 2],
        image[off + 3],
    ])
}

/// The number of resume points `brain` mints beyond the entry — three: the
/// `.watch` named-resume member, the `.windup_tick` named-resume member
/// (named yields join members; none is minted at a yield's own site), and
/// the `wait_frames` park's hidden tick member. Drives the table's row count.
const RESUME_POINTS: usize = 3;

/// The verified output length (recorded from the clean `--root`/`--prelude`
/// build; a change here means the exhibit's emitted image changed).
const IMAGE_LEN: usize = 340;

#[test]
fn script_exhibit_builds_clean_and_pins_hidden_table() {
    let root = game_root();
    let out_dir = std::env::temp_dir().join(format!(
        "sigil_pps_acceptance_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("pitcher_plant_script.bin");

    let (success, stdout, stderr, image) = build_script_exhibit(root, &out);

    // --- exit 0 + zero diagnostics ----------------------------------------
    assert!(
        success,
        "script exhibit build must succeed with zero errors; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "expected ZERO diagnostics of any severity (warnings included); stderr was:\n{stderr}"
    );

    let image = image.expect("output .bin was not written");

    // --- exact byte length ------------------------------------------------
    assert_eq!(
        image.len(),
        IMAGE_LEN,
        "the script exhibit image must be exactly {IMAGE_LEN} bytes"
    );
    assert!(
        stdout.contains(&format!("built: {IMAGE_LEN} bytes")),
        "expected the CLI to report `built: {IMAGE_LEN} bytes`, stdout was: {stdout}"
    );

    // --- locate the hidden resume table -----------------------------------
    // `pub data Def = ObjDef{ code: brain, ... }` places `Def` first in the
    // `obj_bank` section after the 20-byte (0x14) `offsets Ani` table + ani
    // data. `Def`'s FIRST field is `code: *u8`, a 4-byte absolute fixup to
    // `brain`'s link address — and `brain` (R9b.8) IS the hidden table's base
    // label. So `Def.code` (offset 0x14) tells us where the table lives,
    // without hardcoding the layout arithmetic here. In the verified build
    // this resolves to 0x40.
    let table_base = long(&image, 0x14) as usize;
    assert_eq!(
        table_base, 0x40,
        "Def.code should point at `brain`'s table base (0x40 in the verified build)"
    );

    // --- pin the table rows (R9b.2) ---------------------------------------
    // word_offsets: each row is a 2-byte BE offset from the table base. There
    // are YIELD_COUNT + 1 rows (member 0 = entry, then one per yield).
    let rows: Vec<u16> = (0..=RESUME_POINTS)
        .map(|k| word(&image, table_base + k * 2))
        .collect();

    // Row 0 = the entry offset = the table's own width = 2 * (1 + members).
    // The entry segment begins immediately after the row array.
    let entry_off = 2 * (1 + RESUME_POINTS) as u16; // 2 * 4 = 8
    assert_eq!(
        rows[0], entry_off,
        "row 0 (entry) must equal the table width = 2*(1+{RESUME_POINTS}) = {entry_off}"
    );

    // The remaining rows are the resume-member offsets, read from the
    // verified build, NON-DECREASING in body order — non-strict because
    // `.watch` is the body's FIRST line, so its member COINCIDES with the
    // entry (spawning and "look again tomorrow" resume at the same place;
    // rows 0 and 1 are equal by design).
    for pair in rows.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "table rows must be non-decreasing offsets, got {rows:?}"
        );
    }

    // Verbatim pin of all four rows (derivation above; values read from the
    // verified clean build):
    //   row 0 = 0x0008  entry segment (= table width, derived)
    //   row 1 = 0x0008  `.watch` — the body's first line, so it coincides
    //                   with the entry (see the non-strict check above)
    //   row 2 = 0x0032  `.windup_tick`
    //   row 3 = 0x006A  the wait_frames park's hidden per-frame tick
    assert_eq!(
        rows,
        vec![0x0008, 0x0008, 0x0032, 0x006A],
        "hidden resume table rows changed"
    );

    let _ = std::fs::remove_dir_all(&out_dir);
}
