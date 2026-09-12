//! Source identifiers, byte-range spans, source maps, and diagnostics.

use std::fmt;
use std::sync::{Arc, OnceLock};

pub mod read_set;

/// The bit that marks a [`SourceId`] as an EXPANSION rather than a file. File
/// ids count up from 0 in the order files are added, so they never reach it,
/// and keeping expansions in their own range leaves every file id exactly the
/// number it would have been without them.
const EXPANSION_BIT: u32 = 0x8000_0000;

/// One past the last expansion index handed out. `SourceId(u32::MAX)` is the
/// id the front ends use for a diagnostic that belongs to no source line, and
/// it has this bit set, so the index it would decode to is never allocated.
const EXPANSION_LIMIT: u32 = !EXPANSION_BIT;

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
///
/// ## Expansions
///
/// Besides files, a map holds EXPANSIONS ([`SourceMap::add_expansion`]): one
/// entry per run of a macro body or a loop body, each under its own
/// [`SourceId`]. An expansion owns no text. It names the file its lines were
/// written in, the first of those lines, the span that entered it, and the
/// [`Frame`] it prints as. Lines executed inside it carry its id, so a span
/// taken from one of them says WHICH run it came from, and that is the only
/// thing that lets a report name the line that called a macro rather than
/// the macro's body: two calls of one macro execute the same body text, and
/// only the id tells them apart.
///
/// Every accessor except [`SourceMap::label`] reads an expansion's span as the
/// same span in the file it was written in ([`SourceMap::physical`]), so
/// [`text`](SourceMap::text), [`name`](SourceMap::name) and
/// [`location`](SourceMap::location) answer exactly what they answer for the
/// body's own lines. Only `label` renders the trail.
#[derive(Default)]
pub struct SourceMap {
    /// Every file added, in the order added. Index `k` is the source id `k`.
    files: Vec<File>,
    /// Every expansion added, in the order added. Index `k` is the source id
    /// `EXPANSION_BIT | k`.
    expansions: Vec<Expansion>,
}

/// One file's text and name.
///
/// Kept together rather than as parallel vectors so the map is two vectors
/// wide: it travels by value inside every front end's failure value.
struct File {
    text: String,
    name: String,
    /// The byte offset of every line start, filled on the first
    /// [`SourceMap::location`] call against this file. A file that never
    /// locates a span never pays for its index.
    line_starts: OnceLock<Vec<u32>>,
}

/// One run of a macro or loop body. See [`SourceMap`]'s "Expansions" section.
struct Expansion {
    /// The FILE the body's lines were written in: never another expansion, so
    /// resolving a span to its text is one step.
    backing: SourceId,
    /// Byte offset, in `backing`, of the body's first line. A frame's line
    /// number counts from it.
    body_start: u32,
    /// The span that entered this run: the macro call, or for a loop the line
    /// that closes it (see [`Frame`]). It belongs to the ENCLOSING source,
    /// which is a file or an earlier expansion.
    call: Span,
    frame: Frame,
}

/// What one expansion prints as in a call-site trail, in the reference
/// assembler's own spelling.
///
/// asl (md5 `61e672562465725a8c102288a7da9098`) prints a diagnostic raised
/// inside an expansion at the OUTERMOST call's `file(line)`, followed by one
/// frame per expansion from the outside in, and the column last:
///
/// ```text
/// p2_nested.asm(9) outer(2) inner(1):9: error #1200: unknown instruction
/// p4_rept.asm(8) mymac(3) REPT 1(1):9: error #1200: unknown instruction
/// q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):9: error #1200: ...
/// ```
///
/// A frame's number is a line of that expansion's BODY, counted from 1 at the
/// body's first line: the line the next frame was entered from, or for the
/// innermost frame the line the diagnostic is on. A loop is entered from its
/// CLOSING line, because asl collects the whole block before it runs it: the
/// `mymac(3)` above is the `endm` of the `rept`, not the line inside it.
/// A macro frame is followed by a space and a loop frame is not, which is why
/// `REPT 1(2)inner(1)` has none. The measurements, with their probes, are in
/// `docs/superpowers/notes/2026-09-12-as-macro-diag-call-site.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    /// A macro expansion: `name(line)`.
    Macro(Arc<str>),
    /// Iteration `n` (from 1) of a `rept`: `REPT n(line)`.
    Rept(u32),
    /// Iteration `n` (from 1) of a `while`: `WHILE n/line`, with a slash.
    While(u32),
    /// An `irp` iteration: `IRP:next(line)`, where `next` is the text of the
    /// item AFTER this iteration's, empty on the last iteration.
    Irp(Arc<str>),
    /// An `irpc` iteration: `IRPC:'c'(line)`, where `c` is the character
    /// AFTER this iteration's; the last iteration prints `IRPC:'(line)`.
    Irpc(Option<char>),
}

