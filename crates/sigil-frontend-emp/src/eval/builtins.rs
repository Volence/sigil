//! The §6.8 builtins: array/range `len`/`map`/`filter`/`fold`, string
//! `len`/`find`/`slice`/`val`, and the dispatch that resolves a call
//! expression's callee/receiver into one of them.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::Value;
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Dispatch a §6.8 builtin call, extracting the receiver and the builtin's
    /// positional arguments from the two surface forms:
    /// - method form (`recv.method(args...)`, `callee.segments.len() >= 2`): the
    ///   receiver is the callee prefix `recv`, the builtin args are `args`.
    /// - free/pipe form (`method(recv, args...)`, single-segment callee — this
    ///   is also the shape a `recv |> method(args...)` pipe desugars to): the
    ///   receiver is the first arg, the builtin args are the rest.
    ///
    /// Builtins take positional args only; a named arg is diagnosed. A `Poison`
    /// receiver propagates silently.
    pub(super) fn eval_builtin_call(
        &mut self,
        callee: &ast::Path,
        method: String,
        args: &[ast::Arg],
        span: Span,
        env: &mut Env,
    ) -> Value {
        let (receiver, arg_values) = if callee.segments.len() >= 2 {
            // Method form: the receiver is the callee's prefix path.
            let n = callee.segments.len();
            let prefix = ast::Path {
                segments: callee.segments[..n - 1].to_vec(),
                span: callee.span,
            };
            let recv = self.eval_expr(&ast::Expr::Path(prefix), env);
            let vals = self.eval_builtin_args(args, env);
            (recv, vals)
        } else {
            // Free/pipe form: the receiver is the first positional argument.
            if args.is_empty() {
                self.error(span, format!("builtin `{method}` needs a receiver"));
                return Value::Poison;
            }
            let recv = self.eval_expr(&args[0].value, env);
            let vals = self.eval_builtin_args(&args[1..], env);
            (recv, vals)
        };
        if self.aborted {
            return Value::Poison;
        }
        self.eval_builtin(receiver, &method, arg_values, span)
    }

    /// Evaluate a builtin's positional argument expressions to values. A named
    /// argument is a diagnostic (builtins take positional args only); its value
    /// is still evaluated so downstream arity/type checks stay meaningful.
    fn eval_builtin_args(&mut self, args: &[ast::Arg], env: &mut Env) -> Vec<Value> {
        args.iter()
            .map(|a| {
                if a.name.is_some() {
                    self.error(a.span, "builtin methods take positional arguments only");
                }
                self.eval_expr(&a.value, env)
            })
            .collect()
    }

    /// Dispatch a resolved builtin on its receiver value (D-P2.18). Arrays and
    /// ranges share the sequence builtins (`len`/`map`/`filter`/`fold`); strings
    /// have their own set. A `Poison` receiver is silent; any other receiver type
    /// is "`method` is not defined on <type>".
    ///
    /// `len` is answered without materializing (O(1) on a range — its element
    /// count is `max(0, hi - lo)` — so `r.len` / `r.len()` never allocate). For
    /// map/filter/fold a range is consumed *lazily* with a per-element step
    /// charge (`charge = true`), so a huge range trips the step budget rather
    /// than the allocator; an array is already in memory (bounded), so it is not
    /// re-charged (`charge = false`).
    fn eval_builtin(&mut self, receiver: Value, method: &str, args: Vec<Value>, span: Span) -> Value {
        match receiver {
            Value::Array(elems) => {
                if method == "len" {
                    if !self.check_arity(method, &args, 0, span) {
                        return Value::Poison;
                    }
                    return Value::Int(elems.len() as i128);
                }
                self.eval_seq_ops(elems.into_iter(), method, "array", false, args, span)
            }
            Value::Range { lo, hi } => {
                if method == "len" {
                    if !self.check_arity(method, &args, 0, span) {
                        return Value::Poison;
                    }
                    // O(1): never materialize just to count.
                    return Value::Int((hi - lo).max(0));
                }
                self.eval_seq_ops((lo..hi).map(Value::Int), method, "range", true, args, span)
            }
            Value::Str(s) => self.eval_str_builtin(s, method, args, span),
            Value::Poison => Value::Poison,
            other => {
                self.error(span, format!("`{method}` is not defined on {}", other.type_name()));
                Value::Poison
            }
        }
    }

    /// The array/range sequence builtins `map`/`filter`/`fold` over an element
    /// stream (`len` is answered by [`eval_builtin`](Self::eval_builtin) without
    /// consuming the stream). `recv_ty` is the surface receiver type (`"array"`
    /// or `"range"`) for the unknown-method message. When `charge` is set (ranges)
    /// a step is charged per element as it is consumed, so an unbounded stream
    /// aborts on the step budget instead of the allocator; arrays pass `false`
    /// since they are already materialized and bounded.
    fn eval_seq_ops<I: Iterator<Item = Value>>(
        &mut self,
        elems: I,
        method: &str,
        recv_ty: &str,
        charge: bool,
        args: Vec<Value>,
        span: Span,
    ) -> Value {
        match method {
            "map" => {
                if !self.check_arity(method, &args, 1, span) {
                    return Value::Poison;
                }
                let f = args.into_iter().next().unwrap();
                let mut out = Vec::new();
                for el in elems {
                    if charge && !self.bump_step() {
                        self.abort(span, "step budget exceeded");
                        return Value::Poison;
                    }
                    // A `Poison` result (a bad callable, an abort, or an
                    // already-reported element error) poisons the whole map and
                    // stops — one diagnostic, no per-element cascade (D-P2.9).
                    let r = self.apply_callable(f.clone(), vec![el], span);
                    if matches!(r, Value::Poison) {
                        return Value::Poison;
                    }
                    out.push(r);
                }
                Value::Array(out)
            }
            "filter" => {
                if !self.check_arity(method, &args, 1, span) {
                    return Value::Poison;
                }
                let f = args.into_iter().next().unwrap();
                let mut out = Vec::new();
                for el in elems {
                    if charge && !self.bump_step() {
                        self.abort(span, "step budget exceeded");
                        return Value::Poison;
                    }
                    match self.apply_callable(f.clone(), vec![el.clone()], span) {
                        Value::Bool(true) => out.push(el),
                        Value::Bool(false) => {}
                        // The predicate already reported its own error upstream.
                        Value::Poison => return Value::Poison,
                        other => {
                            self.error(
                                span,
                                format!(
                                    "filter predicate must return bool, got {}",
                                    other.type_name()
                                ),
                            );
                            return Value::Poison;
                        }
                    }
                }
                Value::Array(out)
            }
            "fold" => {
                if !self.check_arity(method, &args, 2, span) {
                    return Value::Poison;
                }
                let mut it = args.into_iter();
                let mut acc = it.next().unwrap();
                let f = it.next().unwrap();
                for el in elems {
                    if charge && !self.bump_step() {
                        self.abort(span, "step budget exceeded");
                        return Value::Poison;
                    }
                    // As with `map`, a `Poison` accumulator (bad combiner or
                    // abort) short-circuits to one diagnostic (D-P2.9).
                    acc = self.apply_callable(f.clone(), vec![acc, el], span);
                    if matches!(acc, Value::Poison) {
                        return Value::Poison;
                    }
                }
                acc
            }
            _ => {
                self.error(span, format!("`{method}` is not defined on {recv_ty}"));
                Value::Poison
            }
        }
    }

    /// The string builtins: `len`, `find`, `slice`, `val` (D-P2.18). All indices
    /// are CHAR indices (Genesis strings are ASCII, but multi-byte input still
    /// behaves correctly).
    fn eval_str_builtin(
        &mut self,
        s: String,
        method: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Value {
        match method {
            "len" => {
                if !self.check_arity(method, &args, 0, span) {
                    return Value::Poison;
                }
                Value::Int(s.chars().count() as i128)
            }
            "find" => {
                if !self.check_arity(method, &args, 1, span) {
                    return Value::Poison;
                }
                let needle = match &args[0] {
                    Value::Str(n) => n,
                    Value::Poison => return Value::Poison,
                    other => {
                        self.error(
                            span,
                            format!("`find` needle must be a string, got {}", other.type_name()),
                        );
                        return Value::Poison;
                    }
                };
                // Standard first-occurrence search (NO AS `strstr` last-char bug):
                // find the byte offset, then convert it to a char index.
                match s.find(needle.as_str()) {
                    Some(byte) => Value::Int(s[..byte].chars().count() as i128),
                    None => Value::Int(-1),
                }
            }
            "slice" => {
                if !self.check_arity(method, &args, 2, span) {
                    return Value::Poison;
                }
                // Slice bounds erase a `Value::Typed` to its stored int (§8.3).
                let (start, end) = match (args[0].as_stored_int(), args[1].as_stored_int()) {
                    (Some(a), Some(b)) => (a, b),
                    _ if matches!(args[0], Value::Poison) || matches!(args[1], Value::Poison) => {
                        return Value::Poison;
                    }
                    _ => {
                        let (a, b) = (&args[0], &args[1]);
                        self.error(
                            span,
                            format!(
                                "slice bounds must be int, got {} and {}",
                                a.type_name(),
                                b.type_name()
                            ),
                        );
                        return Value::Poison;
                    }
                };
                let chars: Vec<char> = s.chars().collect();
                let n = chars.len() as i128;
                // Half-open `[start, end)`; validate `0 <= start <= end <= len`.
                if start < 0 || end < start || end > n {
                    self.error(
                        span,
                        format!("slice [{start}..{end}] out of range for string of length {n}"),
                    );
                    return Value::Poison;
                }
                let sub: String = chars[start as usize..end as usize].iter().collect();
                Value::Str(sub)
            }
            "val" => {
                if !self.check_arity(method, &args, 0, span) {
                    return Value::Poison;
                }
                self.str_val(&s, span)
            }
            _ => {
                self.error(span, format!("`{method}` is not defined on string"));
                Value::Poison
            }
        }
    }

    /// Check a builtin got exactly `want` positional arguments; emit an error and
    /// return `false` otherwise. (Any `Poison` argument is left for the caller to
    /// propagate — arity is validated regardless of argument values.)
    fn check_arity(&mut self, method: &str, args: &[Value], want: usize, span: Span) -> bool {
        if args.len() == want {
            return true;
        }
        self.error(
            span,
            format!("`{method}` expects {want} argument(s), got {}", args.len()),
        );
        false
    }

    /// Parse a string as an `.emp` integer literal for the `val` builtin,
    /// emitting a diagnostic and returning `Poison` on failure. Shared by the
    /// bare-path (`s.val`) and call (`s.val()`) forms so their semantics cannot
    /// drift apart.
    pub(super) fn str_val(&mut self, s: &str, span: Span) -> Value {
        match parse_emp_int(s) {
            Some(n) => Value::Int(n),
            None => {
                self.error(span, format!("cannot parse `{s}` as an integer"));
                Value::Poison
            }
        }
    }
}

