# Boundary-crossing transition parcel — rig A/B evidence log

Per-fix before-repro + after-clean, driven by the crossing-drive rig
(notes/2026-07-23-crossing-drive-rig-protocol.md). Canonical debug ROM used for
live proof; both shapes rebuilt fresh per fix.

Baseline canonical (pre-parcel): plain `00f609a5`/421089 · debug `80d14183`/429134.

---

## B6 — promote-frame CC-clobber (rebuild skip) — CLOSED

**Bug:** `Parallax_Update` promote path ended with `move.l #0,Target` (Z=1 from
its immediate source), so `.config_resolved: beq .no_config` was taken on every
smooth-transition promote frame → entire Step5+Step4+fill rebuild skipped →
Hscroll/Vscroll keep the previous frame's contents = one-frame parallax freeze.

**Fix (both twins, length-neutral reorder):** move the `move.l d0,Current` to be
the LAST of the three promote writes, so `.config_resolved` reads Z from d0 — the
same "active config in d0, Z reflecting it" invariant that the `use_target` /
`use_current` paths already satisfy. `parallax.emp` :366-373, `parallax.asm`
:229-236.

**Rig A/B (Hscroll_Buffer sentinel-overwrite, config-agnostic):**
Setup: OJZ scene, `Debug_Scene_Freeze=1`, camX poked 1024, baseline settled
(Scroll_B −512), stage `Target=OJZ_Default, Frames=1`, sentinel `Hscroll_Buffer`
ends (`AA` ×16 at `0xFF850A` and `0xFF887A`), drive the single promote frame.

| | promote-frame Hscroll_Buffer (both ends) | Current_Config | Target | Frames |
|---|---|---|---|---|
| control (normal frame) | overwritten `FC00FE00…` | — | — | — |
| **before-repro** (canonical `80d14183`) | **`AA…AA` survived** (rebuild SKIPPED) | promoted `0x11428` | `0` | `0` |
| **after-clean** (fixed `7460a0c2`) | **`FC00FE00…`** (rebuild RUNS) | promoted `0x11428` | `0` | `0` |

Promotion completes correctly in both (Current←Target, Target cleared); the fix
only restores the rebuild on that frame.

**Scope class:** byte-CHANGING, **length-NEUTRAL** (pure reorder — same three
opcodes). Both shapes keep size + `EndOfRom` (`0x5DB60`): plain 421089, debug
429134. So NO region-slide ripple (controllers..sound_api bases, engine.inc
orgs, repin all unchanged). New canonical (fresh dual builds): plain
**`bb5ddc5a`**/421089 · debug **`7460a0c2`**/429134.

**Gate:** full paired strict **2488/0** (SIGIL_STRICT_GATE, AEON_DIR=branch tree).

---

## Window-slide mask-migration observation — (pending)

## B2 — mode-contract design pass — (pending; GATE CHECKPOINT before cutting)

## B3 — frames-remaining ramp — (pending; inside B2's state machine)

## B1 — re-cross cancel branch — (pending; inside B2's state machine)
