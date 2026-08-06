# 2026-08-06 — HANDOFF after the overnight overseer session (five lanes merged)

Written at the close of the session that merged hyg → srmask → trackc → pw →
ltr-mul. **Everything below was verified at the time of writing. Concurrent
sessions move masters — re-derive before acting.**

## State you inherit (VERIFY FIRST)

| | commit | note |
|---|---|---|
| sigil master | `0ed8a7da` | golden chain **49** |
| aeon master | `c29ffbc` | |

- Both clean, single worktree each, all of tonight's branches deleted. (Many
  OLD branches from earlier sessions remain in both repos — not mine, not
  cleaned, don't assume they're live.)
- **UNPUSHED: sigil 57 / aeon 12. Pushing is VOLENCE'S CALL — do not push.**
- Final countersign, own-run at the merged tip: strict **3502 passed / 0 failed
  / 4 ignored = 3506 == master's `#[test]` total**; `refreeze --check` OK (tip
  `ltr-mul`, chain 49); warn tiers 19 plain-family / 18 debug-family.
- Chain-49 CRCs (full/anchor; anchor SIZES stable, content moved):
  s4 `3b6cad91`/`09beac0f` · s4.debug `e3963874`/`2263979c` ·
  demo `b8df1c2b`/`e8988a2f` · demo.debug `30173928`/`3b5b11e9` ·
  config_a `7660f157`/`1b3b3708` · config_b `ace527ba`/`e6e20c75` ·
  lean `69c20328`/`cd73fb65`.
- Canonical ROMs rebuilt from merged master and verified: aeon `s4.bin`
  `3b6cad91`, `s4.debug.bin` `e3963874`. The emulator is loaded with the debug
  shape.

The authoritative record is the campaign log TAIL:
`~/.claude/projects/-home-volence-sonic-hacks-sigil/memory/spec2-progress.md`
(the 2026-08-06 overnight-overseer entry). Read that before this file.

## VOLENCE'S TWO OPEN GATES — do not pre-empt either

1. **The push.** Both repos are ahead of origin. His call, not yours.
2. **The arc-A play-test.** The byte-changing multiply parcel changed TIMING
   ONLY. Lag frames are **identical, not reduced** — do not tell him to expect
   a speedup. Music needs `DEBUG=1 SOUND_DEBUG_HOTKEYS=1`. The merge is cleanly
   revertible if he rejects it.

## Follow-up queue, in priority order

1. **`collision_lookup.emp:62-66` — the ×80 two-power chain in
   `Collision_GetType`.** The per-sensor-per-frame collision probe: HOTTER than
   every site the byte-changing parcel touched. Pure drop-in
   (`mul_const.w d1, #80, d2`): 40 → 32 cy, 8 B → 8 B, ZERO size ripple, d2
   already licensed by `clobbers(d1-d3/a0)` and dead after. Byte-CHANGING, so
   it needs a refreeze + behavioural A/B + panel — which is exactly why it was
   deferred rather than ridden in at 4am.
2. `entity_window.emp:1605-1612` ×22 chain (adoptable even under the old ≥3
   gate). And `Section_FlatIDXY`'s `.fxy_mul` — NOT a pure swap: it accumulates
   straight from memory, so `mul_bounded.w` needs a register preload at a site
   with only d1 spare. All three are ledgered with measured numbers.
3. **Lane E — edge-model** (rows 2148/2149/2152 + the djnz raw Follow-leg): an
   `Edge::Defer` → `TailOut`/`BranchOut` split retiring `out_verify::
   is_uncond_tail`, a new ISA-crate `is_call`/`is_return`/`is_branch`
   classifier, and the walk-level `falls_into` policy field. Cross-analysis
   blast radius (preserves / out_verify / z80_bus). Never dispatched.
