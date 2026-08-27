//! Port #2 Task 3 — the REAL `math.emp` port, region-level byte gate.
//!
//! `hblank_port.rs`'s sibling: compiles the ACTUAL ported file from aeon's
//! tree — `engine/system/math.emp` — through the production parse -> lower
//! -> place -> resolve -> link pipeline, and asserts the `math` section's
//! flattened bytes equal the reference ROM window at the pinned addresses,
//! in BOTH build shapes.
//!
//! ## `embed_base`: the first port whose `embed` climbs above its own dir
//!
//! Every prior port (`hblank.emp`, `dac_samples.emp`, `mt_bank.emp`, …) set
//! `include_root` to the module's OWN directory and never needed anything
//! more, because their `embed` paths were bare same-directory filenames.
//! `math.emp`'s `embed("../data/sine.bin")` breaks that pattern: it climbs
//! ONE level above its own directory (`engine/system/`) to a SIBLING
//! directory (`engine/data/`). The capability sandbox
//! (`sigil-frontend-emp/src/eval/sandbox.rs::resolve_sandbox_path`) enforces
//! containment against `include_root` alone — a `..` that pops back past
//! `include_root` itself is unconditionally `[sandbox.path-escape]`, and
//! that holds true NO MATTER what `include_root` is set to (with a single
//! root serving as both the join base and the boundary, one `..` always
//! escapes it, by construction). This is a REAL front-end gap this port
//! surfaced and fixed (front-end fix, TDD, small and clearly correct — see
//! the campaign gap ledger): `LowerOptions` now carries a second,
//! independent field, `embed_base`, which is the join BASE relative `embed`
//! paths resolve against; `include_root` stays the sole containment
//! boundary. Here `include_root` = `engine/` (broad enough to contain BOTH
//! the module's own directory and its embed target) and `embed_base` =
//! `engine/system/` (the module's own directory, matching every other
//! port's convention) — so `"../data/sine.bin"` joins onto `embed_base`,
//! climbs to `engine/`, descends to `engine/data/sine.bin`, and the FINAL
//! result is checked against `include_root` and passes.
//!
//! ## No shape define
//!
//! Like `hblank.emp`/`controllers.emp`, `math.emp` carries no `DEBUG`
//! member: the block's CONTENT (the module's code plus its embedded sine
//! table; see `pins::MATH` for the length) is byte-identical plain and debug — only
//! its BASE address shifts (see `pins::MATH`), so the shape
//! lives entirely in the MAP. `lower_module` runs with an EMPTY `defines`
//! vec for both shapes.
//!
//! ## No cross-seam INBOUND
//!
//! Unlike `controllers.emp`, `math.emp` is fully self-contained after the
//! `embed`: `GetSineCosine`'s only operand reference is `Sine_Table(pc,
//! d0.w)`, a PC-relative reference to the module's OWN `pub data` — no
//! external symbol is read.
//!
//! ## Cross-seam OUTBOUND (the bare-name proof)
//!
//! `GetSineCosine` is called from `games/sonic4/player/player_ground.asm`
//! (five call sites) and `games/sonic4/objects/test_parent.asm:96` (`jsr
//! GetSineCosine`). `Sine_Table` itself has no direct external `.asm`
//! consumer (every real reference is the module's own internal PC-relative
//! read), but this test still proves the `pub data` label surfaces as a bare
//! link symbol — mirroring the sound ports' `dc.l`/`dc.w` data-symbol proofs
//! — alongside the `pub proc` proof, both through one synthetic AS-side
//! consumer:
//!
//! - `jsr GetSineCosine` — the real `test_parent.asm:96` shape, an absolute
//!   jsr fixup.
//! - `dc.l Sine_Table` — a bare data-symbol reference, proving the table's
//!   label resolves cross-seam even though no real `.asm` file reads it
//!   directly today.
//!
//! ## Reference windows
//! (sourced from `sigil_harness::pins` — regenerate via repin)
//!
//! Both windows come from `pins::MATH` at run time — base and length, per
//! shape. The numbers are deliberately not restated here: a bound copied into
//! prose is executed by nothing, so nothing can go red when it rots.
//!
//! REFERENCE-DEPENDENT: needs the sibling `aeon` tree (`AEON_DIR`, default
//! `/home/volence/sonic_hacks/aeon`). Absent, both tests SKIP green — unless
//! `SIGIL_STRICT_GATE=1` makes a missing reference a hard failure.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test math_port
//! ```

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::pins;
use sigil_harness::test_support::{reference_tree, strict_gate};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

/// Per-shape region base (sourced from `sigil_harness::pins`).
fn region_base(debug: bool) -> u32 {
    if debug { pins::MATH.debug_base } else { pins::MATH.plain_base }
}

/// REFERENCE-DEPENDENT: `Some(aeon_root)` when the tree carries the three
/// sources this gate reads (`math.emp`, the ambient `types.emp`, and the
/// embedded `sine.bin`); `None` — both tests SKIP green — when it does not,
/// unless `SIGIL_STRICT_GATE=1` makes absence a hard failure.
fn math_tree() -> Option<PathBuf> {
    reference_tree(&["engine/system/math.emp", "engine/system/types.emp", "engine/data/sine.bin"])
}

