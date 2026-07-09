//! `patch` / `bind` → back-patch + [`Fixup`] (Spec 2, Plan 4 — T5, §6.4,
//! D-P4.10): the emit-forward-bind-later mechanism.
//!
//! A `patch name: T` reserves `sizeof(T)` zero bytes at the current emission
//! point and registers an UNBOUND slot; a later `bind name = expr` (in the same
//! section) fills that slot exactly once — writing the value back into the
//! reserved bytes in the section CPU's byte order (through T2's [`encode_scalar`]
//! so byte order is committed in ONE place) if it is a concrete integer, or
//! recording a [`Fixup`] if it is a symbol the linker resolves. This models the
//! "reserve a hole now, know the value later in the same pass" pattern (a jump
//! table's own end offset, a struct's trailing size, a checksum placeholder).
//!
//! ## Diagnostics
//!
//! - `[patch.unbound]` — a slot reserved but never bound (at section end).
//! - `[patch.double-bound]` — a second `bind` of an already-bound slot.
//! - `[patch.unknown]` — a `bind` naming no reserved slot in this section.
//! - `[patch.unsupported]` — a SYMBOL `bind` at a (width, CPU) with no
//!   representable absolute-fixup kind (see [`fixup_kind`]): a 68k width other
//!   than 2/4, or any Z80 symbol bind (Z80 needs a windowed pointer, not a plain
//!   absolute — deferred with the surface integration).
//! - `[patch.cross-section]` — a `bind` reaching a different section's table;
//!   deferred to the surface integration (T6/T7, see below).
//!
//! ## Emission-context decision (T5, the Part-B design question)
//!
//! `patch` / `bind` parse as [`Stmt`](crate::ast::Stmt)s that appear only inside
//! comptime bodies (`comptime block` / `comptime fn`). Those bodies are executed
//! by the Core-FREE evaluator ([`exec_stmts`](crate::eval)), which yields
//! [`Value`](crate::value::Value)s; a section's bytes are only ever emitted LATER
//! in `lower/` from top-level `data` / `proc` items. There is, in the current
//! surface, NO position where a comptime `patch` / `bind` statement's bytes flow
//! into a section's emission stream (proc / `asm { }` bodies are
//! [`AsmStmt`](crate::ast::AsmStmt)s, which have no `patch` / `bind` form). So
//! this task implements the slot + back-patch as a self-contained LOWERING
//! PRIMITIVE ([`PatchTable`]) and tests it directly, per the plan's fallback.
//!
//! SURFACE-INTEGRATION GAP (flagged for T6/T7): wiring a comptime `patch` /
//! `bind` statement to a live section [`PatchTable`] needs an "emit into the
//! current section" context that only exists once `section(...)` / `vma:`
//! (T6) give comptime code a section-emission position. Until then the
//! `Stmt::Patch` / `Stmt::Bind` arms in `eval/control.rs` stay no-ops, and the
//! `sizeof(T)` → width mapping (via the layout engine) and the
//! `[patch.cross-section]` diagnostic (a `bind` reaching a different section's
//! table) are deferred to that integration — a `bind` against a table that does
//! not hold the slot already surfaces here as `[patch.unknown]`.

use super::data::encode_scalar;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Fixup, FixupKind};
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;

/// The value a `bind name = expr` resolves to: either a concrete integer
/// (back-patched into the reserved bytes) or a symbol (recorded as a [`Fixup`]).
#[derive(Clone, Debug, PartialEq)]
pub enum BindValue {
    /// A resolved integer — written into the slot in the section CPU byte order.
    Int(i128),
    /// A symbol reference — recorded as a linker fixup over the reserved bytes.
    Sym(String),
}

/// One reserved, not-yet-filled slot: where in the fragment its bytes sit, how
/// wide they are, and where the `patch` was written (for the unbound diagnostic).
struct Slot {
    offset: usize,
    width: u8,
    span: Span,
    bound: bool,
}

/// A section's forward-patch table: the running fragment bytes plus the slots a
/// `patch` reserved and a `bind` fills. One [`PatchTable`] models one section's
/// emission stream (which is why cross-section binds are structurally a
/// `[patch.unknown]` here — see the module doc's surface-gap note).
pub struct PatchTable {
    cpu: Cpu,
    bytes: Vec<u8>,
    /// Slots by name.
    slots: HashMap<String, Slot>,
    /// Slot names in `patch` order, so the unbound report is deterministic.
    order: Vec<String>,
    fixups: Vec<Fixup>,
    diags: Vec<Diagnostic>,
}