impl Frame {
    fn render(&self, line: u32, out: &mut String) {
        use std::fmt::Write;
        // Writing to a String cannot fail.
        let _ = match self {
            Frame::Macro(name) => write!(out, "{name}({line})"),
            Frame::Rept(n) => write!(out, "REPT {n}({line})"),
            Frame::While(n) => write!(out, "WHILE {n}/{line}"),
            Frame::Irp(next) => write!(out, "IRP:{next}({line})"),
            Frame::Irpc(Some(c)) => write!(out, "IRPC:'{c}'({line})"),
            Frame::Irpc(None) => write!(out, "IRPC:'({line})"),
        };
    }

    /// Whether a space separates this frame from the next one in.
    fn spaced(&self) -> bool {
        matches!(self, Frame::Macro(_))
    }
}

impl SourceMap {
    /// Create an empty source map.
    pub fn new() -> Self {
        SourceMap { files: Vec::new(), expansions: Vec::new() }
    }

    /// Record one run of a body and return the [`SourceId`] its lines execute
    /// under.
    ///
    /// `body` is the id the body's lines carried before this run: a file, or
    /// an enclosing expansion when the body was itself written inside one.
    /// `body_start` is the byte offset of the body's first line in that text.
    /// `call` is the span that entered the run, in the enclosing source.
    ///
    /// Past `2^31 - 1` expansions this returns `body` unchanged, so the run's
    /// diagnostics name the body's own line as they did before expansions
    /// existed, rather than an id that would collide with the no-source
    /// sentinel `SourceId(u32::MAX)`.
    pub fn add_expansion(&mut self, body: SourceId, body_start: u32, call: Span, frame: Frame) -> SourceId {
        let index = self.expansions.len();
        if index >= EXPANSION_LIMIT as usize {
            return body;
        }
        let backing = self.backing(body);
        self.expansions.push(Expansion { backing, body_start, call, frame });
        SourceId(EXPANSION_BIT | index as u32)
    }

    fn expansion(&self, id: SourceId) -> Option<&Expansion> {
        if id.0 & EXPANSION_BIT == 0 {
            return None;
        }
        self.expansions.get((id.0 & !EXPANSION_BIT) as usize)
    }

    /// The file a source's text lives in: the id itself for a file, the file
    /// the body was written in for an expansion.
    pub fn backing(&self, id: SourceId) -> SourceId {
        self.expansion(id).map_or(id, |e| e.backing)
    }

    /// The same bytes of the same file, with the expansion forgotten.
    ///
    /// Two runs of one macro body raise diagnostics at spans that differ only
    /// in their expansion id. Code that asks "have I already reported THIS
    /// line of source" wants them equal, and this is the key it compares.
    pub fn physical(&self, span: Span) -> Span {
        Span { source: self.backing(span.source), ..span }
    }

    /// Add an unnamed source text and return its [`SourceId`].
    pub fn add(&mut self, text: String) -> SourceId {
        self.add_named(String::new(), text)
    }

