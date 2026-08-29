//! The assembled-bar (`EndOfRom`) agreement gate.
//!
//! One ROM address — the end of the assembled region — is written into three
//! checked-in artifacts by three tools, at three steps of a landing:
//!
//! * `derive_offcanon` writes `golden/offcanonical_sizes/<t>.txt`, spelling the
//!   address TWICE: as the `EndOfRom` row and as the `# assembled_end=` header.
//! * `repin` writes `pins::{DEBUG_,}ASSEMBLED_LEN`, the canonical shapes' copy.
//! * `refreeze` writes `golden/provenance.toml`'s per-target `anchor_end` — sourced
//!   from `pins.rs` for the two canonical shapes, and from the size-table HEADER for
//!   the five off-canonical ones (`refreeze::authoritative_anchor_ends`).
//!
//! That last sourcing is why these tests are careful about what they claim, and the
//! claim is not uniform across shapes:
//!
//! * For `s4` / `s4_debug`, comparing the table against `anchor_end` is genuinely
//!   CROSS-TOOL — `repin`'s pins against `derive_offcanon`'s table.
//! * For the off-canonical five, `provenance.toml` holds a COPY of the table header
//!   taken at freeze time, so the comparison is TEMPORAL, not independent: it catches
//!   a table regenerated or hand-edited since the last freeze, and a freeze taken
//!   against a different table state. That is a half-done landing either way, which
//!   is the thing worth catching — but it is not a second witness to the address.
//!
//! Neither is circular (no derivation here is checked against itself), and neither is
//! the whole-ROM check: `native_offcanonical_rom` does that against the golden bytes.
//!
//! Nothing previously compared any of these at all, which is how four off-canonical
//! `assembled_len` values drifted with no gate able to go red — they had no reader
//! either.
//!
//! Every comparison here is SOURCE-ONLY over checked-in files: no aeon tree, no ROM
//! build, no `AEON_DIR`. They therefore never skip, and a shape that cannot be
//! measured fails loudly instead of passing quietly.

use std::path::PathBuf;

use sigil_harness::native::{self, GameProfile};
use sigil_harness::pins;
use sigil_harness::provenance;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden")
}

/// Every frozen target: the `provenance.toml` target key, the size-table stem, and the
/// profile that ships it. The list is the gate's own subject — a shape added to
/// `derive_offcanon`'s targets without a row here is caught by
/// `every_frozen_size_table_is_gated` below.
fn targets() -> Vec<(&'static str, &'static str, GameProfile)> {
    vec![
        ("s4", "s4.txt", native::sonic4_profile(false)),
        ("s4_debug", "s4_debug.txt", native::sonic4_profile(true)),
        ("demo", "demo.txt", native::demo_profile(false)),
        ("demo_debug", "demo_debug.txt", native::demo_profile(true)),
        ("config_a", "config_a.txt", native::config_a_profile()),
        ("config_b", "config_b.txt", native::config_b_profile()),
        ("lean", "lean.txt", native::lean_profile()),
    ]
}

/// The `# assembled_end=0x...` header of a size table, or an error naming the miss.
/// Deliberately a second reader of the same file `load_frozen_table` parses rows out
/// of: the header and the `EndOfRom` row are two copies, and this is what compares them.
fn header_assembled_end(stem: &str) -> Result<usize, String> {
    let path = golden_dir().join("offcanonical_sizes").join(stem);
    let txt = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    for line in txt.lines() {
        if let Some(rest) = line.trim().strip_prefix("# assembled_end=") {
            let v = rest.trim().trim_start_matches("0x");
            return usize::from_str_radix(v, 16).map_err(|e| format!("{stem} assembled_end `{v}`: {e}"));
        }
    }
    Err(format!("{stem}: no `# assembled_end=` header — the table is not derive_offcanon output"))
}

