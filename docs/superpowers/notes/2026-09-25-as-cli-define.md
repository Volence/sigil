# AS-CLI-DEFINE: `-D` on the AS route, with asl's semantics

2026-09-25, branch `parcel/as-cli-define`, base `1e146771` (master at start). Evidence,
scripts, probe sources and run records are in `2026-09-25-as-cli-define/` beside this
note. Scratch: `/home/volence/sonic_hacks/.scratch/as-cli-define/`.

## Headlines

1. **S&K builds from its plain root.** `sigil sonic3k.asm -D Sonic3_Complete=0 -p=FF
   -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before`
   exits 0 with no diagnostic and writes md5 `4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32
   `0658f691`, 2,097,152 bytes: **0 differing bytes** against `buildSK.lua`'s own image,
   three runs, each compare with its planted-byte control. **Sonic 3 Complete** likewise
   with `-D Sonic3_Complete=1`: md5 `b23c88b46f5ec7377f1b6f109d97f5a5`, CRC32 `a651623a`,
   3,360,160 bytes, 0 differing, three runs. The wrapper root is no longer needed.
2. **asl's `-D` is a SET variable bound before every pass**, and that second half is new:
   a `set` may rebind it, but a second pass reads the command-line value again. Probe
   `mpass.asm` (`dc.b FOO` / `FOO set 7` / `dc.b FOO` and a forward branch, 2 passes)
   gives `05 07` under asl. A define seeded once and carried from the previous pass's end
   would give `07 07`.
3. **Probe parity: 73 of 73 rows agree with asl**, and 6 further rows are known
   divergences in which sigil refuses at the command line something asl accepts (a quoted
   value, `0b101`, `@17`, a hex literal past `i64`, a leading-dot name). Two of those are a
   design fork asl does not settle: asl's bytes for a quoted `-D` value change from run to
   run. Refused, not decided (see "Design forks").
4. **Nothing moves without `-D`.** Aeon's four shapes, S1, S2 and the S&K wrapper root are
   byte-identical between the base and tip binaries and equal to their recorded
   identities.

## Instruments

| instrument | identity |
|---|---|
| sigil tip | built from `329bc9be` (clean tree), `--version` `sigil 0.1.0 (329bc9be)`, tree clean, md5 `a73fd0f428a3fa926a3a6777a611930f`, copied to `bin-tip/sigil` so later test builds in the same target dir cannot replace it |
| sigil base | built from `1e146771` (clean tree) in its own target dir `target-base`, `--version` `sigil 0.1.0 (1e146771)`, md5 `774aada7dd20c061c789ba5c1d814d4d` |
| asl, p2bin | only via `asl_ref.sh`'s `asl_run`: md5 `61e672562465725a8c102288a7da9098`, p2bin beside it |
| skdisasm | `git archive 2fcd861c208f342b6d14df694c6422c74f20a4be`, tar md5 `0a4e468053abc841d867d130748629be`, CRC32 `d00ce908`, the value the census and the s3k-whole-rom note record |
| references | `buildSK.lua` and `buildS3Complete.lua` run unmodified, three times each, each in its own copy of that archive; their build scripts' own asl is the pinned build (s3k-whole-rom note) |
| aeon | `/home/volence/sonic_hacks/.aeon-cli-define`, provisioned by `scripts/provision-aeon-ref.sh` at `ec640bcf`, witness `repin --check` printed `pins.rs unchanged` (`logs/repin-check.log`) |
| S1, S2 trees | copies of the census's `s1disasm-gen` and `s2disasm-gen` (pristine plus `build.lua`'s pre-step outputs), compared with the census's re-derived identities, not re-derived here |

**One instrument slip, caught and redone.** The first S3K and parity runs used
`target/release/sigil` after the mutation script had rebuilt it with mutation M6 in place
and then restored the source; `--version` said `329bc9be-dirty`. Both runs were redone
with a binary rebuilt from the clean tree and set aside (`bin-tip/sigil`), and only those
runs are quoted here.

## What `-D` does now

`sigil <input.asm> ... -D NAME[=VALUE][,NAME[=VALUE]]...`, repeatable, the flag and its
list as separate arguments. The grammar lives in
`crates/sigil-frontend-as/src/cli_define.rs`; the semantics in `Options::cli_defines` and
`Asm::seed_cli_defines` (`eval.rs`), called at the start of every pass:

* each name is bound with its value through `define_sym` (so `defined()` sees it) and
  given `SymClass::Var`, so `set`/`:=`/`eval` rebind it and `=`/`equ`/a label of it is
  asl's #2035 through the existing `declare_class`;
* when a name is given twice, the first value stands;
* `Options::defines` is untouched in meaning, as ruled. Adding a field to `Options`
  forced `cli_defines: vec![]` into the struct literals that spell every field, one of
  them in `crates/sigil-harness/src/native.rs`; nothing else there changed.

**Spelling against the existing CLI.** `sigil emp` / `sigil test` take `-D NAME=INT`
(value required). The AS route follows asl instead where they differ: the value is
optional (1), a comma list is one argument, and the value is an expression. Where they
agree it matches: separate argument, repeated flags, `$hex`, `0x` hex, decimal. The
attached `-DNAME` spelling is taken by neither asl (`Invalid option: -DFOO=5`, exit 4)
nor the existing CLI, and is not taken here.

## Probe table (asl against sigil tip)

`scripts/parity.sh <sigil>` runs every row through `asl_run` + `p2bin -p=0` and through
the given sigil; a refusal matches a refusal, exit-0 bytes must be equal
(`logs/parity-tip.log`, `logs/parity-base.log`). The base binary matches 26 rows: the 24 that
asl also refuses (the base refuses every `-D`) and the 2 that pass no `-D`. That is the
table's own control: every row whose answer depends on the define goes red on the base. The first 11 rows
re-run the s3k-whole-rom `dprobe` table; the rest are this parcel's (`logs/probe*.log`
carry asl's diagnostics for each).

| row | `-D` | source | asl | sigil tip |
|---|---|---|---|---|
| value given | `FOO=5` | `dc.b FOO` | `05` | `05` |
| no value | `FOO` | same | `01` | `01` |
| empty value | `FOO=` | same | `01` | `01` |
| comma list | `FOO=1,BAR=2` | `dc.b FOO,BAR` | `0102` | `0102` |
| list, bare then valued | `FOO,BAR=2` | same | `0102` | `0102` |
| repeated flags | `-D FOO=1 -D BAR=2` | same | `0102` | `0102` |
| **repeated name** | `-D FOO=1 -D FOO=2` | `dc.b FOO` | `01` | `01` |
| repeated name in a list | `FOO=1,FOO=2` | same | `01` | `01` |
| bare then valued | `-D FOO -D FOO=2` | same | `01` | `01` |
| valued then bare | `-D FOO=5 -D FOO` | same | `05` | `05` |
| trailing comma | `FOO=5,` | same | `05` | `05` |
| values | `$10`, `$ff`, `0x10`, `10h`, `0FFh`, `%101`, `-1`, `~1`, `1+2`, `1 + 2`, `1<<4`, `(3)`, `1=1`, `' 5'`, `'5 '` | `dc.b FOO` | `10 ff 10 10 ff 05 ff fe 03 03 10 03 01 05 05` | same, each row |
| 32-bit | `$FFFFFFFF` | `dc.l FOO` | `ffffffff` | same |
| 33-bit | `$100000000` | `dc.l FOO` | refused #1320 | refused |
| 64-bit high half | `$123456789` | `dc.l FOO>>32` | `00000001` | same |
| refused at the command line | `FOO=1, BAR=2`, `FOO=1,,BAR=2`, `,FOO=5`, `FOO =5`, `''`, `FOO==5`, `FOO=$`, `FOO=5x`, `FOO=1+`, `FOO=ff`, `FOO=BAR`, `1FOO=1`, `=1`, `FO-O=1`, `FOO?=1`, `@FOO=1` | | `Invalid option: -D`, exit 4 | usage error, exit 2 |
| float | `FOO=1.5` | `dc.b FOO` | refused at the use, #1133 | refused at the command line |
| names | `FOO.BAR=1`, `_FOO=1` | used | `01` | `01` |
| case | `-D foo=5` | `dc.b FOO` | #1010 | refused |
| case, both | `-D FOO=5` | `foo equ 3` / `dc.b foo,FOO` | `0305` | `0305` |
| **`=` after** | `FOO=5` | `FOO = 7` | #2035 | `variables cannot be redefined as constants` |
| **`equ` after** | `FOO=5` | `FOO equ 7` | #2035 | same |
| **label after** | `FOO=5` | `FOO:` | #2035 | same |
| `set` / `:=` / `eval` after | `FOO=5` | `FOO set 7` etc. | `07` | `07` |
| `set` first, then use | `FOO=5` | `FOO set 3` / `dc.b FOO` | `03` | `03` |
| use, set, use | `FOO=5` | d8 | `0507` | `0507` |
| **use, set, use, 2 passes** | `FOO=5` | mpass | `050760000002` | same |
| `ifndef`-guarded set | `FOO=5` / none | `ifndef FOO` / `FOO set 3` / `endif` | `05` / `03` | `05` / `03` |
| `if` | `FOO=0` / `FOO=1` | `if FOO` | `02` / `01` | same |
| `ifdef` / `ifndef` | `FOO=0` / `FOO=5` | | `01` / `02` | same |
| **`defined()`** | `FOO=0` / none | `dc.b defined(FOO)` | `01` / `00` | same |
| builtin | `MOMCPU=5` | `dc.b MOMCPU>>16` | `06` (the builtin wins) | `06` |
| KNOWN: string | `FOO="A"` | `dc.b "A"`, `dc.b FOO` | `4125`, `4164`, `419a`, `41c1` on four runs | refused, exit 2 |
| KNOWN: character | `FOO='A'` | `dc.b FOO` | `c6`, `41`, `21`, `82`, `bb`, `4d` on six runs | refused, exit 2 |
| KNOWN: other spellings | `0b101`, `@17` | | `05`, `0f` | refused, exit 2 |
| KNOWN: past `i64` | `$FFFFFFFFFFFFFFFF` | `dc.l FOO>>32` | `ffffffff` | refused, exit 2 |
| KNOWN: leading dot | `.FOO=1` | `dc.b .FOO` / unused | #1010 / accepted | refused, exit 2 |

Two rows went DIFFER on the first tip build and were fixed, not tabled: `defined(FOO)`
read `00` (the seed went into the environment without `define_sym`, so the
defined-this-pass record never saw it), and `-D .FOO=1` with `dc.b .FOO` read `01` where
asl refuses (sigil's file-level `.FOO` resolves to the seeded key). The fix for the
second is the command-line refusal in the last row.

Not probed: `-D` of an identifier with a `'` tail, and `17o`/`101b` Intel octal and
binary (sigil's lexer refuses both; asl's command line was not asked).

