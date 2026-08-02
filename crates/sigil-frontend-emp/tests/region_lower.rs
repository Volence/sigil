//! Item #7a — region-form `vars` LOWERING: every §4 mapping-table row, every
//! §5 diagnostic (positive + negative), determinism, and a chained-region
//! fixture modeled on the real `engine/ram.asm` shape with hand-computed
//! addresses asserted exactly.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::Module;

fn opts(defines: Vec<(String, i128)>) -> LowerOptions {
    LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines }
}

/// Lower `src`, asserting zero ERROR diagnostics; returns the module + all diags
/// (warnings survive for the lint tests).
fn lower(src: &str, defines: Vec<(String, i128)>) -> (Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "parse diagnostics: {perrs:?}");
    lower_module(&file, &opts(defines))
}

fn lower_ok(src: &str, defines: Vec<(String, i128)>) -> Module {
    let (m, diags) = lower(src, defines);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "unexpected error diagnostics: {errs:?}");
    m
}

/// Absolute VMA of a label anywhere in the module (base + offset).
fn addr(m: &Module, name: &str) -> u32 {
    for s in &m.sections {
        for l in &s.labels {
            if l.name == name {
                return s.vma_origin() + l.offset;
            }
        }
    }
    panic!("label `{name}` not found");
}

/// Value of an equate (alias) symbol.
fn equ(m: &Module, name: &str) -> i64 {
    for s in &m.sections {
        for e in &s.equ_syms {
            if e.name == name {
                if let sigil_ir::expr::Expr::Int(n) = e.expr {
                    return n;
                }
                panic!("equ `{name}` is not an int: {:?}", e.expr);
            }
        }
    }
    panic!("equ `{name}` not found");
}

fn has_error(diags: &[sigil_span::Diagnostic], id: &str) -> bool {
    diags.iter().any(|d| d.level == sigil_span::Level::Error && d.message.contains(id))
}
fn has_warning(diags: &[sigil_span::Diagnostic], id: &str) -> bool {
    diags.iter().any(|d| d.level == sigil_span::Level::Warning && d.message.contains(id))
}

// ---------------------------------------------------------------------------
// §4 mapping-table rows — each construct → its address.
// ---------------------------------------------------------------------------

#[test]
fn scalar_and_array_widths() {
    // `ds.b`/`ds.w`/`ds.l` → u8/u16/u32; `[T; N]` → N*sizeof(T).
    let m = lower_ok(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r {\n\
             A: u8,\n\
             pad(1),\n\
             B: u16,\n\
             C: u32,\n\
             D: [u8; 10],\n\
             E: [u16; 3],\n\
         }\n",
        vec![],
    );
    assert_eq!(addr(&m, "A"), 0xFFFF8000);
    assert_eq!(addr(&m, "B"), 0xFFFF8002); // after A(1) + pad(1)
    assert_eq!(addr(&m, "C"), 0xFFFF8004); // after B(2)
    assert_eq!(addr(&m, "D"), 0xFFFF8008); // after C(4)
    assert_eq!(addr(&m, "E"), 0xFFFF8012); // after D(10)
}

#[test]
fn struct_and_struct_array_sizes() {
    // `[Sst; N]` → N*sizeof(Sst); struct scalar → sizeof(Sst).
    let m = lower_ok(
        "module m\n\
         struct Ent { x: u16, y: u16, flag: u8, pad: u8 }\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r {\n\
             One: Ent,\n\
             Many: [Ent; 4],\n\
             After: u8,\n\
         }\n",
        vec![],
    );
    // sizeof(Ent) = 2+2+1+1 = 6.
    assert_eq!(addr(&m, "One"), 0xFFFF8000);
    assert_eq!(addr(&m, "Many"), 0xFFFF8006);
    assert_eq!(addr(&m, "After"), 0xFFFF8006 + 6 * 4);
}

