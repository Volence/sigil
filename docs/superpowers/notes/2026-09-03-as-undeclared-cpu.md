# An undeclared processor is a hard error, scoped to the assembly unit

2026-09-03 · branch `parcel/as-default-cpu` · sigil master base `849a2412`

## The defect

`sigil_frontend_as::Options::initial_cpu` defaulted to `Cpu::Z80`. That was
honest for the Z80-only M0 build it was written for and silently wrong from the
moment the front-end took 68000 work.

The processor is not a label on the output; it decides how the lexer reads the
source. Under `Cpu::Z80` a `$` is the program counter. Under `Cpu::M68000` it is
a hex prefix. So a 68000 source with no `cpu` line did not fail to target
68000 — it targeted Z80, and reported nothing about it. The previous parcel
measured the consequence: the public Sonic 2 disassembly had been assembling as
a Z80 program on this path. Case-folding the `CPU` directive rescued that
corpus only because it happens to carry a `cpu` line at all.

## Why it refuses rather than warns

A run that reports what it skipped still exits 0, and a silent green is the
class this repo does not drop. There is also no honest warning form here: by
the time anyone reads "the assembler picked a processor for you", the bytes are
already encoded against it.

## The design: the scope is the UNIT, not the file

This is the half whose absence breaks a shipping build, and it was worth
deriving from the aeon tree rather than assuming. At aeon `4f5ad5a1`:

| Fact | Where |
|---|---|
| `cpu 68000` | `games/sonic4/game_root.asm:15`, `games/demo/game_root.asm:15` |
| No `cpu` directive at all | `engine/debug/debugger.asm` — the ONLY one of aeon's 3 `.asm` files without one |
| Included *after* the declaration | `game_root.asm` line 35 (sonic4) / 32 (demo), inside `ifdef __MDDBG__` |
| One root per invocation | `build.sh:175` — `MAIN_ASM="games/${GAME}/game_root.asm"` |

`debugger.asm` is a legitimately silent *included* file beneath a root that has
already declared. A per-FILE refusal fires on it and breaks every aeon shape.

`AsmState` is one state threaded through the root and every file it splices, so
scoping the obligation to the unit is not extra machinery — it is what the
existing state already models. A `cpu` line anywhere in the unit satisfies it.

## Mechanism

- `Options::initial_cpu` is `Option<Cpu>`, `None` by default. **Setting it IS
  declaring** — the harness's residual-AS root (`native.rs`) and the `.emp`
  sound stack drive the front-end on fragments with no directive of their own,
  and they declare `Some(Cpu::M68000)` at the call.
- `AsmState::cpu_declared` — a one-way latch, deliberately NOT part of the
  `save`/`restore` snapshot. Declaring a processor is a property of the unit,
  not scoped assembler state, so `save`/`restore` cannot un-declare it, and
  `restore` re-applying a saved CPU is not itself a declaration
  (`declare_cpu` latches, `set_cpu` does not).
- `PROVISIONAL_CPU = Cpu::Z80` while nothing has declared. Provisional, never a
  target choice: no accepted output is ever encoded against it. It is `Z80`
  precisely because that was the old silent default, which keeps the ONLY
  behavioural change of this parcel the refusal itself.
- **Two detection points.** `Asm::emit` — the single call site of
  `builder.emit_data`, so the one place bytes enter the module — catches a unit
  that produces bytes undeclared, which is where the wrong-target damage is, and
  aborts the pass so the real error is not buried under its own symptoms.
  End-of-`one_pass_with_defer` catches the rest: a byteless unit still had every
  `$` in it lexed against a processor nobody named, and its `equ` values carry
  that decision out to whoever consumes them.
- **Reported first.** The refusal is raised where it is DETECTED and hoisted to
  the front of the pass's diagnostics, because everything else an undeclared
  unit produces is a consequence of it.

## The enumeration of undeclared callers

Every `initial_cpu` site in the workspace, classified by the struct literal that
owns it: `LowerOptions` 339 (emp's own field, untouched), `AsOptions` 112,
`Options` 34. All 146 AS-side sites set the field explicitly and became
`Some(..)`.

**Exactly one caller drove this front-end without declaring:**
`crates/sigil-cli/src/main.rs:98` — `sigil_frontend_as::Options::default()`, the
shipped `sigil <input.asm>` command. It is the one this ruling is about, it had
no test, and it is left calling `Options::default()` on purpose: it now inherits
the refusal, which is the correct behaviour for a command whose input is a
foreign tree.

**No existing test relied on the silent default.** The full suite ran green on
the type change before any gate was added — the fallout of removing the default
was zero. That is a finding, not an assumption: it means the default was doing
nothing for anyone in-tree and only mis-serving outside users.

## Gates

`crates/sigil-cli/tests/cpu_undeclared.rs`, five tests, two of them driving the
CLI PROCESS rather than the library — `Options::initial_cpu` was always
reachable and a library-level test would have passed throughout while the
shipped command silently mis-targeted.

Red-first, each mutation applied from a committed baseline and read back from
disk:

| Mutation | Red |
|---|---|
| A: `cpu_declared: true` (restore the silent default) | 5 red — 3 in `cpu_undeclared.rs`, 2 in `state.rs`. The hazard guard correctly stayed green. |
| B: `cpu_declared = false` on entering an `include` (scope the refusal to the FILE) | 1 red — the hazard guard, and only it |
| C: drop the diagnostic hoist | **green — the gate was vacuous**, see below |

Mutation C is the one worth recording. The ordering assertion stood on
`FOO equ 16`, whose only diagnostic is the refusal itself, so it was first
either way and the assertion proved nothing. The gate was rewritten onto
`FOO equ $10`, which mis-parses under the provisional processor and so gives the
refusal something to be ordered ahead of, plus a `diags.len() >= 2` precondition
that fails loudly if that cascade ever stops happening rather than passing for
the wrong reason. Mutation C was then re-run against the fixed gate and was red.

## Left open

- **`cpu z80undoc`** (`s2.sounddriver.asm:250`) — a declared-but-UNSUPPORTED
  variant spelling, which `directive_cpu` rejects. Deliberately out of scope:
  it is the opposite case (declared-but-unknown, not undeclared) and belongs to
  its own row so the attribution survives. This parcel does make its fix
  obvious — one arm in `directive_cpu`'s `fold_kw` match, alongside `z80` — but
  it is not implemented here.
- **The pre-declaration `$` window.** A unit that declares LATE (code before its
  `cpu` line) has that code lexed against the provisional processor. The
  refusal does not fire, because the unit does declare. `emit` cannot see this:
  the CPU-dependent decision is made in the lexer, and threading it back is a
  larger change than this parcel. In practice the byte-emitting cases still
  fail — the mis-lex produces its own errors — but the diagnosis is the
  cascade, not the cause.
