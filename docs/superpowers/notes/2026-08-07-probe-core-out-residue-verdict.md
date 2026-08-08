# `probe_core` d1/d2 — the verdict, and the question that dissolved

Queue item 3 of the contract-verification queue: the `.cl_hanging` → `.full_back`
trace. Investigation only; nothing was fixed and no `crates/` file was touched.

Measured on sigil `3e0824b1` (branch `probe2`) against aeon `d8c93d7` (branch
`probe2`), in the lane worktrees, by perturbing the aeon corpus and reading the
frozen-baseline gate's **set diff** — never a count. Every perturbation was
reverted by string-replace and the reversion proven with `git diff` (empty) plus a
`PROBE2MARK` residue grep (none); the unperturbed gate was re-run green afterwards.

---

## HEADLINE

**The framing of item 3 rested on a category error, and correcting it dissolves
the question it asked.**

`.cl_hanging`'s `rts` and the partial-height `rts` are **not return paths of the
proc**. They are the returns of an INTERNAL subroutine, `.cell`, which is entered
only by `jbsr .cell`. `bsr`/`jbsr` deliberately contributes **only a fall-through
edge** to the shared CFG, so `.cell`, `.cl_hanging` and `.cl_air` are
**unreachable** in the graph the out verifier walks. They contribute neither
productions nor return paths, and no oracle trace of them can bear on the residue.

From that one fact both registers fall out of the same table:

- **`d2` — verdict (c), CONTRACT-ONLY**, with a refinement that matters: the
  declaration's defect is **width** (`out(d2)` claims 32 bits; the proc produces
  8), while the residue row's cause is a **verifier blind spot** (the producing
  writes are invisible). The two are independent, and *neither one alone closes
  the row*. No caller reads a stale `d2`: `d2.b` is freshly written on **every**
  execution of `.cell`, and `.cell` runs on every path.
- **`d1` — the open sub-question is ANSWERED. Same cause as `d2`.** But the
  brief was right to forbid assuming it, because the census's stated cause *for
  `d2`* was itself wrong — so "same as `d2`" would have been the wrong answer to
  inherit. The path that reaches a return without writing `d1` is not exotic: it
  is the **primary-surface path**, the dominant one.

**Two inherited claims are corrected** (below), and **one live engine comment is
stale** in a way that would mislead the next author.

---

## 1. The structural finding

`crates/sigil-frontend-emp/src/flag_check.rs:333-337`, in `Cfg::edges`:

> `bsr` is spelled like one (three letters, leading `b`) and is NOT one: it CALLS
> and comes back, so its only successor is the fall-through. Giving it the
> branch's taken edge would splice the callee's body into this proc's flow at the
> caller's state — for a LOCAL `bsr .helper` that analyzes the helper with the
> caller's stack still on it.

This is correct and deliberate. Its consequence for `out_verify` is that a block
entered only by a local `bsr` is **unreachable**: `verify_out`'s worklist seeds at
`entry_idx` and only follows edges, so it never visits `.cell`.

The second half is `out_verify::direct_target`, which requires
`!name.contains('$')` — a `$`-mangled local label is not a callee, so the
`callee_uncond_out` credit does not apply either. So a local subroutine is
invisible from **both** directions: no credit for what it produces, and no
obligation for where it returns.

**The polarity is fail-safe.** The analysis over-fires; it cannot bless a false
`out()` this way. This is a **precision gap, not a soundness defect** — the
"verifier-model gap" bucket of `contract_baseline.rs`'s taxonomy, not the
"genuinely-loose contract" bucket. (Corroboration that it does not always bite:
`ZX0_Decompress` uses the same idiom four times and still verifies `out(a0, a1)`,
because its productions sit on the reachable path.)

`preserves` is **not** affected: its `call_target` does not filter mangled
locals, the name is absent from the closure's `effective` map, and
`callee_clobbers` returns `true` for an absent callee — so a local `bsr` reads as
clobber-all, which is conservative.

---

## 2. THE RETURN-PATH × REGISTER TABLE (the spine)

Body: `aeon/games/sonic4/player/player_sensors.emp`, `probe_core`, lines
**103-215** (re-resolved this session). Line numbers below are current.

