# 2026-07-29 — t40 close packet (the z80_sound_driver port — rung 4, THE LAST Z80 CODE FILE)

Status: **CLOSED at checkpoint (b), gate (c) open.** Overseer own-ran both checkpoints
from the branch tips. Three-porter tranche: the step-0 DESIGN GATE + the T-state
capability ENGINEERING (porter-1, endorsed + countersigned at 2871/0) + this byte-
existential TRANSCRIPTION + step-2 + panel (this porter). The rebase, merge, provenance,
roadmap, and sweep are the overseer's.

## 0. Bars (overseer-countersigned at checkpoints a + b)

- Canonical UNCHANGED: plain **`4b66cace`/421041** · debug **`1c256b3b`/429102** — own-run
  EXACT at both checkpoints. **Byte movement ZERO on the blob FRONT** (the existential bar:
  the driver is offset $0000, so one byte here slides the entire corpus — it did not move).
- Strict suite **2882/0 (1 ignored)**, `--no-fail-fast`, own-run at both checkpoints
  (= the 2871/0/1 capability bar + 7 `z80_sound_driver_port` gates + 4 new `pad_to_cycles`
  tests).
- Branch tips: aeon `port-tranche40` (`z80_sound_driver.emp`: transcription + derived pads
  + C3 prose fixes) · sigil `port-tranche40` (`z80_sound_driver_port.rs` + the
  `pad_to_cycles` capability + notes + kill row).

## 1. What landed

- **`engine/sound/z80_sound_driver.emp`** — the faithful 1381-byte transcription of the
  blob front (20 procs + the framing origin + the $0038 RST-38h vector gap). Full contract
  set (the sequencer PER-PROC shape — NO module `invariant: preserves(ix)`); the 4 inbound
  trust conversions stated at each def; 4 outbound windowed-oracle externs; the three
  cycle-balance `ensure`s; the two DAC timing pads DERIVED via `pad_to_cycles`; di/ei +
  de=$4001 + reg-$2A + the resident-code/bank-contract disciplines as C3 module-header prose.
- **`crates/sigil-cli/tests/z80_sound_driver_port.rs`** — the dual-shape windowed oracle
  (7 gates): both shape byte-gates (1381 B each), `both_shapes_same_size`,
  `plain_and_debug_shapes_differ` (9-byte delta), 2 t24 doctor positives (link + const), 1
  doctor-both-equal. Twin = the driver body sliced from the `.asm` (after `phase 0`, before
  the engine includes); const seam (125 syms from s4.lst) + the 4 call targets as per-shape
  equ carriers.
- **`pad_to_cycles`** — the derived-pad capability (step 2; `crates/sigil-frontend-emp`).

## 2. Byte-delta table — ZERO

| ROM | before | after | Δ |
|---|---|---|---|
| plain `s4.bin` | 4b66cace / 421041 | 4b66cace / 421041 | **0** |
| debug `s4.debug.bin` | 1c256b3b / 429102 | 1c256b3b / 429102 | **0** |

The `.asm` stays CANONICAL in the build; the `.emp` is proven by the windowed oracle only
(the seam sub-tranche owns whole-ROM placement). Every derived-pad edit and every C3 prose
edit was byte-gate-verified GREEN both shapes before commit.

## 3. THE HEADLINE — the last Z80 code file; inbound Z80 trust → ZERO; rung 4 delivered

- **THE LAST Z80 CODE FILE.** With the driver ported, all FIVE resident Z80 code files
  (psg/fm/sequencer/sfx/driver) carry the full rung-1..4 contract set. The t32 §5.1 census
  Z80 CODE rows are DONE.
