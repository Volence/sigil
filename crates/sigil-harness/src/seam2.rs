//! seam-2 — the BANKED sound DATA side, as a reusable library (the bank-body
//! emitters + the phased head, mirroring `seam1`'s Option-A pattern).
//!
//! Where seam-1 natively links the RESIDENT Z80 code blob, seam-2 places the
//! `phase 08000h` / `$8000`-window BANKED data the blob reads: the DAC sample
//! payload banks (`dac_samples.emp`), the MT streaming song bank (`mt_bank.emp`),
//! the SFX block (`sfx_bank.emp`), and the engine-table HEAD (`seq_opcode_tab` /
//! `dac_sample_tab` / the generated LUTs). Each bank body is emitted as a
//! byte-deterministic artifact asl BINCLUDEs at its pinned bank address, exactly
//! as `emit_sound_blob` emits the resident blob — the assembled-ROM CRC is the
//! provenance bar.
//!
//! ## Bank layout — the CURRENT baseline (NOT the stale `.emp`-header pins)
//!
//! The authoritative addresses are aeon's `s4.lst` / the current-baselined
//! `mixed_dac_rom` gate, NOT `dac_samples.emp`'s header comment or `dac_port.rs`
//! (both pin the STALE "aeon-f828406" layout `$50000`/`$58000`, a self-consistent
//! WINDOWED oracle at an older baseline — its `SND_*` values are f828406's, e.g.
//! `SND_BLIP_BANK=$A`, whereas the current ROM has `$9`). At the current
//! baseline the DAC banks are:
//!   * `dac_blip_bank`   @ `$48000` — `temp_blip.bin` (2880 B), bank id `$9`.
//!   * `dac_shared_bank` @ `$50000` — the 9 drum `.pcm` (30908 B), bank id `$A`.
//! The `$8000`-align gaps (`$48B40..$50000`, `$578BC..$58000`) are zero pad.

use std::path::Path;

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::SymbolTable;

/// The current-baseline LMA of the DAC blip bank (`temp_blip.bin`).
pub const DAC_BLIP_LMA: u32 = 0x48000;
/// The current-baseline LMA of the DAC shared drum bank (the 9 `.pcm`).
pub const DAC_SHARED_LMA: u32 = 0x50000;

/// The two DAC bank payloads, emitted from `dac_samples.emp` — the exact bytes
/// asl would BINCLUDE at `$48000` / `$50000` (each after an `align $8000`).
pub struct DacBanks {
    /// `dac_blip_bank` @ `$48000` (temp_blip.bin — 2880 B at the current baseline).
    pub blip: Vec<u8>,
    /// `dac_shared_bank` @ `$50000` (the 9 drum samples — 30908 B).
    pub shared: Vec<u8>,
}

/// Lower + place + link `dac_samples.emp` at the CURRENT-baseline bank pins and
/// return the two bank payloads. Byte-deterministic from the tracked `.emp` +
/// its embedded `.pcm`/`.bin` fixtures + the sigil toolchain version.
///
/// The two `bank:` sections carry NO `vma:` (VMA == LMA — the payload lives
/// physically in its bank and its labels resolve there), so `bankid()`/`winptr()`
/// fold from the PLACED addresses: `$48000 >> 15 == 9`, `$50000 >> 15 == 10`.
pub fn emit_dac_banks(aeon: &Path) -> Result<DacBanks, String> {
    let dir = aeon.join("games/sonic4/data/sound");
    let emp = dir.join("dac_samples.emp");
    let src = std::fs::read_to_string(&emp).map_err(|e| format!("read {}: {e}", emp.display()))?;

    let (file, pdiags) = parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("dac_samples.emp parse errors: {pdiags:?}"));
    }

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()), // so embed("dac/kick.pcm") / embed("temp_blip.bin") resolve
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!(
            "dac_samples.emp lower errors: {:?}",
            ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
        ));
    }

    // The CURRENT-baseline two-bank map ($48000/$50000). `text` is the zero-byte
    // equ carrier's benign home (the SND_* are equs, not data cells).
    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_blip_bank\"\nlma_base = 0x{DAC_BLIP_LMA:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_shared_bank\"\nlma_base = 0x{DAC_SHARED_LMA:X}\nsize = 0x8000\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout (bank straddle / ensure?): {d:?}"))?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .map_err(|d| format!("link: {d:?}"))?;

    let blip = linked
        .section("dac_blip_bank")
        .ok_or("linked image missing dac_blip_bank")?
        .bytes
        .clone();
    let shared = linked
        .section("dac_shared_bank")
        .ok_or("linked image missing dac_shared_bank")?
        .bytes
        .clone();
    Ok(DacBanks { blip, shared })
}
