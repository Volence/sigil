# Tranche-7 checkpoint packet — collision.asm (2026-07-10)

The full loop ran to dry on `engine/objects/collision.asm` → `collision.emp`
+ the shared `aabb.emp` template module. Branches: sigil `port-tranche7`,
aeon `sigil-emp-tranche7` — both unmerged, awaiting your gate.

## The headline: your collision code, reviewed honestly (step 5)

**The design is sound.** Minimum-penetration-axis AABB resolution is a
real, standard technique (it's what most modern 2D engines do for
box-vs-box) — you independently arrived at the canonical approach. The
`abs(delta)*2 < combined` trick avoids per-object halving, the +1/−1
contact-keeping offsets are the textbook way to keep the standing test
alive frame-over-frame, and the X-then-Y register discipline with the
a0/a1 stash is tight. The on-object bit lifecycle comment shows you'd
already fought and won the hard half of this bug class on the player
side.

**One real bug found, live-verified, fixed: the standing-bit claim.**
`Touch_Solid`'s top-contact always set `ST_P1_STANDING` on the object —
regardless of which player landed. But `player_sensors.asm`'s ledge probe
scans for "MY standing bit" by player identity (P1→bit 3, P2→bit 4).
Verified in oracle on the pre-fix ROM: a player-2-slot entity landing on
a solid set bit 3 (wrong); post-fix, P1 claims $08 and P2 claims $10,
with the Y-snap byte-exact (y = top − combined_half + 1). Fix is the
4-instruction claim-by-identity block (`cmpa.l #Player_1, a2`), +10 B.

**Known remaining hazard (recommendation, not fixed — your call):
object-side bit staleness.** Nothing ever CLEARS a solid's standing bit
(verified live: teleport the player away, the bit persists). The ledge
probe's slot scan takes the FIRST object with the bit — so in 1P, walking
from solid A onto solid B leaves A's stale bit, and the probe can compute
the balance window from the WRONG object's geometry (your "teeters
forever" family). Recommended fix (cross-file, touches your player code,
so it's yours to gate): store the claimed object's SST address in a
player custom field at Touch_Solid time and have the ledge probe read it
directly — kills the scan loop (~2600 cycles worst-case on the teeter
path), the bit-identity selection, AND the staleness in one move. The
standing bits could then become pure render/AI hints or go entirely.

**Peepholes applied (lockstep, both twins):** `clr.w` ×4 (−8 B, cycle
tie); the aabb template's zero-copy alias skip — when `cdim` aliases
`adim` the lead `move.w d0,d0` was a 2-byte no-op, now comptime-skipped
in the .emp AND `if "adim"<>"cdim"`-guarded in the .inc (−4 B in
TouchResponse, and **rings.asm inherited −4 B for free** — its calls
alias the same way); invalid-type dispatch path no longer reloads the
already-fresh position cache (byte-neutral, dead-path cycles only).
Collision region $170 → **$16E**; EndOfRom unchanged both shapes.

**Recorded, deliberately NOT done:**
- *Player-dim hoist* (width/height re-read per object): saves ~16c per
  collidable test but needs a4/a5, widening the documented clobber
  contract for ~400c/frame at current object counts. All three current
  call sites are state-level jsr chains (verified — nothing live across
  the call), so it's SAFE today, but the contract cost isn't worth it
  yet. Revisit if TouchResponse shows up in Prof_TouchResponse peaks.
- *movem elision at dispatch* (~84c per overlap): the caller saves
  d6-d7/a2-a3 around handlers; tightening handler contracts instead
  (clobbers-declared, which .emp can VERIFY once S2-D6 lands) would drop
  it — but it trades robustness against future handler bugs. Flagged for
  the S2-D6 era, when the language can enforce what the movem defends.
- *Slot-scan structure* (~5300c/frame scanning 66 slots × 2 players):
  fine at this scale; a live-list would be structural work with no
  current demand. Numbers recorded so future profiling has a baseline.

## Loop summary

- **Step 0**: design note `notes/2026-07-10-tranche7-collision-design.md`
  (sigil master `ade45cd` + `972704a`). Gate re-ask RESOLVED + retired.