`probe_core` is stamped four times (lines 221-232) as `Collision_Probe{Down,Up,
Right,Left}`, each `out(d0, d1, d2)` — which is why all four fire identically.

### Reachable exits of the PROC: exactly one

The `rts` at **line 122** (`.done`). Every other `rts` in the body (197, 207, 213)
belongs to `.cell` and returns to a `jbsr` site.

### The paths that reach it

| # | Path | Reaches `.done` via | `d0` | `d1` | `d2` |
|---|---|---|---|---|---|
| **P1** | primary cell holds a surface (`h` = 1..15 partial) | fall-through into `.done` @119 | `moveq #16,d0` @117 → **full** | **NOT WRITTEN** | **NOT WRITTEN** |
| **P2** | `.nothing` — primary empty *and* forward empty | `jbra .done` @141 | `moveq #32,d0` @138 → **full** | `moveq #0,d1` @139 → **full** | `moveq #0,d2` @140 → **full** |
| **P3** | `.empty_fwd` — forward cell holds a surface | `jbra .done` @135 | `moveq #32,d0` @133 → **full** | **NOT WRITTEN** | **NOT WRITTEN** |
| **P4** | `.full_back`, back cell SUPPLIED (`bne .back_supplied` @154 taken) | `jbra .done` @162 | `add.w`/`neg.w` @160-161 → **.w only** | **NOT WRITTEN** | **NOT WRITTEN** |
| **P5** | `.full_back`, back cell empty (fall-through @155-156) | `jbra .done` @162 | `add.w`/`neg.w` @160-161 → **.w only** | `move.w 2(sp),d1` @156 → **.w only** | `move.w (sp),d2` @155 → **.w only** |

Read off the table:

- **`d1`/`d2` fire because P1, P3 and P4 never write them at all** — which is why
  they survived the census's width-off probe. P5 writes both, but only `.w`.
- **`d0` fires only on P4/P5**, where its only writes are `.w` — a pure width gap,
  exactly as the census reported. (`d0` is the `outw` lane's; recorded here as
  context only, and nothing in this lane touched it.)
- Helper macros are accounted for: `probe_sub`/`sub_flip` (lines 54-69) write only
  their `pdst`, which is `d3` at all four call sites.

### The MACHINE truth, which the table above deliberately does not show

Every path executes `jbsr .cell` at least once (line 109 is unconditional; P3 and
P4/P5 call it twice). Inside `.cell`:

- **`d2` is written on every single execution** — `move.b d0, d2` @175, before any
  branch, and `.cl_air` re-writes it `moveq #0,d2` @212.
- **`d1` is written on every return from `.cell`** — `move.b (a1,d3.w), d1` @186 on
  the solid path (and `.cl_hanging` @199 is reached from @196, i.e. *after* @186),
  or `moveq #0,d1` @211 on the air path.

So **`d1.b` and `d2.b` are never stale.** The documented contract (`d1.b` angle,
`d2.b` attr — lines 87-88) holds on the machine. The residue is an artifact.

---

## 3. EVIDENCE — six probes, each read as a set diff

Gate: `contract_baselines_hold_for_every_shipped_shape`, all seven shipped shapes,
`SIGIL_STRICT_GATE=1`, `AEON_DIR` = the lane's aeon worktree. Baseline run green
first, with 54 crates compiled in-worktree (provenance checked, not assumed).

| probe | perturbation | predicted | **measured set diff** |
|---|---|---|---|
| **E0** | `moveq #0,d1` + `moveq #0,d2` inserted at the top of `.cell` | no change if `.cell` is unreachable | **EMPTY** — nothing moved |
| **E1** | the same two writes inserted at `.done` | all 8 `d1`/`d2` rows go | **exactly 8 GONE**, 0 NEW; the 4 `d0` rows stayed |
| **A** | cover P3 + P4/P5, leave **P1** uncovered | rows persist | **EMPTY** (rows persist) → P1 fails |
| **B** | cover P1 + P4/P5, leave **P3** uncovered | rows persist | **EMPTY** (rows persist) → P3 fails |
| **C** | cover P1 + P3 + P5-arm only, leave **P4** uncovered | rows persist | **EMPTY** (rows persist) → P4 fails |
| **E** | cover P1 + P3 + P4/P5 (P2 self-covered) | all 8 go — the exhaustiveness control | **exactly 8 GONE**, 0 NEW |

