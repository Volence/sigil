//! The `.emp` types & layout engine (Spec 2, Plan 3 — T2): the resolved [`Ty`]
//! model, the byte-[`size_of_ty`](Evaluator::size_of_ty) primitive, struct
//! [`layout_of_struct`](Evaluator::layout_of_struct), and the general-purpose
//! inclusive-bounds [`check_in_range`](Evaluator::check_in_range) helper.
//!
//! This module contributes `impl Evaluator` method groups (the crate's
//! multi-file-impl style). The layout machinery mirrors the const evaluator's
//! lazy, memoized, cycle-detected [`resolve_const`](crate::eval) pattern:
//! [`struct_layout_memo`](Evaluator) caches results and a
//! [`layout_in_progress`](Evaluator) stack names cyclic (infinite-size)
//! layouts instead of overflowing the stack.
//!
//! **Scope (T2):** raw offsets + total size + the sizing primitives, and
//! `check_in_range`. The `(size: N)` verification, `@offset` field assertions,
//! the `[layout.odd-field]` warning, `sizeof`/`offsetof` builtin *evaluation*,
//! and the field-mismatch diff are all T3 — layered on top of what is here.
use crate::ast;
use crate::eval::{Env, Evaluator};
use crate::value::Value;
use sigil_span::{Diagnostic, Span};

/// A resolved `.emp` type (D-P3.1): the semantic counterpart of a syntactic
/// [`ast::Type`], with named types resolved against the file's type tables and
/// array lengths / refinement bounds already evaluated to constants.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    /// A primitive integer of `width` bytes (∈ {1, 2, 4}), signed or unsigned.
    Prim {
        /// Byte width: 1, 2, or 4.
        width: u8,
        /// Whether the primitive is signed (`i*`) or unsigned (`u*`).
        signed: bool,
    },
    /// A pointer type `*T`. Always 4 bytes (D-P3.7, the 68k Abs32 default) and,
    /// crucially, sizing it does NOT recurse into the pointee's layout — so a
    /// struct may reference itself by pointer without an infinite-size cycle.
    Ptr(Box<Ty>),
    /// A fixed-length array `[T; n]`.
    Array(Box<Ty>, usize),
    /// A tuple `(A, B, ...)`.
    Tuple(Vec<Ty>),
    /// A named `struct`.
    Struct(String),
    /// A named `bitfield`.
    Bitfield(String),
    /// A named `enum`.
    Enum(String),
    /// A named `newtype`.
    Newtype(String),
    /// A fixed-point type `fixed<I, F>`.
    Fixed {
        /// Integer-part bit width.
        i: u32,
        /// Fraction-part bit width.
        f: u32,
    },
    /// A refined type `T where LO..HI` with INCLUSIVE bounds on BOTH ends
    /// (D-P3.8): `lo <= value <= hi`.
    Refined {
        /// The refined underlying type.
        inner: Box<Ty>,
        /// Inclusive lower bound.
        lo: i128,
        /// Inclusive upper bound.
        hi: i128,
    },
    /// An error was already reported while resolving this type; carries no size
    /// (0) and suppresses further diagnostics, mirroring [`Value::Poison`].
    Poison,
}

impl Ty {
    /// A short human-readable name for diagnostics (T3's `offsetof: {ty} is not
    /// a struct`, etc). Not a full pretty-printer — just enough to name what
    /// went wrong.
    pub(crate) fn describe(&self) -> String {
        match self {
            Ty::Prim { width, signed } => {
                format!("{}{}", if *signed { "i" } else { "u" }, u32::from(*width) * 8)
            }
            Ty::Ptr(inner) => format!("*{}", inner.describe()),
            Ty::Array(inner, n) => format!("[{}; {n}]", inner.describe()),
            Ty::Tuple(elems) => {
                format!("({})", elems.iter().map(Ty::describe).collect::<Vec<_>>().join(", "))
            }
            Ty::Struct(name) => name.clone(),
            Ty::Bitfield(name) => name.clone(),
            Ty::Enum(name) => name.clone(),
            Ty::Newtype(name) => name.clone(),
            Ty::Fixed { i, f } => format!("fixed<{i},{f}>"),
            Ty::Refined { inner, lo, hi } => format!("{} where {lo}..{hi}", inner.describe()),
            Ty::Poison => "<poisoned type>".to_string(),
        }
    }
}

