# 2026-08-06 — OVERNIGHT OVERSEER HANDOFF: finish sigil implementation

Volence's directive, verbatim intent: finish (A) the byte-changing engine
parcel, (B) Track C, and (C) the small-items sweep, so the sigil/checker era's
implementation queue is DRAINED by morning — except items parked on content
triggers, which stay parked. You are the overseer: direct-dispatch Opus
porters, read-only A/B/C lens panels before every merge, strictly sequential
precondition-gated merges, own-run countersigns on every gate. Design calls
are yours to make autonomously (Volence's standing delegation); his morning
gates are ONLY: pushing (do NOT push anything), and play-test acceptance of
arc A. Read the campaign log TAIL first — it is authoritative:
`~/.claude/projects/-home-volence-sonic-hacks-sigil/memory/spec2-progress.md`
(the 2026-08-06 tail-seams close entry is the state you inherit). Also read
`2026-08-04-overseer-handoff-3.md` (the parallel-parcel playbook; §9
especially), `2026-08-03-era-lens-loop.md`, and `porter-brief-boilerplate.md`
(prepend it to every porter brief; its foreground-shells rule is the hard
opener).

## State you inherit (verify before acting — concurrent sessions exist)

- Masters: sigil `e3c36de2` / aeon `4974bf3`, both clean, single worktrees.
- Chain 48; strict 3447/0/4 = 3451; byte bar seven targets (full CRCs:
  s4 b5ffb094 · s4.debug 57fd08f9 · demo cbddc142 · demo.debug b61f462d ·
  config_a 61e4e78e · config_b 07e3f465 · lean b92cb485); warn tiers 19/18.
- UNPUSHED: sigil 17 / aeon 3. Push = Volence's morning call. DO NOT PUSH.

## Arc A — the byte-changing engine parcel (the milestone; run FIRST)

The corpus's first deliberate byte-moving optimization pass since conversion.
Write the spec first (Fable-style ruling doc in specs/), grounded by a scout
census of the byte-changing candidates ledger rows: the LTR multiply chains
(hand chains measured 2-10 cycles from optimal in the mul-w round — the ledger
row has the sites), the mul_bounded.w loop sites, the dead moveq seed, and any
other §-indexed step-5 backlog rows marked byte-changing. Verify every row
against the current tree before dispatch (stale-plan discipline).

Bars for a byte-CHANGING parcel — the byte gate flips from identity to audit:
1. Every byte delta NAMED and explained (scripts/corpus_bytediff.sh exists);
   nothing moves that the spec did not order.
2. BEHAVIORAL identity: oracle deterministic A/B — Frame_Counter-anchored
   input scripts (never press-count), state_hash + memory_hash comparisons,
   the Debug_Scene_Freeze(0xFF8A10)+camera-poke cache-fill identity technique,
   ObjectTest soak scene via the Game_Entry flip. OLD and NEW must hash-match
   on everything except where a cycle win legitimately shifts timing —
   adjudicate any divergence explicitly, never wave it through.
3. LAG-FRAME measurement, before and after, on the soak scene: the win must
   show up (or the parcel line explaining why not), and no scene may regress.
   Lag frames are ground truth, not profiler totals; use a trailing lag
   indicator (the beam-position gate is dead — Tile_Cache_Fill runs in
   VBlank). EMULATOR RULE: all oracle MCP work happens in YOUR foreground
   session — never from porters/subagents (it deadlocks). Porters produce
   ROMs + expected deltas; YOU run the emulator verification.
4. Full 5-site ripple: repin auto-does pins.rs only; engine.inc,
   mixed_dac_rom.rs, repin_pins.rs are HAND-edited; repin.toml only if a
   region was added. Z80-blob-precedes-engine: keep Z80 edits byte-neutral.
5. Golden refreeze to chain 49 with the anchor-primary doctrine and real A/B
   refs for every moved anchor (refreeze demands them). Rebuild canonical
   aeon ROMs after capture. One build shape per invocation (plain ./build.sh
   vs DEBUG=1 — never both from one command); verify CRCs with BOTH builds.
6. Full strict, warn tiers, panel, packet — as always.

Merge on green bars. In the close report, give Volence the morning play-test
instructions (build shapes, music needs DEBUG=1 SOUND_DEBUG_HOTKEYS=1, what
changed = timing only, what to feel for) and state plainly that the merge is
cleanly revertible if play-testing rejects it.

## Arc B — Track C (niche Option)

Spec: `docs/superpowers/specs/2026-08-03-niche-option-spec.md`. Volence's gate
is GIVEN by this directive — dispatch it. It predates B′ and the whole tail
era: a porter must re-verify every spec claim against the current tree first
and report drift to you before building (this spec is the oldest undispateched
one in the repo; expect staleness). Byte-neutral expected; standard bars +
panel. Can run as a parallel lane alongside Arc A's build phase (disjoint
files), merging after Arc A per the sequential queue — or before it if Arc A's
spec round is still in flight; measure, don't assume, any interaction.

