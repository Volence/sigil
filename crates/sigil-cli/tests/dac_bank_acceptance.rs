//! Spec 2, Plan 7 #7-main — D7.6 / R7m.6: the `dac_samples` bank exhibit,
//! PINNED. The companion positive-acceptance to `pitcher_plant_acceptance.rs`,
//! for the banking half of item-7.
//!
//! `examples/game/data/dac_samples.emp` is a faithful `.emp` port of aeon's
//! `games/sonic4/data/sound/dac_samples.asm` STRUCTURE (its own entry module,
//! `module data.dac_samples`, compiled with `--root examples/game --prelude
//! prelude` exactly as the pitcher_plant exhibits are separate entries). It
//! holds three synthetic sample blobs in ONE `(bank: $8000)` section, then
//! emits a 68k descriptor table of per-sample `bankid()` (width 1),
//! `winptr()` (width 2, big-endian), and length (width 2, from the
//! const-bound blob's comptime `.len`) — the `SND_*_BANK/PTR/LEN` shape.
//!
//! ## Honest VMA == LMA via `--map` (whole-branch review 1a)
//!
//! aeon spells the invariant with three hand-written pieces: `align $8000`,
//! a `fatal` straddle guard, and per-sample `(a & $7F8000) >> 15` /
//! `(a & $7FFF) | $8000` / `end - start` constants. This exhibit replaces all
//! three with language features (the `bank:` section property + `bankid()` /
//! `winptr()` builtins + a comptime `ensure`). The DERIVED VALUES are computed
//! by the SAME mask/shift arithmetic as aeon — every `SND_*` below is
//! cross-computed from the fixture address by hand and matched byte for byte.
//!
//! Critically, this build is HONEST about placement (the review's IMPORTANT
//! finding 1a). The `bank:` no-straddle check runs in LMA (physical) space
//! while `bankid()`/`winptr()` fold label VMAs; an earlier draft gave
//! `dac_bank` a `vma: $8000` phase while its bytes chained from LMA 0, so its
//! bank id 1 was VMA-derived while the bytes physically sat in bank 0 — a
//! silent VMA/LMA decoupling (see ledger L7.5). This exhibit instead gives
//! `dac_bank` NO `vma:` (so its labels FOLLOW its placed LMA, R7p.5) and a
//! `--map` region that places `dac_bank` at `lma_base = 0x8000`. So VMA == LMA
//! == $8000: the bytes physically live in bank 1 AND the labels resolve there,
//! and `bankid()` folds to 1 because it is PHYSICALLY TRUE. What DIFFERS from
//! aeon (expected, per D7.2) is only padding: aeon `align`s the bank inside the
//! contiguous ROM, whereas this exhibit's map leaves the [0x10, 0x8000) gap
//! zero-filled. The exhibit argues equivalence of the derived VALUES, NOT
//! identity of padding.
//!
//! This is ALSO the test that exercises a `bank:` section through the
//! `--map` / `emit_rom` region-placement path (the review's Minor: no prior
//! test drove a `bank:` section through region placement — this discharges it).
//!
//! ## Map layout / region placement
//!
//! Sections are matched to regions BY SECTION NAME (`resolve::place_sections`),
//! so the map declares one region per section that lands in the image:
//!   - `snd_table` @ lma_base 0x0000, size 0x10 — the descriptor table (read
//!     first, kept at low addresses so it and the checksum window are stable).
//!   - `text`      @ lma_base 0x0010, size 0x80 — the prelude's DEFAULT section
//!     (top-level prelude items land in `text`; it needs a region or the build
//!     fails with "has no region in the map").
//!   - `dac_bank`  @ lma_base 0x8000, size 0x8000 — the bank, physically in
//!     bank 1 so `bankid()` is physically true.
//!
//! `emit_rom` pads the whole ROM from offset 0 with the map's fill byte (0x00)
//! and applies the Sega header checksum at 0x18E (which lands in the zero-fill
//! gap, outside every asserted window). The ROM therefore spans 0x0000..0x800F
//! (32783 bytes); this test asserts the three MEANINGFUL windows, not the 32KB
//! of intervening fill.

use std::path::Path;
use std::process::Command;

/// The multi-module example root (`examples/game/`), mirroring
/// `pitcher_plant_acceptance.rs::game_root`.
fn game_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/game"))
}

/// Run `sigil emp <entry> --root <root> --prelude prelude --map <map> -o <out>`
/// and return `(success, stdout, stderr, image?)`.
fn build(
    root: &Path,
    entry: &Path,
    map: &Path,
    out: &Path,
) -> (bool, String, String, Option<Vec<u8>>) {
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
            "--map",
            map.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn the sigil binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let bytes = std::fs::read(out).ok();
    (output.status.success(), stdout, stderr, bytes)
}

