//! eval: the driver — line loop, directive dispatch, instruction lowering, emit.

use crate::expand::{render_tokens, replace_word, split_call_args, split_top_commas};
use crate::lexer::lex_line;
use crate::operands::{parse_operands, OperandAtom};
use crate::parser::parse_line_tokens;
use crate::token::{Punct, Tok, Token};
use crate::Options;
use sigil_backend_m68k::m68k::{
    Cond as M68kCond, Instruction as M68kInstruction, Mnemonic as M68kMnemonic,
    Operand as M68kOperand, Size as M68kSize, Xn as M68kXn,
};
use sigil_backend_m68k::M68kBackend;
use sigil_backend_z80::z80::{Cond, Mnemonic, Operand, Reg16, Reg8};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer, LowerError};
use sigil_ir::expr::{BinOp, Fold};
use sigil_ir::{
    asl_width_rule, AbsWidth, DataFragment, Expr, Fixup, FixupKind, IrBuilder, Module, SymbolTable,
    SymbolValue,
};
use sigil_span::{Diagnostic, Level, SourceId, Span};

const EXPAND_CAP: usize = 64;
const PASS_CAP: usize = 8;
/// Bound for `while … endm` (T9.2): caps re-evaluation/body-expansion
/// iterations so a non-convergent condition diagnoses (A5) instead of
/// hanging. Generous relative to any real `while`-driven table-fill idiom.
const WHILE_CAP: usize = 10_000;

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
        let PassOutput {
            module,
            env,
            macros: m,
            functions: f,
            mut diags,
            poison,
        } = one_pass(src, opts, &seed, &macros, &functions);
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
        message: format!(
            "assembly did not converge within {PASS_CAP} passes (symbol values still changing)"
        ),
        primary: Span {
            source: SourceId(0),
            start: 0,
            end: 0,
        },
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
    /// Front-end-only string-valued symbols (`.__str set "BUS ERROR"`).
    /// §7.4: strings NEVER enter `sigil_ir::SymbolValue`; they live here in the
    /// evaluator. Keyed by fully-qualified name exactly like `env` (see
    /// `resolve_str`). NOT carried across passes — asl `set` is a sequential
    /// per-pass assignment and every string symbol in the `__FSTRING` scan is
    /// assigned before it is read (probe p1/p4).
    str_env: std::collections::HashMap<String, String>,
    scope: Option<String>,
    in_section: bool,
    /// Continuous physical location counter (asl-faithful): the real ROM byte
    /// offset of the CURRENTLY-OPEN section's start. The live physical position is
    /// `phys_base + builder.current_offset()`; it advances with every emitted byte
    /// across ALL section switches (cpu/phase/dephase) and is NEVER rewound by
    /// `restore`. `org N` sets it directly; `phase`/`dephase` leave it untouched
    /// and instead adjust `state.disp`. VMA (`$`/labels) = physical + `disp`.
    phys_base: u32,
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
    /// Remaining `while`-body-execution budget for THIS pass (per-`Asm`, so it
    /// resets each pass). Complements the per-loop `WHILE_CAP`: two NESTED
    /// non-convergent `while`s each bounded at `WHILE_CAP` still multiply to
    /// `WHILE_CAP²` body runs, which can hang the pass. This global budget bounds
    /// the TOTAL across all (possibly nested) loops so a pathological input
    /// diagnoses in bounded time. Generous vs. any real table-fill loop.
    while_budget: usize,
}

