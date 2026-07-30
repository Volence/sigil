//! seam-1 — THE RESIDENT-BLOB NATIVE LINK (OQ-1 stand-up + the §2.1 whole-blob byte
//! gate). The five sound `.emp` files (z80_sound_driver / sound_sequencer /
//! sound_sfx / sound_fm / sound_psg) linked as ONE native Z80 module set at VMA
//! `$0000` / LMA `$3DE`, blob order driver→sequencer→sfx→fm→psg, with the 47
//! intra-blob cross-file `extern proc` references resolving INTERNALLY against the
//! sibling sections' `pub proc` exports — NOT fed as per-shape `link_seam` equ
//! carriers the way the five windowed oracles do.
//!
//! This is the design's OQ-1 hard test (`2026-07-29-seam1-design.md` §1.4, §2.1):
//! the five-file combined stand-up dodging the row-1639 default-`sec0` collision.
//! Every file opens EXACTLY ONE named section and NO default `text` carrier (no
//! top-level emitting items precede its `section` block), so concatenating the five
//! independently-lowered modules collides on nothing — the "must lower as one" of
//! the design is satisfied by named-section concatenation + internal link
//! resolution, not by merging five ASTs.
//!
//! ## The two shape-dependent phenomena falling out of the REAL link (§2.2/§2.3)
//!
//!  * the sequencer's 16 internal `if DEBUG==1` bodies EMIT +$7E in the debug shape
//!    (a genuine comptime flag, not a pin), so sfx/fm/psg RE-BASE +$7E — a link
//!    output, confirmed by the debug byte gate;
//!  * the driver's five `call` sites to Sequencer_StopAll / Sfx_StopAll /
//!    SfxDispatch (all after the sequencer's growth) DERIVE their operand bytes from
//!    the real in-module label addresses per shape — the 9 cross-seam operand bytes
//!    fall out of imports, retiring the windowed oracle's `link_seam` +$7E arithmetic.
//!
//! The only equ carriers this native link supplies are the BANKED $8000-window
//! tables (SeqOpcodeTable / SfxBlobWinTab / the fm+psg LUTs) — genuinely external
//! seam-2 data, NOT intra-blob. `DacSampleTable` stays a driver `-D` (a banked
//! window address folded at comptime, per its windowed oracle).
//!
//! Comparand: the reference-ROM blob slice at LMA `$3DE`:
//!  * plain: `s4.bin[0x3DE..0x3DE+0x181C]` (6172 B)
//!  * debug: `s4.debug.bin[0x3DE..0x3DE+0x189A]` (6300 B = 6172 + $7E)
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam1_native_link
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_harness::{assemble_mixed_z80sound_as_side, assert_rom_matches_convsym};
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};
use std::path::{Path, PathBuf};

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// The blob's LMA base = `Z80_Sound_Start` = `BootData + 54`. SHAPE-DEPENDENT: the
/// debug shape grows +4 UPSTREAM of BootData (boot __DEBUG__ content), so the whole
/// blob slides to `$3E2` in debug — VERIFIED from s4.lst / s4.debug.lst
/// (`Z80_Sound_Start: 3DE` plain / `3E2` debug). This CORRECTS the design §2.1/OQ-2
/// comparand, which pinned both shapes at `$3DE` (it fixed the +$7E SIZE growth but
/// assumed the debug BASE held). The blob CONTENT is LMA-independent (all internal
/// refs are VMA `$0000`-relative), so this base only positions the reference slice.
fn blob_lma(debug: bool) -> u32 {
    if debug {
        0x3E2
    } else {
        0x3DE
    }
}
/// The plain-shape blob length (`Z80_SOUND_SIZE`, s4.lst) — 6172 B.
const BLOB_LEN_PLAIN: usize = 0x181C;
/// The debug blob length = plain + $7E (the sequencer's 16 `if DEBUG==1` bodies).
const BLOB_LEN_DEBUG: usize = 0x181C + 0x7E;

