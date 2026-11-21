//! The build's verdicts on a placed program: the drift-guard verdict and the
//! inapplicable-guard allowlist, the guard census, the post-resolve placement contract
//! against the map (anchors, the driving `order`, holes), the `[[budget]]` rows, the
//! per-artifact region check, and what a check-only run reports ([`CheckReport`]).
//!
//! Moved from sigil-harness (`native.rs`, `seam2.rs`) unchanged by G0b Task 4. The sound
//! fold (`check_sound_fold`, seam-2's bank heads) stays in sigil-harness with the seams;
//! the resolved-alignment check is [`crate::align::AlignTable::validate_resolved`].

use sigil_ir::Section;

use crate::diag::{BuildWarning, ChainSources};
use crate::place::{head_label, is_rom_section, ModuleSpec};

/// Enforce that the observed inapplicable (Poison-unresolvable) drift guards are
/// EXACTLY the caller-supplied allowlist — both directions (§t24). The caller homes the
/// set (the game-as-data driver passes its own; each shape could home a different twin
/// subset). `inapplicable` are the "not defined in this link" diagnostics;
/// `link_asserts` supplies each guard's own message (its `.emp` site) via span match.
/// The declared-chain verdict over `check_link_asserts`' output. A guard that
/// folded and failed is REAL drift: every such guard is rendered at its own line
/// through the build's one renderer, and the build fails with all of them. A
/// guard whose `extern()` names a symbol this link does not define is
/// INAPPLICABLE (a gated-off twin) and is handed back for the allowlist check.
pub fn declared_chain_drift_verdict<'a>(
    adiags: &'a [sigil_span::Diagnostic],
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> Result<Vec<&'a sigil_span::Diagnostic>, String> {
    // An `extern()` naming a symbol nothing in this link defines is refused at the
    // reference, before the guard partition: it is never an inapplicable twin.
    let unknown: Vec<&sigil_span::Diagnostic> = adiags
        .iter()
        .filter(|d| {
            d.level == sigil_span::Level::Error && d.message.starts_with(sigil_link::EXTERN_UNKNOWN_ID)
        })
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "extern() names a symbol no module in this link defines: {} error(s):\n{}",
            unknown.len(),
            crate::diag::render::render_diag_lines(&unknown, locate)
        ));
    }
    let (inapplicable, real): (Vec<_>, Vec<_>) = adiags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .partition(|d| d.message.contains("not defined in this link"));
    if !real.is_empty() {
        return Err(format!(
            "declared-chain drift guard FIRED: {} error(s):\n{}",
            real.len(),
            crate::diag::render::render_diag_lines(&real, locate)
        ));
    }
    Ok(inapplicable)
}

