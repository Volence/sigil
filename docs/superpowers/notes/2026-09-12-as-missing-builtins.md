# AS-MISSING-BUILTINS: asl's integer-valued builtins, and the empty argument

Row `AS-MISSING-BUILTINS`, project SIGIL-AS-REPLACEMENT. Branch `parcel/as-missing-builtins`, off
master `19e12c99`. Evidence beside this note in `2026-09-12-as-missing-builtins/`: `probes/` (the
files the tests read: source, listing and `asl_run` verdict), `census/` (every probe round as TSV
rows), `scripts/` (generators, runner, model fit).

**Oracle.** `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`,
through `asl_ref.sh`'s `asl_run`, `-xx -n -q -A -L -U -i .`. Every value below is from a run that
exited 0 with `ASL_DIAG=complete`; a refusal is read for its verdict and diagnostic, never a byte.
Round 1 ran each of its 684 value probes twice, bare and behind an accepted `move.w #$1234,d0`, so
a line asl declined and filled from its last result would disagree between the two: all 684
agreed.

**sigil binaries.** Before: master `19e12c99`, md5 `bc26d20aa3fe64ca44a68b367fcdfe6e`. After:
`786e18c8`, the last commit to change `eval.rs` (every later one touches only test files and
docs), md5 `8d2c6f168113645388c816dbdc4fc433`.

## What changed

- **An empty builtin argument is the integer 0, for every numeric builtin.** It was 0 for
  `lastbit` alone, so `abs()`, `int()` and every float builtin's empty call were refused.
- **Six builtins:** `firstbit`, `bitcnt`, `bitpos`, `toupper` and `tolower` as rows of a new
  `INT_BUILTINS` table (integer-only; a float argument is refused once, in
  `apply_num_builtin`), and `sgn` by name beside `int` and `abs` (it takes either type and answers
  an integer). `lastbit` moved into the table unchanged; `as_lastbit.rs` is untouched and green.
- `INT_BUILTINS` follows `FLOAT_BUILTINS`: one list, read by both `is_num_builtin` and
  `apply_num_builtin`, so a name cannot be recognized by one and unknown to the other.

Files: `crates/sigil-frontend-as/src/eval.rs`, `crates/sigil-frontend-as/tests/as_int_builtins.rs`
(new), a comment in `crates/sigil-frontend-as/tests/as_lastbit.rs`.

Commits, in git order, with the labels this note uses:

| label | commit | what |
|---|---|---|
| C0 | `1cf097a4` | the evidence set: probes, census, scripts |
| C1 | `8b7fcae6` | the empty argument is 0 for every builtin; `INT_BUILTINS`, with `lastbit` |
| C2 | `8109ff82` | `firstbit` |
| C3 | `22936a0a` | `bitcnt` |
| C4 | `e86acb99` | `bitpos` |
| C5 | `07175a10` | `sgn`, and the `signedToString` probes |
| C6 | `786e18c8` | `toupper`, `tolower`, and the name tests |
| C7 | `9a617643` | the `as_lastbit.rs` half-fix comment |
| docs | `ff84a9d6` | the gap ledger, the 09-11 amendment, the mutation logs and tooling |
| C8 | `dab84539` | the test file made clippy-clean |

This note, and the two logs of the re-proof at the code tip, are the commit after these.

## The census

"Before" and "after" are the two binaries above, on the committed probes.

