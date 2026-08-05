//! L1 (Spec 2 language round): the game contract — `interface` / `implement` /
//! `invoke`. An engine declares the hook SIGNATURES and value members as a typed
//! interface; a game provides the one `implement` block binding each member to a
//! symbol or value. The engine names members qualified (`Iface.MEMBER`,
//! `#Iface.proc`) and calls hooks with `invoke Iface.hook` — which lowers to an
//! absolute `jsr` when bound and to ZERO bytes when the hook is `empty`.
//!
//! These are P1's construct-level unit + lower tests over synthetic engine+game
//! module pairs. The real six-target ROM byte-identity (and the Config-A
//! re-freeze) lives in the P2 conversion.

use sigil_frontend_emp::ast;
use sigil_frontend_emp::lower::{lower_module_with_contracts, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::contract::{bind, ContractModule};
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, SymbolTable, SymbolValue};
use sigil_span::{Diagnostic, Level};

/// Parse a module source, asserting it parsed cleanly.
fn parse(src: &str) -> ast::File {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse diags: {perrs:?}");
    file
}

/// Run the bind pass over a set of module sources with the given defines.
fn bind_srcs(srcs: &[&str], defines: &[(String, i128)]) -> Vec<Diagnostic> {
    let files: Vec<ast::File> = srcs.iter().map(|s| parse(s)).collect();
    let ids: Vec<String> = files.iter().map(|f| f.module.path.segments.join(".")).collect();
    let mods: Vec<ContractModule> =
        files.iter().zip(&ids).map(|(f, id)| ContractModule { id, file: f }).collect();
    let (_env, diags) = bind(&mods, defines);
    diags
}

/// Bind `engine`+`game`, then lower the ENGINE module against the resolved env.
/// Returns the lowered module plus every diagnostic (bind + lower).
fn bind_and_lower(engine: &str, game: &str, defines: Vec<(String, i128)>) -> (Module, Vec<Diagnostic>) {
    let ef = parse(engine);
    let gf = parse(game);
    let eid = ef.module.path.segments.join(".");
    let gid = gf.module.path.segments.join(".");
    let mods =
        [ContractModule { id: &eid, file: &ef }, ContractModule { id: &gid, file: &gf }];
    let (env, mut diags) = bind(&mods, &defines);
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module_with_contracts(&ef, &opts, &env);
    diags.extend(ldiags);
    (module, diags)
}

/// Flatten a lowered module to a byte image, defining the game-side link symbols
/// the `invoke`/`#Iface.proc` fixups target (they live in the un-lowered game
/// module, so they must be supplied to the linker).
fn flatten_with_syms(module: &Module, syms: &[(&str, i64)]) -> Vec<u8> {
    let mut table = SymbolTable::new();
    for (name, addr) in syms {
        table.define(name, SymbolValue::Int(*addr));
    }
    let resolved =
        sigil_link::resolve_layout(&module.sections, &table, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &table).expect("link");
    sigil_link::flatten(&linked, 0x00)
}

