# t22 — TRANCHE CLOSE PACKET (compression cluster conversion)

**Fourth tranche under the corrected LEAN amendment.** Scope:
`engine/compression/s4lz_decompress.asm` → `s4lz_decompress.emp` (FIRST),
`engine/compression/zx0_decompress.asm` → `zx0_decompress.emp` (SECOND),
`engine/debug/compression_selftest.asm` → `compression_selftest.emp` (LAST —
the campaign's first DEBUG-ONLY region), full loop
`0 → 1 → 2 → (3→4→5)* → panel → 6`.

Branch tips at close: **aeon-t22 `2a171cf` / sigil-t22 (this packet's commit;
prior `fd92cfe`)**, bases aeon `0a17462` / sigil `453acc3`.
Branch ROMs at close: **plain `06290799`/421157 · debug `e280a49b`/429202**
(totals unchanged from canonical — the two step-2 waves moved code inside the
org-$10000-shielded engine block; PROVENANCE re-baseline due at merge).
Full paired strict at every byte-changing commit; final: **2573/0**
(baseline 2553 + 8 spelling probes + 3 s4lz_port + 2 zx0_port +
4 compression_selftest_port + 2 mixed_tranche22 + 1 repin debug_only unit).
Overseer checkpoint-(a) countersign banked (own dual rebuild + own strict
2572/0 at that point); merge-gate countersign pending.

## Scoreboard

| Workstream | Outcome |
|---|---|
| **s4lz_decompress.emp** (S4LZ_DecompressDict `falls_into` S4LZ_Decompress + TileDelta_Undo) | byte-identical FIRST compile both shapes → step 2 −4 plain-only (2 shape-divergent relaxations; twin ifdef widths) → rows 38+30 flips |
| **zx0_decompress.emp** (ZX0_Decompress) | byte-identical FIRST compile both shapes → step 2 byte-neutral → row 39 flip → `preserves(d2/a2)` §5-VERIFIED (the movem-enforced tightening the brief anticipated — provable only at the jbsr spelling) |
| **compression_selftest.emp** (CompressionSelfTest) | byte-identical FIRST compile (debug arm; plain = zero bytes by machinery) → step 2 −0x10 debug-only → the debug-only region class SHIPPED (repin `debug_only`+`plain_anchor`, TDD) |
| **THREE ownership flips** | rows 30/38/39 KILLED; decls deleted same-commit; load_art.emp now carries ZERO extern decls; flip artifacts = caller-side four-module/three-module link tests |
| **2 demanded features/fixes (TDD)** | repin debug-only regions; **assert diag-label module scoping** (the t22 mixed debug arm caught a REAL pre-existing cross-module `$diagN$` collision — first assert-bearing .emp module pair ever placed in one link) |

## Step 0 (design note `2026-07-24-t22-step0-design.md`, committed before code)

Probes at the real binding class: P1 falls_into pub-pair PASSED as-found
(the language already had the construct — collision.emp chain); P2 opaque
`*DictBase` param PASSED (with the defining-proc-params-require-types
grammar fact banked); P3 register-dest assert PASSED; P4 bare link-immediate
arithmetic PASSED as-found (my pre-fill predicted failure — measurement
corrected the note; twin-verbatim `#CSELF_PAYLOAD_SIZE/2-1` shipped); P5
repin debug_only BUILT (TDD). Trip-check: TILE_SIZE first .emp consumer →
file-local mirror + ensure (kill row 44 born); heat fence rows 1057/1074
honored end-to-end; no at-next-touch row named these files.

## Step-1 gate lists (artifacts — all EXECUTED)

**s4lz** (region $23AE/$FC plain · $243C/$200 debug at s1):
byte gates `s4lz_port::s4lz{,_debug}_region_matches_reference` (green FIRST
compile); negative probe `doctored_tile_size_fires_its_guard` (fires NAMING
the constant); region pins via repin; gate `SIGIL_EMP_S4LZ`; gate-off dual
rebuild exact; flips: `load_art_port::two_module_ownership_flip_{plain,debug}`
(+s4lz module, carrier dropped) + `tile_cache_port::two_module_tail_call_flip_{plain,debug}`
(+s4lz module, dict carrier dropped) — kill rows 38/30 KILLED naming these.
**zx0** (region $24A6/$58 both shapes): byte gates
`zx0_port::zx0{,_debug}_region_matches_reference` (green FIRST compile); no
value seam exists (self-contained — the tranche's negative-probe duty is
carried by the s4lz + selftest guards, stated); flip: the load_art_port
artifact grew to FOUR modules (row 39 KILLED); corpus closure error-gate
credits the §5-verified preserves.
**compression_selftest** (region $6FDC/$228→$218 debug; plain ABSENT,
plain_base $624A anchored at Sound_PostByte): debug byte gate
`compression_selftest_debug_region_matches_reference` (green FIRST compile);
`compression_selftest_plain_region_is_empty` pins the shape fact; the PLAIN
proof = `mixed_tranche22_rom` (full plain ROM byte-matches with the gate ON
and NO module placed); negative probes `doctored_cself_sum_diverges_from_reference`
(the CSELF value seam is live) + `fit_lock_fires_when_vectors_cross_abs_w`
(panel round); CSELF_* values PARSED from the real generated vectors.asm;
mixed acceptance `mixed_tranche22_{rom,debug_rom}` — boot's debug
`bsr.w CompressionSelfTest` (.asm→.emp) + the REVERSE data seam (.emp
consuming AS-side generated CSelf_*/CSELF_* at link).

## Byte-delta table (measured, not predicted)

| Change | Δ plain | Δ debug | Absorbed by |
|---|---|---|---|
| s4lz s1 (+both compression gates, 2 region pins, flips) | 0 | 0 | — (gate-off dual rebuild exact: 4745cbc3/0b7c4804) |
| s4lz s2 (bare Bcc + jbra/jbsr; `beq .lit_extended` + `jbra .no_literals` relax .s plain-only — spans cross the debug dict-hit assert blob; twin ifdef widths) | −4 ($FC→$F8) | 0 | repin 45 pins plain-only; engine.inc 21 plain resume orgs; repin_pins 6 baselines + changelog. CRC → 06290799 |
| zx0 s1+s2 (12-site modernization; flip 39; preserves upgrade) | 0 | 0 | — |
| selftest s1 (+debug-only machinery, mixed arms, diag-scope fix) | 0 | 0 | — |
| selftest s2 (jbsr ×3 relax + 5 CSelf leas bare→abs.w; twin lockstep bsr.s/(sym).w) | 0 (region absent) | −0x10 ($228→$218) | repin 10 pins debug-only; engine.inc selftest+sound_api debug orgs; repin_pins SOUND_API debug baseline. CRC → e280a49b |
| loop pass 1 + panel adjudication (comments, marker struct, fit-lock ensure) | 0 | 0 | — (dual rebuilds exact both rounds) |

## Step-2 filled checklist (per file — all seven items walked)

1. Branch conversions: s4lz all-bare + jbra/jbsr (−4 plain; the two debug
   `.w` retentions are ladder-selected, not pins); zx0 all-bare + jbra/jbsr
   (byte-neutral); selftest jbsr ×16 (−6 of the −0x10). `jmp .lit_end/.match_end
   (pc,dN.w)` ×2 STAY (computed targets).
2. Width pins with site comments: ZERO kept in all three .emp files (the
   shape-divergent widths live twin-side as ifdef arms, commented there).
3. Bare-symbol width rule: complete — CSelf_* bare (abs.w today, fit-locked),
   Art_Staging_Buffer bare (abs.l — outside the sign-extended window),
   CSELF_* value immediates bare (probe-pinned link-imm arithmetic).
4. Brace-indent: file-wide ×3.
5. Idiom list walked: contract reglists RANGE form ✓; label-in-immediate =
   the CSELF class ✓; Sst.field / bareword winptr/bankid / typed VDP fns /
   Sec-Act fields — not-applicable (none of the shapes occur); no
   operand-override widths anywhere.
6. Type-layer walk: `a4: *DictBase` rides the decl (marker struct declared at
   the panel); everything else LOG with reasons — d3/d4 size/dict-len, zx0
   d1 bit-queue / d2 negative-offset: shift/add-chain values, A4-i-gated; no
   existing newtype covers a moved-AND-compared value in these files.
7. Noticing: TWO — (a) the debug-only region class (shipped as machinery:
   repin `debug_only`+`plain_anchor`, shape-split engine.inc arm, plain
   proof = the mixed arm); (b) PROPOSED checklist line: "shape-divergent
   relaxations ride twin ifdef widths; the .emp stays bare" (3rd instance:
   t19 bg_anim, t21 vblank sound-off, t22 s4lz — precedent-by-memory today).

