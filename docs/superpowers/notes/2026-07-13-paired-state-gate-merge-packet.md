# Paired-state gate merge — the churn + scene-pin + mulu batch (2026-07-13)

The reference case for the **paired-state gate rule**: two aeon features and a
sigil language addition landed together across both repos, masters pushed in
lockstep, with a two-cause strict-gate regression caught and fixed before the
push. Written up because the *sequence* — not just the result — is the pattern.

## What landed

| Input | Repo | Effect |
|---|---|---|
| churn-first ObjectTest scene (A2 soak vehicle) | aeon | +0xCC ROM both shapes; first `mulu` consumer |
| OJZ scene-pin hook (`Debug_Scene_Freeze`) | aeon | debug +0xC code, debug RAM +0x2; **plain byte-identical** |
| `mulu` (unsigned word multiply) | sigil | sigil-as can now assemble the churn scene |

Prerequisite (landed earlier, same campaign day): the **oracle profiler
stale-data fix** (`8871a17`) — without a trustworthy profiler the churn agent's
A2 re-measure (which used it) couldn't close, and this whole batch's soak
acceptance rode on it.

## The merge choreography (as executed)

1. Merge both churn branches → master, `--no-ff` (aeon `43e4d64`, sigil `6395c1d`).
2. **Two-step repin for clean attribution** (Volence's call): build both shapes at
   master, `repin` → Wave 1 = churn code-growth (+0xCC, both shapes) → commit
   (`0ecf157`). Then merge the scene-pin feat, rebuild, `repin` → Wave 2 = the
   hook's isolable signal → commit (`5125a95`).
3. **Delta checkpoint.** Wave 2 came back as `{debug RAM +0x2} + {debug ROM +0xC}`,
   plain 100% unchanged. The `+0xC` exceeded the predicted "+2 RAM" — STOPPED and
   reported before merging (the guards are debug-only *code*, 6 B × 2; the
   prediction had modeled the hook as data-only). Confirmed benign → proceeded.
4. Full strict suite → surfaced the **two-cause regression** (below).
5. Fix both causes, re-gate green (2211/0), `repin --check` clean, boot-check.
6. Push both masters **together** (aeon `c4cf2be`, sigil `6c0753d`), provenance,
   worktree cleanup, ledger close.

## The two-cause strict-gate regression (the teaching moment)

Merging the churn scene broke the strict suite in **two independent ways**, each
with a **different owner**:

- **Cause 1 — `mulu` unsupported (churn's game code).** The churn scene uses
  `mulu.w #36/#40,d0`; sigil-as had `muls` but not `mulu`, so the M1.D native
  full-build (`full_build_reproduces_sound_driver_regions`) couldn't assemble.
  **Fix (mine, demanded-features law):** add `mulu` to sigil-isa + both frontends
  (the unsigned twin of `muls`, opmode bit 8), asl-verified byte test, its own
  commit (`9131fbd`). The churn scene is `mulu`'s first consumer, exactly as the
  `proc.rs` roadmap note anticipated.

- **Cause 2 — inline-target byte slides (churn shift + hook shift).** The
  `mixed_dac_rom` reference slices hardcode abs.w/abs.l/imm32 target bytes as
  `[u8]` literals (not pins). The churn's +0xCC ROM shift slid ROM targets (→ the
  coordinator's **placement re-pin**, aeon `c4cf2be` + sigil `51e6611`), and the
  hook's +0x2 debug-RAM shift slid RAM targets in 3 debug slices (→ **mine**).
  Rather than patch the 3 instances, made the **class extinct** (Volence's call):
  pin-spliced every movable target across all 28 slices (`13b1893`). This was the
  *second bite* of the tranche-10 "repin can't track inline target BYTES"
  fragility — recurrence now structurally impossible.

Plus the `repin_pins.rs` **hand-typed tripwire** (deliberately literal, catches
silent repin corruption) consciously updated for the shifted end/MDDBG/RAM pins
(`62d898d`).

## Why the checkpoints mattered (4 stops, all correct)

Every stop returned "it's fine" or "here's the owner" — and every one prevented a
bad push instead of a post-push discovery:

1. **Profiler restart rider** — evidence (launch time > build time) showed the
   running instance already had the fix; did NOT bounce a possibly-in-use instance.
2. **Wave 2 `+0xC` delta** — benign (guard code) but unpredicted; attributed fully
   before merging.
3. **`mulu` gate red** — a churn-caused blocker; scoped the fix, got the ruling,
   shipped in-line.
4. **`mixed_dac_rom` red** — a two-cause / two-owner regression; kicked the
   placement half to the coordinator, took the inline-slice half.

## The rule this ratifies

**Paired-state gate rule:** when a merge shifts addresses that multiple repos'
hand-maintained expectations depend on, (a) attribute every delta to its cause
before merging, (b) split the fix by owner (the shifter fixes its own class), (c)
push both masters together — never leave one ahead (the "t12 stale-window"
failure mode), and (d) when a fragility bites a second time, extinguish the class,
not the instance. Predicted-delta lines must model the change's **code**, not just
its data — a hook is bytes in both senses.

## Provenance

`s4.bin` 452275 B (`0x65B60`, +0xCC; plain byte-identical to pre-hook),
`s4.debug.bin` 460268 B (`0x6765A`, +0xCC+0xC). Full sha256 in
`crates/sigil-harness/golden/PROVENANCE.md` (re-baseline 2026-07-13).
