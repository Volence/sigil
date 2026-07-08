# Pitcher Plant acceptance-exhibit gap analysis (Spec 2, Plan 7 #4)

Date: 2026-07-07
Scope: what `examples/pitcher_plant.emp` still needs to compile end-to-end.
Method: this is **documentation, not a compile target**. The file is the standing
acceptance exhibit for work deferred past Plan 5; #4 (module resolution) supplies
only *some* of what it needs. Every blocker below is grounded in an actual error
from the current front-end, not from the file's own aspirational STATUS comment.

## How the evidence was gathered

`examples/` has a module-collision (two files declare `module badniks.pitcher_plant`),
so the exhibit was copied ALONE into a fresh tempdir alongside a minimal prelude
(`module prelude` + `pub struct ObjDef (size: 8) { code: *u8, map: *u8 }`) and driven:

```
cargo run -q -p sigil-cli -- emp <tmp>/pitcher_plant.emp --root <tmp> --prelude prelude
```

Isolation probes (separate one-module tempdirs) were used to pin two specific
questions: does SST-overlay field access lower today, and is `vars … : sst_custom`
itself the problem. Findings below cite the exact diagnostic text.

---

## Category (a) — content #4 CAN supply (just needs authoring)

These are NOT front-end gaps. They resolve the day someone authors the game
prelude + sibling art/engine modules and lets #4's `use`/prelude resolution wire
them in. No grammar or lowering change is required.

### a1. `ObjDef` is missing most of its fields
The real object descriptor needs typed fields the minimal stand-in lacks.
- Errors: `[struct.unknown-field] struct ObjDef has no field 'art'` (line 111),
  and identically for `col` (114), `zpri` (115), `size` (116), `anim` (117),
  `vel` (128), `frame` (130). `code`/`map` already resolve.
- Construct: `pub data Def = ObjDef{ code: …, map: …, art: …, col: …, … }`.
- Fix: author the full `ObjDef` struct in the prelude with all fields + types.

### a2. Missing prelude TYPES referenced by field initializers
- `Collision` — `unknown name 'Collision.Hurt'` (114), `'Collision.Projectile'` (125).
- `ArtTile` — reached only after `art` field exists; `ArtTile{ tile:…, pal:…, pri:… }`.
- `Size` — `Size{ w:…, h:… }` (116, 127).
- `Vel` — `Vel{ x:…, y:… }` (128).
- (`Anim` is NOT missing — it is declared locally in the file as `enum Anim`, and
  `Anim.Idle`/`Anim.Shoot` resolve; do not list it as a prelude gap.)
- Construct: struct-literal / enum field initializers.
- Fix: author `Collision` (enum), `ArtTile`/`Size`/`Vel` (structs) in the prelude.

### a3. Missing SIBLING labels (art + engine modules)
- `Map_PitcherPlant` — `unknown name 'Map_PitcherPlant'` (110, 123).
- `VRAM_PITCHER_PLANT` — `unknown name 'VRAM_PITCHER_PLANT'` (111).
- `Player_1` — surfaces via `unknown symbol 'Player_1.x_pos'` (see b6).
- `Draw_Sprite`, `ObjectMove` — engine labels reached only once `jbra`/`jbsr` parse
  (currently masked by b1); they are ordinary cross-module labels #4 resolves.
- Construct: cross-module label references.
- Fix: author a sibling art module (`pub data Map_PitcherPlant`, `pub const
  VRAM_PITCHER_PLANT`) and an engine module (`pub proc Draw_Sprite`, `ObjectMove`,
  `Player_1`); `use` / prelude auto-import wires them. **This is exactly the
  mechanism Deliverable 2's 3-module corpus already proves works.**

### a4. `Def.art` cross-item field read
- Error: `unknown name 'Def.art'` (124) — SeedDef reuses `Def.art`.
- This is downstream of a1 (once `Def` has an `art` field it can be read); it is a
  comptime field-read on a prior data item, not a new mechanism. Listed for
  completeness; resolves with a1.

---

