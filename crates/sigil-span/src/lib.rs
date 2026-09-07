//! Source identifiers, byte-range spans, source maps, and diagnostics.

use std::fmt;
use std::sync::OnceLock;

/// Opaque identifier for a source file stored in a [`SourceMap`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SourceId(pub u32);

/// Half-open byte range `[start, end)` within a source file.
///
/// Hashable so a span can identify a diagnostic's site in a deduplication key.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Span {
    /// The source file that contains this span.
    pub source: SourceId,
    /// Byte offset of the first character (inclusive).
    pub start: u32,
    /// Byte offset past the last character (exclusive).
    pub end: u32,
}

impl Span {
    /// Combine two spans into the smallest span containing both. Assumes
    /// both spans belong to the same source; the result keeps `self`'s
    /// [`SourceId`].
    pub fn merge(self, other: Span) -> Span {
        Span { source: self.source, start: self.start.min(other.start), end: self.end.max(other.end) }
    }
}

/// Stores source texts and maps [`Span`]s back to human-readable positions.
///
/// Each source may carry a NAME — the file it was read from. A named source
/// renders a diagnostic's site as `file(line)` through [`SourceMap::label`]; an
/// unnamed one (a source added from a string with no file behind it) has no
/// label, so a renderer degrades to a bare message rather than attributing the
/// diagnostic to a filename it invented.
#[derive(Default)]
pub struct SourceMap {
    texts: Vec<String>,
    names: Vec<String>,
    /// Per source, the byte offset of every line start, filled on the first
    /// [`SourceMap::location`] call against that source. A source that never
    /// locates a span never pays for its index.
    line_starts: Vec<OnceLock<Vec<u32>>>,
}

impl SourceMap {
    /// Create an empty source map.
    pub fn new() -> Self {
        SourceMap { texts: Vec::new(), names: Vec::new(), line_starts: Vec::new() }
    }

    /// Add an unnamed source text and return its [`SourceId`].
    pub fn add(&mut self, text: String) -> SourceId {
        self.add_named(String::new(), text)
    }

    /// Add a source text under the name of the file it came from, and return its
    /// [`SourceId`].
    pub fn add_named(&mut self, name: String, text: String) -> SourceId {
        let id = SourceId(self.texts.len() as u32);
        self.texts.push(text);
        self.names.push(name);
        self.line_starts.push(OnceLock::new());
        id
    }

    /// Return the full source text for the given [`SourceId`].
    pub fn text(&self, id: SourceId) -> &str {
        &self.texts[id.0 as usize]
    }

    /// The name of a source, or `""` when it has none or the id is not in this map.
    pub fn name(&self, id: SourceId) -> &str {
        self.names.get(id.0 as usize).map(|s| s.as_str()).unwrap_or("")
    }

    /// Number of sources held.
    pub fn len(&self) -> usize {
        self.texts.len()
    }

    /// True when no source has been added.
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// `file(line)` for a span in a NAMED source — the shape AS itself reports
    /// (`smps-bug.asm(9): error: …`). `None` when the span's source is not in
    /// this map or carries no name, which is what a diagnostic belonging to no
    /// source line (a whole-run or placement failure) must produce.
    pub fn label(&self, span: Span) -> Option<String> {
        let idx = span.source.0 as usize;
        let name = self.names.get(idx)?;
        if name.is_empty() {
            return None;
        }
        let (line, _col) = self.location(span);
        Some(format!("{name}({line})"))
    }

