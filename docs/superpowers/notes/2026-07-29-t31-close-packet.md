# 2026-07-29 — t31 close packet (game-side G3: test_parent + the SpawnDesc hoist)

Porter: Opus subagent (Fable-dispatched, direct). Brief:
`2026-07-29-t31-g3-test-parent-brief.md`. Design: `2026-07-29-t31-step0-design.md`.

**Outcome:** the census's G3 tranche ported — `test_parent.emp` (the first game-side
STRUCT-overlay object) + the pre-ruled **SpawnDesc hoist** to `children.emp`. This is
the **FIRST byte-moving game-side wave** (the step-5 optimization cut −12 B/shape); it
is also the **first canonical change since t24**. New canonical baseline: plain
**85111814/421041** · debug **eb5e94be/429102** (sizes UNCHANGED — the object-bank
shrink is pad-absorbed). Strict **2757 → 2763** (+4 windowed `test_g3_objects_port`,
+2 whole-ROM `mixed_tranche31`).

Branch tips at close: aeon **`4cc61cf`**, sigil **`799eb45`** (pre-merge; +the ruling
edits fold in).

## Region derivation (both shapes; base shape-invariant, POST-WAVE)

| Lane | region | base (both) | end anchor | len (post-wave) | org resume |
|---|---|---|---|---|---|
| test_parent | TEST_PARENT | `$11030` | **TestStressEmitter** | `$120` (was `$12C`) | `org $11150` |

test_parent.asm's FIRST label `TestChildPart` ($11030) is the region base (=
test_emitter's end anchor — the t30 anchor-error lesson applied at step 0). The
`TestParent` LABEL pin is renamed **`TEST_PARENT_LABEL`** ($110B4 post-wave) so the
`test_parent` region const (`TEST_PARENT`) does not collide (the
`Plane_Buffer`→`PLANE_BUFFER_BASE` precedent; objdef_port.rs updated). Content bytes
track per-shape cross-seam operands → compile-twice. The wave slid the region END and
the whole downstream run −0xC (see the wave section).

## The SpawnDesc hoist (t30's B1 finding 3 / descriptor census, PRE-RULED)

`pub struct SpawnDesc (size: 4) { code: ObjRoutine @0, x_off: i8 @2, y_off: i8 @3 }`
landed in **children.emp** (the format owner — the ObjDef-in-sst.emp precedent). The
two G2 emitters' fused 6-byte `EffectSpawn1` is REPLACED by `SpawnDesc` + a SEPARATE
`dc.w 0` terminator (byte-identical — the G2 gates prove it). test_parent is the
multi-entry consumer: `[SpawnDesc; 3]` + terminator, emitted inline between TestParent
and TestParent_Main (source-order data-between-procs, the sprites.emp `CellOffsets`
precedent). SpawnDesc is a PURELY `.emp`/harness type — the real dual build uses the
raw-`dc` twins and never sees it. Harness visibility via a FILTERED
`spawn_desc_ambient_items` (children.emp is code-bearing → only the struct item rides).
This DISCHARGES the t30 EffectSpawn1-duplicate ledger row. **The G2 gates stayed green
throughout** (byte-neutral hoist) — the neighbouring-tranche canary the brief named.

## The overlay twins (ZERO drift guards — the row-66 class)

`TParentV {life_timer: u16@0, x_dir: u8@2}` (parent state) and `TOrbitChildV
{orbit_angle: u8@0, phase_offset: u8@1, radius: u16@2}` (child state), both over
`Sst.sst_custom` (disjoint lifetimes — parent and child never coexist in one slot).
Natural `vars` layout reproduces the AS `ds.w/ds.b`/`ds.b/ds.b/ds.w` offsets exactly.
**ZERO extern drift guards** — single-consumer (grep of the whole AS `games/` tree finds
no surviving reader of the five overlay equates), so the row-66 (TEmitterV) class, not
the row-61 (DplcV) class. **PORTER-VERIFY resolved:** the two `objvarsCheck` calls
(`> SST_CUSTOM_SIZE → fatal`) are SUBSUMED by the two always-on `[overlay.window-overflow]`
checks — NO separate `ensure`. The `angle`→`orbit_angle` rename avoids
`[overlay.shadows-field]` vs Sst's direct `angle` (single-consumer → no drift guard on
the rename).