- **INBOUND Z80 TRUST = ZERO.** The 4 externs already-ported files declared on symbols this
  driver DEFINES became CHECKED DEFS here (the Mod_ReArm precedent), each verified a
  conservative subset of the real closure — stated at each def site:
  | declared in | extern decl | real def | verdict |
  |---|---|---|---|
  | sound_sfx.emp:84 | `SndDrv_SetBank () preserves(bc,de,ix)` | `clobbers(af,hl) preserves(bc,de,ix,iy)` | subset (omits iy) — SAFE |
  | sound_sfx.emp:85 | `Snd_RouteClassFlags () preserves(bc,de,hl,ix)` | `out(a) preserves(bc,de,hl,ix,iy)` | subset (omits iy+out) — SAFE |
  | sound_sequencer.emp:100 | `Snd_StartSample () preserves(ix)` | `clobbers(af,bc,de,hl) preserves(ix,iy)` | subset (omits iy) — SAFE |
  | sound_sequencer.emp:101 | `Snd_DacLookup () out(hl,carry:ok) preserves(ix)` | `out(hl,carry:ok) preserves(bc,ix,iy)` | subset (omits bc,iy) — SAFE |
  (The brief predicted "the 2 sfx externs"; the real census was **4** — see corrections.)
  The driver in turn declares its OWN 4 outbound windowed-oracle externs (Sequencer_Frame
  preserves(iy); Sequencer_StopAll / Sfx_StopAll / SfxDispatch bare) — die at the seam.
- **RUNG 4 DELIVERED (the T-state capability).** table (`z80_cycles.rs`: the driver-demand
  op subset + `span_cost` + two hard bails) + the EAGER `ensure(cycles(...))` channel
  (porter-1) + **`pad_to_cycles`** (this porter). The three REAL DAC spans verified
  **FILL 195 / DRAIN 195 / DRAINING 194** (C1 re-walked instruction-by-instruction), with
  the doctored proofs: a `+1` nop on the real `.drain` pad fires
  `DRAIN pass must equal FILL (195 T-states)`; a `jr cc` in a span fires
  `[cycles.ambiguous-branch]`; an off-table op fires `[cycles.unknown-op]`. C1's FIRST Z80
  activation, and its subject IS the T-state work.
- **THE THREE-PORTER ARC CREDITED.** design gate (census/windows/contract shapes/C3-prose
  rulings) → capability engineering (cycles core + eager channel + Findings 1-4) →
  transcription + step-2 + panel. Each countersigned at its handoff.

## 4. Corrections list (both directions — the packet owns its errors)

- **UP-CHAIN — the shape-variance refinement (design gate + the overseer's endorsement).**
  Step-0 §2 claimed the window is "IDENTICAL in BOTH shapes / the first Z80 port where plain
  and debug windows are the SAME BYTES." FALSE at the byte level: the window is the same
  SIZE (1381 B) and POSITION ($0000), but the driver `call`s three callees that shift +$7E
  in debug — Sequencer_StopAll ($CB2→$D30), Sfx_StopAll ($11AA→$1228), SfxDispatch
  ($E5D→$EDB) — so 5 call sites (9 operand bytes) diverge. Sequencer_Frame ($0565) is the
  one shape-invariant target. A window-BOUNDARY truth, not a byte-IMAGE truth. The oracle
  feeds per-shape link addresses and gates both shapes as different images. Verified in
  s4.lst vs s4.debug.lst; pointered into the design note §2 + step-1 progress note.
- **UP-CHAIN — `Z80_Sound_End = $1BFA`.** Step-0 §1.1 mislabeled $0565 as `Z80_Sound_End`;
  $0565 is the sequencer base (Sequencer_Frame). The true `Z80_Sound_End` (whole-blob end)
  is $1BFA. Corrected.
- **UP-CHAIN — the brief's 2-vs-4 trust conversions.** The brief named "the 2 sfx externs";
  the real inbound census is 4 (2 sfx + 2 sequencer). Stated in §3.
- **UP-CHAIN — the balanced-`exx` defer (design-gate ruling 3 overturned).** Ruling 3
  directed wiring the rung-2 §4.3 balanced-`exx` recognition NOW as "demanded by this file's
  honest contracts." NOT demanded: both exx-using procs are non-returning loop entries (no
  `ret`), and there is no module invariant, so the conservative exx-clobbers treatment fires
  nothing. DEFERRED to a file that actually declares a preserve across an exx pair (gap-
  ledger row; possibly never). Porter-1's Finding 3 caught it; the overseer overturned
  ruling 3.
