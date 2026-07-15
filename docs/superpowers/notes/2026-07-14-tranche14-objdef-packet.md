# Tranche 14 — objdef data file (the ObjDef-twin driver): MERGE PACKET

**Branches:** sigil `port-tranche14` (off master `a9a5068`), aeon
`.worktrees/sigil-emp-tranche14` (off master `2ecd763`). Loop dry, at merge gate.
**Design note:** `notes/2026-07-14-tranche14-objdef-design.md` (Fable-gated: 4
decisions + 4 riders). **Step-0 gate & step-1 byte-green both Fable-verified.**

## What shipped

The corpus's **first struct-typed data item**: object archetype templates
now emit from a typed `ObjDef` struct via an `objdef()` comptime emitter,
byte-identical to the AS `objdef` macro. The tranche's named payoff — a
machine-checked ObjDef↔Sst **burst-copy correspondence** ensure-chain — is
live and break-verified.

### Commits
**sigil (10):** `690dbcb` step-0 note · `ab84a2e` default params · `2a3d8ee`
layout+chain gate · `c608912` struct-data fixup characterization · `8af2912`
newline params · `f569f14` emitter record tests · `9708850` D-PP.4 reversal +
anim_table jot · `5afc4a1` reference byte gate + OBJDEFS pins · `7b832e1`
step-3(a)+step-6 ledger · `3b2c35f` paired-state probe retarget.
**aeon (4):** `1fae574` ObjDef struct + ensure-chain · `16c1027` objdef()
emitter + vram_art · `3571359` test_objects.emp consumer + gate wiring ·
`3f95626` steps 2-3 framing/comment polish.

## Neither-bucket headlines (step-1 demanded features + gate outcomes)

- **Demanded feature 1 — comptime-fn default parameters** (`ab84a2e`). Reverses
  D-PP.4's "no defaults"; `name: T = expr`, default evaluates in a fresh
  global-only declaration scope (never caller locals / sibling params); a
  param with no default stays required. 6 TDD tests. Reversal recorded in the
  pitcher-plant design doc (`9708850`).
- **Demanded feature 2 — newline-tolerant comptime-fn param lists** (`8af2912`).
  objdef's 14 params read one-per-line; `skip_newlines` in the param loop.
  Surfaced mid-step-1, TDD'd, decl-scoped. (Fable approved retroactively.)
- **Payoff — ObjDef↔Sst ensure-chain** (`1fae574`). `ensure(offsetof(ObjDef,f)
  + TEMPLATE_COPY_SHIFT[8] == offsetof(Sst,f))` per template field (code_addr
  at shift 0), eager comptime. **Break-verified:** SHIFT=9 → all 14 field
  errors; SHIFT=8 green.
- **Novel path characterized** (`c608912`). Struct VALUE with a u16 link-
  difference field → Value16Be, a u32 symbol field → Abs32Be. R1's concern;
  works today, not a demanded feature.
- **Reference byte gate — GREEN both shapes** (`5afc4a1`). Linked
  test_objects.emp records == `s4.bin`/`s4.debug.bin` OBJDEFS windows,
  including x_vel `fixed<8,8>` ($100), render_flags packing, shape-dependent
  Map_TestObj Abs32Be. **Outbound seam:** AS `dc.l ObjDef_Enemy` → the .emp
  `pub data` export. OBJDEFS region + 5 cross-seam symbol pins via repin.
- **Gate-off neutrality VERIFIED.** SIGIL_EMP_OBJDEFS unset → ROMs byte-
  identical to canonical (plain 452500/`11382fa7`, debug 460521/`36bf0f17`).
- **Row-1029 (movem grep) — NEGATIVE, enumerated.** All 24 children.asm movem
  are register save/restore; the block-copy-to-`(An)` anti-pattern is absent.
  No commit owed.
- **Paired-state gate — GREEN.** Full sigil strict `2232 passed / 0 failed`,
  clippy clean, repin `--check` clean, with AEON_DIR=the t14 worktree. One
  prior probe (`drifted_sst_twin_fires_its_own_guard`) required retargeting —
  see step-2 findings.

## What each pass added

### Step 1 (transcribe) — demanded features + byte-green
Above (neither-bucket). Both demanded features + the emitter + consumer +
full byte gate landed here.

### Step 2 (modernize) — findings
- **sst.emp framing widened** (`3f95626`): the module header now frames both
  Sst and ObjDef as the spawn-template structs (was "SST's type-only twin";
  ObjDef now emits via consumers). Byte-neutral.
