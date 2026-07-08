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
//! ## aeon-scheme equivalence (D7.2 — derived VALUES, not padding)
//!
//! aeon spells the invariant with three hand-written pieces: `align $8000`,
//! a `fatal` straddle guard, and per-sample `(a & $7F8000) >> 15` /
//! `(a & $7FFF) | $8000` / `end - start` constants. This exhibit replaces all
//! three with language features (the `bank:` section property + `bankid()` /
//! `winptr()` builtins + a comptime `ensure`). The DERIVED VALUES are computed
//! by the SAME mask/shift arithmetic as aeon — every `SND_*` below is
//! cross-computed from the fixture address by hand and matched byte for byte.
//! What DIFFERS from aeon (expected, per D7.2): aeon `align`s the bank to an
//! absolute $8000 boundary in the ROM; this exhibit instead gives the bank
//! section `vma: $8000` so its LABELS resolve in bank 1 (a NONZERO bank id —
//! the whole point) while its BYTES chain from LMA 0 in the flat image. The
//! straddle check is LMA-based (physical placement), so the bank trivially
//! passes at LMA 0..0x0F; the bank id is VMA-derived, so it folds to 1. The
//! exhibit argues equivalence of those derived values, NOT identity of padding.
//!
//! ## Section / layout order
//!
//! `reachable_modules` seeds the BFS with the ENTRY module first
//! (`data.dac_samples`) and the `--prelude` module second (`prelude`).
//! `place_sequential` (no `--map`) packs sections contiguously from LMA 0 in
//! module-discovery order, and within a module in declaration order. So the
//! image is: `dac_bank` (the three blobs) FIRST, then `snd_table` (the
//! descriptor table) SECOND, then prelude's default `text` section THIRD.
//! The `vma: $8000` on `dac_bank` moves only its VMA origin (label resolution),
//! never its LMA cursor — so the blobs still land at image offset 0.

use std::path::Path;
use std::process::Command;

/// The multi-module example root (`examples/game/`), mirroring
/// `pitcher_plant_acceptance.rs::game_root`.
fn game_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/game"))
}

