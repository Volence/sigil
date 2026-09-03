//! Tranche 22 (file 3) — the REAL `compression_selftest.emp` port,
//! region-level byte gate. THE CAMPAIGN'S FIRST DEBUG-ONLY REGION.
//!
//! Compiles the actual ported file — `engine/debug/compression_selftest.emp`
//! — through the production multi-module pipeline and asserts the
//! `compression_selftest` region's bytes equal the DEBUG reference ROM window.
//! The twin is whole-file `ifdef __DEBUG__`: the region exists ONLY in the
//! debug shape (`pins::COMPRESSION_SELFTEST.plain_len == 0`), so the byte gate
//! has one shape arm; the plain-shape proof (the gate emits NOTHING into the
//! plain ROM) rides the `mixed_tranche22_rom` full-ROM arm.
//!
//! ## Ownership after conv-i8 (#8)
//! The five golden vectors are now `.emp`-native: `compression_selftest.emp`
//! `embed()`s each blob (`CSelf_S4LZ_Plain` … `CSelf_Expected` are its own
//! `pub data` labels), and the three generated VALUES (`CSELF_PAYLOAD_SIZE` /
//! `CSELF_PAYLOAD_SUM` / `CSELF_DICT_LEN`) are `use`d from the generated
//! `engine.compression_vectors` module (`engine/debug/generated/compression_vectors.emp` —
//! its sole author, per tools/gen_compression_vectors.py). So the module
//! SELF-RESOLVES its data + constants: this gate builds it through the real
//! manifest (`use` resolution + `embed` reads), no synthetic label/value
//! injection. Only the genuine cross-seam CARRIERS stay injected:
//! `Art_Staging_Buffer`, the two `.emp`-owned callees `S4LZ_DecompressDict` /
//! `Art_Decompress`, and the two MDDBG assert handlers.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test compression_selftest_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::LowerOptions;
use sigil_frontend_emp::parse_file;
use sigil_frontend_emp::resolve::manifest::{Manifest, ParsedModule};
use sigil_frontend_emp::resolve::{build_program_open_embed, place_sections};
use sigil_harness::pins;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    sigil_harness::test_support::aeon_dir()
}

#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}

/// The cross-seam ADDRESS carriers that stay external to this module, each a
/// `phase`d one-byte AS label at its real debug VMA. (The five `CSelf_*` labels
/// are NO LONGER here — the module owns them as `pub data`.)
fn addr_labels() -> Vec<Section> {
    let table: Vec<(&str, u32)> = vec![
        ("Art_Staging_Buffer", pins::ART_STAGING_BUFFER.debug),
        ("S4LZ_DecompressDict", pins::S4_LZ_DECOMPRESS_DICT.debug),
        // F-6: Art_Decompress lives in THIS module now; its rare arm tail-jumps
        // the cross-seam S4LZ_Decompress.
        ("S4LZ_Decompress", pins::S4_LZ_DECOMPRESS.debug),
        // Art-streaming P2a Task 2 — the ZX0-vs-ZX0R act-pool equivalence walk
        // (gated HAS_ACT_ART_POOL, injected below) calls both decoders directly and
        // names buffer B + the act descriptor as cross-seam addresses. ZX0R region
        // start = ZX0R_Decompress; act-descriptor region start = OJZ_Act1_Descriptor.
        ("ZX0R_Decompress", pins::ZX0_RESUME.debug_base),
        ("Block_Stage_Buffers", pins::BLOCK_STAGE_BUFFERS.debug),
        ("OJZ_Act1_Descriptor", pins::ACT_DESCRIPTOR.debug_base),
        ("MDDBG__ErrorHandler", pins::MDDBG_ERROR_HANDLER),
        ("MDDBG__ErrorHandler_PagesController", pins::MDDBG_ERROR_HANDLER_PAGES_CONTROLLER),
    ];
    let mut out = Vec::new();
    for (i, (name, vma)) in table.iter().enumerate() {
        let vma = *vma;
        let asm = format!("cpu 68000\n\tphase ${vma:X}\n{name}:\n\tdc.b 0\n");
        let opts = AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() };
        let mut secs = assemble(&asm, &opts)
            .unwrap_or_else(|d| panic!("AS assemble ({name}): {d:?}"))
            .sections;
        for mut s in secs.drain(..) {
            s.lma = 0x0200_0000 + (i as u32) * 0x1_0000;
            s.placement = SectionPlacement::Pinned;
            s.group = None;
            out.push(s);
        }
    }
    out
}

