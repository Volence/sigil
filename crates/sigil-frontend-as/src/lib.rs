//! sigil-frontend-as: the quarantined AS-syntax front-end (the byte-exact oracle).
//!
//! Reads Aeon's AS source and lowers it through `sigil_ir::IrStreamer` + the
//! public `sigil_backend_z80::Z80Backend` seam. It never touches the raw ISA
//! codec (`sigil-isa`) nor the linker (`sigil-link`).

mod ast;
mod eval;
mod expand;
mod expr;
mod lexer;
mod operands;
mod parser;
mod state;
mod token;

use std::path::Path;

use sigil_ir::backend::Cpu;
use sigil_ir::Module;
use sigil_span::{Diagnostic, SourceMap};

pub use eval::Assembled;

/// A failed assembly: the diagnostics, plus the [`SourceMap`] their spans resolve
/// against.
///
/// The map is the half a caller cannot reconstruct. `include` splices files the
/// caller never named — hundreds of them for a real program — and each one's spans
/// are offsets into that file, so a bare `Vec<Diagnostic>` cannot be turned back
/// into `file(line)` afterwards. [`SourceMap::label`] does that here.
pub struct Failure {
    /// Every diagnostic the failing pass produced, in the order it raised them.
    pub diags: Vec<Diagnostic>,
    /// The root source and every `include`d file, under the ids the spans carry.
    pub sources: SourceMap,
}

/// The refusal raised when an assembly unit never declares its processor.
///
/// The rule: under AS-compatibility a source that never says which processor it
/// is for is a HARD ERROR, never a silent default of any processor. The scope is
/// the assembly UNIT, not the file — an `include`d file beneath a root that has
/// already declared inherits that declaration and is fine (aeon's
/// `engine/debug/debugger.asm` carries no `cpu` line and never needed one; its
/// root declares `cpu 68000` before including it). What is refused is a unit in
/// which NOTHING declared: not the source, not the caller.
///
/// Refusing rather than warning is the point. A run that reports what it skipped
/// still exits 0, and the failure mode here is silent: the processor decides how
/// `$` lexes, so an undeclared 68000 source assembles clean as a Z80 program and
/// reports nothing.
pub const CPU_UNDECLARED: &str = "no processor declared: this assembly unit never says which \
processor it is for, and sigil will not choose one for it. Declare it on its own line at the top \
of the root source, before any code — `cpu 68000` for a 68000 program, `cpu z80` for a Z80 one. \
An `include`d file needs no line of its own: the declaration is the unit's, and the root's covers \
it. A caller driving this front-end directly declares it by setting `Options::initial_cpu` \
instead.";

/// Every processor spelling a `cpu` directive may name, and the target each one
/// selects. The single source of truth: [`cpu_for_spelling`] resolves against
/// it and [`unsupported_cpu`] lists it back to the reader, so the refusal can
/// never advertise a spelling the directive does not accept.
///
/// **A spelling earns a row here only when it names an instruction set sigil
/// already encodes.** Two kinds qualify:
///
/// - *The same instruction set in different packaging.* `68008` is a 68000 core
///   behind an 8-bit data bus — same instructions, so the same target.
/// - *A superset whose extra instructions this front end refuses by name.*
///   `z80undoc` is the Z80 with its undocumented instructions enabled. Sigil
///   encodes the documented subset and rejects the rest at the instruction: an
///   undocumented mnemonic is `unknown directive or mnemonic`, and an
///   undocumented operand (`ld a,ixl`) is refused where it appears. Accepting
///   the spelling therefore widens *where the refusal is reported*, never what
///   assembles.
///
/// A spelling naming an instruction set sigil does not encode gets no row.
/// `68020`, `z180` and `gbz80` add instructions that would be reported as
/// unknown heads on a target they do not belong to, and `6502`/`8051` are not
/// related processors at all — aliasing any of them onto a row here is how a
/// source silently assembles as something it never asked for.
pub const CPU_SPELLINGS: &[(&str, Cpu)] = &[
    ("68000", Cpu::M68000),
    ("68008", Cpu::M68000),
    ("z80", Cpu::Z80),
    ("z80undoc", Cpu::Z80),
];

/// The target a `cpu` directive's processor name selects, or `None` when this
/// front end does not encode that instruction set. `folded` is the name already
/// lower-cased — AS processor names are case-insensitive.
pub fn cpu_for_spelling(folded: &str) -> Option<Cpu> {
    CPU_SPELLINGS
        .iter()
        .find(|(spelling, _)| *spelling == folded)
        .map(|(_, cpu)| *cpu)
}

/// The refusal raised when a `cpu` directive names a processor this front end
/// does not encode.
///
/// It names the fault and prints the remedy — the accepted lines, listed from
/// [`CPU_SPELLINGS`] itself rather than transcribed — and says why the answer is
/// a refusal rather than the nearest alias: an accepted spelling whose extra
/// instructions sigil does not encode would assemble them as something else,
/// which is the silent-wrong-output class this directive exists to prevent.
pub fn unsupported_cpu(name: &str) -> String {
    let accepted: Vec<String> = CPU_SPELLINGS
        .iter()
        .map(|(spelling, _)| format!("`cpu {spelling}`"))
        .collect();
    format!(
        "unsupported processor `{name}`: sigil's AS-compatibility front end does not encode this \
         instruction set, and will not assemble the source as a different processor instead. \
         Write one of {}. A spelling is accepted only when it names an instruction set sigil \
         encodes, so a wider processor is refused here rather than aliased onto a narrower one \
         and silently mis-assembled.",
        accepted.join(", ")
    )
}

