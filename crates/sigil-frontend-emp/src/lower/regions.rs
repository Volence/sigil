//! Region-form `vars` allocation (item #7a): resolve `region` items + their
//! region-form `vars` blocks into concrete RAM addresses, and emit each region
//! as a reserve-only Core section (VMA-placed, zero image bytes) whose labels /
//! marks / aliases resolve to their addresses.
//!
//! The layout is a PURE function of (region decls, owner-module source order,
//! comptime defines) — no link-order input (§3.1). Chained regions
//! (`after(<region>)`) form a DAG resolved in topological order; a cycle is
//! `[region.chain-cycle]`. RAM emits no image bytes, so `@align`/`pad` here are
//! pure RESERVE advances of the location counter, and the layout checks
//! (`[region.overflow]`, `w_addressable`, `[layout.odd-field]`) run against the
//! real region-absolute VMA.
//!
//! v1 scope (recorded in the item #7a note): resolution is per lowered file.
//! Intra-file `after(<region>)` chaining is fully realized; cross-module
//! chaining and the cross-module `[region.multiple-owners]` check are the
//! whole-program hooks (see [`check_single_owner`]) wired for #7b/#7c.

use crate::ast;
use crate::eval::{run_on_eval_stack, Env, Evaluator};
use sigil_ir::backend::{Cpu, IrStreamer};
use sigil_ir::{EquSym, IrBuilder};
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;

/// One ordered emission step within a region's reserve-only section.
enum EmitOp {
    /// Advance the section cursor by `n` bytes with NO image bytes (`ds`).
    Reserve(u32),
    /// Define a link-visible label at the current cursor.
    Label(String),
}

/// A region resolved to concrete addresses, ready to emit. All fields are plain
/// data (`Send`) so the whole resolution runs on the large comptime-eval stack.
struct ResolvedRegion {
    /// The reserve-only section name (the region name).
    name: String,
    /// The region base VMA (labels resolve at `base + offset`).
    base: u32,
    /// Ordered reserve/label steps reconstructing the region's layout.
    ops: Vec<EmitOp>,
    /// `name: alias(Other)` equates — `(alias_name, target_absolute_address)`.
    aliases: Vec<(String, u32)>,
}

/// Resolve and EMIT every region declared in `file` into `builder` as
/// reserve-only sections, appending diagnostics. A no-op (touches neither the
/// builder nor `diags`) for a file with no `region` items and no region-form
/// `vars` blocks — so every module that uses no RAM regions is byte-identical.
pub(super) fn lower_regions(
    file: &ast::File,
    defines: &[(String, i128)],
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // Quick out: nothing to do unless the file declares a region or a region-
    // form `vars` block. Keeps every region-free module's lowering untouched.
    let has_region = file.items.iter().any(|it| {
        matches!(it, ast::Item::Region(_))
            || matches!(it, ast::Item::Vars(v) if v.name.is_none())
    });
    if !has_region {
        return;
    }

    let (resolved, mut rdiags) = resolve_regions(file, defines);
    diags.append(&mut rdiags);

    // Emit each resolved region as a reserve-only section. RAM sections emit no
    // image bytes (only `Reserve` fragments), so `sigil-link::flatten` skips
    // them; their labels resolve at the region's VMA base.
    for r in resolved {
        if r.ops.is_empty() && r.aliases.is_empty() {
            continue; // an empty region (no fields, no marks) emits nothing.
        }
        builder.switch_section_lma(&r.name, Cpu::M68000, Some(r.base), r.base);
        // Aliases are pure equates (`Name = Other`), attached to this section's
        // carrier; their value is the comptime-known target address.
        for (name, addr) in r.aliases {
            builder.add_equ_sym(EquSym {
                name,
                expr: sigil_ir::expr::Expr::Int(addr as i64),
                span: file.module.span,
            });
        }
        for op in r.ops {
            match op {
                EmitOp::Reserve(n) => {
                    if n > 0 {
                        builder.reserve(n, file.module.span);
                    }
                }
                EmitOp::Label(name) => builder.define_label(&name),
            }
        }
    }
}

