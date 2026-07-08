# Module Resolution + Placement + Prelude Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `.emp` from a single-file compiler into a multi-module assembler — gather sibling `.emp` files, resolve `use`/prelude names across modules, place each module's items into named sections, and link them into one image, so `examples/pitcher_plant.emp` compiles end-to-end.

**Architecture:** Approach A — all new logic in `sigil-frontend-emp`; `sigil-link` and the AS front-end stay byte-for-byte unchanged. A new `resolve/` driver gathers modules, builds a per-module name→canonical-symbol map, lowers each module (unchanged evaluator, plus ambient comptime-def injection for imported types), then runs a **post-lowering rename pass** over the IR (mangling top-level labels + rewriting every fixup target) so the existing flat-symbol-table `link` resolves cross-module references. Single-file `sigil emp <file>` keeps its current path untouched (identity mangling); the new driver activates only for multi-module builds.

**Tech Stack:** Rust workspace (`crates/sigil-frontend-emp`, `sigil-ir`, `sigil-link`, `sigil-cli`); `cargo test` / `cargo clippy`; TOML map file (`sigil.map.toml`).

**Ground truth (verified 2026-07-07, code is authoritative):**
- `link(sections: &[Section], stubs: &SymbolTable)` (`sigil-link/src/lib.rs:50`) builds ONE flat symbol table across all sections, keyed by bare `Label.name`. Multi-module = concat `Vec<Section>` + one `link`. No linker change.
- Evaluator env: `Evaluator<'a>` (`eval/mod.rs:72`) holds `consts/enums/fns/structs/bitfields/newtypes/datas/offsets: HashMap<&'a str, &'a ast::…Decl>`, populated by `index_items` (`eval/mod.rs:263`) via `with_file` (`eval/mod.rs:252`).
- Top-level labels are minted with the RAW item name: `builder.define_label(&decl.name)` (data `lower/mod.rs:257`, offsets `:281`), `builder.define_label(&proc.name)` (`lower/proc.rs:80`). Proc-LOCAL labels are already mangled to `$Proc$name` / `$asm{k}$name` by `lower/hygiene.rs` — **do not disturb**.
- Fixup targets are `sigil_ir::expr::Expr` (`Expr::Sym(String)`, `Expr::Binary{op,lhs,rhs}`). Fixups live in FOUR `Fragment` variants (`sigil-ir/src/lib.rs:55`): `Data(DataFragment{fixups})`, `JmpJsrSym{target}`, `RelaxAbsSym{target, short: RelaxCandidate, long: RelaxCandidate}` (each `RelaxCandidate` holds its own `Fixup`), `Org{target: u32}` (already-resolved, no symbol).
- Undefined label refs do NOT error in the evaluator — they pass through as `Expr::Sym` and only fail at link (`eval/asm.rs:302` via `hygiene.rs:111 resolve_ref` passthrough). So cross-module label refs "just work" as fixups once names are consistent.
- `Module { sections: Vec<Section> }`; `Section { name, cpu, vma_base, lma, labels: Vec<Label{name,offset}>, fragments }` (`sigil-ir/src/lib.rs:235,115,35`).
- Live map file: `sigil.map.toml` → `sigil_link::load_map` → `MemoryMap{regions: Vec<Region{name,lma_base,size,kind,vma_base}>, fill}`, consumed by `sigil_link::emit_rom` for region-bounds validation.

---

## File Structure

**New (in `crates/sigil-frontend-emp/src/`):**
- `resolve/mod.rs` — driver: `build_program(...)` orchestrates gather → env → lower → rename → concat `Vec<Section>`.
- `resolve/manifest.rs` — `Manifest`: scan a root dir, parse each `.emp`, index by `module` header; path/dir-disagreement lint.
- `resolve/imports.rs` — `ResolveEnv` per module (own defs + `use` + prelude, precedence + glob-collision error), `canonical()` mangling, "add `use`" fix-it, export index.
- `resolve/rename.rs` — post-lowering IR rewrite: rename top-level `Label.name` + rewrite every fixup target `Expr` across all fragment variants.

**Modified:**
- `crates/sigil-frontend-emp/src/lib.rs` — `pub mod resolve;`
- `crates/sigil-frontend-emp/src/eval/mod.rs` — `with_file_with_ambient` + ambient extend.
- `crates/sigil-frontend-emp/src/layout.rs` + `src/lower/mod.rs` + `src/lower/proc.rs` — thread an `Ambient` borrow to the eval construction sites; consume `ModuleDecl.in_section` for placement.
- `crates/sigil-cli/src/main.rs` — new multi-module entry + `--root` / `--prelude` flags; single-file path unchanged.

**New assets / tests:**
- `examples/prelude.emp`, `examples/objindex.emp`, `examples/objmath.emp` (corpus siblings).
- `crates/sigil-frontend-emp/tests/resolve_manifest.rs`, `resolve_imports.rs`, `resolve_rename.rs`.
- `crates/sigil-cli/tests/module_resolution.rs` (end-to-end; mirrors existing `crate_graph.rs`).

**Green gate before EVERY commit:** `cargo test --workspace` && `cargo clippy --workspace --all-targets -- -D warnings`. The s4.bin harness (`m1d_rom`/`m1d_debug_rom`) must stay green.

---

## Task 0: Readiness spike (record confirmations + lock the two structural choices)

No production code. Produce a short `docs/superpowers/notes/2026-07-07-item4-t0-spike.md` recording the decisions below, verified against the tree, so later tasks reference concrete seams. Most facts are already confirmed in the header; T0 exists to lock the two open structural choices and re-verify nothing drifted.

**Files:**
- Create: `docs/superpowers/notes/2026-07-07-item4-t0-spike.md`

- [ ] **Step 1: Re-verify the four fixup-bearing fragment variants and the label-minting sites still match the header.**

Run: `grep -n "define_label" crates/sigil-frontend-emp/src/lower/mod.rs crates/sigil-frontend-emp/src/lower/proc.rs`
Run: `grep -n "JmpJsrSym\|RelaxAbsSym\|struct RelaxCandidate\|pub fixup" crates/sigil-ir/src/lib.rs`
Expected: label-minting at `lower/mod.rs` (data/offsets) + `lower/proc.rs` (proc); `RelaxCandidate` has a `fixup: Fixup` field. Record the exact lines.

- [ ] **Step 2: Confirm `Evaluator` fields are all the comptime-def maps we must inject, and `with_file` is the single construction choke point.**