/// THE gate: each shape's profile-visible assembled bar == the provenance chain tip's
/// `anchor_end` for that shape.
///
/// `assembled_len()` reads the shape's committed boundary table live; `anchor_end` was
/// written at the last freeze. What the equality proves differs by shape — cross-tool
/// for the canonical pair, freeze-vs-current for the off-canonical five (see the
/// module header). Both directions of disagreement mean a landing did half its work.
#[test]
fn assembled_len_matches_provenance_tip_for_every_shape() {
    let mut bad: Vec<String> = Vec::new();
    for (key, _stem, profile) in targets() {
        let tip = match provenance::tip_target(&golden_dir(), key) {
            Ok(t) => t,
            // A target absent from the tip is a FAILURE, never a skip: the shape is
            // unmeasurable against the chain, which is the state this gate exists to
            // refuse.
            Err(e) => {
                bad.push(format!("{key}: {e}"));
                continue;
            }
        };
        let derived = profile.assembled_len();
        if derived != tip.anchor_end {
            bad.push(format!(
                "{key}: profile `{}` assembled_len() = {derived:#x} but provenance tip \
                 anchor_end = {:#x} (delta {}{:#x}) — the size table and the freeze \
                 describe different ROMs; re-run derive_offcanon, or refreeze",
                profile.name,
                tip.anchor_end,
                if derived > tip.anchor_end { "+" } else { "-" },
                derived.abs_diff(tip.anchor_end),
            ));
        }
    }
    assert!(bad.is_empty(), "assembled-bar disagreement ({}):\n  {}", bad.len(), bad.join("\n  "));
}

/// The size table's `# assembled_end=` header and its `EndOfRom` row are one value
/// written twice by `derive_offcanon`. They can only diverge if a human edited a
/// GENERATED file — the failure mode that produced every stale address in the
/// hand-typed-ROM-address inventory.
#[test]
fn size_table_header_agrees_with_its_endofrom_row() {
    let mut bad: Vec<String> = Vec::new();
    for (_key, stem, profile) in targets() {
        let header = match header_assembled_end(stem) {
            Ok(v) => v,
            Err(e) => {
                bad.push(e);
                continue;
            }
        };
        let row = profile.assembled_len();
        if header != row {
            bad.push(format!(
                "{stem}: header `# assembled_end={header:#x}` but `EndOfRom {row:#x}` — a \
                 GENERATED table was hand-edited; re-run derive_offcanon instead"
            ));
        }
    }
    assert!(bad.is_empty(), "size-table self-disagreement ({}):\n  {}", bad.len(), bad.join("\n  "));
}

/// `repin`'s canonical pins and `derive_offcanon`'s canonical tables are the two
/// generated descriptions of the SAME shape, produced by different tools at different
/// steps: `repin` writes `pins::{DEBUG_,}ASSEMBLED_LEN` off the build's own listing,
/// `derive_offcanon` writes the table's `EndOfRom` off a live resolve. A landing that
/// repins without re-deriving (or the reverse) leaves the two artifacts describing
/// different layouts. This is the only place they meet.
#[test]
fn canonical_pins_agree_with_the_canonical_size_tables() {
    for debug in [false, true] {
        let pinned = if debug { pins::DEBUG_ASSEMBLED_LEN } else { pins::ASSEMBLED_LEN };
        let frozen = native::sonic4_profile(debug).assembled_len();
        assert_eq!(
            pinned, frozen,
            "canonical {} shape: pins::{}ASSEMBLED_LEN = {pinned:#x} but {} `EndOfRom` = \
             {frozen:#x} — repin and derive_offcanon disagree about the assembled bar",
            if debug { "debug" } else { "plain" },
            if debug { "DEBUG_" } else { "" },
            if debug { "s4_debug.txt" } else { "s4.txt" },
        );
    }
}

/// The gate's subject list must cover every committed size table. Without this, a new
/// off-canonical shape ships with its table ungated and nothing above notices — the
/// same "no reader" hole that let the `assembled_len` literals rot.
#[test]
fn every_frozen_size_table_is_gated() {
    let dir = golden_dir().join("offcanonical_sizes");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    on_disk.sort();
    let mut gated: Vec<String> = targets().iter().map(|(_, s, _)| s.to_string()).collect();
    gated.sort();
    assert_eq!(
        on_disk, gated,
        "committed size tables and this gate's subject list differ — add the new shape's \
         row to `targets()` (and its provenance target) rather than leaving it ungated"
    );
}
