# Seven small AS features: 57 Sonic 2 rows, and the image they produce

2026-09-11, branch `parcel/s2-as-small-features`, base `f4cecd6a` (contains the string-escape
merge `571c4a76`). One commit per feature, then this note. Evidence (every probe source, every
asl listing and transcript, the scripts, the mutation texts and their red logs, the row
multisets, the byte-proof logs and the census) is in `2026-09-11-s2-as-small-features/`
beside this note.

| commit | feature |
|---|---|
| `1b8cb9ae` | 1. `()` is the value 0, and a user function's arguments are counted the way asl counts them |
| `428342b6` | 2. a struct member name may start with a digit; a plain label still may not |
| `d0cea72b` | 3. a shift or rotate with no size is a word operation, in every form |
| `89a33bba` | 4. `pushv` / `popv` |
| `b006f40c` | 5. a macro argument is text, pasted as written, and so is a parameter default |
| `97413ee9` | 6. the `lastbit` builtin |
| `4d52fb99` | 7. `shared` is accepted with a warning on every line |

## Headlines

1. **Sonic 2: 57 error rows leave, one warning row arrives, nothing else moves.** 69 rows
   before, 13 after: the 12 IX/IY half-register rows (out of scope) and the `shared` warning,
   which replaces the `shared` error by decision (below).
2. **Sonic 2 now assembles with only the Z80 half registers stubbed, and its image is asl's.**
   On a tree where every construct this parcel implements is left in its original spelling
   (stub H: only the 12 half-register lines rewritten to asl's own bytes, and the driver moved
   out of the image), sigil exits 0 and its image differs from the reference ROM in **0 bytes
   outside the census's asserted driver window**; the uncompressed driver equals asl's Z80
   record in all 4,872 bytes. The image is byte-identical (md5 `29d1b81e...`) to the one sigil
   builds from the census's fully stubbed stub B tree. Sonic 2's own closing message now reads
   `ROM size is $100000 bytes (1024 KiB). About $661 bytes are padding.`, the line asl prints
   for the reference ROM.
3. **Feature 5 was a silent defect as well as a loud one.** Wherever a macro call's arguments
   lexed, sigil pasted them back re-rendered: `$10` as `16`, `007` as `7`, `%101` as `5`, `12h`
   as `18`, `'A'` as `65`, a tab as a space, at exit 0. A census over the three corpora finds
   15,749 (Sonic 1), 19,115 (Sonic 2) and 30,023 (S3K) arguments whose text changes, counted
   over distinct call lines; all but a few hundred are number respellings with the same value.
   Sonic 2's byte proof covers its 19,115.
4. **S3K: 52 rows leave** (every unsized shift in `sonic3k.asm`), none arrive. **Sonic 1: no
   row moves.**
5. **The `lastbit` location defect is not lastbit's**, and it is BLOCKED here: every diagnostic
   raised inside a macro expansion names the body line, not the call (below).
6. **One brief assumption was wrong in each of features 1, 4, 5 and 6** (last section).

## Provenance

| instrument | identity |
|---|---|
| asl (every probe) | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**, through `asl_run` (`asl_ref.sh`), `-xx -n -q -A -L -U -i .`, one construct per file. Every run log prints the md5. Values are quoted only from `ASL_EXIT=0` runs; refused runs are read for accept-or-refuse only. |
| asl (the Sonic 2 reference ROM) | not rebuilt: the census's reference, copied from its scratch. `b3-final.bin` md5 `9feeb724052c39982d432a7851c98d3e`, p-file `ecf36eb4...`, listing `88299a61...`, Z80 record `d16f5640...` (each checked against the census and the escape note). It came from `s2disasm`'s own asl (md5 `0dee1f98...`) under the census's controls. |
| sigil | built in `/home/volence/sonic_hacks/.scratch/s2-as-small-features/target`, one binary per commit, kept under `bin/`: before `329619a8` (base `f4cecd6a`), after each feature `24c7e4db`, `962fc261`, `d52ae484`, `5d5c1993`, `2f8adac4`, `161b4075`, final `d1e28dfa` (the tree of `4d52fb99`). `logs/binary-md5s.txt`. |
| corpora | `s2disasm` `e45ebf33` plus `build.lua`'s generated inputs (the census's `corpus-gen`, copied); `s1disasm` `f6ece657`; `skdisasm` `2fcd861c`. All `cp -a` copies in the scratch directory. |

## 1. `()` is 0