## The byte-moving wave (step 5) — the oracle's FIRST optimization use

Both step-5 candidates authorized + taken in lockstep across both twins:
1. **Dead `movem.l d2-d3/a1` around GetSineCosine.** GetSineCosine is `.emp`-owned with
   `clobbers()` (preserves everything but its d0/d1 outputs), so a1 survives the call
   and d2/d3 are not even live in TestChildPart_Main → the save/restore is doubly dead.
   **The callee-preserves oracle's FIRST OPTIMIZATION use** — the t30 oracle (born to
   prove a CONTRACT true) here proves SAVES DEAD. −8 B.
2. **Dead caller-side `andi.w #$FF, d0`.** GetSineCosine's leading `andi.w #$FF` makes
   the caller mask redundant. −4 B. The `as Angle` bless re-homed onto `addq.b
   #CHILD_ORBIT_SPEED, d0 as Angle` (the byte add wraps mod 256 → the value that now
   flows to the call). Panel C2 independently re-derived the internal mask from math.emp:22.

**Byte accounting:** −0xC/shape at TEST_PARENT ($12C→$120). The whole downstream run
slid −0xC (TestStressEmitter/TestChurnObj + OBJDEFS/ACT_DESCRIPTOR/SONIC_ANIMS/
PARTICLE_ANIMS bases + every Pin after); ERROR_HANDLER + the ROM tail unmoved, the
trailing pad absorbs it → **ROM size constant**. Ripple (the row-1257 upstream-slide
sweep + 5-site doctrine, HAND-checked): ONE re-pin (7 regions + downstream Pins); 11
gate-arm resume orgs re-pasted (main.asm ×9, act_descriptor.asm ×2); 3 hardcoded
reference-ROM windows in `build_mixed_tranche4_rom` re-derived −0xC (particle_anims/
sonic_anims/act_descriptor heads — self-relative/pin-spliced CONTENTS unchanged, only
read offsets); the repin_pins comment. engine.inc arms unaffected (no engine region
slid). Mixed fns use `pins::` (auto). **$8000 bar:** N/A (the change SHRINKS test_parent;
object-bank plain/debug bases still coincide — TEST_STRESS_EMITTER plain==debug).

## Neither-bucket headlines

- **First byte-moving game-side wave** — the wave discipline's first live game-side run;
  the first canonical change since t24 (85111814/eb5e94be), sizes pad-absorbed.
- **The callee-preserves oracle's first optimization use** — contracts-true AND
  saves-dead; the oracle now underwrites a byte-changing cut.
- **CONFIRMED latent defect → VOLENCE OVERRIDE FLAG:** test_parent's timer-expiry
  self-destruct + DeleteChildren cascade is UNREACHABLE (the object cannot demo the
  behavior it is named for). `life_timer` doubles as the swing-phase counter; the
  left-swing branch resets it to PARENT_LIFETIME before the countdown reaches 0. Traced:
  180→120 (right) → x_dir=1 at 119 → 118→60 (left) → post-subq 59 resets to 180; never
  0. Faithful to both twins → pre-existing, byte-gate-invisible. **Overseer ruling:**
  comments corrected byte-neutrally (code authoritative for bytes); the behavior fix is
  a NAMED VOLENCE OVERRIDE (his test scene; a lifecycle change could silently invalidate
  ObjectTest soak determinism — the t24 never-freed-children pattern). Ledgered with the
  full trace.

## What each pass added

