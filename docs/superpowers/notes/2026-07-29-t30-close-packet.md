# 2026-07-29 — t30 close packet (game-side G2: the effect/child-lifecycle trio)

Porter: Opus subagent (Fable-dispatched, direct). Brief:
`2026-07-29-t30-g2-effect-objects-brief.md`. Design: `2026-07-29-t30-step0-design.md`.

**Outcome:** the census's G2 tranche ported — `test_emitter.emp` +
`test_stress_emitter.emp` + `test_churn.emp`, proving the game→children.emp/
core.emp EFFECT SEAM at scale from the `.emp` side. **Byte-delta ZERO throughout**
(step-1 faithful compiled byte-identical on the first link; step-2 branch flips
byte-identical — every callee is abs.w in both shapes, short Bcc width-select to
.s). Bank shape-invariant (no slide, no $8000 bar). Strict **2691 → 2711** (the
overseer's oracle round: +8 tranche tests, +12 oracle tests).

Branch tips at close: aeon `26ddf69`, sigil `3514e8b` (pre-panel; +oracle finisher
aeon `8131ab1`/sigil `50c97a4` already folded in).

## Region derivation (both shapes; bases shape-invariant, bank NOT slid)

| Lane | region | base (both) | end anchor | len | org resume |
|---|---|---|---|---|---|
| A test_emitter | TestEmitter | `$10FDC` | **TestChildPart** | `$54` | `org $11030` |
| B test_stress_emitter | TestStressEmitter | `$1115C` | TestChurnObj | `$5A` | `org $111B6` |
| C test_churn | TestChurnObj | `$111B6` | ObjDef_PathSwap | `$78` | `org $1122E` |

test_parent.asm sits BETWEEN lane A and lane B (not ported); its FIRST label
`TestChildPart` ($11030) — NOT `TestParent` (its 3rd label, $110C0) — is
test_emitter's end anchor. TestChurnObj (stress's end = churn's start) is gated
out on BOTH sides, so only TestChildPart + ObjDef_PathSwap are AS-visible anchors
in the whole-ROM proof. Content bytes track cross-seam operands per shape →
compile-twice class.

## The oracle — neither-bucket headline (the tranche's biggest story)

**test_churn is the FIRST ported object Main that WRITES a0** (save/restore self
around AllocDynamic, restore before the `jbsr DeleteObject` self-delete). Every
prior object left a0 read-only (trivially preserved). Its `ObjRoutine` dispatch
contract (`proc (a0: *Sst) preserves(a0, d7)`, core.emp:21) MANDATES preserving
a0; it does — but transitively (DeleteObject preserves a0, restored before rts).
The LOCAL `verify_preserved` conservatively treated the trailing preserving-call
as clobbering → a0 unprovable → a HARD BLOCKER (declaring `preserves(a0)` errored
the byte-gate lowering where DeleteObject is a synthetic contract-less callee;
omitting it fired the corpus closure; `clobbers(a0)` violates the ObjRoutine
bound). **The contract system GREW the capability the player cluster needs:** the
overseer's finisher added the CALLEE-PRESERVES ORACLE — `verify_preserved` takes a
CallPolicy that credits a call to a callee whose VERIFIED effective set omits rN
(the same `closure.effective` map `find_dead_saves` reads, one convention);
per-file `check_preserves` DEFERS when only unknown-contract callees block a
declared preserves (a local hazard still errors); the corpus closure is the FINAL
AUTHORITY, with an oracle round between the base and final closure. +12 oracle
tests; the churn now carries `preserves(a0)` with the contract fact in its header.
This is the first transitive-preserve capability — the player state machines
(save/restore-heavy, preserving-call-heavy) will lean on it.

## What each pass added

**Step 1 (demanded features / neither-bucket):**
- **Value16Be word-immediate link-deferral SHIPPED** (as-frontend, sigil `3db4905`)
  — the `move.w #objroutine(Sym), (An)` object-spawn idiom (object_test_state.asm
  spawning the now-.emp-owned TestStressEmitter/TestChurnObj) needs the AS
  front-end to defer an unresolved 16-bit immediate. t29's outbound consumer was a
  `dc.w` (already deferred); t30 is the FIRST tranche gating out an object
  referenced by a `move.w #` IMMEDIATE. `try_defer_long_imm` generalized to size W,
  scoped to memory dests (register-dest word imms stay long-only), with the disp-0
  `SST_code_addr(a1)`→`(An)` fold. **Endorsed as shipped.** Enabled the whole-ROM
  mixed arm (the brief's stated value).
- **EffectSpawn1 typed descriptor record** — sigil's `dc` cannot hold a
  link-EXPRESSION (`extern-extern`), so the emitter descriptor's `dc.w TestParticle
  - ObjCodeBase` objroutine word rides a file-local `struct EffectSpawn1 (size:6)
  { code: ObjRoutine, x_off/y_off: i8, term: u16 }` + `data … = EffectSpawn1{…}`
  (struct fields carry link-exprs; raw dc can't). Byte-identical, sigil-native.
  **Endorsed as shipped — arguably better than the `dc`-link-expr ask it works
  around; the ask stays ledgered.**
- `vram_art` ADOPTED (3 files); `VRAM_TEST_OBJ: VramTile` game-config mirror ×3
  (drift guards resolve against config/constants.asm, which survives).
- Self-contained `vars` overlays (TEmitterV/TStressEmitterV/TChurnV) — UNLIKE t29's
  DplcV, the field equates live ONLY in the gated-out `.asm` (single consumer, no
  surviving AS truth) → ZERO extern drift guards; offset $2E guaranteed by sst.emp.
- Zero externs (every callee resolves module-to-module).

**Loop pass 1 — Step 3 (reads-wrong / asks):**
- CENSUS/BRIEF ANCHOR ERROR caught by the byte gate (TestChildPart, not
  TestParent). Corrected repin.toml/main.asm/pins. CREDIT: the windowed byte gate.
- Contract audit CLEAN: all four inits' `clobbers` = the callee union incl.
  falls_into; churn Main `d0-d3/a1` + `preserves(a0)`; emitters `d0-d3/a1-a2`.
- Comment-claim audit CLEAN (a0/d7 preservation true; the effect-seam comments
  match the callee contracts).
- Asks (all LEDGERED, no file change): the `dc`-link-expr / objroutine-in-data ask;
  item-13 objroutine demand 3→9; the Frames/Timer duration newtype candidate
  (A4-i-gated, low pri); the band-bucket live-example note.

**Loop pass 1 — Step 4 (construct):**
- EffectSpawn1 born DUPLICATED across the 2 emitters (structural clone) — NOT
  hoisted (no shared game-objects/descriptor module yet; VRAM_TEST_OBJ ×3 class).
  LEDGERED: consolidate with G3's child-descriptor format at a shared home.
- Adopt checklist: offsets/table/dispatch/assert — none apply (no dc.w Target-Base
  tables, no sparse collections, no computed dispatch, no DEBUG asserts in-file).
  Nothing built.

**Loop pass 1 — Step 5 (optimize):** no changes. **C1 INACTIVE (named-site
justification):** the per-frame per-object work in each Main is irreducible — a
`subq` countdown + a conditional spawn/alloc/delete + a tail `jbra Draw_Sprite`.
Named sites: TestEmitter_Main / TestStressEmitter_Main (subq/bne/jbra, ~3 instrs
on the common non-expiry frame); TestChurnObj_Main (subq/bne/jbra draw path; the
expiry alloc/delete path is amortized, runs once per lifetime ~every 4-11 frames).
The t24 step-5 measurement regime that "ran ON them" measured the ENGINE paths
they DRIVE (CreateEffect_Normal / AllocDynamic / DeleteObject / CompactDynamicLive
— the compact-on-full A2 soak) — those optimizations landed in t24 and are frozen
(engine untouchable this tranche). No cycle-relevant site in the driver files;
recorded inactive, not run silently. Invariant-ladder / counter-cache /
guard-coverage / silent-tradeoff / debug-growth all walked → nothing takeable.
Hardware cross-check N/A (no VDP/DMA in-file → C3 inactive).

**Loop pass 2: DRY** — step-3 re-audit clean, step-4 empty (EffectSpawn1
ledgered), step-5 empty. One panel round per the dry-panel rule.

**Panel round (A1+B1+C2, synchronous read-only subagents; C3 inactive — no
hardware). DRY = the round returned nothing that reopens the loop.**
- **C2 (correctness-hazard): nothing new** — independently verified every proc's
  `clobbers` = the exact own-writes ∪ transitive-callee union (incl. falls_into and
  tail jbra); `preserves(a0)` holds on BOTH the `.draw` tail path (Draw_Sprite
  preserves a0) and the expiry `rts` path (push once after `bne .draw`, single pop
  at `.no_replace` on both alloc branches, DeleteObject preserves a0 via its
  trailing `lea -sizeof(Sst)(a0),a0`); stack balanced on every path; CC/Bcc pairing
  clean; Sst fields on the correct pointers (a1=child, a0=self); the churn Main
  correctly EXCLUDES a2 (never touched, all callees preserve it); first-frame life
  MIN+spread ≥ 4 cannot underflow into immediate expiry. Confirmed the churn's
  `movea.l (sp),a0` peek is a redundant-but-harmless reload (AllocDynamic already
  preserves a0).
- **B1 (corpus-pattern): nothing new** — the `extern(code)-extern("ObjCodeBase")`
  data idiom (matches objdef.emp:62), the `vars` overlay spelling (test_animated
  DplcV), the VRAM_TEST_OBJ per-file mirror+drift-guard, the range `clobbers` form,
  and the a0-bareword-vs-`Sst.field(a1)`-qualified discrimination all match the
  shipped corpus. Its ONE finding — `EffectSpawn1` duplicated across the two
  emitters and belonging with the format owner (children.emp, the ObjDef-in-sst.emp
  precedent) — is ALREADY the step-4 ledger row; the panel strengthens the hoist
  case. No shared game-objects/descriptor `.emp` module exists yet (FIRM scope) →
  ledgered for G3/shared-home, not an in-tranche change.
- **A1 (cold-reader): findings map to existing ledger rows / corpus-wide asks —
  nothing t30-local to fix.** The two-emitters-95%-identical + `EffectSpawn1`/
  VRAM_TEST_OBJ duplication → the shared-descriptor + game-constants-module hoist
  ledger rows (65/step-4). The hand-computed `∪`-union `clobbers` comments →
  the S2-D6 checked-clobbers lint (ledgered; the comments are the SHIPPED-SIBLING
  idiom — test_particle.emp keeps its `∪` comment, so trimming t30-locally would
  DIVERGE). Inline priority bands (`#N<<RF_PRIORITY_SHIFT`) → the corpus idiom
  (test_particle/test_animated), not t30-local. EXAMINED-AND-LEFT (adjudicated, not
  a miss): (a) the emitter's omission of `PopulateSpawnedPieceCount` — the AS twin
  ALSO omits it (and its explanation); adding a "spawned via Load_Object" claim
  would be a comment-claim-audit violation (TestEmitter has no found spawn site to
  confirm the path) — faithful omission is safer than an unverifiable claim; (b)
  the churn's `preserves(a0)` proof comment + `(§5)` — OVERSEER-AUTHORED (the oracle
  finisher), `§5` is the contract-grammar spec section (a durable anchor the
  codename rule permits), documenting a genuinely subtle transitive-preserve fact;
  kept. (c) `mapping_frame #1` glossed in test_emitter, bare in the twins — AS is
  identically inconsistent; marginal, left faithful.

**Adjudication: DRY.** No panel finding is a NEW t30-local code change — all are
either already-ledgered infrastructure hoists (blocked on FIRM scope), corpus-wide
asks (fixing locally would diverge from shipped siblings), or examined-and-left
with reasoning. One panel round per the dry claim.

## Neither-bucket headlines

- **The callee-preserves oracle** — see above (the tranche's headline; the contract
  system grew transitive-preserve verification because the churn was the first
  object Main to write a0).
- **First-linking-compile byte-identity, both shapes, zero wave** — the whole
  canonical-bytes tranche moved zero bytes through step 2. No re-pin, no ripple.
- **Two demanded frontend features shipped** — Value16Be word-imm deferral +
  EffectSpawn1 (the `dc`-link-expr workaround). Both endorsed.

## Corrections list

1. **CENSUS/BRIEF ANCHOR ERROR:** test_emitter's end anchor is `TestChildPart`
   ($11030), test_parent.asm's FIRST label — NOT `TestParent` ($110C0, its 3rd).
   Caught by the windowed byte gate (84 B / $54 candidate vs a 228 B / $E4 pinned
   window). Corrected repin.toml/main.asm/pins.
2. **SIGIL MASTER HASH:** brief §0 says `e17b403`; the real master is `4faba19`
   (the brief commit itself). Branched off `4faba19`.
3. **row 63 (vram_bytes hoist) did NOT trip** — the brief flagged it conditional;
   no G2 file consumes vram_bytes (all consume vram_art, already shared). Recorded.

## Kill-list rows (added same-commit)

- Rows 64-66: gate-off body twins + org arms (64); VRAM_TEST_OBJ mirrors ×3 (65);
  self-contained `vars` overlays + EffectSpawn1 structs, no surviving AS truth (66,
  which also records row 63 NOT tripped).

## Test artifacts (green by name, SIGIL_STRICT_GATE=1)

Windowed: `test_g2_objects_port::{g2_objects_regions_match_reference,
g2_objects_debug_regions_match_reference, g2_undoctored_compile_equals_the_reference_window,
g2_doctored_reference_diverges}`. Whole-ROM:
`mixed_dac_rom::{mixed_tranche30_rom_matches_assembled_reference,
mixed_tranche30_debug_rom_matches_assembled_reference}`. Frontend feature:
`imm32_defer::{move_w_unresolved_symbol_to_ind_defers_as_value16be,
move_w_unresolved_symbol_to_disp0_folds_and_defers}`. Contract:
`contract_closure_corpus::corpus_closure_residue_is_empty_the_error_gate` (the
oracle round flipped it green).

## Step-6 sweeps (overseer-ordered at the merge gate)

**1. Oracle-consumer census (the transitive-preserve feature's demand baseline).**
`grep preserves( --include=*.emp`: 24 preserves-declaring procs corpus-wide. The
oracle credits a call to a callee whose verified effective omits the preserved reg;
a proc NEEDS it only if it writes-then-restores the reg with a preserving call
before rts (else the local proof already passes). **Demand today = EXACTLY 1:
`TestChurnObj_Main`.** Evidence: pre-oracle `contract_closure_corpus` fired on
`[("TestChurnObj_Main","a0")]` and NOTHING else (the 2691/0 baseline had all 23
other preserves-procs verifying locally); post-oracle it is green — the sole
fail→pass flip. The other 23 either never write the preserved reg (VBlank_Handler
`preserves(d0-d7/a0-a6)`, the sound_api `preserves(sr)` family, …) or restore AFTER
the last call (movem save/restore pairs). This is the baseline the player cluster
(P1+, save/restore-heavy state machines) grows from — the first non-trivial
transitive-preserve consumer.

**2. Descriptor-format census (for the G3 shared-home ruling).** The
CreateEffect_Normal and CreateChild_Normal descriptors share the SAME 4-byte record
`{code: ObjRoutine :w, x_off: i8, y_off: i8}` (children.emp:542-546 documents it),
terminated by a separate `dc.w 0`. Member sites:
- `EffectSpawn1` (single entry) — `test_emitter.emp:27/71`, `test_stress_emitter.emp:27/72` (PORTED, duplicated).
- AS-side effect twins — `test_emitter.asm:47`, `test_stress_emitter.asm:48` (`.particle_desc`; retire at Spec 5).
- **CHILD descriptor (G3, NOT ported)** — `test_parent.asm:144 .child_desc`: THREE `objroutine(TestChildPart)` entries (left/right/above, `dc.b ±24`) + terminator — the MULTI-entry consumer, same 4-byte record.
RULING INPUT for G3: the shared home is a single 4-byte `SpawnDesc` record (NOT
EffectSpawn1's fused 6-byte entry+terminator — B1 finding 3), used as `[SpawnDesc]`
+ explicit terminator; test_parent is its 2nd/3rd/4th consumer. Natural home:
alongside `CreateEffect_Normal`/`CreateChild_Normal` in children.emp (the
ObjDef-in-sst.emp precedent — the record lives with the format's owner), imported by
the game objects.

**3. Value16Be word-imm census (blast radius PROVEN, not assumed).** `grep 'move.w
#objroutine('`: 10 AS sites. The defer activates ONLY on an UNRESOLVED (.emp-owned,
gated-out) target in a memory dest. Categorized:
- **Current consumers (2, both proven byte-exact by `mixed_tranche30`):**
  `object_test_state.asm:54` (TestStressEmitter) + `:228` (TestChurnObj) — the t30
  gated-out objects.
- **Self-stores in gated-out .asm twins** (`test_churn.asm:56/75`) — don't compile
  in the mixed build (the file is gated out); the .emp owns those stores.
- **Resolve-locally, no defer** — `test_particle.asm:26`, `test_player.asm:59`,
  `test_parent.asm:73/140` (self-Main stores in unported AS objects); and
  `object_test_state.asm:38/218` (TestPlayer, unported → resolves).
- **Pending at P1:** when `test_player.asm` ports, `object_test_state.asm:38/218`
  (`objroutine(TestPlayer)`) become the next 2 defer consumers. Feature proven on 2
  live sites; +2 queued.