/// One resident file's placement facts: its section name and its per-shape VMA base
/// (the phase-0 blob offset). driver/sequencer are shape-INVARIANT in base; sfx/fm/
/// psg re-base +$7E in debug (they sit after the sequencer's growth). These are the
/// exact `vma()` values the five windowed oracles already re-base to and byte-match.
struct FileSpec {
    /// Path relative to the aeon root.
    rel_path: &'static str,
    /// The section name the `.emp` opens (and the export lookup key).
    section: &'static str,
    /// VMA base in the plain shape.
    vma_plain: u32,
    /// VMA base in the debug shape.
    vma_debug: u32,
    /// The comptime `-D` constant seam this file folds (its OWN subset).
    consts: fn() -> Vec<(&'static str, i64)>,
}

/// The five files in BLOB ORDER (driver→sequencer→sfx→fm→psg). The order is the
/// layout fact the native link pins (§1.4 OQ-5); the concatenation below emits in
/// exactly this order.
fn file_specs() -> Vec<FileSpec> {
    vec![
        FileSpec {
            rel_path: "engine/sound/z80_sound_driver.emp",
            section: "z80_sound_driver",
            vma_plain: 0x0000,
            vma_debug: 0x0000,
            consts: driver_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sequencer.emp",
            section: "sound_sequencer",
            vma_plain: 0x0565,
            vma_debug: 0x0565,
            consts: sequencer_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sfx.emp",
            section: "sound_sfx",
            vma_plain: 0x0CD7,
            vma_debug: 0x0D55,
            consts: sfx_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_fm.emp",
            section: "sound_fm",
            vma_plain: 0x12C3,
            vma_debug: 0x1341,
            consts: fm_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_psg.emp",
            section: "sound_psg",
            vma_plain: 0x1660,
            vma_debug: 0x16DE,
            consts: psg_consts,
        },
    ]
}

/// The BANKED $8000-window symbols the resident blob references — genuinely
/// external seam-2 data (LUTs / opcode + win tables), shape-INVARIANT (they never
/// move with the resident blob's debug growth). Supplied as equ carriers exactly as
/// the windowed oracles do. `DacSampleTable` is NOT here — it is a driver `-D`.
fn banked_carriers() -> Vec<(&'static str, i64)> {
    vec![
        ("SeqOpcodeTable", 0x856D),           // sequencer
        ("SfxBlobWinTab", 0x845F),            // sfx
        ("FmPitchTableZ", 0x8000),            // fm
        ("LogVolumeLutZ", 0x817C),            // fm
        ("CarrierMaskTableZ", 0x827C),        // fm
        ("SndDefaultPitchTable", 0x8357),     // fm
        ("PsgDivisorTableZ", 0x80BE),         // psg
        ("PsgVolEnv_Ids", 0x8284),            // psg
        ("PsgVolEnv_Ptrs", 0x828F),           // psg
        ("FmVolEnv_Ids", 0x8335),             // psg
        ("FmVolEnv_Ptrs", 0x8338),            // psg
    ]
}

// ---------------------------------------------------------------------------
// The five per-file const seams (verbatim from the five windowed oracles).
// ---------------------------------------------------------------------------

fn driver_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("CHROUTE_COUNT", 0xb), ("CHROUTE_DAC", 0xa), ("CHROUTE_FM6", 0x5),
        ("CHROUTE_PSG1", 0x6), ("DAC_SAMPLE_COUNT", 0xa), ("DacSampleTable", 0x85ad),
        ("DacSample_ds_length", 0x5), ("DacSample_ds_ptr", 0x3), ("SCF_ACTIVE", 0x1),
        ("SCF_IS_DAC", 0x10), ("SCF_IS_FM", 0x4), ("SCF_IS_PSG", 0x8), ("SCF_KEYED_B", 0x1),
        ("SHC_CMD_HI", 0x1), ("SHC_CMD_LO", 0x2), ("SHC_LEN", 0x5), ("SHC_MOD_HI", 0x3),
        ("SHC_MOD_LO", 0x4), ("SHC_ROUTE", 0x0), ("SH_CHANNELS", 0x6), ("SH_CHCOUNT", 0x3),
        ("SH_F_FM6_ADAPTIVE", 0x4), ("SH_PITCHTAB_HI", 0x4), ("SH_PITCHTAB_LO", 0x5),
        ("SH_TEMPO", 0x1), ("SH_TEMPO_MOD", 0x2), ("SND_ALIVE_MARKER", 0x5a),
        ("SND_CTRL_DMA_ACTIVE", 0x1f04), ("SND_CUR_BANK", 0x18f3), ("SND_DAC_PHASE", 0x18f0),
        ("SND_ENGINE_TABLE_BANK", 0xb), ("SND_FADE_CMD_IN", 0x2), ("SND_FADE_DELAY", 0x1),
        ("SND_FADE_DELAY_CTR", 0x1cce), ("SND_FADE_DIRTY", 0x1ccf), ("SND_FADE_SILENCE", 0x7f),
        ("SND_FADE_TARGET", 0x1ccd), ("SND_FM6_ADAPTIVE", 0x18fc),
        ("SND_FM6_CHAN_PTR", 0x18fa), ("SND_FM_KEYON_OPMASK", 0xf0),
        ("SND_MASTER_FADE", 0x1ccc), ("SND_MUSIC_PARAM_BANK", 0x1ca6),
        ("SND_MUSIC_PARAM_FLAGS", 0x1ca9), ("SND_MUSIC_PARAM_PATCHPTR", 0x1caa),
        ("SND_MUSIC_PARAM_PTR", 0x1ca7), ("SND_MUSIC_STOP", 0xff), ("SND_REG_DAC_DATA", 0x2a),
        ("SND_REG_DAC_ENABLE", 0x2b), ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_LFO", 0x22),
        ("SND_REG_LR_AMS_FMS", 0xb4), ("SND_REG_TIMER_A_HI", 0x24),
        ("SND_REG_TIMER_A_LO", 0x25), ("SND_REG_TIMER_CTRL", 0x27), ("SND_REQ_FADE", 0x1f05),
        ("SND_REQ_MUSIC", 0x1f02), ("SND_REQ_PING", 0x1f00), ("SND_REQ_SAMPLE", 0x1f01),
        ("SND_REQ_SFX", 0x1f03), ("SND_REQ_TEMPO", 0x1f06), ("SND_RING_BASE", 0x1900),
        ("SND_RING_LEAD_PRIME", 0x80), ("SND_RING_LEAD_TARGET", 0xc8), ("SND_RING_PAGE", 0x19),
        ("SND_RING_RD", 0x18f4), ("SND_RING_WR", 0x18f5), ("SND_ROM_BANK", 0x18f2),
        ("SND_ROM_LEN", 0x18f8), ("SND_ROM_PTR", 0x18f6), ("SND_SEQ_ACTIVE", 0x1a04),
        ("SND_SEQ_BADOP", 0x1a05), ("SND_SEQ_BASE", 0x1a00), ("SND_SEQ_CHANNELS", 0x1a08),
        ("SND_SEQ_CHCOUNT", 0x1a01), ("SND_SEQ_END", 0x1c9c), ("SND_SEQ_PATCHTAB", 0x1a02),
        ("SND_SEQ_TEMPO", 0x1a00), ("SND_SEQ_TEMPO_MOD", 0x1a07), ("SND_SEQ_TRACE_WR", 0x1a06),
        ("SND_SFX_QUEUE_CNT", 0x1ee4), ("SND_SONG_BANK", 0x18f1), ("SND_STACK_TOP", 0x1ffe),
        ("SND_STAT_ACK_COUNT", 0x1f12), ("SND_STAT_ALIVE", 0x1f10),
        ("SND_STAT_DAC_ACTIVE", 0x1f14), ("SND_STAT_PING_ECHO", 0x1f11),
        ("SND_STAT_TICK", 0x1f13), ("SND_TEMPO_BASE", 0x1cd2), ("SND_TEMPO_CUR", 0x1cd0),
        ("SND_TEMPO_RESTORE", 0xff), ("SND_TEMPO_TARGET", 0x1cd1),
        ("SND_TIMERA_CTRL_PROGRAM", 0x5), ("SND_TIMERA_CTRL_REARM", 0x15),
        ("SND_TIMERA_N", 0x89), ("SND_TIMERA_OVF_MASK", 0x1), ("SND_Z80_BANKREG", 0x6000),
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001), ("SND_Z80_YM_A2", 0x4002),
        ("SND_Z80_YM_A3", 0x4003), ("SeqChannel_len", 0x3c), ("Snd_PitchTabPtr", 0x1ca3),
        ("Snd_SongBase", 0x1ca1), ("Snd_SpindashRev", 0x1ca5), ("sc_detune", 0x3a),
        ("sc_dur_count", 0x4), ("sc_dur_default", 0x5), ("sc_flags", 0xa),
        ("sc_last_patch", 0x7), ("sc_macro_active", 0x3b), ("sc_mod_ctrl", 0x2a),
        ("sc_mod_ptr", 0x2), ("sc_noise_mode", 0x39), ("sc_note", 0x9),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_psgenv", 0x27),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_pt_count", 0x13),
        ("sc_route", 0xb), ("sc_stream_ptr", 0x0), ("sc_tempo_accum", 0x12),
        ("sc_tempo_mod", 0x11), ("sc_volume", 0x8),
    ]
}

