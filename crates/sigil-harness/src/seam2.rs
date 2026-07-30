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

use sigil_frontend_as::{assemble, Options as AsOptions};
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

/// The current-baseline LMA of the SFX block (`sfx_bank.emp`'s `sfx_bank`
/// section) — right after the shape-dependent Moving-Trucks streaming bank, so
/// per shape: plain `$5BAE8` (== `MT_BANK` end) / debug `$5D53A`. The block
/// CONTENT differs per shape only in the `SfxTable` `*u8` pointer cells (they
/// hold the per-shape absolute Sfx_NN addresses); the blob payloads are
/// shape-invariant.
pub const SFX_BANK_LMA_PLAIN: u32 = 0x5BAE8;
/// Debug-shape SFX block base (after the debug MT bank, which adds DrumTest +
/// HCZ2).
pub const SFX_BANK_LMA_DEBUG: u32 = 0x5D53A;
/// The LMA of the `SfxBlobWinTab` head (VMA `$845F` in the `$8000` window,
/// physically at `$58000 + ($845F - $8000)` in the song/SFX bank). The head's
/// bank-head POSITION is shape-invariant (it precedes the shape-dependent song
/// tables), but its CONTENT is shape-dependent: every real cell is a
/// `winptr(Sfx_NN)` and the blobs shift with the shape.
pub const SFX_WIN_TAB_LMA: u32 = 0x5845F;

/// The SFX block BODY (`sfx_bank.emp`) + the co-linked window-pointer HEAD
/// (`sfx_blob_win_tab.emp`) — the coupled unit (the head's `dc.w SFX_WIN_NN`
/// cells resolve as cross-module link symbols against `sfx_bank.emp`'s
/// `SFX_WIN_*` equs, which fold same-module from `winptr(Sfx_NN)`). The DAC
/// body+head shape, per SHAPE (unlike the DAC, both halves shift with the build
/// shape because the SFX block sits after the shape-dependent song tables).
pub struct SfxBodyAndHead {
    /// `sfx_bank` @ `$5BAE8` (plain) / `$5D53A` (debug) — 1864 bytes.
    pub body: Vec<u8>,
    /// `SfxBlobWinTab` @ `$5845F` — the 270-byte (135 × 2) window-pointer head.
    pub head: Vec<u8>,
}

