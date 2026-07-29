# 2026-07-29 — t36 step-1 COMPLETE (sound_sequencer port, rung-3a)

Status: **CHECKPOINT (a) — step 1 verified GREEN.** The faithful `sound_sequencer.emp`
transcription + the dual-shape windowed oracle are done, on top of porter-1's committed
`ex (sp),hl` trampoline feature. Byte movement ZERO (the `.asm` stays canonical). STOP here
per the brief; steps 2-5 not started.

Branch tips (to commit): aeon `port-tranche36` += `engine/sound/sound_sequencer.emp`;
sigil `port-tranche36` += `crates/sigil-cli/tests/sound_sequencer_port.rs` + this note.

## Gate artifacts

- **Windowed byte gate, BOTH shapes** (`sound_sequencer_port.rs`, 6 tests, all green):
  - `sound_sequencer_matches_as_twin_plain` — plain window **$0565..$0CD7 = 1906 B**, DEBUG=0.
  - `sound_sequencer_matches_as_twin_debug` — debug window **$0565..$0D55 = 2032 B**, DEBUG=1.
  - `debug_window_is_plain_plus_7e` — the +$7E = 126 B delta == the 16 internal `__DEBUG__`
    blocks (Seq_Trace body + 15 `call Seq_Trace` sites). SAME base $0565 both shapes.
  - `plain_and_debug_shapes_differ` (shape-variance), `emp_diverges_from_doctored_twin`
    (t24 positive control — moved SeqOpcodeTable diverges; reference window re-assembled each
    run, pins-derived), `doctored_both_sides_stay_equal`.
- **Whole-ROM CRCs vs canonical** (aeon worktree, unchanged — only a `.emp` was added):
  plain **37dd2bb2 / 421041**, debug **bbb822f6 / 429102**. EXACT.
- **Strict paired suite** (`SIGIL_STRICT_GATE=1 AEON_DIR=<aeon-wt> cargo test --workspace`):
  **2813 passed / 0 failed / 1 ignored** = the 2807 baseline + 6 new sequencer-oracle tests,
  zero regressions.

## Trust-surface → ZERO (the headline conversion)

`Mod_ReArm` and `Mod_Advance` — psg's last two `extern proc` declaration-trusted leaves —
are now DEFINED + CHECKED `pub proc`s in `sound_sequencer.emp`. Firing evidence: they carry
real `preserves`/`out` contracts machine-verified by the corpus contract pass
(`contract_closure_corpus` green with the file in-corpus). The t32 §5.2 3-proc trust surface
(Snd_ChanClass [t33], Mod_ReArm, Mod_Advance) reaches ZERO.

## out(carry) cross-proc credit — LIVE end-to-end

The 4 `ret c` consumer sites — `PsgVolEnv_Resolve` ×2 (PsgEnvUpdate entry + `.loop`),
`FmVolEnv_Resolve` ×2 (FmEnvUpdate entry + `.loop`) — exercise `[call.flag-result-unused]`
end-to-end for the first time. Both resolvers are `extern proc () out(hl, carry: found)`;
the corpus test `corpus_flag_results_are_all_consumed` runs the check over the file and finds
**0 firings** (all consumed). Live-proven by probe: dropping `FmEnvUpdate`'s `ret c` fires
`[call.flag-result-unused] proc=FmEnvUpdate callee=FmVolEnv_Resolve flag=carry`; restored →
green. (A 5th consumer rides along: `Seq_HookDac`'s `ret c` on `Snd_DacLookup`, also
`out(hl, carry: ok)`.) NOTE: the credit runs in the CORPUS contract pass, not the windowed
oracle's `lower_module`.

## invariant(ix) — the ADJUDICATED finding (differs from psg/fm)

UNLIKE psg/fm, the sequencer carries **NO module-scope `invariant: preserves(ix)`**.
Empirically confirmed: adding `invariant: preserves(ix)` fires EXACTLY 2 procs —
- `Sequencer_Frame` — "ix written and not restored": it is the channel-walk DRIVER
  (`ld ix, SND_SEQ_CHANNELS`) and tail-jumps to `Sfx_Frame`; **irreducibly** clobbers ix.
- `Seq_ContinueFetch` — "ix written and not restored": its `jp Sequencer_NextOpcode.fetch`
  tail can't credit ix across an exported sub-label (oracle limitation).

Notably, WITH the invariant present, `Sequencer_NextOpcode` and `Sequencer_Channel` (and all
`Seq_Op_*` handlers) PASS — the trampoline bail-credit (invariant_units past
`bailed_reached_return`) verifies ix through the `ex (sp),hl` computed dispatch. So the
tightening DOES work on real dispatch code; it is Sequencer_Frame's irreducible ix-clobber
that forbids the module invariant. Resolution:

- `preserves(ix)` is declared PER-PROC on the verifiable leaf set (Tempo_Ramp/WriteChanMods,
  Fade_Ramp, Porta_Apply, ModUpdate, Seq_RekeySingle/Render, PsgEnvUpdate, FmEnvUpdate,
  Mod_ReArm, Mod_Advance, Mod_ApplyVibrato, MacroTick, all 5 Seq_Hook*, Sequencer_StopAll).
- The 3 loop-entry procs OMIT `preserves(ix)`: `Sequencer_Frame` (genuinely clobbers ix),
  `Sequencer_Channel` + `Sequencer_NextOpcode` (route through the trampoline; ix IS preserved
  in fact but the computed dispatch defeats static proof absent a module invariant — declaring
  an unverifiable preserve is a compile error, forbidden by the honest-contract rule). The
  `.asm` headers CLAIM "ix preserved" for those two; the `.emp` honestly omits it — a
  checker-limitation omission, NOT a header lie.

## Trampoline feature on the REAL file

- Operand form (`Z80IndSp` → `$E3`): EXERCISED + byte-gated — the `.coord` dispatch emits
  `E3 C9` identically in both windows. Carries the endorsed forced-spelling comment
  `// [dispatch.trampoline: SeqOpcodeTable — module-internal targets; credit past bail = module invariant only]`.
- Bail + tightened invariant-credit: DORMANT on the real file (no module invariant → the loop
  procs declare no preserve → the sp_hazard bail is not invoked). Covered by porter-1's
  committed fixtures. See the adjudicated finding above for why the invariant can't be adopted.

## Header-accuracy scoreboard

No new over-claim corrections (the psg/fm class: header claims clobbers(X) while X survives).
Every `preserves` I declared verified; zero false-preserve rejections on the first cut. The
scoreboard stays **36 procs / 6 over-claims**. The 3 loop-entry omitted-preserves(ix) are a
distinct class (checker limitation, not a lie). Contract-PRECISION tightening (several procs
— Fade_Ramp, Mod_Advance, the Seq_Hook* — provably preserve more than declared, e.g. c/de/hl
untouched) is a step-3(b) candidate, not chased at step 1.

## STOP

Checkpoint (a). Step 1 green. No aeon `.asm` edit (byte movement zero). Next (out of scope):
step 2 modernization loop, then dry-panel, then merge.
