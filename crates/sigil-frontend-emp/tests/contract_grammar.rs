//! Contract-grammar v2 surface grammar — `extern proc` (§3), `type X = proc`
//! contract types (§4), and `@scaffolding` (§8). Byte-neutral: these decls emit
//! nothing; the tests exercise parse shape + validation only.

use sigil_frontend_emp::ast::*;
use sigil_frontend_emp::parse_str;

/// Parse and demand zero diagnostics.
fn ok(src: &str) -> File {
    let (file, diags) = parse_str(src);
    assert!(diags.is_empty(), "diagnostics: {diags:?}");
    file
}

fn externs(f: &File) -> Vec<&ExternProcDecl> {
    f.items.iter().filter_map(|i| match i {
        Item::ExternProc(d) => Some(d),
        _ => None,
    }).collect()
}

/// `extern proc Name () clobbers(d0)` parses to an `Item::ExternProc` carrying
/// the name and the declared clobber reglist — the §3 VSync_Wait boundary decl.
#[test]
fn extern_proc_minimal_clobbers() {
    let f = ok("module engine.system\nextern proc VSync_Wait () clobbers(d0)\n");
    let es = externs(&f);
    assert_eq!(es.len(), 1);
    assert_eq!(es[0].name, "VSync_Wait");
    assert!(!es[0].public);
    assert_eq!(es[0].sig.clobbers, Some(vec![("d0".to_string(), None)]));
    assert!(es[0].sig.params.is_empty());
}

/// The full §3 S4LZ_DecompressDict boundary decl: typed params + a multi-reg
/// clobbers + an `out` (the advances-a1 in-out cursor). All clauses, order-free.
#[test]
fn extern_proc_full_contract() {
    let f = ok("module engine.level\n\
                extern proc S4LZ_DecompressDict (a4: *DictBase, d4) clobbers(a3, a4) out(a1)\n");
    let es = externs(&f);
    assert_eq!(es.len(), 1);
    assert_eq!(es[0].name, "S4LZ_DecompressDict");
    assert_eq!(es[0].sig.params.len(), 2);
    assert_eq!(es[0].sig.params[0].0, "a4");
    assert_eq!(es[0].sig.params[1].0, "d4");
    assert_eq!(
        es[0].sig.clobbers,
        Some(vec![("a3".to_string(), None), ("a4".to_string(), None)])
    );
    assert_eq!(es[0].sig.out, Some(vec![("a1".to_string(), None)]));
}

/// `pub extern proc` — the §3 second-consumer hoist to a shared home.
#[test]
fn extern_proc_pub() {
    let f = ok("module engine.shared\npub extern proc Debug_MusicToggle () clobbers(d0-d2/a0/a1)\n");
    let es = externs(&f);
    assert_eq!(es.len(), 1);
    assert!(es[0].public);
    assert_eq!(
        es[0].sig.clobbers,
        Some(vec![("d0".to_string(), Some("d2".to_string())), ("a0".to_string(), None), ("a1".to_string(), None)])
    );
}

/// `extern` stays an ordinary identifier outside the `extern proc` pair — a
/// comptime `extern("Sym")` read in expression position must not be captured.
#[test]
fn extern_ident_still_usable_as_value_read() {
    // `equ` whose value is a comptime extern read — `extern` here is a call, not
    // a decl opener. Must parse cleanly (no "expected declaration" on `extern`).
    let (_f, diags) = parse_str("module engine.x\nequ Song = extern(\"SongTable\")\n");
    assert!(diags.is_empty(), "diagnostics: {diags:?}");
}

// ---------------------------------------------------------------------------
// §4 contract types: `type Name = proc (params) [clauses]`.
// ---------------------------------------------------------------------------

fn contract_types(f: &File) -> Vec<&ContractTypeDecl> {
    f.items.iter().filter_map(|i| match i {
        Item::ContractType(d) => Some(d),
        _ => None,
    }).collect()
}

