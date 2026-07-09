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
use crate::value::{Cell, DataBuf, Value};
use sigil_span::{Diagnostic, Span};
use std::path::PathBuf;

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
        /// Explicit little-endian override (`u16le`, R-T0.1 / DSM.7). When
        /// `true`, emission ALWAYS uses little-endian byte order regardless of
        /// the section's CPU — the whole point of the keyword is a 68k-side
        /// section emitting bytes a Z80 consumer reads. Never affects the
        /// accepted value range (`check_value_fits_ty`/`prim_bounds` ignore it
        /// entirely). `false` for every other primitive keyword; YAGNI on a
        /// `u32le`/`u16be`-on-Z80 pair until a customer exists.
        le: bool,
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
            Ty::Prim { width, signed, le } => {
                format!(
                    "{}{}{}",
                    if *signed { "i" } else { "u" },
                    u32::from(*width) * 8,
                    if *le { "le" } else { "" }
                )
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

/// A resolved bitfield bit layout (T4): the repr's total bit width and each
/// field's placement.
#[derive(Debug, Clone, PartialEq)]
pub struct BitfieldLayout {
    /// The repr type's bit width (8, 16, or 32).
    pub repr_bits: u32,
    /// The fields, in declaration order.
    pub fields: Vec<BitfieldFieldLayout>,
}

/// One field's placement within a [`BitfieldLayout`].
#[derive(Debug, Clone, PartialEq)]
pub struct BitfieldFieldLayout {
    /// The field's name.
    pub name: String,
    /// The field's width in bits.
    pub bits: u32,
    /// The field's least-significant bit position within the repr.
    pub lsb: u32,
}

/// A resolved overlay window passed to [`Evaluator::overlay_layout_fields`]: the
/// base struct, the window field's name when known (`Some` when resolved locally,
/// `None` for an injected clone whose window field name did not travel), and the
/// window's byte offset/size. The overflow diagnostic's `Struct.window` display
/// name is formatted from these ONLY in the error arm ([`WindowRef::desc`]), so
/// the success path allocates nothing.
struct WindowRef<'w> {
    base_struct: &'w str,
    window_field: Option<&'w str>,
    offset: i128,
    size: i128,
}

impl WindowRef<'_> {
    /// The window's display name for the `[overlay.window-overflow]` diagnostic:
    /// `Struct.window` when the field name is known, else just `Struct`.
    fn desc(&self) -> String {
        match self.window_field {
            Some(w) => format!("{}.{w}", self.base_struct),
            None => self.base_struct.to_string(),
        }
    }
}

