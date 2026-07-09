//! Comptime evaluator (Spec 2, Plan 2): the lexical [`Env`] and the
//! [`Evaluator`] state, split across focused (private) submodules —
//! `env` (the scope chain), `expr` (pure expression evaluation),
//! `control` (statement execution / control flow), `call` (`comptime fn`
//! calls and applying callables), `builtins` (the §6.8 builtin methods),
//! and `guards` (`ensure`/`ensure_fatal` and string interpolation).
//!
//! This module (`mod.rs`) owns the [`Evaluator`] struct itself, its
//! constructors, and the crate's top-level entry points ([`eval_const`]);
//! the submodules contribute method groups via additional `impl Evaluator`
//! blocks.
mod asm;
mod builtins;
mod call;
mod classic_compress;
mod compress_common;
mod control;
mod emit;
mod env;
mod expr;
mod float_ns;
pub(crate) mod guards;
mod literals;
mod pattern;
mod s4lz;
mod sandbox;
mod typed;

pub use env::{AssignError, Binding, Env};

use crate::ast;
use crate::value::Value;
use sigil_span::{Diagnostic, Level, Span};
use std::collections::HashMap;
use std::path::PathBuf;

/// Comptime step budget (D-P2.7): a coarse upper bound on evaluation work,
/// guarding against runaway loops/recursion. Later tasks act on exhaustion.
pub const STEP_BUDGET: u64 = 5_000_000;

/// Maximum comptime-fn call depth (D-P2.16). A hard bound below the native
/// stack limit so unbounded recursion is caught and *named* (see
/// [`Evaluator::abort`]) instead of overflowing the process stack.
pub const MAX_CALL_DEPTH: usize = 512;

/// How many innermost call-stack frames an abort message names before it
/// truncates with a leading `...` (keeps a deep repeated chain readable).
const MAX_CHAIN_FRAMES: usize = 12;

/// A control-flow signal threaded out of [`Evaluator::exec_stmts`].
///
/// A statement block either falls off its end (`Normal`, carrying the block's
/// value — the last bare expression statement's value, or `Unit`) or hits an
/// explicit `return` (`Return`, carrying the returned value, which stops the
/// block and bubbles up to the enclosing `comptime fn` boundary).
enum Flow {
    /// The block ran to completion; the payload is its trailing value.
    Normal(Value),
    /// An explicit `return` fired; the payload is the returned value.
    Return(Value),
}

