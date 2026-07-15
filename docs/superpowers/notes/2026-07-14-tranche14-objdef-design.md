# Tranche 14 — objdef data file (the ObjDef-twin driver): Step-0 design note

**Branch:** sigil `port-tranche14` (aeon worktree to be seeded at code-start).
**Status:** STEP 0 — recon + design. No code. Awaiting Volence's gate.
**Steering (Volence, 2026-07-14 ruling):**
1. Optionality must be solved as a GENERAL LANGUAGE MECHANISM (record
   defaults / FRU vs named args) — this note sizes BOTH.
2. The ObjDef↔Sst offset ensure-chain is the tranche's NON-NEGOTIABLE
   payoff — machine-checking the burst-copy correspondence is the whole
   reason this file is the "ObjDef-twin driver."
3. Hazard scope: row 1029 movem grep = step-0 enumeration (own commit if
   it fires); row 1005 EntityScanState = unblock ASSESSMENT only, no
   scope creep; row 1023 overlay-write = background.

---

## 1. What t14 ports

The AS `objdef` macro (`aeon/engine/macros.asm:82`) emits a **26-byte v2
archetype template** — a ROM image spawned by `Load_Object`. The port
target is the ENGINE emitter mechanism, proven by a representative
game-data consumer:

- **Engine (the port):** an `ObjDef` struct-twin + an `objdef(...)`
  emitter, new `.emp` (home decided in §3), gated.
- **Language feature:** the optionality mechanism (§5).
- **The payoff:** the ObjDef↔Sst ensure-chain (§4).
- **Proof:** `games/sonic4/data/objdefs/test_objects.emp` (4 defs) ported
  and byte-gated against the AS-macro `test_objects.asm`; `.asm` twin
  stays as the byte reference (Plan-6 data-file proof model).

This is the corpus's **first struct-typed data item** — every existing
`.emp` data item is a `[u8;N]` byte array (`Sine_Table`,
`CellOffsets_XFlip`). That first is the point.

## 2. Macro anatomy (the thing we must reproduce byte-for-byte)

`objdef code,map,art,zpri,xvel,yvel,wdth,hght,col,anims,anim,sub,rfbits,statbits`
— 14 params, **2 required** (`code`, `map`), **12 optional** (default 0).
Emits, contiguously from `ODZ_START` (macros.asm:140-154):

| ObjDef off | bytes | emission | notes |
|---|---|---|---|
| `$00` | w | `objroutine(code)` | code label − ObjCodeBase (link diff) |
| `$02` | w | `xvel` | |
| `$04` | w | `yvel` | |
| `$06` | b | `rfbits \| (zpri<<RF_PRIORITY_SHIFT)` | **COMPUTED** (packing) |
| `$07` | b | `col` | collision_resp |
| `$08` | l | `map` | mappings ptr |
| `$0C` | w | `art` | art_tile |
| `$0E` | b | `wdth` | |
| `$0F` | b | `hght` | |
| `$10` | b | `anim` | |
| `$11` | b | `sub` | subtype |
| `$12` | l | `anims` | anim_table ptr |
| `$16` | b | `statbits` | status |
| `$17` | b | `0`* | angle (*macro emits it from `sub`? no — `dc.b ODZ_ANI, ODZ_SDEF` at $10/$11; angle is the `dc.b ODZ_BITS, 0` low byte) |
| `$18` | w | `0` | pad ($20-$21 at spawn) |

Total = **26 bytes**, backstopped by `if (*-ODZ_START) <> 26: fatal`.

Two macro behaviors the emitter MUST carry:
- **render_flags is computed** — `rfbits | (zpri << RF_PRIORITY_SHIFT)`
  (RF_PRIORITY_SHIFT = 5, constants.emp:37). Not a field copy.
- **validation** — `if ODZ_PB > 7: fatal "priority exceeds 7"`, plus
  `code`/`map` required. These become emitter checks (`ensure` /
  refinement type).

Because render_flags is computed and priority is validated, the emitter
**cannot** be a bare struct literal — a comptime `fn objdef(...) -> ObjDef`
is needed regardless. **The optionality fork is therefore about that
fn's parameter ergonomics (§5), not about whether a fn exists.**

AS size-suffix quirk (why param names are `wdth`/`hght`/`zpri`/`rfbits`):
AS substitutes params into `dc.w`/`dc.b` size suffixes, so a param named
`w` would corrupt `dc.w`. IRRELEVANT to `.emp` — the port can use clean
names (`width`, `height`, `priority`, `render_flags`).

## 3. The ObjDef struct-twin

Model: `sst.emp` (`aeon/engine/objects/sst.emp`) — a TYPE-ONLY twin that
emits zero bytes, `@`-pins each field offset to the literal, and
`extern("SST_*")` drift-guards the AS side across the link seam.