| builtin | asl (exit status) | sigil before | sigil after |
|---|---|---|---|
| `sgn(x)` | integer: the sign of the 64-bit value (`t_sgn`, 1,862 values, exit 0). Float: accepted, and the answer is an INTEGER: `dc.l sgn(-2.5)` `FFFF FFFF`, `sgn(-0.0)` 0, `sgn(0.1-0.2)` -1 (exit 0). Empty: 0. String: `#10000` abort (exit 3). Two arguments `#1490`, undefined `#1010` (exit 2) | refused in every context (`bad long expression`) | matches every clean probe; refuses every refusal |
| `bitcnt(x)` | set bits of the 64-bit two's complement: `bitcnt(-$1)` 64, `bitcnt(-$80)` 57 (`t_bitcnt`, exit 0). Empty 0. Float `#10000` (exit 3); string `#1136`; two arguments `#1490`; undefined `#1010` | refused | matches; refuses |
| `firstbit(x)` | NOT the lowest set bit: for an even `x` it is; for an odd `x` it is the lowest set bit of `x>>1`, -1 when that is 0. `firstbit($1)` -1, `firstbit($5)` 1, `firstbit($161)` 4, `firstbit($C)` 2 (`t_firstbit`, exit 0). Empty -1. Float `#10000`; string `#1136` | refused | matches; refuses |
| `bitpos(x)` | the bit index of a POSITIVE power of two (`t_bitpos`, 2^0..2^62, exit 0); anything else `#1540 not exactly one bit set` (exit 2): 0, 6, -1, `$FFFEB`, the empty call, `(-$7FFFFFFFFFFFFFFF-1)`, and a forward reference. Float `#10000` | refused | matches; refuses, except a forward `equ` (open, below) |
| `toupper(x)`, `tolower(x)` | a code 0..255, ASCII letters mapped, `$80..$FF` unchanged (`t_toupper`, `t_tolower`, every code, exit 0); outside 0..255 `#1320 range overflow` (exit 2). Float `#10000`; string `#1136` | refused | matches; refuses |
| empty argument | 0 for every builtin: `dc.l abs()` and `dc.l int()` `0000 0000`, `INT(cos())` 1, `INT(exp())` 1; `INT(log())` `#1870`, `INT(acosh())` `#1880`, bare `dc.l sin()` `#1133` (`empty`, `ref_empty_*`) | `lastbit()` only | matches |
| `int`, `abs`, `lastbit` | built before this parcel | unchanged | unchanged; `t_abs` and `t_lastbit` (1,862 values each) match |
| a character constant as the argument | refused: `'a'` is a STRING to a builtin, and so are `'a'+0` and `Q equ 'a'`: `#1136` (`lastbit`, `bitcnt`, `firstbit`, `bitpos`, `toupper`, `tolower`) or an abort (`abs`, `int`, `sgn`) | `lastbit('a')` 6, `abs('a')` `$61`, `int('a')` `$61` | the same, and every new builtin but `bitpos` answers too (`sgn('a')` 1, `toupper('a')` `$41`): OPEN |
| `m5d_default_2p` (`macro pa=2p.bin`) | the default is raw text: `"2p.bin"`, `"2p bin"`, `"2p&q"`, `"1x2"` (`m_default_*`, exit 0) | `macro needs a name` | unchanged: OPEN |

**Names.** A user `function` spelled like a builtin, in the same case, wins (asl `65` for
`sgn function x,x+100` / `sgn(1)`, and for all nine names). One spelled in capitals does not
shadow the lower-case builtin under `-U`. A symbol spelled like a builtin is the symbol when bare
and the builtin in call shape (`names_userfn`, `names_userfn_upper`, `names_symbol`). sigil's
order already did this; the tests pin it.

## `firstbit` is measured, not read off its name

The lowest-set-bit reading is what the name suggests, and what `as_lastbit.rs`'s half-fix table
assumed. asl disagrees on odd values. The rule that fits is: for an odd `x`, shift it right once;
then -1 if nothing is left, else the index of the lowest set bit. It fits all 1,862 rows of
`t_firstbit` (`scripts/fit.py`); the plain lowest-set-bit model misses 452 of them. The table is
exhaustive over 0..1023 and -1..-256, every 2^k, 2^k-1, 2^k+1 and 3*2^k, and 400 seeded random and
sparse 64-bit values.

## Method

- One construct per file for every refusal, so its verdict is attributable. A probe that must be
  clean may hold many constructs: a clean run makes every line a value source.
