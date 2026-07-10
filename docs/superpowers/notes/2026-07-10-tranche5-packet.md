# Tranche 5 checkpoint packet — SPINE COMPLETE (2026-07-10, awaiting Volence)

Fourth tranche under the 5-step loop. Scope A (the spine — Volence-ratified
in the tranche-5 handoff) is DONE on the branch: **game_loop + sound_api**,
both byte-exact both shapes, THIRTEEN-module mixed gates green. Scope B
(stretch: first object port) NOT started — it wants its own step-0 against
the SST-overlay/spawn-template surface, better fresh than appended.

**UNMERGED** — this packet is the checkpoint ask. Branches: sigil
`port-tranche5` (worktree, 3 commits off `cc4535a`), aeon
`sigil-emp-tranche5` (3 commits off `6fe1388`).

## What shipped

- **Step-0 rulings** (`notes/2026-07-10-tranche5-game-loop-design.md`) —
  the handoff's two hazard classes settled deliberately:
  - **H1** (`ifdef SOUND_DRIVER_ENABLED` in a ported CODE file): comptime
    `if` over a define, mt_bank's pattern at PROC-statement position —
    which demanded the tranche's headline language surface (below).
  - **H2** (`gameDebugTick`, the game-contract macro seam): option (d),
    not in the handoff's list — mirror sonic4's macro EXPANSION under the
    same comptime-if. Zero bytes in both pinned shapes (SOUND_DEBUG_HOTKEYS
    is a build.sh env opt-in neither pin sets). (a) rejected: gate-on loses
    the hook; (b) rejected: not byte-neutral + rewrites both games'
    contract; (c) rejected: no demand for an extern-macro construct when
    the body is one `jsr`. Cost named: an engine .emp bakes sonic4's hook
    body → **kill-list row 9**, with the combo matrix as its drift guard
    (it re-extracts the macro body from the REAL config/game.asm each run).
- **Port #1 `game_loop.emp`** (sigil `b683c7f` / aeon `cd43bbb`) — 0x12
  bytes, sixth engine-side gate (engine.inc:136). First CODE module taking
  build-shape defines. Four-combo module matrix vs the AS twin through
  sigil's own AS front-end; TWELVE-module mixed gates.
- **Port #2 `sound_api.emp`** (sigil `9129e80` / aeon `c9470cc`) — 0x1E8
  bytes, twelve procs, the gate inside the SOUND_DRIVER_ENABLED block.
  Design ruling worth reading twice: **slot ADDRESSES stay AS-owned** as
  `equ *_SLOT = extern("SND_Z80_BASE") + extern("SND_REQ_*")` sums — the
  MUSIC_PARAM block derives from a Z80-driver RAM label and floats with
  every driver resize, so a comptime mirror would churn; nothing mirrored
  = nothing to drift = no kill row for them. Only the 7 immediate-position
  values are mirrored + guarded (**kill-list row 10**). The
  SongTable/SongPatchTable imm32s flip the R3 deferral's direction:
  .emp-side consumers of .emp-side definitions through the shared link.