The ObjDef twin is the same pattern, a **compact 26-byte** record:

```
pub struct ObjDef (size: 26) {
    code_addr:      ObjRoutine   @ $00,   // objroutine(code) — link diff
    x_vel:          Velocity     @ $02,
    y_vel:          Velocity     @ $04,
    render_flags:   u8           @ $06,   // rfbits | (priority<<5)
    collision_resp: u8           @ $07,
    mappings:       u32          @ $08,
    art_tile:       VramArtTile  @ $0C,
    width_pixels:   HitboxDim    @ $0E,
    height_pixels:  HitboxDim    @ $0F,
    anim:           AnimId       @ $10,
    subtype:        u8           @ $11,
    anim_table:     u32          @ $12,
    status:         u8           @ $16,
    angle:          Angle        @ $17,
    pad:            u16          @ $18,    // $20-$21 at spawn, re-inited
}
```

Field types reuse the `engine.types` domain vocab (ObjRoutine, Velocity,
VramArtTile, HitboxDim, AnimId, Angle — all erase to raw width). There is
no AS-side `ObjDef` *struct* to drift-guard against (the macro emits raw
`dc`s), so ObjDef's outward guard is the **26-byte total** and the
ensure-chain (§4), not `extern("OBJDEF_*")`.

**Home decision (proposed):** co-locate ObjDef in `sst.emp` (rename its
module intent to "the spawn-template structs") OR a new
`engine.objects.objdef`. Recommendation: **sst.emp**, because the
ensure-chain (§4) needs BOTH structs' `offsetof` visible in one place,
and ObjDef IS the spawn companion to Sst. This also answers row 1005's
"no shared struct home" objection with a natural pairing rather than a
premature `engine.structs` module. (Open to Volence's call.)

## 4. THE PAYOFF — ObjDef↔Sst burst-copy ensure-chain

`Load_Object` (`load_object.emp:44-52`) burst-copies ObjDef → Sst:

```
move.w  (a2)+, Sst.code_addr(a1)    // ObjDef $00 → Sst $00       (shift 0)
lea     Sst.x_vel(a1), a3
move.l  (a2)+, (a3)+  ×6            // ObjDef $02-$19 → Sst $0A-$21 (shift +8)
```

So the correspondence is: **code_addr copies at shift 0; the entire
template block copies at shift +8** (ObjDef `$02` lands at Sst `$0A`).
The machine-check that makes ObjDef a real twin of Sst — and the reason
this file was named the driver — is:

```
// code_addr word: no shift
ensure(offsetof(ObjDef, code_addr) == offsetof(Sst, code_addr),
       "ObjDef/Sst code_addr correspondence broken")

// template block: +8 shift, per field
ensure(offsetof(ObjDef, x_vel)        + 8 == offsetof(Sst, x_vel),        "...")
ensure(offsetof(ObjDef, y_vel)        + 8 == offsetof(Sst, y_vel),        "...")
ensure(offsetof(ObjDef, render_flags) + 8 == offsetof(Sst, render_flags), "...")
ensure(offsetof(ObjDef, collision_resp)+ 8 == offsetof(Sst, collision_resp),"...")
ensure(offsetof(ObjDef, mappings)     + 8 == offsetof(Sst, mappings),     "...")
ensure(offsetof(ObjDef, art_tile)     + 8 == offsetof(Sst, art_tile),     "...")
ensure(offsetof(ObjDef, width_pixels) + 8 == offsetof(Sst, width_pixels), "...")
ensure(offsetof(ObjDef, height_pixels)+ 8 == offsetof(Sst, height_pixels),"...")
ensure(offsetof(ObjDef, anim)         + 8 == offsetof(Sst, anim),         "...")
ensure(offsetof(ObjDef, subtype)      + 8 == offsetof(Sst, subtype),      "...")
ensure(offsetof(ObjDef, anim_table)   + 8 == offsetof(Sst, anim_table),   "...")
ensure(offsetof(ObjDef, status)       + 8 == offsetof(Sst, status),       "...")
ensure(offsetof(ObjDef, angle)        + 8 == offsetof(Sst, angle),        "...")
// pad $18 → Sst prev_anim/anim_frame $20-$21 (re-inited at spawn; guarded by total size)
ensure(offsetof(ObjDef, pad) + 8 == offsetof(Sst, prev_anim), "...")
```

**CONFIRMED buildable today.** The survey verified `offsetof(S, f)`
evaluates to `Value::Int(offset)` in ANY expression position, including
`ensure()` conditions and arithmetic (`eval/expr.rs:166`,
`eval/guards.rs:97`, `tests/eval_layout.rs`). Both `offsetof` calls are
comptime `Int` → the `==` is compared EAGERLY at comptime (no `extern()`
link deferral, unlike the `SST_*` drift guards). This is the preferred
form — the chain fails the BUILD, comptime, naming the drifted field.

Home caveat: both structs must be visible where the chain lives. sst.emp
co-location (§3) makes both `offsetof` in-file — no cross-module-offsetof
question on the non-negotiable payoff.

The magic constant `+ 8` gets a `const TEMPLATE_COPY_SHIFT = 8` with a
site comment tying it to `lea Sst.x_vel(a1)` (Sst.x_vel − Sst.code_addr −
2 = the byte gap the compact ObjDef closes).

## 5. THE OPTIONALITY FORK (sized both ways) — FINALIZED

The survey collapses the fork. Because `render_flags` is a COMPUTED field
(§2), a mediating `fn objdef(...) -> ObjDef` exists no matter what — so
the question is only how that fn receives its 12 optional inputs, and
**both "sides" of Volence's framing already have their per-element half;
each is missing only its don't-repeat-yourself completion:**

### Side A — named args + DEFAULT PARAMS (the fn path)
`objdef(code: "TestSolid_Init", map: Map_TestObj, priority: 3, width: 16,
        height: 16, collision: COLLISION_SOLID)`
- **Named args ALREADY EXIST** (`ast.rs:1037` `Arg.name`, `eval/call.rs:522`
  named binding, positional-after-named rejected, arity checked). The
  ONLY missing piece is **default parameter values** — params today are
  `(name, Type, span)` with no default slot (`ast.rs:757`).
- Mirrors the AS keyword macro `objdef code=…, zpri=3, …` 1:1 — maximally
  faithful; best call-site readability (12 optionals demand names).
- Required addition: `(name, Type, Option<default>, span)` + a
  default-fill in `bind_args`. **ONE localized feature** on machinery that
  already does named binding + arity. Survey's least-invasive finding.
- Bonus totality win: `code`/`map` get NO default → omission is the
  existing `missing argument` error (required-ness for free); `priority`
  becomes a refinement param `(u8 where 0..7) = 0` → "priority exceeds 7"
  is a COMPILE error, upgrading the macro's runtime `fatal`.

### Side B — per-field struct defaults + resurrect `..`/FRU (the struct path)
`ObjDef { ..default, code: …, priority: 3 }`
- **Per-field struct defaults ALREADY EXIST** (`StructField.default`,
  `field: default` elision, `eval/literals.rs:194`). The blocker is the
  exhaustiveness rule: every field must be NAMED, so 12 optionals = 12
  `field: default` lines = the "reads as noise" problem.
- The bulk `..` form was **BUILT AND RETIRED** at the 2026-07-09
  checkpoint ("the page couldn't say WHICH fields it covered") and
  **re-ledgered for exactly this case** — "a struct with enough defaults
  that per-field `default` reads as noise." ObjDef IS that case.
- **But it does NOT solve the emitter.** render_flags is computed, so a
  struct literal (even with `..`) can't pack `priority<<5` — you'd still
  route through `fn objdef`. `..` would only tidy an intermediate
  `ObjDefArgs` wrapper the fn unpacks → strictly more ceremony at the
  call site than Side A's direct `objdef(priority: 3)`.

### RECOMMENDATION: Side A — comptime-fn default parameters.
It is the single general feature that completes the already-shipped named
args into the exact AS-macro ergonomics, keeps the computed render_flags
and priority-validation INSIDE the fn, needs no call-site wrapper, and is
the survey's least-invasive change. It is general (every future emitter
fn benefits — the "general mechanism" the ruling demanded) and totality-
positive (required params + refinement-typed priority). The re-ledgered
`..` is a real want but the WRONG tool for a computed-field record; it
stays ledgered (and the corpus sweep, §8, re-checks it against sst
overlay / any many-default struct).

