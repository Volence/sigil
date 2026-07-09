# Next-session handoff — bg restore first, then sound-migration T2

Written 2026-07-08 (Fable, end of the T0+T1 session) for the next session. Volence has ratified
both next steps; no design conversation needed for step 1, a SHORT design-check for step 2.

## Where everything stands

- **Sound-migration T0+T1 MERGED & PUSHED**: sigil master `d53aa65`, aeon master `e5b256c`.
  Read `notes/2026-07-08-sound-migration-t0-t1-complete.md` + the plan's `## Execution notes`
  (`plans/2026-07-08-sound-migration-t0-t1.md`) for what shipped and every delta. Workspace
  FULLY GREEN (~1429 tests), corpus all-identical, mixed .asm+.emp ROM byte-identical.
- **Spec integration for D2.26 done** in empyrean's WORKING TREE (uncommitted — Volence's
  cadence; it stacks on the also-uncommitted #7/D2.25 pass). Do not commit empyrean.
- Memory file [[spec2-progress]] is current through the merge.

## Step 1 (do first, small & independent): restore the newer forest background

> **STATUS 2026-07-08 (executed): DONE pending Volence boot-check.** Stash APPLIED clean
> (stash@{0} kept until the aeon commit lands — drop it then). One fix needed: the generator's
> OUT path predated the engine/game split (`data/` → `games/sonic4/data/`); regenerated JSON is
> hash-identical to the stashed copy (deterministic). Both ROMs rebuilt per PROVENANCE.
> Lengths UNCHANGED (451198 / 458982; EndOfRom 0x658B4 / 0x673A2 — the +64 tiles fit in align
> padding), content changed (new sha256s in PROVENANCE.md re-baseline section). DAC $50000 /
> MT $60000 UNSHIFTED → no dac_port golden churn; T2 can pin now. Collision bins verified
> byte-identical. Harness + full workspace green (1429 tests). Oracle screenshots show the
> dual-tree colonnade rendering (varied trunk widths). Stash also carried untracked extras:
> `docs/research/s3k_art_style_demo.html` + `games/sonic4/data/sprites/plantbadmaps/` (inert
> for the build) — Volence should decide their fate at commit time. Note the entity_data diff
> adds two Path Swap objects in section 1 and moves rings/a solid block — gameplay tweaks
> beyond the bg, part of the parked work; flagged for the boot-check.

Volence noticed the game shows an OLDER bg. Root cause (investigated, confirmed): aeon
`stash@{0}` — "park forest_bg_gen + editor experiments during byte-exact pin" — holds the NEWER
dual-tree colonnade generator (PAT_W 128→256, two distinct trees/module, hash-based wall,
340 tiles vs 276) plus editor-export tweaks (`entity_data.asm`, `vram_bases.asm`). It was parked
during a byte-exact pin and never unparked.

Process (each step verified before the next):
1. In aeon (clean master): `git stash pop` (or apply; stash files don't overlap the sound-arc
   changes — verify no conflicts). Review what landed; Volence ratified the RESTORE, not blind
   application of unrelated "editor experiments" — eyeball the entity_data/vram_bases diffs and
   flag anything surprising.