## Identity table

| shape | command | tip | base | recorded |
|---|---|---|---|---|
| S&K plain root, 3 runs | `sonic3k.asm -D Sonic3_Complete=0` + buildSK p2bin | `0658f691`, 2,097,152 B, 0 bytes vs reference each run | refuses `-D` (`unexpected argument '-D'`, exit 2) | reference re-derived 3 times: `0658f691` |
| S&K, flag after the p2bin options | same, `-D` last | `0658f691`, 0 bytes | | |
| S3 Complete plain root, 3 runs | `sonic3k.asm -D Sonic3_Complete=1` + same p2bin | `a651623a`, 3,360,160 B, 0 bytes vs reference each run | | reference re-derived 3 times: `a651623a` |
| S&K plain root, no `-D` (control) | | exit 1, the 17 rows, all naming `Sonic3_Complete`, `strip_padding` or `LockonHeader` | | the s3k-whole-rom note's 17 |
| S&K wrapper root | `wrapper.asm`, no `-D` | `0658f691` | `0658f691`, tip == base | |
| S1 | census Q3 | `afe05eee`, 524,288 B | same, tip == base | `afe05eee` |
| S2 | census Q3, unstubbed gen tree | `7b905383`, 1,048,576 B | same, tip == base | `7b905383` |
| aeon `s4` | `sigil build --aeon <tree> --game sonic4` | `91c46c94`, 820,209 B | same | provisioned ROM, same |
| aeon `s4.debug` | `--debug` | `8a378de6`, 846,509 B | same | same |
| aeon `demo` | `--game demo` | `1c7a34d3`, 96,863 B | same | same |
| aeon `demo.debug` | `--game demo --debug` | `72e405a5`, 103,185 B | same | same |