## 6. Capability baseline — CONFIRMED (survey)

- **Struct literal in data position:** YES, EXHAUSTIVE with named elision
  — every field named, `field: default` takes a declared default, no
  silent fill. Diags `[struct.missing-field]`/`[struct.no-default]`/
  `[struct.unknown-field]` (`eval/literals.rs:194`).
- **Per-field defaults:** YES (`StructField.default`). **Bulk `..`/FRU:**
  NO — built + retired 2026-07-09, re-ledgered for the many-default case.
- **Comptime fn:** named args YES; **default params NO** (the §5 add);
  arity checked both directions (`eval/call.rs:265,552`).
- **offsetof:** any expression position incl. `ensure()` arithmetic →
  `Value::Int` (`eval/expr.rs:166`). Powers §4 at comptime.
- **ensure+extern:** `extern()` → `LinkExpr`, guard DEFERS to link
  (`eval/guards.rs:102`); two `offsetof` → both `Int`, EAGER comptime
  compare. §4 uses the eager form.
- **data items:** `data X: T = …` defines symbol `X` at the bytes
  (`lower/mod.rs:1007`), items sit contiguously in decl order; `pub`
  exports across the link seam.
- **objroutine emission:** NO expression-position helper (gap-ledger row
  680-685, OPEN); code_addr emits as a **data-initializer** link
  difference `extern(code) − OBJ_CODE_BASE` (D-PP.3 data-init +
  Value16Be unsigned window, row 702-707 — SHIPPED for the objroutine
  consumer words). The emitter takes `code` as a **string** and spells
  `extern(code) − OBJ_CODE_BASE` internally, sidestepping the open
  label-in-expr ask.

