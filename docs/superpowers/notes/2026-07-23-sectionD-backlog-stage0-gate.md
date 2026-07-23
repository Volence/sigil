# §D backlog batch — stage-0 census + design-gate note (2026-07-23)

**Arc:** the pre-t18 roadmap §D backlog, run concurrently with a Volence+overseer
item-13 construct-walk (this arc does NOT touch `types.emp` or implement newtypes).
**Masters at open:** aeon `bd9ddf2` / sigil `941ec... (941c5f4)`. **Baseline pin (current
canonical, unchanged since the phase-2.5 merge — G5 was byte-neutral):**
plain `406c773b`/`421122` (EndOfRom `0x5DB60`) · debug `5752c2e3`/`429107`
(EndOfRom `0x5F65A`).

This is the ONE design-gate note the arc discipline requires before cutting the
byte-changing / lint items. Items 2 and 5 were allowed to proceed to draft alongside;
2 evaporated, 5's options are drafted below.

---

## Stage-0 census — verdicts (5 items)

| Item | Verdict | One line |
|---|---|---|
| **1. await_slot watchdog** | **REAL — and stage-0 surfaced a REAL BUG that reframes it** | the H-1 repost-gate spin never iterates; item becomes a PLAIN byte-changing correctness fix + the debug watchdog on top. **GATE NEEDED.** |
| **2. @scaffolding attribute** | **EVAPORATED** | shipped in G1 (`parser.rs:254-266` + mandatory-reason enforce + 3 tests); the one ratified keep `Plane_Buffer_Reset` is already annotated (`plane_buffer.emp:62`). No work. |
| **3. D11 name-linkage** | **EVAPORATED** | its guard mechanism (`ensure(extern("X")==X)`) is shipped + mandatory port-loop convention; the auto-detector's target population (seam mirrors) is being drained to zero by the kill-list. Recommend RETIRE the roadmap line. |
| **4. s4lint absorption #1 (Z80-bus contract)** | **REAL — the arc's biggest build** | per-proc `flag_check::Cfg` forward-MUST dataflow over a 2-state bus lattice, cloning `type_slice.rs`. Byte-neutral (lint-only). Two scope questions for the gate. |
| **5. Optional-param (AnimateSprite d3)** | **REAL — design-only** | d3 is a data-dependent implicit input absent from the signature; options below, no implementation this arc. |

**Two of five evaporate** — expected and welcome (the stage-0 discipline working as designed).

---

## ITEM 1 — the await_slot finding (HEADLINE; this is the gate's main business)

### The bug

`Sound_PlayMusic`'s H-1 repost gate is supposed to spin until `MUSIC_SLOT == 0` before
posting a new param block, so a rapid second `PlayMusic` can't tear the block the Z80 is
still consuming. The shipped spin (`engine/sound/sound_api.emp:187-191`, `.asm:91-95`
identical):

```
.await_slot:
    stop_z80                 ; ...ends on a btst (Z reflects bus-grant, irrelevant here)
    tst.b   MUSIC_SLOT       ; Z = (slot == 0)      <- the value we care about
    start_z80                ; move.w #$0000, Z80_BUS_REQUEST   <- SETS Z=1 UNCONDITIONALLY
    bne     .await_slot      ; branches on Z=0 ... but start_z80 just forced Z=1
```

`startZ80` (`engine/macros.asm:233-235`) is a bare `move.w #$0000, (Z80_BUS_REQUEST).l`.
On the 68000, `MOVE` always sets N/Z from the moved data (V/C cleared); moving `$0000`
⇒ `Z=1`. So the `bne` reads `startZ80`'s forced `Z=1`, never the `tst.b` result. **The
loop runs exactly once and always falls through** — the gate does not gate. A rapid repost
still tears the param block / loses the trigger (the exact H-1 race the gate was written
to close). It is rare (needs a second `PlayMusic` before the Z80 consumes the first),
which is why the boot check missed it.

### Why this is certain (evidence, not theory)

- **ISA:** `move.w #imm, ea` sets `Z` from the immediate; `#$0000` ⇒ `Z=1`. Unambiguous.
- **Both twins share it** → the byte gate is structurally blind (MigrateMasks class:
  identical on both sides, so "twins match" proves nothing about correctness).