/// `type HBlankHandler = proc () clobbers(d0, d1, a0)` — the §4 interrupt-context
/// bound; parses to an `Item::ContractType` with the clobber set.
#[test]
fn contract_type_clobbers_bound() {
    let f = ok("module engine.system\ntype HBlankHandler = proc () clobbers(d0, d1, a0)\n");
    let ts = contract_types(&f);
    assert_eq!(ts.len(), 1);
    assert_eq!(ts[0].name, "HBlankHandler");
    assert!(!ts[0].public);
    assert_eq!(
        ts[0].sig.clobbers,
        Some(vec![("d0".to_string(), None), ("d1".to_string(), None), ("a0".to_string(), None)])
    );
}

/// `type ObjRoutine = proc (a0: *Sst) preserves(a0, d7)` — the object-dispatch
/// bound (preserves a0/d7, everything else clobberable). Typed param + preserves.
#[test]
fn contract_type_preserves_and_typed_param() {
    let f = ok("module engine.core\npub type ObjRoutine = proc (a0: *Sst) preserves(a0, d7)\n");
    let ts = contract_types(&f);
    assert_eq!(ts.len(), 1);
    assert_eq!(ts[0].name, "ObjRoutine");
    assert!(ts[0].public);
    assert_eq!(ts[0].sig.params.len(), 1);
    assert_eq!(ts[0].sig.params[0].0, "a0");
    assert_eq!(
        ts[0].sig.preserves,
        vec![("a0".to_string(), None), ("d7".to_string(), None)]
    );
}

// ---------------------------------------------------------------------------
// §6 out(carry: name) flag results + §6 out(rN if cc) conditional register
// results (G2). A flag result is a status-flag-encoded result the caller MUST
// consume; a conditional register result is valid only on the `cc` path.
// ---------------------------------------------------------------------------

/// `extern proc QueueDMA_Important (d1, d2, d3) clobbers(...) out(carry: dropped)`
/// — the §6 flag result on the extern-proc boundary decl. `carry` is not a
/// register: it lands in `out_flags`, NOT the `out` reglist (which stays empty).
#[test]
fn extern_proc_out_carry_flag_result() {
    let f = ok("module engine.objects.dplc\n\
                extern proc QueueDMA_Important (d1, d2, d3) clobbers(d0-d4/a1-a2) out(carry: dropped)\n");
    let es = externs(&f);
    assert_eq!(es.len(), 1);
    assert_eq!(es[0].sig.out_flags.len(), 1);
    assert_eq!(es[0].sig.out_flags[0].flag, "carry");
    assert_eq!(es[0].sig.out_flags[0].name, "dropped");
    // The flag is NOT a register — the out reglist stays empty (the clause was
    // written, so `Some`, but it declares zero out-REGISTERS).
    assert_eq!(es[0].sig.out, Some(vec![]));
    assert!(es[0].sig.out_cond.is_empty());
}

/// `pub proc RingBuffer_Add () clobbers(d4, a0) out(carry: full)` — the §6 flag
/// result on an INTERNAL proc (same grammar as extern; the "RingBuffer_Add
/// class"). It lands in the proc's `out_flags`.
#[test]
fn proc_out_carry_flag_result() {
    let f = ok("module engine.objects.rings\n\
                pub proc RingBuffer_Add () clobbers(d4, a0) out(carry: full) { rts }\n");
    let p = first_proc(&f);
    assert_eq!(p.out_flags.len(), 1);
    assert_eq!(p.out_flags[0].flag, "carry");
    assert_eq!(p.out_flags[0].name, "full");
    assert_eq!(p.out, Some(vec![]));
}