/// Per-pass ceiling on total `while`-body executions (see `Asm::while_budget`).
/// Far above any real Aeon `while`-driven data table, far below the `WHILE_CAP²`
/// (10⁸) a pair of nested non-convergent loops would otherwise grind through.
const GLOBAL_WHILE_CAP: usize = 1_000_000;

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
            str_env: std::collections::HashMap::new(),
            scope: None,
            in_section: false,
            phys_base: 0,
            diags: Vec::new(),
            source: SourceId(0),
            functions: std::collections::BTreeMap::new(),
            macros: std::collections::BTreeMap::new(),
            macro_depth: 0,
            visited: std::collections::BTreeSet::new(),
            include_root: opts.include_root.clone(),
            aborted: false,
            poison_refs: Vec::new(),
            while_budget: GLOBAL_WHILE_CAP,
        }
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic {
            level: Level::Error,
            message: msg.into(),
            primary: span,
        });
    }

    /// The continuous PHYSICAL location counter (real ROM/LMA offset): the open
    /// section's `phys_base` plus its running byte cursor. When no section is open
    /// (just after cpu/phase/dephase closed one, before the next emit reopens it),
    /// `phys_base` has already absorbed the closed section's length, so the current
    /// physical position is simply `phys_base`.
    fn current_physical(&self) -> u32 {
        self.phys_base
            + if self.in_section {
                self.builder.current_offset()
            } else {
                0
            }
    }

    /// The current VMA (`$`/label address): `physical + phase displacement`. Under
    /// no phase (`disp == 0`) this equals the physical location; inside a `phase
    /// addr` block it equals `addr + bytes-since-phase` (the window VMA).
    fn here(&self) -> u32 {
        (self.current_physical() as i64 + self.state.disp) as u32
    }

    /// The current PC as a SIGN-EXTENDED 32→64-bit value: an address with bit 31
    /// set (the 68k RAM aliases `$FFFF0000`/`$FFFF8000`+) becomes NEGATIVE, exactly
    /// as asl stores a phased label (`$FFFFFFFFFFFF80AC` = −32596 for a label at
    /// `$FFFF80AC`). This is what makes `move.w #RAM_Label, d0` fold in range: the
    /// low-RAM address is a small negative that fits a signed word, whereas the
    /// raw unsigned `4294934700` overflows. Byte-identical to the unsigned form
    /// for every wider use (abs.l / `.l` immediate truncate back to the same 32
    /// bits; abs.w / disp16 take the same low word). ROM addresses (< `$80000000`)
    /// are unaffected — sign-extension is a no-op there.
    fn here_i64(&self) -> i64 {
        self.here() as i32 as i64
    }

    fn fold(&self, e: &Expr) -> Fold {
        let here = self.here_i64();
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
    ///
    /// Also expands the front-end-only `int(...)`/`sin(...)` and debug-string
    /// (`strlen`/`strstr`/`val`, with `substr`/`lowstring` nesting) builtins
    /// (T9.3) — the same two passes `directive_db` already ran before parsing
    /// a `dc.b` argument. Without this, `<name> set strstr(str,"%<")` (the
    /// idiom the debugger's `__FSTRING_*` macros use throughout) left
    /// `strstr(...)` as un-expanded tokens here, since `eval_all` backs
    /// `directive_set`/`while`/`if`/`rept`/`phase`/`org`/`align`/`ds`, and
    /// previously only `directive_db` wired the builtins in. Wiring them in
    /// HERE too — rather than only where the gap was first noticed — is what
    /// makes `while`+`set` actually compose with the string builtins, so a
    /// `while (pos>=0) / pos: set strstr(...)` loop (T9.2 `while` + T9.1
    /// `strstr`) now really assembles.
    fn eval_all(&mut self, toks: &[Token], span: Span) -> Option<i64> {
        let expanded = self.expand_calls(toks, 0);
        let expanded = self.expand_int_builtin(&expanded);
        let expanded = self.expand_str_builtins(&expanded);
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

    /// Evaluate front-end-only `int(...)` builtin calls in `toks` (§7.4:
    /// `sin`/`int` are FRONT-END builtins — they must NEVER become
    /// `sigil_ir::Expr` nodes, so this runs as token-level preprocessing
    /// BEFORE `crate::expr::parse_expr` ever sees the line). Scans for each
    /// top-level `int(` call, evaluates its single argument as an f64
    /// expression via `eval_float` (which recognizes nested `sin(...)`/
    /// `int(...)` calls itself, so `int(sin(int(x)))`-style nesting works),
    /// floors the result (AS's `int()` = floor, spike-0-verified against the
    /// 4 committed sine goldens), and replaces the whole `int(...)` span with
    /// a single resolved `Tok::Int` — a completely ordinary integer literal
    /// from here on, indistinguishable from one the source author wrote by
    /// hand. A bare `sin(...)` not wrapped in `int(...)` has no integer
    /// meaning and is left untouched (whatever consumes it downstream will
    /// report a normal "bad expression" diagnostic).
    fn expand_int_builtin(&mut self, toks: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if name == "int"
                    && matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    )
                {
                    let span = toks[i].span;
                    if let Some((args, next)) = split_call_args(toks, i + 1) {
                        let value = match args.as_slice() {
                            [arg] => self.eval_float(arg),
                            _ => None,
                        };
                        match value {
                            Some(v) => out.push(Token {
                                tok: Tok::Int(v.floor() as i64),
                                span,
                            }),
                            None => {
                                self.err(span, "int(): could not evaluate float expression");
                                out.push(Token {
                                    tok: Tok::Int(0),
                                    span,
                                });
                            }
                        }
                        i = next;
                        continue;
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Evaluate a front-end-only f64 expression tree: `+ - * /`, unary
    /// negation, parens, int/float literals, symbol lookups (resolved via the
    /// SAME env/scope as ordinary i64 folding, then promoted to f64), and
    /// nested `sin(...)`/`int(...)` calls. `None` on any unresolved symbol or
    /// malformed shape — mirrors `Fold::Poison` in spirit, but this whole tree
    /// stays out of `sigil_ir::Expr` (§7.4).
    fn eval_float(&self, toks: &[Token]) -> Option<f64> {
        let (v, rest) = self.parse_float_bp(toks, 0)?;
        rest.is_empty().then_some(v)
    }

    fn parse_float_bp<'t>(&self, toks: &'t [Token], min_bp: u8) -> Option<(f64, &'t [Token])> {
        let (mut lhs, mut rest) = self.parse_float_atom(toks)?;
        while let Some(Tok::Punct(p)) = rest.first().map(|t| &t.tok) {
            let bp = match p {
                Punct::Star | Punct::Slash => 8,
                Punct::Plus | Punct::Minus => 7,
                _ => break,
            };
            if bp <= min_bp {
                break;
            }
            let op = *p;
            let (rhs, r2) = self.parse_float_bp(&rest[1..], bp)?;
            lhs = match op {
                Punct::Star => lhs * rhs,
                Punct::Slash => lhs / rhs,
                Punct::Plus => lhs + rhs,
                Punct::Minus => lhs - rhs,
                _ => unreachable!(),
            };
            rest = r2;
        }
        Some((lhs, rest))
    }

    fn parse_float_atom<'t>(&self, toks: &'t [Token]) -> Option<(f64, &'t [Token])> {
        let (head, rest) = toks.split_first()?;
        match &head.tok {
            Tok::Float(f) => Some((*f, rest)),
            Tok::Int(n) => Some((*n as f64, rest)),
            Tok::Punct(Punct::Minus) => {
                let (v, r) = self.parse_float_atom(rest)?;
                Some((-v, r))
            }
            Tok::Punct(Punct::LParen) => {
                let (v, r) = self.parse_float_bp(rest, 0)?;
                match r.first().map(|t| &t.tok) {
                    Some(Tok::Punct(Punct::RParen)) => Some((v, &r[1..])),
                    _ => None,
                }
            }
            Tok::Ident(name)
                if (name == "sin" || name == "int")
                    && matches!(
                        rest.first().map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    ) =>
            {
                let (args, next) = split_call_args(rest, 0)?;
                let inner = match args.as_slice() {
                    [arg] => self.eval_float(arg)?,
                    _ => return None,
                };
                let v = if name == "sin" {
                    inner.sin()
                } else {
                    inner.floor()
                };
                Some((v, &rest[next..]))
            }
            Tok::Ident(name) => {
                let v = if name == "$" {
                    self.here_i64()
                } else {
                    self.env.resolve(name, self.scope.as_deref())?
                };
                Some((v as f64, rest))
            }
            Tok::Dollar => Some((self.here() as f64, rest)),
            _ => None,
        }
    }

    /// Evaluate front-end-only debug-string builtin calls
    /// (`strlen`/`strstr`/`val`) in `toks`, replacing each TOP-LEVEL call span
    /// with a resolved `Tok::Int` — the same shape as `expand_int_builtin`
    /// (§7.4: these are FRONT-END builtins; the string values involved never
    /// become `sigil_ir::Expr` nodes). `substr(...)` itself produces a
    /// STRING, not an int, so it is never substituted at this top level — it
    /// is only ever consumed as a nested argument (via `eval_str`) inside one
    /// of these three, which is how `strlen(substr(...))` /
    /// `strstr(substr(s,p,0),">")` nesting works.
    fn expand_str_builtins(&mut self, toks: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if matches!(name.as_str(), "strlen" | "strstr" | "val")
                    && matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    )
                {
                    let span = toks[i].span;
                    if let Some((args, next)) = split_call_args(toks, i + 1) {
                        match self.eval_str_builtin(name, &args) {
                            Some(v) => out.push(Token {
                                tok: Tok::Int(v),
                                span,
                            }),
                            None => {
                                self.err(
                                    span,
                                    format!("{name}(): could not evaluate string builtin"),
                                );
                                out.push(Token {
                                    tok: Tok::Int(0),
                                    span,
                                });
                            }
                        }
                        i = next;
                        continue;
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Dispatch one of the debug-string builtins that produce an INTEGER:
    ///
    /// - `strlen(str)` → character count.
    /// - `strstr(haystack, needle)` → **STANDARD** 0-based index of the first
    ///   match, or **-1** if absent (asl 1.42 Bld 212 verified: `strstr("abc",
    ///   "c")`=2, `strstr("b>",">")`=1, `strstr("xab","ab")`=1,
    ///   `strstr("abc","z")`=-1 — the alleged "fails on last char" bug does
    ///   NOT reproduce in this asl; deliberately NOT emulated here).
    /// - `val(str)` → re-lexes `str` fresh and folds it as an ordinary AS
    ///   constant expression against the CURRENT env/scope (NOT just a number
    ///   parse): `val("$80")`=0x80, `val("144")`=144, `val("hex+1")` resolves
    ///   symbol `hex` the same way any operand would.
    fn eval_str_builtin(&self, name: &str, args: &[Vec<Token>]) -> Option<i64> {
        match (name, args) {
            ("strlen", [s]) => Some(self.eval_str(s)?.chars().count() as i64),
            ("strstr", [hay, needle]) => {
                let hay = self.eval_str(hay)?;
                let needle = self.eval_str(needle)?;
                Some(match hay.find(&needle) {
                    // `find` returns a BYTE offset; convert to a char count so
                    // a (hypothetical) non-ASCII haystack still reports the
                    // same index asl's char-oriented `strstr` would.
                    Some(byte_idx) => hay[..byte_idx].chars().count() as i64,
                    None => -1,
                })
            }
            ("val", [s]) => self.fold_str_as_expr(&self.eval_str(s)?),
            _ => None,
        }
    }

    /// Resolve a bare identifier reference to its string value, if it names a
    /// string-valued `set` symbol. Key-building mirrors `SymbolTable::resolve`:
    /// `.foo` → `"{scope}.foo"` (needs a scope), `A.b`/`foo` → verbatim.
    fn resolve_str(&self, name: &str) -> Option<String> {
        let key = if let Some(local) = name.strip_prefix('.') {
            format!("{}.{}", self.scope.as_deref()?, local)
        } else {
            name.to_string()
        };
        self.str_env.get(&key).cloned()
    }

    /// Evaluate a front-end-only STRING expression: a plain `Tok::Str`
    /// literal, or a nested `substr(str, pos, len)` / `lowstring(str)` call.
    /// `None` on any other shape (mirrors `Fold::Poison` in spirit — this
    /// value never becomes a `sigil_ir::Expr`, per §7.4). Both nested forms
    /// recurse through `eval_str` for their own string argument, so
    /// `lowstring(substr(...))` / `substr(lowstring(...), ...)` nest freely
    /// (T9.3).
    fn eval_str(&self, toks: &[Token]) -> Option<String> {
        if let [Token {
            tok: Tok::Str(s), ..
        }] = toks
        {
            return Some(s.clone());
        }
        if let [Token {
            tok: Tok::Ident(name),
            ..
        }] = toks
        {
            if let Some(s) = self.resolve_str(name) {
                return Some(s);
            }
        }
        if let [Token {
            tok: Tok::Ident(name),
            ..
        }, ..] = toks
        {
            if name == "substr"
                && matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen)))
            {
                let (args, next) = split_call_args(toks, 1)?;
                if next == toks.len() {
                    return self.eval_substr(&args);
                }
            }
            if name == "lowstring"
                && matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen)))
            {
                let (args, next) = split_call_args(toks, 1)?;
                if next == toks.len() {
                    if let [s_toks] = args.as_slice() {
                        return self.eval_str(s_toks).map(|s| s.to_lowercase());
                    }
                }
            }
        }
        None
    }

    /// `substr(str, pos, len)`: 0-based `pos`; `len == 0` means "from `pos`
    /// to the end of the string" (asl-verified: `substr("hello",1,0)` =
    /// "ello", `substr("hello",1,2)` = "el"). `pos`/`len` are ordinary
    /// constant expressions (a literal, a symbol, arithmetic, …) — not
    /// further string-builtin calls; only the first (`str`) argument nests.
    fn eval_substr(&self, args: &[Vec<Token>]) -> Option<String> {
        let [s_toks, pos_toks, len_toks] = args else {
            return None;
        };
        let s = self.eval_str(s_toks)?;
        let pos = self.fold_const(pos_toks)?;
        let len = self.fold_const(len_toks)?;
        if pos < 0 {
            return None;
        }
        let chars: Vec<char> = s.chars().collect();
        let pos = pos as usize;
        if pos > chars.len() {
            return None;
        }
        let end = match len {
            0 => chars.len(),
            n if n > 0 => (pos + n as usize).min(chars.len()),
            _ => return None,
        };
        Some(chars[pos..end].iter().collect())
    }

    /// `val(str)`: lex `text` fresh (under the CURRENT cpu context) and fold
    /// it as an ordinary constant expression — this is what makes `val` an
    /// AS-EXPRESSION evaluator rather than a plain number parse (it resolves
    /// symbols, honors `$`-prefixed hex, arithmetic, …).
    fn fold_str_as_expr(&self, text: &str) -> Option<i64> {
        let toks = lex_line(text, self.state.cpu, self.source, 0).ok()?;
        self.fold_const(&toks)
    }

    /// Fold a token slice as a plain constant integer expression — the
    /// immutable counterpart of `eval_all` (no diagnostics on failure; `None`
    /// mirrors `Fold::Poison`). Used by the debug-string evaluator wherever a
    /// nested piece is known to be an INTEGER, never a string (`substr`'s
    /// `pos`/`len` arguments, and `val`'s re-lexed expression text).
    fn fold_const(&self, toks: &[Token]) -> Option<i64> {
        let expanded = self.expand_calls(toks, 0);
        let (e, rest) = crate::expr::parse_expr(&expanded)?;
        if !rest.is_empty() {
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
        let span = toks.first().map(|t| t.span).unwrap_or(Span {
            source: self.source,
            start: line.base,
            end: line.base,
        });
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
                [Token {
                    tok: Tok::Ident(p), ..
                }] => params.push(p.clone()),
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
                    if matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    ) {
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
    fn substitute(
        &self,
        body: &[Token],
        params: &[String],
        args: &[Vec<Token>],
        depth: usize,
    ) -> Vec<Token> {
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
        let raw = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
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
        self.eval_all(
            &toks,
            Span {
                source: self.source,
                start: 0,
                end: 0,
            },
        )
    }

    /// `include "path"`: read a file relative to `include_root`, exec its lines
    /// inline. A visited-set prevents re-inclusion (DAG, not tree).
    fn directive_include(&mut self, rest: &[Token], span: Span) {
        let rel = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
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

    /// `BINCLUDE "path"`: read a file's raw bytes and emit them verbatim —
    /// opaque binary data, no parsing (asl-verified: a file containing `ABCD`
    /// emits `41 42 43 44`). Path resolves via `include_root` exactly like
    /// `include` (real Aeon source paths are relative to the aeon root, e.g.
    /// `BINCLUDE "games/sonic4/data/collision/heightmaps.bin"`). Unlike
    /// `include`, this is NOT re-entrancy-guarded by `self.visited` — every
    /// real usage in Aeon is a bare, single-use `BINCLUDE "path"` (no
    /// offset/length args; verified via `grep -rn BINCLUDE` over
    /// `aeon/games` + `aeon/engine`, all 43 call sites bare), and unlike
    /// `include` (which execs the file's lines and so must not re-enter a
    /// cycle), re-BINCLUDEing the same path is a legitimate way to place the
    /// same blob at two different labels — a DAG guard would silently drop
    /// the second copy.
    fn directive_binclude(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        let rel = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
            Some(p) => p,
            None => {
                self.err(span, "BINCLUDE needs a quoted path");
                return;
            }
        };
        let path = match &self.include_root {
            Some(root) => root.join(&rel),
            None => std::path::PathBuf::from(&rel),
        };
        match std::fs::read(&path) {
            Ok(bytes) => self.emit(&bytes, vec![], span),
            Err(e) => self.err(span, format!("cannot BINCLUDE {}: {e}", path.display())),
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
                Some("while") => {
                    i = self.exec_while(lines, i);
                }
                Some("switch") => {
                    i = self.exec_switch(lines, i);
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
            // `NAME: = expr` / `NAME: equ expr`: a colon-label immediately
            // followed by an equate directive defines NAME as a CONSTANT, not a
            // PC label — AS tolerates the decorative colon on an equate (Aeon
            // writes both `RESET_RAM: = $FFFFFF00` and
            // `DEBUGGER__EXTENSIONS__ENABLE: equ 1`). Detect it here so we bind
            // the value rather than emitting a stray location label.
            let b = &parsed.tokens;
            let is_eq = matches!(b.first().map(|t| &t.tok), Some(Tok::Punct(Punct::Eq)));
            let is_equ = matches!(b.first().map(|t| &t.tok), Some(Tok::Ident(s)) if s == "equ");
            if (is_eq || is_equ) && b.len() >= 2 {
                let span = b[0].span;
                self.directive_equate(&name, &b[1..], span);
                return;
            }
            // `NAME: set expr` / `NAME: := expr` — a colon-label immediately
            // followed by a REASSIGNABLE-symbol directive binds NAME as a
            // reassignable value, not a PC label (the colon is decorative, exactly
            // as with `NAME: =`/`NAME: equ` above). This is the shape the
            // debugger's `__FSTRING_*` string-scan macros use for their loop
            // cursor: `.__pos: set strstr(...)+.__pos+2` — a `set` that MUST
            // reassign `.__pos` each iteration so the `while (strstr(...)>=0)`
            // guard makes progress and terminates. Treating it as a PC label
            // instead froze `.__pos` at the current address, so the loop never
            // found its end marker (infinite-loop → unbounded label emission).
            let is_set_kw = matches!(b.first().map(|t| &t.tok), Some(Tok::Ident(s)) if s == "set");
            let is_coloneq = matches!(b.first().map(|t| &t.tok), Some(Tok::Punct(Punct::ColonEq)));
            if (is_set_kw || is_coloneq) && b.len() >= 2 {
                let span = b[0].span;
                self.directive_set(&name, &b[1..], span);
                return;
            }
            self.define_label(&name);
        }
        let mut body = parsed.tokens;
        // `!name` builtin escape (T9.2, asl-verified): a leading `!` forces
        // AS's builtin directive `name` over any same-named user macro —
        // `!error "msg"` / `!align N`. Core carries no macro that shadows a
        // builtin, so the escape reduces to: strip the `!` and dispatch
        // exactly as `name args…` would. (This is unrelated to `!` as the
        // bitwise-or operator — that only ever appears mid-expression,
        // inside an already-consumed head's operand tokens, never as the
        // line's very first token, so there is no ambiguity to resolve.)
        if matches!(body.first().map(|t| &t.tok), Some(Tok::Punct(Punct::Bang))) {
            body = body[1..].to_vec();
        }
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
        // `name equ <expr>` — AS's constant-equate keyword form (equivalent to
        // `name = <expr>` for our purposes). Intercepted here, before the
        // mnemonic/bare-label fallback, for the same reason as `=`/`set`: the
        // head is the symbol NAME, not a mnemonic. Without this, `hex equ $80`
        // defines a stray label `hex` then dispatches `equ` as an instruction,
        // and `dec equ $90` is worse still — `dec` IS a Z80 mnemonic, so the
        // whole line routes to instruction lowering and errors under 68000.
        if matches!(body.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if s == "equ") {
            self.directive_equate(&head, &body[2..], body[0].span);
            return;
        }
        // `name set <expr>` / `name := <expr>` — AS's reassignable-symbol forms
        // (T8). Checked here, before the mnemonic/bare-label fallback below,
        // for the same reason as the `=` equate check just above: the head
        // (`head`) is the accumulator's NAME, not a mnemonic, so without this
        // early intercept a 68000 line like `i set 0` would fall into the
        // bare-label path, define a label `i` at the current PC, and then try
        // to dispatch `set` itself as an instruction (and fail — `set` is
        // only a recognized mnemonic under Z80). `:=` lexes as the single
        // `ColonEq` token (see `token::Punct::ColonEq`), never as `Colon`
        // then `Eq`, so it can never be confused with a `name:` colon-label.
        let is_set_kw = matches!(body.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if s == "set");
        let is_coloneq = matches!(
            body.get(1).map(|t| &t.tok),
            Some(Tok::Punct(Punct::ColonEq))
        );
        if is_set_kw || is_coloneq {
            self.directive_set(&head, &body[2..], body[0].span);
            return;
        }
        if !is_op_keyword(&head)
            && !is_mnemonic(&head)
            && !self.macros.contains_key(&head)
            && !self.is_attribute_macro_head(&head)
        {
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
        let body = if parsed.label_colon.is_some() {
            parsed.tokens
        } else {
            toks
        };
        if body.is_empty() {
            return None;
        }
        let name = match &body[0].tok {
            Tok::Ident(s) => s.clone(),
            _ => return None,
        };
        let second = body.get(1).and_then(|t| {
            if let Tok::Ident(s) = &t.tok {
                Some(s.as_str())
            } else {
                None
            }
        });
        if matches!(second, Some("macro") | Some("struct") | Some("function")) {
            return Some((second.unwrap().to_string(), 1, body));
        }
        if self.macros.contains_key(&name) {
            return Some((name, 0, body));
        }
        if is_op_keyword(&name) || is_mnemonic(&name) {
            return Some((name, 0, body));
        }
        if let Some(Token {
            tok: Tok::Ident(s), ..
        }) = body.get(1)
        {
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
        let fallback = Span {
            source: self.source,
            start: line.base,
            end: line.base,
        };
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
    /// tracking nested blocks with a STACK of expected-closer sets (keyed by
    /// each nested opener's own kind via [`closers_for`]) rather than a flat
    /// depth count keyed on a single caller-supplied opener/closer pair.
    ///
    /// This distinction matters because several DIFFERENT block kinds share
    /// the same literal closer keyword in real AS: `while … endm` AND
    /// `macro … endm` (AND `rept`, which may close with either `endr` or
    /// `endm`) all terminate on `endm`. A flat counter keyed on just the
    /// outer call's own opener (e.g. `capture_macro` passing
    /// `openers=["macro"]`) does NOT increment on a NESTED `while`, so the
    /// nested while's own `endm` was mistaken for the enclosing macro's
    /// `endm` — truncating the macro body before its real end (T9.3
    /// investigation: a `macro` containing a `while … endm` loop, exactly
    /// the shape `__FSTRING_GenerateDecodedString`-style debug macros need,
    /// silently lost its tail and looped forever). The stack fixes this: a
    /// nested opener of ANY kind pushes ITS OWN closer set, so only that
    /// closer set's keyword pops it — regardless of what closer keyword the
    /// enclosing block happens to share with it.
    fn find_block_end(&self, lines: &[SrcLine], start: usize) -> usize {
        let start_kw = self.line_keyword(&lines[start]).unwrap_or_default();
        let mut stack: Vec<&'static [&'static str]> = vec![closers_for(&start_kw)];
        for (idx, line) in lines.iter().enumerate().skip(start + 1) {
            let Some(k) = self.line_keyword(line) else {
                continue;
            };
            let nested_closers = closers_for(&k);
            if !nested_closers.is_empty() {
                // A nested block opener (if/ifdef/ifndef, rept, while, macro,
                // struct, switch) — push its own closer set; only ITS
                // matching closer pops this frame.
                stack.push(nested_closers);
                continue;
            }
            if let Some(top) = stack.last() {
                if top.contains(&k.as_str()) {
                    stack.pop();
                    if stack.is_empty() {
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
        let end = self.find_block_end(lines, start);
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
                Some("if") | Some("ifdef") | Some("ifndef") => {
                    self.eval_cond(kw.as_deref().unwrap(), &argtoks, span)
                }
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

    /// Handle `switch <str-expr> / case "s1" / … / elsecase / … / endcase`
    /// (T9.3, asl-verified): assembles ONLY the body of the first `case`
    /// whose literal string equals the switch value; `elsecase` is the
    /// default (chosen when reached, mirroring `exec_if`'s `else` arm — same
    /// arm-collection shape as `exec_if`, but keyed on STRING equality
    /// against each `case`'s literal instead of a boolean condition). The
    /// switch expression and each `case` literal are evaluated through
    /// `eval_str` (so `switch lowstring(...)` / nested `substr` all compose,
    /// exactly as the debugger's `__FSTRING_*` macros use them). An
    /// unresolved switch expression, or a `case` whose argument isn't a
    /// string, diagnoses but does not abort the block scan. Returns the
    /// index past `endcase`.
    fn exec_switch(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let end = self.find_block_end(lines, start);
        let switch_val = self.eval_str(&arg_toks);
        if switch_val.is_none() {
            self.err(span, "switch needs a string expression");
        }
        // Collect arm-head indices at depth 0: each `case "lit"` (Some(lit))
        // and `elsecase` (None, the default), mirroring `exec_if`'s
        // if/elseif/else head collection but depth-counting `switch`/`endcase`
        // instead of `if`/`endif`.
        let mut heads: Vec<(usize, Option<String>)> = Vec::new();
        let mut depth = 0i32;
        for (idx, line) in lines.iter().enumerate().take(end).skip(start + 1) {
            match self.line_keyword(line).as_deref() {
                Some("switch") => depth += 1,
                Some("endcase") => depth -= 1,
                Some("case") if depth == 0 => {
                    let (_, cargs, cspan) = self.line_kw_args(line);
                    let lit = self.eval_str(&cargs);
                    if lit.is_none() {
                        self.err(cspan, "case needs a string literal");
                    }
                    heads.push((idx, lit));
                }
                Some("elsecase") if depth == 0 => heads.push((idx, None)),
                _ => {}
            }
        }
        heads.push((end, None)); // sentinel
        for w in 0..(heads.len() - 1) {
            let (head, lit) = heads[w].clone();
            let take = match &lit {
                Some(s) => switch_val.as_deref() == Some(s.as_str()),
                None => true, // elsecase: default, taken if reached
            };
            if take {
                let body = &lines[head + 1..heads[w + 1].0];
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
        let end = self.find_block_end(lines, start);
        let body = &lines[start + 1..end];
        for _ in 0..n {
            self.exec(body);
        }
        end + 1
    }

    /// Handle `while (cond) … endm` (T9.2, asl-verified — NOT `endw`: asl
    /// errors "WHILE without ENDM"). Unlike `rept`'s once-folded count, the
    /// condition is a live expression re-evaluated every iteration (typically
    /// against a `set` accumulator advanced in the body), so this can't fold
    /// it once up front the way `exec_rept` does. Bounded by `WHILE_CAP`
    /// with a non-convergence diagnostic (A5, `SIGIL_CORE_SPEC.md` §7.1/§10.4
    /// — the same bounded-loop-or-diagnose contract as the pass loop
    /// (`PASS_CAP`) and macro expansion (`EXPAND_CAP`)) so a condition that
    /// can never resolve to zero can't hang the assembler. Returns the index
    /// past `endm`.
    fn exec_while(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let end = self.find_block_end(lines, start);
        let body = &lines[start + 1..end];
        let mut iterations = 0usize;
        loop {
            if self.aborted {
                break;
            }
            match self.eval_all(&arg_toks, span) {
                Some(0) => break,
                Some(_) => {
                    if iterations >= WHILE_CAP {
                        self.err(
                            span,
                            format!("while loop did not terminate within {WHILE_CAP} iterations (non-convergent condition?)"),
                        );
                        break;
                    }
                    if self.while_budget == 0 {
                        self.err(
                            span,
                            format!("total `while` iterations exceeded the per-pass budget ({GLOBAL_WHILE_CAP}) — a non-convergent (possibly nested) loop"),
                        );
                        self.aborted = true;
                        break;
                    }
                    self.while_budget -= 1;
                    self.exec(body);
                    iterations += 1;
                }
                None => {
                    self.err(span, "unresolved while condition");
                    break;
                }
            }
        }
        end + 1
    }

    /// Handle name-first `Name struct … Name endstruct`: define packed
    /// `Name_field` offsets and `Name_len`. Field lines emit no bytes. Returns the
    /// index past `endstruct`. (Mirrors `capture_macro`: name at `toks[0]`,
    /// `struct` at `toks[1]`.)
    fn capture_struct(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let toks = lex_line(
            &lines[start].text,
            self.state.cpu,
            self.source,
            lines[start].base,
        )
        .unwrap_or_default();
        let span = toks.first().map(|t| t.span).unwrap_or(Span {
            source: self.source,
            start: lines[start].base,
            end: lines[start].base,
        });
        let name = match toks.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "struct needs a name");
                String::new()
            }
        };
        let end = self.find_block_end(lines, start);
        let mut off: i64 = 0;
        for l in &lines[start + 1..end] {
            if let Some((field, width, count)) = self.parse_struct_field(l) {
                // An anonymous reserve field (`ds.b 1` with no name) advances the
                // struct offset but defines no member symbol.
                if !field.is_empty() {
                    self.env
                        .define(&format!("{name}_{field}"), SymbolValue::Int(off));
                }
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
        self.env
            .define(&format!("{name}_len"), SymbolValue::Int(off));
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
                // Anonymous reserve field: a bare `ds.b|ds.w|ds.l N` with no
                // preceding name (e.g. Act's `ds.b 1 ; reserved (pad to word)`).
                // AS still advances the struct offset by its size; it just binds
                // no member symbol. Emit an empty field name and keep the whole
                // token slice (the `ds.*` keyword is the width token).
                Some((
                    Token {
                        tok: Tok::Ident(s), ..
                    },
                    _,
                )) if matches!(s.as_str(), "ds.b" | "ds.w" | "ds.l") => {
                    (String::new(), parsed.tokens.clone())
                }
                Some((
                    Token {
                        tok: Tok::Ident(s), ..
                    },
                    r,
                )) => (s.clone(), r.to_vec()),
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
            if let Some(Token {
                tok: Tok::Str(rhs), ..
            }) = toks.get(pos + 1)
            {
                let lhs = match &toks[..pos] {
                    [Token {
                        tok: Tok::Str(s), ..
                    }] => Some(s.clone()),
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
            [Token {
                tok: Tok::Ident(n), ..
            }] if n == "MOMCPUNAME" => Some(match self.state.cpu {
                Cpu::Z80 => "Z80".into(),
                Cpu::M68000 => "68000".into(),
            }),
            _ => None,
        }
    }

    /// Whether `head` names a `.ATTRIBUTE`-suffix invocation of a captured
    /// macro (T9.2): `head` itself isn't a known macro, but stripping a
    /// trailing `.b`/`.w`/`.l`/`.s` yields one that is. Checked before the
    /// M68000 bare-label/mnemonic-column heuristic in `exec_one` (so
    /// `foo.w d1` — a macro invocation — dispatches, rather than being
    /// mistaken for a label) and drives `dispatch`'s own attribute-macro
    /// arm below.
    fn is_attribute_macro_head(&self, head: &str) -> bool {
        !self.macros.contains_key(head)
            && split_attribute_suffix(head).is_some_and(|(base, _)| self.macros.contains_key(base))
    }

    fn dispatch(&mut self, head: &str, rest: &[Token], span: Span) {
        if let Some((base, suffix)) = split_attribute_suffix(head) {
            if !self.macros.contains_key(head) && self.macros.contains_key(base) {
                self.expand_macro_with_attribute(base, rest, suffix);
                return;
            }
        }
        match head {
            "cpu" => self.directive_cpu(rest, span),
            "phase" => self.directive_phase(rest, span),
            "dephase" => self.directive_dephase(),
            "org" => self.directive_org(rest, span),
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
            // `set NAME, VALUE` — asl's comma-operand spelling of the SET
            // directive (the reassignable-symbol assignment, name in the
            // OPERAND column rather than the label column). Aeon writes this in
            // `rept`-unrolled data init (`set .c, 0` / `set .c, .c+DMAEntry_len`).
            // Verified against asl: `set .c, 0` assigns `.c = 0` exactly like
            // `.c set 0`. Gated to 68000 so the Z80 `set BIT,(ix+d)` bit
            // instruction (same head word) still routes to Z80 lowering below.
            "set" if self.state.cpu == Cpu::M68000 => self.directive_set_comma(rest, span),
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
            // Matched by exact case, not lowercased: this front-end never
            // case-folds identifiers (see `is_op_keyword`/`lex_line` — every
            // other directive here is matched against the exact spelling
            // real Aeon source uses, e.g. lowercase `include`/`org`). Real
            // source spells this directive uppercase at all 43 call sites
            // (`grep -rn BINCLUDE aeon/games aeon/engine`), never `binclude`.
            "BINCLUDE" => self.directive_binclude(rest, span),
            // `END` (asl's end-of-source / entry-point directive). Emits no
            // bytes — bare `END` and `END <entrypoint>` are both emission
            // no-ops (probe: 2026-07-04-m1d-t2-abs-ea-end-probes.md). Aeon's
            // only use is the bare `END` at main.asm:446. Exact-case like
            // `BINCLUDE`; does not collide with the `endif`/`endm`/`endr`/
            // `endcase` block closers (handled in block scanning, not dispatch).
            "end" | "END" => {}
            _ if self.macros.contains_key(head) => self.expand_macro(head, rest),
            // `is_mnemonic` only recognizes Z80 mnemonics; under `cpu 68000` the
            // m68k dispatch (lower_m68k) is still a stub (M1.C T4/T5), so any
            // non-directive head is routed there rather than misreported as
            // "unknown directive or mnemonic".
            _ if self.state.cpu == Cpu::Z80 && is_mnemonic(head) => {
                self.lower_instruction(head, rest, span)
            }
            _ if self.state.cpu == Cpu::M68000 => self.lower_instruction(head, rest, span),
            _ => self.err(span, format!("unknown directive or mnemonic `{head}`")),
        }
    }

    fn open_section_if_needed(&mut self) {
        if !self.in_section {
            // Physical LMA of this section's start = the continuous counter's
            // current value (`phys_base`, already advanced past any closed
            // section). The phased VMA base = physical + `disp` (equals the LMA
            // when not phased). Name by VMA base so the two real output regions
            // stay `sec0`/`sec32768` (the harness/M0 gate keys on those names).
            let vma_base = (self.phys_base as i64 + self.state.disp) as u32;
            let name = format!("sec{vma_base}");
            self.builder
                .switch_section_lma(&name, self.state.cpu, Some(vma_base), self.phys_base);
            self.in_section = true;
        }
    }

    /// Close the open section, folding its emitted length into the continuous
    /// physical counter so the NEXT section starts at the right ROM offset.
    /// Idempotent: a second call while already closed does nothing (so a directive
    /// that closes an already-closed region can't double-advance `phys_base`).
    fn close_section(&mut self) {
        if self.in_section {
            self.phys_base += self.builder.current_offset();
            self.in_section = false;
        }
    }

    fn define_label(&mut self, name: &str) {
        self.open_section_if_needed();
        let value = self.here_i64();
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
        // The `cpu` directive resets padding/supmode to the CPU default,
        // unconditionally (asl-verified — see state.rs::set_cpu). Aeon's real
        // `padding off` at main.asm:3 therefore survives only until the first
        // subsequent `cpu` directive / cpu-changing `restore` (boot.asm's z80
        // load blocks), after which padding is ON for the rest of the ROM.
        self.state.set_cpu(cpu);
        self.close_section();
    }

    /// asl `padding on` (68000) inserts a single `$00` byte before a word-or-
    /// larger datum (`dc.w`/`dc.l`/any instruction) whose logical PC `$` is odd,
    /// keeping 68k data/code word-aligned. Alignment is on the LOGICAL `$`
    /// (`physical + phase disp`), not the physical offset — asl-verified (the
    /// `phase_logodd`/`phase_logeven` probes in
    /// `docs/superpowers/notes/2026-07-04-m1d-t0.1-padding-probes.md`). No-op
    /// under `padding off` (Aeon's initial state), on a Z80 CPU (byte stream), or
    /// at an even `$`. `dc.b` never calls this (alignment 1).
    fn pad_word_align(&mut self, span: Span) {
        if self.state.padding && self.state.cpu == Cpu::M68000 && !self.here().is_multiple_of(2) {
            self.open_section_if_needed();
            self.emit(&[0x00], vec![], span);
        }
    }

    fn directive_phase(&mut self, rest: &[Token], span: Span) {
        match self.eval_all(rest, span) {
            Some(v) => {
                // `phase addr` makes `$` report `addr` at the current physical
                // point WITHOUT moving the physical counter: set the displacement
                // to `addr - physical_now`. Compute the physical point BEFORE
                // closing the section (close folds the length into `phys_base`,
                // which leaves `current_physical()` unchanged — but order-safe).
                let phys_now = self.current_physical();
                self.close_section();
                self.state.disp = v - phys_now as i64;
            }
            None => self.err(span, "phase needs a constant expression"),
        }
    }

    fn directive_dephase(&mut self) {
        // Cancel the phase: `$` reports the physical location again. The physical
        // counter has ADVANCED by the phased block's bytes (folded into `phys_base`
        // by `close_section`), so labels after `dephase` continue from there — they
        // are NOT rewound. `disp` returns to 0 (an explicit balance of `phase`;
        // `restore` never touches it).
        self.close_section();
        self.state.disp = 0;
    }

    /// AS `org <target>` (M1.C T6b). `target` is an ABSOLUTE address (like
    /// `phase`'s argument), evaluated eagerly (matching `directive_align`/
    /// `directive_ds`'s pattern of resolving directive arguments at eval time
    /// rather than deferring an `Expr` into the fragment). Two cases, per the
    /// asl-verified back-patch + absolute-org rules (M1.C T6b investigation):
    ///
    /// - **No section open yet** (e.g. `main.asm`'s very first `org 0`, before
    ///   any byte has been emitted): behaves exactly like `phase`'s no-section
    ///   path — just records the base for the next emit to open a section at.
    /// - **A section IS open** and `target` falls within bytes the section has
    ///   ALREADY written (`target - base <= builder.extent()`): an in-section
    ///   back-patch seek (`org pscStart / dc.b n / org pscEndPos`, the
    ///   `parallax_section_end` idiom) — `IrBuilder::seek` repositions the
    ///   cursor; subsequent `Data`/`Fill` overwrite in place.
    /// - Otherwise (`target` is beyond anything written): a forward jump into
    ///   brand-new territory (`main.asm`'s `org $10000` starting the object
    ///   code bank) — closing the section and re-phasing at `target`, so the
    ///   gap is filled by `flatten`'s ordinary inter-section gap-fill instead of
    ///   growing this section's `Org`+`JmpJsrSym` mix (which `resolve_layout`
    ///   refuses — see its guard — since real engine code between `org 0` and
    ///   `org $10000` contains bare `jmp`/`jsr`).
    fn directive_org(&mut self, rest: &[Token], span: Span) {
        let target_abs = match self.eval_all(rest, span) {
            Some(v) => v as u32,
            None => {
                self.err(span, "org needs a constant expression");
                return;
            }
        };
        // `org N` sets the location counter so `$` == N. `$` == physical + disp,
        // so the physical target is `N - disp` (reduces to N outside any phase,
        // which is every real `org` site). Setting `phys_base` directly is how the
        // physical counter jumps.
        let phys_target = (target_abs as i64 - self.state.disp) as u32;
        if !self.in_section {
            self.phys_base = phys_target;
            return;
        }
        // A section is open. `base` is the VMA of its first byte; `rel` is the
        // target's offset within it. Within the already-written extent this is an
        // in-place back-patch seek (`parallax_section_end`); beyond it, a forward
        // jump that closes the section and re-bases the physical counter (so the
        // gap is inter-section gap-fill, not a growing Org+JmpJsrSym run).
        let base = (self.phys_base as i64 + self.state.disp) as u32;
        if target_abs < base {
            self.err(span, "org target precedes the current phase base");
            return;
        }
        let rel = target_abs - base;
        if rel <= self.builder.extent() {
            self.builder.seek(rel, 0, span);
        } else {
            self.close_section();
            self.phys_base = phys_target;
        }
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

    /// `name set <expr>` / `name := <expr>` (T8): AS's reassignable-symbol
    /// forms, e.g. Aeon's band counters / `OE_PREV_X` sort checks / deform
    /// accumulators. `eval_all` folds `rest` against `self.env` AS IT STANDS
    /// AT THIS LINE, so a self-reference (`i set i+5`) reads the CURRENT
    /// value of `i` — the redefinition below then overwrites it, giving
    /// emission-order imperative semantics (verified against real asl: `i set
    /// 0 / dc.b i / i set i+5 / dc.b i` → `00 05`). Deliberately its own
    /// function rather than an alias of `directive_equate`: `=` is slated to
    /// grow a single-assignment redefinition diagnostic (see that function's
    /// doc), and `set`/`:=` must keep permitting redefinition when it does.
    fn directive_set(&mut self, name: &str, rest: &[Token], span: Span) {
        let q = qualify(name, self.scope.as_deref());
        // asl: `set` may bind a STRING (`.__str set "BUS ERROR"`,
        // `.__str set substr(.__str,0,.__pos)`). Detect the string shape via
        // `eval_str` (literal / substr / lowstring / string-symbol copy) BEFORE
        // the numeric fold, and store it front-end-only (§7.4). Probe p1/p4.
        //
        // INVARIANT (relied on, not enforced): a symbol is int XOR string within
        // a pass. The string branch writes `str_env`, the int branch writes
        // `env`, and neither clears the other, so a `set` that FLIPS a symbol's
        // type mid-pass would leave stale entries in both maps and resolve to
        // whichever the use site consults. This is safe for every real target
        // (the `__FSTRING` scan assigns each symbol one stable type before it is
        // read — probe p1/p4); type-flipping `set` is unsupported. Poison-
        // shadowing the counterpart would be un-probed asl semantics, so it is
        // deliberately NOT done here.
        if let Some(s) = self.eval_str(rest) {
            self.str_env.insert(q, s);
            return;
        }
        if let Some(v) = self.eval_all(rest, span) {
            self.env.define(&q, SymbolValue::Int(v));
        }
    }

    /// `set NAME, VALUE` — the comma-operand form of SET (see the dispatch
    /// arm). Splits the first top-level comma into the target symbol name and
    /// the value expression, then reuses `directive_set`.
    fn directive_set_comma(&mut self, rest: &[Token], span: Span) {
        let groups = split_top_commas(rest);
        if groups.len() != 2 {
            self.err(span, "`set` directive expects `NAME, value`");
            return;
        }
        let name = match groups[0] {
            [Token {
                tok: Tok::Ident(s), ..
            }] => s.clone(),
            _ => {
                self.err(span, "`set` directive target must be a bare symbol");
                return;
            }
        };
        self.directive_set(&name, groups[1], span);
    }

    fn directive_db(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let called = self.expand_calls(g, 0);
            let expanded = self.expand_int_builtin(&called);
            let expanded = self.expand_str_builtins(&expanded);
            // (T6c) A STRING operand — a plain `Tok::Str` literal or a
            // string-builtin call that resolves to one (`substr(...)`,
            // `lowstring(...)`) — emits one ASCII byte per character
            // instead of folding as a numeric expression (asl-verified:
            // `dc.b "AB"` -> `41 42`; `dc.b substr("hello",1,2)` -> `65 6C`).
            // This is the shape only, checked BEFORE the numeric parse below
            // so plain numeric/symbol operands are unaffected.
            if let Some(s) = self.eval_str(&expanded) {
                let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
                self.emit(&bytes, vec![], span);
                continue;
            }
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
                            vec![Fixup {
                                kind: FixupKind::BankPtr16Le,
                                offset: 0,
                                target: qe,
                            }],
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
        self.pad_word_align(span);
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
                            vec![Fixup {
                                kind: FixupKind::Abs16Be,
                                offset: 0,
                                target: qe,
                            }],
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
        self.pad_word_align(span);
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
                            vec![Fixup {
                                kind: FixupKind::Abs32Be,
                                offset: 0,
                                target: qe,
                            }],
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
    /// `movem` routes to [`Self::lower_m68k_movem`] (register-list operand);
    /// every other in-scope mnemonic (incl. `movep`) flows through the shared
    /// branch/dbcc/jmp-jsr/generic paths below.
    fn lower_m68k(&mut self, mn: &str, rest: &[Token], span: Span) {
        // Expand AS `function` calls in the operands FIRST (e.g. the immediate
        // `#vram_art(tile,0,0)` / `#vdpComm(addr,VRAM,DMA)` / `#dmaLength(N)`
        // forms — `macros.asm`'s single-expression functions). `expand_calls`
        // only rewrites `name(args)` where `name` is a known function, so
        // register-indirect EAs (`(a0)`, `(4,a0,d1.w)`) pass through untouched.
        let expanded = self.expand_calls(rest, 0);
        let rest = expanded.as_slice();
        let (base, suffix_size) = split_mnemonic_and_size(mn);
        let mnemonic = match m68k_mnemonic(base) {
            Some(m) => m,
            None => {
                match m68k_out_of_scope(base) {
                    Some(family) => {
                        self.err(span, format!("`{base}` ({family}) is not yet implemented"))
                    }
                    None => self.err(span, format!("`{base}` is not a recognized 68000 mnemonic")),
                }
                return;
            }
        };

        // Every 68k instruction is word-aligned: under `padding on` at an odd `$`,
        // asl prefixes a $00 pad byte (asl-verified — `instr_odd_pad_on` probe).
        // Covers all instruction paths (branch/dbcc/movem/jmp-jsr/generic) since
        // it runs before the dispatch below.
        self.pad_word_align(span);

        if matches!(
            mnemonic,
            M68kMnemonic::Bra | M68kMnemonic::Bsr | M68kMnemonic::Bcc(_)
        ) {
            return self.lower_m68k_branch(mnemonic, suffix_size, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Dbcc(_)) {
            return self.lower_m68k_dbcc(mnemonic, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Movem) {
            return self.lower_m68k_movem(suffix_size, rest, span);
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
    fn lower_m68k_generic(
        &mut self,
        mnemonic: M68kMnemonic,
        suffix_size: Option<M68kSize>,
        atoms: Vec<OperandAtom>,
        span: Span,
    ) {
        let size = match suffix_size.or_else(|| m68k_default_size(mnemonic)) {
            Some(s) => s,
            None => {
                self.err(
                    span,
                    "instruction needs an explicit size suffix (.b/.w/.l)".to_string(),
                );
                return;
            }
        };
        if let Some(pc_idx) = atoms
            .iter()
            .position(|a| matches!(a, OperandAtom::M68kDisp { an, .. } if an == "pc"))
        {
            return self.lower_m68k_pcrel(mnemonic, size, &atoms, pc_idx, span);
        }
        if let Some(pc_idx) = atoms
            .iter()
            .position(|a| matches!(a, OperandAtom::M68kIdx { an, .. } if an == "pc"))
        {
            return self.lower_m68k_pcrel_idx(mnemonic, size, &atoms, pc_idx, span);
        }
        let ops = match self.convert_atoms_m68k(mnemonic, size, &atoms, span) {
            Some(o) => o,
            None => return,
        };
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// `bra`/`bsr`/`Bcc <target>`: Aeon pins the branch width by an explicit
    /// `.s`/`.w` suffix (no relaxation), so `suffix_size` MUST be present and
    /// MUST be `S` or `W`. The target is qualified (`.local` → `Scope.local`)
    /// and `$`-resolved, then handed to the backend's `lower_branch`, which
    /// builds the opcode + a `PcRel8`/`PcRelDisp16` fixup for the linker.
    fn lower_m68k_branch(
        &mut self,
        mnemonic: M68kMnemonic,
        suffix_size: Option<M68kSize>,
        rest: &[Token],
        span: Span,
    ) {
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
                self.err(
                    span,
                    format!("`{dn_name}` is not a valid data register in Dbcc"),
                );
                return;
            }
        };
        let target = self.resolve_dollar(&self.qualify_expr(&target_expr));
        let pc_of_disp_word = Expr::Int((self.here() + 2) as i64);
        let disp_expr = Expr::Binary {
            op: BinOp::Sub,
            lhs: Box::new(target),
            rhs: Box::new(pc_of_disp_word),
        };
        let d = self.fold_imm(&disp_expr, span, i16::MIN as i64, i16::MAX as i64);
        let inst = M68kInstruction {
            mnemonic,
            size: M68kSize::W,
            ops: vec![M68kOperand::Dn(dn), M68kOperand::Disp(d as i32)],
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// `movem.<w|l> <reglist>,<ea>` (STORE) / `movem.<w|l> <ea>,<reglist>`
    /// (LOAD). Exactly one operand is a register list (`d0-d7/a0-a6`, `a2/d2`,
    /// `d0-d3`, a single reg, or a range crossing the d→a boundary like
    /// `d0-a4`); the other is the memory EA. The register list is built into a
    /// CANONICAL mask (bit0=D0..bit7=D7, bit8=A0..bit15=A7) here, in operand
    /// order; the `-(An)` predecrement 16-bit mask REVERSAL is the encoder's
    /// job (`encode_movem`), never the front-end's — asl-verified: for
    /// `movem.l a2/d2,-(sp)` the front-end emits the canonical `RegList(0x0404)`
    /// and asl's bytes are `48 E7 20 20` (= `reverse_bits(0x0404)`), so the
    /// reversal must NOT be pre-applied here. Size is mandatory (`.w`/`.l`).
    fn lower_m68k_movem(&mut self, suffix_size: Option<M68kSize>, rest: &[Token], span: Span) {
        let size = match suffix_size {
            Some(s @ (M68kSize::W | M68kSize::L)) => s,
            Some(_) => {
                self.err(span, "movem is word (.w) or long (.l) only");
                return;
            }
            None => {
                self.err(span, "movem needs an explicit size suffix (.w or .l)");
                return;
            }
        };
        let groups = split_top_commas(rest);
        if groups.len() != 2 {
            self.err(
                span,
                "movem needs two operands: a register list and a memory EA",
            );
            return;
        }
        let list0 = parse_reg_list(groups[0]);
        let list1 = parse_reg_list(groups[1]);
        // The register list is whichever operand parses as one; the OTHER is the
        // memory EA. Operand ORDER selects direction (STORE vs LOAD), so it is
        // preserved. If both or neither parse, the form is malformed.
        let (mask, list_first, mem_toks) = match (list0, list1) {
            (Some(m), None) => (m, true, groups[1]),
            (None, Some(m)) => (m, false, groups[0]),
            (Some(_), Some(_)) => {
                self.err(
                    span,
                    "movem needs a memory EA operand, got two register lists",
                );
                return;
            }
            (None, None) => {
                self.err(
                    span,
                    "movem needs a register-list operand (e.g. `d0-d7/a0-a6`)",
                );
                return;
            }
        };
        let mem_atoms = match parse_operands(mem_toks) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let mem_atom = match mem_atoms.as_slice() {
            [a] => a,
            _ => {
                self.err(span, "movem memory operand must be a single EA");
                return;
            }
        };
        let mem_op = match self.convert_one_atom_m68k(mem_atom, size, span) {
            Some(o) => o,
            None => return,
        };
        let ops = if list_first {
            vec![M68kOperand::RegList(mask), mem_op]
        } else {
            vec![mem_op, M68kOperand::RegList(mask)]
        };
        let inst = M68kInstruction {
            mnemonic: M68kMnemonic::Movem,
            size,
            ops,
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
    fn lower_m68k_pcrel(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        pc_idx: usize,
        span: Span,
    ) {
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
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_pcrel_ea(&inst, 2, target, span);
        self.emit_frag(frag, span);
    }

    /// An instruction with a `(d8,PC,Xn)` source operand (`Label(pc,Xn.w|.l)`,
    /// e.g. jump-table reads `.case_table(pc,d2.w)`). Mirrors
    /// [`Self::lower_m68k_pcrel`] but for the brief-extension-word indexed form:
    /// the pc-idx atom's `disp` is the label target (resolved later as an 8-bit
    /// PC-relative displacement), and its index register becomes the ext word's
    /// `Xn`. The disp8 byte sits at offset 3 (opcode word + ext-word high byte).
    fn lower_m68k_pcrel_idx(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        pc_idx: usize,
        span: Span,
    ) {
        let mut ops = Vec::with_capacity(atoms.len());
        let mut target = None;
        for (i, a) in atoms.iter().enumerate() {
            if i == pc_idx {
                let (disp, xn, xlong) = match a {
                    OperandAtom::M68kIdx {
                        disp, xn, xlong, ..
                    } => (disp, xn, *xlong),
                    _ => unreachable!("pc_idx must index a M68kIdx{{an: \"pc\"}} atom"),
                };
                let xn = match self.m68k_index_reg(xn, span) {
                    Some(x) => x,
                    None => return,
                };
                target = Some(self.qualify_expr(disp));
                ops.push(M68kOperand::Pcd8Xn {
                    d: 0,
                    xn,
                    long: xlong,
                });
            } else {
                match self.convert_one_atom_m68k(a, size, span) {
                    Some(op) => ops.push(op),
                    None => return,
                }
            }
        }
        let target = target.expect("pc_idx must index the pc-relative atom");
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_pcrel_idx_ea(&inst, 3, target, span);
        self.emit_frag(frag, span);
    }

    /// Parse a 68k index-register name (`d0`..`d7` / `a0`..`a7`, `sp` = a7) into
    /// the ISA's `Xn`. Diagnoses (and returns `None`) on a non-register token.
    fn m68k_index_reg(&mut self, xn: &str, span: Span) -> Option<M68kXn> {
        if let Some(n) = m68k_data_reg(xn) {
            Some(M68kXn::D(n))
        } else if let Some(n) = m68k_addr_reg(xn) {
            Some(M68kXn::A(n))
        } else {
            self.err(span, format!("`{xn}` is not a valid index register"));
            None
        }
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

    /// Lower a bare (unsuffixed) absolute-address EA operand — a symbol or an
    /// expression used where a 68k EA is expected, e.g. `lea Sym, a0` or
    /// `move.w Sym, d0`. asl width-selects abs.w/abs.l via `asl_width_rule`
    /// (probe-verified EA-general in M1.D T2). We fold + select in the front
    /// end (the T3 width-selection mechanism for the absolute-EA class), so the
    /// instruction's Data fragment carries the true encoded length and the
    /// multi-pass fixpoint converges. Uses `self.fold` (not `fold_imm`): an
    /// unresolved-this-pass symbol folds to Poison → pessimistic abs.l (matching
    /// asl's forward-symbol width guess). The first resolution can then only
    /// shrink-or-stay (abs.l → abs.w/abs.l), so realistic forward refs converge.
    /// (`asl_width_rule` is non-monotonic at the $FF8000 sign-extension wrap —
    /// see the grow-only caveat in `sigil-link/relax.rs`; that region is
    /// immediately-resolved high-RAM constants in Aeon, and `PASS_CAP` backstops
    /// any pathological oscillation.) The name is recorded in `poison_refs` so a
    /// genuinely-undefined symbol still errors on the converged pass.
    fn abs_ea_from_expr(&mut self, e: &Expr, span: Span) -> M68kOperand {
        let qualified = self.qualify_expr(e);
        match self.fold(&qualified) {
            Fold::Value(v) => match asl_width_rule(v, false) {
                AbsWidth::W => M68kOperand::AbsW((v & 0xFFFF) as i16),
                AbsWidth::L => M68kOperand::AbsL(v as i32),
            },
            Fold::Poison => {
                for name in self.unresolved_names(&qualified) {
                    self.poison_refs.push((name, span));
                }
                // Pessimistic abs.l while unresolved; the converged pass re-folds
                // to a real value (or errors via poison_refs above).
                M68kOperand::AbsL(0)
            }
        }
    }

    /// Convert one operand atom (see [`Self::convert_atoms_m68k`]).
    fn convert_one_atom_m68k(
        &mut self,
        a: &OperandAtom,
        size: M68kSize,
        span: Span,
    ) -> Option<M68kOperand> {
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
                    self.err(
                        span,
                        format!("`{w}` is not a valid 68k register in this context"),
                    );
                    return None;
                }
            }
            OperandAtom::Value(e @ Expr::Sym(name)) => {
                if let Some(n) = m68k_data_reg(name) {
                    M68kOperand::Dn(n)
                } else if let Some(n) = m68k_addr_reg(name) {
                    M68kOperand::An(n)
                } else if name == "sr" {
                    M68kOperand::Sr
                } else if name == "ccr" {
                    M68kOperand::Ccr
                } else {
                    // Bare symbol in EA position = absolute address; asl
                    // width-selects abs.w/abs.l (M1.D T2).
                    self.abs_ea_from_expr(e, span)
                }
            }
            OperandAtom::Value(e) => {
                // Bare numeric/expression operand = 68k absolute addressing;
                // width-selected like the bare-symbol case above (M1.D T2).
                self.abs_ea_from_expr(e, span)
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
                self.err(
                    span,
                    format!("`({w})` is not a valid 68k address-register indirect operand"),
                );
                return None;
            }
            OperandAtom::Indexed { .. } => {
                self.err(
                    span,
                    "z80 `(ix±d)`/`(iy±d)` indexed form is not a valid 68k operand",
                );
                return None;
            }
            OperandAtom::M68kPreDec(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::PreDec(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `-(An)`"),
                    );
                    return None;
                }
            },
            OperandAtom::M68kPostInc(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::PostInc(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `(An)+`"),
                    );
                    return None;
                }
            },
            OperandAtom::M68kInd(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::Ind(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `(An)`"),
                    );
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
            OperandAtom::M68kIdx {
                disp,
                an,
                xn,
                xlong,
            } => {
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
                    self.err(
                        span,
                        format!("`{xn}` is not a valid index register in `(d,An,Xn)`"),
                    );
                    return None;
                };
                let d = self.fold_imm(disp, span, i8::MIN as i64, i8::MAX as i64);
                M68kOperand::Disp8AnXn {
                    d: d as i8,
                    an: an_n,
                    xn: xn_op,
                    long: *xlong,
                }
            }
            OperandAtom::AfShadow => {
                self.err(span, "`af'` is not a 68k operand");
                return None;
            }
        })
    }

    fn build_operands(
        &mut self,
        m: Mnemonic,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Lowered> {
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
                        Fold::Value(v) => {
                            Lowered::Fixed(vec![Operand::Pair(rr), Operand::Imm16(v as u16)])
                        }
                        Fold::Poison => {
                            Lowered::Abs16(vec![Operand::Pair(rr), Operand::Imm16(0)], target)
                        }
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
            Expr::Unary { op, operand } => Expr::Unary {
                op: *op,
                operand: Box::new(self.resolve_dollar(operand)),
            },
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
    fn convert_atoms(
        &mut self,
        m: Mnemonic,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Vec<Operand>> {
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
                    Operand::Indexed {
                        reg: *reg,
                        disp: d as i8,
                    }
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
        let toks = lex_line(
            &lines[start].text,
            self.state.cpu,
            self.source,
            lines[start].base,
        )
        .unwrap_or_default();
        // Two head shapes (both real AS, both asl-verified):
        //   `NAME macro p...`   → toks: Ident(NAME) Ident("macro") [params...]
        //   `NAME: macro p...`  → toks: Ident(NAME) Colon Ident("macro") [params...]
        // The colon form (used by the `__FSTRING`/`__ErrorMessage` debug macros)
        // must peel the label before reading params, else `macro` itself leaks in
        // as the first "param" and shifts every real param by one (binding the
        // caller's arg to a phantom slot). `parse_line_tokens` peels it.
        let parsed = parse_line_tokens(&toks);
        let (name, param_toks): (String, Vec<Token>) = if let Some(lbl) = parsed.label_colon {
            // parsed.tokens: Ident("macro") [params...]; params start at index 1.
            (lbl, parsed.tokens.get(1..).unwrap_or(&[]).to_vec())
        } else {
            let name = match toks.first().map(|t| &t.tok) {
                Some(Tok::Ident(s)) => s.clone(),
                _ => {
                    let span = Span {
                        source: self.source,
                        start: lines[start].base,
                        end: lines[start].base,
                    };
                    self.err(span, "macro needs a name");
                    String::new()
                }
            };
            (name, toks.get(2..).unwrap_or(&[]).to_vec())
        };
        let params: Vec<String> = param_toks
            .iter()
            .filter_map(|t| {
                if let Tok::Ident(p) = &t.tok {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        let end = self.find_block_end(lines, start);
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
        self.expand_macro_inner(name, arg_toks, None);
    }

    /// Expand a `.ATTRIBUTE`-suffix invocation (T9.2): `name` is the BASE
    /// macro (already stripped of its `.SUFFIX` by `dispatch`'s
    /// `split_attribute_suffix` check), `attribute` is the literal suffix
    /// text (`.b`/`.w`/`.l`/`.s`) bound to `.ATTRIBUTE` inside the body.
    fn expand_macro_with_attribute(&mut self, name: &str, arg_toks: &[Token], attribute: &str) {
        self.expand_macro_inner(name, arg_toks, Some(attribute));
    }

    /// Shared implementation: substitute `.ATTRIBUTE` (if this is an
    /// attribute-suffixed call), `ALLARGS` (verbatim arg text), and params
    /// (positional and/or keyword), then execute the resulting lines.
    ///
    /// Real AS binds params two ways, mixable in one call (asl-verified — see
    /// the `macro_keyword_args` snippet): a comma-split arg of the shape
    /// `NAME=value` binds `NAME` by name, regardless of where it sits in the
    /// call; every other arg fills the remaining (not yet keyword-bound)
    /// params positionally, in declaration order. `tst AMP=7,PER=9`,
    /// `tst 3,4`, and `tst PER=5,AMP=2` (params `AMP,PER`) all bind correctly
    /// under this rule.
    ///
    /// `.ATTRIBUTE` is substituted with a plain (unbounded) literal-text
    /// replace, the same way `ALLARGS` is — NOT `replace_word`'s
    /// identifier-boundary match, because `.ATTRIBUTE` is deliberately used
    /// glued onto a mnemonic (`move.ATTRIBUTE`, one lexed ident) as well as
    /// standalone in a string; a boundary check keyed on `is_alphanumeric`
    /// would reject the glued-mnemonic case (the char right before the `.` is
    /// alphanumeric, e.g. the `e` in `move`), which is the primary asl-verified
    /// use (asl-verified: `move.ATTRIBUTE src,d0` with `foo.w d1` → `move.w d1,d0`).
    fn expand_macro_inner(&mut self, name: &str, arg_toks: &[Token], attribute: Option<&str>) {
        if self.macro_depth >= EXPAND_CAP {
            let span = arg_toks.first().map(|t| t.span).unwrap_or(Span {
                source: self.source,
                start: 0,
                end: 0,
            });
            self.err(
                span,
                format!("macro `{name}` expansion too deep (recursive macro?)"),
            );
            return;
        }
        let (params, body) = match self.macros.get(name) {
            Some(m) => m.clone(),
            None => return,
        };
        let all_args = render_tokens(arg_toks);
        let groups = split_top_commas(arg_toks);
        let mut keyword: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut positional: Vec<String> = Vec::new();
        for g in &groups {
            if let [Token {
                tok: Tok::Ident(nm),
                ..
            }, Token {
                tok: Tok::Punct(Punct::Eq),
                ..
            }, value @ ..] = *g
            {
                if !value.is_empty() && params.iter().any(|p| p == nm) {
                    keyword.insert(nm.clone(), render_tokens(value));
                    continue;
                }
            }
            positional.push(render_tokens(g));
        }
        let mut pos_iter = positional.into_iter();
        // An OMITTED argument binds to the EMPTY STRING, not "left unsubstituted"
        // (asl-verified): the Aeon parallax macros gate optional fields on
        // `if "param" = ""` and expect the bare param to vanish where used
        // (`P_VFG := vFactorFg` → `P_VFG := ` on the empty branch, never taken).
        // `replace_word` treats `"` as a word boundary, so an empty binding also
        // collapses `"param"` → `""`, making the guard compare true.
        let arg_values: Vec<(String, String)> = params
            .iter()
            .map(|p| {
                let v = keyword
                    .get(p)
                    .cloned()
                    .or_else(|| pos_iter.next())
                    .unwrap_or_default();
                (p.clone(), v)
            })
            .collect();
        let mut expanded = Vec::new();
        for l in &body {
            let mut text = l.text.clone();
            if let Some(suffix) = attribute {
                text = text.replace(".ATTRIBUTE", suffix);
            }
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
    // A physical line whose last non-whitespace character is `\` is an AS
    // line-continuation: it joins with the following physical line into one
    // logical line (the Aeon `function` definitions in macros.asm /
    // parallax_macros.inc wrap a long body expression this way). The joined
    // logical line takes the FIRST physical line's `base`; the `\` (and the
    // intervening newline) are replaced by spaces so downstream byte offsets
    // stay length-stable and no bogus `\` token reaches the lexer. Only a
    // trailing `\` continues — an interior `\` (e.g. a macro `\1` parameter
    // marker) is untouched.
    let mut pending: Option<(u32, String)> = None;
    for raw in text.split_inclusive('\n') {
        let trimmed = raw.trim_end();
        let is_cont = trimmed.ends_with('\\');
        // Length-preserving cell text: drop the trailing newline's semantics by
        // turning a continuation `\`+tail into spaces, else keep the raw text.
        let cell = if is_cont {
            // Replace the final `\` with a space, and the trailing whitespace
            // (incl. the newline) it had is preserved as-is after it.
            let cut = trimmed.len() - 1; // index of the `\`
            let mut s = String::with_capacity(raw.len());
            s.push_str(&raw[..cut]);
            s.push(' ');
            s.push_str(&raw[cut + 1..]);
            s
        } else {
            raw.to_string()
        };
        match pending.take() {
            Some((start_base, mut acc)) => {
                acc.push_str(&cell);
                if is_cont {
                    pending = Some((start_base, acc));
                } else {
                    lines.push(SrcLine {
                        text: acc,
                        base: start_base,
                    });
                }
            }
            None => {
                if is_cont {
                    pending = Some((base, cell));
                } else {
                    lines.push(SrcLine { text: cell, base });
                }
            }
        }
        base += raw.len() as u32;
    }
    if let Some((start_base, acc)) = pending {
        lines.push(SrcLine {
            text: acc,
            base: start_base,
        });
    }
    lines
}

fn is_op_keyword(s: &str) -> bool {
    matches!(
        s,
        "cpu"
            | "phase"
            | "dephase"
            | "org"
            | "save"
            | "restore"
            | "padding"
            | "supmode"
            | "db"
            | "dw"
            | "dc.b"
            | "dc.w"
            | "dc.l"
            | "equ"
            | "if"
            | "elseif"
            | "else"
            | "endif"
            | "ifdef"
            | "ifndef"
            | "rept"
            | "endr"
            | "endm"
            | "macro"
            | "struct"
            | "endstruct"
            | "function"
            | "include"
            | "BINCLUDE"
            | "error"
            | "fatal"
            | "message"
            | "ds.b"
            | "ds.w"
            | "ds.l"
            | "align"
            | "while"
            | "switch"
            | "case"
            | "elsecase"
            | "endcase"
    )
}

/// The closer keyword(s) that terminate the block a given OPENER keyword
/// starts, or `&[]` if `s` does not open a block at all (used by
/// [`Asm::find_block_end`]'s nesting stack — see its doc for why this must be
/// keyed per-opener rather than a single flat set: `while`/`macro` (and
/// optionally `rept`) all share the literal `endm` closer in real AS).
fn closers_for(s: &str) -> &'static [&'static str] {
    match s {
        "if" | "ifdef" | "ifndef" => &["endif"],
        "rept" => &["endr", "endm"],
        "while" => &["endm"],
        "macro" => &["endm"],
        "struct" => &["endstruct"],
        "switch" => &["endcase"],
        _ => &[],
    }
}

/// Split a bare identifier on a trailing `.b`/`.w`/`.l`/`.s` size suffix,
/// returning the base name and the literal suffix text (e.g. `.w`). Used for
/// `.ATTRIBUTE` macro-suffix synthesis (T9.2): a macro invoked as
/// `name.SUFFIX args` is dispatched by stripping the suffix and checking
/// whether the BASE name is a captured macro — deliberately distinct from
/// `split_mnemonic_and_size` (which returns a parsed `M68kSize` for real
/// mnemonic lowering) so the two never interact: this only ever fires from
/// `dispatch`'s attribute-macro check, which is gated on the base name being
/// a literal entry in `self.macros` — a real mnemonic like `move`/`clr` is
/// never in that map, so `move.w`/`clr.b` etc. keep going through the normal
/// mnemonic-suffix path untouched.
fn split_attribute_suffix(s: &str) -> Option<(&str, &'static str)> {
    if let Some(b) = s.strip_suffix(".b") {
        Some((b, ".b"))
    } else if let Some(b) = s.strip_suffix(".w") {
        Some((b, ".w"))
    } else if let Some(b) = s.strip_suffix(".l") {
        Some((b, ".l"))
    } else if let Some(b) = s.strip_suffix(".s") {
        Some((b, ".s"))
    } else {
        None
    }
}

fn is_mnemonic(s: &str) -> bool {
    mnemonic(s).is_some()
}

fn mnemonic(s: &str) -> Option<Mnemonic> {
    use Mnemonic::*;
    Some(match s {
        "nop" => Nop,
        "ld" => Ld,
        "add" => Add,
        "adc" => Adc,
        "sub" => Sub,
        "sbc" => Sbc,
        "and" => And,
        "or" => Or,
        "xor" => Xor,
        "cp" => Cp,
        "inc" => Inc,
        "dec" => Dec,
        "push" => Push,
        "pop" => Pop,
        "ex" => Ex,
        "exx" => Exx,
        "ret" => Ret,
        "jr" => Jr,
        "jp" => Jp,
        "call" => Call,
        "djnz" => Djnz,
        "rrca" => Rrca,
        "scf" => Scf,
        "ei" => Ei,
        "di" => Di,
        "bit" => Bit,
        "res" => Res,
        "set" => Set,
        "srl" => Srl,
        "rr" => Rr,
        "sla" => Sla,
        "rlc" => Rlc,
        "rrc" => Rrc,
        "rl" => Rl,
        "sra" => Sra,
        "neg" => Neg,
        "im" => Im,
        "ldir" => Ldir,
        _ => return None,
    })
}

fn cond_word(w: &str) -> Option<Cond> {
    use Cond::*;
    Some(match w {
        "nz" => Nz,
        "z" => Z,
        "nc" => Nc,
        "c" => C,
        "po" => Po,
        "pe" => Pe,
        "p" => P,
        "m" => M,
        _ => return None,
    })
}

fn reg8(w: &str) -> Option<Reg8> {
    use Reg8::*;
    Some(match w {
        "a" => A,
        "b" => B,
        "c" => C,
        "d" => D,
        "e" => E,
        "h" => H,
        "l" => L,
        _ => return None,
    })
}

fn reg16(w: &str) -> Option<Reg16> {
    use Reg16::*;
    Some(match w {
        "bc" => Bc,
        "de" => De,
        "hl" => Hl,
        "sp" => Sp,
        "af" => Af,
        "ix" => Ix,
        "iy" => Iy,
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
/// shape (a bare `sr`/`ccr`) is known. `movem`/`movep` (register-list operands)
/// are now in scope too; nothing 68000 the Aeon source uses remains deferred.
fn m68k_mnemonic(base: &str) -> Option<M68kMnemonic> {
    use M68kMnemonic::*;
    Some(match base {
        "move" => Move,
        "movea" => Movea,
        "add" => Add,
        "adda" => Adda,
        "sub" => Sub,
        "suba" => Suba,
        "and" => And,
        "or" => Or,
        "eor" => Eor,
        "cmp" => Cmp,
        "cmpa" => Cmpa,
        "muls" => Muls,
        "addi" => Addi,
        "subi" => Subi,
        "andi" => Andi,
        "ori" => Ori,
        "eori" => Eori,
        "cmpi" => Cmpi,
        "moveq" => Moveq,
        "addq" => Addq,
        "subq" => Subq,
        "asl" => Asl,
        "asr" => Asr,
        "lsl" => Lsl,
        "lsr" => Lsr,
        "rol" => Rol,
        "ror" => Ror,
        "btst" => Btst,
        "bset" => Bset,
        "bclr" => Bclr,
        "clr" => Clr,
        "neg" => Neg,
        "not" => Not,
        "tst" => Tst,
        "tas" => Tas,
        "swap" => Swap,
        "ext" => Ext,
        "lea" => Lea,
        "pea" => Pea,
        "movem" => Movem,
        "movep" => Movep,
        "addx" => Addx,
        "cmpm" => Cmpm,
        "nop" => Nop,
        "rts" => Rts,
        "rte" => Rte,
        "trap" => Trap,
        "bra" => Bra,
        "bsr" => Bsr,
        "jmp" => Jmp,
        "jsr" => Jsr,
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
/// `db<cc>`, `s<cc>`) into its `Cond`. All 16 codes per the ISA's `Cond` enum,
/// plus the two unsigned-branch spellings `hs`/`lo`: on the 68000 HS
/// (higher-or-same) IS carry-clear (CC) and LO (lower) IS carry-set (CS) —
/// asl accepts `bhs`/`blo`/`shs`/`slo`/`dbhs`/`dblo` as exact aliases, and the
/// Aeon source uses `bhs`/`blo` pervasively (~68 sites). They encode to the
/// identical opcode as `bcc`/`bcs`, so this is a pure spelling alias.
fn m68k_cond(w: &str) -> Option<M68kCond> {
    use M68kCond::*;
    Some(match w {
        "t" => T,
        "f" => F,
        "hi" => Hi,
        "ls" => Ls,
        "cc" => Cc,
        "cs" => Cs,
        "hs" => Cc,
        "lo" => Cs,
        "ne" => Ne,
        "eq" => Eq,
        "vc" => Vc,
        "vs" => Vs,
        "pl" => Pl,
        "mi" => Mi,
        "ge" => Ge,
        "lt" => Lt,
        "gt" => Gt,
        "le" => Le,
        _ => return None,
    })
}

/// If `base` names a real 68000 mnemonic that this front-end deliberately does
/// not implement yet, name the family for the diagnostic; else `None`
/// (genuinely unrecognized). Nothing remains out of scope — `movem`/`movep`
/// (with register-list operands) are now handled — so this always returns
/// `None`; it is retained as the seam where a future deferral would name its
/// family.
fn m68k_out_of_scope(_base: &str) -> Option<&'static str> {
    None
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
        // Bit ops (`btst`/`bset`/`bclr`) carry NO suffix in real 68k syntax:
        // the operation size is implicit in the destination (long for a `Dn`
        // target, byte for a memory target) and the encoder (`encode_bit`)
        // re-derives it from the operand, ignoring this field — so the value
        // here only satisfies `Instruction`'s size slot. `B` keeps the source
        // `#bit`/`Dn` immediate fold within byte bounds (bit numbers are ≤ 31).
        Btst | Bset | Bclr => Some(M68kSize::B),
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
        "`(d8,PC,Xn)` indexed PC-relative addressing is not yet supported (only `(d16,PC)` lowers)"
            .to_string()
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

/// The MOVEM register-list bit index of a single register: `d0..d7` → `0..=7`,
/// `a0..a7` (and `sp` = `a7`) → `8..=15`. This is the CANONICAL mask ordering
/// the encoder expects (`Operand::RegList` doc); the `-(An)` reversal is applied
/// inside `encode_movem`, never here. `None` for any non-register word.
fn reg_list_index(w: &str) -> Option<u8> {
    if let Some(n) = m68k_data_reg(w) {
        Some(n)
    } else {
        m68k_addr_reg(w).map(|n| n + 8)
    }
}

/// Parse a MOVEM register-list operand's tokens into a canonical 16-bit mask
/// (bit0=D0..bit7=D7, bit8=A0..bit15=A7), or `None` if the tokens are not a
/// well-formed register list. Grammar: `/`-separated items, each a single
/// register (`d3`, `a2`) or a contiguous range `lo-hi` (`d0-d7`, `a0-a6`, or
/// a d→a crossing range such as `d0-a4`). A range with `lo > hi` is rejected.
/// This is a total, side-effect-free recognizer: it returns `None` (rather than
/// diagnosing) on any non-list shape so the caller can use it to DISCRIMINATE
/// the register-list operand from the memory-EA operand of a `movem`.
fn parse_reg_list(toks: &[Token]) -> Option<u16> {
    if toks.is_empty() {
        return None;
    }
    let mut mask: u16 = 0;
    for item in split_slash(toks) {
        match item {
            // Single register: `d3`, `a2`, `sp`.
            [Token {
                tok: Tok::Ident(w), ..
            }] => {
                mask |= 1u16 << reg_list_index(w)?;
            }
            // Contiguous range: `d0-d7`, `a0-a6`, `d0-a4`.
            [Token {
                tok: Tok::Ident(lo),
                ..
            }, Token {
                tok: Tok::Punct(Punct::Minus),
                ..
            }, Token {
                tok: Tok::Ident(hi),
                ..
            }] => {
                let lo = reg_list_index(lo)?;
                let hi = reg_list_index(hi)?;
                if lo > hi {
                    return None;
                }
                for b in lo..=hi {
                    mask |= 1u16 << b;
                }
            }
            _ => return None,
        }
    }
    Some(mask)
}

/// Split a register-list operand's tokens on top-level `/` separators (a
/// register list never contains parentheses, so no depth tracking is needed).
fn split_slash(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if matches!(t.tok, Tok::Punct(Punct::Slash)) {
            groups.push(&toks[start..i]);
            start = i + 1;
        }
    }
    groups.push(&toks[start..]);
    groups
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
        // An immediate source into a MEMORY destination on the ALU forms is
        // asl's spelling of the corresponding `xxxi` immediate instruction:
        // `cmp #imm,(abs)` ≡ `cmpi`, `and #imm,(abs)` ≡ `andi`, etc. (byte-exact
        // asl-verified: `cmp.b #$80,($FFFF8000).l` == `cmpi.b …` == `0C39 …`).
        // A `Dn` destination is left alone — `add #4,d0` / `cmp #5,d0` are the
        // regular `<ea>,Dn` forms with an immediate source EA (distinct bytes).
        (Cmp, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Cmpi,
        (And, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Andi,
        (Or, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Ori,
        (Add, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Addi,
        (Sub, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Subi,
        (Eor, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Eori,
        (m, _) => m,
    }
}

/// True for a 68k MEMORY effective-address destination (any alterable EA that is
/// neither a data nor address register). Used to route `cmp`/`and`/… `#imm,mem`
/// to their `cmpi`/`andi`/… immediate encodings (see `refine_m68k_mnemonic`).
fn is_mem_dest(op: &M68kOperand) -> bool {
    use M68kOperand::*;
    matches!(
        op,
        Ind(_) | PostInc(_) | PreDec(_) | Disp16An(..) | Disp8AnXn { .. } | AbsW(_) | AbsL(_)
    )
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
    Token {
        tok: Tok::Punct(p),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::Options;
    use sigil_ir::backend::Cpu;
    use sigil_ir::Module;

    fn image(src: &str) -> Vec<u8> {
        let m = run(src, &Options::default()).expect("assemble");
        m.sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default()
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
        assert_eq!(
            m68k_mnemonic("beq"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("bne"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Ne))
        );
        assert_eq!(
            m68k_mnemonic("dbf"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F))
        );
        assert_eq!(
            m68k_mnemonic("dbra"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F))
        );
        assert_eq!(
            m68k_mnemonic("dbeq"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("scc"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("seq"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("st"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::T))
        );
        // Unsigned-branch aliases: bhs == bcc (carry-clear), blo == bcs
        // (carry-set); same for shs/slo and dbhs/dblo. Aeon uses bhs/blo.
        assert_eq!(
            m68k_mnemonic("bhs"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("blo"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Cs))
        );
        assert_eq!(
            m68k_mnemonic("shs"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("slo"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cs))
        );
        assert_eq!(
            m68k_mnemonic("dbhs"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        // `movem`/`movep` are now in scope (register-list operands).
        assert_eq!(m68k_mnemonic("movem"), Some(Mnemonic::Movem));
        assert_eq!(m68k_mnemonic("movep"), Some(Mnemonic::Movep));
        // `cmpm` (F3): encoder always had it; the front-end table did not until
        // M1.D T0.4. Exposed only under __DEBUG__ (compression_selftest.asm:83).
        assert_eq!(m68k_mnemonic("cmpm"), Some(Mnemonic::Cmpm));
        // a genuinely unrecognized word is not misparsed as a stray cc suffix.
        assert_eq!(m68k_mnemonic("banana"), None);
    }

    #[test]
    fn m68k_cond_parses_all_16_condition_codes() {
        use super::m68k_cond;
        use sigil_backend_m68k::m68k::Cond;
        let pairs = [
            ("t", Cond::T),
            ("f", Cond::F),
            ("hi", Cond::Hi),
            ("ls", Cond::Ls),
            ("cc", Cond::Cc),
            ("cs", Cond::Cs),
            ("ne", Cond::Ne),
            ("eq", Cond::Eq),
            ("vc", Cond::Vc),
            ("vs", Cond::Vs),
            ("pl", Cond::Pl),
            ("mi", Cond::Mi),
            ("ge", Cond::Ge),
            ("lt", Cond::Lt),
            ("gt", Cond::Gt),
            ("le", Cond::Le),
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
    fn parse_reg_list_builds_canonical_masks() {
        use super::parse_reg_list;
        use crate::lexer::lex_line;
        let mask = |s: &str| {
            let toks = lex_line(s, Cpu::M68000, sigil_span::SourceId(0), 0).unwrap();
            parse_reg_list(&toks)
        };
        // Single reg: bit0=D0..bit7=D7, bit8=A0..bit15=A7 (canonical order).
        assert_eq!(mask("d0"), Some(0x0001));
        assert_eq!(mask("a2"), Some(0x0400));
        assert_eq!(mask("sp"), Some(0x8000)); // sp == a7 == bit15
                                              // `/` list mixing d and a.
        assert_eq!(mask("a2/d2"), Some(0x0404));
        // `-` range.
        assert_eq!(mask("d0-d3"), Some(0x000F));
        assert_eq!(mask("a0-a6"), Some(0x7F00));
        // Range crossing the d→a boundary is contiguous in canonical order.
        assert_eq!(mask("d0-a4"), Some(0x1FFF));
        // Range + list combined.
        assert_eq!(mask("d0-d7/a0-a6"), Some(0x7FFF));
        assert_eq!(mask("d0-d6/a2"), Some(0x047F));
        // Not a register list.
        assert_eq!(mask("(a0)"), None);
        assert_eq!(mask("-(sp)"), None);
        assert_eq!(mask("d0-x9"), None);
        assert_eq!(mask("d7-d0"), None); // reversed range rejected
    }

    #[test]
    fn m68k_movem_predec_reverses_mask_but_postinc_does_not() {
        // STORE to `-(An)` predecrement: the encoder REVERSES the canonical
        // mask. `a2/d2` canonical = 0x0404 → emitted word 0x2020 (48 E7 20 20).
        assert_eq!(
            image("    cpu 68000\n    movem.l a2/d2,-(sp)\n"),
            vec![0x48, 0xE7, 0x20, 0x20]
        );
        // LOAD from `(An)+` postincrement: canonical mask emitted as-is.
        // `d0-d7/a0-a6` canonical = 0x7FFF → 4C DF 7F FF.
        assert_eq!(
            image("    cpu 68000\n    movem.l (sp)+,d0-d7/a0-a6\n"),
            vec![0x4C, 0xDF, 0x7F, 0xFF]
        );
    }

    #[test]
    fn m68k_movem_single_range_and_mixed_lists() {
        // Single register store to predec: canonical 0x0400 → reversed 0x0020.
        assert_eq!(
            image("    cpu 68000\n    movem.l a2,-(sp)\n"),
            vec![0x48, 0xE7, 0x00, 0x20]
        );
        // Range store to plain (An) indirect: NOT reversed (0x0018).
        assert_eq!(
            image("    cpu 68000\n    movem.l d3-d4,(a3)\n"),
            vec![0x48, 0xD3, 0x00, 0x18]
        );
        // Word range store to predec a7: canonical 0x000F → reversed 0xF000.
        assert_eq!(
            image("    cpu 68000\n    movem.w d0-d3,-(a7)\n"),
            vec![0x48, 0xA7, 0xF0, 0x00]
        );
        // Mixed range+single load from postinc crossing d→a: 0x1FFF, not reversed.
        assert_eq!(
            image("    cpu 68000\n    movem.l (a0)+,d0-a4\n"),
            vec![0x4C, 0xD8, 0x1F, 0xFF]
        );
        // Disp16(An) memory EA store (extension word follows the mask word).
        assert_eq!(
            image("    cpu 68000\n    movem.l d3-d4,(8,a3)\n"),
            vec![0x48, 0xEB, 0x00, 0x18, 0x00, 0x08]
        );
    }

    #[test]
    fn m68k_movep_both_directions() {
        // reg → mem (word): 01 89 00 04.
        assert_eq!(
            image("    cpu 68000\n    movep.w d0,4(a1)\n"),
            vec![0x01, 0x89, 0x00, 0x04]
        );
        // mem → reg (long): 03 4A 00 08.
        assert_eq!(
            image("    cpu 68000\n    movep.l 8(a2),d1\n"),
            vec![0x03, 0x4A, 0x00, 0x08]
        );
    }

    #[test]
    fn m68k_register_indirect_operand_now_lowers_in_t5() {
        // `(a0)` is a register-indirect EA — T4 deferred it to T5; T5 (this
        // task) implements the fixed-length `(An)` family, so it now lowers
        // byte-exact instead of erroring. Bytes verified against real asl
        // (see `m68k_move_w_ind_a0_to_d0` in `tests/snippets_golden.txt`).
        assert_eq!(
            image("    cpu 68000\n    move.w (a0),d0\n"),
            vec![0x30, 0x10]
        );
    }

    #[test]
    fn m68k_pcrelative_disp16_lowers_via_resolve_layout_link() {
        // `(d16,PC)` (T5c): the front-end emits an unresolved `PcRelDisp16`
        // fixup (via `lower_pcrel_ea`); resolving it needs a real link (the
        // front-end's own fold never sees it — see `apply_fixup` in
        // `sigil-link`). `move.w (8,pc),d0` at VMA 0: the extension word sits
        // at offset 2, target = 8, disp = 8 - 2 = 6.
        let src = "    cpu 68000\n    phase 0\n    move.w (8,pc),d0\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // move.w (d16,PC),d0 = 30 3A, then disp word 00 06.
        assert_eq!(bytes, vec![0x30, 0x3A, 0x00, 0x06]);
    }

    #[test]
    fn m68k_pcrelative_disp8_indexed_lowers() {
        // `(d8,PC,Xn)` now lowers to a brief extension word + `PcRelDisp8` fixup.
        // asl-verified: `move.w (8,pc,d0.w),d1` at VMA 0 → `32 3B 00 06` (the
        // literal `8` is a TARGET address; disp = 8 - ext-word-VMA(2) = 6).
        let src = "    cpu 68000\n    phase 0\n    move.w (8,pc,d0.w),d1\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x32, 0x3B, 0x00, 0x06]);
    }

    #[test]
    fn m68k_bare_absolute_operand_width_selects_abs_w() {
        // A bare number (no `#`, no parens) means 68k absolute addressing. Since
        // M1.D T2 this is in scope: asl width-selects abs.w for a target in
        // [0,$7FFF]∪[$FF8000,$FFFFFF]. `$1234` ≤ $7FFF → abs.w: `30 38 12 34`.
        let src = "    cpu 68000\n    move.w $1234,d0\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x30, 0x38, 0x12, 0x34]);
    }

    #[test]
    fn m68k_branch_without_size_suffix_is_a_clear_diagnostic() {
        // T5c: `bra`/`Bcc` are now in scope, but Aeon pins branch width by an
        // explicit `.s`/`.w` suffix (no relaxation) — a bare `bra` must still
        // error, just with a size-suffix diagnostic instead of a scope one.
        let src = "    cpu 68000\n    bra Target\nTarget:\n    rts\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let diags = run(src, &opts)
            .expect_err("branch without a size suffix must be rejected, not lowered");
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
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
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
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        assert!(matches!(
            m.sections[0].fragments[0],
            sigil_ir::Fragment::JmpJsrSym { is_jsr: false, .. }
        ));
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
    }

    #[test]
    fn m68k_missing_size_suffix_is_a_clear_diagnostic() {
        // `move` has no default size and no suffix here — must error, not guess.
        let src = "    cpu 68000\n    move d0,d1\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
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
        let linked = sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new())
            .expect("link must succeed (no unresolvable fixup)");
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
            let linked =
                sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new()).expect("link");
            sigil_link::flatten(&linked, 0x00)
        };
        assert_eq!(
            link("        cpu z80\n        phase 0\n        jr $+2\n"),
            vec![0x18, 0x00]
        );
        assert_eq!(
            link("        cpu z80\n        phase 0\n        jr $-2\n"),
            vec![0x18, 0xFC]
        );
    }

    #[test]
    fn ifdef_gates_emission_by_define_set() {
        let src = "        cpu z80\n        phase 0\n        db 1\n        ifdef __DEBUG__\n        db 0FFh\n        endif\n        ifdef SOUND_DRIVER_ENABLED\n        db 2\n        endif\n";
        let opts = Options {
            initial_cpu: Cpu::Z80,
            defines: vec![("SOUND_DRIVER_ENABLED".into(), 1)],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
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
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
    }

    #[test]
    fn m68k_instruction_after_colon_label_lowers_the_mnemonic_not_the_operand() {
        // Before the column-rule fix, `move.w` was swallowed as a bogus label and
        // only `d0` reached dispatch. Routed correctly, the whole instruction
        // lowers: `move.w d0,d1` = 3200.
        let src = "    cpu 68000\nStart:\n    move.w d0,d1\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x32, 0x00]);
    }

    #[test]
    fn m68k_colon_label_then_instruction_both_handled() {
        // `Foo: rts` on one line: the colon label must be defined AND the
        // remaining head routed as an instruction (label_colon.is_some() clause),
        // even though line.text starts at column 0.
        let src = "    cpu 68000\nFoo: rts\n";
        let opts = Options {
            initial_cpu: Cpu::M68000,
            defines: vec![],
            include_root: None,
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
    }

    #[test]
    fn lowers_common_instructions() {
        let src = "        cpu z80\n        phase 0\n        nop\n        ld a,0Ch\n        ld b,c\n        add a,b\n        jp 1234h\n";
        assert_eq!(
            image(src),
            vec![0x00, 0x3E, 0x0C, 0x41, 0x80, 0xC3, 0x34, 0x12]
        );
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
        let src =
            "        cpu z80\n        phase 0\n        rept 3\n        db 0AAh\n        endr\n";
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
    fn equ_keyword_defines_a_constant() {
        // AS `name equ expr` (parallax_macros.inc `FACTOR_LOCKED equ $0FF`).
        // Also `dec equ $90`: `dec` is a Z80 mnemonic, so without the equate
        // intercept the line would route to instruction lowering.
        let src = "        cpu 68000\n        phase 0\nFOO equ $12\ndec equ $34\n        dc.b FOO\n        dc.b dec\n";
        assert_eq!(image(src), vec![0x12, 0x34]);
    }

    #[test]
    fn colon_label_equate_forms_define_constants_not_labels() {
        // AS tolerates a decorative colon on an equate: `NAME: equ v`
        // (debugger.asm) and `NAME: = v` (ram.asm `RESET_RAM: = $FFFFFF00`).
        let src = "        cpu 68000\n        phase 0\nA: equ $11\nB: = $22\n        dc.b A\n        dc.b B\n";
        assert_eq!(image(src), vec![0x11, 0x22]);
    }

    #[test]
    fn anonymous_struct_reserve_field_advances_offset() {
        // AS: an unnamed `ds.b N` inside a struct reserves space (advances the
        // running offset) but binds no member symbol — the Act struct's
        // `ds.b 1 ; reserved (pad to word)` pattern. Here b=$00, len=3 (a+pad+c
        // = 1+1+1) even though the middle field has no name.
        let src = concat!(
            "        cpu 68000\n        padding off\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "        ds.b 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x02, 0x03]);
    }

    #[test]
    fn char_constant_folds_in_expression() {
        // AS `'…'` packs big-endian; used bare and in expressions.
        let src = "        cpu 68000\n        phase 0\n        dc.l 'INIT'\n";
        assert_eq!(image(src), vec![0x49, 0x4E, 0x49, 0x54]);
    }

    #[test]
    fn binary_literal_folds_in_expression() {
        // AS `%` binary literal (constants.asm `VRAM = %100001`).
        let src = "        cpu 68000\n        phase 0\nVRAM = %100001\n        dc.b VRAM\n";
        assert_eq!(image(src), vec![0x21]);
    }

    #[test]
    fn backslash_line_continuation_joins_function_body() {
        // AS trailing-`\` continuation (macros.asm vdpComm def wraps its body).
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "sum     function a,b, \\\n",
            "                (a) + (b)\n",
            "        dc.b sum(3,4)\n",
        );
        assert_eq!(image(src), vec![0x07]);
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
        let src =
            "        cpu 68000\n        phase 0\n        dc.b 1\n        align 2\n        dc.b 2\n";
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
    fn org_backpatch_seeks_in_section_and_overwrites() {
        // The `parallax_section_end` shape: capture positions via `:=`/`*`
        // (M1.C T6b adds `*` as a PC-symbol atom alongside `$`), seek back to
        // patch a placeholder byte, then resume forward. asl-verified: 63 01 02 03 04.
        let src = "        cpu 68000\n        padding off\n        phase 0\nHdr := *\n        dc.b 0,1,2,3\nEnd := *\n        org Hdr\n        dc.b 99\n        org End\n        dc.b 4\n";
        assert_eq!(image(src), vec![0x63, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn org_forward_past_extent_opens_a_new_phase_like_section() {
        // A forward `org` beyond anything written closes the section and
        // re-phases (main.asm's `org $10000` shape, scaled down) rather than
        // growing the still-open section with a zero-fill run — proven here by
        // checking `module.sections.len()` directly (the byte-level gap-fill is
        // ALSO covered by the `org_forward_new_section` golden snippet, which
        // can't distinguish the two implementations since `flatten` produces
        // identical bytes either way).
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1,2,3,4\n        org 16\n        dc.b 5,6\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(
            module.sections.len(),
            2,
            "forward org must open a new section, not seek in-place"
        );
        assert_eq!(module.sections[0].vma_base, Some(0));
        assert_eq!(module.sections[0].lma, 0);
        assert_eq!(module.sections[1].vma_base, Some(16));
        assert_eq!(module.sections[1].lma, 16);
        // Flatten both sections (image() only returns the first) to prove the
        // multi-section split still gap-fills identically to an in-section run.
        let linked =
            sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        let mut want = vec![1, 2, 3, 4];
        want.extend(std::iter::repeat_n(0x00, 12));
        want.extend([5, 6]);
        assert_eq!(bytes, want);
    }

    #[test]
    fn org_with_no_section_open_yet_just_sets_the_phase_base() {
        // main.asm's very first `org 0` (before any byte is emitted): behaves
        // exactly like `phase`'s no-section-open path — no seek, no section
        // materializes until the next emit.
        let src = "        cpu 68000\n        padding off\n        org 0\n        dc.b 7\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections.len(), 1);
        assert_eq!(module.sections[0].vma_base, Some(0));
        assert_eq!(image(src), vec![0x07]);
    }

    /// Resolve a label's VMA (`vma_origin + offset`) from a finished module.
    fn label_vma(module: &Module, name: &str) -> u32 {
        for sec in &module.sections {
            let origin = sec.vma_origin();
            for l in &sec.labels {
                if l.name == name {
                    return origin + l.offset;
                }
            }
        }
        panic!("label `{name}` not found in module");
    }

    #[test]
    fn phase_dephase_keeps_a_continuous_physical_counter() {
        // The MovingTrucks LMA-continuity model, distilled (asl-probed, Bld 212):
        //   org 0 / 8 bytes / Base / save / cpu z80 / phase 08000h / L1 / 4 bytes
        //   / L1b / dephase / restore / L2 / 2 bytes / L3
        // asl symbol table:
        //   Base=8  L1=8000  L1b=8004  L2=C  L3=E
        // The phase block's 4 bytes advance the PHYSICAL location counter even
        // though labels INSIDE the block report window (0x8000+) VMAs. After
        // dephase/restore the counter CONTINUES from physical (8+4=0xC), it is
        // NOT rewound to a section-local 0 nor to the pre-save base.
        let src = "\
        cpu 68000\n        padding off\n        org 0\n\
        dc.b 1,2,3,4,5,6,7,8\n\
Base:\n\
        save\n        cpu z80\n        phase 08000h\n\
L1:\n\
        db 10h,11h,12h,13h\n\
L1b:\n\
        dephase\n        restore\n\
L2:\n\
        dc.b $AA,$BB\n\
L3:\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(label_vma(&module, "Base"), 0x8, "physical after 8 bytes");
        assert_eq!(label_vma(&module, "L1"), 0x8000, "window VMA inside phase");
        assert_eq!(label_vma(&module, "L1b"), 0x8004, "window VMA + 4");
        assert_eq!(
            label_vma(&module, "L2"),
            0xC,
            "physical CONTINUES past the phase block (8+4), not rewound"
        );
        assert_eq!(label_vma(&module, "L3"), 0xE, "physical + 2 more");
    }

    #[test]
    fn colon_labeled_set_reassigns_rather_than_defining_a_pc_label() {
        // asl-probed (Bld 212): a colon-label immediately followed by `set` is a
        // REASSIGNABLE-symbol assignment (colon decorative), NOT a PC label.
        //   i: set 0 / dc.b i / i: set i+5 / dc.b i / i: set i+5 / dc.b i
        // asl bytes: 00 05 0A. Treating `i:` as a PC label instead froze `i` at
        // the current address — the exact defect that made the debugger's
        // `__FSTRING_*` `.__pos: set strstr(...)` loop never terminate.
        let src = "        cpu 68000\n        padding off\n        org 0\ni:  set 0\n        dc.b i\ni:  set i+5\n        dc.b i\ni:  set i+5\n        dc.b i\n";
        assert_eq!(image(src), vec![0x00, 0x05, 0x0A]);
    }

    #[test]
    fn save_restore_does_not_resurrect_a_dephased_phase() {
        // asl-probed (Bld 212): a `save` taken WHILE phased, then `dephase`, then
        // `restore` does NOT bring the phase displacement back — `restore` only
        // restores cpu/padding/listing. Sequence:
        //   org 0 / 4 bytes / phase $8000 / A / 2 bytes / save / dephase / B
        //   / 2 bytes / restore / C
        // asl: A=8000  B=6  C=8  (C is physical 8, NOT 0x8004).
        let src = "\
        cpu 68000\n        padding off\n        org 0\n\
        dc.b 1,2,3,4\n\
        phase $8000\n\
A:\n\
        dc.b 5,6\n\
        save\n        dephase\n\
B:\n\
        dc.b 7,8\n\
        restore\n\
C:\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(label_vma(&module, "A"), 0x8000);
        assert_eq!(label_vma(&module, "B"), 0x6, "physical after dephase");
        assert_eq!(
            label_vma(&module, "C"),
            0x8,
            "restore must NOT resurrect the dephased displacement"
        );
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
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x01, 0xAA, 0xBB, 0x02]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn binclude_emits_file_bytes_verbatim() {
        // `BINCLUDE "path"` (M1.C T10): opaque binary emit, no parsing — the
        // file's raw bytes go straight into the image. Path resolves via
        // `include_root` exactly like `include` (asl-verified: same base
        // directory, real Aeon source uses `BINCLUDE "games/.../foo.bin"`
        // resolved from the aeon root). Content spans the full byte range
        // (incl. 0x00 and non-ASCII) to prove this is a raw copy, not a
        // text/db-style parse.
        let dir = std::env::temp_dir().join(format!("sigil_binc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let payload: Vec<u8> = vec![0x00, 0x41, 0xFF, 0x0A, 0x80, 0x7F];
        std::fs::write(dir.join("blob.bin"), &payload).unwrap();
        let main = dir.join("main.asm");
        std::fs::write(
            &main,
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1\n        BINCLUDE \"blob.bin\"\n        dc.b 2\n",
        )
        .unwrap();
        let m = crate::assemble_root(&main, &Options::default()).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        let mut want = vec![0x01];
        want.extend_from_slice(&payload);
        want.push(0x02);
        assert_eq!(bytes, want);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_reassigns_a_symbol() {
        // Plain reassignment (no self-reference): the second `set` simply
        // overwrites `i`.
        let src = "        cpu 68000\n        padding off\n        phase 0\ni       set 1\n        dc.b i\ni       set 9\n        dc.b i\n";
        assert_eq!(image(src), vec![0x01, 0x09]);
    }

    #[test]
    fn set_self_reference_reads_the_current_value() {
        // `i set i+5` (T8): the RHS folds against `i`'s CURRENT value at this
        // point in emission order, then overwrites it — verified against
        // real asl (see `set_accumulator` in `tests/snippets_golden.txt`).
        let src = "        cpu 68000\n        padding off\n        phase 0\ni       set 0\n        dc.b i\ni       set i+5\n        dc.b i\n";
        assert_eq!(image(src), vec![0x00, 0x05]);
    }

    #[test]
    fn coloneq_is_identical_to_set() {
        // `:=` (T8) is asl-verified to behave exactly like `set` — see
        // `coloneq_accumulator` in `tests/snippets_golden.txt`. `:=` must
        // lex as ONE `ColonEq` token so `j := 10` is never mistaken for a
        // colon-label (`j:`) followed by a stray `= 10`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nj       := 10\n        dc.b j\nj       := j*2\n        dc.b j\n";
        assert_eq!(image(src), vec![0x0A, 0x14]);
    }

    #[test]
    fn set_accumulates_inside_rept() {
        // The deform-accumulator pattern (`rept` body counter): `set`
        // converges across the multi-pass fixpoint because every value folds
        // immediately from the CURRENT pass's own execution (no dependency
        // on the seeded env from the prior pass) — see `set_in_rept` in
        // `tests/snippets_golden.txt`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nk       set 0\n        rept 4\n        dc.b k\nk       set k+1\n        endr\n";
        assert_eq!(image(src), vec![0x00, 0x01, 0x02, 0x03]);
    }

    // ── T9.1: debug string builtins (substr/strlen/strstr/val) + `!`=OR ────

    #[test]
    fn strlen_of_a_plain_string_literal() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(\"hello\")\n";
        assert_eq!(image(src), vec![5]);
    }

    #[test]
    fn substr_len_zero_means_to_the_end() {
        // asl-verified: `substr("hello",1,0)` = "ello" (len=0 = "to the end").
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(\"hello\",1,0))\n";
        assert_eq!(image(src), vec![4]);
    }

    #[test]
    fn substr_bounded_length() {
        // asl-verified: `substr("hello",1,2)` = "el".
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(\"hello\",1,2))\n";
        assert_eq!(image(src), vec![2]);
    }

    #[test]
    fn strstr_finds_the_last_character() {
        // D5 correction: asl 1.42 Bld 212's `strstr` is STANDARD — it does
        // NOT fail to find a match at the last character (`strstr("b>",">")`
        // = 1, not the alleged buggy "not found").
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"b>\",\">\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn strstr_present_mid_string() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"xab\",\"ab\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn strstr_absent_is_minus_one() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"abc\",\"z\")&$FF\n";
        assert_eq!(image(src), vec![0xFF]);
    }

    #[test]
    fn strstr_nests_over_a_substr_argument() {
        // `strstr(substr(s,p,0),">")` — the debugger's real usage shape.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(substr(\"xxb>\",2,0),\">\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn val_parses_a_dollar_hex_string() {
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b val(\"$80\")\n";
        assert_eq!(image(src), vec![0x80]);
    }

    #[test]
    fn val_parses_a_decimal_string() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b val(\"144\")&$FF\n";
        assert_eq!(image(src), vec![144]);
    }

    #[test]
    fn val_evaluates_a_symbol_plus_arithmetic_in_the_string() {
        // `val` is an AS-EXPRESSION evaluator, not a plain number parse: the
        // string's symbol reference resolves against the CURRENT env.
        let src = "        cpu 68000\n        padding off\n        phase 0\nhex     = $80\n        dc.b val(\"hex+1\")&$FF\n";
        assert_eq!(image(src), vec![0x81]);
    }

    #[test]
    fn bang_is_infix_bitwise_or() {
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b (3!4)&$FF\n";
        assert_eq!(image(src), vec![7]);
    }

    // ── T9.2: `.ATTRIBUTE` macro-suffix + `!name` escape + `while … endm` ──

    #[test]
    fn split_attribute_suffix_strips_known_suffixes_only() {
        use super::split_attribute_suffix;
        assert_eq!(split_attribute_suffix("foo.w"), Some(("foo", ".w")));
        assert_eq!(split_attribute_suffix("foo.b"), Some(("foo", ".b")));
        assert_eq!(split_attribute_suffix("foo.l"), Some(("foo", ".l")));
        assert_eq!(split_attribute_suffix("foo.s"), Some(("foo", ".s")));
        assert_eq!(split_attribute_suffix("foo"), None);
        assert_eq!(split_attribute_suffix("move"), None);
    }

    #[test]
    fn attribute_macro_binds_dot_attribute_in_a_mnemonic() {
        // asl-verified golden (`attribute_macro` in snippets_golden.txt):
        // `foo.w d1` → `move.w d1,d0` = `30 01`; `foo.l d2` → `move.l d2,d0` = `20 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro src\n        move.ATTRIBUTE src,d0\n        endm\n        foo.w d1\n        foo.l d2\n";
        assert_eq!(image(src), vec![0x30, 0x01, 0x20, 0x02]);
    }

    #[test]
    fn attribute_substitutes_inside_a_string_literal_too() {
        // `.ATTRIBUTE` is a plain (unbounded) text substitution — like
        // `ALLARGS` — so it also reaches inside a quoted string in the macro
        // body, not just a bare mnemonic. "x.ATTRIBUTEy" -> "x.wy" (4 chars);
        // without substitution it would stay "x.ATTRIBUTEy" (12 chars).
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro\n        dc.b strlen(\"x.ATTRIBUTEy\")\n        endm\n        foo.w\n";
        assert_eq!(image(src), vec![4]);
    }

    #[test]
    fn attribute_suffix_does_not_hijack_a_plain_mnemonic() {
        // No `move` macro is defined here — `move.w` must keep lowering as
        // the real instruction via `split_mnemonic_and_size`, confirming the
        // attribute-macro path (gated on the BASE name being a literal entry
        // in `self.macros`) never fires for ordinary suffixed mnemonics.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        move.w d1,d0\n";
        assert_eq!(image(src), vec![0x30, 0x01]);
    }

    #[test]
    fn while_loop_reevaluates_condition_each_iteration() {
        // asl-verified golden (`while_loop`): `n set 0 / while (n<3) / dc.b n
        // / n set n+1 / endm` → `00 01 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 0\n        while (n<3)\n        dc.b n\nn       set n+1\n        endm\n";
        assert_eq!(image(src), vec![0, 1, 2]);
    }

    #[test]
    fn while_loop_never_entered_emits_nothing() {
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 5\n        while (n<0)\n        dc.b 1\n        endm\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn while_loop_non_convergent_condition_diagnoses_instead_of_hanging() {
        // A5: a condition that never resolves to zero is bounded by
        // `WHILE_CAP` and diagnosed rather than hanging the assembler.
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 1\n        while (n)\nn       set n\n        endm\n";
        let err = run(src, &Options::default())
            .expect_err("non-convergent while must diagnose, not hang");
        assert!(
            err.iter()
                .any(|d| d.message.contains("while loop did not terminate")),
            "expected a while-non-convergence diagnostic, got {err:?}"
        );
    }

    #[test]
    fn bang_align_pads_to_the_requested_boundary() {
        // asl-verified golden (`bang_align`): odd `dc.b 1`, `!align 2`,
        // `dc.b 2` → `01 00 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1\n        !align 2\n        dc.b 2\n";
        assert_eq!(image(src), vec![1, 0, 2]);
    }

    #[test]
    fn bang_error_forces_the_builtin_directive_and_diagnoses() {
        // `even` is NOT a valid asl directive (verified "unknown
        // instruction"), so only `!error`/`!align` are in scope. A plain
        // `error` (bang or not) doesn't set `aborted`, but it does push a
        // `Level::Error` diagnostic, so `run` still fails the assembly
        // overall (no bytes emitted) — the observable "abort" the spec means.
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        !error \"boom\"\n";
        let err = run(src, &Options::default()).expect_err("!error must fail the assembly");
        assert!(
            err.iter().any(|d| d.message.contains("boom")),
            "got {err:?}"
        );
    }

    // ── T9.3: `lowstring` + `switch/case/elsecase/endcase` ────────────────

    #[test]
    fn lowstring_lowercases_a_plain_literal() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(lowstring(\"ABCD\"))\n";
        assert_eq!(image(src), vec![4]);
    }

    #[test]
    fn lowstring_nests_over_a_substr_argument() {
        // `lowstring(substr(...))` and `substr(lowstring(...), ...)` both
        // recurse through the same `eval_str` entry point (T9.3 doc on
        // `eval_str`), so nesting either way round works.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(lowstring(\"ABCDEF\"),1,3))\n";
        assert_eq!(image(src), vec![3]);
    }

    #[test]
    fn switch_case_selects_the_matching_body() {
        // asl-verified golden (`switch_case_match`): `switch
        // lowstring("HeX") / case "hex" / dc.b $80 / case "dec" / dc.b $90 /
        // elsecase / dc.b $FF / endcase` → `80` — only the matching case's
        // body assembles, the others are skipped entirely.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch lowstring(\"HeX\")\n        case \"hex\"\n        dc.b $80\n        case \"dec\"\n        dc.b $90\n        elsecase\n        dc.b $FF\n        endcase\n";
        assert_eq!(image(src), vec![0x80]);
    }

    #[test]
    fn switch_falls_through_to_elsecase_when_nothing_matches() {
        // asl-verified golden (`switch_elsecase`): a switch value matching no
        // `case` literal takes the `elsecase` (default) body.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch lowstring(\"XYZ\")\n        case \"hex\"\n        dc.b $80\n        case \"dec\"\n        dc.b $90\n        elsecase\n        dc.b $FF\n        endcase\n";
        assert_eq!(image(src), vec![0xFF]);
    }

    #[test]
    fn switch_with_no_matching_case_and_no_elsecase_emits_nothing() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch \"nope\"\n        case \"hex\"\n        dc.b $80\n        endcase\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn nested_switch_inside_a_case_body_resolves_independently() {
        // The outer switch picks its `case "a"` arm; the switch NESTED
        // inside that arm's body has its own independent case/elsecase
        // resolution — proves `find_block_end`'s nesting stack (and
        // `exec_switch`'s depth-0 arm scan) correctly isolate inner from
        // outer `switch`/`case`/`elsecase`/`endcase` keywords.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch \"a\"\n        case \"a\"\n        switch \"z\"\n        case \"y\"\n        dc.b 1\n        elsecase\n        dc.b 2\n        endcase\n        elsecase\n        dc.b 3\n        endcase\n";
        assert_eq!(image(src), vec![2]);
    }

    #[test]
    fn while_loop_nested_inside_a_macro_body_does_not_truncate_the_macro() {
        // Regression (T9.3 investigation): `find_block_end` used to
        // depth-count solely on the CALLER's own opener/closer pair, so
        // `capture_macro`'s `openers=["macro"]` scan didn't increment on a
        // nested `while`, and that nested while's own `endm` was mistaken
        // for the enclosing macro's `endm` — truncating the macro body
        // before its real end and losing the accumulator's increment line,
        // which then hung the (incompletely-captured) `while` until
        // `WHILE_CAP`. Fixed by keying the nesting stack per-opener (see
        // `closers_for`): `while … endm` nested inside `macro … endm` (the
        // exact shape debug-format macros like `__FSTRING_GenerateDecodedString`
        // use) must fully execute the loop AND run the line after it.
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro n\ni       set 0\n        while (i<n)\n        dc.b i\ni       set i+1\n        endm\n        dc.b $FF\n        endm\n        foo 3\n";
        assert_eq!(image(src), vec![0, 1, 2, 0xFF]);
    }

    #[test]
    fn fstring_format_composition_matches_asl() {
        // The payoff (T9.3): a MINIMAL `%<…>`-parsing macro modeled on
        // `debugger.asm`'s `__FSTRING_GenerateDecodedString`, composing
        // `macro` + `while` + `switch`/`case`/`elsecase` + `lowstring` +
        // `substr`/`strstr`/`strlen`/`val` — every debug-surface primitive
        // from T9.1/T9.2/T9.3 in one control-flow shape. Literal text spans
        // emit their LENGTH (`strlen(substr(...))`) rather than their raw
        // bytes: `dc.b <string-expr>` (multi-byte ASCII emission for a
        // bare/computed string argument) was found to be unimplemented in
        // `directive_db` — a real, separate gap outside T9.3's scope (see
        // the T9.3 report) — so this substitutes a byte COUNT for the
        // literal spans while still emitting the real decoded VALUE
        // (`val(...)`) for each `%<…>` token, which is the actual "bytecode"
        // half of the real macro. Byte-for-byte verified against real asl
        // (`fstring_format` in `tests/snippets_golden.txt`): `01 80 01 0A 01`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nhex     = $80\nendl    = $0A\nfstr    macro string\nlpos    set 0\nwpos    set strstr(string,\"%<\")\n        while (wpos>=0)\n        if (wpos-lpos>0)\n        dc.b strlen(substr(string,lpos,wpos-lpos))\n        endif\nepos    set strstr(substr(string,wpos+1,0),\">\")+wpos+1\n        switch substr(string,wpos+2,1)\n        case \".\"\n        switch lowstring(substr(string,wpos+2,2))\n        case \".b\"\n        dc.b val(substr(string,wpos+5,epos-wpos-5))\n        case \".w\"\n        dc.b val(substr(string,wpos+5,epos-wpos-5))|1\n        elsecase\n        dc.b val(substr(string,wpos+5,epos-wpos-5))|3\n        endcase\n        elsecase\n        dc.b val(substr(string,wpos+2,epos-wpos-2))\n        endcase\nlpos    set epos+1\nwpos    set strstr(substr(string,lpos,0),\"%<\")\n        if (wpos>=0)\nwpos    set wpos+lpos\n        endif\n        endm\n        dc.b strlen(substr(string,lpos,0))\n        endm\n        fstr \"A%<.b hex> %<endl>Z\"\n";
        assert_eq!(image(src), vec![0x01, 0x80, 0x01, 0x0A, 0x01]);
    }

    // ── T6c: `dc.b`/`db` STRING operands -> ASCII bytes (ROM header) ───────

    #[test]
    fn dc_b_string_literal_emits_ascii_bytes() {
        // asl-verified (`dc_b_string` in `tests/snippets_golden.txt`):
        // `dc.b "AB"` -> `41 42` (one ASCII byte per char), not a numeric fold.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b \"AB\"\n";
        assert_eq!(image(src), vec![0x41, 0x42]);
    }

    #[test]
    fn dc_b_mixes_string_and_numeric_operands() {
        // asl-verified (`dc_b_string_mixed`): `dc.b "Hi",0` -> `48 69 00` — a
        // string operand and a plain numeric operand in the same comma list.
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b \"Hi\",0\n";
        assert_eq!(image(src), vec![0x48, 0x69, 0x00]);
    }

    #[test]
    fn dc_b_substr_operand_emits_ascii_bytes() {
        // asl-verified (`dc_b_substr`): a T9.1 string-builtin call
        // (`substr(...)`) that RESOLVES to a string (as opposed to
        // `strlen(substr(...))`, which resolves to an int) also emits ASCII
        // bytes, not the byte count: `dc.b substr("hello",1,2)` -> `65 6C`.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b substr(\"hello\",1,2)\n";
        assert_eq!(image(src), vec![0x65, 0x6C]);
    }

    #[test]
    fn db_alias_also_emits_ascii_for_string_operands() {
        // `db` is the same directive as `dc.b` (see the dispatch match arm
        // `"db" | "dc.b" => self.directive_db(...)`), so it must get the same
        // string-operand handling.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        db \"AB\"\n";
        assert_eq!(image(src), vec![0x41, 0x42]);
    }

    #[test]
    fn dc_b_numeric_operand_still_folds_as_before() {
        // Regression guard: a plain numeric operand must still take the
        // numeric-fold path, not be misdetected as a string.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b $41\n";
        assert_eq!(image(src), vec![0x41]);
    }
}
