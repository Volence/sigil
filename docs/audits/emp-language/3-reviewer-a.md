---
kind: audit-stage
system: emp-language
stage: "3 Reviewer A"
agent: Fable
status: done
description: "Best known as an exact-instruction assembler (contract layer beyond any other); not yet a game-logic language. Keep + patch: close 8 proven silent holes (checker round added 3: ungated bus/branch/slot families, call-site-scoped comptime fns, importer-sized structs), make budgets and interrupt contracts reach real code, bind script/dispatch to aeon. No .emp2 yet; functional-everywhere: no. Verdict unchanged by the checker round."
---
# EMP language: reviewer A

> [!verdict] Conclusion
> - **Verdict: best known**, as an exact-instruction language. The register-contract layer is **beyond** every assembler stage 1 checked. As a language for *game logic* it is only **on par** with S3K-style macro assembly: the two constructs built for it (`script`, `dispatch`) cannot target aeon's object dispatcher, and the corpus holds no enemy or boss to judge against.
> - **Disposition: keep, and patch.** Keep exact instructions for the engine. Do not start a separate functional `.emp2` now.
> - **Direction: "exact + checks + fixed-expansion sugar"** (option b), carried through to game logic first. A compiled tier for logic (option c) becomes a bounded experiment *after* the named boss and shmup script exist in the patched language. **Functional everywhere (option d): no**; the per-area table is in section 8.
> - **Five holes proven by building probes with the shipping `sigil` binary** (section 6): sized 68000 immediates truncate silently; a bare number operand is silently an immediate; `preserves(sr.mask)` accepts a path that returns with interrupts masked; `out(carry: …)` is never checked on the callee side; `requires(vblank)` is not checked through a pointer.
> - **The protection thins out exactly where the hardware is.** In the walked routine the language proves register effects and stack balance, and nothing about which data the VBlank shares, the cycle cost of anything with a loop or call, or the VDP's own state.
> - **One free win found on the way:** the DMA enqueue core can use the checked `with ints_off` bracket today by patching the carry bit of the *saved* SR word. `engine/irq.emp:63` says this is structurally impossible. The probe builds clean and is ESTIMATED 4 cycles faster on the success path.

Trees: aeon `936439fa`, sigil `7630ba0d` (same as stages 0 to 2). Probes were built with `sigil/target/release/sigil emp` (binary dated 2026-11-13); sources and outputs are under `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/audit-builds/reviewer-a/`. No repo was written.

Frontend tests, my run: `AEON_DIR=… cargo test --release -p sigil-frontend-emp --offline --no-fail-fast` into the scratch target dir: 138 binaries, **2,784 passed, 0 failed, 0 ignored** (MEASURED; 4.8 s wall clock, release profile, warm build cache, same host as stage 0).

---

## 1. Requirements

### What the language must do

| For | The language must | Today |
|---|---|---|
| **Sonic 4 vertical slice** | Let hot code (VBlank, HInt, DMA, sprite emit, tile streaming, the Z80 driver) be written instruction for instruction, with its timing checked by the build. Let player and object logic be written and reviewed quickly | Exact instructions: yes. Timing checked: 1 `@budget` and 28 `cycles()` calls against 349 hand-counted comment lines (stage 2, MEASURED). Logic: plain assembly with contracts |
| **Mega-acts, infinite levels** | Make data layout a compile-time fact (structs, tables, bank windows, RAM regions) so streaming code cannot drift from its data | Strong: `struct`, `region`, `offsets`, `bank:`, 1,486 `ensure`. Weak spot: 352 guards pin a name to a literal (stage 0) |
| **Run-and-gun boss** (Gunstar / Alien Soldier style: multi-part, state machine, many projectiles) | Named states with per-frame suspend and timed waits; typed links between parts; cheap spawn lists; a way to say "this proc runs in at most N cycles per part" | States are hand-written `move.w #X - ObjCodeBase, code_addr(a0)`. Links are an untyped `movea.w parent_ptr(a0), a1` (`test_parent.emp:69`, `:102`). The spawn list's terminator is "ADJACENCY-CRITICAL" by comment (`test_parent.emp:166`). No cycle bound is possible (loops and calls refuse `@budget`) |
| **Shmup enemy script** (Thunder Force IV style) | A timed sequence (`wait`, move, fire, loop) authored as text, lowering to the dispatcher with no interpreter cost; pooled bullets in a tight loop | `script` with `wait_frames` exists in the language and cannot target aeon (finding 8). The bullet loop is fine as exact code |
| **Another game's developer** | Classic assembly habits must either work or fail loudly | Two silent traps for exactly that reader (findings 1 and 2) |

### Budget targets

| Budget | Target | Basis |
|---|---|---|
| Run-time cost in interrupt, DMA, sprite, streaming and sound code | 0 cycles and 0 bytes over hand assembly | Worst VBlank already uses 82% of vertical blank in release and 97% in DEBUG (stage 1 §4.3, citing frame-architecture stage 2, MEASURED there) |
| Run-time cost of any higher tier on game logic | At most about 5% of a frame on the named workload, proven by `@budget`, not by estimate | Stage 1 costs GCC-class code at +5.6k cycles = 4.4% of a frame for 1 boss + 6 limbs + 64 bullets + 24 enemies (ESTIMATED there) |
| RAM, VRAM | 0. The language may not allocate | A functional lowering that builds intermediate lists is not viable in 64 KB (stage 2 §5.2) |
| ROM | Higher constructs within about 10% of hand size on logic | GCC is 1.4 to 2.5 times larger (stage 2, MEASURED) |
| Authoring cost | A new object in one screen, no import ritual (tenet 2) | `test_emitter.emp`: 6 instructions, 6 `use` lines (MEASURED) *(corrected in the checker round: was "7")* |
| Build loop | Stay near the 1.3 s content loop (`aeon/CLAUDE.md`, PUBLISHED) | Not re-measured here |

### Worst case it must survive

A frame with the boss, its parts, 64 bullets and 24 scripted enemies, raster effects firing in HBlank, and a tile-streaming DMA burst. And a second, slower worst case: a developer who did not write the engine changes a struct or a queue that the VBlank shares with the main loop. The language must stop the second one at build time.

### Interfaces and non-goals

- **Interfaces:** the sigil core IR and linker (Sigil assembler audit); Oracle for source-level debugging (any compiled tier needs a line-to-instruction map there); aeon's object dispatcher (`Sst.code_addr`, table-less, `core.emp`); the DMA and VBlank budgets (owned by the VRAM/DMA audit).
- **Non-goals:** byte-identical AS migration (finished, per stage 1 point 5); general-purpose programming; garbage collection or heap allocation of any kind.

---

## Walk A: one routine through the language

**Routine:** the DMA enqueue core, `QueueDMA_Critical` / `_Important` / `_Deferrable` → `.transfer` (`aeon/engine/system/dma_queue.emp:102-283`), and the VBlank side it races against, `Process_DMA_Critical` (`:358-410`) called from `VInt_Level` (`vblank.emp:206`). It masks interrupts, builds entries the VDP will read verbatim, and is drained inside vertical blank. The region `:102-283` holds 72 instruction lines and 65 comment-only lines (MEASURED).

### Step by step: where an interrupt can land, what protects the step, what is by hand

