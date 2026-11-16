---
kind: audit-stage
system: emp-language
stage: "4 Reviewer B"
agent: Opus
status: done
description: "Best known as an assembler, below the best common tools as a game-logic language; comptime is powerful but unsound (caller-scoped names, unchecked uncalled bodies, unenforced types). Patch the exact tier now (comptime soundness, checked dispatch and bindings, code_word scripts, state tables, compositional budgets); .emp2 only as a gated, statement-shaped choreography tier after comptime soundness; functional everywhere: no."
---
# EMP language: reviewer B

> [!verdict] Verdict and disposition
> - **As an assembler: best known.** The contract layer (transitive `clobbers`, verified `preserves`/`out`, flag outs, contexts, `@budget` over a real cycle table) has no counterpart in any assembler the research checked (stage 1 §3.1). The comptime layer is Zig-class.
> - **As a language for game logic: below the best common tools.** Game state machines are written as untyped immediates (`move.w #X - ObjCodeBase, code_addr(a0)`, 25 sites, MEASURED), hand-numbered state constants, parallel tables and comment-held protocols. The constructs built for exactly this (`script`, `dispatch`, `table`) have 0, 0 and 1 aeon uses. **`script` cannot be used on aeon at all**: its `code_word` encoding was never built (finding 2).
> - **The headline guarantee has a hole exactly at the game-logic boundary.** All 11 `jsr (aN) as Type` dispatch sites are trusted, not checked, and 71 engine contracts depend on that trust (MEASURED, finding 1).
> - **Disposition: patch.** Keep the exact tier as the engine's language. Close the state-machine gap inside it first (ranked list in §8). A functional `.emp2` tier is a **rewrite beside**, gated on a prototype, and only for game choreography (boss phases, enemy scripts, events), never for physics or the engine.
> - **Owner question, "functional everywhere": no.** The escape hatch would have to cover the interrupt handlers, the Z80 driver, DMA and VDP loops, decompression and the streaming caches: about 40% of the code lines and all of the cycle-bound ones (§8.4). That is the exact tier under another name.

**Walk B result.** I wrote the player spindash state and a Gunstar/Thunder Force IV workload three ways each (§W). The improved exact tier (checks and sugar only) removes about 40% of the source lines and most of the hand-kept protocols at **zero cycle and byte cost**. The functional tier removes about 75% more lines on the choreography workloads, at an ESTIMATED 1.0× to 1.15× cycle cost, but only if four lowering rules hold. On the physics state it saves little and adds semantic risk. **The line between tiers belongs at "what happens next" (state choreography) versus "how a frame's numbers are computed" (physics, engine), not at engine versus game.**

---

## 1. Requirements

What the language must let the suite write, and at what cost.

