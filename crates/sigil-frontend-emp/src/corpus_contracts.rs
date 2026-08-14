//! Contract-grammar v2 — the whole-corpus contract walk that feeds the
//! transitive closure ([`crate::closure`]).
//!
//! The closure is a pure algorithm over a name-keyed [`ProcNode`] map; THIS
//! module builds that map from the parsed `.emp` corpus (the §11 Q2 decision: a
//! whole-corpus FRONTEND pass, name-resolved, not a post-link pass — so it
//! reuses the real write detector [`crate::lower::proc_written_registers`] with
//! no drift, and source spans stay native). For each proc it derives:
//!
//! - **local writes** — from the proc's evaluated [`CodeBuf`] (same substrate as
//!   `emp_census`; `a7` stack-discipline filtered exactly as the census does),
//! - **direct callees** — the `Sym` targets of `jsr`/`bsr`/`jbsr` (calls, whose
//!   unresolved names are holes) and of `jmp`/`bra`/`jbra` (tail transfers, kept
//!   only when the target is a known proc so a local-label branch adds no edge),
//! - **indirect sites** — from the AST body: each `jsr (aN) [as Type]` call site
//!   contributes its declared bound (`Some(type)`) or `None` (⊤).
//!
//! Externs (§3) become closure leaves; contract types (§4) become clobber
//! bounds. The report also flags the §11 Q4 collision (a name declared BOTH as
//! `extern proc` and `proc`).

use crate::ast::{self, AsmStmt, ContractTypeDecl, ExternProcDecl, InstrLine, Item, Operand, ProcDecl, ProcSig, TextOrSplice};
use crate::calls::{check_input_undefined, check_live_clobbered, InputFiring, LiveClobberFiring};
use crate::closure::{check_firings, compute_closure, Closure, Firing, ProcNode, RegEffect};
use crate::flag_check::{check_flag_unused, check_result_invalid_path, FlagFiring};
use crate::lower::{
    expand_reglist_regs, preserve_oracle_inputs, proc_written_registers, verified_preserves_regs,
};
use crate::out_verify::{
    check_cond_out_survives, check_inout, check_out, compute_verified_inouts, compute_verified_outs,
    CondOutMap, CondOutSurvivesFiring, InoutCallees, InoutFiring, OutClaim, OutFiring, OutWidth,
    OutWidthMap, OutWidths, UncondOutMap,
};
use crate::preserves::{find_dead_saves, DeadSave};
use crate::z80_out_verify::{check_z80_out, compute_verified_z80_outs, Z80OutFiring, Z80OutMap};
use crate::branch_const::{check_branch_const, BranchConstFiring};
use crate::context::{bracketed_at, check_regions, regions_of, ContextFiring, Region};
use crate::z80_bus::{check_bus_state, BusEntry, BusFiring};
use crate::type_slice::{check_slot_types, SlotTypeMismatch};
use crate::value::{CodeBuf, CodeItem, CodeOperand, Reg};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::{BTreeMap, BTreeSet};

/// The register file the closure tracks — `d0`..`d7` + `a0`..`a6` (`a7`/sp is
/// stack discipline, never a clobber). This is the universe ⊤ ranges over and
/// the set a "preserves-only" contract type clobbers the complement of.
fn universe() -> BTreeSet<String> {
    (0..8).map(|n| format!("d{n}")).chain((0..7).map(|n| format!("a{n}"))).collect()
}

/// Mnemonics that CALL: an unresolved target is a hole (a missing `extern proc`).
const CALL_MNEMONICS: [&str; 3] = ["jsr", "bsr", "jbsr"];
/// Mnemonics that TAIL-TRANSFER: the target's effects become the caller's, but
/// an unresolved target (a local `.loop` label) is NOT a hole — so these edges
/// are kept only when the target resolves to a known proc.
const TAIL_MNEMONICS: [&str; 3] = ["jmp", "bra", "jbra"];

/// The corpus-wide contract analysis result.
#[derive(Debug, Default)]
pub struct ContractReport {
    /// Every proc's transitive `effective` clobber set.
    pub closure: Closure,
    /// The transitive under-declaration firings (§9), sorted (proc, reg).
    pub firings: Vec<Firing>,
    /// The §6 caller-side flag-result firings: `[call.flag-result-unused]` (a
    /// carry result abandoned on some path) and `[call.result-invalid-path]` (a
    /// conditional register result read on its invalid path), sorted (proc,
    /// callee, span).
    pub flag_firings: Vec<FlagFiring>,
    /// The §6 flag firings RECOMPUTED with the VERIFIED-out credit maps (vs
    /// `flag_firings`, which uses the DECLARED maps). §6 deliberately keeps declared
    /// credit (redefine-kill semantics); this is the honest-residual TRIPWIRE — it
    /// must EQUAL `flag_firings` today. The day a corpus change makes them diverge,
    /// declared credit is suppressing a real §6 firing on a shipping ERROR gate, and
    /// the tripwire test fails loudly instead of the firing silently vanishing.
    pub flag_firings_verified_credit: Vec<FlagFiring>,
    /// Names declared BOTH `extern proc` and `proc` (§11 Q4) — with the extern's
    /// span (the mirror that should be deleted when the callee ports).
    pub extern_collisions: Vec<(String, Span)>,
    /// How many procs (incl. externs) the walk collected.
    pub proc_count: usize,
    /// How many extern-proc leaves.
    pub extern_count: usize,
    /// How many contract types.
    pub contract_type_count: usize,
    /// The §6/D1d `[proc.dead-save]` firings: a verified save/restore of a
    /// register the bracketed callee (per the closure's VERIFIED `effective`
    /// set) provably preserves — the pass-3 dead-save worklist. Sorted
    /// (proc, reg, span).
    pub dead_saves: Vec<DeadSave>,
    /// The §6/G4 `[call.input-undefined]` (D1b) firings: a callee register-param
    /// input with no reaching definition on some path at a call site. Sorted
    /// (proc, callee, reg, span).
    pub input_firings: Vec<InputFiring>,
    /// The §6/G4 `[call.live-clobbered]` (D1c) firings: a value defined before a
    /// call and read after it, held in a register the callee EFFECTIVELY
    /// clobbers — pass-3's seatbelt. Sorted (proc, callee, reg, span).
    pub live_clobbered_firings: Vec<LiveClobberFiring>,
    /// The §G4.5 `[proc.out-unverified]` firings: a proc declares `out(rN)` but
    /// the body does not PRODUCE rN on every required return path (the callee-side
    /// out-honesty check). Sorted (proc, reg). NOT yet joined to the error gate —
    /// the checkpoint-B residue is adjudicated before the flip.
    pub out_firings: Vec<OutFiring>,
    /// The `[proc.inout-unverified]` firings: a proc declares `inout(rN)` but the
    /// body leaves rN Broken on some exit path (a partial write, a clobbering or
    /// conditional-out callee, or an unknown callee). Unlike `out`, PASS-THROUGH is
    /// valid, so an unwritten path does NOT fire. Sorted (proc, reg).
    pub inout_firings: Vec<InoutFiring>,
    /// The §7.1 `[proc.out-cond-survives-unverifiable]` firings: a proc declares
    /// `out(rN if cc)` with rN ABSENT from `clobbers` — the survives-claim — but
    /// rN's entry value is not provably intact on every ¬cc return path. Proved
    /// under the closure's callee-preserves ORACLE, so a preserving call does not
    /// kill the proof; this is the FINAL AUTHORITY over the per-file gate's
    /// call-blocked deferrals. Sorted (proc, reg).
    pub survives_firings: Vec<CondOutSurvivesFiring>,
    /// Every proc that MAKES a §7.1 survives claim — declares `out(rN if cc)`
    /// with rN absent from its `clobbers`. Sorted. Exposed because an
    /// assert-empty gate over `survives_firings` is only as meaningful as the set
    /// of claims it ranged over: a contract edit that DELETES a claim would make
    /// the gate quietly vacuous, and this is what a test pins to notice.
    pub survives_claim_sites: Vec<String>,
    /// Every `out(rN: T)` whose `T` the corpus cannot resolve to a width, as
    /// `(proc, register, type)`. Sorted.
    ///
    /// An unresolvable type answers the BARE claim on both sides — 32 bits
    /// obligated, 32 bits credited — so it degrades to exactly the declaration the
    /// author would have written with no type at all, and is no less sound than
    /// one. What it is NOT is what the author asked for: a typo'd or unimported
    /// type name silently means "all 32 bits" instead of the narrow thing intended.
    /// The width cannot be guessed, so the answer is to say so rather than to
    /// choose a number, and this is the surface that says it. Only the corpus walk
    /// can: the newtype table is corpus-wide, so a per-file pass would fire on
    /// every legitimate cross-module type.
    pub unresolvable_out_types: Vec<(String, String, String)>,
    /// Every `out(rN: T)` SLOT the type walk saw, as `(proc, register)`. Sorted.
    ///
    /// The subject matter of [`Self::unresolvable_out_types`], exposed so an
    /// assert-empty over that list can be guarded against ranging over nothing.
    /// A guard built on any map keyed by PROC NAME cannot do it: those carry a key
    /// whether or not the proc declares a type at all, so stripping every type off
    /// the corpus leaves them unchanged. This list loses a row.
    pub typed_out_slots: Vec<(String, String)>,
    /// The verified-out FIXPOINT result — each proc's UNCONDITIONAL outs PROVEN
    /// produced (extern outs seeded verified). The DEFINITION credit source for D1b
    /// must-def and the `out_firings` residue surface; exposed so the consistency
    /// test can assert the residue is exactly the complement of this map (the two
    /// surfaces read ONE source and cannot drift).
    pub verified_uncond_out: UncondOutMap,
    /// The verified-out fixpoint's CONDITIONAL outs (the dual of
    /// `verified_uncond_out` for `out(rN if cc)`).
    pub verified_cond_out: CondOutMap,
    /// The verified-INOUT fixpoint: proc → the `inout(rN)` registers proven
    /// non-Broken on every exit. The complement of `inout_firings` per proc, and the
    /// rule-6 composition credit. Exposed so a test can assert an in-out proc
    /// verifies where the same body under `out` would not (the non-vacuity pair).
    pub verified_inout: UncondOutMap,
    /// Total instructions DROPPED across the corpus because an operand/mnemonic
    /// did not resolve during the single-file eval — the substrate hazard the
    /// cross-file type environment closes. With a complete environment this is
    /// **0**; the corpus pin asserts it, so a silent under-approximation of any
    /// analysis buffer can never return.
    pub dropped_instrs: usize,
    /// Per-proc drop counts (only procs with `> 0`), sorted by proc name — the
    /// "per-file reported event": names exactly which proc lost instructions.
    pub dropped_by_proc: Vec<(String, usize)>,
    /// The `[comptime.unresolved]` surface: `(proc, name, span)` rows, one per
    /// comptime-condition reference — an `if` condition, a `while` condition, or
    /// a `for` iterable — the walk's define/const environment failed to resolve.
    /// Sorted (proc, name, span), exact duplicates deduped.
    /// This is the toggle complement of [`dropped_instrs`]: a missing VALUE
    /// define drops the instruction that needed it, but a statement-`if` whose
    /// condition is `Value::Poison` discards BOTH arms whole — zero drops, zero
    /// firings, the guarded code simply invisible. A profile that lost or
    /// misspelled `DEBUG` or `SOUND_DRIVER_ENABLED` reproduces exactly that, so
    /// the corpus gates pin this EMPTY per shipped shape. Collected AT the two
    /// `if`-condition eval sites (statement and expression), never by a second
    /// walk, so it cannot drift from what the analysis actually evaluated.
    ///
    /// [`dropped_instrs`]: Self::dropped_instrs
    pub comptime_unresolved: Vec<(String, String, Span)>,
    /// The G5 §7 `[call.slot-type-mismatch]` firings: a domain-newtype-typed
    /// callee param slot (`Section_FlatIDXY (d2: GridX, …)`) reached at a call
    /// site by an untyped or wrong-newtype value. Sorted (proc, callee, reg,
    /// span). ERROR-tier — the sec_x/sec_y swap class.
    pub slot_firings: Vec<SlotTypeMismatch>,
    /// The `[branch.condition-constant]` firings: a conditional branch whose
    /// reaching CCR-definition is a compile-time constant, so its outcome is
    /// statically determined (dead code / a clobbered condition — the
    /// `Sound_PlayMusic.await_slot` bug). Sorted (proc, span). The item-4 rider.
    pub branch_const_firings: Vec<BranchConstFiring>,
    /// The `[bus.*]` Z80-bus machine-state firings (item-4 core): a double-stop,
    /// unpaired start, stopped-at-return, or VDP write while the bus is provably
    /// running — the sigil-native absorption of s4lint E006/E007/E008/E011.
    /// Sorted (proc, span). Byte-neutral (corpus-only).
    pub bus_firings: Vec<BusFiring>,
    /// The §3.2 `with`-bracket firings — escape / entry-skip / reacquire.
    /// 68k only: the walk's `proc_bufs` excludes `(cpu: z80)` modules, so a
    /// bracket in a Z80 module is proven by the per-file gate alone. The
    /// per-file gate ([`crate::lower::proc`]) is what FAILS THE BUILD on these;
    /// this is the same computation over the same bodies, surfaced so the corpus
    /// gate can assert the class stays empty (the `check_survives_claims` /
    /// `check_cond_out_survives` precedent). Sorted (proc, ctx, span).
    pub context_firings: Vec<ContextFiring>,
    /// The §3.3 `[context.unsatisfied]` firings: a call site that does not have
    /// every context its callee `requires(...)` active. Sorted (proc, callee,
    /// ctx, span). ERROR-tier — a declared context is never `@as_compat`-softened.
    pub context_unsatisfied: Vec<ContextUnsatisfied>,
    /// Every call site where a callee's `requires(...)` WAS discharged:
    /// `(proc, callee, ctx)`, sorted. The companion an assert-empty over
    /// `context_unsatisfied` needs — that gate is only meaningful if call sites
    /// were EXAMINED, and `call_target_sym` resolves a direct call only, so a
    /// refactor to indirect dispatch would empty the examined set while the
    /// firing set stayed (correctly) empty.
    pub context_discharged: Vec<(String, String, String)>,
    /// `requires(...)` / `grants(...)` clauses naming a context no module
    /// declares: `(proc, ctx, span)`, sorted. A silently-ignored requirement
    /// would be worse than none — it would read as a checked claim.
    pub unknown_context_refs: Vec<(String, String, Span)>,
    /// Every proc that DECLARES a context obligation — `(proc, "requires"|"grants",
    /// ctx)`, sorted. Exposed for the same reason `survives_claim_sites` is: an
    /// assert-empty gate over `context_unsatisfied` is only as meaningful as the
    /// set of claims it ranged over, and a contract edit that deleted every
    /// `requires` would make the gate quietly vacuous.
    pub context_claim_sites: Vec<(String, String, String)>,
    /// Each `with`-bracketed region the corpus lowers: `(proc, ctx)`, sorted with
    /// duplicates kept (one row per region). The bracket census — the adoption
    /// measurement, and the tripwire against a silent un-adoption.
    pub context_regions: Vec<(String, String)>,
    /// The contexts a bracket PROVES hold the Z80 bus — identified from the
    /// RESOLVED operand of the acquire each region splices (the same
    /// resolved-operand-not-macro-name discipline `[bus.*]` itself uses), never
    /// from a context's spelling. A proc requiring one of these is analyzed by
    /// `[bus.*]` from a DECLARED entry state.
    pub bus_contexts: BTreeSet<String>,
    /// The S2-D6 U4 `@allow("clobbers.unanalyzable", "<reason>")` annotations in
    /// force: `(proc, reason)`, sorted by proc. Every proc that opted a
    /// genuinely-unanalyzable computed-dispatch site OUT of the `unbounded`
    /// firing appears here — so the escape-hatch surface stays audited (never
    /// silent). Expected census: single digits (raw computed jumps outside the
    /// typed-dispatch idiom); today's corpus is EMPTY.
    pub unanalyzable_allows: Vec<(String, String)>,
    /// Every SR-DESTINATION write in the walked 68k procs, with WHO wrote it:
    /// `(proc, author, span)`, sorted (proc, span). The typed census behind the
    /// warn tier's authorship gate: `[proc.sr-undeclared]` exempts
    /// `Context`/`AssertDesugar`-authored writes (each proven at its author's
    /// own surface), so the gate reads THIS to assert the exemption never
    /// covers an author whose obligation lands nowhere — and that the desugar
    /// class is actually populated (non-vacuity; the `Context` count is
    /// reported, not pinned — adoption moves it).
    pub sr_writes: Vec<(String, crate::value::ItemAuthor, Span)>,
    /// The AbsSym call-edge census — every call/tail edge the closure collects
    /// through AbsSym recognition (`jsr (sym).l` naming a global symbol), as
    /// `(proc, callee)`, sorted+deduped, with AssertDesugar-authored rails
    /// EXCLUDED by authorship. `invoke Iface.hook` is the sole corpus source of
    /// such an edge, so a shape that binds a hook shows `(invoker, bound_proc)`
    /// here and the `assert`/`raise` rails' `jsr (MDDBG__…).l` never appear. The
    /// edge being live is asserted from THIS census, not inferred from a silent
    /// residue: both corpus invokers declare full-universe clobbers, so their
    /// effect on the closure is otherwise invisible.
    pub abs_call_edges: Vec<(String, String)>,
    /// The would-be holes the authorship exclusion suppresses: every
    /// AssertDesugar-authored abs-call target that resolves to NEITHER a `proc`
    /// nor an `extern proc` — exactly the `unresolved_callees` the residue gate
    /// reports if the exclusion is stripped. The corpus set is the two desugar
    /// link boundaries `{MDDBG__ErrorHandler, MDDBG__ErrorHandler_PagesController}`.
    /// The revert probe asserts this set names exactly those, so the exclusion's
    /// effect is a checked fact rather than a silent absence.
    pub authored_rail_holes: BTreeSet<String>,
    /// The §6 partial-width `[proc.preserves-unverifiable]` firings: a proc declares
    /// `preserves(dN.w)` whose low-word round-trip DEFERRED at the per-file byte
    /// gate (call-blocked) and could NOT be proven under the closure's verified
    /// `effective` oracle either — a false or unprovable word claim. `(proc, "dN.w",
    /// span)`, sorted. The word-facet analog of a full-preserve deferral erroring at
    /// the closure gate: a full deferral surfaces via `[proc.clobber-undeclared]`,
    /// which the word facet silences by design (`dN` is licensed-clobbered), so the
    /// obligation needs this dedicated gate.
    pub word_preserve_firings: Vec<(String, String, Span)>,
    /// The §6 partial-width CONTRACT SURFACE: every `preserves(dN.w)` the corpus
    /// declares, as `(proc, "dN")`, sorted. Spec §3 records the facet for a
    /// width-aware consumer to read; this is that record, and it is also the
    /// NON-VACUITY census for `word_preserve_firings` — an assert-empty firing gate
    /// is only meaningful if claims existed to check, and this says which.
    pub word_preserve_claims: Vec<(String, String)>,
    /// The §G4.5 `[proc.out-unverified]` firings for `(cpu: z80)` procs: a Z80 proc
    /// declares `out(rN)` but the body does not PRODUCE `rN` (a register UNIT) on
    /// every required return path. The Z80 sibling of [`Self::out_firings`],
    /// SEPARATE because the 68k closure skips Z80 modules twice over (`proc_bufs`
    /// excludes them, `Reg::from_name` rejects Z80 spellings) — the check is the
    /// Z80 unit-domain twin. Sorted (proc, unit).
    pub z80_out_firings: Vec<Z80OutFiring>,
    /// The Z80 verified-out FIXPOINT result — each Z80 proc's out UNITS PROVEN
    /// produced (extern outs seeded verified). The residue surface's own source;
    /// exposed so a consistency test can assert the residue is exactly the
    /// complement of this map (the two surfaces read ONE source and cannot drift).
    pub z80_verified_out: Z80OutMap,
    /// Every `(cpu: z80)` proc that DECLARES a register out, as `(proc, unit)`
    /// rows, sorted. The NON-VACUITY census for [`Self::z80_out_firings`] — an
    /// assert-baseline gate over the firings is only as meaningful as the set of
    /// out claims it ranged over, and this is that set (a Z80 out is a DECLARATION,
    /// not comptime-conditional, so it is shape-independent).
    pub z80_out_claims: Vec<(String, String)>,
}

