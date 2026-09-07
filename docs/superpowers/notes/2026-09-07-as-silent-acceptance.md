# AS silent acceptance: four refusals (2026-09-07)

Parcel `parcel/as-silent-acceptance`, branched from sigil master `435e1148`.
Four findings from the 22-seat review of `crates/sigil-frontend-as/src/eval.rs`,
each a case where the AS front end produced bytes, a hang, or a smaller image
with exit 0 and nothing said. All four are now named, located refusals with
non-zero exit. Reference assembler throughout: asl 1.42 Bld 212, the build at
md5 `61e672562465725a8c102288a7da9098`, reached only through
`docs/superpowers/notes/asl-reference/asl_ref.sh` (`asl_run`, exit status read
on every run; a run carrying any error was never used as a source of bytes).

Commits, in order (all on the parcel branch, one per finding):

| finding | commit | test runner (new file under `crates/sigil-frontend-as/tests/`) |
|---|---|---|
| 1 unterminated block | `61dfc7a6` | `as_unterminated_block.rs` (12 tests) |
| 2 dc.w / dc.l / dw truncation | `2f8146a0` | `as_dc_word_long_range.rs` (11 tests) |
| 3 rept budget | `33857754` | `as_rept_budget.rs` (5 tests) |
| 4 macro breadth budget | `4422a54f` | `as_macro_breadth_budget.rs` (3 tests) |

Every red below was produced by running the NEW test file against the
committed code one step earlier (the pre-fix code is the baseline, the fix is
the mutation), and every green by running it again after the fix. Build dir
for every cargo command: `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.target-asfix`.

## Finding 1: an unterminated block closed on the last line (CRITICAL)