fn errors(diags: &[Diagnostic]) -> Vec<String> {
    diags.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

fn has_tag(diags: &[Diagnostic], tag: &str) -> bool {
    diags.iter().any(|d| d.message.contains(tag))
}

// ---- parse -----------------------------------------------------------------

#[test]
fn interface_parses_all_member_kinds() {
    let file = parse(
        "\
module engine.contract
type GameState = proc () clobbers(d0-d7/a0-a6)
pub interface Game {
    const CAMERA_JUMP_LOCK: bool
    const ENTRY_ID: u8
    proc entry: GameState
    hook boot_hook () clobbers(d0-d1/a0-a1) = empty
    hook debug_tick () clobbers(d0-d7/a0-a6) = empty
}
",
    );
    let iface = file.items.iter().find_map(|it| match it {
        ast::Item::Interface(d) => Some(d),
        _ => None,
    });
    let iface = iface.expect("an Interface item");
    assert_eq!(iface.name, "Game");
    assert_eq!(iface.members.len(), 5);
    // The two hooks carry the `= empty` default marker.
    let empties = iface
        .members
        .iter()
        .filter(|m| matches!(&m.kind, ast::InterfaceMemberKind::Hook { default_empty: true, .. }))
        .count();
    assert_eq!(empties, 2, "both hooks are `= empty`");
}

#[test]
fn implement_parses_bindings_and_conditional_group() {
    let file = parse(
        "\
module games.g.manifest
pub implement Game {
    const CAMERA_JUMP_LOCK = true
    const ENTRY_ID = 7
    proc entry = GameState_Init
    if HOTKEYS == 1 {
        hook boot_hook = SoundTest_BootPing
        hook debug_tick = Debug_MusicToggle
    }
}
",
    );
    let imp = file.items.iter().find_map(|it| match it {
        ast::Item::Implement(d) => Some(d),
        _ => None,
    });
    let imp = imp.expect("an Implement item");
    assert_eq!(imp.name, "Game");
    // three top-level bindings + one comptime-if group.
    assert_eq!(imp.bindings.len(), 4);
    let has_group =
        imp.bindings.iter().any(|b| matches!(b, ast::ImplBinding::Group { .. }));
    assert!(has_group, "the `if HOTKEYS == 1` group parses as a Group binding");
}

#[test]
fn invoke_parses_as_an_asm_statement() {
    let file = parse(
        "module engine.boot\nproc boot() {\n    invoke Game.boot_hook\n    rts\n}\n",
    );
    let proc = file.items.iter().find_map(|it| match it {
        ast::Item::Proc(p) => Some(p),
        _ => None,
    });
    let proc = proc.expect("a proc");
    let has_invoke = proc.body.iter().any(
        |s| matches!(s, ast::AsmStmt::Invoke { iface, member, .. } if iface == "Game" && member == "boot_hook"),
    );
    assert!(has_invoke, "the `invoke Game.boot_hook` statement parses");
}

#[test]
fn contract_keywords_do_not_break_ordinary_identifiers() {
    // `hook`/`empty`/`interface`/`implement`/`invoke` are contextual — they stay
    // usable as ordinary names outside the construct positions.
    let file = parse(
        "module m\nconst hook = 1\nconst empty = 2\nconst interface = 3\nproc p() {\n    moveq #hook, d0\n    rts\n}\n",
    );
    let consts =
        file.items.iter().filter(|it| matches!(it, ast::Item::Const(_))).count();
    assert_eq!(consts, 3, "hook/empty/interface stay valid const names");
}

// ---- lowering: invoke jsr-or-nothing (THE load-bearing behavior) ------------

const ENGINE_BOOT: &str = "\
module engine.boot
pub interface Game {
    hook boot_hook () clobbers(d0-d1/a0-a1) = empty
}
proc boot() {
    invoke Game.boot_hook
    rts
}
";

/// A bound `implement` — the hook binds to a game proc.
const GAME_BOUND: &str = "\
module games.g.manifest
pub implement Game {
    hook boot_hook = GameBootHook
}
proc GameBootHook() clobbers(d0-d1/a0-a1) {
    rts
}
";

/// An `implement` that binds NOTHING — the `= empty` default carries the hook.
const GAME_EMPTY: &str = "\
module games.g.manifest
pub implement Game {
}
";

#[test]
fn bound_hook_emits_absolute_jsr() {
    let (module, diags) = bind_and_lower(ENGINE_BOOT, GAME_BOUND, vec![]);
    assert!(errors(&diags).is_empty(), "unexpected diags: {:?}", errors(&diags));
    let bytes = flatten_with_syms(&module, &[("GameBootHook", 0x0000_1234)]);
    // jsr (GameBootHook).l = 4E B9 + 00 00 12 34, then rts = 4E 75.
    assert_eq!(bytes, vec![0x4E, 0xB9, 0x00, 0x00, 0x12, 0x34, 0x4E, 0x75]);
}

#[test]
fn empty_hook_emits_nothing() {
    let (module, diags) = bind_and_lower(ENGINE_BOOT, GAME_EMPTY, vec![]);
    assert!(errors(&diags).is_empty(), "unexpected diags: {:?}", errors(&diags));
    let bytes = flatten_with_syms(&module, &[]);
    // The `invoke` vanished — only the `rts` remains.
    assert_eq!(bytes, vec![0x4E, 0x75]);
}

#[test]
fn empty_and_bound_differ_by_exactly_the_jsr() {
    // The with/without byte contract (§2 fact 2): the empty case is the bound
    // case minus the 6-byte absolute jsr.
    let (bound, _) = bind_and_lower(ENGINE_BOOT, GAME_BOUND, vec![]);
    let (empty, _) = bind_and_lower(ENGINE_BOOT, GAME_EMPTY, vec![]);
    let bound_bytes = flatten_with_syms(&bound, &[("GameBootHook", 0x0000_1234)]);
    let empty_bytes = flatten_with_syms(&empty, &[]);
    assert_eq!(bound_bytes.len(), empty_bytes.len() + 6);
}

// ---- lowering: conditional binding flips on a define ------------------------

#[test]
fn conditional_binding_flips_on_a_define() {
    // The impl binds the hook only under `HOTKEYS == 1`. With the define on, the
    // engine's `invoke` emits the jsr; off, the `= empty` default carries it.
    let game = "\
module games.g.manifest
pub implement Game {
    if HOTKEYS == 1 {
        hook boot_hook = GameBootHook
    }
}
proc GameBootHook() clobbers(d0-d1/a0-a1) {
    rts
}
";
    let (on, on_diags) = bind_and_lower(ENGINE_BOOT, game, vec![("HOTKEYS".into(), 1)]);
    let (off, off_diags) = bind_and_lower(ENGINE_BOOT, game, vec![("HOTKEYS".into(), 0)]);
    assert!(errors(&on_diags).is_empty(), "on diags: {:?}", errors(&on_diags));
    assert!(errors(&off_diags).is_empty(), "off diags: {:?}", errors(&off_diags));
    let on_bytes = flatten_with_syms(&on, &[("GameBootHook", 0x0000_1234)]);
    let off_bytes = flatten_with_syms(&off, &[]);
    assert_eq!(on_bytes, vec![0x4E, 0xB9, 0x00, 0x00, 0x12, 0x34, 0x4E, 0x75]);
    assert_eq!(off_bytes, vec![0x4E, 0x75]);
}

// ---- lowering: const member feeds a comptime if ----------------------------

#[test]
fn const_member_feeds_a_comptime_if() {
    let engine = "\
module engine.camera
pub interface Game {
    const CAMERA_JUMP_LOCK: bool
}
proc camera() {
    if Game.CAMERA_JUMP_LOCK {
        moveq #1, d0
    } else {
        moveq #2, d0
    }
    rts
}
";
    let game_on = "module games.g.manifest\npub implement Game {\n    const CAMERA_JUMP_LOCK = true\n}\n";
    let game_off = "module games.g.manifest\npub implement Game {\n    const CAMERA_JUMP_LOCK = false\n}\n";
    let (on, on_diags) = bind_and_lower(engine, game_on, vec![]);
    let (off, off_diags) = bind_and_lower(engine, game_off, vec![]);
    assert!(errors(&on_diags).is_empty(), "on: {:?}", errors(&on_diags));
    assert!(errors(&off_diags).is_empty(), "off: {:?}", errors(&off_diags));
    // moveq #1,d0 = 70 01 ; moveq #2,d0 = 70 02 ; rts = 4E 75.
    assert_eq!(flatten_with_syms(&on, &[]), vec![0x70, 0x01, 0x4E, 0x75]);
    assert_eq!(flatten_with_syms(&off, &[]), vec![0x70, 0x02, 0x4E, 0x75]);
}

// ---- lowering: proc member as an imm32 -------------------------------------

#[test]
fn proc_member_lowers_as_a_link_imm32() {
    let engine = "\
module engine.boot
type GameState = proc () clobbers(d0-d7/a0-a6)
pub interface Game {
    proc entry: GameState
}
proc boot() {
    move.l #Game.entry, d0
    rts
}
";
    let game = "\
module games.g.manifest
pub implement Game {
    proc entry = GameState_Init
}
proc GameState_Init() clobbers(d0-d7/a0-a6) {
    rts
}
";
    let (module, diags) = bind_and_lower(engine, game, vec![]);
    assert!(errors(&diags).is_empty(), "diags: {:?}", errors(&diags));
    // move.l #GameState_Init, d0 = 20 3C + the 4-byte link address; rts = 4E 75.
    let bytes = flatten_with_syms(&module, &[("GameState_Init", 0x0000_ABCD)]);
    assert_eq!(bytes, vec![0x20, 0x3C, 0x00, 0x00, 0xAB, 0xCD, 0x4E, 0x75]);
}

// ---- negative probes: one per §4 diagnostic --------------------------------

#[test]
fn probe_unimplemented() {
    // An interface with no `implement` in the module set.
    let diags = bind_srcs(
        &["module engine.c\npub interface Game {\n    const X: u8\n}\n"],
        &[],
    );
    assert!(has_tag(&diags, "[contract.unimplemented]"), "{:?}", errors(&diags));
}

#[test]
fn probe_duplicate_impl() {
    let engine = "module engine.c\npub interface Game {\n    const X: u8\n}\n";
    let g1 = "module games.a.m\npub implement Game {\n    const X = 1\n}\n";
    let g2 = "module games.b.m\npub implement Game {\n    const X = 2\n}\n";
    let diags = bind_srcs(&[engine, g1, g2], &[]);
    assert!(has_tag(&diags, "[contract.duplicate-impl]"), "{:?}", errors(&diags));
}

#[test]
fn probe_unknown_member() {
    let engine = "module engine.c\npub interface Game {\n    const X: u8\n}\n";
    let game = "module games.a.m\npub implement Game {\n    const X = 1\n    const NOPE = 2\n}\n";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.unknown-member]"), "{:?}", errors(&diags));
}

