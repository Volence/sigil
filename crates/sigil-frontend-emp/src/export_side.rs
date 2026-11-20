//! The `.emp` front end's side table for the build export (G0a spec 4.5): what the
//! lowering knows about every symbol it defined and the placed sections do not, namely
//! which module defined it, whether it is a procedure and how far its body runs, and
//! whether it is `pub`. Recorded while [`crate::resolve`] lowers each module (after the
//! module's rename, so every name is spelled as its label is, which is the listing's
//! spelling). Placing it (final addresses, extents in bytes) is the driver's step
//! (`sigil-harness` `export_symbols`): this crate does not depend on the linker, and
//! the crate graph (`sigil-cli/tests/crate_graph.rs`) keeps it that way.

use crate::ast;
use sigil_ir::Module;
use sigil_span::Span;
use std::collections::{HashMap, HashSet};

/// What the lowering knew about one symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmpSymbolKind {
    /// A label that is not a procedure's entry (a data item, a proc local, a RAM var).
    Label,
    /// A procedure entry, with its body as fragment indices into the label's section
    /// (see [`sigil_ir::ProcMark`]).
    Proc { start_fragment: usize, end_fragment: usize },
    /// A link-level `equ`.
    Equ,
}

/// One symbol the `.emp` lowering defined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpSymbol {
    /// The spelling its label (or equ) carries after the module's rename.
    pub name: String,
    /// The dotted id of the module that defined it.
    pub module: String,
    pub kind: EmpSymbolKind,
    /// `pub proc` / `pub equ`.
    pub public: bool,
}

/// Every symbol a `.emp` program build defined, in lowering order (first definition
/// of a name only).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProgramSymbols {
    pub symbols: Vec<EmpSymbol>,
    /// The instruction fragments every module lowered ([`sigil_ir::Module::code`]), by
    /// section name and fragment range, for the line table's `code` rows.
    pub code: Vec<(String, std::ops::Range<usize>)>,
    seen: HashSet<String>,
}

impl ProgramSymbols {
    /// Record what module `module_id` defined, from its lowered (and renamed) IR.
    /// `pub_names` is every name the module declares `pub`, spelled as its label is
    /// after the rename ([`pub_item_names`]); a label or equ named there is public.
    pub fn record_module(&mut self, module_id: &str, module: &Module, pub_names: &HashSet<String>) {
        self.code.extend(module.named_code_marks());
        let procs: HashMap<&str, &sigil_ir::ProcMark> =
            module.procs.iter().map(|p| (p.name.as_str(), p)).collect();
        for sec in &module.sections {
            for l in &sec.labels {
                if !self.seen.insert(l.name.clone()) {
                    continue;
                }
                let (kind, public) = match procs.get(l.name.as_str()) {
                    Some(p) => (
                        EmpSymbolKind::Proc { start_fragment: p.start_fragment, end_fragment: p.end_fragment },
                        p.public,
                    ),
                    None => (EmpSymbolKind::Label, pub_names.contains(&l.name)),
                };
                self.symbols.push(EmpSymbol { name: l.name.clone(), module: module_id.to_string(), kind, public });
            }
            for e in &sec.equ_syms {
                if !self.seen.insert(e.name.clone()) {
                    continue;
                }
                self.symbols.push(EmpSymbol {
                    name: e.name.clone(),
                    module: module_id.to_string(),
                    kind: EmpSymbolKind::Equ,
                    public: pub_names.contains(&e.name),
                });
            }
        }
    }

    /// Whether the program defined `name`.
    pub fn defines(&self, name: &str) -> bool {
        self.seen.contains(name)
    }
}

/// Every name `file` declares `pub` that can become a link symbol: the names the
/// resolver exports (`imports::exported_names`: `pub data`/`proc`/`dispatch`/`script`/
/// `offsets`/`equ`, the labels of a region-form `pub vars` block, and the comptime-only
/// names, which no label carries), plus the two `pub` item kinds that index does not
/// list, `pub table` (its base label) and `pub region`. Recurses into `section {}`.
pub fn pub_item_names(file: &ast::File) -> HashSet<String> {
    fn walk(items: &[ast::Item], out: &mut HashSet<String>) {
        for it in items {
            match it {
                ast::Item::Table(t) if t.public => {
                    out.insert(t.name.clone());
                }
                ast::Item::Region(r) if r.public => {
                    out.insert(r.name.clone());
                }
                ast::Item::Section(sec) => walk(&sec.items, out),
                _ => {}
            }
        }
    }
    let mut out: HashSet<String> = crate::resolve::imports::exported_names(file).into_iter().collect();
    walk(&file.items, &mut out);
    out
}


