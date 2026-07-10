//! Tranche 6 negative probes — each proves a tranche-6 guard/seam FAILS LOUD
//! when doctored, against an undoctored control that succeeds (no
//! false-comfort: a probe that "fails" for an unrelated reason would pass
//! vacuously, so every doctored run pairs with a resolving control through
//! the same plumbing).
//!
//! 1. A MISSPELLED cross-seam SST extern dangles loud at link (the sst.emp
//!    drift guards genuinely read the AS-side struct-equ seam).
//! 2. A DRIFTED sst.emp twin (two adjacent u8 fields swapped — compiles
//!    clean, dense layout intact) is caught by ITS OWN drift guard naming
//!    the field, BEFORE any consumer emits wrong displacements.
//! 3. A `.w` ImmLink whose link-folded value overflows the unsigned 16-bit
//!    window is loud (Value16Be totality), on BOTH frontends' word-value
//!    surfaces — the emp `.w` immediate AND the AS `dc.w` compound deferral
//!    (parity by construction on the tranche-6 surface; the F5
//!    comptime-truncation parity gap is separate and stays ledgered).
//! 4. A MISSPELLED objroutine target in the AS-side consumer shape
//!    (`dc.w TestSolid_Innit-ObjCodeBase`) dangles loud while the correctly
//!    spelled control resolves.
//! 5. A REORDERED `falls_into` pair (Main moved above Init) fails the
//!    compile — the Init→Main adjacency is enforced, not accidental.

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

fn read_aeon(rel: &str) -> Option<String> {
    std::fs::read_to_string(aeon_dir().join(rel)).ok()
}

/// Lower one synthetic file (deps' items prepended under `main`'s header) to
/// sections + link asserts. Panics on parse errors (the probes doctor
/// SEMANTICS, never syntax); returns lower diags for probes that expect
/// lower-time failures.
fn lower_with_ambient(
    dep_srcs: &[&str],
    main_src: &str,
) -> (
    Vec<Section>,
    Vec<sigil_ir::LinkAssert>,
    Vec<sigil_span::Diagnostic>,
) {
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
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    (module.sections, module.link_asserts, ldiags)
}

/// Place `sections` (the emp side) at the bank addresses, add the AS-side
/// truth equs + any extra synthetic sections, resolve + link, and return the
/// link-assert check diagnostics + whether resolve/link itself failed.
struct LinkOutcome {
    resolve_link_errors: Vec<String>,
    assert_errors: Vec<String>,
}

fn link_with_truths(
    mut sections: Vec<Section>,
    link_asserts: &[sigil_ir::LinkAssert],
    extra: Vec<Vec<Section>>,
) -> LinkOutcome {
    let map_toml = "fill = 0x00\n\
                    \n\
                    [[region]]\n\
                    name = \"text\"\n\
                    lma_base = 0x0000\n\
                    size = 0x10\n\
                    kind = \"rom\"\n\
                    \n\
                    [[region]]\n\
                    name = \"test_solid\"\n\
                    lma_base = 0x10F7C\n\
                    size = 0xE\n\
                    kind = \"rom\"\n\
                    \n\
                    [[region]]\n\
                    name = \"test_particle\"\n\
                    lma_base = 0x10F8A\n\
                    size = 0x5A\n\
                    kind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );
    let mut lma = 0x0100_0000u32;
    for mut group in extra {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(group);
        lma += 0x10_0000;
    }
    let resolved = match sigil_link::resolve_layout(&sections, &SymbolTable::new(), true) {
        Ok(r) => r,
        Err(diags) => {
            return LinkOutcome {
                resolve_link_errors: diags.iter().map(|d| d.message.clone()).collect(),
                assert_errors: vec![],
            }
        }
    };
    let resolve_link_errors = match sigil_link::link(&resolved, &SymbolTable::new()) {
        Ok(_) => vec![],
        Err(diags) => diags.iter().map(|d| d.message.clone()).collect(),
    };
    let assert_errors = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), link_asserts)
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error)
        .map(|d| d.message.clone())
        .collect();
    LinkOutcome { resolve_link_errors, assert_errors }
}

