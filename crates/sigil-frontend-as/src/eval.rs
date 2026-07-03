//! eval: the driver — line loop, directive dispatch, instruction lowering, emit.

use crate::lexer::lex_line;
use crate::operands::{parse_operands, OperandAtom};
use crate::parser::parse_line_tokens;
use crate::token::{Punct, Tok, Token};
use crate::Options;
use sigil_backend_z80::z80::{Cond, Mnemonic, Operand, Reg16, Reg8};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer, LowerError};
use sigil_ir::expr::Fold;
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, IrBuilder, Module, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, SourceId, Span};

#[derive(Clone)]
struct SrcLine {
    text: String,
    base: u32,
}

pub fn run(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    let mut asm = Asm::new(opts);
    for (k, v) in &opts.defines {
        asm.env.define(k, SymbolValue::Int(*v));
    }
    asm.process(src);
    let (module, mut diags) = asm.builder.finish();
    diags.append(&mut asm.diags);
    if diags.iter().any(|d| d.level == Level::Error) {
        Err(diags)
    } else {
        Ok(module)
    }
}

struct Asm {
    builder: IrBuilder,
    backend: Z80Backend,
    state: crate::state::AsmState,
    env: SymbolTable,
    scope: Option<String>,
    in_section: bool,
    diags: Vec<Diagnostic>,
    source: SourceId,
    functions: std::collections::BTreeMap<String, (Vec<String>, Vec<Token>)>,
}

enum Lowered {
    Fixed(Vec<Operand>),
    Rel(Option<Cond>, Expr),
    Abs16(Vec<Operand>, Expr),
}

