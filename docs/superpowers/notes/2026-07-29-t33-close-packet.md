# 2026-07-29 — t33 close packet (the sound_fm port — rung-2 CONTRACT-model GRADUATION)

Status: **CLOSE PACKET** — merge gate open (overseer countersigned checkpoints a + b).
Porter = Opus subagent, direct-dispatch (Fable). Target = the SECOND resident-blob Z80
CODE port, the INVARIANT-HEAVY half of rung-2 item 9: `engine/sound/sound_fm.asm`
(998 L, 21 labels/procs) → `sound_fm.emp`. Where t32's psg was the contract system's
ACCEPTANCE corpus (63→0 after a finisher wired three checker gaps), t33's fm is its
GRADUATION: the checker was fully wired before its SECOND corpus arrived, so fm's honest
contract set verified DIRECTLY, **0 firings**, no finisher pass needed.

## 0. Bars (overseer-countersigned at checkpoints a + b)
- Branches `port-tranche33` both repos, off the post-t32 masters. Tips: **aeon `90b470c`** /
  **sigil `b194880`**, clean.
- **Canonical ROMs UNCHANGED by this tranche**: plain **`85111814`/421041** · debug
  **`eb5e94be`/429102**. The fm `.emp` adds ZERO (not wired into any build — the
  `z80_sound_driver.asm` blob still includes the canonical `.asm`; the fm window
  $12C3/$1341 is game-bank-independent). Verified by the porter's dual rebuild after EVERY
  edit AND the overseer's.
- **Strict-paired 2788/0 (1 ignored)** with `AEON_DIR` at the branch tree (overseer-
  countersigned) — +5 fm oracle tests over the 2783 baseline.
- **EXPECTED BYTE MOVEMENT: ZERO — HELD** (the `.asm` twin stays canonical; blob-precedes-
  engine).

## 1. What landed
**Porter item-9b (byte identity + the full contract set, checkpoint a):**
- aeon `9ccd89d` — `sound_fm.emp` (21 labels/procs) with the FULL rung-2 register contract
  set machine-verified, **0 firings** (no under-wiring surfaced — the checker was complete).
- sigil `c6ee456` — the `sound_fm_port` WINDOWED oracle (5 tests, byte-identical BOTH shapes,
  925 B) + kill row 71.

**Rulings + loop (checkpoints a→b):**
- aeon `330ee4f` / sigil `1430c2a` — ruling 1 (typed params on Fm_TransposeClamp only) +
  ruling 2 (Fm_ReparkDac conservative `clobbers(af)` + comment) + step-3 ledger records.
- aeon `90b470c` / sigil `b194880` — the 3 dry-panel byte-neutral fixes + panel records.

## 2. Byte-delta table — ZERO
| Commit | Repo | Byte delta |
|---|---|---|
| 9ccd89d · 330ee4f · 90b470c | aeon | 0 (`.emp` not wired into any build; oracle-only + byte-neutral edits) |
| c6ee456 · 1430c2a · b194880 | sigil | 0 (oracle + kill row + ledger + close packet) |

Dual build **85111814/421041 · eb5e94be/429102** unchanged at every step.

## 3. THE GRADUATION — headline: 0 → 0 (pure-port premise HELD)
The full contract system is LIVE and machine-checked on all 21 fm procs, byte-identical both
shapes, with ZERO firings — DIRECTLY, no finisher pass. This is the rung-2 system's proof
point: **the wall t27/t32 kept hitting ("the satellites demand nothing new" falsified) did
NOT recur.** fm demanded no new operand or contract form; every form was already wired
(T1/psg). Contrast:

| | t32 psg (ACCEPTANCE) | t33 fm (GRADUATION) |
|---|---|---|
| Full-set firing arc | 63 → 0 (finisher wired 3 gaps) | **0 → 0** (checker complete) |
| Demanded features | 1 (bare-symbol-imm8 fold) | **0** (pure-port premise HELD) |
| Contract gaps surfaced | 3 (§13.5-A/B/C) | 0 |

**Non-vacuity proven the t24 way:** an injected false `preserves(hl)` on Snd_ChanClass fires
`[proc.preserves-unverifiable] 'h'/'l' is written and not restored` (the move idiom modeled),
reverted — the byte-gate positive-control rule applied to contracts.