/// Assembly options: the seeded symbol environment + the CPU active before any
/// `cpu` directive.
///
/// `Default` is derived, and that is now load-bearing rather than incidental:
/// every field's default is its type's own, so the default carries no
/// assumption about the target at all. It used to hand back `Cpu::Z80`, which
/// is how a 68000 source with no `cpu` line assembled silently as a Z80
/// program.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The processor the CALLER declares for this assembly unit, active before
    /// the first `cpu` directive — or `None` when the caller declares nothing
    /// and the source is expected to say so itself.
    ///
    /// `None` is NOT "assume something". A unit that reaches a CPU-dependent
    /// construct with neither this nor a `cpu` directive having declared a
    /// processor is refused outright ([`CPU_UNDECLARED`]). The old default was
    /// `Cpu::Z80` — honest for the Z80-only M0 build it was written for, and
    /// silently wrong afterwards: a 68000 source with no `cpu` line assembled
    /// as a Z80 program and said nothing, because under `Cpu::Z80` a `$` lexes
    /// as the program counter rather than a hex prefix.
    ///
    /// Setting this IS declaring: it is how a caller that drives the front-end
    /// on a fragment with no directive of its own (the `.emp` sound stack, the
    /// harness's residual-AS root) states the target.
    pub initial_cpu: Option<Cpu>,
    /// Pre-seeded integer symbols: the reference `-D` defines and (later) the
    /// stubbed 68k leaf values. Names are case-sensitive. These keep asl's
    /// silent-override semantics — an in-file `=`/`equ` of the same name wins
    /// (the code-gate / game-config-override defines rely on this).
    pub defines: Vec<(String, i64)>,
    /// Pre-seeded integer symbols the residual AS may NOT redefine — the
    /// `.emp`-owned constants injected by the Stage-3 P5 ownership flip (the
    /// harvest of `engine.constants`'s `pub const`s). Like [`defines`] they seed
    /// the env, but an in-file `=`/`equ` of a guarded name is a hard
    /// `[defines.collision]` error, not a silent override — the structural
    /// no-silent-shadowing guard proving a flipped constant has exactly ONE
    /// author (the `.emp` module). Distinct from [`defines`] precisely because
    /// the code-gate/override defines DO coexist with in-file definitions.
    pub guarded_defines: Vec<(String, i64)>,
    /// Directory that `include` paths resolve against. Set automatically by
    /// [`assemble_root`] from the root file's parent when left `None`.
    pub include_root: Option<std::path::PathBuf>,
}


/// Assemble a single source string into an unlinked [`Module`] (sections carry
/// labels + symbolic fixups; the linker resolves addresses). Returns every
/// diagnostic on failure.
pub fn assemble(src: &str, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    eval::run(src, opts)
}

/// Assemble a root source file, resolving `include` paths relative to its parent
/// directory (unless `opts.include_root` is already set).
pub fn assemble_root(root: &Path, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    assemble_root_located(root, opts).map_err(|f| f.diags)
}

/// Like [`assemble_root`] but keeps the [`SourceMap`] on failure, so each
/// diagnostic renders as `file(line): error: …` — the root file under the name it
/// was opened with, and every `include`d file under its own.
pub fn assemble_root_located(root: &Path, opts: &Options) -> Result<Module, Failure> {
    assemble_root_impl(root, opts, false).map(|a| a.module)
}

/// [`assemble_root_located`] keeping the WARN-tier diagnostics a successful run
/// raised — an `warning` directive the source author wrote. Use this wherever
/// the caller renders diagnostics; the module-only form above drops them, which
/// is right only for a caller with nothing to render them to.
pub fn assemble_root_located_warned(root: &Path, opts: &Options) -> Result<Assembled, Failure> {
    assemble_root_impl(root, opts, false)
}

/// Like [`assemble_root`] but keeps section-label references SYMBOLIC through the final
/// pass so a later relocation (the harness's chained placement)
/// resolves them against each label's placed base. Use for a build whose sections will
/// MOVE after assembly; a pinned build must use [`assemble_root`] (byte-for-byte asl).
pub fn assemble_root_relocating(root: &Path, opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    assemble_root_impl(root, opts, true)
        .map(|a| a.module)
        .map_err(|f| f.diags)
}

/// [`assemble_root_relocating`] keeping the WARN-tier diagnostics a successful
/// run raised, and the [`SourceMap`] that locates them.
pub fn assemble_root_relocating_warned(
    root: &Path,
    opts: &Options,
) -> Result<Assembled, Failure> {
    assemble_root_impl(root, opts, true)
}

fn assemble_root_impl(root: &Path, opts: &Options, relocate: bool) -> Result<Assembled, Failure> {
    let text = std::fs::read_to_string(root).map_err(|e| Failure {
        diags: vec![sigil_span::Diagnostic {
            level: sigil_span::Level::Error,
            message: format!("cannot read {}: {e}", root.display()),
            // The file never opened, so it is in no source map and the message
            // already names it; an id past every registered source keeps the
            // renderer from attributing it to a line.
            primary: sigil_span::Span {
                source: sigil_span::SourceId(u32::MAX),
                start: 0,
                end: 0,
            },
        }],
        sources: SourceMap::new(),
    })?;
    let mut o = opts.clone();
    if o.include_root.is_none() {
        o.include_root = root.parent().map(|p| p.to_path_buf());
    }
    let name = root.display().to_string();
    if relocate {
        eval::run_relocating(&text, &name, &o)
    } else {
        eval::run_located(&text, &name, &o)
    }
}
