//! One name environment per declaration (A4, Empyrean v2 step 0).
//!
//! An imported comptime item (a struct, enum, bitfield, newtype, comptime fn,
//! context, `vars` overlay, a const the definition-site fold cannot reduce, or a
//! `pub data` type stub) is injected into each IMPORTING module's item list so the
//! importer's evaluator can see it. Before this module existed, the injected clone
//! carried its free names verbatim, so they resolved in the IMPORTER's scope: a
//! struct's `[u8; N]` read the importer's `N`, a `pub comptime fn` called the
//! importer's same-named helper, and a private helper the defining module relied on
//! was simply absent (`aeon/docs/EMP_PITFALLS.md` §2 and §8).
//!
//! The fix is hygiene at injection time:
//!
//! 1. [`free_names`] lists the identifiers an item references, minus its own
//!    parameters and locals.
//! 2. Each free name is looked up in the DEFINING module's environment (its own
//!    items, private included, then its own `use` imports). A name bound there to a
//!    comptime item is renamed to [`hidden_name`]`(origin, name)`, a spelling no
//!    source can write, and that item is injected too, under the hidden name, with
//!    its own free names treated the same way in ITS defining module.
//!    [`hygienic_closure`] produces that set, memoized per program.
//! 3. In each importer, a hidden dependency the importer ALSO binds to the very same
//!    item (it imported it too) is mapped back to its plain name and not injected
//!    twice ([`localize`]). Where every dependency maps back, the injected items are
//!    exactly the pre-hygiene clones, so an importer whose names already agreed with
//!    the definition sees no change at all.
//!
//! Prelude names, `-D` defines and `Game.*` contract members are global and are not
//! renamed. Labels (procs, data, equs, region fields) are link symbols, resolved by
//! the link rename pass, and are not renamed here either.
//!
//! A free name bound nowhere at the definition but bound to a comptime item by an
//! importer is the old capture in its purest form: the item only ever worked because
//! of the importer's imports. That is refused, `[name.unbound-at-definition]`.
use super::manifest::{Manifest, ParsedModule};
use super::ConstFoldCache;
use crate::ast;
use sigil_span::{Diagnostic, Level, Span};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

/// The hidden spelling of item `name` defined in module `def_id`: `$def_id$name`.
/// The `mangled_equ` shape (`resolve/imports.rs`): unspellable from source (the
/// lexer has no `$` identifier start), and unique, because a module has one item
/// per name and kind.
pub fn hidden_name(def_id: &str, name: &str) -> String {
    format!("${def_id}${name}")
}

/// Split a [`hidden_name`] back into `(def_id, name)`. `None` for a plain name.
fn split_hidden(s: &str) -> Option<(&str, &str)> {
    s.strip_prefix('$')?.split_once('$')
}

/// Single-segment callees the evaluator dispatches AHEAD of user fns
/// (`eval/call.rs`, `eval_call`): guards, the data-monoid constructors, and the
/// compression/layout intrinsics. Not user-shadowable, so never renamed: a renamed
/// spelling would route a builtin call to a user fn of the same name.
const RESERVED_CALLEES: &[&str] = &[
    "ensure", "ensure_fatal", "byte", "bytes", "winptr", "bankid", "extern", "cycles",
    "pad_to_cycles", "here", "span", "embed", "import", "zx0", "s4lz", "kosinski", "kosinski_m",
    "kosplus", "kosplus_m", "saxman", "enigma", "nemesis", "comper", "rocket",
];

// ---- the walker --------------------------------------------------------

/// A scoped walk over one item that hands every FREE name occurrence (a slot the
/// evaluator resolves in the module environment) to `f`. Locals (parameters,
/// `let`/`var`, `for` variables, lambda parameters, pattern bindings, asm labels)
/// shadow for the rest of their block and are never handed out.
struct Walker<'f> {
    scope: Vec<String>,
    f: &'f mut dyn FnMut(&mut String),
}

