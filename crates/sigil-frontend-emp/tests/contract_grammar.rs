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

/// A full §3 boundary decl: typed params + a multi-reg clobbers + an `out`. The
/// SHAPE is what this pins; the register set is a fixture, not a copy of any
/// current proc's contract. All clauses, order-free.
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
// §6 / §11 Q3 — @discards(name) trailing attribute on a call: the explicit,
// greppable opt-out of the flag-result must-use check.
// ---------------------------------------------------------------------------

/// `jbsr QueueDMA_Important @discards(dropped)` parses with the call instruction
/// carrying `discards = Some("dropped")`; a plain call carries `None`.
#[test]
fn discards_trailing_attribute_parses() {
    let f = ok("module engine.x\n\
                proc P () clobbers(d0-d4/a1-a2) {\n\
                    jbsr QueueDMA_Important @discards(dropped)\n\
                    jbsr Other\n\
                    rts\n\
                }\n");
    let p = first_proc(&f);
    let discards: Vec<Option<String>> = p.body.iter().filter_map(|s| match s {
        AsmStmt::Instr(i) if i.mnemonic == vec![TextOrSplice::Text("jbsr".into())] =>
            Some(i.discards.clone()),
        _ => None,
    }).collect();
    assert_eq!(discards, vec![Some("dropped".to_string()), None]);
}

