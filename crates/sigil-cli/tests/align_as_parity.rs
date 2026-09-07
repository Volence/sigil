//! D2.29 AS-parity vectors: the same logical layout through the AS
//! front-end's `align` and through `.emp` `align` must produce identical
//! bytes (§4.8; AS `even` ports as `align 2`). Lives in sigil-cli because only
//! sigil-cli / sigil-harness may depend on sigil-frontend-as (crate_graph.rs (c)).
//!
//! THE FIRST THREE VECTORS CANNOT SEE HALF OF WHAT `align` DOES, and that is
//! why the last three exist. Each of the three places a datum AFTER the align
//! and flattens with `0x00`: a pad that writes `$00` bytes and a pad that
//! writes nothing at all render the identical image there, because the write
//! cursor skips a reservation and the next datum's `resize` zeroes the gap. The
//! two effects part only where nothing follows the pad — at a section's TAIL,
//! where a reservation cannot lengthen the image and a fill can — and on the
//! `align` RULE itself at a negative position. The last three vectors are those.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Fragment, SymbolTable};

fn as_image(src: &str) -> Vec<u8> {
    let opts = sigil_frontend_as::Options::default();
    let module = sigil_frontend_as::assemble(src, &opts).expect("AS assemble");
    let linked = sigil_link::link(&module.sections, &SymbolTable::new()).expect("AS link");
    sigil_link::flatten(&linked, 0x00).unwrap()
}

fn emp_image(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(diags.is_empty(), "clean lower: {diags:?}");
    let resolved =
        sigil_link::resolve_layout(&m.sections, &SymbolTable::new(), true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00).unwrap()
}

#[test]
fn align_4_parity() {
    let as_img = as_image("\tcpu 68000\n\tdc.b 1,2,3\n\talign 4\n\tdc.b 9\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 3] = [1, 2, 3]\nalign 4\ndata D2: [u8; 1] = [9]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![1, 2, 3, 0, 9], "and both match the hand-derivation");
}

#[test]
fn align_2_word_data_parity() {
    // NOTE: sigil-frontend-as does not implement `even` at all (probed at
    // tranche 0) — the D2.29 "AS `even` ports as `align 2`" translation is
    // MANDATORY at port time, on both frontends. Parity is over `align 2`.
    let as_img = as_image("\tcpu 68000\n\tdc.b 7\n\talign 2\n\tdc.w $1234\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 1] = [7]\nalign 2\ndata D2: [u16; 1] = [$1234]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![7, 0, 0x12, 0x34]);
}

#[test]
fn already_aligned_parity() {
    let as_img = as_image("\tcpu 68000\n\tdc.b 1,2\n\talign 2\n\tdc.b 9\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 2] = [1, 2]\nalign 2\ndata D2: [u8; 1] = [9]\n");
    assert_eq!(emp_img, as_img);
    assert_eq!(as_img, vec![1, 2, 9], "no pad when already on the boundary");
}

/// The lowered `.emp` sections, before layout — for the two vectors below that
/// ask what the pad IS rather than what a mid-image pad renders as.
fn emp_sections(src: &str) -> Vec<sigil_ir::Section> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(diags.is_empty(), "clean lower: {diags:?}");
    m.sections
}

/// THE EFFECT HALF, and the one the three vectors above are structurally blind
/// to: a TRAILING align cannot lengthen the image.
///
/// asl's object file carries no record at all for the addresses an `align`
/// steps over — p2bin's fill value decides what appears there, and nothing
/// appears past the last datum. `dc.b $11` + `align 4` is ONE byte, not four.
/// The measurement and its asl provenance are recorded at
/// `sigil_frontend_as::eval`'s `directive_align`; the AS arm below re-derives
/// it here rather than trusting the prose, and the literal is the independent
/// hand-derivation of the same rule.
#[test]
fn a_trailing_align_does_not_lengthen_the_image() {
    let as_img = as_image("\tcpu 68000\n\tdc.b $11\n\talign 4\n");
    let emp_img = emp_image("module m\ndata D1: [u8; 1] = [$11]\nalign 4\n");
    assert_eq!(as_img, vec![0x11], "asl emits no byte for an align's own addresses");
    assert_eq!(
        emp_img, as_img,
        "`.emp` align must RESERVE like AS `align`, not fill: a trailing fill lengthens \
         the section by up to n-1 bytes asl never emits"
    );
}

/// The same fact stated at the fragment, so a future change that keeps the
/// bytes by accident (a zero-length pad, a flatten that happens to agree) still
/// reddens: the pad the `.emp` front-end records is a `Reserve`, never a `Fill`.
/// This is the shape `sigil-harness`'s relocation passes discriminate on
/// (`recompute_bank_aligns` / `trim_trailing_align_overshoot` match zero-`Fill`
/// only), so the two front-ends must hand them the same fragment kind.
#[test]
fn an_align_pad_is_reserved_not_filled() {
    let secs = emp_sections("module m\ndata D1: [u8; 3] = [1, 2, 3]\nalign 4\ndata D2: [u8; 1] = [9]\n");
    let fills: Vec<&Fragment> =
        secs.iter().flat_map(|s| s.fragments.iter()).filter(|f| matches!(f, Fragment::Fill { .. })).collect();
    let reserves: Vec<u32> = secs
        .iter()
        .flat_map(|s| s.fragments.iter())
        .filter_map(|f| match f {
            Fragment::Reserve { count, .. } => Some(*count),
            _ => None,
        })
        .collect();
    assert!(fills.is_empty(), "an align pad must place no image byte, found Fill(s): {fills:?}");
    assert_eq!(reserves, vec![1], "one 1-byte reservation, $03 -> $04");
}

/// THE RULE half. Both front-ends compute the pad with `sigil_ir::asl_align_pad`
/// — asl rounds up on the low 32 bits of the position read as a SIGNED `i32`
/// with C's truncating remainder, so a `$FFFF….` position rounds TOWARD ZERO
/// and an already-aligned one advances a full `n`. The expectation is derived by
/// calling that shared function, not transcribed: the vector's job is to prove
/// the `.emp` field path is WIRED to it, which a hand-rolled unsigned round-up
/// would fail (it answers 0 here, where the shared rule answers 256).
///
/// No aeon section declares a RAM `vma:`, so this divergence moves no byte
/// today — inert, not impossible, and this is what keeps it from silently
/// coming back.
#[test]
fn a_negative_vma_align_follows_the_shared_signed_rule() {
    const BASE: u32 = 0xFFFF_B000;
    const N: u32 = 256;
    let secs = emp_sections(&format!(
        "module m\nsection s (cpu: m68000, vma: ${BASE:X}) {{\n  align {N}\n  data D: [u8; 1] = [9]\n}}\n"
    ));
    let reserves: Vec<u32> = secs
        .iter()
        .flat_map(|s| s.fragments.iter())
        .filter_map(|f| match f {
            Fragment::Reserve { count, .. } => Some(*count),
            _ => None,
        })
        .collect();
    let want = sigil_ir::asl_align_pad(BASE, N);
    assert_eq!(want, N, "asl advances an already-aligned NEGATIVE position a full n");
    assert_eq!(
        reserves,
        vec![want],
        "the .emp align must use sigil_ir::asl_align_pad; a plain unsigned round-up answers 0"
    );
}