/// A resolved SST overlay layout (Spec 2, Plan 7 #6, Part A — D6.A1/A2/A7/A9):
/// a typed view over a `[u8; N]` window field of a base struct. The overlay's
/// fields lay out by struct rules (declaration order, no padding), and their
/// offsets are OVERLAY-RELATIVE — the window's own offset within the base
/// struct is added only at the field-access sugar site (D6.A9, the next task),
/// never here. A poisoned overlay (window unresolved / capacity blown /
/// shadowing) carries `poisoned = true` so a re-query stays silent.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayInfo {
    /// The base struct this overlay's window belongs to.
    pub base_struct: String,
    /// The window field's byte offset within the base struct.
    pub window_offset: i128,
    /// The window field's byte size (its `N` in `[u8; N]`).
    pub window_size: i128,
    /// The overlay's fields, in declaration order: `(name, overlay-relative
    /// offset, byte size)`.
    pub fields: Vec<(String, i128, i128)>,
    /// The overlay's total laid-out byte size (`sizeof`).
    pub size: i128,
    /// True once a declaration check failed and reported; suppresses re-report.
    pub poisoned: bool,
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
                    "u8" => Ty::Prim { width: 1, signed: false, le: false },
                    "i8" => Ty::Prim { width: 1, signed: true, le: false },
                    "u16" => Ty::Prim { width: 2, signed: false, le: false },
                    "i16" => Ty::Prim { width: 2, signed: true, le: false },
                    "u32" => Ty::Prim { width: 4, signed: false, le: false },
                    "i32" => Ty::Prim { width: 4, signed: true, le: false },
                    // Explicit little-endian override (R-T0.1 / DSM.7): usable
                    // from ANY section, not just Z80 — the point is a 68k-side
                    // section emitting bytes a Z80 consumer reads. No `u32le`,
                    // no `u16be`-on-Z80: YAGNI until a customer exists.
                    "u16le" => Ty::Prim { width: 2, signed: false, le: true },
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
    /// reported).
    ///
    /// The newtype-underlying recursion is cycle-guarded against the shared
    /// [`layout_in_progress`](Evaluator) stack (mirroring
    /// [`size_of_newtype`](Self::size_of_newtype)): a `newtype A = B; newtype B
    /// = A` chain would otherwise recurse forever and overflow the native stack.
    /// On a detected repeat this reports `cyclic type: A -> B -> A` at `span`
    /// and returns `None`. The caller checks whether a diagnostic was already
    /// emitted before adding its own generic "not a struct" message, so a
    /// newtype cycle yields exactly one (specific) diagnostic.
    pub(crate) fn struct_name_for_offsetof(&mut self, ty: &Ty, span: Span) -> Option<String> {
        match ty {
            Ty::Struct(name) => Some(name.clone()),
            Ty::Refined { inner, .. } => self.struct_name_for_offsetof(inner, span),
            Ty::Newtype(name) => {
                self.with_cycle_guard(crate::eval::CycleStack::Layout, name, span, "type", |this| {
                    let decl = this.newtypes.get(name.as_str()).copied()?;
                    let underlying = this.resolve_type(&decl.underlying);
                    this.struct_name_for_offsetof(&underlying, span)
                })?
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
        let v = self.eval_expr(expr, &mut env);
        // A provisional `here()` (a `LinkExpr`) cannot size an array or bound a
        // refinement (D-H.2) — refuse with the specific `[here.provisional]`
        // message rather than the generic "expected an integer" below.
        if self.reject_if_provisional(&v, crate::parser::expr_span(expr)).is_some() {
            return None;
        }
        // A `Value::Typed` (a newtype/fixed value) erases to its stored int
        // (§8.3), so a nominally-typed comptime value is still a usable array
        // length / refinement bound.
        if let Some(n) = v.as_stored_int() {
            return Some(n);
        }
        match v {
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
            Ty::Fixed { i, f } => self.fixed_byte_size(*i, *f, span),
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
        let result = self.with_cycle_guard(crate::eval::CycleStack::Layout, name, span, "type", |this| {
            // Copy the `&'a NewtypeDecl` out so `self` is free to be mutated
            // across the recursive resolve/size below (as
            // `resolve_const`/`layout_of_struct`).
            let Some(decl) = this.newtypes.get(name).copied() else {
                return 0;
            };
            let underlying = this.resolve_type(&decl.underlying);
            this.size_of_ty(&underlying, span)
        });
        result.unwrap_or(0)
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
        // (a) FIELD-TYPE cycle: a deeper recursive call (a field whose type is
        // this struct, directly or transitively) may have closed a cycle back
        // to `name` and already memoized a poisoned (zero-size) layout for it.
        // That poison is the correct answer — do NOT overwrite it with the
        // ordinary, numerically-wrong layout this frame just computed over the
        // truncated (size-0) cyclic field, and skip the T3 checks entirely so a
        // cyclic struct gets ONLY its cycle diagnostic (no size-mismatch/odd-
        // field pile-on). Unlike `Value::Poison`, a `Layout` does not self-
        // propagate through the field loop, so this guard is what makes multi-
        // hop / mutual struct cycles report a poisoned layout, not a lie.
        // NOTE: `name` is STILL on `layout_in_progress` here (it is popped only
        // after the T3 checks below) — see the CRITICAL comment on the checks.
        if let Some(existing) = self.struct_layout_memo.get(name) {
            let existing = existing.clone();
            self.layout_in_progress.pop();
            return existing;
        }
        let layout = Layout { size: offset, fields };
        // T3: struct layout checks, run exactly once per struct on the freshly-
        // computed RAW layout, before it is memoized — a later query short-
        // circuits at the top-of-function memo lookup and never re-enters here.
        //
        // CRITICAL: these run with `name` STILL on `layout_in_progress` (it is
        // popped only after the (b) re-check below). `check_struct_size` /
        // `check_struct_offsets` evaluate the `(size:)` / `@offset` exprs, which
        // may themselves call `sizeof`/`offsetof` on THIS SAME struct
        // (`struct Foo (size: sizeof(Foo)) { .. }`). Keeping `name` in-progress
        // routes that re-entrant `layout_of_struct(name)` into the cycle-
        // detection branch above (a proper `cyclic struct layout: Foo -> Foo`
        // diagnostic) instead of falling through to infinite recursion. Each
        // check bails the moment such a re-entrant call has poisoned our memo
        // entry, so no spurious size-mismatch piles on top of the cycle report.
        self.check_struct_size(name, decl, &layout);
        self.check_struct_offsets(name, decl, &layout);
        self.check_struct_odd_fields(name, decl, &layout);
        // (b) CHECK-EXPR cycle: a `(size:)`/`@offset` expr above may have called
        // `sizeof`/`offsetof(Self)`, closing the layout cycle and slice-poisoning
        // `name`'s memo entry. Keep that poison — do NOT overwrite it with the
        // layout this frame computed (the same overwrite class guarded at (a)).
        if let Some(existing) = self.struct_layout_memo.get(name) {
            let existing = existing.clone();
            self.layout_in_progress.pop();
            return existing;
        }
        self.layout_in_progress.pop();
        self.struct_layout_memo.insert(name.to_string(), layout.clone());
        layout
    }

    /// Compute (or fetch the memoized) [`OverlayInfo`] for the SST overlay named
    /// `name` (Spec 2, Plan 7 #6 — D6.A1/A2/A7/A9), mirroring
    /// [`layout_of_struct`](Self::layout_of_struct)'s memoized, report-once shape.
    ///
    /// Steps: (1) resolve the region path to a `[u8; N]` window field of an
    /// in-scope struct (D6.A1); (2) lay out the overlay's fields by struct rules
    /// (declaration order, no padding — reusing [`size_of_ty`](Self::size_of_ty))
    /// with the odd-field warning (D6.A2); (3) reject any field whose name equals
    /// a DIRECT base-struct field (D6.A7 shadow check); (4) reject a total size
    /// exceeding the window's N bytes (D6.A2 capacity). Every failure path
    /// reports once and memoizes a `poisoned` result so a re-query stays silent.
    ///
    /// Overlays cannot form a layout cycle (an overlay never references another
    /// overlay), so — unlike [`layout_of_struct`](Self::layout_of_struct) — this
    /// needs no in-progress stack; every diagnostic anchors at `decl.span` (or a
    /// field's span), so the caller-supplied `_span` goes unused (kept for
    /// signature symmetry, mirroring [`layout_of_bitfield`](Self::layout_of_bitfield)).
    pub(crate) fn overlay_layout(&mut self, name: &str, _span: Span) -> OverlayInfo {
        if let Some(o) = self.overlay_layout_memo.get(name) {
            return o.clone();
        }
        let poison = |this: &mut Self, base: String| {
            let o = OverlayInfo {
                base_struct: base,
                window_offset: 0,
                window_size: 0,
                fields: Vec::new(),
                size: 0,
                poisoned: true,
            };
            this.overlay_layout_memo.insert(name.to_string(), o.clone());
            o
        };
        // Copy the `&'a VarsDecl` out so `self` is free to be mutated across the
        // recursive resolve/size calls below (mirrors `layout_of_struct`).
        let decl: &'a ast::VarsDecl = match self.overlays.get(name).copied() {
            Some(d) => d,
            None => {
                debug_assert!(false, "overlay_layout called for unknown overlay `{name}`");
                return poison(self, String::new());
            }
        };
        // Injected-overlay fast path (Plan 7 #8): a consumer's clone carries the
        // window binding resolved at the DEFINITION site. Use it verbatim — never
        // re-scan the window in the consumer's namespace (which could rebind it to
        // an unrelated same-named field, or a colliding consumer struct could poison
        // it with a spurious ambiguity). The window resolution, base-struct
        // existence, `[u8; N]` shape, and shadow checks all already ran and reported
        // at the defining module, so they are skipped here; only the pure per-field
        // layout (capacity + odd-field, both keyed solely on the stamped window
        // offset/size) is re-run — it needs nothing from the consumer's structs.
        if let Some(rw) = &decl.resolved_window {
            return self.overlay_layout_from_window(name, decl, rw);
        }
        // (1) Window resolution (D6.A1). The region path is either dotted
        // `[Struct, window]` or bare `[window]`; >2 segments name the dotted form.
        let (base_struct, window_field) = match decl.region.as_slice() {
            [w] => match self.resolve_bare_window(w, decl.span) {
                Some(pair) => pair,
                None => return poison(self, String::new()),
            },
            [s, w] => (s.clone(), w.clone()),
            _ => {
                self.error(
                    decl.span,
                    format!(
                        "[overlay.bad-window] overlay `{name}` window path `{}` has too many segments — use `Struct.window`",
                        decl.region.join(".")
                    ),
                );
                return poison(self, String::new());
            }
        };
        // The named struct must exist.
        if !self.structs.contains_key(base_struct.as_str()) {
            self.error(
                decl.span,
                format!(
                    "[overlay.unknown-window] overlay `{name}` targets unknown struct `{base_struct}`"
                ),
            );
            return poison(self, base_struct);
        }
        let base_layout = self.layout_of_struct(&base_struct, decl.span);
        let Some(window) = base_layout.fields.iter().find(|f| f.name == window_field).cloned() else {
            self.error(
                decl.span,
                format!(
                    "[overlay.unknown-window] overlay `{name}` window `{base_struct}.{window_field}` names no field of struct `{base_struct}`"
                ),
            );
            return poison(self, base_struct);
        };
        // The window must be a `[u8; N]` byte array (D6.A1 v1 restriction) —
        // UNSIGNED bytes exactly, so `[i8; N]` is rejected too.
        let window_n = match &window.ty {
            Ty::Array(elem, n) if matches!(**elem, Ty::Prim { width: 1, signed: false, .. }) => {
                *n as i128
            }
            _ => {
                self.error(
                    decl.span,
                    format!(
                        "[overlay.window-not-bytes] overlay `{name}` window `{base_struct}.{window_field}` is `{}` — overlay windows must be `[u8; N]` (v1)",
                        window.ty.describe()
                    ),
                );
                return poison(self, base_struct);
            }
        };
        let window_offset = window.offset as i128;
        // (3) Shadow check (D6.A7): an overlay field colliding with a DIRECT base
        // field. Reported at the overlay decl; does not stop layout of the rest.
        let mut shadowed = false;
        for f in &decl.fields {
            if base_layout.fields.iter().any(|bf| bf.name == f.name) {
                self.error(
                    f.span,
                    format!(
                        "[overlay.shadows-field] overlay `{name}` field `{}` shadows a direct field of struct `{base_struct}`",
                        f.name
                    ),
                );
                shadowed = true;
            }
        }
        // (2)+(4) Field layout and capacity, shared with the injected-overlay
        // fast path. The shadow check above already ran (it needs the base
        // struct's direct fields), so its verdict seeds `poisoned`.
        let win = WindowRef {
            base_struct: &base_struct,
            window_field: Some(&window_field),
            offset: window_offset,
            size: window_n,
        };
        self.overlay_layout_fields(name, decl, &win, shadowed)
    }

    /// The injected-overlay (Plan 7 #8) entry into overlay layout: a consumer
    /// clone whose window was resolved at the DEFINING module. The window
    /// (`base_struct` + offset + size) is taken verbatim — no window resolution,
    /// base-struct lookup, or shadow check runs in the consumer (all did at the
    /// definition site). Only the per-field layout (D6.A2) is re-run here; it
    /// touches nothing in the consumer's structs, so the result is identical to
    /// the defining module's, and the overlay stays bound where it was defined.
    fn overlay_layout_from_window(
        &mut self,
        name: &str,
        decl: &'a ast::VarsDecl,
        rw: &ast::ResolvedWindow,
    ) -> OverlayInfo {
        let win = WindowRef {
            base_struct: &rw.base_struct,
            window_field: None,
            offset: rw.window_offset,
            size: rw.window_size,
        };
        self.overlay_layout_fields(name, decl, &win, false)
    }

    /// Lay out an overlay's fields over an already-resolved `win`dow (D6.A2):
    /// shared tail of the window-resolving path and the injected-overlay fast path.
    /// Fields pack by struct rules (declaration order, no padding); the odd-field
    /// lint keys on the RUNTIME parity `window.offset + rel`, and total >
    /// `window.size` is `[overlay.window-overflow]`. `shadowed` seeds the poisoned
    /// flag (the shadow check runs only in the window-resolving path, which owns
    /// the base layout).
    fn overlay_layout_fields(
        &mut self,
        name: &str,
        decl: &'a ast::VarsDecl,
        win: &WindowRef,
        shadowed: bool,
    ) -> OverlayInfo {
        let base_struct = win.base_struct;
        let window_offset = win.offset;
        let poison = |this: &mut Self| {
            let o = OverlayInfo {
                base_struct: base_struct.to_string(),
                window_offset: 0,
                window_size: 0,
                fields: Vec::new(),
                size: 0,
                poisoned: true,
            };
            this.overlay_layout_memo.insert(name.to_string(), o.clone());
            o
        };
        let mut offset: i128 = 0;
        let mut fields: Vec<(String, i128, i128)> = Vec::with_capacity(decl.fields.len());
        for f in &decl.fields {
            let ty = self.resolve_type(&f.ty);
            let size = self.size_of_ty(&ty, f.span) as i128;
            // `[layout.odd-field]` (§4.3) applies to overlay word/long fields —
            // keyed on the RUNTIME parity: the field's in-memory offset within
            // the base struct is `window_offset + overlay-relative offset`, and
            // an odd window base flips it. Keying on the relative offset alone
            // would both false-warn (odd base + odd rel = even memory) and
            // silently miss (odd base + even rel = odd memory).
            let mem_offset = window_offset + offset;
            if matches!(size, 2 | 4) && mem_offset % 2 == 1 {
                self.warn(
                    f.span,
                    format!(
                        "[layout.odd-field] overlay {name}: field {} ({}-byte) at odd offset {mem_offset} within `{base_struct}` (window base {window_offset} + {offset})",
                        f.name, size
                    ),
                );
            }
            fields.push((f.name.clone(), offset, size));
            offset += size;
        }
        let total = offset;
        // (4) Capacity (D6.A2): total > window N is an error at the overlay decl.
        if total > win.size {
            self.error(
                decl.span,
                format!(
                    "[overlay.window-overflow] overlay `{name}` is {total} bytes — exceeds `{}` window of {} bytes (over by {})",
                    win.desc(),
                    win.size,
                    total - win.size
                ),
            );
            return poison(self);
        }
        let info = OverlayInfo {
            base_struct: base_struct.to_string(),
            window_offset,
            window_size: win.size,
            fields,
            size: total,
            poisoned: shadowed,
        };
        self.overlay_layout_memo.insert(name.to_string(), info.clone());
        info
    }

    /// Resolve a bare overlay window name `w` (D6.A1): scan in-scope structs for
    /// a field named `w`. Zero hits → `[overlay.unknown-window]`; ≥2 →
    /// `[overlay.ambiguous-window]` listing the candidates as `S.w` and
    /// suggesting the dotted form; exactly one → resolve to `(struct, field)`.
    /// Returns `None` (after reporting) on 0 / ≥2.
    ///
    /// Matching is by AST FIELD NAME regardless of the field's type — the
    /// `[u8; N]` restriction is enforced uniformly in
    /// [`overlay_layout`](Self::overlay_layout) (so a lone non-byte-array `w`
    /// surfaces the precise `[overlay.window-not-bytes]` rather than a
    /// misleading unknown-window).
    ///
    /// CRITICAL: the scan must NOT force `layout_of_struct` on the candidates —
    /// forcing layout runs each struct's declaration checks
    /// (size/offset/odd-field) as a side effect, so merely declaring a
    /// bare-window overlay would switch on validation for every UNRELATED
    /// in-scope struct, and the bare vs dotted spellings of the same overlay
    /// would produce different module diagnostics. Only the single chosen base
    /// struct is laid out, by [`overlay_layout`](Self::overlay_layout) — exactly
    /// as the dotted path behaves.
    fn resolve_bare_window(&mut self, w: &str, span: Span) -> Option<(String, String)> {
        // Deterministic candidate order: struct decls in source order. `structs`
        // is a HashMap, so sort by each struct's declaration span to keep the
        // ambiguity message (and its fix-it) stable across runs.
        let mut hits: Vec<(u32, String)> = self
            .structs
            .iter()
            .filter(|(_, decl)| decl.fields.iter().any(|f| f.name == w))
            .map(|(sname, decl)| (decl.span.start, sname.to_string()))
            .collect();
        hits.sort();
        let hits: Vec<String> = hits.into_iter().map(|(_, s)| s).collect();
        match hits.as_slice() {
            [] => {
                self.error(
                    span,
                    format!(
                        "[overlay.unknown-window] no in-scope struct has a `[u8; N]` field named `{w}`"
                    ),
                );
                None
            }
            [only] => Some((only.clone(), w.to_string())),
            many => {
                let candidates =
                    many.iter().map(|s| format!("{s}.{w}")).collect::<Vec<_>>().join(", ");
                self.error(
                    span,
                    format!(
                        "[overlay.ambiguous-window] window `{w}` is ambiguous across {candidates} — qualify it as `Struct.{w}`"
                    ),
                );
                None
            }
        }
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
        // Re-entrancy guard: evaluating the `(size:)` expr may have called
        // `sizeof`/`offsetof` on this same struct, closing the layout cycle and
        // poisoning our memo entry (which does not exist for a non-cyclic struct
        // until the very end of `layout_of_struct`). That cyclic-layout
        // diagnostic stands alone — bail rather than pile a spurious
        // size-mismatch on top of it.
        if self.struct_layout_memo.contains_key(name) {
            return;
        }
        let computed = layout.size as i128;
        if declared == computed {
            return;
        }
        let mut msg = format!("struct {name}: declared size {declared} but fields total {computed}");
        for f in &layout.fields {
            msg.push_str(&format!("\n  {} @{} ({} byte{})", f.name, f.offset, f.size, if f.size == 1 { "" } else { "s" }));
        }
        // Directional/absolute delta — never a bare negative. `computed <
        // declared` means the fields fall short of the declared size (the struct
        // is "too small"); `computed > declared` overshoots it.
        let delta = computed - declared;
        let dir = if delta < 0 { "too small" } else { "too large" };
        msg.push_str(&format!("\n  {computed} vs {declared} (off by {}, {dir})", delta.abs()));
        self.error(crate::parser::expr_span(size_expr), msg);
    }

    /// D-P3.9 `@offset` field assertions: for every field that carries an
    /// explicit `@ expr`, compare it to that field's computed offset within
    /// `layout`. A mismatch is one diagnostic per offending field.
    fn check_struct_offsets(&mut self, name: &str, decl: &ast::StructDecl, layout: &Layout) {
        // A cycle that closed during `check_struct_size` (or an earlier field's
        // `@offset` expr) already poisoned our memo entry — stop, don't add
        // offset-mismatch noise on top of the cycle report.
        if self.struct_layout_memo.contains_key(name) {
            return;
        }
        for (field_decl, field_layout) in decl.fields.iter().zip(layout.fields.iter()) {
            let Some(offset_expr) = &field_decl.offset else { continue };
            let Some(asserted) = self.eval_const_index(offset_expr) else { continue };
            // Same re-entrancy guard: a self-referential `@offset` expr (e.g.
            // `b: u8 @ sizeof(Self)`) may have closed the layout cycle.
            if self.struct_layout_memo.contains_key(name) {
                return;
            }
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
        // A cycle closed during an earlier check (via a `(size:)`/`@offset` expr)
        // already poisoned our memo entry — don't warn on a struct we're about
        // to return as poison.
        if self.struct_layout_memo.contains_key(name) {
            return;
        }
        for (field_decl, field_layout) in decl.fields.iter().zip(layout.fields.iter()) {
            if matches!(field_layout.size, 2 | 4) && field_layout.offset % 2 == 1 {
                // The warning is anchored at the whole field declaration's span
                // (`field_decl.span`), not just the type or offset token — a
                // deliberate choice so the caret covers the entire offending
                // field, which reads better than pointing at a sub-token.
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

    /// Compute the bit layout of the bitfield named `name` (T4, D-P3.10).
    ///
    /// Fields are declared MSB→LSB: a `cursor` starts at `repr_bits` and walks
    /// downward. A field with an explicit `@ N` anchor is placed at `lsb = N`
    /// (asserting `N + bits <= repr_bits`, else `[bitfield.field-out-of-range]`)
    /// and resets the cursor to `N`; an unanchored field is placed immediately
    /// below the cursor (`lsb = cursor - bits`, asserting `bits <= cursor`, else
    /// `[bitfield.overflow]`). Fields must FIT, not FILL — widths need not sum
    /// to `repr_bits` (unused high/gap bits are simply 0). Overlapping field
    /// ranges (however they arose — usually via anchors) are
    /// `[bitfield.field-overlap]`.
    ///
    /// Bitfields don't recurse (a repr is always a primitive), so unlike
    /// [`layout_of_struct`](Self::layout_of_struct) this needs no memo or
    /// cycle-detection stack — every diagnostic below anchors at the
    /// offending field's own span, so `_span` (kept for signature symmetry
    /// with `layout_of_struct`, and so every call site already has one handy)
    /// goes unused.
    pub(crate) fn layout_of_bitfield(&mut self, name: &str, _span: Span) -> BitfieldLayout {
        // Memoized: a malformed bitfield's diagnostics fire exactly once, not
        // once per referencing literal (mirrors `struct_layout_memo`).
        if let Some(l) = self.bitfield_layout_memo.get(name) {
            return l.clone();
        }
        let Some(decl) = self.bitfields.get(name).copied() else {
            debug_assert!(false, "layout_of_bitfield called for unknown bitfield `{name}`");
            return BitfieldLayout { repr_bits: 0, fields: Vec::new() };
        };
        let repr_ty = self.resolve_type(&decl.repr);
        let repr_bits: u32 = match &repr_ty {
            Ty::Prim { width, signed: false, .. } => u32::from(*width) * 8,
            Ty::Poison => {
                // A poisoned repr already reported (via `resolve_type`); memoize
                // the empty layout so a re-query doesn't re-resolve/re-report.
                let empty = BitfieldLayout { repr_bits: 0, fields: Vec::new() };
                self.bitfield_layout_memo.insert(name.to_string(), empty.clone());
                return empty;
            }
            other => {
                self.error(
                    decl.span,
                    format!("bitfield {name}: repr must be u8, u16, or u32, got {}", other.describe()),
                );
                let empty = BitfieldLayout { repr_bits: 0, fields: Vec::new() };
                self.bitfield_layout_memo.insert(name.to_string(), empty.clone());
                return empty;
            }
        };
        let mut cursor = repr_bits;
        // (lsb, bits) of every field placed so far, for the overlap check.
        let mut placed: Vec<(u32, u32)> = Vec::with_capacity(decl.fields.len());
        let mut fields = Vec::with_capacity(decl.fields.len());
        for f in &decl.fields {
            let lsb = if let Some(anchor) = f.anchor {
                match anchor.checked_add(f.bits) {
                    Some(top) if top <= repr_bits => {
                        cursor = anchor;
                        anchor
                    }
                    _ => {
                        self.error(
                            f.span,
                            format!(
                                "[bitfield.field-out-of-range] bitfield {name}: field {} ({} bit{} @ {anchor}) exceeds the {repr_bits}-bit repr",
                                f.name, f.bits, if f.bits == 1 { "" } else { "s" }
                            ),
                        );
                        continue;
                    }
                }
            } else if f.bits <= cursor {
                let lsb = cursor - f.bits;
                cursor = lsb;
                lsb
            } else {
                self.error(
                    f.span,
                    format!(
                        "[bitfield.overflow] bitfield {name}: field {} ({} bit{}) overflows the {repr_bits}-bit repr (only {cursor} bit{} left)",
                        f.name, f.bits, if f.bits == 1 { "" } else { "s" },
                        if cursor == 1 { "" } else { "s" }
                    ),
                );
                continue;
            };
            let hi = lsb + f.bits; // exclusive top
            if placed.iter().any(|&(p_lsb, p_bits)| lsb < p_lsb + p_bits && p_lsb < hi) {
                self.error(
                    f.span,
                    format!(
                        "[bitfield.field-overlap] bitfield {name}: field {} (bits {lsb}..{}) overlaps another field",
                        f.name, hi - 1
                    ),
                );
                continue;
            }
            placed.push((lsb, f.bits));
            fields.push(BitfieldFieldLayout { name: f.name.clone(), bits: f.bits, lsb });
        }
        let layout = BitfieldLayout { repr_bits, fields };
        self.bitfield_layout_memo.insert(name.to_string(), layout.clone());
        layout
    }

    /// The ONE shared refinement mechanism (D-P3.6): check `val` against the
    /// effective scalar bounds of `ty`, via [`check_in_range`](Self::check_in_range).
    /// This backs newtype/refined construction (T4, `Name(x)`) — bitfield field
    /// widths use `check_in_range` directly (each width already IS its own
    /// `0..=(2^bits-1)` bound, computed in [`layout_of_bitfield`](Self::layout_of_bitfield)'s
    /// caller) rather than routing through here.
    ///
    /// - [`Ty::Refined`] checks `val` against its own `lo..=hi`.
    /// - [`Ty::Newtype`] checks against the newtype's own `where` bound if it
    ///   declared one, else recurses into its resolved underlying type — cycle-
    ///   guarded against the shared [`layout_in_progress`](Evaluator) stack
    ///   (mirroring [`size_of_newtype`](Self::size_of_newtype)) so a
    ///   `newtype A = B; newtype B = A` chain (with no `where` on either) is
    ///   diagnosed, not a stack overflow.
    /// - [`Ty::Prim`] checks against the primitive's natural range (`u8:
    ///   0..=255`, `i8: -128..=127`, etc).
    /// - Anything else (struct/bitfield/enum/…) is not a scalar refinement —
    ///   there is no bound to check, so this returns `true` (those types are
    ///   never constructed via the `Name(x)` call syntax this backs).
    pub(crate) fn check_value_fits_ty(&mut self, ty: &Ty, val: i128, span: Span) -> bool {
        // The outermost type's own name is the "label" every downstream
        // diagnostic blames: a `newtype Angle = u8` whose bound is really the
        // underlying `u8` still names *Angle* (not `u8`) in the message, since
        // that is the type the author wrote at the construction site.
        let label = ty.describe();
        self.check_value_fits_ty_labeled(ty, val, span, &label)
    }

    /// The recursive worker behind [`check_value_fits_ty`](Self::check_value_fits_ty),
    /// carrying `label` — the outermost constructed type's name — so a newtype
    /// that bottoms out in a bare primitive still names the *newtype* rather
    /// than the underlying `u8` in its out-of-range diagnostic.
    fn check_value_fits_ty_labeled(&mut self, ty: &Ty, val: i128, span: Span, label: &str) -> bool {
        match ty {
            Ty::Refined { lo, hi, .. } => {
                self.check_in_range(val, *lo, *hi, span, &format!("{label} construction"))
            }
            Ty::Newtype(name) => {
                if let Some(start) = self.layout_in_progress.iter().position(|n| n == name) {
                    let mut chain: Vec<&str> =
                        self.layout_in_progress[start..].iter().map(|s| s.as_str()).collect();
                    chain.push(name);
                    self.error(span, format!("cyclic type: {}", chain.join(" -> ")));
                    return false;
                }
                let Some(decl) = self.newtypes.get(name.as_str()).copied() else {
                    // Unreachable in practice (mirrors `size_of_newtype`): the
                    // caller only builds `Ty::Newtype(name)` for a name already
                    // known in `self.newtypes`.
                    return true;
                };
                if let Some((lo_expr, hi_expr)) = &decl.refine {
                    // A newtype whose `where` bound (transitively) constructs the
                    // SAME newtype (`newtype N = u8 where 0 .. N(2)`) would re-enter
                    // this validation without bound and abort the process with a
                    // native stack overflow (T8 review, Critical). Guard on a
                    // DEDICATED stack — NOT `layout_in_progress`, whose reuse would
                    // falsely flag the legitimate `where 0 .. sizeof(S)` /
                    // `struct S { x: N }` size re-entrancy — so a construction cycle
                    // is diagnosed like the underlying-chain cycle above.
                    let result =
                        self.with_cycle_guard(crate::eval::CycleStack::Refine, name, span, "type", |this| {
                            match (this.eval_const_index(lo_expr), this.eval_const_index(hi_expr)) {
                                (Some(lo), Some(hi)) => {
                                    this.check_in_range(val, lo, hi, span, &format!("newtype {label}"))
                                }
                                // A non-int bound already reported via `eval_const_index`.
                                _ => false,
                            }
                        });
                    return result.unwrap_or(false);
                }
                // Recurse into the underlying type but KEEP `label` — the author
                // wrote `Angle(x)`, so the eventual `u8` range check must blame
                // `Angle`, not `u8`. (The top-of-arm check above already ruled out
                // `name` being on `layout_in_progress`, and nothing between there
                // and here can push it, so this guard's own check never fires — it
                // exists to pair the push with a guaranteed pop.)
                let result = self.with_cycle_guard(crate::eval::CycleStack::Layout, name, span, "type", |this| {
                    let underlying = this.resolve_type(&decl.underlying);
                    this.check_value_fits_ty_labeled(&underlying, val, span, label)
                });
                result.unwrap_or(false)
            }
            // `le` never affects the accepted range (R-T0.1): the byte-order
            // flag is emission-only, so this range check is identical for
            // `u16` and `u16le`.
            Ty::Prim { width, signed, .. } => {
                let (lo, hi) = prim_bounds(*width, *signed);
                self.check_in_range(val, lo, hi, span, &format!("{label} value"))
            }
            // A `fixed<I,F>` value is stored in an `I+F`-bit SIGNED integer
            // (T5, D2.10): its stored int must fit `-(2^(bits-1)) ..= 2^(bits-1)-1`.
            // So `Fix(999999)` for `newtype Fix = fixed<4,4>` (an 8-bit store,
            // −128..127) is an out-of-range construction, not a silent accept.
            Ty::Fixed { i, f } => {
                let Some(bits) = self.fixed_width_bits(*i, *f, span) else {
                    return false;
                };
                let lo = -(1i128 << (bits - 1));
                let hi = (1i128 << (bits - 1)) - 1;
                self.check_in_range(val, lo, hi, span, &format!("{label} value"))
            }
            // A newtype whose underlying is a struct/bitfield/enum carries no
            // scalar bound this mechanism knows how to check — those types are
            // never constructed via the `Name(x)` call syntax this backs, so
            // there is nothing to range-check.
            _ => true,
        }
    }

    /// The BYTE size of a `fixed<I,F>` (`(I+F)/8`), and the ONE place the
    /// "not a whole number of bytes" diagnostic lives — shared by
    /// [`size_of_ty`](Self::size_of_ty)'s [`Ty::Fixed`] arm and T7's
    /// [`lower_fixed`](Self::lower_fixed) so the two cannot diverge (a non-byte
    /// `fixed<>` must diagnose identically whether it is a struct field or a
    /// top-level `data`/array element). `I`/`F` are each a full `u32`, so the sum
    /// is checked. A non-whole-byte width still returns a best-effort ceil so a
    /// caller has a plausible size.
    pub(crate) fn fixed_byte_size(&mut self, i: u32, f: u32, span: Span) -> usize {
        let Some(bits) = i.checked_add(f) else {
            self.error(span, format!("fixed<{i},{f}> is too large to size"));
            return 0;
        };
        if bits % 8 != 0 {
            self.error(span, format!("fixed<{i},{f}> is not a whole number of bytes"));
            return bits.div_ceil(8) as usize;
        }
        let width_bytes = (bits / 8) as usize;
        // A whole-byte fixed wider than 4 bytes (e.g. `fixed<32,32>` = 8 bytes)
        // is un-storable as a scalar — the 68k `.b/.w/.l` directives are 1/2/4
        // bytes (T8 review, Minor 2). Emission (`lower_fixed`) already rejected
        // it, but layout/sizeof silently sized it, so a struct field of such a
        // type laid out and passed `(size:)`/`offsetof` while emission refused
        // it. Diagnose here so layout and emission AGREE it is unusable. (This is
        // the ONE shared site, so `lower_fixed` no longer re-reports.)
        if width_bytes > 4 {
            self.error(
                span,
                format!(
                    "fixed<{i},{f}> is {width_bytes} bytes; too wide to store as a scalar (max 4) — rescale before storing"
                ),
            );
        }
        width_bytes
    }

    /// The bit width of a `fixed<I,F>` (`I + F`), guarding the two degenerate
    /// widths that would make sized arithmetic silently wrong (T5, review Minor
    /// #7): a zero-bit fixed (no store) and a `>= 128`-bit fixed (unrepresentable
    /// in the i128 comptime domain — [`wrap_bits`](crate::eval) silently skips
    /// wrapping at `bits >= 128`). Either is a `[fixed.too-wide]` diagnostic and
    /// `None`; otherwise the width. `I`/`F` are each a full `u32`, so the sum is
    /// computed in `u64` to avoid its own overflow.
    pub(crate) fn fixed_width_bits(&mut self, i: u32, f: u32, span: Span) -> Option<u32> {
        let bits = u64::from(i) + u64::from(f);
        if bits == 0 || bits >= 128 {
            self.error(
                span,
                format!(
                    "[fixed.too-wide] fixed<{i},{f}> ({bits} bits) is not a usable comptime fixed-point width (need 1..=127 bits)"
                ),
            );
            None
        } else {
            Some(bits as u32)
        }
    }
}

/// The inclusive natural range of a `width`-byte primitive, signed or
/// unsigned. `width` is always 1, 2, or 4 (the [`Ty::Prim`] invariant); any
/// other width is unreachable and falls back to the full `i128` range so a
/// future widening degrades gracefully instead of panicking.
pub(crate) fn prim_bounds(width: u8, signed: bool) -> (i128, i128) {
    match (width, signed) {
        (1, false) => (0, u8::MAX as i128),
        (1, true) => (i8::MIN as i128, i8::MAX as i128),
        (2, false) => (0, u16::MAX as i128),
        (2, true) => (i16::MIN as i128, i16::MAX as i128),
        (4, false) => (0, u32::MAX as i128),
        (4, true) => (i32::MIN as i128, i32::MAX as i128),
        _ => (i128::MIN, i128::MAX),
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

/// The position a `here()` (§7.1) resolves to inside a lowered item — the
/// exact/provisional distinction the fix turns on (D-H.1). `base` is the item's
/// start VMA at the baseline layout (every relaxable counted at its smallest
/// rung). `anchor` is `None` at an EXACT position (`here()` → `Value::Int(base)`,
/// byte-identical to before the fix) or `Some(label)` at a PROVISIONAL one, where
/// the open section already holds a size-relaxable fragment so the physical VMA
/// can still shift; `here()` then yields `Value::LinkExpr(Sym(label))`, resolved
/// against the anchor's post-relaxation VMA at link. For a data item the anchor
/// is the item's OWN label (D-H.3); for an item guard it is an anonymous label
/// the lowering pass mints on use (D-H.8).
#[derive(Clone, Debug)]
pub struct HerePos {
    /// The item's start VMA at baseline layout.
    pub base: u32,
    /// The provisional anchor label, or `None` at an exact position.
    pub anchor: Option<String>,
}

/// Lower the `data` item named `name` in `file` to a checked, CPU-neutral
/// [`DataBuf`] (T7, D-P3.5), returning it (or `None` if no such data item) plus
/// any diagnostics — the emission analogue of [`layout_struct`]. Runs on the
/// shared large-stack evaluation thread (see [`size_of_type`]): a data value can
/// drive comptime-fn calls (array element expressions, defaults).
pub fn eval_data(file: &ast::File, name: &str) -> (Option<DataBuf>, Vec<Diagnostic>) {
    eval_data_at(file, name, None)
}

/// Like [`eval_data`], but threads a `here` position so a `here()` (§7.1) inside
/// the item resolves to the item's start VMA (exact) or its link-time anchor
/// (provisional). The lowering pass supplies the position; other callers pass
/// `None` (no position — `here()` is an error).
pub fn eval_data_at(
    file: &ast::File,
    name: &str,
    here: Option<HerePos>,
) -> (Option<DataBuf>, Vec<Diagnostic>) {
    // Non-lowering callers (tests, `eval_data`) do not link, so a deferred
    // `LinkAssert` from a data-item guard has no consumer here — drop it. The
    // lowering pass calls `eval_data_with_root` directly and drains the asserts.
    let (buf, _asserts, diags) = eval_data_with_root(file, name, here, None);
    (buf, diags)
}

/// Like [`eval_data_at`], but also threads a capability-sandbox
/// `include_root` (Spec 2, Plan 5 — Task 1): the directory `embed`/`import`
/// paths resolve against. `include_root = None` behaves exactly like
/// [`eval_data_at`] (a comptime `embed(...)` inside the item then reports
/// `[sandbox.no-root]`) — every existing caller of `eval_data`/`eval_data_at`
/// is therefore unchanged. Tests point the root at a fixtures directory; the
/// lowering/CLI path does not yet supply a real root (wiring the source file's
/// directory into `lower_data_item` is deferred to the Plan-5 hermeticity task),
/// so through the production compile path `embed`/`import` currently report
/// `[sandbox.no-root]` until that wiring lands.
/// A PUBLIC view of one recorded comptime file read (Spec 2, Plan 5 — Task 5,
/// closing T1-review finding #3): the resolved path, its SHA-256 digest, and
/// its byte length. Field-for-field identical to the internal
/// `eval::sandbox::CaptureEdge` ledger entry — this type exists only so a
/// caller outside the crate (a future build-manifest / provenance report) can
/// read the ledger without reaching into a `pub(crate)` type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capture {
    /// The resolved (sandbox-root-joined, normalized) path that was read.
    pub path: PathBuf,
    /// SHA-256 digest of the file's exact bytes at read time.
    pub hash: [u8; 32],
    /// The file's byte length.
    pub len: u64,
}

pub fn eval_data_with_root(
    file: &ast::File,
    name: &str,
    here: Option<HerePos>,
    include_root: Option<&std::path::Path>,
) -> (Option<DataBuf>, Vec<sigil_ir::LinkAssert>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.apply_here_pos(here);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        if !ev.datas.contains_key(name) {
            ev.error(file.module.span, format!("no data item named `{name}`"));
            return (None, Vec::new(), ev.diags);
        }
        let buf = ev.resolve_data(name, file.module.span);
        check_max_size(&mut ev, name, buf.size, file.module.span);
        let asserts = ev.take_link_asserts();
        (Some(buf), asserts, ev.diags)
    })
}

/// Enforce a data item's `(max_size: expr)` capacity bound (D5.4). Evaluates the
/// `max_size` expression with the item's evaluator, requires a non-negative
/// comptime integer, and errors if the checked buffer's byte length `buf_len`
/// exceeds it — phrased like §7.3's region overflow. A no-op when the item has no
/// `max_size`. Kept on the `eval_data_with_root`/`eval_data_captures` path so BOTH
/// top-level and section-nested data items are covered by one check.
fn check_max_size(ev: &mut Evaluator, name: &str, buf_len: usize, span: Span) {
    let Some(max_expr) = ev.datas.get(name).and_then(|d| d.max_size.clone()) else {
        return;
    };
    let v = ev.eval_expr(&max_expr, &mut Env::new());
    // An already-reported error in the expression stays silent (D-P2.9). Checked
    // BEFORE `as_stored_int` so a poisoned bound can't cascade into a spurious
    // "must be a comptime integer" error.
    if matches!(v, Value::Poison) {
        return;
    }
    // `as_stored_int`: a `Typed`-wrapped int (a domain-newtype bound, e.g. a
    // prelude size type) erases to its stored int per §8.3 — the same rule array
    // lengths and bitfield field values follow.
    match v.as_stored_int() {
        Some(n) if n < 0 => {
            ev.error(span, format!("`max_size` must be >= 0, got {n}"));
        }
        Some(n) => {
            if buf_len as i128 > n {
                ev.error(
                    span,
                    format!(
                        "data `{name}` is {buf_len} bytes — exceeds max_size {n} (over by {} bytes)",
                        buf_len as i128 - n
                    ),
                );
            }
        }
        None => {
            // A provisional here() capacity bound gets the SPECIFIC D-H.2
            // steering message, not the generic "must be a comptime integer".
            if ev.reject_if_provisional(&v, span).is_none() {
                ev.error(
                    span,
                    format!("`max_size` must be a comptime integer, got {}", v.type_name()),
                );
            }
        }
    }
}

/// Like [`eval_data_with_root`], but ALSO returns the capture ledger — every
/// comptime file read (`embed`/`import`) recorded during this evaluation
/// (Spec 2, Plan 5 — Task 5, closing T1-review finding #3 + D-P5.6). Needed as
/// its own seam because [`eval_data_with_root`] builds and drops its
/// [`Evaluator`] entirely inside [`crate::eval::run_on_eval_stack`]'s closure —
/// there is no other way for a caller to observe `ev.captures` afterwards.
/// Ledger order is deterministic: `captures` is a plain `Vec` appended to in
/// evaluation order, so re-running this on the same inputs yields identical
/// output bytes AND an identical capture list.
pub fn eval_data_captures(
    file: &ast::File,
    name: &str,
    here: Option<HerePos>,
    include_root: Option<&std::path::Path>,
) -> (Option<DataBuf>, Vec<Capture>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.apply_here_pos(here);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        if !ev.datas.contains_key(name) {
            ev.error(file.module.span, format!("no data item named `{name}`"));
            return (None, Vec::new(), ev.diags);
        }
        let buf = ev.resolve_data(name, file.module.span);
        check_max_size(&mut ev, name, buf.size, file.module.span);
        let captures = ev
            .captures
            .iter()
            .map(|c| Capture { path: c.path.clone(), hash: c.hash, len: c.len })
            .collect();
        // Any deferred LinkAsserts (D-H.4) are deliberately DROPPED here, like
        // `eval_data_at`: this seam's callers are non-lowering (capture-ledger
        // tests, always `here: None`), so there is no linker to decide them. The
        // lowering pass drains asserts via `eval_data_with_root` instead.
        (Some(buf), captures, ev.diags)
    })
}

/// Lower an `offsets Name { Variant: target, ... }` block (Spec 2, Plan 7
/// backlog #3, Task 6 — the FORWARD direction) to a checked, CPU-neutral
/// [`DataBuf`]: one [`Cell::RelOffset`] per member (each a `dc.w target - Name`
/// word), whose `base` is the table's own label (`decl.name`) and whose
/// `target` is the member's referenced symbol. The sibling of
/// [`eval_data_with_root`] for `offsets` items — it threads `include_root` for
/// signature parity (a target `Expr` is a symbol reference, so no `embed`/
/// `import` runs here today), and its diagnostics/labels flow the same way.
///
/// A target is a symbol NAME, not a comptime value: the reverse direction
/// (`eval_path`'s `Name.Variant` ordinals) never evaluates it, and it names a
/// LINK-time label (a `data`/`proc`/offsets-base symbol) that has no comptime
/// value to evaluate to. So the name is taken BY SHAPE — the path text of a
/// bare `Path`, or the contents of a string literal. When the name genuinely
/// names a link symbol (the intended use), the single-segment path is the same
/// bare name `define_label` emits, so the `RelWord16Be` fixup's
/// `Sub(Sym(target), Sym(base))` resolves. When it does NOT name a real symbol,
/// the fixup surfaces at LINK time as an undefined-symbol error — this pass does
/// not (and cannot, without a full symbol table) verify that a path names a
/// real label. Two cases it DOES catch early, so the author is not left with
/// only `sigil_link`'s generic "unresolved target" message:
///
/// - a single-segment path that resolves to a `const` (an easy mistake: a
///   `const F0 = "frame0"` alias used as `M { A: F0 }` would silently emit a
///   fixup to the nonexistent symbol `F0`) — diagnosed here as "is a const,
///   not a label";
/// - a non-path/non-string expression (e.g. `1 + 1`), evaluated and its name
///   extracted the same way [`lower_ptr`](crate::eval::Evaluator) does (a
///   [`Value::FnRef`]/[`Value::Str`]), else a "must reference a label" error.
///
/// Both use an `<unresolved>` placeholder so the 2-byte slot still lands (sizes
/// stay consistent). A multi-segment path (`mod.thing`, a cross-module
/// reference) is NOT resolved here — it is joined `a.b` and left for a later
/// plan's cross-module resolution / the linker. Dup / reserved-`count`
/// validation is NOT re-done here — it lives once-per-compile in
/// `lower::validate_offsets`.
///
/// NOTE: [`eval_dispatch_with_root`] mirrors this function's shape (fresh env
/// per member, `Path`/`Str`/eval target extraction, `<unresolved>` placeholder)
/// — consider both when editing the target-extraction logic.
pub fn eval_offsets_with_root(
    file: &ast::File,
    decl: &ast::OffsetsDecl,
    include_root: Option<&std::path::Path>,
) -> (Option<DataBuf>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        let mut buf = DataBuf::empty();
        for member in &decl.members {
            // Fresh env per member (parity with `resolve_data`'s per-item
            // `Env::new()`): a fallback `eval_expr` below must not see bindings
            // leaked from an earlier member's evaluation.
            let mut env = Env::new();
            let name = match &member.target {
                ast::Expr::Path(p) => {
                    // A single-segment path that is a KNOWN const is almost
                    // certainly a mistake — a const is not a link label, so the
                    // fixup would target a nonexistent symbol. Flag it clearly
                    // here rather than let the author hit `sigil_link`'s generic
                    // "unresolved target" message. Only a POSITIVELY-identified
                    // const is rejected: a bare name absent from every registry
                    // may still be a valid data/proc/offsets label, so it is
                    // accepted and the linker catches a genuinely-undefined one.
                    if p.segments.len() == 1 && ev.is_const(&p.segments[0]) {
                        ev.error(
                            member.span,
                            format!(
                                "offset entry `{}` target `{}` is a const, not a label; offsets targets must be labels",
                                member.name, p.segments[0]
                            ),
                        );
                        "<unresolved>".to_string()
                    } else {
                        // The symbol name IS the path text. Single segment is the
                        // bare label name (the common case); multi-segment is
                        // joined `a.b` (cross-module resolution is a later plan).
                        p.segments.join(".")
                    }
                }
                // A string literal names a symbol directly (mirrors `lower_ptr`'s
                // `Value::Str` arm and `winptr("name")`).
                ast::Expr::Str(s, _) => s.clone(),
                // Any other expression is evaluated; a `FnRef`/`Str` yields the
                // name (as `lower_ptr` extracts it), anything else is an error.
                other => {
                    let v = ev.eval_expr(other, &mut env);
                    match v {
                        Value::FnRef(n) => n,
                        Value::Str(s) => s,
                        _ => {
                            ev.error(
                                crate::parser::expr_span(other),
                                format!(
                                    "offset entry `{}` must reference a label, got {}",
                                    member.name,
                                    v.type_name()
                                ),
                            );
                            "<unresolved>".to_string()
                        }
                    }
                }
            };
            buf.push(Cell::RelOffset { base: decl.name.clone(), target: name });
        }
        (Some(buf), ev.diags)
    })
}

