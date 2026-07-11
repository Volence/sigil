//! Recursive-descent parser for .emp with declaration-keyword recovery.
use crate::ast::*;
use crate::lexer::{Tok, Token};
use sigil_span::{Diagnostic, Level, Span};

/// Maximum expression nesting depth before the parser errors out instead of
/// recursing (guards against stack-overflow aborts on pathological input).
const MAX_EXPR_DEPTH: u32 = 128;

/// A recursive-descent parser over a token stream, collecting diagnostics
/// instead of failing fast.
pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
    depth: u32,
    /// When true, a `Path` immediately followed by `{` is NOT parsed as a
    /// struct literal — set while parsing `if`/`while` conditions and `for`
    /// iterables (mirrors Rust's rule), and reset inside any delimited
    /// subexpression (parens, brackets, call args, struct-literal field
    /// values) where a struct literal is unambiguous again.
    no_struct_lit: bool,
    /// When true, a bare `{expr}` appearing where no other primary makes
    /// sense (e.g. nested inside an arithmetic sub-expression: `x + {reg}`)
    /// is transparently unwrapped to the inner expression. Set for the
    /// duration of parsing an `asm { }` template body whose splices are
    /// allowed; the operand/mnemonic-position `{splice}` cases are handled
    /// separately (they consume the braces themselves and wrap the result
    /// in `Operand::Splice`/`TextOrSplice::Splice`), so this only matters
    /// for splices nested *inside* a larger expression.
    splice_ctx: bool,
    /// Statement-block nesting depth, guarded separately from the
    /// expression `depth` counter: `if`/`while`/`for` headers parse a
    /// condition EXPRESSION before their block, so sharing one counter
    /// would let the condition's guard pre-fire (consuming nothing) right
    /// before `stmt_block`'s guard, desyncing its brace recovery.
    block_depth: u32,
    /// `///` doc runs attached to items so far (S2-D11(d)); moved into
    /// [`File::docs`] when the file finishes parsing.
    docs: Vec<DocEntry>,
}

impl Parser {
    /// Build a parser over an already-lexed token stream (must end in `Eof`).
    pub fn new(toks: Vec<Token>) -> Self {
        Parser {
            toks,
            pos: 0,
            diags: Vec::new(),
            depth: 0,
            no_struct_lit: false,
            splice_ctx: false,
            block_depth: 0,
            docs: Vec::new(),
        }
    }
    /// Consume the parser, returning every diagnostic collected so far.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> { self.diags }

