//! Inline `offsets` bodies (§4.7's ratified next-offsets increment, tranche 0):
//! the MIXED member form — `Name: Type = value` declares the table entry AND
//! its payload in one place, reusing the exact `data`-item shape so the
//! declared length stays the terminator guard (a `[u8; 4]` initializer missing
//! its `$FF` must not compile). Emission: the dc.w table first, then each
//! inline body in declaration order; by-reference members (`Name: label`) mix
//! freely. One emission grammar, no new `dataset` concept (tenet 1).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};

fn lower(src: &str) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, diags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    (module, diags.into_iter().map(|d| d.message).collect())
}

fn linked_bytes(m: &Module) -> Vec<u8> {
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    m.sections
        .iter()
        .find_map(|s| linked.section(&s.name).map(|ls| ls.bytes.clone()))
        .unwrap_or_default()
}

// ---- 1. emission: table first, bodies in declaration order --------------------

#[test]
fn inline_bodies_emit_after_table_in_decl_order() {
    let src = "\
module m
offsets Ani {
    Idle: [u8; 2] = [7, $FF],
    Seed: [u8; 2] = [3, $FF],
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Table at 0: Idle body at 4, Seed body at 6 (self-relative words),
    // then the bodies in declaration order.
    assert_eq!(linked_bytes(&m), vec![0x00, 0x04, 0x00, 0x06, 7, 0xFF, 3, 0xFF]);
}

#[test]
fn mixed_inline_and_reference_members() {
    let src = "\
module m
offsets M {
    A: [u8; 1] = [9],
    B: Elsewhere,
}
data Elsewhere: [u8; 1] = [5]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Table 4 bytes; A's body at 4; Elsewhere lands after at 5 → word1 = 5.
    assert_eq!(linked_bytes(&m), vec![0x00, 0x04, 0x00, 0x05, 9, 5]);
}

// ---- 2. the REQUIRED increment test: in-block ordinal self-reference ----------

#[test]
fn in_block_ordinal_self_reference() {
    // `Shoot`'s payload reads `Ani.Idle` (== 0) from its OWN block — the
    // "$FD: switch to idle" command byte. Ordinals come from declaration
    // position, so this is well-founded (§4.7's required test).
    let src = "\
module m
offsets Ani {
    Idle:  [u8; 2] = [7, $FF],
    Shoot: [u8; 3] = [4, $FD, Ani.Idle],
}
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    assert_eq!(
        linked_bytes(&m),
        vec![0x00, 0x04, 0x00, 0x06, 7, 0xFF, 4, 0xFD, 0x00],
        "Ani.Idle folds to ordinal 0 inside Shoot's own body"
    );
}

// ---- 3. the terminator guard ---------------------------------------------------

#[test]
fn short_initializer_is_a_type_error() {
    // The declared [u8; 4] length IS the terminator guard: drop the $FF and
    // the initializer no longer fills the type → compile error, not a
    // runtime hang.
    let src = "\
module m
offsets Ani {
    Idle: [u8; 4] = [7, 0, 1],
}
";
    let (_, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("4") && m.contains("3")),
        "the length mismatch is named (declared 4, got 3): {msgs:?}"
    );
}

// ---- 4. reverse ordinals & guards unchanged ------------------------------------

#[test]
fn ordinals_and_count_cover_inline_members() {
    let src = "\
module m
offsets Ani {
    Idle:  [u8; 2] = [7, $FF],
    Seed:  Elsewhere,
    Shoot: [u8; 2] = [4, $FF],
}
data Elsewhere: [u8; 1] = [5]
data Ids: [u8; 4] = [Ani.Idle, Ani.Seed, Ani.Shoot, Ani.count]
";
    let (m, msgs) = lower(src);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let bytes = linked_bytes(&m);
    // Ids is the LAST item: ordinals 0,1,2 and count 3.
    assert_eq!(&bytes[bytes.len() - 4..], &[0, 1, 2, 3]);
}

#[test]
fn inline_offsets_in_z80_section_still_refused() {
    let src = "\
module m
section z (cpu: z80, vma: $0000) {
    offsets T {
        A: [u8; 1] = [1],
    }
}
";
    let (_, msgs) = lower(src);
    assert!(
        msgs.iter().any(|m| m.contains("[offsets.non-68k]")),
        "the 68k-only rule covers the inline form: {msgs:?}"
    );
}