Run: `grep -rn "Evaluator::with_file\|Evaluator::new()" crates/sigil-frontend-emp/src`
Expected: `with_file` called from `layout.rs` (`eval_data_with_root`, `eval_offsets_with_root`) and `proc` lowering. List every call site — these are where `Ambient` must be threaded.

- [ ] **Step 3: Record the two locked decisions.**

Write into the note:
1. **Multi-module driver is a NEW path.** `sigil emp <file>` with no `--root` keeps calling today's `compile_emp` unchanged (identity naming; all existing single-file tests unaffected). Mangling/rename activates only inside `resolve::build_program`.
2. **Ambient comptime defs are injected via a new `Evaluator::with_file_with_ambient(file, ambient)`**, threaded from the driver through `lower/mod.rs` → `layout.rs`/`proc.rs`. Label references need no evaluator change (they pass through as fixups; the rename pass fixes their names post-lowering).

- [ ] **Step 4: Commit.**

```bash
git add docs/superpowers/notes/2026-07-07-item4-t0-spike.md
git commit -m "docs(item4): T0 readiness spike — locked driver + ambient-injection seams"
```

---

## Task 1: Manifest — scan, parse, index modules by header

Build the module index the whole milestone reads from. A `Manifest` owns every parsed `ast::File` (the lifetime anchor for later `&'a` borrows) and maps `module` dotted-path → index. Emits the §3.1 path/dir-disagreement lint (a `Diagnostic` at `Level::Warning`, never an error).

**Files:**
- Create: `crates/sigil-frontend-emp/src/resolve/mod.rs`
- Create: `crates/sigil-frontend-emp/src/resolve/manifest.rs`
- Modify: `crates/sigil-frontend-emp/src/lib.rs` (add `pub mod resolve;`)
- Test: `crates/sigil-frontend-emp/tests/resolve_manifest.rs`

- [ ] **Step 1: Write the failing test.**

```rust
// crates/sigil-frontend-emp/tests/resolve_manifest.rs
use sigil_frontend_emp::resolve::manifest::Manifest;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

#[test]
fn indexes_modules_by_header_and_lints_path_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "badniks/pitcher_plant.emp", "module badniks.pitcher_plant\n");
    write(root, "engine/helpers.emp", "module engine.helpers\n");
    // Header says one thing, directory says another → LINT, not error.
    write(root, "misplaced/here.emp", "module engine.objects.sst\n");

    let (manifest, diags) = Manifest::scan(root);
    assert!(manifest.by_id.contains_key("badniks.pitcher_plant"));
    assert!(manifest.by_id.contains_key("engine.helpers"));
    assert!(manifest.by_id.contains_key("engine.objects.sst"));
    // The mismatch is a warning, and NOTHING is an error.
    assert!(diags.iter().all(|d| d.level != sigil_span::Level::Error));
    assert!(diags.iter().any(|d| d.level == sigil_span::Level::Warning
        && d.message.contains("engine.objects.sst")));
}
```

- [ ] **Step 2: Run it to confirm it fails.**

Run: `cargo test -p sigil-frontend-emp --test resolve_manifest -- --nocapture`
Expected: FAIL to compile — `resolve` module does not exist.

- [ ] **Step 3: Implement `Manifest`.**

```rust
// crates/sigil-frontend-emp/src/resolve/mod.rs
//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod manifest;
```

```rust
// crates/sigil-frontend-emp/src/resolve/manifest.rs
use crate::ast;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One parsed `.emp` module: its declared id, its AST, and its source path.
pub struct ParsedModule {
    pub id: String,
    pub file: ast::File,
    pub path: PathBuf,
}

/// Every `.emp` module discovered under a scan root, indexed by module id.
/// Owns the ASTs — the lifetime anchor for later `&'a ast` borrows.
pub struct Manifest {
    pub modules: Vec<ParsedModule>,
    pub by_id: HashMap<String, usize>,
}

impl Manifest {
    /// Recursively scan `root` for `*.emp`, parse each, index by its `module`
    /// header. Path/dir disagreement (§3.1) is a WARNING, never an error.
    pub fn scan(root: &Path) -> (Manifest, Vec<Diagnostic>) {
        let mut modules = Vec::new();
        let mut by_id = HashMap::new();
        let mut diags = Vec::new();
        let mut files = Vec::new();
        collect_emp(root, &mut files);
        files.sort(); // deterministic order
        for path in files {
            let src = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    diags.push(io_diag(&path, &e));
                    continue;
                }
            };
            let (file, mut pdiags) = crate::parse_str(&src);
            diags.append(&mut pdiags);
            let id = file.module.path.segments.join(".");
            // §3.1 lint: does the id agree with the on-disk location?
            if let Some(expected) = expected_id_from_path(root, &path) {
                if expected != id {
                    diags.push(Diagnostic {
                        level: Level::Warning,
                        message: format!(
                            "module `{id}` is at `{}`, which suggests id `{expected}` \
                             (rename the file/dir or the header to agree)",
                            path.strip_prefix(root).unwrap_or(&path).display()
                        ),
                        span: file.module.span,
                    });
                }
            }
            if let Some(prev) = by_id.insert(id.clone(), modules.len()) {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("module `{id}` declared twice (also at module #{prev})"),
                    span: file.module.span,
                });
            }
            modules.push(ParsedModule { id, file, path });
        }
        (Manifest { modules, by_id }, diags)
    }
}

fn collect_emp(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_emp(&p, out);
        } else if p.extension().is_some_and(|x| x == "emp") {
            out.push(p);
        }
    }
}

/// The module id the on-disk path implies: dir segments from root + file stem.
fn expected_id_from_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let stem = rel.file_stem()?.to_str()?;
    let mut segs: Vec<String> = rel
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .filter_map(|c| c.as_os_str().to_str().map(String::from))
        .collect();
    segs.push(stem.to_string());
    Some(segs.join("."))
}

fn io_diag(path: &Path, e: &std::io::Error) -> Diagnostic {
    Diagnostic { level: Level::Error, message: format!("cannot read `{}`: {e}", path.display()), span: Span::default() }
}
```

Add to `crates/sigil-frontend-emp/src/lib.rs` after the existing `pub mod` lines:
```rust
pub mod resolve;
```

Add `tempfile` to `[dev-dependencies]` in `crates/sigil-frontend-emp/Cargo.toml` if absent (check first: `grep tempfile crates/sigil-frontend-emp/Cargo.toml`).

- [ ] **Step 4: Run the test to confirm it passes.**

Run: `cargo test -p sigil-frontend-emp --test resolve_manifest -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Green gate + commit.**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/resolve crates/sigil-frontend-emp/src/lib.rs \
        crates/sigil-frontend-emp/tests/resolve_manifest.rs crates/sigil-frontend-emp/Cargo.toml
