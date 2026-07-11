# Tranche 10 handoff — core.asm + dplc.asm (written 2026-07-10; tranche 9 SHIPPED)

Tranche 9 (animate.emp) is MERGED AND PUSHED both sides: sigil master
`96cf5ae`, aeon master `1e94102`; strict **2055/0** against master; pins
plain `50f92f57…` / debug `1dfe4a4c…` (master ROMs rebuilt post-merge,
listings current). Packet: `notes/2026-07-10-tranche9-packet.md` (gate
outcome amended in place: PerFrame DELETED, rep() helper documented).
Worktrees removed at close.

## Tranche 10 — engine/objects/core.asm (328 ln) + engine/objects/dplc.asm
(107 ln) — RATIFIED at the tranche-9 gate

Why: core is the object system's SPINE — `RunObjects` (the per-frame tick
loop over every pool), `DeleteObject` (animate's inbound label), slot
alloc/free — the hottest code in the engine, where step 5 earns its keep.
dplc is its small upstream neighbor (DPLC art streaming). Both engine-side,
`engine/objects/`.

## Step 0 FIRST: the `repin` tool (Volence's ask, 2026-07-10 — "20 minutes
for a simple fix smells funny")

Root cause diagnosed at t9: pins live as ~60 hand-typed literals scattered
across ~10 test files; re-pin waves are string substitution, and every
substitution error (slice ends, stale keys) costs a full suite run to find.
Build BEFORE the port:

1. **One generated pin table** — a `repin` tool (sigil-cli subcommand or
   xtask) parses BOTH listings for the known symbol/region list and emits a
   single `pins.rs` module the port tests import. Slice ranges become
   `base..base + len` COMPUTED from the table — the slice-end bug class
   becomes unwritable; no sweep chaining (the tool always reads current
   listings).
2. **Review stays in the loop** — the tool prints the old→new diff for
   eyeballing; the strict suite still independently verifies bytes, so a
   layout bug cannot silently re-pin itself green. Only the typing is
   automated, never the checking.
3. **Self-deriving-but-CONFINED convsym allowlist** — compute which header
   bytes differ, assert diffs are confined to the checksum + ROM-end
   fields (the exact set shifts whenever the deb2 append changes size).
4. Process note: run the AFFECTED test binaries first, full workspace once
   at the end — not full-suite-per-iteration.

Target: a core-sized re-pin wave ≈ the 5-minute floor (two builds + one
suite run). engine.inc org values stay hand-written (they are build inputs
asl reads — but the tool can PRINT the expected org block for pasting).

## Step-0 hazards (settle in the design note BEFORE code)

1. **The biggest upstream window yet**: engine.inc order is `dplc → core →
   sprites → animate → collision → …`. Any byte change slides SPRITES and
   ANIMATE too — animate_port's base ($2D78/$3032) moves for the first
   time, and `DeleteObject` ($281C/$29AE) is INSIDE the ported region
   (animate's inbound label becomes an .emp export read the other way).
2. **Sweep from CURRENT values** — the t9 stale-key lesson (ledgered): when
   two re-pin sweeps run in one tranche, sweep #2's old-values must be
   sweep #1's outputs, not the pre-tranche pins (the $4A7A/$4A70 miss).
3. **Two files, one tranche**: decide gates — one `SIGIL_EMP_CORE` +
   `SIGIL_EMP_DPLC` pair (two regions, rings/collision precedent) vs a
   single combined region. Prior art favors per-file gates.
4. core touches the object POOLS (Dynamic_Slots etc. — RAM labels,
   per-shape VMAs from listings) and is called from game states + the
   engine block — expect the largest outbound-consumer surface so far;
   the object_index/dispatch tables may exercise new operand forms.
5. dplc reads mappings/art pointers — check for `FRAME_*` constants
   (twin candidates) and any game-contract symbols (row-18 class).
6. RunObjects' pool loop is the step-5 target: per-slot overhead ×66 slots
   ×60fps. Profile numbers from the design note, not vibes.

## Mechanics checklist (animate_port.rs is now the leanest model; rings_port
for RAM-label-heavy shapes)

- Branches: sigil `port-tranche10`, aeon worktree `sigil-emp-tranche10` —
  **SEED EDITOR DATA** (`cp -rp games/sonic4/data/editor
  .worktrees/<wt>/games/sonic4/data/`); verify baseline == pins BEFORE edits.
- House format from step 1's step-2: bare Bcc (.emp only — the sigil AS
  front-end pins .asm widths), jbra/jbsr, Sst.field, pinned exceptions
  commented. Expect hand-width shrinks (t9 found 5 in animate).
- Twin growth: `test_support.rs::engine_constant_equs()` is ONE list
  (now 30); counts derive via `twin_guards()`.
- Packet format: per-pass step-3 vs step-5 breakdown + neither-bucket.
- Step-5 live verification in oracle: RunObjects is EVERYTHING — any
  behavior change shows immediately; the profiler MCP tools
  (emulator_get_profiler) can put numbers on the pool loop.

## Carried context

- Pins: plain `50f92f57…` / debug `1dfe4a4c…` (PROVENANCE tail).
- Kill list: rows 16/17/18 open; row 2 now row-1 class; row 3 closed.
- Open asks: unexported-label diagnostics hint (1 data point); assert
  construct (1/2); packed-record view (1/2 — entity_window ratifies,
  tranche 11+); reglist ranges; AnimId/FrameId boundary demand.
- rep() uneven-timing helper lives documented in sonic_anims.emp
  (probe: sonic_anims_port.rs::rep_helper_compiles_and_repeats);
  AF_DURATION is the recorded fallback design.
- Empyrean amendment stack (Volence's cadence, uncommitted) grew: pc-rel
  target addend form, bare-Bcc shrink-lockstep procedure, export-label
  first-consumer note (now historical — consumer deleted with PerFrame).
- Process: cd EXPLICITLY; numbers from LISTINGS; merge only after a dry
  retrospect + Volence's gate.
