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
//!
//! The `$8000`-align gaps (`$48B40..$50000`, `$578BC..$58000`) are zero pad.

use std::path::Path;

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};

use crate::map_placement::load_placement_map;

// ── The map-derived placement authority (Parcel A1) ─────────────────────────
//
// The seam-2 emit places its banked artifacts at addresses whose SOLE authority
// is `games/sonic4/map.toml` — two declared anchors (`dac_banks` @ `$48000`,
// `sound_bank` @ `$58000` vma `$8000`) plus the emit's OWN artifact lengths.
// [`sound_layout`] parses the anchors, measures the head/bank lengths by emitting
// them, and derives every LMA the emit needs. `pins.rs` / `tests/repin_pins.rs`
// keep the literals as INDEPENDENT drift detectors — the emit no longer
// self-certifies its placement.

/// The intra-bank align between the DAC blip bank (`$48000`) and the DAC shared
/// drum bank — the `$8000` `SetBank`-latched hardware boundary `dac_samples.emp`
/// reaches via an intra-section `align $8000`.
const DAC_INTRA_BANK_ALIGN: u32 = 0x8000;

/// The Genesis `$8000`-window bank size. The sound bank spans
/// `[sound_bank, sound_bank + BANK_WINDOW_SIZE)`; its top is the ceiling every
/// in-bank artifact must fit under.
const BANK_WINDOW_SIZE: u32 = 0x8000;

/// The sound-bank section head-labels in the map's declared `order`, in the
/// sequence this emit lays them down: the DAC bank island, then the `$8000`-window
/// head bank, then the Moving-Trucks streaming bank, then the SFX block. A map
/// reorder that breaks this relative order desyncs the derived chain and must fail
/// loud (checked in [`bank_anchors`] against the map's `order` slice).
const SOUND_BANK_ORDER: [&str; 4] =
    ["Dac_Temp_Blip", "SoundTablesZ80_Head", "Song_MovingTrucks", "Sfx_33"];

/// Offset from the `sound_bank` anchor to an even, in-bank scratch base used ONLY
/// to measure the shape-invariant SFX window-head LENGTH before the real SFX-block
/// base is known (the real base needs the Moving-Trucks body length, which sits
/// later in the derivation). The head's LENGTH is placement-invariant (135 window
/// pointers × 2 bytes); its CONTENT is not, but the length measurement never uses
/// the content. Kept an ANCHOR-RELATIVE offset (not an absolute LMA) so the probe
/// base stays inside the sound bank wherever the map puts that bank.
const SFX_LEN_PROBE_OFFSET: u32 = 0x2000;

/// The two declared bank anchors from `games/<g>/map.toml`, validated against the
/// declared byte-emitting `order`.
#[derive(Debug)]
struct BankAnchors {
    /// `dac_banks` anchor LMA — the DAC blip bank head / bank island (`$90000` since
    /// aeon's 2026-08-26 re-layout; `$48000` before it).
    dac_banks: u32,
    /// `sound_bank` anchor LMA — the `$8000`-window head bank (`$A0000` since the
    /// re-layout; `$58000` before it).
    sound_bank: u32,
    /// `sound_bank`'s window VMA (`$8000`).
    sound_bank_vma: u32,
}

/// The seam placement authority, aeon-relative: every sound artifact's LMA derives
/// from this map's `dac_banks` / `sound_bank` anchors. [`bank_anchors`] READS it and
/// [`require_reference_tree`] PROBES it, both through this one constant, so the
/// emitters' precondition and the emitters' first input can never name different
/// files.
pub const SOUND_PLACEMENT_MAP_REL: &str = "games/sonic4/map.toml";

/// A write into the reference tree requires that tree to have been NAMED: `Ok(())`
/// when `AEON_DIR` is set, otherwise an error naming the variable, the default the
/// resolver would have answered with, and the path the write was aimed at.
///
/// The default is RESOLVED, never spelled ([`crate::test_support::unnamed_default_tree`]),
/// so this refusal and the tree it is about cannot name different paths and the guard
/// keeps working when the suite moves — `contract/SUITE_PATHS.md`, "What a resolver owes
/// its reader".
///
/// The argument is invisibility, not damage. Unnamed, [`crate::test_support::aeon_dir`]
/// derives `<suite root>/aeon` — a working checkout somebody else is editing — and
/// `engine/sound/generated/` is gitignored there, so a write into it leaves no trace in
/// `git status` and no record of which process produced the bytes a later read picks up.
/// A fall-back to a live tree is structurally incapable of announcing its own failure; a
/// refusal names the caller's own site and is fixed by exporting one variable.
///
/// Reads keep the fallback. This is the write side only, which is why the check sits
/// in the emitters' shared precondition rather than in the resolver.
///
/// A caller that names the tree on its own command line rather than through the
/// environment declares it by setting `AEON_DIR` from that argument before it emits.
/// Both such callers are the ones aeon's `build.sh` drives with `--aeon .`: the
/// `emit_sound_blob` binary, and `sigil build`, whose native path reaches
/// [`crate::native::ensure_generated`]. This precondition therefore holds for every
/// writing process without the argv path becoming an exception to it.
pub fn require_named_reference_tree(aeon: &Path) -> Result<(), String> {
    // Through the published predicate rather than a private read of the variable, so the
    // gate that asserts this precondition is in force asserts THIS function rather than
    // its own second opinion of what the precondition is.
    if crate::test_support::checkout_var_is_set() {
        return Ok(());
    }
    // The refusal must say what it is protecting, and a resolver that cannot answer must
    // not render as an empty clause: an unresolvable default is reported as the refusal
    // the resolver itself gave.
    let default = match crate::test_support::unnamed_default_tree() {
        Ok(c) => c.path.display().to_string(),
        Err(refusal) => format!("no tree at all ({refusal})"),
    };
    Err(format!(
        "AEON_DIR is not set, so the reference tree written into is one nobody named. Unset, it \
         resolves to {default} — the live aeon working checkout — whose \
         engine/sound/generated/ is gitignored, so a write there leaves no trace in `git status` \
         and nothing records which process produced the bytes a later read picks up. Set AEON_DIR \
         to a reference tree you provisioned (scripts/provision-aeon-ref.sh). Nothing was created. \
         Refused write target: {}",
        aeon.display()
    ))
}