/// Whether `name` is a §6.8 builtin method (D-P2.10 — the closed, non-user-
/// shadowable set). `len` overlaps the array/range and string sets; the receiver
/// type disambiguates at dispatch.
pub(super) fn is_builtin(name: &str) -> bool {
    matches!(name, "len" | "map" | "filter" | "fold" | "find" | "slice" | "val")
}

/// Parse a trimmed string as an `.emp` integer literal for the `val` builtin
/// (D-P2.18): an optional leading `-`, then `$HHHH`/`0xHHHH` (hex), `0bBBBB`
/// (binary), or decimal digits. Returns `None` on any malformed input. Mirrors
/// the lexer's numeric grammar (extended with `0x` as an accepted hex spelling)
/// reduced to integer literals.
fn parse_emp_int(s: &str) -> Option<i128> {
    let t = s.trim();
    let (neg, rest) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t),
    };
    // Select the radix and the digit portion, stripping any prefix.
    let (radix, digits) = if let Some(h) = rest.strip_prefix('$') {
        (16, h)
    } else if let Some(h) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        (16, h)
    } else if let Some(bits) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        (2, bits)
    } else {
        (10, rest)
    };
    // Reject any sign in the digit portion: Rust's `from_str_radix` accepts its
    // own leading `+`/`-`, which would otherwise let `+5`, `$-5`, `$+5` through
    // (our only sign is the one `-` stripped above).
    if digits.starts_with('+') || digits.starts_with('-') {
        return None;
    }
    let mag = i128::from_str_radix(digits, radix).ok()?;
    Some(if neg { -mag } else { mag })
}