- **Step 1**: transcribe byte-exact on the FIRST harness run (both
  shapes, $308A..$31FA / $3344..$34B4) — zero edits needed. FOUR demanded
  features shipped: **F1** splice-in-displacement `{off}({reg})` (+
  indexed form), **F2** proc-local label call args (`.next_object` as a
  template argument; enclosing-owner mangling, end-of-body loudness),
  **F3** cross-module `pub comptime fn` import (param-only bodies; deep
  case fails loud, ledgered), **F4** `Code ++ Code` (the conditional-head
  template shape; per-fragment label spaces pinned loud, fn-scoped
  hygiene ledgered). The aabb.inc `utag` param DIED — hygiene makes
  unique-suffix plumbing obsolete (empyrean amendment candidate).
- **Step 2**: born-modern except two bare-symbol spellings
  (byte-identical); the bra.w handler table documented LOAD-BEARING
  (stride is ABI — never jbra; `dispatch` branch_table encoding
  ledgered).
- **Steps 3/4**: ledger +6 rows (branch-table encoding, local typed reg
  binding, F3 deep case, worktree hermeticity, --root pruning jot,
  Code++ semantics), reglist-ranges row gained its 3rd data point,
  distinct-regs template ask (probe-pinned). Back-prop: the harness
  gained `sigil_harness::test_support` (shared AS-truth equ blob +
  drift-guard filter — ~294 duplicated lines collapsed across 14 files;
  the twin-growth tax is now one list).
- **Step 5**: above. Re-pin sweep: collision $16E + the −6 engine-tail
  slide (rings inherited the alias skip), PROVENANCE re-baselined, new
  pins plain `82aac84d…` / debug `ff897d0b…`. Full surface re-derived
  from listings: collision_lookup $4C02/$5426, sound_api $5D8E/$724C,
  Sound_DrainSfxRing $5ED4/$7392, all pre-collision and post-$10000
  regions verified UNMOVED. **Final gates: 2034 passed / 0 failed
  (strict, both shapes), clippy clean, corpus all-identical** —
  independently re-run by the coordinator after the sweep.
- **Dry check**: the step-5 wave's own retrospect items (F4 semantics,
  AS string-`if` parity works, staleness recommendation) are all
  ledgered/packeted above; the sweep surfaced ONE process item (below),
  now recorded. Nothing else on the re-read. DRY.

## Kill list

Rows 13 (aabb.inc twin — dies at the rings port; zero-disp collapse
probe noted there) and 14 (collision constants block, now 10 values —
ST_P2_STANDING joined at step 5). Row 1's ensure count grew accordingly.

## Process notes

- **Fresh aeon worktrees build a DIFFERENT ROM silently** — the editor's
  gitignored `.bin` working data (games/sonic4/data/editor/) is a build
  input; without it the generators emit air-baseline collision (130KB
  drift, no error). Seeded this worktree by copying the directory; ledger
  row asks for a build.sh warning. Worth a line in aeon's README.
- My kill-list rows edit got swept into the F2 feature commit by the
  implementation agent (content correct, attribution untidy).
- Oracle test-state note: the scroll-test maps Up to direct player
  movement — fabrication scenarios must account for it.
- **Re-pin discipline find (sweep agent)**: my step-5 aeon commit slid
  the changed region's own gate orgs but MISSED the downstream
  collision_lookup/sound_api gate orgs (dead `else` branches in the
  reference build, so the reference ROMs never notice — only the mixed
  builds diverge). Rule for future waves: a region shrink re-derives
  EVERY `SIGIL_EMP_*` org between it and the next org boundary, not
  just its own. Fixed in aeon `b280c6c`.

## Asks

1. **Merge gate ×2**: sigil `port-tranche7` (F1-F4 + gates + back-prop +
   re-pin), aeon `sigil-emp-tranche7` (transcribe + step-5 wave, incl.
   the standing-bit FIX — gameplay-affecting, see headline).
2. **The staleness recommendation**: pointer-field redesign
   (collision + player_sensors together) — schedule as its own small
   piece when you're in player code, or hand it to the campaign as a
   pre-tranche-8 fix item.
3. **Empyrean amendment stack** grows: utag-death (hygiene), F1-F4
   surface docs, Code++ per-fragment-label semantics. Still your cadence.
4. Tranche 8 candidate (my ordering suggestion, not decided): rings.asm —
   it's the aabb.inc twin's kill condition, it just inherited a step-5
   change sight-unseen, and RingCollision is the natural second consumer
   proving the template generalizes. Alternative: animate.asm (AnimId/
   FrameId typed surface continues).