/// The comptime evaluator's mutable state, threaded through evaluation.
///
/// The `'a` lifetime ties the evaluator to a borrowed [`ast::File`]'s items:
/// [`Evaluator::with_file`] indexes the file's `const` and `enum` decls so bare
/// names and `Enum.Variant` paths resolve to them. [`Value`] carries no lifetime
/// (a [`Value::Lambda`] owns its body), so borrowing the file here is free of
/// friction with the `&mut self` mutation during evaluation — the borrowed
/// index is a distinct object from the mutated `diags`/memo.
///
/// [`Evaluator::new`] builds the empty-program evaluator (no file): in that mode
/// there are no file consts/enums, so unknown names still error. This keeps the
/// T2 pure-expression tests working unchanged.
pub struct Evaluator<'a> {
    /// Diagnostics collected during evaluation.
    pub diags: Vec<Diagnostic>,
    /// Steps consumed so far, capped by [`STEP_BUDGET`].
    pub steps: u64,
    /// The active call stack as `(fn name, call-site span)`, for budget and
    /// recursion-cycle reporting in later tasks.
    pub call_stack: Vec<(String, Span)>,
    /// File-level `const` decls, indexed by name (empty in the no-file mode).
    consts: HashMap<&'a str, &'a ast::ConstDecl>,
    /// File-level `equ` decls, indexed by name (empty in the no-file mode).
    /// Resolved through the SAME lazy/memoized/cycle-guarded path as
    /// [`consts`](Self::consts) (R-T0.2 — an equ's value is a comptime
    /// int or a link-time expression, resolved identically to a const's;
    /// the only semantic difference is what a later lowering task does with
    /// the result).
    equs: HashMap<&'a str, &'a ast::EquDecl>,
    /// File-level `enum` decls, indexed by name (empty in the no-file mode).
    /// `pub(crate)` so the [`layout`](crate::layout) module can size enum reprs.
    pub(crate) enums: HashMap<&'a str, &'a ast::EnumDecl>,
    /// File-level `comptime fn` decls, indexed by name (empty in no-file mode).
    fns: HashMap<&'a str, &'a ast::ComptimeFnDecl>,
    /// File-level `struct` decls, indexed by name (empty in the no-file mode).
    /// `pub(crate)` for the [`layout`](crate::layout) module (T2).
    pub(crate) structs: HashMap<&'a str, &'a ast::StructDecl>,
    /// File-level `bitfield` decls, indexed by name (empty in no-file mode).
    /// `pub(crate)` for the [`layout`](crate::layout) module (T2).
    pub(crate) bitfields: HashMap<&'a str, &'a ast::BitfieldDecl>,
    /// File-level `newtype` decls, indexed by name (empty in no-file mode).
    /// `pub(crate)` for the [`layout`](crate::layout) module (T2).
    pub(crate) newtypes: HashMap<&'a str, &'a ast::NewtypeDecl>,
    /// File-level `data` decls, indexed by name (empty in no-file mode). T7's
    /// [`resolve_data`](Self::resolve_data) lowers these to a checked
    /// [`DataBuf`](crate::value::DataBuf).
    pub(crate) datas: HashMap<&'a str, &'a ast::DataDecl>,
    /// File-level `offsets` decls, indexed by name (empty in no-file mode).
    /// Spec 2 Plan 7 backlog #3 (reverse direction): [`eval::expr`](expr)'s
    /// `eval_path` resolves `Name.Variant` to the member's 0-based ordinal and
    /// `Name.count` to `decl.members.len()` — plain comptime ints, mirroring how
    /// [`enums`](Self::enums) resolves `Enum.Variant`. Forward emission
    /// (`dc.w target - Name`) is a separate, later task.
    pub(crate) offsets: HashMap<&'a str, &'a ast::OffsetsDecl>,
    /// File-level `dispatch` decls, indexed by name (empty in no-file mode).
    /// Spec 2 Plan 7 backlog #6, Part B (reverse direction): [`eval::expr`](expr)'s
    /// `eval_path` resolves `Name.Member` to the member's ordinal PRE-SCALED by
    /// the encoding (×2 for `word_offsets`, ×4 for `long_ptrs` — D6.B3) and
    /// `Name.count` to `decl.members.len()` UNSCALED. Plain comptime ints,
    /// mirroring how [`offsets`](Self::offsets) resolves `Name.Variant`. Also
    /// consulted by the forward-emission target kind check
    /// ([`is_data`](Self::is_data)/[`is_dispatch`](Self::is_dispatch)).
    pub(crate) dispatches: HashMap<&'a str, &'a ast::DispatchDecl>,
    /// Named SST overlay decls (`vars Name: window { .. }`), indexed by name
    /// (Spec 2, Plan 7 #6, Part A). Only the *named* overlay form is indexed;
    /// the region form (`vars region { .. }`, `name: None`) stays inert.
    /// [`overlay_layout`](crate::layout::Evaluator::overlay_layout) resolves each
    /// to an [`OverlayInfo`](crate::layout::OverlayInfo) (window + field layout).
    pub(crate) overlays: HashMap<&'a str, &'a ast::VarsDecl>,
    /// Memoized struct layouts, keyed by struct name — the layout analogue of
    /// [`const_memo`](Self::const_memo). A zero-size Poisoned layout records a
    /// struct that already failed (cyclic layout) so it does not re-report.
    pub(crate) struct_layout_memo: HashMap<String, crate::layout::Layout>,
    /// Memoized overlay layouts, keyed by overlay name (Spec 2, Plan 7 #6).
    /// Mirrors [`struct_layout_memo`](Self::struct_layout_memo): each overlay's
    /// window resolution + field layout + declaration checks
    /// (overflow/shadow/unknown/ambiguous window) run and report EXACTLY once,
    /// then the result (poisoned or not) is reused across every reference.
    pub(crate) overlay_layout_memo: HashMap<String, crate::layout::OverlayInfo>,
    /// Memoized bitfield layouts, keyed by bitfield name (T4). Mirrors
    /// [`struct_layout_memo`](Self::struct_layout_memo): a malformed bitfield
    /// (overlap/overflow) is validated and diagnosed exactly ONCE, then reused
    /// across every literal that references it within one evaluation.
    pub(crate) bitfield_layout_memo: HashMap<String, crate::layout::BitfieldLayout>,
    /// The names of structs whose layout is currently being computed, in
    /// reference order — the in-progress stack for cyclic-layout detection,
    /// mirroring [`in_progress`](Self::in_progress) for consts.
    pub(crate) layout_in_progress: Vec<String>,
    /// Set once a hard limit (step budget or call depth) is hit (D-P2.16). While
    /// set, [`eval_expr`](Evaluator::eval_expr) / [`exec_stmts`](Evaluator::exec_stmts)
    /// short-circuit to `Poison` so evaluation unwinds without further work or
    /// diagnostics.
    aborted: bool,
    /// A `return` that fired inside an *expression-position* `if` and must still
    /// exit the enclosing fn. `eval_expr` sets it; the next `exec_stmts` step
    /// picks it up and turns it into a [`Flow::Return`]. (Statement-position
    /// `return`/`if` never need this — they flow through `exec_stmts` directly.)
    ///
    /// INVARIANT: every statement arm that evaluates an operand MUST route it
    /// through [`eval_operand`](Evaluator::eval_operand) and bail on
    /// `Err(Flow::Return)`. Bypassing that check lets a caller's pending return
    /// leak into a callee (the call-arg return-leak bug class).
    pending_return: Option<Value>,
    /// Depth of enclosing comptime-mutable contexts (D-P2.5). A `comptime var`
    /// and its reassignment are only legal where this is non-zero: inside a
    /// `comptime fn` body (bumped in [`eval_call`](Evaluator::eval_call)) or a
    /// nested `comptime block { }` (bumped in the [`Stmt::ComptimeBlock`] arm).
    /// Module-level `const` value expressions run with `comptime_ctx == 0`, so
    /// they have no mutable state.
    comptime_ctx: u32,
    /// Memoized const values, keyed by const name. A `Poison` entry records a
    /// const that already failed (cycle or error) so the failure does not
    /// re-report on subsequent references.
    const_memo: HashMap<String, Value>,
    /// Comptime `-D NAME=INT` defines currently in scope (sound-migration T2
    /// Task 1, R1), keyed by name. Populated once, up front, by
    /// [`seed_defines`](Self::seed_defines) — NOT incrementally like
    /// [`const_memo`](Self::const_memo). This map IS the resolution mechanism:
    /// `eval_path`'s bare-name lookup falls back to it (after locals and
    /// consts/equs) and returns the `Value::Int` directly — a define has no
    /// backing `ast::ConstDecl` to index into [`consts`](Self::consts), so it
    /// never routes through `resolve_const`/`const_memo` at all.
    defines: HashMap<String, i128>,
    /// The names of consts whose value expressions are currently being
    /// evaluated, in reference order — the in-progress stack used to detect and
    /// name cyclic const definitions.
    in_progress: Vec<String>,
    /// Memoized data-item buffers, keyed by data-item name (T7) — the data
    /// analogue of [`const_memo`](Self::const_memo). Data items cannot reference
    /// each other as values in Plan 3 (only consts are name-resolvable), so this
    /// is a plain memo with no cycle machinery; it exists so a shared evaluator
    /// answers repeated queries once.
    data_memo: HashMap<String, crate::value::DataBuf>,
    /// The names of structs whose CHECKED LITERAL is currently being constructed,
    /// in reference order (T7). A struct field's `= default` expression can
    /// construct the same struct (`struct A { x: A = A{} }`), which would recurse
    /// forever — this stack is DISTINCT from [`layout_in_progress`](Self::layout_in_progress)
    /// (which guards layout sizing) because `eval_checked_struct_lit` also *calls*
    /// `layout_of_struct`, so the two must not share a stack.
    pub(crate) struct_construct_in_progress: Vec<String>,
    /// The names of newtypes whose `where` refinement bound is currently being
    /// evaluated, in reference order (T8 whole-branch review, Critical). A
    /// newtype's bound can itself construct the SAME newtype
    /// (`newtype N = u8 where 0 .. N(2)`); validating a `N(x)` then re-enters the
    /// bound eval, which re-enters `N(...)`, recursing without bound and aborting
    /// the process with a native stack overflow. This stack is DISTINCT from
    /// [`layout_in_progress`](Self::layout_in_progress) on purpose: reusing the
    /// layout stack would flag the LEGITIMATE `where 0 .. sizeof(S)` /
    /// `struct S { x: N }` pattern (a size/layout re-entrancy, which shares
    /// `layout_in_progress`) as a false cycle. Construction/refinement re-entrancy
    /// and size/layout re-entrancy are genuinely different concerns.
    pub(crate) refine_check_in_progress: Vec<String>,
    /// Monotonic instantiation counter for `asm { }` / `proc` label hygiene
    /// (D-P4.6). Each `eval_asm` evaluation takes a fresh id `k`; the
    /// [`LabelScope`](crate::lower::hygiene::LabelScope) built from it renames
    /// non-`export` local labels to unique symbols (`$asm{k}$name`) so two
    /// instantiations of the same template never collide, while references to a
    /// `.name` label WITHIN one instantiation rewrite to the same fresh symbol so
    /// intra-body branches still resolve. Exported labels take the stable,
    /// caller-visible `Owner.name` spelling instead (§5.2) — the owner is the
    /// proc name for a proc body and `k` for a raw `asm { }` (T5).
    asm_counter: u32,
    /// The enclosing module's dotted id (`a.b.c`, the `module` header path),
    /// empty in the no-file [`new`](Self::new) mode. Threaded into every label
    /// hygiene [`Owner`](crate::lower::hygiene::Owner) so a hidden local symbol is
    /// unique across the whole multi-module program (Plan 7 #4): the proc name /
    /// instantiation id `k` are only unique WITHIN a module, so two modules with a
    /// `proc init` (or an `asm {}` whose `k` reset to the same value) would
    /// otherwise mint colliding `$init$loop` / `$asm{k}$wait` symbols. Set from
    /// the file in [`with_file`](Self::with_file).
    module_id: String,
    /// The VMA the `here()` comptime builtin resolves to (§7.1), or `None` when
    /// no position is known (outside lowering). Set per data-item by the lowering
    /// pass to `vma_origin + current_offset` — the VMA at the START of the item
    /// being lowered. `here()` is a lowering-time query the pure evaluator cannot
    /// answer on its own, so the position is threaded in here. LIMITATION: it is
    /// the item's start VMA, so a `here()` mid-buffer (after some bytes in the
    /// SAME data item) still reads the item start, not the advanced position.
    here_base: Option<u32>,
    /// The ANCHOR LABEL a PROVISIONAL `here()` resolves to (D-H.1/D-H.2), or
    /// `None` at an EXACT position (where `here()` returns `Value::Int(here_base)`,
    /// byte-identical to today). Set by the lowering pass alongside
    /// [`here_base`](Self::here_base) via [`set_here_provisional`](Self::set_here_provisional)
    /// when the currently-open section already contains a size-relaxable fragment
    /// (`IrBuilder::section_has_relaxable`). When `Some(anchor)`, `here()` yields
    /// `Value::LinkExpr(Sym(anchor))` — a link-time value the linker resolves to
    /// the anchor's post-relaxation VMA — and sets [`here_used`](Self::here_used)
    /// so the lowering pass knows to actually define the anchor label (D-H.8: a
    /// guard-position anchor is minted only when `here()` was used).
    here_anchor: Option<String>,
    /// Deferred link-time assertions (D-H.4): `ensure`/`ensure_fatal` guards whose
    /// condition evaluated to a provisional `here()` [`LinkExpr`](Value::LinkExpr).
    /// `eval_guard` records one here instead of passing/failing the guard at
    /// comptime; the lowering pass drains them (like `diags`) onto the module.
    link_asserts: Vec<sigil_ir::LinkAssert>,
    /// Set true the first time a PROVISIONAL `here()` is evaluated (D-H.8): it
    /// records that the anchor label named by [`here_anchor`](Self::here_anchor)
    /// is actually referenced, so the lowering pass defines it (an item guard's
    /// anonymous anchor is minted only on use, keeping `--map`/symbol tables clean).
    here_used: bool,
    /// The capability-sandbox root (Spec 2, Plan 5 — Task 1): the HARD
    /// containment boundary a resolved `embed`/`import` path must stay
    /// inside. `None` outside a rooted evaluation (e.g. the plain
    /// [`eval_const`] entry point), in which case a comptime file read is
    /// `[sandbox.no-root]`. Set via
    /// [`set_include_root`](Self::set_include_root) by the
    /// [`layout::eval_data_with_root`](crate::layout::eval_data_with_root) seam.
    include_root: Option<PathBuf>,
    /// The join BASE a relative `embed`/`import` path resolves against
    /// (port #2, `math.emp`'s `embed("../data/sine.bin")`): distinct from
    /// `include_root`, which stays the boundary. `None` means "same as
    /// `include_root`" (every pre-existing caller's behavior, unchanged) —
    /// `resolve_sandbox_path` falls back to `include_root` when this is
    /// unset. Lets a module whose `embed` path climbs ABOVE its own
    /// directory (to a sibling directory still inside the broader
    /// `include_root` tree) resolve, without weakening the containment
    /// check itself, which always runs against `include_root` alone. Set via
    /// [`set_embed_base`](Self::set_embed_base).
    embed_base: Option<PathBuf>,
    /// The capture ledger (Task 1): one [`sandbox::CaptureEdge`] per comptime
    /// file read (`embed`, and later `import`/`zx0`), recording the resolved
    /// path, its SHA-256 digest, and its byte length — the provenance record a
    /// later hermeticity task exposes and asserts determinism from.
    pub(crate) captures: Vec<sandbox::CaptureEdge>,
    /// The struct a proc-body param's address register points at (Spec 2, Plan 7
    /// #6, Part A — D6.A3). Populated per-proc by [`eval_proc_body`] from each
    /// `(aN: *S)` param whose pointee bottoms out (through newtype/refined) at a
    /// struct `S`; a non-struct pointee (`*u8`) never enters the map. A bare
    /// single-segment displacement `f(aN)` on a register present here resolves in
    /// FIELD SPACE (direct fields ∪ in-scope overlays over `S`), not as a comptime
    /// expression — so a field name lowers to its byte offset and a const never
    /// silently shadows it. Empty for a raw `asm { }` (no param types there).
    pub(crate) reg_pointee_struct: HashMap<crate::value::Reg, String>,
    /// The CPU this evaluation lowers for, when known (D-PP.1). `Some` only when
    /// evaluating a PROC BODY (threaded in by [`eval_proc_body`] from the
    /// enclosing section's CPU), where the mnemonic-vs-comptime-fn decision for a
    /// bare statement call needs the section's mnemonic table. `None` for a raw
    /// `asm { }` template eval (whose CPU is not fixed until it is spliced into a
    /// section) — a bare statement call is a proc-body construct, so it is only
    /// recognized when this is `Some`.
    cpu: Option<sigil_ir::backend::Cpu>,
    /// Depth of enclosing LABEL-VALUE contexts (D-PP.3). Non-zero only while
    /// evaluating a data-item field initializer or a call argument (bumped by
    /// [`in_label_ctx`](Self::in_label_ctx)); an otherwise-unknown bareword /
    /// dotted path in [`eval_path`](Self::eval_path) becomes a
    /// [`Value::Label`](crate::value::Value::Label) only when this is non-zero,
    /// so a pure comptime expression (`const x = bogus`) keeps its loud
    /// `unknown name`. A COUNTER (not a bool) so nested value positions restore
    /// correctly.
    label_ctx: u32,
    /// Cross-module type-only records for `pub data` items of struct type
    /// (D-PP.5): item name → struct type name. A `pub data Player_1: Sst` emits
    /// bytes, so — unlike a `pub struct`/`const` — it is NOT injected into a
    /// consumer's item list. Instead the resolver injects a TYPE-ONLY clone
    /// (`data.type_only = true`, no bytes) and its `(name → struct)` binding is
    /// stamped HERE at the DEFINING module (mirroring the T0a overlay-window
    /// stamp), so a consumer's `Player_1.field` field-address operand knows the
    /// struct type without the item's initializer being visible. Populated by
    /// [`index_items`](Self::index_items) from every `type_only` data item.
    /// Empty in single-file mode (a local `data` is in [`datas`] instead).
    ///
    /// ADDRESS-half only: the value-read half (`Def.field`) needs the
    /// initializer, which a type-only import does not carry — so a cross-module
    /// VALUE read is a loud not-supported diagnostic, per the spec.
    imported_item_types: HashMap<String, String>,
    /// Memoized comptime VALUEs of module-local data items (D-PP.5, HALF B), keyed
    /// by item name — the value analogue of [`const_memo`](Self::const_memo) for
    /// the `Def.field` value read. A `Poison` entry records an item whose
    /// initializer already failed (a cycle or error) so it does not re-report.
    data_value_memo: HashMap<String, Value>,
    /// The names of data items whose initializer is currently being evaluated for
    /// a VALUE read (D-PP.5), in reference order — the in-progress stack for the
    /// `data A = S{x: B.x}` ↔ `data B = S{x: A.x}` cycle. DISTINCT from
    /// [`in_progress`](Self::in_progress) (consts): a data item and a const can
    /// share a name space but their value-eval recursions are separate concerns,
    /// and mixing the stacks would mis-name a cycle chain.
    data_value_in_progress: Vec<String>,
}

