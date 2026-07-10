//! Tranche 7 — the REAL `collision.emp` port, region-level byte gate.
//!
//! `test_objects_port.rs`'s sibling for the SEVENTH code port and the FIRST
//! ENGINE-region module in the object-bank neighborhood: compiles the ACTUAL
//! ported file from aeon's tree — `engine/objects/collision.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline, and asserts
//! the `collision` section's flattened bytes equal the reference ROM window at
//! the pinned addresses, in BOTH build shapes.
//!
//! ## What this port exercises that the prior six did not
//!
//! - **F1 disp-position splice** — `aabb.emp`'s template emits
//!   `sub.w {boff}({breg}), {delt}` with `{boff}` a comptime int in the
//!   displacement slot (`offsetof(Sst, x_pos)` / `y_pos`), the zero-disp and
//!   d16 shapes both riding the literal path.
//! - **F2 proc-local label VALUE argument** — `TouchResponse` passes its own
//!   `.next_object` local label into `aabb_axis_test(..., .next_object)`; the
//!   template's `bhs.s {mlab}` mangles in the CALLER's hygienic space, so the
//!   branch lands on `TouchResponse`'s `.next_object`, not the template's.
//! - **F3 cross-module `pub comptime fn` import** — `collision.emp` does
//!   `use engine.objects.aabb.{aabb_axis_test}` and splices the imported
//!   template twice (X then Y).
//! - **The typed Sst module-level twin** — `TouchResponse` reads
//!   `Sst.field(a2)` (unqualified `code_addr`/`x_pos`/… never; the proc has no
//!   typed params) while `Touch_Hurt`/`Touch_Solid` read bare `y_pos(a2)` off
//!   their `a2: *Sst` params — both fold to the same displacements comptime.
//! - **The module-level handler table with pc-indexed jsr** —
//!   `jsr Touch_HandlerTable(pc, d4.w)` into a `bra.w`-entry table, the
//!   Volence-ratified dispatch shape.
//! - **The empty falls_into stub chain** — eleven stub handlers alias the one
//!   `rts`; the fallthrough lint enforces the chain.
//!
//! ## Compile technique
//!
//! Like tranche 6, each object module lowers as ONE synthetic `ast::File` with
//! its `use`-dependencies' items PREPENDED (the ambient technique): `sst.emp`
//! (which itself pulls `engine.types`) + `constants.emp` (the collision block:
//! NUM_*/COLLISION_TOUCH/ST_* + their drift guards) + `aabb.emp` (the
//! zero-byte comptime-fn template). The prepended guard `ensure`s ride along
//! and are asserted to PASS against the synthetic AS-side truths.
//!
//! ## Cross-seam symbols
//!
//! INBOUND equs (values): the SST_* struct-equ seam + the engine constants
//! (the collision block's eight, guarded by `constants.emp`). INBOUND labels
//! at true per-shape VMAs (GAME RAM, moves with `__DEBUG__`): `Player_1`
//! (plain `$FFFF89EE` / debug `$FFFF8A10`) and `Dynamic_Slots`
//! (`$FFFF8A8E` / `$FFFF8AB0`), both `.w`-addressed, width-selected to abs.w.
//!
//! OUTBOUND: `TouchResponse` is called from the engine's object manager; a
//! synthetic `bsr.w TouchResponse` consumer proves the sole `pub proc` export
//! surfaces as a bare link symbol resolving to the per-shape region base.
//!
//! ## Reference windows
//!
//! Plain (map base `$308A`): `s4.bin[0x308A..0x31FA]` (0x170 bytes).
//! Debug (map base `$3344`): `s4.debug.bin[0x3344..0x34B4]` (0x170 bytes).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test collision_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The region geometry — SHAPE-DEPENDENT base, shape-invariant size
/// (2026-07-10 pins, both listings).
const COLLISION_LEN: usize = 0x170;

/// Per-shape TRUE VMAs — the region base plus the two GAME-RAM cross-seam
/// labels (game RAM moves with `__DEBUG__`).
struct Shape {
    base: u32,
    player_1: u32,
    dynamic_slots: u32,
}

const PLAIN: Shape = Shape { base: 0x308A, player_1: 0xFFFF_89EE, dynamic_slots: 0xFFFF_8A8E };
const DEBUG: Shape = Shape { base: 0x3344, player_1: 0xFFFF_8A10, dynamic_slots: 0xFFFF_8AB0 };