**Reproduction.** `dc.b $AA / if 1 / dc.b 1,2,3,4 / dc.b 5`, no `endif`.
Base binary: `AA 01 02 03 04`, exit 0. The `dc.b 5` was consumed as the
closer. `m macro / dc.b 1,2 / dc.b 5` with no `endm`: `AA`, exit 0, the whole
tail of the file swallowed as the macro body. The same fallback
(`find_block_end` returning the slice's last index) served `if`, `switch`,
`rept`, `irp`/`irpc`, `while`, `struct` and `macro`. The existing guard in
`capture_macro` fired only when the `macro` head was itself the last line.

**asl.** Exit 2 on every shape, each a named error and no line number:
`#1470 missing ENDIF/ENDCASE` (if, switch), `#1803 REPT without ENDM`,
`#1804 WHILE without ENDM`, `#1801 IRP without ENDM`, `#1800 open macro
definition`, `#1551 open structure definition`. An `END` inside the open
block does not close it (same errors). An `if` opened AFTER `END` is never
read: exit 0, bytes `AA 01`.

**Fix.** `find_block_end` returns `Option<usize>`, `None` when no line closes
the block. Every executor goes through the new `block_end`, which reports at
the opener's line and returns `None`; the executor then returns
`lines.len()`, skipping the rest of its slice. A block opened after `end` is
never reached because `end` sets `aborted` and `exec` stops walking, which is
also asl's behaviour.

**Diagnostic** (CLI, `sigil u_if.asm`, exit 1):

```
u_if.asm(4): error: `if` is never closed: expected `endif` before the end of the enclosing source, and an open block would otherwise swallow every line after it (asl: missing ENDIF/ENDCASE)
u_macro.asm(4): error: `macro` `m` is never closed: expected `endm` before the end of the enclosing source, and an open block would otherwise swallow every line after it (asl: open macro definition)
```

**Red-first.** `as_unterminated_block` on `435e1148`: 9 failed, 3 passed
(the three terminated controls). Quoted failures:
`assembled with exit 0 to 5 byte(s) [AA, 01, 02, 03, 04] instead of refusing
the unterminated block`; the macro row `1 byte(s) [AA]`; rept `[AA, 01, 02,
01, 02]`; while `[AA, 00, 01]`; irp `[AA, 01, 02]`; switch and the nested-if
row `[AA, 01]`; struct `[AA]`; the `END`-inside-open-`if` row `[AA, 01]`.
After the fix: 12 passed, 0 failed.

**Found on the way.** `tests/as_register_in_value_position.rs` closed its
`while` with `endw` and its `if` with `endc`; only the fallback made those
rows pass. asl refuses `endw` (`WHILE without ENDM`) and ACCEPTS `endc`
(probe `c_endc.asm`: `01 05`, exit 0). Sigil knows neither spelling. The rows
now use `endm` and `endif`. `endc` support is left open (below).

## Finding 2: dc.w, dc.l and the Z80 dw truncated silently (HIGH)

**Reproduction.** Base binary, exit 0 on each: `dc.w $12345` gives `23 45`;
`dc.w $10000` gives `00 00`; `dc.w -32769` gives `7F FF`; `dc.l $100000000`
gives `00 00 00 00`; `dc.l -$80000001` gives `7F FF FF FF`; Z80 `dw 74565`
gives `45 23`; Z80 `dw -32769` gives `FF 7F`; `org $20000 / L: dc.w L` gives
a 128 KiB image ending `00 00`. `dc.b` on the same values has always refused.

**asl.** `error #1320: range overflow`, exit 2, on every one of those. The
accepted window is signed floor to unsigned ceiling: one clean run (exit 0)
of `dc.w -1 / $FFFF / -32768 / 65535`, `dc.l -1 / $FFFFFFFF / -$80000000`,
`dc.w $7FFF` gives `FFFF FFFF 8000 FFFF FFFFFFFF FFFFFFFF 80000000 7FFF`;
Z80 `dw -1 / 65535 / -32768` gives `FFFF FFFF 0080`. (asl's Z80 mode rejects
`padding off` and `$`-hex, so the Z80 probes use decimal and no padding line.)

**Fix.** A `check_data_range` helper, called on the resolved (`Fold::Value`)
path of `directive_dw`, `directive_dc_w` and `directive_dc_l` with
`WORD_DATA_RANGE = -0x8000..=0xFFFF` and
`LONG_DATA_RANGE = -0x8000_0000..=0xFFFF_FFFF`. The low bytes are still
emitted after the error so the pass keeps its shape; the error fails the run.
Deferred (linker) paths untouched.

**Diagnostic** (CLI, exit 1), the sentence `dc.b` and the immediates already use:

```
w_bad1.asm(3): error: operand 74565 out of range -32768..=65535
```

**Red-first.** `as_dc_word_long_range` on `61dfc7a6`: 8 failed, 3 passed
(the three in-range acceptance rows, which pin asl's bytes above so a signed
negative in range is never refused). After the fix: 11 passed, 0 failed.

## Finding 3: rept ran unbounded (HIGH)

**Reproduction.** `x set 0 / rept 100000 / rept 100000 / x set x+1 / endr /
endr / dc.l x`: base binary still running at a 20 s timeout (exit 124). The
same with a comment as the only body line: same. `exec_rept` folded its
count once and looped; `exec_while` twelve screens below it checks both a
per-loop cap and a per-pass budget.

**asl.** Also runs both of those past a 30 s timeout, no output. That is a
finding about asl, recorded, not matched. Two things asl does that ARE
matched: the nested pair with NO body line at all finishes in 80 ms, exit 0
(`dc.b 1` after it emits `01`), and `rept 300 / rept 300 / x set x+1` gives
`dc.l x` = `00 01 5F 90`.

**Fix.** A per-pass `rept_budget` of `GLOBAL_REPT_CAP = 1_000_000` (the
`while` figure), drawn once per body run, so a nest of in-range counts is
bounded by what it actually executes. Exhausting it aborts the pass as the
`while` budget does, so an enclosing repeat cannot keep going. A body with no
lines runs zero iterations and costs nothing. Sizing is measured, not
guessed: the new `SIGIL_CENSUS_BUDGET=1` prints the per-pass draw, and one
pass over the corpus roots runs 434 (s1disasm), 572 (s2disasm), 540
(skdisasm) rept bodies; the largest literal count in the three corpora is 32.

**Diagnostic** (CLI, exit 1, 1.8 s):

```
r_nest2.asm(5): error: total `rept` body executions exceeded the per-pass budget (1000000): this `rept` (count 100000), with any repeat enclosing it, runs more bodies than the assembler will execute in one pass (asl runs such a nest without limit)
```

**Red-first.** `as_rept_budget` on `2f8146a0`: 4 failed, 1 passed. Three of
the four fail as `assembly did not terminate within 60s` (the bound the test
names; the assembly runs on a worker thread and the test waits on a channel),
the fourth because `rept 1000001 / dc.b 7 / endr` assembled to a megabyte
with exit 0. After the fix: 5 passed in 1.85 s.

## Finding 4: macro expansion capped in depth, not breadth (HIGH)

**Reproduction.** `m macro / m / m / endm / m / dc.b 1`: base binary still
running at 20 s. The depth cap (64) returns from each leaf and the caller
tries its next line, so two calls per body is 2^64 leaves.

**asl.** Silently wrong too: past a 30 s timeout, printing an ever-growing
`m_breadth.asm(7) m(1) m(1) ...` expansion trace and never a verdict.
Refusing is stricter than the reference, deliberately. The linear recursion
asl accepts, `cnt macro n / if n>0 / dc.b n / cnt n-1 / endif / endm / cnt 5`,
gives `05 04 03 02 01` and is the control.

**Fix.** A per-pass `macro_budget` of `GLOBAL_MACRO_CAP = 1_000_000`, one per
expansion; exhausting it aborts the pass. Measured against the corpus with
`SIGIL_CENSUS_BUDGET=1`: one pass expands 14,449 (s1disasm), 24,842
(s2disasm), 28,930 (skdisasm) macros, so the cap is over thirty times the
largest real program, and a self-doubling macro reaches it in about a second.
Two things fixed alongside because the witness exposed them: the expansion
diagnostics had no invocation span (an argument-less call reported line 1),
so `dispatch`'s line span is now threaded through `expand_macro` and both
refusals name the call line; and the depth-cap refusal fired once per LEAF,
919,281 identical lines before the budget line, so it is now raised once per
invocation position per pass (`expand_faults_seen`, the `cond_faults_seen`
pattern; the return that cuts the leaf is unconditional). No corpus root hits
the depth cap (0 such lines in all three base censuses).

**Diagnostic** (CLI, exit 1, 1.3 s, three stderr lines in total):

```
m_breadth.asm(4): error: macro `m` expansion too deep (recursive macro?)
m_breadth.asm(5): error: macro `m` expansion too deep (recursive macro?)
m_breadth.asm(4): error: macro `m` expansion exceeded the per-pass budget (1000000 expansions): a macro that calls itself more than once per body, or a repeat around such a call, never finishes inside the depth cap of 64 (asl runs this shape without limit)
```

**Red-first.** `as_macro_breadth_budget` on `33857754`: 1 failed as
`assembly did not terminate within 60s`, 2 passed (the linear control and
fifty thousand flat expansions under a `rept`). After the fix: 3 passed in
1.63 s.

## Byte neutrality and the aeon check

**Corpus census.** `sigil <root>` from each root's own directory, exactly as
the 2026-09-05 notes did: `s1disasm/sonic.asm`, `s2disasm/s2.asm`,
`skdisasm/sonic3k.asm`. The roots do not fully assemble at base (exit 1 with
50 / 5223 / 2448 diagnostic lines, the figures earlier notes report), so the
comparison is the diagnostic stream: after each of the four commits stderr is
byte-identical to base on all three roots (`diff` empty), and no new refusal
appears anywhere in the corpus. Script and outputs under
`/home/volence/sonic_hacks/.target-asfix/census.sh` and `corpus-out/{base,fix1..fix4}`.

**Aeon.** The three `.asm` files the engine routes through this front end
were read from `aeon` `origin/master`: `engine/debug/debugger.asm` (806
lines), `games/demo/game_root.asm` (43), `games/sonic4/game_root.asm` (50).
Per construct:

- unterminated blocks: `if`/`endif` balanced 36/36, 1/1, 1/1; debugger.asm's
  12 `macro` heads plus 3 `while` loops match its 15 `endm`; `switch`/`endcase`
  5/5; no `rept`, `irp`, `struct`. Nothing refused.
- `dc.w` / `dc.l` / `dw`: none in any of the three (zero lines). Nothing to range-check.
- `rept`: none. The budget cannot fire.
- macro self-recursion: the only two self-mentions (`Console`, `KDebug`) are
  comment text; the bodies call `_Console.ATTRIBUTE` / `_KDebug.ATTRIBUTE`,
  different macros. No macro calls itself. The budget cannot fire.

## Test totals

- `cargo test --release -p sigil-frontend-as`: **599 passed, 0 failed** across
  46 test binaries (lib 289, plus every integration binary). Clippy
  `--all-targets -D warnings`: clean, exit 0. rustfmt is not a gate in this
  repo (eval.rs carries 117 pre-existing hunks); the four new test files are
  rustfmt-clean where they landed formatted, and the three committed earlier
  were left as committed.
- `SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-cli --no-fail-fast`:
  **691 passed, 0 failed, 1 ignored** across 154 test binaries, no failing
  names. The harness's own banner on that run: `PARTIAL RUN (SIGIL_ALLOW_PARTIAL
  is set). No reference tree is named, so 128 test binaries are
  reference-dependent and every row in them is left UNMEASURED.` It skips
  rather than deriving the sibling checkout, so 26 binaries were measured and
  128 were vacuous greens; the controller's landing gate measures those. No
  aeon build was run.

## Open, and why

- **`endc` as an `if` closer.** asl accepts it (probe `c_endc.asm`, exit 0,
  `01 05`); sigil refuses it, now loudly (`if is never closed`), where before
  it dropped the last line silently. No corpus or aeon file writes `endc` or
  `endw` (grep over all four trees: 0). Teaching it means `closers_for`,
  `exec_if`'s arm scan, the closer-label guard, and two keyword tables; left
  for a parcel that measures it.
- **Deferred word cells are stricter than asl.** `sigil-link`'s `write_value`
  keeps a 16-bit link-expr cell strictly unsigned (0..65535) while a 32-bit
  cell takes the signed-or-unsigned union, so a `dc.w` whose value defers to
  the linker refuses a negative that the resolved path (and asl) accepts.
  Pre-existing, outside this parcel's byte-neutral scope, recorded here.
- **A closer after `END`.** `if 1 / dc.b 1 / end / endif` is accepted by sigil
  (the scan finds the `endif` past `END`) and refused by asl (`missing
  ENDIF`). Laxer than asl in a direction that drops no bytes; not pinned.

## What in the brief turned out wrong or imprecise

- "line ~5930" for finding 2 is the Z80 `dw` site (`directive_dw`); the
  `dc.w` and `dc.l` casts are at the two `directive_dc_*` functions below it.
  All three were fixed; the brief's "dc.w and dc.l" undercounted by one.
- "nested empty repeats run ten billion bodies": true of sigil, but asl
  finishes that exact shape in 80 ms because it short-circuits a body with no
  lines; asl only hangs once the body has a line. Sigil now matches the
  short-circuit and refuses the rest, so the review's reproduction is
  accepted by both assemblers and the budget is exercised by the one-line body.
- "`endw` if while uses the same path": asl's `while` closes on `endm` only;
  `endw` is an error in asl, so there is no `endw` path to cover. The one
  place `endw` appeared was a sigil test fixture relying on the fallback.
- Where asl itself is silently wrong (findings 3 and 4, both hangs with no
  verdict) the brief anticipated it; both are recorded above and not matched.
