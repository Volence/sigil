# PROPOSAL — the port loop grows a "construct pass" + a gated corpus sweep

**Status: DRAFT for Volence's ratification.** Does NOT supersede
`campaign-port-loop.md` until ratified. Grew out of the sfx_bank observation
(2026-07-11): adding an SFX is a repetitive multi-place edit, and nothing in
the loop ever forced the question "should this be a construct?" — it worked
byte-exact, so no signal fired.

## The philosophy (why this belongs in the loop)

We already have one continuous-improvement reflex: when a port reveals the
**language primitive** is missing something (the demanded-features law +
step-3 asks), we build it right there — the language grows *while* we
live-port. This proposal extends the same reflex to **reusable
macros/constructs**: when a port reveals a repeated shape a macro should
own, we build the macro right there too. The point is a corpus that
*compounds* — each port leaves the toolbox bigger — instead of every file
re-hand-rolling the same unroll / table / pad dance because "it works."

The gap this closes: today's construct-discovery (step 3a) is
**pain-triggered** — "what did this port *need*." Code that WORKS but is
repetitive (sfx_bank's 18 copied blocks, DeleteObject's 20 unrolled clears)
emits no pain, so it's never flagged. `clear_longs` only happened because we
happened to look. This makes the looking un-skippable.

## Two changes

### Change 1 — NEW in-loop step: the **Construct pass** (after Retrospect)

Insert between the current step 3 (Retrospect) and step 4 (Back-propagate).
Renumbers 4→5 (Optimize) and pulls the old 4 out to the final gated sweep
(Change 2). Its mandate, per file:

**Scan the ported code for repeated / patterned emission, and for each pattern:**
- **(a) an EXISTING construct covers it → adopt it here.** Convert the
  hand-code to `offsets` / a comptime-fn helper / `dispatch` / (once it
  ships) `table`. Usually byte-neutral (constructs are metadata / emit
  identically — the byte gate is the proof).
- **(b) no construct covers it, and it's SMALL → build the macro in-port.**
  A comptime-fn helper (`clear_longs`, `rep`, `reload_anim_timer` class) is
  minutes of work, byte-neutral, done now. Apply it to this file.
- **(c) no construct covers it, and it's BIG → it becomes a step-3(a) ask.**
  A new grammar/lowering construct (`table`, `dispatch`) can't be built
  in-port — it needs design + ratification + its own implementation. Route
  it out; do NOT hand-build a stopgap.

- **(d) it's DEAD → delete it — but only INCIDENTAL dead code.** "No
  callers" is NOT "dead." Two tiers, and the default is caution:
  - *Incidental* dead code — orphaned or obsoleted **by our own work**
    (a transcription artifact, a label a step-2 shrink killed, code a
    refactor made redundant, scaffolding an upgrade retired) → delete it
    (still surfaced at the merge gate; Volence sees every deletion in the
    packet).
  - *Deliberate* dead code — a proc/feature with no caller that looks
    **intentional**: forward-scaffolding built ahead of its consumer, an
    alternate path, an API waiting for the subsystem that uses it → do NOT
    delete. **Flag it to Volence first**; the call is his (the
    `AnimateSprite_PerFrame` precedent — even a strong −404-byte deletion
    candidate was a gate RULING, not an automatic cut).
  - **When ambiguous, treat it as a feature → flag, don't cut.** Also
    cross-check the twin-scaffolding kill-list: anything there has an
    explicit kill CONDITION — never delete it before that condition is met.

The **size gate (b vs c)** is the load-bearing distinction: build what's
cheap now, route what's expensive to the proper channel. It keeps the step
honest — every port asks "should this be a macro?", the cheap yeses ship,
the expensive yeses become tracked asks.

This step is INSIDE the loop-until-dry (building a macro can surface a fresh
retrospect item — re-run 3 → construct-pass → optimize until dry).

### Change 2 — Back-propagate becomes the **final gated corpus sweep**

Pull the old step 4 (back-propagate to all prior files) OUT of the inner
loop to a single step run ONCE after the tranche converges. This matches the
loop's existing "one combined wave, not two" intent — it just stops
re-sweeping the whole corpus on every inner iteration, and it generalizes
the trigger:

**Any new addition this tranche made that PRIOR FILES could use — a format
idiom, an adopted/built macro, OR an optimization — triggers a sweep of all
previously-ported `.emp` files. Retrofit where clean; LEDGER where blocked.**

- The trigger is "new thing **prior files could use**" — a construct or
  optimization unique to this one file earns no sweep.
- The output is **retrofit-or-ledger**, never retrofit-everywhere: a site
  blocked on an unshipped dependency (e.g. `table`-adoption blocked on the
  composite-cell fixup) gets a ledger row, not a forced conversion — else
  the sweep stalls.
- **Verification differs by kind:** construct-adoption is usually
  byte-neutral (cheap, byte gate). An OPTIMIZATION sweep changes bytes —
  each retrofit pays the re-pin + live-verify tax. Same trigger, different
  cost; budget accordingly.

This also closes the structural hole a per-port step can't reach:
**constructs ship AFTER files are ported.** `table` is being built standalone
(not during a port), so no step-4 would ever carry it back to sfx_bank. The
generalized trigger makes it an obligation of the ADDITION, whenever/however
it ships — the moment `table` lands, the sweep sends someone back to
sfx_bank + the six back-patch macros + the PLC files.

## The revised loop (numbering)

- **0 Recon + design** — unchanged.
- **1 Transcribe** — unchanged (byte-exactness owed here).
- **2 Modernize** — unchanged (spelling / house-format: bare Bcc, jbra/jbsr,
  Sst.field).
- **3 Retrospect** — unchanged (asks / reads-wrong / mirrors → kill-list,
  gaps → ledger). Big construct opportunities land here as asks (3a).
- **4 Construct pass** *(NEW — fills the slot Back-propagate vacates)* —
  adopt existing constructs; build small macros in-port; big ones → 3a asks;
  delete INCIDENTAL dead code (flag deliberate/feature scaffolding to
  Volence first — "no callers" ≠ "dead"). Size-gated. Byte-neutral by
  default.
- **5 Optimize** *(unchanged — still 5)* — engine cycle/algorithm;
  live-verify.
- **Loop 3→4→5 until dry.**
- **6 Corpus sweep** *(this IS the old step-4 Back-propagate, moved out of
  the inner loop to a single final gated pass)* — if the tranche added
  anything prior files could use, sweep all prior `.emp`; retrofit or
  ledger.
- **Merge** — packet, Volence gate, `--no-ff` both sides. Unchanged.

## Why the sfx port is the exemplar

sfx_bank was ported the old way: 18 hand-copied blob/pad blocks + a
122-hole hand-typed sparse table, all byte-exact, all green. Under the
revised loop:
- The **Construct pass** would have flagged the repeated block → since
  `table` is BIG, routed it to a 3a ask (which is now the ratified `table`
  design). The per-item pad `if len%2 …` → a small helper, buildable in-port.
- The **corpus sweep**, triggered when `table` ships, retrofits sfx_bank +
  the six back-patch macros + PLC — or ledgers the packed-cell-blocked rows.

The old loop produced neither because nothing forced the "should this be a
construct?" question against working code.
