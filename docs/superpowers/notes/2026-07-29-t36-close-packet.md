# 2026-07-29 — t36 close packet (the sound_sequencer port — rung-3a INTERPRETER)

Status: **CHECKPOINT (c) OPEN — overseer-countersigned at a + b.** The `.emp` port of the
campaign's BIGGEST resident Z80 file (the music-event-list interpreter), byte movement ZERO,
the extern trust surface driven to ZERO. Overseer own-runs from the tips: plain
**37dd2bb2/421041** EXACT · debug **bbb822f6/429102** EXACT · strict **2813/0 (1 ignored)** ·
the Snd_ChanClass out(carry) fix verified in the diff · kill row 78 verified. Merge/provenance/
roadmap/sweep are the overseer's; t36 merges FIRST (t35 mid-loop takes the overseer-executed
rebase).

Tips: aeon `port-tranche36` @ `5660b33` · sigil `port-tranche36` @ `7f5ddcb`.

## 0. Bars (overseer-countersigned at checkpoints a + b)

- Canonical held EXACT throughout: plain 37dd2bb2/421041 · debug bbb822f6/429102.
- Strict paired suite **2813/0 (1 ignored)** = the 2807 baseline + 6 new sequencer-oracle tests,
  zero regressions, held across every loop step.
- **Byte movement ZERO** — the `.asm` twin is UNTOUCHED (Z80-blob-precedes-engine); every takeable
  this tranche was byte-neutral metadata/comment. STOP-not-absorb held (no byte-moving wave).
- The four other resident Z80 files (sound_psg/fm/sfx/api `.emp`) stayed READ-ONLY.

## 1. What landed

- `engine/sound/sound_sequencer.emp` (aeon) — the faithful `.emp` port of `sound_sequencer.asm`
  (2091 L / 51 labels), proven by the SCALE-1 dual-shape windowed oracle. The FIRST resident Z80
  `.emp` carrying internal `if DEBUG == 1` bodies.
- `crates/sigil-cli/tests/sound_sequencer_port.rs` (sigil) — the windowed byte gate BOTH shapes +
  the t24 positive control + doctored-both-equal (6 tests).
- The `ex (sp),hl` trampoline feature (porter-1, commit lineage 684b567) — `Z80IndSp` operand +
  sp_hazard bail + the verdict tightening.
- Two ledger rows (extern-decl-vs-def drift; the invariant(ix) Option-B upgrade path) + kill row 78
  (the sound_sequencer.asm canonical twin).

## 2. Byte-delta table — ZERO

| Region | plain | debug | Δ |
|---|---|---|---|
| whole ROM | 37dd2bb2 / 421041 | bbb822f6 / 429102 | 0 (canonical, `.asm` untouched) |
| sequencer window | $0565–$0CD7 = 1906 B | $0565–$0D55 = 2032 B | +$7E (the 16 internal `__DEBUG__` blocks) |

The +$7E is INTERNAL to the sequencer (the Seq_Trace body + 15 `call Seq_Trace` sites) — the FIRST
resident Z80 code file whose window is shape-VARIANT at a shape-INVARIANT base ($0565 both). This
+$7E is the very upstream growth that shifted psg's base $1660→$16DE in t32; porting the sequencer,
we now OWN it.

## 3. THE HEADLINE — firing arc 0 → 0 (faithful-port premise HELD)

The interpreter ported with **zero contract firings driven to zero** — like fm's graduation, all
declared preserves verified first-cut, no over-claim rejections. The pure-port premise held on the
biggest, most control-flow-dense Z80 file.

### 3.1 Trust surface → ZERO (the t32 §5.2 3-proc table FORMALLY CLOSED)

`Mod_ReArm` (:814) and `Mod_Advance` (:862) — psg's last two `extern proc` declaration-trusted
leaves — are now DEFINED + CHECKED `pub proc`s here, with machine-verified `preserves`/`out`
contracts (Mod_ReArm `clobbers(af) preserves(bc,de,hl,ix)`; Mod_Advance `out(carry: skip)
clobbers(af,bc,de,hl) preserves(ix)`). Combined with Snd_ChanClass (closed by t33), the t32 §5.2
3-proc trust surface reaches **ZERO** — no declaration-trusted sound-driver leaf remains.

### 3.2 out(carry) cross-proc credit — LIVE end-to-end (6 consumers, non-vacuity proven ×2)