impl Walker<'_> {
    fn name(&mut self, slot: &mut String) {
        if slot.starts_with('.')
            || matches!(slot.as_str(), "true" | "false" | "none")
            || self.scope.iter().any(|s| s == slot)
        {
            return;
        }
        (self.f)(slot);
    }

    fn path_head(&mut self, p: &mut ast::Path) {
        if let Some(head) = p.segments.first_mut() {
            self.name(head);
        }
    }

    fn callee(&mut self, p: &mut ast::Path) {
        let n = p.segments.len();
        if n == 0 {
            return;
        }
        if n == 2 && (p.segments[0] == "math" || p.segments[0] == "as") {
            return;
        }
        let last = p.segments[n - 1].as_str();
        if crate::eval::is_builtin(last) {
            // `xs.map(f)`: the receiver is a value path; a bare `len(..)` is the builtin.
            if n >= 2 {
                self.path_head(p);
            }
            return;
        }
        if n == 1 && RESERVED_CALLEES.contains(&last) {
            return;
        }
        self.path_head(p);
    }

    fn ty(&mut self, t: &mut ast::Type) {
        match t {
            ast::Type::Named(p) => self.path_head(p),
            ast::Type::Ptr(inner) => self.ty(inner),
            ast::Type::Array(inner, len) => {
                self.ty(inner);
                self.expr(len);
            }
            ast::Type::Tuple(ts) => ts.iter_mut().for_each(|t| self.ty(t)),
            ast::Type::Fixed { .. } => {}
            ast::Type::Refined(inner, lo, hi) => {
                self.ty(inner);
                self.expr(lo);
                self.expr(hi);
            }
        }
    }

    fn arg(&mut self, a: &mut ast::Arg) {
        self.expr(&mut a.value);
    }

    fn expr(&mut self, e: &mut ast::Expr) {
        use ast::Expr as E;
        match e {
            E::Int(..) | E::Float(..) | E::Str(..) | E::LocalLabel(..) | E::Default(_) => {}
            E::Path(p) => self.path_head(p),
            E::Unary { expr, .. } => self.expr(expr),
            E::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            E::Call { callee, args, .. } => {
                self.callee(callee);
                args.iter_mut().for_each(|a| self.arg(a));
            }
            E::StructLit { ty, fields, .. } => {
                self.path_head(ty);
                fields.iter_mut().for_each(|(_, v)| self.expr(v));
            }
            E::ArrayLit { elems, .. } | E::TupleLit { elems, .. } => {
                elems.iter_mut().for_each(|x| self.expr(x))
            }
            E::Range { lo, hi, .. } => {
                self.expr(lo);
                self.expr(hi);
            }
            E::If { cond, then, els, .. } => {
                self.expr(cond);
                self.block(then);
                if let Some(els) = els {
                    self.block(els);
                }
            }
            E::For { var, iter, body, .. } => {
                self.expr(iter);
                let mark = self.scope.len();
                self.scope.push(var.clone());
                self.block(body);
                self.scope.truncate(mark);
            }
            E::Asm { body, .. } => self.asm_block(body),
            E::Lambda { params, body, .. } => {
                let mark = self.scope.len();
                self.scope.extend(params.iter().cloned());
                self.expr(body);
                self.scope.truncate(mark);
            }
            E::Match { scrutinee, arms, .. } => {
                self.expr(scrutinee);
                for arm in arms {
                    let mark = self.scope.len();
                    self.pattern(&mut arm.pat);
                    self.expr(&mut arm.body);
                    self.scope.truncate(mark);
                }
            }
            E::SizeOf(t, _) => self.ty(t),
            E::OffsetOf(t, _, _) => self.ty(t),
            E::Rescale { arg, .. } => self.expr(arg),
            E::Index { base, index, .. } => {
                self.expr(base);
                self.expr(index);
            }
            E::Field { base, .. } => self.expr(base),
        }
    }

    fn pattern(&mut self, p: &mut ast::Pattern) {
        match p {
            ast::Pattern::Wildcard(_) => {}
            ast::Pattern::Binding(name, _) => self.scope.push(name.clone()),
            ast::Pattern::Variant { path, subpats, .. } => {
                self.path_head(path);
                subpats.iter_mut().for_each(|s| self.pattern(s));
            }
        }
    }

    /// A statement block: its `let`s scope to the rest of the block only.
    fn block(&mut self, stmts: &mut [ast::Stmt]) {
        let mark = self.scope.len();
        for s in stmts {
            self.stmt(s);
        }
        self.scope.truncate(mark);
    }

    fn stmt(&mut self, s: &mut ast::Stmt) {
        use ast::Stmt as S;
        match s {
            S::Let { name, value, .. } => {
                self.expr(value);
                self.scope.push(name.clone());
            }
            S::LetTuple { names, value, .. } => {
                self.expr(value);
                self.scope.extend(names.iter().cloned());
            }
            S::Var { name, ty, value, .. } => {
                if let Some(t) = ty {
                    self.ty(t);
                }
                self.expr(value);
                self.scope.push(name.clone());
            }
            S::Assign { target, value, .. } => {
                self.path_head(target);
                self.expr(value);
            }
            S::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            S::Expr(e) | S::If(e) | S::For(e) => self.expr(e),
            S::While { cond, body, .. } => {
                self.expr(cond);
                self.block(body);
            }
            S::ComptimeBlock { body, .. } => self.block(body),
            S::Patch { name, ty, .. } => {
                self.ty(ty);
                self.scope.push(name.clone());
            }
            S::Bind { name, value, .. } => {
                self.expr(value);
                self.scope.push(name.clone());
            }
        }
    }

    /// An asm body: its labels are local to the whole body (a branch may name a
    /// label declared below it).
    fn asm_block(&mut self, body: &mut [ast::AsmStmt]) {
        let mark = self.scope.len();
        collect_asm_labels(body, &mut self.scope);
        for s in body.iter_mut() {
            self.asm(s);
        }
        self.scope.truncate(mark);
    }

    fn asm(&mut self, s: &mut ast::AsmStmt) {
        use ast::AsmStmt as A;
        match s {
            A::Label { .. } | A::Trap { .. } | A::RaiseError { .. } | A::RaiseException { .. } => {}
            A::Invoke { .. } => {}
            A::Instr(line) => {
                for m in &mut line.mnemonic {
                    self.text_or_splice(m);
                }
                if let Some(sz) = &mut line.size {
                    self.text_or_splice(sz);
                }
                line.operands.iter_mut().for_each(|o| self.operand(o));
            }
            A::Call(e) | A::Splice(e) => self.expr(e),
            A::Let { ty, .. } | A::AssumeSome { ty, .. } => self.ty(ty),
            A::If { cond, then, els, .. } => {
                self.expr(cond);
                self.asm_block(then);
                if let Some(els) = els {
                    self.asm_block(els);
                }
            }
            A::Assert { src, dest, .. } => {
                self.operand(src);
                if let Some((d, _)) = dest {
                    self.operand(d);
                }
            }
            A::With { ctx, cond, body, args, .. } => {
                self.name(ctx);
                if let Some(c) = cond {
                    self.expr(c);
                }
                args.iter_mut().for_each(|a| self.arg(a));
                self.asm_block(body);
            }
        }
    }

    fn text_or_splice(&mut self, t: &mut ast::TextOrSplice) {
        if let ast::TextOrSplice::Splice(e) = t {
            self.expr(e);
        }
    }

    fn operand(&mut self, o: &mut ast::Operand) {
        use ast::Operand as O;
        match o {
            O::Imm(e) | O::Splice(e) => self.expr(e),
            O::PreDec(inner) | O::PostInc(inner) => self.operand(inner),
            O::Ind { parts, size, .. } => {
                for (e, t) in parts {
                    self.expr(e);
                    if let Some(t) = t {
                        self.text_or_splice(t);
                    }
                }
                if let Some(t) = size {
                    self.text_or_splice(t);
                }
            }
            O::DispInd { disp, inner, field_size_override, .. } => {
                self.expr(disp);
                self.operand(inner);
                if let Some(t) = field_size_override {
                    self.text_or_splice(t);
                }
            }
            O::Plain { expr, size, .. } => {
                self.expr(expr);
                if let Some(t) = size {
                    self.text_or_splice(t);
                }
            }
        }
    }

    fn params(&mut self, params: &mut [(String, ast::Type, Span, Option<ast::Expr>)]) {
        for (_, t, _, default) in params.iter_mut() {
            self.ty(t);
            if let Some(d) = default {
                self.expr(d);
            }
        }
        self.scope.extend(params.iter().map(|(n, ..)| n.clone()));
    }

    fn vars_field(&mut self, f: &mut ast::VarsField) {
        self.ty(&mut f.ty);
        if let Some(a) = &mut f.align {
            self.expr(a);
        }
    }

    fn region_field(&mut self, r: &mut ast::RegionField) {
        match r {
            ast::RegionField::Typed(f) => self.vars_field(f),
            ast::RegionField::Pad { count, .. } => self.expr(count),
            ast::RegionField::Mark { .. } | ast::RegionField::Alias { .. } => {}
            ast::RegionField::Group { cond, then_body, else_body, .. } => {
                self.expr(cond);
                then_body.iter_mut().for_each(|f| self.region_field(f));
                else_body.iter_mut().for_each(|f| self.region_field(f));
            }
        }
    }

    /// The injectable item kinds. Every other kind is a no-op: it is never
    /// injected, so its names are never read in another module's scope.
    fn item(&mut self, item: &mut ast::Item) {
        use ast::Item as I;
        let mark = self.scope.len();
        match item {
            I::Const(c) => {
                if let Some(t) = &mut c.ty {
                    self.ty(t);
                }
                self.expr(&mut c.value);
            }
            I::Struct(s) => {
                if let Some(sz) = &mut s.size {
                    self.expr(sz);
                }
                for f in &mut s.fields {
                    self.ty(&mut f.ty);
                    for e in [&mut f.offset, &mut f.align, &mut f.default].into_iter().flatten() {
                        self.expr(e);
                    }
                }
                s.pads.iter_mut().for_each(|p| self.expr(&mut p.operand));
            }
            I::Enum(e) => {
                if let Some(r) = &mut e.repr {
                    self.ty(r);
                }
                self.scope.extend(e.variants.iter().map(|v| v.name.clone()));
                for v in &mut e.variants {
                    if let Some(x) = &mut v.value {
                        self.expr(x);
                    }
                    v.payload.iter_mut().for_each(|t| self.ty(t));
                }
            }
            I::Bitfield(b) => self.ty(&mut b.repr),
            I::Newtype(n) => {
                self.ty(&mut n.underlying);
                if let Some((lo, hi)) = &mut n.refine {
                    self.expr(lo);
                    self.expr(hi);
                }
                if let Some(s) = &mut n.sentinel {
                    self.expr(s);
                }
            }
            I::ComptimeFn(f) => {
                if let Some(r) = &mut f.ret {
                    self.ty(r);
                }
                self.params(&mut f.params);
                self.block(&mut f.body);
            }
            I::Context(c) => {
                self.params(&mut c.params);
                if let ast::ContextKind::Acquired { acquire, release } = &mut c.kind {
                    self.expr(acquire);
                    if let ast::ReleaseSpec::Code(r) = release {
                        self.expr(r);
                    }
                }
            }
            I::Vars(v) if v.name.is_some() => {
                v.fields.iter_mut().for_each(|f| self.vars_field(f));
                v.region_body.iter_mut().for_each(|r| self.region_field(r));
            }
            // A `pub data` TYPE stub: only its annotation is read in the importer.
            I::Data(d) => {
                if let Some(t) = &mut d.ty {
                    self.ty(t);
                }
            }
            _ => {}
        }
        self.scope.truncate(mark);
    }
}

