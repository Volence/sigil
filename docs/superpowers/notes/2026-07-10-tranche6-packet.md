# Tranche 6 checkpoint packet — COMPLETE THROUGH A DRY RETROSPECT (2026-07-10, awaiting Volence's merge gate)

The OBJECT-BANK OPENER, run under the ratified loop end-to-end:
**test_solid.emp + test_particle.emp** — transcribed byte-exact, modernized
(byte-neutral), retrospected, back-propagated, engine-optimized with LIVE
verification, retrospect pass 2 DRY. **UNMERGED** — this packet is the
checkpoint ask. Branches: sigil `port-tranche6` (worktree, 7 commits off
`e2c967c`), aeon `sigil-emp-tranche6` (main tree, 4 commits off `48c1c10`).

Step 0 (construct-walk #1, solo at Volence's pick) is already on master:
`notes/2026-07-10-tranche6-object-bank-design.md` — the mock prelude hid the
table-less dispatcher at the predicted offset; only `sst_custom` survived.
**Walk #3 (Sonic newtypes vs player physics, Volence driving) is RATIFIED
into the 6/7 gap** so collision.asm ports once into the typed surface.

## What shipped

- **`engine/objects/sst.emp`** — TYPE-ONLY twin of structs.asm's SST (zero
  bytes, no gate/pins): full $50-byte layout, `@`-asserted offsets, 30 drift
  guards over the struct-equ seam, raw int types grouped walk-#3-ready
  (newtypes drop in as annotation diffs). Kill-list row 11.
- **Constants twin grew** RF_COORDMODE/RF_PRIORITY_SHIFT + AF_DELETE
  (8→11 guards; rows 12 + row-2 consolidation) — fulfilling particle_anims'
  jotted "until the constants twin grows an animation block".
- **The two object modules** (regions $10F7C/0xE + $10F8A/0x52, bank
  addresses SHAPE-INVARIANT — a first, one org serves both shapes): typed
  `(a0: *Sst)` bare-field access, `falls_into` Init→Main both files, all
  four labels pub (three have live AS-side consumers through the link).
- **First game-side code gate**: `SIGIL_EMP_TEST_OBJECTS` in
  `gameObjectBankIncludes` (main.asm), org discipline the bank's.
