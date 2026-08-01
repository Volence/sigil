# 2026-08-01 — SOUND-E1: the sound-constants ownership flip (close packet)

Status: **DONE — branches unmerged for the overseer's countersign.** Branch pair
`sound-e1-flip` (aeon `b005b36` + sigil). `engine/sound_constants.asm` (1481 ln,
the last AS-authored definition carrier in the engine) is **DELETED**;
`engine/sound/sound_constants.emp` is the sole author of the sound contract. **All
six ROM targets + every generated Z80 blob byte-identical to the chain tips;
strict 2868/0/4.** NOT merged. Spec: §2 E1 of
`docs/superpowers/specs/2026-08-01-sound-constants-flip-design.md`. seam1/seam2
untouched (their retirement is E2).

## §0 — HEADLINE

The 5 Z80 record layouts (DacSample / FmPatch / SfxHeader / SfxChannel /
SeqChannel — the FIRST Z80-consumed struct twins to flip), the request/status
mailbox, the derived Z80 RAM map, the music event-list opcode space, the SFX
engine constants, and the SongHeader layout now live in one `.emp` module. Field
offsets + total sizes flow as `offsetof`/`sizeof` derivations, never baked. The 68k
consumers import the authority; the resident Z80 blob (seam1) and the DAC-head
co-link (seam2) keep their sigil-side mirrors unchanged. The `.emp` struct
machinery handled the odd-offset Z80 words (dense byte layout, no alignment) with
no balk — no STOP.

## §1 — THE BYTE-IDENTITY PROOF

### Six ROM targets (all == chain tips)

| target | built | tip | match |
|---|---|---|---|
| s4 | `6cf74e65` / 412127 | `6cf74e65` / 412127 | ✓ (build.sh) |
| s4.debug | `16615e46` / 421958 | `16615e46` / 421958 | ✓ (DEBUG=1 build.sh) |
| demo | `9bb8c993` / 90506 | `9bb8c993` / 90506 | ✓ (native_offcanonical_full) |
| demo.debug | `bc7678d0` / 93006 | `bc7678d0` / 93006 | ✓ (native_offcanonical_full) |
| config_a | `78df5e6a` / 422297 | `78df5e6a` / 422297 | ✓ (native_offcanonical_full) |
| config_b | `f38f609b` / 303501 | `f38f609b` / 303501 | ✓ (native_offcanonical_full) |

### The generated Z80 blob (explicit artifact identity)

`emit_sound_blob --aeon . --out-dir engine/sound/generated` re-run against the
flipped tree; all 15 `.bin` artifacts md5-identical to the pre-flip snapshot
(`z80_sound_blob{,_debug}.bin` — the resident driver; `dac_sample_tab.bin`,
`dac_blip_bank.bin`, `dac_shared_bank.bin`, `mt_bank{,_debug}.bin`,
`sfx_bank{,_debug}.bin`, `sfx_blob_win_tab{,_debug}.bin`, `seq_opcode_tab{,_debug}.bin`,
`sound_tables_z80.bin`, `movingtrucks_pitchtable.bin`). Expected: the Z80 blob's
bytes come from seam1's hardcoded tables (kept), independent of both the deleted
`.asm` and the new `.emp` — E1 does not feed them yet (E2 does).

## §2 — PREMISE CORRECTIONS (finding vs reality)

1. **vblank.emp is a FOURTH 68k consumer the finding did not enumerate.** The
   census said "33 link-externs (sound_api 24 / sound_debug 7 / dac_sample_tab 2)"
   but `engine/system/vblank.emp` also `extern("SND_Z80_BASE") +
   extern("SND_CTRL_DMA_ACTIVE")` (the DMA-window flag). Swapped to the authority
   too.
2. **dac_sample_tab.emp's 2 externs are seam-2-co-link-resolved, NOT main-AS-link.**
   The finding claimed all 33 resolve "against the AS file at link". But
   `dac_sample_tab.emp` is emitted by the seam-2 DAC-head co-link, whose lower
   resolves `DAC_SAMPLE_COUNT`/`DacSample_len` via seam2's PINNED CARRIERS (its
   "stand-in for sound_constants.asm"), verified by the emit hard-erroring
   ("unknown name DAC_SAMPLE_COUNT") the moment they were swapped to `use`. Since
   E1 keeps seam2 as-is, these two stay `extern` in E1 and swap in E2 (the seam-2
   dissolution re-points the carriers at the authority). Reported deviation, not a
   hack: swapping them would force a seam2 emit-path change, which E1 forbids. So
   the E1 extern-swap count is **32** (sound_api 24 / sound_debug 6 / vblank 2),
   not 33+2.
3. **AS residual reads ~0 (confirmed) — the harvest is EMPTY, not built.** Grep of
   the whole `.asm` tree: every `SND_*`/struct-offset hit is a comment or a
   game-side re-def of an INDEPENDENT symbol (`SND_ENGINE_TABLE_BANK =
   MovingTrucks_Bank_Start >> 15`, `SFX_BLOB_BANK = SND_ENGINE_TABLE_BANK`). No
   residual `.asm` reads a sound_constants symbol at comptime. So no
   `harvest_sound_constants` was built — the spec's "if the harvest would be empty,
   DON'T build it" clause. `engine.inc` + `m1c_root.asm` just drop the include.

