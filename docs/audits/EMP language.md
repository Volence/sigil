---
kind: audit
status: awaiting-owner
wave: 1
verdict: "best known as an assembler; S3K level as a game-logic language"
description: "Keep and patch. Best known as an exact-instruction assembler (its contract checker is beyond any other), but only S3K level for game logic, and 12 proven holes let a wrong ROM through a green build. Direction: exact instructions + checks + fixed-expansion sugar now; a compiled tier only for choreography, later, behind a measured gate; functional everywhere: no."
---
# EMP language

> [!question] Owner question (added mid-run)
> Should the functional language cover **everything**, the engine and hot paths included, not just game logic? See "Owner direction" on [[Audit board]]. The lead must answer it here.
>
> **Answer: no.** Both reviewers say no independently, and I agree. The per-area evidence is in [[#Functional everywhere, area by area]]. Short form: the code a compiler cannot match (interrupt handlers, the Z80 driver, DMA and VDP loops, decompression, streaming, sprite emit) is 40% to 47% of all code lines (ESTIMATED from the inventory's shares). The escape hatch would hold the engine, and the benefit would land on the 10% to 14% that is game logic.

## Stages (live)

![[Audit stages.base#EMP language]]

Then: 6. Lead (Fable) merges into this page; 7. Controller verifies and reports to you.

## Verdict

> [!verdict] Best known as an assembler; S3K level as a game-logic language. Disposition: keep, and patch
> - **As an exact-instruction assembler it is the best known, and one layer is beyond.** No assembler the research checked (AS, vasm, ca65, RGBDS, KickAssembler, 64tass, SNASM68K) has checked register contracts or cycle budgets. `.emp` has both, and 596 of 605 procedures use them (MEASURED).
> - **As a language for game logic it is at S3K level, which is below the best common tools.** A boss today is written the way S3K writes one: a code pointer stored by hand as the state, hand timers, rules in comments. The two constructs built to beat that (`script`, `dispatch`) cannot be used on aeon at all, because the encoding aeon needs was never built.
> - **Its "no" is not yet trustworthy.** Twelve holes were proven with the shipping assembler (small test files, planted defects, one build report): each builds green and produces wrong bytes or skips a promised check. A thirteenth is proven by reading the code. I re-ran two of them myself and got the same bytes.
> - **The language does not need a restart.** Every gap is an extension of an analysis that already exists. Nothing here calls for *rewrite beside*.
>
> **Direction: option (b), exact instructions + checks + sugar with one fixed expansion, starting with soundness.** A compiled tier (option c) is a later, gated prototype for *choreography only* (boss phases, enemy scripts, events), never for physics or the engine. Functional everywhere (option d): no.

Trees: aeon `936439fa`, sigil `7630ba0d` (both last committed 2026-09-28). All probes used the shipping binary `sigil 0.1.0 (7630ba0d)`, release profile.

### Why this direction

| Option | Answer | Evidence |
|---|---|---|
| (a) Polish the exact design only | No | Leaves rules in comments (241 MUST/NEVER lines), budgets on 1 procedure of 605, and game logic as bare assembly (all MEASURED, stage 2) |
| **(b) Exact + checks + fixed-expansion sugar** | **Yes, now** | Every one of the 12 holes is cured inside it. On reviewer B's paper port it removes about 40% of the source lines of the boss, the enemy script and the spindash state at zero cycles and zero bytes (line counts ESTIMATED from drafts). The Z80 driver already works this way: `pad_to_cycles` picks the padding and an `ensure` re-measures it |
| (c) Compiled tier for game logic | **Later, gated, choreography only** | Affordable in cycles: a C-class compiler costs +5.6k cycles, 4.4% of an NTSC frame, on the named boss + shmup load (ESTIMATED from GCC 15.2 `-O2` codegen priced with sigil's cycle table). But there is no boss or enemy in the corpus to design it against, and the evaluator it would be built on is unsound today (findings 2 and 9) |
| (d) Functional everywhere | **No** | GCC costs 2.6 to 5.1 times on an HBlank handler and 1.47 to 2.30 times on a VDP copy loop (stage 1, same method). The worst release VBlank already uses 82% of its window (97% in DEBUG; MEASURED in the frame audit). No compiler can express "these three paths cost exactly 195 T-states", which is what the Z80 sample loop depends on |

**What the compiled tier would cost if you chose it today.** A second language: a new front end with a register allocator, a source map for Oracle, and the loss of "what I wrote is what runs" in the files newcomers open first. Nobody priced it in days; both reviewers class it Large, risk medium to high. Its run-time price is only known for a compiler that already exists: GCC costs 2.3 to 2.8 times on dispatch and script steps (stage 1, GCC 15.2, cycles from sigil's table) and 1.46 to 1.83 times on aeon's two test objects (stage 2, GCC 13.3, DEBUG ROM routines, cycles ESTIMATED by hand). The "1.0 to 1.25 times" figure in stage 2 is an estimate about a compiler nobody has written.

**Where the line between tiers belongs** (reviewer B, and I adopt it): not "engine against game". It is **"what happens next"** (phases, waits, sequences, spawning) against **"how a frame's numbers are computed"** (physics, collision, sprite emit, anything in an interrupt or a Z80-bus window). The first may one day be compiled. The second stays exact.

### Terms used on this page

| Term | Meaning |
|---|---|
| Exact tier | Today's `.emp`: every 68000 or Z80 instruction in the ROM is one the author wrote |
| Contract | The line on a procedure that says which registers it destroys, keeps and returns. The build checks it through every call |
| Comptime | Code that runs inside the assembler at build time to produce tables and checks. It is already a small functional language |
| `@budget` | A checked upper limit on a procedure's cycle cost |
| `script` | A coroutine object: straight-line code with `yield` and `wait_frames`, lowered to the state dispatch S3K writes by hand |
| `code_word` | The state encoding aeon's object dispatcher uses: the object's first word *is* the routine address. `script` and `dispatch` do not support it |
| Choreography | State and timing logic: phases, waits, attack patterns, spawning |
| Probe | A few-line test file built with the shipping assembler to prove a behaviour |

## What it does best

- **Register contracts, checked through every call.** `clobbers` on 596 of 605 procedures, `preserves` on 176, typed pointer parameters on 248 (MEASURED, stage 0). No other assembler has this layer. It is also exactly what a compiled tier would need to meet hand code safely.
- **Zero run-time cost by construction.** The language sits at 100% of the 68000's ceiling; any shortfall is the author's.
- **Timing proven where it is used.** The Z80 sample loop is correct only if every path costs exactly 195 T-states. The build proves it (`aeon/engine/sound/z80_sound_driver.emp:613`, `:621`, `:623`), and proves the YM2612 write spacing at every pair. The one 68000 budget, `@budget(cycles: 670)` on the DMA drain (`aeon/engine/system/dma_queue.emp:358`), is machine-checked.
- **Layout as a build-time fact.** `struct`, `region`, `offsets`, bank windows, 1,486 `ensure` guards (MEASURED).
- **An honest checker.** One control-flow graph shared by six analyses, each with a written list of what it does not prove. That is why the holes were quick to find.
- **Build-time programs that work.** The raster and scene languages (7,525 lines) lower authored scenes byte-identically and check HBlank costs per fire.
- **Tests.** Frontend suite: 138 binaries, **2,784 passed, 0 failed, 0 ignored** (MEASURED, release profile, 4.8 s wall clock on a warm cache, Ryzen 7 9800X3D, with `AEON_DIR` set; without it 2 tests refuse to run).

## Findings

Ranked by how much wrong output a green build can hide. Short paths: `emp/…` means `sigil/crates/sigil-frontend-emp/src/…`; `aeon/…` is the engine tree. "Probe" results are MEASURED with the shipping binary, exit 0 unless stated.

> [!finding] 1. Three check families are computed and do not stop the build
> - `[bus.*]`, `[branch.condition-constant]` and `[call.slot-type-mismatch]` are printed by `--report contracts` only. `run_contract_gate` never reads them (`sigil/crates/sigil-cli/src/main.rs:2153-2375`; readers only at `:2524-2575`. I re-checked this).
> - Planted defects in a scratch copy of aeon (sonic4 plain, the release shape): a VDP write with the Z80 bus running, and a wrong ID type on a call. `sigil build` exits 0 both times; the second ROM is byte-identical to the clean one (MEASURED, reviewer A).
> - The bus family replaced the crash-class lint that `aeon/build.sh:964-971` retired on 2026-09-07. So a crash-class check left the build and its replacement only prints. The project's own ledger has it open as "reported, not gated".
> - Clean tree: all three families at 0 firings (MEASURED), so gating them costs nothing today.

> [!finding] 2. One declaration can mean different things in different files
> - **Struct layout is computed in the importing module.** `a.emp` declares `N = 4` and `struct S { x: [u8; N], y: u16 }` (6 bytes). A file that imports `S` and has its own `N = 10` gets `sizeof(S) = 12`. Bytes `70 0C`, no diagnostic (probe `p8b`; **re-run by me, same bytes**).
> - **A `comptime fn` resolves its free names at the caller.** A helper returning its own module's `LIMIT = 5` returns 9 when the caller has a `LIMIT = 9` (probe `p2d`; reviewer B's `q1b` found the same independently). A private helper function is replaced the same way (probe `p2e`).
> - This is the most serious hole here: the language's strongest claim is that layout is a build-time fact. Corpus exposure was **not measured**. The corpus already bends around it: 50 literal pins in the two DSL files, a 10-name "ALL LOAD-BEARING" import line per scene client (`aeon/games/sonic4/data/effects/scene_registry.emp:49-58`), and a hard-coded Rust list of globally injected modules (`sigil/crates/sigil-harness/src/native.rs:1802`).

> [!finding] 3. Sized 68000 immediates are truncated silently, and a bare number is an immediate
> - `move.w #$12345, d0` → `303C 2345`. `cmpi.b #300, d0` → `0C00 002C`, a compare against 44 (probes `x1`, `x5`; **`x1` re-run by me, same bytes**). Only a 32-bit range check exists (`emp/lower/code.rs:1396-1405`).
> - The **AS frontend in the same binary refuses these lines** ("operand 74565 out of range"). The byte-identical port moved the corpus from a frontend that checks immediates to one that does not.
> - `move.w $FFFF8000, d0` assembles as `move.w #$8000, d0` (probe `lit`). In Motorola syntax that is a memory read. A named constant in the same position is diagnosed; a literal is not. No hit in the corpus (grep).
> - This is an unbooked member of a class the project believes it has enumerated (`sigil/docs/superpowers/notes/campaign-gap-ledger.md:4520`).

> [!finding] 4. Every typed indirect call is trusted, not checked
> - `jsr (aN) as Type` narrows the caller's contract to the type's bound, and nothing checks the procedures actually installed (`emp/closure.rs:208-216`, which says so itself).
> - **11 dispatch sites; 0 contract failures with the bound trusted, 71 with it removed** (MEASURED, `--report indirect-cost`, sonic4 plain, release shape). They include `RunObjects`, `Player_Main`, `GameLoop` and the VBlank handler.
> - Root cause: a state change is an untyped number, `move.w #X - ObjCodeBase, code_addr(a0)` (25 sites in shipping code, MEASURED). `ObjRoutine` names two unrelated things (`aeon/engine/system/types.emp:59`, `aeon/engine/objects/core.emp:27`).
> - The one conformance check that does run, on interface bindings, is **skipped when the bound procedure declares no `clobbers`** (`emp/resolve/contract.rs:488`, `:493`; I re-read it). A hook that breaks `preserves(a0)` builds green without the clause and fails with it (probes `s12b_*`). Declaring less passes; declaring honestly fails. Latent: all 8 aeon bindings declare the clause.

> [!finding] 5. `script` and `dispatch` cannot target aeon
> - Both store a table ordinal (`emp/ast.rs:730-736`). aeon's dispatcher is table-less (`aeon/engine/objects/core.emp:595-599`).
> - The spec says the `code_word` encoding is "REQUIRED before the first scripted-object port on real Aeon" (`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md:1297`). It has 0 hits in the frontend source (I re-checked). The one exhibit compiles only against a mock prelude (`sigil/examples/game/badniks/pitcher_plant_script.emp:54-61`).
> - So the 0 uses are a blocker, not a choice. The only game-logic constructs in the language have never run on the engine.

> [!finding] 6. Three interrupt-safety contracts are weaker than they read
> - **`preserves(sr.mask)` checks text order, not paths** (`emp/lower/proc.rs:2528`, `:2916-2941`). A path that throws away the saved status word and returns with interrupts off builds green (probe `d_mask_leak`). 8 of the 16 mask sites are hand-spelled (MEASURED by grep).
> - **A flag result is never checked in the procedure that declares it** (`emp/flag_check.rs:1-12`). An exit that never sets the carry, or sets it before the restore that overwrites it, builds green (probes `c_no_pin`, `b_pin_before_restore`). 18 flag-out contracts in 9 files.
> - **`requires(vblank)` is checked only where the callee is named** (`emp/corpus_contracts.rs:1214`). `VInt_Level` requires it and is only ever reached through `VInt_Ptr`, a bare `u32` (`aeon/engine/system/vblank.emp:110`, `:84`; `aeon/engine/ram.emp:406`). Proven by reading the code path; the whole-build fixture was not run.

> [!finding] 7. The language has no notion of data shared with an interrupt
> - Nothing marks a variable as touched by both the main loop and VBlank, so nothing can ask "is this access masked?". In the DMA enqueue walk, four steps are protected only by one hand-written instruction pair (`aeon/engine/system/dma_queue.emp:126-127`).
> - The rule lives in prose: 241 MUST/NEVER/ordering comment lines (MEASURED, stage 2); `vblank.emp` is 360 comment lines of 553 (MEASURED).
> - Data shared between the 68000 and the Z80 (the sound mailbox, the DMA-active flag) is a separate gap: one cell is declared twice by hand (`aeon/engine/sound/sound_api.emp:57-65`), and the known mailbox race is a protocol error that atomic access does not cure.

> [!finding] 8. Cycle budgets cannot reach the code that needs them
> - `@budget` refuses any loop, any call, any tail jump and any entry but the first instruction (`emp/cycle_budget.rs:27-37`, `:75-77`). Result: **1 use in 605 procedures, 0 on the Z80, against 349 comment lines of hand-added cycles** (MEASURED).
> - VBlank, HBlank, sprite emit, every object routine and the Z80 sequencer tick are unbudgetable. Logic over budget shows up only as lag frames. DEBUG and release differ in instructions inside the same procedure and nothing budgets each shape.

> [!finding] 9. Comptime is powerful and unsound
> - **Declared types are not enforced.** A function declared `-> int` returns `()`; an `[int; 3]` parameter accepts `()` elements; an enum payload declared `(int, array)` accepts two ints (probes `p4`, `q3`).
> - **A body nobody calls is never checked.** A function containing `ensure(1 == 0)` and unknown names builds green (probe `q2`). Hand "anchor" functions exist only to force checking (`aeon/engine/effects/raster_dsl.emp:1634`).
> - With finding 2, this drops reviewer B's rating of comptime from "on par to best known" to "below best known in soundness". It matters twice: the engine's tables and proofs are computed here (4,499 lines), and any compiled tier would be built on this evaluator.
> - The two big DSLs use **zero** lambdas, pipes or `map`/`fold`, though all were available. They are imperative code over values.

> [!finding] 10. State machines are ported structure held together by hand proofs
> - 13 `PSTATE_*` values are hand-typed offsets that must match three parallel tables; the guard is a popcount proof and about 45 comment lines (`aeon/games/sonic4/config/constants.emp:248-339`; `aeon/games/sonic4/player/player_common.emp:1982`, `:2059`, `:2088`). Swapping two rows of one table would stay green (reasoned by reviewer B, not run).
> - The engine's camera mirrors two of the game's state numbers (`aeon/engine/level/camera.emp:52-62`). A non-Sonic game inherits a meaningless mirror.
> - `PState_Spindash` reads `a4` without declaring it (`aeon/games/sonic4/player/player_spindash.emp:89`, `:106`); 24 `PBLK_*` constants in 8 files duplicate a struct that exists.
> - Spawn-list terminators are "ADJACENCY-CRITICAL" by comment in 3 of 5 places (`aeon/games/sonic4/objects/test_parent.emp:166`).

> [!finding] 11. A boss cannot say "wait, unless"
> `wait_frames` is a pure park. A boss that must check its health during every wait (the commonest Treasure shape) becomes a hand loop with the check repeated in each one. Found by porting the named boss on paper (reviewer B). Fixable as fixed-expansion sugar.

> [!finding] 12. The corpus reads as a port, and prose does the language's work
> - 16,046 instruction lines carry 48,841 comment-only lines: **3.0 comment lines per instruction**, 57% of all lines (MEASURED).
> - 931 comment lines restate contracts the build already checks. 1,033 comment lines carry a date. 367 echo the old AS operand. 198 name deleted `.asm` files (all MEASURED).
> - The same behaviour is written twice where comptime must model code it cannot run: a 5-line functional model and a 222-byte routine (release) for the palette variant, joined only by a Python gate under the emulator (`aeon/engine/effects/palette_dsl.emp:61-76`, `aeon/engine/effects/palette.emp:806-869`).
> - There is no enemy, boss or projectile pattern in the corpus. `games/sonic4/objects` is 15 files: 11 tests, two dust effects, a ring sparkle and Tails' tails. Every game-logic ratio in this audit rests on DEBUG-only test objects and paper ports.

> [!finding] 13. The frozen spec and the editor are behind the language
> - 24 shipped constructs appear in no `SIGIL_*.md`, including `requires`, `interface`, `@budget`, `cycles()` and `mul_const`: the layer the verdict rests on (MEASURED, stage 0).
> - Tenet 3 ("the instructions don't change") is already relaxed three ways (`mul_const`, branch widths, templates), and its stated reason, byte-exact AS migration, is finished.
> - Editor support is syntax colouring only, with 38 parser words missing. Single-file builds run a weaker checker than the whole build, so a per-file editor loop would show green on code the build rejects.

> [!finding] 14. Smaller gaps
> - **Z80 bank window** is machine state with a bracket shape and no construct: 63 comment lines carry the rule. 6 of 41 Z80 `out` claims are baselined as unverified (MEASURED).
> - **Width pins.** `align` at a not-yet-placed position is refused, so 16 sized branches are kept by hand in `aeon/games/sonic4/debug/game_debug.emp:69-77`, against the conventions.
> - **Known pitfalls still silent:** nested value-`if` folds to `()` at the fold; guards in modules outside the import closure never run (45 to 50 modules, from the pitfalls doc, not re-measured).

### Functional everywhere, area by area

Ratios are GCC against hand code. Stage 1 used GCC 15.2 `-O2 -mcpu=68000` with cycles from sigil's table. Stage 2 used GCC 13.3 `-O2 -m68000` with cycles counted by hand. **The two are not comparable figure for figure.**

| Engine area | Share of code lines | What a compiler cannot match | Cost if compiled | Hatch would hold | Answer |
|---|---|---|---|---|---|
| HBlank handlers | part of 14.0% cycle-critical | Registers pinned across interrupts, port held in an address register, spins solved to the cycle | 2.6 to 5.1 times; 46% of a scan line for one write (stage 1, MEASURED codegen) | All of it | Exact |
| VBlank, DMA drain, VDP copies | same | Unrolled jump-into tables, ordering tied to VDP state | 1.47 to 2.30 times (stage 1); window 82% used release, 97% DEBUG (MEASURED, frame audit) | All of it | Exact |
| Z80 sound driver | 3,758 lines, 81% of cycle-critical | Path-cost *equality* to the T-state; register state held across a polled tick | **Not measured** (no Z80 compiler was run). "Exact" rests on what can be expressed, read from the driver by reviewer A only | All of it | Exact |
| Decompression, tile streaming | within engine systems 24.1% | `(An)+` streams, `dbf`, bit-buffer tricks | About 1.1 to 1.5 times (ESTIMATED) on code that already causes lag frames | Inner loops | Exact |
| Sprite emit, object core | 6.9% + 1.9% tight loops | Fused `dbeq`, pad-byte layout trick | 1.73 times per sprite piece, 194 against 112 cycles (stage 2: GCC codegen MEASURED, cycles ESTIMATED; release ROM) | Hot loops | Exact |
| Player physics | part of 14.1% | Rounding that *is* the S3K behaviour | No physics routine was compiled. Nearest figure: 1.46 to 1.83 times on the two test objects (stage 2, DEBUG ROM routines, cycles ESTIMATED) | Per routine | Exact, with better state-machine surface |
| Choreography (phases, scripts, events) | part of 14.1%; none exists yet | Nothing, with the right calling convention | 1.0 to 1.15 times (ESTIMATED, for a compiler that does not exist) | Nothing | (b) now, (c) behind the gate |
| Build-time tables and DSLs | 13.6% | n/a | 0 | n/a | Already functional; make it sound |

## Reviewer agreement

Reviewer A was Fable, reviewer B was Opus; B never read A.

| Topic | Reviewer A | Reviewer B | My call |
|---|---|---|---|
| As an assembler | Best known; contracts beyond | Best known; contracts beyond | Agreed |
| As a game-logic language | "On par with S3K-style macro assembly" | "Below the best common tools" | Same fact from two sides: **S3K level**, which is the floor and below what SGDK's C or a GOOL-style coroutine gives an author |
| Disposition | Keep and patch | Patch the exact tier | Agreed: keep, patch |
| `script`/`dispatch` 0 uses | A blocker: `code_word` unbuilt | Same, found independently | Agreed. Stage 1's "already built, cheap win" is wrong |
| Functional everywhere | No; hatch up to 47% of lines | No; hatch about 40% | Agreed: no. Both shares are ESTIMATED upper bounds |
| A compiled tier | Not yet; an experiment after the boss exists in the patched language | A gated "rewrite beside", choreography only | Compatible. I take A's order and B's gate and scope |
| Comptime quality | Best known | First "on par to best known"; after the checker round "below best known in soundness" | B's later rating. A's own findings 11 and 12 show the same defect |
| First thing to fix | Silent holes | `code_word` | Holes first: they are small and belong between waves. Then `code_word` |
| Structured `if`/`else` sugar | Yes, rank 8 of 12 | Dropped as low value | Deferred. The old reason against it is obsolete, but the waste is in state registration, not branch spelling |
| "2.2% higher constructs" (stage 2) | Used as is | Under-counts: 1,437 instruction lines carry a typed field operand | B is right. "An assembler with typed operands and contracts, and no state-machine surface" |
| Cost of a `script` | Not costed | +12 cycles per self-yield; about +300 cycles a frame (0.2%) for 24 enemies and a boss (ESTIMATED) | B's. Stage 1's "zero cost" is wrong for aeon |
| Interrupt-shared data | Proposes `shared(vblank)` | Not examined | Keep A's proposal; it does not cover cross-CPU data |
| The palette "two programs" problem | Run procedures at build time on the emulator core | Compile the model (a tier gate test) | Both stand. A's serves the exact tier everywhere and comes first |

Found by one reviewer only: A found findings 1, 2 (struct half), 3, 6, 7. B found findings 4, 10, 11 and the uncalled-body and payload-type halves of 9.

## Checker corrections

36 claims re-opened: **30 hold, 3 wrong, 3 mislabelled**. None changes a verdict. Every probe the verdicts rest on reproduced.

| Correction | Effect |
|---|---|
| Reviewer A listed the ADDX pitfall (§7) as open | **Fixed** in the shipping binary: it is now a loud error. Removed from the open list |
| Reviewer B priced the sprite loop at 1.07 to 1.25 times "if compiled" | That was the estimate for a compiler nobody has written. GCC measured **1.73 to 1.77 times**. Strengthens "hot loops stay exact" |
| Reviewer A labelled "1.3 to 2.8 times" as MEASURED for both stages | Stage 2's cycles are ESTIMATED by hand count; the two stages use different compilers and cycle sources |
| Stage 1 quoted "97% of vertical blank" with no build | That is DEBUG. Release is 82% |
| 25 against 26 state-store sites; "7 instructions" was 6 | 26 total, 25 in shipping code; `TestEmitter_Main` has 6 |
| Reviewer A's walk said `[bus.vdp-write-unstopped]` "fires" | It prints and does not gate. This became finding 1 |
| Neither reviewer had read the Z80 driver | A read the sample loop and the tick header in the follow-up round. `sound_sequencer.emp`, `sound_sfx.emp`, `sound_psg.emp` (about 2,500 code lines) are still **unread**. B's "no" for the Z80 rests on stages 1 and 2 only |
| Neither had read the two comptime DSLs | B read the headers and key blocks and covered the rest by grep. This produced finding 9's second half |
| 9 of 14 pitfall sections unexamined | §2, §7, §8 now examined and probed. §5 and §9 read, not probed. §4, §6, §10, §11, §13 still unexamined (carried below) |
| Interface check skipped without `clobbers` (lens S12-b) | Confirmed by probe; folded into finding 4 |

**Stage 0 process breach, disclosed by that agent:** a mis-typed first test launch wrote about 185 build-artifact files into `sigil/target/release/` (`deps/`, `.fingerprint/`, `build/`). The `sigil` binary aeon links against is unchanged. Nothing was deleted. Carried to the last section.

**Scope not covered, by decision:** `SIGIL_CORE_SPEC`, `SIGIL_M0_*`, `SIGIL_PLAN_AUDIT` and `SIGIL_ORACLE_ISA_SHARING` were read as headings only and belong to the [[Sigil assembler]] audit. About 175 of the 190 hand-written `.emp` files were covered by scripts and greps, not read line by line. `aeon/CODING_CONVENTIONS.md` was read as headings only.

## Plan

Size classes are the reviewers' (Small / Medium / Large); nobody priced them in days. "Bytes" says whether the ROM changes.

| # | Improvement | Gain | Cost | Risk | Bytes |
|---|---|---|---|---|---|
| 1 | **Gate the three printed families** (finding 1): three rows in `run_contract_gate` | A crash-class check stops the build again | Small | None: 0 firings today | No |
| 2 | **One name environment per declaration** (finding 2): a `comptime fn` resolves names where it is defined; a struct is laid out once, in its own module | Removes silent wrong sizes; retires 50 pins, the scene import tax and the Rust helper list | Medium | Code that relies on caller-side lookup changes meaning. The first run must list every name whose two resolutions differ | Only if a divergence exists today (not measured) |
| 3 | **Close the silent holes** (findings 3, 9, 14): immediate range check, bare-literal operand error, enforced comptime return/element/payload types, type-check uncalled bodies, nested-`if` unit error | Four ways to ship a wrong ROM from a green build disappear | Small each | The immediate check may flag existing lines; that list is the point | No, unless a truncating line exists |
| 4 | **Make the contracts true** (findings 4, 6): callee-side flag proof; path-sensitive SR proof; conformance at every `as Type` site and binding, never skipped; `requires` carried on procedure types and typed handler slots | The headline guarantee holds at dispatch and in masked routines | Medium; 71 contracts to correct (release shape) | Medium: one aeon + sigil parcel | No |
| 5 | **State-machine surface** (findings 5, 10, 11): `code_word` encoding; typed `set_state`; one multi-column `states` table with cross-module ordinals; typed inputs on handlers; sentinel record tables; `wait_frames … until` | Named states, timed waits and typed links on aeon; deletes the PSTATE proof, 24 `PBLK_*` constants and the camera mirror | Small to medium | Low; design against a real object, not the test objects | Neutral for `set_state`; a script adds about 4 bytes and 12 cycles per self-yield (ESTIMATED) |
| 6 | **Budgets that compose** (finding 8): callee budgets, declared loop bounds, roots at export labels, link-time sums per build shape | Replaces 349 hand-count lines; turns run-time overruns into build errors; is the acceptance gate any compiled tier needs | Medium | Bounds must be honest; tie them to refinement types | No |
| 7 | **Use the checked interrupt bracket in multi-exit routines** by patching the carry in the saved status word (builds today, probe `best.emp`); make the sound post routine test-and-post with a must-use `out(carry: busy)` | 2 of 8 hand-spelled mask sites become proven; about 4 cycles off each successful enqueue (ESTIMATED); closes the mailbox race with an existing feature | Small | Needs a headless regression run | **Yes** (DMA enqueue, sound post) |
| 8 | **Interrupt-shared data**: `shared(vblank)` on variables (finding 7) | Torn state between main loop and VBlank becomes a build error | Medium to large | False positives until single-instruction access is modelled | No |
| 9 | **Write the named workload** in the result: one multi-part boss, one enemy script, each under `@budget` | The first real evidence for or against a compiled tier | Small once 5 and 6 exist | None | New code |
| 10 | **Comment diet with tooling** (finding 12): generate the interface text from the contract; move dated history to git and decision records; lint contract-restating comments and AS echoes | Files readable by a second developer; the 57% comment share falls sharply | Small tooling, large mechanical edit | Losing real rationale: move it, do not delete it | No |
| 11 | **Spec and grammar from one source** (finding 13): fold the 24 constructs into Spec 2; restate tenet 3 as "every emitted instruction is predictable from its source line and priced"; generate the grammar from the parser tables; a language server later | Ends spec drift; first real editor support | Medium (language server Large) | Low | No |
| 12 | **Run procedures at build time** on the suite's own CPU core inside `comptime test` | One program instead of model + code + Python gate; unit tests for assembly | Medium to large | Build time; memory-model design | No |
| 13 | Link-time `align` gap-fill; a bank-window context for the Z80 (finding 14) | Retires width pins and 63 comment lines of bank rule | Small each | Low. Whether declared contexts work in Z80 modules is unchecked | No |
| 14 | **Compiled choreography tier, prototype only**, behind the gate below | Paper port: boss and enemy script 4 to 7 times shorter than today (ESTIMATED from drafts); "check health during every wait" stated once | Large | Medium to high: a second language | New code only |

**Order.** 1 to 3 are cheap and should land before wave 2 audits the tree. 4 to 6 make the exact tier fit for game logic. 9 is the proving ground. 14 is decided only after 9.

**Gate for item 14** (reviewer B's, with A's trigger). Build the tier only if the boss and script from item 9 are more than about twice the size of the functional sketch or show bug classes the contracts cannot see. Then a prototype must compile one boss phase and the palette model at **no more than 1.15 times the cycles and 1.25 times the bytes** of the hand version, measured by the same `@budget` walker, under these rules:
1. Register contracts are the calling convention; no stack arguments.
2. In-place slot mutation is guaranteed; no heap, no copies.
3. States lower to `code_word`; no interpreter, no hidden tables.
4. Every compiled procedure is fenced out of interrupt and Z80-bus contexts by the existing context checker.
5. Items 2 and 3 are done first, so the tier does not inherit an unsound evaluator.
6. It is statement-shaped (`phase`, `wait`, `repeat`, `until`), not Haskell-shaped. The corpus's own 7,525 lines of build-time code use no functional combinators.

**Dropped.** C through GCC or SGDK (stack calling convention, 1.3 to 5.1 times, no flag results, no contracts). An LLVM or Rust backend (clang calls a library for a 16×16 multiply, MEASURED; Rust's target is tier 3). A bytecode interpreter for objects (about 26 cycles per operation of pure overhead, ESTIMATED). A compiled tier for player physics (little saving, rounding risk where S3K accuracy lives). `while`/`switch` blocks where the compiler picks the compare or table shape.

## Beyond state of the art

| Candidate | Plausibility | Needs |
|---|---|---|
| **Build-time proof that the frame fits**: `ensure(budget(VInt_Level) <= VBLANK_CYCLES - margin)` per build shape | High. The cycle tables and control-flow graph exist. No Genesis toolchain proves its VBlank fits | Item 6 |
| **Checked coroutine objects**: closed resume sets, conformance-checked dispatch, a cycle budget per object | High. No shipped or homebrew Genesis toolchain has it | Items 4, 5, 6 |
| **Interrupt-race checking for assembly** | High for declared variables; whole-program inference is not needed | Item 8 |
| **Assembly unit tests run at build time on the emulator's CPU core** | Medium to high | Item 12 |
| **A choreography tier whose output passes the same contract and cycle checks as hand code** | Medium. The risk is compiler effort, not feasibility | Item 14 |
| **Typed VDP port state** (write latch, auto-increment) as a context | Medium | Value analysis the spec lists as out of scope |

## Needs a live run or the owner's machine

| Item | Why it is open |
|---|---|
| Regression run for the saved-SR carry patch (plan item 7) | Equivalence is argued, not tested. Byte-changing. Needs a headless emulator run |
| First run of the immediate range check over the corpus | Unknown whether any existing line relies on truncation |
| Corpus exposure of finding 2 | Nobody searched for helper names that collide with a caller's, or for modules that disagree on a struct's size |
| Whole-build fixture for `requires` through a pointer (finding 6) | Proven by reading `corpus_contracts.rs`; one planted pointer call would settle it |
| Row-swap test on the player state tables (finding 10) | Reasoned, not run |
| Emulator-measured cycles for the stage 2 routines (112 a sprite piece, 316 on the spindash path) | All stage 2 cycle figures are hand counts; a headless probe would confirm them |
| A Z80 compiler comparison (SDCC) on the sound driver | DID NOT RUN: no Z80 C compiler installed. The Z80 "exact" answer rests on expressibility, not a ratio |
| GCC as SGDK ships it, and vbcc | DID NOT RUN: not available. Ratios hold for GCC 13.3 and 15.2 only |
| Pitfalls §4, §6 (beyond trap C), §10, §11, §13; §5's "error points at an unrelated file"; §9; the imported-data shape of the label-against-int rule; §14 across modules; the 45 to 50 dead-guard modules of §3 | Unexamined or not re-measured by either reviewer |
| `@scaffolding` suppresses a lint that does not exist (lens S13) | Unexamined; 5 uses in the corpus |
| `sound_sequencer.emp`, `sound_sfx.emp`, `sound_psg.emp`; `emp/lower/script.rs` beyond its first 60 lines; `type_slice.rs`, `resumable.rs`, `eval/*`; `aeon/CODING_CONVENTIONS.md` | Not read. The `mulu`/`divu` rule is enforced by comment only and neither reviewer listed it |
| Can `table` express a `SpawnDesc` list today? Can template-generated labels be named from outside? Do declared contexts work in `cpu: z80` modules? | Each was flagged "not checked" by a reviewer and gates part of items 5, 6 or 13 |
| About 185 stray files in `sigil/target/release/` from the stage 0 breach | The controller's or owner's call: `cargo clean -p sigil-frontend-emp --release`, or leave them. Both are safe |

Cut: the build-loop time (docs say 1.3 s) was not re-measured as a requirement. The two builds that did run took 0.6 s (report run) and 3.3 s (full `sigil build` on tmpfs), release-profile sigil, sonic4 plain, one run each, so no item is needed.

## Decisions for you

1. **Language direction.** Take option (b): exact instructions + checks + fixed-expansion sugar, with soundness first; no compiled tier now; functional everywhere: no.
   **Recommended: yes.**
2. **Land plan items 1 to 3 before wave 2 audits the tree**, accepting that the first run may flag existing lines and that item 2 changes what a helper sees.
   **Recommended: yes.** They are the cheapest fixes here and every later audit trusts the build's "no".
3. **Make a real boss and a real enemy script the proving ground** (item 9, after items 5 and 6), and decide the compiled tier only on that evidence, under the gate above (1.15 times cycles, 1.25 times bytes, choreography only, statement-shaped).
   **Recommended: yes.** It replaces an estimate about an unwritten compiler with a measurement.
4. **Unfreeze Spec 2 and restate tenet 3** as "every emitted instruction is predictable from its source line and priced", folding in the 24 unspecified constructs.
   **Recommended: yes.** The byte-exact reason for the old wording is finished. Leave `if`/`else` flow sugar deferred.
5. **Approve the mechanical comment diet** (item 10): generate interface text from contracts, move dated history to git and decision records, strip AS echoes.
   **Recommended: yes, after item 4**, so the generated text describes contracts that are true.

## Reports

- [[emp-language/0-inventory|0 Inventory]]
- [[emp-language/1-research|1 Research]]
- [[emp-language/2-measurements|2 Measurement]]
- [[emp-language/3-reviewer-a|3 Reviewer A]]
- [[emp-language/4-reviewer-b|4 Reviewer B]]
- [[emp-language/5-checker|5 Checker]]