/// The precondition every sound emitter checks before it creates anything: the tree
/// must be NAMED ([`require_named_reference_tree`]) and `aeon` must carry
/// [`SOUND_PLACEMENT_MAP_REL`], the map the emit derives its placement from.
/// `Ok(())` when both hold; otherwise an error NAMING the absent path, with nothing
/// created.
///
/// The naming check runs FIRST. Reversed, an unset `AEON_DIR` would have the content
/// probe consult the live checkout, find a complete tree there, and pass — the write
/// would proceed into exactly the tree the check exists to keep it out of.
///
/// The same is true of WHEN the emitter creates its output directory, and it is the
/// other half of why this is a precondition rather than a late check. That directory
/// lives UNDER `aeon`, so creating it inside a tree that is not there manufactures that
/// tree's root — and the suite's reference guards probe roots (`if !aeon.exists()`).
/// A root conjured by one row flips every such guard from "skip" to "run against an
/// empty tree", so a run's second pass measures a different tree than its first.
/// Validating first keeps a missing reference tree a stable, self-describing
/// condition instead of a mutation of the thing under test.
pub fn require_reference_tree(aeon: &Path) -> Result<(), String> {
    require_named_reference_tree(aeon)?;
    if aeon.join(SOUND_PLACEMENT_MAP_REL).is_file() {
        return Ok(());
    }
    Err(format!(
        "reference tree not at {} (no {SOUND_PLACEMENT_MAP_REL}) — set AEON_DIR to an aeon \
         checkout. Nothing was created: a sound emitter writes UNDER this tree, and creating a \
         directory inside an absent tree makes the tree's root exist, which turns every \
         root-probing skip guard in the suite into a run against an empty tree.",
        aeon.display()
    ))
}

/// Parse the two seam-2 anchors from [`SOUND_PLACEMENT_MAP_REL`] and check the
/// emit's lay-down order is a subsequence of the map's declared `order`. Reuses the
/// harness's map reader ([`load_placement_map`]) — no second map engine.
fn bank_anchors(aeon: &Path) -> Result<BankAnchors, String> {
    let path = aeon.join(SOUND_PLACEMENT_MAP_REL);
    let src = std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    bank_anchors_from_str(&src)
}

/// [`bank_anchors`]'s pure core over a map source string (so the anchor-parse +
/// order-subsequence validation is unit-testable without a real aeon tree).
fn bank_anchors_from_str(src: &str) -> Result<BankAnchors, String> {
    let map = load_placement_map(src)?;

    let anchor = |name: &str| -> Result<&crate::map_placement::Anchor, String> {
        map.anchors_for(true)
            .find(|a| a.name == name)
            .ok_or_else(|| format!("map.toml: no sound-on anchor `{name}` (seam-2 placement authority)"))
    };
    let dac = anchor("dac_banks")?;
    let snd = anchor("sound_bank")?;
    let sound_bank_vma = snd
        .vma
        .ok_or("map.toml: `sound_bank` anchor must declare `vma` (the $8000 window base)")?;

    // The emit's lay-down order must be a subsequence of the map's declared order:
    // a future reorder that moves the DAC island past the head bank (etc.) must not
    // silently keep the emit deriving the old chain.
    let mut last = None;
    for label in SOUND_BANK_ORDER {
        let idx = map
            .order
            .iter()
            .position(|o| o == label)
            .ok_or_else(|| format!("map.toml `order` is missing `{label}` (seam-2 derivation anchor)"))?;
        if let Some(prev) = last {
            if idx <= prev {
                return Err(format!(
                    "map.toml `order` desyncs the seam-2 chain: `{label}` (index {idx}) does not \
                     follow its predecessor (index {prev}); the emit derives DAC island → head bank \
                     → MT bank → SFX block in that order"
                ));
            }
        }
        last = Some(idx);
    }

    Ok(BankAnchors { dac_banks: dac.at, sound_bank: snd.at, sound_bank_vma })
}

/// The seam-2 banked placement, DERIVED from the map anchors + the emit's own
/// artifact lengths (Parcel A1). Every field is a running-cursor derivation off the
/// two declared anchors — no field is a hardcoded LMA. Memoized per aeon root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundLayout {
    /// `dac_blip_bank` LMA — the `dac_banks` anchor.
    pub dac_blip_lma: u32,
    /// `dac_shared_bank` LMA — `dac_banks` anchor + the intra-bank align.
    pub dac_shared_lma: u32,
    /// `sound_tables_z80` head LMA — the `sound_bank` anchor, first head. (The
    /// in-bank offsets quoted below are bank-relative facts of the emitted heads;
    /// the absolute examples date from the `$58000` bank and read `+$48000` since
    /// aeon's 2026-08-26 re-layout.)
    pub sound_tables_z80_lma: u32,
    /// `movingtrucks_pitchtable` head LMA (`$58357`).
    pub pitchtable_lma: u32,
    /// `SfxBlobWinTab` head LMA (`$5845F`).
    pub sfx_win_tab_lma: u32,
    /// `SeqOpcodeTable` head LMA (`$5856D`).
    pub seq_opcode_tab_lma: u32,
    /// `DacSampleTable` head LMA (`$585AD`), last head — the head bank ends here.
    pub dac_sample_tab_lma: u32,
    /// `mt_bank` LMA (`$58628`) — `sound_bank` anchor + the head-bank span.
    pub mt_bank_lma: u32,
    /// `sfx_bank` block base, plain shape (`$5BB10`) — `mt_bank` + the plain MT body.
    pub sfx_bank_lma_plain: u32,
    /// `sfx_bank` block base, debug shape (`$5D560`) — `mt_bank` + the debug MT body.
    pub sfx_bank_lma_debug: u32,
}