fn collect_asm_labels(body: &[ast::AsmStmt], out: &mut Vec<String>) {
    for s in body {
        match s {
            ast::AsmStmt::Label { name, .. } => out.push(name.clone()),
            ast::AsmStmt::If { then, els, .. } => {
                collect_asm_labels(then, out);
                if let Some(els) = els {
                    collect_asm_labels(els, out);
                }
            }
            ast::AsmStmt::With { body, .. } => collect_asm_labels(body, out),
            _ => {}
        }
    }
}

/// The identifiers `item`'s types, initialisers and bodies reference, minus its
/// parameters and locals. Only the FIRST segment of a dotted path is a name in the
/// module environment (`Dir.Left`, `Sst.x_pos`), so that is what is listed.
/// Builtin method names and the evaluator's reserved callees are not names.
pub fn free_names(item: &ast::Item) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut copy = item.clone();
    let mut f = |s: &mut String| {
        out.insert(s.clone());
    };
    Walker { scope: Vec::new(), f: &mut f }.item(&mut copy);
    out
}

/// Rename every free occurrence of a key of `map` in `item` to its value. Locals
/// that shadow a key are left alone, exactly the positions [`free_names`] skips.
pub fn rename_free(item: &mut ast::Item, map: &BTreeMap<String, String>) {
    if map.is_empty() {
        return;
    }
    let mut f = |s: &mut String| {
        if let Some(to) = map.get(s.as_str()) {
            *s = to.clone();
        }
    };
    Walker { scope: Vec::new(), f: &mut f }.item(item);
}