    /// Return the 1-based `(line, column)` of `span.start` within its source.
    ///
    /// Columns count CHARACTERS, not bytes: a multi-byte UTF-8 character before
    /// `span.start` on the same line advances the column by one. An offset
    /// inside a multi-byte character counts that character, and an offset past
    /// the end of the text locates as the end of the text.
    ///
    /// Line lookup is a binary search over the source's line-start index, so
    /// the cost of one call does not grow with the offset; the index itself is
    /// built once per source, on its first call.
    pub fn location(&self, span: Span) -> (u32, u32) {
        let text = self.text(span.source);
        let starts = self.line_starts(span.source);
        let start = (span.start as usize).min(text.len());
        // Every line whose first byte is at or before `start`; the last of them
        // holds it. `starts[0]` is 0, so at least one qualifies.
        let line = starts.partition_point(|&s| s as usize <= start);
        let line_start = starts[line - 1] as usize;
        // A byte begins a character unless it is a UTF-8 continuation byte.
        let chars = text.as_bytes()[line_start..start].iter().filter(|&&b| b & 0xC0 != 0x80).count();
        (line as u32, chars as u32 + 1)
    }

    /// The byte offset of every line start in a source, in ascending order,
    /// beginning with 0. Built on first use and kept for the map's lifetime.
    fn line_starts(&self, id: SourceId) -> &[u32] {
        self.line_starts[id.0 as usize].get_or_init(|| {
            let text = &self.texts[id.0 as usize];
            let mut starts = vec![0u32];
            starts.extend(
                text.bytes().enumerate().filter(|&(_, b)| b == b'\n').map(|(i, _)| i as u32 + 1),
            );
            starts
        })
    }
}

/// Severity level of a [`Diagnostic`].
///
/// Hashable so a level can take part in a diagnostic's deduplication key.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Level {
    /// A hard error that prevents assembly.
    Error,
    /// A non-fatal warning.
    Warning,
    /// An informational note.
    Note,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
        };
        f.write_str(word)
    }
}

