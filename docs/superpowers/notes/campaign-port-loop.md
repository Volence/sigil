# The port loop (canonical — Volence-ratified 2026-07-10, supersedes all prior loop descriptions)

Per file/tranche. The byte gate is a STEP-1 artifact (the transcribe
verifier), not a permanent style cage — after step 1 proves identity,
later steps may change bytes, paying the lockstep + re-pin tax each time.

**After step 1 the goal is BETTER, not SAME** (Volence, 2026-07-11).
Byte-exactness to the original is proven at step 1 and never owed again.
The continuous gate is only `.emp == AS twin` — two forms of the same
file agreeing — and it is satisfied by EDITING BOTH SIDES in lockstep,
never by freezing the .emp to the twin's stale spelling. "The twin has
X", "it wouldn't be byte-exact", and "re-pin is work" are never reasons
to keep old spelling or skip an improvement: the re-pin tax is the
routine, budgeted cost of steps 2–5. Declining an improvement is a
step-5 decision logged in the packet ("not taken, because X") — never a
silent default.

**Step 0 — Recon + design** (when the file carries hazards): read the real
tree, extract per-shape addresses, settle design questions in a written
note BEFORE code. Trivial files skip it.

**Step 1 — Transcribe**: the 1-1 faithful port. Same mnemonics, same
explicit widths, comments carried. Gate + region pin + byte gates both
shapes + mixed-build acceptance + negative probes + gate-off neutrality.
Language features the file DEMANDS ship here (the demanded-features law).
Byte-exactness is proven HERE and only owed here.

**Step 2 — Modernize**: convert to the complete house format. ALL
control flow goes new-style — `bra.w`/`bra.s`/`jmp` → `jbra`,
`bsr.w`/`bsr.s`/`jsr` → `jbsr` (`jmp`/`jsr` stay ONLY for computed
targets), conditional branches go BARE (no `.s`/`.w` — the assembler
width-selects over the relaxation ladder; ratified 2026-07-10). The ONLY
width exceptions are STRUCTURAL, each with a site comment naming which:
(1) transliteration blocks pinned to a macro expansion, (2)
stride-locked jump-table slots (load-bearing `bra.w` tables). "The AS
twin has an explicit width" is NEVER an exception (clarified 2026-07-11;
the core.emp `bne.w RunObjects_Frozen` pin was a misapplication of the
old wording) — the twin is ours: when a bare Bcc relaxes, shrink the
twin in lockstep and re-pin, same as any step-2 byte change. Also:
bare-symbol width-rule spellings, new-style idioms. Bytes MAY change;
constraint is BEHAVIOR-IDENTICAL (spelling/idiom/layout/dead padding —
no logic change). AS twin edits in lockstep, pins re-derived, gates
re-green. No emulator time needed at this step.

  **Brace-indent rule** (Volence, 2026-07-11; style, not
  assembler-enforced): every `{` block body indents ONE level — `if`/
  `else` blocks (`if DEBUG == 1 {`), construct bodies, any braces. Labels
  inside a block keep their usual shallower offset relative to that
  block's instructions. The eye must see membership without hunting for
  the `}`. Formatting-only (bytes unaffected); existing files re-indent
  at their next touch, not as a dedicated wave (a future `sigil fmt`
  would mechanize this — ledgered, zero tooling demand yet).

**Step 3 — Retrospect**: three explicit deliverables —
  (a) **language/format asks**: what did this port need that the language
      or the house format lacks or does awkwardly? (The campaign's main
      feature-discovery engine — a named output, not a side effect.)
  (b) reads-wrong / could-be-better findings;
  (c) new mirrors → kill-list rows, gaps → ledger.

  **The step-3(b) interrogation** (added 2026-07-11, same rationale as
  step 5's: reads-wrong is a checklist, not a vibe. Run EACH; outcomes
  named in the packet):
  - **Comment-claim audit** — every comment that makes a CLAIM ("no X
    can happen yet", "equivalent to Y", "safe because Z") is verified
    against the code AS IT NOW STANDS; a false or half-true claim is a
    finding (the "no band can overflow yet" class — true for the check,
    false for the bookkeeping).
  - **Contract audit** — every proc header's In/Out/Clobbers/Preserves
    matches actual register usage including transitive callees; a
    repurposed register is documented AT the repurpose site.
  - **Name audit** — labels and consts say what they do NOW (a
    `.budget_ok` that also means skipped-the-commit is a rename
    candidate).
  - **Magic-number audit** — every bare literal is a named const or
    carries a site comment saying what it encodes.
  - **Cold-reader test** — trace one frame/call through the file using
    only headers and comments; every point where you must read the
    implementation to know what happens next is a finding.

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

  Also scan for STRUCTURAL clones — N-variant duplicated bodies
  differing in one or two terms (the `emit_piece_loop` class): they are
  adopt/build/ask candidates even when each copy is individually clean
  code, and the varying terms name the template's parameters.

**Step 5 — Optimize**: the real question — is this ENGINE CODE actually
good? Algorithmic/cycle-level, not assembler spelling. Behavior-affecting
changes live here and need LIVE verification (oracle) on top of the
lockstep + re-pin mechanics. "No changes, recorded why" is a valid
outcome — but only AFTER the interrogation below, with each line's
outcome named in the packet.

  **The step-5 interrogation** (added 2026-07-11 after the t11 sprites
  review: a second look found real items behind a "no changes" verdict.
  "Looks hand-optimized" is ANCHORING, not analysis; "no profiler" blocks
  measurement, never inspection. Run EACH line, per hot proc):
  - **Invariant ladder** — classify every loop instruction by the WIDEST
    scope it's invariant over (iteration → object → band → frame →
    build); anything sitting below its scope is a hoist/fold candidate
    (the camera-bias class: per-piece `addi #128` folds into the
    per-frame camera read).
  - **Counter/cache audit** — for every counter, cache, or budget: list
    ALL writers and ALL readers; every path that consumes the guarded
    resource must charge it, or the asymmetry is documented as intended
    (the scanline-budget class: an early-out skipped the COMMIT; a
    sibling path skipped the charge entirely).
  - **Guard-coverage audit** — for every limit/safety check: enumerate
    every emission path; is the check on all of them? Name which checks
    are LOAD-BEARING vs redundant (the dbeq cap-net class: sole guard
    for uncached counts — protect it from future "cleanup").
  - **Hardware cross-check** — every VDP/hardware-facing behavior gets
    checked against the documented quirks (sprite-mask first-sprite-
    on-line exemption, per-line sprite/pixel limits, DMA boundaries…);
    what can't be verified statically becomes a named oracle probe in
    the packet, not a silent assumption.
  - **Silent-tradeoff comments** — every accepted behavioral compromise
    (cascade-down under overflow, a skipped call that's coincidentally
    equivalent, unconditional fairness cycling) gets a site comment
    saying it is CHOSEN — an uncommented compromise is a finding.

  **Hot-path second look**: files on the per-frame hot path (render,
  physics, object/frame loop) get a Fable review pass before the merge
  gate — the checklist raises the floor; the second look is the ceiling.

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