// ---- bindings ----------------------------------------------------------

/// What a short name means in one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    /// An item: its defining module, and whether it is a comptime item (the kinds
    /// this module renames) rather than a link symbol.
    Item { origin: String, comptime: bool },
    /// A prelude name or a `-D` define: one meaning program-wide.
    Global(&'static str),
}

impl Binding {
    fn render(b: Option<&Binding>, name: &str) -> String {
        match b {
            Some(Binding::Item { origin, comptime: true }) => format!("{origin}.{name}"),
            Some(Binding::Item { origin, comptime: false }) => format!("{origin}.{name} (label)"),
            Some(Binding::Global(kind)) => format!("({kind})"),
            None => "-".to_string(),
        }
    }
}

fn is_comptime_kind(item: &ast::Item) -> bool {
    matches!(
        item,
        ast::Item::Const(_)
            | ast::Item::Struct(_)
            | ast::Item::Enum(_)
            | ast::Item::Bitfield(_)
            | ast::Item::Newtype(_)
            | ast::Item::ComptimeFn(_)
            | ast::Item::Context(_)
    ) || matches!(item, ast::Item::Vars(v) if v.name.is_some())
}

/// The name an item binds in its own module, if any (top level or one section deep).
fn item_name(item: &ast::Item) -> Option<&str> {
    use ast::Item as I;
    Some(match item {
        I::Const(d) => &d.name,
        I::Struct(d) => &d.name,
        I::Enum(d) => &d.name,
        I::Bitfield(d) => &d.name,
        I::Newtype(d) => &d.name,
        I::ComptimeFn(d) => &d.name,
        I::Context(d) => &d.name,
        I::Vars(d) => return d.name.as_deref(),
        I::Data(d) => &d.name,
        I::Proc(d) => &d.name,
        I::Equ(d) => &d.name,
        I::Offsets(d) => &d.name,
        I::Dispatch(d) => &d.name,
        I::Script(d) => &d.name,
        I::ExternConst(d) => &d.name,
        _ => return None,
    })
}