impl<'a> Evaluator<'a> {
    /// Create a fresh evaluator with no file context: an empty diagnostic list,
    /// step count, and const/enum index. Bare names resolve only against the
    /// local [`Env`]; there are no file-level consts or enums to fall back to.
    pub fn new() -> Self {
        Evaluator {
            diags: Vec::new(),
            steps: 0,
            call_stack: Vec::new(),
            consts: HashMap::new(),
            equs: HashMap::new(),
            enums: HashMap::new(),
            fns: HashMap::new(),
            structs: HashMap::new(),
            bitfields: HashMap::new(),
            newtypes: HashMap::new(),
            datas: HashMap::new(),
            offsets: HashMap::new(),
            dispatches: HashMap::new(),
            overlays: HashMap::new(),
            struct_layout_memo: HashMap::new(),
            overlay_layout_memo: HashMap::new(),
            bitfield_layout_memo: HashMap::new(),
            layout_in_progress: Vec::new(),
            aborted: false,
            pending_return: None,
            comptime_ctx: 0,
            const_memo: HashMap::new(),
            defines: HashMap::new(),
            in_progress: Vec::new(),
            data_memo: HashMap::new(),
            struct_construct_in_progress: Vec::new(),
            refine_check_in_progress: Vec::new(),
            asm_counter: 0,
            module_id: String::new(),
            here_base: None,
            here_anchor: None,
            here_used: false,
            link_asserts: Vec::new(),
            include_root: None,
            embed_base: None,
            captures: Vec::new(),
            reg_pointee_struct: HashMap::new(),
            cpu: None,
            label_ctx: 0,
            imported_item_types: HashMap::new(),
            data_value_memo: HashMap::new(),
            data_value_in_progress: Vec::new(),
        }
    }