- **RECONCILED — the exx 14-vs-15 nit.** 14 CODE `exx` sites (7 balanced pairs) + 1 COMMENT
  line = 15 grep lines. No functional discrepancy.

## 5. What each pass added

- **Step 1 (transcription):** the 1381-byte body + the dual-shape oracle + the full contract
  set + the 4 trust conversions + the three cycle ensures (literal `rept N/nop` pads at this
  stage). **Firing arc 3 → 0:** the honest-contract checker fired `[proc.out-clobbers-
  overlap]` on `Snd_DacLookup` (h,l) and `Snd_RouteClassFlags` (a) — a register is either an
  `out` result or clobbered scratch, not both. Driven to zero by dropping the out registers
  from the clobbers clause. Preserve-checker non-vacuity separately proven (injected `ld c,0`
  → `[proc.preserves-unverifiable]`).
- **Step 2 (modernize):** `pad_to_cycles` APPLIED to both DAC pads — DRAIN
  `pad_to_cycles(195, cycles(.loop,.fill_body)+10)` = 19 nops; DRAINING
  `pad_to_cycles(194, cycles(.loop,.dma_check)+cycles(.draining,.draining_pad)+10)` = 21
  nops (+1 zero-byte cut label). **DERIVED == HAND** proven by the byte gate (a mismatch
  would have been a STOP finding). The three ensures still verify, now cross-checking the
  derived pads. House format conformant from fresh transcription.
- **Steps 3/4/5 — the loop's EMPTY CIRCUITS.** Byte-frozen (STOP-not-absorb); the driver is
  the canonical `.asm`, already width-minimal, and its hot-loop `jp cc`s are load-bearing
  STRUCTURAL width pins (the cycles ensures ARE the pin — a `jr` narrowing fires the bail).
  No step-4 back-prop and no step-5 shrink available; the one modernization (`pad_to_cycles`)
  was the ruled step-2 item.
- **THE DRY PANEL (A1·B1·C1·C2·C3 — 3 fresh read-only lens subagents):**
  - **A (ceremony) → nothing new** (clause-order, brace-indent, `+k`-displacement
    parenthesization, present-tense prose all conformant vs the sequencer/sfx reference).
  - **B (corpus-pattern) → nothing new** (extern style, `export .afterPoll:` /
    `SndDrv_Sample.afterPoll`, `falls_into`, module/section shape all have corpus precedent).
  - **C1 (T-state — first Z80 activation) → CONFIRMS** the porter: FILL span re-walked
    instruction-by-instruction (22+30+30+27+44+22+20 = 195); DRAIN/DRAINING sums + both pad
    derivations re-derived; table is the documented driver-demand superset, no gap.
  - **C2 (contracts) → CONFIRMS** the porter: the DacLookup decl⊆real subset, the
    out-clobbers-overlap resolution sound, SndDrv_Sample clobbers-all correct.
  - **C3 (hardware prose) → THREE byte-neutral catches, ALL FIXED (comments only):** (b) the
    "$2A re-parked … the sole $4000-touching paths" OVER-CLAIM (omitted LoadSong/init/idle)
    → honest fuller set; (c) the "banked-code crash hazard" line (imported from
    sequencer.emp, no driver-.asm backing) → reframed as the corpus resident-code discipline
    with the .asm-backed DacSampleTable BANK CONTRACT stated separately; (d) the .asm
    header's INHERENT PITCH ASYMMETRY block (DRAIN ~29 cents / DRAINING ~38 cents, math
    C-verified) had been DROPPED → RESTORED. Byte gate GREEN after all three.
  - **Determination: DRY.** A/B nothing new; C1/C2 confirmed; C3's findings were byte-neutral
    and resolved in place (no code/contract/byte change, no re-opened cycle).

