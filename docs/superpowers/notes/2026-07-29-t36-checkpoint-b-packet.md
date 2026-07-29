# 2026-07-29 — t36 checkpoint (b) packet (sound_sequencer, rung-3a)

Status: **CHECKPOINT (b) — the 2→(3→4→5) loop is DRY and the dry-panel (A1·B1·C2·C3) round is
adjudicated. STOP for the overseer's gate (c) + merge.** Byte movement ZERO held throughout
(STOP-not-absorb). This packet is the per-pass loop evidence + the panel round + the takeables.

Branch tips: aeon `port-tranche36` (step-1 def + Fade_Ramp widening + the C2/C3 panel takeables);
sigil `port-tranche36` (oracle + ledger + this packet). Suite **2813/0 (1 ignored)** throughout;
whole-ROM CRCs canonical (37dd2bb2/421041 · bbb822f6/429102) throughout.

**Takeables this loop (all byte-neutral, machine-verified):** (1) step-3(b) Fade_Ramp
`preserves(ix)`→`preserves(c,de,hl,ix)`; (2) panel-C2 Snd_ChanClass extern `out(carry: music)`
(brings Seq_Op_Jump's `jr c` under flag-coverage, non-vacuity proven); (3) panel-C3 Seq_Op_Lfo
comment restoration (the $22-specific repark rationale). Zero `.asm` edits — byte movement ZERO.

## The loop shape (byte-frozen file → the takeable surface is narrow)

Because byte movement is ZERO (the `.asm` is canonical; Z80-blob-precedes-engine), the ONLY
takeable class this tranche is BYTE-NEUTRAL: contract metadata, comments, and byte-neutral
constructs. Every cycle/layout win is a STOP (deferred to the post-conversion optimization
sweep), not a wave. The loop therefore converges fast.

### Pass 1

**Step 2 — house format (byte-neutral).** The fresh transcription already lands house form;
audit outcomes:
- Branch conversions (item 1): N/A — Z80 has no `jbra`/`jbsr`/bare-Bcc class; `jp`/`jr`/`call`
  stay. Nothing to convert.
- Structural width-pins / mem-to-mem `.w` (items 1-3): N/A — Z80 addressing has no .w/.l width
  selection; absolute EAs are already bare symbols (`ld a, (SND_STAT_TICK)`), the Z80 analog of
  the bare-abs-EA idiom (item 5) with no width to spell.
- Brace-indent (item 4): every `{` body (procs + the 16 `if DEBUG == 1` blocks) indents one
  level. CONFORMANT.
- Idiom list (item 5): the parenthesized `(ix+(field+k))` compound displacement is the house
  spelling (flat-form parser fix ledgered); the `ex (sp),hl` trampoline carries its forced-
  spelling comment. CONFORMANT.
- Type-layer walk (item 6): NO wired Z80 newtype layer (the types.emp family is 68k; the
  f-tracking limitation stands). Domain values here (note index, MEV opcode, sample id) live in
  Z80 registers with no newtype to adopt — not a MISS (cross-CPU gap, logged).
- Module header (item 7): states role, gate/canonical status, and EVERY hardware/bank/timing
  contract (the C3 audit surface) + the hard-coded layout. PRESENT + checkable → C3 lens.
- Symbol-resolution ladder (item 8): cross-module sound refs are `extern proc` (the windowed-
  oracle boundary, matching sound_psg.emp precedent — they are CHECKED in their own files, not
  unported leaves). Mod_ReArm/Mod_Advance are DEFINED here (trust surface → 0). The stale-extern
  drift is now a ledgered diagnostic ask.
- Honest-contract (item 9): over-claim clobbers safe; zero unverifiable preserves; all declared
  preserves machine-verified.
- `as`-bless (item 10): N/A — no typed constructions in this Z80 file.
- Noticing (item 11): no new house-format item beyond the already-ledgered `if DEBUG==1`-in-Z80
  idiom (worked first time).

**Step 3 — retrospect.**
- 3(a) language/format asks (all ledgered, none new-this-pass): `(ix+(field+k))` parser
  workaround; `ex (sp),hl as dispatch(SeqOpcodeTable)` typed-dispatch grammar (the invariant(ix)
  machine-credit upgrade path); extern-decl-vs-def drift diagnostic; first-Z80-`if DEBUG==1`.
- 3(b) precision/reads-wrong — **TAKEN (byte-neutral, machine-verified): Fade_Ramp `preserves(ix)`
  → `preserves(c, de, hl, ix)`.** Fade_Ramp is a leaf touching only `a`,`b` (no calls), so
  `c/de/hl/ix` are provably preserved; the checker verified the widening (corpus pass green) and
  the oracle stayed byte-identical. The `.asm` twin needs NO edit: its header `Clobbers af,b`
  already EXHAUSTIVELY LICENSES c/de/hl/ix as preserved (the register-contract convention), so
  the `.emp`'s explicit preserves is consistent, not a divergence.
- 3(b) precision — RE-DERIVED, NOT TAKEN (logged): `Mod_Advance` is already tight (genuinely
  clobbers af,bc,de,hl in the fnum/accum math — `preserves(ix)` only is accurate). The `Seq_Hook*`
  preserves could widen (their extern tail-callees preserve de/hl per fm/psg.emp), but the
  widening COLLAPSES to `de,hl,ix` at best (Fm_/Psg_ intersections) and NO caller of any
  Seq_Hook* relies on de/hl surviving (every call site is `call Seq_Hook* / ret` or `/ jp
  ContinueFetch` re-reading `(ix+d)`), so the widening is zero-consumer honesty at the cost of
  widening 2 externs + gate churn → NOT TAKEN, logged. Extern decls stay conservative SUBSETS
  (safe direction; psg's Mod_Advance extern likewise a valid subset of the widened def).

**Step 4 — construct pass.** Scanned for repeated/patterned emission:
- The 15 `if DEBUG == 1 { ld a, SEQEV_X / call Seq_Trace }` trace blocks (3 variants: bare,
  push/pop hl, push/pop bc+hl) — a `seq_trace(code)` helper is a candidate, but (a) it is
  DEBUG-only, (b) it has 3 bracket variants (not one clean signature), (c) the `.asm` twin keeps
  its `ifdef __DEBUG__` spelling, and (d) it is byte-neutral either way. Marginal; NOT TAKEN this
  pass — flagged for the B1 panel lens to confirm/dispute.
- The `push ix / pop hl|de` base-address idiom (multipoint / Seq_Op_OpBias / Seq_Op_PitchEnv):
  3 sites, each with different subsequent index math — no clean shared shape. NOT TAKEN.
- No `offsets`/`table`/`dispatch` construct fits a byte-frozen hand-rolled shape here (the jump
  table already lives in seq_opcode_tab.emp). Flagged for B1.

**Step 5 — optimize.** The file is a faithful port of already-tuned S3K-derived interpreter code.
Byte movement ZERO ⇒ any algorithmic/cycle win MOVES BYTES = STOP-not-absorb, deferred to the
post-conversion optimization sweep. Interrogation (per hot proc, outcomes logged):
- Invariant ladder / counter-cache / guard-coverage: the hot path is ModUpdate + Sequencer_Channel
  per active channel per frame; every gate (sc_mod_ctrl, sc_psgenv, SCF_KEYED, write-on-change
  shadows) is already minimal — no hoist/fold takeable without moving bytes.
- Hardware cross-check: delegated to the C3 panel lens (bank asserts, $A4/$A0 order, Mod-triangle
  prose, the Timer-A guard).
- **C1 (cycle) decision — INACTIVE, NAMED BASIS (flagged, gate-reviewable):** no in-source
  T-state/cycle annotations exist (the `~6,500 cyc` and `cycle-budget mandate` references are
  PROSE rationale, not per-site counts); rung 4 owns Z80 T-states; and the file is byte-frozen so
  no cycle change is takeable regardless. C1 not run; sites named (ModUpdate, Sequencer_Channel,
  Mod_Advance are the per-frame hot procs). Reversible at the gate.
- Debug-growth boundary: N/A (no new DEBUG growth — the 16 blocks are the faithful port).

### Pass 2 (dry confirmation)

Re-ran 3→4→5 after the Fade_Ramp widening: step 3 finds nothing new (the widening opened no new
reads-wrong; Mod_Advance/Seq_Hook* already re-derived), step 4 adopts/builds nothing (the trace-
block helper stays marginal/DEBUG-only, deferred to the B1 verdict), step 5 takes nothing (byte-
frozen). FULL 3→4→5 circuit empty → DRY claim, panel dispatched.

## Dry-panel (A1 · B1 · C2 · C3) — ADJUDICATION

One panel round (cost-bounded). Weighted toward step-5 (C×2). Net: **ONE actionable finding
(C2's Snd_ChanClass out-carry, TAKEN + verified) + one byte-neutral comment restoration
(C3's Seq_Op_Lfo, TAKEN); everything else is ledgered asks / confirmed-clean / not-taken-with-
reason.** No BUG, no rot, no disputed contract. Findings land here for the overseer's gate.

- **A1 (ceremony/asks) — 6 language ASKS, all LEDGERED (none takeable in-tranche).** Word-field
  `(ix+field)` load/store pseudo (subsumes the parenthesized workaround); stream-cursor fetch;
  Z80 auto-reaching branch (jr→jp); dynamic indexed-field addressing; redundant-save lint
  (BYTE-CHANGING → step-5/post-conversion, correctly deferred); data-block-not-proc spelling.
  A1 also CONFIRMED the omitted-preserves(ix) reads correctly. → feature-discovery, ledgered.
- **B1 (corpus-pattern) — 3 byte-neutral CONSTRUCT candidates, LEDGERED (deferred).** `ix_field_ptr`
  comptime-fn (cross-file incl. read-only sound_fm — blocked, seam/next-fm-touch vehicle);
  `seq_trace(SEQEV_x)` splice (3-variant DEBUG family — a design ask); `ym_reg_guard` (2 in-file
  sites — clean small build). All readability-only, byte-neutral; NOT built in this byte-frozen
  checkpoint-(b) STOP — build-vs-ledger is the overseer's gate call. B1 confirmed the trampoline
  is already-ledgered (Option-B).
- **C2 (correctness-hazard) — SUBSTANTIALLY CLEAN + one TAKEN fix.** All 3 mandated re-derivations
  CONFIRM the porter: `Seq_Op_RepeatEnd clobbers(af,b)` EXACT; the `Sequencer_Channel`/`NextOpcode`
  `preserves(ix)` omission is the honest checker-limitation call (C2 independently traced ix
  through all 30 dispatch targets — none write ix — and confirmed both callers rely on the true
  preservation); `Mod_Advance clobbers(af,bc,de,hl) preserves(ix)` + `Mod_ReArm clobbers(af)
  preserves(bc,de,hl,ix)` EXACT. CC-clobber sweep, loop-back termination, and push/pop-hl
  bracketing all CLEAN. All 20 preserves() claims independently re-derived HONEST.
  **ACTIONABLE (TAKEN):** the `Snd_ChanClass` extern under-declared `out(carry: music)` that
  `Seq_Op_Jump` consumes (`jr c`) — a 6th carry-consumer the flag-check couldn't cover. FIX:
  `extern proc Snd_ChanClass () out(carry: music) preserves(bc, de, ix)` (byte-neutral). VERIFIED:
  corpus + oracle green; NON-VACUITY proven — a redefine probe (`or a` before the `jr c`) now
  FIRES `[call.flag-result-unused] proc=Seq_Op_Jump callee=Snd_ChanClass flag=carry`; reverted →
  green. This is a live instance of the extern-drift hazard the checkpoint-a probe demonstrated;
  closing it brings Seq_Op_Jump under coverage. C2's marginal precision widenings (Tempo_Ramp hl,
  Seq_Trace ix, Seq_Hook* de/hl) NOT taken (zero-consumer; extern subsets stay safe) — ledgered.
- **C3 (hardware-timing) — CLEAN, no t33-class rot.** 8 hardware claims VERIFIED against the
  resident tree: the bank asserts (soundBankHead + the song_table/main.asm co-location/no-straddle
  fatals + the two SetBank sites), the $A4/$A0 write-order + DAC $2A repark (Fm_WriteFreq/Fm_YmWrite/
  Fm_ReparkDac contracts), the Timer-A frame-clock guard ($2A/$2B + $24-$27 refused, exact), the
  ~59.92 Hz clock, $28-part-I, the banked-code hazard, and the include order. 1 unverifiable-
  statically (the S3K zDoModulation/freeze fidelity citations — no S3K source in-tree; carried
  IDENTICALLY from the `.asm` twin, so not port-introduced — flag for an eventual oracle A/B or the
  S3K reference, not a defect). **TAKEN (byte-neutral):** C3's nuance — Seq_Op_Lfo's comment
  abbreviated the $22-specific repark rationale the `.asm` twin spells out; restored the
  explanatory clause (comment-only, byte-neutral).

**Dry verdict:** the panel returned no bug, no rot, no disputed contract. The two takeables (C2
Snd_ChanClass out-carry, C3 Seq_Op_Lfo comment) are byte-neutral and do not create new findings;
the asks/constructs are ledgered feature-discovery, not in-tranche code. One panel round (cost-
bounded per the rule). The overseer gates whether the ledgered constructs (esp. B1 #3 ym_reg_guard)
should be built now vs at the step-6 sweep.

## Evidence block (checkpoint b)

- Windowed oracle BOTH shapes green (plain 1906 B / debug 2032 B); whole-ROM CRCs canonical
  (37dd2bb2/421041 · bbb822f6/429102); strict paired suite 2813/0 (1 ignored).
- Byte movement ZERO (the `.asm` untouched; the only aeon change is the byte-neutral Fade_Ramp
  contract widening).
- Checkpoint-a follow-ups closed (consequence analysis, Option-B upgrade-path ledger, extern-
  decl-vs-def drift probe + ledger).