/// Analyze the parsed corpus with the canonical no-`-D` config (census-parity).
pub fn analyze_corpus(files: &[ast::File]) -> ContractReport {
    analyze_corpus_with(files, &[])
}

/// Build the L1 interface environment for a corpus walk: bind every `interface`
/// against ONE game's `implement` blocks — modules under `games.` are excluded
/// from the bind set unless their id is `game_module_prefix` or sits under it
/// (`games.sonic4` keeps `games.sonic4.game`, drops `games.demo.game`). The
/// ANALYSIS module set is unchanged; only binding is game-selected, because
/// [`crate::resolve::contract::bind`] requires exactly one `implement` per
/// interface and the corpus holds one per game.
///
/// Binding values evaluate with the PASS-1 corpus environment as ambient
/// (`const ENTRY_ID = GS_OJZ_SCROLL_TEST` reads an imported const, which the
/// impl file alone cannot resolve — the production bind pass sees it through
/// resolve's ambient injection, and this is the corpus equivalent). Returned
/// diagnostics are the bind pass's own; a caller pinning the walk should assert
/// no ERRORS among them, since a half-bound env silently poisons every
/// `Iface.MEMBER` condition it failed to fill.
pub fn bind_corpus_interfaces(
    files: &[ast::File],
    defines: &[(String, i128)],
    game_module_prefix: &str,
) -> (crate::contract::InterfaceEnv, Vec<sigil_span::Diagnostic>) {
    let keep = |id: &str| {
        !id.starts_with("games.")
            || id == game_module_prefix
            || id.strip_prefix(game_module_prefix).is_some_and(|rest| rest.starts_with('.'))
    };
    let with_ids: Vec<(String, &ast::File)> = files
        .iter()
        .map(|f| (f.module.path.segments.join("."), f))
        .filter(|(id, _)| keep(id))
        .collect();
    // The bind AMBIENT is game-filtered exactly like the bind MODULE set: the
    // bind only evaluates the selected game's implement, and an env carrying the
    // OTHER game's declarations would let a cross-game name collision resolve
    // last-indexed-wins — selecting bindings the shipped ROM does not. (The
    // analysis walk's own PASS-1 env stays whole-corpus; it analyzes both games
    // in one pass.)
    let mut env: Vec<Item> = Vec::new();
    for (_, file) in &with_ids {
        collect_env(&file.items, &mut env);
    }
    let mods: Vec<crate::resolve::contract::ContractModule> = with_ids
        .iter()
        .map(|(id, f)| crate::resolve::contract::ContractModule { id, file: f })
        .collect();
    crate::resolve::contract::bind_with_ambient(&mods, defines, &env)
}

/// Whether a module declares `(cpu: z80)` — its procs are OUTSIDE the 68k
/// register-contract closure (a mirror of `attr_cpu` in `lower/mod.rs`, kept
/// local so `corpus_contracts` needs no lowering import).
fn module_is_z80(module: &ast::ModuleDecl) -> bool {
    module.attrs.iter().any(|(name, expr)| {
        name == "cpu"
            && matches!(expr, ast::Expr::Path(p)
                if p.segments.last().is_some_and(|s| s.eq_ignore_ascii_case("z80")))
    })
}

/// Analyze the parsed corpus under the given comptime `-D` defines: build the
/// proc/extern/contract-type maps, run the closure, and collect firings +
/// collisions. Comptime-`if` gating is config-sensitive, so the defines choose
/// which code paths lower (the plain canonical build is `SOUND_DRIVER_ENABLED=1`;
/// the census — and `analyze_corpus` — use no defines).
///
/// Runs with an EMPTY interface environment: an L1 `Iface.MEMBER` reference in a
/// comptime condition does not resolve here, and a statement-`if` gated on one
/// discards both arms — visible on [`ContractReport::comptime_unresolved`], never
/// silent. A shape-accurate walk supplies the env via
/// [`analyze_corpus_with_contracts`].
pub fn analyze_corpus_with(files: &[ast::File], defines: &[(String, i128)]) -> ContractReport {
    analyze_corpus_with_contracts(files, defines, &crate::contract::InterfaceEnv::empty())
}

