//! The L1 game-contract bind pass (§3): resolve each `interface` against its one
//! `implement` block across the module set, producing the
//! [`InterfaceEnv`](crate::contract::InterfaceEnv) the evaluator consumes.
//!
//! This is the ML-functor shape — the engine is parameterized over ONE `Game`
//! structure — not typeclass dispatch. The registry requires exactly one
//! `implement` per declared interface; zero and two are the loud
//! `[contract.unimplemented]` / `[contract.duplicate-impl]` errors. Each member
//! is typed against its declaration: a const binds against its declared type, a
//! proc against its declared proc contract type, and a hook against its declared
//! signature (the §4 subcontract relation, reusing
//! [`crate::closure::subcontract_violations`]).
use crate::ast;
use crate::closure::{subcontract_violations, Contract};
use crate::contract::{InterfaceEnv, ResolvedInterface, ResolvedMember};
use crate::regfile::{expand_reglist, RegFile};
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;

/// One module in the bind set: its id (for messages) and its parsed file.
pub struct ContractModule<'a> {
    /// The module's dotted id.
    pub id: &'a str,
    /// The module's parsed file.
    pub file: &'a ast::File,
}

/// Bind every `interface` in `modules` against its one `implement`, returning the
/// resolved environment plus every diagnostic. `defines` is the build-shape `-D`
/// environment consulted by comptime-`if` binding groups and const values.
///
/// Binding values evaluate against each impl FILE alone — the production resolve
/// pass hands this ambient-injected module clones, so imported consts resolve.
/// A caller without that injection (the corpus walk) supplies the imports as an
/// explicit ambient via [`bind_with_ambient`].
pub fn bind(modules: &[ContractModule<'_>], defines: &[(String, i128)]) -> (InterfaceEnv, Vec<Diagnostic>) {
    bind_with_ambient(modules, defines, &[])
}

/// [`bind`] with a cross-file `ambient` declaration slice consulted by every
/// binding-value / group-condition evaluation (the corpus walk's PASS-1
/// environment — see [`crate::corpus_contracts::bind_corpus_interfaces`]).
pub fn bind_with_ambient(
    modules: &[ContractModule<'_>],
    defines: &[(String, i128)],
    ambient: &[ast::Item],
) -> (InterfaceEnv, Vec<Diagnostic>) {
    let mut diags = Vec::new();

    // Collect every interface declaration (name -> decl + owner), and every
    // implement block (interface name -> list of impl + owner file), across the
    // module set (top-level and one level into `section {}` bodies).
    let mut interfaces: HashMap<String, (&ast::InterfaceDecl, &str)> = HashMap::new();
    let mut implements: HashMap<String, Vec<(&ast::ImplementDecl, &ast::File, &str)>> = HashMap::new();
    let mut all_items: Vec<&ast::Item> = Vec::new();

    for m in modules {
        collect(&m.file.items, m.id, m.file, &mut interfaces, &mut implements, &mut all_items, &mut diags);
    }

    let mut env = InterfaceEnv::empty();

    for (iname, (idecl, iowner)) in &interfaces {
        let impls = implements.get(iname).map(Vec::as_slice).unwrap_or(&[]);
        match impls.len() {
            0 => {
                diags.push(err(
                    idecl.span,
                    format!(
                        "[contract.unimplemented] interface `{iname}` (declared in `{iowner}`) has \
                         no `implement` block in this build, a game must implement it"
                    ),
                ));
            }
            1 => {
                let (impl_decl, impl_file, _impl_owner) = impls[0];
                let resolved = resolve_one(
                    idecl, impl_decl, impl_file, &all_items, defines, ambient, &mut diags,
                );
                env.interfaces.insert(iname.clone(), resolved);
            }
            _ => {
                let sites: Vec<String> = impls.iter().map(|(_, _, o)| format!("`{o}`")).collect();
                diags.push(err(
                    impls[1].0.span,
                    format!(
                        "[contract.duplicate-impl] interface `{iname}` is implemented more than \
                         once ({}), exactly one `implement` is allowed per build",
                        sites.join(", ")
                    ),
                ));
            }
        }
    }

    // An `implement` naming an interface that does not exist is itself an error
    // (the reverse of unimplemented) — surface it so a typo'd interface name is
    // loud rather than silently dropped.
    for (iname, impls) in &implements {
        if !interfaces.contains_key(iname) {
            diags.push(err(
                impls[0].0.span,
                format!("[contract.unknown-member] `implement {iname}` names no declared interface"),
            ));
        }
    }

    (env, diags)
}

/// Resolve one `implement` block against its interface declaration.
#[allow(clippy::too_many_arguments)] // internal driver; mirrors bind's state set
fn resolve_one(
    idecl: &ast::InterfaceDecl,
    impl_decl: &ast::ImplementDecl,
    impl_file: &ast::File,
    all_items: &[&ast::Item],
    defines: &[(String, i128)],
    ambient: &[ast::Item],
    diags: &mut Vec<Diagnostic>,
) -> ResolvedInterface {
    // Flatten the binding list, honoring comptime-`if` groups (item-7a precedent).
    let mut flat: Vec<&ast::ImplBinding> = Vec::new();
    flatten_bindings(&impl_decl.bindings, impl_file, defines, ambient, diags, &mut flat);

    // Index the chosen bindings by member name (checking each against the
    // interface's declared members: unknown member, kind mismatch).
    let mut bound: HashMap<String, &ast::ImplBinding> = HashMap::new();
    for b in &flat {
        let (bname, bspan, bkind) = binding_head(b);
        let Some(member) = idecl.members.iter().find(|m| m.name == bname) else {
            diags.push(err(
                bspan,
                format!(
                    "[contract.unknown-member] interface `{}` declares no member `{bname}`",
                    idecl.name
                ),
            ));
            continue;
        };
        if !kind_matches(&member.kind, bkind) {
            diags.push(err(
                bspan,
                format!(
                    "[contract.member-kind] member `{}.{bname}` is declared {}, but is bound as {}",
                    idecl.name,
                    member_kind_word(&member.kind),
                    bkind
                ),
            ));
            continue;
        }
        bound.insert(bname.to_string(), b);
    }

    // For each DECLARED member, resolve its binding (or the default / a
    // missing-member error).
    let mut members: HashMap<String, ResolvedMember> = HashMap::new();
    for member in &idecl.members {
        match &member.kind {
            ast::InterfaceMemberKind::Const(ty) => {
                match bound.get(&member.name) {
                    Some(ast::ImplBinding::Const { value, .. }) => {
                        let (v, mut ds) =
                            crate::eval::eval_expr_in_file_ambient(impl_file, ambient, value, defines);
                        diags.append(&mut ds);
                        check_const_type(&idecl.name, &member.name, ty, &v, member.span, diags);
                        members.insert(member.name.clone(), ResolvedMember::Const(v));
                    }
                    _ => missing(&idecl.name, member, diags),
                }
            }
            ast::InterfaceMemberKind::Proc(ptype) => {
                match bound.get(&member.name) {
                    Some(ast::ImplBinding::Proc { symbol, span, .. }) => {
                        let sym = link_symbol(symbol);
                        check_proc_type(&idecl.name, &member.name, ptype, &sym, all_items, *span, diags);
                        members.insert(member.name.clone(), ResolvedMember::Proc(sym));
                    }
                    _ => missing(&idecl.name, member, diags),
                }
            }
            ast::InterfaceMemberKind::Hook { sig, default_empty } => {
                match bound.get(&member.name) {
                    Some(ast::ImplBinding::Hook { symbol, span, .. }) => {
                        let sym = link_symbol(symbol);
                        check_hook_sig(&idecl.name, &member.name, sig, &sym, all_items, *span, diags);
                        members.insert(member.name.clone(), ResolvedMember::Hook(Some(sym)));
                    }
                    None if *default_empty => {
                        // `= empty` default: an unbound hook's call site emits nothing.
                        members.insert(member.name.clone(), ResolvedMember::Hook(None));
                    }
                    _ => missing(&idecl.name, member, diags),
                }
            }
        }
    }

    ResolvedInterface { members }
}

/// Flatten an `implement` binding list, evaluating each comptime-`if` group's
/// condition against the impl file + defines and recursing into the chosen arm.
fn flatten_bindings<'a>(
    bindings: &'a [ast::ImplBinding],
    impl_file: &ast::File,
    defines: &[(String, i128)],
    ambient: &[ast::Item],
    diags: &mut Vec<Diagnostic>,
    out: &mut Vec<&'a ast::ImplBinding>,
) {
    for b in bindings {
        match b {
            ast::ImplBinding::Group { cond, then_body, else_body, span } => {
                let (v, mut ds) =
                    crate::eval::eval_expr_in_file_ambient(impl_file, ambient, cond, defines);
                diags.append(&mut ds);
                let truthy = match v {
                    crate::value::Value::Bool(x) => x,
                    crate::value::Value::Int(n) => n != 0,
                    crate::value::Value::Poison => continue,
                    other => {
                        diags.push(err(
                            *span,
                            format!(
                                "[contract.member-kind] an `implement` `if` condition must be a \
                                 comptime bool or int, got {}",
                                other.type_name()
                            ),
                        ));
                        continue;
                    }
                };
                let arm = if truthy { then_body } else { else_body };
                flatten_bindings(arm, impl_file, defines, ambient, diags, out);
            }
            other => out.push(other),
        }
    }
}

/// Collect interfaces / implements / all items from `items`, recursing one level
/// into `section {}` bodies (the resolve-pass recursion shape).
fn collect<'a>(
    items: &'a [ast::Item],
    id: &'a str,
    file: &'a ast::File,
    interfaces: &mut HashMap<String, (&'a ast::InterfaceDecl, &'a str)>,
    implements: &mut HashMap<String, Vec<(&'a ast::ImplementDecl, &'a ast::File, &'a str)>>,
    all_items: &mut Vec<&'a ast::Item>,
    diags: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            ast::Item::Interface(d) => {
                if let Some((_, prev)) = interfaces.insert(d.name.clone(), (d, id)) {
                    diags.push(err(
                        d.span,
                        format!(
                            "[contract.duplicate-impl] interface `{}` is declared twice (`{prev}` \
                             and `{id}`), an interface name must be unique",
                            d.name
                        ),
                    ));
                }
            }
            ast::Item::Implement(d) => {
                implements.entry(d.name.clone()).or_default().push((d, file, id));
            }
            ast::Item::Section(sec) => {
                collect(&sec.items, id, file, interfaces, implements, all_items, diags);
                continue;
            }
            _ => {}
        }
        all_items.push(item);
    }
}