## Category (b) — front-end gaps OUTSIDE #4

These need front-end work; #4 cannot supply them by authoring alone.

### b1. `jbra` / `jbsr` auto-reaching pseudo-ops → backlog #8
- Errors: `'jbra' is not a recognized 68000 mnemonic` (182, 187, 215, 225);
  `'jbsr' is not a recognized 68000 mnemonic` (224).
- Construct: new-style direct-label branches `jbra Draw_Sprite` / `jbsr ObjectMove`.
- Backlog: **#8** (jbra/jbsr auto-reaching branches). Unimplemented.
- Downstream effect: because `jbra Draw_Sprite` fails to parse as a terminator,
  every proc that ends in one is ALSO reported as
  `[proc.undeclared-fallthrough] 'wait'/'shoot'/'seed' can reach its closing '}'`
  (161, 194, 219). Those three fallthrough errors are ARTIFACTS of b1, not
  independent blockers — they vanish once `jbra` is a recognized terminator.

### b2. Unsized conditional branches require an explicit `.s`/`.w`
- Errors: `[branch.missing-size] branch needs an explicit size suffix (.s or .w)
  — Aeon pins branch width, no relaxation` (164, 176, 197, 208) for `bne`/`bhi`.
- Construct: `bne .draw`, `bhi .rearm` written unsized.
- Gap: the file's design comment claims conditional branches "auto-size .s↔.w",
  but the current front-end REQUIRES an explicit suffix (no relaxation). Either the
  exhibit must be authored with `bne.s`/`bhi.s` (a doc-only fix), or unsized-branch
  relaxation must be implemented. Related to the #8 branch-sizing family; distinct
  from jbra/jbsr in that it concerns *conditional* branches, which have no far form.

### b3. Statement-position comptime helpers `anim` / `routine` / `facing_abs` /
`despawn_below_level`
- Errors (all `not a recognized 68000 mnemonic`): `anim` (180, 212),
  `routine` (181, 213), `facing_abs` (174), `despawn_below_level` (220).