/// The module's own directory in aeon's tree — where `math.emp` itself
/// lives, and the base for reading the source file.
fn math_dir(aeon: &Path) -> PathBuf {
    aeon.join("engine/system")
}

/// The sandbox's hard containment BOUNDARY — the module's PARENT directory
/// (`engine/`), broad enough to contain both `math.emp` itself
/// (`engine/system/`) and its embed target (`engine/data/`). Paired with
/// `math_embed_base` (below): `include_root` is the boundary,
/// `embed_base` is the join point relative `embed` paths climb FROM.
fn math_include_root(aeon: &Path) -> PathBuf {
    aeon.join("engine")
}

/// The `embed` join base — the module's OWN directory (`engine/system/`),
/// matching every other port's `include_root` convention. `math.emp`'s
/// `embed("../data/sine.bin")` joins onto THIS (not `include_root`), climbs
/// one level to `engine/`, then descends to `engine/data/sine.bin` — the
/// final resolved path is checked against `math_include_root`'s boundary
/// and passes (it's a descendant of `engine/`).
fn math_embed_base(aeon: &Path) -> PathBuf {
    math_dir(aeon)
}

/// The frozen reference ROMs (harness `golden/`), NOT the live tree `s4.bin`
/// (post-flip the tree ROM is sigil-canonical — the committed blob is truth).
fn golden(name: &str) -> Option<Vec<u8>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../sigil-harness/golden/{name}"));
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("golden missing: {}", path.display());
            }
            None
        }
    }
}

/// The map: a `text` region for the module's zero-byte default-section
/// carrier, and the real `math` region pinned at the per-shape reference
/// base, sized to `pins::MATH`'s length (the module's code plus its embedded
/// sine table). Only the region base differs from `hblank_port.rs`'s map
/// shape; both come from `pins::MATH`.
fn map_toml(debug: bool) -> String {
    let base = region_base(debug);
    let len = pins::MATH.plain_len;
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
         name = \"math\"\n\
         lma_base = {base:#x}\n\
         size = {len:#x}\n\
         kind = \"rom\"\n"
    )
}

/// The synthetic AS-side OUTBOUND consumer — THE BARE-NAME PROOF, for BOTH
/// `pub proc GetSineCosine` and `pub data Sine_Table`. Mirrors the real
/// `test_parent.asm:96` shape (`jsr GetSineCosine`, an absolute jsr fixup)
/// plus a `dc.l Sine_Table` data-symbol reference (the sound ports'
/// `dc.l`/`dc.w` proof shape, applied here even though no real `.asm` file
/// reads `Sine_Table` directly — every real reference is the module's own
/// internal PC-relative read).
fn as_outbound_consumer() -> Vec<Section> {
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tjsr     GetSineCosine\n\
               \tdc.l    Sine_Table\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (outbound consumer): {d:?}")).sections
}

/// Parse -> lower (with the module-dir include_root, NO defines) -> place
/// the `.emp` sections into the per-shape map -> append the synthetic
/// outbound-consumer section at a harness-private LMA (clear of both map
/// regions) -> ONE `resolve_layout` -> `link`. Returns the placed+resolved
/// `.emp` sections and the linked image.
fn compile_real_file(aeon: &Path, debug: bool) -> (Vec<Section>, sigil_link::LinkedImage) {
    let dir = math_dir(aeon);
    let emp_path = dir.join("math.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    // engine.types rides ambient (math.emp's `use` for the Angle param,
    // construct-walk #3 — the controllers_port ambient technique).
    let types_src = std::fs::read_to_string(dir.join("types.emp"))
        .unwrap_or_else(|e| panic!("cannot read types.emp: {e}"));
    let (types_file, tdiags) = parse_str(&types_src);
    assert!(
        tdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "types.emp parse errors: {tdiags:?}"
    );
    let file = sigil_frontend_emp::ast::File {
        module: file.module.clone(),
        attrs: file.attrs.clone(),
        items: types_file.items.into_iter().chain(file.items).collect(),
        docs: file.docs.clone(),
    };

    // NO defines: the math block is shape-invariant; the shape lives in the
    // map. include_root = the module's PARENT dir (`engine/`, the sandbox
    // boundary); embed_base = the module's OWN dir (`engine/system/`, the
    // join point) — see `math_include_root`/`math_embed_base`'s docs.
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(math_include_root(aeon)),
        embed_base: Some(math_embed_base(aeon)),
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed?): {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(debug)).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    // Append the synthetic outbound consumer at a harness-private LMA — well
    // clear of `text` ($0..$10) and `math` — so it cannot collide with
    // either map region.
    let mut consumer = as_outbound_consumer();
    for sec in &mut consumer {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked)
}