- **Corroboration in the same file:** `Sound_DrainSfxRing` (`.asm:243-253`) does
  `stopZ80 / tst.b SND_REQ_SFX / <branch on the tst HERE> / … / .dr_done: startZ80` — it
  branches while the bus is still held, then a single `startZ80` at the merge. That is the
  correct shape. `await_slot` is the lone site that branches *after* the release.
- **Design-doc intent** (`aeon/docs/superpowers/2026-07-16-sound-repost-gate-design.md`
  §"The fix", §"The spin bound") is explicit that it must actually wait on a repost.
- Live single-step deferred only because the currently-loaded oracle ROM/symbols are stale
  (`0x6272` decodes to `moveq #1,d0/rts`, not `Sound_PlayMusic`). Live confirmation rides
  the fresh build the fix produces — proposed as implementation step 1.

### The reframed scope (this is what needs a ruling)

**1a — PLAIN correctness fix (byte-CHANGING in the plain shape).** The read must stay under
the bus hold (reliable read), so the fix captures the slot value under `stop_z80`, releases,
then tests the captured value:

```
.await_slot:
    stop_z80
    move.b  MUSIC_SLOT, d1     ; capture under bus hold (was: tst.b MUSIC_SLOT — same 6 bytes)
    start_z80
    tst.b   d1                 ; test the captured value (flags now clean)   (+2 bytes)
    bne     .await_slot
```
`d1` is dead at the spin (input is `d0`=song id; d1 first used at `.emp:209` `z80_bank(d1,a1)`;
proc declares `clobbers(d0-d4)`), so it is a safe scratch. Net **+2 bytes, plain**.
Alternative (branch-before-release, DrainSfxRing-style) is larger; capture-then-test is the
minimal, known-good form. **This moves the PLAIN canonical → plain re-baseline required.**

**1b — the DEBUG watchdog (byte-CHANGING in the debug shape only), now on a loop that
actually loops.** A `DEBUG==1`-gated bounded spin counter (`d4`, also dead at the spin) +
`raise_error` on overrun — the original item-1 ask, using the in-file idioms
(`if DEBUG == 1 { … raise_error "…" }`, cf. the PlaySFX ring-full assert at `.emp:294`).
Self-gates to zero bytes in plain. Exact spelling settled at implementation; shape:

```
    if DEBUG == 1 { move.l #AWAIT_SPIN_LIMIT, d4 }
.await_slot:
    stop_z80
    move.b  MUSIC_SLOT, d1
    start_z80
    tst.b   d1
    beq     .await_ok
    if DEBUG == 1 { subq.l #1, d4  beq .await_wedged }
    bra     .await_slot
    if DEBUG == 1 { .await_wedged: raise_error "Sound_PlayMusic: MUSIC_SLOT never cleared (Z80 wedged)" }
.await_ok:
```

### Discipline (both shapes re-baseline)

- **Plain** moves (1a) → new plain crc32/size, `pins.rs` via `repin`, `engine.inc` gate
  resume orgs (sound_api sits in the engine block; the +2/+N absorbs at the `org $10000`
  sound boundary, so EndOfRom likely unchanged — VERIFY from listings), `mixed_dac_rom.rs` /
  `repin_pins.rs` seams per the 5-site doctrine, PROVENANCE new plain row.
- **Debug** moves (1b) → new debug crc32/size, debug-side pins/orgs, PROVENANCE new debug row.
- Full paired strict failures-first, per-commit ripple, `.asm` twin in lockstep, oracle
  live-confirm of the now-working spin (force `MUSIC_SLOT` nonzero, single-step the `bne`).
- Suggested commit split: **c1** = 1a plain fix (its own bisectable byte-changing commit,
  both twins) → **c2** = 1b debug watchdog. Keeps the correctness fix isolable from the rail.

### Pending-mechanism marker (net opportunity — relates to item 4)

This bug is a **flag def→clobber→use**: `tst.b` defines Z, `startZ80`'s move clobbers it,
`bne` consumes the clobber. A `[branch.flag-clobbered-before-use]` check over the same
`flag_check::Cfg` cc-lattice that item 4 and G2's `[call.flag-result-unused]` already use
would catch this class structurally. Logging as a gap-ledger candidate; a natural rider on
the item-4 machinery (both are cc/bus state over the same CFG). Not this arc unless the gate
wants it pulled in.