**asl** (probes `e1_*`): `()` is a real integer 0 in every expression position. `dc.b
$10|()|$02` is `12`, `dc.b ()` `00`, `( )` and `(())` `00`, `-()` `00`, `~()` `FF`, `~~()` `01`,
`()=0` `01`, `5*()` `00`, `dc.w ()` `0000`, `dc.l ()` `00000000`, and `4/()` is `#1310 division
by 0`. Immediates (`#()`, `#1|()`), `equ`/`set`/`:=`, `if`, `rept` and `ds` all read 0, and so does
the typed evaluator (`int(()+1.5)` is `1`). The Sonic 2 shape itself, the real `music_metadata`
line with its helper functions, gives `90` and `78` (`e1z_music2`).

**Where it is not a plain 0.** A Z80 operand that is exactly `()` is an IMMEDIATE, not an
indirection: `ld a,()` is `3E 00` and `ld hl,()` is `21 00 00`, where `ld a,(0)` is `3A 00 00`;
`ld a,(1|())` is the memory load `3A 01 00`. 68000 `move.w (),d0` and `().w` are the absolute
`3038 0000`, and `()(a0)` is `0(a0)`, which both assemblers shorten to `3010`. An empty OPERAND
is refused: `dc.b 1,,2` and `dc.b 1,` are `#2050 empty argument`.

**User functions count argument text**, and this is what a `()`-is-0 parser gets wrong without
further work: every comma starts an argument, but the text after the last comma counts only if
it is not empty, and a blank is not empty. `f()` is 0 arguments (`#1490` on a one-parameter
function), `f( )` 1, `f(,1)` 2, `f(1,)` 1 (`#1490` on two parameters), `f(,)` 1, `f(1,,2)` 3.
Before this parcel sigil had no arity check at all: a missing argument left the parameter
unsubstituted and a surplus one was dropped. Now `check_call_args` counts by asl's rule (read
off the spans, since the lexer drops blanks) and refuses a mismatch.

**sigil now**: all of the above. `abs()` is `0` in asl and is still refused by sigil (loud).

**Newly different or refused**: nothing that assembled before assembles differently: every
expression containing `()` was a parse failure before. A user function called with the wrong
number of arguments is newly refused, which asl refuses too; none of the three corpora's row
multisets gained a row from it, and the aeon files define no `function` (the four hits are
comments).

## 2. Digit-led struct members

**asl** (`d2_*`): the member is accepted because the symbol it defines is `STRUCT.name`
(`zV.1upPlaying`, or `zV_1upPlaying` without `DOTS`), which starts with a letter. `1upPlaying`
at 1, `B` at 2, `len` 3; the all-digit `2:` and the hex-shaped `12h:` are members, as are a marker
`1up:` and `ds.w`/`ds.l` fields (`01 03 0B 0C`). A struct instance carries it (`Inst.1upPlaying`),
and the Z80 reads it as the driver does (`3A 01 00`, `DD 7E 01`). The same word is refused
wherever a SYMBOL NAME is written: `1up:` and `1up` as labels, `1up equ 5`, `1up = 5`, a bare
`1upPlaying` (`#1020 invalid symbol name` each). The name column follows a label's column rule:
an indented word without a colon is an instruction (`\t1upPlaying\tds.b 1` is `#1200`).

**sigil now**: the struct-body reader reads a digit-led name column as text and lexes only the
rest; nothing else changes, so the plain-label refusals stand. One existing unit test,
`an_unreadable_struct_member_line_is_reported_not_skipped`, used `1upPlaying:` as its unreadable
line; it keeps its property on the indented no-colon spelling asl itself refuses.

**Newly different or refused**: nothing. The line was refused before.

## 3. Shift and rotate size

**asl** (`s3_*`): all eight mnemonics default to WORD in every form, not only the word-only
memory form: `asl $1A(a0)` `E1E8 001A`, `asl (a0)` `E1D0` through `roxr (a0)` `E4D0`; `asl
#1,d0` `E340`; `asl d1,d0` `E360`; the one-operand `asl d3` `E343`; each equal to its `.w`
spelling. The memory form keeps its rules: `.b` and `.l` are `#1130` for all eight, and `asl
#2,(a0)` is `#1391`.

**sigil now**: `m68k_default_size` answers W for the eight. The encoder still refuses the 17
memory shapes asl refuses.

**Newly different or refused**: nothing; the unsized spellings were refused before. Sonic 2's
padding message moved from `$6CE` to `$6AE`: the 8 lines now emit their 32 bytes.