/// Lower + co-link `sfx_bank.emp` (bank body + the `SFX_WIN_*` equ layer) with
/// `sfx_blob_win_tab.emp` (the phased head) at the per-shape SFX-block base and
/// return the body + the head. Supplies the same cross-seam carriers `sfx_port.rs`
/// does — `MovingTrucks_Bank_Start` (@ `$58000`, the bank the SFX block shares) +
/// `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN` (config/sound_ids.asm's ungated
/// equs, read by the drift guards) — and checks all link asserts PASS (the body's
/// 1 co-residency + 3 drift guards; the head's 1 span guard). Byte-deterministic
/// from the tracked `.emp` + its embeds.
pub fn emit_sfx_body_and_head(aeon: &Path, debug: bool) -> Result<SfxBodyAndHead, String> {
    let sfx_dir = aeon.join("games/sonic4/data/sound/sfx");
    let snd_dir = aeon.join("games/sonic4/data/sound");

    // sfx_bank.emp is m68000 (the blob table + the SFX_WIN_* equ layer); it lives
    // in sound/sfx/ so its 18 embed("sfx_*.bin") fixtures resolve there.
    let body = lower_emp_file(&sfx_dir.join("sfx_bank.emp"), &sfx_dir, Cpu::M68000)?;
    // sfx_blob_win_tab.emp declares (cpu: z80); its cells reference the SFX_WIN_*
    // equs cross-module and its span guard defers to a link assert.
    let head = lower_emp_file(&snd_dir.join("sfx_blob_win_tab.emp"), &snd_dir, Cpu::M68000)?;
    let mut link_asserts = body.link_asserts.clone();
    link_asserts.extend(head.link_asserts.clone());

    let sfx_base = if debug { SFX_BANK_LMA_DEBUG } else { SFX_BANK_LMA_PLAIN };
    let sfx_size = 0x60000 - sfx_base; // to the bank top

    let mut sections: Vec<Section> = body.sections;
    sections.extend(head.sections);

    // The co-link map: the equ carriers ("text", zero-byte), the SFX body at its
    // per-shape base, and the phased head at its `$5845F` bank LMA (its `vma:
    // $8000` window is owned by the section attr).
    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x40\nkind = \"rom\"\n\n\
         [[region]]\nname = \"sfx_bank\"\nlma_base = 0x{sfx_base:X}\nsize = 0x{sfx_size:X}\nkind = \"rom\"\n\n\
         [[region]]\nname = \"sfx_blob_win_tab\"\nlma_base = 0x{SFX_WIN_TAB_LMA:X}\nsize = 0x200\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The cross-seam carriers: the bank-start label (@ $58000, for the body's
    // bankid co-residency ensure) + the three config/sound_ids.asm equs the drift
    // guards + the head span guard read (SFX_ID_BASE/SFX_COUNT/SFX_TABLE_LEN).
    let carrier_asm = "cpu 68000\nphase $58000\nMovingTrucks_Bank_Start:\n\tdc.w 0\nSFX_ID_BASE = $33\nSFX_COUNT = 9\nSFX_TABLE_LEN = 135\n";
    let mut carriers = assemble(
        carrier_asm,
        &AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() },
    )
    .map_err(|d| format!("carrier assemble: {d:?}"))?
    .sections;
    for sec in &mut carriers {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout (bank straddle / ensure?): {d:?}"))?;
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("sfx co-residency/drift/span guards fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;

    let body_bytes = linked.section("sfx_bank").ok_or("missing sfx_bank in linked image")?.bytes.clone();
    let head_bytes = linked.section("sfx_blob_win_tab").ok_or("missing sfx_blob_win_tab in linked image")?.bytes.clone();
    Ok(SfxBodyAndHead { body: body_bytes, head: head_bytes })
}

/// The LMA of the `SeqOpcodeTable` head (VMA `$856D` in the `$8000` window,
/// physically at `$58000 + ($856D - $8000)` in the song/SFX bank). The head's
/// bank-head POSITION is shape-invariant, but its CONTENT is shape-DEPENDENT:
/// each cell is a resident `Seq_Op_*` handler VMA, and the handlers re-base
/// after `sound_sequencer.emp`'s `if DEBUG==1` growth.
pub const SEQ_OPCODE_TAB_LMA: u32 = 0x5856D;
/// The `SeqOpcodeTable` byte length: 32 opcode slots × 2 bytes.
pub const SEQ_OPCODE_TAB_LEN: usize = 64;

/// Lower `seq_opcode_tab.emp` (the 32-entry coordination-opcode jump table) and
/// resolve its `dc.w Seq_Op_*` cells against the REAL resident handler VMAs — read
/// off the same seam-1 blob link (`native_sound_blob`) the resident driver is
/// emitted from, so the Seq_Op_* imports resolve to the exact VMAs the
/// `z80_sound_syms.asm` contract exported (design §2c). SHAPE-DEPENDENT: the
/// handlers re-base in the debug shape, so the emitted table differs per shape.
pub fn emit_seq_opcode_tab(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    let dir = aeon.join("engine/sound");
    let module = lower_emp_file(&dir.join("seq_opcode_tab.emp"), &dir, Cpu::M68000)?;
    let link_asserts = module.link_asserts.clone();

    // The table places at VMA $8000; its cell VALUES (resident Seq_Op_* addresses)
    // do not depend on its own placement, so a nominal region suffices.
    let map_toml =
        "fill = 0x00\n\n[[region]]\nname = \"seq_opcode_tab\"\nlma_base = 0x8000\nsize = 0x100\nkind = \"rom\"\n";
    let map = sigil_link::load_map(map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The resident Seq_Op_* handler VMAs (the `dc.w <label>` link targets), read
    // from the SAME blob link the resident driver ships from — so the table cells
    // equal the handlers' real addresses in this shape.
    let symbols = crate::seam1::native_sound_blob(aeon, debug).symbols;
    let pairs: Vec<(String, String)> =
        symbols.into_iter().map(|(n, v)| (n, format!("${v:X}"))).collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = crate::test_support::assemble_equ_pairs(&refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout: {d:?}"))?;
    // Fire any module ensures (none today; the AS twin's span guard awaits a
    // section-length primitive — but thread it so a future guard is not silently
    // ignored, matching the co-link emitters).
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("seq_opcode_tab guards fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;
    Ok(linked.section("seq_opcode_tab").ok_or("missing seq_opcode_tab in linked image")?.bytes.clone())
}

/// The LMA of the `sound_tables_z80` head (VMA `$8000`, the FIRST head table in
/// `soundBankHead`, physically at `$58000`). SHAPE-INVARIANT (pure-math LUTs +
/// fixed vol-env data; 855 bytes both shapes).
pub const SOUND_TABLES_Z80_LMA: u32 = 0x58000;
/// The `sound_tables_z80` byte length (`FmPitchTableZ` .. `FmVolEnv_03` end).
pub const SOUND_TABLES_Z80_LEN: usize = 0x357;

/// Lower `sound_tables_z80.emp` (the 4 pure-math FM/PSG LUTs + the vol-env
/// id-lists, pointer tables, and bodies) placed at VMA `$8000`, so its
/// intra-module `dc.w PsgVolEnv_XX`/`FmVolEnv_XX` pointer cells resolve to their
/// `$8000`-window addresses (Value16Le). SELF-CONTAINED — no external symbols
/// (the pointer cells reference this module's own body labels), so no co-link is
/// needed. SHAPE-INVARIANT (one `.bin` serves both shapes).
pub fn emit_sound_tables_z80(aeon: &Path) -> Result<Vec<u8>, String> {
    let dir = aeon.join("engine/sound");
    let module = lower_emp_file(&dir.join("sound_tables_z80.emp"), &dir, Cpu::M68000)?;
    let link_asserts = module.link_asserts.clone();

    // Place at the head LMA with the section's own `vma: $8000` window — the
    // intra-module pointer cells fold from that window base.
    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"sound_tables_z80\"\nlma_base = 0x{SOUND_TABLES_Z80_LMA:X}\nsize = 0x400\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout: {d:?}"))?;
    // Fire any module ensures (none today; the vol-env count guards await a
    // section-length primitive — threaded so a future guard is not silently
    // ignored, matching the co-link emitters).
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("sound_tables_z80 guards fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;
    Ok(linked.section("sound_tables_z80").ok_or("missing sound_tables_z80 in linked image")?.bytes.clone())
}

/// Emit the seam-2 sound-tables build input to `out_dir`: `sound_tables_z80.bin`
/// (shape-invariant — one file serves both shapes).
pub fn emit_sound_tables_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let bytes = emit_sound_tables_z80(aeon)?;
    let p = out_dir.join("sound_tables_z80.bin");
    std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

/// The `movingtrucks_pitchtable` head LMA — right after `sound_tables_z80`
/// ($58000 + $357) and before `SfxBlobWinTab` ($5845F), inside the `soundBankHead`
/// phase-$8000 window. SHAPE-INVARIANT.
pub const PITCHTABLE_LMA: u32 = 0x58357;
/// The `movingtrucks_pitchtable` byte length (2 * PITCHTAB_COUNT = 2 * 132).
pub const PITCHTABLE_LEN: usize = 264;

/// Lower `movingtrucks_pitchtable.emp` (the `SndDefaultPitchTable` banked head — the
/// exact Zyrinx "Moving Trucks" 132-entry two-page fnum table) placed at VMA `$8357`.
/// SELF-CONTAINED — pure `dc.b` data with no external symbols and no intra-module
/// references, so no co-link is needed (the SndDefaultPitchTable/MovingTrucks_PitchTable
/// labels are provided by `sound_bank.inc`'s AS side ahead of the BINCLUDE, like the
/// other heads). SHAPE-INVARIANT (one `.bin` serves both shapes). The last AS sound
/// head to go native (flip Stage-0), which frees `soundBankHead`'s `include pitchfile`.
pub fn emit_pitchtable(aeon: &Path) -> Result<Vec<u8>, String> {
    let dir = aeon.join("games/sonic4/data/sound");
    let module = lower_emp_file(&dir.join("movingtrucks_pitchtable.emp"), &dir, Cpu::M68000)?;
    let link_asserts = module.link_asserts.clone();

    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"movingtrucks_pitchtable\"\nlma_base = 0x{PITCHTABLE_LMA:X}\nsize = 0x200\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout: {d:?}"))?;
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("movingtrucks_pitchtable guards fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;
    Ok(linked
        .section("movingtrucks_pitchtable")
        .ok_or("missing movingtrucks_pitchtable in linked image")?
        .bytes
        .clone())
}

/// Emit the seam-2 pitch-table build input to `out_dir`:
/// `movingtrucks_pitchtable.bin` (shape-invariant — one file serves both shapes).
pub fn emit_pitchtable_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let bytes = emit_pitchtable(aeon)?;
    let p = out_dir.join("movingtrucks_pitchtable.bin");
    std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

/// Emit the seam-2 seq-opcode-table build inputs to `out_dir`:
/// `seq_opcode_tab{,_debug}.bin` (the 64-byte jump table, shape-dependent — the
/// resident handlers re-base in the debug shape).
pub fn emit_seq_opcode_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (debug, name) in [(false, "seq_opcode_tab.bin"), (true, "seq_opcode_tab_debug.bin")] {
        let bytes = emit_seq_opcode_tab(aeon, debug)?;
        let p = out_dir.join(name);
        std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(())
}

/// Emit the seam-2 SFX build inputs to `out_dir`: `sfx_bank{,_debug}.bin` (the
/// $5BAE8/$5D53A block bodies) + `sfx_blob_win_tab{,_debug}.bin` (the co-linked
/// 270-byte window-pointer heads). SHAPE-DEPENDENT (both halves shift with the
/// build shape — the SFX block sits after the shape-dependent song tables), so
/// there IS a `_debug` variant of each. NO syms file: unlike the MT bank
/// (`SongTable`/`SongPatchTable` read by `sound_api.asm`), no surviving AS code
/// reads `SfxTable` — `sound_sfx.emp`'s `SfxBlobWinTab` reads are native (its
/// address is a seam-1 banked carrier at $845F, unchanged by this unit).
pub fn emit_sfx_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (debug, body_name, head_name) in [
        (false, "sfx_bank.bin", "sfx_blob_win_tab.bin"),
        (true, "sfx_bank_debug.bin", "sfx_blob_win_tab_debug.bin"),
    ] {
        let out = emit_sfx_body_and_head(aeon, debug)?;
        let body_path = out_dir.join(body_name);
        std::fs::write(&body_path, &out.body).map_err(|e| format!("write {}: {e}", body_path.display()))?;
        let head_path = out_dir.join(head_name);
        std::fs::write(&head_path, &out.head).map_err(|e| format!("write {}: {e}", head_path.display()))?;
    }
    Ok(())
}

/// The current-baseline LMA of the Moving-Trucks streaming bank (`mt_bank.emp`'s
/// `mt_bank` section) — right after the engine-table head (`soundBankHead` ends at
/// `$58607`). SHAPE-DEPENDENT content (plain ends `$5BAE8`; debug adds DrumTest +
/// HCZ2 and ends `$5D53A`).
pub const MT_BANK_LMA: u32 = 0x58607;

/// The Moving-Trucks bank body + the labels AS-side 68k code consumes cross-seam
/// (`SongTable`/`SongPatchTable`, read by `sound_api.asm`'s two `movea.l #…`).
pub struct MtBank {
    /// `mt_bank` @ `$58607` — song + pitch table + patch bank + song_table,
    /// shape-dependent length.
    pub bytes: Vec<u8>,
    /// The AS-consumed cross-seam labels: `(name, absolute VMA)`. Emitted as an
    /// `mt_syms{,_debug}.asm` equ file the BINCLUDE build includes so
    /// `sound_api.asm`'s `movea.l #SongTable`/`#SongPatchTable` resolve.
    pub syms: Vec<(String, u32)>,
}

/// Lower + co-link `mt_bank.emp` at the current-baseline bank pin (`$58607`) and
/// return the bank body + the `SongTable`/`SongPatchTable` addresses. Supplies the
/// same THREE cross-seam carriers `mt_port.rs` does — `MovingTrucks_Bank_Start`
/// (label @ `$58000`, bank `$B`) + `SONG_MOVINGTRUCKS`=1 + `SONG_COUNT` (1 plain /
/// 3 debug) — and checks the module's 7 link asserts (5 co-residency + 2 drift
/// guards) all PASS. Byte-deterministic from the tracked `.emp` + its embeds.
pub fn emit_mt_bank(aeon: &Path, debug: bool) -> Result<MtBank, String> {
    let dir = aeon.join("games/sonic4/data/sound");
    let emp = dir.join("mt_bank.emp");
    let src = std::fs::read_to_string(&emp).map_err(|e| format!("read {}: {e}", emp.display()))?;
    let (file, pdiags) = parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("mt_bank.emp parse errors: {pdiags:?}"));
    }
    let debug_val: i128 = if debug { 1 } else { 0 };
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![("DEBUG".to_string(), debug_val)],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!(
            "mt_bank.emp lower errors: {:?}",
            ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
        ));
    }
    let link_asserts = module.link_asserts.clone();

    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"mt_bank\"\nlma_base = 0x{MT_BANK_LMA:X}\nsize = 0x79F9\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The cross-seam carrier: MovingTrucks_Bank_Start label @ VMA $58000 (bank $B,
    // where the head bank lands — the SAME bank mt_bank lands in) + the two song-id
    // equs the drift guards read (SONG_COUNT is shape-dependent, per sound_ids.asm).
    let song_count = if debug { 3 } else { 1 };
    let carrier_asm = format!(
        "cpu 68000\nphase $58000\nMovingTrucks_Bank_Start:\n\tdc.w 0\nSONG_MOVINGTRUCKS = 1\nSONG_COUNT = {song_count}\n"
    );
    let mut carriers = assemble(
        &carrier_asm,
        &AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() },
    )
    .map_err(|d| format!("carrier assemble: {d:?}"))?
    .sections;
    for sec in &mut carriers {
        sec.lma = 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout (bank straddle / ensure?): {d:?}"))?;
    let assert_diags = sigil_link::check_link_asserts(&resolved, &SymbolTable::new(), &link_asserts);
    if assert_diags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("mt_bank co-residency/drift guards fired: {assert_diags:?}"));
    }
    let linked = sigil_link::link(&resolved, &SymbolTable::new()).map_err(|d| format!("link: {d:?}"))?;

    let bytes = linked.section("mt_bank").ok_or("missing mt_bank in linked image")?.bytes.clone();
    // Read SongTable/SongPatchTable from the PLACED section's labels (offset +
    // the section's final LMA). mt_bank is pure data — resolve_layout does not
    // move byte offsets — so the lowered offsets are final.
    let placed = sections.iter().find(|s| s.name == "mt_bank").ok_or("missing placed mt_bank")?;
    let base = placed.lma;
    let mut syms = Vec::new();
    for want in ["SongTable", "SongPatchTable"] {
        let label = placed
            .labels
            .iter()
            .find(|l| l.name == want)
            .ok_or_else(|| format!("mt_bank must export `{want}` (sound_api.asm consumes it)"))?;
        syms.push((want.to_string(), base + label.offset));
    }
    Ok(MtBank { bytes, syms })
}

