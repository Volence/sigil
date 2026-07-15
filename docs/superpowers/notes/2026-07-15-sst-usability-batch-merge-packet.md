# sst-usability-batch — merge packet (Fable's gate)

Branch `sst-usability-batch` (both repos). Two usability items + one tooling
rider, all byte-neutral, ready for the merge gate.

## Contents

| Item | What | Commits |
|---|---|---|
| 1 | Overlay-write spelling `Sst.field:size(aN)` — a `:b`/`:w`/`:l` sized override that DECLARES an overlay width past the field's own size, struct-end-bounded. First consumer: `load_object.emp:70` drops the offsetof escape → `move.l #$FF000000, Sst.prev_anim:l(a1)`. | aeon `0d7e723` / sigil `fe4a647` |
| 2 | EntityScanState struct-twin adoption in `entity_window.emp` (this session). | aeon (new) / sigil (new) |
| rider | Fresh-worktree seed script `tools/seed-worktree.sh` (editor data + generated ojz + data binaries + built ROMs). | aeon `eddbc1c` / sigil ledger `6ea4188` |

Item 1 design note: `notes/2026-07-15-overlay-write-syntax-design.md`.
Item 2 step-0 (Fable-gated): `notes/2026-07-15-offsetof-abs-ea-entityscanstate-step0.md`.

## Item 2 — what shipped

Replaced the 11 `EntityScanState_ess_*` offset consts + `EntityScanState_len`
with a file-local `struct EntityScanState (size: $1A)` twin (sst.emp pattern).
Home = file-local (single consumer — the ratified complement to sst.emp's
"shared struct earns a module" precedent; no `engine.structs` module, no t14
coupling). Adoption counts (all byte-neutral, `entity_window_port` GREEN both
shapes):

- **47** register-relative sites → `EntityScanState.ess_*(aN)` typed access
- **5** stride leas + the `#(MAX_TRACKED_SECTIONS*len)` immediate → `sizeof(EntityScanState)`
- **14** absolute-EA sites → `offsetof(EntityScanState, ess_section_id)` addend
- 12 offset consts DELETED; the 12 drift guards PERSIST (RHS const→literal — now the struct's guards, the sst.emp double-lock)

Verification: entity_window byte gate GREEN both shapes; full workspace strict
GREEN (191 suites, AEON_DIR=worktree, SIGIL_STRICT_GATE=1); clippy clean;
`repin --check` = pins unchanged; gate-off aeon build CRCs unchanged (plain
`11382fa7` / debug `36bf0f17`).

## ⚠ GATE FLAGS — decisions that deviate from the step-0 plan

### 1. The gated `.w` absolute-EA spelling does NOT compile — shipped without it

The step-0 note (and the gated decision) specified the absolute-EA spelling as
`(Entity_Scan_State + sizeof(EntityScanState)*N + offsetof(EntityScanState, field)).w`
"with the explicit `.w` kept", on the strength of a probe that "confirmed it
already lowers correctly." **That premise is false for a LINK-TIME base.**
Re-probed 2026-07-15 (empirical, not from memory):

- `(Entity_Scan_State + offsetof(EntityScanState, ess_section_id)).w`
  → `error: unknown name Entity_Scan_State` at every one of the 14 sites. The
  parenthesized `(expr).w` absolute-override evaluates its contents at comptime
  and cannot defer the link-time extern base.
- `(extern("Entity_Scan_State") + offsetof(...)).w`
  → `error: [here.provisional]` — the extern-address `.w` form collides with
  size-relaxation inside `EntityWindow_EntryForSection`'s jbra body.

So the planned operand-expression `.w` feature remains **genuinely unbuilt**,
and adoption never needed it. The byte-neutral form that DOES work: `offsetof`
/`sizeof` are valid comptime addends in the EXISTING `symbol + const`
absolute-EA form — the row-1004 parenthesized workaround extended to offsetof:

```
cmp.b  Entity_Scan_State + (sizeof(EntityScanState)*1 + offsetof(EntityScanState, ess_section_id)), d1   // N>=1
cmp.b  Entity_Scan_State + offsetof(EntityScanState, ess_section_id), d1                                 // N=0
```

No `.w` suffix — the linker's asl width rule picks abs.w (Entity_Scan_State =
`$FFFFABFC`, word-addressable), identical to the pre-adoption bare form. This
matches the dominant codebase convention for abs.w RAM access (collision_lookup
/camera sites use bare-symbol + width-rule). **Decision for the gate:** ratify
the no-`.w` parenthesized form as the standing spelling, or fund the
operand-expression feature (gap-ledger row 1004's SymOff generalization) if the
explicit `.w` is wanted later. Recorded in gap-ledger t12 clause (a) closure.

### 2. Kill-list: row 22 AMENDED + new row 25, not deleted

The task said "delete their kill-list row." Row 22 is COMPOUND (GAME caps +
engine consts + `EntityScanState`/`Sec`/`Act` offset mirrors) and the 12
EntityScanState drift guards PERSIST as the struct twin's guards — live
scaffolding that dies only when `engine/structs.asm` ports, exactly like the
`Sst` twin (row 11) and `Act`/`Sec` twins (row 7). So: row 22 drops only its
EntityScanState offset-mirror clause (`~41`→`~28` ensures; `Sec`/`Act` + caps +
engine consts stay); the struct twin moves to new **row 25** (row-11 class,
kill = structs.asm ports). Deleting all tracking would have lost a scheduled
demolition the kill-list must not forget.

### 3. gap-ledger / t14 textual conflict

Both this batch and t14 amend `campaign-gap-ledger.md`. Whichever merges second
resolves the conflict by KEEPING BOTH row sets (the edits are to different
entries — t12 clause (a) closure + row-1004 note here; t14 elsewhere).

## Findings (neither step-3 nor step-5 — a language-limit discovery)

The headline is not a modernization or an optimization: it is that the step-0
diagnosis was wrong about the operand-expression path, caught only because the
byte gate was run rather than trusted. Adoption succeeded anyway via the
existing addend form. No new bytes, no new language feature — the "feature" the
batch was scoped around turned out to be unnecessary AND still-unbuilt, and the
ledger now says so plainly so nobody re-plans it on a false premise.