| # | Lines | What happens | Can VBlank land? What is half-done? | What the language proves | What the author tracks by hand |
|---|---|---|---|---|---|
| 1 | `:102-116` | Entry point loads the slot-variable address into `a2`, the queue end into `d4`, tail-jumps to the shared core | Yes. Nothing shared is touched yet | `clobbers(d0-d4/a1-a2)` is checked transitively; the tail to `.transfer` is credited for both `clobbers` and `preserves(sr.mask)` (`lower/proc.rs:2562-2584`) | `a2` and `d4` are untyped. `move.w #DMA_*_End, d4` only works because upper RAM is word-addressable (the file's own not-covered item (b), `:644-646`) |
| 2 | `:126-127` | `move.w sr,-(sp)` / `move.w #$2700,sr` | No, from here | `[proc.sr-undeclared]` forces the SR write into the contract. Stack balance is proven on every path (`preserves.rs` header) | The mask is **hand-spelled**: `with ints_off` allows one exit and this core has five (`irq.emp:58-71`). 8 of the 16 mask sites in the tree are hand-spelled (MEASURED by grep) |
| 3 | `:128-130` | Read the cursor, compare with the end, `beq .full` | No | Nothing | That `DMA_*_Slot` is shared with VBlank is known only from comments. "Full" is tested by equality, which is safe only while each span is a whole number of entries: held by four link-time `ensure`s (`:650-657`) |
| 4 | `:140-145` | Charge the frame's byte cap (read, add, compare, write back) | No. Unmasked, VBlank's `clr.w DMA_Enq_Bytes_Frame` (`vblank.emp:146`) could land between read and write and the charge would be lost | Nothing | The whole read-modify-write rule |
| 5 | `:147-162` | Convert to words, write source and length with `movep` into the entry | No. Unmasked, VBlank would drain a half-written entry straight into VDP registers | `DMAEntry.SizeL(a1)` is a named field | `a1` is not typed as `*DMAEntry`; a wrong field name is not resolved by name (EMP_PITFALLS §14). The `movep` interleave (register numbers at even offsets, values at odd) is not expressible in `struct`; only `sizeof == 14` is pinned (`:332`, and not-covered item (a)) |
| 6 | `:164-175` | Build the VDP command word, advance the cursor, restore SR, **then** clear carry, `rts` | VBlank can land after the restore; the entry is complete by then | `out(carry: dropped)` makes every *caller* consume the carry (`flag_check.rs`) | The order "restore, then pin carry" is a comment (`:77-79`). Nothing checks the callee produces the carry (finding 4) |
| 7 | `:177-197`, `:261-282` | Three reject exits: restore, set carry, `rts`. The split-reject path first rolls back the byte charge | As step 6 | Stack balance per path | That every exit restores the mask: the check is textual order, not paths (finding 3) |
| 8 | `:202-259` | 128 KB split: write two entries, advance the cursor by two | No | Nothing beyond steps 5 and 6 | `ENTRY2_*` offsets are derived consts (good); the "need two free slots" rule is code plus a 9-line comment |
| 9 | `vblank.emp:43-97` | VBlank: `movem` all registers, dispatch through `VInt_Ptr` | n/a (IPL 6) | `clobbers()` + `preserves(d0-d7/a0-a6)` is proven. `irq_frame.pc` gives the stacked PC without a hand constant (`:70`, `:76`) | `VInt_Ptr` is a bare `u32` (`ram.emp:406`); `jsr (a0) as VBlankHandler` bounds clobbers only. `grants(vblank)` is a trust root by design |
| 10 | `dma_queue.emp:358-410` | Drain: computed `jmp` into an unrolled table, 14 bytes per entry to the VDP control port | n/a | `requires(vblank)` is checked at direct call sites. `@budget(cycles: 670)` is machine-checked: the only budget in the tree | The slot count is hand-spelled 8 wide and pinned by `ensure` (`:327`). The slot stride "`lea.l` 6 + `lea.w` 4 + `bra.w` 4 = 14" is a comment plus an `ensure` (`:332`). `targets(...)` is the author's claim (`cycle_budget.rs:59`). The VDP flip-flop precondition is a comment (`:353-356`) |
| 11 | `vblank.emp:128-137`, `:291-295` | Z80 bus: flag bracket when sound is on, full stop when off | n/a | `with z80_stopped` proves acquire/release pairing (gated). `[bus.vdp-write-unstopped]` is computed and printed by `--report contracts` only; it does **not** stop `sigil build` *(corrected in the checker round: was "fires"; see finding 10)* | That the *flag* bracket (sound on) really covers every DMA is a 15-line comment |

### What the hardware sees

- Because of step 2, the VDP only ever receives whole entries. That guarantee rests on one hand-written instruction pair per masked routine.
- The cost of the masked span (how long VBlank can be delayed) is not bounded by anything: `@budget` roots at a proc's first instruction only (`cycle_budget.rs:75`), and this core is entered through an `export` label.
- The drain's 670 cycles are nominal; FIFO waits and DMA stalls are outside the model, as the checker's own header says (`cycle_budget.rs:67-70`).

### Where a mistake would get through today

| Mistake | Caught? | Evidence |
|---|---|---|
| An exit forgets `move.w (sp)+, sr` | **Yes**, `[stack.unbalanced]` | probe `e_no_restore.emp` (MEASURED) |
| An exit discards the saved SR (`addq.l #2, sp`) and returns with interrupts masked for good | **No** | probe `d_mask_leak.emp` builds, exit 0 (MEASURED) |
| An exit pins the carry *before* the restore, so the caller reads its own stale carry | **No** | probe `b_pin_before_restore.emp` builds (MEASURED) |
| An exit never sets the carry at all | **No** | probe `c_no_pin.emp` builds (MEASURED) |
| A new main-loop routine writes `DMA_Deferrable_Slot` without masking | **No** | no construct marks a variable as interrupt-shared; `DMA_Deferrable_DropDest` states "THE CALLER HOLDS THE INTERRUPT MASK" in a comment and a DEBUG-only `assert` (`dma_queue.emp:581`, `:594-598`) |
| The byte charge or cursor update is moved above the mask | **No** | same gap |
| `DMAEntry` gains a field so the `movep` bytes shift | **Partly**: `sizeof == 14` fires on a size change; a same-size reorder passes | `dma_queue.emp:332`, `:643` |
| `Process_DMA_Critical` is called from the main loop | **Yes** for a direct call (`[context.unsatisfied]`, whole-build gate); **no** through a pointer | finding 5 |
| The drain grows past its window | **Yes**, `[cycles.over-budget]` | `dma_queue.emp:358` |
| `#DMA_ENQ_BYTE_CAP` or another immediate outgrows its operand size | **No**, truncated silently | finding 1 |

### The same routine as the best exact-instruction `.emp` would express it

Every line below is still one 68000 instruction the author wrote. Lines marked **today** were built with the shipping binary (probe `best.emp`, MEASURED: builds clean with `out(carry: …)` and `preserves(sr.mask)`). Lines marked **proposed** need the language work listed in section 8.

```emp
// engine/ram.emp
vars … {
    shared(vblank) DMA_Deferrable_Slot: near *DMAEntry,   // proposed: interrupt-shared + typed short pointer
    shared(vblank) DMA_Enq_Bytes_Frame: u16,              // proposed
}

// engine/structs.emp
struct DMAEntry (size: 14) {
    len:     movep u16 @ 1,          // proposed: the field's type says "odd bytes, register numbers between"
    src:     movep u32 @ 3,
    command: u32       @ 10,
}

/// Queue one 68k-to-VRAM transfer. Carry set = dropped; the caller retries next frame.
pub proc QueueDMA_Deferrable (d1: SrcAddr, d2: VramAddr, d3: ByteLen)
        clobbers(d0-d4/a1-a2) out(carry: dropped) preserves(sr.mask) {
        lea     DMA_Deferrable_Slot, a2
        move.w  #DMA_Deferrable_End, d4

    @budget(cycles: 400)                          // proposed: a budget rooted at an export label
    export .transfer (a2: *near *DMAEntry, d4: QueueEnd):   // proposed: typed shared entry
        with ints_off {                           // today: one exit, release proven on every path
            movea.w (a2), a1                      // proposed: a1 takes the type *DMAEntry from a2
            cmpa.w  d4, a1
            beq     .full
            move.w  DMA_Enq_Bytes_Frame, d0       // proposed: legal only because ints_off is held
            add.w   d3, d0
            bcs     .capped
            cmpi.w  #DMA_ENQ_BYTE_CAP, d0         // proposed: immediate range-checked against .w
            bhi     .capped
            move.w  d0, DMA_Enq_Bytes_Frame
            lsr.l   #1, d1
            bclr.l  #23, d1
            movep.l d1, src(a1)                   // typed register: bare field name, checked
            lsr.w   #1, d3
            moveq   #0, d0
            sub.w   d3, d0
            sub.w   d1, d0
            blo     .split
            movep.w d3, len(a1)
        .finish:
            moveq   #0, d0
            move.w  d2, d0
            vdp_comm_reg(d0, VdpTarget.Vram, VdpOp.Dma, false)
            move.l  d0, command(a1)
            lea     sizeof(DMAEntry)(a1), a1
            move.w  a1, (a2)
            andi.w  #$FFFE, (sp)                  // today: clear carry IN THE SAVED SR
            bra     .out
        .split:
            …                                     // as now; ends `bra .finish_second` or `bra .reject`
        .full:
            debug_count(DMA_Overflow_Count)       // comptime template, zero bytes in release
            bra     .reject
        .capped:
            debug_count(Dbg_DMA_Enq_Capped)
        .reject:
            ori.w   #1, (sp)                      // today: set carry in the saved SR
        .out:
        }                                         // release: move.w (sp)+, sr restores mask AND carry
        rts
}
```

What changes for the author and reader:

- **The mask is proven, not spelled.** One bracket, one release, `[context.escape]` on any path that skips it. The five hand-kept exits, and findings 3 and 4 for this routine, disappear. `engine/irq.emp:63-67` says a bracket with a carry result "can NEVER adopt"; patching the saved word is the way round, and it builds today.
- **Cost (ESTIMATED from the PUBLISHED M68000 timing tables, not run on hardware or emulator):** success exit 16 + 16 + 16 = 48 cycles (`andi.w #,(sp)`, restore, `rts`) against 52 today (restore 16, `andi #,ccr` 20, `rts` 16). Reject exits 58 against 52 (they are the rare path). About 8 bytes smaller. Behaviour equivalence is argued, not tested: it needs a headless run before landing.
- **`shared(vblank)` turns steps 3 to 5 of the walk into compile errors** when the access is outside `ints_off`, outside a `requires(vblank)` proc, and not a single instruction.
- **The header comment shrinks from 35 lines to one.** The contract line already says what the `In:`/`Out:`/`Clobbers:`/`SR:` block repeats (`:73-100`).
- **For the drain (`:358-410`):** a comptime `for k in 0..=DMA_CRITICAL_SLOTS` that emits slot `k` and drain `k`, and derives the `targets(...)` list, would remove the hand-spelled 8-wide table and the `== 8` pin. I did not check whether template-generated labels can be named from outside the template today; if not, that is the blocker.

---

## 6. Correctness

Each finding is proven by a probe built with the shipping binary, or by the cited code. "Test" is the test that should fail first.

> [!finding] 1. Sized 68000 immediates are truncated without a diagnostic
> - `move.w #$12345, d0` assembles to `303C 2345`. `move.b #$1234, d0` assembles to `103C 0034`. `cmpi.b #300, d0` assembles to `0C00 002C` (it compares against 44). All exit 0 with no message (MEASURED; probes `x1`, `x2`, `x5`).
> - Only a 32-bit range check exists on the 68000 path (`lower/code.rs:1396-1405`). The Z80 path does range-check (`lower/code.rs:2040`, `:2047`), and `moveq`/`addq` do (probes `x3`, `x4`).
> - Impact: a constant that grows past its operand size (a byte cap, a slot count, a timer) changes behaviour silently. `ensure` pins exist partly to catch this by hand.
> - Test: a frontend test asserting each of the three lines is an error. Rule: the value must fit the operand size as signed or unsigned. Risk: some corpus line may rely on truncation; the first run of the check will list them (not measured).

> [!finding] 2. A bare number in operand position is an immediate on the 68000
> - `move.w $FFFF8000, d0` assembles to `303C 8000`, which is `move.w #$8000, d0`. `move.w 1234, d0` assembles to `303C 04D2` (MEASURED; probes `lit`, `x7`, `x10`). In Motorola syntax both are absolute memory reads.
> - A *named* `const` in the same position is diagnosed (`[operand.const-as-address]`, probe `x6`). A literal is not.
> - Impact: the reader this language is meant to win over (tenet 3: "classic 68000 assembly … the familiarity anchor") gets a different instruction from the one they typed. Not found in the corpus (grep: only Z80 `ld a, $80` forms, where bare immediates are standard).
> - Test: `move.w $FFFF8000, d0` in a 68000 section must be an error telling the author to write `#` or `(…).w`.

> [!finding] 3. `preserves(sr.mask)` on a hand-spelled mask checks text order, not paths
> - The check is: a `move.w sr,-(sp)` appears before the first SR write, and the textually last SR write is `move.w (sp)+,sr` (`lower/proc.rs:2916-2941`; its doc says "Static order only — no path analysis", `:2528`).
> - Probe `d_mask_leak.emp`: one exit does `addq.l #2, sp` and returns at IPL 7. It declares `preserves(sr.mask)` and builds (MEASURED). Control: removing the save entirely does fire `[proc.preserves-sr-unbalanced]` (probe `f_ctl`).
> - Reach: 8 hand-spelled mask sites: `dma_queue.emp:127`, `:549`; `sound_api.emp:144`, `:416`; `section.emp:323`; `bg.emp:141`; `boot.emp:298`; `release_fault.emp:58` (MEASURED by grep; the last two are boot and terminal). Against 8 `with` brackets.
> - Fix options: run the SR save/restore through the existing symbolic stack (the slot for SR is already a tracked stack slot), or remove the need by letting multi-exit sites use the bracket (the saved-SR patch above; or an explicit `release` statement).
> - Test: probe `d` as a must-fail fixture.

> [!finding] 4. A flag result is never verified in the proc that declares it
> - `out(carry: name)` and `out(zero: name)` drive a *caller-side* must-use check only (`flag_check.rs:1-10`). The diagnostic census of the frontend has no callee-side flag id (MEASURED by grep: `[call.flag-result-unused]`, `[proc.out-flag-invalid]`, nothing else).
> - Probes `b` (carry pinned before the SR restore, so it is overwritten) and `c` (one exit never sets it) both build (MEASURED).
> - Reach: 18 flag-out contracts in 9 files (stage 0). The DMA core's own header warns about exactly probe `b` (`dma_queue.emp:77-79`).
> - Fix: a must-define walk for the flag on every return edge, reusing the per-flag role tables `flag_check.rs` already has. An instruction that writes all of CCR (`move …, sr`, `rtr`) kills the definition; `andi/ori #, ccr` and compares define it.
> - Test: probes `b` and `c` as must-fail fixtures.

> [!finding] 5. `requires(ctx)` is checked only where the callee's name is in the instruction
> - The check walks call and tail instructions and skips any whose target is not a symbol (`corpus_contracts.rs:1214`).
> - `VInt_Level` declares `requires(vblank)` and has **no direct caller**: it is stored into `VInt_Ptr` (`boot.emp:313`, `ojz_scroll_test.emp:979`) and reached by `jsr (a0) as VBlankHandler` (`vblank.emp:84`). `VInt_Ptr` is `u32` (`ram.emp:406`); `type VBlankHandler` carries a clobber bound and no context (`vblank.emp:30`). So the requirement on the main VBlank pipeline is checked nowhere, and nothing stops a non-VBlank-safe proc being stored there.
> - Probe `h_ptr.emp` (a main-loop proc calls a `requires(vblank)` proc through `jsr (a0) as H`) builds, but `sigil emp` does not run the whole-build gate at all (see the note below), so the proof here is the code path, not the probe.
> - Fix: let a proc type carry `requires`/`grants`, let a `vars` field have a proc type, and check stores into it.
> - Test: whole-build fixture: `move.l #MainLoopOnlyProc, VInt_Ptr` must fail once `VInt_Ptr: VBlankHandler`.

> [!finding] 6. The language has no notion of interrupt-shared data
> - No declaration says a variable is touched by both the main loop and an interrupt, so no check can ask "is this access masked, in the handler, or atomic?". In the walked routine that is steps 3, 4, 5 and 8.
> - The rule is carried by prose: 241 MUST/NEVER/ordering comment lines tree-wide (stage 2 T6, MEASURED there); in `vblank.emp` alone, 360 of 553 lines are comments (MEASURED).
> - `requires(ints_off)` would already work for callees *if* their callers used the bracket (active brackets are added at the call site, `corpus_contracts.rs:1220`). `DMA_Deferrable_DropDest` cannot use it because its caller hand-spells the mask (`section.emp:323`). So finding 3's cure unlocks this one.
> - Test: fixture with `shared(vblank) X: u16`; `addq.w #1, X` in a plain proc must fail; the same inside `with ints_off` or a `requires(vblank)` proc must pass; a single `move.w X, d0` must pass.

> [!finding] 7. Cycle budgets cannot reach the code that needs them
> - `@budget` refuses any loop, any call, any tail transfer, and any entry other than the proc's first instruction (`cycle_budget.rs:27-37`, `:75`). Result: 1 use in 605 procs (stage 0), and 349 comment lines of hand-added cycles (stage 2).
> - The VBlank pipeline, the HInt handler, the sprite emit loop and every object routine are therefore unbudgetable. The requirement in section 1 ("timing checked by the build") is unmet for all but one routine.
> - Fix, in order of cost: (a) calls: charge the callee's own declared `@budget`; (b) loops: a declared trip bound, `dbf d0, .loop bound(DMA_CRITICAL_SLOTS)`, checked against a refinement type where one exists; (c) roots at `export` labels; (d) a link-time sum, `ensure(budget(VInt_Level) <= VBLANK_CYCLES)`.
> - Test: a proc with one `jbsr` to a budgeted callee and one bounded `dbf` loop must produce a number; today it produces `[cycles.opaque-call]`.

> [!finding] 8. `script` and `dispatch` cannot target aeon's object system
> - Both emit a hidden table and store a table ordinal (`ast.rs:730-736`: `WordOffsets`, `LongPtrs`). aeon's dispatcher is table-less: `Sst.code_addr` holds the code word itself.
> - The spec's own ledger says a `code_word` encoding is "REQUIRED before the first scripted-object port on real Aeon" and that "neither shipped encoding … matches a table-less dispatcher" (`SIGIL_SPEC2_LANGUAGE.md:1297`). It is not built.
> - So the 0 uses of `script` and `dispatch` (stage 0) are not neglect. The only two game-logic constructs in the language have never run on the engine, and their fit for a boss or an enemy script is untested.
> - Test: port `TestParent`/`TestChildPart` (`test_parent.emp`) to a `script` with the `code_word` encoding and require a byte-identical or cycle-equal result under `@budget`.

> [!finding] 9. Silent-failure classes the project already documents remain open
> From `aeon/docs/EMP_PITFALLS.md` (read; each was measured there, not re-run here):
> - §1 (`:16`): a nested `if` in value position yields `()` with no diagnostic at the fold.
> - §3 (`:118`): `ensure`s in a module outside the `use` closure never run; 45 to 50 such modules; an `ensure` comparing a data symbol to an integer "silently always-passes".
> - §14 (`:516`): `Struct.field(aN)` does not resolve the field name; a deleted field surfaces as "the contract closure DROPPED 1 instruction(s)".
> - ~~§7 (`:199`): `add.w dN, aM` was measured encoding as ADDX. I did not re-test whether this is fixed.~~ *(corrected in the checker round: §7 is **fixed**. Re-probed: `unsupported form: Add.W with an address-register destination is not encodable, write adda`, exit 1, MEASURED. It is not an open class.)*
>
> The document's own summary is "the failure mode is silence". For a language whose seventh tenet is "illegal states don't compile", each of these is a defect in the language, not a pitfall for the author.

> [!note] Single-file builds run a weaker checker
> `sigil emp` did not report a caller that discards a carry result (probe `best_unused.emp` builds), nor a direct main-loop call to a `requires(vblank)` proc (probe `g_with.emp` reports only the bracket errors). Those checks live in the whole-build gate (`corpus_contracts.rs`). Any future editor or LSP loop built on per-file evaluation would show green on code the build rejects.

**Edges the template asks about, for a language:**

| Edge | Result |
|---|---|
| Value overflow | Finding 1 (immediates). Displacements are range-checked (probe `x8`: "displacement out of range for (d16,An)") |
| DEBUG against release | `if DEBUG == 1 { }` gates instructions at comptime; `assert.<w>` self-gates. Sound: the two build shapes differ in instructions inside the same proc, so each shape needs its own budget; nothing does this |
| Budget exceeded | Build error where a budget exists (1 proc). Elsewhere: nothing; the frame-architecture audit finds overruns at run time |
| Interrupt races | Findings 3, 5, 6 |
| Z80 bus | Bracket pairing is proven; `cycles()` pins YM2612 write spacing (`sound_fm.emp:220-223`), 28 uses |

---

## 7. Code quality

### How the language is actually used (stage 0 and stage 2 counts, MEASURED there)

| Fact | Number |
|---|---|
| Body lines that are raw instructions | 16,046 of 16,411 (97.8%) |
| Comment-only lines | 48,841; 3.0 per instruction |
| Comment lines restating a checked contract (`In:`/`Out:`/`Clobbers:`) | 931, beside 605 checked headers |
| Hand-counted cycle comments | 349, against 1 `@budget` |
| Dated history comments | 1,033 |
| AS operand echoes (`// (Name).w`) | 367 |
| Shipped constructs with zero uses | 18 |
| Shipped constructs missing from the frozen spec | 24 |

**Reading:** the corpus is assembly with a very good contract checker and a large comptime data language beside it (4,499 comptime lines, 48% in two DSL files). The space between, where a routine's *intent* would live, is filled with prose.

### Idiom and port-residue check

Counts are stage 0's (hand-written files; `generated/` in brackets). The judgment column is mine.

| Check | Count | Judgment |
|---|---|---|
| Build-time work done at run time | 13 static-table loop candidates; 32 literals with the derivation in a comment (0) | Small. Most loops are init-time copies (stage 0). The 32 literals are real: e.g. `bg.emp:985` `lsl.l #7, d1 // m * PLANE_H_CELLS*2`. `mul_const` already exists for this |
| Layout literals | 50 raw `n(aN)`; **0** on a typed register; 14 stride literals; 11 stride shifts | Typed registers work where used (248 of 605 headers). The gap is the 357 untyped headers and registers loaded mid-body (`movea.w parent_ptr(a0), a1` has no type) |
| Hand-spelled pins | 352 `name == literal` + 102 `expr == literal` (2) | About a third of all guards. Two kinds: honest shape pins ("this code is spelled for 8 slots", `dma_queue.emp:327`) that a comptime `for` would remove, and mirrors forced by the import graph (`PBLK_ACCEL == offsetof(PlayerBlock, accel)`, `player_ground.emp:62-75`), which are a language limit |
| Hand-kept protocols | Not scripted. In the walked files: mask-before-touch (6 places), restore-then-pin-carry, VDP flip-flop clear on entry, raster-before-flush order, "caller holds the mask" | The largest quality problem; see finding 6 |
| Assembler residue | 367 AS echoes; 198 `.asm` references; 20 widths-pinned notes; 46 sized branches (0) | The echoes are noise now that `.asm` twins are gone (`dma_queue.emp:26-37` spends 12 lines explaining a deleted file). The widths-pinned notes mark a real lowering gap (link-time immediate plus relaxable destination; memory-to-memory with two symbols: `vblank.emp:142`, `:304-322`) |
| History in comments | 1,033 dated lines; comment share 60% (55%) | `vblank.emp:405-455` is 50 lines of dated narrative about an instruction that was deleted. This belongs in git and in a decision record |

### Docs against code

| Claim | Reality |
|---|---|
| Spec 2 is "FROZEN for v1"; "the entire concept inventory fits on one page" (`SIGIL_SPEC2_LANGUAGE.md:129`, tenet 1) | 24 shipped constructs are in no `SIGIL_*.md` (stage 0), including `requires`, `grants`, `interface`, `@budget`, `cycles()`, `mul_const`. The file opens with about 100 lines of amendment history inside an HTML comment |
| Tenet 3: "The instructions don't change" | Already relaxed three ways: `mul_const`/`mul_bounded` pick the sequence (32 uses), branches pick their width, templates emit instruction lines. The principle that survives is **"every emitted instruction is predictable from the source line"**, and that is the one worth keeping |
| S2-D15: no structured control flow, because "a compiler that chooses branches can never reproduce AS bytes" (`:1301`) | The AS obligation is gone (stage 1 point 5). The stated reason no longer applies; the decision should be re-made on its merits |
| Tenet 2: zero imports for a new object (game prelude) | No prelude in the build; 552 `use` lines (stage 0) |
| `irq.emp:63`: a bracket with a CCR out-contract "can NEVER adopt" | It can (probe `best.emp`) |
| Conventions §5.4: "No comments unless non-obvious" | 60% comment share |
| `vblank.emp:250`: "128,010-cycle frame" | Stage 1 uses 128,006 (PUBLISHED via frame-architecture research). A 4-cycle disagreement inside a comment; harmless, and a sign such figures should come from one constant |
| VS Code grammar "hand-maintained against the parser keyword list" | 38 parser words missing (stage 0). The grammar is the whole of editor support: no hover, no go-to-definition, no diagnostics |

### Complexity against value; do the gates test what they claim?

- **The checker is well built.** One CFG shared by six analyses, each with a stated soundness direction and a named list of what it does not prove (`context.rs:1-60`, `cycle_budget.rs:62-78`, `out_verify.rs` header). That honesty is why the holes in section 6 were quick to find: most are written down as "S2-D7, deferred".
- **Two programs for one behaviour.** Comptime cannot run the instructions it guards. `palette_dsl.emp` models `Palette_DeriveVariant` and a Python gate ties model to code under the emulator (stage 2 §5.3). `player_ground.emp:97` says the same of the slope gate and delegates to `tools/test_slope_symmetry.py`, which parses instruction *text*. Each such pair is a place where model and code can drift between gate runs.
- **Generated docs.** Should be generated from code: the per-proc register interface (replacing the 931 prose lines), the RAM map with its interrupt-sharing class, the cycle table for budgeted procs, and Spec 2's §10 inventory (from the parser's keyword and attribute tables, which would also drive the editor grammar).
- **Could another game's developer follow it?** The instruction lines, yes. The contract headers, yes; they are the best documentation in the tree. The files as they stand, no: `vblank.emp` is two thirds dated argument, and the load-bearing rules are inside it.

