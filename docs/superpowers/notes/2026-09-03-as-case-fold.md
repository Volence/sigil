# AS case folding — directives and mnemonics fold, symbols never do

Landed at sigil `5bbfb2c0` on `parcel/as-case-fold` (one file: `crates/sigil-frontend-as/src/eval.rs`).

## The defect, and why it was silent

`@as_compat` matched its directive and mnemonic keywords in lower case only. The corpus that
the `SIGIL-AS-REPLACEMENT` project exists to assemble — the community Sonic 2 disassembly at
`s2disasm` `e45ebf3` — writes `CPU 68000`, `EQU` and `STRUCT` in capitals.

**The `cpu` directive is the one whose failure is not a diagnostic.** `Options::default()` sets
`initial_cpu: Cpu::Z80`, and under `Cpu::Z80` the lexer reads `$` as `Tok::Dollar` — the program
counter — rather than as a hex prefix. So an unrecognized `CPU 68000` line does not error: it
leaves a 68000 disassembly assembling as a Z80 program, and every subsequent `$FF00` reads as
arithmetic on the PC. This is a silent-wrong-answer class, not a missing-feature class, and it is
the reason the fold's first witness is an emitted byte (`moveq #0,d0` → `70 00`) rather than the
absence of a diagnostic.

## Where the fold lives, and why not in the lexer

`Tok::Ident` carries symbols, macro names, directives and mnemonics in **one** variant. Folding at
the token therefore folds symbol names too — and symbols are deliberately case-sensitive
(`lib.rs`: *"Names are case-sensitive"*), with `.emp` sharing this symbol namespace. A token-level
fold would merge two distinct `.emp` symbols.

The fold is `fold_kw`, applied at each site that DECIDES whether an identifier is a keyword. Every
path that goes on to DEFINE or RESOLVE a name keeps the spelling the source wrote.

The rejected design is not merely argued against; it is the applied mutation behind the two guard
tests (mutation B below), so the guard fails loudly if anyone later moves the fold to the lexer.

## The sites

A property established at one recognition site is not a property of the others. The enumeration
below is the consuming end, and **five of these are not reachable from `dispatch` at all** — the
block-scanning layer and the `exec_one` intercepts each decide keyword-hood for themselves.

| site | layer | what it decides |
|---|---|---|
| `dispatch` match scrutinee | directive dispatch | the directive arms |
| `is_op_keyword` | shared predicate | a second, separate directive-name list |
| `closers_for` | block scanning | which closer keyword ends a block |
| `dispatch_head` rule 1 | block scanning | `NAME MACRO`/`STRUCT`/`FUNCTION` definition heads |
| `dispatch_head` rule 3/4 | block scanning | the folded keyword handed to `exec`/`find_block_end`/`exec_if` |
| `exec_one` — `NAME EQU v` | pre-dispatch intercept | equate vs. stray label + mnemonic |
| `exec_one` — `NAME: EQU v` | pre-dispatch intercept | the same, behind a decorative colon-label |
| `exec_one` — `NAME SET v` / `NAME: SET v` | pre-dispatch intercept | reassignable-symbol binding |
| `parse_struct_field` (×2) | struct capture | `DS.B`/`DS.W`/`DS.L` member widths |
| `def_function` | definition | the `function` keyword |
| `is_mnemonic` / `mnemonic` | Z80 lowering | Z80 instruction table |
| `m68k_mnemonic` | 68000 lowering | 68000 instruction table |
| `split_mnemonic_and_size` | 68000 lowering | `.B`/`.W`/`.L`/`.S` operand size |
| `split_attribute_suffix` | macro expansion | `.ATTRIBUTE` macro-suffix invocation |
| `directive_cpu` operand | directive | the processor NAME (`CPU Z80`) |
| `on_off` | directive operand | `PADDING OFF` / `SUPMODE ON` |

`split_mnemonic_and_size` and `split_attribute_suffix` now share `split_dot_suffix`, so the two
agree on what a trailing size suffix is.

## What deliberately does NOT fold

- **Macro names, labels, equates** and everything downstream of them. `dispatch_head` returns a
  macro invocation's name exactly as written; `dispatch`'s macro arms read the raw head.
- **`.ATTRIBUTE` substitution.** The suffix is RECOGNIZED without regard to case but the macro
  body receives the spelling the call site wrote, because `.ATTRIBUTE` is a verbatim textual
  substitution — the body may paste it straight onto a mnemonic, and folding someone else's text
  on the way through would make the two ends disagree.
- **Register names** (`d0`/`a0`/`sr`/`ccr`, and the Z80 register/condition set in `operands.rs`).
  AS folds these; this front end does not, and the corpus does not need it: every uppercase
  `D0`/`A0`/`SR` token in all 332 `.asm` files is inside a comment. The hazard that argues against
  folding them is real — a label named `A0` used as an absolute address would classify as an
  address register — so this is booked rather than taken.
- **`MOMCPUNAME`** and the other builtin symbol names. A builtin is a symbol, and the string
  VALUES it compares against (`"Z80"`, `"68000"`) are string-literal comparisons, not identifier
  recognition. The corpus reaches for `MOMCPU`, which is unimplemented either way.

## Measurements

**Byte gate — the primary measure. All four aeon shapes, control and post-fold, side by side.**
Control built from master `9abb7dc6` before any edit; post-fold rebuilt from the landed commit with
the four ROMs deleted first. `AEON_DIR=/home/volence/sonic_hacks/.aeon-as-fold` at `4f5ad5a1`.

