//! Section base alignment for the new driver: the 68000 evenness default, raised only by
//! a declared `[[align]]` row.
//!
//! Spec §2 ("facts that are not Aeon's at all"): of the 108 rows the old driver kept in
//! sigil-harness `section_align.rs` (deleted with it, G0b Task 11), 102 said only "a 68000
//! section base is even". That is
//! an ISA fact, so it is the driver's default here, never a row in either description.
//! The six real requirements are `[[align]]` rows: five in `games/sonic4/game.toml`
//! (the Z80 bank windows and the sound fold's mod-8 chain) and `ObjCodeBase` in
//! `engine/engine.toml`.
//!
//! The default covers every section the walk places in the ROM image, whatever CPU its
//! bytes target: each one is either 68000 code or data the 68000 reads by word or long
//! (the inline Z80 idle program is copied into Z80 RAM by a 68000 word loop), so an odd
//! base faults. The old table agreed: its `Z80_IdleProgram` row was 2.

use std::collections::BTreeMap;

use crate::describe::AlignRow;

/// The 68000 rule: every section base in the ROM image is even.
pub const EVEN: u32 = 2;

/// Where a section's alignment came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignSource {
    /// The 68000 evenness default.
    Even,
    /// A declared `[[align]]` row, by the description that declares it.
    Row { origin: String },
}

/// The alignment rule a build places with: the evenness default plus the declared rows of
/// the engine and the game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlignTable {
    rows: BTreeMap<String, (u32, String)>,
}

