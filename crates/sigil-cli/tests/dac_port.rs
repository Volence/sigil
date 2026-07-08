//! Sound-migration T1 — the REAL `dac_samples.emp` port, PINNED.
//!
//! This is the exemplar-file acceptance for the whole migration campaign. It
//! compiles the ACTUAL ported file from aeon's tree —
//! `games/sonic4/data/sound/dac_samples.emp` — through the production
//! parse → lower → place → resolve → link pipeline, with `include_root` set to
//! the module's OWN directory (so `embed("dac/kick.pcm")` and
//! `embed("temp_blip.bin")` resolve), and the two `bank:` sections placed into
//! `--map` regions BY SECTION NAME at the aeon-f828406 pins
//! (`dac_blip_bank` @ $50000, `dac_shared_bank` @ $58000).
//!
//! It pins two independent facts:
//!
//!   (a) PAYLOAD BYTES — the `dac_shared_bank` section's linked bytes equal the
//!       concatenation of the nine `.pcm` files read directly off disk (in the
//!       .asm's order: kick, snare, hat, then the six S3K drums = 30,908 bytes),
//!       and `dac_blip_bank`'s equal `temp_blip.bin` (2,880 bytes). The link
//!       emits the blobs verbatim, contiguously, nothing else.
//!
//!   (b) ALL 30 `SND_*` equ VALUES — every `SND_*_BANK` / `_PTR` / `_LEN` folds
//!       to EXACTLY the value aeon's `s4.lst` records at the current baseline.
//!       `resolve_layout` rewrites each equ's expr to `Expr::Int(v)` post-
//!       placement (the `bankid()`/`winptr()` link-fold against the PLACED
//!       label addresses), so we read `v` straight off the resolved section's
//!       `equ_syms`.
//!
//! The bank labels resolve at their PLACED LMAs (neither `bank:` section has a
//! `vma:`, so VMA == LMA — R7p.5): `Dac_Temp_Blip` @ $50000, and the drums
//! chained from $58000. `bankid(L) = (L & $7F8000) >> 15` folds to $A for the
//! blip bank ($50000 >> 15 == 10) and $B for the shared bank ($58000 >> 15 ==
//! 11); `winptr(L) = (L & $7FFF) | $8000`. Cross-checked against `s4.lst`:
//! e.g. `SND_S3K_KICK_PTR = $F33E` is the last drum's window pointer, matching
//! the .asm's `Dac_S3K_Kick @ ($58000 + 30908 - 1406)`.
//!
//! ## Falsification (TDD-loose, recorded per the task)
//!
//! Before pinning the correct table, `SND_BLIP_BANK`'s expectation was set to a
//! deliberately-wrong `$B` (the shared-bank value) instead of `$A`. The run
//! panicked, reporting the REAL folded value:
//!   `SND_BLIP_BANK: expected 0xB, got 0xA`
//! — `got 0xA` is the genuine link-fold of `bankid("Dac_Temp_Blip")` from the
//! placed address $50000 ((0x50000 & 0x7F8000) >> 15 == 0xA), matching s4.lst.
//! That proves the fold is threaded end-to-end (placement → resolve → equ
//! rewrite), not a copy-pasted golden. The pin below uses the CORRECT $A.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Section, SymbolTable};
use std::path::{Path, PathBuf};

/// The module's own directory in aeon's tree — the `include_root` under which
/// `embed("temp_blip.bin")` and `embed("dac/*.pcm")` resolve, and where the
/// `.emp` and its fixtures live. Honors `AEON_DIR` (mirroring the sigil-harness
/// tests) with the workspace default.
fn sound_dir() -> PathBuf {
    let aeon =
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string());
    Path::new(&aeon).join("games/sonic4/data/sound")
}

/// The two-bank map, pinned at the aeon-f828406 layout. Sections match regions
/// BY NAME; the top-level `equ`/`ensure` items land in the default `text`
/// section, which needs its own region (it emits ZERO bytes here — all the
/// SND_* are equs, not data cells — but `place_sections` still requires a home
/// for it). Region sizes are the $8000 window per bank; `text` is nominal.
fn map_toml() -> &'static str {
    "fill = 0x00\n\
     \n\
     [[region]]\n\
     name = \"text\"\n\
     lma_base = 0x0000\n\
     size = 0x10\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_blip_bank\"\n\
     lma_base = 0x50000\n\
     size = 0x8000\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_shared_bank\"\n\
     lma_base = 0x58000\n\
     size = 0x8000\n\
     kind = \"rom\"\n"
}

