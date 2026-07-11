//! `table` — the counted / sentinel / sparse collection construct (Plan 7 T2-d,
//! design `2026-07-11-counted-sparse-collection-design.md`). Sibling of
//! `offsets`: two emission shapes (record-list `[header?] rows [sentinel?]`, and
//! index = a payload stream plus a key-addressed cell table), sharing the
//! lowering machinery, with disjoint cell byte contracts.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn section_bytes(m: &Module, name: &str) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.section(name).map(|ls| ls.bytes.clone()).unwrap_or_default()
}

// ---- 1. index mode: payload + sparse cell table, item_align, body:before ------

#[test]
fn index_mode_sparse_payload_and_cells() {
    // A is 3 bytes (odd -> item_align:2 fires one $00); B is 2 bytes (even ->
    // no pad). Payload emits first (body: before), then the 4-cell table over
    // keys 1..=4 with holes 0.
    let src = "\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=4, hole: 0, item_align: 2, body: before) {
        1: A = bytes(\"ABC\"),
        3: B = bytes(\"DE\"),
    }
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Payload: A@0 = 41 42 43, pad 00 -> B@4 = 44 45.  Cells@6: key1->A(0),
    // key2->hole 0, key3->B(4), key4->hole 0 (Abs32Be).
    assert_eq!(
        section_bytes(&m, "s"),
        vec![
            0x41, 0x42, 0x43, 0x00, 0x44, 0x45, // payload
            0x00, 0x00, 0x00, 0x00, // key1 -> A @0
            0x00, 0x00, 0x00, 0x00, // key2 -> hole
            0x00, 0x00, 0x00, 0x04, // key3 -> B @4
            0x00, 0x00, 0x00, 0x00, // key4 -> hole
        ]
    );
}

// ---- 2. record-list mode: header u16(count-1) over typed records --------------

#[test]
fn record_list_header_count_minus_one() {
    let src = "\
module m
struct Rec { a: u16, b: u16 }
section s (cpu: m68000) {
    table T: [Rec] (header: u16(count - 1)) {
        Rec { a: $1111, b: $2222 },
        Rec { a: $3333, b: $4444 },
    }
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // count = 2 -> header = 0x0001 (Name sits on it); then the two 4-byte records.
    assert_eq!(
        section_bytes(&m, "s"),
        vec![0x00, 0x01, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44]
    );
}

// ---- 3. derived comptime facts: count / len / min_key / max_key ---------------

#[test]
fn derived_facts() {
    let src = "\
module m
section facts (cpu: m68000) {
    data C: u16 = T.count
    data L: u16 = T.len
    data MN: u16 = T.min_key
    data MX: u16 = T.max_key
}
section t (cpu: m68000) {
    table T (cell: *u8, key: $10..=$12, hole: 0) {
        $10: A = byte(0),
        $12: B = byte(0),
    }
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // count = 2, len = $12-$10+1 = 3, min = $10, max = $12.
    assert_eq!(section_bytes(&m, "facts"), vec![0x00, 0x02, 0x00, 0x03, 0x00, 0x10, 0x00, 0x12]);
}

// ---- 4. Name anchoring: index-mode header sits BEFORE Name's anchor ------------

#[test]
fn index_header_before_name_anchor() {
    // A headered index table: `Name` must anchor to the first CELL, not the
    // header word, so a `ref: *u8 = T` pointer (via the win-tab idiom) targets
    // the cell table's base. We check the linked address difference: the cell
    // table starts 2 bytes after the header.
    let src = "\
module m
section s (cpu: m68000) {
    data P: *u8 = \"Base\"    // a pointer to the table's Name (4 bytes)
    table Base (cell: *u8, key: 1..=1, hole: 0, header: u16(count)) {
        1: A = byte($AA),
    }
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let bytes = section_bytes(&m, "s");
    // Layout (body:after default = payload AFTER the cell table):
    //   P(*u8)@0 (4 bytes) ; header count=1 @4 ; Base = first cell @6 ;
    //   cell(4 bytes)@6 ; payload A@10.  So Base (Name) = 6, NOT the header @4.
    assert_eq!(&bytes[0..4], &[0x00, 0x00, 0x00, 0x06], "Name anchors to first cell (after header)");
    assert_eq!(&bytes[4..6], &[0x00, 0x01], "header count=1 sits before Name");
    assert_eq!(&bytes[6..10], &[0x00, 0x00, 0x00, 0x0A], "cell key1 -> A@10");
    assert_eq!(&bytes[10..11], &[0xAA], "payload A ($AA)");
}

// ---- 5. record-list Name sits ON the header (first byte) ----------------------

#[test]
fn record_list_name_on_header() {
    let src = "\
module m
struct Rec { a: u16 }
section s (cpu: m68000) {
    data P: *u8 = \"T\"
    table T: [Rec] (header: u16(count)) {
        Rec { a: $BEEF },
    }
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let bytes = section_bytes(&m, "s");
    // P@0 (4 bytes) = address of T; T is the header at offset 4.
    assert_eq!(&bytes[0..4], &[0x00, 0x00, 0x00, 0x04], "record-list Name sits on the header");
    assert_eq!(&bytes[4..6], &[0x00, 0x01], "header count=1");
    assert_eq!(&bytes[6..8], &[0xBE, 0xEF], "row");
}

// ---- 6. diagnostics ------------------------------------------------------------

fn diag(src: &str) -> Vec<String> {
    lower(src).1
}

#[test]
fn err_duplicate_key() {
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=3, hole: 0) {
        1: A = byte(0),
        1: B = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("duplicate key")), "{msgs:?}");
}

#[test]
fn err_unordered_key() {
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=3, hole: 0) {
        3: A = byte(0),
        1: B = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("ascending order")), "{msgs:?}");
}

#[test]
fn err_key_out_of_range() {
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=3, hole: 0) {
        9: A = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("outside the domain")), "{msgs:?}");
}

#[test]
fn err_exhaustive_missing_keys() {
    // No hole -> exhaustive: key 2 is missing.
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=3) {
        1: A = byte(0),
        3: B = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("missing keys")), "{msgs:?}");
}

#[test]
fn err_index_needs_key() {
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (cell: *u8) {
        A = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("needs a `key:`")), "{msgs:?}");
}

#[test]
fn err_hole_needs_key() {
    let msgs = diag("\
module m
section s (cpu: m68000) {
    table T (hole: 0) {
        A = byte(0),
    }
}
");
    assert!(msgs.iter().any(|m| m.contains("requires a `key:`")), "{msgs:?}");
}

// ---- 7. exhaustive success (dense keyed, no hole) -----------------------------

#[test]
fn exhaustive_all_keys_present() {
    let src = "\
module m
section s (cpu: m68000) {
    table T (cell: *u8, key: 1..=2, hole: 0) {
        1: A = byte($11),
        2: B = byte($22),
    }
}
";
    let (_m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
}
