//! IR for the Sigil assembler: fragments, sections, symbols, streaming, and image assembly.

use sigil_span::Span;

/// A contiguous run of raw bytes with source provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataFragment {
    /// The raw bytes emitted by the assembler front-end.
    pub bytes: Vec<u8>,
    /// The source span that produced these bytes.
    pub span: Span,
}

/// A single unit of assembled content inside a [`Section`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Fragment {
    /// A raw-byte fragment.
    Data(DataFragment),
}

/// A named, ordered collection of [`Fragment`]s that maps to a contiguous output region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    /// Section name (e.g. `"text"`, `"data"`).
    pub name: String,
    /// Ordered list of fragments that make up this section.
    pub fragments: Vec<Fragment>,
}

impl Section {
    /// Concatenate every fragment's bytes in order into a single image buffer.
    pub fn image_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for frag in &self.fragments {
            match frag {
                Fragment::Data(data) => out.extend_from_slice(&data.bytes),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_span::{SourceId, Span};

    #[test]
    fn data_fragment_holds_bytes_and_span() {
        let span = Span { source: SourceId(0), start: 2, end: 5 };
        let frag = DataFragment { bytes: vec![0x00, 0x3E, 0x05], span };
        assert_eq!(frag.bytes, vec![0x00, 0x3E, 0x05]);
        assert_eq!(frag.span, span);
    }

    #[test]
    fn section_image_bytes_concatenates_fragments() {
        let span = Span { source: SourceId(0), start: 0, end: 0 };
        let section = Section {
            name: "text".to_string(),
            fragments: vec![
                Fragment::Data(DataFragment { bytes: vec![0x00, 0x3E], span }),
                Fragment::Data(DataFragment { bytes: vec![0x05], span }),
            ],
        };
        assert_eq!(section.image_bytes(), vec![0x00, 0x3E, 0x05]);
    }
}
