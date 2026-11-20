//! The packing walk's generic half: measuring a section list at given pins (the
//! relaxation-exact length witness the walk iterates on), the declared-chain placement
//! that turns true bases and spans into Pinned/Chained sections, the align
//! recomputation a relocated pure-data section needs, and the section predicates every
//! pass shares.
//!
//! Moved from sigil-harness (`native.rs`, `seam2.rs`, `module_roots.rs`) unchanged by
//! G0b Task 4. The old walk and its alignment rule, which read the frozen `section_align`
//! table, were deleted with the profile-driven driver (G0b Task 11); the walk is
//! [`crate::walk`] and the alignment rule [`crate::align`].

use std::collections::HashMap;

use sigil_ir::{Fragment, Section, SectionPlacement, SymbolTable};


/// The FROZEN-source emp placement map: one nominal region per DISTINCT section name
/// present in the lowered set. Bases are COSMETIC (base 0, huge size) — the frozen
/// chainer overrides every base from the listing table, so `place_sections` here only
/// needs to give each section a home so it survives to the chainer. `place_sections`
/// does not enforce the size, so the huge size never overflows.
pub fn emp_map_frozen(sections: &[Section]) -> String {
    let mut names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let mut out = String::from("fill = 0x00\n");
    for n in names {
        out.push_str(&format!(
            "\n[[region]]\nname = \"{n}\"\nlma_base = 0x0\nsize = 0x400000\nkind = \"rom\"\n"
        ));
    }
    out
}

/// One measuring resolve's full witness: per-section image length, plus the lowered
/// length of every RELAXABLE fragment (keyed by stable section index, then by the
/// fragment's position among the section's relaxables, in order) — what the
/// non-convergence diagnostic reads the width-flipping sites off.
pub struct Measured {
    pub img: Vec<u32>,
    /// Per-section `(span, lowered length)` of each relaxable fragment, in order.
    pub sites: Vec<Vec<(sigil_span::Span, u32)>>,
}

/// Measure every ROM section's image length with each section pinned at
/// `pin_lma[idx]` — the EXACT length it takes at that base, whether or not the
/// pinned extents happen to intersect. Section lengths are relaxation-dependent (an
/// unsized `lea Table, a1` is abs.w when `Table` resolves under `$8000` on the
/// 24-bit bus, abs.l otherwise; a bare branch is `.s`/`.w` by distance), so the only
/// base a length is true at is the base the section will occupy — and the measuring
/// resolve (`resolve_layout_measuring`: the same placement⇄relaxation fixpoint with
/// the image-soundness checks skipped) makes that base measurable even while a grown
/// section's extent still runs into its successor's pin: label addresses are pin +
/// offset regardless of intersection, and the walk's next pack round moves the run
/// apart by exactly the lengths measured here.
///
/// THE CATCH THIS DESIGN REPLACES (2026-08-26, `player_sensors`): the previous rig
/// measured through the CHECKED resolve, so a colliding measuring layout had to be
/// re-measured at SUBSTITUTE bases — pure data at a scratch slot, else everything
/// under a cumulative +0x400/rank spread. A substitute base is a different address,
/// and a different address can select a different width: the scratch slots
/// (`0x70_0000 + k·0x10_0000`) wrap the 24-bit bus from k = 9, so `collision_data`
/// (slot 41 = `0x300_0000`, masked to `0x0` by `asl_width_rule`) looked
/// abs.w-reachable and twelve `lea` sites in `player_sensors` measured 4 B each
/// where the real base encodes 6. The walk packed the successor 24 B into the
/// section, lengths were "stable", and the build refused naming an innocent pair.
/// Measuring at the round's own pins has no substitute base to lie about — and no
/// spread step to outgrow (the old rig could not even measure a single section grown
/// past ~0x400).
///
/// A ROM section with NO pin measures at a FAR-SCRATCH slot ([`far_scratch_slot`], from
/// [`SCRATCH_FIRST`] in steps of [`SCRATCH_STRIDE`]): those are the sections outside the
/// frozen table, and the far slot is what reproduces asl's conservative widths for
/// references that touch them (asl encodes a forward reference abs.l; a near base would
/// relax it abs.w and the chained layout would settle tighter than the golden shapes).
///
/// A reference from a PINNED section into a never-pinned one takes its width from the
/// slot its target draws, so the slot address is a live measuring input for the pinned
/// sections as well. The slot a section draws is its ordinal among the never-pinned ROM
/// sections, and any never-pinned section ahead of it shifts that ordinal, a zero-byte
/// label-less one included (the `.emp` lowering opens one for a run of top-level items,
/// such as an `ensure` after a file's last `section {}` block). Every slot is therefore
/// one whose whole stride encodes abs.l on the 24-bit bus, so the ordinal decides no
/// width and moves no section.
pub fn measure_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
) -> Result<Measured, String> {
    lens_pinned(sections, pin_lma, true)
}