fn sequencer_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_stream_ptr", 0x00), ("sc_mod_ptr", 0x02), ("sc_dur_count", 0x04),
        ("sc_dur_default", 0x05), ("sc_patch", 0x06), ("sc_volume", 0x08),
        ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_loop_ptr", 0x0C), ("sc_repeat_ptr", 0x0E), ("sc_repeat_count", 0x10),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_pt_cursor", 0x14), ("sc_points", 0x15), ("sc_transpose", 0x1A),
        ("sc_pan", 0x1B), ("sc_opbias", 0x1C), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_last_pan", 0x24), ("sc_fill_master", 0x25),
        ("sc_fill_count", 0x26), ("sc_psgenv", 0x27), ("sc_psgenv_cur", 0x28),
        ("sc_psgenv_out", 0x29), ("sc_env", 0x27), ("sc_env_cur", 0x28),
        ("sc_env_out", 0x29), ("sc_mod_ctrl", 0x2A), ("sc_mod_wait", 0x2B),
        ("sc_mod_speed", 0x2C), ("sc_mod_delta", 0x2D), ("sc_mod_steps", 0x2E),
        ("sc_mod_speed_raw", 0x2F), ("sc_mod_step_raw", 0x30), ("sc_mod_wait_raw", 0x31),
        ("sc_mod_delta_raw", 0x32), ("sc_mod_accum", 0x33), ("sc_base_freq", 0x35),
        ("sc_last_freq", 0x37), ("sc_noise_mode", 0x39), ("sc_detune", 0x3A),
        ("sc_macro_active", 0x3B), ("sx_gain", 0x40), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C),
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_REKEY_B", 5), ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC_B", 7),
        ("MEV_VOL", 0xE0), ("MEV_REST", 0x80), ("MEV_NOTE_BASE", 0x81),
        ("PsgVolEnvCtl_Loop", 0x80), ("PsgVolEnvCtl_Sustain", 0x81), ("PsgVolEnvCtl_Rest", 0x83),
        ("FmVolEnvCtl_Loop", 0x80), ("FmVolEnvCtl_Sustain", 0x81), ("FmVolEnvCtl_Rest", 0x83),
        ("TAG_MAC_NEXT", 0xE0), ("TAG_MAC_REG", 0xE1), ("TAG_MAC_LOOP", 0xE2), ("TAG_MAC_END", 0xE3),
        ("FNUM_LO", 0x284), ("FNUM_HI", 0x508),
        ("SEQEV_NOTEON", 1), ("SEQEV_NOTEOFF", 2), ("SEQEV_VOL", 3), ("SEQEV_PATCH", 4),
        ("SEQEV_DAC", 5), ("SEQEV_LOOP", 6), ("SEQEV_JUMP", 7), ("SEQEV_END", 8),
        ("SEQEV_RPT_START", 9), ("SEQEV_RPT_END", 10),
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_SILENCE_T3", 0xDF), ("CHROUTE_PSGN", 9),
        ("SND_REG_LFO", 0x22), ("SND_REG_TIMER_A_HI", 0x24), ("SND_REG_TIMER_CTRL", 0x27),
        ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_DAC_ENABLE", 0x2B),
        ("SND_FADE_DELAY", 1), ("SND_FADE_STEP", 2), ("SND_SEQ_TRACE_LEN", 0x20),
        ("SND_Z80_PSG", 0x7F11), ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_STAT_TICK", 0x1F13), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHCOUNT", 0x1A01),
        ("SND_SEQ_CHANNELS", 0x1A08), ("SND_SEQ_BADOP", 0x1A05), ("SND_SEQ_TRACE", 0x1CAC),
        ("SND_SEQ_TRACE_WR", 0x1A06), ("SND_TEMPO_CUR", 0x1CD0), ("SND_TEMPO_TARGET", 0x1CD1),
        ("SND_TEMPO_BASE", 0x1CD2), ("SND_MASTER_FADE", 0x1CCC), ("SND_FADE_TARGET", 0x1CCD),
        ("SND_FADE_DELAY_CTR", 0x1CCE), ("SND_FADE_DIRTY", 0x1CCF),
        ("Snd_SongBase", 0x1CA1), ("Snd_SpindashRev", 0x1CA5),
    ]
}

