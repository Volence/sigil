//! Recursive-descent parser for .emp with declaration-keyword recovery.
use crate::ast::*;
use crate::lexer::{Tok, Token};
use sigil_span::{Diagnostic, Level, Span};

/// A recursive-descent parser over a token stream, collecting diagnostics
/// instead of failing fast.
pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    /// Build a parser over an already-lexed token stream (must end in `Eof`).
    pub fn new(toks: Vec<Token>) -> Self {
        Parser { toks, pos: 0, diags: Vec::new() }
    }
    /// Consume the parser, returning every diagnostic collected so far.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> { self.diags }

    // ---- cursor helpers ----
    fn peek(&self) -> &Tok { &self.toks[self.pos].tok }
    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
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
    fn skip_newlines(&mut self) { while self.eat(&Tok::Newline) {} }
    fn expect_line_end(&mut self) {
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

    // ---- file ----
    /// Parse a whole file: module header, module-level attributes, then items.
    pub fn file(&mut self) -> File {
        self.skip_newlines();
        let diags_before = self.diags.len();
        let module = self.module_decl();
        if self.diags.len() > diags_before {
            // Fatal: no valid `module` header to anchor on. Item parsers for
            // const/enum/.../section land in Tasks 7-13; bail out here
            // rather than falling into their `unimplemented!` stubs.
            return File { module, attrs: Vec::new(), items: Vec::new() };
        }
        // module-level attributes: `@as_compat`, `@allow(naming.pascal)`
        let mut attrs = Vec::new();
        loop {
            self.skip_newlines();
            if !self.at(&Tok::At) { break; }
            let aspan = self.span();
            self.bump();
            let name = self.expect_ident("attribute name");
            let mut args = Vec::new();
            if self.eat(&Tok::LParen) {
                loop {
                    args.push(self.expr());
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(&Tok::RParen, "`)`");
            }
            self.expect_line_end();
            attrs.push(Attr { name, args, span: aspan.merge(self.prev_span()) });
        }
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if self.at(&Tok::Eof) { break; }
            match self.item() {
                Some(item) => items.push(item),
                None => self.recover_to_next_decl(),
            }
        }
        File { module, attrs, items }
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
        self.expect_line_end();
        ModuleDecl { path, in_section, span: start }
    }

    fn path(&mut self) -> Path {
        let start = self.span();
        let mut segments = vec![self.expect_ident("name")];
        while self.at(&Tok::Dot) && matches!(self.peek2(), Tok::Ident(_)) {
            self.bump(); // dot
            segments.push(self.expect_ident("name"));
        }
        Path { segments, span: Span { source: start.source, start: start.start, end: self.prev_span().end } }
    }

    /// Dispatch on the leading contextual keyword. Returns None on an
    /// unrecognized opener (caller recovers).
    fn item(&mut self) -> Option<Item> {
        let public = self.eat_kw("pub");
        if self.at_kw("use") { return Some(Item::Use(self.use_decl())); }
        if self.at_kw("const") { return Some(Item::Const(self.const_decl(public))); }
        if self.at_kw("enum") { return Some(Item::Enum(self.enum_decl(public))); }
        if self.at_kw("bitfield") { return Some(Item::Bitfield(self.bitfield_decl(public))); }
        if self.at_kw("struct") { return Some(Item::Struct(self.struct_decl(public))); }
        if self.at_kw("vars") { return Some(Item::Vars(self.vars_decl(public))); }
        if self.at_kw("data") { return Some(Item::Data(self.data_decl(public))); }
        if self.at_kw("proc") { return Some(Item::Proc(self.proc_decl(public))); }
        if self.at_kw("comptime") { return Some(Item::ComptimeFn(self.comptime_fn_decl(public))); }
        if self.at_kw("section") { return Some(Item::Section(self.section_decl())); }
        let span = self.span();
        self.diag_at(span, format!("expected a declaration, found {:?}", self.peek()));
        None
    }

    /// Error recovery: skip until a token that can start a declaration.
    fn recover_to_next_decl(&mut self) {
        const OPENERS: [&str; 11] = ["use", "const", "enum", "bitfield", "struct",
                                     "vars", "data", "proc", "comptime", "section", "pub"];
        let mut depth = 0i32;
        loop {
            match self.peek() {
                Tok::Eof => return,
                Tok::LBrace => { depth += 1; self.bump(); }
                Tok::RBrace => { depth -= 1; self.bump(); }
                Tok::Ident(s) if depth <= 0 && OPENERS.contains(&s.as_str()) => return,
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
        let mut names = UseNames::Whole;
        loop {
            if !self.at(&Tok::Dot) { break; }
            match self.peek2().clone() {
                Tok::Ident(_) => {
                    self.bump();
                    segments.push(self.expect_ident("name"));
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
        self.expect_line_end();
        let base = Path {
            segments,
            span: Span { source: pstart.source, start: pstart.start, end: self.prev_span().end },
        };
        UseDecl { base, names, span: start }
    }

    // ---- stubs replaced by later tasks (each panics with the task that owns it) ----
    // These are unreachable via `parser_decls.rs` today (no test exercises
    // const/enum/bitfield/struct/vars/data/proc/comptime-fn/section bodies
    // yet), so clippy sees them as effectively dead until their owning WP
    // lands; keep them `unimplemented!` per spec rather than deleting.
    #[allow(dead_code)] // owned by Task 7
    fn const_decl(&mut self, _p: bool) -> ConstDecl { unimplemented!("Task 7") }
    #[allow(dead_code)] // owned by Task 7
    fn enum_decl(&mut self, _p: bool) -> EnumDecl { unimplemented!("Task 7") }
    #[allow(dead_code)] // owned by Task 8
    fn bitfield_decl(&mut self, _p: bool) -> BitfieldDecl { unimplemented!("Task 8") }
    #[allow(dead_code)] // owned by Task 8
    fn struct_decl(&mut self, _p: bool) -> StructDecl { unimplemented!("Task 8") }
    #[allow(dead_code)] // owned by Task 9
    fn vars_decl(&mut self, _p: bool) -> VarsDecl { unimplemented!("Task 9") }
    #[allow(dead_code)] // owned by Task 9
    fn data_decl(&mut self, _p: bool) -> DataDecl { unimplemented!("Task 9") }
    #[allow(dead_code)] // owned by Task 10
    fn proc_decl(&mut self, _p: bool) -> ProcDecl { unimplemented!("Task 10") }
    #[allow(dead_code)] // owned by Task 11
    fn comptime_fn_decl(&mut self, _p: bool) -> ComptimeFnDecl { unimplemented!("Task 11") }
    #[allow(dead_code)] // owned by Task 13
    fn section_decl(&mut self) -> SectionDecl { unimplemented!("Task 13") }

    // Task 6 adds expressions; Task 5 needs a minimal expr for attribute args.
    /// Minimal placeholder — REPLACED WHOLESALE by Task 6 (expressions WP).
    pub(crate) fn expr(&mut self) -> Expr {
        let start = self.span();
        match self.peek().clone() {
            Tok::Int(v) => { self.bump(); Expr::Int(v, start) }
            Tok::Float(v) => { self.bump(); Expr::Float(v, start) }
            Tok::Str(s) => { self.bump(); Expr::Str(s, start) }
            Tok::Ident(_) => Expr::Path(self.path()),
            other => {
                self.diag_at(start, format!("expected an expression, found {other:?}"));
                self.bump();
                Expr::Path(Path { segments: vec!["<error>".into()], span: start })
            }
        }
    }
}