/// A resolved struct byte layout: total size and per-field placement.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Total byte size (the next-free offset after the last field).
    pub size: usize,
    /// The fields, in declaration order.
    pub fields: Vec<FieldLayout>,
}

/// One field's placement within a [`Layout`].
#[derive(Debug, Clone, PartialEq)]
pub struct FieldLayout {
    /// The field's name.
    pub name: String,
    /// The field's byte offset from the start of the struct.
    pub offset: usize,
    /// The field's resolved type.
    pub ty: Ty,
    /// The field's byte size.
    pub size: usize,
}

impl<'a> Evaluator<'a> {
    /// Resolve a syntactic [`ast::Type`] to a semantic [`Ty`], reporting (and
    /// returning [`Ty::Poison`] for) any error along the way.
    ///
    /// A single-segment name resolves as: a primitive (`u8`/`i8`/…/`u32`/`i32`),
    /// then a name in the struct/bitfield/enum/newtype table, else an unknown
    /// type. Comptime-only types (`string`/`Width`/`Operand`, which appear only
    /// in comptime-enum payloads) are NOT data-layout types — resolving one as a
    /// *data* type is an unknown-type error here; those are exercised in T6.
    ///
    /// Every diagnostic here is anchored at a precise span carried by the
    /// syntax itself (a `path.span` for a name, the length expression's span for
    /// an array bound), so no caller-supplied fallback span is needed.
    pub(crate) fn resolve_type(&mut self, t: &ast::Type) -> Ty {
        match t {
            ast::Type::Named(path) => {
                if path.segments.len() != 1 {
                    // Multi-segment paths (module-qualified names) are not
                    // resolvable as data-layout types yet.
                    let full = path.segments.join(".");
                    self.error(path.span, format!("unknown type: {full}"));
                    return Ty::Poison;
                }
                let name = path.segments[0].as_str();
                match name {
                    "u8" => Ty::Prim { width: 1, signed: false },
                    "i8" => Ty::Prim { width: 1, signed: true },
                    "u16" => Ty::Prim { width: 2, signed: false },
                    "i16" => Ty::Prim { width: 2, signed: true },
                    "u32" => Ty::Prim { width: 4, signed: false },
                    "i32" => Ty::Prim { width: 4, signed: true },
                    _ if self.structs.contains_key(name) => Ty::Struct(name.to_string()),
                    _ if self.bitfields.contains_key(name) => Ty::Bitfield(name.to_string()),
                    _ if self.enums.contains_key(name) => Ty::Enum(name.to_string()),
                    _ if self.newtypes.contains_key(name) => Ty::Newtype(name.to_string()),
                    _ => {
                        self.error(path.span, format!("unknown type: {name}"));
                        Ty::Poison
                    }
                }
            }
            ast::Type::Ptr(inner) => Ty::Ptr(Box::new(self.resolve_type(inner))),
            ast::Type::Array(inner, len_expr) => {
                let inner_ty = self.resolve_type(inner);
                let len_span = crate::parser::expr_span(len_expr);
                match self.eval_const_index(len_expr) {
                    // `n >= 0` and fits `usize`: a real array length. A huge but
                    // in-`i128` length must NOT silently truncate via `as usize`.
                    Some(n) if n >= 0 => match usize::try_from(n) {
                        Ok(len) => Ty::Array(Box::new(inner_ty), len),
                        Err(_) => {
                            self.error(len_span, format!("array length {n} too large"));
                            Ty::Poison
                        }
                    },
                    Some(n) => {
                        self.error(len_span, format!("array length must be non-negative, got {n}"));
                        Ty::Poison
                    }
                    None => {
                        // A non-int length (or an already-reported error) —
                        // the diagnostic is emitted by `eval_const_index`.
                        Ty::Poison
                    }
                }
            }
            ast::Type::Tuple(elems) => {
                Ty::Tuple(elems.iter().map(|e| self.resolve_type(e)).collect())
            }
            ast::Type::Fixed { i, f } => Ty::Fixed { i: *i, f: *f },
            ast::Type::Refined(inner, lo_expr, hi_expr) => {
                let inner_ty = self.resolve_type(inner);
                let lo = self.eval_const_index(lo_expr);
                let hi = self.eval_const_index(hi_expr);
                match (lo, hi) {
                    (Some(lo), Some(hi)) => {
                        Ty::Refined { inner: Box::new(inner_ty), lo, hi }
                    }
                    // A non-int bound reports via `eval_const_index`.
                    _ => Ty::Poison,
                }
            }
        }
    }