/// `out(a1 if cc)` — the D2.35 conditional register result: a1 is a real out
/// register (it joins the `out` reglist, so the closure charges it) AND carries
/// its `if cc` validity guard in `out_cond`.
#[test]
fn proc_out_conditional_register_result() {
    let f = ok("module engine.level\n\
                proc AllocDynamic () clobbers(d0) out(a1 if cc) { rts }\n");
    let p = first_proc(&f);
    // a1 is a genuine out register — present in the reglist for the closure.
    assert_eq!(p.out, Some(vec![("a1".to_string(), None)]));
    // ...and carries its cc guard.
    assert_eq!(p.out_cond.len(), 1);
    assert_eq!(p.out_cond[0].reg, "a1");
    assert_eq!(p.out_cond[0].cc, "cc");
    assert!(p.out_flags.is_empty());
}

/// A `proc` may mix a plain out register, a flag result, and a conditional
/// register result in one `out(...)` clause: `out(d0, a1 if cc, carry: dropped)`.
#[test]
fn proc_out_mixed_reg_cond_and_flag() {
    let f = ok("module m\n\
                proc P () clobbers(d1) out(d0, a1 if cc, carry: dropped) { rts }\n");
    let p = first_proc(&f);
    assert_eq!(
        p.out,
        Some(vec![("d0".to_string(), None), ("a1".to_string(), None)])
    );
    assert_eq!(p.out_cond.len(), 1);
    assert_eq!(p.out_cond[0].reg, "a1");
    assert_eq!(p.out_flags.len(), 1);
    assert_eq!(p.out_flags[0].name, "dropped");
}

/// A non-flag name before the colon in `out(...)` is `[proc.out-flag-invalid]`
/// (register-validity mirrors the clobbers/preserves lowering-time check).
#[test]
fn out_flag_invalid_name_is_diagnosed() {
    let (_f, diags) = parse_str(
        "module m\nproc P () clobbers() out(nonsense: x) { rts }\n",
    );
    let (_m, lerrs) = lower_module(
        &_f,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let all: Vec<_> = diags.iter().chain(lerrs.iter()).collect();
    assert!(
        all.iter().any(|d| d.message.contains("[proc.out-flag-invalid]")),
        "diagnostics: {all:?}"
    );
}

/// A bogus condition code in `out(rN if cc)` is `[proc.out-cond-invalid]`.
#[test]
fn out_cond_invalid_cc_is_diagnosed() {
    let (_f, _diags) = parse_str(
        "module m\nproc P () clobbers() out(a1 if zzz) { rts }\n",
    );
    let (_m, lerrs) = lower_module(
        &_f,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        lerrs.iter().any(|d| d.message.contains("[proc.out-cond-invalid]")),
        "diagnostics: {lerrs:?}"
    );
}

/// Flag results and conditional register results are byte-neutral — pure
/// contract metadata, exactly like the G1 boundary grammar.
#[test]
fn flag_and_cond_results_are_byte_neutral() {
    let plain = flatten("module m\nproc P () clobbers(d4, a0) { rts }\n");
    let flagged = flatten("module m\nproc P () clobbers(d4, a0) out(carry: full) { rts }\n");
    assert_eq!(flagged, plain, "out(carry:) must not change emitted bytes");
    let cond = flatten("module m\nproc Q () clobbers(d0) out(a1 if cc) { rts }\n");
    let plainq = flatten("module m\nproc Q () clobbers(d0) out(a1) { rts }\n");
    assert_eq!(cond, plainq, "out(rN if cc) must not change emitted bytes");
}

// ---------------------------------------------------------------------------
// §8 @scaffolding("reason") — item-level attribute, inert metadata in G1.
// ---------------------------------------------------------------------------

fn first_proc(f: &File) -> &ProcDecl {
    f.items.iter().find_map(|i| match i {
        Item::Proc(p) => Some(p),
        _ => None,
    }).expect("a proc")
}

/// `@scaffolding("reason")` on a proc parses and attaches to the proc's attrs
/// with its reason string — the §8 Plane_Buffer_Reset case.
#[test]
fn scaffolding_attr_attaches_to_proc() {
    let f = ok("module engine.render\n\
                @scaffolding(\"VInt_Lag race fix — forward reset hook\")\n\
                pub proc Plane_Buffer_Reset () clobbers() { rts }\n");
    let p = first_proc(&f);
    assert_eq!(p.attrs.len(), 1);
    assert_eq!(p.attrs[0].name, "scaffolding");
    assert_eq!(p.attrs[0].args.len(), 1);
}

/// `@scaffolding` without a reason string is `[scaffolding.reason-required]` —
/// the reason is mandatory (§8).
#[test]
fn scaffolding_requires_reason() {
    let (_f, diags) = parse_str(
        "module engine.render\n@scaffolding()\npub proc P () clobbers() { rts }\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("[scaffolding.reason-required]")),
        "diagnostics: {diags:?}"
    );
}

// ---------------------------------------------------------------------------
// Byte-neutrality: extern proc / contract types / @scaffolding emit NOTHING and
// never change a real proc's bytes (the G1 invariant — contract text is inert).
// ---------------------------------------------------------------------------

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;

fn flatten(src: &str) -> Vec<u8> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse: {perrs:?}");
    let (module, lerrs) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        !lerrs.iter().any(|d| matches!(d.level, sigil_span::Level::Error)),
        "lower errors: {lerrs:?}"
    );
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true).expect("resolve");
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

