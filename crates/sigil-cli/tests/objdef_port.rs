//! Tranche 14 — the objdef data file (the ObjDef-twin driver).
//!
//! This gate covers the DATA-emission side of object spawning: the `ObjDef`
//! struct-twin (co-located with `Sst` in `engine/objects/sst.emp`), its
//! burst-copy ensure-chain, and (later tests) the `objdef()` emitter + the
//! `test_objects.emp` consumer byte-gated against the AS macro.
//!
//! REFERENCE-DEPENDENT: reads the sibling aeon tree via `AEON_DIR`. For the
//! t14 branch this must point at the worktree
//! (`aeon/.worktrees/sigil-emp-tranche14`) per the paired-state gate.

use sigil_frontend_emp::eval::eval_const;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::value::Value;
use sigil_ir::backend::Cpu;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}

fn parse_file(rel: &str) -> sigil_frontend_emp::ast::File {
    let path = aeon_dir().join(rel);
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let (file, diags) = parse_str(&src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {diags:?}",
        path.display()
    );
    file
}

/// A synthetic module carrying `sst.emp`'s items (which need `engine.types`
/// prepended) plus any extra source appended — so `offsetof(ObjDef, …)` /
/// `offsetof(Sst, …)` resolve against the real struct decls.
fn sst_ambient_with(extra: &str) -> sigil_frontend_emp::ast::File {
    let types = parse_file("engine/system/types.emp");
    let sst = parse_file("engine/objects/sst.emp");
    let (extra_file, diags) = parse_str(&format!("module probe\n{extra}"));
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "probe parse errors: {diags:?}"
    );
    let mut items = Vec::new();
    items.extend(types.items);
    items.extend(sst.items);
    items.extend(extra_file.items);
    sigil_frontend_emp::ast::File {
        module: sst.module.clone(),
        attrs: sst.attrs.clone(),
        items,
        docs: sst.docs.clone(),
    }
}

/// The eager offsetof ensure-chain (and the whole ObjDef struct layout) must
/// lower WITHOUT error — a broken burst-copy correspondence is a build error.
/// The `extern("SST_*")` drift guards defer to link and do not fire here.
#[test]
fn objdef_burst_copy_ensure_chain_passes() {
    let file = sst_ambient_with("");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, diags) = lower_module(&file, &opts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "ObjDef ensure-chain / layout must lower clean, got: {diags:?}"
    );
}

/// ObjDef is the compact 26-byte record: spot-check the offsets that define
/// the burst-copy correspondence (code_addr@0, x_vel@2, mappings@8, pad@$18).
#[test]
fn objdef_compact_layout_offsets() {
    for (probe, want) in [
        ("const R = offsetof(ObjDef, code_addr)", 0),
        ("const R = offsetof(ObjDef, x_vel)", 2),
        ("const R = offsetof(ObjDef, render_flags)", 6),
        ("const R = offsetof(ObjDef, mappings)", 8),
        ("const R = offsetof(ObjDef, art_tile)", 0xC),
        ("const R = offsetof(ObjDef, anim_table)", 0x12),
        ("const R = offsetof(ObjDef, pad)", 0x18),
        ("const R = sizeof(ObjDef)", 26),
    ] {
        let file = sst_ambient_with(probe);
        let (v, diags) = eval_const(&file, "R");
        assert!(diags.is_empty(), "{probe}: unexpected diagnostics: {diags:?}");
        assert_eq!(v, Some(Value::Int(want)), "{probe}");
    }
}

// ---- the novel path: a struct-typed data item with link-valued fields ----
// The objdef emitter returns an ObjDef struct VALUE whose code_addr is a
// link DIFFERENCE (u16 → Value16Be) and whose mappings is a plain symbol
// (u32 → Abs32Be). No prior .emp data item is struct-typed, so characterize
// the combination directly before building the emitter on it.

use sigil_ir::FixupKind;

fn lower_inline(src: &str) -> sigil_ir::Module {
    let (file, diags) = parse_str(src);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {diags:?}"
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors: {ldiags:?}"
    );
    module
}