/// The hygienic label of a dispatch member's inline body (Plan 7 #9a).
/// `$` is unlexable by both frontends (the `__here$<module>$<n>` precedent,
/// D-H.8), so it can never collide with a user symbol; module+table+member is
/// program-unique (duplicate members are a `validate_dispatch` error, and
/// duplicate table names are whole-program duplicate-label link errors — the
/// same story as the table's own base label today).
pub(crate) fn dispatch_body_label(module: &ast::Path, table: &str, member: &str) -> String {
    format!("__dispatch${}${table}${member}", module.segments.join("."))
}

/// Lower a `dispatch Name (encoding: E) { Member: target, ... }` block's
/// FORWARD emission (Spec 2, Plan 7 backlog #6, Part B — D6.B2) to a checked,
/// CPU-neutral [`DataBuf`]. The sibling of [`eval_offsets_with_root`] for
/// `dispatch` items; it REUSES the same [`Cell::RelOffset`] cell for the
/// `word_offsets` encoding — each member emits a `dc.w member_target - Name`
/// word whose `base` is the table's own label (`decl.name`) and whose `target`
/// is the member's referenced symbol, folded at link time to a signed word
/// (`RelWord16Be`). The base label (`decl.name`) is defined at the table's
/// first byte by the caller ([`lower_dispatch_item`](crate::lower)), exactly
/// as for `offsets`.
///
/// Both v1 encodings ship (D6.B2): `word_offsets` emits the `RelOffset` word
/// above; `long_ptrs` emits a 4-byte ABSOLUTE pointer (`dc.l target`, Abs32)
/// per member, reusing the same [`Cell::SymRef`] the struct-data label-pointer
/// field emits. Target extraction, scale/indexing, base label, validation, and
/// the kind check are all encoding-generic; only the cell shape differs.
///
/// Target-name extraction mirrors `eval_offsets_with_root` by SHAPE (a bare
/// `Path` is a symbol name, a `Str` names it directly, anything else is
/// evaluated and its `FnRef`/`Str` name taken). ADDITIONALLY, per D6.B4, a
/// single-segment target that resolves MODULE-LOCALLY to a non-code item
/// (`data`/`const`/`offsets`/`vars`/`dispatch`, recursing one section level
/// via `index_items`) is `[dispatch.target-not-code]` — a dispatch table into
/// data is the jump-to-garbage this construct exists to kill. A name that is
/// unresolvable module-locally (a proc, or a cross-module `use`d target) is
/// left to the linker (v1 does not kind-check cross-module; link fails loudly
/// on a genuinely-undefined symbol). Dup / reserved-`count` validation lives
/// once-per-compile in `lower::validate_dispatch`, not here.
pub fn eval_dispatch_with_root(
    file: &ast::File,
    decl: &ast::DispatchDecl,
    include_root: Option<&std::path::Path>,
) -> (Option<DataBuf>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        let mut buf = DataBuf::empty();
        for member in &decl.members {
            // Fresh env per member (parity with `eval_offsets_with_root`).
            let mut env = Env::new();
            let name = match &member.target {
                // 9a: an inline body's row targets the anonymous proc's
                // hygienic label; it is code by construction, so the
                // [dispatch.target-not-code] kind check does not apply.
                ast::DispatchTarget::Body(_) => {
                    dispatch_body_label(&file.module.path, &decl.name, &member.name)
                }
                ast::DispatchTarget::Label(target) => match target {
                    ast::Expr::Path(p) => {
                        if p.segments.len() == 1 {
                            // D6.B4: module-local kind check. Only a POSITIVELY
                            // non-code item is rejected; a bare name that is a proc
                            // OR is unknown here is accepted and left to link.
                            if let Some(kind) = ev.non_code_kind(&p.segments[0]) {
                                ev.error(
                                    member.span,
                                    format!(
                                        "[dispatch.target-not-code] dispatch `{}` member `{}` targets {kind} `{}` — a dispatch table must point at code",
                                        decl.name, member.name, p.segments[0]
                                    ),
                                );
                                "<unresolved>".to_string()
                            } else {
                                p.segments.join(".")
                            }
                        } else {
                            // Multi-segment (cross-module) target: kind-unchecked in
                            // v1 (D6.B4 ledger note); joined `a.b` for the linker.
                            p.segments.join(".")
                        }
                    }
                    ast::Expr::Str(s, _) => s.clone(),
                    other => {
                        let v = ev.eval_expr(other, &mut env);
                        match v {
                            Value::FnRef(n) => n,
                            Value::Str(s) => s,
                            _ => {
                                ev.error(
                                    crate::parser::expr_span(other),
                                    format!(
                                        "dispatch `{}` member `{}` must reference a label, got {}",
                                        decl.name,
                                        member.name,
                                        v.type_name()
                                    ),
                                );
                                "<unresolved>".to_string()
                            }
                        }
                    }
                },
            };
            // Target extraction above is encoding-generic; only the cell shape
            // differs. `word_offsets` emits a self-relative signed word
            // (`dc.w target - Name`, RelWord16Be) reusing the `offsets`
            // machinery; `long_ptrs` emits a 4-byte ABSOLUTE pointer
            // (`dc.l target`, Abs32) reusing the same `Cell::SymRef` the
            // struct-data label-pointer field emits (`lower_ptr`, emit.rs).
            match decl.encoding {
                ast::DispatchEncoding::WordOffsets => {
                    buf.push(Cell::RelOffset { base: decl.name.clone(), target: name });
                }
                ast::DispatchEncoding::LongPtrs => {
                    buf.push(Cell::SymRef { name, width: 4, windowed: false });
                }
            }
        }
        (Some(buf), ev.diags)
    })
}