/// A whole-tree manifest scan plus the synthetic entry `use
/// engine.compression_selftest`. `doctor_sum`, when set, REPLACES the scanned
/// `engine.compression_vectors` module with an in-memory one carrying a
/// different `CSELF_PAYLOAD_SUM` — the negative probe that the emitted assert
/// immediate genuinely rides the generated value.
fn scanned_manifest(aeon: &Path, doctor_sum: Option<&str>) -> (Manifest, String) {
    let (mut manifest, mdiags) = Manifest::scan(aeon);
    assert!(
        mdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "manifest scan errors: {mdiags:?}"
    );

    if let Some(sum) = doctor_sum {
        let idx = *manifest
            .by_id
            .get("engine.compression_vectors")
            .expect("engine.compression_vectors must be scanned (the generated compression_vectors.emp)");
        let src = format!(
            "module engine.compression_vectors\n\npub const CSELF_PAYLOAD_SIZE = 744\npub const CSELF_PAYLOAD_SUM = {sum}\npub const CSELF_DICT_LEN = 256\n"
        );
        let source = sigil_span::SourceId((manifest.modules.len() + 7) as u32);
        let (file, pdiags) = parse_file(&src, source);
        assert!(
            pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
            "doctored compression_vectors parse errors: {pdiags:?}"
        );
        manifest.modules[idx].file = file;
    }

    let entry_id = "compression_selftest_gate_entry".to_string();
    let src = "module compression_selftest_gate_entry\n\nuse engine.compression_selftest\n";
    let source = sigil_span::SourceId(manifest.modules.len() as u32);
    let (file, pdiags) = parse_file(src, source);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "entry parse errors: {pdiags:?}"
    );
    let idx = manifest.modules.len();
    manifest.by_id.insert(entry_id.clone(), idx);
    manifest.sources.insert(source, aeon.join("__compression_selftest_gate_entry__.emp"));
    manifest.modules.push(ParsedModule {
        id: entry_id.clone(),
        file,
        path: aeon.join("__compression_selftest_gate_entry__.emp"),
    });
    (manifest, entry_id)
}

/// Build `compression_selftest.emp` (+ its `use`d generated constants) through
/// the real manifest, place the `compression_selftest` section at `base`
/// (`len` bytes), inject the remaining cross-seam carriers, and link. Returns
/// the resolved sections + linked image + the module's link asserts (the abs.w
/// fit-lock). `doctor_sum` feeds the negative probe.
fn compile_at(
    base: u32,
    len: usize,
    doctor_sum: Option<&str>,
) -> (Vec<Section>, sigil_link::LinkedImage, Vec<sigil_ir::LinkAssert>) {
    let aeon = aeon_dir();
    let (manifest, entry_id) = scanned_manifest(&aeon, doctor_sum);

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(aeon.clone()),
        embed_base: Some(aeon.clone()),
        defines: vec![
            ("DEBUG".to_string(), 1),
            ("SOUND_DRIVER_ENABLED".to_string(), 1),
            ("HAS_ACT_ART_POOL".to_string(), 1), // sonic4 ships the act pool (P2a Task 2 eq walk)
        ],
    };
    let aeon_root = aeon.clone();
    let embed_base_for = move |_id: &str| -> Option<PathBuf> { Some(aeon_root.clone()) };
    let (mut sections, link_asserts, bdiags) =
        build_program_open_embed(&manifest, &entry_id, None, &opts, &embed_base_for);
    assert!(
        bdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "build_program errors: {:?}",
        bdiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );

    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n[[region]]\nname = \"compression_selftest\"\nlma_base = {base:#x}\nsize = {len:#x}\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors: {pdiags:?}"
    );

    sections.extend(addr_labels());

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked, link_asserts)
}

