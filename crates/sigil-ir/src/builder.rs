//! `IrBuilder`: the concrete `IrStreamer` that materialises a `Module`.

use crate::backend::{Cpu, IrStreamer};
use crate::{DataFragment, Fixup, Fragment, Label, Module, Section};
use sigil_span::{Diagnostic, Span};

/// One section under construction: metadata + a running byte cursor.
struct OpenSection {
    name: String,
    cpu: Cpu,
    vma_base: Option<u32>,
    lma: u32, // physical load address of this section's start
    labels: Vec<Label>,
    fragments: Vec<Fragment>,
    cursor: u32,     // VMA/PC offset from section start (counts Data+Fill+Reserve)
    max_offset: u32, // highest `cursor` ever reached (the org back-patch "extent")
}

impl OpenSection {
    /// Track the running cursor's high-water mark after every mutation.
    fn bump_max(&mut self) {
        if self.cursor > self.max_offset {
            self.max_offset = self.cursor;
        }
    }
}

/// Concrete `IrStreamer` that accumulates `Section`s and yields a `Module`.
#[derive(Default)]
pub struct IrBuilder {
    done: Vec<Section>,
    open: Option<OpenSection>,
    diags: Vec<Diagnostic>,
}

impl IrBuilder {
    pub fn new() -> Self {
        IrBuilder::default()
    }

    /// Close the open section (if any) into `done`.
    fn close(&mut self) {
        if let Some(o) = self.open.take() {
            self.done.push(Section {
                name: o.name,
                cpu: o.cpu,
                vma_base: o.vma_base,
                lma: o.lma,
                labels: o.labels,
                fragments: o.fragments,
            });
        }
    }

    /// Byte offset within the currently-open section (0 if none open). The
    /// single source of truth for the front-end's `$`/label position.
    pub fn current_offset(&self) -> u32 {
        self.open.as_ref().map_or(0, |o| o.cursor)
    }

    /// The highest `current_offset()` ever reached in the currently-open
    /// section (0 if none open). This is the "extent" a front-end's `org`
    /// directive compares its target against to decide whether it's an
    /// in-section back-patch seek (target within the extent already written)
    /// or a forward jump into brand-new territory (target beyond it).
    pub fn extent(&self) -> u32 {
        self.open.as_ref().map_or(0, |o| o.max_offset)
    }

    /// Open a new section with an EXPLICIT physical load address `lma` (distinct
    /// from `vma_base` under phase). Closes any currently-open section first. This
    /// is the front-end's door: it computes a continuous physical LMA counter and
    /// a phased VMA base, which differ whenever a `phase` displacement is active.
    /// The trait's [`IrStreamer::switch_section`] keeps the old default
    /// (`lma = vma_base.unwrap_or(0)`) for backends/tests that don't model phase.
    pub fn switch_section_lma(&mut self, name: &str, cpu: Cpu, vma_base: Option<u32>, lma: u32) {
        self.close();
        self.open = Some(OpenSection {
            name: name.to_string(),
            cpu,
            vma_base,
            lma,
            labels: Vec::new(),
            fragments: Vec::new(),
            cursor: 0,
            max_offset: 0,
        });
    }

    /// Seek the open section's write cursor to `target` (backward or forward)
    /// and push an `Org` marker fragment recording `fill` (the byte that fills
    /// any forward gap when the fragments are later replayed into bytes — see
    /// `Section::image_bytes`). Panics if no section is open (front-end bug).
    pub fn seek(&mut self, target: u32, fill: u8, span: Span) {
        let s = self.section_mut();
        s.fragments.push(Fragment::Org { target, fill, span });
        s.cursor = target;
        s.bump_max();
    }

    /// Consume the builder: close the open section and return the module + diags.
    pub fn finish(mut self) -> (Module, Vec<Diagnostic>) {
        self.close();
        (Module { sections: self.done }, self.diags)
    }

    /// Borrow the open section, panicking if a fragment is emitted with no
    /// section open (a front-end bug — the front-end always opens a section
    /// before emitting).
    fn section_mut(&mut self) -> &mut OpenSection {
        self.open
            .as_mut()
            .expect("emit with no open section (front-end must switch_section first)")
    }
}