// ---------------------------------------------------------------------------------
// Layouts and constants (G0a spec 4.5)
// ---------------------------------------------------------------------------------
//
// What the lowering resolves, kept in a registry keyed by `(defining module, name)`.
// Layouts and constants are computed PER IMPORTER on demand (`layout.rs`): every module's
// evaluator indexes its own items plus the clones the resolver injects from the modules
// it imports. [`ProgramLayouts::record_importer`] reads each importer's view of every
// struct, bitfield, enum, overlay and `pub const` it can see, after the module is
// lowered, and files it under the module that DECLARES the item. A key two views
// disagree on is a fault ([`EmpExportError::TwoLayouts`] / [`EmpExportError::TwoConsts`]),
// which is how an item that resolved to two layouts (A4's hazard) is refused instead of
// exported twice. The pass reads and records only: it runs in its own evaluator and its
// diagnostics are dropped, so it moves no byte and no diagnostic. Like
// [`ProgramSymbols`], it is placed into records by the driver (`sigil-harness`
// `export_layouts`), because this crate does not depend on the linker.
//
// ## Type spelling (the one place these rules are written)
//
// A field's or constant's `type` is the resolved type, spelled:
//
// | type                    | spelling                          |
// |-------------------------|-----------------------------------|
// | primitive               | `u8` `i8` `u16` `i16` `u32` `i32` `u16le` |
// | pointer `*T`            | `ptr<T>`, e.g. `ptr<Sst>`         |
// | array `[T; n]`          | `[T; n]`                          |
// | tuple                   | `(A, B)`                          |
// | struct, bitfield, enum, newtype | its own name (never a hidden injected name) |
// | `fixed<I, F>`           | `fx{I}.{F}`, e.g. `fx8.8`         |
// | refined `T where LO..HI`| `T where LO..HI`                  |
//
// A constant with no written type annotation is `int`; a boolean is `bool`; an enum
// value is its enum's name; a newtype or fixed-point value is that type's spelling.
//
// ## Units
//
// A `struct` layout's `size`, and each field's `offset` and `size`, are bytes from the
// start of the struct. A `bitfield` layout's `size` is its repr in bytes, and each
// field's `offset` is its least significant bit and its `size` its width, both in BITS,
// with `type: "bits"`. An `enum` layout's `size` is its repr in bytes, and it carries one
// field per payload-free variant: `offset` is the variant's DISCRIMINANT, `size` the
// repr's bytes, `type` the enum's name (the schema's `offset` is the only integer slot a
// variant value fits in). A comptime enum has no data layout and is not exported. An
// overlay is attached to its base struct's record: `offset` and `size` are its window's
// (bytes into the base struct, and the window's capacity), and its fields' offsets are
// relative to the window start.

use crate::layout::Ty;
use crate::resolve::hygiene::split_hidden;
use sigil_span::SourceId;
use std::collections::BTreeMap;

/// How much of the layout/constant registry a build computes (G0a spec 4.5).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutPass {
    /// Nothing: no registry, no evaluator, no cost. A build that writes no sidecar
    /// (`--no-build-json`, `--check`, no `-o`) and every gate or test build.
    #[default]
    Off,
    /// The export: each item evaluated once, in its OWNER's view (the module that
    /// declares it), and only in a module that declares one.
    Owner,
    /// The cross-check, A4's detector half: every importer's view of every item it
    /// can see, so an item two modules resolved differently is refused. Run by the
    /// `build_export_layouts_crosscheck` gate over the 7 shapes, not by a build.
    Full,
    /// [`Full`](Self::Full) with a planted disagreement (a non-owner view's struct is
    /// 2 bytes larger, its constants 1 larger): the gate's control, proving the
    /// cross-check is not blind. Never used by a build.
    FullSkewedForTest,
}

/// One field of a layout, an overlay or an enum (see the units above).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpField {
    pub name: String,
    pub offset: u32,
    pub size: u32,
    pub ty: String,
}

/// An overlay window over a base struct.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpOverlay {
    pub name: String,
    pub base: String,
    /// The module that declares the base struct (the overlay's own module is the key).
    pub base_module: String,
    pub offset: u32,
    pub size: u32,
    pub fields: Vec<EmpField>,
}

/// What a layout describes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmpLayoutKind {
    Struct,
    Bitfield,
    Enum,
}