#[test]
fn probe_member_kind() {
    // The member is a const, but the impl binds it as a hook.
    let engine = "module engine.c\npub interface Game {\n    const X: u8\n}\n";
    let game = "module games.a.m\npub implement Game {\n    hook X = SomeProc\n}\n";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.member-kind]"), "{:?}", errors(&diags));
}

#[test]
fn probe_hook_signature() {
    // The bound proc clobbers MORE than the declared hook permits (d2 is outside
    // the declared d0-d1 bound) — the §4 subcontract violation.
    let engine = "\
module engine.c
pub interface Game {
    hook tick () clobbers(d0-d1) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook tick = TooGreedy
}
proc TooGreedy() clobbers(d0-d2) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.hook-signature]"), "{:?}", errors(&diags));
}

#[test]
fn probe_missing_member() {
    // A REQUIRED member (a const, no default) is left unbound.
    let engine = "module engine.c\npub interface Game {\n    const X: u8\n    const Y: u8\n}\n";
    let game = "module games.a.m\npub implement Game {\n    const X = 1\n}\n";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.missing-member]"), "{:?}", errors(&diags));
}

#[test]
fn required_hook_without_empty_default_must_be_bound() {
    // A hook WITHOUT `= empty` is required; omitting it is missing-member.
    let engine = "\
module engine.c
pub interface Game {
    hook must () clobbers(d0)
}
";
    let game = "module games.a.m\npub implement Game {\n}\n";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.missing-member]"), "{:?}", errors(&diags));
}