    /// Resolve a [`Ty`] down to the name of the struct it names, for
    /// `offsetof(T, field)` (T3). Bottoms out through the erasing wrappers —
    /// [`Ty::Refined`] and [`Ty::Newtype`] — so `offsetof` works on a refined
    /// or newtype-wrapped struct type, not just a bare `Ty::Struct`. Returns
    /// `None` for anything that doesn't bottom out at a struct (including
    /// `Ty::Poison`, which stays silent — the poisoning resolve already
    /// reported); the caller diagnoses the `None` case with the *original*
    /// resolved type so the message names what was actually written.
    pub(crate) fn struct_name_for_offsetof(&mut self, ty: &Ty) -> Option<String> {
        match ty {
            Ty::Struct(name) => Some(name.clone()),
            Ty::Refined { inner, .. } => self.struct_name_for_offsetof(inner),
            Ty::Newtype(name) => {
                let decl = self.newtypes.get(name.as_str()).copied()?;
                let underlying = self.resolve_type(&decl.underlying);
                self.struct_name_for_offsetof(&underlying)
            }
            _ => None,
        }
    }

    /// Evaluate a comptime expression that must reduce to an integer (an array
    /// length or a refinement bound), returning the `i128`. A non-int result is
    /// a diagnostic (returning `None`); an already-`Poison` result is silent
    /// (also `None`), so a reported sub-error does not double-report.
    fn eval_const_index(&mut self, expr: &ast::Expr) -> Option<i128> {
        let mut env = Env::new();
        match self.eval_expr(expr, &mut env) {
            Value::Int(n) => Some(n),
            Value::Poison => None,
            other => {
                self.error(
                    crate::parser::expr_span(expr),
                    format!("expected an integer, got {}", other.type_name()),
                );
                None
            }
        }
    }

    /// The byte size of a resolved [`Ty`] (D-P3.7). Struct sizing is memoized
    /// and cycle-detected via [`layout_of_struct`](Self::layout_of_struct);
    /// other named types (newtype/enum/bitfield) forward to their underlying /
    /// repr, which is itself struct-memoized when relevant.
    pub(crate) fn size_of_ty(&mut self, ty: &Ty, span: Span) -> usize {
        match ty {
            Ty::Prim { width, .. } => *width as usize,
            // A pointer is 4 bytes and does NOT recurse into the pointee — this
            // is what makes by-pointer self-reference finite (D-P3.7).
            Ty::Ptr(_) => 4,
            Ty::Array(inner, n) => {
                // `elem_size * n` can overflow `usize`; diagnose rather than
                // panic (debug) or wrap (release).
                let elem = self.size_of_ty(inner, span);
                match elem.checked_mul(*n) {
                    Some(total) => total,
                    None => {
                        self.error(span, "type too large to size");
                        0
                    }
                }
            }
            Ty::Tuple(elems) => {
                // Sum with overflow checking; a wrapping sum would corrupt every
                // downstream offset.
                let mut total = 0usize;
                for e in elems {
                    let s = self.size_of_ty(e, span);
                    match total.checked_add(s) {
                        Some(t) => total = t,
                        None => {
                            self.error(span, "type too large to size");
                            return 0;
                        }
                    }
                }
                total
            }
            Ty::Fixed { i, f } => {
                // `i` and `f` are each a full `u32` per the parser, so `i + f`
                // can overflow `u32` — widen the add.
                let Some(bits) = i.checked_add(*f) else {
                    self.error(span, format!("fixed<{i},{f}> is too large to size"));
                    return 0;
                };
                if bits % 8 != 0 {
                    self.error(
                        span,
                        format!("fixed<{i},{f}> is not a whole number of bytes"),
                    );
                    // Best-effort ceil so callers still get a plausible size.
                    return bits.div_ceil(8) as usize;
                }
                (bits / 8) as usize
            }
            Ty::Refined { inner, .. } => self.size_of_ty(inner, span),
            // Newtype sizing recurses through the underlying type, which may
            // form a `newtype A = B; newtype B = A` cycle that never passes
            // through a `Ty::Struct` hop — so it needs its own cycle guard.
            Ty::Newtype(name) => self.size_of_newtype(name, span),
            Ty::Enum(name) => {
                // A plain enum always has a repr; default to `u8` (1) if absent.
                let decl = self.enums.get(name.as_str()).copied();
                match decl.and_then(|d| d.repr.as_ref()) {
                    Some(repr) => {
                        let repr_ty = self.resolve_type(repr);
                        self.size_of_ty(&repr_ty, span)
                    }
                    None => 1,
                }
            }
            Ty::Bitfield(name) => {
                let decl = self.bitfields.get(name.as_str()).copied();
                match decl {
                    Some(d) => {
                        let repr_ty = self.resolve_type(&d.repr);
                        self.size_of_ty(&repr_ty, span)
                    }
                    None => 0,
                }
            }
            Ty::Struct(name) => self.layout_of_struct(name, span).size,
            Ty::Poison => 0,
        }
    }

