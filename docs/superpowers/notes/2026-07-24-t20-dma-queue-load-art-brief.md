# TRANCHE 20 BRIEF — dma_queue / load_art conversion (LEAN tranche)

**Dispatch: overseer-cut 2026-07-24 (overnight, Volence-authorized continued
autonomy). Single-lane; t19 merged and swept — no coordination partner.**
Second tranche under the corrected LEAN amendment (in-tranche step 5 FULL;
standalone parcels on other files defer post-conversion).

**Canonical sources (read before cutting code):**
`docs/superpowers/notes/campaign-port-loop.md` (re-read at EVERY step
boundary), the t19 close packet (`notes/2026-07-24-t19-close-packet.md` — the
freshest process template, incl. the debug-blob-vs-branch-reach pattern and
the shape-dependent region precedent), the campaign gap-ledger + kill-list
for the step-0 sweep.

## Scope (port ORDER MATTERS)

1. **`engine/system/dma_queue.asm`** (266 L; `Init_DMA_Queue`,
   `QueueDMA_Critical/Important/Deferrable` + `QueueDMATransfer`,
   `Process_DMA_Critical/Important/Deferrable`, `Drain_Budgeted_Queue`)
   → `dma_queue.emp` — FIRST.
2. **`engine/level/load_art.asm`** (117 L; `Art_Decompress`, `Level_LoadArt`)
   → `load_art.emp` — SECOND (it calls `QueueDMA_Critical`, which becomes
   .emp-owned in step 1 of this same tranche; porting in this order makes
   load_art a normal .emp→.emp caller instead of a churned extern).

Full loop `0 → 1 → 2 → (3 → 4 → 5)* → 6 → merge`; steps 0/1/2 per file;
one dry-panel; step 6 once; one close packet.

## Mechanics (standing bars — t19 values)

- Branch `port-tranche20` both repos, worktrees; **seed aeon worktree's
  `games/sonic4/data/editor/` by rsync from main and verify canonical CRCs
  before any code**.
- **Canonical: plain `eab19b3f`/421159 · debug `f1c1aa12`/429204** (masters
  aeon `3938250` / sigil `d937eeb`). Strict baseline **2509/0** paired,
  AEON_DIR at the branch tree. One shape per build invocation.
- cwd resets every Bash call — cd explicitly; explicit-path commits only;
  never push; failures-first test output; STOP with named probe lists for
  anything needing the emulator (oracle MCP is overseer-only — subagent
  calls deadlock).

## THE HEADLINE OBLIGATION — the QueueDMA ownership flip

`dma_queue.asm` porting flips `QueueDMA_Deferrable`/`QueueDMA_Critical`
ownership under EXISTING .emp callers. The proof-mechanism feed-forward rule
(port-loop step 1) makes the persisted two-module link test REQUIRED per flip
(t15 section/entity_window is the template):

- **dplc.emp** (`QueueDMA_Deferrable`, extern decl at dplc.emp:30/32 — the
  PINNED clobber contract `d0-d4/a1-a2`, callers skip saving a3 across the
  call) — link test + extern decl DELETED same-commit.
- **bg_anim.emp** (`QueueDMA_Deferrable`, decl added in t19) — same: link
  test + decl deleted same-commit. Kill-list row 32 DIES here — update it.
- The .emp proc signatures must carry the SAME contract the extern decls
  pinned (`(d1, d2, d3) clobbers(d0-d4/a1-a2) out(carry: dropped)`) — the
  dead-save callers rely on exactly it; widening is a step-3 finding to
  surface, never a silent change.

## Step-0 hazard pre-sweep (overseer findings — verify and complete;
const-keyed trip-check still owed)

**dma_queue.asm**
- **`movep.l`/`movep.w`** both directions — IN the sigil ISA
  (`crates/sigil-isa/src/m68k.rs` `encode_movep`, golden vectors present).
  Step-0 probe: confirm BOTH frontends' spelling for `movep` with
  displacement operands (`d16(An)` each direction) at the real binding class.
  If the .emp frontend lacks the surface spelling, it is a DEMANDED feature
  (step-1 law), TDD per the divs.w precedent.
- **Three `rept`/`set` assembly-time unrolls**: (1) Init's 32-slot pre-fill,
  (2) Process_DMA_Critical's stride-locked jump table
  (`bra.w .drain_end-.c*8` — 8-byte stride arithmetic, the load-bearing
  `bra.w` table-slot class = STRUCTURAL width pins with site comments),
  (3) the drain unroll. These are the `{code}`-splice loop-template class
  (`emit_piece_loop` reference) — design the .emp shapes in the step-0 note
  BEFORE code; the twin keeps its rept spelling (lockstep is byte-level).
  `jmp .jump_table(a1)` is a computed target — `jmp` stays, per the idiom
  list. `trap #0` filler slots — confirm ISA/frontend support (same probe
  class as movep).