/// Resolve every region in `file` to concrete addresses (no emission). Runs on
/// the large comptime-eval stack (deep struct layouts). Pure computation —
/// returns plain data + diagnostics.
fn resolve_regions(
    file: &ast::File,
    defines: &[(String, i128)],
) -> (Vec<ResolvedRegion>, Vec<Diagnostic>) {
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.seed_defines(defines);
        let mut diags = Vec::new();

        // 1. Index region decls by name; `[region.duplicate]` on a repeat.
        let mut region_by_name: HashMap<&str, &ast::RegionDecl> = HashMap::new();
        for it in &file.items {
            if let ast::Item::Region(r) = it {
                if let Some(prev) = region_by_name.insert(r.name.as_str(), r) {
                    diags.push(Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "[region.duplicate] region `{}` is declared more than once \
                             (a region must be declared exactly once per link)",
                            r.name
                        ),
                        primary: r.span,
                    });
                    // Keep the FIRST decl authoritative for layout.
                    region_by_name.insert(r.name.as_str(), prev);
                }
            }
        }

        // 2. Group region-form `vars` blocks by region name, in source order.
        let mut blocks_by_region: HashMap<&str, Vec<&ast::VarsDecl>> = HashMap::new();
        let mut vars_order: Vec<&str> = Vec::new();
        for it in &file.items {
            if let ast::Item::Vars(v) = it {
                if v.name.is_none() {
                    let region = v.region.first().map(String::as_str).unwrap_or("");
                    if !region_by_name.contains_key(region) {
                        diags.push(Diagnostic {
                            level: Level::Error,
                            message: format!(
                                "[region.unknown] `vars {region}` names region `{region}`, which is \
                                 not declared (add a `region {region} @ base .. limit`)"
                            ),
                            primary: v.span,
                        });
                        continue;
                    }
                    blocks_by_region.entry(region).or_default().push(v);
                    if !vars_order.contains(&region) {
                        vars_order.push(region);
                    }
                }
            }
        }

        // 3. Resolve region bases/sizes in `after`-DAG topological order, laying
        //    out each region once (memoized). Regions are visited in decl order
        //    for determinism; `after` forces a parent to resolve first.
        let mut memo: HashMap<String, (u32, u32)> = HashMap::new(); // name -> (base, size)
        let mut resolved: Vec<ResolvedRegion> = Vec::new();
        let mut region_names: Vec<&str> = file
            .items
            .iter()
            .filter_map(|it| match it {
                ast::Item::Region(r) => Some(r.name.as_str()),
                _ => None,
            })
            .collect();
        region_names.dedup();

        for name in region_names {
            let mut visiting: Vec<String> = Vec::new();
            resolve_one(
                name,
                &region_by_name,
                &blocks_by_region,
                &mut ev,
                &mut memo,
                &mut resolved,
                &mut visiting,
                &mut diags,
            );
        }

        diags.append(&mut ev.diags);
        (resolved, diags)
    })
}

/// Resolve region `name` (base + size + emission ops), memoized. Follows
/// `after(<parent>)` recursively with a `visiting` stack for cycle detection.
#[allow(clippy::too_many_arguments)]
fn resolve_one(
    name: &str,
    region_by_name: &HashMap<&str, &ast::RegionDecl>,
    blocks_by_region: &HashMap<&str, Vec<&ast::VarsDecl>>,
    ev: &mut Evaluator,
    memo: &mut HashMap<String, (u32, u32)>,
    resolved: &mut Vec<ResolvedRegion>,
    visiting: &mut Vec<String>,
    diags: &mut Vec<Diagnostic>,
) -> (u32, u32) {
    if let Some(&bs) = memo.get(name) {
        return bs;
    }
    let Some(decl) = region_by_name.get(name).copied() else {
        return (0, 0); // unknown parent — already diagnosed at the vars site.
    };
    if visiting.iter().any(|n| n == name) {
        diags.push(Diagnostic {
            level: Level::Error,
            message: format!(
                "[region.chain-cycle] region `{name}`'s `after(..)` chain is cyclic: {} -> {name}",
                visiting.join(" -> ")
            ),
            primary: decl.span,
        });
        memo.insert(name.to_string(), (0, 0));
        return (0, 0);
    }
    visiting.push(name.to_string());

    // Resolve the base (explicit address or the parent region's running end).
    let base = match &decl.base {
        ast::RegionBase::Addr(expr) => eval_u32(ev, expr, diags),
        ast::RegionBase::After { region, span } => {
            if !region_by_name.contains_key(region.as_str()) {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!(
                        "[region.unknown] `after({region})` names region `{region}`, which is not declared"
                    ),
                    primary: *span,
                });
                0
            } else {
                let (pbase, psize) = resolve_one(
                    region, region_by_name, blocks_by_region, ev, memo, resolved, visiting, diags,
                );
                pbase.wrapping_add(psize)
            }
        }
    };

    // Evaluate the limit up front so the walk can name the field that crosses it.
    let limit = eval_u32(ev, &decl.limit, diags);

    // Lay out this region's vars blocks (source order) from `base`.
    let empty = Vec::new();
    let blocks = blocks_by_region.get(name).unwrap_or(&empty);
    let mut lay = Layout::new(base);
    lay.limit_seen = Some(limit);
    for block in blocks {
        lay.walk(&block.region_body, ev, diags);
    }
    let size = lay.cursor.wrapping_sub(base);

    // `[region.overflow]` — the running end crossed the region limit.
    if lay.cursor > limit {
        let over = lay.cursor - limit;
        let crossing = lay
            .first_overflow_field
            .clone()
            .unwrap_or_else(|| "<unnamed>".to_string());
        diags.push(Diagnostic {
            level: Level::Error,
            message: format!(
                "[region.overflow] region `{name}` overflows its limit by {over} bytes \
                 (field `{crossing}` crosses ${limit:08X})"
            ),
            primary: decl.span,
        });
    }

    // `w_addressable` — every byte in [base, limit) reachable by sign-extended
    // `.w` addressing (bit 15 of the low word set across the whole window).
    if decl.w_addressable {
        let last = limit.wrapping_sub(1);
        let ok = (base & 0xFFFF) >= 0x8000
            && (last & 0xFFFF) >= 0x8000
            && (base >> 16) == (last >> 16);
        if !ok {
            diags.push(Diagnostic {
                level: Level::Error,
                message: format!(
                    "[region.not-w-addressable] region `{name}` `[${base:08X}, ${limit:08X})` is not \
                     fully `.w`-addressable — some byte's low word has bit 15 clear (would resolve to ROM)"
                ),
                primary: decl.span,
            });
        }
    }

    // Resolve aliases against the completed address map (forward refs allowed).
    let mut aliases = Vec::new();
    for (alias_name, target, span) in &lay.alias_reqs {
        match lay.addr_map.get(target) {
            Some(&addr) => {
                aliases.push((alias_name.clone(), addr));
            }
            None => diags.push(Diagnostic {
                level: Level::Error,
                message: format!(
                    "[region.unknown] alias `{alias_name}` targets `{target}`, which is not a field \
                     or mark in region `{name}`"
                ),
                primary: *span,
            }),
        }
    }

    memo.insert(name.to_string(), (base, size));
    resolved.push(ResolvedRegion {
        name: name.to_string(),
        base,
        ops: lay.ops,
        aliases,
    });
    visiting.pop();
    (base, size)
}

