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
//! (f) `aabb_axis_test` with `stmp` aliasing `cdim` is a COMPILE ERROR naming
//!     the MUST-NOT-alias constraint (retro-fix-audit-1 item 7: the template
//!     carries `ensure(stmp != cdim)` / `ensure(stmp != delt)` — the distinct-
//!     regs ledger row, now resolved via comptime Reg-equality ensure).

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
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
    coords: String,
    collision: String,
}

fn sources() -> Option<Sources> {
    Some(Sources {
        types: read_aeon("engine/system/types.emp")?,
        sst: read_aeon("engine/objects/sst.emp")?,
        constants: read_aeon("engine/system/constants.emp")?,
        aabb: read_aeon("engine/objects/aabb.emp")?,
        // coords carries abs_w (ambient-hoist parcel folded collision's AABB
        // |delta| sites onto the shared template).
        coords: read_aeon("engine/coords.emp")?,
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

// ---- (a) drifted sst.emp twin → its own guard names the field ---------------

// The `drifted_sst_twin_fires_its_own_guard_naming_the_field` probe retired with
// the conv-a structs flip: sst.emp's `SST_*` drift wall is deleted (the struct is
// the sole author, harvested into the residual AS), so a drifted field moves ROM
// bytes and is caught by the six-target byte-identity, not a link guard.

// ---- (b) drifted constants collision-block value → RETIRED ------------------
//
// The `drifted_constants_collision_value_fires_its_guard` probe drove
// `constants.emp`'s `ensure(extern("ST_ON_OBJECT") == ST_ON_OBJECT)` drift
// guard. The Stage-3 P5 ownership flip made `ST_ON_OBJECT` (and the other engine
// constants) SOLE-authored by `constants.emp` — harvested into guarded AS
// defines — so there is no AS-side twin to drift and its mirror guard was
// deleted. Doctoring the `.emp` value can no longer fire a guard, so this probe
// tested a retired mechanism and is removed. The `width_pixels` field-twin probe
// above (a `structs`/`sst` wall) is unaffected and still fires.

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
    let control = lower_with_ambient(&[&s.types, &s.sst, &s.constants, &s.aabb, &s.coords], &s.collision);
    assert!(
        !control.iter().any(|d| d.message.contains("[proc.undeclared-fallthrough]")),
        "control must have no undeclared-fallthrough diagnostic: {:?}",
        control.iter().map(|d| d.message.as_str()).collect::<Vec<_>>()
    );

    // The doctor: remove the `falls_into Touch_Enemy` from the FIRST stub, so
    // `Touch_None` becomes an empty body with no terminator and no falls_into
    // — it will run into whatever follows, which the fallthrough lint flags.
    let doctored = s.collision.replace(
        "proc Touch_None () clobbers() falls_into Touch_Enemy {}",
        "proc Touch_None () clobbers() {}",
    );
    assert_ne!(doctored, s.collision, "the doctor must have found its target");

    let diags = lower_with_ambient(&[&s.types, &s.sst, &s.constants, &s.aabb, &s.coords], &doctored);
    assert!(
        diags.iter().any(|d| d.message.contains("[proc.undeclared-fallthrough]")
            && d.message.contains("Touch_None")),
        "a stub that lost its falls_into must fire [proc.undeclared-fallthrough] naming it: {:?}",
        diags.iter().map(|d| d.message.as_str()).collect::<Vec<_>>()
    );
}

// ---- (f) stmp aliasing cdim is now a COMPILE ERROR (retro-fix item 7) -------

#[test]
fn aabb_stmp_aliasing_cdim_is_a_compile_error() {
    let Some(s) = sources() else {
        eprintln!("skip: aeon tree not present");
        return;
    };

    // The real aabb.emp template, instantiated with `stmp` aliasing `cdim`
    // (both d0) — the contract's MUST-NOT-alias rule (the scratch neg/double
    // clobbers the combined-dim before the compare). As of retro-fix-audit-1
    // item 7, aabb_axis_test carries `ensure(stmp != cdim)` / `ensure(stmp !=
    // delt)`, so this mis-instantiation is a COMPILE ERROR naming the constraint
    // — was a silent miscompile (gap-ledger tranche-7 distinct-regs row, now
    // RESOLVED via comptime Reg-equality ensure).
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
        diags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("stmp MUST NOT alias cdim")),
        "stmp aliasing cdim must now be a COMPILE ERROR naming the constraint (item 7): {:?}",
        errors(&diags)
    );
}
