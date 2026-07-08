# Design — the sound-data migration arc (migration-campaign tranche 1)

Date: 2026-07-08 (Fable, design conversation with Volence). Status: **APPROVED by Volence,
2026-07-08** — scope, all four gated ledger rulings, the emitter contract, and the staging
ratified in conversation; remaining technical calls delegated to Fable per standing practice
and recorded here as DSM.1–DSM.9. Inputs: the handoff note
(`notes/2026-07-08-sound-migration-design-handoff.md`), the #7 banks design
(`2026-07-08-spec2-plan7-item7-banks-design.md`, D7.1–D7.7 + ledger L7.1–L7.5), the 9d gate
(D9.3), the `dac_samples.emp` exhibit, and a fresh survey of aeon's live sound data
(summarized in §Facts below — file:line references are to aeon at the survey date).

## The one-paragraph thesis

Aeon's sound-data subsystem is two 32KB banks' worth of blobs and tables whose entire
correctness story is spelled by hand: `align $8000` ceremony, six `fatal` co-location/straddle
guards, twice-copied bank/window derivations, and ~150KB of GENERATED `dc.b` `.asm` nobody
reads. This arc ports the data (NOT the driver) to `.emp`, byte-identical at the ROM level:
the straddle guards collapse into `bank:` sections, the co-location fatals become explicit
cross-source `ensure(bankid(...))` link assertions, the derivations become
`bankid()`/`winptr()`/`.len`, and the generated `.asm` becomes generated `.bin` + `embed`.
It is also the demonstrated-need gate for four ledgered decisions — all four are ruled below.

## Facts (the survey, condensed)

- **Two banks, both hand-placed today.** (1) The shared DAC bank: 9 raw PCM samples,
  30,908/32,768 bytes (94% full), `align $8000` + straddle fatal
  (`dac_samples.asm`, ~line 87). (2) The MT streaming bank: engine-table head (`soundBankHead`
  Z80 phase block, `main.asm:138–140`, includes `sfx_blob_win_tab.asm`) + Song_MovingTrucks +
  pitch-table stream + patches + DrumTest (DEBUG) + HCZ2 + patches (DEBUG) + 9 SFX blobs +
  patch banks; `align $8000` at `main.asm:129`, `SND_ENGINE_TABLE_BANK =
  MovingTrucks_Bank_Start >> 15` at `main.asm:135`.
- **Six hand fatals.** Three straddle guards (`song_table.asm:91,110,128` — the 128 one also
  states the engine-table-head rule) and the SFX pair (`main.asm:231–232` straddle,
  `main.asm:239–241` co-residency with `SND_ENGINE_TABLE_BANK`).
- **The engine-table-head rule.** The sequencer reads FM pitch / log-volume / opcode tables
  window-relative from the bank head; every song stream and SFX blob must therefore share
  MT's bank id or those reads see garbage. Co-location is the binding constraint on layout —
  not fitting.
- **The DacSample descriptor** (`engine/sound_constants.asm:259–271`): 9 bytes — bank u8,
  rate u8 (reserved), codec u8, ptr u16 **little-endian**, len u16 **LE**, loop u16 **LE**.
  `sfx_blob_win_tab` is LE `dw` too. Both are Z80-consumed.
- **Handoff corrections.** (a) No zx0 anywhere in the sound path — samples are raw PCM
  (compression deliberately dropped when the shared bank landed); backlog #10 (compression
  builtins) is confirmed independent of this arc. (b) The python emitters
  (`song_packer.py`, `zyrinx_player.py`, `smps_import.py`, `sfx_transcode.py`,
  `zyrinx_port.py`) emit `dc.b` `.asm`, not `.bin` — "embed the generated blobs" requires an
  emitter contract change (ruled in DSM.4).