---

## ITEM 4 — s4lint absorption #1: Z80-bus machine-state contract (biggest build)

**Shape (confirmed against `s4lint.py` + the Cfg substrate):** a per-proc forward-MUST
dataflow over `flag_check::Cfg` (`crates/sigil-frontend-emp/src/flag_check.rs:147`), a
2-state bus-ownership lattice `{Stopped, Running}` with `Running` = unsafe top, join = meet
(disagreeing edges ⇒ `Running`), cloning `type_slice.rs` (G5's slot-type slice) — same
`transfer`/`join`/`type_state_in` skeleton, wired into `analyze_corpus`
(`corpus_contracts.rs:121`). Byte-neutral: runs in corpus tests, never the ROM build path.

The three s4lint codes it subsumes (`aeon/tools/s4lint.py`):
- **E006** (`:1090`) — VDP/DMA-port *write* whose IN-state is not provably `Stopped`.
- **E007** (`:513`) — a return (`rts`/`rte`) reached with IN-state `Stopped` (stopped and
  never restarted).
- **E011** (`:1061`) — `stopZ80` with IN-state already `Stopped` (double-stop).

The upgrade over s4lint is real: s4lint tracks a **flat scalar** `z80_state` with no CFG
joins (`:418`, `:494`); the net gets true path-sensitivity from the meet — E006 fires
exactly when a write isn't dominated by a stop on ALL paths.

**GATE QUESTION 4a — the E008 scope fence.** s4lint splits the "unpaired" family: the mirror
of E007 — `startZ80` while already `Running` (start-with-no-preceding-stop) — is emitted as
**E008** (`:1071`), not E007. The roadmap census lists item #1 as "E006/E007/E011". Does the
net's first cut ALSO absorb E008 (trivial: a `startZ80` transfer whose IN-state is `Running`
fires), or leave it to §5 preserves? Recommend **include E008** — it's one more firing
condition on the same lattice, and the pair (double-stop / double-start) is the whole
balance invariant. Cheap completeness.

**GATE QUESTION 4b — macro-expansion recognition (the one genuinely new problem).** s4lint
keys off macro *names* (`stopZ80`/`startZ80`/VDP-port symbols). By the time the corpus lint
sees an evaluated `CodeBuf`, `stop_z80()`/`start_z80()` are already expanded to their
instruction bodies (`move.w #$0100/#$0000, Z80_BUS_REQUEST` + the btst spin) and VDP writes
are raw `move`s to `VDP_CTRL`/`VDP_DATA`/`$C00000`/`$C00004`. So the net must recognize the
**expanded instructions** (a move to the `Z80_BUS_REQUEST` address = a bus toggle; a move
whose destination is a VDP port = a fenced access), not macro tokens. This is a resolvable
modeling question (match on resolved operand addresses/symbols), but it is the load-bearing
implementation risk and I want the gate to see it before I cut. Register-indirect VDP
destinations (`(a4)`) stay unflaggable (s4lint punts them too, `:1133`) — same soundness
bailout as the type slice's unverifiable paths.

**Evaporation check:** nothing in Sigil catches this today (all `z80` hits in the Rust tree
are the Z80 *codegen backend*, unrelated to 68k bus ownership). Genuinely unabsorbed.

---

## ITEM 3 — D11 verdict: EVAPORATED (recommend retire the line)

"D11 name-linkage" is a roadmap-only gloss (`pre-t18-roadmap.md:188`); the review's real
title (`aeon/docs/reviews/2026-07-16-emp-port-optimization-review.md:653-659`) is **"Local-
mirror ban + drift-guard completeness"** — flag a file-local const equal in value to a
reachable `extern`/shared twin symbol that lacks an `ensure(extern(...))` guard; prefer `use`
imports over mirrors. The v2 spec explicitly parks D8–D11 (`spec:288-289`); no G-feature
absorbed it (G1–G5 shipped proc/register contracts, not const-value mirror detection).