/// One resolved layout, keyed `(module, name)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpLayout {
    pub name: String,
    pub module: String,
    pub kind: EmpLayoutKind,
    pub size: u32,
    pub fields: Vec<EmpField>,
    pub overlays: Vec<EmpOverlay>,
}

/// One exported constant: an integer, boolean or enum value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmpConst {
    pub name: String,
    pub module: String,
    pub ty: String,
    pub value: i64,
}

/// Why the registry refuses to hand out layouts or constants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmpExportError {
    /// One `(module, name)` resolved to two different layouts.
    TwoLayouts { module: String, name: String },
    /// One `(module, name)` resolved to two different constants.
    TwoConsts { module: String, name: String },
    /// An exported constant's value does not fit `i64`.
    ConstOutOfRange { module: String, name: String, value: i128 },
    /// A `pub const` of its own module that did not evaluate (it poisoned): the export
    /// does not silently omit it.
    Const { module: String, name: String, why: String },
    /// A layout the record cannot state (a size or offset past `u32`, an overlay whose
    /// base struct is not exported or not unique).
    Layout { module: String, name: String, why: String },
}

/// How one constant's value classifies.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ConstShape {
    /// An integer, boolean or enum value, with its type spelling.
    Exported { ty: String, value: i128 },
    /// A value of another kind (named by [`crate::value::Value::type_name`]).
    Skipped(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ConstEntry {
    shape: ConstShape,
    /// Set by the DECLARING module's own view of its own `pub const`: the only view
    /// that exports. A clone another module imported only cross-checks it.
    exported_by_owner: bool,
}

/// Every layout and constant the lowering resolved, with the faults it met. Built while
/// [`crate::resolve`] lowers (only for a build that records, see `BuiltProgram::layouts`).
#[derive(Clone, Debug, Default)]
pub struct ProgramLayouts {
    layouts: BTreeMap<(String, String), EmpLayout>,
    overlays: BTreeMap<(String, String), EmpOverlay>,
    consts: BTreeMap<(String, String), ConstEntry>,
    faults: Vec<EmpExportError>,
}

/// What one importer's evaluator resolved, produced off the registry so the evaluation
/// can run on the comptime stack and the filing happens here.
enum Seen {
    Layout(EmpLayout),
    Overlay { module: String, overlay: EmpOverlay },
    Const { module: String, name: String, shape: ConstShape, owner: bool },
    Fault(EmpExportError),
}

fn demangle(name: &str) -> &str {
    split_hidden(name).map_or(name, |(_, n)| n)
}

/// The spelling of `ty` (rules in the module header above).
pub fn type_spelling(ty: &Ty) -> String {
    match ty {
        Ty::Prim { width, signed, le } => {
            format!("{}{}{}", if *signed { "i" } else { "u" }, u32::from(*width) * 8, if *le { "le" } else { "" })
        }
        Ty::Ptr(inner) => format!("ptr<{}>", type_spelling(inner)),
        Ty::Array(inner, n) => format!("[{}; {n}]", type_spelling(inner)),
        Ty::Tuple(elems) => format!("({})", elems.iter().map(type_spelling).collect::<Vec<_>>().join(", ")),
        Ty::Struct(n) | Ty::Bitfield(n) | Ty::Enum(n) | Ty::Newtype(n) => demangle(n).to_string(),
        Ty::Fixed { i, f } => format!("fx{i}.{f}"),
        Ty::Refined { inner, lo, hi } => format!("{} where {lo}..{hi}", type_spelling(inner)),
        Ty::Poison => "<poison>".to_string(),
    }
}

fn has_poison(ty: &Ty) -> bool {
    match ty {
        Ty::Poison => true,
        Ty::Ptr(t) | Ty::Array(t, _) => has_poison(t),
        Ty::Tuple(ts) => ts.iter().any(has_poison),
        Ty::Refined { inner, .. } => has_poison(inner),
        _ => false,
    }
}

/// An item the importer's OWN module declares and that did not resolve is a fault (its
/// own build would not call it an error if nothing used it, and the export must not
/// silently omit it); a clone another module imported that did not resolve is left to
/// that module's own view.
fn unresolved(out: &mut Vec<Seen>, owner: bool, module: String, name: String, what: &str) {
    if owner {
        out.push(Seen::Fault(EmpExportError::Layout {
            module,
            name,
            why: format!("the {what} did not resolve to a layout in its own module"),
        }));
    }
}

fn collect_items<'f>(items: &'f [ast::Item], out: &mut Vec<&'f ast::Item>) {
    for it in items {
        match it {
            ast::Item::Section(sec) => collect_items(&sec.items, out),
            _ => out.push(it),
        }
    }
}

