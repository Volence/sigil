//! Tranche 9 — the REAL `animate.emp` port, region-level byte gate.
//!
//! `rings_port.rs`'s sibling for the NINTH code port: compiles the ACTUAL
//! ported file from aeon's tree — `engine/objects/animate.emp` — through the
//! production parse -> lower -> place -> resolve -> link pipeline, and asserts
//! the `animate` section's flattened bytes equal the reference ROM window at
//! the pinned addresses, in BOTH build shapes.
//!
//! ## What this port exercises that the prior eight did not
//!
//! - **Local-label ARITHMETIC in a pc-indexed EA** — the control-code
//!   dispatch `jmp .cc_table-4(pc,d0.w)` (×2): the d8 slot carries
//!   `label - 4`, and the -4 lands INSIDE the jmp instruction itself, so no
//!   relocated label can absorb it.
//! - **A cross-proc local-label reference** — the PerFrame dispatch table's
//!   $FB entry is `bra.w AnimateSprite.cc_delete` (the shared delete stub
//!   lives in the OTHER interpreter's scope). Spec §5's `ProcName.label`
//!   surface, first real consumer.
//! - **Cross-seam width relaxation, both directions** — `.cc_delete`'s
//!   `jbra DeleteObject` (step 2's static-tail-call spelling; step 1's
//!   `jmp` landed abs.w `4EF8`) must relax to `bra.w` with the per-shape
//!   displacement, and the outbound consumer proves the mirror image: a
//!   bare `jsr AnimateSprite` from an AS unit (undefined in-unit) relaxes
//!   to abs.w.
//! - **A comptime-fn template with MEMORY operands and a module const** —
//!   `reload_anim_timer(src: Reg)` expands `Sst.anim_timer(a0)` +
//!   `#DUR_DYNAMIC` inside `asm {}` (aabb only spliced registers/labels).
//!   The AS twin's macro `tag` uniquifier param has no counterpart —
//!   template labels are hygienic (the utag-death pattern, third exhibit).
//!
//! ## Cross-seam symbols
//!
//! INBOUND equs (values): the SST_* struct-equ seam + the engine constants
//! twin (30 after this tranche's animation-block growth — the AF_* truth
//! re-homed to `engine/constants.asm` so script DATA files survive the
//! `SIGIL_EMP_ANIMATE` gate; kill-list row 2). Animate owns NO game-side or
//! module-local mirrors and touches NO RAM cells — the leanest inbound set
//! of the campaign. INBOUND labels at true per-shape VMAs: `DeleteObject`
//! and `Sound_PlaySFX` (the latter referenced only when
//! SOUND_DRIVER_ENABLED=1).
//!
//! OUTBOUND: all three procs are `pub` (`AnimateSprite` called by
//! player_common + the test objects; `AnimateSprite_PerFrame` currently has
//! ZERO callers — ported faithfully, flagged in the tranche packet;
//! `RefreshSpritePieceCount` is intra-module today). The consumer probe
//! mirrors player_common's bare `jsr AnimateSprite` and must land on the
//! abs.w encoding (`4EB8 base`) for mixed-build parity.
//!
//! ## Reference windows (2026-07-10 pins, from the master listings)
//!
//! Plain (map base `$2D78`): `s4.bin[0x2D78..0x3080]` (0x308 bytes).
//! Debug (map base `$3032`): `s4.debug.bin[0x3032..0x333A]` (0x308 bytes).
//! Length is shape-INVARIANT (no `__DEBUG__` code in this file).
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, the gates SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test animate_port
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

/// Per-shape geometry + TRUE cross-seam VMAs (2026-07-10 pins, both listings).
struct Shape {
    base: u32,
    len: usize,
    /// `(name, vma)` for every INBOUND label this shape references.
    labels: &'static [(&'static str, u32)],
}

/// Region-relative proc/anchor offsets — shape-INVARIANT (equal length, no
/// conditional code), so shared constants rather than `Shape` fields.
/// (Step 2 shrank the region 0x312 → 0x308: bare Bcc/jbra relaxation found
/// five suboptimal hand widths; every offset below is from the re-derived
/// listings.)
/// `.cc_delete`'s `jbra DeleteObject` (bra.w `6000 xxxx`).
const CC_DELETE_OFF: usize = 0x104;
/// `AnimateSprite_PerFrame:` (listing `$2EEC`/`$31A6`).
const PERFRAME_OFF: usize = 0x174;
/// `RefreshSpritePieceCount:` (listing `$3062`/`$331C`).
const REFRESH_OFF: usize = 0x2EA;