fn sfx_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_stream_ptr", 0x00), ("sc_dur_count", 0x04), ("sc_dur_default", 0x05),
        ("sc_volume", 0x08), ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_last_pan", 0x24), ("sc_base_freq", 0x35), ("sc_noise_mode", 0x39),
        ("sx_priority", 0x39), ("sx_patch_base", 0x3B), ("sx_saved_route", 0x3D),
        ("sx_kind", 0x3F), ("sx_gain", 0x40), ("sx_duck", 0x41), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C), ("SfxChannel_len", 0x44),
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC", 0x80),
        ("SFXEL_NONE", 0), ("SFXEL_FM", 1), ("SFXEL_PSG", 2), ("SFXEL_NOISE", 3),
        ("SFXH_PRIORITY", 0), ("SFXH_FLAGS", 1), ("SFXH_CHCOUNT", 2), ("SFXH_GAIN", 3),
        ("SFXH_DUCK", 4), ("SFXH_CAP", 5), ("SFXH_CHANNELS", 8),
        ("SFXHC_ROUTE", 0), ("SFXHC_CMD_HI", 2), ("SFXHC_CMD_LO", 3),
        ("SFXHC_VOICE_HI", 4), ("SFXHC_VOICE_LO", 5), ("SFXHC_LEN", 6),
        ("SHF_CONTINUOUS_B", 0), ("SHF_CONTINUOUS", 1),
        ("CHROUTE_FM3", 2), ("CHROUTE_FM4", 3), ("CHROUTE_FM5", 4), ("CHROUTE_PSG1", 6),
        ("CHROUTE_PSG2", 7), ("CHROUTE_PSG3", 8), ("CHROUTE_PSGN", 9), ("CHROUTE_COUNT", 0x0B),
        ("SFX_VOICE_COUNT", 7), ("SFX_DUCK_RAMP_STEP", 4), ("SFX_EXTEND_FRAMES", 0x0A),
        ("SFX_QUEUE_DEPTH", 3), ("SFX_ID_BASE", 0x33), ("SFX_TABLE_LEN", 0x87),
        ("SFX_BLOB_BANK", 0x0B), ("SFXID_REV_LOOP", 0xAB),
        ("SND_SFX_CHANNELS", 0x1D00), ("SND_SFX_QUEUE", 0x1EDC), ("SND_SFX_QUEUE_CNT", 0x1EE4),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SFX_DUCK_TARGET", 0x1EE6), ("SND_REQ_BASE", 0x1F00),
        ("SND_SEQ_CHCOUNT", 0x1A01), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHANNELS", 0x1A08),
        ("Snd_SpindashRev", 0x1CA5), ("SND_Z80_PSG", 0x7F11), ("SND_PSG_SILENCE_T3", 0xDF),
    ]
}

