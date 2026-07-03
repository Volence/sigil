//! regen — recon + golden regeneration for the M0 integration harness.
//!
//! Pipeline:
//!   1. Build the Aeon reference ROM FRESH with `asl` (via aeon/build.sh) so
//!      that `s4.bin` and `s4.lst` come from the SAME invocation and the `.lst`
//!      carries real resolved symbol values (not `$`-placeholders).
//!   2. Parse the `.lst` symbol table.
//!   3. Derive the Region A / Region B extraction windows from bracketing
//!      68k-context anchor labels (no hard-coded offsets).
//!   4. Extract the two golden blobs from `s4.bin`, cross-checking Region A's
//!      head byte-signature `C3 3B 00` (`jp SndDrv_Init`).
//!   5. Write the committed config (`windows.toml`, `stub-syms.toml`) and
//!      goldens (`region_a.bin`, `region_b.bin`).
//!
//! Run: `cargo run -p sigil-harness --bin regen`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sigil_harness::{
    build_harness, derive_region_a, derive_region_b, diff_region, load_stub_syms,
    parse_lst_symbols, LmaMap, RegionWindow,
};
use sigil_ir::Cpu;

/// The Region A driver head: `jp SndDrv_Init` = `C3 3B 00`. A wrong window
/// (drifted anchors, off-by-one, wrong ROM) is caught here.
const REGION_A_HEAD: [u8; 3] = [0xC3, 0x3B, 0x00];

/// The DEFINITIVE external 68k leaf-stub name set, finalized in Task 5 by
/// assembling regions A+B in isolation and stubbing exactly the symbols that
/// survive as unresolved leaves (every A/B-internal symbol resolves without a
/// stub). regen re-derives their VALUES from each fresh `.lst` so the gate stays
/// self-consistent when the reference ROM drifts (which shifts DAC/SFX addresses).
///
/// These are DEFINED OUTSIDE A+B:
///   * `SND_ENGINE_TABLE_BANK` — main.asm (= `MovingTrucks_Bank_Start>>15`);
///     read by the driver (`ld a,SND_ENGINE_TABLE_BANK`).
///   * `SND_<sample>_{BANK,PTR,LEN}` — games/.../data/sound/dac_samples.asm (10
///     DAC samples: BLIP/KICK/SNARE/HAT + the six S3K drums); read by region B's
///     dac_sample_tab.asm as the DacSample records.
///   * `Sfx_XX` — games/.../data/sound/sfx/sfx_XX.asm blob labels; windowed by
///     region B's sfx_blob_win_tab.asm via `sfx_winptr(Sfx_XX)`.
///   * `SFX_ID_BASE` / `SFX_TABLE_LEN` — games/.../data/sound/sfx/sfx_table.asm
///     (transcoder-generated, NOT sound_constants.asm); read by region A's driver
///     id-range check (`sub SFX_ID_BASE` / `cp SFX_TABLE_LEN`).
///
/// `SFX_BLOB_BANK` is deliberately NOT here: it is defined INSIDE region A
/// (sound_sfx.asm: `SFX_BLOB_BANK = sfx_bankid(Sfx_33)`) and resolves internally
/// once `Sfx_33` is stubbed — stubbing it would mask a real internal dep.
///
/// Names absent from the fresh `.lst` are skipped (and build_harness would then
/// fail loudly on the unresolved leaf — the intended alarm, not a silent pass).
const STUB_SYM_NAMES: &[&str] = &[
    "SND_ENGINE_TABLE_BANK",
    // --- driver id-range constants (sfx_table.asm; read by region A) ---
    "SFX_ID_BASE",
    "SFX_TABLE_LEN",
    // --- DAC samples (dac_samples.asm; read by dac_sample_tab.asm) ---
    "SND_BLIP_BANK",
    "SND_BLIP_PTR",
    "SND_BLIP_LEN",
    "SND_KICK_BANK",
    "SND_KICK_PTR",
    "SND_KICK_LEN",
    "SND_SNARE_BANK",
    "SND_SNARE_PTR",
    "SND_SNARE_LEN",
    "SND_HAT_BANK",
    "SND_HAT_PTR",
    "SND_HAT_LEN",
    "SND_S3K_KICK_BANK",
    "SND_S3K_KICK_PTR",
    "SND_S3K_KICK_LEN",
    "SND_S3K_SNARE_BANK",
    "SND_S3K_SNARE_PTR",
    "SND_S3K_SNARE_LEN",
    "SND_S3K_HITOM_BANK",
    "SND_S3K_HITOM_PTR",
    "SND_S3K_HITOM_LEN",
    "SND_S3K_MIDTOM_BANK",
    "SND_S3K_MIDTOM_PTR",
    "SND_S3K_MIDTOM_LEN",
    "SND_S3K_LOWTOM_BANK",
    "SND_S3K_LOWTOM_PTR",
    "SND_S3K_LOWTOM_LEN",
    "SND_S3K_FLOORTOM_BANK",
    "SND_S3K_FLOORTOM_PTR",
    "SND_S3K_FLOORTOM_LEN",
    // --- SFX blob labels (sfx/sfx_XX.asm; windowed by sfx_blob_win_tab.asm) ---
    "Sfx_33",
    "Sfx_34",
    "Sfx_35",
    "Sfx_36",
    "Sfx_3C",
    "Sfx_62",
    "Sfx_AB",
    "Sfx_B6",
    "Sfx_B9",
];

