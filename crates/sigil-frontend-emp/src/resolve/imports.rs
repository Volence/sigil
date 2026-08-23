//! Per-module name resolution (Spec 2 §3.2): map each short name in scope to the
//! canonical cross-module symbol it refers to (own definitions, `use` imports,
//! or the prelude), and offer an "add `use`" fix-it for names that are exported
//! elsewhere but not yet imported.
use crate::ast;
use sigil_span::{Diagnostic, Level};
use std::collections::{HashMap, HashSet};

/// The canonical, collision-proof name of a top-level item: its module id and
/// item name joined by a dot. Item names contain no dots, so this is unambiguous.
pub fn canonical(module_id: &str, name: &str) -> String {
    format!("{module_id}.{name}")
}

/// The owner-mangled link name of a NON-`pub` equ (C1 item 4): `$module$NAME`,
/// exactly the hidden-symbol shape a non-export local label uses. The `$`
/// wrapping is unspellable from source, so the equ stays module-local and two
/// modules' same-named private equs no longer collide in the flat link table.
pub fn mangled_equ(module_id: &str, name: &str) -> String {
    format!("${module_id}${name}")
}

/// Every non-`pub` equ name a file declares (recursing into `section {}` bodies,
/// consistent with [`defined_names`]) — the C1-item-4 set the rename pass mangles
/// (both the equ's own [`EquSym`](sigil_ir::EquSym) and its in-module references,
/// in lockstep). `pub equ`s are excluded: they keep their plain cross-seam name.
pub fn nonpub_equ_names(file: &ast::File) -> Vec<String> {
    let mut out = Vec::new();
    collect_nonpub_equs(&file.items, &mut out);
    out
}

fn collect_nonpub_equs(items: &[ast::Item], out: &mut Vec<String>) {
    for it in items {
        match it {
            ast::Item::Equ(e) if !e.is_pub => out.push(e.name.clone()),
            ast::Item::Section(sec) => collect_nonpub_equs(&sec.items, out),
            _ => {}
        }
    }
}

/// Every `pub equ` name a file declares (recursing into `section {}` bodies) —
/// the cross-seam-contract equs that keep their plain link name (C1 item 4). Two
/// modules declaring the same `pub equ` genuinely collide in the flat link
/// table; `build_program` uses this to name both modules in that diagnostic.
pub fn pub_equ_names(file: &ast::File) -> Vec<String> {
    let mut out = Vec::new();
    collect_pub_equs(&file.items, &mut out);
    out
}

fn collect_pub_equs(items: &[ast::Item], out: &mut Vec<String>) {
    for it in items {
        match it {
            ast::Item::Equ(e) if e.is_pub => out.push(e.name.clone()),
            ast::Item::Section(sec) => collect_pub_equs(&sec.items, out),
            _ => {}
        }
    }
}

/// Every module's `pub` top-level LABEL/VALUE names → its module id.
/// (Types/consts/fns are handled by the ambient path in Task 3, but they are
/// indexed here too so `suggest_use` can point at any exported name.)
pub struct ExportIndex {
    /// name → list of module ids that export it (list to detect ambiguity).
    by_name: HashMap<String, Vec<String>>,
    /// (module_id, name) exported? — for qualified-reference validation.
    exported: HashSet<(String, String)>,
    /// (module_id, name) of every exported `pub equ`. A `pub equ` is a PLAIN
    /// cross-seam link symbol whose name is unique across the whole program
    /// (`build_program`'s `[equ.collision]` check enforces that), so an importer
    /// binds the short name to the bare symbol rather than to a module-qualified
    /// canonical one — see [`ExportIndex::import_target`].
    equ_exports: HashSet<(String, String)>,
}