#[test]
fn mark_alias_pad_align() {
    // `mark Name`, `Name: alias(Other)`, `pad(N)`, `@align(N)`.
    let m = lower_ok(
        "module m\n\
         region r @ $FFFF0000 .. $FFFF8000\n\
         vars r {\n\
             Buf: [u8; 100],\n\
             mark Buf_End,\n\
             Staging: alias(Buf),\n\
             pad(4),\n\
             Ring: [u8; 256] @align(256),\n\
         }\n",
        vec![],
    );
    assert_eq!(addr(&m, "Buf"), 0xFFFF0000);
    assert_eq!(addr(&m, "Buf_End"), 0xFFFF0064); // mark at Buf + 100, no advance
    assert_eq!(equ(&m, "Staging"), 0xFFFF0000i64); // alias == Buf's address
    // after Buf(100) + pad(4) = $FFFF0068, then @align(256). Region align uses AS's
    // in-phase semantics (round_up(cursor + n, n) — always a full extra `n`), so
    // $FFFF0068 → round_up($FFFF0168, 256) = $FFFF0200 (NOT the plain-round-up
    // $FFFF0100). This is the asl behavior the RAM byte-identity depends on.
    assert_eq!(addr(&m, "Ring"), 0xFFFF0200);
}

#[test]
fn conditional_group_shape_divergent() {
    // Size-varying group WITH @shape_divergent — DEBUG on vs off give distinct,
    // exact layouts downstream of the group.
    let src = "module m\n\
        region r @ $FFFF8000 .. $FFFFFF00\n\
        vars r {\n\
            Head: u16,\n\
            if __DEBUG__ @shape_divergent {\n\
                Dbg: u32,\n\
                pad(2),\n\
            }\n\
            Tail: u16,\n\
            mark End,\n\
        }\n";
    let plain = lower_ok(src, vec![("__DEBUG__".into(), 0)]);
    assert_eq!(addr(&plain, "Tail"), 0xFFFF8002); // group empty in release
    assert_eq!(addr(&plain, "End"), 0xFFFF8004);
    let debug = lower_ok(src, vec![("__DEBUG__".into(), 1)]);
    assert_eq!(addr(&debug, "Dbg"), 0xFFFF8002);
    assert_eq!(addr(&debug, "Tail"), 0xFFFF8002 + 4 + 2); // Dbg(4)+pad(2)
    assert_eq!(addr(&debug, "End"), addr(&debug, "Tail") + 2);
}

#[test]
fn conditional_group_shape_invariant_needs_no_annotation() {
    // Size-EQUAL arms (u8 vs pad(1)) need no @shape_divergent — proven invariant.
    let src = "module m\n\
        region r @ $FFFF8000 .. $FFFFFF00\n\
        vars r {\n\
            if __DEBUG__ { Live: u8 } else { pad(1) }\n\
            After: u8,\n\
        }\n";
    let plain = lower_ok(src, vec![("__DEBUG__".into(), 0)]);
    assert_eq!(addr(&plain, "After"), 0xFFFF8001);
    let debug = lower_ok(src, vec![("__DEBUG__".into(), 1)]);
    assert_eq!(addr(&debug, "Live"), 0xFFFF8000);
    assert_eq!(addr(&debug, "After"), 0xFFFF8001); // same as release — invariant
}

// ---------------------------------------------------------------------------
// Chained-region fixture — modeled on engine/ram.asm (two fixed regions + a
// chained third), hand-computed addresses asserted exactly.
// ---------------------------------------------------------------------------