/// The running layout state for one region: a byte cursor, the ordered emission
/// ops, the field/mark address map, and pending alias requests.
struct Layout {
    cursor: u32,
    ops: Vec<EmitOp>,
    addr_map: HashMap<String, u32>,
    alias_reqs: Vec<(String, String, Span)>,
    first_overflow_field: Option<String>,
    limit_seen: Option<u32>,
}

impl Layout {
    fn new(base: u32) -> Self {
        Layout {
            cursor: base,
            ops: Vec::new(),
            addr_map: HashMap::new(),
            alias_reqs: Vec::new(),
            first_overflow_field: None,
            limit_seen: None,
        }
    }

    /// Reserve `n` bytes (advance the cursor + record the op), coalescing with a
    /// trailing `Reserve` so the emission stays compact.
    fn reserve(&mut self, n: u32) {
        if n == 0 {
            return;
        }
        self.cursor = self.cursor.wrapping_add(n);
        if let Some(EmitOp::Reserve(prev)) = self.ops.last_mut() {
            *prev = prev.wrapping_add(n);
        } else {
            self.ops.push(EmitOp::Reserve(n));
        }
    }

    /// Advance the cursor to the next multiple of `align` (reserve semantics).
    /// Aligns the region-ABSOLUTE address, matching AS's `align`.
    fn align_to(&mut self, align: u32) {
        if align <= 1 {
            return;
        }
        let rem = self.cursor % align;
        if rem != 0 {
            self.reserve(align - rem);
        }
    }

    fn label(&mut self, name: &str) {
        self.ops.push(EmitOp::Label(name.to_string()));
        self.addr_map.insert(name.to_string(), self.cursor);
    }

    /// Walk a region-body field list, placing each item in declaration order.
    fn walk(&mut self, body: &[ast::RegionField], ev: &mut Evaluator, diags: &mut Vec<Diagnostic>) {
        for field in body {
            self.place(field, ev, diags);
        }
    }