    /// Run `body` with label-value resolution ENABLED (D-PP.3), restoring the
    /// prior depth on the way out. In this scope an otherwise-unknown bareword /
    /// dotted path in [`eval_path`](Self::eval_path) becomes a
    /// [`Value::Label`](crate::value::Value::Label) (a deferred link symbol)
    /// instead of the `unknown name` error — the fallback is confined to the two
    /// comptime VALUE positions the spec names (data-item field initializers and
    /// call arguments), so a pure comptime expression context keeps its loud
    /// `unknown name`. A DEPTH counter (not a bool) so nesting — a call-arg whose
    /// value is a struct literal whose field is a bareword — stays enabled
    /// throughout and restores correctly.
    ///
    /// The counter therefore PROPAGATES into every expression nested under a
    /// wrapped position: an array literal INSIDE a struct field
    /// (`E{ table: [a, b] }`) resolves its elements as labels, while a TOP-LEVEL
    /// data-item array initializer (`data D: [*u8; 2] = [a, b]`) is never
    /// wrapped and keeps the loud `unknown name` — pinned by
    /// `tests/label_values.rs` (U4 stacks on this exact boundary).
    pub(super) fn in_label_ctx<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        self.label_ctx += 1;
        let out = body(self);
        self.label_ctx -= 1;
        out
    }

    /// Whether label-value resolution is currently enabled (inside a data-item
    /// field initializer or a call argument, D-PP.3). The fallback in
    /// [`eval_path`](Self::eval_path) consults this so a bareword only becomes a
    /// label where a symbol reference is meaningful.
    pub(super) fn label_ctx_active(&self) -> bool {
        self.label_ctx > 0
    }

    /// Set the VMA `here()` resolves to for the item about to be evaluated
    /// (§7.1), at an EXACT position — `here()` yields `Value::Int(vma)`
    /// byte-identically to before this fix. The lowering pass calls this before
    /// resolving each data item / guard whose section holds no relaxable fragment.
    pub(crate) fn set_here_base(&mut self, vma: u32) {
        self.here_base = Some(vma);
        self.here_anchor = None;
    }

    /// Set a PROVISIONAL `here()` position (D-H.1): the fallback baseline `vma`
    /// (for diagnostics only) plus the `anchor` label whose post-relaxation VMA
    /// `here()` resolves to. `here()` then yields `Value::LinkExpr(Sym(anchor))`
    /// and marks the anchor USED. Called by the lowering pass when the open
    /// section already contains a size-relaxable fragment.
    pub(crate) fn set_here_provisional(&mut self, vma: u32, anchor: String) {
        self.here_base = Some(vma);
        self.here_anchor = Some(anchor);
    }

    /// Whether a provisional `here()` was actually evaluated (D-H.8) — the signal
    /// the lowering pass uses to define an item-guard's anonymous anchor label
    /// only on use. Consumed by the deferred-guard path (D-H.4, T4).
    pub(crate) fn here_anchor_used(&self) -> bool {
        self.here_used
    }

    /// Take the deferred link-time assertions collected during this evaluation
    /// (D-H.4), leaving the evaluator's list empty. The lowering pass drains these
    /// onto the module (like `diags`).
    pub(crate) fn take_link_asserts(&mut self) -> Vec<sigil_ir::LinkAssert> {
        std::mem::take(&mut self.link_asserts)
    }

    /// Apply a [`HerePos`](crate::layout::HerePos): an exact position sets a bare
    /// `here_base` (`here()` → `Value::Int`); a provisional one sets the anchor
    /// too (`here()` → `Value::LinkExpr`). `None` leaves `here()` an error. The
    /// single seam every `eval_data*`/guard entry point routes its position
    /// through, so the exact/provisional split is applied identically everywhere.
    pub(crate) fn apply_here_pos(&mut self, here: Option<crate::layout::HerePos>) {
        match here {
            Some(crate::layout::HerePos { base, anchor: Some(a) }) => {
                self.set_here_provisional(base, a)
            }
            Some(crate::layout::HerePos { base, anchor: None }) => self.set_here_base(base),
            None => {}
        }
    }

    /// Whether evaluation aborted (a failing `ensure_fatal`, D5.3). The item-guard
    /// harness reads this to decide whether to keep lowering the module's items.
    pub(crate) fn was_aborted(&self) -> bool {
        self.aborted
    }

    /// Create an evaluator that can resolve names against `file`'s top-level
    /// `const` and `enum` items. Later duplicate names (a parse-level concern)
    /// are resolved last-wins by the index build; duplicate diagnosis is not
    /// this task's job.
    pub fn with_file(file: &'a ast::File) -> Self {
        let mut ev = Evaluator::new();
        // The module id qualifies hidden label-hygiene locals so they are unique
        // across the whole multi-module program (Plan 7 #4).
        ev.module_id = file.module.path.segments.join(".");
        ev.index_items(&file.items);
        ev
    }

    /// Register a list of items into the name indexes, recursing into
    /// `section { ... }` blocks (§7.1) so a section-nested `data`/`const`/`fn`/…
    /// is name-resolvable exactly like a top-level one (a flat namespace, as in
    /// the AS model). Section placement itself is handled later, by the lowering
    /// pass — this is purely the evaluator's name resolution.
    fn index_items(&mut self, items: &'a [ast::Item]) {
        for item in items {
            match item {
                ast::Item::Const(c) => {
                    self.consts.insert(c.name.as_str(), c);
                }
                ast::Item::Equ(e) => {
                    self.equs.insert(e.name.as_str(), e);
                }
                ast::Item::Enum(e) => {
                    self.enums.insert(e.name.as_str(), e);
                }
                ast::Item::ComptimeFn(f) => {
                    self.fns.insert(f.name.as_str(), f);
                }
                ast::Item::Struct(s) => {
                    self.structs.insert(s.name.as_str(), s);
                }
                ast::Item::Bitfield(b) => {
                    self.bitfields.insert(b.name.as_str(), b);
                }
                ast::Item::Newtype(n) => {
                    self.newtypes.insert(n.name.as_str(), n);
                }
                ast::Item::Data(d) => {
                    if d.type_only {
                        // A cross-module TYPE-ONLY import (D-PP.5): record only its
                        // (name → struct) binding for the field-ADDRESS operand; it
                        // carries no real initializer, so it must NOT enter `datas`
                        // (which drives value reads and byte emission). The struct
                        // type it names lives in the `ty` annotation the resolver
                        // preserved on the clone.
                        if let Some(ast::Type::Named(p)) = &d.ty {
                            if p.segments.len() == 1 {
                                self.imported_item_types
                                    .insert(d.name.clone(), p.segments[0].clone());
                            }
                        }
                    } else {
                        self.datas.insert(d.name.as_str(), d);
                    }
                }
                ast::Item::Offsets(o) => {
                    // Index for `Name.Variant` / `Name.count` resolution in
                    // `eval_path`. Every per-item evaluator re-indexes the whole
                    // file, so this MUST stay here — but duplicate-member ERROR
                    // reporting must NOT (it would fire once per per-item
                    // evaluator); that check lives once-per-compile in
                    // `lower::validate_offsets`.
                    self.offsets.insert(o.name.as_str(), o);
                }
                ast::Item::Dispatch(d) => {
                    // Index for `Name.Member` / `Name.count` resolution in
                    // `eval_path` and the target kind check. Duplicate-member /
                    // reserved-`count` ERROR reporting lives once-per-compile in
                    // `lower::validate_dispatch`, NOT here (this re-indexes on
                    // every per-item evaluator, mirroring the `Offsets` arm).
                    self.dispatches.insert(d.name.as_str(), d);
                }
                ast::Item::Vars(v) => {
                    // Only the NAMED overlay form (`vars Name: window { .. }`)
                    // is indexed; the region form (`name: None`) is inert by
                    // design (Plan 7 #6 OUT-list — no region allocation).
                    if let Some(name) = &v.name {
                        self.overlays.insert(name.as_str(), v);
                    }
                }
                ast::Item::Section(s) => self.index_items(&s.items),
                _ => {}
            }
        }
    }

    /// Whether `name` is a file-level `const` (the same table `eval_path`
    /// consults). Exposed for `offsets` forward lowering
    /// ([`eval_offsets_with_root`](crate::layout::eval_offsets_with_root)) to
    /// give a const-alias target a clear early diagnostic — a `const` is not a
    /// valid offsets label. A read-only membership check, so the `consts` field
    /// itself stays private.
    pub(crate) fn is_const(&self, name: &str) -> bool {
        self.consts.contains_key(name)
    }

    /// Whether `name` is a file-level `data` item (module-local, section-nested
    /// one level via `index_items`). Used by the `dispatch` forward-emission
    /// target kind check (D6.B4): a member targeting a `data` item is
    /// `[dispatch.target-not-code]`.
    pub(crate) fn is_data(&self, name: &str) -> bool {
        self.datas.contains_key(name)
    }

    /// The STRUCT type name a data item `name` bottoms out at, if it is a known
    /// data item of struct type (D-PP.5). Peeled through newtype/refined wrappers
    /// by [`struct_name_for_offsetof`](Self::struct_name_for_offsetof) exactly as
    /// `offsetof(T, f)` is, so a `data P: Newtype = …` whose underlying is a
    /// struct still resolves. The type comes from the explicit `ty` annotation, or
    /// — absent one — from a struct-literal initializer that names its own type
    /// (§4.5, mirroring `resolve_data`'s inference). `None` for an unknown name, a
    /// non-struct-typed item, or a type-only import (whose bytes/init are not
    /// visible — see [`imported_item_types`](Self::imported_item_types) for the
    /// cross-module ADDRESS path).
    ///
    /// Resolution MUST NOT report: a bad type is diagnosed at the item's own decl
    /// site, so this probes silently and swallows any diagnostics it provokes —
    /// duplicating them here would double-report. The caller decides what a
    /// `None` means (fall through to a link symbol, or error) at its own span.
    pub(crate) fn data_item_struct_name(&mut self, name: &str) -> Option<String> {
        // The annotation `Type` to resolve. The LOCAL data item wins over a
        // cross-module type-only import under a name shadow (spec-review
        // ISSUE-1): the operand's base symbol resolves to the local item's
        // label, so its OFFSETS must come from the local item's struct too —
        // consulting the imported stub first would fuse the local base with the
        // FOREIGN struct's field table, a silently-wrong address (and disagree
        // with the value half, which already reads the local item). Local-wins
        // also matches the value ladder's precedence everywhere else; a
        // shadow-warning lint is ledger material. Only when there is NO local
        // item does the imported stub's stamped struct name apply. The local
        // type comes from the explicit `ty` annotation, or from a struct-
        // literal initializer that names its own type (§4.5, mirroring
        // `resolve_data`'s inference).
        let ann: ast::Type = if let Some(decl) = self.datas.get(name).copied() {
            let decl: &'a ast::DataDecl = decl;
            match &decl.ty {
                Some(t) => t.clone(),
                None => match &decl.value {
                    ast::Expr::StructLit { ty, .. } => ast::Type::Named(ty.clone()),
                    _ => return None,
                },
            }
        } else if let Some(sname) = self.imported_item_types.get(name) {
            ast::Type::Named(ast::Path {
                segments: vec![sname.clone()],
                span: Span { source: sigil_span::SourceId(0), start: 0, end: 0 },
            })
        } else {
            return None;
        };
        // Resolve + peel to a struct through the SAME ladder `offsetof(T, f)`
        // uses. Resolution MUST NOT report (see doc): snapshot the diag length
        // and truncate back after probing, so a mis-typed receiver is diagnosed
        // at ITS OWN decl site (or, for a genuine miss, at the caller's span) —
        // never doubly here. A non-struct / unresolvable annotation yields
        // `None`, so the caller falls through rather than panicking on a bad
        // `layout_of_struct`.
        let before = self.diags.len();
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        let ty = self.resolve_type(&ann);
        let out = self.struct_name_for_offsetof(&ty, span);
        self.diags.truncate(before);
        out
    }

    /// Whether `name` names a data item whose comptime VALUE can be read for a
    /// `Def.field` access (D-PP.5, HALF B): a module-local data item with a
    /// struct-literal initializer, OR a cross-module type-only import (which
    /// resolves to the loud not-supported diagnostic in `resolve_data_value`).
    /// Non-struct-literal local data items (arrays, `bytes(..)`, etc.) are NOT
    /// field-accessible values, so they yield `false` and the caller falls
    /// through to the U3 label fallback — a bareword `Table.foo` that is not a
    /// struct value stays a link reference.
    ///
    /// PRECEDENCE (D-PP.5): a name that is ALSO an `enum`/`offsets`/`dispatch`
    /// keeps that meaning for `Name.member` (`Enum.Variant`, an ordinal, a
    /// scaled ordinal) — those steps run AFTER this one in `eval_path`, so the
    /// data-value read must not shadow them. The maps are expected disjoint
    /// (a same-named data item + enum is a naming error), but gating here
    /// preserves the pre-D-PP.5 order regardless. It DOES win over the U3 label
    /// fallback (the whole point).
    pub(crate) fn data_value_readable(&self, name: &str) -> bool {
        if self.enums.contains_key(name)
            || self.offsets.contains_key(name)
            || self.dispatches.contains_key(name)
        {
            return false;
        }
        if self.imported_item_types.contains_key(name) {
            return true;
        }
        self.datas
            .get(name)
            .is_some_and(|d| matches!(&d.value, ast::Expr::StructLit { .. }))
    }

    /// Read a module-local data item's comptime VALUE for a field access (D-PP.5,
    /// HALF B), evaluating its struct-literal initializer lazily and memoizing —
    /// the data analogue of [`resolve_const`](Self::resolve_const).
    ///
    /// - A memoized value (including `Poison`) returns directly.
    /// - A repeated in-progress name closes a cycle: report `cyclic data value:
    ///   <chain>` at `ref_span`, memoize `Poison`, return `Poison` (so the
    ///   `data A = S{x: B.x}` ↔ `B = S{x: A.x}` pair errors, never hangs).
    /// - A cross-module type-only import has NO initializer here — the VALUE read
    ///   is out of scope (only the ADDRESS half crosses modules), so it is a loud
    ///   `[value.cross-module]` not-supported diagnostic.
    ///
    /// Callers gate on [`data_value_readable`](Self::data_value_readable) first.
    fn resolve_data_value(&mut self, name: &str, ref_span: Span) -> Value {
        if let Some(v) = self.data_value_memo.get(name) {
            return v.clone();
        }
        // A cross-module type-only import: the initializer is not visible here.
        if !self.datas.contains_key(name) && self.imported_item_types.contains_key(name) {
            self.error(
                ref_span,
                format!(
                    "[value.cross-module] reading field values from the imported data item `{name}` is not supported (only its field ADDRESS crosses modules)"
                ),
            );
            self.data_value_memo.insert(name.to_string(), Value::Poison);
            return Value::Poison;
        }
        if let Some(start) = self.data_value_in_progress.iter().position(|n| n == name) {
            let mut chain: Vec<&str> =
                self.data_value_in_progress[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            self.error(ref_span, format!("cyclic data value: {}", chain.join(" -> ")));
            self.data_value_memo.insert(name.to_string(), Value::Poison);
            return Value::Poison;
        }
        // Borrow the `&'a DataDecl` out of the index (lifetime `'a`, from the
        // file) so `self` stays free to mutate across the recursive eval below —
        // the same borrow-decoupling `resolve_const` relies on.
        let decl: &'a ast::DataDecl =
            self.datas.get(name).copied().expect("caller ensures a local struct-lit data item");
        self.data_value_in_progress.push(name.to_string());
        // A data-item value read is an INDEPENDENT construction root: reading
        // `Def.art` while a sibling item's `Copy = ArtTile{ … }` construction is
        // still on the `struct_construct_in_progress` stack would spuriously trip
        // that stack's same-STRUCT-NAME guard (`ArtTile -> ArtTile`) — a false
        // cycle, since `Def` and `Copy` are distinct items. The genuine data
        // cycle (`A.x` ↔ `B.x`) is caught by `data_value_in_progress` above.
        // Swap in a fresh struct-construct stack for the initializer eval so an
        // item's OWN default-recursion (`struct A { x: A = A{} }`) is still
        // guarded within it, then restore the caller's stack.
        let saved = std::mem::take(&mut self.struct_construct_in_progress);
        let mut env = Env::new();
        let v = self.eval_expr(&decl.value, &mut env);
        self.struct_construct_in_progress = saved;
        self.data_value_in_progress.pop();
        self.data_value_memo.insert(name.to_string(), v.clone());
        v
    }

    /// Whether `name` is a file-level `offsets` table (its base label is a
    /// data-emitting item, not code). Used by the `dispatch` target kind check.
    pub(crate) fn is_offsets(&self, name: &str) -> bool {
        self.offsets.contains_key(name)
    }

    /// Whether `name` is a file-level `dispatch` table (its own base label is a
    /// data-emitting item, not code). Used by the `dispatch` target kind check.
    pub(crate) fn is_dispatch(&self, name: &str) -> bool {
        self.dispatches.contains_key(name)
    }

    /// Whether `name` is a named SST overlay (`vars Name: window { .. }`), which
    /// defines no emitted symbol at all. Used by the `dispatch` target kind
    /// check to name the miss precisely rather than let it drift to link.
    pub(crate) fn is_overlay(&self, name: &str) -> bool {
        self.overlays.contains_key(name)
    }

    /// The human-readable kind of `name` if it resolves module-locally to a
    /// NON-CODE item (`data`/`const`/`offsets`/`dispatch`/overlay `vars`), else
    /// `None`. Drives the `dispatch` target kind check (D6.B4): a target that
    /// resolves here is `[dispatch.target-not-code]`; `None` means either a
    /// `proc` (code — accepted) or an unknown name (left to link). Items are
    /// section-nested one level via [`index_items`](Self::index_items), so this
    /// sees section-nested data/consts too. Precedence is irrelevant — the maps
    /// are expected disjoint (no module-local cross-kind duplicate-name check
    /// exists here; name resolution errors on genuine collisions elsewhere), so
    /// the first match names the kind.
    pub(crate) fn non_code_kind(&self, name: &str) -> Option<&'static str> {
        if self.is_data(name) {
            Some("data item")
        } else if self.is_const(name) {
            Some("const")
        } else if self.is_offsets(name) {
            Some("offset table")
        } else if self.is_dispatch(name) {
            Some("dispatch table")
        } else if self.is_overlay(name) {
            Some("overlay")
        } else {
            None
        }
    }

    /// Push an [`Error`](Level::Error) diagnostic at `span`.
    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Error, message: msg.into(), primary: span });
    }

    /// Push a [`Warning`](Level::Warning) diagnostic at `span`. Used by
    /// default-on lints (e.g. `[layout.odd-field]`, T3) that report but do not
    /// poison — the check that triggered it still has a usable value/layout.
    pub fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Warning, message: msg.into(), primary: span });
    }

    /// Push a [`Note`](Level::Note) diagnostic at `span`. Used to attach
    /// follow-up context to a preceding error — e.g. the comptime-generator
    /// call-site provenance note (§9, D-P4.11) emitted when a spliced comptime
    /// call's generated table contains an error.
    pub fn note(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic { level: Level::Note, message: msg.into(), primary: span });
    }

    /// Charge one evaluation step. Returns `false` once [`STEP_BUDGET`] is
    /// exceeded so callers can bail out; keeps counting otherwise.
    pub fn bump_step(&mut self) -> bool {
        self.steps += 1;
        self.steps <= STEP_BUDGET
    }

    /// Abort evaluation on a hard limit (step budget or call depth, D-P2.16).
    ///
    /// Sets the [`aborted`](Self::aborted) flag (so all in-flight evaluation
    /// short-circuits and unwinds) and emits *one* error naming the active call
    /// chain — the innermost non-terminating callees, not an opaque quota. Only
    /// the first abort reports; later triggers during unwinding are ignored.
    fn abort(&mut self, span: Span, reason: &str) {
        if self.aborted {
            return;
        }
        self.aborted = true;
        let names: Vec<&str> = self.call_stack.iter().map(|(n, _)| n.as_str()).collect();
        // Keep the message bounded when a deep chain repeats the same callee:
        // show only the innermost `MAX_CHAIN_FRAMES`, prefixed with `...`.
        let chain = if names.len() > MAX_CHAIN_FRAMES {
            format!("... -> {}", names[names.len() - MAX_CHAIN_FRAMES..].join(" -> "))
        } else {
            names.join(" -> ")
        };
        let msg = if chain.is_empty() {
            reason.to_string()
        } else {
            format!("{reason}: in {chain}")
        };
        self.error(span, msg);
    }

    /// Resolve the file-level const OR equ named `name`, evaluating it lazily
    /// and memoizing the result. `ref_span` is the reference site, used to
    /// locate a cyclic-definition error.
    ///
    /// `equ` (R-T0.2) shares this ENTIRE path with `const` — same memo, same
    /// in-progress cycle stack, same fresh global-only env — because an equ's
    /// value is resolved exactly like a const's; the only difference is what a
    /// later lowering task does with the result (link-symbol emission vs.
    /// nothing). Sharing one memo/cycle-stack pair means a const and an equ of
    /// the SAME name would collide (one clobbers the other's memo entry); that
    /// is an existing-shape limitation shared with how two same-named consts
    /// already behave (last-wins at index-build time, per `index_items`'s
    /// doc), not a new one introduced here.
    ///
    /// - A memoized value (including a memoized `Poison`) is returned directly.
    /// - If `name` is already on the in-progress stack, this reference closes a
    ///   cycle: report `cyclic const definition: <chain>` at `ref_span`, memoize
    ///   `Poison` for `name` so the cascade suppresses, and return `Poison`.
    /// - Otherwise push `name`, evaluate its value expr in a fresh global-only
    ///   env (consts/equs see each other only by name, never each other's
    ///   locals), pop, memoize, and return.
    ///
    /// Callers must only invoke this for a `name` known to be in `self.consts`
    /// or `self.equs`.
    fn resolve_const(&mut self, name: &str, ref_span: Span) -> Value {
        if let Some(v) = self.const_memo.get(name) {
            return v.clone();
        }
        if let Some(start) = self.in_progress.iter().position(|n| n == name) {
            // Name the cycle as the chain from where it was first entered back
            // to this repeated reference, e.g. `A -> B -> A`.
            let mut chain: Vec<&str> = self.in_progress[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            self.error(ref_span, format!("cyclic const definition: {}", chain.join(" -> ")));
            self.const_memo.insert(name.to_string(), Value::Poison);
            return Value::Poison;
        }
        // Copy the value expr out of the index so it is borrowed from the file
        // (lifetime `'a`), not from `self`. That leaves `self` free to be
        // mutated (diags/memo/in_progress) across the recursive `eval_expr`
        // below. Consts are checked first (existing precedence, D2.12);
        // `equs` is the R-T0.2 fallback.
        let value_expr: &'a ast::Expr = if let Some(decl) = self.consts.get(name).copied() {
            &decl.value
        } else {
            let decl: &'a ast::EquDecl =
                self.equs.get(name).copied().expect("caller ensures the const/equ exists");
            &decl.value
        };
        self.in_progress.push(name.to_string());
        let mut env = Env::new();
        let v = self.eval_expr(value_expr, &mut env);
        self.in_progress.pop();
        self.const_memo.insert(name.to_string(), v.clone());
        v
    }

    /// Inject `-D NAME=INT` comptime defines into the module's global scope
    /// (sound-migration T2 Task 1, R1). Called once per evaluator, right after
    /// [`with_file`](Self::with_file) — BEFORE any item evaluates — so a bare
    /// reference to a define (`if DEBUG == 1 { .. }`) resolves exactly like a
    /// `const` reference: [`eval_path`](expr::Evaluator::eval_path) falls back
    /// to [`defines`](Self::defines) after `consts`/`equs` and returns the
    /// `Value::Int` directly. That map lookup is the WHOLE mechanism — a
    /// define has no `ast::ConstDecl` to evaluate, so `resolve_const` and its
    /// `const_memo`/cycle machinery are never involved (an already-resolved
    /// int can't cycle; this is R1's "pre-seeded resolved entry").
    ///
    /// A `name` already declared by the module as an INDEXED named item
    /// (const, equ, enum, fn, struct, bitfield, newtype, data, offsets,
    /// dispatch, or named overlay) is a collision the define does NOT win: it
    /// is silently left unseeded here so the module's own declaration resolves
    /// names afterward (never a silent shadow). The LOUD `[defines.collision]`
    /// diagnostic itself is NOT this method's job: every per-item evaluator
    /// calls `seed_defines` once, so reporting here would duplicate the
    /// diagnostic once per item. The lowering pass's `validate_defines` is the
    /// once-per-compile driver that reports it, mirroring how
    /// `validate_offsets`/`validate_dispatch` already keep their own
    /// once-per-compile duplicate/reserved-name checks out of the evaluator.
    ///
    /// `proc`/`script` names are ALSO `[defines.collision]` per R1, but only
    /// `validate_defines` can see them ([`index_items`](Self::index_items) has
    /// no proc/script table, so there is nothing to skip against here); the
    /// hard Error it emits fails the compile, so the define this method seeds
    /// for such a name is never observable.
    pub(crate) fn seed_defines(&mut self, defines: &[(String, i128)]) {
        for (name, value) in defines {
            if self.consts.contains_key(name.as_str())
                || self.equs.contains_key(name.as_str())
                || self.enums.contains_key(name.as_str())
                || self.fns.contains_key(name.as_str())
                || self.structs.contains_key(name.as_str())
                || self.bitfields.contains_key(name.as_str())
                || self.newtypes.contains_key(name.as_str())
                || self.datas.contains_key(name.as_str())
                || self.offsets.contains_key(name.as_str())
                || self.dispatches.contains_key(name.as_str())
                || self.overlays.contains_key(name.as_str())
            {
                continue;
            }
            self.defines.insert(name.clone(), *value);
        }
    }

    /// Select one of the three DISTINCT cycle-guard stacks for
    /// [`with_cycle_guard`](Self::with_cycle_guard). The stacks stay separate
    /// fields for correctness (see their doc comments); this only picks among
    /// them, never merges them.
    fn cycle_stack_mut(&mut self, stack: CycleStack) -> &mut Vec<String> {
        match stack {
            CycleStack::Layout => &mut self.layout_in_progress,
            CycleStack::Construct => &mut self.struct_construct_in_progress,
            CycleStack::Refine => &mut self.refine_check_in_progress,
        }
    }

    /// Shared cycle-guard boilerplate (behavior-neutral consolidation of the
    /// repeated push/check/pop + chain-diagnostic pattern that appears at each
    /// of the simple guarded sites: `effective_underlying`, `size_of_newtype`,
    /// `struct_name_for_offsetof`, the newtype-underlying-chain and
    /// refine-bound branches of `check_value_fits_ty_labeled`, and
    /// `eval_checked_struct_lit`.
    ///
    /// If `name` is already on the selected stack, this emits exactly one
    /// `cyclic {kind}: A -> B -> A` diagnostic at `span` (naming the in-progress
    /// chain from where `name` first entered, plus the repeat) and returns
    /// `None` WITHOUT running `body`. Otherwise it pushes `name`, runs `body`,
    /// pops (on every path — the pop happens here, once, regardless of how
    /// `body` returns), and returns `Some(result)`.
    ///
    /// Deliberately does NOT touch the complex `layout_of_struct` site: that
    /// guard is interleaved with the struct-layout memo re-check and the
    /// whole-cycle-slice poisoning, which this simple push/body/pop shape does
    /// not model.
    pub(crate) fn with_cycle_guard<T>(
        &mut self,
        stack: CycleStack,
        name: &str,
        span: Span,
        kind: &str,
        body: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        let stack_ref = self.cycle_stack_mut(stack);
        if let Some(start) = stack_ref.iter().position(|n| n == name) {
            let mut chain: Vec<&str> = stack_ref[start..].iter().map(|s| s.as_str()).collect();
            chain.push(name);
            let msg = format!("cyclic {kind}: {}", chain.join(" -> "));
            self.error(span, msg);
            return None;
        }
        self.cycle_stack_mut(stack).push(name.to_string());
        let result = body(self);
        self.cycle_stack_mut(stack).pop();
        Some(result)
    }
}

