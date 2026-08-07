//! `mul_const` / `mul_bounded` through the full front-end (the 2026-08-03
//! cost-model multiply design): parse → eval-time expansion → lowering →
//! link. The unit tests in `mul_lower` pin the cost DECISION; these pin the
//! construct's integration seams — byte identity with the hand spelling where
//! mulu wins, the contract machinery seeing the expansion's REAL writes (the
//! silently-clobbered-scratch failure mode the design names), loop labels
//! surviving hygiene + link, and the per-CPU refusal.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn lower_cpu(src: &str, cpu: Cpu) -> (Module, Vec<Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: cpu, include_root: None, embed_base: None, defines: vec![] },
    )
}

fn lower(src: &str) -> (Module, Vec<Diagnostic>) {
    lower_cpu(src, Cpu::M68000)
}

fn flatten(module: &Module) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

fn errors(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.level == Level::Error).collect()
}

// Where the model picks mulu (every corpus stride), the construct's bytes ARE
// the hand spelling's bytes — the byte-identical adoption rule's foundation.
#[test]
fn mulu_winning_mul_const_is_byte_identical_to_hand_mulu() {
    for n in [36u32, 40, 66, 80, 160] {
        let (m_new, d_new) = lower(&format!(
            "module m\nproc p() clobbers(d0) {{\n    mul_const d0, #{n}\n    rts\n}}\n"
        ));
        let (m_old, d_old) = lower(&format!(
            "module m\nproc p() clobbers(d0) {{\n    mulu.w #{n}, d0\n    rts\n}}\n"
        ));
        assert!(errors(&d_new).is_empty(), "n={n}: {d_new:?}");
        assert!(errors(&d_old).is_empty(), "n={n}: {d_old:?}");
        assert_eq!(flatten(&m_new), flatten(&m_old), "n={n}");
    }
}

// The chosen chain's SCRATCH write is an ordinary detected write: a proc that
// declares only dst warns [proc.clobber-undeclared] on the scratch, and
// declaring both is quiet. This is the design's clobber contract, enforced by
// the EXISTING machinery over the expansion — not by a construct-special path.
#[test]
fn scratch_clobber_is_seen_by_the_clobber_lint() {
    let (_m, diags) = lower(
        "module m\nproc p() clobbers(d0) {\n    mul_const d0, #3, d1\n    rts\n}\n",
    );
    assert!(
        has_tag(&diags, "[proc.clobber-undeclared]"),
        "the chain writes d1; the lint must see it: {diags:?}"
    );
    let (_m, diags) = lower(
        "module m\nproc p() clobbers(d0, d1) {\n    mul_const d0, #3, d1\n    rts\n}\n",
    );
    assert!(
        !has_tag(&diags, "[proc.clobber-undeclared]"),
        "declared scratch is quiet: {diags:?}"
    );
}

// The dst write satisfies a declared `out(d0)` — the write detector sees the
// expansion's real destination writes, so no false [proc.out-unwritten].
#[test]
fn out_contract_sees_the_expanded_writes() {
    let (_m, diags) = lower(
        "module m\nproc p() out(d0) clobbers(d1) {\n    mul_const d0, #3, d1\n    rts\n}\n",
    );
    assert!(errors(&diags).is_empty(), "{diags:?}");
    assert!(!has_tag(&diags, "[proc.out-unwritten]"), "{diags:?}");
}

// The chain expansion, linked: ×3 lowers to the pinned 5-instruction chain —
// 10 bytes of code, golden-pinned here byte-for-byte.
// moveq #0,d1 = 72 00 · move.w d0,d1 = 32 00 · move.l d1,d0 = 20 01 ·
// add.l d0,d0 = D0 80 · add.l d1,d0 = D0 81 · rts = 4E 75.
#[test]
fn chain_bytes_are_pinned() {
    let (module, diags) = lower(
        "module m\nproc p() clobbers(d0, d1) {\n    mul_const d0, #3, d1\n    rts\n}\n",
    );
    assert!(errors(&diags).is_empty(), "{diags:?}");
    assert_eq!(
        flatten(&module),
        vec![0x72, 0x00, 0x32, 0x00, 0x20, 0x01, 0xD0, 0x80, 0xD0, 0x81, 0x4E, 0x75]
    );
}

// The mul_bounded loop candidate: labels are minted uniquely, the branches
// resolve, and the whole body links. Two constructs in one proc prove label
// uniqueness (the evaluator counter separates them).
#[test]
fn bounded_loop_labels_link_and_are_unique() {
    let src = "module m\nproc p() clobbers(d0, d1, d2, d3) {\n\
        \x20   mul_bounded d0, d1, #2, d2\n\
        \x20   mul_bounded d0, d3, #2, d2\n\
        \x20   rts\n}\n";
    let (module, diags) = lower(src);
    assert!(errors(&diags).is_empty(), "{diags:?}");
    let bytes = flatten(&module);
    // setup(8) + bcs(2) + add(2) + dbf(4) per construct, + rts: 2 × 16 + 2.
    assert_eq!(bytes.len(), 34, "two loop lowerings plus rts");
}