impl AlignTable {
    /// Build the table from `(origin, rows)` pairs (`origin` names the file in errors).
    /// A label declared twice, in one file or across two, is refused: two declarations of
    /// one section's requirement is a fork, and neither may silently win. A row whose
    /// value is not a power of two above 1 is refused (the loaders refuse it too; this is
    /// the table's own guard for rows built in code).
    pub fn new<'a>(sources: impl IntoIterator<Item = (&'a str, &'a [AlignRow])>) -> Result<AlignTable, String> {
        let mut rows: BTreeMap<String, (u32, String)> = BTreeMap::new();
        for (origin, list) in sources {
            for r in list {
                if r.align < 2 || !r.align.is_power_of_two() {
                    return Err(format!(
                        "[align.bad-row] {origin}: `{}` declares {}, which is not a power of two above 1",
                        r.label, r.align
                    ));
                }
                if let Some((_, first)) = rows.get(&r.label) {
                    return Err(format!(
                        "[align.duplicate] `{}` is declared by {first} and again by {origin}; one \
                         section has one alignment requirement, so keep one row",
                        r.label
                    ));
                }
                rows.insert(r.label.clone(), (r.align, origin.to_string()));
            }
        }
        Ok(AlignTable { rows })
    }

    /// The alignment the section headed by `head` must be placed at: its row, else
    /// [`EVEN`].
    pub fn required_for(&self, head: &str) -> u32 {
        self.rows.get(head).map_or(EVEN, |(a, _)| *a)
    }

    /// Where [`Self::required_for`]'s answer for `head` comes from.
    pub fn source_of(&self, head: &str) -> AlignSource {
        match self.rows.get(head) {
            Some((_, origin)) => AlignSource::Row { origin: origin.clone() },
            None => AlignSource::Even,
        }
    }

    /// The declared rows, by label.
    pub fn rows(&self) -> impl Iterator<Item = (&str, u32)> {
        self.rows.iter().map(|(l, (a, _))| (l.as_str(), *a))
    }

    /// The base a section headed by `head` packs to when the walk's cursor stands at
    /// `running`: `running` rounded up to [`Self::required_for`]. The one spelling of
    /// that arithmetic in the new driver; a sound prediction that has to agree with the
    /// walk calls this.
    pub fn packed_base(&self, running: u32, head: &str) -> u32 {
        let a = self.required_for(head);
        running.div_ceil(a) * a
    }

    /// Every ROM section of `resolved` whose base its requirement forbids, as one refusal
    /// naming each (`[layout.alignment-violated]`). The walk packs by this table, so this
    /// measures the sections it places by another rule: an anchor's address, a label-less
    /// blob's contiguity.
    pub fn validate_resolved(&self, resolved: &[sigil_ir::Section]) -> Result<(), String> {
        let mut faults = Vec::new();
        for s in resolved.iter().filter(|s| crate::place::is_rom_section(s)) {
            let Some(head) = crate::place::head_label(s) else { continue };
            let a = self.required_for(head);
            if !s.lma.is_multiple_of(a) {
                let why = match self.source_of(head) {
                    AlignSource::Even => "the 68000 evenness default".to_string(),
                    AlignSource::Row { origin } => format!("its [[align]] row in {origin}"),
                };
                faults.push(format!(
                    "section `{}` (head label `{head}`) needs alignment {a} ({why}) but is placed at {:#x}",
                    s.name, s.lma
                ));
            }
        }
        if faults.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "[layout.alignment-violated] {} section(s):\n  - {}",
                faults.len(),
                faults.join("\n  - ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, align: u32) -> AlignRow {
        AlignRow { label: label.into(), align }
    }

    #[test]
    fn default_is_even_and_a_row_raises_it() {
        let engine = [row("ObjCodeBase", 0x10000)];
        let game = [row("Sfx_33", 8)];
        let t = AlignTable::new([("engine.toml", &engine[..]), ("game.toml", &game[..])]).unwrap();
        assert_eq!(t.required_for("Anything"), EVEN);
        assert_eq!(t.required_for("ObjCodeBase"), 0x10000);
        assert_eq!(t.packed_base(0x3, "Anything"), 0x4);
        assert_eq!(t.packed_base(0x5BB1A, "Sfx_33"), 0x5BB20);
        assert_eq!(t.packed_base(0x8F00, "ObjCodeBase"), 0x10000);
        assert_eq!(t.source_of("Sfx_33"), AlignSource::Row { origin: "game.toml".into() });
    }

    #[test]
    fn a_label_declared_twice_is_refused() {
        let a = [row("Sfx_33", 8)];
        let b = [row("Sfx_33", 16)];
        let err = AlignTable::new([("engine.toml", &a[..]), ("game.toml", &b[..])]).unwrap_err();
        assert!(err.contains("engine.toml") && err.contains("game.toml") && err.contains("Sfx_33"), "{err}");
        let bad = [row("X", 3)];
        assert!(AlignTable::new([("g", &bad[..])]).unwrap_err().contains("power of two"));
    }

    // ── Moved from the profile-driven driver's `declared_alignment_tests` (G0b Task 11),
    // onto this table: the post-resolve half (`validate_resolved`) and the walk's rounding
    // (`packed_base`). The pre-walk "every head label has a row" half has no successor: an
    // undeclared section takes the evenness default by design (spec section 2). ──

    fn sec(name: &str, label: &str, lma: u32) -> sigil_ir::Section {
        let span = sigil_span::Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        sigil_ir::Section {
            name: name.into(),
            cpu: sigil_ir::Cpu::M68000,
            vma_base: None,
            lma,
            labels: vec![sigil_ir::Label { name: label.into(), offset: 0 }],
            fragments: vec![sigil_ir::Fragment::Data(sigil_ir::DataFragment { bytes: vec![0u8; 0x10], fixups: vec![], span })],
            placement: sigil_ir::SectionPlacement::Pinned,
            reserved_span: 0x10,
            group: None,
            bank: None,
            space: sigil_ir::AddressSpace::Image,
            equ_syms: Vec::new(),
        }
    }

    /// The sound rows the moved tests were written against: `Sfx_33` 8 (aeon's mod-8 fold
    /// wall), `Dac_Temp_Blip` `$8000` (one Z80 `SetBank` window).
    fn sound_table() -> AlignTable {
        let game = [row("Sfx_33", 8), row("Dac_Temp_Blip", 0x8000)];
        AlignTable::new([("games/sonic4/game.toml", &game[..])]).unwrap()
    }

    #[test]
    fn a_declared_section_at_a_conforming_base_passes() {
        sound_table().validate_resolved(&[sec("sfx_bank_blob", "Sfx_33", 0xA3B20)]).unwrap();
    }

    /// A base the declaration forbids is refused, naming the section, its head label, the
    /// requirement, the row's file and the base.
    #[test]
    fn a_resolved_base_that_violates_the_declaration_is_refused() {
        let e = sound_table().validate_resolved(&[sec("sfx_bank_blob", "Sfx_33", 0xA3B24)]).unwrap_err();
        assert!(e.contains("[layout.alignment-violated]"), "{e}");
        assert!(e.contains("`sfx_bank_blob`") && e.contains("`Sfx_33`"), "{e}");
        assert!(e.contains("needs alignment 8"), "{e}");
        assert!(e.contains("games/sonic4/game.toml"), "the row's file is not named: {e}");
        assert!(e.contains("0xa3b24"), "the base is not named: {e}");
    }

    /// The Z80 bank window is a requirement the walk never rounds to (the bank heads are
    /// anchor-held), so this check is the only instrument on it, and it reads the
    /// requirement itself, not a capped quantum.
    #[test]
    fn the_bank_window_requirement_is_measured_on_the_resolved_anchor() {
        let t = sound_table();
        t.validate_resolved(&[sec("dac_banks", "Dac_Temp_Blip", 0x90000)]).unwrap();
        let e = t.validate_resolved(&[sec("dac_banks", "Dac_Temp_Blip", 0x90010)]).unwrap_err();
        assert!(e.contains("needs alignment 32768"), "{e}");
        assert!(e.contains("0x90010"), "{e}");
    }

    /// A section no row names is measured against the evenness default (the successor of
    /// `an_unpinned_section_needs_a_declaration_and_is_measured_after_the_walk`, whose
    /// "needs a declaration" half the spec replaced by the default).
    #[test]
    fn an_undeclared_section_is_measured_against_the_evenness_default() {
        let t = sound_table();
        t.validate_resolved(&[sec("palette", "Palette_LoadPal", 0x2000)]).unwrap();
        let e = t.validate_resolved(&[sec("palette", "Palette_LoadPal", 0x2001)]).unwrap_err();
        assert!(e.contains("`Palette_LoadPal`") && e.contains("evenness default"), "{e}");
    }

    /// A label-less blob has nothing to declare under, and is skipped.
    #[test]
    fn a_label_less_blob_is_out_of_both_halves_scope() {
        let mut blob = sec("boot_blob", "unused", 0x3D7);
        blob.labels.clear();
        sound_table().validate_resolved(std::slice::from_ref(&blob)).unwrap();
    }

    /// Every fault in one run, so a report names all of them rather than the first.
    #[test]
    fn every_faulting_section_is_named_in_one_report() {
        let secs = [sec("sfx_bank_blob", "Sfx_33", 0x1004), sec("palette", "Palette_LoadPal", 0x2001)];
        let e = sound_table().validate_resolved(&secs).unwrap_err();
        assert!(e.contains("2 section(s)"), "{e}");
        assert!(e.contains("`Sfx_33`") && e.contains("`Palette_LoadPal`"), "{e}");
    }

    /// The walk's rounding (and seam 2's prediction) is a function of the running cursor
    /// and the declaration for the head label, and nothing else; an undeclared head label
    /// rounds to the evenness default.
    #[test]
    fn packed_chained_base_rounds_to_the_declared_alignment_only() {
        let t = sound_table();
        assert_eq!(t.packed_base(0x1011, "Sfx_33"), 0x1018);
        assert_eq!(t.packed_base(0x1011, "GameLoop"), 0x1012);
        assert_eq!(t.packed_base(0x1011, "Dac_Temp_Blip"), 0x8000);
        assert_eq!(t.packed_base(0x1018, "Sfx_33"), 0x1018, "already aligned: unmoved");
        assert_eq!(t.packed_base(0x1011, "NoSuchHeadLabelAnywhere"), 0x1012);
    }
}