/// The first far-scratch slot address, above every ROM address a pinned section holds.
pub const SCRATCH_FIRST: u32 = 0x0070_0000;

/// The distance between consecutive far-scratch slots, and so the largest never-pinned
/// section a slot holds without running into the next slot.
pub const SCRATCH_STRIDE: u32 = 0x0010_0000;

/// The first far-scratch slot at or above `from` whose whole stride encodes abs.l.
///
/// A slot's addresses reach the width choice through [`sigil_ir::asl_width_rule`], which
/// masks to the 68000's 24-bit bus and selects abs.w in `[0, 0x7FFF]` and
/// `[0xFF_8000, 0xFF_FFFF]`. The raw slot addresses climb past that wrap every 16 slots,
/// and a slot whose masked range reaches either window would measure a reference into its
/// section SHORT (abs.w) where every other slot measures it abs.l, so such a slot is
/// skipped. The abs.w windows are the two ends of the 24-bit space, so a stride that does
/// not straddle the wrap and whose first and last addresses both encode abs.l encodes
/// abs.l throughout.
pub fn far_scratch_slot(from: u32) -> Result<u32, String> {
    let mut slot = from;
    loop {
        let first = slot as i64;
        let last = first + SCRATCH_STRIDE as i64 - 1;
        let one_wrap = first >> 24 == last >> 24;
        if one_wrap
            && sigil_ir::asl_width_rule(first, true) == sigil_ir::AbsWidth::L
            && sigil_ir::asl_width_rule(last, true) == sigil_ir::AbsWidth::L
        {
            return Ok(slot);
        }
        slot = next_scratch(slot)?;
    }
}

/// The raw far-scratch slot after `slot`. LOUD ON EXHAUSTION: a cursor that would pass
/// the top of the address type is a refusal, never a wrap.
pub fn next_scratch(slot: u32) -> Result<u32, String> {
    slot.checked_add(SCRATCH_STRIDE).ok_or_else(|| {
        format!(
            "span pass: the far-scratch cursor ran out of 32-bit address space past {slot:#x}; \
             there are more never-pinned ROM sections than it has slots"
        )
    })
}