**Step 1 (demanded / neither-bucket):** SpawnDesc hoist; the two overlays; ZERO
externs (Draw_Sprite/DeleteObject/CreateChild_Normal/DeleteChildren/GetSineCosine
resolve module-to-module); first-linking-compile byte-identity both shapes. GAP: a bare
LOCAL `pub` label minus an extern in DATA position fails `[here.provisional]` — worked
around via `extern("TestChildPart") - extern("ObjCodeBase")` (forcing link-time
resolution of a same-module label; the emitters' cross-region form generalized).
Ledgered.

**Loop pass 1 — Step 2 (idioms/type-layer):** branch modernization byte-identical.
`Overlay.field(aN)` JOINS the step-2 idiom list (campaign-port-loop.md, the t29
feed-forward). **Type-layer event (MANDATORY):** GetSineCosine is the FIRST `.emp`
caller of a typed-`Angle`-param proc — the `slot_type_corpus` lint fired
`[call.slot-type-mismatch]` on the untyped d0, requiring the `as Angle` bless (the
type-layer walk is not always optional at a typed-param call site).

**Loop pass 1 — Step 3/4:** contract audit clean; no construct built (the SpawnDesc
hoist is the tranche's shared-record). Kill rows 67/68/69/65b same-tranche.

**Loop pass 1 — Step 5:** the byte-moving wave (both candidates, above). C1 named-site
re-answered on the slimmer orbit path: inactive (irreducible per-frame work + the table
lookup).

**Loop pass 2: DRY** — step-3 re-audit clean (contracts still exact post-wave), step-4
empty, step-5 empty. One panel round.

**Panel (A1 + B1 + C2; C1 flagged-call; C3 inactive — no VDP/DMA):**
- **C2 (correctness): CLEAN.** Both wave candidates verified independently from source
  (candidate 1 doubly-dead; candidate 2's internal mask confirmed at math.emp:22, bless
  correctly re-homed). All four procs' `clobbers` exact; a0/d7 preserved on every path;
  overlays non-colliding; SpawnDesc walk correct; CC/Bcc + stack balance clean.
- **B1 (corpus): nothing that reopens.** One flag-not-cut ledger candidate — twin-shared
  write-only `TOrbitChildV.phase_offset`/`radius` (faithful to the AS twin; deliberate
  orbit scaffolding). Everything else matches the shipped corpus.
- **A1 (cold-reader): two items → overseer rulings** (both adjudicated):
  1. the unreachable self-destruct (RULED — Volence override flag; comments corrected);
  2. the `table` sentinel-record-list adoption candidate (RULED — LEDGERED-stands:
     corpus-wide, overrides a same-day pre-ruling, §4.9 impl status unconfirmed;
     adjacency-critical comments added at the three sentinel sites).
  A1 finding 3 (extern-in-data vs bare-in-immediate asymmetry) reinforced the
  objroutine-constructor row with demand data; the moveq/clr.b tidy is sub-threshold.

## Corrections list

1. **SIGIL MASTER HASH (a two-time pattern):** brief §0 says `72baa7b`; the real master
   is `402b6db` (the brief commit itself). Branched off `402b6db` (dispatch had it right).
   Overseer amendment: henceforth briefs say "sigil master = this brief's commit"; the
   corrections row closes.
2. **Census §3a overlay-collision bound:** phrased `_len > SST_interact-SST_sst_custom`;
   `objvarsCheck` (macros.asm:60) tests `> SST_CUSTOM_SIZE`. Same mechanism; subsumed by
   the vars overflow check either way.
3. **Oracle consumer count did NOT grow:** brief §2 anticipated growth "IF test_parent's
   Main writes a0" — it does not; count stays 1 (TestChurnObj_Main).
4. **row 63 (vram_bytes) did NOT trip:** test_parent consumes `vram_art` (like G2), not
   `vram_bytes`.
5. **PROCESS — cross-worktree contamination (self-caught + fixed):** a missing `cd` on a
   ledger append+commit sent the panel-findings commit onto the **t32 branch**
   (violating "don't touch its work"). Caught via the log; extracted the exact content
   to tranche31 (committed there), `reset --soft`-removed the stray commit from t32,
   restoring its tip to the porter's `03f7b9d` with all four uncommitted files intact
   (verified: t32 ledger 0 of my content). cd-every-call re-earned; all subsequent
   commands use explicit `cd`.

## Step-6 sweep — the oracle-optimization census (overseer-ordered)

**The dead-save-around-a-preserving-callee class candidate 1 opened (ENUMERATE for a
future wave, not this tranche).** grep of every `-(sp)` push bracketing a `jbsr`/`jsr`
in the object/system `.emp` corpus, classified by the bracketed callee's preserve
contract:
- **DEAD (confirmed, taken this tranche):** test_parent TestChildPart_Main — `movem.l
  d2-d3/a1` around GetSineCosine (`clobbers()`). The SOLE confirmed dead instance.
- **NECESSARY (callee outputs/clobbers the saved reg) — the t24-named "6 alloc sites"
  are LIVE:** children.emp `move.l a1,-(sp)` around AllocDynamic/AllocEffect at
  CreateChild_Normal:154, CreateChild_Complex:265, CreateChild_FlipAware:347,
  CreateEffect_Normal:575 — AllocDynamic `clobbers(d0) out(a1 if eq) preserves(a0)` /
  AllocEffect `clobbers(d0) out(a1 if eq)`: a1 IS the alloc's OUTPUT, so the cursor save
  is REQUIRED (the t24 note flagged the right shape; the contract makes them live).
- **NECESSARY (other):** sprites.emp:399 (a0 around Emit_ObjectPieces — a0 in its
  `clobbers`); core.emp:529 (a2 around `jsr (a1) as ObjRoutine` — ObjRoutine preserves
  only a0/d7); animate.emp:219 (a1 around `jsr (a2) as AnimCallback` — empty/unknown
  target contract, defensive).
- **STRUCTURAL park (not a single-call bracket):** sprites.emp:350 (a2 pushed :350,
  popped :408 — the band-pointer parked across the whole multi-sprite walk, not one call).
**Outcome:** the class is SPARSE — candidate 1 is the only confirmed dead site; the alloc
sites are live (a1 is the output). No future wave is seeded with additional confirmed
sites, but the enumeration is on record. A dead save only appears where a caller
over-conservatively brackets a FULLY-preserving leaf (the GetSineCosine shape).

## Kill-list rows (added same-tranche)

Rows 67 (test_parent gate-off body + org arm), 68 (TParentV/TOrbitChildV overlays,
zero drift guards), 69 (SpawnDesc + its AS twins across THREE files — one row, full
member list), 65b (VRAM_TEST_OBJ 4th mirror); row 66's EffectSpawn1 reference retired
into row 69.

## Test artifacts (green by name, SIGIL_STRICT_GATE=1, AEON_DIR at the aeon worktree)

Windowed: `test_g3_objects_port::{g3_objects_regions_match_reference,
g3_objects_debug_regions_match_reference, g3_undoctored_compile_equals_the_reference_window,
g3_doctored_reference_diverges}`. Whole-ROM:
`mixed_dac_rom::{mixed_tranche31_rom_matches_assembled_reference,
mixed_tranche31_debug_rom_matches_assembled_reference}`. Canary: G2
(`test_g2_objects_port` + `mixed_tranche30`) green post-hoist. Slot lint:
`slot_type_corpus::retrofitted_corpus_has_zero_slot_mismatches`. Full strict **2763/0
(1 ignored)**.

## Census STATUS AMENDMENT (folded into 2026-07-29-game-side-census.md)

G3 PORTED (test_parent.emp; byte-moving wave −12 B/shape, new canonical
85111814/eb5e94be). Oracle consumer count UNCHANGED at 1 (Mains don't write a0); the
oracle gained its first OPTIMIZATION use. The census's G3-first overlay prediction was
already corrected by t29 (test_animated is the debut). Next: the P1 player keystone arc.