fn fm_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_route", 0x0B), ("sc_patch", 0x06), ("sc_pan", 0x1B), ("sc_transpose", 0x1A),
        ("sc_detune", 0x3A), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_opbias", 0x1C),
        ("sc_flags", 0x0A), ("sc_fill_master", 0x25), ("sc_fill_count", 0x26),
        ("sc_env_cur", 0x28), ("sc_env_out", 0x29), ("sx_gain", 0x40), ("sx_patch_base", 0x3B),
        ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_ALG_FB", 0xB0), ("SND_REG_LR_AMS_FMS", 0xB4),
        ("SND_REG_OP_DT_MUL", 0x30), ("SND_REG_OP_TL", 0x40), ("SND_REG_OP_RS_AR", 0x50),
        ("SND_REG_OP_AM_D1R", 0x60), ("SND_REG_OP_D2R", 0x70), ("SND_REG_OP_D1L_RR", 0x80),
        ("SND_REG_OP_SSG_EG", 0x90), ("SND_REG_FNUM_HI", 0xA4), ("SND_REG_FNUM_LO", 0xA0),
        ("SND_REG_KEY_ONOFF", 0x28),
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_Z80_YM_A2", 0x4002), ("SND_Z80_YM_A3", 0x4003),
        ("SND_SFX_BASE", 0x1D00), ("SND_FM_TL_MAX", 0x7F), ("SND_FM_KEYON_OPMASK", 0xF0),
        ("CHROUTE_FM6", 5), ("FmPatch_len", 0x20), ("FmPatch_fp_tl", 6),
        ("SCF_KEYED_B", 1),
        ("REGDELTA_OP_MASK", 3), ("REGDELTA_GROUP_MASK", 0x0F),
        ("REGDELTA_GROUP_COUNT", 6), ("REGDELTA_GROUP_SHIFT", 2),
        ("PITCHTAB_MAX_IDX", 0x83), ("PITCHTAB_COUNT", 0x84), ("FMPITCH_MAX_IDX", 0x5E),
        ("FNUM_HI", 0x508), ("FNUM_LO", 0x284),
        ("SND_STAT_DAC_ACTIVE", 0x1F14), ("SND_MASTER_FADE", 0x1CCC),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SEQ_PATCHTAB", 0x1A02),
        ("Snd_PitchTabPtr", 0x1CA3),
        ("SND_FM_SCRATCH", 0x1C9C), ("SND_FM_SCRATCH_LEN", 5),
    ]
}

