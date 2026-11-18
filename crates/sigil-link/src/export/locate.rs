//! The span-to-source seam. The exporter is driver-independent: it never knows how
//! a front end maps a `Span` to a file and line, only that something does. Later
//! tasks implement this over `ChainSources`; the spike needs no locator.

use sigil_span::Span;

/// Where a span lands: an index into `BuildExport::files` and a 1-based line, the
/// outermost line of any expansion chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Located {
    pub file: u32,
    pub line: u32,
}

/// A span authority. `None` means the span names no known file (a no-source row).
pub trait SpanLocator {
    fn locate(&self, span: Span) -> Option<Located>;
}
