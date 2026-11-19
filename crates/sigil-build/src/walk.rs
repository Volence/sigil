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
}