/// First-diff window comparison, mirroring `pitcher_plant_acceptance.rs`.
fn assert_window_identical(expected: &[u8], actual: &[u8], base: usize, what: &str) {
    if expected == actual {
        return;
    }
    let n = expected.len().min(actual.len());
    if let Some(i) = (0..n).find(|&i| expected[i] != actual[i]) {
        panic!(
            "{what}: first byte diff at window offset {i:#x} (image {:#x}): expected {:#04x} != got {:#04x}\n\
             expected[..] = {:02X?}\n     got[..] = {:02X?}",
            base + i,
            expected[i],
            actual[i],
            expected,
            actual,
        );
    }
    panic!(
        "{what}: lengths differ — expected {} bytes, got {} bytes (common prefix matches)",
        expected.len(),
        actual.len()
    );
}

/// aeon's `SND_*_BANK` derivation: `(addr & $7F8000) >> 15`. A runtime fn (not a
/// const-folded literal in-line) so the arithmetic reads verbatim as the exhibit
/// argues it — the whole point of the derivation is to show these masks/shifts.
fn bankid(addr: u32) -> u8 {
    ((addr & 0x7F_8000) >> 15) as u8
}
/// aeon's `SND_*_PTR` derivation: `(addr & $7FFF) | $8000` (written big-endian
/// in a 68k section — `BankPtr16Be`).
fn winptr(addr: u32) -> u16 {
    ((addr & 0x7FFF) | 0x8000) as u16
}

fn w(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}
fn b(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}

/// The `--map` region file, mirroring `module_resolution.rs`'s map tests: one
/// region per section that lands in the image, `fill = 0x00`. Written into the
/// test's tmpdir so the source tree carries no committed map (the exhibit is a
/// program-path acceptance, not a corpus build).
fn map_toml() -> &'static str {
    "fill = 0x00\n\
     \n\
     [[region]]\n\
     name = \"snd_table\"\n\
     lma_base = 0x0000\n\
     size = 0x10\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"text\"\n\
     lma_base = 0x0010\n\
     size = 0x80\n\
     kind = \"rom\"\n\
     \n\
     [[region]]\n\
     name = \"dac_bank\"\n\
     lma_base = 0x8000\n\
     size = 0x8000\n\
     kind = \"rom\"\n"
}

/// The `snd_table` window (image 0x0000..0x0010), hand-derived from the fixtures
/// INDEPENDENT of any compiler run.
///
///   LAYOUT ARITHMETIC (computed independently of read-back):
///
///   The bank labels resolve at their PLACED addresses: `dac_bank` has NO
///   `vma:`, so its labels follow the map region's `lma_base = 0x8000`
///   (R7p.5) — VMA == LMA. So Dac_Kick @ 0x8000, Dac_Snare @ 0x8006,
///   Dac_Hat @ 0x800B (6 + 5 + 4 bytes contiguous).
///
///   `snd_table` (region base 0x0000) emits three descriptors, each 1+2+2 = 5
///   bytes; the `ensure(...)` guards emit ZERO bytes (comptime guards). Derived
///   values, by the SAME mask/shift as aeon, from each sample's address:
///     bankid(L) = (L & $7F8000) >> 15 ; winptr(L) = (L & $7FFF) | $8000 (BE)
///     KICK  : bank (0x8000 & 0x7F8000)>>15 = 1 ; ptr (0x8000 & 0x7FFF)|0x8000 = 0x8000 ; len 6
///     SNARE : bank (0x8006 & 0x7F8000)>>15 = 1 ; ptr (0x8006 & 0x7FFF)|0x8000 = 0x8006 ; len 5
///     HAT   : bank (0x800B & 0x7F8000)>>15 = 1 ; ptr (0x800B & 0x7FFF)|0x8000 = 0x800B ; len 4
///     table content = 3 * 5 = 15 bytes (0x00..0x0F); the region's 16th byte
///     (0x0F) is zero-fill (region size 0x10, content 15).
fn expected_snd_table() -> Vec<u8> {
    let mut d = Vec::new();
    // KICK: Dac_Kick @ 0x8000.
    b(&mut d, bankid(0x8000)); // 1
    w(&mut d, winptr(0x8000)); // 0x8000 (BE)
    w(&mut d, 6); // KickBlob.len = 6
    // SNARE: Dac_Snare @ 0x8006.
    b(&mut d, bankid(0x8006)); // 1
    w(&mut d, winptr(0x8006)); // 0x8006 (BE)
    w(&mut d, 5); // SnareBlob.len = 5
    // HAT: Dac_Hat @ 0x800B.
    b(&mut d, bankid(0x800B)); // 1
    w(&mut d, winptr(0x800B)); // 0x800B (BE)
    w(&mut d, 4); // HatBlob.len = 4
    assert_eq!(d.len(), 0x0F, "three 5-byte descriptors = 15 bytes");
    d.push(0x00); // region 0x10, one trailing fill byte
    d
}