Every S3K compare in `logs/s3k_define.log` carries its planted-byte control, reported
exactly. As a liveness check on the define itself, the `-D Sonic3_Complete=1` image
compared with the S&K reference differs in 1,282,377 bytes and in length.

## Red-first proofs

`scripts/mutate.sh` applies each mutation to the committed tree (`329bc9be`), prints it
on disk with `git diff`, runs both runners, and restores the file from HEAD
(`logs/mutate.log`; the last line reports 0 changed paths). Every mutation targets the
implementation, never a checker.

| mutation | red in | quoted |
|---|---|---|
| M1 a bare `-D FOO` defaults to 0 | `values_and_spellings`; unit `accepts_what_asl_accepts_with_asls_value`, `a_repeated_name_keeps_its_first_value` | `assertion left == right failed: ["-D", "FOO"] on` |
| M2 the define enters with no class, so `=`/`equ`/a label silently win | `set_rebinds_and_constants_are_refused` | `["-D", "FOO=5"] must be refused.` |
| M3 bound on the first pass only, carried after | `rebound_at_every_pass`, `defined_and_ifdef_see_it`, `set_rebinds_and_constants_are_refused` | `assertion left == right failed` at `as_cli_define.rs:135` |
| M4 a repeated name keeps its LAST value | `values_and_spellings`; unit `a_repeated_name_keeps_its_first_value` | `assertion left == right failed: ["-D", "FOO=1", "-D", "FOO=2"] on` |
| M5 bound in the environment only, not through `define_sym` | `defined_and_ifdef_see_it` | `assertion left == right failed` at `as_cli_define.rs:141` |
| M6 a leading-dot name accepted | `malformed_arguments_are_usage_errors`; unit `refuses_the_documented_gaps` | `` `-D .FOO=1`: ... unresolved symbol `FOO` `` and ``unwrap_err() on an Ok value: [(".FOO", 1)]`` |