/// Force the named SST overlay's layout so its always-on declaration checks
/// (window resolution, capacity, shadow) fire whether or not anything accesses
/// the overlay (Spec 2, Plan 7 #6 — D6.A2 "always-on"). Returns the diagnostics;
/// the overlay itself emits ZERO bytes, so there is no buffer to return. The
/// region form (`vars region { .. }`, `name: None`) is inert — the caller must
/// not invoke this for it.
pub fn validate_overlay(file: &ast::File, name: &str, span: Span) -> Vec<Diagnostic> {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.overlay_layout(name, span);
        ev.diags
    })
}

/// Resolve the SST overlay `name`'s WINDOW against `file`'s own namespace (its
/// DEFINING module), returning the binding (base struct + window offset/size) if
/// it resolves cleanly (Plan 7 #8). Used by the resolve pass to STAMP the binding
/// onto the overlay clone injected into a consumer, so the window travels with the
/// overlay instead of being re-derived from the consumer's structs. Diagnostics
/// are dropped here — the defining module's own `validate_overlay` pass reports
/// them once; a re-report at injection would duplicate. Returns `None` for a
/// poisoned or region-form overlay (the consumer then falls back to today's
/// re-resolution, which for a poisoned overlay is already silent).
pub fn resolve_overlay_window(file: &ast::File, name: &str) -> Option<ast::ResolvedWindow> {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        let span = file.module.span;
        let info = ev.overlay_layout(name, span);
        if info.poisoned {
            return None;
        }
        Some(ast::ResolvedWindow {
            base_struct: info.base_struct,
            window_offset: info.window_offset,
            window_size: info.window_size,
        })
    })
}

