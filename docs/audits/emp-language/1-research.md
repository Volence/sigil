---
kind: audit-stage
system: emp-language
stage: "1 Research"
agent: Opus
status: done
description: "Shipped Genesis games put game logic in native asm. Interpreters handle only leaf jobs. Measured GCC 15 (-O2 -m68000): 1.07-1.4x hand cycles on arrays and struct math, 2.3-5x on dispatch, scripts and HInt. Named boss+shmup load: +5.6k cycles/frame (4.4%). Clang m68k calls __mulsi3."
---
# EMP language: research (sections 3 and 4)

> [!verdict] Conclusion: what the evidence supports (research, not the disposition)
> 1. **No shipped Genesis engine wrote its game logic in a compiled or high-level language, with one exception.** Every commercial engine checked writes object and boss logic as native 68000 code. It uses state dispatch through index or code-pointer tables. Interpreted scripts appear only for **leaf** jobs: animation, music, timed event lists and VDP command lists. Treasure (Gunstar Heroes, Alien Soldier), Technosoft (Thunder Force IV), BlueSky (Vectorman) and Sonic Team (S3K) all fit this pattern (script scans in §3.3). The one documented C title is *Sonic Spinball*, written by Sega Technical Institute (PUBLISHED, Morawiec). Most Genesis code was asm.
> 2. **The best "higher-level game logic" designs ran on hardware 10 to 100 times faster than this.** GOOL (Crash, PS1) was "mostly interpreted". GOAL (Jak, PS2) was a native compiler. Gavin's own figure is that compilers are "about half as good as a talented assembly programmer" (PUBLISHED). The idea that carries over to a 68000 is GOOL's *state + code block that suspends per frame* model. That is a coroutine, and it lowers to a resume table at zero cost. `.emp`'s `script` is already this construct. It ships with **0 aeon uses** (inventory §1a).
> 3. **I measured a modern compiler on this exact CPU, on Genesis-shaped workloads.** GCC 15.2 `-O2 -mcpu=68000` against hand asm, with cycles from sigil's own 68000 cost table (MEASURED, §4.2):
>    - **1.02 to 1.12×** for straight struct math and array loops (MoveSprite, a TF4-style bullet pool).
>    - **1.4×** for a multi-part boss limb.
>    - **2.3 to 2.8×** for state dispatch and a shmup enemy script step.
>    - **2.6 to 5.1×** for an HBlank handler.
>
>    The losses are almost entirely the C calling convention, missing zero-extension idioms, and compare chains in place of jump tables. The register allocator is not the problem. **LLVM/clang m68k called `__mulsi3` for a 16×16 multiply** (MEASURED). Rust's m68k target is tier 3 and defaults to the 68010 (PUBLISHED). Neither is a credible Genesis backend today.
> 4. **The named non-Sonic load, costed.** One boss core with 6 linked limbs, 64 bullets and 24 scripted shmup enemies:
>    - Hand asm: about **16.3k cycles**.
>    - GCC: about **21.9k cycles**.
>    - The difference is **+5.6k cycles, 4.4% of a 128,006-cycle NTSC frame** (ESTIMATED from the measured per-unit costs).
>
>    For game logic, a good compiler tier costs a few percent of a frame. For interrupt code it does not fit: the measured worst VBlank already uses 97% of vertical blank (frame-architecture stage 2), and the GCC HBlank handler uses 46% of a scan line.
> 5. **The constraint that set tenet 3 is gone.** "The instructions don't change" was justified by byte-exact AS migration (S2-D15: "a compiler that chooses branches can never reproduce AS bytes"). That migration is finished: 190 `.emp` files, and the only `.asm` left is roots, the vendored debugger and editor exports (inventory). A compiled tier for *new* code no longer conflicts with any live obligation.
> 6. **`.emp`'s contract layer has no counterpart in any assembler I checked.** The checked items are transitive `clobbers`, symbolically verified `preserves`, `out`/flag-out, and per-proc `@budget` over a real 68000 cycle table. AS, vasm, ca65, RGBDS, KickAssembler, 64tass and SNASM68K/ASM68K have macros, scopes, structs and assertions, but no register-effect or cycle contracts (manual check, §3.1). This layer is also exactly what a compiled tier would need in order to meet hand-written code safely. **A compiled tier would be checked by the same `@budget` walker**, because sigil prices instructions, not source lines. I used it that way to measure the GCC output in this report.
>
> **Directions, as the evidence ranks them (for the reviewers to judge):**
>
> | Direction | Evidence |
> |---|---|
> | (a) Exact everywhere, polished | Supported by every shipped engine. Leaves the 2.3 to 2.8× "dispatch and script" tax on the human, not the CPU |
> | (b) Exact plus checks and sugar | The cheapest wins have precedent: GOOL-style coroutines (already built as `script`), typed state dispatch (built as `dispatch`), and structured flow sugar that lowers 1:1 |
> | (c) A compiled tier for game logic, exact for hot paths | Measured cost is small for logic, but only if the tier uses register calling and owns its jump tables, which C does not. No shipped Genesis precedent besides Spinball. Closest modern precedent is SGDK (C library, about 8% asm by bytes, measured), which runs hot paths in asm |
> | (d) Functional everywhere | Contradicted by the numbers for HInt/VBlank (2.6 to 5.1×), the Z80 driver (needs exact cycle spacing), and the VDP copy loops (1.5 to 2.3×) |