/// [`analyze_corpus_with`] with a bound L1 interface environment — the walk the
/// per-shape corpus gates run. The env decides comptime conditions gated on
/// game-contract members (`if Game.CAMERA_JUMP_LOCK { }` in `Camera_Update`):
/// with it, the member resolves to the SHAPE'S game's binding and the guarded arm
/// enters the analysis exactly when it enters the shipped ROM; without it the
/// reference lands on `comptime_unresolved` and the arm is invisible. The env is
/// built by [`crate::resolve::contract::bind`] over the engine + ONE game's
/// modules (`bind` requires exactly one `implement` per interface, and the corpus
/// holds one per game).
pub fn analyze_corpus_with_contracts(
    files: &[ast::File],
    defines: &[(String, i128)],
    contracts: &crate::contract::InterfaceEnv,
) -> ContractReport {
    let mut nodes: BTreeMap<String, ProcNode> = BTreeMap::new();
    let mut types: BTreeMap<String, RegEffect> = BTreeMap::new();
    let mut extern_names: BTreeSet<String> = BTreeSet::new();
    let mut proc_names: BTreeSet<String> = BTreeSet::new();
    let mut extern_spans: BTreeMap<String, Span> = BTreeMap::new();
    let mut counter: u32 = 0;
    // §6 flag-result wiring: the flag / conditional-result contracts a callee
    // declares, keyed by name, plus each proc's evaluated CodeBuf + the call
    // sites carrying `@discards` (the caller-side check needs cross-module
    // contract knowledge, so it runs after the whole corpus is walked).
    let mut flag_callees: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut cond_callees: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut proc_bufs: Vec<ProcBuf> = Vec::new();
    let mut dropped_by_proc: Vec<(String, usize)> = Vec::new();
    let mut comptime_unresolved: Vec<(String, String, Span)> = Vec::new();

    // PASS 1 — the cross-file TYPE ENVIRONMENT: every declaration item across the
    // whole corpus (structs / consts / newtypes / …), so PASS 2's per-file eval
    // resolves field operands on IMPORTED structs instead of silently dropping
    // them. A general environment (not the resolve pass's `use`-driven per-file
    // ambient, whose maintenance proved incomplete for this analysis).
    let mut env: Vec<Item> = Vec::new();
    for file in files {
        collect_env(&file.items, &mut env);
    }

    // PASS 2 — walk every file, evaluating each proc body against `env`. A
    // `(cpu: z80)` module is SKIPPED: the transitive clobber/write-set closure is
    // the 68k register-contract model, and a Z80 proc's instructions carry no 68k
    // effect — analyzing them would "drop" every Z80 mnemonic as unrecognized
    // (a false under-approximation). Z80 contract analysis (push/pop preserves,
    // shadow sets, di/ei) is its own rung-2 class. The module's consts/types
    // stay visible (PASS 1 collected them).
    for file in files {
        if module_is_z80(&file.module) {
            continue;
        }
        // The per-file oracle inputs (§register-preserve credit) — built from the
        // SAME sources the primary `lower` checker feeds `check_preserves`, so the
        // corpus verdict stays consistent with the primary path (see
        // `PreserveInputs`). Rebuilt per file: both sets are file-local, matching
        // the primary checker's per-file scope.
        let file_noreturn = crate::lower::collect_noreturn_symbols(&file.items);
        let file_sr_mask_preservers =
            crate::lower::collect_sr_mask_preservers(&file.items, file, defines, contracts);
        let preserve_inputs = PreserveInputs {
            noreturn: &file_noreturn,
            sr_mask_preservers: &file_sr_mask_preservers,
        };
        collect_items(
            &file.items,
            file,
            &mut nodes,
            &mut types,
            &mut extern_names,
            &mut proc_names,
            &mut extern_spans,
            &mut counter,
            &mut flag_callees,
            &mut cond_callees,
            &mut proc_bufs,
            defines,
            &env,
            &mut dropped_by_proc,
            &mut comptime_unresolved,
            contracts,
            &preserve_inputs,
        );
    }
    dropped_by_proc.sort();
    let dropped_instrs = dropped_by_proc.iter().map(|(_, n)| n).sum();

    // §11 Q4: a name declared both `extern proc` and `proc` collides.
    let mut extern_collisions: Vec<(String, Span)> = extern_names
        .intersection(&proc_names)
        .map(|n| (n.clone(), extern_spans[n]))
        .collect();
    extern_collisions.sort_by(|a, b| a.0.cmp(&b.0));

    let extern_count = extern_names.len();
    let contract_type_count = types.len();
    let proc_count = nodes.len();

    // The corpus-wide `@noreturn` symbol set (noreturn-tail model): the oracle
    // round consults it so a `preserves` proc whose error rail tails into a
    // `@noreturn` handler is not charged for a path that never returns.
    let noreturn: BTreeSet<String> = files
        .iter()
        .flat_map(|f| crate::lower::collect_noreturn_symbols(&f.items))
        .collect();

    let mut closure = compute_closure(&nodes, &types);

    // The callee-preserves ORACLE round (§5) — the transitive upgrade to each proc's
    // oracle-FREE base verified-preserves. A proc that save/restores a register
    // across a call to a callee the closure has proven preserves it (rN ∉
    // effective(callee)) genuinely preserves rN, though the base (no-callee-knowledge)
    // seed could not show it — the `TestChurnObj_Main` shape (pop a0, then a trailing
    // `jsr DeleteObject` that itself `preserves(a0)`). Re-verify every declared
    // `preserves` under the oracle built from the BASE effective map (the SAME map +
    // `callee_clobbers` convention `find_dead_saves` reads). The credit is MONOTONE
    // over the base: a preserving callee only REMOVES clobbers, so the recomputed
    // effective can only shrink (final ⊆ base), and any base-proven preserves stays
    // proven under the oracle — one round then a single closure recompute suffices,
    // no outer fixpoint. Call CYCLES / unknown callees stay conservative (the closure
    // fixpoint already folded SCCs into `effective`; the oracle reads them as
    // clobber-all).
    {
        let base_effective = closure.effective.clone();
        let mut upgraded = false;
        for pb in &proc_bufs {
            if pb.preserve_check.is_empty() {
                continue;
            }
            let Some(node) = nodes.get_mut(&pb.name) else { continue };
            if node.verified_preserves == pb.preserve_names {
                continue; // already fully credited by the oracle-free base
            }
            let status = crate::preserves::verify_preserved(
                &pb.buf.items,
                &pb.preserve_check,
                crate::preserves::CallPolicy::Oracle(&base_effective),
                pb.falls_into.as_deref(),
                &noreturn,
            );
            let all_verified = pb.preserve_check.iter().all(|r| {
                matches!(status.get(r), Some(crate::preserves::PreserveStatus::Verified))
            });
            if all_verified {
                node.verified_preserves = pb.preserve_names.clone();
                upgraded = true;
            }
        }
        if upgraded {
            closure = compute_closure(&nodes, &types);
        }
    }

    let firings = check_firings(&nodes, &closure);

    // §6 partial-width word-facet gate. The per-file byte gate DEFERS a call-blocked
    // `preserves(dN.w)` low-word round-trip (verifies under `PreserveAll`, silent);
    // THIS is its final authority — re-prove the low word under the closure's
    // verified `effective` oracle, so a preserving callee keeps the round-trip alive
    // exactly as the full-preserve oracle round does. A register that still does not
    // round-trip its low word here is a false/unprovable word claim (the analog the
    // `[proc.clobber-undeclared]` closure gate provides for a full deferral, which
    // the word facet silences by licensing `dN`'s clobber). ClobberAll would refire
    // every DEFERRED (correct) claim, so the Oracle is mandatory.
    let mut word_preserve_firings: Vec<(String, String, Span)> = Vec::new();
    let mut word_preserve_claims: Vec<(String, String)> = Vec::new();
    for pb in &proc_bufs {
        if pb.preserve_word_check.is_empty() {
            continue;
        }
        for r in &pb.preserve_word_check {
            word_preserve_claims.push((pb.name.clone(), r.to_string()));
        }
        let status = crate::preserves::verify_preserved_word(
            &pb.buf.items,
            &pb.preserve_word_check,
            crate::preserves::CallPolicy::Oracle(&closure.effective),
            pb.falls_into.as_deref(),
            &noreturn,
        );
        for r in &pb.preserve_word_check {
            if !matches!(status.get(r), Some(crate::preserves::PreserveStatus::Verified)) {
                word_preserve_firings.push((pb.name.clone(), format!("{r}.w"), pb.span));
            }
        }
    }
    word_preserve_firings.sort_by(|a, b| (&a.0, &a.1, a.2.start).cmp(&(&b.0, &b.1, b.2.start)));
    word_preserve_claims.sort();

    // Callee contract maps shared by the caller-side checks (§6 invalid-path, D1b
    // must-def, D1c). Built once here, after the whole corpus is walked.
    let callee_params: BTreeMap<String, BTreeSet<String>> =
        nodes.iter().map(|(n, node)| (n.clone(), node.params.clone())).collect();
    let callee_out: BTreeMap<String, BTreeSet<String>> =
        nodes.iter().map(|(n, node)| (n.clone(), node.out.clone())).collect();
    // The UNCONDITIONAL subset of each callee's outs — `node.out` INCLUDES a
    // conditional `out(rN if cc)` register (the parser folds it into the reglist,
    // its cc-guard riding `cond_callees`). The caller-side ERROR gates may only
    // treat an out defined on EVERY return edge as a redefine/definition, so
    // subtract the conditional-out registers: crediting a conditional out
    // unconditionally would be a FALSE NEGATIVE on a shipping ERROR gate — §6
    // (invalid-path taint-kill) and D1b (must-def credit) both consume this via
    // the shared `call_unconditional_outs` primitive. D1c/closure keep the full
    // `callee_out` (a conditional out IS a produced result there). This is the
    // map-level form of [`ast::ProcDecl::unconditional_outs`] — the callee names
    // here are already canonical on both sides (`conds_of` canonicalizes).
    let callee_uncond_out: BTreeMap<String, BTreeSet<String>> = callee_out
        .iter()
        .map(|(n, outs)| {
            let cond: BTreeSet<&String> = cond_callees
                .get(n)
                .into_iter()
                .flatten()
                .map(|(reg, _)| reg)
                .collect();
            (n.clone(), outs.iter().filter(|r| !cond.contains(r)).cloned().collect())
        })
        .collect();
    // The declared `inout(...)` registers per proc — the check_inout obligation set,
    // and (once verified) the rule-6 composition credit. Kept apart from
    // `callee_uncond_out`, which folds them in for caller-side crediting.
    let callee_inout: BTreeMap<String, BTreeSet<String>> =
        nodes.iter().map(|(n, node)| (n.clone(), node.inout.clone())).collect();
    // Each proc's effective clobber set as a plain name set — the rule-5 input to
    // the inout verifier (a register a callee destroys breaks an in-out value
    // threaded across the call). A ⊤ effect clobbers everything, so it expands to
    // the whole register file; a proc absent here is an unknown callee (Broken).
    let all_regs: BTreeSet<String> = ["d0", "d1", "d2", "d3", "d4", "d5", "d6", "d7", "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let effective_clobbers: BTreeMap<String, BTreeSet<String>> = closure
        .effective
        .iter()
        .map(|(n, eff)| {
            let set = if eff.top { all_regs.clone() } else { eff.regs.clone() };
            (n.clone(), set)
        })
        .collect();

    // The VERIFIED-out FIXPOINT (contract-grammar v2 §G4.5). An out is credited as
    // a reaching DEFINITION only once PROVEN produced on every required return path
    // (extern outs seed verified — §3 boundary axioms). The DEFINITION gates below
    // — D1b must-def and the out-verify residue surface — credit THESE maps instead
    // of the raw DECLARED `callee_uncond_out`/`cond_callees`, so the FindStagedBlock
    // existence-lie can no longer silently satisfy a must-def input. The
    // REDEFINE-excuse consumers (§6 taint-kill, D1c held-value) KEEP the declared
    // maps: a width-unverified out genuinely redefines its register (low word
    // fresh), so it still kills taint / is a produced result — see the dividing-line
    // table in the residue note.
    let proc_items: BTreeMap<String, &[CodeItem]> =
        proc_bufs.iter().map(|pb| (pb.name.clone(), pb.buf.items.as_slice())).collect();
    // Proc -> its declared `falls_into` successor. A fall-off-end here is not a
    // return: control continues into the successor inside the same call, so the
    // out claim is CHARGED AGAINST THE SUCCESSOR's verified outs rather than
    // against this body — the same credit a tail transfer gets. The successor's
    // NAME is the payload, because a "has a successor" bit cannot credit anything
    // and would leave the claim resting on nothing.
    let falls_into_succ: BTreeMap<String, String> = proc_bufs
        .iter()
        .filter_map(|pb| pb.falls_into.clone().map(|succ| (pb.name.clone(), succ)))
        .collect();
    // Proc -> the WIDTH each typed `out(dN: T)` claims. A bare `out(rN)` is absent
    // and keeps its 32-bit meaning, so this map only ever RELAXES an obligation —
    // and only where an author wrote a type. It serves both directions at once:
    // a proc's own row is what its returns are charged, and every other proc's row
    // is what a call / tail / fall-off to it may be credited.
    let (out_widths, unresolvable_out_types, typed_out_slots) = collect_out_widths(files);
    let (verified_uncond_out, verified_cond_out): (UncondOutMap, CondOutMap) =
        compute_verified_outs(
            &proc_items,
            &callee_uncond_out,
            &cond_callees,
            &extern_names,
            &falls_into_succ,
            &noreturn,
            &out_widths,
        );

    // The VERIFIED-INOUT fixpoint — the in-out analogue of the out fixpoint. An
    // `inout(rN)` is verified iff the exit-side check leaves rN non-Broken on every
    // path (pass-through counts), with callee credit drawn only from verified
    // outs/inouts. The width map is `inout(rN: T)`'s (so `inout(d5: u16)` charges at
    // `.w`); an in-out register absent there is a bare 32-bit claim.
    let inout_widths = collect_inout_widths(files);
    let verified_inout: UncondOutMap = compute_verified_inouts(
        &proc_items,
        &callee_inout,
        &verified_uncond_out,
        &cond_callees,
        &effective_clobbers,
        &extern_names,
        &falls_into_succ,
        &noreturn,
        &inout_widths,
    );
    // The caller-side CREDIT union: a call to a verified-out OR verified-inout callee
    // reproduces the register for the caller. D1b must-def and the out-verify credit
    // read THIS, so an in-out callee credits its callers exactly as the parcel-A
    // `out` form did — no D1b/§6 regression from moving `d5`/`a4` out→inout. (D1c and
    // §6-declared keep the DECLARED `callee_uncond_out`, which already folds inout in
    // via `node.out`.)
    let verified_out_credit: UncondOutMap = {
        let mut m = verified_uncond_out.clone();
        for (name, regs) in &verified_inout {
            m.entry(name.clone()).or_default().extend(regs.iter().cloned());
        }
        m
    };

    // §6 caller-side flag checks, now that every callee's contract is known. §6
    // keeps the DECLARED credit (redefine-kill semantics). `flag_firings_verified`
    // recomputes the invalid-path check against the VERIFIED credit — the honest-
    // residual tripwire: it must EQUAL `flag_firings` today, so the day a corpus
    // change makes declared-credit SUPPRESS a §6 firing the verified credit would
    // show, the tripwire fails loudly instead of a real firing silently vanishing
    // on a shipping ERROR gate.
    let mut flag_firings: Vec<FlagFiring> = Vec::new();
    let mut flag_firings_verified: Vec<FlagFiring> = Vec::new();
    for pb in &proc_bufs {
        let unused =
            check_flag_unused(&pb.name, &pb.buf.items, &flag_callees, &pb.discarded, Cpu::M68000);
        flag_firings_verified.extend(unused.iter().cloned());
        flag_firings.extend(unused);
        flag_firings.extend(check_result_invalid_path(
            &pb.name,
            &pb.buf.items,
            &cond_callees,
            &callee_uncond_out,
        ));
        flag_firings_verified.extend(check_result_invalid_path(
            &pb.name,
            &pb.buf.items,
            &verified_cond_out,
            &verified_out_credit,
        ));
    }
    // rung-2 §13.3 sub-part 3 — the Z80 caller-must-consume flag check. PASS 2
    // skips `(cpu: z80)` modules from the 68k register-contract closure (a Z80
    // proc carries no 68k effect), but the flag-result must-use check is
    // inherently cross-proc: a Z80 caller that `jr c`s on a callee's
    // `out(carry:)` is credited, one that abandons it fires
    // `[call.flag-result-unused]`. Z80 procs and their flag callees are
    // self-contained within Z80 modules (a Z80 proc is reachable only from Z80),
    // so a SEPARATE Cpu::Z80 pass collects their flag contracts + CodeBufs and
    // runs the check — the 68k closure stays Z80-free. `check_result_invalid_path`
    // is NOT run for Z80 (no corpus site declares a Z80 conditional register
    // result — the represented-not-wired boundary).
    let mut z80_flag_callees: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut z80_proc_bufs: Vec<ProcBuf> = Vec::new();
    // §G4.5 Z80 out-honesty inputs, collected in the SAME Z80 pass: each proc /
    // extern's declared register-out UNIT set, each proc's `falls_into` successor,
    // and the Z80 extern names (fixpoint boundary seeds).
    let mut z80_declared_out: Z80OutMap = BTreeMap::new();
    let mut z80_falls_into: BTreeMap<String, String> = BTreeMap::new();
    let mut z80_extern_names: BTreeSet<String> = BTreeSet::new();
    for file in files {
        if module_is_z80(&file.module) {
            collect_z80_flag_procs(
                &file.items, file, &mut counter, defines, &env, &mut z80_flag_callees,
                &mut z80_proc_bufs, &mut comptime_unresolved, contracts,
                &mut z80_declared_out, &mut z80_falls_into, &mut z80_extern_names,
            );
        }
    }
    for pb in &z80_proc_bufs {
        let unused =
            check_flag_unused(&pb.name, &pb.buf.items, &z80_flag_callees, &pb.discarded, Cpu::Z80);
        flag_firings_verified.extend(unused.iter().cloned());
        flag_firings.extend(unused);
    }

    // §G4.5 Z80 callee-side out-honesty. The Z80 unit-domain twin of the 68k
    // out-verify below: every declared `out(rN)` register unit must be PRODUCED on
    // every required return path. The VERIFIED-out FIXPOINT grounds each claim only
    // in already-verified callee / tail / `falls_into` outs (extern outs seed
    // verified — §3 boundary axioms), so a proc whose out grounds only in an
    // unverified callee out appears in the residue. The `noreturn` set is the
    // corpus-wide one computed above; it spares a tail transfer into an `@noreturn`
    // handler the out obligation, exactly as the 68k `exit_diverges` does.
    let z80_proc_items: BTreeMap<String, &[CodeItem]> =
        z80_proc_bufs.iter().map(|pb| (pb.name.clone(), pb.buf.items.as_slice())).collect();
    let z80_verified_out = compute_verified_z80_outs(
        &z80_proc_items,
        &z80_declared_out,
        &z80_extern_names,
        &z80_falls_into,
        &noreturn,
    );
    let mut z80_out_firings: Vec<Z80OutFiring> = Vec::new();
    let mut z80_out_claims: Vec<(String, String)> = Vec::new();
    for pb in &z80_proc_bufs {
        let Some(units) = z80_declared_out.get(&pb.name) else { continue };
        if units.is_empty() {
            continue;
        }
        for u in units {
            z80_out_claims.push((pb.name.clone(), u.clone()));
        }
        let out_units: Vec<String> = units.iter().cloned().collect();
        z80_out_firings.extend(check_z80_out(
            &pb.name,
            &pb.buf.items,
            &out_units,
            &z80_verified_out,
            z80_falls_into.get(&pb.name).map(String::as_str),
            &noreturn,
        ));
    }
    z80_out_firings.sort_by(|a, b| (&a.proc, &a.unit).cmp(&(&b.proc, &b.unit)));
    z80_out_claims.sort();

    // Deterministic order (proc, callee, flag); spans stay in encounter order
    // via the stable sort.
    let flag_sort = |a: &FlagFiring, b: &FlagFiring| {
        (&a.proc, &a.callee, &a.flag).cmp(&(&b.proc, &b.callee, &b.flag))
    };
    flag_firings.sort_by(flag_sort);
    flag_firings_verified.sort_by(flag_sort);

    // D1d dead-save worklist: run over every proc's CodeBuf against the closure's
    // VERIFIED effective sets (never raw declared text — pass-3 cuts code on this).
    //
    // And because pass-3 cuts code on this, the closure it reads is the CONSERVATIVE
    // one. A `jsr (aN) as Type` narrowing is trusted, not proven — nothing checks
    // that the procs installed in the dispatch table satisfy the bound (S12) — and
    // for THIS consumer the unsafe direction is the narrow one: the smaller a
    // callee's clobber set looks, the more of the caller's saves look dead. Advice
    // to DELETE a save must never rest on an unverified bound, so dead-save
    // detection treats every indirect site as ⊤. The warn-tier analyses below keep
    // the trusting closure, where a narrower set only suppresses warnings and the
    // frozen baselines already encode it.
    let ds_closure =
        crate::closure::compute_closure_with(&nodes, &types, crate::closure::IndirectPolicy::Unbounded);
    let mut dead_saves: Vec<DeadSave> = Vec::new();
    for pb in &proc_bufs {
        dead_saves.extend(find_dead_saves(&pb.name, &pb.buf.items, &ds_closure.effective));
    }
    dead_saves.sort_by(|a, b| {
        (&a.proc, a.reg, a.span.start).cmp(&(&b.proc, b.reg, b.span.start))
    });

    // §6/G4 caller-side input + liveness checks. D1b keys off each callee's
    // declared register-param inputs; D1c keys off the closure's VERIFIED
    // effective clobber set (minus declared outputs). Maps built above.
    let mut input_firings: Vec<InputFiring> = Vec::new();
    let mut live_clobbered_firings: Vec<LiveClobberFiring> = Vec::new();
    for pb in &proc_bufs {
        let caller_params =
            nodes.get(&pb.name).map(|n| n.params.clone()).unwrap_or_default();
        // D1b credits an out as a DEFINITION ⇒ VERIFIED maps (the flip foundation).
        // An inout callee reproduces the register too, so the credit union folds in
        // verified inouts — a caller passing `d5` (defined by an inout DrawRings
        // call) onward is not flagged undefined.
        input_firings.extend(check_input_undefined(
            &pb.name,
            &caller_params,
            &pb.buf.items,
            &callee_params,
            &verified_out_credit,
            &verified_cond_out,
        ));
        live_clobbered_firings.extend(check_live_clobbered(
            &pb.name,
            &caller_params,
            &pb.buf.items,
            &closure.effective,
            &callee_out,
            &callee_uncond_out,
        ));
    }
    input_firings.sort_by(|a, b| {
        (&a.proc, &a.callee, &a.reg, a.span.start).cmp(&(&b.proc, &b.callee, &b.reg, b.span.start))
    });
    live_clobbered_firings.sort_by(|a, b| {
        (&a.proc, &a.callee, &a.reg, a.span.start).cmp(&(&b.proc, &b.callee, &b.reg, b.span.start))
    });

    // §G4.5 callee-side out-honesty: every declared `out(rN)` must be PRODUCED on
    // every required return path. The registers CHECKED are each proc's DECLARED
    // outs; the callee/tail-target CREDIT is drawn from the VERIFIED maps (ruling 3
    // — the out-verify residue surface reports the SAME fact D1b's must-def credits,
    // so the WARN residue and the ERROR gate can never disagree on whether an out is
    // honest). This is the fixpoint's own residue: a proc whose out grounds only in
    // an unverified callee out appears here, where a declared-credit reading would
    // discharge it. NO shipping proc currently has that shape, so BOTH credit maps
    // below are discriminated by synthetics, one per map —
    // `tests/corpus_contracts.rs::the_out_residue_surface_uses_verified_credit_not_declared`
    // for `verified_uncond_out`, and `::the_conditional_out_credit_surface_also_uses_verified_credit`
    // for `verified_cond_out`. They are separate arguments and one guard does not
    // cover the other.
    let mut out_firings: Vec<OutFiring> = Vec::new();
    let empty_widths: BTreeMap<String, OutClaim> = BTreeMap::new();
    for pb in &proc_bufs {
        // The out OBLIGATION excludes inout registers: they are folded into
        // `callee_uncond_out` for caller crediting, but their exit-side claim is the
        // threaded-cursor one (pass-through valid), checked by `check_inout` below —
        // NOT the produce-on-every-path one `check_out` charges.
        let own_inout = callee_inout.get(&pb.name);
        let uncond: Vec<Reg> = callee_uncond_out
            .get(&pb.name)
            .into_iter()
            .flatten()
            .filter(|r| !own_inout.is_some_and(|s| s.contains(*r)))
            .filter_map(|r| Reg::from_name(r))
            .collect();
        let cond: Vec<(Reg, String)> = cond_callees
            .get(&pb.name)
            .into_iter()
            .flatten()
            .filter_map(|(reg, cc)| Reg::from_name(reg).map(|r| (r, cc.clone())))
            .collect();
        if uncond.is_empty() && cond.is_empty() {
            continue;
        }
        out_firings.extend(check_out(
            &pb.name,
            &pb.buf.items,
            &uncond,
            &cond,
            &verified_out_credit,
            &verified_cond_out,
            pb.span,
            // A declared `falls_into` continues into its successor inside the
            // same call, so control running off this body's end is not a return —
            // the claim is charged against the successor's verified outs, one
            // derivation shared with the fixpoint above.
            pb.falls_into.as_deref(),
            &noreturn,
            OutWidths {
                own: out_widths.get(&pb.name).unwrap_or(&empty_widths),
                callees: &out_widths,
            },
        ));
    }
    out_firings.sort_by(|a, b| (&a.proc, &a.reg, a.span.start).cmp(&(&b.proc, &b.reg, b.span.start)));

    // The in-out exit-side residue: every declared `inout(rN)` must be non-Broken on
    // every exit path (pass-through valid). Registers checked are each proc's
    // DECLARED inouts; callee credit is the VERIFIED out (rule 3) + VERIFIED inout
    // (rule 6) maps, with `effective_clobbers` for the clobber rule (5).
    let mut inout_firings: Vec<InoutFiring> = Vec::new();
    for pb in &proc_bufs {
        let tracked: Vec<Reg> = callee_inout
            .get(&pb.name)
            .into_iter()
            .flatten()
            .filter_map(|r| Reg::from_name(r))
            .collect();
        if tracked.is_empty() {
            continue;
        }
        inout_firings.extend(check_inout(
            &pb.name,
            &pb.buf.items,
            &tracked,
            OutWidths {
                own: inout_widths.get(&pb.name).unwrap_or(&empty_widths),
                callees: &inout_widths,
            },
            InoutCallees {
                uncond_out: &verified_uncond_out,
                inout: &verified_inout,
                cond_out: &cond_callees,
                effective_clobbers: &effective_clobbers,
            },
            pb.span,
            pb.falls_into.as_deref(),
            &noreturn,
        ));
    }
    inout_firings.sort_by(|a, b| (&a.proc, &a.reg, a.span.start).cmp(&(&b.proc, &b.reg, b.span.start)));

    // §7.1 the SURVIVES half: a cond-out register absent from `clobbers` must be
    // provably PRESERVED on every ¬cc return path. The per-file gate proves what a
    // single file can (`ClobberAll`, deferring anything blocked solely by a call);
    // THIS is the final authority — the closure's verified `effective` map lets a
    // preserving callee keep the proof alive, exactly as the §5 preserves oracle
    // does. Checked registers come from the DECLARED cond list and the DECLARED
    // clobbers, because that pair is precisely what the contract's reader sees.
    // 68k only, twice over and deliberately: `proc_bufs` excludes `(cpu: z80)`
    // modules (they collect into `z80_proc_bufs`), and `Reg::from_name` rejects
    // every Z80 spelling below. The per-file gate carries the matching guard with
    // the `VALID_CCS` rationale. When ccs go CPU-parametric, BOTH need revisiting.
    let mut survives_firings: Vec<CondOutSurvivesFiring> = Vec::new();
    let mut survives_claim_sites: Vec<String> = Vec::new();
    for pb in &proc_bufs {
        let Some(node) = nodes.get(&pb.name) else { continue };
        if !node.has_clobber_contract {
            continue; // no clobber contract ⇒ no membership to read ⇒ no claim
        }
        let cond: Vec<(Reg, String)> = pb
            .cond_out_pairs
            .iter()
            .filter_map(|(reg, cc)| Reg::from_name(reg).map(|r| (r, cc.clone())))
            .collect();
        if cond.is_empty() {
            continue;
        }
        if cond.iter().any(|(r, _)| !node.declared_clobbers.contains(&r.to_string())) {
            survives_claim_sites.push(pb.name.clone());
        }
        survives_firings.extend(check_cond_out_survives(
            &pb.name,
            &pb.buf.items,
            &cond,
            &node.declared_clobbers,
            crate::preserves::CallPolicy::Oracle(&closure.effective),
            pb.span,
        ));
    }
    survives_claim_sites.sort();
    survives_firings.sort_by(|a, b| {
        (&a.proc, &a.reg, &a.cc, a.span.start).cmp(&(&b.proc, &b.reg, &b.cc, b.span.start))
    });

    // G5 §7 tier 5 — the caller-side domain-newtype slot check. The corpus's
    // newtype names gate which param/out slots are DOMAIN-typed (a plain `u8`/`*Act`
    // param is not a slot the check engages — §7 no-ceremony); `typed_params` /
    // `typed_out` map each such slot to its register index + newtype. `effective`
    // (the transitive clobber closure) + `callee_out` model the post-call degrade.
    let newtype_names = collect_newtype_names(files);
    let option_payloads = collect_option_payloads(files, &newtype_names);
    let (typed_params, typed_out) = collect_typed_slots(files, &newtype_names);
    // The caller-facing degrade contract: each callee's DECLARED clobbers (the
    // S2-D6 ERROR gate proves declared ⊇ actual-minus-preserved, so declared is
    // the sound over-approximation to reason a caller's type across a call). A
    // callee that declares NO clobber contract maps to `None` — clobber-all.
    let callee_clobbers: BTreeMap<String, Option<BTreeSet<String>>> = nodes
        .iter()
        .map(|(n, node)| {
            let decl = if node.has_clobber_contract || node.is_extern {
                Some(node.declared_clobbers.clone())
            } else {
                None
            };
            (n.clone(), decl)
        })
        .collect();
    let mut slot_firings: Vec<SlotTypeMismatch> = Vec::new();
    for pb in &proc_bufs {
        let own = typed_params.get(&pb.name).cloned().unwrap_or_default();
        slot_firings.extend(check_slot_types(
            &pb.name,
            &pb.buf.items,
            &typed_params,
            &typed_out,
            &callee_out,
            &callee_clobbers,
            &newtype_names,
            &option_payloads,
            &own,
        ));
    }
    slot_firings.sort_by(|a, b| {
        (&a.proc, &a.callee, &a.reg, a.span.start).cmp(&(&b.proc, &b.callee, &b.reg, b.span.start))
    });

    // [branch.condition-constant] — the item-4 rider. A conditional branch whose
    // reaching CCR-def is a compile-time constant (statically-decided outcome).
    let mut branch_const_firings: Vec<BranchConstFiring> = Vec::new();
    for pb in &proc_bufs {
        branch_const_firings.extend(check_branch_const(&pb.name, &pb.buf.items));
    }
    branch_const_firings.sort_by(|a, b| (&a.proc, a.span.start).cmp(&(&b.proc, b.span.start)));

    // §3 the DECLARED machine-state tier. Regions are recovered per proc from the
    // marks a `with` bracket plants; the per-region escape / entry-skip /
    // reacquire proofs are the per-file gate's (they need only one body), so what
    // runs HERE is the inherently cross-proc half: `[context.unsatisfied]`, the
    // claim census, and the bracket census.
    let declared_contexts = collect_context_names(files);
    let mut proc_regions: BTreeMap<String, Vec<Region>> = BTreeMap::new();
    let mut context_regions: Vec<(String, String)> = Vec::new();
    let mut bus_contexts: BTreeSet<String> = BTreeSet::new();
    let mut context_firings: Vec<ContextFiring> = Vec::new();
    for pb in &proc_bufs {
        // ONE mark scan per proc feeds the census, the bus-context
        // identification, and the bracket proofs.
        let (regions, mark_firings) = regions_of(&pb.name, &pb.buf.items);
        for r in &regions {
            context_regions.push((pb.name.clone(), r.ctx.clone()));
            if crate::z80_bus::region_acquires_bus(&pb.buf.items, r) {
                bus_contexts.insert(r.ctx.clone());
            }
        }
        // Cpu::M68000 is exact, not a default: `proc_bufs` excludes `(cpu: z80)`
        // modules (they collect into `z80_proc_bufs`), so every buf here is 68k.
        // A `with` in a Z80 module is therefore proven by the PER-FILE gate only,
        // which threads the module's real CPU — recorded in the census's doc.
        context_firings.extend(check_regions(
            &pb.name, &pb.buf.items, Cpu::M68000, &regions, mark_firings,
        ));
        proc_regions.insert(pb.name.clone(), regions);
    }
    context_regions.sort();
    context_firings.sort_by(|a, b| {
        (&a.proc, &a.ctx, a.span.start).cmp(&(&b.proc, &b.ctx, b.span.start))
    });

    let mut context_unsatisfied: Vec<ContextUnsatisfied> = Vec::new();
    let mut context_discharged: Vec<(String, String, String)> = Vec::new();
    let mut unknown_context_refs: Vec<(String, String, Span)> = Vec::new();
    let mut context_claim_sites: Vec<(String, String, String)> = Vec::new();
    for file in files {
        collect_context_claims(&file.items, &declared_contexts, &mut context_claim_sites, &mut unknown_context_refs);
    }
    context_claim_sites.sort();
    unknown_context_refs.sort_by(|a, b| (&a.0, &a.1, a.2.start).cmp(&(&b.0, &b.1, b.2.start)));
    for pb in &proc_bufs {
        let Some(node) = nodes.get(&pb.name) else { continue };
        // The contexts active for the WHOLE body: the caller's own declared
        // requirement (its callers guarantee it) plus its grants (the trust root).
        let always: BTreeSet<String> = node.requires.union(&node.grants).cloned().collect();
        let regions = proc_regions.get(&pb.name).map(|r| r.as_slice()).unwrap_or(&[]);
        for (idx, it) in pb.buf.items.iter().enumerate() {
            let CodeItem::Instr { mnemonic, ops, span, author, .. } = it else { continue };
            // An authored divergent rail contributes no callee edge — here it
            // also discharges no context obligation (the rail's target is the
            // compiler's own contract, not a call the host proc makes).
            if matches!(author, crate::value::ItemAuthor::AssertDesugar) {
                continue;
            }
            if !CALL_MNEMONICS.contains(&mnemonic.as_str())
                && !TAIL_MNEMONICS.contains(&mnemonic.as_str())
            {
                continue;
            }
            let Some(target) = call_target_sym(ops) else { continue };
            let Some(callee) = nodes.get(&target) else { continue };
            if callee.requires.is_empty() {
                continue;
            }
            let mut active = always.clone();
            active.extend(bracketed_at(regions, idx));
            for ctx in &callee.requires {
                if active.contains(ctx) {
                    context_discharged.push((pb.name.clone(), target.clone(), ctx.clone()));
                } else {
                    context_unsatisfied.push(ContextUnsatisfied {
                        proc: pb.name.clone(),
                        callee: target.clone(),
                        ctx: ctx.clone(),
                        span: *span,
                    });
                }
            }
        }
    }
    context_unsatisfied.sort_by(|a, b| {
        (&a.proc, &a.callee, &a.ctx, a.span.start).cmp(&(&b.proc, &b.callee, &b.ctx, b.span.start))
    });
    context_discharged.sort();

    // [bus.*] — the item-4 core Z80-bus machine-state lint (byte-neutral). The
    // entry seed comes from the DECLARED tier: a proc that requires/grants a
    // bus-holding context runs with the bus held at entry, a fact the inference
    // tier cannot derive and `[context.unsatisfied]` checks at every call site.
    let mut bus_firings: Vec<BusFiring> = Vec::new();
    for pb in &proc_bufs {
        // Seeded from `requires` ONLY. `grants` is an explicitly UNVERIFIED trust
        // root (§3.2 — the assembler cannot check hardware dispatch), and this
        // seed gates a crash-class check: `[bus.vdp-write-unstopped]` fires only
        // on a definite Running, so a wrong `Held` silences it for a whole proc.
        // A requirement at least has `[context.unsatisfied]` behind it at every
        // call site.
        let entry = match nodes.get(&pb.name) {
            Some(n) if n.requires.iter().any(|c| bus_contexts.contains(c)) => BusEntry::Held,
            _ => BusEntry::Unknown,
        };
        bus_firings.extend(check_bus_state(&pb.name, &pb.buf.items, entry));
    }
    bus_firings.sort_by(|a, b| (&a.proc, a.span.start).cmp(&(&b.proc, b.span.start)));

    // S2-D6 U4 — every `@allow("clobbers.unanalyzable", "<reason>")` in force,
    // listed so the escape-hatch surface stays audited (never silent).
    let mut unanalyzable_allows: Vec<(String, String)> = Vec::new();
    for file in files {
        collect_unanalyzable_allows(file, &file.items, &mut unanalyzable_allows);
    }
    unanalyzable_allows.sort();
    unanalyzable_allows.dedup();

    // The SR-write authorship census: every write-form instruction whose
    // destination is SR, with the author the emitting construct recorded
    // (`ItemAuthor`). 68k `proc_bufs` only — SR is a 68k register, and the
    // `[proc.sr-undeclared]` lint this census audits is 68k-only.
    let mut sr_writes: Vec<(String, crate::value::ItemAuthor, Span)> = Vec::new();
    for pb in &proc_bufs {
        for item in &pb.buf.items {
            let CodeItem::Instr { mnemonic, ops, span, author, .. } = item else { continue };
            if crate::lower::writes_dest_register(mnemonic)
                && matches!(ops.last(), Some(crate::value::CodeOperand::Sr))
            {
                sr_writes.push((pb.name.clone(), author.clone(), *span));
            }
        }
    }
    sr_writes.sort_by(|a, b| (&a.0, a.2.start).cmp(&(&b.0, b.2.start)));

    // The AbsSym call-edge census + the excluded-rail hole census.
    // ONE walk over the same 68k `proc_bufs` the closure's edge builder reads: an
    // abs-long `jsr/jmp (sym).l` call/tail with a global target. AssertDesugar-
    // authored rails split OUT of the edge set exactly as at the edge builder,
    // and are recorded as holes when their target resolves to no proc/extern —
    // the counterfactual residue the exclusion suppresses.
    let mut abs_call_edges: Vec<(String, String)> = Vec::new();
    let mut authored_rail_holes: BTreeSet<String> = BTreeSet::new();
    for pb in &proc_bufs {
        for item in &pb.buf.items {
            let CodeItem::Instr { mnemonic, ops, author, .. } = item else { continue };
            if !CALL_MNEMONICS.contains(&mnemonic.as_str())
                && !TAIL_MNEMONICS.contains(&mnemonic.as_str())
            {
                continue;
            }
            if !ops.iter().any(|o| matches!(o, CodeOperand::AbsSym { .. })) {
                continue;
            }
            let Some(target) = call_target_sym(ops) else { continue };
            if matches!(author, crate::value::ItemAuthor::AssertDesugar) {
                if !nodes.contains_key(crate::closure::resolve_callee_key(&nodes, &target)) {
                    authored_rail_holes.insert(target);
                }
            } else {
                abs_call_edges.push((pb.name.clone(), target));
            }
        }
    }
    abs_call_edges.sort();
    abs_call_edges.dedup();

    // Both body-eval passes (68k PASS 2 + the Z80 flag pass) have contributed by
    // here. Exact-duplicate rows collapse (a template instantiated twice evaluates
    // the same condition twice); distinct sites stay distinct.
    comptime_unresolved.sort_by(|a, b| {
        (&a.0, &a.1, a.2.source.0, a.2.start, a.2.end)
            .cmp(&(&b.0, &b.1, b.2.source.0, b.2.start, b.2.end))
    });
    comptime_unresolved.dedup();

    ContractReport {
        closure,
        firings,
        flag_firings,
        flag_firings_verified_credit: flag_firings_verified,
        extern_collisions,
        proc_count,
        extern_count,
        contract_type_count,
        dead_saves,
        input_firings,
        live_clobbered_firings,
        out_firings,
        inout_firings,
        survives_firings,
        survives_claim_sites,
        unresolvable_out_types,
        typed_out_slots,
        verified_uncond_out,
        verified_cond_out,
        verified_inout,
        dropped_instrs,
        dropped_by_proc,
        comptime_unresolved,
        slot_firings,
        branch_const_firings,
        bus_firings,
        context_firings,
        context_unsatisfied,
        context_discharged,
        unknown_context_refs,
        context_claim_sites,
        context_regions,
        bus_contexts,
        unanalyzable_allows,
        sr_writes,
        abs_call_edges,
        authored_rail_holes,
        word_preserve_firings,
        word_preserve_claims,
        z80_out_firings,
        z80_verified_out,
        z80_out_claims,
    }
}

/// One `[context.unsatisfied]` firing (§3.3): at `span` in `proc`, the call to
/// `callee` is not dominated by an active `ctx`, which `callee` requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextUnsatisfied {
    /// The calling proc.
    pub proc: String,
    /// The callee whose requirement went undischarged.
    pub callee: String,
    /// The context the callee requires.
    pub ctx: String,
    /// The call site.
    pub span: Span,
}