fn psg_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_route", 0x0B), ("sc_flags", 0x0A), ("sc_volume", 0x08),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_noise_mode", 0x39), ("sc_detune", 0x3A), ("sx_gain", 0x40),
        ("CHROUTE_PSG1", 6), ("CHROUTE_PSGN", 9), ("SCF_KEYED_B", 1),
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_ATTEN_SILENT", 0x0F), ("SND_Z80_PSG", 0x7F11),
        ("SND_PSG_VOL_LATCH", 0x90), ("SND_PSG_TONE_LATCH", 0x80), ("SND_PSG_SILENCE_N", 0xFF),
        ("SND_PSG_SILENCE_T1", 0x9F), ("SND_PSG_SILENCE_T2", 0xBF), ("SND_PSG_SILENCE_T3", 0xDF),
        ("SND_PSG_NOISE_VOL", 0xF0), ("SND_PSG_NOISE_CTRL", 0xE0),
        ("SND_MASTER_FADE", 0x1CCC), ("SND_SFX_DUCK_LEVEL", 0x1EE5),
        ("PSGVOLENV_COUNT", 0x0B), ("FMVOLENV_COUNT", 3),
    ]
}

// ---------------------------------------------------------------------------
// The native link.
// ---------------------------------------------------------------------------

/// Lower one resident `.emp` file, returning its single named section (vma_base and
/// LMA still the `.emp`'s own — the caller re-bases per shape). Constants → `-D`;
/// `DEBUG` appended per shape. The intra-blob cross-file `extern proc` references
/// stay UNRESOLVED here (deferred to the joint link).
fn lower_one(aeon: &Path, spec: &FileSpec, debug: bool, doctor: Option<(&str, i64)>) -> Section {
    let path = aeon.join(spec.rel_path);
    let dir = path.parent().expect("file has a parent dir").to_path_buf();
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        spec.rel_path
    );
    let mut defines: Vec<(String, i128)> = (spec.consts)()
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v,
            };
            (n.to_string(), v as i128)
        })
        .collect();
    defines.push(("DEBUG".to_string(), if debug { 1 } else { 0 }));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {:?}",
        spec.rel_path,
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    module
        .sections
        .into_iter()
        .find(|s| s.name == spec.section)
        .unwrap_or_else(|| panic!("{} did not emit section {}", spec.rel_path, spec.section))
}

/// Stand up the native link and return the flattened blob bytes (driver→seq→sfx→fm→
/// psg concatenated), assembled with `DEBUG` as a real comptime flag per shape.
fn native_blob(debug: bool) -> Vec<u8> {
    native_blob_doctored(debug, None)
}

/// `native_blob` with an optional single-symbol DOCTOR (t24 non-vacuity control):
/// the override is applied to every file's `-D` const seam AND to the banked
/// carriers, so a doctored input that the blob genuinely reads MUST perturb the
/// output. Used by the positive controls below to prove the byte gate is not vacuous.
fn native_blob_doctored(debug: bool, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let aeon = aeon_dir();
    let specs = file_specs();

    // Lower + re-base each resident section to its per-shape VMA / LMA.
    let mut sections: Vec<Section> = Vec::new();
    for spec in &specs {
        let mut sec = lower_one(&aeon, spec, debug, doctor);
        let vma = if debug { spec.vma_debug } else { spec.vma_plain };
        sec.vma_base = Some(vma);
        sec.lma = blob_lma(debug) + vma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        sections.push(sec);
    }

    // The banked $8000-window symbols as equ carriers at harness-private LMAs (well
    // clear of the blob's $3DE..$1C78 span). Shape-invariant.
    let carrier_pairs: Vec<(String, String)> = banked_carriers()
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v,
            };
            (n.to_string(), format!("${v:X}"))
        })
        .collect();
    let carrier_refs: Vec<(&str, &str)> =
        carrier_pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = sigil_harness::test_support::assemble_equ_pairs(&carrier_refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    // ONE resolve + link over the union — the intra-blob `extern proc` references
    // resolve against the sibling sections' `pub proc` exports (the seam's payoff).
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    // Concatenate the five resident sections in BLOB ORDER.
    let mut out = Vec::new();
    for spec in &specs {
        let bytes = linked
            .section(spec.section)
            .unwrap_or_else(|| panic!("linked image missing section {}", spec.section))
            .bytes
            .clone();
        out.extend(bytes);
    }
    out
}

