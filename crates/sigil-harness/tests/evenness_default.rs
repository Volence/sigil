//! The new driver's alignment rule against the old driver's table (G0b Task 5).
//!
//! `section_align::DECLARED` is the old driver's 108-row alignment table. Spec §2: 102 of
//! those rows say only "a 68000 section base is even", an ISA fact, which the new driver
//! states once as its default (`sigil_build::align::EVEN`); the other six are the real
//! requirements, and they are `[[align]]` rows in the descriptions. This test derives EVERY
//! row of the old table from the new rule (the default plus the engine and game rows read
//! from the reference tree) and requires the default to be the source of exactly 102.
//!
//! Expectations are read from source: the row count, the six exceptions and each value come
//! from `DECLARED` and the descriptions, never from a literal here, except the 102 the spec
//! names (and the test proves it from the table: 102 = rows - exceptions).

use sigil_build::align::{AlignSource, AlignTable};
use sigil_build::describe::{load_engine, load_game};
use sigil_harness::section_align::DECLARED;
use sigil_harness::test_support;

#[test]
fn evenness_default_reproduces_the_102_rows() {
    let needed = ["engine/engine.toml", "games/sonic4/game.toml", "games/demo/game.toml"];
    let Some(aeon) = test_support::reference_tree(&needed) else {
        return;
    };
    let engine = load_engine(&aeon.join(needed[0])).unwrap_or_else(|e| panic!("{e}"));
    let sonic4 = load_game(&aeon.join(needed[1])).unwrap_or_else(|e| panic!("{e}"));
    let demo = load_game(&aeon.join(needed[2])).unwrap_or_else(|e| panic!("{e}"));
    // The old table is one table for both games, so the new rule it is compared with is the
    // engine rows plus every game's rows. A game row for a label another game also declares
    // would be refused by AlignTable::new as a duplicate.
    let table = AlignTable::new([
        ("engine.toml", &engine.align[..]),
        ("sonic4 game.toml", &sonic4.align[..]),
        ("demo game.toml", &demo.align[..]),
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    assert!(!DECLARED.is_empty(), "UNMEASURABLE: section_align::DECLARED is empty");
    let mut by_default = 0usize;
    let mut by_row = 0usize;
    let mut faults = Vec::new();
    for d in DECLARED {
        let got = table.required_for(d.label);
        if got != d.required {
            faults.push(format!("`{}`: the old table says {:#x}, the new rule {got:#x}", d.label, d.required));
        }
        match table.source_of(d.label) {
            AlignSource::Even => by_default += 1,
            AlignSource::Row { .. } => by_row += 1,
        }
    }
    // Every declared row is a label the old table also carries: a description row the old
    // table lacks would be a requirement with no twin to agree with.
    for (label, align) in table.rows() {
        if !DECLARED.iter().any(|d| d.label == label) {
            faults.push(format!("`{label}` ({align:#x}) is an [[align]] row the old table does not carry"));
        }
    }
    assert!(faults.is_empty(), "{} disagreement(s):\n  {}", faults.len(), faults.join("\n  "));
    assert_eq!(by_row, table.rows().count(), "every description row must be one old-table row");
    assert_eq!(by_default + by_row, DECLARED.len());
    assert_eq!(by_default, 102, "the default must reproduce 102 rows (spec §2); it reproduces {by_default} of {}", DECLARED.len());
    eprintln!("evenness default: {by_default} row(s) from EVEN, {by_row} from [[align]] rows, of {}", DECLARED.len());
}