## How this was done

| Step | What | Label |
|---|---|---|
| Local disassemblies | Script and dispatch idiom scans over Gunstar Heroes, Alien Soldier, Thunder Force IV and Vectorman, from linear-sweep disassemblies under `/home/volence/sonic_hacks/The Adventures of Batman and Robin/*_disasm/code/disasm.asm`, plus `skdisasm/sonic3k.asm` and S.C.E. Scripts are in the appendix | MEASURED (heuristic, so a lower bound on interpreter use) |
| Compiler codegen | C versions of 8 Genesis-shaped routines, compiled on Compiler Explorer (godbolt.org API, 2026-11-16) with **GCC 15.2.0 m68k-unknown-elf** at `-O2`, `-Os`, `-O3` (`-mcpu=68000 -fomit-frame-pointer`) and **clang trunk `--target=m68k -mcpu=68000 -O2`**. Bytes come from the assembled object (`binaryObject`) | MEASURED |
| Cycle counts | Each hand routine and each GCC output (`%` removed, otherwise unchanged) placed in a `.emp` proc under `@budget(cycles: 1)` and run through `sigil emp` (binary `sigil 0.1.0 (7630ba0d)`, built 2026-11-13). The `[cycles.over-budget]` diagnostic reports the worst-path cost from `m68k_cycles.rs`. I spot-checked MoveSprite by hand against the 68000 timing table (164 = 64 + 84 + 16) | MEASURED (sigil's table; `muls` priced at its 70-cycle ceiling) |
| Web | Manuals and primary pages, cited inline | PUBLISHED |

No local m68k compiler exists on this machine. `clang -print-targets` lists no m68k (LLVM 23.1.4, Arch build), and there is no `m68k-*-gcc`, vbcc or zig (MEASURED). That is why the codegen ran remotely. Only my own synthetic C test routines were sent to Compiler Explorer, nothing from the user's repos. All files went to the scratchpad. No repo was written.

---

## 3. Best known approach

### 3.0 The floor: S3K / S.C.E. as a *language* (what they make you write)

S3K is AS-era macro asm plus a hand-built continuation model. **The object's code pointer is its state.**

| Idiom (skdisasm `sonic3k.asm`, grep) | Count | What it is |
|---|---|---|
| `move.l #Label,(a0)` (object code pointer = next state) | 1,761 | First-class code pointer as state (continuation passing) |
| `move.l #Label,$34(a0)` (callback after a wait) | 731 | A hand-written "resume here after timer" |
| `jmp Obj_Wait` | 211 | Timer-driven suspend, a coroutine `wait_frames` by hand |
| `CreateChild*_` | 579 | Data-driven child spawning from descriptor tables |
| `Animate_Raw*` | 448 | Animation *bytecode* (leaf interpreter) |

All MEASURED by grep. S.C.E. keeps the same model under names: `code_addr(a0)` 125 writes and `wait_addr(a0)` 10. **aeon** has 34 `code_addr` writes in 25 files and **0** `script` uses. So the floor already *is* a coroutine system, spelled by hand with untyped `move.l #label`. **Cost:** zero beyond the instructions. **Where it breaks:** a stale or garbage resume pointer is legal, and the suspend/resume protocol lives in comments. `.emp`'s `script` construct was designed to close exactly this gap (Spec2 §5.6, "a stale/garbage resume value is unconstructible").

### 3.1 Assemblers and macro systems

| System | How it works | Cost | Where it breaks | Source |
|---|---|---|---|---|
| **AS (Alfred Arnold)**, aeon's origin | Multi-CPU macro assembler: text macros, `function`, `struct`/`endstruct`, `phase`, conditional assembly, sections | None at run time | Untyped text macros; errors far from cause; no register or effect checks | AS manual, http://john.ccac.rwth-aachen.de:8000/as/as_EN.html; Spec2 header (AS inventory: 60 macros, ~230 guard sites) |
| **SNASM68K / ASM68K** (Cross Products / SN Systems; Sega's sanctioned kit) | Macro assembler with `rs` structure offsets, local labels, groups and sections; drove the SNASM dev hardware | None | Same class as AS. Sonic 1 community disassemblies target it. **Which assembler Sonic Team used in 1991 is not confirmed** (only a forum claim) | https://segaretro.org/SNASM68K, https://info.sonicretro.org/Disassemblies |
| **vasm** (m68k) | Motorola, mot and Devpac syntaxes; **peephole optimizer** (`-opt-movem`, `-opt-st`, `-opt-allbra`, `-sc` lists side-effecting ones; `-devpac` disables all) | Optimizations change flags (e.g. `opt-st`, `opt-movem`) | The assembler silently rewrites instructions, so flags you rely on can change. This is the opposite of tenet 3 | vasm `cpu_m68k.texi` (mirror: git.applefritter.com/Macintosh-SW/NetBoot/src/branch/master/vasm-1/doc) |
| **ca65** (cc65) | `.proc`/`.scope` nesting, `.struct`/`.union`/`.enum`, text macros, cheap `@` locals, `.smart` 65816 width tracking | None | No runtime control flow; `.smart` "cannot trace execution flow" | https://cc65.github.io/doc/ca65.html (read) |
| **RGBDS** (Game Boy) | Text macros, `SECTION` with `BANK[]`, `LOAD` blocks, `CHARMAP`, `RS` structs, `UNION`, `ASSERT`/`STATIC_ASSERT` | None | The bank/section model is the best of the 8-bit set. No register contracts | https://rgbds.gbdev.io/docs/rgbasm.5 (page did not render in fetch; features from the manual's known sections, so treat as PUBLISHED, not re-read) |
| **KickAssembler** (C64) | Assembler plus a full JavaScript-like **script language**: functions, data structures, pseudocommands, import of binary formats | None at run time; build-time scripting | Two languages in one file. `.emp` collapsed this into one comptime mechanism (D2.6) | https://www.theweb.dk/KickAssembler/webhelp/content/cpt_Introduction.html |
| **64tass** | `.proc`, `.struct`, `.union`, namespaces, functions | None | Same class as ca65 | tass64 manual (not re-read this run) |
| **asmx** (Bruce Tomlin) | Small multi-CPU assembler | None | Homebrew tool; no evidence of Sega first-party use | not checked; no verdict depends on it |
| **NESHLA** (6502) | High-level assembler: `if`/`while`/`switch`, typed vars, functions | Compiler chooses branches | Already studied: Plan-7 Part III, the reason for S2-D15's "no" | `sigil/docs/superpowers/specs/2026-07-06-sigil-spec2-p7-language-completion-research.md:280-326` |

> [!finding] The contract layer is new
> None of the manuals above offer register-effect contracts: transitive clobbers, verified preserves, outputs, flag outputs. None offers per-routine cycle budgets checked against a timing table either. ca65 is read for this run, and its closest feature is `.smart`, a width tracker that "cannot trace execution flow". The rest are from known manual contents. The nearest prior art is academic **Typed Assembly Language** (Morrisett et al., TOPLAS 1999), which types registers and the stack for safety but has no cycle model. External demo tools such as 68kcounter count cycles outside the assembler (Plan-7 Part II, T2-g).

### 3.2 Compiled languages on 68k-class hardware

| Language / toolchain | State on a plain 68000 | Measured or published codegen | Where it breaks | Source |
|---|---|---|---|---|
| **GCC (SGDK)** | Mature. SGDK release flags: `-m68000 -O3 -flto -fno-web -fno-gcse -fno-tree-loop-ivcanon -fomit-frame-pointer -fno-builtin` | §4.2: 1.02 to 5.1× hand cycles depending on shape. SGDK's own library is **58 `.c` files (2.08 MB) and 14 68k `.s` files (171 KB)**, i.e. hot paths (DMA, memory, maths3D, tile, bitmap, task) in asm | Stack-argument ABI; memset recognition turns loops into library calls unless `-fno-builtin` (MEASURED: `CountLoop` → `jsr memset`) | https://github.com/Stephane-D/SGDK/blob/master/makefile.gen; GitHub tree API, 2026-11-16 (MEASURED) |
| **vbcc** | Mature on Amiga. `__reg("d0")` register parameters; separate 68000/020/060 inline variants | No published Genesis cycle comparison found | Smaller community; no Genesis runtime | vbcc manual (m68k backend chapter); EAB GCC 6.2 thread (benchmarks, ratios only): https://eab.abime.net/showthread.php?t=85474&page=3 |
| **LLVM / clang M68k** | Experimental target since March 2021. **Not built in Arch's LLVM 23** (MEASURED) | **clang trunk emitted `jsr __mulsi3` for `(int32)a*b>>8` of two `int16`** (42 B vs GCC's 12 B `muls.w`), and clang's code is 5 to 51% larger than GCC on 5 of the other 6 routines, 14% smaller on `EnemyScript` (MEASURED) | The 68000 is a second-class subtarget | https://lists.llvm.org/pipermail/cfe-dev/2021-March/067873.html |
| **Rust** | `m68k-unknown-none-elf` is **tier 3** (no official builds, `build-std` required), default CPU **M68010** | Inherits LLVM M68k (above) | 68010 default; LLVM codegen | https://doc.rust-lang.org/rustc/platform-support/m68k-unknown-none-elf.html; PR #135085 |
| **Zig** | Rides LLVM's experimental M68k | not measured | Same as LLVM | not checked; no verdict depends on it (the LLVM finding is the binding one) |
| **Millfork** | Middle-level language for 6502/8080/Z80/6809 | n/a | **No 68000 backend** | https://github.com/KarolS/millfork |
| **Prog8** | 6502 first; newer docs add an m68k backend (amiga500 = plain 68000, amiga1200, qemu68k), virtual registers mapped to hardware with spilling | not measured | Amiga-only, no Genesis target; IRQ helpers missing on m68k | https://prog8.readthedocs.io/en/latest/targetsystem.html |
| **Cowgol** | Ada-inspired; backends include **68000** (Atari ST, Amiga CP/M-68k, Linux m68k), Z80, 6502, PDP-11 | Author: "pleasantly surprised by how good the generated code is", "doesn't match gcc" | Hobby compiler; global static allocation | https://github.com/davidgiven/cowgol |

### 3.3 Game-specific languages and interpreters

| System | How it works | Cost | Where it breaks | Source |
|---|---|---|---|---|
| **GOOL** (Crash 1-3, PS1) | Lisp dialect. `defgstate` states, each with `:trans` (runs every frame) and `:code` blocks that keep their own PC and **suspend per frame**; events between objects; 56 primitives, 420 macros | "Mostly interpreted"; by Crash 2 basic expressions compiled to machine code. A sample state was about 25 R3000 words | Gavin: compilers are "about half as good as a talented assembly programmer"; the hottest ~5% of functions are hand asm | https://all-things-andy-gavin.com/2011/03/12/making-crash-bandicoot-gool-part-9/ |
| **GOAL** (Jak, PS2) | Compiled Scheme-like language, native code, written in Allegro CL | Native | Needed PS2-class hardware and a custom compiler | https://en.wikipedia.org/wiki/Game_Oriented_Assembly_Lisp |
| **SCUMM** | Bytecode VM with cooperative threads (≤25), each run once per frame unless delayed | Interpretive overhead, fine for adventure games | Unsuited to action-game inner loops | https://www.scummvm.org/old/docs/specs/scrp.php |
| **Another World VM** | ~20 KB host VM; the whole game is bytecode (203,546 B unpacked); 64 script slots, each runs once per frame until a break opcode. **Shipped on Mega Drive (1992)**: only the VM was re-written | VM host per platform | A polygon adventure with few actors, not a run-and-gun | https://fabiensanglard.net/anotherWorld_code_review/ |
| **Treasure** (Gunstar Heroes, Alien Soldier) | Object logic is **native** code. 96-byte objects with link fields `$58`/`$5C` for multi-part bosses (AS: 380/104 refs). Index dispatch: `lea Tbl(pc,d0.w),a0; adda.w (a0),a0; jmp (a0)` (Gunstar `disasm.asm:4098-4102`). **Interpreters only as leaves:** Gunstar `loc_0022DE` is a command-list runner (`move.w (a0)+,d0; bmi end; lsl #2; movea.l Tbl(pc,d0.w),a1; jsr (a1); bra`). Alien Soldier `sub_032D60` is a **timed event sequencer** (wait word + op index, loops at end). The `$48` field is an animation cursor (Gunstar 8 sites, AS 5) | Native | Nothing in the code is shared across many bosses except the link protocol, which is untyped (the analysis notes "$58 is used for both code pointers and object addresses") | local disassemblies, lines cited; `gunstar_disasm/ANALYSIS.md` |
| **Thunder Force IV** | Type-segregated 32-byte pools for bullets and enemies; RAM-patched HBlank. Scan finds **no per-object script cursor** and one command dispatcher (`sub_011A16`, `move.w (a0)+,d0` → `movea.l (a1,d0.w),a2; jsr (a2)`) | Native | Heuristic negative. The enemy movement could still be table-driven in a way the scan does not see. Reviewers may confirm | `thunderforce4_disasm/code/disasm.asm:14585`; `ANALYSIS.md` |
| **Vectorman** | Absolute code pointers as state (`move.l #NewState,$4(a4)`); 4 per-object cursor sites | Native | n/a | Plan-7 Part II R1; scan |
| **Batman & Robin** (Clockwork Tortoise) | **The exception:** a two-level threaded-code object interpreter (`movea.w (a2)+,a0; jmp (a0)`) with a yield opcode whose saved script PC *is* the state | Threaded dispatch per op (about 26 cycles ESTIMATED: `movea.w (a2)+,a0` 8 + `jmp (a0)` 8 + return path) | The game had a performance reputation the reviewers can weigh; not measured here | Plan-7 Part II R2 (claims verified against raw asm at the time) |
| **Konami** (Contra Hard Corps, Castlevania Bloodlines, Rocket Knight) | — | — | **BLOCKED:** no Mega Drive disassembly found locally or by search. No verdict here depends on Konami | search, 2026-11-16 |
| **Sonic's own scripts** | Leaf bytecode only: animation scripts (`$FF`/`$FE`/`$FD` commands; S3K `Animate_Raw*`), SMPS music on the Z80/68k, object layout data | Interpreter per leaf | n/a | skdisasm, s2disasm |

**What fraction of a shipped game was in each style** (ESTIMATED from the scans above; the patterns are a lower bound for interpreters):

| Game | Native object/boss logic | Interpreted | Evidence |
|---|---|---|---|
| S3K | essentially all | animation, music (SMPS), layouts | 1,761 code-pointer states vs 448 `Animate_Raw` |
| Gunstar Heroes / Alien Soldier | essentially all | 1 command-list runner each; AS adds 1 timed sequencer; animation cursors | 167 / 85 computed jumps vs 1 / 2 fetch-dispatch loops |
| Thunder Force IV | essentially all | 1 command dispatcher | 37 computed jumps vs 1 |
| Batman & Robin | the interpreter itself | **the object model** | Plan-7 R2 |
| Sonic Spinball | **C** (STI) | — | Morawiec, Sega-16 interview; https://info.sonicretro.org/Sonic_the_Hedgehog_Spinball/Development |
| Another World (MD) | VM host only | the whole game | Sanglard |
| Xeno Crisis (2019, homebrew) | C on SGDK | — | Bitmap Bureau; https://www.gamedeveloper.com/design/making-a-sega-mega-drive-genesis-game-in-2019 |

### 3.4 Modern ideas from outside the Genesis

| Idea | How it works | What it costs here | Where it breaks | Already in `.emp`? | Source |
|---|---|---|---|---|---|
| **Zig `comptime`** | One language at build and run time; types as values | Zero (build time) | The run-time half is LLVM codegen | **Yes, the comptime half** (D2.6 one mechanism; 384 `comptime fn` lines) | ziglang.org docs |
| **Coroutines lowered to state machines** (Rust `async`/generators, C# iterators, Protothreads) | The compiler splits a function at each `yield`, stores a resume index, and emits a switch / jump table. Protothreads need 2 bytes per thread | The same as hand `routine(a0)` dispatch: about 44 cycles (§4.2 W3, hand) | Locals do not survive a yield unless stored in the object, which matches Genesis practice | **Yes: `script`** (Spec2 §5.6), unused | Dunkels et al., "Protothreads", SenSys 2006 |
| **Rust traits / ownership as contracts** | Static capability and aliasing rules | Zero (erased) | Needs a type system over registers | Partly: `requires`/`grants`, `interface`, `context`/`with` (inventory §1a/1b) | — |
| **Effect systems** (Koka, algebraic effects) | A function's side effects are part of its type | Zero | — | **Yes, for registers and flags:** `clobbers`/`preserves`/`out`, checked transitively; Z80 bus-state contract | — |
| **Typed registers / TAL** | Registers carry types; calls check them | Zero | — | Yes: `aN: *Struct` params (248 headers), domain newtypes at call sites | Morrisett et al. 1999 |
| **Typed state dispatch** | Closed sum type of states | Zero | — | `dispatch` (0 uses), `comptime enum` + `match` (comptime only) | Spec2 §5.5 |

> [!note] The one idea every high-level game language shares
> GOOL, SCUMM, Another World and Batman & Robin share one model: **an object is a per-frame coroutine with named states and suspend points.** On the 68000 this costs nothing when the compiler lowers it to the code-pointer or resume-index dispatch that S3K already writes by hand. Everything else these languages add (garbage collection, dynamic dispatch, interpreted expressions) is what cost them their 2× or more.

### 3.5 The named non-Sonic workload, from the sources

- **Run-and-gun multi-part boss (Gunstar/Alien Soldier):** native state code per part; parent/sibling links at fixed object offsets; per-part timers; a timed event sequencer for attack patterns (AS `sub_032D60`). A language needs typed links (`ref<Entity>`, Plan-7 T2-h), state dispatch, per-frame suspend and wait, and cheap spawning of children and projectiles.
- **Shmup enemy script (TF4):** segregated bullet pools with a tight integration loop. A language needs pool and collection kinds (Plan-7 T2-h: TF4 type-segregated pools) and a cheap per-enemy step. If enemy patterns are authored as data, it needs a typed byte-command format with a verified interpreter (Plan-7 T1-c, re-gated in D2.26).

---

## 4. Hardware ceiling

Shared numbers from frame-architecture stage 1 (`aeon/docs/audits/frame-architecture/1-research.md:196-226`): **128,006 68000 cycles per NTSC frame**, **≈488.6 cycles per scan line** (H40), ≈18,566 cycles of vertical blank, ≈2.38 cycles frozen per DMA byte. A shared [[Hardware limits]] page does not exist yet. New items for this system: the compiler-recoverable fraction below.

### 4.1 The language's ceiling is the CPU

Every `.emp` instruction line is a 68000 or Z80 instruction the author wrote, so `.emp` *as a language* sits at **100% of the hardware ceiling by construction**. Any shortfall is the author's, not the language's. The question for a higher tier is therefore only **how much of that ceiling a compiler gives up, and where.**

### 4.2 Measured: GCC m68k against hand asm on Genesis-shaped routines

GCC 15.2.0 `-O2 -mcpu=68000 -fomit-frame-pointer` (MEASURED; `-O3` gave byte-identical sizes; `-Os` was smaller on 3 of 7). Cycles are the worst path through sigil `@budget` (68000 table), with `rts` as the stand-in end. Ratios are GCC ÷ hand.

| # | Workload | Hand asm | GCC | Ratio (cycles / bytes) | Main cause of the loss |
|---|---|---|---|---|---|
| W1 | `MoveSprite` (S3K `sonic3k.asm:36035`, aeon `ObjectMove` `core.emp:765`) | 164 cyc / 32 B | 184 cyc / 40 B | 1.12× / 1.25× | Stack argument load (16 cyc); `addi.w #,d16(An)` split into 3 instructions. Inlined: ≈168, 1.02× (ESTIMATED) |
| W2 | TF4-style bullet, one active on-screen iteration | 182 cyc | 194 cyc | 1.07× | `cmpa.l #end` + `bne` instead of `dbf` |
| W3 | Routine dispatch on `routine(a0)` (4 states) | 44 cyc constant (30 measured + `jmp d8(pc,Xn)` 14) | ≈122 cyc worst arm (128 measured with `rts` for `jra`) / 74 B | **2.8×** | GCC chose a **compare chain**, not a table, for 4 cases; re-stores the argument before each tail call |
| W4 | Multi-part boss limb (follow parent link, table offsets by parent state, timer, state step) | 228 cyc / 70 B | 318 cyc / 116 B | 1.39× / 1.66× | `andi.l #255` zero-extension (16 cyc), absolute `lea` tables, `a2` save/restore, stack argument |
| W5 | 16×16 → 32 fixed multiply, `>>8` | 110 cyc / 6 B | 130 cyc / 12 B | 1.18× | Arguments from the stack. **clang: `jsr __mulsi3`, 42 B** |
| W6 | Copy longwords to the VDP data port | 34 cyc/long (rolled, `dbf`); 21.75 (×8 unrolled) | 50 cyc/long | 1.47× / 2.30× | Port kept as `abs.l` in every store instead of an `An` register; `-Os` emits a worse `tst`/`jne`/`subq` loop |
| W7 | Shmup enemy script step (wait opcode path) | ≈138 cyc (124 + dispatch `jmp` 14) | ≈316 cyc (302 + 14) / 168 B (`-Os` 130 B) | **2.29×** | `movem` of 3 callee-saved regs, stack arguments, `andi.l #255`, an extra pointer copy |
| W8 | HBlank handler: one VSRAM write per line from a table | 88 cyc (RAM pointer); **44 cyc** (registers pinned to HInt) | 226 cyc / 60 B | 2.6× / **5.1×** | `movem` save/restore, a 16-bit index rebuilt as `andi.l`/`add.l`, every VDP access `abs.l`. Of a 488-cycle line: **46%** vs 18% / 9% |

Other compiler behaviour MEASURED in the same run:
- RAM at `$FFFF8000` became `sub.l a0,a0; move.w -32768(a0)`, not `abs.w`.
- A zeroing loop became `jsr memset`.
- No `dbf` was ever emitted.
- GCC's `-O2` and `-O3` sizes were identical across the set.

> [!finding] What a compiler can and cannot recover on a 68000
> | Area | Can recover | Cannot (or does not today) | Evidence |
> |---|---|---|---|
> | Register allocation, leaf code | Yes: W1/W2 within 2 to 12% | — | W1, W2 |
> | Calling convention | Only with a custom ABI (vbcc `__reg`; GCC global register variables / `-ffixed-reg`) | Mainline GCC m68k passes every argument on the stack: +16 cyc per argument, plus `movem` for callee-saved registers | W1, W4, W5, W7, W8 |
> | Flag results (Sonic's "Z set = found") | Not in C | No C ABI returns in CCR. `.emp` has `out(carry: …)` (18 uses) | inventory §1b |
> | Addressing modes | `d16(An)`, `(An)+`, `d8(An,Xn)` | `abs.w` RAM; keeping a hardware port in `An`; `.w` index without zero-extension | W4, W6, W8 |
> | Jump tables | For larger switches (W7: PC-relative word table) | Small switches become compare chains; no code-pointer-as-state (S3K `move.l #X,(a0)`) without function pointers and their ABI | W3, W7 |
> | Loops | Pointer-increment loops | `dbf`; unrolling to the VDP FIFO | W2, W6 |
> | Multiply | GCC: `muls.w`/`mulu.w` when operands are provably 16-bit | clang: library call | W5 |
> | Cycle counting | No mainstream compiler models 68000 cycles | Cycle-exact code (HInt per-line work, YM2612 write spacing, DMA windows) cannot be *specified* to a compiler, only checked after | sigil `@budget` works on any instruction stream, so a compiled tier **can be checked**, not steered |
> | Interrupt safety | `__attribute__((interrupt))` saves what it uses | Instruction-level ordering and atomicity are not promised by the language (in practice `move.l` to the port stays one instruction); the Z80 bus handshake must be explicit | W8 |

### 4.3 Where each engine area sits against the ceiling (the owner's "functional everywhere" question)

| Engine area | Share of code (inventory §5) | Binding limit | Compiled-tier cost (from §4.2) | Headroom today | Verdict input |
|---|---|---|---|---|---|
| HBlank / VBlank handlers | part of 14.0% cycle-critical | Per-line ≈488 cyc; VBlank ≈18,566 | 2.6 to 5.1× (W8) | **Worst VBlank 18,052 = 97%** (DEBUG); release 15,242 = 82% (frame stage 2, MEASURED) | No room for any compiled slowdown |
| Z80 sound driver | 3,758 lines = 81% of cycle-critical | YM2612 address→data spacing guarded by `ensure(cycles(...) >= YM_ADDR_TO_DATA_MIN_T)` (`sound_fm.emp:220`) | No compiler (SDCC, z88dk) models T-states | n/a | Exact required |
| DMA queue, VDP copy loops | in cycle-critical | DMA window; 2.38 cyc/byte | 1.47 to 2.30× (W6) | Drain inside the 97% VBlank | Exact required |
| Decompression, tile streaming | in engine systems 24.1% | Main-loop cycles; `Tile_Cache_Fill` up to 109.6k in one call, which is the lag spike (frame stage 2) | ≈1.1 to 1.5× (ESTIMATED from W2/W6 loop shapes) | Already overruns on diagonal frames | Exact preferred; a 1.3× loss adds lag frames |
| Object system core | 6.9% | `RunObjects` 11k to 17k/tick in physics play, release (MEASURED, frame stage 2) | 1.02 to 2.8× | ≈50% of frame spare on average | Hot helpers exact |
| **Game and object logic** | **14.1%** (10.1% without test harness) | Logic cycles | **1.07 to 2.3×** | `Player_Main` 8.8k/tick, release | **Compiled tier affordable** if it owns its ABI and dispatch |
| Named workload: 1 boss + 6 limbs + 64 bullets + 24 scripted enemies | — | Logic cycles | Hand ≈16.3k, GCC ≈21.9k: **+5.6k = 4.4% of a frame** (ESTIMATED: 6×W4 + 64×W2 + 24×W7) | — | Affordable; bullets dominate and lose only 7% |

> [!note] Hardware-ceiling summary
> The language is at 100% of the 68000 by construction. A C-class compiler gets back ≈90 to 98% of the ceiling on array and struct math, but only ≈35 to 45% on dispatch, scripts and interrupt code. A compiler that controls its own calling convention, its jump tables and code pointers as state would close most of the dispatch gap. That claim is ESTIMATED from the W3/W7 loss breakdown: the ABI and zero-extension account for most of the extra cycles. No such 68000 compiler exists to measure.

---

## Coverage

| Source | Read | Result |
|---|---|---|
| `emp-language/0-inventory.md` | all | used throughout |
| `aeon/docs/audits/AUDIT_TEMPLATE.md`, `Audit board.md`, `frame-architecture/1-research.md` §4, `2-measurements.md` | template; board 1-60; frame research 1-20, 196-226; measurements 1-30, 170-262 | hardware numbers, owner's "functional everywhere" question |
| `Audit process review.md` | 1-200 | applied research rule 5. §3.3's verdict cells come from my own scans of the raw disassembly, not from the `ANALYSIS.md` summaries; the "no clobber/cycle contracts elsewhere" claim names which manuals were re-read |
| `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` | 1-150, 854-980, ledger 1290-1301 | tenets, `dispatch`, `script`, S2-D15 |
| `empyrean/docs/SIGIL_IDEAS_FUNCTIONAL_TIER.md` | all | the `.emp2` premise; the in-place-mutation rule |
| Other `SIGIL_*.md` (CORE, M0, ORACLE_ISA, PLAN_AUDIT, AEON_COMPAT) | not read | outside §3/§4 (reviewers' sections) |
| `sigil/docs/superpowers/specs/2026-07-06-…-language-completion-research.md` | 1-25, 180-330 | earlier non-Sonic research, used and re-checked |
| `sigil-frontend-emp` source | `tests/t40_cycles.rs` 1-80; `cycle_budget.rs` header via grep | `cycles()` is Z80-only; `@budget` is the 68000 path |
| EMP_PITFALLS*, `aeon/CLAUDE.md`, `sigil/editors` | not read | reviewers' sections 6/7 |
| Corpus | `core.emp:762-803`, `games/sonic4/objects/test_parent.emp:1-150`; grep counts | sample of the port-shaped style |
| Gunstar / Alien Soldier / TF4 / Vectorman disassemblies | scans + lines cited (`gunstar 1985-2030, 4098-4102, 51530-51580`; `tf4 14570-14610`; `AS 21930-21960`) | §3.3 |
| skdisasm, S.C.E. | grep counts; `sonic3k.asm:36035-36046` | §3.0, W1 |

## Appendix: reproduction

All under `/tmp/claude-1000/-home-volence-sonic-hacks/f8a970f2-04e1-487a-bbda-d3729df1480b/scratchpad/emp-research/`:
- `w.c`, `x.c`, `h.c`: the C workloads.
- `gba.py`: Compiler Explorer client; prints the asm and per-function bytes from the object.
- `out_*.s`: compiler outputs for GCC -O2/-Os/-O3 and clang.
- `meas.py` + `specs1-5.json`, `m1.emp`: the cycle measurements through `sigil emp` with `@budget(cycles: 1)`.
- `scan2.py` (fetch→dispatch interpreter loops) and `scan3.py` (per-object `movea.l off(aX),aY … (aY)+ … move.l aY,off(aX)` cursors): the disassembly scans.

Cycle numbers include an `rts` (16) where a routine ends. Loop iterations subtract it, as stated per row. `muls` is priced at its 70-cycle ceiling.