git commit -m "feat(resolve): module manifest — scan/parse/index by header + path lint (§3.1)"
```

---

## Task 2: `use` resolution + canonical mangling + post-lowering rename

The core. Build, per module, a `ResolveEnv` mapping every short name it can reference to a canonical symbol (own defs → own canonical; `use` imports → defining module's canonical; unresolved-but-exported-elsewhere → "add `use`" fix-it). Then a post-lowering `rename` pass rewrites top-level `Label.name` and every fixup `Expr::Sym` in all four fragment variants. This makes a multi-module program of procs/data that reference each other's labels link correctly. (Comptime type imports are Task 3.)

**Canonical scheme:** `canonical(module_id, name) = format!("{module_id}.{name}")` — collision-proof because item names contain no dots, so the (module, item) split of any canonical string is unique.

**Files:**
- Create: `crates/sigil-frontend-emp/src/resolve/imports.rs`
- Create: `crates/sigil-frontend-emp/src/resolve/rename.rs`
- Modify: `crates/sigil-frontend-emp/src/resolve/mod.rs` (add `build_program`)
- Test: `crates/sigil-frontend-emp/tests/resolve_imports.rs`, `crates/sigil-frontend-emp/tests/resolve_rename.rs`, `crates/sigil-cli/tests/module_resolution.rs`

### 2a — Export index + `ResolveEnv` (imports.rs)

- [ ] **Step 1: Write the failing test.**

```rust
// crates/sigil-frontend-emp/tests/resolve_imports.rs
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::imports::{canonical, ExportIndex, ResolveEnv};

#[test]
fn canonical_is_module_qualified() {
    assert_eq!(canonical("badniks.pitcher_plant", "init"), "badniks.pitcher_plant.init");
}

#[test]
fn use_list_resolves_to_defining_module_canonical() {
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str(
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\nproc init (a0: *u8) {}\n",
    );
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (env, diags) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    assert!(diags.is_empty());
    // Own private proc → own canonical.
    assert_eq!(env.resolve("init"), Some("badniks.plant.init".to_string()));
    // Imported name → defining module's canonical.
    assert_eq!(env.resolve("Draw_Sprite"), Some("engine.helpers.Draw_Sprite".to_string()));
}

#[test]
fn unimported_but_exported_elsewhere_yields_add_use_fixit() {
    let (helpers, _) = parse_str("module engine.helpers\npub proc Draw_Sprite (a0: *u8) {}\n");
    let (obj, _) = parse_str("module badniks.plant\nproc init (a0: *u8) {}\n"); // NO use
    let idx = ExportIndex::build(&[("engine.helpers", &helpers), ("badniks.plant", &obj)]);
    let (env, _) = ResolveEnv::build("badniks.plant", &obj, &idx, None);
    // Not directly resolvable, but the env can SUGGEST the missing use.
    assert_eq!(env.resolve("Draw_Sprite"), None);
    assert_eq!(
        env.suggest_use("Draw_Sprite"),
        Some("add `use engine.helpers.{Draw_Sprite}`".to_string())
    );
}
```

- [ ] **Step 2: Run it to confirm it fails.**

Run: `cargo test -p sigil-frontend-emp --test resolve_imports`
Expected: FAIL to compile — `imports` module missing.

- [ ] **Step 3: Implement `imports.rs`.**

```rust
// crates/sigil-frontend-emp/src/resolve/imports.rs
use crate::ast;
use sigil_span::{Diagnostic, Level};
use std::collections::HashMap;

pub fn canonical(module_id: &str, name: &str) -> String {
    format!("{module_id}.{name}")
}

/// Every module's `pub` top-level LABEL/VALUE names → its module id.
/// (Types/consts/fns are handled by the ambient path in Task 3, but they are
/// indexed here too so `suggest_use` can point at any exported name.)
pub struct ExportIndex {
    /// name → list of (module_id) that export it (list to detect ambiguity).
    by_name: HashMap<String, Vec<String>>,
    /// (module_id, name) exported? — for qualified-reference validation.
    exported: std::collections::HashSet<(String, String)>,
}

impl ExportIndex {
    pub fn build(modules: &[(&str, &ast::File)]) -> Self {
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        let mut exported = std::collections::HashSet::new();
        for (id, file) in modules {
            for name in exported_names(file) {
                by_name.entry(name.clone()).or_default().push((*id).to_string());
                exported.insert(((*id).to_string(), name));
            }
        }
        ExportIndex { by_name, exported }
    }
    pub fn is_exported(&self, module_id: &str, name: &str) -> bool {
        self.exported.contains(&(module_id.to_string(), name.to_string()))
    }
}

/// Iterate the `pub` top-level names of a file (all item kinds that can be
/// referenced across modules: data/proc/offsets/const/struct/enum/bitfield/newtype).
pub fn exported_names(file: &ast::File) -> Vec<String> {
    file.items.iter().filter_map(item_pub_name).collect()
}

/// The (is_pub, name) of any top-level item, or None for `use`/`section`.
fn item_pub_name(item: &ast::Item) -> Option<String> {
    match item {
        ast::Item::Data(d) if d.public => Some(d.name.clone()),
        ast::Item::Proc(p) if p.public => Some(p.name.clone()),
        ast::Item::Offsets(o) if o.public => Some(o.name.clone()),
        ast::Item::Const(c) if c.public => Some(c.name.clone()),
        ast::Item::Struct(s) if s.public => Some(s.name.clone()),
        ast::Item::Enum(e) if e.public => Some(e.name.clone()),
        ast::Item::Bitfield(b) if b.public => Some(b.name.clone()),
        ast::Item::Newtype(n) if n.public => Some(n.name.clone()),
        _ => None,
    }
}