/// The debug-shape byte gate: the .emp code+data region vs the shipped
/// s4.debug.bin window.
#[test]
fn compression_selftest_debug_region_matches_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM not at {} (set AEON_DIR)", rom_path.display());
        return;
    };

    let base = pins::COMPRESSION_SELFTEST.debug_base;
    let len = pins::COMPRESSION_SELFTEST.debug_len;
    let (resolved, linked, link_asserts) = compile_at(base, len, None);
    let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "the module's link asserts must PASS at the real addresses (the abs.w \
         fit-lock retired at cd8cd15 — any surviving assert is not it): {diags:?}"
    );
    let expected = &refrom[base as usize..base as usize + len];
    let section =
        linked.section("compression_selftest").expect("linked image must carry the region");
    // Packed placement: the pins span runs to the NEXT section's aligned base, so the
    // window can end in ALIGNMENT FILL that the standalone compile does not emit. Same
    // tolerance the other region gates carry. (Item 28's +2 in `bg` shifted this region
    // and grew its tail from 0 to 14 bytes, which is what surfaced this.)
    assert!(
        section.bytes.len() <= expected.len(),
        "compression_selftest (debug): candidate {} bytes OVERRUNS the pinned window {}",
        section.bytes.len(),
        expected.len()
    );
    let fill = expected.len() - section.bytes.len();
    // Raised 16 -> 0x20 for art-streaming-p2-task6 (2026-08-08): the page_cache
    // section insertion shifted this region onto a coarser alignment boundary,
    // growing the all-zero tail to 0x10 bytes (same class of fill as bg's item-28
    // growth). native_full_rom proves the real bytes byte-exact; all-zero still required.
    assert!(
        fill <= 0x20 && expected[section.bytes.len()..].iter().all(|&b| b == 0),
        "compression_selftest (debug): {fill}-byte window tail is not short all-zero \
         alignment fill — that is a real length mismatch, not packing"
    );
    let expected = &expected[..section.bytes.len()];
    if let Some(i) = (0..expected.len()).find(|&i| section.bytes[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(expected.len());
        panic!(
            "compression_selftest (debug): first diff at {i:#x}\n  candidate[{lo:#x}..{hi:#x}]: {:02x?}\n  expected[{lo:#x}..{hi:#x}]:  {:02x?}",
            &section.bytes[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// The debug-only-region shape fact, pinned: the plain shape carries ZERO
/// bytes (the full-ROM proof that the gated plain build emits nothing is the
/// `mixed_tranche22_rom` arm).
#[test]
fn compression_selftest_plain_region_is_empty() {
    assert_eq!(pins::COMPRESSION_SELFTEST.plain_len, 0, "debug-only region: plain must be empty");
}

/// Negative probe: a DOCTORED `CSELF_PAYLOAD_SUM` truth must CHANGE the
/// emitted bytes (the assert immediates genuinely ride the generated value —
/// a hardcoded mirror would keep matching and hide generator drift).
#[test]
fn doctored_cself_sum_diverges_from_reference() {
    let aeon = aeon_dir();
    let rom_path = aeon.join("s4.debug.bin");
    let Ok(refrom) = std::fs::read(&rom_path) else {
        if strict_gate() {
            panic!("SIGIL_STRICT_GATE set but reference missing: {}", rom_path.display());
        }
        eprintln!("skip: reference ROM missing");
        return;
    };
    let base = pins::COMPRESSION_SELFTEST.debug_base;
    let len = pins::COMPRESSION_SELFTEST.debug_len;
    let (_, linked, _) = compile_at(base, len, Some("$1234"));
    let expected = &refrom[base as usize..base as usize + len];
    let section =
        linked.section("compression_selftest").expect("linked image must carry the region");
    assert_ne!(
        section.bytes, expected,
        "a doctored CSELF_PAYLOAD_SUM must change the emitted bytes — the value seam is dead if this matches"
    );
}

/// The abs.w FIT-LOCK is RETIRED (aeon cd8cd15, 2026-08-03): the
/// bug005-sprites-player parcel's DEBUG growth pushed Config-A's CSelf_*
/// labels past $8000 — the exact crossing the ensure existed to make
/// conscious — and the ruling ACCEPTED the per-shape abs.l widening
/// (boot-time cold path; the width rule handles either side). This probe's
/// old form asserted the ensure fires past $8000; its successor asserts the
/// NEW contract: the module lowers and links CLEANLY with vectors past the
/// window, and the bare CSelf_* operands genuinely width-select — a
/// below-window placement emits the shorter abs.w encodings, an
/// above-window placement the longer abs.l ones.
#[test]
fn retired_fit_lock_stays_silent_and_operands_width_select_past_abs_w() {
    // The ported module plus the generated values it `use`s — the two sources this
    // probe compiles. It opens no ROM: both arms are placement/link-only.
    if sigil_harness::test_support::reference_tree(&[
        "engine/debug/compression_selftest.emp",
        "engine/debug/generated/compression_vectors.emp",
    ])
    .is_none()
    {
        return;
    }
    // Below the window: every CSelf_* label sits under $8000 → abs.w.
    let low_len = {
        let (resolved, linked, link_asserts) =
            compile_at(0x4000, pins::COMPRESSION_SELFTEST.debug_len, None);
        let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "below-window placement must link cleanly: {diags:?}"
        );
        linked.section("compression_selftest").expect("region").bytes.len()
    };
    // Past the window (the old probe's base): CSelf_Expected lands beyond
    // $8000. The retired ensure must stay SILENT and the link must succeed —
    // with the operands widened to abs.l.
    let high_len = {
        let (resolved, linked, link_asserts) =
            compile_at(0x7800, pins::COMPRESSION_SELFTEST.debug_len + 0x40, None);
        let diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
        assert!(
            diags.iter().all(|d| d.level != sigil_span::Level::Error),
            "the retired fit-lock must stay silent past $8000 (cd8cd15): {diags:?}"
        );
        linked.section("compression_selftest").expect("region").bytes.len()
    };
    assert!(
        high_len > low_len,
        "the bare CSelf_* operands must width-select abs.l past $8000 \
         (high placement {high_len:#x} must out-size low placement {low_len:#x})"
    );
}