// mul_bounded above the loop boundary is exactly `mulu.w src,dst`.
#[test]
fn bounded_above_boundary_is_byte_identical_to_mulu() {
    let (m_new, d_new) = lower(
        "module m\nproc p() clobbers(d0, d1) {\n    mul_bounded d0, d1, #16\n    rts\n}\n",
    );
    let (m_old, d_old) = lower(
        "module m\nproc p() clobbers(d0, d1) {\n    mulu.w d1, d0\n    rts\n}\n",
    );
    assert!(errors(&d_new).is_empty(), "{d_new:?}");
    assert!(errors(&d_old).is_empty(), "{d_old:?}");
    assert_eq!(flatten(&m_new), flatten(&m_old));
}

// Whole-pipeline determinism: same source, same bytes, twice.
#[test]
fn lowering_is_deterministic_end_to_end() {
    let src = "module m\nproc p() clobbers(d0, d1, d2) {\n\
        \x20   mul_const d0, #66, d1\n\
        \x20   mul_bounded d0, d2, #2, d1\n\
        \x20   rts\n}\n";
    let (m1, d1) = lower(src);
    let (m2, d2) = lower(src);
    assert!(errors(&d1).is_empty(), "{d1:?}");
    assert_eq!(d1.len(), d2.len());
    assert_eq!(flatten(&m1), flatten(&m2));
}

// Z80: a loud [mul.non-68k] error, not a silent drop or a confusing
// unknown-mnemonic fallback.
#[test]
fn z80_bodies_refuse_the_construct() {
    let (_m, diags) = lower_cpu(
        "module m\nproc f() {\n    mul_const d0, #66\n    ret\n}\n",
        Cpu::Z80,
    );
    assert!(has_tag(&diags, "[mul.non-68k]"), "{diags:?}");
}

// The diagnostic surface through the full pipeline: range, aliasing, size.
#[test]
fn refusals_surface_through_the_pipeline() {
    let (_m, diags) =
        lower("module m\nproc p() {\n    mul_const d0, #65536\n    rts\n}\n");
    assert!(has_tag(&diags, "[mul.const-range]"), "{diags:?}");
    let (_m, diags) =
        lower("module m\nproc p() {\n    mul_const d0, #66, d0\n    rts\n}\n");
    assert!(has_tag(&diags, "[mul.scratch-aliases]"), "{diags:?}");
    // `.w` is now a valid contract; `.l` (and any other suffix) refuses.
    let (_m, diags) =
        lower("module m\nproc p() {\n    mul_const.l d0, #66\n    rts\n}\n");
    assert!(has_tag(&diags, "[mul.size]"), "{diags:?}");
    let (_m, diags) =
        lower("module m\nproc p() {\n    mul_bounded d0, d1\n    rts\n}\n");
    assert!(has_tag(&diags, "[mul.operands]"), "{diags:?}");
}

// The word contract through the full pipeline: `mul_const.w` at each corpus
// stride emits BYTE-IDENTICAL code to the hand-derived left-to-right shift-add
// chain. The LTR arm is a candidate at 2 set bits, and at each of these strides
// it is the only chain the generator offers, so `choose()` takes it over
// `mulu.w` on cycles.
#[test]
fn word_strides_are_byte_identical_to_hand_chains() {
    // (n, first-shift, second-shift) of `move.w d0,d1 / lsl.w #p,d0 /
    // add.w d1,d0 / <tail ×2^q>`: 66 → (5,1), 80 → (2,4), 160 → (2,5).
    // A q = 1 tail is a single doubling and is spelled `add.w d0,d0` (4 cycles)
    // rather than `lsl.w #1,d0` (8) — same two bytes, so ×66 is the stride this
    // preference actually reaches.
    for (n, p, q) in [(66u32, 5, 1), (80, 2, 4), (160, 2, 5)] {
        let tail = if q == 1 {
            "    add.w d0, d0\n".to_string()
        } else {
            format!("    lsl.w #{q}, d0\n")
        };
        let (m_new, d_new) = lower(&format!(
            "module m\nproc p() clobbers(d0, d1) {{\n    mul_const.w d0, #{n}, d1\n    rts\n}}\n"
        ));
        let (m_old, d_old) = lower(&format!(
            "module m\nproc p() clobbers(d0, d1) {{\n\
             \x20   move.w d0, d1\n\
             \x20   lsl.w #{p}, d0\n\
             \x20   add.w d1, d0\n\
             {tail}\
             \x20   rts\n}}\n"
        ));
        assert!(errors(&d_new).is_empty(), "n={n}: {d_new:?}");
        assert!(errors(&d_old).is_empty(), "n={n}: {d_old:?}");
        assert_eq!(flatten(&m_new), flatten(&m_old), "n={n} not byte-identical");
    }
}

