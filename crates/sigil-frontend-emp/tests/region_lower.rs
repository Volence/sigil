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
    // after Buf(100) + pad(4) = $FFFF0068, then @align(256) → $FFFF0100.
    assert_eq!(addr(&m, "Ring"), 0xFFFF0100);
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
    // Player_Phys = 8*2 = 16 → $FFFF8040, already 256-... no: $8040 → align 256
    // → $FFFF8100.
    assert_eq!(addr(&m, "Ring"), 0xFFFF8100);
    assert_eq!(addr(&m, "Game_RAM_End"), 0xFFFF8100 + 256);

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