/// `@discards` is byte-neutral: it emits the same bytes as the plain call (pure
/// metadata for the flag-result must-use check).
#[test]
fn discards_is_byte_neutral() {
    let plain = flatten("module m\nproc P () clobbers(d0) { bsr Sub\n rts }\nproc Sub () clobbers(d0) { rts }\n");
    let disc = flatten("module m\nproc P () clobbers(d0) { bsr Sub @discards(dropped)\n rts }\nproc Sub () clobbers(d0) { rts }\n");
    assert_eq!(disc, plain, "@discards must not change emitted bytes");
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

/// S2-D6 U4 — `@allow("clobbers.unanalyzable", "<reason>")` parses with both
/// args. A LEADING attr routes to MODULE scope (`file.attrs`, exactly like
/// `layout.odd-field` — the parser's greedy module-attr loop), which the closure
/// honors for every proc in the module.
#[test]
fn unanalyzable_allow_parses_at_module_scope() {
    let f = ok("module engine.dispatch\n\
                @allow(\"clobbers.unanalyzable\", \"raw trampoline, open target set\")\n\
                pub proc Trampoline () clobbers(d0) { jsr (a1) }\n");
    assert_eq!(f.attrs.len(), 1, "leading @allow attaches at module scope: {:?}", f.attrs);
    assert_eq!(f.attrs[0].name, "allow");
    assert_eq!(f.attrs[0].args.len(), 2);
}

/// S2-D6 U4 — a NON-leading `@allow` (after another item) attaches at PROC scope
/// (`p.attrs`); the closure honors the annotation in either scope.
#[test]
fn unanalyzable_allow_parses_at_proc_scope() {
    let f = ok("module engine.dispatch\n\
                pub proc First () clobbers() { rts }\n\
                @allow(\"clobbers.unanalyzable\", \"open target set\")\n\
                pub proc Trampoline () clobbers(d0) { jsr (a1) }\n");
    let tramp = f
        .items
        .iter()
        .find_map(|i| match i {
            Item::Proc(p) if p.name == "Trampoline" => Some(p),
            _ => None,
        })
        .expect("Trampoline proc");
    assert_eq!(tramp.attrs.len(), 1, "non-leading @allow attaches at proc scope: {:?}", tramp.attrs);
    assert_eq!(tramp.attrs[0].name, "allow");
}

/// S2-D6 U4 — `@allow("clobbers.unanalyzable")` WITHOUT a reason is
/// `[clobbers.unanalyzable-reason-required]` (the reason is mandatory, mirroring
/// `@scaffolding`). An empty reason string is equally rejected.
#[test]
fn unanalyzable_allow_requires_reason() {
    let (_f, diags) = parse_str(
        "module engine.dispatch\n@allow(\"clobbers.unanalyzable\")\npub proc P () clobbers(d0) { jsr (a1) }\n",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("[clobbers.unanalyzable-reason-required]")),
        "missing reason must be rejected: {diags:?}"
    );
    let (_f2, diags2) = parse_str(
        "module engine.dispatch\n@allow(\"clobbers.unanalyzable\", \"\")\npub proc P () clobbers(d0) { jsr (a1) }\n",
    );
    assert!(
        diags2.iter().any(|d| d.message.contains("[clobbers.unanalyzable-reason-required]")),
        "empty reason must be rejected: {diags2:?}"
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

// ---------------------------------------------------------------------------
// The derived canonical out views (`ProcDecl::unconditional_outs` /
// `cond_out_regs`) — the ONE place the conditional/unconditional split lives.
// ---------------------------------------------------------------------------

use sigil_frontend_emp::regfile::RegFile;
use std::collections::BTreeSet;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// The parser folds an `out(rN if cc)` register into BOTH `out_cond` and the
/// plain `out` reglist; `unconditional_outs` is the view with the guarded ones
/// subtracted, and `cond_out_regs` is the guarded set itself.
#[test]
fn unconditional_outs_subtracts_the_guarded_registers() {
    let f = ok("module m\nproc P () clobbers(d1) out(d0, a1 if eq) { rts }\n");
    let p = first_proc(&f);
    assert_eq!(p.unconditional_outs(RegFile::M68k), set(&["d0"]));
    assert_eq!(p.cond_out_regs(RegFile::M68k), set(&["a1"]));
}

/// Ranges expand before the subtraction: `out(d0-d2, d1 if eq)` leaves d0+d2.
#[test]
fn unconditional_outs_expands_ranges_before_subtracting() {
    let f = ok("module m\nproc P () clobbers(a0) out(d0-d2, d1 if eq) { rts }\n");
    let p = first_proc(&f);
    assert_eq!(p.unconditional_outs(RegFile::M68k), set(&["d0", "d2"]));
}

/// The subtraction is CANONICAL, not textual: `sp` and `a7` are one register, so
/// `out(sp if eq)` leaves nothing unconditional. A raw-text subtraction compares
/// `{"a7"}` against `{"sp"}` and credits the conditional result as unconditional.
#[test]
fn unconditional_outs_is_canonical_not_textual() {
    let f = ok("module m\nproc P () clobbers(d0) out(sp if eq) { rts }\n");
    let p = first_proc(&f);
    assert_eq!(p.cond_out_regs(RegFile::M68k), set(&["a7"]));
    assert!(p.unconditional_outs(RegFile::M68k).is_empty());
}

/// A proc with no `out(...)` at all has empty views (no `Option` ceremony at the
/// call sites).
#[test]
fn unconditional_outs_of_a_proc_with_no_out_is_empty() {
    let f = ok("module m\nproc P () clobbers(d0) { rts }\n");
    let p = first_proc(&f);
    assert!(p.unconditional_outs(RegFile::M68k).is_empty());
    assert!(p.cond_out_regs(RegFile::M68k).is_empty());
}

/// The same views on a Z80 signature expand PAIR sugar to halves — `out(hl if z)`
/// guards `h` and `l`, so neither survives into the unconditional set.
#[test]
fn unconditional_outs_expands_z80_pairs() {
    let f = ok("module m (cpu: z80)\n\
                extern proc P () out(bc, hl if z)\n");
    let sig = &externs(&f)[0].sig;
    let guarded: BTreeSet<String> = sig.cond_out_guards(RegFile::Z80).into_keys().collect();
    assert_eq!(guarded, set(&["h", "l"]));
    assert_eq!(sig.unconditional_outs(RegFile::Z80), set(&["b", "c"]));
}

/// `cond_out_guards` keys EVERY guarded register — a mixed mention keeps its key,
/// the axis it differs from `cond_out_pairs` on — and carries the CONDITION each
/// one is guarded by, the fact the §4 subcontract relation compares. Two guard
/// clauses on one register contribute both codes.
#[test]
fn cond_out_guards_carry_each_registers_condition_codes() {
    let f = ok("module m\nextern proc P () clobbers(d1) out(d0, a1 if eq, a2 if mi, a2 if pl)\n");
    let sig = &externs(&f)[0].sig;
    let guards = sig.cond_out_guards(RegFile::M68k);
    assert_eq!(guards.keys().cloned().collect::<BTreeSet<_>>(), set(&["a1", "a2"]));
    assert_eq!(guards["a1"], set(&["eq"]));
    assert_eq!(guards["a2"], set(&["mi", "pl"]));
    assert_eq!(sig.unconditional_outs(RegFile::M68k), set(&["d0"]));
}

/// The condition is CANONICAL, not textual: `hs`/`lo` are the documented aliases
/// of `cc`/`cs`, so a guard spelled either way must compare equal. A raw-text
/// comparison would reject a target guarding `hs` against a bound spelling `cc`.
#[test]
fn cond_out_guards_fold_the_cc_aliases() {
    let f = ok("module m\nextern proc P () clobbers(d1) out(a1 if hs, a2 if lo)\n");
    let sig = &externs(&f)[0].sig;
    let guards = sig.cond_out_guards(RegFile::M68k);
    assert_eq!(guards["a1"], set(&["cc"]));
    assert_eq!(guards["a2"], set(&["cs"]));
}