/// Parse one `.emp` file to an AST, failing loudly on parse errors.
fn parse_file(path: &std::path::Path) -> sigil_frontend_emp::ast::File {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

/// One synthetic file: `deps`' items prepended to `main`'s own, under `main`'s
/// module header (the ambient-injection technique).
fn with_ambient(
    deps: Vec<sigil_frontend_emp::ast::File>,
    main: sigil_frontend_emp::ast::File,
) -> sigil_frontend_emp::ast::File {
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(main.items);
    sigil_frontend_emp::ast::File {
        module: main.module.clone(),
        attrs: main.attrs.clone(),
        items,
        docs: main.docs.clone(),
    }
}

/// The AS-side value seam: the SST struct-generated equs (structs.asm) and the
/// engine constants the `constants.emp` twin guards against (the collision
/// block's eight, plus the older ones the twin also carries). A trailing
/// label+`dc.w` opens a section so the equs flush (the collision_lookup
/// pattern).
fn as_constant_equs() -> Vec<Section> {
    // The 30 `SST_*` field pins + 19 engine constants both `.emp` twins guard
    // (SOURCE OF TRUTH: `structs.asm` / `constants.asm`), shared via
    // `sigil_harness::test_support`.
    sigil_harness::test_support::as_engine_constants_and_sst_equs()
}

/// One synthetic AS-side label phased at `vma` — a `dc.b 0` carrier whose LABEL
/// address is load-bearing (`Player_1`/`Dynamic_Slots` are abs.w EAs, so their
/// positions must match the real game-RAM addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — mirrors the engine object manager's
/// `bsr.w TouchResponse`, assembled through the AS front-end with the label
/// UNDEFINED in-unit (the `.emp` owns it). Proves the sole `pub proc` export
/// surfaces as a bare link symbol.
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tbsr.w   TouchResponse\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `collision` region pinned at the per-shape reference base, sized to the
/// 0x170-byte block.
fn map_toml(base: u32) -> String {
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"collision\"\n\
         lma_base = {base:#x}\n\
         size = {COLLISION_LEN:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `collision.emp` with its ambient dependencies (sst + types
/// (via sst) + constants + aabb), place it at the per-shape base, append the
/// synthetic cross-seam sections, and link. Returns (resolved sections, linked
/// image, all captured link asserts).
fn compile_real_file(
    shape: &Shape,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let aabb = parse_file(&aeon.join("engine/objects/aabb.emp"));
    let collision = parse_file(&aeon.join("engine/objects/collision.emp"));

    // engine.types rides in front of sst (sst.emp imports it); aabb is the
    // zero-byte template `collision.emp` splices via F3.
    let file = with_ambient(vec![types, sst, constants, aabb], collision);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "collision.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    for group in [
        &mut as_constant_equs(),
        &mut as_label_at("Player_1", shape.player_1),
        &mut as_label_at("Dynamic_Slots", shape.dynamic_slots),
        &mut as_outbound_consumer(),
    ] {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.append(group);
        lma += 0x10_0000;
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// All prepended drift guards must be captured and PASS against the synthetic
/// AS-side truths: sst.emp's 30 SST_* pins plus constants.emp's 19 (the four
/// button + two hw-port + CTYPE_AIR + RF pair + AF_DELETE + VDP_Shadow_len that
/// predate this tranche, plus the collision block's eight new
/// NUM_*/COLLISION_TOUCH/ST_* guards) = 49.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    assert_eq!(guards, 49, "sst.emp's 30 + constants.emp's 19 drift guards must be captured");
    let diags = sigil_link::check_link_asserts(resolved, &SymbolTable::new(), link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the drift guards must all PASS: {diags:?}"
    );
}

/// On mismatch, report the first differing offset plus context on each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(),
        expected.len(),
        "{what}: length mismatch — candidate {} bytes, expected {} bytes",
        candidate.len(),
        expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(candidate.len());
        panic!(
            "{what}: first diff at offset {i:#x} (region-relative)\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &candidate[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// The region reference gate + the cross-seam label pins + the outbound
/// bare-name proof + the drift guards, shared body.
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let (resolved, linked, link_asserts) = compile_real_file(shape);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("collision").expect("linked image must carry collision");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + COLLISION_LEN],
        &format!("collision vs {rom_name}[{base:#x}..{:#x}]", base + COLLISION_LEN),
    );

    // Cross-seam label pins (act_descriptor_port.rs / test_objects_port.rs
    // style): the first instruction is `lea (Player_1).w, a2` — abs.w word at
    // region offset 2 must equal the low half of Player_1's VMA — and the
    // second `lea (Dynamic_Slots).w, a3` sits at region offset 0x1C.
    let player_word = u16::from_be_bytes([section.bytes[2], section.bytes[3]]);
    assert_eq!(
        player_word,
        (shape.player_1 & 0xFFFF) as u16,
        "`lea (Player_1).w, a2` must carry Player_1's abs.w address"
    );
    let dynamic_word = u16::from_be_bytes([section.bytes[0x1E], section.bytes[0x1F]]);
    assert_eq!(
        dynamic_word,
        (shape.dynamic_slots & 0xFFFF) as u16,
        "`lea (Dynamic_Slots).w, a3` must carry Dynamic_Slots's abs.w address"
    );

    // Outbound bare-name proof: the AS-side `bsr.w TouchResponse` fixup
    // resolves to the per-shape region base. The consumer is the FOURTH
    // synthetic group: 0x0100_0000 + 3 × 0x10_0000.
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0130_0000)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    let disp = i16::from_be_bytes([consumer.bytes[2], consumer.bytes[3]]);
    let expected_disp = (shape.base as i64 - (consumer.lma as i64 + 2)) as i16;
    assert_eq!(
        disp, expected_disp,
        "bare-name proof: `bsr.w TouchResponse` must resolve to the region base"
    );
}

/// (plain) the `collision` region == `s4.bin[0x308A..0x31FA]`.
#[test]
fn collision_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) the `collision` region == `s4.debug.bin[0x3344..0x34B4]`.
#[test]
fn collision_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}