impl ExportIndex {
    /// Build the export index from every module's `pub` top-level names.
    pub fn build(modules: &[(&str, &ast::File)]) -> Self {
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        let mut exported = HashSet::new();
        let mut equ_exports = HashSet::new();
        for (id, file) in modules {
            for name in exported_names(file) {
                // Guard against the same module appearing twice in `modules`:
                // a duplicated (module, name) must NOT double-count, or
                // `suggest_use` would wrongly see the name as ambiguous.
                let owners = by_name.entry(name.clone()).or_default();
                if !owners.iter().any(|o| o == id) {
                    owners.push((*id).to_string());
                }
                exported.insert(((*id).to_string(), name));
            }
            for name in pub_equ_names(file) {
                equ_exports.insert(((*id).to_string(), name));
            }
        }
        ExportIndex { by_name, exported, equ_exports }
    }

    /// Whether `module_id` exports a `pub` top-level name `name`.
    pub fn is_exported(&self, module_id: &str, name: &str) -> bool {
        self.exported.contains(&(module_id.to_string(), name.to_string()))
    }

    /// The symbol an importer must bind the short name `name` to when it comes
    /// from `module_id`. Ordinary items are renamed to their module-qualified
    /// [`canonical`] symbol, so two modules' same-named items never collide in
    /// the flat link table. A `pub equ` is the exception: its definition keeps
    /// the bare spelling (that IS the construct's `.emp`→`.asm` purpose, and the
    /// name is program-unique), so an import must bind to the bare name for
    /// definition and use to meet at link.
    pub fn import_target(&self, module_id: &str, name: &str) -> String {
        if self.equ_exports.contains(&(module_id.to_string(), name.to_string())) {
            name.to_string()
        } else {
            canonical(module_id, name)
        }
    }
}

/// The `pub` names of a file (all item kinds that can be referenced across
/// modules: data/proc/offsets/const/equ/struct/enum/bitfield/newtype). Recurses
/// into `section {}` bodies so section-nested `pub` items are exported too — without
/// this a `pub data` inside a section is invisible cross-module (Task 0.5 fix).
pub fn exported_names(file: &ast::File) -> Vec<String> {
    let mut out = Vec::new();
    collect_exported(&file.items, &mut out);
    out
}

fn collect_exported(items: &[ast::Item], out: &mut Vec<String>) {
    for item in items {
        if let Some(name) = item_pub_name(item) {
            out.push(name);
        }
        // A `pub vars` REGION block exports every field/mark/alias LABEL it
        // defines (item #7a §3.3 — labels, not equates; cross-seam by bare name).
        if let ast::Item::Vars(v) = item {
            if v.public {
                region_field_names(v, out);
            }
        }
        if let ast::Item::Section(sec) = item {
            collect_exported(&sec.items, out);
        }
    }
}