The `[call.flag-result-unused]` machinery runs over the file (via the corpus contract pass, NOT
`lower_module`) and finds ZERO unused firings — all carry results consumed:
- `PsgVolEnv_Resolve` ×2 (PsgEnvUpdate entry + `.loop`), `FmVolEnv_Resolve` ×2 (FmEnvUpdate entry +
  `.loop`) — the 4 `ret c` sites the design named; `Snd_DacLookup` ×1 (Seq_HookDac `ret c`) — the 5th.
- **The 6th (the C2 panel catch):** `Snd_ChanClass`'s carry, consumed by `Seq_Op_Jump`'s `jr c`.
- NON-VACUITY proven twice by drop-probes: (a) dropping `FmEnvUpdate`'s `ret c` fires
  `[call.flag-result-unused] callee=FmVolEnv_Resolve`; (b) an `or a` redefine before Seq_Op_Jump's
  `jr c` fires `proc=Seq_Op_Jump callee=Snd_ChanClass flag=carry`. Both reverted → green.

### 3.3 The `ex (sp),hl` trampoline (demanded feature — discovery + tightening + RED proof)

- **2/3 pre-built discovery (step-0):** the ISA encoder ($E3) and the AS-frontend `(sp)` parse were
  ALREADY done; the only missing piece was the preserve-model representation (`Z80IndSp` operand +
  `sp_hazard` match) — the module was explicitly PRE-WIRED to receive it. Scope shrank from "new
  instruction + new exit model" to "one operand variant + flip one `return false`."
- **The overseer verdict TIGHTENING (ruling iii):** past `bailed_reached_return`, a unit is credited
  Verified ONLY if `!ever_clobbered[i] && invariant_units.contains(i)` — closing the silent-Verified
  hole (a locally-untouched NON-invariant register a dispatch target clobbers).
- **RED-test proof:** `z80_trampoline_untouched_noninvariant_preserve_is_unverifiable` empirically
  RED (reverting the verdict block makes `de` false-pass with no diagnostic); restored → GREEN.
- **On the REAL file:** the operand form ($E3, `E3 C9`) is exercised + byte-gated in BOTH windows.
  The bail-credit path is DORMANT here (no module invariant — see §3.4); fixture-covered.

### 3.4 The invariant(ix) ADJUDICATED FINDING (overseer-accepted as the right resolution)

