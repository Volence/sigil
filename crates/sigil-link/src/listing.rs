//! `s4.lst` symbol-listing emitter. Target: the AS `-L` symbol-table section
//! that `tools/s4budget.py::parse_symbol_table` and the Oracle symbol loader
//! consume. Scope = symbol name, 24-bit hex value, C(code)/-(equate) marker,
//! `|` separator, the `Symbol Table (* = unused):` header, `N symbols` footer.

/// One symbol row. `is_equate` picks the `-` (equate) vs `C` (code) marker.
///
/// `value` is always the symbol's VMA, the address the code RUNS at. `lma`
/// records the address its bytes are STORED at, and ONLY when the two differ:
/// `None` means unphased (VMA == LMA, or a value symbol that has no storage at
/// all), `Some(l)` means the symbol is PHASED and `l` is where its bytes live.
/// The listing's address rows never carried that distinction, so every consumer
/// that needed it had to re-derive it from somewhere else; see [`emit_listing`]'s
/// Phase Table for what the file says about it now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingSymbol {
    pub name: String,
    pub value: u32,
    pub is_equate: bool,
    pub unused: bool,
    pub lma: Option<u32>,
}

/// Is `part` a synthetic compiler block scope (`asm0`, `asm1`, …)? Those are
/// block-internal names with no source meaning — pure backtrace noise.
fn is_asm_block_scope(part: &str) -> bool {
    part.strip_prefix("asm").is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Rewrite the sigil-canonical mangled `.emp` symbol names into debugger-friendly
/// `Parent.local` names AND drop pure compiler-plumbing synthetics — the appendix
/// filter policy (Stage-3 P2b, OQ-B):
///
///   1. `$<module.path>$<Parent>$<local>` (a `.emp` proc-local, e.g.
///      `$engine.boot$EntryPoint$wait_dma`) → `EntryPoint.wait_dma`. KEEP.
///   2. `__offsets$<module.path>$<Parent>$<local>` (a comptime offset-table entry
///      a debugger user names, e.g. `__offsets$…$Ani_Sonic$Walk`) → `Ani_Sonic.Walk`.
///      KEEP.
///   3. `__align$…` internals and any name carrying an `asm<N>` block scope
///      (e.g. `$engine.boot$asm1$wait_z80`) → DROPPED.
///
/// Plain (unmangled) names pass through untouched. The mangled form uses `$` which
/// `convsym`'s `as_lst` name parser rejects (so mangled locals never reach the deb2
/// table); demangling to a `$`-free `Parent.local` lets the source-meaningful ones
/// survive it, while the plumbing stays dropped by removal here.
pub fn demangle_symbols(symbols: &[ListingSymbol]) -> Vec<ListingSymbol> {
    let mut out = Vec::with_capacity(symbols.len());
    for s in symbols {
        // Plain names (no mangling separator) are already source names.
        if !s.name.contains('$') {
            out.push(s.clone());
            continue;
        }
        let parts: Vec<&str> = s.name.split('$').filter(|p| !p.is_empty()).collect();
        // `__align$module$N` and any name with an `asm<N>` synthetic scope are
        // plumbing — dropped (not emitted → convsym never sees them).
        if parts.first() == Some(&"__align") || parts.iter().any(|p| is_asm_block_scope(p)) {
            continue;
        }
        // `$module$Parent$local` and `__offsets$module$Parent$local` both demangle
        // to their trailing `Parent.local` (the two most-specific components).
        if parts.len() >= 2 {
            let parent = parts[parts.len() - 2];
            let local = parts[parts.len() - 1];
            out.push(ListingSymbol { name: format!("{parent}.{local}"), ..s.clone() });
        }
        // A degenerate single-component mangled name (should not occur — top-level
        // procs emit unmangled) is dropped rather than emit a bare `$`-form.
    }
    out
}

/// Emit the AS-`-L`-compatible symbol-table section. Address symbols are
/// address-sorted; each row is `[*]NAME : HEX C` `|`. One symbol per line keeps it
/// trivially parseable (both consumers iterate matches, so layout is cosmetic).
///
/// # The three sections, and why equates get their own
///
/// The ADDRESS symbols are rendered TWICE, as two views of one table:
///
///  1. the Oracle body listing (`(depth) N/HEXADDR : Name:`), which Oracle's
///     `LoadFromAsListing`/`ParseLineHeader` reads and from which
///     `aeon/tools/scene_spans.py::lst_proc_sizes` derives proc sizes;
///  2. the `Symbol Table (* = unused):` section + its `N symbols` trailer, which
///     `aeon/tools/s4budget.py::parse_listing` reads.
///
/// s4budget CROSS-CHECKS those two views — same length, same `(name, value)`
/// sequence, both equal to the trailer's own count — precisely so a partial parse
/// cannot masquerade as a small program. That invariant is load-bearing, and it is
/// what decides where an EQUATE goes: an equate is a VALUE, not an address, so it
/// belongs in neither view. Putting it only in the symbol table would break the 1:1
/// check; putting it in both would make Oracle resolve a constant as a code address
/// and inject a phantom head into every proc-size window.
///
/// So equates get a THIRD section, appended after the trailer, with a row shape of
/// its own: `EQU <name> = $<8 hex digits>`. That shape matches none of the four
/// consumer grammars in play — s4budget's ` NAME : HEX C|- |` symbol row (no `:`,
/// no `|`), its `(N) i/HEX :` source row and `<N> symbols` trailers, scene_spans'
/// identical `LST_HEAD_RE` address head, and effects_gates' `(0) `-prefixed probe.
/// A tool that wants the value of a published `pub equ` matches
/// `^EQU (\S+) = \$([0-9A-F]{8})$` and cannot collide with an address row.
///
/// The section (and its `N equates` trailer) is OMITTED entirely when there are no
/// equates, so a listing with none is byte-identical to the pre-equate format.
///
/// Values render as the `u32` [`ListingSymbol`] carries: a negative equate appears
/// as its two's-complement pattern, exactly as an address-width AS listing would
/// render it.
///
/// # The Phase Table, and why it is UNCONDITIONAL
///
/// Every address row above prints a VMA. For most symbols that is also the LMA and
/// nothing is lost; for a symbol in a PHASED section (`section … (vma: $8000)`) the
/// printed address is a bank-local runtime address and the bytes are stored
/// somewhere else entirely. The listing said nothing about which was which, so a
/// consumer that needed the distinction had to re-derive it, and re-derivation from
/// the listing alone is impossible: the only recoverable signal is the magnitude of
/// the number, and a phased VMA is an ordinary-looking small address. A fourth
/// section states it instead:
///
/// ```text
///   Phase Table (every address above is a VMA):
///   -------------------------------------------
///
/// PHASE-COUNT 6
/// PHASE SoundTablesZ80_Head VMA $00008000 LMA $000B8000
/// ```
///
/// It is emitted ALWAYS, even at count 0, and that is the whole point. The
/// ambiguity being closed is one bit per LISTING, not one per symbol: with the
/// section unconditional, no section at all means an older sigil that does not know
/// about phasing, `PHASE-COUNT 0` means this sigil looked and found nothing phased,
/// and rows are the phased set with the storage address each one hides. An
/// omitted-when-empty section would leave those first two cases spelled the same
/// way, which is the one reading a consumer cannot recover from.
///
/// The cost is that an unphased listing is no longer byte-identical to the
/// pre-phase format. That trade is deliberate: the byte identity this project
/// protects is the ROM's, not the listing's.
///
/// The row shape matches none of the four consumer grammars, on the same reasoning
/// as the equate row: no `(depth) N/HEX :` head, no ` NAME : HEX C |` symbol row,
/// and the count line says `PHASE-COUNT n`, never `<n> symbols`, so neither
/// s4budget trailer regex sees it.
pub fn emit_listing(symbols: &[ListingSymbol]) -> String {
    let (equates, addrs): (Vec<&ListingSymbol>, Vec<&ListingSymbol>) =
        symbols.iter().partition(|s| s.is_equate);
    let mut rows = addrs;
    rows.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));
    let unused = rows.iter().filter(|s| s.unused).count();

    let mut out = String::new();

    // Oracle's `LoadFromAsListing` reads the per-line BODY listing (via
    // `ParseLineHeader`: `(depth) num/hexaddr :  ... Name:`), NOT the symbol-table
    // section that s4budget reads. Emit one Oracle-parseable body line per symbol
    // first — verified against the real Oracle Symbols.cpp AND s4budget: the body
    // lines precede s4budget's `Symbol Table` header (so it ignores them) and the
    // symbol-table rows below fail Oracle's `ParseLineHeader` (so Oracle ignores
    // them). Each consumer reads exactly its own half of one file.
    for (i, s) in rows.iter().enumerate() {
        out.push_str(&format!("(0) {}/{:X} :        {}:\n", i + 1, s.value, s.name));
    }

    out.push_str("  Symbol Table (* = unused):\n");
    out.push_str("  --------------------------\n\n");
    for s in &rows {
        let star = if s.unused { "*" } else { " " };
        out.push_str(&format!("{star}{} : {:X} C |\n", s.name, s.value));
    }
    out.push_str(&format!("\n   {} symbols\n", rows.len()));
    out.push_str(&format!("    {unused} unused symbols\n"));

    if !equates.is_empty() {
        let mut eqs = equates;
        // Name-sorted: an equate has no address to order by, and a stable order
        // makes the section diffable across builds.
        eqs.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
        out.push_str("\n  Equate Table (name = value; values, not addresses):\n");
        out.push_str("  ---------------------------------------------------\n\n");
        for s in &eqs {
            out.push_str(&format!("EQU {} = ${:08X}\n", s.name, s.value));
        }
        out.push_str(&format!("\n   {} equates\n", eqs.len()));
    }

    // The Phase Table. Address-sorted, matching the two address views above, so a
    // phased row is found at the same place in the ordering as its address row.
    // Emitted unconditionally: `PHASE-COUNT 0` is a POSITIVE statement that this
    // build looked and nothing was phased, which absence cannot express.
    let mut phased: Vec<&&ListingSymbol> = rows.iter().filter(|s| s.lma.is_some()).collect();
    phased.sort_by(|a, b| a.value.cmp(&b.value).then(a.name.cmp(&b.name)));
    out.push_str("\n  Phase Table (every address above is a VMA):\n");
    out.push_str("  -------------------------------------------\n\n");
    out.push_str(&format!("PHASE-COUNT {}\n", phased.len()));
    for s in &phased {
        let lma = s.lma.expect("filtered to Some above");
        out.push_str(&format!("PHASE {} VMA ${:08X} LMA ${:08X}\n", s.name, s.value, lma));
    }

    out
}

