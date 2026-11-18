//! The span-to-source seam. The exporter is driver-independent: it never knows how
//! a front end maps a `Span` to a file and line, only that something does. The one
//! implementation is `sigil_harness::native::ChainSources`, which reads both front
//! ends' maps over one file-id space. This is the only locator trait.

use sigil_span::Span;

/// Where a span lands: a file id (see [`SpanLocator::file`]) and the 1-based line of
/// the OUTERMOST source line, i.e. the line a person wrote after following any
/// macro or loop expansion back to its call site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourcePos {
    pub file: u32,
    pub line: u32,
}

/// A file as its front end names it. The locator knows files by the name the front
/// end opened them under, not by their row in `BuildExport.files` (which is
/// root-relative and only exists once the read set is final), so a consumer that
/// wants the row maps `path` to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRef {
    pub path: String,
}

/// A span authority.
pub trait SpanLocator {
    /// The outermost source position of `span`, or `None` when it names no known
    /// file (the no-source id, an unreadable file): a no-source row, never a guess.
    fn outermost(&self, span: Span) -> Option<SourcePos>;
    /// The file `id` names, for any `id` this locator's `outermost` returned.
    /// Panics on an id it never issued: that is a caller bug, not data.
    fn file(&self, id: u32) -> &FileRef;
}