- Construct: bareword helper invocations in statement position inside a `proc`.
- Gap: these must be AUTHORED as `comptime fn`s AND the grammar must accept a
  comptime-helper call in statement position (today the instruction parser tries
  each leading bareword as a 68000 mnemonic and rejects it). Both an authoring gap
  (the helper bodies don't exist) and a grammar gap (statement-position invocation).

### b4. `spawn(...)` named-argument call with `inherit` and `Vec{}` literal args
- Errors: `unknown function 'spawn'` (204) + `[prov.comptime] error is inside a
  table generated by this comptime call`.
- Construct: `spawn(SeedDef, offset: Vec{ x: -16, y: -4 }, flip: inherit)`.
- Gap: needs (1) the `spawn` comptime fn authored, (2) NAMED-ARG call syntax,
  (3) the `inherit` keyword/value, and (4) `Vec{}` struct literals as call
  arguments. Grammar + authoring, larger than b3.

### b5. SST-overlay field access as displacement — `timer(a0)` / `x_pos(a0)` /
`y_vel(a0)` — **VERIFIED does NOT lower today**
- Errors: `unknown name 'timer'` (157, 163, 179, 185, 195, 196, 207, 211),
  `unknown name 'x_pos'` (173), `unknown name 'y_vel'` (223).
- Isolation probe (fresh one-module tempdir):
  ```
  module m
  vars V: sst_custom { timer: u8, }
  proc p (a0: *u8) { subq.b #1, timer(a0)  rts }
  ```
  → `unknown name 'timer'`. But the SAME file WITHOUT the `timer(a0)` operand
  (`vars … : sst_custom { timer: u8 }` + a bare `rts`) builds cleanly (`built: 2
  bytes`).
- Conclusion: the `vars … : sst_custom { … }` DECLARATION parses and lowers today;
  what is unbuilt is FIELD-ACCESS-AS-DISPLACEMENT — resolving `timer(a0)` to
  `sst_custom_base + field_offset(a0)` in an instruction operand. This is a
  front-end feature that still needs to be built; #4 does not provide it.

### b6. Symbolic operand in a straight-line instruction — `Player_1.x_pos`
- Error: `unknown symbol 'Player_1.x_pos'` (reported at 51, the module header, as
  the unresolved link symbol).
- Construct: `move.w Player_1.x_pos, d0` — a symbolic MEMORY operand on a
  straight-line instruction.
- Gap: today only branch/jmp/jsr TARGETS defer to the linker; a symbolic operand on
  an ordinary instruction does not. Even once `Player_1` exists (a3), the `.x_pos`
  displacement + link-deferred operand needs front-end support. (Combines a sibling
  label a3 with an unbuilt operand-resolution path — the label alone is not enough.)

### b7. Bareword proc-label as a pointer field value — `code: init` vs `code: "init"`
- Error: `unknown name 'init'` (109), `unknown name 'seed'` (122).
- Construct: `ObjDef{ code: init, … }` — a BARE proc name in a `*u8` field.
- Gap: the evaluator wants a resolvable symbol; today the working spelling is the
  STRING form `code: "init"` (a string label reference, which Deliverable 2 uses
  and which links cleanly). So `code: init` (bareword) is an unbuilt convenience;
  the string form is the supported spelling today. Doc-or-feature choice.

---

## Non-blockers / harness artifacts (not gaps)

- **Module-id vs filename mismatch**: `module 'badniks.pitcher_plant' is at
  'pitcher_plant.emp', which suggests id 'pitcher_plant'` (51). This is only because
  the probe copied the file to a flat `pitcher_plant.emp`; under
  `examples/badniks/pitcher_plant.emp` the id matches. Not a language gap.
- **`*Sst` pointee type**: `proc init (a0: *Sst)` does NOT error even though `Sst`
  is undefined — a pointer param accepts an unresolved pointee (it is just a 4-byte
  address). Confirmed in isolation (`proc p (a0: *Sst) { rts }` → `built: 2 bytes`).
  So no "unknown type Sst" appears; do not list it as a blocker.

---

## Summary table

| # | Blocker | Category | Resolves via |
|---|---------|----------|--------------|
| a1 | `ObjDef` missing fields (art/col/zpri/size/anim/vel/frame) | a | author prelude struct |
| a2 | Missing types `Collision`/`ArtTile`/`Size`/`Vel` | a | author prelude types |
| a3 | Missing siblings `Map_PitcherPlant`/`VRAM_PITCHER_PLANT`/`Player_1`/`Draw_Sprite`/`ObjectMove` | a | author art+engine modules (#4 `use`/prelude) |
| a4 | `Def.art` cross-item read | a | downstream of a1 |
| b1 | `jbra`/`jbsr` unrecognized | b | backlog **#8** |
| b2 | unsized `bne`/`bhi` need `.s`/`.w` | b | branch relaxation (or author sized) |
| b3 | `anim`/`routine`/`facing_abs`/`despawn_below_level` stmt helpers | b | author comptime fn + stmt-position grammar |
| b4 | `spawn(...)` named-arg call | b | author comptime fn + named-arg/`inherit`/`Vec{}`-arg grammar |
| b5 | SST-overlay field access `timer(a0)` — **verified not lowering** | b | build field-access-as-displacement |
| b6 | symbolic operand `Player_1.x_pos` in straight-line instr | b | link-deferred operand resolution |
| b7 | bareword `code: init` (string `"init"` works today) | b | bareword-label-as-pointer value |

**Bottom line.** #4 (this milestone) can supply everything in category (a): the
prelude struct/types + sibling art/engine modules, wired by `use`/prelude
resolution — the exact composition Deliverable 2 proves. The exhibit stays blocked
on category (b): `jbra`/`jbsr` (#8) + branch relaxation, statement-position and
named-arg comptime helpers (authoring + grammar), SST-overlay field access
(verified unbuilt today), straight-line symbolic operands, and the bareword-label
pointer convenience. None are grammar-parse failures — all are lowering/authoring
gaps, consistent with the file's own STATUS note.