## PER-PASS: step-3 vs step-5

**Pass — steps 0-2 (per file):**
- *step-3 flavored:* the P4 pre-fill correction (bare link-imm arithmetic
  WORKS — twin-verbatim spelling); the defining-proc-params-require-types
  fact; the P6-class demanded fix (diag-label scoping) surfaced by the mixed
  arm, not a probe.
- *step-5:* the −4/−0x10 relaxation waves (size wins; C1 later confirmed the
  `beq.s .lit_extended` not-taken path as a real −4 cyc/token streaming win).

**Pass 1 — 3(a) (all lines run; `2026-07-24-t22-loop-pass1.md`):** ceremony
scan → the three adjudicated shapes (unrolls kept: stride-locked landing
slots; TileDelta re-classed at the panel to fold-idiom ADOPTION debt;
selftest triple: protocol-visibility beats dedup); escape census cleanest to
date (one ensure, wording fixed per A1-7); domain scan LOG-only; noticing →
the ifdef-width proposal.

**Pass 1 — 3(b) (all lines run):** TWO real zero-byte fixes shipped both
twins — the a1/TileDelta endpoint-coincidence comment and the `lsr #5` =
bytes/TILE_SIZE relationship. All other header claims verified (the
comment-claim audit later gained C2's dict-entry bound refinement).

**Pass 1 — step-4 (all adjudications named):** NOT-ADOPTED 14-entry unrolls
(landing-slot stride is load-bearing); TileDelta → ledgered (re-classed by
B1); selftest template NOT BUILT (size bar + protocol visibility); no dead
code; no new mirrors.

**Pass 1 — step-5 (FULL interrogation, per hot proc; heat: S4LZ shared body
= streaming block-decompress path (fenced), plain entry + ZX0 = load-time,
TileDelta = decompress tail, selftest = boot-cold debug):** every line's
outcome in the pass-1 note. Threshold ruling: **NO CUT — nothing within
reach of ≥1k cyc/f outside the fenced streaming charter**; zero named oracle
probes needed. C1 independently endorsed with cycle math (the +16cy header
claim EXACT; all relaxations wins or neutral; TileDelta 266 cyc/tile clean).

**Pass 2: EMPTY at all three steps** (pass-1 output was comments + ledger
rows) → dry claim → panel.

## PANEL ROUND (A1+B1+C1+C2+C3 — all five lenses; read-only; one round)

**DRY STOOD** (t18-t21 bar: adjudication yielded comments, one marker type
declaration, one link-assert ensure, ledger rows, and record corrections —
no algorithmic, construct, or optimization re-work; the one adoption-shaped
item was LEDGERED per the t21 A1-1 precedent). Every finding adjudicated:

- *A1 (cold reader, 7):* THREE real. (1) `*DictBase` PHANTOM TYPE — defined
  nowhere, silently accepted (a LATENT corpus find: the spelling rode in the
  row-30 decl since t16) → `struct DictBase { }` marker SHIPPED + the
  enforcement ask LEDGERED (undeclared type names in signatures should
  error — the F3-class gap made concrete; the corpus-wide phantom sweep at
  step 6 found DictBase was the ONLY instance). (2) the 8 near-identical
  addressing-mode comments + the un-locked "$8000 today" drift marker
  (CONVERGENT with C2-5) → one file-level note + the
  `ensure(extern("CSelf_Expected") < $8000)` FIT-LOCK shipped, with negative
  probe `fit_lock_fires_when_vectors_cross_abs_w`. (3) zx0's shared-exit rts
  uncommented AND the pass-1 record claimed otherwise → header + site
  comments shipped both twins; pass-1 record CORRECTED (record-accuracy).
  (4) preserves comment was process-narration → rewritten to behavioral
  facts. (5) dict-entry a0-demotion note shipped. (6) assert-zero two
  spellings — DECLINED with reason: the tst/cmp forms are twin-byte-locked
  distinct encodings; unifying costs a debug re-pin ripple for zero
  behavioral gain (logged, not silent). (7) census wording fixed.
- *B1 (corpus, 8 + 3 verifications):* ONE real — the pass-1 "rept grammar
  ask" was adjudicated on INCOMPLETE inventory: the fold-over-asm emit idiom
  EXISTS (clear_longs/dma_queue class) → re-classed to at-next-touch
  ADOPTION debt, ledgered WITH the pre-existing sibling debt sites
  (parallax 20×, entity_window 9×) and the cite-direction fix. Landing-slot
  and selftest rulings upheld (rationale amended). Verifications: TILE_SIZE
  sole name-mirror confirmed (value-level 32s noted → row 44 amended);
  symbol-reference sweep confirmed.
- *C1 (perf): ENDORSE — no contested cut.* "+16 cycles per match" EXACT
  (6+10, width-independent); all step-2 relaxations wins or neutral (the
  `.lit_extended` relax = real −4 cyc per literal-bearing token); TileDelta
  266 cyc/tile derived clean; ONE log-grade row (movem-fill variant,
  ~20 cy/tile, clobber-widening — ledgered).
- *C2 (correctness): ZERO real bugs.* Rebase arithmetic + assert direction/
  widths re-derived clean; zx0 carry-threading verified (C-vs-X distinction
  at the refill; bsr/rts CCR-transparent); the computed-unroll d8(pc,Xn)
  arithmetic re-derived exactly (no shared latent bug); save/restore
  reliances all sound (incl. tile_cache's a5/a6 and load_art's d4). FOUR
  ledger-grade: (2) the dest-bound claim UNDER-stated the dict case →
  header CORRECTED both twins (dest+dict ≤ 32766; today's deepest use
  3072); (5) abs.w growth asymmetry → fit-lock shipped + row; (6+7)
  encoder-trust invariants with no debug backstop (count-0 wild copy;
  size&31 tile-delta) → one ledger row (future debug-hardening slot).
- *C3 (hardware): ALL-CLEAR on all four questions* — VBlank full-movem+rte
  transparency over mid-flight decompress state; selftest boot placement
  safe (VBlank_Ready=0 → lag path only; Art_Staging_Buffer untouched by
  it); Z80/bus clean. ONE ledger-candidate row (debug-vs-plain lag-metric
  caution: dict-hit assert cycles can diverge prefetch coverage across
  shapes; Lag_Frame_Count carries a load-time offset — baseline-subtract).

## Step-6 corpus sweep (enumeration, per-site outcomes — EXECUTED)

1. **Phantom-pointee sweep** (the A1-1 class, whole corpus): pointee types
   in ALL `: *T` signatures = Act/Sec (engine.structs), Sst (sst.emp), u8
   (builtin), DictBase (declared this round) — **DictBase was the corpus's
   only phantom; class now EMPTY**.
2. **Diag-label scoping** — corpus-wide by construction (every module's mint
   now carries its id); zero per-site retrofits; diag.rs unit tests keep the
   empty-scope historical names.
3. **debug_only region class** — no prior ported file is debug-only;
   retrofit sites arrive with the sound_debug/debugger/error_handler ports.
   NOT-AN-INSTANCE today.
4. **Fit-lock class** (link-value window locks over movable generated data)
   — CSelf is the corpus's only .emp-consumed generated blob;
   NOT-AN-INSTANCE elsewhere.
5. **falls_into / bare link-imm arithmetic** — existing capabilities newly
   exercised, nothing to retrofit.

## NEITHER-BUCKET HEADLINES

- **The mixed arm caught a real frontend bug** (the P6 diag-label
  collision): the acceptance machinery working as designed — the first
  assert-bearing .emp module PAIR ever placed in one link exposed a latent
  cross-module symbol collision the whole prior corpus couldn't see.
  TDD'd fix, byte-neutral, overseer-endorsed.
- **The debug-only region class shipped whole**: repin `debug_only` +
  `plain_anchor` (TDD), the shape-split engine.inc arm keeping generated
  data AS-side, plain-emptiness proven at full-ROM scale by the mixed plain
  arm, and the region machinery's Region shape unchanged for every existing
  consumer.
- **Spelling selects provability**: zx0's `preserves(d2/a2)` is UNPROVABLE
  at the faithful `bsr.s` step-1 spelling (the CFG's len-3 `b*` heuristic
  walks the subroutine body's shared rts as a return path) and PROVEN at
  the step-2 `jbsr` spelling — the corpus closure error-gate then *required*
  the credit. The step-2 modernization made the contract enforceable, not
  just prettier. S2-D6 unification ask ledgered.
- **Convergent panel findings earn the headline**: A1 and C2 independently
  flagged the CSelf $8000 growth trap (fixed with the fit-lock + probe);
  C2 refined pass-1's own comment-claim audit with the dict-entry bound the
  porter's read missed (dest+dict ≤ 32766) — the panel's second set of eyes
  beat the checklist again, 5 lenses / 3 real + 1 convergent-structural.
- **Three flips, zero extern decls left in load_art.emp** — the compression
  seam is fully .emp-owned; t23's boot port inherits a pure-callee world
  (CompressionSelfTest was boot's last non-.emp callee).