/// Which cycle-guard stack a [`Evaluator::with_cycle_guard`] call targets. The
/// three underlying `Vec<String>` fields are distinct BY DESIGN (see their doc
/// comments on [`Evaluator`]) — re-entrancy on layout/size, on struct-literal
/// construction, and on newtype refinement-bound eval are genuinely different
/// concerns, and merging their stacks would false-cycle legitimate patterns
/// (e.g. `newtype N = u8 where 0..sizeof(S)` alongside `struct S { x: N }`).
/// This enum only SELECTS among the three; it never merges them.
#[derive(Clone, Copy)]
pub(crate) enum CycleStack {
    /// [`Evaluator::layout_in_progress`] — size/layout/offsetof re-entrancy.
    Layout,
    /// [`Evaluator::struct_construct_in_progress`] — struct-literal
    /// construction re-entrancy.
    Construct,
    /// [`Evaluator::refine_check_in_progress`] — newtype refinement-bound-eval
    /// re-entrancy.
    Refine,
}

impl Default for Evaluator<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluate the top-level `const` named `name` in `file` to a comptime
/// [`Value`], returning it alongside every diagnostic emitted.
///
/// If no const of that name exists, returns `(None, [error])` reporting
/// `no const named `<name>``. Otherwise resolution is lazy and memoized: the
/// named const's value expression is evaluated, resolving referenced consts on
/// demand and detecting cyclic definitions (which yield [`Value::Poison`] plus a
/// diagnostic naming the cycle). A successful evaluation returns
/// `(Some(value), diags)` — `diags` may still be non-empty if the value
/// contains a reported error (its `Poison` is surfaced as `Some(Poison)`).
pub fn eval_const(file: &crate::ast::File, name: &str) -> (Option<Value>, Vec<Diagnostic>) {
    eval_const_with_root(file, name, None, &[])
}

