//! The one renderer every error diagnostic the native build reports goes through.
//!
//! A build failure is a list of [`Diagnostic`]s; this module turns that list into
//! the lines a reader sees, one line per diagnostic, in the shape
//!
//! ```text
//!   <path>:<line>:<col>: [Error] <message>
//! ```
//!
//! The `[Error]` token is the level, printed the same way for every family: the
//! comptime `ensure`s `build_program` evaluates, the deferred `extern()` guards
//! `check_link_asserts` evaluates after `resolve_layout`, and the sound emitters'
//! co-link guards. A lane that counts failing guards counts `[Error]` tokens, so
//! a family that rendered through any other formatter would be invisible to it.
//! The location prefix is the `.emp` house dialect (`path:line:col:`); a
//! diagnostic whose span no known source explains keeps its byte span instead of
//! being attributed to a line it is not about.

use std::path::Path;

use sigil_span::{Diagnostic, Level, SourceId, SourceMap, Span};

/// The source texts a set of single-file lowerings read, keyed by the
/// [`SourceId`] each file was parsed under, so a diagnostic raised against any
/// of them locates as `path:line:col`.
///
/// The seam-2 emitters parse each `.emp` file on its own; two files parsed under
/// the same id would make one file's spans point into the other's text, so every
/// file registers here FIRST and is parsed under the id it is handed back.
#[derive(Default)]
pub struct SourceTexts {
    map: SourceMap,
}

impl SourceTexts {
    /// An empty register.
    pub fn new() -> SourceTexts {
        SourceTexts { map: SourceMap::new() }
    }

    /// Register `text` as the content of `path`; parse it under the returned id.
    pub fn add(&mut self, path: &Path, text: &str) -> SourceId {
        self.map.add_named(path.display().to_string(), text.to_string())
    }

    /// `path:line:col` of `span`'s start, or `None` when `span` belongs to no
    /// registered file.
    ///
    /// "Belongs" is [`SourceMap::contains`], not a range check against the
    /// file count: an expansion id is past every file index, so a range check
    /// reads a valid expansion span as unlocated. A span inside an expansion
    /// locates in the file its body was written in ([`SourceMap::physical`]),
    /// because this renderer speaks the `.emp` dialect, `path:line:col`, which
    /// has no call-site trail; the AS surface renders that trail through
    /// [`SourceMap::label`].
    pub fn locate(&self, span: Span) -> Option<String> {
        if !self.map.contains(span.source) {
            return None;
        }
        let name = self.map.name(span.source);
        if name.is_empty() {
            return None;
        }
        let (line, col) = self.map.location(span);
        Some(format!("{name}:{line}:{col}"))
    }
}

/// One line per diagnostic, every level, located through `locate` where it can
/// be and carrying the raw byte span where it cannot.
pub fn render_diag_lines(diags: &[&Diagnostic], locate: &dyn Fn(Span) -> Option<String>) -> String {
    diags
        .iter()
        .map(|d| match locate(d.primary) {
            Some(loc) => format!("  {loc}: [{:?}] {}", d.level, d.message),
            None => format!("  [{:?}] {} @ {:?}", d.level, d.message, d.primary),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The verdict over a guard family's diagnostics: `Ok(())` when none is an
/// error, else `Err` carrying `what`, the error count, and EVERY error rendered
/// through [`render_diag_lines`], so a reader sees each failing guard at its own
/// line and a lane that counts them counts them all.
pub fn link_assert_failure(
    what: &str,
    diags: &[Diagnostic],
    locate: &dyn Fn(Span) -> Option<String>,
) -> Result<(), String> {
    let errors: Vec<&Diagnostic> = diags.iter().filter(|d| d.level == Level::Error).collect();
    if errors.is_empty() {
        return Ok(());
    }
    Err(format!("{what}: {} error(s):\n{}", errors.len(), render_diag_lines(&errors, locate)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(level: Level, message: &str, source: u32, start: u32) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            primary: Span { source: SourceId(source), start, end: start + 1 },
        }
    }

    #[test]
    fn located_line_carries_path_line_col_and_the_level_token() {
        let mut texts = SourceTexts::new();
        let id = texts.add(Path::new("probe/a.emp"), "module a\nconst X = 1\nensure(X == 2, \"no\")\n");
        assert_eq!(id, SourceId(0));
        let d = diag(Level::Error, "no", 0, 21);
        let line = render_diag_lines(&[&d], &|s| texts.locate(s));
        assert_eq!(line, "  probe/a.emp:3:1: [Error] no");
    }

    /// A span inside an expansion of the renderer's own map is located, in the
    /// file its body was written in: `path:line:col`, this renderer's dialect,
    /// with no call-site trail. Before `contains`, the file-count range check
    /// read the expansion id (bit 31 set, past every file index) as "no file"
    /// and the line fell back to its raw byte span. An expansion id the map
    /// never handed out is still no location.
    #[test]
    fn a_span_inside_an_expansion_of_the_map_is_located_in_its_body_file() {
        let mut texts = SourceTexts::new();
        // Line 2 (`\tnop`) is the body's first line, at byte 8; `bad` is byte
        // 14, line 3 column 2; the call `m` is byte 25, line 5.
        let id = texts.add(Path::new("probe/m.asm"), "m macro\n\tnop\n\tbad\n\tendm\n\tm\n");
        let call = Span { source: id, start: 25, end: 26 };
        let run = texts.map.add_expansion(id, 8, call, sigil_span::Frame::Macro("m".into()));
        let d = Diagnostic {
            level: Level::Error,
            message: "bad".to_string(),
            primary: Span { source: run, start: 14, end: 15 },
        };
        let line = render_diag_lines(&[&d], &|s| texts.locate(s));
        assert_eq!(line, "  probe/m.asm:3:2: [Error] bad");
        assert_eq!(texts.locate(Span { source: SourceId(run.0 + 1), start: 0, end: 0 }), None);
    }

    #[test]
    fn unlocatable_span_keeps_its_bytes_and_the_level_token() {
        let texts = SourceTexts::new();
        let d = diag(Level::Warning, "w", 7, 3);
        let line = render_diag_lines(&[&d], &|s| texts.locate(s));
        assert!(line.starts_with("  [Warning] w @ Span"), "{line}");
    }

    #[test]
    fn verdict_is_ok_without_errors_and_lists_every_error_otherwise() {
        let texts = SourceTexts::new();
        let locate = |s: Span| texts.locate(s);
        let warn = diag(Level::Warning, "soft", 0, 0);
        assert_eq!(link_assert_failure("g", std::slice::from_ref(&warn), &locate), Ok(()));
        let e1 = diag(Level::Error, "first", 0, 0);
        let e2 = diag(Level::Error, "second", 0, 5);
        let err = link_assert_failure("g fired", &[warn, e1, e2], &locate).unwrap_err();
        assert!(err.starts_with("g fired: 2 error(s):\n"), "{err}");
        assert_eq!(err.matches("[Error]").count(), 2, "{err}");
        assert!(err.contains("first") && err.contains("second"), "{err}");
        assert!(!err.contains("Diagnostic {"), "{err}");
    }
}