### 3.1 The contract set (21 procs + 1 extern, all verified)
`invariant: preserves(ix)` inherited + verified on all procs; `clobbers`/`out(<reg>)`
(incl. the `af`→`out(a) clobbers(f)` split on Fm_ChSel; multi-out `out(b,c)` /
`out(d,e)` / `out(hl, carry: music)`); `preserves`; `falls_into`
(Fm_NoteOn→Fm_NoteOnFreq→Fm_NoteOnFreqExact, a 3-proc chain); `extern proc Mod_ReArm`.

### 3.2 The trust-conversion (psg's biggest extern trust closed)
psg TRUSTED `extern proc Snd_ChanClass () preserves(bc, de, ix)`. fm DEFINES it at
**$12EE (plain) / $136C (debug)** — the EXACT addresses psg's oracle used as equ carriers —
and the checker now PROVES `preserves(bc, de)` explicitly + `ix` via the invariant,
confirming `hl` is NOT preserved (the `push ix / pop hl` MOVE idiom). Of psg's 3-proc trust
surface (t32 §5.2), Snd_ChanClass is now CHECKED; Mod_ReArm remains extern-trusted (fm's one
external call), Mod_Advance is not called by fm.

### 3.3 The depth-4 LIFO pass (the census-flagged hard test)
The sibling proof's pair-slot stack model verified `preserves(ix)` through fm's real nesting,
**no checker gap**: Fm_PatchTlGroup reaches stack DEPTH 4 (`push bc/de/hl/ix … pop hl …`)
inside a `djnz` loop; Fm_PatchOpGroup depth 3 (psg's deepest was 1); the `push ix / pop hl|bc`
MOVE idiom + `push af/pop af` bracket all verify; per-iteration balanced brackets converge at
the loop-head join.

## 4. The 3 dry-panel catches (A1+B1+C2+C3; C1 inactive on named basis)
The panel earned its keep — three real findings the porter walked past, all fixed byte-neutrally
(oracle 5/5, 925 B unchanged):
- **C2 — a THIRD header over-claim:** Fm_WriteFreq over-claims `hl` too (never written; callees
  preserve it). Tightened → `clobbers(af,bc) preserves(de,hl)`, checker verifies. C2's three
  mandatory re-derivations (depth-4 LIFO / Snd_ChanClass move-idiom / a corrected-de proc) all
  independently CONFIRMED.
- **C3 — stale hardware prose:** the header (carried from the .asm) claimed the DAC loop
  "re-selects $2A EVERY PASS" + "de=$4001 by construction" — FALSE against z80_sound_driver.asm
  (parks $2A once at init; the Timer-A tick reloads de). Code disciplines sound; the stated WHY
  stale. Corrected byte-neutrally (the .asm twin carries the same stale prose — an at-next-touch
  fix; lockstep is byte-level not text-level).
- **A1 — cold-reader:** Fm_NoteOnFreqExact's header under-described the single key-on chokepoint;
  now names the fill reload / vol-env reset / key-off-first EG retrigger. 6 ceremony/construct
  candidates → LOG-only (byte-locked).
- **B1 — refutation:** A1's "data-table-as-proc is a NEW shape" REFUTED — RegDeltaGroupBase's
  `pub proc () clobbers() { dc.b }` is the CANONICAL corpus idiom (seq_opcode_tab/dac_sample_tab).
  `out(carry:)` precised as established vocabulary, not fm-new.

## 5. THE HEADER-ACCURACY CENSUS (the running scoreboard rung 3 inherits)
Across the **two contracted files — 36 procs total (15 psg + 21 fm)** — the machine checker
caught **6 header over-claims** (psg 3 [t32] + fm 3 [t33]), ALL in the SAFE direction
(over-claiming `clobbers`, i.e. callers assume worse), ALL machine-corrected. **Zero unsafe
under-claims** (a false `preserves` would have fired the checker). The 6 affected procs (psg 4 /
fm 2) leave **30 of 36 headers accurate as written = 83% accuracy** on 15-year-old hand headers.

| File | Proc | .asm header | Corrected contract | The lie |
|---|---|---|---|---|
| psg | Psg_EnvCursorReset | `Clobbers af` | `preserves(af, …)` | clobbers NOTHING (two `(ix+d)` imm stores) |
| psg | PsgVolEnv_Resolve | `Clobbers bc` | `preserves(c)` | only `b` (the djnz counter) dies |
| psg | FmVolEnv_Resolve | `Clobbers bc` | `preserves(c)` | (the FM mirror — same lie) |
| psg | Psg_EmitDivisor | `Clobbers af,bc,de` | `preserves(b,de,hl)` | `de` push/pop-bracketed + `b` survive |
| **fm** | **Fm_NoteOff** | `Clobbers af,bc,de` | `preserves(de,hl)` | `de` survives (absolute-YM addr; callee-preserved) |
| **fm** | **Fm_WriteFreq** | `Clobbers af,bc,de,hl` | `preserves(de,hl)` | `de` AND `hl` survive (C2 caught the `hl`) |

(Counting note: 6 works both ways — 6 over-claim facts [psg 3 + fm 3, folding psg's VolEnv
mirror-pair as one class and splitting fm Fm_WriteFreq's two regs], and 6 affected procs [psg 4
+ fm 2]. Rung 3's 74-proc corpus [sequencer 51 + sfx 23] inherits this scoreboard.)

## 6. Corrections list (brief/design/census errors caught — tree wins)
- **(a) Census §5.1 "22 routines" → 21 labels** (incl. the RegDeltaGroupBase data table + the
  Fm_NoteOnFreqExact alias entry). The "6 Out-hdrs" matched EXACTLY; the "20 Clobbers-hdrs"
  count included the corrected over-claims.
- **(b) Brief §1 shape-variance REFINED** — fm is *more* shape-invariant than psg: its ENTIRE
  external link seam (Mod_ReArm + 4 banked tables) is shape-invariant, because fm defines
  Snd_ChanClass internally (psg's shape-varying extern is gone). The ONLY shape-variance is the
  section's own re-based internal call/jp targets — the fm oracle's `link_seam` needs no shape
  parameter (psg's did).
- **(c) Design §2.3 typed-OUT over-promise** — the spelling `out(hl: u16)` is NOT implemented;
  `out(<reg>: X)` is the flag-result grammar (`[proc.out-flag-invalid]`). The typed PARAMS
  `(a: u8, c: u8, ix: *SeqChannel)` DO bind (item-4's producer works on live `(ix+d)` code, not
  just the synthetic fixture), so Fm_TransposeClamp took the typed params with an untyped
  `out(hl)`. Ledgered as the typed-register-out grammar ask.

## 7. Overseer rulings applied (recorded)
- **Ruling 1** — typed params adopted on **Fm_TransposeClamp ONLY** (the design-named site);
  the other 20 procs stay `()` until a rung needs them. Byte-neutral.
- **Ruling 2** — Fm_ReparkDac keeps conservative `clobbers(af)` (f survives — only `ld`
  instructions — but the checker doesn't track flag writes, so a lone `preserves(f)` would be
  unverifiable manual-honor); a site comment records it. **Demand 1 ledgered** (f-tracking in
  `z80_writes`, this site).

## 8. Kill-list + ledger state
- **Kill row 71** (sound_fm twin) — present-tense: the full contract set LIVE (0→0), the trust-
  conversion, the depth-4 LIFO, the 3 header over-claims (Fm_NoteOff de; Fm_WriteFreq de+hl); the
  oracle = the sole drift guard until the scale-2 resident-blob seam.
- **Ledger** (sections "t33 fm port" + "t33 fm dry-panel"): the graduation arc, the LIFO pass,
  the trust-conversion, the corrections, the typed-out grammar ask, demand 1 (f-tracking), the
  YM-write-preamble + 6 log-only construct candidates, the first in-file `out(carry:)` locus.

## 9. Residue for rung 3 (sequencer + sfx — the interpreters)
1. **The header-accuracy scoreboard** (§5) — rung 3's 74 procs (sequencer 51 + sfx 23) extend it.
2. **`out(carry:)` cross-proc must-use** — declared/valid in psg+fm; the `[call.flag-result-unused]`
   credit first exercises end-to-end when the sequencer ports (its `jr c/z` consumers live there).
   fm added the first IN-FILE producer+consumer (Snd_ChanClass → Fm_PatchPtr/Fm_SetVolume), but the
   oracle uses `lower_module` only, so the cross-proc flag pass is unexercised there.
3. **The `ex (sp),hl` trampoline** (sound_sequencer.asm) — the preserve-proof's REPRESENTED loud
   bailout goes LIVE at rung 3.
4. **The extern trust surface** — Mod_ReArm + Mod_Advance convert to verification when the
   sequencer ports (they are defined there).
5. **Demand 1 (f-tracking)** + **the typed-register-out grammar** — ledgered checker/grammar asks.
6. **The construct candidates** — the YM-write preamble + the saturating-fold/clamp/operator-loop
   scaffold shapes (log-only, byte-locked; evaluate at the whole-ROM seam or Spec-5).