| Workload | What the language must express | Cost bound |
|---|---|---|
| **Sonic 4 vertical slice** | Player physics bit-exact to the S3K reference (fixed-point wraps, `asr` flooring); a 13-state player machine with enter/exit hooks (`player_common.emp:1982-2107`); ~20 to 40 badnik/gimmick objects; one boss per zone | Exact tier: 0 cycles over hand asm. Logic must fit the release object budget (stage 1 §4.3: `RunObjects` 11k to 17k per tick, `Player_Main` 8.8k, MEASURED release) |
| **Mega-acts / infinite levels** | Streaming and caches in the exact tier; level bake in comptime or tools; data formats typed at the item (`data X: [T; N]`, `table`) | Streaming already overruns on diagonal frames (stage 1 §4.3), so 0% compiler tax there |
| **Named non-Sonic: Gunstar Heroes / Alien Soldier boss** | A core plus 4 to 8 linked parts with typed parent links; phase state machine; per-frame "while waiting, also check HP" logic; hit events; spread and aimed projectiles, 64 to 128 live | Logic ≤ 5% of a frame over hand asm (stage 1's +5.6k cycles = 4.4% for a C-class compiler is the ceiling I accept) |
| **Named non-Sonic: Thunder Force IV enemy script** | 24+ enemies, each a timed sequence (move N frames, turn, fire, repeat, exit), many sharing a template with parameters | Per-enemy logic within ~10% of hand; ROM size ≤ 1.25× |

**Budgets.** Exact tier: 0 cycles, 0 bytes of tax (by construction). Any compiled tier: ≤ 1.15× cycles on logic, ≤ 1.25× bytes, **zero heap**, in-place slot mutation, all locals that live across a frame boundary inside the 32-byte `sst_custom` window (`sst.emp:82`) or a declared overlay, with a loud error on overflow.

**Worst case it must survive.** 80 sprites + 128 bullets + a boss with 6 parts in one frame, release shape. A compiled tier must stay out of every IRQ context and every Z80-bus window.

**Interfaces.** Sigil's IR (`IrStreamer`; Spec2 header: no IR changes), the contract closure (`corpus_contracts.rs`), `@budget` (`cycle_budget.rs`), Oracle debug info (source maps for generated code), Aurora (reads object definitions).

**Non-goals.** Garbage collection, runtime closures, dynamic dispatch beyond tables, runtime floats, a C ABI.

---

## W. Walk B: the same logic written three ways

### W1. Real piece: `PState_Spindash` and the player state machine

Source: `games/sonic4/player/player_spindash.emp:89-195` plus its registration in `config/constants.emp:248-339` and `player_common.emp:1982-2107`.

**(a) Current `.emp`, as shipped** (abridged; every line is real):

```
const PBLK_QUADRANT = 16                                   // :40, hand-typed layout literal
const PBLK_JUMPBUF  = 17                                   // :41
ensure(PBLK_QUADRANT == offsetof(PlayerBlock, quadrant), …) // :51, the pin that keeps it honest
ensure(PBLK_JUMPBUF == offsetof(PlayerBlock, jump_buffer), …)

pub proc PState_Spindash (a0: *Sst) clobbers(d0-d7/a1-a2) { // a4 is read but not declared
        btst    #BUTTON_DOWN_BIT, Ctrl_1_Held
        beq     .release
        move.w  PlayerV.spindash_charge(a0), d0
        beq     .rev
        move.w  d0, d1
        asr.w   #5, d1
        sub.w   d1, d0
        move.w  d0, PlayerV.spindash_charge(a0)
.rev:   tst.b   PBLK_JUMPBUF(a4)
        …                                                  // 63 instruction lines, 30 comment lines
.floor_gone:
        moveq   #PSTATE_AIRBALL, d0                         // a byte offset, hand-numbered elsewhere
        jbra    Player_SetState
}
// Registration, in two other files: PSTATE_SPINDASH = 4 (constants.emp:294), a row in each of
// Player_States / PState_EnterHooks / PState_ExitHooks (player_common.emp:1985, :2062, :2091),
// PSTATE_COUNT, the name in pstate_values_ok's list (constants.emp:327), and ~60 comment lines
// (constants.emp:248-291) explaining why the four edits must agree.
```

**(b) Improved exact `.emp`** (every instruction identical; new surface marked ★):

```
pub proc PState_Spindash (a0: *Player, a4: *PlayerBlock)    // ★ typed a4: a checked input
        conforms PlayerState {                              // ★ contract taken from the type
        btst    #BUTTON_DOWN_BIT, Ctrl_1_Held
        beq     .release
        move.w  spindash_charge(a0), d0                    // ★ *Player = Sst + PlayerV view
        …
.rev:   tst.b   jump_buffer(a4)                            // ★ no PBLK_* const, no pin
        …
.floor_gone:
        set_state PlayerStates.AirBall                     // ★ = moveq #12,d0 ; jbra Player_SetState
}

// ONE declaration replaces the 13 PSTATE_* consts, three tables, the count ensures and
// pstate_values_ok. Emits the same three dc.w tables (one per column) and exports
// PlayerStates.<Row> as cross-module ordinals pre-scaled x2 (S2-D12a).
pub states PlayerStates (encoding: word_offsets, columns: run/enter/exit) bound PlayerState {
        Ground:   run PState_Ground,   enter PHook_GroundEnter,   exit PHook_Null,
        Spindash: run PState_Spindash, enter PHook_SpindashEnter, exit PHook_SpindashExit,
        …
}
```

**(c) Functional `.emp2`**:

```
state Spindash (p: Player, blk: PlayerBlock) : PlayerStates =
  if not held(DOWN) then release(p, blk)
  else
    p.charge := p.charge - (p.charge >>a 5)          -- arithmetic shift: floors, like asr
    when take(blk.jump_buffer) do
      p.charge := min(p.charge + CHARGE_STEP, CHARGE_MAX); sfx SPINDASH
    match floor_follow(p, blk, speed: 0)              -- exact .emp helper, called by contract
      | Gone -> AirBall
      | _    -> Spindash

release(p, blk) =
  p.ground_speed := facing(p) ?? (SPINDASH_BASE + (p.charge >> 8) * $80)  -- ?? = negate if flipped
  camera_hold 16; blk.jump_buffer := 0; sfx DASH
  goto Roll now                                       -- transition, and run Roll this frame
```

| | (a) current | (b) improved exact | (c) `.emp2` |
|---|---|---|---|
| Source lines for this state (body + registration + pins) | 107 body + 22 file pins + ~6 registration edits in 2 files + the ~90-line PSTATE rule block shared by all 13 states | ~75 body (comments shrink with the prose they no longer need) + 1 row | ~16 + 1 row (ESTIMATED from the drafts above) |
| Bytes | **222** (`$111DC-$112BA`, `s4.lst:2207-2216`, release, MEASURED) | 222, identical by construction | 222 to ~270 (ESTIMATED: +0 if the four rules below hold; +48 if the `??` negate and the decay skip are not recognised) |
| Cycles, common charging path (held, no tap, floor present), callees excluded | **≈316** (ESTIMATED: 31 instructions summed from M68000UM §8 times, short branches) | ≈316, identical | ≈316 to 360 (ESTIMATED: +36 on zero-charge frames if the compiler drops the `beq .rev` skip; a naive `facing * x` would add a 70-cycle `muls`) |
| Bug surface | stale state numbering (4 edits kept equal by a popcount proof); untyped `a4` input; trusted dispatch bound; 63 lines of flags and registers | numbering, input and dispatch bound all compiler-checked; flags/registers remain | flags/registers gone; **new risk: rounding semantics** (`>>a` vs `/`) and hidden cost |

> [!note] What the walk shows for the player
> The physics state gains little from `.emp2`: the logic is arithmetic whose exact rounding *is* the S3K behaviour, and the hand code is already short. Almost all of the waste is in the **registration** (four hand-synced edits, a proof, 90 lines of explanation) and the **layout pins**. Both are fixed in the exact tier at zero cost.

### W2. Named non-Sonic workload: run-and-gun boss + shmup enemy

Not in the corpus. Drafted against aeon's real object model (table-less `code_addr` dispatch, `core.emp:595-599`; `CreateChild_Normal`; `sst_custom` overlays).

**(a) Current `.emp`** (as aeon would write it today; boss core shown, arms follow `TestChildPart_Main`'s shape):

```
vars BossV: Sst.sst_custom { timer: u16, shots_left: u8, hp: u8 }

pub proc Boss_Enter (a0: *Sst) clobbers(d0-d3/a1) preserves(a0) {
        subq.w  #1, y_pos(a0)
        subq.w  #1, BossV.timer(a0)
        bne     .draw
        move.b  #3, BossV.shots_left(a0)
        move.w  #VOLLEY_GAP, BossV.timer(a0)
        move.w  #Boss_Phase1 - ObjCodeBase, code_addr(a0)   // untyped, unchecked target
.draw:  jbra    Draw_Sprite
}
pub proc Boss_Phase1 (a0: *Sst) clobbers(d0-d6/a1-a2) preserves(a0) {
        cmpi.b  #BOSS_HP/2, BossV.hp(a0)      // must be repeated in EVERY waiting state
        bhi     .alive
        move.w  #Boss_Phase2 - ObjCodeBase, code_addr(a0)
        jbra    Boss_Phase2
.alive: subq.w  #1, BossV.timer(a0)
        bne     .draw
        moveq   #5, d2
        jbsr    Boss_FireSpread
        move.w  #VOLLEY_GAP, BossV.timer(a0)
        subq.b  #1, BossV.shots_left(a0)
        bne     .draw
        move.b  #3, BossV.shots_left(a0)
        move.w  #REST_FRAMES, BossV.timer(a0)
.draw:  jbra    Draw_Sprite
}
data boss_parts: [SpawnDesc; 3] = [ … ]
data boss_parts_term: u16 = 0          // ADJACENCY-CRITICAL, by comment (test_parent.emp:166 idiom)

// TF4 enemy: three procs (Init / Leg / Exit), same shape, ~30 lines.
```

**(b) Improved exact `.emp`** (`script` with the `code_word` encoding, a `table` sentinel list, typed spawn):

```
script Boss (a0: *Sst) (encoding: code_word, base: ObjCodeBase) shows Draw_Sprite {
        spawn_children boss_parts               // table (sentinel): terminator owned by the item
    .enter:
        subq.w  #1, y_pos(a0)
        subq.w  #1, BossV.timer(a0)
        beq     .phase1
        yield   .enter
    .phase1:
        move.b  #3, BossV.shots_left(a0)
    .volley:
        move.w  #VOLLEY_GAP, BossV.timer(a0)
    .wait:  cmpi.b  #BOSS_HP/2, BossV.hp(a0)    // still hand-repeated per wait loop
        bls     .phase2
        subq.w  #1, BossV.timer(a0)
        beq     .fire
        yield   .wait
    .fire:  moveq   #5, d2
        jbsr    Boss_FireSpread
        subq.b  #1, BossV.shots_left(a0)
        bne     .volley
        wait_frames #REST_FRAMES, BossV.timer(a0)   // HP NOT checked during this park
        jbra    .phase1
    .phase2: …
}

script Zig (a0: *Sst) (encoding: code_word, base: ObjCodeBase) shows Draw_Sprite {
        move.w  #-$200, x_vel(a0)
        move.b  #4, ZigV.legs(a0)
    .leg:   move.w  #24, ZigV.timer(a0)
    .fly:   jbsr    ObjectMove
        subq.w  #1, ZigV.timer(a0)
        beq     .turn
        yield   .fly
    .turn:  neg.w   y_vel(a0)
        jbsr    Enemy_FireAimed
        subq.b  #1, ZigV.legs(a0)
        bne     .leg
    .exit:  jbsr    ObjectMove
        despawn_offscreen
        yield   .exit
}
```

**(c) Functional `.emp2`**:

```
boss Gunner (hp: 64) {
  parts  { arm_l = Arm(side: -1); arm_r = Arm(side: +1); turret = Turret }
  phase Enter  = during 60 frames { y -= 1 } then Phase1
  phase Phase1 = loop { repeat 3 { wait 20 frames; fire spread(5, aimed: player) }; wait 60 frames }
                 until hp <= 32 then Phase2      -- checked at EVERY resume, nested waits included
  phase Phase2 = …
  on hit(d)    = { hp -= d; flash 4 frames }
}
part Arm(side: i8) = follow parent at arm_offset[parent.phase] * side

enemy Zig = { vel.x := -2.0 px
              repeat 4 { during 24 frames { move }; vel.y := -vel.y; fire Aimed }
              forever { move; when offscreen despawn } }
```

| | (a) current | (b) improved exact | (c) `.emp2` |
|---|---|---|---|
| Lines, boss core + Zig (ESTIMATED from the drafts) | ~45 + ~30 | ~30 + ~16 | ~10 + ~4 |
| Per-frame cycles, Zig flying (own code, callees excluded) | ≈36 (`subq.w d16(a0)` 16 + `bne` 10 + `jbra Draw_Sprite` 10) | ≈48 (adds `move.w #seg,(a0)` 12 at the self-yield; ESTIMATED) | ≈48 if `during` lowers like (b); the counter is one compiler-allocated window word |
| 24 enemies + 1 boss, per frame | baseline | +≈300 cycles, 0.2% of a 128,006-cycle frame (ESTIMATED) | same as (b) + `until` check ≈18 per boss frame (ESTIMATED) |
| Bytes | baseline | ≈ baseline (+4 per yield store, −tables) | ESTIMATED 1.0× to 1.2× (depends on spread/aim helpers staying exact calls) |
| Bug surface | stale/garbage `code_addr` (unchecked, see finding 1); a path that forgets to draw; terminator adjacency by comment; **the HP check repeated by hand in every waiting loop** | resume set closed and drawing guaranteed (D9.6); terminator owned by the item; HP check still per loop, and `wait_frames` cannot carry it | HP check declared once (GOOL's `:trans`); window overflow becomes a compile error; **new risk: compiler correctness and cost visibility** |

> [!finding] W-1. The functional tier earns its place on choreography, not on physics
> On the boss and the shmup enemy, `.emp2` is 4× to 7× shorter than today and removes a real bug class (a condition that must hold across every wait). On the spindash state it is 4× shorter but the saving is mostly registration and pins, which the exact tier also removes, and it adds rounding risk where S3K accuracy lives. **Tier line:** state choreography (phases, waits, sequences, events, spawning) may be `.emp2`; per-frame numeric work (physics, collision, sprite emit) and everything in an IRQ or bus window stays exact.

> [!finding] W-2. `wait_frames` is a pure park, so it cannot express "wait unless"
> In (b) the boss's HP threshold cannot be checked during `wait_frames`; every interruptible wait becomes a hand loop with the check repeated (`.wait` above). This is the single most common shape in a Treasure-style boss. An exact-tier fix exists: `wait_frames #N, slot until <cc-test>` or a per-script `every_frame { … }` block inserted at each resume point. That is still byte-transparent sugar.

---

## 6. Correctness

> [!finding] 1. Every typed indirect dispatch is trusted, not checked (the totality claim has a hole at the game-logic boundary)
> - `jsr (aN) as Type` narrows the clobber closure to the type's bound, but nothing checks that the procs actually installed satisfy it. Sigil says so itself: `closure.rs:208-216` ("TRUSTED, not proven … called from exactly one place, interface `implement` binding, and never at a dispatch site").
> - **MEASURED** (`sigil build --aeon <copy> --report indirect-cost`, binary `sigil 0.1.0 (7630ba0d)`, release build of sigil, 0.6 s wall clock, Ryzen 7 9800X3D; output `<scratch>/audit-builds/reviewer-b/ic.txt`): **11 bounded dispatch sites** (RunObjects ×2 `as ObjRoutine`, Player_Main `as PlayerState`, Player_SetState ×2 `as PlayerHook`, Player_Ability, Player_SensorPair ×2, AnimateSprite, GameLoop, VBlank_Handler). **0** clobber firings with the bound trusted, **71** with it forced to ⊤.
> - Root cause on the object side is the store, not the call: `move.w #X - ObjCodeBase, code_addr(a0)` is an integer immediate (~~25~~ **26** `move` sites with `ObjCodeBase`, MEASURED by grep; *corrected in the checker round: one of the 26 is the fixture `test/fixtures/sigil_objroutine_probe.emp`, so 25 in shipping code*). The compiler never learns that `X` is an object routine. `ObjRoutine` even names two unrelated things: a `newtype ObjRoutine = u16` (`engine/system/types.emp:59`) and a contract type `type ObjRoutine = proc (a0: *Sst) preserves(a0, d7.w)` (`engine/objects/core.emp:27`).
> - The only backstop is DEBUG-only and runtime (`Debug_AssertObjLoop`, `core.emp:479-484`).
> - **Test that fails first:** a fixture object `proc Bad (a0: *Sst) clobbers(d7) { moveq #0,d7; jbra Draw_Sprite }` installed by `move.w #Bad - ObjCodeBase, code_addr(a0)`. Expected `[dispatch.target-exceeds-bound]`; today the build is green.
> - Fix: a typed store (`set_state a0, Bad` checked against `ObjRoutine`), and conformance of every member of every `offsets`/`dispatch` table that a typed `jsr … as T` indexes.
- *Added in the checker round (lens S12-b):* the one place sigil *does* check conformance, interface `implement` bindings, is skipped outright when the bound proc declares no `clobbers` (`resolve/contract.rs:488`, `:493`, `p.clobbers.as_ref()?`). MEASURED, probe `s12b_*`: a hook declared `preserves(a0) clobbers(d0)` bound to a proc that trashes `d1` and `a0` builds green (exit 0) when the proc has no clause, and fails with `[contract.hook-signature]` when it declares `clobbers(d1/a0)`. So declaring *less* passes where declaring honestly fails. All 8 aeon bindings declare `clobbers` today (grep), so it is latent. Fix: a bound proc without a clause gets its *inferred* closure (or ⊤) checked against the signature, never a skip. See the checker round below.

> [!finding] 2. `script` cannot target aeon: the `code_word` encoding was never built
> - Aeon's dispatcher is table-less: SST offset 0 holds `routine − ObjCodeBase` (`core.emp:595-599`, `sst.emp:28-32`). `script` ships only `word_offsets` and `long_ptrs`, which store a table ordinal (Spec2 §5.6).
> - Spec2 S2-D12(b) says `code_word` is "REQUIRED before the first scripted-object port on real Aeon". `grep code_word crates/sigil-frontend-emp/src` returns **0 hits** (MEASURED). The exhibit admits it compiles only against a mock prelude (`examples/game/badniks/pitcher_plant_script.emp:54-61`).
> - So the construct the research names as the right model for both named workloads (stage 1 conclusion 2) is unavailable, and the 0 uses are a blocker, not a choice.
> - **Test that fails first:** `script s (a0: *Sst) (encoding: code_word, base: ObjCodeBase) shows Draw_Sprite { yield .x … }` in a sonic4 object; expected: builds, and the stored immediate equals `.x − ObjCodeBase`. Today: unknown encoding.

> [!finding] 3. Comptime type annotations are not enforced, so the §1 silent `()` still escapes
> - MEASURED on `7630ba0d` with probe `p4.emp`: `comptime fn NEST(a,b) -> int { return if a == 1 {1} else { if b == 1 {1} else {0} } }` returns `()` for `NEST(0,1)` with **no diagnostic at the fold and none for the declared `-> int`**. A `[int; 3]` parameter accepted an array containing two `()` values. The error appears only where `()` meets `==` or `+` (`[eq.cross-type]`).
> - So the pitfall is narrowed (`EMP_PITFALLS.md` §1 amendment) but the language still lets an annotated function lie about its type. Comptime is where the engine's tables and layout proofs are computed (4,499 comptime-fn lines, stage 0 §5).
> - **Test that fails first:** the probe above with `ensure(NEST(0,1) == 1)`: expected green (or a return-type error at the fn); today `[eq.cross-type]` red at the caller.
> - Two other pitfalls re-checked: §7 (`add.w d0,a0` encoded as ADDX) is now a loud error (MEASURED, probe `p2.emp`). §14 (unknown field in `Struct.field(aN)`) is named correctly in a single-module build (MEASURED, probe `p5.emp`); I did not reproduce the multi-module path the pitfall records.

> [!finding] 4. The player state numbering is protected by a hand-written proof, because the language cannot share a table's ordinals
> - 13 `PSTATE_*` values are hand-typed byte offsets (`config/constants.emp:292-316`) that must equal 2× the row index in **three** parallel `offsets` tables (`player_common.emp:1982`, `:2059`, `:2088`). The guard is a comptime popcount over a hand-kept list (`constants.emp:325-339`) plus three `.count` ensures, explained by ~45 comment lines (`:248-291`).
> - The `offsets` construct already derives `Player_States.Spindash`, but ordinals are module-local (Spec2 S2-D12(a)), so the game's constants module cannot use them, and one construct cannot key three tables.
> - Engine leak: `engine/level/camera.emp` mirrors `PSTATE_JUMP`/`PSTATE_ROLLJUMP` by number because the engine cannot `use` the game (`constants.emp:287-291`). A game-agnostic engine reading a game's state numbering breaks the engine/game wall, and a non-Sonic game would inherit a meaningless mirror.
> - Port-caused: yes. It reproduces the AS `routine` idiom.
> - **Test that fails first:** swap the `Roll` and `Spindash` rows of `PState_ExitHooks` only. Today the build stays green (the proof covers the values and each table's count, not which row holds which handler), and Spindash's exit hook runs on leaving Roll. Expected: a compile error (in a `states` table the row IS the state, so the swap is unrepresentable).

> [!finding] 5. Register inputs are checked only where a proc declares them, and state handlers do not
> - `PState_Spindash (a0: *Sst)` reads `a4` (`player_spindash.emp:106`, `:108`, `:134`) without declaring it. `[call.input-undefined]` checks declared params at direct calls only, and indirect dispatch not at all (`calls.rs:225-229`). The bound `PlayerState = proc (a0: *Sst) … clobbers(d0-d7, a1-a4)` (`player_common.emp:51`) even lists `a4` as clobberable, while every state treats it as an input.
> - Effect: the per-slot design rests on every caller loading `a4` first ("THIS IS THE WHOLE PER-SLOT MECHANISM", `player_common.emp:462-466`). A new caller of a state that forgets `player_block` would silently use whatever `a4` holds, and nothing in the language would say so.
> - **Test that fails first:** a new `[proc.input-undeclared]` lint over `PState_*`: expected to fire on `a4` today.

> [!finding] 6. Language gaps force workarounds that the conventions forbid
> - **Width pins.** `align` at a provisional position is refused (D2.29), so `game_debug.emp:69-77` keeps 16 sized branches and `(X).w` operands against `aeon/CLAUDE.md:70` ("explicit `.s`/`.w` only in `@as_compat` ports, patched fields"). The 20 "widths pinned" notes (stage 0 §4) are the same gap. Fix: S2-D16(c) link-time gap-fill.
> - **Sentinel adjacency by comment.** Five hand terminators (`test_parent.emp:166-170`, `test_emitter.emp:55-59`, `test_stress_emitter.emp:54-58`, `object_test_state.emp:382`, `demo_data.emp:47`), three of them labelled "ADJACENCY-CRITICAL". `table` (D2.36) has a sentinel mode; the comment says the record form is deferred. I did not verify whether `table` can express a `SpawnDesc` list today.
> - **Cycle budgets.** `@budget` refuses any loop or call (`cycle_budget.rs:28-40`), so it has 1 use against 349 hand-counted cycle comments (stage 2 §4b). For a boss or a per-object budget this makes the checker unusable on real logic.

> [!note] Edges a language audit must cover, and what I found
> | Edge | Finding |
> |---|---|
> | Wraparound | Typed: `addq.b #CHILD_ORBIT_SPEED, d0 as Angle` (`test_parent.emp:111`) records the u8 wrap. Good |
> | Slot / window exhaustion | Overlay windows are size-checked always (`vars X: Sst.sst_custom`, 32 bytes). A compiled tier must keep this a hard error |
> | DEBUG vs release | `assert.<w>` self-gates; setup lines do not (Pitfall §6 Trap C). DEBUG and release differ in code size per region (`core.emp:3-6`). No language-level finding beyond the pitfall |
> | Budget exceeded | Logic over budget is invisible at build time (finding 6, `@budget`); it shows up only as lag frames |

---

## 7. Code quality

### 7.1 Doc claims against the code (stage 0 §2 list)

| Claim | Holds? | Evidence |
|---|---|---|
| C1 Spec2 frozen, §10 inventory closed | **No** | 24 shipped constructs absent from every `SIGIL_*.md` (stage 0 §1e), including the whole contract layer the verdict rests on |
| C3 "The instructions don't change" | **No, and rightly** | `wait_frames`, `yield`, `assert.<w>`, `mul_const`, relaxation all choose instructions. Tenet 3 already reads "instructions are visible and deterministic", not "hand-written" |
| C4 zero-import prelude | No | aeon does not pass `--prelude`; 552 `use` lines |
| C12 sized branches only in `@as_compat` | No | 46 sized branches, 0 `@as_compat`; the `game_debug.emp` set is a language-gap workaround (finding 6) |
| C13 `struct` for ALL structures, no manual const chains | **No** | 24 `const PBLK_*` in 8 files where `PlayerBlock` exists; 163 `CONST(aN)` displacement operands in total (MEASURED, my script) |
| C17 grammar hand-maintained against the parser | No | 38 parser words missing (stage 0 §6) |
| Header comment `player_spindash.emp:8` "Gate SIGIL_EMP_PLAYER_SPINDASH (a plain ifndef in the residual root)" | **Stale** | The name appears nowhere else in `engine/`, `games/`, `tools/` or `build.sh` (MEASURED by grep) |
| `examples/game/prelude.emp` "the game prelude is REAL" (D2.22f) | Partly | Real in sigil's examples; aeon has its own different `Sst` and no prelude, so the script/dispatch exhibits are proven only against a mock |

### 7.2 Idiom and port-residue check (stage 0 counts, reviewer reading)

| Check | Count (stage 0, MEASURED) | My reading |
|---|---|---|
| Build-time work at run time | 13 static-table loop candidates | Mostly init copies; not a game-logic issue |
| Layout literals | 50 raw `n(aN)`, 0 on typed registers | **Understated.** The same idiom under a name: 163 `CONST(aN)` operands, e.g. `PBLK_JUMPBUF(a4)` (finding 4/5 area, Challenge 4) |
| Hand-spelled pins | 352 name==literal `ensure`s | The PBLK pins exist only because `a4` is untyped: 24 `const PBLK_*` in 8 files (MEASURED by grep; the comment at `player_spindash.emp:48` counts 21). The PSTATE proof exists only because ordinals do not cross modules |
| Hand-kept protocols | not counted by script | In game logic: the PSTATE four-edit rule; "hooks preserve a0/d0/d7" (`player_common.emp:2040`, `:2112`, prose plus a trusted bound); sentinel adjacency (5); "every object routine MUST return with a0 and d7.w" (`core.emp:479-484`, trusted bound plus a DEBUG assert) |
| Assembler residue | 367 AS echoes, 198 `.asm` refs | Not in the game-logic files I read except `player_common.emp` (21 `.asm` refs) |
| History in comments | 1,033 dated lines; 60% comment share | The spindash body is 30 comment lines to 63 instructions; most explain S3K semantics (worth keeping), a third restate contracts |

**Verdict on shape: native guards on ported structure.** The guards are real language use (typed overlays, `out(carry:)`, contexts, `as Type`). The **state-machine representation is ported**: integer `code_addr` immediates, hand-numbered byte-offset states, parallel tables, sentinel terminators, displacement constants. Findings 1, 4 and 5 are caused by that structure. That is evidence for fixing the *language* surface for state machines before rewriting any object code.

### 7.3 Does the language carry its weight?

- **The contract layer does.** 596 of 605 procs carry `clobbers`; the closure and `preserves` verification are used everywhere and catch real bugs (stage 0 §3).
- **Typed operands do, and stage 2 under-counts them.** 1,437 instruction lines use a symbolic struct or overlay field operand (604 bare `field(aN)`, 833 `Struct.field(aN)`), plus 979 `jbra`/`jbsr` and 45 `as Type` annotations (MEASURED, my script over the 190 hand files). "2.2% higher constructs" counts statement lines only (Challenge 3).
- **The game-logic constructs do not**, because one is blocked (finding 2) and the others were built against a mock prelude rather than against aeon's object model.
- **Could another game's developer follow it?** For engine code, yes with the contracts as a guide. For a boss: no. They would copy `TestParent_Main`'s style, with untyped state stores and hand timers, and get no help from the compiler on the four bug classes in W2.

---

## 8. Verdict and plan

### 8.1 Verdict

| Layer | Rating (tool scale) | Why |
|---|---|---|
| Instruction tier + contracts | **Best known; the contract checker is beyond** | No assembler checked by stage 1 has transitive register-effect contracts or cycle budgets over a real timing table |
| Comptime | **On par to best known** | Zig-class; weakened by unchecked return types (finding 3) |
| Game-logic authoring | **Below the best common tools** | SGDK's C, GOOL-style coroutines and S3K's own continuation idiom all express a boss more directly. The `.emp` answer exists on paper but not on aeon |
| **Overall** | **Best known as an assembler; below as a game language** | |

**Disposition: patch** (the exact tier, now). **`.emp2`: rewrite beside, gated** (below). Reason: the measured waste sits in state-machine *surface* (registration, stores, pins, prose), and every item that removes it is byte-neutral or byte-transparent. A compiled tier is affordable for choreography (stage 1: +4.4% for C, stage 2: 1.0× to 1.1× with register contracts, both ESTIMATED), but only after the exact tier can express typed states, or the new tier would have nothing typed to lower into.

### 8.2 Ranked improvements

| # | Improvement | Gain | Cost | Risk | Depends on | Bytes |
|---|---|---|---|---|---|---|
| 1 | **`code_word` encoding for `script`/`dispatch`** (S2-D12b) + typed state store `set_state aN, Proc` checked against `ObjRoutine` | Unblocks coroutines on aeon; closes stale/garbage state pointers for every converted object | Small–medium (sigil): one encoding row + one statement | Low | — | Neutral for `set_state`; a script adds one store per self-yield (+4 B, +12 cycles) |
| 2 | **Check `as Type` conformance** (S12): the targets set = typed stores + table members; run `subcontract_violations` | Makes the contract claim true at the 11 dispatch sites | Medium: 71 contracts to correct (MEASURED) | Medium (aeon+sigil parcel) | 1 (typed stores give the target set) | Neutral |
| 3 | **`states` multi-column table + cross-module ordinals** (S2-D12a) | Deletes PSTATE_* numbering, the popcount proof, ~45 comment lines; lets camera.emp ask the game through the contract, not a mirror | Small | Low | — | Neutral |
| 4 | **Comptime soundness**: enforce declared return and element types; make a value-position `if` without a value-yielding `else` an error | Kills pitfall §1 at the source | Small | Low (may surface latent folds: good) | — | Neutral |
| 5 | **Typed inputs**: params on state handlers; `[proc.input-undeclared]` for read-before-write registers | Finding 5; deletes 24 PBLK consts and their pins | Small | Low | — | Neutral |
| 6 | **Compositional `@budget`**: callee budgets as costs, loop trip counts from `dbf` counters bounded by refinement types | Turns 349 hand cycle comments into checks; per-object frame budgets for boss logic | Medium | Low | — | Neutral |
| 7 | **Interruptible waits**: `wait_frames … until <test>` / `every_frame { }` in scripts (W-2) | The Treasure boss shape without hand loops | Small | Low | 1 | Transparent |
| 8 | Link-time `align` gap-fill (S2-D16c) | Retires width pins (finding 6) | Small | Low | — | Neutral |
| 9 | Generate In/Out/Clobbers text from headers (hover or `sigil doc`) and strip the 931 prose copies, dated history and AS echoes | Comment share down sharply; no drift | Small | None | — | Neutral |
| 10 | **`.emp2` prototype** (gate below) | 4× to 7× shorter choreography (W2) | Large | Medium–high | 1–3, 6 | New code only |

**`.emp2` gate.** Build it only if a prototype front-end, lowering into the same `CodeBuf` so the closure and `@budget` check its output unchanged, compiles (i) `palette_dsl`'s `variant_word` model into the `Palette_DeriveVariant` loop (stage 2 §5.3) and (ii) one boss phase from W2, at **≤ 1.15× cycles and ≤ 1.25× bytes** (MEASURED by `@budget` after item 6), under four hard rules:
1. **Register contracts are the calling convention.** No stack ABI, no callee-save `movem` it did not need.
2. **In-place slot mutation is guaranteed.** `x := x - 1` on a slot field is `subq d16(An)`; no copies, no heap.
3. **States lower to `code_word`** (item 1). No interpreter, no hidden tables.
4. **Fenced by context.** Every `.emp2` proc carries an implicit `requires(main_loop)`, so the existing context checker keeps it out of IRQ and Z80-bus windows.

### 8.3 Beyond the state of the art

| Candidate | Plausibility |
|---|---|
| Checked coroutine objects: typed closed resume sets, conformance-checked dispatch and a per-object compositional cycle budget | **High.** Items 1, 2, 6. No Genesis toolchain, shipped or homebrew, has this (stage 1 §3) |
| A functional choreography tier whose output is checked by the same register-contract and cycle machinery as hand asm | **Medium.** Novel combination; the risk is compiler effort, not feasibility |
| Contracts verified across every indirect dispatch in a whole game | **High** after item 2 |

### 8.4 Owner question: functional everywhere?

**No.** Per area, from stage 1 §4.2-4.3 and stage 2 §5:

| Engine area | Share of code lines (stage 0 §5) | What a compiler cannot match | Cost if compiled | What the escape hatch would hold |
|---|---|---|---|---|
| HBlank/VBlank | part of 14.0% cycle-critical | per-line ≈488 cycles; register pinning; port in `An` | 2.6× to 5.1× (stage 1 W8: GCC 15.2 codegen MEASURED, cycles from sigil's table) | all of it |
| Z80 driver | 3,758 lines (81% of cycle-critical) | YM write spacing (`ensure(cycles(...))`), DAC timing | no compiler models T-states | all of it |
| DMA / VDP copy | cycle-critical | FIFO-paced unrolls, `dbf` | 1.47× to 2.30× (stage 1 W6, same method) | all of it |
| Decompression, streaming caches | engine systems 24.1% | `(An)+` streams, bit tricks | ≈1.1× to 1.5× (ESTIMATED) on code that already lags | most of it |
| Object core, sprite emit, collision | 6.9% + tight loops 1.9% | `dbeq` fusion, pad-byte tricks | ~~1.07× to 1.25× (stage 2 §5.2, ESTIMATED)~~ **1.73× (1.77× for asm-style C) per sprite piece, GCC 13.3 codegen MEASURED, cycles ESTIMATED by hand count (stage 2 §5.2)**. The 1.07× to 1.25× is stage 2's estimate for a hypothetical tier nobody has written. *Corrected in the checker round* | the hot loops |
| Game choreography | part of 14.1% | — | 1.0× to 1.15× with the four rules (ESTIMATED) | nothing |

The hatch would hold roughly 40% of all code lines (14.0 + 1.9 + most of 24.1, ESTIMATED) and every line where cycles bind. A language whose escape hatch is the majority of the engine is the exact tier with a different default.

### 8.5 Dropped ideas

| Idea | Why dropped |
|---|---|
| C via GCC/SGDK, or an LLVM backend | Measured 1.3× to 5.1× (stages 1 and 2); clang emits `jsr __mulsi3` for a 16×16 multiply; Rust m68k is tier 3 |
| A bytecode VM for object logic (Batman & Robin style) | ≈26 cycles per op of pure overhead (stage 1, ESTIMATED); the coroutine lowering gets the same model for free |
| General `if`/`while` structured flow in `.emp` | Lower value than items 1 to 7: the waste is in state registration and stores, not in branch spelling. S2-D15's byte-exact-AS argument is obsolete, though, so this can be reopened on demand |
| `.emp2` for the player physics | W1: little saving, real rounding risk where S3K accuracy lives |

---

## Coverage

| File / crate in scope | Lines read | Result |
|---|---|---|
| `emp-language/0-inventory.md`, `1-research.md`, `2-measurements.md` | all (per-file table skimmed) | used; Challenges 1-5 |
| `aeon/docs/audits/AUDIT_TEMPLATE.md`, `Audit board.md`, `Audit process review.md` | all | template followed |
| `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` (1,310) | 124-330, 827-1060, 1113-1137, 1283-1310 (ledger, long rows truncated at 1,800 chars) | findings 2, 4; §7.1 C1/C3 |
| `empyrean/docs/SIGIL_IDEAS_FUNCTIONAL_TIER.md` (147) | all | W1/W2 (c), §8.2 gate |
| `SIGIL_AEON_COMPAT_NOTES.md` (62) | all | read, nothing (AS-port notes) |
| `SIGIL_CORE_SPEC.md`, `SIGIL_M0_CATALOG.md`, `SIGIL_M0_DESIGN.md`, `SIGIL_ORACLE_ISA_SHARING.md`, `SIGIL_PLAN_AUDIT_2026-07-01.md` | headings only | not language surface; left to the assembler audit |
| `aeon/docs/EMP_PITFALLS.md` (555) | 1-75, 148-210, 416-460, 516-555 | finding 3 (re-probed §1, §7, §14) |
| `sigil/docs/EMP_PITFALLS_EQUALITY.md` (220) | headings | read, nothing beyond aeon's §12/§13 |
| `aeon/CLAUDE.md` (191) | grep for conventions (lines 5-75, 132-136) | §7.1 C12, C13 |
| `sigil/editors/vscode/` (274) | README all; grammar via stage 0 | §7.1 C17 |
| `sigil-frontend-emp` (113,871 lines incl. tests) | `closure.rs` 195-230, 400-470; `corpus_contracts.rs` 1-80, 880-910; `cycle_budget.rs` 1-60; `calls.rs` 225-229; greps for `code_word`, `subcontract_violations`, `ObjRoutine` | findings 1, 2, 5, 6. The rest of the crate **not read**; I relied on stage 0 for feature status |
| `sigil-cli/src/main.rs` | 150-200 (build usage) | used to run the report |
| `sigil/examples/game/badniks/pitcher_plant_script.emp` | 1-260 (all) | finding 2, W2 (b) |
| `games/sonic4/player/player_spindash.emp` (195) | all | W1; findings 4, 5; §7.1 stale gate |
| `games/sonic4/player/player_common.emp` (2,625) | 425-530, 1185-1210, 1970-2130 | findings 4, 5 |
| `games/sonic4/config/constants.emp` (535) | 240-345 | finding 4 |
| `games/sonic4/objects/test_parent.emp` (214), `test_enemy.emp` (65) | all | W2 (a); finding 6 |
| `engine/objects/core.emp` (803) | 1-40, 440-560 | finding 1 |
| `engine/objects/sst.emp` (190) | 1-60, 82 | §1 window size; finding 1 |
| `engine/system/types.emp` | 50-65 | finding 1 (`ObjRoutine` twice) |
| `games/sonic4/debug/game_debug.emp` | 53-86 | finding 6 |
| `games/sonic4/map.toml` | grep `object_bank` | read, nothing |
| `s4.lst` (release) | 2048-2049, 2101-2105, 2207-2218 | W1 bytes |
| Remaining ~175 hand `.emp` files and the 14 `generated/` files | **not read line by line**; covered only by stage 0's counts and my two scripts (symbolic-operand count, `CONST(aN)` count, `ObjCodeBase` stores) | no finding claimed from them |

Probes and outputs (all in `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/audit-builds/reviewer-b/`): `ic.txt` (indirect-cost report, run on an rsync copy of aeon at `aeon/` there, no files written into it, checked with `find -newer`); `probes/p1-p5.emp`. Binary: `audit-builds/sigil-inv/release/sigil`, `sigil 0.1.0 (7630ba0d)`. No repo was written except this file.

> [!warning] Challenges to stages 0 to 2
> 1. **`script` and `dispatch` "0 uses" is a blocker, not an adoption gap** (stage 1 conclusion 2, stage 2 §3 "nothing in the corpus shows whether they work at scale"). `script` cannot target aeon's table-less dispatcher: `code_word` is unbuilt (0 hits in `src/`), Spec2 S2-D12(b) calls it required, and the exhibit compiles only against a mock prelude (`pitcher_plant_script.emp:54-61`). Also, stage 1's "lowers to a resume table at zero cost" is wrong for aeon twice over: aeon has no table, and a self-yield stores its word every frame (+12 cycles, ESTIMATED).
> 2. **Stage 1 conclusion 4 quotes "97% of vertical blank" without its build.** That is the DEBUG figure; release is 82% (its own §4.3 table and the process review, row 16). The same slip the process review flagged.
> 3. **Stage 2's "2.2% higher constructs" under-counts the language in use.** It classes whole lines, so typed operands inside instruction lines count as raw asm. 1,437 instruction lines carry a struct or overlay field operand, and there are 979 `jbra`/`jbsr` and 45 `as Type` (MEASURED, my regex over the 190 hand files). "An assembler with a contract checker bolted on" is too strong; "an assembler with typed operands and contracts, and no state-machine surface" is what the code shows.
> 4. **Stage 0's layout-literal row (50 raw, 0 on typed registers) misses named displacement constants.** 163 `CONST(aN)` operands (MEASURED), including the 24 `const PBLK_*` that duplicate `PlayerBlock` and need `offsetof` pins. They are layout literals with names.
> 5. **Stage 1 says "with one exception" (Spinball), but its own table lists Batman & Robin's object model as interpreted.** Minor, but the conclusion should say two.

---

## Checker round

> [!verdict] What changed after the checker's three gaps
> - **The disposition holds: patch the exact tier now; `.emp2` is a gated rewrite beside, for choreography only; functional everywhere: no.** The sprite-loop correction (1.73×, not 1.07× to 1.25×) strengthens "the hot loops stay exact".
> - **One rating drops.** §8.1 rated comptime "on par to best known". After reading the two DSLs and re-probing, it is **best-in-class in reach, below best known in soundness**. Free names in a `comptime fn` body resolve at the *caller* (dynamic scoping). Uncalled bodies are never checked. Declared payload and return types are not enforced. Zig, the comparison §8.1 used, is lexically scoped and enforces its types. (Zig is also lazy about uncalled functions, so that one is shared.)
> - **The new prediction for `.emp2`:** the existing pure tier works only because of 50 literal pins, elaboration anchors, a caller-import discipline and 29 poison fixtures. A second tier built on the same evaluator would inherit all of it. **So the `.emp2` gate gains a precondition: lexical scoping, every body type-checked whether called or not, enforced types.** That is improvement 4, widened and moved up to rank 2.
> - **Finding 1 widens (lens S12-b).** The one conformance check sigil does run, on interface bindings, is skipped when the bound proc declares no `clobbers`. MEASURED: a hook bound to a proc that breaks its `preserves(a0)` builds green. It is latent in aeon today.

### CR-1. The existing functional tier: `raster_dsl.emp` and `scene_dsl.emp`

I read both headers in full (raster 1-140, scene 1-120), plus `compose` (raster 523-597), the accessor block (768-812), the elaboration anchors (1620-1648, 3690-3720), the trailer note (3405-3420) and scene 418-432 and 1227-1260. I also read the client `ojz_scenes.emp` 205-290 and `scene_registry.emp` 44-58, and the harness helper list (`sigil-harness/src/native.rs:1800-1824`). The remaining roughly 6,500 DSL lines were covered only by the greps below.

**What it is (MEASURED, my awk/grep, comment = a line starting with `//`):**

| | `raster_dsl.emp` | `scene_dsl.emp` |
|---|---|---|
| Lines: code / comment / blank | 1,274 / 2,455 (64%) / 132 | 1,205 / 2,386 (65%) / 73 |
| `comptime fn` (pub) | 97 (24) | 78 (76) |
| Lines containing `ensure(` (grep, includes comment mentions) | 162 | 84 |
| Inlined-literal pins (`ensure(NAME == literal)`, stage 0) | 19 | 31 |
| `comptime enum` / record structs | 3 / 0 | 8 / 0 (6 `struct` mentions, ROM structs) |
| Lambdas, `\|>`, `map`/`filter`/`fold` | **0** | **0** (2 grep hits are the name `derived_remap`) |
| Elaboration-anchor fns (`zz_*`) | 2 | 0 |

Corpus-wide there are 6 `|> fold(…)` sites and 1 `map(`, all asm-unroll generators in `parallax.emp`, `dma_queue.emp` and `core.emp`. The functional glue in Spec2 D2.12 (pipe, lambdas, `map`/`filter`/`fold`) is not used by the language's two largest pure programs. **They are written as imperative code over values:** `comptime var` accumulators, nested `for`, `out = out ++ [x]`, and integer 0/1 flags in place of booleans (`compose`, raster 523-597).

**What works:**
- **It does the job.** It is the build-time half of the raster and parallax systems, and it lowers authored scenes byte-identically to the configs it replaced (`scene_dsl.emp` header, `scene_equiv_proof.emp`). Its checks are real domain checks: per-fire HBlank cycle costs and solved spin counts (raster 1039-1222), interval and density checks, band pairing and ownership. Exhaustive `match` over enums caught a real op_mask trap (raster note above `RasterOp`). The whole sonic4 build, these DSLs included, takes about 0.6 s wall clock (release sigil, Ryzen 7 9800X3D, from the `ic.txt` run), so evaluation cost is not a problem.
- **Forced designs that turned out better.** Pitfall §5 (`extern()` poisons comptime) forced the raster trailer to carry parameters rather than built `DMAEntry`s (raster 3405-3420). The file argues that this is the better shape anyway: one runtime builder instead of two.

**What fails (each measured on `sigil 0.1.0 (7630ba0d)`; probes in `<scratch>/audit-builds/reviewer-b/probes/`):**

| Defect | Pitfall | Probe | Result | Cost in the DSLs |
|---|---|---|---|---|
| A body's free names resolve at the **caller** | §2 | `q1`: helper `use`s `LO`/`HI` and loops `LO..HI`; called from a module that does not import them | **Now loud**: `unknown name LO` and the caller's ensure fails (exit 1). The "silently empty range" in the pitfall text is narrowed | 50 literal pins. Every opcode, bound and bit is spelled as a number in the bodies (raster 14-22, scene 51-75) |
| the same, when the caller happens to have the name | §2 | `q1b`: caller defines its own `LO = 0`, `HI = 10` | **Silent capture**: the helper returns 10, not 3. Only my ensure exposed it (exit 1 from the ensure, no name diagnostic) | A helper's meaning depends on the caller's namespace. scene_dsl requires callers to glob-import it (scene 61-64) |
| Uncalled bodies are never checked | (scene header, raster 1624) | `q2`: uncalled fn containing `ensure(1 == 0)` and `unknown_name + MISSING_CONST` | **builds green, exit 0** | Hand anchors (`zz_elaboration_anchor`, raster 1634; `band()`'s hand-word twin, raster 3698) exist only to force elaboration |
| Enum payload types are documentation | §13-adjacent; finding 3 | `q3`: `enum Op { Cram(int, array) }`, `Op.Cram(5, 7)` | **builds green, exit 0** | Every guard is a hand `ensure` in a constructor (raster note above `RasterOp`). 7 match-accessor fns stand in for missing record fields (raster 768-812) |
| A struct declaration is re-elaborated in each importer | §8 | not re-probed | read: `scene_registry.emp:49-58` and generated `effects_scenes.emp:33` carry the same 10-name import line "ALL … LOAD-BEARING" for names the file never spells | Every scene client pays an import tax set by the engine's struct internals |
| Nested value-`if` folds to `()` | §1 | finding 3's `p4` | still silent at the fold | scene_dsl cites the workaround (flat accumulators, single-level `if`) 14 times |
| Label compared to int is defined false | §3, EQUALITY §12 | not re-probed | read: scene 422-425 | enum *variants* are used as discriminators, never a `Label = 0` default (forced, documented) |

Two more things I saw:
- **Fixed-size arrays without defaults.** `ojz_scenes.emp` pads every scene to 16 layers with `no_layer()` (116 lines in that one file). That is the `[T; N]` contract (§13) with no fill or default syntax.
- **A stale header.** The scene_dsl header (7-16) still says the file has "ZERO body-elaboration coverage". It now has callers (`scene_registry.emp`, generated `effects_scenes.emp`) and 29 poison fixtures name scene, raster, band or budget guards (`games/sonic4/test/poison/`, 48 files, filename grep). The header is wrong, and the danger it describes (an uncalled `pub` fn ships unchecked, `q2`) still exists for any helper without a caller.
- **Language semantics live in the harness.** Which modules are glob-injected everywhere (and so which names are "safe" in a body) is a hard-coded Rust list, `COMPTIME_HELPERS` in `sigil-harness/src/native.rs:1802`. Order matters there, a later duplicate wins silently, and an external Python gate guards it. That is a scoping rule outside the language.

**Prediction for `.emp2` (reasoned from the above):**
1. **The authors will write imperative code over values, not combinators.** With lambdas and `fold` available since D2.12 (2026-07-05) and the DSLs written in August, the 2,479 code lines use none. A choreography tier should therefore be **statement-shaped**: `script`/`yield`/`wait` blocks over slot fields, like W2(b). A Haskell-like expression language is the wrong target. That matches the line I drew in W ("what happens next", not "how numbers are computed").
2. **The evaluator's semantics are the risk, not codegen.** Every DSL defect above is in name resolution, checking coverage or types. None is a performance problem. A `.emp2` front end that reuses this evaluator would make game logic depend on the caller's imports and leave unreached states unchecked. For a boss with 12 phases, an unreached phase is the normal case during development.
3. **What this asks for, before any `.emp2` work** (amends the §8.2 gate): (a) lexical scoping of `comptime fn` bodies at the definition site, which retires the 50 pins, the scene-import tax and `COMPTIME_HELPERS`; (b) every body type-checked whether or not anything calls it, with only *evaluation* left lazy, which retires the anchors; (c) enforced parameter, return and payload types, plus records with named fields and defaults, which retires the accessor boilerplate and the `no_layer()` padding. These are front-end changes that pay off in the exact tier at once, so they are worth doing whether or not `.emp2` ever ships.

### CR-2. §8.4 GCC figure: corrected inline

The object-core row now reads **1.73× (1.77× for asm-style C)**: stage 2's GCC 13.3 `-O2` codegen (MEASURED), with cycles ESTIMATED by hand count from the M68000UM tables (`2-measurements.md:254`, `:29`). The 1.07× to 1.25× I had quoted is stage 2's ESTIMATE for a hypothetical tier (`:16`, `:309`). I also labelled the HBlank and DMA rows with their source.

> [!note] The two stages' ratios are not comparable
> Stage 1 used **GCC 15.2.0** (Compiler Explorer, `-O2 -mcpu=68000 -fomit-frame-pointer`), with cycles from **sigil's `@budget` table** on worst paths, on 8 synthetic Genesis-shaped routines (`1-research.md:44`, `:156`). Stage 2 used **GCC 13.3** (Ubuntu cross package, `-m68000 -O2`), with cycles **hand-counted** and no wait states, on 3 real aeon routines (`2-measurements.md:26-29`). The compiler versions, the cycle source and the workloads all differ. Read each ratio only within its own stage. The 1.07× in stage 1 W2 (TF4 bullet) and the 1.07× in stage 2's hypothetical tier are two unrelated numbers that happen to coincide.

The verdict does not change. The hatch estimate (about 40% of lines, ESTIMATED) never depended on this row, and a measured 1.73× on the sprite loop is a stronger reason to keep it exact than the estimate was.

### CR-3. Lens S12-b, folded into finding 1

`find_proc_contract` (`resolve/contract.rs:470-501`) returns `None` when the bound `proc` (`:488`) or `extern proc` (`:493`) has no `clobbers` clause. `check_proc_type` (`:383`) and `check_hook_sig` (`:398`) then return without checking anything. That means `preserves`, `out`, flag outs and params are skipped too, not just clobbers.

| Probe (`--root`, program mode) | Bound proc | Result |
|---|---|---|
| `s12b_noclob` | `proc Bad ()` with `moveq #0,d1; movea.l d1,a0; rts`, bound to `hook h () preserves(a0) clobbers(d0)` | **built: 14 bytes, exit 0** |
| `s12b_withclob` | same body, `clobbers(d1/a0)` | `[contract.hook-signature] … clobbers a0 … clobbers d1 … does not preserve a0`, exit 1 |

The caller in both probes, `proc Caller () clobbers(d0) { invoke G.h }`, is accepted in the no-clause case because `closure.rs:208-216` trusts the hook's declared bound. So a proc that declares *less* passes, while one that declares honestly fails, and the trusted narrowing ends up with no check at all. **Exposure today: 0.** All 8 bindings in the two aeon `implement Game` blocks name procs that declare `clobbers` (grep: `GameState_OJZScroll_Init`, `RingSparkle_Spawn`, `RingArt_StreamFrame`, `Spring_Launched`, `Solid_Pushed`, `SoundTest_BootPing`, `Debug_MusicToggle`, `GameState_Demo_Init`). It becomes live the first time a game author binds a hook to an unannotated proc, which is exactly what a new non-Sonic game would do.

**Finding 1's fix, amended:** (a) a bound proc without a clause is checked using its *inferred* closure, or ⊤ if none can be inferred, never skipped; (b) the same conformance runs over typed stores and `offsets`/`dispatch` members (the original fix); (c) the test that fails first is `s12b_noclob`, which must go red. Improvement 2 in §8.2 now covers both halves. Cost and risk are unchanged: Medium.

### CR-4. §8.2 ranking after this round

| # | Change |
|---|---|
| 4 → **2** | **Comptime soundness, widened**: lexical body scoping, type-check unreached bodies, enforced param/return/payload types, records with named fields and defaults (CR-1 item 3). Medium cost (it was Small) |
| 2 → 3 | `as Type` plus binding conformance (S12 + S12-b, CR-3) |
| 10 | The `.emp2` gate adds the precondition: new rank 2 done. The prototype must be statement-shaped choreography (CR-1 prediction 1) |

The other rows stay where they were.

### Checker-round coverage

| Item | Lines read | Result |
|---|---|---|
| `5-checker.md` | all | 3 gaps addressed; row 22 (25→26) corrected inline |
| `engine/effects/raster_dsl.emp` (3,861) | 1-140, 172-460 (fn heads), 523-600, 768-812, 1620-1650, 3405-3420, 3690-3730; whole file by grep | CR-1 |
| `engine/level/scene_dsl.emp` (3,664) | 1-120, 418-432, 1225-1260; whole file by grep | CR-1 |
| `games/sonic4/data/effects/ojz_scenes.emp`, `scene_registry.emp` | 205-290; 44-58 | CR-1 |
| `sigil-harness/src/native.rs` | 1800-1830 | CR-1 (`COMPTIME_HELPERS`) |
| `sigil-frontend-emp/src/resolve/contract.rs` (522) | 360-522 | CR-3 |
| `sigil-frontend-emp/src/eval/call.rs` | grep `Lambda` | lambdas exist (CR-1) |
| `engine/system/game_contract.emp`, both `game.emp` | 16-60 (headers); implement blocks | CR-3 exposure |
| `aeon/docs/EMP_PITFALLS.md` §2, §5, §8 | in full | CR-1 table |
| `1-research.md`, `2-measurements.md` | grep for the GCC figures (`:44`, `:156`, `:16`, `:26-29`, `:254`, `:309`) | CR-2 |
| Z80 driver (checker hole, not assigned to me) | **not read** | My §8.4 "no" for the Z80 still rests on stage 1/2 and the `ensure(cycles(...))` design, not on a reading |

New probes: `q1`, `q1b`, `q2`, `q3`, `s12b_noclob`, `s12b_withclob` under `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/audit-builds/reviewer-b/probes/`, each built with `audit-builds/sigil-inv/release/sigil` (`sigil 0.1.0 (7630ba0d)`), with output only to the probe directories. No repo was written except this file.
