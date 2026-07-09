# 68k engine-port campaign — kickoff handoff (written overnight 2026-07-09)

For Volence's morning. Both overnight arcs are done and checkpoint-ready (details in their
own notes); with them merged, **Plan 7 is COMPLETE (#1–#10 all landed)** and Spec 2 is ready
to FREEZE. This note proposes the freeze checklist, the first migration targets, and the
morning checkpoint list.

## Morning checkpoint list — ITEMS 1–3 EXECUTED 2026-07-09 (Volence approved; Fable ran the sweep)

1. ~~Merge sigil `seam-reeval` → master~~ **DONE — merge `bf247da`** (`--no-ff`). Item-B
   ruling (`extern("NAME")` + AS equ export) + Item-A dispositions ratified with the merge
   (`specs/2026-07-09-seam-reeval-decisions.md`; packet:
   `notes/2026-07-09-seam-reeval-complete.md`).
2. ~~Merge sigil `compression-builtins` → master~~ **DONE — merge `2576d91`** (`--no-ff`;
   packet: `notes/2026-07-09-compression-builtins-complete.md`).
   **Post-merge master validation: 1654/0 workspace, strict gates 15/0, clippy clean;
   pushed. Worktrees removed, branches deleted. Plan 7 #1–#10 COMPLETE ON MASTER.**
3. ~~Commit the empyrean spec working tree~~ **DONE — empyrean `070a118`, pushed** (the
   three stacked passes D2.25/D2.26/D2.27 + ledger dispositions + the Plan 4/5/6 plan
   docs; the docs-cadence backlog is clear).
4. **Ratify this kickoff** (freeze checklist + first targets below) → campaign starts.
   Volence indicated go (2026-07-09 morning); the freeze ceremony below is the campaign
   session's first act.

## Spec-FREEZE checklist (proposed)

Plan 7's contract was: research → implement → FROZEN spec. With #10 merged:

- [x] ~~Volence commits the empyrean working tree~~ DONE (`070a118`) — the spec text now
      matches shipped reality through D2.27.
- [ ] Declare §10's concept inventory CLOSED for v1 (the A-Spec2.3 gate stays for
      amendments; the headroom rule already makes future additions non-breaking).
- [ ] The deferred ledger is the freeze's "explicitly NOT in v1" list — every row has an
      owner/unblock condition as of tonight's re-evaluation (S2-D14 + 9d re-affirmed with
      full-arc evidence; nothing left in "undecided" state).
- [ ] Tag/record the freeze in empyrean (a short FREEZE note or a `SIGIL_SPEC2` version
      stamp — Volence's call on ceremony level).
- [ ] Known non-blockers carried into the campaign (recorded, not gating): F1 flake watch
      item; the mt/sfx extern()-migration ride-along; `.emp` adoption of `s4lz()` in
      aeon's build (below).

## First migration targets (surveyed tonight, ranked by blast radius)

The campaign's port loop per file: add a `SIGIL_EMP_<NAME>` gate (copy the exact
`ifndef … include … else org $ADDR endif` spelling from `games/sonic4/main.asm:111/:154/:232`),
write the `.emp`, pin the region in `sigil.map.toml`, byte-gate both shapes, negative
probes, merge. All six code candidates below are `__DEBUG__`-define-free — the cheap
(sfx-style) gate shape.

**Code targets (start here):**
1. **`engine/system/hblank.asm`** (18 lines) — 2 labels, 1 imported RAM equ
   (`HBlank_Handler_Ptr`, now readable via `extern()` if needed), 2 referencing files.
   The ideal first code port.
2. **`engine/system/controllers.asm`** (62 lines) — straight-line I/O, standard local
   labels, single caller (`vblank.asm`).
3. **`engine/system/math.asm`** (27 lines) — `GetSineCosine` + a BINCLUDE sine table
   (= `embed()`); more callers (player_ground ×4) but call sites only need the symbol.
4. Then: `collision_lookup.asm` (44 ln, 6 imports), `vdp_init.asm` (47 ln).

**Data quick wins (interleave anytime):** `vram_bases.asm` (8 ln, pure equ arithmetic —
now expressible end-to-end: `.emp` equ export + AS reads), `ojz_act_pool.asm` (14 ln,
BINCLUDE×3 + dc.l pointer table — the proven dac_samples shape), `particle_anims.asm`
(15 ln, the `offsets` construct's shape), `plantbadmaps_anims.asm` (6 ln).

**Deliberately deferred (hazards, surveyed):** `vectors.asm` (tiny but ~20-symbol fan-in +
org 0 header adjacency), `z80_init.asm` (Z80 payload), `game_loop.asm` (SOUND_DRIVER_ENABLED
ifdef + game-supplied `gameDebugTick` macro — port after the gate pattern is proven),
macro-heavy data (parallax, test_mappings' `sprSize()`, objdefs) — these want the macro-arg
story exercised deliberately, not stumbled into on port #1.

**Byte-gate infrastructure already in place:** harness gates diff the full main.asm tree vs
`aeon/s4.bin` (pins in `crates/sigil-harness/golden/PROVENANCE.md`: plain 451198 B
`8ce6dd7e…`, debug 458982 B `13c7b063…`); mixed-build harness + convsym allowlists proven
across three sound tranches; `extern()` closes the cross-seam constant-read gap the sound
arc kept hitting.

**Recorded follow-up riding the campaign:** `.emp` adoption of `s4lz()` inside aeon's
build (replacing the tools/s4lz.py call sites in ojz_block_gen's flow) — its own byte-gate;
the K-sweep/dict-selection logic stays caller-side. Also the mt/sfx co-residency ensures →
`extern("SND_ENGINE_TABLE_BANK")` ride-along.

## Suggested campaign cadence

Port #1 (hblank) in one sitting including the gate-pattern writeup; then batch 2–3 small
files per tranche with the same worktree/checkpoint discipline as the sound arc. Re-evaluate
after the first tranche whether code ports surface new spec gaps (the ledger's
ride-the-tranche items are queued for exactly that).