/// Run `sigil emp <entry> --root <root> --prelude prelude -o <out>` and return
/// `(success, stdout, stderr, image?)` — the pitcher_plant runner, retargeted at
/// the dac_samples entry.
fn build(root: &Path, entry: &Path, out: &Path) -> (bool, String, String, Option<Vec<u8>>) {
    let output = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([
            "emp",
            entry.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
            "--prelude",
            "prelude",
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

/// First-diff byte comparison, mirroring `pitcher_plant_acceptance.rs`.
fn assert_byte_identical(expected: &[u8], actual: &[u8], what: &str) {
    if expected == actual {
        return;
    }
    let n = expected.len().min(actual.len());
    if let Some(i) = (0..n).find(|&i| expected[i] != actual[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x} ({i}): expected {:#04x} != got {:#04x}\n\
             expected[{i:#x}..] = {:02X?}\n     got[{i:#x}..] = {:02X?}",
            expected[i],
            actual[i],
            &expected[i..(i + 8).min(expected.len())],
            &actual[i..(i + 8).min(actual.len())],
        );
    }
    panic!(
        "{what}: lengths differ — expected {} bytes, got {} bytes (common prefix matches)",
        expected.len(),
        actual.len()
    );
}

fn w(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}
fn b(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
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

/// The full expected image, hand-derived from the fixtures and prelude,
/// INDEPENDENT of any compiler run.
///
///   LAYOUT ARITHMETIC (computed independently of read-back):
///
///   Section `dac_bank` (bank: $8000, vma: $8000) packs FIRST at LMA 0. Its
///   LABELS resolve at vma_origin $8000 (the `vma:` attr), its BYTES at LMA 0:
///     Dac_Kick  : LMA 0x00, vma $8000, 6 bytes (11 22 33 44 55 66)
///     Dac_Snare : LMA 0x06, vma $8006, 5 bytes (A1 A2 A3 A4 A5)
///     Dac_Hat   : LMA 0x0B, vma $800B, 4 bytes (F0 F1 F2 F3)
///     dac_bank content = 15 bytes (0x00..0x0F). No relaxable fragments, so its
///     reserved placement span == 15 → `snd_table` starts at LMA 0x0F.
///
///   Section `snd_table` packs SECOND at LMA 0x0F. Each descriptor = 1+2+2 = 5
///   bytes; the `ensure(...)` guards emit ZERO bytes (comptime guards). Derived
///   values, by the SAME mask/shift as aeon, from each sample's VMA:
///     bankid(L) = (L & $7F8000) >> 15 ; winptr(L) = (L & $7FFF) | $8000 (BE)
///     KICK  : bank (0x8000 & 0x7F8000)>>15 = 1 ; ptr (0x8000 & 0x7FFF)|0x8000 = 0x8000 ; len 6
///     SNARE : bank (0x8006 & 0x7F8000)>>15 = 1 ; ptr (0x8006 & 0x7FFF)|0x8000 = 0x8006 ; len 5
///     HAT   : bank (0x800B & 0x7F8000)>>15 = 1 ; ptr (0x800B & 0x7FFF)|0x8000 = 0x800B ; len 4
///     snd_table content = 3 * 5 = 15 bytes (0x0F..0x1E). No relaxable
///     fragments → reserved span == 15 → prelude's `text` starts at LMA 0x1E.
///
///   Section prelude `text` packs THIRD at LMA 0x1E (30), decl order:
///     Map_PitcherPlant [1,0,0,0]          @ 0x1E, 4 bytes
///     Draw_Sprite   : tst.b d0 / rts      @ 0x22, 0x4A00 0x4E75
///     ObjectMove    : clr.w d1 / rts      @ 0x26, 0x4241 0x4E75
///     SpawnObject   : moveq #0,d2 / rts    @ 0x2A, 0x7400 0x4E75
///     Despawn_Check : tst.w d3 / rts      @ 0x2E, 0x4A43 0x4E75
///     Player_1 : Sst{id:1, rest 0}         @ 0x32, 0x50 bytes (id=0x0001, then zeros)
///     end of image = 0x32 + 0x50 = 0x82 = 130 bytes.
///
///   (The prelude opcode encodings are proven byte-for-byte by
///   `pitcher_plant_acceptance.rs`'s own derivation and the isa golden corpus;
///   they are re-derived from `prelude.emp` here for a standalone check.)
#[allow(clippy::too_many_lines)]
fn expected_image() -> Vec<u8> {
    let mut d = Vec::new();

    // === dac_bank section — LMA 0x00 (three synthetic blobs, contiguous) ===
    d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]); // Dac_Kick  @ 0x00
    d.extend_from_slice(&[0xA1, 0xA2, 0xA3, 0xA4, 0xA5]); // Dac_Snare @ 0x06
    d.extend_from_slice(&[0xF0, 0xF1, 0xF2, 0xF3]); // Dac_Hat   @ 0x0B
    assert_eq!(d.len(), 0x0F, "dac_bank ends / snd_table begins here");

    // === snd_table section — LMA 0x0F (per-sample {bank, ptr, len}) ===
    // KICK: Dac_Kick @ vma $8000.
    b(&mut d, bankid(0x8000)); // 1
    w(&mut d, winptr(0x8000)); // 0x8000 (BE)
    w(&mut d, 6); // KickBlob.len = 6
    // SNARE: Dac_Snare @ vma $8006.
    b(&mut d, bankid(0x8006)); // 1
    w(&mut d, winptr(0x8006)); // 0x8006 (BE)
    w(&mut d, 5); // SnareBlob.len = 5
    // HAT: Dac_Hat @ vma $800B.
    b(&mut d, bankid(0x800B)); // 1
    w(&mut d, winptr(0x800B)); // 0x800B (BE)
    w(&mut d, 4); // HatBlob.len = 4
    assert_eq!(d.len(), 0x1E, "snd_table ends / prelude `text` begins here");

    // === prelude `text` section — LMA 0x1E (30) ===
    // `pub data Map_PitcherPlant: [u8;4] = [1, 0, 0, 0]`
    d.extend_from_slice(&[1, 0, 0, 0]);
    assert_eq!(d.len(), 0x22);
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
    assert_eq!(d.len(), 0x32, "Player_1 begins here");
    // `pub data Player_1: Sst = Sst{ id: 1, ...all-else-zero }` — $50 bytes.
    w(&mut d, 1);
    d.extend_from_slice(&[0u8; 0x50 - 2]);
    assert_eq!(d.len(), 0x82, "end of image (130 bytes)");

    d
}

/// The headline positive proof: the REAL multi-module build of the dac_samples
/// exhibit produces ZERO diagnostics and the FULL 130-byte image matches the
/// hand-derivation, byte for byte — every `SND_*` value link-folded from the
/// bank section's VMA addresses.
#[test]
fn dac_samples_full_image_is_byte_exact() {
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
    let out = out_dir.join("dac_samples.bin");

    let (success, stdout, stderr, image) = build(root, &entry, &out);

    assert!(
        success,
        "dac_samples build must succeed with zero errors; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.trim().is_empty(),
        "expected ZERO diagnostics of any severity; stderr was:\n{stderr}"
    );
    assert!(
        stdout.contains("built: 130 bytes"),
        "expected the CLI to report `built: 130 bytes`, stdout was: {stdout}"
    );

    let image = image.expect("output .bin was not written");
    let expected = expected_image();
    assert_eq!(expected.len(), 130, "hand-derived expectation must total 130 bytes");
    assert_byte_identical(&expected, &image, "dac_samples acceptance exhibit");
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