const PLAIN: Shape = Shape {
    base: 0x2D78,
    len: 0x308,
    labels: &[("DeleteObject", 0x281C), ("Sound_PlaySFX", 0x5E5C)],
};

const DEBUG: Shape = Shape {
    base: 0x3032,
    len: 0x308,
    labels: &[("DeleteObject", 0x29AE), ("Sound_PlaySFX", 0x731A)],
};

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

/// The AS-side value seam: SST struct equs + the engine constants twin's 30.
/// `override_pair` doctors exactly one entry (the drift-probe seam — see
/// `doctored_twin_mirror_fires_its_guard`).
fn as_constant_equs_with(override_pair: Option<(&str, &str)>) -> Vec<Section> {
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    if let Some((name, rhs)) = override_pair {
        let slot = pairs
            .iter_mut()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("override: `{name}` is not in the equ blob"));
        slot.1 = rhs;
    }
    sigil_harness::test_support::assemble_equ_pairs(&pairs)
}

/// One synthetic AS-side label phased at `vma` — a `dc.b 0` carrier whose LABEL
/// address is load-bearing (the abs.w `jmp DeleteObject` operand and the
/// `bsr.w Sound_PlaySFX` displacement must resolve to the real per-shape
/// addresses).
fn as_label_at(name: &str, vma: u32) -> Vec<Section> {
    let asm = format!("cpu 68000\nphase ${vma:X}\n{name}:\n\tdc.b 0\n");
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(&asm, &opts).unwrap_or_else(|d| panic!("AS assemble (synthetic {name}): {d:?}")).sections
}

/// The AS-side OUTBOUND consumer — mirrors player_common.asm's bare
/// `jsr AnimateSprite`, assembled with the label UNDEFINED in-unit (the
/// `.emp` owns it). Proves the `pub proc` export surfaces as a bare link
/// symbol AND that the width relaxation lands on the abs.w encoding the
/// reference build uses (`4EB8 xxxx` — player_common assembles against a
/// KNOWN backward label there; the mixed build must reach the same bytes
/// through the seam).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tjsr     AnimateSprite\n\
               \trts\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// The map: a `text` region for the zero-byte default-section carrier, and the
