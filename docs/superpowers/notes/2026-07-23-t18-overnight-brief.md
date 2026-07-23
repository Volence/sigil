# t18 — parallax port: RUN BRIEF (overseer-authored)

> **MODE CHANGE 2026-07-23 (day 2): DAYTIME RUN — the "overnight autonomy rules" section
> below is SUPERSEDED.** t18 runs as a NORMAL tranche: full port loop with live overseer
> gates (step-0 design note TO THE GATE before cutting; step-1 verifier artifacts; live
> oracle work allowed, foreground; the 3→4→5 cycle closes ONLY via the DRY-PANEL rule —
> campaign-port-loop.md at master, weighted composition A×1/B×1-2/C×2-3 with C3 active
> since parallax is VDP/HInt-heavy; then step 6 and the merge ceremony at the gate).
> Masters at dispatch: **aeon c39f308 / sigil 48ffe9f**; NEW canonical **ab787bd1/421122 ·
> 6a19669f/429165** (§D arc merged — sound_api changed; re-verify at start, fetch-first).
> Everything else below — the inherited obligations, worktree/seed discipline, per-commit
> ripple, honest-stop conditions — stands unchanged. The [bus.*] and
> [branch.condition-constant] lints are now LIVE on the corpus your port joins.

> **STATUS: READY.** Volence pastes the dispatch block (campaign log / overseer message) when
> the §D backlog arc has closed or checkpointed cleanly. This note is the durable copy — the
> running agent re-reads it as authoritative. Masters at authoring: aeon `bd9ddf2` /
> sigil `7b9afa6` (re-verify at dispatch; fetch-first).

## The arc

t18 = port `engine/level/parallax.asm` (837 lines, ~11 procs, the last big un-ported engine
domain; measured 18-22%/frame in streaming probes — real step-5 material) through the FULL
port loop (campaign-port-loop.md AT CURRENT MASTER — it gained the type-layer items at
`5f242ff`; this is the first port under them).

## Inherited obligations (step-0 must list ALL of these; none are optional)

1. **HBlank RAM-jmp trampoline — ledger row 1088, BINDING.** The ratified first-consumer
   design: ROM HInt vector → fixed RAM slot with patched `jmp`; handler owns save/restore +
   `rte`; install helper; HInt disabled when no handler installed. t18 step-0/1 builds it
   (hblank.emp + twin + boot HInt vector + install sites; byte-changing → re-pin).
   **Its raster-timing LIVE verification is a MORNING item** (see overnight rules).
2. **Hscroll_Dirty_Start/End deletion rider** (parked from Phase-2.5): the only writers are
   parallax.asm :440-445 — step-2 modernization drops the dead stores and deletes the RAM
   symbols (parity statement per the standing RAM-deletion rider).
3. **Parallax H2/H3 optimization candidates** (Tier-B, review board) — step-5 material,
   behavior-affecting → design + interrogation overnight, LIVE-verified cuts in the morning.
4. **Type-layer walk (step-2 item 6, first outing):** parallax's domain values (scanline
   indexes, scroll offsets, deform table entries) are mostly shift/add-computed — expect
   LEDGER-not-type outcomes, but the packet must walk the item and log each verdict.
5. **Step-0 ledger sweep** for file-implicating hazards (standing rule) — rows 1058 (the
   perf charter), 1088, the P1a deformShiftDefault history (H1 already shipped — do not
   re-derive), and any parallax-touching riders the sweep finds.

## OVERNIGHT AUTONOMY RULES (this run is unattended — these are hard)

- **Authorized: port-loop steps 0, 1, 2 in full, then the 3→4→5 inner cycle** — with step 5
  restricted each cycle to INTERROGATION + design notes + any cut that needs NO live
  verification. Because live-verified step-5 cuts are deferred, the cycle CANNOT be declared
  DRY overnight — so step 6 (corpus sweep) and the merge ceremony are morning-gate work by
  construction, not overnight items. Steps 0/1 proceed WITHOUT the usual live overseer
  gate — but every gate ARTIFACT is still produced (step-0 design note, step-1 verifier
  evidence, per-step packet sections) so the morning review can retro-gate.
- **NOT authorized overnight:** any behavior-affecting cut whose verification needs the
  emulator (oracle) — that includes the trampoline's raster-timing check and H2/H3 cuts.
  Implement + statically verify + byte-gate them if the design is clean; the LIVE check is
  the morning gate's first item. If static confidence is not total, leave it designed-only.
- **No merge, no push, no master commits, no PROVENANCE re-baseline.** Everything on branch
  `port-tranche18` in seeded worktrees (tools/seed-worktree.sh; reuse if present — the
  level-data-blob trap is documented in the phase2.5 note §R2). Per-commit ripple discipline
  (pins + hand-sites travel INSIDE each byte-changing commit); full paired strict
  failures-first at every commit boundary — a red gate STOPS the run at that commit with a
  written checkpoint, never "fix it forward" into uncertainty.
- **Honest-stop conditions:** blocked on a design call the overseer would gate; a gate
  artifact that can't be produced cleanly; strict red you can't attribute in one attempt;
  ANY temptation to guess. Stop = write the checkpoint into the packet note, leave trees
  committed-clean, end. A half-ported clean checkpoint beats a full-ported guess. Session
  depth rule applies double overnight.
- **Deliverables by morning:** branch tips both repos; step-0 design note (incl. the
  trampoline inheritance); packet-in-progress with filled per-step checklists; the type-layer
  walk verdicts; a MORNING-GATE list (everything deferred: raster live-verify, H2/H3 A/B,
  merge ceremony). The overseer runs attack-the-diff + the live items at the morning gate.
