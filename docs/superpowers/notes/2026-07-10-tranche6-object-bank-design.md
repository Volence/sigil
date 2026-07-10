# Tranche 6 step-0 design — the object-bank opener (construct-walk #1 deliverable)

2026-07-10. Volence offered the interactive walk, chose the solo note with
a review checkpoint before step-1 code — this note is that checkpoint
artifact. The tranche-7 gate question (do his queued structural engine
changes touch `engine/objects/collision.asm`?) was asked and answered
"not sure yet" — **re-ask at the tranche-7 kickoff before transcribing.**

Scope (ratified): `games/sonic4/objects/test_solid.asm` (22 ln) +
`games/sonic4/objects/test_particle.asm` (48 ln). Both proc-shaped —
`code_word`/S2-D12b stays parked for the first scripted badnik.

## Part 1 — the walk itself: production prelude vs. the real engine

The kickoff flagged this construct class highest-drift-risk of the
campaign ("the mock-prelude class that hid the table-less dispatcher").
Verdict: **the risk was real, the drift is total at the layout level, and
the construct machinery underneath survives contact intact.**

### The drift map (exhibit `Sst` vs. `engine/structs.asm` SST)

| Offset | exhibit prelude / sst_overlay | real SST | reading |
|---|---|---|---|
| $00 | `id: u16` | **`code_addr`** — word offset from ObjCodeBase, 0 = empty | The mock hid the table-less dispatcher AGAIN, at the exact offset the kickoff predicted. There is no object-id indirection anywhere: the SST's first word IS the dispatch. |
| $02 | pad | `x_pos ds.l` (16.16 subpixel) | exhibit's `x_pos: u16 @ $10` lands on the real `mappings` pointer |
| $10 | `x_pos: u16` | `mappings ds.l` | " |
| $18/$1A | `x_vel/y_vel @ $18/$1A` | `anim ds.b $18` / `anim_table ds.l $1A` | real velocities live at $0A/$0C |
| $20 | `resume: ScriptPc` | `prev_anim ds.b` | the exhibit invented a routine slot at $20; the real routine slot is `code_addr` at $00 |
| $2E | `sst_custom: [u8; 34]` | `sst_custom ds.b 34` | **MATCHES** — window offset, size, and $50 total all survive |

### What survives, what dies, what's newly demanded

- **Survives**: the entire construct surface — `struct` with dense
  `@`-asserted offsets, `vars` overlays over `sst_custom`, typed
  `a0: *Sst` bare-field access, qualified `Sst.field(aN)` access,
  `offsets` anim tables, `falls_into`, comptime-fn templates. All real,
  all lowering today. No *field-access* language gap exists.
- **Dies**: every layout fact in the exhibit prelude, and the exhibit's
  `routine` helper mechanics (`pea {p}` + pop the low word into a $20
  slot). The real store is `move.w #objroutine(X), SST_code_addr(a0)` —
  a **word immediate of a link-time symbol difference**. (Amusing pun,
  recorded so nobody "discovers" it later: because ObjCodeBase sits at
  exactly $10000, the pea-trick's address-low-word EQUALS
  `objroutine(x)` for the bank's first 64KB — a coincidence of the org,
  not a design; it is also 2 bytes/~10 cycles worse. Not used.)
- **Newly demanded** (step-1, the demanded-features law):
  **`.w` ImmLink** — `lower_m68k_imm_link` fences to `.l` today
  (code.rs:615); the link side already has everything (`Value16Be`
  general value fixups, `Sub(Sym, Sym)` difference exprs proven by
  `RelWord16Be` offset tables). Frontend-width slice only. This is the
  ledger row ".b/.w imm-link widths" getting its `.w` half built — and
  it half-unblocks kill-row-4 stage 2. `.b` stays ledgered (no consumer
  yet). Range/truncation semantics must match the AS front-end
  (gap-ledger F5, word-imm truncation parity, is IN PLAY here — the
  byte gate plus an explicit parity probe cover it).
- **Already-satisfied demands**: the tranche-4 imm32-d16 deferral
  (`move.l #Ani_Particle, SST_anim_table(a0)`) needs NO ImmLink+symbolic
  -operand extension — once SST offsets are comptime (D1 below), the
  destination folds to a plain `d16(An)` and the existing `.l` ImmLink
  carries it. The deferral resolves by design, not by new machinery.
  `falls_into` (both files fall through Init→Main) is implemented.

## Part 2 — design decisions

### D1. `engine/objects/sst.emp` — a TYPE-ONLY twin of the real SST

New module `engine.objects.sst`, **zero bytes emitted** (the
`engine/system/constants.emp` precedent: "opens no section" — no gate,
no org, no pins, no re-pin tax, no lockstep partner). Contents:

