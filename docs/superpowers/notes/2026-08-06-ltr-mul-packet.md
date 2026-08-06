# 2026-08-06 — the byte-changing multiply parcel packet (`ltr-mul`)

Lane: `ltr-mul` pair (sigil off `4cbb2463`, rebased onto `686e6f62` · aeon off
`4974bf3`, rebased onto `76013f2`). Porter parcel; no merge-state claims. Base
documents: specs/2026-08-06-byte-changing-mul-parcel.md (incl. its §6
amendment), the 2026-08-05 `mul_const.w`/`mul_bounded.w` design, and
notes/2026-08-05-mul-w-packet.md. This packet states only what differs.

This is the corpus's first deliberate byte-MOVING optimization pass since
conversion. The byte gate is an AUDIT, not identity: every delta is named
below, and behavioural identity is the overseer's emulator A/B, not a porter
claim. The A/B **passed** — evidence and its coverage limits in
`2026-08-06-ltr-mul-ab.md` (the provenance `ab` ref for chain entry 49),
summarised in §4a.

## 1 · What moved

R1 relaxed the word LTR arm's gate from `>= 3` set bits to `>= 2`, so the LTR
chain competes with the two-power arm at every corpus stride and `choose()`
takes it on cycles (32/32/34 versus 34/40/44). The suppression the mul-w round
shipped — deliberate, to hold byte-identity — is retired; its step-5 finding
and the design §3 paragraph both carry "taken by this parcel" pointers now.

