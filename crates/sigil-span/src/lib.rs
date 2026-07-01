//! Source identifiers, byte-range spans, source maps, and diagnostics.

use std::fmt;

/// Opaque identifier for a source file stored in a [`SourceMap`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SourceId(pub u32);

/// Half-open byte range `[start, end)` within a source file.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Span {
    /// The source file that contains this span.
    pub source: SourceId,
    /// Byte offset of the first character (inclusive).
    pub start: u32,
    /// Byte offset past the last character (exclusive).
    pub end: u32,
}

/// Stores source texts and maps [`Span`]s back to human-readable positions.
#[derive(Default)]
pub struct SourceMap {
    texts: Vec<String>,
}

impl SourceMap {
    /// Create an empty source map.
    pub fn new() -> Self {
        SourceMap { texts: Vec::new() }
    }

    /// Add a source text and return its [`SourceId`].
    pub fn add(&mut self, text: String) -> SourceId {
        let id = SourceId(self.texts.len() as u32);
        self.texts.push(text);
        id
    }

    /// Return the full source text for the given [`SourceId`].
    pub fn text(&self, id: SourceId) -> &str {
        &self.texts[id.0 as usize]
    }

    /// Return the 1-based `(line, column)` of `span.start` within its source.
    pub fn location(&self, span: Span) -> (u32, u32) {
        let text = self.text(span.source);
        let mut line = 1u32;
        let mut col = 1u32;
        for (i, ch) in text.char_indices() {
            if i as u32 >= span.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

/// Severity level of a [`Diagnostic`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
    fn diagnostic_display_matches_contract() {
        let diag = Diagnostic {
            level: Level::Error,
            message: "unexpected token".to_string(),
            primary: Span { source: SourceId(0), start: 2, end: 5 },
        };
        assert_eq!(diag.to_string(), "error: unexpected token [2..5]");
    }
}