/// Like [`eval_const`], but also threads a capability-sandbox `include_root`
/// (Spec 2, Plan 5 — Task 2): the directory `embed`/`import` paths resolve
/// against, mirroring [`crate::layout::eval_data_with_root`]. `include_root =
/// None` behaves exactly like [`eval_const`] (a comptime `import(...)` inside
/// the const then reports `[sandbox.no-root]`).
///
/// `defines` (sound-migration T2 Task 1, R1) are seeded into the evaluator's
/// global scope via [`Evaluator::seed_defines`] before `name` resolves, so a
/// const's value expression can reference a `-D` define like any other name.
///
/// This is the seam a bare `const V = import(...)` test uses to observe the
/// imported [`Value`] directly (no `data` item / byte layout needed) — the
/// production compile path does not yet supply a real root for consts either
/// (same deferred wiring note as `eval_data_with_root`).
pub fn eval_const_with_root(
    file: &crate::ast::File,
    name: &str,
    include_root: Option<&std::path::Path>,
    defines: &[(String, i128)],
) -> (Option<Value>, Vec<Diagnostic>) {
    // Run on a dedicated thread with a large stack so the native call stack has
    // headroom for [`MAX_CALL_DEPTH`] comptime frames (D-P2.16): the depth bound,
    // not a native stack overflow, is what stops runaway recursion.
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.seed_defines(defines);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        if let Some(v) = ev.defines.get(name).copied() {
            return (Some(Value::Int(v)), ev.diags);
        }
        if !ev.consts.contains_key(name) && !ev.equs.contains_key(name) {
            // Message unchanged for the pure-const-absent case (existing
            // callers/tests pin this exact wording); `equ` (R-T0.2) is folded
            // into the same lookup rather than earning a separate entry point,
            // since it resolves through the identical `resolve_const` path.
            ev.error(file.module.span, format!("no const named `{name}`"));
            return (None, ev.diags);
        }
        let value = ev.resolve_const(name, file.module.span);
        (Some(value), ev.diags)
    })
}