M2 is the over-acceptance the ruling forbids: with it, `-D FOO=5` and `FOO = 7` exits 0,
which is what wiring the flag to `Options::defines` would have done.

**Runners.** `cargo test -p sigil-cli --test as_cli_define` (the CLI end to end, 6 tests)
and `cargo test -p sigil-frontend-as --lib cli_define` (the grammar, 4 tests). Both are in
the workspace suite. Expected values are asl's, from the probe table.

## Design forks (stopped on, not decided)

1. **A quoted `-D` value.** asl accepts `-D FOO="A"` and `-D FOO='A'` and then emits
   different bytes for the symbol on every run (table above), so asl settles nothing.
   sigil refuses both at the command line with a message saying why. Whether sigil should
   instead define a string variable (and pack a character constant as the source does)
   is an owner call; nothing in the corpora needs it.
2. **A float `-D` value.** asl defines a float variable (`-D FOO=1.5` then `dc.b FOO` is
   #1133 at the use). `Options::cli_defines` holds integers, so sigil refuses at the
   command line. Supporting it would mean a float-valued define, which the front end has
   (`float_env`) but no option carries. Owner call; nothing needs it.

## Booked (to the gap ledger)

* The command-line integer spellings asl takes and sigil's source lexer does not:
  `0b101`, `@17`, and a hex literal past `i64` (asl wraps `$FFFFFFFFFFFFFFFF` to -1 and
  reads `$10000000000000000` as 0). Loud today.
* `Options::defines` names are invisible to `defined()`: `run_impl` seeds them into the
  environment but not the defined-this-pass record, so `ifdef X` says yes while
  `defined(X)` says 0 for the same harness define. Measured with a throwaway test, run once
  in the worktree and deleted, kept as `scripts/zz_tmp_defines_defined.rs.txt`: `dc.b defined(X)` then an
  `ifdef X` arm emits `00 01` through `defines` and `01 01` through `cli_defines`. Aeon
  does not spell `defined(` (the `defined_arg_is_defined` doc says so). Not changed, as
  ruled.

## What this does not show

* **Whether either ROM plays.** No emulator was touched.
* **Other `sonic3k.asm` switches.** Only `Sonic3_Complete`, the one the build scripts set.
* **Sonic 3 alone** (`s3.asm`, no `-D`) is still the multi-character `'...'` class the
  s3k-whole-rom note booked; it is another agent's lexer parcel and was not run here.

## Things in the brief that turned out wrong

1. **"e.g. `-D NAME=VALUE`, `-DNAME`"** as the existing CLI's conventions: neither
   `sigil emp`/`sigil test` nor asl takes the attached `-DNAME` (asl: `Invalid option:
   -DFOO=5`, exit 4), so it is not a convention to match and is not taken.
2. **"Leave `Options.defines` and its consumers (the aeon harness) exactly as they
   are."** Their meaning is untouched, but the harness's `Options` literal in
   `sigil-harness/src/native.rs` had to gain `cli_defines: Vec::new()` to compile once
   the option existed. A struct-update spelling would have hidden the new field from
   every reader of that literal, so the explicit empty field was chosen.
3. **The ruling's "a later `set` in the source may change it"** is true and incomplete:
   the change lasts only to the end of the pass, because asl binds the `-D` value again
   at the start of each pass (headline 2). An implementation of the ruling as worded,
   seeding once, passes every one-pass row and fails `mpass`.
4. **The s3k-whole-rom sizing** ("`$hex`, and an `i64` range check") undersold the value
   grammar: asl evaluates a full expression with every integer syntax at the command
   line (`1+2`, `1<<4`, `~1`, `0FFh`, `0b101`, `@17`). sigil reuses its own front end's
   lexer and expression parser for it, which covers everything but `0b`/`@` octal.