fn main() {
    if let Err(e) = run() {
        eprintln!("regen: ERROR: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // .../sigil/crates/sigil-harness -> .../aeon
    let aeon = manifest
        .join("../../../aeon")
        .canonicalize()
        .map_err(|e| format!("cannot locate aeon dir: {e}"))?;
    let golden_dir = manifest.join("golden");

    // 1. Fresh build.
    println!("regen: building reference ROM (aeon/build.sh) ...");
    let status = Command::new("./build.sh")
        .current_dir(&aeon)
        .status()
        .map_err(|e| format!("failed to spawn aeon/build.sh: {e}"))?;
    if !status.success() {
        return Err(format!("aeon/build.sh failed with {status}"));
    }

    // 2. Read the just-built artifacts (same invocation).
    let bin = std::fs::read(aeon.join("s4.bin"))
        .map_err(|e| format!("read s4.bin: {e}"))?;
    let lst = std::fs::read_to_string(aeon.join("s4.lst"))
        .map_err(|e| format!("read s4.lst: {e}"))?;
    println!(
        "regen: s4.bin = {} bytes, s4.lst = {} lines",
        bin.len(),
        lst.lines().count()
    );

    // 3. Parse the symbol table.
    let syms = parse_lst_symbols(&lst);
    println!("regen: parsed {} symbols from the listing", syms.len());

    // 4/5. Derive windows.
    let region_a = derive_region_a(&syms)?;
    let region_b = derive_region_b(&syms)?;

    // 6. Extract + cross-check.
    let a_bytes = slice_window(&bin, &region_a, "Region A")?;
    let b_bytes = slice_window(&bin, &region_b, "Region B")?;
    let a_head = a_bytes.get(..3);
    if a_head != Some(&REGION_A_HEAD[..]) {
        return Err(format!(
            "Region A head mismatch: got {:02X?}, expected {:02X?} (jp SndDrv_Init) \
             — the LMA/window derivation is wrong",
            a_head, REGION_A_HEAD
        ));
    }

    std::fs::create_dir_all(&golden_dir)
        .map_err(|e| format!("create golden dir: {e}"))?;
    std::fs::write(golden_dir.join("region_a.bin"), a_bytes)
        .map_err(|e| format!("write region_a.bin: {e}"))?;
    std::fs::write(golden_dir.join("region_b.bin"), b_bytes)
        .map_err(|e| format!("write region_b.bin: {e}"))?;

    // 7. windows.toml
    write_windows_toml(&golden_dir, &region_a, &region_b)?;

    // 8. stub-syms.toml
    let stubs = collect_stub_syms(&syms);
    write_stub_syms_toml(&golden_dir, &stubs)?;

    // 9. Summary.
    println!("\n=== DERIVED WINDOWS ===");
    print_window("Region A (phase 0    driver)", &region_a);
    print_window("Region B (phase 8000 MT bank)", &region_b);
    // `a_head` is guaranteed `Some(&[C3,3B,00])` here (verified + returned above).
    println!(
        "\nregion_a.bin[0..3] = {:02X?}  (expected {:02X?}: jp SndDrv_Init) => OK",
        a_head, REGION_A_HEAD,
    );
    println!("\n=== SEEDED STUB SYMBOLS ({}) ===", stubs.len());
    for (name, val) in &stubs {
        println!("  {name} = {val:#x}");
    }
    let missing: Vec<&str> = STUB_SYM_NAMES
        .iter()
        .copied()
        .filter(|n| !stubs.iter().any(|(name, _)| name == n))
        .collect();
    if !missing.is_empty() {
        println!("  (not present in .lst, skipped: {missing:?})");
    }
    println!("\nregen: wrote goldens + config to {}", golden_dir.display());

    // 10. THE MOMENT OF TRUTH — assemble A+B with Sigil and compare byte-for-byte
    //     against the reference blobs just extracted from the asl ROM.
    println!("\n=== SIGIL ASSEMBLE + COMPARE ===");
    let harness_root = manifest.join("harness_root.asm");
    let stubs = load_stub_syms(&golden_dir);
    let mut map = LmaMap::new();
    map.set(Cpu::Z80, Some(0x0), region_a.lma as u32);
    map.set(Cpu::Z80, Some(0x8000), region_b.lma as u32);
    let img = build_harness(&aeon, &harness_root, &stubs, &map)?;

    let sig_a = img
        .section("sec0")
        .ok_or_else(|| "no linked section `sec0` (region A)".to_string())?
        .bytes
        .clone();
    let sig_b = img
        .section("sec32768")
        .ok_or_else(|| "no linked section `sec32768` (region B)".to_string())?
        .bytes
        .clone();
    std::fs::write(golden_dir.join("sigil_a.bin"), &sig_a)
        .map_err(|e| format!("write sigil_a.bin: {e}"))?;
    std::fs::write(golden_dir.join("sigil_b.bin"), &sig_b)
        .map_err(|e| format!("write sigil_b.bin: {e}"))?;

    // region_a/region_b .bin were written above; re-read them so we compare against
    // exactly what is committed.
    let ref_a = std::fs::read(golden_dir.join("region_a.bin")).map_err(|e| e.to_string())?;
    let ref_b = std::fs::read(golden_dir.join("region_b.bin")).map_err(|e| e.to_string())?;

    let mut diverged: Vec<String> = Vec::new();
    match diff_region(&img, "sec0", &ref_a) {
        Ok(()) => println!("  Region A (sec0):      MATCH ({} bytes)", sig_a.len()),
        Err(e) => {
            println!("  Region A (sec0):      DIVERGE — {e}");
            diverged.push(format!("A: {e}"));
        }
    }
    match diff_region(&img, "sec32768", &ref_b) {
        Ok(()) => println!("  Region B (sec32768):  MATCH ({} bytes)", sig_b.len()),
        Err(e) => {
            println!("  Region B (sec32768):  DIVERGE — {e}");
            diverged.push(format!("B: {e}"));
        }
    }
    // The M0 acceptance gate: any divergence is a hard failure (non-zero exit),
    // not just verbose output. The sigil_*.bin / region_*.bin debug artifacts are
    // written above regardless, so a failing run still leaves the blobs to diff.
    if !diverged.is_empty() {
        return Err(format!(
            "SIGIL OUTPUT DIVERGED FROM REFERENCE ({} region(s)): {}",
            diverged.len(),
            diverged.join(" | ")
        ));
    }
    Ok(())
}

fn slice_window<'a>(
    bin: &'a [u8],
    w: &RegionWindow,
    label: &str,
) -> Result<&'a [u8], String> {
    let start = w.lma as usize;
    let end = start
        .checked_add(w.len as usize)
        .ok_or_else(|| format!("{label}: window length overflow"))?;
    bin.get(start..end).ok_or_else(|| {
        format!(
            "{label}: window [{start:#x}..{end:#x}) out of ROM bounds ({} bytes)",
            bin.len()
        )
    })
}

