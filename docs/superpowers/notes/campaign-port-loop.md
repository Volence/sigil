# The port loop (canonical — Volence-ratified 2026-07-10, supersedes all prior loop descriptions)

Per file/tranche. The byte gate is a STEP-1 artifact (the transcribe
verifier), not a permanent style cage — after step 1 proves identity,
later steps may change bytes, paying the lockstep + re-pin tax each time.

**Step 0 — Recon + design** (when the file carries hazards): read the real
tree, extract per-shape addresses, settle design questions in a written
note BEFORE code. Trivial files skip it.

**Step 1 — Transcribe**: the 1-1 faithful port. Same mnemonics, same
explicit widths, comments carried. Gate + region pin + byte gates both
shapes + mixed-build acceptance + negative probes + gate-off neutrality.
Language features the file DEMANDS ship here (the demanded-features law).
Byte-exactness is proven HERE and only owed here.

**Step 2 — Modernize**: convert to the complete house format — jbra/jbsr,
BARE conditional branches (no `.s`/`.w` — the assembler width-selects over
the relaxation ladder; ratified 2026-07-10; exceptions: transliteration
blocks pinned to a macro expansion, templates byte-locked to an AS twin's
explicit widths, load-bearing bra.w tables), bare-symbol width-rule
spellings, new-style idioms. Bytes MAY change;
constraint is BEHAVIOR-IDENTICAL (spelling/idiom/layout/dead padding —
no logic change). AS twin edits in lockstep, pins re-derived, gates
re-green. No emulator time needed at this step.

**Step 3 — Retrospect**: three explicit deliverables —
  (a) **language/format asks**: what did this port need that the language
      or the house format lacks or does awkwardly? (The campaign's main
      feature-discovery engine — a named output, not a side effect.)
  (b) reads-wrong / could-be-better findings;
  (c) new mirrors → kill-list rows, gaps → ledger.

**Step 4 — Construct pass** (Volence-ratified 2026-07-11): the same reflex
as the demanded-features law, widened from language PRIMITIVES to reusable
MACROS/CONSTRUCTS — build the toolbox up WHILE live-porting so the corpus
compounds instead of re-hand-rolling shapes. Step 3(a) is pain-triggered
("what did this port NEED"); code that WORKS but is repetitive emits no
signal, so this pass makes the looking un-skippable. Scan for repeated /
patterned emission; for each, one of FOUR verbs (size-gated, byte-neutral
by default):
  (a) **adopt** — an existing construct fits (`offsets` / a comptime-fn
      helper / `dispatch` / `table`) → convert here;
  (b) **build** — no construct fits and it's SMALL (a comptime-fn helper,
      `clear_longs`/`rep` class) → build it in-port, minutes, byte-neutral;
  (c) **ask** — no construct fits and it's BIG (new grammar/lowering,
      `table` class) → it becomes a step-3(a) ask, its own design + build;
      do NOT hand-build a stopgap;
  (d) **delete** — DEAD code. "No callers" ≠ "dead": *incidental* dead code
      (orphaned/obsoleted by OUR work) → cut (surfaced at the merge gate);
      *deliberate/feature* dead code (forward-scaffolding, an alternate
      path, an API awaiting its consumer) → FLAG to Volence first, never
      auto-cut (the `AnimateSprite_PerFrame` precedent). When ambiguous →
      treat as feature → flag. Cross-check the kill-list (kill-conditions).

  **The construct inventory** (the "adopt" checklist — check the file's
  patterns against EACH; don't work from memory. Spec §10 is the canonical
  list; this is the working cheat-sheet, keep it current as constructs ship):
  - `offsets` — `dc.w Target-Base` self-relative word tables (dense ordinal).
  - `table` (D2.36) — counted/sentinel/SPARSE keyed collection: sparse
    `{id: ptr}` blob banks (sfx_bank), count-header record lists (PLC lists,
    the six back-patch macros). NOT dense-conditional-multi-cell yet (mt_bank
    gap, ledgered).
  - `dispatch` (D2.21) — computed state/jump dispatch (encoding-agnostic).
  - comptime-fn helpers — repeated-emission templates: `clear_longs` (unrolled
    fill), `rep` (repeated bytes), `reload_anim_timer`/`perform_dplc`
    (instruction templates), `aabb_axis_test`, `ojz_sec` (validating record
    constructor), `objroutine(label)` (label − ObjCodeBase).
  - contracts — `clobbers`/`preserves`/`out` (reglist form), `let rN: Type`
    (body-position typed register).
  - spelling idioms (step 2, not this pass) — bare Bcc, `jbra`/`jbsr`,
    `Sst.field`, bareword `bankid`/`winptr`, label-in-immediate.

**Step 5 — Optimize**: the real question — is this ENGINE CODE actually
good? Algorithmic/cycle-level, not assembler spelling. Behavior-affecting
changes live here and need LIVE verification (oracle) on top of the
lockstep + re-pin mechanics. "No changes, recorded why" is a valid
outcome.

**Loop until dry**: after step 5, retrospect again; anything found →
construct-pass/optimize again; repeat until a retrospect pass comes up
EMPTY.

**Step 6 — Corpus sweep** (Volence-ratified 2026-07-11; was the old
in-loop step-4 back-propagate, pulled OUT to a single final GATED pass —
"one combined wave, not two", stated plainly): ANY new addition this
tranche made that PRIOR FILES could use — a format idiom, an adopted/built
macro/construct, OR an optimization — triggers a sweep of ALL
previously-ported `.emp` files. **Retrofit where clean; LEDGER where
blocked** (a site waiting on an unshipped dependency gets a ledger row, not
a forced conversion — else the sweep stalls). Trigger is "new thing PRIOR
FILES could use" (something unique to this one file earns no sweep).
Verification differs by kind: construct-adoption is usually byte-neutral
(cheap, byte gate); an OPTIMIZATION sweep changes bytes (re-pin +
live-verify per site). This trigger also closes the hole a per-port step
can't reach: constructs ship AFTER files are ported (a standalone build
like `table` has no in-tranche step-4), so the obligation attaches to the
ADDITION, whenever/however it ships.

**Merge**: only after a dry retrospect + the corpus sweep — checkpoint
packet to Volence, his gate, then --no-ff merge both sides + push. Every
merge to master is FINISHED code, not faithful-but-stale-idiom code.

**Packet format (Volence-ratified 2026-07-10)**: the packet ends with a
"What each pass added" section separating STEP-3 findings (asks / reads-wrong
/ kill rows / ledger) from STEP-5 findings (optimizations taken + not-taken),
PER LOOP PASS — so each look of the tranche shows what it added. Findings
that fit neither (step-1 demanded features, live bugs, probe outcomes) keep
their own headline bucket.

Keep tranches small (2-3 files): step 2 makes re-pins routine, and
short-lived branches keep the re-pin tax per-tranche instead of
compounding against master drift.
