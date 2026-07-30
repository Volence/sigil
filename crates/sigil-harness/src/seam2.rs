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
use sigil_ir::{Section, SectionPlacement, SymbolTable};

/// The current-baseline LMA of the DAC blip bank (`temp_blip.bin`).
pub const DAC_BLIP_LMA: u32 = 0x48000;
/// The current-baseline LMA of the DAC shared drum bank (the 9 `.pcm`).
pub const DAC_SHARED_LMA: u32 = 0x50000;
/// The current-baseline LMA of the `DacSampleTable` head descriptor (VMA `$85AD`
/// in the `$8000` window, physically at `$58000 + ($85AD - $8000)` in the song
/// bank). Shape-INVARIANT (`s4.lst` == `s4.debug.lst`; the reference slice at
/// this offset is byte-identical plain/debug — the t24 head-shape control).
pub const DAC_SAMPLE_TAB_LMA: u32 = 0x585AD;
/// The `DacSampleTable` byte length: 10 descriptors × 9 bytes.
pub const DAC_SAMPLE_TAB_LEN: usize = 90;

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

/// The DAC bank BODIES + the co-linked descriptor HEAD (`DacSampleTable`) — the
/// Option-Y artifact set. Where [`emit_dac_banks`] emits only the two payload
/// banks, this ALSO lowers `engine/sound/dac_sample_tab.emp` and CO-LINKS it with
/// `dac_samples.emp` in one link: the head's `dc.b SND_KICK_BANK` / `dc.w
/// SND_KICK_PTR` / `dc.w SND_KICK_LEN` cells resolve as CROSS-MODULE link symbols
/// against `dac_samples.emp`'s `SND_*` equs (which fold same-module from
/// `bankid`/`winptr`/`.len`). NO `-D`, NO 30-value mirror — the `SND_*` names live
/// once, at the producer. This is the "twins present, both paths byte-identical"
/// dual-proof substrate the head port needs BEFORE any `.asm` deletion.
pub struct DacBodyAndHead {
    /// `dac_blip_bank` @ `$48000` (temp_blip.bin).
    pub blip: Vec<u8>,
    /// `dac_shared_bank` @ `$50000` (the 9 drum samples).
    pub shared: Vec<u8>,
    /// `DacSampleTable` @ `$585AD` — the 90-byte descriptor head (10 × 9 bytes).
    pub head: Vec<u8>,
}

/// Lower one `.emp` file at `initial_cpu` with `dir` as the embed/include root,
/// returning its full `Module` (sections + link_asserts). Panics-free: lower/parse
/// errors surface as `Err`.
fn lower_emp_file(
    path: &Path,
    dir: &Path,
    initial_cpu: Cpu,
) -> Result<sigil_ir::Module, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let (file, pdiags) = parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("{} parse errors: {pdiags:?}", path.display()));
    }
    let opts = LowerOptions {
        initial_cpu,
        include_root: Some(dir.to_path_buf()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!(
            "{} lower errors: {:?}",
            path.display(),
            ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
        ));
    }
    Ok(module)
}

/// Co-link `dac_samples.emp` (bank bodies + the `SND_*` equ carrier) with
/// `dac_sample_tab.emp` (the phased head) and return the two banks + the head.
/// The head's size-guard `ensure(10*9 == extern("DAC_SAMPLE_COUNT") *
/// extern("DacSample_len"))` is checked against the engine's real values (10, 9)
/// supplied as equ carriers (the same values `sound_constants.asm` defines).
pub fn emit_dac_body_and_head(aeon: &Path) -> Result<DacBodyAndHead, String> {
    let dac_dir = aeon.join("games/sonic4/data/sound");
    let eng_dir = aeon.join("engine/sound");

    // dac_samples.emp is m68000 (the banks + the SND_* equ carrier).
    let samples = lower_emp_file(&dac_dir.join("dac_samples.emp"), &dac_dir, Cpu::M68000)?;
    // dac_sample_tab.emp declares `module ... (cpu: z80)`; its head cells reference
    // the SND_* equs cross-module and its ensure defers to a link assert.
    let tab = lower_emp_file(&eng_dir.join("dac_sample_tab.emp"), &eng_dir, Cpu::M68000)?;
    let link_asserts = tab.link_asserts.clone();

    // The `.emp` sections (banks + both equ carriers + the head) are map-placed;
    // the size-guard carriers are pinned separately BELOW (they bypass the map).
    let mut sections: Vec<Section> = samples.sections;
    sections.extend(tab.sections);

    // The co-link map: the equ carriers ("text", zero-byte, stacked), the two DAC
    // banks, and the phased head at its `$585AD` song-bank LMA (its `vma: $8000`
    // window is owned by the section attr — a map vma_base would be overridden).
    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x40\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_blip_bank\"\nlma_base = 0x{DAC_BLIP_LMA:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_shared_bank\"\nlma_base = 0x{DAC_SHARED_LMA:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_sample_tab\"\nlma_base = 0x{DAC_SAMPLE_TAB_LMA:X}\nsize = 0x100\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The size-guard externs (DAC_SAMPLE_COUNT / DacSample_len) as equ carriers at
    // harness-private PINNED LMAs — the co-link's stand-in for `sound_constants.asm`.
    // Added AFTER place_sections (a pinned carrier bypasses the map).
    let pairs: Vec<(&str, &str)> = vec![("DAC_SAMPLE_COUNT", "10"), ("DacSample_len", "9")];
    let mut carriers = crate::test_support::assemble_equ_pairs(&pairs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout (bank straddle / ensure?): {d:?}"))?;
    // The deferred size-guard: fire the head's `ensure(...)` against the resolved
    // DAC_SAMPLE_COUNT / DacSample_len (a link-time drift guard).
    let assert_diags =
        sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("dac_sample_tab size-guard fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .map_err(|d| format!("link: {d:?}"))?;

    let blip = linked.section("dac_blip_bank").ok_or("missing dac_blip_bank")?.bytes.clone();
    let shared = linked.section("dac_shared_bank").ok_or("missing dac_shared_bank")?.bytes.clone();
    let head = linked.section("dac_sample_tab").ok_or("missing dac_sample_tab")?.bytes.clone();
    Ok(DacBodyAndHead { blip, shared, head })
}

/// Emit the seam-2 DAC build inputs to `out_dir` (the wire's artifacts, mirroring
/// seam-1's `emit_sound_blob`): `dac_blip_bank.bin` (the $48000 payload),
/// `dac_shared_bank.bin` (the $50000 payload), and `dac_sample_tab.bin` (the
/// co-linked 90-byte descriptor head). Byte-DETERMINISTIC from the tracked `.emp`
/// + `.pcm` + toolchain; the assembled-ROM CRC is the provenance bar. The DAC side
/// is shape-INVARIANT (one blip + one shared + one head, no `-D`/`__DEBUG__`), so —
/// unlike the resident blob — there is NO `_debug` variant.
pub fn emit_dac_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let out = emit_dac_body_and_head(aeon)?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let p = out_dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))
    };
    write("dac_blip_bank.bin", &out.blip)?;
    write("dac_shared_bank.bin", &out.shared)?;
    write("dac_sample_tab.bin", &out.head)?;
    Ok(())
}