---

## 8. Verdict and plan

> [!verdict] Verdict: best known (one layer beyond, one layer missing)
> | Layer | Against the best common tools |
> |---|---|
> | Register, stack and bracket contracts | **Beyond.** No assembler in stage 1's survey has them; I found no error in that survey |
> | Layout, sections, comptime data | **Best known**: on a level with the best of ca65/RGBDS/KickAssembler combined, in one mechanism |
> | Timing | **On par in practice.** The cycle model exists and is sound; it covers 1 proc |
> | Interrupt and hardware-state safety | **On par** with macro assembly: half the mask sites are hand-spelled; shared data is unmodelled |
> | Game logic | **On par** with S3K/S.C.E.: code-pointer states by hand. The constructs meant to beat this do not bind to the engine |
> | Reliability of the tool's own "no" | **Below** what tenet 7 promises, until findings 1, 2, 9 are closed |
> | Editor support | **Below** common tools (highlighting only) |

**Disposition: keep, and patch.** Reason: the foundation (IR, CFG, contract analyses, comptime) is sound and is the hard part; every gap found is an extension of an analysis that already exists, not a redesign. Nothing here calls for *rewrite beside*.

### The direction question

| Option | Answer | Why |
|---|---|---|
| (a) Polish the exact design only | No | It leaves protocols in comments, budgets on 1 proc, and game logic as bare assembly |
| **(b) Exact + checks + sugar with a fixed, documented expansion** | **Yes, now** | Every finding in section 6 is cured inside it. `mul_const` is the precedent: the compiler picks, the result is predictable and priced |
| (c) A compiled functional tier for game logic | **Not yet; run it as an experiment after (b)** | See below |
| (d) Functional everywhere | **No** | Table below |

