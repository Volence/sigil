//! The new driver's packing walk: every ROM section's base from the map's `order`, its
//! `[[anchor]]` rows and the alignment table, with no frozen size table.
//!
//! Spec §5.1: "the frozen size tables do not move. They identify islands and seed
//! measurement, and the map's `[[anchor]]` rows now declare the islands." The old walk
//! (sigil-harness `native::packed_true_bases`) reads each section's provisional base out
//! of a per-shape frozen table: an island is a section whose frozen base equals an anchor
//! address, and the frozen bases are where round 0 measures. Here:
//!
//!   * an ISLAND is exactly a section an anchor in force HOLDS (`[[anchor]] holds = "<row>"`,
//!     an `order` row: a head label or `section:<name>`). It is placed at the anchor's `at`.
//!     The run head must be one: the walk has no other first address.
//!   * every other section with a head label packs at the running cursor rounded up to its
//!     alignment ([`AlignTable::packed_base`](crate::align::AlignTable::packed_base)); a label-less blob packs flush.
//!   * the SEQUENCE is the map's `order`. A ROM section no row names (a label-less blob, or
//!     a zero-byte marker the map leaves out) rides directly after the section before it in
//!     the program's own section order, so an AS residual's label-less tail stays behind
//!     the label it was assembled after.
//!   * round 0 packs with each section's pre-relaxation maximum (`placement_span`), then
//!     every round measures each labeled, non-bank section at the base it was packed at and
//!     repacks, until no length moves (at most [`MAX_ROUNDS`](crate::walk::MAX_ROUNDS)). A phase bank and a
//!     label-less blob are measured at a far scratch slot, as the old walk measures them.
//!
//! LOUD, never silent: a run that packs past an anchor is refused naming the anchor, the
//! section whose end crosses it and the overrun in bytes ([`WalkError::AnchorOverrun`](crate::walk::WalkError::AnchorOverrun));
//! the overlap check of the old walk stays as the final witness.

use std::collections::HashMap;

use sigil_ir::Section;

use crate::align::AlignTable;
use crate::place::{head_label, image_lens_pinned, is_rom_section, measure_pinned, order_rank_of, width_flip_report};

/// How many measure-and-repack rounds the walk takes before it calls the layout an
/// oscillation (the old walk's bound).
pub const MAX_ROUNDS: usize = 8;

/// One `[[anchor]]` in force for the shape, bound to the section it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorPin {
    /// The anchor's `name` (what errors and overlays call it).
    pub name: String,
    /// The `order` row of the section it holds.
    pub holds: String,
    /// The section's ROM address.
    pub at: u32,
}

/// Why the walk refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkError {
    /// An anchor in force holds a row no ROM section of this shape is declared by.
    AnchorUnbound { anchor: String, holds: String },
    /// Two anchors hold one section.
    AnchorTwice { section: String, first: String, second: String },
    /// The first section of the sequence is not held by an anchor.
    NoRunHead { section: String },
    /// The packed run reaches past an anchor: `overrun` bytes from `last` (the section
    /// whose end crosses it) into the island the anchor holds.
    AnchorOverrun { anchor: String, holds: String, at: u32, last: String, end: u32, overrun: u32 },
    /// Two consecutive anchors are out of address order in the sequence.
    AnchorsOutOfOrder { first: String, second: String },
    /// The walk did not reach a fixed point.
    NoFixedPoint(String),
    /// A measuring resolve failed, or the converged layout overlaps.
    Resolve(String),
}

