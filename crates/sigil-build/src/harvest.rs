//! What the build gathers before it places anything: the residual AS side's result
//! ([`AsSide`]), the lowered `.emp` program ([`EmpProgram`]) and its prepared inputs,
//! the comptime-helper manifest rewrites the whole-program lower and the RAM harvest
//! share, and the `--extra-entry` resolution and refusal.
//!
//! Moved from sigil-harness (`native.rs`) unchanged by G0b Task 4. The harvests proper
//! are the game-as-data driver's (`crate::driver`), reading the descriptions; the
//! profile-reading copies in sigil-harness were deleted with that driver (G0b Task 11).

use std::path::Path;

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve;
use sigil_ir::{Module, Section};

use crate::diag::BuildWarning;
use crate::place::ModuleSpec;

/// Give EVERY reachable module a glob (`.*`) import of ALL pure-comptime helper
/// modules, so each module's ambient set is the FULL transitively-closed helper
/// closure. This is required because ambient injection is SHALLOW (one `use` level):
/// a consumer of `sst` gets the `Sst` struct but not the `types.ObjRoutine` its
/// field references unless the consumer ALSO imports `types`. Blanketing every
/// module with every helper glob resolves all cross-helper comptime deps.
///
/// Byte-neutral: `collect_pub_comptime` injects only COMPTIME items (const/struct/
/// enum/bitfield/newtype/comptime-fn/vars) — which lower to ZERO bytes and ZERO
/// link symbols — and never `ensure`/`data`/`proc`, so no drift-guard is duplicated
/// and no data/label is re-emitted. Existing helper `use`s are removed first so a
/// helper is imported exactly once (a doubled glob would inject its decls twice).
pub fn normalize_helper_imports(
    manifest: &mut resolve::manifest::Manifest,
    helper_ids: &[&str],
    also_drop: &[&str],
) {
    use sigil_frontend_emp::ast;

    // Parse one synthetic file of `use <helper>.*` lines to mint real Use items
    // (with valid Path ASTs) — cheaper than hand-building the AST.
    let mut src = String::from("module native_helper_globs\n");
    for id in helper_ids {
        src.push_str(&format!("use {id}.*\n"));
    }
    let (glob_file, _d) = sigil_frontend_emp::parse_str(&src);
    let glob_uses: Vec<ast::Item> = glob_file
        .items
        .into_iter()
        .filter(|i| matches!(i, ast::Item::Use(_)))
        .collect();

    let is_helper_use = |item: &ast::Item| -> bool {
        if let ast::Item::Use(u) = item {
            let base = u.base.segments.join(".");
            helper_ids.contains(&base.as_str()) || also_drop.contains(&base.as_str())
        } else {
            false
        }
    };

    for pm in manifest.modules.iter_mut() {
        let own_id = pm.file.module.path.segments.join(".");
        // Drop existing helper imports (they'd double the glob-injected decls).
        pm.file.items.retain(|i| !is_helper_use(i));
        // Prepend one glob per helper, minus the module's own (ambient never
        // injects a module into itself, but a self-glob is a needless clone).
        let mut prepend: Vec<ast::Item> = glob_uses
            .iter()
            .filter(|i| {
                if let ast::Item::Use(u) = i {
                    u.base.segments.join(".") != own_id
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        prepend.append(&mut pm.file.items);
        pm.file.items = prepend;
    }
}

/// The whole-program build's helper rewrite: [`publicize_helper_comptime`], then
/// [`normalize_helper_imports`], over `helpers`. Returns the import-rule Errors of
/// the `use` lines the rewrite removes ([`helper_import_errors`]), taken BEFORE
/// either step, against the tree as written: the removed lines never reach the
/// resolve pass, and after publicizing a private helper name would pass. The
/// caller reports them for the modules its build lowers
/// ([`helper_import_errors_in`]).
pub fn rewrite_helper_imports(
    manifest: &mut resolve::manifest::Manifest,
    helpers: &[&str],
) -> Vec<(usize, sigil_span::Diagnostic)> {
    let removed = helper_import_errors(manifest, helpers);
    publicize_helper_comptime(manifest, helpers);
    normalize_helper_imports(manifest, helpers, &[]);
    removed
}

/// The import rule for the `use` lines [`normalize_helper_imports`] removes: every
/// `use` of one of `helper_ids`, in every module, checked against the tree as
/// written (before [`publicize_helper_comptime`] widens the helpers' exports). Each
/// entry is the importing module's index and the Error at that `use`'s own span.
///
/// The rewrite replaces an author's `use engine.objects.frames.{refresh_piece_count}`
/// with a glob, so the resolve pass, which owns the rule, never sees the line. Without
/// this, a helper `use` naming an item that does not exist, or a private one, built
/// clean whenever nothing read the name. Kill condition: retire this with
/// `normalize_helper_imports`. Once the author's `use` lines reach the resolve pass
/// unrewritten, the pass applies the rule itself.
pub fn helper_import_errors(
    manifest: &resolve::manifest::Manifest,
    helper_ids: &[&str],
) -> Vec<(usize, sigil_span::Diagnostic)> {
    use sigil_frontend_emp::resolve::imports::{use_decl_errors, use_decls, ExportIndex};
    let pairs: Vec<(&str, &sigil_frontend_emp::ast::File)> =
        manifest.modules.iter().map(|pm| (pm.id.as_str(), &pm.file)).collect();
    let index = ExportIndex::build(&pairs);
    let mut out = Vec::new();
    for (i, pm) in manifest.modules.iter().enumerate() {
        for u in use_decls(&pm.file.items) {
            if helper_ids.contains(&u.base.segments.join(".").as_str()) {
                out.extend(use_decl_errors(u, &index).into_iter().map(|d| (i, d)));
            }
        }
    }
    out
}

/// The [`helper_import_errors`] whose importing module the build lowered (`lowered`,
/// the modules whose code the build contains), minus any the build already
/// reported: the rewrite removes top-level `use` lines only, so a helper `use`
/// inside a `section {}` body reaches the resolve pass and is reported there too.
pub fn helper_import_errors_in(
    manifest: &resolve::manifest::Manifest,
    errors: &[(usize, sigil_span::Diagnostic)],
    lowered: &[String],
    reported: &[sigil_span::Diagnostic],
) -> Vec<sigil_span::Diagnostic> {
    let lowered: std::collections::HashSet<&str> = lowered.iter().map(String::as_str).collect();
    errors
        .iter()
        .filter(|(i, d)| lowered.contains(manifest.modules[*i].id.as_str()) && !reported.contains(d))
        .map(|(_, d)| d.clone())
        .collect()
}

/// Publicize the PRIVATE comptime items (const/struct/enum/bitfield/newtype/
/// comptime-fn/vars-overlay) of the helper modules, so glob ambient injection pulls
/// a helper's full comptime closure — including private callees like `vdp.emp`'s
/// `target_bits`, invoked by the pub `vdp_comm`. Deliberately leaves `equ` (a
/// link-symbol emitter), `data`, and `proc` untouched: only comptime kinds, which
/// lower to ZERO bytes and ZERO link symbols, so visibility is byte-neutral.
pub fn publicize_helper_comptime(manifest: &mut resolve::manifest::Manifest, helpers: &[&str]) {
    use sigil_frontend_emp::ast;
    fn walk(items: &mut [ast::Item]) {
        for item in items.iter_mut() {
            match item {
                ast::Item::Const(d) => d.public = true,
                ast::Item::Struct(d) => d.public = true,
                ast::Item::Enum(d) => d.public = true,
                ast::Item::Bitfield(d) => d.public = true,
                ast::Item::Newtype(d) => d.public = true,
                ast::Item::ComptimeFn(d) => d.public = true,
                ast::Item::Vars(d) if d.name.is_some() => d.public = true,
                ast::Item::Section(sec) => walk(&mut sec.items),
                _ => {}
            }
        }
    }
    for pm in manifest.modules.iter_mut() {
        let id = pm.file.module.path.segments.join(".");
        if helpers.contains(&id.as_str()) {
            walk(&mut pm.file.items);
        }
    }
}

/// Resolve one `--extra-entry` argument to a manifest module id — an author names a
/// poison the way it is on disk, and a lane names it the way the module declares
/// itself, so both spellings resolve. PRECEDENCE, in order: a dotted module id wins;
/// then the argument as a path under the scan root (`<aeon>/<arg>`); then the argument
/// as a path in its own right (cwd-relative or absolute). So an argument that is both
/// a declared module id and a relative filename resolves as the id.
///
/// A name that resolves to nothing is an ERROR, never a skip: a lane whose subject
/// vanished (renamed, moved, deleted) must fail loudly rather than pass vacuously.
/// A file that failed to PARSE does not reach here: `Manifest::scan` registers such a
/// module anyway, but it raises an Error diagnostic doing so, and `build_emp` bails on
/// the scan's errors before any extra entry is resolved.
pub fn resolve_extra_entry(
    manifest: &resolve::manifest::Manifest,
    aeon: &Path,
    arg: &str,
) -> Result<String, String> {
    if manifest.by_id.contains_key(arg) {
        return Ok(arg.to_string());
    }

    // Path form: compare canonicalized paths against the scanned set, so `./a/b.emp`,
    // `a/b.emp` and an absolute path all name the same module.
    let candidates = [aeon.join(arg), std::path::PathBuf::from(arg)];
    for cand in candidates.iter().filter_map(|p| std::fs::canonicalize(p).ok()) {
        for pm in &manifest.modules {
            if std::fs::canonicalize(&pm.path).is_ok_and(|p| p == cand) {
                // `ParsedModule::id` is the key `by_id` (and so the reachability BFS)
                // is built on — the module's DECLARED id, which a `[module.path-mismatch]`
                // module spells differently from its path.
                return Ok(pm.id.clone());
            }
        }
    }

    Err(format!(
        "--extra-entry `{arg}`: no such module under the scan root {}, it names \
         neither a scanned module id nor a scanned `.emp` file path",
        aeon.display()
    ))
}

/// Refuse an extra entry that would CONTRIBUTE to the artifact. `--extra-entry`
/// exists to run a module's comptime guards inside the real build profile, and it is
/// byte-neutral by contract: a module reached only this way must contribute nothing.
/// A module NAMED to the flag that declared bytes would place into a shipping region
/// (or chain onto the RAM map and move every address after it) and silently change
/// the build, so it is refused by name here rather than emitted.
///
/// The refused kinds are the ROM-byte producers (`section`/`data`/`proc`/`table`/
/// `offsets`/`dispatch`/`script`/`align`), `equ` (which mints a link symbol), and
/// whatever the compiler's own RAM-allocator predicate
/// ([`file_declares_region`](sigil_frontend_emp::lower::file_declares_region))
/// recognizes — called rather than restated, so the refusal tracks the allocator if
/// that predicate grows a case.
///
/// Everything else an `.emp` module can declare folds to zero bytes and zero link
/// symbols, the line `publicize_helper_comptime` draws. `implement`/`interface` are
/// the one accepted kind that is not purely local: an extra entry carrying either
/// joins the whole-program contract bind pass. That cannot drift the artifact — a
/// second `implement` for a bound interface, or a second `interface` of one name, is a
/// hard error in [`contract::bind`](sigil_frontend_emp::resolve::contract) — so it
/// fails loudly rather than rebinding a member.
///
/// LIMITATION, stated because the flag's promise is otherwise flat: this check covers
/// the NAMED module ONLY. The named module's own `use` edges are ordinary imports, and
/// nothing re-runs this refusal over them — importing a module the profile already
/// reaches costs nothing, but an extra entry that imports a byte-emitting module from
/// OUTSIDE the closure does pull it in. What catches that is the placement layer (an
/// unmapped section name fails `map.toml` placement) and the golden CRC gates, not
/// this function.
pub fn refuse_artifact_contribution(
    manifest: &resolve::manifest::Manifest,
    module_id: &str,
    arg: &str,
) -> Result<(), String> {
    use sigil_frontend_emp::ast;
    let idx = manifest.by_id[module_id];
    let file = &manifest.modules[idx].file;
    let emitter = file
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Section(_) => Some(("section", "emits into the ROM")),
            ast::Item::Data(_) => Some(("data", "emits into the ROM")),
            ast::Item::Proc(_) => Some(("proc", "emits into the ROM")),
            ast::Item::Table(_) => Some(("table", "emits into the ROM")),
            ast::Item::Offsets(_) => Some(("offsets", "emits into the ROM")),
            ast::Item::Dispatch(_) => Some(("dispatch", "emits into the ROM")),
            ast::Item::Script(_) => Some(("script", "emits into the ROM")),
            ast::Item::Align(_) => Some(("align", "emits into the ROM")),
            ast::Item::Equ(_) => Some(("equ", "mints a link symbol")),
            _ => None,
        })
        // RAM allocation is the compiler's predicate to define. The label below is
        // only for the message — a kind the predicate grows and this scan does not
        // recognize still refuses, under its generic name.
        .or_else(|| {
            if !sigil_frontend_emp::lower::file_declares_region(file) {
                return None;
            }
            let kind = file
                .items
                .iter()
                .find_map(|item| match item {
                    ast::Item::Region(_) => Some("region"),
                    // The overlay form (`vars Name: window { … }`) is a comptime view
                    // over existing bytes; the region form (no name) allocates.
                    ast::Item::Vars(v) if v.name.is_none() => Some("vars"),
                    _ => None,
                })
                .unwrap_or("a RAM allocator");
            Some((kind, "allocates RAM"))
        });
    match emitter {
        None => Ok(()),
        Some((kind, effect)) => Err(format!(
            "--extra-entry `{arg}`: module `{module_id}` declares `{kind}`, which {effect}. \
             --extra-entry evaluates a module's comptime guards inside the real build \
             profile and is byte-neutral by contract; a module that contributes to the \
             artifact must enter the build through the registry and a `use` edge instead."
        )),
    }
}

/// A finished AS-side assembly: its sections plus any warn-tier diagnostic the
/// residual AS source raised (an author-written `warning` directive), already
/// located against the AS front end's own source map.
///
/// The warnings ride out of here rather than being dropped at the seam because
/// the whole point of the warn tier is that the build SUCCEEDS and still says
/// something. Aeon writes no `warning` today (measured 2026-09-04: zero
/// directive-column sites across its residual `.asm` files), so this vector is
/// empty on every shipped shape — which makes the first one anybody adds
/// visible in the build's tally line the day it is written, instead of silent.
pub struct AsSide {
    pub module: Module,
    pub warnings: Vec<BuildWarning>,
    /// The AS front end's OWN [`SourceMap`](sigil_span::SourceMap): the only
    /// authority that can read a span carried by one of `module`'s fragments.
    ///
    /// It used to be dropped here, and dropping it is what made the chained
    /// build's mislocation unfixable rather than merely wrong: with the AS map
    /// gone, the `.emp` manifest's index was the only authority left in scope
    /// and every AS span was read through it. See [`ChainSources`](crate::diag::ChainSources).
    pub sources: sigil_span::SourceMap,
}

/// The placed `.emp` program: its sections, its deferred link asserts (drift
/// guards), and every non-error diagnostic the lowering produced.
pub struct EmpProgram {
    /// Every placed `.emp` section.
    pub sections: Vec<Section>,
    /// The whole program's deferred link asserts (drift guards).
    pub link_asserts: Vec<sigil_ir::LinkAssert>,
    /// How many `ensure` evaluations the reachable modules decided at comptime
    /// (the complement of `link_asserts`; see [`sigil_ir::Module::comptime_guards`]).
    pub comptime_guards: usize,
    /// Warnings and notes from all four lowering diagnostic sources (manifest scan,
    /// entry parse, program build, placement), deduplicated and location-resolved.
    pub warnings: Vec<BuildWarning>,
    /// The location authority these warnings resolved through. Handed out because
    /// the LINK tier produces warn-tier diagnostics of its own — a `Level::Warning`
    /// [`LinkAssert`](sigil_ir::LinkAssert) such as `[layout.odd-item]` fails at
    /// link time, long after the manifest that owns the source texts is gone — and
    /// they must render through the same index rather than a second one.
    pub sources: sigil_frontend_emp::resolve::manifest::SourceIndex,
    /// The module roots this build placed, derived from the game's map over the
    /// scanned tree (sigil-harness `module_roots`).
    pub registry: Vec<ModuleSpec>,
    /// The lowering's symbol side table (module, procedure extent, `pub`) for the
    /// build export.
    pub symbols: sigil_frontend_emp::export_side::ProgramSymbols,
    /// The lowering's layout and constant registry for the build export.
    pub layouts: sigil_frontend_emp::export_side::ProgramLayouts,
}

/// Natively lower + place every map-derived `.emp` module for `profile`. Returns the
/// placed sections, the whole program's deferred link asserts (drift guards), and
/// every reportable non-error diagnostic the lowering produced. The placement map
/// bases are COSMETIC (the chainer recomputes every base from the frozen table), so a
/// nominal one-region-per-section map suffices.
/// The whole-program build's inputs, exactly as the driver's `build_emp` lowers them: the scanned
/// manifest after the helper-import rewrite and with the synthetic entry minted, the
/// shape's lowering options, and what the scan and the entry parse reported. Shared
/// with the driver's `name_env_report` so a report reads the SAME program the build lowers.
pub struct PreparedProgram {
    pub manifest: resolve::manifest::Manifest,
    pub mdiags: Vec<sigil_span::Diagnostic>,
    pub pdiags: Vec<sigil_span::Diagnostic>,
    pub source: sigil_span::SourceId,
    pub registry: Vec<ModuleSpec>,
    pub removed_import_errors: Vec<(usize, sigil_span::Diagnostic)>,
    pub entry_id: String,
    pub opts: LowerOptions,
}
