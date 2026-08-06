# Partial-width preserves — `preserves(dN.w)` (2026-08-06)

Status: RULED (Fable, overnight overseer session). Closes the design half of
ledger row 2144 (the overseer ruling that machine code must never be widened to
work around a verifier's model gap — the model gains the width instead). Census
verified against sigil `247ae9ef` / aeon `4974bf3`: no width axis exists in the
grammar today (`preserves(d5.w)` parses as a bareword and dies semantically);
the full-`.l`-only round-trip credit lives at `preserves.rs:792–794` (drifted
from the row's `:470` — correct the row's citation in this parcel); the slot
machinery (`preserves.rs:125–151`) already tracks pushed byte-widths, so half
the model exists.

## 1 · The demand, honestly stated

Three tranches of witnesses (t34 `Player_Main` d7, t39 `TestPlayer` d7, the
five define-gates DEBUG arms: `Collected_ParkSlot` d2 +
`EntityWindow_{TrySpawnRing,RescanRings,ScanRingsRight,PopulateSectionRings}`
d5) share one shape: `move.w dK, -(sp)` … full-width interior write (`moveq`)
… `move.w (sp)+, dK`. The TRUE contract is: the register's LOW WORD is
preserved; the upper word leaves as the interior write's sign bits. Today these
procs are forced to declare the conservative full `clobbers(dK)` — a false
statement of what callers may rely on, standing only because the model cannot
say the true thing.

**Demand check the porter performs first:** verify at least one witness has a
real caller relying on the low word surviving (the object-dispatch loop's
counter register is the expected consumer — check RunObjects' d7 discipline
across the `Player_Main` call). If NO witness has a live consumer, report at
the checkpoint before building — the overseer re-adjudicates against the
zero-customer-adoption-is-ceremony bar. (Expectation from the tranche history:
the consumer is real; this is a verification step, not an open question.)

## 2 · Surface ruling

`preserves(dN.w)` — a register FACET claim, the exact analog of
`preserves(sr.mask)`: dotted facet, flows through the existing reg-list
grammar as a string, validated semantically.

- **Meaning**: on return, dN's low word equals its entry low word. The upper
  word is UNSPECIFIED — implicitly clobberable, licensed by the facet claim
  itself. dN must NOT also appear in `clobbers` (`[proc.preserves-overlap]`
  or the existing overlap diagnostic extends to the facet).
- **No `.uw` clobber spelling.** `clobbers` is permission, and permission for
  the upper word is implied by a `.w`-only preserve. The sr precedent names
  both facets because BOTH carry independent claims there; a data register's
  upper half has no independent claimant. Zero new vocabulary beyond the one
  form with witnesses.
- **`.b` is REFUSED** (no witness — demand-gated, the standing discipline).
  **`aN.w` is REFUSED** (address-register word writes sign-extend the full
  register; a word facet claim on aN is semantically treacherous and
  witness-free). **`.l` is REFUSED** (bare dN IS the full claim; one spelling
  per meaning). Each refusal gets its own message arm + negative probe.

## 3 · Model ruling

- **Round-trip credit**: at the `preserves.rs:792` credit site, a
  `Width::W` save/restore pair round-trips the WORD facet. A full (bare dN)
  claim still requires `.l` (unchanged). A `.w` claim is satisfied by a `.w`
  OR `.l` round-trip (stronger proves weaker — pin this with a probe, both
  polarities: `.l` round-trip proves `.w` claim; `.w` round-trip REFUSES a
  bare full claim, which is exactly today's behavior and must not regress).
- **Interior writes** between save and restore are licensed for the claimed
  facet (that is the point of the bracket); writes AFTER the restore refute,
  exactly as full-width does today.
- **Callee-credit propagation (the oracle/closure): conservative v1.** A
  `.w`-preserving callee is, to every full-width consumer (CalleePreserves,
  the transfer-tail credit, the sr oracles), a CLOBBER of dN — sound,
  monotone, and identical to today's verdicts. No consumer weakens. The facet
  is recorded in the contract surface for width-aware consumers to read the
  day one exists; building that consumer is NOT this parcel. Pin the
  conservative reading (a caller claiming preserves(dN) through a
  `.w`-preserving callee must still REFUSE).
- **Non-vacuity**: a probe where the `.w` claim is checked against a body
  with NO round-trip (must refuse — the credit cannot pass by measuring
  nothing).

## 4 · Adoption (same parcel, byte-neutral)

Flip the witnesses' conservative `clobbers(dK)` to `preserves(dK.w)`:
`Player_Main` d7, `TestPlayer` d7, `Collected_ParkSlot` d2, the four
`EntityWindow_*` d5 (aeon `entity_window.emp` :368/380 d2 pair, :859/861 d5
pair, per the census — re-verify lines in your tree). The define-gates arms
are DEBUG-gated: their verification runs under the `-D` harness shapes, not
the define-free frontend tests — state in the packet which gate proves each
site. Contract text only: byte bar stays identity ×7. If any witness's body
does NOT actually round-trip the word (the census lied), STOP and report —
do not widen the claim or the code.

## 5 · Bars

Standard byte-NEUTRAL parcel bars (boilerplate): identity ×7, full strict
with closing arithmetic, refreeze --check untouched, warn-tier ID sets
identical ×7, negative probes both polarities for every new message arm,
non-vacuity guards. Ledger: row 2144 CLOSED (design + model + adoption;
correct its drifted `preserves.rs:470`→`:792` citation); row 2138's
vocabulary note gains a pointer here. Lane-M (sr.mask plumbing) touches the
oracle-input wiring this parcel reads — the overseer sequences the merges;
rebase and re-prove per the queue.

## §6 — 2026-08-06 amendment (Fable, at the pw porter's §4 stop)

The porter's pre-build verification caught §4 contradicting §3: the census's
"five define-gates arms" was wrong. Only FOUR witnesses round-trip the word in
their own bodies: `Player_Main` d7 (player_common.emp 340/346), `TestPlayer`
d7 (test_player.emp 244/246), `Collected_ParkSlot` d2 (entity_window.emp
377/388, DEBUG arm), `EntityWindow_TrySpawnRing` d5 (entity_window.emp
999/1019, DEBUG arm). The other three EntityWindow procs
(`ScanRingsRight`/`PopulateSectionRings`/`RescanRings`) carry d5 only
TRANSITIVELY through `jbsr EntityWindow_TrySpawnRing` — under §3's
conservative v1 they must NOT flip (a `.w`-preserving callee is a full-width
clobber to callers), and they are hereby the REAL-CORPUS witnesses for §3's
conservative-refusal pin: their `clobbers(d0-d5,a0)` stays, and the pin
proves a caller claiming `preserves(d5)` through the `.w`-preserving callee
still refuses. §4's adoption list is amended to the four genuine witnesses.
The §1 demand check PASSED (RunObjects' `dbf d7` counter crosses the object
dispatch; Debug_AssertObjLoop asserts it) — the consumer is real.