**Why (c) waits.**
1. **There is nothing to design it against.** `games/sonic4/objects` is 15 files: 11 `test_*`, two dust effects, a ring sparkle and Tails' tails (MEASURED). No enemy, no boss, no projectile pattern. The player code is the only real logic, and it is port-shaped.
2. **Its promised cost is an estimate about a compiler that does not exist.** Stage 2's "1.0 to 1.25 times" is ESTIMATED; the numbers for a real compiler (GCC) are 1.3 to 2.8 times on dispatch and scripts (MEASURED codegen bytes; cycles are from sigil's table in stage 1 and ESTIMATED by hand count in stage 2) *(label corrected in the checker round)*. The difference is the calling convention and in-place mutation, which is exactly the contract layer (b) builds out.
3. **Stripped of comments, exact logic is already short.** `TestEmitter_Main` is 6 instructions (`test_emitter.emp`) *(corrected in the checker round: was "7")*. The functional sketch's real gains (named states, a hidden state made explicit, typed records, no mirrored facing code) come from `script`, sum-typed state, typed links and templates. All are (b).
4. **It costs a second language**: a register allocator, a new debugger mapping in Oracle, and the loss of "what I wrote is what runs" in the place newcomers start.

**What would change this answer:** once the boss and the enemy script exist in the patched language, if they are more than about twice the size of the functional sketch, or show bug classes the contracts cannot see, build the tier. Its entry conditions are then testable: register-contract calling convention, guaranteed in-place slot mutation, every proc under `@budget`, and within 1.15 times of the hand version (MEASURED by the same walker).