fn collect_stub_syms(syms: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    STUB_SYM_NAMES
        .iter()
        .filter_map(|&n| syms.get(n).map(|&v| (n.to_string(), v)))
        .collect()
}

fn print_window(label: &str, w: &RegionWindow) {
    println!(
        "  {label}: vma_base={:#x}  lma={:#x}  len={:#x} ({} bytes)",
        w.vma_base, w.lma, w.len, w.len
    );
}

fn write_windows_toml(
    dir: &Path,
    a: &RegionWindow,
    b: &RegionWindow,
) -> Result<(), String> {
    let s = format!(
        "# Z80 golden extraction windows — DERIVED by `regen` from a fresh asl build.\n\
         # Do not hand-edit; run `cargo run -p sigil-harness --bin regen` to regenerate.\n\
         #\n\
         # Region A: resident phase-0 driver, bracketed by 68k labels\n\
         #   Z80_Sound_Start / Z80_Sound_End (== Z80_SOUND_SIZE).\n\
         # Region B: phase-08000h Moving-Trucks / SFX engine-table bank,\n\
         #   LMA = MovingTrucks_Bank_Start, ends at Song_MovingTrucks.\n\
         \n\
         [region_a]\n\
         vma_base = {}\n\
         lma = {}\n\
         len = {}\n\
         \n\
         [region_b]\n\
         vma_base = {}\n\
         lma = {}\n\
         len = {}\n",
        a.vma_base, a.lma, a.len, b.vma_base, b.lma, b.len,
    );
    std::fs::write(dir.join("windows.toml"), s).map_err(|e| format!("write windows.toml: {e}"))
}

fn write_stub_syms_toml(dir: &Path, stubs: &[(String, u64)]) -> Result<(), String> {
    let mut s = String::new();
    s.push_str("# External 68k leaf-stub symbols referenced by Region A/B but DEFINED\n");
    s.push_str("# OUTSIDE A+B (dac_samples.asm bank/ptr/len, the MT/SFX bank id, the\n");
    s.push_str("# sfx/sfx_XX.asm blob labels). This is the DEFINITIVE set (STUB_SYM_NAMES\n");
    s.push_str("# in regen.rs, finalized in Task 5); regen re-derives their VALUES from\n");
    s.push_str("# each fresh asl build's s4.lst so the gate stays self-consistent when the\n");
    s.push_str("# reference ROM drifts. Do not hand-edit; run `regen` to regenerate.\n");
    s.push_str("# Values are hex.\n");
    s.push('\n');
    for (name, val) in stubs {
        s.push_str(&format!("{name} = {val:#x}\n"));
    }
    std::fs::write(dir.join("stub-syms.toml"), s).map_err(|e| format!("write stub-syms.toml: {e}"))
}