/// Every `context` NAME the corpus declares (§3.1) — recursing `section {}`
/// bodies like every other collector here. The set a `requires`/`grants` clause
/// is validated against; the FLAVOR check (`grants` of an acquired context) is
/// the per-file gate's, which has the decl in hand.
fn collect_context_names(files: &[ast::File]) -> BTreeSet<String> {
    fn walk(items: &[Item], out: &mut BTreeSet<String>) {
        for item in items {
            match item {
                Item::Context(c) => {
                    out.insert(c.name.clone());
                }
                Item::Section(s) => walk(&s.items, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    for file in files {
        walk(&file.items, &mut out);
    }
    out
}

/// Collect every proc's `requires`/`grants` clause into the claim census, and
/// any clause naming an undeclared context into `unknown`.
fn collect_context_claims(
    items: &[Item],
    declared: &BTreeSet<String>,
    claims: &mut Vec<(String, String, String)>,
    unknown: &mut Vec<(String, String, Span)>,
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                for (kind, list) in [("requires", &p.requires), ("grants", &p.grants)] {
                    for (ctx, span) in list.iter() {
                        claims.push((p.name.clone(), kind.to_string(), ctx.clone()));
                        if !declared.contains(ctx) {
                            unknown.push((p.name.clone(), ctx.clone(), *span));
                        }
                    }
                }
            }
            Item::ExternProc(e) => {
                for (ctx, span) in e.sig.requires.iter() {
                    claims.push((e.name.clone(), "requires".to_string(), ctx.clone()));
                    if !declared.contains(ctx) {
                        unknown.push((e.name.clone(), ctx.clone(), *span));
                    }
                }
            }
            Item::Section(s) => collect_context_claims(&s.items, declared, claims, unknown),
            _ => {}
        }
    }
}

/// Recurse the item list (into `section {}` blocks) collecting every proc for
/// which an `@allow("clobbers.unanalyzable", "<reason>")` is IN FORCE — whether
/// declared on the proc itself or on its module — as `(name, reason)` (U4).
/// Mirrors the corpus walk's section recursion.
fn collect_unanalyzable_allows(file: &ast::File, items: &[Item], out: &mut Vec<(String, String)>) {
    for item in items {
        match item {
            Item::Proc(p) => {
                if let Some(reason) = unanalyzable_reason_in_force(file, p) {
                    out.push((p.name.clone(), reason));
                }
            }
            Item::Section(s) => collect_unanalyzable_allows(file, &s.items, out),
            _ => {}
        }
    }
}

/// The leaf-segment name of a `Type` when it is a domain NEWTYPE — the payload of
/// a G5 typed slot. A `u8`/`u16`/`fixed<…>` primitive, a `*Ptr`, or any type
/// whose leaf is not a declared newtype yields `None` (that slot checks nothing).
fn newtype_of(ty: &ast::Type, newtypes: &BTreeSet<String>) -> Option<String> {
    if let ast::Type::Named(path) = ty {
        if let Some(leaf) = path.segments.last() {
            if newtypes.contains(leaf) {
                return Some(leaf.clone());
            }
        }
    }
    None
}

/// The register-file index (`d0`..`d7` = 0..7, `a0`..`a7` = 8..15) of a slot's
/// register spelling.
fn slot_reg_idx(name: &str) -> Option<usize> {
    Reg::from_name(name).map(|r| r as usize)
}

/// Every declared newtype's UNDERLYING type, by BARE name (recursing sections).
/// Read only by [`out_claim_of`], which needs the payload a newtype erases to and
/// not merely whether the name is one.
///
/// The key is the bare leaf name, matching how every other G5 consumer resolves a
/// type (`newtype_of` reads `path.segments.last()`). That makes the table
/// UNSCOPED: two modules declaring the same newtype name share one row. A NAME
/// COLLISION is therefore resolved by [`collect_out_widths`] toward the widest
/// reading rather than by declaration order — see the merge there. Module-scoping
/// the whole type layer is the real repair and is ledgered; it cannot be done for
/// widths alone without creating a second type authority scoped differently from
/// the one the slot check uses.
fn collect_newtype_underlying(files: &[ast::File]) -> BTreeMap<String, Vec<ast::Type>> {
    fn walk(items: &[Item], out: &mut BTreeMap<String, Vec<ast::Type>>) {
        for item in items {
            match item {
                Item::Newtype(n) => {
                    out.entry(n.name.clone()).or_default().push(n.underlying.clone());
                }
                Item::Section(s) => walk(&s.items, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    for file in files {
        walk(&file.items, &mut out);
    }
    out
}

/// The [`OutClaim`] a value of type `ty` states — how many low-order bytes of a
/// register it occupies, on each of the claim's two sides.
///
/// **A NEWTYPE NARROWS to its underlying type**, transitively (`EntryRef` →
/// `EntryIndex` → `i16` → two bytes). It has to: `out(d0: EntryRef)` and
/// `out(d0: u16)` are two spellings of ONE slot, so if a domain type did not carry
/// its width, a typed out could never state one — the author's only route to a
/// narrow claim would be deleting the domain type, which is exactly the
/// declaration the `[call.slot-type-mismatch]` slice needs. Narrowing also keeps a
/// single authority for how wide a `SectionId` is.
///
/// Anything whose width is not derivable here — an unknown name, a struct, an
/// enum, an array, a tuple — answers [`OutWidth::L`]. That is the CONSERVATIVE
/// answer, not a fallback: L is the bare-`out` obligation, so an unresolvable type
/// relaxes nothing and the check stays exactly as strict as it was before anyone
/// wrote a type.
///
/// `depth` bounds a newtype cycle (`newtype A = B` / `newtype B = A`). Such a
/// corpus is rejected elsewhere; here it must merely not hang.
fn out_claim_of(
    ty: &ast::Type,
    newtypes: &BTreeMap<String, Vec<ast::Type>>,
    depth: usize,
) -> OutClaim {
    if depth == 0 {
        return OutClaim::exact(OutWidth::L);
    }
    match ty {
        // A refinement narrows the VALUE range, never the storage — the width is
        // the underlying type's.
        ast::Type::Refined(inner, _, _) => out_claim_of(inner, newtypes, depth - 1),
        // A 68k pointer is a full address register's worth.
        ast::Type::Ptr(_) => OutClaim::exact(OutWidth::L),
        ast::Type::Fixed { i, f } => OutClaim::exact(bits_to_width(i + f)),
        ast::Type::Named(path) => {
            let Some(leaf) = path.segments.last() else { return OutClaim::exact(OutWidth::L) };
            match leaf.as_str() {
                "u8" | "i8" => OutClaim::exact(OutWidth::B),
                "u16" | "i16" | "u16le" => OutClaim::exact(OutWidth::W),
                "u32" | "i32" => OutClaim::exact(OutWidth::L),
                // A name declared more than once corpus-wide is AMBIGUOUS, and the
                // two sides of the claim resolve it in opposite directions: the
                // proc's own obligation takes the WIDEST reading, a caller's credit
                // the NARROWEST. Neither number alone is fail-safe — taking the
                // widest for both makes a caller's verdict depend on an unrelated
                // module's declaration, and through an EXTERN (verified by axiom,
                // with no body to re-prove it) that is a plain over-credit.
                _ => match newtypes.get(leaf) {
                    Some(unders) => unders
                        .iter()
                        .map(|u| out_claim_of(u, newtypes, depth - 1))
                        .reduce(OutClaim::merge)
                        .unwrap_or(OutClaim::exact(OutWidth::L)),
                    None => OutClaim::exact(OutWidth::L),
                },
            }
        }
        ast::Type::Array(_, _) | ast::Type::Tuple(_) => OutClaim::exact(OutWidth::L),
    }
}

/// The register width holding `bits`, rounded UP to a 68k operand size and capped
/// at a long — a register has no wider claim to make.
fn bits_to_width(bits: u32) -> OutWidth {
    match bits {
        0..=8 => OutWidth::B,
        9..=16 => OutWidth::W,
        _ => OutWidth::L,
    }
}

/// Build the corpus-wide `out(dN: T)` width map: proc name → register → the width
/// its declared type claims. A proc with no typed out is absent, and a register
/// with no type is absent from its proc's row — both mean the bare 32-bit claim.
///
/// Externs are included: their declared outs are §3 boundary axioms that the
/// fixpoint seeds VERIFIED, so a typed extern out must credit its callers at the
/// declared width and not a long.
#[allow(clippy::type_complexity)]
fn collect_out_widths(
    files: &[ast::File],
) -> (OutWidthMap, Vec<(String, String, String)>, Vec<(String, String)>) {
    // One proc's row: EVERY declared out register, typed ones at their type's
    // width and untyped ones at the bare 32-bit claim.
    //
    // The untyped registers are carried EXPLICITLY, even though an absent register
    // already reads as `L`. They are what makes the collision merge below able to
    // see a BARE declaration at all: without them a duplicate proc name whose
    // other declaration is untyped would contribute no row, and the typed one
    // would stand alone and relax it.
    fn row(
        out: Option<&[(String, Option<String>)]>,
        out_types: &[(String, ast::Type, Span)],
        newtypes: &BTreeMap<String, Vec<ast::Type>>,
    ) -> BTreeMap<String, OutClaim> {
        let mut row: BTreeMap<String, OutClaim> = expand_reglist_regs(out.unwrap_or(&[]))
            .into_iter()
            .filter_map(|name| {
                Reg::from_name(&name).map(|r| (r.to_string(), OutClaim::exact(OutWidth::L)))
            })
            .collect();
        for (reg, ty, _) in out_types {
            // Canonical `d0`..`a7` spelling — the same names the production state
            // is keyed by. A non-register endpoint (a `carry:` flag result never
            // reaches here) drops out.
            let Some(r) = Reg::from_name(reg) else { continue };
            // An ADDRESS-register result is pinned to `L` whatever its type says.
            // Every 68k write to an address register covers all 32 bits, so `L` is
            // both the only width it can be obligated at and the only width it can
            // honestly credit — a narrower reading would cap callers below what the
            // hardware produces. The TYPE still flows to `collect_typed_slots`, so
            // a domain type on an address result keeps its `[call.slot-type-mismatch]`
            // meaning; only its WIDTH is ignored, because an address register has no
            // width to state.
            let claim = if (r as usize) >= 8 {
                OutClaim::exact(OutWidth::L)
            } else {
                out_claim_of(ty, newtypes, 16)
            };
            row.insert(r.to_string(), claim);
        }
        row
    }
    // Merge `r` into `out[name]`, folding every register mentioned by EITHER row
    // through `OutClaim::merge` — widest `strict`, narrowest `credit` — and
    // treating a register absent from one row as its bare 32-bit claim. Proc names are meant to be unique corpus-wide (a
    // duplicate is ill-formed and `[proc]`-level checks say so), so this is a
    // fail-safe rather than a feature: without it one file's typed `out(d0: u8)`
    // would relax another file's BARE `out(d0)` under the same name, which is the
    // one construction where writing a type anywhere changes a bare declaration's
    // verdict somewhere else.
    fn merge(out: &mut OutWidthMap, name: &str, r: BTreeMap<String, OutClaim>) {
        match out.get_mut(name) {
            None => {
                out.insert(name.to_string(), r);
            }
            Some(existing) => {
                let regs: BTreeSet<String> =
                    existing.keys().chain(r.keys()).cloned().collect();
                let bare = OutClaim::exact(OutWidth::L);
                let merged = regs
                    .into_iter()
                    .map(|reg| {
                        let a = existing.get(&reg).copied().unwrap_or(bare);
                        let b = r.get(&reg).copied().unwrap_or(bare);
                        (reg, a.merge(b))
                    })
                    .collect();
                *existing = merged;
            }
        }
    }
    fn walk(items: &[Item], newtypes: &BTreeMap<String, Vec<ast::Type>>, out: &mut OutWidthMap) {
        for item in items {
            match item {
                Item::Proc(p) => {
                    let r = row(p.out.as_deref(), &p.out_types, newtypes);
                    if !r.is_empty() {
                        merge(out, &p.name, r);
                    }
                }
                Item::ExternProc(e) => {
                    let r = row(e.sig.out.as_deref(), &e.sig.out_types, newtypes);
                    if !r.is_empty() {
                        merge(out, &e.name, r);
                    }
                }
                Item::Section(s) => walk(&s.items, newtypes, out),
                _ => {}
            }
        }
    }
    let newtypes = collect_newtype_underlying(files);
    let mut out = OutWidthMap::new();
    for file in files {
        walk(&file.items, &newtypes, &mut out);
    }
    // The same walk again for the unresolvable-type report. Kept separate from the
    // width build so the width answer stays a pure function of the type: a name
    // that cannot be resolved still answers the bare claim, and reporting it must
    // not change what it answers.
    let mut unresolvable = Vec::new();
    let mut slots = Vec::new();
    fn scan(
        items: &[Item],
        newtypes: &BTreeMap<String, Vec<ast::Type>>,
        out: &mut Vec<(String, String, String)>,
        slots: &mut Vec<(String, String)>,
    ) {
        fn each(
            owner: &str,
            out_types: &[(String, ast::Type, Span)],
            newtypes: &BTreeMap<String, Vec<ast::Type>>,
            out: &mut Vec<(String, String, String)>,
            slots: &mut Vec<(String, String)>,
        ) {
            for (reg, ty, _) in out_types {
                let Some(r) = Reg::from_name(reg) else { continue };
                slots.push((owner.to_string(), r.to_string()));
                if (r as usize) >= 8 {
                    continue; // an address result is pinned to a long; its type states no width
                }
                if let Some(name) = unresolvable_leaf(ty, newtypes, 16) {
                    out.push((owner.to_string(), r.to_string(), name));
                }
            }
        }
        for item in items {
            match item {
                Item::Proc(p) => each(&p.name, &p.out_types, newtypes, out, slots),
                Item::ExternProc(e) => each(&e.name, &e.sig.out_types, newtypes, out, slots),
                Item::ContractType(t) => each(&t.name, &t.sig.out_types, newtypes, out, slots),
                Item::Section(s) => scan(&s.items, newtypes, out, slots),
                _ => {}
            }
        }
    }
    for file in files {
        scan(&file.items, &newtypes, &mut unresolvable, &mut slots);
    }
    unresolvable.sort();
    slots.sort();
    (out, unresolvable, slots)
}

/// The corpus-wide `inout(rN: T)` width map: proc → register → the width its type
/// claims — the [`collect_out_widths`] analogue for the in-out facet. A bare
/// `inout(rN)` (or an address register whatever its type) is `L`; a typed data
/// register narrows to its type. This is what makes `inout(d5: u16)` charge the
/// exit-production check at `.w`, so `addq.w` verifies and `addq.b` breaks.
fn collect_inout_widths(files: &[ast::File]) -> OutWidthMap {
    fn row(
        inout: Option<&[(String, Option<String>)]>,
        inout_types: &[(String, ast::Type, Span)],
        newtypes: &BTreeMap<String, Vec<ast::Type>>,
    ) -> BTreeMap<String, OutClaim> {
        let mut row: BTreeMap<String, OutClaim> = expand_reglist_regs(inout.unwrap_or(&[]))
            .into_iter()
            .filter_map(|name| {
                Reg::from_name(&name).map(|r| (r.to_string(), OutClaim::exact(OutWidth::L)))
            })
            .collect();
        for (reg, ty, _) in inout_types {
            let Some(r) = Reg::from_name(reg) else { continue };
            let claim = if (r as usize) >= 8 {
                OutClaim::exact(OutWidth::L) // an address register has no width to state
            } else {
                out_claim_of(ty, newtypes, 16)
            };
            row.insert(r.to_string(), claim);
        }
        row
    }
    fn walk(items: &[Item], newtypes: &BTreeMap<String, Vec<ast::Type>>, out: &mut OutWidthMap) {
        for item in items {
            match item {
                Item::Proc(p) => {
                    let r = row(p.inout.as_deref(), &p.inout_types, newtypes);
                    if !r.is_empty() {
                        out.insert(p.name.clone(), r);
                    }
                }
                Item::ExternProc(e) => {
                    let r = row(e.sig.inout.as_deref(), &e.sig.inout_types, newtypes);
                    if !r.is_empty() {
                        out.insert(e.name.clone(), r);
                    }
                }
                Item::Section(s) => walk(&s.items, newtypes, out),
                _ => {}
            }
        }
    }
    let newtypes = collect_newtype_underlying(files);
    let mut out = OutWidthMap::new();
    for file in files {
        walk(&file.items, &newtypes, &mut out);
    }
    out
}

/// The leaf NAME of a type whose width [`out_claim_of`] cannot derive, or `None`
/// when it can. Mirrors that function's shape exactly — a divergence between the
/// two would report a type that resolves, or stay silent about one that does not.
fn unresolvable_leaf(
    ty: &ast::Type,
    newtypes: &BTreeMap<String, Vec<ast::Type>>,
    depth: usize,
) -> Option<String> {
    if depth == 0 {
        return Some("<type nesting too deep to resolve>".to_string());
    }
    match ty {
        ast::Type::Refined(inner, _, _) => unresolvable_leaf(inner, newtypes, depth - 1),
        // A pointer is a long by the 68k register rule, not by resolution.
        ast::Type::Ptr(_) => None,
        ast::Type::Fixed { .. } => None,
        ast::Type::Named(path) => {
            let leaf = path.segments.last()?;
            match leaf.as_str() {
                "u8" | "i8" | "u16" | "i16" | "u16le" | "u32" | "i32" => None,
                _ => match newtypes.get(leaf) {
                    // A colliding name resolves iff EVERY reading does.
                    Some(unders) => {
                        unders.iter().find_map(|u| unresolvable_leaf(u, newtypes, depth - 1))
                    }
                    None => Some(leaf.clone()),
                },
            }
        }
        ast::Type::Array(_, _) | ast::Type::Tuple(_) => {
            Some("<an array or tuple has no register width>".to_string())
        }
    }
}

/// Collect every declared newtype NAME across the corpus (recursing sections).
/// The set that gates which param/out slots are domain-typed.
fn collect_newtype_names(files: &[ast::File]) -> BTreeSet<String> {
    fn walk(items: &[Item], out: &mut BTreeSet<String>) {
        for item in items {
            match item {
                Item::Newtype(n) => {
                    out.insert(n.name.clone());
                }
                Item::Section(s) => walk(&s.items, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    for file in files {
        walk(&file.items, &mut out);
    }
    out
}

/// Map each niche-option newtype (`newtype SlotRef = SlotId ? $FF`) to its
/// PAYLOAD newtype name (`SlotId`). The slot check reads it to recognise a
/// niche-option flowing into its own payload slot as `[option.unguarded-use]`
/// rather than the generic mismatch. An option whose payload is a bare prim
/// (no newtype name) is absent — the slot check tracks domain newtypes only.
fn collect_option_payloads(
    files: &[ast::File],
    newtypes: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    fn walk(items: &[Item], newtypes: &BTreeSet<String>, out: &mut BTreeMap<String, String>) {
        for item in items {
            match item {
                Item::Newtype(n) if n.sentinel.is_some() => {
                    if let Some(payload) = newtype_of(&n.underlying, newtypes) {
                        out.insert(n.name.clone(), payload);
                    }
                }
                Item::Section(s) => walk(&s.items, newtypes, out),
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    for file in files {
        walk(&file.items, newtypes, &mut out);
    }
    out
}

/// Build the per-callee typed-slot maps: `typed_params[name]` = the proc's
/// newtype-typed param slots `(reg_idx, newtype)`; `typed_out[name]` = its
/// `out(dN: Type)` slots. A proc with no domain-typed slot is absent from both.
#[allow(clippy::type_complexity)]
fn collect_typed_slots(
    files: &[ast::File],
    newtypes: &BTreeSet<String>,
) -> (BTreeMap<String, Vec<(usize, String)>>, BTreeMap<String, Vec<(usize, String)>>) {
    fn walk(
        items: &[Item],
        newtypes: &BTreeSet<String>,
        params: &mut BTreeMap<String, Vec<(usize, String)>>,
        outs: &mut BTreeMap<String, Vec<(usize, String)>>,
    ) {
        for item in items {
            match item {
                Item::Proc(p) => {
                    let mut ps = Vec::new();
                    for (reg, ty, _) in &p.params {
                        if let (Some(idx), Some(nt)) = (slot_reg_idx(reg), newtype_of(ty, newtypes))
                        {
                            ps.push((idx, nt));
                        }
                    }
                    if !ps.is_empty() {
                        params.insert(p.name.clone(), ps);
                    }
                    let mut os = Vec::new();
                    for (reg, ty, _) in &p.out_types {
                        if let (Some(idx), Some(nt)) = (slot_reg_idx(reg), newtype_of(ty, newtypes))
                        {
                            os.push((idx, nt));
                        }
                    }
                    if !os.is_empty() {
                        outs.insert(p.name.clone(), os);
                    }
                }
                Item::Section(s) => walk(&s.items, newtypes, params, outs),
                _ => {}
            }
        }
    }
    let mut params = BTreeMap::new();
    let mut outs = BTreeMap::new();
    for file in files {
        walk(&file.items, newtypes, &mut params, &mut outs);
    }
    (params, outs)
}

/// PASS 1 of the corpus type environment: clone every DECLARATION item that
/// [`Evaluator::index_items`](crate::eval) resolves names against — everything
/// EXCEPT proc/extern/contract-type/script BODIES (indexing a body as ambient
/// adds nothing and would duplicate it) and the non-declaration directives
/// (`use`/`ensure`/`align`/comptime tests). Recurses `section { … }` so a
/// section-nested declaration joins the flat namespace exactly as the evaluator
/// treats it.
fn collect_env(items: &[Item], out: &mut Vec<Item>) {
    for item in items {
        match item {
            Item::Const(_)
            | Item::Equ(_)
            | Item::Enum(_)
            | Item::Bitfield(_)
            | Item::Struct(_)
            | Item::Offsets(_)
            | Item::Table(_)
            | Item::Dispatch(_)
            | Item::Vars(_)
            | Item::Data(_)
            | Item::ComptimeFn(_)
            | Item::Newtype(_)
            // A `context` decl is what a `with` bracket resolves against. Without
            // it here the walk's CodeBufs would silently LACK every bracket's
            // acquire/release — a body missing its bus toggles, invisible to the
            // `dropped_instrs` pin (nothing is dropped; the statement simply
            // lowers bare). Every downstream net would then under-approximate.
            | Item::Context(_) => out.push(item.clone()),
            Item::Section(s) => collect_env(&s.items, out),
            _ => {}
        }
    }
}

/// A proc's evaluated CodeBuf + the call-site spans carrying `@discards`, held
/// for the §6 caller-side flag checks (run after the whole corpus is walked so
/// every callee's flag/conditional contract is known).
struct ProcBuf {
    name: String,
    buf: CodeBuf,
    discarded: Vec<Span>,
    span: Span,
    /// The callee-preserves oracle round's inputs (built once from the ProcDecl):
    /// the declared `preserves` registers to re-verify under the closure oracle,
    /// and the full declared set to credit iff all round-trip. Empty when the proc
    /// declares no `preserves` (or a malformed one).
    preserve_check: Vec<Reg>,
    preserve_names: BTreeSet<String>,
    /// The §6 partial-width word-facet registers (`preserves(dN.w)`) to re-verify
    /// under the closure oracle — the deferred, call-blocked low-word round-trips
    /// the per-file byte gate left silent. Empty when the proc declares no `.w`
    /// facet (or a malformed `preserves`).
    preserve_word_check: Vec<Reg>,
    /// The proc's declared `falls_into` successor (if any) — the tail-credit
    /// context the callee-preserves oracle round feeds to `verify_preserved` so a
    /// fall-through into a preserving successor is credited.
    falls_into: Option<String>,
    /// The `(register, cc)` pairs that can carry a §7.1 survives claim
    /// ([`ast::ProcDecl::cond_out_pairs`]) — canonical, and already excluding a
    /// register ALSO named unconditionally (whose out is unconditional, and for
    /// which the claim would have no legal remedy). The raw `out_cond` list
    /// cannot tell the difference.
    cond_out_pairs: Vec<(String, String)>,
}

/// The set of status flags a decl's `out(carry: name)` clauses name.
fn flags_of(out_flags: &[ast::FlagResult]) -> BTreeSet<String> {
    out_flags.iter().map(|f| f.flag.clone()).collect()
}

/// The `(reg, cc)` pairs a decl's `out(rN if cc)` clauses name.
/// The register is CANONICALIZED through the 68k register file (`sp` → `a7`) —
/// this pass is 68k-only (a `(cpu: z80)` module is skipped above).
///
/// ONE consumer needs it: the `callee_uncond_out` subtraction compares these
/// names against a set expanded by that file, so a raw `sp` filters nothing and
/// credits a conditional out as an unconditional one on a shipping ERROR gate.
/// The other two consumers — `conditional_out_edge_credits` and `check_out`'s
/// cond list — read these names through `Reg::from_name`, which is already alias
/// tolerant. Canonicalizing at COLLECTION makes the invariant a property of the
/// map rather than a duty each consumer must remember. An unrecognizable name
/// keeps its raw spelling (it matches nothing downstream, which is the same
/// outcome as dropping it, minus the silent loss).
fn conds_of(out_cond: &[ast::CondResult]) -> Vec<(String, String)> {
    out_cond
        .iter()
        .map(|c| {
            let reg = ast::canonical_contract_reg(&c.reg, crate::regfile::RegFile::M68k)
                .unwrap_or_else(|| c.reg.clone());
            (reg, c.cc.clone())
        })
        .collect()
}

/// The spans of a proc body's call instructions carrying `@discards` (recursing
/// comptime-`if` branches, like [`collect_indirect_sites`]). A `@discards` inside
/// a comptime-fn template body is not seen (the AST-body limitation the walk
/// already carries for indirect sites); no corpus call site discards today.
fn collect_discarded(body: &[AsmStmt], out: &mut Vec<Span>) {
    for stmt in body {
        match stmt {
            AsmStmt::Instr(i) if i.discards.is_some() => out.push(i.span),
            AsmStmt::If { then, els, .. } => {
                collect_discarded(then, out);
                if let Some(e) = els {
                    collect_discarded(e, out);
                }
            }
            AsmStmt::With { body, .. } => collect_discarded(body, out),
            _ => {}
        }
    }
}

/// The per-file inputs the register-preserve oracle threads into `check_preserves`
/// at its two corpus call sites (`preserve_oracle_inputs`, `verified_preserves_regs`):
/// the `@noreturn` symbols and the SR-mask preservers of the file being walked.
/// Built once per file from the SAME sources the primary `lower` checker uses
/// (`collect_noreturn_symbols` / `collect_sr_mask_preservers`), so the corpus
/// register verdict stays consistent with the primary path the day a proc both
/// register-preserves AND claims the mask through an external tail. Inert on the
/// frozen corpus (no such proc), measured so.
struct PreserveInputs<'a> {
    noreturn: &'a BTreeSet<String>,
    sr_mask_preservers: &'a BTreeMap<String, BTreeSet<String>>,
}

/// Recurse the item list (into `section {}` blocks), registering every proc /
/// extern proc / contract type.
#[allow(clippy::too_many_arguments)]
fn collect_items(
    items: &[Item],
    file: &ast::File,
    nodes: &mut BTreeMap<String, ProcNode>,
    types: &mut BTreeMap<String, RegEffect>,
    extern_names: &mut BTreeSet<String>,
    proc_names: &mut BTreeSet<String>,
    extern_spans: &mut BTreeMap<String, Span>,
    counter: &mut u32,
    flag_callees: &mut BTreeMap<String, BTreeSet<String>>,
    cond_callees: &mut BTreeMap<String, Vec<(String, String)>>,
    proc_bufs: &mut Vec<ProcBuf>,
    defines: &[(String, i128)],
    env: &[Item],
    dropped_by_proc: &mut Vec<(String, usize)>,
    comptime_unresolved: &mut Vec<(String, String, Span)>,
    contracts: &crate::contract::InterfaceEnv,
    preserve_inputs: &PreserveInputs,
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                proc_names.insert(p.name.clone());
                let (node, buf, dropped, unresolved) =
                    proc_node(p, file, counter, defines, env, contracts, preserve_inputs);
                if dropped > 0 {
                    dropped_by_proc.push((p.name.clone(), dropped));
                }
                comptime_unresolved
                    .extend(unresolved.into_iter().map(|(n, s)| (p.name.clone(), n, s)));
                nodes.insert(p.name.clone(), node);
                // §6 flag / conditional contracts this proc exposes to callers.
                let flags = flags_of(&p.out_flags);
                if !flags.is_empty() {
                    flag_callees.insert(p.name.clone(), flags);
                }
                let conds = conds_of(&p.out_cond);
                if !conds.is_empty() {
                    cond_callees.insert(p.name.clone(), conds);
                }
                // Stash the CodeBuf + discard sites for the post-walk checks.
                if let Some(buf) = buf {
                    let (preserve_check, preserve_names, preserve_word_check) =
                        preserve_oracle_inputs(
                            p,
                            &buf,
                            preserve_inputs.noreturn,
                            preserve_inputs.sr_mask_preservers,
                        );
                    let mut discarded = Vec::new();
                    collect_discarded(&p.body, &mut discarded);
                    proc_bufs.push(ProcBuf {
                        name: p.name.clone(),
                        buf,
                        discarded,
                        span: p.span,
                        preserve_check,
                        preserve_names,
                        preserve_word_check,
                        falls_into: p.falls_into.clone(),
                        cond_out_pairs: p.cond_out_pairs(crate::regfile::RegFile::M68k),
                    });
                }
            }
            Item::ExternProc(e) => {
                extern_names.insert(e.name.clone());
                extern_spans.insert(e.name.clone(), e.span);
                nodes.insert(e.name.clone(), extern_node(e));
                let flags = flags_of(&e.sig.out_flags);
                if !flags.is_empty() {
                    flag_callees.insert(e.name.clone(), flags);
                }
                let conds = conds_of(&e.sig.out_cond);
                if !conds.is_empty() {
                    cond_callees.insert(e.name.clone(), conds);
                }
            }
            Item::ContractType(t) => {
                types.insert(t.name.clone(), contract_type_bound(t));
            }
            Item::Section(s) => collect_items(
                &s.items, file, nodes, types, extern_names, proc_names, extern_spans, counter,
                flag_callees, cond_callees, proc_bufs, defines, env, dropped_by_proc,
                comptime_unresolved, contracts, preserve_inputs,
            ),
            _ => {}
        }
    }
}

/// rung-2 §13.3 sub-part 3 — walk a `(cpu: z80)` module's items (recursing
/// sections), collecting each proc/extern's `out(carry: …)` flag contract into
/// `z80_flag_callees` and each body-bearing proc's Cpu::Z80-evaluated CodeBuf
/// (with its `@discards` spans) into `z80_proc_bufs`. The whole-corpus flag
/// must-use check needs both sides (callee contract + caller body); this pass
/// supplies them WITHOUT feeding the 68k closure — keeping the register-contract
/// half of the PASS-2 Z80 skip intact.
#[allow(clippy::too_many_arguments)] // mirrors collect_items' collector-state set
fn collect_z80_flag_procs(
    items: &[Item],
    file: &ast::File,
    counter: &mut u32,
    defines: &[(String, i128)],
    env: &[Item],
    z80_flag_callees: &mut BTreeMap<String, BTreeSet<String>>,
    z80_proc_bufs: &mut Vec<ProcBuf>,
    comptime_unresolved: &mut Vec<(String, String, Span)>,
    contracts: &crate::contract::InterfaceEnv,
    z80_declared_out: &mut Z80OutMap,
    z80_falls_into: &mut BTreeMap<String, String>,
    z80_extern_names: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            Item::Proc(p) => {
                let flags = flags_of(&p.out_flags);
                if !flags.is_empty() {
                    z80_flag_callees.insert(p.name.clone(), flags);
                }
                // §G4.5: the declared register-out UNIT set (the reglist part of
                // `out(hl, carry: found)` is `{h, l}`; the carry flag rides
                // `out_flags` and is not a register unit). Recorded even when empty
                // so the fixpoint and the claim census range over exactly the procs
                // that declare a register out.
                let out_units = p.unconditional_outs(crate::regfile::RegFile::Z80);
                if !out_units.is_empty() {
                    z80_declared_out.insert(p.name.clone(), out_units);
                }
                if let Some(succ) = &p.falls_into {
                    z80_falls_into.insert(p.name.clone(), succ.clone());
                }
                let (buf, _diags, next, _dropped, unresolved) = crate::eval::eval_proc_body_env(
                    file, &p.name, &p.params, &p.body, p.span, *counter, Cpu::Z80, defines, env, contracts,
                );
                *counter = next;
                // A Z80 module's comptime conditions read the SAME define set —
                // a lost toggle must be as loud here as in a 68k module.
                comptime_unresolved
                    .extend(unresolved.into_iter().map(|(n, s)| (p.name.clone(), n, s)));
                if let Some(buf) = buf {
                    let mut discarded = Vec::new();
                    collect_discarded(&p.body, &mut discarded);
                    z80_proc_bufs.push(ProcBuf {
                        name: p.name.clone(),
                        buf,
                        discarded,
                        span: p.span,
                        // Z80 preserves are proven by the z80_preserves sibling,
                        // not the 68k callee-preserves oracle: empty inputs make
                        // the oracle round skip this buf (preserve_check empty).
                        preserve_check: Vec::new(),
                        preserve_names: std::collections::BTreeSet::new(),
                        // The §6 word facet is a 68k register concept; a Z80 buf's
                        // preserves are the z80_preserves sibling's job.
                        preserve_word_check: Vec::new(),
                        falls_into: None,
                        // The §7.1 survives walk is 68k-only; a Z80 buf never
                        // reaches it (these live in their own vector).
                        cond_out_pairs: Vec::new(),
                    });
                }
            }
            Item::ExternProc(e) => {
                let flags = flags_of(&e.sig.out_flags);
                if !flags.is_empty() {
                    z80_flag_callees.insert(e.name.clone(), flags);
                }
                // A Z80 `extern proc`'s declared out is a §3 boundary AXIOM — no
                // body to prove, so the fixpoint seeds it VERIFIED. Recorded so a
                // Z80 proc crediting an extern out (call / tail / falls_into) can
                // ground its own claim in it.
                z80_extern_names.insert(e.name.clone());
                let out_units = e.sig.unconditional_outs(crate::regfile::RegFile::Z80);
                if !out_units.is_empty() {
                    z80_declared_out.insert(e.name.clone(), out_units);
                }
            }
            Item::Section(s) => collect_z80_flag_procs(
                &s.items, file, counter, defines, env, z80_flag_callees, z80_proc_bufs,
                comptime_unresolved, contracts, z80_declared_out, z80_falls_into,
                z80_extern_names,
            ),
            _ => {}
        }
    }
}

/// Build a [`ProcNode`] from a body-bearing `proc` decl, returning the evaluated
/// CodeBuf too (for the §6 caller-side flag checks).
fn proc_node(
    p: &ProcDecl,
    file: &ast::File,
    counter: &mut u32,
    defines: &[(String, i128)],
    env: &[Item],
    contracts: &crate::contract::InterfaceEnv,
    preserve_inputs: &PreserveInputs,
) -> (ProcNode, Option<CodeBuf>, usize, Vec<(String, Span)>) {
    let (buf, _diags, next, dropped, unresolved) = crate::eval::eval_proc_body_env(
        file, &p.name, &p.params, &p.body, p.span, *counter, Cpu::M68000, defines, env, contracts,
    );
    *counter = next;

    let mut local_writes = BTreeSet::new();
    let mut direct_callees = Vec::new();
    let mut verified_preserves = BTreeSet::new();
    if let Some(buf) = &buf {
        // Local writes — `a7` filtered as stack discipline (census caveat 5).
        local_writes = proc_written_registers(buf).into_iter().filter(|r| r != "a7").collect();
        // Provably-preserved registers (declared + D2.32 movem-verified).
        verified_preserves = verified_preserves_regs(
            p,
            buf,
            preserve_inputs.noreturn,
            preserve_inputs.sr_mask_preservers,
        );
        // Direct-call edges from the resolved CodeBuf (post-comptime accurate).
        for it in &buf.items {
            if let CodeItem::Instr { mnemonic, ops, author, .. } = it {
                // Authorship exclusion: an `ItemAuthor::AssertDesugar`
                // instruction is a compiler-emitted divergent rail (the assert /
                // raise-error / raise-exception `jsr (HANDLER).l` + terminal
                // `jmp (PAGES).l`). Its target is the compiler's own contract,
                // not a return-path edge of the host proc — the same premise the
                // noreturn model consumes for the rail's terminal — so it
                // contributes NO callee edge. A HAND-written call to the same
                // symbol (authored `User`) stays a plain edge.
                if matches!(author, crate::value::ItemAuthor::AssertDesugar) {
                    continue;
                }
                if !CALL_MNEMONICS.contains(&mnemonic.as_str())
                    && !TAIL_MNEMONICS.contains(&mnemonic.as_str())
                {
                    continue;
                }
                if let Some(target) = call_target_sym(ops) {
                    direct_callees.push(target);
                }
            }
        }
    }

    let node = ProcNode {
        local_writes,
        direct_callees,
        indirect_sites: collect_indirect_sites(&p.body, p.is_resumable()),
        is_extern: false,
        // §6 partial-width: a `preserves(dN.w)` licenses clobbering the full `dN`
        // for callers (conservative v1 — the caller-visible contract is identical
        // to `clobbers(dN)`), so `dN` joins the declared clobber set. It is NOT in
        // `verified_preserves`, so the closure keeps clobbering it (no credit to
        // callers) while `[proc.clobber-undeclared]` stays silenced. The low-word
        // round-trip is the word-facet oracle gate's separate obligation.
        declared_clobbers: {
            let mut c = expand_reglist_regs(p.clobbers.as_deref().unwrap_or(&[]));
            c.extend(crate::lower::preserve_word_regs(p));
            c
        },
        // The facet itself, kept distinguishable from the plain clobber above (spec
        // §3: recorded in the contract surface for a width-aware consumer).
        word_preserves: crate::lower::preserve_word_regs(p),
        params: param_regs_typed(&p.params),
        // Fold `inout` into `out` for caller-side crediting: an in-out register is
        // written/produced by the callee exactly as an out is, so the closure /
        // D1c / §6 must see it as a result. The exit-side obligation is separate.
        out: {
            let mut o = expand_reglist_regs(p.out.as_deref().unwrap_or(&[]));
            o.extend(expand_reglist_regs(p.inout.as_deref().unwrap_or(&[])));
            o
        },
        inout: expand_reglist_regs(p.inout.as_deref().unwrap_or(&[])),
        has_clobber_contract: p.clobbers.is_some(),
        verified_preserves,
        requires: p.requires.iter().map(|(n, _)| n.clone()).collect(),
        grants: p.grants.iter().map(|(n, _)| n.clone()).collect(),
        unanalyzable_allowed: unanalyzable_reason_in_force(file, p).is_some(),
    };
    (node, buf, dropped, unresolved)
}

/// The S2-D6 U4 escape hatch: the reason string of an
/// `@allow("clobbers.unanalyzable", "<reason>")` in force for proc `p`, or `None`
/// if none is. Mirrors the module-scope `@allow` machinery ("one rule") but
/// honors the annotation in EITHER scope: a MODULE-level `@allow` (`file.attrs`,
/// where the parser routes a leading attr, exactly like `layout.odd-field`) OR a
/// PROC-level `@allow` (`p.attrs`, where a non-leading attr lands). A reason
/// string is mandatory and parse-enforced (`[clobbers.unanalyzable-reason-required]`),
/// so a malformed form still counts as "in force" for suppression, listed with
/// an empty reason. Suppresses ONLY the ⊤/unbounded firing.
fn unanalyzable_reason_in_force(file: &ast::File, p: &ProcDecl) -> Option<String> {
    attrs_unanalyzable_reason(&p.attrs).or_else(|| attrs_unanalyzable_reason(&file.attrs))
}

/// The reason from an `@allow("clobbers.unanalyzable", "<reason>")` in an attr
/// list (module- or item-level share the [`Attr`] shape), or `None`.
fn attrs_unanalyzable_reason(attrs: &[ast::Attr]) -> Option<String> {
    for a in attrs.iter().filter(|a| a.name == "allow") {
        let first_is_unanalyzable =
            a.args.first().and_then(ast::Arg::str_value) == Some("clobbers.unanalyzable");
        if first_is_unanalyzable {
            return Some(a.args.get(1).and_then(ast::Arg::str_value).unwrap_or("").to_string());
        }
    }
    None
}

/// Build a leaf [`ProcNode`] from an `extern proc` decl (§3). The leaf's
/// effective clobber set is `clobbers ∪ out`: an `out` register is WRITTEN by
/// the callee (an advanced in-out cursor like S4LZ's `a1`), so a caller relying
/// on it across the call is wrong and must be charged it — exactly as a
/// body-bearing proc's `local_writes` already includes its out-register writes.
fn extern_node(e: &ExternProcDecl) -> ProcNode {
    // The FULL `out` reglist (conditional results included, per
    // [`ast::ProcSig::unconditional_outs`]' dividing line): `effective` answers
    // "does the callee WRITE this", and a conditional result is written on its
    // cc edge. The UNCONDITIONAL subtraction happens once, downstream, where a
    // gate needs an out to be a DEFINITION on every edge.
    let mut out = expand_reglist_regs(e.sig.out.as_deref().unwrap_or(&[]));
    let inout = expand_reglist_regs(e.sig.inout.as_deref().unwrap_or(&[]));
    out.extend(inout.iter().cloned());
    let mut effective = sig_clobbers(&e.sig);
    effective.extend(out.iter().cloned());
    ProcNode {
        is_extern: true,
        declared_clobbers: effective,
        params: sig_param_regs(&e.sig),
        out,
        inout,
        has_clobber_contract: e.sig.clobbers.is_some(),
        requires: e.sig.requires.iter().map(|(n, _)| n.clone()).collect(),
        ..Default::default()
    }
}

/// The register EFFECT a contract type imposes on its dispatch targets (§4): the
/// registers a conforming target may WRITE. An explicit `clobbers(...)` is the
/// clobber bound; a preserves-only type (e.g. `ObjRoutine preserves(a0, d7)`)
/// bounds it to everything-not-preserved (the whole register file minus its
/// preserves).
///
/// The bound's `out` is UNIONED in, matching [`extern_node`]: an `out` register
/// is WRITTEN by the callee, so a caller holding a value in it across the
/// dispatch is wrong and must be charged the write. Omitting it would make the
/// effect at a `jsr (aN) as T` site narrower than the truth, and the consequence
/// is DESTRUCTIVE rather than merely permissive — `preserves::find_dead_saves`
/// reads the closure's effective set, so a save that is load-bearing exactly
/// because the dispatch target writes an `out` register would be reported as
/// `[proc.dead-save]` and deleted. Conditional outs (`out(rN if cc)`) are
/// included: the register is written on the cc edge, so it is destroyed from the
/// caller's view on every edge — the FULL `out` reglist is the conservative
/// read here, not [`ast::ProcSig::unconditional_outs`].
fn contract_type_bound(t: &ContractTypeDecl) -> RegEffect {
    let mut regs = match &t.sig.clobbers {
        Some(c) => expand_reglist_regs(c),
        None => {
            let preserved = expand_reglist_regs(&t.sig.preserves);
            universe().difference(&preserved).cloned().collect()
        }
    };
    regs.extend(expand_reglist_regs(t.sig.out.as_deref().unwrap_or(&[])));
    RegEffect { top: false, regs }
}

/// A contract signature's clobbers as a register set.
fn sig_clobbers(sig: &ProcSig) -> BTreeSet<String> {
    expand_reglist_regs(sig.clobbers.as_deref().unwrap_or(&[]))
}

/// Register names of a `proc`'s params (spellings ARE registers, §5.1).
fn param_regs_typed(params: &[(String, ast::Type, Span)]) -> BTreeSet<String> {
    params.iter().filter_map(|(name, _, _)| reg_name(name)).collect()
}

/// Register names of a contract-signature's params (`Option<Type>`).
fn sig_param_regs(sig: &ProcSig) -> BTreeSet<String> {
    sig.params.iter().filter_map(|(name, _, _)| reg_name(name)).collect()
}

/// Canonicalize a param name to a register spelling, or `None` if it is not a
/// register (defensive — proc params are register spellings today).
fn reg_name(name: &str) -> Option<String> {
    Reg::from_name(name).map(|r| r.to_string())
}

/// The GLOBAL-symbol target of a call/tail-shaped instruction, across every
/// direct-target spelling the shared extractor reads: the bare `Sym` (`jsr Foo`),
/// the offset absolute `SymOff` (`jmp Item.field`), and the pinned absolute
/// `AbsSym` (`jsr (sym).l`). The last is what `invoke Iface.hook` lowers to (an
/// engine→game call), so recognizing it charges the bound proc's clobbers into
/// the invoker's closure. `None` for a register-indirect `jsr (aN)` (no symbolic
/// target), or a LOCAL-label target: hygiene mangles local labels as
/// `$module$proc$label`, and a `bra`/`jbra` to a local label (`.loop`) is
/// intra-proc control flow, never a callee — the `$` marks it so it is dropped
/// from both the edge set and the hole/unresolved report (a real proc/extern name
/// never contains `$`). The CALLER gates the mnemonic
/// ([`CALL_MNEMONICS`]/[`TAIL_MNEMONICS`]); this reads only the target.
///
/// Delegates to [`crate::flag_check::transfer_target_sym`], the AbsSym-aware
/// extractor the noreturn model and the abs seam also read, so the closure
/// recognizes `jsr (sym).l` for one spelling of "the symbol a transfer names". A
/// recognized callee — bare `Sym`, `SymOff`, or `AbsSym` — resolves the same way:
/// a `proc` charges its contract, an `extern proc` its declared surface, and any
/// other name (INCLUDING an equ/link boundary) is an `unresolved_callees` HOLE.
/// The residue-empty gate therefore means every call edge the corpus ships is
/// contract-charged or compiler-owned.
///
/// The `assert`/`raise` desugars also lower to `jsr (MDDBG__ErrorHandler).l` /
/// `jmp (…_PagesController).l` naming contractless `pub equ … = extern(…)+off`
/// LINK BOUNDARIES. Those are excluded from the closure not by symbol kind but by
/// AUTHORSHIP: they carry `ItemAuthor::AssertDesugar`, and the collection sites
/// drop an authored divergent rail (the rail's target is the compiler's own
/// contract, not a return-path edge of the host proc). A hand-written
/// `jsr (Equ).l` (authored `User`) is a plain edge whose honest exit is an
/// `extern proc` contract for the boundary. The lowering `invoke` → `jsr (sym).l`
/// is proven in the game_contract tests, and the direct-call propagation it
/// composes with is proven by `direct_jsr_and_bsr_call_edges`.
fn call_target_sym(ops: &[CodeOperand]) -> Option<String> {
    crate::flag_check::transfer_target_sym(ops)
        .filter(|s| !s.contains('$'))
        .map(str::to_string)
}

/// Scan a proc body (recursing comptime-`if` branches) for indirect call sites,
/// returning each site's declared bound: `Some(type)` for `jsr (aN) as Type`,
/// `None` for an unbounded `jsr (aN)`. A call whose target is a bare symbol
/// (direct) contributes no indirect site.
///
/// `is_resumable` reconciles the closure gate with the `@resumable` stackless
/// contract (bookmark ask 1): a resumable proc exits by a terminal computed
/// `jmp (aN)` (aN != a7) to a caller-loaded continuation — a BOUNDED, return-like
/// terminator, not a dispatch into unknown code. Its register budget is already
/// pinned (a resumable proc MUST declare `clobbers`, and the stackless scan bounds
/// the body), so such an exit contributes NO unbounded ⊤ site. See
/// [`is_resumable_continuation_exit`] for the exact (and deliberately narrow) shape
/// this credits.
fn collect_indirect_sites(body: &[AsmStmt], is_resumable: bool) -> Vec<Option<String>> {
    let mut sites = Vec::new();
    walk_body_for_indirect(body, is_resumable, &mut sites);
    sites
}

fn walk_body_for_indirect(body: &[AsmStmt], is_resumable: bool, sites: &mut Vec<Option<String>>) {
    for stmt in body {
        match stmt {
            AsmStmt::Instr(instr) => {
                if is_indirect_call(instr) {
                    // A `@resumable` proc's untyped `jmp (aN)` continuation exit is
                    // credited as a bounded terminator, not an unbounded site — so
                    // it does not force the proc's effect to ⊤.
                    if is_resumable && is_resumable_continuation_exit(instr) {
                        continue;
                    }
                    sites.push(instr.dispatch_bound.clone());
                }
            }
            AsmStmt::If { then, els, .. } => {
                walk_body_for_indirect(then, is_resumable, sites);
                if let Some(e) = els {
                    walk_body_for_indirect(e, is_resumable, sites);
                }
            }
            AsmStmt::With { body, .. } => walk_body_for_indirect(body, is_resumable, sites),
            _ => {}
        }
    }
}

/// A `@resumable` proc's terminal continuation exit that the closure credits as
/// bounded: an UNTYPED indirect TAIL transfer (`jmp (aN)`) through an address
/// register that is NOT a7. It leaves the proc to a caller-loaded continuation, so
/// it is return-like, not a call into unknown code that would clobber the caller's
/// registers.
///
/// Deliberately narrow, on three axes, each keeping the credit sound:
/// - **TAIL only** (`jmp`/`bra`/`jbra`, never a CALL): an indirect CALL in a
///   resumable proc (`jsr (aN)`) is already build-fatal via `[resumable.stack-op]`
///   (it pushes), so it is not a shape this must — or does — credit.
/// - **aN != a7**: a `jmp (sp)`/`jmp (a7)` READS the stack (also
///   `[resumable.stack-op]`); excluding it keeps the credit fail-safe (an
///   unrecognizable / spliced operand register is NOT credited either).
/// - **UNTYPED only** (`dispatch_bound.is_none()`): a `jmp (aN) as Type` (bookmark
///   ask 5) is bounded by the dispatch TYPE's effect through the existing indirect-
///   site machinery, which is STRICTER (it charges the type's clobbers). Excluding
///   the typed form here lets that bound supersede this credit cleanly the day ask
///   5 lands — no double-handling, no weakening.
fn is_resumable_continuation_exit(instr: &InstrLine) -> bool {
    let Some(m) = single_text(&instr.mnemonic) else { return false };
    if !TAIL_MNEMONICS.contains(&m) || instr.dispatch_bound.is_some() {
        return false;
    }
    matches!(
        instr.operands.first().and_then(ind_single_register),
        Some(r) if r != Reg::A7
    )
}

/// The single base register of a bare register-indirect operand (`(aN)`), or
/// `None` for any richer form (displaced, indexed, sized, multi-part, or a
/// non-register expression). `Reg::from_name` maps the `sp` spelling to `a7`, so a
/// `jmp (sp)` resolves to [`Reg::A7`] and is caught by the caller's exclusion.
fn ind_single_register(op: &Operand) -> Option<Reg> {
    let Operand::Ind { parts, size, .. } = op else { return None };
    if size.is_some() || parts.len() != 1 {
        return None;
    }
    let (expr, part_size) = &parts[0];
    if part_size.is_some() {
        return None;
    }
    let ast::Expr::Path(p) = expr else { return None };
    match p.segments.as_slice() {
        [seg] => Reg::from_name(seg),
        _ => None,
    }
}

/// True when an AST instruction is an indirect call/tail-transfer — a call-shaped
/// mnemonic whose first operand is a register-indirect EA (`jsr (a1)` /
/// `jsr (a0, d4.w)`), as opposed to a direct `jsr Foo`.
fn is_indirect_call(instr: &InstrLine) -> bool {
    let Some(m) = single_text(&instr.mnemonic) else { return false };
    if !CALL_MNEMONICS.contains(&m) && !TAIL_MNEMONICS.contains(&m) {
        return false;
    }
    matches!(instr.operands.first(), Some(Operand::Ind { .. }))
}

/// The mnemonic as a single literal string, or `None` if it is spliced.
fn single_text(mnemonic: &[TextOrSplice]) -> Option<&str> {
    match mnemonic {
        [TextOrSplice::Text(s)] => Some(s.as_str()),
        _ => None,
    }
}