#[test]
fn chained_engine_game_fixture() {
    let src = "module m\n\
        const SYSTEM_STACK: u32 = $FFFFFF00\n\
        const NUM_DYNAMIC: u32 = 4\n\
        struct Sst { x: u16, y: u16, id: u8, sub: u8, w: u16, h: u16 }\n\
        region lower_ram @ $FFFF0000 .. $FFFF8000\n\
        pub region upper_ram @ $FFFF8000 .. SYSTEM_STACK, w_addressable\n\
        pub region game_ram @ after(upper_ram) .. SYSTEM_STACK, w_addressable\n\
        vars lower_ram {\n\
            Tile_Cache: [u8; 9600],\n\
            mark Lower_RAM_End,\n\
        }\n\
        pub vars upper_ram {\n\
            VBlank_Flag: u8,\n\
            pad(1),\n\
            Frame_Counter: u16,\n\
            Game_State: u32,\n\
            mark Object_RAM,\n\
            Slots: [Sst; NUM_DYNAMIC],\n\
            if __DEBUG__ @shape_divergent {\n\
                Lag_Frame_Count: u32,\n\
                Debug_Scene_Freeze: u8,\n\
                pad(1),\n\
            }\n\
            mark Engine_RAM_End,\n\
        }\n\
        pub vars game_ram {\n\
            Player_Phys: [u16; 8],\n\
            Ring: [u8; 256] @align(256),\n\
            mark Game_RAM_End,\n\
        }\n";

    // sizeof(Sst) = 2+2+1+1+2+2 = 10; NUM_DYNAMIC=4 → Slots = 40.
    // --- release shape ---
    let m = lower_ok(src, vec![("__DEBUG__".into(), 0)]);
    assert_eq!(addr(&m, "VBlank_Flag"), 0xFFFF8000);
    assert_eq!(addr(&m, "Frame_Counter"), 0xFFFF8002); // VBlank(1)+pad(1)
    assert_eq!(addr(&m, "Game_State"), 0xFFFF8004);
    assert_eq!(addr(&m, "Object_RAM"), 0xFFFF8008); // after Game_State(4)
    assert_eq!(addr(&m, "Slots"), 0xFFFF8008);
    // Engine_RAM_End = Object_RAM + 40 (no debug block) = $FFFF8030.
    assert_eq!(addr(&m, "Engine_RAM_End"), 0xFFFF8030);
    // game_ram base = after(upper_ram) = Engine_RAM_End = $FFFF8030.
    assert_eq!(addr(&m, "Player_Phys"), 0xFFFF8030);
    // Player_Phys = 8*2 = 16 → $FFFF8040. @align(256) uses AS in-phase semantics
    // (round_up(cursor + n, n)): $FFFF8040 → round_up($FFFF8140, 256) = $FFFF8200
    // (a full extra 256 beyond the plain-round-up $FFFF8100). This mirrors AS's
    // `align` inside a `phase`, which the real game-RAM Player_Pos_Ring depends on.
    assert_eq!(addr(&m, "Ring"), 0xFFFF8200);
    assert_eq!(addr(&m, "Game_RAM_End"), 0xFFFF8200 + 256);

    // --- debug shape: the @shape_divergent block shifts everything after it ---
    let d = lower_ok(src, vec![("__DEBUG__".into(), 1)]);
    assert_eq!(addr(&d, "Slots"), 0xFFFF8008); // unchanged (before the block)
    // debug block = Lag(4)+Freeze(1)+pad(1) = 6 → Engine_RAM_End = $8030 + 6.
    assert_eq!(addr(&d, "Engine_RAM_End"), 0xFFFF8036);
    assert_eq!(addr(&d, "Player_Phys"), 0xFFFF8036); // game_ram chases the shift
}

// ---------------------------------------------------------------------------
// Determinism.
// ---------------------------------------------------------------------------

#[test]
fn determinism_same_input_same_addresses() {
    let src = "module m\n\
        region r @ $FFFF8000 .. $FFFFFF00\n\
        vars r { A: u16, B: u32, mark E }\n";
    let a = lower_ok(src, vec![]);
    let b = lower_ok(src, vec![]);
    for name in ["A", "B", "E"] {
        assert_eq!(addr(&a, name), addr(&b, name), "address of {name} not deterministic");
    }
}

// ---------------------------------------------------------------------------
// Diagnostics — positive + negative for each §5 check.
// ---------------------------------------------------------------------------

#[test]
fn region_duplicate() {
    let (_, d) = lower(
        "module m\nregion r @ $FFFF8000 .. $FFFFFF00\nregion r @ $FFFF9000 .. $FFFFFF00\n",
        vec![],
    );
    assert!(has_error(&d, "[region.duplicate]"));
    // negative: two distinct regions do not trip it.
    let (_, d2) = lower(
        "module m\nregion a @ $FFFF8000 .. $FFFF9000\nregion b @ $FFFF9000 .. $FFFFFF00\n",
        vec![],
    );
    assert!(!has_error(&d2, "[region.duplicate]"));
}

#[test]
fn region_unknown() {
    let (_, d) = lower("module m\nvars ghost { A: u8 }\n", vec![]);
    assert!(has_error(&d, "[region.unknown]"));
    // negative: a declared region resolves.
    let (_, d2) = lower(
        "module m\nregion r @ $FFFF8000 .. $FFFFFF00\nvars r { A: u8 }\n",
        vec![],
    );
    assert!(!has_error(&d2, "[region.unknown]"));
}

#[test]
fn region_chain_cycle() {
    let (_, d) = lower(
        "module m\n\
         region a @ after(b) .. $FFFFFF00\n\
         region b @ after(a) .. $FFFFFF00\n",
        vec![],
    );
    assert!(has_error(&d, "[region.chain-cycle]"));
    // negative: a linear chain is fine.
    let (_, d2) = lower(
        "module m\n\
         region a @ $FFFF8000 .. $FFFFFF00\n\
         region b @ after(a) .. $FFFFFF00\n\
         vars a { X: u8 }\n",
        vec![],
    );
    assert!(!has_error(&d2, "[region.chain-cycle]"));
}

