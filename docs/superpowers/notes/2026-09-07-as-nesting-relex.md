# 2026-09-07: AS front end, a block body lexed once regardless of nesting depth

Parcel `parcel/as-nesting-relex`, base master `0a07b4ac`, one code commit
`c3a4c4ff` plus this note. Performance parcel: every emitted byte and every
diagnostic is identical before and after (census below). Finding
`sig-nesting-relex` (review seat, derived) said block bodies are re-lexed once
per enclosing level; the measurement here confirms the mechanism and doubles
the per-level factor the finding implied.

## Mechanism, measured

A temporary tally in `lex_into` (two process-global atomics, calls and bytes,
printed by the CLI under `SIGIL_LEX_TALLY`; applied by script to a probe build
and reverted, never committed) on a ladder of the same 4000-line body under
0 / 8 / 16 / 32 nested `if 1` blocks (release CLI, base binary
`9cfa4825b7b7a659bcd8ac71202ef8f9`):

| depth | lex calls | bytes scanned | calls per body line |
|---|---|---|---|
| 0 | 16012 | 380756 | 4 |
| 8 | 144348 | 3430004 | 36 |
| 16 | 273196 | 6485488 | 68 |
| 32 | 532428 | 12615056 | 133 |

Four lexes per body line at depth 0 are the floor: `exec` asks
`line_keyword` (one lex) and `exec_one` lexes the line again, in each of two
passes. Every enclosing block adds four more per pass pair: `block_end` calls
`find_block_end`, which walks every line of the body asking `line_keyword`,
and `exec_if` then walks the same body a second time collecting arm heads
(`elseif`/`else` at depth 0), again through `line_keyword`. Each ask reached
`dispatch_head_checked`, which lexed the whole line (operand included) from
the text. `rept`, `while`, `irp`, `switch`, `macro` and `struct` bodies all
reach `block_end` and pay the first walk the same way. The finding's
"+0.052 s per level, 52 percent of the depth-0 cost per level" understated the
mechanism: two walks per level, not one, so the per-level cost is close to
the depth-0 cost of the body (four lexes per line at depth 0, four more per
level).

## The fix

`SrcLine` carries a `HeadMemo`: the keyword `line_keyword` last answered for
that line, together with the `HeadKey` it answered under. The key is every
input of `dispatch_head` that is not the line itself: the CPU (`state.cpu`), a
stamp renewed on every insert into the macro table (`macros_gen`, because a
name at the head routes as an invocation only while the table holds it), and
the stamp of the innermost substituting macro frame (renewed by `shift`, which
is the only mutation of a frame's substitution inputs; 0 when no frame is
live or the innermost one is suspended for a `rept`/`while` replay, which is
exactly when `subst_frame_text` returns `None`). Stamps come from one
process-global atomic counter so a memo filled under one state can never
match another state's key, across passes (each pass is a fresh `Asm` and a
fresh split of the lines) or across assemblers; `SrcLine::clone` starts the
copy with an empty memo, since a macro body is cloned per expansion and a loop
body per capture. `line_keyword` answers from the memo when the key matches
and lexes otherwise, storing the keyword as `Rc<str>` so the remaining
per-level walk clones a refcount and not a `String` (with `String` the ladder
still rose from 0.011 to 0.018 s at depth 32; with `Rc<str>`, 0.012 s).
`dispatch_head`, `line_kw_args`, `head_label` and every other consumer of the
tokens are untouched: they run once per block head, not per body line.
Chosen over caching the token stream because the memoised value must be
cheap to hand back on every walk, and a `Vec<Token>` clone per line per level
would have halved the slope rather than removed it; chosen over memoising
`find_block_end`'s result because that result is a function of the same
state, so the key would be identical and the memo per line serves the arm
scan and `exec`'s own ask as well.

Block semantics are unchanged: the unterminated-block refusal, the `rept` and
`while` budgets, `endc`/`endif` handling and the pass structure all read the
keyword through the same function and get the same answer.

## Ladder, before and after

Release CLI, three runs per depth, image 14000 bytes md5 `e3f065054e6b` on
both arms at every depth, exit 0 everywhere.