/// One `comptime test` outcome (S2-D11(a)).
#[derive(Debug)]
pub struct TestOutcome {
    /// The test's declared display name.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Diagnostics to SHOW for a failure. Empty on a pass — a passing
    /// `expect_error` test SWALLOWS its captured (expected) diagnostics.
    pub diags: Vec<Diagnostic>,
}

/// Run every `comptime test` block in `file` (S2-D11(a)) — the `sigil test`
/// engine. Each test evaluates its body as a comptime block on a FRESH
/// evaluator (module scope; v1 is module-local — the colocated case).
///
/// - normal test: PASS iff the body produced no Error-level diagnostic and
///   did not abort (a failing `ensure` is the canonical failure; its
///   interpolated message is the report);
/// - `expect_error: "[id]"`: PASS iff some body diagnostic contains the id
///   substring (the captured diagnostics are then swallowed — they were the
///   point); FAIL otherwise, with a synthesized explanation plus whatever
///   the body actually said.
pub fn run_module_tests(
    file: &crate::ast::File,
    include_root: Option<&std::path::Path>,
    defines: &[(String, i128)],
) -> Vec<TestOutcome> {
    let mut out = Vec::new();
    // Duplicate names are refused HERE too (Item-10 review M3): the build
    // path's validate pass never runs under `sigil test`, and the report
    // keys on names.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for item in &file.items {
        let crate::ast::Item::ComptimeTest(t) = item else { continue };
        if !seen.insert(t.name.as_str()) {
            out.push(TestOutcome {
                name: t.name.clone(),
                passed: false,
                diags: vec![Diagnostic {
                    level: Level::Error,
                    message: format!("comptime test `{}` is declared twice in this module", t.name),
                    primary: t.span,
                }],
            });
            continue;
        }
        let (aborted, diags) = run_on_eval_stack(|| {
            let mut ev = Evaluator::with_file(file);
            ev.seed_defines(defines);
            if let Some(root) = include_root {
                ev.set_include_root(root.to_path_buf());
            }
            let mut env = Env::new();
            let _ = ev.exec_stmts(&t.body, &mut env);
            // Belt (Item-10 review): a deferred link-time condition inside a
            // test body would otherwise pass VACUOUSLY (the linker never runs
            // here) — fail it loudly instead.
            let leaked = !ev.take_link_asserts().is_empty();
            (ev.was_aborted() || leaked, {
                let mut d = ev.diags;
                if leaked {
                    d.push(Diagnostic {
                        level: Level::Error,
                        message: "a test body deferred a link-time condition — `sigil test` \
                                  never links, so it can never be decided; test against \
                                  comptime-exact values"
                            .to_string(),
                        primary: t.span,
                    });
                }
                d
            })
        });
        let errored = aborted || diags.iter().any(|d| d.level == Level::Error);
        match &t.expect_error {
            None => out.push(TestOutcome {
                name: t.name.clone(),
                passed: !errored,
                diags: if errored { diags } else { Vec::new() },
            }),
            Some(id) => {
                // "Must NOT compile" means an ERROR — a warning that happens
                // to contain the id compiles fine (Item-10 review M4).
                if diags
                    .iter()
                    .any(|d| d.level == Level::Error && d.message.contains(id.as_str()))
                {
                    out.push(TestOutcome { name: t.name.clone(), passed: true, diags: Vec::new() });
                } else {
                    let mut shown = vec![Diagnostic {
                        level: Level::Error,
                        message: format!(
                            "expected the body to diagnose `{id}`, but it {}",
                            if diags.is_empty() { "compiled cleanly" } else { "said instead:" }
                        ),
                        primary: t.span,
                    }];
                    shown.extend(diags);
                    out.push(TestOutcome { name: t.name.clone(), passed: false, diags: shown });
                }
            }
        }
    }
    out
}