impl std::fmt::Display for WalkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalkError::AnchorUnbound { anchor, holds } => write!(
                f,
                "[map.anchor-unbound] anchor `{anchor}` holds `{holds}`, but no ROM section in this \
                 shape is declared by that row"
            ),
            WalkError::AnchorTwice { section, first, second } => write!(
                f,
                "[map.anchor-twice] anchors `{first}` and `{second}` both hold section `{section}`"
            ),
            WalkError::NoRunHead { section } => write!(
                f,
                "[map.no-run-head] the first section in the map's order, `{section}`, is not held by \
                 an [[anchor]], so the walk has no address to start the run at"
            ),
            WalkError::AnchorOverrun { anchor, holds, at, last, end, overrun } => write!(
                f,
                "[layout.anchor-overrun] the run before anchor `{anchor}` ({holds} at {at:#x}) ends at \
                 {end:#x}: section `{last}` reaches {overrun} byte(s) ({overrun:#x}) past the anchor. \
                 The anchor is a fixed address, so this content does not fit in front of it: move \
                 content behind the island, or move the anchor"
            ),
            WalkError::AnchorsOutOfOrder { first, second } => write!(
                f,
                "[map.anchor-order] anchor `{second}` comes after `{first}` in the map's order but \
                 holds a lower address"
            ),
            WalkError::NoFixedPoint(m) => write!(
                f,
                "[layout.no-fixed-point] the packing walk did not converge in {MAX_ROUNDS} rounds \
                 (relaxation oscillation).{m}"
            ),
            WalkError::Resolve(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for WalkError {}

fn is_phase_bank(s: &Section) -> bool {
    s.vma_base.map(|v| v != s.lma && v >= 0x8000).unwrap_or(false)
}

/// The order key a section is declared by (its head label, or `section:<name>`), when
/// a row names it.
fn declared_row<'r>(s: &Section, rank: &HashMap<&'r str, usize>, order: &'r [String]) -> Option<&'r str> {
    order_rank_of(s, rank).map(|r| order[r].as_str())
}

/// The walk's sequence over `sections`' ROM indices: named sections by map rank, each
/// unnamed one directly after its predecessor in `sections` order.
pub fn sequence(sections: &[Section], order: &[String]) -> Vec<usize> {
    let rank: HashMap<&str, usize> = order.iter().enumerate().map(|(r, s)| (s.as_str(), r)).collect();
    // (rank, position after the named section, index); an unnamed section before any
    // named one rides at the front.
    let mut keyed: Vec<(i64, usize, usize)> = Vec::new();
    let mut cur_rank: i64 = -1;
    let mut sub = 0usize;
    for (i, s) in sections.iter().enumerate() {
        if !is_rom_section(s) {
            continue;
        }
        match order_rank_of(s, &rank) {
            Some(r) => {
                cur_rank = r as i64;
                sub = 0;
                keyed.push((cur_rank, 0, i));
            }
            None => {
                sub += 1;
                keyed.push((cur_rank, sub, i));
            }
        }
    }
    keyed.sort();
    keyed.into_iter().map(|(_, _, i)| i).collect()
}

/// Every ROM section's base, keyed by index (`None` for a RAM section).
pub fn true_bases(
    sections: &[Section],
    order: &[String],
    anchors: &[AnchorPin],
    align: &AlignTable,
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
) -> Result<Vec<Option<u32>>, WalkError> {
    let n = sections.len();
    let rank: HashMap<&str, usize> = order.iter().enumerate().map(|(r, s)| (s.as_str(), r)).collect();

    // ── Bind every anchor in force to the one section its row declares. ──
    let mut held: Vec<Option<usize>> = vec![None; n]; // section -> anchor index
    for (ai, a) in anchors.iter().enumerate() {
        let hit: Vec<usize> = (0..n)
            .filter(|&i| is_rom_section(&sections[i]) && declared_row(&sections[i], &rank, order) == Some(a.holds.as_str()))
            .collect();
        let Some(&i) = hit.first() else {
            return Err(WalkError::AnchorUnbound { anchor: a.name.clone(), holds: a.holds.clone() });
        };
        if let Some(prev) = held[i] {
            return Err(WalkError::AnchorTwice {
                section: sections[i].name.clone(),
                first: anchors[prev].name.clone(),
                second: a.name.clone(),
            });
        }
        held[i] = Some(ai);
    }

    let seq = sequence(sections, order);
    // The run head: the first section that is not an empty label-less blob (an AS
    // residual opens with a few of those; they emit nothing and carry no label, so
    // where they sit moves no byte, and they take the run head's address).
    let empty_blob = |i: usize| head_label(&sections[i]).is_none() && sections[i].placement_span() == 0;
    let Some(&first) = seq.iter().find(|&&i| !empty_blob(i)) else { return Ok(vec![None; n]) };
    let Some(head_anchor) = held[first] else {
        return Err(WalkError::NoRunHead { section: head_label(&sections[first]).unwrap_or(&sections[first].name).to_string() });
    };
    let run_start = anchors[head_anchor].at;
    // Anchors must ascend along the sequence: a later anchor at a lower address would
    // make the run between them negative.
    let mut last_anchor: Option<usize> = None;
    for &i in &seq {
        if let Some(ai) = held[i] {
            if let Some(prev) = last_anchor {
                if anchors[ai].at < anchors[prev].at {
                    return Err(WalkError::AnchorsOutOfOrder {
                        first: anchors[prev].name.clone(),
                        second: anchors[ai].name.clone(),
                    });
                }
            }
            last_anchor = Some(ai);
        }
    }

    // Measured lengths: round 0 packs with each section's pre-relaxation maximum.
    let mut lens: Vec<u32> = sections.iter().map(|s| if is_rom_section(s) { s.placement_span() } else { 0 }).collect();
    let mut sites: Vec<Vec<(sigil_span::Span, u32)>> = vec![Vec::new(); n];
    let mut last_flip = String::new();
    for round in 0..=MAX_ROUNDS {
        // Pack.
        let mut bases: Vec<Option<u32>> = vec![None; n];
        let mut running: Option<(u32, usize)> = None; // (cursor, last section)
        let mut overrun: Option<WalkError> = None;
        for &i in &seq {
            let s = &sections[i];
            let base = match held[i] {
                Some(ai) => {
                    let a = &anchors[ai];
                    if let Some((end, last)) = running {
                        if end > a.at && overrun.is_none() {
                            overrun = Some(WalkError::AnchorOverrun {
                                anchor: a.name.clone(),
                                holds: a.holds.clone(),
                                at: a.at,
                                last: head_label(&sections[last]).unwrap_or(&sections[last].name).to_string(),
                                end,
                                overrun: end - a.at,
                            });
                        }
                    }
                    a.at
                }
                None => match running {
                    // Only an empty label-less blob precedes the run head.
                    None => run_start,
                    Some((r, _)) => match head_label(s) {
                        Some(h) => align.packed_base(r, h),
                        None => r,
                    },
                },
            };
            bases[i] = Some(base);
            if running.is_some() || !empty_blob(i) {
                running = Some((base + lens[i], i));
            }
        }
        if round == MAX_ROUNDS {
            break;
        }
        // Measure every labeled, non-bank section at the base it was packed at.
        let pins: Vec<Option<u32>> = (0..n)
            .map(|i| {
                let s = &sections[i];
                if is_rom_section(s) && head_label(s).is_some() && !is_phase_bank(s) {
                    bases[i]
                } else {
                    None
                }
            })
            .collect();
        let m = measure_pinned(sections, &pins).map_err(|e| WalkError::Resolve(format!("{e} (walk round {round})")))?;
        if round > 0 && m.img == lens {
            // A fixed point: every length is the one measured at its own base.
            if let Some(e) = overrun {
                return Err(e);
            }
            if let Err(collision) = image_lens_pinned(sections, &pins) {
                return Err(WalkError::Resolve(format!(
                    "[layout.overlap] the packed layout overlaps at its real bases: {collision}"
                )));
            }
            return Ok(bases);
        }
        if round > 0 {
            last_flip = width_flip_report(sections, &seq, &lens, &sites, &m.img, &m.sites, &pins, locate);
        }
        lens = m.img;
        sites = m.sites;
    }
    Err(WalkError::NoFixedPoint(if last_flip.is_empty() { String::new() } else { format!(" {last_flip}") }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{AddressSpace, Cpu, Fragment, Label, SectionPlacement};
    use sigil_span::{SourceId, Span};

    fn sec(label: &str, len: u32) -> Section {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        Section {
            name: format!("s_{label}"),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0,
            space: AddressSpace::Image,
            labels: vec![Label { name: label.into(), offset: 0 }],
            fragments: vec![Fragment::Fill { value: 0, count: len, span }],
            placement: SectionPlacement::Chained,
            reserved_span: len,
            group: None,
            bank: None,
            equ_syms: Vec::new(),
        }
    }

    fn walk(growth: u32) -> Result<Vec<Option<u32>>, WalkError> {
        let sections = vec![sec("Head", 0x10), sec("Grow", growth), sec("Island", 4), sec("Tail", 3), sec("After", 2)];
        let order: Vec<String> = ["Head", "Grow", "Island", "Tail", "After"].iter().map(|s| s.to_string()).collect();
        let anchors = vec![
            AnchorPin { name: "boot".into(), holds: "Head".into(), at: 0 },
            AnchorPin { name: "bank".into(), holds: "Island".into(), at: 0x40 },
        ];
        let align = AlignTable::new(std::iter::empty()).unwrap();
        true_bases(&sections, &order, &anchors, &align, &|_| None)
    }

    /// Spec §5.1 / plan Review Focus: content that grows in front of an island either still
    /// fits (the island stays at its anchor, everything after it is unmoved) or is refused
    /// naming the anchor, the section that crosses it and the overrun, never placed
    /// over the island or silently pushing it.
    #[test]
    fn growth_before_island_places_or_refuses() {
        // Fits, with room: the island stays at 0x40, the tail packs after it.
        assert_eq!(walk(0x2E).unwrap(), vec![Some(0), Some(0x10), Some(0x40), Some(0x44), Some(0x48)]);
        // Fits exactly: Grow ends at the anchor.
        assert_eq!(walk(0x30).unwrap(), vec![Some(0), Some(0x10), Some(0x40), Some(0x44), Some(0x48)]);
        // One byte too many: refused by name and size.
        assert_eq!(
            walk(0x31).unwrap_err(),
            WalkError::AnchorOverrun {
                anchor: "bank".into(),
                holds: "Island".into(),
                at: 0x40,
                last: "Grow".into(),
                end: 0x41,
                overrun: 1
            }
        );
        // The message says where and by how much.
        let msg = walk(0x50).unwrap_err().to_string();
        assert!(msg.contains("[layout.anchor-overrun]") && msg.contains("`bank`") && msg.contains("`Grow`") && msg.contains("32 byte(s)"), "{msg}");
        // The evenness default: an odd tail rounds the next section up to even.
        assert_eq!(walk(0x30).unwrap()[4], Some(0x48));
    }

    #[test]
    fn an_unheld_run_head_and_an_unbound_anchor_are_refused() {
        let sections = vec![sec("Head", 4), sec("Next", 4)];
        let order: Vec<String> = vec!["Head".into(), "Next".into()];
        let align = AlignTable::new(std::iter::empty()).unwrap();
        let none: Vec<AnchorPin> = Vec::new();
        assert_eq!(true_bases(&sections, &order, &none, &align, &|_| None).unwrap_err(), WalkError::NoRunHead { section: "Head".into() });
        let stray = vec![AnchorPin { name: "x".into(), holds: "Gone".into(), at: 0 }];
        assert_eq!(
            true_bases(&sections, &order, &stray, &align, &|_| None).unwrap_err(),
            WalkError::AnchorUnbound { anchor: "x".into(), holds: "Gone".into() }
        );
    }

    // ── Moved from the profile-driven driver's walk tests (`placement_validation_tests`,
    // `derived_layout_tests`, G0b Task 11) onto this walk. What they proved about the frozen
    // provisional bases (the pin residue, drift warnings, the tolerance, the prov-sort
    // fallback) has no successor here: this walk reads no frozen table. Every expectation is
    // derived from the fixture's sizes, the declared alignments and the anchors. ──

    fn order_of(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|s| s.to_string()).collect()
    }

    fn pin(holds: &str, at: u32) -> AnchorPin {
        AnchorPin { name: format!("a_{holds}"), holds: holds.into(), at }
    }

    fn even() -> AlignTable {
        AlignTable::new(std::iter::empty()).unwrap()
    }

    fn bases(secs: &[Section], order: &[String], anchors: &[AnchorPin], align: &AlignTable) -> Vec<Option<u32>> {
        true_bases(secs, order, anchors, align, &|_| None).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The map's rank sequences the walk, whatever the sections' baked bases say: `High`
    /// (baked 0x200) declared ahead of `Low` (baked 0x100) heads the run at its anchor, and
    /// `Low` packs right after it.
    #[test]
    fn drives_order_by_map_rank() {
        let mut low = sec("Low", 0x10);
        low.lma = 0x100;
        let mut high = sec("High", 0x10);
        high.lma = 0x200;
        let got = bases(&[low, high], &order_of(&["High", "Low"]), &[pin("High", 0x200)], &even());
        assert_eq!(got, vec![Some(0x210), Some(0x200)]);
    }

    /// The `section:<name>` spelling of a row ranks exactly as the head-label spelling does.
    #[test]
    fn section_row_drives_the_packer_rank() {
        let secs = [sec("Low", 0x10), sec("High", 0x10)];
        let by_name = bases(&secs, &order_of(&["section:s_High", "Low"]), &[pin("section:s_High", 0x200)], &even());
        let by_label = bases(&secs, &order_of(&["High", "Low"]), &[pin("High", 0x200)], &even());
        assert_eq!(by_name, vec![Some(0x210), Some(0x200)], "High heads the run, Low packs after it");
        assert_eq!(by_name, by_label, "a `section:` row ranks exactly as the label row would");
    }

    /// The alignment input is the declaration: `Sfx_33` declares 8 and `GameLoop` takes the
    /// evenness default, both baked at a 16-aligned 0x1020 (the residue a base-reading walk
    /// would turn into 16). After a 0x11-byte head the cursor is 0x1011, so `Sfx_33` packs at
    /// 0x1018 and `GameLoop` at 0x1012.
    #[test]
    fn the_walk_packs_to_the_declared_alignment_not_the_pin_residue() {
        let rows = [crate::describe::AlignRow { label: "Sfx_33".into(), align: 8 }];
        let align = AlignTable::new([("game.toml", &rows[..])]).unwrap();
        for (label, want) in [("Sfx_33", 0x1018u32), ("GameLoop", 0x1012)] {
            let mut tail = sec(label, 0x10);
            tail.lma = 0x1020;
            let got = bases(&[sec("Head", 0x11), tail], &order_of(&["Head", label]), &[pin("Head", 0x1000)], &align);
            assert_eq!(got[1], Some(want), "`{label}` packs to its declaration");
        }
    }

    /// The zero-byte terminus lands exactly at the image end by its own (default)
    /// alignment, with no special case in the walk.
    #[test]
    fn the_zero_byte_terminus_packs_to_the_image_end_by_declaration() {
        let got = bases(&[sec("Head", 0x12), sec("EndOfRom", 0)], &order_of(&["Head", "EndOfRom"]), &[pin("Head", 0x1000)], &even());
        assert_eq!(got[1], Some(0x1012));
    }

    /// A pure-data section grown 8 KB keeps its base and packs its neighbour downstream by
    /// exactly the growth (the profile-driven driver also warned with the delta against its
    /// frozen provisional base; there is none here).
    #[test]
    fn grown_pure_data_packs_downstream() {
        let order = order_of(&["Head", "Grown", "Tail"]);
        let at = |grown: u32| bases(&[sec("Head", 0x10), sec("Grown", grown), sec("Tail", 0x10)], &order, &[pin("Head", 0x1000)], &even());
        assert_eq!(at(0x10), vec![Some(0x1000), Some(0x1010), Some(0x1020)]);
        assert_eq!(at(0x10 + 0x2000), vec![Some(0x1000), Some(0x1010), Some(0x1020 + 0x2000)]);
    }

    /// A gap before a section is preserved only when an anchor declares it: the same
    /// section baked at 0x2000 packs contiguously unless an anchor holds it there.
    #[test]
    fn stale_provisional_gap_is_an_island_only_when_declared() {
        let mut next = sec("Next", 0x10);
        next.lma = 0x2000;
        let secs = [sec("Head", 0x10), next];
        let order = order_of(&["Head", "Next"]);
        assert_eq!(bases(&secs, &order, &[pin("Head", 0x1000)], &even())[1], Some(0x1010), "undeclared gap packs contiguously");
        assert_eq!(
            bases(&secs, &order, &[pin("Head", 0x1000), pin("Next", 0x2000)], &even())[1],
            Some(0x2000),
            "a declared anchor stays absolute"
        );
    }

    /// A declared anchor is hard: growth that fills the room before it builds with the
    /// anchor where the map says, and one byte more is refused naming the overrun.
    #[test]
    fn growth_into_a_declared_anchor_still_fails_loud() {
        let order = order_of(&["Head", "Grown", "Bank"]);
        let anchors = [pin("Head", 0x1000), pin("Bank", 0x2000)];
        let room = 0x2000 - 0x1010;
        let walk = |grown: u32| true_bases(&[sec("Head", 0x10), sec("Grown", grown), sec("Bank", 0x10)], &order, &anchors, &even(), &|_| None);
        assert_eq!(walk(room).unwrap()[2], Some(0x2000), "the declared anchor is held absolute");
        let e = walk(room + 1).unwrap_err();
        assert!(matches!(e, WalkError::AnchorOverrun { overrun: 1, .. }), "{e}");
    }

    /// A CODE section holding exactly one `lea (T).w/.l, a0`: 4 bytes when `T` resolves
    /// under $8000 on the 24-bit bus, 6 above.
    fn code_abs(label: &str, target: &str) -> Section {
        use sigil_ir::{Expr, Fixup, FixupKind, RelaxCandidate};
        let sym = || Expr::Sym(target.into());
        let short = RelaxCandidate { bytes: vec![0x41, 0xF8, 0, 0], fixup: Fixup { kind: FixupKind::Abs16Be, offset: 2, target: sym() } };
        let long = RelaxCandidate { bytes: vec![0x41, 0xF9, 0, 0, 0, 0], fixup: Fixup { kind: FixupKind::Abs32Be, offset: 2, target: sym() } };
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let mut s = sec(label, 0);
        s.fragments = vec![Fragment::RelaxAbsSym { short, long, target: sym(), span }];
        s.reserved_span = 4;
        s
    }

    /// `Wide` fills up to $7FE0, then a growable allotment, then the code, then the data
    /// its operand targets.
    fn boundary_walk(allot: u32) -> Result<Vec<Option<u32>>, WalkError> {
        let secs = [sec("Wide", 0x6FE0), sec("Allot", allot), code_abs("Code", "T"), sec("T", 0x10)];
        true_bases(&secs, &order_of(&["Wide", "Allot", "Code", "T"]), &[pin("Wide", 0x1000)], &even(), &|_| None)
    }

    /// Below the boundary the code measures its SHORT form (4 B): T sits right after it.
    #[test]
    fn base_dependent_length_reproduces_provisional_bases_at_frozen_sizes() {
        assert_eq!(boundary_walk(0x10).unwrap(), vec![Some(0x1000), Some(0x7FE0), Some(0x7FF0), Some(0x7FF4)]);
    }

    /// Growth pushes the code past $8000, where its operand widens (4 -> 6 B): T is placed
    /// from the length the code has AT ITS PACKED BASE (0x8010 + 6 = 0x8016; the abs.w
    /// arithmetic would give 0x8014).
    #[test]
    fn growth_across_the_boundary_places_the_successor_from_the_long_form() {
        let got = boundary_walk(0x30).unwrap();
        assert_eq!(got[2], Some(0x8010));
        assert_eq!(got[3], Some(0x8016), "T is placed from the LONG form");
    }

    /// An operand naming a symbol no section defines cannot be measured at any base: the
    /// walk refuses instead of packing from a fabricated length.
    #[test]
    fn an_unresolvable_operand_refuses_loud() {
        let secs = [sec("Head", 0x10), code_abs("Code", "Nowhere")];
        let e = true_bases(&secs, &order_of(&["Head", "Code"]), &[pin("Head", 0x1000)], &even(), &|_| None).unwrap_err();
        assert!(e.to_string().contains("Nowhere"), "{e}");
    }

    /// Where a zero-byte, label-less section sits must not move any section: `Head` |
    /// `Code` (`lea T, a0`) | `Next` | `Wide` (0x8000 bytes) | `ahead` zero-byte blobs | `T`.
    /// Derived, the same for every `ahead`: Code at 0x1010 measuring 6 B (T is above
    /// $8000), Next 0x1016, Wide 0x1026, T 0x9026.
    #[test]
    fn zero_byte_sections_ahead_of_a_never_pinned_target_move_nothing() {
        let blob = || {
            let mut b = sec("x", 0);
            b.name = "text".into();
            b.labels.clear();
            b
        };
        for ahead in 0..=41usize {
            let mut secs = vec![sec("Head", 0x10), code_abs("Code", "T"), sec("Next", 0x10), sec("Wide", 0x8000)];
            secs.extend((0..ahead).map(|_| blob()));
            secs.push(sec("T", 0x10));
            let last = secs.len() - 1;
            let got = true_bases(&secs, &order_of(&["Head", "Code", "Next", "Wide", "T"]), &[pin("Head", 0x1000)], &even(), &|_| None)
                .unwrap_or_else(|e| panic!("{ahead} zero-byte sections ahead: {e}"));
            assert_eq!(
                [got[0], got[1], got[2], got[3], got[last]],
                [Some(0x1000), Some(0x1010), Some(0x1016), Some(0x1026), Some(0x9026)],
                "{ahead} zero-byte label-less sections ahead of `T` moved the layout"
            );
        }
    }
}
