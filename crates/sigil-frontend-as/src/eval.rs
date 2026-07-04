//! eval: the driver — line loop, directive dispatch, instruction lowering, emit.

use crate::expand::{render_tokens, replace_word, split_call_args, split_top_commas};
use crate::lexer::lex_line;
use crate::operands::{parse_operands, OperandAtom};
use crate::parser::parse_line_tokens;
use crate::token::{Punct, Tok, Token};
use crate::Options;
use sigil_backend_m68k::m68k::{
    Cond as M68kCond, Instruction as M68kInstruction, Mnemonic as M68kMnemonic, Operand as M68kOperand,
    Size as M68kSize, Xn as M68kXn,
};
use sigil_backend_m68k::M68kBackend;
use sigil_backend_z80::z80::{Cond, Mnemonic, Operand, Reg16, Reg8};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer, LowerError};
use sigil_ir::expr::{BinOp, Fold};
use sigil_ir::{DataFragment, Expr, Fixup, FixupKind, IrBuilder, Module, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level, SourceId, Span};

const EXPAND_CAP: usize = 64;
const PASS_CAP: usize = 8;

#[derive(Clone)]
struct SrcLine {
    text: String,
    base: u32,
}

/// Collected macro definitions: name → (params, body lines).
type MacroTable = std::collections::BTreeMap<String, (Vec<String>, Vec<SrcLine>)>;
/// Collected function definitions: name → (params, body tokens).
type FunctionTable = std::collections::BTreeMap<String, (Vec<String>, Vec<Token>)>;

pub fn run(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    // Seed pass 0 with the provided defines; each later pass is seeded with the
    // previous pass's discovered symbols so forward references resolve. Macro and
    // function definitions are carried forward too, so an `ifndef`-guarded
    // definition collected on pass 0 stays available on later passes when its
    // guard symbol suppresses re-collection.
    let mut seed = SymbolTable::new();
    for (k, v) in &opts.defines {
        seed.define(k, SymbolValue::Int(*v));
    }
    let mut macros = MacroTable::new();
    let mut functions = FunctionTable::new();
    let mut prev = seed.clone();
    for pass in 0..PASS_CAP {
        let PassOutput { module, env, macros: m, functions: f, mut diags, poison } =
            one_pass(src, opts, &seed, &macros, &functions);
        if pass > 0 && env == prev {
            // Converged: this pass's result is authoritative. The env is now final,
            // so any operand that still folded to Poison references a genuinely
            // undefined symbol — promote each to a hard error (a missing stub /
            // typo that would otherwise have silently emitted a 0x00 byte).
            for (name, span) in poison {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("unresolved symbol `{name}` in operand"),
                    primary: span,
                });
            }
            return if diags.iter().any(|d| d.level == Level::Error) {
                Err(diags)
            } else {
                Ok(module)
            };
        }
        prev = env.clone();
        seed = env;
        macros = m;
        functions = f;
    }
    Err(vec![Diagnostic {
        level: Level::Error,
        message: format!("assembly did not converge within {PASS_CAP} passes (symbol values still changing)"),
        primary: Span { source: SourceId(0), start: 0, end: 0 },
    }])
}

/// The outputs of a single assembly pass.
struct PassOutput {
    module: Module,
    env: SymbolTable,
    macros: MacroTable,
    functions: FunctionTable,
    diags: Vec<Diagnostic>,
    /// Operand symbols that folded to Poison this pass (name + site span).
    poison: Vec<(String, Span)>,
}

/// One assembly pass seeded with `seed_env` (symbols) plus the macro/function
/// definition tables from prior passes. Returns the module, the discovered
/// symbol table, the (possibly extended) definition tables, diagnostics, and the
/// unresolved-operand references seen this pass.
fn one_pass(
    src: &str,
    opts: &Options,
    seed_env: &SymbolTable,
    seed_macros: &MacroTable,
    seed_functions: &FunctionTable,
) -> PassOutput {
    let mut asm = Asm::new(opts);
    asm.env = seed_env.clone();
    asm.macros = seed_macros.clone();
    asm.functions = seed_functions.clone();
    asm.process(src);
    let (module, mut diags) = asm.builder.finish();
    diags.append(&mut asm.diags);
    PassOutput {
        module,
        env: asm.env,
        macros: asm.macros,
        functions: asm.functions,
        diags,
        poison: asm.poison_refs,
    }
}