impl PatchTable {
    /// A fresh table emitting for `cpu`.
    pub fn new(cpu: Cpu) -> Self {
        PatchTable {
            cpu,
            bytes: Vec::new(),
            slots: HashMap::new(),
            order: Vec::new(),
            fixups: Vec::new(),
            diags: Vec::new(),
        }
    }

    /// Emit literal bytes into the fragment (ordinary emission around the
    /// patches, so a test — or a future section stream — can interleave real
    /// data with reserved slots and check offsets line up).
    pub fn emit_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// The fragment's current byte length — the offset the next emission lands at.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// True if nothing has been emitted yet.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// `patch name: T` — reserve `width` (= `sizeof(T)`) zero bytes at the current
    /// offset and register an unbound slot. A repeated `patch` of the same name
    /// keeps the FIRST slot (the later reservation still emits its hole so byte
    /// offsets stay honest, but does not shadow the binding target); a genuine
    /// duplicate-`patch` diagnostic is a surface concern deferred with the rest.
    pub fn patch(&mut self, name: &str, width: u8, span: Span) {
        let offset = self.bytes.len();
        self.bytes.resize(offset + width as usize, 0);
        if !self.slots.contains_key(name) {
            self.slots.insert(name.to_string(), Slot { offset, width, span, bound: false });
            self.order.push(name.to_string());
        }
    }

    /// `bind name = value` — fill the slot named `name` exactly once. Diagnoses
    /// `[patch.unknown]` if no such slot, `[patch.double-bound]` if it was already
    /// bound; otherwise writes an [`Int`](BindValue::Int) back into the reserved
    /// bytes in CPU byte order, or records a [`Sym`](BindValue::Sym) [`Fixup`].
    pub fn bind(&mut self, name: &str, value: BindValue, span: Span) {
        let Some(slot) = self.slots.get_mut(name) else {
            self.diags.push(err(
                span,
                format!("[patch.unknown] `bind {name}` names no reserved patch slot in this section"),
            ));
            return;
        };
        if slot.bound {
            self.diags.push(err(
                span,
                format!("[patch.double-bound] patch slot `{name}` is already bound — a slot may be bound only once"),
            ));
            return;
        }
        slot.bound = true;
        let (offset, width) = (slot.offset, slot.width);
        match value {
            BindValue::Int(n) => {
                // No `u16le` customer in `bind`/`patch` yet (R-T0.1 is data-cell
                // scoped) — always the CPU-driven byte order.
                let encoded = encode_scalar(n, width, self.cpu, false);
                self.bytes[offset..offset + width as usize].copy_from_slice(&encoded);
            }
            BindValue::Sym(target) => match fixup_kind(self.cpu, width) {
                Some(kind) => self.fixups.push(Fixup {
                    kind,
                    offset: offset as u32,
                    target: Expr::Sym(target),
                }),
                None => self.diags.push(err(
                    span,
                    format!(
                        "[patch.unsupported] no width-{width} symbol fixup for a bind in this section"
                    ),
                )),
            },
        }
    }

    /// Finish the table: report every still-unbound slot as `[patch.unbound]`
    /// (at its `patch` span, in `patch` order) and return the fragment bytes, the
    /// recorded fixups, and all diagnostics.
    pub fn finish(mut self) -> (Vec<u8>, Vec<Fixup>, Vec<Diagnostic>) {
        for name in &self.order {
            let slot = &self.slots[name];
            if !slot.bound {
                self.diags.push(err(
                    slot.span,
                    format!("[patch.unbound] patch slot `{name}` was reserved but never bound"),
                ));
            }
        }
        (self.bytes, self.fixups, self.diags)
    }
}

/// The absolute-fixup kind for a width-`width` symbol bind in a `cpu` section.
/// 68k big-endian only for now (the tested path is integer binds); a Z80 / other
/// width has no representable kind here and diagnoses at the call site.
fn fixup_kind(cpu: Cpu, width: u8) -> Option<FixupKind> {
    match (cpu, width) {
        (Cpu::M68000, 4) => Some(FixupKind::Abs32Be),
        (Cpu::M68000, 2) => Some(FixupKind::Abs16Be),
        _ => None,
    }
}

/// Build an error diagnostic at `span`.
fn err(span: Span, message: String) -> Diagnostic {
    Diagnostic { level: Level::Error, message, primary: span }
}
