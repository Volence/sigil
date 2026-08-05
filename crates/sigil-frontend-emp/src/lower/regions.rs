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
//! Cross-module `after(<region>)` chaining (item #7c): a region may chain onto a
//! region declared in ANOTHER module (the game's `game_ram @ after(upper_ram)`
//! onto the engine's `upper_ram`). The authoritative cross-module ends are
//! resolved WHOLE-PROGRAM once by [`resolve_program_region_ends`] (a fixpoint over
//! the `after`-DAG spanning modules, so a parent always resolves before its
//! dependents regardless of module lowering order — principled, not incidental),
//! and threaded back into each module's per-file [`lower_regions`] as
//! `external_ends`. A local region resolves intra-file exactly as before; only an
//! `after(<region>)` whose parent is NOT local consults `external_ends`. The
//! cross-module `[region.multiple-owners]` check stays in [`check_single_owner`].

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
    /// The region's exclusive limit VMA (the `.. limit` budget ceiling) — for the
    /// RAM map report (T1); does not affect emission.
    limit: u32,
    /// Bytes this region spends on alignment/`pad(N)` padding rather than fields —
    /// for the RAM map report (T1); does not affect emission.
    padding: u32,
}

/// Resolve and EMIT every region declared in `file` into `builder` as
/// reserve-only sections, appending diagnostics. A no-op (touches neither the
/// builder nor `diags`) for a file with no `region` items and no region-form
/// `vars` blocks — so every module that uses no RAM regions is byte-identical.
pub(super) fn lower_regions(
    file: &ast::File,
    defines: &[(String, i128)],
    external_ends: &HashMap<String, u32>,
    builder: &mut IrBuilder,
    diags: &mut Vec<Diagnostic>,
) {
    // Quick out: nothing to do unless the file declares a region or a region-
    // form `vars` block. Keeps every region-free module's lowering untouched.
    if !file_declares_region(file) {
        return;
    }

    let (resolved, mut rdiags, _memo) = resolve_regions(file, defines, external_ends);
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

/// [`resolve_regions`]'s result: `(resolved regions, diagnostics,
/// name → (base, end) span map)`.
type ResolvedRegions = (Vec<ResolvedRegion>, Vec<Diagnostic>, HashMap<String, (u32, u32)>);

/// Resolve every region in `file` to concrete addresses (no emission). Runs on
/// the large comptime-eval stack (deep struct layouts). Pure computation —
/// returns plain data + diagnostics.
fn resolve_regions(
    file: &ast::File,
    defines: &[(String, i128)],
    external_ends: &HashMap<String, u32>,
) -> ResolvedRegions {
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

        // 2. Group region-form `vars` blocks by region name, in source order
        //    (multiple blocks per region allocate in that order, §2.3).
        let mut blocks_by_region: HashMap<&str, Vec<&ast::VarsDecl>> = HashMap::new();
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
                external_ends,
                &mut ev,
                &mut memo,
                &mut resolved,
                &mut visiting,
                &mut diags,
            );
        }

        diags.append(&mut ev.diags);
        (resolved, diags, memo)
    })
}

/// Does `file` declare a `region` item or a region-form `vars` block? Used to
/// skip region resolution entirely for the (overwhelming) majority of modules
/// that use no RAM regions — keeping their lowering byte-identical.
pub fn file_declares_region(file: &ast::File) -> bool {
    file.items.iter().any(|it| {
        matches!(it, ast::Item::Region(_))
            || matches!(it, ast::Item::Vars(v) if v.name.is_none())
    })
}

/// One RAM region's resolved geometry for the map report (T1). A plain data row —
/// name, base/end address, used size, padding bytes, and the budget limit — so the
/// CLI (and, later, Spec-3 editor inlay hints — the data shape is deliberately kept
/// free of any render/format concern) can present "what is each region's real number".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RamRegionRow {
    /// The region name (`lower_ram`, `upper_ram`, `game_ram`, …).
    pub name: String,
    /// `pub region` (exported) vs a module-private region.
    pub public: bool,
    /// The region base VMA (`@ base ..`).
    pub base: u32,
    /// The exclusive limit VMA (`.. limit`) — the budget ceiling.
    pub limit: u32,
    /// Bytes actually allocated (`end - base`).
    pub size: u32,
    /// Bytes within `size` spent on `@align`/`pad(N)` padding (not fields).
    pub padding: u32,
}