fn read_ref(name: &str) -> Option<Vec<u8>> {
    let path = aeon_dir().join(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            if strict_gate() {
                panic!("SIGIL_STRICT_GATE set but reference missing: {}", path.display());
            }
            eprintln!("skip: reference ROM not at {} (set AEON_DIR)", path.display());
            None
        }
    }
}

fn assert_blob_matches(blob: &[u8], expected: &[u8], debug: bool, what: &str) {
    assert_eq!(
        blob.len(),
        expected.len(),
        "{what}: length mismatch (blob {} vs reference {})",
        blob.len(),
        expected.len()
    );
    if let Some(i) = (0..blob.len()).find(|&i| blob[i] != expected[i]) {
        let lo = i.saturating_sub(8);
        let hi = (i + 16).min(blob.len());
        panic!(
            "{what}: first diff at blob offset {i:#x} (LMA {:#x})\n  blob[{lo:#x}..{hi:#x}]: {:02x?}\n   ref[{lo:#x}..{hi:#x}]: {:02x?}",
            blob_lma(debug) as usize + i,
            &blob[lo..hi],
            &expected[lo..hi]
        );
    }
}

/// §2.1 (PLAIN): the natively-linked blob == the reference ROM slice
/// `s4.bin[0x3DE..0x3DE+0x181C]`. Proves the five-file combined stand-up AND that
/// the driver's `call Sequencer_Frame` etc. resolve to the real in-module labels.
#[test]
fn native_blob_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let blob = native_blob(false);
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    assert_blob_matches(&blob, expected, false, "native blob (plain) vs s4.bin[$3DE..$1BFA]");
}

/// §2.1/§2.2/§2.3 (DEBUG): the natively-linked blob == the reference ROM slice
/// `s4.debug.bin[0x3DE..0x3DE+0x189A]`. Proves the +$7E sequencer growth EMITS (not
/// pins), sfx/fm/psg re-base +$7E as link outputs, and the driver's 9 cross-seam
/// operand bytes derive from the real imports per shape.
#[test]
fn native_blob_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let blob = native_blob(true);
    let base = blob_lma(true) as usize;
    let expected = &refrom[base..base + BLOB_LEN_DEBUG];
    assert_blob_matches(&blob, expected, true, "native blob (debug) vs s4.debug.bin[$3E2..$1C7C]");
}

/// The debug blob is exactly $7E longer than the plain blob (the sequencer's 16
/// `if DEBUG==1` bodies), and both are the canonical `Z80_SOUND_SIZE`.
#[test]
fn blob_lengths_are_canonical() {
    assert_eq!(BLOB_LEN_DEBUG - BLOB_LEN_PLAIN, 0x7E, "debug grows +$7E over plain");
    assert_eq!(BLOB_LEN_PLAIN, 0x181C, "plain blob is Z80_SOUND_SIZE = $181C");
}

/// t24 positive control (BANKED-CARRIER axis, non-vacuity): doctoring a banked
/// $8000-window symbol (`SeqOpcodeTable`, read by the sequencer's opcode dispatch)
/// perturbs the blob, so it must DIVERGE from the reference — proving the byte gate
/// genuinely compares, and the carrier seam is a load-bearing input.
#[test]
fn blob_diverges_when_banked_carrier_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(false, Some(("SeqOpcodeTable", 0x8ABC)));
    assert_ne!(doctored, expected, "the byte gate is vacuous if a doctored carrier still matches");
}

/// t24 positive control (CONST `-D` axis, non-vacuity): doctoring an absolute RAM
/// address the blob reads (`SND_STAT_TICK`, written across driver + sequencer)
/// perturbs the blob's abs-mem operands, so it must DIVERGE from the reference.
#[test]
fn blob_diverges_when_const_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let base = blob_lma(false) as usize;
    let expected = &refrom[base..base + BLOB_LEN_PLAIN];
    let doctored = native_blob_doctored(false, Some(("SND_STAT_TICK", 0x1DED)));
    assert_ne!(doctored, expected, "a moved SND_STAT_TICK must change the blob's abs-mem operands");
}

// ===========================================================================
// §2.4 — THE WHOLE-ROM DUAL-BUILD GATE (gate-off ≡ pure-AS canonical; gate-on =
// the native blob spliced into the real ROM ≡ the same canonical bytes).
// ===========================================================================