impl ProgramLayouts {
    /// File one layout under `(module, name)`. A second, DIFFERENT layout for the same
    /// key is the two-layouts hazard (A4) and is recorded as a fault, never merged.
    pub fn record_layout(&mut self, layout: EmpLayout) {
        let key = (layout.module.clone(), layout.name.clone());
        match self.layouts.get(&key) {
            Some(existing) if *existing != layout => {
                self.faults.push(EmpExportError::TwoLayouts { module: key.0, name: key.1 });
            }
            Some(_) => {}
            None => {
                self.layouts.insert(key, layout);
            }
        }
    }

    fn record_overlay(&mut self, module: String, overlay: EmpOverlay) {
        let key = (module, overlay.name.clone());
        match self.overlays.get(&key) {
            Some(existing) if *existing != overlay => {
                self.faults.push(EmpExportError::TwoLayouts { module: key.0, name: key.1 });
            }
            Some(_) => {}
            None => {
                self.overlays.insert(key, overlay);
            }
        }
    }

    fn record_const(&mut self, module: String, name: String, shape: ConstShape, owner: bool) {
        let key = (module, name);
        match self.consts.get_mut(&key) {
            Some(existing) if existing.shape != shape => {
                self.faults.push(EmpExportError::TwoConsts { module: key.0, name: key.1 });
            }
            Some(existing) => existing.exported_by_owner |= owner,
            None => {
                self.consts.insert(key, ConstEntry { shape, exported_by_owner: owner });
            }
        }
    }

    /// Read what the importer `file` (a module's items plus the clones injected into
    /// it) resolves, and file it. `home` maps a source id to the module it belongs to.
    pub(crate) fn record_importer(
        &mut self,
        file: &ast::File,
        home: &HashMap<SourceId, String>,
        defines: &[(String, i128)],
        contracts: &crate::contract::InterfaceEnv,
        include_root: Option<&crate::Sandbox>,
        pass: LayoutPass,
    ) {
        if pass == LayoutPass::Off {
            return;
        }
        // The default export reads only the items this module declares; a module that
        // declares none of the kinds below costs a scan of its item list and nothing else.
        let owner_only = pass == LayoutPass::Owner;
        if owner_only && !declares_exportable(file) {
            return;
        }
        let skew = pass == LayoutPass::FullSkewedForTest;
        let seen = crate::eval::run_on_eval_stack(|| {
            resolve_importer(file, home, defines, contracts, include_root, owner_only, skew)
        });
        for s in seen {
            match s {
                Seen::Layout(l) => self.record_layout(l),
                Seen::Overlay { module, overlay } => self.record_overlay(module, overlay),
                Seen::Const { module, name, shape, owner } => self.record_const(module, name, shape, owner),
                Seen::Fault(f) => self.faults.push(f),
            }
        }
    }

    /// Per const shape, `(exported, skipped)` among the `pub const`s the build resolved:
    /// `int`, `bool`, `enum` and `typed` are exported shapes; every other
    /// [`crate::value::Value::type_name`] is skipped by kind.
    pub fn const_shape_counts(&self) -> BTreeMap<String, (usize, usize)> {
        let mut out: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for e in self.consts.values().filter(|e| e.exported_by_owner) {
            match &e.shape {
                ConstShape::Exported { ty, .. } => {
                    let kind = match ty.as_str() {
                        "bool" => "bool",
                        "int" => "int",
                        _ => "typed-or-enum",
                    };
                    out.entry(kind.to_string()).or_default().0 += 1;
                }
                ConstShape::Skipped(kind) => out.entry((*kind).to_string()).or_default().1 += 1,
            }
        }
        out
    }

    /// Per layout kind, how many layouts and overlays the build resolved.
    pub fn layout_counts(&self) -> (usize, usize, usize, usize) {
        let n = |k: EmpLayoutKind| self.layouts.values().filter(|l| l.kind == k).count();
        (n(EmpLayoutKind::Struct), n(EmpLayoutKind::Bitfield), n(EmpLayoutKind::Enum), self.overlays.len())
    }
}