- **Consumption shape.** `dac_samples.asm` emits `SND_*_BANK/PTR/LEN` as CONSTANTS consumed
  by `.asm` engine code — not as a ROM data table (the exhibit's `snd_table` section is a
  demonstration shape, not the port target). Symbol flow is therefore .emp→.asm for the
  SND_* constants and .asm→.emp for `MovingTrucks_Bank_Start`/`SND_ENGINE_TABLE_BANK`.

## Decisions

- **DSM.1 — scope (Volence-ratified): sound DATA only.** `games/sonic4/data/sound/` ports to
  `.emp`; the Z80 driver AND the engine-table head (`soundBankHead` — engine tables, driver
  side) stay `.asm` this arc, per the standing "68k first, Z80 DAC last" order. The
  co-location fatals in `main.asm:110–241` port as part of the data (they guard data layout).

- **DSM.2 — L7.5 RESOLVED (Volence-ratified): reject `vma:` on `bank:` sections.** A `bank:`
  section's labels follow its placed LMA; the combination with an explicit `vma:` is a
  compile error whose message names the hazard (bankid folds VMAs, the straddle check runs in
  LMA space — a decoupled pair is a wrong-latch-on-hardware trap). Zero-cost today (all real
  sound data is VMA==LMA); relaxable on demonstrated need, the house pattern.

- **DSM.3 — 9d RE-GATED (Volence-ratified): streams stay `embed`.** The byte-command DSL is
  not built; song/sfx streams remain tool-generated blobs. D9.3's gate moves from "the sound
  migration starts" to **"a human wants to hand-author or meaningfully diff a stream in
  source."** 9c's gate (wait_frames rule-of-three) is UNTOUCHED by this arc — the ported
  files are pure data, no scripts — so 9c stays gated as written.

- **DSM.4 — the emitter contract (Volence-ratified): `--emit-bin` + `embed`.** The python
  emitters gain a `.bin` output mode (they already compute the raw bytes; `emit_asm` is a
  formatting wrapper). The `.emp` sound files `embed` the `.bin`s; blob lengths come from
  `.len`, replacing the `*_End` label pairs. Generated `.asm` disappears from the sound tree.
  A one-time tool-level check pins `--emit-bin` bytes == the bytes the `dc.b` path assembled.

- **DSM.5 — L7.1 RE-GATED, sharper (Volence-ratified): no packing linker.** With two banks
  whose layout is dictated by co-residency, auto-packing has nothing to pack; hand placement
  is two `--map` regions. New gate: **"3+ floating bank sections, or a real fit failure
  encountered in a real port."** Stays on the ledger (S2-D13/S2-D14 integration), not
  forgotten. Note the byte-identity synergy: aeon always-aligns while D7.2 bumps only on
  straddle, so the regions are PINNED at aeon's actual bank addresses anyway — hand placement
  is required for byte-identity, not merely tolerable.

- **DSM.6 — L7.4 RESOLVED: what the tables promise the driver.** (a) Descriptor VALUES are
  produced in `.emp` as link-expr constants (`bankid()`/`winptr()`/`.len` — the exhibit's
  derivations) and EXPORTED to `.asm` engine code, which keeps owning the descriptor
  emission this arc. (b) Window-relative pointers are `winptr()` — no new idiom. (c) The
  co-residency promises (engine-table-head rule, SFX co-residency) become explicit
  `ensure(bankid(X) == SND_ENGINE_TABLE_BANK, "...")` link assertions carrying the original
  fatal messages, reading the `.asm`-defined bank symbols cross-source. (d) Z80-consumed
  cells emitted from `.emp` are little-endian — DSM.7's surface. Latch-write sequences and
  re-bank protocols remain driver-side `.asm` (out of scope with the driver).

