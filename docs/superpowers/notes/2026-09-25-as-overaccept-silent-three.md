# AS-OVERACCEPT-SILENT-THREE: three silent over-acceptances, measured and closed

Row `AS-OVERACCEPT-SILENT-THREE` of `docs/QUEUE.md`: rows `dir_padding_not_on_off`,
`expr_function_arg_count` and `save_missing_restore` of
`crates/sigil-frontend-as/tests/over_acceptance/ledger.txt`, the three over-acceptances the
over-acceptance gate (`docs/superpowers/notes/2026-09-10-over-acceptance-gate.md`) found on its
first run.

## Outcome

| row | before this parcel | after |
|---|---|---|
| `dir_padding_not_on_off` | accepted: every word but `off` read as ON, `supmode` the same | refused as asl refuses, both directives |
| `expr_function_arg_count` | ALREADY refused at the base: `check_call_args` has compared asl's argument count against the declared count, both directions, since `1b8cb9ae` (2026-09-11, one day after the row was written) | unchanged; row retired |
| `save_missing_restore` | accepted: the save stack was never checked at the end of the unit | refused, blamed on the open `save` line |

All three rows leave the ledger, so each probe is now a live tripwire:
`MIN_AGREED_REFUSALS` 51 to 54, and the gate's module doc counts move to 11 ledgered and 77
live tripwires.

## The oracle

Only `asl_run` from `docs/superpowers/notes/asl-reference/asl_ref.sh`, pinned build md5
`61e672562465725a8c102288a7da9098` (printed at the head of the probe run), invocation
`-xx -n -q -A -L -U -i .`, one construct per probe after `cpu 68000` / `org 0` and before
`dc.b 0` / `end`. Every refused probe was read for accept-or-refuse and the numbered class
only; no byte is quoted from any run. Every listing footer was `complete` except
`fn_zero_decl_bare` (`INCOMPLETE`, which is a refusal either way and is only context).

asl names no line for `#1460 missing RESTORE` (`INTERNAL: error #1460`); sigil names the
open `save`, which is the line that has to change. That is wording, not verdict.

## Probe table

60 probes. Columns: asl's verdict and class; sigil at the base (`93927682`, binary md5
`d8d0c33b144f448a134958b1493df3f9`); sigil at the tip of commit 2 (`3316e24f`, binary md5
`44e94a6bb51019ca0e0b796d48245066`) with its diagnostic; whether the tip agrees with asl.
Result: **0 disagreements after, 24 before**, and the 24 that changed are exactly the 24 that
disagreed. The `fn_*` rows did not change: they agreed at the base.

