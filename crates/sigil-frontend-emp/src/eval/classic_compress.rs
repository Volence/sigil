//! Classic-format `.emp` comptime compression builtins (Plan-7 #10, T2b):
//! `kosinski`/`kosinski_m`/`kosplus`/`kosplus_m`/`saxman`/`enigma`/
//! `nemesis`/`comper`/`rocket`, wired atop the vendored, safe wrappers in
//! `sigil-clownlzss-sys`/`sigil-clownnemesis-sys` (T2a).
//!
//! Mirrors `s4lz.rs`'s split (see that file's module doc, itself modeled on
//! `sandbox.rs`'s `eval_zx0`/`zx0_from_data`) between argument arity/type/
//! name checking (`eval_*`) and the flatten-and-compress core (`*_from_data`),
//! so unit tests can drive the core directly with a hand-built [`DataBuf`]
//! without constructing AST argument nodes.
//!
//! **CR4 (raw format streams)**: every builtin here emits EXACTLY the -sys
//! wrapper's output bytes — no aeon 4-byte wrapper (that's `zx0`-specific),
//! no headers beyond what the format itself defines.
//!
//! **CR5 (diagnostics)**: every typed [`sigil_clownlzss_sys::Error`] /
//! [`sigil_clownnemesis_sys::Error`] variant this task's builtins can
//! surface is mapped to its own `[builtin.constraint-id]` diagnostic below —
//! no catch-all that hides which constraint fired. The data-argument
//! extraction/validation prologue (arity/name/type checking, cell
//! flattening) is shared via [`Evaluator::eval_sole_data_arg`] (this file)
//! and [`Evaluator::flatten_data_buf_tagged`] (`compress_common.rs`); each
//! builtin still writes its OWN diagnostic text/ids per the plan's
//! instruction to keep per-builtin messages distinguishable.
//!
//! None of these builtins read a file themselves (same as `zx0`/`s4lz`) —
//! their `data` argument already carries its own capture edge from
//! whatever produced it — so none of them need a sandbox root.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::{Cell, DataBuf, Value};
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Shared arg-shape prologue for every classic builtin that takes JUST
    /// one positional `data` argument and no named arguments (`kosinski`,
    /// `kosplus`, `enigma`, `nemesis`, `comper`, `rocket`): exactly one
    /// positional argument, evaluated and required to be a [`Value::Data`].
    /// `name` is the builtin's name, used both in the arity-error message
    /// and as the `[<name>.arg]` diagnostic id (mirrors `zx0`'s `[zx0.arg]`).
    ///
    /// A second positional argument, or any named argument, is a diagnostic
    /// (not a panic) naming `name` — mirrors `eval_zx0`'s single-arg check.
    fn eval_sole_data_arg(&mut self, name: &str, args: &[ast::Arg], span: Span, env: &mut Env) -> Option<DataBuf> {
        if args.len() != 1 {
            self.error(span, format!("`{name}` expects exactly 1 argument, got {}", args.len()));
            return None;
        }
        if args[0].name.is_some() {
            self.error(args[0].span, format!("`{name}` takes a positional argument"));
        }
        let arg = self.eval_expr(&args[0].value, env);
        if self.aborted || self.pending_return.is_some() {
            return None;
        }
        match arg {
            Value::Data(buf) => Some(buf),
            Value::Poison => None,
            other => {
                self.error(span, format!("[{name}.arg] {name} expects a Data value, got {}", other.type_name()));
                None
            }
        }
    }

    /// Shared arg-COLLECTION prologue for the `_m` (moduled) builtins: one
    /// positional `data` argument plus an optional named `module_size:`
    /// (default `$1000`, mirroring `sigil_clownlzss_sys::MAX_MODULE_SIZE`).
    /// Returns `(data_buf, module_size)` on success. `name` is the calling
    /// builtin's name (`kosinski_m`, `kosplus_m`, ...), used in every
    /// diagnostic here so a moduled-builtin failure is always attributable.
    ///
    /// `module_size` must be a comptime, non-negative integer that fits
    /// `u16` — the upper bound (`> $1000`) and the `== 0` case are both
    /// CALLER-checked (each `_m` builtin's own `[<name>.module-size]`
    /// diagnostic wording differs slightly from the plain `-sys` wrapper's
    /// `ModuleSizeTooLarge`, since `module_size == 0` never reaches the C
    /// wrapper at all — clownlzss's own moduled compressor would divide by
    /// it).
    fn eval_moduled_args(
        &mut self,
        name: &str,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Option<(DataBuf, u16)> {
        let mut data_arg: Option<&ast::Arg> = None;
        let mut module_size_arg: Option<&ast::Arg> = None;
        for arg in args {
            match arg.name.as_deref() {
                None => {
                    if data_arg.is_some() {
                        self.error(arg.span, format!("`{name}` takes exactly one positional data argument"));
                    } else {
                        data_arg = Some(arg);
                    }
                }
                Some("module_size") => {
                    if module_size_arg.is_some() {
                        self.error(arg.span, "`module_size` given more than once");
                    }
                    module_size_arg = Some(arg);
                }
                Some(other) => {
                    self.error(arg.span, format!("unknown named argument `{other}` to `{name}`"));
                }
            }
        }
        let Some(data_arg) = data_arg else {
            self.error(span, format!("`{name}` requires a data argument"));
            return None;
        };
        let data_val = self.eval_expr(&data_arg.value, env);
        if self.aborted || self.pending_return.is_some() {
            return None;
        }
        let buf = match data_val {
            Value::Data(buf) => buf,
            Value::Poison => return None,
            other => {
                self.error(span, format!("[{name}.arg] {name} expects a Data value, got {}", other.type_name()));
                return None;
            }
        };

        let module_size: u16 = match module_size_arg {
            None => sigil_clownlzss_sys::MAX_MODULE_SIZE as u16,
            Some(arg) => {
                let v = self.eval_expr(&arg.value, env);
                if self.aborted || self.pending_return.is_some() {
                    return None;
                }
                let n = match v.as_stored_int() {
                    Some(n) => n,
                    None => {
                        self.error(
                            arg.span,
                            format!(
                                "`module_size` must be a comptime int, got {}",
                                v.type_name()
                            ),
                        );
                        return None;
                    }
                };
                if n <= 0 || n > sigil_clownlzss_sys::MAX_MODULE_SIZE as i128 {
                    self.error(
                        span,
                        format!(
                            "[{name}.module-size] {name} module_size {n:#x} must be in 1..={:#x} \
                             (0 is not a valid module size; the 12-bit moduled header cannot represent \
                             more than {:#x})",
                            sigil_clownlzss_sys::MAX_MODULE_SIZE,
                            sigil_clownlzss_sys::MAX_MODULE_SIZE,
                        ),
                    );
                    return None;
                }
                n as u16
            }
        };

        Some((buf, module_size))
    }

    /// Map a [`sigil_clownlzss_sys::Error`] to a `[<name>.*]` diagnostic and
    /// report it, for the classic (clownlzss-backed) builtins. `name` is
    /// the calling builtin's name (used as the diagnostic id prefix).
    fn report_clownlzss_error(&mut self, name: &str, span: Span, err: sigil_clownlzss_sys::Error) {
        use sigil_clownlzss_sys::Error;
        match err {
            Error::ModuleSizeTooLarge { requested, max } => {
                self.error(
                    span,
                    format!(
                        "[{name}.module-size] {name} module_size {requested:#x} exceeds the maximum \
                         {max:#x} representable by the moduled header"
                    ),
                );
            }
            // Defensive: the module_size argument parser above already
            // rejects 0 with its own [<name>.module-size] diagnostic, so
            // this arm is unreachable from `.emp` today — kept exhaustive
            // (no catch-all) so a future call path cannot slip through
            // silently.
            Error::ModuleSizeZero => {
                self.error(
                    span,
                    format!("[{name}.module-size] {name} module_size 0 is not a valid module size"),
                );
            }
            Error::DataTooLargeForModuled { len, max } => {
                self.error(
                    span,
                    format!(
                        "[{name}.data-too-large] {name} input is {len} bytes — the 16-bit moduled \
                         header cannot represent more than {max} bytes at this module_size"
                    ),
                );
            }
            Error::CompressedSizeExceedsU16 { actual } => {
                self.error(
                    span,
                    format!(
                        "[{name}.too-large] {name} compressed output is {actual} bytes, exceeds the \
                         65535-byte u16 header field"
                    ),
                );
            }
            Error::NotWordEven { len } => {
                self.error(
                    span,
                    format!("[{name}.word-even] {name} input is {len} bytes — must be word-even (a multiple of 2)"),
                );
            }
            Error::Overflow => {
                self.error(span, format!("[{name}.overflow] {name} compression failed"));
            }
        }
    }

    // -----------------------------------------------------------------
    // Kosinski / Kosinski-Moduled
    // -----------------------------------------------------------------

    /// `kosinski(data)` (Plan-7 #10, T2b, MUST-HAVE): plain Kosinski
    /// compression, emitting the raw stream (CR4 — no aeon wrapper).
    pub(super) fn eval_kosinski(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("kosinski", args, span, env) else {
            return Value::Poison;
        };
        self.kosinski_from_data(buf, span)
    }

    pub(crate) fn kosinski_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "kosinski") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_kosinski(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("kosinski", span, e);
                Value::Poison
            }
        }
    }

    /// `kosinski_m(data)` / `kosinski_m(data, module_size: N)` (Plan-7 #10,
    /// T2b, MUST-HAVE): Kosinski-Moduled compression. `module_size` defaults
    /// to `$1000`; `N > $1000` or `N == 0` is a loud `[kosinski_m.module-size]`
    /// diagnostic (checked here, arg-shape level, before ever calling the
    /// C compressor — `N == 0` in particular is never a valid moduled split).
    pub(super) fn eval_kosinski_m(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some((buf, module_size)) = self.eval_moduled_args("kosinski_m", args, span, env) else {
            return Value::Poison;
        };
        self.kosinski_m_from_data(buf, module_size, span)
    }

    pub(crate) fn kosinski_m_from_data(&mut self, buf: DataBuf, module_size: u16, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "kosinski_m") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_kosinski_moduled(&input, module_size) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("kosinski_m", span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Kosinski+ / Kosinski+-Moduled
    // -----------------------------------------------------------------

    /// `kosplus(data)` (Plan-7 #10, T2b): plain Kosinski+ compression,
    /// emitting the raw stream (CR4).
    pub(super) fn eval_kosplus(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("kosplus", args, span, env) else {
            return Value::Poison;
        };
        self.kosplus_from_data(buf, span)
    }

    pub(crate) fn kosplus_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "kosplus") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_kosplus(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("kosplus", span, e);
                Value::Poison
            }
        }
    }

    /// `kosplus_m(data)` / `kosplus_m(data, module_size: N)` (Plan-7 #10,
    /// T2b): Kosinski+-Moduled compression, same `module_size` contract as
    /// `kosinski_m`.
    pub(super) fn eval_kosplus_m(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some((buf, module_size)) = self.eval_moduled_args("kosplus_m", args, span, env) else {
            return Value::Poison;
        };
        self.kosplus_m_from_data(buf, module_size, span)
    }

    pub(crate) fn kosplus_m_from_data(&mut self, buf: DataBuf, module_size: u16, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "kosplus_m") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_kosplus_moduled(&input, module_size) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("kosplus_m", span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Saxman
    // -----------------------------------------------------------------

    /// `saxman(data)` / `saxman(data, header: bool)` (Plan-7 #10, T2b):
    /// Saxman compression. `header` defaults to `true` (the with-header
    /// variant, which prefixes a 2-byte LE compressed-size field), matching
    /// the plan's "with-header default" ruling. Follows `s4lz`'s named-arg
    /// collection pattern (`eval_s4lz`): unknown named arguments and
    /// argument-given-twice are diagnostics, not panics.
    pub(super) fn eval_saxman(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let mut data_arg: Option<&ast::Arg> = None;
        let mut header_arg: Option<&ast::Arg> = None;
        for arg in args {
            match arg.name.as_deref() {
                None => {
                    if data_arg.is_some() {
                        self.error(arg.span, "`saxman` takes exactly one positional data argument");
                    } else {
                        data_arg = Some(arg);
                    }
                }
                Some("header") => {
                    if header_arg.is_some() {
                        self.error(arg.span, "`header` given more than once");
                    }
                    header_arg = Some(arg);
                }
                Some(other) => {
                    self.error(arg.span, format!("unknown named argument `{other}` to `saxman`"));
                }
            }
        }
        let Some(data_arg) = data_arg else {
            self.error(span, "`saxman` requires a data argument");
            return Value::Poison;
        };
        let data_val = self.eval_expr(&data_arg.value, env);
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let buf = match data_val {
            Value::Data(buf) => buf,
            Value::Poison => return Value::Poison,
            other => {
                self.error(span, format!("[saxman.arg] saxman expects a Data value, got {}", other.type_name()));
                return Value::Poison;
            }
        };

        let header = match header_arg {
            None => true,
            Some(arg) => {
                let v = self.eval_expr(&arg.value, env);
                if self.aborted || self.pending_return.is_some() {
                    return Value::Poison;
                }
                match v {
                    Value::Bool(b) => b,
                    Value::Poison => return Value::Poison,
                    other => {
                        self.error(arg.span, format!("`header` must be a bool, got {}", other.type_name()));
                        return Value::Poison;
                    }
                }
            }
        };

        self.saxman_from_data(buf, header, span)
    }

    pub(crate) fn saxman_from_data(&mut self, buf: DataBuf, header: bool, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "saxman") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_saxman(&input, header) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("saxman", span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Enigma
    // -----------------------------------------------------------------

    /// `enigma(data)` (Plan-7 #10, T2b): Enigma compression, emitting the
    /// raw stream (CR4). `data` must be word-even (a multiple of 2 bytes) —
    /// checked here via [`sigil_clownlzss_sys::Error::NotWordEven`] surfaced
    /// as `[enigma.word-even]`.
    pub(super) fn eval_enigma(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("enigma", args, span, env) else {
            return Value::Poison;
        };
        self.enigma_from_data(buf, span)
    }

    pub(crate) fn enigma_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "enigma") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_enigma(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("enigma", span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Nemesis
    // -----------------------------------------------------------------

    /// Map a [`sigil_clownnemesis_sys::Error`] to a `[nemesis.*]` diagnostic
    /// and report it (CR5) — the clownnemesis-backed sibling of
    /// [`report_clownlzss_error`](Self::report_clownlzss_error), kept
    /// separate since the two `-sys` crates expose distinct `Error` enums.
    fn report_clownnemesis_error(&mut self, span: Span, err: sigil_clownnemesis_sys::Error) {
        use sigil_clownnemesis_sys::Error;
        match err {
            Error::NotTileAligned { len } => {
                self.error(
                    span,
                    format!(
                        "[nemesis.tile-granularity] nemesis input is {len} bytes — must be a multiple of \
                         $20 (one 8x8 4bpp tile)"
                    ),
                );
            }
            Error::TooManyTiles { tiles, max_tiles } => {
                self.error(
                    span,
                    format!(
                        "[nemesis.too-many-tiles] nemesis input has {tiles} tiles, exceeds the maximum \
                         {max_tiles} representable by the 15-bit header field"
                    ),
                );
            }
            Error::Overflow => {
                self.error(span, "[nemesis.overflow] nemesis compression failed");
            }
        }
    }

    /// `nemesis(data)` (Plan-7 #10, T2b): Nemesis compression, emitting the
    /// raw stream (CR4). `data` must be a multiple of `$20` bytes (one
    /// 8x8 4bpp Genesis tile) and at most 32767 tiles (the 15-bit header
    /// field's ceiling) — both enforced by
    /// [`sigil_clownnemesis_sys::compress`] and surfaced here.
    pub(super) fn eval_nemesis(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("nemesis", args, span, env) else {
            return Value::Poison;
        };
        self.nemesis_from_data(buf, span)
    }

    pub(crate) fn nemesis_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "nemesis") else {
            return Value::Poison;
        };
        match sigil_clownnemesis_sys::compress(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownnemesis_error(span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Comper
    // -----------------------------------------------------------------

    /// `comper(data)` (Plan-7 #10, T2b): Comper compression, emitting the
    /// raw stream (CR4). `data` must be word-even, same constraint as
    /// `enigma` — surfaced via `report_clownlzss_error`'s existing
    /// `NotWordEven` arm as `[comper.word-even]`.
    pub(super) fn eval_comper(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("comper", args, span, env) else {
            return Value::Poison;
        };
        self.comper_from_data(buf, span)
    }

    pub(crate) fn comper_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "comper") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_comper(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("comper", span, e);
                Value::Poison
            }
        }
    }

    // -----------------------------------------------------------------
    // Rocket
    // -----------------------------------------------------------------

    /// `rocket(data)` (Plan-7 #10, T2b): Rocket compression, emitting the
    /// raw stream (CR4). Upstream has no header-less Rocket compressor, so
    /// (unlike Saxman) there is no `header:` argument.
    pub(super) fn eval_rocket(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let Some(buf) = self.eval_sole_data_arg("rocket", args, span, env) else {
            return Value::Poison;
        };
        self.rocket_from_data(buf, span)
    }

    pub(crate) fn rocket_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let Some(input) = self.flatten_data_buf_tagged(&buf, span, "rocket") else {
            return Value::Poison;
        };
        match sigil_clownlzss_sys::compress_rocket(&input) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(e) => {
                self.report_clownlzss_error("rocket", span, e);
                Value::Poison
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Evaluator;

    fn span() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn kosinski_from_data_matches_sys_wrapper() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        let input = vec![0xAB, 0xCD, 0xAB, 0xCD, 0x11, 0x22, 0x33, 0x44];
        buf.push(Cell::Bytes(input.clone()));
        let result = ev.kosinski_from_data(buf, span());
        let expected = sigil_clownlzss_sys::compress_kosinski(&input).unwrap();
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }

    #[test]
    fn kosinski_from_data_symbolic_cell_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::SymRef { name: "Foo".into(), width: 2, windowed: false });
        let result = ev.kosinski_from_data(buf, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[kosinski.symbolic]")));
    }

    #[test]
    fn kosinski_m_from_data_matches_sys_wrapper() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        let input: Vec<u8> = (0..64u8).collect();
        buf.push(Cell::Bytes(input.clone()));
        let result = ev.kosinski_m_from_data(buf, 0x100, span());
        let expected = sigil_clownlzss_sys::compress_kosinski_moduled(&input, 0x100).unwrap();
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }

    /// `saxman(data, header: true)`'s compressed-size-exceeds-u16 case
    /// (CR5): mirrors `sigil_clownlzss_sys`'s own
    /// `saxman_with_header_rejects_compressed_size_over_u16` gate test —
    /// ~70KB of incompressible pseudo-random data pushes the WITH-HEADER
    /// compressed size past `u16::MAX`. Exercised via `saxman_from_data`
    /// directly (not the `.emp` integration suite) since constructing a
    /// 70KB `.emp` source literal would be impractical — same rationale as
    /// `s4lz_from_data_too_large_input_errors` in `s4lz.rs`.
    #[test]
    fn saxman_from_data_with_header_compressed_size_exceeds_u16_errors() {
        let mut plain = Vec::with_capacity(70_000);
        let mut state: u32 = 0x2463_5910;
        for _ in 0..70_000 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            plain.push((state & 0xFF) as u8);
        }
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(plain));
        let result = ev.saxman_from_data(buf, true, span());
        assert_eq!(result, Value::Poison);
        assert!(
            ev.diags.iter().any(|d| d.message.contains("[saxman.too-large]")),
            "expected a [saxman.too-large] diagnostic, got {:?}",
            ev.diags
        );
    }

    #[test]
    fn saxman_from_data_header_false_matches_sys_wrapper() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        let input: Vec<u8> = (0..64u8).collect();
        buf.push(Cell::Bytes(input.clone()));
        let result = ev.saxman_from_data(buf, false, span());
        let expected = sigil_clownlzss_sys::compress_saxman(&input, false).unwrap();
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }

    /// `nemesis(data)`'s too-many-tiles case (CR5): a real `.emp` fixture
    /// spanning 32768 tiles would be a ~1MB file for no diagnostic value
    /// (same rationale `sigil-clownnemesis-sys`'s own
    /// `tile_count_boundary_check_logic` test documents) — exercised via
    /// `nemesis_from_data` directly instead.
    #[test]
    fn nemesis_from_data_too_many_tiles_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        let one_too_many = vec![0u8; (sigil_clownnemesis_sys::MAX_TILES + 1) * sigil_clownnemesis_sys::TILE_SIZE];
        buf.push(Cell::Bytes(one_too_many));
        let result = ev.nemesis_from_data(buf, span());
        assert_eq!(result, Value::Poison);
        assert!(
            ev.diags.iter().any(|d| d.message.contains("[nemesis.too-many-tiles]")),
            "expected a [nemesis.too-many-tiles] diagnostic, got {:?}",
            ev.diags
        );
    }
}