- **DSM.7 — LE u16 data cells land in T0.** The concrete surface (an explicit `u16le` cell
  type vs. endianness inference from a `cpu: z80` section context) is the implementer's call
  within one constraint: explicit-at-the-usage-site beats spooky inference when a 68k-side
  section emits Z80-consumed bytes (the DacSample case is exactly that). First intended
  customer is `sfx_blob_win_tab`; NOTE honestly that the win-tab lives INSIDE the `.asm`
  phase head today — if T3 finds it inseparable from the head, it stays `.asm` this arc and
  the LE cell remains probe-proven for the next arc. The gap is certain, only the first
  customer is conditional.

- **DSM.8 — staging (Volence-ratified): T0 gap prelude + three bank-shaped tranches.** Each
  tranche a mergeable, fully-green checkpoint:
  - **T0 — gaps + probes (the front-loaded risk):** (1) LE u16 cells (DSM.7); (2) the
    cross-source symbol probes BOTH directions — `.asm` engine code reading an `.emp`
    link-expr constant (`SND_KICK_BANK`), and an `.emp` `ensure` reading an `.asm` label
    (`MovingTrucks_Bank_Start`). If either direction fails in the mixed build, THAT is the
    arc's real blocker and it surfaces first; (3) emitter `--emit-bin` + the byte-equality
    check (DSM.4).
  - **T1 — the DAC bank:** real `dac_samples.emp` (the exhibit shape, 9 real samples),
    pinned region, SND_* constants exported to engine `.asm`.
  - **T2 — the MT streaming bank:** stream embeds placed after the `.asm` head in the pinned
    region; `song_table`/`SongPatchTable` as `.emp` data; the three song_table fatals
    replaced by section membership + no-straddle + the engine-table-head `ensure`.
  - **T3 — SFX:** blob embeds, `sfx_table`, the SFX straddle/co-residency ensures; the
    win-tab call per DSM.7.

- **DSM.9 — verification bar.** Per hand-written file: byte-diff via the ports harness
  (Plan-6 pattern). Per tranche: `sigil diff` full-ROM green vs the AS reference, workspace
  FULLY GREEN (the post-re-baseline bar — no allowlist). Stream blobs: the DSM.4 tool-level
  byte-equality check, then ROM-level identity covers them. Any divergence must be itemized
  and argued (here-fix precedent); expected divergences: none — regions are pinned at aeon's
  addresses, so even padding matches.

## Watch-outs for the implementer

- The mixed-build symbol seam is the arc's highest-risk unknown; do not start T1 until both
  T0 probes are green. The .emp→.asm direction carries LINK-EXPR values (bankid of a
  final address) — verify the AS frontend consumes a symbol whose value folds at link, not
  just comptime ints.
- The DEBUG-conditional members (DrumTest, HCZ2) mean the MT bank has two build shapes; the
  ensures and the byte-diff net must hold in BOTH (`__DEBUG__` on and off).
- Region pinning: take the bank base addresses from the CURRENT aeon build's map, and assert
  them (ensure/capacity style) rather than hard-coding silently — aeon ROM sizes drift.
- Rule-of-three standing note: if a sound table touches the table-emit seam
  (lower_offsets/dispatch/script), extract the shared shape; don't detour otherwise.
- Process: the #7 pattern — worktree off master, frozen rulings, strict TDD w/ recorded RED,
  commit-per-task, subagent-driven, two-stage reviews on load-bearing tasks, whole-branch
  adversarial review + byte-diff probes, controller verifies independently, NO merge without
  a Volence checkpoint.

## Ledger updates (for the empyrean spec integration at checkpoint)

| id | disposition |
|---|---|
| L7.1 | RE-GATED: "3+ floating bank sections, or a real fit failure in a real port" (DSM.5) |
| L7.4 | RESOLVED by DSM.6 |
| L7.5 | RESOLVED by DSM.2 (reject) |
| D9.3 / 9d | RE-GATED: "hand-authoring / source-diffing demand for a stream" (DSM.3) |
| 9c | unchanged — gate not exercised by this arc (DSM.3) |
| #10 compression builtins | confirmed independent of sound (no zx0 in the path); sequence separately |
