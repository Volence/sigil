# 2026-07-29 — t38 step-1 BLOCKER: player_sensors demands an in-proc self-relative jump table

Porter: Opus subagent. Follows `2026-07-29-t38-step0-recon.md`. Reached at the
step-1 transcription. **This is a design-boundary checkpoint** — step 1 cannot
complete byte-exactly without a frontend increment that touches a deliberate
language-design choice, so it is reported for the overseer's ruling rather than
built unilaterally.

## What IS portable at step 1 (no blocker)

- `probeSub` / `probeCore` AS macros → comptime fns (`emit_piece_loop` precedent,
  sprites.emp): comptime `bool`/`int`/`Reg`/`Label` params + `if pnegate { neg.w d0 }`
  comptime arms. probeCore stamps a full labeled routine with an internal `.cell`
  local subroutine — expressible. `Label` param type is shipped (perform_dplc/
  aabb_axis_test precedent).
- The computed `jsr (a2)` in Player_SensorPair → a local `type SensorProbe = proc(...)`
  + `jsr (a2) as SensorProbe` (core.emp `jsr (a1) as ObjRoutine` precedent).
- `movem.w d3-d5, -(sp)` (boot.emp precedent), fall-through via `falls_into`
  (Ceiling→Surface, WallAt→WallDir), bare-abs-EA tables (ROM ≥$8000 → `.l`).

## The BLOCKER: two in-proc self-relative jump tables

`Player_SensorSurface` (`.case_table`, L280-284) and `Player_SensorWallDir`
(`.dir_table`, L383-387) each use the classic S3K load-word-offset dispatch:

```
        add.w   d2, d2
        move.w  .case_table(pc, d2.w), d2     ; load self-rel offset word
        jmp     .case_table(pc, d2.w)          ; jump to base + offset
.case_table:
        dc.w    .probe_down-.case_table        ; INLINE, self-relative
        dc.w    .probe_left-.case_table
        dc.w    .probe_up-.case_table
        dc.w    .probe_right-.case_table
.probe_down: ...
```

The table is **INLINE in the proc**, immediately after the `jmp`; the targets are
proc-LOCAL labels. The AS frontend compiles `dc.w .local - .local` fine (it is in the
reference ROM). The **EMP frontend cannot express it**, and every existing construct
is a wrong fit for byte-exactness. Five probes (`scratchpad/probe_*.emp`):

1. `dc.w .a - .case_table` in a proc → **`\`-\` not defined for label and label`**.
   `Value::Label` DELIBERATELY rejects arithmetic (value.rs: "rejects comptime address
   arithmetic (`init + 2`)"). `lower_dc` also has no arm for a symbol-difference value
   (its catch-all is `[dc.comptime-only]`).
2. `offsets Name { … }` INSIDE a proc → `jmp Name(pc,d2.w)` = **`[asm.splice-kind]
   expected int, Reg, or Sym, got struct`** (in-proc offsets name resolves to a struct,
   not a Sym).
3. `offsets Name { … }` at MODULE level, consumed `lea Name(pc),a1; move (a1,d2.w)` →
   **compiles**, but adds a `lea` (+bytes) AND relocates the table → different
   pc-relative displacement → NOT byte-exact.
4. `offsets`/`dispatch (encoding: word_offsets)` at MODULE level in the DIRECT
   `Name(pc,d2.w)` position → **compiles, correct `dc.w target-base` words**. But the
   table is at MODULE scope, not inline → the `move.w`/`jmp` pc-relative displacement
   differs from the AS inline table → NOT byte-exact.
5. `dispatch`/`offsets` with inline bodies declared INSIDE a proc → **parser rejects it
   outright** ("expected end of line" / "expected a declaration, found Comma"). Both
   constructs are TOP-LEVEL-ONLY.

**Conclusion:** the `offsets` RelOffset machinery + `Cell::RelOffset` IR exist and are
correct, but no surface syntax reaches them from an INLINE proc position. player_sensors
is the first ported file to need a self-relative jump table where the AS layout puts the
table INLINE and PC-relative (the object dispatch tables live in DATA sections consumed
via `lea`; this one is code-inline).

## Proposed feature (two options — overseer's design call)

**Option A — `dc.w <label> - <label>` → `Cell::RelOffset` (most faithful, smallest).**
In `eval_arith`, `(Value::Label t, Value::Label b, BinOp::Sub)` → a new value carrying
the symbol difference; `lower_dc` maps it to `Cell::RelOffset { base: b, target: t }`
(the linker already folds it). Reject the value outside data position. Mirrors the AS
source 1-1, keeps the table inline, byte-exact. COST: crosses the deliberate "`Label`
rejects arithmetic" rule — but a symbol DIFFERENCE (same-section, position-independent
int) is categorically distinct from `Label + Int` address arithmetic, so the rule can
stay for `+`/address-offset while `label - label` (a defined integer) is admitted. This
is a language-semantics addition → overseer ratification.

**Option B — inline `offsets`/`dispatch` as a proc statement usable as a PC-relative
base.** Larger (parser accepts the decl as a proc statement; the name resolves to a Sym
at its inline address for `Name(pc,d2.w)`). Reuses the construct and pre-stages item-4's
`offsets` adoption. COST: parser + scoping work; the inline-body form (`Member: { … }`)
would also need to co-locate bodies inline byte-exactly.

**Recommendation:** Option A. It is the minimal, faithful transcription (the AS twin's
exact spelling), reuses the shipped `Cell::RelOffset` fixup, and keeps step 1 a true
1-1 port; the `offsets`/`dispatch` inline story (Option B) is a bigger construct effort
better addressed as its own increment. Step-3(a) would still LEDGER the `offsets`-inline
readability ask.

## Impact on the tranche

t38 is NOT the clean CANONICAL-BYTES port the brief assumed — it demands a frontend
feature FIRST (the demanded-features law: the feature ships at step 1). Once the feature
lands, the rest of the port (comptime fns + wrappers + gate + windowed/whole-ROM +
repin + contract closure + the §3 retirement checklist at step 2+) proceeds as planned.
The step-0 findings (esp. the row-74 `_pl_*` guard-survival condition-NOT-met, all 5
contract matches) are unaffected and stand.
</content>