| shape | control crc/size | post-fold crc/size |
|---|---|---|
| `s4.bin` | `14ee2440` / 719700 | `14ee2440` / 719700 |
| `s4.debug.bin` | `142294b3` / 737683 | `142294b3` / 737683 |
| `demo.bin` | `0c456778` / 96474 | `0c456778` / 96474 |
| `demo.debug.bin` | `2e603d53` / 101339 | `2e603d53` / 101339 |

Not a side-car: aeon's `build.sh` routes `engine/debug/debugger.asm` and both `game_root.asm`
through this front end, so this parcel could have moved the shipping game's bytes. It did not.

**Suite:** 4246 passed / 0 failed / 2 ignored, `--workspace --no-fail-fast`, `SIGIL_STRICT_GATE=1`.
Reconciles as master's 4237 + 9 new (the diff is one file and adds exactly nine `#[test]`).

**s2disasm: 237 → 85,335 diagnostics, and the rise is the deliverable.**

The count moving was pre-declared. The decomposition:

- **Zero of the 129 distinct before-diagnostics survive.** The whole before-set was case-related,
  either directly or as cascade from the mis-selected processor.
- The corpus reached 4 of 332 files before; it now reaches 7 and produces 8,486 DISTINCT
  `file(line)` sites (85,335 counts the macro/`rept` multiplication of those sites).
- **A corpus that had been assembling as the wrong processor has never had its 68000 path
  exercised.** That path is now exercised for the first time, and what it reports is the honest
  inventory of what `@as_compat` does not yet implement:

| distinct sites | bucket | what it is |
|---|---|---|
| 3,163 | `unresolved symbol in operand` | downstream of the rows below |
| 2,601 | `bad operand expression` | downstream |
| 2,307 | `expected mnemonic, directive, or label` | **the nameless `+`/`-` labels** — the already-ruled `d-22` gap, verified by reading the cited lines |
| 251 | `is not a recognized 68000 mnemonic` | see the split below |
| 115 | absolute address needs an explicit width | in-scope-width limitation |
| 58 | instruction needs an explicit size suffix | default-size table |

The 251 unrecognized heads are 23 distinct names, and they split cleanly:

- **Real 68000 mnemonics outside the Aeon-scoped table:** `bchg`, `exg`, `roxl`.
- **AS directives not implemented:** `shift`, `label`, `irpc`, `charset`, `enum`, `nextenum`,
  `enumconf`, `listing`, `page`, `pushv`, `popv`.
- **Z80 mnemonics reached while the CPU is still 68000:** `ld`, `jp`, `di`. One cause, one line —
  `s2.sounddriver.asm(250)` says `cpu z80undoc`, which `directive_cpu` rejects as
  `unsupported cpu`, so the sound driver never leaves the 68000. `z80undoc` is a real AS CPU name.
- **Block closers reaching `dispatch`:** `endif`, `endm`, from nested `if` inside a macro body
  (`s2.macrosetup.asm:62`, the `even`/`ds` macros). A block-nesting gap in macro expansion, not a
  case gap.
- **Bare symbols in contexts not otherwise reached:** `HorizontalScrollBuffer`, `SoundQueue`,
  `zTrack`, `zVar`.

## Red-first proof

Every new test was proven red under an applied mutation, restored each time from the committed
baseline `5bbfb2c0` with `git checkout HEAD --`. The mutation was quoted back from disk and named
by `git diff --stat` before each red run.

| mutation | applied to | tests it reddens |
|---|---|---|
| **A** — `fold_kw` neutered to the identity | `eval.rs` | `uppercase_cpu_directive_selects_the_68000`, `directive_keywords_fold_at_every_recognition_site`, `block_keywords_fold_in_the_scanning_layer`, `mnemonics_and_size_suffixes_fold`, `fold_kw_leaves_lower_case_borrowed_and_does_not_touch_digits` — **plus the pre-existing `binclude_emits_file_bytes_verbatim`**, an independent witness that the `BINCLUDE` arm's fold is exercised |
| **B** — fold at the token in `lex_line` (the rejected design) | `lexer.rs` | `symbols_differing_only_in_case_stay_distinct`, `macro_names_do_not_fold` (21 tests red in total) |
| **C** — `split_dot_suffix` made case-sensitive | `eval.rs` | `attribute_macro_suffix_folds_and_substitutes_verbatim`, `split_dot_suffix_only_splits_a_single_trailing_letter`, `mnemonics_and_size_suffixes_fold` |

Mutation A left two of the nine green. That is not a pass: it says those two tests' property is
carried by `split_dot_suffix` rather than by `fold_kw`, which is why mutation C exists. Each test
is red under the mutation that removes ITS OWN mechanism.

## Booked, not done

- ~~`z80undoc` (and the rest of AS's CPU-name set) — one line in `directive_cpu`, and it is what
  keeps `s2.sounddriver.asm` on the wrong processor today.~~ **DONE** —
  `2026-09-03-as-cpu-variant-spellings.md`. It was not one line, and "the rest of AS's CPU-name
  set" is the wrong shape for the answer: a spelling is accepted only when it names an
  instruction set sigil encodes, so the set stays four. The same match was silently reading
  every NUMERIC spelling as `68000` (`cpu 6502` → exit 0, 68000 bytes).
- `bchg`/`exg`/`roxl` and the eleven unimplemented AS directives above.
- The macro-body block-nesting gap at `s2.macrosetup.asm:62`.
- Register-name folding — argued against above; the hazard is a label named `A0`.
- `MOMCPUNAME`/`MOMCPU` and builtin-symbol name folding — a symbol-surface call, parked with the
  `.emp` half.