/// The name of any `pub` top-level item, or None for private / `use` / `section`.
fn item_pub_name(item: &ast::Item) -> Option<String> {
    match item {
        ast::Item::Data(d) if d.public => Some(d.name.clone()),
        ast::Item::Proc(p) if p.public => Some(p.name.clone()),
        ast::Item::Offsets(o) if o.public => Some(o.name.clone()),
        ast::Item::Dispatch(d) if d.public => Some(d.name.clone()),
        // `pub script` exports the hidden resume table's BASE label — the
        // engine handle — exactly like `pub dispatch` (Plan 7 #9b, R9b.8). The
        // resume labels themselves stay hidden (`$…$` symbols the rename pass
        // skips); only the base name crosses modules.
        ast::Item::Script(s) if s.public => Some(s.name.clone()),
        ast::Item::Const(c) if c.public => Some(c.name.clone()),
        // A `pub comptime fn` is a comptime-only item (no bytes, no link symbol,
        // F3/tranche 7): it exports its NAME so a consumer's `use a.{template}`
        // resolves, and the resolver's ambient-injection path (`pub_comptime_name`
        // in resolve/mod.rs already covers ComptimeFn) makes the DECL visible so
        // the call evaluates. A non-`pub` comptime fn stays module-private.
        ast::Item::ComptimeFn(f) if f.public => Some(f.name.clone()),
        // A `pub context` exports its NAME so a consumer's `use m.{ctx}` resolves;
        // the DECL travels via the ambient-injection path (`pub_comptime_name`).
        ast::Item::Context(c) if c.public => Some(c.name.clone()),
        ast::Item::Struct(s) if s.public => Some(s.name.clone()),
        ast::Item::Enum(e) if e.public => Some(e.name.clone()),
        ast::Item::Bitfield(b) if b.public => Some(b.name.clone()),
        ast::Item::Newtype(n) if n.public => Some(n.name.clone()),
        // `pub vars` OVERLAY form (`vars Name: window { .. }`, D6.A8): a named,
        // exportable, comptime-only module item. The region form (`name: None`)
        // exports its FIELD/MARK/ALIAS labels instead — handled specially in
        // `collect_exported` (an item can export many names).
        ast::Item::Vars(v) if v.public && v.name.is_some() => v.name.clone(),
        // A `pub equ` exports its NAME so a consumer's `use m.{NAME}` resolves.
        // Unlike every sibling above, the imported name binds to the BARE link
        // symbol (`ExportIndex::import_target`): a `pub equ`'s definition is
        // never module-qualified, because a still-`.asm` consumer names it
        // unqualified across the link seam. A non-`pub` equ stays module-private
        // and carries the owner-mangled `$module$NAME` link name instead.
        ast::Item::Equ(e) if e.is_pub => Some(e.name.clone()),
        _ => None,
    }
}

/// Collect the field/mark/alias LABEL names a region-form `vars` block defines,
/// recursing into conditional groups (item #7a §2.2). These are ordinary
/// link-visible labels: a `pub vars` block exports them cross-module / cross-
/// seam (§3.3), and every region-form block's names are module-local defined
/// names (rename map). Overlay-form blocks (`v.name.is_some()`) contribute
/// nothing here.
pub(crate) fn region_field_names(v: &ast::VarsDecl, out: &mut Vec<String>) {
    if v.name.is_some() {
        return;
    }
    collect_region_field_names(&v.region_body, out);
}

fn collect_region_field_names(body: &[ast::RegionField], out: &mut Vec<String>) {
    for f in body {
        match f {
            ast::RegionField::Typed(t) => out.push(t.name.clone()),
            ast::RegionField::Mark { name, .. } => out.push(name.clone()),
            ast::RegionField::Alias { name, .. } => out.push(name.clone()),
            ast::RegionField::Pad { .. } => {}
            ast::RegionField::Group { then_body, else_body, .. } => {
                collect_region_field_names(then_body, out);
                collect_region_field_names(else_body, out);
            }
        }
    }
}

/// Every name a file DEFINES (pub or private), for own-canonical mapping.
/// Recurses into `section {}` bodies so section-nested items enter the rename map
/// and `report_unresolved` accepts references to them (Task 0.5 fix). Kept
/// consistent with `exported_names`: same recursion, same item kinds.
fn defined_names(file: &ast::File) -> Vec<String> {
    let mut out = Vec::new();
    collect_defined(&file.items, &mut out);
    out
}

