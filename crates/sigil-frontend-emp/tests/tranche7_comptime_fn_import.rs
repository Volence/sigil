//! Tranche 7 — F3: cross-module import of `pub comptime fn`.
//!
//! A `pub comptime fn template(...) -> Code` in module A must be importable by a
//! consumer module B via `use a.{template}` and callable in B's proc bodies. A
//! comptime fn is a comptime-only item — no bytes, no link symbol — so the
//! consumer just needs the DECL visible (the resolver injects it ambiently). The
//! gap this closes: `imports.rs` skipped `Item::ComptimeFn` in both the pub-name
//! export index and the all-names listing, so the `use` could not resolve.
//!
//! Also pins the F2 interaction — the production shape: a caller passes its own
//! `.local` label to an IMPORTED template and hygiene keeps the caller's and the
//! template's label spaces distinct.

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// Build a multi-module program; return its concatenated sections + diagnostics.
fn build(files: &[(&str, &str)], entry: &str) -> (Vec<Section>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(
        mdiags.iter().all(|d| d.level != Level::Error),
        "manifest errors: {:?}",
        mdiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (sections, _asserts, diags) = build_program(&manifest, entry, None, &opts);
    (sections, diags)
}

fn errors(diags: &[Diagnostic]) -> Vec<&str> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.as_str()).collect()
}

/// Flatten the whole program (all sections) to a single byte image.
fn flatten(sections: &[Section]) -> Vec<u8> {
    let resolved = sigil_link::resolve_layout(sections, &SymbolTable::new(), true)
        .expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

// A defines the pub template; B imports and calls it.
const A_TEMPLATE: &str = "\
module a
pub comptime fn jump(off: int, breg: Reg, dst: Reg) -> Code {
    return asm {
        move.w  {off}({breg}), {dst}
        sub.w   {off}({breg}), {dst}
    }
}
";

const B_CONSUMER: &str = "\
module b
use a.{jump}
pub proc Consumer () {
    jump(2, a3, d1)
    jump(8, a2, d0)
    rts
}
";

// Same as B, but the template is INLINED (single-module control): the import
// must produce byte-identical output to the inlined equivalent.
const INLINE_CONTROL: &str = "\
module b
comptime fn jump(off: int, breg: Reg, dst: Reg) -> Code {
    return asm {
        move.w  {off}({breg}), {dst}
        sub.w   {off}({breg}), {dst}
    }
}
pub proc Consumer () {
    jump(2, a3, d1)
    jump(8, a2, d0)
    rts
}
";

#[test]
fn imported_comptime_fn_resolves_and_calls() {
    let (_secs, diags) = build(&[("a.emp", A_TEMPLATE), ("b.emp", B_CONSUMER)], "b");
    assert!(errors(&diags).is_empty(), "import/call errors: {:?}", errors(&diags));
}

#[test]
fn imported_comptime_fn_bytes_match_inlined() {
    let (imp_secs, id) = build(&[("a.emp", A_TEMPLATE), ("b.emp", B_CONSUMER)], "b");
    assert!(errors(&id).is_empty(), "import errors: {:?}", errors(&id));
    let (inl_secs, ld) = build(&[("b.emp", INLINE_CONTROL)], "b");
    assert!(errors(&ld).is_empty(), "inline errors: {:?}", errors(&ld));
    assert_eq!(
        flatten(&imp_secs),
        flatten(&inl_secs),
        "imported-template bytes must equal the inlined equivalent"
    );
}

// ---- F3 negative: a non-pub comptime fn is NOT importable -------------------

const A_PRIVATE: &str = "\
module a
comptime fn secret(off: int, breg: Reg, dst: Reg) -> Code {
    return asm { move.w {off}({breg}), {dst} }
}
";

const B_WANTS_PRIVATE: &str = "\
module b
use a.{secret}
pub proc Consumer () {
    secret(2, a3, d1)
    rts
}
";

#[test]
fn non_pub_comptime_fn_is_not_importable() {
    let (_secs, diags) = build(&[("a.emp", A_PRIVATE), ("b.emp", B_WANTS_PRIVATE)], "b");
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("secret") && (e.contains("no `pub`") || e.contains("pub"))),
        "expected a loud unknown-import diagnostic naming `secret`, got: {errs:?}"
    );
}