UNLIKE psg/fm, the sequencer carries **NO module `invariant: preserves(ix)`** — empirically,
adding one fires EXACTLY 2 procs: `Sequencer_Frame` (irreducibly clobbers ix — it is the channel
walker `ld ix,SND_SEQ_CHANNELS`, tail-jp Sfx_Frame) and `Seq_ContinueFetch` (its `jp
Sequencer_NextOpcode.fetch` can't credit ix across an exported sub-label). Notably, WITH the
invariant present, `Sequencer_NextOpcode`/`Sequencer_Channel` and all 30 handlers PASS via the
trampoline bail-credit — so the tightening DOES work on real dispatch code; it is Sequencer_Frame's
structural ix-clobber that forbids the invariant.

- Resolution: `preserves(ix)` declared per-proc on the verifiable leaf set; OMITTED on
  Frame/Channel/NextOpcode (declaring an unverifiable preserve is a compile error — the honest-
  contract rule). The `.asm` "ix preserved" headers on Channel/NextOpcode are a CHECKER-LIMITATION
  omission, NOT a lie (C2 independently traced ix through all 30 dispatch targets — none write ix).
- **Consequence analysis (both callers rely on the true preservation):** `Sequencer_Frame`
  ADVANCES the same ix with `add ix,de` after `call Sequencer_Channel` (NOT a per-channel reload —
  see corrections §6); `sound_sfx.asm:294` (Sfx_Frame) reads `(ix+sc_flags)` + calls `Sfx_Restore` +
  `add ix,de` after. So the omitted contract is a real reliance the computed dispatch defeats.
- **Named upgrade path (ledgered):** Option-B `ex (sp),hl as dispatch(SeqOpcodeTable)` table-
  membership closure — credit basis "all SeqOpcodeTable members preserve ix" INSTEAD of a module
  invariant Sequencer_Frame cannot satisfy — would restore machine-credit on Channel/NextOpcode.

## 4. The extern-decl-vs-def SILENT DRIFT hazard (probe + the live instance C2 caught)

- **Probe evidence (overseer-demanded, reverted):** doctored psg's `extern proc Mod_ReArm` to
  falsely claim `af` preserved (the real def clobbers af). The corpus pass AND the full workspace
  strict suite fired **NOTHING** — 2813/0 unchanged. So the corpus does NOT cross-check an extern
  decl against a same-corpus `pub proc` def: a stale extern silently drifts.
- **The LIVE instance C2 caught — the textbook confirmation:** `Snd_ChanClass` was extern-declared
  here WITHOUT `out(carry: music)`, yet `Seq_Op_Jump` CONSUMES that carry (`jr c`) and the checked
  def (`sound_fm.emp:178`) DOES declare it. The under-declared extern left Seq_Op_Jump's carry
  consumer OUTSIDE `[call.flag-result-unused]` coverage — exactly the drift the probe demonstrated
  is uncaught. FIXED byte-neutral (`out(carry: music)` added); the 6th consumer is now covered.
- **Ledger row + kill:** extern-decl-vs-same-corpus-def consistency check = a demanded diagnostic
  ask (assert the extern is a valid clobbers-superset / preserves-subset of the def). Kill = the
  seam sub-tranche (owns cross-file Z80 unification; once the sound files link as one module the
  externs disappear and the def is the single source).

## 5. What each pass added (step-3 vs step-5, per the standing packet format)

**Step-1 (demanded/neither-bucket):** the trampoline feature (§3.3); the trust-conversion (§3.1);
the out(carry) 6-consumer arc (§3.2); the first Z80 `if DEBUG == 1` `.emp` (16 blocks); the
invariant(ix) adjudicated finding (§3.4).

**Loop pass 1 — step 3 findings:**
- 3(a) asks (all LEDGERED, none new-in-tranche): `(ix+(field+k))` parser workaround; the Option-B
  typed-dispatch grammar; extern-decl-vs-def drift; first-Z80-`if DEBUG==1`.
- 3(b) precision — **TAKEN (byte-neutral, machine-verified):** `Fade_Ramp preserves(ix)` →
  `preserves(c,de,hl,ix)` (leaf touching only a,b). RE-DERIVED-NOT-TAKEN: `Mod_Advance` already
  tight; `Seq_Hook*` widenings collapse to de,hl,ix with zero consumers (extern subsets stay safe).

**Loop pass 1 — step 5 findings:**
- **NO byte-takeable — byte-frozen (STOP-not-absorb held).** The file is a faithful port of tuned
  S3K-derived interpreter code; any algorithmic/cycle win MOVES BYTES = a STOP, deferred to the
  post-conversion optimization sweep. Interrogation (invariant ladder / counter-cache / guard-
  coverage / hardware cross-check) logged; the per-frame hot procs (ModUpdate, Sequencer_Channel,
  Mod_Advance) are already minimal.
- **C1 (cycle) INACTIVE — NAMED BASIS (on the record, gate-reviewable):** no in-source T-state/cycle
  annotations (the `~6,500 cyc` and `cycle-budget mandate` references are PROSE rationale, not
  per-site counts); rung 4 owns Z80 T-states; the file is byte-frozen so no cycle change is takeable.

**Loop pass 2:** the FULL 3→4→5 circuit came up empty (step 3 nothing new, step 4 built nothing —
the trace-block/reg-guard helpers stayed marginal/DEBUG/byte-neutral, deferred; step 5 no byte-
takeable). DRY → panel dispatched.

**Dry-panel (A1·B1·C2·C3, one round, weighted to step-5) — neither-bucket + takeables:**
- **C2 (correctness):** substantially CLEAN — 3 mandated re-derivations CONFIRM the porter
  (Seq_Op_RepeatEnd `clobbers(af,b)` exact; the Channel/NextOpcode ix-omission is the honest call;
  Mod_Advance/Mod_ReArm exact); CC-clobber/termination/hl-bracketing sweeps clean; all 20 preserves()
  honest. ONE actionable finding TAKEN (§4 Snd_ChanClass out-carry).
- **C3 (hardware prose):** CLEAN — 8 hardware claims VERIFIED against the resident tree (bank
  asserts + co-location/no-straddle fatals + SetBank sites; $A4/$A0 order + $2A repark; the Timer-A
  $24-$27 + $2A/$2B guard; ~59.92 Hz; $28 part-I; banked-code hazard; include order); 0 t33-class
  rot; 1 unverifiable-statically (S3K zDoModulation fidelity citations — carried identically from
  the twin, not port-introduced). ONE byte-neutral comment restoration TAKEN (Seq_Op_Lfo $22 repark).
- **A1 (ceremony):** 6 language asks ledgered (word-field access, cursor-fetch, Z80 auto-reaching
  branch, indexed-field addressing, redundant-save lint [byte-changing → deferred], data-block-not-
  proc). Confirmed the preserves(ix) omission reads correctly.
- **B1 (corpus-pattern):** 3 byte-neutral construct candidates ledgered (`ix_field_ptr` — cross-file
  into read-only sound_fm, blocked; `seq_trace` 3-variant splice — design ask; `ym_reg_guard` —
  clean 2-site build). Build-vs-ledger is the overseer's gate call.

## 6. Corrections list (the packet owns its errors — tree wins)

- **MY consequence-analysis FIRST DRAFT WAS WRONG.** The step-1-complete draft said "Frame reloads
  ix per channel" → concluded no caller relies on ix survival. FALSE: Sequencer_Frame sets ix ONCE
  (`ld ix,SND_SEQ_CHANNELS`) and ADVANCES with `add ix,de` after each `call Sequencer_Channel`, so
  it DOES rely on ix survival — as does the 2nd caller `sound_sfx.asm:294` (which I initially missed
  entirely). Corrected at checkpoint-a follow-up; the honest analysis STRENGTHENS the finding.
- **Step-0 shape-variance correction carries over:** the brief framed the window as shape-variant
  BASE / layout-invariant (the psg precedent); verified FALSE — the base is shape-INVARIANT ($0565
  both), the LAYOUT is shape-variant (+$7E internal). The first Z80 `.emp` needing `if DEBUG==1`.
- **Census `Out`-count:** 2 PROCS across 4 comment lines (Porta_Apply + Mod_Advance), not 4 procs.

## 7. Census update — the t32 §5.1 sound_sequencer row → DONE

| Metric | t32 §5.1 (indicative) | Measured | Note |
|---|---|---|---|
| Top-level labels | 51 | **51** | EXACT |
| — code procs | — | **~46** | the transcribed `pub proc`s |
| — non-body labels | — | **3** | Seq_ContinueFetch thunk; Seq_FmKeyoffChsels `dc.b`; Seq_Trace (DEBUG-only) |
| — falls-into continuation | — | **1** | Seq_RekeyRender (falls-into from Seq_RekeySingle) |
| Clobbers headers | ~25 | 24 | in band |
| Preserves headers | ~20 | 19 | in band |
| Out (carry) procs | 2 | **2** | Porta_Apply + Mod_Advance |

**Header-accuracy scoreboard: stays 36 procs / 6 over-claims — this file was FIRST-CUT CLEAN**
(zero over-claim rejections; every declared preserves verified on the first lower). NEW class
surfaced: contract-PRECISION-tightening (a header `Clobbers X` that under-states the preserved set)
— Fade_Ramp TOOK it (preserves widened, machine-verified); the other candidates (Tempo_Ramp hl,
Seq_Trace ix, Seq_Hook* de/hl, extern true-contracts) are zero-consumer and LEDGERED not-taken. The
3 Channel/NextOpcode/Frame omitted-preserves(ix) are the checker-limitation class (§3.4), distinct
from an over-claim.

## 8. Kill-list + ledger state

- Kill row **78** — `sound_sequencer.asm` canonical twin; kill = the seam sub-tranche (also closes
  rows 70/71 psg/fm twins + the extern-decl-vs-def drift ledger row).
- Ledger rows added: extern-decl-vs-def drift (demanded diagnostic ask); invariant(ix) Option-B
  upgrade path; the 6 A1 asks; the 3 B1 construct candidates; the C2 precision opportunities.

## 9. Overseer rulings applied (recorded)

- invariant(ix) omission ACCEPTED as the right resolution (a + consequence analysis + Option-B).
- The extern-drift probe demanded + run (fired NOTHING) → ledgered; psg/fm read-only, probe reverted.
- C1 INACTIVE named-basis ruling on the record.
- Byte movement ZERO / STOP-not-absorb held; the panel takeables are byte-neutral only.

## 10. Residue for rung-3b (sfx) + rung 4 (driver) + the seam sub-tranche

- **rung-3b (sound_sfx):** the struct-prefix mirror; reuses Mod* (now checked here) + Sequencer_Channel
  (the SFX shared-interp caller at sound_sfx.asm:294 — the 2nd ix-reliance site).
- **rung 4 (driver):** T-state capability FIRST (owns Z80 cycle costs; the C1 lens activates there).
- **the seam sub-tranche:** retires the sound `.asm` twins (rows 70/71/78), unifies the sound files
  as one linked module (closing the extern-decl-vs-def drift structurally), and is the kill vehicle
  for the Option-B typed-dispatch grammar (the invariant(ix) machine-credit upgrade) + the B1
  cross-file `ix_field_ptr` construct sweep.