/// Every name `file` itself defines → whether ANY item of that name is a comptime
/// item. Region-form `vars` fields are labels.
fn own_names(file: &ast::File) -> HashMap<String, bool> {
    let mut out: HashMap<String, bool> = HashMap::new();
    let mut add = |n: &str, ct: bool| {
        let e = out.entry(n.to_string()).or_insert(false);
        *e |= ct;
    };
    let walk = |items: &[ast::Item], add: &mut dyn FnMut(&str, bool)| {
        for it in items {
            if let Some(n) = item_name(it) {
                add(n, is_comptime_kind(it));
            }
            if let ast::Item::Vars(v) = it {
                let mut labels = Vec::new();
                super::imports::region_field_names(v, &mut labels);
                labels.iter().for_each(|l| add(l, false));
            }
        }
    };
    walk(&file.items, &mut add);
    for it in &file.items {
        if let ast::Item::Section(s) = it {
            walk(&s.items, &mut add);
        }
    }
    out
}

/// The module environment a `use`-importing module sees: own items over `use`
/// imports over the prelude over `-D` defines — the precedence `ResolveEnv::build`
/// and the ambient prepend order give.
fn module_bindings(
    pm: &ParsedModule,
    manifest: &Manifest,
    prelude: Option<&ParsedModule>,
    defines: &[(String, i128)],
) -> HashMap<String, Binding> {
    let mut map: HashMap<String, Binding> = HashMap::new();
    for (k, _) in defines {
        map.insert(k.clone(), Binding::Global("define"));
    }
    if let Some(p) = prelude {
        if p.id != pm.id {
            for n in super::imports::exported_names(&p.file) {
                map.insert(n, Binding::Global("prelude"));
            }
        }
    }
    for u in super::imports::use_decls(&pm.file.items) {
        let base = u.base.segments.join(".");
        let Some(&bi) = manifest.by_id.get(&base) else { continue };
        let bm = &manifest.modules[bi];
        if bm.id == pm.id {
            continue;
        }
        let exported: HashSet<String> = super::imports::exported_names(&bm.file).into_iter().collect();
        let base_own = own_names(&bm.file);
        let mut bind = |n: &str| {
            if exported.contains(n) {
                let comptime = base_own.get(n).copied().unwrap_or(false);
                map.insert(n.to_string(), Binding::Item { origin: bm.id.clone(), comptime });
            }
        };
        match &u.names {
            ast::UseNames::Whole | ast::UseNames::Blank => {}
            ast::UseNames::Glob => exported.iter().for_each(|n| bind(n)),
            ast::UseNames::List(names) => names.iter().for_each(|n| bind(n)),
        }
    }
    for (n, comptime) in own_names(&pm.file) {
        map.insert(n, Binding::Item { origin: pm.id.clone(), comptime });
    }
    map
}

// ---- the memo and the closure ------------------------------------------

/// The definition-site analysis of every item of one name in one module.
struct Node {
    /// The items, as injected (overlay window stamped, foldable const folded), under
    /// their PUBLIC names with every comptime-bound free name already hidden.
    items: Vec<ast::Item>,
    /// `true` per item when it is a public, injectable top-level item.
    public: Vec<bool>,
    /// Each comptime dependency, as (origin module, name).
    deps: Vec<(String, String)>,
    /// Every free name of the items, with its binding at the definition.
    free: Vec<(String, Option<Binding>)>,
}

/// Per-build memo for [`hygienic_closure`], keyed by `(def_id, name)`.
///
/// Scoped to ONE build, like [`ConstFoldCache`], and for the same reason: a cache
/// that outlived a build and was keyed by names that repeat across builds would be
/// wrong silently.
pub(crate) struct HygieneMemo<'f> {
    prelude_id: Option<String>,
    defines: Vec<(String, i128)>,
    include_root: Option<PathBuf>,
    folds: &'f RefCell<ConstFoldCache>,
    bindings: HashMap<String, Rc<HashMap<String, Binding>>>,
    nodes: HashMap<(String, String), Rc<Node>>,
    faults: Vec<Diagnostic>,
    reported: HashSet<(String, String, String)>,
}

impl<'f> HygieneMemo<'f> {
    pub(crate) fn new(
        prelude_id: Option<&str>,
        defines: &[(String, i128)],
        include_root: Option<&std::path::Path>,
        folds: &'f RefCell<ConstFoldCache>,
    ) -> Self {
        HygieneMemo {
            prelude_id: prelude_id.map(str::to_string),
            defines: defines.to_vec(),
            include_root: include_root.map(|p| p.to_path_buf()),
            folds,
            bindings: HashMap::new(),
            nodes: HashMap::new(),
            faults: Vec::new(),
            reported: HashSet::new(),
        }
    }