E0 is the load-bearing one: **two full-width `moveq`s inside `.cell` changed
nothing at all**, which is the direct proof of the blind spot. E1 localises every
failure to the `.done` `rts`. A/B/C attribute one failing path each. E proves the
path set `{P1,P2,P3,P4,P5}` is exhaustive — with those three sites covered, no
obligation remains, so there is no sixth path hiding.

Config C required diverting the `bne .back_supplied` taken edge through a scratch
label, because P4 and P5 merge immediately at `.back_supplied` and cannot otherwise
be separated. P5's own status (`.w`-only writes → fails at full width, would be
credited under width-off) is a static reading of lines 155-156, corroborated by C
persisting.

---

## 4. THE `d2` VERDICT — (c) CONTRACT-ONLY, and why not (a) or (b)

**Not (a), a real bug.** For `d2` to be stale a caller would have to read bits
above 7, since bits 0-7 are freshly written on every path (§2). The caller sweep
(§5) finds no such read anywhere in the tree.

**Not (b), benign-by-downstream-filter.** There is no filter to name. `d2` is not
"ignored on exactly those paths" — it is *correct* on those paths. Verdict (b)
would misdescribe the situation and would leave a reader hunting for a guard that
does not exist.

**(c) CONTRACT-ONLY.** `out(d2)` means all 32 bits; the proc produces 8. The
declaration overclaims **width**, and the honest narrowing is `out(d2: u8)` under
item 2's ruled mechanism. The same holds for `d1`.

**The refinement that must not be lost:** narrowing the type **does not close the
row**. On P1/P3/P4 there is no write to `d1`/`d2` *at any width* in the reachable
graph. Both mechanisms are required — see §7.

---

## 5. CALLER SWEEP

Seven machine call sites to the four probes, **all in `player_sensors.emp`**: two
indirect (`jsr (a2)`, lines 243/250, in `Player_SensorPair`), four direct (442,
446, 450, 454, in `Player_SensorWallDir`), one direct (539, `Player_AtLedgeEdge`).
The only `lea …Probe*(pc), a2` loads are 344/353/363/371, all funnelling into the
single `jbsr Player_SensorPair` @373. Transitive consumers were swept too
(`Player_SensorFloor`/`Ceiling`/`Surface`/`WallAt`/`WallDir` forward `d1`/`d2`
verbatim): 14 further call sites across `player_air.emp`, `player_ground.emp`,
`player_spindash.emp`, `player_common.emp`, `test_player.emp`.

**Result: nothing anywhere reads `d1` or `d2` above bit 7 in a way that reaches a
consumer.**

- `d1` is read at `.b` only: `btst #0,d1` (bit 0), `move.b d1,d3`,
  `move.b d1,d4`, `move.b d1,angle(a0)`.
- `d2` has **exactly one consumer in the whole tree**: `test_player.emp:252`
  `tst.b d2` — byte, and unguarded (it precedes the `d0` tests). Every shipping
  player state ignores `d2` entirely.
- The **one** wider-than-byte touch is `Player_SensorPair` lines 246-247
  (`move.w d1,-(sp)` / `move.w d2,-(sp)`) with the matching pops at 251-252. It is
  a pure round-trip: the words are popped into `d3`/`d4` and only the low byte is
  ever extracted (`move.b d4,d1` @257, `move.b d3,d2` @258). The 8 undefined high
  bits are pushed and popped but never consumed.

That round-trip is worth a standing note rather than a fix: it is correct today
**only because the extraction is `move.b`**. Narrowing the pushes to `.b` without
narrowing the pops, or widening the extraction to `.w`, turns it into a real bug.
Ledgered.

---

## 6. CORRECTIONS TO INHERITED CLAIMS

