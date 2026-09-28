//! THE MODULE ROOTS: which `.emp` modules a build places, DERIVED from the tree it
//! builds.
//!
//! A native build lowers the modules reachable from one synthetic entry module, whose
//! `use` lines are the roots. The roots come from the game's placement map
//! (`games/<g>/map.toml`): every `order` row names a byte-emitting section, and the
//! module that defines that row is a root. Deleting a module in the game tree is
//! therefore one edit on the game side (the file and its `order` row), and adding one
//! is the file plus its row. Nothing here names a game module that the map already
//! names.
//!
//! ── HOW A ROW NAMES ITS MODULE ──
//!
//! * a LABEL row (`"ObjDef_Static"`): the module under this build's scope that
//!   defines a label item (`data`/`proc`/`offsets`/`dispatch`/`script`/`table`) of
//!   that name. `pub` or not: a module-private label still links under its bare name
//!   (`Collected_Init`, `Ani_Sonic` are private heads the map names);
//! * a SECTION row (`"section:<name>"`): the module that declares that section, by
//!   `module … in <name>` or a nested `section <name> { … }`;
//! * a COMPILER-MINTED label (`"__align$<module>$0"`): the module id between the
//!   first two `$`, which is how the compiler spells the owner into the name.
//!
//! The SCOPE is `engine.*` plus the building game's `games.<g>.*`. Another game's
//! modules share labels with this one by design (both games define `GameHeader`), so
//! they are never candidates. Neither are [`SIGIL_PROBE_FIXTURES`]: modules the aeon
//! tree carries FOR sigil's own negative-probe tests, frozen copies of a live object
//! that define its labels by construction and are never part of any build.
//!
//! A label row nothing in scope defines is refused by name (`[map.order-orphan]`; a
//! `section:` row is left to `[map.order-unknown-section]`, which owns it): a map
//! row whose module was deleted without the row is a stale map, and dropping it
//! silently would hide exactly the edit this derivation exists to make safe. A row
//! two in-scope modules define (a test fixture that copies a shipped object) is
//! decided by the program's own `use` graph: the candidate the other roots already
//! reach is the one the build places. If none or several are reached, the row is
//! refused as `[map.order-ambiguous]`.
//!
//! ── WHAT THE MAP DOES NOT SAY: THE SHAPE GATES ──
//!
//! `order` is the UNION over every shape a map serves; each shape's placed sequence
//! is a subsequence of it. The map does not say which rows a given shape drops, and
//! for these modules neither does their source (they emit their bytes
//! unconditionally and are simply not rooted). [`SHAPE_GATES`] is that residue: the
//! modules only some shapes place, and the shape predicate each one rides. A gate
//! row is a FILTER over the derived set, so a gated module the tree no longer has is
//! inert here (deleting one needs no change in this file). The rows it still costs
//! are the ones for a NEW shape-conditional module, which must be added here, and a
//! RENAMED one, whose new id no row matches. The home that retires this table is a
//! per-row `when` in `map.toml`'s `order` (the vocabulary `[[anchor]]`/`[[hole]]`
//! already carry), which the game owns.
//!
//! ── WHAT THE MAP DOES NOT SAY: THE ENGINE TERMINUS ──
//!
//! `engine.epilogue` places `EndOfRom`, the zero-byte ROM terminus every build needs
//! (the header's ROM-end word, the fault handler's blob-end contract and the deb2
//! appendix all read it). The sonic4 map carries its row; the demo map does not
//! (its header excludes zero-byte markers from `order`), and nothing `use`s the
//! module. It is rooted unconditionally as [`ENGINE_TERMINUS`], an engine contract
//! rather than a game module list, beside the entry's other engine seeds
//! (`engine.game_contract`, `engine.ram`).

use std::collections::{HashMap, HashSet, VecDeque};

use sigil_frontend_emp::ast;
use sigil_frontend_emp::resolve::imports::use_decls;
use sigil_frontend_emp::resolve::manifest::Manifest;

use crate::map_placement::section_row;

/// One placed `.emp` module and one section it declares.
///
/// The `module_id` is a root of the build's synthetic entry. The `section` is one
/// section name the module places (a module that declares two sections contributes
/// two specs); it is what a `[[hole]]`'s `filled_by` is checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleSpec {
    pub module_id: String,
    pub section: String,
}

/// The shape a build makes, as the gates read it.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub debug: bool,
    pub crash_report: bool,
    pub sound_on: bool,
    /// `SOUND_DEBUG_HOTKEYS == 1` in the shape's comptime defines.
    pub sound_debug_hotkeys: bool,
    /// `SOUND_DBG_MIRROR == 1` in the shape's comptime defines.
    pub sound_dbg_mirror: bool,
}