// ---- F3 x F2: imported template + caller's own .local label -----------------
//
// The production shape (aabb.emp's aabb_axis_test called from collision.emp with
// `.next_object`): a caller passes its OWN proc-local label to an IMPORTED
// template that ALSO defines its own internal `.aov`. Hygiene must keep the two
// label spaces distinct.

const A_AABB: &str = "\
module a
pub comptime fn axis(apos: Reg, breg: Reg, boff: int, cdim: Reg, delt: Reg, stmp: Reg, mlab: Label) -> Code {
    return asm {
        move.w  {apos}, {cdim}
        add.w   {delt}, {cdim}
        move.w  {apos}, {delt}
        sub.w   {boff}({breg}), {delt}
        move.w  {delt}, {stmp}
        bpl.s   .aov
        neg.w   {stmp}
    .aov:
        add.w   {stmp}, {stmp}
        cmp.w   {cdim}, {stmp}
        bhs.s   {mlab}
    }
}
";

const B_COLLISION: &str = "\
module b
use a.{axis}
pub proc Collision () {
    axis(d0, a3, 2, d1, d2, d3, .next_object)
    nop
    axis(d0, a2, 6, d1, d2, d3, .next_object)
    nop
.next_object:
    rts
}
";

#[test]
fn imported_template_with_caller_local_label_hygiene() {
    let (secs, diags) = build(&[("a.emp", A_AABB), ("b.emp", B_COLLISION)], "b");
    assert!(errors(&diags).is_empty(), "hygiene lower errors: {:?}", errors(&diags));
    // Link must succeed: the two internal `.aov` (fresh per instantiation) and the
    // caller's `.next_object` all resolve against distinct mangled symbols.
    let img = flatten(&secs);
    assert!(!img.is_empty());
    // The caller's `.next_object` and the template's `.aov` are DISTINCT symbols.
    let text = secs.iter().find(|s| s.name == "text").expect("text section");
    let aov = text.labels.iter().filter(|l| l.name.contains("aov")).count();
    let next = text.labels.iter().filter(|l| l.name.contains("next_object")).count();
    assert!(aov >= 2, "expected 2 fresh internal .aov labels, got {aov}: {:?}", text.labels);
    assert!(next >= 1, "expected the caller's .next_object label, got {next}");
}

// ---- F3 scope boundary: the DEEP case fails LOUDLY --------------------------
//
// The SIMPLE case shipped here is a comptime fn whose body uses only its params
// (aabb_axis_test's exact shape). A fn body that references its HOME module's
// own consts is a DEEPER resolution problem — the ambient injection makes the fn
// DECL visible in the consumer, but the fn body then evaluates in the CONSUMER's
// namespace, where the home module's (un-imported) private const is unknown. The
// contract is that this case fails LOUDLY (naming the symbol), never silently
// miscompiles. Flagged in the tranche-7 report as a follow-up (canonicalizing an
// injected fn body's home-module references is a rename-pass extension).

const A_DEEP: &str = "\
module a
const HIDDEN = 7
pub comptime fn addk(dst: Reg) -> Code {
    return asm { addi.w #HIDDEN, {dst} }
}
";
const B_DEEP: &str = "\
module b
use a.{addk}
pub proc Consumer () {
    addk(d0)
    rts
}
";

#[test]
fn imported_fn_referencing_home_private_const_fails_loudly() {
    let (_secs, diags) = build(&[("a.emp", A_DEEP), ("b.emp", B_DEEP)], "b");
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|e| e.contains("HIDDEN")),
        "the deep cross-ref case must fail LOUDLY naming the symbol, got: {errs:?}"
    );
}