| depth | base (load 4.5) | fix (load 3.1 to 4.7) | asl 1.42 Bld 212 (load 8.3) |
|---|---|---|---|
| 0 | 0.011 0.011 0.010 | 0.012 0.011 0.011 | 0.007 0.007 0.013 |
| 8 | 0.030 0.029 0.030 | 0.011 0.011 0.011 | 0.006 0.007 0.007 |
| 16 | 0.050 0.049 0.050 | 0.012 0.011 0.011 | 0.007 0.007 0.007 |
| 32 | 0.090 0.094 0.087 | 0.012 0.013 0.013 | 0.007 0.007 0.007 |

Base is linear at about +2.5 ms per level (8.2x at depth 32); fix is flat
within a millisecond. asl through `asl_ref.sh`'s `asl_run`, exit 0 on every
run. Lexer tally on the fixed probe build: 16012 / 16092 / 16172 / 16332
calls at depths 0 / 8 / 16 / 32, that is 10 extra lexes per level, all of them
the level's own `if 1` and `endif` lines (5 lexes per pass for the pair:
the opener's `exec` keyword ask, `line_kw_args_checked` and `head_label`,
the closer's keyword ask and its label binding; two passes); body lines stay
at 4 per line at every depth.
Bytes 380756 / 381748 / 382776 / 384856.

## s2disasm root

`sigil s2.asm` from `/home/volence/sonic_hacks/s2disasm`, five runs each,
exit 1 with 5240 stderr lines on every run (the root does not assemble at
base; see the census).

| arm | runs (s) | median | uptime load at start / end |
|---|---|---|---|
| base | 6.256 6.059 6.037 6.953 8.330 | 6.256 | 3.52 / 5.72 |
| fix | 6.474 5.252 5.170 5.143 6.549 | 5.252 | 8.38 / 8.80 |
| base control, re-run beside fix | 6.105 6.073 6.002 6.125 6.061 | 6.073 | 8.80 / 6.68 |

About 14 percent off the whole run; the best fix run is 5.14 s against the
best base run 6.00 s. The finding's "s2disasm heavy, 5.82 s against asl's
0.40 s (14.5x)" is therefore mostly NOT this mechanism: 5.2 s remain after it
is gone, and where they go was not profiled here (left open below).

## Byte census

`sigil <root>` from each root's own directory, base binary against fix
binary, stderr / stdout / exit compared byte for byte:

| root | exit | stderr lines | stdout | image |
|---|---|---|---|---|
| s1disasm/sonic.asm | 1 / 1 | 50 / 50 identical | empty / empty | none / none |
| s2disasm/s2.asm | 1 / 1 | 5240 / 5240 identical | empty / empty | none / none |
| skdisasm/sonic3k.asm | 1 / 1 | 2448 / 2448 identical | empty / empty | none / none |

No root emits an image at base (each refuses before the link), so the census
is the diagnostic stream, as in the 2026-09-05 and 2026-09-07 notes; the
s2disasm count is 5240 now where the silent-acceptance note reported 5223,
which is the base binary's count too (parcels landed since).

Aeon `origin/master` `ec640bcf70e263167223a33d987d4661ff221b7e`, exported
with `git archive` to a private tree, the three `.asm` files the engine
routes through this front end, each assembled standalone from the tree root
with both binaries, `-o` requested:

| file | lines | base | fix | verdict |
|---|---|---|---|---|
| games/demo/game_root.asm | 43 | exit 0, 0 stderr lines, image 0 bytes | same | identical |
| games/sonic4/game_root.asm | 50 | exit 0, 0 stderr lines, image 0 bytes | same | identical |
| engine/debug/debugger.asm | 806 | exit 1, 23 stderr lines, no image | same 23 lines | identical |

The game roots emit nothing standalone: their only include,
`engine/debug/debugger.asm`, is gated on `__MDDBG__`, which the engine's
build seeds through `Options::defines` along with harvested engine
constants. So a fourth arm, a carrier root per game (`cpu 68000`,
`__MDDBG__ equ 1`, `include` of the game root) that takes the include: exit 1
with one identical diagnostic on both arms for each game (`debugger.asm(29):
unresolvable equ DEBUGGER__EXTENSIONS__BTN_A_DEBUGGER`, a symbol the engine's
harvested defines seed). Standalone `debugger.asm`'s 23 diagnostics start with
`no processor declared` and then `trailing tokens in expression` at its `equ`
lines (53 to 122), the same on both arms. The engine-faithful proof of the
four ROM shapes is the controller's landing gate, as the brief says.