### Functional everywhere, area by area

Ratios are stage 1 (W-numbers, MEASURED codegen through sigil's cycle table) and stage 2 (MEASURED bytes, ESTIMATED cycles). No new measurement here.

| Area | What a compiler cannot match | Cost against today | Escape hatch it would need | Answer |
|---|---|---|---|---|
| HBlank handlers | Register pinning across interrupts, port held in an address register, `rte`-as-release (`raster.emp:1195`), blanking spins solved to the cycle | 2.6 to 5.1 times (W8); 46% of a scan line for one VSRAM write | The whole handler | Exact |
| VBlank pipeline and DMA drain | Unrolled jump-into table at 72 cycles per entry; ordering rules tied to VDP state | 1.5 to 2.3 times on VDP copy loops (W6); the window is 82% used in release, 97% in DEBUG | The whole pipeline | Exact |
| Z80 sound driver (81% of cycle-critical lines) | T-state spacing between YM2612 writes (`cycles()` guards), balanced paths; no Z80 compiler models T-states | Not measured (stage 2: DID NOT RUN) | The whole driver | Exact *(now based on a reading of the driver: see "Checker round", gap 2)* |
| Decompression, tile streaming | `(An)+` streaming, `dbf`, bit-buffer tricks | About 1.1 to 1.5 times (ESTIMATED, stage 1); these paths already cause lag frames | Inner loops | Exact |
| Sprite emit, object core | Fused `dbeq`, pad-byte layout trick | 1.07 to 1.77 times per piece (stage 2) | Inner loops | Exact |
| Player and object logic (10 to 14% of code) | In-place field updates, flag results, tail calls: recoverable with the right ABI | 1.3 to 1.8 times with GCC; unknown with a purpose-built tier | Per-routine `asm` blocks | **(b) now; (c) as an experiment** |
| Comptime tables and DSLs | n/a: already functional | 0 | n/a | Keep |

So "functional everywhere with an inline-asm escape hatch" would put the escape hatch around up to 47% of all code lines (cycle-critical 14.0% + tight loops 1.9% + engine systems 24.1% + object core 6.9%, stage 0 §5; an upper bound, since not every engine-systems proc is hot) and deliver its benefit to the 10 to 14% that is game logic. That is option (b) with a harder migration.

### Ranked improvements

| # | Improvement | Gain | Cost | Risk | Depends on | Byte-changing |
|---|---|---|---|---|---|---|
| 1 | Close the silent holes: immediate range check (F1), bare-literal operand error (F2), nested-`if` unit, unresolved `Struct.field` (F9) | Removes four ways to ship a wrong ROM from a green build | Small each | F1 may flag existing lines; that list is the point | none | No (unless a truncating line exists) |
| 2 | Callee-side flag-result proof (F4) and path-sensitive SR proof (F3) | The two contracts every masked, flag-returning routine relies on become true | Medium; reuses the flag tables and the symbolic stack | Low | none | No |
| 3 | Let multi-exit masked routines use the bracket: adopt the saved-SR carry patch in `QueueDMA_*` and `DMA_Deferrable_DestPending`; correct `irq.emp:63` | Two of eight hand-spelled sites become proven; about 4 cycles off every successful enqueue (ESTIMATED) | Small | Needs a headless regression run | none | **Yes** (two routines) |
| 4 | Interrupt-shared data: `shared(ctx)` on `vars`, proc types that carry `requires`, typed handler slots (`VInt_Ptr`, the HBlank slot) (F5, F6) | The engine's worst bug class (torn state between main loop and VBlank) becomes a build error. No assembler does this | Medium to large | False positives until atomic single-instruction access is modelled | 2 | No |
| 5 | Composable budgets: callee budgets, loop bounds, `export` roots, link-time sums against the VBlank and scan-line constants (F7) | Replaces 349 hand-count comment lines; turns frame-architecture's run-time overruns into build errors; is also the acceptance gate any compiled tier needs | Medium | Bounds must be honest; tie them to refinement types | none | No |
| 6 | Bind `script`/`dispatch` to aeon: the `code_word` encoding, proc-name-as-value (replacing `extern("X") - extern("ObjCodeBase")`), typed `ref<Sst>` links, `table` sentinel for spawn lists (F8) | Game logic gets named states, `wait_frames`, typed parent/child links: what the boss and shmup need | Medium | Design against a real object, not the test objects | none | No for new code |
| 7 | **Write the named workload** in the result: one multi-part boss, one enemy script, with `@budget` on each routine | The first real evidence for or against a compiled tier | Small once 5 and 6 exist | none | 5, 6 | New code |
| 8 | Fixed-expansion flow sugar: `if <cc> { }` / `else` over one branch each; the author still writes the compare. Re-decide S2-D15 | Removes most of the 2,111 label lines and one-use label names; keeps one-line-to-one-instruction | Small to medium | Syntax debate; keep it to forms with exactly one possible lowering | none | No |
| 9 | Comptime execution of procs: run a 68000/Z80 proc inside `comptime test` on the suite's own CPU core | One program instead of model + code + Python gate (palette, slope gate); procs become unit-testable at build time | Medium to large; `SIGIL_ORACLE_ISA_SHARING.md` already plans a shared ISA crate | Build time; memory-model design | Oracle core as a library | No |
| 10 | Comment diet with tooling: generate the interface block from the contract; move dated history to git; lint contract-restating comments and AS echoes | The files become readable by a second developer; 60% comment share falls sharply | Small tooling, large mechanical edit | Losing real rationale: move it to decision records, do not delete | none | No |
| 11 | Spec and editor from one source: generate §10 and the TextMate grammar from the parser tables; fold the 24 unspecified constructs into Spec 2; then an LSP (hover = contract + cycles) | Ends spec drift; first real editor support | Medium (LSP large) | none | none | No |
| 12 | Experiment: compiled expression/state tier for game logic, gated as described above | Possibly large for game authors | Large | High: a second language | 5, 6, 7 | New code only |

### Beyond-state-of-the-art candidates

| Candidate | Plausibility |
|---|---|
| **Build-time frame budget proof**: `ensure(budget(VInt_Level) <= VBLANK_CYCLES - margin)` per build shape, over composed worst-case budgets | High. The cycle tables and CFG exist; it needs improvement 5. I know of no Genesis toolchain that proves its VBlank fits |
| **Interrupt-race checking for assembly** (improvement 4) | High for the declared-variable form; whole-program inference is a research project and not needed |
| **Proc-level tests executed at build time on the emulator's CPU core** (improvement 9) | Medium to high |
| **Typed VDP port state** (flip-flop, autoincrement) as a context like `z80_stopped` | Medium. The declared-context machinery fits a two-state latch; the spec lists write-sequence tracking as out of scope for needing value analysis |

### Dropped ideas

| Idea | Why dropped |
|---|---|
| A Haskell-style `.emp2` now | No workload to design against; cost unmeasured; most of its readability comes from constructs that are (b) |
| Functional everywhere | Per-area table |
| C through GCC/SGDK for logic | Stack ABI and promotion: 1.3 to 2.8 times (stage 1: MEASURED codegen through sigil's cycle table, GCC 15; stage 2: MEASURED bytes, **ESTIMATED** cycles, GCC 13.3; the two are not comparable figure for figure) *(label corrected in the checker round)*; no flag results; no contracts |
| `while`/`switch` blocks where the compiler chooses the compare or the table shape | Breaks "predictable from the source line". `dispatch` and `script` cover the table cases explicitly |
| Rewriting the checker around one unified dataflow framework | It already shares one CFG; the separate lattices are cheap and each documents its own soundness |

---

## Coverage table

"Read" means the lines were on screen and considered. Whole-corpus greps are listed separately and are not "read".

| File or crate | Lines read | Result |
|---|---|---|
| `emp-language/0-inventory.md` | 1-330 of 1,085 (the rest is the per-file table and scripts; rows for 4 files looked up) | used; challenges 1, 2 |
| `emp-language/1-research.md` | all | used; challenge 3 |
| `emp-language/2-measurements.md` | all | used; challenges 4, 5 |
| `AUDIT_TEMPLATE.md`, `Audit board.md` | all | followed |
| `Audit process review.md` | about the first 60% (150 non-blank lines; long table cells cut at 420 characters) | read; followed its reviewer rules (sections 1, 6, 7, 8, coverage table, mandatory walk) |
| `SIGIL_SPEC2_LANGUAGE.md` (1,310) | 1-150, 209-300, 827-980, 1117-1136, ledger rows S2-D4/7/9/10/12/15 | findings 8, 10 (section 7 docs table) |
| `SIGIL_IDEAS_FUNCTIONAL_TIER.md` | all | section 8 |
| `SIGIL_ORACLE_ISA_SHARING.md` | 1-40 + headings | improvement 9 |
| `SIGIL_CORE_SPEC.md`, `SIGIL_M0_CATALOG.md`, `SIGIL_M0_DESIGN.md`, `SIGIL_PLAN_AUDIT_2026-07-01.md`, `SIGIL_AEON_COMPAT_NOTES.md` | headings only | not read; assembler-core scope |
| `aeon/docs/EMP_PITFALLS.md` (555) | 1-75, 118-210, 516-555; headings of the rest | finding 9 |
| `sigil/docs/EMP_PITFALLS_EQUALITY.md` (220) | 1-40 | read, nothing new (it is §12 of the above) |
| `aeon/CLAUDE.md` | all | section 7 docs table |
| `aeon/CODING_CONVENTIONS.md` | headings only | one claim (§5.4) used via stage 0 |
| other `aeon/docs/*` | not read | — |
| `engine/system/dma_queue.emp` | all 657 | Walk A; findings 3, 4, 6, 7 |
| `engine/system/vblank.emp` | all 553 | Walk A; findings 5, 6 |
| `engine/irq.emp` | all 96 | findings 3, 6; the "can NEVER adopt" correction |
| `engine/z80_bus.emp`, `engine/system/hblank.emp` | code lines only (comments skipped) | read, nothing |
| `engine/effects/raster.emp` | 1150-1235 of 2,426 | read, nothing (good use of `ints_off_until_rte`) |
| `engine/sound/sound_fm.emp` | 195-232 | read, nothing (good use of `cycles()`) |
| `engine/objects/core.emp` | 225-262 of 803 | finding 8 context |
| `games/sonic4/objects/test_parent.emp` | 60-214 of 214 | section 1; finding 8 |
| `games/sonic4/objects/test_emitter.emp` | code lines | section 8 |
| `games/sonic4/player/player_ground.emp` | 60-130 | section 7 |
| `engine/level/scene_dsl.emp` | 46 code lines | read, nothing |
| Rest of the corpus (about 185 hand files) and `generated/` (14 files) | **not read**; touched only by greps for mask sites, `requires`/`grants`, `DMA_*_Slot`, `VInt_Ptr`, bare-literal operands | relied on stage 0 and 2 counts |
| `sigil-frontend-emp/src/context.rs` | 1-120, 300-420 of 525 | finding 6 |
| `…/cycle_budget.rs` | 1-110 of 2,002 | finding 7 |
| `…/preserves.rs`, `flag_check.rs`, `out_verify.rs`, `closure.rs` | first 50-75 lines each | findings 3, 4 |
| `…/corpus_contracts.rs` | 1150-1262 of 2,657 | finding 5 |
| `…/lower/proc.rs` | 2470-2610, 2916-2941 of 3,384 | finding 3 |
| `…/lower/code.rs` | 1380-1412; grep for range checks | finding 1 |
| `…/parser.rs` | operand parser comments 3385-3600 (filtered); not the rest | finding 2 |
| `…/ast.rs` | 730-738 | finding 8 |
| Rest of the crate (about 50 files, about 50k lines) and `tests/` | not read; test suite **run** (2,784 passed, 0 failed, 0 ignored) | — |
| `sigil/editors/vscode` | `README.md`, `package.json`, `language-configuration.json` all; grammar first 60 lines | section 7 (stage 0's finding confirmed, nothing new) |

## Challenges to stages 0 to 2

> [!note] Challenges
> 1. **Stage 0 §1b says `preserves(...)` is "verified by symbolic stack".** True for the register file. For `sr` / `sr.mask` on a hand-spelled bracket the check is textual order only (`lower/proc.rs:2528`, `:2916-2941`), and probe `d_mask_leak.emp` passes. The row should split the two.
> 2. **Stage 0 lists `out(carry: name)` as SHIPPED without qualification, and CCR liveness as PARTIAL.** The carry/zero result is checked at callers only; probes `b` and `c` show the declaring proc is unchecked. That belongs in the PARTIAL description by name.
> 3. **Stage 1 says "`.emp`'s `script` is already this construct. It ships with 0 aeon uses", and ranks coroutines and typed dispatch as "already built" cheap wins.** Neither can target aeon: the `code_word` encoding the spec calls REQUIRED for aeon is not built (`SIGIL_SPEC2_LANGUAGE.md:1297`; `ast.rs:730-736`). The zero is a missing feature, not unused capacity, and the win is not yet cheap.
> 4. **Stage 2 presents `TestParent_Main` / `TestChildPart_Main` as the game-logic sample.** They are DEBUG-only test objects (stage 2 says so), and the corpus has no enemy or boss at all (15 files, listed in section 8). Any ratio drawn from them describes a 72-byte timer routine, not boss logic. The "functional tier 1.0 to 1.25 times" column is an estimate about a compiler nobody has written and should not be read beside the MEASURED GCC column as if comparable.
> 5. **Stage 2 §0: "`@budget` refuses loops and calls", so sigil's model "could not be used"; stage 1 used `@budget` for all its numbers.** Both are true (stage 1 measured straight-line bodies and single iterations), but the two stages therefore use different cycle sources (sigil's table against hand counts from the manual) for overlapping claims. The lead should not mix their ratios without saying so.
> 6. **Stage 0 C3 lists three exceptions to tenet 3 and stops.** Add: a bare numeric operand is assembled as an immediate, and sized immediates are truncated (findings 1, 2). Those are places where the instruction in the ROM is *not* the instruction on the line.
> 7. **Not a challenge, a limit of mine:** the `[context.unsatisfied]` behaviour in finding 5 is from reading `corpus_contracts.rs`; I did not run a whole-tree `sigil build` with a planted pointer call. The controller can confirm with one fixture.

---

## Checker round

> [!verdict] Result of the follow-up round
> - **Verdict and direction are unchanged**: best known as an exact-instruction language; keep and patch; option (b) now, (c) as a later experiment, (d) no.
> - **The checker was right on all five gaps.** Three analysis families do not gate the build (now MEASURED with a planted defect, finding 10). Pitfall §7 is fixed. Two labels and one count were wrong and are corrected inline above, each marked.
> - **Two new silent holes turned up while examining the unread pitfalls** (findings 11 and 12, both MEASURED with the shipping binary): a `comptime fn` resolves its free names in the *caller's* module, and a struct's layout is computed in the *importer's* module. In both, a same-named constant in the importing file silently changes the result.
> - **The Z80 driver was read.** The "exact" answer for the Z80 now rests on the code: its hot loop is correct only because every path costs exactly 195 T-states, and the language proves that at build time. It is also the best existing example of option (b).
> - **`shared(ctx)` does not cover the 68k-to-Z80 mailbox race.** That race is a protocol error between two correctly held brackets. It is curable with a feature the language already has (a must-use carry result on the post routine), which is cheaper than a new construct.

Trees unchanged: aeon `936439fa`, sigil `7630ba0d` (binary `sigil 0.1.0 (7630ba0d)`). All new probes and outputs are under `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/audit-builds/reviewer-a-r2/` (`probes/` for single-file and `--root` probes; `aeon/` is a scratch copy of the aeon tree used for whole-build fixtures). No repo was written.

### Gap 1. Lens S12-c: three families are computed and not gated

> [!finding] 10. `[bus.*]`, `[branch.condition-constant]` and `[call.slot-type-mismatch]` do not stop `sigil build`
> - **Code:** `run_contract_gate` (`sigil-cli/src/main.rs:2153-2375`) never reads `report.bus_firings`, `report.branch_const_firings` or `report.slot_firings`. The only readers in `main.rs` are the report printer (`:2524-2575`).
> - **Fixture (MEASURED).** In a scratch copy of aeon whose clean build is byte-identical to the repo's `s4.bin` (crc `193507d9`, 850,296 bytes), I planted two defects, one at a time:
>
>   | Planted | `--report contracts` | `sigil build` |
>   |---|---|---|
>   | A proc that stops the bus, starts it, writes `VDP_CTRL`, starts it again, then `moveq #0, d0` / `beq` | `[bus.vdp-write-unstopped]` 1, `[bus.start-without-stop]` 1, `[branch.condition-constant]` 1 | **exit 0**, ROM written, no line about any of the three in the build output |
>   | `as SfxId` stripped from `Player_Jump` (`player_ground.emp:1110`) | `[call.slot-type-mismatch]` 1 | **exit 0**, ROM byte-identical to the clean build |
>
>   Clean tree: all three families are at 0 firings (MEASURED), so adding them to the zero-firing list would cost nothing today.
> - **Other gates:** `[call.slot-type-mismatch]` has a corpus test (`sigil-cli/tests/slot_type_corpus.rs`) that fails in CI. The bus and branch families have unit tests only; nothing asserts they are empty on the corpus (grep over `sigil-cli/tests` and `sigil-harness`: no reader).
> - **Why it matters more than it looks.** `z80_bus.rs` calls itself "the sigil-native absorption of `s4lint.py`'s E006/E007/E008/E011 crash-class checks". `aeon/build.sh:964-971` retired s4lint on 2026-09-07 and says "nothing mechanical states the E001/E002/E006/E007/W021/W024 invariants for `.emp`". So a crash-class check was removed from the build and its replacement only prints. The project knows: the row `BUS-NET-BUILD-GATE-ROW` in `sigil/docs/superpowers/notes/campaign-gap-ledger.md` (dated 2026-09-07) says "reported, not gated". It is still open on `7630ba0d`.
> - **Impact on a typed-ID design.** `as SfxId` and every newtype on a call slot are advisory unless CI runs. A developer using only `./build.sh` gets a green build for a wrong ID type.
> - **Fix:** three `empty_gate` rows in `run_contract_gate`. **Test:** the two fixtures above as must-fail builds.

**Corrections that follow from this:**
- Walk A step 11 said `[bus.vdp-write-unstopped]` "fires". Corrected inline: it is printed, not gated. What *is* gated for that step is the declared bracket (`[context.escape]`, `[context.reacquire]`, in the zero-firing list at `main.rs:2247`).
- Ranked improvement 1 ("close the silent holes") gains this item. It is the cheapest entry in the table.
- Section 8's row "Reliability of the tool's own 'no'": still **below**, with one more reason.

Timing context for the fixture builds: `sigil build` of sonic4 plain, 3.3 s wall clock, release-profile `sigil`, one run, scratch tree on tmpfs, same host as stage 0.

### Gap 2. The Z80 tier, read

**What I read.** `engine/sound/z80_sound_driver.emp`: every code line from 106 to 775 (comments stripped), plus the comment blocks at 410-426, 483-500, 551-612, 790-830 and the whole `SndDrv_TimerATick` header and its first 12 instructions. `sound_fm.emp`: all 23 proc headers (and 195-232 from the first round). `sound_sequencer.emp`, `sound_sfx.emp`, `sound_psg.emp`: **counted by grep only, not read**. Checker modules: `z80_cycles.rs` 1-80 of 1,499; `z80_preserves.rs` 1-70 of 712; `z80_out_verify.rs` 1-40 of 458; `z80_bus.rs` 1-60 of 275.

**Counts over the five Z80 code files (MEASURED by grep, comment lines excluded where it matters):**

| Fact | Count |
|---|---|
| Procs | 146 |
| With `clobbers` | 146 |
| With `preserves` | 99 |
| With an `out(...)` | 30 lines; the build tracks 41 Z80 out claims, **6 unverified and baselined** (`Sfx_MusicChanPtr`, `Sfx_ResolveBlob` ×2, `Sfx_SelectVoice`, `Snd_DacLookup` ×2) |
| With typed input parameters | **2** (`Fm_TransposeClampChrom`, `Fm_TransposeClamp`); inputs elsewhere are prose: 67 `In:` comment lines |
| `cycles()` uses / `pad_to_cycles` / `@budget` | 42 / 2 / **0** |
| `with` brackets / `requires` / `grants` | 0 / 0 / 0 |
| `jr`/`djnz` instructions (refused inside a `cycles()` span) | 331 |
| Lines mentioning "bank-safe" or "RESIDENT" (nearly all comments) | 63 |

**One path walked: the DAC streaming loop** (`SndDrv_Sample`, `:420-634`) and the tick that interrupts it.

| Step | What happens | What the language proves | What is by hand |
|---|---|---|---|
| Loop top `:438-450` | Emit one ring byte to the DAC, poll Timer A | The pass is exactly 195 T: `ensure(cycles(.loop, .exhaust) == 195)` (`:613`) | Nothing |
| FILL `:462-475` | Copy one ROM byte into the ring through the bank window | Same ensure | That the `$8000` window is on the *sample* bank here. 63 comment lines carry the bank rule; no construct |
| DRAIN `:551-605` | 68k DMA in flight: consume only, with a branchless underrun guard | `pad_to_cycles(195, …, dense: true)` **derives** the pad from the measured cost; the ensure at `:621` re-measures it. `ensure((SND_Z80_YM_A1 & $FF) == 1)` pins the `cp e` trick's constant | That `e` really holds `$01` for the whole sample (the constant is pinned; the register's content is not). A 30-line comment still restates the arithmetic |
| DRAINING tail `:489-501` | Producer exhausted; 194 T | Derived pad + ensure (`:623`) | — |
| Deferred tick (the "third path") | `SndDrv_TimerATick` returns at once while DMA is up: +27 T | **Nothing**: the driver's own comment says "a THIRD path they do not measure" | The +27 T figure, hand-counted |
| Tick proper | Spill `b, c, ix, hl'` to RAM, run the sequencer frame, poll the mailbox, reload, refill | `clobbers(af, bc, de, hl, ix, iy)`, which is the maximum and so constrains nothing | Spill and reload match the loop's register map. "The tail-call does NOT re-park `$2A` or restore `de=$4001` — CALLERS must." The tick's worst-case length is unbounded by any check (it has loops and calls, which `@budget` refuses) |
| YM2612 writes `:253-272`, `:520-538` | Address write, then data write | `ensure(cycles(.x_addr, .x_data) >= YM_ADDR_TO_DATA_MIN_T)` at every pair | Nothing |

**What this says about the direction question.**
- **"Exact" for the Z80 is now a read conclusion, not an assumption.** The loop's correctness condition is *path-cost equality to the T-state*, plus state held in registers (including the shadow set) across a polled tick, plus tricks such as comparing against the low byte of a port address because `cp e` is 4 T and `cp 1` is 7. No compiler offers "these three paths cost the same". The cost ratio of a compiled Z80 tier remains **not measured** (stage 2: DID NOT RUN); the row is "exact" on expressibility, not on a measured ratio.
- **The Z80 loop is option (b) already working.** `pad_to_cycles` is sugar with a fixed, documented expansion (`jr` 12 T and `nop` 4 T units), chosen by the build from a measured cost, and re-checked by an `ensure`. It replaced a hand-counted pad. That is the pattern I recommend carrying to the 68000 side.
- **The same gaps as on the 68000, with one more.** Budgets do not reach looping code (finding 7): 0 `@budget` on the Z80, and the tick has no bound. Inputs are prose in 144 of 146 procs, so `[call.input-undefined]` and slot typing have almost nothing to check; `Snd_AckSlot`'s "`hl` is DEAD at all eight clear-sites" is a liveness fact kept by comment. New here: **the ROM bank window is machine state with a bracket shape and no construct.** `Run_SeqFrame_OnSongBank` is called "B1 bracket" in its own comment. A declared context (`with song_bank { }`, `requires(sample_bank)`) is the existing `z80_stopped` mechanism applied to a second latch. I did not check whether declared contexts are implemented for `cpu: z80` modules; there are 0 uses.
- **6 of 41 Z80 out claims are baselined as unverified** (15%, MEASURED from `--report contracts`). The Z80 out proof is real but not complete on this corpus.

> [!note] Limit
> `sound_sequencer.emp`, `sound_sfx.emp` and `sound_psg.emp` (4,827 lines, 2,500 code lines) were counted, not read. Nothing above is a claim about how readable or how port-shaped the sequencer is.

### Gap 3. Finding 9 corrected; the unread pitfalls examined

§7 is fixed (corrected inline). The remaining sections:

| Pitfall | Status | Result |
|---|---|---|
| §2 comptime helper imports resolve at the call site | **Examined, probed** | Finding 11. The documented *loud* shapes are loud (`unresolved name LIMIT, add use k.{LIMIT}`; `range bounds must be int, got int and label`). A *silent* shape exists and is worse than the document says |
| §5 `extern()` poisons comptime-ness | **Read, not probed** | Still described as a live rule in the corpus (`[here.provisional]` errors). I did reproduce the related refusal: a link-time label steering a comptime `if` fails loudly with `[here.provisional]` (probes `la`-`ld`). The "error points at an unrelated file" claim is **unexamined** |
| §8 a struct is re-elaborated in each importing module | **Examined, probed** | Finding 12. Reproduced as documented (error blames the declaring file), and a silent variant found |
| §9 `Game.*` absent in layout and harvest contexts | **Read, not probed** | Needs the build harness's harvest passes; **unexamined by measurement**. By reading, it is the same root as §2 and §8: there is more than one name environment for one declaration |
| EQUALITY §12: label beside an int stays *defined* `false` | **Examined, partly** | Not reproduced in the shapes I built. In an `ensure`, an own-module label against an int is decided **at link by its address** (13 probes `ea`-`ed`, `fa`-`ff`, `gb`, `gc`, `ge`: e.g. `ensure(l == 4)` passes for a proc at address 4 and `ensure(l == 5)` fires). Combined with `||` or `&&` it is refused (`[here.provisional] a bool value cannot combine with a link-time here() value`). The documented always-`false` answer for a label in a `[Label; N]` slot of an *imported* `pub data` symbol was **not built**; that shape stays unexamined |

> [!finding] 11. A `comptime fn` is dynamically scoped: the caller's names silently replace the helper's own
> - `helper.emp` imports `LIMIT = 5` from module `k` and defines `pub comptime fn lim()` returning `LIMIT` and `count_to_limit()` looping `0..LIMIT`. `main.emp` imports both functions and has its own `const LIMIT = 9`. Result: `moveq #count_to_limit(), d0` / `moveq #lim(), d1` assemble to `70 09 72 09`. Both return **9**, exit 0, no diagnostic (MEASURED; probe `p2d`).
> - The same for functions: `helper.emp` has a private `twice(x) = x * 2` and `pub comptime fn four() = twice(2)`. `main.emp` has its own `twice(x) = x * 3`. `four()` assembles to `70 06`: **6** (MEASURED; probe `p2e`).
> - Control: with no `LIMIT` at the caller the build fails loudly; with `use k.{LIMIT}` *at the caller* it returns 5 (probes `p2`, `p2b`, `p2c`). So a helper's result is a property of each caller's namespace.
> - This is `EMP_PITFALLS.md` §2, which records the near-miss form. The shadowing form is not in the document. Corpus exposure was **not measured**: I did not search for helper free names that collide with a caller's local names. The corpus already bends around the rule: `engine/objects/dplc.emp:58-60`, `:111-115` and `:196-198` hold literals in function bodies and repeat a helper three times because "a private comptime fn is not visible at the CALLER's expansion site".
> - **Why it matters for the direction question.** The comptime layer is the language's existing functional tier (4,499 lines, two DSLs). Lexical scope is the first property a functional language needs: a function must mean the same thing wherever it is called. Any higher tier built on this evaluator inherits the defect. Fix it before, not after.
> - **Fix:** resolve a `comptime fn`'s free names in its defining module (capture the module environment at definition). **Test:** probes `p2d` and `p2e` must give 5 and 4. Risk: corpus code that relies on call-site resolution (a helper meant to read the caller's constant) will change meaning; the first run should list every free name whose two resolutions differ.

