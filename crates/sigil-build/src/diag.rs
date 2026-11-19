//! The build's diagnostics: the warn tier ([`BuildWarning`], [`collect_warnings`]),
//! the chained build's location authority ([`ChainSources`]), and the one error
//! renderer ([`render`]).
//!
//! Moved from `sigil-harness` (`native.rs`, `diag_render.rs`) unchanged by G0b Task 4.
//! Every other generic module carries these types, so they move first; sigil-harness
//! re-exports them at their old paths.

use sigil_frontend_emp::resolve;
use sigil_ir::{Fragment, Section};
use sigil_link::export::locate::{SourcePos, SpanLocator};

/// The one renderer every error diagnostic the native build reports goes through.
pub mod render;

/// A non-error diagnostic the `.emp` build produced, with its source location
/// already resolved.
///
/// The [`Manifest`](sigil_frontend_emp::resolve::manifest::Manifest) that owns the
/// source texts lives and dies inside [`build_emp`], so the `path:line:col` is
/// rendered where it is knowable and travels with the diagnostic. `Warning` and
/// `Note` both ride this channel — errors abort the build instead.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildWarning {
    /// Severity: [`Level::Warning`](sigil_span::Level::Warning) or
    /// [`Level::Note`](sigil_span::Level::Note).
    pub level: sigil_span::Level,
    /// The bracketed lint id (`proc.sr-undeclared`), or the empty string for a
    /// diagnostic whose message carries no `[id]` prefix.
    pub id: String,
    /// `path:line:col` of the primary span, or `None` for a span with no source file.
    pub location: Option<String>,
    /// The full message, `[id]` prefix included.
    pub message: String,
    /// The primary span, kept so a consumer that needs more than the rendered
    /// location (a caret, an editor jump, a `-Werror` promotion) has it.
    pub primary: sigil_span::Span,
}

impl BuildWarning {
    /// Pair `d` with the location `locate` gives its primary span. The lint id is
    /// the leading `[...]` group, which is the corpus convention for every
    /// classified diagnostic.
    ///
    /// The LOCATION AUTHORITY IS THE ARGUMENT, because there is more than one and
    /// they are not interchangeable (see [`from_as`](Self::from_as)). Every
    /// constructor below is this one with a particular authority named, so a new
    /// caller has to say which map reads its spans instead of inheriting whichever
    /// one happened to be in scope.
    fn located(d: &sigil_span::Diagnostic, locate: &dyn Fn(sigil_span::Span) -> Option<String>) -> BuildWarning {
        let id = d
            .message
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .map(|(id, _)| id.to_string())
            .unwrap_or_default();
        BuildWarning {
            level: d.level,
            id,
            location: locate(d.primary),
            message: d.message.clone(),
            primary: d.primary,
        }
    }

    /// The same pairing for a diagnostic raised by the AS front end, located
    /// through ITS [`SourceMap`](sigil_span::SourceMap).
    ///
    /// A separate constructor rather than a second [`collect_warnings`] call,
    /// because the two location authorities are NOT interchangeable: a
    /// [`SourceIndex`](sigil_frontend_emp::resolve::manifest::SourceIndex) keys
    /// paths by `SourceId` from the `.emp` manifest, and an AS span's id counts
    /// the AS root and its `include` splices. Feeding one map's span to the
    /// other's index does not fail — it names a DIFFERENT FILE, confidently.
    pub fn from_as(d: &sigil_span::Diagnostic, sources: &sigil_span::SourceMap) -> BuildWarning {
        BuildWarning::located(d, &|span| sources.label(span))
    }
}