## §3 — CONSTRUCT-BY-CONSTRUCT PORT MAP

| sound_constants.asm | sound_constants.emp |
|---|---|
| `X = value` (flat) | `pub const X = value` |
| `X = BASE + off` (derived slot) | `pub const X = BASE + off` (derivation, visible) |
| `Struct … endstruct` (5 Z80 records) | `pub struct Struct { field: u8/u16/[u8;N], … }` (dense, `ds.w`→`u16`) |
| `Struct_len` (endstruct-generated) | `pub const Struct_len = sizeof(Struct)` |
| `alias = Struct_field` | `pub const alias = offsetof(Struct, field)` (sc_*/sx_*/SFXH_*) |
| `alias = Struct_psgenv` (sc_env unify) | `pub const sc_env = offsetof(SeqChannel, sc_psgenv)` |
| `SND_SEQ_END = base + CHROUTE_COUNT*SeqChannel_len` | same (bases × `sizeof`, derivation visible) |
| `SND_SFX_BASE = (… + $FF) & $FF00` (page-align) | same |
| `if COND … error "…"` (wall) | `ensure(!COND-inverted, "… WHY moved here")` |
| the 13-field shared-prefix `if` | 13 per-field `ensure(offsetof(SfxChannel,f) == offsetof(SeqChannel,f), …)` |
| `name function p, expr` | `pub comptime fn name(p: int) -> int { return expr }` |
| `SND_TIMERA_N = timerAReload(SND_FRAME_MILLIHZ)` | same (comptime-fn call feeding a pub const) |

Module home: **`module engine.sound_constants`** (file `engine/sound/sound_constants.emp`),
matching the flat sound-tree convention (`engine.sound_api` / `engine.sound_fm` /
`engine.dac_sample_tab` all live under `engine/sound/` with `engine.<name>` module
paths). Type-only (emits zero bytes), no `(cpu:)` tag (layout is CPU-agnostic dense
bytes) — same shape as `engine.structs`.

### The import model (why glob)