/// The shape predicate a gated module rides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeGate {
    /// DEBUG equipment: placed only when `debug`.
    Debug,
    /// The MD Debugger island: placed when `debug || crash_report`.
    FaultIsland,
    /// The lean loud-failure handler, the other arm of the island's `if`.
    LeanFault,
    /// Sound-on only (the sound caller and the Z80 banks).
    SoundOn,
    /// Sound-off only (the Z80 idle program).
    SoundOff,
    /// The `SOUND_DEBUG_HOTKEYS` dev opt-in.
    SoundDebugHotkeys,
    /// The `SOUND_DBG_MIRROR` dev opt-in.
    SoundDbgMirror,
}

impl ShapeGate {
    /// Does `shape` place a module this gate rides?
    pub fn admits(self, shape: &Shape) -> bool {
        match self {
            ShapeGate::Debug => shape.debug,
            ShapeGate::FaultIsland => shape.debug || shape.crash_report,
            ShapeGate::LeanFault => !(shape.debug || shape.crash_report),
            ShapeGate::SoundOn => shape.sound_on,
            ShapeGate::SoundOff => !shape.sound_on,
            ShapeGate::SoundDebugHotkeys => shape.sound_debug_hotkeys,
            ShapeGate::SoundDbgMirror => shape.sound_dbg_mirror,
        }
    }
}

/// THE SHAPE-GATE RESIDUE: modules only some shapes place, keyed by module id. See
/// the module header for why this is not in the map yet and what retires it.
pub const SHAPE_GATES: &[(&str, ShapeGate)] = &[
    // DEBUG equipment (owner ruling 2026-08-05: a harness you drive is equipment, and
    // equipment does not ship). Each emits its procs unconditionally; plain never
    // roots them.
    ("engine.compression_selftest", ShapeGate::Debug),
    ("games.sonic4.object_test_state", ShapeGate::Debug),
    ("games.sonic4.test_player", ShapeGate::Debug),
    ("games.sonic4.test_enemy", ShapeGate::Debug),
    ("games.sonic4.test_animated", ShapeGate::Debug),
    ("games.sonic4.test_particle", ShapeGate::Debug),
    ("games.sonic4.test_emitter", ShapeGate::Debug),
    ("games.sonic4.test_parent", ShapeGate::Debug),
    ("games.sonic4.test_stress_emitter", ShapeGate::Debug),
    ("games.sonic4.test_churn", ShapeGate::Debug),
    ("games.sonic4.particle_anims", ShapeGate::Debug),
    // The fault-handler split (crash-report ruling, owner 2026-08-04): the island
    // under `debug || crash_report`, the lean handler under the `else`.
    (ERROR_HANDLER_MODULE_ID, ShapeGate::FaultIsland),
    (RELEASE_FAULT_MODULE_ID, ShapeGate::LeanFault),
    // Sound-on only: the sound caller and the Z80 bank sections.
    ("engine.sound_api", ShapeGate::SoundOn),
    ("games.sonic4.dac_banks", ShapeGate::SoundOn),
    // The bank-1 song bank carries one of two ids: `mt_bank_blob`, or `mt_bank`, the
    // id its path (`games/sonic4/data/sound/mt_bank.emp`) names. Both rows gate it, so
    // no sound-off shape roots the bank under either id.
    ("games.sonic4.mt_bank_blob", ShapeGate::SoundOn),
    ("games.sonic4.mt_bank", ShapeGate::SoundOn),
    ("games.sonic4.sfx_bank_blob", ShapeGate::SoundOn),
    ("games.sonic4.soundbankhead", ShapeGate::SoundOn),
    ("games.sonic4.song_bank2", ShapeGate::SoundOn),
    // Sound-off only: the Z80 idle program (the map's `[[hole]]` it fills is
    // `when = "sound_off"` too).
    ("engine.z80_init", ShapeGate::SoundOff),
    // The Config-A dev opt-ins, canonically empty and unrooted in every shipped shape.
    ("games.sonic4.game_debug", ShapeGate::SoundDebugHotkeys),
    ("engine.debug.sound_debug", ShapeGate::SoundDbgMirror),
];

/// The MD Debugger island module.
pub const ERROR_HANDLER_MODULE_ID: &str = "engine.debug.error_handler";

/// The lean shape's loud-failure fault handler, the island's other arm.
pub const RELEASE_FAULT_MODULE_ID: &str = "engine.system.release_fault";