fn collect_defined(items: &[ast::Item], out: &mut Vec<String>) {
    for it in items {
        match it {
            ast::Item::Data(d) => out.push(d.name.clone()),
            ast::Item::Proc(p) => out.push(p.name.clone()),
            ast::Item::Offsets(o) => out.push(o.name.clone()),
            ast::Item::Dispatch(d) => out.push(d.name.clone()),
            // A script's name labels its hidden resume table, and the table's
            // OWN rows self-reference it (`dc.w resume_k - name`), so the name
            // must enter the own-defined map — without this arm ANY script
            // (even unreferenced) fails `report_unresolved` under the program
            // path (Plan 7 #9b review fold-in).
            ast::Item::Script(s) => out.push(s.name.clone()),
            ast::Item::Const(c) => out.push(c.name.clone()),
            // A comptime fn (pub or private) is an own-defined name — so a
            // same-module call resolves and `report_unresolved` accepts it, and a
            // consumer's imported call renames to the canonical `a.template` (F3).
            ast::Item::ComptimeFn(f) => out.push(f.name.clone()),
            ast::Item::Struct(s) => out.push(s.name.clone()),
            ast::Item::Enum(e) => out.push(e.name.clone()),
            ast::Item::Bitfield(b) => out.push(b.name.clone()),
            ast::Item::Newtype(n) => out.push(n.name.clone()),
            // Overlay-form `vars` (D6.A8): a named module item. Region form
            // (`name: None`) defines its field/mark/alias LABEL names (item #7a),
            // which enter the rename map like ordinary labels.
            ast::Item::Vars(v) => {
                if let Some(name) = &v.name {
                    out.push(name.clone());
                } else {
                    region_field_names(v, out);
                }
            }
            ast::Item::Section(sec) => collect_defined(&sec.items, out),
            _ => {}
        }
    }
}

/// One module's short-name → canonical-symbol resolution table.
pub struct ResolveEnv<'a> {
    map: HashMap<String, String>,
    index: &'a ExportIndex,
}

impl<'a> ResolveEnv<'a> {
    /// Build the env for `module_id`. Precedence when the same short name is
    /// reachable multiple ways: LOCAL > explicit `use` > prelude.
    /// `prelude` is the optional prelude module id/file (Task 3 passes Some).
    pub fn build(
        module_id: &str,
        file: &ast::File,
        index: &'a ExportIndex,
        prelude: Option<(&str, &ast::File)>,
    ) -> (ResolveEnv<'a>, Vec<Diagnostic>) {
        let mut map = HashMap::new();
        let mut diags = Vec::new();

        // Resolve explicit `use` imports into their OWN map first, so the
        // collision check is scoped to genuine EQUAL-precedence conflicts
        // (use-vs-use) — not spurious use-shadows-prelude ones. Recurses one
        // level into `section {}` bodies so a section-nested `use` is honored
        // too (sections do not nest further — Task 1 rejects that at parse time).
        let mut use_map: HashMap<String, String> = HashMap::new();
        collect_uses(&file.items, module_id, index, &mut use_map, &mut diags);

        // Compose the final map in precedence order (later overlays win silently):
        //   prelude pub names (lowest) < explicit `use` < own definitions (highest).
        if let Some((pid, pfile)) = prelude {
            if pid != module_id {
                for name in exported_names(pfile) {
                    let c = index.import_target(pid, &name);
                    map.insert(name, c);
                }
            }
        }
        for (name, canon) in use_map {
            map.insert(name, canon);
        }
        for name in defined_names(file) {
            let c = canonical(module_id, &name);
            map.insert(name, c);
        }
        // C1 item 4: equ names enter the map so an in-module operand reference
        // to an equ (a symbol reference — an equ used as an absolute address is
        // NOT inlined) resolves in `report_unresolved` and the rename pass
        // rewrites the reference and the equ's own `EquSym` in lockstep. Inserted
        // LAST so a module-local equ wins over an imported/prelude name of the
        // same spelling. A `pub equ` maps to ITSELF — it keeps its plain
        // cross-seam name; a non-`pub` equ maps to its owner-mangled
        // `$module$NAME`, so two modules' same-named private equs never collide.
        for name in pub_equ_names(file) {
            map.insert(name.clone(), name);
        }
        for name in nonpub_equ_names(file) {
            let m = mangled_equ(module_id, &name);
            map.insert(name, m);
        }
        (ResolveEnv { map, index }, diags)
    }

    /// Resolve a short name to its canonical cross-module symbol, if in scope.
    pub fn resolve(&self, name: &str) -> Option<String> {
        self.map.get(name).cloned()
    }