/// [`measure_pinned`]'s body, shared with the checked form. `tolerate_overlap`
/// selects `resolve_layout_measuring` (measuring: exact widths at the given pins,
/// overlap permitted) vs `resolve_layout` (checked: the image-soundness witness —
/// the walk runs it once at the converged bases, and `declared_spans_by_index` at
/// the final ones). The unique-name tag makes the read-back unambiguous (the tree
/// carries same-named `text`/`sec<lma>` sections); names never affect bytes.
pub fn lens_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
    tolerate_overlap: bool,
) -> Result<Measured, String> {
    let mut tagged: Vec<Section> = sections.to_vec();
    let mut scratch: u32 = SCRATCH_FIRST;
    for (i, s) in tagged.iter_mut().enumerate() {
        s.name = format!("{}\u{0}{i}", s.name);
        if is_rom_section(s) {
            // Force Pinned so `resolve_layout` honours the lma we set (a Chained section
            // would otherwise ignore it and pack within its group, defeating the pin).
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            match pin_lma.get(i) {
                Some(Some(p)) => s.lma = *p,
                _ => {
                    let slot = far_scratch_slot(scratch)?;
                    s.lma = slot;
                    // keep vma tracking the scratch lma so labels don't leak a stale base
                    if s.vma_base.map(|v| v < 0x8000).unwrap_or(false) {
                        s.vma_base = None;
                    }
                    scratch = next_scratch(slot)?;
                }
            }
        }
    }
    let stubs = SymbolTable::new();
    let resolved = if tolerate_overlap {
        sigil_link::resolve_layout_measuring(&tagged, &stubs, true)
    } else {
        sigil_link::resolve_layout(&tagged, &stubs, true)
    }
    .map_err(|d| {
        format!("span pass: resolve_layout: {} diag(s); first {:?}", d.len(), d.first())
    })?;
    let mut img = vec![0u32; sections.len()];
    let mut sites = vec![Vec::new(); sections.len()];
    for s in &resolved {
        if let Some(idx) = s.name.rsplit('\u{0}').next().and_then(|t| t.parse::<usize>().ok()) {
            if is_rom_section(s) {
                img[idx] = s.image_len();
                // Relaxables lower 1:1 (the resolve maps each fragment to its chosen
                // candidate in place), so the input's relaxable positions read the
                // lowered lengths off the same indices.
                if s.fragments.len() == sections[idx].fragments.len() {
                    sites[idx] = sections[idx]
                        .fragments
                        .iter()
                        .zip(&s.fragments)
                        .filter(|(before, _)| is_relaxable(before))
                        .map(|(before, after)| (fragment_span(before), fragment_len(after)))
                        .collect();
                }
            }
        }
    }
    Ok(Measured { img, sites })
}

pub fn is_relaxable(f: &Fragment) -> bool {
    matches!(f, Fragment::RelaxAbsSym { .. } | Fragment::JmpJsrSym { .. } | Fragment::RelaxLadder { .. })
}

pub fn fragment_span(f: &Fragment) -> sigil_span::Span {
    match f {
        Fragment::Data(d) => d.span,
        Fragment::Fill { span, .. }
        | Fragment::Reserve { span, .. }
        | Fragment::Org { span, .. }
        | Fragment::JmpJsrSym { span, .. }
        | Fragment::RelaxAbsSym { span, .. }
        | Fragment::RelaxLadder { span, .. } => *span,
    }
}

/// The image bytes one LOWERED fragment contributes (a lowered relaxable is `Data`).
pub fn fragment_len(f: &Fragment) -> u32 {
    match f {
        Fragment::Data(d) => d.bytes.len() as u32,
        Fragment::Fill { count, .. } => *count,
        _ => 0,
    }
}

/// Per-section image length (exact, relaxables lowered to `Data`) from the CHECKED
/// resolve at `pin_lma` — the image-soundness witness form of [`measure_pinned`].
pub fn image_lens_pinned(
    sections: &[Section],
    pin_lma: &[Option<u32>],
) -> Result<Vec<u32>, String> {
    lens_pinned(sections, pin_lma, false).map(|m| m.img)
}

/// The head label of a section — its lowest-offset label — the name an `order` row
/// spells when it is a label row. `None` for a label-less blob.
pub fn head_label(sec: &Section) -> Option<&str> {
    sec.labels.iter().min_by_key(|l| l.offset).map(|l| l.name.as_str())
}

/// The `order` row a section is declared by, and its rank: the head label's row when
/// there is one, else the `section:<name>` row. A section whose head label is CONTENT-
/// DERIVED (`ojz_effects_editor_act1`, minted by `tools/effects_gen.py`) is authorable
/// only by the second spelling. `validate_placement` rejects a section declared BOTH ways.
pub fn order_rank_of(sec: &Section, rank: &HashMap<&str, usize>) -> Option<usize> {
    head_label(sec)
        .and_then(|l| rank.get(l).copied())
        .or_else(|| rank.get(crate::map_placement::section_row_key(&sec.name).as_str()).copied())
}