/// Every top-level name a file DEFINES (pub or private), for own-canonical mapping.
fn defined_names(file: &ast::File) -> Vec<String> {
    file.items.iter().filter_map(|it| match it {
        ast::Item::Data(d) => Some(d.name.clone()),
        ast::Item::Proc(p) => Some(p.name.clone()),
        ast::Item::Offsets(o) => Some(o.name.clone()),
        ast::Item::Const(c) => Some(c.name.clone()),
        ast::Item::Struct(s) => Some(s.name.clone()),
        ast::Item::Enum(e) => Some(e.name.clone()),
        ast::Item::Bitfield(b) => Some(b.name.clone()),
        ast::Item::Newtype(n) => Some(n.name.clone()),
        _ => None,
    }).collect()
}

/// One module's short-name → canonical-symbol resolution table.
pub struct ResolveEnv<'a> {
    map: HashMap<String, String>,
    index: &'a ExportIndex,
}

impl<'a> ResolveEnv<'a> {
    /// Build the env for `module_id`. Precedence when the same short name is
    /// reachable multiple ways: LOCAL > explicit `use` > prelude.
    /// `prelude` is the optional prelude module id (Task 3 passes Some).
    pub fn build(
        module_id: &str,
        file: &ast::File,
        index: &'a ExportIndex,
        prelude: Option<(&str, &ast::File)>,
    ) -> (ResolveEnv<'a>, Vec<Diagnostic>) {
        let mut map = HashMap::new();
        let mut diags = Vec::new();

        // Lowest precedence: prelude pub names.
        if let Some((pid, pfile)) = prelude {
            if pid != module_id {
                for name in exported_names(pfile) {
                    map.insert(name.clone(), canonical(pid, &name));
                }
            }
        }
        // Middle: explicit `use`.
        for item in &file.items {
            if let ast::Item::Use(u) = item {
                resolve_use(module_id, u, index, &mut map, &mut diags);
            }
        }
        // Highest: own definitions (overwrite anything imported).
        for name in defined_names(file) {
            map.insert(name.clone(), canonical(module_id, &name));
        }
        (ResolveEnv { map, index }, diags)
    }

    pub fn resolve(&self, name: &str) -> Option<String> {
        self.map.get(name).cloned()
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
                        span: u.span,
                    });
                    continue;
                }
                if let Some(prev) = map.insert(n.clone(), canonical(&base, n)) {
                    if prev != canonical(&base, n) {
                        diags.push(Diagnostic {
                            level: Level::Error,
                            message: format!(
                                "`{n}` imported from `{base}` collides with `{prev}` (name already in scope)"
                            ),
                            span: u.span,
                        });
                    }
                }
            }
        }
        ast::UseNames::Glob => {
            // Re-scan the export index for everything under `base`.
            for (name, owners) in index.by_name.iter() {
                if owners.iter().any(|o| o == &base) {
                    if let Some(prev) = map.insert(name.clone(), canonical(&base, name)) {
                        if prev != canonical(&base, name) {
                            diags.push(Diagnostic {
                                level: Level::Error,
                                message: format!(
                                    "glob `use {base}.*` brings `{name}`, which collides with `{prev}`"
                                ),
                                span: u.span,
                            });
                        }
                    }
                }
            }
        }
        ast::UseNames::Whole => {
            let _ = module_id; // `use base` (whole) — qualified refs handled at rename time
        }
    }
}
```

Add `pub mod imports;` to `resolve/mod.rs`.

> **Note on `pub` field names:** the header lists ten `pub public: bool` fields in `ast.rs`. Verify each item variant's public-flag field is named `public` (`grep -n "pub public" crates/sigil-frontend-emp/src/ast.rs` — confirmed at lines 102,119,152,180,212,236,265,280,299,357). If any item kind lacks a `public` field, adjust `item_pub_name`/`defined_names` accordingly.

- [ ] **Step 4: Run to confirm pass.**

Run: `cargo test -p sigil-frontend-emp --test resolve_imports`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/sigil-frontend-emp/src/resolve/imports.rs crates/sigil-frontend-emp/src/resolve/mod.rs \
        crates/sigil-frontend-emp/tests/resolve_imports.rs
git commit -m "feat(resolve): ExportIndex + ResolveEnv — use resolution, precedence, add-use fix-it"
```

### 2b — Post-lowering rename pass (rename.rs)

- [ ] **Step 1: Write the failing test.**

```rust
// crates/sigil-frontend-emp/tests/resolve_rename.rs
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::rename::rename_module;
use sigil_ir::backend::Cpu;
use std::collections::HashMap;

#[test]
fn renames_labels_and_fixup_targets() {
    // A module whose data points at a proc label; rename both to canonicals.
    let (file, d) = parse_str(
        "module m.a\ndata Def: [*u8; 1] = [init]\nproc init (a0: *u8) {}\n",
    );
    assert!(d.iter().all(|x| x.level != sigil_span::Level::Error), "{d:?}");
    let (mut module, _) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });

    let mut map = HashMap::new();
    map.insert("Def".to_string(), "m.a.Def".to_string());
    map.insert("init".to_string(), "m.a.init".to_string());
    rename_module(&mut module, &map);

    // The proc's entry label is now canonical.
    let has_canon_label = module.sections.iter()
        .flat_map(|s| &s.labels).any(|l| l.name == "m.a.init");
    assert!(has_canon_label, "expected renamed label m.a.init");
    // The data fixup target is now canonical (no bare `init` remains).
    let bare_init_target = module.sections.iter().flat_map(|s| &s.fragments)
        .any(|f| fixup_targets(f).iter().any(|t| t == "init"));
    assert!(!bare_init_target, "bare `init` fixup target should have been renamed");
}

// Test helper: collect every symbol name appearing in a fragment's fixup targets.
fn fixup_targets(f: &sigil_ir::Fragment) -> Vec<String> {
    let mut out = Vec::new();
    sigil_frontend_emp::resolve::rename::collect_target_syms(f, &mut out);
    out
}
```

- [ ] **Step 2: Run to confirm it fails.**

Run: `cargo test -p sigil-frontend-emp --test resolve_rename`
Expected: FAIL to compile — `rename` module missing.

- [ ] **Step 3: Implement `rename.rs`. Must cover ALL four fixup-bearing fragment variants.**