/// Evaluate an arbitrary comptime `expr` against `file`'s tables to an integer
/// (§7.1) — used by the lowering pass to resolve a section's `vma:` attribute.
/// Returns `None` (with any diagnostics) if the expression is not a comptime
/// integer.
pub fn eval_attr_int(file: &ast::File, expr: &ast::Expr) -> (Option<i128>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        let mut env = Env::new();
        let v = ev.eval_expr(expr, &mut env);
        // Unreachable today — this evaluator carries no here-position, so a
        // `here()` in an attribute is the "no current position" error before a
        // LinkExpr could form — but front it anyway (D-H.2's specific message)
        // so a future position-threaded attribute cannot silently regress to
        // the generic "not a comptime integer".
        if ev.reject_if_provisional(&v, crate::parser::expr_span(expr)).is_some() {
            return (None, ev.diags);
        }
        (v.as_stored_int(), ev.diags)
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

/// Lay out the bitfield named `name` in `file` (T4), returning its
/// [`BitfieldLayout`] (or `None` if no such bitfield) plus any diagnostics —
/// the bitfield analogue of [`layout_struct`].
pub fn layout_bitfield(file: &ast::File, name: &str) -> (Option<BitfieldLayout>, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        if !ev.bitfields.contains_key(name) {
            ev.error(file.module.span, format!("no bitfield named `{name}`"));
            return (None, ev.diags);
        }
        let layout = ev.layout_of_bitfield(name, file.module.span);
        (Some(layout), ev.diags)
    })
}

/// Resolve `ty` against `file`'s type tables and check `val` against its
/// effective scalar bounds (T4, D-P3.6) — a thin wrapper over
/// [`Evaluator::check_value_fits_ty`] for direct testing, mirroring
/// [`size_of_type`].
pub fn check_value_fits_ty(file: &ast::File, ty: &ast::Type, val: i128) -> (bool, Vec<Diagnostic>) {
    crate::eval::run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        let span = file.module.span;
        let resolved = ev.resolve_type(ty);
        let ok = ev.check_value_fits_ty(&resolved, val, span);
        (ok, ev.diags)
    })
}