// ---------------------------------------------------------------------------
// The source digest
// ---------------------------------------------------------------------------

/// The source digest's format version, written as `DIGEST-FORMAT`. It moves when the
/// grammar does.
pub const SOURCE_DIGEST_FORMAT: u32 = 1;

/// The source digest's header line, after its two-space indent.
pub const SOURCE_DIGEST_HEADER: &str =
    "Source Digest (the files this build read, and the ROM it wrote):";

/// The base a digest path is written relative to. Declaration order is row order: the
/// aeon root first, then the named roots by the bytes of their names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DigestRoot {
    /// The `--aeon` tree, written with no `root=` field.
    Aeon,
    /// `/`, for a file under neither other root: `root=filesystem`.
    Filesystem,
    /// The sigil checkout the assembler was compiled from: `root=sigil`.
    Sigil,
}

impl DigestRoot {
    fn token(self) -> Option<&'static str> {
        match self {
            DigestRoot::Aeon => None,
            DigestRoot::Filesystem => Some("filesystem"),
            DigestRoot::Sigil => Some("sigil"),
        }
    }

    fn from_token(token: &str) -> Option<DigestRoot> {
        match token {
            "filesystem" => Some(DigestRoot::Filesystem),
            "sigil" => Some(DigestRoot::Sigil),
            _ => None,
        }
    }
}

/// How a digested file reached the build: the `origin=` field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestOrigin {
    /// A file under the aeon root that the build only read.
    Source,
    /// A file this same build wrote and then read back.
    Generated,
    /// An executable the build ran.
    Tool,
    /// A file outside the aeon root.
    External,
}

impl DigestOrigin {
    fn token(self) -> &'static str {
        match self {
            DigestOrigin::Source => "source",
            DigestOrigin::Generated => "generated",
            DigestOrigin::Tool => "tool",
            DigestOrigin::External => "external",
        }
    }

    fn from_token(token: &str) -> Option<DigestOrigin> {
        match token {
            "source" => Some(DigestOrigin::Source),
            "generated" => Some(DigestOrigin::Generated),
            "tool" => Some(DigestOrigin::Tool),
            "external" => Some(DigestOrigin::External),
            _ => None,
        }
    }
}

/// A path as the digest writes it: relative to `root`, `/`-separated.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DigestPath {
    pub root: DigestRoot,
    pub path: String,
}

/// One `DIGEST-READ` row: a file the build read, as it was when read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestRead {
    pub crc: u32,
    pub size: u64,
    pub origin: DigestOrigin,
    pub file: DigestPath,
}

/// Everything a `Source Digest` section states. [`emit_source_digest`] writes it and
/// [`parse_source_digest`] reads it back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDigest {
    /// The assembler's semver.
    pub assembler_version: String,
    /// The assembler's source revision, as `sigil --version` reports it.
    pub revision: String,
    /// The assembler's tree state word, as `sigil --version` reports it.
    pub tree_state: String,
    /// The build target flag's spelling.
    pub target: String,
    /// The game the target builds.
    pub game: String,
    /// The debug axis.
    pub debug: bool,
    /// Every `--extra-entry` argument, as given.
    pub extra_entries: Vec<String>,
    /// The define environment the `.emp` build lowered with.
    pub defines: Vec<(String, i128)>,
    /// How many `.emp` files the module scan found.
    pub scan_files: u64,
    /// CRC-32 over the scanned paths: see [`digest_scan_identity`].
    pub scan_crc: u32,
    /// Every file the build read.
    pub reads: Vec<DigestRead>,
    /// CRC-32 of the full shipped ROM file.
    pub rom_crc: u32,
    /// Byte size of the full shipped ROM file.
    pub rom_size: u64,
    /// Where `-o` wrote the ROM; `None` when the build was given no `-o`.
    pub rom_output: Option<DigestPath>,
}

/// `DIGEST-SCAN`'s `(files, crc)` for the paths a module scan found: the count, and
/// CRC-32 over the paths sorted by bytes, each followed by one LF.
pub fn digest_scan_identity(paths: &[String]) -> (u64, u32) {
    let mut sorted: Vec<&str> = paths.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    let mut text = String::new();
    for p in &sorted {
        text.push_str(p);
        text.push('\n');
    }
    (sorted.len() as u64, sigil_span::read_set::crc32(text.as_bytes()))
}

/// A value that must be written as one `key=value` token.
fn digest_token(what: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c == '=') {
        return Err(format!(
            "the digest's {what} `{value}` is empty or contains whitespace or `=`, so it cannot be \
             written as one key=value token"
        ));
    }
    Ok(())
}

fn digest_define_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(format!("the define `{name}` is not an identifier, so its digest row would not parse"));
    }
    Ok(())
}

/// A path must be relative, `/`-separated, with no empty, `.` or `..` component and no
/// line break, or the row that carries it cannot be read back as the same file.
fn digest_path(p: &DigestPath) -> Result<(), String> {
    let s = &p.path;
    let ok = !s.is_empty()
        && !s.contains(['\n', '\r'])
        && s.split('/').all(|c| !c.is_empty() && c != "." && c != "..");
    if !ok {
        return Err(format!(
            "the digest path `{}` is not a relative path without `.`/`..` components and line \
             breaks, so its row would not name one file",
            s.escape_debug()
        ));
    }
    Ok(())
}

/// ` path=<p>`, preceded by ` root=<name>` when the root is a named one. `path=` is last
/// and runs to the end of the line, which is what lets a path contain a space.
fn digest_path_fields(p: &DigestPath) -> String {
    match p.root.token() {
        Some(root) => format!(" root={root} path={}", p.path),
        None => format!(" path={}", p.path),
    }
}

/// Render the `Source Digest` section: the FIRST thing in a listing, so it is written
/// ahead of [`emit_listing`]'s text and closed by `DIGEST-END` and one blank line.
///
/// # Why it goes first
///
/// Every consumer of the rest of the listing either anchors on a row shape the section
/// cannot produce, or reads from the `Symbol Table` header forward. The one that reads
/// state from section headers, oracle's `SymbolTable::parse`, counts every unrecognised
/// line after an `Equate Table` or `Phase Table` header as damage and refuses a listing so
/// damaged when the ROM carries no symbol appendix to bind against; before the `Symbol
/// Table` header it is in its body state, which by design passes over non-matching lines.
///
/// # The grammar
///
/// Every line after the header, its rule and one blank line starts with `DIGEST-` and one
/// keyword from `FORMAT ASSEMBLER SHAPE DEFINE SCAN READ AGGREGATE ROM END`; no keyword is
/// a prefix of another. Fields are single-space `key=value` tokens, and `path=` is always
/// the last field of a line that carries it and runs to the end of the line. READ rows are
/// sorted by `(root, path)` with the aeon root first; the AGGREGATE is CRC-32 over the
/// READ lines exactly as written, each with its LF. The full grammar and field meanings
/// are in `docs/superpowers/notes/2026-09-11-lst-source-digest.md`.
///
/// Refuses, rather than writes, anything the grammar cannot carry: an empty read set, a
/// token with whitespace, a path that is absolute or has a line break or a `.`/`..`
/// component, a path listed twice, or an origin that disagrees with its root.
pub fn emit_source_digest(d: &SourceDigest) -> Result<String, String> {
    digest_token("assembler version", &d.assembler_version)?;
    digest_token("assembler revision", &d.revision)?;
    digest_token("assembler tree state", &d.tree_state)?;
    digest_token("target", &d.target)?;
    digest_token("game", &d.game)?;
    for e in &d.extra_entries {
        digest_token("extra entry", e)?;
        if e.contains(',') || e == "none" {
            return Err(format!(
                "the extra entry `{e}` contains `,` or is spelled `none`, so the digest's \
                 extra-entries list could not be read back as the same entries"
            ));
        }
    }

    let mut defines: Vec<&(String, i128)> = d.defines.iter().collect();
    defines.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (name, _) in &defines {
        digest_define_name(name)?;
    }
    if let Some(w) = defines.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(format!("the define `{}` appears twice in the digest's define environment", w[0].0));
    }

    if d.reads.is_empty() {
        return Err("the build recorded no file read, so the read set was not captured and the \
                    digest would vouch for nothing"
            .to_string());
    }
    let mut reads: Vec<&DigestRead> = d.reads.iter().collect();
    reads.sort_by(|a, b| a.file.cmp(&b.file));
    for r in &reads {
        digest_path(&r.file)?;
        if (r.origin == DigestOrigin::External) != (r.file.root != DigestRoot::Aeon) {
            return Err(format!(
                "the digest row for `{}` is origin={} under {:?}, and a row is external exactly \
                 when it lies outside the aeon root",
                r.file.path,
                r.origin.token(),
                r.file.root
            ));
        }
    }
    if let Some(w) = reads.windows(2).find(|w| w[0].file == w[1].file) {
        return Err(format!("the digest lists `{}` twice", w[0].file.path));
    }
    if let Some(o) = &d.rom_output {
        digest_path(o)?;
    }

    let rule = "-".repeat(SOURCE_DIGEST_HEADER.chars().count());
    let mut out = format!("  {SOURCE_DIGEST_HEADER}\n  {rule}\n\n");
    out.push_str(&format!("DIGEST-FORMAT {SOURCE_DIGEST_FORMAT}\n"));
    out.push_str(&format!(
        "DIGEST-ASSEMBLER sigil version={} revision={} tree={}\n",
        d.assembler_version, d.revision, d.tree_state
    ));
    let entries =
        if d.extra_entries.is_empty() { "none".to_string() } else { d.extra_entries.join(",") };
    out.push_str(&format!(
        "DIGEST-SHAPE target={} game={} debug={} extra-entries={entries}\n",
        d.target,
        d.game,
        u8::from(d.debug)
    ));
    for (name, value) in defines {
        out.push_str(&format!("DIGEST-DEFINE {name}={value}\n"));
    }
    out.push_str(&format!("DIGEST-SCAN pattern=*.emp files={} crc={:08x}\n", d.scan_files, d.scan_crc));
    let mut read_lines = String::new();
    for r in &reads {
        read_lines.push_str(&format!(
            "DIGEST-READ crc={:08x} size={} origin={}{}\n",
            r.crc,
            r.size,
            r.origin.token(),
            digest_path_fields(&r.file)
        ));
    }
    let aggregate = sigil_span::read_set::crc32(read_lines.as_bytes());
    out.push_str(&read_lines);
    out.push_str(&format!("DIGEST-AGGREGATE crc={aggregate:08x} reads={}\n", reads.len()));
    let destination = match &d.rom_output {
        Some(p) => digest_path_fields(p),
        None => " output=none".to_string(),
    };
    out.push_str(&format!("DIGEST-ROM crc={:08x} size={}{destination}\n", d.rom_crc, d.rom_size));
    out.push_str("DIGEST-END\n\n");
    Ok(out)
}