/// Modules the aeon tree carries for sigil's own tests, never a build's roots.
///
/// `games.sonic4.sigil_objroutine_probe` is the frozen copy of the solid test object
/// that `crates/sigil-cli/tests/tranche6_negative_probes.rs` compiles standalone (its
/// header says so and forbids it to grow). It defines `TestSolid_Init`, the head label
/// of the live `test_solid` section, by construction. Excluding it here is sigil
/// naming its own fixture, not a game module list: the game never places it.
pub const SIGIL_PROBE_FIXTURES: &[&str] = &["games.sonic4.sigil_objroutine_probe"];

/// The ROM terminus module, rooted in every build. See the module header.
pub const ENGINE_TERMINUS: &str = "engine.epilogue";

/// The gate a module rides, or `None` for a module every shape places.
pub fn gate_of(module_id: &str) -> Option<ShapeGate> {
    SHAPE_GATES.iter().find(|(id, _)| *id == module_id).map(|(_, g)| *g)
}

/// Every section name `file` declares: the header's `in <name>` and each nested
/// `section <name> { … }`. A module that declares neither lands its bytes in the
/// default `text` section.
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

/// The label-defining item names of `items` (`pub` or not), recursing into nested
/// sections. Consts, types and equates are not labels, so a row never resolves to one.
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

/// The module a compiler-minted label (`__align$<module>$0`) names, if `row` is one.
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

/// The `use` closure of `seeds` over `manifest`, as module indices. An unknown id is
/// skipped here; `build_program` reports it against the `use` that names it.
fn use_closure(manifest: &Manifest, seeds: &[usize]) -> HashSet<usize> {
    let mut seen: HashSet<usize> = seeds.iter().copied().collect();
    let mut queue: VecDeque<usize> = seeds.iter().copied().collect();
    while let Some(i) = queue.pop_front() {
        for u in use_decls(&manifest.modules[i].file.items) {
            let target = u.base.segments.join(".");
            if let Some(&j) = manifest.by_id.get(&target) {
                if seen.insert(j) {
                    queue.push_back(j);
                }
            }
        }
    }
    seen
}

/// The lookups an `order` row resolves through, over one game's in-scope modules.
pub struct RowIndex<'m> {
    manifest: &'m Manifest,
    game_scope: String,
    by_label: HashMap<String, Vec<usize>>,
    by_section: HashMap<String, Vec<usize>>,
}

impl<'m> RowIndex<'m> {
    /// Index `manifest`'s modules under `engine.*` and `<game_prefix>.*`, minus
    /// [`SIGIL_PROBE_FIXTURES`].
    pub fn new(manifest: &'m Manifest, game_prefix: &str) -> RowIndex<'m> {
        let mut index = RowIndex {
            manifest,
            game_scope: format!("{game_prefix}."),
            by_label: HashMap::new(),
            by_section: HashMap::new(),
        };
        for (i, pm) in manifest.modules.iter().enumerate() {
            if !index.in_scope(&pm.id) {
                continue;
            }
            let mut labels = Vec::new();
            item_labels(&pm.file.items, &mut labels);
            for l in labels {
                let owners = index.by_label.entry(l).or_default();
                if !owners.contains(&i) {
                    owners.push(i);
                }
            }
            for sec in declared_sections(&pm.file) {
                index.by_section.entry(sec).or_default().push(i);
            }
        }
        index
    }

    /// Is `id` a module this game's rows may name?
    pub fn in_scope(&self, id: &str) -> bool {
        (id.starts_with("engine.") || id.starts_with(&self.game_scope))
            && !SIGIL_PROBE_FIXTURES.contains(&id)
    }

    /// The manifest indices of every in-scope module that defines `row`, before any
    /// shape gate. Empty means the row names nothing.
    pub fn candidates(&self, row: &str) -> Vec<usize> {
        if let Some(sec) = section_row(row) {
            self.by_section.get(sec).cloned().unwrap_or_default()
        } else if let Some(owner) = minted_owner(row) {
            match self.manifest.by_id.get(owner) {
                Some(&i) if self.in_scope(owner) => vec![i],
                _ => Vec::new(),
            }
        } else {
            self.by_label.get(row).cloned().unwrap_or_default()
        }
    }

    /// The module ids [`Self::candidates`] names.
    pub fn owners(&self, row: &str) -> Vec<&'m str> {
        self.candidates(row).into_iter().map(|i| self.manifest.modules[i].id.as_str()).collect()
    }
}