/// A single compiler diagnostic with a severity level, message, and primary span.
///
/// Renders as `<level>: <message> [<start>..<end>]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Severity of this diagnostic.
    pub level: Level,
    /// Human-readable message.
    pub message: String,
    /// The primary source span that triggered this diagnostic.
    pub primary: Span,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} [{}..{}]",
            self.level, self.message, self.primary.start, self.primary.end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_add_text_and_location() {
        let mut map = SourceMap::new();
        let id = map.add("nop\nld a, 5\n".to_string());
        // add/text round-trip
        assert_eq!(map.text(id), "nop\nld a, 5\n");
        // byte 0 => line 1, col 1
        assert_eq!(map.location(Span { source: id, start: 0, end: 3 }), (1, 1));
        // byte 4 ('l' of "ld", start of line 2) => line 2, col 1
        assert_eq!(map.location(Span { source: id, start: 4, end: 6 }), (2, 1));
        // byte 7 ('a' operand) => line 2, col 4
        assert_eq!(map.location(Span { source: id, start: 7, end: 8 }), (2, 4));
    }

    /// Columns count characters. The expected pairs are the character walk's
    /// answers, so any indexed lookup must reproduce them byte for byte,
    /// including an offset inside a multi-byte character and one past the end.
    #[test]
    fn location_columns_count_characters_not_bytes() {
        // Byte layout: 0 'a' 1 'b' 2 '\n' 3..5 'é' 5..8 '中' 8 'x' 9..13 '😀' 13 'y' 14 '\n' 15 'z'
        let utf = "ab\n\u{e9}\u{4e2d}x\u{1F600}y\nz";
        assert_eq!(utf.len(), 16);
        let mut map = SourceMap::new();
        let id = map.add(utf.to_string());
        let expected: [(u32, u32); 19] = [
            (1, 1), (1, 2), (1, 3),
            (2, 1), (2, 2), (2, 2), (2, 3), (2, 3), (2, 3), (2, 4), (2, 5), (2, 5), (2, 5), (2, 5), (2, 6),
            (3, 1),
            // Past the end: the end of the text.
            (3, 2), (3, 2), (3, 2),
        ];
        for (start, want) in expected.iter().enumerate() {
            let start = start as u32;
            assert_eq!(map.location(Span { source: id, start, end: start }), *want, "start {start}");
        }
        let empty = map.add(String::new());
        assert_eq!(map.location(Span { source: empty, start: 0, end: 0 }), (1, 1));
        assert_eq!(map.location(Span { source: empty, start: 5, end: 5 }), (1, 1));
        // A trailing newline opens an empty final line.
        let nl = map.add("x\n".to_string());
        assert_eq!(map.location(Span { source: nl, start: 2, end: 2 }), (2, 1));
        assert_eq!(map.location(Span { source: nl, start: 3, end: 3 }), (2, 1));
    }

    /// Locating a span at the bottom of a large source costs the same as one at
    /// the top, within noise. A walk from byte 0 fails this by three orders of
    /// magnitude in either build profile (measured 773 us per bottom lookup in
    /// release; the top lookup is a few nanoseconds).
    #[test]
    fn location_at_the_bottom_costs_no_more_than_at_the_top() {
        use std::time::Instant;
        let line = "\tmove.w\t#$1234,(a0)+\t; a comment of typical width\n";
        let mut text = String::new();
        while text.len() < 1_600_000 {
            text.push_str(line);
        }
        let len = text.len() as u32;
        let lines = text.matches('\n').count() as u32;
        let mut map = SourceMap::new();
        let id = map.add(text);
        let n = 5229;
        // The first call pays for the index; every timed call below is a lookup.
        assert_eq!(map.location(Span { source: id, start: len - 1, end: len }), (lines, line.len() as u32));

        let time = |start: u32| {
            let t0 = Instant::now();
            for _ in 0..n {
                std::hint::black_box(map.location(Span { source: id, start, end: start + 1 }));
            }
            t0.elapsed()
        };
        let top = time(0);
        let bottom = time(len - 1);
        assert!(
            bottom < std::time::Duration::from_secs(2),
            "{n} bottom lookups took {bottom:?}; the lookup is walking the file"
        );
        // Both sides run under the same load, so the ratio is load-independent.
        let ratio = bottom.as_secs_f64() / top.as_secs_f64().max(1e-9);
        assert!(ratio < 1000.0, "bottom {bottom:?} is {ratio:.0}x the top {top:?}; the lookup is walking the file");
    }

    #[test]
    fn span_and_source_id_are_copy_with_public_fields() {
        // Construct Span literally via its public fields.
        let span = Span { source: SourceId(7), start: 2, end: 5 };
        // Copy it, then keep using the original (requires Span: Copy).
        let copied = span;
        assert_eq!(span.source, SourceId(7));
        assert_eq!(span.start, 2);
        assert_eq!(span.end, 5);
        assert_eq!(copied, span);

        // SourceId is Copy with a public field.
        let id = SourceId(7);
        let id_copied = id;
        assert_eq!(id_copied, id);
        assert_eq!(id.0, 7);
    }

    #[test]
    fn label_names_the_source_the_span_belongs_to() {
        let mut map = SourceMap::new();
        let root = map.add_named("root.asm".to_string(), "nop\nnop\nnop\n".to_string());
        let inc = map.add_named("sub/part.asm".to_string(), "a\nb\nc\nd\n".to_string());
        let anon = map.add("nop\n".to_string());

        // Each id resolves against ITS OWN text: byte 8 is line 3 of the root and
        // line 5 of nothing else.
        assert_eq!(
            map.label(Span { source: root, start: 8, end: 9 }).as_deref(),
            Some("root.asm(3)")
        );
        // Byte 6 of the included file is line 4 — the includer's name never appears.
        assert_eq!(
            map.label(Span { source: inc, start: 6, end: 7 }).as_deref(),
            Some("sub/part.asm(4)")
        );
        // A source with no name, and an id in no map at all, both decline to invent
        // a location rather than defaulting to the first file.
        assert_eq!(map.label(Span { source: anon, start: 0, end: 1 }), None);
        assert_eq!(map.label(Span { source: SourceId(u32::MAX), start: 0, end: 0 }), None);
        assert_eq!(map.name(root), "root.asm");
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn diagnostic_display_matches_contract() {
        let diag = Diagnostic {
            level: Level::Error,
            message: "unexpected token".to_string(),
            primary: Span { source: SourceId(0), start: 2, end: 5 },
        };
        assert_eq!(diag.to_string(), "error: unexpected token [2..5]");
    }
}
