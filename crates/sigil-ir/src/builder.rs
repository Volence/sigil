//! `IrBuilder`: the concrete `IrStreamer` that materialises a `Module`.

use crate::backend::{Cpu, IrStreamer};
use crate::{DataFragment, Fixup, Fragment, Label, Module, Section};
use sigil_span::{Diagnostic, Span};

/// One section under construction: metadata + a running byte cursor.
struct OpenSection {
    name: String,
    cpu: Cpu,
    vma_base: Option<u32>,
    labels: Vec<Label>,
    fragments: Vec<Fragment>,
    cursor: u32, // VMA/PC offset from section start (counts Data+Fill+Reserve)
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
                lma: o.vma_base.unwrap_or(0),
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
            labels: Vec::new(),
            fragments: Vec::new(),
            cursor: 0,
        });
    }

    fn emit_data(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span) {
        let n = bytes.len() as u32;
        let s = self.section_mut();
        s.fragments.push(Fragment::Data(DataFragment { bytes: bytes.to_vec(), fixups, span }));
        s.cursor += n;
    }

    fn emit_fill(&mut self, count: u32, value: u8, span: Span) {
        let s = self.section_mut();
        s.fragments.push(Fragment::Fill { value, count, span });
        s.cursor += count;
    }

    fn reserve(&mut self, count: u32, span: Span) {
        let s = self.section_mut();
        s.fragments.push(Fragment::Reserve { count, span });
        s.cursor += count;
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
    fn diag_is_collected_and_finish_returns_it() {
        use sigil_span::{Diagnostic, Level};
        let mut b = IrBuilder::new();
        b.switch_section("s", Cpu::Z80, None);
        b.diag(Diagnostic { level: Level::Error, message: "boom".into(), primary: span() });
        let (_m, diags) = b.finish();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "boom");
    }
}