#[test]
fn region_overflow() {
    let (_, d) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFF8004\n\
         vars r { A: u32, Over: u16 }\n",
        vec![],
    );
    assert!(has_error(&d, "[region.overflow]"), "diags: {d:?}");
    // The message names the crossing field and the over-by amount.
    let msg = d.iter().find(|x| x.message.contains("[region.overflow]")).unwrap();
    assert!(msg.message.contains("Over"), "should name crossing field: {}", msg.message);
    // negative: a region that fits does not overflow.
    let (_, d2) = lower(
        "module m\nregion r @ $FFFF8000 .. $FFFF8010\nvars r { A: u32 }\n",
        vec![],
    );
    assert!(!has_error(&d2, "[region.overflow]"));
}

#[test]
fn region_not_w_addressable() {
    // base low word < $8000 → not reachable by sign-extended .w.
    let (_, d) = lower(
        "module m\nregion r @ $FFFF0000 .. $FFFF4000, w_addressable\nvars r { A: u8 }\n",
        vec![],
    );
    assert!(has_error(&d, "[region.not-w-addressable]"));
    // negative: an upper-RAM window IS .w-addressable.
    let (_, d2) = lower(
        "module m\nregion r @ $FFFF8000 .. $FFFFFF00, w_addressable\nvars r { A: u8 }\n",
        vec![],
    );
    assert!(!has_error(&d2, "[region.not-w-addressable]"));
}

#[test]
fn vars_shape_divergent() {
    // Size-varying group WITHOUT @shape_divergent → error.
    let (_, d) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r {\n\
             if __DEBUG__ { A: u32 } else { pad(1) }\n\
             After: u8,\n\
         }\n",
        vec![("__DEBUG__".into(), 0)],
    );
    assert!(has_error(&d, "[vars.shape-divergent]"));
    // negative: same shape WITH the annotation is accepted.
    let (_, d2) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r {\n\
             if __DEBUG__ @shape_divergent { A: u32 } else { pad(1) }\n\
             After: u8,\n\
         }\n",
        vec![("__DEBUG__".into(), 0)],
    );
    assert!(!has_error(&d2, "[vars.shape-divergent]"));
    // negative: size-EQUAL arms need no annotation.
    let (_, d3) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r { if __DEBUG__ { A: u16 } else { pad(2) } After: u8 }\n",
        vec![("__DEBUG__".into(), 0)],
    );
    assert!(!has_error(&d3, "[vars.shape-divergent]"));
}

#[test]
fn layout_odd_field() {
    // A u16 at an odd address → the odd-field lint (warning).
    let (_, d) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r { A: u8, B: u16 }\n",
        vec![],
    );
    assert!(has_warning(&d, "[layout.odd-field]"), "diags: {d:?}");
    // negative: an explicit pad(1) fixes it.
    let (_, d2) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r { A: u8, pad(1), B: u16 }\n",
        vec![],
    );
    assert!(!has_warning(&d2, "[layout.odd-field]"));
    // negative: a byte array at an odd address is fine (no word alignment need).
    let (_, d3) = lower(
        "module m\n\
         region r @ $FFFF8000 .. $FFFFFF00\n\
         vars r { A: u8, B: [u8; 4] }\n",
        vec![],
    );
    assert!(!has_warning(&d3, "[layout.odd-field]"));
}

#[test]
fn conditional_group_accepts_comparison_condition() {
    // Spec §8.1 ratified the corpus spelling `if DEBUG == 1` — a COMPARISON,
    // which evaluates to a Bool. The region-group condition must accept it (the
    // #7a fixture only exercised the bare-integer `if __DEBUG__` form). Both the
    // then-arm (DEBUG=1) and else-arm (DEBUG=0) select correctly, and a field
    // AFTER the group lands at the shape-correct address.
    let src = "module m\n\
        region r @ $FFFF8000 .. $FFFFFF00\n\
        vars r {\n\
            Head: u16,\n\
            if DEBUG == 1 @shape_divergent { Dbg: u32, }\n\
            Tail: u16,\n\
        }\n";
    // DEBUG=1: Head@8000, Dbg@8002, Tail@8006.
    let m1 = lower_ok(src, vec![("DEBUG".into(), 1)]);
    assert_eq!(addr(&m1, "Head"), 0xFFFF8000);
    assert_eq!(addr(&m1, "Dbg"), 0xFFFF8002);
    assert_eq!(addr(&m1, "Tail"), 0xFFFF8006);
    // DEBUG=0: the group is empty, Tail@8002.
    let m0 = lower_ok(src, vec![("DEBUG".into(), 0)]);
    assert_eq!(addr(&m0, "Head"), 0xFFFF8000);
    assert_eq!(addr(&m0, "Tail"), 0xFFFF8002);
    assert!(m0.sections.iter().all(|s| s.labels.iter().all(|l| l.name != "Dbg")));
}