/// The non-convergence diagnostic's payload: for every section whose measured length
/// differs between two consecutive rounds, the section, the base it was measured at,
/// both lengths, and the `file:line` of every relaxable site whose lowered width
/// differs between the rounds (with both widths) — the RELAXING SITE, so the author
/// sees the `lea`/branch whose encoding depends on where the section lands instead
/// of an innocent overlapping pair.
#[allow(clippy::too_many_arguments)]
pub fn width_flip_report(
    sections: &[Section],
    order: &[usize],
    img_before: &[u32],
    sites_before: &[Vec<(sigil_span::Span, u32)>],
    img_after: &[u32],
    sites_after: &[Vec<(sigil_span::Span, u32)>],
    pins: &[Option<u32>],
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> String {
    let mut lines = Vec::new();
    for &i in order {
        if img_before[i] == img_after[i] {
            continue;
        }
        let head = head_label(&sections[i]).map(|l| format!(" (`{l}`)")).unwrap_or_default();
        let base = pins[i].map(|b| format!("{b:#x}")).unwrap_or_else(|| "scratch".to_string());
        let mut line = format!(
            "section `{}`{head} measures {:#x} then {:#x} at base {base}",
            sections[i].name, img_before[i], img_after[i]
        );
        let flips: Vec<String> = sites_before[i]
            .iter()
            .zip(&sites_after[i])
            .filter(|((_, a), (_, b))| a != b)
            .map(|((span, a), (_, b))| {
                let at = locate(*span)
                    .unwrap_or_else(|| format!("source {} @{}", span.source.0, span.start));
                format!("{at} ({a} B -> {b} B)")
            })
            .collect();
        if !flips.is_empty() {
            line.push_str("; width-flipping sites: ");
            line.push_str(&flips.join(", "));
        }
        lines.push(line);
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("Sections whose length depends on their base: {}", lines.join(" | "))
    }
}

/// The declared per-section SIZE (exact asl span the chainer reserves), keyed by stable
/// index. Each ROM section is pinned at its `true_base`; the resolved layout's per-run
/// advance (next.true_base − this.true_base within an abutting run; image_len before an
/// org hole) is its span. This is what anchors relaxation to asl's widths.
pub fn declared_spans_by_index(
    sections: &[Section],
    true_bases: &[Option<u32>],
) -> Result<Vec<Option<u32>>, String> {
    // A trailing `align $8000` on the pre-sound-bank data blob (HeightMaps) makes its
    // image OVERSHOOT the sound-bank anchor, so pinning both at their true bases collides
    // in the measuring resolve. Scratch the phase-region sections (the sound banks) for
    // the measurement — they are pure data (their own span comes from the frozen gap, not
    // this image) — so the resolve is overlap-free; the pre-bank blob is still measured at
    // its true base (its span then clamps to the gap-to-anchor above).
    let phase_region = phase_region_mask(sections, true_bases);
    let pin: Vec<Option<u32>> = (0..sections.len())
        .map(|i| if phase_region[i] { None } else { true_bases[i] })
        .collect();
    // Declared spans measure at the FINAL true_bases — the packed layout is already
    // overlap-free (no scratch fallback: every length here is the real one), and each
    // pure-data span comes from its base-to-next gap.
    let img = image_lens_pinned(sections, &pin).map_err(|e| format!("span pass (declared): {e}"))?;
    let mut rom: Vec<(usize, u32, u32)> = (0..sections.len())
        .filter_map(|i| true_bases.get(i).and_then(|o| *o).map(|tb| (i, tb, img[i])))
        .collect();
    rom.sort_by_key(|&(_, tb, _)| tb);
    let mut span = vec![None; sections.len()];
    for i in 0..rom.len() {
        let (idx, tb, im) = rom[i];
        let adv = if i + 1 < rom.len() {
            let gap = rom[i + 1].1.saturating_sub(tb);
            // The declared size is the GAP to the next section when they abut (gap ≈
            // image), and also when the image OVERSHOOTS the next section — a trailing
            // `align $8000` (the sound bank) measures more padding at a scratch pin than
            // fits at the real base, so the next anchor clamps the reserved span. A gap
            // MUCH larger than the image is a genuine org hole: reserve only the image
            // (the anchor after it pins; the interval fills).
            if gap <= im + 0x10 { gap } else { im }
        } else {
            im
        };
        span[idx] = Some(if im == 0 { 0 } else { adv.max(1) });
    }
    Ok(span)
}

/// ALIGN-RECOMPUTE-ON-RELOCATION. A trailing `align $8000` (the pre-sound-bank data
/// blob HeightMaps) bakes its padding as a `Fill` sized for the section's AS-RESIDUAL
/// position. When the frozen chainer relocates the section, that baked pad is stale —
/// its image OVERSHOOTS the section's (correctly clamped) reserved span and collides
/// with the next hard-org anchor (`MovingTrucks_Bank_Start` at 0x58000). asl would have
/// recomputed the pad for the new position; the linker never shrank a baked `align`.
///
/// This recomputes it: for a PURE-DATA section (no width-variable fragment — the only
/// case whose image length is known pre-relaxation, and exactly the align-blob shape),
/// if the image exceeds `span`, trim the overshoot off the TRAILING zero-`Fill` (the
/// align padding). Only zero-Fill is trimmed and only down to `span`, so no real datum
/// is ever cut; an overshoot that is NOT trailing zero-Fill is left intact for
/// `resolve_layout` to reject loudly. Byte-neutral for the pinned (sonic4) path, where
/// `image_len == span` and the guard never fires.
pub fn trim_trailing_align_overshoot(s: &mut Section, span: u32) {
    let has_variable = s.fragments.iter().any(|f| {
        matches!(
            f,
            Fragment::JmpJsrSym { .. } | Fragment::RelaxAbsSym { .. } | Fragment::RelaxLadder { .. }
        )
    });
    if has_variable {
        return;
    }
    let img = s.image_len();
    if img <= span {
        return;
    }
    let mut overshoot = img - span;
    while overshoot > 0 {
        match s.fragments.last_mut() {
            Some(Fragment::Fill { value: 0, count, .. }) => {
                if *count > overshoot {
                    *count -= overshoot;
                    overshoot = 0;
                } else {
                    overshoot -= *count;
                    s.fragments.pop();
                }
            }
            // Not trailing zero-fill: leave the overshoot for resolve_layout to reject.
            _ => break,
        }
    }
}

/// The minimum alignment (bank size) an INTERNAL `align` must target for
/// `recompute_bank_aligns` to touch it. `align $8000` (the DAC/MT sound-bank pads)
/// is the only aeon align at or above this; `align 2` word-aligns and any small
/// bulk zero-fill fall below it and are left exactly as baked — so a relocated
/// section's HEAD (e.g. HeightMaps at its true base) never moves.
pub const BANK_ALIGN: u32 = 0x8000;

/// ALIGN-RECOMPUTE-ON-RELOCATION for INTERNAL bank aligns (the general form of
/// `trim_trailing_align_overshoot`, which handles only the trailing pad).
///
/// A pure-data section can carry `align $8000` pads MID-section (main.asm's
/// BINCLUDE arm: HeightMaps → art → `align $8000` → DAC blip → `align $8000` →
/// DAC shared → `align $8000` → MovingTrucks — ONE section). `directive_align`
/// bakes each as a fixed `Fill{0, pad}` computed against the section's AS-RESIDUAL
/// base. When the chainer re-pins the section at its true (frozen) base, that
/// fixed pad carries the following content off the absolute bank boundary by the
/// relocation delta (config_a's DAC banks landed +0xC at 0x4800c/0x5000c). asl
/// aligns to ABSOLUTE N-multiples independent of the base, so the pad must be
/// recomputed.
///
/// Replays the BAKED and TRUE absolute positions in parallel from `baked_base` /
/// `true_base`. For each zero-`Fill` that is a MINIMAL pad landing on a
/// `>= BANK_ALIGN` boundary (a bank align — never a word-align or bulk fill), the
/// pad is rewritten so the following content resumes on that SAME absolute
/// boundary (`new_pad = baked_after − true_pos`); the two cursors re-sync there.
/// Non-bank fragments advance both cursors by their length, preserving the delta.
/// Byte-neutral for the pinned path (`baked_base == true_base`, the loop is a
/// no-op) and for any section with no `>= BANK_ALIGN` internal align.
///
/// Pure-data only (guarded): a width-variable fragment makes image offsets
/// base-dependent in a way this static replay does not model — such a section is
/// left untouched (it has no bank align in practice).
pub fn recompute_bank_aligns(s: &mut Section, baked_base: u32, true_base: u32) {
    if baked_base == true_base {
        return;
    }
    // Only a section whose fragments advance the cursor monotonically by their own
    // length can be replayed statically here. A width-variable fragment (base-
    // dependent offsets) or an `Org` (cursor seek) is left untouched — neither
    // occurs in the sound-bank blob this targets.
    let simple = s.fragments.iter().all(|f| {
        matches!(f, Fragment::Data(_) | Fragment::Fill { .. } | Fragment::Reserve { .. })
    });
    if !simple {
        return;
    }
    let mut baked = baked_base;
    let mut tru = true_base;
    for f in &mut s.fragments {
        match f {
            Fragment::Fill { value: 0, count, .. } => {
                let c = *count;
                let baked_after = baked.wrapping_add(c);
                // A bank align = a minimal pad (`c < BANK_ALIGN`) whose post-position is a
                // >= BANK_ALIGN multiple. The boundary is `baked_after` itself (the absolute
                // bank start the residual align reached). Recompute so `tru` resumes there.
                let is_bank_align =
                    c < BANK_ALIGN && baked_after != 0 && baked_after % BANK_ALIGN == 0;
                if is_bank_align && baked_after >= tru {
                    *count = baked_after - tru;
                    tru = baked_after;
                    baked = baked_after;
                } else {
                    baked = baked_after;
                    tru = tru.wrapping_add(c);
                }
            }
            Fragment::Data(d) => {
                let l = d.bytes.len() as u32;
                baked = baked.wrapping_add(l);
                tru = tru.wrapping_add(l);
            }
            Fragment::Fill { count, .. } | Fragment::Reserve { count, .. } => {
                baked = baked.wrapping_add(*count);
                tru = tru.wrapping_add(*count);
            }
            _ => {}
        }
    }
}

/// Apply the declared-order computed-base chain to `sections`: split ROM/RAM, sort ROM
/// by ascending true base, and Pin each org anchor / Chain the rest reserving its
/// declared span. Returns the placed sections (ROM then RAM). Shared by the driver and
/// the layout diagnostic so they can never drift.
pub fn apply_declared_chain(
    sections: Vec<Section>,
    true_bases: &[Option<u32>],
    spans: &[Option<u32>],
) -> Vec<Section> {
    type IndexedSection = (usize, Section);
    let indexed: Vec<IndexedSection> = sections.into_iter().enumerate().collect();
    let (mut rom, ram): (Vec<IndexedSection>, Vec<IndexedSection>) =
        indexed.into_iter().partition(|(_, s)| is_rom_section(s));
    // DECLARED ORDER = ascending true base (baked for sonic4; frozen for demo/Config).
    rom.sort_by_key(|(idx, s)| (true_bases[*idx].unwrap_or(s.lma), s.placement_span()));

    let mut cur_group = String::new();
    let mut gi = 0usize;
    let mut chain_end: Option<u32> = None;
    for (idx, s) in rom.iter_mut() {
        let orig_lma = s.lma;
        let tb = true_bases[*idx].unwrap_or(orig_lma);
        let span = spans[*idx].unwrap_or_else(|| s.placement_span());
        // A GENUINELY-PHASED section runs at a VMA distinct from its ROM lma. Two kinds:
        //   - a HARD-ORG SOUND BANK (vma at a real phase ≥ 0x8000 — resident/moving-
        //     trucks): its labels use the phase VMA and its ROM org is absolute; ANCHOR.
        //   - an INLINE Z80 section (vma = 0 — the no-sound Z80 idle): its internal
        //     labels are Z80-relative (vma 0); the 68k reaches it via a neighbour label,
        //     so it CHAINS but KEEPS its vma=0.
        // A normal AS section carries vma == baked lma (NOT phased) — the check MUST
        // compare against the section's OWN baked lma, never the re-based `tb` (else a
        // baked object-bank address ≥ 0x8000 is misread as a phase and its stale sonic4
        // VMA leaks into every reference to its labels).
        let phase_bank = s.vma_base.map(|v| v != orig_lma && v >= 0x8000).unwrap_or(false);
        let keep_vma = phase_bank || s.vma_base == Some(0);
        let is_anchor =
            chain_end.is_none() || phase_bank || chain_end.map(|e| tb > e).unwrap_or(true);
        s.reserved_span = span;
        // Recompute stale INTERNAL bank-boundary aligns (the DAC/MT `align $8000`
        // pads inside a relocated pure-data blob) for the new base, then the
        // TRAILING align (see each fn's doc). A VMA-tracking (non-phase) section
        // only — a phase bank keeps its baked absolute image.
        if !keep_vma {
            recompute_bank_aligns(s, orig_lma, tb);
        }
        trim_trailing_align_overshoot(s, span);
        if !keep_vma {
            s.vma_base = None; // VMA tracks the (re-based) LMA — no stale baked address.
        }
        if is_anchor {
            gi += 1;
            cur_group = format!("declchain{gi}");
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Pinned;
            s.lma = tb;
            chain_end = Some(tb + span);
        } else {
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Chained;
            s.lma = 0; // base COMPUTED — the baked resume-org literal is discarded.
            chain_end = Some(chain_end.unwrap().max(tb) + span);
        }
    }

    let mut all: Vec<Section> = rom.into_iter().map(|(_, s)| s).collect();
    all.extend(ram.into_iter().map(|(_, s)| s));
    all
}

/// Mark the PHASE-REGION ROM sections (a phase bank — vma ≥ 0x8000 — and the label-less
/// data tail chaining after it, up to the next labeled section). These are the hard-org
/// sound banks whose image (with its own trailing align) would collide in the span-pass
/// resolve; their declared span comes from frozen gaps, not from a measured image, so
/// they are scratched during measurement.
pub fn phase_region_mask(sections: &[Section], true_bases: &[Option<u32>]) -> Vec<bool> {
    let n = sections.len();
    let mut order: Vec<usize> = (0..n).filter(|&i| true_bases[i].is_some()).collect();
    order.sort_by_key(|&i| true_bases[i].unwrap());
    let mut mask = vec![false; n];
    let mut in_phase = false;
    for &i in &order {
        let s = &sections[i];
        let is_phase_bank =
            s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false);
        if is_phase_bank {
            in_phase = true;
            mask[i] = true;
        } else if in_phase {
            // stay in the phase region only for label-less data tails; a real labeled
            // section (object_test_state) ends it.
            let has_own_label = !s.labels.is_empty();
            if has_own_label {
                in_phase = false;
            } else {
                mask[i] = true;
            }
        }
    }
    mask
}