/// Parse → lower (with the sound-dir include-root) → place into the map →
/// resolve. Returns the placed+resolved sections (equ exprs folded to
/// `Expr::Int`) plus the linked image, asserting a clean pipeline at each stage.
fn compile_real_file() -> (Vec<Section>, sigil_link::LinkedImage) {
    let dir = sound_dir();
    let emp_path = dir.join("dac_samples.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        // The module's OWN directory — so `embed("dac/kick.pcm")` /
        // `embed("temp_blip.bin")` resolve within the capability sandbox.
        include_root: Some(dir.clone()),
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(map_toml()).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed (bank straddle / ensure?): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (resolved, linked)
}

/// Read a folded equ value from the resolved sections. After `resolve_layout`,
/// every equ's `expr` is `Expr::Int(v)` (the `bankid()`/`winptr()`/`.len` fold
/// against the FINAL placed addresses), so this is a direct read — a non-`Int`
/// expr (or a missing name) is a hard failure, not a silent skip.
fn equ_value(sections: &[Section], name: &str) -> i64 {
    for sec in sections {
        for eq in &sec.equ_syms {
            if eq.name == name {
                match &eq.expr {
                    Expr::Int(v) => return *v,
                    other => panic!("equ `{name}` did not fold to Int post-resolve: {other:?}"),
                }
            }
        }
    }
    panic!("equ `{name}` not found in any resolved section");
}

/// (a) PAYLOAD BYTES: both bank sections' linked bytes equal the fixtures on
/// disk — the shared bank == the nine `.pcm` files concatenated in .asm order
/// (30,908 bytes), the blip bank == `temp_blip.bin` (2,880 bytes).
#[test]
fn dac_bank_payloads_match_disk_fixtures() {
    let dir = sound_dir();
    let (_resolved, linked) = compile_real_file();

    // The nine drums, in the .asm's exact order.
    let drum_files = [
        "dac/kick.pcm",
        "dac/snare.pcm",
        "dac/hat.pcm",
        "dac/s3k_snare.pcm",
        "dac/s3k_hitom.pcm",
        "dac/s3k_midtom.pcm",
        "dac/s3k_lowtom.pcm",
        "dac/s3k_floortom.pcm",
        "dac/s3k_kick.pcm",
    ];
    let mut expected_shared = Vec::new();
    for f in drum_files {
        expected_shared.extend_from_slice(
            &std::fs::read(dir.join(f)).unwrap_or_else(|e| panic!("read {f}: {e}")),
        );
    }
    assert_eq!(
        expected_shared.len(),
        30_908,
        "the nine drums must total 30,908 bytes (fixture drift?)"
    );
    let expected_blip = std::fs::read(dir.join("temp_blip.bin")).expect("read temp_blip.bin");
    assert_eq!(expected_blip.len(), 2_880, "temp_blip.bin must be 2,880 bytes");

    let shared = linked
        .section("dac_shared_bank")
        .expect("linked image must carry dac_shared_bank");
    assert_eq!(
        shared.bytes, expected_shared,
        "dac_shared_bank payload must equal the nine .pcm files concatenated in .asm order"
    );

    let blip = linked
        .section("dac_blip_bank")
        .expect("linked image must carry dac_blip_bank");
    assert_eq!(
        blip.bytes, expected_blip,
        "dac_blip_bank payload must equal temp_blip.bin verbatim"
    );
}

/// (b) ALL 30 `SND_*` equ VALUES fold to EXACTLY the aeon `s4.lst` baseline.
/// The triples are `(name, expected)` for BANK / PTR / LEN, cross-checked
/// against `s4.lst` (search each symbol there to re-derive on a re-baseline).
#[test]
fn snd_equ_values_match_s4lst_baseline() {
    let (resolved, _linked) = compile_real_file();
    let v = |name: &str| equ_value(&resolved, name);

    // (BANK, PTR, LEN) per sample — the s4.lst pins.
    // BLIP bank $A (@ $50000); every drum bank $B (@ $58000+).
    let expect: &[(&str, i64, i64, i64)] = &[
        ("SND_BLIP", 0xA, 0x8000, 0xB40),
        ("SND_KICK", 0xB, 0x8000, 0x57E),
        ("SND_SNARE", 0xB, 0x857E, 0xEA4),
        ("SND_HAT", 0xB, 0x9422, 0xF0),
        ("SND_S3K_SNARE", 0xB, 0x9512, 0xEA4),
        ("SND_S3K_HITOM", 0xB, 0xA3B6, 0xE8C),
        ("SND_S3K_MIDTOM", 0xB, 0xB242, 0x1230),
        ("SND_S3K_LOWTOM", 0xB, 0xC472, 0x15B6),
        ("SND_S3K_FLOORTOM", 0xB, 0xDA28, 0x1916),
        ("SND_S3K_KICK", 0xB, 0xF33E, 0x57E),
    ];

    for (base, bank, ptr, len) in expect {
        assert_eq!(
            v(&format!("{base}_BANK")),
            *bank,
            "{base}_BANK: expected {bank:#X}, got {:#X}",
            v(&format!("{base}_BANK"))
        );
        assert_eq!(
            v(&format!("{base}_PTR")),
            *ptr,
            "{base}_PTR: expected {ptr:#X}, got {:#X}",
            v(&format!("{base}_PTR"))
        );
        assert_eq!(
            v(&format!("{base}_LEN")),
            *len,
            "{base}_LEN: expected {len:#X}, got {:#X}",
            v(&format!("{base}_LEN"))
        );
    }
}
