//! eval: the driver — line loop, directive dispatch, instruction lowering, emit.

use crate::expand::{
    render_tokens, split_call_args, split_top_commas, substitute_frame, substitute_name,
};
use crate::lexer::{lex_line, lex_line_recover};
use crate::operands::{parse_operands, OperandAtom};
use crate::parser::parse_line_tokens;
use crate::token::{Punct, Tok, Token};
use crate::{cpu_for_spelling, unsupported_cpu, Failure, Options};
use sigil_backend_m68k::m68k::{
    Cond as M68kCond, Instruction as M68kInstruction, Mnemonic as M68kMnemonic,
    Operand as M68kOperand, Size as M68kSize, Xn as M68kXn,
};
use sigil_backend_m68k::M68kBackend;
use sigil_backend_z80::z80::{Cond, Mnemonic, Operand, Reg16, Reg8};
use sigil_backend_z80::Z80Backend;
use sigil_ir::backend::{Backend, Cpu, IrStreamer, LowerError};
use sigil_ir::expr::{BinOp, Fold};
use sigil_ir::{
    asl_width_rule, AbsWidth, DataFragment, EquSym, Expr, Fixup, FixupKind, IrBuilder, Module,
    SymbolTable, SymbolValue,
};
use sigil_span::{Diagnostic, Level, SourceId, Span};

const EXPAND_CAP: usize = 64;
const PASS_CAP: usize = 16;
/// Bound for `while … endm` (T9.2): caps re-evaluation/body-expansion
/// iterations so a non-convergent condition diagnoses (A5) instead of
/// hanging. Generous relative to any real `while`-driven table-fill idiom.
const WHILE_CAP: usize = 10_000;

#[derive(Clone)]
struct SrcLine {
    text: String,
    base: u32,
    /// The spliced file this line was read from. Carried per line rather than
    /// per assembler so an `include`d file's line — and a macro body line, which
    /// executes at a call site in a different file — reports the file that
    /// actually contains the text.
    source: SourceId,
}

/// Collected macro definitions: name → (params, body lines).
type MacroTable = std::collections::BTreeMap<String, MacroDef>;

/// One captured macro definition.
#[derive(Clone)]
struct MacroDef {
    /// Declared parameter names, in order. A `{INTLABEL}` group is NOT one of
    /// them: it declares a capture, not a slot, and consumes no argument
    /// position wherever it is written (asl-verified — `m macro {INTLABEL},pp,qq`,
    /// `n macro pp,{INTLABEL},qq` and `o macro pp,qq,{INTLABEL}` all bind
    /// `pp`/`qq` from `11,22`). The lexer already swallows the whole `{…}` group
    /// without emitting a token, so the list below is right by construction.
    params: Vec<String>,
    body: Vec<SrcLine>,
    /// Whether the parameter list carries a `{INTLABEL}` group, which makes the
    /// invocation line's label the macro's to place rather than the assembler's.
    int_label: bool,
}
/// Collected function definitions: name → (params, body tokens).
type FunctionTable = std::collections::BTreeMap<String, (Vec<String>, Vec<Token>)>;

pub fn run(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    run_impl(src, "", opts, false).map_err(|f| f.diags)
}

/// Like [`run`] but keeps the [`SourceMap`](sigil_span::SourceMap) the diagnostics'
/// spans resolve against, so a caller can render each one as `file(line)`.
/// `root_name` is the file `src` was read from; every `include`d file names itself.
pub fn run_located(src: &str, root_name: &str, opts: &Options) -> Result<Module, Failure> {
    run_impl(src, root_name, opts, false)
}

/// Like [`run`] but FORCES the final label-relocation deferral pass even when the module
/// converges poison-free. For a CHAINED build (the harness's frozen-table
/// placement moves sections after assembly), every `dc.l`/`dc.w`/… that references a
/// section LABEL must stay symbolic so the linker relocates it against the label's placed
/// base — otherwise a poison-free residual (config_a) bakes a stale this-pass VMA (the
/// row-94 parallax `P_DBG := DeformTable_Zero` pointer). A PINNED build never needs this
/// (sections don't move → bake == relocate), so ordinary `run` stays byte-for-byte asl.
pub fn run_relocating(src: &str, root_name: &str, opts: &Options) -> Result<Module, Failure> {
    run_impl(src, root_name, opts, true)
}

fn run_impl(
    src: &str,
    root_name: &str,
    opts: &Options,
    force_relocate: bool,
) -> Result<Module, Failure> {
    // Seed pass 0 with the provided defines; each later pass is seeded with the
    // previous pass's discovered symbols so forward references resolve. Macro and
    // function definitions are carried forward too, so an `ifndef`-guarded
    // definition collected on pass 0 stays available on later passes when its
    // guard symbol suppresses re-collection.
    let mut seed = SymbolTable::new();
    for (k, v) in &opts.defines {
        seed.define(k, SymbolValue::Int(*v));
    }
    // Guarded defines seed the env exactly like ordinary defines (so the
    // residual AS reads the `.emp`-owned constant at comptime); the difference
    // is enforcement, not seeding — `directive_equate` rejects an in-file
    // redefinition of a guarded name (the collision guard, Stage-3 P5).
    for (k, v) in &opts.guarded_defines {
        seed.define(k, SymbolValue::Int(*v));
    }
    let mut macros = MacroTable::new();
    let mut functions = FunctionTable::new();
    // Label names known from the previous pass — seeds each pass so a FORWARD
    // branch/jump target that names a label is recognized as a label (kept
    // symbolic in `fixup_target`) before its definition line executes.
    let mut labels: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Label-referencing equ names known from the previous pass — same forward-
    // reference role as `labels`, for the debugger's `DEBUGGER__* = <label>` table.
    let mut label_ref_equs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut prev = seed.clone();
    // Every equ-sym name any pass exports, in first-seen order. An
    // `ifndef`-guarded definition block executes (and exports) on pass 0,
    // then SKIPS on later passes once its guard symbol is seeded — the
    // definitions persist via env seeding, but the export side effect
    // vanished with the skipped block (tranche-3 review finding). The
    // converged module gets any missing exports re-attached from the
    // CONVERGED env, whose values are authoritative.
    let mut ever_exported: Vec<(String, Span)> = Vec::new();
    let mut ever_exported_names: std::collections::HashSet<String> = Default::default();
    // The most recent pass's spliced-file map. Diagnostics are returned from a
    // single pass, so the map returned with them is that pass's own.
    let mut last_sources = sigil_span::SourceMap::new();
    for pass in 0..PASS_CAP {
        let PassOutput {
            module,
            env,
            macros: m,
            functions: f,
            diags,
            poison,
            labels: pass_labels,
            label_ref_equs: pass_label_ref_equs,
            sources,
        } = one_pass(src, root_name, opts, &seed, &macros, &functions, &labels, &label_ref_equs);
        last_sources = sources;
        for sec in &module.sections {
            for eq in &sec.equ_syms {
                if ever_exported_names.insert(eq.name.clone()) {
                    ever_exported.push((eq.name.clone(), eq.span));
                }
            }
        }
        if pass > 0 && env == prev {
            // Converged: this pass's env is authoritative. A final bonus pass (seeded
            // from it, `defer_unresolved_jsr_jmp` set) does two things the ordinary
            // passes cannot: (1) a `jsr`/`jmp` bare-symbol target still folding to Poison
            // is a genuine cross-seam reference (a sibling `.emp` `pub proc`, joined at
            // LINK time) → deferred `Fragment::JmpJsrSym` instead of an error; (2) a
            // `dc.l`/`dc.w`/`jsr`/… referencing a section LABEL stays SYMBOLIC (an `Abs*`
            // fixup) so the linker relocates it against the label's placed VMA.
            //
            // It runs when leftover poison needs (1), OR when `force_relocate` needs (2)
            // for a CHAINED build (sections move → a baked label VMA goes stale; the
            // row-94 parallax pointer). A poison-free PINNED build skips it: sections
            // don't move, so a baked label == its relocated value — return the ordinary
            // module, byte-for-byte asl (the overwhelmingly common path).
            if poison.is_empty() && !force_relocate {
                let mut module = module;
                restore_missing_equ_exports(&mut module, &ever_exported, &env);
                attach_guarded_equ_exports(&mut module, &opts.guarded_defines);
                return if diags.iter().any(|d| d.level == Level::Error) {
                    Err(Failure { diags, sources: last_sources })
                } else {
                    Ok(module)
                };
            }
            let bonus = one_pass_with_defer(
                src,
                root_name,
                opts,
                &seed,
                &macros,
                &functions,
                &pass_labels,
                &pass_label_ref_equs,
                true,
            );
            let mut diags = bonus.diags;
            for (name, span) in bonus.poison {
                diags.push(Diagnostic {
                    level: Level::Error,
                    message: format!("unresolved symbol `{name}` in operand"),
                    primary: span,
                });
            }
            let mut bonus_module = bonus.module;
            restore_missing_equ_exports(&mut bonus_module, &ever_exported, &env);
            attach_guarded_equ_exports(&mut bonus_module, &opts.guarded_defines);
            return if diags.iter().any(|d| d.level == Level::Error) {
                Err(Failure { diags, sources: bonus.sources })
            } else {
                Ok(bonus_module)
            };
        }
        prev = env.clone();
        seed = env;
        macros = m;
        functions = f;
        labels = pass_labels;
        label_ref_equs = pass_label_ref_equs;
    }
    // Non-convergence is a property of the whole run, not of any one line, so it
    // carries no source: an id past every registered file makes the renderer print
    // a bare message rather than attribute the failure to line 1 of the root.
    Err(Failure {
        diags: vec![Diagnostic {
            level: Level::Error,
            message: format!(
                "assembly did not converge within {PASS_CAP} passes (symbol values still changing)"
            ),
            primary: Span {
                source: SourceId(u32::MAX),
                start: 0,
                end: 0,
            },
        }],
        sources: last_sources,
    })
}

/// The outputs of a single assembly pass.
struct PassOutput {
    module: Module,
    /// The files this pass spliced, under the ids its spans carry. Every pass
    /// rebuilds it (a fresh `Asm` per pass), and the pass that produced the
    /// returned diagnostics is the one whose map resolves them.
    sources: sigil_span::SourceMap,
    env: SymbolTable,
    macros: MacroTable,
    functions: FunctionTable,
    diags: Vec<Diagnostic>,
    /// Operand symbols that folded to Poison this pass (name + site span).
    poison: Vec<(String, Span)>,
    /// Every fully-qualified label name defined this pass (grown from the seed).
    /// Threaded into the next pass so a forward-referenced label is known before
    /// its definition line — see [`Asm::known_labels`].
    labels: std::collections::HashSet<String>,
    /// Every label-referencing `equ`/`=` name defined this pass — see
    /// [`Asm::label_ref_equs`]. Threaded into the next pass.
    label_ref_equs: std::collections::HashSet<String>,
}

/// True if `e` names any symbol (a `Sym` node anywhere in the tree). Used by the
/// deferred cross-seam equate path: a RHS `eval_all` couldn't fold necessarily
/// contains an unresolved symbol, and a full parse of it is worth exporting as a
/// symbolic `equ_sym` only when it actually references one (a pure-`Int` parse
/// would have folded).
fn expr_has_sym(e: &Expr) -> bool {
    match e {
        Expr::Sym(_) => true,
        Expr::Binary { lhs, rhs, .. } => expr_has_sym(lhs) || expr_has_sym(rhs),
        Expr::Unary { operand, .. } => expr_has_sym(operand),
        Expr::Int(_) => false,
    }
}

/// Re-attach equ exports the CONVERGED pass lost to a guard-skipped block
/// (tranche-3 review finding — see `run`'s `ever_exported` comment): any name
/// some earlier pass exported that the final module lacks gets an `EquSym`
/// with its value from the converged `env` (authoritative — a forward-ref-
/// dependent equ gets its FINAL value, not the earlier pass's). Attached to
/// the first section (the link flattens equ_syms; placement is arbitrary).
/// String equs never export (they resolve to `None` here), matching
/// `directive_equate`. A module with no sections has nothing to attach to —
/// and nothing to link either.
fn restore_missing_equ_exports(
    module: &mut Module,
    ever_exported: &[(String, Span)],
    env: &SymbolTable,
) {
    if ever_exported.is_empty() || module.sections.is_empty() {
        return;
    }
    let present: std::collections::HashSet<&str> = module
        .sections
        .iter()
        .flat_map(|s| s.equ_syms.iter())
        .map(|e| e.name.as_str())
        .collect();
    let missing: Vec<EquSym> = ever_exported
        .iter()
        .filter(|(n, _)| !present.contains(n.as_str()))
        .filter_map(|(n, sp)| {
            env.resolve(n, None).map(|v| EquSym { name: n.clone(), expr: Expr::Int(v), span: *sp })
        })
        .collect();
    module.sections[0].equ_syms.extend(missing);
}

/// Export each guarded `-D` define (the `.emp`-owned constants injected by the
/// P5 ownership flip) as a link-level `EquSym`, so an `.emp` module that
/// references the constant as a BARE LINK SYMBOL — an absolute-address operand
/// like `boot.emp`'s `move.b #$40, HW_PORT_1_DATA`, NOT a `use engine.constants`
/// import — resolves through the joint link exactly as it did when
/// `engine/constants.asm`'s `=` equate exported the same symbol. Byte-neutral:
/// an `EquSym` is zero bytes and (like every AS `=` equate) is FILTERED from the
/// deb2 symbol appendix, so re-homing the export from the deleted `.asm` `=` to
/// this harvested-define carrier changes no ROM byte. A name already present as
/// an equ_sym or a label is skipped (the definer wins; no duplicate-symbol).
fn attach_guarded_equ_exports(module: &mut Module, guarded: &[(String, i64)]) {
    if guarded.is_empty() || module.sections.is_empty() {
        return;
    }
    let present: std::collections::HashSet<&str> = module
        .sections
        .iter()
        .flat_map(|s| s.equ_syms.iter().map(|e| e.name.as_str()).chain(s.labels.iter().map(|l| l.name.as_str())))
        .collect();
    let sp = Span { source: SourceId(0), start: 0, end: 0 };
    let add: Vec<EquSym> = guarded
        .iter()
        .filter(|(n, _)| !present.contains(n.as_str()))
        .map(|(n, v)| EquSym { name: n.clone(), expr: Expr::Int(*v), span: sp })
        .collect();
    module.sections[0].equ_syms.extend(add);
}

/// One assembly pass seeded with `seed_env` (symbols) plus the macro/function
/// definition tables from prior passes. Returns the module, the discovered
/// symbol table, the (possibly extended) definition tables, diagnostics, and the
/// unresolved-operand references seen this pass.
// The parameters are the pass-to-pass seed tables — one per carried table, by design.
#[allow(clippy::too_many_arguments)]
fn one_pass(
    src: &str,
    root_name: &str,
    opts: &Options,
    seed_env: &SymbolTable,
    seed_macros: &MacroTable,
    seed_functions: &FunctionTable,
    seed_labels: &std::collections::HashSet<String>,
    seed_label_ref_equs: &std::collections::HashSet<String>,
) -> PassOutput {
    one_pass_with_defer(
        src, root_name, opts, seed_env, seed_macros, seed_functions, seed_labels,
        seed_label_ref_equs, false,
    )
}

/// Like [`one_pass`], but also threads [`Asm::defer_unresolved_jsr_jmp`] —
/// `run`'s bonus final pass sets this `true` so a still-Poison `jsr`/`jmp`
/// bare-symbol target defers instead of joining `poison_refs`.
// The parameters are the pass-to-pass seed tables — one per carried table, by design.
#[allow(clippy::too_many_arguments)]
fn one_pass_with_defer(
    src: &str,
    root_name: &str,
    opts: &Options,
    seed_env: &SymbolTable,
    seed_macros: &MacroTable,
    seed_functions: &FunctionTable,
    seed_labels: &std::collections::HashSet<String>,
    seed_label_ref_equs: &std::collections::HashSet<String>,
    defer_unresolved_jsr_jmp: bool,
) -> PassOutput {
    let mut asm = Asm::new_with_defer(opts, defer_unresolved_jsr_jmp);
    asm.env = seed_env.clone();
    asm.macros = seed_macros.clone();
    asm.functions = seed_functions.clone();
    asm.known_labels = seed_labels.clone();
    asm.label_ref_equs = seed_label_ref_equs.clone();
    asm.process(root_name, src);
    // The end-of-unit half of the no-declared-processor refusal. `emit` catches
    // a unit that PRODUCES bytes without a declaration, which is where the
    // silent-wrong-target damage is; this catches the rest — a unit that
    // declares nothing and emits nothing still had every `$` in it lexed
    // against a processor nobody named, and its `equ` values carry that
    // decision out to whoever consumes them.
    //
    // Scoped to the UNIT: `asm.state` is one state threaded through the root and
    // every file it `include`s, so a `cpu` line anywhere in the unit satisfies
    // this, and an included file of its own carries no obligation.
    if !asm.state.cpu_declared {
        let span = Span { source: sigil_span::SourceId(0), start: 0, end: 0 };
        asm.refuse_undeclared_cpu(span);
    }
    // Task B1 (seam re-eval): a source consisting ONLY of `equ`s (no section
    // ever opens) would otherwise strand `pending_equ_syms` — force a carrier
    // section open so `finish()` never silently drops them.
    if !asm.pending_equ_syms.is_empty() {
        asm.open_section_if_needed();
    }
    let (mut module, mut diags) = asm.builder.finish();
    dedup_section_names(&mut module.sections);
    diags.append(&mut asm.diags);
    // Report the refusal FIRST. Everything else an undeclared unit produces is a
    // consequence of it: under the provisional processor a `$` lexes as the
    // program counter rather than a hex prefix, so a 68000 source mis-parses from
    // line 1 and the real cause arrives after a screen of its own symptoms. It is
    // raised where it is DETECTED (at the first emitted byte, or at the end of the
    // unit) and read where it EXPLAINS.
    if let Some(pos) = diags.iter().position(|d| d.message == crate::CPU_UNDECLARED) {
        let d = diags.remove(pos);
        diags.insert(0, d);
    }
    PassOutput {
        module,
        env: asm.env,
        macros: asm.macros,
        functions: asm.functions,
        diags,
        poison: asm.poison_refs,
        labels: asm.known_labels,
        label_ref_equs: asm.label_ref_equs,
        sources: asm.sources,
    }
}

/// Give every NON-EMPTY auto-opened section a unique name. Two sections that
/// open at the same VMA base (e.g. a second bank re-phased at the same address)
/// both get the bare `sec{vma}` name from `open_section_if_needed`; here the
/// first keeps it and later ones get a `#1`/`#2`/… suffix, so `link()`'s
/// duplicate-symbol diagnostic (M1.D T3) doesn't misfire on a genuine
/// second bank.
///
/// EMPTY (zero-fragment) sections are skipped FOR NAMING ONLY: they place no
/// bytes, so `flatten`/`flatten_checked` exclude them from byte emission, and
/// so they must NOT consume the bare name — otherwise a stray empty `sec0`
/// would steal `sec0` from the real region-A driver, which is then
/// linked/looked-up by that name. The section itself is NOT dropped:
/// `IrBuilder::finish` keeps it, and it — with any `equ_syms` it carries —
/// survives into the link, which is load-bearing for the Task B1 equ export
/// (a pending-equ carrier section is exactly such an empty section).
fn dedup_section_names(sections: &mut [sigil_ir::Section]) {
    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for sec in sections.iter_mut() {
        if sec.fragments.is_empty() {
            continue;
        }
        match counts.get_mut(&sec.name) {
            Some(n) => {
                *n += 1;
                sec.name = format!("{}#{}", sec.name, *n);
            }
            None => {
                counts.insert(sec.name.clone(), 0);
            }
        }
    }
}

struct Asm {
    builder: IrBuilder,
    z80: Z80Backend,
    m68k: M68kBackend,
    state: crate::state::AsmState,
    env: SymbolTable,
    /// Front-end-only string-valued symbols (`.__str set "BUS ERROR"`).
    /// §7.4: strings NEVER enter `sigil_ir::SymbolValue`; they live here in the
    /// evaluator. Keyed by fully-qualified name exactly like `env` (see
    /// `resolve_str`). NOT carried across passes — asl `set` is a sequential
    /// per-pass assignment and every string symbol in the `__FSTRING` scan is
    /// assigned before it is read (probe p1/p4).
    str_env: std::collections::HashMap<String, String>,
    /// Front-end-only FLOAT-valued symbols (`sample_rate_scale := 1.0`, which
    /// is how `s2.sounddriver.asm`'s `dac_sample_metadata` macro carries an
    /// optional per-sample rate scale into `int(label.sample_rate*scale)`).
    /// §7.4: a float NEVER enters `sigil_ir::SymbolValue` — like a string, it
    /// lives here in the evaluator and is collapsed to an integer by
    /// `int(...)` before any IR node sees it. Keyed by fully-qualified name
    /// exactly like `env` and `str_env` (see [`Self::resolve_float_sym`]).
    ///
    /// A symbol is int XOR float within a pass, and unlike the int/string pair
    /// this one is ENFORCED in both directions: an assignment writes one map
    /// and removes the name from the other. It has to be — `sample_rate_scale`
    /// is reassigned on every `dac_sample_metadata` expansion, so a stale
    /// entry of the other type would outlive its own assignment.
    float_env: std::collections::HashMap<String, f64>,
    scope: Option<String>,
    /// The scope in force where the OUTERMOST macro expansion currently on the
    /// stack was invoked — the nearest scope that is not itself an expansion.
    /// `None` outside any expansion, where [`Self::scope`] is already that scope.
    ///
    /// A `.`-local bound by `equ`/`=`/`set`/`:=` inside an expansion lands HERE,
    /// and expansion nesting is transparent to it: an inner macro's `.v := 5`
    /// under `Base:` lists as `Base.v : 5`, not as anything belonging to the
    /// outer expansion, and `dc.b .v` reads `05` both inside the outer body and
    /// after the whole nest returns.
    outer_scope: Option<String>,
    /// [`scan_dot_labels`] per macro name. The body is fixed once captured, so
    /// the set is too; `capture_macro` drops the entry when a name is redefined.
    dot_label_cache: std::collections::BTreeMap<String, std::rc::Rc<std::collections::BTreeSet<String>>>,
    in_section: bool,
    /// Continuous physical location counter (asl-faithful): the real ROM byte
    /// offset of the CURRENTLY-OPEN section's start. The live physical position is
    /// `phys_base + builder.current_offset()`; it advances with every emitted byte
    /// across ALL section switches (cpu/phase/dephase) and is NEVER rewound by
    /// `restore`. `org N` sets it directly; `phase`/`dephase` leave it untouched
    /// and instead adjust `state.disp`. VMA (`$`/labels) = physical + `disp`.
    phys_base: u32,
    diags: Vec<Diagnostic>,
    /// The file currently being executed. Spans lexed from a [`SrcLine`] take the
    /// line's own [`SrcLine::source`]; this is the fallback for the few sites that
    /// lex a synthesized string with no line behind it (`\{…}` interpolation,
    /// `val()`), which belong to whichever file is executing.
    source: SourceId,
    /// Every spliced file's text under the id its spans carry: the root source at
    /// [`SourceId(0)`] and one id per `include`d file, in inclusion order. This is
    /// what turns a span into `file(line)`; without it a diagnostic's offset has
    /// nothing to resolve against.
    sources: sigil_span::SourceMap,
    functions: FunctionTable,
    macros: MacroTable,
    /// The label written on the line of a `{INTLABEL}` macro invocation, parked
    /// between [`Asm::exec_one`] recognising it and [`Asm::expand_macro_inner`]
    /// binding it to `__LABEL__`. Set only on the step immediately before the
    /// expansion it belongs to, and taken there.
    pending_int_label: Option<String>,
    macro_depth: usize,
    /// One entry per macro expansion currently on the stack, innermost last.
    /// Parameter / `ALLARGS` / `.ATTRIBUTE` substitution reads the INNERMOST
    /// frame at the moment a body line's text is consumed, which is what makes
    /// `shift` observable: the directive mutates the frame, and every body line
    /// reached afterwards is substituted against the mutated binding.
    ///
    /// Only the innermost frame is consulted. An outer expansion's parameters
    /// are already baked into anything an inner expansion can see: a nested
    /// `macro` definition captures text the outer frame has substituted (see
    /// [`Self::capture_macro`]), and a nested invocation's argument tokens are
    /// substituted on the invoking line before dispatch.
    macro_frames: Vec<MacroFrame>,
    /// Monotonic per-pass counter identifying each macro EXPANSION. A `.`-local
    /// label defined inside a macro body is scoped to its expansion (asl-verified
    /// — `docs/superpowers/notes/2026-07-04-m1d-t4-macro-local-scope-probes.md`),
    /// NOT to the caller's global label; this counter names that per-expansion
    /// scope so two expansions in one global scope don't collide. Resets to 0
    /// each pass (fresh `Asm`), so expansion order — hence scope names — is stable
    /// across passes once conditionals converge.
    macro_expansion_seq: u32,
    visited: std::collections::BTreeSet<std::path::PathBuf>,
    include_root: Option<std::path::PathBuf>,
    aborted: bool,
    /// Whether this pass already raised [`crate::CPU_UNDECLARED`]. One refusal
    /// per pass: the condition is a property of the whole unit, so repeating it
    /// per emit site would say the same thing hundreds of times. Its own flag
    /// rather than a reuse of `aborted`, which `fatal` also sets.
    cpu_refused: bool,
    /// Operand symbols that folded to Poison this pass (name + site span). On an
    /// intermediate pass these are just not-yet-resolved forward refs; on the
    /// CONVERGED pass the env is final, so any entry here is genuinely undefined
    /// and `run` promotes it to an error.
    poison_refs: Vec<(String, Span)>,
    /// Remaining `while`-body-execution budget for THIS pass (per-`Asm`, so it
    /// resets each pass). Complements the per-loop `WHILE_CAP`: two NESTED
    /// non-convergent `while`s each bounded at `WHILE_CAP` still multiply to
    /// `WHILE_CAP²` body runs, which can hang the pass. This global budget bounds
    /// the TOTAL across all (possibly nested) loops so a pathological input
    /// diagnoses in bounded time. Generous vs. any real table-fill loop.
    while_budget: usize,
    /// Task B1 (seam re-eval): int `equ`s seen while NO section is open yet,
    /// held here rather than forcing one open (see `directive_equate`'s doc —
    /// eagerly opening a section there perturbs `directive_org`'s no-section
    /// fast path on real Aeon source). Flushed onto the builder's newly
    /// opened section by `open_section_if_needed` the next time one actually
    /// opens; drained (never left stranded) as long as the module contains at
    /// least one section, which every real program does.
    pending_equ_syms: Vec<EquSym>,
    /// Port #2 (math.emp follow-up): set only on the ONE extra bonus pass
    /// `run` performs when normal convergence still leaves `jsr`/`jmp`
    /// bare-symbol targets Poison. `false` on every ordinary pass (every
    /// pre-existing compile is byte-identical — this field is additive-only).
    /// When `true`, a `jsr`/`jmp` bare-symbol target that folds to `Poison`
    /// emits a length-variable `Fragment::JmpJsrSym` (deferred to the
    /// linker's relaxation fixpoint, mirroring the `.emp` front-end's
    /// `jbra`/`jbsr`) instead of `poison_refs`-then-error: a genuinely
    /// cross-compilation-unit target (a sibling `.emp` module's `pub proc`,
    /// joined only at LINK time) is not an error here — it becomes one only
    /// if `resolve_layout`/`link` can't find it either.
    defer_unresolved_jsr_jmp: bool,
    /// Every fully-qualified LABEL name known to this pass — seeded from the
    /// previous pass (so a FORWARD-referenced label is already present) and
    /// grown as `define_label` sees each definition. `fixup_target` consults
    /// this to keep a branch/jump target that names a label SYMBOLIC rather than
    /// baking its this-pass VMA into an `Expr::Int`: the linker resolves a label
    /// against its own (relaxation-shifted) section-label table, so a symbolic
    /// target stays correct when `resolve_layout` GROWS a length-variable
    /// `JmpJsrSym` between the branch and its target (a cross-seam mixed build).
    /// Baking is kept only for env-only `equ`/`set` targets the linker cannot
    /// see. Labels vs. `equ`s are indistinguishable in `env` (both hold an
    /// `Int`), so this dedicated name set is the discriminator.
    known_labels: std::collections::HashSet<String>,
    /// Every `equ`/`=` name whose VALUE derives from a section LABEL
    /// (`HandlerPtr = Handler`, `X = Label+4`, or a chain `X = Y` onto another
    /// such equ) — the debugger's `DEBUGGER__*` handler-address table is the
    /// canonical shape. Such an equ's folded `Int` value shifts when a
    /// width-grown `JmpJsrSym` moves its underlying label, so on the deferral
    /// pass it is exported to the linker as a SYMBOLIC `equ_sym` (the linker
    /// folds it post-relax) and a `dc.l`/`jsr`/... through it is treated like a
    /// label reference (kept symbolic). A pure-constant equ never enters this
    /// set and keeps baking. Threaded across passes so a forward reference
    /// through such an equ is recognized before its definition line.
    label_ref_equs: std::collections::HashSet<String>,
    /// Every REASSIGNABLE set-symbol (`set`/`:=`) whose CURRENT value derives
    /// from a section LABEL (`P_DBG := DeformTable`, or a chain `P_DFG :=
    /// PC_FG_T` onto another such set), mapped to the `relax_safe_fold`ed
    /// SYMBOLIC snapshot it holds AT THIS EMISSION POINT. `engine/parallax_macros.inc`
    /// is the canonical shape: `P_DBG := deformBg` then `dc.l P_DFG, P_DBG` inside
    /// a config record the chainer relocates — a baked absolute would go stale.
    /// Unlike `label_ref_equs`, a set is REDEFINED, so this holds the value at the
    /// current point in emission order (each `:=` overwrites or clears the entry),
    /// giving per-use-site snapshot semantics without an SSA rename: emission is
    /// sequential, so consulting the entry at each use captures exactly what the
    /// set held then. NOT threaded across passes (set is imperative/sequential —
    /// no forward references); rebuilt every pass. Consulted only on the deferral
    /// pass (via `expr_refs_label`/`relax_safe_fold`, both guarded by
    /// `keep_labels_symbolic` at every emit site), so ordinary-pass bytes are
    /// unchanged. A set reassigned to a label-free value clears its entry and
    /// reverts to baking.
    set_sym_symbolic: std::collections::HashMap<String, Expr>,
    /// Names seeded from [`Options::guarded_defines`] — the `.emp`-owned
    /// constants the residual AS may consume but not re-author. An in-file
    /// `=`/`equ` of any of these is a `[defines.collision]` error (the P5
    /// no-silent-shadowing guard). Constant across passes (seeded from opts).
    guarded_defines: std::collections::HashSet<String>,
    /// Every `NAME struct … endstruct` layout declared this pass, by name.
    /// Read by [`Asm::instantiate_struct`] when a later line names one in the
    /// mnemonic column (`v_snddriver_ram: SMPS_RAM`) and by
    /// [`Asm::capture_struct`] when a struct body embeds another struct.
    structs: std::collections::HashMap<String, StructDef>,
    /// The label written on the line whose mnemonic column names a struct,
    /// parked between [`Asm::exec_one`] defining it and
    /// [`Asm::instantiate_struct`] hanging the members off it. Taken there;
    /// `None` at the instantiation is asl's `#2040 structure name missing`.
    pending_struct_label: Option<String>,
}

/// One `NAME struct … endstruct` layout.
///
/// **Two offset tables, because asl keeps two and they DISAGREE.** A member
/// declared `b: ds.w 1` at an odd running offset under `padding on` binds the
/// declaration-scope symbol `S.b` to the offset BEFORE the alignment pad, while
/// the element the struct table records — and every instantiation reads — is the
/// offset AFTER it. Probe `q7.asm`, `a ds.b 1 / b ds.w 1 / c ds.b 1 / d ds.l 1`:
///
/// ```text
///   symbols        S.a=0  S.b=1  S.c=4  S.d=5   S.len=$A
///   struct table     0      2      4      6
///   inst.X - inst    0      2      4      6
/// ```
///
/// `elems` is the second table. The first is written straight into `env` as the
/// body is walked, which is also what makes a member usable in a LATER member's
/// count expression (`ds.b SMPS_RAM.v_1up_ram_end-SMPS_RAM.v_1up_ram`, the last
/// line of Sonic 1's `SMPS_RAM`).
#[derive(Clone, Debug)]
struct StructDef {
    /// What joins the struct name to a member name — `.` when the declaration
    /// carries the `DOTS` modifier, `_` otherwise (probe `q8.asm`: a bare
    /// `A struct` yields `A_a`/`A_len`, and probe `q11.asm` shows an INSTANCE of
    /// it yields `j_u` — the separator is a property of the struct, not of the
    /// site). Case-folded recognition: S2 writes `struct dots`, `STRUCT DOTS`.
    sep: char,
    /// Total size, in bytes: the running offset at `endstruct`. Bound as
    /// `NAME<sep>len`.
    len: i64,
    /// `(member path, offset from the instance base)`, in declaration order —
    /// the offsets an instantiation adds to its base. A member path may itself
    /// be dotted where a struct body embeds another struct: Sonic 1's
    /// `SMPS_RAM` embeds 17 `SMPS_Track`s and asl FLATTENS them, so
    /// `SMPS_RAM.v_music_dac_track.PlaybackControl` is one element here.
    elems: Vec<(String, i64)>,
}

/// One line of a struct body, after the label column is split off.
enum StructMember {
    /// `[name:] ds.b|ds.w|ds.l <count>` — reserves `width * count` bytes.
    Field { name: String, width: i64, count: i64 },
    /// `[name:] <another struct's name>` — embeds that struct's whole layout.
    Embed { name: String, struct_name: String },
    /// `name:` alone. Binds the running offset and reserves nothing; Sonic 1's
    /// `SMPS_RAM` has 21 of them (`v_1up_ram`, `v_track_ram`, every
    /// `*_tracks_end`) and four are read by name from the corpus.
    Marker { name: String },
    /// A non-empty body line whose LABEL COLUMN does not begin with an
    /// identifier, so nothing about it can be read. Reported rather than
    /// skipped, because skipping a struct-body line is a wrong SIZE and not a
    /// missing symbol, and a wrong size moves every variable declared after
    /// the instance.
    ///
    /// This is not hypothetical. Sonic 2's `zVar` declares
    /// `1upPlaying: ds.b 1` — asl accepts an identifier that begins with a
    /// digit and sigil's lexer does not — and skipping it made `zVar.len`
    /// $17 against asl's $18, with **exit 0 on both sides and no diagnostic
    /// anywhere**. Probe `q19.asm`.
    Unreadable,
}

/// Per-pass ceiling on total `while`-body executions (see `Asm::while_budget`).
/// Far above any real Aeon `while`-driven data table, far below the `WHILE_CAP²`
/// (10⁸) a pair of nested non-convergent loops would otherwise grind through.
const GLOBAL_WHILE_CAP: usize = 1_000_000;

enum Lowered {
    Fixed(Vec<Operand>),
    Rel(Option<Cond>, Expr),
    Abs16(Vec<Operand>, Expr),
}

impl Asm {
    /// Sets [`Asm::defer_unresolved_jsr_jmp`] — `false` for every ordinary
    /// pass; `true` only for `run`'s bonus final pass (see that field's doc).
    fn new_with_defer(opts: &Options, defer_unresolved_jsr_jmp: bool) -> Self {
        Asm {
            builder: IrBuilder::new(),
            z80: Z80Backend,
            m68k: M68kBackend,
            state: crate::state::AsmState::new(opts.initial_cpu),
            env: SymbolTable::new(),
            str_env: std::collections::HashMap::new(),
            float_env: std::collections::HashMap::new(),
            scope: None,
            outer_scope: None,
            dot_label_cache: std::collections::BTreeMap::new(),
            in_section: false,
            phys_base: 0,
            diags: Vec::new(),
            source: SourceId(0),
            sources: sigil_span::SourceMap::new(),
            functions: std::collections::BTreeMap::new(),
            macros: std::collections::BTreeMap::new(),
            pending_int_label: None,
            macro_depth: 0,
            macro_frames: Vec::new(),
            macro_expansion_seq: 0,
            visited: std::collections::BTreeSet::new(),
            include_root: opts.include_root.clone(),
            aborted: false,
            cpu_refused: false,
            poison_refs: Vec::new(),
            while_budget: GLOBAL_WHILE_CAP,
            pending_equ_syms: Vec::new(),
            defer_unresolved_jsr_jmp,
            known_labels: std::collections::HashSet::new(),
            label_ref_equs: std::collections::HashSet::new(),
            set_sym_symbolic: std::collections::HashMap::new(),
            guarded_defines: opts.guarded_defines.iter().map(|(k, _)| k.clone()).collect(),
            structs: std::collections::HashMap::new(),
            pending_struct_label: None,
        }
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.diags.push(Diagnostic {
            level: Level::Error,
            message: msg.into(),
            primary: span,
        });
    }

    /// The continuous PHYSICAL location counter (real ROM/LMA offset): the open
    /// section's `phys_base` plus its running byte cursor. When no section is open
    /// (just after cpu/phase/dephase closed one, before the next emit reopens it),
    /// `phys_base` has already absorbed the closed section's length, so the current
    /// physical position is simply `phys_base`.
    fn current_physical(&self) -> u32 {
        self.phys_base
            + if self.in_section {
                self.builder.current_offset()
            } else {
                0
            }
    }

    /// The current VMA (`$`/label address): `physical + phase displacement`. Under
    /// no phase (`disp == 0`) this equals the physical location; inside a `phase
    /// addr` block it equals `addr + bytes-since-phase` (the window VMA).
    fn here(&self) -> u32 {
        (self.current_physical() as i64 + self.state.disp) as u32
    }

    /// The current PC as a SIGN-EXTENDED 32→64-bit value: an address with bit 31
    /// set (the 68k RAM aliases `$FFFF0000`/`$FFFF8000`+) becomes NEGATIVE, exactly
    /// as asl stores a phased label (`$FFFFFFFFFFFF80AC` = −32596 for a label at
    /// `$FFFF80AC`). This is what makes `move.w #RAM_Label, d0` fold in range: the
    /// low-RAM address is a small negative that fits a signed word, whereas the
    /// raw unsigned `4294934700` overflows. Byte-identical to the unsigned form
    /// for every wider use (abs.l / `.l` immediate truncate back to the same 32
    /// bits; abs.w / disp16 take the same low word). ROM addresses (< `$80000000`)
    /// are unaffected — sign-extension is a no-op there.
    fn here_i64(&self) -> i64 {
        self.here() as i32 as i64
    }

    /// The scope a value-BINDING `.`-local takes inside a macro expansion: the
    /// nearest scope that is not an expansion. Outside a macro this is just the
    /// current scope.
    /// Where there is no enclosing global label at all, the scope is the EMPTY
    /// one rather than "no scope". `qualify` already writes such a name under its
    /// own bare spelling (`.v`), and AS resolves it: a macro whose body carries
    /// `.v := 7` and `.lb:`/`beq.s .lb` in a file with no label above it lists
    /// `67FC` and `dc.b .v` reads `07`. Handing the empty scope through makes the
    /// READER build the same key the writer used, instead of refusing for want of
    /// a scope name.
    fn real_scope(&self) -> Option<&str> {
        let s = if self.macro_frames.is_empty() {
            self.scope.as_deref()
        } else {
            self.outer_scope.as_deref()
        };
        Some(s.unwrap_or(""))
    }

    /// The scope a `.`-local REFERENCE resolves in, and the whole resolution
    /// rule in one place.
    ///
    /// A name the innermost expansion's body defines as a PLAIN LABEL belongs to
    /// that expansion; every other `.`-local belongs to the caller's real scope.
    /// The set is [`scan_dot_labels`] of the body, computed before the body runs,
    /// so a name's scope is a property of the MACRO, not of where in the body the
    /// reference happens to sit.
    ///
    /// That is what forecloses the wrong-label fall-through. A rule of the shape
    /// "look in the expansion, and if that misses fall back to the caller" makes
    /// a macro's own forward branch to `.done` reach a caller's `.done` whenever
    /// the expansion has not defined its own yet — a missing-label error turning
    /// into a branch to the wrong address. asl does exactly that, and it is not
    /// even self-consistent about it: `mown` (body `beq.s .tgt` … `.tgt:`) called
    /// under a `Base:` that also carries `.tgt:` assembles `67FE`, the CALLER's
    /// label, in a single-pass file (`d1.asm`), and `6704`, its OWN, the moment
    /// an unrelated forward reference elsewhere forces a second pass
    /// (`d2.asm` — same construction, `2 passes`). Under this rule the body's own
    /// definition wins in both, because the expansion owns the name for the whole
    /// expansion or does not own it at all. There is no lookup order to lose a
    /// race in, and no pass on which the answer differs.
    ///
    /// A body that does NOT define the name still reaches the caller, which is
    /// asl's behaviour and is load-bearing: `mref` (body `beq.s .tgt`, no
    /// definition) under `Base:` with a later `.tgt:` assembles `6704`, and
    /// `Base.tgt : 1006 C` is in the symbol table.
    fn dot_scope(&self, name: &str) -> Option<&str> {
        match self.macro_frames.last() {
            Some(f) if !f.dot_labels.contains(name) => self.real_scope(),
            _ => Some(self.scope.as_deref().unwrap_or("")),
        }
    }

    /// The value of a numeric BUILTIN symbol — one whose value the assembler
    /// holds itself rather than reading from the program's symbol table.
    /// Resolved in the front end so such a name folds to a concrete value
    /// immediately and never survives as a `Sym` fixup target for the linker.
    ///
    /// - `$` — the current program counter.
    /// - `MOMCPU` — the CPU currently selected, as the integer asl reports it:
    ///   `$68000` under the 68000 (`dc.l MOMCPU` ⇒ `0006 8000`) and `$80`
    ///   under the Z80 (`dw MOMCPU` ⇒ `80 00`). This is the value
    ///   `s2.macrosetup.asm`'s
    ///   `notZ80 function cpu,(cpu<>128)&&(cpu<>32988)` tests, and it is not
    ///   an ornament: leaving it undefined makes every `if notZ80(MOMCPU)` in
    ///   that file read FALSE, so the Z80 arm of `org`, `cnop`, `align`,
    ///   `even` and `ds` is what gets assembled under the 68000. That is a
    ///   wrong branch, not a missing symbol — it emits bytes.
    /// - `TRUE` / `FALSE` — 1 and 0 (`dc.b TRUE,FALSE` ⇒ `0100`). Undefined,
    ///   they take the same shape as `MOMCPU`: `s2.macrosetup.asm:76`'s
    ///   `if TRUE` reads FALSE and drops the block it guards without a word.
    ///
    /// A builtin outranks the symbol table, which is asl's own rule and not a
    /// simplification: it refuses `TRUE = 7` and `MOMCPU = 9` outright
    /// (`error #2035: variables cannot be redefined as constants`) and goes on
    /// reporting 1 and `$68000`.
    fn builtin_num(&self, name: &str) -> Option<i64> {
        match name {
            "$" => Some(self.here_i64()),
            "MOMCPU" => Some(match self.state.cpu {
                Cpu::M68000 => 0x68000,
                Cpu::Z80 => 0x80,
            }),
            "TRUE" => Some(1),
            "FALSE" => Some(0),
            _ => None,
        }
    }

    fn fold(&self, e: &Expr) -> Fold {
        let env = &self.env;
        e.fold(&|name| {
            self.builtin_num(name)
                .or_else(|| env.resolve(name, self.dot_scope(name)))
        })
    }

    /// Fold an immediate to a value in [lo,hi]. Out-of-range → diagnostic + clamp.
    /// Unresolved (Poison) → 0 placeholder for THIS pass (byte-stable so a forward
    /// ref that resolves on a later pass doesn't perturb layout), but the offending
    /// symbol names are recorded: on the converged pass `run` promotes them to
    /// unresolved-symbol errors (the env is final there, so a still-Poison operand
    /// is genuinely undefined rather than a pending forward ref).
    fn fold_imm(&mut self, e: &Expr, span: Span, lo: i64, hi: i64) -> i64 {
        match self.fold(e) {
            Fold::Value(v) if v >= lo && v <= hi => v,
            Fold::Value(v) => {
                self.err(span, format!("operand {v} out of range {lo}..={hi}"));
                v.clamp(lo, hi)
            }
            Fold::Poison => {
                for name in self.unresolved_names(e) {
                    self.poison_refs.push((name, span));
                }
                0
            }
        }
    }

    /// Collect the symbol names in `e` that do NOT resolve in the current env
    /// (ignoring the builtins `fold` handles itself — see [`Self::builtin_num`]).
    /// These are the names that made an operand fold to Poison.
    fn unresolved_names(&self, e: &Expr) -> Vec<String> {
        fn walk(this: &Asm, e: &Expr, out: &mut Vec<String>) {
            match e {
                Expr::Int(_) => {}
                Expr::Sym(name) => {
                    if this.builtin_num(name).is_none()
                        && this.env.resolve(name, this.dot_scope(name)).is_none()
                    {
                        out.push(name.clone());
                    }
                }
                Expr::Binary { lhs, rhs, .. } => {
                    walk(this, lhs, out);
                    walk(this, rhs, out);
                }
                Expr::Unary { operand, .. } => walk(this, operand, out),
            }
        }
        let mut out = Vec::new();
        walk(self, e, &mut out);
        // A name can appear more than once in one operand (e.g. `X+X`); report it once.
        out.sort();
        out.dedup();
        out
    }

    /// Fold a whole token slice as one constant expression (used by phase, etc.).
    ///
    /// Also expands the front-end-only `int(...)`/`sin(...)` and debug-string
    /// (`strlen`/`strstr`/`val`, with `substr`/`lowstring` nesting) builtins
    /// (T9.3) — the same two passes `directive_db` already ran before parsing
    /// a `dc.b` argument. Without this, `<name> set strstr(str,"%<")` (the
    /// idiom the debugger's `__FSTRING_*` macros use throughout) left
    /// `strstr(...)` as un-expanded tokens here, since `eval_all` backs
    /// `directive_set`/`while`/`if`/`rept`/`phase`/`org`/`align`/`ds`, and
    /// previously only `directive_db` wired the builtins in. Wiring them in
    /// HERE too — rather than only where the gap was first noticed — is what
    /// makes `while`+`set` actually compose with the string builtins, so a
    /// `while (pos>=0) / pos: set strstr(...)` loop (T9.2 `while` + T9.1
    /// `strstr`) now really assembles.
    fn eval_all(&mut self, toks: &[Token], span: Span) -> Option<i64> {
        let expanded = self.expand_calls(toks, 0);
        let expanded = self.expand_int_builtin(&expanded);
        let expanded = self.expand_str_builtins(&expanded);
        let expanded = self.expand_str_comparisons(&expanded);
        let (e, rest) = crate::expr::parse_expr(&expanded)?;
        if !rest.is_empty() {
            self.err(span, "trailing tokens in expression");
            return None;
        }
        match self.fold(&e) {
            Fold::Value(v) => Some(v),
            Fold::Poison => None,
        }
    }

    /// Evaluate front-end-only `int(...)` builtin calls in `toks` (§7.4:
    /// `sin`/`int` are FRONT-END builtins — they must NEVER become
    /// `sigil_ir::Expr` nodes, so this runs as token-level preprocessing
    /// BEFORE `crate::expr::parse_expr` ever sees the line). Scans for each
    /// top-level `int(` call — spelled in ANY case, since asl matches builtin
    /// FUNCTION names case-insensitively even under `-U` (probe `f1.asm`:
    /// `INT(3.7)` and `int(3.7)` both assemble to `3`, and S1's
    /// `MacroSetup.asm:218` writes `roundFloatToInteger function
    /// float,INT(float+0.5)` in capitals) — evaluates its single argument as a
    /// TYPED expression via `eval_num` (which recognizes nested `sin(...)`/
    /// `int(...)` calls itself, so `int(sin(int(x)))`-style nesting works),
    /// floors a float result and passes an integer one through unchanged
    /// (AS's `int()` = floor: `INT(-3.7)` = -4, `INT(7)` = 7), and replaces
    /// the whole `int(...)` span with
    /// a single resolved `Tok::Int` — a completely ordinary integer literal
    /// from here on, indistinguishable from one the source author wrote by
    /// hand. A bare `sin(...)` not wrapped in `int(...)` has no integer
    /// meaning and is left untouched (whatever consumes it downstream will
    /// report a normal "bad expression" diagnostic).
    fn expand_int_builtin(&mut self, toks: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if name.eq_ignore_ascii_case("int")
                    && matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    )
                {
                    let span = toks[i].span;
                    if let Some((args, next)) = split_call_args(toks, i + 1) {
                        let value = match args.as_slice() {
                            [arg] => self.eval_num(arg),
                            _ => None,
                        };
                        match value {
                            Some(v) => out.push(Token {
                                tok: Tok::Int(match v {
                                    Num::Int(i) => i,
                                    Num::Float(f) => f.floor() as i64,
                                }),
                                span,
                            }),
                            None => {
                                self.err(span, "int(): could not evaluate float expression");
                                out.push(Token {
                                    tok: Tok::Int(0),
                                    span,
                                });
                            }
                        }
                        i = next;
                        continue;
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Evaluate a front-end-only TYPED expression tree — the evaluator behind
    /// `int(...)`/`sin(...)` arguments and float-valued symbol assignments.
    ///
    /// It walks the SAME operator surface as [`crate::expr::parse_expr`] and
    /// borrows that module's [`crate::expr::infix_bp`] ladder verbatim, so the
    /// two cannot drift on precedence. What differs is only the value domain:
    /// this one carries [`Num`] (int XOR float) and applies AS's type rules,
    /// whereas `parse_expr` builds an `Expr` tree that is folded in i64.
    ///
    /// `None` on any unresolved symbol, malformed shape, or type error —
    /// mirrors `Fold::Poison` in spirit. The whole tree stays out of
    /// `sigil_ir::Expr` (§7.4): a float never becomes an IR node, it is
    /// collapsed to a `Tok::Int` by [`Self::expand_int_builtin`] first.
    fn eval_num(&self, toks: &[Token]) -> Option<Num> {
        let (v, rest) = self.parse_num_bp(toks, 0)?;
        rest.is_empty().then_some(v)
    }

    /// The f64 value of a front-end-only expression, for callers that want a
    /// float regardless of the operand types (`sin(...)`'s argument).
    fn eval_float(&self, toks: &[Token]) -> Option<f64> {
        self.eval_num(toks).map(Num::as_f64)
    }

    fn parse_num_bp<'t>(&self, toks: &'t [Token], min_bp: u8) -> Option<(Num, &'t [Token])> {
        let (mut lhs, mut rest) = self.parse_num_atom(toks)?;
        while let Some(Tok::Punct(p)) = rest.first().map(|t| &t.tok) {
            let (bp, op) = match crate::expr::infix_bp(*p) {
                Some(x) if x.0 > min_bp => x,
                _ => break,
            };
            let (rhs, r2) = self.parse_num_bp(&rest[1..], bp)?;
            lhs = apply_num_binop(op, lhs, rhs)?;
            rest = r2;
        }
        Some((lhs, rest))
    }

    fn parse_num_atom<'t>(&self, toks: &'t [Token]) -> Option<(Num, &'t [Token])> {
        let (head, rest) = toks.split_first()?;
        match &head.tok {
            Tok::Float(f) => Some((Num::Float(*f), rest)),
            Tok::Int(n) => Some((Num::Int(*n), rest)),
            // Unary minus is TYPE-PRESERVING (`INT(-3.7)` = -4, `INT(-7/2)`
            // = -3): negating an int keeps it an int.
            Tok::Punct(Punct::Minus) => {
                let (v, r) = self.parse_num_atom(rest)?;
                Some((
                    match v {
                        Num::Int(i) => Num::Int(i.wrapping_neg()),
                        Num::Float(f) => Num::Float(-f),
                    },
                    r,
                ))
            }
            // `~x` / `~~x` are INTEGER operators; asl refuses a float operand.
            Tok::Punct(Punct::Tilde) => {
                let (v, r) = self.parse_num_atom(rest)?;
                Some((Num::Int(!v.as_i64()?), r))
            }
            Tok::Punct(Punct::TildeTilde) => {
                let (v, r) = self.parse_num_atom(rest)?;
                Some((Num::Int((v.as_i64()? == 0) as i64), r))
            }
            Tok::Punct(Punct::LParen) => {
                let (v, r) = self.parse_num_bp(rest, 0)?;
                match r.first().map(|t| &t.tok) {
                    Some(Tok::Punct(Punct::RParen)) => Some((v, &r[1..])),
                    _ => None,
                }
            }
            // asl's builtin FUNCTION names are matched case-insensitively even
            // under `-U` (which makes user SYMBOLS case-sensitive): probe
            // `f1.asm` assembles `INT(3.7)` and `int(3.7)` identically to `3`,
            // and asl reports an unknown one uppercased (`error #1860: unknown
            // function MIN` for a written `min(`).
            Tok::Ident(name)
                if (name.eq_ignore_ascii_case("sin") || name.eq_ignore_ascii_case("int"))
                    && matches!(
                        rest.first().map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    ) =>
            {
                let (args, next) = split_call_args(rest, 0)?;
                let inner = match args.as_slice() {
                    [arg] => self.eval_num(arg)?,
                    _ => return None,
                };
                let v = if name.eq_ignore_ascii_case("sin") {
                    Num::Float(inner.as_f64().sin())
                } else {
                    // `INT` of an INTEGER is that integer (probe `f1.asm(11)`:
                    // `dc.l INT(7)` -> `0000 0007`), and of a float is its
                    // FLOOR, not a truncation toward zero (`INT(-3.7)` ->
                    // `FFFF FFFC` = -4, `INT(-3.2)` -> -4, `INT(-3.0)` -> -3).
                    match inner {
                        Num::Int(i) => Num::Int(i),
                        Num::Float(f) => Num::Int(f.floor() as i64),
                    }
                };
                Some((v, &rest[next..]))
            }
            Tok::Ident(name) => {
                // A float-valued symbol (`sample_rate_scale := 1.0`, S2's
                // `dac_sample_metadata`) outranks the integer table: the two
                // are disjoint by construction (an assignment writes one and
                // clears the other), so the order only decides which stale
                // entry loses, never which live one wins.
                if let Some(f) = self.resolve_float_sym(name) {
                    return Some((Num::Float(f), rest));
                }
                let v = match self.builtin_num(name) {
                    Some(v) => v,
                    None => self.env.resolve(name, self.dot_scope(name))?,
                };
                Some((Num::Int(v), rest))
            }
            Tok::Dollar => Some((Num::Int(self.here_i64()), rest)),
            Tok::Punct(Punct::Star) => Some((Num::Int(self.here_i64()), rest)),
            _ => None,
        }
    }

    /// The f64 bound to a front-end-only float symbol, or `None`. Scoped
    /// exactly like [`Self::resolve_str`] / the integer env.
    fn resolve_float_sym(&self, name: &str) -> Option<f64> {
        // The same key the assignment wrote, built by the same function, so a
        // reader can never disagree with its writer about where a name lives.
        let key = qualify(name, self.dot_scope(name));
        self.float_env.get(&key).copied()
    }

    /// Evaluate front-end-only debug-string builtin calls
    /// (`strlen`/`strstr`/`val`) in `toks`, replacing each TOP-LEVEL call span
    /// with a resolved `Tok::Int` — the same shape as `expand_int_builtin`
    /// (§7.4: these are FRONT-END builtins; the string values involved never
    /// become `sigil_ir::Expr` nodes). `substr(...)` itself produces a
    /// STRING, not an int, so it is never substituted at this top level — it
    /// is only ever consumed as a nested argument (via `eval_str`) inside one
    /// of these three, which is how `strlen(substr(...))` /
    /// `strstr(substr(s,p,0),">")` nesting works.
    fn expand_str_builtins(&mut self, toks: &[Token]) -> Vec<Token> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if matches!(name.as_str(), "strlen" | "strstr" | "val")
                    && matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    )
                {
                    let span = toks[i].span;
                    if let Some((args, next)) = split_call_args(toks, i + 1) {
                        match self.eval_str_builtin(name, &args) {
                            Some(v) => out.push(Token {
                                tok: Tok::Int(v),
                                span,
                            }),
                            None => {
                                self.err(
                                    span,
                                    format!("{name}(): could not evaluate string builtin"),
                                );
                                out.push(Token {
                                    tok: Tok::Int(0),
                                    span,
                                });
                            }
                        }
                        i = next;
                        continue;
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Fold `<string-expr> (= | <>) "literal"` sub-patterns to a `Tok::Int(0/1)`
    /// so asl string comparisons compose INSIDE boolean expressions
    /// (`&&`/`||`/parens), not just as a whole `if` condition. Runs after
    /// [`Self::expand_str_builtins`] (so `strlen`/`strstr`/`val` are already
    /// ints) and before `parse_expr`.
    ///
    /// The discriminator that a comparison is string-typed is a **string-literal
    /// RHS** (asl: `"a"="b"` folds to 0/1 — probe `probe_strcmp` 2026-07-05:
    /// `((strlen(t)==2)&&(substr(t,0,1)=="."))` = true, `("x"<>"y")` = 1). The
    /// LHS is the trailing string-expr in the already-emitted output — a string
    /// literal, a string-valued `set` symbol, or a `substr(...)`/`lowstring(...)`
    /// call, exactly what [`Self::eval_str`] resolves. If the trailing tokens
    /// don't resolve to a string the operator is left untouched (ordinary numeric
    /// `=`), so a genuine `numeric = 5` is unaffected (its RHS isn't a string
    /// anyway). Only `debugger.asm`'s `%<…>` decoder exercises this; latent until
    /// the `__DEBUG__` build (M1.D T5).
    fn expand_str_comparisons(&self, toks: &[Token]) -> Vec<Token> {
        let mut out: Vec<Token> = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            let is_cmp = matches!(toks[i].tok, Tok::Punct(Punct::Eq) | Tok::Punct(Punct::Ne));
            if is_cmp {
                if let Some((rhs, next)) = self.leading_str_rhs(&toks[i + 1..]) {
                    if let Some(lhs_len) = trailing_str_expr_len(&out) {
                        let lhs = &out[out.len() - lhs_len..];
                        if let Some(lv) = self.eval_str(lhs) {
                            let ne = matches!(toks[i].tok, Tok::Punct(Punct::Ne));
                            let eq = lv == rhs;
                            let span = toks[i].span;
                            out.truncate(out.len() - lhs_len);
                            out.push(Token { tok: Tok::Int((eq ^ ne) as i64), span });
                            i += 1 + next;
                            continue;
                        }
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// The string value at the START of `toks` and the token count it spans —
    /// the right-hand operand of a `=`/`<>` whose left side is string-typed. A
    /// bare `Tok::Str` literal (1 token) is the written form; a balanced
    /// `( … )` group holding one is the SUBSTITUTED form, which is what a
    /// comparison against a user `function`'s parameter always looks like
    /// (`expand_calls` parenthesises each argument, so `chkop`'s `<>ref`
    /// becomes `<>("0(")`). `None` when the operand is not a string, which
    /// leaves an ordinary numeric comparison alone.
    fn leading_str_rhs(&self, toks: &[Token]) -> Option<(String, usize)> {
        match toks.first()?.tok {
            Tok::Str(ref s) => Some((s.clone(), 1)),
            Tok::Punct(Punct::LParen) => {
                let end = matching_rparen(toks, 0)?;
                let v = self.eval_str(&toks[..=end])?;
                Some((v, end + 1))
            }
            _ => None,
        }
    }

    /// Dispatch one of the debug-string builtins that produce an INTEGER:
    ///
    /// - `strlen(str)` → character count.
    /// - `strstr(haystack, needle)` → **STANDARD** 0-based index of the first
    ///   match, or **-1** if absent (asl 1.42 Bld 212 verified: `strstr("abc",
    ///   "c")`=2, `strstr("b>",">")`=1, `strstr("xab","ab")`=1,
    ///   `strstr("abc","z")`=-1 — the alleged "fails on last char" bug does
    ///   NOT reproduce in this asl; deliberately NOT emulated here).
    /// - `val(str)` → re-lexes `str` fresh and folds it as an ordinary AS
    ///   constant expression against the CURRENT env/scope (NOT just a number
    ///   parse): `val("$80")`=0x80, `val("144")`=144, `val("hex+1")` resolves
    ///   symbol `hex` the same way any operand would.
    fn eval_str_builtin(&self, name: &str, args: &[Vec<Token>]) -> Option<i64> {
        match (name, args) {
            ("strlen", [s]) => Some(self.eval_str(s)?.chars().count() as i64),
            ("strstr", [hay, needle]) => {
                let hay = self.eval_str(hay)?;
                let needle = self.eval_str(needle)?;
                Some(match hay.find(&needle) {
                    // `find` returns a BYTE offset; convert to a char count so
                    // a (hypothetical) non-ASCII haystack still reports the
                    // same index asl's char-oriented `strstr` would.
                    Some(byte_idx) => hay[..byte_idx].chars().count() as i64,
                    None => -1,
                })
            }
            ("val", [s]) => self.fold_str_as_expr(&self.eval_str(s)?),
            _ => None,
        }
    }

    /// Resolve a bare identifier reference to its string value, if it names a
    /// string-valued `set` symbol. `.foo` → `"{scope}.foo"`, `A.b`/`foo` →
    /// verbatim, with the scope chosen by [`Self::dot_scope`].
    fn resolve_str(&self, name: &str) -> Option<String> {
        // The same key `directive_set` wrote, built by the same function, so a
        // reader can never disagree with its writer about where a name lives.
        let key = qualify(name, self.dot_scope(name));
        self.str_env.get(&key).cloned()
    }

    /// Evaluate a front-end-only STRING expression: a plain `Tok::Str`
    /// literal, or a nested `substr(str, pos, len)` / `lowstring(str)` call.
    /// `None` on any other shape (mirrors `Fold::Poison` in spirit — this
    /// value never becomes a `sigil_ir::Expr`, per §7.4). Both nested forms
    /// recurse through `eval_str` for their own string argument, so
    /// `lowstring(substr(...))` / `substr(lowstring(...), ...)` nest freely
    /// (T9.3).
    fn eval_str(&self, toks: &[Token]) -> Option<String> {
        // Parentheses around a string expression are transparent, exactly as
        // they are around a numeric one: asl folds `strlen(("abc"))` to 3 and
        // `strlen(lowstring(("ABCD")))` to 4. This is not a curiosity —
        // `expand_calls` PARENTHESISES every argument it substitutes into a
        // user `function` body, so `chkop function op,ref,(...strlen(ref)...)`
        // hands its own `strlen` a `("0(")`, and a `substr`/`lowstring`/
        // comparison chain over function parameters is unreachable without
        // this peel.
        if let Some(inner) = peel_parens(toks) {
            return self.eval_str(inner);
        }
        if let [Token {
            tok: Tok::Str(s), ..
        }] = toks
        {
            return Some(s.clone());
        }
        if let [Token {
            tok: Tok::Ident(name),
            ..
        }] = toks
        {
            if let Some(s) = self.resolve_str(name) {
                return Some(s);
            }
        }
        if let [Token {
            tok: Tok::Ident(name),
            ..
        }, ..] = toks
        {
            if name == "substr"
                && matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen)))
            {
                let (args, next) = split_call_args(toks, 1)?;
                if next == toks.len() {
                    return self.eval_substr(&args);
                }
            }
            if name == "lowstring"
                && matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Punct(Punct::LParen)))
            {
                let (args, next) = split_call_args(toks, 1)?;
                if next == toks.len() {
                    if let [s_toks] = args.as_slice() {
                        return self.eval_str(s_toks).map(|s| s.to_lowercase());
                    }
                }
            }
        }
        None
    }

    /// `substr(str, pos, len)`: 0-based `pos`; `len == 0` means "from `pos`
    /// to the end of the string" (asl-verified: `substr("hello",1,0)` =
    /// "ello", `substr("hello",1,2)` = "el"). `pos`/`len` are ordinary
    /// constant expressions (a literal, a symbol, arithmetic, …) — not
    /// further string-builtin calls; only the first (`str`) argument nests.
    fn eval_substr(&self, args: &[Vec<Token>]) -> Option<String> {
        let [s_toks, pos_toks, len_toks] = args else {
            return None;
        };
        let s = self.eval_str(s_toks)?;
        let pos = self.fold_const(pos_toks)?;
        let len = self.fold_const(len_toks)?;
        if pos < 0 {
            return None;
        }
        let chars: Vec<char> = s.chars().collect();
        // asl edge semantics (probe `probe_substr` 2026-07-05): a `pos` at OR past
        // the end yields "" (not an error — `substr("abc",5,0)`=""), and a NEGATIVE
        // len also yields "" (`substr("abc",3,-1)`=""). Both are hit by
        // `debugger.asm`'s `%<…>` decoder when a token has no trailing param (e.g.
        // `%<.w d0>` → `.__param: set substr(string, len, -1)` → "" → defaults to
        // "hex"). A len past the end clamps to the available tail (already matched).
        let pos = (pos as usize).min(chars.len());
        let end = match len {
            0 => chars.len(),                             // len 0 = to end
            n if n > 0 => (pos + n as usize).min(chars.len()),
            _ => pos,                                      // negative len = empty
        };
        Some(chars[pos..end].iter().collect())
    }

    /// `val(str)`: lex `text` fresh (under the CURRENT cpu context) and fold
    /// it as an ordinary constant expression — this is what makes `val` an
    /// AS-EXPRESSION evaluator rather than a plain number parse (it resolves
    /// symbols, honors `$`-prefixed hex, arithmetic, …).
    fn fold_str_as_expr(&self, text: &str) -> Option<i64> {
        let toks = lex_line(text, self.state.cpu, self.source, 0).ok()?;
        self.fold_const(&toks)
    }

    /// Fold a token slice as a plain constant integer expression — the
    /// immutable counterpart of `eval_all` (no diagnostics on failure; `None`
    /// mirrors `Fold::Poison`). Used by the debug-string evaluator wherever a
    /// nested piece is known to be an INTEGER, never a string (`substr`'s
    /// `pos`/`len` arguments, and `val`'s re-lexed expression text).
    fn fold_const(&self, toks: &[Token]) -> Option<i64> {
        let expanded = self.expand_calls(toks, 0);
        let (e, rest) = crate::expr::parse_expr(&expanded)?;
        if !rest.is_empty() {
            return None;
        }
        match self.fold(&e) {
            Fold::Value(v) => Some(v),
            Fold::Poison => None,
        }
    }

    /// Parse a name-first AS `function` definition and store it.
    ///
    /// Real AS / aeon syntax: `<name> function <formal_args...>, <body_expr>`, e.g.
    /// `timerAReload function mhz, 1024 - (1000000000000 / ((mhz) * 18773))`.
    /// The comma-separated items after `function` are the formal parameters,
    /// except the LAST, which is the body expression. (In aeon every function has
    /// exactly one formal, but this handles any arity.)
    fn def_function(&mut self, line: &SrcLine) {
        let substituted = self.subst_frame(line);
        let line = substituted.as_ref().unwrap_or(line);
        let toks = match lex_line(&line.text, self.state.cpu, line.source, line.base) {
            Ok(t) => t,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        // toks[0] = name, toks[1] = `function`, toks[2..] = formals..., body.
        let span = toks.first().map(|t| t.span).unwrap_or(Span {
            source: line.source,
            start: line.base,
            end: line.base,
        });
        let name = match toks.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "function needs a name");
                return;
            }
        };
        if !matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if fold_kw(s) == "function") {
            self.err(span, "function needs the `function` keyword");
            return;
        }
        let groups = split_top_commas(&toks[2..]);
        // Need at least one formal group plus the body group.
        if groups.len() < 2 || groups.last().map(|g| g.is_empty()).unwrap_or(true) {
            self.err(span, "function needs `<params...>, <body>`");
            return;
        }
        let body = groups[groups.len() - 1].to_vec();
        let mut params = Vec::new();
        for g in &groups[..groups.len() - 1] {
            match g {
                [Token {
                    tok: Tok::Ident(p), ..
                }] => params.push(p.clone()),
                _ => {
                    self.err(span, "bad function parameter");
                    return;
                }
            }
        }
        self.functions.insert(name, (params, body));
    }

    /// Expand every known-function call `fname(args)` in `toks` into its
    /// parenthesised, parameter-substituted body (recursively). Unknown `Ident(`
    /// is left untouched (it may be a `(nn)`-style group, not a call).
    fn expand_calls(&self, toks: &[Token], depth: usize) -> Vec<Token> {
        if depth > EXPAND_CAP {
            return toks.to_vec();
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < toks.len() {
            if let Tok::Ident(name) = &toks[i].tok {
                if let Some((params, body)) = self.functions.get(name) {
                    if matches!(
                        toks.get(i + 1).map(|t| &t.tok),
                        Some(Tok::Punct(Punct::LParen))
                    ) {
                        if let Some((args, next)) = split_call_args(toks, i + 1) {
                            let expanded = self.substitute(body, params, &args, depth);
                            let span = toks[i].span;
                            out.push(paren(Punct::LParen, span));
                            out.extend(self.expand_calls(&expanded, depth + 1));
                            out.push(paren(Punct::RParen, span));
                            i = next;
                            continue;
                        }
                    }
                }
            }
            out.push(toks[i].clone());
            i += 1;
        }
        out
    }

    /// Replace each body identifier equal to a parameter with its (expanded,
    /// parenthesised) argument tokens.
    fn substitute(
        &self,
        body: &[Token],
        params: &[String],
        args: &[Vec<Token>],
        depth: usize,
    ) -> Vec<Token> {
        let mut out = Vec::new();
        for t in body {
            if let Tok::Ident(name) = &t.tok {
                if let Some(idx) = params.iter().position(|p| p == name) {
                    if let Some(arg) = args.get(idx) {
                        let expanded_arg = self.expand_calls(arg, depth + 1);
                        out.push(paren(Punct::LParen, t.span));
                        out.extend(expanded_arg);
                        out.push(paren(Punct::RParen, t.span));
                        continue;
                    }
                }
            }
            out.push(t.clone());
        }
        out
    }

    /// Execute a root source. `root_name` is the file it was read from — it names
    /// every diagnostic that lands outside an `include`, so it is the name a user
    /// sees first. An empty name registers the root as unnamed (a string with no
    /// file behind it), and diagnostics there render without a location.
    fn process(&mut self, root_name: &str, src: &str) {
        let id = self.sources.add_named(root_name.to_string(), src.to_string());
        self.source = id;
        let lines = split_src_lines(src, id);
        self.exec(&lines);
    }

    /// Fold `\{expr}` sequences in the first string token to their decimal value.
    fn interp_string(&mut self, rest: &[Token]) -> String {
        let raw = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
            Some(s) => s,
            None => return String::new(),
        };
        self.interp_text(&raw)
    }

    /// Fold every `\{expr}` sequence in `raw` to the expression's decimal value.
    /// A sequence whose expression does not resolve is left verbatim, so a later
    /// pass (or a diagnostic at the use site) still sees the original text.
    /// Idempotent on text that carries no `\{`.
    fn interp_text(&mut self, raw: &str) -> String {
        let mut out = String::new();
        let mut cur = raw;
        while let Some(pos) = cur.find("\\{") {
            out.push_str(&cur[..pos]);
            let after = &cur[pos + 2..];
            match after.find('}') {
                Some(end) => {
                    let expr_text = &after[..end];
                    match self.fold_text(expr_text) {
                        Some(v) => out.push_str(&v.to_string()),
                        None => {
                            out.push_str("\\{");
                            out.push_str(expr_text);
                            out.push('}');
                        }
                    }
                    cur = &after[end + 1..];
                }
                None => {
                    out.push_str("\\{");
                    cur = after;
                }
            }
        }
        out.push_str(cur);
        out
    }

    /// Lex + fold a short expression string (for `\{…}` interpolation).
    fn fold_text(&mut self, text: &str) -> Option<i64> {
        let toks = lex_line(text, self.state.cpu, self.source, 0).ok()?;
        self.eval_all(
            &toks,
            Span {
                source: self.source,
                start: 0,
                end: 0,
            },
        )
    }

    /// AS symbol-name composition: a `{expr}` group written OUTSIDE a string
    /// literal and outside a comment is replaced by the expression's value
    /// rendered as text, and the result is pasted into the surrounding
    /// identifier — `zone_id_{cur_str}` with `cur_str := "3"` names `zone_id_3`.
    /// The group may sit anywhere in the name (leading, interior, trailing),
    /// several may appear in one name, and it composes on the DEFINING side too
    /// (`zone_id_{cur_str} = $55` defines `zone_id_3`) (asl-verified).
    ///
    /// A string-valued expression pastes its characters; an integer pastes its
    /// decimal digits. Text inside a `"…"`/`'…'` literal is NOT composed — `dc.b
    /// "brace {cur}"` emits the eight characters `brace {cur}` (asl-verified) —
    /// and a `;` ends the scan, so a brace in a trailing comment is inert. The
    /// literal-skipping rule is the lexer's own (an unescaped closing quote ends
    /// the literal), which also lets the closing `}` be found across a literal
    /// that itself contains one (`{"\{n}"}`).
    ///
    /// Returns `None` when the line needs no rewriting, so the common line pays
    /// only a `{` scan. A group whose expression does not resolve is diagnosed
    /// and left verbatim: it must not silently paste a truncated name.
    fn subst_name_braces(&mut self, line: &SrcLine) -> Option<SrcLine> {
        if !line.text.contains('{') {
            return None;
        }
        let text = line.text.clone();
        let bytes = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut i = 0usize;
        let mut changed = false;
        while i < bytes.len() {
            match bytes[i] {
                b';' => {
                    out.push_str(&text[i..]);
                    break;
                }
                q @ (b'"' | b'\'') => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && bytes[i] != q {
                        i += 1;
                    }
                    i = (i + 1).min(bytes.len());
                    out.push_str(&text[start..i]);
                }
                b'{' => match brace_group_end(bytes, i) {
                    Some(end) => {
                        let inner = text[i + 1..end].to_string();
                        match self.eval_name_brace(&inner, line) {
                            Some(v) => {
                                out.push_str(&v);
                                changed = true;
                            }
                            None => {
                                self.err(
                                    Span {
                                        source: line.source,
                                        start: line.base,
                                        end: line.base,
                                    },
                                    format!("`{{{inner}}}` in a symbol name did not resolve"),
                                );
                                out.push_str(&text[i..=end]);
                            }
                        }
                        i = end + 1;
                    }
                    None => {
                        out.push_str(&text[i..]);
                        break;
                    }
                },
                _ => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && !matches!(bytes[i], b';' | b'"' | b'\'' | b'{') {
                        i += 1;
                    }
                    out.push_str(&text[start..i]);
                }
            }
        }
        changed.then_some(SrcLine {
            text: out,
            base: line.base,
            source: line.source,
        })
    }

    /// Render one `{…}` group's contents as the text AS pastes into the name:
    /// a string expression contributes its characters (with any `\{…}` inside a
    /// literal folded first, so `{"\{n}"}` composes), an integer expression its
    /// decimal digits. `None` when neither shape resolves.
    fn eval_name_brace(&mut self, inner: &str, line: &SrcLine) -> Option<String> {
        let toks = lex_line(inner, self.state.cpu, line.source, line.base).ok()?;
        if toks.is_empty() {
            return None;
        }
        if let Some(s) = self.eval_str(&toks) {
            return Some(self.interp_text(&s));
        }
        let span = Span {
            source: line.source,
            start: line.base,
            end: line.base,
        };
        self.eval_all(&toks, span).map(|v| v.to_string())
    }

    /// `include "path"`: read a file relative to `include_root`, exec its lines
    /// inline. A visited-set prevents re-inclusion (DAG, not tree).
    fn directive_include(&mut self, rest: &[Token], span: Span) {
        let rel = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
            Some(p) => p,
            None => {
                self.err(span, "include needs a quoted path");
                return;
            }
        };
        let path = match &self.include_root {
            Some(root) => root.join(&rel),
            None => std::path::PathBuf::from(&rel),
        };
        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !self.visited.insert(canon) {
            return; // already included (DAG guard)
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                // The included file gets its OWN SourceId, so a diagnostic raised
                // while executing it names that file and its own line number
                // rather than the includer's. `self.source` follows the file being
                // executed and is restored on the way out, so the includer's
                // remaining lines report the includer again.
                let id = self.sources.add_named(path.display().to_string(), text);
                let outer = self.source;
                self.source = id;
                let lines = split_src_lines(self.sources.text(id), id);
                self.exec(&lines);
                self.source = outer;
            }
            Err(e) => self.err(span, format!("cannot include {}: {e}", path.display())),
        }
    }

    /// `BINCLUDE "path"`: read a file's raw bytes and emit them verbatim —
    /// opaque binary data, no parsing (asl-verified: a file containing `ABCD`
    /// emits `41 42 43 44`). Path resolves via `include_root` exactly like
    /// `include` (real Aeon source paths are relative to the aeon root, e.g.
    /// `BINCLUDE "games/sonic4/data/collision/heightmaps.bin"`). Unlike
    /// `include`, this is NOT re-entrancy-guarded by `self.visited` — every
    /// real usage in Aeon is a bare, single-use `BINCLUDE "path"` (no
    /// offset/length args; verified via `grep -rn BINCLUDE` over
    /// `aeon/games` + `aeon/engine`, all 43 call sites bare), and unlike
    /// `include` (which execs the file's lines and so must not re-enter a
    /// cycle), re-BINCLUDEing the same path is a legitimate way to place the
    /// same blob at two different labels — a DAG guard would silently drop
    /// the second copy.
    fn directive_binclude(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        let rel = match rest.iter().find_map(|t| {
            if let Tok::Str(s) = &t.tok {
                Some(s.clone())
            } else {
                None
            }
        }) {
            Some(p) => p,
            None => {
                self.err(span, "BINCLUDE needs a quoted path");
                return;
            }
        };
        let path = match &self.include_root {
            Some(root) => root.join(&rel),
            None => std::path::PathBuf::from(&rel),
        };
        match std::fs::read(&path) {
            Ok(bytes) => self.emit(&bytes, vec![], span),
            Err(e) => self.err(span, format!("cannot BINCLUDE {}: {e}", path.display())),
        }
    }

    /// Execute a slice of logical lines in order, handling block directives.
    fn exec(&mut self, lines: &[SrcLine]) {
        let mut i = 0;
        while i < lines.len() {
            if self.aborted {
                return;
            }
            match self.line_keyword(&lines[i]).as_deref() {
                // A block OPENER may carry a label, and the label is a label:
                // it binds at the PC before the directive runs. `exec_one` is
                // the only path that used to bind one, and these arms bypass
                // it, so each opener binds its own here. `macro`/`struct`/
                // `function` are deliberately absent — those directives CONSUME
                // the name in the label field as the definition's name rather
                // than placing it (asl: `M: macro` leaves `M` out of the symbol
                // table as a location).
                Some("if") | Some("ifdef") | Some("ifndef") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_if(lines, i);
                }
                Some("rept") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_rept(lines, i);
                }
                Some("irp") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_irp(lines, i, IterKind::Groups);
                }
                Some("irpc") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_irp(lines, i, IterKind::Chars);
                }
                Some("while") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_while(lines, i);
                }
                Some("switch") => {
                    self.bind_head_label(&lines[i]);
                    i = self.exec_switch(lines, i);
                }
                Some("struct") => {
                    i = self.capture_struct(lines, i);
                }
                Some("function") => {
                    self.def_function(&lines[i]);
                    i += 1;
                }
                Some("macro") => {
                    i = self.capture_macro(lines, i);
                }
                _ => {
                    self.exec_one(&lines[i]);
                    i += 1;
                }
            }
        }
    }

    fn exec_one(&mut self, line: &SrcLine) {
        // Macro parameters / `ALLARGS` resolve BEFORE `{…}` name composition:
        // a brace group may be written around a parameter, and AS pastes the
        // parameter text first, then evaluates the group.
        // A parked struct label belongs to the line that parked it and to no
        // other. Cleared here so a line that parks one and then dispatches
        // something that is NOT a struct cannot leave it for a later line.
        self.pending_struct_label = None;
        let substituted = self.subst_frame(line);
        let line = substituted.as_ref().unwrap_or(line);
        // `{expr}` groups compose symbol names (see `subst_name_braces`), so they
        // are resolved into plain text before the line is lexed — the lexer
        // swallows a `{…}` group without emitting a token (it is a macro-param
        // attribute there), which would otherwise truncate the composed name.
        let composed = self.subst_name_braces(line);
        let line = composed.as_ref().unwrap_or(line);
        let toks = match lex_line(&line.text, self.state.cpu, line.source, line.base) {
            Ok(t) => t,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        if toks.is_empty() {
            return;
        }
        let parsed = parse_line_tokens(&toks);
        if let Some(name) = parsed.label_colon.clone() {
            // `NAME: = expr` / `NAME: equ expr`: a colon-label immediately
            // followed by an equate directive defines NAME as a CONSTANT, not a
            // PC label — AS tolerates the decorative colon on an equate (Aeon
            // writes both `RESET_RAM: = $FFFFFF00` and
            // `DEBUGGER__EXTENSIONS__ENABLE: equ 1`). Detect it here so we bind
            // the value rather than emitting a stray location label.
            let b = &parsed.tokens;
            let is_eq = matches!(b.first().map(|t| &t.tok), Some(Tok::Punct(Punct::Eq)));
            let is_equ =
                matches!(b.first().map(|t| &t.tok), Some(Tok::Ident(s)) if fold_kw(s) == "equ");
            if (is_eq || is_equ) && b.len() >= 2 {
                let span = b[0].span;
                self.directive_equate(&name, &b[1..], span);
                return;
            }
            // `NAME: set expr` / `NAME: := expr` — a colon-label immediately
            // followed by a REASSIGNABLE-symbol directive binds NAME as a
            // reassignable value, not a PC label (the colon is decorative, exactly
            // as with `NAME: =`/`NAME: equ` above). This is the shape the
            // debugger's `__FSTRING_*` string-scan macros use for their loop
            // cursor: `.__pos: set strstr(...)+.__pos+2` — a `set` that MUST
            // reassign `.__pos` each iteration so the `while (strstr(...)>=0)`
            // guard makes progress and terminates. Treating it as a PC label
            // instead froze `.__pos` at the current address, so the loop never
            // found its end marker (infinite-loop → unbounded label emission).
            // `eval` is asl's processor-neutral spelling of the same directive:
            // on a Z80 `set` is a real instruction (`set 3,a` ⇒ `CB DF`), so the
            // disassemblies reach for `eval` instead. asl accepts both under
            // both CPUs and they name ONE symbol class — `b set 3` followed by
            // `b eval 4` reassigns without complaint, and `a equ 1` followed by
            // `a eval 2` raises the same `#2030 constants cannot be redefined as
            // variables` that `a set 2` does.
            let is_set_kw = matches!(b.first().map(|t| &t.tok),
                Some(Tok::Ident(s)) if matches!(&*fold_kw(s), "set" | "eval"));
            let is_coloneq = matches!(b.first().map(|t| &t.tok), Some(Tok::Punct(Punct::ColonEq)));
            if (is_set_kw || is_coloneq) && b.len() >= 2 {
                let span = b[0].span;
                self.directive_set(&name, &b[1..], span);
                return;
            }
            // `NAME: label expr` — the same decorative-colon shape as the
            // equate forms above, and the corpus writes it
            // (`Obj28_Properties___LABEL__: label *`). Intercepted here so the
            // name is bound ONCE, by the directive, with the directive's value.
            let is_label_kw =
                matches!(b.first().map(|t| &t.tok), Some(Tok::Ident(s)) if fold_kw(s) == "label");
            if is_label_kw && b.len() >= 2 {
                let span = b[0].span;
                self.directive_label(&name, &b[1..], span);
                return;
            }
            // `NAME: mac` where `mac` declares `{INTLABEL}`: the label belongs
            // to the MACRO, which places it (or does not). Defining it here as
            // well would put it at the invocation address on top of wherever the
            // body puts it. asl defines NOTHING for a capture the body drops —
            // `LabA: sup` on a `{INTLABEL}` macro whose body is a bare `nop`
            // leaves `LabA` out of the symbol table entirely, while `LabB:` on
            // an otherwise identical macro without the group lists as `1002 C`.
            if self.head_takes_int_label(b) {
                self.pending_int_label = Some(name);
            } else {
                // A struct instantiation needs the label the members hang off,
                // and `dispatch` is handed only the mnemonic column. Park it
                // for `instantiate_struct` to take, exactly as
                // `pending_int_label` is parked for `expand_macro_inner`. Set
                // BEFORE the label is defined so the ordering matches the
                // colon-less path below, and taken on the very next dispatch.
                self.pending_struct_label = Some(name.clone());
                self.define_label(&name);
            }
        }
        let mut body = parsed.tokens;
        // `!name` builtin escape: a leading `!` resolves `name` against AS's
        // BUILTIN table only, and a user macro of that name is not consulted at
        // all. asl 1.42 Bld 212, `-xx -n -q -A -L -U -i .`:
        //
        // ```text
        //    4/     100 :                     ds macro
        //    5/     100 :                     	!ds.ATTRIBUTE ALLARGS
        //   10/     101 : (MACRO)              	ds.b	4
        //   10/     101 :                             !ds.b 4
        //   11/     105 : 33                  	dc.b	$33
        // ```
        //
        // — a `ds` macro whose own body says `!ds.ATTRIBUTE` expands ONCE and
        // reserves 4 bytes ($101 → $105); the `!` line does not re-enter the
        // macro. The bypass is unconditional, not a fallback: a `!` on a name
        // that is ONLY a user macro is an error, never an invocation
        // (`!mym` on `mym macro` ⇒ `error #1200: unknown instruction MYM`),
        // and so is a `!` on a name that is nothing at all (`!frobnicate` ⇒
        // the same #1200). The `!` must be glued to the name: `! ds.b 3` with
        // a space is `#1200` on an empty mnemonic.
        //
        // Recorded rather than merely stripped, because "strip and dispatch as
        // `name args…`" is wrong exactly where the escape is needed — it is
        // what turned the corpus's own `ds` macro into unbounded self-recursion.
        // (This is unrelated to `!` as the bitwise-or operator — that only ever
        // appears mid-expression, inside an already-consumed head's operand
        // tokens, never as the line's very first token, so there is no ambiguity
        // to resolve.)
        let bang_span = match body.first() {
            Some(Token { tok: Tok::Punct(Punct::Bang), span }) => Some(*span),
            _ => None,
        };
        if bang_span.is_some() {
            body = body[1..].to_vec();
        }
        if body.is_empty() {
            return;
        }
        // A forced-builtin line is a directive/mnemonic line by construction:
        // asl resolves the name in the builtin table and nowhere else, so none
        // of the equate/`set`/`label`/bare-label interpretations below can
        // apply, and the column rule that distinguishes a bare label from an
        // instruction has nothing to decide.
        if let Some(bang) = bang_span {
            // The `!` is a PREFIX of the name, not a separate word: asl reads
            // `! ds.b 3` as an empty mnemonic and reports `#1200`, on the same
            // line where `!ds.b 3` reserves three bytes. Refused here rather
            // than accepted quietly, so sigil does not assemble a spelling asl
            // refuses. Spans are `line.base + column` (see `lex_line`), so
            // adjacency is `bang.end == name.start`.
            if bang.end != body[0].span.start {
                self.err(bang, "`!` must be written against the name it forces");
                return;
            }
            let Tok::Ident(head) = &body[0].tok else {
                self.err(body[0].span, "`!` must be followed by a directive or mnemonic name");
                return;
            };
            let (head, span) = (head.clone(), body[0].span);
            self.dispatch_builtin(&head, &body[1..], span);
            return;
        }
        let head = match &body[0].tok {
            Tok::Ident(s) => s.clone(),
            _ => {
                self.err(body[0].span, "expected mnemonic, directive, or label");
                return;
            }
        };
        if body.len() >= 2 && matches!(body[1].tok, Tok::Punct(Punct::Eq)) {
            self.directive_equate(&head, &body[2..], body[0].span);
            return;
        }
        // `name equ <expr>` — AS's constant-equate keyword form (equivalent to
        // `name = <expr>` for our purposes). Intercepted here, before the
        // mnemonic/bare-label fallback, for the same reason as `=`/`set`: the
        // head is the symbol NAME, not a mnemonic. Without this, `hex equ $80`
        // defines a stray label `hex` then dispatches `equ` as an instruction,
        // and `dec equ $90` is worse still — `dec` IS a Z80 mnemonic, so the
        // whole line routes to instruction lowering and errors under 68000.
        if matches!(body.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if fold_kw(s) == "equ") {
            self.directive_equate(&head, &body[2..], body[0].span);
            return;
        }
        // `name set <expr>` / `name := <expr>` — AS's reassignable-symbol forms
        // (T8). Checked here, before the mnemonic/bare-label fallback below,
        // for the same reason as the `=` equate check just above: the head
        // (`head`) is the accumulator's NAME, not a mnemonic, so without this
        // early intercept a 68000 line like `i set 0` would fall into the
        // bare-label path, define a label `i` at the current PC, and then try
        // to dispatch `set` itself as an instruction (and fail — `set` is
        // only a recognized mnemonic under Z80). `:=` lexes as the single
        // `ColonEq` token (see `token::Punct::ColonEq`), never as `Colon`
        // then `Eq`, so it can never be confused with a `name:` colon-label.
        // `eval` is the same directive under asl's processor-neutral name (see
        // the colon-label arm above): `vcFeedback eval val` in the SMPS include
        // binds `vcFeedback` exactly as `vcFeedback set val` would. It carries
        // no CPU gate because `eval` is not a mnemonic on any supported target.
        //
        // COLUMN-GATED, because without a colon the name must sit in asl's LABEL
        // field. asl reads an INDENTED `\ti\teval 5` as an instruction named `i`
        // (`#1200 unknown instruction`) while the column-0 `i eval 5` assigns,
        // and the decorative-colon spelling `\ti:\teval 5` assigns at any
        // indentation — which is why the colon arm above carries no such gate.
        //
        // Ungated, this arm fires on the OPERAND: `set` and `eval` are ordinary
        // symbol names to asl (`eval` as a label, then `dc.b eval&$ff`, emits
        // its low byte), and an indented `dc.b eval&$ff` presents here as head
        // `dc.b` with `eval` in `body[1]` — assigning a symbol named `dc.b` and
        // emitting nothing at all, silently.
        let name_in_label_field = body[0].span.start == line.base;
        let is_set_kw = name_in_label_field
            && matches!(body.get(1).map(|t| &t.tok),
                Some(Tok::Ident(s)) if matches!(&*fold_kw(s), "set" | "eval"));
        let is_coloneq = matches!(
            body.get(1).map(|t| &t.tok),
            Some(Tok::Punct(Punct::ColonEq))
        );
        if is_set_kw || is_coloneq {
            self.directive_set(&head, &body[2..], body[0].span);
            return;
        }
        // `name label <expr>` — AS's address-symbol form, the colon-less twin of
        // the arm in the colon path above. Same reason for the early intercept as
        // `=`/`equ`/`set`: the head is the symbol NAME, not a mnemonic.
        if matches!(body.get(1).map(|t| &t.tok), Some(Tok::Ident(s)) if fold_kw(s) == "label")
            && body.len() >= 3
        {
            self.directive_label(&head, &body[2..], body[0].span);
            return;
        }
        if !is_op_keyword(&head)
            && !is_mnemonic(&head)
            && !self.macros.contains_key(&head)
            && !self.is_attribute_macro_head(&head)
        {
            // AS's column rule, and it holds on EVERY cpu: a bare label (no
            // colon) sits at column 0; an instruction is indented. A colon
            // label was already stripped above, so any remaining head on such a
            // line is an instruction regardless of column. Head token column =
            // `span.start - line.base` (see lex_line: span.start = base + col).
            //
            // The rule is not a 68000 fallback for the absent m68k mnemonic
            // table — it is what decides whether an UNRECOGNIZED head is a
            // definition or a diagnostic, and that question is cpu-independent.
            // Probed against `asl` (S1's binary, md5 61e67256…) under `CPU Z80`,
            // four shapes:
            //
            //   indented `zqp_bogus`      -> error #1200 unknown instruction
            //   column-0 `zqp_bogus`      -> exit 0; it is a label
            //   indented `zqp_bogus a,b`  -> error #1200, naming the HEAD
            //   indented `ldi`            -> exit 0; a real Z80 instruction
            //
            // Restricting the rule to the 68000 made rows 1, 3 and 4 silent
            // under Z80: the head was bound as a label, no bytes were emitted,
            // and the exit stayed 0 — so an unimplemented Z80 mnemonic (row 4)
            // shortened the output with no diagnostic at all. Applying the rule
            // on every cpu routes all three to `dispatch`, whose final arm
            // reports `unknown directive or mnemonic`.
            {
                let indented = body[0].span.start > line.base;
                if parsed.label_colon.is_some() || indented {
                    self.dispatch(&head, &body[1..], body[0].span);
                    return;
                }
            }
            // The colon-less twin of the `{INTLABEL}` arm above. Both spellings
            // reach the capture (asl: `Tbl outer 3` with no colon binds
            // `__LABEL__` to `Tbl`), and the substituted nested form
            // `__LABEL__ inner aa` arrives here as exactly this shape.
            if self.head_takes_int_label(&body[1..]) {
                self.pending_int_label = Some(head.clone());
            } else {
                // The colon-less twin of the struct-label park above.
                self.pending_struct_label = Some(head.clone());
                self.define_label(&head);
            }
            if body.len() == 1 {
                return;
            }
            let rest = &body[1..];
            let rhead = match &rest[0].tok {
                Tok::Ident(s) => s.clone(),
                _ => {
                    self.err(rest[0].span, "expected mnemonic or directive after label");
                    return;
                }
            };
            self.dispatch(&rhead, &rest[1..], rest[0].span);
            return;
        }
        self.dispatch(&head, &body[1..], body[0].span);
    }

    /// The routing keyword of a line, its index within `body`, and `body` (the
    /// tokens after any colon-label). Rules, in order:
    ///  1. second token is `macro`/`struct`/`function` ⇒ that keyword (a DEFINITION,
    ///     regardless of whether the name is already known — so re-executed
    ///     definition lines route correctly across passes).
    ///  2. the leading name is a known macro ⇒ the name (an INVOCATION; its args,
    ///     even if they look like keywords, are not block openers).
    ///  3. the leading name is a directive/mnemonic ⇒ the name.
    ///  4. a bare label followed by an Ident ⇒ that following Ident (e.g. `Tab db 0`).
    ///  5. otherwise ⇒ the leading name.
    fn dispatch_head(&self, line: &SrcLine) -> Option<(String, usize, Vec<Token>)> {
        self.dispatch_head_checked(line).map(|(k, i, b, _)| (k, i, b))
    }

    /// [`Self::dispatch_head`] plus the lex diagnostic, if the line's OPERAND
    /// did not tokenise. The head is recovered from the tokens that lexed
    /// cleanly before the failure ([`lex_line_recover`]), because the head is
    /// what block-structure scanning routes on and losing it desynchronises
    /// the nesting — see that function's own note. Callers that go on to
    /// EVALUATE the recovered tokens must surface the diagnostic first;
    /// [`Self::line_kw_args_checked`] is the entry point that does.
    fn dispatch_head_checked(
        &self,
        line: &SrcLine,
    ) -> Option<(String, usize, Vec<Token>, Option<Diagnostic>)> {
        let substituted = self.subst_frame_text(&line.text);
        let text = substituted.as_deref().unwrap_or(&line.text);
        let (toks, lex_err) = lex_line_recover(text, self.state.cpu, line.source, line.base);
        let (kw, idx, body) = self.head_of_tokens(toks)?;
        Some((kw, idx, body, lex_err))
    }

    /// The routing keyword within an already-tokenised line. Split out of
    /// [`Self::dispatch_head_checked`] so the head rules below are stated once
    /// and apply identically to a fully-lexed line and to the clean PREFIX of
    /// one whose operand did not lex.
    fn head_of_tokens(&self, toks: Vec<Token>) -> Option<(String, usize, Vec<Token>)> {
        if toks.is_empty() {
            return None;
        }
        let parsed = parse_line_tokens(&toks);
        let body = if parsed.label_colon.is_some() {
            parsed.tokens
        } else {
            toks
        };
        if body.is_empty() {
            return None;
        }
        let name = match &body[0].tok {
            Tok::Ident(s) => s.clone(),
            _ => return None,
        };
        let second = body.get(1).and_then(|t| {
            if let Tok::Ident(s) = &t.tok {
                Some(s.as_str())
            } else {
                None
            }
        });
        // The definition keywords fold, and the FOLDED spelling is what is
        // returned: every consumer of this keyword (`exec`'s block routing,
        // `find_block_end`/`closers_for`, `exec_if`'s arm collection) matches
        // it against lower-case literals.
        let second_kw = second.map(fold_kw);
        if matches!(second_kw.as_deref(), Some("macro" | "struct" | "function")) {
            return Some((second_kw.unwrap().into_owned(), 1, body));
        }
        // A macro INVOCATION returns the macro's name exactly as written: a
        // macro name is a symbol, and folding it here would both mis-resolve
        // the invocation and let a macro named `While`/`If` masquerade as a
        // block opener in `exec`.
        if self.macros.contains_key(&name) {
            return Some((name, 0, body));
        }
        if is_keyword(&name) {
            return Some((fold_kw(&name).into_owned(), 0, body));
        }
        if let Some(Token {
            tok: Tok::Ident(s), ..
        }) = body.get(1)
        {
            // Folded only when it really is a keyword — `Tab DB 0` routes as
            // `db`, while `Tab SomeMacro 0` keeps the macro's own spelling.
            let kw = if is_keyword(s) {
                fold_kw(s).into_owned()
            } else {
                s.clone()
            };
            return Some((kw, 1, body));
        }
        Some((name, 0, body))
    }

    /// The dispatch keyword of a line (after peeling an optional label), or None
    /// for a blank/label-only line. A line whose OPERAND does not lex still has
    /// a keyword and still reports one, because block structure is decided by
    /// the head alone: asl counts a nested `if`/`endif` inside a branch it never
    /// evaluates, and a scan that cannot see the head cuts the block short.
    fn line_keyword(&self, line: &SrcLine) -> Option<String> {
        self.dispatch_head(line).map(|(kw, _, _)| kw)
    }

    /// The name in the LABEL field of a line whose head is a block directive,
    /// or `None` where the line carries no label. A label is a label whatever
    /// keyword follows it: asl binds it at the PC the line sits on, and the
    /// value does not depend on the directive. For `if` in particular the
    /// binding does not depend on the CONDITION either — asl 1.42 Beta Bld 212,
    /// `-cpu 68000 -q -U -L`, the same source with `Rev` 0 then 1:
    ///
    /// ```text
    ///        4/     100 : AA                      dc.b $AA
    ///        5/     101 : =>TRUE               Lab:    if Rev=0     ⇒  Lab : 101 C
    ///
    ///        4/     100 : AA                      dc.b $AA
    ///        5/     101 : =>FALSE              Lab:    if Rev=0     ⇒  Lab : 101 C
    /// ```
    ///
    /// The colon-less spelling obeys AS's column rule, the same one `exec_one`
    /// applies to an unrecognized head: a bare name is a label only at column
    /// 0. Written indented, asl answers `#1200 unknown instruction` and does
    /// not process the directive at all (`  L\tif 1=1` is followed by
    /// `ELSEIF/ENDIF without IF`), so there is nothing to bind.
    fn head_label(&self, line: &SrcLine) -> Option<String> {
        let substituted = self.subst_frame_text(&line.text);
        let text = substituted.as_deref().unwrap_or(&line.text);
        let (toks, _) = lex_line_recover(text, self.state.cpu, line.source, line.base);
        if toks.is_empty() {
            return None;
        }
        if let Some(name) = parse_line_tokens(&toks).label_colon {
            return Some(name);
        }
        // Colon-less: `head_of_tokens` reports the keyword's index within the
        // line's tokens, so an index of 1 is exactly "token 0 is the label
        // field". Spans are `line.base + column` (see `lex_line`), so column 0
        // is `span.start == line.base`.
        let (_, idx, body) = self.head_of_tokens(toks)?;
        if idx != 1 || body[0].span.start != line.base {
            return None;
        }
        match &body[0].tok {
            Tok::Ident(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Bind the label field of a block-head line at the current PC, if it has
    /// one. `define_label` is the same entry point `exec_one` uses, so such a
    /// label is a relocatable section label and becomes the enclosing scope for
    /// the `.local` names that follow it — asl agrees on both counts
    /// (`L: if 1=1` then `.loc: dc.b $CC` lists ` L : 101 C` and
    /// ` L.loc : 102 C`).
    fn bind_head_label(&mut self, line: &SrcLine) {
        if let Some(name) = self.head_label(line) {
            self.define_label(&name);
        }
    }

    /// [`Self::line_kw_args`] for a head whose arguments are about to be
    /// EVALUATED. Structure scanning may read a head recovered from a partly
    /// lexed line and ignore the rest; evaluating one must not, because the
    /// recovered arguments are a truncation of what was written and folding
    /// them would answer a question the source did not ask. The lex diagnostic
    /// is raised and the keyword withheld, so the caller declines the arm
    /// loudly instead of guessing at it.
    fn line_kw_args_checked(&mut self, line: &SrcLine) -> (Option<String>, Vec<Token>, Span) {
        let fallback = Span {
            source: line.source,
            start: line.base,
            end: line.base,
        };
        match self.dispatch_head_checked(line) {
            Some((_, _, _, Some(d))) => {
                self.diags.push(d);
                (None, Vec::new(), fallback)
            }
            Some((kw, idx, body, None)) => {
                let span = body.get(idx).map(|t| t.span).unwrap_or(fallback);
                let args = body.get(idx + 1..).unwrap_or(&[]).to_vec();
                (Some(kw), args, span)
            }
            None => (None, Vec::new(), fallback),
        }
    }

    /// The keyword + the tokens after it + the keyword span, for a block head.
    fn line_kw_args(&self, line: &SrcLine) -> (Option<String>, Vec<Token>, Span) {
        let fallback = Span {
            source: line.source,
            start: line.base,
            end: line.base,
        };
        match self.dispatch_head(line) {
            Some((kw, idx, body)) => {
                let span = body.get(idx).map(|t| t.span).unwrap_or(fallback);
                let args = body.get(idx + 1..).unwrap_or(&[]).to_vec();
                (Some(kw), args, span)
            }
            None => (None, Vec::new(), fallback),
        }
    }

    /// Find the index of the terminator matching the block opened at `start`,
    /// tracking nested blocks with a STACK of expected-closer sets (keyed by
    /// each nested opener's own kind via [`closers_for`]) rather than a flat
    /// depth count keyed on a single caller-supplied opener/closer pair.
    ///
    /// This distinction matters because several DIFFERENT block kinds share
    /// the same literal closer keyword in real AS: `while … endm` AND
    /// `macro … endm` (AND `rept`, which may close with either `endr` or
    /// `endm`) all terminate on `endm`. A flat counter keyed on just the
    /// outer call's own opener (e.g. `capture_macro` passing
    /// `openers=["macro"]`) does NOT increment on a NESTED `while`, so the
    /// nested while's own `endm` was mistaken for the enclosing macro's
    /// `endm` — truncating the macro body before its real end (T9.3
    /// investigation: a `macro` containing a `while … endm` loop, exactly
    /// the shape `__FSTRING_GenerateDecodedString`-style debug macros need,
    /// silently lost its tail and looped forever). The stack fixes this: a
    /// nested opener of ANY kind pushes ITS OWN closer set, so only that
    /// closer set's keyword pops it — regardless of what closer keyword the
    /// enclosing block happens to share with it.
    fn find_block_end(&self, lines: &[SrcLine], start: usize) -> usize {
        let start_kw = self.line_keyword(&lines[start]).unwrap_or_default();
        let mut stack: Vec<&'static [&'static str]> = vec![closers_for(&start_kw)];
        for (idx, line) in lines.iter().enumerate().skip(start + 1) {
            let Some(k) = self.line_keyword(line) else {
                continue;
            };
            let nested_closers = closers_for(&k);
            if !nested_closers.is_empty() {
                // A nested block opener (if/ifdef/ifndef, rept, while, macro,
                // struct, switch) — push its own closer set; only ITS
                // matching closer pops this frame.
                stack.push(nested_closers);
                continue;
            }
            if let Some(top) = stack.last() {
                if top.contains(&k.as_str()) {
                    stack.pop();
                    if stack.is_empty() {
                        return idx;
                    }
                }
            }
        }
        lines.len().saturating_sub(1)
    }

    /// Execute an `if`/`ifdef`/`ifndef` … `endif` region; run the first true arm.
    /// Returns the index just past `endif`.
    fn exec_if(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let end = self.find_block_end(lines, start);
        // Collect arm-head indices at depth 0: start, then each elseif/else.
        let mut heads = vec![start];
        let mut depth = 0i32;
        for (idx, line) in lines.iter().enumerate().take(end).skip(start + 1) {
            match self.line_keyword(line).as_deref() {
                Some("if") | Some("ifdef") | Some("ifndef") => depth += 1,
                Some("endif") => depth -= 1,
                Some("elseif") | Some("else") if depth == 0 => heads.push(idx),
                _ => {}
            }
        }
        heads.push(end); // sentinel
        for w in 0..(heads.len() - 1) {
            let head = heads[w];
            let (kw, argtoks, span) = self.line_kw_args_checked(&lines[head]);
            let take = match kw.as_deref() {
                Some("if") | Some("ifdef") | Some("ifndef") => {
                    self.eval_cond(kw.as_deref().unwrap(), &argtoks, span)
                }
                Some("elseif") => self.eval_if_expr(&argtoks, span),
                Some("else") => true,
                _ => false,
            };
            if take {
                let body = &lines[head + 1..heads[w + 1]];
                self.exec(body);
                // The line that TERMINATES the taken arm — the next
                // `elseif`/`else`, or the closing `endif` — is read while the
                // assembler is still emitting, so its label field binds at the
                // PC the arm ended on. Exactly one such line exists per `if`
                // region, which is why this sits here and not in the head scan:
                // a line closing an arm that was NOT taken is read inside a
                // skipped region and binds nothing. asl, same source with the
                // condition flipped:
                //
                // ```text
                //    5/       1 : =>TRUE               	if 1=1
                //    6/       1 : BB                  	dc.b $BB
                //    7/       2 : [5]                  L:	endif      ⇒ L : 2
                //
                //    5/       1 : =>FALSE              	if 1=0
                //    7/       1 : [5]                  L:	endif      ⇒ absent,
                //                                      `#1: symbol undefined`
                // ```
                //
                // Guarded on the keyword because `find_block_end` falls back to
                // the last line of an UNTERMINATED region, which is a body line
                // the arm never reached rather than a closer.
                let closer = &lines[heads[w + 1]];
                if matches!(
                    self.line_keyword(closer).as_deref(),
                    Some("elseif" | "else" | "endif")
                ) {
                    self.bind_head_label(&lines[heads[w + 1]]);
                }
                break;
            }
        }
        end + 1
    }

    /// Handle `switch <str-expr> / case "s1" / … / elsecase / … / endcase`
    /// (T9.3, asl-verified): assembles ONLY the body of the first `case`
    /// whose literal string equals the switch value; `elsecase` is the
    /// default (chosen when reached, mirroring `exec_if`'s `else` arm — same
    /// arm-collection shape as `exec_if`, but keyed on STRING equality
    /// against each `case`'s literal instead of a boolean condition). The
    /// switch expression and each `case` literal are evaluated through
    /// `eval_str` (so `switch lowstring(...)` / nested `substr` all compose,
    /// exactly as the debugger's `__FSTRING_*` macros use them). An
    /// unresolved switch expression, or a `case` whose argument isn't a
    /// string, diagnoses but does not abort the block scan. Returns the
    /// index past `endcase`.
    fn exec_switch(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let end = self.find_block_end(lines, start);
        let switch_val = self.eval_str(&arg_toks);
        if switch_val.is_none() {
            self.err(span, "switch needs a string expression");
        }
        // Collect arm-head indices at depth 0: each `case "lit"` (Some(lit))
        // and `elsecase` (None, the default), mirroring `exec_if`'s
        // if/elseif/else head collection but depth-counting `switch`/`endcase`
        // instead of `if`/`endif`.
        let mut heads: Vec<(usize, Option<String>)> = Vec::new();
        let mut depth = 0i32;
        for (idx, line) in lines.iter().enumerate().take(end).skip(start + 1) {
            match self.line_keyword(line).as_deref() {
                Some("switch") => depth += 1,
                Some("endcase") => depth -= 1,
                Some("case") if depth == 0 => {
                    let (_, cargs, cspan) = self.line_kw_args(line);
                    let lit = self.eval_str(&cargs);
                    if lit.is_none() {
                        self.err(cspan, "case needs a string literal");
                    }
                    heads.push((idx, lit));
                }
                Some("elsecase") if depth == 0 => heads.push((idx, None)),
                _ => {}
            }
        }
        heads.push((end, None)); // sentinel
        for w in 0..(heads.len() - 1) {
            let (head, lit) = heads[w].clone();
            let take = match &lit {
                Some(s) => switch_val.as_deref() == Some(s.as_str()),
                None => true, // elsecase: default, taken if reached
            };
            if take {
                let body = &lines[head + 1..heads[w + 1].0];
                self.exec(body);
                break;
            }
        }
        end + 1
    }

    /// Handle `rept N … endr`. `N` is folded once at the `rept` line (with `$` =
    /// the current phased VMA). Returns the index past `endr`.
    fn exec_rept(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let n = match self.eval_all(&arg_toks, span) {
            Some(v) if v >= 0 => v as usize,
            Some(_) => {
                self.err(span, "negative rept count");
                0
            }
            None => {
                self.err(span, "unresolved rept count");
                0
            }
        };
        let end = self.find_block_end(lines, start);
        let captured = self.capture_loop_body(&lines[start + 1..end]);
        let body: &[SrcLine] = captured.as_deref().unwrap_or(&lines[start + 1..end]);
        for _ in 0..n {
            self.exec(body);
        }
        self.release_loop_body(captured.is_some());
        end + 1
    }

    /// Handle `irp NAME,<items> … endm` and `irpc NAME,<string> … endm`: run
    /// the body once per item, substituting `NAME`'s text into it.
    ///
    /// **The two differ only in where the item list comes from.**
    ///
    /// `irp`'s items are the operand's top-level comma groups as RAW TEXT, never
    /// evaluated (asl-verified, probe `p8.asm` case 8b — `irp v,1+2,$FF` over
    /// `dc.b "[v]"` emits `[1+2]` and `[$FF]`, not `[3]` and `[255]`), with
    /// surrounding whitespace dropped. A comma inside a quoted item does not
    /// split it (case 7c: `irp v,"a,b","c"` is two items).
    ///
    /// `irpc`'s operand is a STRING EXPRESSION, evaluated once, and the items
    /// are its characters — spaces included. It is not the literal text: a `set`
    /// symbol resolves (case 6d), escapes are decoded (`"A\x5AB"` is three
    /// characters A, Z, B — case 7d), and an INTEGER result is rendered in
    /// decimal and then walked digit by digit (case 8a: `irpc c,65` is `6` then
    /// `5`, `irpc c,1+2` is one iteration of `3`). An operand that resolves to
    /// nothing at all is an error and runs ZERO iterations (case 6e).
    ///
    /// **An EMPTY list is one EMPTY iteration, not none.** Both spellings:
    /// `irp v,` runs the body once with `v` empty (case 6a), and `irpc c,""`
    /// does the same (case 7a, `dc.b "<c>"` → `dc.b "<>"`). This is why
    /// `s2.macrosetup.asm(301)` guards its `irp op,ALLARGS` with `if ARGCOUNT>0`
    /// — without the guard an empty `jmpTos` would define a nameless label. A
    /// missing comma entirely (`irp v`) is a different thing and is asl's
    /// error #1110, with the body skipped (cases 9c/9d).
    ///
    /// Substitution is textual, case-sensitive, and obeys the macro-parameter
    /// boundary rule: `"c"` and `_c_` take the value while `xcx` does not
    /// (case 6g). It happens on top of the enclosing expansion's substitution,
    /// which — exactly as for `rept`/`while` — is applied ONCE where the loop is
    /// entered and then suspended, so a `shift` in the body advances the frame
    /// without changing the body's own text:
    ///
    /// ```text
    ///   35/ 1021 : 7031 01     dc.b "p1",1
    ///   35/ 1024 : 7031 02     dc.b "p1",2
    ///   35/ 1027 : 7031 03     dc.b "p1",3
    /// ```
    ///
    /// (probe `p8.asm` case 8d — `sh macro aa` called `sh p1,p2,p3`, the body
    /// shifting each iteration; `aa` stays `p1` throughout, and the frame HAS
    /// advanced by the time the line after the loop reads it.)
    ///
    /// Returns the index past the closer.
    fn exec_irp(&mut self, lines: &[SrcLine], start: usize, kind: IterKind) -> usize {
        let (_, arg_toks, span) = self.line_kw_args_checked(&lines[start]);
        let end = self.find_block_end(lines, start);
        // The head's own text, for `irp`'s RAW-TEXT items. Recomputed rather
        // than threaded out of `line_kw_args_checked` because `subst_frame_text`
        // is pure and this is the identical call it already made — the token
        // spans below index into exactly this string.
        let head_text = self
            .subst_frame_text(&lines[start].text)
            .unwrap_or_else(|| lines[start].text.clone());
        let items = self.irp_items(&arg_toks, span, kind, &head_text, lines[start].base);
        let Some((name, items)) = items else {
            return end + 1;
        };
        let captured = self.capture_loop_body(&lines[start + 1..end]);
        let body: Vec<SrcLine> = captured
            .clone()
            .unwrap_or_else(|| lines[start + 1..end].to_vec());
        for item in &items {
            if self.aborted {
                break;
            }
            let iter: Vec<SrcLine> = body
                .iter()
                .map(|l| SrcLine {
                    text: substitute_name(&l.text, &name, item),
                    base: l.base,
                    source: l.source,
                })
                .collect();
            self.exec(&iter);
        }
        self.release_loop_body(captured.is_some());
        end + 1
    }

    /// The loop variable name and the item texts for an `irp`/`irpc` head, or
    /// `None` (with the diagnostic already raised) when the head cannot supply
    /// them. Split out of [`Self::exec_irp`] so the block is still SKIPPED as a
    /// block on the error path — asl reports #1110 and steps over the body
    /// rather than executing it (probe `p9.asm` cases 9c/9d).
    fn irp_items(
        &mut self,
        arg_toks: &[Token],
        span: Span,
        kind: IterKind,
        head_text: &str,
        base: u32,
    ) -> Option<(String, Vec<String>)> {
        let kw = match kind {
            IterKind::Groups => "irp",
            IterKind::Chars => "irpc",
        };
        // The name is the first comma group and must be a lone identifier; the
        // list is everything after that first comma. Finding the comma by index
        // (rather than reusing the split) keeps `irpc`'s operand as ONE token run
        // so a comma inside its string expression stays inside it.
        let comma = arg_toks.iter().position(|t| {
            matches!(t.tok, Tok::Punct(Punct::Comma))
        });
        let Some(comma) = comma else {
            self.err(span, format!("`{kw}` needs a loop variable and a list, separated by a comma"));
            return None;
        };
        let name = match arg_toks.get(..comma) {
            Some([Token { tok: Tok::Ident(s), .. }]) => s.clone(),
            _ => {
                self.err(span, format!("`{kw}` needs a single identifier as its loop variable"));
                return None;
            }
        };
        let rest = &arg_toks[comma + 1..];
        let items = match kind {
            IterKind::Groups => split_top_commas(rest)
                .into_iter()
                .map(|g| slice_source(head_text, base, g))
                .collect(),
            IterKind::Chars => {
                // asl evaluates the operand: a string expression first, then a
                // numeric one rendered in decimal. An EMPTY operand field folds
                // to 0 and therefore iterates once over the character `0` — the
                // one place where "empty" is not the empty string (probe
                // `p7.asm` case 7b, `irpc c,` over `dc.b "<c>"` → `dc.b "<0>"`).
                let s = if rest.is_empty() {
                    "0".to_string()
                } else {
                    match self.eval_str(rest) {
                    Some(s) => s,
                    None => match self.eval_all(rest, span) {
                        Some(v) => v.to_string(),
                        None => {
                            self.err(span, "unresolved `irpc` string expression");
                            return None;
                        }
                    },
                    }
                };
                if s.is_empty() {
                    vec![String::new()]
                } else {
                    s.chars().map(|c| c.to_string()).collect()
                }
            }
        };
        Some((name, items))
    }

    /// Materialize a `rept`/`while` body against the innermost expansion and
    /// suspend that expansion's substitution for the replay, matching AS: the
    /// loop body is substituted ONCE where the loop is entered, and a `shift`
    /// inside it advances the frame without changing the body's own text
    /// (asl-verified — see [`MacroFrame::suspend`]). Returns `None` when there
    /// is nothing to substitute, so the caller replays the source lines
    /// directly.
    fn capture_loop_body(&mut self, body: &[SrcLine]) -> Option<Vec<SrcLine>> {
        if !self.frame_substitutes() {
            return None;
        }
        let captured: Vec<SrcLine> = body
            .iter()
            .map(|l| self.subst_frame(l).unwrap_or_else(|| l.clone()))
            .collect();
        if let Some(f) = self.macro_frames.last_mut() {
            f.suspend += 1;
        }
        Some(captured)
    }

    /// Undo [`Self::capture_loop_body`]'s suspension. `captured` is whether that
    /// call actually suspended one.
    fn release_loop_body(&mut self, captured: bool) {
        if captured {
            if let Some(f) = self.macro_frames.last_mut() {
                f.suspend -= 1;
            }
        }
    }

    /// Handle `while (cond) … endm` (T9.2, asl-verified — NOT `endw`: asl
    /// errors "WHILE without ENDM"). Unlike `rept`'s once-folded count, the
    /// condition is a live expression re-evaluated every iteration (typically
    /// against a `set` accumulator advanced in the body), so this can't fold
    /// it once up front the way `exec_rept` does. Bounded by `WHILE_CAP`
    /// with a non-convergence diagnostic (A5, `SIGIL_CORE_SPEC.md` §7.1/§10.4
    /// — the same bounded-loop-or-diagnose contract as the pass loop
    /// (`PASS_CAP`) and macro expansion (`EXPAND_CAP`)) so a condition that
    /// can never resolve to zero can't hang the assembler. Returns the index
    /// past `endm`.
    fn exec_while(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let (_, arg_toks, span) = self.line_kw_args(&lines[start]);
        let end = self.find_block_end(lines, start);
        let captured = self.capture_loop_body(&lines[start + 1..end]);
        let body: &[SrcLine] = captured.as_deref().unwrap_or(&lines[start + 1..end]);
        let mut iterations = 0usize;
        loop {
            if self.aborted {
                break;
            }
            match self.eval_all(&arg_toks, span) {
                Some(0) => break,
                Some(_) => {
                    if iterations >= WHILE_CAP {
                        self.err(
                            span,
                            format!("while loop did not terminate within {WHILE_CAP} iterations (non-convergent condition?)"),
                        );
                        break;
                    }
                    if self.while_budget == 0 {
                        self.err(
                            span,
                            format!("total `while` iterations exceeded the per-pass budget ({GLOBAL_WHILE_CAP}) — a non-convergent (possibly nested) loop"),
                        );
                        self.aborted = true;
                        break;
                    }
                    self.while_budget -= 1;
                    self.exec(body);
                    iterations += 1;
                }
                None => {
                    self.err(span, "unresolved while condition");
                    break;
                }
            }
        }
        self.release_loop_body(captured.is_some());
        end + 1
    }

    /// Handle name-first `Name struct … Name endstruct`: define packed
    /// `Name_field` offsets and `Name_len`. Field lines emit no bytes. Returns the
    /// index past `endstruct`. (Mirrors `capture_macro`: name at `toks[0]`,
    /// `struct` at `toks[1]`.)
    fn capture_struct(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let head = self.subst_frame(&lines[start]);
        let head = head.as_ref().unwrap_or(&lines[start]);
        let toks = lex_line(
            &head.text,
            self.state.cpu,
            lines[start].source,
            lines[start].base,
        )
        .unwrap_or_default();
        let span = toks.first().map(|t| t.span).unwrap_or(Span {
            source: lines[start].source,
            start: lines[start].base,
            end: lines[start].base,
        });
        let name = match toks.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            _ => {
                self.err(span, "struct needs a name");
                String::new()
            }
        };
        // `NAME struct [MODIFIER…]`. The one modifier that changes a NAME is
        // `DOTS`, which makes `.` the separator between the struct and its
        // members instead of `_` (probe `q8.asm`). Recognised case-folded:
        // Sonic 1 writes `struct DOTS`, Sonic 2 writes both `STRUCT DOTS` and
        // `struct dots`. Any other modifier asl accepts (EXTNAMES, …) is
        // ignored here rather than refused — none of them moves an offset, and
        // neither corpus writes one.
        let dots = toks[1..]
            .iter()
            .any(|t| matches!(&t.tok, Tok::Ident(s) if fold_kw(s) == "dots"));
        let sep = if dots { '.' } else { '_' };
        let end = self.find_block_end(lines, start);
        let mut off: i64 = 0;
        let mut elems: Vec<(String, i64)> = Vec::new();
        // Every member symbol is bound into `env` AS THE BODY IS WALKED, not
        // afterwards: Sonic 1's `SMPS_RAM` sizes its own last field from two of
        // its own earlier markers (`ds.b SMPS_RAM.v_1up_ram_end-SMPS_RAM.v_1up_ram`),
        // so a member's count expression must be able to read the members above it.
        for l in &lines[start + 1..end] {
            match self.parse_struct_member(l) {
                Some(StructMember::Field { name: field, width, count }) => {
                    // The declaration-scope symbol takes the offset BEFORE this
                    // field's alignment pad; the element takes it after. See
                    // [`StructDef`] for the asl listing the two are read off.
                    let pre = off;
                    // asl's `padding` flag, inside a struct exactly as outside
                    // it: with `padding on` (asl's default) a `ds.w`/`ds.l`
                    // field starting at an odd offset is preceded by one pad
                    // byte. With `padding off` — which Aeon sets globally at the
                    // top of `main.asm` — there is no rounding at all.
                    let start_off = if self.state.padding && width >= 2 && off % 2 != 0 {
                        off + 1
                    } else {
                        off
                    };
                    off = start_off + width * count;
                    // An anonymous reserve field (`ds.b 1` with no name)
                    // advances the offset but defines no member symbol.
                    if !field.is_empty() {
                        self.define_struct_member(&name, sep, &field, pre, span);
                        elems.push((field, start_off));
                    }
                }
                // A marker binds the running offset and reserves nothing, so
                // its two tables cannot disagree.
                Some(StructMember::Marker { name: field }) => {
                    self.define_struct_member(&name, sep, &field, off, span);
                    elems.push((field, off));
                }
                // An embedded struct is placed VERBATIM at the running offset —
                // it is never re-aligned, and its own internal padding is not
                // recomputed against the parent's parity (probe `q10.asm`: a
                // 2-aligned inner `r` lands at the ODD parent offset 3). Its
                // whole element table is flattened in under the member name,
                // which is what makes `SMPS_RAM.v_music_dac_track.PlaybackControl`
                // a name at all.
                Some(StructMember::Embed { name: field, struct_name }) => {
                    let Some(inner) = self.structs.get(&struct_name).cloned() else {
                        continue;
                    };
                    if !field.is_empty() {
                        self.define_struct_member(&name, sep, &field, off, span);
                        elems.push((field.clone(), off));
                        for (m, o) in &inner.elems {
                            let path = format!("{field}{sep}{m}");
                            self.define_struct_member(&name, sep, &path, off + o, span);
                            elems.push((path, off + o));
                        }
                    }
                    off += inner.len;
                }
                // The lexer-refused case has already said something specific;
                // this adds the consequence, which is the part that matters —
                // an unread member line is a wrong struct SIZE.
                Some(StructMember::Unreadable) => {
                    let s = Span { source: l.source, start: l.base, end: l.base };
                    self.err(s, format!("struct `{name}` has a member line this cannot read; its size and every member after it would be wrong"));
                }
                None => {}
            }
        }
        self.define_struct_member(&name, sep, "len", off, span);
        self.structs.insert(name, StructDef { sep, len: off, elems });
        end + 1
    }

    /// `label: NAME` — place one instance of struct `NAME` at the current PC.
    ///
    /// Reserves `NAME<sep>len` bytes and hangs every element off `label`.
    /// Sonic 1's `_Variables.asm(114)` is the whole reason this row matters:
    /// `v_snddriver_ram: SMPS_RAM` is $5C0 bytes inside a `phase`d RAM map, so
    /// getting the size wrong moves every variable declared after it — which
    /// the disassembly then reports on itself at `_Variables.asm(430)`.
    ///
    /// The reservation is `ds.b len`, NOT `ds.w`/`ds.l`: an instance is placed
    /// verbatim and is never word-aligned, even under `padding on` and even
    /// when the struct's first member is a `ds.w`. Probe `q9.asm` puts a
    /// word-leading struct at an odd `org $2001` and asl leaves it there, while
    /// the bare `ds.w 1` two lines down pads to $3002.
    fn instantiate_struct(&mut self, struct_name: &str, span: Span) {
        let Some(def) = self.structs.get(struct_name).cloned() else {
            return;
        };
        // asl `#2040 structure name missing` — an unlabelled instantiation
        // reserves nothing at all (probe `q8.asm`: the PC does not move).
        let Some(label) = self.pending_struct_label.take() else {
            self.err(span, "structure name missing");
            return;
        };
        self.open_section_if_needed();
        let base = self.here_i64();
        for (member, off) in &def.elems {
            let full = format!("{label}{}{member}", def.sep);
            self.env.define(&full, SymbolValue::Int(base + off));
            self.known_labels.insert(full.clone());
            self.export_equ_sym(full, base + off, span);
        }
        if def.len > 0 {
            self.builder.reserve(def.len as u32, span);
        }
    }

    /// Bind `NAME<sep>MEMBER` to `value`, in `env` and on the link-level equate
    /// seam both. Struct member offsets are int equates semantically, so they
    /// export on the same Item-B seam `equ` uses (tranche 3: `.emp` drift guards
    /// read `extern("VDP_Shadow_len")`).
    fn define_struct_member(&mut self, name: &str, sep: char, member: &str, value: i64, span: Span) {
        let full = format!("{name}{sep}{member}");
        self.env.define(&full, SymbolValue::Int(value));
        self.export_equ_sym(full, value, span);
    }

    /// Export an int equate to the module's link-level `equ_syms` so `.emp`
    /// code can read it via `extern()` (Task B1, seam re-eval; extended to
    /// struct-generated symbols in tranche 3). `add_equ_sym` panics with no
    /// open section, and an equate CAN occur before any section has opened —
    /// see `directive_equate`'s inline comment for why eagerly opening a
    /// section here is WRONG (`org`/`phase` rely on possibly-section-free
    /// evaluation): attach to the currently open section if one exists,
    /// otherwise stash in `pending_equ_syms` for the next section that opens.
    fn export_equ_sym(&mut self, name: String, value: i64, span: Span) {
        if self.in_section {
            self.builder.add_equ_sym(EquSym { name, expr: Expr::Int(value), span });
        } else {
            self.pending_equ_syms.push(EquSym { name, expr: Expr::Int(value), span });
        }
    }

    /// Parse one line of a struct body into the member it declares, or `None`
    /// for a blank/comment line and for anything this does not model.
    ///
    /// Three shapes, all of which both corpora write:
    ///  - `[name:] ds.b|ds.w|ds.l <count>` — a reserve field;
    ///  - `[name:] <struct name>` — an embedded struct (Sonic 1's `SMPS_RAM`
    ///    embeds `SMPS_Track` 17 times);
    ///  - `name:` alone — a marker, which reserves nothing.
    ///
    /// The `ds.*` head is matched LITERALLY rather than dispatched, and that is
    /// deliberate: both corpora define a `ds` MACRO (`MacroSetup.asm:86`) that
    /// beats the builtin everywhere else, whose 68000 arm is `!ds.ATTRIBUTE
    /// ALLARGS` and whose Z80 arm emits `db 0` bytes. Reading the width off the
    /// written token gives the same offsets on the 68000 path without routing a
    /// struct body through macro expansion.
    fn parse_struct_member(&mut self, line: &SrcLine) -> Option<StructMember> {
        let Some((name, width, count)) = self.parse_struct_field(line) else {
            // Blank and comment-only lines lex to nothing and are genuinely
            // not members; anything else that got here has a label column this
            // cannot read, and that is [`StructMember::Unreadable`].
            //
            // A line the LEXER refuses counts as unreadable too, and its own
            // diagnostic is surfaced rather than replaced: `1upPlaying:` fails
            // as a malformed number, which names the real problem far better
            // than anything this could say. Dropping it — which is what a bare
            // `.ok()?` did — is how the same line silently shortened Sonic 2's
            // `zVar` by a byte.
            let substituted = self.subst_frame(line);
            let l = substituted.as_ref().unwrap_or(line);
            match lex_line(&l.text, self.state.cpu, l.source, l.base) {
                Ok(toks) => return (!toks.is_empty()).then_some(StructMember::Unreadable),
                Err(d) => {
                    self.diags.push(d);
                    return Some(StructMember::Unreadable);
                }
            }
        };
        match width {
            // `parse_struct_field` reports width 0 for a head it could not read
            // as `ds.*`: either a bare label or an embedded struct.
            0 => {
                if name.is_empty() {
                    return None;
                }
                match self.struct_embed_name(line) {
                    Some(struct_name) => Some(StructMember::Embed { name, struct_name }),
                    None => Some(StructMember::Marker { name }),
                }
            }
            _ => Some(StructMember::Field { name, width, count }),
        }
    }

    /// The struct named in the mnemonic column of a struct-body line, if the
    /// line embeds one. `None` when the mnemonic column is empty (a marker) or
    /// names something that is not a declared struct.
    fn struct_embed_name(&mut self, line: &SrcLine) -> Option<String> {
        let substituted = self.subst_frame(line);
        let line = substituted.as_ref().unwrap_or(line);
        let toks = lex_line(&line.text, self.state.cpu, line.source, line.base).ok()?;
        let parsed = parse_line_tokens(&toks);
        let head = if parsed.label_colon.is_some() {
            parsed.tokens.first()
        } else {
            // No colon: the label is the first token and the struct name, if
            // any, is the second.
            parsed.tokens.get(1)
        };
        match head.map(|t| &t.tok) {
            Some(Tok::Ident(s)) if self.structs.contains_key(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Parse a `<field> ds.b|ds.w|ds.l <count>` struct-member line.
    /// Returns `(field, width, count)`, or None for a blank/comment line.
    /// A width of `0` means the line named no `ds.*` — the caller decides
    /// whether that is a marker or an embedded struct.
    fn parse_struct_field(&mut self, line: &SrcLine) -> Option<(String, i64, i64)> {
        let substituted = self.subst_frame(line);
        let line = substituted.as_ref().unwrap_or(line);
        let toks = lex_line(&line.text, self.state.cpu, line.source, line.base).ok()?;
        if toks.is_empty() {
            return None;
        }
        let parsed = parse_line_tokens(&toks);
        let (field, rest): (String, Vec<Token>) = if let Some(l) = parsed.label_colon {
            (l, parsed.tokens)
        } else {
            match parsed.tokens.split_first() {
                // Anonymous reserve field: a bare `ds.b|ds.w|ds.l N` with no
                // preceding name (e.g. Act's `ds.b 1 ; reserved (pad to word)`).
                // AS still advances the struct offset by its size; it just binds
                // no member symbol. Emit an empty field name and keep the whole
                // token slice (the `ds.*` keyword is the width token).
                Some((
                    Token {
                        tok: Tok::Ident(s), ..
                    },
                    _,
                )) if matches!(fold_kw(s).as_ref(), "ds.b" | "ds.w" | "ds.l") => {
                    (String::new(), parsed.tokens.clone())
                }
                Some((
                    Token {
                        tok: Tok::Ident(s), ..
                    },
                    r,
                )) => (s.clone(), r.to_vec()),
                _ => return None,
            }
        };
        // Width `0` = "the mnemonic column is not a `ds.*`". The line still
        // carries a NAME, and that name is a member either way: a struct-body
        // line with a label and nothing else is a marker, and one whose
        // mnemonic column names another struct embeds it. Returning `None` here
        // — as this did before markers and embedding were modelled — silently
        // dropped 21 of Sonic 1's `SMPS_RAM` members and all 17 of its embedded
        // tracks, which is a wrong SIZE and not merely a missing symbol.
        let width = match rest.first().map(|t| &t.tok) {
            Some(Tok::Ident(w)) => match fold_kw(w).as_ref() {
                "ds.b" => 1,
                "ds.w" => 2,
                "ds.l" => 4,
                _ => return Some((field, 0, 0)),
            },
            _ => return Some((field, 0, 0)),
        };
        let span = rest[0].span;
        let count = self.eval_all(&rest[1..], span).unwrap_or(1);
        Some((field, width, count))
    }

    fn eval_cond(&mut self, kw: &str, arg_toks: &[Token], span: Span) -> bool {
        match kw {
            "ifdef" => self.cond_defined(arg_toks),
            "ifndef" => !self.cond_defined(arg_toks),
            _ => self.eval_if_expr(arg_toks, span),
        }
    }

    fn cond_defined(&self, arg_toks: &[Token]) -> bool {
        matches!(arg_toks.first().map(|t| &t.tok), Some(Tok::Ident(n)) if self.env.resolve(n, self.dot_scope(n)).is_some())
    }

    /// `if MOMCPUNAME="Z80"` / `<lhs>="str"` / `"a"="a"` / `"a"<>"b"` string
    /// (in)equality, else numeric `!= 0`. Strings never enter `sigil_ir::Expr`
    /// (§7.4: no AS-specific concept in IR) — the shape is detected and folded
    /// to a bool directly here, before any numeric `Expr` is built.
    fn eval_if_expr(&mut self, toks: &[Token], span: Span) -> bool {
        if let Some(pos) = toks
            .iter()
            .position(|t| matches!(t.tok, Tok::Punct(Punct::Eq) | Tok::Punct(Punct::Ne)))
        {
            if let Some(Token {
                tok: Tok::Str(rhs), ..
            }) = toks.get(pos + 1)
            {
                let lhs = match &toks[..pos] {
                    [Token {
                        tok: Tok::Str(s), ..
                    }] => Some(s.clone()),
                    other => self.string_value(other),
                };
                if let Some(lhs) = lhs {
                    let eq = lhs == *rhs;
                    let is_ne = matches!(toks[pos].tok, Tok::Punct(Punct::Ne));
                    return if is_ne { !eq } else { eq };
                }
            }
        }
        self.eval_all(toks, span).map(|v| v != 0).unwrap_or(false)
    }

    /// The string value of a builtin like MOMCPUNAME (else None).
    fn string_value(&self, toks: &[Token]) -> Option<String> {
        match toks {
            [Token {
                tok: Tok::Ident(n), ..
            }] if n == "MOMCPUNAME" => Some(match self.state.cpu {
                Cpu::Z80 => "Z80".into(),
                Cpu::M68000 => "68000".into(),
            }),
            _ => None,
        }
    }

    /// Whether `head` names a `.ATTRIBUTE`-suffix invocation of a captured
    /// macro (T9.2): `head` itself isn't a known macro, but stripping a
    /// trailing `.b`/`.w`/`.l`/`.s` yields one that is. Checked before the
    /// M68000 bare-label/mnemonic-column heuristic in `exec_one` (so
    /// `foo.w d1` — a macro invocation — dispatches, rather than being
    /// mistaken for a label) and drives `dispatch`'s own attribute-macro
    /// arm below.
    fn is_attribute_macro_head(&self, head: &str) -> bool {
        !self.macros.contains_key(head)
            && split_attribute_suffix(head).is_some_and(|(base, _)| self.macros.contains_key(base))
    }

    /// Whether the first token of `toks` names a macro whose parameter list
    /// carries `{INTLABEL}` — i.e. whether a label written on this line is the
    /// macro's to place rather than a location label at the invocation address.
    ///
    /// The `.ATTRIBUTE`-suffix spelling counts: the capture is a property of the
    /// definition, and a suffixed call expands the same body.
    fn head_takes_int_label(&self, toks: &[Token]) -> bool {
        let Some(Token { tok: Tok::Ident(name), .. }) = toks.first() else {
            return false;
        };
        let def = self.macros.get(name).or_else(|| {
            split_attribute_suffix(name).and_then(|(base, _)| self.macros.get(base))
        });
        def.is_some_and(|m| m.int_label)
    }

    /// AS's `label` directive: bind `name` to an expression, as an ADDRESS.
    ///
    /// It is neither `equ` nor a plain label. Its VALUE is any expression
    /// (`A label *`, `B label $2000`, `C: label *+4` list as `1000`, `2000`,
    /// `1004`), and unlike `equ` the symbol carries the code segment — every one
    /// of those rows ends `C` in asl's table, where a `:=` row ends `-`. And
    /// unlike a plain label inside a macro body it is EXPORTED to the caller: it
    /// is the whole reason the corpus's `{INTLABEL}` macros can define the
    /// caller's table head from inside their own expansion.
    ///
    /// It also OPENS the scope it names, in the caller — `Table label *` inside
    /// an expansion makes a later `.cnt` read back as `Table.cnt` at top level.
    /// Where the expression is the current PC, which is every corpus and every
    /// probe use (`label *` on 68000, `label $` on Z80), that is exactly
    /// [`Self::define_label`] and it is reused whole, so the symbol relocates
    /// with its section like any other label. A value that is NOT the PC cannot
    /// be a placed label, so it binds as a scope-opening constant.
    fn directive_label(&mut self, name: &str, rest: &[Token], span: Span) {
        let Some(v) = self.eval_all(rest, span) else {
            self.err(span, "label needs a value");
            return;
        };
        self.open_section_if_needed();
        // A `.`-local `label` qualifies against the CALLER's real scope, not the
        // expansion's — it is a value-binding form, and those all land in the
        // caller (see [`Self::real_scope`]). A global one OPENS its scope there.
        let qualified = if name.starts_with('.') {
            qualify(name, self.real_scope())
        } else {
            self.open_scope(name);
            name.to_string()
        };
        self.env.define(&qualified, SymbolValue::Int(v));
        self.known_labels.insert(qualified.clone());
        // Only a PC-valued `label` is a PLACED label the linker can relocate.
        // Any other value is a constant that happens to be typed as an address,
        // and handing it to the builder would claim a position for it here.
        if v == self.here_i64() {
            self.builder.define_label(&qualified);
        }
    }

    /// Open `name` as the local-label scope, in the CALLER's frame of reference.
    ///
    /// Inside an expansion the real scope is [`Self::outer_scope`] — `self.scope`
    /// holds the expansion's own unspellable name — so writing `self.scope` there
    /// would be undone the moment the expansion returns.
    fn open_scope(&mut self, name: &str) {
        if self.macro_frames.is_empty() {
            self.scope = Some(name.to_string());
        } else {
            self.outer_scope = Some(name.to_string());
        }
    }

    fn dispatch(&mut self, head: &str, rest: &[Token], span: Span) {
        if let Some((base, suffix)) = split_attribute_suffix(head) {
            if !self.macros.contains_key(head) && self.macros.contains_key(base) {
                self.expand_macro_with_attribute(base, rest, suffix);
                return;
            }
        }
        self.dispatch_resolved(head, rest, span, false);
    }

    /// `!name args…` — the [forced-builtin escape](Self::exec_one). Same body
    /// as [`Self::dispatch`] with every macro consultation removed: neither the
    /// `.ATTRIBUTE`-suffix expansion above nor the invocation arm below is
    /// reachable, so a name that is only a macro falls through to the
    /// mnemonic/unknown arms and is reported, which is asl's `#1200`.
    fn dispatch_builtin(&mut self, head: &str, rest: &[Token], span: Span) {
        self.dispatch_resolved(head, rest, span, true);
    }

    /// The shared body of [`Self::dispatch`] and [`Self::dispatch_builtin`].
    /// `forced_builtin` suppresses the one macro-invocation arm below; the
    /// `.ATTRIBUTE` arm is in `dispatch` alone and so is already absent here.
    fn dispatch_resolved(
        &mut self,
        head: &str,
        rest: &[Token],
        span: Span,
        forced_builtin: bool,
    ) {
        // A USER MACRO BEATS EVERY BUILTIN OF THE SAME NAME — directives and
        // mnemonics alike — and `!` is the only escape. asl, with a `org` macro
        // and a `move` macro in scope:
        //
        // ```text
        //   10/     100 : 11                  	dc.b	$11
        //   11/     101 : (MACRO)              	org	$200
        //   11/     101 : EE                          dc.b    $EE
        //   12/     102 : 22                  	dc.b	$22
        //   13/     103 : (MACRO)              	move.w	#1,d0
        //   13/     103 : DD                          dc.b    $DD
        //   14/     104 : 44                  	dc.b	$44
        //   15/     300 :                     	!org	$300
        //   16/     300 : 55                  	dc.b	$55
        // ```
        //
        // `org $200` moves the counter by ONE byte (the macro's `dc.b $EE`),
        // not to $200; `move.w` emits `DD`; only `!org $300` reaches the
        // builtin. Checked HERE, ahead of the keyword table, because the
        // keyword arms below would otherwise silently win: `s2.macrosetup.asm`
        // redefines `org` (forward-only, padding-counting) and `align` (as
        // `cnop 0,n`, i.e. through that same `org`), and running asl's builtins
        // for those two is running a different program with no diagnostic.
        //
        // The mnemonic side of the rule was already right — `lower_instruction`
        // sits at the bottom of the match — and so was the `.ATTRIBUTE`-suffix
        // side, which `dispatch` resolves before calling here.
        if !forced_builtin && self.macros.contains_key(head) {
            self.expand_macro(head, rest);
            return;
        }
        // The DIRECTIVE/MNEMONIC name folds (`fold_kw`); `head` itself stays
        // raw and is what every macro lookup and every symbol-defining arm
        // below uses, so a macro or label named `Foo` is never rewritten.
        match fold_kw(head).as_ref() {
            "cpu" => self.directive_cpu(rest, span),
            "phase" => self.directive_phase(rest, span),
            "dephase" => self.directive_dephase(),
            "org" => self.directive_org(rest, span),
            // A `label` line with NO name. Every named spelling is intercepted
            // in `exec_one` before dispatch, so reaching here means the name
            // field was empty — which is what `__LABEL__ label *` becomes in a
            // `{INTLABEL}` macro invoked without a label. asl lists the line and
            // defines nothing, with no diagnostic:
            //
            // ```text
            //   12/ 1007 : =>FALSE                      if ""<>""
            //   12/ 1007 :                      label *
            // ```
            "label" => {}
            "save" => self.state.save(),
            "restore" => {
                if let Err(m) = self.state.restore() {
                    self.err(span, m);
                }
            }
            "padding" => self.state.padding = on_off(rest),
            "supmode" => self.state.supmode = on_off(rest),
            "db" | "dc.b" => self.directive_db(rest, span),
            "dw" => self.directive_dw(rest, span),
            "dc.w" => self.directive_dc_w(rest, span),
            "dc.l" => self.directive_dc_l(rest, span),
            "ds.b" => self.directive_ds(1, rest, span),
            "ds.w" => self.directive_ds(2, rest, span),
            "ds.l" => self.directive_ds(4, rest, span),
            "align" => self.directive_align(rest, span),
            // `set NAME, VALUE` — asl's comma-operand spelling of the SET
            // directive (the reassignable-symbol assignment, name in the
            // OPERAND column rather than the label column). Aeon writes this in
            // `rept`-unrolled data init (`set .c, 0` / `set .c, .c+DMAEntry_len`).
            // Verified against asl: `set .c, 0` assigns `.c = 0` exactly like
            // `.c set 0`. Gated to 68000 so the Z80 `set BIT,(ix+d)` bit
            // instruction (same head word) still routes to Z80 lowering below.
            "set" if self.state.cpu == Cpu::M68000 => {
                self.directive_set_comma("set", rest, span)
            }
            // `eval NAME, VALUE` — asl's processor-neutral spelling of the same
            // directive, and the one the disassemblies use precisely because
            // `set` is a Z80 bit instruction. UNGATED: `eval` is a mnemonic on
            // no supported target, so `eval j,9` under `CPU Z80` is the
            // directive there too (asl-verified). `sound/_smps2asm_inc.asm`
            // writes this form 68 times.
            "eval" => self.directive_set_comma("eval", rest, span),
            "error" => {
                let m = self.interp_string(rest);
                self.err(span, m);
            }
            "fatal" => {
                let m = self.interp_string(rest);
                self.err(span, m);
                self.aborted = true;
            }
            "message" => {
                let _ = self.interp_string(rest);
            }
            "include" => self.directive_include(rest, span),
            // Real Aeon source spells this directive uppercase at all 43 call
            // sites (`grep -rn BINCLUDE aeon/games aeon/engine`); the AS
            // surface accepts either spelling like any other directive, so the
            // arm is written in the folded (lower-case) form.
            "binclude" => self.directive_binclude(rest, span),
            // `END` (asl's end-of-source / entry-point directive). Emits no
            // bytes — bare `END` and `END <entrypoint>` are both emission
            // no-ops (probe: 2026-07-04-m1d-t2-abs-ea-end-probes.md). Aeon's
            // only use is the bare `END` at main.asm:446. Does not collide with
            // the `endif`/`endm`/`endr`/`endcase` block closers (handled in
            // block scanning, not dispatch).
            "end" => {}
            "shift" => self.directive_shift(span),
            // Unreachable for a plain dispatch (the precedence check at the top
            // of this function has already expanded it) and correctly dead for a
            // forced-builtin one; kept as the explicit statement that a macro
            // never reaches the mnemonic arm below.
            _ if !forced_builtin && self.macros.contains_key(head) => {
                self.expand_macro(head, rest)
            }
            // `label: SomeStruct` — placing an instance of a declared struct.
            // BELOW the directive arms deliberately: a struct may be named
            // anything, and a declaration called `org` must not take the `org`
            // line away from the builtin. ABOVE the mnemonic arms, because a
            // struct name in the mnemonic column is exactly what `SMPS_RAM` and
            // `SoundQueue` are, and reaching instruction lowering with one is
            // the `X is not a recognized 68000 mnemonic` this row removes.
            _ if self.structs.contains_key(head) => self.instantiate_struct(head, span),
            // `is_mnemonic` only recognizes Z80 mnemonics; under `cpu 68000` the
            // m68k dispatch (lower_m68k) is still a stub (M1.C T4/T5), so any
            // non-directive head is routed there rather than misreported as
            // "unknown directive or mnemonic".
            _ if self.state.cpu == Cpu::Z80 && is_mnemonic(head) => {
                self.lower_instruction(head, rest, span)
            }
            _ if self.state.cpu == Cpu::M68000 => self.lower_instruction(head, rest, span),
            _ => self.err(span, format!("unknown directive or mnemonic `{head}`")),
        }
    }

    fn open_section_if_needed(&mut self) {
        if !self.in_section {
            // Physical LMA of this section's start = the continuous counter's
            // current value (`phys_base`, already advanced past any closed
            // section). The phased VMA base = physical + `disp` (equals the LMA
            // when not phased). Name by VMA base so the two real output regions
            // stay `sec0`/`sec32768` (the harness/M0 gate keys on those names).
            // Collisions between two auto-opened sections at the same VMA base
            // are disambiguated later, over NON-EMPTY sections only, by
            // `dedup_section_names` (an empty stray section — excluded from byte
            // emission by `flatten` and from naming disambiguation, but NOT
            // dropped: it and any `equ_syms` it carries survive into the link —
            // must not steal the bare name from a real region).
            let vma_base = (self.phys_base as i64 + self.state.disp) as u32;
            let name = format!("sec{vma_base}");
            self.builder
                .switch_section_lma(&name, self.state.cpu, Some(vma_base), self.phys_base);
            self.in_section = true;
            // Task B1 (seam re-eval): flush any int `equ`s recorded while no
            // section was open onto this newly opened one (see
            // `directive_equate`'s doc for why they couldn't attach eagerly).
            for eq in self.pending_equ_syms.drain(..) {
                self.builder.add_equ_sym(eq);
            }
        }
    }

    /// Close the open section, folding its emitted length into the continuous
    /// physical counter so the NEXT section starts at the right ROM offset.
    /// Idempotent: a second call while already closed does nothing (so a directive
    /// that closes an already-closed region can't double-advance `phys_base`).
    fn close_section(&mut self) {
        if self.in_section {
            self.phys_base += self.builder.current_offset();
            self.in_section = false;
        }
    }

    fn define_label(&mut self, name: &str) {
        self.open_section_if_needed();
        let value = self.here_i64();
        let qualified = if name.starts_with('.') {
            qualify(name, self.scope.as_deref())
        } else {
            self.scope = Some(name.to_string());
            name.to_string()
        };
        self.env.define(&qualified, SymbolValue::Int(value));
        self.known_labels.insert(qualified.clone());
        self.builder.define_label(&qualified);
    }

    fn directive_cpu(&mut self, rest: &[Token], span: Span) {
        // A processor name that begins with a digit reaches the lexer as an
        // integer literal (`68000`, `68008`), one that begins with a letter as
        // an identifier (`z80`, `z80undoc`). Both are SPELLINGS and both are
        // carried to the table as written: an integer's VALUE is what
        // distinguishes `68000` from `6502`, so discarding it and reading every
        // numeric name as one fixed processor is how a source for an unrelated
        // instruction set assembles clean as a 68000.
        let name = match rest.first().map(|t| &t.tok) {
            Some(Tok::Ident(s)) => s.clone(),
            Some(Tok::Int(n)) => n.to_string(),
            _ => {
                self.err(span, "cpu needs a name");
                return;
            }
        };
        // The processor NAME folds with the directive that carries it — real
        // sources write `CPU Z80` / `cpu z80` interchangeably, and getting this
        // wrong is not a diagnostic but a silent change of target: under
        // `Cpu::Z80` a `$` lexes as the program counter rather than a hex
        // prefix, so an unrecognized `CPU 68000` line leaves a 68000 source
        // assembling as a Z80 program. A name the table does not carry is
        // refused for the same reason, from the other side: sigil does not
        // encode that instruction set, so any target it picked instead would be
        // the wrong one.
        let folded = fold_kw(&name);
        let Some(cpu) = cpu_for_spelling(&folded) else {
            self.err(span, unsupported_cpu(&folded));
            return;
        };
        // The `cpu` directive resets padding/supmode to the CPU default,
        // unconditionally (asl-verified — see state.rs::set_cpu). Aeon's real
        // `padding off` at main.asm:3 therefore survives only until the first
        // subsequent `cpu` directive / cpu-changing `restore` (boot.asm's z80
        // load blocks), after which padding is ON for the rest of the ROM.
        // `declare_cpu`, not `set_cpu`: this is the unit DECLARING its processor,
        // which is what lifts the `CPU_UNDECLARED` refusal. `restore` re-applies a
        // saved CPU through `set_cpu` and declares nothing.
        self.state.declare_cpu(cpu);
        self.close_section();
    }

    /// asl `padding on` (68000) inserts a single `$00` byte before a word-or-
    /// larger datum (`dc.w`/`dc.l`/any instruction) whose logical PC `$` is odd,
    /// keeping 68k data/code word-aligned. Alignment is on the LOGICAL `$`
    /// (`physical + phase disp`), not the physical offset — asl-verified (the
    /// `phase_logodd`/`phase_logeven` probes in
    /// `docs/superpowers/notes/2026-07-04-m1d-t0.1-padding-probes.md`). No-op
    /// under `padding off` (Aeon's initial state), on a Z80 CPU (byte stream), or
    /// at an even `$`. `dc.b` never calls this (alignment 1).
    fn pad_word_align(&mut self, span: Span) {
        if self.state.padding && self.state.cpu == Cpu::M68000 && !self.here().is_multiple_of(2) {
            self.open_section_if_needed();
            self.emit(&[0x00], vec![], span);
        }
    }

    fn directive_phase(&mut self, rest: &[Token], span: Span) {
        match self.eval_all(rest, span) {
            Some(v) => {
                // `phase addr` makes `$` report `addr` at the current physical
                // point WITHOUT moving the physical counter: set the displacement
                // to `addr - physical_now`. Compute the physical point BEFORE
                // closing the section (close folds the length into `phys_base`,
                // which leaves `current_physical()` unchanged — but order-safe).
                let phys_now = self.current_physical();
                self.close_section();
                self.state.disp = v - phys_now as i64;
            }
            None => self.err(span, "phase needs a constant expression"),
        }
    }

    fn directive_dephase(&mut self) {
        // Cancel the phase: `$` reports the physical location again. The physical
        // counter has ADVANCED by the phased block's bytes (folded into `phys_base`
        // by `close_section`), so labels after `dephase` continue from there — they
        // are NOT rewound. `disp` returns to 0 (an explicit balance of `phase`;
        // `restore` never touches it).
        self.close_section();
        self.state.disp = 0;
    }

    /// AS `org <target>` (M1.C T6b). `target` is an ABSOLUTE address (like
    /// `phase`'s argument), evaluated eagerly (matching `directive_align`/
    /// `directive_ds`'s pattern of resolving directive arguments at eval time
    /// rather than deferring an `Expr` into the fragment). Two cases, per the
    /// asl-verified back-patch + absolute-org rules (M1.C T6b investigation):
    ///
    /// - **No section open yet** (e.g. `main.asm`'s very first `org 0`, before
    ///   any byte has been emitted): behaves exactly like `phase`'s no-section
    ///   path — just records the base for the next emit to open a section at.
    /// - **A section IS open** and `target` falls within bytes the section has
    ///   ALREADY written (`target - base <= builder.extent()`): an in-section
    ///   back-patch seek (`org pscStart / dc.b n / org pscEndPos`, the
    ///   `parallax_section_end` idiom) — `IrBuilder::seek` repositions the
    ///   cursor; subsequent `Data`/`Fill` overwrite in place.
    /// - Otherwise (`target` is beyond anything written): a forward jump into
    ///   brand-new territory (`main.asm`'s `org $10000` starting the object
    ///   code bank) — closing the section and re-phasing at `target`, so the
    ///   gap is filled by `flatten`'s ordinary inter-section gap-fill instead of
    ///   growing this section's `Org`+`JmpJsrSym` mix (which `resolve_layout`
    ///   refuses — see its guard — since real engine code between `org 0` and
    ///   `org $10000` contains bare `jmp`/`jsr`).
    fn directive_org(&mut self, rest: &[Token], span: Span) {
        let target_abs = match self.eval_all(rest, span) {
            Some(v) => v as u32,
            None => {
                self.err(span, "org needs a constant expression");
                return;
            }
        };
        // `org N` sets the location counter so `$` == N. `$` == physical + disp,
        // so the physical target is `N - disp` (reduces to N outside any phase,
        // which is every real `org` site). Setting `phys_base` directly is how the
        // physical counter jumps.
        let phys_target = (target_abs as i64 - self.state.disp) as u32;
        if !self.in_section {
            self.phys_base = phys_target;
            // R7p.1: the org is an explicit placement authority. The next section
            // opened here must be `Pinned` at this counter (its gap from the
            // predecessor is intentional), not `Chained` and compacted by the
            // link-time placement pass.
            self.builder.pin_next_section();
            return;
        }
        // A section is open. `base` is the VMA of its first byte; `rel` is the
        // target's offset within it. Within the already-written extent this is an
        // in-place back-patch seek (`parallax_section_end`); beyond it, a forward
        // jump that closes the section and re-bases the physical counter (so the
        // gap is inter-section gap-fill, not a growing Org+JmpJsrSym run).
        let base = (self.phys_base as i64 + self.state.disp) as u32;
        if target_abs < base {
            self.err(span, "org target precedes the current phase base");
            return;
        }
        let rel = target_abs - base;
        if rel <= self.builder.extent() {
            self.builder.seek(rel, 0, span);
        } else {
            self.close_section();
            self.phys_base = phys_target;
            // Forward org past the section extent → the next auto-opened section
            // is `Pinned` at the org'd counter (R7p.1): its gap is an intentional
            // inter-section gap, which the placement pass must preserve rather
            // than compact (`org_forward_new_section` golden).
            self.builder.pin_next_section();
        }
    }

    fn directive_equate(&mut self, name: &str, rest: &[Token], span: Span) {
        // An equate is not a label: qualify a local `.foo` against the current
        // scope (so `ld a,.foo` resolves) but do NOT open a scope. `qualify`
        // leaves non-dotted global names unchanged. Inside a macro expansion the
        // scope is the CALLER's ([`Self::real_scope`]) — asl `-U`, `.eqs = 3`
        // inside a macro under `Base:` lists as `Base.eqs : 3`.
        let q = qualify(name, self.real_scope());
        // The P5 no-silent-shadowing guard: a guarded `.emp`-owned constant may
        // NOT be re-authored in the residual AS. An in-file `=`/`equ` of such a
        // name fails LOUD (never silently prefers either side) — the structural
        // proof that a flipped constant has exactly one author, the `.emp` module.
        if self.guarded_defines.contains(&q) {
            self.err(
                span,
                format!(
                    "[defines.collision] `{q}` is an .emp-owned constant injected as a guarded define; \
                     it must not be redefined in the residual AS (delete this in-file definition — \
                     the .emp module engine.constants is the sole author)"
                ),
            );
            return;
        }
        // asl: `equ` may bind a STRING just as `set` does (`GAME_CONSOLE equ
        // "SEGA GENESIS    "`, then read back via `strlen(GAME_CONSOLE)` in the
        // engine's gameHeader width asserts). Detect the string shape via
        // `eval_str` (literal / substr / lowstring / string-symbol copy) BEFORE
        // the numeric fold and store it front-end-only (§7.4) — mirrors
        // `directive_set`. Without this a string `equ` was silently dropped
        // (neither map written), so `strlen()`/`substr()` on it could not
        // resolve. The int XOR string invariant per pass still holds.
        if let Some(s) = self.eval_str(rest) {
            // `\{expr}` folds where the string is BOUND, not where it is read:
            // `s := "\{n}"` with `n := 3` binds `s` to `"3"` for good, and a
            // later `n := 42` leaves `s` alone (asl-verified). Folding here also
            // makes `strlen(s)` see the rendered text rather than the source
            // spelling.
            let s = self.interp_text(&s);
            self.float_env.remove(&q);
            self.str_env.insert(q, s);
            return;
        }
        if let Some(f) = self.float_rhs(rest) {
            self.str_env.remove(&q);
            self.float_env.insert(q, f);
            return;
        }
        if let Some(v) = self.eval_all(rest, span) {
            self.float_env.remove(&q);
            self.env.define(&q, SymbolValue::Int(v));
            // A label-referencing equate (`HandlerPtr = Handler`, the debugger's
            // `DEBUGGER__* = MDDBG__* = ErrorHandler + N` chain): its VALUE is a
            // relaxation-shiftable label address. DETECT and register such a name
            // on EVERY pass — the detection is byte-neutral (it only feeds
            // `expr_refs_label`, whose consumer sites are all `keep_labels_symbolic`-
            // gated, inert off the deferral pass) and MUST run on the ordinary
            // passes so the threaded set carries a producer equ (defined LATE, e.g.
            // mddbg_symbols.asm) to a consumer equ (defined EARLY, debugger.asm)
            // that references it. The env value stays `Int(v)`, so convergence is
            // unaffected. On the deferral pass ONLY, a registered name is EXPORTED
            // to the linker as a SYMBOLIC equ_sym (`relax_safe_fold` keeps section
            // labels / chained label-ref equs symbolic and bakes env-only
            // subterms — `X = Label + CONST` ships `Sym(Label) + Int(CONST)`); the
            // linker folds it post-relax onto the shifted label. A pure-constant
            // equ never matches and keeps baking `Int(v)`.
            let sym_rhs = crate::expr::parse_expr(&self.expand_calls(rest, 0))
                .and_then(|(e, tail)| tail.is_empty().then_some(e))
                .map(|e| self.resolve_dollar(&self.qualify_expr(&e)))
                .filter(|e| self.expr_refs_label(e));
            let equ_expr = match &sym_rhs {
                Some(e) => {
                    self.label_ref_equs.insert(q.clone());
                    if self.keep_labels_symbolic() {
                        self.relax_safe_fold(e)
                    } else {
                        Expr::Int(v)
                    }
                }
                None => Expr::Int(v),
            };
            // Task B1 (seam re-eval): export the int equate to the module's
            // link-level `equ_syms` so `.emp` code can read it via `extern()`.
            // AS equates today live ONLY in `self.env` (front-end-private), so
            // they never reach the linker's symbol table — this closes that
            // seam. `run`'s pass loop builds a FRESH `IrBuilder` every pass
            // (`one_pass` → `Asm::new`) and returns only the CONVERGED final
            // pass's module, so an unconditional call here already yields
            // exactly one `EquSym` per name carrying the fully-folded final
            // value — no separate once-only/final-pass gating needed.
            //
            // `add_equ_sym` panics with no open section, and an equate CAN occur
            // before any section has opened. Naively calling
            // `open_section_if_needed()` here (mirroring `directive_binclude`)
            // was tried and REJECTED: unlike those directives, `equ` runs at
            // points `directive_org`/`directive_phase` rely on being possibly
            // section-FREE. `directive_org`'s `!self.in_section` branch
            // unconditionally jumps `phys_base` to the target with no
            // backward-move check (real Aeon `org 0` after a RAM `phase` block
            // whose `dephase` folded the RAM reservation size into `phys_base`
            // relies on exactly this reset). Eagerly opening a section here
            // would leave a stray section open across that boundary, flipping
            // `org` onto its OTHER branch (`target_abs < base` validated) and
            // spuriously erroring "org target precedes the current phase base"
            // — reproduced against the real `aeon` corpus (`engine.inc`'s
            // `org 0`, preceded by `engine/debug/debugger.asm`'s leading
            // `DEBUGGER__EXTENSIONS__ENABLE: equ 1`). So: attach the EquSym to
            // the builder's CURRENTLY open section if one exists; otherwise
            // stash it in a pending list `close_section`/`switch_section_lma`
            // flush into the next section that actually opens, WITHOUT
            // side-effecting `in_section`/`phys_base` here.
            if self.in_section {
                self.builder.add_equ_sym(EquSym { name: q, expr: equ_expr, span });
            } else {
                self.pending_equ_syms.push(EquSym { name: q, expr: equ_expr, span });
            }
        } else {
            // `eval_all` failed: the RHS references a symbol not resolvable in
            // THIS AS unit — a cross-seam `.emp` label joined only at link time
            // (`Game_Entry = GameState_OJZScroll_Init`; the mixed-build
            // `ErrorHandler = ErrorHandlerBlob` alias + its `MDDBG__* =
            // ErrorHandler + N` chain). Without this the equate was silently
            // dropped and every reference to it dangled at link. Emit it as a
            // DEFERRED symbolic `equ_sym` (`relax_safe_fold` keeps the unresolved
            // symbol symbolic and bakes any env-only subterm) so
            // `resolve_layout`'s `fold_equ_syms` folds it off the external base
            // once that label is placed (equ-off-link-external-base). Emitted on
            // EVERY pass — a minimal AS unit with no cross-seam `jsr`/`jmp` poison
            // never triggers the bonus pass, so gating on it would drop the equate
            // in exactly those mixed harnesses; `run` keeps only the final module,
            // and the raw env is untouched (the RHS never folded), so convergence
            // is unaffected. An equate whose base is absent from the final link is
            // simply not defined (`fold_equ_syms` leaves it), and only a REAL
            // reference to it errors — at the fixup, as an unplaced label would.
            if let Some(e) = crate::expr::parse_expr(&self.expand_calls(rest, 0))
                .and_then(|(e, tail)| tail.is_empty().then_some(e))
                .map(|e| self.resolve_dollar(&self.qualify_expr(&e)))
                .filter(expr_has_sym)
            {
                self.label_ref_equs.insert(q.clone());
                let equ_expr = self.relax_safe_fold(&e);
                if self.in_section {
                    self.builder.add_equ_sym(EquSym { name: q, expr: equ_expr, span });
                } else {
                    self.pending_equ_syms.push(EquSym { name: q, expr: equ_expr, span });
                }
            }
        }
    }

    /// The FLOAT value of an `equ`/`=`/`set`/`:=` right-hand side, or `None`
    /// when it is not float-typed.
    ///
    /// Checked between the string branch and the integer branch of both
    /// assignment directives. asl binds a float to a symbol exactly as it
    /// binds an integer — probe `f2.asm` lines 8 and 11 list `fx = 3.7` as
    /// `=3.7` and `fy equ 2.5` as `=2.5`, and its symbol table prints
    /// `fx : 3.7`. `s2.sounddriver.asm(3901)`'s `sample_rate_scale := 1.0`
    /// (immediately reassigned to a macro parameter, then read by
    /// `int(label.sample_rate*sample_rate_scale)`) is the corpus demand.
    ///
    /// Only a genuinely FLOAT result diverts here: `eval_num` returns
    /// `Num::Float` only when a float literal or a float symbol is somewhere
    /// in the tree, so every integer assignment still takes the integer branch
    /// — including the ones `eval_num` could evaluate but the integer folder
    /// handles with far more machinery (label references, deferral, the
    /// symbolic-equ export).
    /// Deliberately `expand_calls` and then the typed evaluator, NOT the full
    /// [`Self::expand_operand_builtins`] chain: `eval_num` recognizes
    /// `int(...)`/`sin(...)` itself, and it reports NOTHING. Routing this probe
    /// through the erroring expansion instead double-reports every failure, and
    /// it was measured doing so — `s2.asm(87677)`'s
    /// `.loop_counter = int(log(number))` (asl's `log` builtin, which sigil does
    /// not have) went from 6 diagnostics to 12 across the S2 corpus, one pair per
    /// call site: a speculative type test must not be able to raise a diagnostic.
    fn float_rhs(&mut self, rest: &[Token]) -> Option<f64> {
        let expanded = self.expand_calls(rest, 0);
        match self.eval_num(&expanded)? {
            Num::Float(f) => Some(f),
            Num::Int(_) => None,
        }
    }

    /// `name set <expr>` / `name := <expr>` (T8): AS's reassignable-symbol
    /// forms, e.g. Aeon's band counters / `OE_PREV_X` sort checks / deform
    /// accumulators. `eval_all` folds `rest` against `self.env` AS IT STANDS
    /// AT THIS LINE, so a self-reference (`i set i+5`) reads the CURRENT
    /// value of `i` — the redefinition below then overwrites it, giving
    /// emission-order imperative semantics (verified against real asl: `i set
    /// 0 / dc.b i / i set i+5 / dc.b i` → `00 05`). Deliberately its own
    /// function rather than an alias of `directive_equate`: `=` is slated to
    /// grow a single-assignment redefinition diagnostic (see that function's
    /// doc), and `set`/`:=` must keep permitting redefinition when it does.
    fn directive_set(&mut self, name: &str, rest: &[Token], span: Span) {
        // A `.`-local `set` inside a macro expansion binds in the CALLER's scope,
        // not the expansion's, and macro nesting is transparent to it
        // ([`Self::real_scope`]). This is what carries `zoneOrderedTable`'s
        // `.cur_zone_str` / `.zone_entries_left` across to the separate
        // `zoneTableEntry` expansions that read and reassign them.
        let q = qualify(name, self.real_scope());
        // asl: `set` may bind a STRING (`.__str set "BUS ERROR"`,
        // `.__str set substr(.__str,0,.__pos)`). Detect the string shape via
        // `eval_str` (literal / substr / lowstring / string-symbol copy) BEFORE
        // the numeric fold, and store it front-end-only (§7.4). Probe p1/p4.
        //
        // INVARIANT (relied on, not enforced): a symbol is int XOR string within
        // a pass. The string branch writes `str_env`, the int branch writes
        // `env`, and neither clears the other, so a `set` that FLIPS a symbol's
        // type mid-pass would leave stale entries in both maps and resolve to
        // whichever the use site consults. This is safe for every real target
        // (the `__FSTRING` scan assigns each symbol one stable type before it is
        // read — probe p1/p4); type-flipping `set` is unsupported. Poison-
        // shadowing the counterpart would be un-probed asl semantics, so it is
        // deliberately NOT done here.
        if let Some(s) = self.eval_str(rest) {
            // `\{expr}` folds where the string is BOUND, not where it is read
            // (asl-verified): `s := "\{n}"` captures `n`'s value at this
            // assignment, and a later reassignment of `n` does not reach `s`.
            let s = self.interp_text(&s);
            self.float_env.remove(&q);
            self.str_env.insert(q, s);
            return;
        }
        if let Some(f) = self.float_rhs(rest) {
            self.str_env.remove(&q);
            self.float_env.insert(q, f);
            return;
        }
        if let Some(v) = self.eval_all(rest, span) {
            self.float_env.remove(&q);
            self.env.define(&q, SymbolValue::Int(v));
            // Relocation capability (flip Stage 2): if the RHS — after splicing
            // any set-symbol it CHAINS through (`P_DFG := PC_FG_T`) — references a
            // section LABEL, remember its `relax_safe_fold`ed symbolic snapshot so
            // a later `dc.l`/`jsr`/... of this set keeps the label symbolic and
            // relocates, instead of baking the this-pass VMA (`P_DBG := deformBg`
            // in a chainer-relocated parallax record). Reassigning to a label-free
            // value CLEARS the entry (snapshot/sequential semantics). Built on
            // every pass (a chain needs the map populated in order within the
            // deferral pass); byte-neutral because the map is read only under
            // `keep_labels_symbolic`. `relax_safe_fold` splices set-symbols at its
            // root, so a chained set stores the underlying label expr directly.
            let sym_rhs = crate::expr::parse_expr(&self.expand_calls(rest, 0))
                .and_then(|(e, tail)| tail.is_empty().then_some(e))
                .map(|e| self.resolve_dollar(&self.qualify_expr(&e)));
            match sym_rhs {
                Some(ref e) if self.expr_refs_label(e) => {
                    let folded = self.relax_safe_fold(e);
                    self.set_sym_symbolic.insert(q, folded);
                }
                _ => {
                    self.set_sym_symbolic.remove(&q);
                }
            }
        }
    }

    /// `set NAME, VALUE` / `eval NAME, VALUE` — the comma-operand form of SET
    /// (see the dispatch arms). Splits the first top-level comma into the
    /// target symbol name and the value expression, then reuses
    /// `directive_set`. `kw` is the spelling the line actually used, so a
    /// diagnostic quotes the word the author wrote.
    fn directive_set_comma(&mut self, kw: &str, rest: &[Token], span: Span) {
        let groups = split_top_commas(rest);
        if groups.len() != 2 {
            self.err(span, format!("`{kw}` directive expects `NAME, value`"));
            return;
        }
        let name = match groups[0] {
            [Token {
                tok: Tok::Ident(s), ..
            }] => s.clone(),
            _ => {
                self.err(span, format!("`{kw}` directive target must be a bare symbol"));
                return;
            }
        };
        self.directive_set(&name, groups[1], span);
    }

    /// The front-end-only builtin layer over ONE operand's tokens: user
    /// `function` calls, then `int(...)`/`sin(...)`, then the string builtins
    /// and string comparisons — each collapsing to an ordinary `Tok::Int`
    /// (or, for `substr`/`lowstring`, a `Tok::Str`) before
    /// [`crate::expr::parse_expr`] ever runs (§7.4).
    ///
    /// This exists because the layer used to be wired into `dc.b` ALONE.
    /// `dc.w`/`dc.l`/`dw` ran `expand_calls` and nothing else, so every
    /// `int(...)` in a word or long operand reached `parse_expr` unexpanded
    /// and died as `bad word expression` — 166 of Sonic 1's 318 frontend
    /// diagnostics, all of them on the two `dc.w MakeFMFrequency(op)` /
    /// `dc.w MakePSGFrequency(op)` lines that build `FM_Notes` and
    /// `PSGFrequencies`. A builtin that works at one width and not another is
    /// not a missing feature so much as a trap, so the widths share one
    /// function rather than three parallel pipelines.
    fn expand_operand_builtins(&mut self, toks: &[Token]) -> Vec<Token> {
        let expanded = self.expand_calls(toks, 0);
        let expanded = self.expand_int_builtin(&expanded);
        let expanded = self.expand_str_builtins(&expanded);
        self.expand_str_comparisons(&expanded)
    }

    /// The position of a FLOAT-typed leaf in `toks` — a literal that no
    /// `int(...)` consumed, or a name bound in [`Self::float_env`].
    fn float_leaf(&self, toks: &[Token]) -> Option<Span> {
        toks.iter().find_map(|t| match &t.tok {
            Tok::Float(_) => Some(t.span),
            Tok::Ident(n) => self.resolve_float_sym(n).map(|_| t.span),
            _ => None,
        })
    }

    /// Reduce an operand that still mentions a float to a plain integer token,
    /// or say where the float is that stops it.
    ///
    /// A float LEAF does not by itself make an operand invalid: asl's
    /// comparison operators take floats and yield integers, so `dc.l 3.5<4` is
    /// `0000 0001` (probe `f2.asm(16)`). What asl refuses is a float
    /// **result** in an integer context — `dc.l 3.7` and `dc.l fx` (after
    /// `fx = 3.7`) both draw `error #1133: expected integer or string, but got
    /// floating point number` (probes `f1.asm(17-19)`, `f3.asm(5)`).
    ///
    /// So a float-free operand is returned untouched — the integer path with
    /// its labels, forward references and link deferral is left entirely
    /// alone, which is why this cannot perturb any program that assembles
    /// today. Only when a float is present does the typed evaluator run, and
    /// then it must land on an integer.
    ///
    /// Running BEFORE the numeric parse is what makes the answer name the real
    /// problem. A float TOKEN can never parse as an `Expr` (there is no float
    /// atom), so it would read as a generic `bad word expression`; a float
    /// SYMBOL is worse — it parses as a bare `Expr::Sym`, folds to Poison and
    /// defers to the linker, reporting a symbol that has a perfectly good
    /// value as an undefined one.
    fn collapse_float_operand(&mut self, toks: &[Token]) -> Result<Vec<Token>, Span> {
        let Some(fsp) = self.float_leaf(toks) else {
            return Ok(toks.to_vec());
        };
        match self.eval_num(toks) {
            Some(Num::Int(v)) => Ok(vec![Token { tok: Tok::Int(v), span: fsp }]),
            _ => Err(fsp),
        }
    }

    fn directive_db(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let called = self.expand_calls(g, 0);
            let expanded = self.expand_int_builtin(&called);
            let expanded = self.expand_str_builtins(&expanded);
            let expanded = match self.collapse_float_operand(&expanded) {
                Ok(t) => t,
                Err(fsp) => {
                    self.err(fsp, FLOAT_IN_INT_CONTEXT);
                    continue;
                }
            };
            // (T6c) A STRING operand — a plain `Tok::Str` literal or a
            // string-builtin call that resolves to one (`substr(...)`,
            // `lowstring(...)`) — emits one ASCII byte per character
            // instead of folding as a numeric expression (asl-verified:
            // `dc.b "AB"` -> `41 42`; `dc.b substr("hello",1,2)` -> `65 6C`).
            // This is the shape only, checked BEFORE the numeric parse below
            // so plain numeric/symbol operands are unaffected.
            if let Some(s) = self.eval_str(&expanded) {
                let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
                self.emit(&bytes, vec![], span);
                continue;
            }
            // Fold any nested string comparison (`substr(...)="x"`) to 0/1 before
            // the numeric parse (mirrors `eval_all`; T5).
            let expanded = self.expand_str_comparisons(&expanded);
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad byte expression");
                    continue;
                }
            };
            // Fold against the current env. A value in range emits its byte; an
            // out-of-range value clamps + diagnoses (asl parity, via fold_imm's
            // path); an UNRESOLVED expression (bare symbol OR compound) DEFERS to
            // the linker as a general link-expr VALUE — one placeholder byte $00 +
            // a `Value8` fixup carrying the full parsed+qualified expr tree
            // (R-T0.4). This is the consumption half of the .emp→.asm seam: aeon's
            // `dac_sample_tab.asm` reads `SND_*` bank ids via `db BANK`, and those
            // constants move to `.emp` as link-folded equ symbols. In an ALL-AS
            // assembly everything folds by the final pass and this arm never fires
            // (byte-diff net). Mirrors `directive_dw`'s deferral, at width 1.
            let qe = self.qualify_expr(&e);
            match self.fold(&qe) {
                Fold::Value(v) => {
                    if !(-128..=0xFF).contains(&v) {
                        self.err(span, format!("operand {v} out of range {}..={}", -128, 0xFF));
                    }
                    self.emit(&[v.clamp(-128, 0xFF) as u8], vec![], span);
                }
                Fold::Poison => {
                    self.emit(
                        &[0x00],
                        vec![Fixup {
                            kind: FixupKind::Value8,
                            offset: 0,
                            // Bake env-resolvable subterms; defer only the true
                            // cross-seam leaf (mirrors `directive_dw`).
                            target: self.partial_fold(&qe),
                        }],
                        span,
                    );
                }
            }
        }
    }

    fn directive_dw(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        for g in split_top_commas(rest) {
            let expanded = self.expand_operand_builtins(g);
            let expanded = match self.collapse_float_operand(&expanded) {
                Ok(t) => t,
                Err(fsp) => {
                    self.err(fsp, FLOAT_IN_INT_CONTEXT);
                    continue;
                }
            };
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad word expression");
                    continue;
                }
            };
            let qe = self.qualify_expr(&e);
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let w = v as u16;
                    self.emit(&[(w & 0xFF) as u8, (w >> 8) as u8], vec![], span);
                }
                Fold::Poison => {
                    // ANY unresolved expression (bare symbol OR compound) defers
                    // to the linker as a general link-expr VALUE — two placeholder
                    // bytes + a `Value16Le` fixup carrying the full parsed+
                    // qualified expr tree (R-T0.4; the linker folds arbitrary
                    // trees, per the `RelWord16Be` offset-table precedent).
                    //
                    // This deliberately REPLACES the old `BankPtr16Le` special
                    // case (which deferred ONLY a bare `Expr::Sym` and rejected
                    // compounds): `Value16Le` writes the folded value VERBATIM
                    // after an unsigned-window range check, whereas `BankPtr16Le`
                    // is an ADDRESS kind — its 68k `BankPtr16Be` counterpart masks
                    // the windowed low-16, and truncating an out-of-range fold to
                    // `value as u16` is the silent-wrong-bytes class. A
                    // `dw SND_KICK_LEN` where LEN=$057E must emit $057E verbatim,
                    // not silently masked/truncated; window masking, when needed,
                    // belongs in SOURCE (aeon's `sfx_winptr()` macro writes
                    // `(v & $7FFF) | $8000` explicitly, and that tree folds here).
                    self.emit(
                        &[0x00, 0x00],
                        vec![Fixup {
                            kind: FixupKind::Value16Le,
                            offset: 0,
                            // Bake env-resolvable subterms (e.g. `sfx_winptr`'s
                            // `SFX_WIN_MASK`/`SFX_WIN_BASE` equs) HERE — the
                            // linker only sees the true cross-seam leaf.
                            target: self.partial_fold(&qe),
                        }],
                        span,
                    );
                }
            }
        }
    }

    /// `dc.w <expr>,...` — big-endian 16-bit words (asl: BE, unlike the Z80
    /// `dw`'s little-endian). Mirrors `directive_dw`'s expr-list parsing.
    ///
    /// Unresolved arm, two cases (R-T0.4's planned migration, taken up in
    /// tranche 6 when its first cross-seam customer arrived):
    /// - a bare `Sym` keeps its `Abs16Be` (ADDRESS) behavior — existing
    ///   consumers store pointer words, and the address kind's signed range
    ///   check is the right one for them;
    /// - a COMPOUND expression defers as a general link-expr VALUE — two
    ///   placeholder bytes + a `Value16Be` fixup carrying the parsed+qualified
    ///   tree (the `dw`/`Value16Le` precedent, BE for 68k sections). The
    ///   demand shape is `dc.w objroutine(TestSolid_Init)` — a
    ///   `sym - ObjCodeBase` bank-offset word whose sym is `.emp`-owned in
    ///   the mixed build. `Value16Be` range-checks the folded value to the
    ///   unsigned 16-bit window, so an out-of-range difference is loud, not
    ///   silently truncated.
    fn directive_dc_w(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        self.pad_word_align(span);
        for g in split_top_commas(rest) {
            let expanded = self.expand_operand_builtins(g);
            let expanded = match self.collapse_float_operand(&expanded) {
                Ok(t) => t,
                Err(fsp) => {
                    self.err(fsp, FLOAT_IN_INT_CONTEXT);
                    continue;
                }
            };
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad word expression");
                    continue;
                }
            };
            let qe = self.qualify_expr(&e);
            // On the deferral pass, a `dc.w` whose value references a section
            // label (an offset-table row `Target-Base`, or a truncated address)
            // must carry the label(s) SYMBOLICALLY — a width-grown `JmpJsrSym`
            // shifts them and a baked word would go stale. `Value16Be` matches
            // the resolved path's `v as u16` low-16 truncation. See
            // `keep_labels_symbolic`.
            let qed = self.resolve_dollar(&qe);
            if self.keep_labels_symbolic() && self.expr_refs_label(&qed) {
                self.emit(
                    &[0x00, 0x00],
                    vec![Fixup { kind: FixupKind::Value16Be, offset: 0, target: self.relax_safe_fold(&qed) }],
                    span,
                );
                continue;
            }
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let w = (v as u16).to_be_bytes();
                    self.emit(&w, vec![], span);
                }
                Fold::Poison => {
                    if matches!(qe, Expr::Sym(_)) {
                        self.emit(
                            &[0x00, 0x00],
                            vec![Fixup {
                                kind: FixupKind::Abs16Be,
                                offset: 0,
                                target: qe,
                            }],
                            span,
                        );
                    } else {
                        self.emit(
                            &[0x00, 0x00],
                            vec![Fixup {
                                kind: FixupKind::Value16Be,
                                offset: 0,
                                // Bake env-resolvable subterms here — the
                                // linker only sees the true cross-seam leaf
                                // (the `directive_dw` pattern).
                                target: self.partial_fold(&qe),
                            }],
                            span,
                        );
                    }
                }
            }
        }
    }

    /// `dc.l <expr>,...` — big-endian 32-bit longwords.
    fn directive_dc_l(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        self.pad_word_align(span);
        for g in split_top_commas(rest) {
            let expanded = self.expand_operand_builtins(g);
            let expanded = match self.collapse_float_operand(&expanded) {
                Ok(t) => t,
                Err(fsp) => {
                    self.err(fsp, FLOAT_IN_INT_CONTEXT);
                    continue;
                }
            };
            let e = match crate::expr::parse_expr(&expanded) {
                Some((e, [])) => e,
                _ => {
                    self.err(span, "bad long expression");
                    continue;
                }
            };
            let qe = self.resolve_dollar(&self.qualify_expr(&e));
            // On the deferral pass, a `dc.l` whose value references a section
            // label must carry that label SYMBOLICALLY (an `Abs32Be` fixup),
            // not its this-pass VMA — a width-grown `JmpJsrSym` shifts the label
            // and a baked long would go stale. See `keep_labels_symbolic`.
            if self.keep_labels_symbolic() && self.expr_refs_label(&qe) {
                self.emit(
                    &[0x00, 0x00, 0x00, 0x00],
                    vec![Fixup { kind: FixupKind::Abs32Be, offset: 0, target: self.relax_safe_fold(&qe) }],
                    span,
                );
                continue;
            }
            match self.fold(&qe) {
                Fold::Value(v) => {
                    let l = (v as u32).to_be_bytes();
                    self.emit(&l, vec![], span);
                }
                Fold::Poison => {
                    if matches!(qe, Expr::Sym(_)) {
                        self.emit(
                            &[0x00, 0x00, 0x00, 0x00],
                            vec![Fixup {
                                kind: FixupKind::Abs32Be,
                                offset: 0,
                                target: qe,
                            }],
                            span,
                        );
                    } else {
                        self.err(span, "unresolved long expression");
                        self.emit(&[0x00, 0x00, 0x00, 0x00], vec![], span);
                    }
                }
            }
        }
    }

    /// `ds.b`/`ds.w`/`ds.l <count>` — reserve `count * unit` bytes with no
    /// image bytes (verified against asl: a `ds` run with nothing emitted
    /// after it never materializes in the flat binary — matches
    /// `Fragment::Reserve`, not a real `Fill`).
    fn directive_ds(&mut self, unit: u32, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.eval_all(rest, span) {
            Some(v) if v >= 0 => self.builder.reserve(v as u32 * unit, span),
            Some(_) => self.err(span, "negative ds count"),
            None => self.err(span, "unresolved ds count"),
        }
    }

    /// `align <n>` — advance the location counter to a multiple of `n`.
    ///
    /// The pad is `asl_align_pad` (`sigil-ir`): asl rounds up on the LOW 32 BITS
    /// OF THE PC READ AS A SIGNED `i32`, with C's truncating remainder. A
    /// non-negative PC (every ROM address) gets the plain round-up and a
    /// no-op when already aligned; a negative PC (every `$FFFF….` RAM address)
    /// rounds toward zero instead of down, so it usually lands one block high
    /// and an already-aligned address advances a full `n`.
    ///
    /// The regime is the SIGN OF THE PC, not `disp`: `phase $B000` + `ds.b 5` +
    /// `align 256` gives `$B100`, the same as the unphased form. What `disp`
    /// still decides here is the KIND of pad — a phased region is Aeon RAM under
    /// `padding off`, where the pad is a `Reserve` (address-only, no image
    /// bytes), against a ROM section where it is a real `$00` `Fill`.
    fn directive_align(&mut self, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.eval_all(rest, span) {
            Some(n) if n > 0 => {
                let n = n as u32;
                let pad = sigil_ir::asl_align_pad(self.here(), n);
                if pad > 0 {
                    if self.state.disp != 0 {
                        self.builder.reserve(pad, span);
                    } else {
                        self.builder.emit_fill(pad, 0, span);
                    }
                }
            }
            Some(_) => self.err(span, "align needs a positive constant"),
            None => self.err(span, "unresolved align constant"),
        }
    }

    fn lower_instruction(&mut self, mn: &str, rest: &[Token], span: Span) {
        self.open_section_if_needed();
        match self.state.cpu {
            Cpu::Z80 => self.lower_z80(mn, rest, span),
            Cpu::M68000 => self.lower_m68k(mn, rest, span),
        }
    }

    fn lower_z80(&mut self, mn: &str, rest: &[Token], span: Span) {
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let m = match mnemonic(mn) {
            Some(m) => m,
            None => {
                self.err(span, "not a mnemonic");
                return;
            }
        };
        match self.build_operands(m, &atoms, span) {
            Some(Lowered::Fixed(ops)) => {
                let f = self.z80.lower(m, &ops, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Rel(cond, target)) => {
                let f = self.z80.lower_rel(m, cond, target, span);
                self.emit_frag(f, span);
            }
            Some(Lowered::Abs16(ops, target)) => {
                let f = self.z80.lower_abs16(m, &ops, target, span);
                self.emit_frag(f, span);
            }
            None => {}
        }
    }

    /// M1.C T4/T5/T5b/T5c: the 68000 core. Straight-line register/immediate
    /// forms, the fixed-length register-indirect EA family, `lea`/`pea`, and
    /// explicit-width absolute addressing fold immediately (no fixups). T5c
    /// adds control transfer, each routed BEFORE the generic fold-based path
    /// because its target is resolved later (by the linker), not by this
    /// pass's fold:
    ///  - `bra`/`bsr`/`Bcc` → [`Self::lower_m68k_branch`] (size-pinned by the
    ///    `.s`/`.w` suffix, no relaxation).
    ///  - `Dbcc` (`dbf`/`dbra`/`db<cc>`) → [`Self::lower_m68k_dbcc`] (the
    ///    displacement FOLDS immediately — see that method's doc for why
    ///    that's safe).
    ///  - `jmp`/`jsr` with a bare symbol/expression target → width-selected in
    ///    this pass (M1.D T3): the target folds from the current env, picks
    ///    abs.w/abs.l via `asl_width_rule`, and emits a finished `Fragment::Data`
    ///    (opcode + `Abs16Be`/`Abs32Be` fixup) via `lower_jmp_jsr_abs` — so the
    ///    cursor advances by the true width. `jmp`/`jsr` with an EA operand (e.g.
    ///    `(a0)`) falls through to the generic path like any other instruction.
    ///  - `(d16,PC)` operands (any mnemonic) → [`Self::lower_m68k_pcrel`].
    ///
    /// `movem` routes to [`Self::lower_m68k_movem`] (register-list operand);
    /// every other in-scope mnemonic (incl. `movep`) flows through the shared
    /// branch/dbcc/jmp-jsr/generic paths below.
    fn lower_m68k(&mut self, mn: &str, rest: &[Token], span: Span) {
        // Expand AS `function` calls in the operands FIRST (e.g. the immediate
        // `#vram_art(tile,0,0)` / `#vdpComm(addr,VRAM,DMA)` / `#dmaLength(N)`
        // forms — `macros.asm`'s single-expression functions). `expand_calls`
        // only rewrites `name(args)` where `name` is a known function, so
        // register-indirect EAs (`(a0)`, `(4,a0,d1.w)`) pass through untouched.
        let expanded = self.expand_calls(rest, 0);
        let rest = expanded.as_slice();
        let (base, suffix_size) = split_mnemonic_and_size(mn);
        let mnemonic = match m68k_mnemonic(base) {
            Some(m) => m,
            None => {
                match m68k_out_of_scope(base) {
                    Some(family) => {
                        self.err(span, format!("`{base}` ({family}) is not yet implemented"))
                    }
                    None => self.err(span, format!("`{base}` is not a recognized 68000 mnemonic")),
                }
                return;
            }
        };

        // Every 68k instruction is word-aligned: under `padding on` at an odd `$`,
        // asl prefixes a $00 pad byte (asl-verified — `instr_odd_pad_on` probe).
        // Covers all instruction paths (branch/dbcc/movem/jmp-jsr/generic) since
        // it runs before the dispatch below.
        self.pad_word_align(span);

        if matches!(
            mnemonic,
            M68kMnemonic::Bra | M68kMnemonic::Bsr | M68kMnemonic::Bcc(_)
        ) {
            return self.lower_m68k_branch(mnemonic, suffix_size, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Dbcc(_)) {
            return self.lower_m68k_dbcc(mnemonic, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Movem) {
            return self.lower_m68k_movem(suffix_size, rest, span);
        }
        if matches!(mnemonic, M68kMnemonic::Jmp | M68kMnemonic::Jsr) {
            let atoms = match parse_operands(rest) {
                Ok(a) => a,
                Err(d) => {
                    self.diags.push(d);
                    return;
                }
            };
            // A bare symbol/expression target (no EA parens) is 68k absolute
            // addressing whose WIDTH (abs.w vs abs.l) is selected in the
            // front-end pass loop (M1.D T3) — see the block below. An EA
            // operand (`(a0)`, `(Label).w`, `(d16,PC)`, ...) falls through to
            // the generic path below.
            if let [OperandAtom::Value(e)] = atoms.as_slice() {
                let target = self.resolve_dollar(&self.qualify_expr(e));
                let is_jsr = matches!(mnemonic, M68kMnemonic::Jsr);
                // Width selection in the front-end pass loop (M1.D T3): fold the target
                // from the current-pass env and pick abs.w/abs.l via `asl_width_rule`.
                // Unknown-this-pass (Poison) → abs.w (OPTIMISTIC) — probe-verified as asl's
                // least fixpoint (grow-only): the multi-pass `env == prev` loop then only
                // ever grows a width W→L (label addresses are monotone-nondecreasing across
                // passes), so it converges to exactly asl's minimal widths. The finished
                // Data fragment carries the true length, so the cursor advances truthfully
                // and downstream section LMAs (`phys_base`) are correct by construction.
                // See docs/superpowers/notes/2026-07-04-m1d-t3-jmpjsr-width-probes.md.
                //
                // The fold selects the width AND supplies the fixup value: when
                // resolved, BAKE the literal (`Expr::Int(v)`) into the fixup,
                // mirroring `abs_ea_from_expr`. `equ` constants live only in the
                // front-end env, never as section labels, so the linker (which
                // builds its symbol table from section labels alone) cannot
                // resolve a symbolic `equ` target; baking the folded value is
                // also correct for labels (front-end VMA == link's
                // vma_origin+offset on convergence — a real forward label folds
                // to Value on the converged pass and is baked too). The
                // symbolic form is kept only for the unresolved-this-pass
                // (Poison) case; on an ordinary converged pass a still-Poison
                // target is a genuinely-undefined symbol and errors via
                // `poison_refs`.
                //
                // Port #2 (math.emp follow-up): on `run`'s BONUS final pass
                // (`defer_unresolved_jsr_jmp`, set only when ordinary
                // convergence still left `jsr`/`jmp` targets Poison), a
                // still-Poison target instead defers as a length-variable
                // `Fragment::JmpJsrSym` — the SAME deferral the `.emp`
                // front-end's `jbra`/`jbsr` already uses, fully supported by
                // `resolve_layout`'s relaxation ladder. This lets `jsr
                // SomeSiblingModuleProc` (a real cross-seam call, e.g. aeon's
                // `jsr GetSineCosine` when `SIGIL_EMP_MATH` is on) resolve at
                // LINK time instead of hard-erroring at ASSEMBLE time.
                if self.defer_unresolved_jsr_jmp && matches!(self.fold(&target), Fold::Poison) {
                    let frag = self.m68k.lower_jmp_jsr_sym(is_jsr, target, span);
                    self.builder.emit_fragment(frag);
                    return;
                }
                let (width, fixup_target) = match self.fold(&target) {
                    Fold::Value(v) => {
                        // asl-faithful width from the resolved value; but on the
                        // deferral pass keep a section-label target SYMBOLIC so a
                        // later `JmpJsrSym` width growth (which shifts the target
                        // label) resolves correctly at link, not against a stale
                        // baked address. See `keep_labels_symbolic`.
                        let ft = if self.keep_labels_symbolic() && self.expr_refs_label(&target) {
                            self.relax_safe_fold(&target)
                        } else {
                            Expr::Int(v)
                        };
                        (asl_width_rule(v, false), ft)
                    }
                    Fold::Poison => {
                        for name in self.unresolved_names(&target) {
                            self.poison_refs.push((name, span));
                        }
                        (AbsWidth::W, target)
                    }
                };
                let frag = self.m68k.lower_jmp_jsr_abs(is_jsr, fixup_target, width, span);
                self.emit_frag(Ok(frag), span);
                return;
            }
            // `jmp`/`jsr (abs).w/.l` (an EXPLICIT-width absolute-EA target — the
            // debugger's `jsr (MDDBG__ErrorHandler).l` in __ErrorMessage): the
            // generic path bakes the folded address, which a width-grown
            // `JmpJsrSym` shifts out from under. On the deferral pass, if the
            // address references a section label (directly or through a
            // label-ref equ), emit the SAME opcode + a symbolic `Abs16Be`/
            // `Abs32Be` fixup so the linker fills it post-relax. The `.w`/`.l`
            // suffix PINS the width (no relaxation, so no width-boundary
            // hazard); the abs hole sits at offset 2 (jmp/jsr take no other
            // extension words). A resolved non-label / ordinary pass falls
            // through to the generic (baked) path unchanged.
            if let [OperandAtom::M68kAbs { addr, long }] = atoms.as_slice() {
                let qualified = self.resolve_dollar(&self.qualify_expr(addr));
                if self.keep_labels_symbolic() && self.expr_refs_label(&qualified) {
                    let inst = M68kInstruction {
                        mnemonic,
                        size: M68kSize::L,
                        ops: vec![if *long { M68kOperand::AbsL(0) } else { M68kOperand::AbsW(0) }],
                    };
                    if let Ok(mut frag) = self.m68k.lower_inst(&inst, span) {
                        debug_assert!(frag.bytes.len() >= if *long { 6 } else { 4 });
                        frag.fixups.push(Fixup {
                            kind: if *long { FixupKind::Abs32Be } else { FixupKind::Abs16Be },
                            offset: 2,
                            target: self.relax_safe_fold(&qualified),
                        });
                        self.emit_frag(Ok(frag), span);
                        return;
                    }
                }
            }
            return self.lower_m68k_generic(mnemonic, suffix_size, atoms, span);
        }

        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        self.lower_m68k_generic(mnemonic, suffix_size, atoms, span);
    }

    /// The shared tail of `lower_m68k` for every mnemonic that does NOT need
    /// its own special-cased target handling: resolve the size, detect (and
    /// deflect to [`Self::lower_m68k_pcrel`]) a `(d16,PC)` operand, else
    /// convert every atom and fold-lower the instruction directly. Also the
    /// fallback for `jmp`/`jsr` once an EA-operand form (not a bare symbol)
    /// has been ruled out by the caller.
    fn lower_m68k_generic(
        &mut self,
        mnemonic: M68kMnemonic,
        suffix_size: Option<M68kSize>,
        atoms: Vec<OperandAtom>,
        span: Span,
    ) {
        let size = match suffix_size
            .or_else(|| m68k_special_reg_size(mnemonic, &atoms))
            .or_else(|| m68k_default_size(mnemonic))
        {
            Some(s) => s,
            None => {
                self.err(
                    span,
                    "instruction needs an explicit size suffix (.b/.w/.l)".to_string(),
                );
                return;
            }
        };
        if let Some(pc_idx) = atoms
            .iter()
            .position(|a| matches!(a, OperandAtom::M68kDisp { an, .. } if an == "pc"))
        {
            return self.lower_m68k_pcrel(mnemonic, size, &atoms, pc_idx, span);
        }
        if let Some(pc_idx) = atoms
            .iter()
            .position(|a| matches!(a, OperandAtom::M68kIdx { an, .. } if an == "pc"))
        {
            return self.lower_m68k_pcrel_idx(mnemonic, size, &atoms, pc_idx, span);
        }
        if let Some(frag) = self.try_defer_long_imm(mnemonic, size, &atoms, span) {
            self.emit_frag(Ok(frag), span);
            return;
        }
        if let Some(frag) = self.try_defer_lea_abs(mnemonic, &atoms, span) {
            self.emit_frag(Ok(frag), span);
            return;
        }
        let ops = match self.convert_atoms_m68k(mnemonic, size, &atoms, span) {
            Some(o) => o,
            None => return,
        };
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// Tranche-4 act_descriptor (the "pinned-width abs-sym mode" re-scoped at
    /// tranche 3): `lea (Sym).w/.l, aN` whose Sym is unresolved (an
    /// `.emp`-side cross-seam label — ojz_scroll_test's
    /// `lea (OJZ_Act1_Descriptor).l, a0`) defers to the linker as a pinned
    /// `Abs16Be`/`Abs32Be` fixup instead of hard-erroring. The WIDTH is
    /// pinned by the explicit suffix (the consumer spells `.l`, byte-
    /// identical to what asl's width rule picked with the include present) —
    /// no relax candidates needed. `lea` has no other extension words, so
    /// the address hole sits at offset 2. A RESOLVED symbol falls through to
    /// the eager path unchanged; a BARE (suffix-less) unresolved symbol
    /// still hard-errors — the suffix is how the author pins what asl chose.
    fn try_defer_lea_abs(
        &mut self,
        mnemonic: M68kMnemonic,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<DataFragment> {
        if mnemonic != M68kMnemonic::Lea {
            return None;
        }
        let (addr, long, reg) = match atoms {
            [OperandAtom::M68kAbs { addr, long }, OperandAtom::RegOrCond(w)] => {
                (addr, *long, m68k_addr_reg(w)?)
            }
            [OperandAtom::M68kAbs { addr, long }, OperandAtom::Value(Expr::Sym(w))] => {
                (addr, *long, m68k_addr_reg(w)?)
            }
            _ => return None,
        };
        let qualified = self.qualify_expr(addr);
        if !matches!(self.fold(&qualified), Fold::Poison) {
            return None; // resolved: the eager path picks it up, byte-identical
        }
        let inst = M68kInstruction {
            mnemonic,
            size: M68kSize::L,
            ops: vec![
                if long { M68kOperand::AbsL(0) } else { M68kOperand::AbsW(0) },
                M68kOperand::An(reg),
            ],
        };
        let mut frag = self.m68k.lower_inst(&inst, span).ok()?;
        debug_assert!(frag.bytes.len() >= 4, "lea (abs),aN must encode opcode + ext word(s)");
        frag.fixups.push(Fixup {
            kind: if long { FixupKind::Abs32Be } else { FixupKind::Abs16Be },
            offset: 2,
            target: self.partial_fold(&qualified),
        });
        Some(frag)
    }

    /// A 16- or 32-bit immediate operand whose expr is unresolved defers to the
    /// linker as a `Value16Be`/`Value32Be` fixup instead of hard-erroring.
    ///
    /// t30 (game-side G2) extends the original imm32 deferral to size W — the
    /// `move.w #objroutine(Sym), (An)` object-spawn idiom (object_test_state.asm's
    /// `move.w #objroutine(TestStressEmitter), SST_code_addr(a1)` where the effect
    /// object is now `.emp`-owned, so `Sym` is unresolved on the AS side). The
    /// word source immediate is a single extension word (2 bytes) at offset 2 —
    /// the disp-0 `SST_code_addr(a1)` dest folds to `(An)` (mode 2, no dest ext
    /// word) in the same `lower_inst` the eager path uses, so the fixup offset is
    /// 2 exactly as in the imm32 case. `Value16Be` carries the low-16 truncation
    /// (`objroutine` = `Sym - ObjCodeBase`, a bank offset ≤ $FFFF).
    ///
    /// R3 (sound-migration T2, Task 3): a 32-bit immediate operand
    /// (`movea.l #expr,aN` / `move.l #expr,dN`) whose expr is unresolved
    /// defers to the linker as a `Value32Be` fixup instead of hard-erroring —
    /// the imm32 counterpart of `directive_dw`'s `db`/`dw` deferral (T0),
    /// mirroring its "ANY unresolved expression (bare symbol OR compound)
    /// defers" rule. `#imm` is exactly the width-4 case of that Value-kind
    /// family; the address kinds (`Abs16Be`/`Abs32Be`, used by `dc.w`/`dc.l`)
    /// are deliberately NOT reused here — those defer ONLY a bare `Expr::Sym`
    /// and hard-error any compound (R-T0.4's asymmetry note), whereas R3
    /// explicitly wants compounds to defer too.
    ///
    /// Scoped to destination shapes where the immediate's extension words are
    /// PROVABLY the only ones BEFORE the destination's own (so the fixup
    /// offset — immediately after the 2-byte opcode — is correct without
    /// relying on the general `encode_move`/`encode_movea` source-before-dest
    /// extension-word ordering): a bare `aN` (`movea.l`) or bare `dN`
    /// (`move.l`) destination contributes ZERO extension words of its own; an
    /// absolute `(abs).w`/`(abs).l` destination (port #1 hblank: the real
    /// `move.l #HBlank_Null, (HBlank_Handler_Ptr).w` shape) contributes its
    /// OWN extension word(s) strictly AFTER the immediate's four bytes (m68k
    /// encodes source extension words before destination extension words),
    /// so the fixup offset is still exactly 2 — verified against the real
    /// reference encoding `21FC 0000228E 8022` (s4.lst:5794: opcode, 4-byte
    /// imm32, then the abs.w dest ext word). Any other destination
    /// (`(d16,An)`, `(An)+`, `(d8,An,Xn)`, `(d16,PC)`, ...) falls through to
    /// the normal eager path unmodified — resolved operands there are
    /// untouched, and an unresolved one still hard-errors on the converged
    /// pass exactly as before.
    ///
    /// Returns `None` when this isn't a deferrable shape (resolved value, not
    /// `Movea`/`Move`, not size `L`, or a destination outside the three above)
    /// so the caller falls through to the existing eager path unchanged.
    ///
    /// Encoding goes through the REAL encoder with a placeholder-0 immediate
    /// (`Imm(0)` at `.l` emits two zero extension words — the hole — right
    /// after the opcode word), then the fixup is attached at offset 2: the
    /// same pattern as the backend's `lower_branch`/`lower_dbcc`/
    /// `lower_pcrel_ea`, keeping the m68k opcode bit-layout in `sigil-isa`
    /// rather than duplicated here. The absolute destination's own address is
    /// resolved EAGERLY here (`fold_imm`) — only the source immediate defers;
    /// an absolute destination that is ITSELF unresolved still falls through
    /// to the eager path (and hard-errors there, unchanged), since `movea`
    /// only ever takes a register destination and a `move.l` destination
    /// address is not the cross-seam shape this deferral exists for.
    fn try_defer_long_imm(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<DataFragment> {
        if size != M68kSize::L && size != M68kSize::W {
            return None;
        }
        // A bare register destination classifies as `RegOrCond` (the Z80
        // register/cond word list — `sp`, the a7 alias, lands here) OR falls
        // through to `Value(Expr::Sym(_))` (68k `aN`/`dN` aren't Z80 words) —
        // `convert_one_atom_m68k`'s `OperandAtom::Value(e @ Expr::Sym(name))`
        // arm handles the same dual shape for register operands generally.
        // An absolute destination (`(abs).w`/`(abs).l`) is `M68kAbs` —
        // `move.l` only (an absolute `movea` destination isn't a legal 68k
        // shape; `m68k_addr_reg`/`m68k_data_reg` below reject non-register
        // names regardless, so this arm is naturally Move-only in practice).
        // REGISTER destinations stay LONG-only (R3's original scope); the t30 W
        // extension is scoped to MEMORY destinations (the object-spawn shape). A
        // word immediate into a register (`move.w #Sym, d0`) falls through to the
        // eager path and errors as before.
        let (imm_expr, dst) = match atoms {
            [OperandAtom::Imm(e), OperandAtom::RegOrCond(w)] if size == M68kSize::L => {
                let dst = match mnemonic {
                    M68kMnemonic::Movea => M68kOperand::An(m68k_addr_reg(w)?),
                    // `move.l #imm, sp` lands here and MUST stay None:
                    // `m68k_data_reg("sp")` is None, so it falls to the eager
                    // path, which encodes the movea form. Accepting `sp` here
                    // would mis-encode it as a Dn destination.
                    M68kMnemonic::Move => M68kOperand::Dn(m68k_data_reg(w)?),
                    _ => return None,
                };
                (e, dst)
            }
            [OperandAtom::Imm(e), OperandAtom::Value(Expr::Sym(w))] if size == M68kSize::L => {
                let dst = match mnemonic {
                    M68kMnemonic::Movea => M68kOperand::An(m68k_addr_reg(w)?),
                    M68kMnemonic::Move => M68kOperand::Dn(m68k_data_reg(w)?),
                    _ => return None,
                };
                (e, dst)
            }
            [OperandAtom::Imm(e), OperandAtom::M68kAbs { addr, long }] if mnemonic == M68kMnemonic::Move && size == M68kSize::L => {
                // The destination address resolves EAGERLY (it's not the
                // cross-seam leaf this deferral targets) — an unresolved
                // absolute destination falls through to the eager path.
                let qualified_addr = self.qualify_expr(addr);
                let v = self.fold_imm(&qualified_addr, span, i32::MIN as i64, u32::MAX as i64);
                let dst = if *long {
                    M68kOperand::AbsL(v as i32)
                } else {
                    M68kOperand::AbsW((v & 0xFFFF) as i16)
                };
                (e, dst)
            }
            [OperandAtom::Imm(e), OperandAtom::M68kDisp { disp, an }] if mnemonic == M68kMnemonic::Move => {
                // `move.l #Sym, d16(An)` — the tranche-4 particle_anims
                // consumer shape (`move.l #Ani_Particle, SST_anim_table(a0)`,
                // test_particle.asm). The displacement resolves EAGERLY (an
                // SST_* equ, not the cross-seam leaf); m68k orders SOURCE
                // extension words before the destination's, so the imm32
                // hole stays at offset 2 and the d16 ext word follows it —
                // the same offset proof as the absolute arm (encoding:
                // opcode word, 4-byte imm32, d16 word).
                let n = m68k_addr_reg(an)?;
                let qd = self.qualify_expr(disp);
                let d = self.fold_imm(&qd, span, i16::MIN as i64, i16::MAX as i64);
                // Zero-offset fold: `Sym(a1)` with Sym == 0 (e.g. SST_code_addr)
                // encodes (An) mode 2 — no dest ext word — exactly as asl's
                // zeroOffsetOptimization does on the eager path. That fold runs
                // AFTER lower_inst, which the deferred frag skips, so fold here or
                // the imm hole ships an extra $0000 disp word (+2 bytes, ROM drift).
                let dst = if d == 0 { M68kOperand::Ind(n) } else { M68kOperand::Disp16An(d as i16, n) };
                (e, dst)
            }
            [OperandAtom::Imm(e), OperandAtom::M68kInd(reg)] if mnemonic == M68kMnemonic::Move && size == M68kSize::W => {
                // `move.w #Sym, (An)` — the literal `(a1)` object-spawn dest (mode
                // 2, no dest ext word). W-only: the L (An) memory dest deliberately
                // stays loud (R3 scoped move.l deferral to register/abs/disp).
                let n = m68k_addr_reg(reg)?;
                (e, M68kOperand::Ind(n))
            }
            _ => return None,
        };
        let qualified = self.qualify_expr(imm_expr);
        // Unresolved (Poison) exprs ALWAYS defer. A RESOLVED immediate that names
        // a section LABEL also defers on the deferral pass (`keep_labels_symbolic`):
        // `move.l #Label` bakes the label's VMA, which a width-grown `JmpJsrSym`
        // would shift out from under — so carry the label symbolically and let
        // the linker fill it post-relax. A resolved NON-label immediate takes the
        // existing eager path (byte-identical), and on every ordinary pass this is
        // exactly the pre-existing Poison-only rule.
        let is_poison = matches!(self.fold(&qualified), Fold::Poison);
        let refs_label = self.keep_labels_symbolic() && self.expr_refs_label(&qualified);
        if !is_poison && !refs_label {
            return None;
        }
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops: vec![M68kOperand::Imm(0), dst],
        };
        // These shapes always encode (Imm source + a legal dest for the
        // mnemonic); an Err here would be an isa bug, and falling through to
        // the eager path still fails loud (poison ref + the same encode
        // error), never silent.
        let mut frag = self.m68k.lower_inst(&inst, span).ok()?;
        let (kind, min_len) = if size == M68kSize::L {
            (FixupKind::Value32Be, 6)
        } else {
            (FixupKind::Value16Be, 4)
        };
        debug_assert!(
            frag.bytes.len() >= min_len,
            "move(a).{{w,l}} #imm,dest must encode to at least opcode word + the immediate"
        );
        frag.fixups.push(Fixup {
            kind,
            offset: 2,
            // Bake env-resolvable subterms; defer only the true cross-seam leaf
            // (mirrors the `db`/`dw` deferral — a compound imm32 with an
            // env-only equ subterm must not ship that equ as a `Sym`). On the
            // deferral pass a section-LABEL subterm ALSO stays symbolic
            // (`relax_safe_fold`) so a width-growth shift is honored at link.
            target: if refs_label {
                self.relax_safe_fold(&qualified)
            } else {
                self.partial_fold(&qualified)
            },
        });
        Some(frag)
    }

    /// `bra`/`bsr`/`Bcc <target>`: Aeon pins the branch width by an explicit
    /// `.s`/`.w` suffix (no relaxation), so `suffix_size` MUST be present and
    /// MUST be `S` or `W`. The target is qualified (`.local` → `Scope.local`)
    /// and `$`-resolved, then handed to the backend's `lower_branch`, which
    /// builds the opcode + a `PcRel8`/`PcRelDisp16` fixup for the linker.
    fn lower_m68k_branch(
        &mut self,
        mnemonic: M68kMnemonic,
        suffix_size: Option<M68kSize>,
        rest: &[Token],
        span: Span,
    ) {
        let size = match suffix_size {
            Some(s @ (M68kSize::S | M68kSize::W)) => s,
            Some(_) => {
                self.err(span, "branch size suffix must be `.s` or `.w`");
                return;
            }
            None => {
                self.err(span, "branch needs an explicit size suffix (.s or .w) — Aeon pins branch width, no relaxation");
                return;
            }
        };
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let target = match atoms.as_slice() {
            [OperandAtom::Value(e)] => self.fixup_target(e),
            _ => {
                self.err(span, "branch needs a single label target");
                return;
            }
        };
        let frag = self.m68k.lower_branch(mnemonic, size, target, span);
        self.emit_frag(frag, span);
    }

    /// `Dbcc Dn,target` (`dbf`/`dbra`/`db<cc>`): always a fixed 4-byte
    /// instruction (opcode word + a 16-bit displacement word) — NEVER
    /// relaxed, unlike `jmp`/`jsr`'s abs.w/abs.l choice. Because the byte
    /// width never depends on the resolved displacement, the displacement
    /// can safely FOLD immediately here (through the front-end's normal
    /// multi-pass symbol convergence) rather than deferring to a linker
    /// fixup — a forward reference just resolves on a later pass, the same
    /// way a forward `equ` does, and the placeholder `0` byte-pattern is
    /// stable meanwhile. asl measures the displacement from the extension
    /// word's own address (`instruction_start + 2` — the same PC-ref
    /// convention `FixupKind::PcRelDisp16` documents for `bra.w`/`Bcc.w`),
    /// confirmed against `crates/sigil-isa/tests/corpus_m68k/mod.rs`
    /// (`"dbf d0,*"` / `"dbeq d1,*"` both fold to `Disp(-2)`, i.e.
    /// `self_address - (self_address + 2)`) and against real `asl` (see
    /// `m68k_dbf_d0_self`/`m68k_dbeq_d1_self` in `tests/snippets_golden.txt`).
    fn lower_m68k_dbcc(&mut self, mnemonic: M68kMnemonic, rest: &[Token], span: Span) {
        let atoms = match parse_operands(rest) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let (dn_name, target_expr) = match atoms.as_slice() {
            [OperandAtom::Value(Expr::Sym(dn)), OperandAtom::Value(t)] => (dn.clone(), t.clone()),
            _ => {
                self.err(span, "Dbcc needs `Dn,target` operands");
                return;
            }
        };
        let dn = match m68k_data_reg(&dn_name) {
            Some(n) => n,
            None => {
                self.err(
                    span,
                    format!("`{dn_name}` is not a valid data register in Dbcc"),
                );
                return;
            }
        };
        let target = self.resolve_dollar(&self.qualify_expr(&target_expr));
        let pc_of_disp_word = Expr::Int((self.here() + 2) as i64);
        let disp_expr = Expr::Binary {
            op: BinOp::Sub,
            lhs: Box::new(target),
            rhs: Box::new(pc_of_disp_word),
        };
        let d = self.fold_imm(&disp_expr, span, i16::MIN as i64, i16::MAX as i64);
        let inst = M68kInstruction {
            mnemonic,
            size: M68kSize::W,
            ops: vec![M68kOperand::Dn(dn), M68kOperand::Disp(d as i32)],
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// `movem.<w|l> <reglist>,<ea>` (STORE) / `movem.<w|l> <ea>,<reglist>`
    /// (LOAD). Exactly one operand is a register list (`d0-d7/a0-a6`, `a2/d2`,
    /// `d0-d3`, a single reg, or a range crossing the d→a boundary like
    /// `d0-a4`); the other is the memory EA. The register list is built into a
    /// CANONICAL mask (bit0=D0..bit7=D7, bit8=A0..bit15=A7) here, in operand
    /// order; the `-(An)` predecrement 16-bit mask REVERSAL is the encoder's
    /// job (`encode_movem`), never the front-end's — asl-verified: for
    /// `movem.l a2/d2,-(sp)` the front-end emits the canonical `RegList(0x0404)`
    /// and asl's bytes are `48 E7 20 20` (= `reverse_bits(0x0404)`), so the
    /// reversal must NOT be pre-applied here. Size is mandatory (`.w`/`.l`).
    fn lower_m68k_movem(&mut self, suffix_size: Option<M68kSize>, rest: &[Token], span: Span) {
        let size = match suffix_size {
            Some(s @ (M68kSize::W | M68kSize::L)) => s,
            Some(_) => {
                self.err(span, "movem is word (.w) or long (.l) only");
                return;
            }
            None => {
                self.err(span, "movem needs an explicit size suffix (.w or .l)");
                return;
            }
        };
        let groups = split_top_commas(rest);
        if groups.len() != 2 {
            self.err(
                span,
                "movem needs two operands: a register list and a memory EA",
            );
            return;
        }
        let list0 = parse_reg_list(groups[0]);
        let list1 = parse_reg_list(groups[1]);
        // The register list is whichever operand parses as one; the OTHER is the
        // memory EA. Operand ORDER selects direction (STORE vs LOAD), so it is
        // preserved. If both or neither parse, the form is malformed.
        let (mask, list_first, mem_toks) = match (list0, list1) {
            (Some(m), None) => (m, true, groups[1]),
            (None, Some(m)) => (m, false, groups[0]),
            (Some(_), Some(_)) => {
                self.err(
                    span,
                    "movem needs a memory EA operand, got two register lists",
                );
                return;
            }
            (None, None) => {
                self.err(
                    span,
                    "movem needs a register-list operand (e.g. `d0-d7/a0-a6`)",
                );
                return;
            }
        };
        let mem_atoms = match parse_operands(mem_toks) {
            Ok(a) => a,
            Err(d) => {
                self.diags.push(d);
                return;
            }
        };
        let mem_atom = match mem_atoms.as_slice() {
            [a] => a,
            _ => {
                self.err(span, "movem memory operand must be a single EA");
                return;
            }
        };
        let mut mem_op = match self.convert_one_atom_m68k(mem_atom, size, span) {
            Some(o) => o,
            None => return,
        };
        // `movem` reaches its memory EA via this path (not `convert_atoms_m68k`),
        // so apply the zero-disp collapse here too — asl optimizes `movem.l
        // d0-d7,0(a0)` → `48D0` (mode 2), probe-verified. `movem` is never
        // `movep`, so the collapse is unconditional.
        collapse_zero_disp(&mut mem_op);
        let ops = if list_first {
            vec![M68kOperand::RegList(mask), mem_op]
        } else {
            vec![mem_op, M68kOperand::RegList(mask)]
        };
        let inst = M68kInstruction {
            mnemonic: M68kMnemonic::Movem,
            size,
            ops,
        };
        let frag = self.m68k.lower_inst(&inst, span);
        self.emit_frag(frag, span);
    }

    /// An instruction with a `(d16,PC)` source operand (any mnemonic: `move`,
    /// `tst`, `cmp`, ...). `pc_idx` is the index of that atom within `atoms`
    /// (already located by the caller). `(d16,PC)` is illegal as a
    /// DESTINATION EA (`encode_ea` rejects it there — real 68k only reads
    /// through PC-relative), so wherever it legally appears it is the single
    /// EA operand of a 1-operand form or the SOURCE of a 2-operand form; both
    /// `encode_move`/`encode_alu_ea`/`encode_control`/etc. process the source
    /// EA's extension words first (right after the 2-byte opcode word), so
    /// the `(d16,PC)` extension word always starts at byte offset 2 —
    /// confirmed against `lower_pcrel_ea`'s own unit test (`lea (d16,PC),a0`)
    /// and against real asl (`m68k_move_w_pcd16_to_d0` in
    /// `tests/snippets_golden.txt`).
    fn lower_m68k_pcrel(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        pc_idx: usize,
        span: Span,
    ) {
        let mut ops = Vec::with_capacity(atoms.len());
        let mut target = None;
        for (i, a) in atoms.iter().enumerate() {
            if i == pc_idx {
                let disp = match a {
                    OperandAtom::M68kDisp { disp, .. } => disp,
                    _ => unreachable!("pc_idx must index a M68kDisp{{an: \"pc\"}} atom"),
                };
                target = Some(self.fixup_target(disp));
                ops.push(M68kOperand::Pcd16(0));
            } else {
                match self.convert_one_atom_m68k(a, size, span) {
                    Some(op) => ops.push(op),
                    None => return,
                }
            }
        }
        let target = target.expect("pc_idx must index the pc-relative atom");
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_pcrel_ea(&inst, 2, target, span);
        self.emit_frag(frag, span);
    }

    /// An instruction with a `(d8,PC,Xn)` source operand (`Label(pc,Xn.w|.l)`,
    /// e.g. jump-table reads `.case_table(pc,d2.w)`). Mirrors
    /// [`Self::lower_m68k_pcrel`] but for the brief-extension-word indexed form:
    /// the pc-idx atom's `disp` is the label target (resolved later as an 8-bit
    /// PC-relative displacement), and its index register becomes the ext word's
    /// `Xn`. The disp8 byte sits at offset 3 (opcode word + ext-word high byte).
    fn lower_m68k_pcrel_idx(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        pc_idx: usize,
        span: Span,
    ) {
        let mut ops = Vec::with_capacity(atoms.len());
        let mut target = None;
        for (i, a) in atoms.iter().enumerate() {
            if i == pc_idx {
                let (disp, xn, xlong) = match a {
                    OperandAtom::M68kIdx {
                        disp, xn, xlong, ..
                    } => (disp, xn, *xlong),
                    _ => unreachable!("pc_idx must index a M68kIdx{{an: \"pc\"}} atom"),
                };
                let xn = match self.m68k_index_reg(xn, span) {
                    Some(x) => x,
                    None => return,
                };
                target = Some(self.fixup_target(disp));
                ops.push(M68kOperand::Pcd8Xn {
                    d: 0,
                    xn,
                    long: xlong,
                });
            } else {
                match self.convert_one_atom_m68k(a, size, span) {
                    Some(op) => ops.push(op),
                    None => return,
                }
            }
        }
        let target = target.expect("pc_idx must index the pc-relative atom");
        let mnemonic = refine_m68k_mnemonic(mnemonic, &ops);
        let inst = M68kInstruction {
            mnemonic,
            size,
            ops,
        };
        let frag = self.m68k.lower_pcrel_idx_ea(&inst, 3, target, span);
        self.emit_frag(frag, span);
    }

    /// Parse a 68k index-register name (`d0`..`d7` / `a0`..`a7`, `sp` = a7) into
    /// the ISA's `Xn`. Diagnoses (and returns `None`) on a non-register token.
    fn m68k_index_reg(&mut self, xn: &str, span: Span) -> Option<M68kXn> {
        if let Some(n) = m68k_data_reg(xn) {
            Some(M68kXn::D(n))
        } else if let Some(n) = m68k_addr_reg(xn) {
            Some(M68kXn::A(n))
        } else {
            self.err(span, format!("`{xn}` is not a valid index register"));
            None
        }
    }

    /// Convert operand atoms to resolved 68k operands for the fold-based
    /// (no-fixup) core: `Dn`/`An`/`Imm` (plus bare `sr`/`ccr`), the
    /// register-indirect family, and explicit-width absolute (`M68kAbs`).
    /// Any width-selecting bare-`(expr)` or `(d8,PC,Xn)` atom is rejected
    /// with a diagnostic (the latter is the only PC-relative form still
    /// unsupported — see [`Self::lower_m68k_pcrel`] for `(d16,PC)`).
    fn convert_atoms_m68k(
        &mut self,
        mnemonic: M68kMnemonic,
        size: M68kSize,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Vec<M68kOperand>> {
        let mut ops = Vec::with_capacity(atoms.len());
        for a in atoms {
            ops.push(self.convert_one_atom_m68k(a, size, span)?);
        }
        // asl zero-displacement optimization — `movep` is the sole 68000
        // exception (see `collapse_zero_disp`).
        if mnemonic != M68kMnemonic::Movep {
            for op in &mut ops {
                collapse_zero_disp(op);
            }
        }
        Some(ops)
    }

    /// Lower a bare (unsuffixed) absolute-address EA operand — a symbol or an
    /// expression used where a 68k EA is expected, e.g. `lea Sym, a0` or
    /// `move.w Sym, d0`. asl width-selects abs.w/abs.l via `asl_width_rule`
    /// (probe-verified EA-general in M1.D T2). We fold + select in the front
    /// end (the T3 width-selection mechanism for the absolute-EA class), so the
    /// instruction's Data fragment carries the true encoded length and the
    /// multi-pass fixpoint converges. Uses `self.fold` (not `fold_imm`): an
    /// unresolved-this-pass symbol folds to Poison → optimistic abs.w (M1.D T3
    /// probe: asl selects the least fixpoint for the absolute-EA class — `lea` at
    /// $7FFA → 41F8 7FFE, abs.w). The multi-pass loop then only ever grows a
    /// width W→L, converging to asl's minimal width, so realistic forward refs
    /// converge. See docs/superpowers/notes/2026-07-04-m1d-t3-jmpjsr-width-probes.md.
    /// (`asl_width_rule` is non-monotonic at the $FF8000 sign-extension wrap —
    /// see the grow-only caveat in `sigil-link/relax.rs`; that region is
    /// immediately-resolved high-RAM constants in Aeon, and `PASS_CAP` backstops
    /// any pathological oscillation.) The name is recorded in `poison_refs` so a
    /// genuinely-undefined symbol still errors on the converged pass.
    fn abs_ea_from_expr(&mut self, e: &Expr, span: Span) -> M68kOperand {
        let qualified = self.qualify_expr(e);
        match self.fold(&qualified) {
            Fold::Value(v) => match asl_width_rule(v, false) {
                AbsWidth::W => M68kOperand::AbsW((v & 0xFFFF) as i16),
                AbsWidth::L => M68kOperand::AbsL(v as i32),
            },
            Fold::Poison => {
                for name in self.unresolved_names(&qualified) {
                    self.poison_refs.push((name, span));
                }
                // Optimistic abs.w while unresolved (M1.D T3): asl selects the least
                // fixpoint for the absolute-EA class too (probe: lea at $7FFA → 41F8
                // 7FFE, abs.w). The multi-pass loop then only grows W→L, converging
                // to asl's minimal width. The converged pass re-folds to the real
                // value (or errors via poison_refs above).
                M68kOperand::AbsW(0)
            }
        }
    }

    /// Convert one operand atom (see [`Self::convert_atoms_m68k`]).
    fn convert_one_atom_m68k(
        &mut self,
        a: &OperandAtom,
        size: M68kSize,
        span: Span,
    ) -> Option<M68kOperand> {
        Some(match a {
            OperandAtom::Imm(e) => {
                let (lo, hi) = m68k_imm_bounds(size);
                let v = self.fold_imm(e, span, lo, hi);
                M68kOperand::Imm(v as i32)
            }
            OperandAtom::RegOrCond(w) => {
                if let Some(n) = m68k_addr_reg(w) {
                    M68kOperand::An(n)
                } else if let Some(n) = m68k_data_reg(w) {
                    M68kOperand::Dn(n)
                } else {
                    self.err(
                        span,
                        format!("`{w}` is not a valid 68k register in this context"),
                    );
                    return None;
                }
            }
            OperandAtom::Value(e @ Expr::Sym(name)) => {
                if let Some(n) = m68k_data_reg(name) {
                    M68kOperand::Dn(n)
                } else if let Some(n) = m68k_addr_reg(name) {
                    M68kOperand::An(n)
                } else if name == "sr" {
                    M68kOperand::Sr
                } else if name == "ccr" {
                    M68kOperand::Ccr
                } else if name == "usp" {
                    M68kOperand::Usp
                } else {
                    // Bare symbol in EA position = absolute address; asl
                    // width-selects abs.w/abs.l (M1.D T2).
                    self.abs_ea_from_expr(e, span)
                }
            }
            OperandAtom::Value(e) => {
                // Bare numeric/expression operand = 68k absolute addressing;
                // width-selected like the bare-symbol case above (M1.D T2).
                self.abs_ea_from_expr(e, span)
            }
            OperandAtom::Mem(_) => {
                self.err(
                        span,
                        "absolute address operand `(expr)` needs an explicit `.w`/`.l` width suffix (width-selecting bare `(expr)` is out of scope)",
                    );
                return None;
            }
            OperandAtom::M68kAbs { addr, long } => {
                let qualified = self.qualify_expr(addr);
                let v = self.fold_imm(&qualified, span, i32::MIN as i64, u32::MAX as i64);
                if *long {
                    M68kOperand::AbsL(v as i32)
                } else {
                    M68kOperand::AbsW((v & 0xFFFF) as i16)
                }
            }
            // `(sp)` is the `a7` alias but lexes down the pre-existing Z80
            // `hl`/`bc`/`de`/`sp` branch (see `classify`), not `M68kInd`.
            OperandAtom::IndReg(w) if w == "sp" => M68kOperand::Ind(7),
            OperandAtom::IndReg(w) => {
                self.err(
                    span,
                    format!("`({w})` is not a valid 68k address-register indirect operand"),
                );
                return None;
            }
            OperandAtom::Indexed { .. } => {
                self.err(
                    span,
                    "z80 `(ix±d)`/`(iy±d)` indexed form is not a valid 68k operand",
                );
                return None;
            }
            OperandAtom::M68kPreDec(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::PreDec(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `-(An)`"),
                    );
                    return None;
                }
            },
            OperandAtom::M68kPostInc(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::PostInc(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `(An)+`"),
                    );
                    return None;
                }
            },
            OperandAtom::M68kInd(reg) => match m68k_addr_reg(reg) {
                Some(n) => M68kOperand::Ind(n),
                None => {
                    self.err(
                        span,
                        format!("`{reg}` is not a valid address register in `(An)`"),
                    );
                    return None;
                }
            },
            OperandAtom::M68kDisp { disp, an } => {
                let n = match m68k_addr_reg(an) {
                    Some(n) => n,
                    None => {
                        self.err(span, m68k_disp_an_error(an));
                        return None;
                    }
                };
                let d = self.fold_imm(disp, span, i16::MIN as i64, i16::MAX as i64);
                M68kOperand::Disp16An(d as i16, n)
            }
            OperandAtom::M68kIdx {
                disp,
                an,
                xn,
                xlong,
            } => {
                let an_n = match m68k_addr_reg(an) {
                    Some(n) => n,
                    None => {
                        self.err(span, m68k_disp_an_error(an));
                        return None;
                    }
                };
                let xn_op = if let Some(n) = m68k_data_reg(xn) {
                    M68kXn::D(n)
                } else if let Some(n) = m68k_addr_reg(xn) {
                    M68kXn::A(n)
                } else {
                    self.err(
                        span,
                        format!("`{xn}` is not a valid index register in `(d,An,Xn)`"),
                    );
                    return None;
                };
                let d = self.fold_imm(disp, span, i8::MIN as i64, i8::MAX as i64);
                M68kOperand::Disp8AnXn {
                    d: d as i8,
                    an: an_n,
                    xn: xn_op,
                    long: *xlong,
                }
            }
            OperandAtom::AfShadow => {
                self.err(span, "`af'` is not a 68k operand");
                return None;
            }
        })
    }

    fn build_operands(
        &mut self,
        m: Mnemonic,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Lowered> {
        if matches!(m, Mnemonic::Jr | Mnemonic::Djnz) {
            let (cond, target_atom) = match atoms {
                [OperandAtom::RegOrCond(w), t] => (cond_word(w), t),
                [t] => (None, t),
                _ => {
                    self.err(span, "bad jr/djnz operands");
                    return None;
                }
            };
            let target = match target_atom {
                OperandAtom::Value(e) => self.resolve_dollar(&self.qualify_expr(e)),
                _ => {
                    self.err(span, "jr/djnz needs a label target");
                    return None;
                }
            };
            return Some(Lowered::Rel(cond, target));
        }
        if matches!(m, Mnemonic::Jp | Mnemonic::Call) {
            let (cond, target_opt) = self.split_control_target(atoms);
            if let Some(target) = target_opt {
                if matches!(target, Expr::Sym(_)) {
                    let mut ops = Vec::new();
                    if let Some(cc) = cond {
                        ops.push(Operand::Cc(cc));
                    }
                    return Some(match self.fold(&target) {
                        Fold::Value(v) => {
                            ops.push(Operand::Imm16(v as u16));
                            Lowered::Fixed(ops)
                        }
                        Fold::Poison => {
                            ops.push(Operand::Imm16(0));
                            Lowered::Abs16(ops, target)
                        }
                    });
                }
            }
        }
        if matches!(m, Mnemonic::Ld) {
            if let [OperandAtom::RegOrCond(w), OperandAtom::Value(e @ Expr::Sym(_))] = atoms {
                if let Some(rr) = reg16(w) {
                    let target = self.qualify_expr(e);
                    return Some(match self.fold(&target) {
                        Fold::Value(v) => {
                            Lowered::Fixed(vec![Operand::Pair(rr), Operand::Imm16(v as u16)])
                        }
                        Fold::Poison => {
                            Lowered::Abs16(vec![Operand::Pair(rr), Operand::Imm16(0)], target)
                        }
                    });
                }
            }
        }
        let ops = self.convert_atoms(m, atoms, span)?;
        Some(Lowered::Fixed(ops))
    }

    /// For jp/call: split off a leading condition and return the bare target expr.
    fn split_control_target(&self, atoms: &[OperandAtom]) -> (Option<Cond>, Option<Expr>) {
        match atoms {
            [OperandAtom::RegOrCond(w), OperandAtom::Value(e)] if cond_word(w).is_some() => {
                (cond_word(w), Some(self.qualify_expr(e)))
            }
            [OperandAtom::Value(e)] => (None, Some(self.qualify_expr(e))),
            _ => (None, None),
        }
    }

    /// Replace `$` (current-PC) sub-expressions with a concrete Int so the
    /// relative-jump fixup carries a resolvable target. Other symbols stay
    /// symbolic so real (possibly forward) labels still take the fixup path.
    /// Mirrors `fold`'s rule that `$` never survives as a Sym fixup target.
    fn resolve_dollar(&self, e: &Expr) -> Expr {
        match e {
            Expr::Sym(name) if name == "$" => Expr::Int(self.here() as i64),
            Expr::Binary { op, lhs, rhs } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.resolve_dollar(lhs)),
                rhs: Box::new(self.resolve_dollar(rhs)),
            },
            Expr::Unary { op, operand } => Expr::Unary {
                op: *op,
                operand: Box::new(self.resolve_dollar(operand)),
            },
            other => other.clone(),
        }
    }

    /// Qualify every `.`-local `Sym` in the tree against the current scope.
    /// RECURSES into compound expressions (mirroring `resolve_dollar`): a `.`-local
    /// can sit nested inside arithmetic — jump-table targets like `.cc_table-4(pc,…)`
    /// and computed branch targets like `.drain_end-.c*8` — and each nested local
    /// must qualify, or the linker's global-scope fold can never resolve it.
    fn qualify_expr(&self, e: &Expr) -> Expr {
        match e {
            Expr::Sym(name) if name.starts_with('.') => {
                Expr::Sym(qualify(name, self.dot_scope(name)))
            }
            Expr::Binary { op, lhs, rhs } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.qualify_expr(lhs)),
                rhs: Box::new(self.qualify_expr(rhs)),
            },
            Expr::Unary { op, operand } => Expr::Unary {
                op: *op,
                operand: Box::new(self.qualify_expr(operand)),
            },
            other => other.clone(),
        }
    }

    /// PARTIALLY fold a fixup target expression: rewrite every subterm that
    /// resolves in the CURRENT AS env to its `Expr::Int`, leaving only the
    /// genuinely-unresolved symbols (typically the sole `.emp`-side cross-seam
    /// label) as `Sym`. Used on the deferral paths (`db`/`dw`/imm32) so a
    /// compound target like `sfx_winptr(Sfx_33)` = `(Sfx_33 & SFX_WIN_MASK) |
    /// SFX_WIN_BASE` bakes the env-only constants `SFX_WIN_MASK`/`SFX_WIN_BASE`
    /// (asl `=` equs the linker's section-label table cannot see) HERE, so the
    /// linker fold sees `(Sfx_33 & 32767) | 32768` — only `Sfx_33` deferred.
    ///
    /// This is the deferred-expr analogue of `fixup_target`'s bake-what-you-can
    /// rationale (the env-only `set`/`equ` note there): whole-expr folding is not
    /// enough when ONE leaf is external — the resolvable leaves must still be
    /// captured at this site, not shipped as `Sym`s the linker can't resolve.
    /// A whole expr that folds is returned as a single `Expr::Int`; a fully
    /// external leaf is returned unchanged.
    ///
    /// No-drift argument, stated directly: the subterms we bake here are BY
    /// CONSTRUCTION AS-env-resolvable — local `=`/`equ`/label values that live
    /// only in this evaluator's env and the linker's section-label table never
    /// sees. So baking one can never shadow a linker-visible section label:
    /// the two namespaces are disjoint, and only the still-external leaves (the
    /// ones this fold leaves untouched) reach the linker at all.
    fn partial_fold(&self, e: &Expr) -> Expr {
        // If the WHOLE subtree folds, collapse it to a literal.
        if let Fold::Value(v) = self.fold(e) {
            return Expr::Int(v);
        }
        // Otherwise recurse, folding the resolvable branches.
        match e {
            Expr::Binary { op, lhs, rhs } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.partial_fold(lhs)),
                rhs: Box::new(self.partial_fold(rhs)),
            },
            Expr::Unary { op, operand } => Expr::Unary {
                op: *op,
                operand: Box::new(self.partial_fold(operand)),
            },
            // A leaf that didn't fold above is genuinely external — keep it.
            other => other.clone(),
        }
    }

    /// Resolve a PC-relative branch / jump-table target destined for a fixup.
    /// Qualifies `.`-locals (deep), resolves `$`, then FOLDS against the current
    /// env and BAKES a resolved target as `Expr::Int` — mirroring the jmp/jsr and
    /// `abs_ea_from_expr` bake (M1.D T3). Baking is required, not just tidy: the
    /// target may reference an env-only `set`/`equ` symbol the linker's
    /// section-label table cannot see (`.c`, a per-iteration `rept` counter in
    /// `dma_queue.asm`'s `bra.w .drain_end-.c*8`), and the counter's value must be
    /// captured HERE (its value at this instruction), not deferred to the linker
    /// where only its final value survives. A still-unresolved (forward) target
    /// stays fully-qualified-symbolic for the linker to resolve or reject — the
    /// branch width is fixed, so the placeholder never perturbs layout.
    fn fixup_target(&self, e: &Expr) -> Expr {
        let qualified = self.resolve_dollar(&self.qualify_expr(e));
        // On the deferral (bonus) pass, keep any subterm that names a section
        // LABEL symbolic instead of baking its this-pass VMA — see
        // `keep_labels_symbolic` / `relax_safe_fold`. This is what makes a
        // PC-relative branch whose target sits past a width-grown `JmpJsrSym`
        // resolve correctly in the combined link.
        if self.keep_labels_symbolic() && self.expr_refs_label(&qualified) {
            return self.relax_safe_fold(&qualified);
        }
        match self.fold(&qualified) {
            Fold::Value(v) => Expr::Int(v),
            Fold::Poison => qualified,
        }
    }

    /// Whether label references in a fixup target must be kept SYMBOLIC rather
    /// than baked to their this-pass VMA. True ONLY on `run`'s deferral (bonus)
    /// pass — the ONE pass that emits a length-variable `Fragment::JmpJsrSym`
    /// (an unresolved cross-seam `jsr`/`jmp`, deferred to the linker's
    /// relaxation ladder). When `resolve_layout` later GROWS such a fragment
    /// abs.w→abs.l, it SHIFTS every following section label — but a baked
    /// absolute constant in a fixup target does not move, so a branch/`dc.l`/
    /// `dc.w`/`jsr` whose target sits past the grown fragment would resolve to a
    /// stale (pre-growth) address. Keeping labels symbolic lets the linker
    /// resolve them against its own relaxation-shifted section-label table.
    ///
    /// On EVERY ordinary pass this is `false`, so baking — and thus byte output
    /// — is UNCHANGED: a non-deferral module has no length-variable AS-side
    /// fragment, so its front-end label VMAs already equal the link-time ones
    /// (the `stale_fold_repro` T3 invariant). The whole behavior change is
    /// scoped to the mixed cross-seam builds that actually relax.
    fn keep_labels_symbolic(&self) -> bool {
        self.defer_unresolved_jsr_jmp
    }

    /// Does `e` reference at least one section LABEL — directly, or through a
    /// label-referencing `equ` (`dc.l HandlerPtr` where `HandlerPtr = Handler`)?
    /// A `$`-derived `Int`, a pure-constant `equ`/`set`, or an unresolved
    /// cross-seam external all answer `false` — only a name the linker resolves
    /// to a RELAXATION-SHIFTABLE address (a section label, or an equ_sym that
    /// folds onto one) counts.
    fn expr_refs_label(&self, e: &Expr) -> bool {
        match e {
            Expr::Sym(name) => {
                self.known_labels.contains(name)
                    || self.label_ref_equs.contains(name)
                    || self.set_sym_symbolic.contains_key(name)
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.expr_refs_label(lhs) || self.expr_refs_label(rhs)
            }
            Expr::Unary { operand, .. } => self.expr_refs_label(operand),
            Expr::Int(_) => false,
        }
    }

    /// Partially fold a fixup target while keeping every section-LABEL reference
    /// symbolic, so the linker resolves it against its relaxation-shifted table
    /// (see `keep_labels_symbolic`). A subtree that references NO label is baked
    /// whole to `Expr::Int` (env-only `equ`/`set`/`$` values the linker cannot
    /// see — captured HERE, exactly as `partial_fold` does); a subtree that DOES
    /// reference a label is descended, folding only its label-free branches and
    /// leaving each label `Sym` in place. `$` must already be resolved out (call
    /// on a `resolve_dollar`-ed expression) — `$` is not a label, so it would
    /// otherwise survive as an unresolvable `Sym`.
    fn relax_safe_fold(&self, e: &Expr) -> Expr {
        // A reassignable set-symbol currently bound to a label: splice in its
        // symbolic snapshot (itself already relax-safe) so the label rides through
        // placement. This also resolves a CHAIN (`P := Q := Label`) — the stored
        // snapshot for `P` is built by folding `Sym("Q")` here, which splices to
        // `Q`'s underlying label expr.
        if let Expr::Sym(name) = e {
            if let Some(sub) = self.set_sym_symbolic.get(name) {
                return sub.clone();
            }
        }
        if !self.expr_refs_label(e) {
            if let Fold::Value(v) = self.fold(e) {
                return Expr::Int(v);
            }
        }
        match e {
            Expr::Binary { op, lhs, rhs } => Expr::Binary {
                op: *op,
                lhs: Box::new(self.relax_safe_fold(lhs)),
                rhs: Box::new(self.relax_safe_fold(rhs)),
            },
            Expr::Unary { op, operand } => Expr::Unary {
                op: *op,
                operand: Box::new(self.relax_safe_fold(operand)),
            },
            other => other.clone(),
        }
    }

    /// Convert operand atoms to resolved z80 operands, by mnemonic.
    fn convert_atoms(
        &mut self,
        m: Mnemonic,
        atoms: &[OperandAtom],
        span: Span,
    ) -> Option<Vec<Operand>> {
        // M0 invariant: a 16-bit pair operand means the immediate is 16-bit (ld rr,nn). Holds for the driver's mnemonic set.
        let has_pair_companion = atoms
            .iter()
            .any(|a| matches!(a, OperandAtom::RegOrCond(w) if reg16(w).is_some()));
        let control_flow = matches!(m, Mnemonic::Jp | Mnemonic::Call | Mnemonic::Ret);
        let bit_op = matches!(m, Mnemonic::Bit | Mnemonic::Res | Mnemonic::Set);
        let mut ops = Vec::with_capacity(atoms.len());
        for (i, a) in atoms.iter().enumerate() {
            let op = match a {
                OperandAtom::RegOrCond(w) => {
                    if control_flow && i == 0 {
                        if let Some(cc) = cond_word(w) {
                            Operand::Cc(cc)
                        } else {
                            self.reg_operand(w, span)?
                        }
                    } else {
                        self.reg_operand(w, span)?
                    }
                }
                OperandAtom::IndReg(w) => match w.as_str() {
                    "hl" => Operand::IndHl,
                    "bc" => Operand::IndBc,
                    "de" => Operand::IndDe,
                    // `ex (sp),hl` — the encoder special-cases [Pair(Sp), Pair(Hl)] -> E3.
                    "sp" if matches!(m, Mnemonic::Ex) => Operand::Pair(Reg16::Sp),
                    _ => {
                        self.err(span, "bad indirect register");
                        return None;
                    }
                },
                OperandAtom::Indexed { reg, disp } => {
                    let d = self.fold_imm(disp, span, -128, 127);
                    Operand::Indexed {
                        reg: *reg,
                        disp: d as i8,
                    }
                }
                OperandAtom::Mem(e) => {
                    let v = self.fold_imm(e, span, 0, 0xFFFF);
                    Operand::Mem(v as u16)
                }
                OperandAtom::Value(e) => {
                    if bit_op && i == 0 {
                        let b = self.fold_imm(e, span, 0, 7);
                        Operand::Bit(b as u8)
                    } else if matches!(m, Mnemonic::Im) {
                        let v = self.fold_imm(e, span, 0, 2);
                        Operand::Imm8(v as u8)
                    } else if matches!(m, Mnemonic::Jp | Mnemonic::Call) {
                        // A literal address for jp/call is a 16-bit immediate
                        // (symbolic targets take the Abs16 fixup path earlier).
                        let v = self.fold_imm(e, span, 0, 0xFFFF);
                        Operand::Imm16(v as u16)
                    } else if has_pair_companion {
                        let v = self.fold_imm(e, span, -0x8000, 0xFFFF);
                        Operand::Imm16(v as u16)
                    } else {
                        let v = self.fold_imm(e, span, -128, 0xFF);
                        Operand::Imm8(v as u8)
                    }
                }
                OperandAtom::AfShadow => Operand::AfShadow,
                OperandAtom::Imm(_) => {
                    // `#imm` is a 68k-only operand form (see `convert_atoms_m68k`);
                    // the z80 lexer never emits a `#` token, so this is unreachable
                    // in practice, but the match must stay exhaustive.
                    self.err(span, "`#` immediate is not valid z80 syntax");
                    return None;
                }
                OperandAtom::M68kPreDec(_)
                | OperandAtom::M68kPostInc(_)
                | OperandAtom::M68kInd(_)
                | OperandAtom::M68kDisp { .. }
                | OperandAtom::M68kIdx { .. }
                | OperandAtom::M68kAbs { .. } => {
                    // These 68k-only EA shapes (see `convert_atoms_m68k`) don't
                    // arise from z80 syntax in practice (`a0`.."a7" aren't z80
                    // register names), but the match must stay exhaustive.
                    self.err(span, "this operand form is not valid z80 syntax");
                    return None;
                }
            };
            ops.push(op);
        }
        Some(ops)
    }

    fn reg_operand(&mut self, w: &str, span: Span) -> Option<Operand> {
        if let Some(r) = reg8(w) {
            Some(Operand::Reg(r))
        } else if let Some(rr) = reg16(w) {
            Some(Operand::Pair(rr))
        } else if w == "i" {
            Some(Operand::RegI)
        } else if w == "r" {
            Some(Operand::RegR)
        } else if let Some(cc) = cond_word(w) {
            Some(Operand::Cc(cc))
        } else {
            self.err(span, format!("bad register/condition `{w}`"));
            None
        }
    }

    fn emit_frag(&mut self, frag: Result<DataFragment, LowerError>, span: Span) {
        match frag {
            Ok(f) => {
                let bytes = f.bytes.clone();
                self.emit(&bytes, f.fixups, span);
            }
            Err(e) => self.err(span, e.to_string()),
        }
    }

    fn emit(&mut self, bytes: &[u8], fixups: Vec<Fixup>, span: Span) {
        // The one place bytes enter the module, so the one place that can ask
        // whether the unit ever said what processor they are for. Every encoded
        // byte is a CPU-dependent decision; producing one against a processor
        // nobody named is the silent failure this refuses.
        if !self.state.cpu_declared {
            self.refuse_undeclared_cpu(span);
            return;
        }
        // The builder advances its own section cursor (the single source of
        // truth read back via `current_offset()`); the front-end keeps none.
        self.builder.emit_data(bytes, fixups, span);
    }

    /// Raise [`CPU_UNDECLARED`] once and abort the pass.
    ///
    /// Aborting matters as much as the diagnostic. Under the provisional
    /// processor a source written for the other one mis-lexes from its first
    /// line, so continuing buries the one true error under a screenful of
    /// consequences of it.
    fn refuse_undeclared_cpu(&mut self, span: Span) {
        if !self.cpu_refused {
            self.cpu_refused = true;
            self.err(span, crate::CPU_UNDECLARED);
        }
        self.aborted = true;
    }

    /// Capture `<name> macro [params] … endm`. Returns the index past `endm`.
    fn capture_macro(&mut self, lines: &[SrcLine], start: usize) -> usize {
        let head = self.subst_frame(&lines[start]);
        let head = head.as_ref().unwrap_or(&lines[start]);
        let toks = lex_line(
            &head.text,
            self.state.cpu,
            lines[start].source,
            lines[start].base,
        )
        .unwrap_or_default();
        // Two head shapes (both real AS, both asl-verified):
        //   `NAME macro p...`   → toks: Ident(NAME) Ident("macro") [params...]
        //   `NAME: macro p...`  → toks: Ident(NAME) Colon Ident("macro") [params...]
        // The colon form (used by the `__FSTRING`/`__ErrorMessage` debug macros)
        // must peel the label before reading params, else `macro` itself leaks in
        // as the first "param" and shifts every real param by one (binding the
        // caller's arg to a phantom slot). `parse_line_tokens` peels it.
        let parsed = parse_line_tokens(&toks);
        let (name, param_toks): (String, Vec<Token>) = if let Some(lbl) = parsed.label_colon {
            // parsed.tokens: Ident("macro") [params...]; params start at index 1.
            (lbl, parsed.tokens.get(1..).unwrap_or(&[]).to_vec())
        } else {
            let name = match toks.first().map(|t| &t.tok) {
                Some(Tok::Ident(s)) => s.clone(),
                _ => {
                    let span = Span {
                        source: lines[start].source,
                        start: lines[start].base,
                        end: lines[start].base,
                    };
                    self.err(span, "macro needs a name");
                    String::new()
                }
            };
            (name, toks.get(2..).unwrap_or(&[]).to_vec())
        };
        let params: Vec<String> = param_toks
            .iter()
            .filter_map(|t| {
                if let Tok::Ident(p) = &t.tok {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        let end = self.find_block_end(lines, start);
        // An UNCLOSED definition. `find_block_end` answers with the last line it
        // scanned, so a head on that line leaves nothing between head and end and
        // the body slice would be inverted. Say so instead: the alternative is a
        // panic, and the way this is reached is not a malformed source file but a
        // pasted expansion-scope name — see [`Asm::bind_macro_arg`].
        if end <= start {
            let span = Span {
                source: lines[start].source,
                start: lines[start].base,
                end: lines[start].base,
            };
            self.err(span, format!("macro `{name}` definition has no `endm`"));
            return lines.len();
        }
        // A macro DEFINED inside an expanding macro body captures text the
        // enclosing expansion has already substituted — including its
        // `ALLARGS`, frozen at the shift state in force here. The inner macro
        // therefore carries the OUTER call's arguments for the rest of the
        // assembly, and its own invocation arguments do not rebind them
        // (asl-verified: probe `p3.asm` case 3e defines `inner2` after a
        // `shift` in `de 61,62,63`, and `inner2 51,52` emits `in<51|62,63>` —
        // the outer's post-shift `ALLARGS`, not the inner's `51,52`).
        let body: Vec<SrcLine> = lines[start + 1..end]
            .iter()
            .map(|l| self.subst_frame(l).unwrap_or_else(|| l.clone()))
            .collect();
        self.dot_label_cache.remove(&name);
        let int_label = head_declares_int_label(&head.text);
        self.macros.insert(name, MacroDef { params, body, int_label });
        end + 1
    }

    /// Expand a macro invocation: substitute `ALLARGS` (verbatim arg text) and
    /// params (positional and/or keyword), then execute the resulting lines.
    ///
    /// Real AS binds params two ways, mixable in one call (asl-verified — see
    /// the `macro_keyword_args` snippet): a comma-split arg of the shape
    /// `NAME=value` binds `NAME` by name, regardless of where it sits in the
    /// call; every other arg fills the remaining (not yet keyword-bound)
    /// params positionally, in declaration order. `tst AMP=7,PER=9`,
    /// `tst 3,4`, and `tst PER=5,AMP=2` (params `AMP,PER`) all bind correctly
    /// under this rule.
    fn expand_macro(&mut self, name: &str, arg_toks: &[Token]) {
        self.expand_macro_inner(name, arg_toks, None);
    }

    /// Expand a `.ATTRIBUTE`-suffix invocation (T9.2): `name` is the BASE
    /// macro (already stripped of its `.SUFFIX` by `dispatch`'s
    /// `split_attribute_suffix` check), `attribute` is the literal suffix
    /// text (`.b`/`.w`/`.l`/`.s`) bound to `.ATTRIBUTE` inside the body.
    fn expand_macro_with_attribute(&mut self, name: &str, arg_toks: &[Token], attribute: &str) {
        self.expand_macro_inner(name, arg_toks, Some(attribute));
    }

    /// Shared implementation: substitute `.ATTRIBUTE` (if this is an
    /// attribute-suffixed call), `ALLARGS` (verbatim arg text), and params
    /// (positional and/or keyword), then execute the resulting lines.
    ///
    /// Real AS binds params two ways, mixable in one call (asl-verified — see
    /// the `macro_keyword_args` snippet): a comma-split arg of the shape
    /// `NAME=value` binds `NAME` by name, regardless of where it sits in the
    /// call; every other arg fills the remaining (not yet keyword-bound)
    /// params positionally, in declaration order. `tst AMP=7,PER=9`,
    /// `tst 3,4`, and `tst PER=5,AMP=2` (params `AMP,PER`) all bind correctly
    /// under this rule.
    ///
    /// `.ATTRIBUTE` is substituted with a plain (unbounded) literal-text
    /// match, the same way `ALLARGS` is — NOT the parameters'
    /// identifier-boundary match, because `.ATTRIBUTE` is deliberately used
    /// glued onto a mnemonic (`move.ATTRIBUTE`, one lexed ident) as well as
    /// standalone in a string; a boundary check keyed on `is_alphanumeric`
    /// would reject the glued-mnemonic case (the char right before the `.` is
    /// alphanumeric, e.g. the `e` in `move`), which is the primary asl-verified
    /// use (asl-verified: `move.ATTRIBUTE src,d0` with `foo.w d1` → `move.w d1,d0`).
    /// Bind one macro argument, qualifying a bare `.`-local against the CALLER
    /// scope (asl evaluates arguments in caller context — [`Self::scope`] still
    /// holds that scope here, before the expansion swaps in its own).
    ///
    /// A `.`-local that names a **string-valued** `set` symbol is substituted BY
    /// VALUE, as a quoted literal: the qualified name `" macro#N.local"` (space +
    /// `#`) can't re-lex as a single identifier, so `switch`/`lowstring`/`substr`
    /// in the callee couldn't resolve it — but the value round-trips.
    /// `debugger.asm`'s `__FSTRING_*` pass `.__operand`/`.__param` string locals
    /// into `__FSTRING_PushArgument` this way (probe `probe_argkind` 2026-07-05).
    /// A label / int-local keeps the qualified NAME, resolved via the symbol table
    /// (e.g. `aabb_axis_test`'s `.next_object` arg). Latent until __DEBUG__ (T5).
    fn bind_macro_arg(&self, v: String) -> String {
        if is_bare_local(&v) {
            if let Some(s) = self.resolve_str(&v) {
                // Quoted so it re-lexes as one `Tok::Str`. Assumes the value has
                // no embedded `"` — true for every debugger operand/param
                // descriptor (`"d0"`, `".w"`, `"#"`, …); a value containing a
                // quote would produce a broken literal (none occurs in aeon).
                return format!("\"{s}\"");
            }
            return qualify(&v, self.dot_scope(&v));
        }
        v
    }

    /// Substitute the innermost expansion's `.ATTRIBUTE`, `ALLARGS` and
    /// parameters into a body line's text, or `None` when there is no
    /// expansion to substitute from (root source) or the innermost one is
    /// suspended (a `rept`/`while` body already substituted at loop entry).
    ///
    /// Called wherever a line's text is CONSUMED rather than merely carried, so
    /// that the binding in force is the one at the moment the line is reached:
    /// [`Self::exec_one`], [`Self::dispatch_head`] (hence every block scan and
    /// every `if`/`switch`/`rept`/`while` head), [`Self::def_function`],
    /// [`Self::capture_macro`], [`Self::capture_struct`] and
    /// [`Self::parse_struct_field`].
    ///
    /// One pass, no rescanning: what a binding pastes in is the caller's text
    /// and stays it, even when that text spells another of this expansion's
    /// parameter names. [`substitute_frame`] carries the rule and asl's row for
    /// it.
    fn subst_frame_text(&self, text: &str) -> Option<String> {
        let f = self.macro_frames.last()?;
        if f.suspend > 0 {
            return None;
        }
        Some(substitute_frame(
            text,
            f.attribute.as_deref(),
            &f.all_args(),
            f.int_label.as_deref(),
            &f.params,
            &f.bound,
            f.arg_count(),
        ))
    }

    /// [`Self::subst_frame_text`] lifted to a whole line, preserving its span
    /// anchor so diagnostics still point at the macro body.
    fn subst_frame(&self, line: &SrcLine) -> Option<SrcLine> {
        let text = self.subst_frame_text(&line.text)?;
        Some(SrcLine { text, base: line.base, source: line.source })
    }

    /// Whether a body line reached now would be substituted — the test
    /// `rept`/`while` use to decide between replaying borrowed source lines and
    /// materializing a substituted copy.
    fn frame_substitutes(&self) -> bool {
        self.macro_frames.last().is_some_and(|f| f.suspend == 0)
    }

    /// AS's `shift`: drop the innermost expansion's first argument. Outside a
    /// macro it is an error — asl reports it as `EXITM not called from within
    /// macro` (probe `p4.asm` case 4a), sharing its not-in-a-macro check; the
    /// wording here names the directive that was actually written.
    fn directive_shift(&mut self, span: Span) {
        match self.macro_frames.last_mut() {
            Some(f) => f.shift(),
            None => self.err(span, "`shift` outside a macro expansion"),
        }
    }

    fn expand_macro_inner(&mut self, name: &str, arg_toks: &[Token], attribute: Option<&str>) {
        // TAKEN FIRST, before any early return. `exec_one` parks the invocation
        // line's label here and this is the only consumer; a return that left it
        // parked would hand one call's label to the NEXT expansion, which is a
        // wrong symbol rather than a missing one. Both early returns below are
        // reachable — the recursion cap fires on the corpus's own `zoneTableEntry`.
        let captured = self.pending_int_label.take();
        if self.macro_depth >= EXPAND_CAP {
            let span = arg_toks.first().map(|t| t.span).unwrap_or(Span {
                source: self.source,
                start: 0,
                end: 0,
            });
            self.err(
                span,
                format!("macro `{name}` expansion too deep (recursive macro?)"),
            );
            return;
        }
        let MacroDef { params, body, int_label } = match self.macros.get(name) {
            Some(m) => m.clone(),
            None => return,
        };
        // A macro that does not declare the capture never sees a label; one that
        // does but was invoked bare gets the EMPTY text — which is what makes the
        // corpus's `if "__LABEL__"<>""` guard in `rsttarget` a guard at all
        // (asl: `dc.b "[]"`, `if ""<>""` FALSE, and the bare `label *` that
        // follows defines nothing and is not an error).
        let int_label = int_label.then(|| captured.unwrap_or_default());
        let all_args = render_tokens(arg_toks);
        let groups = split_top_commas(arg_toks);
        // `ARGCOUNT`'s entry value. `split_top_commas` returns ONE (empty) group
        // for an empty operand field, which is right for binding — the first
        // parameter gets the empty default either way — and wrong for counting:
        // asl reports 0 for `ac` and 2 for `ac ,` (probe `p5.asm`). The empty
        // field is the reachable case, not a curiosity: `jmpTos` with no
        // arguments relays an empty `ALLARGS` into `jmpTosInternal2`, whose
        // `if ARGCOUNT>0` is the only thing standing between that and an
        // `irp op,` over one empty item defining a nameless label.
        let argc = if arg_toks.is_empty() { 0 } else { groups.len() as i64 };
        let mut keyword: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut positional: Vec<String> = Vec::new();
        for g in &groups {
            if let [Token {
                tok: Tok::Ident(nm),
                ..
            }, Token {
                tok: Tok::Punct(Punct::Eq),
                ..
            }, value @ ..] = *g
            {
                if !value.is_empty() && params.iter().any(|p| p == nm) {
                    keyword.insert(nm.clone(), render_tokens(value));
                    continue;
                }
            }
            positional.push(render_tokens(g));
        }
        let mut pos_iter = positional.into_iter();
        // The caller's local-label scope, captured BEFORE the expansion swaps in
        // its own scope below. A macro argument that is a bare `.`-local
        // (`aabb_axis_test …,.next_object,…`) names a label in the CALLER's scope
        // — asl evaluates arguments in the caller context — so it must be
        // qualified here, before substitution, not against the expansion scope.
        // Every `bind_macro_arg` below therefore runs while `self` still describes
        // the caller, and [`Self::dot_scope`] picks the caller's expansion or its
        // real scope by the same rule a reference written in the caller would get.
        let caller_scope = self.scope.clone();
        // An OMITTED argument binds to the EMPTY STRING, not "left unsubstituted"
        // (asl-verified): the Aeon parallax macros gate optional fields on
        // `if "param" = ""` and expect the bare param to vanish where used
        // (`P_VFG := vFactorFg` → `P_VFG := ` on the empty branch, never taken).
        // `replace_word` treats `"` as a word boundary, so an empty binding also
        // collapses `"param"` → `""`, making the guard compare true.
        let mut bound: Vec<String> = Vec::with_capacity(params.len());
        // `filled` tracks which parameter slots an argument was actually
        // written for, as opposed to the empty default. `ALLARGS` after a
        // `shift` renders the arguments that were SUPPLIED, so an unsupplied
        // trailing parameter must not contribute an empty group to it
        // (asl-verified: probe `p4.asm` case 4e, `mp aa` on params `n1,n2,n3`,
        // shifts to `s[][][][]` — an empty `ALLARGS`, not `,`).
        let mut filled: Vec<bool> = Vec::with_capacity(params.len());
        for p in &params {
            let supplied = keyword.get(p).cloned().or_else(|| pos_iter.next());
            filled.push(supplied.is_some());
            let v = self.bind_macro_arg(supplied.unwrap_or_default());
            bound.push(v);
        }
        // Surplus positional arguments — the ones the parameter list could not
        // hold — get the SAME binding as the ones it could. They reach the body
        // only through `ALLARGS`, but that is a text the body can consume as a
        // symbol reference, so an argument's meaning cannot depend on which side
        // of the parameter count it fell.
        let surplus: Vec<String> = pos_iter.map(|v| self.bind_macro_arg(v)).collect();
        // The argument groups `ALLARGS` walks: every supplied parameter slot in
        // PARAMETER order (so a keyword call shifts in the order the callee
        // declared, asl-verified — probe `p4.asm` case 4b, `kw k2=aa,k1=bb` on
        // params `k1,k2` shifts to `ALLARGS` = `aa`, the value bound to `k2`),
        // then any surplus positional arguments the parameter list could not
        // hold (probe `p4.asm` case 4d, one parameter and three arguments
        // shifts to `bb,cc`).
        let mut all: Vec<String> = bound
            .iter()
            .zip(filled.iter())
            .filter(|(_, f)| **f)
            .map(|(v, _)| v.clone())
            .collect();
        all.extend(surplus);
        // A `.`-local written LITERALLY in this macro body is scoped to the
        // EXPANSION, not the caller's global label (asl-verified, T4 probe
        // P1/P3): two expansions of one macro in a single global scope each own a
        // private copy, and asl neither collides them nor exposes them as caller-
        // qualified user symbols. Give the body a fresh, reserved scope name so
        // `qualify(".x", scope)` → `<expansion>.x` is unique per expansion, then
        // restore the caller's scope. The reserved prefix cannot alias a user
        // global label (no source label begins with a space). A `.`-local that
        // came in through an ARGUMENT was already qualified against `caller_scope`
        // above, so it points at the caller's label and is unaffected here. All
        // aeon body `.`-locals are def+ref within one expansion and reached only
        // by fixed-length short branches, so this affects no layout.
        //
        // Limitations (none exercised by aeon): a macro body that references a
        // caller-scope `.`-local WITHOUT it being passed as an argument, or
        // defines a NON-dotted global label meant to become the outer scope
        // afterwards, would diverge — aeon does neither.
        self.macro_expansion_seq += 1;
        let dot_labels = match self.dot_label_cache.get(name) {
            Some(set) => set.clone(),
            None => {
                let set = std::rc::Rc::new(scan_dot_labels(&body));
                self.dot_label_cache.insert(name.to_string(), set.clone());
                set
            }
        };
        // The outermost expansion on the stack records the scope it was invoked
        // from; every expansion nested inside it keeps that same real scope, so a
        // value-binding `.`-local reaches out through the whole nest in one step.
        let outer_scope = self.outer_scope.clone();
        let outermost = self.macro_frames.is_empty();
        if outermost {
            self.outer_scope = caller_scope.clone();
        }
        self.scope = Some(format!(" macro#{}", self.macro_expansion_seq));
        self.macro_depth += 1;
        let shift_cap = params.len().max(argc.max(0) as usize);
        self.macro_frames.push(MacroFrame {
            params,
            bound,
            all,
            all_raw: all_args,
            shifted: 0,
            argc,
            shift_cap,
            attribute: attribute.map(str::to_string),
            suspend: 0,
            dot_labels,
            int_label,
        });
        self.exec(&body);
        self.macro_frames.pop();
        self.macro_depth -= 1;
        // A `label` directive inside the body opens the CALLER's scope, and the
        // scope it opened OUTLIVES the expansion — that is what carries
        // `zoneOrderedTable`'s `.zone_table_name` and every `Table.cnt` read
        // after the call. The real scope lives in `outer_scope` while an
        // expansion is running, so the outermost frame hands it back as
        // `self.scope` on the way out instead of restoring the stale entry
        // scope, and a nested frame leaves `outer_scope` alone rather than
        // restoring over a change an inner body made. asl, `Tbl outer 3` where
        // `outer` calls `inner` and `inner` writes `__LABEL__ label *`:
        //
        // ```text
        //   17/ 1000 : =$1000               Tbl label *
        //   17/ 100E : =$7                  .cnt := 7
        //   19/ 1019 : 07                  	dc.b Tbl.cnt
        // ```
        if outermost {
            self.scope = self.outer_scope.clone();
            self.outer_scope = outer_scope;
        } else {
            self.scope = caller_scope;
        }
    }
}

// ── free helpers ────────────────────────────────────────────────────────────

/// Whether a `NAME macro …` head declares the internal-label capture.
///
/// AS writes it as a brace group in the parameter list, and the group is a
/// KEYWORD, so it folds even under `-U` (asl: `lo macro {intlabel}` under
/// `Aa:` emits `lo=<Aa> <Aa>` for `"lo=<__LABEL__> <__label__>"`). Declaring it
/// twice is not an error and means what declaring it once means.
///
/// The scan stops at a `;` so a brace inside a trailing comment is inert, and it
/// reads the raw head text because the lexer swallows a `{…}` group without
/// emitting a token — which is also why the group never reaches the parameter
/// list as a phantom slot.
fn head_declares_int_label(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b';' => return false,
            b'{' => {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'}' {
                    j += 1;
                }
                if j >= bytes.len() {
                    return false;
                }
                if text[start..j].trim().eq_ignore_ascii_case("intlabel") {
                    return true;
                }
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    false
}

/// The SOURCE TEXT a token group was lexed from, recovered through the group's
/// own spans, or `""` for an empty group.
///
/// `irp`'s items are raw text and must stay the text the author wrote.
/// [`render_tokens`] cannot supply that: it re-renders `Tok::Int` in decimal, so
/// `irp v,$FF` over `dc.b "[v]"` emits `[255]` where asl emits `[$FF]` — wrong
/// BYTES with no diagnostic, which is the failure mode a loop construct has
/// instead of an error message. Measured against asl, probe `p8.asm` case 8b:
///
/// ```text
///   16/ 100F : 5B31 2B32 5D     dc.b "[1+2]"
///   16/ 1014 : 5B24 4646 5D     dc.b "[$FF]"
/// ```
///
/// Slicing by span also drops a trailing comment and any surrounding whitespace
/// for free, because the lexer never gave either one a token.
///
/// `base` is the line's byte offset, which is what [`lex_line`] added to every
/// span it produced from `text`.
fn slice_source(text: &str, base: u32, group: &[Token]) -> String {
    let (Some(first), Some(last)) = (group.first(), group.last()) else {
        return String::new();
    };
    let start = (first.span.start.saturating_sub(base)) as usize;
    let end = (last.span.end.saturating_sub(base)) as usize;
    match text.get(start..end) {
        Some(s) => s.to_string(),
        // Unreachable for spans this function's own caller produced; a rendered
        // fallback is still a working item rather than a panic.
        None => render_tokens(group),
    }
}

/// What an `irp`/`irpc` head iterates over: `irp`'s comma-separated argument
/// groups, or `irpc`'s characters. The two directives share everything else —
/// block structure, closers, body substitution and the empty-list rule — so
/// they share one implementation and differ by this.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IterKind {
    Groups,
    Chars,
}

/// The binding one macro expansion substitutes into its body, and the state
/// `shift` mutates.
///
/// AS keeps TWO vectors, and `shift` advances both (asl-verified, probe `p2.asm`
/// case 2a — `zt 1,2,3,4` on params `pp,qq,rr` lists
/// `<1|2|3|1,2,3,4>` → `<2|3||2,3,4>` → `<3|||3,4>` → `<|||4>` → `<|||>`):
///
/// * [`Self::bound`] — one slot per declared parameter, filled left-to-right at
///   entry and shifted left with EMPTY fill. **"Empty" is sigil's choice, not
///   asl's**: AS keeps its `\001\00N` placeholder in a slot a shift vacated, so
///   `strlen` there yields 2 and `<>""` is TRUE. A slot never SUPPLIED is
///   genuinely empty in both, which is the case the corpus recursion guards
///   depend on. The divergence is deliberate, corpus-unreachable, and set out
///   with its listing rows under "Deliberately NOT replicated" in
///   `docs/superpowers/notes/2026-09-03-as-shift-macro-argument-walk.md`.
///   It is not a window onto the
///   argument list: with three parameters and four arguments the fourth
///   argument never reaches the third parameter (`<2|3||…>`, third slot empty
///   while `ALLARGS` still holds `4`).
/// * [`Self::all`] — the argument groups, shifted left with no refill, which
///   is what `ALLARGS` renders after a shift.
///
/// A shift past exhaustion is a no-op (the fifth row above repeats the fourth).
struct MacroFrame {
    /// Declared parameter names, in order; parallel to [`Self::bound`].
    params: Vec<String>,
    /// Current value of each parameter. Shifts left, empty-filled.
    bound: Vec<String>,
    /// Argument groups still in scope for `ALLARGS`. Shifts left, no refill.
    all: Vec<String>,
    /// `ALLARGS` before any shift: the invocation's argument text rendered as a
    /// whole, not a re-join of [`Self::all`]. The two agree on every argument
    /// shape, but rendering the token run once is what the byte-exact
    /// `%<…>`-string substitution in aeon's debugger macros already depends on,
    /// so the whole-run rendering stays the source of truth while it is intact.
    all_raw: String,
    /// How many times this expansion has shifted.
    shifted: usize,
    /// `ARGCOUNT` before any shift: the number of comma-separated argument
    /// groups the invocation actually wrote, and 0 when the operand field is
    /// empty. It is NOT `split_top_commas`'s group count, which is 1 for an
    /// empty slice — asl-verified, probe `p5.asm`: `ac` and `ac ` are 0 while
    /// `ac ,` is 2 and `ac 1,,3` is 3.
    argc: i64,
    /// How many shifts still MOVE `ARGCOUNT`. asl stops decrementing once the
    /// argument store is exhausted, and that store is as long as the LONGER of
    /// the parameter list and the argument list — see [`Self::arg_count`].
    shift_cap: usize,
    /// `.ATTRIBUTE` text for a `.b`/`.w`/`.l`/`.s`-suffixed invocation.
    attribute: Option<String>,
    /// Nonzero while a `rept`/`while` body captured from this frame replays.
    /// AS substitutes such a body ONCE, where the loop is entered, and replays
    /// the substituted text — so a `shift` inside the body still advances the
    /// frame but does not change the body's own text (asl-verified: probe
    /// `p2.asm` case 2f shows `<21|21,22,23,24>` on every iteration despite a
    /// `shift` each time, while probe `p3.asm` case 3a shows the frame HAS
    /// advanced twice once the loop exits — `post<|23,24>`).
    suspend: usize,
    /// The `.`-local names this macro's body defines as PLAIN LABELS. Those
    /// belong to the EXPANSION; every other `.`-local a body line mentions
    /// belongs to the caller's real scope. See [`scan_dot_labels`].
    dot_labels: std::rc::Rc<std::collections::BTreeSet<String>>,
    /// The invocation line's label text, for a macro whose parameter list
    /// carries `{INTLABEL}`. `None` where the macro does not declare the
    /// capture, and then `__LABEL__` is not a substitution at all but ordinary
    /// text (asl-verified: `m macro pp` emitting `"L=<__LABEL__>"` under
    /// `Lab1: m 11` emits the nine characters `__LABEL__`).
    ///
    /// `shift` does not touch it — it is not an argument. asl, `sm macro
    /// {INTLABEL},pp` called `Lb2: sm 5,6` with a `shift` first:
    ///
    /// ```text
    ///   12/ 100B : 3C36 3E20 3C4C     dc.b "<6> <Lb2> <>"
    /// ```
    int_label: Option<String>,
}

impl MacroFrame {
    /// `ARGCOUNT` for the current shift state, and it is NOT one quantity
    /// tracked across a shift — the two states answer from different places.
    ///
    /// Before any shift it is the number of arguments the call WROTE. From the
    /// first shift on it is the number of PARAMETERS the macro DECLARED, minus
    /// the shift count — so it can go negative, and a one-parameter macro called
    /// with three arguments drops 3 → 0 → -1 → -2 rather than counting the
    /// arguments down. The decrement stops once `max(params, args)` shifts have
    /// happened, because a shift past the end of the argument store is a no-op.
    ///
    /// asl `-U`, 16 rows over the (parameters × arguments) grid, probes `p3.asm`
    /// and `p4.asm`. Two of them from `p3.asm`, `one macro pp` / `one 11,22,33`
    /// (left) and `three macro q1,q2,q3` / `three 11,22,33,44,55` (right), each
    /// printing `dc.w ARGCOUNT` before the first shift and after each of four:
    ///
    /// ```text
    ///   27/ 1002 : 0003    dc.w 3      37/ 103E : 0005    dc.w 5
    ///   27/ 1004 : 0000    dc.w 0      37/ 1040 : 0002    dc.w 2
    ///   27/ 1006 : FFFF    dc.w -1     37/ 1042 : 0001    dc.w 1
    ///   27/ 1008 : FFFE    dc.w -2     37/ 1044 : 0000    dc.w 0
    ///   27/ 100A : FFFE    dc.w -2     37/ 1046 : FFFF    dc.w -1
    /// ```
    ///
    /// Left: one parameter, three arguments — `params - shifts` goes negative,
    /// and stops at `max(1,3)=3` shifts. Right: three parameters, five arguments
    /// — the entry value is the ARGUMENT count 5, every value after it is
    /// `3 - shifts`.
    ///
    /// The corpus needs only the unshifted half: `s2.macrosetup.asm(301)`'s
    /// `if ARGCOUNT>0` guards `jmpTosInternal2`, which declares NO parameters
    /// and performs no `shift` — the enclosing `jmpTosInternal` shifts in its
    /// OWN frame and relays `ALLARGS`, so the value read is the relayed
    /// argument count. The shifted half is implemented anyway because the rule
    /// is measured and a guess left in its place is the thing that rots.
    fn arg_count(&self) -> i64 {
        if self.shifted == 0 {
            return self.argc;
        }
        self.params.len() as i64 - self.shifted.min(self.shift_cap) as i64
    }

    /// `ALLARGS` for the current shift state.
    fn all_args(&self) -> String {
        if self.shifted == 0 {
            self.all_raw.clone()
        } else {
            self.all.join(",")
        }
    }

    /// Drop the first argument: parameters slide left and the vacated tail slot
    /// becomes empty; `ALLARGS` loses its leading group. Exhausted is stable.
    fn shift(&mut self) {
        if !self.bound.is_empty() {
            self.bound.remove(0);
            self.bound.push(String::new());
        }
        if !self.all.is_empty() {
            self.all.remove(0);
        }
        self.shifted += 1;
    }
}

/// Index of the `}` closing the `{` at `open` in `bytes`, or `None` if the group
/// is unterminated. Nested `{…}` groups are matched by depth, and a `"…"`/`'…'`
/// literal inside the group is skipped whole — so `{"\{n}"}` closes on its LAST
/// `}`, not on the one that belongs to the interpolation inside the literal.
fn brace_group_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            q @ (b'"' | b'\'') => {
                i += 1;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split source text into `SrcLine`s (each with its byte offset within `source`).
/// Used for both the root source and included files; `source` is the [`SourceId`]
/// the text was registered under, so every span lexed from these lines resolves
/// against the file the text came from.
fn split_src_lines(text: &str, source: SourceId) -> Vec<SrcLine> {
    let mut lines = Vec::new();
    let mut base = 0u32;
    // A physical line whose last non-whitespace character is `\` is an AS
    // line-continuation: it joins with the following physical line into one
    // logical line (the Aeon `function` definitions in macros.asm /
    // parallax_macros.inc wrap a long body expression this way). The joined
    // logical line takes the FIRST physical line's `base`; the `\` (and the
    // intervening newline) are replaced by spaces so downstream byte offsets
    // stay length-stable and no bogus `\` token reaches the lexer. Only a
    // trailing `\` continues — an interior `\` (e.g. a macro `\1` parameter
    // marker) is untouched.
    let mut pending: Option<(u32, String)> = None;
    for raw in text.split_inclusive('\n') {
        let trimmed = raw.trim_end();
        let is_cont = trimmed.ends_with('\\');
        // Length-preserving cell text: drop the trailing newline's semantics by
        // turning a continuation `\`+tail into spaces, else keep the raw text.
        let cell = if is_cont {
            // Replace the final `\` with a space, and the trailing whitespace
            // (incl. the newline) it had is preserved as-is after it.
            let cut = trimmed.len() - 1; // index of the `\`
            let mut s = String::with_capacity(raw.len());
            s.push_str(&raw[..cut]);
            s.push(' ');
            s.push_str(&raw[cut + 1..]);
            s
        } else {
            raw.to_string()
        };
        match pending.take() {
            Some((start_base, mut acc)) => {
                acc.push_str(&cell);
                if is_cont {
                    pending = Some((start_base, acc));
                } else {
                    lines.push(SrcLine {
                        text: acc,
                        base: start_base,
                        source,
                    });
                }
            }
            None => {
                if is_cont {
                    pending = Some((base, cell));
                } else {
                    lines.push(SrcLine { text: cell, base, source });
                }
            }
        }
        base += raw.len() as u32;
    }
    if let Some((start_base, acc)) = pending {
        lines.push(SrcLine {
            text: acc,
            base: start_base,
            source,
        });
    }
    lines
}

/// The canonical (lower-case) spelling of a DIRECTIVE or MNEMONIC keyword.
///
/// AS matches its own directive and instruction keywords without regard to
/// case, so real-world sources spell them however the author liked
/// (`CPU 68000`, `EQU`, `STRUCT`, `move.W`). Symbols are a different question
/// and are deliberately NOT folded here: `lib.rs` documents them as
/// case-sensitive and `.emp` shares this symbol namespace, so folding a name
/// would merge two distinct `.emp` symbols. The fold therefore lives at each
/// site that DECIDES "is this identifier a keyword", never on `Tok::Ident`
/// itself and never on any path that goes on to define or resolve a name.
///
/// Borrowing when the input is already lower case keeps the hot path (every
/// line of every pass reaches `is_op_keyword`/`is_mnemonic`) allocation-free.
fn fold_kw(s: &str) -> std::borrow::Cow<'_, str> {
    if s.bytes().any(|b| b.is_ascii_uppercase()) {
        std::borrow::Cow::Owned(s.to_ascii_lowercase())
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

/// Whether `s` names a keyword this front end recognizes at all — a directive
/// (`is_op_keyword`) or an instruction (`is_mnemonic`). Used by `dispatch_head`
/// to decide whether the folded or the RAW spelling of a head is the one to
/// hand downstream: a keyword is returned folded (so block scanning matches
/// against lower-case literals), anything else — a macro name, a label — is
/// returned exactly as written.
fn is_keyword(s: &str) -> bool {
    is_op_keyword(s) || is_mnemonic(s)
}

fn is_op_keyword(s: &str) -> bool {
    matches!(
        fold_kw(s).as_ref(),
        "cpu"
            | "phase"
            | "dephase"
            | "org"
            | "save"
            | "restore"
            | "padding"
            | "supmode"
            | "db"
            | "dw"
            | "dc.b"
            | "dc.w"
            | "dc.l"
            | "equ"
            | "if"
            | "elseif"
            | "else"
            | "endif"
            | "ifdef"
            | "ifndef"
            | "rept"
            | "irp"
            | "irpc"
            | "endr"
            | "endm"
            | "shift"
            | "macro"
            | "struct"
            | "endstruct"
            | "function"
            | "include"
            | "binclude"
            | "error"
            | "fatal"
            | "message"
            | "ds.b"
            | "ds.w"
            | "ds.l"
            | "align"
            | "while"
            | "switch"
            | "case"
            | "elsecase"
            | "endcase"
    )
}

/// The closer keyword(s) that terminate the block a given OPENER keyword
/// starts, or `&[]` if `s` does not open a block at all (used by
/// [`Asm::find_block_end`]'s nesting stack — see its doc for why this must be
/// keyed per-opener rather than a single flat set: `while`/`macro` (and
/// optionally `rept`) all share the literal `endm` closer in real AS).
fn closers_for(s: &str) -> &'static [&'static str] {
    match fold_kw(s).as_ref() {
        "if" | "ifdef" | "ifndef" => &["endif"],
        "rept" => &["endr", "endm"],
        // asl closes BOTH on either keyword (probe `p8.asm` case 8e, `p9.asm`
        // case 9e: an `irpc` shut with `endr` and an `irp` shut with `endr`
        // both expand), and the corpus writes `endm` at every one of its ten
        // sites.
        "irp" | "irpc" => &["endr", "endm"],
        "while" => &["endm"],
        "macro" => &["endm"],
        // asl closes a `struct` on either keyword (probe `q8.asm`: a `struct`
        // shut with `ends` yields the same `B.len` a bare `endstruct` does).
        // Both corpora write `endstruct` at all six sites, three of them with
        // the struct's own name in the LABEL column (`SoundQueue ENDSTRUCT`),
        // which `line_keyword`'s bare-label rule already routes here.
        "struct" => &["endstruct", "ends"],
        "switch" => &["endcase"],
        _ => &[],
    }
}

/// Split a bare identifier on a trailing `.b`/`.w`/`.l`/`.s` size suffix,
/// returning the base name and the literal suffix text (e.g. `.w`). Used for
/// `.ATTRIBUTE` macro-suffix synthesis (T9.2): a macro invoked as
/// `name.SUFFIX args` is dispatched by stripping the suffix and checking
/// whether the BASE name is a captured macro — deliberately distinct from
/// `split_mnemonic_and_size` (which returns a parsed `M68kSize` for real
/// mnemonic lowering) so the two never interact: this only ever fires from
/// `dispatch`'s attribute-macro check, which is gated on the base name being
/// a literal entry in `self.macros` — a real mnemonic like `move`/`clr` is
/// never in that map, so `move.w`/`clr.b` etc. keep going through the normal
/// mnemonic-suffix path untouched.
/// Recognition of the suffix is case-insensitive (`Foo.W` invokes the same
/// macro as `foo.w` does), but the suffix TEXT handed back is a slice of the
/// caller's own string rather than a canonical literal: `.ATTRIBUTE` is a
/// verbatim textual substitution into the macro body, so the body must see the
/// spelling the call site wrote.
fn split_attribute_suffix(s: &str) -> Option<(&str, &str)> {
    let (base, c) = split_dot_suffix(s)?;
    if matches!(c, b'b' | b'w' | b'l' | b's') {
        Some((base, &s[base.len()..]))
    } else {
        None
    }
}

/// Index of the `)` matching the `(` at `open`, or `None` if `toks[open]` is
/// not a `(` or the group never closes within `toks`.
fn matching_rparen(toks: &[Token], open: usize) -> Option<usize> {
    if !matches!(toks.get(open)?.tok, Tok::Punct(Punct::LParen)) {
        return None;
    }
    let mut depth = 0usize;
    for (k, t) in toks.iter().enumerate().skip(open) {
        match t.tok {
            Tok::Punct(Punct::LParen) => depth += 1,
            Tok::Punct(Punct::RParen) => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// The interior of `toks` when the whole slice is ONE balanced `( … )` group
/// with a non-empty body, else `None`. Parentheses around a string expression
/// are transparent in asl (`strlen(("abc"))` = 3), and the redundant pair is
/// how a substituted user-`function` argument always arrives.
///
/// The balance check is what makes this safe: `("a")+("b")` opens and closes
/// before the end, so it is NOT peeled and does not masquerade as one group.
fn peel_parens(toks: &[Token]) -> Option<&[Token]> {
    if matching_rparen(toks, 0)? != toks.len() - 1 || toks.len() < 3 {
        return None;
    }
    Some(&toks[1..toks.len() - 1])
}

/// Length (in tokens) of the trailing string-expression at the END of `out`, or
/// `None` if the last token can't begin a string comparison LHS. Used by
/// [`Evaluator::expand_str_comparisons`] to find the operand to the left of a
/// `=`/`<>` whose RHS is a string literal. A string-expr is: a string literal
/// (1 token), a bare identifier (1 token — a candidate string-valued symbol,
/// validated by `eval_str`), a balanced `substr(...)`/`lowstring(...)` call
/// ending in `)`, or a bare balanced `( … )` group — parentheses around a
/// string expression are transparent (asl folds `(("he"))<>"he"` to 0), and
/// `expand_calls` parenthesises every argument it substitutes into a user
/// `function` body, so a comparison against a function parameter arrives in
/// exactly that shape. `eval_str` still decides whether the group IS a string,
/// so an ordinary numeric `(a+b)=…` is left untouched.
fn trailing_str_expr_len(out: &[Token]) -> Option<usize> {
    // `out.last()?` (not `out[n-1]`): an expression whose FIRST token is a
    // comparison operator (`dc.b <>"x"`) reaches here with `out` empty — `n - 1`
    // would underflow-panic in debug. Empty → no LHS → None (the malformed input
    // then falls through to a normal "bad expression" diagnostic, not a crash).
    let last = out.last()?;
    let n = out.len();
    match &last.tok {
        Tok::Str(_) | Tok::Ident(_) => Some(1),
        Tok::Punct(Punct::RParen) => {
            // Walk back to the matching `(`; the ident before it must name a
            // string-producing builtin.
            let mut depth = 0usize;
            let mut j = n;
            while j > 0 {
                j -= 1;
                match &out[j].tok {
                    Tok::Punct(Punct::RParen) => depth += 1,
                    Tok::Punct(Punct::LParen) => {
                        depth -= 1;
                        if depth == 0 {
                            let before = j.checked_sub(1).and_then(|k| out.get(k));
                            if let Some(Token {
                                tok: Tok::Ident(name),
                                ..
                            }) = before
                            {
                                if name == "substr" || name == "lowstring" {
                                    return Some(n - (j - 1));
                                }
                                return None;
                            }
                            // Nothing (or a non-identifier) before the `(`: a
                            // bare group, transparent around whatever it holds.
                            return Some(n - j);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        _ => None,
    }
}

fn is_mnemonic(s: &str) -> bool {
    mnemonic(s).is_some()
}

fn mnemonic(s: &str) -> Option<Mnemonic> {
    use Mnemonic::*;
    Some(match fold_kw(s).as_ref() {
        "nop" => Nop,
        "ld" => Ld,
        "add" => Add,
        "adc" => Adc,
        "sub" => Sub,
        "sbc" => Sbc,
        "and" => And,
        "or" => Or,
        "xor" => Xor,
        "cp" => Cp,
        "inc" => Inc,
        "dec" => Dec,
        "push" => Push,
        "pop" => Pop,
        "ex" => Ex,
        "exx" => Exx,
        "ret" => Ret,
        "jr" => Jr,
        "jp" => Jp,
        "call" => Call,
        "djnz" => Djnz,
        "rrca" => Rrca,
        "rlca" => Rlca,
        "rla" => Rla,
        "rra" => Rra,
        "daa" => Daa,
        "cpl" => Cpl,
        "ccf" => Ccf,
        "halt" => Halt,
        "rst" => Rst,
        "scf" => Scf,
        "ei" => Ei,
        "di" => Di,
        "bit" => Bit,
        "res" => Res,
        "set" => Set,
        "srl" => Srl,
        "rr" => Rr,
        "sla" => Sla,
        "rlc" => Rlc,
        "rrc" => Rrc,
        "rl" => Rl,
        "sra" => Sra,
        "neg" => Neg,
        "im" => Im,
        "ldir" => Ldir,
        _ => return None,
    })
}

fn cond_word(w: &str) -> Option<Cond> {
    use Cond::*;
    Some(match w {
        "nz" => Nz,
        "z" => Z,
        "nc" => Nc,
        "c" => C,
        "po" => Po,
        "pe" => Pe,
        "p" => P,
        "m" => M,
        _ => return None,
    })
}

fn reg8(w: &str) -> Option<Reg8> {
    use Reg8::*;
    Some(match w {
        "a" => A,
        "b" => B,
        "c" => C,
        "d" => D,
        "e" => E,
        "h" => H,
        "l" => L,
        _ => return None,
    })
}

fn reg16(w: &str) -> Option<Reg16> {
    use Reg16::*;
    Some(match w {
        "bc" => Bc,
        "de" => De,
        "hl" => Hl,
        "sp" => Sp,
        "af" => Af,
        "ix" => Ix,
        "iy" => Iy,
        _ => return None,
    })
}

/// Split a 68k mnemonic token on a trailing `.b`/`.w`/`.l`/`.s` size suffix.
/// Returns the bare base mnemonic and the parsed size (`None` if no suffix —
/// the caller falls back to `m68k_default_size`, or errors if that's also `None`).
/// The suffix match is case-insensitive (`move.W` is the same instruction as
/// `move.w` to AS) but the returned BASE is a slice of the caller's own string,
/// so the base keeps its original spelling and `m68k_mnemonic` folds it itself.
fn split_mnemonic_and_size(s: &str) -> (&str, Option<M68kSize>) {
    if let Some((base, c)) = split_dot_suffix(s) {
        let size = match c {
            b'b' => Some(M68kSize::B),
            b'w' => Some(M68kSize::W),
            b'l' => Some(M68kSize::L),
            b's' => Some(M68kSize::S),
            _ => None,
        };
        if let Some(size) = size {
            return (base, Some(size));
        }
    }
    (s, None)
}

/// Split `s` on a trailing `.<letter>`, returning the base slice and the
/// LOWER-CASED suffix letter. Shared by the two suffix splitters so both agree
/// on what a suffix is. Returns `None` when the last two bytes are not a dot
/// followed by an ASCII letter (identifiers are ASCII here — `is_ident_tail`
/// admits no multi-byte character — but the char-boundary check keeps the
/// slicing sound regardless).
fn split_dot_suffix(s: &str) -> Option<(&str, u8)> {
    let n = s.len();
    if n < 2 || !s.is_char_boundary(n - 2) {
        return None;
    }
    let tail = s.as_bytes();
    if tail[n - 2] != b'.' || !tail[n - 1].is_ascii_alphabetic() {
        return None;
    }
    Some((&s[..n - 2], tail[n - 1].to_ascii_lowercase()))
}

/// The T4/T5/T5b/T5c in-scope 68000 mnemonic table: straight-line
/// register/immediate core, the fixed-length register-indirect EA family,
/// `lea`/`pea`, explicit-width absolute addressing, and (T5c) control
/// transfer — `bra`/`bsr`/`Bcc`, `Dbcc` (`dbf`/`dbra`/`db<cc>`), `Scc`, and
/// `jmp`/`jsr`. `move`/`andi`/`ori` are refined to `MoveToSr`/`MoveFromSr`/
/// `AndiCcr`/`OriCcr` post-hoc by `refine_m68k_mnemonic` once the operand
/// shape (a bare `sr`/`ccr`) is known. `movem`/`movep` (register-list operands)
/// are now in scope too; nothing 68000 the Aeon source uses remains deferred.
fn m68k_mnemonic(base: &str) -> Option<M68kMnemonic> {
    use M68kMnemonic::*;
    Some(match fold_kw(base).as_ref() {
        "move" => Move,
        "movea" => Movea,
        "add" => Add,
        "adda" => Adda,
        "sub" => Sub,
        "suba" => Suba,
        "and" => And,
        "or" => Or,
        "eor" => Eor,
        "cmp" => Cmp,
        "cmpa" => Cmpa,
        "muls" => Muls,
        "mulu" => Mulu,
        "divs" => Divs,
        "divu" => Divu,
        "addi" => Addi,
        "subi" => Subi,
        "andi" => Andi,
        "ori" => Ori,
        "eori" => Eori,
        "cmpi" => Cmpi,
        "moveq" => Moveq,
        "addq" => Addq,
        "subq" => Subq,
        "asl" => Asl,
        "asr" => Asr,
        "lsl" => Lsl,
        "lsr" => Lsr,
        "rol" => Rol,
        "ror" => Ror,
        "roxl" => Roxl,
        "roxr" => Roxr,
        "btst" => Btst,
        "bset" => Bset,
        "bclr" => Bclr,
        "bchg" => Bchg,
        "clr" => Clr,
        "neg" => Neg,
        "not" => Not,
        "tst" => Tst,
        "tas" => Tas,
        "swap" => Swap,
        "ext" => Ext,
        "lea" => Lea,
        "pea" => Pea,
        "movem" => Movem,
        "movep" => Movep,
        "addx" => Addx,
        "cmpm" => Cmpm,
        "exg" => Exg,
        "nop" => Nop,
        "rts" => Rts,
        "rte" => Rte,
        "trap" => Trap,
        "bra" => Bra,
        "bsr" => Bsr,
        "jmp" => Jmp,
        "jsr" => Jsr,
        "dbf" | "dbra" => Dbcc(M68kCond::F),
        _ => {
            if let Some(rest) = base.strip_prefix("db") {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Dbcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('b') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Bcc(c));
                }
            }
            if let Some(rest) = base.strip_prefix('s') {
                if let Some(c) = m68k_cond(rest) {
                    return Some(Scc(c));
                }
            }
            return None;
        }
    })
}

/// Parse a 68000 condition-code mnemonic suffix (the `<cc>` in `b<cc>`,
/// `db<cc>`, `s<cc>`) into its `Cond`. All 16 codes per the ISA's `Cond` enum,
/// plus the two unsigned-branch spellings `hs`/`lo`: on the 68000 HS
/// (higher-or-same) IS carry-clear (CC) and LO (lower) IS carry-set (CS) —
/// asl accepts `bhs`/`blo`/`shs`/`slo`/`dbhs`/`dblo` as exact aliases, and the
/// Aeon source uses `bhs`/`blo` pervasively (~68 sites). They encode to the
/// identical opcode as `bcc`/`bcs`, so this is a pure spelling alias.
fn m68k_cond(w: &str) -> Option<M68kCond> {
    use M68kCond::*;
    Some(match w {
        "t" => T,
        "f" => F,
        "hi" => Hi,
        "ls" => Ls,
        "cc" => Cc,
        "cs" => Cs,
        "hs" => Cc,
        "lo" => Cs,
        "ne" => Ne,
        "eq" => Eq,
        "vc" => Vc,
        "vs" => Vs,
        "pl" => Pl,
        "mi" => Mi,
        "ge" => Ge,
        "lt" => Lt,
        "gt" => Gt,
        "le" => Le,
        _ => return None,
    })
}

/// If `base` names a real 68000 mnemonic that this front-end deliberately does
/// not implement yet, name the family for the diagnostic; else `None`
/// (genuinely unrecognized). Nothing remains out of scope — `movem`/`movep`
/// (with register-list operands) are now handled — so this always returns
/// `None`; it is retained as the seam where a future deferral would name its
/// family.
fn m68k_out_of_scope(_base: &str) -> Option<&'static str> {
    None
}
/// The default size for a bare `move` whose operand list names one of the
/// 68000's non-EA special registers.
///
/// [`m68k_default_size`] is keyed by mnemonic alone, and it cannot answer for
/// `move`: the size depends on WHICH move this is, and that is only knowable
/// from the operands. But `refine_m68k_mnemonic` — which turns `move` into
/// `MoveToCcr`/`MoveToSr`/`MoveToUsp`/`MoveFromUsp` — runs after the operand
/// atoms have been converted, which is after the size is needed. So the size
/// is read straight off the ATOMS here, before either step.
///
/// Every one of these forms has exactly one legal size in the ISA, so this is
/// not a preference — it is the only size the encoder will accept, and it is
/// what asl emits for the bare spelling (asl-verified: `move d6,ccr` = `44C0`,
/// `move #$2700,sr` = `46FC 2700`, `move a6,usp` = `4E66`). The suffixed
/// spellings are unaffected: `suffix_size` wins.
fn m68k_special_reg_size(m: M68kMnemonic, atoms: &[OperandAtom]) -> Option<M68kSize> {
    if m != M68kMnemonic::Move {
        return None;
    }
    atoms.iter().find_map(|a| match a {
        OperandAtom::Value(Expr::Sym(name)) => match name.as_str() {
            // `move <ea>,ccr` and `move <ea>,sr` / `move sr,<ea>` are word ops.
            "ccr" | "sr" => Some(M68kSize::W),
            // `move An,usp` / `move usp,An` are long ops.
            "usp" => Some(M68kSize::L),
            _ => None,
        },
        _ => None,
    })
}


/// The implicit size for mnemonics real 68k syntax never suffixes (`moveq`,
/// `swap`, `nop`, `rts`, `rte`, `tas`, `trap`, `lea`, `pea`, `jmp`, `jsr`,
/// `Dbcc`, `Scc`). Verified against `crates/sigil-isa/tests/corpus_m68k/mod.rs`:
/// `moveq` is always encoded `Size::L` (the encoder truncates the data to a
/// signed byte regardless); `lea`/`pea` are always long (an address is always
/// 32 bits); `jmp`/`jsr`/the fixed-opcode control forms carry `Size::W` in the
/// corpus, `Dbcc` is always `Size::W` (its displacement is a fixed 16-bit
/// word), and `Scc` is always `Size::B` (byte-fixed opcode) — in every case
/// purely because `Instruction` requires *a* size field; the encoder ignores
/// it for them. Branches (`bra`/`bsr`/`Bcc`) deliberately have NO default:
/// Aeon pins branch width by an explicit `.s`/`.w` suffix, never relaxes.
fn m68k_default_size(m: M68kMnemonic) -> Option<M68kSize> {
    use M68kMnemonic::*;
    match m {
        Moveq => Some(M68kSize::L),
        Lea | Pea => Some(M68kSize::L),
        Swap | Nop | Rts | Rte | Tas | Trap => Some(M68kSize::W),
        Jmp | Jsr => Some(M68kSize::W),
        // Bit ops (`btst`/`bset`/`bclr`) carry NO suffix in real 68k syntax:
        // the operation size is implicit in the destination (long for a `Dn`
        // target, byte for a memory target) and the encoder (`encode_bit`)
        // re-derives it from the operand, ignoring this field — so the value
        // here only satisfies `Instruction`'s size slot. `B` keeps the source
        // `#bit`/`Dn` immediate fold within byte bounds (bit numbers are ≤ 31).
        Btst | Bset | Bclr | Bchg => Some(M68kSize::B),
        // `exg` and the USP moves have no size field at all; asl takes the bare
        // spelling and `.l` and rejects `.b`/`.w`. Both corpora write both
        // spellings (`exg d0,d1` and `exg.l d1,d2`).
        Exg | MoveToUsp | MoveFromUsp => Some(M68kSize::L),
        // Word-only, and asl takes the bare spelling: `move d6,ccr` (S2, 5
        // sites) is the same `44C0 | ea` as `move.w d6,ccr` (S1, 2 sites).
        MoveToCcr => Some(M68kSize::W),
        Dbcc(_) => Some(M68kSize::W),
        Scc(_) => Some(M68kSize::B),
        _ => None,
    }
}

/// Fold bounds for a `#imm` operand at a given size — generous enough to admit
/// either the signed or the bit-pattern-equivalent unsigned spelling; the
/// encoder (`imm_ext_words`/`moveq`/`addq` range checks) does the real
/// business-rule validation and surfaces an `IsaError` on overflow.
fn m68k_imm_bounds(size: M68kSize) -> (i64, i64) {
    match size {
        M68kSize::B => (-128, 0xFF),
        M68kSize::W => (-0x8000, 0xFFFF),
        M68kSize::L | M68kSize::S => (i32::MIN as i64, u32::MAX as i64),
    }
}

/// `d0`..`d7` → `Some(0..=7)`; anything else (including out-of-range `d8`+) → `None`.
fn m68k_data_reg(w: &str) -> Option<u8> {
    let n: u8 = w.strip_prefix('d')?.parse().ok()?;
    (n <= 7).then_some(n)
}

/// The `an`-slot error for `(d,An)`/`(d,An,Xn)` when it's not a real address
/// register. `pc` parses down the same `(expr,ident)` shape as `(d16,An)`/
/// `(d8,An,Xn)` (see `classify`). `(d16,PC)` is intercepted and lowered
/// earlier (see `lower_m68k_generic`'s pc-relative scan), so this only ever
/// fires for the still-unsupported `(d8,PC,Xn)` indexed form (an `M68kIdx`
/// atom) — hence its own naming diagnostic rather than the generic
/// "not a valid address register" one.
fn m68k_disp_an_error(an: &str) -> String {
    if an == "pc" {
        "`(d8,PC,Xn)` indexed PC-relative addressing is not yet supported (only `(d16,PC)` lowers)"
            .to_string()
    } else {
        format!("`{an}` is not a valid address register in `(d,An)`/`(d,An,Xn)`")
    }
}

/// `a0`..`a7` → `Some(0..=7)`; `sp` is the `a7` alias. Anything else → `None`.
fn m68k_addr_reg(w: &str) -> Option<u8> {
    if w == "sp" {
        return Some(7);
    }
    let n: u8 = w.strip_prefix('a')?.parse().ok()?;
    (n <= 7).then_some(n)
}

/// The MOVEM register-list bit index of a single register: `d0..d7` → `0..=7`,
/// `a0..a7` (and `sp` = `a7`) → `8..=15`. This is the CANONICAL mask ordering
/// the encoder expects (`Operand::RegList` doc); the `-(An)` reversal is applied
/// inside `encode_movem`, never here. `None` for any non-register word.
fn reg_list_index(w: &str) -> Option<u8> {
    if let Some(n) = m68k_data_reg(w) {
        Some(n)
    } else {
        m68k_addr_reg(w).map(|n| n + 8)
    }
}

/// Parse a MOVEM register-list operand's tokens into a canonical 16-bit mask
/// (bit0=D0..bit7=D7, bit8=A0..bit15=A7), or `None` if the tokens are not a
/// well-formed register list. Grammar: `/`-separated items, each a single
/// register (`d3`, `a2`) or a contiguous range `lo-hi` (`d0-d7`, `a0-a6`, or
/// a d→a crossing range such as `d0-a4`). A range with `lo > hi` is rejected.
/// This is a total, side-effect-free recognizer: it returns `None` (rather than
/// diagnosing) on any non-list shape so the caller can use it to DISCRIMINATE
/// the register-list operand from the memory-EA operand of a `movem`.
fn parse_reg_list(toks: &[Token]) -> Option<u16> {
    if toks.is_empty() {
        return None;
    }
    let mut mask: u16 = 0;
    for item in split_slash(toks) {
        match item {
            // Single register: `d3`, `a2`, `sp`.
            [Token {
                tok: Tok::Ident(w), ..
            }] => {
                mask |= 1u16 << reg_list_index(w)?;
            }
            // Contiguous range: `d0-d7`, `a0-a6`, `d0-a4`.
            [Token {
                tok: Tok::Ident(lo),
                ..
            }, Token {
                tok: Tok::Punct(Punct::Minus),
                ..
            }, Token {
                tok: Tok::Ident(hi),
                ..
            }] => {
                let lo = reg_list_index(lo)?;
                let hi = reg_list_index(hi)?;
                if lo > hi {
                    return None;
                }
                for b in lo..=hi {
                    mask |= 1u16 << b;
                }
            }
            _ => return None,
        }
    }
    Some(mask)
}

/// Split a register-list operand's tokens on top-level `/` separators (a
/// register list never contains parentheses, so no depth tracking is needed).
fn split_slash(toks: &[Token]) -> Vec<&[Token]> {
    let mut groups = Vec::new();
    let mut start = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if matches!(t.tok, Tok::Punct(Punct::Slash)) {
            groups.push(&toks[start..i]);
            start = i + 1;
        }
    }
    groups.push(&toks[start..]);
    groups
}

/// Post-hoc mnemonic refinement for the operand-shape-dependent variants: a
/// `move` to/from the bare `sr` pseudo-register is really `MoveToSr`/
/// `MoveFromSr`; `andi`/`ori` targeting the bare `ccr` pseudo-register are
/// really `AndiCcr`/`OriCcr`. The encoder dispatches solely on `Mnemonic`, so
/// this must happen before building the `Instruction`.
fn refine_m68k_mnemonic(mnemonic: M68kMnemonic, ops: &[M68kOperand]) -> M68kMnemonic {
    use M68kMnemonic::*;
    match (mnemonic, ops) {
        (Move, [_, M68kOperand::Sr]) => MoveToSr,
        (Move, [M68kOperand::Sr, _]) => MoveFromSr,
        // `move <ea>,ccr` is its own opcode (44C0 | ea); there is no
        // move-FROM-ccr on the 68000, so a leading `ccr` is left to fail loud
        // in `encode_ea` ("ccr is not a general EA") rather than mis-refined.
        (Move, [_, M68kOperand::Ccr]) => MoveToCcr,
        (Move, [_, M68kOperand::Usp]) => MoveToUsp,
        (Move, [M68kOperand::Usp, _]) => MoveFromUsp,
        (Andi, [_, M68kOperand::Ccr]) => AndiCcr,
        (Ori, [_, M68kOperand::Ccr]) => OriCcr,
        // An immediate source into a MEMORY destination on the ALU forms is
        // asl's spelling of the corresponding `xxxi` immediate instruction:
        // `cmp #imm,(abs)` ≡ `cmpi`, `and #imm,(abs)` ≡ `andi`, etc. (byte-exact
        // asl-verified: `cmp.b #$80,($FFFF8000).l` == `cmpi.b …` == `0C39 …`).
        // A `Dn` destination is left alone — `add #4,d0` / `cmp #5,d0` are the
        // regular `<ea>,Dn` forms with an immediate source EA (distinct bytes).
        (Cmp, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Cmpi,
        // A `cmp` with an ADDRESS-register destination is asl's spelling of
        // `cmpa` (probe `probe_cmpa` 2026-07-05: `cmp.l a0,a1` == `cmpa.l a0,a1`
        // == `B3C8`). Only `debugger.asm`'s `assert` macro (`cmp.ATTRIBUTE
        // dest,src` with an An `dest`) exercises this — latent until __DEBUG__
        // (M1.D T5). `add`/`sub` have the analogous `adda`/`suba` aliases, but no
        // An-dest form of them appears in either build, so they are left to fail
        // loud if one ever does (never silently mis-encoded).
        (Cmp, [_, M68kOperand::An(_)]) => Cmpa,
        (And, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Andi,
        (Or, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Ori,
        (Add, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Addi,
        (Sub, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Subi,
        (Eor, [M68kOperand::Imm(_), d]) if is_mem_dest(d) => Eori,
        (m, _) => m,
    }
}

/// True for a 68k MEMORY effective-address destination (any alterable EA that is
/// neither a data nor address register). Used to route `cmp`/`and`/… `#imm,mem`
/// to their `cmpi`/`andi`/… immediate encodings (see `refine_m68k_mnemonic`).
fn is_mem_dest(op: &M68kOperand) -> bool {
    use M68kOperand::*;
    matches!(
        op,
        Ind(_) | PostInc(_) | PreDec(_) | Disp16An(..) | Disp8AnXn { .. } | AbsW(_) | AbsL(_)
    )
}

/// The `.`-local names a macro body defines as PLAIN LABELS — a `.name:` or a
/// column-0 `.name` carrying an instruction — as opposed to the `.name equ`,
/// `.name =`, `.name set` and `.name :=` forms, which BIND A VALUE.
///
/// AS scopes the two differently inside a macro expansion, and the difference is
/// syntactic, not value-kind (`asl -U`, the corpus's own flags):
///
/// * a plain label is private to the expansion and never enters the symbol
///   table — `mlab` twice under `Base:` gives `6702` in both expansions and
///   `dc.w .done-Base` after them is `error #1010: symbol undefined`;
/// * every value-binding form lands in the CALLER's scope and IS a symbol —
///   `.eq equ 3` / `.lb label *` / `.asn := 5` in a macro under `Base:` list as
///   `Base.eq : 3`, `Base.lb : 1000 C`, `Base.asn : 5`.
///
/// So this set is the whole difference: a `.`-local in this set resolves against
/// the expansion, everything else against the caller. It is computed from the
/// body ONCE, before the body runs, which is what makes the answer independent of
/// where in the body the reference sits — see [`Asm::dot_scope`].
///
/// A macro DEFINED inside this body owns its own labels, so those lines are
/// skipped; `rept`/`while`/`irp`/`irpc` are counted while skipping because they
/// close with `endm` too.
fn scan_dot_labels(body: &[SrcLine]) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    let mut nested = 0usize;
    for line in body {
        let text = line.text.as_str();
        let mut words = text.split_whitespace();
        let w0 = words.next().unwrap_or("");
        let w1 = words.next().unwrap_or("");
        let macro_head = fold_kw(w1) == "macro" || fold_kw(w0) == "macro";
        if nested > 0 {
            if macro_head || matches!(&*fold_kw(w0), "rept" | "while" | "irp" | "irpc") {
                nested += 1;
            } else if fold_kw(w0) == "endm" {
                nested -= 1;
            }
            continue;
        }
        if macro_head {
            nested = 1;
            continue;
        }
        let trimmed = text.trim_start();
        let indented = trimmed.len() != text.len();
        let Some(rest) = trimmed.strip_prefix('.') else {
            continue;
        };
        let len = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if len == 0 {
            continue;
        }
        let (name, tail) = rest.split_at(len);
        // `:=` is ONE token, so the colon strip must not bite it.
        let colon = tail.starts_with(':') && !tail.starts_with(":=");
        let tail = if colon { &tail[1..] } else { tail };
        let next = tail.trim_start();
        // A value binding, in any of its spellings — including the decorative
        // colon `exec_one` also tolerates (`.__pos: set …`).
        if next.starts_with(":=")
            || next.starts_with('=')
            || matches!(
                &*fold_kw(next.split_whitespace().next().unwrap_or("")),
                "equ" | "set" | "eval"
            )
        {
            continue;
        }
        // The two label shapes `exec_one` accepts, and only those: a COLON label
        // at any indentation (`parse_line_tokens` peels it wherever it sits —
        // aeon's `assert` macro writes its `\t.skip:` indented), and a bare one
        // at column 0 (AS's column rule, where an indented head is an
        // instruction instead).
        if colon || !indented {
            out.insert(format!(".{name}"));
        }
    }
    out
}

/// Qualify a name: `.local` → `Scope.local` (if scope); else unchanged.
fn qualify(name: &str, scope: Option<&str>) -> String {
    if name.starts_with('.') {
        match scope {
            Some(s) => format!("{s}{name}"),
            None => name.to_string(),
        }
    } else {
        name.to_string()
    }
}

/// asl zero-displacement optimization (probe-verified, unconditional — NOT
/// `-A`-gated; probe in the T4 notes' companion `zd` matrix): a `(d16,An)` EA
/// whose displacement is 0 encodes as `(An)`, dropping the extension word
/// (`move.b d0,0(a0)` → `1080` not `1140 0000`; `movem.l d0-d7,0(a0)` → `48D0`
/// not `48E8 …0000`). This must be decided in the FRONT END, not the encoder,
/// so the fragment carries the true (2-byte-shorter) EA length and the layout
/// cursor stays correct. Callers apply it to every EA-general instruction;
/// `movep` is the sole 68000 exception — mode 5 `(d16,An)` is its ONLY legal
/// addressing (no register-indirect form), so it keeps `03C8 0000`. Every other
/// EA instruction legally accepts mode 2 `(An)` wherever it accepts mode 5, so
/// the collapse is always safe (this is 68000-specific; 68020+ is out of scope
/// — Aeon pins `cpu 68000`).
///
/// Convergence: the displacement reaching here has already passed through
/// `fold_imm`, which coerces an unresolved (forward-ref) displacement to a
/// placeholder 0. So a Poison disp is optimized to `(An)` on an early pass and
/// GROWS to `(d16,An)` (2→4 bytes) once it resolves nonzero — the same
/// optimistic-short, grow-only discipline as the abs.w→abs.l / jmp-jsr width
/// machinery. A resolved-nonzero disp never shrinks back to 0, so the fixpoint
/// is monotone and converges to asl's minimal encoding (an extra pass, at worst,
/// for a forward-ref disp that transits the placeholder 0). Aeon's real
/// zero-disp sites (the `Init_DMA_Queue` `rept` at `.c=0`) resolve immediately
/// and never transit a placeholder.
fn collapse_zero_disp(op: &mut M68kOperand) {
    if let M68kOperand::Disp16An(0, n) = *op {
        *op = M68kOperand::Ind(n);
    }
}

/// Qualify a macro argument that is a bare `.`-local against the CALLER's scope.
/// A label passed into a macro (`…,.next_object,…`) names a symbol in the caller,
/// so it must be resolved in the caller's scope, not the expansion's private one
/// (see `expand_macro_inner`). Only a *clean* bare local (`.name`) is rewritten;
/// compound or non-local argument text passes through untouched.
///
/// KNOWN over-match (not exercised by Aeon): the predicate cannot distinguish a
/// bare local label from a lone size suffix / fractional token — a macro argument
/// of literally `.w`/`.b`/`.l`/`.5` also satisfies `.` + alphanumerics and would
/// be rewritten to `caller_scope.w` etc. Aeon never passes such a value as a
/// macro argument (size suffixes ride on the mnemonic, not an argument slot).
/// A bare `.`-local name: `.` followed by one or more identifier chars and
/// nothing else (e.g. `.next_object`, `.__operand`), as opposed to a dotted
/// expression or a `.`-suffixed literal.
fn is_bare_local(v: &str) -> bool {
    v.starts_with('.')
        && v.len() > 1
        && v[1..].chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn on_off(rest: &[Token]) -> bool {
    !matches!(rest.first().map(|t| &t.tok), Some(Tok::Ident(w)) if fold_kw(w) == "off")
}

fn paren(p: Punct, span: Span) -> Token {
    Token {
        tok: Tok::Punct(p),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::Options;
    use sigil_ir::backend::Cpu;
    use sigil_ir::Module;

    /// Assemble AND LINK, so branch displacements to labels are the RESOLVED
    /// bytes rather than the front-end's unapplied-fixup placeholders. Every
    /// listing row that pins a `beq.s` target needs this rather than [`image`].
    fn linked_image(src: &str) -> Vec<u8> {
        let m = run(src, &Options::default()).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        sigil_link::flatten(&linked, 0x00)
    }

    /// Whether the source assembles AND LINKS. An unresolved symbol survives
    /// the front end as a deferred fixup, so it is the LINK that refuses it —
    /// `run` alone returns `Ok` and would make an absent-symbol assertion
    /// vacuous.
    fn links(src: &str) -> bool {
        let Ok(m) = run(src, &Options::default()) else {
            return false;
        };
        let Ok(resolved) =
            sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
        else {
            return false;
        };
        sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).is_ok()
    }

    /// The offset a named label occupies in the assembled module's sections, or
    /// `None` where no section carries it. This is the linker's own view of a
    /// label — `image`/`linked_image` cannot see it, because a front-end fold
    /// can produce the same bytes from a constant of equal value.
    fn section_label(src: &str, name: &str) -> Option<u32> {
        let m = run(src, &Options::default()).expect("assemble");
        m.sections
            .iter()
            .flat_map(|s| s.labels.iter())
            .find(|l| l.name == name)
            .map(|l| l.offset)
    }

    fn image(src: &str) -> Vec<u8> {
        let m = run(src, &Options::default()).expect("assemble");
        m.sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default()
    }

    /// The image bytes a leading `ds.l 1` contributes before anything that
    /// follows it in the same section. A reservation advances the write cursor,
    /// so the next thing to write fills the gap rather than packing over it,
    /// and every offset into such a section is measured from here.
    const DS_L_1_GAP: usize = 4;

    /// Assert an image the way `p2bin` reports one for a `ds`-then-emit shape:
    /// a reservation advances the write cursor, so the gap it opens is filled
    /// by whatever writes next and the image spans the whole reserved range.
    /// These images run to tens of kilobytes for a RAM-side probe, so the
    /// assertion is total size, the trailing bytes, and zero everywhere else —
    /// which for these shapes is the entire rest of the image.
    ///
    /// The expected pairs are read off `p2bin`'s own output, not off sigil's:
    /// `asl -xx -n -q -A -L -U -i .` then `p2bin <probe>.p <probe>.bin`.
    fn assert_p2bin_image(src: &str, size: usize, tail: &[u8]) {
        let img = image(src);
        assert_eq!(img.len(), size, "image size (p2bin's is {size})");
        let split = size - tail.len();
        assert_eq!(&img[split..], tail, "image tail at offset {split:#x}");
        assert!(
            img[..split].iter().all(|&b| b == 0),
            "the reservation's gap fills with zero: byte {:?} is not",
            img[..split].iter().position(|&b| b != 0)
        );
    }

    /// Address-register-destination ALU hygiene (effects-P2 corruption fix,
    /// 2026-08-12). End-to-end: `add/sub dN,aM` must FAIL to assemble (they alias
    /// ADDX/SUBX — `D549`-style silent memory corruption), matching this file's
    /// stated "left to fail loud" intent for add/sub. `cmp` An-dest is asl's `cmpa`
    /// spelling (promoted). The explicit `adda`/`suba`/`cmpa` spellings assemble.
    #[test]
    fn alu_address_register_destination_spelling_probes() {
        let head = "        cpu 68000\n        padding off\n        phase 0\n";
        // adda/suba/cmpa explicit spellings assemble to the address-arithmetic words.
        assert_eq!(image(&format!("{head}        adda.w a0,a1\n")), vec![0xD2, 0xC8]);
        assert_eq!(image(&format!("{head}        suba.l a0,a1\n")), vec![0x93, 0xC8]);
        assert_eq!(image(&format!("{head}        cmpa.l a0,a1\n")), vec![0xB3, 0xC8]);
        // `cmp.l a0,a1` promotes to cmpa (asl parity — the debugger `assert` macro).
        assert_eq!(image(&format!("{head}        cmp.l a0,a1\n")), vec![0xB3, 0xC8]);
        // `add.w dN,aM` / `sub` now FAIL LOUD (were silent ADDX garbage).
        assert!(run(&format!("{head}        add.w d2,a1\n"), &Options::default()).is_err(),
                "add.w d2,a1 must fail to assemble (needs adda)");
        assert!(run(&format!("{head}        sub.l d0,a1\n"), &Options::default()).is_err(),
                "sub.l d0,a1 must fail to assemble (needs suba)");
    }

    /// Every `EquSym` named `name` across all sections of an assembled module
    /// (Task B1) — a `Vec` so a test can assert exactly-one-ness itself rather
    /// than this helper silently picking the first/last.
    fn equ_syms_named<'a>(m: &'a Module, name: &str) -> Vec<&'a sigil_ir::EquSym> {
        m.sections
            .iter()
            .flat_map(|s| s.equ_syms.iter())
            .filter(|e| e.name == name)
            .collect()
    }

    #[test]
    fn int_equ_is_exported_exactly_once_with_final_value() {
        // Task B1: `FOO equ $B` must export exactly one `EquSym("FOO", Int(0xB))`
        // to the module — the AS-side `equ` now reaches the linker's symbol
        // table, not just the front-end's private fold env.
        let m = run("\tcpu 68000\nFOO equ $B\n\tdc.b 0\n", &Options::default()).expect("assemble");
        let syms = equ_syms_named(&m, "FOO");
        assert_eq!(syms.len(), 1, "expected exactly one EquSym(FOO), got {syms:?}");
        assert_eq!(syms[0].expr, sigil_ir::Expr::Int(0xB));
    }

    #[test]
    fn string_equ_is_not_exported() {
        // The string-equ shape (`GAME_CONSOLE equ "SEGA GENESIS    "`) stays
        // front-end-only (§7.4) — it must NOT reach the module's equ_syms.
        let m = run(
            "\tcpu 68000\nGAME_CONSOLE equ \"SEGA GENESIS    \"\n\tdc.b 0\n",
            &Options::default(),
        )
        .expect("assemble");
        assert!(
            equ_syms_named(&m, "GAME_CONSOLE").is_empty(),
            "a string equ must not be exported"
        );
    }

    #[test]
    fn set_directive_is_not_exported() {
        // `set`/`:=` is a separate, reassignable-symbol directive (T8) — it
        // must never export an EquSym, only `equ` does.
        let m = run("\tcpu 68000\nbar set 1\n\tdc.b 0\n", &Options::default()).expect("assemble");
        assert!(
            equ_syms_named(&m, "bar").is_empty(),
            "a `set` symbol must not be exported as an equ"
        );
    }

    #[test]
    fn ifndef_guarded_equs_and_structs_still_export() {
        // Tranche-3 branch-review finding (latent): pass 0 executes an
        // `ifndef`-guarded block and exports its equs; the guard symbol is
        // then seeded into later passes, so the CONVERGED pass skips the
        // block — bytes correct, but the export side effect vanished with
        // it, and any `.emp` `extern("FOO")` failed with a misleading
        // "unresolved symbol". The run loop now carries the ever-exported
        // set across passes and re-attaches missing exports from the
        // CONVERGED env (values authoritative — a forward-ref-dependent equ
        // gets its final value, not pass 0's).
        let src = "\tcpu 68000\n\
                   \tifndef GUARD\n\
                   GUARD = 1\n\
                   FOO = 42\n\
                   V struct\n\
                   a ds.b 1\n\
                   b ds.w 1\n\
                   V endstruct\n\
                   \tendif\n\
                   Stub:\n\
                   \tdc.b 0\n";
        let m = run(src, &Options::default()).expect("assemble");
        let foo = equ_syms_named(&m, "FOO");
        assert_eq!(foo.len(), 1, "guarded FOO must still export exactly once, got {foo:?}");
        assert_eq!(foo[0].expr, sigil_ir::Expr::Int(42));
        let vlen = equ_syms_named(&m, "V_len");
        assert_eq!(vlen.len(), 1, "guarded V_len must still export exactly once, got {vlen:?}");
        let guard = equ_syms_named(&m, "GUARD");
        assert_eq!(guard.len(), 1, "the guard symbol itself is an int equate and exports too");
    }

    #[test]
    fn struct_len_and_field_offsets_are_exported_as_equ_syms() {
        // Tranche 3 (the vdp_init constants-twin): struct-generated
        // `Name_len` / `Name_field` symbols are int equates SEMANTICALLY
        // (comptime constants derived from struct layout), so they ride the
        // same Item-B export path as `equ` — an `.emp` drift guard reads
        // them via `extern("VDP_Shadow_len")` exactly like a hand-written
        // equate. `padding off` (Aeon's global setting): b at 1, len 3.
        let m = run(
            "\tcpu 68000\n\tpadding off\nV struct\na ds.b 1\nb ds.w 1\nV endstruct\n\tdc.b 0\n",
            &Options::default(),
        )
        .expect("assemble");
        let len = equ_syms_named(&m, "V_len");
        assert_eq!(len.len(), 1, "expected exactly one EquSym(V_len), got {len:?}");
        assert_eq!(len[0].expr, sigil_ir::Expr::Int(3));
        let a = equ_syms_named(&m, "V_a");
        assert_eq!(a.len(), 1, "expected exactly one EquSym(V_a), got {a:?}");
        assert_eq!(a[0].expr, sigil_ir::Expr::Int(0));
        let b = equ_syms_named(&m, "V_b");
        assert_eq!(b.len(), 1, "expected exactly one EquSym(V_b), got {b:?}");
        assert_eq!(b[0].expr, sigil_ir::Expr::Int(1));
    }

    #[test]
    fn label_referencing_equ_exports_the_final_folded_value_exactly_once() {
        // `Foo_len equ Foo_End-Foo` can only fold to a concrete int once every
        // label's address is known — which happens only on the CONVERGED pass.
        // Exactly one EquSym must survive (the one from the converged builder),
        // carrying the fully-folded final value, not an intermediate guess.
        let src = "\tcpu 68000\nFoo:\n\tdc.b 1,2,3,4,5\nFoo_End:\nFoo_len equ Foo_End-Foo\n\tdc.b 0\n";
        let m = run(src, &Options::default()).expect("assemble");
        let syms = equ_syms_named(&m, "Foo_len");
        assert_eq!(syms.len(), 1, "expected exactly one EquSym(Foo_len), got {syms:?}");
        assert_eq!(syms[0].expr, sigil_ir::Expr::Int(5));
    }

    #[test]
    fn leading_comparison_operand_does_not_panic() {
        // Regression (M1.D T5 review): `expand_str_comparisons` sees a `<>`/`=`
        // with a string-literal RHS and an EMPTY left context, so
        // `trailing_str_expr_len` must return None rather than underflow-panic on
        // `out[n-1]`. The malformed operand must be handled gracefully (a
        // diagnostic or the pre-existing silent-fold) — the point is it must not
        // CRASH the assembler. Reaching this assert at all means no panic fired.
        for src in ["\tcpu 68000\n\tdc.b <>\"x\"\n", "\tcpu 68000\n\tdc.b =\"x\"\n"] {
            let _ = run(src, &Options::default());
        }
    }

    #[test]
    fn split_mnemonic_and_size_strips_known_suffixes() {
        use super::split_mnemonic_and_size;
        use sigil_backend_m68k::m68k::Size;
        assert_eq!(split_mnemonic_and_size("move.w"), ("move", Some(Size::W)));
        assert_eq!(split_mnemonic_and_size("move.l"), ("move", Some(Size::L)));
        assert_eq!(split_mnemonic_and_size("clr.b"), ("clr", Some(Size::B)));
        assert_eq!(split_mnemonic_and_size("bra.s"), ("bra", Some(Size::S)));
        assert_eq!(split_mnemonic_and_size("moveq"), ("moveq", None));
        assert_eq!(split_mnemonic_and_size("swap"), ("swap", None));
    }

    #[test]
    fn m68k_mnemonic_recognizes_in_scope_bases() {
        use super::m68k_mnemonic;
        use sigil_backend_m68k::m68k::Mnemonic;
        assert_eq!(m68k_mnemonic("move"), Some(Mnemonic::Move));
        assert_eq!(m68k_mnemonic("moveq"), Some(Mnemonic::Moveq));
        assert_eq!(m68k_mnemonic("addq"), Some(Mnemonic::Addq));
        assert_eq!(m68k_mnemonic("swap"), Some(Mnemonic::Swap));
        assert_eq!(m68k_mnemonic("ext"), Some(Mnemonic::Ext));
        assert_eq!(m68k_mnemonic("nop"), Some(Mnemonic::Nop));
        assert_eq!(m68k_mnemonic("rts"), Some(Mnemonic::Rts));
        assert_eq!(m68k_mnemonic("rte"), Some(Mnemonic::Rte));
        // T5 adds the fixed-length EA family plus `lea`/`pea` — both in-scope now.
        assert_eq!(m68k_mnemonic("lea"), Some(Mnemonic::Lea));
        assert_eq!(m68k_mnemonic("pea"), Some(Mnemonic::Pea));
        // T5c adds control transfer: branches, Dbcc, Scc, jmp/jsr.
        assert_eq!(m68k_mnemonic("jmp"), Some(Mnemonic::Jmp));
        assert_eq!(m68k_mnemonic("jsr"), Some(Mnemonic::Jsr));
        assert_eq!(m68k_mnemonic("bra"), Some(Mnemonic::Bra));
        assert_eq!(m68k_mnemonic("bsr"), Some(Mnemonic::Bsr));
        assert_eq!(
            m68k_mnemonic("beq"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("bne"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Ne))
        );
        assert_eq!(
            m68k_mnemonic("dbf"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F))
        );
        assert_eq!(
            m68k_mnemonic("dbra"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::F))
        );
        assert_eq!(
            m68k_mnemonic("dbeq"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("scc"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("seq"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Eq))
        );
        assert_eq!(
            m68k_mnemonic("st"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::T))
        );
        // Unsigned-branch aliases: bhs == bcc (carry-clear), blo == bcs
        // (carry-set); same for shs/slo and dbhs/dblo. Aeon uses bhs/blo.
        assert_eq!(
            m68k_mnemonic("bhs"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("blo"),
            Some(Mnemonic::Bcc(sigil_backend_m68k::m68k::Cond::Cs))
        );
        assert_eq!(
            m68k_mnemonic("shs"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        assert_eq!(
            m68k_mnemonic("slo"),
            Some(Mnemonic::Scc(sigil_backend_m68k::m68k::Cond::Cs))
        );
        assert_eq!(
            m68k_mnemonic("dbhs"),
            Some(Mnemonic::Dbcc(sigil_backend_m68k::m68k::Cond::Cc))
        );
        // `movem`/`movep` are now in scope (register-list operands).
        assert_eq!(m68k_mnemonic("movem"), Some(Mnemonic::Movem));
        assert_eq!(m68k_mnemonic("movep"), Some(Mnemonic::Movep));
        // `cmpm` (F3): encoder always had it; the front-end table did not until
        // M1.D T0.4. Exposed only under __DEBUG__ (compression_selftest.asm:83).
        assert_eq!(m68k_mnemonic("cmpm"), Some(Mnemonic::Cmpm));
        // a genuinely unrecognized word is not misparsed as a stray cc suffix.
        assert_eq!(m68k_mnemonic("banana"), None);
    }

    #[test]
    fn m68k_cond_parses_all_16_condition_codes() {
        use super::m68k_cond;
        use sigil_backend_m68k::m68k::Cond;
        let pairs = [
            ("t", Cond::T),
            ("f", Cond::F),
            ("hi", Cond::Hi),
            ("ls", Cond::Ls),
            ("cc", Cond::Cc),
            ("cs", Cond::Cs),
            ("ne", Cond::Ne),
            ("eq", Cond::Eq),
            ("vc", Cond::Vc),
            ("vs", Cond::Vs),
            ("pl", Cond::Pl),
            ("mi", Cond::Mi),
            ("ge", Cond::Ge),
            ("lt", Cond::Lt),
            ("gt", Cond::Gt),
            ("le", Cond::Le),
        ];
        for (w, c) in pairs {
            assert_eq!(m68k_cond(w), Some(c), "cc word `{w}`");
        }
        assert_eq!(m68k_cond("xx"), None);
    }

    // ---------------------------------------------------------------------
    // AS `STRUCT … DOTS`. Every expectation below is a byte column read off an
    // asl 1.42 Bld 212 listing (`-xx -n -q -A -L -U -E -i .`, the S1 binary,
    // md5 61e672562465725a8c102288a7da9098). The probe files are committed at
    // `docs/superpowers/notes/2026-09-03-as-struct-probes/`.
    //
    // THREE OF THESE RULES ARE PINNED BY NOTHING ELSE. The 1,553-symbol RAM
    // byte sweep against Sonic 1's `_Variables.asm` stays GREEN under a
    // deliberate break of each: instance alignment (probe q9), embed
    // re-alignment (q10) and the pre-/post-pad table split (q7) are all
    // unreachable in that corpus, because `v_snddriver_ram` lands at the even
    // $FFFFF000, `SMPS_Track.len` is an even $30, and neither struct has a
    // `ds.w`/`ds.l` member at an odd offset. These tests are the only gate.
    // ---------------------------------------------------------------------

    /// `q7.asm`. asl records TWO offsets per member and they disagree wherever
    /// `padding on` inserts a pad byte. The DECLARATION-SCOPE symbol takes the
    /// offset BEFORE the pad; the struct element — which is what an
    /// instantiation reads — takes it after.
    ///
    /// ```text
    ///    9/ 1000 : 0000 0001 0004          dc.w S.a,S.b,S.c,S.d,S.len
    ///       1006 : 0005 000A
    ///   10/ 100A : (STRUCT)             inst:    S
    ///   11/ 1014 : 0000 0002 0004          dc.w inst.a-inst,inst.b-inst,inst.c-inst,inst.d-inst
    ///       101A : 0006
    /// ```
    #[test]
    fn struct_symbols_are_pre_pad_while_elements_are_post_pad() {
        let decl = "S struct DOTS\n\
                    a:\tds.b 1\n\
                    b:\tds.w 1\n\
                    c:\tds.b 1\n\
                    d:\tds.l 1\n\
                    \tendstruct\n";
        // The declaration-scope symbols: 0, 1, 4, 5, and len $A.
        assert_eq!(
            image(&format!("\tcpu 68000\n\torg $1000\n{decl}\tdc.w S.a,S.b,S.c,S.d,S.len\n")),
            vec![0, 0, 0, 1, 0, 4, 0, 5, 0, 0x0A]
        );
        // The same members through an instance: 0, 2, 4, 6.
        assert_eq!(
            image(&format!(
                "\tcpu 68000\n\torg $1000\n{decl}\
                 \tdc.w inst.a-inst,inst.b-inst,inst.c-inst,inst.d-inst\ninst:\tS\n"
            )),
            vec![0, 0, 0, 2, 0, 4, 0, 6]
        );
    }

    /// `q9.asm`. An instance is placed VERBATIM and is never word-aligned, even
    /// under `padding on` and even when the struct leads with a `ds.w` — while
    /// a bare `ds.w 1` two lines away at the same odd address does pad. asl
    /// puts `i1: W` at the odd $2001 and `a1` at $2003.
    #[test]
    fn a_struct_instance_is_never_word_aligned() {
        // `q14.asm`, byte-identical to asl. The `dc.w` occupies $1000..$1008
        // and the `ds.b 1` leaves the PC at the ODD $1009, where the
        // word-leading instance is placed anyway — so `after` is $100C, and the
        // MEMBERS hang off the odd base rather than off a rounded one.
        //
        // ```text
        //    7/ 1000 : 1009 100C 0000      	dc.w i1,after,i1.w-i1,i1.x-i1
        //       1006 : 0002
        //    9/ 1009 : (STRUCT)             i1:	W
        //   10/ 100C :                     after:
        // ```
        let src = "\tcpu 68000\n\torg $1000\n\
                   W struct DOTS\n\
                   w:\tds.w 1\n\
                   x:\tds.b 1\n\
                   \tendstruct\n\
                   \tdc.w i1,after,i1.w-i1,i1.x-i1\n\
                   \tds.b 1\n\
                   i1:\tW\n\
                   after:\n";
        assert_eq!(
            &image(src)[..8],
            &[0x10, 0x09, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x02]
        );
    }

    /// `q10.asm`. A struct embedded in another struct is placed at the parent's
    /// running offset with NO re-alignment, and its own element table is
    /// flattened in verbatim — so an inner `ds.w` member can land at an ODD
    /// parent offset. asl, for `h: ds.b 1` then `n: T` where `T` is
    /// `p: ds.b 1 / r: ds.w 1`:
    ///
    /// ```text
    ///   16/ 1006 : 0000 0001 0001          dc.w S.h,S.n,S.n.p,S.n.r,S.z,S.len
    ///       100C : 0003 0005 0006
    /// ```
    ///
    /// This is Sonic 1's `SMPS_RAM`, which embeds `SMPS_Track` eighteen times
    /// and is why `SMPS_RAM.v_music_dac_track.PlaybackControl` is a name.
    #[test]
    fn an_embedded_struct_is_flattened_and_never_re_aligned() {
        let src = "\tcpu 68000\n\torg $1000\n\
                   T struct DOTS\n\
                   p:\tds.b 1\n\
                   r:\tds.w 1\n\
                   \tendstruct\n\
                   S struct DOTS\n\
                   h:\tds.b 1\n\
                   n:\tT\n\
                   z:\tds.b 1\n\
                   \tendstruct\n\
                   \tdc.w S.h,S.n,S.n.p,S.n.r,S.z,S.len\n";
        assert_eq!(
            image(src),
            vec![0, 0, 0, 1, 0, 1, 0, 3, 0, 5, 0, 6],
            "the inner `r` sits at the ODD parent offset 3"
        );
    }

    /// `q8.asm` / `q11.asm`. The separator is a property of the STRUCT, not of
    /// the site: a declaration without `DOTS` yields `A_a`/`A_len`, and an
    /// INSTANCE of it yields `j_u` — while `q10.asm` shows `j.u` is undefined
    /// there (asl `#1010`). `DOTS` is recognized case-folded, because Sonic 2
    /// writes `struct dots` at one site and `STRUCT DOTS` at three.
    #[test]
    fn dots_selects_the_separator_for_declaration_and_instance_alike() {
        let src = "\tcpu 68000\n\torg $1000\n\
                   A struct\n\
                   a:\tds.b 1\n\
                   b:\tds.b 1\n\
                   \tendstruct\n\
                   B struct dots\n\
                   a:\tds.b 1\n\
                   \tendstruct\n\
                   \tdc.w A_a,A_b,A_len,B.a,B.len\n\
                   \tdc.w j_a-j,j_b-j\nj:\tA\n";
        assert_eq!(image(src), vec![0, 0, 0, 1, 0, 2, 0, 0, 0, 1, 0, 0, 0, 1]);
    }

    /// A member that is a bare label reserves nothing and binds the running
    /// offset; Sonic 1's `SMPS_RAM` has 21 of them and reads four back BY NAME
    /// from inside its own body (`ds.b SMPS_RAM.v_1up_ram_end-SMPS_RAM.v_1up_ram`,
    /// `s1.sounddriver.ram.asm:108`). That self-reference is also the reason
    /// members are bound as the body is WALKED rather than at `endstruct`.
    #[test]
    fn markers_bind_the_running_offset_and_are_readable_within_the_body() {
        let src = "\tcpu 68000\n\torg $1000\n\
                   S struct DOTS\n\
                   first:\n\
                   a:\tds.b 3\n\
                   mid:\n\
                   b:\tds.b 2\n\
                   last:\n\
                   copy:\tds.b S.last-S.mid\n\
                   \tendstruct\n\
                   \tdc.w S.first,S.mid,S.last,S.copy,S.len\n";
        // first=0, mid=3, last=5, copy=5, and copy is `last-mid` = 2 wide, so
        // len = 7.
        assert_eq!(image(src), vec![0, 0, 0, 3, 0, 5, 0, 5, 0, 7]);
    }

    /// `q12.asm` / `q8.asm`. Both corpora close three of their six structs with
    /// the struct's own name in the LABEL column (`SoundQueue ENDSTRUCT`), and
    /// asl also accepts `ends`. All three spellings must close the block.
    #[test]
    fn a_struct_closes_on_endstruct_named_or_bare_and_on_ends() {
        for closer in ["\tendstruct\n", "S ENDSTRUCT\n", "\tends\n"] {
            let src = format!(
                "\tcpu 68000\n\torg $1000\n\
                 S STRUCT DOTS\n\
                 \ta:\tds.b 1\n\
                 \tb:\tds.b 1\n\
                 {closer}\
                 \tdc.w S.a,S.b,S.len\n"
            );
            assert_eq!(image(&src), vec![0, 0, 0, 1, 0, 2], "closer `{closer:?}`");
        }
    }

    /// An anonymous `ds.*` advances the offset and binds no name — Sonic 1's
    /// `SMPS_RAM` is 12 such gaps, one of them `ds.b $13`, and Sonic 2's
    /// `HorizontalScrollBuffer` is nothing BUT anonymous fields.
    #[test]
    fn anonymous_reserve_fields_advance_the_offset_and_bind_nothing() {
        let src = "\tcpu 68000\n\torg $1000\n\
                   H struct dots\n\
                   \tds.l 224\n\
                   \tds.l 16\n\
                   \tds.b $40\n\
                   H endstruct\n\
                   \tdc.w H.len\n";
        // 224*4 + 16*4 + $40 = $380 + $40 + $40 = $400.
        assert_eq!(image(src), vec![0x04, 0x00]);
    }

    /// `q19.asm`. A struct-body line this cannot read is a wrong SIZE, not a
    /// missing symbol, so it is reported rather than skipped.
    ///
    /// Sonic 2's `zVar` declares `1upPlaying: ds.b 1`; asl takes an identifier
    /// beginning with a digit and sigil's lexer does not. Skipped, that made
    /// `zVar.len` $17 against asl's $18 — **exit 0 on both sides, no
    /// diagnostic anywhere, and every member after it one byte low.**
    #[test]
    fn an_unreadable_struct_member_line_is_reported_not_skipped() {
        let src = "\tcpu 68000\n\torg $0\n\
                   V struct dots\n\
                   \ta:\tds.b 1\n\
                   \t1upPlaying:\tds.b 1\n\
                   \tb:\tds.b 1\n\
                   V endstruct\n\
                   \tdc.w V.a,V.b,V.len\n";
        let diags = run(src, &Options::default()).expect_err("must not assemble");
        assert!(
            diags.iter().any(|d| d.message.contains("member line this cannot read")),
            "expected the wrong-size refusal, got {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn m68k_register_words_recognized() {
        use super::{m68k_addr_reg, m68k_data_reg};
        assert_eq!(m68k_data_reg("d0"), Some(0));
        assert_eq!(m68k_data_reg("d7"), Some(7));
        assert_eq!(m68k_data_reg("d8"), None);
        assert_eq!(m68k_data_reg("a0"), None);
        assert_eq!(m68k_addr_reg("a0"), Some(0));
        assert_eq!(m68k_addr_reg("a7"), Some(7));
        assert_eq!(m68k_addr_reg("sp"), Some(7));
        assert_eq!(m68k_addr_reg("d0"), None);
    }

    #[test]
    fn parse_reg_list_builds_canonical_masks() {
        use super::parse_reg_list;
        use crate::lexer::lex_line;
        let mask = |s: &str| {
            let toks = lex_line(s, Cpu::M68000, sigil_span::SourceId(0), 0).unwrap();
            parse_reg_list(&toks)
        };
        // Single reg: bit0=D0..bit7=D7, bit8=A0..bit15=A7 (canonical order).
        assert_eq!(mask("d0"), Some(0x0001));
        assert_eq!(mask("a2"), Some(0x0400));
        assert_eq!(mask("sp"), Some(0x8000)); // sp == a7 == bit15
                                              // `/` list mixing d and a.
        assert_eq!(mask("a2/d2"), Some(0x0404));
        // `-` range.
        assert_eq!(mask("d0-d3"), Some(0x000F));
        assert_eq!(mask("a0-a6"), Some(0x7F00));
        // Range crossing the d→a boundary is contiguous in canonical order.
        assert_eq!(mask("d0-a4"), Some(0x1FFF));
        // Range + list combined.
        assert_eq!(mask("d0-d7/a0-a6"), Some(0x7FFF));
        assert_eq!(mask("d0-d6/a2"), Some(0x047F));
        // Not a register list.
        assert_eq!(mask("(a0)"), None);
        assert_eq!(mask("-(sp)"), None);
        assert_eq!(mask("d0-x9"), None);
        assert_eq!(mask("d7-d0"), None); // reversed range rejected
    }

    #[test]
    fn m68k_movem_predec_reverses_mask_but_postinc_does_not() {
        // STORE to `-(An)` predecrement: the encoder REVERSES the canonical
        // mask. `a2/d2` canonical = 0x0404 → emitted word 0x2020 (48 E7 20 20).
        assert_eq!(
            image("    cpu 68000\n    movem.l a2/d2,-(sp)\n"),
            vec![0x48, 0xE7, 0x20, 0x20]
        );
        // LOAD from `(An)+` postincrement: canonical mask emitted as-is.
        // `d0-d7/a0-a6` canonical = 0x7FFF → 4C DF 7F FF.
        assert_eq!(
            image("    cpu 68000\n    movem.l (sp)+,d0-d7/a0-a6\n"),
            vec![0x4C, 0xDF, 0x7F, 0xFF]
        );
    }

    #[test]
    fn m68k_movem_single_range_and_mixed_lists() {
        // Single register store to predec: canonical 0x0400 → reversed 0x0020.
        assert_eq!(
            image("    cpu 68000\n    movem.l a2,-(sp)\n"),
            vec![0x48, 0xE7, 0x00, 0x20]
        );
        // Range store to plain (An) indirect: NOT reversed (0x0018).
        assert_eq!(
            image("    cpu 68000\n    movem.l d3-d4,(a3)\n"),
            vec![0x48, 0xD3, 0x00, 0x18]
        );
        // Word range store to predec a7: canonical 0x000F → reversed 0xF000.
        assert_eq!(
            image("    cpu 68000\n    movem.w d0-d3,-(a7)\n"),
            vec![0x48, 0xA7, 0xF0, 0x00]
        );
        // Mixed range+single load from postinc crossing d→a: 0x1FFF, not reversed.
        assert_eq!(
            image("    cpu 68000\n    movem.l (a0)+,d0-a4\n"),
            vec![0x4C, 0xD8, 0x1F, 0xFF]
        );
        // Disp16(An) memory EA store (extension word follows the mask word).
        assert_eq!(
            image("    cpu 68000\n    movem.l d3-d4,(8,a3)\n"),
            vec![0x48, 0xEB, 0x00, 0x18, 0x00, 0x08]
        );
    }

    #[test]
    fn m68k_movep_both_directions() {
        // reg → mem (word): 01 89 00 04.
        assert_eq!(
            image("    cpu 68000\n    movep.w d0,4(a1)\n"),
            vec![0x01, 0x89, 0x00, 0x04]
        );
        // mem → reg (long): 03 4A 00 08.
        assert_eq!(
            image("    cpu 68000\n    movep.l 8(a2),d1\n"),
            vec![0x03, 0x4A, 0x00, 0x08]
        );
    }

    #[test]
    fn m68k_register_indirect_operand_now_lowers_in_t5() {
        // `(a0)` is a register-indirect EA — T4 deferred it to T5; T5 (this
        // task) implements the fixed-length `(An)` family, so it now lowers
        // byte-exact instead of erroring. Bytes verified against real asl
        // (see `m68k_move_w_ind_a0_to_d0` in `tests/snippets_golden.txt`).
        assert_eq!(
            image("    cpu 68000\n    move.w (a0),d0\n"),
            vec![0x30, 0x10]
        );
    }

    #[test]
    fn m68k_pcrelative_disp16_lowers_via_resolve_layout_link() {
        // `(d16,PC)` (T5c): the front-end emits an unresolved `PcRelDisp16`
        // fixup (via `lower_pcrel_ea`); resolving it needs a real link (the
        // front-end's own fold never sees it — see `apply_fixup` in
        // `sigil-link`). `move.w (8,pc),d0` at VMA 0: the extension word sits
        // at offset 2, target = 8, disp = 8 - 2 = 6.
        let src = "    cpu 68000\n    phase 0\n    move.w (8,pc),d0\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // move.w (d16,PC),d0 = 30 3A, then disp word 00 06.
        assert_eq!(bytes, vec![0x30, 0x3A, 0x00, 0x06]);
    }

    #[test]
    fn m68k_pcrelative_disp8_indexed_lowers() {
        // `(d8,PC,Xn)` now lowers to a brief extension word + `PcRelDisp8` fixup.
        // asl-verified: `move.w (8,pc,d0.w),d1` at VMA 0 → `32 3B 00 06` (the
        // literal `8` is a TARGET address; disp = 8 - ext-word-VMA(2) = 6).
        let src = "    cpu 68000\n    phase 0\n    move.w (8,pc,d0.w),d1\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x32, 0x3B, 0x00, 0x06]);
    }

    #[test]
    fn m68k_divs_word_encodes() {
        // Signed word divide `<ea>,Dn`. Same EA machinery as muls, base 0b1000
        // (muls is 0b1100), opmode 111 (signed). asl-verified (tools/asl):
        //   divs.w d4,d2      = 85 C4
        //   divs.w #10,d2     = 85 FC 00 0A
        //   divs.w d0,d1      = 83 C0
        //   divs.w ($1234).w,d0 = 81 F8 12 34
        let src = "    cpu 68000\n    divs.w d4,d2\n    divs.w #10,d2\n    divs.w d0,d1\n    divs.w ($1234).w,d0\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(
            bytes,
            vec![0x85, 0xC4, 0x85, 0xFC, 0x00, 0x0A, 0x83, 0xC0, 0x81, 0xF8, 0x12, 0x34]
        );
    }

    #[test]
    fn m68k_divu_word_encodes() {
        // Unsigned word divide — opmode 011 (unsigned), base 0b1000. asl-verified:
        //   divu.w d4,d2 = 84 C4
        //   divu.w d3,d5 = 8A C3
        let src = "    cpu 68000\n    divu.w d4,d2\n    divu.w d3,d5\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x84, 0xC4, 0x8A, 0xC3]);
    }

    #[test]
    fn m68k_bare_absolute_operand_width_selects_abs_w() {
        // A bare number (no `#`, no parens) means 68k absolute addressing. Since
        // M1.D T2 this is in scope: asl width-selects abs.w for a target in
        // [0,$7FFF]∪[$FF8000,$FFFFFF]. `$1234` ≤ $7FFF → abs.w: `30 38 12 34`.
        let src = "    cpu 68000\n    move.w $1234,d0\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x30, 0x38, 0x12, 0x34]);
    }

    #[test]
    fn m68k_branch_without_size_suffix_is_a_clear_diagnostic() {
        // T5c: `bra`/`Bcc` are now in scope, but Aeon pins branch width by an
        // explicit `.s`/`.w` suffix (no relaxation) — a bare `bra` must still
        // error, just with a size-suffix diagnostic instead of a scope one.
        let src = "    cpu 68000\n    bra Target\nTarget:\n    rts\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let diags = run(src, &opts)
            .expect_err("branch without a size suffix must be rejected, not lowered");
        assert!(
            diags.iter().any(|d| d.message.contains("size suffix")),
            "expected a size-suffix diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn m68k_bra_w_qualifies_local_target_against_current_scope() {
        // A bare `.local` branch target must be qualified to `Scope.local`
        // BEFORE lowering (the linker resolves in global scope only) — this
        // is the exact hazard `qualify_expr` exists for. `Start:` opens scope
        // `Start`, so `bra.w .loop` must resolve against `Start.loop`, not a
        // bare `.loop` (which the linker would never find).
        let src = "    cpu 68000\n    phase 0\nStart:\n    bra.w .loop\n.loop:\n    rts\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // bra.w .loop: op@0, disp word@2, target=4 (right after the 4-byte
        // branch), disp = 4-2 = 2; then rts (4E75) at the target.
        assert_eq!(bytes, vec![0x60, 0x00, 0x00, 0x02, 0x4E, 0x75]);
    }

    #[test]
    fn m68k_jmp_jsr_bare_symbol_selects_width_in_front_end() {
        // `jmp Lbl`/`jsr Lbl` selects its abs.w/abs.l width in the front-end
        // pass loop (M1.D T3) and emits a finished `Fragment::Data` — NOT a
        // width-deferred `JmpJsrSym`. A low (<=0x7FFF) target selects abs.w, so
        // the fragment is already 4 bytes long and `resolve_layout`/`link` are a
        // pass-through. See the width-selection block in `lower_m68k`.
        let src = "    cpu 68000\n    phase 0\nLbl:\n    jmp Lbl\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        assert!(matches!(
            m.sections[0].fragments[0],
            sigil_ir::Fragment::Data(_)
        ));
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x4E, 0xF8, 0x00, 0x00]);
    }

    #[test]
    fn m68k_missing_size_suffix_is_a_clear_diagnostic() {
        // `move` has no default size and no suffix here — must error, not guess.
        let src = "    cpu 68000\n    move d0,d1\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let diags = run(src, &opts).expect_err("missing size suffix must be rejected");
        assert!(
            diags.iter().any(|d| d.message.contains("size suffix")),
            "expected a size-suffix diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn equate_as_16bit_operand_folds_not_fixups() {
        // BufSize is an EQUATE (not a label); it must FOLD, since the linker
        // cannot resolve a fixup to a non-label. Assemble + LINK + flatten.
        let src = "        cpu z80\n        phase 0\nBufSize = 1234h\n        ld hl,BufSize\n        dw BufSize\n";
        let m = run(src, &Options::default()).expect("assemble");
        let linked = sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new())
            .expect("link must succeed (no unresolvable fixup)");
        let bytes = sigil_link::flatten(&linked, 0x00);
        // ld hl,1234h = 21 34 12 ; dw 1234h = 34 12
        assert_eq!(bytes, vec![0x21, 0x34, 0x12, 0x34, 0x12]);
    }

    #[test]
    fn jr_dollar_relative_arithmetic_resolves_binary() {
        // Exercises resolve_dollar's Binary recursion: `$` in `jr $±2` must fold
        // to the instruction's own PC (0 under phase 0) before the fixup is made.
        // Linker: disp = target - inst_end_vma, inst_end_vma = 2 for a jr at PC 0.
        // `jr $+2` -> target 2, disp 0  -> 18 00
        // `jr $-2` -> target -2, disp -4 -> 18 FC
        let link = |src: &str| {
            let m = run(src, &Options::default()).expect("assemble");
            let linked =
                sigil_link::link(&m.sections, &sigil_ir::SymbolTable::new()).expect("link");
            sigil_link::flatten(&linked, 0x00)
        };
        assert_eq!(
            link("        cpu z80\n        phase 0\n        jr $+2\n"),
            vec![0x18, 0x00]
        );
        assert_eq!(
            link("        cpu z80\n        phase 0\n        jr $-2\n"),
            vec![0x18, 0xFC]
        );
    }

    #[test]
    fn ifdef_gates_emission_by_define_set() {
        let src = "        cpu z80\n        phase 0\n        db 1\n        ifdef __DEBUG__\n        db 0FFh\n        endif\n        ifdef SOUND_DRIVER_ENABLED\n        db 2\n        endif\n";
        let opts = Options {
            initial_cpu: Some(Cpu::Z80),
            defines: vec![("SOUND_DRIVER_ENABLED".into(), 1)],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x01, 0x02]);
    }

    #[test]
    fn if_elseif_else_takes_one_branch() {
        let src = "        cpu z80\n        phase 0\nX = 2\n        if X = 1\n        db 10h\n        elseif X = 2\n        db 20h\n        else\n        db 30h\n        endif\n";
        assert_eq!(image(src), vec![0x20]);
    }

    #[test]
    fn if_momcpuname_string_equality() {
        let src = "        cpu z80\n        phase 0\n        if MOMCPUNAME=\"Z80\"\n        db 0AAh\n        else\n        db 0BBh\n        endif\n";
        assert_eq!(image(src), vec![0xAA]);
    }

    #[test]
    fn if_literal_string_equality_and_inequality() {
        // Literal `"a"="a"` / `"a"<>"b"` must fold to a bool directly (never
        // through sigil_ir::Expr — strings are not an IR concept, §7.4).
        let src = "        cpu z80\n        phase 0\n        if \"a\"=\"a\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"=\"b\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"<>\"b\"\n        db 1\n        else\n        db 0\n        endif\n        if \"a\"<>\"a\"\n        db 1\n        else\n        db 0\n        endif\n";
        assert_eq!(image(src), vec![0x01, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn nested_if_inside_taken_branch() {
        let src = "        cpu z80\n        phase 0\nX = 1\n        if X = 1\n        db 1\n        if X = 1\n        db 2\n        endif\n        db 3\n        endif\n";
        assert_eq!(image(src), vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn m68k_operandless_instruction_reaches_lower_not_swallowed_as_label() {
        // `rts` is NOT a Z80 mnemonic (Z80 has `ret`) and carries no operand, so
        // it is a clean discriminator: if the indented head were misclassified as
        // a bare label, `body.len() == 1` would define it and return with NO
        // bytes emitted. Routed correctly it reaches lower_m68k and (T4) lowers
        // for real: `rts` = 4E75.
        let src = "    cpu 68000\n    rts\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
    }

    #[test]
    fn m68k_instruction_after_colon_label_lowers_the_mnemonic_not_the_operand() {
        // Before the column-rule fix, `move.w` was swallowed as a bogus label and
        // only `d0` reached dispatch. Routed correctly, the whole instruction
        // lowers: `move.w d0,d1` = 3200.
        let src = "    cpu 68000\nStart:\n    move.w d0,d1\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x32, 0x00]);
    }

    #[test]
    fn m68k_colon_label_then_instruction_both_handled() {
        // `Foo: rts` on one line: the colon label must be defined AND the
        // remaining head routed as an instruction (label_colon.is_some() clause),
        // even though line.text starts at column 0.
        let src = "    cpu 68000\nFoo: rts\n";
        let opts = Options {
            initial_cpu: Some(Cpu::M68000),
            defines: vec![],
            include_root: None,
            guarded_defines: vec![],
        };
        let m = run(src, &opts).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x4E, 0x75]);
    }

    #[test]
    fn lowers_common_instructions() {
        let src = "        cpu z80\n        phase 0\n        nop\n        ld a,0Ch\n        ld b,c\n        add a,b\n        jp 1234h\n";
        assert_eq!(
            image(src),
            vec![0x00, 0x3E, 0x0C, 0x41, 0x80, 0xC3, 0x34, 0x12]
        );
    }

    #[test]
    fn db_dw_le_and_equate() {
        let src = "        cpu z80\n        phase 0\nGAP = 4\n        db 1,2,3\n        dw 0284h\n        db GAP\n";
        assert_eq!(image(src), vec![0x01, 0x02, 0x03, 0x84, 0x02, 0x04]);
    }

    #[test]
    fn local_equate_resolves_in_scope() {
        let src = "        cpu z80\n        phase 0\nScope:\n.k      = 5\n        ld a,.k\n";
        assert_eq!(image(src), vec![0x3E, 0x05]);
    }

    #[test]
    fn rept_dollar_gap_fill() {
        // 3 nops (0x00), then fill to phased VMA 8 with `db 0` ⇒ 8 total bytes.
        let src = "        cpu z80\n        phase 0\n        nop\n        nop\n        nop\n        rept 8-$\n        db 0\n        endr\n";
        assert_eq!(image(src), vec![0x00; 8]);
    }

    #[test]
    fn rept_constant_count() {
        let src =
            "        cpu z80\n        phase 0\n        rept 3\n        db 0AAh\n        endr\n";
        assert_eq!(image(src), vec![0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn functions_fold_including_truncating_div() {
        let src = concat!(
            "        cpu z80\n        phase 0\n",
            "SFX_WIN_MASK = 32767\n",
            "SFX_WIN_BASE = 32768\n",
            // Name-first (real AS): `<name> function <formal>, <body>`.
            "sfx_winptr function addr, ((addr) & SFX_WIN_MASK) | SFX_WIN_BASE\n",
            "sfx_bankid function addr, (addr) >> 15\n",
            "timerAReload function hz, 1024 - (1000000000 / ((hz) * 18773))\n",
            "Sfx_33   = 0D69Ah\n",
            "        dw sfx_winptr(Sfx_33)\n",
            "        db sfx_bankid(0C0000h)\n",
            "        db timerAReload(59)\n",
        );
        // sfx_winptr(0xD69A)=(0xD69A&0x7FFF)|0x8000=0xD69A → LE 9A D6
        // sfx_bankid(0xC0000)=0xC0000>>15=0x18 ; timerAReload(59)=122=0x7A
        assert_eq!(image(src), vec![0x9A, 0xD6, 0x18, 0x7A]);
    }

    #[test]
    fn pbyte_macro_momcpuname_allargs_under_z80() {
        let src = concat!(
            "        cpu z80\n        phase 0\n",
            "        ifndef pbyte_defined\n",
            "pbyte_defined = 1\n",
            "pbyte   macro\n",
            "        if MOMCPUNAME=\"Z80\"\n",
            "        db      ALLARGS\n",
            "        else\n",
            "        dc.b    ALLARGS\n",
            "        endif\n",
            "        endm\n",
            "        endif\n",
            "        pbyte 1,2,3,255\n",
        );
        assert_eq!(image(src), vec![0x01, 0x02, 0x03, 0xFF]);
    }

    #[test]
    fn macro_positional_params() {
        let src = "        cpu z80\n        phase 0\nemit2   macro x,y\n        db x,y\n        endm\n        emit2 10h,20h\n";
        assert_eq!(image(src), vec![0x10, 0x20]);
    }

    #[test]
    fn macro_keyword_args_bind_by_name() {
        // asl-verified (see tst snippet in snippets_golden.txt): `NAME=value`
        // binds a param by name regardless of its position in the call.
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst AMP=7,PER=9\n",
        );
        assert_eq!(image(src), vec![0x07, 0x09]);
    }

    #[test]
    fn macro_positional_args_still_work_alongside_keyword_binding() {
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst 3,4\n",
        );
        assert_eq!(image(src), vec![0x03, 0x04]);
    }

    #[test]
    fn macro_keyword_args_are_order_independent() {
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "tst     macro AMP,PER\n",
            "        dc.b AMP\n        dc.b PER\n        endm\n",
            "        tst PER=5,AMP=2\n",
        );
        assert_eq!(image(src), vec![0x02, 0x05]);
    }

    #[test]
    fn struct_word_field_pads_running_offset_to_even_under_padding_on() {
        // asl-verified: with `padding on` (asl's default), a `ds.w`/`ds.l`
        // (width >= 2) field pads the running struct offset up to the next
        // even address AFTER it's placed — even though the field's own start
        // offset is not pre-aligned. Probed against real asl:
        // `a ds.b 1 / b ds.w 1 / c ds.b 1` -> a=0 b=1 c=4 len=5.
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "b       ds.w 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_b\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x01, 0x04, 0x05]);
    }

    #[test]
    fn struct_word_field_uses_naive_offset_under_padding_off() {
        // asl-verified: with `padding off` (Aeon's real global state, set at
        // the top of main.asm), struct fields are NOT even-rounded — the
        // running offset advances by exactly the field size. Probed against
        // real asl: `a ds.b 1 / b ds.w 1 / c ds.b 1` -> a=0 b=1 c=3 len=4.
        let src = concat!(
            "        cpu 68000\n        padding off\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "b       ds.w 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_b\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x01, 0x03, 0x04]);
    }

    #[test]
    fn function_name_first_simple_double() {
        // Self-contained: `dbl(x) = (x)*2`, name-first. db dbl(5) = 10 = 0x0A.
        let src = "        cpu z80\n        phase 0\ndbl function x, (x)*2\n        db dbl(5)\n";
        assert_eq!(image(src), vec![0x0A]);
    }

    #[test]
    fn struct_offsets_and_len_drive_indexed_disp() {
        // Packed: a(1) b(1) c(2) → a=0 b=1 c=2 len=4. Then (ix+SeqChannel_b) = (ix+1).
        // Name-first (real AS): `SeqChannel struct` … `SeqChannel endstruct`.
        let src = "        cpu z80\n        phase 0\nSeqChannel struct\na       ds.b 1\nb       ds.b 1\nc       ds.w 1\nSeqChannel endstruct\n        ld a,(ix+SeqChannel_b)\n        db SeqChannel_len\n";
        // ld a,(ix+1) = DD 7E 01 ; db 4 = 04
        assert_eq!(image(src), vec![0xDD, 0x7E, 0x01, 0x04]);
    }

    #[test]
    fn struct_three_byte_fields_len_and_offsets() {
        // Three ds.b 1 fields → offsets 0/1/2, DacSample_len = 3.
        let src = "        cpu z80\n        phase 0\nDacSample struct\np       ds.b 1\nq       ds.b 1\nr       ds.b 1\nDacSample endstruct\n        db DacSample_p, DacSample_q, DacSample_r, DacSample_len\n";
        assert_eq!(image(src), vec![0x00, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn equ_keyword_defines_a_constant() {
        // AS `name equ expr` (parallax_macros.inc `FACTOR_LOCKED equ $0FF`).
        // Also `dec equ $90`: `dec` is a Z80 mnemonic, so without the equate
        // intercept the line would route to instruction lowering.
        let src = "        cpu 68000\n        phase 0\nFOO equ $12\ndec equ $34\n        dc.b FOO\n        dc.b dec\n";
        assert_eq!(image(src), vec![0x12, 0x34]);
    }

    #[test]
    fn colon_label_equate_forms_define_constants_not_labels() {
        // AS tolerates a decorative colon on an equate: `NAME: equ v`
        // (debugger.asm) and `NAME: = v` (ram.asm `RESET_RAM: = $FFFFFF00`).
        let src = "        cpu 68000\n        phase 0\nA: equ $11\nB: = $22\n        dc.b A\n        dc.b B\n";
        assert_eq!(image(src), vec![0x11, 0x22]);
    }

    #[test]
    fn anonymous_struct_reserve_field_advances_offset() {
        // AS: an unnamed `ds.b N` inside a struct reserves space (advances the
        // running offset) but binds no member symbol — the Act struct's
        // `ds.b 1 ; reserved (pad to word)` pattern. Here b=$00, len=3 (a+pad+c
        // = 1+1+1) even though the middle field has no name.
        let src = concat!(
            "        cpu 68000\n        padding off\n        phase 0\n",
            "Rec     struct\n",
            "a       ds.b 1\n",
            "        ds.b 1\n",
            "c       ds.b 1\n",
            "Rec     endstruct\n",
            "        dc.b Rec_a\n        dc.b Rec_c\n        dc.b Rec_len\n",
        );
        assert_eq!(image(src), vec![0x00, 0x02, 0x03]);
    }

    #[test]
    fn char_constant_folds_in_expression() {
        // AS `'…'` packs big-endian; used bare and in expressions.
        let src = "        cpu 68000\n        phase 0\n        dc.l 'INIT'\n";
        assert_eq!(image(src), vec![0x49, 0x4E, 0x49, 0x54]);
    }

    #[test]
    fn binary_literal_folds_in_expression() {
        // AS `%` binary literal (constants.asm `VRAM = %100001`).
        let src = "        cpu 68000\n        phase 0\nVRAM = %100001\n        dc.b VRAM\n";
        assert_eq!(image(src), vec![0x21]);
    }

    #[test]
    fn backslash_line_continuation_joins_function_body() {
        // AS trailing-`\` continuation (macros.asm vdpComm def wraps its body).
        let src = concat!(
            "        cpu 68000\n        phase 0\n",
            "sum     function a,b, \\\n",
            "                (a) + (b)\n",
            "        dc.b sum(3,4)\n",
        );
        assert_eq!(image(src), vec![0x07]);
    }

    #[test]
    fn dc_w_emits_big_endian_words() {
        // asl: `dc.w $1234,$5678` -> 12 34 56 78 (BE, not Z80 `dw`'s LE).
        let src = "        cpu 68000\n        phase 0\n        dc.w $1234,$5678\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn dc_l_emits_big_endian_longs() {
        let src = "        cpu 68000\n        phase 0\n        dc.l $12345678\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn ds_b_trailing_reserve_contributes_no_image_bytes() {
        // Matches real asl/p2bin: a trailing `ds` with nothing written after it
        // never materializes into the flat image (verified against asl).
        let src = "        cpu 68000\n        phase 0\n        ds.b 3\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn ds_w_and_ds_l_reserve_scale_by_unit_width() {
        let src = "        cpu 68000\n        phase 0\n        ds.w 2\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections[0].vma_len(), 4);
        assert_eq!(module.sections[0].image_len(), 0);

        let src = "        cpu 68000\n        phase 0\n        ds.l 1\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections[0].vma_len(), 4);
        assert_eq!(module.sections[0].image_len(), 0);
    }

    #[test]
    fn align_pads_zero_bytes_to_next_boundary() {
        // Odd offset 1 -> align 2 pads one zero byte, then the next dc.b lands
        // at the aligned offset (verified against asl: fill byte is 0x00).
        let src =
            "        cpu 68000\n        phase 0\n        dc.b 1\n        align 2\n        dc.b 2\n";
        assert_eq!(image(src), vec![0x01, 0x00, 0x02]);
    }

    #[test]
    fn align_is_a_noop_when_already_aligned() {
        let src = "        cpu 68000\n        phase 0\n        dc.w $1234\n        align 2\n        dc.b 9\n";
        assert_eq!(image(src), vec![0x12, 0x34, 0x09]);
    }

    #[test]
    fn align_pads_to_large_power_of_two_boundary() {
        let src = "        cpu 68000\n        phase 0\n        dc.b 1,2,3\n        align $10\n        dc.b 4\n";
        let mut want = vec![0x01, 0x02, 0x03];
        want.extend(std::iter::repeat_n(0x00, 13));
        want.push(0x04);
        assert_eq!(image(src), want);
    }

    #[test]
    fn org_backpatch_seeks_in_section_and_overwrites() {
        // The `parallax_section_end` shape: capture positions via `:=`/`*`
        // (M1.C T6b adds `*` as a PC-symbol atom alongside `$`), seek back to
        // patch a placeholder byte, then resume forward. asl-verified: 63 01 02 03 04.
        let src = "        cpu 68000\n        padding off\n        phase 0\nHdr := *\n        dc.b 0,1,2,3\nEnd := *\n        org Hdr\n        dc.b 99\n        org End\n        dc.b 4\n";
        assert_eq!(image(src), vec![0x63, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn org_forward_past_extent_opens_a_new_phase_like_section() {
        // A forward `org` beyond anything written closes the section and
        // re-phases (main.asm's `org $10000` shape, scaled down) rather than
        // growing the still-open section with a zero-fill run — proven here by
        // checking `module.sections.len()` directly (the byte-level gap-fill is
        // ALSO covered by the `org_forward_new_section` golden snippet, which
        // can't distinguish the two implementations since `flatten` produces
        // identical bytes either way).
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1,2,3,4\n        org 16\n        dc.b 5,6\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(
            module.sections.len(),
            2,
            "forward org must open a new section, not seek in-place"
        );
        assert_eq!(module.sections[0].vma_base, Some(0));
        assert_eq!(module.sections[0].lma, 0);
        assert_eq!(module.sections[1].vma_base, Some(16));
        assert_eq!(module.sections[1].lma, 16);
        // Flatten both sections (image() only returns the first) to prove the
        // multi-section split still gap-fills identically to an in-section run.
        let linked =
            sigil_link::link(&module.sections, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        let mut want = vec![1, 2, 3, 4];
        want.extend(std::iter::repeat_n(0x00, 12));
        want.extend([5, 6]);
        assert_eq!(bytes, want);
    }

    #[test]
    fn org_with_no_section_open_yet_just_sets_the_phase_base() {
        // main.asm's very first `org 0` (before any byte is emitted): behaves
        // exactly like `phase`'s no-section-open path — no seek, no section
        // materializes until the next emit.
        let src = "        cpu 68000\n        padding off\n        org 0\n        dc.b 7\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(module.sections.len(), 1);
        assert_eq!(module.sections[0].vma_base, Some(0));
        assert_eq!(image(src), vec![0x07]);
    }

    /// Resolve a label's VMA (`vma_origin + offset`) from a finished module.
    fn label_vma(module: &Module, name: &str) -> u32 {
        for sec in &module.sections {
            let origin = sec.vma_origin();
            for l in &sec.labels {
                if l.name == name {
                    return origin + l.offset;
                }
            }
        }
        panic!("label `{name}` not found in module");
    }

    #[test]
    fn phase_dephase_keeps_a_continuous_physical_counter() {
        // The MovingTrucks LMA-continuity model, distilled (asl-probed, Bld 212):
        //   org 0 / 8 bytes / Base / save / cpu z80 / phase 08000h / L1 / 4 bytes
        //   / L1b / dephase / restore / L2 / 2 bytes / L3
        // asl symbol table:
        //   Base=8  L1=8000  L1b=8004  L2=C  L3=E
        // The phase block's 4 bytes advance the PHYSICAL location counter even
        // though labels INSIDE the block report window (0x8000+) VMAs. After
        // dephase/restore the counter CONTINUES from physical (8+4=0xC), it is
        // NOT rewound to a section-local 0 nor to the pre-save base.
        let src = "\
        cpu 68000\n        padding off\n        org 0\n\
        dc.b 1,2,3,4,5,6,7,8\n\
Base:\n\
        save\n        cpu z80\n        phase 08000h\n\
L1:\n\
        db 10h,11h,12h,13h\n\
L1b:\n\
        dephase\n        restore\n\
L2:\n\
        dc.b $AA,$BB\n\
L3:\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(label_vma(&module, "Base"), 0x8, "physical after 8 bytes");
        assert_eq!(label_vma(&module, "L1"), 0x8000, "window VMA inside phase");
        assert_eq!(label_vma(&module, "L1b"), 0x8004, "window VMA + 4");
        assert_eq!(
            label_vma(&module, "L2"),
            0xC,
            "physical CONTINUES past the phase block (8+4), not rewound"
        );
        assert_eq!(label_vma(&module, "L3"), 0xE, "physical + 2 more");
    }

    #[test]
    fn colon_labeled_set_reassigns_rather_than_defining_a_pc_label() {
        // asl-probed (Bld 212): a colon-label immediately followed by `set` is a
        // REASSIGNABLE-symbol assignment (colon decorative), NOT a PC label.
        //   i: set 0 / dc.b i / i: set i+5 / dc.b i / i: set i+5 / dc.b i
        // asl bytes: 00 05 0A. Treating `i:` as a PC label instead froze `i` at
        // the current address — the exact defect that made the debugger's
        // `__FSTRING_*` `.__pos: set strstr(...)` loop never terminate.
        let src = "        cpu 68000\n        padding off\n        org 0\ni:  set 0\n        dc.b i\ni:  set i+5\n        dc.b i\ni:  set i+5\n        dc.b i\n";
        assert_eq!(image(src), vec![0x00, 0x05, 0x0A]);
    }

    #[test]
    fn save_restore_does_not_resurrect_a_dephased_phase() {
        // asl-probed (Bld 212): a `save` taken WHILE phased, then `dephase`, then
        // `restore` does NOT bring the phase displacement back — `restore` only
        // restores cpu/padding/listing. Sequence:
        //   org 0 / 4 bytes / phase $8000 / A / 2 bytes / save / dephase / B
        //   / 2 bytes / restore / C
        // asl: A=8000  B=6  C=8  (C is physical 8, NOT 0x8004).
        let src = "\
        cpu 68000\n        padding off\n        org 0\n\
        dc.b 1,2,3,4\n\
        phase $8000\n\
A:\n\
        dc.b 5,6\n\
        save\n        dephase\n\
B:\n\
        dc.b 7,8\n\
        restore\n\
C:\n";
        let module = run(src, &Options::default()).expect("assemble");
        assert_eq!(label_vma(&module, "A"), 0x8000);
        assert_eq!(label_vma(&module, "B"), 0x6, "physical after dephase");
        assert_eq!(
            label_vma(&module, "C"),
            0x8,
            "restore must NOT resurrect the dephased displacement"
        );
    }

    #[test]
    fn message_interpolates_and_emits_no_bytes() {
        // false `if` guards fatal; message with \{expr} just evaluates; db N emits.
        let src = "        cpu z80\n        phase 0\nN = 5\n        if N <> 5\n        fatal \"bad size \\{N}\"\n        endif\n        message \"N is \\{N}\"\n        db N\n";
        assert_eq!(image(src), vec![0x05]);
    }

    #[test]
    fn fatal_on_true_condition_is_an_error() {
        let src = "        cpu z80\n        phase 0\nN = 6\n        if N <> 5\n        fatal \"bad size \\{N}\"\n        endif\n";
        assert!(run(src, &Options::default()).is_err());
    }

    #[test]
    fn forward_equate_resolves_across_passes() {
        // LATER is used by `db` BEFORE it is defined; the fixpoint resolves it.
        let src = "        cpu z80\n        phase 0\n        db LATER\nLATER   = 7\n";
        assert_eq!(image(src), vec![0x07]);
    }

    #[test]
    fn two_level_forward_chain_resolves() {
        // db A ; A = B ; B = 7  — needs 3 passes to settle.
        let src = "        cpu z80\n        phase 0\n        db A\nA       = B\nB       = 7\n";
        assert_eq!(image(src), vec![0x07]);
    }

    #[test]
    fn include_pulls_in_a_file() {
        let dir = std::env::temp_dir().join(format!("sigil_inc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("part.inc"), "        db 0AAh,0BBh\n").unwrap();
        let main = dir.join("main.asm");
        std::fs::write(&main, "        cpu z80\n        phase 0\n        db 1\n        include \"part.inc\"\n        db 2\n").unwrap();
        let m = crate::assemble_root(&main, &Options::default()).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        assert_eq!(bytes, vec![0x01, 0xAA, 0xBB, 0x02]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn binclude_emits_file_bytes_verbatim() {
        // `BINCLUDE "path"` (M1.C T10): opaque binary emit, no parsing — the
        // file's raw bytes go straight into the image. Path resolves via
        // `include_root` exactly like `include` (asl-verified: same base
        // directory, real Aeon source uses `BINCLUDE "games/.../foo.bin"`
        // resolved from the aeon root). Content spans the full byte range
        // (incl. 0x00 and non-ASCII) to prove this is a raw copy, not a
        // text/db-style parse.
        let dir = std::env::temp_dir().join(format!("sigil_binc_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let payload: Vec<u8> = vec![0x00, 0x41, 0xFF, 0x0A, 0x80, 0x7F];
        std::fs::write(dir.join("blob.bin"), &payload).unwrap();
        let main = dir.join("main.asm");
        std::fs::write(
            &main,
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1\n        BINCLUDE \"blob.bin\"\n        dc.b 2\n",
        )
        .unwrap();
        let m = crate::assemble_root(&main, &Options::default()).expect("assemble");
        let bytes = m
            .sections
            .first()
            .map(|s| s.image_bytes())
            .unwrap_or_default();
        let mut want = vec![0x01];
        want.extend_from_slice(&payload);
        want.push(0x02);
        assert_eq!(bytes, want);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_reassigns_a_symbol() {
        // Plain reassignment (no self-reference): the second `set` simply
        // overwrites `i`.
        let src = "        cpu 68000\n        padding off\n        phase 0\ni       set 1\n        dc.b i\ni       set 9\n        dc.b i\n";
        assert_eq!(image(src), vec![0x01, 0x09]);
    }

    #[test]
    fn set_self_reference_reads_the_current_value() {
        // `i set i+5` (T8): the RHS folds against `i`'s CURRENT value at this
        // point in emission order, then overwrites it — verified against
        // real asl (see `set_accumulator` in `tests/snippets_golden.txt`).
        let src = "        cpu 68000\n        padding off\n        phase 0\ni       set 0\n        dc.b i\ni       set i+5\n        dc.b i\n";
        assert_eq!(image(src), vec![0x00, 0x05]);
    }

    #[test]
    fn coloneq_is_identical_to_set() {
        // `:=` (T8) is asl-verified to behave exactly like `set` — see
        // `coloneq_accumulator` in `tests/snippets_golden.txt`. `:=` must
        // lex as ONE `ColonEq` token so `j := 10` is never mistaken for a
        // colon-label (`j:`) followed by a stray `= 10`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nj       := 10\n        dc.b j\nj       := j*2\n        dc.b j\n";
        assert_eq!(image(src), vec![0x0A, 0x14]);
    }

    #[test]
    fn set_accumulates_inside_rept() {
        // The deform-accumulator pattern (`rept` body counter): `set`
        // converges across the multi-pass fixpoint because every value folds
        // immediately from the CURRENT pass's own execution (no dependency
        // on the seeded env from the prior pass) — see `set_in_rept` in
        // `tests/snippets_golden.txt`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nk       set 0\n        rept 4\n        dc.b k\nk       set k+1\n        endr\n";
        assert_eq!(image(src), vec![0x00, 0x01, 0x02, 0x03]);
    }

    // ── T9.1: debug string builtins (substr/strlen/strstr/val) + `!`=OR ────

    #[test]
    fn strlen_of_a_plain_string_literal() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(\"hello\")\n";
        assert_eq!(image(src), vec![5]);
    }

    #[test]
    fn strlen_resolves_a_string_equ_symbol() {
        // A string bound with `equ` (not just `set`) must be readable by the
        // string builtins — Aeon's engine/system/header.inc does
        // `GAME_CONSOLE equ "SEGA GENESIS    "` then `strlen(GAME_CONSOLE)`.
        // Regression: `equ` used to drop the string value (neither env nor
        // str_env written), so this reported "could not evaluate string builtin".
        let src = "        cpu 68000\n        padding off\n        phase 0\nS       equ \"SEGA GENESIS    \"\n        dc.b strlen(S)\n";
        assert_eq!(image(src), vec![16]);
    }

    #[test]
    fn align_inside_a_phase_at_a_rom_address_is_a_plain_roundup() {
        // A `phase` does NOT change the align rule — the SIGN of the phased PC
        // does, and $B005 is positive. asl 1.42 Bld 212, corpus flags
        // (`docs/superpowers/probes/2026-09-03-align/p1.asm`):
        //
        //        4/    B000 :          ds.b  5
        //        5/    B005 :          align 256
        //        6/    B100 : B100     L: dc.w L
        //
        // $B100, not $B200. The `ds.b` and the align pad open a gap, and the
        // trailing `dc.w L` fills it: `p2bin p1.p p1.bin` is 258 bytes, zero
        // everywhere but `B1 00` at offset $100 — L's low word at L's own
        // offset from the phase base.
        let src = "        cpu 68000\n        padding off\n        phase $B000\n        ds.b 5\n        align 256\nL:      dc.w L\n        dephase\n";
        assert_p2bin_image(src, 258, &[0xB1, 0x00]);
    }

    #[test]
    fn align_at_a_ram_address_overshoots_a_whole_block() {
        // The RAM side of the same rule: asl aligns on the low 32 bits read as
        // an i32, with C's truncating remainder, so a $FFFF…. PC rounds toward
        // zero and lands a block high. pos = $FFFFB02A, n=256 → $FFFFB200, where
        // a plain round-up would say $FFFFB100. asl 1.42 Bld 212, corpus flags
        // (`probes/2026-09-03-align/p9.asm`):
        //
        //        4/FFFFFFFFFFFF0000 :          ds.b  $B02A
        //        5/FFFFFFFFFFFFB02A :          align 256
        //        6/FFFFFFFFFFFFB200 : B200     L: dc.w L&$FFFF
        //
        // This is the block that puts Aeon's `Player_Pos_Ring` above the naive
        // boundary — the real content of the 2026-07-08 probe, whose four rows
        // were all RAM addresses recorded by their low half.
        //
        // `p2bin p9.p p9.bin` is 45,570 bytes ($B202), zero but for `B2 00` at
        // offset $B200 — the reservation's gap, filled by the `dc.w` that
        // follows it, at L's own offset from the phase base.
        let src = "        cpu 68000\n        padding off\n        phase $FFFF0000\n        ds.b $B02A\n        align 256\nL:      dc.w L&$FFFF\n        dephase\n";
        assert_p2bin_image(src, 0xB202, &[0xB2, 0x00]);
    }

    #[test]
    fn align_two_moves_an_even_ram_address_but_not_an_even_rom_one() {
        // The same asymmetry at n=2, where it costs 2 bytes at every even RAM
        // address. asl 1.42 Bld 212, corpus flags (`probes/2026-09-03-align/`):
        //
        //   p11   4/FFFFFFFFFFFF0000 :   ds.b  $B02A
        //         5/FFFFFFFFFFFFB02A :   align 2
        //         6/FFFFFFFFFFFFB02C : B02C   M: dc.w M&$FFFF
        //
        //   p12   4/    B000 :            ds.b  $2A
        //         5/    B02A :            align 2
        //         6/    B02A : B02A       M: dc.w M
        //
        // `p2bin` on the two: 45,102 bytes ending `B0 2C` at $B02C, and 44
        // bytes ending `B0 2A` at $2A. The two extra bytes the RAM side costs
        // are visible in the image size, not only in the label.
        let ram = "        cpu 68000\n        padding off\n        phase $FFFF0000\n        ds.b $B02A\n        align 2\nM:      dc.w M&$FFFF\n        dephase\n";
        assert_p2bin_image(ram, 0xB02E, &[0xB0, 0x2C]);
        let rom = "        cpu 68000\n        padding off\n        phase $B000\n        ds.b $2A\n        align 2\nM:      dc.w M\n        dephase\n";
        assert_p2bin_image(rom, 0x2C, &[0xB0, 0x2A]);
    }

    #[test]
    fn align_outside_a_phase_is_a_standard_roundup() {
        // A positive PC with no phase: the ordinary round-up. `dc.b`×5 makes
        // the pre-align bytes image (offset 5, here() 5); align 256 → $100;
        // then `dc.w L` emits L's value $0100 at image offset $100.
        // asl+p2bin on this exact source give 258 bytes ending `01 00`
        // (`probes/2026-09-03-align`, the `f1` shape) — asl leaves the pad as a
        // HOLE in the `.p` and p2bin zero-fills it, where sigil emits the 251
        // zeros directly; the image is the same either way.
        // The phased form of the same position answers identically: a `phase`
        // does not change the rule, only the sign of the PC does.
        let src = "        cpu 68000\n        padding off\n        org 0\n        dc.b 1,2,3,4,5\n        align 256\nL:      dc.w L\n";
        let img = image(src);
        assert_eq!(img.len(), 0x102);
        assert_eq!(&img[..5], &[1, 2, 3, 4, 5]);
        assert_eq!(&img[0x100..], &[0x01, 0x00]);
    }

    #[test]
    fn substr_len_zero_means_to_the_end() {
        // asl-verified: `substr("hello",1,0)` = "ello" (len=0 = "to the end").
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(\"hello\",1,0))\n";
        assert_eq!(image(src), vec![4]);
    }

    #[test]
    fn substr_bounded_length() {
        // asl-verified: `substr("hello",1,2)` = "el".
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(\"hello\",1,2))\n";
        assert_eq!(image(src), vec![2]);
    }

    #[test]
    fn strstr_finds_the_last_character() {
        // D5 correction: asl 1.42 Bld 212's `strstr` is STANDARD — it does
        // NOT fail to find a match at the last character (`strstr("b>",">")`
        // = 1, not the alleged buggy "not found").
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"b>\",\">\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn strstr_present_mid_string() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"xab\",\"ab\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn strstr_absent_is_minus_one() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(\"abc\",\"z\")&$FF\n";
        assert_eq!(image(src), vec![0xFF]);
    }

    #[test]
    fn strstr_nests_over_a_substr_argument() {
        // `strstr(substr(s,p,0),">")` — the debugger's real usage shape.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strstr(substr(\"xxb>\",2,0),\">\")&$FF\n";
        assert_eq!(image(src), vec![1]);
    }

    #[test]
    fn val_parses_a_dollar_hex_string() {
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b val(\"$80\")\n";
        assert_eq!(image(src), vec![0x80]);
    }

    #[test]
    fn val_parses_a_decimal_string() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b val(\"144\")&$FF\n";
        assert_eq!(image(src), vec![144]);
    }

    #[test]
    fn val_evaluates_a_symbol_plus_arithmetic_in_the_string() {
        // `val` is an AS-EXPRESSION evaluator, not a plain number parse: the
        // string's symbol reference resolves against the CURRENT env.
        let src = "        cpu 68000\n        padding off\n        phase 0\nhex     = $80\n        dc.b val(\"hex+1\")&$FF\n";
        assert_eq!(image(src), vec![0x81]);
    }

    #[test]
    fn bang_is_infix_bitwise_or() {
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b (3!4)&$FF\n";
        assert_eq!(image(src), vec![7]);
    }

    // ── T9.2: `.ATTRIBUTE` macro-suffix + `!name` escape + `while … endm` ──

    // ── `{INTLABEL}` / `__LABEL__` / the `label` directive ──────────────────
    //
    // Ground truth for every expected value below is an `asl -L` listing (AS
    // V1.42 Beta Bld 212), invoked with the Sonic 2 build's own flags minus the
    // two that only redirect output: `asl -xx -n -q -A -L -U -i .`. `-U` forces
    // case-sensitivity and every row carries it.

    /// The head of a `{INTLABEL}` source, shared by the probes below so a test
    /// body reads as the macro under test rather than as boilerplate.
    fn intlabel_src(body: &str) -> String {
        format!("\tcpu 68000\n\tpadding off\n\torg $1000\n{body}")
    }

    #[test]
    fn intlabel_capture_leaves_the_label_to_the_macro_to_place() {
        // The capture SUPPRESSES the ordinary label definition. `sup` declares
        // the group and drops the capture, so `LabA` is defined nowhere and is
        // absent from asl's symbol table; `nosup` is identical but for the
        // group, and `LabB` lists as `1002 C`:
        //
        // ```text
        //   10/ 1000 : (MACRO)              LabA:	sup
        //   10/ 1000 : 4E71                        nop
        //   11/ 1002 : (MACRO)              LabB:	nosup
        //   11/ 1002 : 4E71                        nop
        //   12/ 1004 : 1002                	dc.w LabB
        // ```
        let defs = "sup macro {INTLABEL}\n\tnop\n\tendm\nnosup macro\n\tnop\n\tendm\n";
        let src = intlabel_src(&format!("{defs}LabA:\tsup\nLabB:\tnosup\n\tdc.w LabB\n"));
        assert_eq!(image(&src), vec![0x4E, 0x71, 0x4E, 0x71, 0x10, 0x02]);
        // The dropped capture is not merely unplaced — the name does not exist.
        let bad = intlabel_src(&format!("{defs}LabA:\tsup\nLabB:\tnosup\n\tdc.w LabA\n"));
        assert!(
            !links(&bad),
            "a capture the body never places must leave no symbol behind"
        );
        // The control: the same reference to the macro that does NOT declare the
        // group links, so the refusal above is the capture and not the shape.
        let ok = intlabel_src(&format!("{defs}LabA:\tsup\nLabB:\tnosup\n\tdc.w LabB\n"));
        assert!(links(&ok));
    }

    #[test]
    fn intlabel_consumes_no_argument_position_wherever_it_is_written() {
        // `{INTLABEL}` declares a capture, not a slot. Three macros differing
        // only in where the group sits bind `pp`/`qq` identically from `11,22`
        // (`0B16` three times):
        //
        // ```text
        //   13/ 1000 : (MACRO)              L1:	m 11,22
        //   13/ 1000 : 0B16                        dc.b 11,22
        //   14/ 1002 : (MACRO)              L2:	n 11,22
        //   14/ 1002 : 0B16                        dc.b 11,22
        //   15/ 1004 : (MACRO)              L3:	o 11,22
        //   15/ 1004 : 0B16                        dc.b 11,22
        // ```
        let defs = "m macro {INTLABEL},pp,qq\n\tdc.b pp,qq\n\tendm\n\
                    n macro pp,{INTLABEL},qq\n\tdc.b pp,qq\n\tendm\n\
                    o macro pp,qq,{INTLABEL}\n\tdc.b pp,qq\n\tendm\n";
        let src = intlabel_src(&format!("{defs}L1:\tm 11,22\nL2:\tn 11,22\nL3:\to 11,22\n"));
        assert_eq!(image(&src), vec![11, 22, 11, 22, 11, 22]);
    }

    #[test]
    fn label_directive_binds_any_expression_as_an_address() {
        // `label` takes an expression, not just the PC, and tolerates the
        // decorative colon exactly as `equ` does:
        //
        // ```text
        //    4/ 1000 : =$1000               A	label *
        //    5/ 1000 : =$2000               B	label $2000
        //    6/ 1000 : =$1004               C:	label *+4
        //    7/ 1000 : 4E71                	nop
        //    8/ 1002 : =$1002               D	label *
        //    9/ 1002 : 1000 2000 1004      	dc.w A,B,C,D
        // ```
        let src = intlabel_src(
            "A\tlabel *\nB\tlabel $2000\nC:\tlabel *+4\n\tnop\nD\tlabel *\n\tdc.w A,B,C,D\n",
        );
        assert_eq!(
            image(&src),
            vec![0x4E, 0x71, 0x10, 0x00, 0x20, 0x00, 0x10, 0x04, 0x10, 0x02]
        );
    }

    #[test]
    fn label_star_inside_an_expansion_defines_the_callers_symbol() {
        // The corpus's `offsetTable`, whole. `__LABEL__` substitutes to the
        // caller's `Table`, the `:=` binds the caller's `current_offset_table`,
        // and `label *` places `Table` where the body chose:
        //
        // ```text
        //    8/ 1000 : (MACRO)              Table:	offsetTable
        //    8/ 1000 : =$1000               current_offset_table := Table
        //    8/ 1000 : =$1000               Table label *
        //    9/ 1000 : 0002                	dc.w Target-Table
        //   11/ 1002 : 4E71                	nop
        //   12/ 1004 : 1000                	dc.w current_offset_table
        // ```
        let src = intlabel_src(
            "offsetTable macro {INTLABEL}\ncurrent_offset_table := __LABEL__\n__LABEL__ label *\n\tendm\n\
             Table:\toffsetTable\n\tdc.w Target-Table\nTarget:\n\tnop\n\tdc.w current_offset_table\n",
        );
        assert_eq!(image(&src), vec![0x00, 0x02, 0x4E, 0x71, 0x10, 0x00]);
    }

    #[test]
    fn label_composes_the_name_the_capture_is_pasted_into() {
        // The capture pastes into a surrounding name, in every position the
        // corpus writes: as a suffix inside a string, as an interior segment of
        // a colon label, and as a prefix of one:
        //
        // ```text
        //   11/ 1000 : =$1000               Tbl label *
        //   11/ 1000 : 7A6F 6E65 616E              dc.b "zoneanimcount_Tbl"
        //   11/ 1011 : =$1011               Prefix_Tbl: label *
        //   11/ 1011 : 4E71                        nop
        //   11/ 1013 : =$1013               Tbl_End label *
        //   12/ 1013 : 1000 1011 1013      	dc.w Tbl,Prefix_Tbl,Tbl_End
        // ```
        let src = intlabel_src(
            "comp macro {INTLABEL}\n__LABEL__ label *\n\tdc.b \"zoneanimcount___LABEL__\"\n\
             Prefix___LABEL__: label *\n\tnop\n__LABEL___End label *\n\tendm\n\
             Tbl:\tcomp\n\tdc.w Tbl,Prefix_Tbl,Tbl_End\n",
        );
        let mut want = b"zoneanimcount_Tbl".to_vec();
        want.extend_from_slice(&[0x4E, 0x71, 0x10, 0x00, 0x10, 0x11, 0x10, 0x13]);
        assert_eq!(image(&src), want);
    }

    #[test]
    fn an_absent_invocation_label_captures_the_empty_text() {
        // A `{INTLABEL}` macro invoked with no label binds the EMPTY text, which
        // is what makes the corpus's `if "__LABEL__"<>""` guard a guard. The
        // bare `label *` on the untaken side is neither an error nor a
        // definition — asl lists it and moves on:
        //
        // ```text
        //   10/ 1000 : 5B46 6F6F 5D                dc.b "[Foo]"
        //   10/ 1005 : =>TRUE                       if "Foo"<>""
        //   10/ 1005 : =$1005               Foo label *
        //   11/ 1005 : 5B5D                        dc.b "[]"
        //   11/ 1007 : =>FALSE                      if ""<>""
        //   11/ 1007 :                      label *
        //   12/ 1007 : 1005                	dc.w Foo
        // ```
        let src = intlabel_src(
            "m macro {INTLABEL}\n\tdc.b \"[__LABEL__]\"\n\tif \"__LABEL__\"<>\"\"\n\
             __LABEL__ label *\n\tendif\n\tendm\n\
             Foo:\tm\n\tm\n\tdc.w Foo\n",
        );
        let mut want = b"[Foo][]".to_vec();
        want.extend_from_slice(&[0x10, 0x05]);
        assert_eq!(image(&src), want);
    }

    /// The nine-position boundary matrix, run once per candidate kind. An
    /// ALPHANUMERIC abutting an edge that could continue an identifier blocks
    /// the substitution; `_` is an identifier character but not alphanumeric, so
    /// it never blocks. `ALLARGS` answers the same way as a parameter.
    ///
    /// ```text
    ///   10/ 1000 : dc.b "1[_Zz] 2[1pp] 3[Xpp] 4[.Zz] 5[ppX] 6[pp1] 7[Zz_] 8[(Zz)] 9[__Zz__] A[Foo_Zz_Bar] B[xALLARGSx] C[_Zz_]"
    /// ```
    #[test]
    fn an_alphanumeric_blocks_a_substitution_and_an_underscore_does_not() {
        let src = intlabel_src(
            "pm macro pp\n\tdc.b \"1[_pp] 2[1pp] 3[Xpp] 4[.pp] 5[ppX] 6[pp1] 7[pp_] 8[(pp)] \
             9[__pp__] A[Foo_pp_Bar] B[xALLARGSx] C[_ALLARGS_]\"\n\tendm\n\tpm Zz\n",
        );
        assert_eq!(
            String::from_utf8(image(&src)).unwrap(),
            "1[_Zz] 2[1pp] 3[Xpp] 4[.Zz] 5[ppX] 6[pp1] 7[Zz_] 8[(Zz)] 9[__Zz__] \
             A[Foo_Zz_Bar] B[xALLARGSx] C[_Zz_]"
        );
    }

    /// The same matrix for the capture, which answers identically — position by
    /// position — even though its own name begins and ends with `_`.
    ///
    /// ```text
    ///   11/ 1065 : dc.b "1[_Qq] 2[1__LABEL__] 3[X__LABEL__] 4[.Qq] 5[__LABEL__X] 6[__LABEL__1] 7[Qq_] 8[(Qq)] A[Foo_Qq_Bar]"
    /// ```
    #[test]
    fn the_captured_label_obeys_the_same_boundary_rule_as_a_parameter() {
        let src = intlabel_src(
            "lm macro {INTLABEL}\n\tdc.b \"1[___LABEL__] 2[1__LABEL__] 3[X__LABEL__] 4[.__LABEL__] \
             5[__LABEL__X] 6[__LABEL__1] 7[__LABEL___] 8[(__LABEL__)] A[Foo___LABEL___Bar]\"\n\tendm\n\
             Qq:\tlm\n",
        );
        assert_eq!(
            String::from_utf8(image(&src)).unwrap(),
            "1[_Qq] 2[1__LABEL__] 3[X__LABEL__] 4[.Qq] 5[__LABEL__X] 6[__LABEL__1] 7[Qq_] \
             8[(Qq)] A[Foo_Qq_Bar]"
        );
    }

    #[test]
    fn a_label_opened_scope_outlives_the_expansion_that_opened_it() {
        // `outer` passes its capture on to `inner`, which places it. The scope
        // that `label` opens is the CALLER's, it survives two expansion returns,
        // and `.cnt := 7` written in `outer`'s body after the nested call lands
        // in it — read back at top level as `Tbl.cnt`. A colon-less invocation
        // label is captured, and so is a dotted one, with its dot:
        //
        // ```text
        //   16/ 1000 : (MACRO)              Tbl	outer 3
        //   16/ 1000 :  (MACRO-2)           Tbl inner 3
        //   16/ 1000 : =$1000               Tbl label *
        //   16/ 1000 : 03                          dc.b 3
        //   16/ 1001 : =$7                  .cnt := 7
        //   17/ 1001 : 1000                	dc.w Tbl
        //   18/ 1003 : 07                  	dc.b Tbl.cnt
        //   19/ 1004 : (MACRO)              .loc	dot
        //   19/ 1004 : 4E71                        nop
        //   19/ 1006 : =$1006               .loc label *
        //   20/ 1006 : 1006                	dc.w .loc
        // ```
        let src = intlabel_src(
            "inner macro aa,{INTLABEL}\n__LABEL__ label *\n\tdc.b aa\n\tendm\n\
             outer macro aa,{INTLABEL}\n__LABEL__ inner aa\n.cnt := 7\n\tendm\n\
             dot macro {INTLABEL}\n\tnop\n__LABEL__ label *\n\tendm\n\
             Tbl\touter 3\n\tdc.w Tbl\n\tdc.b Tbl.cnt\n.loc\tdot\n\tdc.w .loc\n",
        );
        assert_eq!(
            image(&src),
            vec![0x03, 0x10, 0x00, 0x07, 0x4E, 0x71, 0x10, 0x06]
        );
    }

    #[test]
    fn the_capture_declaration_and_its_reference_both_fold_case() {
        // `{intlabel}` declares the capture and `__label__` reads it, under `-U`
        // — they are AS keywords, and a keyword folds where a parameter name
        // does not. A macro that does NOT declare the group leaves `__LABEL__`
        // as the nine ordinary characters it is written with:
        //
        // ```text
        //   10/ 1000 : 3C41 613E 3C41              dc.b "<Aa><Aa>"
        //   11/ 1008 : 3C5F 5F4C 4142              dc.b "<__LABEL__>"
        // ```
        let src = intlabel_src(
            "lo macro {intlabel}\n\tdc.b \"<__LABEL__><__label__>\"\n\tendm\n\
             nd macro pp\n\tdc.b \"<__LABEL__>\"\n\tendm\n\
             Aa:\tlo\n\tnd 1\n",
        );
        assert_eq!(
            String::from_utf8(image(&src)).unwrap(),
            "<Aa><Aa><__LABEL__>"
        );
    }

    #[test]
    fn a_label_placed_capture_is_a_relocatable_symbol_the_linker_can_reach() {
        // `label` at the PC produces a PLACED label, not a constant that happens
        // to equal the PC. The difference is invisible to a backward `dc.w`,
        // which the front end folds in-pass — it shows in a fixup the front end
        // DEFERS and the linker resolves from the section's symbol table. A
        // `bra.w` to a capture the macro places is exactly that shape:
        //
        // ```text
        //    8/ 1000 : 6000 0002           	bra.w Dest
        //    9/ 1004 : (MACRO)              Dest	mk
        //    9/ 1004 : =$1004               Dest label *
        //    9/ 1004 : 4E71                        nop
        //   10/ 1006 : 1004                	dc.w Dest
        // ```
        let src = intlabel_src(
            "mk macro {INTLABEL}\n__LABEL__ label *\n\tnop\n\tendm\n\tbra.w Dest\nDest\tmk\n\tdc.w Dest\n",
        );
        assert_eq!(
            linked_image(&src)[0x1000..],
            [0x60, 0x00, 0x00, 0x02, 0x4E, 0x71, 0x10, 0x04]
        );
        // The bytes above do not by themselves prove the symbol is PLACED: the
        // front end folds `Dest` out of its own env on the converged pass, so a
        // `label` that bound only a constant would produce them too. What
        // separates the two is whether the SECTION carries the label, which is
        // what the linker's own symbol table is built from and what the
        // relaxation deferral reads. Assert it against the plain-label twin, so
        // the expected offset is derived from the equivalent program rather than
        // copied from the row above.
        let twin = intlabel_src("\tbra.w Dest\nDest:\n\tnop\n\tdc.w Dest\n");
        assert_eq!(section_label(&src, "Dest"), section_label(&twin, "Dest"));
        assert_eq!(section_label(&src, "Dest"), Some(4));
    }

    #[test]
    fn a_capture_the_recursion_cap_refused_is_not_handed_to_the_next_call() {
        // The capture is parked between `exec_one` recognising it and
        // `expand_macro_inner` binding it, so EVERY path out of that function has
        // to consume it. The runaway is the path that does not reach the
        // binding: the deepest `D deep` parks a capture the refused expansion
        // never takes, and an UNLABELLED `q` afterwards would then read `D` — a
        // wrong symbol, not a missing one. asl runs the guard FALSE, so the
        // `error` inside it never fires:
        //
        // ```text
        //   12/ 1000 : (MACRO)              D       deep
        //   13/ 1000 : (MACRO)              	q
        //   13/ 1000 : =>FALSE                      if ""<>""
        //   13/ 1000 :                                 error "captured <>"
        // ```
        //
        // The runaway itself is diagnosed here (sigil caps the nest where asl
        // silently stops), so the assertion is on the diagnostic SET: the cap,
        // and nothing about a capture.
        let src = intlabel_src(
            "deep macro {INTLABEL}\nD\tdeep\n\tendm\n\
             q macro {INTLABEL}\n\tif \"__LABEL__\"<>\"\"\n\terror \"captured <__LABEL__>\"\n\tendif\n\tendm\n\
             \tdeep\n\tq\n",
        );
        let diags = run(&src, &Options::default()).expect_err("the runaway is diagnosed");
        let messages: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("expansion too deep")),
            "expected the runaway to be diagnosed, got {messages:?}"
        );
        assert!(
            !messages.iter().any(|m| m.contains("captured")),
            "the refused expansion's capture reached the next call: {messages:?}"
        );
    }

    #[test]
    fn split_attribute_suffix_strips_known_suffixes_only() {
        use super::split_attribute_suffix;
        assert_eq!(split_attribute_suffix("foo.w"), Some(("foo", ".w")));
        assert_eq!(split_attribute_suffix("foo.b"), Some(("foo", ".b")));
        assert_eq!(split_attribute_suffix("foo.l"), Some(("foo", ".l")));
        assert_eq!(split_attribute_suffix("foo.s"), Some(("foo", ".s")));
        assert_eq!(split_attribute_suffix("foo"), None);
        assert_eq!(split_attribute_suffix("move"), None);
    }

    #[test]
    fn attribute_macro_binds_dot_attribute_in_a_mnemonic() {
        // asl-verified golden (`attribute_macro` in snippets_golden.txt):
        // `foo.w d1` → `move.w d1,d0` = `30 01`; `foo.l d2` → `move.l d2,d0` = `20 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro src\n        move.ATTRIBUTE src,d0\n        endm\n        foo.w d1\n        foo.l d2\n";
        assert_eq!(image(src), vec![0x30, 0x01, 0x20, 0x02]);
    }

    #[test]
    fn attribute_substitutes_inside_a_string_literal_but_not_before_a_letter() {
        // `.ATTRIBUTE` reaches inside a quoted string, and it is NOT unbounded:
        // an alphanumeric immediately after it blocks the match, exactly as it
        // does for `ALLARGS` and for a parameter. Only the LEADING check is
        // skipped, and only because `.` cannot continue an identifier — which is
        // what keeps the glued-mnemonic use (`move.ATTRIBUTE`) working.
        //
        // asl `-xx -n -q -A -L -U -i .`, one `foo.w` expansion:
        //
        // ```text
        //    8/ 1000 : 0C          dc.b strlen("x.ATTRIBUTEy")
        //    8/ 1001 : 05          dc.b strlen("x.w y")
        // ```
        //
        // Twelve characters unsubstituted, then five substituted.
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro\n        dc.b strlen(\"x.ATTRIBUTEy\")\n        dc.b strlen(\"x.ATTRIBUTE y\")\n        endm\n        foo.w\n";
        assert_eq!(image(src), vec![12, 5]);
    }

    #[test]
    fn attribute_suffix_does_not_hijack_a_plain_mnemonic() {
        // No `move` macro is defined here — `move.w` must keep lowering as
        // the real instruction via `split_mnemonic_and_size`, confirming the
        // attribute-macro path (gated on the BASE name being a literal entry
        // in `self.macros`) never fires for ordinary suffixed mnemonics.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        move.w d1,d0\n";
        assert_eq!(image(src), vec![0x30, 0x01]);
    }

    #[test]
    fn while_loop_reevaluates_condition_each_iteration() {
        // asl-verified golden (`while_loop`): `n set 0 / while (n<3) / dc.b n
        // / n set n+1 / endm` → `00 01 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 0\n        while (n<3)\n        dc.b n\nn       set n+1\n        endm\n";
        assert_eq!(image(src), vec![0, 1, 2]);
    }

    #[test]
    fn while_loop_never_entered_emits_nothing() {
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 5\n        while (n<0)\n        dc.b 1\n        endm\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn while_loop_non_convergent_condition_diagnoses_instead_of_hanging() {
        // A5: a condition that never resolves to zero is bounded by
        // `WHILE_CAP` and diagnosed rather than hanging the assembler.
        let src = "        cpu 68000\n        padding off\n        phase 0\nn       set 1\n        while (n)\nn       set n\n        endm\n";
        let err = run(src, &Options::default())
            .expect_err("non-convergent while must diagnose, not hang");
        assert!(
            err.iter()
                .any(|d| d.message.contains("while loop did not terminate")),
            "expected a while-non-convergence diagnostic, got {err:?}"
        );
    }

    #[test]
    fn bang_align_pads_to_the_requested_boundary() {
        // asl-verified golden (`bang_align`): odd `dc.b 1`, `!align 2`,
        // `dc.b 2` → `01 00 02`.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b 1\n        !align 2\n        dc.b 2\n";
        assert_eq!(image(src), vec![1, 0, 2]);
    }

    #[test]
    fn bang_error_forces_the_builtin_directive_and_diagnoses() {
        // `even` is NOT a valid asl directive (verified "unknown
        // instruction"), so only `!error`/`!align` are in scope. A plain
        // `error` (bang or not) doesn't set `aborted`, but it does push a
        // `Level::Error` diagnostic, so `run` still fails the assembly
        // overall (no bytes emitted) — the observable "abort" the spec means.
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        !error \"boom\"\n";
        let err = run(src, &Options::default()).expect_err("!error must fail the assembly");
        assert!(
            err.iter().any(|d| d.message.contains("boom")),
            "got {err:?}"
        );
    }

    // ── T9.3: `lowstring` + `switch/case/elsecase/endcase` ────────────────

    #[test]
    fn lowstring_lowercases_a_plain_literal() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(lowstring(\"ABCD\"))\n";
        assert_eq!(image(src), vec![4]);
    }

    #[test]
    fn lowstring_nests_over_a_substr_argument() {
        // `lowstring(substr(...))` and `substr(lowstring(...), ...)` both
        // recurse through the same `eval_str` entry point (T9.3 doc on
        // `eval_str`), so nesting either way round works.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b strlen(substr(lowstring(\"ABCDEF\"),1,3))\n";
        assert_eq!(image(src), vec![3]);
    }

    #[test]
    fn switch_case_selects_the_matching_body() {
        // asl-verified golden (`switch_case_match`): `switch
        // lowstring("HeX") / case "hex" / dc.b $80 / case "dec" / dc.b $90 /
        // elsecase / dc.b $FF / endcase` → `80` — only the matching case's
        // body assembles, the others are skipped entirely.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch lowstring(\"HeX\")\n        case \"hex\"\n        dc.b $80\n        case \"dec\"\n        dc.b $90\n        elsecase\n        dc.b $FF\n        endcase\n";
        assert_eq!(image(src), vec![0x80]);
    }

    #[test]
    fn switch_falls_through_to_elsecase_when_nothing_matches() {
        // asl-verified golden (`switch_elsecase`): a switch value matching no
        // `case` literal takes the `elsecase` (default) body.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch lowstring(\"XYZ\")\n        case \"hex\"\n        dc.b $80\n        case \"dec\"\n        dc.b $90\n        elsecase\n        dc.b $FF\n        endcase\n";
        assert_eq!(image(src), vec![0xFF]);
    }

    #[test]
    fn switch_with_no_matching_case_and_no_elsecase_emits_nothing() {
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch \"nope\"\n        case \"hex\"\n        dc.b $80\n        endcase\n";
        assert_eq!(image(src), Vec::<u8>::new());
    }

    #[test]
    fn nested_switch_inside_a_case_body_resolves_independently() {
        // The outer switch picks its `case "a"` arm; the switch NESTED
        // inside that arm's body has its own independent case/elsecase
        // resolution — proves `find_block_end`'s nesting stack (and
        // `exec_switch`'s depth-0 arm scan) correctly isolate inner from
        // outer `switch`/`case`/`elsecase`/`endcase` keywords.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        switch \"a\"\n        case \"a\"\n        switch \"z\"\n        case \"y\"\n        dc.b 1\n        elsecase\n        dc.b 2\n        endcase\n        elsecase\n        dc.b 3\n        endcase\n";
        assert_eq!(image(src), vec![2]);
    }

    #[test]
    fn while_loop_nested_inside_a_macro_body_does_not_truncate_the_macro() {
        // Regression (T9.3 investigation): `find_block_end` used to
        // depth-count solely on the CALLER's own opener/closer pair, so
        // `capture_macro`'s `openers=["macro"]` scan didn't increment on a
        // nested `while`, and that nested while's own `endm` was mistaken
        // for the enclosing macro's `endm` — truncating the macro body
        // before its real end and losing the accumulator's increment line,
        // which then hung the (incompletely-captured) `while` until
        // `WHILE_CAP`. Fixed by keying the nesting stack per-opener (see
        // `closers_for`): `while … endm` nested inside `macro … endm` (the
        // exact shape debug-format macros like `__FSTRING_GenerateDecodedString`
        // use) must fully execute the loop AND run the line after it.
        let src = "        cpu 68000\n        padding off\n        phase 0\nfoo     macro n\ni       set 0\n        while (i<n)\n        dc.b i\ni       set i+1\n        endm\n        dc.b $FF\n        endm\n        foo 3\n";
        assert_eq!(image(src), vec![0, 1, 2, 0xFF]);
    }

    #[test]
    fn fstring_format_composition_matches_asl() {
        // The payoff (T9.3): a MINIMAL `%<…>`-parsing macro modeled on
        // `debugger.asm`'s `__FSTRING_GenerateDecodedString`, composing
        // `macro` + `while` + `switch`/`case`/`elsecase` + `lowstring` +
        // `substr`/`strstr`/`strlen`/`val` — every debug-surface primitive
        // from T9.1/T9.2/T9.3 in one control-flow shape. Literal text spans
        // emit their LENGTH (`strlen(substr(...))`) rather than their raw
        // bytes: `dc.b <string-expr>` (multi-byte ASCII emission for a
        // bare/computed string argument) was found to be unimplemented in
        // `directive_db` — a real, separate gap outside T9.3's scope (see
        // the T9.3 report) — so this substitutes a byte COUNT for the
        // literal spans while still emitting the real decoded VALUE
        // (`val(...)`) for each `%<…>` token, which is the actual "bytecode"
        // half of the real macro. Byte-for-byte verified against real asl
        // (`fstring_format` in `tests/snippets_golden.txt`): `01 80 01 0A 01`.
        let src = "        cpu 68000\n        padding off\n        phase 0\nhex     = $80\nendl    = $0A\nfstr    macro string\nlpos    set 0\nwpos    set strstr(string,\"%<\")\n        while (wpos>=0)\n        if (wpos-lpos>0)\n        dc.b strlen(substr(string,lpos,wpos-lpos))\n        endif\nepos    set strstr(substr(string,wpos+1,0),\">\")+wpos+1\n        switch substr(string,wpos+2,1)\n        case \".\"\n        switch lowstring(substr(string,wpos+2,2))\n        case \".b\"\n        dc.b val(substr(string,wpos+5,epos-wpos-5))\n        case \".w\"\n        dc.b val(substr(string,wpos+5,epos-wpos-5))|1\n        elsecase\n        dc.b val(substr(string,wpos+5,epos-wpos-5))|3\n        endcase\n        elsecase\n        dc.b val(substr(string,wpos+2,epos-wpos-2))\n        endcase\nlpos    set epos+1\nwpos    set strstr(substr(string,lpos,0),\"%<\")\n        if (wpos>=0)\nwpos    set wpos+lpos\n        endif\n        endm\n        dc.b strlen(substr(string,lpos,0))\n        endm\n        fstr \"A%<.b hex> %<endl>Z\"\n";
        assert_eq!(image(src), vec![0x01, 0x80, 0x01, 0x0A, 0x01]);
    }

    // ── T6c: `dc.b`/`db` STRING operands -> ASCII bytes (ROM header) ───────

    #[test]
    fn dc_b_string_literal_emits_ascii_bytes() {
        // asl-verified (`dc_b_string` in `tests/snippets_golden.txt`):
        // `dc.b "AB"` -> `41 42` (one ASCII byte per char), not a numeric fold.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b \"AB\"\n";
        assert_eq!(image(src), vec![0x41, 0x42]);
    }

    #[test]
    fn dc_b_mixes_string_and_numeric_operands() {
        // asl-verified (`dc_b_string_mixed`): `dc.b "Hi",0` -> `48 69 00` — a
        // string operand and a plain numeric operand in the same comma list.
        let src =
            "        cpu 68000\n        padding off\n        phase 0\n        dc.b \"Hi\",0\n";
        assert_eq!(image(src), vec![0x48, 0x69, 0x00]);
    }

    #[test]
    fn dc_b_substr_operand_emits_ascii_bytes() {
        // asl-verified (`dc_b_substr`): a T9.1 string-builtin call
        // (`substr(...)`) that RESOLVES to a string (as opposed to
        // `strlen(substr(...))`, which resolves to an int) also emits ASCII
        // bytes, not the byte count: `dc.b substr("hello",1,2)` -> `65 6C`.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b substr(\"hello\",1,2)\n";
        assert_eq!(image(src), vec![0x65, 0x6C]);
    }

    #[test]
    fn db_alias_also_emits_ascii_for_string_operands() {
        // `db` is the same directive as `dc.b` (see the dispatch match arm
        // `"db" | "dc.b" => self.directive_db(...)`), so it must get the same
        // string-operand handling.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        db \"AB\"\n";
        assert_eq!(image(src), vec![0x41, 0x42]);
    }

    #[test]
    fn dc_b_numeric_operand_still_folds_as_before() {
        // Regression guard: a plain numeric operand must still take the
        // numeric-fold path, not be misdetected as a string.
        let src = "        cpu 68000\n        padding off\n        phase 0\n        dc.b $41\n";
        assert_eq!(image(src), vec![0x41]);
    }

    #[test]
    fn duplicate_vma_base_sections_get_distinct_names() {
        let src = "\
        cpu 68000\n\
        phase $8000\n\
        dc.b 1\n\
        dephase\n\
        phase $8000\n\
        dc.b 2\n\
        dephase\n";
        let module = run(src, &Options::default()).expect("assemble");
        let names: Vec<&str> = module.sections.iter().map(|s| s.name.as_str()).collect();
        let unique: std::collections::HashSet<&&str> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "section names collided: {names:?}");
    }

    // -----------------------------------------------------------------------
    // `try_defer_long_imm` — R3's imm32 deferral, extended to absolute
    // destinations (port #1 hblank: `boot.asm:185`'s real shape,
    // `move.l #HBlank_Null, (HBlank_Handler_Ptr).w`). R3 originally covered
    // only bare `aN`/`dN` register destinations (`movea.l #SongTable, a0`);
    // this is the same imm32-source deferral with an `(abs).w`/`(abs).l`
    // destination instead of a register.
    // -----------------------------------------------------------------------

    /// A resolved `.w`-absolute destination whose source immediate is a
    /// genuinely cross-seam (never-resolving) symbol must defer to a
    /// `Value32Be` link fixup, not hard-error. Mirrors the real
    /// `move.l #HBlank_Null, (HBlank_Handler_Ptr).w` shape: `HBlank_Handler_Ptr`
    /// resolves locally (a `ds.l 1` label right here), `HBlank_Null` never
    /// does (it's `.emp`-side only) — so this must NOT hit
    /// `unresolved symbol` at the converged pass.
    #[test]
    fn move_l_imm_unresolved_source_defers_with_abs_w_dest() {
        let src = "\
        cpu 68000\n\
        RamPtr: ds.l 1\n\
        \tmove.l #CrossSeamOnly, (RamPtr).w\n";
        let m = run(src, &Options::default()).expect("assemble (must defer, not hard-error)");
        // `RamPtr: ds.l 1` opens a four-byte gap that the instruction after it
        // fills, then the instruction itself: opcode word + 4-byte imm32 fixup
        // hole + 2-byte abs.w dest ext word. asl+p2bin on this source give 12
        // bytes, `00 00 00 00` then `21FC …`.
        let bytes = m.sections.iter().find(|s| !s.image_bytes().is_empty()).map(|s| s.image_bytes());
        assert_eq!(
            bytes.as_deref().map(|b| b.len()),
            Some(DS_L_1_GAP + 8),
            "move.l #imm,(abs).w must encode to 8 bytes (opcode + imm32 + abs.w ext word), \
             behind the four-byte gap `ds.l 1` opens"
        );
    }

    /// Tranche-4 extension: the same imm32-source deferral with a `(d16,An)`
    /// destination — `test_particle.asm`'s real shape
    /// (`move.l #Ani_Particle, SST_anim_table(a0)`, the anim-table pointer
    /// write every object spawn template uses). The displacement (`$1A`
    /// here) resolves eagerly; only the source immediate defers. Source
    /// extension words precede the destination's, so the fixup hole stays
    /// at offset 2 with the d16 word after it.
    #[test]
    fn move_l_imm_unresolved_source_defers_with_disp_an_dest() {
        let src = "\
        cpu 68000\n\
        SST_anim_table = $1A\n\
        \tmove.l #CrossSeamOnly, SST_anim_table(a0)\n";
        let m = run(src, &Options::default()).expect("assemble (must defer, not hard-error)");
        let bytes = m.sections.iter().find(|s| !s.image_bytes().is_empty()).map(|s| s.image_bytes());
        assert_eq!(
            bytes.as_deref().map(|b| b.len()),
            Some(8),
            "move.l #imm,d16(An) must encode to 8 bytes (opcode + imm32 + d16 ext word)"
        );
        // Opcode: move.l #imm -> (d16,a0) = 0x217C; the d16 ext word ($001A)
        // sits AFTER the imm32 hole — the offset-2 proof in bytes.
        let b = bytes.unwrap();
        assert_eq!(&b[0..2], &[0x21, 0x7C], "move.l #imm,(d16,a0) opcode");
        assert_eq!(&b[6..8], &[0x00, 0x1A], "d16 ext word follows the imm32 hole");
    }

    /// Same shape with an explicit `.l`-absolute destination
    /// (`(abs).l`, 4-byte extension word) — the sibling width.
    #[test]
    fn move_l_imm_unresolved_source_defers_with_abs_l_dest() {
        let src = "\
        cpu 68000\n\
        \tmove.l #CrossSeamOnly, ($FFFF8022).l\n";
        let m = run(src, &Options::default()).expect("assemble (must defer, not hard-error)");
        let bytes = m.sections.iter().find(|s| !s.image_bytes().is_empty()).map(|s| s.image_bytes());
        assert_eq!(
            bytes.as_deref().map(|b| b.len()),
            Some(10),
            "move.l #imm,(abs).l must encode to 10 bytes (opcode + imm32 + abs.l ext word)"
        );
    }

    /// `movea.l` sibling: an unresolved source with an absolute destination is
    /// nonsensical for `movea` (its destination is always `aN`), so this stays
    /// out of scope — a plain regression guard that `movea.l #imm,aN` (the
    /// R3-original shape) is untouched by the abs-dest extension.
    #[test]
    fn movea_l_imm_unresolved_source_still_defers_with_reg_dest() {
        let src = "\
        cpu 68000\n\
        \tmovea.l #CrossSeamOnly, a0\n";
        let m = run(src, &Options::default()).expect("assemble (must defer, not hard-error)");
        let bytes = m.sections.iter().find(|s| !s.image_bytes().is_empty()).map(|s| s.image_bytes());
        assert_eq!(bytes.as_deref().map(|b| b.len()), Some(6));
    }

    /// The fixup's TARGET offset must be exactly right: byte-diff the deferred
    /// encoding against a resolved control case with the same shape (a known
    /// immediate instead of a cross-seam symbol) — the opcode and the abs.w
    /// extension word must be byte-identical; only the 4 immediate bytes
    /// (the fixup hole, here filled by the linker with the resolved value)
    /// differ. This pins that the abs.w destination's own extension word is
    /// unperturbed by the deferral (it's encoded by the SAME `lower_inst` call,
    /// after the immediate, exactly as the real `21FC 0000228E 8022` reference
    /// bytes show — see s4.lst:5794).
    #[test]
    fn move_l_imm_abs_w_dest_fixup_offset_matches_resolved_control() {
        let deferred_src = "\
        cpu 68000\n\
        RamPtr: ds.l 1\n\
        \tmove.l #CrossSeamOnly, (RamPtr).w\n";
        let resolved_src = "\
        cpu 68000\n\
        RamPtr: ds.l 1\n\
        \tmove.l #$1234, (RamPtr).w\n";
        let m_deferred = run(deferred_src, &Options::default()).expect("deferred assemble");
        let m_resolved = run(resolved_src, &Options::default()).expect("resolved assemble");
        let bytes_deferred = m_deferred
            .sections
            .iter()
            .find(|s| !s.image_bytes().is_empty())
            .map(|s| s.image_bytes())
            .expect("deferred section");
        let bytes_resolved = m_resolved
            .sections
            .iter()
            .find(|s| !s.image_bytes().is_empty())
            .map(|s| s.image_bytes())
            .expect("resolved section");
        // Both sources open with `RamPtr: ds.l 1`, whose four-byte gap the
        // instruction fills, so the instruction starts at `DS_L_1_GAP` and
        // every offset below is measured from there. asl+p2bin on the resolved
        // source give 12 bytes: `00 00 00 00 21 FC 00 00 12 34 00 00`.
        let g = DS_L_1_GAP;
        assert_eq!(&bytes_deferred[..g], &[0, 0, 0, 0], "the `ds.l 1` gap fills with zero");
        assert_eq!(&bytes_resolved[..g], &[0, 0, 0, 0], "the `ds.l 1` gap fills with zero");
        // Opcode word and the abs.w dest ext word must match byte-for-byte;
        // only the imm32 hole legitimately differs (0 placeholder vs the
        // resolved $1234).
        assert_eq!(&bytes_deferred[g..g + 2], &bytes_resolved[g..g + 2], "opcode word must match");
        assert_eq!(
            &bytes_deferred[g + 6..g + 8],
            &bytes_resolved[g + 6..g + 8],
            "abs.w dest ext word must match"
        );
        assert_eq!(
            &bytes_deferred[g + 2..g + 6],
            &[0, 0, 0, 0],
            "deferred imm32 hole is the zero placeholder"
        );
        assert_eq!(
            &bytes_resolved[g + 2..g + 6],
            &[0x00, 0x00, 0x12, 0x34],
            "resolved control's imm32"
        );
    }

    // -----------------------------------------------------------------------
    // Port #2 (math.emp): `jsr`/`jmp` to a symbol genuinely undefined within
    // this AS compile unit (a cross-seam `.emp` proc, joined only at LINK
    // time) must DEFER as a `Fragment::JmpJsrSym` — not hard-error. Mirrors
    // aeon's real `games/sonic4/objects/test_parent.asm:96` shape (`jsr
    // GetSineCosine`), where `GetSineCosine` is defined in a sibling
    // `.emp` module when `SIGIL_EMP_MATH` is on.
    // -----------------------------------------------------------------------

    #[test]
    fn m68k_jsr_to_genuinely_external_symbol_defers_instead_of_erroring() {
        // No `GetSineCosine` anywhere in this source — exactly the shape a
        // real cross-seam `.emp` proc call takes from the AS side.
        let src = "    cpu 68000\nConsumer:\n    jsr GetSineCosine\n    rts\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).unwrap_or_else(|d| {
            panic!("expected a deferred compile, not a hard error: {d:?}")
        });
        // The jsr fragment must be a length-variable JmpJsrSym (deferred to
        // the linker's relaxation fixpoint), not a finished Data fragment.
        let frag = &m.sections[0].fragments[0];
        assert!(
            matches!(frag, sigil_ir::Fragment::JmpJsrSym { .. }),
            "expected Fragment::JmpJsrSym, got {frag:?}"
        );
    }

    #[test]
    fn m68k_jsr_to_genuinely_external_symbol_resolves_end_to_end_via_joint_link() {
        // The deferred JmpJsrSym must actually resolve once joined with a
        // section that DOES define the target — the end-to-end proof (mirrors
        // `math_port.rs`'s outbound-consumer harness pattern).
        let src = "    cpu 68000\nConsumer:\n    jsr GetSineCosine\n    rts\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).expect("deferred compile must succeed");

        let target_src = "    cpu 68000\n    phase $2468\nGetSineCosine:\n    rts\n";
        let target_m = run(target_src, &opts).expect("target assemble");
        let mut target_sections = target_m.sections;
        for sec in &mut target_sections {
            sec.lma = 0x2468;
            sec.placement = sigil_ir::SectionPlacement::Pinned;
        }

        let mut sections = m.sections;
        sections.extend(target_sections);
        let resolved = sigil_link::resolve_layout(&sections, &sigil_ir::SymbolTable::new(), true)
            .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
            .unwrap_or_else(|d| panic!("link: {d:?}"));
        let consumer = linked.sections.iter().find(|s| s.lma == 0).expect("consumer section");
        // jsr abs.w GetSineCosine ($2468, low address -> abs.w rung):
        // opcode 4EB8, address $2468.
        assert_eq!(&consumer.bytes[0..4], &[0x4E, 0xB8, 0x24, 0x68]);
    }

    #[test]
    fn m68k_jsr_to_locally_resolved_symbol_is_unaffected_by_the_deferral() {
        // A `jsr` target that DOES resolve within the same compile must stay
        // on the existing eager path, byte-identical to before this change —
        // the inertness proof for every currently-passing jsr/jmp compile.
        // Same source/expectation as
        // `m68k_jmp_jsr_bare_symbol_selects_width_in_front_end`, but for jsr.
        let src = "    cpu 68000\n    phase 0\nLbl:\n    jsr Lbl\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).expect("assemble");
        assert!(
            matches!(m.sections[0].fragments[0], sigil_ir::Fragment::Data(_)),
            "a locally-resolved jsr target must stay a finished Data fragment, not defer"
        );
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
        let bytes = sigil_link::flatten(&linked, 0x00);
        assert_eq!(bytes, vec![0x4E, 0xB8, 0x00, 0x00]);
    }

    #[test]
    fn m68k_jsr_to_truly_undefined_symbol_still_errors_after_joint_link() {
        // A deferred JmpJsrSym whose target is NEVER supplied by anything
        // (not even a sibling module) must still fail LOUD at resolve_layout
        // — deferral must not turn a genuine typo into a silent zero. And the
        // error must NAME the symbol with the cross-seam steer (I1 review
        // finding): the deferral moved a pure-AS typo'd jsr from assemble-time
        // (which named the symbol) to this link-time arm, so the link-time
        // wording must be at least as good.
        let src = "    cpu 68000\nConsumer:\n    jsr TotallyUndefined\n    rts\n";
        let opts = Options { initial_cpu: Some(Cpu::M68000), defines: vec![], include_root: None, guarded_defines: vec![], };
        let m = run(src, &opts).expect("deferred compile must succeed (front-end no longer errors)");
        let err = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect_err("a target defined nowhere must still fail at resolve_layout");
        assert!(
            err.iter().any(|d| d.level == sigil_span::Level::Error
                && d.message.contains("jmp/jsr target")
                && d.message.contains("`TotallyUndefined`")
                && d.message.contains("not defined in this link")
                && d.message.contains("cross-seam")),
            "expected a loud resolve_layout error naming `TotallyUndefined` with the \
             cross-seam steer, got: {err:?}"
        );
    }

    // ---- case folding: directives and mnemonics fold, symbols never do ----

    /// The one that decides the target processor, and the one whose failure is
    /// silent rather than loud: `Options::default()` starts on the Z80, where
    /// `$` lexes as the program counter instead of a hex prefix. An unfolded
    /// `CPU 68000` therefore does not produce a diagnostic — it leaves a 68000
    /// source assembling as a Z80 program. The witness is a 68000 encoding:
    /// `moveq #0,d0` is `70 00`, and there is no Z80 reading of that line.
    #[test]
    fn uppercase_cpu_directive_selects_the_68000() {
        assert_eq!(
            image("        CPU 68000\n        padding off\n        phase 0\n        moveq #0,d0\n"),
            vec![0x70, 0x00],
            "`CPU 68000` in capitals must select the 68000"
        );
        // `CPU Z80` folds in the same place: the operand is as much part of the
        // directive as the keyword is. `ld a,0` is `3E 00` on the Z80.
        assert_eq!(
            image("        CPU Z80\n        ld a,0\n"),
            vec![0x3E, 0x00],
            "`CPU Z80` in capitals must select the Z80"
        );
        // And `$` must now lex as a hex prefix, which it does not under Z80.
        assert_eq!(
            image("        CPU 68000\n        padding off\n        phase 0\n        dc.b $AB\n"),
            vec![0xAB],
            "under a folded `CPU 68000`, `$AB` is a hex literal, not the PC"
        );
    }

    /// THE GUARD ON THE CONSTRAINT. Symbols are documented case-sensitive
    /// (`lib.rs`) and `.emp` shares this namespace, so a fold that reached the
    /// symbol table would silently merge two distinct names. Two symbols
    /// differing only in case must stay two symbols with their own values —
    /// asserted on the EMITTED BYTES, so it holds regardless of how the
    /// environment happens to be keyed internally.
    #[test]
    fn symbols_differing_only_in_case_stay_distinct() {
        let head = "        cpu 68000\n        padding off\n        phase 0\n";
        assert_eq!(
            image(&format!("{head}Foo equ $11\nFOO equ $22\nfoo equ $33\n        dc.b Foo,FOO,foo\n")),
            vec![0x11, 0x22, 0x33],
            "`Foo`/`FOO`/`foo` must be three distinct symbols"
        );
        // Labels too, not just equates — and a label whose spelling collides
        // with a directive keyword in the other case must still be a label.
        assert_eq!(
            image(&format!(
                "{head}Bar:\n        dc.b $01\nBAR:\n        dc.b $02\n        dc.b Bar,BAR\n"
            )),
            vec![0x01, 0x02, 0x00, 0x01],
            "`Bar` and `BAR` must be distinct labels at distinct addresses"
        );
    }

    /// Directive keywords fold wherever they are recognized, and the sites are
    /// several: `dispatch`'s arms, the `<name> EQU <v>` intercept in `exec_one`,
    /// the same intercept behind a decorative colon-label, and the `.b`/`.w`/
    /// `.l` operand-size suffixes. Each row here is a DIFFERENT recognition
    /// site — folding one of them is not folding the others.
    #[test]
    fn directive_keywords_fold_at_every_recognition_site() {
        let head = "        CPU 68000\n        PADDING OFF\n        PHASE 0\n";
        // dispatch arms: DC.B / DC.W / DC.L, and `PADDING OFF` above (an
        // ON/OFF operand — an unfolded `OFF` reads as `on` and would insert an
        // alignment pad before the DC.W below).
        assert_eq!(
            image(&format!("{head}        DC.B $01\n        DC.W $0203\n")),
            vec![0x01, 0x02, 0x03],
            "DC.B/DC.W/PADDING OFF must all fold"
        );
        // `<name> EQU <v>` — the exec_one intercept, not a dispatch arm.
        assert_eq!(
            image(&format!("{head}Val EQU $2A\n        DC.B Val\n")),
            vec![0x2A]
        );
        // The same intercept behind a decorative colon-label: a SEPARATE site
        // in `exec_one`, reached only when the line carries `NAME:`.
        assert_eq!(
            image(&format!("{head}Val:    EQU $2B\n        DC.B Val\n")),
            vec![0x2B]
        );
        // SET, both spellings of the site.
        assert_eq!(
            image(&format!("{head}Acc SET $05\n        DC.B Acc\nAcc SET $06\n        DC.B Acc\n")),
            vec![0x05, 0x06]
        );
        // DS.B reserves. It advances the PC by its count AND opens a gap the
        // following `DC.B` fills, so the image carries both facts: `After` is
        // $03, and it sits at offset 3 behind two bytes of gap fill. asl+p2bin
        // on the same source give the same four bytes.
        assert_eq!(
            image(&format!("{head}        DC.B $01\n        DS.B 2\nAfter:\n        DC.B After\n")),
            vec![0x01, 0x00, 0x00, 0x03],
            "DS.B must fold and advance the PC by its count"
        );
        // ORG moves the PC; the folded keyword is what makes the label agree.
        assert_eq!(
            image(&format!("{head}        ORG $10\nHere:\n        DC.B Here\n")),
            vec![0x10],
            "ORG must fold"
        );
    }

    /// Block-structure keywords are recognized in `dispatch_head`/`closers_for`
    /// — the block-SCANNING layer, which never reaches `dispatch` at all. An
    /// unfolded `IF`/`ENDIF` does not error; it silently fails to open a block,
    /// so the wrong arm assembles. Assert on which arm's bytes came out.
    #[test]
    fn block_keywords_fold_in_the_scanning_layer() {
        let head = "        CPU 68000\n        PADDING OFF\n        PHASE 0\n";
        assert_eq!(
            image(&format!("{head}        IF 1\n        DC.B $AA\n        ELSE\n        DC.B $BB\n        ENDIF\n")),
            vec![0xAA],
            "IF/ELSE/ENDIF must fold — the taken arm is the `1` arm"
        );
        assert_eq!(
            image(&format!("{head}        IF 0\n        DC.B $AA\n        ELSE\n        DC.B $BB\n        ENDIF\n")),
            vec![0xBB]
        );
        assert_eq!(
            image(&format!("{head}        REPT 3\n        DC.B $77\n        ENDR\n")),
            vec![0x77, 0x77, 0x77],
            "REPT/ENDR must fold"
        );
        // STRUCT/ENDSTRUCT — the corpus spells these in capitals, and the
        // member offsets they define are what the rest of the file indexes by.
        assert_eq!(
            image(&format!(
                "{head}Rec STRUCT\nfirst   DS.B 1\nsecond  DS.W 1\n        ENDSTRUCT\n        DC.B Rec_first,Rec_second,Rec_len\n"
            )),
            vec![0x00, 0x01, 0x03],
            "STRUCT/ENDSTRUCT and their DS.* member widths must fold"
        );
        // MACRO/ENDM, and an invocation whose macro NAME keeps its own case.
        assert_eq!(
            image(&format!("{head}Emit MACRO v\n        DC.B v\n        ENDM\n        Emit $5A\n")),
            vec![0x5A],
            "MACRO/ENDM must fold"
        );
    }

    /// A macro name is a symbol, so it does NOT fold: `Emit` and `emit` are two
    /// macros. This is the same constraint as `symbols_differing_only_in_case`,
    /// checked at the one place where a folded head would have been convenient.
    #[test]
    fn macro_names_do_not_fold() {
        let head = "        cpu 68000\n        padding off\n        phase 0\n";
        assert_eq!(
            image(&format!(
                "{head}Emit macro v\n        dc.b $A0+v\n        endm\nemit macro v\n        dc.b $B0+v\n        endm\n        Emit 1\n        emit 2\n"
            )),
            vec![0xA1, 0xB2],
            "`Emit` and `emit` must stay two distinct macros"
        );
    }

    /// Mnemonics and their size suffixes fold, on both processors.
    #[test]
    fn mnemonics_and_size_suffixes_fold() {
        let head = "        CPU 68000\n        PADDING OFF\n        PHASE 0\n";
        assert_eq!(image(&format!("{head}        MOVEQ #0,d0\n")), vec![0x70, 0x00]);
        assert_eq!(image(&format!("{head}        MOVE.W d0,d1\n")), vec![0x32, 0x00]);
        // Mixed case in the suffix alone — a separate code path from the base.
        assert_eq!(image(&format!("{head}        move.W d0,d1\n")), vec![0x32, 0x00]);
        assert_eq!(image(&format!("{head}        Move.L d0,d1\n")), vec![0x22, 0x00]);
        assert_eq!(image(&format!("{head}        NOP\n")), vec![0x4E, 0x71]);
        // Z80 side.
        assert_eq!(image("        CPU Z80\n        NOP\n"), vec![0x00]);
        assert_eq!(image("        CPU Z80\n        LD a,0\n"), vec![0x3E, 0x00]);
    }

    /// `split_attribute_suffix` recognizes the suffix without regard to case
    /// but hands the macro body the spelling the CALL SITE wrote, because
    /// `.ATTRIBUTE` is a verbatim textual substitution — the body may paste it
    /// straight onto a mnemonic, and the fold has to keep that self-consistent
    /// rather than canonicalize someone else's text.
    #[test]
    fn attribute_macro_suffix_folds_and_substitutes_verbatim() {
        let head = "        cpu 68000\n        padding off\n        phase 0\n";
        let src = format!("{head}emit macro\n        move.ATTRIBUTE d0,d1\n        endm\n        emit.W\n");
        assert_eq!(
            image(&src),
            vec![0x32, 0x00],
            "`emit.W` must reach the attribute-macro path and paste `.W` onto `move`"
        );
    }

    #[test]
    fn fold_kw_leaves_lower_case_borrowed_and_does_not_touch_digits() {
        use super::fold_kw;
        assert!(matches!(fold_kw("move.w"), std::borrow::Cow::Borrowed(_)));
        assert_eq!(fold_kw("MOVE.W"), "move.w");
        assert_eq!(fold_kw("Sonic_Object_1"), "sonic_object_1");
        assert_eq!(fold_kw("68000"), "68000");
    }

    #[test]
    fn split_dot_suffix_only_splits_a_single_trailing_letter() {
        use super::split_dot_suffix;
        assert_eq!(split_dot_suffix("move.w"), Some(("move", b'w')));
        assert_eq!(split_dot_suffix("move.W"), Some(("move", b'w')));
        assert_eq!(split_dot_suffix("move"), None);
        assert_eq!(split_dot_suffix("a.b.c"), Some(("a.b", b'c')));
        assert_eq!(split_dot_suffix("x."), None);
        assert_eq!(split_dot_suffix("."), None);
        assert_eq!(split_dot_suffix(""), None);
        // A trailing dot-DIGIT is not a size suffix.
        assert_eq!(split_dot_suffix("foo.1"), None);
    }

    /// AS symbol-name composition. Every expected value below is READ OFF an
    /// `asl -L` listing of the same source (AS V1.42 Beta Bld 212,
    /// `s2disasm/build_tools/Linux-x86_64/asl`), never off sigil's own output:
    ///
    /// ```text
    ///   7/     100 : 0055                    dc.w zone_id_{cur_str}
    ///   9/     104 : =$77                 zone_id_{cur_str}b = $77
    ///  10/     104 : 0077                    dc.w zone_id_3b
    ///  11/     106 : 0055                    dc.w zone_id_{cur}
    /// ```
    ///
    /// so: the group takes a string-valued symbol OR an integer one, it composes
    /// on the DEFINING side as well as in an operand, and an integer contributes
    /// its decimal digits.
    #[test]
    fn name_brace_composes_a_symbol_name_from_a_string_or_an_integer() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "cur := 3\n",
            "cur_str := \"3\"\n",
            "zone_id_3 = $55\n",
            "	dc.w zone_id_{cur_str}\n",
            "	dc.w zone_id_{cur}\n",
            "zone_id_{cur_str}b = $77\n",
            "	dc.w zone_id_3b\n",
        );
        assert_eq!(image(src), vec![0x00, 0x55, 0x00, 0x55, 0x00, 0x77]);
    }

    /// A group may lead the name, sit inside it, hold a full expression, and
    /// several may compose one name. asl listing of the same four lines:
    ///
    /// ```text
    ///   8/     100 : 0022                    dc.w {"n"}{cur}
    ///   9/     102 : 0055                    dc.w zone_id_{cur}_x
    ///  10/     104 : 0066                    dc.w xx{cur+0}
    /// ```
    #[test]
    fn name_brace_composes_at_any_position_and_from_an_expression() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "cur := 3\n",
            "n3 = $22\n",
            "zone_id_3_x = $55\n",
            "xx3 = $66\n",
            "	dc.w {\"n\"}{cur}\n",
            "	dc.w zone_id_{cur}_x\n",
            "	dc.w xx{cur+0}\n",
        );
        assert_eq!(image(src), vec![0x00, 0x22, 0x00, 0x55, 0x00, 0x66]);
    }

    /// Composition stops at a string literal and at a comment. asl emits the
    /// braces as literal text — listing row for `dc.b "brace {cur} in string"`:
    ///
    /// ```text
    ///  11/     106 : 6272 6163 6520 7B63 7572 7D20 696E 2073 7472 696E 67
    /// ```
    ///
    /// (`7B 63 75 72 7D` is `{cur}` itself), and a trailing `; … {` comment — the
    /// shape `s2.asm`'s `; struct blockMapElement {` has — must not be scanned.
    #[test]
    fn name_brace_is_inert_inside_a_string_literal_and_in_a_comment() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "cur := 3\n",
            "	dc.b \"brace {cur} in string\"\n",
            "	dc.b $99	; struct blockMapElement {\n",
        );
        let mut want: Vec<u8> = b"brace {cur} in string".to_vec();
        want.push(0x99);
        assert_eq!(image(src), want);
    }

    /// A `{…}` group whose expression does not resolve is a hard error, never a
    /// silently truncated name: without the group the line would read
    /// `zone_id_`, a DIFFERENT symbol that may well exist.
    #[test]
    fn name_brace_that_does_not_resolve_is_diagnosed() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zone_id_ = $11\n",
            "	dc.w zone_id_{nosuch}\n",
        );
        let diags = run(src, &Options::default())
            .expect_err("an unresolvable `{…}` name group must fail the assembly");
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("`{nosuch}` in a symbol name did not resolve")),
            "expected the name-composition diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// `\{expr}` in a string folds where the string is BOUND, not where it is
    /// read: `s` keeps the digit `3` across the later `n := 42`. asl listing —
    /// the `="3"` on the assignment row is asl showing the value it stored:
    ///
    /// ```text
    ///   4/     100 : ="3"                 s := "\{n}"
    ///   6/     102 : 33                      dc.b s
    ///   7/     103 : =$2A                 n := 42
    ///   8/     103 : 33                      dc.b s
    /// ```
    #[test]
    fn string_set_folds_interpolation_at_binding_time() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "n := 3\n",
            "s := \"\\{n}\"\n",
            "	dc.b s\n",
            "n := 42\n",
            "	dc.b s\n",
        );
        assert_eq!(image(src), vec![0x33, 0x33]);
    }

    /// The closing `}` is found across a string literal that contains one, so a
    /// bound string may itself be composed into a name — the shape
    /// `s2.macros.asm`'s `zoneanimcount_{"\{zoneanimcur}"}` uses.
    #[test]
    fn name_brace_closes_across_a_literal_holding_a_brace() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "k := 7\n",
            "count_7 = $44\n",
            "	dc.w count_{\"\\{k}\"}\n",
        );
        assert_eq!(image(src), vec![0x00, 0x44]);
    }

    // ── `shift`: the variadic macro argument walk ───────────────────────────
    //
    // Every expected value below is read off an `asl -L -U` listing row
    // (AS V1.42 Beta Bld 212, `s2disasm/build_tools/Linux-x86_64/asl`), quoted
    // in each test. `-U` is the invocation: it sets asl's `CASESENSITIVE` to 1,
    // which is the namespace this front-end implements and the flag every asl
    // oracle in this repo passes. Without it asl folds every identifier —
    // including each macro argument's value at bind time — and the rows below
    // come back upper-cased, describing a different assembler. The probe
    // sources are recorded in
    // `docs/superpowers/notes/2026-09-03-as-shift-macro-argument-walk.md`.

    /// The shape both corpus uses have: guard on the first argument, emit,
    /// `shift` past what was consumed, re-invoke with `ALLARGS`. The recursion
    /// terminates when `ALLARGS` runs dry and the guard sees an unbound
    /// parameter.
    ///
    /// asl listing, `cp 1,2,3,4,5,6` on params `a1,a2`:
    /// ```text
    ///   16/  0 : (MACRO)     cp 1,2,3,4,5,6
    ///   16/  0 : 01            dc.b 1
    ///   16/  1 : 02            dc.b 2
    ///   16/  2 : (MACRO-2)     cp 3,4,5,6
    ///   16/  2 : 03            dc.b 3
    ///   16/  3 : 04            dc.b 4
    ///   16/  4 : (MACRO-3)     cp 5,6
    ///   16/  4 : 05            dc.b 5
    ///   16/  5 : 06            dc.b 6
    ///   16/  6 : (MACRO-4)     cp
    ///   16/  6 : FF            dc.b $FF
    /// ```
    #[test]
    fn shift_drives_the_variadic_argument_walk() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "cp macro a1,a2\n",
            "	if \"a1\"<>\"\"\n",
            "	dc.b a1\n",
            "	dc.b a2\n",
            "	shift\n",
            "	shift\n",
            "	cp ALLARGS\n",
            "	else\n",
            "	dc.b $FF\n",
            "	endif\n",
            "	endm\n",
            "	cp 1,2,3,4,5,6\n",
        );
        assert_eq!(image(src), vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xFF]);
    }

    /// `ALLARGS` loses its leading group per shift, down to empty, and a shift
    /// past exhaustion leaves it empty rather than erroring.
    ///
    /// asl listing, `zb aa,bb,cc,dd` on params `b1,b2,b3` (`strlen` of the
    /// substituted `ALLARGS` after each shift):
    /// ```text
    ///   41/  0 : 0B     dc.b strlen("aa,bb,cc,dd")
    ///   41/  1 : 08     dc.b strlen("bb,cc,dd")
    ///   41/  2 : 05     dc.b strlen("cc,dd")
    ///   41/  3 : 02     dc.b strlen("dd")
    ///   41/  4 : 00     dc.b strlen("")
    ///   41/  5 : 00     dc.b strlen("")
    /// ```
    #[test]
    fn shift_drops_one_leading_group_from_allargs() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zb macro b1,b2,b3\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "	zb aa,bb,cc,dd\n",
        );
        assert_eq!(image(src), vec![0x0B, 0x08, 0x05, 0x02, 0x00, 0x00]);
    }

    /// Parameters slide left one argument per shift, and the vacated tail slot
    /// becomes empty.
    ///
    /// asl listing, `zc 1,2,3` on params `c1,c2,c3` — emitting `c1`,`c3`, then
    /// after a shift `c1`,`c2`, then after another `c1`:
    /// ```text
    ///   44/  D : 01     dc.b 1
    ///   44/  E : 03     dc.b 3
    ///   44/  F : 02     dc.b 2
    ///   44/ 10 : 03     dc.b 3
    ///   44/ 11 : 03     dc.b 3
    /// ```
    #[test]
    fn shift_walks_the_parameters_one_argument_at_a_time() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zc macro c1,c2,c3\n",
            "	dc.b c1\n",
            "	dc.b c3\n",
            "	shift\n",
            "	dc.b c1\n",
            "	dc.b c2\n",
            "	shift\n",
            "	dc.b c1\n",
            "	endm\n",
            "	zc 1,2,3\n",
        );
        assert_eq!(image(src), vec![0x01, 0x03, 0x02, 0x03, 0x03]);
    }

    /// The parameter vector is not a window that slides along the argument
    /// list: it holds one slot per DECLARED parameter and empty-fills behind
    /// the shift, so with two parameters and four arguments the third and
    /// fourth arguments never reach a parameter at all — even though `ALLARGS`
    /// still carries them.
    ///
    /// asl listing, `pw 5,6,7,8` on params `q1,q2`, emitting `q1` after each
    /// shift — the third emission has no operand left and asl diagnoses it:
    /// ```text
    ///   24/  3 : 05                  dc.b 5
    ///   24/  4 : 06                  dc.b 6
    ///   > > > p5.asm(24) PW(5):14: error: invalid symbol name
    ///   24/  5 :                     dc.b
    /// ```
    #[test]
    fn shift_empty_fills_the_parameter_vector_rather_than_rewindowing() {
        let head = "	cpu 68000\n	padding off\n	phase 0\n";
        let body = concat!(
            "pw macro q1,q2\n",
            "	dc.b q1\n",
            "	shift\n",
            "	dc.b q1\n",
            "	shift\n",
            "	dc.b q1\n",
            "	endm\n",
        );
        // Two shifts exhaust a two-parameter vector: `q1` is empty, not `7`.
        assert!(
            run(&format!("{head}{body}	pw 5,6,7,8\n"), &Options::default()).is_err(),
            "the third argument must not reach `q1` after two shifts"
        );
        // Control: one shift stays within the parameter count and assembles.
        let one_shift = concat!(
            "pw macro q1,q2\n",
            "	dc.b q1\n",
            "	shift\n",
            "	dc.b q1\n",
            "	endm\n",
            "	pw 5,6,7,8\n",
        );
        assert_eq!(image(&format!("{head}{one_shift}")), vec![0x05, 0x06]);
    }

    /// A `rept` body is substituted once, where the loop is entered, and
    /// replayed: a `shift` inside it advances the frame — visible after the
    /// loop — without rewriting the body's own text.
    ///
    /// asl listing, `zd aaa,bbbb,ccccc` on param `d1`, a `rept 2` whose body
    /// shifts and emits `strlen(ALLARGS)`, then one emission after the loop:
    /// ```text
    ///   42/  6 : 0E     dc.b strlen("aaa,bbbb,ccccc")
    ///   42/  7 : 0E     dc.b strlen("aaa,bbbb,ccccc")
    ///   42/  8 : 0E     dc.b strlen("aaa,bbbb,ccccc")
    ///   42/  9 : 05     dc.b strlen("ccccc")
    /// ```
    #[test]
    fn a_shift_inside_a_rept_body_advances_the_frame_but_not_the_body_text() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zd macro d1\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	rept 2\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "	zd aaa,bbbb,ccccc\n",
        );
        assert_eq!(image(src), vec![0x0E, 0x0E, 0x0E, 0x05]);
    }

    /// Shift state belongs to one expansion. An inner macro's shift consumes
    /// the inner call's arguments and leaves the caller's binding intact.
    ///
    /// asl listing, `eout aaaa,bbbbb` calling `ein qq,rrr` (which shifts):
    /// ```text
    ///   43/  A : 0A                  dc.b strlen("aaaa,bbbbb")
    ///   43/  B : (MACRO-2)            ein qq,rrr
    ///   43/  B : 03                  dc.b strlen("rrr")
    ///   43/  C : 0A                  dc.b strlen("aaaa,bbbbb")
    /// ```
    #[test]
    fn an_inner_expansions_shift_leaves_the_outer_frame_alone() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "ein macro e1\n",
            "	shift\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "eout macro e2\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	ein qq,rrr\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "	eout aaaa,bbbbb\n",
        );
        assert_eq!(image(src), vec![0x0A, 0x03, 0x0A]);
    }

    /// A macro DEFINED inside an expanding body captures text the enclosing
    /// expansion has already substituted, `ALLARGS` included and frozen at the
    /// shift state in force at capture. Its own invocation arguments do not
    /// rebind it.
    ///
    /// asl listing, `zf aa,bbb,cccc` shifting once, then defining `zfin` whose
    /// body reads `ALLARGS`, then calling `zfin zzzzz`:
    /// ```text
    ///   44/  D : (MACRO-2)   zfin zzzzz
    ///   44/  D : 08          dc.b strlen("bbb,cccc")
    /// ```
    /// `08` is the OUTER post-shift `ALLARGS` (`bbb,cccc`), not the inner
    /// call's `zzzzz` (which would be 5).
    #[test]
    fn a_macro_defined_inside_an_expansion_freezes_the_outer_allargs() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zf macro f1\n",
            "	shift\n",
            "zfin macro g1\n",
            "	dc.b strlen(\"ALLARGS\")\n",
            "	endm\n",
            "	zfin zzzzz\n",
            "	endm\n",
            "	zf aa,bbb,cccc\n",
        );
        assert_eq!(image(src), vec![0x08]);
    }

    /// `shift` needs an expansion to shift. asl reports it through its
    /// not-in-a-macro check (`p4.asm(6): error: EXITM not called from within
    /// macro` for a bare `shift` at top level); sigil names the directive that
    /// was written.
    #[test]
    fn shift_outside_a_macro_expansion_is_an_error() {
        let src = "	cpu 68000\n	padding off\n	phase 0\n	shift\n";
        let diags = run(src, &Options::default()).expect_err("bare `shift` must diagnose");
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("`shift` outside a macro expansion")),
            "expected the outside-a-macro refusal, got {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// Argument text carries its case through a `shift` and into emitted bytes.
    ///
    /// asl `-U` — the case-sensitive invocation this front-end is the
    /// compatible surface for, and the one every `asl` oracle in this repo runs
    /// — applies no case transformation to a macro argument at any point, so
    /// what the caller wrote is what lands in the data:
    ///
    /// ```text
    ///   11/ 1000 : (MACRO)                  ws    aa, bb , cc
    ///   11/ 1000 : 453C 6161 2C62              dc.b    "E<aa,bb,cc>"
    ///   11/ 100B : 533C 6262 2C63              dc.b    "S<bb,cc>"
    /// ```
    ///
    /// The row also pins the rendering: `ALLARGS` before a shift is the
    /// invocation's argument run with the separators normalized (the written
    /// `aa, bb , cc` renders `aa,bb,cc`), and after a shift it is the surviving
    /// groups rejoined.
    #[test]
    fn shift_carries_argument_case_into_emitted_bytes() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "ws macro pp,qq\n",
            "	dc.b \"E<ALLARGS>\"\n",
            "	shift\n",
            "	dc.b \"S<ALLARGS>\"\n",
            "	endm\n",
            "	ws aa, bb , cc\n",
        );
        assert_eq!(image(src), b"E<aa,bb,cc>S<bb,cc>".to_vec());
    }

    /// A post-shift `ALLARGS` that composes a symbol NAME resolves the symbol
    /// the caller spelled, case for case — the silent-wrong-answer direction,
    /// since a folded spelling would be a DIFFERENT symbol in this front-end's
    /// case-sensitive namespace and would resolve to a different value rather
    /// than fail.
    ///
    /// asl `-U`, `Mix_Ss equ $77` and `pick zz,Ss` on param `qq`, shifting once:
    ///
    /// ```text
    ///    9/    0 : (MACRO)                  pick    zz,Ss
    ///    9/    0 : 0077                        dc.w    Mix_{"Ss"}
    /// ```
    ///
    /// and the folded spelling names nothing, rather than the same symbol:
    ///
    /// ```text
    ///    9/    0 : (MACRO)                  pick    zz,SS
    ///  > > > n7.asm(9) pick(2):17: error #1010: symbol undefined
    ///  > > >         dc.w    Mix_{"SS"}
    /// ```
    #[test]
    fn a_composed_name_from_a_post_shift_allargs_keeps_the_arguments_case() {
        let head = "	cpu 68000\n	padding off\n	phase 0\nMix_Ss equ $77\n";
        let body = concat!(
            "pick macro qq\n",
            "	shift\n",
            "	dc.w Mix_{\"ALLARGS\"}\n",
            "	endm\n",
        );
        assert_eq!(image(&format!("{head}{body}	pick zz,Ss\n")), vec![0x00, 0x77]);
        // The folded spelling composes a name nothing declares, so it survives
        // the front-end as an unresolved reference rather than folding onto
        // `Mix_Ss` and quietly assembling to $77.
        let m = run(&format!("{head}{body}	pick zz,SS\n"), &Options::default())
            .expect("the folded call still lowers");
        let targets: Vec<String> = m.sections[0]
            .fragments
            .iter()
            .flat_map(|f| match f {
                sigil_ir::Fragment::Data(d) => {
                    d.fixups.iter().map(|x| format!("{:?}", x.target)).collect()
                }
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(targets, vec![r#"Sym("Mix_SS")"#.to_string()]);
    }

    /// What a binding pastes in is the caller's text and stays it. A macro
    /// argument whose text happens to spell one of the callee's own parameter
    /// names must NOT be substituted a second time — AS resolves a body's
    /// parameter references to placeholders when the macro is captured, so text
    /// arriving at expansion time is inert.
    ///
    /// asl `-U`, `mm macro pp,qq` — the first call passes `qq` as a value, the
    /// second passes `pp`:
    ///
    /// ```text
    ///   11/ 1000 : (MACRO)                  mm    qq,zz
    ///   11/ 1000 : 453C 7171 2C7A              dc.b    "E<qq,zz>"
    ///   11/ 1008 : 533C 7A7A 3E                dc.b    "S<zz>"
    ///   12/ 100D : (MACRO)                  mm    xx,pp,yy
    ///   12/ 100D : 453C 7878 2C70              dc.b    "E<xx,pp,yy>"
    ///   12/ 1018 : 533C 7070 2C79              dc.b    "S<pp,yy>"
    /// ```
    #[test]
    fn pasted_argument_text_is_not_rescanned_for_parameter_names() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mm macro pp,qq\n",
            "	dc.b \"E<ALLARGS>\"\n",
            "	shift\n",
            "	dc.b \"S<ALLARGS>\"\n",
            "	endm\n",
            "	mm qq,zz\n",
            "	mm xx,pp,yy\n",
        );
        assert_eq!(
            image(src),
            b"E<qq,zz>S<zz>E<xx,pp,yy>S<pp,yy>".to_vec()
        );
    }

    /// A keyword call's `ALLARGS` before any shift is the invocation text as
    /// WRITTEN — keyword syntax, written order and all — while after a shift it
    /// is the supplied slots rejoined in PARAMETER order. The two renderings
    /// therefore disagree here, and asl `-U` matches the written text:
    ///
    /// ```text
    ///   11/ 1000 : (MACRO)                  kw    k2=aa,k1=bb
    ///   11/ 1000 : 453C 6B32 3D61              dc.b    "E<k2=aa,k1=bb>"
    ///   11/ 100E : 533C 6161 3E                dc.b    "S<aa>"
    ///   13/ 1021 : (MACRO)                  kw    aa,k2=bb
    ///   13/ 1021 : 453C 6161 2C6B              dc.b    "E<aa,k2=bb>"
    ///   13/ 102C : 533C 6262 3E                dc.b    "S<bb>"
    /// ```
    ///
    /// `S<aa>` is the value bound to `k2`, the callee's second parameter — not
    /// the second group the caller wrote.
    #[test]
    fn a_keyword_calls_allargs_is_written_text_before_a_shift_and_parameter_order_after() {
        let head = "	cpu 68000\n	padding off\n	phase 0\n";
        let body = concat!(
            "kw macro k1,k2\n",
            "	dc.b \"E<ALLARGS>\"\n",
            "	shift\n",
            "	dc.b \"S<ALLARGS>\"\n",
            "	endm\n",
        );
        assert_eq!(
            image(&format!("{head}{body}	kw k2=aa,k1=bb\n")),
            b"E<k2=aa,k1=bb>S<aa>".to_vec()
        );
        assert_eq!(
            image(&format!("{head}{body}	kw aa,k2=bb\n")),
            b"E<aa,k2=bb>S<bb>".to_vec()
        );
    }

    // ── `.`-local scope inside a macro expansion ────────────────────────────
    //
    // Every expected value below is an `asl -L` row from AS V1.42 Beta Bld 212
    // (`s2disasm/build_tools/Linux-x86_64/asl`) run with the Sonic 2 build's own
    // flags, `-xx -n -q -A -L -U -i .` — `-U` above all, which forces the
    // case-sensitive namespace this front-end implements.

    /// A `.`-local bound by `:=` inside a macro expansion lands in the CALLER's
    /// scope, and is a real symbol there — readable both bare and qualified.
    ///
    /// ```text
    ///    8/    1000 : (MACRO)                mset
    ///    8/    1000 : =$7                  .v      :=      7
    ///    9/    1000 : 07                     dc.b    .v
    ///   10/    1001 : 07                     dc.b    Base.v
    ///
    ///    Base.v :                         7 - |
    /// ```
    #[test]
    fn a_dot_local_set_inside_a_macro_binds_in_the_callers_scope() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mset macro\n",
            ".v := 7\n",
            "	endm\n",
            "Base:\n",
            "	mset\n",
            "	dc.b .v\n",
            "	dc.b Base.v\n",
        );
        assert_eq!(image(src), vec![0x07, 0x07]);
    }

    /// `equ`, `=` and `set` bind the same way `:=` does — the split is
    /// syntactic form, not value kind.
    ///
    /// ```text
    ///   10/    1000 : (MACRO)                mform
    ///   10/    1000 : =$3                  .eqs    =       3
    ///   10/    1000 : =$4                  .sets   set     4
    ///   10/    1000 : =$5                  .asn    :=      5
    ///   11/    1000 : 03                     dc.b    .eqs
    ///   12/    1001 : 04                     dc.b    .sets
    ///   13/    1002 : 05                     dc.b    .asn
    ///
    ///    Base.asn :                       5 - |  Base.eqs :                       3 - |
    /// ```
    #[test]
    fn every_value_binding_form_of_a_dot_local_reaches_the_callers_scope() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mform macro\n",
            ".eqs = 3\n",
            ".sets set 4\n",
            ".asn := 5\n",
            "	endm\n",
            "Base:\n",
            "	mform\n",
            "	dc.b .eqs\n",
            "	dc.b .sets\n",
            "	dc.b .asn\n",
        );
        assert_eq!(image(src), vec![0x03, 0x04, 0x05]);
    }

    /// Expansion nesting is TRANSPARENT to a value-binding `.`-local: an inner
    /// macro's `:=` reaches the outermost caller's scope in one step, not the
    /// enclosing expansion's.
    ///
    /// ```text
    ///   12/    1000 : (MACRO)                outer
    ///   12/    1000 :  (MACRO-2)                   inner
    ///   12/    1000 : =$5                  .v      :=      5
    ///   12/    1000 : 05                          dc.b    .v
    ///   13/    1001 : 05                     dc.b    .v
    ///
    ///    Base.v :                         5 - |
    /// ```
    #[test]
    fn a_nested_expansions_dot_local_set_reaches_the_outermost_callers_scope() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "inner macro\n",
            ".v := 5\n",
            "	endm\n",
            "outer macro\n",
            "	inner\n",
            "	dc.b .v\n",
            "	endm\n",
            "Base:\n",
            "	outer\n",
            "	dc.b .v\n",
        );
        assert_eq!(image(src), vec![0x05, 0x05]);
    }

    /// A `.`-local written as a PLAIN LABEL is private to its expansion: two
    /// expansions in one scope each bind their own, and the name is not a
    /// caller-qualified symbol at all.
    ///
    /// ```text
    ///   10/    1000 : (MACRO)                mlab
    ///   10/    1000 : 6702                        beq.s   .done
    ///   10/    1002 : 4E71                        nop
    ///   10/    1004 :                     .done:
    ///   11/    1004 : (MACRO)                mlab
    ///   11/    1004 : 6702                        beq.s   .done
    ///   11/    1006 : 4E71                        nop
    ///   11/    1008 :                     .done:
    ///  > > > b1.asm(12):7: error #1010: symbol undefined
    ///  > > > .done
    ///  > > >  dc.w .done-Base
    /// ```
    #[test]
    fn a_dot_local_plain_label_belongs_to_its_own_expansion() {
        let head = "	cpu 68000\n	padding off\n	phase 0\n";
        let body = concat!(
            "mlab macro\n",
            "	beq.s .done\n",
            "	nop\n",
            ".done:\n",
            "	endm\n",
            "Base:\n",
            "	mlab\n",
            "	mlab\n",
        );
        assert_eq!(
            linked_image(&format!("{head}{body}")),
            vec![0x67, 0x02, 0x4E, 0x71, 0x67, 0x02, 0x4E, 0x71]
        );
        // …and the caller cannot see it, which is what makes the two expansions
        // legal rather than a double definition. asl says the same, by name:
        // `error #1010: symbol undefined` on `.done`, with no `Base.done` in the
        // symbol table. Here the caller-qualified name is what dangles.
        let m = run(
            &format!("{head}{body}	dc.w .done-Base\n"),
            &Options::default(),
        )
        .expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let err = format!(
            "{:?}",
            sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
                .expect_err("a macro-body `.`-local label must not be visible to the caller")
        );
        assert!(
            err.contains("Base.done"),
            "expected `Base.done` to dangle, got {err}"
        );
    }

    /// A body that does NOT define the name reaches the CALLER's label — this is
    /// the half of the rule that makes a caller-scope reference from inside a
    /// macro work at all.
    ///
    /// ```text
    ///    9/    1000 : (MACRO)                mref
    ///    9/    1000 : 6704                        beq.s   .tgt
    ///    9/    1002 : 4E71                        nop
    ///   10/    1004 : 4E71                   nop
    ///   11/    1006 :                     .tgt:
    ///
    ///    Base.tgt :                    1006 C |
    /// ```
    #[test]
    fn a_macro_body_reaches_a_dot_local_label_only_the_caller_defines() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mref macro\n",
            "	beq.s .tgt\n",
            "	nop\n",
            "	endm\n",
            "Base:\n",
            "	mref\n",
            "	nop\n",
            ".tgt:\n",
            "	nop\n",
        );
        assert_eq!(
            linked_image(src),
            vec![0x67, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71]
        );
    }

    /// An expansion owns a name it defines for the WHOLE expansion, so a forward
    /// branch to it cannot fall through to a same-named label in the caller.
    ///
    /// The caller defines `.tgt` at 0 and the body defines its own at 6; the
    /// branch is `6704`, six bytes forward to the body's:
    ///
    /// ```text
    ///   12/       0 : (MACRO)                mown
    ///   12/       0 : 6704                        beq.s   .tgt
    ///   12/       2 : 4E71                        nop
    ///   12/       4 : 4E71                        nop
    ///   12/       6 :                     .tgt:
    ///   13/       6 : 0008                   dc.w    Later
    /// ```
    ///
    /// asl reaches that row on a two-pass assembly. On a ONE-pass assembly of
    /// the same construction — the identical file with the `dc.w Later` forward
    /// reference removed — asl instead emits `67FE`, the CALLER's label, because
    /// its lookup is order-dependent and the body's own definition had not been
    /// reached yet. Scoping the name to the expansion for the whole expansion is
    /// what removes that difference; see [`Asm::dot_scope`].
    #[test]
    fn an_expansion_owns_a_dot_local_it_defines_for_the_whole_expansion() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mown macro\n",
            "	beq.s .tgt\n",
            "	nop\n",
            "	nop\n",
            ".tgt:\n",
            "	endm\n",
            "Base:\n",
            ".tgt:\n",
            "	mown\n",
            "	dc.w Later\n",
            "Later:\n",
        );
        assert_eq!(
            linked_image(src),
            vec![0x67, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x00, 0x08]
        );
    }

    /// A colon label is a label wherever it sits. AS's column rule only decides
    /// the COLON-LESS head, so a macro body may indent its `.`-locals and still
    /// own them — aeon's `assert` macro writes `\t.skip:` that way, and the
    /// engine's whole assert family branches to it.
    ///
    /// ```text
    ///   11/       0 :                     .skip:
    ///   12/       0 : (MACRO)                mind
    ///   12/       0 : 6704                        beq.s   .skip
    ///   12/       2 : 4E71                        nop
    ///   12/       4 : 4E71                        nop
    ///   12/       6 :                             .skip:
    ///   13/       6 : 0008                   dc.w    Later
    /// ```
    ///
    /// `6704` is the body's own, four bytes forward, with the caller's `.skip`
    /// sitting at zero.
    #[test]
    fn an_indented_colon_label_is_still_the_expansions_own() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mind macro\n",
            "	beq.s .skip\n",
            "	nop\n",
            "	nop\n",
            "	.skip:\n",
            "	endm\n",
            "Base:\n",
            ".skip:\n",
            "	mind\n",
            "	dc.w Later\n",
            "Later:\n",
        );
        assert_eq!(
            linked_image(src),
            vec![0x67, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x00, 0x08]
        );
    }

    /// The backward direction of the same rule: a reference AFTER the body's own
    /// definition binds the body's, with the caller's `.tgt` sitting two bytes
    /// earlier.
    ///
    /// ```text
    ///   13/    1002 : (MACRO)                mback
    ///   13/    1002 : 4E71                        nop
    ///   13/    1004 :                     .tgt:
    ///   13/    1004 : 4E71                        nop
    ///   13/    1006 : 67FC                        beq.s   .tgt
    /// ```
    #[test]
    fn a_backward_reference_binds_the_expansions_own_label() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mback macro\n",
            "	nop\n",
            ".tgt:\n",
            "	nop\n",
            "	beq.s .tgt\n",
            "	endm\n",
            "Base:\n",
            ".tgt:\n",
            "	nop\n",
            "	mback\n",
        );
        assert_eq!(
            linked_image(src),
            vec![0x4E, 0x71, 0x4E, 0x71, 0x4E, 0x71, 0x67, 0xFC]
        );
    }

    /// The live corpus shape: a table macro seeds counters in the caller's
    /// scope, and separate entry expansions read and reassign them.
    ///
    /// ```text
    ///   14/       0 : (MACRO)                zot     4
    ///   14/       0 : =$0                  .tab    :=      *
    ///   14/       0 : =$4                  .cnt    :=      4
    ///   15/       0 : (MACRO)                zte     $11
    ///   15/       0 : 04                          dc.b    .cnt
    ///   15/       1 : 11                          dc.b    $11
    ///   15/       2 : =$5                  .cnt    :=      .cnt+1
    ///   16/       2 : (MACRO)                zte     $22
    ///   16/       2 : 05                          dc.b    .cnt
    ///   16/       3 : 22                          dc.b    $22
    ///   16/       4 : =$6                  .cnt    :=      .cnt+1
    ///   17/       4 : 06                     dc.b    .cnt
    ///   18/       5 : 06                     dc.b    Table.cnt
    ///   19/       6 : 0000                   dc.w    .tab
    /// ```
    #[test]
    fn a_table_macros_counters_carry_across_separate_entry_expansions() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zot macro len\n",
            ".tab := *\n",
            ".cnt := len\n",
            "	endm\n",
            "zte macro v\n",
            "	dc.b .cnt\n",
            "	dc.b v\n",
            ".cnt := .cnt+1\n",
            "	endm\n",
            "Table:\n",
            "	zot 4\n",
            "	zte $11\n",
            "	zte $22\n",
            "	dc.b .cnt\n",
            "	dc.b Table.cnt\n",
            "	dc.w .tab\n",
        );
        assert_eq!(
            image(src),
            vec![0x04, 0x11, 0x05, 0x22, 0x06, 0x06, 0x00, 0x00]
        );
    }

    /// And with `shift` driving a recursive expansion — the shape
    /// `zoneTableEntry` is: each recursion is a NEW expansion whose caller is
    /// the previous expansion, and the counter still lands one scope outside the
    /// whole nest.
    ///
    /// ```text
    ///   15/       0 : (MACRO)                zte     $11,$22,$33
    ///   15/       0 : 00                              dc.b        .cnt
    ///   15/       1 : 11                              dc.b        $11
    ///   15/       2 : =$1                  .cnt    :=      .cnt+1
    ///   15/       2 :  (MACRO-2)                       zte $22,$33
    ///   15/       2 : 01                              dc.b        .cnt
    ///   15/       3 : 22                              dc.b        $22
    ///   15/       4 : =$2                  .cnt    :=      .cnt+1
    ///   15/       4 :   (MACRO-3)                      zte $33
    ///   15/       4 : 02                              dc.b        .cnt
    ///   15/       5 : 33                              dc.b        $33
    ///   15/       6 : =$3                  .cnt    :=      .cnt+1
    ///   16/       6 : 03                     dc.b    .cnt
    /// ```
    #[test]
    fn a_recursive_shift_macro_accumulates_in_the_scope_outside_the_nest() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "zte macro value\n",
            "	if \"value\"<>\"\"\n",
            "	dc.b .cnt\n",
            "	dc.b value\n",
            ".cnt := .cnt+1\n",
            "	shift\n",
            "	zte ALLARGS\n",
            "	endif\n",
            "	endm\n",
            "Table:\n",
            ".cnt := 0\n",
            "	zte $11,$22,$33\n",
            "	dc.b .cnt\n",
        );
        assert_eq!(
            image(src),
            vec![0x00, 0x11, 0x01, 0x22, 0x02, 0x33, 0x03]
        );
    }

    /// The discriminating case, and the one place this front-end deliberately
    /// refuses where asl assembles.
    ///
    /// The body defines `.done` inside a conditional arm and branches to it from
    /// outside. A rule that looks in the expansion and falls back when it misses
    /// therefore lands somewhere that depends on an ARGUMENT — and asl, which is
    /// such a rule, silently does. Same file, same written branch, `mcond 0` then
    /// `mcond 1`, both two-pass:
    ///
    /// ```text
    ///   14/       2 :                     .done:
    ///   15/       2 : (MACRO)                mcond   0
    ///   15/       2 : 67FE                        beq.s   .done
    ///   15/       6 : =>FALSE                      if 0
    ///   15/       6 :                     .done:
    ///
    ///   14/       2 :                     .done:
    ///   15/       2 : (MACRO)                mcond   1
    ///   15/       2 : 6704                        beq.s   .done
    ///   15/       6 : =>TRUE                       if 1
    ///   15/       8 :                     .done:
    /// ```
    ///
    /// `67FE` is two bytes BACKWARD, to `Base.done`; `6704` is four forward, to
    /// the body's own. One written branch, two destinations, no diagnostic.
    ///
    /// The name is the macro's here, because the macro's body declares it — so
    /// the `mcond 0` reference stays inside the expansion and dangles loudly
    /// instead. That is a divergence and it is on purpose: nothing in the Sonic 2
    /// disassembly or in aeon writes this shape, and the alternative reading is a
    /// branch to an address the author did not write. Should a real consumer ever
    /// need asl's answer, the change is to narrow what the body-label scan claims
    /// — never to add a fall-back, which brings the order-dependence back with it.
    #[test]
    fn a_label_declared_only_in_an_untaken_arm_stays_the_expansions_and_dangles() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mcond macro want\n",
            "	beq.s .done\n",
            "	nop\n",
            "	if want\n",
            "	nop\n",
            ".done:\n",
            "	endif\n",
            "	endm\n",
            "Base:\n",
            "	nop\n",
            ".done:\n",
            "	mcond 0\n",
            "	dc.w Later\n",
            "Later:\n",
        );
        let m = run(src, &Options::default()).expect("assemble");
        let resolved = sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
            .expect("resolve_layout");
        let err = format!(
            "{:?}",
            sigil_link::link(&resolved, &sigil_ir::SymbolTable::new())
                .expect_err("the body's own `.done` must not fall through to the caller's")
        );
        assert!(
            !err.contains("Base.done"),
            "the reference must stay inside the expansion, not reach `Base.done`: {err}"
        );
        // The taken arm is the half both readings agree on, and it is asl's
        // second row above: `4E71 6704 4E71 4E71 000A`.
        let taken = src.replace("	mcond 0\n", "	mcond 1\n");
        assert_eq!(
            linked_image(&taken),
            vec![0x4E, 0x71, 0x67, 0x04, 0x4E, 0x71, 0x4E, 0x71, 0x00, 0x0A]
        );
    }

    /// The expansion scope's name is deliberately unspellable — a leading space
    /// and a `#`, so no source label can alias it — which also means it cannot
    /// survive being PASTED into body text. A `.`-local argument passed from
    /// inside an expansion is qualified against that name, and if the callee then
    /// puts it back into a line through `ALLARGS`, the line reads
    /// `dc.w  macro#1.val` and its second token is the `macro` keyword.
    ///
    /// asl has no such limit — the argument names the caller expansion's private
    /// label and the value is its address:
    ///
    /// ```text
    ///   15/       0 : (MACRO)                outer
    ///   15/       2 :                     .val:
    ///   15/       4 :  (MACRO-2)                   sp      1,.val
    ///   15/       4 : 0002                        dc.w    .val
    /// ```
    ///
    /// What is pinned here is only that the shape is LOUD. It reached
    /// `capture_macro` as a definition with nothing after it, and a body slice
    /// running backwards is a panic, which is the one outcome a front-end may not
    /// have.
    #[test]
    fn a_pasted_expansion_scope_name_is_refused_rather_than_panicking() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "sp macro p1\n",
            "	shift\n",
            "	dc.w ALLARGS\n",
            "	endm\n",
            "outer macro\n",
            "	nop\n",
            ".val:\n",
            "	nop\n",
            "	sp 1,.val\n",
            "	endm\n",
            "Base:\n",
            "	outer\n",
        );
        let diags = run(src, &Options::default()).expect_err("must diagnose, not panic");
        assert!(
            diags.iter().any(|d| d.message.contains("has no `endm`")),
            "expected the unclosed-definition refusal, got {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// A SURPLUS positional argument — one the parameter list could not hold —
    /// is qualified exactly like a bound one, so a bare `.`-local passed past
    /// the last parameter and read back through `ALLARGS` still names the
    /// caller's symbol.
    ///
    /// ```text
    ///   10/       0 : (MACRO)                sp      1,.val
    ///   10/       0 :                             shift
    ///   10/       0 : 5A                          dc.b    .val
    ///
    ///    Base.val :                      5A - |
    /// ```
    ///
    /// The sharp version is a callee whose own body declares the same name. The
    /// argument still means the CALLER's label — asl evaluates it in the caller's
    /// context — so nothing the callee's body says can be allowed to reinterpret
    /// it, and the only place that can be settled is where the argument is bound:
    ///
    /// ```text
    ///   12/       2 :                     .val:
    ///   14/       4 : (MACRO)                sp      1,.val
    ///   14/       4 :                             shift
    ///   14/       4 : 0002                        dc.w    .val
    ///   14/       6 : 4E71                        nop
    ///   14/       8 :                     .val:
    /// ```
    ///
    /// `0002` is the caller's, two bytes in, not the callee's at eight.
    #[test]
    fn a_surplus_positional_dot_local_argument_is_qualified_like_a_bound_one() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "sp macro p1\n",
            "	shift\n",
            "	dc.b ALLARGS\n",
            "	endm\n",
            "Base:\n",
            ".val := $5A\n",
            "	sp 1,.val\n",
        );
        assert_eq!(image(src), vec![0x5A]);

        let shadowed = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "sp macro p1\n",
            "	shift\n",
            "	dc.w ALLARGS\n",
            "	nop\n",
            ".val:\n",
            "	endm\n",
            "Base:\n",
            "	nop\n",
            ".val:\n",
            "	nop\n",
            "	sp 1,.val\n",
        );
        assert_eq!(
            linked_image(shadowed),
            vec![0x4E, 0x71, 0x4E, 0x71, 0x00, 0x02, 0x4E, 0x71]
        );
    }

    // ---------------------------------------------------------------------
    // s2.macrosetup.asm: the three largest AS-frontend sites, and the branch
    // selection underneath them that emitted no diagnostic at all.
    //
    // Every expected value below is an `asl -L` listing row, AS V1.42 Beta
    // Bld 212 run with the Sonic 2 build's own flags minus the two that only
    // redirect output: `asl -xx -n -q -A -L -U -i .`. `-U` (case-sensitive) is
    // on every one of them.
    //
    // `linked_image`, not `image`: an undefined `MOMCPU`/`TRUE` survives the
    // front end as a deferred fixup and is refused by the LINKER, so a
    // front-end-only assertion about them would be vacuous.
    // ---------------------------------------------------------------------

    /// `MOMCPU` is the selected CPU as an integer, and `TRUE`/`FALSE` are 1
    /// and 0. asl:
    ///
    /// ```text
    ///        2/       0 : 0006 8000                   dc.l MOMCPU        ; cpu 68000
    ///        2/       0 : 80 00                       dw MOMCPU          ; cpu z80
    ///        2/       0 : 0100                        dc.b TRUE,FALSE
    /// ```
    ///
    /// A builtin outranks the symbol table: asl refuses `TRUE = 7` and
    /// `MOMCPU = 9` (`error #2035: variables cannot be redefined as
    /// constants`) and keeps reporting 1 and `$68000`.
    #[test]
    fn momcpu_and_true_false_are_builtin_values() {
        let m68k = "	cpu 68000\n	padding off\n	phase 0\n";
        assert_eq!(
            linked_image(&format!("{m68k}	dc.l MOMCPU\n")),
            vec![0x00, 0x06, 0x80, 0x00]
        );
        assert_eq!(
            linked_image("	cpu z80\n	phase 0\n	dw MOMCPU\n"),
            vec![0x80, 0x00]
        );
        assert_eq!(
            linked_image(&format!("{m68k}	dc.b TRUE,FALSE\n")),
            vec![0x01, 0x00]
        );
    }

    /// The silent half. `notZ80(MOMCPU)` and `TRUE` decide which arm of
    /// `s2.macrosetup.asm`'s `org`/`cnop`/`align`/`even`/`ds` is assembled;
    /// undefined, both read FALSE and the WRONG arm emits bytes with nothing
    /// said about it. asl:
    ///
    /// ```text
    ///        3/       0 : =>TRUE                      if notZ80(MOMCPU)
    ///        4/       0 : 11                                  dc.b $11
    ///        5/       1 : =>FALSE                     else
    ///        8/       1 : =>TRUE                      if TRUE
    ///        9/       1 : 33                                  dc.b $33
    /// ```
    #[test]
    fn momcpu_and_true_select_the_arm_asl_selects() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "notZ80 function cpu,(cpu<>128)&&(cpu<>32988)\n",
            "	if notZ80(MOMCPU)\n		dc.b $11\n	else\n		dc.b $22\n	endif\n",
            "	if TRUE\n		dc.b $33\n	else\n		dc.b $44\n	endif\n",
        );
        assert_eq!(linked_image(src), vec![0x11, 0x33]);
    }

    /// `s2.macrosetup.asm:52`'s `even` end to end, which is what the two arms
    /// above are really deciding. asl pads to the even address through the
    /// 68000 arm and never reaches the Z80 arm's `$`:
    ///
    /// ```text
    ///       15/       1 : =>TRUE                       if notZ80(MOMCPU)
    ///       15/       1 : =>TRUE                               if (*)&1
    ///       15/       1 : 00                                          dc.b 0
    ///       15/       2 : =>FALSE                      else
    ///       15/       2 :                                     if ($)&1
    /// ```
    #[test]
    fn even_macro_takes_the_68000_arm_and_pads() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "notZ80 function cpu,(cpu<>128)&&(cpu<>32988)\n",
            "myeven macro\n",
            "	if notZ80(MOMCPU)\n		if (*)&1\n			dc.b 0\n		endif\n",
            "	else\n		if ($)&1\n			db 0\n		endif\n	endif\n",
            "	endm\n",
            "	dc.b 1\n	myeven\n	dc.b $22\n",
        );
        assert_eq!(linked_image(src), vec![0x01, 0x00, 0x22]);
    }

    /// `!name` resolves against the BUILTIN table only. The corpus's own `ds`
    /// macro (`s2.macrosetup.asm:66`) is defined in terms of the builtin it
    /// shadows, so a `!` that merely strips and re-dispatches re-enters the
    /// macro forever. asl expands it exactly once and reserves four bytes:
    ///
    /// ```text
    ///        4/     100 :                     ds macro
    ///        5/     100 :                             !ds.ATTRIBUTE ALLARGS
    ///        6/     100 :                             endm
    ///       10/     100 : 11                          dc.b    $11
    ///       10/     101 : (MACRO)                     ds.b    4
    ///       10/     101 :                             !ds.b 4
    ///       11/     105 : 33                          dc.b    $33
    /// ```
    ///
    /// `$101 → $105` is the reservation, and `dc.b $33` at `$105` is the proof
    /// it happened once. The section is address-only after the `ds`, so the
    /// assertion is on the LABEL the reservation places, not on image bytes —
    /// see `a_reservation_fills_like_p2bin_and_trims_like_p2bin` for what the
    /// image does around a reservation.
    #[test]
    fn bang_forces_the_builtin_past_a_macro_of_that_name() {
        let head = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "ds macro\n	!ds.ATTRIBUTE ALLARGS\n	endm\n",
        );
        // asl's own widths for the three suffixes, read off the same probe:
        // `ds.b 4` spans 4 bytes, `ds.w 3` spans 6, `ds.l 2` spans 8.
        for (line, span) in [("	ds.b 4\n", 4), ("	ds.w 3\n", 6), ("	ds.l 2\n", 8)] {
            let src = format!("{head}	dc.b $11\n{line}After:\n");
            assert_eq!(
                section_label(&src, "After"),
                Some(1 + span),
                "the `!` line must reach the builtin once, not re-enter `ds`: {line:?}"
            );
        }
    }

    /// `ds` reserves address space, and asl reserves by leaving a GAP in the
    /// object file which `p2bin` then FILLS for anything that follows it in the
    /// image. Three shapes, all against asl 1.42 + p2bin on the same source with
    /// a `ds` macro shadowing the builtin (`!ds.ATTRIBUTE ALLARGS`):
    ///
    /// ```text
    ///   bytes after it:  11 00 00 00 00 22 00 00 00 00 00 00 33 00 00 00 00 00 00 00 00 44 00 00 00 16
    ///   trailing:        11 22
    ///   phased RAM:      11 00 00 00 00 00 00 00 00 22
    /// ```
    ///
    /// The first is the one that used to diverge: the reservation placed no byte
    /// AND left the write cursor alone, so everything after it packed short of
    /// its own address — a 20-byte-short image at exit 0 with no diagnostic,
    /// while the trailing label still resolved to $16. The cursor now advances
    /// and the image grows only where something writes, which is both halves of
    /// p2bin's rule at once: the gap fills, a trailing reservation is trimmed,
    /// and a section that is nothing but reservations (Aeon's phased `$FFFF….`
    /// RAM regions) still places no byte.
    #[test]
    fn a_reservation_fills_like_p2bin_and_trims_like_p2bin() {
        let head = concat!(
            "\tcpu 68000\n\tpadding off\n\torg 0\n",
            "ds macro\n\t!ds.ATTRIBUTE ALLARGS\n\tendm\n",
        );
        // Bytes after a reservation: the gap fills, and the trailing `dc.l Here`
        // reads back the same $16 the label has.
        assert_eq!(
            linked_image(&format!(
                "{head}\tdc.b $11\n\tds.b 4\n\tdc.b $22\n\tds.w 3\n\tdc.b $33\n\
                 \tds.l 2\n\tdc.b $44\nHere:\n\tdc.l Here\n"
            )),
            vec![
                0x11, 0, 0, 0, 0, 0x22, 0, 0, 0, 0, 0, 0, 0x33, 0, 0, 0, 0, 0, 0, 0, 0, 0x44,
                0x00, 0x00, 0x00, 0x16,
            ]
        );
        // Trailing reservation: nothing writes past it, so p2bin trims it.
        assert_eq!(
            linked_image(&format!("{head}\tdc.b $11\n\tdc.b $22\n\tds.b 4\nTail:\n")),
            vec![0x11, 0x22]
        );
        // A phased RAM block between two ROM bytes: the reservations place no
        // image byte of their own, and `flatten`'s inter-section gap fill stands
        // in for p2bin's.
        assert_eq!(
            linked_image(concat!(
                "\tcpu 68000\n\tpadding off\n\torg 0\n",
                "\tdc.b $11\n",
                "\tphase $FFFF0000\n",
                "A:\tds.b 4\nB:\tds.w 2\n",
                "\tdephase\n",
                "\tdc.b $22\n",
            )),
            vec![0x11, 0, 0, 0, 0, 0, 0, 0, 0, 0x22]
        );
    }

    /// The other half of the escape, and the reason it exists: a user macro
    /// BEATS the builtin of the same name, for a directive and for a mnemonic
    /// alike. asl, with `org` and `move` macros in scope:
    ///
    /// ```text
    ///       10/     100 : 11                          dc.b    $11
    ///       11/     101 : (MACRO)                     org     $200
    ///       11/     101 : EE                          dc.b    $EE
    ///       12/     102 : 22                          dc.b    $22
    ///       13/     103 : (MACRO)                     move.w  #1,d0
    ///       13/     103 : DD                          dc.b    $DD
    ///       14/     104 : 44                          dc.b    $44
    ///       15/     300 :                             !org    $300
    ///       16/     300 : 55                          dc.b    $55
    /// ```
    ///
    /// `org $200` advances by the macro's single byte instead of seeking to
    /// $200, and only the `!` line seeks. Getting this backwards is silent:
    /// `s2.macrosetup.asm` redefines `org` (forward-only, padding-counting) and
    /// `align` (as `cnop 0,n`, through that same `org`), so a builtin that wins
    /// runs a different program and says nothing.
    #[test]
    fn a_user_macro_shadows_the_builtin_of_the_same_name() {
        let head = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "org macro address\n	dc.b $EE\n	endm\n",
            "move macro\n	dc.b $DD\n	endm\n",
        );
        assert_eq!(
            linked_image(&format!("{head}	dc.b $11\n	org $200\n	dc.b $22\n")),
            vec![0x11, 0xEE, 0x22],
            "the `org` MACRO runs; the builtin would have sought to $200"
        );
        assert_eq!(
            linked_image(&format!("{head}	dc.b $11\n	move.w #1,d0\n	dc.b $44\n")),
            vec![0x11, 0xDD, 0x44],
            "a macro shadows a MNEMONIC too, `.w` attribute and all"
        );
        // And the `!` line is the one that reaches the builtin: seeking to $300
        // from $101 leaves 511 bytes of gap-fill ahead of the `$55`.
        let forced = linked_image(&format!("{head}	dc.b $11\n	!org $300\n	dc.b $55\n"));
        assert_eq!(forced.len(), 0x301);
        assert_eq!(forced[0], 0x11);
        assert_eq!(forced[0x300], 0x55);
    }

    /// The bypass is not a fallback. asl resolves the name after `!` in the
    /// builtin table and NOWHERE else, so a name that is only a user macro is
    /// an error rather than an invocation — and so is a name that is nothing:
    ///
    /// ```text
    ///       15/     10F : (MACRO)                     mym
    ///       15/     10F : AA                          dc.b    $AA
    /// > > > p3.asm(16):3: error #1200: unknown instruction
    /// > > > MYM
    /// > > >  !mym                     ; bang on a user macro that is not a builtin
    ///       16/     110 :                             !mym
    ///       17/     110 : 44                          dc.b    $44
    /// ```
    ///
    /// The unsuffixed `mym` on line 15 emits `AA`; the `!mym` on line 16 emits
    /// nothing and is diagnosed.
    #[test]
    fn bang_never_falls_back_to_a_user_macro() {
        let head = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mym macro\n	dc.b $AA\n	endm\n",
        );
        assert_eq!(
            linked_image(&format!("{head}	mym\n	dc.b $44\n")),
            vec![0xAA, 0x44],
            "without the `!` the macro is what runs"
        );
        assert!(
            run(&format!("{head}	!mym\n	dc.b $44\n"), &Options::default()).is_err(),
            "`!mym` names no builtin: asl says #1200, and so must this"
        );
        assert!(
            run(
                "	cpu 68000\n	padding off\n	phase 0\n	!frobnicate 1\n",
                &Options::default()
            )
            .is_err(),
            "`!` on a name that is nothing at all is the same #1200"
        );
    }

    /// The escape composes with a colon label and with a shadowed non-`ds`
    /// builtin, and the `!` must be GLUED to the name — a space makes the `!`
    /// itself the mnemonic, which is nothing:
    ///
    /// ```text
    ///        8/     101 :                     Lbl:    !ds.b   3
    ///        9/     104 : 22                          dc.b    $22
    /// > > > p5.asm(10):3: error #1200: unknown instruction
    /// > > >  ! ds.b   3               ; bang separated by a space
    ///       12/     106 : (MACRO)                     align   4
    ///       12/     106 : EE                          dc.b    $EE
    ///       13/     107 : 44                          dc.b    $44
    ///       14/     108 :                             !align  4
    ///       15/     108 : 55                          dc.b    $55
    /// ```
    #[test]
    fn bang_composes_with_a_label_and_binds_tightly() {
        let head = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "align macro\n	dc.b $EE\n	endm\n",
        );
        // Line 12 vs 14: the bare name runs the macro (`EE`), the `!` name runs
        // the builtin. `align 4` at an offset already a multiple of 4 is a no-op
        // there, so the builtin contributes nothing and `$55` follows `$44`.
        assert_eq!(
            linked_image(&format!("{head}	dc.b $44\n	!align 4\n	dc.b $55\n")),
            vec![0x44, 0x00, 0x00, 0x00, 0x55],
            "`!align` must reach the builtin, not emit the macro's $EE"
        );
        assert_eq!(
            linked_image(&format!("{head}	dc.b $44\n	align 4\n	dc.b $55\n")),
            vec![0x44, 0xEE, 0x55],
            "without the `!` the macro is still what runs"
        );
        // A colon label on the same line as the escape.
        let labelled = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "ds macro\n	!ds.ATTRIBUTE ALLARGS\n	endm\n",
            "	dc.b $11\n",
            "Lbl:	!ds.b 3\n",
            "After:\n",
        );
        assert_eq!(section_label(labelled, "Lbl"), Some(1));
        assert_eq!(section_label(labelled, "After"), Some(4));
        assert!(
            run(
                "	cpu 68000\n	padding off\n	phase 0\n	! align 4\n",
                &Options::default()
            )
            .is_err(),
            "`! name` with a space is #1200 on an empty mnemonic, not the builtin"
        );
    }

    /// A line whose OPERAND does not lex still has a head, and block nesting
    /// is decided by the head. asl counts the nested `if`/`endif` inside a
    /// branch it never evaluates — `[4]` closes line 4, `[3]` closes line 3 —
    /// so `dc.b $55` stays inside the false block and the file is two bytes:
    ///
    /// ```text
    ///        3/       1 : =>FALSE                     if 0
    ///        4/       1 :                                     if ($)&1
    ///        6/       1 : [4]                                 endif
    ///        7/       1 :                                     dc.b $55
    ///        8/       1 : [3]                         endif
    ///        9/       1 : 22                          dc.b $22
    /// ```
    ///
    /// Lose the head and the inner `endif` pops the OUTER frame instead: the
    /// conditional ends early, `dc.b $55` escapes into the emitted image, and
    /// the real `endif` is left over as a spurious mnemonic diagnostic
    /// pointing at a line that is not the fault.
    #[test]
    fn unlexable_operand_does_not_break_conditional_nesting() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	dc.b $11\n",
            "	if 0\n		if ($)&1\n			dc.b $44\n		endif\n		dc.b $55\n	endif\n",
            "	dc.b $22\n",
        );
        assert_eq!(linked_image(src), vec![0x11, 0x22]);
    }

    /// The other side of that recovery: a head recovered from a partly-lexed
    /// line may be COUNTED, but its truncated arguments must never be
    /// EVALUATED as if they were what the source wrote. The arm is declined
    /// and the lex diagnostic is what the caller hears — once, at the line
    /// that actually carries the fault.
    #[test]
    fn arm_head_that_does_not_lex_is_loud_not_guessed() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	if ($)&1\n	dc.b $11\n	endif\n	dc.b $22\n",
        );
        let diags = run(src, &Options::default()).expect_err("must not assemble silently");
        let msgs: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(msgs, vec!["`$` with no hex digits"], "got {msgs:?}");
    }

    /// Parentheses around a string expression are transparent. asl:
    ///
    /// ```text
    ///        2/       0 : 03                          dc.b strlen(("abc"))
    ///        3/       1 : 03                          dc.b strlen((("abc")))
    ///        4/       2 : 02                          dc.b strstr(("hello"),("ll"))
    ///        5/       3 : 02                          dc.b strlen(substr(("hello"),0,2))
    ///        6/       4 : 04                          dc.b strlen(lowstring(("ABCD")))
    ///        7/       5 : 00                          dc.b (("he"))<>"he"
    ///        8/       6 : 01                          dc.b ("he")<>("hf")
    ///        9/       7 : 02                          dc.b strlen(( "ab" ))
    /// ```
    #[test]
    fn parentheses_around_a_string_expression_are_transparent() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	dc.b strlen((\"abc\"))\n",
            "	dc.b strlen(((\"abc\")))\n",
            "	dc.b strstr((\"hello\"),(\"ll\"))\n",
            "	dc.b strlen(substr((\"hello\"),0,2))\n",
            "	dc.b strlen(lowstring((\"ABCD\")))\n",
            "	dc.b ((\"he\"))<>\"he\"\n",
            "	dc.b (\"he\")<>(\"hf\")\n",
            "	dc.b strlen(( \"ab\" ))\n",
        );
        assert_eq!(linked_image(src), vec![3, 3, 2, 2, 4, 0, 1, 2]);
    }

    /// Which is what makes a user `function` parameter reach the string
    /// builtins at all: an argument is substituted PARENTHESISED, so
    /// `s2.macrosetup.asm:104`'s
    /// `chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)`
    /// hands its own `strlen` a `("0(")`. asl:
    ///
    /// ```text
    ///        5/       0 : 03                          dc.b slen("abc")
    ///        6/       1 : 00                          dc.b chkop("0(a0)","0(")
    ///        7/       2 : 01                          dc.b chkop("d0","0(")
    ///        8/       3 : 02                          dc.b strlen("0(")
    ///        9/       4 : 00                          dc.b sub2("hello",2)<>"he"
    /// ```
    #[test]
    fn function_parameters_reach_the_string_builtins() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)\n",
            "slen function s,strlen(s)\n",
            "sub2 function s,n,substr(s,0,n)\n",
            "	dc.b slen(\"abc\")\n",
            "	dc.b chkop(\"0(a0)\",\"0(\")\n",
            "	dc.b chkop(\"d0\",\"0(\")\n",
            "	dc.b strlen(\"0(\")\n",
            "	dc.b sub2(\"hello\",2)<>\"he\"\n",
        );
        assert_eq!(linked_image(src), vec![3, 0, 1, 2, 0]);
    }

    // ── irp / irpc / ARGCOUNT ───────────────────────────────────────────────
    //
    // Every expectation below is a byte column read off a real `asl` listing,
    // produced by `s1disasm/build_tools/Linux-x86_64/asl` (md5
    // 61e672562465725a8c102288a7da9098 — S1 ships upstream AS, S2 ships the
    // flamewing fork and they are NOT the same build) invoked the way the
    // corpus's own build invokes it: `-xx -n -q -A -L -U -E -i .`.

    /// `irpc` walks characters, `irp` walks top-level comma groups, and the
    /// loop variable is pasted as TEXT — into a string literal included. asl,
    /// probe `p1.asm`:
    ///
    /// ```text
    ///    6/    1000 : 3C41 3E     dc.b "<A>"
    ///    6/    1003 : 3C42 3E     dc.b "<B>"
    ///    6/    1006 : 3C43 3E     dc.b "<C>"
    ///   10/    1009 : 0B          dc.b 11
    ///   10/    100A : 16          dc.b 22
    ///   10/    100B : 21          dc.b 33
    /// ```
    #[test]
    fn irpc_walks_characters_and_irp_walks_comma_groups() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	irpc c,\"ABC\"\n	dc.b \"<c>\"\n	endm\n",
            "	irp v,11,22,33\n	dc.b v\n	endm\n",
        );
        assert_eq!(
            image(src),
            vec![b'<', b'A', b'>', b'<', b'B', b'>', b'<', b'C', b'>', 11, 22, 33]
        );
    }

    /// Both spellings close on `endm` AND on `endr`. asl, probe `p8.asm` case
    /// 8e and `p9.asm` case 9e:
    ///
    /// ```text
    ///   39/    102E : 7B51 7D     dc.b "{Q}"
    ///   39/    1031 : 7B52 7D     dc.b "{R}"
    ///   29/    1031 : 04          dc.b 4
    ///   29/    1032 : 05          dc.b 5
    /// ```
    #[test]
    fn irp_and_irpc_close_on_either_endm_or_endr() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	irpc c,\"QR\"\n	dc.b \"{c}\"\n	endr\n",
            "	irp v,4,5\n	dc.b v\n	endr\n",
        );
        assert_eq!(image(src), vec![b'{', b'Q', b'}', b'{', b'R', b'}', 4, 5]);
    }

    /// **An empty list is ONE EMPTY iteration, not none** — for both spellings.
    /// This is the rule `s2.macrosetup.asm(301)`'s `if ARGCOUNT>0` guard exists
    /// to stop, and the rule S1's `demoinput ,    $8C` lines depend on: `irpc
    /// btn,"buttons"` with an empty `buttons` runs the `switch` once against an
    /// empty character and matches no `case`. asl, probe `p6.asm` case 6a and
    /// `p7.asm` case 7a:
    ///
    /// ```text
    ///    7/    1002 : 11          dc.b $11        ← irp v, (one iteration)
    ///    7/    1002 : 3C3E        dc.b "<>"       ← irpc c,"" (one iteration)
    /// ```
    #[test]
    fn an_empty_list_is_one_empty_iteration_not_zero() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	irp v,\n	dc.b $11\n	endm\n",
            "	irpc c,\"\"\n	dc.b \"<c>\"\n	endm\n",
            "	irp v,,\n	dc.b $22\n	endm\n",
        );
        assert_eq!(image(src), vec![0x11, b'<', b'>', 0x22, 0x22]);
    }

    /// The loop variable obeys the macro-parameter boundary rule and is
    /// CASE-SENSITIVE under `-U`: `"c"` and `_c_` take the value, `xcx` does
    /// not, and `Cv` answers only to its own spelling. asl, probe `p6.asm` case
    /// 6g and `p7.asm` case 7f:
    ///
    /// ```text
    ///   38/    1016 : 4141 7863 785F 415F     dc.b "A", 'A', "xcx", "_A_"
    ///   33/    102A : 3C41 3E3C 6376 3E       dc.b "<A><cv>"
    /// ```
    #[test]
    fn loop_variable_obeys_the_boundary_rule_and_is_case_sensitive() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	irpc c,\"A\"\n	dc.b \"c\", \"xcx\", \"_c_\"\n	endm\n",
            "	irpc Cv,\"B\"\n	dc.b \"<Cv><cv>\"\n	endm\n",
        );
        let mut want: Vec<u8> = b"Axcx_A_".to_vec();
        want.extend_from_slice(b"<B><cv>");
        assert_eq!(image(src), want);
    }

    /// `irp`'s items are the SOURCE TEXT the author wrote — never re-rendered
    /// from tokens, which would print `$FF` as `255`. asl, probe `p8.asm` case
    /// 8b:
    ///
    /// ```text
    ///   16/    100F : 5B31 2B32 5D     dc.b "[1+2]"
    ///   16/    1014 : 5B24 4646 5D     dc.b "[$FF]"
    /// ```
    ///
    /// `irpc`'s operand, by contrast, is EVALUATED: a `set` string resolves, an
    /// integer renders in decimal and is then walked digit by digit (case 8a,
    /// `irpc c,1+2` is one iteration of `3`).
    #[test]
    fn irp_items_are_raw_text_while_irpc_evaluates_its_operand() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	irp v,1+2,$FF\n	dc.b \"[v]\"\n	endm\n",
            "sstr	set \"PQ\"\n",
            "	irpc c,sstr\n	dc.b \"c\"\n	endm\n",
            "	irpc c,1+2\n	dc.b \"<c>\"\n	endm\n",
        );
        let mut want: Vec<u8> = b"[1+2][$FF]".to_vec();
        want.extend_from_slice(b"PQ");
        want.extend_from_slice(b"<3>");
        assert_eq!(image(src), want);
    }

    /// A loop nested inside a macro is substituted ONCE where it is entered and
    /// then replayed, exactly as `rept`/`while` are: a `shift` in the body
    /// advances the frame without changing the body's own text, and the frame
    /// HAS advanced by the line after the loop. asl, probe `p8.asm` cases 8c
    /// and 8d:
    ///
    /// ```text
    ///   25/    101B : 5807        dc.b "X",7
    ///   25/    101D : 5907        dc.b "Y",7
    ///   35/    1021 : 7031 01     dc.b "p1",1
    ///   35/    1024 : 7031 02     dc.b "p1",2
    ///   35/    1027 : 7031 03     dc.b "p1",3
    /// ```
    #[test]
    fn a_macro_nested_loop_substitutes_once_and_a_shift_inside_it_does_not_retext() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "mm	macro pp,qq\n	irpc c,\"pp\"\n	dc.b \"c\",qq\n	endm\n	endm\n",
            "	mm XY,7\n",
            "sh	macro aa\n	irp v,1,2,3\n	dc.b \"aa\",v\n	shift\n	endm\n	endm\n",
            "	sh p1,p2,p3\n",
        );
        assert_eq!(
            image(src),
            vec![b'X', 7, b'Y', 7, b'p', b'1', 1, b'p', b'1', 2, b'p', b'1', 3]
        );
    }

    /// `ARGCOUNT` before any shift is the number of argument groups the call
    /// WROTE, and an empty operand field is 0 rather than one empty group. asl,
    /// probe `p5.asm`:
    ///
    /// ```text
    ///    8/    1002 : 0000        dc.w 0      ← `ac`
    ///   12/    100A : 0002        dc.w 2      ← `ac ,`
    ///   14/    100E : 0002        dc.w 2      ← `ac 1,`
    ///   18/    1016 : 0003        dc.w 3      ← `ac 1,,3`
    /// ```
    #[test]
    fn argcount_counts_written_argument_groups_and_an_empty_field_is_zero() {
        let head = "	cpu 68000\n	padding off\n	phase 0\nac	macro\n	dc.b ARGCOUNT\n	endm\n";
        assert_eq!(image(&format!("{head}	ac\n")), vec![0]);
        assert_eq!(image(&format!("{head}	ac ,\n")), vec![2]);
        assert_eq!(image(&format!("{head}	ac 1,\n")), vec![2]);
        assert_eq!(image(&format!("{head}	ac ,1\n")), vec![2]);
        assert_eq!(image(&format!("{head}	ac 1,,3\n")), vec![3]);
        assert_eq!(image(&format!("{head}	ac 1,2,3\n")), vec![3]);
    }

    /// **After a shift `ARGCOUNT` answers from the PARAMETER list, not the
    /// argument list** — so a one-parameter macro called with three arguments
    /// drops 3 → 0 → -1 → -2 rather than counting its arguments down, and the
    /// decrement stops once `max(parameters, arguments)` shifts have happened.
    /// asl, probes `p3.asm` and `p4.asm`, five `dc.w ARGCOUNT` per row:
    ///
    /// ```text
    ///   one  pp        / one 11,22,33          3, 0, -1, -2, -2
    ///   three q1,q2,q3 / three 11               1, 2,  1,  0,  0
    ///   three q1,q2,q3 / three 11,22,33,44,55   5, 2,  1,  0, -1
    /// ```
    #[test]
    fn argcount_after_a_shift_counts_parameters_down_and_stops_when_exhausted() {
        let body = "	dc.w ARGCOUNT\n	shift\n	dc.w ARGCOUNT\n	shift\n	dc.w ARGCOUNT\n	shift\n	dc.w ARGCOUNT\n	shift\n	dc.w ARGCOUNT\n";
        let head = format!(
            "	cpu 68000\n	padding off\n	phase 0\none	macro pp\n{body}	endm\nthree	macro q1,q2,q3\n{body}	endm\n"
        );
        let w = |v: &[i16]| -> Vec<u8> {
            v.iter().flat_map(|n| (*n as u16).to_be_bytes()).collect()
        };
        assert_eq!(image(&format!("{head}	one 11,22,33\n")), w(&[3, 0, -1, -2, -2]));
        assert_eq!(image(&format!("{head}	one 11\n")), w(&[1, 0, 0, 0, 0]));
        assert_eq!(image(&format!("{head}	one\n")), w(&[0, 0, 0, 0, 0]));
        assert_eq!(image(&format!("{head}	three 11,22,33\n")), w(&[3, 2, 1, 0, 0]));
        assert_eq!(image(&format!("{head}	three 11\n")), w(&[1, 2, 1, 0, 0]));
        assert_eq!(
            image(&format!("{head}	three 11,22,33,44,55\n")),
            w(&[5, 2, 1, 0, -1])
        );
    }

    /// `ARGCOUNT` is a SUBSTITUTION, not a symbol: it pastes its digits into the
    /// body text, folds case, obeys the boundary rule — and YIELDS to a
    /// parameter declared with that name. asl, probe `p9.asm` cases 9a and 9b:
    ///
    /// ```text
    ///    9/    1002 : 315B 325D 2032 ...     dc.b "1[2] 2[xARGCOUNTx] 3[_2_] 4[2] 5[2]"
    ///   15/    1027 : 5B7A 7A5D              dc.b "[zz]"
    /// ```
    #[test]
    fn argcount_substitutes_like_allargs_and_a_parameter_of_that_name_wins() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "ac2	macro pp\n",
            "	dc.b \"1[ARGCOUNT] 2[xARGCOUNTx] 3[_ARGCOUNT_] 4[argcount] 5[ArgCount]\"\n",
            "	endm\n",
            "	ac2 7,8\n",
            "ac3	macro ARGCOUNT\n	dc.b \"[ARGCOUNT]\"\n	endm\n",
            "	ac3 zz\n",
        );
        let mut want: Vec<u8> = b"1[2] 2[xARGCOUNTx] 3[_2_] 4[2] 5[2]".to_vec();
        want.extend_from_slice(b"[zz]");
        assert_eq!(image(src), want);
    }

    /// The whole `jmpTos` chain S2 builds out of these three constructs at once:
    /// a zero-parameter macro relaying `ALLARGS`, a `shift` in the frame above
    /// it, `if ARGCOUNT>0` as the guard, and `irp op,ALLARGS` inside it. The
    /// guard is what stops the empty case from running the loop once over an
    /// empty item and defining a nameless label — so the two calls below must
    /// emit DIFFERENT things, and the empty one must emit nothing from the loop.
    #[test]
    fn argcount_guards_an_irp_over_a_relayed_allargs() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "inner	macro\n	if ARGCOUNT>0\n	irp op,ALLARGS\n	dc.b \"<op>\"\n	endm\n	endif\n	endm\n",
            "outer	macro UseNop\n	shift\n	inner ALLARGS\n	endm\n",
            "top	macro\n	outer 1,ALLARGS\n	endm\n",
            "	dc.b $AA\n",
            "	top zz1,zz2\n",
            "	dc.b $BB\n",
            "	top\n",
            "	dc.b $CC\n",
        );
        let mut want: Vec<u8> = vec![0xAA];
        want.extend_from_slice(b"<zz1><zz2>");
        want.push(0xBB);
        want.push(0xCC);
        assert_eq!(image(src), want);
    }

    /// A head with no comma at all is asl's error #1110 and the body is SKIPPED,
    /// not run (probe `p9.asm` cases 9c/9d) — the block must still be stepped
    /// over as a block, or the lines after it desynchronise.
    #[test]
    fn an_irp_head_without_a_list_diagnoses_and_skips_its_body() {
        let src = concat!(
            "	cpu 68000\n	padding off\n	phase 0\n",
            "	dc.b $AA\n",
            "	irp v\n	dc.b $11\n	endm\n",
            "	irpc c\n	dc.b $22\n	endm\n",
            "	dc.b $BB\n",
        );
        let m = run(src, &Options::default());
        let diags = match &m {
            Ok(_) => panic!("a list-less irp head must diagnose"),
            Err(f) => f.clone(),
        };
        assert_eq!(diags.len(), 2, "one per head: {diags:?}");
        // The body did not run and the lines around it still line up.
        assert!(
            diags.iter().all(|d| d.message.contains("loop variable")),
            "{diags:?}"
        );
    }

    /// An unrecognized INDENTED head under `CPU Z80` is a diagnostic, not a
    /// label. It used to be bound silently: no diagnostic, no bytes, exit 0 —
    /// so an unimplemented Z80 mnemonic shortened the output with nothing to
    /// read. Expectations come from `asl` (S1's binary, upstream AS, md5
    /// `61e672562465725a8c102288a7da9098`, `-U` on every invocation), which
    /// classifies the four shapes below as:
    ///
    /// ```text
    ///     indented  zqp_bogus       error #1200: unknown instruction   ZQP_BOGUS
    ///     column-0  zqp_bogus       exit 0 — it is a label
    ///     indented  zqp_bogus a,b   error #1200: unknown instruction   ZQP_BOGUS
    ///     indented  ldi             exit 0 — a real Z80 instruction
    /// ```
    ///
    /// Row 4 is the one that matters: `ldi` is a Z80 instruction this assembler
    /// does not encode, and before the column rule applied under Z80 it was
    /// eaten in silence. `asl` assembles `nop / ldi / nop` at org 0 to
    /// `00 ED A0 00`; this assembler emitted `00 00` and exited 0.
    #[test]
    fn unrecognized_indented_head_under_z80_is_loud_not_a_label() {
        for head in ["zqp_bogus", "zqp_bogus a,b", "ldi"] {
            let src = format!("	cpu z80\n	padding off\n	phase 0\n	{head}\n	nop\n");
            let diags = match run(&src, &Options::default()) {
                Ok(_) => panic!("`{head}` assembled silently; it must diagnose"),
                Err(d) => d,
            };
            let msgs: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
            let want = if head.starts_with("ldi") {
                "unknown directive or mnemonic `ldi`"
            } else {
                "unknown directive or mnemonic `zqp_bogus`"
            };
            assert_eq!(msgs, vec![want], "head `{head}` gave {msgs:?}");
        }
    }

    /// The other half of AS's column rule, and the reason the fix is a column
    /// test rather than a blanket refusal: a head in COLUMN 0 under `CPU Z80`
    /// really is a label, and `asl` accepts it with exit 0.
    #[test]
    fn unrecognized_column_zero_head_under_z80_is_still_a_label() {
        let src = "	cpu z80\n	padding off\n	phase 0\nzqp_bogus\n	nop\n";
        let m = run(src, &Options::default()).expect("a column-0 head is a label");
        assert!(
            m.sections
                .iter()
                .flat_map(|s| s.labels.iter())
                .any(|l| l.name == "zqp_bogus"),
            "the column-0 head must still bind: {:?}",
            m.sections
                .iter()
                .flat_map(|s| s.labels.iter().map(|l| l.name.clone()))
                .collect::<Vec<_>>()
        );
    }

    /// The silent bind had a second effect beyond the missing bytes, and it is
    /// the one a byte comparison alone would not have found: the phantom label
    /// OPENED A LOCAL-LABEL SCOPE, so every `.local` defined after it was
    /// qualified under the phantom instead of the real enclosing label, and
    /// references from before it went unresolved.
    ///
    /// This is exactly what `s2disasm`'s `.is_psg` and `.voiceptr` did — three
    /// `unresolved symbol` diagnostics that were collateral damage of the 17
    /// eaten `ldi` heads above them, and that disappear once the heads are loud.
    #[test]
    fn an_unrecognized_z80_head_does_not_open_a_local_label_scope() {
        let src = "	cpu z80\n	padding off\n	phase 0\n\
                   Outer:\n	ld	a,(.loc)\n	zqp_bogus\n.loc:\n	nop\n";
        let diags = run(src, &Options::default()).expect_err("the bogus head must diagnose");
        let msgs: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        // The ONLY complaint is the head. `.loc` still belongs to `Outer`, so
        // the reference above it resolves; when the head was bound as a label
        // this read `unresolved symbol `.loc` in operand` instead — and the
        // head itself was never mentioned at all.
        assert_eq!(
            msgs,
            vec!["unknown directive or mnemonic `zqp_bogus`"],
            "got {msgs:?}"
        );
    }

    // ---------------------------------------------------------------
    // `~~` — asl's LOGICAL not.
    //
    // Every expectation below is a byte column read off an `asl` listing,
    // and BOTH shipped builds produce it: the flamewing fork that `s2disasm`
    // ships and the upstream build that `s1disasm` ships agree on every row
    // of the probe, byte column for byte column and error line for error
    // line. Provenance: `Macro Assembler 1.42 Beta [Bld 212]`, run
    // `-xx -n -q -A -L -U -i .`.
    //
    // `~~` is ONE greedy token, not two `~`. Folding it as two bitwise
    // complements cancels (`!!x == x`) and hands back the operand with no
    // diagnostic, which is why this was silent.
    // ---------------------------------------------------------------

    /// The defect in one row: `dc.b ~~0,~~1,~~5` is `01 00 00`, not `00 01 05`.
    #[test]
    fn double_tilde_is_logical_not_not_two_complements() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0,~~1,~~5,~~-1,~~$FF\n"),
            [0x01, 0x00, 0x00, 0x00, 0x00]
        );
    }

    /// A single `~` is untouched by the above: it is still one's complement.
    /// Without this row a "fix" that made `~` logical would also pass.
    #[test]
    fn single_tilde_is_still_bitwise_complement() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.l ~0,~1,~$0F\n"),
            [
                0xFF, 0xFF, 0xFF, 0xFF, // ~0
                0xFF, 0xFF, 0xFF, 0xFE, // ~1
                0xFF, 0xFF, 0xFF, 0xF0, // ~$0F
            ]
        );
    }

    /// Maximal munch, stated as bytes: `~~~x` is `~~` then `~`, so it is the
    /// LOGICAL not of the COMPLEMENT — zero for every operand but `-1`.
    #[test]
    fn triple_tilde_is_logical_not_of_a_complement() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~~0,~~~1,~~~5,~~~-1\n"),
            [0x00, 0x00, 0x00, 0x01]
        );
    }

    /// `~~` binds at the atom tier, tighter than EVERY binary operator —
    /// arithmetic, multiplicative, bitwise and relational alike. Each row is a
    /// separate listing column, because one row cannot separate the tiers.
    #[test]
    fn logical_not_binds_tighter_than_every_binary_operator() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0+1,~~1+1,~~(0+1)\n"),
            [0x02, 0x01, 0x00]
        );
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0*3,~~2*3,-~~0\n"),
            [0x03, 0x00, 0xFF]
        );
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0|2,~~0&3,~~0!1\n"),
            [0x03, 0x01, 0x00]
        );
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0=1,~~1=0\n"),
            [0x01, 0x01]
        );
    }

    /// The operand may be parenthesised, negative, or separated by space —
    /// `~~ 0` is the same token as `~~0`. (`~ -1` is NOT: asl splits it at the
    /// binary minus and reports #1110 on the bare `~`. sigil accepts it; that
    /// divergence is corpus-unreachable and booked, not gated.)
    #[test]
    fn logical_not_accepts_parens_negatives_and_a_following_space() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~ 0,~~ 1,~~(0),~~(5),~~(-1)\n"),
            [0x01, 0x00, 0x01, 0x00, 0x00]
        );
    }

    /// asl's booleans and integers interconvert freely, so `~~` of a
    /// comparison is the comparison negated, and `~~0` compares equal to TRUE.
    #[test]
    fn logical_not_of_a_comparison_negates_it() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~(1=1),~~(1=2),(~~0)=(1=1)\n"),
            [0x00, 0x01, 0x01]
        );
    }

    /// The corpus's composition. `s2.macrosetup.asm(245)` chains three `~~`
    /// through `||`; `s2.sounddriver.asm(3253)` writes `(~~A)&&(~~B)`.
    #[test]
    fn logical_not_composes_with_the_boolean_connectives() {
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0||~~0,~~0||~~1,~~1||~~1\n"),
            [0x01, 0x01, 0x00]
        );
        assert_eq!(
            image("\tcpu 68000\n\tpadding off\n\tdc.b ~~0&&~~0,~~0&&~~1,~~1&&~~1\n"),
            [0x01, 0x00, 0x00]
        );
    }

    /// The reason this is a code-generation defect and not a wrong number:
    /// `if ~~FLAG` is how `s2disasm` writes "if FLAG is off", 96 times over
    /// four files, and every one of those flags is 0. Reading `~~0` as `0`
    /// takes the WRONG ARM of all 96.
    #[test]
    fn if_on_a_logical_not_takes_the_arm_asl_takes() {
        let src = "\tcpu 68000\n\tpadding off\nFLAG = 0\n\
                   \tif ~~FLAG\n\tdc.b $AA\n\telse\n\tdc.b $BB\n\tendif\n";
        assert_eq!(image(src), [0xAA]);
        let src_on = "\tcpu 68000\n\tpadding off\nFLAG = 1\n\
                      \tif ~~FLAG\n\tdc.b $AA\n\telse\n\tdc.b $BB\n\tendif\n";
        assert_eq!(image(src_on), [0xBB]);
    }

    /// `~~` lives inside MACRO BODIES in the corpus (`jmpTosInternal`,
    /// `_btst`), so it has to survive capture and re-rendering as text. A
    /// token that lexes correctly but renders back as two `~` would pass every
    /// row above and still break every real site.
    #[test]
    fn logical_not_survives_a_macro_body_round_trip() {
        let src = "\tcpu 68000\n\tpadding off\n\
                   gate macro flag,yes,no\n\tif ~~flag\n\tdc.b yes\n\telse\n\tdc.b no\n\tendif\n\tendm\n\
                   val macro x\n\tdc.b ~~x,~~~x\n\tendm\n\
                   Zero = 0\nOne = 1\n\
                   \tgate Zero,$AA,$BB\n\tgate One,$CC,$DD\n\tval 0\n\tval 5\n";
        assert_eq!(image(src), [0xAA, 0xDD, 0x01, 0x00, 0x00, 0x00]);
    }

    /// The `~~` token also has to render BACK to `~~` when a macro ARGUMENT
    /// carrying it is substituted as text (`render_tokens`/`punct_str`) —
    /// including into a string literal, where a wrong rendering becomes a
    /// literal byte. `dc.b "[~~0]"` is `5B 7E 7E 30 5D` off the asl listing.
    ///
    /// **This row is language-derived and corpus-UNEXERCISED**: all 96
    /// `s2disasm` sites spell `~~` in a macro BODY (stored as raw text, so it
    /// never round-trips through `punct_str`) or at top level. No corpus
    /// passes `~~` as a macro argument. Without this row the `punct_str` arm
    /// is unreachable from any test — the macro-BODY row above stays green
    /// with that arm broken, which is how the hole was found.
    #[test]
    fn logical_not_renders_back_through_a_macro_argument() {
        let src = "\tcpu 68000\n\tpadding off\n\
                   one macro v\n\tdc.b v\n\tendm\n\
                   all macro\n\tdc.b ALLARGS\n\tendm\n\
                   str macro v\n\tdc.b \"[v]\"\n\tendm\n\
                   \tone ~~0\n\tone ~~1\n\tall ~~0,~~5,~~~0\n\tstr ~~0\n";
        assert_eq!(
            image(src),
            [0x01, 0x00, 0x01, 0x00, 0x00, 0x5B, 0x7E, 0x7E, 0x30, 0x5D]
        );
    }

    /// The instruction-generation half, at the shape that actually bites:
    /// `s2.macrosetup.asm`'s `jmpTosInternal` gates its whole body on
    /// `if ~~removeJmpTos`, and `removeJmpTos` is 0. Reading `~~0` as 0 does
    /// not mis-assemble the jump table — it DELETES it, silently.
    #[test]
    fn the_jmp_table_gate_emits_its_table() {
        let src = "\tcpu 68000\n\tpadding off\n\torg $1000\nremoveJmpTos = 0\n\
                   tbl macro\n\tif ~~removeJmpTos\n\tirp op,ALLARGS\nop label *\n\tjmp (op).l\n\tendm\n\tendif\n\tendm\n\
                   \ttbl A,B\n";
        // Two `jmp (abs).l`: `4EF9` then each entry's own 32-bit address —
        // twelve bytes at `$1000`, and NOTHING before them. `linked_image`
        // flattens from zero, so the table is the image's tail.
        let img = linked_image(src);
        assert_eq!(img.len(), 0x1000 + 12, "the table is 12 bytes at $1000");
        assert_eq!(
            &img[0x1000..],
            [0x4E, 0xF9, 0x00, 0x00, 0x10, 0x00, 0x4E, 0xF9, 0x00, 0x00, 0x10, 0x06]
        );
    }
}

/// The diagnostic for a floating-point value reaching a context that requires
/// an integer — asl's `error #1133: expected integer or string, but got
/// floating point number` (probe `.f1probe/f1.asm(17)`: `dc.l 3.7`).
///
/// AS's expression evaluator is typed, and a float has no integer meaning of
/// its own: the source must say which integer it wants, via `int(...)` (floor)
/// or the corpus's `roundFloatToInteger` (`int(x+0.5)`). Truncating one
/// silently here would be the wrong-bytes class — a program that never says
/// how to round would get a rounding anyway.
const FLOAT_IN_INT_CONTEXT: &str =
    "floating point value where an integer is required (wrap it in `int(...)`)";

/// A front-end-only NUMBER: AS's expression evaluator is TYPED, and the
/// distinction is byte-visible, not cosmetic.
///
/// asl 1.42 Bld 212, probe `.f1probe/f2.asm`, listing columns quoted:
///
/// ```text
///   4/  0 : FFFF FFFD    dc.l INT(-7/2)      ; int/int -> TRUNCATING int div, -3
///  26/ 3C : 0000 025E    dc.l INT(15.39*1024*1024*2/FM_Sample_Rate+0.5)
/// ```
///
/// `INT(-7/2)` is **-3**, not -4: `-7` and `2` are both integers, so `/` is
/// integer division and `INT` then floors an integer (a no-op). Evaluating
/// the same tree in f64 throughout gives `floor(-3.5)` = **-4** — a silently
/// wrong byte, from a program that assembles clean. Type is therefore
/// carried, not erased.
///
/// The float side is IEEE `f64` (binary64), asl-proven rather than assumed:
/// `dc.l INT(1e17+1-1e17)` gives `0000 0000` (probe `f2.asm(24)`). `1e17+1`
/// is not representable in binary64 and rounds back to `1e17`; an 80-bit
/// extended (64-bit mantissa) would represent it exactly and answer 1.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    fn as_f64(self) -> f64 {
        match self {
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }
    /// The integer behind this value, or `None` if it is a float. Backs the
    /// operators AS refuses on floats (`error #1134: expected integer, but
    /// got floating point number` — probe `f3.asm` lines 18-20 for `&`,
    /// `<<` and `!`).
    fn as_i64(self) -> Option<i64> {
        match self {
            Num::Int(i) => Some(i),
            Num::Float(_) => None,
        }
    }
    fn is_float(self) -> bool {
        matches!(self, Num::Float(_))
    }
}

/// Apply one binary operator to two [`Num`]s under AS's type rules.
///
/// `BinOp` comes straight from [`crate::expr::infix_bp`], so this match is the
/// single place the typed evaluator says what each operator MEANS on floats;
/// the arms are exhaustive, so a new operator in that ladder is a compile
/// error here until it is given a meaning.
///
/// Three tiers, all asl-minted (probes `.f1probe/f1.asm`, `f2.asm`, `f3.asm`):
///
/// * **Arithmetic** (`+ - * /`) — float if EITHER side is float, otherwise
///   integer. `dc.l 7/2` = 3 and `dc.l -7/2` = -3 (truncation toward zero, the
///   same `i64::wrapping_div` the integer folder uses), while
///   `dc.l INT(7.0/2)` = 3 through a real 3.5.
/// * **Bit/modulo** (`& | ! << >> #`) — INTEGER ONLY. asl refuses a float
///   operand outright: `dc.l INT(7.5&3)` / `INT(7.5<<1)` / `INT(7.5!3)` each
///   draw `error #1134: expected integer, but got floating point number`.
///   Returning `None` here is what turns that into sigil's own diagnostic
///   rather than a silent truncation.
/// * **Comparison / logical** (`= <> < > <= >= && ||`) — accept floats and
///   yield an INTEGER 0/1, so the result composes into ordinary integer
///   expressions: `dc.l 3.5<4` = `0000 0001`, and `if 3.5>2` takes the
///   true arm.
fn apply_num_binop(op: BinOp, lhs: Num, rhs: Num) -> Option<Num> {
    use BinOp::*;
    let float_math = lhs.is_float() || rhs.is_float();
    Some(match op {
        Add if float_math => Num::Float(lhs.as_f64() + rhs.as_f64()),
        Sub if float_math => Num::Float(lhs.as_f64() - rhs.as_f64()),
        Mul if float_math => Num::Float(lhs.as_f64() * rhs.as_f64()),
        Div if float_math => Num::Float(lhs.as_f64() / rhs.as_f64()),
        Add => Num::Int(lhs.as_i64()?.wrapping_add(rhs.as_i64()?)),
        Sub => Num::Int(lhs.as_i64()?.wrapping_sub(rhs.as_i64()?)),
        Mul => Num::Int(lhs.as_i64()?.wrapping_mul(rhs.as_i64()?)),
        Div => {
            let d = rhs.as_i64()?;
            if d == 0 {
                return None;
            }
            Num::Int(lhs.as_i64()?.wrapping_div(d))
        }
        Mod => {
            let d = rhs.as_i64()?;
            if d == 0 {
                return None;
            }
            Num::Int(lhs.as_i64()?.wrapping_rem(d))
        }
        And => Num::Int(lhs.as_i64()? & rhs.as_i64()?),
        Or => Num::Int(lhs.as_i64()? | rhs.as_i64()?),
        Xor => Num::Int(lhs.as_i64()? ^ rhs.as_i64()?),
        Shl => Num::Int(lhs.as_i64()?.wrapping_shl(rhs.as_i64()? as u32)),
        Shr => Num::Int(lhs.as_i64()?.wrapping_shr(rhs.as_i64()? as u32)),
        Eq => Num::Int((lhs.as_f64() == rhs.as_f64()) as i64),
        Ne => Num::Int((lhs.as_f64() != rhs.as_f64()) as i64),
        Lt => Num::Int((lhs.as_f64() < rhs.as_f64()) as i64),
        Gt => Num::Int((lhs.as_f64() > rhs.as_f64()) as i64),
        Le => Num::Int((lhs.as_f64() <= rhs.as_f64()) as i64),
        Ge => Num::Int((lhs.as_f64() >= rhs.as_f64()) as i64),
        LogAnd => Num::Int((lhs.as_f64() != 0.0 && rhs.as_f64() != 0.0) as i64),
        LogOr => Num::Int((lhs.as_f64() != 0.0 || rhs.as_f64() != 0.0) as i64),
    })
}
