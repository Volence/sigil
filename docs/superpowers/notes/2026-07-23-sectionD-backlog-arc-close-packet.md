# §D backlog arc — close packet (2026-07-23)

**Arc:** the pre-t18 roadmap §D backlog (stage-0 census + gate note
`2026-07-23-sectionD-backlog-stage0-gate.md`). Run concurrently with a
Volence+overseer item-13 construct-walk; this arc did NOT touch `types.emp` or
implement newtypes.

**Branches at close (both unpushed, HELD for overseer attack-the-diff):**
- aeon `.worktrees/sectionD-backlog` @ **`4b5a2c0`** (c1 `c0db661`, c2 `4b5a2c0`).
- sigil `.worktrees/sectionD-backlog` @ **`93a309f`** (rider `a184ac6`, re-baseline
  `32bc836`, core `93a309f`).

**Baseline pins.** Canonical (pre-arc) plain `406c773b`/`421122` · debug
`5752c2e3`/`429107`. **After c1/c2 (re-baselined, sigil `32bc836`):** plain
`ab787bd1`/`421122` (length-neutral — absorbed at `org $10000`) · debug
`6a19669f`/`429165` (+58, the DEBUG watchdog blobs). Item-4 core (`93a309f`) is
**byte-NEUTRAL** — a corpus-only lint, never on the ROM build path; pins unchanged
from `32bc836`.

**Merge order:** this arc merges **FIRST**, before t18's morning merge, onto the
current masters. Each repo owns only its own merge (queue discipline).

---

## What the arc did (5 stage-0 items → outcomes)

| Item | Verdict | Outcome |
|---|---|---|
| 1. await_slot watchdog | **REAL — reframed to a bug + watchdog** | c1 correctness fix + c2 DEBUG watchdog; live-proven. The arc **headline**. |
| 2. @scaffolding attribute | EVAPORATED | shipped in G1, already applied to `Plane_Buffer_Reset`. No work. |
| 3. D11 name-linkage | RETIRED | guard shipped + mandatory; auto-detector target population is a draining kill-list artifact. Roadmap line retired. |
| 4. s4lint absorption #1 (Z80-bus) | **REAL — the arc's biggest build** | rider `[branch.condition-constant]` + core `[bus.*]` (E006/E007/E008/E011). |
| 5. optional-param (AnimateSprite d3) | DESIGN-ONLY | Option A (`?`-marker) when animate next touched; Option D (typed duration byte) gated behind item-13. No impl. |

Two of five evaporated — the stage-0 discipline working as designed.

---

## HEADLINE — the await_slot / wait_alive constant-flag-clobber bug

### The defect (both twins, both boot-invisible, both gate-blind)

`Sound_PlayMusic.await_slot` and `Sound_Init.wait_alive` each placed `startZ80`
(`move.w #$0000, Z80_BUS_REQUEST` → forces `Z=1`) **between** a `tst`/`cmp` and
the conditional branch, so the `bne` read the move's forced `Z`, never the value
test. **The spin ran exactly once and always fell through** — the repost gate
never gated; the driver-boot handshake never blocked. Rare races (needs a second
event before the Z80 consumes the first), so boot never tripped it; identical in
both `.emp` and `.asm` twins, so the byte gate was structurally blind (the
MigrateMasks class).

### The evidence ladder (lint → disasm → ISA → live single-step)

1. **Lint.** `[branch.condition-constant]` (the item-4 rider, built first) fired on
   BOTH sites in the FIRST-CORPUS-RUN — the reaching CCR-def is a compile-time
   constant (`move #$0000`), so the branch outcome is statically decided.
2. **Disasm / ISA.** `move.w #imm, ea` sets `Z` from the immediate; `#$0000 ⇒ Z=1`.
   Unambiguous — the `bne` can never loop. Corroborated in-file: `Sound_DrainSfxRing`
   branches BEFORE `startZ80` (the correct shape); `await_slot` was the lone site
   branching after the release.
3. **Live single-step (the proof the fix works).** Under a forced-nonzero
   `MUSIC_SLOT`, single-stepping the post-fix `bne` with the c2 DEBUG watchdog
   counter in `d4`: the counter drained **`$8000 → $7C64`** before the slot cleared
   — i.e. the loop iterated ~924 times (vs "exactly once" pre-fix). The spin now
   actually spins. **DONE + accepted.**

### The fix (c1) + the watchdog (c2)

- **c1 (`c0db661`, both twins):** capture the slot/marker byte under the bus hold,
  test AFTER `startZ80` (flags clean). `await_slot` uses `d1` (free in
  `clobbers(d0-d4)`); `Sound_Init` declares `clobbers(d0)` (honest contract update —
  boot calls it in post-boot setup). Byte-changing plain, length-neutral. Capture-
  then-test was ruled over branch-before-release (the latter is structurally
  unavailable here — a held-bus probe deadlocks the handshake; the Z80 needs the bus
  released each iteration to run and post the marker).