- `pub struct Sst (size: $50)` — the REAL layout, dense, **no `_pad`
  fields needed**: unlike the exhibits' elided shapes, every byte of the
  real SST is a named field (`code_addr` u16, `x_pos`/`y_pos` u32,
  `x_vel`/`y_vel` i16, … `sst_custom [u8; 34]`), each with an
  `@`-asserted offset carrying structs.asm's `$NN` comments.
- **Raw int types only.** No `Angle`/`SubPixel`/`VramTile` newtypes —
  construct-walk #3 (Volence driving) owns that naming; typing fields
  ahead of it would front-run the decision (same ruling as tranche 3's
  `d0: Angle` deferral).
- **Drift guards**: one `ensure(extern("SST_<field>") == $NN, …)` per
  field plus `SST_len`/`SST_sst_custom`, riding the tranche-3 struct-equ
  seam (AS `struct` auto-exports `SST_*`). The `@` assertions pin the
  .emp side to the literals; the ensures pin the AS side to the same
  literals — double-locked, a mismatch fails the link naming the field.
- **Kill-list row** (same-commit, per practice): this is a full mirror
  of structs.asm's SST. Kill condition: `engine/structs.asm` ports —
  sst.emp then BECOMES the definition and the ensures invert or die.
- objroutine's home: an `equ ObjCodeBase = extern("ObjCodeBase")`
  re-export lives here too, so object modules spell
  `#(Label - ObjCodeBase)` without each declaring the extern. If a
  comptime *expression* helper (`objroutine(x)` usable in operand
  position, returning the link expr) is cheap, ship it here; if it
  needs new machinery, spell the subtraction inline and ledger the
  helper (comptime value-fns over labels → link exprs).

### D2. `constants.emp` grows two blocks (fulfilling a jotted ask)

- **Render-flags block**: `pub const RF_COORDMODE = 3`,
  `RF_PRIORITY_SHIFT = 5` + drift ensures (mirrors
  `engine/constants.asm:177-179`).
- **Animation block**: `pub const AF_DELETE = $FB` + ensure (mirrors
  animate.asm) — `particle_anims.emp` explicitly jotted "local mirror +
  drift guard **until the constants twin grows an animation block**".
  Growing it now makes step 4 concrete: particle_anims drops its local
  mirror and imports the twin's. Kill-list rows extend the existing
  constants-twin entry, same kill condition (constants.asm/animate.asm
  block ports).

### D3. The two object modules

`games/sonic4/objects/test_solid.emp` (module
`games.sonic4.test_solid` in section `test_solid`) and
`test_particle.emp` (module `games.sonic4.test_particle` in section
`test_particle`). **Two sections, each map-pinned** — sidesteps any
intra-section multi-module ordering question; the regions are adjacent
so the single gate still covers both (D4). In-file tuning consts
(`PARTICLE_GRAVITY` etc.) stay module-local consts, comments carried.

- Step-1 spelling: qualified `Sst.subtype(a0)`-style access (byte- and
  width-identical displacements; the field names ARE structs.asm's
  names, so the transcribe diff stays mechanical), `jsr`/`jmp` kept,
  explicit widths kept, header contracts carried as comments.
- `TestSolid_Init falls_into TestSolid_Main`, `TestParticle falls_into
  TestParticle_Main`. All four labels `pub` — three have AS-side
  consumers TODAY: `objdef code=TestSolid_Init`
  (data/objdefs/test_objects.asm), `dc.w objroutine(TestParticle)`
  (test_emitter.asm:48, test_stress_emitter.asm:49), plus each file's
  own Main store.
- `Ani_Particle` gains `pub` (particle_anims.emp) — first .emp↔.emp
  cross-module consumer, exactly the seam the handoff predicted the
  imm32 would make .emp↔.emp.

### D4. The gate — first game-side code gate

One define, `SIGIL_EMP_TEST_OBJECTS`, wrapping BOTH adjacent includes
inside `gameObjectBankIncludes` (games/sonic4/main.asm:43-44), else-arm
orgs to the region end per shape — the established engine.inc comment
block carries over verbatim including the "never set for other games"
warning. New class facts, worth naming in the packet: the gate site
lives in a GAME-side macro body (expands at engine.inc:241 inside the
bank), and the org discipline is the bank's (`org $10000`,
`ObjCodeBase`, 64KB budget — the bank's `__BUDGET_OBJBANK` guard keeps
watching in both build shapes).

### D5. AS-side consumers ride the shared link

In the mixed build the AS unit no longer defines the four labels;
`objdef`'s `dc.w objroutine(TestSolid_Init)` and the emitters'
`dc.w objroutine(TestParticle)` become cross-seam word differences
resolved at link. Expected to work through the existing deferred-diff
path (RelWord16Be); step 1 PROVES it with the mixed gate + probes.
NOTE the signed-window subtlety: RelWord16Be range-checks i16, so a
bank offset ≥ $8000 (bank grown past 32KB) would falsely reject —
today's bank is nowhere near; **gap-ledger row** (asl truncates mod
2^16 here; parity question rides the F5 family).