> [!finding] 12. A struct's layout is computed in the importing module's namespace
> - `a.emp`: `pub const N = 4` and `pub struct S { x: [u8; N], y: u16 }` (6 bytes). `main.emp`: `use a.{S}` and its own `const N = 10`. `moveq #sizeof(S), d0` assembles to `70 0C`: **12**, exit 0, no diagnostic (MEASURED; probe `p8b`).
> - Without the caller's `N` the build fails with `a.emp:4:13: error: expected an integer, got label`, blaming the declaring file (probe `p8`), exactly as §8 documents.
> - Impact: two modules can disagree about one struct's size and field offsets with a green build. For a language whose strongest claim is that data layout is a compile-time fact (section 1, mega-acts row), this is the most serious hole in this audit. Corpus exposure **not measured**.
> - Same root cause as finding 11, and the same fix: elaborate a declaration once, in its defining module. **Test:** probe `p8b` must give 6; probe `p8` must build.

Findings 11 and 12 join ranked improvement 1. They do not need new language surface; they need one name environment per declaration.

### Gap 4. Finding 1 linked to its class; labels fixed

- **Class.** Finding 1 (sized 68000 immediates truncate) is a member of the lens-sweep class `sig-guard-consumer-class` ("checked in one place, used somewhere that never sees the check") and a sibling of `sig-dcw-dcl-truncate` (fixed at merge `81f05a29`; six further sites fixed at `0a729aaf`, note `sigil/docs/superpowers/notes/2026-09-07-check-then-use.md`).
- **It is not in the residual row.** `CHECK-THEN-USE-RESIDUAL` (`campaign-gap-ledger.md:4520`) lists four sites: `.emp` `pub equ` lowering through `Expr::Int(n as i64)` (`lower/mod.rs`, three places), `code.rs` spelling the `abs.w` window itself, `BankPtr16*` (a documented exception), and link-time `>> n` for n in 64..128. The `.emp` 68000 sized-immediate path is in neither the fixed six nor the residual four. It is an **unbooked member of a class the project believes it has enumerated**.
- **The same binary's AS frontend refuses these lines** (MEASURED; probe `imm.asm`): `operand 74565 out of range -32768..=65535`, `operand 4660 out of range -128..=255`, `operand 300 out of range -128..=255`. The `.emp` frontend accepts all three (`30 3C 23 45`, `0C 00 00 2C`, re-run this round). So the byte-identical port moved the corpus from a frontend that checks immediates to one that does not.
- **Label fix.** "1.3 to 2.8 times (MEASURED, stages 1 and 2)" is corrected inline in both places it appeared: stage 1 is MEASURED codegen costed through sigil's cycle table (GCC 15); stage 2 is MEASURED bytes with **ESTIMATED** cycles (GCC 13.3). The two ratios are not comparable figure for figure.
- **Count fix.** `TestEmitter_Main` has 6 instructions, corrected inline in sections 1 and 8. The point it supported is slightly stronger.