/// Push a `[contract.missing-member]` error for an unbound required member.
fn missing(iface: &str, member: &ast::InterfaceMember, diags: &mut Vec<Diagnostic>) {
    diags.push(err(
        member.span,
        format!(
            "[contract.missing-member] required member `{iface}.{}` ({}) is not bound by the \
             `implement` block",
            member.name,
            member_kind_word(&member.kind)
        ),
    ));
}

/// The name, span, and a display word for a binding's KIND (for the unknown /
/// member-kind diagnostics).
fn binding_head(b: &ast::ImplBinding) -> (&str, Span, &'static str) {
    match b {
        ast::ImplBinding::Const { name, span, .. } => (name, *span, "a const"),
        ast::ImplBinding::Proc { name, span, .. } => (name, *span, "a proc"),
        ast::ImplBinding::Hook { name, span, .. } => (name, *span, "a hook"),
        ast::ImplBinding::Group { span, .. } => ("", *span, "a group"),
    }
}

/// Whether a binding kind word matches a declared member kind.
fn kind_matches(kind: &ast::InterfaceMemberKind, binding_word: &str) -> bool {
    matches!(
        (kind, binding_word),
        (ast::InterfaceMemberKind::Const(_), "a const")
            | (ast::InterfaceMemberKind::Proc(_), "a proc")
            | (ast::InterfaceMemberKind::Hook { .. }, "a hook")
    )
}