/// THE CHAINED BUILD'S LOCATION AUTHORITY: both front ends' maps, plus the one id
/// space that lets a bare [`Span`](sigil_span::Span) say which of them can read it.
///
/// # The defect this type exists to close
///
/// A chained build lowers ONE section list out of TWO front ends
/// ([`resolve_chained`]: the AS module's sections, then the `.emp` ones), and every
/// consumer downstream — the packing walk's width-flip report, the `resolve_layout`
/// failure, the link failure, the warn tier — receives a `Span` and nothing else. A
/// `Span` is an id and a byte range. AS file ids count the AS root and its `include`
/// splices; `.emp` ids count the files the manifest scanned. **Both count from 0**,
/// so the ranges overlap and reading an AS span through the `.emp` index does not
/// fail — it answers with a real path, a real line, and the WRONG FILE, which is
/// the failure mode [`BuildWarning::from_as`] already names for the warn tier.
///
/// # How the id space is made one
///
/// At the seam (and ONLY at the seam) every span carried by an AS-side section is
/// REBASED by [`rebase_section`](Self::rebase_section): a file id `k` becomes
/// `k + as_base`, where `as_base` is the number of ids the `.emp` index covers.
/// After that, an id below `as_base` is an `.emp` file and an id at or above it is
/// an AS file, for every span in the list, and [`locate`](Self::locate) needs no
/// help from its caller to tell them apart. Adding a constant to every AS id is
/// injective, so spans that were distinct stay distinct (the warn tier's
/// `(level, message, span)` dedup key keeps its meaning) and spans that used to
/// COLLIDE across the two front ends stop colliding.
///
/// EXPANSION ids are joined the same way, in their own high range
/// ([`SourceId::is_expansion`](sigil_span::SourceId::is_expansion)). BOTH front ends make
/// them: the AS front end for every macro, `rept` and `irp` run, and the `.emp` front end
/// for every template splice (G0a spec 4.4, `Frame::Template`), and both count their
/// expansion indices from 0, so the two ranges overlap exactly as the file ranges do. The
/// `.emp` index keeps expansion indices `0 .. emp_expansions`; every AS expansion index
/// `k` becomes `k + emp_expansions`, by the same rebase and at the same seam as its files.
/// The no-source id (`SourceId(u32::MAX)`) decodes to an index neither map issues, is
/// never moved, and answers `None` under either authority: an unattributed diagnostic
/// renders as the bare message.
///
/// # What this does NOT do
///
/// It does not make the two front ends' diagnostics render alike. An `.emp` span
/// renders `path:line:col` and an AS span renders asl's own `file(line):col` with
/// its call-site trail — the split ruled in `docs/OVERSEER.md`, and the same split
/// [`BuildWarning::from_as`] has always produced. This type decides WHICH authority
/// reads a span, never how that authority spells its answer.
pub struct ChainSources {
    /// The `.emp` manifest's index: ids `0 .. as_base`.
    emp: sigil_frontend_emp::resolve::manifest::SourceIndex,
    /// The AS front end's own map, at ITS own numbering. A rebased id is brought
    /// back down before this map is asked anything.
    as_map: sigil_span::SourceMap,
    /// Where the `.emp` ids stop and the rebased AS ids start.
    as_base: u32,
    /// Where the `.emp` EXPANSION indices stop and the rebased AS ones start.
    emp_expansions: u32,
    /// How many expansions the AS map issued: the AS indices that are moved.
    as_expansions: u32,
    /// Every file of the joined id space, by id (`.emp` ids, then rebased AS ids):
    /// what [`SpanLocator::file`] hands out.
    files: Vec<sigil_link::export::locate::FileRef>,
}

/// Whether `emp_len` `.emp` ids and `as_len` AS ids fit in one file-id space below
/// the expansion range — the refusal [`ChainSources::join`] makes, taken as a
/// function of the two SIZES alone so the boundary can be exercised at a value no
/// `SourceIndex` could be built at.
pub fn check_joined_space(emp_len: usize, as_len: usize) -> Result<(), String> {
    let top = emp_len.saturating_add(as_len);
    if top > u32::MAX as usize || sigil_span::SourceId(top as u32).is_expansion() {
        return Err(format!(
            "[chain.source-id-space] the chained build's joined source-id space \
             ({emp_len} `.emp` ids + {as_len} AS ids) reaches the expansion range, so \
             a rebased AS file id could not be told from a macro expansion and every \
             diagnostic past the boundary would name the wrong source"
        ));
    }
    Ok(())
}