/// Emit the Moving-Trucks bank build inputs to `out_dir`: `mt_bank.bin` (plain) +
/// `mt_bank_debug.bin` (debug) + the matching `mt_syms{,_debug}.asm` equ files
/// (`SongTable`/`SongPatchTable` at their emitted addresses, so the BINCLUDE build's
/// `sound_api.asm` cross-seam `movea.l`s resolve). SHAPE-DEPENDENT (the two songs
/// the debug build adds), so — like the resident blob — there IS a `_debug` variant.
pub fn emit_mt_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (debug, bin_name, syms_name) in [
        (false, "mt_bank.bin", "mt_syms.asm"),
        (true, "mt_bank_debug.bin", "mt_syms_debug.asm"),
    ] {
        let mt = emit_mt_bank(aeon, debug)?;
        let bin_path = out_dir.join(bin_name);
        std::fs::write(&bin_path, &mt.bytes).map_err(|e| format!("write {}: {e}", bin_path.display()))?;
        let mut syms_src = String::from(
            "; GENERATED by sigil emit_sound_blob (seam-2 mt_bank) — do not hand-edit.\n\
             ; The Moving-Trucks bank labels sound_api.asm consumes cross-seam.\n",
        );
        for (name, addr) in &mt.syms {
            syms_src.push_str(&format!("{name} = ${addr:X}\n"));
        }
        let syms_path = out_dir.join(syms_name);
        std::fs::write(&syms_path, syms_src).map_err(|e| format!("write {}: {e}", syms_path.display()))?;
    }
    Ok(())
}