    /// Take the `[name.unbound-at-definition]` errors accumulated so far.
    pub(crate) fn take_faults(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.faults)
    }

    fn bindings_of(&mut self, module_id: &str, manifest: &Manifest) -> Rc<HashMap<String, Binding>> {
        if let Some(b) = self.bindings.get(module_id) {
            return b.clone();
        }
        let prelude = self
            .prelude_id
            .as_deref()
            .and_then(|p| manifest.by_id.get(p))
            .map(|&i| &manifest.modules[i]);
        let b = match manifest.by_id.get(module_id) {
            Some(&i) => Rc::new(module_bindings(&manifest.modules[i], manifest, prelude, &self.defines)),
            None => Rc::new(HashMap::new()),
        };
        self.bindings.insert(module_id.to_string(), b.clone());
        b
    }

    /// The definition-site analysis of `name` in module `def_id`.
    fn node(&mut self, def_id: &str, name: &str, manifest: &Manifest) -> Rc<Node> {
        let key = (def_id.to_string(), name.to_string());
        if let Some(n) = self.nodes.get(&key) {
            return n.clone();
        }
        let Some(&di) = manifest.by_id.get(def_id) else {
            let empty = Rc::new(Node { items: vec![], public: vec![], deps: vec![], free: vec![] });
            self.nodes.insert(key, empty.clone());
            return empty;
        };
        let def = &manifest.modules[di];
        let env = self.bindings_of(def_id, manifest);
        let mut items = Vec::new();
        let mut public = Vec::new();
        for it in def.file.items.iter().chain(def.file.items.iter().filter_map(|i| match i {
            ast::Item::Section(s) => Some(s.items.iter()),
            _ => None,
        }).flatten())
        {
            if item_name(it) != Some(name) {
                continue;
            }
            if let Some(n) = super::pub_comptime_name(it) {
                debug_assert_eq!(n, name);
                items.push(self.prepare(def, it));
                public.push(true);
            } else if is_comptime_kind(it) {
                items.push(self.prepare(def, it));
                public.push(false);
            } else if super::pub_struct_data_name(it).is_some() {
                items.push(super::data_type_stub(it));
                public.push(true);
            }
        }
        let mut free: BTreeSet<String> = BTreeSet::new();
        for it in &items {
            free.extend(free_names(it));
        }
        let mut deps = Vec::new();
        let mut rename = BTreeMap::new();
        let mut free_rows = Vec::new();
        for f in free {
            let b = env.get(&f).cloned();
            if let Some(Binding::Item { origin, comptime: true }) = &b {
                rename.insert(f.clone(), hidden_name(origin, &f));
                deps.push((origin.clone(), f.clone()));
            }
            free_rows.push((f, b));
        }
        for it in &mut items {
            rename_free(it, &rename);
        }
        let node = Rc::new(Node { items, public, deps, free: free_rows });
        self.nodes.insert(key, node.clone());
        node
    }

    /// One item as it is injected: an overlay's window stamped against its defining
    /// file, a `const` folded at the definition site where it folds.
    fn prepare(&mut self, def: &ParsedModule, item: &ast::Item) -> ast::Item {
        let mut cloned = item.clone();
        super::stamp_overlay_window(&def.file, &mut cloned);
        if let ast::Item::Const(c) = &mut cloned {
            let folded = self.folds.borrow_mut().folded(
                &def.id,
                &def.file,
                &c.name,
                &self.defines,
                self.include_root.as_deref(),
            );
            if let Some(lit) = folded {
                c.value = ast::Expr::Int(lit, c.span);
            }
        }
        cloned
    }
}

fn set_item_name(item: &mut ast::Item, to: &str) {
    use ast::Item as I;
    let slot = match item {
        I::Const(d) => &mut d.name,
        I::Struct(d) => &mut d.name,
        I::Enum(d) => &mut d.name,
        I::Bitfield(d) => &mut d.name,
        I::Newtype(d) => &mut d.name,
        I::ComptimeFn(d) => &mut d.name,
        I::Context(d) => &mut d.name,
        I::Vars(d) => {
            d.name = Some(to.to_string());
            return;
        }
        I::Data(d) => &mut d.name,
        _ => return,
    };
    *slot = to.to_string();
}

/// The public item(s) named `name` in `def`, under their public name, with each free
/// name bound in the DEFINING module's environment renamed to its [`hidden_name`],
/// plus every such dependency (renamed the same way in ITS defining module), each
/// under its hidden name. Every analysis is memoized in `memo`, so each `(def_id,
/// name)` is elaborated once per program; the returned vector is a fresh clone.
pub(crate) fn hygienic_closure(
    def: &ParsedModule,
    name: &str,
    manifest: &Manifest,
    memo: &mut HygieneMemo,
) -> Vec<ast::Item> {
    let node = memo.node(&def.id, name, manifest);
    let mut out: Vec<ast::Item> = node
        .items
        .iter()
        .zip(&node.public)
        .filter(|(_, p)| **p)
        .map(|(i, _)| i.clone())
        .collect();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut stack: Vec<(String, String)> = node.deps.clone();
    while let Some(dep) = stack.pop() {
        if !seen.insert(dep.clone()) {
            continue;
        }
        let dn = memo.node(&dep.0, &dep.1, manifest);
        let hidden = hidden_name(&dep.0, &dep.1);
        for it in &dn.items {
            let mut h = it.clone();
            set_item_name(&mut h, &hidden);
            out.push(h);
        }
        stack.extend(dn.deps.iter().cloned());
    }
    out
}