/// True for an image-bearing ROM section (VMA below the RAM/phase floor), as opposed
/// to a RAM/phase section that never participates in ROM layout.
pub fn is_rom_section(s: &Section) -> bool {
    match s.vma_base {
        Some(v) => v < 0x00F0_0000,
        None => true,
    }
}

/// A placed section's one-past-end ROM address: `lma + image_len` — THE derivation
/// behind every synthesized `<Base>_End` marker and the `section:<name>` region end.
pub fn section_end(s: &Section) -> u32 {
    s.lma.wrapping_add(s.image_len())
}

/// The Genesis `$8000`-window bank size. The sound bank spans
/// `[sound_bank, sound_bank + BANK_WINDOW_SIZE)`; its top is the ceiling every
/// in-bank artifact must fit under.
pub const BANK_WINDOW_SIZE: u32 = 0x8000;

/// The room the `mt_bank` region has: from its LMA to the top of the `$8000` bank
/// window that starts at `bank_start`.
/// The bytes from `lma` to the top of its `$8000` Z80 `SetBank` window: the room every
/// seam-2 artifact placed at `lma` must fit in (the [`mt_bank_room`] precedent, without
/// a named window base). Each seam-2 synthetic map sizes its regions with this instead
/// of a hand-typed literal: a literal smaller than the artifact was never read (the
/// pre-A2 `0x400` for a 1,116 B `sound_tables_z80`), and one larger would let an
/// artifact cross into the next window.
pub fn bank_room(lma: u32) -> u32 {
    BANK_WINDOW_SIZE - (lma & (BANK_WINDOW_SIZE - 1))
}

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