- **c2 (`4b5a2c0`, both twins):** `SPIN_WATCHDOG_LIMIT = $8000` (~250 ms at
  ~15–20 µs/iter, ≥100× any healthy latency); `if DEBUG==1 { counter + subq +
  raise_error on overrun }`. Self-gates to **zero bytes in plain** (the documented
  no-timeout tradeoff stands); debug `+58`, with an `asl` ripple in DEBUG only (the
  RaiseError blobs push forward branches `.s→.w`, the PlaySFX finding-2 precedent).

**The dividend:** the lint paid for itself on run #1 — it found a SECOND silent
shipped bug (`Sound_Init.wait_alive`) nobody was looking for. Correctness-hardening
is the point.

---

## Item-4 — the Z80-bus machine-state lint

### Two lints, one CFG substrate

Both ride the shared `flag_check::Cfg` (the §11-Q1 CFG the whole contract system
reuses), cloning the `type_slice.rs` / `branch_const.rs` worklist skeleton
(transfer / join=meet / post-fixpoint walk).

**Rider — `[branch.condition-constant]` (`a184ac6`).** A conditional branch whose
reaching CCR-def is a compile-time constant (`move #imm`/`moveq`/`clr`) on every
path is statically decided. Sound (no intent inference): 3-point lattice
`Top ⊒ Const{z,n} ⊒ Dyn`, join=meet, degrade-on-anything-not-provably-transparent
(the OPPOSITE polarity to `flag_check`'s carry analysis — must not false-fire).

**Core — `[bus.*]` (`93a309f`).** The sigil-native absorption of s4lint's
E006/E007/E008/E011:
- `[bus.double-stop]` (E011) — `stop_z80` reached provably `Stopped`.
- `[bus.start-without-stop]` (E008) — `start_z80` reached provably `Running`.
- `[bus.stopped-at-return]` (E007) — a return reached provably `Stopped`.
- `[bus.vdp-write-unstopped]` (E006) — a resolvable VDP-port write reached provably
  `Running` (the crash class).

3-point MUST lattice `{Stopped, Running, Unknown}`, join=meet (disagreement ⇒
`Unknown`). **Zero-FP polarity (the rider discipline the gate mandated):** the
entry seed is `Unknown` — a caller may already hold the bus, and that is not
locally provable — so a lone unpaired toggle at proc entry is deliberately NOT
flagged; only a bus state the code itself made definite fires. This costs s4lint's
"unpaired-at-entry" `Running`-seed catches and buys guaranteed no-FP. The upgrade
over s4lint is real path-sensitivity: s4lint tracks a flat scalar with no joins; a
disagreeing join here is `Unknown`, never a false fire.

### Gate rulings honored

- **4a — E008 included.** The double-stop / double-start pair is the whole balance
  invariant; one more firing condition on the same lattice.
- **4b — expanded-operand recognition.** Keys off the RESOLVED operand, not the
  macro name: by corpus-lint time `stop_z80()`/`start_z80()` are already expanded to
  `move.w #$0100/#$0000, Z80_BUS_REQUEST`. A `move` whose dest is `Z80_BUS_REQUEST`
  ($A11100) is a toggle; a store whose dest is a VDP port (`VDP_CTRL`/`VDP_DATA`,
  $C00000–$C00007) is a fenced access. The `btst #0, Z80_BUS_REQUEST` READ in the
  stop spin is not a `move` → correctly not a toggle (pinned by a test).
- **E006 punt (mirrors s4lint.py:1133's `(a4)` caveat, noted in the doc comment).**
  A VDP write through a register-indirect dest (`move.w d1, (a4)` — the corpus's
  DMA/VRAM setup) is UNRESOLVABLE (address in a register) → not flagged. So E006 is
  largely inert on the real corpus; `sound_api` (the sole bus-toggle file) is where
  E007/E008/E011 have teeth.

---

## Lint scoreboards

### `[branch.condition-constant]` (rider) — 2/2 true, zero FP

FIRST-CORPUS-RUN (131 procs, both shapes): **exactly 2 firings, BOTH real bugs**
(`Sound_PlayMusic.await_slot` + `Sound_Init.wait_alive`, same constant-flag-clobber
class). After c1/c2: **0 firings** — the fixes closed both. Zero false positives.

### `[bus.*]` (core) — FIRST-CORPUS-RUN

| Shape | procs | `[bus.*]` firings | `[branch.condition-constant]` firings |
|---|---|---|---|
| plain (no `-D`) | 126 bodies | **0** | **0** |
| debug (`-D DEBUG=1`) | 126 bodies | **0** | **0** |

**Zero on a just-fixed corpus is success** — and the report says so with the
numbers. The corpus is s4lint-clean; every `stop/start` pair in `sound_api`
balances; VDP writes are all indirect `(a4)`/`(a2)` (E006 inert, as predicted).
`sound_api.emp` is the **sole** file containing a bus toggle (verified).

**Teeth proven by sentinel** (the "zero could be a silent not-analyzed zero" trap,
closed): injecting a deliberate second `stop_z80` into `Sound_PostByte`'s REAL
`stop_z80()` comptime-fn expansion, then running the bin over the corpus, fires
exactly `[bus.double-stop] @ Sound_PostByte`. So the clean zero is a real clean —
the lint analyzes the real expanded bus toggles and would catch a violation.

### Build health

frontend-emp suite green (110 tests, incl. the 8 new `[bus.*]` TDD tests +
5 `[branch.condition-constant]`); zero NEW clippy (the one lib warning is
pre-existing in `eval/mod.rs:721`, untouched).

---

## Oracle live-proof history — reusable protocol + 3 failure modes

Full write-up banked as a gap-ledger row (`campaign-gap-ledger.md`, "Oracle
live-single-step protocol"). Summary: the successful protocol single-steps the
post-fix `bne` under a forced-nonzero slot and reads the `d4` watchdog drain
(`$8000→$7C64`). Three attempts wedged first — each now a reusable caution:

1. **Seed-worktree re-fire** — stale ROM/symbols loaded (`0x6272` decoded to
   `moveq #1,d0/rts`, not `Sound_PlayMusic`); hash-compare + reload symbols from the
   fresh `.lst` before trusting any address.
2. **Fail-loud build guard** — a background watcher can rebuild underneath the
   session; a tiny build-stamp guard is worth it ONLY if genuinely small (honest
   call — for a one-shot proof the manual hash-compare is cheaper). Idea, not mandate.
3. **reset→0xFFFF wedge** — poke state AFTER reset+run-to, never before (reset
   re-inits Z80 RAM and clears the poke).
4. **press-onto-breakpoint terminal wedge** — never `press` while sitting on a
   breakpoint (deadlocks the step engine); resume past it, or drive state via pokes.

---

## Ledger + roadmap bookkeeping

- **4 ledger rows** appended (`campaign-gap-ledger.md`): the oracle-proof protocol +
  its 3 failure-mode cautions (seed-worktree re-fire · fail-loud build guard ·
  reset→0xFFFF wedge · press-onto-breakpoint terminal wedge).
- **Roadmap §D** (`pre-t18-roadmap.md`) marked: item 1 DONE+reframed, item 5
  design-ruled, @scaffolding EVAPORATED, **D11 RETIRED**, s4lint absorption #1 BUILT
  (remaining candidates #2 W026 / #3 E010 / #4 debug-seam left standing).

---

## Per-pass breakdown (step-3 retrospect vs step-5 engine, + neither-bucket)

This arc is diagnostics/correctness, not a port-loop tranche, but framed to the
same discipline:

- **Correctness (step-1-adjacent):** the c1 fix is a genuine bug closure the byte
  gate could NOT catch (both twins shared it) — surfaced only by a NEW lint. The
  standing lesson: a shared-twin defect needs a semantic net, not a diff.
- **Language / diagnostics (step-3):** item-4 is pure language-solidification — two
  new sound lints on the shared CFG, absorbing s4lint toward the "one tool, one
  truth" end-state. `[branch.flag-clobbered-before-use]` (a generalization of the
  constant-flag-clobber class over the same cc-lattice) is logged as a natural
  future rider (gate deferred it — not pulled into this arc).
- **Engine optimization (step-5):** none — the arc is byte-neutral except the c1/c2
  correctness+watchdog, which are fixes, not optimizations.
- **Neither-bucket headline:** the lint-pays-for-itself dividend — `Sound_Init` was
  a second, unlooked-for shipped bug. This is the correctness-hardening the campaign
  values most; foreground it at the gate.

---

## The animate_port "red" was a phantom — AEON_DIR invocation discipline

An earlier draft of this packet flagged 2 failing `animate_port` reference gates
(a 4-byte `bsr`-displacement drift, region-relative `0x133`, `32ec` vs `32e8`) as a
"pre-existing red". **That was a self-inflicted invocation error, not a defect
anywhere.** The failure came from running `cargo test -p sigil-cli --test
animate_port` with the DEFAULT `AEON_DIR` — the **MAIN** aeon checkout (at
`bd9ddf2`, PRE-c1/c2, a pre-fix ROM layout) — while the sigil branch carries the
POST-c1/c2 re-baselined pins. A pinned harness pointed at a differently-pinned tree
produces exactly this phantom drift: the pins expect the post-fix layout, the tree
supplies the pre-fix bytes.

Pointing the harness at the BRANCH tree (matching pins) is clean:
`AEON_DIR=…/aeon/.worktrees/sectionD-backlog cargo test -p sigil-cli --test
animate_port` → **4/4 green** (confirmed). The overseer's own paired strict against
the branch aeon tree is **2484/0/1, INCLUDING animate_port**. The mismatch
**evaporates at merge by construction** — once aeon master carries c1/c2, the main
checkout's layout matches the pins again. No main-checkout rebuild and no re-pin are
needed.

**Lesson (banked):** AEON_DIR invocation discipline — always point a pinned harness
at a tree pinned to the SAME baseline. Running the default (main-checkout) AEON_DIR
after a branch re-baseline cross-contaminates pins-vs-bytes and manufactures phantom
drift. Strict gates for a re-baselined branch must set `AEON_DIR` to the branch
worktree.

---

## Status: HELD for overseer attack-the-diff

Nothing pushed. On the overseer's countersign: this arc merges FIRST → t18 daytime
dispatch on the fresh masters → item-13 implementation brief behind it.