/// `@scaffolding` is inert: the proc's emitted bytes are identical with and
/// without the attribute (the §8 "inert metadata now" guarantee).
#[test]
fn scaffolding_is_byte_neutral() {
    let without = flatten("module m\nproc P () clobbers() { moveq #0, d0\n rts }\n");
    let with = flatten(
        "module m\n@scaffolding(\"kept for the forward reset hook\")\nproc P () clobbers() { moveq #0, d0\n rts }\n",
    );
    assert_eq!(with, without, "@scaffolding must not change emitted bytes");
}

/// `extern proc` and `type = proc` emit no bytes and no label: a module with
/// them flattens to exactly the same image as one without them.
#[test]
fn boundary_decls_emit_nothing() {
    let bare = flatten("module m\nproc P () clobbers() { rts }\n");
    let decorated = flatten(
        "module m\n\
         extern proc VSync_Wait () clobbers(d0)\n\
         type ObjRoutine = proc (a0: *Sst) preserves(a0, d7)\n\
         proc P () clobbers() { rts }\n",
    );
    assert_eq!(decorated, bare, "boundary decls must emit nothing");
}

// ---------------------------------------------------------------------------
// §4 `as ContractType` dispatch-bound annotation on a call instruction.
// ---------------------------------------------------------------------------

/// `jsr (a1) as ObjRoutine` parses with the instruction carrying its
/// dispatch bound; a bare `jsr (a1)` carries none.
#[test]
fn dispatch_bound_as_annotation_parses() {
    let f = ok("module engine.core\n\
                proc RunObjects () clobbers(d0-d7/a0-a6) {\n\
                    jsr (a1) as ObjRoutine\n\
                    jsr (a2)\n\
                    rts\n\
                }\n");
    let p = first_proc(&f);
    let bounds: Vec<Option<String>> = p.body.iter().filter_map(|s| match s {
        AsmStmt::Instr(i) if i.mnemonic == vec![TextOrSplice::Text("jsr".into())] =>
            Some(i.dispatch_bound.clone()),
        _ => None,
    }).collect();
    assert_eq!(bounds, vec![Some("ObjRoutine".to_string()), None]);
}

/// The `as` annotation is byte-neutral: `jsr (a1) as ObjRoutine` emits the same
/// bytes as `jsr (a1)` (the bound is pure metadata for the closure).
#[test]
fn dispatch_bound_is_byte_neutral() {
    let plain = flatten("module m\nproc P () clobbers(d0-d7/a0-a6) { jsr (a1)\n rts }\n");
    let bound = flatten("module m\nproc P () clobbers(d0-d7/a0-a6) { jsr (a1) as ObjRoutine\n rts }\n");
    assert_eq!(bound, plain, "`as` dispatch bound must not change emitted bytes");
}
