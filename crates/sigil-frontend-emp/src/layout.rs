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
    pub(crate) fn resolve_type(&mut self, t: &ast::Type, span: Span) -> Ty {
        match t {
            ast::Type::Named(path) => {
                if path.segments.len() != 1 {
                    // Multi-segment paths (module-qualified names) are not
                    // resolvable as data-layout types yet.
                    let full = path.segments.join(".");
                    self.error(span, format!("unknown type: {full}"));
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
                        self.error(span, format!("unknown type: {name}"));
                        Ty::Poison
                    }
                }
            }
            ast::Type::Ptr(inner) => Ty::Ptr(Box::new(self.resolve_type(inner, span))),
            ast::Type::Array(inner, len_expr) => {
                let inner_ty = self.resolve_type(inner, span);
                match self.eval_const_index(len_expr) {
                    Some(n) if n >= 0 => Ty::Array(Box::new(inner_ty), n as usize),
                    Some(n) => {
                        self.error(span, format!("array length must be non-negative, got {n}"));
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
                Ty::Tuple(elems.iter().map(|e| self.resolve_type(e, span)).collect())
            }
            ast::Type::Fixed { i, f } => Ty::Fixed { i: *i, f: *f },
            ast::Type::Refined(inner, lo_expr, hi_expr) => {
                let inner_ty = self.resolve_type(inner, span);
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
            Ty::Array(inner, n) => self.size_of_ty(inner, span) * n,
            Ty::Tuple(elems) => elems.iter().map(|e| self.size_of_ty(e, span)).sum(),
            Ty::Fixed { i, f } => {
                let bits = i + f;
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
            Ty::Newtype(name) => {
                // Copy the underlying `Type` out (borrowed from the file) so
                // `self` is free to be mutated by the recursive resolve/size.
                let decl = self.newtypes.get(name.as_str()).copied();
                match decl {
                    Some(d) => {
                        let underlying = self.resolve_type(&d.underlying, d.span);
                        self.size_of_ty(&underlying, span)
                    }
                    None => 0,
                }
            }
            Ty::Enum(name) => {
                // A plain enum always has a repr; default to `u8` (1) if absent.
                let decl = self.enums.get(name.as_str()).copied();
                match decl.and_then(|d| d.repr.as_ref()) {
                    Some(repr) => {
                        let repr_ty = self.resolve_type(repr, span);
                        self.size_of_ty(&repr_ty, span)
                    }
                    None => 1,
                }
            }
            Ty::Bitfield(name) => {
                let decl = self.bitfields.get(name.as_str()).copied();
                match decl {
                    Some(d) => {
                        let repr_ty = self.resolve_type(&d.repr, d.span);
                        self.size_of_ty(&repr_ty, span)
                    }
                    None => 0,
                }
            }
            Ty::Struct(name) => self.layout_of_struct(name, span).size,
            Ty::Poison => 0,
        }
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
            // Name the cycle as the chain from first entry to this repeat.
            let mut chain: Vec<&str> =
                self.layout_in_progress[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            self.error(span, format!("cyclic struct layout: {}", chain.join(" -> ")));
            let poisoned = Layout { size: 0, fields: Vec::new() };
            self.struct_layout_memo.insert(name.to_string(), poisoned.clone());
            return poisoned;
        }
        // Copy the `&'a StructDecl` out of the index so its fields are borrowed
        // from the file (lifetime `'a`), leaving `self` free to be mutated
        // across the recursive resolve/size calls below (as `resolve_const`).
        let decl: &'a ast::StructDecl = match self.structs.get(name).copied() {
            Some(d) => d,
            None => {
                // Not a known struct — a Poisoned (zero-size) layout. The
                // unknown-type diagnostic is the caller's (resolve_type's) job.
                return Layout { size: 0, fields: Vec::new() };
            }
        };
        self.layout_in_progress.push(name.to_string());
        let mut offset = 0usize;
        let mut fields = Vec::with_capacity(decl.fields.len());
        for field in &decl.fields {
            let ty = self.resolve_type(&field.ty, field.span);
            let size = self.size_of_ty(&ty, field.span);
            fields.push(FieldLayout { name: field.name.clone(), offset, ty, size });
            offset += size;
        }
        self.layout_in_progress.pop();
        let layout = Layout { size: offset, fields };
        self.struct_layout_memo.insert(name.to_string(), layout.clone());
        layout
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
pub fn size_of_type(file: &ast::File, ty: &ast::Type) -> (usize, Vec<Diagnostic>) {
    let mut ev = Evaluator::with_file(file);
    let span = file.module.span;
    let resolved = ev.resolve_type(ty, span);
    let size = ev.size_of_ty(&resolved, span);
    (size, ev.diags)
}

/// Lay out the struct named `name` in `file`, returning its [`Layout`] (or
/// `None` if no such struct) plus any diagnostics.
pub fn layout_struct(file: &ast::File, name: &str) -> (Option<Layout>, Vec<Diagnostic>) {
    let mut ev = Evaluator::with_file(file);
    if !ev.structs.contains_key(name) {
        ev.error(file.module.span, format!("no struct named `{name}`"));
        return (None, ev.diags);
    }
    let layout = ev.layout_of_struct(name, file.module.span);
    (Some(layout), ev.diags)
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