/// One LF-terminated line off the front of `rest`.
fn digest_line<'a>(rest: &mut &'a str, what: &str) -> Result<&'a str, String> {
    let Some(end) = rest.find('\n') else {
        return Err(format!("the digest ends before its {what} line"));
    };
    let line = &rest[..end];
    *rest = &rest[end + 1..];
    Ok(line)
}

/// The values of `fields`, which must be exactly the space-separated `key=value` tokens
/// of `text`, in that order.
fn digest_fields<'a>(text: &'a str, keys: &[&str], line: &str) -> Result<Vec<&'a str>, String> {
    let tokens: Vec<&str> = text.split(' ').collect();
    if tokens.len() != keys.len() {
        return Err(format!("digest line `{line}` does not carry exactly the fields {keys:?}"));
    }
    tokens
        .iter()
        .zip(keys)
        .map(|(t, k)| {
            t.strip_prefix(k)
                .and_then(|v| v.strip_prefix('='))
                .ok_or_else(|| format!("digest line `{line}` has `{t}` where `{k}=` belongs"))
        })
        .collect()
}

fn digest_hex8(v: &str, line: &str) -> Result<u32, String> {
    if v.len() != 8 || !v.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(format!("digest line `{line}` carries `{v}` where 8 lowercase hex digits belong"));
    }
    u32::from_str_radix(v, 16).map_err(|e| format!("digest line `{line}`: {e}"))
}

fn digest_dec(v: &str, line: &str) -> Result<u64, String> {
    v.parse::<u64>().map_err(|_| format!("digest line `{line}` carries `{v}` where a decimal count belongs"))
}

/// `[root=<name>] path=<rest of line>` from the tail of a READ or ROM line.
fn digest_parse_path(fixed: &str, path: &str, line: &str) -> Result<(Vec<String>, DigestPath), String> {
    let mut tokens: Vec<String> = fixed.split(' ').map(str::to_string).collect();
    let mut root = DigestRoot::Aeon;
    if let Some(last) = tokens.last() {
        if let Some(name) = last.strip_prefix("root=") {
            root = DigestRoot::from_token(name)
                .ok_or_else(|| format!("digest line `{line}` names an unknown root `{name}`"))?;
            tokens.pop();
        }
    }
    let file = DigestPath { root, path: path.to_string() };
    digest_path(&file)?;
    Ok((tokens, file))
}