    // ---- cursor helpers ----
    fn peek(&self) -> &Tok { &self.toks[self.pos].tok }
    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
    }
    /// Peek `n` tokens ahead of the cursor (clamped to the trailing `Eof`),
    /// for the rare multi-token lookahead — e.g. distinguishing the
    /// `rescale<I,F>(x)` builtin from an ordinary `rescale < n` comparison.
    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }
    fn span(&self) -> Span { self.toks[self.pos].span }
    fn prev_span(&self) -> Span { self.toks[self.pos.saturating_sub(1)].span }
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 { self.pos += 1; }
        t
    }
    fn at(&self, t: &Tok) -> bool { self.peek() == t }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.at(t) { self.bump(); true } else { false }
    }
    fn expect(&mut self, t: &Tok, what: &str) -> bool {
        if self.eat(t) { return true; }
        let span = self.span();
        self.diag_at(span, format!("expected {what}, found {:?}", self.peek()));
        false
    }
    /// Is the current token this contextual keyword?
    fn at_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == kw)
    }
    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.at_kw(kw) { self.bump(); true } else { false }
    }
    fn expect_ident(&mut self, what: &str) -> String {
        if let Tok::Ident(s) = self.peek().clone() {
            self.bump();
            s
        } else {
            let span = self.span();
            self.diag_at(span, format!("expected {what}, found {:?}", self.peek()));
            String::from("<error>")
        }
    }
    /// Like [`Parser::expect_ident`], but rejects the small set of names
    /// reserved as DECLARATION names (currently just `equ` — R-T0.2: `equ` is
    /// a new top-level item keyword, so a declaration named `equ` would be
    /// silently ambiguous with the item opener). Deliberately narrow: this
    /// does NOT reserve `equ` in ordinary value/path/field-name positions
    /// (those keep calling plain `expect_ident`), matching how every other
    /// contextual keyword in this grammar (`const`, `enum`, ...) stays an
    /// unreserved name outside its own opener position.
    fn expect_decl_name(&mut self, what: &str) -> String {
        if self.at_kw("equ") {
            let span = self.span();
            self.diag_at(span, format!("`equ` is reserved and cannot be used as {what}"));
            self.bump();
            return String::from("<error>");
        }
        self.expect_ident(what)
    }
    fn skip_newlines(&mut self) {
        loop {
            if self.eat(&Tok::Newline) {
                continue;
            }
            // A `///` reaching THIS skip is not at an item position (the item
            // loops use `skip_newlines_collecting_docs` instead): warn
            // `[doc.dangling]` and keep parsing (S2-D11(d) — loud, not fatal).
            if matches!(self.peek(), Tok::DocLine(_)) {
                let sp = self.span();
                self.warn_dangling_doc(sp);
                self.bump();
                continue;
            }
            break;
        }
    }

    /// Item-position trivia skip (S2-D11(d)): newlines are skipped freely; a
    /// `///` run is COLLECTED and returned (joined with `\n`) for the caller
    /// to attach to the item that follows. Blank lines and ordinary comments
    /// between the run and the item do not detach it (they are trivia).
    fn skip_newlines_collecting_docs(&mut self) -> Option<(String, Span)> {
        let mut text = String::new();
        let mut span: Option<Span> = None;
        loop {
            if self.eat(&Tok::Newline) {
                continue;
            }
            if let Tok::DocLine(line) = self.peek().clone() {
                let sp = self.span();
                self.bump();
                if span.is_some() {
                    text.push('\n');
                }
                text.push_str(&line);
                span = Some(span.map_or(sp, |s: Span| s.merge(sp)));
                continue;
            }
            break;
        }
        span.map(|s| (text, s))
    }
    fn expect_line_end(&mut self) {
        // A trailing same-line `///` (`const A = 1 /// doc`) gets the same
        // friendly [doc.dangling] every other misplaced doc position gets,
        // not a doc-blind "expected end of line" (S2-D11(d) review m1).
        if let Tok::DocLine(_) = self.peek() {
            let sp = self.span();
            self.warn_dangling_doc(sp);
            self.bump();
        }
        if !self.at(&Tok::Eof) && !self.eat(&Tok::Newline) {
            let span = self.span();
            self.diag_at(span, "expected end of line".to_string());
            // recover: skip to next newline
            while !self.at(&Tok::Newline) && !self.at(&Tok::Eof) { self.bump(); }
        }
    }
    /// Record an error diagnostic at `span`.
    pub fn diag_at(&mut self, span: Span, message: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Error, message: message.into(), primary: span });
    }
    /// Record a warning diagnostic at `span`.
    fn warn_at(&mut self, span: Span, message: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Warning, message: message.into(), primary: span });
    }
    /// Warn `[doc.dangling]` for a `///` run that attaches to nothing.
    fn warn_dangling_doc(&mut self, span: Span) {
        self.warn_at(
            span,
            "[doc.dangling] this `///` doc comment attaches to nothing — docs go on \
             the line(s) directly above an item (use `//` for an ordinary comment)",
        );
    }

    // ---- file ----
    /// Parse a whole file: module header, module-level attributes, then items.
    pub fn file(&mut self) -> File {
        self.skip_newlines();
        let module = self.module_decl();
        // module-level attributes: `@as_compat`, `@allow(naming.pascal)`.
        // Doc runs here — above an attr, between attrs, or after the last
        // attr — are COLLECTED and carried forward to the first item (the
        // least surprising rule: attrs are metadata, docs describe the item
        // that follows them, mirroring Rust's docs-relative-to-attrs order).
        let mut attrs = Vec::new();
        let mut carried_docs: Option<(String, Span)> = None;
        loop {
            if let Some((t, sp)) = self.skip_newlines_collecting_docs() {
                carried_docs = Some(match carried_docs.take() {
                    Some((mut ct, csp)) => {
                        ct.push('\n');
                        ct.push_str(&t);
                        (ct, csp.merge(sp))
                    }
                    None => (t, sp),
                });
            }
            if !self.at(&Tok::At) { break; }
            let aspan = self.span();
            self.bump();
            let name = self.expect_ident("attribute name");
            let mut args = Vec::new();
            if self.eat(&Tok::LParen) {
                // `@attr()` is legal: empty parens mean zero args.
                if !self.at(&Tok::RParen) {
                    loop {
                        args.push(self.expr());
                        if !self.eat(&Tok::Comma) { break; }
                    }
                }
                self.expect(&Tok::RParen, "`)`");
            }
            // Span computed before the line end so the newline isn't included.
            let aspan_full = aspan.merge(self.prev_span());
            self.expect_line_end();
            attrs.push(Attr { name, args, span: aspan_full });
        }
        let mut items = Vec::new();
        loop {
            let mut docs = self.skip_newlines_collecting_docs();
            // Docs carried over the attrs region (above/between/after attrs)
            // prepend to the first item's own run.
            if let Some((ct, csp)) = carried_docs.take() {
                docs = Some(match docs {
                    Some((t, sp)) => (format!("{ct}\n{t}"), csp.merge(sp)),
                    None => (ct, csp),
                });
            }
            if self.at(&Tok::Eof) {
                if let Some((_, sp)) = docs {
                    self.warn_dangling_doc(sp);
                }
                break;
            }
            match self.item() {
                Some(item) => {
                    if let Some((text, _)) = docs {
                        self.docs.push(DocEntry { item_span: item_span(&item), text });
                    }
                    items.push(item);
                }
                None => {
                    if let Some((_, sp)) = docs {
                        self.warn_dangling_doc(sp);
                    }
                    self.recover_to_next_decl(false);
                }
            }
        }
        File { module, attrs, items, docs: std::mem::take(&mut self.docs) }
    }

    fn module_decl(&mut self) -> ModuleDecl {
        let start = self.span();
        if !self.eat_kw("module") {
            self.diag_at(start, "file must start with a `module` declaration");
            return ModuleDecl {
                path: Path { segments: vec!["<error>".into()], span: start },
                in_section: None,
                span: start,
            };
        }
        let path = self.path();
        let in_section = if self.eat_kw("in") { Some(self.expect_ident("section name")) } else { None };
        // Span computed before the line end so the newline isn't included.
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        ModuleDecl { path, in_section, span }
    }

    fn path(&mut self) -> Path {
        let start = self.span();
        let mut segments = vec![self.expect_ident("name")];
        while self.at(&Tok::Dot) && matches!(self.peek2(), Tok::Ident(_)) {
            self.bump(); // dot
            segments.push(self.expect_ident("name"));
        }
        // Merge (not a raw `start.start..prev.end`) so the span is never
        // inverted: if the opening `expect_ident` consumed nothing (the token
        // was not an ident), `prev_span().end` can precede `start.start`, and a
        // raw range would yield `end < start`. `merge` takes min-start/max-end,
        // keeping the normal case exact while clamping the error case to a valid
        // span.
        Path { segments, span: start.merge(self.prev_span()) }
    }

    /// Dispatch on the leading contextual keyword. Returns None on an
    /// unrecognized opener (caller recovers).
    fn item(&mut self) -> Option<Item> {
        let public = self.eat_kw("pub");
        if public && (self.at_kw("use") || self.at_kw("section")) {
            let sp = self.prev_span();
            self.diag_at(sp, "`pub` is not valid on this declaration");
        }
        if self.at_kw("use") { return Some(Item::Use(self.use_decl())); }
        if self.at_kw("const") { return Some(Item::Const(self.const_decl(public))); }
        if self.at_kw("equ") { return Some(Item::Equ(self.equ_decl(public))); }
        if self.at_kw("enum") { return Some(Item::Enum(self.enum_decl(public, false))); }
        if self.at_kw("bitfield") { return Some(Item::Bitfield(self.bitfield_decl(public))); }
        if self.at_kw("struct") { return Some(Item::Struct(self.struct_decl(public))); }
        if self.at_kw("offsets") { return Some(Item::Offsets(self.offsets_decl(public))); }
        if self.at_kw("table") { return Some(Item::Table(Box::new(self.table_decl(public)))); }
        if self.at_kw("dispatch") { return Some(Item::Dispatch(self.dispatch_decl(public))); }
        if self.at_kw("vars") { return Some(Item::Vars(self.vars_decl(public))); }
        if self.at_kw("data") { return Some(Item::Data(self.data_decl(public))); }
        if self.at_kw("proc") { return Some(Item::Proc(self.proc_decl(public))); }
        if self.at_kw("script") { return Some(Item::Script(self.script_decl(public))); }
        if self.at_kw("newtype") { return Some(Item::Newtype(self.newtype_decl(public))); }
        // `align N` (D2.29, §4.8) — contextual item opener per the `equ`
        // precedent. The `= `guard keeps a hypothetical assignment-shaped line
        // out (mirroring patch/bind's rule); `align` stays an ordinary
        // identifier in every expression position (item() only runs here).
        if self.at_kw("align") && !matches!(self.peek2(), Tok::Eq) {
            if public {
                let sp = self.prev_span();
                self.diag_at(sp, "`pub` is not valid on this declaration");
            }
            let start = self.span();
            self.bump(); // `align`
            let n = self.expr();
            let span = start.merge(self.prev_span());
            self.expect_line_end();
            return Some(Item::Align(AlignDecl { n, span }));
        }
        if self.at_kw("comptime") {
            // `comptime enum Name { ... }` — a payload-carrying enum, distinct
            // from `comptime fn`. Peek past `comptime` before committing to
            // either reading. `comptime test "name" { … }` (S2-D11(a)) is the
            // third form.
            if matches!(self.peek2(), Tok::Ident(s) if s == "enum") {
                self.bump(); // `comptime`
                return Some(Item::Enum(self.enum_decl(public, true)));
            }
            if matches!(self.peek2(), Tok::Ident(s) if s == "test") {
                if public {
                    let sp = self.span();
                    self.diag_at(sp, "`pub` is not valid on this declaration");
                }
                return Some(Item::ComptimeTest(self.comptime_test_decl()));
            }
            return Some(Item::ComptimeFn(self.comptime_fn_decl(public)));
        }
        // Item-position guard: `ensure(...)` / `ensure_fatal(...)`. Contextual
        // opener (§10 policy) — only fires when the keyword is immediately
        // followed by `(`, so `ensure` stays usable as an ordinary name (D5.1).
        if (self.at_kw("ensure") || self.at_kw("ensure_fatal"))
            && matches!(self.peek2(), Tok::LParen)
        {
            if public {
                let sp = self.prev_span();
                self.diag_at(sp, "`pub` is not valid on this declaration");
            }
            let start = self.span();
            let fatal = self.at_kw("ensure_fatal");
            let call = self.expr(); // parses the whole `ensure(...)` call
            let span = start.merge(self.prev_span());
            self.expect_line_end();
            return Some(Item::Ensure(EnsureDecl { fatal, call, span }));
        }
        if self.at_kw("section") { return Some(Item::Section(self.section_decl())); }
        let span = self.span();
        self.diag_at(span, format!("expected a declaration, found {:?}", self.peek()));
        None
    }

    /// Error recovery: skip until a token that can start a declaration.
    ///
    /// `in_block` must be true when the caller is inside a brace-delimited
    /// item list (i.e. a `section { ... }` body) and false at true top
    /// level (the whole-file item list, which has no enclosing `{`). This
    /// matters for a stray `}` seen at brace-depth 0 (relative to where
    /// recovery started): inside a block that `}` is the block's OWN
    /// closer, which recovery must leave unconsumed so the caller's own
    /// `at(&Tok::RBrace)` check (and subsequent `expect`) sees it — bumping
    /// it here would desync recovery, letting it eat the section's closing
    /// brace and swallow following top-level items as bogus section
    /// members. At true top level there is no enclosing `{` to protect, so
    /// a stray `}` is just garbage to skip past (the old behavior),
    /// otherwise recovery would loop forever re-diagnosing the same token.
    fn recover_to_next_decl(&mut self, in_block: bool) {
        const OPENERS: [&str; 20] = ["use", "const", "equ", "enum", "bitfield", "struct",
                                     "vars", "data", "proc", "script", "comptime", "section", "pub",
                                     "newtype", "offsets", "table", "dispatch", "ensure", "ensure_fatal",
                                     "align"];
        let mut depth = 0i32;
        loop {
            match self.peek() {
                Tok::Eof => return,
                Tok::RBrace if in_block && depth == 0 => return,
                Tok::LBrace => { depth += 1; self.bump(); }
                Tok::RBrace => { depth -= 1; self.bump(); }
                // A `///` line is a safe sync point (it only occurs at line
                // starts): stop so the caller's next collecting-skip attaches
                // it to the recovered-to item instead of silently eating it
                // (S2-D11(d) review m2).
                Tok::DocLine(_) if depth <= 0 => return,
                Tok::Ident(s) if depth <= 0 && OPENERS.contains(&s.as_str()) => {
                    // `ensure`/`ensure_fatal` are CONTEXTUAL openers — only a real
                    // item when followed by `(`. A bare occurrence is not an item
                    // (`item()` won't consume it), so stopping here would spin the
                    // recovery loop (recovery returns without consuming, `item()`
                    // re-fails on the same token). Skip past a non-guard occurrence.
                    let guard_kw = s == "ensure" || s == "ensure_fatal";
                    // `align` is likewise contextual: `align = 5` is NOT an
                    // item (item() skips it when `=` follows), so recovery
                    // must not stop there either — same spin hazard as the
                    // guards (Item-3 review C1: stopping without consuming
                    // made `align = 5` an infinite parse loop).
                    let align_non_item =
                        s == "align" && matches!(self.peek_at(1), Tok::Eq);
                    if (guard_kw && !matches!(self.peek_at(1), Tok::LParen)) || align_non_item {
                        self.bump();
                    } else {
                        return;
                    }
                }
                _ => { self.bump(); }
            }
        }
    }

    fn use_decl(&mut self) -> UseDecl {
        let start = self.span();
        self.bump(); // `use`
        // parse dotted path, stopping before `.{` and `.*`
        let pstart = self.span();
        let mut segments = vec![self.expect_ident("module path")];
        let mut base_end = self.prev_span().end;
        let mut names = UseNames::Whole;
        loop {
            if !self.at(&Tok::Dot) { break; }
            match self.peek2().clone() {
                Tok::Ident(_) => {
                    self.bump();
                    segments.push(self.expect_ident("name"));
                    base_end = self.prev_span().end;
                }
                Tok::Star => { self.bump(); self.bump(); names = UseNames::Glob; break; }
                Tok::LBrace => {
                    self.bump(); self.bump(); // `.` `{`
                    let mut list = Vec::new();
                    loop {
                        self.skip_newlines();
                        list.push(self.expect_ident("imported name"));
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.skip_newlines();
                    self.expect(&Tok::RBrace, "`}`");
                    names = UseNames::List(list);
                    break;
                }
                _ => break,
            }
        }
        let base = Path {
            segments,
            span: Span { source: pstart.source, start: pstart.start, end: base_end },
        };
        // Span computed before the line end so the newline isn't included.
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        UseDecl { base, names, span }
    }

    /// A full type, including an optional trailing `where LO..HI` refinement.
    fn ty(&mut self) -> Type {
        let base = self.ty_base();
        self.maybe_where_refine(base)
    }

    /// A type with NO `where`-refinement handling — the depth-guarded base
    /// case reused directly by [`Parser::newtype_decl`], which parses its
    /// `where` clause itself into a dedicated AST field rather than a nested
    /// [`Type::Refined`].
    fn ty_base(&mut self) -> Type {
        // Same depth guard as the expression grammar: `*`/`[`/`(` type arms
        // all recurse, so a `****...u8` bomb would otherwise abort the process.
        if self.depth >= MAX_EXPR_DEPTH {
            let span = self.span();
            self.diag_at(span, "type nesting too deep (max 128)");
            return Type::Named(Path { segments: vec!["<error>".into()], span });
        }
        self.depth += 1;
        let r = self.ty_inner();
        self.depth -= 1;
        r
    }

    /// If a contextual `where` follows, consume it and wrap `base` as a
    /// [`Type::Refined`]; otherwise return `base` unchanged. A malformed
    /// range after `where` is diagnosed (via [`Parser::try_where_range`])
    /// and `base` is returned unwrapped.
    fn maybe_where_refine(&mut self, base: Type) -> Type {
        if self.eat_kw("where") {
            match self.try_where_range() {
                Some((lo, hi)) => Type::Refined(Box::new(base), lo, hi),
                None => base,
            }
        } else {
            base
        }
    }

    /// Parse the `LO..HI` range following an already-consumed contextual
    /// `where`. Diagnoses (without panicking) if what follows isn't a range
    /// expression, and returns `None` in that case.
    fn try_where_range(&mut self) -> Option<(Expr, Expr)> {
        let range = self.expr();
        if let Expr::Range { lo, hi, .. } = range {
            Some((*lo, *hi))
        } else {
            self.diag_at(expr_span(&range), "expected a range `LO..HI` after `where`");
            None
        }
    }

    fn ty_inner(&mut self) -> Type {
        match self.peek().clone() {
            Tok::Star => { self.bump(); Type::Ptr(Box::new(self.ty())) }
            Tok::LBracket => {
                self.bump();
                let elem = self.ty();
                self.expect(&Tok::Semi, "`;` in array type");
                let len = self.expr();
                self.expect(&Tok::RBracket, "`]`");
                Type::Array(Box::new(elem), len)
            }
            Tok::LParen => {
                self.bump();
                let mut elems = vec![self.ty()];
                while self.eat(&Tok::Comma) { elems.push(self.ty()); }
                self.expect(&Tok::RParen, "`)`");
                Type::Tuple(elems)
            }
            // `fixed<I, F>` — a fixed-point type; only recognized when `fixed`
            // is immediately followed by `<` (otherwise `fixed` is an ordinary,
            // unreserved type/value name).
            Tok::Ident(s) if s == "fixed" && matches!(self.peek2(), Tok::Lt) => {
                self.bump(); // `fixed`
                self.bump(); // `<`
                let i = self.expect_u32_lit("an integer bit width");
                self.expect(&Tok::Comma, "`,`");
                let f = self.expect_u32_lit("a fraction bit width");
                self.expect(&Tok::Gt, "`>`");
                Type::Fixed { i, f }
            }
            _ => Type::Named(self.path()),
        }
    }

    /// A `u32` integer literal, e.g. the `8` in `fixed<8, 8>` or `rescale<8, 8>`.
    /// On anything else, diagnoses and returns `0` without consuming a token
    /// that an enclosing frame (`,`/`>`/etc.) needs to see.
    fn expect_u32_lit(&mut self, what: &str) -> u32 {
        match self.peek().clone() {
            Tok::Int(v) => {
                let sp = self.span();
                self.bump();
                match u32::try_from(v) {
                    Ok(n) => n,
                    Err(_) => {
                        self.diag_at(sp, format!("{what} out of range (must fit in u32)"));
                        0
                    }
                }
            }
            _ => {
                let sp = self.span();
                self.diag_at(sp, format!("expected {what}"));
                if !matches!(self.peek(),
                    Tok::RBrace | Tok::RParen | Tok::RBracket | Tok::Newline
                    | Tok::Comma | Tok::Gt | Tok::Eof) {
                    self.bump();
                }
                0
            }
        }
    }

    fn const_decl(&mut self, public: bool) -> ConstDecl {
        let start = self.span();
        self.bump(); // `const`
        let name = self.expect_decl_name("a constant name");
        let ty = if self.eat(&Tok::Colon) { Some(self.ty()) } else { None };
        self.expect(&Tok::Eq, "`=`");
        let value = self.expr();
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        ConstDecl { public, name, ty, value, span }
    }

    /// An `equ NAME = expr` declaration (R-T0.2): grammar mirrors
    /// [`Parser::const_decl`] minus the optional type annotation — an equ's
    /// value is always an untyped comptime int or link-time expression.
    fn equ_decl(&mut self, public: bool) -> EquDecl {
        let start = self.span();
        self.bump(); // `equ`
        let name = self.expect_decl_name("an equ name");
        self.expect(&Tok::Eq, "`=`");
        let value = self.expr();
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        EquDecl { is_pub: public, name, value, span }
    }

    /// Parse an `enum Name: repr { variants... }` (`comptime == false`, repr
    /// required) or `comptime enum Name [: repr] { variants... }`
    /// (`comptime == true`, repr optional, variants may carry payload types).
    /// The `enum`/`comptime` leading keyword(s) must already be consumed by
    /// the caller for the comptime case (see `item`'s dispatch); this always
    /// consumes the `enum` keyword itself.
    fn enum_decl(&mut self, public: bool, comptime: bool) -> EnumDecl {
        let start = self.span();
        self.bump(); // `enum`
        let name = self.expect_ident("enum name");
        let repr = if comptime {
            if self.eat(&Tok::Colon) { Some(self.ty()) } else { None }
        } else {
            self.expect(&Tok::Colon, "`:` (enums require a repr, e.g. `: u8`)");
            Some(self.ty())
        };
        self.expect(&Tok::LBrace, "`{`");
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let vspan = self.span();
            let vname = self.expect_ident("variant name");
            let value = if self.eat(&Tok::Eq) { Some(self.expr()) } else { None };
            let payload = if self.eat(&Tok::LParen) {
                let mut tys = Vec::new();
                if !self.at(&Tok::RParen) {
                    loop {
                        tys.push(self.ty());
                        if !self.eat(&Tok::Comma) { break; }
                        if self.at(&Tok::RParen) { break; } // trailing comma
                    }
                }
                self.expect(&Tok::RParen, "`)`");
                tys
            } else { Vec::new() };
            variants.push(EnumVariant { name: vname, value, payload, span: vspan });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        EnumDecl { public, comptime, name, repr, variants, span: start.merge(self.prev_span()) }
    }

    /// Parse a `newtype Name = Underlying [where LO..HI]` declaration.
    fn newtype_decl(&mut self, public: bool) -> NewtypeDecl {
        let start = self.span();
        self.bump(); // `newtype`
        let name = self.expect_ident("newtype name");
        self.expect(&Tok::Eq, "`=`");
        let underlying = self.ty_base();
        let refine = if self.eat_kw("where") { self.try_where_range() } else { None };
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        NewtypeDecl { public, name, underlying, refine, span }
    }

    /// Parse a `bitfield Name: repr { field: bits [@ anchor], ... }` declaration.
    fn bitfield_decl(&mut self, public: bool) -> BitfieldDecl {
        let start = self.span();
        self.bump(); // `bitfield`
        let name = self.expect_ident("bitfield name");
        self.expect(&Tok::Colon, "`:` (bitfields require a repr)");
        let repr = self.ty();
        self.expect(&Tok::LBrace, "`{`");
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let fspan = self.span();
            let fname = self.expect_ident("field name");
            self.expect(&Tok::Colon, "`:`");
            let bits = match self.peek().clone() {
                // The lexer never produces negative `Int`s (`-` is a separate
                // token), so try_from + a zero check cover all range errors.
                Tok::Int(v) => {
                    let sp = self.span();
                    self.bump();
                    match u32::try_from(v) {
                        Ok(b) if b > 0 => b,
                        _ => {
                            self.diag_at(sp, "bit width must be 1..=4294967295");
                            1
                        }
                    }
                }
                _ => {
                    let sp = self.span();
                    self.diag_at(sp, "expected a bit width");
                    if !matches!(self.peek(), Tok::RBrace | Tok::Newline | Tok::Comma) {
                        self.bump();
                    }
                    1
                }
            };
            let anchor = if self.eat(&Tok::At) {
                match self.peek().clone() {
                    Tok::Int(v) => {
                        let sp = self.span();
                        self.bump();
                        match u32::try_from(v) {
                            Ok(a) => Some(a),
                            Err(_) => {
                                self.diag_at(sp, "bit anchor out of range");
                                None
                            }
                        }
                    }
                    _ => {
                        let sp = self.span();
                        self.diag_at(sp, "expected a bit anchor after `@`");
                        if !matches!(self.peek(), Tok::RBrace | Tok::Newline | Tok::Comma) {
                            self.bump();
                        }
                        None
                    }
                }
            } else { None };
            fields.push(BitfieldField { name: fname, bits, anchor, span: fspan });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        BitfieldDecl { public, name, repr, fields, span: start.merge(self.prev_span()) }
    }

    /// Parse a `struct Name [(size: expr)] { field: ty [@ offset] [= default], ... }` declaration.
    fn struct_decl(&mut self, public: bool) -> StructDecl {
        let start = self.span();
        self.bump(); // `struct`
        let name = self.expect_ident("struct name");
        let size = if self.eat(&Tok::LParen) {
            if !self.eat_kw("size") {
                let sp = self.span();
                self.diag_at(sp, "expected `size:` in struct attribute list");
            }
            self.expect(&Tok::Colon, "`:`");
            let e = self.expr();
            self.expect(&Tok::RParen, "`)`");
            Some(e)
        } else { None };
        self.expect(&Tok::LBrace, "`{`");
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let fspan = self.span();
            let fname = self.expect_ident("field name");
            self.expect(&Tok::Colon, "`:`");
            let fty = self.ty();
            let offset = if self.eat(&Tok::At) { Some(self.expr()) } else { None };
            let default = if self.eat(&Tok::Eq) { Some(self.expr()) } else { None };
            fields.push(StructField { name: fname, ty: fty, offset, default, span: fspan });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        StructDecl { public, name, size, fields, span: start.merge(self.prev_span()) }
    }

    /// Parse an `offsets Name { Variant: target, ... }` declaration.
    fn offsets_decl(&mut self, public: bool) -> OffsetsDecl {
        let start = self.span();
        self.bump(); // `offsets`
        let name = self.expect_ident("offsets name");
        self.expect(&Tok::LBrace, "`{`");
        let mut members = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let mspan = self.span();
            let mname = self.expect_ident("offset entry name");
            self.expect(&Tok::Colon, "`:`");
            // §4.7 mixed form: speculatively parse a TYPE — if `=` follows,
            // this is an INLINE member (`Name: [u8; 4] = [...]`, the exact
            // `data`-item shape); otherwise rewind (dropping any speculative
            // diagnostics) and parse the by-reference target expression.
            let save_pos = self.pos;
            let save_diags = self.diags.len();
            let speculative = self.ty();
            let target = if self.at(&Tok::Eq) {
                self.bump(); // `=`
                OffsetsTarget::Inline(speculative, self.expr())
            } else {
                self.pos = save_pos;
                self.diags.truncate(save_diags);
                OffsetsTarget::Ref(self.expr())
            };
            members.push(OffsetsMember { name: mname, target, span: mspan });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        OffsetsDecl { public, name, members, span: start.merge(self.prev_span()) }
    }

    /// Parse a `table Name [: [RowType]] [(attrs)] { rows }` declaration
    /// (Plan 7 T2-d). A contextual item opener (the `offsets`/`align`
    /// precedent). The attribute knobs and row grammar are described on
    /// [`TableDecl`]; the ratified design lives in
    /// `2026-07-11-counted-sparse-collection-design.md`.
    fn table_decl(&mut self, public: bool) -> TableDecl {
        let start = self.span();
        self.bump(); // `table`
        let name = self.expect_ident("table name");
        // Optional `: [RowType]` element-type annotation.
        let row_type = if self.eat(&Tok::Colon) {
            self.expect(&Tok::LBracket, "`[` in table row type");
            let ty = self.ty();
            self.expect(&Tok::RBracket, "`]`");
            Some(ty)
        } else {
            None
        };
        let attrs = self.table_attrs();
        self.expect(&Tok::LBrace, "`{`");
        let mut rows = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) {
                break;
            }
            rows.push(self.table_row());
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                break;
            }
            self.skip_newlines();
            if self.at(&Tok::RBrace) {
                break; // trailing comma
            }
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        TableDecl { public, name, row_type, attrs, rows, span: start.merge(self.prev_span()) }
    }

    /// Parse a `table`'s optional `(attr, ...)` knob list. Returns a default
    /// (all-`None`) [`TableAttrs`] when no `(` follows.
    fn table_attrs(&mut self) -> TableAttrs {
        let start = self.span();
        let mut attrs = TableAttrs {
            cell: None,
            key: None,
            hole: None,
            header: None,
            sentinel: None,
            item_align: None,
            body: None,
            span: start,
        };
        if !self.eat(&Tok::LParen) {
            return attrs;
        }
        loop {
            self.skip_newlines();
            if self.at(&Tok::RParen) {
                break;
            }
            let key = self.expect_ident("table attribute name");
            self.expect(&Tok::Colon, "`:`");
            match key.as_str() {
                "cell" => attrs.cell = Some(self.ty()),
                "key" => attrs.key = Some(self.table_key_domain()),
                "hole" => attrs.hole = Some(self.expr()),
                "header" => {
                    // `Type(Expr)` — a count word over the reserved `count`.
                    let ty = self.ty();
                    self.expect(&Tok::LParen, "`(` in table header");
                    let e = self.expr();
                    self.expect(&Tok::RParen, "`)`");
                    attrs.header = Some((ty, e));
                }
                "sentinel" => attrs.sentinel = Some(self.expr()),
                "item_align" => attrs.item_align = Some(self.expr()),
                "body" => {
                    let sp = self.span();
                    let word = self.expect_ident("`before` or `after`");
                    attrs.body = match word.as_str() {
                        "before" => Some(BodyPlacement::Before),
                        "after" => Some(BodyPlacement::After),
                        _ => {
                            self.diag_at(sp, "table `body:` must be `before` or `after`");
                            None
                        }
                    };
                }
                other => {
                    let sp = self.prev_span();
                    self.diag_at(
                        sp,
                        format!(
                            "unknown table attribute `{other}` (expected cell/key/hole/\
                             header/sentinel/item_align/body)"
                        ),
                    );
                    // Best-effort skip of the value expression.
                    let _ = self.expr();
                }
            }
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&Tok::RParen, "`)`");
        attrs.span = start.merge(self.prev_span());
        attrs
    }

    /// Parse a `key:` domain — an inclusive integer range `lo..=hi` (v1). The
    /// bounds are parsed at a binding power ABOVE the `..` range operator (5),
    /// so the pratt parser does not swallow `..` into `lo` (it would otherwise
    /// read `lo..` as a half-open [`Expr::Range`] and choke on the `=`).
    fn table_key_domain(&mut self) -> KeyDomain {
        let lo = self.expr_bp(5);
        self.expect(&Tok::DotDot, "`..=` in table key range");
        if !self.eat(&Tok::Eq) {
            let sp = self.span();
            self.diag_at(sp, "table `key:` range must be inclusive (`lo..=hi`)");
        }
        let hi = self.expr_bp(5);
        KeyDomain::Range(lo, hi)
    }

    /// Parse one `table` row: an optional `Key:` prefix, then either a
    /// comma-separated `Label = DataExpr` part list (blob mode) or a single
    /// record literal (typed mode). Within a parts row, a `,` continues the
    /// row only when a `Label =` part follows (else it separates rows) — this
    /// keeps keyed multi-part rows unambiguous (design §3).
    fn table_row(&mut self) -> TableRow {
        let start = self.span();
        // Optional key: speculatively parse `<expr> :`; rewind if no colon.
        let save_pos = self.pos;
        let save_diags = self.diags.len();
        let key_candidate = self.expr_no_struct_lit();
        let key = if self.at(&Tok::Colon) {
            self.bump(); // `:`
            Some(key_candidate)
        } else {
            self.pos = save_pos;
            self.diags.truncate(save_diags);
            None
        };
        let body = if matches!(self.peek(), Tok::Ident(_)) && matches!(self.peek2(), Tok::Eq) {
            // Parts (blob) mode.
            let mut parts = Vec::new();
            loop {
                let pspan = self.span();
                let label = self.expect_ident("table part label");
                self.expect(&Tok::Eq, "`=`");
                let value = self.expr();
                parts.push(TablePart { label, value, span: pspan.merge(self.prev_span()) });
                // Continue as another PART only when `, Label =` follows;
                // otherwise the comma separates ROWS (handled by the caller).
                if self.at(&Tok::Comma)
                    && matches!(self.peek_at(1), Tok::Ident(_))
                    && matches!(self.peek_at(2), Tok::Eq)
                {
                    self.bump(); // `,`
                    continue;
                }
                break;
            }
            TableRowBody::Parts(parts)
        } else {
            // Typed (record) mode.
            TableRowBody::Record(self.expr())
        };
        TableRow { key, body, span: start.merge(self.prev_span()) }
    }

    /// Parse a `dispatch Name (encoding: E) { Member: target, ... }`
    /// declaration (D6.B1). The `(encoding: E)` attribute is REQUIRED (no
    /// default — D6.B2); the member grammar mirrors [`Self::offsets_decl`].
    /// `Member: { … }` (inline body, 9a — D9.1) parses the same statement
    /// grammar as a `proc` body (labels, instruction lines, comptime calls).
    fn dispatch_decl(&mut self, public: bool) -> DispatchDecl {
        let start = self.span();
        self.bump(); // `dispatch`
        let name = self.expect_ident("dispatch name");
        let encoding = self.dispatch_encoding_attr();
        self.expect(&Tok::LBrace, "`{`");
        let mut members = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let mspan = self.span();
            let mname = self.expect_ident("dispatch member name");
            self.expect(&Tok::Colon, "`:`");
            if self.at(&Tok::LBrace) {
                // 9a (D9.1): `Member: { … }` — an inline body, sugar for an
                // anonymous per-member proc. Same statement grammar as a
                // `proc` body (labels, instruction lines, comptime calls).
                self.bump(); // `{`
                let body = self.asm_body(/* splices_allowed = */ false);
                self.expect(&Tok::RBrace, "`}`");
                members.push(DispatchMember {
                    name: mname,
                    target: DispatchTarget::Body(body),
                    span: mspan.merge(self.prev_span()),
                });
            } else {
                let target = self.expr();
                members.push(DispatchMember {
                    name: mname,
                    target: DispatchTarget::Label(target),
                    span: mspan,
                });
            }
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        DispatchDecl { public, name, encoding, members, span: start.merge(self.prev_span()) }
    }

    /// Parse the required `(encoding: E)` attribute of a `dispatch` or
    /// `script` decl (construct-neutral wording — both require it).
    /// Missing parens/key, or an unknown encoding ident, each produce one
    /// error mentioning the valid encodings; parsing continues with a
    /// best-guess default (`word_offsets`) so the member list still parses.
    fn dispatch_encoding_attr(&mut self) -> DispatchEncoding {
        if !self.eat(&Tok::LParen) {
            let sp = self.span();
            self.diag_at(sp, "this declaration requires an `(encoding: word_offsets | long_ptrs)` attribute");
            return DispatchEncoding::WordOffsets;
        }
        if !self.eat_kw("encoding") {
            let sp = self.span();
            self.diag_at(sp, "expected `encoding:` in the attribute list");
        }
        self.expect(&Tok::Colon, "`:`");
        let esp = self.span();
        let ident = self.expect_ident("dispatch encoding");
        let encoding = match ident.as_str() {
            "word_offsets" => DispatchEncoding::WordOffsets,
            "long_ptrs" => DispatchEncoding::LongPtrs,
            other => {
                self.diag_at(
                    esp,
                    format!(
                        "unknown dispatch encoding `{other}` — valid encodings are \
                         `word_offsets` and `long_ptrs`"
                    ),
                );
                DispatchEncoding::WordOffsets
            }
        };
        self.expect(&Tok::RParen, "`)`");
        encoding
    }

    /// Parse a `vars region { .. }` (region form) or `vars name: region { .. }`
    /// (overlay form) declaration.
    fn vars_decl(&mut self, public: bool) -> VarsDecl {
        let start = self.span();
        self.bump(); // `vars`
        let first = self.expect_ident("region or overlay name");
        let (name, region) = if self.eat(&Tok::Colon) {
            let mut region = vec![self.expect_ident("overlay region (e.g. sst_custom)")];
            while self.at(&Tok::Dot) && matches!(self.peek2(), Tok::Ident(_)) {
                self.bump(); // .
                region.push(self.expect_ident("name"));
            }
            (Some(first), region)
        } else {
            (None, vec![first])
        };
        self.expect(&Tok::LBrace, "`{`");
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let fspan = self.span();
            let fname = self.expect_ident("field name");
            self.expect(&Tok::Colon, "`:`");
            let fty = self.ty();
            let align = if self.at(&Tok::At) && matches!(self.peek2(), Tok::Ident(s) if s == "align") {
                self.bump(); // @
                self.bump(); // align
                self.expect(&Tok::LParen, "`(`");
                let e = self.expr();
                self.expect(&Tok::RParen, "`)`");
                Some(e)
            } else { None };
            fields.push(VarsField { name: fname, ty: fty, align, span: fspan });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        VarsDecl {
            public,
            name,
            region,
            fields,
            resolved_window: None,
            span: start.merge(self.prev_span()),
        }
    }

    /// Parse a `data NAME[: Ty] = value` declaration.
    fn data_decl(&mut self, public: bool) -> DataDecl {
        let start = self.span();
        self.bump(); // `data`
        let name = self.expect_ident("data item name");
        // Optional `(max_size: expr)` capacity attribute (D5.4). Mirrors
        // `struct Name (size: expr)`; sits BEFORE the `: Ty` annotation so the
        // grammar is `data Name (max_size: E) [: Ty] = value`.
        let max_size = if self.eat(&Tok::LParen) {
            if !self.eat_kw("max_size") {
                let sp = self.span();
                self.diag_at(sp, "expected `max_size:` in data attribute list");
            }
            self.expect(&Tok::Colon, "`:`");
            let e = self.expr();
            self.expect(&Tok::RParen, "`)`");
            Some(e)
        } else { None };
        let ty = if self.eat(&Tok::Colon) { Some(self.ty()) } else { None };
        self.expect(&Tok::Eq, "`=`");
        let value = self.expr();
        let span = start.merge(self.prev_span());
        self.expect_line_end();
        // `type_only` is a resolver-set cross-module injection flag (D-PP.5),
        // never a source construct — the parser always emits a real data item.
        DataDecl { public, name, ty, max_size, value, span, type_only: false }
    }

    /// Parse a parenthesized `(name: Ty, ...)` typed-register parameter list
    /// (trailing comma tolerated). Shared by `proc` and `script` — R9b.1 pins
    /// script params as "exactly as `proc`", so there is ONE grammar.
    fn param_list(&mut self) -> Vec<(String, Type, Span)> {
        self.expect(&Tok::LParen, "`(`");
        let mut params = Vec::new();
        if !self.at(&Tok::RParen) {
            loop {
                let pspan = self.span();
                let pname = self.expect_ident("parameter (register) name");
                self.expect(&Tok::Colon, "`:`");
                let pty = self.ty();
                params.push((pname, pty, pspan));
                if !self.eat(&Tok::Comma) { break; }
                if self.at(&Tok::RParen) { break; } // trailing comma
            }
        }
        self.expect(&Tok::RParen, "`)`");
        params
    }

    /// Parse the register reglist inside a `clobbers(...)` / `out(...)` /
    /// `preserves(...)` clause, consuming the closing `)`. The opening `(` must
    /// already have been eaten. As of C1 item 2 all three share ONE grammar: a
    /// list of segments, each a single register or an inclusive `lo-hi` range
    /// (`d0-d3/a1`, the movem-reglist form), separated by `/` OR `,` (comma
    /// singles stay legal for `clobbers`/`out` back-compat; `sr` composes as a
    /// single). The empty form (`()`) yields an empty vec — the explicit
    /// "touches nothing" / "returns nothing" contract, distinct from no
    /// declaration at all. Register-name VALIDITY + range expansion is a
    /// lowering-time check (`[proc.clobber-invalid]` / `[proc.out-invalid]` /
    /// `[proc.preserves-invalid]`), not parse-time, so this accepts any
    /// identifier as a segment endpoint.
    fn reg_list(&mut self) -> Vec<(String, Option<String>)> {
        let mut list = Vec::new();
        if !self.at(&Tok::RParen) {
            loop {
                let lo = self.expect_ident("register");
                let hi = if self.eat(&Tok::Minus) {
                    Some(self.expect_ident("range-end register"))
                } else {
                    None
                };
                list.push((lo, hi));
                // Segments separate on `/` (movem) OR `,` (clobbers/out singles).
                if !self.eat(&Tok::Slash) && !self.eat(&Tok::Comma) {
                    break;
                }
                if self.at(&Tok::RParen) { break; } // trailing comma
            }
        }
        self.expect(&Tok::RParen, "`)`");
        list
    }

    /// Parse a `proc name(params...) [clobbers(...)] [out(...)] [preserves(...)]
    /// [falls_into name] { body }` declaration. Clause order is free.
    fn proc_decl(&mut self, public: bool) -> ProcDecl {
        let start = self.span();
        self.bump(); // `proc`
        let name = self.expect_ident("proc name");
        let params = self.param_list();
        let mut clobbers = None;
        let mut preserves = Vec::new();
        let mut out = None;
        let mut falls_into = None;
        loop {
            if self.eat_kw("clobbers") {
                self.expect(&Tok::LParen, "`(`");
                // `clobbers()` — the empty form is the explicit "touches
                // nothing" contract (distinct from no declaration at all). C1
                // item 2: accepts the movem-reglist grammar (`d0-d3/a1`).
                clobbers = Some(self.reg_list());
            } else if self.eat_kw("out") {
                // `out(d0-d1/a1)` — the third partition member (S2-D6e), C1
                // item 2: same reglist grammar as `clobbers`/`preserves`.
                // `out()` empty is the explicit "returns nothing" contract.
                self.expect(&Tok::LParen, "`(`");
                out = Some(self.reg_list());
            } else if self.eat_kw("preserves") {
                // `preserves(d0-d1/a0)` — the movem-style reglist (S2-D6b
                // syntactic slice), now the shared `clobbers`/`out` grammar.
                self.expect(&Tok::LParen, "`(`");
                preserves = self.reg_list();
            } else if self.eat_kw("falls_into") {
                falls_into = Some(self.expect_ident("target proc name"));
            } else {
                break;
            }
        }
        self.expect(&Tok::LBrace, "`{`");
        let body = self.asm_body(/* splices_allowed = */ false);
        self.expect(&Tok::RBrace, "`}`");
        ProcDecl { public, name, params, clobbers, preserves, out, falls_into, body, span: start.merge(self.prev_span()) }
    }

    /// Parse a `script name(params) (encoding: E) [shows label] { body }`
    /// declaration (Plan 7 #9b — R9b.1). Params parse exactly as `proc`
    /// params; the `(encoding: E)` attribute is REQUIRED (dispatch's rule —
    /// the hidden table is engine contract); `shows` declares the per-frame
    /// epilogue (D9.6), overridable per yield site.
    fn script_decl(&mut self, public: bool) -> ScriptDecl {
        let start = self.span();
        self.bump(); // `script`
        let name = self.expect_ident("script name");
        let params = self.param_list();
        let encoding = self.dispatch_encoding_attr();
        let epilogue = if self.eat_kw("shows") { Some(self.script_label()) } else { None };
        self.expect(&Tok::LBrace, "`{`");
        let body = self.script_body();
        self.expect(&Tok::RBrace, "`}`");
        ScriptDecl { public, name, params, encoding, epilogue, body, span: start.merge(self.prev_span()) }
    }

    /// Parse an epilogue label reference: `Draw_Sprite` or `.rearm`.
    fn script_label(&mut self) -> ScriptLabel {
        let start = self.span();
        let local = self.eat(&Tok::Dot);
        let name = self.expect_ident("epilogue label");
        ScriptLabel { name, local, span: start.merge(self.prev_span()) }
    }

    /// Body of a `script` (R9b.1): the `proc` statement grammar plus two
    /// contextual statement openers — `loop { … }` and `yield [label]`.
    /// Neither collides with real code: no 68k/Z80 mnemonic is named `loop`
    /// or `yield`, and a comptime CALL is only recognized with an adjacent
    /// `(` (so a fn named `yield` is unreachable here anyway — fine).
    ///
    /// The `loop` arm recurses, so it is guarded by the same `block_depth`
    /// counter/ceiling as [`Parser::stmt_block`] (and for the same reason:
    /// nested `loop {` is shaped like a paren-bomb, and an unbounded descent
    /// would abort the process on adversarial input instead of diagnosing).
    fn script_body(&mut self) -> Vec<ScriptStmt> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) || self.at(&Tok::Eof) { break; }
            if self.at_kw("loop") {
                let start = self.span();
                self.bump(); // `loop`
                if self.block_depth >= MAX_EXPR_DEPTH {
                    let span = self.span();
                    self.diag_at(span, "block nesting too deep (max 128)");
                    // Do not recurse into `script_body` again; consume the
                    // whole `{ ... }` so this loop provably makes progress.
                    self.skip_unparsed_block();
                    continue;
                }
                self.block_depth += 1;
                self.expect(&Tok::LBrace, "`{`");
                let body = self.script_body();
                self.expect(&Tok::RBrace, "`}`");
                self.block_depth -= 1;
                out.push(ScriptStmt::Loop { body, span: start.merge(self.prev_span()) });
                continue;
            }
            if self.at_kw("wait_frames") {
                // D2.30(c): `wait_frames #N, <slot>` — script-body-only
                // contextual statement, beside `loop`/`yield`.
                let start = self.span();
                self.bump(); // `wait_frames`
                if !self.eat(&Tok::Hash) {
                    let sp = self.span();
                    self.diag_at(sp, "`wait_frames` takes an immediate: `wait_frames #N, <slot>`");
                }
                let n = self.expr();
                self.expect(&Tok::Comma, "`,`");
                let slot = self.operand(false);
                let span = start.merge(self.prev_span());
                self.expect_line_end_or_rbrace();
                out.push(ScriptStmt::WaitFrames { n, slot, span });
                continue;
            }
            if self.at_kw("yield") {
                let start = self.span();
                self.bump(); // `yield`
                // D2.30: `yield shows <label>` overrides the per-frame
                // epilogue; `yield .label` names the RESUME point; the old
                // bare-label epilogue spelling is retired (it misread as a
                // resume target — the audit's finding) with a teaching error.
                let mut epilogue = None;
                let mut resume = None;
                if self.eat_kw("shows") {
                    epilogue = Some(self.script_label());
                } else if self.at(&Tok::Dot) {
                    resume = Some(self.script_label());
                } else if matches!(self.peek(), Tok::Ident(_)) {
                    let sp = self.span();
                    self.diag_at(
                        sp,
                        "`yield <label>` was retired (D2.30) — write `yield shows <label>` to override the per-frame epilogue, or `yield .label` to name where the next frame resumes",
                    );
                    self.bump(); // consume the label so the line recovers
                }
                // Same line-end rule as instruction lines: a `}` may close
                // the body on the same line (`{ yield }` parses like `{ nop }`).
                self.expect_line_end_or_rbrace();
                out.push(ScriptStmt::Yield { epilogue, resume, span: start.merge(self.prev_span()) });
                continue;
            }
            // Everything else is one ordinary proc-body statement (labels,
            // instructions, statement-position comptime calls) — the exact
            // grammar `asm_body` uses, factored into `asm_stmt`. Scripts never
            // allow `{expr}` splices (procs don't either), so pass `false`.
            if let Some(stmt) = self.asm_stmt(/* splices_allowed = */ false) {
                out.push(ScriptStmt::Asm(stmt));
            }
        }
        out
    }

    /// Body of a `proc` or an `asm { }` template. Statements are
    /// newline-separated: labels (`.name:` / `export .name:`), instruction
    /// lines, and statement-position comptime calls.
    fn asm_body(&mut self, splices_allowed: bool) -> Vec<AsmStmt> {
        // Enable transparent `{expr}` unwrapping for splices nested inside a
        // larger expression (`x + {reg}`) for the duration of this body;
        // restored on exit so an enclosing (non-splice) context is unaffected.
        let saved_splice_ctx = self.splice_ctx;
        self.splice_ctx = splices_allowed;
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) || self.at(&Tok::Eof) { break; }
            if let Some(stmt) = self.asm_stmt(splices_allowed) {
                out.push(stmt);
            }
        }
        self.splice_ctx = saved_splice_ctx;
        out
    }

    /// Parse ONE proc-body statement: a label (`.name:` / `export .name:`), a
    /// statement-position comptime call (`ident(...)`), or an instruction line.
    /// Factored out of [`Parser::asm_body`] so `script` bodies reuse the exact
    /// same statement grammar (Plan 7 #9b — R9b.1). The caller owns the loop,
    /// the newline-skipping, and the `RBrace`/`Eof` termination check; this fn
    /// assumes it is positioned on a real statement start. `splices_allowed`
    /// threads through to [`Parser::instr_line`] identically to before. Returns
    /// `None` only when the underlying statement parse elects not to produce a
    /// statement (it never currently does — the shape mirrors `instr_line`'s
    /// no-consume-on-error convention so callers stay in step regardless).
    fn asm_stmt(&mut self, splices_allowed: bool) -> Option<AsmStmt> {
        let start = self.span();
        // `export .name:` / `.name:`
        let export = self.at_kw("export") && matches!(self.peek2(), Tok::Dot);
        if export { self.bump(); }
        if self.at(&Tok::Dot) && matches!(self.peek2(), Tok::Ident(_)) {
            self.bump();
            let name = self.expect_ident("label name");
            self.expect(&Tok::Colon, "`:` after label");
            return Some(AsmStmt::Label { name, export, span: start.merge(self.prev_span()) });
        }
        if export {
            // `export` not followed by a dot-label: diagnose and fall
            // through to normal statement parsing so we still make
            // progress (don't consume a closer/newline).
            let sp = self.span();
            self.diag_at(sp, "expected `.label` after `export`");
        }
        // `todo!` / `unreachable!` statement traps (S2-D11(e)). The `!` must
        // be DIRECTLY adjacent (mirroring the call-adjacency rule below), so
        // an expression-position `!x` operand can never be shadowed. No 68k/
        // Z80 mnemonic is named `todo`/`unreachable`, so mnemonics stay
        // unshadowed too (tenet 3).
        if let Tok::Ident(w) = self.peek().clone() {
            if matches!(w.as_str(), "todo" | "unreachable")
                && matches!(self.peek2(), Tok::Bang)
                && self.adjacent_to_next()
            {
                let kind =
                    if w == "todo" { TrapKind::Todo } else { TrapKind::Unreachable };
                self.bump(); // ident
                self.bump(); // `!`
                let message = if self.at(&Tok::LParen) {
                    self.bump();
                    // `todo!()` (empty parens, the Rust-muscle-memory
                    // spelling) is the same as bare `todo!`.
                    let m = if self.at(&Tok::RParen) {
                        None
                    } else if let Tok::Str(s) = self.peek().clone() {
                        self.bump();
                        Some(s)
                    } else {
                        let sp = self.span();
                        self.diag_at(
                            sp,
                            format!("expected a string literal message in `{w}!(...)`"),
                        );
                        None
                    };
                    self.expect(&Tok::RParen, "`)`");
                    m
                } else {
                    None
                };
                let span = start.merge(self.prev_span());
                self.expect_line_end_or_rbrace();
                return Some(AsmStmt::Trap { kind, message, span });
            }
        }
        // statement-position comptime `if` (tranche 5, H1): `if` is a
        // reserved statement-leading keyword (S2-D1) and no 68k/Z80 mnemonic
        // is named `if`, so this can never shadow an instruction line — and
        // S2-D15's control-flow-sugar "no" means it can never AMBIGUATE with
        // a future runtime `if` either.
        if self.at_kw("if") {
            return Some(self.asm_if(splices_allowed));
        }
        // statement-position local typed-register binding (Spec 2, C2):
        // `let <reg>: <Type>` (NO `=`). `let` is reserved statement-leading
        // (S2-D1) and no 68k/Z80 mnemonic is named `let`, so this can never
        // shadow an instruction line. The register-token class + the absence of
        // `=` make it syntactically disjoint from any comptime value binding
        // (`let name = expr`); register-name VALIDITY is a lowering-time check
        // (mirroring params / the clobber-lint model), so a bad name still parses
        // here and is diagnosed with its type in hand.
        if self.at_kw("let") {
            return Some(self.asm_let());
        }
        // statement-position comptime call: `ident(` where the `(` is
        // DIRECTLY adjacent to the identifier (no space) — `bne (a0)`
        // is an instruction with a parenthesized operand, not a call.
        if matches!(self.peek(), Tok::Ident(_)) && matches!(self.peek2(), Tok::LParen)
            && self.adjacent_to_next() {
            let e = self.expr();
            self.expect_line_end();
            return Some(AsmStmt::Call(e));
        }
        Some(AsmStmt::Instr(self.instr_line(splices_allowed)))
    }

    /// Parse a `let <reg>: <Type>` local typed-register binding (Spec 2, C2).
    /// The leading `let` is already confirmed by the caller. Reuses the SAME
    /// type grammar as a proc param (`self.ty()`), so `<Type>` accepts everything
    /// a param does (`*Struct` pointer views, value newtypes). No initializer —
    /// the register already holds its value; `let` only types it. The register
    /// name is captured as a bare identifier (validity checked at lowering, like
    /// params), and a line end / `}` must follow like every other statement form.
    fn asm_let(&mut self) -> AsmStmt {
        let start = self.span();
        self.bump(); // `let`
        let reg = self.expect_ident("register name after `let`");
        self.expect(&Tok::Colon, "`:` after register in `let <reg>: <Type>`");
        let ty = self.ty();
        let span = start.merge(self.prev_span());
        self.expect_line_end_or_rbrace();
        AsmStmt::Let { reg, ty, span }
    }

    /// `if cond { asm... } [else if ... | else { asm... }]` at proc/asm
    /// statement position. The branches parse with the SAME statement grammar
    /// as the enclosing body ([`Parser::asm_body`]), so labels, nested `if`s,
    /// traps, and comptime calls all work inside. The condition parses with
    /// struct literals disabled (`if_expr`'s rule): `if X { ... }` must not
    /// read `X {` as a struct literal. Newlines before `else` (and between
    /// `else` and its `{`) are skipped — `} else {`, a next-line `else`, and
    /// an `else` with its brace on the next line all spell the same statement
    /// (the caller's loop skips newlines anyway, so the lookahead costs
    /// nothing). A statement after a closing brace on the SAME line is
    /// diagnosed like every other statement form's trailing junk.
    ///
    /// Recursion (nested `if`, `else if`, and branch bodies) is guarded by
    /// the same `block_depth` counter/ceiling as `stmt_block`/`loop` (and for
    /// the same reason: adversarial `if 1 { if 1 { …` is shaped like a
    /// paren-bomb, and an unbounded descent would abort the process instead
    /// of diagnosing). This also bounds the eval side's `lower_asm_stmt`/
    /// label-collection recursions, since AST depth is parser-bounded.
    fn asm_if(&mut self, splices_allowed: bool) -> AsmStmt {
        let start = self.span();
        self.bump(); // `if`
        let cond = self.expr_no_struct_lit();
        if self.block_depth >= MAX_EXPR_DEPTH {
            let span = self.span();
            self.diag_at(span, "block nesting too deep (max 128)");
            // Do not recurse; consume the whole `{ ... }` so the caller's
            // loop provably makes progress.
            self.skip_unparsed_block();
            return AsmStmt::If {
                cond,
                then: Vec::new(),
                els: None,
                span: start.merge(self.prev_span()),
            };
        }
        self.block_depth += 1;
        self.expect(&Tok::LBrace, "`{`");
        let then = self.asm_body(splices_allowed);
        self.expect(&Tok::RBrace, "`}`");
        self.trailing_junk_after_brace();
        self.skip_newlines();
        let els = if self.eat_kw("else") {
            self.skip_newlines();
            if self.at_kw("if") {
                Some(vec![self.asm_if(splices_allowed)])
            } else {
                self.expect(&Tok::LBrace, "`{`");
                let body = self.asm_body(splices_allowed);
                self.expect(&Tok::RBrace, "`}`");
                self.trailing_junk_after_brace();
                Some(body)
            }
        } else {
            None
        };
        self.block_depth -= 1;
        AsmStmt::If { cond, then, els, span: start.merge(self.prev_span()) }
    }

    /// Diagnose a statement continuing on the same line after a closing `}`
    /// (`if X { nop } moveq #0, d0`) — every other statement form requires a
    /// line end, so `if` does too. `else` is legal there (the caller reads
    /// it); a newline / `}` / EOF ends the statement normally. Consumes
    /// nothing: the caller's newline-skipping loop recovers on its own.
    fn trailing_junk_after_brace(&mut self) {
        if !(self.at(&Tok::Newline)
            || self.at(&Tok::RBrace)
            || self.at(&Tok::Eof)
            || self.at_kw("else"))
        {
            let sp = self.span();
            self.diag_at(sp, "expected end of line after `}`");
        }
    }

    /// Is the token immediately after the current one lexically adjacent to
    /// it (no whitespace between them)? Used to disambiguate `spawn(...)`
    /// (a call) from `bne (a0)` (an instruction with a parenthesized operand).
    fn adjacent_to_next(&self) -> bool {
        let cur_end = self.span().end;
        let next_start = self.toks[(self.pos + 1).min(self.toks.len() - 1)].span.start;
        cur_end == next_start
    }

    /// A single machine-instruction line: mnemonic (possibly spliced),
    /// optional size suffix, then comma-separated operands.
    fn instr_line(&mut self, splices_allowed: bool) -> InstrLine {
        let start = self.span();
        // mnemonic: (ident|{splice})+ — parts concatenate: `b{cc}`
        let mut mnemonic = Vec::new();
        loop {
            match self.peek().clone() {
                Tok::Ident(_) => mnemonic.push(TextOrSplice::Text(self.expect_ident("mnemonic"))),
                Tok::LBrace if splices_allowed => {
                    self.bump();
                    mnemonic.push(TextOrSplice::Splice(self.expr()));
                    self.expect(&Tok::RBrace, "`}`");
                }
                _ => break,
            }
            // continue only if the next part is DIRECTLY adjacent (no gap):
            let prev_end = self.prev_span().end;
            if self.span().start != prev_end { break; }
            if !(matches!(self.peek(), Tok::Ident(_)) || (splices_allowed && self.at(&Tok::LBrace))) { break; }
        }
        if mnemonic.is_empty() {
            let sp = self.span();
            self.diag_at(sp, format!("expected an instruction, found {:?}", self.peek()));
            // do not consume closers/newline per convention:
            if !matches!(self.peek(), Tok::RBrace | Tok::RParen | Tok::RBracket | Tok::Newline) {
                self.bump();
            }
        }
        // optional size suffix: `.b` / `.{w}` — must be DIRECTLY adjacent to
        // the mnemonic (`subq.b`), else a leading dot belongs to the first
        // operand instead (`bne     .draw` — a local-label reference, not
        // a size suffix separated from `bne` by whitespace).
        //
        // The literal set is b/w/l/s — `.s` is the short-branch pin (`bra.s`),
        // deliberately wider than operand index sizes (b/w/l only). Any other
        // adjacent post-dot ident is NOT a size: consume nothing and fall
        // through, so a typo like `bne.draw` parses `.draw` as the
        // branch-target operand (identical to `bne .draw`) instead of
        // silently swallowing the target as a bogus size.
        let size = if self.at(&Tok::Dot) && self.span().start == self.prev_span().end {
            match self.peek2().clone() {
                Tok::Ident(s) if matches!(s.as_str(), "b" | "w" | "l" | "s") => {
                    self.bump(); // dot
                    self.bump(); // size ident
                    Some(TextOrSplice::Text(s))
                }
                Tok::LBrace if splices_allowed => {
                    self.bump(); // dot
                    self.bump(); // lbrace
                    let e = self.expr();
                    self.expect(&Tok::RBrace, "`}`");
                    Some(TextOrSplice::Splice(e))
                }
                _ => None,
            }
        } else { None };
        // operands
        let mut operands = Vec::new();
        if !self.at(&Tok::Newline) && !self.at(&Tok::RBrace) && !self.at(&Tok::Eof) {
            loop {
                operands.push(self.operand(splices_allowed));
                if !self.eat(&Tok::Comma) { break; }
            }
        }
        // Span computed before the line end so the newline isn't included.
        let span = start.merge(self.prev_span());
        self.expect_line_end_or_rbrace();
        InstrLine { mnemonic, size, operands, span }
    }

    /// Like [`Parser::expect_line_end`], but also accepts a directly-following
    /// `}`/EOF (the last instruction in a body needn't end in a newline).
    fn expect_line_end_or_rbrace(&mut self) {
        if self.at(&Tok::RBrace) || self.at(&Tok::Eof) { return; }
        self.expect_line_end();
    }

    /// A single instruction operand: immediate, pre-dec/post-inc, indirect,
    /// displacement-indirect, a bare/local-label path, or a `{splice}`.
    fn operand(&mut self, splices_allowed: bool) -> Operand {
        let start = self.span();
        match self.peek().clone() {
            Tok::Hash => {
                self.bump();
                // `#{expr}` — a spliced immediate; `{` is not otherwise a
                // valid expression opener, so `expr()` alone can't parse it.
                if splices_allowed && self.at(&Tok::LBrace) {
                    self.bump();
                    let e = self.expr();
                    self.expect(&Tok::RBrace, "`}`");
                    Operand::Imm(e)
                } else {
                    Operand::Imm(self.expr())
                }
            }
            // Pre-decrement is deliberately lenient about whitespace:
            // `- (a7)` is accepted as `-(a7)` because at operand position
            // there is no other sane reading of Minus followed by LParen
            // (a parenthesized negation would be written `#-(...)`).
            Tok::Minus if matches!(self.peek2(), Tok::LParen) => {
                self.bump();
                let inner = self.paren_operand(splices_allowed, start);
                Operand::PreDec(Box::new(inner))
            }
            Tok::LParen => {
                let inner = self.paren_operand(splices_allowed, start);
                if self.eat(&Tok::Plus) {
                    Operand::PostInc(Box::new(inner))
                } else {
                    inner
                }
            }
            Tok::LBrace if splices_allowed => {
                self.bump();
                let e = self.expr();
                self.expect(&Tok::RBrace, "`}`");
                // F1 (tranche 7): a `{splice}` immediately followed by `(` is a
                // spliced DISPLACEMENT — `{off}(aN)` / `{off}({reg})` — not a
                // whole-operand splice. Continue into the same displacement-
                // indirect grammar a literal displacement uses, with the spliced
                // expression as the displacement. Eval range-checks it identically
                // to a literal disp (i16 for `d16(An)`) and diagnoses a non-int
                // splice with `[asm.splice-kind]`.
                if self.at(&Tok::LParen) {
                    let inner = self.paren_operand(splices_allowed, self.span());
                    let span = start.merge(self.prev_span());
                    return Operand::DispInd {
                        disp: e,
                        inner: Box::new(inner),
                        disp_spliced: true,
                        span,
                    };
                }
                Operand::Splice(e)
            }
            Tok::Dot if matches!(self.peek2(), Tok::Ident(_)) => {
                // local-label operand: `.draw` — represent as a Path ".draw"
                self.bump();
                let name = self.expect_ident("label");
                let span = start.merge(self.prev_span());
                let mut expr = Expr::Path(Path { segments: vec![format!(".{name}")], span });
                // The expression grammar can't OPEN with a `.local` atom, but
                // once one is parsed it takes binary continuations like any
                // other term — `.cc_table-4` (the tranche-9 dispatch-anchor
                // idiom). `(` is not a binary operator, so the displacement
                // check below is unaffected.
                expr = self.binary_continue(expr, 1);
                // A local label immediately followed by `(` is a DISPLACEMENT
                // (tranche 8 — `pea .raise(pc)`, the pc-relative self-address
                // idiom AS spells `pea *(pc)`): continue into the same
                // displacement-indirect grammar every other disp form uses.
                if self.at(&Tok::LParen) {
                    let inner = self.paren_operand(splices_allowed, self.span());
                    let span = start.merge(self.prev_span());
                    return Operand::DispInd {
                        disp: expr,
                        inner: Box::new(inner),
                        disp_spliced: false,
                        span,
                    };
                }
                Operand::Plain { expr, size: None, span }
            }
            _ => {
                let e = self.expr();
                // NOTE: `timer(a0)` where `timer` is a bare ident arrives as
                // Expr::Call via primary_expr — normalize an all-positional
                // call to displacement-indexed addressing. The `!args.is_empty()`
                // guard is deliberate: an empty call `f()` stays an Expr::Call
                // for the semantic layer — zero-part indirection isn't a 68k
                // addressing mode.
                if let Expr::Call { callee, args, span } = &e {
                    if !args.is_empty() && args.iter().all(|a| a.name.is_none()) {
                        // Mirror paren_operand: recover per-part index sizes
                        // (`d0.w` arrived as Path["d0","w"] inside the arg)...
                        let parts = args.iter()
                            .map(|a| split_size_suffix(a.value.clone()))
                            .collect::<Vec<_>>();
                        // ...and consume a trailing size after the `)` so
                        // `timer(a0).l` owns its `.l` (leaving it behind would
                        // corrupt the rest of the operand list).
                        let size = self.trailing_size(splices_allowed);
                        let ispan = span.merge(self.prev_span());
                        return Operand::DispInd {
                            disp: Expr::Path(callee.clone()),
                            inner: Box::new(Operand::Ind { parts, size, span: ispan }),
                            disp_spliced: false,
                            span: ispan,
                        };
                    }
                }
                // displacement form: expr immediately followed by `(...)` — `4(a0, d0.w)`
                if self.at(&Tok::LParen) {
                    let inner = self.paren_operand(splices_allowed, self.span());
                    let span = start.merge(self.prev_span());
                    return Operand::DispInd {
                        disp: e,
                        inner: Box::new(inner),
                        disp_spliced: false,
                        span,
                    };
                }
                let size = self.trailing_size(splices_allowed);
                let span = start.merge(self.prev_span());
                Operand::Plain { expr: e, size, span }
            }
        }
    }

    /// `( part {, part} )` with optional per-part `.w`/`.l` and optional
    /// trailing size after the close paren: `(VDP_Ctrl).l`
    ///
    /// Index-size caveat: in `(a0, d0.w)` the `.w` is consumed by `path()`
    /// as a path segment (`d0.w` → Path["d0","w"]). `split_size_suffix`
    /// undoes that: a multi-segment path whose LAST segment is exactly
    /// "b"/"w"/"l" splits into (path-without-last, Some(size)). A struct
    /// field genuinely named `w` in operand position would need parens —
    /// accepted, and lint-flagged later (this is the AS size-suffix bug
    /// class, resolved by rule instead of corruption).
    fn paren_operand(&mut self, splices_allowed: bool, start: Span) -> Operand {
        self.expect(&Tok::LParen, "`(`");
        let mut parts = Vec::new();
        loop {
            let e = if self.at(&Tok::LBrace) && splices_allowed {
                self.bump();
                let e = self.expr();
                self.expect(&Tok::RBrace, "`}`");
                e
            } else {
                self.expr()
            };
            let (e, mut psize) = split_size_suffix(e);
            if psize.is_none() { psize = self.trailing_size(splices_allowed); }
            parts.push((e, psize));
            if !self.eat(&Tok::Comma) { break; }
            if self.at(&Tok::RParen) { break; } // trailing comma
        }
        self.expect(&Tok::RParen, "`)`");
        let size = self.trailing_size(splices_allowed);
        Operand::Ind { parts, size, span: start.merge(self.prev_span()) }
    }

    /// `.w` / `.l` / `.{expr}` if directly adjacent, else None.
    fn trailing_size(&mut self, splices_allowed: bool) -> Option<TextOrSplice> {
        if !self.at(&Tok::Dot) { return None; }
        let adjacent = self.span().start == self.prev_span().end;
        if !adjacent { return None; }
        match self.peek2().clone() {
            Tok::Ident(s) if s == "b" || s == "w" || s == "l" => {
                self.bump(); self.bump();
                Some(TextOrSplice::Text(s))
            }
            Tok::LBrace if splices_allowed => {
                self.bump(); self.bump();
                let e = self.expr();
                self.expect(&Tok::RBrace, "`}`");
                Some(TextOrSplice::Splice(e))
            }
            _ => None,
        }
    }

    /// Parse a `comptime fn name(params...) [-> ret] { body... }` declaration.
    /// `comptime test "name" [(expect_error: "[diag.id]")] { … }` (S2-D11(a)).
    fn comptime_test_decl(&mut self) -> ComptimeTestDecl {
        let start = self.span();
        self.bump(); // `comptime`
        self.bump(); // `test`
        let name = if let Tok::Str(s) = self.peek().clone() {
            self.bump();
            s
        } else {
            let sp = self.span();
            self.diag_at(sp, "a `comptime test` takes a string name: `comptime test \"name\" { ... }`");
            String::from("<unnamed>")
        };
        let expect_error = if self.eat(&Tok::LParen) {
            let kw = self.expect_ident("attribute name");
            if kw != "expect_error" {
                let sp = self.prev_span();
                self.diag_at(sp, "the only `comptime test` attribute is `expect_error`");
            }
            self.expect(&Tok::Colon, "`:`");
            let id = if let Tok::Str(s) = self.peek().clone() {
                self.bump();
                Some(s)
            } else {
                let sp = self.span();
                self.diag_at(sp, "`expect_error` takes a diagnostic-id string, e.g. \"[struct.missing-field]\"");
                None
            };
            self.expect(&Tok::RParen, "`)`");
            id
        } else {
            None
        };
        let body = self.stmt_block();
        ComptimeTestDecl { name, expect_error, body, span: start.merge(self.prev_span()) }
    }

    fn comptime_fn_decl(&mut self, public: bool) -> ComptimeFnDecl {
        let start = self.span();
        self.bump(); // `comptime`
        if !self.eat_kw("fn") {
            let sp = self.span();
            self.diag_at(sp, "expected `fn` after `comptime` at item position");
        }
        let name = self.expect_ident("function name");
        self.expect(&Tok::LParen, "`(`");
        let mut params = Vec::new();
        if !self.at(&Tok::RParen) {
            loop {
                let pspan = self.span();
                let pname = self.expect_ident("parameter name");
                self.expect(&Tok::Colon, "`:`");
                let pty = self.ty();
                params.push((pname, pty, pspan));
                if !self.eat(&Tok::Comma) { break; }
                if self.at(&Tok::RParen) { break; } // trailing comma
            }
        }
        self.expect(&Tok::RParen, "`)`");
        let ret = if self.eat(&Tok::Arrow) { Some(self.ty()) } else { None };
        let body = self.stmt_block();
        ComptimeFnDecl { public, name, params, ret, body, span: start.merge(self.prev_span()) }
    }

    /// `{ stmt* }` — a comptime statement block (newline-separated statements).
    ///
    /// Guarded by its own `block_depth` counter (NOT the expression `depth`
    /// counter): block nesting is shaped like a paren-bomb (`if x {` nested
    /// hundreds of times), so an unbounded recursive descent here would
    /// abort the process on adversarial input instead of producing a
    /// diagnostic. The counter must be separate because `if`/`while`/`for`
    /// parse a condition expression right before this block — a shared
    /// counter lets the condition guard pre-fire without consuming anything,
    /// leaving this guard's recovery staring at a non-`{` token.
    fn stmt_block(&mut self) -> Vec<Stmt> {
        if self.block_depth >= MAX_EXPR_DEPTH {
            let span = self.span();
            self.diag_at(span, "block nesting too deep (max 128)");
            self.skip_unparsed_block();
            return Vec::new();
        }
        self.block_depth += 1;
        self.expect(&Tok::LBrace, "`{`");
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) || self.at(&Tok::Eof) { break; }
            out.push(self.stmt());
        }
        self.expect(&Tok::RBrace, "`}`");
        self.block_depth -= 1;
        out
    }

    /// Depth-guard recovery for a `{ ... }` block the parser refuses to
    /// recurse into (shared by [`Parser::stmt_block`] and `script_body`'s
    /// `loop` arm). Recovery must CONSUME so the caller provably makes
    /// progress: scan forward to the block's `{` (whatever garbage precedes
    /// it), then balance-scan to its matching `}` and consume that too. Only
    /// Eof stops us early.
    fn skip_unparsed_block(&mut self) {
        while !self.at(&Tok::LBrace) && !self.at(&Tok::Eof) { self.bump(); }
        if self.eat(&Tok::LBrace) {
            let mut d = 1i32;
            while d > 0 && !self.at(&Tok::Eof) {
                match self.peek() {
                    Tok::LBrace => d += 1,
                    Tok::RBrace => d -= 1,
                    _ => {}
                }
                self.bump();
            }
        }
    }

    /// A single comptime statement inside a `comptime fn` body (or a nested
    /// `comptime block { }`/`if`/`while`/`for` body).
    fn stmt(&mut self) -> Stmt {
        let start = self.span();
        // Reserved statement keywords cannot be assignment targets:
        // `let = 5` gets ONE diagnostic and a skip to line end, not a
        // cascade of expected-ident/expected-expression errors. (`patch`
        // and `bind` are deliberately absent — they stay contextual and
        // fall through to the assignment path below.)
        if let Tok::Ident(kw) = self.peek() {
            if matches!(kw.as_str(), "let" | "return" | "if" | "else" | "for" | "while" | "comptime" | "match")
                && matches!(self.peek2(), Tok::Eq)
            {
                let kw = kw.clone();
                self.diag_at(start, format!("`{kw}` is reserved and cannot be assigned"));
                self.bump(); // the keyword
                // skip the rest of the line; never consume the newline/closer
                while !matches!(self.peek(), Tok::Newline | Tok::RBrace | Tok::Eof) {
                    self.bump();
                }
                return Stmt::Expr(Expr::Path(Path { segments: vec!["<error>".into()], span: start }));
            }
        }
        // `else` with no preceding `if`: diagnose once, discard its block
        // (if any) so recovery stays clean, and make progress.
        if self.at_kw("else") {
            self.diag_at(start, "`else` without a matching `if`");
            self.bump(); // `else`
            if self.at(&Tok::LBrace) {
                let _ = self.stmt_block();
            }
            return Stmt::Expr(Expr::Path(Path { segments: vec!["<error>".into()], span: start }));
        }
        if self.eat_kw("let") {
            if self.at(&Tok::LParen) {
                self.bump();
                let mut names = vec![self.expect_ident("name")];
                while self.eat(&Tok::Comma) { names.push(self.expect_ident("name")); }
                self.expect(&Tok::RParen, "`)`");
                self.expect(&Tok::Eq, "`=`");
                let value = self.expr();
                let span = start.merge(self.prev_span());
                self.expect_line_end_or_rbrace();
                return Stmt::LetTuple { names, value, span };
            }
            let name = self.expect_ident("name");
            self.expect(&Tok::Eq, "`=`");
            let value = self.expr();
            let span = start.merge(self.prev_span());
            self.expect_line_end_or_rbrace();
            return Stmt::Let { name, value, span };
        }
        if self.at_kw("comptime") {
            match self.peek2().clone() {
                Tok::Ident(s) if s == "block" => {
                    self.bump(); self.bump();
                    let body = self.stmt_block();
                    return Stmt::ComptimeBlock { body, span: start.merge(self.prev_span()) };
                }
                Tok::Ident(s) if s == "var" => {
                    self.bump(); self.bump();
                    let name = self.expect_ident("variable name");
                    let ty = if self.eat(&Tok::Colon) { Some(self.ty()) } else { None };
                    self.expect(&Tok::Eq, "`=`");
                    let value = self.expr();
                    let span = start.merge(self.prev_span());
                    self.expect_line_end_or_rbrace();
                    return Stmt::Var { name, ty, value, span };
                }
                // `comptime for` / `comptime if` / `comptime while` — the marker
                // is a no-op inside an already-comptime context; consume and fall through.
                Tok::Ident(s) if s == "for" || s == "if" || s == "while" => { self.bump(); }
                _ => {
                    let sp = self.span();
                    self.diag_at(sp, "expected `block`, `var`, `for`, `if`, or `while` after `comptime`");
                    // do not consume closers/newline:
                    if !matches!(self.peek(), Tok::RBrace | Tok::RParen | Tok::RBracket | Tok::Newline) {
                        self.bump();
                    }
                }
            }
        }
        if self.eat_kw("return") {
            let value = if self.at(&Tok::Newline) || self.at(&Tok::RBrace) { None } else { Some(self.expr()) };
            let span = start.merge(self.prev_span());
            self.expect_line_end_or_rbrace();
            return Stmt::Return { value, span };
        }
        // `patch`/`bind` are CONTEXTUAL: `patch = 5` is an assignment to a
        // variable named `patch` (falls through to the assignment path),
        // not a malformed patch declaration — hence the `!Eq` lookahead.
        if self.at_kw("patch") && !matches!(self.peek2(), Tok::Eq) {
            self.bump(); // `patch`
            let name = self.expect_ident("patch slot name");
            self.expect(&Tok::Colon, "`:`");
            let ty = self.ty();
            let span = start.merge(self.prev_span());
            self.expect_line_end_or_rbrace();
            return Stmt::Patch { name, ty, span };
        }
        if self.at_kw("bind") && !matches!(self.peek2(), Tok::Eq) {
            self.bump(); // `bind`
            let name = self.expect_ident("patch slot name");
            self.expect(&Tok::Eq, "`=`");
            let value = self.expr();
            let span = start.merge(self.prev_span());
            self.expect_line_end_or_rbrace();
            return Stmt::Bind { name, value, span };
        }
        if self.at_kw("while") {
            self.bump();
            let cond = self.expr_no_struct_lit();
            let body = self.stmt_block();
            return Stmt::While { cond, body, span: start.merge(self.prev_span()) };
        }
        if self.at_kw("if") {
            let e = self.if_expr();
            return Stmt::If(e);
        }
        if self.at_kw("for") {
            let e = self.for_expr();
            return Stmt::For(e);
        }
        // assignment: path = expr (lookahead with rewind)
        if matches!(self.peek(), Tok::Ident(_)) {
            let save = self.pos;
            let path = self.path();
            if self.eat(&Tok::Eq) {
                let value = self.expr();
                let span = start.merge(self.prev_span());
                self.expect_line_end_or_rbrace();
                return Stmt::Assign { target: path, value, span };
            }
            self.pos = save;
        }
        let e = self.expr();
        self.expect_line_end_or_rbrace();
        Stmt::Expr(e)
    }

    /// Parse an expression with struct literals disabled (if/while/for headers).
    fn expr_no_struct_lit(&mut self) -> Expr {
        let saved = self.no_struct_lit;
        self.no_struct_lit = true;
        let e = self.expr();
        self.no_struct_lit = saved;
        e
    }

    /// Parse a `section name [(attr: value, ...)] [{ items... }]` declaration.
    ///
    /// The block form nests ordinary items (`section z80_driver (cpu: z80) {
    /// data X = ... }`); the bare form (no `{`) declares a section with no
    /// inline items and ends at the line end, same as other single-line decls.
    fn section_decl(&mut self) -> SectionDecl {
        let start = self.span();
        self.bump(); // `section`
        let name = self.expect_ident("section name");
        let mut attrs = Vec::new();
        if self.eat(&Tok::LParen) {
            if !self.at(&Tok::RParen) {
                loop {
                    let aname = self.expect_ident("attribute name");
                    self.expect(&Tok::Colon, "`:`");
                    attrs.push((aname, self.expr()));
                    if !self.eat(&Tok::Comma) { break; }
                    if self.at(&Tok::RParen) { break; } // trailing comma
                }
            }
            self.expect(&Tok::RParen, "`)`");
        }
        let mut items = Vec::new();
        if self.eat(&Tok::LBrace) {
            loop {
                let docs = self.skip_newlines_collecting_docs();
                if self.at(&Tok::RBrace) || self.at(&Tok::Eof) {
                    if let Some((_, sp)) = docs {
                        self.warn_dangling_doc(sp);
                    }
                    break;
                }
                let parsed = self.item();
                if let (Some(item), Some((text, _))) = (&parsed, &docs) {
                    // A doc on the REJECTED nested-section item below would be
                    // a phantom entry (its item never survives) — skip the
                    // attach there; the [section.nested] error is the story.
                    if !matches!(parsed, Some(Item::Section(_))) {
                        self.docs.push(DocEntry { item_span: item_span(item), text: text.clone() });
                    }
                } else if let (None, Some((_, sp))) = (&parsed, &docs) {
                    self.warn_dangling_doc(*sp);
                }
                match parsed {
                    // A comptime test has no placement meaning, and the
                    // runner sweeps MODULE items only — a section-nested test
                    // would parse, strip, and silently NEVER RUN (Item-10
                    // review M1): reject it loudly, like nested sections.
                    Some(Item::ComptimeTest(t)) => {
                        self.diag_at(
                            t.span,
                            format!(
                                "[test.in-section] comptime test `{}` is inside a section                                  body — tests have no placement; declare it at module level",
                                t.name
                            ),
                        );
                    }
                    Some(Item::Section(inner)) => {
                        // Sections do not nest (locked decision): placement-within-
                        // placement has no ratified meaning, and `lower_section_items`
                        // has no `Item::Section` arm — it would silently drop
                        // everything inside (data bytes, guards, capacity checks).
                        // Reject loudly here instead; dropping the inner item is safe
                        // because the diagnostic makes the loss visible.
                        self.diag_at(
                            inner.span,
                            format!(
                                "[section.nested] section `{}` is nested inside section `{name}` \
                                 — sections do not nest; declare it at module level",
                                inner.name
                            ),
                        );
                    }
                    Some(i) => items.push(i),
                    None => self.recover_to_next_decl(true),
                }
            }
            self.expect(&Tok::RBrace, "`}`");
            SectionDecl { name, attrs, items, span: start.merge(self.prev_span()) }
        } else {
            let span = start.merge(self.prev_span());
            self.expect_line_end();
            SectionDecl { name, attrs, items, span }
        }
    }

    // ---- expressions (precedence climbing) ----
    // Levels (loosest → tightest):
    //   1 ||   2 &&   3 == != < <= > >=   4 ..   5 ++   6 | ^   7 &
    //   8 << >>   9 + -   10 * / %   unary   postfix(call)   primary
    /// A full expression, including the `|>` pipe. Pipe is looser than every
    /// binary operator and left-associative, so it wraps `expr_bp` as a thin
    /// outer layer rather than threading through the precedence climb. Each
    /// `lhs |> rhs` desugars to an ordinary [`Expr::Call`], so the evaluator
    /// needs no pipe node of its own.
    pub(crate) fn expr(&mut self) -> Expr {
        let mut lhs = self.expr_bp(1);
        while self.at(&Tok::PipeGt) {
            self.bump(); // `|>`
            // Both sides are parsed with `expr_bp(1)`, which stops at the next
            // `|>` (pipe is not one of its operators) — so pipe stays the
            // loosest layer and left-associative. `a + b |> f` is `f(a + b)`;
            // `xs |> f |> g` is `g(f(xs))`. Parsing the target as a full binary
            // expr (not just a primary) means a non-call/non-name target like
            // `a |> f + b` becomes `desugar_pipe`'s clean "must be a call or
            // name" diagnostic instead of orphaning the trailing `+ b`.
            let rhs = self.expr_bp(1);
            lhs = self.desugar_pipe(lhs, rhs);
        }
        lhs
    }

    /// Turn `lhs |> rhs` into a plain call node (the piped value becomes the
    /// first positional argument):
    /// - `rhs = f(args...)` → `f(lhs, args...)`
    /// - `rhs = f` (a bare path) → `f(lhs)`
    /// - anything else (int, lambda literal, ...) → a diagnostic; `lhs` is
    ///   returned unchanged. A lambda as the DIRECT pipe target
    ///   (`xs |> |f| ...`) appears in no spec example — the real use is a
    ///   lambda passed INSIDE a call on the rhs (`xs |> map(|f| ...)`), which
    ///   the call branch handles — so it is rejected with a clear message
    ///   rather than given a bespoke AST affordance.
    fn desugar_pipe(&mut self, lhs: Expr, rhs: Expr) -> Expr {
        let lhs_span = expr_span(&lhs);
        match rhs {
            Expr::Call { callee, mut args, span } => {
                let arg = Arg { name: None, value: lhs, span: lhs_span };
                args.insert(0, arg);
                let span = lhs_span.merge(span);
                Expr::Call { callee, args, span }
            }
            Expr::Path(p) => {
                let span = lhs_span.merge(p.span);
                let arg = Arg { name: None, value: lhs, span: lhs_span };
                Expr::Call { callee: p, args: vec![arg], span }
            }
            Expr::Lambda { span, .. } => {
                self.diag_at(span, "pipe into a lambda literal is not supported; \
                                    name the lambda or use map/filter/fold");
                lhs
            }
            other => {
                self.diag_at(expr_span(&other), "right side of `|>` must be a function call or name");
                lhs
            }
        }
    }

    fn expr_bp(&mut self, min_bp: u8) -> Expr {
        let lhs = self.unary_expr();
        self.binary_continue(lhs, min_bp)
    }

    /// The operator half of [`Parser::expr_bp`], with the left side already
    /// parsed. Split out so operand-position atoms the expression grammar
    /// can't open (a `.local` label) can still take binary continuations —
    /// `jmp .cc_table-4(pc,d0.w)`'s `label - 4` (tranche 9).
    fn binary_continue(&mut self, mut lhs: Expr, min_bp: u8) -> Expr {
        loop {
            let (op, bp) = match self.peek() {
                Tok::OrOr => (BinOp::Or, 1),
                Tok::AndAnd => (BinOp::And, 2),
                Tok::EqEq => (BinOp::Eq, 3), Tok::Ne => (BinOp::Ne, 3),
                Tok::Lt => (BinOp::Lt, 3), Tok::Le => (BinOp::Le, 3),
                Tok::Gt => (BinOp::Gt, 3), Tok::Ge => (BinOp::Ge, 3),
                Tok::DotDot => (BinOp::Add /*unused — range path uses is_range below*/, 4),
                Tok::PlusPlus => (BinOp::Concat, 5),
                Tok::Pipe => (BinOp::BitOr, 6), Tok::Caret => (BinOp::BitXor, 6),
                Tok::Amp => (BinOp::BitAnd, 7),
                Tok::Shl => (BinOp::Shl, 8), Tok::Shr => (BinOp::Shr, 8),
                Tok::Plus => (BinOp::Add, 9), Tok::Minus => (BinOp::Sub, 9),
                Tok::Star => (BinOp::Mul, 10), Tok::Slash => (BinOp::Div, 10),
                Tok::Percent => (BinOp::Mod, 10),
                _ => break,
            };
            if bp < min_bp { break; }
            let is_range = matches!(self.peek(), Tok::DotDot);
            self.bump();
            let rhs = self.expr_bp(bp + 1); // left-assoc
            let span = expr_span(&lhs).merge(expr_span(&rhs));
            lhs = if is_range {
                Expr::Range { lo: Box::new(lhs), hi: Box::new(rhs), span }
            } else {
                Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span }
            };
        }
        lhs
    }

    fn unary_expr(&mut self) -> Expr {
        // Same depth guard as `primary_expr`: a long `-`/`!`/`~` chain would
        // otherwise recurse once per operator — bounded only by input size,
        // not by a constant — and abort the process with a stack overflow.
        if self.depth >= MAX_EXPR_DEPTH {
            let span = self.span();
            self.diag_at(span, "expression nesting too deep (max 128)");
            return Expr::Path(Path { segments: vec!["<error>".into()], span });
        }
        let start = self.span();
        let op = match self.peek() {
            Tok::Minus => Some(UnOp::Neg),
            Tok::Bang => Some(UnOp::Not),
            Tok::Tilde => Some(UnOp::BitNot),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            self.depth += 1;
            let expr = self.unary_expr();
            self.depth -= 1;
            let span = start.merge(expr_span(&expr));
            return Expr::Unary { op, expr: Box::new(expr), span };
        }
        self.postfix_expr()
    }

    /// A primary expression followed by any chain of postfix operators
    /// (D2.33): `base[i]` comptime indexing and `.field` access off a
    /// non-path base (`embed(...).len` — a PATH primary eats its own dots as
    /// segments, so this arm only ever fires after calls/literals/parens).
    ///
    /// Postfix binds tighter than unary (`-x[0]` negates the element), and
    /// both operators require the opener on the SAME line: a line-leading
    /// `[` stays an array literal, and `Tok::Newline` between base and `.`
    /// keeps statement boundaries intact (both loops test the token stream,
    /// where the newline token intervenes).
    fn postfix_expr(&mut self) -> Expr {
        let mut e = self.primary_expr();
        loop {
            if self.at(&Tok::LBracket) {
                self.bump();
                // Struct literals are unambiguous again inside `[...]` —
                // same save/restore as the array-literal primary.
                let saved_nsl = self.no_struct_lit;
                self.no_struct_lit = false;
                self.skip_newlines();
                let index = self.expr();
                self.skip_newlines();
                self.expect(&Tok::RBracket, "`]`");
                self.no_struct_lit = saved_nsl;
                let span = expr_span(&e).merge(self.prev_span());
                e = Expr::Index { base: Box::new(e), index: Box::new(index), span };
                continue;
            }
            // `.field` — only off a NON-path base (paths consume their own
            // dotted segments in `path()`), and never a method CALL: a `(`
            // after the field would be a method call on an expression
            // result, which has no evaluation route yet — steer instead of
            // mis-parsing.
            if self.at(&Tok::Dot) {
                if let Tok::Ident(name) = self.peek2().clone() {
                    // The size-suffix guard (the `split_size_suffix` rule,
                    // applied postfix): `timer(a0).l` in operand position is
                    // a SIZE, not a field — postfix field access never
                    // consumes the four size letters, so `trailing_size`
                    // still finds them. A comptime struct field genuinely
                    // named `b`/`w`/`l`/`s` on a call result needs a const
                    // binding first — same accepted trade as the operand
                    // rule.
                    if matches!(name.as_str(), "b" | "w" | "l" | "s") {
                        break;
                    }
                    if matches!(self.peek_at(2), Tok::LParen) {
                        let sp = self.span();
                        self.diag_at(
                            sp,
                            "method calls on an expression result are not supported — \
                             bind the receiver to a `const` first, then call through the name",
                        );
                        return e;
                    }
                    self.bump(); // dot
                    self.bump(); // ident
                    let span = expr_span(&e).merge(self.prev_span());
                    e = Expr::Field { base: Box::new(e), name, span };
                    continue;
                }
            }
            break;
        }
        e
    }

    fn primary_expr(&mut self) -> Expr {
        // Depth guard: error out instead of recursing into pathologically
        // nested input (e.g. hundreds of `(`), which would abort the process
        // with a stack overflow. Guarded entry points: `primary_expr`,
        // `unary_expr`, and `ty` — together they cover every unbounded
        // recursion path (`expr_bp` recurses only via unary/primary).
        if self.depth >= MAX_EXPR_DEPTH {
            let span = self.span();
            self.diag_at(span, "expression nesting too deep (max 128)");
            return Expr::Path(Path { segments: vec!["<error>".into()], span });
        }
        self.depth += 1;
        let r = self.primary_expr_inner();
        self.depth -= 1;
        r
    }

    fn primary_expr_inner(&mut self) -> Expr {
        let start = self.span();
        match self.peek().clone() {
            Tok::Int(v) => { self.bump(); Expr::Int(v, start) }
            Tok::Float(v) => { self.bump(); Expr::Float(v, start) }
            Tok::Str(s) => { self.bump(); Expr::Str(s, start) }
            Tok::LBracket => {
                self.bump();
                // Struct literals are unambiguous again once inside `[...]`,
                // even if an enclosing if/while/for header disabled them.
                let saved_nsl = self.no_struct_lit;
                self.no_struct_lit = false;
                let mut elems = Vec::new();
                self.skip_newlines();
                if !self.at(&Tok::RBracket) {
                    loop {
                        elems.push(self.expr());
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) { break; }
                        self.skip_newlines();
                        if self.at(&Tok::RBracket) { break; } // trailing comma
                    }
                }
                self.expect(&Tok::RBracket, "`]`");
                self.no_struct_lit = saved_nsl;
                Expr::ArrayLit { elems, span: start.merge(self.prev_span()) }
            }
            Tok::LParen => {
                self.bump();
                // Struct literals are unambiguous again once inside `(...)`.
                let saved_nsl = self.no_struct_lit;
                self.no_struct_lit = false;
                let mut elems = vec![self.expr()];
                let mut saw_comma = false;
                while self.eat(&Tok::Comma) {
                    saw_comma = true;
                    if self.at(&Tok::RParen) { break; } // trailing comma
                    elems.push(self.expr());
                }
                self.expect(&Tok::RParen, "`)`");
                self.no_struct_lit = saved_nsl;
                if !saw_comma {
                    // plain grouping: `(e)`
                    elems.pop().unwrap()
                } else {
                    // any comma makes a tuple, so `(1,)` is a 1-element TupleLit
                    Expr::TupleLit { elems, span: start.merge(self.prev_span()) }
                }
            }
            // A splice nested inside a larger expression, e.g. the `{reg}` in
            // `VDP_Shadow_Table + {reg}`. Operand/mnemonic-position splices
            // (a `{expr}` that IS the whole operand/mnemonic part) are
            // handled by their own callers before they ever reach here — this
            // arm only fires for a `{` that `expr()` encounters mid-expression.
            Tok::LBrace if self.splice_ctx => {
                self.bump();
                let e = self.expr();
                self.expect(&Tok::RBrace, "`}`");
                e
            }
            // A `|` where an expression STARTS is a lambda `|p, ...| body`.
            // Infix bit-or `|` only ever follows a primary, so it is reached in
            // `expr_bp`, not here — there is no ambiguity. `||` lexes as
            // `Tok::OrOr`, so zero-param lambdas are unwritable by construction.
            Tok::Pipe => {
                self.bump(); // opening `|`
                let mut params = Vec::new();
                // `| |` (two pipes) leaves the list empty: diagnose rather than
                // silently accept a zero-param lambda (spec lambdas take ≥1 elem).
                if self.at(&Tok::Pipe) {
                    self.diag_at(start, "lambda needs at least one parameter");
                } else {
                    loop {
                        params.push(self.expect_ident("lambda parameter"));
                        if !self.eat(&Tok::Comma) { break; }
                    }
                }
                self.expect(&Tok::Pipe, "`|` to close the lambda parameter list");
                // The body is a full expression: `|x| x + 1` binds the `+`, and
                // inside `map(|f| f + 1)` the body stops at the enclosing `)`.
                let body = self.expr();
                let span = start.merge(expr_span(&body));
                Expr::Lambda { params, body: Box::new(body), span }
            }
            Tok::Ident(_) => {
                // `comptime for/if` in expression position: the `comptime`
                // marker is a no-op inside an already-comptime context.
                if self.at_kw("comptime")
                    && matches!(self.peek2(), Tok::Ident(s) if s == "for" || s == "if") {
                    self.bump();
                }
                if self.at_kw("if") { return self.if_expr(); }
                if self.at_kw("for") { return self.for_expr(); }
                if self.at_kw("asm") { return self.asm_expr(); }
                if self.at_kw("match") { return self.match_expr(); }
                // Type-argument builtins are recognized only when the ident is
                // DIRECTLY followed by their opener — otherwise they are
                // ordinary, unreserved names (§10 guidance).
                if self.at_kw("sizeof") && matches!(self.peek2(), Tok::LParen) {
                    return self.sizeof_expr();
                }
                if self.at_kw("offsetof") && matches!(self.peek2(), Tok::LParen) {
                    return self.offsetof_expr();
                }
                // `rescale` sits in EXPRESSION position, where `<` is a valid
                // comparison operator — unlike `fixed<...>`, which lives in
                // type position where `<` is never infix. So committing on a
                // bare `rescale <` would mis-parse `rescale < 5` (an ordinary
                // name compared with `<`) into a broken `Rescale` node. Require
                // the full `< int ,` prefix — the unambiguous `rescale<I,F>`
                // shape — before committing; anything else falls through to
                // ordinary path/binary parsing.
                if self.at_kw("rescale")
                    && matches!(self.peek2(), Tok::Lt)
                    && matches!(self.peek_at(2), Tok::Int(_))
                    && matches!(self.peek_at(3), Tok::Comma)
                {
                    return self.rescale_expr();
                }
                let path = self.path();
                match self.peek() {
                    Tok::LParen => {
                        self.bump();
                        // Struct literals are unambiguous again inside call args.
                        let saved_nsl = self.no_struct_lit;
                        self.no_struct_lit = false;
                        let mut args = Vec::new();
                        self.skip_newlines();
                        if !self.at(&Tok::RParen) {
                            loop {
                                args.push(self.arg());
                                self.skip_newlines();
                                if !self.eat(&Tok::Comma) { break; }
                                self.skip_newlines();
                                if self.at(&Tok::RParen) { break; } // trailing comma
                            }
                        }
                        self.expect(&Tok::RParen, "`)`");
                        self.no_struct_lit = saved_nsl;
                        Expr::Call { callee: path, args, span: start.merge(self.prev_span()) }
                    }
                    Tok::LBrace if !self.no_struct_lit => {
                        // struct literal: Path{ field: e, ... }
                        self.bump();
                        // Struct literals are unambiguous again inside field values.
                        let saved_nsl = self.no_struct_lit;
                        self.no_struct_lit = false;
                        let mut fields = Vec::new();
                        self.skip_newlines();
                        if !self.at(&Tok::RBrace) {
                            loop {
                                // `..` was built and RETIRED at the tranche-0
                                // checkpoint (it couldn't say WHICH fields it
                                // covered) — teach the named spelling.
                                if self.eat(&Tok::DotDot) {
                                    let sp = self.prev_span();
                                    self.diag_at(
                                        sp,
                                        "`..` rest-fill was retired — name each elided field \
                                         instead: `field: default` (S2-D13(h), checkpoint \
                                         ruling)",
                                    );
                                    self.skip_newlines();
                                    if self.eat(&Tok::Comma) {
                                        self.skip_newlines();
                                    }
                                    if self.at(&Tok::RBrace) { break; }
                                    continue;
                                }
                                let name = self.expect_ident("field name");
                                self.expect(&Tok::Colon, "`:`");
                                // `field: default` — the contextual named-
                                // elision marker (S2-D13(h)): exact bareword
                                // `default` ending the field (comma / `}` /
                                // newline). Any other continuation parses as
                                // an ordinary expression, so a const named
                                // `default` stays usable in arithmetic.
                                let value = if self.at_kw("default")
                                    && matches!(
                                        self.peek2(),
                                        Tok::Comma | Tok::RBrace | Tok::Newline | Tok::Eof
                                    ) {
                                    let dspan = self.span();
                                    self.bump(); // `default`
                                    Expr::Default(dspan)
                                } else {
                                    self.expr()
                                };
                                fields.push((name, value));
                                self.skip_newlines();
                                if !self.eat(&Tok::Comma) { break; }
                                self.skip_newlines();
                                if self.at(&Tok::RBrace) { break; }
                            }
                        }
                        self.expect(&Tok::RBrace, "`}`");
                        self.no_struct_lit = saved_nsl;
                        Expr::StructLit { ty: path, fields, span: start.merge(self.prev_span()) }
                    }
                    _ => Expr::Path(path),
                }
            }
            // F2 (tranche 7): a proc-LOCAL label reference `.name` in expression
            // position — accepted so it can appear as a `Label`-typed CALL
            // ARGUMENT (`axis_test(d4, ..., .next_object)`). It is ONLY meaningful
            // in a label-value context; the evaluator rejects it loudly in any
            // pure comptime expression position (`const x = .foo`), so parsing it
            // here never leaks a silent Label into ordinary expressions.
            Tok::Dot if matches!(self.peek2(), Tok::Ident(_)) => {
                self.bump(); // `.`
                let name = self.expect_ident("label");
                Expr::LocalLabel(name, start.merge(self.prev_span()))
            }
            other => {
                self.diag_at(start, format!("expected an expression, found {other:?}"));
                // Never consume a closer (or newline) an enclosing frame will
                // expect — bumping it would cascade into bogus diagnostics.
                if !matches!(self.peek(),
                    Tok::RBrace | Tok::RParen | Tok::RBracket | Tok::Newline) {
                    self.bump();
                }
                Expr::Path(Path { segments: vec!["<error>".into()], span: start })
            }
        }
    }

    fn arg(&mut self) -> Arg {
        let start = self.span();
        // `name: value` — ident followed by colon
        if matches!(self.peek(), Tok::Ident(_)) && matches!(self.peek2(), Tok::Colon) {
            let name = self.expect_ident("argument name");
            self.bump(); // colon
            let value = self.expr();
            let span = start.merge(expr_span(&value));
            return Arg { name: Some(name), value, span };
        }
        let value = self.expr();
        let span = expr_span(&value);
        Arg { name: None, value, span }
    }

    /// `if cond { then... } [else { els... } | else if ...]`.
    ///
    /// The condition is parsed with struct literals disabled (Rust's rule):
    /// `if x { ... }` must not read `x {` as a struct literal.
    fn if_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `if`
        let cond = self.expr_no_struct_lit();
        let then = self.stmt_block();
        let els = if self.eat_kw("else") {
            if self.at_kw("if") {
                Some(vec![Stmt::If(self.if_expr())])
            } else {
                Some(self.stmt_block())
            }
        } else { None };
        Expr::If { cond: Box::new(cond), then, els, span: start.merge(self.prev_span()) }
    }

    /// `for var in iter { body... }`. The iterable is parsed with struct
    /// literals disabled, same as `if_expr`'s condition.
    fn for_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `for`
        let var = self.expect_ident("loop variable");
        if !self.eat_kw("in") {
            let sp = self.span();
            self.diag_at(sp, "expected `in`");
        }
        let iter = self.expr_no_struct_lit();
        let body = self.stmt_block();
        Expr::For { var, iter: Box::new(iter), body, span: start.merge(self.prev_span()) }
    }

    /// `asm { ... }` — a quoted Code template; splices are legal inside.
    fn asm_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `asm`
        self.expect(&Tok::LBrace, "`{`");
        let body = self.asm_body(/* splices_allowed = */ true);
        self.expect(&Tok::RBrace, "`}`");
        Expr::Asm { body, span: start.merge(self.prev_span()) }
    }

    /// `match scrutinee { Pat => body, ... }`. The scrutinee is parsed with
    /// struct literals disabled, same as `if`/`for` (Rust's rule): `match x {
    /// ... }` must not read `x {` as a struct literal.
    fn match_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `match`
        let scrutinee = self.expr_no_struct_lit();
        self.expect(&Tok::LBrace, "`{`");
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; }
            let arm_start = self.span();
            let pat = self.pattern();
            self.expect(&Tok::FatArrow, "`=>`");
            let body = self.expr();
            let span = arm_start.merge(expr_span(&body));
            arms.push(MatchArm { pat, body, span });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) { break; }
            self.skip_newlines();
            if self.at(&Tok::RBrace) { break; } // trailing comma
        }
        self.skip_newlines();
        self.expect(&Tok::RBrace, "`}`");
        let span = start.merge(self.prev_span());
        if arms.is_empty() {
            self.diag_at(span, "`match` must have at least one arm");
        }
        Expr::Match { scrutinee: Box::new(scrutinee), arms, span }
    }

    /// A single match-arm pattern: `_`, a bare lowercase binding, or a
    /// (possibly-qualified) variant path with optional parenthesized
    /// subpatterns: `Anim.Idle`, `Token.Literal(s)`.
    fn pattern(&mut self) -> Pattern {
        let start = self.span();
        if self.at_kw("_") {
            self.bump();
            return Pattern::Wildcard(start);
        }
        let path = self.path();
        let is_binding = path.segments.len() == 1
            && path.segments[0].chars().next().is_some_and(|c| c.is_lowercase());
        if is_binding {
            return Pattern::Binding(path.segments[0].clone(), path.span);
        }
        let mut subpats = Vec::new();
        if self.eat(&Tok::LParen) {
            if !self.at(&Tok::RParen) {
                loop {
                    subpats.push(self.pattern());
                    if !self.eat(&Tok::Comma) { break; }
                    if self.at(&Tok::RParen) { break; } // trailing comma
                }
            }
            self.expect(&Tok::RParen, "`)`");
        }
        let span = path.span.merge(self.prev_span());
        Pattern::Variant { path, subpats, span }
    }

    /// `sizeof(T)` — the byte size of a type.
    fn sizeof_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `sizeof`
        self.expect(&Tok::LParen, "`(`");
        let ty = self.ty();
        self.expect(&Tok::RParen, "`)`");
        Expr::SizeOf(Box::new(ty), start.merge(self.prev_span()))
    }

    /// `offsetof(T, field)` — the byte offset of `field` within `T`.
    fn offsetof_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `offsetof`
        self.expect(&Tok::LParen, "`(`");
        let ty = self.ty();
        self.expect(&Tok::Comma, "`,`");
        let field = self.expect_ident("field name");
        self.expect(&Tok::RParen, "`)`");
        Expr::OffsetOf(Box::new(ty), field, start.merge(self.prev_span()))
    }

    /// `rescale<I, F>(x)` — reinterpret a fixed-point value under a new
    /// `fixed<I, F>` scale.
    fn rescale_expr(&mut self) -> Expr {
        let start = self.span();
        self.bump(); // `rescale`
        self.expect(&Tok::Lt, "`<`");
        let i = self.expect_u32_lit("an integer bit width");
        self.expect(&Tok::Comma, "`,`");
        let f = self.expect_u32_lit("a fraction bit width");
        self.expect(&Tok::Gt, "`>`");
        self.expect(&Tok::LParen, "`(`");
        let arg = self.expr();
        self.expect(&Tok::RParen, "`)`");
        Expr::Rescale { i, f, arg: Box::new(arg), span: start.merge(self.prev_span()) }
    }
}

