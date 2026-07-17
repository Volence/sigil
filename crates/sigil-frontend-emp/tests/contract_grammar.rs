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