/// Fit the injected items of ONE importer to its own environment, in place.
///
/// `injected` holds every injected item with its `(def_id, name)` origin. A hidden
/// dependency the importer binds to the SAME item is mapped back to the plain name
/// and dropped (the importer already has that item, injected through its own
/// import); every other hidden dependency is kept, once. A free name unbound at an
/// item's definition that the importer binds to a comptime item is refused.
pub(crate) fn localize(
    importer: &ParsedModule,
    injected: Vec<(ast::Item, String, String)>,
    manifest: &Manifest,
    memo: &mut HygieneMemo,
) -> Vec<ast::Item> {
    let env = memo.bindings_of(&importer.id, manifest);
    let maps_back = |origin: &str, n: &str| {
        matches!(env.get(n), Some(Binding::Item { origin: o, comptime: true }) if o == origin)
    };
    let mut back: BTreeMap<String, String> = BTreeMap::new();
    let mut kept_hidden: HashSet<(String, std::mem::Discriminant<ast::Item>)> = HashSet::new();
    let mut hidden: Vec<ast::Item> = Vec::new();
    let mut plain: Vec<ast::Item> = Vec::new();
    let mut checked: HashSet<(String, String)> = HashSet::new();
    for (item, origin, name) in injected {
        if checked.insert((origin.clone(), name.clone())) {
            let node = memo.node(&origin, &name, manifest);
            for (f, b) in &node.free {
                if b.is_some() {
                    continue;
                }
                if let Some(Binding::Item { origin: co, comptime: true }) = env.get(f) {
                    if memo.reported.insert((origin.clone(), name.clone(), f.clone())) {
                        let span = node.items.first().map(item_span).unwrap_or(importer.file.module.span);
                        memo.faults.push(Diagnostic {
                            level: Level::Error,
                            message: format!(
                                "[name.unbound-at-definition] `{f}` in `{name}` is not bound in module \
                                 `{origin}`; add a `use` there (it resolved only through the importer \
                                 `{}`, to `{co}.{f}`)",
                                importer.id
                            ),
                            primary: span,
                        });
                    }
                }
            }
        }
        let item_nm = item_name(&item).unwrap_or("").to_string();
        match split_hidden(&item_nm) {
            Some((o, n)) => {
                if maps_back(o, n) {
                    back.insert(item_nm.clone(), n.to_string());
                } else if kept_hidden.insert((item_nm.clone(), std::mem::discriminant(&item))) {
                    hidden.push(item);
                }
            }
            None => plain.push(item),
        }
    }
    // Hidden dependencies first: they can never collide with a plain name, and the
    // plain items keep their pre-hygiene relative order (last-wins precedence).
    let mut out = hidden;
    out.extend(plain);
    if !back.is_empty() {
        for it in &mut out {
            rename_free(it, &back);
        }
    }
    out
}

fn item_span(item: &ast::Item) -> Span {
    use ast::Item as I;
    match item {
        I::Const(d) => d.span,
        I::Struct(d) => d.span,
        I::Enum(d) => d.span,
        I::Bitfield(d) => d.span,
        I::Newtype(d) => d.span,
        I::ComptimeFn(d) => d.span,
        I::Context(d) => d.span,
        I::Vars(d) => d.span,
        I::Data(d) => d.span,
        _ => Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
    }
}

// ---- the report --------------------------------------------------------

/// One `--report name-env` row: in `importer`, the injected `item` (from `def`)
/// names `name`, bound at the definition to `at_def` and in the importer to
/// `caller`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameEnvRow {
    pub importer: String,
    pub def: String,
    pub item: String,
    pub name: String,
    pub at_def: String,
    pub caller: String,
    /// The definition binds a comptime item and the importer binds the name to
    /// something else: the injected item used to MEAN the importer's binding.
    pub differs: bool,
    /// Bound nowhere at the definition, bound to a comptime item by the importer.
    pub unbound: bool,
}