#[test]
fn place_sections_skips_ram_reserve_section() {
    // Item #7b's first task: `place_sections` must SKIP the reserve-only RAM
    // section a region-form `vars` block lowers to (`vma_origin >= $F00000`) —
    // matching its region name (`upper_ram`) against the ROM map would spuriously
    // fire the "no region in the map" error. A module here produces TWO sections:
    // a RAM reserve section (`upper_ram`) and a ROM data section (`rom_sec`). The
    // map declares ONLY the ROM region.
    use sigil_frontend_emp::resolve::place_sections;
    use sigil_ir::map::{MemoryMap, Region, RegionKind};

    let m = lower_ok(
        "module m in rom_sec\n\
         pub region upper_ram @ $FFFF8000 .. $FFFFF000\n\
         pub vars upper_ram { Cam: u32, mark UpperEnd }\n\
         pub data D: [u8; 4] = [1, 2, 3, 4]\n",
        vec![],
    );
    let mut sections = m.sections.clone();
    // Sanity: the lowering produced both the RAM reserve section and the ROM data
    // section (order-independent).
    assert!(sections.iter().any(|s| s.name == "upper_ram" && s.vma_origin() >= 0x00F0_0000));
    assert!(sections.iter().any(|s| s.name == "rom_sec"));

    let map = MemoryMap::new(
        vec![Region {
            name: "rom_sec".into(),
            lma_base: 0x1000,
            size: 0x1000,
            kind: RegionKind::Rom,
            vma_base: None,
        }],
        0x00,
    );
    let diags = place_sections(&mut sections, &map);
    let errs: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(errs.is_empty(), "RAM section must be skipped, not error: {errs:?}");

    // The ROM section was placed at the region base; the RAM section was skipped
    // (its VMA base untouched, still `$FFFF8000`).
    let rom = sections.iter().find(|s| s.name == "rom_sec").unwrap();
    assert_eq!(rom.lma, 0x1000, "ROM section placed at region base");
    let ram = sections.iter().find(|s| s.name == "upper_ram").unwrap();
    assert_eq!(ram.vma_origin(), 0xFFFF_8000, "RAM section VMA base preserved");
}

#[test]
fn multiple_owners_check() {
    use sigil_frontend_emp::lower::check_single_owner;
    let (fa, _) = parse_str("module a\nvars shared { A: u8 }\n");
    let (fb, _) = parse_str("module b\nvars shared { B: u8 }\n");
    let diags = check_single_owner(&[("a", &fa), ("b", &fb)]);
    assert!(has_error(&diags, "[region.multiple-owners]"), "diags: {diags:?}");
    // negative: both blocks in one module is allowed (source-order blocks).
    let (fc, _) = parse_str("module a\nvars shared { A: u8 }\nvars shared { B: u8 }\n");
    let diags2 = check_single_owner(&[("a", &fc)]);
    assert!(!has_error(&diags2, "[region.multiple-owners]"), "diags: {diags2:?}");
}

// ---------------------------------------------------------------------------
// Cross-module `after(..)` (item #7c) — the game's `game_ram` chains onto the
// engine's `upper_ram` declared in ANOTHER module. `resolve_program_region_ends`
// resolves the whole-program ends; a parent resolves before its dependents
// regardless of module order (principled, not incidental).
// ---------------------------------------------------------------------------

#[test]
fn cross_module_after_chains_across_modules() {
    use sigil_frontend_emp::lower::resolve_program_region_ends;
    let (engine, _) = parse_str(
        "module engine.ram\n\
         region upper_ram @ $FFFF8000 .. $FFFFFF00\n\
         vars upper_ram { A: [u8; 100], mark Engine_RAM_End }\n",
    );
    let (game, _) = parse_str(
        "module game.ram\n\
         region game_ram @ after(upper_ram) .. $FFFFFF00\n\
         vars game_ram { B: u16, mark Game_RAM_End }\n",
    );
    // Deliberately list the GAME (dependent) BEFORE the ENGINE (parent) — the
    // fixpoint still converges to the parent-first topological answer.
    let (ends, diags) =
        resolve_program_region_ends(&[("game.ram", game), ("engine.ram", engine)], &[]);
    assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    // upper_ram end = $FFFF8000 + 100 = $FFFF8064 (== Engine_RAM_End).
    assert_eq!(ends["upper_ram"], 0xFFFF_8064);
    // game_ram base = upper_ram end = $FFFF8064; + B(2) = $FFFF8066.
    assert_eq!(ends["game_ram"], 0xFFFF_8066);
}