**(i) The census's `d2` premise is wrong on both halves.** It states: *"`d2` is
written in exactly ONE place — `.cl_air`'s `moveq #0, d2`"*. Measured: `d2` is
written in **four** places — @175 `move.b d0, d2`, @140 `.nothing`, @155
`.full_back`, @212 `.cl_air`. And the one place it names, `.cl_air`, is
*unreachable* to the analysis, so it is the one write that could not have mattered
either way. The same sentence's companion claim — that `.cl_hanging` and "the
partial-height `rts`" are return paths that fail to write `d2` — is the category
error of §1: neither is a return path of the proc.

This is a **fifth citation-decay-class instance**, with a new flavour: the numbers
were fine and the *reading* was wrong. Substance-checking the claim (grep for every
write of `d2`) is what caught it, exactly as the pickup rule prescribes.

**(ii) A live engine comment is stale and teaches the wrong thing.**
`aeon/engine/objects/rings.emp:156-160` says `out(a4)` *"false-positives
`[proc.out-unwritten]` because a4 is written only via `(a4)+`, which the heuristic
misses"*. That gap is **closed** — `lower/proc.rs:1162-1172` (effect 2) reports
auto-inc/dec bases for any mnemonic and any operand position, and
`out_verify::produced_regs` credits every address-register write at full width.

Proven, not argued: inserting **only** `tst.b (a4)+` / `tst.b -(a4)` (a net-zero
pair of auto-inc/dec-only instructions, no plain writes) at the top of
`InsertSpriteMasks` removed **exactly one row** — `InsertSpriteMasks :: out(a4)` —
and nothing else. So `(a4)+` alone does credit `a4`, and the comment's stated
reason for the firing is not the real one. Per the house comment rule (present-
tense contract fact only) it should be rewritten or deleted; it currently encodes
a retired limitation as a live one.

---

## 7. THE SECOND TARGET — `DrawRings` / `InsertSpriteMasks` (4 rows)

Both examined; not rushed. `DrawRings` = `engine/objects/rings.emp:164-247`,
`InsertSpriteMasks` = `engine/objects/sprites.emp:749-776`. Both are single-exit
(one `rts`, no tail transfer, cannot run off the end) and neither uses the local
`bsr` idiom, so §1 does not apply to them.

| proc | exit paths that write **neither** `a4` nor `d5` |
|---|---|
| `DrawRings` | **P1** zero rings (`beq .done` @201) · **P2** cap already reached at loop entry (`bhs .done` @206) · **P4** every ring culled off-screen (both `bhi .skip_ring`, `dbf` falls through @243) |
| `InsertSpriteMasks` | **M1** non-positive height (`ble .masks_done` @752) · **M2** cap already reached (`bge .masks_done` @756) |

On the paths that *do* emit, `a4` advances via `(a4)+` (credited — §6ii) and `d5`
is written **`addq.b #1, d5`** — byte only, against a `u16` declaration.

**Verdict: (c) CONTRACT-ONLY, but a different contract defect from the probes.**
`a4` and `d5` here are not outputs at all — they are **threaded in-outs**: the
caller `Render_Sprites` seeds them once (`lea Sprite_Table_Buffer, a4` /
`moveq #0, d5`, `sprites.emp:221-222`) and each callee conditionally advances
them. `out(rN)` cannot express that, and `preserves(rN)` cannot either, because
neither is true on all paths. Both callers read them unconditionally afterwards
(`tst.w d5` @453, `move.b #0,-5(a4)` @455, `move.w d5,d0` @484,
`move.w d5,Sprites_Rendered` @488), and that is correct precisely because the
un-written case is an identity.