/// Derive the seam-2 banked placement from `games/<g>/map.toml` + the emit's own
/// artifact lengths. The head-bank members' LMAs are the `sound_bank` anchor plus
/// the running byte-offsets of the heads THIS emit produces; `mt_bank` follows the
/// head-bank span; the per-shape `sfx_bank` base follows the (shape-dependent)
/// Moving-Trucks body length. Memoized per aeon root (the derivation lowers ~7
/// artifacts). Recursion-free: the length measurements call the emit CORES
/// (`*_at`), never the public `sound_layout`-consuming wrappers.
///
/// The two CHAINED members (`mt_bank`, `sfx_bank`) are predicted with the packing
/// walk's own alignment function, `native::packed_chained_base`, keyed by the same
/// head labels the walk reads — so the prediction and the placement consume ONE
/// declaration (`section_align::DECLARED`) and no frozen table. No shape-dependent
/// input enters the alignment: the plain and debug SFX bases differ only by the MT
/// body length ahead of them.
pub fn sound_layout(aeon: &Path) -> Result<SoundLayout, String> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, SoundLayout>>,
    > = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(l) = cache.lock().unwrap().get(aeon) {
        return Ok(*l);
    }

    let a = bank_anchors(aeon)?;

    let dac_blip_lma = a.dac_banks;
    let dac_shared_lma = a.dac_banks + DAC_INTRA_BANK_ALIGN;

    // The head bank: each member follows the running span of the heads before it.
    // The head CONTENTS are placement-invariant (their window pointers fold from the
    // `$8000` vma / other banks, not from their own head LMA), so measuring a head's
    // LENGTH via its public emitter — even before its real LMA is fixed — is exact.
    let sound_tables_z80_lma = a.sound_bank;
    let l_stz = emit_sound_tables_z80(aeon)?.len() as u32;

    let pitchtable_lma = sound_tables_z80_lma + l_stz;
    let l_pitch = emit_pitchtable(aeon)?.len() as u32;

    let sfx_win_tab_lma = pitchtable_lma + l_pitch;
    // The SFX window head's real body base isn't known yet (it needs the MT body
    // length, derived below); its LENGTH is placement-invariant, so measure it at a
    // scratch base.
    let l_sfxhead =
        emit_sfx_body_and_head_at(aeon, a.sound_bank + SFX_LEN_PROBE_OFFSET, sfx_win_tab_lma)?
            .head
            .len() as u32;

    let seq_opcode_tab_lma = sfx_win_tab_lma + l_sfxhead;
    let l_seq = emit_seq_opcode_tab(aeon, false)?.len() as u32;

    let dac_sample_tab_lma = seq_opcode_tab_lma + l_seq;
    let l_dach =
        emit_dac_body_and_head_at(aeon, dac_blip_lma, dac_shared_lma, dac_sample_tab_lma)?.head.len()
            as u32;

    // mt_bank and sfx_bank are the only two members of this layout that are CHAINED
    // AND PACKED (everything above is a declared map anchor, a phase-bank org, or a
    // section-internal offset — none of which the packing walk can move). The walk
    // does NOT lay them down contiguously: it rounds the running cursor up to the
    // section's DECLARED alignment (8 for both, aeon's mod-8 fold wall). A plain `+`
    // here agrees with the walk only for as long as every sum happens to land on the
    // quantum already, and stops agreeing the moment the sound head grows by a
    // non-multiple — folding every SFX pointer short of its blob.
    //
    // So predict with the walk's OWN function, keyed by the walk's own head labels,
    // rather than restate its arithmetic or its input.
    let mt_bank_lma =
        crate::native::packed_chained_base(dac_sample_tab_lma + l_dach, "Song_MovingTrucks")?;
    let l_mt_plain = emit_mt_bank_at(aeon, false, mt_bank_lma, a.sound_bank)?.bytes.len() as u32;
    let l_mt_debug = emit_mt_bank_at(aeon, true, mt_bank_lma, a.sound_bank)?.bytes.len() as u32;

    let sfx_bank_lma_plain = crate::native::packed_chained_base(mt_bank_lma + l_mt_plain, "Sfx_33")?;
    let sfx_bank_lma_debug = crate::native::packed_chained_base(mt_bank_lma + l_mt_debug, "Sfx_33")?;

    let _ = a.sound_bank_vma; // consumed by seam1's DacSampleTable window derivation
    let layout = SoundLayout {
        dac_blip_lma,
        dac_shared_lma,
        sound_tables_z80_lma,
        pitchtable_lma,
        sfx_win_tab_lma,
        seq_opcode_tab_lma,
        dac_sample_tab_lma,
        mt_bank_lma,
        sfx_bank_lma_plain,
        sfx_bank_lma_debug,
    };
    cache.lock().unwrap().insert(aeon.to_path_buf(), layout);
    Ok(layout)
}

/// The `DacSampleTable` head's `$8000`-window VMA — `sound_bank`'s window base plus
/// the head's offset within the head bank. seam-1 supplies this to the resident
/// driver (`-D DacSampleTable`); deriving it here ties the driver's window pointer
/// to the same map authority the head placement flows from.
pub fn dac_sample_table_vma(aeon: &Path) -> Result<u32, String> {
    let a = bank_anchors(aeon)?;
    let l = sound_layout(aeon)?;
    Ok(a.sound_bank_vma + (l.dac_sample_tab_lma - l.sound_tables_z80_lma))
}

/// The `$8000`-window VMAs of the head-bank members seam-1 supplies as equ
/// carriers — derived from the same [`sound_layout`] the placement flows from,
/// exactly as [`dac_sample_table_vma`] is.
///
/// seam-1 carries these as HAND-WRITTEN literals (`banked_carriers`), and those
/// literals are baked into the shipped resident driver's operand bytes. Nothing
/// compared the two, so the failure mode was: an SFX id-range growth moves the
/// derived head, the literal stays put, the golden byte gate breaks, and the
/// natural remediation — refreeze — blesses the WRONG blob. The `0x856D ->
/// 0x8571` bump recorded in seam-1's own comment is that exact move having
/// happened once already (lens sweep, seat LINK, finding S9).
///
/// `native.rs` states the rule this restores: "a second copy of this arithmetic
/// is the bug, not the fix — that is the lesson of the three unmaintained copies
/// of the sound-bank addresses."
pub fn banked_head_vmas(aeon: &Path) -> Result<Vec<(&'static str, u32)>, String> {
    let a = bank_anchors(aeon)?;
    let l = sound_layout(aeon)?;
    let vma = |lma: u32| a.sound_bank_vma + (lma - l.sound_tables_z80_lma);
    Ok(vec![
        ("SndDefaultPitchTable", vma(l.pitchtable_lma)),
        ("SfxBlobWinTab", vma(l.sfx_win_tab_lma)),
        ("SeqOpcodeTable", vma(l.seq_opcode_tab_lma)),
    ])
}

/// The Genesis cartridge BANK ID of the sound bank — the `$8000`-window page the
/// head bank / Moving-Trucks bank / SFX block all share. Derived from the same map
/// authority the placement flows from, so the seam-1 `SND_ENGINE_TABLE_BANK` /
/// `SFX_BLOB_BANK` consts move with the bank instead of being pinned literals.
///
/// The mask/shift form is EXACTLY `bankid()`'s (`(sym & $7F8000) >> 15`, see
/// `sigil-frontend-emp/src/eval/builtins.rs::eval_bankid`) — the value aeon's
/// `bankid(MovingTrucks_Bank_Start)` co-residency ensures fold against.
pub fn sound_bank_id(aeon: &Path) -> Result<u32, String> {
    Ok((sound_layout(aeon)?.sound_tables_z80_lma & 0x7F_8000) >> 15)
}

/// The `DacSampleTable` byte length: 10 descriptors × 12 bytes, + the 3-byte
/// head-tail alignment pad aeon's `dac_sample_tab.emp` appends (`DacHeadPad`),
/// which is 7 bytes since the SFX id range reached $BB — so 120 + 7 = 127.
///
/// Was `10 × 9 = 90` until sound-pkg-3 (2026-08-10) grew the descriptor to 12
/// bytes (`ds_vol` + the mix-cursor reserve appended, so no existing offset
/// moved). This constant was not re-pinned then, which is what has kept the
/// `seam2_*` family red under `SIGIL_STRICT_GATE=1` ever since; corrected
/// 2026-08-11. It tracks the EMITTED span including the pad, so it moves again if
/// the pad re-rounds — as it does whenever a head upstream of the table resizes.
pub const DAC_SAMPLE_TAB_LEN: usize = 127;

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
    let l = sound_layout(aeon)?;
    emit_dac_banks_at(aeon, l.dac_blip_lma, l.dac_shared_lma)
}