/// A display word for a declared member's kind.
fn member_kind_word(kind: &ast::InterfaceMemberKind) -> &'static str {
    match kind {
        ast::InterfaceMemberKind::Const(_) => "a const",
        ast::InterfaceMemberKind::Proc(_) => "a proc",
        ast::InterfaceMemberKind::Hook { .. } => "a hook",
    }
}

/// The link symbol a bound proc/hook path names: the dotted join (a bare name
/// stays bare; a dotted `a.b.c` becomes the cross-seam link name).
fn link_symbol(path: &ast::Path) -> String {
    path.segments.join(".")
}

/// A light const-value / declared-type consistency check. Full typed-const
/// enforcement is L5's job (and rides at the USE site, like `extern NAME: Type`);
/// this catches the gross kind mismatch — a non-comptime-int value (Code, Data,
/// a struct) bound to an integer/newtype const member.
fn check_const_type(
    iface: &str,
    member: &str,
    ty: &ast::Type,
    value: &crate::value::Value,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    use crate::value::Value;
    // Only int-ish members get the sanity check; a bool/enum/typed value that
    // stores as an int passes. A Code/Data/Unit/FnRef value never fills a const.
    let ok = matches!(
        value,
        Value::Int(_)
            | Value::Bool(_)
            | Value::Enum { .. }
            | Value::Label(_)
            | Value::Poison
            | Value::Typed { .. }
    );
    if !ok {
        diags.push(err(
            span,
            format!(
                "[contract.member-kind] const `{iface}.{member}` (declared `{}`) is bound to a \
                 non-value ({})",
                type_name(ty),
                value.type_name()
            ),
        ));
    }
}