/// An importer's view: every layout-bearing item and `pub const` in `file`, keyed to the
/// module that declares it. Items whose resolution reported a problem in this view are
/// left out (a build error elsewhere says so).
/// Whether `file` itself (not the clones injected into it) declares a struct, bitfield,
/// plain enum, named overlay or `pub const`.
fn declares_exportable(file: &ast::File) -> bool {
    let mut items = Vec::new();
    collect_items(&file.items, &mut items);
    let own = file.module.span.source;
    items.into_iter().any(|it| match it {
        ast::Item::Struct(d) => d.span.source == own,
        ast::Item::Bitfield(d) => d.span.source == own,
        ast::Item::Enum(d) => d.span.source == own && !d.comptime,
        ast::Item::Vars(d) => d.span.source == own && d.name.is_some(),
        ast::Item::Const(d) => d.span.source == own && d.public,
        _ => false,
    })
}

fn resolve_importer(
    file: &ast::File,
    home: &HashMap<SourceId, String>,
    defines: &[(String, i128)],
    contracts: &crate::contract::InterfaceEnv,
    include_root: Option<&crate::Sandbox>,
    owner_only: bool,
    skew: bool,
) -> Vec<Seen> {
    use crate::eval::Evaluator;
    let mut out = Vec::new();
    let mut ev = Evaluator::with_file(file);
    ev.seed_defines(defines);
    ev.seed_interfaces(contracts);
    if let Some(root) = include_root {
        ev.set_include_root(root.clone());
    }
    // `(declaring module, plain name)` of an item, from its home span or hidden spelling.
    let key = |span: Span, name: &str| -> Option<(String, String, bool)> {
        if let Some((def, plain)) = split_hidden(name) {
            return Some((def.to_string(), plain.to_string(), false));
        }
        let module = home.get(&span.source)?.clone();
        Some((module, name.to_string(), span.source == file.module.span.source))
    };
    let mut items = Vec::new();
    collect_items(&file.items, &mut items);
    for item in items {
        match item {
            ast::Item::Struct(s) => {
                if !ev.structs.get(s.name.as_str()).is_some_and(|d| std::ptr::eq(*d, s)) {
                    continue;
                }
                let Some((module, name, owner)) = key(s.span, &s.name) else { continue };
                if owner_only && !owner {
                    continue;
                }
                let l = ev.layout_of_struct(&s.name, s.span);
                if l.fields.len() != s.fields.len() || l.fields.iter().any(|f| has_poison(&f.ty)) {
                    unresolved(&mut out, owner, module, name, "struct");
                    continue;
                }
                let mut fields = Vec::with_capacity(l.fields.len());
                let mut bad = None;
                for f in &l.fields {
                    match (u32::try_from(f.offset), u32::try_from(f.size)) {
                        (Ok(offset), Ok(size)) => {
                            fields.push(EmpField { name: f.name.clone(), offset, size, ty: type_spelling(&f.ty) })
                        }
                        _ => bad = Some(f.name.clone()),
                    }
                }
                match (bad, u32::try_from(l.size)) {
                    (None, Ok(size)) => out.push(Seen::Layout(EmpLayout {
                        name,
                        module,
                        kind: EmpLayoutKind::Struct,
                        size: if skew && !owner { size + 2 } else { size },
                        fields,
                        overlays: Vec::new(),
                    })),
                    (bad, _) => out.push(Seen::Fault(EmpExportError::Layout {
                        module,
                        name,
                        why: format!("a size or offset past u32 ({})", bad.unwrap_or_else(|| "struct size".into())),
                    })),
                }
            }
            ast::Item::Bitfield(b) => {
                if !ev.bitfields.get(b.name.as_str()).is_some_and(|d| std::ptr::eq(*d, b)) {
                    continue;
                }
                let Some((module, name, owner)) = key(b.span, &b.name) else { continue };
                if owner_only && !owner {
                    continue;
                }
                let l = ev.layout_of_bitfield(&b.name, b.span);
                if l.fields.len() != b.fields.len() {
                    unresolved(&mut out, owner, module, name, "bitfield");
                    continue;
                }
                let fields = l
                    .fields
                    .iter()
                    .map(|f| EmpField { name: f.name.clone(), offset: f.lsb, size: f.bits, ty: "bits".to_string() })
                    .collect();
                out.push(Seen::Layout(EmpLayout {
                    name,
                    module,
                    kind: EmpLayoutKind::Bitfield,
                    size: l.repr_bits / 8,
                    fields,
                    overlays: Vec::new(),
                }));
            }
            ast::Item::Enum(e) if !e.comptime => {
                if !ev.enums.get(e.name.as_str()).is_some_and(|d| std::ptr::eq(*d, e)) {
                    continue;
                }
                let Some((module, name, owner)) = key(e.span, &e.name) else { continue };
                if owner_only && !owner {
                    continue;
                }
                let width = match e.repr.as_ref().map(|r| ev.resolve_type(r)) {
                    Some(Ty::Prim { width, .. }) => u32::from(width),
                    _ => 1,
                };
                let values = ev.enum_variant_values(e);
                let mut fields = Vec::new();
                let mut ok = true;
                for (v, val) in e.variants.iter().zip(&values) {
                    if !v.payload.is_empty() {
                        continue;
                    }
                    match val.and_then(|d| u32::try_from(d).ok()) {
                        Some(offset) => fields.push(EmpField { name: v.name.clone(), offset, size: width, ty: name.clone() }),
                        None => ok = false,
                    }
                }
                if ok {
                    out.push(Seen::Layout(EmpLayout {
                        name,
                        module,
                        kind: EmpLayoutKind::Enum,
                        size: width,
                        fields,
                        overlays: Vec::new(),
                    }));
                } else {
                    out.push(Seen::Fault(EmpExportError::Layout {
                        module,
                        name,
                        why: "an enum discriminant outside 0..=u32::MAX".into(),
                    }));
                }
            }
            ast::Item::Vars(v) => {
                let Some(oname) = v.name.as_deref() else { continue };
                if !ev.overlays.get(oname).is_some_and(|d| std::ptr::eq(*d, v)) {
                    continue;
                }
                let Some((module, name, owner)) = key(v.span, oname) else { continue };
                if owner_only && !owner {
                    continue;
                }
                let info = ev.overlay_layout(oname, v.span);
                if info.poisoned || info.fields.len() != v.fields.len() {
                    unresolved(&mut out, owner, module, name, "overlay");
                    continue;
                }
                let tys: Vec<String> = v.fields.iter().map(|f| type_spelling(&ev.resolve_type(&f.ty))).collect();
                let conv = |x: i128| u32::try_from(x).ok();
                let fields: Option<Vec<EmpField>> = info
                    .fields
                    .iter()
                    .zip(tys)
                    .map(|((n, off, size), ty)| Some(EmpField { name: n.clone(), offset: conv(*off)?, size: conv(*size)?, ty }))
                    .collect();
                // The base struct as THIS module's scope names it: its declaration says which
                // module declares it, so two modules' same-named structs stay apart.
                let base_module = ev
                    .structs
                    .get(info.base_struct.as_str())
                    .and_then(|d| key(d.span, &info.base_struct))
                    .map(|(m, _, _)| m);
                match (fields, conv(info.window_offset), conv(info.window_size), base_module) {
                    (Some(fields), Some(offset), Some(size), Some(base_module)) => out.push(Seen::Overlay {
                        module,
                        overlay: EmpOverlay {
                            name,
                            base: demangle(&info.base_struct).to_string(),
                            base_module,
                            offset,
                            size,
                            fields,
                        },
                    }),
                    _ => out.push(Seen::Fault(EmpExportError::Layout {
                        module,
                        name,
                        why: "an overlay offset or size past u32, or a base struct no module declares".into(),
                    })),
                }
            }
            ast::Item::Const(c) if c.public => {
                if !ev.consts.get(c.name.as_str()).is_some_and(|d| std::ptr::eq(*d, c)) {
                    continue;
                }
                let Some((module, name, owner)) = key(c.span, &c.name) else { continue };
                if owner_only && !owner {
                    continue;
                }
                let v = ev.resolve_const(&c.name, c.span);
                let shape = match v {
                    crate::value::Value::Poison => {
                        if owner {
                            out.push(Seen::Fault(EmpExportError::Const {
                                module,
                                name,
                                why: "the constant did not evaluate in its own module".into(),
                            }));
                        }
                        continue;
                    }
                    crate::value::Value::Int(n) => {
                        let ty = match &c.ty {
                            Some(t) => type_spelling(&ev.resolve_type(t)),
                            None => "int".to_string(),
                        };
                        ConstShape::Exported { ty, value: if skew && !owner { n + 1 } else { n } }
                    }
                    crate::value::Value::Bool(b) => ConstShape::Exported { ty: "bool".into(), value: i128::from(b) },
                    crate::value::Value::Typed { ty, val } => match val.as_stored_int() {
                        Some(n) => ConstShape::Exported { ty: type_spelling(&ty), value: n },
                        None => ConstShape::Skipped("typed"),
                    },
                    crate::value::Value::Enum { ty_name, variant, payload } => {
                        let decl = ev.enums.get(ty_name.as_str()).copied();
                        match decl {
                            Some(d) if !d.comptime && payload.is_empty() => {
                                let values = ev.enum_variant_values(d);
                                match d.variants.iter().position(|x| x.name == variant).and_then(|i| values[i]) {
                                    Some(n) => ConstShape::Exported { ty: demangle(&ty_name).to_string(), value: n },
                                    None => {
                                        if owner {
                                            out.push(Seen::Fault(EmpExportError::Const {
                                                module,
                                                name,
                                                why: "the enum variant has no discriminant".into(),
                                            }));
                                        }
                                        continue;
                                    }
                                }
                            }
                            _ => ConstShape::Skipped("comptime-enum"),
                        }
                    }
                    other => ConstShape::Skipped(other.type_name()),
                };
                if owner {
                    if let ConstShape::Exported { value, .. } = &shape {
                        if i64::try_from(*value).is_err() {
                            out.push(Seen::Fault(EmpExportError::ConstOutOfRange {
                                module: module.clone(),
                                name: name.clone(),
                                value: *value,
                            }));
                        }
                    }
                }
                out.push(Seen::Const { module, name, shape, owner });
            }
            _ => {}
        }
    }
    out
}