### Gap 5. Does `shared(ctx)` cover the 68k-to-Z80 mailbox race?

**No.** It covers the atomicity half, which is already correct, and not the protocol half, which is the bug.

- **The race** (engine-systems review 2026-11-13, defect 6): the Z80 reads a request slot, acts, then clears it (`Snd_AckSlot`, `z80_sound_driver.emp:810`: `ld (hl), 0`). The 68k posts fade, tempo, pause and jingle requests unconditionally (`Sound_FadeCmd` and siblings tail-call `Sound_PostByte`, `sound_api.emp:109-116`). A post that lands between the Z80's read and its clear is erased; a post onto an unconsumed slot overwrites it.
- **Why `shared(ctx)` misses it.** Every 68k access is already inside `with ints_off { with z80_stopped { } }`. Each access is atomic; the declared-holder rule would pass. The error is that the *sequence* "slot is empty → write" is not enforced on the 68k side. The Z80 cannot mask a bus request, so the consumer cannot be made atomic; the producer must test before it writes. `Sound_PlayMusic` does (`.await_slot`, `:263-277`). The others do not, and the rule is a comment: "One transport op per frame is the 68k-side contract … don't" (`sound_api.emp:578-581`).
- **What does cover it, with today's language.** Make the post routine test-and-post inside its one bus hold and return `out(carry: busy)`. The existing caller-side must-use check (`[call.flag-result-unused]`, gated) then forces every caller to handle a busy slot. The slots are already private `equ`s in `sound_api.emp`, so no other module can write them. This is a library change, byte-changing, and needs no new construct.
- **What stays outside my proposal.** Cross-CPU shared data as a language concept:
  - One declaration for one cell. Today a slot is a Z80-side `const` offset plus a hand-written 68k-side `equ X_SLOT = SND_Z80_BASE + SND_REQ_X` (`sound_api.emp:57-65`; `vblank.emp:19` for the DMA flag). Nothing ties the two or says which CPU may write.
  - Direction and protocol (single writer; "post only if empty"; "consumer clears after acting").
  - The DMA-active flag (`SND_CTRL_DMA_ACTIVE`: raised and lowered by the 68k in `vblank.emp` and `section.emp`, polled by the Z80) is the same kind of cell and has documented residual races in the driver's own comments (`SndDrv_TimerATick` header: "a window-narrowing guard, not a mutual exclusion").
  
  I list this as **out of scope for improvement 4** and as a candidate extension: a `mailbox`/`shared(z80)` declaration that emits both addresses and makes the checked post routine the only writer. Plausibility medium; it should wait until improvement 4 exists on one CPU.

