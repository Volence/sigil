# Checkpoint — Plan 7 #9 (9a+9b) MERGED to master

2026-07-08 (Fable controller session). Volence approved merge at the checkpoint;
master `c29027f` (--no-ff merge of plan7-item9, pushed), worktree removed, branch
deleted. Post-merge: feature suites + module_resolution + both acceptance pins green,
clippy --workspace clean. Baseline unchanged: exactly the 4 allowlisted sigil-harness
reds (aeon sound-driver strlen drift).

Shipped: 9a dispatch inline bodies (R9a.1-6) + 9b script/yield (R9b.1-12) — plans,
RED-evidence notes, and the whole-branch adversarial verdict are in
docs/superpowers/{plans,notes}/2026-07-08-item9*. Spec §5.5 update, NEW §5.6, D2.24,
§10 inventory are in the empyrean WORKING TREE (uncommitted, Volence's cadence).

## Open threads for whoever works next

- **#7 banks is NEXT** — needs Volence engaged; bring ledger **L-H.1 / S2-D13(e)**
  (cross-section origin staleness under growth) to that conversation, it re-opens
  section placement anyway.
- **Rule-of-three extraction due:** the table-emit shape (eval → stream_data →
  define_label → emit_data) has three verbatim instances (lower_offsets_item,
  lower_dispatch_item, lower_script_item) — extract a shared helper next time that
  seam is touched.
- **9c** (value yields, for, break, script-calls-script) needs no new ratification,
  just its own short design note; **9d** byte-command DSL stays gated per D9.3.
- **DX-2 (ledger):** duplicate top-level symbol collisions report at span 0 (link
  layer, pre-existing) — candidate spanned frontend pre-check.
- Known exhibit caveat: pitcher_plant_script's windup is 1 frame shorter than the
  proc version (documented at the transition site + in the 9b notes).
