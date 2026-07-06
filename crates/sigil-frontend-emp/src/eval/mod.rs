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
mod control;
mod emit;
mod env;
mod expr;
mod float_ns;
mod guards;
mod literals;
mod pattern;
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
    /// Memoized struct layouts, keyed by struct name — the layout analogue of
    /// [`const_memo`](Self::const_memo). A zero-size Poisoned layout records a
    /// struct that already failed (cyclic layout) so it does not re-report.
    pub(crate) struct_layout_memo: HashMap<String, crate::layout::Layout>,
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
    /// The VMA the `here()` comptime builtin resolves to (§7.1), or `None` when
    /// no position is known (outside lowering). Set per data-item by the lowering
    /// pass to `vma_origin + current_offset` — the VMA at the START of the item
    /// being lowered. `here()` is a lowering-time query the pure evaluator cannot
    /// answer on its own, so the position is threaded in here. LIMITATION: it is
    /// the item's start VMA, so a `here()` mid-buffer (after some bytes in the
    /// SAME data item) still reads the item start, not the advanced position.
    here_base: Option<u32>,
    /// The capability-sandbox root (Spec 2, Plan 5 — Task 1): the directory
    /// `embed`/`import` paths resolve against. `None` outside a rooted
    /// evaluation (e.g. the plain [`eval_const`] entry point), in which case a
    /// comptime file read is `[sandbox.no-root]`. Set via
    /// [`set_include_root`](Self::set_include_root) by the
    /// [`layout::eval_data_with_root`](crate::layout::eval_data_with_root) seam.
    include_root: Option<PathBuf>,
    /// The capture ledger (Task 1): one [`sandbox::CaptureEdge`] per comptime
    /// file read (`embed`, and later `import`/`zx0`), recording the resolved
    /// path, its SHA-256 digest, and its byte length — the provenance record a
    /// later hermeticity task exposes and asserts determinism from.
    pub(crate) captures: Vec<sandbox::CaptureEdge>,
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
            enums: HashMap::new(),
            fns: HashMap::new(),
            structs: HashMap::new(),
            bitfields: HashMap::new(),
            newtypes: HashMap::new(),
            datas: HashMap::new(),
            offsets: HashMap::new(),
            struct_layout_memo: HashMap::new(),
            bitfield_layout_memo: HashMap::new(),
            layout_in_progress: Vec::new(),
            aborted: false,
            pending_return: None,
            comptime_ctx: 0,
            const_memo: HashMap::new(),
            in_progress: Vec::new(),
            data_memo: HashMap::new(),
            struct_construct_in_progress: Vec::new(),
            refine_check_in_progress: Vec::new(),
            asm_counter: 0,
            here_base: None,
            include_root: None,
            captures: Vec::new(),
        }
    }

    /// Set the VMA `here()` resolves to for the item about to be evaluated
    /// (§7.1). The lowering pass calls this before resolving each data item.
    pub(crate) fn set_here_base(&mut self, vma: u32) {
        self.here_base = Some(vma);
    }

    /// Create an evaluator that can resolve names against `file`'s top-level
    /// `const` and `enum` items. Later duplicate names (a parse-level concern)
    /// are resolved last-wins by the index build; duplicate diagnosis is not
    /// this task's job.
    pub fn with_file(file: &'a ast::File) -> Self {
        let mut ev = Evaluator::new();
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
                    self.datas.insert(d.name.as_str(), d);
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
                ast::Item::Section(s) => self.index_items(&s.items),
                _ => {}
            }
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

    /// Resolve the file-level const named `name`, evaluating it lazily and
    /// memoizing the result. `ref_span` is the reference site, used to locate a
    /// cyclic-definition error.
    ///
    /// - A memoized value (including a memoized `Poison`) is returned directly.
    /// - If `name` is already on the in-progress stack, this reference closes a
    ///   cycle: report `cyclic const definition: <chain>` at `ref_span`, memoize
    ///   `Poison` for `name` so the cascade suppresses, and return `Poison`.
    /// - Otherwise push `name`, evaluate its value expr in a fresh global-only
    ///   env (consts see each other only by name, never each other's locals),
    ///   pop, memoize, and return.
    ///
    /// Callers must only invoke this for a `name` known to be in `self.consts`.
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
        // Copy the `&'a ConstDecl` out of the index so its `value` expr is
        // borrowed from the file (lifetime `'a`), not from `self`. That leaves
        // `self` free to be mutated (diags/memo/in_progress) across the
        // recursive `eval_expr` below.
        let decl: &'a ast::ConstDecl =
            self.consts.get(name).copied().expect("caller ensures the const exists");
        self.in_progress.push(name.to_string());
        let mut env = Env::new();
        let v = self.eval_expr(&decl.value, &mut env);
        self.in_progress.pop();
        self.const_memo.insert(name.to_string(), v.clone());
        v
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
    eval_const_with_root(file, name, None)
}

/// Like [`eval_const`], but also threads a capability-sandbox `include_root`
/// (Spec 2, Plan 5 — Task 2): the directory `embed`/`import` paths resolve
/// against, mirroring [`crate::layout::eval_data_with_root`]. `include_root =
/// None` behaves exactly like [`eval_const`] (a comptime `import(...)` inside
/// the const then reports `[sandbox.no-root]`).
///
/// This is the seam a bare `const V = import(...)` test uses to observe the
/// imported [`Value`] directly (no `data` item / byte layout needed) — the
/// production compile path does not yet supply a real root for consts either
/// (same deferred wiring note as `eval_data_with_root`).
pub fn eval_const_with_root(
    file: &crate::ast::File,
    name: &str,
    include_root: Option<&std::path::Path>,
) -> (Option<Value>, Vec<Diagnostic>) {
    // Run on a dedicated thread with a large stack so the native call stack has
    // headroom for [`MAX_CALL_DEPTH`] comptime frames (D-P2.16): the depth bound,
    // not a native stack overflow, is what stops runaway recursion.
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        if let Some(root) = include_root {
            ev.set_include_root(root.to_path_buf());
        }
        if !ev.consts.contains_key(name) {
            ev.error(file.module.span, format!("no const named `{name}`"));
            return (None, ev.diags);
        }
        let value = ev.resolve_const(name, file.module.span);
        (Some(value), ev.diags)
    })
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
pub fn eval_proc_body(
    file: &crate::ast::File,
    name: &str,
    body: &[ast::AsmStmt],
    span: Span,
    asm_counter_start: u32,
) -> (Option<crate::value::CodeBuf>, Vec<Diagnostic>, u32) {
    run_on_eval_stack(|| {
        let mut ev = Evaluator::with_file(file);
        ev.asm_counter = asm_counter_start;
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
        }
    }
}