/// Check a bound proc member against its declared proc contract type (§4). Skips
/// silently when the contract type or the bound proc's own contract is not in
/// scope (an unbounded/undeclared boundary — like the closure's unknown-callee
/// leaf; a real signature error still fires when both sides are declared).
fn check_proc_type(
    iface: &str,
    member: &str,
    ptype: &ast::Type,
    bound_sym: &str,
    all_items: &[&ast::Item],
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    let ast::Type::Named(p) = ptype else { return };
    let tyname = p.segments.last().map(String::as_str).unwrap_or("");
    let Some(decl) = find_contract_type(all_items, tyname) else { return };
    let Some(bound) = find_proc_contract(all_items, bound_sym) else { return };
    let declared = contract_of_sig(&decl.sig, diags, span);
    report_subcontract(iface, member, bound_sym, "proc", &bound, &declared, span, diags);
}

/// Check a bound hook member against its declared hook signature (§4).
fn check_hook_sig(
    iface: &str,
    member: &str,
    sig: &ast::ProcSig,
    bound_sym: &str,
    all_items: &[&ast::Item],
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(bound) = find_proc_contract(all_items, bound_sym) else { return };
    let declared = contract_of_sig(sig, diags, span);
    report_subcontract(iface, member, bound_sym, "hook", &bound, &declared, span, diags);
}

/// Run the §4 subcontract relation (`bound ⊑ declared`) and, on any violation,
/// emit `[contract.hook-signature]` naming both sites.
#[allow(clippy::too_many_arguments)]
fn report_subcontract(
    iface: &str,
    member: &str,
    bound_sym: &str,
    what: &str,
    bound: &Contract,
    declared: &Contract,
    span: Span,
    diags: &mut Vec<Diagnostic>,
) {
    let violations = subcontract_violations(bound, declared);
    if !violations.is_empty() {
        diags.push(err(
            span,
            format!(
                "[contract.hook-signature] {what} `{iface}.{member}` bound to `{bound_sym}` does \
                 not satisfy the declared signature: {}",
                violations.join("; ")
            ),
        ));
    }
}

