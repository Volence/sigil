//! `native::FROZEN_TABLES` is the committed `golden/offcanonical_sizes/` directory,
//! compiled in: one row per file there, each row carrying its own file's text.
//!
//! A table committed to the directory without a row would be derived, reviewed and
//! committed, and then silently never loaded by any binary; a row carrying another row's
//! file would load the wrong shape's boundaries under the right name. Both are checked
//! against the directory as it is on disk, which is also what the rows were compiled
//! from, so a failure here is always a wrong row, never a stale build.

use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn frozen_tables_embed_every_committed_table() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/offcanonical_sizes");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("list {}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    assert!(
        !on_disk.is_empty(),
        "{} holds no tables, so there is nothing to hold the embedded list to",
        dir.display()
    );

    let embedded: BTreeSet<String> =
        sigil_harness::native::FROZEN_TABLES.iter().map(|(n, _)| n.to_string()).collect();
    assert_eq!(
        embedded.len(),
        sigil_harness::native::FROZEN_TABLES.len(),
        "FROZEN_TABLES names a table twice"
    );
    assert_eq!(
        embedded, on_disk,
        "FROZEN_TABLES and {} disagree about which tables exist",
        dir.display()
    );

    for (name, text) in sigil_harness::native::FROZEN_TABLES {
        let file = std::fs::read_to_string(dir.join(name)).expect("read a committed table");
        assert!(
            text == file,
            "the FROZEN_TABLES row `{name}` does not carry {}",
            dir.join(name).display()
        );
    }
    eprintln!("{} frozen table(s) embedded, each its own file", embedded.len());
}