```rust
// crates/sigil-frontend-emp/src/resolve/rename.rs
//! Post-lowering IR rewrite: rename top-level labels and every fixup target
//! symbol to its canonical cross-module name, so the flat-symbol-table linker
//! resolves cross-module references. Proc-local `$Proc$name` / `$asm{k}$name`
//! symbols are NOT in the map (they contain `$`), so they pass through
//! unchanged — local hygiene is preserved.
use sigil_ir::expr::Expr;
use sigil_ir::{Fragment, Module};
use std::collections::HashMap;

/// Rewrite `module` in place: rename `Label.name` and every fixup target `Expr`
/// per `map` (short name → canonical). Names absent from `map` are left as-is.
pub fn rename_module(module: &mut Module, map: &HashMap<String, String>) {
    for sec in &mut module.sections {
        for label in &mut sec.labels {
            if let Some(canon) = map.get(&label.name) {
                label.name = canon.clone();
            }
        }
        for frag in &mut sec.fragments {
            rename_fragment(frag, map);
        }
    }
}

fn rename_fragment(frag: &mut Fragment, map: &HashMap<String, String>) {
    match frag {
        Fragment::Data(df) => {
            for fx in &mut df.fixups {
                rewrite_expr(&mut fx.target, map);
            }
        }
        Fragment::JmpJsrSym { target, .. } => rewrite_expr(target, map),
        Fragment::RelaxAbsSym { target, short, long, .. } => {
            rewrite_expr(target, map);
            rewrite_expr(&mut short.fixup.target, map);
            rewrite_expr(&mut long.fixup.target, map);
        }
        Fragment::Fill { .. } | Fragment::Reserve { .. } | Fragment::Org { .. } => {}
    }
}

fn rewrite_expr(e: &mut Expr, map: &HashMap<String, String>) {
    match e {
        Expr::Sym(name) => {
            if let Some(canon) = map.get(name) {
                *name = canon.clone();
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            rewrite_expr(lhs, map);
            rewrite_expr(rhs, map);
        }
        _ => {}
    }
}

/// Test/diagnostic helper: collect every symbol name in a fragment's fixup targets.
pub fn collect_target_syms(frag: &Fragment, out: &mut Vec<String>) {
    let mut visit = |e: &Expr, out: &mut Vec<String>| collect_expr(e, out);
    match frag {
        Fragment::Data(df) => for fx in &df.fixups { visit(&fx.target, out) },
        Fragment::JmpJsrSym { target, .. } => visit(target, out),
        Fragment::RelaxAbsSym { target, short, long, .. } => {
            visit(target, out);
            collect_expr(&short.fixup.target, out);
            collect_expr(&long.fixup.target, out);
        }
        _ => {}
    }
}

fn collect_expr(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Sym(n) => out.push(n.clone()),
        Expr::Binary { lhs, rhs, .. } => { collect_expr(lhs, out); collect_expr(rhs, out); }
        _ => {}
    }
}
```

> **Verify the `Expr` variant set** before finalizing: `grep -n "pub enum Expr" -A 30 crates/sigil-ir/src/expr.rs`. If `Expr` has more compound variants (e.g. `Unary`, `Paren`), extend `rewrite_expr`/`collect_expr` to recurse into them. If `RelaxCandidate`'s fixup field is not named `fixup`, adjust (confirmed via T0 Step 1).

Add `pub mod rename;` to `resolve/mod.rs`.

- [ ] **Step 4: Run to confirm pass.**

Run: `cargo test -p sigil-frontend-emp --test resolve_rename`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add crates/sigil-frontend-emp/src/resolve/rename.rs crates/sigil-frontend-emp/src/resolve/mod.rs \
        crates/sigil-frontend-emp/tests/resolve_rename.rs
git commit -m "feat(resolve): post-lowering rename — labels + fixup targets across all fragment variants"
```

### 2c — Driver `build_program` + end-to-end 2-module link

- [ ] **Step 1: Write the failing end-to-end test.**

```rust
// crates/sigil-cli/tests/module_resolution.rs
// Two modules that reference each other's labels must link into one image.
use std::process::Command;

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

#[test]
fn two_modules_cross_reference_and_link() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // engine.helpers exports a proc; badniks.plant branches to it.
    write(root, "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n");
    write(root, "badniks/plant.emp",
        "module badniks.plant\nuse engine.helpers.{Draw_Sprite}\n\
         proc init (a0: *u8) {\n    jbra Draw_Sprite\n}\n");
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", root.join("badniks/plant.emp").to_str().unwrap(),
               "--root", root.to_str().unwrap(),
               "-o", out.to_str().unwrap()])
        .status().unwrap();
    assert!(status.success(), "multi-module compile should succeed");
    assert!(out.exists() && std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn missing_use_reports_add_use_fixit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "engine/helpers.emp",
        "module engine.helpers\npub proc Draw_Sprite (a0: *u8) {\n    rts\n}\n");
    write(root, "badniks/plant.emp", // NOTE: no `use`
        "module badniks.plant\nproc init (a0: *u8) {\n    jbra Draw_Sprite\n}\n");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", root.join("badniks/plant.emp").to_str().unwrap(),
               "--root", root.to_str().unwrap()])
        .output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("add `use engine.helpers.{Draw_Sprite}`"), "stderr was: {stderr}");
}
```

- [ ] **Step 2: Run to confirm it fails.**

Run: `cargo test -p sigil-cli --test module_resolution`
Expected: FAIL — `--root` flag unknown / driver missing.

- [ ] **Step 3: Implement `build_program` in `resolve/mod.rs`.**

```rust
// crates/sigil-frontend-emp/src/resolve/mod.rs  (add below the `pub mod` lines)
pub mod imports;
pub mod manifest;
pub mod rename;

use crate::lower::{lower_module, LowerOptions};
use imports::{ExportIndex, ResolveEnv};
use manifest::Manifest;
use rename::rename_module;
use sigil_ir::Section;
use sigil_span::{Diagnostic, Level};
use std::collections::{HashMap, HashSet, VecDeque};