- **Demanded features** (the step-1 law):
  1. **`.w` ImmLink** (`Value16Be` @2) — the objroutine store
     `move.w #(Main − ObjCodeBase), code_addr(a0)`, a link-time symbol
     difference in a word immediate. `.b` stays ledgered (consumer-gated);
     kill-row-4 stage 2 is now half-unblocked.
  2. **asl zero-displacement collapse** in the emp frontend (`(0,An)`→`(An)`,
     movep excepted, all lowering paths) — `code_addr(a0)` must emit the
     4-byte `30BC` shape. Mirrors the AS front-end.
  3. **AS-side deferred `dc.w` compounds** (`Value16Be`, the `dw` precedent)
     — R-T0.4's planned migration, taken up when its first cross-seam
     customer arrived: `objdef`'s `dc.w objroutine(TestSolid_Init)` with the
     sym .emp-owned. Bare-`Sym` keeps `Abs16Be` address behavior.
  4. **Fixup diagnostics**: a compound target with a dangling leaf now NAMES
     the missing symbol(s) (probe 4's finding).
- **The tranche-4 imm32-d16 deferral resolved BY DESIGN**: with SST offsets
  comptime (the typed struct), `move.l #ANI_PARTICLE, anim_table(a0)`'s
  destination folds and the existing `.l` ImmLink carries it — .emp↔.emp
  through the link (Ani_Particle consumed via `equ = extern(...)`, the
  ratified R3-flip spelling; no new machinery).

## Step 2 — modernize (byte-neutral this time)

jbra/jbsr ×4 relax to the SAME abs.w jmp/jsr encodings (targets ~57KB away,
beyond bra.w reach) — zero byte change, no re-pin. Typed bare fields fold to
identical displacements. Contracts declared (`clobbers()` empty on
TestSolid_Init = verified touches-no-regs; the particle pair spelled out
comma-by-comma — see asks).

## Steps 3/4 — retrospect + back-prop

Back-prop landed: **particle_anims de-mirrored AF_DELETE** (imports the
twin's; its port test + tranche-4 probes retargeted to doctor the twin
THROUGH the import; mixed counts 2→12); **exhibit banners** (prelude.emp +
sst_overlay.emp now state the production-SST divergence — teach constructs
there, take layout from the twin).

### Language/format asks (deliverable a)

| Ask | Evidence |
|---|---|
| **Label values in imm exprs** (→ LinkExpr; unlocks an `objroutine(label)` expression helper) | the objroutine store must self-reference via `extern("TestSolid_Main")` — legal (R3-flip) but ceremony; EVERY object port hits it per routine store |
| **equ hygiene** — non-pub equs are link-global | two modules' same-named `OBJ_CODE_BASE` equs collided at link; manual name prefixes required |
| **clobbers() reglist ranges** — `clobbers(d0-d4/a1-a3)` is a parse error while `preserves()` takes movem-reglists | the 8-register contract spelled comma-by-comma |
| **`use`-import of offsets-table labels** as link values | ANI_PARTICLE goes through `extern()` although both sides are .emp |
| (jot, not asked) struct-field reflection for guard generation | sst.emp's 30 hand-written ensures |

All in the gap ledger with full context (+ the recorded Value16Be/asl
truncation divergence — a NEGATIVE deferred `dc.w` difference is loud where
asl truncates; deliberate totality, F5 family).

## Step 5 — engine optimize (LIVE-VERIFIED, −8 B)

test_particle, both twins in lockstep:
- **gravity**: load-add-store → `addi.w #GRAV, y_vel(a0)` RMW (−6 B, ~9
  cycles/particle/frame, hot under the stress emitter; d0 was dead).
- **init**: `moveq #0,d0` + `move.b` → `clr.b anim(a0)` (−2 B). Volence
  challenged this against the Amiga moveq lore — settled with exact yacht
  timings: **16(3/1) vs 16(3/1), a cycle-exact tie**; the moveq idiom wins
  only when the zero register is REUSED across stores (12/store vs 16) or
  for register-longword zeroing. Single store + dead d0 → clr. Template
  guidance recorded here so future objects pick correctly by case.
- TestSolid: optimal as-is. Corpus swept: no other load-add-store instances.
- **Live verification (oracle)**: hand-injected a particle into effect slot
  60 on the new ROM — every init store read back exact (anim=0 via clr.b;
  anim_table=$309DE, the SHIFTED Ani_Particle, proving the .l link value;
  routine word $0FCA), gravity integrated +$20/frame, AF_DELETE despawned
  it, game ran normally throughout.
- **Re-pin paid in full**: everything above the bank shifted −8; gate orgs
  (main.asm ×4, act_descriptor.asm ×2) + all sigil pins re-derived from
  fresh listings (incl. act_descriptor_port's 40-row extern table);
  PROVENANCE re-baselined. New pins: plain `588adf81…`, debug `ed96301f…`.

## Loop-until-dry

Pass 2 over the steps' own yields: clr.b challenge answered (above), stale
doc typos fixed in the sweep, corpus pattern-sweep clean, demo neutrality
re-proven → **DRY, merge-eligible**.

## Verification totals

Strict workspace **1998/0** (tranche-5 close: 1985), clippy clean. Port
gates byte-exact BOTH shapes; **18/18 mixed full-ROM gates** (FIFTEEN-module
composition); 5 negative probes (each doctored run paired with a resolving
control); gate-off neutrality: plain + debug re-proven post-optimize, demo
byte-identical to master (`fe936829…`).

## Process notes (worth keeping)

- **`DEBUG=1 ./build.sh` writes s4.bin** — s4.debug.bin is a MANUAL copy
  (`cp s4.bin s4.debug.bin; cp s4.lst s4.debug.lst`). An earlier neutrality
  check in this session read the stale file (vacuous) before this was
  caught; the re-derivation discipline caught it. Consider a build.sh
  `DEBUG` output-name switch (jot).
- **Oracle: resuming from a breakpoint re-breaks at the same PC** without
  advancing (frame_token proves it) — frame-stepping via `emulator_press`
  is the reliable idiom. Worth an oracle-side fix (step-over-on-resume).

## Open asks (the merge gate)

1. **Merge** sigil `port-tranche6` → master (--no-ff), remove worktree;
   merge aeon `sigil-emp-tranche6` → master (--no-ff); push both.
2. **Construct-walk #3** (Sonic newtypes vs player physics, ~30 min, you
   driving) scheduled in the 6/7 gap — before collision.asm.
3. **Tranche-7 gate re-ask**: do your queued structural engine changes touch
   `engine/objects/collision.asm`? ("not sure yet" at tranche-6 kickoff.)
4. **Empyrean amendment stack** grows: `.w` ImmLink width, the emp zero-disp
   collapse rule, the AS `dc.w` Value16Be deferral (+ its recorded asl
   divergence), fixup dangling-leaf naming. Rides your docs cadence with the
   tranche-5 list.