/// [`emit_dac_banks`]'s explicit-placement core (map-derivation-free, so
/// [`sound_layout`] can call it without re-entry). Places the two DAC banks at
/// `blip_lma` / `shared_lma`.
fn emit_dac_banks_at(aeon: &Path, blip_lma: u32, shared_lma: u32) -> Result<DacBanks, String> {
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

    // The map-derived two-bank layout ($48000/$50000). `text` is the zero-byte
    // equ carrier's benign home (the SND_* are equs, not data cells).
    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x10\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_blip_bank\"\nlma_base = 0x{blip_lma:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_shared_bank\"\nlma_base = 0x{shared_lma:X}\nsize = 0x8000\nkind = \"rom\"\n"
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
    defines: Vec<(String, i128)>,
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
        defines,
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
    let l = sound_layout(aeon)?;
    emit_dac_body_and_head_at(aeon, l.dac_blip_lma, l.dac_shared_lma, l.dac_sample_tab_lma)
}

/// [`emit_dac_body_and_head`] with an optional composition-input doctor: when
/// `doctor_blip_lma` is `Some(lma)`, the `dac_blip_bank` payload is co-linked at
/// `lma` instead of its map-derived base, so the head's `SND_BLIP_BANK`/
/// `SND_BLIP_PTR` cells re-fold from the moved bank (`bankid`/`winptr`). The
/// row-91 t24 non-vacuity control for the DAC family: a moved bank must make the
/// composed head DIVERGE from the frozen golden slice. Mirrors
/// `seam1::native_blob_doctored`'s banked-carrier axis.
pub fn emit_dac_body_and_head_doctored(
    aeon: &Path,
    doctor_blip_lma: Option<u32>,
) -> Result<DacBodyAndHead, String> {
    let l = sound_layout(aeon)?;
    let blip_lma = doctor_blip_lma.unwrap_or(l.dac_blip_lma);
    emit_dac_body_and_head_at(aeon, blip_lma, l.dac_shared_lma, l.dac_sample_tab_lma)
}

/// [`emit_dac_body_and_head`]'s explicit-placement core: co-link the DAC banks at
/// `blip_lma` / `shared_lma` and the descriptor head at `sample_tab_lma`.
/// Map-derivation-free so [`sound_layout`] can measure the head length without
/// re-entry. The head's cell CONTENT folds from the bank placements
/// (`bankid`/`winptr`), not from `sample_tab_lma` (its own window is the section's
/// `vma: $8000` attr), so the head placement is content-neutral.
fn emit_dac_body_and_head_at(
    aeon: &Path,
    blip_lma: u32,
    shared_lma: u32,
    sample_tab_lma: u32,
) -> Result<DacBodyAndHead, String> {
    let dac_dir = aeon.join("games/sonic4/data/sound");
    let eng_dir = aeon.join("engine/sound");

    // dac_samples.emp is m68000 (the banks + the SND_* equ carrier).
    let samples = lower_emp_file(&dac_dir.join("dac_samples.emp"), &dac_dir, Cpu::M68000, vec![])?;
    // dac_sample_tab.emp declares `module ... (cpu: z80)`; its head cells reference
    // the SND_* equs cross-module (link-resolved against dac_samples.emp). Its size
    // guard `use`s DAC_SAMPLE_COUNT / DacSample_len from the sound-constants
    // authority — seeded here as comptime `-D` (the E2 dissolution: the old pinned
    // "10"/"9" equ carriers are gone; the values flow from sound_constants.emp
    // through the same one-authority eval the resident blob uses), so the guard
    // folds at COMPTIME with nothing to drift.
    let auth = crate::seam1::sound_authority_consts(aeon);
    let dac_defines: Vec<(String, i128)> = ["DAC_SAMPLE_COUNT", "DacSample_len"]
        .iter()
        .map(|&n| {
            let v = auth
                .get(n)
                .copied()
                .unwrap_or_else(|| panic!("sound_constants.emp must define `{n}` (DAC head size guard)"));
            (n.to_string(), v as i128)
        })
        .collect();
    let tab = lower_emp_file(&eng_dir.join("dac_sample_tab.emp"), &eng_dir, Cpu::M68000, dac_defines)?;
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
         [[region]]\nname = \"dac_blip_bank\"\nlma_base = 0x{blip_lma:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_shared_bank\"\nlma_base = 0x{shared_lma:X}\nsize = 0x8000\nkind = \"rom\"\n\n\
         [[region]]\nname = \"dac_sample_tab\"\nlma_base = 0x{sample_tab_lma:X}\nsize = 0x100\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .map_err(|d| format!("resolve_layout (bank straddle / ensure?): {d:?}"))?;
    // The head's size guard now folds at COMPTIME (its `use`d DAC_SAMPLE_COUNT /
    // DacSample_len are seeded above), so no link assert is generated for it. The
    // check is threaded regardless — a future deferred guard must not be silently
    // ignored (matching the other co-link emitters).
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
/// co-linked 90-byte descriptor head). Byte-DETERMINISTIC from the tracked
/// `.emp` + `.pcm` + toolchain; the assembled-ROM CRC is the provenance bar. The DAC side
/// is shape-INVARIANT (one blip + one shared + one head, no `-D`/`__DEBUG__`), so —
/// unlike the resident blob — there is NO `_debug` variant.
pub fn emit_dac_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    require_reference_tree(aeon)?;
    let out = emit_dac_body_and_head(aeon)?;
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let p = out_dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))
    };
    write("dac_blip_bank.bin", &out.blip)?;
    write("dac_shared_bank.bin", &out.shared)?;
    write("dac_sample_tab.bin", &out.head)?;
    Ok(())
}

/// The SFX block BODY (`sfx_bank.emp`) + the co-linked window-pointer HEAD
/// (`sfx_blob_win_tab.emp`) — the coupled unit (the head's `dc.w SFX_WIN_NN`
/// cells resolve as cross-module link symbols against `sfx_bank.emp`'s
/// `SFX_WIN_*` equs, which fold same-module from `winptr(Sfx_NN)`). The DAC
/// body+head shape, per SHAPE (unlike the DAC, both halves shift with the build
/// shape because the SFX block sits after the shape-dependent song tables).
pub struct SfxBodyAndHead {
    /// `sfx_bank` @ `$5BB10` (plain) / `$5D560` (debug) — 1864 bytes.
    pub body: Vec<u8>,
    /// `SfxBlobWinTab` @ `$5845F` — the 270-byte (135 × 2) window-pointer head.
    pub head: Vec<u8>,
}