/// On mismatch, report the first differing offset plus 8 bytes of context on
/// each side.
fn assert_region_matches(candidate: &[u8], expected: &[u8], what: &str) {
    // A gate over an EMPTY image proves nothing, and the tolerance below would
    // hide that: with no candidate bytes it shrinks `expected` to zero length, the
    // length assert compares 0 == 0, and the diff loop runs over an empty range —
    // so the test passes if the module emits nothing at all. Confirmed live on
    // OJZ_BG_ANIM, a 14-byte all-zero plain window (lens sweep, seat GATE, S15).
    assert!(
        !candidate.is_empty(),
        "{what}: the module emitted NO BYTES — a region gate over an empty window \
         proves nothing. Either the module stopped emitting, or this pin should not exist."
    );
    // Packed placement (Wave-B B-0) may end a region window in ALIGNMENT FILL: the
    // pins span runs to the next section's aligned base. Tolerate a short (< 16 B)
    // all-zero tail beyond the lowered image; every real byte still compares.
    let expected = if expected.len() > candidate.len()
        && expected.len() - candidate.len() < 16
        && expected[candidate.len()..].iter().all(|&b| b == 0)
    {
        &expected[..candidate.len()]
    } else {
        expected
    };
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

/// (plain) The `math` section's linked bytes equal the `s4.bin` window at
/// `pins::MATH`'s plain base/len,
/// AND the outbound consumer's fixups resolve to the correct per-shape
/// addresses — the bare-name proof, for both `GetSineCosine` and
/// `Sine_Table`.
#[test]
fn math_region_matches_reference() {
    let Some(aeon) = math_tree() else { return };
    let Some(refrom) = golden("s4.bin") else {
        eprintln!("skip: golden s4.bin not present");
        return;
    };

    let (_resolved, linked) = compile_real_file(&aeon, false);

    let base = region_base(false) as usize;
    let expected = &refrom[base..base + pins::MATH.plain_len];
    let section = linked.section("math").expect("linked image must carry math");
    assert_region_matches(&section.bytes, expected, "math (plain) vs the s4.bin window at pins::MATH");

    // The bare-name proof: `jsr GetSineCosine`'s target (`pins::MATH.plain_base`) fits the
    // abs.w range, so the deferred `JmpJsrSym`'s relaxation ladder picks the
    // SHORT form — opcode `4EB8` + a 2-byte abs.w address at bytes [2..4).
    // GetSineCosine is the FIRST proc in the section, so it resolves to the
    // section base, $2464 (plain). `dc.l Sine_Table` follows at [4..8) —
    // Sine_Table starts right after the 24-byte code body, at $2464+24 =
    // $247C.
    let consumer = linked.section("sec0").expect("linked image must carry the outbound consumer");
    assert_eq!(
        &consumer.bytes[0..2],
        &[0x4E, 0xB8],
        "sanity: `jsr` opcode must be absolute-word form (4EB8) — $2464 fits abs.w"
    );
    assert_eq!(
        &consumer.bytes[2..4],
        &(base as u16).to_be_bytes(),
        "bare-name proof: `jsr GetSineCosine` must resolve to $2464 (plain)"
    );
    assert_eq!(
        &consumer.bytes[4..8],
        &((base + pins::SINE_TABLE_OFF) as u32).to_be_bytes(),
        "bare-name proof: `dc.l Sine_Table` must resolve to $0000247C (plain)"
    );
}

/// (debug) The `math` section's linked bytes equal
/// the `s4.debug.bin` window at `pins::MATH`'s debug base/len, AND the
/// outbound consumer's fixups
/// resolve to the correct per-shape addresses.
#[test]
fn math_debug_region_matches_reference() {
    let Some(aeon) = math_tree() else { return };
    let Some(refrom) = golden("s4.debug.bin") else {
        eprintln!("skip: golden s4.debug.bin not present");
        return;
    };

    let (_resolved, linked) = compile_real_file(&aeon, true);

    let base = region_base(true) as usize;
    let expected = &refrom[base..base + pins::MATH.debug_len];
    let section = linked.section("math").expect("linked image must carry math");
    assert_region_matches(&section.bytes, expected, "math (debug) vs the s4.debug.bin window at pins::MATH");

    // $25F6 also fits abs.w — see the plain variant's comment for the byte
    // layout (opcode `4EB8` + 2-byte abs.w address at [2..4), then `dc.l
    // Sine_Table` at [4..8)).
    let consumer = linked.section("sec0").expect("linked image must carry the outbound consumer");
    assert_eq!(
        &consumer.bytes[0..2],
        &[0x4E, 0xB8],
        "sanity: `jsr` opcode must be absolute-word form (4EB8) — $25F6 fits abs.w"
    );
    assert_eq!(
        &consumer.bytes[2..4],
        &(base as u16).to_be_bytes(),
        "bare-name proof: `jsr GetSineCosine` must resolve to $25F6 (debug)"
    );
    assert_eq!(
        &consumer.bytes[4..8],
        &((base + pins::SINE_TABLE_OFF) as u32).to_be_bytes(),
        "bare-name proof: `dc.l Sine_Table` must resolve to $0000260E (debug)"
    );
}