- **Paired-state interaction (live finding, not a defect):** the eager ObjDef
  chain catches TEMPLATE-field drift at comptime, earlier than the extern
  link-seam guard. This shadowed the tranche-6 `drifted_sst_twin` probe (it
  swapped template fields anim/subtype expecting clean lowering + a link
  catch). **Reconciled** (`3b2c35f`): retargeted the probe to two runtime
  fields (prev_frame/sprite_piece_count, past ObjDef's extent) so it still
  isolates the extern guard; template-field drift is covered by objdef_port's
  SHIFT break-test. A genuine detection improvement, not a regression.

### Step 3 (retrospect) — findings

**3(a) language/format asks** (interrogation, per proc = objdef()/the chain):
| item | outcome |
|---|---|
| Ceremony scan | **ASK (ledgered):** the 15-line ensure-chain is one intent; the clean form iterates a field-name list, but `offsetof` takes a LITERAL field — blocked. Big ask (offsetof-over-field-list / `struct_corresponds` builtin), NOT hand-built (chain stays; each line clear + break-tested). |
| Comment-as-compensation | objroutine idiom: code_addr's `extern(code)-extern("ObjCodeBase")` carries a 4-line comment for a missing `objroutine(code)` helper → data point on ledger 680-685. |
| Escape-hatch census | 3 `extern()` + 2 `string`-typed symbol params (code/map) = the bareword-label / `Label`-type want (R2) → data point on 680-685. |
| Domain-type scan | newtype candidates: CollisionType (`collision:u8` = COLLISION_* enum), RenderFlags (`render_bits`+priority pack), RomPtr now 2-field (mappings, anim_table; R4). Ledgered for [[emp-sonic-newtype-candidates]]. |

**3(b) reads-wrong** (interrogation):
| item | outcome |
|---|---|
| Comment-claim audit | clean — objdef.emp's link-shape claims (Value16Be code_addr / Abs32Be mappings) and the TEMPLATE_COPY_SHIFT derivation are byte-gate-verified. |
| Contract audit | N/A (comptime fns, no register contracts). |
| Name audit | clean — priority/render_bits/width read better than the macro's zpri/rfbits/wdth. |
| Magic-number audit | **fixed** (`3f95626`, byte-neutral): vram_art bit positions + `angle:0`/`pad:0` rationale commented. |
| Cold-reader test | clean — header + field comments carry the trace. |

**3(c) mirrors/gaps:** no new twin-scaffolding mirrors (ObjDef is a real
struct; sst.emp's extern guards keep their existing kill condition = sst.asm
port). vram_art carries a "relocate at 2nd .emp consumer" note.

### Step 4 (construct pass) — findings
- **ensure-chain construct** → verb (c) ASK, ledgered, NOT built (offsetof-
  over-field-list blocker; a hand-built helper would be a stopgap with no
  readability win given the literal-field constraint).
- **vram_art** → already built in step 1 (macros.asm's `vram_art` function;
  first .emp consumer). No new constructs. No structural clones found.

### Step 5 (optimize) — findings
objdef() is **comptime data emission — zero runtime cost** (static ROM
records). Interrogation:
| item | outcome |
|---|---|
| Invariant ladder | N/A (no runtime loop). |
| Counter/cache audit | N/A. |
| Guard-coverage audit | the priority `0..7` refinement is the sole guard; it's on the param → covers every objdef() call. Load-bearing. |
| Hardware cross-check | N/A (data, not VDP-facing). |
| Silent-tradeoff comments | none. |
→ **No changes** (no runtime code exists to optimize). Recorded.

### Loop-until-dry
Second retrospect after step 5: all findings are ledgered asks / newtype
candidates / at-next-touch retrofits — no new code surfaced. **Dry.**

### Step 6 (corpus sweep) — enumeration (`7b832e1`)
Two new features swept across the whole .emp corpus, every site's outcome named:
- **newline-tolerant params:** RETROFIT-AVAILABLE at `aabb_axis_test` (9 params)
  and `ojz_sec` (6 params); format-only → LEDGERED "reformat at next touch"
  per the brace-indent precedent (no dedicated cross-tranche wave). Every
  other corpus comptime fn (≤3 params / param-less) = not-an-instance.
- **default params:** NO prior-file instance (corpus fns take all-required
  reg/Label args) — objdef-specific demand. Named.
- **demo_data.asm objdef (R3):** RETROFIT-AVAILABLE but LEDGERED — the file is
  mostly non-objdef data; its objdef converts when demo_data ports as a whole
  (consumer's own cadence).

## Provenance
**No re-baseline.** SIGIL_EMP_OBJDEFS is a gate; the default build ships the
AS `.asm` unchanged → canonical ROMs (plain 452500/`11382fa7`, debug
460521/`36bf0f17`) are byte-identical (verified by rebuild). The mixed build
(gate on) produces the same bytes (byte gate proves it) but is not the
default. Worktree reference ROMs built for the gate; merge close-out
re-verifies on the default AEON_DIR.

## Open items carried forward (all ledgered, none blocking merge)
- offsetof-over-field-list / `struct_corresponds` construct (the ensure-chain
  ceremony ask).
- objroutine-in-expr helper + bareword-label/`Label` param type (680-685; new
  data points).
- newtype candidates: CollisionType, RenderFlags, RomPtr.
- anim_table-as-symbol path untested (all 4 records default it to 0; first
  animated consumer proves it).
- newline-params retrofits (aabb_axis_test/ojz_sec) at next touch; demo_data
  objdef at its own port.