**Found, out of scope**: sigil refuses a shift whose operand is `$1234.w` or `$12345678.l`
("trailing tokens in operand") even with `.w` written; asl assembles both. The tests use the
addressing modes sigil encodes.

## 4. `pushv` / `popv`

**asl** (`p4_*`): `pushv [stack],sym,...` pushes each current value in list order; `popv
[stack],sym,...` pops into each symbol in list order. So one list is last-in-first-out too:
after `pushv ,A,B`, `popv ,A,B` SWAPS them (`02 01`) and `popv ,B,A` restores (`01 02`); nested
saves unwind (`06 04 02`). Stacks are named and case-sensitive; the empty name is the default
stack. Ints, floats and a label's value are saved; it works in a macro, across a second pass and
under Z80. Refused: an empty stack `#1530`, no symbol `#1110`, an undefined symbol either side
`#1010`. A stack left full is `warning #230`, one per stack, exit 0. A string symbol makes asl
abort (exit 134): no answer. asl lets `popv` OVERWRITE a constant or label with a different value
(`E equ 5` reads 7 afterwards).

**sigil now**: all of it, with two refusals asl does not make: the string case, and a `popv`
into a constant or label that would change its value (a same-value round trip is accepted). That
second one is a deliberate, loud divergence: rewriting an `equ` from a stack would leave the
exported equate, and a label its section position, disagreeing with the symbol. No corpus does
it.

## 5. Macro arguments are text

**asl** (`m5*`): the operand is cut at commas outside parentheses, brackets and literals; a `;`
outside a literal starts the comment; each argument is trimmed at both ends and keeps its
inside, tabs included (`m a\t\tb` pastes `a\t\tb`). `m ,bb,` is three arguments, `m (1,2),3` two,
`m "a,b",c` two, `m af',bb` two. `ALLARGS` is the trimmed arguments rejoined with bare commas (`m
  aa  ,  bb  ,cc` gives `aa,bb,cc`; `m aa, pb = 5` gives `aa,pb = 5`). A keyword splits at the
FIRST `=` outside parentheses and literals, both sides trimmed: `m pb==5` binds `=5`, `m pb= 5`
binds `5`, `m 2<=3` is `#1811` (a keyword named `2<`). Every digit shape pastes as written:
`2p.bin`, `1up`, `0x41`, `0FFh 1F 2G`, `$10`, `007`, `%101`, `12h`, `1.5e3`, `'A'`, and so do
`a@b` and `\x41` (which the string it lands in then unescapes to `A`). A parameter default is
pasted as written too (`pa=$10` gives `$10`).

**sigil before**: the invocation's operand was lexed; an unlexable word refused the line, and a
lexable one was re-rendered from its token (`$10` as `16` and so on, silently). `==` and `<=`
were single tokens, so `pb==5` bound `5` and `2<=3` was accepted.

**sigil now**: `exec_one` keeps the statement's text (`call_line`), a call whose operand does
not lex keeps its head through a clean-prefix lex (`macro_call_prefix`), and `call_args` cuts
the arguments from the text (`split_macro_args`, `keyword_eq_offset` in `expand.rs`). The
default text is cut from the definition line by span (`slice_source`, the helper `irp` already
used for the same defect).

**Newly different**: yes, in the direction of asl, and it is measured. The census (a scaffold
binary, never committed, `scripts/patch_census.py`, `logs/census-kinds.txt`) lists every
argument whose text as written differs from its re-rendered token, per corpus: Sonic 1 15,716
number respellings with the same value and 33 others; Sonic 2 18,756, 322 and 37 that differ
only in spacing; S3K 29,160 and 863. The "others" are expressions whose numbers are respelled
(`textLoc(7,16)` pasted as `textLoc($07,$10)`, `make_art_tile($4A4,0,0)`), with the same values.
One kind changes a TYPE: a float literal with an integral value (`1316.00`, `1.0`) used to
re-render as an integer. It sits in each sound driver's PSG frequency table and S3K's `DAC_Setup`
default; for the frequency table `PSG_Sample_Rate` is the integer `Z80_Clock/16`, and
`223721/2632` floors and rounds to 85 either way. **Sonic 2's instances are proven in bytes**
(its byte proof above builds all 19,115 with the new text and differs from asl in nothing).
**Sonic 1's and S3K's are not proven in bytes**: neither reaches an image yet. Their row
multisets and `message` output are unchanged.