/// Stack size for the comptime-evaluation thread (see [`eval_const`]). Sized to
/// comfortably hold [`MAX_CALL_DEPTH`] comptime frames even in unoptimized
/// debug builds, where per-frame stack usage is large.
const EVAL_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Run `f` on a dedicated thread with a large ([`EVAL_STACK_BYTES`]) stack so
/// the native call stack has headroom for deep comptime evaluation (the
/// [`MAX_CALL_DEPTH`] bound, not a native overflow, is what stops runaway
/// recursion — D-P2.16). Shared by [`eval_const`] and the layout entry points
/// (which may drive comptime-fn calls via array lengths / refinement bounds).
/// A scoped thread lets `f` borrow non-`'static` data (the source `File`).
pub(crate) fn run_on_eval_stack<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(EVAL_STACK_BYTES)
            .spawn_scoped(scope, f)
            .expect("failed to spawn comptime evaluation thread");
        match handle.join() {
            Ok(v) => v,
            // Re-raise the original panic on the caller's thread so its payload,
            // message, and backtrace are preserved rather than flattened.
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}

/// Evaluate a `proc` body to a resolved [`CodeBuf`](crate::value::CodeBuf)
/// against `file`'s comptime context (Plan 4, T4 — §5.1), returning it plus any
/// diagnostics. This REUSES [`Evaluator::eval_asm`] — the exact same
/// `AsmStmt`→`CodeBuf` walk `asm { }` instantiation uses — so proc lowering and
/// `asm { }` share one operand/label path (D-P4.1). A proc body differs only in
/// that it is parsed with `splices_allowed = false`, so no `{splice}` ever
/// appears. The proc `name` is the owner scope for label hygiene (T5, §5.2): a
/// non-export `.loop:` renames fresh per instantiation (an intra-proc reference
/// still resolves) while an `export .entry:` becomes the caller-visible
/// `name.entry` other code can `bra` to. Params are declarative register
/// bindings (§5.1) that emit no code and need no env seeding — a body's register
/// operand resolves via the register name directly. Returns `None` only if
/// evaluation did not yield a `Code` value (it always does today; the guard keeps
/// the seam total).
///
/// `asm_counter_start` seeds the instantiation counter (D-P4.6): lowering builds
/// a FRESH evaluator per proc, so the counter would otherwise restart at 0 each
/// proc and two comptime-generated `asm { }` bodies from different procs would
/// mint colliding `$asm0…` symbols. The caller threads the counter across every
/// proc (passing the previous proc's returned value in), keeping `k` globally
/// monotonic. The advanced counter is returned as the third tuple element.
#[allow(clippy::too_many_arguments)] // internal driver; mirrors lower_module's state set
pub fn eval_proc_body(
    file: &crate::ast::File,
    name: &str,
    params: &[(String, ast::Type, Span)],
    body: &[ast::AsmStmt],
    span: Span,
    asm_counter_start: u32,
    cpu: sigil_ir::backend::Cpu,
    defines: &[(String, i128)],
) -> (Option<crate::value::CodeBuf>, Vec<Diagnostic>, u32) {
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.seed_defines(defines);
        ev.asm_counter = asm_counter_start;
        // The proc's section CPU is known here (unlike a raw `asm {}` template):
        // record it so a bare statement call in the body can consult the
        // mnemonic table for the mnemonic-vs-comptime-fn decision (D-PP.1).
        ev.cpu = Some(cpu);
        // Bind each `(aN: *S)` param whose pointee bottoms out at a struct into
        // the register→struct map (D6.A3). A bare-field displacement `f(aN)` on
        // such a register then resolves in field space. Param NAMES are register
        // spellings (§5.1), matching the clobber-lint model. Resolving the
        // pointee type MUST NOT report: an unresolvable/non-struct pointee simply
        // does not participate — its own decl-site diagnostics belong elsewhere,
        // and duplicating them here would double-report. So resolve on a scratch
        // evaluator whose diagnostics are discarded — built ONCE per proc (not
        // once per param): its only job is to run type resolution silently.
        let mut probe = Evaluator::with_file(file);
        for (pname, pty, pspan) in params {
            let Some(reg) = crate::value::Reg::from_name(pname) else { continue };
            if let ast::Type::Ptr(inner) = pty {
                let inner_ty = probe.resolve_type(inner);
                if let Some(sname) = probe.struct_name_for_offsetof(&inner_ty, *pspan) {
                    ev.reg_pointee_struct.insert(reg, sname);
                }
            }
        }
        drop(probe);
        let mut env = Env::new();
        let buf = match ev.eval_asm_owned(body, span, &mut env, Some(name)) {
            Value::Code(buf) => Some(buf),
            _ => None,
        };
        (buf, ev.diags, ev.asm_counter)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i(n: i128) -> Value {
        Value::Int(n)
    }

    #[test]
    fn evaluator_error_collects_diagnostic() {
        let mut ev = Evaluator::new();
        let span = Span { source: sigil_span::SourceId(0), start: 1, end: 2 };
        ev.error(span, "boom");
        assert_eq!(ev.diags.len(), 1);
        assert_eq!(ev.diags[0].level, Level::Error);
        assert_eq!(ev.diags[0].message, "boom");
    }

    #[test]
    fn bump_step_reports_budget_exhaustion() {
        let mut ev = Evaluator::new();
        assert!(ev.bump_step());
        ev.steps = STEP_BUDGET - 1;
        // The step that reaches exactly the budget is still allowed...
        assert!(ev.bump_step());
        assert_eq!(ev.steps, STEP_BUDGET);
        // ...the next one exceeds it.
        assert!(!ev.bump_step());
    }

    #[test]
    fn eval_const_missing_reports_error() {
        let (v, diags) = crate::eval::eval_const(&empty_file(), "MISSING");
        assert!(v.is_none());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("no const named `MISSING`"));
    }

    #[test]
    fn comptime_var_outside_context_is_diagnosed_but_still_bound() {
        // `comptime var` at `comptime_ctx == 0` (module/const level) is illegal.
        // Surface syntax can't reach this (a `comptime var` only parses inside a
        // fn/comptime-block body, which bump the context), so drive `exec_stmts`
        // directly to prove the guard fires — and that the name is still bound
        // (mutable) so downstream references don't cascade.
        let mut ev = Evaluator::new();
        let mut env = Env::new();
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        let stmts = vec![ast::Stmt::Var {
            name: "x".to_string(),
            ty: None,
            value: ast::Expr::Int(7, span),
            span,
        }];
        assert_eq!(ev.comptime_ctx, 0);
        let _ = ev.exec_stmts(&stmts, &mut env);
        assert!(
            ev.diags.iter().any(|d| d.message.contains("comptime var is only allowed")),
            "diagnostics were {:?}",
            ev.diags
        );
        assert_eq!(env.lookup("x"), Some(&i(7)));
    }

    fn empty_file() -> crate::ast::File {
        use crate::ast::*;
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        File {
            module: ModuleDecl {
                path: Path { segments: vec!["m".into()], span },
                in_section: None,
                span,
            },
            attrs: vec![],
            items: vec![],
            docs: vec![],
        }
    }
}