/// The AS-side truth equs (the real structs.asm/constants.asm values).
fn as_truth_equs() -> Vec<Section> {
    let asm = "cpu 68000\n\
               SST_code_addr = $00\n\
               SST_x_pos = $02\n\
               SST_y_pos = $06\n\
               SST_x_vel = $0A\n\
               SST_y_vel = $0C\n\
               SST_render_flags = $0E\n\
               SST_collision_resp = $0F\n\
               SST_mappings = $10\n\
               SST_art_tile = $14\n\
               SST_width_pixels = $16\n\
               SST_height_pixels = $17\n\
               SST_anim = $18\n\
               SST_subtype = $19\n\
               SST_anim_table = $1A\n\
               SST_status = $1E\n\
               SST_angle = $1F\n\
               SST_prev_anim = $20\n\
               SST_anim_frame = $21\n\
               SST_anim_timer = $22\n\
               SST_mapping_frame = $23\n\
               SST_prev_frame = $24\n\
               SST_sprite_piece_count = $25\n\
               SST_parent_ptr = $26\n\
               SST_sibling_ptr = $28\n\
               SST_slot_tag = $2A\n\
               SST_entity_section_id = $2B\n\
               SST_entity_list_index = $2C\n\
               SST_layer = $2D\n\
               SST_sst_custom = $2E\n\
               SST_len = $50\n\
               ObjCodeBase = $10000\n\
               Stub:\n\
               \tdc.w 0\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (truth equs): {d:?}")).sections
}

fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// Compile the REAL test_solid.emp (with the real sst.emp ambient) and link
/// against the truths — the resolving CONTROL every doctored probe pairs
/// with. Extra synthetic sections ride along per probe.
fn solid_outcome(sst_src: &str, solid_src: &str, extra: Vec<Vec<Section>>) -> LinkOutcome {
    let (sections, asserts, ldiags) = lower_with_ambient(&[sst_src], solid_src);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "unexpected lower errors: {ldiags:?}"
    );
    let mut all = vec![as_truth_equs(), as_label_at("Draw_Sprite", 0x2970)];
    all.extend(extra);
    link_with_truths(sections, &asserts, all)
}

#[test]
fn misspelled_sst_extern_dangles_loud_while_control_resolves() {
    let Some(sst) = read_aeon("engine/objects/sst.emp") else {
        eprintln!("skip: aeon tree not present");
        return;
    };
    let solid = read_aeon("games/sonic4/objects/test_solid.emp").unwrap();

    // Control: the real pair resolves clean.
    let control = solid_outcome(&sst, &solid, vec![]);
    assert!(
        control.resolve_link_errors.is_empty() && control.assert_errors.is_empty(),
        "control must resolve clean: link={:?} asserts={:?}",
        control.resolve_link_errors,
        control.assert_errors
    );

    // Doctored: one guard's extern name misspelled — the assert's symbol
    // dangles and the check is LOUD (never silently skipped).
    let doctored_sst = sst.replace("extern(\"SST_subtype\")", "extern(\"SST_subtypo\")");
    assert_ne!(doctored_sst, sst, "the doctor must have found its target");
    let outcome = solid_outcome(&doctored_sst, &solid, vec![]);
    assert!(
        outcome
            .assert_errors
            .iter()
            .any(|m| m.contains("SST_subtypo")),
        "a misspelled cross-seam extern must dangle LOUD naming the symbol: {:?}",
        outcome.assert_errors
    );
}

#[test]
fn drifted_sst_twin_fires_its_own_guard_naming_the_field() {
    let Some(sst) = read_aeon("engine/objects/sst.emp") else {
        eprintln!("skip: aeon tree not present");
        return;
    };
    let solid = read_aeon("games/sonic4/objects/test_solid.emp").unwrap();

    // The drift: swap the two adjacent u8 fields `anim` ($18) and `subtype`
    // ($19) in the twin ONLY — dense layout stays valid, the module compiles,
    // and consumers would emit wrong displacements... but the twin's own
    // guards fire first, naming both drifted fields.
    let doctored = sst
        .replace("anim: u8 @ $18,", "subtype: u8 @ $18,")
        .replace("subtype: u8 @ $19,", "anim: u8 @ $19,")
        .replace(
            "ensure(extern(\"SST_anim\") == $18,",
            "ensure(extern(\"SST_anim\") == $19,",
        )
        .replace(
            "ensure(extern(\"SST_subtype\") == $19,",
            "ensure(extern(\"SST_subtype\") == $18,",
        );
    assert_ne!(doctored, sst, "the doctor must have found its targets");
    let outcome = solid_outcome(&doctored, &solid, vec![]);
    assert!(
        outcome.assert_errors.iter().any(|m| m.contains("subtype"))
            && outcome.assert_errors.iter().any(|m| m.contains("anim")),
        "a drifted twin must be caught by its OWN guards naming the fields: {:?}",
        outcome.assert_errors
    );
}