**Newly refused**: a keyword argument written with `<=` or `>=` (`m 2<=3`), as asl refuses it;
no corpus row arrived from it. **Still refused**: a DEFAULT the lexer cannot read (`m macro
pa=2p.bin` is "macro needs a name"; asl accepts it). No corpus writes one.

## 6. `lastbit`

**asl** (`l6_*`): the index of the highest set bit of a 64-bit integer, -1 for none.
`lastbit(1)` 0, `(5)` 2, `($80)` 7, `($FFFEB)` 19, `($7FFFFFFF)` 30, `($80000000)` 31,
`($100000000)` 32, `(-1)` and `(-2)` 63, `(0)` -1, `2<<lastbit($FFFEB)` `$100000`. Any case,
inside `equ`, `if`, `ds`, an immediate, a Z80 `db` and `ld`, with forward references (a forward
label at `$41` gives 6), and through Sonic 2's `cnop`/`org` macros (a `$123`-byte image pads to
`$1FF`). `lastbit()` is `FF`. Refused: two arguments `#1490`, a string `#1136`, undefined `#1010`;
a float makes asl abort (`#10000`, exit 3).

**sigil now**: all of it; `lastbit` is an integer builtin beside `int` and `abs`.

**The location half: BLOCKED.** The census said the two rows named the macro body
(`s2.macrosetup.asm(20)`/`(22)`) where asl names the call. That is true, and it is not a
property of lastbit: asl reports `l6_cnop_undef.asm(16) cnop(1) org(1): error ...` (the call
line, then the chain), and sigil reports every diagnostic raised inside a macro expansion at the
body line (the 30 `music_metadata` rows all named `sounddriver.asm(3817)`, their macro's body,
not their 30 call sites). A call-site chain needs `sigil_span::Diagnostic` to carry a second
location or every in-macro message in every corpus to change text; both are real design costs,
so it is left for its own parcel, and no test here pins either answer.

## 7. `shared`, and the decision

**asl, without `-c`** (`s7*`): every `shared` line assembles, exit 0, with `warning #30: no
sharefile created, SHARED ignored`, one per line, whatever the operands (a label, two, a
variable, an expression, a forward reference, an undefined name, none). With `-c`, asl writes a
C header that Sonic 2's `build.lua` reads to patch the driver's compressed size after `p2bin`.

**The decision (flagged): accept the line, do not evaluate the operands, and warn on every
line.** sigil writes no share file, so it is asl without `-c`; the warning keeps the line from
being a silently ignored input, which this lane treats as a defect. Refusing it would refuse a
line asl assembles, over a patch that writes zero changed bytes at the census revision;
accepting it silently is the lane's defect. The post-`p2bin` patch itself is not reproduced; it
belongs to the driver-placement work, where sigil holds the symbol and needs no side file. This
is the one row that ARRIVES in Sonic 2's diff, and it arrives on purpose.

## The half-fix tests and their red-first proofs

