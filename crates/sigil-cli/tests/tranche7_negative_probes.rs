//! Tranche 7 negative probes — each proves a tranche-7 guard/seam/feature
//! FAILS LOUD (or, for the documented sharp edge, assembles by design) when
//! doctored, against an undoctored control that succeeds where one exists (no
//! false-comfort: a probe that "fails" for an unrelated reason would pass
//! vacuously, so every doctored run that has a control pairs with a resolving
//! one through the same plumbing).
//!
//! (a) A DRIFTED sst.emp twin (a field offset changed) is caught by ITS OWN
//!     drift guard naming the field, riding collision.emp's ambient prepend.
//! (b) A DRIFTED constants.emp collision-block value (`ST_ON_OBJECT`) fires
//!     its own guard naming the constant.
//! (c) F1: a non-`int` splice (a `Reg`) in the aabb template's displacement
//!     slot is the `[asm.splice-kind]` diagnostic.
//! (d) F2: an unknown proc-local `.label` passed as `aabb_axis_test`'s `mlab`
//!     argument is a LOUD error naming the label (not a silent link dangle).
//! (e) A BROKEN falls_into stub chain (a `falls_into` removed so a stub gains
//!     fallthrough) fires the `[proc.undeclared-fallthrough]` diagnostic.
//! (f) The DOCUMENTED sharp edge: `aabb_axis_test` with `stmp` aliasing `cdim`
//!     ASSEMBLES CLEAN (the contract doc — and the .inc twin — carry the
//!     MUST-NOT-alias warning; the distinct-regs ask is ledgered). This probe
//!     PINS that the template does not (yet) reject the mis-instantiation.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn read_aeon(rel: &str) -> Option<String> {
    std::fs::read_to_string(aeon_dir().join(rel)).ok()
}

/// Lower one synthetic file (deps' items prepended under `main`'s header) to
/// its lower diagnostics. Panics on parse errors in `main` (the probes doctor
/// SEMANTICS, never syntax — except (e), which doctors a whole proc decl and
/// still parses); dep parse errors also panic. Returns the lower diags.
fn lower_with_ambient(dep_srcs: &[&str], main_src: &str) -> Vec<sigil_span::Diagnostic> {
    let mut items = Vec::new();
    for src in dep_srcs {
        let (file, diags) = parse_str(src);
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "dep parse errors: {diags:?}"
        );
        items.extend(file.items);
    }
    let (main, diags) = parse_str(main_src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "main parse errors: {diags:?}"
    );
    items.extend(main.items);
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        // collision.emp's A2 rail (item 1) references DEBUG in `if DEBUG == 1`
        // blocks; bind it (0 = plain, rail elided) so the ambient prepend lowers.
        defines: vec![("DEBUG".to_string(), 0)],
    };
    let (_module, ldiags) = lower_module(&file, &opts);
    ldiags
}

/// The four aeon sources collision.emp is compiled with (types + sst +
/// constants ambient, plus the aabb template). Skips (returns None) if the
/// aeon tree is absent.
struct Sources {
    types: String,
    sst: String,
    constants: String,
    aabb: String,
    collision: String,
}

fn sources() -> Option<Sources> {
    Some(Sources {
        types: read_aeon("engine/system/types.emp")?,
        sst: read_aeon("engine/objects/sst.emp")?,
        constants: read_aeon("engine/system/constants.emp")?,
        aabb: read_aeon("engine/objects/aabb.emp")?,
        collision: read_aeon("engine/objects/collision.emp")?,
    })
}

fn errors(diags: &[sigil_span::Diagnostic]) -> Vec<&str> {
    diags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message.as_str())
        .collect()
}

/// The AS-side value seam (the REAL structs.asm SST_* + constants.asm values
/// the twins guard against) — a trailing label+`dc.w` opens a section so the
/// equs flush (the collision_lookup / test_objects pattern).
fn as_truth_equs() -> Vec<Section> {
    // The 30 `SST_*` field pins + 18 engine constants both `.emp` twins guard
    // (SOURCE OF TRUTH: `structs.asm` / `constants.asm`), shared via
    // `sigil_harness::test_support`. The drifted-constants probe below builds
    // its own DOCTORED blob via `with_engine_constant_override`.
    sigil_harness::test_support::as_engine_constants_and_sst_equs()
}

/// Lower the quintet (types + sst + constants + aabb ambient, collision main)
/// to (sections, link_asserts, lower_diags) — the doctorable pipeline for the
/// drift probes.
fn lower_quintet(
    types: &str,
    sst: &str,
    constants: &str,
    aabb: &str,
    collision: &str,
) -> (Vec<Section>, Vec<sigil_ir::LinkAssert>, Vec<sigil_span::Diagnostic>) {
    let mut items = Vec::new();
    for src in [types, sst, constants, aabb] {
        items.extend(parse_str(src).0.items);
    }
    let (main, _) = parse_str(collision);
    items.extend(main.items.clone());
    let file = sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    };
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            // collision.emp's A2 rail (item 1) references DEBUG; bind it (0 =
            // plain, rail elided) so the doctored twin lowers to the link asserts.
            defines: vec![("DEBUG".to_string(), 0)],
        },
    );
    (module.sections, module.link_asserts, ldiags)
}