- **Demanded features, shipped mid-port** (the tranche-4 law):
  1. **Statement-position comptime `if`** in proc/asm bodies (H1's
     carrier) — recursive label scope with export-flavor policing,
     script-body label-under-if refusal (a `yield` can't nest by type),
     paren-bomb depth guard, `[asm.if-not-comptime]`; 12 tests.
  2. **`ImmLink`** — link-time `.l` immediates (`#extern(...)`/equ alias)
     → `Value32Be` fixup at offset 2, the emp mirror of the AS side's
     `try_defer_long_imm`. `.b`/`.w` stays the ledgered gap (kill-row-4
     stage 2 still blocked on it, unchanged). Side-effect ratify-me: a
     provisional `here()` in `.l` immediate position now DEFERS (sound
     per D2.23 — the fixup resolves at final positions; bankid-derived
     values keep their rejection).
  3. **Positional abs-sym fence** — `[lower.abs-sym-operand]` relaxed:
     ext-word operands BEFORE the sym operand are legal (the stopZ80
     `move.w #$0100, (Z80_BUS_REQUEST).l` shape; 68k emits ext words in
     operand order so the abs field stays LAST); AFTER stays fenced.
  4. **`sr`/`ccr` operands** (register-class words, the AS front-end's
     rule).
- **Probes** (`tranche5_negative_probes.rs`, 5): misspelled cross-seam
  symbol loud; oversize hotkeys combo COLLIDES with the resume bytes loud;
  drain define load-bearing (−4 B); doctored mirror fails its OWN guard
  against a resolving composition; misspelled extern dangles while an
  undoctored control resolves.
- **Numbers**: strict workspace **1977/0** (tranche-4 close: 1944), clippy
  clean, gate-off neutrality sha256 ×3 at `907a9029…` + debug `7148f938…`
  + demo builds clean, reference pins UNCHANGED (no re-baseline this
  tranche — first tranche with zero re-pins).

## Review discipline record

Per-item two-stage reviews ran on both ports; both earned their keep:
- Port #1's review caught the **mod-2^16-vacuous outbound proof** (the
  far-carrier consumer "passed" regardless of LMA — the displacement only
  matched modulo 65536) → consumers now PHASED in ±32K; the missing
  pc-rel16 link range check is gap-ledgered. Also: unguarded parser
  recursion (paren-bomb class), trailing-junk-after-brace diagnostics,
  export-flavor policing.
- Port #2's review caught the SAME vacuity pattern reintroduced in the
  sound_api consumer (fixed), the abs-sym doc-proof landing on the wrong
  fn, a vacuous misspelled-extern probe (now has a resolving control),
  and a WRONG reads-wrong ledger row (8 tail-calls not 6; only 2 in `.s`
  reach; real jbra saving −4 B not −12 — corrected before step 5 can act
  on bad numbers).
- Two-prong whole-branch review (design conformance + adversarial
  cross-feature, 22 executed probe programs incl. two-frontend byte
  oracles): design prong near-clean (three doc-count fixes, applied);
  adversarial prong found NO cross-feature interaction bug — the flagship
  compose (sr + comptime-if + bare call + pinned-abs + ImmLink + jbra +
  todo!) is byte-exact against hand-assembled 68k in both define
  polarities. Its one Important find, **F1: `move.l #imm, sr` silently
  emitted `sr := $0000` + the imm-word executing as an opcode** — a
  PRE-EXISTING shared ISA hole (both frontends emitted it; asl the real
  assembler rejects) that this branch's `sr` surface made newly writable —
  FIXED at the ISA level (`encode_move_sr` word-only policing heals both
  frontends; pinned). Polish landed: quick-form ImmLink steering (no
  placeholder leak), `(sr).w` early steering. F4 (unmodeled legal SR/CCR
  forms, all loud), F5 (pre-existing word-imm truncation parity
  divergence), F6 (todo-in-unchosen-arm, mangled-name cosmetics) →
  gap-ledger rows.

## Open asks (each a recorded decision when taken)

1. **Spec addendum pass** (empyrean, docs cadence): statement-position
   comptime `if` (H1), `ImmLink` + the here()-in-.l-imm behavior change,
   the positional fence, `sr`/`ccr`. All jotted in the gap ledger.
2. **Step-5 queue** (post-merge, byte-different): the two in-reach
   `bra.w` tail-calls (Sound_Ping/Sound_PlaySample) → `jbra` (−4 B);
   re-pin + aeon lockstep like tranche-4's step 5.
3. **Stretch B** (first object port — test_solid/test_particle): fresh
   step-0 against `examples/sst_overlay.emp` + the pitcher_plant exhibits;
   opens the object-bank neighborhood (SST overlays, spawn templates,
   objroutine dispatch, the code_word encoding's demand moment).
4. pc-rel16 link range check (ledgered; will flip two inherited far-carrier
   proofs in older port tests when it lands — collision_lookup's noted).
5. Kill-list rows 9/10 dispositions ride the usual cadence.

## Post-merge state (when Volence ratifies)

Merge --no-ff both sides, push, remove worktree/branch, then the step-5
queue (ask 2) as its own commit pair. The empyrean amendment stack
(D2.33 + 2026-07-10b + D2.34 + this tranche's addendum) stays in the
working tree per the docs cadence.