/// Whether `emp_len` `.emp` expansions and `as_len` AS expansions fit in one
/// expansion range below the no-source id: the expansion half of the refusal
/// [`ChainSources::join`] makes ([`check_joined_space`] is the file half).
pub fn check_joined_expansions(emp_len: usize, as_len: usize) -> Result<(), String> {
    let top = emp_len.saturating_add(as_len);
    if top > u32::MAX as usize || (top > 0 && sigil_span::SourceId::from_expansion_index(top as u32 - 1).is_none()) {
        return Err(format!(
            "[chain.source-id-space] the chained build's joined expansion range \
             ({emp_len} `.emp` template expansions + {as_len} AS expansions) reaches the \
             no-source id, so a rebased AS expansion could not be told from a line with no \
             source and every location past the boundary would be lost"
        ));
    }
    Ok(())
}

/// Which authority reads a span of the joined id space, and the span as that authority
/// numbers it.
enum Side {
    /// The `.emp` index, at its own (unmoved) numbering.
    Emp(sigil_span::Span),
    /// The AS map, brought back down to its own numbering.
    As(sigil_span::Span),
    /// Neither: the no-source id, or an expansion index neither front end issued.
    Neither,
}

impl ChainSources {
    /// THE SEAM: join the two authorities and produce the chained section list —
    /// the AS side moved into the joined id space, then the `.emp` side, which
    /// already numbers in it. `emp` keeps ids `0 .. emp.len()`; the AS map's files
    /// move to `emp.len() ..`, and so does every span the AS sections carry.
    ///
    /// THE THREE ACTS ARE ONE CONSTRUCTOR ON PURPOSE. A `ChainSources` that exists
    /// beside un-rebased AS sections is worse than no `ChainSources` at all: it
    /// reads every AS span as an `.emp` id and answers with the same confident
    /// wrong file the type was built to stop. Because the list a caller goes on to
    /// resolve comes OUT of here, there is no second list for it to use instead,
    /// and the seam cannot be half-applied by someone who did not know there was a
    /// second half. The concatenation order lives here too, beside the id rule it
    /// has to agree with, rather than at the call site where the two could drift.
    ///
    /// LOUD ON UNMEASURABLE: if the joined space would reach the expansion range,
    /// a rebased file id would be indistinguishable from an expansion and every
    /// location past the boundary would be wrong in a new way. That is refused by
    /// name rather than truncated, wrapped, or silently located
    /// ([`check_joined_space`]). It takes 2^31 source files to provoke and has
    /// never been seen; the refusal is here because the alternative is a wrong
    /// file.
    pub fn join(
        emp: sigil_frontend_emp::resolve::manifest::SourceIndex,
        as_map: sigil_span::SourceMap,
        mut as_sections: Vec<Section>,
        emp_sections: Vec<Section>,
    ) -> Result<(ChainSources, Vec<Section>), String> {
        let as_base = emp.len();
        check_joined_space(as_base, as_map.len())?;
        let (emp_expansions, as_expansions) = (emp.expansion_count(), as_map.expansion_count());
        check_joined_expansions(emp_expansions, as_expansions)?;
        let files = (0..as_base)
            .map(|k| emp.path(k as u32).map(|p| p.display().to_string()).unwrap_or_default())
            .chain((0..as_map.len()).map(|k| as_map.name(sigil_span::SourceId(k as u32)).to_string()))
            .map(|path| sigil_link::export::locate::FileRef { path })
            .collect();
        let chain = ChainSources {
            emp,
            as_map,
            as_base: as_base as u32,
            emp_expansions: emp_expansions as u32,
            as_expansions: as_expansions as u32,
            files,
        };
        for sec in &mut as_sections {
            chain.rebase_section(sec);
        }
        as_sections.extend(emp_sections);
        Ok((chain, as_sections))
    }

