# t23 — TRANCHE CLOSE PACKET (boot conversion + the BootData seam)

**Fifth tranche under the corrected LEAN amendment.** Scope:
`engine/system/boot.asm` → SPLIT (boot.asm code + boot_data.asm data tail,
the brief's ruled .asm-data-tail seam) → `boot.emp` (the ONLY port file),
full loop `0 → 1 → 2 → (3→4→5)* → panel → 6`. Overseer checkpoint-(a)
countersign banked (own dual rebuild + own strict 2584/0 at that point);
three rulings executed in the loop (P3 demanded-feature / wave-queueing /
shadow-offset hoist).

Branch tips at close: **aeon-t23 (this packet's sibling commit; prior
`41a5b8e`) / sigil-t23 (this packet's commit; prior `b854208`)**, bases
aeon `7c97070` / sigil `a8211e8` (the brief commit).
Branch ROMs at close: **plain `01832b1a`/421157 · debug `154076f8`/429232**
(the ruling-2 wave moved the canonical CRCs: −2 both shapes at boot slides
every engine base; the debug TOTAL +30 is the convsym symbol-table appendix
re-encoding — ASSEMBLED_LEN/DEBUG_ASSEMBLED_LEN both UNCHANGED, org-$10000
shield; PROVENANCE re-baseline due at merge).
Full paired strict at every byte-changing commit; final: **2588/0**
(baseline 2573 + 8 tranche23 probes + 6 boot_port + 2 mixed_tranche23; the
lower_code `.b` fence test repinned net 0; 1 pre-existing ignored is the
baseline's own).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **The split commit (P6, FIRST)** | boot.asm → boot.asm + boot_data.asm + engine.inc gate (data tail in BOTH arms — no splice crosses the Z80 source include); cursor-contract assert wall (head 26 / blob +54 / evenness / total lock, BOTH sound arms); byte-neutral EXACT; wall negative-fired by hand; demo's idle arm proves the else-wall live |
| **boot.emp** (single pub proc EntryPoint, ⊤ contract + sr) | byte-identical BOTH shapes at the first LINKING compile (two demanded features + one spelling normalization + sr contract surfaced by the first lower — the probe law working) → step 2 byte-neutral modernize → loop pass 1 → the −2 wave |
| **FOUR demanded features (TDD)** | `.b` imm-link deferral (Value8 @ ext-word low byte, unsigned-8 loud); movem pinned-abs seam (Abs16/32Be @ offset 4); **moveq link-imm8 (ruling 1): DISTINCT `ImmSigned8` fixup, SIGNED [-128,127], out-of-window fails the link naming the moveq window**; assert-wall class (AS-side, both assemblers) |
| **Off-canonical gates** | sound-off twin-parity plain+debug (oracle-fed; the boot region is SHAPE-DIVERGENT at sound-off — moveq 2 bytes shorter — bases derive from the oracle, never pins) + the hotkeys (1,1) drift-matrix arm (gameBootHook mirror vs the REAL game.asm expansion) |
| **Ruling-3 hoist** | VDP_Shadow offsets → engine.vdp shared block (MODE1/MODE2/HINT + panel-caught MODE3 from parallax); TWO consumer mirrors deleted, ensures ride the twin; byte-neutral |
| **The ruling-2 wave** | clr.w Frame_Accumulator (−2 both shapes, cycle-EQUAL per C1: 16cy = 16cy) — ONE wave, 77 pins −2, 30 engine.inc orgs, REGION_A_LMA, repin_pins baselines, and the row-1257 sweep which caught TWO stale fixture classes and MIGRATED them to pins permanently |
| **Ownership flips** | NONE (per brief); the reset vector's `dc.l EntryPoint` flipped to the .asm→.emp class, proven inside the mixed arm |

## Step 0 (design note `2026-07-24-t23-step0-design.md`, committed before code)

Probes at the real binding class: P1 forward pc-rel PASSED as-found; P2
link-imm16 arithmetic PASSED as-found; **P3 moveq FAILED AS PREDICTED →
pinned as evidence → overseer ruling 1 → SHIPPED in pass 1**; P4 imm32
halves PASSED, **imm8 half FAILED → `.b` deferral SHIPPED at step 0 (TDD)**;
P5/P6 executed as full-scale gates (mixed arm / split dual-rebuild). P7
(movem pinned-abs) surfaced at the first lower — demanded-feature law, TDD.
Trip-check: no at-next-touch row named the files; rows 109/1088/1257/1482(d)
and the t21 sr_masked fence all honored. Convsym's `-a` ROM-appendix was
identified as the split's load-bearing constraint (wall = existing symbols +
if/fatal only). Deviations (3), all overseer-endorsed: setVDPReg precedent
cite corrected (brief error, recorded per t21 precedent); HW ports uniform
bare-link; first-compile bar bound at first LINKING compile.

## Step-1 gate lists (artifacts — all EXECUTED)

**Split (aeon `2a32423`)**: dual rebuild EXACT (06290799/e280a49b); wall
negative-fire (doctored movem-head → asw fatal naming the invariant,
reverted); `./build.sh demo` green (idle-arm wall live).
**boot.emp (aeon `ec6f0c5` / sigil `17c2fbc`)**: byte gates
`boot_port::boot{,_debug}_region_matches_reference` both shapes; negative
probe `doctored_psg_port_fires_its_guard` (fires NAMING the constant);
region pins via repin (BOOT $200/$1AA/$1AE at step 1) + 11 symbol pins +
repin_pins asserts; gate `SIGIL_EMP_BOOT`; gate-off dual rebuild exact;
mixed acceptance `mixed_tranche23_{rom,debug_rom}` with the P5 proofs NAMED
in-arm (reset vector → the .emp-owned EntryPoint; org-resume lands BootData
canonically); floating values (Z80_SOUND_SIZE / GAME_ENTRY_ID / Game_Entry)
listing-PARSED, never hardcoded.
**Off-canonical (pass 1, post-ruling)**:
`boot_port::boot_sound_off_twin_parity_{plain,debug}` +
`boot_hotkeys_shape_twin_parity` (t21 AS-side-ROM-as-oracle class,
self-consistent worlds).

## Byte-delta table (measured, not predicted)

| Change | Δ plain | Δ debug | Absorbed by |
|---|---|---|---|
| split commit (P6) | 0 | 0 | — (dual rebuild exact) |
| boot.emp step 1 (+gates, pins, mixed arm, 2 demanded features) | 0 | 0 | — (additive; gate-off exact) |
| step 2 modernize (jbsr ×7 / jbra ×2 / bare Bcc / 11 overrides bared / brace-indent) | 0 | 0 | — (every relaxation ladder-neutral) |
| pass 1: P3 feature + parity gates; 3(b) comment fixes; refinement typing; ruling-3 hoist | 0 | 0 | — (comment/comptime/frontend only) |
| **the ruling-2 wave: clr.w Frame_Accumulator (twin lockstep)** | **−2** ($1AA→$1A8) | **−2** ($1AE→$1AC) | repin 77 pins −2; 30 engine.inc resume orgs; REGION_A_LMA $3E0→$3DE; repin_pins baselines+changelog; row-1257 sweep → vdp_init_port carrier + mixed_tranche3 windows MIGRATED to pins. CRCs → 01832b1a / 154076f8 (debug total +30 = convsym re-encoding; assembled lens unchanged) |
| panel adjudication (comment batch both twins + MODE3 hoist completion) | 0 | 0 | — (dual rebuild exact post-wave values) |

## Step-2 filled checklist (all seven items walked — full table in the step-0 note's checkpoint addendum)

1. Branch conversions: jbsr ×7, jbra ×2, bare Bcc file-wide — zero byte
   movement (no wave from step 2 itself). Macro-mirror block keeps bsr.w
   (structural, kill row 45).
2. Width pins with site comments: movem `(RAM_Start).w` (pinned-abs seam);
   row-1046 kept-width class ×7 (the four region-timing stores were CAUGHT
   live by the gate when bared — the class is enforced, not folklore);
   macro-mirror spellings. Panel A1 consolidated the sharded comments to
   block form and turned the class into a ledgered compiler ask (demand +7).
3. Bare-symbol width rule: 11 operand overrides dropped (HW ports abs.l,
   RAM cells abs.w, comptime VDP_CTRL folds).
4. Brace-indent: comptime-ifs at the instruction column, bodies one deeper.
5. Idiom list: reglist RANGE form (`d0-d7/a0-a6/sr`); label-in-immediate;
   bare sym+const over link base; rest not-applicable, named.
6. Type-layer walk: LOG-only — GameStateId (item-13 candidate), SongId
   bless at the mirror (deferred with hook-typing), cursor registers are
   protocol-raw by design.
7. Noticing: the `A-B(aN)` displacement grammar gap → ledgered ask, not a
   checklist line (single site; the named-derive workaround reads better).

## PER-PASS: step-3 vs step-5

**Pass — steps 0-2:**
- *step-3 flavored:* P3/P4c probe outcomes; the displacement-parse gap; the
  brief's setVDPReg cite correction; the sr contract surface.
- *step-5:* clr.w candidate QUEUED (not taken at step 2 — the wave-batching
  reading the overseer then ratified as ruling 2).

**Pass 1 — 3(a) (all lines run; `2026-07-24-t23-loop-pass1.md`):** ceremony
clean; cursor what-comments → typed-table demand; escape census cleanest
to date (1 ensure + 1 mirror block); domain scan LOG-only; noticing → the
displacement-grammar ask.

**Pass 1 — 3(b) (all lines run):** FIVE zero-byte fixes both twins (2 false
claims — "word count", "longword count"; 3 uncommented magic numbers; +
the cold-reader register map at the movem preload). All other claims
verified (C1 later confirmed the 264-cycle figure EXACT).

**Pass 1 — step-4 (all adjudications named):** ruling-3 hoist EXECUTED;
row-1482(d) NOT-BUILD demand +2; set_vdp_reg NOT-BUILD (census CORRECTED
by the panel to 3 sites — adoption debt); controller-clone NOT ADOPTED
(taste gate); vdp_reg/bytes_to_lcnt refinement-typed (macro-port rule);
align-2 adjudication recorded (A1-4 catch: the record hole, not the call).

**Pass 1 — step-5 (FULL interrogation; heat: COLD, runs once):** every line
named in the pass-1 note; threshold ruling NO CUT beyond the wave item;
two hardware questions routed to C3 by name. The WAVE ran as pass-1's
step-5 action per ruling 2 (sole-item addendum honored).

**Pass 2: EMPTY at all three steps** (the wave/hoist opened nothing; new
comments re-audited) → dry claim → panel.

## PANEL ROUND (A1+B1+C1+C2+C3 — five lenses, synchronous, read-only, one round)

**DRY STOOD** (t21/t22 bar: adjudication yielded comments both twins, one
ruling-completion adoption, ledger rows, and record corrections — zero
algorithmic/optimization rework). Every finding adjudicated:

- *A1 (7):* (1) the row-1046 kept-width citation-comment class = the file's
  biggest remaining comment-as-compensation → LEDGERED as a compiler ask
  (demand +7, strongest single-tranche data the class has). (2) sharded
  4-line comments → consolidated to block form. (3) header over-claimed
  VDP_CTRL's comptime need → corrected. (4) align-2 adjudication missing
  from the record → record corrected (the A1 class: record hole, code
  fine). (5) twin-narration align comment → de-narrated. (6) "all 68K
  registers" vs a7 → clarified both twins. (7) §-anchors unresolvable cold
  → header doc line both twins + campaign jot. Verified: gameBootHook
  mirror instruction-exact vs the real macro; cursor comments lockstep-held.
- *B1 (9):* TWO REAL inventory catches — (1) set_vdp_reg clock is THREE
  (hblank:56 IS write-through; parallax.emp:230 ships with prior demand
  data) → adoption debt ledgered + pass-1 record corrected; (2) the
  ruling-3 hoist stranded parallax's VDP_MODE3_OFF (boot was the 3rd file,
  not 2nd) → **hoist COMPLETED at the panel** (byte-neutral; row 47
  amended). Plus bytes_to_lcnt census corrected (6+ sites/3 files — row 48
  amended) and the VDP_REG_CMD/STEP vocabulary jot. Verified correct:
  controller-clone ruling, stop_z80 usage, PSG derive, sr writes inline,
  refinement typing.
- *C1: ENDORSE, zero corrections.* 264-cycle YM delay EXACT (25×10+14);
  RAM clear ~360k cyc ≈ 2.8 frames, mostly DMA-hidden; the fill is fully
  hidden by the parallel work (47ms clear vs ~21ms fill); clr.w is EXACTLY
  cycle-equal (16cy = 16cy, −2 bytes) — the wave's "smaller, not slower"
  is precise, not approximate; all three polls minimal.
- *C2: ZERO real bugs.* Cursor walk re-derived byte-exact against the
  table with the wall pinning every waypoint; d0=0 invariant endpoints
  pinned (ends BY DESIGN at the HW_VERSION read); wrapping clear + SP
  situation sound (stack unused until after the movem reload); keyoff
  values/counts exact; CC-clobber scan clean; all four boot-path
  combinations traced sound; movem excludes a7 ✓. THREE ledger rows:
  the YM busy-pacing compromise (shared with C3), the ifdef-vs-==1
  config-guard divergence (campaign jot), the reset-entry-conditional
  guard note (comment shipped).
- *C3: ALL-CLEAR on the two named questions + its own TMSS charter item.*
  (1) PSG-in-fill-window: SAFE but the comment's topological claim was
  the wrong argument — the guarantee is TEMPORAL (47ms RAM clear dominates
  the ~21ms display-off fill); invariant comment SHARPENED both twins with
  the restructure hazard named. (2) interrupt bring-up airtight (stale-F
  immediate-fire at unmask fully handled). (3) Z80/YM handshake correct —
  the 264-cycle delay is on the RIGHT pulse (second reset LOW), the short
  first pulse is REQUIRED (Z80 RAM inaccessible under reset). (4) YM keyoff
  burst: writes mostly swallowed on silicon (~3.5µs vs ~25µs busy) —
  accepted compromise, comment shipped both twins naming the reset pulse
  as the real silence guarantee. (5) TMSS: warm-boot's pre-handshake VDP
  status read is safe on every reset path (TMSS ROM precedes the cart);
  comment shipped. Zero oracle probes needed.

## Step-6 corpus sweep (enumeration, per-site outcomes — EXECUTED)

1. **moveq-deferral invalidated-assumption sweep**: parallax.emp:32's "a
   link-time extern difference cannot defer there" is now FALSE →
   comment CORRECTED (the mirror STAYS as the chosen spelling — named
   count + drift-lock beats inline extern arithmetic); sound_api's
   "out of moveq's signed range" comments remain TRUE (and now enforced
   by the ImmSigned8 window). No other cannot-defer claims in the corpus.
2. **`.b`/`.w` imm-deferral vs kill row 10**: the row's OWN kill condition
   TRIPPED (t23 shipped `.b`) → row amended with the SPLIT outcome: 5
   untyped sound_api mirrors retire at next touch; the 2 TYPED SfxId
   mirrors are BLOCKED (they carry F1 enforcement a bare link name
   cannot — typed-extern grammar is the remaining dependency).
3. **movem pinned-abs seam**: boot's own lea+movem is the cursor protocol,
   not a workaround; no prior file spells a movem-abs workaround.
   NOT-AN-INSTANCE elsewhere.
4. **Shadow-offset block**: swept to completion at the panel (hblank +
   boot + parallax = all three inline spellers; class now EMPTY).
5. **Refinement-typed comptime fns / forward pc-rel / oracle-fed parity
   technique**: existing capabilities newly exercised or proof techniques —
   nothing to retrofit.

## NEITHER-BUCKET HEADLINES

- **The imm-deferral family is now WIDTH-COMPLETE with three distinct
  semantic windows**: Value8 (unsigned ext-word byte), ImmWord16Be (the
  word union), Value32Be (long), and the new ImmSigned8 (moveq's
  opcode-embedded signed byte) — each window matched to its instruction
  class per the t20 rule; a moveq of 200 and a move.b of 500 both fail the
  LINK loudly instead of mis-assembling. Kill row 10's kill condition
  tripped as a direct consequence.
- **The wave was the row-1257 bar's first full LIVE execution**: −2 at the
  FIRST region slid 77 pins, 30 resume orgs, and the Z80 blob anchor; the
  sweep caught exactly two stale fixture classes and MIGRATED them to
  pins-derived expressions — the migration candidate the row named is now
  partially executed and self-extinguishing.
- **The panel's B1 lens caught the porter's own ruling-execution gap**: the
  ruling-3 hoist was executed against an incomplete inventory (parallax's
  MODE3 mirror missed); the panel completed the ruling byte-neutrally. The
  second-set-of-eyes record now stands at every tranche since t18.
- **The oracle-fed twin-parity class grew a shape-divergent variant**: at
  sound-off the boot region is 2 bytes SHORTER (moveq vs move.w #imm), so
  the parity arms derive base/len from the oracle world itself — the first
  parity gate whose region geometry differs from every pinned shape.
- **boot.emp closes the engine/system directory's code files** and inherits
  a pure-callee world (all 9 callees .emp-owned, zero extern decls in the
  file, zero ownership flips owed).