/// Read a `Source Digest` section off the front of `text`, returning it and the rest of
/// the listing after the section's closing blank line.
///
/// Strict where the grammar is: every line kind in order, every field present, READ
/// rows sorted and unique, and the AGGREGATE recomputed from the READ lines as written,
/// so a doctored or truncated section is an error rather than a digest.
pub fn parse_source_digest(text: &str) -> Result<(SourceDigest, &str), String> {
    let mut rest = text;
    let header = format!("  {SOURCE_DIGEST_HEADER}");
    let rule = format!("  {}", "-".repeat(SOURCE_DIGEST_HEADER.chars().count()));
    if digest_line(&mut rest, "header")? != header {
        return Err("the listing does not start with a Source Digest header".to_string());
    }
    if digest_line(&mut rest, "rule")? != rule {
        return Err("the Source Digest header is not followed by its rule".to_string());
    }
    if !digest_line(&mut rest, "blank")?.is_empty() {
        return Err("the Source Digest rule is not followed by a blank line".to_string());
    }

    let line = digest_line(&mut rest, "FORMAT")?;
    if line != format!("DIGEST-FORMAT {SOURCE_DIGEST_FORMAT}") {
        return Err(format!("expected `DIGEST-FORMAT {SOURCE_DIGEST_FORMAT}`, found `{line}`"));
    }

    let line = digest_line(&mut rest, "ASSEMBLER")?;
    let body = line
        .strip_prefix("DIGEST-ASSEMBLER sigil ")
        .ok_or_else(|| format!("expected a DIGEST-ASSEMBLER line, found `{line}`"))?;
    let a = digest_fields(body, &["version", "revision", "tree"], line)?;
    let (assembler_version, revision, tree_state) = (a[0].to_string(), a[1].to_string(), a[2].to_string());

    let line = digest_line(&mut rest, "SHAPE")?;
    let body = line
        .strip_prefix("DIGEST-SHAPE ")
        .ok_or_else(|| format!("expected a DIGEST-SHAPE line, found `{line}`"))?;
    let s = digest_fields(body, &["target", "game", "debug", "extra-entries"], line)?;
    let debug = match s[2] {
        "0" => false,
        "1" => true,
        other => return Err(format!("digest line `{line}` has debug={other}, not 0 or 1")),
    };
    let extra_entries: Vec<String> =
        if s[3] == "none" { Vec::new() } else { s[3].split(',').map(str::to_string).collect() };
    let (target, game) = (s[0].to_string(), s[1].to_string());

    let mut defines: Vec<(String, i128)> = Vec::new();
    let mut line = digest_line(&mut rest, "DEFINE or SCAN")?;
    while let Some(body) = line.strip_prefix("DIGEST-DEFINE ") {
        let (name, value) =
            body.split_once('=').ok_or_else(|| format!("digest line `{line}` is not NAME=INT"))?;
        digest_define_name(name)?;
        let value: i128 =
            value.parse().map_err(|_| format!("digest line `{line}` carries a value that is not an integer"))?;
        if defines.last().is_some_and(|(prev, _)| prev.as_bytes() >= name.as_bytes()) {
            return Err(format!("digest line `{line}` is out of order or repeats a define"));
        }
        defines.push((name.to_string(), value));
        line = digest_line(&mut rest, "DEFINE or SCAN")?;
    }

    let body = line
        .strip_prefix("DIGEST-SCAN ")
        .ok_or_else(|| format!("expected a DIGEST-SCAN line, found `{line}`"))?;
    let sc = digest_fields(body, &["pattern", "files", "crc"], line)?;
    if sc[0] != "*.emp" {
        return Err(format!("digest line `{line}` scans `{}`, not `*.emp`", sc[0]));
    }
    let (scan_files, scan_crc) = (digest_dec(sc[1], line)?, digest_hex8(sc[2], line)?);

    let mut reads: Vec<DigestRead> = Vec::new();
    let mut read_lines = String::new();
    let mut line = digest_line(&mut rest, "READ")?;
    while let Some(body) = line.strip_prefix("DIGEST-READ ") {
        let (fixed, path) = body
            .split_once(" path=")
            .ok_or_else(|| format!("digest line `{line}` carries no path= field"))?;
        let (tokens, file) = digest_parse_path(fixed, path, line)?;
        let joined = tokens.join(" ");
        let f = digest_fields(&joined, &["crc", "size", "origin"], line)?;
        let origin = DigestOrigin::from_token(f[2])
            .ok_or_else(|| format!("digest line `{line}` has an unknown origin `{}`", f[2]))?;
        if (origin == DigestOrigin::External) != (file.root != DigestRoot::Aeon) {
            return Err(format!("digest line `{line}` is external exactly when it names a root, and it does not"));
        }
        if reads.last().is_some_and(|prev: &DigestRead| prev.file >= file) {
            return Err(format!("digest line `{line}` is out of order or repeats a file"));
        }
        reads.push(DigestRead { crc: digest_hex8(f[0], line)?, size: digest_dec(f[1], line)?, origin, file });
        read_lines.push_str(line);
        read_lines.push('\n');
        line = digest_line(&mut rest, "READ or AGGREGATE")?;
    }
    if reads.is_empty() {
        return Err("the digest carries no DIGEST-READ row".to_string());
    }

    let body = line
        .strip_prefix("DIGEST-AGGREGATE ")
        .ok_or_else(|| format!("expected a DIGEST-AGGREGATE line, found `{line}`"))?;
    let ag = digest_fields(body, &["crc", "reads"], line)?;
    let (claimed, count) = (digest_hex8(ag[0], line)?, digest_dec(ag[1], line)?);
    let actual = sigil_span::read_set::crc32(read_lines.as_bytes());
    if claimed != actual || count != reads.len() as u64 {
        return Err(format!(
            "the digest's aggregate says crc={claimed:08x} reads={count} and its READ rows give \
             crc={actual:08x} reads={}",
            reads.len()
        ));
    }

    let line = digest_line(&mut rest, "ROM")?;
    let body = line
        .strip_prefix("DIGEST-ROM ")
        .ok_or_else(|| format!("expected a DIGEST-ROM line, found `{line}`"))?;
    let (rom_fields, rom_output) = match body.split_once(" path=") {
        Some((fixed, path)) => {
            let (tokens, file) = digest_parse_path(fixed, path, line)?;
            (tokens.join(" "), Some(file))
        }
        None => {
            let fixed = body
                .strip_suffix(" output=none")
                .ok_or_else(|| format!("digest line `{line}` carries neither path= nor output=none"))?;
            (fixed.to_string(), None)
        }
    };
    let r = digest_fields(&rom_fields, &["crc", "size"], line)?;
    let (rom_crc, rom_size) = (digest_hex8(r[0], line)?, digest_dec(r[1], line)?);

    if digest_line(&mut rest, "END")? != "DIGEST-END" {
        return Err("the digest is not closed by DIGEST-END".to_string());
    }
    if !digest_line(&mut rest, "closing blank")?.is_empty() {
        return Err("DIGEST-END is not followed by a blank line".to_string());
    }

    let digest = SourceDigest {
        assembler_version,
        revision,
        tree_state,
        target,
        game,
        debug,
        extra_entries,
        defines,
        scan_files,
        scan_crc,
        reads,
        rom_crc,
        rom_size,
        rom_output,
    };
    Ok((digest, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str, value: u32, eq: bool, unused: bool) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: eq, unused, lma: None }
    }

    /// A PHASED address symbol: runs at `value`, stored at `lma`.
    fn phased(name: &str, value: u32, lma: u32) -> ListingSymbol {
        ListingSymbol { name: name.into(), value, is_equate: false, unused: false, lma: Some(lma) }
    }

    #[test]
    fn emits_s4budget_parseable_rows() {
        // Mirror s4budget's regex: (\*?)([\w.]+)\s*:\s*(hex|"str")\s+([C\-])\s*\|
        let out = emit_listing(&[
            sym("Main", 0x000000, false, false),
            sym("Boot", 0x40, false, false),
            sym("Unused", 0x2000, false, true),
        ]);
        assert!(out.contains("Symbol Table"));
        assert!(out.contains("unused"));
        // address-sorted; every symbol-table row is an address, marker C.
        assert!(out.contains("Main : 0 C |"));
        assert!(out.contains("Boot : 40 C |"));
        assert!(out.contains("*Unused : 2000 C |"));
        assert!(out.contains("3 symbols"));
        assert!(out.contains("1 unused symbols"));
        // No equates in this set → no Equate Table at all (format unchanged).
        assert!(!out.contains("Equate Table"), "an empty equate section was emitted:\n{out}");
    }

    #[test]
    fn regex_intersection_matches_each_row() {
        // A pure-Rust stand-in for s4budget's regex to prove the grammar holds.
        let out = emit_listing(&[sym("Air_LandState", 0x10AF2, false, false)]);
        let re_ok = out.lines().any(|l| {
            let l = l.trim_start();
            // [*]name : HEX (C|-) |
            l.contains(" : ") && l.trim_end().ends_with('|')
                && (l.contains(" C |") || l.contains(" - |"))
        });
        assert!(re_ok, "no parseable row in:\n{out}");
    }

    #[test]
    fn demangler_keeps_proc_local_and_offsets_drops_plumbing() {
        let out = demangle_symbols(&[
            // (1) a .emp proc-local → Parent.local, KEPT.
            sym("$engine.boot$EntryPoint$wait_dma", 0x210, false, false),
            // (2) a source-meaningful comptime offset entry → Parent.local, KEPT.
            sym("__offsets$games.sonic4.sonic_anims$Ani_Sonic$Walk", 0x256F2, false, false),
            // (3a) an asm<N> block scope → DROPPED.
            sym("$engine.boot$asm1$wait_z80", 0x260, false, false),
            // (3b) an __align internal → DROPPED.
            sym("__align$games.sonic4.sonic_anims$0", 0x2574A, false, false),
            // plain → untouched.
            sym("EntryPoint", 0x200, false, false),
        ]);
        let names: Vec<&str> = out.iter().map(|s| s.name.as_str()).collect();
        // KEPT + demangled, value preserved.
        assert!(names.contains(&"EntryPoint.wait_dma"), "proc-local demangle: {names:?}");
        assert!(names.contains(&"Ani_Sonic.Walk"), "__offsets demangle: {names:?}");
        assert_eq!(out.iter().find(|s| s.name == "EntryPoint.wait_dma").unwrap().value, 0x210);
        // plain pass-through.
        assert!(names.contains(&"EntryPoint"));
        // DROPPED — the t24 must-NOT-survive control.
        assert!(!names.iter().any(|n| n.contains("asm1")), "asm block scope leaked: {names:?}");
        assert!(!names.iter().any(|n| n.contains("__align") || *n == "sonic_anims.0"), "align internal leaked: {names:?}");
        // No `$` survives into the demangled set (convsym would drop those).
        assert!(!names.iter().any(|n| n.contains('$')), "a mangled `$` name survived: {names:?}");
    }

    #[test]
    fn demangler_is_asm_block_scope_precise() {
        assert!(is_asm_block_scope("asm0"));
        assert!(is_asm_block_scope("asm12"));
        // NOT block scopes — real names beginning with `asm` keep their locals.
        assert!(!is_asm_block_scope("asm"));
        assert!(!is_asm_block_scope("asmName"));
        assert!(!is_asm_block_scope("assemble"));
    }

    #[test]
    fn emits_oracle_body_lines_before_symbol_table() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("Boot", 0x40, false, false),
        ]);
        // Oracle body lines (ParseLineHeader format) come first, address-sorted.
        // `(depth) N/HEXADDR :        Name:`
        assert!(out.contains("(0) 1/40 :        Boot:"), "missing/incorrect body line:\n{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "missing/incorrect body line:\n{out}");
        // Every body line must precede the Symbol Table header (s4budget reads only
        // after that header; Oracle reads only the body lines).
        let body_idx = out.find("(0) 1/40").unwrap();
        let tab_idx = out.find("Symbol Table").unwrap();
        assert!(body_idx < tab_idx, "body lines must precede the symbol-table section");
        // The symbol-table section is still present and unchanged.
        assert!(out.contains("Main : 1000 C |"));
        assert!(out.contains("Boot : 40 C |"));
    }

    /// The equate row shape, stated as a contract: an equate lives ONLY in the
    /// Equate Table, as `EQU <name> = $<8 hex>`, and appears in NEITHER of the two
    /// address views.
    ///
    /// This is the half of the equ-listing parcel the emitter owns. `pub equ` mints
    /// a link-level `EquSym` with no label, so before this an equate had no row of
    /// any kind and a comptime-computed constant was unreadable by any tool.
    #[test]
    fn equates_get_a_value_row_and_no_address_row() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("SCENE_OJZ_BUDGET", 0x2C, true, false),
            sym("Boot", 0x40, false, false),
        ]);
        // The value row exists, in the Equate Table, with the computed value.
        assert!(out.contains("Equate Table"), "no equate section:\n{out}");
        assert!(out.contains("EQU SCENE_OJZ_BUDGET = $0000002C"), "no equate row:\n{out}");
        assert!(out.contains("1 equates"), "no equate trailer:\n{out}");
        // …and NO address view names it — neither body line nor symbol-table row.
        assert!(
            !out.lines().any(|l| l.starts_with("(0) ") && l.contains("SCENE_OJZ_BUDGET")),
            "an equate reached the Oracle address listing:\n{out}"
        );
        assert!(
            !out.contains("SCENE_OJZ_BUDGET : "),
            "an equate reached the symbol table:\n{out}"
        );
        // The two address views stay 1:1 with each other AND with the trailer —
        // the s4budget cross-check that decides where an equate may live.
        assert!(out.contains("(0) 1/40 :        Boot:"), "body numbering:\n{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "body numbering:\n{out}");
        assert!(out.contains("Boot : 40 C |") && out.contains("Main : 1000 C |"), "{out}");
        assert!(out.contains("2 symbols"), "the equate must not inflate the count:\n{out}");
    }

    /// COLLISION CONTROL. An equate row must never parse as an address row under
    /// any of aeon's `.lst` consumer grammars. Those are, verbatim:
    ///
    ///  * `tools/scene_spans.py::LST_HEAD_RE` —
    ///    `^\(\d+\) \d+/([0-9A-F]+) :\s+([A-Za-z_][A-Za-z0-9_]*):\s*$`
    ///    (drives `lst_proc_sizes`, hence `demo_specialization_witness.py`'s
    ///    proc-size differential);
    ///  * `tools/effects_gates.py`'s dense-stream probe —
    ///    `line.startswith("(0) ") and line.rstrip().endswith("<Name>:")`;
    ///  * `tools/s4budget.py`'s `_SYM_ROW_RE`
    ///    `^\s*(\*?)([\w.$]+)\s*:\s*([0-9A-Fa-f]+)\s+([C\-])\s*\|\s*$`, its
    ///    `_SRC_ROW_RE`, and its `<N> symbols` / `<N> unused symbols` trailers.
    ///
    /// Every one of them is an ADDRESS reader. The equate row starts with the
    /// literal `EQU `, carries no `/`, no ` : `, no `|`, and its trailer says
    /// `equates`, not `symbols` — proven here against a hostile equate deliberately
    /// named like a proc and valued like a ROM address.
    #[test]
    fn equate_row_never_parses_as_an_address_row() {
        // Named like a proc, valued like a real ROM address: if the shapes could
        // collide at all, this row is the one that would do it.
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            sym("OJZ_GradientStream", 0x10AF2, true, false),
        ]);

        // scene_spans.LST_HEAD_RE, transcribed.
        let head_re = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(") ") else { return false };
            if depth.is_empty() || !depth.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((hex, rest)) = rest.split_once(" :") else { return false };
            if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return false;
            }
            let name = rest.trim_start();
            let Some(name) = name.strip_suffix(':') else { return false };
            !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && !name.as_bytes()[0].is_ascii_digit()
        };
        // effects_gates' dense-stream probe, transcribed.
        let gate_probe =
            |l: &str| l.starts_with("(0) ") && l.trim_end().ends_with("OJZ_GradientStream:");
        // s4budget's `_SYM_ROW_RE`, transcribed.
        let sym_row = |l: &str| -> bool {
            let l = l.trim_start().trim_start_matches('*');
            let Some((name, rest)) = l.split_once(':') else { return false };
            let name = name.trim_end();
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.$".contains(&b))
            {
                return false;
            }
            let rest = rest.trim_start();
            let Some((hex, rest)) = rest.split_once(' ') else { return false };
            hex.bytes().all(|b| b.is_ascii_hexdigit())
                && !hex.is_empty()
                && matches!(rest.trim().trim_end_matches('|').trim(), "C" | "-")
                && rest.trim_end().ends_with('|')
        };
        // s4budget's two trailers, transcribed.
        let trailer = |l: &str| {
            let t = l.trim();
            t.strip_suffix(" symbols").is_some_and(|n| {
                let n = n.trim_end_matches("unused").trim_end();
                !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
            })
        };

        // Only the two EQUATE-SECTION lines are under test — the address half of
        // this listing is supposed to match, and does (controls below).
        for line in out.lines().filter(|l| l.starts_with("EQU ") || l.contains("equates")) {
            assert!(!head_re(line), "an equate line parsed as an address head: {line:?}");
            assert!(!gate_probe(line), "an equate line matched the effects gate probe: {line:?}");
            assert!(!sym_row(line), "an equate line parsed as a symbol-table row: {line:?}");
            assert!(!trailer(line), "an equate line parsed as a symbol trailer: {line:?}");
        }
        // Positive controls on the SAME transcriptions: the address rows DO match,
        // so the assertions above test the row shape, not a broken transcription.
        let addr = emit_listing(&[sym("OJZ_GradientStream", 0x10AF2, false, false)]);
        assert!(
            addr.lines().any(head_re),
            "the transcribed LST_HEAD_RE matches no address row, the control is broken:\n{addr}"
        );
        assert!(addr.lines().any(gate_probe), "the transcribed gate probe is broken:\n{addr}");
        assert!(addr.lines().any(sym_row), "the transcribed _SYM_ROW_RE is broken:\n{addr}");
        assert!(addr.lines().any(trailer), "the transcribed trailer regex is broken:\n{addr}");
        // And the equate's value is readable from its own row regardless.
        assert!(out.contains("EQU OJZ_GradientStream = $00010AF2"), "{out}");
    }

    /// The s4budget INVARIANT this design exists to preserve, asserted directly:
    /// the two address views are the same `(name, value)` sequence, of the same
    /// length, equal to the `N symbols` trailer — with equates present.
    #[test]
    fn equates_do_not_disturb_the_two_view_cross_check() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("A_EQ", 0x2C, true, false),
            sym("Boot", 0x40, false, false),
            sym("Z_EQ", 0xFFFF0000, true, false),
            sym("Tail", 0x8000, false, true),
        ]);
        let body: Vec<(String, u32)> = out
            .lines()
            .filter_map(|l| l.strip_prefix("(0) "))
            .map(|l| {
                let (head, name) = l.split_once(" :").unwrap();
                let hex = head.split_once('/').unwrap().1;
                (name.trim().trim_end_matches(':').to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        let table: Vec<(String, u32)> = out
            .lines()
            .filter(|l| l.trim_end().ends_with(" C |"))
            .map(|l| {
                let l = l.trim_start_matches([' ', '*']);
                let (name, rest) = l.split_once(" : ").unwrap();
                let hex = rest.split_once(' ').unwrap().0;
                (name.to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        assert_eq!(body, table, "the two address views must be one table");
        assert_eq!(body.len(), 3, "only the three address symbols: {body:?}");
        assert!(out.contains("3 symbols") && out.contains("1 unused symbols"), "{out}");
        assert!(out.contains("2 equates"), "{out}");
    }

    /// THE CASE THAT CLOSES THE AMBIGUITY, and the one nothing else exercises.
    ///
    /// A listing of an entirely unphased program still carries the Phase Table,
    /// with `PHASE-COUNT 0` and no rows. Without the unconditional header a reader
    /// cannot tell "this sigil looked and found nothing phased" from "this sigil
    /// predates the marker and every address here might be either", and the two would
    /// be spelled identically, as an absent section.
    #[test]
    fn unphased_listing_still_carries_a_count_zero_phase_table() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            sym("Boot", 0x40, false, false),
            sym("OBJ_len", 0x40, true, false),
        ]);
        assert!(out.contains("Phase Table"), "no phase section on an unphased listing:\n{out}");
        assert!(out.contains("PHASE-COUNT 0"), "no zero count:\n{out}");
        // No rows at all, and in particular no row for the unphased symbols.
        assert!(
            !out.lines().any(|l| l.starts_with("PHASE ") && l.contains(" VMA ")),
            "a row was emitted for an unphased symbol:\n{out}"
        );
        // The rest of the listing is untouched: the marker ADDS a section, it does
        // not reinterpret or renumber any existing row.
        assert!(out.contains("(0) 1/40 :        Boot:"), "{out}");
        assert!(out.contains("(0) 2/1000 :        Main:"), "{out}");
        assert!(out.contains("Boot : 40 C |") && out.contains("Main : 1000 C |"), "{out}");
        assert!(out.contains("2 symbols"), "the phase table must not disturb the count:\n{out}");
        assert!(out.contains("EQU OBJ_len = $00000040"), "{out}");
    }

    /// A phased symbol gets a row naming BOTH addresses, and its address rows are
    /// unchanged: `value` was already the VMA and stays the VMA everywhere.
    #[test]
    fn phased_symbols_get_vma_and_lma_rows() {
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            phased("SoundTablesZ80_Head", 0x8000, 0xE12C0),
            phased("SfxBlobWinTab", 0x845F, 0xE171F),
            sym("Tail", 0x20000, false, false),
        ]);
        assert!(out.contains("PHASE-COUNT 2"), "wrong count:\n{out}");

        // CROSS-LANE CONTRACT, in TWO shapes deliberately.
        //
        // The consumer is `oracle`'s `SymbolTable::parse`, at
        // `crates/oracle-core/src/symbols.rs:553` (verified present when this was
        // written). It is named by REPO AND PARSER rather than by the session that
        // asked for it: a session handle stops existing, and a comment citing one
        // reads in six weeks as a note from a ghost, which is worse than no
        // attribution because the next editor cannot tell whether the dependency is
        // still real. THE PARSER IS THE THING THAT WILL STILL BE THERE TO BREAK.
        //
        // It keys rows on `^PHASE ` WITH THE TRAILING SPACE, and the count on
        // `^PHASE-COUNT `. THE TRAILING SPACE IS LOAD-BEARING: `^PHASE` and
        // `^PHASE-COUNT` are NOT DISJOINT AS PREFIXES, so a consumer keying on the
        // bare word matches both and will not notice. That is not hypothetical: the
        // first version of THIS TEST did exactly that, asserting only that every
        // line began with `PHASE`, which would have kept passing while the two keys
        // silently merged into one. The pin failed in the way the pin exists to
        // prevent, one level up from where it was written.
        //
        // So the two shapes are asserted SEPARATELY and must stay disjoint.
        //
        // `PHASE-COUNT` rather than `PHASE COUNT` because a symbol legitimately
        // named `COUNT` emits `PHASE COUNT VMA $...`, matching `^PHASE COUNT `
        // exactly as the count line did. Population is zero today and the collision
        // is reachable, so the old spelling was unambiguous ON CURRENT EVIDENCE,
        // which is a different property from unambiguous.
        //
        // The alternative was to discriminate on the THIRD TOKEN (`VMA` versus a
        // number). Rejected, and the reason is the one that outlives this comment:
        // that is a POSITIONAL rule, and positional rules break on the most ordinary
        // future edit there is, ADDING A FIELD. `PHASE-COUNT` puts the discriminator
        // in the line's own identity, so it survives the format gaining columns.
        let rows: Vec<&str> = out.lines().filter(|l| l.contains(" VMA $")).collect();
        assert!(!rows.is_empty(), "no phase rows to check, the assertion would be vacuous");
        for line in &rows {
            assert!(
                line.starts_with("PHASE ") && !line.starts_with("PHASE-"),
                "a phase ROW must match `^PHASE ` at column 0, oracle keys on it: {line:?}"
            );
        }
        let counts: Vec<&str> =
            out.lines().filter(|l| l.starts_with("PHASE-COUNT ")).collect();
        assert_eq!(counts.len(), 1, "exactly one count line, at column 0: {counts:?}");

        assert!(
            out.contains("PHASE SoundTablesZ80_Head VMA $00008000 LMA $000E12C0"),
            "missing/incorrect phased row:\n{out}"
        );
        assert!(
            out.contains("PHASE SfxBlobWinTab VMA $0000845F LMA $000E171F"),
            "missing/incorrect phased row:\n{out}"
        );
        // Address-sorted, like the two address views.
        let a = out.find("PHASE SoundTablesZ80_Head").unwrap();
        let b = out.find("PHASE SfxBlobWinTab").unwrap();
        assert!(a < b, "phase rows are not address-sorted:\n{out}");
        // The two address views still carry the phased symbols at their VMA, and
        // still cross-check 1:1 against the trailer, so the marker is ADDITIVE.
        assert!(out.contains("(0) 2/8000 :        SoundTablesZ80_Head:"), "{out}");
        assert!(out.contains("SoundTablesZ80_Head : 8000 C |"), "{out}");
        assert!(out.contains("4 symbols"), "{out}");
        // No unphased symbol acquired a row.
        assert!(!out.contains("PHASE Anchor"), "{out}");
        assert!(!out.contains("PHASE Tail"), "{out}");
    }

    /// COLLISION CONTROL for the phase rows, the same shape as the equate one: the
    /// header, the rule, the count line and a hostile row must parse as an address
    /// row under NONE of aeon's four `.lst` consumer grammars, proven against the
    /// same transcriptions with positive controls so a broken transcription cannot
    /// pass this test by matching nothing.
    #[test]
    fn phase_lines_never_parse_as_an_address_row() {
        // Named like a proc, valued like a real ROM address at both ends.
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            phased("OJZ_GradientStream", 0x10AF2, 0x2C0FE),
        ]);
        // Same transcriptions the equate control uses.
        let head_re = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(") ") else { return false };
            if depth.is_empty() || !depth.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((hex, rest)) = rest.split_once(" :") else { return false };
            if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return false;
            }
            let name = rest.trim_start();
            let Some(name) = name.strip_suffix(':') else { return false };
            !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && !name.as_bytes()[0].is_ascii_digit()
        };
        let gate_probe = |l: &str| l.starts_with("(0) ") && l.trim_end().ends_with(':');
        let sym_row = |l: &str| -> bool {
            let l = l.trim_start().trim_start_matches('*');
            let Some((name, rest)) = l.split_once(':') else { return false };
            let name = name.trim_end();
            if name.is_empty()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.$".contains(&b))
            {
                return false;
            }
            let rest = rest.trim_start();
            let Some((hex, rest)) = rest.split_once(' ') else { return false };
            hex.bytes().all(|b| b.is_ascii_hexdigit())
                && !hex.is_empty()
                && matches!(rest.trim().trim_end_matches('|').trim(), "C" | "-")
                && rest.trim_end().ends_with('|')
        };
        // BOTH s4budget trailers, transcribed as anchored matches.
        let trailer = |l: &str| {
            let t = l.trim();
            [" symbols", " unused symbols"].iter().any(|suffix| {
                t.strip_suffix(suffix)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        };

        // Every line the phase section contributes: header, rule, blank, count, row.
        let phase_start = out.find("  Phase Table").expect("no phase section");
        for line in out[phase_start..].lines() {
            assert!(!head_re(line), "a phase line parsed as an address head: {line:?}");
            assert!(!gate_probe(line), "a phase line matched the effects gate probe: {line:?}");
            assert!(!sym_row(line), "a phase line parsed as a symbol-table row: {line:?}");
            assert!(!trailer(line), "a phase line parsed as a symbol trailer: {line:?}");
        }
        // Positive controls on the SAME transcriptions: real rows DO match, so the
        // assertions above test the phase row shape, not a dead transcription.
        let addr = emit_listing(&[sym("OJZ_GradientStream", 0x10AF2, false, false)]);
        assert!(addr.lines().any(head_re), "the transcribed LST_HEAD_RE is broken:\n{addr}");
        assert!(addr.lines().any(gate_probe), "the transcribed gate probe is broken:\n{addr}");
        assert!(addr.lines().any(sym_row), "the transcribed _SYM_ROW_RE is broken:\n{addr}");
        assert!(addr.lines().any(trailer), "the transcribed trailer regex is broken:\n{addr}");
        // And the count line specifically, which is the line that is ALWAYS there.
        assert!(!trailer("PHASE-COUNT 0"), "the count line parsed as a symbol trailer");
        assert!(!sym_row("PHASE-COUNT 0"), "the count line parsed as a symbol row");
    }

    /// s4budget's cross-check invariant, re-asserted with PHASED symbols present:
    /// the two address views stay one table, at the VMA, and the trailer agrees.
    #[test]
    fn phase_table_does_not_disturb_the_two_view_cross_check() {
        let out = emit_listing(&[
            sym("Main", 0x1000, false, false),
            phased("BankHead", 0x8000, 0xE0000),
            sym("Boot", 0x40, false, false),
            sym("A_EQ", 0x2C, true, false),
        ]);
        let body: Vec<(String, u32)> = out
            .lines()
            .filter_map(|l| l.strip_prefix("(0) "))
            .map(|l| {
                let (head, name) = l.split_once(" :").unwrap();
                let hex = head.split_once('/').unwrap().1;
                (name.trim().trim_end_matches(':').to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        let table: Vec<(String, u32)> = out
            .lines()
            .filter(|l| l.trim_end().ends_with(" C |"))
            .map(|l| {
                let l = l.trim_start_matches([' ', '*']);
                let (name, rest) = l.split_once(" : ").unwrap();
                let hex = rest.split_once(' ').unwrap().0;
                (name.to_string(), u32::from_str_radix(hex, 16).unwrap())
            })
            .collect();
        assert_eq!(body, table, "the two address views must be one table");
        assert_eq!(body.len(), 3, "only the three address symbols: {body:?}");
        assert!(out.contains("3 symbols"), "{out}");
        // The phased symbol appears in BOTH address views at its VMA, and once more
        // in the phase table with its LMA. Three rows, one truth, no reinterpretation.
        assert!(body.contains(&("BankHead".to_string(), 0x8000)), "{body:?}");
        assert!(out.contains("PHASE-COUNT 1") && out.contains("PHASE BankHead VMA $00008000 LMA $000E0000"), "{out}");
    }

    /// An EQUATE is never phased, whatever it carries: it has a value, not storage.
    #[test]
    fn an_equate_never_reaches_the_phase_table() {
        let out = emit_listing(&[
            sym("Anchor", 0x200, false, false),
            ListingSymbol {
                name: "BANK_BASE".into(),
                value: 0x8000,
                is_equate: true,
                unused: false,
                lma: Some(0xE0000),
            },
        ]);
        assert!(out.contains("PHASE-COUNT 0"), "an equate was counted as phased:\n{out}");
        assert!(!out.contains("PHASE BANK_BASE"), "an equate got a phase row:\n{out}");
        assert!(out.contains("EQU BANK_BASE = $00008000"), "{out}");
    }

    fn dp(root: DigestRoot, path: &str) -> DigestPath {
        DigestPath { root, path: path.into() }
    }

    /// A digest carrying one row of every origin and root, deliberately out of order,
    /// with a path that contains a space.
    fn sample_digest() -> SourceDigest {
        let (scan_files, scan_crc) =
            digest_scan_identity(&["games/b.emp".to_string(), "engine/a.emp".to_string()]);
        SourceDigest {
            assembler_version: "0.1.0".into(),
            revision: "158feb5ec84876e5c7ea44e7019b1921ee72d593".into(),
            tree_state: "clean".into(),
            target: "sonic4".into(),
            game: "sonic4".into(),
            debug: false,
            extra_entries: Vec::new(),
            defines: vec![("MAX_RING_BUFFER".into(), 128), ("DEBUG".into(), 0), ("NEG".into(), -3)],
            scan_files,
            scan_crc,
            reads: vec![
                DigestRead {
                    crc: 0x946d_49d7,
                    size: 21741,
                    origin: DigestOrigin::Source,
                    file: dp(DigestRoot::Aeon, "games/sonic4/map.toml"),
                },
                DigestRead {
                    crc: 0xa477_aa73,
                    size: 2195,
                    origin: DigestOrigin::External,
                    file: dp(DigestRoot::Sigil, "crates/sigil-harness/golden/offcanonical_sizes/s4.txt"),
                },
                DigestRead {
                    crc: 0xfe15_03bc,
                    size: 127,
                    origin: DigestOrigin::Generated,
                    file: dp(DigestRoot::Aeon, "engine/sound/generated/dac_sample_tab.bin"),
                },
                DigestRead {
                    crc: 0x0102_0304,
                    size: 9,
                    origin: DigestOrigin::External,
                    file: dp(DigestRoot::Filesystem, "opt/some file.bin"),
                },
                DigestRead {
                    crc: 0xf43b_95b0,
                    size: 1_909_472,
                    origin: DigestOrigin::Tool,
                    file: dp(DigestRoot::Aeon, "tools/convsym"),
                },
            ],
            rom_crc: 0xb09c_cd65,
            rom_size: 820_229,
            rom_output: Some(dp(DigestRoot::Aeon, "s4.bin")),
        }
    }

    /// The emitted text, line for line, against the grammar committed in
    /// `docs/superpowers/notes/2026-09-11-lst-source-digest.md`.
    #[test]
    fn source_digest_spells_the_committed_grammar() {
        let text = emit_source_digest(&sample_digest()).expect("the sample renders");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "  Source Digest (the files this build read, and the ROM it wrote):");
        assert_eq!(lines[1], format!("  {}", "-".repeat(64)));
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "DIGEST-FORMAT 1");
        assert_eq!(
            lines[4],
            "DIGEST-ASSEMBLER sigil version=0.1.0 revision=158feb5ec84876e5c7ea44e7019b1921ee72d593 tree=clean"
        );
        assert_eq!(lines[5], "DIGEST-SHAPE target=sonic4 game=sonic4 debug=0 extra-entries=none");
        assert_eq!(
            &lines[6..9],
            ["DIGEST-DEFINE DEBUG=0", "DIGEST-DEFINE MAX_RING_BUFFER=128", "DIGEST-DEFINE NEG=-3"],
            "defines are sorted by name"
        );
        assert_eq!(
            lines[9],
            format!("DIGEST-SCAN pattern=*.emp files=2 crc={:08x}", sigil_span::read_set::crc32(b"engine/a.emp\ngames/b.emp\n"))
        );
        let reads: Vec<&str> = lines.iter().copied().filter(|l| l.starts_with("DIGEST-READ ")).collect();
        assert_eq!(
            reads,
            [
                "DIGEST-READ crc=fe1503bc size=127 origin=generated path=engine/sound/generated/dac_sample_tab.bin",
                "DIGEST-READ crc=946d49d7 size=21741 origin=source path=games/sonic4/map.toml",
                "DIGEST-READ crc=f43b95b0 size=1909472 origin=tool path=tools/convsym",
                "DIGEST-READ crc=01020304 size=9 origin=external root=filesystem path=opt/some file.bin",
                "DIGEST-READ crc=a477aa73 size=2195 origin=external root=sigil path=crates/sigil-harness/golden/offcanonical_sizes/s4.txt",
            ],
            "rows sort by (root, path), the aeon root first"
        );
        // The aggregate, recomputed here from its stated definition rather than trusted.
        let mut canonical = String::new();
        for r in &reads {
            canonical.push_str(r);
            canonical.push('\n');
        }
        let aggregate =
            format!("DIGEST-AGGREGATE crc={:08x} reads=5", sigil_span::read_set::crc32(canonical.as_bytes()));
        assert_eq!(lines[15], aggregate);
        assert_eq!(lines[16], "DIGEST-ROM crc=b09ccd65 size=820229 path=s4.bin");
        assert_eq!(lines[17], "DIGEST-END");
        assert_eq!(lines[18], "", "one blank line closes the section");
        assert_eq!(lines.len(), 19);
        assert!(text.ends_with("DIGEST-END\n\n"), "closed by DIGEST-END and exactly one blank line");
    }

    #[test]
    fn source_digest_round_trips_through_its_parser() {
        let d = sample_digest();
        let section = emit_source_digest(&d).expect("the sample renders");
        let listing = emit_listing(&[sym("Main", 0x200, false, false), sym("OBJ_len", 0x40, true, false)]);
        let whole = format!("{section}{listing}");
        let (back, rest) = parse_source_digest(&whole).expect("the section parses");
        assert_eq!(rest, listing, "the section must end exactly where the unchanged listing starts");
        let mut want = d.clone();
        want.defines.sort_by(|a, b| a.0.cmp(&b.0));
        want.reads.sort_by(|a, b| a.file.cmp(&b.file));
        assert_eq!(back, want);

        let mut no_output = sample_digest();
        no_output.rom_output = None;
        no_output.extra_entries = vec!["games.sonic4.test.poison".into(), "engine/x.emp".into()];
        let section = emit_source_digest(&no_output).expect("renders");
        assert!(section.contains("DIGEST-ROM crc=b09ccd65 size=820229 output=none\n"), "{section}");
        assert!(section.contains("extra-entries=games.sonic4.test.poison,engine/x.emp\n"), "{section}");
        let (back, _) = parse_source_digest(&section).expect("parses");
        assert_eq!(back.rom_output, None);
        assert_eq!(back.extra_entries, no_output.extra_entries);
    }

    #[test]
    fn source_digest_scan_identity_is_the_crc_of_sorted_paths() {
        let a = digest_scan_identity(&["b/y.emp".to_string(), "a/x.emp".to_string()]);
        let b = digest_scan_identity(&["a/x.emp".to_string(), "b/y.emp".to_string()]);
        assert_eq!(a, b, "the identity must not depend on walk order");
        assert_eq!(a, (2, sigil_span::read_set::crc32(b"a/x.emp\nb/y.emp\n")));
    }

    #[test]
    fn source_digest_refuses_what_its_grammar_cannot_carry() {
        type Mutation = Box<dyn Fn(&mut SourceDigest)>;
        let cases: Vec<(&str, Mutation)> = vec![
            ("a line break in a path", Box::new(|d| d.reads[0].file.path = "a\nb".into())),
            ("an absolute path", Box::new(|d| d.reads[0].file.path = "/etc/passwd".into())),
            ("a dot-dot component", Box::new(|d| d.reads[0].file.path = "games/../x".into())),
            ("an empty path", Box::new(|d| d.reads[0].file.path = String::new())),
            ("a file listed twice", Box::new(|d| d.reads.push(d.reads[0].clone()))),
            ("an external row under the aeon root", Box::new(|d| d.reads[0].origin = DigestOrigin::External)),
            ("a source row under a named root", Box::new(|d| d.reads[1].origin = DigestOrigin::Source)),
            ("an empty read set", Box::new(|d| d.reads.clear())),
            ("an extra entry with a comma", Box::new(|d| d.extra_entries = vec!["a,b".into()])),
            ("an extra entry spelled none", Box::new(|d| d.extra_entries = vec!["none".into()])),
            ("whitespace in a token", Box::new(|d| d.tree_state = "clean at capture".into())),
            ("a define that is not an identifier", Box::new(|d| d.defines.push(("A-B".into(), 1)))),
            ("a define listed twice", Box::new(|d| d.defines.push(("DEBUG".into(), 1)))),
            ("a ROM path with a line break", Box::new(|d| d.rom_output = Some(dp(DigestRoot::Aeon, "s4\n.bin")))),
        ];
        assert!(
            emit_source_digest(&sample_digest()).is_ok(),
            "the unmutated sample must render, or every refusal below is vacuous"
        );
        for (what, mutate) in cases {
            let mut d = sample_digest();
            mutate(&mut d);
            assert!(emit_source_digest(&d).is_err(), "rendered a digest with {what}");
        }
    }

    /// The parser is the grammar's executable statement: a doctored row, a reordered
    /// row or a truncated section is an error, never a digest.
    #[test]
    fn source_digest_parser_refuses_a_doctored_section() {
        let good = emit_source_digest(&sample_digest()).expect("renders");
        assert!(parse_source_digest(&good).is_ok(), "the control must parse");
        let doctored = good.replace("crc=946d49d7", "crc=946d49d8");
        assert_ne!(doctored, good, "the mutation must apply");
        let err = parse_source_digest(&doctored).expect_err("a doctored row parsed");
        assert!(err.contains("aggregate"), "{err}");
        let truncated = good.replace("DIGEST-END\n", "");
        assert!(parse_source_digest(&truncated).is_err(), "a section without DIGEST-END parsed");
        let lines: Vec<&str> = good.lines().collect();
        let (i, j) = (10, 11);
        assert!(lines[i].starts_with("DIGEST-READ ") && lines[j].starts_with("DIGEST-READ "));
        let mut swapped: Vec<&str> = lines.clone();
        swapped.swap(i, j);
        let swapped = format!("{}\n", swapped.join("\n"));
        assert!(parse_source_digest(&swapped).is_err(), "out-of-order rows parsed");
    }

    /// No keyword is a prefix of another, so `^DIGEST-<KEYWORD> ` can only ever match
    /// its own kind of line. The keyword set is read off the rendered text, not retyped.
    #[test]
    fn source_digest_keywords_are_prefix_disjoint() {
        let text = emit_source_digest(&sample_digest()).expect("renders");
        let keywords: std::collections::BTreeSet<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("DIGEST-"))
            .map(|rest| rest.split(' ').next().expect("a keyword"))
            .collect();
        let want: std::collections::BTreeSet<&str> =
            ["FORMAT", "ASSEMBLER", "SHAPE", "DEFINE", "SCAN", "READ", "AGGREGATE", "ROM", "END"].into();
        assert_eq!(keywords, want);
        for a in &keywords {
            for b in &keywords {
                assert!(a == b || !b.starts_with(a), "`{a}` is a prefix of `{b}`");
            }
        }
    }

    /// COLLISION CONTROL, and the placement it depends on. Every line the section
    /// contributes must parse as nothing under each `.lst` consumer grammar read at
    /// its owner's committed revision, transcribed below with a positive control on a
    /// real listing row, so a broken transcription cannot pass by matching nothing.
    ///
    ///  * oracle `crates/oracle-core/src/symbols.rs` (`origin/main` 9c33ca05):
    ///    `parse_body_line` (4 tokens, `(..)` then `N/HEX` then `:` then `Name:`) and the
    ///    three section-header transitions (`Symbol Table`, `Equate Table`, `Phase Table`
    ///    after `trim_start`). In the body state a non-matching line counts nothing;
    ///    after an Equate or Phase header every non-matching line counts as damage,
    ///    which is why the section goes FIRST and why no line of it may look like a
    ///    header;
    ///  * aeon `tools/s4budget.py` (`origin/master` 826159e7): `_SRC_ROW_RE`,
    ///    `_SYMTAB_HEADER_RE`, `_SYM_ROW_RE` and both trailer regexes;
    ///  * aeon `tools/scene_spans.py` `LST_HEAD_RE`, `tools/effects_gates.py`'s
    ///    `(0) `-prefix probe, `tools/deb2_probe.py` `SYMROW`, the `^\s*(\S+)\s*:\s*HEX\s+[A-Z]\s*\|`
    ///    reader of `tools/test_zx0r_resume_net.py`, and the `^EQU ` / `^PHASE` readers;
    ///  * sigil `test_support::listing_symbol_addr`: a line starting with ` NAME : `.
    #[test]
    fn source_digest_lines_never_parse_as_a_consumer_row() {
        let is_hex = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit());
        // oracle parse_body_line, transcribed.
        let oracle_body = |l: &str| -> bool {
            let tok: Vec<&str> = l.split_whitespace().collect();
            if tok.len() != 4 || tok[2] != ":" || !(tok[0].starts_with('(') && tok[0].ends_with(')')) {
                return false;
            }
            let Some((_, hex)) = tok[1].split_once('/') else { return false };
            is_hex(hex) && tok[3].strip_suffix(':').is_some_and(|n| !n.is_empty())
        };
        // oracle's section-header transitions, transcribed.
        let oracle_header = |l: &str| {
            let h = l.trim_end().trim_start();
            h.starts_with("Symbol Table") || h.starts_with("Equate Table") || h.starts_with("Phase Table")
        };
        // s4budget _SRC_ROW_RE: ^\((\d+)\)\s*(\d+)\s*/\s*([0-9A-Fa-f]+)\s*:\s+(\S.*):$
        let src_row = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(')') else { return false };
            if depth.is_empty() || !depth.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            let idx = idx.trim();
            if idx.is_empty() || !idx.bytes().all(|b| b.is_ascii_digit()) {
                return false;
            }
            let Some((hex, rest)) = rest.split_once(':') else { return false };
            is_hex(hex.trim()) && rest.starts_with(char::is_whitespace) && rest.trim().ends_with(':') && rest.trim().len() > 1
        };
        // s4budget _SYMTAB_HEADER_RE.
        let symtab_header = |l: &str| l.trim() == "Symbol Table (* = unused):";
        // s4budget _SYM_ROW_RE: ^\s*(\*?)([\w.$]+)\s*:\s*([0-9A-Fa-f]+)\s+([C\-])\s*\|\s*$
        let sym_row = |l: &str| -> bool {
            let l = l.trim_start().trim_start_matches('*');
            let Some((name, rest)) = l.split_once(':') else { return false };
            let name = name.trim_end();
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.$".contains(&b)) {
                return false;
            }
            let rest = rest.trim_start();
            let Some((hex, rest)) = rest.split_once(char::is_whitespace) else { return false };
            is_hex(hex) && matches!(rest.trim().trim_end_matches('|').trim(), "C" | "-") && rest.trim_end().ends_with('|')
        };
        // s4budget's two trailers.
        let trailer = |l: &str| {
            let t = l.trim();
            [" symbols", " unused symbols"].iter().any(|suffix| {
                t.strip_suffix(suffix).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        };
        // scene_spans LST_HEAD_RE: ^\(\d+\) \d+/([0-9A-F]+) :\s+([A-Za-z_][A-Za-z0-9_]*):\s*$
        let head_re = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix('(') else { return false };
            let Some((depth, rest)) = rest.split_once(") ") else { return false };
            let Some((idx, rest)) = rest.split_once('/') else { return false };
            let Some((hex, rest)) = rest.split_once(" :") else { return false };
            let name = rest.trim();
            !depth.is_empty()
                && depth.bytes().all(|b| b.is_ascii_digit())
                && !idx.is_empty()
                && idx.bytes().all(|b| b.is_ascii_digit())
                && is_hex(hex)
                && name.strip_suffix(':').is_some_and(|n| {
                    n.bytes().next().is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                        && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                })
        };
        // effects_gates' probe: startswith("(0) ").
        let gate_probe = |l: &str| l.starts_with("(0) ");
        // deb2_probe SYMROW: ^([ *])(\S+) : ([0-9A-F]+) C \|$
        let deb2_row = |l: &str| -> bool {
            let Some(rest) = l.strip_prefix(' ').or_else(|| l.strip_prefix('*')) else { return false };
            let Some((name, rest)) = rest.split_once(" : ") else { return false };
            let Some(hex) = rest.strip_suffix(" C |") else { return false };
            !name.is_empty() && !name.contains(char::is_whitespace) && is_hex(hex)
        };
        // test_zx0r_resume_net: ^\s*(\S+)\s*:\s*([0-9A-Fa-f]{4,8})\s+[A-Z]\s*\|
        let loose_row = |l: &str| -> bool {
            let t = l.trim_start();
            let name_end = t.find(char::is_whitespace).unwrap_or(t.len()).min(t.find(':').unwrap_or(t.len()));
            let (name, rest) = t.split_at(name_end);
            let Some(rest) = rest.trim_start().strip_prefix(':') else { return false };
            let rest = rest.trim_start();
            let hex_end = rest.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(rest.len());
            let (hex, rest) = rest.split_at(hex_end);
            let rest = rest.trim_start();
            !name.is_empty()
                && (4..=8).contains(&hex.len())
                && rest.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && rest[1..].trim_start().starts_with('|')
        };
        // listing_symbol_addr: a line starting with ` NAME : `.
        let symbol_addr = |l: &str| l.starts_with(' ') && l.contains(" : ");
        let equ_or_phase = |l: &str| l.starts_with("EQU ") || l.starts_with("PHASE");

        // A hostile path shaped like a symbol-table row, on top of the sample's own.
        let mut hostile = sample_digest();
        hostile.reads.push(DigestRead {
            crc: 1,
            size: 1,
            origin: DigestOrigin::Source,
            file: dp(DigestRoot::Aeon, "odd : 10AF2 C |"),
        });
        let section = emit_source_digest(&hostile).expect("renders");
        let listing = emit_listing(&[
            sym("OJZ_GradientStream", 0x10AF2, false, false),
            sym("OJZ_BUDGET", 0x2C, true, false),
            phased("BankHead", 0x8000, 0xE0000),
        ]);
        for line in section.lines() {
            assert!(!oracle_body(line), "oracle parse_body_line took a digest line: {line:?}");
            assert!(!oracle_header(line), "a digest line reads as an oracle section header: {line:?}");
            assert!(!src_row(line), "s4budget _SRC_ROW_RE took a digest line: {line:?}");
            assert!(!symtab_header(line), "a digest line reads as the Symbol Table header: {line:?}");
            assert!(!sym_row(line), "s4budget _SYM_ROW_RE took a digest line: {line:?}");
            assert!(!trailer(line), "a digest line reads as a symbols trailer: {line:?}");
            assert!(!head_re(line), "scene_spans LST_HEAD_RE took a digest line: {line:?}");
            assert!(!gate_probe(line), "effects_gates' probe took a digest line: {line:?}");
            assert!(!deb2_row(line), "deb2_probe SYMROW took a digest line: {line:?}");
            assert!(!loose_row(line), "the loose symbol-row reader took a digest line: {line:?}");
            assert!(!symbol_addr(line), "listing_symbol_addr would read a digest line: {line:?}");
            assert!(!equ_or_phase(line), "a digest line reads as an EQU or PHASE row: {line:?}");
        }
        // Positive controls on the SAME transcriptions, over the listing that follows.
        let lines: Vec<&str> = listing.lines().collect();
        assert!(lines.iter().any(|l| oracle_body(l)), "oracle_body transcription is dead:\n{listing}");
        assert!(lines.iter().any(|l| oracle_header(l)), "oracle_header transcription is dead");
        assert!(lines.iter().any(|l| src_row(l)), "_SRC_ROW_RE transcription is dead");
        assert!(lines.iter().any(|l| symtab_header(l)), "_SYMTAB_HEADER_RE transcription is dead");
        assert!(lines.iter().any(|l| sym_row(l)), "_SYM_ROW_RE transcription is dead");
        assert!(lines.iter().any(|l| trailer(l)), "trailer transcription is dead");
        assert!(lines.iter().any(|l| head_re(l)), "LST_HEAD_RE transcription is dead");
        assert!(lines.iter().any(|l| gate_probe(l)), "gate probe transcription is dead");
        assert!(lines.iter().any(|l| deb2_row(l)), "SYMROW transcription is dead");
        assert!(lines.iter().any(|l| loose_row(l)), "loose-row transcription is dead");
        assert!(lines.iter().any(|l| symbol_addr(l)), "listing_symbol_addr transcription is dead");
        assert!(lines.iter().any(|l| equ_or_phase(l)), "EQU/PHASE transcription is dead");

        // PLACEMENT: walked with oracle's state machine, every digest line is met in the
        // body state, the one state that counts nothing it does not recognise.
        let whole = format!("{section}{listing}");
        let mut in_body = true;
        let section_lines = section.lines().count();
        for (i, line) in whole.lines().enumerate() {
            if oracle_header(line) {
                in_body = false;
            }
            if i < section_lines {
                assert!(in_body, "digest line {i} is met after an oracle section header: {line:?}");
            }
        }
        assert!(!in_body, "the walk never reached the listing's own sections, the control is dead");
    }
}