**One genuine latent fragility, escalated rather than papered.** The item-2
adoption bar says a caller reading wider than the adopted type is a real finding.
Here the callee writes `d5` at `.b` and the caller reads it at `.w` in three
places (plus `assert.w d1, eq, d5` @475 under DEBUG). It is correct **only**
because the caller's `moveq #0, d5` clears the full long and `MAX_VDP_SPRITES =
80` (`engine/system/constants.emp:240`) keeps the count under 256. Raise that
constant above 255 and `addq.b` wraps while every word read keeps believing the
high byte. **`out(d5: u8)` would therefore be the wrong adoption** — it would
declare a byte contract to callers who want a count. Ledgered with that kill
condition.

---

## 8. RECOMMENDED FIX AND EXPECTED RESIDUE DELTA

Three independent pieces. **The sequencing finding is the important part: for the
8 probe rows, each of the first two closes ZERO rows on its own.**

**(1) Width types on `d1`/`d2` — `out(d1: u8, d2: u8)`** on all four
`Collision_Probe*` procs. Note this must also update the function-pointer type
`SensorProbe` (`player_sensors.emp:49`), which carries its own `out(d0, d1, d2)`.
Alone: **0 rows** (P1/P3/P4 have no write at any width).

**(2) Production credit for a local `bsr`/`jbsr`.** Model `jbsr .L` as a call
whose credit is the MUST-intersection of productions over the paths from `.L` to
its own `rts`es. Alone: **0 rows** — `.cell`'s full-width MUST-production is the
empty set (its three exits produce `d0` full-width on only two of them, and
`d1`/`d2` at `.b`).

**(1) and (2) together: all 8 rows close.** With `u8` as the target width,
`.cell`'s MUST-production covers `d1` and `d2` on all three of its exits (`.cl_air`
@211-212 by `moveq`; the @197 exit via @175 and @186; `.cl_hanging` @207 likewise,
since @199 is reached from @196, after @186). Every `.done`-reaching path executes
`jbsr .cell` at least once, so every path gets the credit.

Two design points a spec must settle before (2) is built, flagged not resolved:

- A block that is **both** fallen into and `bsr`'d has `rts`es that *are* genuine
  proc return paths. `.cell` is not such a block (its only entries are the three
  `jbsr`s), but the model must not assume that.
- Recursion / mutual local `bsr` needs a termination story; a depth cap that bails
  to "credit nothing" keeps the fail-safe polarity.

Corpus yield of (2) is **exactly these 8 rows today** — a full census of the idiom
(8 files, 31 sites) shows no other residue proc uses it.

**(3) An `inout(rN)` facet** for the threaded cursor/counter shape: "on every
required return path, rN is either PRODUCED on this pass or holds its ENTRY
value". Both halves already exist and can be composed rather than written —
`out_verify`'s production dataflow and `preserves::verify_preserved_on` scoped to
the exits, which is exactly how `check_cond_out_survives` is already built.
Closes **4 rows** (`DrawRings`/`InsertSpriteMasks` × `a4`/`d5`) as
`inout(a4), inout(d5: u8)`. See §7's escalation before adopting the `u8` on `d5`.

### Projected burn-down (30 rows, re-derived from the census this session)

| item | rows | running total |
|---|---|---|
| baseline today | — | **30** |
| item 2, width types (the `outw` lane's 15) | −15 | 15 |
| **this lane: (1)+(2) together** | **−8** | **7** |
| **this lane: (3) `inout`** | **−4** | **3** |
| `S4LZ_Decompress::a1` root + its 2 dependants | −3 | **0** |

15 + 8 + 3 + 4 = 30, so the four causes account for the residue exactly.

---

## 9. WHAT I DID NOT DO, AND WHY

- **No oracle trace was run, deliberately.** Item 3 asked for one, but the question
  turned out not to be a runtime question: `.cl_hanging` is not an exit, and the
  staleness hypothesis is refuted statically by "`d2.b` is written unconditionally
  at line 175 on every `.cell` call, and every path calls `.cell`" plus a caller
  sweep showing no read above bit 7. An A/B run would have measured agreement
  between two ROMs that differ in no bytes. If a future reader wants the runtime
  confirmation anyway, the honest form is a watchpoint on `d2`'s high byte at the
  seven call sites — not a `.cl_hanging` → `.full_back` trace, which traces a path
  that returns to `jbsr`, not to a caller.
- **`d0` was not touched** on any proc — the `outw` lane owns all `d0` width rows.
  §2 records `d0`'s per-path status only as context, and the finding that `d0`'s
  single failing path is `.full_back` (P4/P5) may save that lane a derivation.
- **No `crates/` file was edited**, and `contract_baseline.rs` was not opened for
  writing. All measurement went through the existing gate's set diff.
- **The `inout` facet is a proposal, not a ruling.** It needs Fable's design pass;
  in particular whether `inout` should imply anything about `clobbers` membership,
  the way `out(rN if cc)` does.
