//! The new driver's module roots: which modules a shape places, from the map's `order`
//! and the closed `[[gate]]` rules, with no hardcoded gate list.
//!
//! The derivation is the old driver's (sigil-harness `module_roots::derive_module_roots`,
//! deleted with it, G0b Task 11):
//! every `order` row names the module(s) that define it (a label, a `section:` row, or the
//! owner a compiler-minted label names); a row more than one placed module defines is
//! settled by the `use` graph of everything else the build roots; the engine terminus is
//! always rooted. What changed is where "placed" comes from. The old driver asked
//! `SHAPE_GATES`, a list of only the modules SOME shape drops, so a new module with no
//! row ships everywhere; here every candidate module goes through
//! [`crate::gating::gate_modules`], which refuses a module no rule names.
//!
//! Scope: a candidate is a module the build scanned under one of its named roots
//! ([`ParsedModule::root`](sigil_frontend_emp::resolve::manifest::ParsedModule::root), G0b
//! Task 9): its identity follows its root, never its id's prefix. The old rule (a module
//! under `engine.` or the game's prefix) let a module with a game's id anywhere in the
//! checkout be a candidate and kept a module under the game's root with another id out;
//! the contract closure scoped by file root meanwhile, so the two scopes could disagree.
//! The old rule also hid the `sigil_objroutine_probe` fixture by a hardcoded id; here it
//! is an ordinary candidate that its game's `[[gate]]` row places in no shape.

use std::collections::{HashMap, HashSet, VecDeque};

use sigil_frontend_emp::ast;
use sigil_frontend_emp::resolve::imports::use_decls;
use sigil_frontend_emp::resolve::manifest::Manifest;

use crate::describe::{GateRule, ResolvedShape};
use crate::driver::BuildError;
use crate::gating::{gate_modules, ModuleId, ScannedModule};
use crate::map_placement::section_row;
use crate::place::ModuleSpec;

/// The sections a module's file declares: its `in section` header, then each `section`
/// block, else the default `text`.
pub fn declared_sections(file: &ast::File) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(s) = &file.module.in_section {
        out.push(s.clone());
    }
    for item in &file.items {
        if let ast::Item::Section(sec) = item {
            if !out.contains(&sec.name) {
                out.push(sec.name.clone());
            }
        }
    }
    if out.is_empty() {
        out.push("text".to_string());
    }
    out
}

fn item_labels(items: &[ast::Item], out: &mut Vec<String>) {
    for item in items {
        let named = match item {
            ast::Item::Data(d) => Some(&d.name),
            ast::Item::Proc(p) => Some(&p.name),
            ast::Item::Offsets(o) => Some(&o.name),
            ast::Item::Dispatch(d) => Some(&d.name),
            ast::Item::Script(s) => Some(&s.name),
            ast::Item::Table(t) => Some(&t.name),
            _ => None,
        };
        if let Some(n) = named {
            out.push(n.clone());
        }
        if let ast::Item::Section(sec) = item {
            item_labels(&sec.items, out);
        }
    }
}

/// The owning module id a compiler-minted row (`__<kind>$<module>$<n>`) names.
fn minted_owner(row: &str) -> Option<&str> {
    if !row.starts_with("__") {
        return None;
    }
    let mut parts = row.split('$');
    parts.next()?;
    let owner = parts.next()?;
    parts.next()?;
    Some(owner)
}

fn use_closure(manifest: &Manifest, seeds: &[usize]) -> HashSet<usize> {
    let mut seen: HashSet<usize> = seeds.iter().copied().collect();
    let mut queue: VecDeque<usize> = seeds.iter().copied().collect();
    while let Some(i) = queue.pop_front() {
        for u in use_decls(&manifest.modules[i].file.items) {
            if let Some(&j) = manifest.by_id.get(&u.base.segments.join(".")) {
                if seen.insert(j) {
                    queue.push_back(j);
                }
            }
        }
    }
    seen
}