/// The placed native-blob sections for the WHOLE-ROM mixed link: the five files
/// lowered (const seam `-D` + the shape `DEBUG` flag), re-based to their per-shape
/// VMA, and pinned at LMA `blob_lma(debug) + vma`. NO banked equ carriers here — in
/// the whole ROM the banked `$8000`-window tables are STILL AS-included (seam-2), so
/// SeqOpcodeTable / SfxBlobWinTab / the FM+PSG LUTs resolve against the AS side
/// through the joint link; the intra-blob `extern proc`s resolve internally.
fn placed_blob_sections(debug: bool) -> Vec<Section> {
    let aeon = aeon_dir();
    let mut sections = Vec::new();
    for spec in &file_specs() {
        let mut sec = lower_one(&aeon, spec, debug, None);
        let vma = if debug { spec.vma_debug } else { spec.vma_plain };
        sec.vma_base = Some(vma);
        sec.lma = blob_lma(debug) + vma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        sections.push(sec);
    }
    sections
}

/// Compose the AS side (gate ON) with the placed native blob and emit the full ROM
/// through the whole-ROM `sigil.map.toml` — the `build_mixed_rom` shape.
fn build_seam1_rom(debug: bool) -> Vec<u8> {
    let aeon = aeon_dir();
    let as_module = assemble_mixed_z80sound_as_side(&aeon, debug).unwrap_or_else(|e| panic!("{e}"));
    let mut sections = as_module.sections;
    sections.extend(placed_blob_sections(debug));

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (mixed seam1): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (mixed seam1): {d:?}"));

    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read map {}: {e}", map_path.display()));
    let map = sigil_link::load_map(&map_src).unwrap_or_else(|e| panic!("load map: {e}"));
    sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom (mixed seam1): {e}"))
}

/// §2.4 (PLAIN): the native blob spliced into the REAL ROM (gate ON) is
/// byte-identical to the canonical `s4.bin` (modulo the convsym/fixheader header
/// fields, derived + confined). The downstream engine + banks are unchanged by
/// construction (the blob is the same length).
#[test]
fn mixed_seam1_rom_matches_reference_plain() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let rom = build_seam1_rom(false);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        sigil_harness::pins::ASSEMBLED_LEN,
        "seam1 mixed (plain) vs s4.bin",
    );
}

/// §2.4 (DEBUG): the native blob (with the +$7E sequencer growth EMITTED, blob base
/// $3E2) spliced into the REAL debug ROM is byte-identical to canonical
/// `s4.debug.bin`.
#[test]
fn mixed_seam1_rom_matches_reference_debug() {
    let Some(refrom) = read_ref("s4.debug.bin") else { return };
    let rom = build_seam1_rom(true);
    assert_rom_matches_convsym(
        &rom,
        &refrom,
        sigil_harness::pins::DEBUG_ASSEMBLED_LEN,
        "seam1 mixed (debug) vs s4.debug.bin",
    );
}

/// t24 WHOLE-ROM positive control: doctoring the blob's `SND_STAT_TICK` `-D` must
/// make the spliced ROM DIVERGE from canonical — proving the whole-ROM gate is not
/// vacuous (the blob's bytes genuinely enter the ROM). Built via a doctored blob.
#[test]
fn mixed_seam1_rom_diverges_when_blob_doctored() {
    let Some(refrom) = read_ref("s4.bin") else { return };
    let aeon = aeon_dir();
    let as_module = assemble_mixed_z80sound_as_side(&aeon, false).unwrap_or_else(|e| panic!("{e}"));
    let mut sections = as_module.sections;
    // Doctored blob: SND_STAT_TICK moved on the .emp side only.
    for spec in &file_specs() {
        let mut sec = lower_one(&aeon, spec, false, Some(("SND_STAT_TICK", 0x1DED)));
        sec.vma_base = Some(spec.vma_plain);
        sec.lma = blob_lma(false) + spec.vma_plain;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        sections.push(sec);
    }
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link: {d:?}"));
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sigil.map.toml");
    let map_src = std::fs::read_to_string(&map_path).unwrap();
    let map = sigil_link::load_map(&map_src).unwrap();
    let rom = sigil_link::emit_rom(&linked, &map).unwrap_or_else(|e| panic!("emit_rom: {e}"));
    // The doctored blob region must differ from canonical somewhere in $3DE..$1BFA.
    let base = blob_lma(false) as usize;
    assert_ne!(
        &rom[base..base + BLOB_LEN_PLAIN],
        &refrom[base..base + BLOB_LEN_PLAIN],
        "the whole-ROM gate is vacuous if a doctored blob still matches canonical"
    );
}