/// Every resolved layout, overlays attached to their base struct. Refuses
/// ([`EmpExportError::TwoLayouts`]) a `(module, name)` that resolved to two layouts.
pub fn emp_layouts(reg: &ProgramLayouts) -> Result<Vec<EmpLayout>, EmpExportError> {
    if let Some(f) = reg.faults.iter().find(|f| matches!(f, EmpExportError::TwoLayouts { .. } | EmpExportError::Layout { .. })) {
        return Err(f.clone());
    }
    let mut out: Vec<EmpLayout> = reg.layouts.values().cloned().collect();
    for ((module, _), ov) in &reg.overlays {
        let cands: Vec<usize> = out
            .iter()
            .enumerate()
            .filter(|(_, l)| l.kind == EmpLayoutKind::Struct && l.name == ov.base && l.module == ov.base_module)
            .map(|(i, _)| i)
            .collect();
        match cands.as_slice() {
            [i] => out[*i].overlays.push(ov.clone()),
            other => {
                return Err(EmpExportError::Layout {
                    module: module.clone(),
                    name: ov.name.clone(),
                    why: format!(
                        "its base struct `{}::{}` matches {} exported layouts, expected one",
                        ov.base_module,
                        ov.base,
                        other.len()
                    ),
                })
            }
        }
    }
    Ok(out)
}