#[test]
fn word_imm_link_range_violation_is_loud_on_both_frontends() {
    // emp side: a `.w` link immediate whose folded value ($12345) overflows
    // the unsigned 16-bit window — Value16Be totality, loud at link.
    let emp = "module m in test_solid\n\
               equ BIG = extern(\"Huge\")\n\
               pub proc P () {\n\
               \tmove.w  #BIG, d0\n\
               \trts\n\
               }\n";
    let (sections, asserts, ldiags) = lower_with_ambient(&[], emp);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "unexpected lower errors: {ldiags:?}"
    );
    let huge = {
        let asm = "cpu 68000\nHuge = $12345\nStub:\n\tdc.w 0\n";
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
    };
    let outcome = link_with_truths(sections, &asserts, vec![huge]);
    assert!(
        !outcome.resolve_link_errors.is_empty(),
        "an overflowing .w link immediate must fail the link"
    );

    // AS side, same window: a deferred `dc.w` compound folding out of range
    // is equally loud (parity by construction — both are Value16Be).
    let consumer = {
        let asm = "cpu 68000\nConsumer:\n\tdc.w Huge-Base\n";
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
    };
    let truths = {
        let asm = "cpu 68000\nHuge = $23456\nBase = $1\nStub:\n\tdc.w 0\n";
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
    };
    let mut sections = Vec::new();
    let mut lma = 0x0100_0000u32;
    for mut group in [consumer, truths] {
        for sec in group.iter_mut() {
            sec.lma = lma;
            sec.placement = SectionPlacement::Pinned;
            sec.group = None;
        }
        sections.extend(group);
        lma += 0x10_0000;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .expect("layout resolves — the range check is the LINK's");
    assert!(
        sigil_link::link(&resolved, &SymbolTable::new()).is_err(),
        "an overflowing deferred dc.w value must fail the link"
    );
}

#[test]
fn misspelled_objroutine_target_dangles_while_control_resolves() {
    let Some(sst) = read_aeon("engine/objects/sst.emp") else {
        eprintln!("skip: aeon tree not present");
        return;
    };
    let solid = read_aeon("games/sonic4/objects/test_solid.emp").unwrap();

    let consumer = |target: &str| -> Vec<Section> {
        let asm = format!("cpu 68000\nConsumer:\n\tdc.w {target}-ObjCodeBase\n");
        let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
        assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
    };

    // Control: the real consumer spelling resolves through the shared link.
    let control = solid_outcome(&sst, &solid, vec![consumer("TestSolid_Init")]);
    assert!(
        control.resolve_link_errors.is_empty(),
        "the undoctored objroutine consumer must resolve: {:?}",
        control.resolve_link_errors
    );

    // Doctored: a typo'd label dangles loud at link.
    let outcome = solid_outcome(&sst, &solid, vec![consumer("TestSolid_Innit")]);
    assert!(
        outcome
            .resolve_link_errors
            .iter()
            .any(|m| m.contains("TestSolid_Innit")),
        "a misspelled objroutine target must dangle LOUD naming the symbol: {:?}",
        outcome.resolve_link_errors
    );
}

#[test]
fn reordered_falls_into_pair_fails_compile() {
    let Some(sst) = read_aeon("engine/objects/sst.emp") else {
        eprintln!("skip: aeon tree not present");
        return;
    };
    let solid = read_aeon("games/sonic4/objects/test_solid.emp").unwrap();

    // The doctor: move TestSolid_Main ABOVE TestSolid_Init by swapping the
    // two proc declarations (comments and all), leaving the `falls_into`
    // annotation pointing at a proc that no longer follows it.
    let init_start = solid.find("pub proc TestSolid_Init").expect("Init proc present");
    let main_start = solid.find("pub proc TestSolid_Main").expect("Main proc present");
    assert!(init_start < main_start, "source order sanity");
    let head = &solid[..init_start];
    let init_block = &solid[init_start..main_start];
    let main_block = &solid[main_start..];
    let doctored = format!("{head}{main_block}{init_block}");

    let (_, _, ldiags) = lower_with_ambient(&[&sst], &doctored);
    assert!(
        ldiags.iter().any(|d| d.level == sigil_span::Level::Error
            && d.message.contains("falls_into")),
        "a reordered falls_into pair must FAIL the compile naming the contract: {ldiags:?}"
    );
}