/// The prelude `text` window (image 0x0010..0x0074), hand-derived from
/// `prelude.emp`. Identical to the shape proven byte-for-byte by
/// `pitcher_plant_acceptance.rs`; re-derived here for a standalone check.
///
///   Map_PitcherPlant [1,0,0,0]          @ 0x10, 4 bytes
///   Draw_Sprite   : tst.b d0 / rts      @ 0x14, 0x4A00 0x4E75
///   ObjectMove    : clr.w d1 / rts      @ 0x18, 0x4241 0x4E75
///   SpawnObject   : moveq #0,d2 / rts    @ 0x1C, 0x7400 0x4E75
///   Despawn_Check : tst.w d3 / rts      @ 0x20, 0x4A43 0x4E75
///   Player_1 : Sst{id:1, rest 0}         @ 0x24, 0x50 bytes (id=0x0001, then zeros)
///   text content = 4 + 16 + 0x50 = 0x64 (100) bytes (0x10..0x74).
fn expected_text() -> Vec<u8> {
    let mut d = Vec::new();
    // `pub data Map_PitcherPlant: [u8;4] = [1, 0, 0, 0]`
    d.extend_from_slice(&[1, 0, 0, 0]);
    // `pub proc Draw_Sprite () { tst.b d0 ; rts }` — 0x4A00, 0x4E75
    w(&mut d, 0x4A00);
    w(&mut d, 0x4E75);
    // `pub proc ObjectMove () { clr.w d1 ; rts }` — 0x4241, 0x4E75
    w(&mut d, 0x4241);
    w(&mut d, 0x4E75);
    // `pub proc SpawnObject () { moveq #0, d2 ; rts }` — 0x7400, 0x4E75
    w(&mut d, 0x7400);
    w(&mut d, 0x4E75);
    // `pub proc Despawn_Check () { tst.w d3 ; rts }` — 0x4A43, 0x4E75
    w(&mut d, 0x4A43);
    w(&mut d, 0x4E75);
    assert_eq!(d.len(), 0x14, "Player_1 begins here (window-relative)");
    // `pub data Player_1: Sst = Sst{ id: 1, ...all-else-zero }` — $50 bytes.
    w(&mut d, 1);
    d.extend_from_slice(&[0u8; 0x50 - 2]);
    assert_eq!(d.len(), 0x64, "text content = 100 bytes");
    d
}

/// The `dac_bank` window (image 0x8000..0x800F): the three synthetic blobs,
/// contiguous, physically in bank 1.
fn expected_dac_bank() -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]); // Dac_Kick  @ 0x8000
    d.extend_from_slice(&[0xA1, 0xA2, 0xA3, 0xA4, 0xA5]); // Dac_Snare @ 0x8006
    d.extend_from_slice(&[0xF0, 0xF1, 0xF2, 0xF3]); // Dac_Hat   @ 0x800B
    assert_eq!(d.len(), 0x0F, "15 bytes of synthetic sample data");
    d
}