    /// Byte size of the newtype named `name`, cycle-detected against the shared
    /// [`layout_in_progress`](Evaluator) stack (which also carries in-flight
    /// struct names). A `newtype A = B; newtype B = A` cycle — or a
    /// newtype↔struct cycle that happens to close on a newtype hop — is reported
    /// as `cyclic type: A -> B -> A` and sized 0, instead of overflowing the
    /// native stack. Newtype sizes are not memoized (a deferred perf nit); the
    /// underlying is re-resolved each time, which is cheap and side-effect-free.
    fn size_of_newtype(&mut self, name: &str, span: Span) -> usize {
        if let Some(start) = self.layout_in_progress.iter().position(|n| n == name) {
            let mut chain: Vec<&str> =
                self.layout_in_progress[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            self.error(span, format!("cyclic type: {}", chain.join(" -> ")));
            return 0;
        }
        // Copy the `&'a NewtypeDecl` out so `self` is free to be mutated across
        // the recursive resolve/size below (as `resolve_const`/`layout_of_struct`).
        let Some(decl) = self.newtypes.get(name).copied() else {
            return 0;
        };
        self.layout_in_progress.push(name.to_string());
        let underlying = self.resolve_type(&decl.underlying);
        let size = self.size_of_ty(&underlying, span);
        self.layout_in_progress.pop();
        size
    }

    /// Compute (or fetch the memoized) byte layout of the struct named `name`,
    /// mirroring [`resolve_const`](crate::eval)'s memo + cycle machinery.
    ///
    /// Fields are placed in declaration order, each at the next free byte (no
    /// padding — `offset += size` per field), and the total `size` is the final
    /// offset. A struct that contains itself BY VALUE (directly or transitively)
    /// has infinite size: the cycle is reported as `cyclic struct layout: A ->
    /// B -> A`, and a zero-size Poisoned layout is memoized to stop the cascade.
    /// A by-POINTER self-reference is fine — [`size_of_ty`](Self::size_of_ty) on
    /// a [`Ty::Ptr`] returns 4 without laying out the pointee.
    pub(crate) fn layout_of_struct(&mut self, name: &str, span: Span) -> Layout {
        if let Some(l) = self.struct_layout_memo.get(name) {
            return l.clone();
        }
        if let Some(start) = self.layout_in_progress.iter().position(|n| n == name) {
            // The cycle is the in-progress slice from first entry to this repeat.
            let cycle: Vec<String> = self.layout_in_progress[start..].to_vec();
            let mut chain: Vec<&str> = cycle.iter().map(|s| s.as_str()).collect();
            chain.push(name);
            // One diagnostic names the whole chain — not one error per member.
            self.error(span, format!("cyclic struct layout: {}", chain.join(" -> ")));
            // Poison EVERY struct on the cycle, not just the repeated `name`.
            // Each other member is mid-layout on this same in-progress chain and
            // would otherwise be memoized — as a side effect of laying out the
            // entry struct — with a plausible-looking but WRONG layout (non-empty
            // fields, sizes computed over a poisoned-as-0 nested struct). A later
            // direct query for any cycle member (e.g. T3's per-struct `(size: N)`
            // verification over a shared `Evaluator`) must get the poison, not a
            // lie. Unlike `Value::Poison`, a `Layout` cannot self-propagate poison
            // through the field loop, so we seed the whole slice explicitly here.
            let poisoned = Layout { size: 0, fields: Vec::new() };
            for member in &cycle {
                self.struct_layout_memo.insert(member.clone(), poisoned.clone());
            }
            return poisoned;
        }
        // Copy the `&'a StructDecl` out of the index so its fields are borrowed
        // from the file (lifetime `'a`), leaving `self` free to be mutated
        // across the recursive resolve/size calls below (as `resolve_const`).
        let decl: &'a ast::StructDecl = match self.structs.get(name).copied() {
            Some(d) => d,
            None => {
                // Unreachable in practice: every caller (`resolve_type` →
                // `Ty::Struct`, and `layout_struct`) pre-checks `self.structs`,
                // so a `Ty::Struct(name)` names a real struct. Kept as a defined,
                // non-panicking fallback (zero-size) so a future refactor that
                // reaches here degrades gracefully rather than exploding.
                debug_assert!(false, "layout_of_struct called for unknown struct `{name}`");
                return Layout { size: 0, fields: Vec::new() };
            }
        };
        self.layout_in_progress.push(name.to_string());
        let mut offset = 0usize;
        let mut fields = Vec::with_capacity(decl.fields.len());
        for field in &decl.fields {
            let ty = self.resolve_type(&field.ty);
            let size = self.size_of_ty(&ty, field.span);
            fields.push(FieldLayout { name: field.name.clone(), offset, ty, size });
            // Offsets are bytes; a pathological struct could overflow `usize`.
            match offset.checked_add(size) {
                Some(o) => offset = o,
                None => {
                    self.error(field.span, "type too large to size");
                    offset = 0;
                }
            }
        }
        self.layout_in_progress.pop();
        // CRITICAL: a deeper recursive call may have closed a cycle back to
        // `name` and already memoized a poisoned (zero-size) layout for it. That
        // poison is the correct answer — do NOT overwrite it with the ordinary,
        // numerically-wrong layout this frame just computed over the truncated
        // (size-0) cyclic field. Unlike `Value::Poison`, a `Layout` does not
        // self-propagate through the field loop, so this guard is what makes
        // multi-hop / mutual struct cycles report a poisoned layout, not a lie.
        if let Some(existing) = self.struct_layout_memo.get(name) {
            return existing.clone();
        }
        let layout = Layout { size: offset, fields };
        // T3: struct layout checks. These run exactly once per struct, right
        // here on the freshly-computed RAW layout, before it is memoized — a
        // later query short-circuits at the top-of-function memo lookup and
        // never re-enters this block, so none of these re-fire. They are also
        // unreachable for any struct on a layout cycle: both early-return paths
        // above (the cycle-detection block and the CRITICAL re-check just
        // above) return before this point, so a cyclic struct's own diagnostic
        // is the only one it gets — no size-mismatch/odd-field pile-on.
        self.check_struct_size(name, decl, &layout);
        self.check_struct_offsets(name, decl, &layout);
        self.check_struct_odd_fields(name, decl, &layout);
        self.struct_layout_memo.insert(name.to_string(), layout.clone());
        layout
    }