/// Section-relative (offset, kind) fixups across a section's Data fragments.
fn section_fixups(sec: &sigil_ir::Section) -> Vec<(u32, FixupKind)> {
    let mut out = Vec::new();
    let mut base = 0u32;
    for frag in &sec.fragments {
        if let sigil_ir::Fragment::Data(d) = frag {
            for f in &d.fixups {
                out.push((base + f.offset, f.kind));
            }
            base += d.bytes.len() as u32;
        }
    }
    out
}

#[test]
fn struct_data_item_routes_symbol_and_difference_fields_to_fixups() {
    let src = "module m\n\
        struct Rec (size: 6) { a: u16 @ 0, b: u32 @ 2 }\n\
        data D: Rec = Rec { a: extern(\"Foo\") - extern(\"Base\"), b: extern(\"Bar\") }\n";
    let module = lower_inline(src);
    let sec = module
        .sections
        .iter()
        .find(|s| !s.image_bytes().is_empty())
        .expect("a data section");
    assert_eq!(sec.image_bytes().len(), 6, "Rec is 6 bytes");
    let fixups = section_fixups(sec);
    let at = |off: u32| fixups.iter().find(|(o, _)| *o == off).map(|(_, k)| *k);
    assert_eq!(
        at(0),
        Some(FixupKind::Value16Be),
        "u16 link-difference field → Value16Be at offset 0; fixups: {fixups:?}"
    );
    assert_eq!(
        at(2),
        Some(FixupKind::Abs32Be),
        "u32 plain-symbol field → Abs32Be at offset 2; fixups: {fixups:?}"
    );
}

// ---- the objdef() emitter: end-to-end record emission -------------------
// Ambient = types + sst (ObjDef) + constants (RF_PRIORITY_SHIFT) + objdef,
// then an inline consumer mirroring test_objects.asm. Proves render_flags
// packing, defaults, art_tile, and the code_addr/mappings fixups.

fn emitter_ambient(consumer_body: &str) -> sigil_frontend_emp::ast::File {
    let types = parse_file("engine/system/types.emp");
    let sst = parse_file("engine/objects/sst.emp");
    let constants = parse_file("engine/system/constants.emp");
    let objdef = parse_file("engine/objects/objdef.emp");
    let (consumer, diags) = parse_str(&format!("module probe\n{consumer_body}"));
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "consumer parse errors: {diags:?}"
    );
    let (module, attrs, docs) = (consumer.module.clone(), consumer.attrs.clone(), consumer.docs.clone());
    let mut items = Vec::new();
    for f in [types, sst, constants, objdef, consumer] {
        items.extend(f.items);
    }
    sigil_frontend_emp::ast::File { module, attrs, items, docs }
}

#[test]
fn objdef_solid_record_bytes_and_fixups() {
    // ObjDef_Solid: priority 3, 16x16, COLLISION_SOLID(8).
    let file = emitter_ambient(
        "const VRAM_TEST_OBJ = $03E0\n\
         data ObjDef_Solid: ObjDef = objdef(code: \"TestSolid_Init\", map: \"Map_TestObj\", \
             art: vram_art(VRAM_TEST_OBJ), priority: 3, width: 16, height: 16, collision: 8)\n",
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, diags) = lower_module(&file, &opts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "emitter lower errors: {diags:?}"
    );
    let sec = module
        .sections
        .iter()
        .find(|s| !s.image_bytes().is_empty())
        .expect("a data section");
    let bytes = sec.image_bytes();
    assert_eq!(bytes.len(), 26, "one ObjDef record is 26 bytes");

    // Literal (non-fixup) fields.
    assert_eq!(bytes[0x06], 0x60, "render_flags = 0 | (3 << 5) = 0x60");
    assert_eq!(bytes[0x07], 0x08, "collision_resp = COLLISION_SOLID");
    assert_eq!(&bytes[0x0C..0x0E], &[0x03, 0xE0], "art_tile = VRAM_TEST_OBJ (pal/pri 0)");
    assert_eq!(bytes[0x0E], 16, "width");
    assert_eq!(bytes[0x0F], 16, "height");
    assert_eq!(&bytes[0x02..0x06], &[0, 0, 0, 0], "x_vel/y_vel default 0");
    assert_eq!(&bytes[0x12..0x16], &[0, 0, 0, 0], "anim_table default 0");
    assert_eq!(&bytes[0x18..0x1A], &[0, 0], "pad = 0");

    // Fixups: code_addr word (Value16Be) @0, mappings long (Abs32Be) @8.
    let fixups = section_fixups(sec);
    let at = |off: u32| fixups.iter().find(|(o, _)| *o == off).map(|(_, k)| *k);
    assert_eq!(at(0), Some(FixupKind::Value16Be), "code_addr word; fixups: {fixups:?}");
    assert_eq!(at(8), Some(FixupKind::Abs32Be), "mappings long; fixups: {fixups:?}");
}