/// The rows for one importer and one injected item.
pub(crate) fn report_rows(
    importer: &ParsedModule,
    def_id: &str,
    name: &str,
    manifest: &Manifest,
    memo: &mut HygieneMemo,
    out: &mut Vec<NameEnvRow>,
) {
    let env = memo.bindings_of(&importer.id, manifest);
    let node = memo.node(def_id, name, manifest);
    for (f, b) in &node.free {
        let c = env.get(f);
        let def_ct = matches!(b, Some(Binding::Item { comptime: true, .. }));
        let caller_ct = matches!(c, Some(Binding::Item { comptime: true, .. }));
        if !def_ct && !(b.is_none() && caller_ct) {
            continue;
        }
        let differs = def_ct && c.is_some() && c != b.as_ref();
        let unbound = b.is_none() && caller_ct;
        out.push(NameEnvRow {
            importer: importer.id.clone(),
            def: def_id.to_string(),
            item: name.to_string(),
            name: f.clone(),
            at_def: Binding::render(b.as_ref(), f),
            caller: Binding::render(c, f),
            differs,
            unbound,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(src: &str) -> Vec<ast::Item> {
        let (file, diags) = crate::parse_str(src);
        assert!(diags.iter().all(|d| d.level != Level::Error), "{diags:?}");
        file.items
    }

    fn names(v: &[&str]) -> BTreeSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn free_names_excludes_params_and_locals() {
        let it = items(
            "module m\n\
             comptime fn f(x: int, n: Count) -> int {\n\
                 let y = x + LIMIT\n\
                 comptime var acc = 0\n\
                 for i in 0..n { acc = acc + i + y + helper(i) }\n\
                 let g = |z| z + OTHER\n\
                 return acc.len() + len(xs) + ensure(true, \"m\")\n\
             }\n",
        );
        // Params (x, n), lets (y, g), the var (acc), the loop var (i) and the lambda
        // param (z) are locals; builtins and reserved callees are not names.
        assert_eq!(free_names(&it[0]), names(&["Count", "LIMIT", "OTHER", "helper", "int", "xs"]));
    }

    #[test]
    fn free_names_sees_struct_field_types() {
        let it = items(
            "module m\n\
             struct S (size: SZ) {\n\
                 a: [u8; N],\n\
                 b: T,\n\
                 c: *Dir,\n\
             }\n",
        );
        assert_eq!(free_names(&it[0]), names(&["Dir", "N", "SZ", "T", "u8"]));
    }

    #[test]
    fn rename_free_leaves_a_shadowing_local_alone() {
        let mut it = items(
            "module m\ncomptime fn f(N: int) -> int { return N + M }\ncomptime fn g() -> int { return N }\n",
        );
        let map: BTreeMap<String, String> =
            [("N".to_string(), "$a$N".to_string()), ("M".to_string(), "$a$M".to_string())].into();
        rename_free(&mut it[0], &map);
        rename_free(&mut it[1], &map);
        assert_eq!(free_names(&it[0]), names(&["$a$M", "int"]));
        assert_eq!(free_names(&it[1]), names(&["$a$N", "int"]));
    }

    #[test]
    fn hidden_name_is_the_mangled_equ_shape() {
        assert_eq!(hidden_name("engine.vdp", "target_bits"), "$engine.vdp$target_bits");
        assert_eq!(split_hidden("$engine.vdp$target_bits"), Some(("engine.vdp", "target_bits")));
        assert_eq!(split_hidden("target_bits"), None);
    }

    /// Write `files` under a fresh root and run the name-env report from `main`.
    fn report(files: &[(&str, &str)]) -> Vec<NameEnvRow> {
        let dir = tempfile::tempdir().unwrap();
        for (rel, src) in files {
            std::fs::write(dir.path().join(rel), src).unwrap();
        }
        let (manifest, d) = Manifest::scan(dir.path());
        assert!(d.iter().all(|d| d.level != Level::Error), "{d:?}");
        let opts = crate::lower::LowerOptions {
            initial_cpu: sigil_ir::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        };
        let (rows, diags) = super::super::name_env_report(&manifest, "main", None, &opts);
        assert!(diags.iter().all(|d| d.level != Level::Error), "{diags:?}");
        rows
    }

    const P8B_A: &str = "module a\npub const N = 4\npub struct S {\n    x: [u8; N],\n    y: u16,\n}\n";
    const P8B_MAIN: &str = "module main\nuse a.{S}\nconst N = 10\nsection s (cpu: m68000, vma: $0) {\n  proc P () clobbers(d0) {\n        moveq   #sizeof(S), d0\n        rts\n  }\n}\n";

    #[test]
    fn report_names_a_struct_size_captured_by_the_importer() {
        let rows = report(&[("a.emp", P8B_A), ("main.emp", P8B_MAIN)]);
        let hit: Vec<_> = rows.iter().filter(|r| r.differs).collect();
        assert_eq!(hit.len(), 1, "{rows:#?}");
        assert_eq!((hit[0].importer.as_str(), hit[0].item.as_str(), hit[0].name.as_str()), ("main", "S", "N"));
        assert_eq!((hit[0].at_def.as_str(), hit[0].caller.as_str()), ("a.N", "main.N"));
        assert!(rows.iter().all(|r| !r.unbound));
    }

    #[test]
    fn report_names_a_helper_name_only_the_importer_binds() {
        let rows = report(&[
            ("helper.emp", "module helper\npub comptime fn lim() -> int {\n    return LIMIT\n}\n"),
            ("main.emp", "module main\nuse helper.{lim}\nconst LIMIT = 9\nsection s (cpu: m68000, vma: $0) {\n  proc P () clobbers(d0) {\n        moveq   #lim(), d0\n        rts\n  }\n}\n"),
        ]);
        let un: Vec<_> = rows.iter().filter(|r| r.unbound).collect();
        assert_eq!(un.len(), 1, "{rows:#?}");
        assert_eq!((un[0].item.as_str(), un[0].name.as_str(), un[0].at_def.as_str()), ("lim", "LIMIT", "-"));
        assert!(rows.iter().all(|r| !r.differs));
    }
}