It has evaporated by two paths, not by being built:
1. **The guard mechanism shipped and is mandatory.** `ensure(extern("X")==X)` lowers to a
   link-time `LinkAssert`, is a standing port-loop step-2 rule, and has adopters everywhere
   (collision_lookup's 8, sound_api's 7, etc.). The deliverable D11 wrapped is done by hand
   each port.
2. **The auto-detector's target population is a draining seam artifact.** D11's own examples
   are seam mirrors "whose byte-gate protection dies with the .asm twin" — they exist only
   because `.emp` duplicates a value still owned by an `.asm` twin, and the seam is nearly
   gone (census: "largely self-contained now", 3 cross-seam calls). Every remaining mirror is
   tracked with a kill condition in the kill-list, headed to zero as the last `.asm` port
   retires. The port-loop + kill-list is a manual, tracked drain of exactly D11's population.

**Thin residual (real, low-value, do NOT open a row):** intra-`.emp` duplicate consts (the
review's `act_descriptor` SECTION_SIZE_SHIFT / ENEMY_PATROL_SPEED cases) don't evaporate with
the seam. A completeness lint over the resolved symbol table (`crates/sigil-link` +
`guard_assert_count`) would still have ~2–3 instances of work, against a shared home
(`use engine.constants`) that already exists and a mandatory-guard convention that already
forces the fix at authoring. **Recommendation:** retire the "D11 name-linkage" roadmap line;
fold its one live obligation where it already lives (port-loop step 2 + kill-list); log the
intra-`.emp` completeness check as a low-priority link/s4lint warning idea, not a
diagnostics-tier item.

---

## ITEM 2 — @scaffolding verdict: EVAPORATED (already shipped + already applied)

Fully implemented in G1: `@scaffolding` is parsed and its mandatory reason string enforced
(`crates/sigil-frontend-emp/src/parser.rs:254-266`, `[scaffolding.reason-required]`), the
inert-metadata semantics documented (`ast.rs:727-731`), three tests
(`contract_grammar.rs:285/298/336` — attaches / requires-reason / byte-neutral). The single
ratified zero-caller keep it targets, `Plane_Buffer_Reset`, already carries the real
attribute (`aeon/engine/level/plane_buffer.emp:62`). No other decl in the aeon corpus is a
pending scaffolding candidate (Phase-2.5/D7 census's others are delete-targets or parks;
caller-grep confirms `Plane_Buffer_Reset` is the lone zero-caller keep). **Nothing to build.**

---

## ITEM 5 — optional-param design options (design-only; for the gate/Volence)

**The case:** `AnimateSprite (a0: *Sst)` (`animate.emp:81`) also reads `d3` — but only on the
`DUR_DYNAMIC` branch of `reload_anim_timer` (`animate.emp:66`), i.e. its definedness is
**data-dependent on the script's duration byte**, invisible to the caller. Two real callers
split: `test_particle.emp:53` does NOT set d3; `player_common.asm:260` DOES. This is the exact
`[call.input-undefined]` (D1b) evidence bug the v2 spec §6 cites ("shipped in two templates").
No `@optional` grammar exists. The tension: D1b exists to catch missing inputs, but d3's
requirement is statically unknowable at the call site — so any pure suppression *accepts* the
bug class it was meant to catch.

- **Option A — `?`-optional param `(a0: *Sst, d3?: u8)`.** Low ceremony; D1b simply doesn't
  require a reaching def for a `?`-param. **Honest cost:** it disables the check for d3 —
  documents the optionality in the signature (better than a header comment) but provides no
  enforcement. Suitable *because* the requirement is genuinely unmodelable at the caller.
- **Option B — predicated param `(d3: u8 if <cc>)`** (mirror the shipped `out(a1 if cc)`).
  **Rejected:** the predicate here is data-dependent (script duration == DUR_DYNAMIC), not a
  call-site cc — there is no expressible condition. Doesn't fit; noted so the gate sees it
  was considered.
- **Option C — status quo (no signature slot, header comment only).** What ships today; d3
  never enters D1b. Zero enforcement, zero visibility. The baseline to beat.
- **Option D — push the obligation into the DATA.** Type the animation-script duration byte
  so a `DUR_DYNAMIC` script statically carries a "d3 required" obligation, checked where the
  script is *installed*, not where AnimateSprite is called. This is the only option that
  actually *enforces* the invariant, but it is heavy (script data must carry type info) and
  couples to the item-13 domain-type family (AnimId/MappingFrame). Long-term "right" answer.

**Recommendation for the gate:** ship **Option A** (`?`-optional) as the low-ceremony honest
marker *if/when* item-13 touches animate — it makes the optionality first-class and greppable
without pretending to enforce the unenforceable — and record **Option D** as the enforcing
successor gated behind the animation-script domain-type work. This is a language-taste call;
deferring the pick to the gate + Volence per the design-defer convention. **No implementation
this arc.**

---

## Gate questions (for the overseer / Volence)

1. **Item 1 reframe:** accept that the H-1 spin is broken and item 1 becomes a PLAIN
   byte-changing correctness fix (1a, capture-then-test, +2 bytes plain, both twins) +
   the debug watchdog (1b)? Both shapes re-baseline; c1/c2 split as proposed. (I hold before
   cutting either.)
2. **Item 1 fix form:** capture-then-test (recommended) vs branch-before-release
   (DrainSfxRing-style)? Either closes the bug; the former is minimal.
3. **Item 1 flag-lint marker:** log `[branch.flag-clobbered-before-use]` as a gap-ledger
   candidate now (rider on item-4's cc-lattice), or pull it into this arc?
4. **Item 4a:** does the Z80-bus lint's first cut include E008 (start-while-running)?
   Recommend yes.
5. **Item 4b:** approve the expanded-instruction recognition approach (match resolved
   `Z80_BUS_REQUEST` / VDP-port operand addresses, not macro tokens)?
6. **Item 3:** ratify retiring the D11 roadmap line + logging the thin intra-`.emp` residual
   as a low-priority warning idea?
7. **Item 5:** Option A now (when animate is next touched) + Option D as the enforcing
   successor — or hold entirely for item-13?

Items 2 (evaporated) and the ledger/roadmap bookkeeping land at close; nothing byte-changing
or lint-shipping is cut until this note is ruled.

---

## ADDENDUM — GATE RULED (2026-07-23) + `[branch.condition-constant]` built + FIRST-CORPUS-RUN

**All 7 questions ruled** (overseer code-verified the headline first). Reframe accepted
(c1 plain fix + c2 debug watchdog, cut-on-branch, merge HELD behind t18's morning merge);
capture-then-test confirmed (branch-before-release is structurally unavailable — held-bus
deadlock); D11 retirement + item-5 (Option A now / Option D successor, no impl this arc)
ratified; item-4 first cut includes E008; expanded-operand recognition approved.

**Ruling 3 — the lint formulation, RATIFIED + PROMOTED to an item-4 rider (not a ledger
row):** `[branch.condition-constant]` — a conditional branch whose reaching CCR-writer is a
compile-time-**constant** source (`move #imm` / `moveq #imm` / `clr`), making the outcome
statically decided. No intent inference (sound). Overseer soundness riders folded in:
`dbf`/`dbra` never consume CCR (untouched); `Bcc` on C/V after a MOVE is also decided (MOVE
clears both — fire it); the realistic FP surface is comptime-template instantiations going
constant under one parameter (`@allow`, mandatory reason). Guard-rails: **(a)** size-capped —
if the rider outgrows a small bounded effort or first-run FPs outgrow the `@allow` story,
demote to a ledger row and ship item-4 core alone (honest call, reported either way); **(b)**
FIRST-CORPUS-RUN REPORT REQUIRED — every firing beyond `await_slot` is a potential shipped bug
for gate triage, nothing silently fixed/allowed. Coverage: `.emp` corpus only.

### `[branch.condition-constant]` — BUILT (sigil branch `sectionD-backlog`, commit `a184ac6`)

Sound constant-fold dataflow over `flag_check::Cfg`: a 3-point lattice `Top ⊒ Const{z,n} ⊒
Dyn`, join = meet, degrade-on-anything-not-provably-CC-transparent (the OPPOSITE polarity to
`flag_check`'s false-negative-leaning carry analysis — this must not false-fire). 5 TDD tests
(RED-witnessed): the `await_slot` shape fires, the legitimate `btst`/`cmp` spins do NOT, a
`moveq`-fed branch fires, meet-disagreeing constants do NOT. Zero new clippy; frontend-emp
suite green (102+ unchanged). `emp_contracts` bin prints the new firing list.

### FIRST-CORPUS-RUN — 131 procs, both shapes: EXACTLY 2 FIRINGS, BOTH REAL BUGS

Both are the same `startZ80`-clobbers-the-flag class (a `cmp`/`tst` under bus-hold, then
`start_z80` = `move.w #$0000, Z80_BUS_REQUEST` forcing `Z=1`, then a conditional branch
reading the clobber → the branch is statically NEVER taken → the spin never iterates):

1. **`Sound_PlayMusic.await_slot`** (`sound_api.emp:187-191`) — the known bug; **c1 target.**
2. **`Sound_Init.wait_alive`** (`sound_api.emp:132-142`) — **NEW.** `move.w #$2700,sr /
   stop_z80 / cmp.b #SND_ALIVE_MARKER, ALIVE_SLOT / start_z80 / bne .wait_alive`. The 68k's
   "block until the Z80 driver posts STAT_ALIVE before touching sound" boot handshake **never
   actually blocks** — it probes `ALIVE_SLOT` once and proceeds regardless of whether the
   driver finished init. Same corroboration (`Sound_DrainSfxRing` branches before `startZ80`);
   same fix (capture-then-test).

**GATE TRIAGE (Sound_Init):** same defect class, same file, same fix as `await_slot` — I
recommend **folding Sound_Init's fix into c1** (one commit closes both the same way).
Awaiting the triage ruling before cutting c1's final scope; `Sound_Init` is NOT fixed until
ruled. (The lint paying for itself on run #1 — a second silent shipped bug — is the intended
correctness-hardening dividend.)

**NEXT (meanwhile-authorized):** build item-4 core (E006/E007/E008/E011 Z80-bus machine-state
lint) on the same branch (byte-neutral). c1/c2 aeon fixes held for the Sound_Init triage.

### ITEM-4 CORE — BUILT (sigil `93a309f`) + ARC CLOSED

`[bus.*]` Z80-bus machine-state lint shipped: `[bus.double-stop]`/`[bus.start-without-stop]`/
`[bus.stopped-at-return]`/`[bus.vdp-write-unstopped]` (E011/E008/E007/E006) over `flag_check::Cfg`;
3-point MUST lattice `{Stopped, Running, Unknown}`, zero-FP (Unknown entry seed). 8 TDD tests
RED-witnessed; frontend-emp green (110); zero new clippy. Gate rulings honored: E008 included (4a);
expanded-operand recognition (4b, keys off resolved `Z80_BUS_REQUEST`/VDP-port operands); E006 punts
indirect `(a4)` VDP writes (s4lint.py:1133 caveat, documented). FIRST-CORPUS-RUN (126 procs, both
shapes over the c1/c2-fixed corpus): **0 `[bus.*]` AND 0 `[branch.condition-constant]`** (was 2 → both
fixed). Teeth sentinel-proven (injected double-stop into `Sound_PostByte`'s real `stop_z80()`
expansion → fires). Full close in `2026-07-23-sectionD-backlog-arc-close-packet.md`. HELD for overseer.

### MERGED 2026-07-23 (attack-the-diff PASS)

Masters: aeon **`c39f308`** / sigil **`0c27746`** (--no-ff, sigil gap-ledger conflict resolved
keep-both). Dual rebuild from merged aeon master confirmed NEW CANONICAL plain **`ab787bd1`/421122** ·
debug **`6a19669f`/429165**; full paired strict on merged masters **2484/0/1**; PROVENANCE master rows
match. Pre-merge doc fixes applied (debug CRC `05537ebf`→`6a19669f`; the animate_port "red" corrected to
a self-inflicted AEON_DIR invocation error — phantom drift from a pinned harness on a stale-main tree,
evaporates at merge by construction). Branches + worktrees swept. Arc merges FIRST; t18 daytime dispatch
re-cuts on the fresh masters.
