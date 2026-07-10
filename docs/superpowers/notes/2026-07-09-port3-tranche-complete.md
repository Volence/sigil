# Tranche 3 (collision_lookup.asm + vdp_init.asm) — COMPLETE through step 3, awaiting checkpoint

Second tranche under the ratified 4-step loop; the last two small code targets from the
kickoff ranking. Both branches UNMERGED.

- **sigil `port-tranche3`** (worktree `.worktrees/port-tranche3`, off master `f9c6dab`,
  6 commits): indexed EAs (`115b7e0`) → preserves() slice (`b785eff`) → step-1 gates →
  step-2 modernize (`72efa0e`) → ledger passes → review fixes (head).
- **aeon `sigil-emp-tranche3`** (main tree, off `925f411`, 4 commits): transcriptions +
  gates → step-2 modernize → §10 conventions → code-sense polish (head).
- **empyrean** (working tree, UNCOMMITTED per docs cadence): the 2026-07-09d amendment —
  D2.32 (§2 row, §5.1, S2-D6 ledger row, §10 growth note) + the struct-equ-export
  extension noted on D2.27.

**Validation:** strict gates green END-TO-END — both new port gates (both shapes), 8
tranche-3 negative probes (incl. the NEW pc-rel target-position class), tranche-2/1/sound
gates intact over the modernized files, **eight-module full-ROM mixed gates byte-identical
to both reference ROMs**, aeon gate-off byte-neutrality sha256 ×3 (plain `8ce6dd7e…`,
debug `13c7b063…`). Workspace **1883/0 strict, zero skips**, clippy clean.
Reviews: two-prong — adversarial branch review (1 Important + 4 minors, ALL FIXED
on-branch, empirically probed; 1 latent pre-existing item ledgered) + code-sense pass
(verdict: vdp_init passes as from-scratch .emp; 7 polish items LANDED; 5 byte-changing
items on the reads-wrong list below).

## What the tranche shipped beyond the two files