impl RamRegionRow {
    /// The running end address (`base + size`) — the next free VMA / where a chained
    /// region continues.
    pub fn end(&self) -> u32 {
        self.base.wrapping_add(self.size)
    }
    /// The region's total capacity (`limit - base`); `0` if the limit precedes the
    /// base (a malformed region — reported elsewhere).
    pub fn capacity(&self) -> u32 {
        self.limit.saturating_sub(self.base)
    }
    /// Bytes free between the running end and the limit (the budget headroom); `0` if
    /// the region overflows its limit ([`region.overflow`] catches that as an error).
    pub fn headroom(&self) -> u32 {
        self.limit.saturating_sub(self.end())
    }
}

/// Resolve every region in `file` into report rows (T1) — the RAM map's per-region
/// geometry (name, base, limit, used size, padding). Reuses the exact resolver the
/// byte-emitting [`lower_regions`] uses, so the numbers are the SHIPPING layout, not a
/// re-derivation. `external_ends` threads cross-module `after(..)` parents (from
/// [`resolve_program_region_ends`]) exactly as the per-file lowering does. Rows come
/// back in region declaration order; diagnostics are the resolver's (overflow,
/// odd-field, unknown parent) — the caller renders them.
pub fn collect_region_report(
    file: &ast::File,
    defines: &[(String, i128)],
    external_ends: &HashMap<String, u32>,
) -> (Vec<RamRegionRow>, Vec<Diagnostic>) {
    if !file_declares_region(file) {
        return (Vec::new(), Vec::new());
    }
    // Index each region's `public` flag (the ResolvedRegion doesn't carry it).
    let mut public_by_name: HashMap<&str, bool> = HashMap::new();
    for it in &file.items {
        if let ast::Item::Region(r) = it {
            public_by_name.entry(r.name.as_str()).or_insert(r.public);
        }
    }
    let (resolved, diags, _memo) = resolve_regions(file, defines, external_ends);
    let rows = resolved
        .into_iter()
        .map(|r| {
            let size = r.ops.iter().fold(0u32, |acc, op| match op {
                EmitOp::Reserve(n) => acc.wrapping_add(*n),
                EmitOp::Label(_) => acc,
            });
            RamRegionRow {
                public: public_by_name.get(r.name.as_str()).copied().unwrap_or(false),
                name: r.name,
                base: r.base,
                limit: r.limit,
                size,
                padding: r.padding,
            }
        })
        .collect();
    (rows, diags)
}

/// Whole-program region-END resolution (item #7c): resolve EVERY region's running
/// end address across all region-owning `modules` (each an ambient-prepended
/// synthetic file, so its `use`d comptime sizes resolve), following `after(..)`
/// chains that may cross module boundaries. Returns `region name -> end address`.
///
/// A fixpoint over the cross-module `after`-DAG: each pass re-resolves every
/// region module against the ends known so far, until no end changes. A parent
/// therefore resolves before its dependents regardless of the `modules` order
/// (the ordering is principled, not incidental). An acyclic DAG of N region
/// modules converges within N passes; if the (N+1)-th pass still moves an end, an
/// `after(..)` cycle spans modules — reported as `[region.chain-cycle]` (the
/// whole-program analog of the intra-file cycle [`resolve_one`] already catches).
///
/// Diagnostics OTHER than the cross-module cycle (overflow, unknown parent,
/// odd-field, …) are the per-file [`lower_regions`] pass's job — it runs
/// afterward with this map, so they are reported exactly once there. Returns an
/// empty map (no passes) for a program with no region modules.
pub fn resolve_program_region_ends(
    modules: &[(&str, ast::File)],
    defines: &[(String, i128)],
) -> (HashMap<String, u32>, Vec<Diagnostic>) {
    let region_modules: Vec<&(&str, ast::File)> =
        modules.iter().filter(|(_, f)| file_declares_region(f)).collect();
    if region_modules.is_empty() {
        return (HashMap::new(), Vec::new());
    }

    let mut ends: HashMap<String, u32> = HashMap::new();
    // N region modules → an acyclic chain settles within N passes; one extra pass
    // both confirms the fixpoint and detects a cross-module cycle (still moving).
    let cap = region_modules.len() + 1;
    for pass in 0..=cap {
        let mut progress = false;
        for (_id, file) in &region_modules {
            // Per-module resolution against the ends known so far; per-region
            // diagnostics are discarded here (the per-file pass owns them).
            let (_resolved, _diags, memo) = resolve_regions(file, defines, &ends);
            for (name, (base, size)) in memo {
                let end = base.wrapping_add(size);
                if ends.get(&name) != Some(&end) {
                    ends.insert(name, end);
                    progress = true;
                }
            }
        }
        if !progress {
            return (ends, Vec::new());
        }
        if pass == cap {
            // Still moving after N+1 passes over N modules ⇒ a cross-module
            // `after(..)` cycle. Name the regions still unsettled.
            let mut names: Vec<&str> = ends.keys().map(String::as_str).collect();
            names.sort_unstable();
            let diag = Diagnostic {
                level: Level::Error,
                message: format!(
                    "[region.chain-cycle] a cross-module `after(..)` chain does not converge \
                     (regions: {}) — an `after(..)` cycle spans modules",
                    names.join(", ")
                ),
                primary: region_modules[0].1.module.span,
            };
            return (ends, vec![diag]);
        }
    }
    (ends, Vec::new())
}