/// Check the module's drift-guard `ensure`s against the REAL AS truths (the
/// collision region is NOT placed — so the always-recorded
/// `[layout.odd-item]` even-address parity asserts on collision's own labels
/// are filtered out; only the value-drift guards matter here), returning the
/// ERROR-level assert messages.
fn check_guards_against_truths(link_asserts: &[sigil_ir::LinkAssert]) -> Vec<String> {
    let mut sections = as_truth_equs();
    for sec in &mut sections {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let guards: Vec<sigil_ir::LinkAssert> =
        sigil_harness::test_support::drift_guards_only(link_asserts).cloned().collect();
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (truths): {d:?}"));
    sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &guards)
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message.clone())
        .collect()
}

// ---- (a) drifted sst.emp twin → its own guard names the field ---------------

#[test]
fn drifted_sst_twin_fires_its_own_guard_naming_the_field() {
    let Some(s) = sources() else {
        eprintln!("skip: aeon tree not present");
        return;
    };

    // Control: the real quintet's guards PASS against the real AS truths.
    let (_c_sec, c_asserts, c_ldiags) =
        lower_quintet(&s.types, &s.sst, &s.constants, &s.aabb, &s.collision);
    assert!(
        c_ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "control must lower clean: {:?}",
        errors(&c_ldiags)
    );
    let control = check_guards_against_truths(&c_asserts);
    assert!(control.is_empty(), "control guards must all PASS: {control:?}");

    // The drift: change what the twin's guard EXPECTS for `width_pixels` ($16
    // → $15) — the exact shape of structs.asm drifting while sst.emp stays put
    // (or vice versa). The `@`-placed field itself is untouched, so the module
    // lowers clean; the guard `ensure(extern("SST_width_pixels") == $15, ...)`
    // then reads the REAL AS $16 against the doctored $15 and fires LOUD at
    // link, naming the field — BEFORE any consumer emits a wrong displacement.
    let doctored_sst = s.sst.replace(
        "ensure(extern(\"SST_width_pixels\") == $16,",
        "ensure(extern(\"SST_width_pixels\") == $15,",
    );
    assert_ne!(doctored_sst, s.sst, "the doctor must have found its target");
    let (_sec, asserts, ldiags) =
        lower_quintet(&s.types, &doctored_sst, &s.constants, &s.aabb, &s.collision);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the doctored twin still lowers clean (the guard is a link assert): {:?}",
        errors(&ldiags)
    );
    let fired = check_guards_against_truths(&asserts);
    assert!(
        fired.iter().any(|m| m.contains("width_pixels")),
        "the drifted twin's guard must fire LOUD naming `width_pixels`: {fired:?}"
    );
}

// ---- (b) drifted constants collision-block value → its own guard ------------

#[test]
fn drifted_constants_collision_value_fires_its_guard() {
    let Some(s) = sources() else {
        eprintln!("skip: aeon tree not present");
        return;
    };

    // The drift: change `ST_ON_OBJECT`'s value in the twin (5 → 4). Its guard
    // `ensure(extern("ST_ON_OBJECT") == ST_ON_OBJECT, ...)` now reads the real
    // AS value 5 against the doctored 4 and fires LOUD at link, naming the
    // constant.
    let doctored = s.constants.replace("pub const ST_ON_OBJECT   = 5", "pub const ST_ON_OBJECT   = 4");
    assert_ne!(doctored, s.constants, "the doctor must have found its target");
    let (_sec, asserts, ldiags) =
        lower_quintet(&s.types, &s.sst, &doctored, &s.aabb, &s.collision);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the doctored twin still lowers clean (the guard is a link assert): {:?}",
        errors(&ldiags)
    );
    let fired = check_guards_against_truths(&asserts);
    assert!(
        fired.iter().any(|m| m.contains("ST_ON_OBJECT")),
        "the drifted constants guard must fire LOUD naming `ST_ON_OBJECT`: {fired:?}"
    );
}

// ---- (c) F1: Reg splice in aabb's disp slot → [asm.splice-kind] -------------