/// Build a register-partition [`Contract`] from a proc signature's reglists (68k).
///
/// The `out` reglist SPLITS: the parser folds an `out(rN if cc)` register into
/// both `out_cond` and the plain reglist (out-verify needs it there), so the
/// conditional registers ride `Contract::out_cond` and `Contract::out` carries
/// only the unconditional remainder. Collapsing the two lets a target producing
/// `rN` only on its cc edge satisfy a bound promising `out(rN)` on every return —
/// and such a target may simultaneously and legally declare `clobbers(rN)`, i.e.
/// state outright that it destroys the register the bound promises callers.
///
/// The conditional half carries its CONDITION CODES
/// ([`ast::ProcSig::cond_out_guards`]), not just the register names: the relation
/// compares a bound's demanded edge against the target's, and a register-only view
/// lets `out(a1 if ne)` satisfy `out(a1 if eq)`.
///
/// Both halves of that split come from [`ast::ProcSig::unconditional_outs`] /
/// [`ast::ProcSig::cond_out_guards`], which expand through ONE register file. The
/// `expand_reglist` call below runs for its DIAGNOSTICS only: a minuend and a
/// subtrahend produced by two different expanders is the raw-vs-canonical bug
/// class in miniature — these two disagree on `sp`, which the seam drops as
/// `[contract.unknown-register]` while the production expander canonicalizes it
/// to `a7`.
fn contract_of_sig(sig: &ast::ProcSig, diags: &mut Vec<Diagnostic>, span: Span) -> Contract {
    let mut on_err = |m: String| diags.push(err(span, m));
    let clobbers = expand_reglist(sig.clobbers.as_deref().unwrap_or(&[]), RegFile::M68k, &mut on_err);
    let preserves = expand_reglist(&sig.preserves, RegFile::M68k, &mut on_err);
    // Validity reporting only — the SET comes from the accessor below.
    expand_reglist(sig.out.as_deref().unwrap_or(&[]), RegFile::M68k, &mut on_err);
    let out = sig.unconditional_outs(RegFile::M68k);
    let out_cond = sig.cond_out_guards(RegFile::M68k);
    let params: std::collections::BTreeSet<String> = sig
        .params
        .iter()
        .filter_map(|(name, _, _)| crate::value::Reg::from_name(name).map(|_| name.clone()))
        .collect();
    Contract { clobbers, preserves, params, out, out_cond }
}

/// Build a [`Contract`] from a body `proc` or an `extern proc` found by link name
/// in the module set. `None` when the name is not a declared proc/extern (an
/// unbounded boundary the signature check skips).
fn find_proc_contract(all_items: &[&ast::Item], sym: &str) -> Option<Contract> {
    for item in all_items {
        match item {
            ast::Item::Proc(p) if p.name == sym => {
                let sig = ast::ProcSig {
                    params: p.params.iter().map(|(n, t, s)| (n.clone(), Some(t.clone()), *s)).collect(),
                    clobbers: p.clobbers.clone(),
                    preserves: p.preserves.clone(),
                    out: p.out.clone(),
                    out_flags: p.out_flags.clone(),
                    out_cond: p.out_cond.clone(),
                    out_types: p.out_types.clone(),
                    inout: p.inout.clone(),
                    inout_types: p.inout_types.clone(),
                    requires: p.requires.clone(),
                };
                // A proc that declares no clobber contract is an unbounded leaf
                // (opt-in contracts, like the closure): skip the subset check.
                p.clobbers.as_ref()?;
                let mut sink = Vec::new();
                return Some(contract_of_sig(&sig, &mut sink, p.span));
            }
            ast::Item::ExternProc(e) if e.name == sym => {
                e.sig.clobbers.as_ref()?;
                let mut sink = Vec::new();
                return Some(contract_of_sig(&e.sig, &mut sink, e.span));
            }
            _ => {}
        }
    }
    None
}

/// Find a `type Name = proc (...)` contract-type declaration by name.
fn find_contract_type<'a>(all_items: &[&'a ast::Item], name: &str) -> Option<&'a ast::ContractTypeDecl> {
    all_items.iter().find_map(|item| match item {
        ast::Item::ContractType(t) if t.name == name => Some(t),
        _ => None,
    })
}

/// A short display for a type (for messages).
fn type_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(p) => p.segments.join("."),
        ast::Type::Ptr(inner) => format!("*{}", type_name(inner)),
        _ => "<type>".to_string(),
    }
}

fn err(span: Span, message: String) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}
