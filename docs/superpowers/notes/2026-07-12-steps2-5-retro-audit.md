# Steps 2–5 retro-audit — every port before the checklists existed

**Opened 2026-07-12 (Volence + Fable).** Trigger: an advisor pass over
`dplc.emp` found that tranche 10's step 5 covered core's RunObjects only —
dplc got no logged interrogation, and a fresh look found two real items
(below). The step-3(a)/3(b)/5 interrogation checklists were only added to
the loop on **2026-07-11** (t11), so **every file ported before sprites.emp
never faced them**. This doc walks steps 2–5 back over the ported corpus
at today's standard.

**This is NOT "t10 was done badly" in general** — the packet record shows
real step-5 work in most tranches (t6 −8 B live-verified; t7 the honest
collision review; t8 a hot-loop win; t9 an explicit not-taken record).
The systemic gaps are:

1. **Cold-file skips in multi-file tranches** — step 5 gravitates to the
   hot file; the cold file gets no line at all (t10: core profiled,
   dplc silent).
2. **Pre-checklist files** — t4–t10 ran step 3/5 as judgment, not
   checklist; t0–t3 ran under the OLD 4-step loop (no step 5 existed).
3. **Post-shipped-construct retrofits** — `assert`/`table`/`let`-reg/
   asm-splice all shipped AFTER most files were ported; comment-claims
   that could now be checked invariants are sitting as prose.

## Method (per file)

Run the ratified checklists from `campaign-port-loop.md` against the file
AS IT STANDS on master (post-consolidation, post-occupancy):

- **Step 2 residue** — idiom sweep (jbra/jbsr, widths, brace style, house
  comments). Expect clean; consolidation already swept most.
- **Step 3(a)** — ceremony scan, comment-as-compensation, escape-hatch
  census, domain-type scan.
- **Step 3(b)** — comment-claim audit, contract audit, name audit,
  magic-number audit, cold-reader test.
- **Step 4** — construct retrofit: would a shipped construct
  (`assert`, `table`, comptime-fn skeleton, `let` reg) collapse a shape
  here? (Corpus-sweep precedent: perform_dplc dedup, sfx_bank table.)
- **Step 5** — invariant ladder, counter/cache audit, guard-coverage
  audit, hardware cross-check, silent-tradeoff comments. Per hot proc.
  Oracle probes where static reading can't settle it (press-only soaks —
  see ledger row 990 for the step_out/resume/press wedge).

**Finding buckets:** `[PROC]` process gap (no logged line — record only),
`[OPT]` missed optimization, `[HAZ]` latent engine hazard, `[RETRO]`
shipped-construct retrofit, `[CERT]` audited clean. Fixes ship as
retro-fix batches (precedent: 2026-07-11-retro-fixes-CA1-RA1), never
inline in this doc's commits.

## Brief for the auditing agent (one file per sitting)

You are running a retro-audit sitting on ONE ported `.emp` file. The two
completed entries below (dplc.emp, animate.emp) are your calibration
exhibits — read them first; your output must match their evidence
density, not their length.

**Ground rules:**

1. **Audit the file as it stands on aeon master TODAY** (post-
   consolidation, post-occupancy) — not the packet-era version. Read the
   whole file, its `.asm` twin, and the tranche packet's record for it.
2. **Run EVERY checklist line** from `campaign-port-loop.md` — step
   3(a), 3(b), step-4 construct pass, step-5 (per hot proc). An outcome
   is named even when it's "audited, nothing" — that's a [CERT] line,
   and it counts.
3. **Every finding carries evidence**: file:line, plus the verification
   you did. A claim about a callee's contract means you READ the
   callee's contract text (enforced vs incidental preserves are
   different things — see the animate/Sound_PlaySFX exhibit). A claim
   that a comment lies means you traced the code that falsifies it
   (see the dplc "can slip one frame" exhibit). No vibes.
4. **When a finding is an instance of a PATTERN, enumerate the pattern
   corpus-wide before writing it up** (grep is fine for enumeration —
   the rings A1-fold sibling was found that way). One pattern with all
   its sites beats N duplicate findings in later sittings.
5. **Credit the original record where it holds.** If the packet's
   not-taken reasoning survives your re-check, say so — the audit's
   credibility depends on not manufacturing findings.
6. **Findings only — NO fixes in audit commits.** Bucket as [PROC] /
   [OPT] / [HAZ] / [RETRO] / [CERT]. Fixes ship later as retro-fix
   batches. DEBUG-assert candidates use the shipped diagnostics
   construct and must note byte-neutrality in release.