/// Resolve region `name` (base + size + emission ops), memoized. Follows
/// `after(<parent>)` recursively with a `visiting` stack for cycle detection.
#[allow(clippy::too_many_arguments)]
fn resolve_one(
    name: &str,
    region_by_name: &HashMap<&str, &ast::RegionDecl>,
    blocks_by_region: &HashMap<&str, Vec<&ast::VarsDecl>>,
    external_ends: &HashMap<String, u32>,
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
            if region_by_name.contains_key(region.as_str()) {
                // Local parent — resolve it intra-file (recurses into the DAG).
                let (pbase, psize) = resolve_one(
                    region, region_by_name, blocks_by_region, external_ends, ev, memo, resolved,
                    visiting, diags,
                );
                pbase.wrapping_add(psize)
            } else if let Some(&pend) = external_ends.get(region.as_str()) {
                // Cross-module parent (item #7c) — its running end was resolved
                // whole-program by `resolve_program_region_ends` and threaded in.
                pend
            } else {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!(
                        "[region.unknown] `after({region})` names region `{region}`, which is not \
                         declared (in this module or any module it chains onto)"
                    ),
                    primary: *span,
                });
                0
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
        limit,
        padding: lay.padding,
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
    /// Bytes reserved for alignment (`@align`) or explicit `pad(N)` — the RAM map
    /// report's "padding" column (T1). Pure accounting; never read by emission.
    padding: u32,
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
            padding: 0,
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

    /// Advance the cursor for a field `@align(n)` (reserve semantics), matching
    /// AS's `align` INSIDE A PHASE (`sigil-frontend-as` `directive_align`). A `vars`
    /// region is the `.emp` analog of an AS `phase`d RAM section (VMA `$FFFF….`,
    /// `disp != 0`), and asl's in-phase align is NOT a plain round-up: it advances
    /// by `round_up(cursor + n, n)` — ALWAYS at least one full `n` beyond the
    /// cursor, even when the cursor is already `n`-aligned (asl 1.42, live-probed).
    /// This is what places `Player_Pos_Ring` at `$FFFFB500` (not `$FFFFB400`) when
    /// game RAM chains from the non-256-aligned `Engine_RAM_End` — the byte-identity
    /// requirement. (Spec §2.2's "next multiple of N" wording is refined here to the
    /// corpus reality; regions are RAM-only, so the phased regime always applies.)
    fn align_to(&mut self, align: u32) {
        if align <= 1 {
            return;
        }
        // Mirror `directive_align` exactly (`round_up(pos + n, n)`), valid for any
        // `n` (not only powers of two). Aeon RAM sits far below the `u32` ceiling,
        // so `cursor + align` never overflows (the same domain the AS side folds).
        let target = (self.cursor + align).next_multiple_of(align);
        let pad = target - self.cursor;
        self.padding = self.padding.wrapping_add(pad); // T1 report accounting only.
        self.reserve(pad);
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
                self.padding = self.padding.wrapping_add(n); // T1 report accounting only.
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
                // Select the arm by the comptime condition. The ratified corpus
                // spelling (spec §8.1) is `if DEBUG == 1` — a comparison, which
                // yields `Value::Bool`; a bare integer flag (`if FLAG`) is also
                // accepted (nonzero = the `then` arm).
                let take_then = eval_group_cond(ev, cond, diags);
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

/// Evaluate a region-form conditional-group condition (`if <cond> { .. }`). Per
/// spec §8.1 the ratified corpus spelling is `if DEBUG == 1` — a comparison,
/// yielding `Value::Bool`; a bare integer flag (`if FLAG`) is also accepted
/// (nonzero = the `then` arm). Anything else is not a comptime condition.
fn eval_group_cond(ev: &mut Evaluator, expr: &ast::Expr, diags: &mut Vec<Diagnostic>) -> bool {
    let mut env = Env::new();
    let v = ev.eval_expr(expr, &mut env);
    if let crate::value::Value::Bool(b) = v {
        return b;
    }
    match v.as_stored_int() {
        Some(n) => n != 0,
        None => {
            diags.push(Diagnostic {
                level: Level::Error,
                message: "region conditional-group condition is not a comptime boolean or integer"
                    .to_string(),
                primary: crate::parser::expr_span(expr),
            });
            false
        }
    }
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