#[test]
fn well_formed_contract_binds_clean() {
    // Control: a complete, correctly-typed implement produces no diagnostics.
    let engine = "\
module engine.c
type GameState = proc () clobbers(d0-d7/a0-a6)
pub interface Game {
    const CAMERA_JUMP_LOCK: bool
    const ENTRY_ID: u8
    proc entry: GameState
    hook boot_hook () clobbers(d0-d1/a0-a1) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    const CAMERA_JUMP_LOCK = true
    const ENTRY_ID = 7
    proc entry = GameState_Init
    hook boot_hook = BootPing
}
proc GameState_Init() clobbers(d0-d7/a0-a6) {
    rts
}
proc BootPing() clobbers(d0-d1/a0-a1) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(errors(&diags).is_empty(), "expected a clean bind, got {:?}", errors(&diags));
}

/// §4 — a CONDITIONAL producer does not satisfy an UNCONDITIONAL `out` promise.
/// The hook promises callers `out(a1)` on every return; the bound proc produces
/// a1 only on its `eq` edge. Callers of the hook read a1 with no cc test.
#[test]
fn probe_hook_conditional_out_does_not_satisfy_unconditional_promise() {
    let engine = "\
module engine.c
pub interface Game {
    hook alloc () clobbers(d0) out(a1) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook alloc = CondAlloc
}
proc CondAlloc() clobbers(d0) out(a1 if eq) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.hook-signature]"), "{:?}", errors(&diags));
    assert!(
        errors(&diags).iter().any(|m| m.contains("does not produce output `a1`")),
        "must name the unproduced unconditional result: {:?}",
        errors(&diags)
    );
}

