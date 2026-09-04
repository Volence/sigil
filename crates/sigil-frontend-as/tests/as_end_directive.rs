//! AS's `END` directive ENDS THE ASSEMBLY.
//!
//! asl stops reading source at the `END` line. Everything after it — the rest of
//! the file, the rest of an INCLUDING file, the remaining iterations of a loop
//! `END` sits inside — is never assembled, and asl says nothing about it: the
//! listing simply stops, with 0 errors and 0 warnings. A front end that treats
//! the directive as a no-op therefore emits EXTRA BYTES SILENTLY. There is no
//! diagnostic to count and no complaint to notice; only an image comparison
//! separates the two behaviours, which is why this gate asserts bytes.
//!
//! Every expectation below is read off the listing of `asl` 1.42 Beta Bld 212 —
//! the binary committed at `s2disasm/build_tools/Linux-x86_64/asl` — for the
//! identical source text. The probes are committed under
//! `docs/superpowers/notes/2026-09-04-as-end-probes/`, and `run.sh` there reruns
//! any of them.
//!
//! ## Division of labour with the golden snippets
//!
//! The single-file shapes (bare `end`, `end` under a false conditional, `end` in
//! a macro expansion, `end` in a `rept` body, `END <entrypoint>`) are pinned as
//! `asl_snippets` golden blocks, whose bytes are MINTED by real asl rather than
//! typed here. This file carries the one shape that file cannot express, because
//! a golden block is a single source string with no filesystem: `end` inside an
//! INCLUDED file, which ends the whole unit and not merely the include.

use sigil_frontend_as::{assemble_root, Options};
use sigil_ir::SymbolTable;

/// Assemble a root file from disk, link, and flatten to an image.
fn image(root: &std::path::Path) -> Vec<u8> {
    let m = assemble_root(root, &Options::default())
        .unwrap_or_else(|e| panic!("did not assemble {}:\n{e:?}", root.display()));
    let resolved = sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|e| panic!("did not resolve:\n{e:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|e| panic!("did not link:\n{e:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// `end` in an included file ends the WHOLE unit — the parent's own line after
/// the `include` never runs either.
///
/// asl listing for `tests/vectors/as_end_include/root.asm` (`(1)` marks the
/// included file's own line numbering; the listing ends at its `end`):
///
/// ```text
///       11/       0 : 11                  	dc.b $11
///       12/       1 : =>FALSE              	if 0
///       13/       1 :                     	end
///       14/       1 : [12]                 	endif
///       15/       1 : 22                  	dc.b $22
///       16/       2 :                     	include "part.asm"
/// (1)    1/       2 : 33                  	dc.b $33
/// (1)    2/       3 :                     	end
/// ```
///
/// p2bin image: `11 22 33`. The two bytes asl does NOT emit are `part.asm`'s
/// trailing `dc.b $99` and `root.asm`'s `dc.b $44` after the `include`; a no-op
/// `end` emits `11 22 33 99 44`.
#[test]
fn end_in_an_included_file_stops_the_whole_unit() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/as_end_include/root.asm");
    assert_eq!(
        image(&root),
        vec![0x11, 0x22, 0x33],
        "`end` inside an included file must end the assembly unit, so neither the \
         include's own trailing byte ($99) nor the parent's byte after the include \
         ($44) is emitted"
    );
}

/// The same fixture states the OTHER half in one run: the `end` inside the
/// `if 0` arm is not executed, so `dc.b $22` after the `endif` is still
/// assembled. Asserted separately because it is the failure a naive
/// "stop at the first `end` token" implementation produces — that one emits
/// `11` alone and still passes any test that only checks bytes were dropped.
#[test]
fn end_under_a_false_conditional_does_not_stop_the_assembly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/as_end_include/root.asm");
    let img = image(&root);
    assert!(
        img.len() >= 2 && img[1] == 0x22,
        "the `dc.b $22` after the `endif` must survive — the `end` inside the FALSE \
         `if` arm is never executed (asl listing line 12: `=>FALSE`). Got {img:02X?}"
    );
}

/// An `end`-terminated unit is a SUCCESSFUL assembly, not an aborted one: asl
/// reports `0 errors, 0 warnings` for every probe above. This is the assertion
/// that keeps the implementation off the `fatal` path, which shares the same
/// internal stop signal but raises a diagnostic first.
#[test]
fn end_terminates_without_raising_a_diagnostic() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors/as_end_include/root.asm");
    assert!(
        assemble_root(&root, &Options::default()).is_ok(),
        "`end` must end the unit cleanly — asl reports 0 errors and 0 warnings"
    );
}