## 7. Hazard assessments (per Volence's scope)

- **Row 1029 (movem small-block-copy anti-pattern grep) — FIRES NEGATIVE.**
  Enumerated `children.asm`: all 24 `movem` sites are register
  save/restore pairs (`movem …,-(sp)` / `movem (sp)+,…`), including the
  `d3-d4/a0-a1` shapes at :198/:202/:260 — the CORRECT use of movem. The
  anti-pattern (movem block-copy TO an address register) is ABSENT.
  `load_object`'s own burst copy was already converted to `move.l
  (a2)+,(a3)+` in the t13 step-5 follow-up. **Nothing to peephole; no
  commit owed.** (If a future children.asm port surfaces a real
  block-copy site, the peephole rides its own commit then.)
- **Row 1005 (EntityScanState struct-twin) — UNBLOCK ASSESSMENT: STILL
  BLOCKED, stays a note.** The row waits on `offsetof` in **ABSOLUTE-EA
  position** (`Entity_Scan_State + sizeof*N + field` as a memory operand)
  OR a shared struct home. t14 exercises `offsetof` in EXPRESSION and
  DATA-value position (the ensure-chain + the emitter) — a DIFFERENT
  position from the absolute-EA operand the row needs; the survey
  confirms expression-position works but says nothing about the operand
  lowering path. So t14 does NOT unblock it. If §3 lands a shared
  `engine.objects` structs home, that satisfies the row's SECOND
  condition — revisit at the corpus sweep (§8), still not t14 scope.
- **Row 1023 (overlay-write syntax) — BACKGROUND.** offsetof workaround
  shipped; no action.

## 8. Tranche plan (port loop)

- **Step 1 (transcribe):** ObjDef twin + `objdef()` emitter fn +
  ensure-chain; the DEMANDED language feature (§5 winner) ships here
  (demanded-features law). `test_objects.emp` byte-gated vs
  `test_objects.asm`, both shapes, gate-off neutral, mixed-build accept.
- **Step 2 (modernize):** house format / brace-indent; the emitter's
  own control flow.
- **Step 3 (retrospect):** the optionality feature IS the headline
  step-3(a) ask; interrogate for the rest.
- **Step 4 (construct pass):** ObjDef emitter is itself the construct;
  scan the consumer for repeated shapes.
- **Step 5 (optimize):** data emitter — no hot path; likely "no changes,
  recorded why."
- **Step 6 (corpus sweep):** the optionality mechanism is a NEW language
  feature PRIOR files could use — enumerate struct-literal / defaulting
  sites across the corpus (sst overlay, entity_window's deferred
  EntityScanState, any `[u8;N]` that wants a struct). Retrofit clean /
  ledger blocked.

## 9. Decisions for Volence's gate

1. **Optionality mechanism → RECOMMEND comptime-fn default parameters**
   (§5 Side A). Named args already ship; this is the one general feature
   that completes them, keeps the computed render_flags + priority check
   in the fn, needs no call-site wrapper, is least-invasive, and is
   totality-positive. `..`/FRU stays ledgered (wrong tool for a
   computed-field record; re-swept in §8). — GATE: approve default-params
   as the demanded step-1 feature, or direct otherwise.
2. **ObjDef home → RECOMMEND co-locate the struct + ensure-chain in
   sst.emp** (both `offsetof` in-file → payoff carries no cross-module
   risk), emitter `fn objdef` in a thin new `engine/objects/objdef.emp`
   that `use`s ObjDef. sst.emp's header widens to "the spawn-template
   structs." — GATE: sst.emp co-location vs a fresh `engine.objects`
   structs module.
3. **ensure-chain form → RESOLVED: eager two-offsetof comptime compare**
   (§4/§6) — both `Value::Int`, no link deferral. No open question.
4. **Priority validation → RECOMMEND refinement param** `(u8 where 0..7)
   = 0` (compile error, upgrades the macro's runtime `fatal`). — GATE:
   confirm the totality upgrade is wanted (it is a behavior improvement,
   not a byte change — the emitted byte is identical for valid input).