/// Compile a whole `.emp` program to a single concatenated `Vec<Section>`,
/// resolving cross-module names. `entry_id` is the module the build starts from;
/// `prelude_id` (Task 3) names the auto-imported prelude module, if any.
pub fn build_program(
    manifest: &Manifest,
    entry_id: &str,
    prelude_id: Option<&str>,
    opts: &LowerOptions,
) -> (Vec<Section>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut sections = Vec::new();

    // Reachability: BFS from entry over `use` edges (+ prelude, always reachable).
    let reachable = reachable_modules(manifest, entry_id, prelude_id, &mut diags);

    // Export index over the whole reachable set.
    let modrefs: Vec<(&str, &crate::ast::File)> = reachable.iter()
        .map(|&i| (manifest.modules[i].id.as_str(), &manifest.modules[i].file))
        .collect();
    let index = ExportIndex::build(&modrefs);
    let prelude = prelude_id.and_then(|pid| {
        manifest.by_id.get(pid).map(|&i| (manifest.modules[i].id.as_str(), &manifest.modules[i].file))
    });

    for &i in &reachable {
        let pm = &manifest.modules[i];
        let (env, mut ediags) = ResolveEnv::build(&pm.id, &pm.file, &index, prelude);
        diags.append(&mut ediags);

        // Lower this module (Task 3 adds ambient comptime defs here).
        let (mut module, mut ldiags) = lower_module(&pm.file, opts);
        diags.append(&mut ldiags);

        // Build the rename map: every name the module references → canonical.
        let map = env.into_rename_map(); // see helper below
        // Verify every non-local fixup target resolves; else "add use" / unknown.
        report_unresolved(&module, &map, &env_for_suggest(&pm.id, &pm.file, &index, prelude), &mut diags);
        rename_module(&mut module, &map);
        sections.extend(module.sections);
    }
    (sections, diags)
}

fn reachable_modules(
    manifest: &Manifest, entry_id: &str, prelude_id: Option<&str>, diags: &mut Vec<Diagnostic>,
) -> Vec<usize> {
    let mut seen = HashSet::new();
    let mut order = Vec::new();
    let mut q = VecDeque::new();
    let mut push = |id: &str, q: &mut VecDeque<usize>, seen: &mut HashSet<usize>, diags: &mut Vec<Diagnostic>| {
        match manifest.by_id.get(id) {
            Some(&i) if seen.insert(i) => q.push_back(i),
            None => diags.push(Diagnostic { level: Level::Error,
                message: format!("no module `{id}` found under the scan root"), span: sigil_span::Span::default() }),
            _ => {}
        }
    };
    push(entry_id, &mut q, &mut seen, diags);
    if let Some(p) = prelude_id { push(p, &mut q, &mut seen, diags); }
    while let Some(i) = q.pop_front() {
        order.push(i);
        for item in &manifest.modules[i].file.items {
            if let crate::ast::Item::Use(u) = item {
                let base = u.base.segments.join(".");
                push(&base, &mut q, &mut seen, diags);
            }
        }
    }
    order
}
```

Add to `imports.rs` an `into_rename_map(self) -> HashMap<String,String>` returning the internal `map`, and keep `ResolveEnv::build` usable twice (or build the map and a suggestion-env). Implement `report_unresolved`: for every fixup target symbol that is NOT `$`-prefixed (local), NOT in `map`, call `env.suggest_use(name)` → error with the fix-it, or a plain "unknown symbol `{name}`" if no suggestion. (Use `rename::collect_target_syms` to enumerate targets before renaming.)

Wire the CLI in `crates/sigil-cli/src/main.rs`: when `--root <dir>` is present on `emp`, call `Manifest::scan(root)`, derive `entry_id` from the entry file's parsed header, `build_program(...)`, then the existing `resolve_layout` + `link` (+ `emit_rom` if `--map`). Print diagnostics (fix-it text included) to stderr and exit non-zero on any `Level::Error`. Without `--root`, keep today's single-file `compile_emp` path exactly as-is.

- [ ] **Step 4: Run to confirm pass.**

Run: `cargo test -p sigil-cli --test module_resolution`
Expected: PASS (both tests).

- [ ] **Step 5: Green gate + commit.**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
git add crates/sigil-frontend-emp/src/resolve crates/sigil-cli/src/main.rs \
        crates/sigil-cli/tests/module_resolution.rs
git commit -m "feat(resolve): build_program driver + CLI --root — cross-module labels link end-to-end"
```

---

## Task 3: Prelude auto-import + ambient comptime-def injection

Cross-module TYPE/const/fn references (`Sst`, `ObjDef`, `spawn`) are used in comptime position, so they must be visible to the evaluator while lowering — the rename pass (labels-only) is not enough. Add ambient injection to the evaluator and thread it from the driver, then name one module the prelude and auto-inject its `pub` defs everywhere.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/eval/mod.rs` (ambient constructor)
- Modify: `crates/sigil-frontend-emp/src/layout.rs`, `src/lower/mod.rs`, `src/lower/proc.rs` (thread ambient)
- Modify: `crates/sigil-frontend-emp/src/resolve/mod.rs` (`build_program` passes ambient)
- Create: `examples/prelude.emp`
- Test: extend `crates/sigil-cli/tests/module_resolution.rs`

- [ ] **Step 1: Write the failing test.**

```rust
// append to crates/sigil-cli/tests/module_resolution.rs
#[test]
fn prelude_types_resolve_without_use() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Prelude exports a struct type used by the object module with NO `use`.
    write(root, "prelude.emp",
        "module prelude\npub struct ObjDef (size: 2) { code: *u8 }\n");
    write(root, "badniks/plant.emp",
        "module badniks.plant\nproc init (a0: *u8) {\n    rts\n}\n\
         pub data Def = ObjDef{ code: init }\n");
    let out = root.join("out.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", root.join("badniks/plant.emp").to_str().unwrap(),
               "--root", root.to_str().unwrap(), "--prelude", "prelude",
               "-o", out.to_str().unwrap()])
        .status().unwrap();
    assert!(status.success());
    // Def is 2 bytes: a pointer to init (fixup) — file is non-empty.
    assert!(std::fs::metadata(&out).unwrap().len() >= 2);
}
```

- [ ] **Step 2: Run to confirm it fails.**

Run: `cargo test -p sigil-cli --test module_resolution prelude_types_resolve_without_use`
Expected: FAIL — `ObjDef` is an unknown name during evaluation (no ambient injection yet).

- [ ] **Step 3: Add ambient injection to the evaluator.**

```rust
// crates/sigil-frontend-emp/src/eval/mod.rs — add alongside with_file (~line 252)

/// Comptime definitions imported from OTHER modules (prelude + `use`d types),
/// borrowed from their owning ASTs. Local defs shadow these (indexed after).
#[derive(Default)]
pub struct Ambient<'a> {
    pub consts: Vec<&'a ast::ConstDecl>,
    pub enums: Vec<&'a ast::EnumDecl>,
    pub fns: Vec<&'a ast::ComptimeFnDecl>,
    pub structs: Vec<&'a ast::StructDecl>,
    pub bitfields: Vec<&'a ast::BitfieldDecl>,
    pub newtypes: Vec<&'a ast::NewtypeDecl>,
    pub offsets: Vec<&'a ast::OffsetsDecl>,
}