/// Derive the build's module roots from `order` (the game map's `order` array) over
/// `manifest` (the scanned tree).
///
/// * `game_prefix` is `games.<g>`, the building game's module-id prefix.
/// * `seeds` are the entry's fixed engine/game seeds (contract, manifest module, RAM
///   modules); they take part in the `use` closure that settles an ambiguous row.
/// * `map_name` names the map in diagnostics.
///
/// Roots come out in map order (first row that names each module), then
/// [`ENGINE_TERMINUS`] if no row named it. Each root yields one [`ModuleSpec`] per
/// section it declares.
pub fn derive_module_roots(
    manifest: &Manifest,
    order: &[String],
    game_prefix: &str,
    shape: &Shape,
    seeds: &[&str],
    map_name: &str,
) -> Result<Vec<ModuleSpec>, String> {
    let index = RowIndex::new(manifest, game_prefix);
    let admitted = |i: usize| gate_of(&manifest.modules[i].id).is_none_or(|g| g.admits(shape));

    let mut orphans: Vec<String> = Vec::new();
    // (row, admitted candidates) for rows more than one module defines.
    let mut pending: Vec<(String, Vec<usize>)> = Vec::new();
    // (row index, module) for every row that names exactly one admitted module.
    let mut picked: Vec<(usize, usize)> = Vec::new();

    for (ri, row) in order.iter().enumerate() {
        let candidates = index.candidates(row);
        if candidates.is_empty() {
            // A `section:` row that names no section is refused later, by the placement
            // check that owns that row kind (`[map.order-unknown-section]`), with the
            // same row in its text. Refusing it here too would put two diagnostics on one
            // mistake; a LABEL row has no such later owner, so it is refused here.
            if section_row(row).is_none() {
                orphans.push(row.clone());
            }
            continue;
        }
        let live: Vec<usize> = candidates.into_iter().filter(|&i| admitted(i)).collect();
        match live.len() {
            // Every module that defines the row is gated out of this shape.
            0 => {}
            1 => picked.push((ri, live[0])),
            _ => pending.push((row.clone(), live)),
        }
    }

    if !orphans.is_empty() {
        return Err(format!(
            "[map.order-orphan] {} `order` row(s) in `{map_name}` name nothing a module under \
             `engine.*` or `{game_prefix}.*` defines (no label of that name, no section of \
             that name, no module a compiler-minted label names). A row names the section a \
             module places, so a row with no module is a stale map: delete the row with its \
             module, or restore the module:\n  - {}",
            orphans.len(),
            orphans.join("\n  - ")
        ));
    }

    // An ambiguous row is settled by the `use` graph of everything else the build roots.
    if !pending.is_empty() {
        let mut seed_idx: Vec<usize> = seeds.iter().filter_map(|s| manifest.by_id.get(*s).copied()).collect();
        seed_idx.extend(picked.iter().map(|&(_, m)| m));
        if let Some(&t) = manifest.by_id.get(ENGINE_TERMINUS) {
            seed_idx.push(t);
        }
        let reached = use_closure(manifest, &seed_idx);
        let mut ambiguous: Vec<String> = Vec::new();
        for (row, live) in &pending {
            let hit: Vec<usize> = live.iter().copied().filter(|i| reached.contains(i)).collect();
            if hit.len() == 1 {
                let ri = order.iter().position(|r| r == row).expect("pending row came from order");
                picked.push((ri, hit[0]));
            } else {
                let names: Vec<&str> = live.iter().map(|&i| manifest.modules[i].id.as_str()).collect();
                ambiguous.push(format!(
                    "`{row}` is defined by {} and the rest of the build reaches {} of them",
                    names.join(", "),
                    if hit.is_empty() { "none".to_string() } else { format!("{}", hit.len()) }
                ));
            }
        }
        if !ambiguous.is_empty() {
            return Err(format!(
                "[map.order-ambiguous] {} `order` row(s) in `{map_name}` name a label more than one \
                 in-scope module defines, and the program's `use` graph does not pick one. Rename \
                 one definition, or make the intended module reachable:\n  - {}",
                ambiguous.len(),
                ambiguous.join("\n  - ")
            ));
        }
        picked.sort_by_key(|&(ri, _)| ri);
    }

    let mut roots: Vec<usize> = Vec::new();
    for (_, m) in picked {
        if !roots.contains(&m) {
            roots.push(m);
        }
    }
    match manifest.by_id.get(ENGINE_TERMINUS) {
        Some(&t) if !roots.contains(&t) => roots.push(t),
        Some(_) => {}
        None => {
            return Err(format!(
                "[map.no-terminus] the tree has no `{ENGINE_TERMINUS}` module. Every build roots \
                 it for `EndOfRom`, the ROM terminus the header, the fault handler and the symbol \
                 appendix read"
            ))
        }
    }

    Ok(roots
        .into_iter()
        .flat_map(|i| {
            let pm = &manifest.modules[i];
            declared_sections(&pm.file)
                .into_iter()
                .map(move |section| ModuleSpec { module_id: pm.id.clone(), section })
        })
        .collect())
}