- Rounds: 1,368 probes (round 1, every builtin by every edge case, bare and behind a preamble),
  89 (round 2, the tables and one-construct follow-ups), 57 (round 3, names and macro defaults),
  82 (the committed evidence set), 15 (round 5, Sonic 1's `signedToString` taken apart), 2
  (round 6, its `sgn` half).
- The tests rebuild the expected image from asl's listing, continuation rows included, and read a
  listing only when the probe's recorded `ASL_EXIT` is 0; a refusal test requires a non-zero
  recorded exit. So no expected byte is copied by hand.

## Tests, and each one proven red first

`crates/sigil-frontend-as/tests/as_int_builtins.rs`, its own test binary (`--test
as_int_builtins`), 22 tests. Each mutation was applied to the COMMITTED tree by
`scripts/mutproof.sh`, which refuses a dirty tree, has `scripts/mutate.py` refuse an old text
that does not occur exactly once and quote the mutated lines back from disk, shows `git diff
--stat`, runs the binary, and restores the file with `git show HEAD:<path> > <path>`; every run
ended `tracked changes now: 0`. The mutation texts (`<label>.old` / `.new`) and each run's whole
log (`red-<label>.log`) are in `mutations/`.

| commit | mutation (line as read back from disk) | went red | first failing assertion |
|---|---|---|---|
| C1 | `if arg.is_empty() && false {` | `an_empty_argument_is_zero_for_every_builtin` | `empty: asl assembles it, sigil refused: ["int(): could not evaluate float expression", ...]` |
| C1 | `return Some(Num::Int(1));` | both empty-argument tests | `ref_empty_log: asl refuses it, sigil built [00, 00, 00, 00, EE]` |
| C2 | `// ("firstbit", firstbit),` | firstbit table + contexts | `ctx_firstbit: asl assembles it, sigil refused: ["bad byte expression", ...]` |
| C2 | `... i64::from(x.trailing_zeros()) + 1 })` in `firstbit` | firstbit table + contexts | `ctx_firstbit: sigil's image differs from asl's listing at $0`: asl `06`, sigil `07` |
| C2 | `let x = x;` (the plain lowest set bit) | firstbit table | `t_firstbit: ... at $4`: asl `FF FF FF FF`, sigil `00 00 00 00` |
| C2 | `Num::Float(v) => f(v as i64).map(Num::Int),` | firstbit refusals | `ref_firstbit_float: asl refuses it, sigil built [00, 00, 00, 01, EE]` |
| C3 | `// ("bitcnt", bitcnt),` | bitcnt table + contexts | `ctx_bitcnt: asl assembles it, sigil refused: [...]` |
| C3 | `Some(i64::from(x.count_ones()) + 1)` | bitcnt table + contexts | `ctx_bitcnt: sigil's image differs from asl's listing at $0` |
| C3 | `Some(i64::from((x as u32).count_ones()))` | bitcnt table only | `t_bitcnt: ... at $1003` (a negative row) |
| C3 | the float guess | firstbit + bitcnt refusals | `ref_bitcnt_float: asl refuses it, sigil built [00, 00, 00, 02, EE]` |
| C4 | `// ("bitpos", bitpos),` | bitpos table + contexts | `ctx_bitpos: asl assembles it, sigil refused: [...]` |
| C4 | `... i64::from(x.trailing_zeros()) + 1)` in `bitpos` | bitpos table + contexts | `ctx_bitpos: asl assembles it, sigil refused: ["bad byte expression"]` |
| C4 | `(x != 0).then(...)` (lowest set bit of anything) | bitpos refusals | `ref_bitpos_6: asl refuses it, sigil built [00, 00, 00, 01, EE]` |
| C4 | `(x != 0 && x & x.wrapping_sub(1) == 0)` (drops `x > 0`) | bitpos refusals | `ref_bitpos_min: asl refuses it, sigil built [00, 00, 00, 3F, EE]` |
| C4 | the float guess | three refusal tests | `ref_bitcnt_float: ...`; bitpos's own is caught by `ref_bitpos_floatsym` (2.5 guessed as 2 answers 1) |
| C5 | `// \|\| name.eq_ignore_ascii_case("sgn")` | all four sgn value tests | `signed_sgn: asl assembles it, sigil refused: [...]` |
| C5 | `Num::Int(i) => i.signum() + 1,` | three sgn value tests | `signed_sgn: sigil's image differs from asl's listing at $2` |
| C5 | `Num::Float(_) => return None,` (sgn as integer-only) | both float tests | `ctx_sgn_float: asl assembles it, sigil refused: ["floating point value where an integer is required ..."]` |
| C6 | `// ("toupper", toupper),` and `// ("tolower", tolower),` | tables, contexts, call-shape names | `names_userfn_upper: asl assembles it, sigil refused: ["bad byte expression"]` |
| C6 | `... to_ascii_uppercase()) + 1)`, `... to_ascii_lowercase()) + 1)` | tables, contexts, names | `names_userfn_upper: ... at $4` / `at $5` |
| C6 | `u8::try_from(x & 0xFF)` (the range half-fix) | toupper/tolower refusals | `ref_toupper_256: asl refuses it, sigil built [00, EE]` |
| C6 | the float guess | all four integer-only refusal tests | `ref_bitpos_floatsym: asl refuses it, sigil built [00, 00, 00, 01, EE]` |
| C6 | user functions never win over a builtin name | `a_user_function_spelled_like_a_builtin_wins` | `names_userfn: ... at $0` |
| C6 | user-function lookup case-insensitive | `a_builtin_name_is_the_builtin_only_in_call_shape` | `names_userfn_upper: ... at $0` |

The brief's three mutations map onto this design as follows. "Remove the `is_num_builtin` arm" is
removing the table row for the five `INT_BUILTINS` names, because `is_num_builtin` reads the table;
for `sgn` it is the literal arm. "Swap the float refusal for a guess" is one shared branch for the
integer-only family; `sgn` ACCEPTS a float, so its mutation is the inverse, refusing one.

**One proof was not first-time clean.** C2's off-by-one mutation did not apply on its first run:
its old text named the doc comment that used to follow `firstbit`, and `mutate.py` refused
(`old text occurs 0 times`, `MUTATION FAILED TO APPLY`). It was rewritten on `firstbit`'s own two
lines and run at C3's tip `22936a0a`, whose `firstbit` is C2's unchanged, and went red there.

## Open, and why

Each is in the gap ledger under this note's date.

- **`AS-STRING-PLUS-CONCAT`: silent wrong bytes, pre-existing, and the reason Sonic 1's
  `signedToString` still does not assemble.** The brief expected `sgn` to be the gap. Taken apart
  (probes `v_*`), `sgn`'s half assembles (`signed_sgn`, tested). What fails is asl's `+` on two
  strings, which CONCATENATES: `dc.b "-"+"x"` is `2D 78` in asl and `A5` in sigil (the codes
  added), and `f function number,"a"+"b"` / `dc.b f(1)` is `61 62` against `C3`. Both exit 0, on
  master and on this branch. With an interpolated operand sigil refuses instead.
- **`AS-FUNCTION-BODY-INTERPOLATION`: silent wrong bytes, pre-existing.**
  `f function number,"$\{abs(number)}"` / `dc.b f(-5)` is `24 35` in asl and the fifteen bytes of
  the source text `$\{abs(number)}` in sigil, exit 0.
- **`AS-CHAR-CONSTANT-BUILTIN-ARG`: silent accept-more.** sigil's lexer folds a character constant
  into `Tok::Int` at lex time, so the typed evaluator cannot tell `'a'` from `97`, and a symbol set
  from one carries no type either. Matching asl needs a character-constant token (or a string type
  on `equ` symbols): a lexer change that reaches every consumer of `Tok::Int`, so the brief's escape
  hatch applies. Before this parcel `lastbit`, `abs` and `int` already answered; the new builtins
  inherit it.
- **`AS-BITPOS-FORWARD-EQU`: silent accept-more.** asl refuses `bitpos` of a forward reference on
  pass 1, where the symbol is still 0. sigil refuses a forward label the same way (its first look
  gives 0) but answers a forward `equ`: `dc.b bitpos(Later)` / `Later equ 8` is `03`. No corpus
  writes `bitpos`.
- **`AS-STRING-FUNCTION-SET`** (loud): `S set f(-5)` for a string-valued user function, then
  `dc.b S`: asl `2D 24 35`, sigil `unresolved symbol S`.
- **`AS-MACRO-DEFAULT-UNLEXABLE`** (loud, not attempted): `capture_macro` lexes the whole macro head
  as one token line, so a default that does not lex (`2p.bin`, a digit-led word) fails the entire
  head and sigil reports `macro needs a name`, which names the wrong thing. asl takes a default as
  raw text, spaces included (`"2p bin"`). The fix is a text-level parameter-list splitter that
  still respects quotes, `{INTLABEL}` groups and parentheses, replacing how every macro head in the
  engine's build path is parsed; that is a parser rewrite with its own byte-identity risk, not a
  builtin, so it is ledgered rather than folded in here.

## Scoped suite and clippy

`SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-frontend-as -p sigil-cli --no-fail-fast`,
target `/home/volence/sonic_hacks/.scratch/as-missing-builtins/target`, no `AEON_DIR`, each run
stamped with its tree by `scripts/run_suite.sh`:

| tip | binaries | passed | failed | ignored | unmeasured |
|---|---:|---:|---:|---:|---:|
| `9a617643` (C7) | 252 | 1720 | 0 | 1 | 133 |
| `dab84539` (C8, the code tip) | 252 | 1720 | 0 | 1 | 133 |

Both runs are stamped `tracked-changes=0` and end `SUITE_END rc=0`. The 133 are the
reference-dependent binaries the partial-run banner leaves UNMEASURED, and none of them is read
as passing. `as_int_builtins` ran in both (`test result: ok. 22 passed`), as did `as_lastbit`
(`4 passed`), and no skip line names either.

`cargo clippy --release -p sigil-frontend-as -p sigil-cli --all-targets -- -D warnings`: at C7
it FAILED, 105 errors, all in `as_int_builtins.rs`: 104 `tabs_in_doc_comments` (the verbatim
listing quotes) and one `manual_is_multiple_of`. C8 fixed both, the tabs with the file-scoped
allow `charset.rs` already uses. At `dab84539` clippy exits 0. Its log still holds 19 lines that
begin `warning:`. They are gcc `-Wmaybe-uninitialized` warnings from the vendored C++ in
`sigil-clownlzss-sys` (`vendor/compressors/enigma.h:253`), relayed from that crate's build
script; they are not clippy's, and not from this parcel.

**Re-proof at the code tip.** C8 changed the test file's listing parser, so two proofs were re-run
at `dab84539` under fresh labels. `f2d` (the plain lowest-set-bit `firstbit`) turned
`firstbit_matches_asl_over_its_table` red (`t_firstbit: ... at $4`). `f6c` (the float guess)
turned all four integer-only refusal tests red (`ref_bitcnt_float: asl refuses it, sigil built
[00, 00, 00, 02, EE]`). Both logs are in `mutations/`.

This note's own commit changes no compiled path.

## Things in the brief that turned out wrong

1. **"`abs()` with an empty argument, which asl evaluates to 0"** is true and narrower than the
   truth: asl reads an empty argument as 0 for EVERY numeric builtin, `int()` and the float family
   included. The fix is one rule, not an `abs` case.
2. **The implied unblocking of Sonic 1.** The 09-06 ledger row names `sgn(` in
   `s1disasm/MacroSetup.asm(221)` as the corpus demand. `sgn` there feeds `substr`'s LENGTH, not an
   interpolation (the interpolation is `abs`), and with `sgn` built the function still does not
   assemble, because of string `+` (above). The row's population was the wrong gap.
3. **"An integer-only builtin refusing a float rather than guessing"** as the family's house
   style: `sgn` is the exception. It accepts a float and answers an integer, so it cannot be an
   integer-only row, and the brief's float-guess mutation has no meaning for it.
4. **The family, as named.** `tolower` is in asl and was not in the 09-06 name census; it is
   `toupper`'s mirror and landed with it. And `firstbit` is not the lowest set bit, which the
   census table's phrasing and `as_lastbit.rs`'s half-fix table both assumed.
5. **The mutation "remove the builtin's `is_num_builtin` arm"** presumes one arm per builtin. The
   integer-only family is one table, read by `is_num_builtin`, on purpose (the same one-list rule as
   `FLOAT_BUILTINS`), so the arm is the table row.