#[test]
fn cross_module_after_cycle_reported() {
    use sigil_frontend_emp::lower::resolve_program_region_ends;
    // Two modules whose regions `after(..)` each other — a cross-module cycle.
    let (a, _) = parse_str(
        "module a\n\
         region ra @ after(rb) .. $FFFFFF00\n\
         vars ra { X: u8 }\n",
    );
    let (b, _) = parse_str(
        "module b\n\
         region rb @ after(ra) .. $FFFFFF00\n\
         vars rb { Y: u8 }\n",
    );
    let (_ends, diags) = resolve_program_region_ends(&[("a", a), ("b", b)], &[]);
    assert!(has_error(&diags, "[region.chain-cycle]"), "diags: {diags:?}");
}

/// T1 — the RAM map report (`collect_region_report`): the per-region row carries the
/// base, running end, allocated size, alignment/`pad` padding, and the budget limit,
/// with `end`/`capacity`/`headroom` derived. Chains cross-module `after(..)` via the
/// whole-program end map, exactly as the shipping build resolves it.
#[test]
fn ram_report_rows_carry_geometry_and_padding() {
    use sigil_frontend_emp::lower::{collect_region_report, resolve_program_region_ends};
    use std::collections::HashMap;

    // Engine RAM: one region with an @align gap + a pad(1) — the padding column.
    let (engine, _) = parse_str(
        "module engine.ram\n\
         region upper_ram @ $FFFF8000 .. $FFFFFF00\n\
         vars upper_ram {\n\
             A: u8,\n\
             pad(1),\n\
             B: [u8; 4] @align(256),\n\
         }\n",
    );
    // Game RAM chained onto the engine region (base = upper_ram's running end).
    let (game, _) = parse_str(
        "module games.g.ram\n\
         region game_ram @ after(upper_ram) .. $FFFFFF00\n\
         vars game_ram { C: u16 }\n",
    );

    // Whole-program ends first, then per-module rows against them.
    let (ends, ediags) =
        resolve_program_region_ends(&[("engine.ram", engine.clone()), ("games.g.ram", game.clone())], &[]);
    assert!(ediags.is_empty(), "end diags: {ediags:?}");

    let (erows, ed) = collect_region_report(&engine, &[], &ends);
    assert!(ed.iter().all(|d| d.level != sigil_span::Level::Error), "{ed:?}");
    let up = &erows[0];
    assert_eq!(up.name, "upper_ram");
    assert_eq!(up.base, 0xFFFF_8000);
    assert_eq!(up.limit, 0xFFFF_FF00);
    // A(1) + pad(1) → cursor $FFFF8002; @align(256) is the AS-phase align (round_up
    // of cursor+256 — always at least a full 256 beyond, even when aligned), landing
    // $FFFF8200, i.e. +510 pad; then B(4). size = 2 + 510 + 4 = 516; padding = 1 + 510.
    assert_eq!(up.size, 516);
    assert_eq!(up.padding, 1 + 510);
    assert_eq!(up.end(), 0xFFFF_8204);
    assert_eq!(up.capacity(), 0x7F00);
    assert_eq!(up.headroom(), 0xFFFF_FF00 - 0xFFFF_8204);

    let (grows, gd) = collect_region_report(&game, &[], &ends);
    assert!(gd.iter().all(|d| d.level != sigil_span::Level::Error), "{gd:?}");
    let gr = &grows[0];
    assert_eq!(gr.name, "game_ram");
    assert_eq!(gr.base, up.end(), "game_ram chains onto upper_ram's running end");
    assert_eq!(gr.size, 2);
    assert_eq!(gr.padding, 0);

    // A region-free module yields no rows and no diagnostics.
    let (plain, _) = parse_str("module m\nconst K = 1\n");
    let (rows, diags) = collect_region_report(&plain, &[], &HashMap::new());
    assert!(rows.is_empty() && diags.is_empty());
}
