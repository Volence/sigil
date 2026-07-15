# offsetof in absolute-EA + EntityScanState struct-twin — step-0 note

**Batch:** sst-usability-batch, item 2. Byte-neutral, TDD. **Needs Fable's
gate BEFORE code.** Closes gap-ledger row 1005 (the t12 deferral).

## What this closes

t12 deferred the EntityScanState struct-twin (row 1005) for two reasons:
(a) `offsetof` didn't work in ABSOLUTE-EA operand position, which the
absolute-address entry accesses need; (b) no home for a single-consumer
struct-twin. This item builds the operand support, stands up the twin, and
adopts it in entity_window.emp byte-neutral — deleting the 11 mirrored
offset consts + their 12 drift guards (+ their kill-list row) same-commit.

## Enumeration — the two access shapes in entity_window.emp

The 11 `EntityScanState_ess_*` consts (entity_window.emp:52-63) + 12 drift
guards (:97-108) are consumed two ways:

### A. Register-relative — `ess_field(aN)` (47 sites) — ALREADY supported
`move.b d1, EntityScanState_ess_section_id(a1)` etc. (:638, :651-658, and
44 more). A struct-twin makes these typed field access (`ess_section_id(a1)`
on an `a1: *EntityScanState`) via the EXISTING D6.A3/A4 path — the sst.emp
mechanism, no new language work. Byte-neutral.
Also `EntityScanState_len(aN)` stride advances (`lea EntityScanState_len(a3),a3`,
6 sites :604/788/911/1302/1683) and `#(MAX_TRACKED_SECTIONS*EntityScanState_len)`
(:818) → `sizeof(EntityScanState)` (comptime int). Byte-neutral.

### B. Absolute-EA — `Entity_Scan_State + (len*N + ess_field)` — THE BLOCKER
~14 sites, all needing offsetof (+ sizeof) inside an absolute-address operand:
- **Section-match unroll ×2** (:1402-1408, :1490-1496) — 8 sites:
  `cmp.b Entity_Scan_State+(EntityScanState_len*N+EntityScanState_ess_section_id), d1`
  for N=0..3.
- **Copy-out unroll** (:1615-1618) — 4 sites:
  `move.b Entity_Scan_State+(EntityScanState_len*N+EntityScanState_ess_section_id), (a0)+`
  for N=1..3 (+ the N=0 bare form).
- **Bare field leas** (:599, :1563) — 2 sites:
  `lea Entity_Scan_State+EntityScanState_ess_section_id, a0`.

Target spelling (offsetof + sizeof in the absolute operand):
```
cmp.b   (Entity_Scan_State + sizeof(EntityScanState)*2 + offsetof(EntityScanState, ess_section_id)).w, d1
```
(The 5 base-only `lea Entity_Scan_State, aN` sites — :733/817/901/1292/1667
— load the base for register-relative access; unaffected.)

## The language feature — offsetof/sizeof in absolute-EA operand position

`offsetof` and `sizeof` already evaluate to `Value::Int` in any EXPRESSION
position (proven in tranche 14: they compose in `ensure` arithmetic). The
t12 blocker is specifically the ABSOLUTE-EA OPERAND parse/lowering path
rejecting an operand expression that contains them. Step-0 scope is to
CONFIRM the exact failure (parse-time vs eval-time) and design the minimal
fix; the byte-neutral goal is that
`(Entity_Scan_State + sizeof(S)*N + offsetof(S, f)).w` lowers to the same
absolute-word operand as the const-arithmetic form does today. Distinct
from t14's spliced-index / data-position offsetof — this is offsetof in a
COMPTIME operand-displacement expression that folds to an absolute address.
Implementation confirmed + TDD'd after the gate.

## Ess twin home — RECOMMEND file-local in entity_window.emp

EntityScanState has a SINGLE consumer (entity_window.emp) and a distinct
domain (entity-window despawn bookkeeping), unlike Sst — which many object
modules share, justifying its own `engine.objects.sst` module.

- **RECOMMEND: file-local in entity_window.emp.** Co-locate the struct-twin
  + its 12 drift guards with its sole consumer. Establishes the clean
  complement to sst.emp's precedent: **shared structs get their own module;
  single-consumer structs live in that module.** No new module for one
  struct; the typed-reg access resolves against a locally-declared struct.
- Alternative — **sst.emp** (t14 widened it to "the spawn-template
  structs"): rejected — Ess is not a spawn-template; hosting it there
  broadens sst.emp's charter to "all engine structs," and on THIS branch
  (off master) sst.emp is still SST-only (ObjDef is t14, pending), so the
  dependency would also couple this batch to t14 for no design gain.
- Alternative — a new `engine.structs` shared module: premature for one
  struct; revisit if a THIRD single-consumer struct-twin appears.

(Note on base: this batch branches off master, so it does not depend on
t14. The file-local home keeps that decoupling; if Fable prefers the
sst.emp home, this batch would rebase onto t14 — flag at the gate.)

## Adoption plan (byte-neutral; entity_window byte gate is the verifier)
1. Build offsetof/sizeof-in-absolute-EA operand support (TDD).
2. Stand up `struct EntityScanState` (file-local) with `@` offsets +
   `extern("EntityScanState_*")` drift guards (the sst.emp pattern),
   pointing at the same truths the 11 consts + 12 guards do now.
3. Convert the 47 register-relative sites → typed `ess_field(aN)`; the
   strides/immediate → `sizeof(EntityScanState)`; the ~14 absolute-EA
   sites → the offsetof/sizeof operand form.
4. DELETE the 11 mirrored `EntityScanState_ess_*` consts + 12 drift guards
   + their twin-scaffolding kill-list row, SAME COMMIT.
5. entity_window_port byte gate GREEN both shapes; full strict + clippy +
   repin `--check` (nothing moves). Close row 1005.

## Open decisions for Fable's gate
1. **Ess home:** file-local in entity_window.emp (recommended) vs sst.emp
   (couples to t14) vs a new shared struct module.
2. **Absolute-EA spelling:** `(base + sizeof(S)*N + offsetof(S,f)).w`
   confirmed as the target? (vs keeping an explicit `.w`/`.l` size on the
   absolute operand — it's already there today.)
3. **Branch base:** confirm off-master (decoupled from t14) is right, given
   the recommended file-local home needs nothing from t14.