### Coverage added this round

| File or crate | Lines read | Result |
|---|---|---|
| `emp-language/5-checker.md` | all 123 | the five gaps |
| `sigil-cli/src/main.rs` | 2153-2375, 2440-2600 of about 3,100 | finding 10 |
| `sigil-frontend-emp/src/z80_bus.rs` | 1-60 of 275 | finding 10 |
| `…/branch_const.rs` | 1-50 | finding 10 |
| `…/z80_cycles.rs` | 1-80 of 1,499 | gap 2 |
| `…/z80_preserves.rs` | 1-70 of 712 | gap 2 |
| `…/z80_out_verify.rs` | 1-40 of 458 | gap 2 |
| `…/tests/z80_bus.rs`, `tests/branch_const.rs`, `sigil-cli/tests/slot_type_corpus.rs` | the fixture shapes only (about 110 lines in all) | used to build the planted defects |
| `engine/sound/z80_sound_driver.emp` (1,827) | code lines 106-775 (comments stripped); comments 410-426, 483-500, 551-612, 790-830; `SndDrv_TimerATick` header and first 12 instructions. Other code after 830 **not read** | gap 2 |
| `engine/sound/sound_fm.emp` | 23 proc headers | gap 2 |
| `engine/sound/sound_sequencer.emp`, `sound_sfx.emp`, `sound_psg.emp` | **not read**; grep counts only | gap 2 counts |
| `engine/sound/sound_api.emp` | 84-125, 573-582; proc headers by grep | gap 5 |
| `aeon/docs/research/2026-11-13-engine-systems-review.md` | 30-47 | gap 5 |
| `aeon/docs/EMP_PITFALLS.md` | 76-117, 159-173, 199-272, 336-456 (adds §2, §5, §7, §8, §9, §12) | gap 3; §4, §6, §10, §11, §13 still headings only |
| `sigil/docs/EMP_PITFALLS_EQUALITY.md` | 55-110 | gap 3 |
| `sigil/docs/superpowers/notes/2026-09-07-check-then-use.md` | 1-40, 125-145, 290-329 of 329 | gap 4 |
| `sigil/docs/superpowers/notes/campaign-gap-ledger.md` | 4505-4560 | gaps 1, 4 |
| `aeon/build.sh` | 960-975 | finding 10 |
| `aeon/engine/objects/dplc.emp` | 58-70, 111-123, 164-203 | finding 11 |

Runs this round (all MEASURED, shipping release `sigil`): 3 whole-tree `sigil build` runs and 3 `--report contracts` runs in the scratch copy (clean, slot plant, bus and branch plant); 38 single-file or `--root` probes. No test suite was re-run; the first round's total (2,784 passed, 0 failed, 0 ignored) stands unchanged and was not repeated.

**Not done this round:** lens S12-b and the two comptime DSLs were sent to reviewer B and are not examined here. `CODING_CONVENTIONS.md` is still headings only.