- **CCR-surgery idiom**: `andi.b #$FE, ccr` / `ori.b #1, ccr` under a
  restored SR (`move.w (sp)+, sr` THEN the ccr op — order is load-bearing:
  the carry contract must survive the SR restore). C2-lens target; confirm
  frontend support for ccr-destination ops at step 0.
- **`disableInts` + SR-save bracket** — the SR-mask bracket construct
  candidate was ledgered in t19 (6 sites, pair-use design question) — this
  file ADDS sites; step-4 adjudication point, demand data either way.
- **Codename comments**: the "item 11:" references (:37-43, :103, :111,
  :147) are ephemeral session codenames — the 3(b) codename audit replaces
  them with the behavioral fact (already adjacent in the comment).
- **The 128KB single-slot split edge** (one free slot → first half enqueued,
  carry CLEAR — torn-art risk ledgered by the t19 panel onto the dma_queue
  rollback work): shipped behavior — do NOT fix in a port tranche (behavior
  fixes ride their own parcel). Carry the honest comments; the ledger row
  already exists.
- `vdpCommReg` — the .emp counterpart (`vdp_comm_reg`) shipped in t15;
  adopt it (macro-port rule already paid).
- **Contract prose**: QueueDMATransfer's header documents preserved regs
  (a3/a5-a6, d5-d7) — under the exhaustive-license convention that's the
  license's complement; make sure the .emp contract says it the house way
  (clobbers is exhaustive; no disclaiming prose).
- RAM symbols: DMA_Queue/slots/budget/overflow-count (debug-gated
  `DMA_Overflow_Count` — `ifdef __DEBUG__` site) — shape-aware.

**load_art.asm**
- Callees: `S4LZ_Decompress`/`ZX0_Decompress`/`VSync_Wait` stay .asm —
  extern-proc decls with drift-guard comments (dplc row-32 spelling; each
  adds its kill-list obligation for the tranche that later ports them).
  `BG_Init` is .emp-owned since t19 — normal `use engine.bg` import.
  `QueueDMA_Critical` — .emp-owned after file 1: normal import, NO extern.
- **Shape-dependent paths**: the `.drop_page` out-of-line handler is
  `ifdef __DEBUG__` RaiseError vs release drain-and-retry — TWO different
  code paths per shape (not just a blob) + the placement comment explains
  branch-reach (the t19 debug-blob-vs-reach tension, already solved
  twin-side here). `RaiseError` → `raise_error` (shipped construct). Port
  faithfully; the byte gate binds both shapes.
- **Register-discipline comment block** (:49-56, loop-live regs chosen to
  survive callees; d7 carries length because QueueDMA clobbers d4) — the
  contract audit verifies it against the REAL callee contracts, which are
  now .emp-side and checkable; this is the showcase for cross-checking a
  hand-written liveness table against typed contracts.
- Tail-call `bra.w S4LZ_Decompress` / `ZX0_Decompress` → `jbra` to extern
  (cross-file tail call; check the game_loop jsr-precedent boundary rule —
  compression code placement decides jbra vs kept-jmp; name the decision).
- `Art_Staging_Buffer` (.l spelled — RAM above abs.w range? verify which
  width the twin resolves and keep the house bare spelling).
- Type layer: `d2` VRAM byte dest is VramAddr-class IN ARITHMETIC (page
  stride add) — A4-i-gated, log-not-type; page table cursor/count are
  candidates to LOG. `(a0: *Act)` on Level_LoadArt per t19 idiom.

## Step-5 / panel notes

- Both files are DMA/VDP/interrupt-critical: **C3 mandatory** in the panel
  (A1+B1+C1+C2+C3).
- Hot paths: Process_DMA_* run per-frame in VBlank (the drain budget is the
  VBlank window itself); QueueDMA_* run from gameplay code. Level_LoadArt is
  init-cold. Profile-first via overseer probes where live numbers matter;
  the ≥~1k cyc/f threshold stands. The Critical jump-table drain is already
  zero-branch unrolled (~64 cyc/entry per its header) — expect measured
  no-cut outcomes; the interrogation lines still run per proc.
- Step-6 candidates to watch: whatever the rept-unroll .emp shape becomes
  may back-propagate (clear_longs class); the SR-bracket adjudication.

## Acceptance

Per-file step-1 gate lists with named artifacts (byte gates both shapes,
region pins, mixed-build acceptance, negative probes, gate-off CRCs, the TWO
ownership-flip link tests); full paired strict green from the branch tree at
every byte-changing commit; dry = full 3→4→5 circuit empty then a clean panel
round; step-6 enumeration; close packet per house format; ledger/kill rows
same-commit. STOP at the merge gate — the overseer countersigns (fresh
strict, dual rebuild, hot-path second look on dma_queue.emp) and runs the
merge sequence + PROVENANCE re-baseline.
