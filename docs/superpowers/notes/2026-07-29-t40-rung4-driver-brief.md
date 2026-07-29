# 2026-07-29 — t40 brief: Z80 rung-4 — the T-STATE CAPABILITY + the z80_sound_driver port

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target: `engine/sound/z80_sound_driver.asm` (1495 L, 21 routines per t32 §5.1) →
`z80_sound_driver.emp` — the TOP of the sound stack and the LAST Z80 code file. The
rung ladder reserved rung 4 for exactly this file's three demands: the cycle-exact DAC
loop (T-states), the di/ei interrupt lattice, and the shadow register set (exx /
ex af,af'). **THE T-STATE CAPABILITY DESIGNS AND SHIPS FIRST** — no transcription
before the design gate passes. Sigil master = THIS brief's commit; aeon master
**`fa474cd`**.

## 0. Bars

- Canonical: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102**. Strict baseline
  **2856/0 (1 ignored)**. EXPECTED BYTE MOVEMENT ZERO, STOP-not-absorb (the driver IS
  the front of the blob — everything slides behind it).
- Branches `port-tranche40` BOTH repos, worktrees `.worktrees/port-tranche40`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first --no-fail-fast, keep commits small). The
  other five resident Z80 files (+ all their .emp) READ-ONLY. Parallel porter on
  `port-tranche39` (68k objects — different files).
- **STOP 1 = the step-0 DESIGN GATE** (the t36 trampoline precedent): recon + window
  derivation + the T-state capability design, committed, then STOP for endorsement.
  Then (a) after step 1, (b) after the loop/panel, (c) mine.

## 1. Step-0 deliverables (the design gate's cargo)

1. **Recon vs the t32 §5.1 row** (21 routines, ~17 Clobbers + 4 Preserves + 3 Out —
   indicative): census the real contract surface, the di/ei sites, the exx/ex af,af'
   sites, the DAC loop's in-source cycle annotations (rung 4 OWNS T-states — C1
   activates for the first time on the Z80 side), and the 2 die-at-port boundary
   externs sfx declared (they become checked defs here — trust conversion, the
   Mod_ReArm precedent; state it).
2. **Window derivation BOTH shapes** from the listings (the driver is the blob's
   FRONT — expect base near the blob origin; verify its own `__DEBUG__` content and
   the downstream shift it feeds).
3. **THE T-STATE CAPABILITY DESIGN** — read the reserved design sections in the rung
   docs (2026-07-29-z80-rung2-contracts.md + the t32/t33 design notes name what rung
   4 was reserved to build) and the DAC loop's actual annotations, then design:
   - the annotation/contract surface (per-proc? per-loop? `cycles(...)` clause?) —
     ONLY what the driver's own claims demand; no speculative generality (the
     adoption-over-cleverness law);
   - the checker: instruction T-state table (base + conditional-taken variants) for
     the subset of Z80 ops the driver's timed regions use; path accounting through
     the timed loop (branch min/max or exact-per-path — let the DAC loop's shape
     decide); loud bail for any op/form outside the table;
   - the failing-first test plan (a doctored timed loop whose count drifts by one
     T-state MUST fire; the true loop verifies; non-vacuity per t24);
   - what rung 4 does NOT model (interrupt latency, bus contention with the 68k —
     name the exclusions explicitly; the banked-code hazard prose stays C3).
4. **The di/ei + shadow-set contract story**: how preserves/clobbers interact with
   exx/ex af,af' (shadow banks = separate units? a swap model like the sp slots? —
   design from the driver's real usage, not generality) and whether di/ei need any
   contract surface at all beyond C3 prose (they may not — argue it either way).

## 2. After endorsement

Step 1 = transcription + dual-shape windowed oracle + full contract set + the T-state
verification LIVE on the DAC loop + trust conversion of sfx's 2 driver externs +
t24 controls. Then the loop (byte-frozen, STOP-not-absorb), panel **A1 + B1 + C2 +
C3 + C1 ACTIVE** (C1's first Z80 activation — the T-state work IS its subject), dry
by panel, checkpoint (b). Kill row same-commit (the row-78 shape; its kill = the
seam sub-tranche, which after t40 holds ALL SIX sound twins).

## 3. Duties

Ledger per pass (the capability's asks/exclusions; any header lies vs the both-CPU
scoreboard); close packet with the acceptance delta vs t36/t37 + census §5.1 → ALL
Z80 CODE ROWS DONE; corrections list. After t40: the seam sub-tranche (input set
FINAL) + the generator + T1 close the campaign's port phase.