impl Asm {
    fn new(opts: &Options) -> Self {
        Asm {
            builder: IrBuilder::new(),
            backend: Z80Backend,
            state: crate::state::AsmState::new(opts.initial_cpu),
            env: SymbolTable::new(),
            scope: None,
            in_section: false,
            diags: Vec::new(),
            source: SourceId(0),
            functions: std::collections::BTreeMap::new(),
        }
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Error, message: msg.into(), primary: span });
    }

    fn here(&self) -> u32 {
        self.state.vma_base.unwrap_or(0) + self.builder.current_offset()
    }

    fn fold(&self, e: &Expr) -> Fold {
        let here = self.here() as i64;
        let scope = self.scope.clone();
        let env = &self.env;
        e.fold(&|name| {
            // `$` is resolved to the current PC here in the front-end: any
            // `$`-bearing expression folds to a concrete value immediately and
            // never survives as a Sym fixup target, so the linker never sees `$`.
            if name == "$" {
                Some(here)
            } else {
                env.resolve(name, scope.as_deref())
            }
        })
    }

    /// Fold an immediate to a value in [lo,hi]. Out-of-range → diagnostic + clamp.
    /// Unresolved (Poison) → 0 placeholder (byte-stable; the fixpoint resolves later).
    fn fold_imm(&mut self, e: &Expr, span: Span, lo: i64, hi: i64) -> i64 {
        match self.fold(e) {
            Fold::Value(v) if v >= lo && v <= hi => v,
            Fold::Value(v) => {
                self.err(span, format!("operand {v} out of range {lo}..={hi}"));
                v.clamp(lo, hi)
            }
            Fold::Poison => 0,
        }
    }

    /// Fold a whole token slice as one constant expression (used by phase, etc.).
    fn eval_all(&mut self, toks: &[Token], span: Span) -> Option<i64> {
        let expanded = self.expand_calls(toks);
        let (e, rest) = crate::expr::parse_expr(&expanded)?;
        if !rest.is_empty() {
            self.err(span, "trailing tokens in expression");
            return None;
        }
        match self.fold(&e) {
            Fold::Value(v) => Some(v),
            Fold::Poison => None,
        }
    }

    /// Parse `function name(p1,p2,...) = <body tokens>` and store it.
    fn def_function(&mut self, line: &SrcLine) {
        let toks = match lex_line(&line.text, self.state.cpu, self.source, line.base) {
            Ok(t) => t,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        // toks[0] = `function`, toks[1] = name, toks[2] = `(`, params..., `)`, `=`, body...
        let span = toks.first().map(|t| t.span).unwrap_or(Span { source: self.source, start: line.base, end: line.base });
        let name = match toks.get(1).map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "function needs a name");
                return;
            }
        };
        let mut i = 2;
        if !matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen))) {
            self.err(span, "function needs `(params)`");
            return;
        }
        i += 1;
        let mut params = Vec::new();
        loop {
            match toks.get(i).map(|t| &t.tok) {
                Some(Tok::Punct(Punct::RParen)) => {
                    i += 1;
                    break;
                }
                Some(Tok::Punct(Punct::Comma)) => i += 1,
                Some(Tok::Ident(p)) => {
                    params.push(p.clone());
                    i += 1;
                }
                _ => {
                    self.err(span, "bad function parameter list");
                    return;
                }
            }
        }
        if !matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Punct(Punct::Eq))) {
            self.err(span, "function needs `= <expr>`");
            return;
        }
        i += 1;
        let body = toks[i..].to_vec();
        self.functions.insert(name, (params, body));
    }

    /// Expand every known-function call `fname(args)` in `toks` into its
    /// parenthesised, parameter-substituted body (recursively). Unknown `Ident(`
    /// is left untouched (it may be a `(nn)`-style group, not a call).
    fn expand_calls(&self, toks: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if let Some((params, body)) = self.functions.get(name) {
                    if matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen))) {
                        if let Some((args, next)) = split_call_args(toks, i + 1) {
                            let expanded = self.substitute(body, params, &args);
                            let span = toks[i].span;
                            out.push(paren(Punct::LParen, span));
                            out.extend(self.expand_calls(&expanded));
                            out.push(paren(Punct::RParen, span));
                            i = next;
                            continue;
                        }
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Replace each body identifier equal to a parameter with its (expanded,
    /// parenthesised) argument tokens.
    fn substitute(&self, body: &[Token], params: &[String], args: &[Vec<Token>]) -> Vec<Token> {
        let mut out = Vec::new();
        for t in body {
            if let Tok::Ident(name) = &t.tok {
                if let Some(idx) = params.iter().position(|p| p == name) {
                    if let Some(arg) = args.get(idx) {
                        let expanded_arg = self.expand_calls(arg);
                        out.push(paren(Punct::LParen, t.span));
                        out.extend(expanded_arg);
                        out.push(paren(Punct::RParen, t.span));
                        continue;
                    }
                }
            }
            out.push(t.clone());
        }
        out
    }

    fn process(&mut self, src: &str) {
        let mut lines = Vec::new();
        let mut base = 0u32;
        for raw in src.split_inclusive('\n') {
            lines.push(SrcLine { text: raw.to_string(), base });
            base += raw.len() as u32;
        }
        self.exec(&lines);
    }

    /// Execute a slice of logical lines in order, handling block directives.
    fn exec(&mut self, lines: &[SrcLine]) {
        let mut i = 0;
        while i < lines.len() {
            match self.line_keyword(&lines[i]).as_deref() {
                Some("if") | Some("ifdef") | Some("ifndef") => {
                    i = self.exec_if(lines, i);
                }
                Some("rept") => {
                    i = self.exec_rept(lines, i);
                }
                Some("struct") => {
                    i = self.capture_struct(lines, i);
                }
                Some("function") => {
                    self.def_function(&lines[i]);
                    i += 1;
                }
                _ => {
                    self.exec_one(&lines[i]);
                    i += 1;
                }
            }
        }
    }

    fn exec_one(&mut self, line: &SrcLine) {
        let toks = match lex_line(&line.text, self.state.cpu, self.source, line.base) {
            Ok(t) => t,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        if toks.is_empty() {
            return;
        }
        let parsed = parse_line_tokens(&toks);
        if let Some(name) = parsed.label_colon.clone() {
            self.define_label(&name);
        }
        let body = parsed.tokens;
        if body.is_empty() {
            return;
        }
        let head = match &body[0].tok {
            Tok::Ident(s) => s.clone(),
            _ => {
                self.err(body[0].span, "expected mnemonic, directive, or label");
                return;
            }
        };
        if body.len() >= 2 && matches!(body[1].tok, Tok::Punct(Punct::Eq)) {
            self.directive_equate(&head, &body[2..], body[0].span);
            return;
        }
        if !is_op_keyword(&head) && !is_mnemonic(&head) {
            self.define_label(&head);
            if body.len() == 1 {
                return;
            }
            let rest = &body[1..];
            let rhead = match &rest[0].tok {
                Tok::Ident(s) => s.clone(),
                _ => {
                    self.err(rest[0].span, "expected mnemonic or directive after label");
                    return;
                }
            };
            self.dispatch(&rhead, &rest[1..], rest[0].span);
            return;
        }
        self.dispatch(&head, &body[1..], body[0].span);
    }

    /// The dispatch keyword of a line (after peeling an optional label), or None
    /// for a blank/label-only/lex-error line.
    fn line_keyword(&self, line: &SrcLine) -> Option<String> {
        let toks = lex_line(&line.text, self.state.cpu, self.source, line.base).ok()?;
        if toks.is_empty() {
            return None;
        }
        let parsed = parse_line_tokens(&toks);
        let body = if parsed.label_colon.is_some() { parsed.tokens } else { toks };
        let first = body.first()?;
        let name = match &first.tok {
            Tok::Ident(s) => s.clone(),
            _ => return None,
        };
        // A leading bareword that is neither directive nor mnemonic is a bare
        // label; the dispatch keyword is the next token.
        if !is_op_keyword(&name) && !is_mnemonic(&name) && body.len() > 1 {
            if let Tok::Ident(s) = &body[1].tok {
                return Some(s.clone());
            }
        }
        Some(name)
    }

    /// The keyword + the tokens after it + the keyword span, for a block head.
    fn line_kw_args(&self, line: &SrcLine) -> (Option<String>, Vec<Token>, Span) {
        let toks = match lex_line(&line.text, self.state.cpu, self.source, line.base) {
            Ok(t) => t,
            Err(_) => return (None, Vec::new(), Span { source: self.source, start: line.base, end: line.base }),
        };
        let parsed = parse_line_tokens(&toks);
        let body = if parsed.label_colon.is_some() { parsed.tokens } else { toks };
        // Peel a leading bare label.
        let (kw_idx, kw) = match body.first() {
            Some(Token { tok: Tok::Ident(s), .. })
                if !is_op_keyword(s) && !is_mnemonic(s) && body.len() > 1 =>
            {
                match &body[1].tok {
                    Tok::Ident(s2) => (1usize, Some(s2.clone())),
                    _ => (0usize, Some(s.clone())),
                }
            }
            Some(Token { tok: Tok::Ident(s), .. }) => (0usize, Some(s.clone())),
            _ => (0usize, None),
        };
        let span = body.get(kw_idx).map(|t| t.span).unwrap_or(Span { source: self.source, start: line.base, end: line.base });
        let args = body.get(kw_idx + 1..).unwrap_or(&[]).to_vec();
        (kw, args, span)
    }

    /// Find the index of the terminator matching the block opened at `start`,
    /// depth-counting nested blocks. Returns the terminator index (or the last
    /// line index if unterminated).
    fn find_block_end(&self, lines: &[SrcLine], start: usize, openers: &[&str], closers: &[&str]) -> usize {
        let mut depth = 0i32;
        for (idx, line) in lines.iter().enumerate().skip(start) {
            let kw = self.line_keyword(line);
            if let Some(k) = kw.as_deref() {
                if idx == start || openers.contains(&k) {
                    depth += 1;
                } else if closers.contains(&k) {
                    depth -= 1;
                    if depth == 0 {
                        return idx;
                    }
                }
            }
        }
        lines.len().saturating_sub(1)
    }

    /// Execute an `if`/`ifdef`/`ifndef` … `endif` region; run the first true arm.
    /// Returns the index just past `endif`.
    fn exec_if(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let end = self.find_block_end(lines, start, &["if", "ifdef", "ifndef"], &["endif"]);
        // Collect arm-head indices at depth 0: start, then each elseif/else.
        let mut heads = vec![start];
        let mut depth = 0i32;
        for (idx, line) in lines.iter().enumerate().take(end).skip(start + 1) {
            match self.line_keyword(line).as_deref() {
                Some("if") | Some("ifdef") | Some("ifndef") => depth += 1,
                Some("endif") => depth -= 1,
                Some("elseif") | Some("else") if depth == 0 => heads.push(idx),
                _ => {}
            }
        }
        heads.push(end); // sentinel
        for w in 0..(heads.len() - 1) {
            let head = heads[w];
            let (kw, argtoks, span) = self.line_kw_args(&lines[head]);
            let take = match kw.as_deref() {
                Some("if") | Some("ifdef") | Some("ifndef") => self.eval_cond(kw.as_deref().unwrap(), &argtoks, span),
                Some("elseif") => self.eval_if_expr(&argtoks, span),
                Some("else") => true,
                _ => false,
            };
            if take {
                let body = &lines[head + 1..heads[w + 1]];
                self.exec(body);
                break;
            }
        }
        end + 1
    }

    /// Handle `rept N … endr`. `N` is folded once at the `rept` line (with `$` =
    /// the current phased VMA). Returns the index past `endr`.
    fn exec_rept(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let n = match self.eval_all(&arg_toks, span) {
            Some(v) if v >= 0 => v as usize,
            Some(_) => {
                self.err(span, "negative rept count");
                0
            }
            None => {
                self.err(span, "unresolved rept count");
                0
            }
        };
        let end = self.find_block_end(lines, start, &["rept"], &["endr", "endm"]);
        let body = &lines[start + 1..end];
        for _ in 0..n {
            self.exec(body);
        }
        end + 1
    }

    /// Handle `struct Name … endstruct`: define packed `Name_field` offsets and
    /// `Name_len`. Field lines emit no bytes. Returns the index past `endstruct`.
    fn capture_struct(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, args, span) = self.line_kw_args(&lines[start]);
        let name = match args.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "struct needs a name");
                String::new()
            }
        };
        let end = self.find_block_end(lines, start, &["struct"], &["endstruct"]);
        let mut off: i64 = 0;
        for l in &lines[start + 1..end] {
            if let Some((field, width, count)) = self.parse_struct_field(l) {
                self.env.define(&format!("{name}_{field}"), SymbolValue::Int(off));
                off += width * count;
            }
        }
        self.env.define(&format!("{name}_len"), SymbolValue::Int(off));
        end + 1
    }

    /// Parse a `<field> ds.b|ds.w|ds.l <count>` struct-member line.
    /// Returns `(field, width, count)`, or None for a blank/comment line.
    fn parse_struct_field(&mut self, line: &SrcLine) -> Option<(String, i64, i64)> {
        let toks = lex_line(&line.text, self.state.cpu, self.source, line.base).ok()?;
        if toks.is_empty() {
            return None;
        }
        let parsed = parse_line_tokens(&toks);
        let (field, rest): (String, Vec<Token>) = if let Some(l) = parsed.label_colon {
            (l, parsed.tokens)
        } else {
            match parsed.tokens.split_first() {
                Some((Token { tok: Tok::Ident(s), .. }, r)) => (s.clone(), r.to_vec()),
                _ => return None,
            }
        };
        let width = match rest.first().map(|t| &t.tok) {
            Some(Tok::Ident(w)) if w == "ds.b" => 1,
            Some(Tok::Ident(w)) if w == "ds.w" => 2,
            Some(Tok::Ident(w)) if w == "ds.l" => 4,
            _ => return None,
        };
        let span = rest[0].span;
        let count = self.eval_all(&rest[1..], span).unwrap_or(1);
        Some((field, width, count))
    }

    fn eval_cond(&mut self, kw: &str, arg_toks: &[Token], span: Span) -> bool {
        match kw {
            "ifdef" => self.cond_defined(arg_toks),
            "ifndef" => !self.cond_defined(arg_toks),
            _ => self.eval_if_expr(arg_toks, span),
        }
    }

    fn cond_defined(&self, arg_toks: &[Token]) -> bool {
        matches!(arg_toks.first().map(|t| &t.tok), Some(Tok::Ident(n)) if self.env.resolve(n, self.scope.as_deref()).is_some())
    }

    /// `if MOMCPUNAME="Z80"` / `<lhs>="str"` string equality, else numeric `!= 0`.
    fn eval_if_expr(&mut self, toks: &[Token], span: Span) -> bool {
        if let Some(pos) = toks.iter().position(|t| matches!(t.tok, Tok::Punct(Punct::Eq))) {
            if let Some(Token { tok: Tok::Str(rhs), .. }) = toks.get(pos + 1) {
                let lhs = self.string_value(&toks[..pos]);
                return lhs.as_deref() == Some(rhs.as_str());
            }
        }
        self.eval_all(toks, span).map(|v| v != 0).unwrap_or(false)
    }

    /// The string value of a builtin like MOMCPUNAME (else None).
    fn string_value(&self, toks: &[Token]) -> Option<String> {
        match toks {
            [Token { tok: Tok::Ident(n), .. }] if n == "MOMCPUNAME" => Some(match self.state.cpu {
                Cpu::Z80 => "Z80".into(),
                Cpu::M68000 => "68000".into(),
            }),
            _ => None,
        }
    }

    fn dispatch(&mut self, head: &str, rest: &[Token], span: Span) {
        match head {
            "cpu" => self.directive_cpu(rest, span),
            "phase" => self.directive_phase(rest, span),
            "dephase" => self.directive_dephase(),
            "save" => self.state.save(),
            "restore" => {
                if let Err(m) = self.state.restore() {
                    self.err(span, m);
                }
            }
            "padding" => self.state.padding = on_off(rest),
            "supmode" => self.state.supmode = on_off(rest),
            "db" | "dc.b" => self.directive_db(rest, span),
            "dw" => self.directive_dw(rest, span),
            _ if is_mnemonic(head) => self.lower_instruction(head, rest, span),
            _ => self.err(span, format!("unknown directive or mnemonic `{head}`")),
        }
    }

    fn open_section_if_needed(&mut self) {
        if !self.in_section {
            let name = format!("sec{}", self.state.vma_base.unwrap_or(0));
            // lma defaults to vma_base (IrBuilder); Plan 5's map assigns real
            // LMAs and handles same-phase section re-entry. Same-(cpu,vma)
            // re-entry within one assembly would currently collide at flatten —
            // out of the M0 single-region-per-phase gate.
            self.builder.switch_section(&name, self.state.cpu, self.state.vma_base);
            self.in_section = true;
        }
    }

    fn close_section(&mut self) {
        self.in_section = false;
    }

    fn define_label(&mut self, name: &str) {
        self.open_section_if_needed();
        let value = self.here() as i64;
        let qualified = if name.starts_with('.') {
            qualify(name, self.scope.as_deref())
        } else {
            self.scope = Some(name.to_string());
            name.to_string()
        };
        self.env.define(&qualified, SymbolValue::Int(value));
        self.builder.define_label(&qualified);
    }

    fn directive_cpu(&mut self, rest: &[Token], span: Span) {
        let name = match rest.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            Some(Tok::Int(_)) => "68000".to_string(),
            _ => {
                self.err(span, "cpu needs a name");
                return;
            }
        };
        let cpu = match name.as_str() {
            "z80" => Cpu::Z80,
            "68000" | "68008" => Cpu::M68000,
            other => {
                self.err(span, format!("unsupported cpu `{other}`"));
                return;
            }
        };
        self.state.cpu = cpu;
        self.close_section();
    }

    fn directive_phase(&mut self, rest: &[Token], span: Span) {
        match self.eval_all(rest, span) {
            Some(v) => {
                self.state.vma_base = Some(v as u32);
                self.close_section();
            }
            None => self.err(span, "phase needs a constant expression"),
        }
    }

    fn directive_dephase(&mut self) {
        self.state.vma_base = None;
        self.close_section();
    }

    fn directive_equate(&mut self, name: &str, rest: &[Token], span: Span) {
        if let Some(v) = self.eval_all(rest, span) {
            // An equate is not a label: qualify a local `.foo` against the
            // current scope (so `ld a,.foo` resolves) but do NOT open a scope.
            // `qualify` leaves non-dotted global names unchanged.
            let q = qualify(name, self.scope.as_deref());
            self.env.define(&q, SymbolValue::Int(v));
        }
    }

    fn directive_db(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let expanded = self.expand_calls(g);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad byte expression");
                    continue;
                }
            };
            let v = self.fold_imm(&e, span, -128, 0xFF);
            self.emit(&[v as u8], vec![], span);
        }
    }

    fn directive_dw(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            if let [Token { tok: Tok::Ident(name), .. }] = g {
                if !is_reg_or_cond_word(name) && !self.functions.contains_key(name) {
                    let target = Expr::Sym(qualify(name, self.scope.as_deref()));
                    self.emit(
                        &[0x00, 0x00],
                        vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target }],
                        span,
                    );
                    continue;
                }
            }
            let expanded = self.expand_calls(g);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad word expression");
                    continue;
                }
            };
            let v = self.fold_imm(&e, span, -0x8000, 0xFFFF) as u16;
            self.emit(&[(v & 0xFF) as u8, (v >> 8) as u8], vec![], span);
        }
    }

    fn lower_instruction(&mut self, mn: &str, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let m = match mnemonic(mn) {
            Some(m) => m,
            None => {
                self.err(span, "not a mnemonic");
                return;
            }
        };
        match self.build_operands(m, &atoms, span) {
            Some(Lowered::Fixed(ops)) => {
                let f = self.backend.lower(m, &ops, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Rel(cond, target)) => {
                let f = self.backend.lower_rel(m, cond, target, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Abs16(ops, target)) => {
                let f = self.backend.lower_abs16(m, &ops, target, span);
                self.emit_frag(f, span);
            }
            None => {}
        }
    }

    fn build_operands(&mut self, m: Mnemonic, atoms: &[OperandAtom], span: Span) -> Option<Lowered> {
        if matches!(m, Mnemonic::Jr | Mnemonic::Djnz) {
            let (cond, target_atom) = match atoms {
                [OperandAtom::RegOrCond(w), t] => (cond_word(w), t),
                [t] => (None, t),
                _ => {
                    self.err(span, "bad jr/djnz operands");
                    return None;
                }
            };
            let target = match target_atom {
                OperandAtom::Value(e) => self.qualify_expr(e),
                _ => {
                    self.err(span, "jr/djnz needs a label target");
                    return None;
                }
            };
            return Some(Lowered::Rel(cond, target));
        }
        if matches!(m, Mnemonic::Jp | Mnemonic::Call) {
            let (cond, target_opt) = self.split_control_target(atoms);
            if let Some(target @ Expr::Sym(_)) = target_opt {
                let mut ops = Vec::new();
                if let Some(cc) = cond {
                    ops.push(Operand::Cc(cc));
                }
                ops.push(Operand::Imm16(0));
                return Some(Lowered::Abs16(ops, target));
            }
        }
        if matches!(m, Mnemonic::Ld) {
            if let [OperandAtom::RegOrCond(w), OperandAtom::Value(e @ Expr::Sym(_))] = atoms {
                if let Some(rr) = reg16(w) {
                    let target = self.qualify_expr(e);
                    let ops = vec![Operand::Pair(rr), Operand::Imm16(0)];
                    return Some(Lowered::Abs16(ops, target));
                }
            }
        }
        let ops = self.convert_atoms(m, atoms, span)?;
        Some(Lowered::Fixed(ops))
    }

    /// For jp/call: split off a leading condition and return the bare target expr.
    fn split_control_target(&self, atoms: &[OperandAtom]) -> (Option<Cond>, Option<Expr>) {
        match atoms {
            [OperandAtom::RegOrCond(w), OperandAtom::Value(e)] if cond_word(w).is_some() => {
                (cond_word(w), Some(self.qualify_expr(e)))
            }
            [OperandAtom::Value(e)] => (None, Some(self.qualify_expr(e))),
            _ => (None, None),
        }
    }

    /// Qualify a bare local `.name` Sym against the current scope; else unchanged.
    fn qualify_expr(&self, e: &Expr) -> Expr {
        match e {
            Expr::Sym(name) if name.starts_with('.') => {
                Expr::Sym(qualify(name, self.scope.as_deref()))
            }
            other => other.clone(),
        }
    }

    /// Convert operand atoms to resolved z80 operands, by mnemonic.
    fn convert_atoms(&mut self, m: Mnemonic, atoms: &[OperandAtom], span: Span) -> Option<Vec<Operand>> {
        // M0 invariant: a 16-bit pair operand means the immediate is 16-bit (ld rr,nn). Holds for the driver's mnemonic set.
        let has_pair_companion = atoms
            .iter()
            .any(|a| matches!(a, OperandAtom::RegOrCond(w) if reg16(w).is_some()));
        let control_flow = matches!(m, Mnemonic::Jp | Mnemonic::Call | Mnemonic::Ret);
        let bit_op = matches!(m, Mnemonic::Bit | Mnemonic::Res | Mnemonic::Set);
        let mut ops = Vec::with_capacity(atoms.len());
        for (i, a) in atoms.iter().enumerate() {
            let op = match a {
                OperandAtom::RegOrCond(w) => {
                    if control_flow && i == 0 {
                        if let Some(cc) = cond_word(w) {
                            Operand::Cc(cc)
                        } else {
                            self.reg_operand(w, span)?
                        }
                    } else {
                        self.reg_operand(w, span)?
                    }
                }
                OperandAtom::IndReg(w) => match w.as_str() {
                    "hl" => Operand::IndHl,
                    "bc" => Operand::IndBc,
                    "de" => Operand::IndDe,
                    _ => {
                        self.err(span, "bad indirect register");
                        return None;
                    }
                },
                OperandAtom::Indexed { reg, disp } => {
                    let d = self.fold_imm(disp, span, -128, 127);
                    Operand::Indexed { reg: *reg, disp: d as i8 }
                }
                OperandAtom::Mem(e) => {
                    let v = self.fold_imm(e, span, 0, 0xFFFF);
                    Operand::Mem(v as u16)
                }
                OperandAtom::Value(e) => {
                    if bit_op && i == 0 {
                        let b = self.fold_imm(e, span, 0, 7);
                        Operand::Bit(b as u8)
                    } else if matches!(m, Mnemonic::Im) {
                        let v = self.fold_imm(e, span, 0, 2);
                        Operand::Imm8(v as u8)
                    } else if matches!(m, Mnemonic::Jp | Mnemonic::Call) {
                        // A literal address for jp/call is a 16-bit immediate
                        // (symbolic targets take the Abs16 fixup path earlier).
                        let v = self.fold_imm(e, span, 0, 0xFFFF);
                        Operand::Imm16(v as u16)
                    } else if has_pair_companion {
                        let v = self.fold_imm(e, span, -0x8000, 0xFFFF);
                        Operand::Imm16(v as u16)
                    } else {
                        let v = self.fold_imm(e, span, -128, 0xFF);
                        Operand::Imm8(v as u8)
                    }
                }
                OperandAtom::AfShadow => Operand::AfShadow,
            };
            ops.push(op);
        }
        Some(ops)
    }

    fn reg_operand(&mut self, w: &str, span: Span) -> Option<Operand> {
        if let Some(r) = reg8(w) {
            Some(Operand::Reg(r))
        } else if let Some(rr) = reg16(w) {
            Some(Operand::Pair(rr))
        } else if w == "i" {
            Some(Operand::RegI)
        } else if w == "r" {
            Some(Operand::RegR)
        } else if let Some(cc) = cond_word(w) {
            Some(Operand::Cc(cc))
        } else {
            self.err(span, format!("bad register/condition `{w}`"));
            None
        }
    }

    fn emit_frag(&mut self, frag: Result<DataFragment, LowerError>, span: Span) {
        match frag {
            Ok(f) => {
                let bytes = f.bytes.clone();
                self.emit(&bytes, f.fixups, span);
            }
            Err(e) => self.err(span, e.to_string()),
        }
    }

    fn emit(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span) {
        // The builder advances its own section cursor (the single source of
        // truth read back via `current_offset()`); the front-end keeps none.
        self.builder.emit_data(bytes, fixups, span);
    }
}

// ── free helpers ────────────────────────────────────────────────────────────

fn is_op_keyword(s: &str) -> bool {
    matches!(
        s,
        "cpu" | "phase" | "dephase" | "save" | "restore" | "padding" | "supmode"
            | "db" | "dw" | "dc.b" | "equ"
            | "if" | "elseif" | "else" | "endif" | "ifdef" | "ifndef"
            | "rept" | "endr" | "endm" | "macro" | "struct" | "endstruct"
            | "function" | "include" | "error" | "fatal" | "message"
            | "ds.b" | "ds.w" | "ds.l"
    )
}

fn is_mnemonic(s: &str) -> bool {
    mnemonic(s).is_some()
}

fn mnemonic(s: &str) -> Option<Mnemonic> {
    use Mnemonic::*;
    Some(match s {
        "nop" => Nop, "ld" => Ld, "add" => Add, "adc" => Adc, "sub" => Sub, "sbc" => Sbc,
        "and" => And, "or" => Or, "xor" => Xor, "cp" => Cp, "inc" => Inc, "dec" => Dec,
        "push" => Push, "pop" => Pop, "ex" => Ex, "exx" => Exx, "ret" => Ret, "jr" => Jr,
        "jp" => Jp, "call" => Call, "djnz" => Djnz, "rrca" => Rrca, "scf" => Scf,
        "ei" => Ei, "di" => Di, "bit" => Bit, "res" => Res, "set" => Set, "srl" => Srl,
        "rr" => Rr, "sla" => Sla, "rlc" => Rlc, "rrc" => Rrc, "rl" => Rl, "sra" => Sra,
        "neg" => Neg, "im" => Im, "ldir" => Ldir,
        _ => return None,
    })
}

fn cond_word(w: &str) -> Option<Cond> {
    use Cond::*;
    Some(match w {
        "nz" => Nz, "z" => Z, "nc" => Nc, "c" => C, "po" => Po, "pe" => Pe, "p" => P, "m" => M,
        _ => return None,
    })
}

fn reg8(w: &str) -> Option<Reg8> {
    use Reg8::*;
    Some(match w {
        "a" => A, "b" => B, "c" => C, "d" => D, "e" => E, "h" => H, "l" => L,
        _ => return None,
    })
}

fn reg16(w: &str) -> Option<Reg16> {
    use Reg16::*;
    Some(match w {
        "bc" => Bc, "de" => De, "hl" => Hl, "sp" => Sp, "af" => Af, "ix" => Ix, "iy" => Iy,
        _ => return None,
    })
}

fn is_reg_or_cond_word(w: &str) -> bool {
    reg8(w).is_some() || reg16(w).is_some() || cond_word(w).is_some() || w == "i" || w == "r"
}

/// Qualify a name: `.local` → `Scope.local` (if scope); else unchanged.
fn qualify(name: &str, scope: Option<&str>) -> String {
    if name.starts_with('.') {
        match scope {
            Some(s) => format!("{s}{name}"),
            None => name.to_string(),
        }
    } else {
        name.to_string()
    }
}

fn on_off(rest: &[Token]) -> bool {
    !matches!(rest.first().map(|t| &t.tok), Some(Tok::Ident(w)) if w == "off")
}

fn paren(p: Punct, span: Span) -> Token {
    Token { tok: Tok::Punct(p), span }
}

/// Given `toks` with a `(` at index `lparen`, split the argument groups by
/// depth-0 commas and return `(args, index_past_matching_rparen)`. None if unbalanced.
fn split_call_args(toks: &[Token], lparen: usize) -> Option<(Vec<Vec<Token>>, usize)> {
    let mut depth = 0i32;
    let mut i = lparen;
    let mut args: Vec<Vec<Token>> = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    while i < toks.len() {
        match &toks[i].tok {
            Tok::Punct(Punct::LParen) => {
                depth += 1;
                if depth > 1 {
                    cur.push(toks[i].clone());
                }
                i += 1;
            }
            Tok::Punct(Punct::RParen) => {
                depth -= 1;
                if depth == 0 {
                    args.push(cur);
                    return Some((args, i + 1));
                }
                cur.push(toks[i].clone());
                i += 1;
            }
            Tok::Punct(Punct::Comma) if depth == 1 => {
                args.push(std::mem::take(&mut cur));
                i += 1;
            }
            _ => {
                cur.push(toks[i].clone());
                i += 1;
            }
        }
    }
    None
}

/// Split a token slice on top-level (non-parenthesised) commas.
fn split_top_commas(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::Punct(Punct::LParen) => depth += 1,
            Tok::Punct(Punct::RParen) => depth -= 1,
            Tok::Punct(Punct::Comma) if depth == 0 => {
                groups.push(&toks[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    groups.push(&toks[start..]);
    groups
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::Options;
    use sigil_ir::backend::Cpu;

    fn image(src: &str) -> Vec<u8> {
        let m = run(src, &Options::default()).expect("assemble");
        m.sections.first().map(|s| s.image_bytes()).unwrap_or_default()
    }

    #[test]
    fn ifdef_gates_emission_by_define_set() {
        let src = "        cpu z80\n        phase 0\n        db 1\n        ifdef __DEBUG__\n        db 0FFh\n        endif\n        ifdef SOUND_DRIVER_ENABLED\n        db 2\n        endif\n";
        let opts = Options { initial_cpu: Cpu::Z80, defines: vec![("SOUND_DRIVER_ENABLED".into(), 1)] };
        let m = run(src, &opts).expect("assemble");
        let bytes = m.sections.first().map(|s| s.image_bytes()).unwrap_or_default();
        assert_eq!(bytes, vec![0x01, 0x02]);
    }

    #[test]
    fn if_elseif_else_takes_one_branch() {
        let src = "        cpu z80\n        phase 0\nX = 2\n        if X = 1\n        db 10h\n        elseif X = 2\n        db 20h\n        else\n        db 30h\n        endif\n";
        assert_eq!(image(src), vec![0x20]);
    }

    #[test]
    fn if_momcpuname_string_equality() {
        let src = "        cpu z80\n        phase 0\n        if MOMCPUNAME=\"Z80\"\n        db 0AAh\n        else\n        db 0BBh\n        endif\n";
        assert_eq!(image(src), vec![0xAA]);
    }

    #[test]
    fn nested_if_inside_taken_branch() {
        let src = "        cpu z80\n        phase 0\nX = 1\n        if X = 1\n        db 1\n        if X = 1\n        db 2\n        endif\n        db 3\n        endif\n";
        assert_eq!(image(src), vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn lowers_common_instructions() {
        let src = "        cpu z80\n        phase 0\n        nop\n        ld a,0Ch\n        ld b,c\n        add a,b\n        jp 1234h\n";
        assert_eq!(image(src), vec![0x00, 0x3E, 0x0C, 0x41, 0x80, 0xC3, 0x34, 0x12]);
    }

    #[test]
    fn db_dw_le_and_equate() {
        let src = "        cpu z80\n        phase 0\nGAP = 4\n        db 1,2,3\n        dw 0284h\n        db GAP\n";
        assert_eq!(image(src), vec![0x01, 0x02, 0x03, 0x84, 0x02, 0x04]);
    }

    #[test]
    fn local_equate_resolves_in_scope() {
        let src = "        cpu z80\n        phase 0\nScope:\n.k      = 5\n        ld a,.k\n";
        assert_eq!(image(src), vec![0x3E, 0x05]);
    }

    #[test]
    fn rept_dollar_gap_fill() {
        // 3 nops (0x00), then fill to phased VMA 8 with `db 0` ⇒ 8 total bytes.
        let src = "        cpu z80\n        phase 0\n        nop\n        nop\n        nop\n        rept 8-$\n        db 0\n        endr\n";
        assert_eq!(image(src), vec![0x00; 8]);
    }

    #[test]
    fn rept_constant_count() {
        let src = "        cpu z80\n        phase 0\n        rept 3\n        db 0AAh\n        endr\n";
        assert_eq!(image(src), vec![0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn functions_fold_including_truncating_div() {
        let src = concat!(
            "        cpu z80\n        phase 0\n",
            "SFX_WIN_MASK = 32767\n",
            "SFX_WIN_BASE = 32768\n",
            "function sfx_winptr(addr) = ((addr) & SFX_WIN_MASK) | SFX_WIN_BASE\n",
            "function sfx_bankid(addr) = (addr) >> 15\n",
            "function timerAReload(hz) = 1024 - (1000000000 / ((hz) * 18773))\n",
            "Sfx_33   = 0D69Ah\n",
            "        dw sfx_winptr(Sfx_33)\n",
            "        db sfx_bankid(0C0000h)\n",
            "        db timerAReload(59)\n",
        );
        // sfx_winptr(0xD69A)=(0xD69A&0x7FFF)|0x8000=0xD69A → LE 9A D6
        // sfx_bankid(0xC0000)=0xC0000>>15=0x18 ; timerAReload(59)=122=0x7A
        assert_eq!(image(src), vec![0x9A, 0xD6, 0x18, 0x7A]);
    }

    #[test]
    fn struct_offsets_and_len_drive_indexed_disp() {
        // Packed: a(1) b(1) c(2) → a=0 b=1 c=2 len=4. Then (ix+SeqChannel_b) = (ix+1).
        let src = "        cpu z80\n        phase 0\n        struct SeqChannel\na       ds.b 1\nb       ds.b 1\nc       ds.w 1\n        endstruct\n        ld a,(ix+SeqChannel_b)\n        db SeqChannel_len\n";
        // ld a,(ix+1) = DD 7E 01 ; db 4 = 04
        assert_eq!(image(src), vec![0xDD, 0x7E, 0x01, 0x04]);
    }
}