    /// `span` with its AS id moved into the joined space: a file id past the `.emp`
    /// files, an expansion id past the `.emp` expansions. The no-source id (and any
    /// expansion index the AS map never issued) is returned unchanged — see the type
    /// docs.
    pub fn rebase(&self, span: sigil_span::Span) -> sigil_span::Span {
        let source = match span.source.expansion_index() {
            Some(k) if k < self.as_expansions => sigil_span::SourceId::from_expansion_index(k + self.emp_expansions)
                .expect("check_joined_expansions keeps every rebased index below the no-source id"),
            Some(_) => return span,
            None => sigil_span::SourceId(span.source.0 + self.as_base),
        };
        sigil_span::Span { source, ..span }
    }

    /// Which authority reads `span`, a span of the joined id space.
    fn side(&self, span: sigil_span::Span) -> Side {
        let down = |source| Side::As(sigil_span::Span { source, ..span });
        match span.source.expansion_index() {
            Some(k) if k < self.emp_expansions => Side::Emp(span),
            Some(k) if k - self.emp_expansions < self.as_expansions => {
                down(sigil_span::SourceId::from_expansion_index(k - self.emp_expansions).expect("an issued AS index"))
            }
            Some(_) => Side::Neither,
            None if span.source.0 < self.as_base => Side::Emp(span),
            None => down(sigil_span::SourceId(span.source.0 - self.as_base)),
        }
    }

    /// Rebase every span an AS-SIDE section carries, in place.
    ///
    /// The `match` is EXHAUSTIVE ON PURPOSE — no `_` arm. A fragment variant added
    /// with a span of its own must be named here, and until it is the crate does not
    /// compile; a wildcard would let the next variant's spans travel unrebased and
    /// land back on the `.emp` index, which is this whole defect again with a new
    /// spelling. (`RelaxCandidate`'s `Fixup` carries no span of its own — the
    /// fragment's span covers the instruction — so the candidate lists hold none.)
    fn rebase_section(&self, sec: &mut Section) {
        for frag in &mut sec.fragments {
            match frag {
                Fragment::Data(d) => d.span = self.rebase(d.span),
                Fragment::Fill { span, .. }
                | Fragment::Reserve { span, .. }
                | Fragment::JmpJsrSym { span, .. }
                | Fragment::RelaxAbsSym { span, .. }
                | Fragment::RelaxLadder { span, .. }
                | Fragment::Org { span, .. } => *span = self.rebase(*span),
            }
        }
        for eq in &mut sec.equ_syms {
            eq.span = self.rebase(eq.span);
        }
    }

    /// Rebase an AS-side warning's primary span.
    ///
    /// Its rendered `location` is NOT touched: [`BuildWarning::from_as`] already
    /// resolved it against the AS map at its own numbering, and that string is
    /// correct. What moves is `primary`, which rides along for a consumer that
    /// wants more than the string (a caret, an editor jump, a `-Werror`
    /// promotion) — and such a consumer, in a chained build, will read it through
    /// [`locate`](Self::locate). Leaving it un-rebased would make the one field
    /// that is supposed to be re-readable the one field that lies.
    pub fn rebase_warning(&self, w: &mut BuildWarning) {
        w.primary = self.rebase(w.primary);
    }

    /// `span`'s location through the authority that can read it, or `None` when
    /// neither can.
    ///
    /// A `.emp` template expansion locates where its line was WRITTEN (the template
    /// body), exactly as a diagnostic there always has; the line that spliced it is
    /// the export's question ([`SpanLocator::outermost`]), not the diagnostics'.
    pub fn locate(&self, span: sigil_span::Span) -> Option<String> {
        match self.side(span) {
            Side::Emp(span) => self.emp.locate(span),
            Side::As(span) => self.as_map.label(span),
            Side::Neither => None,
        }
    }
}