## Arc C — the small-items sweep (drain the ledger)

Group into three lanes (adjust if your scouts find better seams), panels and
sequential merges as always:

1. **preserves-precision lane**: the partial-width preserves model (row 2142 —
   `preserves(dN.w)`-class claims; THE oldest standing demand, three tranches
   of witnesses: t34/t39 d7, define-gates d5 arms — this is the largest single
   item in the sweep, spec it properly); terminal-only mask credit (extend
   `terminal_external_tail` or successor to mid-body external tails —
   Parallax_Update's `jbra Parallax_Step5_Vscroll` is the true-but-unproven
   witness); module-invariant union in `collect_sr_mask_preservers`
   (corpus-dead, cheap); the empty `sr_mask_preservers` coupling in
   `verified_preserves_regs`/`preserve_oracle_inputs` (thread the real set).
2. **extractor/edges lane**: fold the four remaining bare-`Sym` extractor
   spellings onto `transfer_target_sym` behind measured re-runs (the family
   ledger row lists all six; preserves.rs `call_target` and calls.rs
   `direct_target` change oracle behavior — measure, paired probes);
   row 2147 (`out_verify::is_uncond_tail` re-derivation — either take the
   consult-at-consumption fix or close the row with the reasoned refusal);
   row 2150 (ISA-crate is_call/is_return/is_branch classifier — this retires
   the mnemonic-shape heuristic family for good); the djnz raw Follow-leg
   (unify through branch_edge WITH a ruled taken-edge semantics for the
   nonsensical trailing/external case — refusal is fine, decide and pin it);
   pin the SymOff-in-call-position consequence (both polarities).
3. **hygiene lane**: the Draw_Sprite tail class — adopt `preserves(a0)` on the
   ~18 games/ Main routines the t-credit census enumerated (closure-preserver
   credit, TestChurnObj_Main is the precedent; byte-neutral contract text);
   out_verify's `falls_into` false positive (ERROR-tier, safe polarity,
   ledgered at edge-split — fix the consumer to state its falls_into policy);
   the three structurally-unflippable define-free gates (layering constraint:
   sigil-harness depends on sigil-frontend-emp — fix if a clean inversion
   exists, otherwise CLOSE the row with the citation as permanently
   structural, honestly); @budget adoption census — adopt where a proc has a
   real prose/derivation customer (do not force all 52 measurable procs;
   zero-customer adoption is ceremony).

If any item turns out already-done or wrong-as-ledgered (8th+ stale-plan
catch), close it with the citation — that is a finished item too.

## STAYS PARKED (do not touch; closing these is not in scope)

The 8 OJZ `_act1` path-mismatch survivors + `parallax_configs` (await a second
act); L2/L7 human DSLs (await first content, by ruling); T2 oracle residue
(dump-to-file + set-PC — oracle repo, optional ONLY if everything else is done
and only overseer-foreground); T3/T5-T7 confirm-at-close sweeps (fold into
the close report if cheap, else leave).

## Standing rules (every one was paid for — the tail has the receipts)

Foreground-shells is the hard opener of every porter brief. Worktrees in
`.worktrees/`, seeded, baseline-proven. Merges: re-check tree cleanliness AND
master position immediately before each one; countersign = `git log
master..branch` EMPTY + tree-diff fully explained; after ANY aeon merge,
refresh every in-flight lane's aeon worktree (§9 — verify 0 own commits
before reset). Chase every test-count delta to the named function. Lens
panels: fresh read-only subagents, explicitly prohibited from tree mutation.
Packets carry no merge-state. Land-order for two-repo parcels is MEASURED per
parcel. Provenance = CRC32+size, never SHA1. Concurrent sessions can move
masters mid-arc — the precondition gate catches it; the countersign after
master-moves-ahead is `log master..branch` empty, not diff-empty. Stranded
porters are resumed by message-with-context; watch and nudge.

## At close

Append the arc-close entry to the campaign log tail (chronological, newest at
bottom; foreground the catches — Volence values them most). Leave both repos
clean, single-worktree, branches deleted, NOTHING pushed. Write Volence a
morning summary: per-arc deliverables, every panel catch, the new chain-49
CRCs and what moved and why, unpushed counts, play-test instructions, and the
honest list of anything NOT finished with the reason. If a blocker forces a
choice between shipping something unsound and leaving an item open — leave it
open and say so; the bar does not bend overnight.
