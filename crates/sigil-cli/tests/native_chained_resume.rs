//! Flip Stage 2 · S1.2 — THE COMPUTED-RESUME (Chained placement) PROOF.
//!
//! Stage 1's native driver PINS every AS-residual section at its baked `org`
//! address — including the ~30 gate else-arm resume `org`s that are hardcoded
//! CANONICAL-sonic4 addresses (`engine.inc`/`main.asm`, each carrying the in-tree
//! NOTE "never set for other games"). Those literals are why demo + Config-A/B
//! could not build natively (their layouts resume elsewhere — see
//! `2026-07-30-flip-stage1-demo-config-native-blocked.md`).
//!
//! This gate proves the resume literals are REDUNDANT: sort the combined
//! (AS-residual ∪ natively-placed `.emp`) ROM sections by address and re-tag every
//! section that ABUTS its predecessor as `Chained` — its base COMPUTED by the linker
//! from the predecessor's end (the flip design §1.3: "the placement of one section
//! IS the resume point of the next"). To prove the address is genuinely computed and
//! not read back, ZERO each chained section's baked `lma` AND clear its baked
//! `vma_base` (so the computed lma is the SOLE address authority — an arbitrary
//! resume org would leave a stale VMA). The emitted ROM must still byte-match the
//! canonical asl ROM (header-neutral: the checksum `$18E` and ROM-end `$1A4` are the
//! full-file/appendix fields the assembled-bar comparison excludes).
//!
//! CONSEQUENCE (the Stage-2 unblock): because the linker computes every resume
//! point, the sonic4 resume-org LITERALS are inert, and a native driver that chains
//! this way reproduces ANY game/config layout from its own `.emp` placement — no
//! per-game resume-org parameterization in aeon. This is the mechanism the demo +
//! Config-A/B native drivers (the six-proof preamble) build on.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob \
//!   AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test native_chained_resume
//! ```
use sigil_harness::{native, pins};
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
#[track_caller]
fn strict_gate() -> bool {
    sigil_harness::test_support::strict_gate()
}
fn read_ref(aeon: &Path, name: &str) -> Option<Vec<u8>> {
    match std::fs::read(aeon.join(name)) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {name}");
            }
            None
        }
    }
}

// A ROM (image-bearing) section, as opposed to a RAM/phase section (VMA `$FFFFxxxx`,
// zero image bytes) which never participates in ROM layout.
fn is_rom(s: &Section) -> bool {
    match s.vma_base {
        Some(v) => v < 0x00F0_0000,
        None => true,
    }
}