    /// Add a source text under the name of the file it came from, and return its
    /// [`SourceId`].
    pub fn add_named(&mut self, name: String, text: String) -> SourceId {
        let id = SourceId(self.files.len() as u32);
        self.files.push(File { text, name, line_starts: OnceLock::new() });
        id
    }

    /// Return the full source text for the given [`SourceId`]: for an
    /// expansion, the text of the file its body was written in.
    pub fn text(&self, id: SourceId) -> &str {
        &self.files[self.backing(id).0 as usize].text
    }

    /// The name of a source, or `""` when it has none or the id is not in this
    /// map. An expansion answers with the name of the file its body was
    /// written in.
    pub fn name(&self, id: SourceId) -> &str {
        self.files.get(self.backing(id).0 as usize).map_or("", |f| f.name.as_str())
    }

    /// Number of FILES held. Expansions are not counted: they own no text, and
    /// a caller that walks `0..len()` is walking files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// True when no file has been added.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// `file(line):col` for a span in a NAMED source, which is asl's own shape
    /// (`smps-bug.asm(9):17: error: …`). `None` when the span's source is not
    /// in this map or carries no name, which is what a diagnostic belonging to
    /// no source line (a whole-run or placement failure) must produce.
    ///
    /// **A span inside an expansion renders asl's call-site trail**:
    /// `file(line) frame frame…:col`, where `file(line)` is the OUTERMOST call
    /// and each [`Frame`] is one expansion from the outside in (see `Frame` for
    /// the measured spellings). The file named is the one holding the call,
    /// not the one holding the macro body, because that is what asl names
    /// (`p3_incbody.asm(3) mymac(1):9` for a body written in `p3_mac.inc`). The
    /// column is the span's own column in the line it was written on, as for
    /// any other span.
    ///
    /// The column is here because [`location`](Self::location) already computes
    /// it and this function used to throw it away, so the AS surface reported
    /// less than the same binary's `.emp` surface from the same data. Two
    /// diagnostics about two operands of one line are one line number and two
    /// columns: without the column they render byte-identically and a reader
    /// cannot tell which operand either is about.
    ///
    /// **The spelling is asl's, measured rather than chosen.** On the reference
    /// build (md5 `61e672562465725a8c102288a7da9098`) asl reports
    /// `h.asm(2):9: error #1010: symbol undefined` for `Val equ Missing`, and
    /// across the five assignment spellings its numbers are 9, 7, 9, 8, 10,
    /// which are exactly the 1-based columns at which `Missing` starts in each.
    /// The evidence sits in `sigil-frontend-as/src/eval.rs` near the
    /// unresolved-assignment group. So sigil's old `file(line)` was asl's
    /// format with the column deleted, and a `file(line,col)` would have been a
    /// third dialect answering a question asl had already answered.
    ///
    /// `.emp` renders `path:line:col:` and keeps its own shape; the split is
    /// ruled in `docs/OVERSEER.md` on the argument that a compatibility surface
    /// should be the thing it is compatible with, which is the same argument
    /// that fixes this spelling.
    pub fn label(&self, span: Span) -> Option<String> {
        // Innermost frame first, each with its body-relative line.
        let mut trail: Vec<(&Frame, u32)> = Vec::new();
        let mut at = span;
        while let Some(e) = self.expansion(at.source) {
            let (line, _) = self.location(at);
            let (first, _) = self.location(Span { source: e.backing, start: e.body_start, end: e.body_start });
            trail.push((&e.frame, line.saturating_sub(first) + 1));
            // A call always belongs to a file or to an EARLIER expansion, so
            // the walk ends; the bound holds even for a span from another map.
            if trail.len() > self.expansions.len() {
                return None;
            }
            at = e.call;
        }
        let name = &self.files.get(at.source.0 as usize)?.name;
        if name.is_empty() {
            return None;
        }
        let (line, _) = self.location(at);
        let (_, col) = self.location(span);
        let mut out = format!("{name}({line})");
        let mut spaced = true;
        for (frame, line) in trail.iter().rev() {
            if spaced {
                out.push(' ');
            }
            frame.render(*line, &mut out);
            spaced = frame.spaced();
        }
        out.push_str(&format!(":{col}"));
        Some(out)
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
    ///
    /// A span inside an expansion locates in the file its body was written in
    /// ([`SourceMap::physical`]).
    pub fn location(&self, span: Span) -> (u32, u32) {
        let span = self.physical(span);
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
        let file = &self.files[id.0 as usize];
        file.line_starts.get_or_init(|| {
            let text = &file.text;
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
        // line 5 of nothing else. The trailing `:1` is the COLUMN, in asl's own
        // `file(line):col` spelling; byte 8 is the first byte of root's line 3.
        assert_eq!(
            map.label(Span { source: root, start: 8, end: 9 }).as_deref(),
            Some("root.asm(3):1")
        );
        // Byte 6 of the included file is line 4 — the includer's name never appears.
        assert_eq!(
            map.label(Span { source: inc, start: 6, end: 7 }).as_deref(),
            Some("sub/part.asm(4):1")
        );
        // A column that is NOT 1, so the field is shown to carry the position
        // rather than a constant: byte 9 is root's line 3, second character.
        assert_eq!(
            map.label(Span { source: root, start: 9, end: 10 }).as_deref(),
            Some("root.asm(3):2")
        );
        // A source with no name, and an id in no map at all, both decline to invent
        // a location rather than defaulting to the first file.
        assert_eq!(map.label(Span { source: anon, start: 0, end: 1 }), None);
        assert_eq!(map.label(Span { source: SourceId(u32::MAX), start: 0, end: 0 }), None);
        assert_eq!(map.name(root), "root.asm");
        assert_eq!(map.len(), 3);
    }

    /// The byte offset of 1-based line `n` of `text`.
    fn line_start(text: &str, n: usize) -> u32 {
        text.split_inclusive('\n').take(n - 1).map(str::len).sum::<usize>() as u32
    }

    /// A span at column `col` (1-based) of line `n`, in `source`.
    fn at(map: &SourceMap, source: SourceId, n: usize, col: u32) -> Span {
        let s = line_start(map.text(source), n) + col - 1;
        Span { source, start: s, end: s + 1 }
    }

    /// One macro call: the trail names the CALL's file and line, then the
    /// macro and the line of its body, and never the file the body is in.
    /// asl on the same two files (`p3_incbody.asm` including `p3_mac.inc`):
    /// `p3_incbody.asm(3) mymac(1):9: error #1200: unknown instruction`.
    /// The column is sigil's own (characters, a tab is one), not asl's 9,
    /// which counts the tab as eight inside an expansion.
    #[test]
    fn a_span_in_a_macro_run_labels_as_the_call_then_the_macro_line() {
        let mut map = SourceMap::new();
        let root = map.add_named("p3_incbody.asm".into(), "\tcpu 68000\n\tinclude \"p3_mac.inc\"\n\tmymac\n".into());
        let inc = map.add_named("p3_mac.inc".into(), "; line 1\nmymac macro\n\tbogus_in_inc_body\n\tendm\n".into());
        let run = map.add_expansion(inc, line_start(map.text(inc), 3), at(&map, root, 3, 2), Frame::Macro("mymac".into()));
        assert_eq!(map.label(at(&map, run, 3, 2)).as_deref(), Some("p3_incbody.asm(3) mymac(1):2"));
        // Every other accessor reads the run as the body's own file.
        assert_eq!(map.physical(at(&map, run, 3, 2)), at(&map, inc, 3, 2));
        assert_eq!(map.location(at(&map, run, 3, 2)), (3, 2));
        assert_eq!(map.name(run), "p3_mac.inc");
        assert_eq!(map.text(run), map.text(inc));
        assert_eq!(map.backing(run), inc);
        // The body's own spans are untouched by the run, and files are counted
        // as before.
        assert_eq!(map.label(at(&map, inc, 3, 2)).as_deref(), Some("p3_mac.inc(3):2"));
        assert_eq!(map.len(), 2);
    }

    /// Two runs of one body are two ids with one physical span, which is what
    /// "already reported at this line" keys on.
    #[test]
    fn two_runs_of_one_body_differ_by_id_and_agree_physically() {
        let text = "\tcpu 68000\nmymac macro\n\tnop\n\tbogus_in_body\n\tendm\n\tnop\n\tmymac\n\tmymac\n";
        let mut map = SourceMap::new();
        let f = map.add_named("p1_simple.asm".into(), text.into());
        let body = line_start(text, 3);
        let one = map.add_expansion(f, body, at(&map, f, 7, 2), Frame::Macro("mymac".into()));
        let two = map.add_expansion(f, body, at(&map, f, 8, 2), Frame::Macro("mymac".into()));
        assert_ne!(one, two);
        assert_eq!(map.physical(at(&map, one, 4, 2)), map.physical(at(&map, two, 4, 2)));
        // asl: `p1_simple.asm(7) mymac(2):9` and `p1_simple.asm(8) mymac(2):9`.
        assert_eq!(map.label(at(&map, one, 4, 2)).as_deref(), Some("p1_simple.asm(7) mymac(2):2"));
        assert_eq!(map.label(at(&map, two, 4, 2)).as_deref(), Some("p1_simple.asm(8) mymac(2):2"));
    }

    /// Nesting through a loop: the macro frame is followed by a space, the loop
    /// frame is not, and the loop is entered from its closing line. asl on this
    /// text (`q7_nest_rept_space.asm`):
    /// `q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):9: error #1200`.
    #[test]
    fn a_nested_trail_through_a_loop_spells_every_frame_as_asl_does() {
        let text = "inner macro\n\tbogus_inner\n\tendm\nouter macro\n\trept 1\n\tnop\n\tinner\n\tendm\n\tendm\n\tcpu 68000\n\touter\n";
        let mut map = SourceMap::new();
        let f = map.add_named("q7_nest_rept_space.asm".into(), text.into());
        let outer = map.add_expansion(f, line_start(text, 5), at(&map, f, 11, 2), Frame::Macro("outer".into()));
        let rept = map.add_expansion(outer, line_start(text, 6), at(&map, outer, 8, 2), Frame::Rept(1));
        let inner = map.add_expansion(f, line_start(text, 2), at(&map, rept, 7, 2), Frame::Macro("inner".into()));
        // Each run's backing is the FILE, whatever id its body was reached under.
        assert_eq!(map.backing(rept), f);
        assert_eq!(
            map.label(at(&map, inner, 2, 2)).as_deref(),
            Some("q7_nest_rept_space.asm(11) outer(4) REPT 1(2)inner(1):2")
        );
        // Two macro frames in a row are separated by a space. asl
        // (`q6_three.asm`): `q6_three.asm(14) outer(1) mid(3) inner(2):9`.
        let text6 = "inner macro\n\tnop\n\tbogus_inner\n\tendm\nmid macro\n\tnop\n\tnop\n\tinner\n\tendm\nouter macro\n\tmid\n\tendm\n\tcpu 68000\n\touter\n";
        let g = map.add_named("q6_three.asm".into(), text6.into());
        let o = map.add_expansion(g, line_start(text6, 11), at(&map, g, 14, 2), Frame::Macro("outer".into()));
        let m = map.add_expansion(g, line_start(text6, 6), at(&map, o, 11, 2), Frame::Macro("mid".into()));
        let i = map.add_expansion(g, line_start(text6, 2), at(&map, m, 8, 2), Frame::Macro("inner".into()));
        assert_eq!(map.label(at(&map, i, 3, 2)).as_deref(), Some("q6_three.asm(14) outer(1) mid(3) inner(2):2"));
    }

    /// The four loop spellings, each against the asl line it was read from.
    #[test]
    fn loop_frames_spell_rept_while_irp_and_irpc_as_asl_does() {
        let mut map = SourceMap::new();
        // `p8_toprept.asm(5) REPT 2(1):2`, the closing line, second iteration.
        let t = "\tcpu 68000\n\tnop\n\trept 2\n\tbogus_top_rept\n\tendm\n";
        let f = map.add_named("p8_toprept.asm".into(), t.into());
        let r = map.add_expansion(f, line_start(t, 4), at(&map, f, 5, 2), Frame::Rept(2));
        assert_eq!(map.label(at(&map, r, 4, 2)).as_deref(), Some("p8_toprept.asm(5) REPT 2(1):2"));
        // `q3_topwhile.asm(7) WHILE 1/2:2`: a slash, not parentheses.
        let t = "\tcpu 68000\ncnt set 0\n\twhile cnt<2\n\tnop\n\tbogus_while2\ncnt set cnt+1\n\tendm\n";
        let f = map.add_named("q3_topwhile.asm".into(), t.into());
        let w = map.add_expansion(f, line_start(t, 4), at(&map, f, 7, 2), Frame::While(1));
        assert_eq!(map.label(at(&map, w, 5, 2)).as_deref(), Some("q3_topwhile.asm(7) WHILE 1/2:2"));
        // `q1_irp3.asm(5) IRP:bb(2):9` then `IRP:(2)` on the last item.
        let t = "\tcpu 68000\n\tirp x,aa,bb,cc\n\tnop\n\tbogus_irp3 x\n\tendm\n";
        let f = map.add_named("q1_irp3.asm".into(), t.into());
        let first = map.add_expansion(f, line_start(t, 3), at(&map, f, 5, 2), Frame::Irp("bb".into()));
        let last = map.add_expansion(f, line_start(t, 3), at(&map, f, 5, 2), Frame::Irp("".into()));
        assert_eq!(map.label(at(&map, first, 4, 2)).as_deref(), Some("q1_irp3.asm(5) IRP:bb(2):2"));
        assert_eq!(map.label(at(&map, last, 4, 2)).as_deref(), Some("q1_irp3.asm(5) IRP:(2):2"));
        // `r6_irpc_abc.asm(4) IRPC:'b'(1):9` and, on the last, `IRPC:'(1):9`.
        let t = "\tcpu 68000\n\tirpc x,\"abc\"\n\tbogus_irpc x\n\tendm\n";
        let f = map.add_named("r6_irpc_abc.asm".into(), t.into());
        let mid = map.add_expansion(f, line_start(t, 3), at(&map, f, 4, 2), Frame::Irpc(Some('b')));
        let end = map.add_expansion(f, line_start(t, 3), at(&map, f, 4, 2), Frame::Irpc(None));
        assert_eq!(map.label(at(&map, mid, 3, 2)).as_deref(), Some("r6_irpc_abc.asm(4) IRPC:'b'(1):2"));
        assert_eq!(map.label(at(&map, end, 3, 2)).as_deref(), Some("r6_irpc_abc.asm(4) IRPC:'(1):2"));
    }

    /// An id this map never handed out, and the no-source sentinel, still
    /// decline to name a location, expansion bit or not.
    #[test]
    fn an_unknown_expansion_id_labels_as_nothing() {
        let mut map = SourceMap::new();
        let f = map.add_named("f.asm".into(), "nop\n".into());
        map.add_expansion(f, 0, Span { source: f, start: 0, end: 0 }, Frame::Rept(1));
        assert_eq!(map.label(Span { source: SourceId(EXPANSION_BIT | 7), start: 0, end: 0 }), None);
        assert_eq!(map.label(Span { source: SourceId(u32::MAX), start: 0, end: 0 }), None);
        assert_eq!(map.physical(Span { source: SourceId(u32::MAX), start: 0, end: 0 }).source, SourceId(u32::MAX));
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