/// The export's locator: the outermost line, in the joined id space. An expansion of
/// either front end (an AS macro or loop run, a `.emp` template splice) follows
/// `Expansion.call` out to the line that invoked it (the walk
/// [`SourceMap::label`](sigil_span::SourceMap::label) makes); a rebased AS id is
/// brought back down before the AS map is asked and the answer moved up again.
impl SpanLocator for ChainSources {
    fn outermost(&self, span: sigil_span::Span) -> Option<SourcePos> {
        match self.side(span) {
            Side::Emp(span) => {
                let (file, line) = self.emp.outermost(span)?;
                Some(SourcePos { file, line })
            }
            Side::As(span) => {
                let (id, line) = self.as_map.outermost(span)?;
                Some(SourcePos { file: id.0 + self.as_base, line })
            }
            Side::Neither => None,
        }
    }

    fn file(&self, id: u32) -> &sigil_link::export::locate::FileRef {
        self.files
            .get(id as usize)
            .unwrap_or_else(|| panic!("file id {id} was never issued by this locator ({} files)", self.files.len()))
    }
}

impl std::fmt::Display for BuildWarning {
    /// `path:line:col: <level>: <message>` — the shape
    /// `render_program_diags` gives errors, so both tiers read as one system.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{loc}: {}: {}", self.level, self.message),
            None => write!(f, "{}: {}", self.level, self.message),
        }
    }
}

/// Location-resolve and deduplicate every non-error diagnostic in `sources`,
/// dropping those attributed to `generated`.
///
/// `generated` names a module the TOOL wrote — [`build_emp`]'s synthetic entry,
/// whose bare `use` lines drive the reachability BFS. A build report is for code
/// its reader can edit; that module has no on-disk file, so a diagnostic against it
/// is unactionable by construction and would only teach the reader to distrust the
/// channel. The exclusion is by exact [`SourceId`](sigil_span::SourceId) — the
/// tool/user boundary, not an exemption over corpus content, and it cannot widen.
/// `None` excludes nothing, for a caller that generates no module.
///
/// Deduplication is by `(level, message, span)` — the key
/// [`build_program_with`](sigil_frontend_emp::resolve) already uses for its own
/// cross-module merge. It collapses an exact repeat of one diagnostic at one site,
/// so the reported count is a count of DISTINCT problems. Order is stable — source
/// order, then first-seen within a source — so the rendered list is reproducible
/// build to build and a diff of two builds is meaningful.
pub fn collect_warnings(
    index: &resolve::manifest::SourceIndex,
    sources: &[&[sigil_span::Diagnostic]],
    generated: Option<sigil_span::SourceId>,
) -> Vec<BuildWarning> {
    collect_warnings_located(&|span| index.locate(span), sources, generated)
}

/// [`collect_warnings`] with the LOCATION AUTHORITY named explicitly, for the
/// chained build — where the diagnostics being collected were raised over a
/// section list built out of BOTH front ends and the `.emp` manifest's index is
/// therefore not the authority for all of them (see [`ChainSources`]).
///
/// Same filter, same dedup key, same order; only the map that answers
/// `path:line:col` differs, which is exactly the thing a mixed build cannot leave
/// implicit.
pub fn collect_warnings_located(
    locate: &dyn Fn(sigil_span::Span) -> Option<String>,
    sources: &[&[sigil_span::Diagnostic]],
    generated: Option<sigil_span::SourceId>,
) -> Vec<BuildWarning> {
    let mut seen = std::collections::HashSet::new();
    sources
        .iter()
        .flat_map(|ds| ds.iter())
        .filter(|d| d.level != sigil_span::Level::Error && Some(d.primary.source) != generated)
        .filter(|d| seen.insert((d.level, d.message.clone(), d.primary)))
        .map(|d| BuildWarning::located(d, locate))
        .collect()
}