impl IrStreamer for IrBuilder {
    fn switch_section(&mut self, name: &str, cpu: Cpu, vma_base: Option<u32>) {
        self.close();
        self.open = Some(OpenSection {
            name: name.to_string(),
            cpu,
            vma_base,
            lma: vma_base.unwrap_or(0),
            labels: Vec::new(),
            fragments: Vec::new(),
            cursor: 0,
            max_offset: 0,
        });
    }

    fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span) {
        let n = bytes.len() as u32;
        let s = self.section_mut();
        s.fragments.push(Fragment::Data(DataFragment { bytes: bytes.to_vec(), fixups, span }));
        s.cursor += n;
        s.bump_max();
    }

    fn emit_fill(&mut self, count: u32, value: u8, span: Span) {
        let s = self.section_mut();
        s.fragments.push(Fragment::Fill { value, count, span });
        s.cursor += count;
        s.bump_max();
    }

    fn reserve(&mut self, count: u32, span: Span) {
        let s = self.section_mut();
        s.fragments.push(Fragment::Reserve { count, span });
        s.cursor += count;
        s.bump_max();
    }

    fn emit_fragment(&mut self, frag: Fragment, advance: u32) {
        let s = self.section_mut();
        s.fragments.push(frag);
        s.cursor += advance;
        s.bump_max();
    }

    fn define_label(&mut self, name: &str) {
        let s = self.section_mut();
        let offset = s.cursor;
        s.labels.push(Label { name: name.to_string(), offset });
    }

    fn diag(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::IrBuilder;
    use crate::backend::{Cpu, IrStreamer};
    use crate::{Fixup, FixupKind, Expr, Fragment};
    use sigil_span::{SourceId, Span};

    fn span() -> Span { Span { source: SourceId(0), start: 0, end: 0 } }

    #[test]
    fn builds_two_sections_with_labels_fills_and_fixups() {
        let mut b = IrBuilder::new();
        // Region A @ vma 0.
        b.switch_section("regionA", Cpu::Z80, Some(0x0000));
        b.define_label("Start");
        b.emit_data(&[0x00], vec![], span()); // nop
        b.emit_data(
            &[0x11, 0x00, 0x00],
            vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 1, target: Expr::Sym("Tab".into()) }],
            span(),
        );
        // Region B @ vma 0x8000.
        b.switch_section("regionB", Cpu::Z80, Some(0x8000));
        b.emit_fill(0x45F, 0xAA, span());
        b.define_label("Tab");
        b.emit_data(&[0x9A, 0xD6], vec![], span());
        b.reserve(4, span());

        let (module, diags) = b.finish();
        assert!(diags.is_empty());
        assert_eq!(module.sections.len(), 2);

        let a = &module.sections[0];
        assert_eq!(a.name, "regionA");
        assert_eq!(a.vma_base, Some(0x0000));
        assert_eq!(a.lma, 0x0000); // default lma = vma_base
        assert_eq!(a.labels, vec![crate::Label { name: "Start".into(), offset: 0 }]);
        assert_eq!(a.fragments.len(), 2);

        let bsec = &module.sections[1];
        assert_eq!(bsec.vma_base, Some(0x8000));
        // Label `Tab` lands after the 0x45F fill.
        assert_eq!(bsec.labels, vec![crate::Label { name: "Tab".into(), offset: 0x45F }]);
        // Fill + Data(2) contribute image; Reserve(4) does not.
        assert_eq!(bsec.image_len(), 0x45F + 2);
        assert_eq!(bsec.vma_len(), 0x45F + 2 + 4);
        assert!(matches!(bsec.fragments[2], Fragment::Reserve { count: 4, .. }));
    }

    #[test]
    fn current_offset_tracks_open_section_cursor() {
        let mut b = IrBuilder::new();
        // No section open yet ⇒ offset 0.
        assert_eq!(b.current_offset(), 0);
        b.switch_section("s", Cpu::Z80, Some(0x8000));
        assert_eq!(b.current_offset(), 0);
        b.emit_data(&[0x00, 0x3E, 0x0C], vec![], span());
        assert_eq!(b.current_offset(), 3);
        b.emit_fill(4, 0xAA, span());
        assert_eq!(b.current_offset(), 3 + 4);
        b.reserve(8, span());
        assert_eq!(b.current_offset(), 3 + 4 + 8);
        // Switching sections resets the cursor.
        b.switch_section("t", Cpu::Z80, None);
        assert_eq!(b.current_offset(), 0);
    }

    #[test]
    fn emit_fragment_pushes_raw_fragment_and_advances_cursor_by_given_amount() {
        // The M1.C T5c door for `Fragment::JmpJsrSym`: the caller supplies the
        // fragment (already built by the backend) and the baseline (abs.w)
        // cursor advance, since the real width is chosen later by
        // `resolve_layout`, not known yet at emission time.
        let mut b = IrBuilder::new();
        b.switch_section("s", Cpu::M68000, None);
        let frag = Fragment::JmpJsrSym { is_jsr: true, target: Expr::Sym("Sub".into()), span: span() };
        b.emit_fragment(frag.clone(), 4);
        assert_eq!(b.current_offset(), 4);
        b.define_label("After");
        let (module, _diags) = b.finish();
        let sec = &module.sections[0];
        assert_eq!(sec.fragments, vec![frag]);
        assert_eq!(sec.labels, vec![crate::Label { name: "After".into(), offset: 4 }]);
    }

    #[test]
    fn diag_is_collected_and_finish_returns_it() {
        use sigil_span::{Diagnostic, Level};
        let mut b = IrBuilder::new();
        b.switch_section("s", Cpu::Z80, None);
        b.diag(Diagnostic { level: Level::Error, message: "boom".into(), primary: span() });
        let (_m, diags) = b.finish();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "boom");
    }

    #[test]
    fn extent_tracks_high_water_mark_across_a_backward_seek() {
        // The `parallax_section_end` shape: write 4 bytes, seek back to 0 (a
        // back-patch), write 1 byte. `extent()` must stay at the high-water
        // mark (4) throughout — a subsequent `org` to anything <= 4 is still a
        // safe in-section seek, not a forward jump into new territory.
        let mut b = IrBuilder::new();
        b.switch_section("s", Cpu::M68000, None);
        assert_eq!(b.extent(), 0);
        b.emit_data(&[0, 1, 2, 3], vec![], span());
        assert_eq!(b.extent(), 4);
        assert_eq!(b.current_offset(), 4);
        b.seek(0, 0x00, span());
        assert_eq!(b.current_offset(), 0); // cursor moved back...
        assert_eq!(b.extent(), 4); // ...but the extent remembers the high-water mark
        b.emit_data(&[0x63], vec![], span());
        assert_eq!(b.current_offset(), 1);
        assert_eq!(b.extent(), 4); // still 4: this write didn't exceed the prior extent
        b.seek(4, 0x00, span());
        assert_eq!(b.current_offset(), 4);
        assert_eq!(b.extent(), 4);

        let (module, _diags) = b.finish();
        let sec = &module.sections[0];
        assert!(matches!(sec.fragments[1], Fragment::Org { target: 0, .. }));
        assert!(matches!(sec.fragments[3], Fragment::Org { target: 4, .. }));
    }

    #[test]
    fn seek_forward_beyond_extent_still_updates_extent() {
        // `seek` itself is a mechanical cursor move; it doesn't refuse a target
        // beyond the current extent (the front-end's `directive_org` is what
        // decides whether such a target should instead close the section and
        // re-phase — this proves `seek`'s own bookkeeping stays correct either way).
        let mut b = IrBuilder::new();
        b.switch_section("s", Cpu::M68000, None);
        b.emit_data(&[1, 2], vec![], span());
        assert_eq!(b.extent(), 2);
        b.seek(10, 0x00, span());
        assert_eq!(b.current_offset(), 10);
        assert_eq!(b.extent(), 10);
    }
}
