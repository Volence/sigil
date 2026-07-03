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

use sigil_harness::{derive_region_a, derive_region_b, parse_lst_symbols, RegionWindow};

/// The Region A driver head: `jp SndDrv_Init` = `C3 3B 00`. A wrong window
/// (drifted anchors, off-by-one, wrong ROM) is caught here.
const REGION_A_HEAD: [u8; 3] = [0xC3, 0x3B, 0x00];

/// Best-effort external 68k leaf-stub names to seed `stub-syms.toml`.
///
/// These are the symbols Region A/B reference that are DEFINED OUTSIDE A+B (in
/// games/sonic4/main.asm, .../data/sound/dac_samples.asm, engine/sound/
/// sound_sfx.asm) — bank ids and $8000-window pointers/lengths derived from the
/// physical ROM addresses of DAC samples and the SFX/engine-table bank. They
/// stay unresolved when A+B are assembled in isolation, so the linker needs them
/// as leaf stubs.
///
/// NOTE: this is a BEST-EFFORT seed. The DEFINITIVE stub set is finalized in
/// Task 5, when assembling A+B in isolation reveals exactly which symbols remain
/// unresolved. Names here that are absent from the fresh `.lst` are skipped.
const STUB_SYM_NAMES: &[&str] = &[
    "SND_ENGINE_TABLE_BANK",
    "SFX_BLOB_BANK",
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
    s.push_str("# OUTSIDE A+B (dac_samples.asm bank/ptr/len, the MT/SFX bank id, etc.).\n");
    s.push_str("# Seeded by `regen` from a fresh asl build.\n");
    s.push_str("#\n");
    s.push_str("# BEST-EFFORT SEED ONLY. The DEFINITIVE stub set is finalized in Task 5,\n");
    s.push_str("# when assembling A+B in isolation reveals exactly which symbols remain\n");
    s.push_str("# unresolved. Values are hex.\n");
    s.push('\n');
    for (name, val) in stubs {
        s.push_str(&format!("{name} = {val:#x}\n"));
    }
    std::fs::write(dir.join("stub-syms.toml"), s).map_err(|e| format!("write stub-syms.toml: {e}"))
}