/// Lower + co-link `sfx_bank.emp` (bank body + the `SFX_WIN_*` equ layer) with
/// `sfx_blob_win_tab.emp` (the phased head) at the per-shape SFX-block base and
/// return the body + the head. Supplies the same cross-seam carriers `sfx_port.rs`
/// does — `MovingTrucks_Bank_Start` (@ the `sound_bank` anchor, the bank the SFX
/// block shares) +
/// `SFX_ID_BASE`/`SFX_COUNT`/`SFX_TABLE_LEN` (config/sound_ids.asm's ungated
/// equs, read by the drift guards) — and checks all link asserts PASS (the body's
/// 1 co-residency + 3 drift guards; the head's 1 span guard). Byte-deterministic
/// from the tracked `.emp` + its embeds.
pub fn emit_sfx_body_and_head(aeon: &Path, debug: bool) -> Result<SfxBodyAndHead, String> {
    let l = sound_layout(aeon)?;
    let sfx_base = if debug { l.sfx_bank_lma_debug } else { l.sfx_bank_lma_plain };
    emit_sfx_body_and_head_at(aeon, sfx_base, l.sfx_win_tab_lma)
}

/// [`emit_sfx_body_and_head`] with an optional composition-input doctor: when
/// `doctor_sfx_base` is `Some(lma)`, the SFX block is co-linked at `lma` instead
/// of its map-derived per-shape base, so every `SFX_WIN_NN = winptr(Sfx_NN)` equ
/// re-folds from the moved blobs and the co-linked `SfxBlobWinTab` head DIVERGES.
/// The row-91 t24 non-vacuity control for the SFX-head family. The alternate base
/// MUST stay inside bank `$B` (`$58000..$5FFFF`) or the body's co-residency
/// ensures fire instead (a different, guard-firing control the negative probes
/// already own).
pub fn emit_sfx_body_and_head_doctored(
    aeon: &Path,
    debug: bool,
    doctor_sfx_base: Option<u32>,
) -> Result<SfxBodyAndHead, String> {
    let l = sound_layout(aeon)?;
    let sfx_base = doctor_sfx_base
        .unwrap_or(if debug { l.sfx_bank_lma_debug } else { l.sfx_bank_lma_plain });
    emit_sfx_body_and_head_at(aeon, sfx_base, l.sfx_win_tab_lma)
}

/// [`emit_sfx_body_and_head`]'s explicit-placement core: co-link the SFX block at
/// `sfx_base` and the window-pointer head at `head_lma`. Free of the
/// [`sound_layout`] derivation (it reads only the map anchors, which do not depend
/// on any artifact length) so [`sound_layout`] can measure the
/// (placement-invariant) head length before the real `sfx_base` is known. The
/// block CONTENT folds from `sfx_base`
/// (`winptr(Sfx_NN)`); the head's own placement (`head_lma`) is content-neutral
/// (its window is the section's `vma: $8000` attr). The block payloads are
/// shape-invariant — the per-shape difference is entirely which `sfx_base` the
/// caller passes.
fn emit_sfx_body_and_head_at(
    aeon: &Path,
    sfx_base: u32,
    head_lma: u32,
) -> Result<SfxBodyAndHead, String> {
    let sfx_dir = aeon.join("games/sonic4/data/sound/sfx");
    let snd_dir = aeon.join("games/sonic4/data/sound");

    // sfx_bank.emp is m68000 (the blob table + the SFX_WIN_* equ layer); it lives
    // in sound/sfx/ so its 18 embed("sfx_*.bin") fixtures resolve there.
    let body = lower_emp_file(&sfx_dir.join("sfx_bank.emp"), &sfx_dir, Cpu::M68000, vec![])?;
    // sfx_blob_win_tab.emp declares (cpu: z80); its cells reference the SFX_WIN_*
    // equs cross-module and its span guard defers to a link assert.
    let head = lower_emp_file(&snd_dir.join("sfx_blob_win_tab.emp"), &snd_dir, Cpu::M68000, vec![])?;
    let mut link_asserts = body.link_asserts.clone();
    link_asserts.extend(head.link_asserts.clone());

    // The SFX block runs to the top of the sound bank, DERIVED from the map anchor
    // (`sound_bank + $8000`) rather than pinned — a hardcoded top would silently
    // wrap this u32 subtraction into a ~4 GB region the moment the bank moves up.
    let bank_start = bank_anchors(aeon)?.sound_bank;
    let bank_top = bank_start + BANK_WINDOW_SIZE;
    let sfx_size = bank_top.checked_sub(sfx_base).ok_or_else(|| {
        format!(
            "SFX block base {sfx_base:#X} is above the sound-bank top {bank_top:#X} \
             (bank {bank_start:#X} + {BANK_WINDOW_SIZE:#X}) — the block cannot fit its bank"
        )
    })?;

    let mut sections: Vec<Section> = body.sections;
    sections.extend(head.sections);

    // The co-link map: the equ carriers ("text", zero-byte), the SFX body at its
    // per-shape base, and the phased head at its `$5845F` bank LMA (its `vma:
    // $8000` window is owned by the section attr).
    let map_toml = format!(
        "fill = 0x00\n\n\
         [[region]]\nname = \"text\"\nlma_base = 0x0000\nsize = 0x40\nkind = \"rom\"\n\n\
         [[region]]\nname = \"sfx_bank\"\nlma_base = 0x{sfx_base:X}\nsize = 0x{sfx_size:X}\nkind = \"rom\"\n\n\
         [[region]]\nname = \"sfx_blob_win_tab\"\nlma_base = 0x{head_lma:X}\nsize = 0x200\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The cross-seam carriers: the bank-start label (@ the map's `sound_bank`
    // anchor, for the body's bankid co-residency ensure) + the SFX_TABLE_LEN equ
    // the head span guard
    // reads (sfx_blob_win_tab.emp's `ensure(135 == extern("SFX_TABLE_LEN"))`).
    // Parcel F2: SFX_TABLE_LEN is SOURCED FROM sfx_bank.emp itself (SfxTable.len,
    // via the authority-eval) — no hardcoded mirror; the body's own SFX_ID_BASE/
    // SFX_COUNT/SFX_TABLE_LEN drift guards retired (the derivation IS the authority,
    // nothing external to cross-check), so only the head's span guard needs a carrier.
    let sfx_table_len = crate::seam1::sfx_bank_authority_consts(aeon)
        .get("SFX_TABLE_LEN")
        .copied()
        .ok_or("sfx_bank.emp authority missing SFX_TABLE_LEN")?;
    let carrier_asm = format!(
        "cpu 68000\nphase ${bank_start:X}\nMovingTrucks_Bank_Start:\n\tdc.w 0\nSFX_TABLE_LEN = {sfx_table_len}\n"
    );
    let mut carriers = assemble(
        &carrier_asm,
        &AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() },
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

/// The `SeqOpcodeTable` byte length: 32 opcode slots × 2 bytes. (Its LMA — VMA
/// `$856D` in the `$8000` window, physically `$5856D` — is map-derived; see
/// [`sound_layout`]'s `seq_opcode_tab_lma`.)
pub const SEQ_OPCODE_TAB_LEN: usize = 64;

/// Lower `seq_opcode_tab.emp` (the 32-entry coordination-opcode jump table) and
/// resolve its `dc.w Seq_Op_*` cells against the REAL resident handler VMAs — read
/// off the same seam-1 blob link (`native_sound_blob`) the resident driver is
/// emitted from, so the Seq_Op_* imports resolve to the exact VMAs the
/// `z80_sound_syms.asm` contract exported (design §2c). SHAPE-DEPENDENT: the
/// handlers re-base in the debug shape, so the emitted table differs per shape.
pub fn emit_seq_opcode_tab(aeon: &Path, debug: bool) -> Result<Vec<u8>, String> {
    emit_seq_opcode_tab_doctored(aeon, debug, None)
}

/// [`emit_seq_opcode_tab`] with an optional composition-input doctor: when
/// `doctor` is `Some((handler, vma))`, that resident `Seq_Op_*` handler's VMA
/// carrier is overridden, so the table cell referencing it re-folds to `vma`.
/// The row-91 t24 non-vacuity control for the seq-opcode family: a moved handler
/// must make the composed table DIVERGE from the frozen golden slice. Mirrors
/// `seam1::native_blob_doctored`'s banked-carrier axis directly (the handler VMAs
/// ARE seam-1 carriers).
pub fn emit_seq_opcode_tab_doctored(
    aeon: &Path,
    debug: bool,
    doctor: Option<(&str, i64)>,
) -> Result<Vec<u8>, String> {
    let dir = aeon.join("engine/sound");
    let module = lower_emp_file(&dir.join("seq_opcode_tab.emp"), &dir, Cpu::M68000, vec![])?;
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
    // from the SAME resident-driver link the blob ships from — so the table cells
    // equal the handlers' real addresses in this shape. The symbols-only path (not
    // the full `native_sound_blob`) avoids lowering the resident DRIVER here: the
    // driver requests `DacSampleTable`, whose derivation flows through
    // [`sound_layout`] → this emitter, and lowering it would re-enter that chain.
    let symbols = crate::seam1::native_sound_symbols(aeon, debug);
    let pairs: Vec<(String, String)> = symbols
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v as i64,
            };
            (n, format!("${v:X}"))
        })
        .collect();
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