// The whole-ROM native build touches the shared `engine/sound/generated` dir.
static GEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn build_chained(aeon: &Path, debug: bool) -> Vec<u8> {
    native::ensure_generated(aeon);
    let as_side = native::assemble_native_all_gates_as_side(aeon, debug).unwrap();
    let native::EmpProgram { sections: emp_sections, link_asserts, .. } =
        native::build_native_emp(aeon, debug).unwrap();

    let mut sections: Vec<Section> = as_side.sections;
    sections.extend(emp_sections);

    // Only ROM sections chain; RAM/phase sections pass through untouched.
    let (mut rom, ram): (Vec<Section>, Vec<Section>) = sections.into_iter().partition(is_rom);
    rom.sort_by_key(|s| s.lma);

    // Walk in address order: an anchor (a gap from its predecessor) is Pinned in a
    // fresh uniquely-named group; a section abutting its predecessor is Chained into
    // that group so the linker packs it right after — base COMPUTED, baked lma/VMA
    // discarded.
    let mut group_idx = 0usize;
    let mut cur_group = String::new();
    let mut prev_end: Option<u32> = None;
    for s in rom.iter_mut() {
        let orig_lma = s.lma;
        let span = s.placement_span();
        if prev_end == Some(orig_lma) {
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Chained;
            s.lma = 0;
            if s.vma_base == Some(orig_lma) {
                s.vma_base = None;
            }
        } else {
            group_idx += 1;
            cur_group = format!("chain{group_idx}");
            s.group = Some(cur_group.clone());
            s.placement = SectionPlacement::Pinned;
        }
        prev_end = Some(orig_lma + span);
    }

    let mut all = rom;
    all.extend(ram);

    let stubs = SymbolTable::new();
    let resolved = sigil_link::resolve_layout(&all, &stubs, true)
        .unwrap_or_else(|d| panic!("resolve_layout: {} diag(s); first {:?}", d.len(), d.first()));
    // Same drift-guard partition as the driver: a real Value(0) drift is a hard
    // fail; an unresolvable-extern (gated-off twin) guard is inapplicable here.
    let adiags = sigil_link::check_link_asserts(&resolved, &stubs, &link_asserts);
    let real: Vec<_> = adiags
        .iter()
        .filter(|d| d.level == sigil_span::Level::Error && !d.message.contains("not defined in this link"))
        .collect();
    assert!(real.is_empty(), "real drift guard fired: {:?}", real.first());

    let linked = sigil_link::link(&resolved, &stubs).unwrap_or_else(|d| panic!("link: {:?}", d.first()));
    // K5: the region geometry is the per-game map's (sigil.map.toml retired).
    let map_path = aeon.join("games/sonic4/map.toml");
    let map = sigil_link::load_map(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"))
}

/// Compare over the assembled bar HEADER-NEUTRAL (zero the checksum `$18E` and the
/// ROM-end pointer `$1A4` — the fields the golden computes over the full appendix'd
/// file). Returns the count of differing bytes.
fn header_neutral_diffs(a: &[u8], b: &[u8], n: usize) -> usize {
    let mut a = a[..n].to_vec();
    let mut b = b[..n].to_vec();
    for buf in [&mut a, &mut b] {
        buf[0x18e..0x190].fill(0);
        buf[0x1a4..0x1a8].fill(0);
    }
    (0..n).filter(|&i| a[i] != b[i]).count()
}

/// (PLAIN) computed-resume chained placement == canonical asl `s4.bin`.
#[test]
#[ignore = "RETIRED by Wave-B B-0: the pinned-baked substrate this proof chains from is
frozen at the asl-era layout, and pins.rs now tracks the PACKED shipped layout (they diverge
at the first size-changing parcel). The live, continuously-enforced form of this proof is the
shipped chained build itself: every target places via packed_true_bases and the six golden
gates hold. Kept for archaeology with the Stage-1 pinned bootstrap shape."]
fn chained_resume_plain() {
    let aeon = aeon_dir();
    let Some(refrom) = read_ref(&aeon, "s4.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = build_chained(&aeon, false);
    let n = pins::ASSEMBLED_LEN;
    assert_eq!(
        header_neutral_diffs(&rom, &refrom, n),
        0,
        "computed-resume (plain) diverged from s4.bin over [0,{n:#x})"
    );
}

/// (DEBUG) computed-resume chained placement == canonical asl `s4.debug.bin`.
#[test]
#[ignore = "RETIRED by Wave-B B-0: the pinned-baked substrate this proof chains from is
frozen at the asl-era layout, and pins.rs now tracks the PACKED shipped layout (they diverge
at the first size-changing parcel). The live, continuously-enforced form of this proof is the
shipped chained build itself: every target places via packed_true_bases and the six golden
gates hold. Kept for archaeology with the Stage-1 pinned bootstrap shape."]
fn chained_resume_debug() {
    let aeon = aeon_dir();
    let Some(refrom) = read_ref(&aeon, "s4.debug.bin") else { return };
    let _guard = GEN_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let rom = build_chained(&aeon, true);
    let n = pins::DEBUG_ASSEMBLED_LEN;
    assert_eq!(
        header_neutral_diffs(&rom, &refrom, n),
        0,
        "computed-resume (debug) diverged from s4.debug.bin over [0,{n:#x})"
    );
}