/// The matching pair conforms: a hook declaring the AllocDynamic shape
/// (`clobbers(d0/a1) out(a1 if eq)` — a1 is a result on the eq edge and
/// indeterminate scratch elsewhere) is satisfied by a proc declaring exactly
/// that. The clobber license comes from the hook's own `clobbers`, and the
/// conditional promise is met by a conditional producer.
#[test]
fn hook_conditional_out_bound_to_the_honest_alloc_shape_conforms() {
    let engine = "\
module engine.c
pub interface Game {
    hook alloc () clobbers(d0/a1) out(a1 if eq) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook alloc = AllocDynamic
}
proc AllocDynamic() clobbers(d0/a1) out(a1 if eq) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(
        !has_tag(&diags, "[contract.hook-signature]"),
        "the honest conditional-out shape must conform: {:?}",
        errors(&diags)
    );
}

/// A hook's conditional promise names an EDGE, not just a register: a hook
/// declaring `out(a1 if eq)` is not satisfied by a target that fills a1 only on
/// `ne`, because the hook's callers test `eq` and read a1 there. Comparing
/// register names alone accepts this pair silently.
#[test]
fn probe_hook_conditional_out_rejects_a_target_guarding_a_different_cc() {
    let engine = "\
module engine.c
pub interface Game {
    hook alloc () clobbers(d0/a1) out(a1 if eq) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook alloc = AllocDynamic
}
proc AllocDynamic() clobbers(d0/a1) out(a1 if ne) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.hook-signature]"), "{:?}", errors(&diags));
    assert!(
        errors(&diags).iter().any(|m| m.contains("conditional output `a1`") && m.contains("`eq`")),
        "must name the register and the edge the target fails to produce on: {:?}",
        errors(&diags)
    );
}

/// The condition compares CANONICALLY: `hs` and `lo` are the documented aliases of
/// `cc` and `cs`, so a target guarding `hs` satisfies a hook promising the same
/// guard spelled `cc`. A raw-text comparison rejects two spellings of one
/// condition, and this is the only test that drives the fold through the real
/// declaration path rather than a hand-built `Contract`.
#[test]
fn hook_conditional_out_folds_the_cc_aliases() {
    let engine = "\
module engine.c
pub interface Game {
    hook alloc () clobbers(d0/a1) out(a1 if cc) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook alloc = AllocDynamic
}
proc AllocDynamic() clobbers(d0/a1) out(a1 if hs) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(
        !has_tag(&diags, "[contract.hook-signature]"),
        "`hs` and `cc` are one condition: {:?}",
        errors(&diags)
    );
}

/// THE WALL: a hook declaring the AllocEffect shape (`clobbers(d0) out(a1 if
/// eq)`) claims a1 SURVIVES the ne edge — its callers may hold a1 across the
/// call and re-read it there. A target that also clobbers a1 leaves it
/// indeterminate on that edge and must be rejected. The claim is encoded purely
/// by a1's ABSENCE from the hook's `clobbers`, so the clobber license must not be
/// widened by a conditional out.
#[test]
fn probe_hook_survives_claim_rejects_a_target_that_clobbers_the_register() {
    let engine = "\
module engine.c
pub interface Game {
    hook alloc () clobbers(d0) out(a1 if eq) = empty
}
";
    let game = "\
module games.a.m
pub implement Game {
    hook alloc = AllocDynamic
}
proc AllocDynamic() clobbers(d0/a1) out(a1 if eq) {
    rts
}
";
    let diags = bind_srcs(&[engine, game], &[]);
    assert!(has_tag(&diags, "[contract.hook-signature]"), "{:?}", errors(&diags));
    assert!(
        errors(&diags).iter().any(|m| m.contains("clobbers `a1`")),
        "must name the register whose survives-claim the target breaks: {:?}",
        errors(&diags)
    );
}