### D6. Step-2 modernize plan (recorded now, executed after step 1)

- Typed params: `proc … (a0: *Sst)` + bare-field access
  (`subtype(a0)`), per the sst_overlay exhibit.
- `jsr` → `jbsr`, `jmp` → `jbra` tail-calls (4 sites: solid's
  Draw_Sprite jmp; particle's ObjectMove/AnimateSprite jsr + Draw_Sprite
  jmp). Cross-seam targets sit ~$10000-ish away — if in bra.w reach,
  −2 B/site and a re-pin; the relaxer decides.
- `clobbers()`/`preserves` contracts lifted from the header comments
  (test_solid Main: `clobbers(d0-d3/a1)`; test_particle both:
  `clobbers(d0-d4/a1-a3)`; Init procs per checklist ruling — declare
  what's verified).
- No conditionals in either file, so the unsized-conditionals rule has
  no sites; no `lea.l` noise; binary masks n/a.
- `#6<<RF_PRIORITY_SHIFT` and `#RF_COORDMODE` immediates fold comptime
  off the constants twin — byte-identical, no ImmLink involvement.

### D7. Verification plan (step 1)

- Branches: sigil worktree `port-tranche6`, aeon `sigil-emp-tranche6`.
- Byte gates both shapes (plain + `__DEBUG__`) at freshly derived pins;
  mixed full-ROM acceptance with all prior gates stacked + the new one;
  gate-off neutrality sha256 ×3 + demo clean (the define must be
  meaningless for demo).
- `tranche6_negative_probes.rs`, minimum set: (1) misspelled cross-seam
  SST field name → loud link failure; (2) doctored sst.emp field offset
  → its OWN drift ensure fires (not a byte mismatch downstream); (3)
  `.w` ImmLink range violation → loud (value > $FFFF window); (4)
  word-imm truncation PARITY probe vs the AS front-end (F5 family); (5)
  misspelled `objroutine` target in an AS-side consumer dangles loud
  while the undoctored control resolves; (6) `falls_into` pair broken
  (reordered) → compile error, proving adjacency is enforced not
  accidental.
- Word-ImmLink feature lands with its own unit/steering tests in the
  frontend (quick-form steering already exists for `.l` — extend the
  matrix to `.w`, including the moveq/quick-family steer).

## Part 3 — retrospect seeds (to check at step 3, not conclusions)

- The exhibit prelude (`examples/game/prelude.emp`) is now PROVEN
  layout-wrong against the real engine. Post-tranche, either re-point
  the exhibits at the real `engine.objects.sst` shapes or banner them
  as historical — decide at retrospect, don't let the teaching material
  contradict the production truth silently. (`ScriptPc`/`resume` needs
  care: the script construct discovers the resume slot BY TYPE; the
  real engine's resume slot is `code_addr @ $00`. The first scripted
  badnik will force this reconciliation — jot for S2-D12b's demand
  moment.)
- `routine`/`anim`/`spawn` helper family: NOT built this tranche
  (no call sites in the targets use them; test objects write fields
  directly). Their production forms want the real facts learned here
  (routine = word ImmLink store to `code_addr`). First badnik demand.
- ObjDef as a typed .emp struct (the 26-byte template image) — big,
  real, and NOT demanded until an ObjDef consumer file ports
  (data/objdefs/test_objects.asm is a natural tranche-7/8 companion to
  collision.asm).

## Kill-list rows to add (same-commit as the code)

1. sst.emp Sst struct mirror ← structs.asm SST (kill: structs.asm ports).
2. constants.emp RF block ← constants.asm (kill: constants block ports).
3. constants.emp AF_DELETE ← animate.asm (kill: animate.asm ports);
   SUPERSEDES particle_anims' local-mirror row when step 4 lands.

## Gap-ledger rows to add

1. RelWord16Be signed-window vs bank offsets ≥ $8000 (asl truncation
   parity, F5 family).
2. Comptime expression-position `objroutine(label)` helper if it turns
   out to need machinery (else it ships in D1 and this row is void).
3. `.b` ImmLink still unbuilt (the `.w` half lands here; consumer-gated).

## Addendum (Volence review, 2026-07-10) — RATIFIED, with a scheduling decision

Design approved as written (type-only twin, raw ints). Volence raised
"full how-we-want-SST now?"; settled: **construct-walk #3 (the Sonic
newtype set vs player physics, Volence driving) is PULLED FORWARD to
between tranches 6 and 7** — the typing pass runs against
player_ground/collision hot code, then back-props onto sst.emp + the two
object modules (a type-annotation diff over a 2-file corpus), so
collision.asm (T7, 32 SST refs) ports ONCE into the final typed surface.
Tranche-6 packet must carry walk #3 as a named gate item for tranche 7,
alongside the collision.asm structural-changes re-ask. sst.emp is to be
authored walk-#3-ready: fields grouped/commented so newtypes drop in as
annotations, not a rewrite.