/// The `sound_tables_z80` byte length (`FmPitchTableZ` .. `FmVolEnv_03` end).
/// SHAPE-INVARIANT (pure-math LUTs + fixed vol-env data; 855 bytes both shapes).
/// Its LMA is the `sound_bank` anchor (`$58000`), the FIRST head in the head bank;
/// see [`sound_layout`]'s `sound_tables_z80_lma`.
pub const SOUND_TABLES_Z80_LEN: usize = 0x357;

/// Lower `sound_tables_z80.emp` (the 4 pure-math FM/PSG LUTs + the vol-env
/// id-lists, pointer tables, and bodies) placed at VMA `$8000`, so its
/// intra-module `dc.w PsgVolEnv_XX`/`FmVolEnv_XX` pointer cells resolve to their
/// `$8000`-window addresses (Value16Le). SELF-CONTAINED — no external symbols
/// (the pointer cells reference this module's own body labels), so no co-link is
/// needed. SHAPE-INVARIANT (one `.bin` serves both shapes).
pub fn emit_sound_tables_z80(aeon: &Path) -> Result<Vec<u8>, String> {
    emit_sound_tables_z80_doctored(aeon, None)
}

/// [`emit_sound_tables_z80`] with an optional composition-input doctor: when
/// `doctor_vma` is `Some(vma)`, the section's `$8000` window base is overridden
/// to `vma`, so every intra-module `dc.w PsgVolEnv_XX`/`FmVolEnv_XX` pointer cell
/// re-folds (`Value16Le`) to `vma + offset`. The row-91 t24 non-vacuity control
/// for the sound-tables family: a moved window must make the composed table
/// DIVERGE from the frozen golden slice (the byte-exact `$8000`-based layout is
/// load-bearing — the resident FM/PSG writers read these labels through the
/// window).
pub fn emit_sound_tables_z80_doctored(
    aeon: &Path,
    doctor_vma: Option<u32>,
) -> Result<Vec<u8>, String> {
    let dir = aeon.join("engine/sound");
    let module = lower_emp_file(&dir.join("sound_tables_z80.emp"), &dir, Cpu::M68000, vec![])?;
    let link_asserts = module.link_asserts.clone();

    // Place at the map-derived head LMA (the `sound_bank` anchor) with the section's
    // own `vma: $8000` window — the intra-module pointer cells fold from that window
    // base, so the head bytes are placement-invariant (the LMA only fixes where the
    // BINCLUDE lands in the whole-ROM link).
    let head_lma = bank_anchors(aeon)?.sound_bank;
    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"sound_tables_z80\"\nlma_base = 0x{head_lma:X}\nsize = 0x400\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    // The doctor overrides the window base BEFORE placement so the intra-module
    // pointer cells fold from the moved VMA (the section attr's `vma: $8000` is
    // the default; a map region has no vma_base to contend with).
    if let Some(vma) = doctor_vma {
        for sec in &mut sections {
            if sec.name == "sound_tables_z80" {
                sec.vma_base = Some(vma);
            }
        }
    }
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
    require_reference_tree(aeon)?;
    let bytes = emit_sound_tables_z80(aeon)?;
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let p = out_dir.join("sound_tables_z80.bin");
    std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

/// The `movingtrucks_pitchtable` byte length (2 * PITCHTAB_COUNT = 2 * 132).
/// SHAPE-INVARIANT. Its LMA — right after `sound_tables_z80` (`$58357`), inside the
/// `soundBankHead` window — is map-derived; see [`sound_layout`]'s `pitchtable_lma`.
pub const PITCHTABLE_LEN: usize = 264;

/// Lower `movingtrucks_pitchtable.emp` (the `SndDefaultPitchTable` banked head — the
/// exact Zyrinx "Moving Trucks" 132-entry two-page fnum table) placed at VMA `$8357`.
/// SELF-CONTAINED — pure `dc.b` data with no external symbols and no intra-module
/// references, so no co-link is needed (the SndDefaultPitchTable/MovingTrucks_PitchTable
/// labels are provided by `sound_bank.inc`'s AS side ahead of the BINCLUDE, like the
/// other heads). SHAPE-INVARIANT (one `.bin` serves both shapes). The last AS sound
/// head to go native (flip Stage-0), which frees `soundBankHead`'s `include pitchfile`.
pub fn emit_pitchtable(aeon: &Path) -> Result<Vec<u8>, String> {
    emit_pitchtable_doctored(aeon, false)
}

/// [`emit_pitchtable`] with an optional composition-input doctor: when `doctor`
/// is `true`, the FIRST page-0 data cell of the `.emp` source (`dc.b $00, …`) is
/// edited to `$01` before parse, and the table is recomposed from that doctored
/// source. The row-91 t24 non-vacuity control for the pitchtable family: because
/// the table is pure `dc.b` with a 1:1 source→output map (no fold, no placement
/// sensitivity), a single changed source cell must make the composed table
/// DIVERGE from the frozen golden slice — proving the byte gate catches any
/// table drift (the AS-side size guard only covers LENGTH drift).
pub fn emit_pitchtable_doctored(aeon: &Path, doctor: bool) -> Result<Vec<u8>, String> {
    let dir = aeon.join("games/sonic4/data/sound");
    let emp = dir.join("movingtrucks_pitchtable.emp");
    let mut src =
        std::fs::read_to_string(&emp).map_err(|e| format!("read {}: {e}", emp.display()))?;
    if doctor {
        // The first data cell: the `$00` immediately after the first `dc.b`.
        let anchor = src.find("dc.b").ok_or("pitchtable has no dc.b to doctor")?;
        let cell = src[anchor..]
            .find("$00")
            .ok_or("pitchtable's first data cell is not $00 (source drifted?)")?
            + anchor;
        src.replace_range(cell..cell + 3, "$01");
    }
    let (file, pdiags) = parse_str(&src);
    if pdiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("movingtrucks_pitchtable parse errors: {pdiags:?}"));
    }
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir.clone()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    if ldiags.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!(
            "movingtrucks_pitchtable lower errors: {:?}",
            ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
        ));
    }
    let link_asserts = module.link_asserts.clone();

    // Pure `dc.b` with no intra-module refs → placement-invariant bytes; place at the
    // map-derived head-bank base (the `sound_bank` anchor). The real head LMA
    // (`$58357`) is [`sound_layout`]'s `pitchtable_lma`, consumed by the golden gate.
    let head_lma = bank_anchors(aeon)?.sound_bank;
    let map_toml = format!(
        "fill = 0x00\n\n[[region]]\nname = \"movingtrucks_pitchtable\"\nlma_base = 0x{head_lma:X}\nsize = 0x200\nkind = \"rom\"\n"
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
    require_reference_tree(aeon)?;
    let bytes = emit_pitchtable(aeon)?;
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let p = out_dir.join("movingtrucks_pitchtable.bin");
    std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

/// Emit the seam-2 seq-opcode-table build inputs to `out_dir`:
/// `seq_opcode_tab{,_debug}.bin` (the 64-byte jump table, shape-dependent — the
/// resident handlers re-base in the debug shape).
pub fn emit_seq_opcode_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    require_reference_tree(aeon)?;
    let mut artifacts = Vec::new();
    for (debug, name) in [(false, "seq_opcode_tab.bin"), (true, "seq_opcode_tab_debug.bin")] {
        artifacts.push((name, emit_seq_opcode_tab(aeon, debug)?));
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (name, bytes) in artifacts {
        let p = out_dir.join(name);
        std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(())
}

/// Emit the seam-2 SFX build inputs to `out_dir`: `sfx_bank{,_debug}.bin` (the
/// $5BB10/$5D560 block bodies) + `sfx_blob_win_tab{,_debug}.bin` (the co-linked
/// 270-byte window-pointer heads). SHAPE-DEPENDENT (both halves shift with the
/// build shape — the SFX block sits after the shape-dependent song tables), so
/// there IS a `_debug` variant of each. NO syms file: unlike the MT bank
/// (`SongTable`/`SongPatchTable` read by `sound_api.asm`), no surviving AS code
/// reads `SfxTable` — `sound_sfx.emp`'s `SfxBlobWinTab` reads are native (its
/// address is a seam-1 banked carrier at $845F, unchanged by this unit).
pub fn emit_sfx_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    require_reference_tree(aeon)?;
    let mut artifacts = Vec::new();
    for (debug, body_name, head_name) in [
        (false, "sfx_bank.bin", "sfx_blob_win_tab.bin"),
        (true, "sfx_bank_debug.bin", "sfx_blob_win_tab_debug.bin"),
    ] {
        let out = emit_sfx_body_and_head(aeon, debug)?;
        artifacts.push((body_name, out.body));
        artifacts.push((head_name, out.head));
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (name, bytes) in artifacts {
        let p = out_dir.join(name);
        std::fs::write(&p, &bytes).map_err(|e| format!("write {}: {e}", p.display()))?;
    }
    Ok(())
}

/// The Moving-Trucks bank body + the byte offsets at which its two pointer tables
/// begin (`SongTable`/`SongPatchTable`, read cross-seam by `sound_api.emp`). The
/// offsets partition `bytes` into the three artifacts the split emits.
pub struct MtBank {
    /// `mt_bank` @ `$58628` — song streams + pitch table + patch bank + the two
    /// pointer tables, shape-dependent length.
    pub bytes: Vec<u8>,
    /// Byte offset within `bytes` where `SongTable` begins (a `SONG_COUNT`*4-byte
    /// table). Everything before it is the body.
    pub song_table_off: usize,
    /// Byte offset within `bytes` where `SongPatchTable` begins (the parallel
    /// `SONG_COUNT`*4-byte table, contiguous after `SongTable`, ending the blob).
    pub song_patch_table_off: usize,
}

/// Lower + co-link `mt_bank.emp` at the map-derived bank pin (`$58628`) and return
/// the bank body + the `SongTable`/`SongPatchTable` addresses. Supplies the same
/// THREE cross-seam carriers `mt_port.rs` does — `MovingTrucks_Bank_Start` (label @
/// the `sound_bank` anchor) + `SONG_MOVINGTRUCKS`=1 + `SONG_COUNT` (1 plain / 3 debug) —
/// and checks the module's 7 link asserts (5 co-residency + 2 drift guards) all
/// PASS. Byte-deterministic from the tracked `.emp` + its embeds.
pub fn emit_mt_bank(aeon: &Path, debug: bool) -> Result<MtBank, String> {
    emit_mt_bank_at(aeon, debug, sound_layout(aeon)?.mt_bank_lma, bank_anchors(aeon)?.sound_bank)
}

/// [`emit_mt_bank`]'s explicit-placement core: place the Moving-Trucks bank at
/// `mt_bank_lma`, with the `MovingTrucks_Bank_Start` co-residency carrier phased at
/// `bank_start` (the map's `sound_bank` anchor). BOTH addresses arrive as
/// PARAMETERS — this core is map-derivation-free so [`sound_layout`] can measure
/// the (shape-dependent) bank length that fixes the following SFX-block base
/// without re-entering itself; do not reach for `bank_anchors`/`sound_layout` here.
fn emit_mt_bank_at(
    aeon: &Path,
    debug: bool,
    mt_bank_lma: u32,
    bank_start: u32,
) -> Result<MtBank, String> {
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
         [[region]]\nname = \"mt_bank\"\nlma_base = 0x{mt_bank_lma:X}\nsize = 0x79F9\nkind = \"rom\"\n"
    );
    let map = sigil_link::load_map(&map_toml).map_err(|d| format!("map load: {d:?}"))?;
    let mut sections = module.sections;
    let pd = place_sections(&mut sections, &map);
    if pd.iter().any(|d| d.level == sigil_span::Level::Error) {
        return Err(format!("place_sections errors: {pd:?}"));
    }

    // The cross-seam carrier: MovingTrucks_Bank_Start label @ the caller-supplied
    // `sound_bank` anchor VMA (where the head bank lands — the SAME bank mt_bank
    // lands in) + the two song-id equs the drift guards read (SONG_COUNT is
    // shape-dependent, per sound_ids.asm).
    let song_count = if debug { 3 } else { 1 };
    let carrier_asm = format!(
        "cpu 68000\nphase ${bank_start:X}\nMovingTrucks_Bank_Start:\n\tdc.w 0\nSONG_MOVINGTRUCKS = 1\nSONG_COUNT = {song_count}\n"
    );
    let mut carriers = assemble(
        &carrier_asm,
        &AsOptions { initial_cpu: Some(Cpu::M68000), ..AsOptions::default() },
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
    // Read SongTable/SongPatchTable offsets from the PLACED section's labels.
    // mt_bank is pure data — resolve_layout does not move byte offsets — so a
    // label's `offset` (relative to the section head) is its final index into
    // `bytes` (the section head is byte 0 of the linked image slice).
    let placed = sections.iter().find(|s| s.name == "mt_bank").ok_or("missing placed mt_bank")?;
    let off = |want: &str| -> Result<usize, String> {
        let label = placed
            .labels
            .iter()
            .find(|l| l.name == want)
            .ok_or_else(|| format!("mt_bank must export `{want}` (sound_api.emp consumes it)"))?;
        Ok(label.offset as usize)
    };
    let song_table_off = off("SongTable")?;
    let song_patch_table_off = off("SongPatchTable")?;
    Ok(MtBank { bytes, song_table_off, song_patch_table_off })
}

/// Emit the Moving-Trucks bank build inputs to `out_dir` as a THREE-WAY SPLIT per
/// shape: `mt_bank_body{,_debug}.bin` (the song streams + heads, bytes
/// `[0, SongTable)`), `mt_songtable{,_debug}.bin` (the `SONG_COUNT`*4-byte song
/// pointer table), and `mt_songpatchtable{,_debug}.bin` (the parallel patch-pointer
/// table that ends the blob). `mt_bank_blob.emp` embeds the three as contiguous
/// labeled members, so `SongTable`/`SongPatchTable` are native section labels the
/// whole-ROM link resolves — no emitted equ artifact. SHAPE-DEPENDENT (the two songs
/// the debug build adds), so each artifact has a `_debug` variant.
pub fn emit_mt_artifacts(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    require_reference_tree(aeon)?;
    let mut artifacts: Vec<(&str, Vec<u8>)> = Vec::new();
    for (debug, body_name, st_name, spt_name) in [
        (false, "mt_bank_body.bin", "mt_songtable.bin", "mt_songpatchtable.bin"),
        (true, "mt_bank_body_debug.bin", "mt_songtable_debug.bin", "mt_songpatchtable_debug.bin"),
    ] {
        let mt = emit_mt_bank(aeon, debug)?;
        let st = mt.song_table_off;
        let spt = mt.song_patch_table_off;
        // The two tables are contiguous and end the blob: body | SongTable | SongPatchTable.
        if !(st <= spt && spt <= mt.bytes.len()) {
            return Err(format!(
                "mt_bank table offsets out of order (body|SongTable|SongPatchTable): \
                 SongTable={st:#x}, SongPatchTable={spt:#x}, len={:#x}",
                mt.bytes.len()
            ));
        }
        let body = &mt.bytes[..st];
        let songtable = &mt.bytes[st..spt];
        let songpatchtable = &mt.bytes[spt..];
        // Identity: the concatenation of the three artifacts is the un-split blob.
        debug_assert_eq!(
            body.len() + songtable.len() + songpatchtable.len(),
            mt.bytes.len(),
            "mt_bank split must partition the blob exactly"
        );
        for (name, data) in
            [(body_name, body), (st_name, songtable), (spt_name, songpatchtable)]
        {
            artifacts.push((name, data.to_vec()));
        }
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    for (name, data) in artifacts {
        let path = out_dir.join(name);
        std::fs::write(&path, &data).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seam-2 slice of the real map: the two declared anchors + the four
    /// sound-bank order labels in their canonical relative order.
    const GOOD_MAP: &str = "\
order = [\"Dac_Temp_Blip\", \"SoundTablesZ80_Head\", \"Song_MovingTrucks\", \"Sfx_33\"]
[[anchor]]
name = \"dac_banks\"
at = 0x48000
when = \"sound_on\"
[[anchor]]
name = \"sound_bank\"
at = 0x58000
vma = 0x8000
when = \"sound_on\"
";

    #[test]
    fn bank_anchors_reads_the_two_declared_anchors() {
        let a = bank_anchors_from_str(GOOD_MAP).expect("valid map parses");
        assert_eq!(a.dac_banks, 0x48000);
        assert_eq!(a.sound_bank, 0x58000);
        assert_eq!(a.sound_bank_vma, 0x8000);
    }

    #[test]
    fn missing_sound_bank_anchor_fails_loud() {
        let doctored = GOOD_MAP.replace("name = \"sound_bank\"", "name = \"sound_bank_renamed\"");
        let err = bank_anchors_from_str(&doctored).unwrap_err();
        assert!(err.contains("sound_bank"), "got: {err}");
    }

    #[test]
    fn sound_bank_anchor_without_vma_fails_loud() {
        let doctored = GOOD_MAP.replace("vma = 0x8000\n", "");
        let err = bank_anchors_from_str(&doctored).unwrap_err();
        assert!(err.contains("vma"), "got: {err}");
    }

    /// A reordered `order` (SFX before its MT-bank predecessor) must desync loud —
    /// the emit derives DAC island → head bank → MT bank → SFX block in that order.
    #[test]
    fn reordered_sound_bank_order_desyncs_loud() {
        let doctored = GOOD_MAP.replace(
            "order = [\"Dac_Temp_Blip\", \"SoundTablesZ80_Head\", \"Song_MovingTrucks\", \"Sfx_33\"]",
            "order = [\"Dac_Temp_Blip\", \"SoundTablesZ80_Head\", \"Sfx_33\", \"Song_MovingTrucks\"]",
        );
        let err = bank_anchors_from_str(&doctored).unwrap_err();
        assert!(err.contains("desyncs the seam-2 chain"), "got: {err}");
    }

    #[test]
    fn missing_order_label_fails_loud() {
        let doctored = GOOD_MAP.replace("\"Song_MovingTrucks\", ", "");
        let err = bank_anchors_from_str(&doctored).unwrap_err();
        assert!(err.contains("Song_MovingTrucks"), "got: {err}");
    }
}