// The word chain's scratch write is seen by the clobber lint exactly like the
// long form's — the word path shares expand_item's authorship/analysis tail.
#[test]
fn word_scratch_clobber_is_seen_by_the_clobber_lint() {
    let (_m, diags) = lower(
        "module m\nproc p() clobbers(d0) {\n    mul_const.w d0, #66, d1\n    rts\n}\n",
    );
    assert!(
        has_tag(&diags, "[proc.clobber-undeclared]"),
        "the word chain writes d1; the lint must see it: {diags:?}"
    );
}

// The TEMPLATE path (the corpus's `mul_cache_stride` shape): a `mul_const.w`
// inside a comptime fn returning `asm {}` is a cpu-less template that DEFERS and
// expands at the caller's CodeBuf completion — its bytes must equal the
// statement-position spelling exactly (both adoption call sites depend on this).
#[test]
fn word_through_a_comptime_asm_template_is_byte_identical() {
    let via_fn = "module m\n\
        comptime fn stride(d: Reg, s: Reg) -> Code {\n\
        \x20   return asm { mul_const.w {d}, #80, {s} }\n\
        }\n\
        proc p() clobbers(d0, d1) {\n\
        \x20   stride(d0, d1)\n\
        \x20   rts\n}\n";
    let direct = "module m\nproc p() clobbers(d0, d1) {\n\
        \x20   mul_const.w d0, #80, d1\n\
        \x20   rts\n}\n";
    let (m_fn, d_fn) = lower(via_fn);
    let (m_direct, d_direct) = lower(direct);
    assert!(errors(&d_fn).is_empty(), "{d_fn:?}");
    assert!(errors(&d_direct).is_empty(), "{d_direct:?}");
    assert_eq!(flatten(&m_fn), flatten(&m_direct));
}

// `sizeof(...)` is an ordinary comptime integer in the multiplier position, so a
// stride can DERIVE from the struct it walks instead of restating its size. The
// derivation is what earns the spelling: change the struct and the multiplier
// changes with it, which is the property a restated stride cannot offer.
#[test]
fn a_sizeof_multiplier_derives_from_the_struct() {
    // Two layouts of one struct, 26 and 22 bytes — the same shape and the same
    // one-field difference the corpus's own EntityScanState grew.
    let via_sizeof = |size: &str, tail: &str| {
        format!(
            "module m\n\
             struct S (size: {size}) {{\n\
             \x20   a: u32 @ $00,\n\
             \x20   b: u32 @ $04,\n\
             \x20   c: u32 @ $08,\n\
             \x20   d: u32 @ $0C,\n\
             \x20   e: u32 @ $10,\n\
             \x20   f: u16 @ $14,\n\
             {tail}\
             }}\n\
             proc p() clobbers(d0, d1) {{\n\
             \x20   mul_const.w d0, #sizeof(S), d1\n\
             \x20   rts\n}}\n"
        )
    };
    let via_literal = |n: u32| {
        format!(
            "module m\nproc p() clobbers(d0, d1) {{\n\
             \x20   mul_const.w d0, #{n}, d1\n\
             \x20   rts\n}}\n"
        )
    };

    // Accepted, and identical to the literal spelling of the same number.
    let (m_sizeof, d_sizeof) = lower(&via_sizeof("$1A", "    g: u32 @ $16,\n"));
    let (m_literal, d_literal) = lower(&via_literal(26));
    assert!(errors(&d_sizeof).is_empty(), "sizeof multiplier refused: {d_sizeof:?}");
    assert!(errors(&d_literal).is_empty(), "{d_literal:?}");
    assert_eq!(flatten(&m_sizeof), flatten(&m_literal), "sizeof($1A) != #26");

    // NON-VACUITY, the half that matters: a different struct size produces
    // different bytes, and they are the bytes of THAT size's literal. A spelling
    // that compiled but ignored the struct would pass the assertion above.
    let (m_shrunk, d_shrunk) = lower(&via_sizeof("$16", ""));
    let (m_lit22, d_lit22) = lower(&via_literal(22));
    assert!(errors(&d_shrunk).is_empty(), "{d_shrunk:?}");
    assert!(errors(&d_lit22).is_empty(), "{d_lit22:?}");
    assert_ne!(flatten(&m_sizeof), flatten(&m_shrunk), "the multiplier ignored the struct");
    assert_eq!(flatten(&m_shrunk), flatten(&m_lit22), "sizeof($16) != #22");
}

// `.b` (like `.l`) refuses — a byte-width multiply is an unratified contract.
#[test]
fn word_byte_suffix_refuses() {
    let (_m, diags) =
        lower("module m\nproc p() {\n    mul_const.b d0, #66\n    rts\n}\n");
    assert!(has_tag(&diags, "[mul.size]"), "{diags:?}");
}
