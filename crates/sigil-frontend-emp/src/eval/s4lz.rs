//! `s4lz(data)` / `s4lz(data, dict: d)` / `s4lz(data, tile_delta: true)`
//! (Plan-7 #10, Tier 1, CR-S4LZ): S4LZ-v3-compresses a [`Value::Data`] at
//! comptime via the pure-Rust byte-exact port in [`sigil_s4lz`].
//!
//! Mirrors `zx0`'s split (see `sandbox.rs`'s `eval_zx0`/`zx0_from_data`
//! doc comments) between argument arity/type/name checking
//! ([`eval_s4lz`](Evaluator::eval_s4lz)) and the flatten-and-compress core
//! ([`s4lz_from_data`](Evaluator::s4lz_from_data)), so a unit test can drive
//! the core directly with a hand-built [`DataBuf`] (e.g. an oversized input,
//! or an odd-length dictionary) without constructing AST argument nodes.
//!
//! Unlike `zx0`, `s4lz` has two OPTIONAL named arguments (`dict:`/
//! `tile_delta:`) on top of the one positional `data` argument — this
//! follows `embed`'s `skip:`/`len:` named-argument convention (see
//! `sandbox.rs`'s `eval_embed`): unknown named arguments and
//! argument-given-twice are both diagnostics, not panics.
//!
//! `s4lz` reads no file itself (same as `zx0`) — both `data` and `dict:`
//! already carry their own capture edges from whatever produced them — so
//! it needs no sandbox root and never calls `record_capture`.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::{Cell, DataBuf, Value};
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// `s4lz(data, dict: d, tile_delta: b)` (Plan-7 #10, Tier 1): argument
    /// arity/name/type checking, then delegates to
    /// [`s4lz_from_data`](Self::s4lz_from_data).
    ///
    /// `data` is the sole positional argument; `dict` and `tile_delta` are
    /// optional named arguments. `dict`, when given, must itself be a
    /// [`Value::Data`] (same acceptance rules as `data`). `tile_delta`, when
    /// given, must be a comptime `bool`. Giving both `dict:` and
    /// `tile_delta:` is `[s4lz.dict-tile-delta-exclusive]` — checked here
    /// (arg-shape level) so it fires even before the flatten-to-bytes core
    /// runs, mirroring `s4lz.py`'s `ValueError` at the top of `compress()`.
    pub(super) fn eval_s4lz(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let mut data_arg: Option<&ast::Arg> = None;
        let mut dict_arg: Option<&ast::Arg> = None;
        let mut tile_delta_arg: Option<&ast::Arg> = None;
        for arg in args {
            match arg.name.as_deref() {
                None => {
                    if data_arg.is_some() {
                        self.error(arg.span, "`s4lz` takes exactly one positional data argument");
                    } else {
                        data_arg = Some(arg);
                    }
                }
                Some("dict") => {
                    if dict_arg.is_some() {
                        self.error(arg.span, "`dict` given more than once");
                    }
                    dict_arg = Some(arg);
                }
                Some("tile_delta") => {
                    if tile_delta_arg.is_some() {
                        self.error(arg.span, "`tile_delta` given more than once");
                    }
                    tile_delta_arg = Some(arg);
                }
                Some(other) => {
                    self.error(arg.span, format!("unknown named argument `{other}` to `s4lz`"));
                }
            }
        }
        let Some(data_arg) = data_arg else {
            self.error(span, "`s4lz` requires a data argument");
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
                self.error(span, format!("[s4lz.arg] s4lz expects a Data value, got {}", other.type_name()));
                return Value::Poison;
            }
        };

        // Evaluate `tile_delta:` (default false) before `dict:` so the
        // dict+tile_delta exclusivity check can fire without needing the
        // dict bytes flattened first.
        let tile_delta = match tile_delta_arg {
            None => false,
            Some(arg) => {
                let v = self.eval_expr(&arg.value, env);
                if self.aborted || self.pending_return.is_some() {
                    return Value::Poison;
                }
                match v {
                    Value::Bool(b) => b,
                    Value::Poison => return Value::Poison,
                    other => {
                        self.error(
                            arg.span,
                            format!("`tile_delta` must be a bool, got {}", other.type_name()),
                        );
                        return Value::Poison;
                    }
                }
            }
        };

        if tile_delta && dict_arg.is_some() {
            self.error(
                span,
                "[s4lz.dict-tile-delta-exclusive] s4lz cannot take both `dict:` and `tile_delta:` \
                 (mirrors s4lz.py's ValueError: dictionary is not supported with tile_delta)",
            );
            return Value::Poison;
        }

        let dict_bytes = match dict_arg {
            None => None,
            Some(arg) => {
                let v = self.eval_expr(&arg.value, env);
                if self.aborted || self.pending_return.is_some() {
                    return Value::Poison;
                }
                let dict_buf = match v {
                    Value::Data(b) => b,
                    Value::Poison => return Value::Poison,
                    other => {
                        self.error(
                            arg.span,
                            format!("[s4lz.dict-arg] s4lz dict must be a Data value, got {}", other.type_name()),
                        );
                        return Value::Poison;
                    }
                };
                match self.flatten_data_buf(&dict_buf, span, "s4lz.dict") {
                    Some(bytes) => Some(bytes),
                    None => return Value::Poison,
                }
            }
        };

        if let Some(d) = &dict_bytes {
            if !d.len().is_multiple_of(2) {
                self.error(
                    span,
                    format!(
                        "[s4lz.dict-odd] s4lz dict length {} must be word-even (mirrors s4lz.py's ValueError)",
                        d.len()
                    ),
                );
                return Value::Poison;
            }
        }

        self.s4lz_from_data(buf, dict_bytes, tile_delta, span)
    }

    /// Flatten a [`DataBuf`]'s cells to raw bytes, mirroring `zx0_from_data`'s
    /// cell-walk (`sandbox.rs`) exactly: `Cell::Bytes` extends directly; a
    /// width-1 `Cell::Scalar` contributes its one (range-checked) byte; a
    /// WIDER `Cell::Scalar` has no committed byte order yet (Plan 4's
    /// concern) and a `Cell::SymRef`/`RelOffset`/`Expr` names something not
    /// yet resolved (also Plan 4) — all three are diagnostics here, never a
    /// panic, since `s4lz` can only compress concrete bytes.
    ///
    /// `tag` names the call site in the diagnostic code (`s4lz` for the main
    /// `data` argument, `s4lz.dict` for `dict:`) so a dict-side failure is
    /// distinguishable from a data-side one.
    fn flatten_data_buf(&mut self, buf: &DataBuf, span: Span, tag: &str) -> Option<Vec<u8>> {
        let mut input = Vec::with_capacity(buf.size);
        for cell in &buf.cells {
            match cell {
                Cell::Bytes(b) => input.extend_from_slice(b),
                Cell::Scalar { value, width: 1, .. } => {
                    if !(super::builtins::BYTE_LO..=super::builtins::BYTE_HI).contains(value) {
                        self.error(
                            span,
                            format!("[{tag}.byte-range] s4lz input byte {value} does not fit 8 bits"),
                        );
                        return None;
                    }
                    input.push((*value & 0xFF) as u8);
                }
                Cell::Scalar { .. } => {
                    self.error(
                        span,
                        format!(
                            "[{tag}.byte-order] s4lz input has a multi-byte scalar with no committed \
                             byte order — build it from raw bytes (embed/bytes)"
                        ),
                    );
                    return None;
                }
                Cell::SymRef { .. } => {
                    self.error(span, format!("[{tag}.symbolic] s4lz input has an unresolved symbol reference"));
                    return None;
                }
                Cell::RelOffset { .. } => {
                    self.error(span, format!("[{tag}.symbolic] s4lz input has an unresolved offset-table entry"));
                    return None;
                }
                Cell::Expr { .. } => {
                    self.error(span, format!("[{tag}.symbolic] s4lz input has an unresolved link-expr value"));
                    return None;
                }
            }
        }
        Some(input)
    }

    /// The flatten-compress-wrap core of `s4lz` (Plan-7 #10, Tier 1). See
    /// [`eval_s4lz`](Self::eval_s4lz)'s doc comment for why this is split
    /// out. `dict` is already-flattened bytes (or `None`); `tile_delta` is
    /// already resolved to a plain bool; both are validated (word-even dict,
    /// dict+tile_delta exclusivity) by the caller.
    ///
    /// Asserts the flattened `data` length fits the wrapper's `u16` size
    /// field combined with the dictionary (mirrors `s4lz.py`'s `dict_len +
    /// data_len > MAX_WINDOW` check) before calling
    /// [`sigil_s4lz::try_compress`].
    pub(crate) fn s4lz_from_data(
        &mut self,
        buf: DataBuf,
        dict: Option<Vec<u8>>,
        tile_delta: bool,
        span: Span,
    ) -> Value {
        let Some(input) = self.flatten_data_buf(&buf, span, "s4lz") else {
            return Value::Poison;
        };

        // Defense-in-depth, currently UNREACHABLE: s4lz.py (and our port)
        // reject dict+data > MAX_WINDOW (32766) first, so no input can reach
        // this u16-header-field ceiling today. It stays because the header
        // packs data_len as u16 BE — if MAX_WINDOW were ever raised past
        // 65535, this check (not silent truncation) must be what fires.
        if input.len() > 0xFFFF {
            self.error(
                span,
                format!(
                    "[s4lz.too-large] s4lz input is {} bytes, exceeds the 65535-byte u16 size field",
                    input.len()
                ),
            );
            return Value::Poison;
        }

        let opts = sigil_s4lz::Options { tile_delta, dictionary: dict.unwrap_or_default() };
        match sigil_s4lz::try_compress(&input, &opts) {
            Ok(out) => {
                let mut result = DataBuf::empty();
                result.push(Cell::Bytes(out));
                Value::Data(result)
            }
            Err(sigil_s4lz::CompressError::DictTileDeltaExclusive) => {
                // Unreachable in practice: eval_s4lz already rejects this
                // combination before calling here. Kept as a defensive
                // diagnostic (not a panic) in case a future caller of this
                // `pub(crate)` core skips that check.
                self.error(
                    span,
                    "[s4lz.dict-tile-delta-exclusive] s4lz cannot take both `dict:` and `tile_delta:`",
                );
                Value::Poison
            }
            Err(sigil_s4lz::CompressError::DictLengthOdd(n)) => {
                self.error(span, format!("[s4lz.dict-odd] s4lz dict length {n} must be word-even"));
                Value::Poison
            }
            Err(sigil_s4lz::CompressError::WindowExceeded { dict_len, data_len }) => {
                self.error(
                    span,
                    format!(
                        "[s4lz.too-large] s4lz dict+data {} bytes (dict {dict_len} + data {data_len}) \
                         exceeds the {}-byte window",
                        dict_len + data_len,
                        32766
                    ),
                );
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
    fn s4lz_from_data_too_large_input_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(vec![0u8; 0x10000]));
        let result = ev.s4lz_from_data(buf, None, false, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[s4lz.too-large]")));
    }

    #[test]
    fn s4lz_from_data_multibyte_scalar_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: 0x1234, width: 2, signed: false, le: false });
        let result = ev.s4lz_from_data(buf, None, false, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[s4lz.byte-order]")));
    }

    #[test]
    fn s4lz_from_data_byte_range_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: 999, width: 1, signed: false, le: false });
        let result = ev.s4lz_from_data(buf, None, false, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[s4lz.byte-range]")));
    }

    #[test]
    fn s4lz_from_data_empty_input() {
        let mut ev = Evaluator::new();
        let result = ev.s4lz_from_data(DataBuf::empty(), None, false, span());
        let expected = sigil_s4lz::compress(&[], &sigil_s4lz::Options::default());
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
        assert!(ev.diags.is_empty(), "empty input should not diagnose: {:?}", ev.diags);
    }

    #[test]
    fn s4lz_from_data_odd_dict_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(vec![1, 2, 3, 4]));
        let result = ev.s4lz_from_data(buf, Some(vec![1, 2, 3]), false, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[s4lz.dict-odd]")));
    }

    #[test]
    fn s4lz_from_data_dict_matches_core_compress() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(vec![0xAB, 0xCD, 0xAB, 0xCD]));
        let dict = vec![0xAB, 0xCD, 0x11, 0x22];
        let result = ev.s4lz_from_data(buf, Some(dict.clone()), false, span());
        let expected = sigil_s4lz::compress(
            &[0xAB, 0xCD, 0xAB, 0xCD],
            &sigil_s4lz::Options::with_dictionary(dict),
        );
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }

    #[test]
    fn s4lz_from_data_tile_delta_matches_core_compress() {
        let mut ev = Evaluator::new();
        let data: Vec<u8> = (0..64u8).collect();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(data.clone()));
        let result = ev.s4lz_from_data(buf, None, true, span());
        let expected = sigil_s4lz::compress(&data, &sigil_s4lz::Options::with_tile_delta());
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => assert_eq!(b, &expected),
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }
}