/// What a build roots, given its inputs.
pub struct RootsInput<'a> {
    pub manifest: &'a Manifest,
    pub order: &'a [String],
    /// The game's module prefix (`games.sonic4`).
    pub game_prefix: &'a str,
    /// Engine rules then game rules, in that order (rule indices in errors count across both).
    pub rules: &'a [GateRule],
    pub shape: &'a ResolvedShape,
    /// The modules the synthetic entry `use`s before the placed ones.
    pub seeds: &'a [String],
    /// The engine's terminus module (`EndOfRom`).
    pub terminus: &'a str,
    pub map_name: &'a str,
}

/// Is module `pm` in the build's scope: scanned under one of its named roots? The one
/// scope rule for the candidates a shape may place ([`derive_roots`], [`gate_scan`]), and
/// the same scan the contract closure analyses ([`crate::driver::Project::scan`]).
pub fn in_candidate_scope(pm: &sigil_frontend_emp::resolve::manifest::ParsedModule) -> bool {
    pm.root.is_some()
}

/// The module roots `input.shape` places, one [`ModuleSpec`] per declared section, in
/// `order` order with the terminus last.
pub fn derive_roots(input: &RootsInput<'_>) -> Result<Vec<ModuleSpec>, BuildError> {
    let RootsInput { manifest, order, game_prefix, rules, shape, seeds, terminus, map_name } = *input;
    let in_scope = |id: &str| manifest.by_id.get(id).is_some_and(|&i| in_candidate_scope(&manifest.modules[i]));

    let mut by_label: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_section: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, pm) in manifest.modules.iter().enumerate() {
        if !in_scope(&pm.id) {
            continue;
        }
        let mut labels = Vec::new();
        item_labels(&pm.file.items, &mut labels);
        for l in labels {
            let owners = by_label.entry(l).or_default();
            if !owners.contains(&i) {
                owners.push(i);
            }
        }
        for sec in declared_sections(&pm.file) {
            by_section.entry(sec).or_default().push(i);
        }
    }
    let candidates = |row: &str| -> Vec<usize> {
        if let Some(sec) = section_row(row) {
            by_section.get(sec).cloned().unwrap_or_default()
        } else if let Some(owner) = minted_owner(row) {
            match manifest.by_id.get(owner) {
                Some(&i) if in_scope(owner) => vec![i],
                _ => Vec::new(),
            }
        } else {
            by_label.get(row).cloned().unwrap_or_default()
        }
    };

    let Some(&term) = manifest.by_id.get(terminus) else {
        return Err(BuildError::Context(format!(
            "[map.no-terminus] the tree has no `{terminus}` module (engine.toml `terminus`). Every \
             build roots it for `EndOfRom`"
        )));
    };

    // ── The closed gate, over every module a row could place, and the terminus. ──
    let rows: Vec<Vec<usize>> = order.iter().map(|r| candidates(r)).collect();
    let mut scan_idx: Vec<usize> = Vec::new();
    for c in rows.iter().flatten().copied().chain(std::iter::once(term)) {
        if !scan_idx.contains(&c) {
            scan_idx.push(c);
        }
    }
    let scan: Vec<ScannedModule> = scan_idx
        .iter()
        .map(|&i| ScannedModule { id: manifest.modules[i].id.clone(), path: manifest.modules[i].path.clone() })
        .collect();
    let kept: HashSet<ModuleId> = gate_modules(&scan, rules, shape)?.into_iter().collect();
    if !kept.contains(terminus) {
        return Err(BuildError::Context(format!(
            "[gate.terminus-dropped] the engine terminus `{terminus}` is gated out of shape `{}`; it \
             places `EndOfRom`, which every shape needs",
            shape.name
        )));
    }
    let admitted = |i: usize| kept.contains(&manifest.modules[i].id);

    let mut orphans: Vec<String> = Vec::new();
    let mut pending: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut picked: Vec<(usize, usize)> = Vec::new();
    for (ri, row) in order.iter().enumerate() {
        let c = &rows[ri];
        if c.is_empty() {
            if section_row(row).is_none() {
                orphans.push(row.clone());
            }
            continue;
        }
        let live: Vec<usize> = c.iter().copied().filter(|&i| admitted(i)).collect();
        match live.len() {
            0 => {}
            1 => picked.push((ri, live[0])),
            _ => pending.push((ri, live)),
        }
    }
    if !orphans.is_empty() {
        return Err(BuildError::Context(format!(
            "[map.order-orphan] {} `order` row(s) in `{map_name}` name nothing a module under \
             `engine.*` or `{game_prefix}.*` defines:\n  - {}",
            orphans.len(),
            orphans.join("\n  - ")
        )));
    }
    if !pending.is_empty() {
        let mut seed_idx: Vec<usize> = seeds.iter().filter_map(|s| manifest.by_id.get(s.as_str()).copied()).collect();
        seed_idx.extend(picked.iter().map(|&(_, m)| m));
        seed_idx.push(term);
        let reached = use_closure(manifest, &seed_idx);
        let mut ambiguous: Vec<String> = Vec::new();
        for (ri, live) in &pending {
            let hit: Vec<usize> = live.iter().copied().filter(|i| reached.contains(i)).collect();
            if hit.len() == 1 {
                picked.push((*ri, hit[0]));
            } else {
                let names: Vec<&str> = live.iter().map(|&i| manifest.modules[i].id.as_str()).collect();
                ambiguous.push(format!(
                    "`{}` is defined by {} and the rest of the build reaches {} of them",
                    order[*ri],
                    names.join(", "),
                    hit.len()
                ));
            }
        }
        if !ambiguous.is_empty() {
            return Err(BuildError::Context(format!(
                "[map.order-ambiguous] {} `order` row(s) in `{map_name}` name a label more than one \
                 placed module defines, and the `use` graph does not pick one:\n  - {}",
                ambiguous.len(),
                ambiguous.join("\n  - ")
            )));
        }
        picked.sort_by_key(|&(ri, _)| ri);
    }
    let mut roots: Vec<usize> = Vec::new();
    for (_, m) in picked {
        if !roots.contains(&m) {
            roots.push(m);
        }
    }
    if !roots.contains(&term) {
        roots.push(term);
    }
    Ok(roots
        .into_iter()
        .flat_map(|i| {
            let pm = &manifest.modules[i];
            declared_sections(&pm.file).into_iter().map(move |section| ModuleSpec { module_id: pm.id.clone(), section })
        })
        .collect())
}

/// Every module the closed gate is asked about for `order` (what an aeon description's
/// `[[gate]]` rows must cover): the in-scope candidates of every row, and the terminus.
pub fn gate_scan(manifest: &Manifest, order: &[String], terminus: &str) -> Vec<ModuleId> {
    let mut out: Vec<ModuleId> = Vec::new();
    let mut push = |id: &str| {
        if !out.iter().any(|o| o == id) {
            out.push(id.to_string());
        }
    };
    let mut labels_of: Vec<(usize, Vec<String>, Vec<String>)> = Vec::new();
    for (i, pm) in manifest.modules.iter().enumerate() {
        if in_candidate_scope(pm) {
            let mut l = Vec::new();
            item_labels(&pm.file.items, &mut l);
            labels_of.push((i, l, declared_sections(&pm.file)));
        }
    }
    for row in order {
        for (i, labels, secs) in &labels_of {
            let id = manifest.modules[*i].id.as_str();
            let hit = if let Some(sec) = section_row(row) {
                secs.iter().any(|s| s == sec)
            } else if let Some(owner) = minted_owner(row) {
                owner == id
            } else {
                labels.iter().any(|l| l == row)
            };
            if hit {
                push(id);
            }
        }
    }
    push(terminus);
    out
}