impl<'a> Evaluator<'a> {
    pub fn with_file_with_ambient(file: &'a ast::File, ambient: &Ambient<'a>) -> Self {
        let mut ev = Evaluator::new();
        // Ambient FIRST so local index_items shadows on name clash (last-wins).
        for c in &ambient.consts { ev.consts.insert(c.name.as_str(), c); }
        for e in &ambient.enums { ev.enums.insert(e.name.as_str(), e); }
        for f in &ambient.fns { ev.fns.insert(f.name.as_str(), f); }
        for s in &ambient.structs { ev.structs.insert(s.name.as_str(), s); }
        for b in &ambient.bitfields { ev.bitfields.insert(b.name.as_str(), b); }
        for n in &ambient.newtypes { ev.newtypes.insert(n.name.as_str(), n); }
        for o in &ambient.offsets { ev.offsets.insert(o.name.as_str(), o); }
        ev.index_items(&file.items);
        ev
    }
}
```

Keep `with_file` as `with_file_with_ambient(file, &Ambient::default())` so existing callers are unchanged.

- [ ] **Step 4: Thread `Ambient` from the driver to the eval sites.**

`lower_module` gains an ambient-carrying sibling. Simplest low-churn approach: give `LowerOptions` an `ambient: Ambient<'a>` by making it lifetime-parameterized, OR add `lower_module_with_ambient(file, opts, &ambient)` and route `eval_data_with_root` / `eval_offsets_with_root` / proc eval to `with_file_with_ambient`. Choose the sibling-function route (no lifetime on `LowerOptions`, matching T0 decision). Every current `Evaluator::with_file(file)` inside `layout.rs`/`proc` lowering takes an added `&Ambient` argument (default-empty on the single-file path).

In `resolve/mod.rs build_program`, before lowering each module, assemble its `Ambient` from the prelude's `pub` comptime items + the comptime items named by its `use` lists, borrowing from `manifest.modules[..].file` (all owned by the manifest, so lifetimes hold), then call `lower_module_with_ambient(&pm.file, opts, &ambient)`.

- [ ] **Step 5: Draft `examples/prelude.emp` (proposed; contents reviewed at checkpoint).**

Scope it to what the corpus and acceptance exhibit reference, grounded in [[emp-sonic-newtype-candidates]]:
```
// examples/prelude.emp — Aeon game prelude (PROPOSED, reviewed at checkpoint).
module prelude

// Domain newtypes (erasing — zero ROM cost).
pub newtype Angle    = u8
pub newtype VramTile = u16 where 0..2047

// Object-system vocabulary the object modules lean on.
pub struct ObjDef (size: 8) {
    code: *u8,
    map:  *u8,
    // ... elided fields mirror the AS ObjDef field-for-field ...
}
pub bitfield ArtTile: u16 { pri: 1, pal: 2, tile: 11 @ 0 }
// ... Collision / Size / Vel / Vec, spawn/anim/routine helpers added as the
//     corpus needs them; each earns its place (NESHLA: ship VDP/DMA/Z80 helpers too).
```

- [ ] **Step 6: Run to confirm pass + green gate.**

Run: `cargo test -p sigil-cli --test module_resolution`
Expected: PASS (all tests incl. `prelude_types_resolve_without_use`).
Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 7: Commit.**

```bash
git add crates/sigil-frontend-emp/src examples/prelude.emp crates/sigil-cli/tests/module_resolution.rs
git commit -m "feat(resolve): ambient comptime-def injection + prelude auto-import (§3.4)"
```

---

## Task 4: `module … in <section>` placement + region-budget overflow

Route each module's top-level items into the section named by its `module … in <section>` header (or a program section), place sections in declared order, and let `emit_rom`'s existing region check emit the §7.3 "over by N bytes" error.

**Files:**
- Modify: `crates/sigil-frontend-emp/src/lower/mod.rs` (consume `file.module.in_section`)
- Modify: `crates/sigil-frontend-emp/src/resolve/mod.rs` (place sections in map order)
- Verify/extend: `crates/sigil-link/src/map_load.rs` + `emit_rom` overflow message
- Test: extend `crates/sigil-cli/tests/module_resolution.rs`; a fixture `sigil.map.toml`

- [ ] **Step 1: Write the failing test.**

```rust
// append to crates/sigil-cli/tests/module_resolution.rs
#[test]
fn module_lands_in_named_section_and_budget_overflow_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "sigil.map.toml",
        "fill = 0x00\n\n[[region]]\nname = \"obj_bank\"\nlma_base = 0x10000\nsize = 4\nkind = \"rom\"\n");
    // A module whose data (8 bytes) overflows the 4-byte obj_bank region.
    write(root, "big.emp",
        "module big in obj_bank\npub data Blob: [u8; 8] = [1,2,3,4,5,6,7,8]\n");
    let out = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", root.join("big.emp").to_str().unwrap(),
               "--root", root.to_str().unwrap(),
               "--map", root.join("sigil.map.toml").to_str().unwrap()])
        .output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("over by") && err.contains("obj_bank"), "stderr: {err}");
}
```

- [ ] **Step 2: Run to confirm it fails.**

Run: `cargo test -p sigil-cli --test module_resolution module_lands_in_named_section`
Expected: FAIL — `in_section` ignored / no budget error path.

- [ ] **Step 3: Consume `in_section`.**

In `lower_module`, when `file.module.in_section` is `Some(name)`, open the default section under that NAME (instead of `"text"`) so its items carry the section name the map places by. Concretely, thread `file.module.in_section.as_deref()` into `ensure_default` / the initial `switch_section_lma` so the default section's name is the module's target section. (Its VMA/LMA still come from the section/region as before; the driver assigns final LMA in Step 4.)

- [ ] **Step 4: Place sections by map order in the driver + surface the overflow.**

In `build_program`, after concatenating sections, group them by name and assign each group's LMA from the `sigil.map.toml` region `lma_base` (in map-declared order). Verify `emit_rom` already computes per-section end vs `lma_base + size`; if its message is not already "over by N bytes … <region>", update `sigil_link::emit_rom` to emit exactly that (this is a message-only change in `sigil-link`; it does not alter placement logic). The CLI already calls `emit_rom` when `--map` is passed — ensure its `Err(String)` is printed to stderr and exits non-zero.