#[test]
fn objdef_priority_over_7_is_a_compile_error() {
    // The refinement upgrade of the macro's runtime `fatal "priority exceeds 7"`.
    let file = emitter_ambient(
        "data Bad: ObjDef = objdef(code: \"X\", map: \"Y\", priority: 8)\n",
    );
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (_module, diags) = lower_module(&file, &opts);
    assert!(
        diags.iter().any(|d| d.level == sigil_span::Level::Error),
        "priority: 8 must be a compile error (0..7 refinement), got: {diags:?}"
    );
}

// ---- the reference byte gate: test_objects.emp vs s4.bin windows --------
// Compiles the REAL games/sonic4/data/objdefs/test_objects.emp through the
// production pipeline, resolves its cross-seam symbols (the four routine
// labels + Map_TestObj + ObjCodeBase) to their true per-shape VMAs, and
// asserts the linked objdef records equal the reference ROM window. Both
// shapes (the objdefs region base + Map_TestObj are shape-dependent game
// data; the routine labels are shape-invariant bank code).

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_ir::{SectionPlacement, SymbolTable};

/// One synthetic AS-side label phased at `vma`.
fn as_label_at(name: &str, vma: u32) -> Vec<sigil_ir::Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts)
        .unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}"))
        .sections
}

/// Per-shape true VMAs of the objdef byte gate's cross-seam targets.
struct ObjShape {
    region_base: u32,
    region_len: usize,
    map_test_obj: u32,
    rom: &'static str,
}