    /// D-P3.9 `(size: N)` verification: if `decl` declares an explicit size,
    /// compare it to `layout`'s computed total. A mismatch is ONE diagnostic —
    /// the headline delta plus a field-by-field diff (name/offset/size for
    /// every field) — so the author can see exactly which field to fix instead
    /// of just "sizes disagree" (this replaces AS's `if X_len <> N / error`
    /// idiom, §4.3).
    fn check_struct_size(&mut self, name: &str, decl: &ast::StructDecl, layout: &Layout) {
        let Some(size_expr) = &decl.size else { return };
        let Some(declared) = self.eval_const_index(size_expr) else { return };
        let computed = layout.size as i128;
        if declared == computed {
            return;
        }
        let mut msg = format!("struct {name}: declared size {declared} but fields total {computed}");
        for f in &layout.fields {
            msg.push_str(&format!("\n  {} @{} ({} byte{})", f.name, f.offset, f.size, if f.size == 1 { "" } else { "s" }));
        }
        msg.push_str(&format!("\n  {computed} vs {declared} (off by {})", computed - declared));
        self.error(crate::parser::expr_span(size_expr), msg);
    }

    /// D-P3.9 `@offset` field assertions: for every field that carries an
    /// explicit `@ expr`, compare it to that field's computed offset within
    /// `layout`. A mismatch is one diagnostic per offending field.
    fn check_struct_offsets(&mut self, name: &str, decl: &ast::StructDecl, layout: &Layout) {
        for (field_decl, field_layout) in decl.fields.iter().zip(layout.fields.iter()) {
            let Some(offset_expr) = &field_decl.offset else { continue };
            let Some(asserted) = self.eval_const_index(offset_expr) else { continue };
            let computed = field_layout.offset as i128;
            if asserted != computed {
                self.error(
                    crate::parser::expr_span(offset_expr),
                    format!(
                        "struct {name}: field {} at offset {computed} but @offset asserts {asserted}",
                        field_layout.name
                    ),
                );
            }
        }
    }