struct Asm {
    builder: IrBuilder,
    z80: Z80Backend,
    m68k: M68kBackend,
    state: crate::state::AsmState,
    env: SymbolTable,
    scope: Option<String>,
    in_section: bool,
    diags: Vec<Diagnostic>,
    source: SourceId,
    functions: FunctionTable,
    macros: MacroTable,
    macro_depth: usize,
    visited: std::collections::BTreeSet<std::path::PathBuf>,
    include_root: Option<std::path::PathBuf>,
    aborted: bool,
    /// Operand symbols that folded to Poison this pass (name + site span). On an
    /// intermediate pass these are just not-yet-resolved forward refs; on the
    /// CONVERGED pass the env is final, so any entry here is genuinely undefined
    /// and `run` promotes it to an error.
    poison_refs: Vec<(String, Span)>,
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
            z80: Z80Backend,
            m68k: M68kBackend,
            state: crate::state::AsmState::new(opts.initial_cpu),
            env: SymbolTable::new(),
            scope: None,
            in_section: false,
            diags: Vec::new(),
            source: SourceId(0),
            functions: std::collections::BTreeMap::new(),
            macros: std::collections::BTreeMap::new(),
            macro_depth: 0,
            visited: std::collections::BTreeSet::new(),
            include_root: opts.include_root.clone(),
            aborted: false,
            poison_refs: Vec::new(),
        }
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Error, message: msg.into(), primary: span });
    }

    fn here(&self) -> u32 {
        // When no section is open (just after phase/dephase/cpu closed one and
        // before the next emit reopens it), the new region has emitted 0 bytes.
        self.state.vma_base.unwrap_or(0)
            + if self.in_section { self.builder.current_offset() } else { 0 }
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
    /// Unresolved (Poison) → 0 placeholder for THIS pass (byte-stable so a forward
    /// ref that resolves on a later pass doesn't perturb layout), but the offending
    /// symbol names are recorded: on the converged pass `run` promotes them to
    /// unresolved-symbol errors (the env is final there, so a still-Poison operand
    /// is genuinely undefined rather than a pending forward ref).
    fn fold_imm(&mut self, e: &Expr, span: Span, lo: i64, hi: i64) -> i64 {
        match self.fold(e) {
            Fold::Value(v) if v >= lo && v <= hi => v,
            Fold::Value(v) => {
                self.err(span, format!("operand {v} out of range {lo}..={hi}"));
                v.clamp(lo, hi)
            }
            Fold::Poison => {
                for name in self.unresolved_names(e) {
                    self.poison_refs.push((name, span));
                }
                0
            }
        }
    }

    /// Collect the symbol names in `e` that do NOT resolve in the current env
    /// (ignoring `$`, which `fold` handles specially). These are the names that
    /// made an operand fold to Poison.
    fn unresolved_names(&self, e: &Expr) -> Vec<String> {
        fn walk(this: &Asm, e: &Expr, out: &mut Vec<String>) {
            match e {
                Expr::Int(_) => {}
                Expr::Sym(name) => {
                    if name != "$" && this.env.resolve(name, this.scope.as_deref()).is_none() {
                        out.push(name.clone());
                    }
                }
                Expr::Binary { lhs, rhs, .. } => {
                    walk(this, lhs, out);
                    walk(this, rhs, out);
                }
                Expr::Unary { operand, .. } => walk(this, operand, out),
            }
        }
        let mut out = Vec::new();
        walk(self, e, &mut out);
        // A name can appear more than once in one operand (e.g. `X+X`); report it once.
        out.sort();
        out.dedup();
        out
    }

    /// Fold a whole token slice as one constant expression (used by phase, etc.).
    fn eval_all(&mut self, toks: &[Token], span: Span) -> Option<i64> {
        let expanded = self.expand_calls(toks, 0);
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

    /// Parse a name-first AS `function` definition and store it.
    ///
    /// Real AS / aeon syntax: `<name> function <formal_args...>, <body_expr>`, e.g.
    /// `timerAReload function mhz, 1024 - (1000000000000 / ((mhz) * 18773))`.
    /// The comma-separated items after `function` are the formal parameters,
    /// except the LAST, which is the body expression. (In aeon every function has
    /// exactly one formal, but this handles any arity.)
    fn def_function(&mut self, line: &SrcLine) {
        let toks = match lex_line(&line.text, self.state.cpu, self.source, line.base) {
            Ok(t) => t,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        // toks[0] = name, toks[1] = `function`, toks[2..] = formals..., body.
        let span = toks.first().map(|t| t.span).unwrap_or(Span { source: self.source, start: line.base, end: line.base });
        let name = match toks.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "function needs a name");
                return;
            }
        };
        if !matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if s == "function") {
            self.err(span, "function needs the `function` keyword");
            return;
        }
        let groups = split_top_commas(&toks[2..]);
        // Need at least one formal group plus the body group.
        if groups.len() < 2 || groups.last().map(|g| g.is_empty()).unwrap_or(true) {
            self.err(span, "function needs `<params...>, <body>`");
            return;
        }
        let body = groups[groups.len() - 1].to_vec();
        let mut params = Vec::new();
        for g in &groups[..groups.len() - 1] {
            match g {
                [Token { tok: Tok::Ident(p), .. }] => params.push(p.clone()),
                _ => {
                    self.err(span, "bad function parameter");
                    return;
                }
            }
        }
        self.functions.insert(name, (params, body));
    }

    /// Expand every known-function call `fname(args)` in `toks` into its
    /// parenthesised, parameter-substituted body (recursively). Unknown `Ident(`
    /// is left untouched (it may be a `(nn)`-style group, not a call).
    fn expand_calls(&self, toks: &[Token], depth: usize) -> Vec<Token> {
        if depth > EXPAND_CAP {
            return toks.to_vec();
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if let Some((params, body)) = self.functions.get(name) {
                    if matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen))) {
                        if let Some((args, next)) = split_call_args(toks, i + 1) {
                            let expanded = self.substitute(body, params, &args, depth);
                            let span = toks[i].span;
                            out.push(paren(Punct::LParen, span));
                            out.extend(self.expand_calls(&expanded, depth + 1));
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
    fn substitute(&self, body: &[Token], params: &[String], args: &[Vec<Token>], depth: usize) -> Vec<Token> {
        let mut out = Vec::new();
        for t in body {
            if let Tok::Ident(name) = &t.tok {
                if let Some(idx) = params.iter().position(|p| p == name) {
                    if let Some(arg) = args.get(idx) {
                        let expanded_arg = self.expand_calls(arg, depth + 1);
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
        let lines = split_src_lines(src);
        self.exec(&lines);
    }

    /// Fold `\{expr}` sequences in the first string token to their decimal value.
    fn interp_string(&mut self, rest: &[Token]) -> String {
        let raw = match rest.iter().find_map(|t| if let Tok::Str(s) = &t.tok { Some(s.clone()) } else { None }) {
            Some(s) => s,
            None => return String::new(),
        };
        let mut out = String::new();
        let mut cur = raw.as_str();
        while let Some(pos) = cur.find("\\{") {
            out.push_str(&cur[..pos]);
            let after = &cur[pos + 2..];
            match after.find('}') {
                Some(end) => {
                    let expr_text = &after[..end];
                    match self.fold_text(expr_text) {
                        Some(v) => out.push_str(&v.to_string()),
                        None => {
                            out.push_str("\\{");
                            out.push_str(expr_text);
                            out.push('}');
                        }
                    }
                    cur = &after[end + 1..];
                }
                None => {
                    out.push_str("\\{");
                    cur = after;
                }
            }
        }
        out.push_str(cur);
        out
    }

    /// Lex + fold a short expression string (for `\{…}` interpolation).
    fn fold_text(&mut self, text: &str) -> Option<i64> {
        let toks = lex_line(text, self.state.cpu, self.source, 0).ok()?;
        self.eval_all(&toks, Span { source: self.source, start: 0, end: 0 })
    }

    /// `include "path"`: read a file relative to `include_root`, exec its lines
    /// inline. A visited-set prevents re-inclusion (DAG, not tree).
    fn directive_include(&mut self, rest: &[Token], span: Span) {
        let rel = match rest.iter().find_map(|t| if let Tok::Str(s) = &t.tok { Some(s.clone()) } else { None }) {
            Some(p) => p,
            None => {
                self.err(span, "include needs a quoted path");
                return;
            }
        };
        let path = match &self.include_root {
            Some(root) => root.join(&rel),
            None => std::path::PathBuf::from(&rel),
        };
        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !self.visited.insert(canon) {
            return; // already included (DAG guard)
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let lines = split_src_lines(&text);
                self.exec(&lines);
            }
            Err(e) => self.err(span, format!("cannot include {}: {e}", path.display())),
        }
    }

    /// Execute a slice of logical lines in order, handling block directives.
    fn exec(&mut self, lines: &[SrcLine]) {
        let mut i = 0;
        while i < lines.len() {
            if self.aborted {
                return;
            }
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
                Some("macro") => {
                    i = self.capture_macro(lines, i);
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
        if !is_op_keyword(&head) && !is_mnemonic(&head) && !self.macros.contains_key(&head) {
            // Under 68000 there is no mnemonic table yet (M1.C T4/T5), so
            // `is_mnemonic` (Z80-only) cannot tell a bare label from an
            // instruction. Fall back to AS's column rule: a bare label (no
            // colon) sits at column 0; an instruction is indented. A colon
            // label was already stripped above, so any remaining head on such a
            // line is an instruction regardless of column. Head token column =
            // `span.start - line.base` (see lex_line: span.start = base + col).
            if self.state.cpu == Cpu::M68000 {
                let indented = body[0].span.start > line.base;
                if parsed.label_colon.is_some() || indented {
                    self.dispatch(&head, &body[1..], body[0].span);
                    return;
                }
            }
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

    /// The routing keyword of a line, its index within `body`, and `body` (the
    /// tokens after any colon-label). Rules, in order:
    ///  1. second token is `macro`/`struct`/`function` ⇒ that keyword (a DEFINITION,
    ///     regardless of whether the name is already known — so re-executed
    ///     definition lines route correctly across passes).
    ///  2. the leading name is a known macro ⇒ the name (an INVOCATION; its args,
    ///     even if they look like keywords, are not block openers).
    ///  3. the leading name is a directive/mnemonic ⇒ the name.
    ///  4. a bare label followed by an Ident ⇒ that following Ident (e.g. `Tab db 0`).
    ///  5. otherwise ⇒ the leading name.
    fn dispatch_head(&self, line: &SrcLine) -> Option<(String, usize, Vec<Token>)> {
        let toks = lex_line(&line.text, self.state.cpu, self.source, line.base).ok()?;
        if toks.is_empty() {
            return None;
        }
        let parsed = parse_line_tokens(&toks);
        let body = if parsed.label_colon.is_some() { parsed.tokens } else { toks };
        if body.is_empty() {
            return None;
        }
        let name = match &body[0].tok {
            Tok::Ident(s) => s.clone(),
            _ => return None,
        };
        let second = body.get(1).and_then(|t| if let Tok::Ident(s) = &t.tok { Some(s.as_str()) } else { None });
        if matches!(second, Some("macro") | Some("struct") | Some("function")) {
            return Some((second.unwrap().to_string(), 1, body));
        }
        if self.macros.contains_key(&name) {
            return Some((name, 0, body));
        }
        if is_op_keyword(&name) || is_mnemonic(&name) {
            return Some((name, 0, body));
        }
        if let Some(Token { tok: Tok::Ident(s), .. }) = body.get(1) {
            return Some((s.clone(), 1, body));
        }
        Some((name, 0, body))
    }

    /// The dispatch keyword of a line (after peeling an optional label), or None
    /// for a blank/label-only/lex-error line.
    fn line_keyword(&self, line: &SrcLine) -> Option<String> {
        self.dispatch_head(line).map(|(kw, _, _)| kw)
    }

    /// The keyword + the tokens after it + the keyword span, for a block head.
    fn line_kw_args(&self, line: &SrcLine) -> (Option<String>, Vec<Token>, Span) {
        let fallback = Span { source: self.source, start: line.base, end: line.base };
        match self.dispatch_head(line) {
            Some((kw, idx, body)) => {
                let span = body.get(idx).map(|t| t.span).unwrap_or(fallback);
                let args = body.get(idx + 1..).unwrap_or(&[]).to_vec();
                (Some(kw), args, span)
            }
            None => (None, Vec::new(), fallback),
        }
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

    /// Handle name-first `Name struct … Name endstruct`: define packed
    /// `Name_field` offsets and `Name_len`. Field lines emit no bytes. Returns the
    /// index past `endstruct`. (Mirrors `capture_macro`: name at `toks[0]`,
    /// `struct` at `toks[1]`.)
    fn capture_struct(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let toks = lex_line(&lines[start].text, self.state.cpu, self.source, lines[start].base)
            .unwrap_or_default();
        let span = toks.first().map(|t| t.span).unwrap_or(Span { source: self.source, start: lines[start].base, end: lines[start].base });
        let name = match toks.first().map(|t| &t.tok) {
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
                // asl-verified, and it depends on the `padding` state: with
                // `padding on` (asl's default), a `ds.w`/`ds.l` field
                // (width >= 2) pads the running offset up to the next even
                // address once it's placed (the field's own start is NOT
                // pre-aligned, only the offset that follows). With
                // `padding off` — which Aeon sets globally at the top of
                // main.asm — there is NO rounding; the naive running offset
                // is used. Probed against real asl with
                // `a ds.b 1 / b ds.w 1 / c ds.b 1`:
                //   padding on  -> a=0 b=1 c=4 len=5
                //   padding off -> a=0 b=1 c=3 len=4  (Aeon's real layout).
                if self.state.padding && width >= 2 && off % 2 != 0 {
                    off += 1;
                }
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

    /// `if MOMCPUNAME="Z80"` / `<lhs>="str"` / `"a"="a"` / `"a"<>"b"` string
    /// (in)equality, else numeric `!= 0`. Strings never enter `sigil_ir::Expr`
    /// (§7.4: no AS-specific concept in IR) — the shape is detected and folded
    /// to a bool directly here, before any numeric `Expr` is built.
    fn eval_if_expr(&mut self, toks: &[Token], span: Span) -> bool {
        if let Some(pos) = toks
            .iter()
            .position(|t| matches!(t.tok, Tok::Punct(Punct::Eq) | Tok::Punct(Punct::Ne)))
        {
            if let Some(Token { tok: Tok::Str(rhs), .. }) = toks.get(pos + 1) {
                let lhs = match &toks[..pos] {
                    [Token { tok: Tok::Str(s), .. }] => Some(s.clone()),
                    other => self.string_value(other),
                };
                if let Some(lhs) = lhs {
                    let eq = lhs == *rhs;
                    let is_ne = matches!(toks[pos].tok, Tok::Punct(Punct::Ne));
                    return if is_ne { !eq } else { eq };
                }
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
            "dc.w" => self.directive_dc_w(rest, span),
            "dc.l" => self.directive_dc_l(rest, span),
            "ds.b" => self.directive_ds(1, rest, span),
            "ds.w" => self.directive_ds(2, rest, span),
            "ds.l" => self.directive_ds(4, rest, span),
            "align" => self.directive_align(rest, span),
            "error" => {
                let m = self.interp_string(rest);
                self.err(span, m);
            }
            "fatal" => {
                let m = self.interp_string(rest);
                self.err(span, m);
                self.aborted = true;
            }
            "message" => {
                let _ = self.interp_string(rest);
            }
            "include" => self.directive_include(rest, span),
            _ if self.macros.contains_key(head) => self.expand_macro(head, rest),
            // `is_mnemonic` only recognizes Z80 mnemonics; under `cpu 68000` the
            // m68k dispatch (lower_m68k) is still a stub (M1.C T4/T5), so any
            // non-directive head is routed there rather than misreported as
            // "unknown directive or mnemonic".
            _ if self.state.cpu == Cpu::Z80 && is_mnemonic(head) => self.lower_instruction(head, rest, span),
            _ if self.state.cpu == Cpu::M68000 => self.lower_instruction(head, rest, span),
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
            let expanded = self.expand_calls(g, 0);
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
            let expanded = self.expand_calls(g, 0);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad word expression");
                    continue;
                }
            };
            let qe = self.qualify_expr(&e);
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let w = v as u16;
                    self.emit(&[(w & 0xFF) as u8, (w >> 8) as u8], vec![], span);
                }
                Fold::Poison => {
                    // A bare unresolved symbol defers to the linker as a
                    // little-endian address fixup; a compound unresolved
                    // expression is a real error (byte-stable placeholder).
                    if matches!(qe, Expr::Sym(_)) {
                        self.emit(
                            &[0x00, 0x00],
                            vec![Fixup { kind: FixupKind::BankPtr16Le, offset: 0, target: qe }],
                            span,
                        );
                    } else {
                        self.err(span, "unresolved word expression");
                        self.emit(&[0x00, 0x00], vec![], span);
                    }
                }
            }
        }
    }

    /// `dc.w <expr>,...` — big-endian 16-bit words (asl: BE, unlike the Z80
    /// `dw`'s little-endian). Mirrors `directive_dw`'s expr-list parsing.
    fn directive_dc_w(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let expanded = self.expand_calls(g, 0);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad word expression");
                    continue;
                }
            };
            let qe = self.qualify_expr(&e);
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let w = (v as u16).to_be_bytes();
                    self.emit(&w, vec![], span);
                }
                Fold::Poison => {
                    if matches!(qe, Expr::Sym(_)) {
                        self.emit(
                            &[0x00, 0x00],
                            vec![Fixup { kind: FixupKind::Abs16Be, offset: 0, target: qe }],
                            span,
                        );
                    } else {
                        self.err(span, "unresolved word expression");
                        self.emit(&[0x00, 0x00], vec![], span);
                    }
                }
            }
        }
    }

    /// `dc.l <expr>,...` — big-endian 32-bit longwords.
    fn directive_dc_l(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let expanded = self.expand_calls(g, 0);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad long expression");
                    continue;
                }
            };
            let qe = self.qualify_expr(&e);
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let l = (v as u32).to_be_bytes();
                    self.emit(&l, vec![], span);
                }
                Fold::Poison => {
                    if matches!(qe, Expr::Sym(_)) {
                        self.emit(
                            &[0x00, 0x00, 0x00, 0x00],
                            vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: qe }],
                            span,
                        );
                    } else {
                        self.err(span, "unresolved long expression");
                        self.emit(&[0x00, 0x00, 0x00, 0x00], vec![], span);
                    }
                }
            }
        }
    }

    /// `ds.b`/`ds.w`/`ds.l <count>` — reserve `count * unit` bytes with no
    /// image bytes (verified against asl: a `ds` run with nothing emitted
    /// after it never materializes in the flat binary — matches
    /// `Fragment::Reserve`, not a real `Fill`).
    fn directive_ds(&mut self, unit: u32, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.eval_all(rest, span) {
            Some(v) if v >= 0 => self.builder.reserve(v as u32 * unit, span),
            Some(_) => self.err(span, "negative ds count"),
            None => self.err(span, "unresolved ds count"),
        }
    }

    /// `align <n>` — pad with zero bytes up to the next multiple of `n`
    /// (verified against asl: fill byte is `0x00`; a no-op when already
    /// aligned). Unlike `ds`, real Aeon usage always aligns something that
    /// follows in the same section, so this pads via a real `Fill` (visible
    /// zero bytes), matching asl's observed behavior when writes follow.
    fn directive_align(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.eval_all(rest, span) {
            Some(n) if n > 0 => {
                let n = n as u32;
                let pos = self.here();
                let pad = (n - (pos % n)) % n;
                if pad > 0 {
                    self.builder.emit_fill(pad, 0, span);
                }
            }
            Some(_) => self.err(span, "align needs a positive constant"),
            None => self.err(span, "unresolved align constant"),
        }
    }

    fn lower_instruction(&mut self, mn: &str, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.state.cpu {
            Cpu::Z80 => self.lower_z80(mn, rest, span),
            Cpu::M68000 => self.lower_m68k(mn, rest, span),
        }
    }

    fn lower_z80(&mut self, mn: &str, rest: &[Token], span: Span) {
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
                let f = self.z80.lower(m, &ops, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Rel(cond, target)) => {
                let f = self.z80.lower_rel(m, cond, target, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Abs16(ops, target)) => {
                let f = self.z80.lower_abs16(m, &ops, target, span);
                self.emit_frag(f, span);
            }
            None => {}
        }
    }

    /// M1.C T4/T5/T5b/T5c: the 68000 core. Straight-line register/immediate
    /// forms, the fixed-length register-indirect EA family, `lea`/`pea`, and
    /// explicit-width absolute addressing fold immediately (no fixups). T5c
    /// adds control transfer, each routed BEFORE the generic fold-based path
    /// because its target is resolved later (by the linker), not by this
    /// pass's fold:
    ///  - `bra`/`bsr`/`Bcc` → [`Self::lower_m68k_branch`] (size-pinned by the
    ///    `.s`/`.w` suffix, no relaxation).
    ///  - `Dbcc` (`dbf`/`dbra`/`db<cc>`) → [`Self::lower_m68k_dbcc`] (the
    ///    displacement FOLDS immediately — see that method's doc for why
    ///    that's safe).
    ///  - `jmp`/`jsr` with a bare symbol/expression target → a
    ///    `Fragment::JmpJsrSym` (width chosen later by the linker's
    ///    `resolve_layout`); `jmp`/`jsr` with an EA operand (e.g. `(a0)`)
    ///    falls through to the generic path like any other instruction.
    ///  - `(d16,PC)` operands (any mnemonic) → [`Self::lower_m68k_pcrel`].
    ///
    /// Only `movem`/`movep` remain out of scope — see `m68k_out_of_scope`.
    fn lower_m68k(&mut self, mn: &str, rest: &[Token], span: Span) {
        let (base, suffix_size) = split_mnemonic_and_size(mn);
        let mnemonic = match m68k_mnemonic(base) {
            Some(m) => m,
            None => {
                match m68k_out_of_scope(base) {
                    Some(family) => self.err(span, format!("`{base}` ({family}) is not yet implemented")),
                    None => self.err(span, format!("`{base}` is not a recognized 68000 mnemonic")),
                }
                return;
            }
        };

        if matches!(mnemonic, M68kMnemonic::Bra | M68kMnemonic::Bsr | M68kMnemonic::Bcc(_)) {
            return self.lower_m68k_branch(mnemonic, suffix_size, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Dbcc(_)) {
            return self.lower_m68k_dbcc(mnemonic, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Jmp | M68kMnemonic::Jsr) {
            let atoms = match parse_operands(rest) {
                Ok(a) => a,
                Err(d) => {
                    self.diags.push(d);
                    return;
                }
            };
            // A bare symbol/expression target (no EA parens) is 68k absolute
            // addressing whose WIDTH (abs.w vs abs.l) is chosen later by the
            // linker's `resolve_layout` — see `sigil-backend-m68k`'s
            // `lower_jmp_jsr_sym` doc. An EA operand (`(a0)`, `(Label).w`,
            // `(d16,PC)`, ...) falls through to the generic path below.
            if let [OperandAtom::Value(e)] = atoms.as_slice() {
                let target = self.qualify_expr(e);
                let is_jsr = matches!(mnemonic, M68kMnemonic::Jsr);
                let frag = self.m68k.lower_jmp_jsr_sym(is_jsr, target, span);
                // The baseline (all-abs.w) width is 4 bytes; `resolve_layout`
                // assumes THIS baseline when shifting subsequent label
                // offsets (see `sigil-link/src/relax.rs::shift_breakpoints`),
                // so the cursor must advance by exactly 4 here regardless of
                // the eventual real width.
                self.builder.emit_fragment(frag, 4);
                return;
            }
            return self.lower_m68k_generic(mnemonic, suffix_size, atoms, span);
        }

        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        self.lower_m68k_generic(mnemonic, suffix_size, atoms, span);
    }

    /// The shared tail of `lower_m68k` for every mnemonic that does NOT need
    /// its own special-cased target handling: resolve the size, detect (and
    /// deflect to [`Self::lower_m68k_pcrel`]) a `(d16,PC)` operand, else
    /// convert every atom and fold-lower the instruction directly. Also the
    /// fallback for `jmp`/`jsr` once an EA-operand form (not a bare symbol)
    /// has been ruled out by the caller.
    fn lower_m68k_generic(&mut self, mnemonic: M68kMnemonic, suffix_size: Option<M68kSize>, atoms: Vec<OperandAtom>, span: Span) {
        let size = match suffix_size.or_else(|| m68k_default_size(mnemonic)) {
            Some(s) => s,
            None => {
                self.err(span, "instruction needs an explicit size suffix (.b/.w/.l)".to_string());
                return;
            }
        };
        if let Some(pc_idx) = atoms
            .iter()
            .position(|a| matches!(a, OperandAtom::M68kDisp { an, .. } if an == "pc"))
        {
            return self.lower_m68k_pcrel(mnemonic, size, &atoms, pc_idx, span);
        }
        let ops = match self.convert_atoms_m68k(mnemonic, size, &atoms, span) {
            Some(o) => o,
            None => return,
        };
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction { mnemonic, size, ops };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// `bra`/`bsr`/`Bcc <target>`: Aeon pins the branch width by an explicit
    /// `.s`/`.w` suffix (no relaxation), so `suffix_size` MUST be present and
    /// MUST be `S` or `W`. The target is qualified (`.local` → `Scope.local`)
    /// and `$`-resolved, then handed to the backend's `lower_branch`, which
    /// builds the opcode + a `PcRel8`/`PcRelDisp16` fixup for the linker.
    fn lower_m68k_branch(&mut self, mnemonic: M68kMnemonic, suffix_size: Option<M68kSize>, rest: &[Token], span: Span) {
        let size = match suffix_size {
            Some(s @ (M68kSize::S | M68kSize::W)) => s,
            Some(_) => {
                self.err(span, "branch size suffix must be `.s` or `.w`");
                return;
            }
            None => {
                self.err(span, "branch needs an explicit size suffix (.s or .w) — Aeon pins branch width, no relaxation");
                return;
            }
        };
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let target = match atoms.as_slice() {
            [OperandAtom::Value(e)] => self.resolve_dollar(&self.qualify_expr(e)),
            _ => {
                self.err(span, "branch needs a single label target");
                return;
            }
        };
        let frag = self.m68k.lower_branch(mnemonic, size, target, span);
        self.emit_frag(frag, span);
    }

    /// `Dbcc Dn,target` (`dbf`/`dbra`/`db<cc>`): always a fixed 4-byte
    /// instruction (opcode word + a 16-bit displacement word) — NEVER
    /// relaxed, unlike `jmp`/`jsr`'s abs.w/abs.l choice. Because the byte
    /// width never depends on the resolved displacement, the displacement
    /// can safely FOLD immediately here (through the front-end's normal
    /// multi-pass symbol convergence) rather than deferring to a linker
    /// fixup — a forward reference just resolves on a later pass, the same
    /// way a forward `equ` does, and the placeholder `0` byte-pattern is
    /// stable meanwhile. asl measures the displacement from the extension
    /// word's own address (`instruction_start + 2` — the same PC-ref
    /// convention `FixupKind::PcRelDisp16` documents for `bra.w`/`Bcc.w`),
    /// confirmed against `crates/sigil-isa/tests/corpus_m68k/mod.rs`
    /// (`"dbf d0,*"` / `"dbeq d1,*"` both fold to `Disp(-2)`, i.e.
    /// `self_address - (self_address + 2)`) and against real `asl` (see
    /// `m68k_dbf_d0_self`/`m68k_dbeq_d1_self` in `tests/snippets_golden.txt`).
    fn lower_m68k_dbcc(&mut self, mnemonic: M68kMnemonic, rest: &[Token], span: Span) {
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let (dn_name, target_expr) = match atoms.as_slice() {
            [OperandAtom::Value(Expr::Sym(dn)), OperandAtom::Value(t)] => (dn.clone(), t.clone()),
            _ => {
                self.err(span, "Dbcc needs `Dn,target` operands");
                return;
            }
        };
        let dn = match m68k_data_reg(&dn_name) {
            Some(n) => n,
            None => {
                self.err(span, format!("`{dn_name}` is not a valid data register in Dbcc"));
                return;
            }
        };
        let target = self.resolve_dollar(&self.qualify_expr(&target_expr));
        let pc_of_disp_word = Expr::Int((self.here() + 2) as i64);
        let disp_expr = Expr::Binary { op: BinOp::Sub, lhs: Box::new(target), rhs: Box::new(pc_of_disp_word) };
        let d = self.fold_imm(&disp_expr, span, i16::MIN as i64, i16::MAX as i64);
        let inst = M68kInstruction {
            mnemonic,
            size: M68kSize::W,
            ops: vec![M68kOperand::Dn(dn), M68kOperand::Disp(d as i32)],
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// An instruction with a `(d16,PC)` source operand (any mnemonic: `move`,
    /// `tst`, `cmp`, ...). `pc_idx` is the index of that atom within `atoms`
    /// (already located by the caller). `(d16,PC)` is illegal as a
    /// DESTINATION EA (`encode_ea` rejects it there — real 68k only reads
    /// through PC-relative), so wherever it legally appears it is the single
    /// EA operand of a 1-operand form or the SOURCE of a 2-operand form; both
    /// `encode_move`/`encode_alu_ea`/`encode_control`/etc. process the source
    /// EA's extension words first (right after the 2-byte opcode word), so
    /// the `(d16,PC)` extension word always starts at byte offset 2 —
    /// confirmed against `lower_pcrel_ea`'s own unit test (`lea (d16,PC),a0`)
    /// and against real asl (`m68k_move_w_pcd16_to_d0` in
    /// `tests/snippets_golden.txt`).
    fn lower_m68k_pcrel(&mut self, mnemonic: M68kMnemonic, size: M68kSize, atoms: &[OperandAtom], pc_idx: usize, span: Span) {
        let mut ops = Vec::with_capacity(atoms.len());
        let mut target = None;
        for (i, a) in atoms.iter().enumerate() {
            if i == pc_idx {
                let disp = match a {
                    OperandAtom::M68kDisp { disp, .. } => disp,
                    _ => unreachable!("pc_idx must index a M68kDisp{{an: \"pc\"}} atom"),
                };
                target = Some(self.qualify_expr(disp));
                ops.push(M68kOperand::Pcd16(0));
            } else {
                match self.convert_one_atom_m68k(a, size, span) {
                    Some(op) => ops.push(op),
                    None => return,
                }
            }
        }
        let target = target.expect("pc_idx must index the pc-relative atom");
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction { mnemonic, size, ops };
        let frag = self.m68k.lower_pcrel_ea(&inst, 2, target, span);
        self.emit_frag(frag, span);
    }

    /// Convert operand atoms to resolved 68k operands for the fold-based
    /// (no-fixup) core: `Dn`/`An`/`Imm` (plus bare `sr`/`ccr`), the
    /// register-indirect family, and explicit-width absolute (`M68kAbs`).
    /// Any width-selecting bare-`(expr)` or `(d8,PC,Xn)` atom is rejected
    /// with a diagnostic (the latter is the only PC-relative form still
    /// unsupported — see [`Self::lower_m68k_pcrel`] for `(d16,PC)`).
    fn convert_atoms_m68k(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Vec<M68kOperand>> {
        let _ = mnemonic; // every fold-based form shares the same atom conversion
        let mut ops = Vec::with_capacity(atoms.len());
        for a in atoms {
            ops.push(self.convert_one_atom_m68k(a, size, span)?);
        }
        Some(ops)
    }

    /// Convert one operand atom (see [`Self::convert_atoms_m68k`]).
    fn convert_one_atom_m68k(&mut self, a: &OperandAtom, size: M68kSize, span: Span) -> Option<M68kOperand> {
        Some(match a {
                OperandAtom::Imm(e) => {
                    let (lo, hi) = m68k_imm_bounds(size);
                    let v = self.fold_imm(e, span, lo, hi);
                    M68kOperand::Imm(v as i32)
                }
                OperandAtom::RegOrCond(w) => {
                    if let Some(n) = m68k_addr_reg(w) {
                        M68kOperand::An(n)
                    } else if let Some(n) = m68k_data_reg(w) {
                        M68kOperand::Dn(n)
                    } else {
                        self.err(span, format!("`{w}` is not a valid 68k register in this context"));
                        return None;
                    }
                }
                OperandAtom::Value(Expr::Sym(name)) => {
                    if let Some(n) = m68k_data_reg(name) {
                        M68kOperand::Dn(n)
                    } else if let Some(n) = m68k_addr_reg(name) {
                        M68kOperand::An(n)
                    } else if name == "sr" {
                        M68kOperand::Sr
                    } else if name == "ccr" {
                        M68kOperand::Ccr
                    } else {
                        self.err(
                            span,
                            format!(
                                "absolute/symbolic operand `{name}` is out of scope for T5 (register-direct/#immediate/register-indirect only); deferred to T5b"
                            ),
                        );
                        return None;
                    }
                }
                OperandAtom::Value(_) => {
                    self.err(
                        span,
                        "bare numeric/expression operand implies 68k absolute addressing, out of scope for T5; deferred to T5b",
                    );
                    return None;
                }
                OperandAtom::Mem(_) => {
                    self.err(
                        span,
                        "absolute address operand `(expr)` needs an explicit `.w`/`.l` width suffix (width-selecting bare `(expr)` is out of scope)",
                    );
                    return None;
                }
                OperandAtom::M68kAbs { addr, long } => {
                    let qualified = self.qualify_expr(addr);
                    let v = self.fold_imm(&qualified, span, i32::MIN as i64, u32::MAX as i64);
                    if *long {
                        M68kOperand::AbsL(v as i32)
                    } else {
                        M68kOperand::AbsW((v & 0xFFFF) as i16)
                    }
                }
                // `(sp)` is the `a7` alias but lexes down the pre-existing Z80
                // `hl`/`bc`/`de`/`sp` branch (see `classify`), not `M68kInd`.
                OperandAtom::IndReg(w) if w == "sp" => M68kOperand::Ind(7),
                OperandAtom::IndReg(w) => {
                    self.err(span, format!("`({w})` is not a valid 68k address-register indirect operand"));
                    return None;
                }
                OperandAtom::Indexed { .. } => {
                    self.err(span, "z80 `(ix±d)`/`(iy±d)` indexed form is not a valid 68k operand");
                    return None;
                }
                OperandAtom::M68kPreDec(reg) => match m68k_addr_reg(reg) {
                    Some(n) => M68kOperand::PreDec(n),
                    None => {
                        self.err(span, format!("`{reg}` is not a valid address register in `-(An)`"));
                        return None;
                    }
                },
                OperandAtom::M68kPostInc(reg) => match m68k_addr_reg(reg) {
                    Some(n) => M68kOperand::PostInc(n),
                    None => {
                        self.err(span, format!("`{reg}` is not a valid address register in `(An)+`"));
                        return None;
                    }
                },
                OperandAtom::M68kInd(reg) => match m68k_addr_reg(reg) {
                    Some(n) => M68kOperand::Ind(n),
                    None => {
                        self.err(span, format!("`{reg}` is not a valid address register in `(An)`"));
                        return None;
                    }
                },
                OperandAtom::M68kDisp { disp, an } => {
                    let n = match m68k_addr_reg(an) {
                        Some(n) => n,
                        None => {
                            self.err(span, m68k_disp_an_error(an));
                            return None;
                        }
                    };
                    let d = self.fold_imm(disp, span, i16::MIN as i64, i16::MAX as i64);
                    M68kOperand::Disp16An(d as i16, n)
                }
                OperandAtom::M68kIdx { disp, an, xn, xlong } => {
                    let an_n = match m68k_addr_reg(an) {
                        Some(n) => n,
                        None => {
                            self.err(span, m68k_disp_an_error(an));
                            return None;
                        }
                    };
                    let xn_op = if let Some(n) = m68k_data_reg(xn) {
                        M68kXn::D(n)
                    } else if let Some(n) = m68k_addr_reg(xn) {
                        M68kXn::A(n)
                    } else {
                        self.err(span, format!("`{xn}` is not a valid index register in `(d,An,Xn)`"));
                        return None;
                    };
                    let d = self.fold_imm(disp, span, i8::MIN as i64, i8::MAX as i64);
                    M68kOperand::Disp8AnXn { d: d as i8, an: an_n, xn: xn_op, long: *xlong }
                }
                OperandAtom::AfShadow => {
                    self.err(span, "`af'` is not a 68k operand");
                    return None;
                }
        })
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
                OperandAtom::Value(e) => self.resolve_dollar(&self.qualify_expr(e)),
                _ => {
                    self.err(span, "jr/djnz needs a label target");
                    return None;
                }
            };
            return Some(Lowered::Rel(cond, target));
        }
        if matches!(m, Mnemonic::Jp | Mnemonic::Call) {
            let (cond, target_opt) = self.split_control_target(atoms);
            if let Some(target) = target_opt {
                if matches!(target, Expr::Sym(_)) {
                    let mut ops = Vec::new();
                    if let Some(cc) = cond {
                        ops.push(Operand::Cc(cc));
                    }
                    return Some(match self.fold(&target) {
                        Fold::Value(v) => {
                            ops.push(Operand::Imm16(v as u16));
                            Lowered::Fixed(ops)
                        }
                        Fold::Poison => {
                            ops.push(Operand::Imm16(0));
                            Lowered::Abs16(ops, target)
                        }
                    });
                }
            }
        }
        if matches!(m, Mnemonic::Ld) {
            if let [OperandAtom::RegOrCond(w), OperandAtom::Value(e @ Expr::Sym(_))] = atoms {
                if let Some(rr) = reg16(w) {
                    let target = self.qualify_expr(e);
                    return Some(match self.fold(&target) {
                        Fold::Value(v) => Lowered::Fixed(vec![Operand::Pair(rr), Operand::Imm16(v as u16)]),
                        Fold::Poison => Lowered::Abs16(vec![Operand::Pair(rr), Operand::Imm16(0)], target),
                    });
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

    /// Replace `$` (current-PC) sub-expressions with a concrete Int so the
    /// relative-jump fixup carries a resolvable target. Other symbols stay
    /// symbolic so real (possibly forward) labels still take the fixup path.
    /// Mirrors `fold`'s rule that `$` never survives as a Sym fixup target.
    fn resolve_dollar(&self, e: &Expr) -> Expr {
        match e {
            Expr::Sym(name) if name == "$" => Expr::Int(self.here() as i64),
            Expr::Binary { op, lhs, rhs } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_dollar(lhs)),
                rhs: Box::new(self.resolve_dollar(rhs)),
            },
            Expr::Unary { op, operand } => {
                Expr::Unary { op: *op, operand: Box::new(self.resolve_dollar(operand)) }
            }
            other => other.clone(),
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
                    // `ex (sp),hl` — the encoder special-cases [Pair(Sp), Pair(Hl)] -> E3.
                    "sp" if matches!(m, Mnemonic::Ex) => Operand::Pair(Reg16::Sp),
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
                OperandAtom::Imm(_) => {
                    // `#imm` is a 68k-only operand form (see `convert_atoms_m68k`);
                    // the z80 lexer never emits a `#` token, so this is unreachable
                    // in practice, but the match must stay exhaustive.
                    self.err(span, "`#` immediate is not valid z80 syntax");
                    return None;
                }
                OperandAtom::M68kPreDec(_)
                | OperandAtom::M68kPostInc(_)
                | OperandAtom::M68kInd(_)
                | OperandAtom::M68kDisp { .. }
                | OperandAtom::M68kIdx { .. }
                | OperandAtom::M68kAbs { .. } => {
                    // These 68k-only EA shapes (see `convert_atoms_m68k`) don't
                    // arise from z80 syntax in practice (`a0`.."a7" aren't z80
                    // register names), but the match must stay exhaustive.
                    self.err(span, "this operand form is not valid z80 syntax");
                    return None;
                }
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

    /// Capture `<name> macro [params] … endm`. Returns the index past `endm`.
    fn capture_macro(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let toks = lex_line(&lines[start].text, self.state.cpu, self.source, lines[start].base)
            .unwrap_or_default();
        // toks: Ident(name) Ident("macro") [param idents/commas...]
        let name = match toks.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                let span = Span { source: self.source, start: lines[start].base, end: lines[start].base };
                self.err(span, "macro needs a name");
                String::new()
            }
        };
        let params: Vec<String> = toks
            .get(2..)
            .unwrap_or(&[])
            .iter()
            .filter_map(|t| if let Tok::Ident(p) = &t.tok { Some(p.clone()) } else { None })
            .collect();
        let end = self.find_block_end(lines, start, &["macro"], &["endm"]);
        let body: Vec<SrcLine> = lines[start + 1..end].to_vec();
        self.macros.insert(name, (params, body));
        end + 1
    }

    /// Expand a macro invocation: substitute `ALLARGS` (verbatim arg text) and
    /// params (positional and/or keyword), then execute the resulting lines.
    ///
    /// Real AS binds params two ways, mixable in one call (asl-verified — see
    /// the `macro_keyword_args` snippet): a comma-split arg of the shape
    /// `NAME=value` binds `NAME` by name, regardless of where it sits in the
    /// call; every other arg fills the remaining (not yet keyword-bound)
    /// params positionally, in declaration order. `tst AMP=7,PER=9`,
    /// `tst 3,4`, and `tst PER=5,AMP=2` (params `AMP,PER`) all bind correctly
    /// under this rule.
    fn expand_macro(&mut self, name: &str, arg_toks: &[Token]) {
        if self.macro_depth >= EXPAND_CAP {
            let span = arg_toks.first().map(|t| t.span).unwrap_or(Span { source: self.source, start: 0, end: 0 });
            self.err(span, format!("macro `{name}` expansion too deep (recursive macro?)"));
            return;
        }
        let (params, body) = match self.macros.get(name) {
            Some(m) => m.clone(),
            None => return,
        };
        let all_args = render_tokens(arg_toks);
        let groups = split_top_commas(arg_toks);
        let mut keyword: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        let mut positional: Vec<String> = Vec::new();
        for g in &groups {
            if let [Token { tok: Tok::Ident(nm), .. }, Token { tok: Tok::Punct(Punct::Eq), .. }, value @ ..] = *g {
                if !value.is_empty() && params.iter().any(|p| p == nm) {
                    keyword.insert(nm.clone(), render_tokens(value));
                    continue;
                }
            }
            positional.push(render_tokens(g));
        }
        let mut pos_iter = positional.into_iter();
        let arg_values: Vec<(String, String)> = params
            .iter()
            .filter_map(|p| {
                keyword
                    .get(p)
                    .cloned()
                    .or_else(|| pos_iter.next())
                    .map(|v| (p.clone(), v))
            })
            .collect();
        let mut expanded = Vec::new();
        for l in &body {
            let mut text = l.text.clone();
            text = text.replace("ALLARGS", &all_args);
            for (p, a) in &arg_values {
                text = replace_word(&text, p, a);
            }
            expanded.push(SrcLine { text, base: l.base });
        }
        self.macro_depth += 1;
        self.exec(&expanded);
        self.macro_depth -= 1;
    }
}

// ── free helpers ────────────────────────────────────────────────────────────

/// Split source text into `SrcLine`s (each with its byte offset). Used for both
/// the root source and included files.
fn split_src_lines(text: &str) -> Vec<SrcLine> {
    let mut lines = Vec::new();
    let mut base = 0u32;
    for raw in text.split_inclusive('\n') {
        lines.push(SrcLine { text: raw.to_string(), base });
        base += raw.len() as u32;
    }
    lines
}

fn is_op_keyword(s: &str) -> bool {
    matches!(
        s,
        "cpu" | "phase" | "dephase" | "save" | "restore" | "padding" | "supmode"
            | "db" | "dw" | "dc.b" | "dc.w" | "dc.l" | "equ"
            | "if" | "elseif" | "else" | "endif" | "ifdef" | "ifndef"
            | "rept" | "endr" | "endm" | "macro" | "struct" | "endstruct"
            | "function" | "include" | "error" | "fatal" | "message"
            | "ds.b" | "ds.w" | "ds.l" | "align"
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

/// Split a 68k mnemonic token on a trailing `.b`/`.w`/`.l`/`.s` size suffix.
/// Returns the bare base mnemonic and the parsed size (`None` if no suffix —
/// the caller falls back to `m68k_default_size`, or errors if that's also `None`).
fn split_mnemonic_and_size(s: &str) -> (&str, Option<M68kSize>) {
    if let Some(b) = s.strip_suffix(".b") {
        (b, Some(M68kSize::B))
    } else if let Some(b) = s.strip_suffix(".w") {
        (b, Some(M68kSize::W))
    } else if let Some(b) = s.strip_suffix(".l") {
        (b, Some(M68kSize::L))
    } else if let Some(b) = s.strip_suffix(".s") {
        (b, Some(M68kSize::S))
    } else {
        (s, None)
    }
}

/// The T4/T5/T5b/T5c in-scope 68000 mnemonic table: straight-line
/// register/immediate core, the fixed-length register-indirect EA family,
/// `lea`/`pea`, explicit-width absolute addressing, and (T5c) control
/// transfer — `bra`/`bsr`/`Bcc`, `Dbcc` (`dbf`/`dbra`/`db<cc>`), `Scc`, and
/// `jmp`/`jsr`. `move`/`andi`/`ori` are refined to `MoveToSr`/`MoveFromSr`/
/// `AndiCcr`/`OriCcr` post-hoc by `refine_m68k_mnemonic` once the operand
/// shape (a bare `sr`/`ccr`) is known. Only `movem`/`movep` remain out of
/// scope — see `m68k_out_of_scope`.
fn m68k_mnemonic(base: &str) -> Option<M68kMnemonic> {
    use M68kMnemonic::*;
    Some(match base {
        "move" => Move, "movea" => Movea,
        "add" => Add, "adda" => Adda, "sub" => Sub, "suba" => Suba,
        "and" => And, "or" => Or, "eor" => Eor, "cmp" => Cmp, "cmpa" => Cmpa, "muls" => Muls,
        "addi" => Addi, "subi" => Subi, "andi" => Andi, "ori" => Ori, "eori" => Eori, "cmpi" => Cmpi,
        "moveq" => Moveq, "addq" => Addq, "subq" => Subq,
        "asl" => Asl, "asr" => Asr, "lsl" => Lsl, "lsr" => Lsr, "rol" => Rol, "ror" => Ror,
        "btst" => Btst, "bset" => Bset, "bclr" => Bclr,
        "clr" => Clr, "neg" => Neg, "not" => Not, "tst" => Tst, "tas" => Tas,
        "swap" => Swap, "ext" => Ext, "lea" => Lea, "pea" => Pea,
        "nop" => Nop, "rts" => Rts, "rte" => Rte, "trap" => Trap,
        "bra" => Bra, "bsr" => Bsr,
        "jmp" => Jmp, "jsr" => Jsr,
        "dbf" | "dbra" => Dbcc(M68kCond::F),
        _ => {
            if let Some(rest) = base.strip_prefix("db") {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Dbcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('b') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Bcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('s') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Scc(c));
                }
            }
            return None;
        }
    })
}

/// Parse a 68000 condition-code mnemonic suffix (the `<cc>` in `b<cc>`,
/// `db<cc>`, `s<cc>`) into its `Cond`. All 16 codes per the ISA's `Cond` enum.
fn m68k_cond(w: &str) -> Option<M68kCond> {
    use M68kCond::*;
    Some(match w {
        "t" => T, "f" => F, "hi" => Hi, "ls" => Ls, "cc" => Cc, "cs" => Cs,
        "ne" => Ne, "eq" => Eq, "vc" => Vc, "vs" => Vs, "pl" => Pl, "mi" => Mi,
        "ge" => Ge, "lt" => Lt, "gt" => Gt, "le" => Le,
        _ => return None,
    })
}

/// If `base` names it a real 68000 mnemonic that this front-end deliberately
/// does not implement yet (`movem`/`movep`), name the family for the
/// diagnostic; else `None` (genuinely unrecognized).
fn m68k_out_of_scope(base: &str) -> Option<&'static str> {
    match base {
        "movem" | "movep" => Some("movem/movep"),
        _ => None,
    }
}

/// The implicit size for mnemonics real 68k syntax never suffixes (`moveq`,
/// `swap`, `nop`, `rts`, `rte`, `tas`, `trap`, `lea`, `pea`, `jmp`, `jsr`,
/// `Dbcc`, `Scc`). Verified against `crates/sigil-isa/tests/corpus_m68k/mod.rs`:
/// `moveq` is always encoded `Size::L` (the encoder truncates the data to a
/// signed byte regardless); `lea`/`pea` are always long (an address is always
/// 32 bits); `jmp`/`jsr`/the fixed-opcode control forms carry `Size::W` in the
/// corpus, `Dbcc` is always `Size::W` (its displacement is a fixed 16-bit
/// word), and `Scc` is always `Size::B` (byte-fixed opcode) — in every case
/// purely because `Instruction` requires *a* size field; the encoder ignores
/// it for them. Branches (`bra`/`bsr`/`Bcc`) deliberately have NO default:
/// Aeon pins branch width by an explicit `.s`/`.w` suffix, never relaxes.
fn m68k_default_size(m: M68kMnemonic) -> Option<M68kSize> {
    use M68kMnemonic::*;
    match m {
        Moveq => Some(M68kSize::L),
        Lea | Pea => Some(M68kSize::L),
        Swap | Nop | Rts | Rte | Tas | Trap => Some(M68kSize::W),
        Jmp | Jsr => Some(M68kSize::W),
        Dbcc(_) => Some(M68kSize::W),
        Scc(_) => Some(M68kSize::B),
        _ => None,
    }
}

/// Fold bounds for a `#imm` operand at a given size — generous enough to admit
/// either the signed or the bit-pattern-equivalent unsigned spelling; the
/// encoder (`imm_ext_words`/`moveq`/`addq` range checks) does the real
/// business-rule validation and surfaces an `IsaError` on overflow.
fn m68k_imm_bounds(size: M68kSize) -> (i64, i64) {
    match size {
        M68kSize::B => (-128, 0xFF),
        M68kSize::W => (-0x8000, 0xFFFF),
        M68kSize::L | M68kSize::S => (i32::MIN as i64, u32::MAX as i64),
    }
}

/// `d0`..`d7` → `Some(0..=7)`; anything else (including out-of-range `d8`+) → `None`.
fn m68k_data_reg(w: &str) -> Option<u8> {
    let n: u8 = w.strip_prefix('d')?.parse().ok()?;
    (n <= 7).then_some(n)
}

/// The `an`-slot error for `(d,An)`/`(d,An,Xn)` when it's not a real address
/// register. `pc` parses down the same `(expr,ident)` shape as `(d16,An)`/
/// `(d8,An,Xn)` (see `classify`). `(d16,PC)` is intercepted and lowered
/// earlier (see `lower_m68k_generic`'s pc-relative scan), so this only ever
/// fires for the still-unsupported `(d8,PC,Xn)` indexed form (an `M68kIdx`
/// atom) — hence its own naming diagnostic rather than the generic
/// "not a valid address register" one.
fn m68k_disp_an_error(an: &str) -> String {
    if an == "pc" {
        "`(d8,PC,Xn)` indexed PC-relative addressing is not yet supported (only `(d16,PC)` lowers)".to_string()
    } else {
        format!("`{an}` is not a valid address register in `(d,An)`/`(d,An,Xn)`")
    }
}

/// `a0`..`a7` → `Some(0..=7)`; `sp` is the `a7` alias. Anything else → `None`.
fn m68k_addr_reg(w: &str) -> Option<u8> {
    if w == "sp" {
        return Some(7);
    }
    let n: u8 = w.strip_prefix('a')?.parse().ok()?;
    (n <= 7).then_some(n)
}

/// Post-hoc mnemonic refinement for the operand-shape-dependent variants: a
/// `move` to/from the bare `sr` pseudo-register is really `MoveToSr`/
/// `MoveFromSr`; `andi`/`ori` targeting the bare `ccr` pseudo-register are
/// really `AndiCcr`/`OriCcr`. The encoder dispatches solely on `Mnemonic`, so
/// this must happen before building the `Instruction`.
fn refine_m68k_mnemonic(mnemonic: M68kMnemonic, ops: &[M68kOperand]) -> M68kMnemonic {
    use M68kMnemonic::*;
    match (mnemonic, ops) {
        (Move, [_, M68kOperand::Sr]) => MoveToSr,
        (Move, [M68kOperand::Sr, _]) => MoveFromSr,
        (Andi, [_, M68kOperand::Ccr]) => AndiCcr,
        (Ori, [_, M68kOperand::Ccr]) => OriCcr,
        (m, _) => m,
    }
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
    fn split_mnemonic_and_size_strips_known_suffixes() {
        use super::split_mnemonic_and_size;
        use sigil_backend_m68k::m68k::Size;
        assert_eq!(split_mnemonic_and_size("move.w"), ("move", Some(Size::W)));
        assert_eq!(split_mnemonic_and_size("move.l"), ("move", Some(Size::L)));
        assert_eq!(split_mnemonic_and_size("clr.b"), ("clr", Some(Size::B)));
        assert_eq!(split_mnemonic_and_size("bra.s"), ("bra", Some(Size::S)));
        assert_eq!(split_mnemonic_and_size("moveq"), ("moveq", None));
        assert_eq!(split_mnemonic_and_size("swap"), ("swap", None));
    }

    #[test]
    fn m68k_mnemonic_recognizes_in_scope_bases() {
        use super::m68k_mnemonic;
        use sigil_backend_m68k::m68k::Mnemonic;
        assert_eq!(m68k_mnemonic("move"), Some(Mnemonic::Move));
        assert_eq!(m68k_mnemonic("moveq"), Some(Mnemonic::Moveq));
        assert_eq!(m68k_mnemonic("addq"), Some(Mnemonic::Addq));
        assert_eq!(m68k_mnemonic("swap"), Some(Mnemonic::Swap));
        assert_eq!(m68k_mnemonic("ext"), Some(Mnemonic::Ext));
        assert_eq!(m68k_mnemonic("nop"), Some(Mnemonic::Nop));
        assert_eq!(m68k_mnemonic("rts"), Some(Mnemonic::Rts));
        assert_eq!(m68k_mnemonic("rte"), Some(Mnemonic::Rte));
        // T5 adds the fixed-length EA family plus `lea`/`pea` — both in-scope now.
        assert_eq!(m68k_mnemonic("lea"), Some(Mnemonic::Lea));
        assert_eq!(m68k_mnemonic("pea"), Some(Mnemonic::Pea));
        // T5c adds control transfer: branches, Dbcc, Scc, jmp/jsr.
        assert_eq!(m68k_mnemonic("jmp"), Some(Mnemonic::Jmp));
        assert_eq!(m68k_mnemonic("jsr"), Some(Mnemonic::Jsr));
        assert_eq!(m68k_mnemonic("bra"), Some(Mnemonic::Bra));
        assert_eq!(m68k_mnemonic("bsr"), Some(Mnemonic::Bsr));
        assert_eq!(m68k_mnemonic("beq"), Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Eq)));
        assert_eq!(m68k_mnemonic("bne"), Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Ne)));
        assert_eq!(m68k_mnemonic("dbf"), Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F)));
        assert_eq!(m68k_mnemonic("dbra"), Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F)));
        assert_eq!(m68k_mnemonic("dbeq"), Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::Eq)));
        assert_eq!(m68k_mnemonic("scc"), Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cc)));
        assert_eq!(m68k_mnemonic("seq"), Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Eq)));
        assert_eq!(m68k_mnemonic("st"), Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::T)));
        // `movem`/`movep` remain out of scope.
        assert_eq!(m68k_mnemonic("movem"), None);
        assert_eq!(m68k_mnemonic("movep"), None);
        // a genuinely unrecognized word is not misparsed as a stray cc suffix.
        assert_eq!(m68k_mnemonic("banana"), None);
    }

    #[test]
    fn m68k_cond_parses_all_16_condition_codes() {
        use super::m68k_cond;
        use sigil_backend_m68k::m68k::Cond;
        let pairs = [
            ("t", Cond::T), ("f", Cond::F), ("hi", Cond::Hi), ("ls", Cond::Ls),
            ("cc", Cond::Cc), ("cs", Cond::Cs), ("ne", Cond::Ne), ("eq", Cond::Eq),
            ("vc", Cond::Vc), ("vs", Cond::Vs), ("pl", Cond::Pl), ("mi", Cond::Mi),
            ("ge", Cond::Ge), ("lt", Cond::Lt), ("gt", Cond::Gt), ("le", Cond::Le),
        ];
        for (w, c) in pairs {
            assert_eq!(m68k_cond(w), Some(c), "cc word `{w}`");
        }
        assert_eq!(m68k_cond("xx"), None);
    }

    #[test]
    fn m68k_register_words_recognized() {
        use super::{m68k_addr_reg, m68k_data_reg};
        assert_eq!(m68k_data_reg("d0"), Some(0));
        assert_eq!(m68k_data_reg("d7"), Some(7));
        assert_eq!(m68k_data_reg("d8"), None);
        assert_eq!(m68k_data_reg("a0"), None);
        assert_eq!(m68k_addr_reg("a0"), Some(0));
        assert_eq!(m68k_addr_reg("a7"), Some(7));
        assert_eq!(m68k_addr_reg("sp"), Some(7));
        assert_eq!(m68k_addr_reg("d0"), None);
    }

    #[test]
    fn m68k_register_indirect_operand_now_lowers_in_t5() {
        // `(a0)` is a register-indirect EA — T4 deferred it to T5; T5 (this
        // task) implements the fixed-length `(An)` family, so it now lowers
        // byte-exact instead of erroring. Bytes verified against real asl
        // (see `m68k_move_w_ind_a0_to_d0` in `tests/snippets_golden.txt`).
        assert_eq!(image("    cpu 68000\n    move.w (a0),d0\n"), vec![0x30, 0x10]);
    }

    #[test]
    fn m68k_pcrelative_disp16_lowers_via_resolve_layout_link() {
        // `(d16,PC)` (T5c): the front-end emits an unresolved `PcRelDisp16`
        // fixup (via `lower_pcrel_ea`); resolving it needs a real link (the
        // front-end's own fold never sees it — see `apply_fixup` in
        // `sigil-link`). `move.w (8,pc),d0` at VMA 0: the extension word sits
        // at offset 2, target = 8, disp = 8 - 2 = 6.
        let src = "    cpu 68000\n    phase 0\n    move.w (8,pc),d0\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true).expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // move.w (d16,PC),d0 = 30 3A, then disp word 00 06.
        assert_eq!(bytes, vec![0x30, 0x3A, 0x00, 0x06]);
    }

    #[test]
    fn m68k_pcrelative_disp8_indexed_still_not_supported() {
        // `(d8,PC,Xn)` remains out of scope (only `(d16,PC)` lowers in T5c).
        let src = "    cpu 68000\n    move.w (8,pc,d0.w),d1\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let diags = run(src, &opts).expect_err("(d8,PC,Xn) must be rejected, not lowered");
        assert!(
            diags.iter().any(|d| d.message.contains("not yet supported")),
            "expected a not-yet-supported diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn m68k_absolute_address_operand_diagnoses_t5_not_a_crash() {
        // A bare symbol/number (no `#`, no parens) means 68k absolute addressing
        // — out of scope for T4 (deferred to T5).
        let src = "    cpu 68000\n    move.w $1234,d0\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let diags = run(src, &opts).expect_err("absolute address operand must be rejected, not lowered");
        assert!(
            diags.iter().any(|d| d.message.contains("T5")),
            "expected a T5-deferral diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn m68k_branch_without_size_suffix_is_a_clear_diagnostic() {
        // T5c: `bra`/`Bcc` are now in scope, but Aeon pins branch width by an
        // explicit `.s`/`.w` suffix (no relaxation) — a bare `bra` must still
        // error, just with a size-suffix diagnostic instead of a scope one.
        let src = "    cpu 68000\n    bra Target\nTarget:\n    rts\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let diags = run(src, &opts).expect_err("branch without a size suffix must be rejected, not lowered");
        assert!(
            diags.iter().any(|d| d.message.contains("size suffix")),
            "expected a size-suffix diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn m68k_bra_w_qualifies_local_target_against_current_scope() {
        // A bare `.local` branch target must be qualified to `Scope.local`
        // BEFORE lowering (the linker resolves in global scope only) — this
        // is the exact hazard `qualify_expr` exists for. `Start:` opens scope
        // `Start`, so `bra.w .loop` must resolve against `Start.loop`, not a
        // bare `.loop` (which the linker would never find).
        let src = "    cpu 68000\n    phase 0\nStart:\n    bra.w .loop\n.loop:\n    rts\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true).expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // bra.w .loop: op@0, disp word@2, target=4 (right after the 4-byte
        // branch), disp = 4-2 = 2; then rts (4E75) at the target.
        assert_eq!(bytes, vec![0x60, 0x00, 0x00, 0x02, 0x4E, 0x75]);
    }

    #[test]
    fn m68k_jmp_jsr_bare_symbol_defers_width_to_resolve_layout() {
        // `jmp Lbl`/`jsr Lbl` must emit `Fragment::JmpJsrSym` (not fold
        // immediately) — its abs.w/abs.l width is chosen later by the
        // linker's `resolve_layout`. A low (<=0x7FFF) target selects abs.w.
        let src = "    cpu 68000\n    phase 0\nLbl:\n    jmp Lbl\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        assert!(matches!(m.sections[0].fragments[0], sigil_ir::Fragment::JmpJsrSym { is_jsr: false, .. }));
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true).expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
    }

    #[test]
    fn m68k_missing_size_suffix_is_a_clear_diagnostic() {
        // `move` has no default size and no suffix here — must error, not guess.
        let src = "    cpu 68000\n    move d0,d1\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let diags = run(src, &opts).expect_err("missing size suffix must be rejected");
        assert!(
            diags.iter().any(|d| d.message.contains("size suffix")),
            "expected a size-suffix diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn equate_as_16bit_operand_folds_not_fixups() {
        // BufSize is an EQUATE (not a label); it must FOLD, since the linker
        // cannot resolve a fixup to a non-label. Assemble + LINK + flatten.
        let src = "        cpu z80\n        phase 0\nBufSize = 1234h\n        ld hl,BufSize\n        dw BufSize\n";
        let m = run(src, &Options::default()).expect("assemble");
        let linked = sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new()).expect("link must succeed (no unresolvable fixup)");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // ld hl,1234h = 21 34 12 ; dw 1234h = 34 12
        assert_eq!(bytes, vec![0x21, 0x34, 0x12, 0x34, 0x12]);
    }

    #[test]
    fn jr_dollar_relative_arithmetic_resolves_binary() {
        // Exercises resolve_dollar's Binary recursion: `$` in `jr $±2` must fold
        // to the instruction's own PC (0 under phase 0) before the fixup is made.
        // Linker: disp = target - inst_end_vma, inst_end_vma = 2 for a jr at PC 0.
        // `jr $+2` -> target 2, disp 0  -> 18 00
        // `jr $-2` -> target -2, disp -4 -> 18 FC
        let link = |src: &str| {
            let m = run(src, &Options::default()).expect("assemble");
            let linked = sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new()).expect("link");
            sigil_link::flatten(&linked, 0x00)
        };
        assert_eq!(link("        cpu z80\n        phase 0\n        jr $+2\n"), vec![0x18, 0x00]);
        assert_eq!(link("        cpu z80\n        phase 0\n        jr $-2\n"), vec![0x18, 0xFC]);
    }

    #[test]
    fn ifdef_gates_emission_by_define_set() {
        let src = "        cpu z80\n        phase 0\n        db 1\n        ifdef __DEBUG__\n        db 0FFh\n        endif\n        ifdef SOUND_DRIVER_ENABLED\n        db 2\n        endif\n";
        let opts = Options { initial_cpu: Cpu::Z80, defines: vec![("SOUND_DRIVER_ENABLED".into(), 1)], include_root: None };
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
    fn if_literal_string_equality_and_inequality() {
        // Literal `"a"="a"` / `"a"<>"b"` must fold to a bool directly (never
        // through sigil_ir::Expr — strings are not an IR concept, §7.4).
        let src = "        cpu z80\n        phase 0\n        if \"a\"=\"a\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"=\"b\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"<>\"b\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"<>\"a\"\n        db 1\n        else\n        db 0\n        endif\n";
        assert_eq!(image(src), vec![0x01, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn nested_if_inside_taken_branch() {
        let src = "        cpu z80\n        phase 0\nX = 1\n        if X = 1\n        db 1\n        if X = 1\n        db 2\n        endif\n        db 3\n        endif\n";
        assert_eq!(image(src), vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn m68k_operandless_instruction_reaches_lower_not_swallowed_as_label() {
        // `rts` is NOT a Z80 mnemonic (Z80 has `ret`) and carries no operand, so
        // it is a clean discriminator: if the indented head were misclassified as
        // a bare label, `body.len() == 1` would define it and return with NO
        // bytes emitted. Routed correctly it reaches lower_m68k and (T4) lowers
        // for real: `rts` = 4E75.
        let src = "    cpu 68000\n    rts\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        let bytes = m.sections.first().map(|s| s.image_bytes()).unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
    }

    #[test]
    fn m68k_instruction_after_colon_label_lowers_the_mnemonic_not_the_operand() {
        // Before the column-rule fix, `move.w` was swallowed as a bogus label and
        // only `d0` reached dispatch. Routed correctly, the whole instruction
        // lowers: `move.w d0,d1` = 3200.
        let src = "    cpu 68000\nStart:\n    move.w d0,d1\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        let bytes = m.sections.first().map(|s| s.image_bytes()).unwrap_or_default();
        assert_eq!(bytes, vec![0x32, 0x00]);
    }

    #[test]
    fn m68k_colon_label_then_instruction_both_handled() {
        // `Foo: rts` on one line: the colon label must be defined AND the
        // remaining head routed as an instruction (label_colon.is_some() clause),
        // even though line.text starts at column 0.
        let src = "    cpu 68000\nFoo: rts\n";
        let opts = Options { initial_cpu: Cpu::M68000, defines: vec![], include_root: None };
        let m = run(src, &opts).expect("assemble");
        let bytes = m.sections.first().map(|s| s.image_bytes()).unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
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
            // Name-first (real AS): `<name> function <formal>, <body>`.
            "sfx_winptr function addr, ((addr) & SFX_WIN_MASK) | SFX_WIN_BASE\n",
            "sfx_bankid function addr, (addr) >> 15\n",
            "timerAReload function hz, 1024 - (1000000000 / ((hz) * 18773))\n",
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
    fn pbyte_macro_momcpuname_allargs_under_z80() {
        let src = concat!(
            "        cpu z80\n        phase 0\n",
            "        ifndef pbyte_defined\n",
            "pbyte_defined = 1\n",
            "pbyte   macro\n",
            "        if MOMCPUNAME=\"Z80\"\n",
            "        db      ALLARGS\n",
            "        else\n",
            "        dc.b    ALLARGS\n",
            "        endif\n",
            "        endm\n",
            "        endif\n",
            "        pbyte 1,2,3,255\n",
        );
        assert_eq!(image(src), vec![0x01, 0x02, 0x03, 0xFF]);
    }

    #[test]
    fn macro_positional_params() {
        let src = "        cpu z80\n        phase 0\nemit2   macro x,y\n        db x,y\n        endm\n        emit2 10h,20h\n";
        assert_eq!(image(src), vec![0x10, 0x20]);
    }

    #[test]
    fn macro_keyword_args_bind_by_name() {
        // asl-verified (see tst snippet in snippets_golden.txt): `NAME=value`
        // binds a param by name regardless of its position in the call.
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst AMP=7,PER=9\n",
        );
        assert_eq!(image(src), vec![0x07, 0x09]);
    }

    #[test]
    fn macro_positional_args_still_work_alongside_keyword_binding() {
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst 3,4\n",
        );
        assert_eq!(image(src), vec![0x03, 0x04]);
    }

    #[test]
    fn macro_keyword_args_are_order_independent() {
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst PER=5,AMP=2\n",
        );
        assert_eq!(image(src), vec![0x02, 0x05]);
    }

    #[test]
    fn struct_word_field_pads_running_offset_to_even_under_padding_on() {
        // asl-verified: with `padding on` (asl's default), a `ds.w`/`ds.l`
        // (width >= 2) field pads the running struct offset up to the next
        // even address AFTER it's placed — even though the field's own start
        // offset is not pre-aligned. Probed against real asl:
        // `a ds.b 1 / b ds.w 1 / c ds.b 1` -> a=0 b=1 c=4 len=5.
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "b       ds.w 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_b\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x01, 0x04, 0x05]);
    }

    #[test]
    fn struct_word_field_uses_naive_offset_under_padding_off() {
        // asl-verified: with `padding off` (Aeon's real global state, set at
        // the top of main.asm), struct fields are NOT even-rounded — the
        // running offset advances by exactly the field size. Probed against
        // real asl: `a ds.b 1 / b ds.w 1 / c ds.b 1` -> a=0 b=1 c=3 len=4.
        let src = concat!(
            "        cpu 68000\n        padding off\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "b       ds.w 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_b\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x01, 0x03, 0x04]);
    }

    #[test]
    fn function_name_first_simple_double() {
        // Self-contained: `dbl(x) = (x)*2`, name-first. db dbl(5) = 10 = 0x0A.
        let src = "        cpu z80\n        phase 0\ndbl function x, (x)*2\n        db dbl(5)\n";
        assert_eq!(image(src), vec![0x0A]);
    }

    #[test]
    fn struct_offsets_and_len_drive_indexed_disp() {
        // Packed: a(1) b(1) c(2) → a=0 b=1 c=2 len=4. Then (ix+SeqChannel_b) = (ix+1).
        // Name-first (real AS): `SeqChannel struct` … `SeqChannel endstruct`.
        let src = "        cpu z80\n        phase 0\nSeqChannel struct\na       ds.b 1\nb       ds.b 1\nc       ds.w 1\nSeqChannel endstruct\n        ld a,(ix+SeqChannel_b)\n        db SeqChannel_len\n";
        // ld a,(ix+1) = DD 7E 01 ; db 4 = 04
        assert_eq!(image(src), vec![0xDD, 0x7E, 0x01, 0x04]);
    }

    #[test]
    fn struct_three_byte_fields_len_and_offsets() {
        // Three ds.b 1 fields → offsets 0/1/2, DacSample_len = 3.
        let src = "        cpu z80\n        phase 0\nDacSample struct\np       ds.b 1\nq       ds.b 1\nr       ds.b 1\nDacSample endstruct\n        db DacSample_p, DacSample_q, DacSample_r, DacSample_len\n";
        assert_eq!(image(src), vec![0x00, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn dc_w_emits_big_endian_words() {
        // asl: `dc.w $1234,$5678` -> 12 34 56 78 (BE, not Z80 `dw`'s LE).
        let src = "        cpu 68000\n        phase 0\n        dc.w $1234,$5678\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn dc_l_emits_big_endian_longs() {
        let src = "        cpu 68000\n        phase 0\n        dc.l $12345678\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn ds_b_trailing_reserve_contributes_no_image_bytes() {
        // Matches real asl/p2bin: a trailing `ds` with nothing written after it
        // never materializes into the flat image (verified against asl).
        let src = "        cpu 68000\n        phase 0\n        ds.b 3\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn ds_w_and_ds_l_reserve_scale_by_unit_width() {
        let src = "        cpu 68000\n        phase 0\n        ds.w 2\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections[0].vma_len(), 4);
        assert_eq!(module.sections[0].image_len(), 0);

        let src = "        cpu 68000\n        phase 0\n        ds.l 1\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections[0].vma_len(), 4);
        assert_eq!(module.sections[0].image_len(), 0);
    }

    #[test]
    fn align_pads_zero_bytes_to_next_boundary() {
        // Odd offset 1 -> align 2 pads one zero byte, then the next dc.b lands
        // at the aligned offset (verified against asl: fill byte is 0x00).
        let src = "        cpu 68000\n        phase 0\n        dc.b 1\n        align 2\n        dc.b 2\n";
        assert_eq!(image(src), vec![0x01, 0x00, 0x02]);
    }

    #[test]
    fn align_is_a_noop_when_already_aligned() {
        let src = "        cpu 68000\n        phase 0\n        dc.w $1234\n        align 2\n        dc.b 9\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x09]);
    }

    #[test]
    fn align_pads_to_large_power_of_two_boundary() {
        let src = "        cpu 68000\n        phase 0\n        dc.b 1,2,3\n        align $10\n        dc.b 4\n";
        let mut want = vec![0x01, 0x02, 0x03];
        want.extend(std::iter::repeat_n(0x00, 13));
        want.push(0x04);
        assert_eq!(image(src), want);
    }

    #[test]
    fn message_interpolates_and_emits_no_bytes() {
        // false `if` guards fatal; message with \{expr} just evaluates; db N emits.
        let src = "        cpu z80\n        phase 0\nN = 5\n        if N <> 5\n        fatal \"bad size \\{N}\"\n        endif\n        message \"N is \\{N}\"\n        db N\n";
        assert_eq!(image(src), vec![0x05]);
    }

    #[test]
    fn fatal_on_true_condition_is_an_error() {
        let src = "        cpu z80\n        phase 0\nN = 6\n        if N <> 5\n        fatal \"bad size \\{N}\"\n        endif\n";
        assert!(run(src, &Options::default()).is_err());
    }

    #[test]
    fn forward_equate_resolves_across_passes() {
        // LATER is used by `db` BEFORE it is defined; the fixpoint resolves it.
        let src = "        cpu z80\n        phase 0\n        db LATER\nLATER   = 7\n";
        assert_eq!(image(src), vec![0x07]);
    }

    #[test]
    fn two_level_forward_chain_resolves() {
        // db A ; A = B ; B = 7  — needs 3 passes to settle.
        let src = "        cpu z80\n        phase 0\n        db A\nA       = B\nB       = 7\n";
        assert_eq!(image(src), vec![0x07]);
    }

    #[test]
    fn include_pulls_in_a_file() {
        let dir = std::env::temp_dir().join(format!("sigil_inc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("part.inc"), "        db 0AAh,0BBh\n").unwrap();
        let main = dir.join("main.asm");
        std::fs::write(&main, "        cpu z80\n        phase 0\n        db 1\n        include \"part.inc\"\n        db 2\n").unwrap();
        let m = crate::assemble_root(&main, &Options::default()).expect("assemble");
        let bytes = m.sections.first().map(|s| s.image_bytes()).unwrap_or_default();
        assert_eq!(bytes, vec![0x01, 0xAA, 0xBB, 0x02]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