| probe | asl | sigil before | sigil after | after vs asl |
|---|---|---|---|---|
| `fn_fewer` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 2, this call passes 1 | agree |
| `fn_more` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 1, this call passes 2 | agree |
| `fn_more_in_equ` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 1, this call passes 2 | agree |
| `fn_more_in_if_false` | accept | accept | accept | agree |
| `fn_nested_more` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 1, this call passes 2 | agree |
| `fn_ok` | accept | accept | accept | agree |
| `fn_three_for_two` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 2, this call passes 3 | agree |
| `fn_two_none` | refuse (error #1490: wrong numbers of function arguments) | refuse | refuse: wrong number of function arguments: `F` takes 2, this call passes 0 | agree |
| `fn_two_ok` | accept | accept | accept | agree |
| `fn_zero_decl` | refuse (error #1110: wrong number of operands;error #1860: unknown function) | refuse | refuse: function needs `<params...>, <body>`; fn_zero_decl.asm(4):7: error: bad word expression | agree |
| `fn_zero_decl_bare` | refuse (error #1110: wrong number of operands) | refuse | refuse: function needs `<params...>, <body>` | agree |
| `fn_zero_decl_one` | refuse (error #1110: wrong number of operands;error #1860: unknown function) | refuse | refuse: function needs `<params...>, <body>`; fn_zero_decl_one.asm(4):7: error: bad word expression | agree |
| `pad_0` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `0` | agree |
| `pad_1` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `1` | agree |
| `pad_ON` | accept | accept | accept | agree |
| `pad_Off` | accept | accept | accept | agree |
| `pad_On` | accept | accept | accept | agree |
| `pad_comment` | accept | accept | accept | agree |
| `pad_empty` | refuse (error #1110: wrong number of operands) | accept | refuse: `padding` takes exactly one argument, ON or OFF, got 0 | agree |
| `pad_maybe` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `maybe` | agree |
| `pad_oFf` | accept | accept | accept | agree |
| `pad_off` | accept | accept | accept | agree |
| `pad_on` | accept | accept | accept | agree |
| `pad_onx` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `onx` | agree |
| `pad_paren` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `(on)` | agree |
| `pad_str` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `"on"` | agree |
| `pad_symdef` | accept | accept | accept | agree |
| `pad_two` | refuse (error #1110: wrong number of operands) | accept | refuse: `padding` takes exactly one argument, ON or OFF, got 2 | agree |
| `pad_undef` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `padding` accepts only ON or OFF, not `zzundef` | agree |
| `restore_only` | refuse (error #1450: RESTORE without SAVE) | refuse | refuse: `restore` with no matching `save` | agree |
| `save_in_if_false` | accept | accept | accept | agree |
| `save_in_include_only` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `save_in_include_restore_root` | accept | accept | accept | agree |
| `save_in_macro` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `save_locate` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `save_macro_balanced_twice` | accept | accept | accept | agree |
| `save_nested_balanced` | accept | accept | accept | agree |
| `save_no_end` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `save_only` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `save_restore` | accept | accept | accept | agree |
| `save_restore_twice` | refuse (error #1450: RESTORE without SAVE) | refuse | refuse: `restore` with no matching `save` | agree |
| `save_twice_one` | refuse (error #1460: missing RESTORE) | accept | refuse: `save` with no matching `restore` before the end of the source | agree |
| `sup_0` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `0` | agree |
| `sup_1` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `1` | agree |
| `sup_OFF` | accept | accept | accept | agree |
| `sup_ON` | accept | accept | accept | agree |
| `sup_Off2` | accept | accept | accept | agree |
| `sup_On` | accept | accept | accept | agree |
| `sup_comment` | accept | accept | accept | agree |
| `sup_empty` | refuse (error #1110: wrong number of operands) | accept | refuse: `supmode` takes exactly one argument, ON or OFF, got 0 | agree |
| `sup_maybe` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `maybe` | agree |
| `sup_oFf` | accept | accept | accept | agree |
| `sup_off` | accept | accept | accept | agree |
| `sup_on` | accept | accept | accept | agree |
| `sup_onx` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `onx` | agree |
| `sup_paren` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `(on)` | agree |
| `sup_str` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `"on"` | agree |
| `sup_symdef` | accept | accept | accept | agree |
| `sup_two` | refuse (error #1110: wrong number of operands) | accept | refuse: `supmode` takes exactly one argument, ON or OFF, got 2 | agree |
| `sup_undef` | refuse (error #1520: only ON/OFF allowed) | accept | refuse: `supmode` accepts only ON or OFF, not `zzundef` | agree |

### Probe bodies

Each body sits between `\tcpu 68000\n\torg 0\n` and `\tdc.b 0\n\tend\n`. Three probes are
written whole: `save_no_end` (`cpu 68000`, `org 0`, `save`, `dc.b 0`, no `end`),
`save_in_include_only` (root includes `inc_save.inc`, which holds one `save`) and
`save_in_include_restore_root` (the same include, then `restore` in the root).

| probe | body |
|---|---|
| `pad_maybe` | `\tpadding maybe\n` |
| `pad_on` | `\tpadding on\n` |
| `pad_off` | `\tpadding off\n` |
| `pad_ON` | `\tpadding ON\n` |
| `pad_Off` | `\tpadding Off\n` |
| `pad_empty` | `\tpadding\n` |
| `pad_1` | `\tpadding 1\n` |
| `pad_0` | `\tpadding 0\n` |
| `pad_str` | `\tpadding "on"\n` |
| `pad_two` | `\tpadding on,off\n` |
| `pad_symdef` | `ON equ 5\n\tpadding on\n` |
| `pad_onx` | `\tpadding onx\n` |
| `pad_paren` | `\tpadding (on)\n` |
| `pad_undef` | `\tpadding zzundef\n` |
| `sup_maybe` | `\tsupmode maybe\n` |
| `sup_on` | `\tsupmode on\n` |
| `sup_off` | `\tsupmode off\n` |
| `sup_OFF` | `\tsupmode OFF\n` |
| `sup_empty` | `\tsupmode\n` |
| `sup_1` | `\tsupmode 1\n` |
| `sup_two` | `\tsupmode on,off\n` |
| `fn_ok` | `F\tfunction x,x+1\n\tdc.w F(1)\n` |
| `fn_more` | `F\tfunction x,x+1\n\tdc.w F(1,2)\n` |
| `fn_fewer` | `F\tfunction x,y,x+y\n\tdc.w F(1)\n` |
| `fn_zero_decl` | `F\tfunction 7\n\tdc.w F()\n` |
| `fn_zero_decl_one` | `F\tfunction 7\n\tdc.w F(1)\n` |
| `fn_zero_decl_bare` | `F\tfunction 7\n\tdc.w F\n` |
| `fn_two_none` | `F\tfunction x,y,x+y\n\tdc.w F()\n` |
| `fn_two_ok` | `F\tfunction x,y,x+y\n\tdc.w F(1,2)\n` |
| `fn_nested_more` | `F\tfunction x,x+1\n\tdc.w F(F(1),2)\n` |
| `fn_three_for_two` | `F\tfunction x,y,x+y\n\tdc.w F(1,2,3)\n` |
| `fn_more_in_if_false` | `F\tfunction x,x+1\n\tif 0\n\tdc.w F(1,2)\n\tendif\n` |
| `fn_more_in_equ` | `F\tfunction x,x+1\nV\tequ F(1,2)\n` |
| `save_only` | `\tsave\n` |
| `save_restore` | `\tsave\n\trestore\n` |
| `restore_only` | `\trestore\n` |
| `save_twice_one` | `\tsave\n\tsave\n\trestore\n` |
| `save_in_if_false` | `\tif 0\n\tsave\n\tendif\n` |
| `save_in_macro` | `M\tmacro\n\tsave\n\tendm\n\tM\n` |
| `save_restore_twice` | `\tsave\n\trestore\n\trestore\n` |
| `pad_On` | `\tpadding On\n` |
| `pad_oFf` | `\tpadding oFf\n` |
| `pad_comment` | `\tpadding off ; a trailing comment\n` |
| `sup_On` | `\tsupmode On\n` |
| `sup_oFf` | `\tsupmode oFf\n` |
| `sup_ON` | `\tsupmode ON\n` |
| `sup_Off2` | `\tsupmode Off\n` |
| `sup_onx` | `\tsupmode onx\n` |
| `sup_0` | `\tsupmode 0\n` |
| `sup_paren` | `\tsupmode (on)\n` |
| `sup_str` | `\tsupmode "on"\n` |
| `sup_undef` | `\tsupmode zzundef\n` |
| `sup_symdef` | `ON equ 5\n\tsupmode on\n` |
| `sup_comment` | `\tsupmode off ; a trailing comment\n` |
| `save_nested_balanced` | `\tsave\n\tsave\n\trestore\n\trestore\n` |
| `save_macro_balanced_twice` | `M\tmacro\n\tsave\n\trestore\n\tendm\n\tM\n\tM\n` |
| `save_locate` | `\tnop\n\tsave\n\tsave\n\trestore\n` |

## What asl accepts, per directive

- **`padding` and `supmode`** take exactly one argument, and it is the KEYWORD `on` or `off` in
  any case. Not an expression: `1`, `0`, `(on)`, `"on"` and an undefined name are all `#1520 only
  ON/OFF allowed`, and `ON equ 5` in scope does not stop `padding on` being the keyword. Zero
  arguments and `on,off` are `#1110 wrong number of operands`. The two directives answer every
  probe identically. They were the only two callers of the old helper.
- **User `function` arity** is checked both ways: more arguments (`fn_more`, `fn_three_for_two`,
  `fn_nested_more`, `fn_more_in_equ`) and fewer (`fn_fewer`, `fn_two_none`) are `#1490`; a call
  inside a false `if` is not evaluated and is accepted. A zero-parameter `function` cannot be
  declared at all (`F function 7` is `#1110` at the declaration); sigil also refuses that
  declaration (`function needs <params...>, <body>`), so it agrees.
- **`save` / `restore`** is one stack for the whole UNIT, counted over executed lines only. An
  open `save` at the end is `#1460 missing RESTORE` wherever it came from (root, macro body,
  include, a missing `end`); a `save` in an include closed by the root is fine; a `save` in a
  false `if` is fine. `restore` with nothing to pop is `#1450 RESTORE without SAVE`, which
  sigil already refused and still does.

## The change

`crates/sigil-frontend-as/src/eval.rs`:

- `Asm::on_off_arg(name, rest, span) -> Option<bool>` replaces the free `on_off`. A wrong count
  reports ``` `padding` takes exactly one argument, ON or OFF, got N ``` at the directive; any
  argument that is not one identifier folding to `on`/`off` reports
  ``` `padding` accepts only ON or OFF, not `maybe` ``` at the argument. On a refusal the flag is
  left as it was.
- `Asm::save_sites` records the line of each `save`, pushed and popped in the same dispatch arms
  that push and pop `AsmState`'s snapshot. `Asm::report_unrestored_saves`, called at the end of
  every pass beside `report_unpopped_value_stacks`, reports each still-open `save` as
  ``` `save` with no matching `restore` before the end of the source ``` on that `save` line.
  `state.rs` is untouched, deliberately: the save snapshot is where a code-page change would
  live, and another lane is editing that.

Rendered, all in the AS shape: `pad_maybe.asm(3):10: error: ...`,
`save_only.asm(3):2: error: ...`, `inc_save.inc(1):2: error: ...` for the include case, and
`save_in_macro.asm(6) M(1):2: error: ...` for the macro case. No em or en dash in any message.

## Blast radius

Greps (`blastgrep.sh` in the parcel scratch), over `.asm .inc .s .z80 .a68` files,
case-insensitive, comment stripped, directive position (after a label or leading whitespace).
The positive controls are the same pattern shape on `org` and `dc.b`, which every tree spells.

| pattern | s1disasm | s2disasm | skdisasm | aeon `.aeon-overaccept` (`ec640bcf`) |
|---|---|---|---|---|
| control `org` | 4 | 5 | 11 | 0 |
| control `dc.b` | 2637 | 5640 | 38240 | 15 |
| `padding` | 3 | 3 | 5 | 3 |
| `supmode` | 1 | 1 | 1 | 3 |
| `save` | 2 | 2 | 7 | 0 |
| `restore` | 2 | 2 | 7 | 0 |
| `function` definition | 25 | 49 | 42 | 0 |

- Every `padding` site in all four trees is `padding off` and every `supmode` site is
  `supmode on`: none newly refuses.
- The corpus `save`/`restore` sites pair one to one inside each file (s1 `sound/z80.asm` 8/235,
  `sonic.asm` 322/353; s2 `s2.asm` 318/348 and 90858/90860; skdisasm `Sound/Z80 Sound
  Driver.asm` 225/4433 and 4441/5313, `sonic3k.asm` 226/257 and 9950/9995, `s3.asm` 8352/8393,
  `sonic3k.macros.asm` 132/135 inside `levselstr` and 139/149 at top level). The full suite's
  corpus gates assemble all three with the new check live, and pass.
- Arity code is unchanged, so no `function` site can newly refuse.
- Aeon `.emp` sources: no line starts with `padding`, `supmode`, `save` or `restore` (control:
  121 `.emp` files matched `fn`/`const`/`import` at line start with the same grep).

No site in any corpus or in aeon newly refuses.

## Four-shape identity, and what it attests

Built in `/home/volence/sonic_hacks/.aeon-overaccept` (aeon `ec640bcf`, provisioned by
`scripts/provision-aeon-ref.sh`, witness `repin --check` printed `pins.rs unchanged`), once
with the base pair (sigil `93927682` md5 `d8d0c33b...`, emit md5 `8cc8ca93...`) and once with
the tip pair (sigil `3316e24f` md5 `44e94a6b...`, emit md5 `083e1370...`), via `SIGIL_BUILD` /
`SIGIL_EMIT`:

| shape | before | after | `cmp` |
|---|---|---|---|
| `s4.bin` | `91c46c94/820209` | `91c46c94/820209` | identical |
| `s4.debug.bin` | `8a378de6/846509` | `8a378de6/846509` | identical |
| `demo.bin` | `1c7a34d3/96863` | `1c7a34d3/96863` | identical |
| `demo.debug.bin` | `72e405a5/103185` | `72e405a5/103185` | identical |

All eight equal the golden CRCs in `provenance.toml`'s tail. Reachability: aeon DOES reach
`on_off_arg` (`padding off` and `supmode on` in `games/sonic4/game_root.asm`,
`games/demo/game_root.asm` and `engine/debug/debugger.asm`), and every one takes the accept path with the same value as before, so the
identity attests that path. `report_unrestored_saves` runs on every pass with an empty stack
(aeon has no `save`), so for it the identity attests only that nothing else moved. The arity
path is unchanged code.

## Red-first proofs

Each mutation applied to `eval.rs` at the committed tip, shown on disk by `git diff`, run,
then `eval.rs` restored from `git show HEAD:` and the diff confirmed empty
(`mutate.py` in the parcel scratch). The gate is run with `--test-threads=1`, because under
the parallel runner its failure text is lost (gap ledger, this parcel's last row).

- **M1, the old rule back** (`+ return Some(!matches!(... fold_kw(w) == "off"));` at the top of
  `on_off_arg`): `a_switch_refuses_every_argument_but_on_or_off` red, ``7 of 7 shapes asl
  refuses were not refused naming "`padding` accepts only ON or OFF": "\tpadding maybe\n" ->
  Ok(())``; gate `no_unledgered_over_acceptance` red, `sigil ACCEPTS 1 program(s) asl REFUSES
  that are not in the ledger: dir_padding_not_on_off  asl refuses: #1520 only ON/OFF allowed`;
  `feed_control_the_ledger_is_not_an_escape_hatch` red, `only 53 probes are refused by BOTH
  assemblers, below the floor of 54`.
- **M2, `supmode` unchecked** (`- if let Some(v) = self.on_off_arg("supmode", rest, span) {` /
  `+ if let Some(v) = Some(true) {`): the same test red, ``7 of 7 ... naming "`supmode` accepts
  only ON or OFF": "\tsupmode maybe\n" -> Ok(())``. The gate stays GREEN here: it has no
  `supmode` probe (gap ledger).
- **M3, no end-of-unit check** (`- asm.report_unrestored_saves();`):
  `a_save_left_open_at_the_end_of_the_source_is_refused` red, ``3 of 3 shapes asl refuses were
  not refused naming "`save` with no matching `restore`": "\tsave\n" -> Ok(())``;
  `the_open_save_refusal_names_the_save_line` red, `an open `save` must be refused`; gate red,
  `sigil ACCEPTS 1 program(s) ... save_missing_restore  asl refuses: #1460 missing RESTORE`, and
  the floor red at 53.
- **M4, arity check off** (`+ if false && passed != params.len() {`): the pre-existing
  `as_empty_parens::a_user_function_counts_its_arguments_the_way_asl_does` red, `3 of 3
  refusals were not refused with a diagnostic naming `wrong number of function arguments``;
  gate red, `sigil ACCEPTS 1 program(s) ... expr_function_arg_count  asl refuses: #1490 wrong
  numbers of function arguments`, floor red at 53. This is the proof that retiring the row
  made the probe a live tripwire.

Each red names its own mechanism: M1/M2 the ON/OFF message, M3 the `save` message and
`#1460`, M4 the arity message and `#1490`; no mutation reddened a neighbouring refusal.

## Tests

- New: `crates/sigil-frontend-as/tests/as_on_off_save_balance.rs`, five tests, run by
  `cargo test -p sigil-frontend-as` (integration test target `as_on_off_save_balance`). Every
  accept and every refuse in it is one of the asl probes above.
- Gate: `tests/as_over_acceptance.rs`, three ledger rows retired, `MIN_AGREED_REFUSALS` 54.

### Corpus assembly, direct

The suite's whole-ROM gates assemble Sonic 1 (`as_switch_setting_roms`, two variants) and
Sonic 2 (`as_sonic2_whole_rom`) to their reference ROMs with the new check live, and pass.
Sonic 3 & Knuckles is not assemblable by sigil yet (`codepage` and others), so its
`save`/`restore` sites are not reached through a passing build. As a direct control, each
disassembly root (`sonic.asm`, `s2.asm`, `sonic3k.asm`, `s3.asm`) was run through the base and
the tip binary with no build setup (`corpus_roots.sh` in the parcel scratch): every root fails
the same way under both (layout for s1, front end for the others, for reasons unrelated to this
parcel) and the diagnostic streams are byte-identical before and after, so no root gains a
`save` or ON/OFF refusal.

## Suite

`cargo test --release --workspace --no-fail-fast`, `CARGO_TARGET_DIR` the parcel scratch
target, `AEON_DIR=/home/volence/sonic_hacks/.aeon-overaccept`, stamped
`HEAD=3316e24f` on `parcel/as-overaccept-silent-three` (tree dirty only in
`campaign-gap-ledger.md`): **5615 passed, 1 failed, 2 ignored** across 497 result lines.

The one failure is `sigil-harness --test m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`,
a precondition refusal and not a verdict: `NO REFERENCE TREE IS NAMED, so this run can measure
nothing it could attribute, and STOPS` (it declined the derived `oracle-old` checkout). Re-run
at the same tip with `ORACLE_DIR=/home/volence/sonic_hacks/oracle-old` named, `m1b_gate` is
5 passed, 0 failed. Nothing in it touches the ON/OFF or save code.

`cargo clippy --release --workspace --all-targets -- -D warnings` at `3316e24f`: clean
(exit 0).

## Open items

- The gate has no `supmode` probe and no include-held open `save` probe; `supmode`'s refusal
  is guarded only by `as_on_off_save_balance.rs` (mutation M2 leaves the gate green). Booked in
  the gap ledger.
- `listing` vocabulary is still unchecked (asl `#1520` on `listing zqp_bogus`), a documented
  choice in `directive_listing_control`, not a caller of the helper. Booked.
- The gate's failure text is lost under libtest's default parallel runner, because
  `sigil_verdict` swaps the process-wide panic hook. Booked with a kill condition.

## Where the brief was wrong

- Hypothesis 2 was already false at the base: `expr_function_arg_count` was not an
  over-acceptance any more. Arity is checked, both more and fewer, since `1b8cb9ae`
  (2026-09-11). The row survived in the ledger because a now-agreeing row is reported, not
  failed. The work for it was retirement plus proving the probe is now a tripwire (M4).
- "Find every caller of that helper": there were exactly two (`padding`, `supmode`), as the
  brief supposed.