/// The headline positive proof: the REAL multi-module `--map` build of the
/// dac_samples exhibit produces ZERO diagnostics; every meaningful window
/// matches the hand-derivation byte for byte — every `SND_*` value link-folded
/// from the bank section's PHYSICAL (VMA == LMA) addresses in bank 1.
#[test]
fn dac_samples_windows_are_byte_exact() {
    let root = game_root();
    let entry = root.join("data/dac_samples.emp");
    let out_dir = std::env::temp_dir().join(format!(
        "sigil_dac_acceptance_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&out_dir).unwrap();
    let map = out_dir.join("sigil.map.toml");
    std::fs::write(&map, map_toml()).unwrap();
    let out = out_dir.join("dac_samples.bin");

    let (success, stdout, stderr, image) = build(root, &entry, &map, &out);

    assert!(
        success,
        "dac_samples --map build must succeed with zero errors; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "expected ZERO diagnostics of any severity; stderr was:\n{stderr}"
    );
    // ROM spans 0x0000..0x800F: last byte = dac_bank base 0x8000 + 15.
    assert!(
        stdout.contains("built: 32783 bytes"),
        "expected the CLI to report `built: 32783 bytes` (0x800F), stdout was: {stdout}"
    );

    let image = image.expect("output .bin was not written");
    assert_eq!(image.len(), 0x800F, "ROM spans 0x0000..0x800F (dac_bank base + 15)");

    // The three MEANINGFUL windows (the intervening 0x0074..0x8000 is zero-fill,
    // plus the header checksum at 0x18E, which we do not assert).
    assert_window_identical(&expected_snd_table(), &image[0x0000..0x0010], 0x0000, "snd_table");
    assert_window_identical(&expected_text(), &image[0x0010..0x0074], 0x0010, "prelude text");
    assert_window_identical(&expected_dac_bank(), &image[0x8000..0x800F], 0x8000, "dac_bank");
}

// ===========================================================================
// Negative probes (R7m.6) — tmpdir sources, exercised through the same CLI.
// ===========================================================================

/// Run `sigil emp <entry> --root <root> -o <out>` on a single-file tmpdir
/// module (no prelude needed for the probes).
fn build_probe(root: &Path, entry: &Path, out: &Path) -> (bool, String, Option<Vec<u8>>) {
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn the sigil binary");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let bytes = std::fs::read(out).ok();
    (output.status.success(), stderr, bytes)
}

/// NEGATIVE: a `(bank: $10)` section holding MORE than $10 bytes cannot fit its
/// bank — the linker fails with the §7.3 "over by K bytes" budget diagnostic
/// (R7m.2). $12 bytes into a $10 bank → over by 2.
#[test]
fn oversized_bank_section_fails_over_by() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("probe.emp"),
        // 18 bytes ($12) into a $10 bank.
        "module probe\n\
         section big (cpu: m68000, bank: $10) {\n\
           data Blob: [u8;18] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17]\n\
         }\n",
    )
    .unwrap();
    let out = root.join("out.bin");
    let (success, stderr, _image) = build_probe(root, &root.join("probe.emp"), &out);
    assert!(!success, "an over-budget bank section must fail the build; stderr:\n{stderr}");
    assert!(
        stderr.contains("over by"),
        "expected an `over by` budget diagnostic, stderr was:\n{stderr}"
    );
}

/// POSITIVE bump pin: two chained sections where the SECOND is bank-constrained
/// and WOULD straddle a boundary if placed at the packing cursor — assert the
/// linker BUMPED it to the next boundary (bump-only-when-straddling, D7.2).
///
///   LAYOUT ARITHMETIC:
///     `filler` (cpu m68000, no bank) packs FIRST at LMA 0, $C bytes long, so
///       the packing cursor ends at 0xC.
///     `win` (cpu m68000, bank: $10) is 8 bytes. Placed at the cursor 0xC it
///       would span [0xC, 0x14) — straddling the 0x10 boundary. So the
///       placement pass bumps its base to next_multiple_of(0x10) = 0x10.
///     After the bump `win` sits at [0x10, 0x18). The gap 0xC..0x10 (4 bytes)
///       is zero-filled. `win`'s first byte (a distinctive 0xEE) lands at image
///       offset 0x10, NOT 0xC — the pin.
#[test]
fn chained_bank_section_bumps_when_it_would_straddle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(
        root.join("probe.emp"),
        "module probe\n\
         section filler (cpu: m68000) {\n\
           data Fill: [u8;12] = [0,0,0,0,0,0,0,0,0,0,0,0]\n\
         }\n\
         section win (cpu: m68000, bank: $10) {\n\
           data Win: [u8;8] = [$EE,1,2,3,4,5,6,7]\n\
         }\n",
    )
    .unwrap();
    let out = root.join("out.bin");
    let (success, stderr, image) = build_probe(root, &root.join("probe.emp"), &out);
    assert!(success, "the bump build must succeed; stderr:\n{stderr}");
    let image = image.expect("output .bin was not written");
    // filler ($C) + 4-byte bump gap + win (8) → 0x18 bytes total.
    assert_eq!(image.len(), 0x18, "image must span through the bumped `win` section");
    // The bump gap (0xC..0x10) is zero-filled...
    assert!(
        image[0xC..0x10].iter().all(|&x| x == 0),
        "the 0xC..0x10 bump gap must be zero-filled, got {:02X?}",
        &image[0xC..0x10]
    );
    // ...and `win`'s distinctive first byte lands at the boundary, not the cursor.
    assert_eq!(
        image[0x10], 0xEE,
        "`win` must be bumped to the 0x10 boundary; byte at 0x10 = {:#04X}",
        image[0x10]
    );
    assert_eq!(
        image[0xC], 0x00,
        "`win`'s first byte must NOT be at the un-bumped cursor 0xC; byte there = {:#04X}",
        image[0xC]
    );
}