Importing a DERIVED pub const re-evaluates its RHS in the CONSUMER's scope
(item-7b's established finding), and this module's chains are DEEP
(`SND_MUSIC_PARAM_* → Snd_SpindashRev → … → SND_SEQ_END → CHROUTE_COUNT ×
sizeof(SeqChannel)`). So the consumers `use engine.sound_constants.*` (glob), which
brings every base + derivation + struct into scope. The one EXTERNAL base,
`Z80_RAM` (engine.constants, for `SND_Z80_BASE`), arrives via engine.constants'
COMPTIME_HELPER auto-glob in the real build; the standalone port harnesses seed it
as a `-D` define. `engine.sound_constants` is deliberately NOT a new COMPTIME_HELPER
— only 4 modules consume it, so an explicit glob (preserved by
`normalize_helper_imports` since it is not a helper) is cleaner than polluting every
module's namespace with ~200 sound names.

## §4 — RETIREMENTS + RE-HOMES (§4 template)

Strict **2868 → 2868** (net 0): one probe RE-HOMED in place, no test retired.

| test | file | change | re-home |
|---|---|---|---|
| `misspelled_extern_slot_is_loud` | tranche5_negative_probes | doctored `extern("SND_REQ_MUSIC")` → now doctors the bare `SND_Z80_BASE + SND_REQ_MUSIC` | the misspelling is now an unknown-name LOWER error (the glob-imported bare name), caught by `resolves()`; the loudness guarantee is preserved, the surface moved extern→comptime |

Consumer-side retirements (byte-neutral, no probe firing on them):
- `sound_debug.emp`'s local `SEQ_CHANNEL_LEN` / `SND_SEQ_TRACE_LEN` mirrors + their
  two `ensure(extern(..) == mirror)` drift guards — RETIRED. The imported
  `SeqChannel_len` (`sizeof`) / `SND_SEQ_TRACE_LEN` fold directly (comptime), so the
  displacement `(SeqChannel_len - SEQ_MIRROR_CHBYTES)(a0)` and the
  `#SND_SEQ_TRACE_LEN-1` moveq size correctly with no mirror. Re-home: the import IS
  the authority; the fit guards (`SEQ_MIRROR_CHBYTES <= SeqChannel_len`) stay,
  now over the imported name.
- sound_api.emp's `equ *_SLOT = extern(..) + extern(..)` sums → `= SND_Z80_BASE +
  SND_* ` (comptime folds); the `#extern("SND_ALIVE_MARKER/…")` immediates → bare
  `#SND_*`. Byte-identical (same values, link-fold → comptime-fold).

Test-harness ripple (all re-verified byte-identical against the reference ROMs):
- `vblank_port` / `sound_api_port` / `game_loop_port` / `load_art_port`: prepend
  `sound_constants.emp`'s items into the standalone lower (the conv-B
  prepend-the-authority pattern) + seed `Z80_RAM` as a `-D` define; drop the now-stale
  `SND_Z80_BASE`/`SND_CTRL_DMA_ACTIVE`/`SND_*` value-seam equs (they are comptime
  consts now). `m1c_root.asm`: drop the `include "engine/sound_constants.asm"` (its
  vector table reads no sound symbol — mirrors main.asm's front-matter).

## §5 — THE HARVEST DECISION

**Not built (empty).** §2.3 — no residual `.asm` reads a sound_constants symbol at
comptime, so `harvest_sound_constants` would be dead machinery. Deleting the
`engine.inc` + `m1c_root.asm` includes is the whole AS-side change. (Contrast
conv-A/conv-B/#7b, which each needed a real harvest because the residual AS folded
`ds` sizes / `phase` / abs-EA operands from the flipped symbols; the sound residual
folds none.)

## §6 — STEP-3 (retrospect) vs STEP-5 (engine opt)

- **Step-3:** the flip removes the whole 1481-line AS file, ~14 `extern()` sites
  across 4 `.emp` consumers, and sound_debug's two mirror+drift-guard pairs — a net
  reduction in twin scaffolding with zero behavior change. The `[layout.odd-field]`
  default-on WARNING fires (non-fatal, dedup'd once/compile) for the legitimately
  unaligned Z80 words (DacSample.ds_ptr @+3, SeqChannel/SfxChannel
  sc_mod_accum/@+51 / sc_base_freq/@+53 / sc_last_freq/@+55, SfxChannel.sx_patch_base
  @+59). Language-ask candidate: `check_struct_odd_fields` does NOT honor
  `@allow("layout.odd-field")` (only the DATA-side `layout.odd-item` does) — a Z80
  struct authored with deliberately-unaligned words cannot silence the lint at the
  struct/module level. Ledgered (§8); harmless here (warning, build passes).
- **Step-5:** none. Pure ownership move; no lowering changed, no ROM byte moved.

## §7 — NEITHER-BUCKET HEADLINES

- **The sc_*/sx_*/SFXH_* offset aliases + the 5 comptime fns have NO consumer in
  E1** (the Z80 driver reads them via seam1's `-D` defines, kept). They are authored
  as the AUTHORITY the flip creates — the same "VdpShadow has no `.emp` consumer"
  shape as conv-A. E2 harvests them to replace seam1's hardcoded 399-entry mirror.
- **ym_timerA_n_from_tempo/period_ns/hz are a self-referential chain never called at
  a value site** (documentation math, as in the AS twin). Ported as pub comptime fns
  (pub → not "unused"); `dac_rate_hz` + `timerAReload` ARE called (feed
  SND_DAC_RATE_HZ / SND_TIMERA_N).
- **sound_constants.emp is reached only through the 4 consumers' explicit glob** (it
  is not a COMPTIME_HELPER), so its ~40 ensures run once when the module is lowered
  in the full build (via any consumer's reachability). Confirmed by demo — which is
  SOUND_DRIVER_ENABLED=0 yet still lowers sound_constants via vblank's glob (vblank
  is in every build) and stays byte-identical.

## §8 — GAP-LEDGER SWEEP

1. **`@allow("layout.odd-field")` at struct/module scope** — the struct odd-field
   lint (`check_struct_odd_fields`, layout.rs) is unconditionally emitted; only the
   DATA-side `layout.odd-item` consults `allows_lint`. A Z80 record with
   intentionally-unaligned words (this module's 5 structs) has no way to silence it.
   Nice-to-have; non-blocking (warning only).
2. **dac_sample_tab's table-body length half** (the `10*9` LHS is a hand literal, not
   a measured section length) — unchanged by E1; still awaits a comptime
   section-length primitive (already ledgered).

## §9 — KILL-LIST CLOSURES

- The seam1 399-entry resident-blob mirror + seam2's DAC-head carriers are the E2
  targets; E1 leaves them live (spec §2 E1). No kill-list row closes here — E1
  creates the authority the E2 rows will retire against. Kill-list note: the sound
  contract now has a single `.emp` author, so the E2 rows ("seam1 `*_consts` tables
  hardcoded", "seam2 pinned DAC carriers") acquire a concrete replacement source.

## §10 — GATES (failures-first)

| gate | result |
|---|---|
| full strict `cargo test --workspace` (SIGIL_STRICT_GATE=1) | **2868 / 0 / 4** |
| `native_full_rom` (sonic4 plain+debug byte identity) | 3 / 0 |
| `native_offcanonical_full` (demo{,_debug} + config_a/b) | 7 / 0 |
| `emit_sound_blob` artifact diff (15 `.bin`) | 0 drift |
| `vblank_port` / `sound_api_port` / `game_loop_port` / `load_art_port` / `tranche5_negative_probes` | all green post-ripple |

## §11 — COMMITS (unmerged)

- aeon `sound-e1-flip` `b005b36`: sound_constants.emp + the 4 consumer swaps +
  `.asm` deletion + engine.inc drop.
- sigil `sound-e1-flip`: the 5 port-harness ripples + `m1c_root.asm` include drop +
  this note.