    fn place(&mut self, field: &ast::RegionField, ev: &mut Evaluator, diags: &mut Vec<Diagnostic>) {
        match field {
            ast::RegionField::Typed(f) => {
                // `@align(N)` first (advance to the boundary), then place.
                if let Some(a) = &f.align {
                    let n = eval_u32(ev, a, diags);
                    self.align_to(n);
                }
                let ty = ev.resolve_type(&f.ty);
                let size = ev.size_of_ty(&ty, f.span) as u32;
                // `[layout.odd-field]` — a word-or-wider field at an odd address
                // is AS's silent address-error trap.
                if (self.cursor & 1) == 1 && ev.ty_needs_even(&ty, f.span) {
                    diags.push(Diagnostic {
                        level: Level::Warning,
                        message: format!(
                            "[layout.odd-field] field `{}` needs an even address but lands at \
                             ${:08X} (odd) — add an explicit `pad(1)` or `@align(2)` before it",
                            f.name, self.cursor
                        ),
                        primary: f.span,
                    });
                }
                self.label(&f.name);
                self.note_overflow(&f.name, self.cursor.wrapping_add(size));
                self.reserve(size);
            }
            ast::RegionField::Pad { count, span } => {
                let n = eval_u32(ev, count, diags);
                self.note_overflow("<pad>", self.cursor.wrapping_add(n));
                self.reserve(n);
                let _ = span;
            }
            ast::RegionField::Mark { name, .. } => {
                self.label(name);
            }
            ast::RegionField::Alias { name, target, span } => {
                // Resolved after the region's address map is complete.
                self.alias_reqs.push((name.clone(), target.clone(), *span));
            }
            ast::RegionField::Group { cond, shape_divergent, then_body, else_body, span } => {
                // Prove the two arms' sizes: a size-varying group must be
                // annotated `@shape_divergent`, else `[vars.shape-divergent]`.
                let then_sz = measure(then_body, ev);
                let else_sz = measure(else_body, ev);
                if then_sz != else_sz && !shape_divergent {
                    diags.push(Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "[vars.shape-divergent] this conditional field group's arms differ in size \
                             ({then_sz} vs {else_sz} bytes), so every field after it moves between \
                             build shapes — declare the divergence with `@shape_divergent`"
                        ),
                        primary: *span,
                    });
                }
                // Select the arm by the comptime condition (nonzero = `then`).
                let take_then = eval_i128(ev, cond, diags) != 0;
                if take_then {
                    self.walk(then_body, ev, diags);
                } else {
                    self.walk(else_body, ev, diags);
                }
            }
        }
    }

    /// Record the first field whose END crosses the region limit (for the
    /// `[region.overflow]` message). The limit itself is checked once, after the
    /// full walk — this only names the crossing field.
    fn note_overflow(&mut self, name: &str, end: u32) {
        if let Some(limit) = self.limit_seen {
            if end > limit && self.first_overflow_field.is_none() {
                self.first_overflow_field = Some(name.to_string());
            }
        }
    }
}

/// The reserve-only byte size of a field list under the current comptime env,
/// WITHOUT placing anything (for the `@shape_divergent` size-equality proof and
/// nested-group sizing). `@align` inside an arm is measured relative to a fresh
/// zero cursor — the two arms are compared for RELATIVE size equivalence.
fn measure(body: &[ast::RegionField], ev: &mut Evaluator) -> u32 {
    let mut sink = Vec::new();
    let mut lay = Layout::new(0);
    lay.walk(body, ev, &mut sink);
    lay.cursor
}

/// Evaluate a comptime expression to an `i128` (defines already seeded on `ev`).
fn eval_i128(ev: &mut Evaluator, expr: &ast::Expr, diags: &mut Vec<Diagnostic>) -> i128 {
    let mut env = Env::new();
    let v = ev.eval_expr(expr, &mut env);
    match v.as_stored_int() {
        Some(n) => n,
        None => {
            diags.push(Diagnostic {
                level: Level::Error,
                message: "region/field expression is not a comptime integer".to_string(),
                primary: crate::parser::expr_span(expr),
            });
            0
        }
    }
}

/// Evaluate a comptime expression to a 32-bit address/count.
fn eval_u32(ev: &mut Evaluator, expr: &ast::Expr, diags: &mut Vec<Diagnostic>) -> u32 {
    (eval_i128(ev, expr, diags) as u64 & 0xFFFF_FFFF) as u32
}

/// Whole-program check (§2.3): a region's `vars` blocks must all live in ONE
/// owner module — `[region.multiple-owners]` otherwise. Pure diagnostic pass
/// over `(module_id, file)` pairs; returns no diagnostics for a program with no
/// region-form `vars` blocks, so a region-free build is untouched.
pub fn check_single_owner(modules: &[(&str, &ast::File)]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // region name -> (first owner module id, the vars-decl span in that module)
    let mut owner: HashMap<String, String> = HashMap::new();
    for (mid, file) in modules {
        for it in &file.items {
            if let ast::Item::Vars(v) = it {
                if v.name.is_none() {
                    let region = v.region.first().cloned().unwrap_or_default();
                    match owner.get(&region) {
                        Some(prev) if prev != mid => {
                            diags.push(Diagnostic {
                                level: Level::Error,
                                message: format!(
                                    "[region.multiple-owners] region `{region}` has `vars` blocks in \
                                     both module `{prev}` and module `{mid}` — all `vars` for a region \
                                     must live in one module"
                                ),
                                primary: v.span,
                            });
                        }
                        Some(_) => {} // same module, allowed (source-order blocks).
                        None => {
                            owner.insert(region, mid.to_string());
                        }
                    }
                }
            }
        }
    }
    diags
}
