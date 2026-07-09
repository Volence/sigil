//! Checked emission (Spec 2, Plan 3 — T7, D-P3.5): lowering a typed comptime
//! [`Value`] to a CPU-neutral, structured [`DataBuf`], and the `data NAME: T =
//! expr` item evaluation that drives it.
//!
//! This is the Plan 3 / Plan 4 seam. [`lower_to_data`](Evaluator::lower_to_data)
//! range-checks each scalar against its [`Ty`] (the i128 → sized-primitive
//! "emission range-check", D-P3.3) and records the STRUCTURE — a scalar keeps
//! its width/signedness, a byte run stays raw, a pointer stays a symbolic
//! [`Cell::SymRef`] — but it commits NO endianness and resolves NO pointer
//! address. Serializing cells to bytes in target order and turning a `SymRef`
//! into a relocation is all Plan 4.
use super::{Env, Evaluator};
use crate::ast;
use crate::layout::{prim_bounds, Ty};
use crate::value::{Cell, DataBuf, Value};
use sigil_span::Span;

impl<'a> Evaluator<'a> {
    /// Lower a comptime `value` to a checked, CPU-neutral [`DataBuf`], range-
    /// checking it against `ty` (T7, D-P3.5 / D-P3.3). A [`Poison`](Value::Poison)
    /// value or a [`Ty::Poison`] is already-reported: return an empty buffer
    /// silently (D-P2.9).
    pub(crate) fn lower_to_data(&mut self, value: &Value, ty: &Ty, span: Span) -> DataBuf {
        // This guard filters Poison up front, so the per-kind leaf lowerings
        // below never see a Poison `value` — their type-mismatch diagnostics are
        // therefore unconditional (a non-Poison, wrong-shape value).
        if matches!(value, Value::Poison) || matches!(ty, Ty::Poison) {
            return DataBuf::empty();
        }
        // A link-time value (a provisional `here()`, a `bankid()`, or any
        // arithmetic over them) emitted into a data cell. `lower_link_expr` splits
        // the two paths (R7m.4): a PLAIN `LinkExpr(Sym(anchor))` keeps the
        // byte-proven D-H.3 `Cell::SymRef` address lowering; a residual arithmetic
        // tree becomes a general `Cell::Expr` VALUE cell (S2-D13f un-deferred —
        // this REPLACES the old arithmetic-then-emit refusal).
        if let Value::LinkExpr(expr) = value {
            return self.lower_link_expr(expr, ty, span);
        }
        match ty {
            Ty::Prim { width, signed, le } => self.lower_prim(value, *width, *signed, *le, span),
            Ty::Fixed { i, f } => self.lower_fixed(value, *i, *f, span),
            // A newtype/refined value erases to its stored int (§8.3) and is
            // emitted at the EFFECTIVE UNDERLYING width — re-checking the range
            // at emission, since a value in-range for the newtype's `where` bound
            // must still fit the underlying primitive it is stored in.
            Ty::Newtype(_) => {
                let Some(n) = value.as_stored_int() else {
                    self.emit_expected_int(value, ty, span);
                    return DataBuf::empty();
                };
                let underlying = self.effective_underlying(ty, span);
                if matches!(underlying, Ty::Poison) {
                    return DataBuf::empty();
                }
                self.lower_to_data(&Value::Int(n), &underlying, span)
            }
            Ty::Refined { inner, .. } => {
                let Some(n) = value.as_stored_int() else {
                    self.emit_expected_int(value, ty, span);
                    return DataBuf::empty();
                };
                self.lower_to_data(&Value::Int(n), inner, span)
            }
            Ty::Bitfield(name) => self.lower_bitfield(value, name, span),
            Ty::Enum(name) => self.lower_enum(value, name, span),
            Ty::Array(elem, n) => self.lower_array(value, elem, *n, span),
            Ty::Tuple(elems) => self.lower_tuple(value, elems, span),
            Ty::Struct(name) => self.lower_struct(value, name, span),
            // The Plan-4 pointer SEAM: emit a symbolic reference, never an
            // address (D-P3.7). Plan 4 resolves it to a fixup.
            Ty::Ptr(_) => self.lower_ptr(value, span),
            Ty::Poison => DataBuf::empty(),
        }
    }