/// Span of any expression node (helper for span merging).
pub(crate) fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Int(_, s) | Expr::Float(_, s) | Expr::Str(_, s) | Expr::Default(s) => *s,
        Expr::Path(p) => p.span,
        Expr::LocalLabel(_, s) => *s,
        Expr::Unary { span, .. } | Expr::Binary { span, .. } | Expr::Call { span, .. }
        | Expr::StructLit { span, .. } | Expr::ArrayLit { span, .. }
        | Expr::TupleLit { span, .. } | Expr::Range { span, .. } | Expr::If { span, .. }
        | Expr::For { span, .. } | Expr::Asm { span, .. }
        | Expr::Lambda { span, .. } | Expr::Match { span, .. }
        | Expr::Rescale { span, .. } | Expr::Index { span, .. }
        | Expr::Field { span, .. } => *span,
        Expr::SizeOf(_, s) | Expr::OffsetOf(_, _, s) => *s,
    }
}

/// `d0.w` arrives from `path()` as Path["d0","w"] — split the trailing
/// single-letter size segment back off (see `paren_operand`'s doc comment).
pub(crate) fn split_size_suffix(e: Expr) -> (Expr, Option<TextOrSplice>) {
    if let Expr::Path(p) = &e {
        if p.segments.len() >= 2 {
            let last = p.segments.last().unwrap();
            if last == "b" || last == "w" || last == "l" {
                let mut q = p.clone();
                let size = q.segments.pop().unwrap();
                return (Expr::Path(q), Some(TextOrSplice::Text(size)));
            }
        }
    }
    (e, None)
}

/// Test-only hook: parse a bare expression.
pub fn parse_expr_for_tests(src: &str) -> Expr {
    let (tokens, errs) = crate::lexer::lex(src, sigil_span::SourceId(0));
    assert!(errs.is_empty(), "{errs:?}");
    let mut p = Parser::new(tokens);
    let e = p.expr();
    assert!(p.diags.is_empty(), "{:?}", p.diags);
    e
}