fn objdef_reference_gate(shape: &ObjShape) {
    use sigil_harness::pins;
    let aeon = aeon_dir();
    let rom_path = aeon.join(shape.rom);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if std::env::var("SIGIL_STRICT_GATE").is_ok() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    // Ambient: types + sst (ObjDef) + constants (RF_PRIORITY_SHIFT) + objdef,
    // prepended to the real consumer.
    let deps = [
        parse_file("engine/system/types.emp"),
        parse_file("engine/objects/sst.emp"),
        parse_file("engine/system/constants.emp"),
        parse_file("engine/objects/objdef.emp"),
    ];
    let consumer = parse_file("games/sonic4/data/objdefs/test_objects.emp");
    let (module_hdr, attrs, docs) = (consumer.module.clone(), consumer.attrs.clone(), consumer.docs.clone());
    let mut items = Vec::new();
    for d in deps {
        items.extend(d.items);
    }
    items.extend(consumer.items);
    let file = sigil_frontend_emp::ast::File { module: module_hdr, attrs, items, docs };

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "test_objects.emp lower errors: {ldiags:?}"
    );

    // Pin every section manually (no map): the one byte-emitting data section
    // at the region base, the empty dep sections at scratch LMAs clear of it.
    let mut sections: Vec<sigil_ir::Section> = module.sections;
    let mut scratch = 0x0200_0000u32;
    let mut found_data = false;
    for s in sections.iter_mut() {
        if !s.image_bytes().is_empty() {
            s.lma = shape.region_base;
            s.vma_base = Some(shape.region_base);
            found_data = true;
        } else {
            s.lma = scratch;
            s.vma_base = Some(scratch);
            scratch += 0x1000;
        }
        s.placement = SectionPlacement::Pinned;
        s.group = None;
    }
    assert!(found_data, "test_objects.emp must emit a data section");

    // Synthetic cross-seam definitions at harness-private LMAs (clear of the
    // objdefs region): ObjCodeBase as a value equ, the rest as phased labels.
    // Plus the OUTBOUND mixed-build seam: an AS-side spawn-list consumer
    // (`dc.l ObjDef_Enemy`, the object_test_state.asm shape) with the symbol
    // UNDEFINED in-unit — it must resolve to the .emp's `pub data` export.
    let obj_code_base = format!("${:X}", pins::OBJ_CODE_BASE.plain);
    let mut synth = sigil_harness::test_support::assemble_equ_pairs(&[("ObjCodeBase", obj_code_base.as_str())]);
    let outbound_asm = "cpu 68000\nObjDefConsumer:\n\tdc.l ObjDef_Enemy\n";
    let mut outbound = assemble(outbound_asm, &AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() })
        .unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}"))
        .sections;
    let mut lma = 0x0100_0000u32;
    for group in [
        &mut synth,
        &mut as_label_at("TestStatic_Main", pins::TEST_STATIC_MAIN.plain),
        &mut as_label_at("TestSolid_Init", pins::TEST_SOLID_INIT.plain),
        &mut as_label_at("TestEnemy_Init", pins::TEST_ENEMY_INIT.plain),
        &mut as_label_at("TestParent", pins::TEST_PARENT.plain),
        &mut as_label_at("Map_TestObj", shape.map_test_obj),
        &mut outbound,
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

    let data = linked
        .sections
        .iter()
        .find(|s| s.lma == shape.region_base)
        .expect("linked image must carry the objdefs section at the region base");
    let expected = &refrom[shape.region_base as usize..shape.region_base as usize + shape.region_len];
    assert_region_matches(&data.bytes, expected, &format!("objdefs vs {}", shape.rom));

    // Outbound seam: the AS `dc.l ObjDef_Enemy` must resolve to the .emp
    // export's VMA = region base + 2 records (ObjDef_Enemy is the third).
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0160_0000)
        .expect("linked image must carry the outbound consumer");
    let resolved_enemy = u32::from_be_bytes([consumer.bytes[0], consumer.bytes[1], consumer.bytes[2], consumer.bytes[3]]);
    assert_eq!(
        resolved_enemy,
        shape.region_base + 2 * 26,
        "AS-side `dc.l ObjDef_Enemy` must resolve to the .emp export ({}=+52)",
        shape.rom
    );
}

/// On mismatch, report the first differing offset with context.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    assert_eq!(
        candidate.len(), expected.len(),
        "{what}: length mismatch — candidate {} vs expected {}",
        candidate.len(), expected.len()
    );
    if let Some(i) = (0..candidate.len()).find(|&i| candidate[i] != expected[i]) {
        let lo = i.saturating_sub(4);
        let hi = (i + 12).min(candidate.len());
        panic!(
            "{what}: first diff at region offset {i:#x} (record {}, field-off {:#x})\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected [{lo:#x}..{hi:#x}]: {:02x?}",
            i / 26, i % 26, &candidate[lo..hi], &expected[lo..hi]
        );
    }
}

#[test]
fn objdefs_match_reference_plain() {
    use sigil_harness::pins;
    objdef_reference_gate(&ObjShape {
        region_base: pins::OBJDEFS.plain_base,
        region_len: pins::OBJDEFS.plain_len,
        map_test_obj: pins::MAP_TEST_OBJ.plain,
        rom: "s4.bin",
    });
}

#[test]
fn objdefs_match_reference_debug() {
    use sigil_harness::pins;
    objdef_reference_gate(&ObjShape {
        region_base: pins::OBJDEFS.debug_base,
        region_len: pins::OBJDEFS.debug_len,
        map_test_obj: pins::MAP_TEST_OBJ.debug,
        rom: "s4.debug.bin",
    });
}