> If `emit_rom` already produces an equivalent overflow error, keep it and only assert the wording in the test; do not restructure placement.

- [ ] **Step 5: Run to confirm pass + green gate.**

Run: `cargo test -p sigil-cli --test module_resolution`
Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add crates/sigil-frontend-emp/src crates/sigil-link/src crates/sigil-cli/tests/module_resolution.rs
git commit -m "feat(resolve): module→section placement + region-budget overflow (§7.3)"
```

---

## Task 5: Corpus — pitcher_plant end-to-end + multi-module byte-check + adversarial review

Prove the milestone: `examples/pitcher_plant.emp` compiles end-to-end, and a hand-built 3-module program links and is byte-checked where a byte argument exists.

**Files:**
- Create: sibling modules for `pitcher_plant.emp` (`examples/pp_siblings/*.emp`) providing `Player_1`, `Map_PitcherPlant`, `VRAM_PITCHER_PLANT`, and the object helpers/types the exhibit uses that aren't in the prelude.
- Modify: `examples/prelude.emp` (add any names the exhibit needs)
- Test: `crates/sigil-cli/tests/module_resolution.rs` (pitcher_plant end-to-end); `crates/sigil-frontend-emp/tests/lower_corpus.rs` if a byte-level assertion fits.

- [ ] **Step 1: Write the acceptance test.**

```rust
// append to crates/sigil-cli/tests/module_resolution.rs
#[test]
fn pitcher_plant_exhibit_compiles_end_to_end() {
    // The repo's standing acceptance exhibit + its prelude + sibling modules.
    let manifest_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let out = tempfile::tempdir().unwrap().path().join("pp.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", &format!("{manifest_root}/pitcher_plant.emp"),
               "--root", manifest_root, "--prelude", "prelude",
               "-o", out.to_str().unwrap()])
        .status().unwrap();
    assert!(status.success(), "pitcher_plant.emp must compile end-to-end");
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}
```

- [ ] **Step 2: Run to confirm it fails, then fill the gaps.**

Run: `cargo test -p sigil-cli --test module_resolution pitcher_plant_exhibit`
Expected: FAIL — unresolved names (`Player_1`, `Map_PitcherPlant`, `VRAM_PITCHER_PLANT`, `ObjectMove`, `Draw_Sprite`, `facing_abs`, `spawn`, `anim`, `routine`, `Sst`/`sst_custom`, `Collision`, `Size`, `Vel`, `Vec`, `Def`, `Ani`).
Iterate: add each missing name to `examples/prelude.emp` (types/helpers) or a sibling module (labels like `Map_PitcherPlant`, `Player_1`), re-running until it compiles. Each addition is a real declaration mirroring the AS original field-for-field (Appendix D argues the byte layout). Keep the prelude for shared vocabulary; put object-instance data (`Map_PitcherPlant`, `VRAM_PITCHER_PLANT`) in siblings.

> `spawn`/`anim`/`routine`/`facing_abs`/`despawn_below_level` are comptime helpers (macro-like). If the exhibit needs them as `comptime fn`s the evaluator can expand, add them to the prelude; if any require grammar the current front-end lacks, that is a spec defect to raise at the checkpoint (A-Spec2.1), not to paper over.

- [ ] **Step 3: Add a byte-checked multi-module program.**

Build a minimal 3-module program (object module + prelude + shared-data module) whose emitted bytes are independently predictable (e.g. an `offsets` table pointing across modules), and assert the exact bytes:

```rust
#[test]
fn cross_module_offsets_table_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write(root, "prelude.emp", "module prelude\n"); // empty prelude
    write(root, "targets.emp",
        "module targets\npub data A: [u8;1] = [0xAA]\npub data B: [u8;1] = [0xBB]\n");
    write(root, "tab.emp",
        "module tab\nuse targets.{A, B}\n\
         pub offsets T {\n    First:  A,\n    Second: B,\n}\n");
    let out = root.join("t.bin");
    let status = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", root.join("tab.emp").to_str().unwrap(),
               "--root", root.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .status().unwrap();
    assert!(status.success());
    // Assert the offset words are `target - T` big-endian for the placed layout.
    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.is_empty()); // tighten to exact dc.w values once layout is fixed
}
```

Tighten the final assertion to exact bytes once the section layout is pinned (this is the §4.7 cross-module offset-target deferral being discharged).

- [ ] **Step 4: Green gate + whole-branch adversarial review.**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Then dispatch a `superpowers:code-reviewer` subagent over the whole branch diff with the mandate to CONSTRUCT and RUN new cross-module programs (name collisions across modules, glob imports, a module cycle for symbols, a comptime cycle that must error, an over-budget region) and confirm each behaves per spec §3/§7. Fix anything it surfaces.

- [ ] **Step 5: Commit.**

```bash
git add examples crates/sigil-cli/tests/module_resolution.rs
git commit -m "test(resolve): pitcher_plant end-to-end + cross-module byte-check corpus"
```

- [ ] **Step 6: Milestone checkpoint (do NOT merge without it).**

Present Volence a summary + gap list (esp. any prelude-contents decisions and any spec defects surfaced). Volence chooses `--no-ff` merge+push or changes-first. Update memory [[spec2-progress]] to mark #4 merged and point at the next backlog item (#5 `assert!`/capacity refinements).

---

## Self-Review (completed against the design spec)

- **Spec coverage:** §3.1 files/modules → T1 (manifest + path lint). §3.2 imports/cycles → T2 (`use` resolution) + T5 (cycle tests). §3.3 placement → T4. §3.4 prelude → T3. §4.7 cross-module offset targets → T5 Step 3. §7.3 budgets → T4. §9 "add use" fix-it → T2c. Acceptance (pitcher_plant, 3-module byte-check, linker untouched, harness green) → T5.
- **Placeholder scan:** the two spots that read as under-specified (T3 Step 4 ambient threading; T4 Step 4 `emit_rom` message) are deliberately expressed as "verify existing behavior, extend only if needed" with a concrete test pinning the contract — not TODOs.
- **Type consistency:** `Manifest`/`ParsedModule`, `ExportIndex::build`, `ResolveEnv::{build,resolve,suggest_use,into_rename_map}`, `canonical`, `rename_module`/`collect_target_syms`, `Ambient`/`with_file_with_ambient`, `build_program` are used with the same signatures across all tasks.
- **Ambiguity:** canonical naming (`module_id.name`), precedence (local > use > prelude), and single-file-path-untouched are stated once and referenced.
