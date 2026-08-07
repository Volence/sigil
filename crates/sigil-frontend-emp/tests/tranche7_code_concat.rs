//! The `Code` monoid `++` (§6.2, tranche 7): `asm { } ++ asm { }` composes
//! fragments in emission order. Demanded by the aabb template's conditional
//! lead instruction (`let head = if aliased { asm { } } else { asm { move … } };
//! return head ++ asm { …body… }`) — the emp twin of an AS macro `if` guard.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;
use sigil_span::Level;

fn emp_bytes(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    assert!(errs.is_empty(), "lower diagnostics: {errs:?}");
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    linked.sections.iter().flat_map(|s| s.bytes.clone()).collect()
}

/// Concat of two non-empty fragments emits both, in order, byte-identical to
/// the single-block spelling.
#[test]
fn code_concat_appends_in_emission_order() {
    let split = emp_bytes(
        "module t in t\n\
         comptime fn f() -> Code {\n\
             let head = asm { moveq   #1, d0 }\n\
             return head ++ asm { moveq   #2, d1\n\
                 rts }\n\
         }\n\
         pub proc P () {\n\
             f()\n\
         }\n",
    );
    let joined = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 moveq   #1, d0\n\
                 moveq   #2, d1\n\
                 rts\n\
         }\n",
    );
    assert_eq!(split, joined, "split ++ spelling must equal the single block");
}

/// The conditional-head shape: an EMPTY `asm { }` head is the monoid identity —
/// concat with it emits only the tail.
#[test]
fn empty_code_is_concat_identity() {
    let with_empty_head = emp_bytes(
        "module t in t\n\
         comptime fn f(skip: bool) -> Code {\n\
             let head = if skip { asm { } } else { asm { moveq   #1, d0 } }\n\
             return head ++ asm { rts }\n\
         }\n\
         pub proc P () {\n\
             f(true)\n\
         }\n",
    );
    let bare = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 rts\n\
         }\n",
    );
    assert_eq!(with_empty_head, bare, "empty head must vanish");
}

/// Each `asm { }` block is its own hygiene scope, and `++` composes ITEMS, not
/// label spaces: a tail fragment's branch to a label defined in the head
/// fragment is a LOUD unresolved-symbol error, never silent wrong bytes.
/// (Pinned semantics — fn-call-scoped hygiene is a ledgered ask; a template
/// needing a shared label keeps it in ONE fragment, like the aabb head shape.)
#[test]
fn cross_fragment_label_fails_loudly() {
    let src = "module t in t\n\
         comptime fn f() -> Code {\n\
             let head = asm { .top:\n\
                 nop }\n\
             return head ++ asm { bra.s   .top }\n\
         }\n\
         pub proc P () {\n\
             f()\n\
         }\n";
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    let errs: Vec<_> = diags.iter().filter(|d| d.level == Level::Error).collect();
    if errs.is_empty() {
        let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
            .expect("resolve_layout");
        let err = sigil_link::link(&resolved, &SymbolTable::new())
            .expect_err("cross-fragment label must NOT silently resolve");
        assert!(
            format!("{err:?}").contains("unresolved symbol"),
            "must fail as an unresolved symbol, got: {err:?}"
        );
    }
}

/// A copy run SIZED BY THE STRUCT — the shape `engine.parallax`'s shadow-view
/// entry copy uses. The fold emits `sizeof(S)/4` long moves and a word tail when
/// the size is not long-divisible, so the run's total pointer advance is exactly
/// `sizeof(S)`.
///
/// Both polarities, because the interesting half is the FAILURE mode: at the
/// current size the derived run is BYTE-IDENTICAL to the hand-written one, so
/// the byte gate cannot tell a derived copy from a restated one — and only the
/// derived run follows the struct when it grows. A restated copy silently
/// truncates, which is the defect class the spelling exists to make
/// unrepresentable.
///
/// The domain is EVEN sizes only, and that is a hardware constraint rather than
/// a simplification: with an odd stride the k-th array element starts at
/// `base + k*size`, whose parity alternates, so the run's leading `move.l` lands
/// on an odd address for every odd k — a 68000 address error. The generator's
/// aeon call site carries an `ensure(sizeof(band_entry) % 2 == 0, …)` for
/// exactly that reason; this test stays inside the guarded domain.
#[test]
fn a_struct_sized_copy_run_grows_with_the_struct() {
    let generator = "\
         comptime fn copy_entry() -> Code {\n\
             let longs = 0..(sizeof(E) / 4) |> fold(asm {}, |acc, _i| acc ++ asm {\n\
                 move.l  (a1)+, (a4)+\n\
             })\n\
             if sizeof(E) % 4 == 0 { return longs }\n\
             return longs ++ asm { move.w  (a1)+, (a4)+ }\n\
         }\n\
         pub proc P () {\n\
             copy_entry()\n\
         }\n";
    let with_fields = |fields: &str| {
        format!("module t in t\nstruct E {{\n{fields}}}\n{generator}")
    };
    let ten = "    a: u8, b: u8, c: u8, d: u8, e: u8,\n    f: u8, g: u8, h: u8, i: u8, j: u8,\n";
    let twelve = format!("{ten}    k: u8, l: u8,\n");

    let derived_10 = emp_bytes(&with_fields(ten));
    let derived_12 = emp_bytes(&with_fields(&twelve));

    // The restated spelling: the run the struct happens to need TODAY.
    let restated = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 move.l  (a1)+, (a4)+\n\
                 move.l  (a1)+, (a4)+\n\
                 move.w  (a1)+, (a4)+\n\
         }\n",
    );

    // Polarity 1 — adopting the derived spelling costs nothing: at the current
    // size the two are the same bytes, which is exactly why the byte gate is
    // structurally blind to the difference between them.
    assert_eq!(
        derived_10, restated,
        "the derived run must be byte-identical to the restated one at the current size"
    );
    // Polarity 2 — grow the struct by two fields and only the derived run
    // follows it. The restated run is `restated` in both worlds; it does not
    // appear here because it CANNOT change, and that is the finding.
    let grown = emp_bytes(
        "module t in t\n\
         pub proc P () {\n\
                 move.l  (a1)+, (a4)+\n\
                 move.l  (a1)+, (a4)+\n\
                 move.l  (a1)+, (a4)+\n\
         }\n",
    );
    assert_eq!(
        derived_12, grown,
        "at 12 bytes the derived run must be three long moves"
    );
    assert_ne!(
        derived_12, restated,
        "two more fields must change the derived run; if they do not, the copy truncates"
    );
    // Non-vacuity: the two hand-written comparands are themselves different, so
    // the pair of assertions above cannot both hold by the comparands collapsing.
    assert_ne!(restated, grown, "the two hand-written runs must differ");
}