4. **Lane X — extractor fold** (rows 2175/2176): four bare-`Sym` matchers onto
   `transfer_target_sym`, each a scoped soundness parcel needing a MEASURED
   oracle re-run. **Row 2175 is WRONG about one of them:** `preserves::
   call_target` is not the byte-identical spelling the row claims — it is a
   rev-find with no `$`-hygiene filter, so folding it changes behaviour on two
   axes, not one. Never dispatched.
5. Row 2090's **68k invariant enforcer** — and note the ordering trap recorded
   in the row: teaching `validate_module_invariants` the SR family ALONE would
   take the credit live with still no enforcement. Enforcement FIRST.
6. The `restore_slot` POP/PEEK unification (structural root of the pw peek
   hole); deleting the now-dominated two-power arm 5a in `mul_lower.rs`; the
   `Parallax_CheckBoundary` d2 live-caller contract contradiction.

## Corrections to standing doctrine — believe these over older docs

- **The "5-site ripple" is now THREE sites.** `engine.inc` and
  `mixed_dac_rom.rs` were DELETED from both repos (verified `git ls-files`).
  Live surface: `pins.rs` (auto via repin) + `crates/sigil-harness/tests/
  repin_pins.rs` (hand literals) + `repin.toml` (only when a region is ADDED).
  The 2026-08-06 overnight handoff brief and the byte-changing spec's R6 both
  repeated the stale five — R6 is corrected on master.
- **A `pause`-anchored oracle A/B is INVALID for a cycle-changing parcel.**
  `pause` stops at an arbitrary intra-frame instant whose phase differs between
  ROMs *because* the timing changed, so any poke lands at a different point in
  the frame and the scene diverges from your own stimulus. Anchor on a
  deterministic PC (`run_to VInt_Level`, `wait_for_break`) BEFORE poking. A
  differing VDP *register file* with identical VRAM/CRAM/VSRAM is a transient
  latch; a differing *framebuffer* with identical VDP memory is raster phase.
  Neither is a defect.
- **Verify the loaded cart by HASH, not the reload diagnostic.**
  `reload_rom` reported "reload was silently rejected" when the load had
  SUCCEEDED. Use `memory_hash addr=0 len=<file size>` vs the expected CRC32.
  Also re-check `Frame_Counter` after any gap — the emulator free-runs while
  you do other work.
- **`git -C <abs path>` for every repo-targeted git command.** A leading `cd`
  in a compound command retargeted a merge this session and inverted a
  determined land order. Same class as a porter leaving work staged in a main
  checkout (which also happened, and was cleaned).

## What the session proved about process — keep doing this

- **Lens panels are the highest-yield thing in the loop.** FOUR gate-green
  lanes carried real soundness defects; every one was caught before merge, and
  two were found independently by two different lenses. Never merge on green
  gates alone.
- **Mandatory drift checkpoints before building an old spec** caught a
  FICTIONAL flagship (the niche-option spec's Alloc-$FF target does not exist)
  before a line was written on it.
- **Census discipline:** "verifying the members of a set is not verifying the
  set", and re-running a census with the same mental template reproduces its
  blind spot. The scout census here said seven; there were eleven.
- Land order for two-repo parcels is DETERMINED (by dependency) or MEASURED,
  never assumed — and some parcels are land-together with both windows red.

## Standing rules (unchanged, all paid for)

Porter briefs open with the foreground-shells-through-final-report hard rule
(`porter-brief-boilerplate.md`, prepend it). Worktrees in `.worktrees/`, seeded
(`games/sonic4/data/editor/` + `engine/debug/generated/`), baseline-proven
byte-identical BEFORE any edit. Merges: precondition-gated, strictly
sequential, countersign = `git log master..branch` EMPTY + clean tree; refresh
every in-flight lane's worktree after any merge. All oracle MCP work in the
OVERSEER'S foreground only — never a subagent. Packets carry no merge-state.
Provenance is CRC32+size, never SHA1. Byte bar = seven targets, derived from
`crates/sigil-harness/golden/`, never assumed.