## 6. Census — t32 §5.1 ALL Z80 CODE ROWS DONE; the seam input set is FINAL

- **The Z80 code front is EMPTY.** psg (t32) · fm (t33) · sequencer (t36) · sfx (t37) ·
  **driver (t40)** — all five resident Z80 code files ported, contracted, and windowed-gated
  both shapes. Rung ladder 1→4 complete (register contracts · out/carry · falls_into ·
  invariant-or-per-proc ix · **T-states**).
- **THE SEAM SUB-TRANCHE INPUT SET IS FINAL:**
  - twin rows to retire: **70 / 71 / 78 / 83 / 87** (psg / fm / sequencer / sfx / driver).
  - the 4 OUTBOUND driver externs (Sequencer_Frame, Sequencer_StopAll, Sfx_StopAll,
    SfxDispatch) → import at the link.
  - the DRIFT-DIAGNOSTIC ledger set the seam closes: the extern-decl-vs-def drift row (t36),
    the transitive-clobbers-completeness row (t37 — `[call.clobbers-incomplete]`), the 4
    inbound driver trust conversions (t40), and the balanced-`exx` defer (t40, if a
    preserve-across-exx demand ever appears).
- **SCOREBOARD update (both CPUs):**
  - **New exercised diagnostic:** `[proc.out-clobbers-overlap]` — first fired in the sound
    corpus at t40 (2 procs: DacLookup h/l, RouteClassFlags a), self-corrected in the .emp.
  - **Header-accuracy tally:** the driver's `.asm` headers were HONEST — ZERO header lies
    (unlike psg's 3, fm's 2 over-claims, sfx's iy under-claim). The out-clobbers-overlap was
    an `.emp`-AUTHORING catch (a new diagnostic exercised), not a `.asm` header lie. Sound
    Z80 header-lie tally stands at psg 3 + fm 2 + sfx 1 = 6 corrected over the corpus; the
    driver adds 0.

## 7. Kill-list + ledger state

- **Kill row 87** (driver twin — `z80_sound_driver.asm` canonical body; seam sub-tranche
  kill). *(Numbered 84 on-branch; renumbered to 87 at the overseer rebase — t39 merged rows
  84/85/86 while t40 was in the loop; first-merged keeps the numbers per the t32/t35
  precedent. The seam row-set is 70/71/78/83/87.)*
- Gap-ledger rows this tranche: the balanced-`exx` defer (step-1, up-chain correction +
  deferred rung-4 wiring). `pad_to_cycles` is IMPLEMENTED, not a gap — recorded in the
  progress note, not the gap-ledger.

## 8. Overseer rulings applied (recorded)

- Surface spelling `ensure(cycles(span) == N, msg)` (message required); the branch bail HARD
  (`[cycles.ambiguous-branch]` / `[cycles.unknown-op]`); di/ei = C3 PROSE (no lattice);
  jp-cc = the STRUCTURAL pin the cycles ensures enforce; balanced-`exx` DEFERRED (ruling 3
  overturned). `pad_to_cycles` = the ONE ruled step-2 modernization (derived==hand,
  byte-gate-proven).

## 9. Residue → the seam sub-tranche (input set FINAL) + T1 + the generator

After t40 the campaign's port phase has ONE structural body left: the SEAM SUB-TRANCHE that
links the six sound files as one native module (retiring rows 70/71/78/83/87 + the 8
driver-boundary externs + the drift-diagnostic ledger set), then the generator + T1 close
the port phase. No new Z80 code remains to port. Byte movement stayed ZERO on the blob
front throughout — the existential bar held.