    /// Lower a primitive: unwrap to an i128 (accepting `Int` or a `Typed`
    /// wrapping one), emission-range-check it against the primitive's natural
    /// range, and emit one [`Cell::Scalar`]. The cell is emitted even on a range
    /// failure (best-effort) so a struct's total size still lines up with its
    /// layout — the diagnostic is what matters.
    fn lower_prim(&mut self, value: &Value, width: u8, signed: bool, le: bool, span: Span) -> DataBuf {
        let ty = Ty::Prim { width, signed, le };
        let Some(n) = value.as_stored_int() else {
            self.emit_expected_int(value, &ty, span);
            return DataBuf::empty();
        };
        // `le` never affects the accepted range (R-T0.1) — identical bounds to
        // the non-`le` primitive of the same (width, signed).
        let (lo, hi) = prim_bounds(width, signed);
        self.emit_range_check(n, lo, hi, &ty.describe(), span);
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: n, width, signed, le });
        buf
    }

    /// Lower a `fixed<I,F>`: emit the STORED scaled int (`x·2^F`) as a SIGNED
    /// scalar, range-checked against the signed `I+F`-bit store. Enforces the
    /// [`Cell::Scalar`] width invariant (∈ {1,2,4}): a non-whole-byte fixed
    /// diagnoses (via the shared [`fixed_byte_size`](Self::fixed_byte_size), the
    /// same check `size_of_ty` uses), and a fixed wider than 4 bytes (e.g.
    /// `fixed<32,32>` = 8 bytes) is rejected — the 68k `.b/.w/.l` directives are
    /// 1/2/4 bytes, and wide fixed types are multiply intermediates you `rescale`
    /// down before storing.
    fn lower_fixed(&mut self, value: &Value, i: u32, f: u32, span: Span) -> DataBuf {
        let Some(n) = value.as_stored_int() else {
            self.emit_expected_int(value, &Ty::Fixed { i, f }, span);
            return DataBuf::empty();
        };
        // `fixed_width_bits` guards the degenerate 0 / ≥128-bit widths.
        let Some(bits) = self.fixed_width_bits(i, f, span) else {
            return DataBuf::empty();
        };
        // Shared sizing (emits BOTH the "not a whole number of bytes" and the
        // ">4 bytes, too wide" diagnostics identically to `size_of_ty`, so layout
        // and emission cannot disagree — T8 review, Minor 2).
        let width_bytes = self.fixed_byte_size(i, f, span);
        if bits % 8 != 0 {
            // A partial-byte fixed cannot be emitted as a scalar; the whole-byte
            // diagnostic already fired in `fixed_byte_size`.
            return DataBuf::empty();
        }
        if width_bytes > 4 {
            // Too-wide diagnostic already fired in `fixed_byte_size`; just bail.
            return DataBuf::empty();
        }
        let lo = -(1i128 << (bits - 1));
        let hi = (1i128 << (bits - 1)) - 1;
        self.emit_range_check(n, lo, hi, &Ty::Fixed { i, f }.describe(), span);
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: n, width: width_bytes as u8, signed: true, le: false });
        buf
    }

    /// Lower a bitfield value: it is the already-packed repr integer (from T4's
    /// [`eval_bitfield_lit`](Self::eval_bitfield_lit)); emit it as an unsigned
    /// scalar of the repr's byte width.
    fn lower_bitfield(&mut self, value: &Value, name: &str, span: Span) -> DataBuf {
        let layout = self.layout_of_bitfield(name, span);
        let Some(n) = value.as_stored_int() else {
            self.error(
                span,
                format!("bitfield {name} value must be an integer, got {}", value.type_name()),
            );
            return DataBuf::empty();
        };
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: n, width: (layout.repr_bits / 8) as u8, signed: false, le: false });
        buf
    }

    /// Lower an enum value: emit the ACTIVE variant's discriminant (computed as
    /// in T4's cast) as a scalar of the enum's repr width/signedness.
    fn lower_enum(&mut self, value: &Value, name: &str, span: Span) -> DataBuf {
        let Value::Enum { variant, .. } = value else {
            self.error(span, format!("expected a {name} enum value, got {}", value.type_name()));
            return DataBuf::empty();
        };
        let Some(decl) = self.enums.get(name).copied() else {
            return DataBuf::empty();
        };
        let values = self.enum_variant_values(decl);
        let disc = decl
            .variants
            .iter()
            .position(|v| &v.name == variant)
            .and_then(|idx| values[idx]);
        // A `None` discriminant is a variant whose value expr already errored
        // (reported by `enum_variant_values`) — stay silent.
        let Some(disc) = disc else {
            return DataBuf::empty();
        };
        let (width, signed) = self.enum_repr_prim(name, span);
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: disc, width, signed, le: false });
        buf
    }

    /// The (width, signed) of an enum's repr, defaulting to `u8` when absent or
    /// non-primitive (matching [`size_of_ty`](Self::size_of_ty)'s enum sizing).
    fn enum_repr_prim(&mut self, name: &str, _span: Span) -> (u8, bool) {
        let repr = self.enums.get(name).copied().and_then(|d| d.repr.as_ref());
        if let Some(repr) = repr {
            if let Ty::Prim { width, signed, .. } = self.resolve_type(repr) {
                return (width, signed);
            }
        }
        (1, false)
    }

    /// Lower a `[T; n]` array: the value must be a [`Value::Array`] of length
    /// exactly `n` (a mismatch is diagnosed); lower each element against `elem`
    /// and concatenate.
    ///
    /// A [`Value::Str`] is accepted here too, but ONLY when `elem` is a 1-byte
    /// primitive (`u8`/`i8`) — `data Msg: [u8;5] = "HELLO"` (lexical gaps, Task
    /// 4). This is the byte-context reading of a string literal; it never
    /// touches [`lower_ptr`], which owns the separate pointer-context reading
    /// (a string names a SYMBOL there, unchanged). A string against any other
    /// element type (`[u16;N]`, …) falls through to the ordinary
    /// "expected an array" mismatch below.
    fn lower_array(&mut self, value: &Value, elem: &Ty, n: usize, span: Span) -> DataBuf {
        if let Value::Str(s) = value {
            if matches!(elem, Ty::Prim { width: 1, .. }) {
                return self.lower_str_as_byte_array(s, n, span);
            }
        }
        let Value::Array(elems) = value else {
            self.error(span, format!("expected an array of length {n}, got {}", value.type_name()));
            return DataBuf::empty();
        };
        if elems.len() != n {
            self.error(
                span,
                format!("array length mismatch: expected {n} element(s), got {}", elems.len()),
            );
        }
        let mut buf = DataBuf::empty();
        for el in elems {
            buf = DataBuf::concat(buf, self.lower_to_data(el, elem, span));
        }
        buf
    }

    /// Lower a string literal against a `[u8;n]`/`[i8;n]` array type (lexical
    /// gaps, Task 4): raw ASCII bytes, exact-length checked the SAME way an
    /// ordinary array literal is (`lower_array`'s length-mismatch diagnostic,
    /// same wording) — the author sizes `n` to include any terminator; there
    /// is no implicit trailing 0.
    fn lower_str_as_byte_array(&mut self, s: &str, n: usize, span: Span) -> DataBuf {
        let Some(bytes) = self.ascii_bytes(s, span) else {
            return DataBuf::empty();
        };
        if bytes.len() != n {
            self.error(
                span,
                format!("array length mismatch: expected {n} element(s), got {}", bytes.len()),
            );
        }
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(bytes));
        buf
    }

    /// Convert a string's characters to raw ASCII bytes (T7/lexical gaps, Task
    /// 4 — strings default to RAW ASCII in data position). A non-ASCII
    /// character (any codepoint > 127) is `[emit.non-ascii]`, naming the
    /// offending character, mirroring the char-literal ASCII-only rule (Task
    /// 3); scanning stops at the first one and the whole conversion poisons to
    /// `None`.
    pub(super) fn ascii_bytes(&mut self, s: &str, span: Span) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_ascii() {
                out.push(ch as u8);
            } else {
                self.error(
                    span,
                    format!(
                        "[emit.non-ascii] string byte must be ASCII; {ch:?} is not — use a numeric literal or an escape"
                    ),
                );
                return None;
            }
        }
        Some(out)
    }

    /// Lower a tuple: the value must be a [`Value::Tuple`] of matching arity;
    /// lower each element against its corresponding tuple type and concatenate.
    fn lower_tuple(&mut self, value: &Value, elem_tys: &[Ty], span: Span) -> DataBuf {
        let Value::Tuple(vals) = value else {
            self.error(span, format!("expected a tuple, got {}", value.type_name()));
            return DataBuf::empty();
        };
        if vals.len() != elem_tys.len() {
            self.error(
                span,
                format!("tuple arity mismatch: expected {} element(s), got {}", elem_tys.len(), vals.len()),
            );
        }
        let mut buf = DataBuf::empty();
        for (v, t) in vals.iter().zip(elem_tys.iter()) {
            buf = DataBuf::concat(buf, self.lower_to_data(v, t, span));
        }
        buf
    }

    /// Lower a struct: the value must be a [`Value::Struct`]. A struct LITERAL
    /// checked against `name` (via
    /// [`eval_checked_struct_lit`](Self::eval_checked_struct_lit)) is already
    /// name/default-checked — its `fields` are GUARANTEED to contain exactly one
    /// entry per declared field, so the shape check below is a no-op for it.
    /// A value that bypassed that check (chiefly `import`, Spec 2 Plan 5 — Task
    /// 2, D-P5.4, whose generic `Value::Struct` comes straight from a JSON/TOML
    /// object with no guarantee its keys match `name`'s fields at all) is
    /// checked HERE instead: a declared field missing from `fields` is
    /// `[struct.missing-field]`, and a `fields` entry naming no declared field is
    /// `[struct.unknown-field]` — both diagnostics, not a silent mis-size.
    /// (A literal is a no-op here only when lowered against its OWN type; a
    /// mismatched explicit annotation — `data P: Point = Other{...}` — is a
    /// genuine shape mismatch and deliberately trips the same check.)
    ///
    /// Walk the struct's [`Layout`](crate::layout::Layout) fields in declaration
    /// order, lower each field's value against its field type, and concatenate —
    /// so the emitted cells fall at the layout's offsets.
    fn lower_struct(&mut self, value: &Value, name: &str, span: Span) -> DataBuf {
        let Value::Struct { fields, .. } = value else {
            self.error(span, format!("expected a {name} struct value, got {}", value.type_name()));
            return DataBuf::empty();
        };
        // `layout_of_struct` returns an owned `Layout` (not borrowed from self),
        // so the field-lowering loop below is free to `&mut self`.
        let layout = self.layout_of_struct(name, span);
        // The shape check (see the doc comment above): every declared field must
        // be present, and every provided field must be declared. Reported before
        // lowering so a shape mismatch surfaces even if every matched field
        // would otherwise lower cleanly.
        for fl in &layout.fields {
            if !fields.iter().any(|(n, _)| n == &fl.name) {
                self.error(
                    span,
                    format!("[struct.missing-field] struct {name}: field `{}` was not provided", fl.name),
                );
            }
        }
        for (fname, _) in fields {
            if !layout.fields.iter().any(|fl| &fl.name == fname) {
                self.error(span, format!("[struct.unknown-field] struct {name} has no field `{fname}`"));
            }
        }
        let mut buf = DataBuf::empty();
        for fl in &layout.fields {
            // A field absent from the value (a missing no-default field the
            // checked literal filled with Poison, or — now diagnosed above — an
            // import shape mismatch) lowers silently to nothing past the
            // diagnostic already reported.
            if let Some((_, v)) = fields.iter().find(|(n, _)| n == &fl.name) {
                buf = DataBuf::concat(buf, self.lower_to_data(v, &fl.ty, span));
            }
        }
        buf
    }

    /// Lower a pointer field (the Plan-4 SEAM): extract a symbol NAME from the
    /// value — a [`Value::FnRef`] (a bare `comptime fn` name) or a
    /// [`Value::Str`] naming a symbol — and emit a [`Cell::SymRef`] of width 4
    /// (D-P3.7). The address is NOT resolved (Plan 4). If no name can be
    /// extracted, diagnose and emit a placeholder `SymRef` so the 4-byte slot is
    /// still accounted for.
    fn lower_ptr(&mut self, value: &Value, span: Span) -> DataBuf {
        let name = match value {
            Value::FnRef(n) => Some(n.clone()),
            Value::Str(s) => Some(s.clone()),
            // A first-class LABEL value (D-PP.3) — a bareword/dotted proc/data
            // reference — lowers to the SAME `Cell::SymRef` the string form
            // does, so `code: init` == `code: "init"` byte-for-byte.
            Value::Label(n) => Some(n.clone()),
            // Poison is filtered by `lower_to_data`, so `_` is a genuine non-ref.
            _ => None,
        };
        let mut buf = DataBuf::empty();
        match name {
            // A plain absolute pointer (NOT windowed — that is `winptr(sym)`).
            Some(name) => buf.push(Cell::SymRef { name, width: 4, windowed: false }),
            // An INT literal in a pointer slot folds to a plain width-4 absolute
            // VALUE cell — no fixup (T3 P3). This is the sparse-table null idiom:
            // a `0` is an unused/empty pointer slot (`SfxTable`'s 126 gap cells),
            // written directly rather than through a symbol. A non-zero int folds
            // the same way (a literal absolute address), so a stray nonzero can't
            // silently do something other than what `0` does. Big-endian, matching
            // the `Abs32Be` byte layout a `SymRef` in this slot would resolve to.
            None if matches!(value, Value::Int(_)) => {
                if let Value::Int(n) = value {
                    // The folded cell is a 4-byte UNSIGNED absolute address, so
                    // range-check against `0..=u32::MAX` before pushing — an
                    // out-of-range int must NOT silently truncate to its low 4
                    // bytes (byte-identical to a `0` null). Mirrors the scalar
                    // path (`lower_prim`)'s `emit_range_check`; the cell is still
                    // emitted (best-effort) so the table's size lines up.
                    self.emit_range_check(*n, 0, u32::MAX as i128, "*u8 (absolute pointer cell)", span);
                    buf.push(Cell::Scalar { value: *n, width: 4, signed: false, le: false });
                }
            }
            None => {
                self.error(
                    span,
                    format!("pointer field needs a symbol reference, got {}", value.type_name()),
                );
                buf.push(Cell::SymRef { name: "<unresolved>".to_string(), width: 4, windowed: false });
            }
        }
        buf
    }

    /// Lower a link-time value (a [`Value::LinkExpr`]) into a data cell.
    ///
    /// Two paths, deliberately kept distinct (R7m.4/R7m.5):
    ///
    /// - A PLAIN `LinkExpr(Sym(anchor))` — a bare symbol reference, chiefly a
    ///   provisional `here()` (the item's own label) — keeps its byte-proven
    ///   D-H.3 lowering UNCHANGED: a `Cell::SymRef` of the field's declared width
    ///   (`self_ref_width`), an ADDRESS cell the linker fixes up to the anchor's
    ///   post-relaxation VMA. Width 1 stays an error (no 8-bit absolute address
    ///   kind — a `SymRef` is a pointer). This path is frozen (winptr/SymRef
    ///   byte-identity, R7m.5).
    ///
    /// - An ARITHMETICALLY-combined link value (a non-`Sym` residual tree —
    ///   `here() + 2`, `bankid(L)`, …) now EMITS as a general link-expr data cell
    ///   (`Cell::Expr`, S2-D13f un-deferred, D7.3/R7m.4) — REPLACING the old
    ///   `[here.provisional]` arithmetic-then-emit refusal (here-fix design case
    ///   5). It is a VALUE, not an address: width 1 is REQUIRED (aeon's `ds_bank`
    ///   is one byte), and the linker range-checks the fold against an UNSIGNED
    ///   window, never an address range.
    fn lower_link_expr(
        &mut self,
        expr: &sigil_ir::expr::Expr,
        ty: &Ty,
        span: Span,
    ) -> DataBuf {
        // A bare symbol keeps the frozen SymRef address lowering (byte-proven).
        if let sigil_ir::expr::Expr::Sym(name) = expr {
            let Some(width) = self.self_ref_width(ty, span) else {
                return DataBuf::empty();
            };
            if width == 1 {
                self.error(
                    span,
                    "[here.provisional] a provisional `here()` cannot emit into a 1-byte field — \
                     an absolute address needs 2 or 4 bytes (u16/u32 or a pointer)"
                        .to_string(),
                );
                return DataBuf::empty();
            }
            let mut buf = DataBuf::empty();
            buf.push(Cell::SymRef { name: name.clone(), width, windowed: false });
            return buf;
        }
        // A residual arithmetic tree: the general link-expr VALUE cell. Width is
        // the declared cell width exactly like a `Cell::Scalar` (§ w ∈ {1,2,4}),
        // reusing `self_ref_width` — but width 1 is ALLOWED here (a value, not an
        // address), so we bypass the SymRef-only 1-byte refusal above.
        let Some(width) = self.self_ref_width(ty, span) else {
            return DataBuf::empty();
        };
        // `u16le` (R-T0.1): the `le` override survives through a residual
        // arithmetic tree exactly like it survives a plain scalar — a
        // `data B: u16le = bankid("L")` in a 68k section must still select
        // `Value16Le`, not the section's default `Value16Be`.
        let le = self.self_ref_le(ty, span);
        let mut buf = DataBuf::empty();
        buf.push(Cell::Expr { expr: expr.clone(), width, le });
        buf
    }

    /// The byte width a provisional `here()` `Cell::SymRef` takes in a field of
    /// type `ty` (D-H.3): a pointer is 4; a primitive is its declared width
    /// (through newtype/refined to the underlying primitive). Any other field type
    /// cannot hold an absolute address — a loud error yielding `None`.
    fn self_ref_width(&mut self, ty: &Ty, span: Span) -> Option<u8> {
        match ty {
            Ty::Ptr(_) => Some(4),
            Ty::Prim { width, .. } => Some(*width),
            Ty::Newtype(_) | Ty::Refined { .. } => {
                let underlying = self.effective_underlying(ty, span);
                if matches!(underlying, Ty::Poison) {
                    return None;
                }
                self.self_ref_width(&underlying, span)
            }
            other => {
                self.error(
                    span,
                    format!(
                        "[here.provisional] a provisional `here()` cannot emit into a {} field — \
                         it needs a u16/u32 or a pointer field to hold its link-time address",
                        other.describe()
                    ),
                );
                None
            }
        }
    }

    /// The `u16le` override (R-T0.1) for a `Cell::Expr` field of type `ty`,
    /// mirroring [`self_ref_width`](Self::self_ref_width)'s traversal through
    /// `Newtype`/`Refined` down to the underlying primitive. A pointer never
    /// carries the flag (always `false`). `span` is only used if a
    /// `Newtype`/`Refined` traversal needs to re-report a cyclic-type error;
    /// in practice `self_ref_width` (called first, same `ty`/`span`) already
    /// reports and bails on any such error, so this path is silent in
    /// practice — but stays total rather than assuming that ordering.
    fn self_ref_le(&mut self, ty: &Ty, span: Span) -> bool {
        match ty {
            Ty::Prim { le, .. } => *le,
            Ty::Newtype(_) | Ty::Refined { .. } => {
                match self.effective_underlying(ty, span) {
                    Ty::Prim { le, .. } => le,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// The emission range-check (D-P3.3): if `n` falls outside `lo..=hi`, report
    /// `[emit.out-of-range]` naming the value and the type. The caller still
    /// emits its (best-effort) cell so downstream sizes line up.
    fn emit_range_check(&mut self, n: i128, lo: i128, hi: i128, ty_desc: &str, span: Span) {
        if n < lo || n > hi {
            self.error(span, format!("[emit.out-of-range] {n} does not fit {ty_desc} ({lo}..={hi})"));
        }
    }

    /// Report that emission expected an integer value for a scalar-typed field.
    fn emit_expected_int(&mut self, value: &Value, ty: &Ty, span: Span) {
        self.error(
            span,
            format!("[emit.type] expected an integer for {}, got {}", ty.describe(), value.type_name()),
        );
    }

    /// Resolve the `data` item named `name` to a checked [`DataBuf`], memoizing
    /// the result (T7). Mirrors [`resolve_const`](Self::resolve_const)'s lazy
    /// memo; data items cannot reference each other as values in Plan 3, so no
    /// cycle machinery is needed. Callers must only invoke this for a `name`
    /// known to be in `self.datas`.
    pub(crate) fn resolve_data(&mut self, name: &str, _ref_span: Span) -> DataBuf {
        if let Some(b) = self.data_memo.get(name) {
            return b.clone();
        }
        // Copy the `&'a DataDecl` out so `self` is free to be mutated across the
        // recursive eval/lower below.
        let decl: &'a ast::DataDecl =
            self.datas.get(name).copied().expect("caller ensures the data item exists");
        let mut env = Env::new();
        let value = self.eval_expr(&decl.value, &mut env);
        let buf = self.lower_data_value(decl, value);
        self.data_memo.insert(name.to_string(), buf.clone());
        buf
    }

    /// Turn a data item's evaluated `value` into its [`DataBuf`] (T7):
    /// - a [`Poison`](Value::Poison) value is already-reported → empty, silent;
    /// - a [`Value::Data`] (from `byte`/`bytes`/`++`) is ALREADY the checked,
    ///   CPU-neutral buffer — return it directly (no target type needed);
    /// - otherwise determine the target [`Ty`] from the explicit annotation, or
    ///   infer it from a struct-literal initializer that names its type (§4.5),
    ///   and [`lower_to_data`](Self::lower_to_data) against it. A missing,
    ///   uninferable type is a diagnostic.
    fn lower_data_value(&mut self, decl: &'a ast::DataDecl, value: Value) -> DataBuf {
        if matches!(value, Value::Poison) {
            return DataBuf::empty();
        }
        if let Value::Data(buf) = value {
            // A `Data`-monoid initializer (byte/bytes/++) is already lowered, but
            // an explicit annotation still pins the size — a `data D: [u8;3] =
            // bytes([1,2])` that produces the wrong byte count is a mismatch.
            if let Some(t) = &decl.ty {
                let ty = self.resolve_type(t);
                if !matches!(ty, Ty::Poison) {
                    let declared = self.size_of_ty(&ty, decl.span);
                    if declared != buf.size {
                        self.error(
                            decl.span,
                            format!(
                                "[emit.size-mismatch] data `{}`: declared type is {declared} byte(s), initializer produced {}",
                                decl.name, buf.size
                            ),
                        );
                    }
                }
            }
            return buf;
        }
        let ty = match &decl.ty {
            Some(t) => self.resolve_type(t),
            None => match &decl.value {
                // The initializer names its type — infer it (§4.5).
                ast::Expr::StructLit { ty, .. } => {
                    self.resolve_type(&ast::Type::Named(ty.clone()))
                }
                _ => {
                    self.error(
                        decl.span,
                        format!(
                            "data `{}` needs a type annotation (its initializer does not name a type)",
                            decl.name
                        ),
                    );
                    return DataBuf::empty();
                }
            },
        };
        self.lower_to_data(&value, &ty, decl.span)
    }
}