/// Every exported constant: each `pub const` of a lowered module whose value is an
/// integer, boolean or enum. Refuses a `(module, name)` that resolved to two values
/// ([`EmpExportError::TwoConsts`]) and a value past `i64` ([`EmpExportError::ConstOutOfRange`]).
pub fn emp_consts(reg: &ProgramLayouts) -> Result<Vec<EmpConst>, EmpExportError> {
    if let Some(f) = reg.faults.iter().find(|f| matches!(f, EmpExportError::TwoConsts { .. } | EmpExportError::ConstOutOfRange { .. } | EmpExportError::Const { .. })) {
        return Err(f.clone());
    }
    let mut out = Vec::new();
    for ((module, name), e) in &reg.consts {
        if !e.exported_by_owner {
            continue;
        }
        if let ConstShape::Exported { ty, value } = &e.shape {
            let value = i64::try_from(*value)
                .map_err(|_| EmpExportError::ConstOutOfRange { module: module.clone(), name: name.clone(), value: *value })?;
            out.push(EmpConst { name: name.clone(), module: module.clone(), ty: ty.clone(), value });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{AddressSpace, Cpu, EquSym, Expr, Label, Section, SectionPlacement};
    use sigil_span::{SourceId, Span};

    fn file(src: &str) -> ast::File {
        let (f, d) = crate::parse_str(src);
        assert!(d.iter().all(|d| d.level != sigil_span::Level::Error), "{d:?}");
        f
    }

    #[test]
    fn pub_item_names_lists_every_pub_label_kind_and_no_private_one() {
        let f = file(
            "module m\n\
             pub data PubData: [u8; 1] = [1]\n\
             data PrivData: [u8; 1] = [2]\n\
             pub proc PubProc() { rts }\n\
             proc PrivProc() { rts }\n\
             pub equ PUB_EQU = 3\n\
             equ PRIV_EQU = 4\n",
        );
        let names = pub_item_names(&f);
        for n in ["PubData", "PubProc", "PUB_EQU"] {
            assert!(names.contains(n), "{n} missing from {names:?}");
        }
        for n in ["PrivData", "PrivProc", "PRIV_EQU"] {
            assert!(!names.contains(n), "{n} is private: {names:?}");
        }
    }

    #[test]
    fn record_module_marks_pub_labels_and_equs_public() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let module = Module {
            sections: vec![Section {
                name: "text".into(),
                cpu: Cpu::M68000,
                vma_base: None,
                lma: 0,
                space: AddressSpace::Image,
                labels: vec![Label { name: "PubData".into(), offset: 0 }, Label { name: "PrivData".into(), offset: 1 }],
                fragments: vec![],
                placement: SectionPlacement::Pinned,
                reserved_span: 0,
                group: None,
                bank: None,
                equ_syms: vec![EquSym { name: "PUB_EQU".into(), expr: Expr::Int(3), span }],
            }],
            link_asserts: vec![],
            comptime_guards: 0,
            procs: vec![],
            code: vec![],
        };
        let pubs: HashSet<String> = ["PubData", "PUB_EQU"].iter().map(|s| s.to_string()).collect();
        let mut t = ProgramSymbols::default();
        t.record_module("m", &module, &pubs);
        let public = |n: &str| t.symbols.iter().find(|s| s.name == n).unwrap().public;
        assert!(public("PubData"));
        assert!(!public("PrivData"));
        assert!(public("PUB_EQU"));
    }

    fn layout(size: u32) -> EmpLayout {
        EmpLayout {
            name: "Sst".into(),
            module: "engine.objects.sst".into(),
            kind: EmpLayoutKind::Struct,
            size,
            fields: vec![EmpField { name: "code_addr".into(), offset: 0, size: 2, ty: "ObjRoutine".into() }],
            overlays: Vec::new(),
        }
    }

    #[test]
    fn two_layouts_is_refused() {
        let mut reg = ProgramLayouts::default();
        reg.record_layout(layout(80));
        reg.record_layout(layout(80)); // the same answer twice is one answer
        assert_eq!(emp_layouts(&reg).unwrap().len(), 1);
        reg.record_layout(layout(82)); // a second, different answer for the same key
        assert_eq!(
            emp_layouts(&reg),
            Err(EmpExportError::TwoLayouts { module: "engine.objects.sst".into(), name: "Sst".into() })
        );
        // A different module's `Sst` is a different key, not a second answer.
        let mut other = ProgramLayouts::default();
        other.record_layout(layout(80));
        other.record_layout(EmpLayout { module: "game.sst".into(), size: 82, ..layout(82) });
        assert_eq!(emp_layouts(&other).unwrap().len(), 2);
    }

    #[test]
    fn two_consts_is_refused() {
        let int = |v: i128| ConstShape::Exported { ty: "int".into(), value: v };
        let mut reg = ProgramLayouts::default();
        reg.record_const("m".into(), "K".into(), int(1), true);
        reg.record_const("m".into(), "K".into(), int(1), false);
        assert_eq!(emp_consts(&reg).unwrap().len(), 1);
        reg.record_const("m".into(), "K".into(), int(2), false);
        assert_eq!(emp_consts(&reg), Err(EmpExportError::TwoConsts { module: "m".into(), name: "K".into() }));
        // A different kind of value is a different answer too.
        let mut kinds = ProgramLayouts::default();
        kinds.record_const("m".into(), "K".into(), int(1), true);
        kinds.record_const("m".into(), "K".into(), ConstShape::Skipped("data"), false);
        assert!(matches!(emp_consts(&kinds), Err(EmpExportError::TwoConsts { .. })));
    }

    #[test]
    fn type_spelling_follows_the_documented_table() {
        let prim = |width, signed| Ty::Prim { width, signed, le: false };
        assert_eq!(type_spelling(&prim(2, false)), "u16");
        assert_eq!(type_spelling(&prim(1, true)), "i8");
        assert_eq!(type_spelling(&Ty::Ptr(Box::new(Ty::Struct("Sst".into())))), "ptr<Sst>");
        assert_eq!(type_spelling(&Ty::Fixed { i: 8, f: 8 }), "fx8.8");
        assert_eq!(type_spelling(&Ty::Array(Box::new(prim(1, false)), 4)), "[u8; 4]");
        assert_eq!(type_spelling(&Ty::Newtype("$engine.types$Coord".into())), "Coord");
    }
}