    /// The full short-name → canonical-symbol map, for the rename pass. Borrowed
    /// (not consumed) so the driver can still call [`resolve`](Self::resolve) /
    /// [`suggest_use`](Self::suggest_use) for unresolved-reference diagnostics.
    pub fn rename_map(&self) -> &std::collections::HashMap<String, String> {
        &self.map
    }

    /// If `name` is exported by exactly one other module, produce the fix-it text.
    pub fn suggest_use(&self, name: &str) -> Option<String> {
        let owners = self.index.by_name.get(name)?;
        match owners.as_slice() {
            [only] => Some(format!("add `use {only}.{{{name}}}`")),
            _ => None, // ambiguous or none → generic error, no single fix-it
        }
    }
}

/// Walk `items` calling `resolve_use` on every `Item::Use`, recursing one level
/// into `section {}` bodies so a section-nested `use` is honored too (mirrors
/// `collect_exported`/`collect_defined`'s recursion shape).
fn collect_uses(
    items: &[ast::Item],
    module_id: &str,
    index: &ExportIndex,
    map: &mut HashMap<String, String>,
    diags: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            ast::Item::Use(u) => resolve_use(module_id, u, index, map, diags),
            ast::Item::Section(sec) => collect_uses(&sec.items, module_id, index, map, diags),
            _ => {}
        }
    }
}

fn resolve_use(
    module_id: &str,
    u: &ast::UseDecl,
    index: &ExportIndex,
    map: &mut HashMap<String, String>,
    diags: &mut Vec<Diagnostic>,
) {
    let base = u.base.segments.join(".");
    match &u.names {
        ast::UseNames::List(names) => {
            for n in names {
                if !index.is_exported(&base, n) {
                    diags.push(Diagnostic {
                        level: Level::Error,
                        message: format!("module `{base}` has no `pub` name `{n}`"),
                        primary: u.span,
                    });
                    continue;
                }
                let target = index.import_target(&base, n);
                if let Some(prev) = map.insert(n.clone(), target.clone()) {
                    if prev != target {
                        diags.push(Diagnostic {
                            level: Level::Error,
                            message: format!(
                                "`{n}` imported from `{base}` collides with `{prev}` (name already in scope)"
                            ),
                            primary: u.span,
                        });
                    }
                }
            }
        }
        ast::UseNames::Glob => {
            // Re-scan the export index for everything under `base`. A glob whose
            // base matches NO module in the index is almost always a typo — flag
            // it (mirrors the List arm's "no such name" feedback) rather than
            // silently importing nothing.
            let mut matched_any = false;
            for (name, owners) in index.by_name.iter() {
                if owners.iter().any(|o| o == &base) {
                    matched_any = true;
                    let target = index.import_target(&base, name);
                    if let Some(prev) = map.insert(name.clone(), target.clone()) {
                        if prev != target {
                            diags.push(Diagnostic {
                                level: Level::Error,
                                message: format!(
                                    "glob `use {base}.*` brings `{name}`, which collides with `{prev}`"
                                ),
                                primary: u.span,
                            });
                        }
                    }
                }
            }
            if !matched_any {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("glob `use {base}.*` matches no module with `pub` names"),
                    primary: u.span,
                });
            }
        }
        ast::UseNames::Whole => {
            // `use base` (whole) binds NO short names: only `use base.{…}` (List)
            // and `use base.*` (Glob) put names in scope. A whole-module `use` is
            // therefore a no-op for name resolution — a later `base.Name` reference
            // won't resolve through it — so warn rather than let that be mysterious.
            let _ = module_id;
            diags.push(Diagnostic {
                level: Level::Warning,
                message: format!(
                    "[import.no-names] whole-module `use {base}` imports no names — use `use {base}.{{…}}` or `use {base}.*` to bring names into scope"
                ),
                primary: u.span,
            });
        }
    }
}
