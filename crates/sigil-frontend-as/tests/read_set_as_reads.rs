//! The AS frontend's file reads (the root, an `include`, a `BINCLUDE`) land in
//! `sigil_span::read_set` with the bytes the assembly consumed, so a native build's
//! `.lst` source digest names every file the AS residual pulled in.
//!
//! Measured on a synthetic tree because the shipped corpus's AS residual need not
//! BINCLUDE anything at a given aeon revision; the end-to-end gate
//! (`sigil-cli/tests/lst_source_digest.rs`) prints how many BINCLUDE targets it found and
//! requires each to be a row.

use sigil_frontend_as::{assemble_root, Options};
use sigil_span::read_set;

#[test]
fn the_root_an_include_and_a_binclude_are_recorded_with_their_bytes() {
    let dir = std::env::temp_dir().join(format!("sigil_read_set_as_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).expect("scratch dir");
    let root_text = "\tinclude \"sub/inc.asm\"\n\tBINCLUDE \"sub/blob.bin\"\n";
    let inc_text = "\tdc.b 1\n";
    let blob = [0xAAu8, 0xBB, 0xCC];
    std::fs::write(dir.join("main.asm"), root_text).expect("write root");
    std::fs::write(dir.join("sub/inc.asm"), inc_text).expect("write include");
    std::fs::write(dir.join("sub/blob.bin"), blob).expect("write blob");

    let opts = Options { initial_cpu: Some(sigil_ir::Cpu::M68000), ..Options::default() };
    assemble_root(&dir.join("main.asm"), &opts).unwrap_or_else(|e| panic!("did not assemble: {e:?}"));

    let dir = read_set::canonical(&dir);
    let got: Vec<(String, u32, u64)> = read_set::snapshot()
        .reads
        .iter()
        .filter_map(|r| {
            let rel = r.path.strip_prefix(&dir).ok()?;
            Some((rel.to_string_lossy().into_owned(), r.crc, r.size))
        })
        .collect();
    let want = vec![
        ("main.asm".to_string(), read_set::crc32(root_text.as_bytes()), root_text.len() as u64),
        ("sub/blob.bin".to_string(), read_set::crc32(&blob), blob.len() as u64),
        ("sub/inc.asm".to_string(), read_set::crc32(inc_text.as_bytes()), inc_text.len() as u64),
    ];
    assert_eq!(got, want, "the AS reads the recorder holds for this tree");
}