#[test]
fn reg_splice_in_aabb_disp_position_is_splice_kind() {
    // A local aabb-shaped template whose `boff` is declared `Reg` (not `int`)
    // and spliced into the displacement slot — the exact aabb.emp shape with
    // the arg kind broken. Must be `[asm.splice-kind]`.
    let ldiags = lower_with_ambient(
        &[],
        concat!(
            "module m in collision\n",
            "comptime fn axis(boff: Reg, breg: Reg) -> Code {\n",
            "    return asm {\n",
            "        move.w  {boff}({breg}), d1\n",
            "    }\n",
            "}\n",
            "pub proc P () {\n",
            "    axis(d0, a3)\n",
            "    rts\n",
            "}\n",
        ),
    );
    let errs = errors(&ldiags);
    assert!(
        errs.iter().any(|e| e.contains("[asm.splice-kind]")),
        "a Reg splice in the disp slot must be [asm.splice-kind], got: {errs:?}"
    );
}

// ---- (d) F2: unknown .label mlab arg → loud, naming it ----------------------

#[test]
fn unknown_local_label_mlab_arg_is_loud_naming_it() {
    // The aabb consumer shape: an imported (here, local) template branching to
    // `{mlab}`, called with a proc-local label that is never defined —
    // mirroring `aabb_axis_test(..., .next_object)` with a typo'd target.
    let ldiags = lower_with_ambient(
        &[],
        concat!(
            "module m in collision\n",
            "comptime fn axis(mlab: Label) -> Code {\n",
            "    return asm {\n",
            "        bhs.s   {mlab}\n",
            "    }\n",
            "}\n",
            "pub proc P () {\n",
            "    axis(.no_such_object)\n",
            "    nop\n",
            ".next_object:\n",
            "    rts\n",
            "}\n",
        ),
    );
    let errs = errors(&ldiags);
    assert!(
        errs.iter().any(|e| e.contains("no_such_object")),
        "an unknown .label mlab arg must be loud naming it, got: {errs:?}"
    );
}

// ---- (e) broken falls_into chain → [proc.undeclared-fallthrough] ------------

#[test]
fn broken_falls_into_stub_chain_fires_fallthrough() {
    let Some(s) = sources() else {
        eprintln!("skip: aeon tree not present");
        return;
    };

    // Control: the real chain lowers with no undeclared-fallthrough diagnostic.
    let control = lower_with_ambient(&[&s.types, &s.sst, &s.constants, &s.aabb], &s.collision);
    assert!(
        !control.iter().any(|d| d.message.contains("[proc.undeclared-fallthrough]")),
        "control must have no undeclared-fallthrough diagnostic: {:?}",
        control.iter().map(|d| d.message.as_str()).collect::<Vec<_>>()
    );

    // The doctor: remove the `falls_into Touch_Enemy` from the FIRST stub, so
    // `Touch_None` becomes an empty body with no terminator and no falls_into
    // — it will run into whatever follows, which the fallthrough lint flags.
    let doctored = s
        .collision
        .replace("proc Touch_None () falls_into Touch_Enemy {}", "proc Touch_None () {}");
    assert_ne!(doctored, s.collision, "the doctor must have found its target");

    let diags = lower_with_ambient(&[&s.types, &s.sst, &s.constants, &s.aabb], &doctored);
    assert!(
        diags.iter().any(|d| d.message.contains("[proc.undeclared-fallthrough]")
            && d.message.contains("Touch_None")),
        "a stub that lost its falls_into must fire [proc.undeclared-fallthrough] naming it: {:?}",
        diags.iter().map(|d| d.message.as_str()).collect::<Vec<_>>()
    );
}

// ---- (f) documented sharp edge: stmp aliasing cdim ASSEMBLES ----------------

#[test]
fn aabb_stmp_aliasing_cdim_assembles_by_design() {
    let Some(s) = sources() else {
        eprintln!("skip: aeon tree not present");
        return;
    };

    // The real aabb.emp template, instantiated with `stmp` aliasing `cdim`
    // (both d0) — the contract doc (and the aabb.inc twin) say this MUST NOT
    // be done (the scratch clobbers the combined-dim before the compare), but
    // NOTHING checks it: the template splices the Reg args verbatim, so it
    // ASSEMBLES CLEAN and emits silently-wrong code. This probe PINS that
    // behavior — the distinct-regs predicate is the ledgered ask
    // (docs/superpowers/notes/campaign-gap-ledger.md, tranche-7 row); until it
    // ships, mis-instantiation is a miscompile, not a diagnostic.
    let consumer = concat!(
        "module m in collision\n",
        "pub proc P () {\n",
        "    moveq   #0, d0\n",
        "    moveq   #0, d1\n",
        // cdim and stmp BOTH d0 — the forbidden alias.
        "    aabb_axis_test(d4, a3, $2, d0, d1, d0, d1, d0, .out)\n",
        "    nop\n",
        ".out:\n",
        "    rts\n",
        "}\n",
    );
    let diags = lower_with_ambient(&[&s.aabb], consumer);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "stmp aliasing cdim must ASSEMBLE CLEAN (documented sharp edge, no check yet): {:?}",
        errors(&diags)
    );
}
