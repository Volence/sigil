//! L1 P2 — the game contract through the WHOLE-PROGRAM build path. P1's
//! `game_contract.rs` proves the `bind` pass and the `lower_module_with_contracts`
//! seam in isolation; these prove the P2 wiring: `build_program` runs the bind
//! pass over its reachable module set and threads the resolved env into every
//! module's lowering. A signature violation therefore fails the BUILD (not just a
//! unit `bind`), and a well-formed manifest's `invoke` lowers to a `jsr` across
//! the module boundary — exactly the aeon conversion's mechanism.

use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::resolve::{build_program_open_embed, manifest::Manifest};
use sigil_ir::backend::Cpu;
use sigil_ir::Section;
use sigil_span::{Diagnostic, Level};

fn write(dir: &std::path::Path, rel: &str, src: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, src).unwrap();
}

/// Build a whole program from `(rel, src)` files entering at `entry`, returning
/// the concatenated sections + every build diagnostic (the bind pass's included).
fn build(files: &[(&str, &str)], entry: &str) -> (Vec<Section>, Vec<Diagnostic>) {
    let dir = tempfile::tempdir().unwrap();
    for (rel, content) in files {
        write(dir.path(), rel, content);
    }
    let (manifest, mdiags) = Manifest::scan(dir.path());
    assert!(mdiags.iter().all(|d| d.level != Level::Error), "manifest errors: {mdiags:?}");
    let opts =
        LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] };
    // The OPEN build path the real aeon conversion uses: a hook's target is a
    // cross-seam symbol the engine module does not `use` (it resolves at the joint
    // link), so unresolved references defer rather than error — exactly like
    // boot.emp's `invoke Game.boot_hook` -> `jsr SoundTest_BootPing`.
    let (sections, _asserts, diags) =
        build_program_open_embed(&manifest, entry, None, &opts, &|_| None);
    (sections, diags)
}

fn errors(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

const ENGINE: &str = "\
module engine.c
pub interface Game {
    hook tick () clobbers(d0-d1) = empty
}
pub proc run() clobbers(d0-d1) {
    invoke Game.tick
    rts
}
";

/// THE required whole-build negative probe: a manifest binds a hook to a proc
/// that clobbers MORE than the interface declares. The bind pass runs INSIDE
/// `build_program` over the reachable set (engine + game, reached via the entry's
/// `use` edges), so the excess is a BUILD error — not something a separate unit
/// `bind` call has to be remembered to run.
#[test]
fn whole_build_rejects_a_hook_that_clobbers_too_much() {
    let game = "\
module games.g.m
pub implement Game {
    hook tick = TooGreedy
}
pub proc TooGreedy() clobbers(d0-d2) {
    rts
}
";
    let entry = "module app\nuse engine.c\nuse games.g.m\n";
    let (_secs, diags) = build(
        &[("engine/c.emp", ENGINE), ("games/g/m.emp", game), ("app.emp", entry)],
        "app",
    );
    assert!(
        has_tag(&diags, "[contract.hook-signature]"),
        "the whole build must reject the over-clobbering hook bind; got {:?}",
        errors(&diags)
    );
}

/// The reachable interface with no `implement` anywhere in the program is an
/// unimplemented BUILD error (the functor has no argument).
#[test]
fn whole_build_rejects_an_unimplemented_interface() {
    let entry = "module app\nuse engine.c\n";
    let (_secs, diags) = build(&[("engine/c.emp", ENGINE), ("app.emp", entry)], "app");
    assert!(
        has_tag(&diags, "[contract.unimplemented]"),
        "an interface with no implement in the reachable set must fail the build; got {:?}",
        errors(&diags)
    );
}

/// The positive control: a well-formed manifest builds clean AND the engine's
/// `invoke Game.tick` lowers to a real `jsr` targeting the bound game proc — the
/// bind env is threaded through the whole-program lowering, not just resolved.
#[test]
fn whole_build_binds_clean_and_invoke_lowers_to_a_jsr() {
    let game = "\
module games.g.m
pub implement Game {
    hook tick = GoodTick
}
pub proc GoodTick() clobbers(d0-d1) {
    rts
}
";
    let entry = "module app\nuse engine.c\nuse games.g.m\n";
    let (sections, diags) = build(
        &[("engine/c.emp", ENGINE), ("games/g/m.emp", game), ("app.emp", entry)],
        "app",
    );
    assert!(errors(&diags).is_empty(), "expected a clean whole build, got {:?}", errors(&diags));
    // The engine.c `run` proc's `invoke Game.tick` must have emitted a `jsr`
    // (opcode 0x4E,0xB9 = jsr abs.l) — i.e. the whole-program lowering saw the
    // bound hook and threaded the env into engine.c's lowering.
    let jsr_present = |secs: &[Section]| {
        secs.iter().any(|s| s.image_bytes().windows(2).any(|w| w == [0x4E, 0xB9]))
    };
    assert!(jsr_present(&sections), "the bound `invoke Game.tick` must lower to a jsr");

    // The discriminator: the SAME engine with an EMPTY implement (hook stays
    // `= empty`) emits NO jsr — the `invoke` vanishes. Proves the jsr above is
    // the bound hook, not incidental.
    let empty_impl = "module games.g.m\npub implement Game {\n}\n";
    let (empty_secs, empty_diags) = build(
        &[("engine/c.emp", ENGINE), ("games/g/m.emp", empty_impl), ("app.emp", entry)],
        "app",
    );
    assert!(errors(&empty_diags).is_empty(), "empty-impl build diags: {:?}", errors(&empty_diags));
    assert!(!jsr_present(&empty_secs), "the `= empty` hook must emit no jsr");
}
