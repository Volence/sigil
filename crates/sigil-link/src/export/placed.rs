//! Where the front ends' symbols landed: the placed sections, indexed by label name,
//! plus the folded equates. Both front ends' side tables (`sigil-frontend-emp` and
//! `sigil-frontend-as`, `export_side`) name their symbols and ask this for the final
//! address, so the address rule is stated once, here, and it is the listing's rule
//! (`sigil-harness` `listing_from_resolved`): the FIRST section in placed order that
//! defines a name owns it, its VMA is `vma_at(offset)`, and it carries an `lma` only
//! when its section is genuinely phased (a VMA origin that differs from the LMA, over
//! bytes actually stored).
//!
//! # The space rule, stated once
//!
//! A section's `space` is [`section_space`]: `z80` for a Z80-CPU section, and `z80` for
//! a section of any CPU that is PHASED INTO A Z80 BANK WINDOW the build's memory map
//! declares ([`Z80Window`], from a `z80_bank` region): its first byte lies in the
//! window's ROM range and its VMA origin is the address the window maps that byte to.
//! Such a section's VMA is a Z80 window address (`$8000` is the head of the bank, not
//! ROM `$8000`), so stating it in `m68k` would name the wrong byte to a 68000 lookup
//! (queue row `G0A-SOUND-BANK-LABEL-SPACE`). The 68000 CPU tag those sections carry
//! says which front end lowered them, not who reads the address. Everything else is
//! `m68k`. The symbol table here and the line walk ([`super::lines`]) both read this one
//! function, so a symbol and the row under it never disagree on their space.

use super::{Space, SymbolKind, SymbolRecord};
use sigil_ir::{Cpu, Fragment, Section, SymbolTable};
use std::collections::HashMap;

/// A Z80 bank window as the build's memory map declares it (a `z80_bank` region): the
/// ROM bytes `[lma, lma + size)` the Z80 reads at `[vma, vma + size)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Z80Window {
    pub lma: u32,
    pub vma: u32,
    pub size: u32,
}

impl Z80Window {
    fn holds(&self, lma: u32) -> bool {
        lma >= self.lma && lma - self.lma < self.size
    }
}

/// THE space rule (see the module doc): the address space `sec`'s VMAs are stated in.
pub fn section_space(sec: &Section, windows: &[Z80Window]) -> Space {
    if sec.cpu == Cpu::Z80 {
        return Space::Z80;
    }
    let vma = sec.vma_at(0);
    let phased = vma != sec.lma && sec.image_len() > 0;
    let in_window = |w: &Z80Window| w.holds(sec.lma) && vma == w.vma.wrapping_add(sec.lma - w.lma);
    if phased && windows.iter().any(in_window) {
        Space::Z80
    } else {
        Space::M68k
    }
}

/// One label at its final placement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacedLabel {
    /// Index into the placed sections.
    pub section: usize,
    /// Byte offset from the section start.
    pub offset: u32,
    pub vma: u32,
    pub lma: Option<u32>,
    pub space: Space,
    /// The label sits in a reserve-only section (a RAM block: no stored byte).
    pub ram: bool,
}

/// The placed sections indexed for symbol lookup.
pub struct PlacedSymbols<'a> {
    sections: &'a [Section],
    labels: HashMap<&'a str, PlacedLabel>,
    equs: HashMap<String, i64>,
}

/// A section that reserves address space and stores nothing: Aeon's RAM blocks.
fn reserve_only(sec: &Section) -> bool {
    sec.fragments.iter().any(|f| matches!(f, Fragment::Reserve { .. }))
        && sec.fragments.iter().all(|f| matches!(f, Fragment::Reserve { .. } | Fragment::Org { .. }))
}