2. Regenerate: `python3 tools/forest_bg_gen.py` (writes `editor_bg_override.json`).
3. Rebuild BOTH ROMs per PROVENANCE: `DEBUG=1 SOUND_DRIVER_ENABLED=1 ./build.sh sonic4`, then
   plain `./build.sh sonic4`. The ROM CHANGES (that's the point).
4. Re-baseline the sigil harness per `crates/sigil-harness/.../PROVENANCE.md` flow (the m1d +
   mixed_dac_rom pins: ASSEMBLED_LEN values and any convsym offsets may move — update them in
   ONE place now that assert_rom_matches/allowlists live in the harness lib). ALL harness tests
   green against the new reference before committing anything.
5. Volence should boot it (oracle emulator MCP tools exist if useful) and confirm the bg looks
   right before the aeon commit.

⚠ Cautions: (a) aeon's tools pytest suite CLOBBERS untracked collision/*.bin — do NOT run the
full suite casually; if it happens, the current tables can be re-extracted from the PRE-restore
ROM bytes at $2C1FA/$2D1FA/$2E1FA/$2E2FA (sizes 4096/4096/256/256) — but note after the bg
rebuild the addresses may shift; re-derive from s4.lst (HeightMaps label). (b) Collision stays
PINNED (S&K-import tables) — Volence deferred the OJZ collision switch; do not adopt generator
output as a side effect of the rebuild (the build only regenerates collision if editor
project.json exists — it doesn't — but VERIFY the four .bin files are byte-unchanged after the
rebuild anyway).

## Step 2: sound-migration T2 — the Moving Trucks streaming bank

The design (DSM.1–9, `specs/2026-07-08-sound-migration-tranche1-design.md`) already covers T2's
shape; hold a SHORT design-check with Volence only if real files contradict it. Scope: the MT
bank streams (song_movingtrucks, pitchtable_stream, patches, DrumTest, HCZ2 + patches as
`embed` of `--emit-bin` outputs), `song_table`/`SongPatchTable` as .emp data, the three
song_table fatals → section membership + no-straddle + cross-source
`ensure(bankid(X) == SND_ENGINE_TABLE_BANK, ...)` asserts. The engine-table head
(`soundBankHead`, main.asm:138–140) STAYS .asm (driver side); the .emp streams pin AFTER it in
the same bank. Everything needed is proven: equ export, both-direction seam (ports.rs probes),
`--emit-bin` (24/24 byte-equal, no interior labels), the gate pattern (SIGIL_EMP_DAC precedent —
T2 wants its own define, e.g. SIGIL_EMP_MT).

T2 wrinkles the DAC tranche didn't have:
- DEBUG-conditional members (DrumTest, HCZ2) → TWO build shapes; ensures + byte-diff must hold
  in both (mixed_dac_rom.rs already runs both variants — extend it).
- `MovingTrucks_Bank_Start`/`SND_ENGINE_TABLE_BANK` are DEFINED in main.asm between the head and
  the streams — the .emp side references them cross-seam (Probe B pattern).
- `movingtrucks_pitchtable.asm` (the ENGINE-DEFAULT inline copy, inside the Z80 phase blob) is
  DRIVER-side — NOT ported in T2. Only the _stream copy is. Its hand-edit-vs-generator conflict
  (SndDefaultPitchTable alias, commits 5556d76/6c44e46) must be reconciled before anyone
  regenerates it — flag to Volence if touched.
- The bank base for MT is $60000 TODAY but moves if step 1's bg rebuild grows earlier content —
  wait, it can't: MT is align $8000 AFTER the DAC banks at $50000-$60000... but if pre-DAC
  content grows past $50000 the WHOLE sound block shifts and the dac pins break LOUDLY (the
  linker overlap/region errors) — if that happens, re-pin dac + MT regions from the new s4.lst
  and update dac_port.rs goldens + the .emp header comment. Do step 1 FIRST so T2 pins against
  the final layout.

Process: the #7/T0+T1 pattern — worktree off sigil master (though T2 is mostly aeon-side .emp +
harness tests; sigil-side changes should be near-zero — that's the point of T0), plan doc with
frozen rulings, TDD, subagent-driven, two-stage reviews on load-bearing tasks, byte-diff nets,
NO merge without a Volence checkpoint.

## Open items ledger (unchanged from the completion note)

1. Collision adopt-vs-pin: PINNED for now (Volence: "we plan on changing collision eventually").
2. Tools-suite clobber guard: unguarded; just don't run the full suite blindly.
3. Production sigil map home for the equ carrier `text` section (cutover-era, not urgent).
4. Empyrean working tree: #7 + D2.26 integrations uncommitted (Volence commits).