Each mutation: `scripts/mutate.py` (refuses unless the old text occurs exactly once, then quotes
the mutated lines back from disk), the feature's test file run, the file restored with `git show
HEAD:<path>` and the tracked tree required clean (`logs/mutations/red-*.log`, each with the HEAD
it ran at). Every mutation compiled and went red; none stayed green.

| # | half-fix | mutated line, read back from disk | red |
|---|---|---|---|
| m1a | Z80 `()` read as an indirection | `operands.rs:283 return Ok(OperandAtom::Mem(Expr::Int(0)));` | `a_z80_operand_of_empty_parens_is_an_immediate_not_an_indirection`, `an_empty_group_is_zero_in_68000_operands` |
| m1b | no arity check | `eval.rs:3534 if false && passed != params.len() {` | `a_user_function_counts_its_arguments_the_way_asl_does` |
| m1c | count blind to blanks | `expand.rs:352 if last_empty && rparen > lparen {` | same |
| m1d | count = group count | `expand.rs:352 if false && last_empty && adjacent && rparen > lparen {` | same |
| m1e | an empty operand read as 0 | `expr.rs:160 if toks.is_empty() { return Some((Expr::Int(0), toks)); }` | `an_empty_operand_is_still_refused` |
| m1f | typed evaluator not taught | `eval.rs:2851 if false =>` (its `()` arm) | `an_empty_group_is_zero_in_every_data_width` |
| m1g | feature absent | `expr.rs:516 if false =>` (the `()` arm) | all 7 |
| m2a | digit-led words lexed as identifiers everywhere | `lexer.rs:304 out.push(Token { tok: Tok::Ident(run.to_string()), span: span_at(start, i) });` | `a_digit_led_plain_label_is_still_refused`, `an_indented_digit_led_member_without_a_colon_is_refused` |
| m2b | column rule dropped | `eval.rs:10973 None \| Some(b' ' \| b'\t' \| b';') => Some((name, end)),` | `an_indented_digit_led_member_without_a_colon_is_refused` |
| m2c | feature absent | `eval.rs:10962 if true \|\| !bytes[start].is_ascii_digit() {` | the 3 acceptance tests |
| m3a | `asl` only | `eval.rs:11577 Asl => Some(M68kSize::W),` | both default tests |
| m3b | default long | `eval.rs:11577 ... => Some(M68kSize::L),` | both default tests |
| m3c | feature absent | `eval.rs:11577 ... => None,` | both default tests |
| m4a | a queue, not a stack | `eval.rs:10087 ...and_then(\|v\| if v.is_empty() { None } else { Some(v.remove(0)) })...` | `nested_saves_unwind_last_in_first_out`, `one_list_is_last_in_first_out_too` |
| m4b | `popv` reads its list backwards | `eval.rs:10086 for (name, nspan) in names.into_iter().rev() {` | `one_list_is_last_in_first_out_too` |
| m4c | one stack for every name | `eval.rs:10159 [Token { tok: Tok::Ident(_), .. }] => String::new(),` | `each_stack_name_is_its_own_stack`, `a_stack_left_full_is_one_warning_per_stack` |
| m4d | feature absent (both dispatch arms deleted) | see below | 5 of 6 |
| m5a | unlexable words kept, lexable ones re-rendered | `eval.rs:4512 text: match lex_line(&operand[s..e], ...) { Ok(t) if !t.is_empty() => render_tokens(&t), _ => operand[s..e].to_string() },` | `every_argument_is_pasted_as_written`, `arguments_split_at_commas_and_keep_their_insides`, `a_keyword_argument_splits_at_its_first_equals_sign` |
| m5b | lexable ones pasted, unlexable ones still refused | `eval.rs:4135 Err(d) => match None::<Vec<Token>> {` | `every_argument_is_pasted_as_written`, `sonic_2_s_palette_call_assembles`, `arguments_split_at_commas_and_keep_their_insides` |
| m5c | `ALLARGS` not the bare-comma join | `eval.rs:10383 ...join(", ");` | the two `ALLARGS` tests |
| m5d | keyword value untrimmed | `eval.rs:10434 let (kw_name, kw_value) = (a.text[..eq].trim(), &a.text[eq + 1..]);` | `a_keyword_argument_splits_at_its_first_equals_sign` |
| m5e | defaults still re-rendered | `eval.rs:9951 )) => Some(render_tokens(text)),` | `a_parameter_default_is_pasted_as_written` |
| m6a | 32-bit | `eval.rs:18195 31 - i64::from((x as u32).leading_zeros())` | `lastbit_is_the_highest_set_bit_of_a_64_bit_integer` |
| m6b | no bit set as 0 | `eval.rs:18193 0` | same |
| m6c | lowest set bit (`firstbit`) | `eval.rs:18195 i64::from((x as u64).trailing_zeros())` | 3 of 4 |
| m6d | one case only | `eval.rs:18181 \|\| name == "lastbit"` | `lastbit_works_wherever_an_integer_is_read` |
| m6e | feature absent (the name deleted from `is_num_builtin`) | see below | 3 of 4 |
| m7a | accepted silently | `eval.rs:10274 std::mem::drop(Diagnostic {` | both |
| m7b | refused | `eval.rs:10275 level: Level::Error,` | both |
| m7c | operands evaluated | `eval.rs:6356 "shared" if !rest.is_empty() && rest.iter().all(\|t\| ...) => self.directive_shared(span),` | `the_operands_are_not_evaluated` |
| m7d | feature absent (the arm deleted) | see below | both |

**A correction to the harness's own output.** m4d, m6e and m7d replace text with NOTHING, and
for an empty replacement `mutate.py` quotes line 1 of the file rather than the mutated site (its
"quote back" looks for the new text, which is empty). The deletion itself is proven by the
script's other check, that the old text no longer occurs in the file read back, and by the
tests going red; the region was not quoted.

## Diagnostic multiset diffs, exact lines

`scripts/run_diag.sh`, stderr sorted and diffed; full multisets in `rows/`. Per-feature diffs
were taken after each commit; the whole parcel:

**Sonic 2** (`corpus-gen`, the 69-row configuration): 69 rows before, 13 after. Left: the 30
`s2.sounddriver.asm(3817):5: error: bad byte expression`; the 11 `1upPlaying` rows
(`s2.sounddriver.asm(159)` twice, and `1678`, `1712`, `1730`, `2118`, `2590`, `2623`, `2688`,
`2699`, `3153`); the 8 `instruction needs an explicit size suffix` at `s2.asm` 36098, 37592,
39179, 39180, 39181, 39200, 40447, 46924; `s2.asm(69387)` `pushv` and `s2.asm(69774)` `popv`;
the 3 `malformed number` at `s2.asm` 3935, 3936, 3937; `s2.macrosetup.asm(20)` and `(22)`; and
`s2.asm(91275)` `shared`. Arrived: `s2.asm(91275):2: warning: \`shared\` is ignored: sigil
writes no share file (asl without \`-c\` says "no sharefile created, SHARED ignored")`. By feature:
30, 11, 8, 2, 3, 2, and the `shared` swap. **What remains**: the 12 IX/IY half-register rows
(`iyl`/`iyu`/`ixl`/`ixu`, the separate Z80UNDOC parcel) and the `shared` warning.

**Sonic 1** (`sonic.asm`): 1 row before, 1 after, identical (the second-address-space row,
`sound/z80.asm(9)`). stdout identical.

**S3K** (`sonic3k.asm`): 189 before, 137 after. Left: 52 `instruction needs an explicit size
suffix` rows at `sonic3k.asm` 6033, 6960, 9217, 9232, 22241, 22242, 22243, 22274, 24099, 27460,
27461, 27462, 27496, 28935, 30577, 30578, 30579, 30609, 43756, 43757, 51902, 51903, 65019, 65020,
72154, 73755, 75075, 75076, 77694, 77992, 79422, 83235, 83256, 84613, 84614, 84650, 84656, 84725,
84726, 84767, 84869, 84885, 84940, 84942, 85006, 85013, 85910, 85911, 85921, 85922, 144342,
144343 (all feature 3). Arrived: none. `sonic3k.asm` has 52 unsized shift lines by grep; the
census's 76 S3K sites is its own tree-wide count, not re-measured here. Its one `lastbit` site
(`sonic3k.asm(203577)`, `cnop -1,2<<lastbit(*)`) gives no row before or after.

## The Sonic 2 byte proof

`scripts/run_s2_bytes.sh` with the census's committed `compare.py` unchanged (`logs/s2-bytes-f7.log`,
`logs/compare-f7-stub*.txt`):

```text
stub H  SIGIL_EXIT=0 rows=1 (the shared warning)  image 29d1b81e332245f1bd06cb3c4a0df773 size=3150600
WINDOW driver hole [0xEC0E8, 0xED04C) len 0xF64  (Snd_Driver from the p-file, guess from s2.constants.asm)
WINDOW asserted: exactly one window, extent 0xF64 bytes = 0.376% of the reference
CONTROL planted outside at 0x200,0x7FFF0,0xFFFFE and inside at 0xEC100 -> reported [('0x200', 1), ('0x7fff0', 1), ('0xffffe', 1)]
DIFF outside the window: 0 bytes in 0 runs
DRIVER at 0x300000, 4872 bytes vs asl Z80 record: 0 bytes differ
stub B  SIGIL_EXIT=0 rows=0  image 29d1b81e332245f1bd06cb3c4a0df773 (the same image)
```

Stub H (`scripts/stub_h.py`, `logs/stub-H.log`) rewrites 12 half-register lines to asl's listed
bytes and moves the driver (`!org $300000`, `phase 0`, a trailing `dephase`); the 30 omitted
FLAGS, `1upPlaying`, the unsized shifts, `pushv`/`popv`, the `2p` palettes, `lastbit` and
`shared` stay as the disassembly writes them. The zero has three controls: the planted bytes the
instrument reports, the escape parcel's baseline on which the same instrument reported 2,176, and
stub B, which uses none of this parcel's constructs and gives the same image. No byte outside
the asserted window changes.

## aeon

```text
git -C /home/volence/sonic_hacks/aeon fetch -q origin
git -C /home/volence/sonic_hacks/aeon rev-parse origin/master   -> 3492ce3a442af8aa54e031d1a0d01a1880933f9f
git -C ... ls-tree -r --name-only 3492ce3a | grep '\.asm$'       -> engine/debug/debugger.asm, games/demo/game_root.asm, games/sonic4/game_root.asm
```

Read at that revision (`git show`, saved to the scratch directory):

| construct | hits | where |
|---|---:|---|
| `()` | 2 | both inside `;` comments (`stsstr()`) |
| `function` | 4 | all comments |
| `<=` `>=` `==` | 16 | all in `if`/`while` conditions, none in a macro argument |
| `pushv` `popv` `lastbit` `shared`, unsized shifts, `struct` | 0 | |
| macro calls (feature 5) | debugger.asm only | every call of its 12 macros sits inside another macro's body; neither `game_root.asm` calls a macro, so the AS unit expands none |

No aeon byte can move from this parcel at `3492ce3a`. The controller's strict landing gate
proves the four engine ROMs.

## `eval.rs` hunks, named

- `use crate::expand::{...}`: `keyword_eq_index` out, `keyword_eq_offset` and `split_macro_args` in.
- `struct Asm`: two fields, `value_stacks` (feature 4) and `call_line` (feature 5), and their
  initialisers in `new_with_defer`.
- `one_pass_with_defer`: `asm.report_unpopped_value_stacks()` after `process` (4).
- `exec_one`: the lex-failure arm tries `macro_call_prefix`; `call_line` is set (5).
- new `macro_call_prefix`, `call_args` (5).
- `parse_num_atom`: a `()` arm (1); its builtin arm evaluates through `builtin_arg` (6).
- `eval_num`: followed by new `builtin_arg` (6).
- `expand_int_builtin`: evaluates through `builtin_arg` (6).
- `check_call_args`: the arity check (1).
- `struct_embed_name`, `parse_struct_field`: the digit-led name column; new
  `struct_field_width` split out of `parse_struct_field` (2).
- `dispatch_resolved`: `pushv`, `popv`, `shared` arms (4, 7).
- new `directive_pushv`, `directive_popv`, `restore_pushed`, `value_stack_operands`,
  `directive_shared`, `report_unpopped_value_stacks` (4, 7).
- `capture_macro`: a default's text by `slice_source` (5).
- `expand_macro_inner`: arguments from `call_args`, `ALLARGS` as the bare-comma join, keyword
  split by `keyword_eq_offset` (5).
- `struct MacroFrame`: the `all_raw` doc; new `struct CallArg` (5).
- `m68k_default_size`: the eight shifts and rotates (3).
- free functions: new `PushedValue` and its `Display`, `stack_label` (4), `digit_led_member_label`
  (2), `lastbit` (6); `is_num_builtin` and `apply_num_builtin` learn `lastbit` (6).
- unit test `an_unreadable_struct_member_line_is_reported_not_skipped`: its line (2).

Outside `eval.rs`: `expr.rs` (`parse_atom`'s `()` arm), `operands.rs` (`()` and `().w`
operands), `expand.rs` (`asl_call_arg_count`, `split_macro_args`, `keyword_eq_offset`,
`skip_literal`, `trim_blanks`; `keyword_eq_index` removed). Nothing in `sigil-cli` or
`sigil-link`.

## The probe matrix

240 probes (`scripts/gen_probes*.py`), each through the pinned asl and each sigil binary
(`logs/probe-before.log`, `probe-round2-before.log`, `probe-round3-before.log`, final
`logs/probe-final.log`). `scripts/check_listings.py` compares asl's whole listing byte stream,
continuation lines included, with sigil's on the final binary (`logs/check-final.txt`): **172
MATCH, 50 BOTH-REFUSE, 11 sigil refuses what asl accepts, 7 sigil accepts what asl refuses.** Every
one of the 18 is outside this parcel or deliberate:

| probe | what | status |
|---|---|---|
| `p4_pop_into_equ_diff`, `p4_pop_into_label_diff` | `popv` overwriting a constant | deliberate (feature 4) |
| `e1_abs_empty` | `abs()` is 0 in asl | open, loud |
| `l6_firstbit`, `l6_bitcnt` | asl builtins no corpus uses | open, loud |
| `m5d_default_2p` | an unlexable parameter default | open, loud |
| `s3_mem_nosize`, `s3_mem_nosize_rot`, `s3_mem_w` | `$1234.w` / `$12345678.l` as a shift operand | pre-existing, loud |
| `e1_paren0` | `move.w (0),d0` needs `.w`/`.l` in sigil | pre-existing, loud |
| `e1_macro_omitted` | a probe whose parameter `b` also rewrote `dc.b` to `dc.`: asl reads `dc.` as a word; sigil refuses | probe artifact, pre-existing |
| `m5_label_compose`, `_col0`, `_ctl` | a label a macro body defines, referenced outside: asl `#1010`, sigil resolves it (`_ctl` is the all-letters control, so not a digit matter) | pre-existing, silent: worth its own row |
| `m5_value_hexh` | `dc.b 12h` under `cpu 68000`: asl `#1020`, sigil reads Intel hex | pre-existing |
| `d2z_instance`, `d2z_member` | a `ds.b` struct field under `cpu z80` with no `ds` macro: asl `#1200`, sigil reads it literally | probe artifact; the corpus supplies the macro |
| `m5k_eq_in_str` | quotes inside `message` text | probe artifact, pre-existing |

## Scoped suite and clippy

`SIGIL_ALLOW_PARTIAL=1 cargo test --release -p sigil-frontend-as -p sigil-cli --no-fail-fast`,
target `/home/volence/sonic_hacks/.scratch/s2-as-small-features/target`, no `AEON_DIR`, each run
stamped with its tree (`logs/suite-*.summary.txt`):

| after | binaries | passed | failed | ignored |
|---|---:|---:|---:|---:|
| 1 | 233 | 1586 | 0 | 1 |
| 2 | 234 | 1591 | 0 | 1 |
| 3 | 235 | 1594 | 0 | 1 |
| 4 | 236 | 1600 | 0 | 1 |
| 5 | 237 | 1606 | 0 | 1 |
| 6 | 238 | 1610 | 0 | 1 |
| 7 | 239 | 1612 | 0 | 1 |

Every run: 130 reference-dependent binaries UNMEASURED (the partial-run banner). The first run of
feature 2 failed one test (`an_unreadable_struct_member_line_is_reported_not_skipped`, above)
before its line was changed. `cargo clippy --release -p sigil-frontend-as -p sigil-cli
--all-targets -- -D warnings` exited 0 after every feature.

At the final code tip `4d52fb99`, stamped `tracked-changes=0` (`logs/suite-final.summary.txt`):
239 binaries, 1612 passed, 0 failed, 1 ignored, 130 unmeasured; clippy exit 0. This note's own
commit changes no compiled path.

## Open, and why

- **The call-site chain for in-macro diagnostics** (BLOCKED, feature 6).
- **`abs()`, `firstbit`, `bitcnt`, an unlexable parameter default**: asl accepts, sigil refuses
  loudly. No corpus uses them.
- **A label defined in a macro body is visible outside the expansion in sigil, not in asl**
  (`m5_label_compose_ctl`): accept-more and silent; not a feature of this parcel.
- **The CLI's failure line counts warnings as errors**: one error and one `shared` warning print
  `assembly failed: 2 errors`; the same holds before this parcel for an author `warning`
  (`probes/cli_warn_count.asm`, baseline binary). It lives in `sigil-cli`'s `run_asm`, which
  another agent owns.
  Closed 2026-09-12: see `2026-09-12-cli-failure-counts-warnings.md`.
- **Sonic 1 and S3K argument texts** changed by feature 5 are unproven in bytes until those
  corpora reach an image.

## Things in the brief that turned out wrong

1. **"asl treats `()` as 0"** (feature 1): true in expressions, not in a Z80 operand, where `()`
   is an immediate and not the indirection `(0)`; and a `()`-is-0 parser needs asl's argument
   count for user functions, or `f()` and `f(1,)` fold where asl refuses them.
2. **"`popv` restoring the wrong value after nested pushes"** as the half-fix (feature 4): nested
   pushes are the easy case. The trap is one LIST: asl's `popv ,A,B` swaps what `pushv ,A,B`
   saved, so the natural "mirror the list" implementation is the wrong one.
3. **"a macro argument containing a digit-led word", 3 rows** (feature 5): the loud face of a
   silent defect. Any lexable argument was re-rendered, so `$10` pasted as `16` at exit 0; the
   census counts tens of thousands of re-rendered arguments across the corpora.
4. **"the diagnostic still points at the macro instead of the call"** (feature 6): that is not a
   lastbit property but how every in-macro diagnostic is located, and fixing it is a design
   change of its own.
5. **The census's S3K count of 76 unsized shifts**: 52 rows left `sonic3k.asm`, which has 52 such
   lines; the 76 was not re-measured.
