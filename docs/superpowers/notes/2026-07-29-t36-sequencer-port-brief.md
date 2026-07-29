# 2026-07-29 — t36 brief: Z80 rung-3a — the sound_sequencer port (scale-1)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target: `engine/sound/sound_sequencer.asm` (2091 L, 51 routines) → `sound_sequencer.emp`
— the interpreter, the biggest Z80 file, and the tranche where the trust surface goes
to ZERO. Rung 3 = sequencer + sfx; this tranche is the SEQUENCER ONLY (sfx is rung-3b,
reuses Mod*). Sigil master = THIS brief's commit; aeon master **`84d23c8`**.

## 0. Bars

- Canonical: plain **`37dd2bb2`/421041** · debug **`bbb822f6`/429102**. Strict baseline
  **2803/0 (1 ignored)**.
- Branches `port-tranche36` BOTH repos, worktrees `.worktrees/port-tranche36`; full
  standard rules (editor rsync, one shape per invocation, cd-every-call, explicit
  paths, no `git add -u`, failures-first, rebuild-worktree-ROMs-after-rebase).
  EXPECTED BYTE MOVEMENT ZERO, STOP-not-absorb; the `.asm` stays canonical
  (Z80-blob-precedes-engine: byte-neutral is the only safe class).
- The other four resident Z80 files READ-ONLY. Parallel porter on `port-tranche35`
  (game-side P2) — different files; merge order ruled at the gates.
- The port-loop doc gained the nine ratified rules since t33 — re-read step 2 items
  1-11 + Standing Patterns + the panel-activation rules before writing a line.
- Checkpoints: **STOP 1 = step-0 design** (see §2 — the trampoline is a demanded
  compiler feature; no sigil wiring before endorsement), then (a)/(b)/(c); loop text +
  t24 doctored controls; valve standing.

## 1. Charter (the t32/t33 template, interpreter corpus)

Scale (1) windowed oracle BOTH shapes (derive the sequencer windows from the listings —
psg/fm precedent says shape-variant bases, layout-invariant; VERIFY, don't assume).
Kill row with the seam-sub-tranche condition (the row-70 shape). The t32 close packet
§5.1 row is the demand table: 51 routines, ~25 Clobbers + 20 Preserves + 2 Out headers
(indicative upper bounds). Full contract set per the psg/fm precedent — every proc
machine-checked, honest-contract derivation per the ratified rule (callee-union ∪
locally-written; conservative direction on f). **The header-accuracy scoreboard
inherits**: 36 procs / 6 machine-corrected over-claims so far — expect more header
lies in a 51-routine file; correct with evidence, all-safe-direction is the pattern
to confirm or refute.

## 2. THE DEMANDED FEATURE — the `ex (sp),hl` trampoline (STOP-1 design gate)

`sound_sequencer.asm:1085` (Sequencer_NextOpcode dispatch): `push hl` → table math →
`ex (sp),hl` → `ret` = computed jump through the return address into one of 32
`SeqOpcodeTable` handlers, leaving hl = restored stream ptr. Two distinct demands —
design BOTH before wiring (demanded-feature TDD, the t27/t32 class):
1. The `ex (sp),hl` instruction form itself, if the wired set lacks it (psg/fm never
   exercised it).
2. The **ret-as-computed-dispatch semantics**: this `ret` is NOT a proc exit — the
   checker's control-flow/exit model and the sibling pair-slot proof both meet a
   deliberate `push` consumed by `ex (sp),hl` + `ret`. Expect the LIFO proof to fail
   on TRUE code here; that failure is the feature spec, not a bug to suppress. Design
   the bless/spelling with the checker (the bless-on-the-producer rule applies: the
   dispatch site names its target class), the handler contract story (handlers receive
   hl = post-opcode stream ptr; zero-tick handlers `jr/jp .fetch`, time-advancing ones
   store + `ret`), and the falls-into/join model for the 32 targets. Commit the design
   note; STOP for overseer endorsement before implementing.

## 3. Known inputs (verify at step 0; tree wins)

- **Trust surface → 0**: `Mod_ReArm` (:814) and `Mod_Advance` (:862) — the last two
  declaration-trusted externs — are DEFINED here and become CHECKED definitions; state
  the conversion explicitly in the report (the t33 Snd_ChanClass precedent). Their psg
  consumers (Psg_ApplyMod shares the Mod_Advance core) ride the existing extern decls.
- **`out(carry:)` cross-proc credit goes LIVE**: the `PsgVolEnv_Resolve` (:674/:704)
  and `FmVolEnv_Resolve` (:744/:781) call sites carry the carry-bail consumers the t32
  residue named; the `[call.flag-result-unused]` credit exercises end-to-end — report.
- `invariant(ix)` per the psg/fm precedent WHERE the headers support it — the
  interpreter walks channel structs via ix but VERIFY per-proc, don't blanket-assume.
- Typed params: the Fm_TransposeClamp pattern inherits — typed at design-named sites
  only, `()` stays the conscious default. The f-tracking limitation stands (demand-1
  ledgered): conservative `clobbers(af)` is the honest direction.
- Hardware/bank prose (SND_SONG_BANK asserts, chip-holds-last-value facts, the
  spindash-transpose fold rationale) = present-tense header prose, C3-verified — NOT
  register contracts (§9-A/§9-B stand).
- Any other operand form the wired set lacks = demanded-feature TDD or STOP; the
  parenthesized `(ix+(field+k))` house spelling stands (flat-form fix still ledgered).

## 4. Panel

**A1 + B1 + C2 + C3 ACTIVE** (C3: the bank asserts, the $A4/$A0 write-order claims,
the Mod-triangle chip-interaction prose — verified read-only against the resident
tree). C1 conditional per the panel-activation rules (check for in-source cycle
annotations; rung 4 owns T-states; flagged call with named sites either way). Lenses
synchronous; dry by panel.

## 5. Duties

Kill rows same-commit; ledger per pass (the trust-surface-zero row, the trampoline
feature, any new residue); close packet with the acceptance delta vs psg/fm (what the
interpreter corpus caught that the drivers didn't — expected candidates: the
trampoline class, header accuracy at 51-routine scale, out(carry) end-to-end);
corrections list; census §5.1 row → DONE. After t36: rung-3b sfx (struct-prefix
mirror, reuses Mod*) → rung-4 driver (T-state capability FIRST) → the seam
sub-tranches.