R3 shed the word loop's `moveq #0` multiplicand seed. That zero was dead under
the word contract (the `add.w` body reads only `s`'s low word), so the loop
prices 24 + 14·M instead of 28 + 14·M and its mulu boundary moves from M = 2 to
M = 3 (mulu at M ≥ 4). Row 2165 predicted exactly this; the prediction was
confirmed and taken.

Nine sites moved, two are refused-with-reason, and **three were missed by the
census entirely** (§7 step-5 item 0 — the panel's catches; spec §7 then §8
withdraw R2's "every multiply is construct-spelled" / "no opportunistic adoption
debt survives" claims as FALSE). The accurate end state: **9 moved, 2
refused-with-reason, 3 missed-and-now-ledgered, 1 legitimate-non-adoption
recorded** — a census of 11 sites, not 7, and not a completed sweep.

| Group | Site | Chosen lowering | Δcy | Δsize |
|---|---|---|---|---|
| A | section:154 ×66 | LTR `move.w/lsl.w#5/add.w/lsl.w#1` | −2 | 0 |
| A | tile_cache:229 ×66 | LTR | −2 | 0 |
| A | tile_cache:77 ×80 (comptime fn, 2 splices) | LTR `…#2/add.w/…#4` | −8 ×2 | 0 |
| A | plane_buffer:76 ×160 | LTR `…#2/add.w/…#5` | −10 | 0 |
| D | plane_buffer:245 ×80 | LTR (in-place) | −8 | 0 |
| D | section:280 ×160 | LTR (in-place) | −10 | 0 |
| C | tile_cache:1355 ×80 | LTR (in-place; copy-back shed) | −4 | −2 B |
| B | section `.gxy_mul` | **`mulu.w d1,d0`** | ≈−600 worst | −16 B, −2 labels |
| B | tile_cache `.mul_loop` | **`mulu.w d1,d3`** | ≈−600 worst | −14 B, −2 labels |

Group A needed no `.emp` edit — those sites already spelled `mul_const.w`, and
R1 alone re-derived their lowering. That is the construct paying off: a
generator ruling improved five emission sites with zero source churn.

## 2 · R4: the model chose `mulu` at both loop sites

Both loops compute `sec_y × grid_w`, and both are guarded so `sec_y < grid_h`
where the build asserts `GRID_W * GRID_H <= MAX_ACT_SECTIONS` (48). 48 is the
honest bound, derived from the module `ensure`, and it sits far past the M ≥ 4
boundary — so `mulu` wins on worst case, with or without the R3 seed shed. This
is the R4 "model-decides" exit working as designed, not a failure: the hand
loop is cheaper for small `sec_y`, the construct budgets the worst case, and
both sites are documented cold paths. That worst-case-vs-typical fact is stated
once at each site, per spec §5.

`Section_GetSecPtrXY` used the no-scratch spelling: it has no free scratch (d2
holds live `sec_x`), and the loop candidate is dominated at this bound anyway,
so the no-scratch form is the honest spelling rather than an invented register.

## 3 · The delta audit (seven targets)

Anchor SIZE is unchanged on every target — the ~32 B code shrink is absorbed by
padding before the fixed `EndOfRom` — while anchor CONTENT moved. Full files
shrink 99-102 B on the appendix-bearing shapes (per-target below) because Group B removed four deb2
labels (`.gxy_mul`, `.gxy_add_x`, `.mul_loop`, `.mul_test`); `lean` carries no
appendix and is size-stable, content-only.

| target | full (golden → new) | anchor CRC (golden → new) | anchor size |
|---|---|---|---|
| s4 | `b5ffb094`/411267 → `3b6cad91`/411167 | `b5be8fef` → `09beac0f` | 383336 (stable) |
| s4.debug | `57fd08f9`/423671 → `e3963874`/423571 | `8448717a` → `2263979c` | 391000 (stable) |
| demo | `cbddc142`/91429 → `b8df1c2b`/91330 | `1a72e3e0` → `e8988a2f` | 70180 (stable) |
| demo.debug | `b61f462d`/94133 → `30173928`/94031 | `efd16be5` → `3b5b11e9` | 70180 (stable) |
| config_a | `61e4e78e`/424049 → `7660f157`/423949 | `bf1dde89` → `1b3b3708` | 391000 (stable) |
| config_b | `07e3f465`/301305 → `ace527ba`/301205 | `db32d41b` → `e6e20c75` | 273808 (stable) |
| lean | `b92cb485`/379110 → `69c20328`/379110 | `d32cee18` → `cd73fb65` | 379110 (stable) |

**Where the shift lands, exactly — PER SHAPE.** From the committed old size
tables against a fresh derivation. The plain shape moves **nine** boundary
symbols, all inside one band; the debug-family shapes move **eleven**, the two
extra being debug-only symbols that sit inside the same band
(`CompressionSelfTest` 0x7548 → 0x7528, `CSelf_S4LZ_Plain` 0x7760 → 0x7740, both
−0x20). The distinction matters here rather than being pedantry: the debug shape
is precisely the one whose RAM delta the moved-pointer refutation defends, so
its own ladder is what has to be quoted.

Plain shape (debug addresses differ; the deltas are identical):

```
Collision_GetType    0x5260 -> 0x5250   -0x10
Collision_ProbeDown  0x52d0 -> 0x52c0   -0x10
Section_Init         0x57c4 -> 0x57b4   -0x10
Camera_Init          0x5c00 -> 0x5be0   -0x20
Parallax_Init        0x5db0 -> 0x5d90   -0x20
Art_Decompress       0x638a -> 0x636a   -0x20
BG_Init              0x63ee -> 0x63ce   -0x20
BgAnim_Init          0x64d0 -> 0x64b0   -0x20
Sound_PostByte       0x656e -> 0x654e   -0x20
```

Everything else is byte-stationary — including every object, art, mapping and
level-data symbol (`ObjCodeBase` 0x10000, `Map_TestObj`, `Ani_Sonic`,
`ObjDef_*`, `PState_*`, `DeformTable_Zero`, `HeightMaps`, `OJZ_*`), and every
code symbol below `Tile_Cache_GetTile` 0x4580 (`Load_Object`, `TouchResponse`,
`RingBuffer_Add`, `VBlank_Handler`, `GameLoop`, `InitObjectRAM`). `EndOfRom` is
unchanged. **This is load-bearing for divergence attribution: no ROM pointer to
object/art/mapping data moved.** The only ROM addresses that moved are engine
code entry points in `[0x4580, 0x656E)`.

**Scratch-liveness audit (the parcel's real behavioural risk class).** The LTR
chain leaves the scratch register holding a *different* final value than the
two-power chain did (for ×80: `x` rather than `x << 4`). Every adopted site was
audited for a downstream read of its scratch before the next write:

| site | scratch | next touch after the site | verdict |
|---|---|---|---|
| section:154 | d1 | none (proc ends) | dead |
| tile_cache:229 | d4 | `move.w Sec.sec_block_dict_len(a1), d4` (write) | dead |
| tile_cache:100 (`mul_cache_stride`) | d2 | `move.w (a0,d1.w), d2` (write) | dead |
| tile_cache:1360 (`mul_cache_stride`) | d3 | `move.w d4, d3` (write) | dead |
| plane_buffer:76 | d2 | `move.w #TILE_CACHE_ROWS, d2` (write) | dead |
| plane_buffer:245 | d3 | `move.w Cache_Origin_Col, d3` (write) | dead |
| section:280 | d2 | `move.w d6, d2` (write) | dead |
| tile_cache:1355 | d4 | `move.w d5, d4` (write) | dead |

No site depends on a scratch's final value. The `.fr_no_coll` even-row path
skips the multiply in both the old and new spellings, so d4's carry-through
there is unchanged.

**The Group C tie-break is unobservable, which is why nothing pins it.** Spec §3
directed the porter to pin the tie-break where costs tie at ×80. After R1 there
is no tie to pin: 5c strictly dominates 5a (equal only at b = 0, where the two
arms emit the IDENTICAL byte sequence, and cheaper for every b ≥ 1), so no
`choose()` outcome — and therefore no emitted byte — can depend on how a 5a/5c
tie is broken. `word_two_power_arm_never_wins` sweeps every 2-set-bit n in
`0..=$FFFF` and pins exactly that. The directive is answered by proving it
vacuous rather than left silently unfulfilled.

## 4a · The behavioural bar (overseer-run), stated with its limits

- **PLAIN shape** — the shipping artifact, and what the owner play-tests:
  **byte-identical in work RAM, VDP (VRAM/CRAM/VSRAM/register file) and
  framebuffer** at an in-level anchor (RAM64k `3845DADA`, VDP combined
  `4E59D92B7D06E719`, framebuffer `CED4B186836B7421` — same both ROMs). A
  screenshot confirms full zone art with the player rendered, so the tile cache,
  section lookups and plane buffer — every changed multiply site — are genuinely
  exercised.
- **DEBUG shape** — VDP bit-identical at all three checkpoints (frozen scene
  `0A39113FDC4C3CB6`, 6-step camera sweep driving cache fills
  `6C9E7D7099782FE4`, 600-frame ObjectTest churn soak `4AC6459142D74AE6`), lag
  frames identical at every anchor. Work RAM differs **only** as dead scratch
  residue in dead stack, attributed three independent ways: `BISECT1` (zero
  relocation, same delta, identical VDP — kills the return-address theory), the
  static liveness audit in §3, and the plain shape's byte-identical RAM (the
  delta appears exactly where debug instrumentation spills more registers to
  stack, and nowhere else).
- **LAG FRAMES ARE IDENTICAL, NOT REDUCED** (26/40/52/52, both ROMs). The cycle
  wins are real at the sites but land well inside the frame budget on these
  scenes and do **not** convert into recovered lag frames. **No lag win is
  claimed.** The honest result: the win is measured in cycles at the sites
  (−2/−2/−8/−8/−10/−10/−4 plus Group B's ≈−600 worst case on the cold paths), no
  scene regressed, and no lag frame was recovered on the scenes exercised.
- **Not run**: a live A/B of `BISECT2` (Groups A+C+D, also zero-relocation).
  Those sites rest on the byte-delta audit, the static liveness audit and the
  full-parcel identity above, not on a dedicated live control.

## 4b · What the review panel established (stronger than the porter's own claims)

Recording this because in two places the panel proved something the packet had
only asserted, and in one place it proved the packet's caution unnecessary.

- **The LTR arithmetic is PROVEN, not merely pinned.** All three multipliers
  re-derived independently (×66: x→32x→33x→66x; ×80 and ×160: x→4x→5x→80x/160x),
  with an airtight mod-2^16 argument: every emitted op is `.w`, and `.w` ops ARE
  the mod-2^16 ring operations, so an overflowing intermediate cannot corrupt
  the low word for ANY x. The equivalence oracle executes each chosen lowering
  through an ISA-faithful interpreter written independently of the chain
  constructor, so the proof is not circular.
- **The Group B upper-word hazard is REFUTED BY PROOF, not by scene luck** —
  materially stronger than this packet's "undefined but unread" framing. Both
  sites are guarded (`sec_y < grid_h`), so the product is
  `< grid_w·grid_h ≤ MAX_ACT_SECTIONS = 48`; a product under 2^16 leaves the
  upper word ZERO — exactly what `moveq #0` + `add.w` produced before. A nonzero
  upper word would require `grid_w·grid_h ≥ 65536`, barred by the standing
  `MAX_ACT_SECTIONS ≤ 496` assert. **That holds for every future act, not just
  today's.** Downstream was traced to the next full write at both sites anyway:
  no `.l` read, no `movea.l`, no index-as-long, no `swap`, no long compare.
  Condition codes: the next instruction at each site is itself a flag-setter,
  and the Group C/D chains whose last op changed reach no branch without an
  intervening setter.
- **Scratch liveness re-derived independently at 8/8 sites**, including branch
  targets, the DEBUG assert block and the `.fr_no_coll` join — with the
  structural clincher this packet's table missed: the pre-parcel code already
  destroyed the same registers, so liveness is *unchanged* and only the residual
  VALUE differs.
- **The D1c row removal is genuine**, verified from `closure.rs`'s definition
  (effective sets are BODY-derived, not declaration-derived) rather than from
  the packet's prose.
- **The refusals are correct**: d2 is live at the cited downstream reads.
- On the ripple: all 14 provenance CRCs were independently recomputed, the
  region ladder re-derived from PLANE_BUFFER down to SOUND_API with the
  −0x10/−0x10/−0x20 arithmetic reconciled against the per-site byte table, and a
  negative sweep run for all 21 pre-parcel addresses in the moved band across
  every `.rs`/`.toml`/`.sh` in the tree — zero stale hits. The 5-site ripple is
  complete.

## 4 · Bars

- **Full strict** (`--no-fail-fast`, `SIGIL_STRICT_GATE=1`, AEON_DIR = this
  lane's aeon), **after the refreeze and the final rebase onto the Track C
  masters**: **3476 passed / 0 failed / 4 ignored = 3480**, and the branch's own
  `#[test]` total is **3480** — closes exactly. (The total moved twice from
  master, never from this parcel: 3452 at the refreeze, 3455 with `srmask`'s
  `preserve_oracle_threading.rs`, 3480 with Track C's suites.) Before the
  refreeze the run was 3425/23/4: those 23 were entirely the
  stale-golden / frozen-reference class a byte-changing parcel produces
  (`*_anchor_matches_golden`, `*_full_file`, `*_size_table_rederives_native`,
  `native_full_sonic4_*`, `boot_*_region_matches_reference`,
  `config_b_frozen_placement_exact`, `config_b_doctored_size_table_breaks_the_build`
  via its undoctored control, `deform_pointer_equals_placed_label_vma` via its
  frozen-placement sanity gate, `flipped_config_a_anchor_matches_golden`). All
  23 cleared on the refreeze; none needed a hand fix, which is itself the
  evidence that they were reference staleness and not behaviour.
- **Warn tiers ×7:** the firing lint-ID SET is identical across all seven
  targets (`module.path-mismatch`, `proc.undeclared-fallthrough`,
  `proc.out-unwritten`, `proc.clobber-undeclared`) — re-verified on the Track C
  base, where the three new `option.*` ids fire **zero** times, so the set is
  unchanged (checked, not assumed: `[option.raw-sentinel]` is planned
  zero-firing and is). Counts are 19 on the
  plain-family shapes (s4, demo, config_b: 9/6/3/1) and 18 on the
  debug-family shapes (s4.debug, demo.debug, config_a, lean: 9/5/3/1) — the
  pre-parcel baseline exactly. No deliberate lint delta in this parcel and none
  introduced; the adoptions add no warning (every proc already declared its
  scratch in `clobbers`).
- **Refreeze:** chain entry **49** (`ltr-mul`), appended by `refreeze --freeze`
  with a real `--ab` ref (`docs/superpowers/notes/2026-08-06-ltr-mul-ab.md`) —
  mandatory here because all seven anchors moved. The one command ran
  `capture_goldens.sh --write`, `derive_offcanonical_sizes.sh` and `repin` in
  order, then appended the entry; the seven frozen CRCs equal the porter's
  earlier independent capture exactly. **`refreeze --check`: OK (tip `ltr-mul`,
  chain len 49).** Canonical ROMs rebuilt after capture, one shape per
  invocation, and both verified against the newly frozen blobs (`3b6cad91` /
  `e3963874`).
- **repin:** `pins.rs unchanged` after regeneration. The post-flip ripple is
  pins.rs (auto) plus the hermetic `repin_pins.rs` SOUND_API base literals
  (hand, −0x20 both shapes); `engine.inc` and `mixed_dac_rom.rs` do not exist in
  this tree, and no region was added, so `repin.toml` is untouched. Z80 stays
  byte-neutral (no Z80 source moved).
- **`corpus_bytediff.sh`:** `RESULT: all identical` — and **inert for this
  parcel**: `examples/` contains no multiply construct at all, so the probe
  cannot exercise a single changed path. It is reported for completeness, not as
  evidence. The whole-ROM audit above is the evidence, exactly as bar #1 says.
- **Gate probes — and an earlier "both polarities" claim CORRECTED.** The
  original 1-bit half of this probe was VACUOUS and is replaced: a 1-bit
  multiplier cannot discriminate 5c's `>= 2` threshold, because arm 5's outer
  guard already excludes it and, even with both guards removed, 5c would lose to
  the plain shift arm (22 cy / 4 B against 18 / 2) — so `choose()` returns
  `lsl.w` whatever the gate says, and mutating the gate left the test green.
  `word_gate_boundary_generates_and_selects_the_ltr_arm` now probes the
  CANDIDATE SET, where the gate is decidable: at 2 set bits the LTR arm must be
  GENERATED (a `>= 3` regression removes it, the two-power arm wins by default,
  and every corpus stride changes bytes) and must then win `choose()`. Its 1-bit
  assertions are kept but re-labelled for what they honestly pin — arm-5 gating
  in general, i.e. no chain candidate touches the scratch register even when one
  is offered. `word_two_power_arm_never_wins` additionally sweeps every 2-set-bit
  n in `0..=$FFFF`. The unit oracle
  executes every chosen lowering with garbage upper words, and
  `word_bounded_semantics_and_boundary` sweeps every in-bound src and pins the
  moved M = 3 / M = 4 boundary and the seedless loop shape.

## 5 · A soundness win the parcel earned as a side effect

`Section_GetSecPtrXY` no longer needs d2 as a multiply counter, so it simply
does not touch d2 — and the D1c `[call.live-clobbered]` firing
`(Parallax_CheckBoundary, Section_GetSecPtrXY, d2)` left the baseline.

This is worth foregrounding because of *why* the row existed. The old body did
preserve d2's value (it stack-saved and restored `sec_x`), but the save was
conditional — inside the `sec_y != 0` arm — and the verifier does not recognise
a conditional individual save/restore as a preserve. So the contract had to
declare d2 clobbered, and a real caller was flagged for holding a value across a
call that the analysis believed destroyed it. Adopting the construct deleted the
counter, which deleted the conditional save, which deleted the imprecision, which
deleted the firing. The hazard row is gone because the hazard's *cause* is gone,
not because a baseline was edited to be quiet — the D1c gate has teeth precisely
so that a narrowing like this must be adjudicated, and this one was.

The proc's declared `clobbers(d1-d2)` deliberately still names d2: the
declaration is a uniform interface promise (matching `Section_FlatIDXY`), not a
description of the body. That is a contract choice, and the site comment now says
so in present tense.

## 6 · Ledger and docs

- Row 2165 **CLOSED** with the R4 outcome (both sites adopted, model chose
  `mulu`, bound `MAX_ACT_SECTIONS` = 48; the dead-seed prediction confirmed and
  taken; the D1c side effect recorded).
- Rows recorded for the two refusals and the follow-up work: the
  `TileCache_CopyBlockColumn` ×80 pair **CLOSED-AS-RULED** (the reason named, and
  anchored on symbol/block names rather than line numbers, since the row exists
  to stop a FUTURE sweep and line numbers drift), the `parallax.emp` ×10
  **CLOSED-AS-RULED** legitimate non-adoption, and **OPENED**: the step-3
  three-address ask, the three census misses, the dominated candidate 5a, the
  `shift_run_word` k = 1 pessimisation, and the `Section_GetSecPtrXY`
  over-declaration.
- The mul-w packet's step-5 item 3 and the mul-const-w design §3 paragraph both
  carry "taken by this parcel" pointers.
- No row opened for the LTR suppression — it died here, as the spec directed.

## 7 · Step-3 vs step-5 findings

**Step-3 (language asks).**

1. **Three-address `mul_const.w dDst, dSrc, #n, dScratch`** (dst ≠ src, src
   PRESERVED) — the parcel's one real language gap, and it is measured, not
   speculative. `tile_cache.emp:326-329` and `:374-377` compute `d5 = d2 × 80`
   with d2 live afterwards (`:373`, `:393`). `mul_const.w` multiplies in place,
   so adoption needs a preload (+4 cy / +2 B) or the no-scratch form (+18 cy) —
   both regress a hot tile-cache path, so spec §6 refused adoption at both. A
   three-address form would lower to the corpus's own single-temp idiom
   (`move.w src,dst / lsl / add.w src,dst / lsl`) — exactly the bytes those
   sites already carry — making them byte-neutral adoptions. The interesting
   part is the contract, not the operand count: the fourth operand's role flips
   from "may come back clobbered" to "is the preserved source", so it needs its
   own surface name rather than a positional overload.
2. The `shl_l` companion and the DEBUG `assert src ≤ M` mechanism remain open
   and untouched (rows 2004 / 2166). `mul_bounded`'s bound is still trusted;
   this parcel is its first real adopter, which is exactly the kill condition
   row 2166 names — worth noting that the adopters landed on `mulu`, whose cost
   is bound-independent, so the untrusted-bound exposure did not grow.

**Step-5 (engine findings, not taken).**

0. **THE CENSUS WAS WRONG THREE TIMES, and the hottest ×80 stride in the engine
   was never in it.** The spec's §3 census said "exactly seven un-adopted
   sites"; there are eleven. All three misses were verified against the tree
   before being recorded:

   - **`collision_lookup.emp`, `Collision_GetType`** — a two-power ×80 word
     chain with a separate scratch, the exact "form (a)" shape Groups A/D
     adopt, in the player-sensor collision probe called per sensor per frame.
     **Hotter than every site this parcel touched.** A pure drop-in
     (`mul_const.w d1, #80, d2`): 40 → 32 cy, 8 B → 8 B, zero ripple, d2
     already licensed and verifiably dead after. It was NAMED in the corpus's
     own `mul_cache_stride` comment and still fell out of the census.
   - **`entity_window.emp`, `EntityWindow_MigrateMasks`** — a ×22 chain (three
     set bits), adoptable even under the OLD ≥ 3 gate, so the mul-w round
     missed it too.
   - **`Section_FlatIDXY`'s `.fxy_mul`** — a third repeated-add loop, same
     `sec_y × grid_w` family as Group B, four live callers, accumulating
     directly from memory so the construct needs a preload.

   Also now recorded: **`parallax.emp`'s ×10 is a LEGITIMATE non-adoption**
   (hand 28 cy vs 30 with the preload the live source forces) — correct as
   written, and previously not recorded as a decision at all.

   None is adopted tonight: each is byte-changing and demands a fresh refreeze,
   A/B and panel — the collision site most of all, precisely because it is the
   hottest and so the one whose verification must not be rushed. They are the
   highest-value follow-up parcel in the queue, with their numbers measured.

   **Bar #3, answered honestly.** The bar said the tile-cache hot-path win must
   show up or the packet must say why not. The wins landed at the sites the
   census named — and the hottest ×80 stride in the engine was never in the
   census. That is a **census failure, not a measurement failure**, and it is
   also why lag frames did not move (§4a): the parcel optimized real code that
   simply was not the bottleneck.

   **The process lesson, which is the more valuable half.** The porter brief's
   rule is to verify each spec claim against the current tree before building
   it. I verified that every site the census *named* existed and was as
   described — and that is exactly the check that cannot catch this class. A
   completeness claim ("exactly seven") is only testable by re-running the
   census independently, which I did not do until the panel forced it, and even
   then my re-census found only the loop-shaped miss because I searched for
   `dbf` accumulate loops and not for two-power shift chains. **Verifying the
   members of a set is not verifying the set, and re-running a census with the
   same mental template as the original reproduces the original's blind spot.**
   Every false completeness claim is withdrawn in the spec, the ledger and here
   rather than quietly narrowed.

1. The two refused ×80 sites (§7 step-3 item 1) are the headline non-adoption.
   They are recorded as *optimal*, not as debt: `((x<<2)+x)<<4` is the right
   code for a preserved source, and the construct surface — not the engine — is
   what lacks the spelling.
2. `mul_cache_stride`'s comptime fn is now the only remaining ×80 abstraction,
   and it covers exactly the two "form (a)" sites. The two refused sites are its
   "form (b)" cousins. If the three-address form ships, the natural follow-up is
   a second comptime fn (or a widened `mul_cache_stride`) so all four ×80 sites
   name the stride constant rather than open-coding 80.
3. `Section_GetSecPtrXY`'s `clobbers(d1-d2)` now over-declares (the body never
   touches d2) — and this is NOT the "no demand today" item an earlier draft
   called it. **A live caller already depends on the undeclared preservation:**
   `parallax.emp`'s `Parallax_CheckBoundary` holds a value in d2 across
   `jbsr Section_GetSecPtrXY` and reads it afterwards, so it is correct only
   because the body preserves d2 while the contract explicitly disclaims it.
   Narrowing to `clobbers(d1)` would make the contract match reality and
   legalise what the caller already does. Not taken here because a byte-changing
   parcel should not also move a public contract, but ledgered with urgency it
   did not previously have — the parcel deleted the only MECHANICAL record of the
   dependency when the D1c firing for that exact triple left the baseline.
4. Both Group B sites are cold paths (stated at each) and both now pay a flat
   70 rather than a data-dependent cost. Nothing measured suggests they were
   hot; if a future profile disagrees, the honest fix is a tighter bound — but
   **the tightening differs per site, because the two sites bound DIFFERENT
   operands**. `mul_bounded.w` constrains the COUNTER: at `tile_cache.emp` the
   counter is `sec_y`, so the act's real grid HEIGHT is the right bound; at
   `section.emp` the operands are reversed and the counter is `grid_w`, so grid
   height bounds nothing there and the right bound is the act's real grid WIDTH.
   An earlier draft prescribed grid height for both, which is wrong for the
   section site.

**Neither-bucket headlines.**

- **A generator ruling improved five emission sites with zero source churn.**
  Group A needed no `.emp` edit at all; relaxing one gate re-derived every
  already-adopted stride. That is the payoff the construct was bought for, and
  it only shows up on a parcel that is allowed to move bytes — which is why the
  mul-w round's deliberate suppression was the right call *then* and the wrong
  state to leave standing.
- **The layout shift is Group B's alone; the other groups are size-stable.**
  Bisect builds: R1-only (Group A, no `.emp` edits) is 411267 bytes and is
  genuinely ZERO-relocation — its deb2 appendix is byte-identical to the golden,
  so every symbol name and address is unchanged. A+C+D is *also* 411267 bytes,
  but that is size-stable, NOT zero-relocation: Group C-1355 sheds 2 B and that
  shift IS realized in the band symbols, with alignment padding absorbing it
  before the total. Only Group B — the two hand loops and their four labels —
  moves the file size and shifts everything downstream by −0x20.
- **Cost-table granularity is doing real work again.** The ×160 site sat at 44
  vs mulu's 46 under the two-power arm and now sits at 34; the ×66 site moved 34
  → 32. A ±2 correction on `move`/`lsl`/`add` no longer flips chain↔mulu at any
  stride, so this parcel has *widened* the margin the mul-w packet flagged as
  uncomfortably thin. The loop sites moved the other way — they are now decided
  by a bound (48) that is an order of magnitude past the boundary (4), so they
  are insensitive to table drift too. Both families are further from their
  decision boundaries than they were yesterday.

## 8 · Commits

- sigil `ltr-mul` (rebased onto `686e6f62`): the generator behaviour (R1 gate +
  R3 seed shed + comment rewrites + test re-pins + the new boundary probe), the
  ripple (repin'd `pins.rs`, the `repin_pins.rs` SOUND_API literals, the D1c
  baseline row), and the ledger/doc actions + this packet.
- aeon `ltr-mul` (rebased onto `76013f2`): the Group B/C/D adoptions, and the
  worst-case-contract comments at both loop sites (byte-neutral, verified).

Behaviour, ripple, docs and the refreeze are separate commits.

**Rebases.** The branches were rebased three times mid-parcel as masters moved,
and every gate was re-proven own-run after each — the frozen goldens make this
mandatory, because a byte-moving change on master would silently stale this
parcel's refreeze and leave the `--ab` ref describing the wrong delta.

The final base is sigil `a630fd3f` / aeon `ad4c6ef` (the **Track C**
niche-sentinel Option merge), which is the demanding one: it changed the aeon
**corpus** (`types.emp`, `objects/core.emp`, `entity_window.emp`, `rings.emp`,
`sst.emp`, `constants.emp`) as well as the sigil frontend. All **seven** targets
were rebuilt and reproduce their frozen chain-49 CRCs exactly — `3b6cad91`,
`e3963874`, `b8df1c2b`, `30173928`, `7660f157`, `ace527ba`, `69c20328`, with
every anchor CRC and anchor size unchanged too — so Track C is byte-neutral
against this parcel and the refreeze stands unmodified. Nothing was re-frozen on
top of it.

Two adjacencies resolved rather than assumed away: Track C's `assume_some`
zero-cost arm lands in the same `m68k_cycles.rs`/`z80_cycles.rs` pre-table match
block this parcel's generator prices through — both arms survive and the mul
cost decisions are unchanged — and the gap-ledger conflicted as a pure
append-vs-append, resolved keeping both lanes' rows. The earlier `srmask` base
was likewise verified byte-neutral.

## 9 · Traps recorded (they cost two full A/B passes)

Both are emulator-harness traps, both are campaign-general, and neither is
specific to this parcel — but a cycle-changing parcel is exactly what exposes
the first one.

1. **An A/B anchored on `pause` is INVALID for a cycle-changing parcel.**
   `pause` lands at an arbitrary intra-frame instant whose phase differs between
   the two ROMs *precisely because* the code timing changed. A poke applied
   there lands at a different point within the frame, and the scene then
   diverges from the harness's own stimulus — a self-inflicted difference that
   looks exactly like a behavioural regression. Anchor on a deterministic PC
   (`run_to VInt_Level`) before poking. This produced, and then retracted, a
   false "ObjectTest churn ends with different object positions" finding.
2. **`reload_rom`'s diagnostic can lie.** It reported "reload was silently
   rejected" (pointer unchanged) when the load had in fact succeeded. Verify the
   cart by hashing it (`memory_hash addr 0 len <filesize>` against the expected
   CRC32), never by trusting the reload diagnostic.

## 10 · State

The refreeze is DONE (chain entry 49, §4), the post-refreeze strict run is green
with closing arithmetic, and `refreeze --check` passes. The branches are
UNMERGED and carry no merge-state claim; the panel and the merge are the
overseer's.