1. **`preserves(...)` — the S2-D6(b) syntactic slice (D2.32)**, pulled FORWARD to the
   tranche opening per the step-2-apex rule. Declared movem-reglist attribute verified
   against the literal stack-movem pairs: intersects-must-equal coverage (wrong-list
   early-exit restores caught, disjoint nested saves legal), `movem.l` required (a `.w`
   restore sign-extends — the review's Important), error-tier, NOT silenced by
   `@as_compat`. `HBlank_Dispatch` is the first consumer.
2. **`(An,Xn)` / `d8(An,Xn)` indexed EAs** — the third .emp operand-grammar gap closed
   (sp-alias/movem-reglist bucket); AS-parity pinned incl. the real Flush_VDP_Shadow
   vector; base-size-suffix and bare-`(pc,Xn)` edges diagnose with steering.
3. **Struct-equ export** — AS struct-generated `_len`/`_field` symbols export as link
   equs (Item-B seam extended): enables layout-derived drift guards
   (`ensure(extern("VDP_Shadow_len") == …)`) and pre-paves vars-era struct reads.
4. **Cross-seam PC-RELATIVE references proven** — first `lea Sym(pc)` and first `bsr.w`
   to AS-side labels in ported bodies; full-ROM proof that .emp pc-rel distances survive
   joint layout; new probe class pins that target POSITION is load-bearing.
5. **Twin growth pattern exercised** — engine.constants +CTYPE_AIR +VDP_Shadow_len (8
   guards); first module OUTSIDE engine/system (the sibling-dir ambient wiring).
6. **Clobber-lint refinements** — sp stack-ARITHMETIC exemption (add/sub families +
   `lea N(sp),sp`; stack REPLACEMENT still flags); the retro clobbers() annotations
   landed on controllers/math + all three new procs.
7. **Small-opens bundle: 3 of 4 landed** (pc reserved-token doc line, PcRel range
   messages name the target symbol, Owner.label(pc) test). abs.l destinations RE-SCOPED:
   needs a pinned-width abs-sym mode — feature work for the port that spells it.

## The reads-wrong list (byte-CHANGING, post-port commits — Volence's standing rule)

From the code-sense review, each its own commit after merge, oldest-hazard first:

1. **Collision_GetType: delete the stack push of the world column** — Y is already in
   d1; shift both in place, drop `move.w d1,d2` + push/pop + `.cgt_air_pop`, shrink
   clobbers to d0/d1. Biggest item; pairs with:
2. **Tail call**: `jbsr Tile_Cache_GetCollision / rts` → `jbra`.
3. **vdp_init: `clr.l VDP_Dirty_Mask`** for both moveq/move.l zero-writes (RAM operand —
   the I/O read-before-write hazard does NOT apply; controllers' TH writes stay move.b).
4. *(marginal)* Flush early-exit via shift-out loop (`lsr.l #1,d1 / bcc.s` + `beq .done`)
   — VBlank-budget relevant only if headroom ever matters.
5. *(marginal)* controllers P1/P2 dedup via pointer loop (needs the adjacent RAM layout
   it already has).

## RETROSPECT #3 (step 3 — rulings requested)

| Entry | Recommendation |
|---|---|
| Items 1–7 above | SHIPPED in-tranche — ratify with merge |
| **The no-effect proc** | RULED + SHIPPED same day: `clobbers()` = verified touches-nothing (lint-enforced); HBlank_Null annotated |
| **Checklist self-review**: the standing step-2 checklist held up; two additions surfaced — stack-discipline writes (now lint-exempt by class) and the no-effect-proc ambiguity above | Amend checklist with both once ruled |
| **abs.l destinations** | RULED + SHIPPED same day: explicit `(expr).w`/`(expr).l`, both positions, int + symbolic (pinned single fixup); bare symbols stay the default; `(a0).w` now rejects |
| **`ifndef`-guarded equ/struct export gap** (review finding, latent, PRE-EXISTING) | FIXED on-branch at Volence's packet-review call (ever-exported set carried across passes, re-attached from the converged env; pinned by test) |
| **Comptime Data indexing** + typed Data views (`[i16; 320]` embeds) | RATIFIED IN (packet review) — one work item, tranche 4's opening; A-Spec2.3 record rides the build |
| **Typed data-register params** (`d0: Angle`) — CONFIRMED WORKING | Application deliberately deferred to construct walk #3 (Volence driving) — don't front-run the newtype naming |
| `~mask` `&$FF` ceremony | RESOLVED: probe showed `#~(mask)` already works (signed imm8) — the `&$FF` was inherited asl spelling, now dropped from controllers.emp |
| **Unsized conditionals** | RULED + SWEPT same day: unsized in new-style files (8 sites across 3 files, byte-identical); explicit sizes only under `@as_compat`; checklist + §10 amended |

## Checkpoint asks

1. Merge sigil `port-tranche3` → master (`--no-ff`), remove worktree, delete branch.
2. Merge aeon `sigil-emp-tranche3` → master (`--no-ff`), delete branch.
3. Rule on the retrospect table (esp. the empty-`clobbers()` question).
4. **STEP 5 (ratified at the packet review): optimize.** The reads-wrong list lands as
   post-merge commits (each re-gated against a REBUILT reference — these change the
   reference ROM); later retrospects may send step-5 work back to already-ported files.
5. **Tranche 4 proposal (CORRECTED at recon):** the data quick-wins — `vram_bases`,
   `ojz_act_pool`, `particle_anims`, and `sonic_anims` (83 ln, the full 15-member
   offsets+inline-bodies shape). `plantbadmaps_anims` DROPPED: recon found it is not in
   the build at all (parked editor export, zero s4.lst hits — no window to byte-gate);
   its naming question is ledgered with a free-rename window. Alternatively `game_loop.asm` now
   that the gate pattern is proven (its SOUND_DRIVER_ENABLED ifdef + gameDebugTick macro
   are the next hazard class to design against deliberately).
