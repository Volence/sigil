# Completion handoff — Plan 7 #3 (bidirectional offset-table `offsets`) DONE, awaiting checkpoint

Written 2026-07-06 after implementing backlog #3. Supersedes the pre-work handoff
`2026-07-06-spec2-plan7-item3-offset-table-handoff.md`.

## Status: IMPLEMENTATION COMPLETE, UNMERGED (milestone → Volence `--no-ff` checkpoint)

Branch `offset-table` (worktree `sigil/.worktrees/offset-table`), off master `232bc6e`, 13 commits,
HEAD `29708a6`. **1024 workspace tests pass, `cargo clippy --workspace --all-targets -- -D warnings`
clean.** Whole-branch adversarial review = **zero defects**.

## What shipped: the `offsets Name { Variant: target, ... }` construct

- **FORWARD:** emits one BE word per member = `target - Name`. New `FixupKind::RelWord16Be`
  (`sigil-ir`), resolved + signed-word range-checked (±$7FFF, overflow = compile error) in
  `sigil-link::apply_fixup`. `Cell::RelOffset{base,target}` (value.rs) → `stream_data` (lower/data.rs)
  → `lower_offsets_item` + `eval_offsets_with_root` (lower/mod.rs, layout.rs). Base label at the
  table's first byte. Member target's symbol name extracted BY SHAPE (a label is a link-time
  symbol, not a comptime value). 68k-BE only; Z80 diagnosed.
- **REVERSE:** `Name.Variant` = 0-based comptime ordinal (plain int, no coercion), `Name.count` =
  member count. `count` reserved; duplicate members rejected once-per-compile in
  `lower::validate_offsets`; const-alias/non-label targets diagnosed early.
- **Verified byte-identical to the independent AS front-end** (`sigil-frontend-as` folds
  `dc.w Target-Base` via multi-pass fixed-point — a genuine cross-check), forward + negative.

Docs: design `docs/superpowers/specs/2026-07-06-offset-table-design.md`; plan
`docs/superpowers/plans/2026-07-06-offset-table.md`; example `examples/offset_table.emp`.

## Checkpoint items for Volence (before merge)

1. **Merge decision:** `--no-ff` merge of `offset-table` + push (the established cadence), or changes first.
2. **Spec freeze is a DRAFT, not committed.** `empyrean` is a separate repo with your uncommitted
   WIP in `SIGIL_SPEC2_LANGUAGE.md`, and spec authorship is Fable's role — so the §4.7 text is
   drafted at `docs/superpowers/specs/2026-07-06-offset-table-spec-section.md` for you/Fable to lift
   in. It documents: the surface; forward/reverse; `count` reserved; the no-auto-pad divergence from
   AS `dc.w`; the ordinals-are-ints (not a coercing enum) rationale; the R1 data-table-only scope;
   the deferred knobs.
3. **Intentional divergence to bless:** offset tables follow §4.3 no-auto-pad — unlike AS `dc.w`'s
   word-alignment, an odd-address base doesn't silently pad (tables are word-aligned in practice).
4. **Optional cross-cutting cleanup (not blocking):** `apply_fixup`'s `Abs16Be`/`PcRelDisp16`/
   `RelWord16Be` arms lack a `bytes.len()` bounds-check before writing (theoretical only —
   `stream_data` always sizes the 2-byte hole). Could be a small shared-helper follow-up across all
   fixed-width arms.

## Deferred (each a later item)

`base:`/`start:` override, `dc.l` long offsets, Z80 offset tables, cross-module/multi-segment
targets, inline-target blocks (co-located frame bodies inside `offsets{}`). All are diagnosed or
fail loudly, none silently mis-encode.

## NEXT after checkpoint: backlog #4

**Scan/manifest module resolution + map-file placement + game prelude (S2-D3)** — unblocks code
ports at scale (cross-module `use` resolution, placement policy). The offset-table's
cross-module/multi-segment target deferral folds into this. See the ordered backlog in
`docs/superpowers/notes/2026-07-06-spec2-plan7-item3-offset-table-handoff.md` (the pre-work handoff,
list at the bottom) and [[emp-data-table-dsl-candidates]].