pub fn enforce_inapplicable_allowlist_against(
    inapplicable: &[&sigil_span::Diagnostic],
    link_asserts: &[sigil_ir::LinkAssert],
    allowlist: &[(&str, &str)],
) -> Result<(), String> {
    // The guard's own message text (its Text parts), for the site match.
    let site_of = |d: &sigil_span::Diagnostic| -> String {
        link_asserts
            .iter()
            .find(|a| a.span == d.primary)
            .map(|a| {
                a.message
                    .iter()
                    .filter_map(|p| match p {
                        sigil_ir::MsgPart::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect::<String>()
            })
            .unwrap_or_default()
    };

    let mut matched = vec![false; allowlist.len()];
    for d in inapplicable {
        // The unresolved extern name(s) are the backtick-delimited tokens in the
        // Poison message ("references symbol(s) `X` not defined in this link").
        let externs: Vec<&str> = d.message.split('`').skip(1).step_by(2).collect();
        let site = site_of(d);
        let hit = allowlist.iter().enumerate().find(|(i, (name, token))| {
            !matched[*i] && externs.contains(name) && site.contains(token)
        });
        match hit {
            Some((i, _)) => matched[i] = true,
            None => {
                return Err(format!(
                    "unresolvable-extern drift guard NOT in the allowlist, either the \
                     extern name is wrong or a new twin-parity guard needs a ruling.\n  \
                     externs: {externs:?}\n  site: {site}"
                ));
            }
        }
    }
    if let Some(i) = matched.iter().position(|m| !m) {
        return Err(format!(
            "inapplicable-guard allowlist is STALE: the guard {:?} \
             (extern, site) no longer folds to an unresolvable-extern Poison, it now resolves \
             (or drifted to a Value(0) fail). Re-verify and update the allowlist.",
            allowlist[i],
        ));
    }
    if std::env::var("NATIVE_DEBUG").is_ok() {
        eprintln!(
            "NATIVE_DEBUG: {} inapplicable drift guard(s) matched the allowlist exactly",
            inapplicable.len()
        );
    }
    Ok(())
}

/// The post-resolve placement contract: the resolved layout against the shape's
/// declared `map.toml` — island anchors, the driving `order`, and both halves of every
/// `[[hole]]` (its `after` anchor is present, and its reserved interior is empty but for
/// the module `filled_by` names).
///
/// `registry` is the shape's module list — the same `profile.registry` the build places
/// from. The hole's interior half needs it to turn `filled_by`'s MODULE id into the set
/// of section names permitted inside the reserved span; taking that set from the
/// resolved layout instead would assert only that the layout agrees with itself.
pub fn validate_placement(
    resolved: &[Section],
    pmap: &crate::map_placement::PlacementMap,
    sound_on: bool,
    registry: &[ModuleSpec],
) -> Result<(), String> {
    const ANCHOR_GAP: u32 = 0x400;
    // ROM sections, lma-sorted, with (head_label, section_name, lma, byte_len, is_phase_bank).
    let mut rows: Vec<(String, &str, u32, usize, bool)> = resolved
        .iter()
        .filter(|s| is_rom_section(s))
        .map(|s| {
            let id = head_label(s).unwrap_or_default().to_string();
            let pb = s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
            (id, s.name.as_str(), s.lma, s.image_bytes().len(), pb)
        })
        .collect();
    rows.sort_by_key(|r| r.2);

    // ── Anchors: every anchor in force holds its section at its address ──
    //
    // An island is exactly the section an anchor in force HOLDS (`[[anchor]] holds`, the
    // walk's rule, `crate::walk`): it must sit at the anchor's `at`. G0b final review A-I1:
    // this check used to recover the islands from the layout by gap (a section starting
    // more than `ANCHOR_GAP` past the previous section's end, or a phase bank), so a run
    // that ended 0 to `ANCHOR_GAP` bytes short of a non-bank anchor (content the walk had
    // placed correctly) was refused as `[map.anchor-absent]`, blaming the map. An anchor
    // with no `holds` (an overlay row, a test map) is checked by address alone: some ROM
    // section starts at its `at`.
    let held_by = |a: &crate::map_placement::Anchor, id: &str, name: &str| match &a.holds {
        Some(h) => h == id || *h == crate::map_placement::section_row_key(name),
        None => true,
    };
    for a in pmap.anchors_for(sound_on) {
        if !rows.iter().any(|(id, name, lma, _, _)| *lma == a.at && held_by(a, id, name)) {
            let what = match &a.holds {
                Some(h) => format!("the section it holds, `{h}`, is not at that address in this build"),
                None => "no ROM section starts at that address in this build".to_string(),
            };
            return Err(format!(
                "[map.anchor-absent] declared anchor `{}` at {:#X}: {what}, the layout no longer anchors it (stale map or shape gate)",
                a.name, a.at
            ));
        }
    }
    // ── Undeclared islands: a gap the map does not declare ──
    //
    // The converse, by inference: a section the layout leaves a gap of more than
    // `ANCHOR_GAP` in front of (or a phase bank, or the run head) is an island, and an
    // island needs an `[[anchor]]` at its address.
    let declared: std::collections::HashSet<u32> = pmap.anchors_for(sound_on).map(|a| a.at).collect();
    let mut prev_end: Option<u32> = None;
    for (_, _, lma, len, pb) in &rows {
        let is_island = prev_end.is_none() || *pb || *lma > prev_end.unwrap() + ANCHOR_GAP;
        if is_island && !declared.contains(lma) {
            return Err(format!(
                "[map.undeclared-island] ROM section at {lma:#X} is an ANCHOR_GAP-inferred island but no `[[anchor]] at = {lma:#X}` is declared, add it to the placement map"
            ));
        }
        prev_end = Some(lma.saturating_add(*len as u32));
    }

    // ── Order (K5 DRIVE-CONFIRMATION): every byte-emitting section must be DECLARED in
    //    `order` (completeness — the map drives, so it must name every emitter), and the
    //    resolved byte-emitting sequence must follow the declared positions strictly. ──
    //
    //    A row spelled `section:<name>` declares a section by its NAME (the `module … in
    //    <name>` target) instead of its head label — the only authorable spelling for a
    //    section whose head label is content-derived. It ranks exactly where the row sits;
    //    a zero-byte section it names is inert (a reserved slot, authorable before the
    //    content exists); a name absent from this build, or a section declared both ways,
    //    is rejected. The `[map.order-undeclared]` text names the head label as before —
    //    aeon fixtures assert on it.
    if !pmap.order.is_empty() {
        let pos: std::collections::HashMap<&str, usize> =
            pmap.order.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
        for row in &pmap.order {
            if let Some(name) = crate::map_placement::section_row(row) {
                if !rows.iter().any(|r| r.1 == name) {
                    return Err(format!(
                        "[map.order-unknown-section] `order` row `{row}` names no ROM section in this build, the name is the `module … in <name>` target; fix the spelling or drop the row"
                    ));
                }
            }
        }
        for (id, name, _, _, _) in &rows {
            let key = crate::map_placement::section_row_key(name);
            if !id.is_empty() && pos.contains_key(id.as_str()) && pos.contains_key(key.as_str()) {
                return Err(format!(
                    "[map.order-double-declared] section `{name}` is declared twice in `order`, by its head label `{id}` and by `{key}`; keep exactly one row"
                ));
            }
        }
        let mut last: Option<(usize, String)> = None;
        for (id, name, _, len, _) in &rows {
            if *len == 0 {
                continue;
            }
            // The row this section is declared by: its `section:` row when one exists,
            // else its head label (a label-less blob is exempt, as before).
            let key = crate::map_placement::section_row_key(name);
            let declared_as = if pos.contains_key(key.as_str()) { key } else { id.clone() };
            if declared_as.is_empty() {
                continue;
            }
            let Some(&p) = pos.get(declared_as.as_str()) else {
                return Err(format!(
                    "[map.order-undeclared] byte-emitting section `{id}` is not in the declared `order`, the map DRIVES placement now, so every emitter must be declared; add it in its layout position"
                ));
            };
            if let Some((lp, lid)) = &last {
                if p <= *lp {
                    return Err(format!(
                        "[map.order-diverged] the resolved layout places `{declared_as}` after `{lid}`, but the declared `order` has `{declared_as}` before it, the packer did not honour the driving order (packer bug)"
                    ));
                }
            }
            last = Some((p, declared_as));
        }
    }

    // ── Holes, first half: the `after` anchor label must resolve ──
    //
    // PRESENCE. This arm proves the hole has a subject, and it runs first so a hole with
    // no left edge is refused by the name aeon fixtures assert on
    // (`[map.hole-anchor-missing]`) rather than by the interior half's own
    // `[map.hole-anchor-unresolved]`.
    for h in pmap.holes_for(sound_on) {
        let present = resolved.iter().any(|s| s.labels.iter().any(|l| l.name == h.after));
        if !present {
            return Err(format!(
                "[map.hole-anchor-missing] declared hole after `{}` (at {:#X}), its `after` label is not in the resolved layout",
                h.after, h.at
            ));
        }
    }

    // ── Holes, second half: the reserved interior is empty but for its filler ──
    //
    // The `negative` half — see [`hole_interior_faults`], which owns the derivation and
    // the five refusals it makes instead of a silent pass. Every fault it finds is a
    // placement violation, so they are reported together rather than one at a time: a
    // stale `at` that swallows three sections is one declaration to correct, and a
    // caller shown only the first would fix it three times.
    let faults = hole_interior_faults(resolved, pmap, sound_on, registry)?;
    if !faults.is_empty() {
        return Err(faults.join("\n"));
    }
    Ok(())
}

/// THE HOLE'S INTERIOR IS RESERVED: over one shape's resolved layout, every
/// byte-emitting section that occupies part of a declared `[[hole]]` without being the
/// module that hole declares as its filler.
///
/// A `[[hole]]` is a reserved empty span: it opens at its `after` label, runs to `at`
/// (the address the post-hole data resumes at), and the module named by `filled_by`
/// is the one thing allowed inside it. This is the only reader of `at` as a BOUND and
/// the only reader of `filled_by` at all, so without it a packed layout that puts
/// another emitter in that span, or a declaration whose `at` has drifted away from the
/// layout, produces a ROM whose post-hole data no longer begins where the map says it
/// does, with no build diagnostic of any kind.
///
/// [`validate_placement`] calls this on the shipped ROM build path, so its faults are
/// build errors.
///
/// THE PERMITTED SET IS DERIVED, NEVER TRANSCRIBED. `filled_by` is a MODULE id
/// (`engine.z80_init`); the sections it may occupy come from `registry` — the module
/// list the build is handed, upstream of every section it goes on to place. So this
/// cannot drift from the build's own idea of which section belongs to that module.
///
/// LOUD WHEN IT CANNOT MEASURE (`Err`), never a silent pass, because a hole check that
/// passes by finding nothing to check is the exact defect it exists to close:
///
/// * `[map.hole-anchor-unresolved]` — the `after` label is in no resolved section (the
///   hole has no left edge). This is the one tag the shipped path cannot reach:
///   `validate_placement`'s presence arm refuses the same layout first, by name, with
///   `[map.hole-anchor-missing]`. It keeps the function honest when it is driven
///   directly.
/// * `[map.hole-anchor-ambiguous]` — the `after` label resolves in more than one
///   section, so the hole opens at two different addresses.
/// * `[map.hole-bounds-degenerate]` — `at` is at or before the `after` label, so the
///   declared interior is empty and nothing inside it can be judged.
/// * `[map.hole-filler-unknown]` — `filled_by` names no module in this shape's
///   registry, so the permitted set is empty and every occupant, including the intended
///   filler, would read as an intruder.
/// * `[map.hole-filler-absent]` — the filler's sections are all absent from the resolved
///   layout, so the hole is not filled by the thing that is supposed to fill it.
///
/// `Ok(vec![])` over a shape whose `when` gates every hole out is a correct empty
/// answer, not coverage: a CALLER claiming "no hole is violated" must first establish
/// that some shape declares a live hole (see the population guard in this function's
/// gate).
pub fn hole_interior_faults(
    resolved: &[Section],
    pmap: &crate::map_placement::PlacementMap,
    sound_on: bool,
    registry: &[ModuleSpec],
) -> Result<Vec<String>, String> {
    let mut faults = Vec::new();
    for h in pmap.holes_for(sound_on) {
        // ── The hole's left edge: where the `after` label actually resolved ──
        let mut sites = resolved.iter().filter_map(|s| {
            s.labels.iter().find(|l| l.name == h.after).map(|l| s.lma + l.offset)
        });
        let Some(start) = sites.next() else {
            return Err(format!(
                "[map.hole-anchor-unresolved] declared hole after `{}` (at {:#X}) has no left edge, its `after` label is in no resolved section, so the span it reserves cannot be measured and NOTHING about the hole was checked",
                h.after, h.at
            ));
        };
        if let Some(other) = sites.next() {
            return Err(format!(
                "[map.hole-anchor-ambiguous] declared hole after `{}` (at {:#X}) opens at two addresses ({start:#X} and {other:#X}), one label resolved in more than one section, so the reserved span has no single left edge",
                h.after, h.at
            ));
        }
        if h.at <= start {
            return Err(format!(
                "[map.hole-bounds-degenerate] declared hole after `{}` opens at {start:#X} but declares `at = {:#X}`, which is at or before it, the declared interior is empty, so this hole can never refuse anything and reads as checked while checking nothing",
                h.after, h.at
            ));
        }

        // ── The permitted occupants: the filler module's sections, from the registry ──
        let permitted: Vec<&str> =
            registry.iter().filter(|m| m.module_id == h.filled_by).map(|m| m.section.as_str()).collect();
        if permitted.is_empty() {
            return Err(format!(
                "[map.hole-filler-unknown] declared hole after `{}` (at {:#X}) is filled_by `{}`, which names no module in this shape's registry, the permitted-occupant set is empty, so the intended filler itself would read as an intruder and the answer would be a fault about the wrong thing",
                h.after, h.at, h.filled_by
            ));
        }
        if !resolved.iter().any(|s| permitted.contains(&s.name.as_str())) {
            return Err(format!(
                "[map.hole-filler-absent] declared hole after `{}` (at {:#X}) is filled_by `{}` (section(s) {:?}), and this build placed none of them, the hole's filler is not in the layout, so an empty interior would mean the hole is UNFILLED rather than reserved",
                h.after, h.at, h.filled_by, permitted
            ));
        }

        // ── The occupants: every byte-emitting ROM section overlapping [start, at) ──
        for s in resolved.iter().filter(|s| is_rom_section(s)) {
            let len = s.image_bytes().len() as u32;
            if len == 0 || permitted.contains(&s.name.as_str()) {
                continue;
            }
            let (lo, hi) = (s.lma.max(start), s.lma.saturating_add(len).min(h.at));
            if lo >= hi {
                continue;
            }
            faults.push(format!(
                "[map.hole-interior-occupied] the hole declared after `{}`, interior [{start:#X},{:#X}), reserved for `{}`, is occupied at [{lo:#X},{hi:#X}) by byte-emitting section `{}` (head `{}`). Either that section drifted into the reserved span, or the hole's declared `at` no longer matches this layout; either way the post-hole data does not resume at {:#X} the way the map says it does.",
                h.after,
                h.at,
                h.filled_by,
                s.name,
                head_label(s).unwrap_or("<label-less>"),
                h.at,
            ));
        }
    }
    Ok(faults)
}

/// How many guards a resolve decided, by family. Every figure is OBSERVED at the
/// moment the guard decided, never a source scan and never one count subtracted
/// from another: the comptime count the evaluator increments per verdict, and the
/// link's own [`sigil_link::LinkAssertTally`]. A family that decided nothing reads
/// as zero, and a guard that decided nothing is counted in neither column.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardCensus {
    /// `ensure` evaluations that reached a comptime verdict.
    pub comptime_guards: usize,
    /// `LinkAssert` conditions the link WATCHED fold to a value, pass or fail.
    /// Read straight off the tally's `decided`, so a condition that folded to
    /// nothing at all cannot arrive here as a decided guard.
    pub link_asserts_decided: usize,
    /// `LinkAssert` conditions whose fold named a symbol this link does not
    /// define (a gated-off twin); neither passed nor failed, and each one is on
    /// the profile's allowlist or the resolve would have failed.
    ///
    /// NOT an undefined `extern()`: that is refused at the reference itself
    /// ([`sigil_link::EXTERN_UNKNOWN_ID`]) and fails the drift verdict before any
    /// census is built, so it never reaches this column. What does reach it is a
    /// residual `Sym` leaf with no `extern()` reference record beside it, which
    /// today means `bankid()`, `winptr()`, or an immediate-normalized label
    /// naming a section outside this profile's closure.
    pub link_asserts_inapplicable: usize,
}

impl GuardCensus {
    /// The census from a resolve's records: the comptime count the lowering
    /// drained, the link's OWN observation of what each `LinkAssert` condition
    /// did ([`sigil_link::LinkAssertTally`]), and the inapplicable subset the
    /// drift verdict handed back.
    ///
    /// `link_asserts_decided` is `tally.decided`: the count the fold incremented
    /// when it produced a value. It is NOT `conditions - inapplicable`. The
    /// difference is what a subtraction cannot see: a condition that folded to
    /// NOTHING (poisoned, faulted, or skipped because its every unresolved leaf
    /// was already refused at its `extern()` reference) is neither decided nor
    /// inapplicable, and subtraction reports it as decided.
    ///
    /// Two reconciliations, both hard errors rather than a quiet wrong number:
    ///
    /// 1. The tally's own bucket identity (`decided + undecided == conditions`),
    ///    false only on a counting bug inside the walk.
    /// 2. Every diagnostic the verdict partitioned as inapplicable must carry the
    ///    span of a condition the link RECORDED unresolvable, matched one-to-one.
    ///    The partition is a text match on "not defined in this link" over a
    ///    different code path; this is where the two paths are made to agree
    ///    about what a condition is, instead of being assumed to.
    pub fn from_verdict(
        comptime_guards: usize,
        tally: &sigil_link::LinkAssertTally,
        inapplicable: &[&sigil_span::Diagnostic],
    ) -> Result<GuardCensus, String> {
        if !tally.accounted() {
            return Err(format!(
                "internal: the link-assert walk lost a condition: {} decided + {} undecided != {} \
                 conditions. The guard census cannot be reported from a walk that does not \
                 account for every condition it saw.",
                tally.decided, tally.undecided, tally.conditions
            ));
        }
        let mut unmatched: Vec<sigil_span::Span> = tally.unresolvable_spans.clone();
        for d in inapplicable {
            match unmatched.iter().position(|s| *s == d.primary) {
                Some(i) => {
                    unmatched.remove(i);
                }
                None => {
                    return Err(format!(
                        "internal: a diagnostic was partitioned as an INAPPLICABLE guard but the \
                         link recorded no unresolvable condition at its span, so it is not a \
                         gated-off twin. Its wording matched \"not defined in this link\" while \
                         coming from somewhere else, which would both hide a real verdict and \
                         miscount the census.\n  message: {}",
                        d.message
                    ));
                }
            }
        }
        Ok(GuardCensus {
            comptime_guards,
            link_asserts_decided: tally.decided,
            link_asserts_inapplicable: inapplicable.len(),
        })
    }
}

/// Render a declared-chain stage's failure: the stage name, the diagnostic
/// count, then every diagnostic located through the authority that can read its
/// span.
///
/// The stages this renders (`resolve_layout`, `link`) run over the CHAINED section
/// list — AS-side sections and `.emp` sections in one vector — so the authority is
/// [`ChainSources`] and not either front end's map alone. It took the `.emp`
/// manifest's index until 2026-09-18, which meant a diagnostic arising in an
/// AS-side section was located through the `.emp` index and named a different
/// file, confidently.
pub fn render_declared_chain(what: &str, d: &[sigil_span::Diagnostic], sources: &ChainSources) -> String {
    let all: Vec<&sigil_span::Diagnostic> = d.iter().collect();
    format!(
        "declared-chain: {what}: {} diag(s):\n{}",
        d.len(),
        crate::diag::render::render_diag_lines(&all, &|span| sources.locate(span))
    )
}

/// The object code bank's used cursor = the resolved LMA of the map-declared budget
/// cursor label (`cursor_head`, e.g. `DeformTable_Zero`) — the head of the first section
/// PAST the object bank, whose start IS the bank terminus (object code ends where the data
/// region begins; they pack contiguously). This is the map-owned successor to the retired
/// `engine.inc` `__BUDGET_DATA` marker (K4 inc-6B): the object bank and the data region
/// share the `[$10000,$20000)` window and the data region extends BEYOND it, so an LMA
/// window scan cannot separate them — only the declared boundary label can. `None` if the
/// label is absent from the resolved layout (a game that declares no cursor).
pub fn object_bank_cursor(resolved: &[Section], cursor_head: &str) -> Option<u32> {
    for s in resolved {
        for l in &s.labels {
            if l.name == cursor_head {
                return Some(s.lma.wrapping_add(l.offset));
            }
        }
    }
    None
}

/// Enforce EVERY `[[budget]]` row of the placement map: the row's cursor (the resolved
/// LMA of its declared head label, [`object_bank_cursor`]) must not exceed the row's
/// declared `ceiling`. Returns `(region, used bytes)` per row, in map order. This is the
/// map-owned successor to `engine.inc`'s `if * > $20000 / error`, generalised from the
/// one row spelled `object_bank` to every row the map declares.
///
/// A row that cannot be measured is refused (`[map.budget-unmeasurable]`), never read
/// as within budget: a row naming no map region, a row with no `cursor`, or a cursor
/// label the resolved layout does not define. A map with no `[[budget]]` row checks
/// nothing (`Ok(vec![])`). Runs on every native build (both the pinned canonical
/// driver and the off-canonical chainer feed it their resolved sections).
pub fn check_budgets(
    resolved: &[Section],
    map: &sigil_ir::map::MemoryMap,
    pmap: &crate::map_placement::PlacementMap,
) -> Result<Vec<(String, u32)>, String> {
    let mut used = Vec::with_capacity(pmap.budgets.len());
    for b in &pmap.budgets {
        let Some(head) = b.cursor.as_deref() else {
            return Err(format!(
                "[map.budget-unmeasurable] budget row `{}` declares no `cursor`, so its ceiling {:#X} cannot be measured",
                b.region, b.ceiling
            ));
        };
        let Some(cursor) = object_bank_cursor(resolved, head) else {
            return Err(format!(
                "[map.budget-unmeasurable] budget row `{}` cursor `{head}` is defined by no resolved section, so its ceiling {:#X} cannot be measured",
                b.region, b.ceiling
            ));
        };
        used.push((b.region.clone(), map.check_budget(&b.region, b.ceiling, cursor)?));
    }
    Ok(used)
}

/// The region check every seam-2 per-artifact tail ends in (`place_sections` +
/// `resolve_layout` + `link` never emit a ROM, so `emit_rom`'s check never ran on
/// them): each linked section the synthetic map PLACED (its group names one of the
/// map's regions) must fit that region ([`sigil_link::check_regions`]). The
/// cross-seam carriers the tails pin outside the map (`group = None`, at
/// `$1000000`+) are not the map's and are not checked against it.
pub fn check_placed_regions(linked: &sigil_link::LinkedImage, map: &sigil_ir::map::MemoryMap) -> Result<(), String> {
    let placed = sigil_link::LinkedImage {
        sections: linked
            .sections
            .iter()
            .filter(|s| s.region.as_deref().is_some_and(|g| map.region_by_name(g).is_some()))
            .cloned()
            .collect(),
    };
    sigil_link::check_regions(&placed, map)
}

/// What a check-only run reports: the guard census and the warn tier.
pub struct CheckReport {
    /// The guards the resolve decided, by family.
    pub guards: GuardCensus,
    /// Every non-error diagnostic the resolve produced, for the caller to render.
    pub warnings: Vec<BuildWarning>,
}