7. **Static analysis first.** If a question genuinely needs the
   emulator, NAME the probe in your findings rather than improvising —
   and if you do run oracle, press-only frame driving (never bare
   `resume` before `press`; never interleave `step_out` with `press` —
   ledger row 990, it wedges the emulator).
8. **Output**: append a `### <file> (t<N>) — audited <date>` section
   after the last completed one, flip the file's status-table row to
   DONE with a finding count, one commit, message style:
   `docs(audit): <file> full-checklist sitting — <top findings>`.
   Your final report to the orchestrator = the findings section
   verbatim; it will be gate-reviewed against the code before the
   sitting is accepted.

## Corpus status table

| File | Tranche | Packet step-5 evidence for THIS file | Audit |
|---|---|---|---|
| engine/objects/dplc.emp | t10 | none (core only) | **DONE 2026-07-12** — 3 findings below |
| engine/objects/core.emp | t10 | RunObjects profiled; 2 not-taken recorded | **DONE 2026-07-12** — 8 findings incl. the mid-walk compact hazard (see collision entry) |
| engine/objects/sprites.emp | t11 | full checklist + Fable second look (A1 etc.) | **exempt** — set the standard |
| engine/objects/animate.emp | t9 | "no opt taken, reasons recorded" | **DONE 2026-07-12** — t9 verdicts confirmed; 10 new findings (below) |
| engine/objects/rings.emp | t8 | RingCollision win, live-verified + not-taken record | **DONE 2026-07-12** — A1-fold residue + certs |
| engine/objects/collision.emp | t7 | step-5 review (Volence's AABB, honest review) | **DONE 2026-07-12** — HEADLINE mid-walk compact hazard; Touch_Solid certified |
| engine/objects/aabb.emp | t7 | same tranche | **DONE 2026-07-12** — ensure-the-alias-constraint retrofit |
| engine/system/game_loop.emp | t5 | one yield (SR-mask hazard comment) | **DONE 2026-07-12** — clean |
| games/sonic4 object bank (test_particle/test_solid, act_descriptor) | t6 | −8 B live-verified | pending |
| data: particle_anims / sonic_anims | t4 | step-5 queue (pads) — executed? verify | pending (data files — 3(a)/3(b) focus) |
| engine/level/collision_lookup.emp | t3 (OLD loop) | n/a — step 5 didn't exist | **DONE 2026-07-12** — clean, transitive-clobber model file |
| engine/system/vdp_init.emp | t3 (OLD loop) | n/a | **DONE 2026-07-12** — ensure(len<=32) retrofit + flush-race probe named |
| engine/system/controllers.emp | t2 (OLD loop) | n/a | **DONE 2026-07-12** — clean; Press_Accum-consumer probe named |
| engine/system/math.emp | t2 (OLD loop) | n/a | **DONE 2026-07-12** — clean, boundary verified exact |
| engine/system/hblank.emp | pre-loop | n/a | **DONE 2026-07-12** — handler-contract comment wanted |
| engine/sound/sound_api.emp | pre-loop + A2 audit | own A2 audit | light pass 2026-07-12 — A2 credited; triggered the clobbers-semantics ruling; full sitting deferred |
| data/sound: mt_bank, sfx_bank, dac_samples | pre-campaign (M1.D/Plan-6/7 era); sfx_bank got the `table` retrofit | n/a | pending — low priority, data-shaped |
| support twins: sst.emp, constants.emp, types.emp | grown across tranches | n/a | audit as cross-cutting pass at the end |

## Findings

### dplc.emp (t10) — audited 2026-07-12, advisor pass

1. **[PROC]** t10 step 5 never touched dplc; no logged skip. (Norm
   post-2026-07-11: per-file line even when "nothing taken".)
2. **[RETRO] Single-entry invariant is prose, could be a checked
   invariant.** Header claims "contiguous art layout → exactly 1 DPLC
   entry per frame — guaranteed single DMA". Nothing checks it. With the
   diagnostics construct shipped: DEBUG `assert` that entry count == 1
   (d4 == 0 after the `subq`) — byte-neutral in release, converts the
   claim comment into a checked invariant. Cheap; candidate for the next
   retro-fix batch. (If the invariant is REAL corpus-wide, the entry
   loop + 5-reg movem around the queue call is dead generality — a
   follow-on [OPT] specialization, gated on Volence confirming the
   build-tool guarantee.)
3. **[HAZ] `prev_frame` committed before the DMA enqueue; the queue can
   silently drop.** `QueueDMATransfer.full` (dma_queue.asm) bumps a
   DEBUG counter and rts — no carry, no retry. dplc writes
   `Sst.prev_frame` BEFORE enqueueing, so on a full queue the object
   believes its art loaded and shows the previous frame's tiles until
   `mapping_frame` changes again — for rarely-changing frames (monitor
   break, signpost settle) that's wrong art indefinitely. The
   "Deferrable — can slip one frame" doc line is inaccurate: there is no
   slip, only permanent drop for that frame value. Sibling: the 128 KB
   split path with one slot left queues the first half and silently
   drops the second (partial stale art). Fix candidates (engine work,
   Volence's call): commit `prev_frame` only after successful enqueue,
   or return carry from QueueDMA (RingBuffer_Add precedent) and skip the
   commit on failure. Contract change also touches bg_anim.asm.
   Predates the port — inherited, not introduced.

### animate.emp (t9) — audited 2026-07-12, full-checklist sitting (Fable)

**Credit first: the t9 record was honest and strong** — not-taken items
recorded WITH numbers (flip-sync ~56c behavior-load-bearing; d1
re-derivation ~16c cold; bra.w tables ≈cost-neutral), `andi.w #$FF`
verified load-bearing (reads-dead but isn't), the dead
`AnimateSprite_PerFrame` caught at the gate (absent today ✓), and the
whole thing live-verified in oracle. This sitting re-confirms those
step-5 verdicts. Everything below is NEW — almost all of it is the 3(b)
guard/claim class and diagnostics-era retrofits that didn't exist at t9.

1. **[HAZ] `.evt_set_field` writes a script-supplied offset into the SST
   unbounded** — `move.b 3(a1,d1.w), (a0,d0.w)` with d0 straight from
   script data. An offset ≥ sizeof(Sst) writes into the NEXT object's
   SST (neighbour corruption, nightmare-class to debug). DEBUG
   `assert.w d0, lo, #sizeof(Sst)` — byte-neutral in release.
2. **[HAZ] Two hang-class script-authoring traps (in-frame infinite
   loops = full game hang, not garbage art):**
   (a) `AF_BACK` with N=0 — `.cc_back` subtracts 0, re-reads the same
   `AF_BACK` byte, dispatches forever within one frame.
   (b) A frameless script (byte 1 is a non-exiting control code, e.g.
   `dc.b dur, AF_END`) — `.cc_end` clears anim_frame, re-reads byte 1,
   loops forever. DEBUG asserts: N ≠ 0 at `.cc_back`; the full cure is
   the script DSL (finding 9).
3. **[HAZ-lite] `AF_CHANGE` to the CURRENT anim silently fails to
   restart** — target == prev_anim takes the unchanged path, so
   anim_frame never resets; the object freezes at the AF_CHANGE
   position and re-dispatches every timer expiry. Site comment
   minimum; DEBUG assert (target ≠ current) optional.
4. **[HAZ-lite] `AF_SET_FIELD` targeting mapping_frame bypasses
   `RefreshSpritePieceCount`** — stale piece count → SAT emits wrong
   piece count for the frame. Site comment + DEBUG assert
   (offset ≠ Sst.mapping_frame).
5. **[OPT] redundant d1 save around `Sound_PlaySFX`** — the callee's
   `preserves(d1/a0)` is ENFORCED (movem-backed, contract-stable);
   animate's `movem.l a1/d1` can be `move.l a1` (a1 stays — it's only
   INCIDENTALLY preserved, contract says don't rely). −4 B, ~24 cyc,
   cold path. Same back-prop class as the rings A1 miss: the
   `preserves()` contract shipped and callers were never re-swept.
6. **[guard-coverage] the `cmpi.b #9 / bhi .cc_end` in `.control_code`
   is unreachable** from all 6 entry paths (every entry pre-checks
   ≥ AF_SET_FIELD; `neg.b` maps $F7-$FF → 1-9 exactly). Pure defense —
   per the checklist it needs a CHOSEN site comment or removal
   (−6 B on the dispatch path). Note its fallback (.cc_end) would
   itself loop on a corrupt byte, so as defense it's half-hearted;
   naming it is the point.
7. **[OPT-micro]** `.evt_sound`'s d0 arg load sits outside the
   `SOUND_DRIVER_ENABLED` gate — 4 dead bytes in the SND=0 shape only.
8. **[NOTE]** `RefreshSpritePieceCount`'s `(a1,d2.w)` sign-extended
   index caps mapping files at <$8000 bytes — a comptime `ensure()`
   candidate when mapping data ports to `.emp`.
9. **[ASK — the 3(a) headline] animation scripts are THE
   byte-command-DSL demand case** (Plan-7 research item #3). A typed
   script construct makes findings 1-4 UNREPRESENTABLE: typed args
   (AF_BACK count ≠ 0 by refinement), required terminator, even-length
   invariant checked at comptime, field-write whitelist. This file +
   sonic_anims/particle_anims are the demand evidence; attach to the
   existing ledger row.
10. **[RETRO/step 4]** the fetch-frame-and-dispatch tail (6 lines)
    appears 3 full + 2 partial times — comptime-fn dedup candidate,
    byte-neutral (emit_piece_loop skeleton precedent).

No oracle probes required — all findings static-decidable. Optional:
re-profile AnimateSprite's frame share post-occupancy (t9's numbers
predate the live-list).

### core.emp (t10, post-occupancy) — audited 2026-07-12 (Fable)

Fresh occupancy code is GOOD — capacity-guard logic verified sound
(pop-succeeded ⇒ stale entry exists ⇒ compact reclaims ⇒ append has
room), §6 DEBUG rails verified, `clear_longs` derive-from-type is the
construct pattern working. Findings are mostly at the SEAMS:

1. **[HAZ-lite/contract] RunObjects_Frozen relies on Draw_Sprite
   preserving a2 — the contract doesn't say that.** Frozen's comment:
   "Draw_Sprite preserves a0/d7/a2, so no save is needed". Draw_Sprite's
   header (sprites.emp:47-50): "Preserves: a0, d7" — a2 is INCIDENTAL
   (body happens not to touch it). If Draw_Sprite ever grows an a2 use,
   the frozen dynamic walk corrupts its cursor silently. Introduced by
   occupancy step 3 (fresh code). Zero-cost fix: promote a2 into
   Draw_Sprite's documented+enforced preserves, or fix Frozen's comment
   + save a2. Same enforced-vs-incidental class as the animate finding.
2. **[HAZ-lite/claim] AllocDynamic's compact-on-full guard depends on an
   UNDOCUMENTED caller invariant** — compaction treats code_addr==0 as
   dead, so a caller that allocs a second slot before initializing the
   first would get the first silently dropped from the live list
   (claimed-but-invisible zombie), only when the list is full. All
   current callers verified clean (children.asm ×4, load_object.asm,
   object_test_state.asm — all write code_addr immediately). Fix:
   document "callers must set code_addr before the next AllocDynamic"
   in the header; optionally a spec §6 note.
3. **[RETRO] DeleteObject's "shouldn't happen" out-of-range path clears
   64 bytes at a wild address** — an a0 past the effect range falls to
   `.clear_slot` and zeroes memory beyond Object_RAM. DEBUG assert at
   entry (a0 within Object_RAM..End, Debug_AssertObjLoop spelling) turns
   silent corruption into a named error.
4. **[RETRO] No double-delete guard** — deleting an already-deleted
   pool slot pushes its address onto the free stack TWICE → a later
   double-alloc puts two objects in one SST (the classic catastrophic
   Sonic bug class). DEBUG assert candidate: code_addr ≠ 0 at entry.
   (Caveat: needs assert-construct memory-operand support, else
   `if DEBUG==1 { tst.w (a0) … raise_error }`.)
5. **[RETRO→ram.asm] Frozen's merged System+Effect 24-slot sweep
   depends on RAM adjacency nothing drift-locks.** Comptime `ensure()`
   CANNOT check it (link-time addresses) — but ram.asm can
   (`if Effect_Slots <> System_Slots+SST_len*NUM_SYSTEM / error`), all
   symbols resolve there. Ledger note: "link-time ensure" is a small
   language gap (demand row 1).
6. **[3(b)] Unused import**: NUM_TOTAL_SLOTS — stale from the
   pre-occupancy 66-slot sweep. Remove at next touch. Language ask:
   sigil has no unused-import lint (would have caught this).
7. **[OPT, recorded-not-urged] .run_culled reloads Camera_X/Y from RAM
   per checked object** (~24 cyc/object); frame-invariant, but a
   register hoist is blocked by dispatch clobbers (only a0/d7 survive
   object code) — a reload-after-dispatch pattern nets a few hundred
   cyc/frame at high occupancy. Post-occupancy RunObjects is 1.9% of
   frame, so this is small; profile-then-decide.
8. **[CERT]**: ObjectMove/X/Y = S3K-standard shape, alternatives
   cost-equal (invariant-ladder outcome: no change). AllocDynamic /
   CompactDynamicLive logic sound. extern() census: ~7 RAM-symbol
   sites = S2-D3 demand-data increment (known gap, no new ask).

### collision.emp + aabb.emp (t7, post-occupancy/splice) — audited 2026-07-12 (Fable)

**THE HEADLINE FINDING OF THE AUDIT SO FAR — [HAZ] mid-walk
compact-on-full (really a core.emp/occupancy hazard, surfaced by
tracing the collision walk):**

AllocDynamic's capacity guard runs CompactDynamicLive whenever an alloc
finds Dynamic_Live_Count == NUM_DYNAMIC. But allocs happen MID-DISPATCH:
object routines spawn children (children.asm, called from object code)
and the ObjectTest emitters alloc every frame from inside RunObjects.
CompactDynamicLive MOVES entries down and shrinks the count while a
walker (.run_culled / .frozen_dyn / TouchResponse's dyn segment) holds a
cursor into the array. After a mid-walk compact: the cursor points past
the compacted prefix, the walker's snapshot count keeps it reading the
STALE TAIL (compaction rewrites only the kept prefix — tail words keep
their old values), and a stale duplicate that still passes the tst.w
guard **double-dispatches an object in one frame** — the exact A1 bug
class the occupancy design fought. CompactDynamicLive's own header
claims the alloc-guard case runs "before any dispatch this frame" —
FALSE for mid-dispatch spawns; the comment encodes the wrong
assumption. Reachable: churn-heavy frames where deletes have zeroed
entries (count still high) and a spawner allocs — the ObjectTest stress
scene (33/40 occupied + per-frame particle churn) is plausibly close.
RECOMMEND NOW (rail): DEBUG walk-live flag (st/clr at each walker's
entry/exit) + `assert` not-set in CompactDynamicLive — cheap,
soak-testable. DESIGN FIX = Volence ruling (occupancy amendment A2
candidate): (a) hole-fill append at full count (positions stable;
bends spawn-order for the filler), (b) treat full-count-during-walk as
alloc-fail (callers already handle .alloc_fail), or (c) overflow latch
drained at frame end.

Other findings:

1. **[CERT] Touch_Solid verified line-by-line** — min-pen axis logic,
   sign handling, 1px maintain-contact bias, rising/falling gates all
   correct. The t7 honest-review verdict holds.
2. **[CERT] touch_test_target** — every aabb template instantiation
   satisfies the stmp non-alias constraint; the movea.w stash
   round-trip is exact (sign-extend in, low-word out); the
   COLLISION_TOUCH bhi guard + cache-freshness reasoning verified.
   The skeleton-with-holes dedup + stub falls_into chain (every stub
   aliases one rts, lint-guarded) are exemplary construct use.
3. **[RETRO] aabb.emp: the "stmp MUST NOT alias cdim or delt"
   constraint is prose — make it a comptime `ensure`** (Reg equality
   already works: lead_move compares `adim != cdim`). Compile error
   instead of silent wrong code for a future call site. Optional
   sibling: delt==apos aliasing breaks the apos-read-only promise —
   consider ensuring or documenting.
4. **[NOTE] aabb boundary**: delta = −32768 survives `neg.w` as $8000 →
   doubled = 0 → false overlap. Unreachable through current callers
   (cull windows bound |delta| ≪ $4000); one site comment would
   immunize it against new callers.
5. **[NOTE] handler contract** ("a5-a6 MUST survive") is enforced
   nowhere — fine while handlers are stubs; the first real handler
   should carry enforced `preserves()`.
6. **[LEAD] `ensure(extern("SST_interact") == …)` WORKS** (the
   interact_off drift-lock) — extern-in-ensure resolves through the
   link seam. core finding 5's RAM-adjacency lock may be expressible
   in .emp directly after all; try before the ram.asm fallback.

### rings.emp (t8 + art/colour rework) — audited 2026-07-12 (Fable)

1. **[OPT]** the DrawRings A1-fold residue — see the byte-shackle sweep
   section (the audit's rings headline). Two tiers: minimum = fold each
   `subi #8`+`addi #offset` pair into one immediate (−8 B, −16 cyc/ring,
   no restructuring); full = bias the cached camera regs once (d6/d7
   load site) and drop all four per-ring ops (−16 B, −32 cyc/ring),
   cull immediates compensate at comptime.
2. **[CERT] the t8 rolling-pointer + swap-with-last removal is PROVEN
   correct** — backward iteration means the swapped-in entry (from a
   higher index) was already visited: no double-test, no miss. Per-player
   count re-read is correct after P1 removals.
3. **[CERT] all five callee-contract claims in RingCollision's header
   verified EXACT** against today's procs (Collected_MarkRing d0-d1/a0;
   EntityWindow_EntryForSection d1/a0; EntityLoaded_Clear d0/d2/a0;
   Sound_PlayRing d0/a0; RingBuffer_Remove d1-d2/a0-a1).
4. **[RETRO-micro]** RingBuffer_Remove has no bounds check on the index —
   DEBUG assert (d0 ≤ last) candidate.
5. **[NOTE — named assumption]** DrawRings emits mid-chain SAT entries;
   final-link=0 termination is Render_Sprites' job (t11-audited file).
6. **[CERT]** RingBuffer_Add's stack-based ×6 keeps the clobber contract
   tight (deliberate); the DEBUG drop-assert (always-fails-on-drop with
   register comparand) is sound; the per-frame anim-attr hoist (d4) is
   the invariant ladder already applied.

### game_loop / controllers / math / vdp_init / collision_lookup / hblank (t2/t3/t5 + pre-loop) — audited 2026-07-12 (Fable)

Small files (25-66 ln each), first-ever step-5 pass for the old-loop
ones. Overall verdict: **clean — the transliterations were careful and
the recent retrofits (drift-lock ensures, tradeoff comments) reached
these files.** Real items:

- **[CERT+] math.emp**: cos overlap boundary verified exact — angle $FF
  → ×2+$80 = $27E reads the last word of the $280 table, no overflow;
  typed embed length doubles as a size check. Exemplary.
- **[CERT+] controllers.emp**: L+R/U+D worn-pad guard is a CHOSEN,
  commented tradeoff (re-edge on blip end — named by design); edge
  accumulation logic verified ((old^new)&new). Step-4 candidate: the
  P1/P2 duplicated body is a 2-instance comptime-fn candidate
  (borderline — note only). Named probe: confirm the Press_Accum
  consumer clears after read (cross-file; §5 design says lag-frame
  accumulation, so a non-clearing consumer would stick presses).
- **[RETRO-micro] vdp_init.emp**: Flush's `btst d2` mask aliases mod 32
  and the moveq caps at 127 — `ensure(VDP_Shadow_len <= 32)` comptime
  drift-lock (imported const, so ensure CAN see it). Named probe:
  confirm no dirty-bit writer runs in interrupt context (the
  read-mask→clr.l window would lose a mid-flush dirty set; if all
  writers are main-loop + flush is VBlank-context, race-free).
- **[3(b)-micro] hblank.emp**: the handler-side contract is unstated —
  dispatch preserves d0-d1/a0, so handlers may clobber ONLY those; one
  comment line at HBlank_Handler_Ptr / HBlank_Null states it.
- **[CERT] game_loop.emp**: non-returning loop (contract moot but
  harmless); drain + debug-hook gating correct. collision_lookup.emp:
  bounds logic verified (lsr makes operands non-negative, signed
  compares safe); transitive tail-call clobbers documented RIGHT —
  a model for other files.
- **sound_api.emp: light pass only** — it carries its own recent audit
  (the A2 ring-drain fix + enforced/incidental preserve distinctions);
  its contract language triggered the cross-cutting finding below. Full
  sitting deferred; credited.

### CROSS-CUTTING — the audit's decision item: `clobbers()` semantics
need ONE ruling (Volence)

The corpus holds two incompatible conventions:

- **Exhaustive-license** (the S2-D6 lint direction, and how callers
  behave): `clobbers()` is the COMPLETE license — everything not listed
  is contractually preserved. RingCollision's five-callee reliance,
  Frozen's a2-across-Draw_Sprite, and collision_lookup's transitive
  documentation all assume this.
- **Minimum-license** (sound_api.emp's explicit language): regs outside
  the clobber list are "INCIDENTAL — NOT a guarantee, do not rely";
  only `preserves()` (movem-enforced) is contractual.

Under exhaustive-license: sound_api's warning text is wrong and should
be rewritten; animate may drop BOTH saves around Sound_PlaySFX (−8 B);
Frozen's a2 reliance is fine as-is. Under minimum-license: Frozen's a2
reliance is a live bug-in-waiting, animate's a1 save is load-bearing,
and dozens of not-in-clobbers reliances corpus-wide need re-audit.
**Recommendation: ratify exhaustive-license** — it matches the S2-D6
checked-clobbers future (the lint will VERIFY the license), makes
today's caller behavior correct, and turns the fix into one text edit
in sound_api + two comment edits. The earlier core finding 1 and
animate finding 5 resolve per this ruling.

### Mechanical byte-shackle sweep — corpus-wide, 2026-07-12

**Question (Volence):** did pre-checklist step 2/5 fail to optimize because
byte-exactness anchoring bled past step 1? (The t6 packet's "byte-neutral
this time" and the bne.w incident are the documented smoke.)

**Method:** grep-class sweep of every ported `.emp` for mechanically
checkable residue: `addi/subi #1-8` (addq/subq-able), `move #imm≤127,dN`
(moveq candidates), `cmpi #0` (tst-able), `lsl #1` (add-able), and local
`Bcc.w` (the bne.w shrink class). Decimal + hex immediate forms.

**Result: the corpus is largely CLEAN on peephole classes** — core,
sprites, dplc, collision, aabb, game_loop, and all old-loop files show
zero hits. The idiom floor (moveq/tst/addq usage) was already right in
the transliterations. The `.w` clusters that do exist are documented
structural pins (animate's `.cc_table` bra.w ×9 = pc-indexed 4-byte
slots, comment names the exception; entity_window's DEBUG-conditional
locks per the t12 design note).

**One confirmed residue instance — rings.emp DrawRings (lines ~209-217),
and it's the restructuring class, not the peephole class:**

- Surface: `subi.w #8` ×2 should at minimum be `subq.w` (−4 B), and each
  `subi #8` + `addi #VDP_SPRITE_{X,Y}_OFFSET` pair folds to one
  immediate (−8 B, −16 cyc per ring).
- Real finding **[OPT]+[PROC]**: this is the **A1 camera-bias-fold class**
  — DrawRings re-adds the SAT bias per ring exactly the way
  Emit_ObjectPieces did per piece before A1. `Camera_{X,Y}_Biased`
  ALREADY exist in RAM (ram.asm:275, computed per-frame by
  Render_Sprites). Bias DrawRings' camera regs once before the loop
  (folding the −8 ring-centre offset into the same load) and the loop
  body drops ~16 B / 32 cyc per drawn ring; cull-check immediates
  compensate at comptime. Caveats for the fix batch: verify DrawRings
  runs after Render_Sprites' biased-camera write in frame order (else
  compute locally, 2 instrs/frame); X=0 sprite-mask check semantics
  unchanged (it already tests the biased value).
- Process lesson: this is a **t11-A1 step-4 back-prop miss** — when A1
  landed, "who else writes SAT entries with per-sprite bias?" was one
  grep away (this sweep's grep). Back-prop after an engine optimization
  should enumerate the PATTERN's other instances, not just the file's.

**Honest scope note:** the sweep covers greppable classes only. The
deeper step-5 classes (counter/cache asymmetry, guard coverage,
algorithmic) are not greppable — that's what the per-file sittings
below are for. Interim answer to the opening question: step 2 was NOT
meaningfully shackled on idiom-level output; the shackle shows in
missed RESTRUCTURING wins (A1 itself only surfaced in the t11 second
look, and its rings sibling is still unfixed).

## RULINGS (Volence, 2026-07-12)

1. **A2 mid-walk compact: RAIL FIRST, decide after soak.** Land the
   DEBUG walk-live flag (st/clr at each live-list walker's entry/exit)
   + `assert` not-set at CompactDynamicLive entry; soak ObjectTest;
   the design fix (alloc-fail / latch / hole-fill) is ruled only if it
   fires. Occupancy spec gets an A2 note pointing here.
2. **clobbers() = EXHAUSTIVE LICENSE, ratified** — canonical text now in
   campaign-port-loop.md (before the Step-4 section). Consequences:
   sound_api's "incidental — do not rely" text is nonconformant
   (rewrite in the fix batch); animate drops BOTH saves around
   Sound_PlaySFX; Frozen's a2 reliance is correct as-is (Draw_Sprite's
   header gains a2 nowhere — not-listed already covers it; its explicit
   "Preserves: a0, d7" line stays as movem-emphasis only if enforced,
   else reads as prose emphasis).
3. **Retro-fix batch: FULL BATCH, one branch, now** (brief below).
4. **Step-4/6 pattern-enumeration amendment: RATIFIED as written** —
   canonical text appended to campaign-port-loop.md Step 6.

## Retro-fix batch brief (for the implementing agent)

One branch off both masters (`retro-fix-audit-1` sigil / aeon), one
review, one re-pin wave at the end. Work in this order — byte-neutral
items first, the two byte-changing items last. EVERY item lands with its
twin edited in lockstep and the relevant port gate green both shapes.
Mind the in-flight t12 entity_window work — do NOT touch
entity_window.{asm,emp}; anything landing there goes through the t12
tranche instead.

**Byte-neutral (DEBUG-shape-only or comment/ensure — release ROM
unchanged; DEBUG ROM re-pins expected):**
1. core: A2 rail — walk-live DEBUG flag set/cleared by .run_culled,
   RunObjects_Frozen's dyn walk, and TouchResponse's dyn segment;
   `assert` clear at CompactDynamicLive entry. New DEBUG-only RAM byte.
2. core: DeleteObject entry assert (a0 within Object_RAM..End — reuse
   the Debug_AssertObjLoop spelling) + double-delete assert (code_addr
   ≠ 0 at entry; if the assert construct lacks memory operands, load to
   a saved scratch first — RingBuffer_Add's register-comparand
   precedent).
3. core: document the set-code_addr-immediately caller invariant in
   AllocDynamic's header; delete the unused NUM_TOTAL_SLOTS import;
   fix CompactDynamicLive's "before any dispatch this frame" comment to
   name the rail instead.
4. animate: AF_SET_FIELD bounds assert (offset < sizeof(Sst)) + assert
   offset ≠ mapping_frame; AF_BACK N≠0 assert; site comments for the
   AF_CHANGE-to-self freeze and the frameless-script hang (assert if a
   cheap spelling exists, else comment + script-DSL ledger ref).
5. animate: drop BOTH saves around Sound_PlaySFX (movem pair deleted —
   exhaustive-license ruling). NOTE: this IS byte-changing (−8 B) —
   group with item 10's re-pin.
6. dplc: DEBUG assert entry_count == 1 (d4 == 0 after the subq) — the
   single-entry invariant, comment becomes checked.
7. aabb: comptime ensure stmp ≠ cdim && stmp ≠ delt in aabb_axis_test;
   one-line comment on the −32768 neg.w edge.
8. vdp_init: `ensure(VDP_Shadow_len <= 32)`; hblank: handler-contract
   comment (handlers may clobber only d0-d1/a0).
9. sound_api: rewrite the enforced/incidental paragraph per the
   exhaustive-license ruling (a1/d2-d7 preservation is contractual;
   the movem'd d1/a0 stay "enforced-emphasis"). Try
   `ensure(extern("Effect_Slots") == extern("System_Slots") +
   sizeof(Sst)*NUM_SYSTEM)` in core (collision.emp extern-ensure
   precedent); if extern-in-ensure rejects RAM labels, put the
   equivalent if/error in ram.asm and add the link-time-ensure ledger
   row.

**Byte-changing (re-pin wave + oracle verify, LAST):**
10. rings: the DrawRings camera-bias fold — bias d6/d7 once at the
    cache site (fold the −8 centre offset in), drop all four per-ring
    subi/addi, compensate the two cull immediates at comptime. Twin in
    lockstep. Oracle verify: rings render identical (SAT compare at a
    fixed Frame_Counter anchor — frame-lock, press-only driving), then
    re-pin.
11. dplc: move the `prev_frame` commit AFTER a successful enqueue OR
    give QueueDMATransfer a carry return consumed by perform_dplc —
    design choice documented in the packet; bg_anim.asm calls
    Deferrable too, keep its behavior unchanged. Oracle verify art
    loads under queue pressure if reproducible, else static + gate.

Packet at the end: per-item outcome + the re-pin diff, findings
cross-referenced to this doc's sections.

## Proposed audit order

1. **animate.emp** (t9) — hot path, pre-checklist "not taken" verdict is
   exactly the anchoring class the checklist was written for.
2. **core.emp** (t10) — hot path; heavily reworked since port
   (occupancy, diagnostics) so the packet record is stale anyway.
3. **collision.emp + aabb.emp** (t7) — hot path; splice touched aabb.
4. **rings.emp** (t8) — hot path but strongest existing record; also
   re-touched twice since.
5. **game_loop.emp** (t5), object bank (t6).
6. OLD-loop files (t2/t3 + pre-loop): controllers, math,
   collision_lookup, vdp_init, hblank, sound_api — never had ANY step 5.
7. Data files (t4 anims; sound banks) — 3(a)/3(b) focus.
8. Cross-cutting: sst/constants/types twins.

One file per sitting, findings appended here, fixes batched. Hot-path
files get the oracle where static reading can't settle a counter/guard
question (press-only driving).
