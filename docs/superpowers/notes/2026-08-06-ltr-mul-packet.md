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

Nine sites moved, two are refused-with-reason:

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
generator ruling improved six emission sites with zero source churn.

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
shrink 100 B on the appendix-bearing shapes because Group B removed four deb2
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

**Where the shift lands, exactly.** From the committed old size tables against a
fresh derivation, only nine boundary symbols moved, all inside one band:

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

## 4 · Bars

- **Full strict** (`--no-fail-fast`, `SIGIL_STRICT_GATE=1`, AEON_DIR = this
  lane's aeon), **after the refreeze**: **3448 passed / 0 failed / 4 ignored =
  3452**, and the branch's own `#[test]` total is **3452** — closes exactly.
  Before the refreeze the same run was 3425/23/4: those 23 were entirely the
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
  `proc.out-unwritten`, `proc.clobber-undeclared`), with counts 19 on the
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
- **`corpus_bytediff.sh`:** `RESULT: all identical`. Honest reading: the example
  corpus contains no 2-bit `mul_const.w`/`mul_bounded.w`, so this probe does not
  exercise the changed path. The whole-ROM audit above is the real evidence —
  the probe is sigil-only and example-scoped, exactly as bar #1 says.
- **Negative probes, both polarities, on the new gate boundary:**
  `word_gate_boundary_two_bits_chain_one_bit_shift` pins that a 2-bit multiplier
  takes the scratch chain (LTR by cost) and a 1-bit multiplier still takes the
  scratch-free `lsl.w` arm even when a scratch is offered. The unit oracle
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
- Two rows **OPENED**: the `tile_cache.emp:326-329` / `:374-377` non-adoptions
  (closed-as-ruled, with the reason named so no sweep re-proposes them), and the
  step-3 language ask for the three-address form.
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

1. The two refused ×80 sites (§7 step-3 item 1) are the headline non-adoption.
   They are recorded as *optimal*, not as debt: `((x<<2)+x)<<4` is the right
   code for a preserved source, and the construct surface — not the engine — is
   what lacks the spelling.
2. `mul_cache_stride`'s comptime fn is now the only remaining ×80 abstraction,
   and it covers exactly the two "form (a)" sites. The two refused sites are its
   "form (b)" cousins. If the three-address form ships, the natural follow-up is
   a second comptime fn (or a widened `mul_cache_stride`) so all four ×80 sites
   name the stride constant rather than open-coding 80.
3. `Section_GetSecPtrXY`'s `clobbers(d1-d2)` now over-declares (the body
   preserves d2). Narrowing it to `clobbers(d1)` would be honest and would let
   callers carry a typed `GridX` in d2 across the call — but it is a contract
   change with its own caller-side ripple and no demand today, so it is left
   alone and the comment explains the choice. Deliberately not taken here: a
   byte-changing parcel should not also move a public contract.
4. Both Group B sites were labelled cold paths in the source and both now pay a
   flat 70 rather than a data-dependent cost. Nothing measured suggests they
   were hot; if a future profile disagrees, the honest fix is a bound tightened
   to the *act's* real grid height rather than the global ceiling — which would
   put the loop back in play at M ≤ 3.

**Neither-bucket headlines.**

- **A generator ruling improved six emission sites with zero source churn.**
  Group A needed no `.emp` edit at all; relaxing one gate re-derived every
  already-adopted stride. That is the payoff the construct was bought for, and
  it only shows up on a parcel that is allowed to move bytes — which is why the
  mul-w round's deliberate suppression was the right call *then* and the wrong
  state to leave standing.
- **The entire layout shift comes from Group B alone.** Bisect builds confirm
  it: R1-only (Group A, no `.emp` edits) is 411267 bytes, and A+C+D is *also*
  411267 — the C-1355 −2 B is absorbed by alignment padding. Only removing the
  two hand loops and their four labels moves anything. So "byte-changing" here
  decomposes into a large, purely-opcode change that shifts nothing, plus one
  small structural change that shifts everything downstream by −0x20.
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