    /// `[layout.odd-field]` (§4.3): a default-on WARNING (not an error — some
    /// Z80-side layouts are legitimately unaligned) for every word/long
    /// (2- or 4-byte) field that lands at an odd byte offset.
    fn check_struct_odd_fields(&mut self, name: &str, decl: &ast::StructDecl, layout: &Layout) {
        for (field_decl, field_layout) in decl.fields.iter().zip(layout.fields.iter()) {
            if matches!(field_layout.size, 2 | 4) && field_layout.offset % 2 == 1 {
                self.warn(
                    field_decl.span,
                    format!(
                        "[layout.odd-field] struct {name}: field {} ({}-byte) at odd offset {}",
                        field_layout.name, field_layout.size, field_layout.offset
                    ),
                );
            }
        }
    }

    /// Range membership with INCLUSIVE bounds on BOTH ends (D-P3.8): returns
    /// `true` iff `lo <= val <= hi`. On failure, push a spanned, interpolated
    /// diagnostic `{ctx}: {val} not in {lo}..{hi}` and return `false`.
    ///
    /// General-purpose: this backs refinements, bitfield-field widths, and enum
    /// casts (T4). `ctx` is a caller label, e.g. `"refinement out-of-range"`.
    pub(crate) fn check_in_range(
        &mut self,
        val: i128,
        lo: i128,
        hi: i128,
        span: Span,
        ctx: &str,
    ) -> bool {
        if lo <= val && val <= hi {
            true
        } else {
            self.error(span, format!("{ctx}: {val} not in {lo}..{hi}"));
            false
        }
    }
}

// ---- test-friendly entry points ---------------------------------------

/// Resolve `ty` against `file`'s type tables and return its byte size plus any
/// diagnostics — the layout analogue of [`eval_const`](crate::eval::eval_const).
///
/// Runs on the shared large-stack evaluation thread (see
/// [`run_on_eval_stack`](crate::eval::run_on_eval_stack)): an array length or
/// refinement bound can be a recursive comptime-fn call, so the layout entry
/// points inherit the same native-stack headroom as `eval_const`.
pub fn size_of_type(file: &ast::File, ty: &ast::Type) -> (usize, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        let span = file.module.span;
        let resolved = ev.resolve_type(ty);
        let size = ev.size_of_ty(&resolved, span);
        (size, ev.diags)
    })
}

/// Lay out the struct named `name` in `file`, returning its [`Layout`] (or
/// `None` if no such struct) plus any diagnostics. Runs on the shared
/// large-stack evaluation thread (see [`size_of_type`]).
pub fn layout_struct(file: &ast::File, name: &str) -> (Option<Layout>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        if !ev.structs.contains_key(name) {
            ev.error(file.module.span, format!("no struct named `{name}`"));
            return (None, ev.diags);
        }
        let layout = ev.layout_of_struct(name, file.module.span);
        (Some(layout), ev.diags)
    })
}

/// Lay out several structs through a SINGLE shared [`Evaluator`] (unlike
/// [`layout_struct`], which builds a fresh one per call), returning each query's
/// layout (`None` for an unknown struct name) plus the accumulated diagnostics.
///
/// This exposes the *shared-memo* behaviour T3 relies on: per-struct `(size: N)`
/// verification runs many `layout_of_struct` queries through one evaluator, so a
/// direct query for a "middle" struct of a cycle must return the poisoned layout
/// seeded when the cycle was first detected — not a stale, wrong finite layout.
pub fn layout_structs_shared(
    file: &ast::File,
    names: &[&str],
) -> (Vec<Option<Layout>>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        let mut out = Vec::with_capacity(names.len());
        for &name in names {
            if ev.structs.contains_key(name) {
                out.push(Some(ev.layout_of_struct(name, file.module.span)));
            } else {
                out.push(None);
            }
        }
        (out, ev.diags)
    })
}

/// Check `val` against the INCLUSIVE range `lo..=hi` (D-P3.8), returning the
/// result plus any diagnostic. A thin wrapper over
/// [`Evaluator::check_in_range`] for direct testing.
pub fn check_in_range(val: i128, lo: i128, hi: i128) -> (bool, Vec<Diagnostic>) {
    let mut ev = Evaluator::new();
    let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
    let ok = ev.check_in_range(val, lo, hi, span, "range check");
    (ok, ev.diags)
}