impl<'a> PlacedSymbols<'a> {
    /// Index `sections` (post-`resolve_layout`). `stubs` is the external table the
    /// link saw, so an equate folds exactly as the link folds it. `windows` are the
    /// shape's Z80 bank windows, which decide a phased section's space
    /// ([`section_space`]).
    pub fn new(sections: &'a [Section], stubs: &SymbolTable, windows: &[Z80Window]) -> PlacedSymbols<'a> {
        let mut labels = HashMap::new();
        for (i, sec) in sections.iter().enumerate() {
            let phased = sec.vma_origin() != sec.lma && sec.image_len() > 0;
            let space = section_space(sec, windows);
            let ram = reserve_only(sec);
            for l in &sec.labels {
                let vma = sec.vma_at(l.offset);
                // A z80 address in the banked window is not unique across banks, so the
                // schema requires its load address whether or not the section is phased.
                let banked = space == Space::Z80 && (0x8000..=0xFFFF).contains(&vma);
                let lma = (phased || banked).then(|| sec.lma.wrapping_add(l.offset));
                labels.entry(l.name.as_str()).or_insert(PlacedLabel {
                    section: i,
                    offset: l.offset,
                    vma,
                    lma,
                    space,
                    ram,
                });
            }
        }
        let equs = crate::resolved_equates(sections, stubs).into_iter().collect();
        PlacedSymbols { sections, labels, equs }
    }

    /// The label `name` at its placement, if any section defines it.
    pub fn label(&self, name: &str) -> Option<PlacedLabel> {
        self.labels.get(name).copied()
    }

    /// The folded value of equate `name`; `None` when it did not fold (the listing
    /// omits it too).
    pub fn equ(&self, name: &str) -> Option<i64> {
        self.equs.get(name).copied()
    }

    /// The extent of the body that fragments `[start, end)` of the label's section
    /// emitted: from the label to the last byte any of them stored (`Data`/`Fill`;
    /// `Org` seeks, `Reserve` stores nothing). Fragment indices survive relaxation
    /// (each relaxable fragment is lowered in place), so this is the same replay
    /// `Section::image_bytes` makes. Errs when the replayed cursor at `start` is not
    /// the label's offset, which would mean the indices no longer name the body.
    pub fn extent(&self, label: &PlacedLabel, start: usize, end: usize) -> Result<u32, String> {
        let sec = &self.sections[label.section];
        if start > end || start > sec.fragments.len() {
            return Err(format!("section `{}`: fragment range {start}..{end} is outside it", sec.name));
        }
        // A trailing zero fill the chain trimmed off the section end leaves `end` past
        // the last fragment; the bytes it named are gone, so the body ends earlier.
        let end = end.min(sec.fragments.len());
        let mut cursor: u32 = 0;
        let mut last: Option<u32> = None;
        for (i, frag) in sec.fragments.iter().enumerate().take(end) {
            if i == start && cursor != label.offset {
                return Err(format!(
                    "section `{}`: fragment {start} starts at offset {cursor:#X}, the label is at {:#X}",
                    sec.name, label.offset
                ));
            }
            let len = match frag {
                Fragment::Org { target, .. } => {
                    cursor = *target;
                    continue;
                }
                Fragment::Reserve { count, .. } => {
                    cursor += count;
                    continue;
                }
                Fragment::Data(d) => d.bytes.len() as u32,
                Fragment::Fill { count, .. } => *count,
                other => return Err(format!("section `{}`: an unlowered fragment {other:?}", sec.name)),
            };
            cursor += len;
            if i >= start && len > 0 {
                last = Some(last.map_or(cursor, |l: u32| l.max(cursor)));
            }
        }
        Ok(last.map_or(0, |l| l.saturating_sub(label.offset)))
    }

    /// The record for label `name` at its placement, `kind` as the front end knows it
    /// (a label in a reserve-only section is `Ram` whatever the front end said).
    pub fn label_record(
        &self,
        name: &str,
        kind: SymbolKind,
        module: Option<String>,
        public: bool,
    ) -> Option<SymbolRecord> {
        let p = self.label(name)?;
        Some(SymbolRecord {
            name: name.to_string(),
            demangled: None,
            space: Some(p.space),
            vma: p.vma,
            lma: p.lma,
            kind: if p.ram { SymbolKind::Ram } else { kind },
            size: None,
            module,
            public,
        })
    }

    /// The record for equate `name`: a value, so no space and no load address.
    pub fn equ_record(&self, name: &str, module: Option<String>, public: bool) -> Option<SymbolRecord> {
        let v = self.equ(name)?;
        Some(SymbolRecord {
            name: name.to_string(),
            demangled: None,
            space: None,
            // WRAPPED to 32 bits on purpose: the listing renders an equate's i64 value
            // the same way (`sigil-harness` `listing_from_resolved`, `value as u32`,
            // printed `EQU NAME = $%08X`), so a negative or wide equate here names the
            // same number the listing names, which is what gate 5 compares.
            vma: v as u32,
            lma: None,
            kind: SymbolKind::Equ,
            size: None,
            module,
            public,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_ir::{AddressSpace, DataFragment, Label, SectionPlacement};
    use sigil_span::{SourceId, Span};

    fn span() -> Span {
        Span { source: SourceId(0), start: 0, end: 0 }
    }

    fn data(n: usize) -> Fragment {
        Fragment::Data(DataFragment { bytes: vec![0x4E; n], fixups: vec![], span: span() })
    }

    fn sec(fragments: Vec<Fragment>, labels: Vec<(&str, u32)>) -> Section {
        Section {
            name: "s".into(),
            cpu: Cpu::M68000,
            vma_base: None,
            lma: 0x1000,
            space: AddressSpace::Image,
            labels: labels.into_iter().map(|(n, o)| Label { name: n.into(), offset: o }).collect(),
            fragments,
            placement: SectionPlacement::Pinned,
            reserved_span: 0,
            group: None,
            bank: None,
            equ_syms: vec![],
        }
    }

    #[test]
    fn extent_runs_to_the_last_stored_byte_of_the_body_only() {
        // pad(2) | proc A: 4 + 6 | align fill(2) after the body | proc B: 4
        let pad = Fragment::Fill { value: 0, count: 2, span: span() };
        let s = vec![sec(vec![pad.clone(), data(4), data(6), pad, data(4)], vec![("A", 2), ("B", 14)])];
        let p = PlacedSymbols::new(&s, &SymbolTable::new(), &[]);
        let a = p.label("A").unwrap();
        assert_eq!(a.vma, 0x1002);
        assert_eq!(p.extent(&a, 1, 3), Ok(10));
        let b = p.label("B").unwrap();
        assert_eq!(p.extent(&b, 4, 5), Ok(4));
        // Indices that do not start at the label are refused, not measured.
        assert!(p.extent(&a, 0, 3).is_err());
    }

    #[test]
    fn an_equate_outside_u32_wraps_as_the_listing_renders_it() {
        let mut s = sec(vec![data(2)], vec![("A", 0)]);
        let eq = |name: &str, v: i64| sigil_ir::EquSym { name: name.into(), expr: sigil_ir::Expr::Int(v), span: span() };
        s.equ_syms = vec![eq("NEG", -2), eq("WIDE", 0x1_0000_0005)];
        let secs = vec![s];
        let p = PlacedSymbols::new(&secs, &SymbolTable::new(), &[]);
        // The listing's row for each: `ListingSymbol { value: v as u32 }`.
        for (name, v) in [("NEG", -2i64), ("WIDE", 0x1_0000_0005)] {
            let r = p.equ_record(name, None, false).unwrap();
            assert_eq!(r.vma, v as u32, "{name}");
            let row = crate::emit_listing(&[crate::ListingSymbol {
                name: name.into(),
                value: v as u32,
                is_equate: true,
                unused: false,
                lma: None,
            }]);
            assert!(row.contains(&format!("EQU {name} = ${:08X}", r.vma)), "{row}");
        }
        assert_eq!(p.equ_record("NEG", None, false).unwrap().vma, 0xFFFF_FFFE);
    }

    /// G0A-SOUND-BANK-LABEL-SPACE: a 68000-CPU section phased into a declared Z80 bank
    /// window is stated in z80, with its load address; nothing else moves.
    #[test]
    fn a_section_phased_into_a_z80_bank_window_is_z80_space() {
        let win = [Z80Window { lma: 0xB_8000, vma: 0x8000, size: 0x8000 }];
        let at = |lma: u32, vma: Option<u32>| {
            let mut s = sec(vec![data(4)], vec![("L", 2)]);
            (s.lma, s.vma_base) = (lma, vma);
            s
        };
        let space = |s: Section, w: &[Z80Window]| {
            let secs = vec![s];
            let p = PlacedSymbols::new(&secs, &SymbolTable::new(), w);
            let r = p.label_record("L", SymbolKind::Label, None, false).unwrap();
            (r.space, r.vma, r.lma)
        };
        // In the window, at the VMA the window maps its first byte to: z80, with lma.
        assert_eq!(space(at(0xB_8100, Some(0x8100)), &win), (Some(Space::Z80), 0x8102, Some(0xB_8102)));
        // The same section with no window declared: the CPU's space.
        assert_eq!(space(at(0xB_8100, Some(0x8100)), &[]), (Some(Space::M68k), 0x8102, Some(0xB_8102)));
        // Phased, in the window's ROM range, but not to the window's mapping.
        assert_eq!(space(at(0xB_8100, Some(0x9000)), &win).0, Some(Space::M68k));
        // In the window but not phased (the bank's unphased data, VMA = ROM address).
        assert_eq!(space(at(0xB_8100, None), &win).0, Some(Space::M68k));
        // Phased to $8000, but its bytes are outside every window.
        assert_eq!(space(at(0xC_0000, Some(0x8000)), &win).0, Some(Space::M68k));
        // A Z80-CPU section is z80 whatever the windows say.
        let mut z = at(0x400, Some(0x10));
        z.cpu = Cpu::Z80;
        assert_eq!(space(z, &[]).0, Some(Space::Z80));
    }

    #[test]
    fn first_section_wins_and_reserve_only_is_ram() {
        let mut ram = sec(vec![Fragment::Reserve { count: 4, span: span() }], vec![("R", 0), ("A", 0)]);
        ram.vma_base = Some(0xFFFF_8000);
        let s = vec![ram, sec(vec![data(2)], vec![("A", 0)])];
        let p = PlacedSymbols::new(&s, &SymbolTable::new(), &[]);
        let r = p.label_record("R", SymbolKind::Label, None, false).unwrap();
        assert_eq!((r.kind, r.space, r.vma, r.lma), (SymbolKind::Ram, Some(Space::M68k), 0xFFFF_8000, None));
        assert_eq!(p.label("A").unwrap().section, 0);
    }
}