## Tests

`cargo test --release -p sigil-frontend-as` (target
`/home/volence/sonic_hacks/.target-relex`): 626 passed, 0 failed, 0 ignored
over 52 result lines, no failing names. `SIGIL_ALLOW_PARTIAL=1 cargo test
--release -p sigil-cli`: 702 passed, 0 failed, 1 ignored over 156 result
lines, exit 0; reference rows were unmeasured under that variable. `cargo
clippy -p sigil-frontend-as --all-targets -- -D warnings`: clean.

New test `eval::tests::a_block_body_is_lexed_once_regardless_of_nesting_depth`
(runner: `cargo test -p sigil-frontend-as
a_block_body_is_lexed_once_regardless_of_nesting_depth`), driven by a
`cfg(test)` thread-local lexer tally (`lexer::lex_tally`,
`lexer::reset_lex_tally`; zero cost outside test builds). It assembles a
400-line body at depth 0 and at depth 32, asserts the images are equal, and
bounds the extra lexes at 16 per level (176 bytes per level).

Red-first, mutation applied to the committed fix (the memo removed from
`line_keyword`, which is the base behaviour):

```
-        let key = self.head_key();
-        if let Some((memo_key, kw)) = &*line.head.0.borrow() {
-            if *memo_key == key {
-                return kw.clone();
-            }
-        }
-        let kw = self.dispatch_head(line).map(|(kw, _, _)| std::rc::Rc::from(kw));
-        *line.head.0.borrow_mut() = Some((key, kw.clone()));
-        kw
+        self.dispatch_head(line).map(|(kw, _, _)| std::rc::Rc::from(kw))
```

```
test eval::tests::a_block_body_is_lexed_once_regardless_of_nesting_depth ... FAILED
panicked at crates/sigil-frontend-as/src/eval.rs:15542:9:
32 enclosing blocks cost 55616 extra line lexes and 1036544 extra bytes over the
flat body's 1608 lexes / 31568 bytes: the body is being re-lexed per enclosing level
test result: FAILED. 0 passed; 1 failed
```

Restored with `git checkout -- crates/sigil-frontend-as/src/eval.rs` from the
committed baseline (`git status` clean), then green: 1 passed.

## Left open

- Where s2disasm's remaining 5.2 s go. Not profiled; the nesting mechanism
  was 14 percent of the root's time, not the 60 percent the aeon depth
  figure suggested for that tree, and aeon's own front-end time was not
  measured here (the engine's route needs the harness's harvested defines).
- `exec_one` lexes an executed line a second time after `exec`'s keyword ask
  (the 4-per-line floor is 2 lexes per pass). Sharing that lex would need the
  tokens, not the keyword, and `exec_one` lexes text that has been through
  `subst_name_braces`, so it is a different input; not attempted.
- The memo is per `SrcLine`, so an `include` executed twice or a macro body
  expanded twice re-lexes on the second copy. That is the depth-0 cost, not
  the depth-linear one, and this parcel did not measure it.

## What in the brief was wrong

- The per-level factor: the brief's model was one re-lex per enclosing level;
  it is two walks per level (closer search plus arm-head scan), each lexing
  every body line, in each pass.
- The premise that s2disasm's 14.5x against asl is this mechanism: it is
  about 14 percent of that run.
- The three aeon files cannot be given an engine-faithful before/after
  standalone: two are `__MDDBG__`-gated wrappers that emit nothing, and the
  third needs symbols the engine seeds. The comparison above states what each
  arm produced; the ROM-shape proof is the controller's.

Scripts and outputs lived under the private scratch
`/home/volence/sonic_hacks/.scratch-relex` (ladder generator, timing and
census runners, the probe patch, the mutation), deleted at the end of the
parcel as the brief required; every figure they produced is in this note.