/// real `animate` region pinned at the per-shape base.
fn map_toml(base: u32, len: usize) -> String {
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
         name = \"animate\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// Compile the real `animate.emp` with its ambient dependencies (types + sst +
/// constants) and the given build-shape defines, place it at the per-shape
/// base, append the synthetic cross-seam sections, and link.
fn compile_real_file(
    shape: &Shape,
    defines: &[(&str, i128)],
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    compile_real_file_with(shape, defines, None)
}

/// `compile_real_file` with the drift-probe equ-override seam exposed.
fn compile_real_file_with(
    shape: &Shape,
    defines: &[(&str, i128)],
    override_pair: Option<(&str, &str)>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let types = parse_file(&aeon.join("engine/system/types.emp"));
    let sst = parse_file(&aeon.join("engine/objects/sst.emp"));
    let constants = parse_file(&aeon.join("engine/system/constants.emp"));
    let animate = parse_file(&aeon.join("engine/objects/animate.emp"));

    let file = with_ambient(vec![types, sst, constants], animate);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.join("engine/objects")),
        embed_base: None,
        defines: defines.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "animate.emp lower errors: {ldiags:?}"
    );
    let link_asserts = module.link_asserts;

    let map = sigil_link::load_map(&map_toml(shape.base, shape.len)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    let mut lma = 0x0100_0000u32;
    let mut groups: Vec<Vec<Section>> = vec![as_constant_equs_with(override_pair)];
    for (name, vma) in shape.labels {
        groups.push(as_label_at(name, *vma));
    }
    groups.push(as_outbound_consumer());
    for group in &mut groups {
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

/// All prepended drift guards must be captured and PASS: sst.emp's 30 SST_*
/// pins + constants.emp's 30 (24 pre-tranche + the animation block's 6).
/// animate.emp itself carries ZERO module-local mirrors.
fn assert_drift_guards(resolved: &[Section], link_asserts: &[sigil_ir::LinkAssert]) {
    let guards = sigil_harness::test_support::guard_assert_count(link_asserts);
    let want = 30 + sigil_harness::test_support::engine_constant_equs().len();
    assert_eq!(
        guards, want,
        "sst.emp's 30 + constants.emp's {} drift guards must be captured",
        sigil_harness::test_support::engine_constant_equs().len()
    );
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

/// The region reference gate + cross-seam pins + the outbound bare-name
/// proof + the drift guards, shared body. Reference shapes always run
/// SOUND_DRIVER_ENABLED=1 (both pinned ROMs have sound on).
fn reference_gate(shape: &Shape, rom_name: &str) {
    let rom_path = aeon_dir().join(rom_name);
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let defines: Vec<(&str, i128)> = vec![("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, linked, link_asserts) = compile_real_file(shape, &defines);
    assert_drift_guards(&resolved, &link_asserts);

    let base = shape.base as usize;
    let section = linked.section("animate").expect("linked image must carry animate");
    assert_region_matches(
        &section.bytes,
        &refrom[base..base + shape.len],
        &format!("animate vs {rom_name}[{base:#x}..{:#x}]", base + shape.len),
    );

    // Cross-seam pin: `.cc_delete`'s `jbra DeleteObject` (step 2 — the static
    // tail-call house spelling; step 1 transcribed the original `jmp`, which
    // relaxed to abs.w `4EF8`) must relax to bra.w with the per-shape
    // displacement — same 4-byte length, so no region slide.
    let delete_obj = shape.labels.iter().find(|(n, _)| *n == "DeleteObject").unwrap().1;
    let disp =
        (delete_obj as i64 - (shape.base as i64 + CC_DELETE_OFF as i64 + 2)) as i16 as u16;
    assert_eq!(
        &section.bytes[CC_DELETE_OFF..CC_DELETE_OFF + 4],
        &[0x60, 0x00, (disp >> 8) as u8, disp as u8],
        "`jbra DeleteObject` must relax to bra.w with the per-shape displacement"
    );

    // Outbound bare-name proof: the AS-side bare `jsr AnimateSprite` must
    // relax to the abs.w encoding (`4EB8 base`) — player_common's shape in
    // the mixed build. The consumer is the LAST synthetic group: equ blob +
    // N labels + consumer.
    let consumer_lma = 0x0100_0000u32 + (1 + shape.labels.len() as u32) * 0x10_0000;
    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == consumer_lma)
        .expect("linked image must carry the outbound consumer at its harness-private LMA");
    assert_eq!(
        &consumer.bytes[0..4],
        &[0x4E, 0xB8, (shape.base >> 8) as u8, shape.base as u8],
        "bare-name proof: `jsr AnimateSprite` must relax to abs.w at the region base"
    );

    // Structural anchors: both secondary procs open with the shared
    // `andi.b #$F9, SST_render_flags(a0)` prologue (PerFrame) and
    // `movea.l SST_mappings(a0), a1` (Refresh) — offset drift would mean a
    // mid-region size change the whole-region diff already caught; these
    // name the spot for the re-pin sweep.
    assert_eq!(
        &section.bytes[PERFRAME_OFF..PERFRAME_OFF + 2],
        &[0x02, 0x28],
        "AnimateSprite_PerFrame must sit at +0x17A (andi.b #imm opword)"
    );
    assert_eq!(
        &section.bytes[REFRESH_OFF..REFRESH_OFF + 2],
        &[0x22, 0x68],
        "RefreshSpritePieceCount must sit at +0x2F4 (movea.l d16(a0) opword)"
    );
}

/// (plain) the `animate` region == `s4.bin[0x2D78..0x3080]`.
#[test]
fn animate_region_matches_reference() {
    reference_gate(&PLAIN, "s4.bin");
}

/// (debug) the `animate` region == `s4.debug.bin[0x3032..0x333A]`.
#[test]
fn animate_debug_region_matches_reference() {
    reference_gate(&DEBUG, "s4.debug.bin");
}

// ── The SND combo probe ─────────────────────────────────────────────────────

/// The AS-twin oracle for the SOUND_DRIVER_ENABLED dimension: animate.asm
/// assembled through the sigil AS front-end at the PLAIN base with the same
/// equ prelude the .emp gets, per-combo defines. animate.asm is include-free
/// (the reloadAnimTimer macro is defined in-file), so no include splicing —
/// the simplest oracle of the campaign.
fn as_twin_bytes(snd_on: bool) -> Vec<u8> {
    let aeon = aeon_dir();
    let animate_src = std::fs::read_to_string(aeon.join("engine/objects/animate.asm"))
        .expect("animate.asm must be readable");

    let mut prelude = String::from("cpu 68000\nsupmode on\n");
    let mut pairs = sigil_harness::test_support::sst_field_equs();
    pairs.extend(sigil_harness::test_support::engine_constant_equs());
    for (name, rhs) in pairs {
        prelude.push_str(&format!("{name} = {rhs}\n"));
    }
    for (name, vma) in PLAIN.labels {
        prelude.push_str(&format!("{name} = ${vma:X}\n"));
    }
    let src = format!("{prelude}org ${:X}\n{animate_src}\n", PLAIN.base);

    let mut defines: Vec<(String, i64)> = Vec::new();
    if snd_on {
        defines.push(("SOUND_DRIVER_ENABLED".to_string(), 1));
    }
    let opts = AsOptions { initial_cpu: Cpu::M68000, defines, ..AsOptions::default() };
    let out = assemble(&src, &opts).unwrap_or_else(|d| panic!("AS twin assemble: {d:?}"));
    let mut sections = out.sections;
    for sec in &mut sections {
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("AS twin resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS twin link failed: {d:?}"));
    let sec = linked
        .sections
        .iter()
        .find(|s| s.lma == PLAIN.base && !s.bytes.is_empty())
        .unwrap_or_else(|| panic!("AS twin must emit a section at {:#x}", PLAIN.base));
    sec.bytes.clone()
}

/// SND 0/1: the .emp vs the AS-twin oracle, module-level. This is the
/// conditional-MIRRORING drift guard (the oracle re-reads the real
/// animate.asm every run — a macro or conditional change AS-side that the
/// .emp doesn't mirror fails here naming the first diverging byte).
#[test]
fn snd_combo_matches_as_twin() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/animate.asm").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    for snd_on in [true, false] {
        let defines: Vec<(&str, i128)> =
            vec![("SOUND_DRIVER_ENABLED", i128::from(snd_on))];
        let (_, linked, _) = compile_real_file(&PLAIN, &defines);
        let section = linked.section("animate").expect("linked image must carry animate");
        let expected = as_twin_bytes(snd_on);
        assert_region_matches(
            &section.bytes,
            &expected,
            &format!("animate combo (snd={snd_on}) vs AS twin"),
        );
    }
}

// ── The twin-mirror drift probe (kill-list row 2's guard) ───────────────────

/// A DOCTORED twin truth (`AF_SET_FIELD` = $F6 AS-side while constants.emp
/// says $F7) must fire the twin's `ensure(extern(…))` guard NAMING the
/// constant — proving the tranche's SIX new animation-block guards ride
/// animate's gate like the originals, paired with the undoctored control
/// (the reference gates above). AF_SET_FIELD is the one whose drift would
/// silently re-classify frame bytes as control codes — the worst failure
/// mode this file has.
#[test]
fn doctored_twin_mirror_fires_its_guard() {
    let aeon = aeon_dir();
    if !aeon.join("engine/objects/animate.emp").exists() {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but aeon sources missing at {}", aeon.display());
        }
        eprintln!("skip: aeon sources not at {} (set AEON_DIR)", aeon.display());
        return;
    }
    let defines: Vec<(&str, i128)> = vec![("SOUND_DRIVER_ENABLED", 1)];
    let (resolved, _, link_asserts) =
        compile_real_file_with(&PLAIN, &defines, Some(("AF_SET_FIELD", "$F6")));
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    let fired: Vec<_> = diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect();
    assert!(
        !fired.is_empty(),
        "the doctored AF_SET_FIELD truth must fire constants.emp's drift guard"
    );
    assert!(
        fired.iter().any(|d| d.message.contains("AF_SET_FIELD")),
        "the fired guard must NAME the drifted constant: {fired:?}"
    );
}
