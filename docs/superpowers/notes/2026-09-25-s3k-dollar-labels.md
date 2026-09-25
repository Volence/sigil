# S3K `$$name`: AS named temporary symbols, measured and implemented

2026-09-25, parcel `S3K-FOUR-CLASSES` (project SIGIL-AS-REPLACEMENT), row `$$name`,
branch `parcel/s3k-dollar-labels`, base `120be609` (master, read from the tree at start).
Evidence, scripts, probe sources and logs are in `2026-09-25-s3k-dollar-labels/` beside
this note.

## Step 0: the 101-row S3K run, reproduced

| Instrument | Identity |
|---|---|
| sigil (base) | built in the worktree at `120be609`, md5 `80630d6e3675ef9af730890923d47353`, `--version` revision `120be60971c58e43e712b285f411db2c605b388f`, tree clean |
| skdisasm | `git archive` of `2fcd861c208f342b6d14df694c6422c74f20a4be`, tar md5 `0a4e468053abc841d867d130748629be` (the census's) |
| gen tree | the codepage parcel's `sk-gen` copied; `diff -rq` against the fresh archive shows only the 99 `Sound/DAC/generated` and 3 `Sound/PCM/generated` files and `wrapper.asm` (`Sonic3_Complete = 0` then `include "sonic3k.asm"`) |

`run_sigil.sh` (the codepage parcel's, paths moved to this scratch), the census argument
list. Exit 1, 101 rows. Multiset diff against the codepage parcel's
`sk-wrapper-after-79e30fa6.rows` (`setdiff-base-vs-codepage-after.txt`): **0 left, 0
entered, 101 common** (`$$name` 93, `abcd`/`subx` 6, `(d8,PC,Xn)` 2). Nothing had moved,
so step 1 proceeded. Committed as `baseline(...)` before any implementation.

Each of the 93 rows is one source line: `sonic3k.asm` has exactly 93 lines containing
`$$`, all in 68000 code (the Z80 driver, `Sound/Z80 Sound Driver.asm`, has none; `s3.asm`
has 83 more but is not in the wrapper root).

## Step 1: what asl does, by probing

### The oracle

Only `asl_run` from `docs/superpowers/notes/asl-reference/asl_ref.sh` (pinned build md5
`61e672562465725a8c102288a7da9098`, CRC32 `28d3da2f`), invocation `-xx -n -q -A -L -U -i .`,
then that directory's `p2bin -p=0` (md5 `4f2fff99c3347bafb93b12d5be1db754`). Bytes are
quoted only from runs that exited 0 with `ASL_DIAG=complete`; a refusal probe quotes asl's
diagnostic and nothing else. Every probe orgs at `$1200`, uses two same-named `$$` labels
in different scopes at different addresses where the rule allows it, and reads them as
multi-digit words, so "resolves", "resolves to the other one" and "undefined" are three
different answers. Runner `probe.sh` / `probeall.sh`; sources `probes/*.asm`; the whole
transcript on the final binary is `probes-final.log`.

### The rules

| rule | probes |
|---|---|
| `$$` then a letter or `_` starts a name whose tail is the identifier tail (`$$a_b`, `$$c.d`); `$$x.y` is its own name, not `$$x` | f17, f15 |
| `$$`, `$$1` are `#1020 invalid symbol name`, under both CPUs | e14, e15, e18, f06 |
| `$$.y:` is accepted as a label, and the same spelling is `#1010` as a reference | k01 |
| Names are case sensitive | e10, d07 |
| The same under `cpu z80` as under `cpu 68000`; `$` next to it is still the location counter there | f05, i10 |
| A name is keyed by the most recent NON-temporary symbol written, forward and backward | d01, d09 |
| A plain label, `equ`, `set`, `:=`, `=`, the `label` directive, an `enum`/`nextenum` member and `endstruct` end the scope; a label on the using line ends it before its operand is read | d04, d05, d06, d10, f16, g06, g10, g14, g16, i16 |
| ...but a binder reads its own right-hand side in the scope above it | e06 |
| `cpu 68000` ends it on `PADDING`, `cpu z80`/`z80undoc` on `INLWORDMODE`, `padding` on `PADDING`, `supmode` on `INSUPMODE`, `listing` on `LISTON`, `restore` on `MACEXP` (read from asl's symbol table: the `.`-local after each lists under that name) | e08, f07, f08, f09, f12, g11, g12, g13, g20, i26, h01, h02 |
| The scope is the NAME: `V set 2` after `V set 1` returns to V's `$$` names; `cpu 68000` then `padding off` share `PADDING`; two `$$x:` either side of a second `cpu 68000` are `#1000` | f01, j06, e01, i01, g02, g04 |
| `.`-locals, nameless labels, `$$` bindings, `save`, `page`, `charset`, `phase`/`dephase`, `org`, `if`, `rept`, a macro definition and a call whose body writes no label do not end it; a `$$x:` does not change the `.`-local scope | d02, d03, d08, e05, e07, e12, e13, e19, f13, f19, f20, g05, g07 |
| A PC `$$x:` in a macro body is that expansion's: two expansions each read their own, the caller cannot read it | e02, f03, f10, f14, i30 |
| ...and per iteration in `rept` | i02 |
| A body reads the caller's `$$x` when it has filed none of its own IN THAT SCOPE: ownership is by spelling AND scope. A body reading `$$q` above its own `Lb:`/`$$q:` reads the caller's (or `#1010` if there is none); a second expansion, entered with `Lb` still the scope, reads its own forward `$$q` | f02, f11, g17, i27, e11, k02, k06, k07 |
| A nested body reads the enclosing instance's `$$a` | i25 |
| A body's plain label is still the scope after the call, by its BARE name (two expansions' `Lb:` each followed by a caller `$$z:` are `#1000`) | e03, g01, i28 |
| A `$$` VALUE binding (`equ`, `set`, `label`) in a body is global, as a plain one is | k03, k04 |
| Every operand shape (`#`, `(pc)`, `.w`, `dc.l`, arithmetic) | f18, i31, j02, j03, j04, j05, x05 |
| `DEFINED($$x)` is 0 and `ifdef $$x` false whether or not it is bound | i07, j01 |
| asl's symbol table lists a `$$x` as ` x` plus 20 hex digits (e.g. ` x1ffd4ba3eb9ffadf4db3`), one suffix per scope | d01 `.lst` |
| `section` makes a `$$` defined inside it local to the section, as it does any symbol (sigil has no `section`) | e04, f04, z02 |

`section`, the `.`-local rule under `cpu` (g03) and the `A1` probes are not `$$`
behaviour; see "What this parcel does not change".

## Step 2: the design

asl has one "last global symbol" that both `.`-locals and `$$` names hang off, and
sigil already models its writers for the `.`-local scope: `define_label`'s plain branch
and `open_scope` (every value binder, the `label` directive, the last `enum` member).
`Asm::temp_scope` is written at exactly those two points, plus `open_temp_scope` for the
writers sigil's `.`-local scope does not follow (`cpu`, `padding`, `supmode`, `listing`,
`restore`, `endstruct`). It is never saved or restored around an expansion, which is what
asl does.

- Lexer: `$$` + letter/`_` is one `Tok::Ident` spelled with its `$$`, under both CPUs.
  `$$.` is refused by name. Anything else after `$$` keeps the old refusals.
- Key: `$$name@scope` (`temp_sym_base`). `@` cannot be spelled, so the key is out of the
  user namespace, and it names the scope in every diagnostic
  (``unresolved symbol `$$x@PADDING` ``). It has no leading space on purpose: that
  prefix marks an instance-private key and exempts it from the `#1000` check.
- PC labels: `define_label` files a `$$x:` in the innermost live expansion instance as
  ` exp#N.$$x@scope` and records the base in that instance's `written` set.
- References: `sym_key` sends a `$$` name to `temp_sym_key`, which looks for an instance
  that filed this base this pass (`written`) or last pass (`prev_owned`, which
  `index_instance_owned` already builds from every ` exp#N.` key), innermost first.
  The body scan is not consulted because it can only see spellings (k02). Idempotent on
  a built key, as `sym_key` must be.
- Value binders go through `binder_key` (global, like a plain binder); string and float
  reads through `value_ref_key`, so reader and writer cannot disagree.
- `DEFINED`/`ifdef` answer 0/false for a `$$` name.

No new scoping machinery: the key rides the existing expansion-instance filing and the
previous-pass ownership index, and the scope rides the existing scope writers.

## The probe table (input, asl bytes, sigil bytes)

Bytes from `$1200`. asl's bytes only from exit-0 runs with the pass loop complete; a non-zero asl exit quotes its diagnostic. Sources in `probes/`.

| probe | input | asl | sigil | verdict |
|---|---|---|---|---|
| d01 | two scopes, same names, forward and backward | exit 0: `` 12044e711204120812084e714e714e711210121412101214 `` | exit 0: `` 12044e711204120812084e714e714e711210121412101214 `` | same |
| d02 | `.loc` labels between def and use | exit 0: `` 4e714e7112024e711206 `` | exit 0: `` 4e714e7112024e711206 `` | same |
| d03 | nameless `+`/`-` between def and use | exit 0: `` 4e714e7112024e711202 `` | exit 0: `` 4e714e7112024e711202 `` | same |
| d04 | `$$x` before the first label, read after `A1:` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@A1` for fixup in section sec4608 at offset 6 `` | both-refuse |
| d05 | `A2 equ` between def and use | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@A2` for fixup in section sec4608 at offset 4 `` | both-refuse |
| d06 | `A2 set` between def and use | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@A2` for fixup in section sec4608 at offset 4 `` | both-refuse |
| d07 | `$$X` read for `$$x` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$X@A1` for fixup in section sec4608 at offset 4 `` | both-refuse |
| d08 | `$$v equ $$x+$21` in two scopes | exit 0: `` 4e714e71122324044e714e71124d `` | exit 0: `` 4e714e71122324044e714e71124d `` | same |
| d09 | `bra.s`/`beq.w` to `$$` labels | exit 0: `` 4e714e7160fc670000044e714e71 `` | exit 0: `` 4e714e7160fc670000044e714e71 `` | same |
| d10 | `B1: dc.w $$x` (label on the using line) | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@B1` for fixup in section sec4608 at offset 4 `` | both-refuse |
| e01 | `V set 1`/`$$x`/`V set 2`/`$$x` | exit 2: `` #1000: symbol double defined `` | exit 1: `` symbol double defined: `$$x@V` `` | both-refuse |
| e02 | body defines and reads `$$l`, called twice | exit 0: `` 4e714e714e7112044e714e71120a1202 `` | exit 0: `` 4e714e714e7112044e714e71120a1202 `` | same |
| e03 | body writes `Lb:`, caller reads `$$x` after | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@Lb` for fixup in section sec4608 at offset 6 `` | both-refuse |
| e04 | `section` between def and use | exit 0: `` 4e714e714e7112021202 `` | exit 1: `` `section` is not a recognized 68000 mnemonic `` | OVER-REFUSE |
| e05 | `phase` block, `$$y` inside it | exit 0: `` 4e714e714e711202340412023404 `` | exit 0: `` 4e714e714e711202340412023404 `` | same |
| e06 | `B equ $$x` (binder reads old scope) | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| e07 | `$$x:` then `.l:`, read `A1.l` | exit 0: `` 4e714e714e7112041202 `` | exit 0: `` 4e714e714e7112041202 `` | same |
| e08 | `cpu 68000` between def and use | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@PADDING` for fixup in section sec4612 at offset 0 `` | both-refuse |
| e09 | `$$v set` redefined | exit 0: `` 4e7112342345 `` | exit 0: `` 4e7112342345 `` | same |
| e10 | `$$x` and `$$X` both defined | exit 0: `` 4e714e714e7112021204 `` | exit 0: `` 4e714e714e7112021204 `` | same |
| e11 | `$$x` passed as a macro argument | exit 0: `` 4e714e714e711202 `` | exit 0: `` 4e714e714e711202 `` | same |
| e12 | macro DEFINITION between def and use | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| e13 | `rept` between def and use | exit 0: `` 4e714e714e714e711202 `` | exit 0: `` 4e714e714e714e711202 `` | same |
| e14 | `$$:` and `dc.w $$` | exit 2: `` #1020: invalid symbol name `` | exit 1: `` `$` with no hex digits `` | both-refuse |
| e15 | `$$1:` | exit 2: `` #1020: invalid symbol name `` | exit 1: `` `$` with no hex digits `` | both-refuse |
| e16 | z80 `org $1200` (probe design) | exit 2: `` #1020: invalid symbol name `` | exit 1: `` trailing tokens in expression `` | both-refuse |
| e17 | z80 `dw $$` (probe design) | exit 2: `` #1020: invalid symbol name `` | exit 1: `` trailing tokens in expression `` | both-refuse |
| e18 | 68k `dc.w $$` | exit 2: `` #1020: invalid symbol name `` | exit 1: `` `$` with no hex digits `` | both-refuse |
| e19 | `charset` between def and use | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| e20 | undefined `$$z` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$z@A1` for fixup in section sec4608 at offset 6 `` | both-refuse |
| f01 | `V set`/`W set`/`V set` returns to V's names | exit 0: `` 4e714e711200 `` | exit 0: `` 4e714e711200 `` | same |
| f02 | body reads caller's `$$x` | exit 0: `` 4e714e714e711202 `` | exit 0: `` 4e714e714e711202 `` | same |
| f03 | body owns `$$x`, caller keeps its own | exit 0: `` 4e714e714e7112041202 `` | exit 0: `` 4e714e714e7112041202 `` | same |
| f04 | `$$y` defined inside `section`, read after | exit 2: `` #1010: symbol undefined `` | exit 1: `` `section` is not a recognized 68000 mnemonic `` | both-refuse |
| f05 | z80: two scopes | exit 0: `` 000001120000000612 `` | exit 0: `` 000001120000000612 `` | same |
| f06 | z80 `dw $$` | exit 2: `` #1020: invalid symbol name `` | exit 1: `` bad word expression `` | both-refuse |
| f07 | `padding off` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@PADDING` for fixup in section sec4608 at offset 4 `` | both-refuse |
| f08 | `supmode on` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@INSUPMODE` for fixup in section sec4608 at offset 4 `` | both-refuse |
| f09 | `listing on` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@LISTON` for fixup in section sec4608 at offset 4 `` | both-refuse |
| f10 | body `$$q`, caller reads after | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$q@A1` for fixup in section sec4608 at offset 6 `` | both-refuse |
| f11 | body reads caller's LATER `$$q` | exit 0: `` 4e714e7112064e71 `` | exit 0: `` 4e714e7112064e71 `` | same |
| f12 | `save`/`restore` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@MACEXP` for fixup in section sec4608 at offset 4 `` | both-refuse |
| f13 | `if 1` block between | exit 0: `` 4e714e714e711202 `` | exit 0: `` 4e714e714e711202 `` | same |
| f14 | body `Lb:`+`$$q`, called twice | exit 0: `` 4e714e714e7112044e714e714e71120c `` | exit 0: `` 4e714e714e7112044e714e714e71120c `` | same |
| f15 | `$$x.y` is its own name | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x.y@A1` for fixup in section sec4608 at offset 6 `` | both-refuse |
| f16 | `B1 label *` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@B1` for fixup in section sec4608 at offset 4 `` | both-refuse |
| f17 | `$$a_b`, `$$c.d` | exit 0: `` 4e714e714e7112021204 `` | exit 0: `` 4e714e714e7112021204 `` | same |
| f18 | `-$$x+$$x*2` | exit 0: `` 4e714e7112024e711202 `` | exit 0: `` 4e714e7112024e711202 `` | same |
| f19 | labels in a `phase`, read after `dephase` | exit 0: `` 4e714e712402 `` | exit 0: `` 4e714e712402 `` | same |
| f20 | `org` between | exit 0: `` 4e714e710000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001202 `` | exit 0: `` 4e714e710000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001202 `` | same |
| g01 | body `Lb:` twice, caller `$$z:` after each | exit 2: `` #1000: symbol double defined `` | exit 1: `` symbol double defined: `$$z@Lb` `` | both-refuse |
| g02 | `$$x:` either side of a second `cpu 68000` | exit 2: `` #1000: symbol double defined `` | exit 1: `` symbol double defined: `$$x@PADDING` `` | both-refuse |
| g03 | `.l` either side of `cpu 68000` (no `$$`) | exit 0: `` 4e714e714e711204 `` | exit 1: `` symbol double defined: `A1.l` `` | OVER-REFUSE |
| g04 | `$$x:` after `cpu`, again after `padding off` | exit 2: `` #1000: symbol double defined `` | exit 1: `` symbol double defined: `$$x@PADDING` `` | both-refuse |
| g05 | `page 0` between | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| g06 | `enum E1,E2` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@E2` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g07 | `save` between | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| g08 | `restore` without `save` | exit 2: `` #1450: RESTORE without SAVE `` | exit 1: `` `restore` with no matching `save` `` | both-refuse |
| g09 | `pushv` (probe design) | exit 2: `` #1110: wrong number of operands `` | exit 1: `` `pushv` needs a stack name and at least one symbol: `pushv [stack],symbol[,symbol...]` `` | both-refuse |
| g10 | `V set 1` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@V` for fixup in section sec4608 at offset 6 `` | both-refuse |
| g11 | `padding on` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@PADDING` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g12 | `supmode off` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@INSUPMODE` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g13 | `listing off/on` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@LISTON` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g14 | `struct`...`endstruct` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@Str_len` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g15 | `shared` between | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| g16 | `nextenum` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@E3` for fixup in section sec4608 at offset 4 `` | both-refuse |
| g17 | body reads caller `$$x` defined between two calls | exit 0: `` 4e714e7112064e714e711206 `` | exit 0: `` 4e714e7112064e714e711206 `` | same |
| g18 | `A1.$$x` | exit 2: `` #1020: invalid symbol name `` | exit 1: `` bad word expression `` | both-refuse |
| g19 | `.pre:` then `$$x` | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| g20 | `cpu z80`/`cpu 68000` between | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@PADDING` for fixup in section sec4612 at offset 0 `` | both-refuse |
| i01 | `cpu 68000` then `padding off` share `PADDING` | exit 0: `` 4e714e711202 `` | exit 0: `` 4e714e711202 `` | same |
| i02 | `rept 2` body owns `$$r` | exit 0: `` 4e714e714e7112044e714e71120a `` | exit 0: `` 4e714e714e7112044e714e71120a `` | same |
| i05 | `$$x label *` | exit 0: `` 4e714e711204 `` | exit 0: `` 4e714e711204 `` | same |
| i07 | `defined()`/`ifdef`/`ifndef` of `$$` | exit 0: `` 4e714e71000000002222 `` | exit 0: `` 4e714e71000000002222 `` | same |
| i09 | `#$$x-A1` (`A1` is a register to asl) | exit 3: `` #10000: internal error `` | exit 0: `` 4e714e71303c1202323c001241fafff43438120200011202 `` | OVER-ACCEPT |
| i10 | z80 `jr`/`ld`/`dw` with `$$x` and `$` | exit 0: `` 000018fd21011218fe0b1201123a0112 `` | exit 0: `` 000018fd21011218fe0b1201123a0112 `` | same |
| i16 | forward read across `B1:` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@A1` for fixup in section sec4608 at offset 2 `` | both-refuse |
| i25 | nested body reads the enclosing instance | exit 0: `` 4e714e714e714e714e7112064e714e714e71120e1202 `` | exit 0: `` 4e714e714e714e714e7112064e714e714e71120e1202 `` | same |
| i26 | `$$x:` after `cpu z80`, read after `cpu 68000` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$x@PADDING` for fixup in section sec4613 at offset 1 `` | both-refuse |
| i27 | body reads caller `$$q` in two scopes | exit 0: `` 4e714e714e7112024e714e714e714e71120c `` | exit 0: `` 4e714e714e7112024e714e714e714e71120c `` | same |
| i28 | body `Lb:` then caller `A1b set` | exit 0: `` 4e714e714e714e711206 `` | exit 0: `` 4e714e714e714e711206 `` | same |
| i29 | `$$y` read after `W set` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$y@W` for fixup in section sec4608 at offset 10 `` | both-refuse |
| i30 | three expansions each own `$$x` | exit 0: `` 4e714e7112024e7112064e71120a `` | exit 0: `` 4e714e7112024e7112064e71120a `` | same |
| i31 | `cmpi.w #$$x` | exit 0: `` 4e714e714e710c40120412041234 `` | exit 0: `` 4e714e714e710c40120412041234 `` | same |
| j01 | `defined()` before/after, `if defined($$v)` | exit 0: `` 4e7100004e710000000000012222 `` | exit 0: `` 4e7100004e710000000000012222 `` | same |
| j02 | `#`, `(pc)`, `.w`, `dc.l` operands | exit 0: `` 4e714e71303c120241fafff83438120200011202 `` | exit 0: `` 4e714e71303c120241fafff83438120200011202 `` | same |
| j03 | `$$x-Lab1+$10` | exit 0: `` 4e714e710012 `` | exit 0: `` 4e714e710012 `` | same |
| j04 | `#$$x-Lab1`, `#($$x-Lab1)*3` | exit 0: `` 4e714e71323c0002343c0006 `` | exit 0: `` 4e714e71323c0002343c0006 `` | same |
| j05 | `#$$x+$10` | exit 0: `` 4e714e71323c1212 `` | exit 0: `` 4e714e71323c1212 `` | same |
| j06 | `set`-scoped names in V and W | exit 0: `` 4e714e7112004e714e714e711208 `` | exit 0: `` 4e714e7112004e714e714e711208 `` | same |
| j07 | `#A1+$$x` (`A1` register) | exit 2: `` #1145: expected integer, floating point number or string but `` | exit 0: `` 4e714e71323c2402 `` | OVER-ACCEPT |
| j08 | `#($$x-A1)` (`A1` register) | exit 3: `` #10000: internal error `` | exit 0: `` 4e714e71323c0002 `` | OVER-ACCEPT |
| j03 | `$$x-Lab1+$10` | exit 0: `` 4e714e710012 `` | exit 0: `` 4e714e710012 `` | same |
| j04 | `#$$x-Lab1`, `#($$x-Lab1)*3` | exit 0: `` 4e714e71323c0002343c0006 `` | exit 0: `` 4e714e71323c0002343c0006 `` | same |
| k01 | `$$_x`, `$$.y` | exit 2: `` #1010: symbol undefined `` | exit 1: `` `$$.` does not start a temporary symbol name: follow `$$` with a letter or `_` `` | both-refuse |
| k02 | body reads `$$q` before its own `Lb:`/`$$q:` | exit 0: `` 4e714e7112024e714e71 `` | exit 0: `` 4e714e7112024e714e71 `` | same |
| k03 | body `$$v equ`, `$$w set` | exit 0: `` 4e714e7112241234 `` | exit 0: `` 4e714e7112241234 `` | same |
| k04 | body `$$x label *` | exit 0: `` 4e714e711204 `` | exit 0: `` 4e714e711204 `` | same |
| k05 | `pushv` (probe design) | exit 2: `` #1110: wrong number of operands `` | exit 1: `` `pushv` needs a stack name and at least one symbol: `pushv [stack],symbol[,symbol...]` `` | both-refuse |
| k06 | k02 with no caller `$$q` | exit 2: `` #1010: symbol undefined `` | exit 1: `` unresolved symbol `$$q@Lab1` for fixup in section sec4608 at offset 2 `` | both-refuse |
| k07 | second expansion reads its own forward `$$q` | exit 0: `` 4e714e714e7112044e714e714e71120c12164e714e714e711216 `` | exit 0: `` 4e714e714e7112044e714e714e71120c12164e714e714e711216 `` | same |
| h02 | scope after `save`/`cpu z80undoc`/`restore` (listing only) | exit 0: `` 4e7100004e7100004e714e714e71 `` | exit 0: `` 4e7100004e7100004e714e714e71 `` | same |
| x01 | `$$v set` read by `dc.w` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| x02 | `$$v equ` read by `dc.w` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| x03 | control: plain `set` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| x04 | control: `dc.l Lab1+$10000` | exit 0: `` 4e7100011200 `` | exit 0: `` 4e7100011200 `` | same |
| x05 | `dc.l $$x` | exit 0: `` 4e714e7100001202 `` | exit 0: `` 4e714e7100001202 `` | same |
| x06 | control: `#A1+Lab2`, `A1` not a label | exit 2: `` #1145: expected integer, floating point number or string but `` | exit 1: `` `A1` is a register, not a value: expected an integer, floating point number or string `` | both-refuse |
| y01 | `$$v set` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| y02 | `$$v set` in `#` | exit 0: `` 4e71303c1234 `` | exit 0: `` 4e71303c1234 `` | same |
| y03 | `$$v :=` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| y04 | `$$v =` | exit 0: `` 4e711234 `` | exit 0: `` 4e711234 `` | same |
| z01 | control: `#A1+Lab2` with `A1:` a label (no `$$`) | exit 2: `` #1145: expected integer, floating point number or string but `` | exit 0: `` 4e714e71323c2402 `` | OVER-ACCEPT |
| z02 | control: `section` (no `$$`) | exit 0: `` 4e714e711200 `` | exit 1: `` `section` is not a recognized 68000 mnemonic `` | OVER-REFUSE |

## Acceptance (a): the S3K run

Same tree, wrapper and arguments as step 0, sigil `9097415d` (md5
`32288022c5919cc0cb604840c4f9a5d7`). `setdiff-after-vs-base.txt`:

| class | base | after | left | entered |
|---|---:|---:|---:|---:|
| `$$name` | 93 | 0 | 93 | 0 |
| `abcd`/`subx` | 6 | 6 | 0 | 0 |
| `(d8,PC,Xn)` | 2 | 2 | 0 | 0 |
| **total** | **101** | **8** | **93** | **0** |

All 93 left, nothing entered, no class rose, no new class. No new rows appear because the
front end already read the whole file before (a lex error does not stop the pass); the
run still stops at the front end on the other two classes, so the whole-ROM bytes are
not measurable yet. `sk-wrapper-after-9097415d.rows` is the 8-row file.

## Acceptance (b): bytes against asl

**Probes.** 114 probes (`final.txt`, split into `probes/`), all run on the final
binary (`probes-final.log`):

| verdict | count |
|---|---:|
| same (both exit 0, bytes equal) | 63 |
| both refuse | 44 |
| sigil refuses, asl accepts | 3 |
| sigil accepts, asl refuses | 4 |
| bytes differ | 0 |

Every one of the 7 disagreements is pre-existing and not `$$`, each with a `$$`-free
control that shows the same result on the base binary: `section` is not a sigil
directive (e04, control z02); the `.`-local scope under `cpu` (g03, which has no `$$`,
same refusal on base); and a label named `A1` read as a value where asl reads the
register (i09, j07, j08, control z01, `323C 2402` on base). All three are in the gap
ledger. For every both-refuse row the sigil diagnostic was checked to come from this
mechanism (it names the built key, e.g. ``unresolved symbol `$$x@PADDING` ``, or is the
lexer's own refusal for `$$`, `$$1`, `$$.`), apart from the four probe-design rows (e16,
e17, g09, k05) and g08 (`restore` without `save`), which both refuse for their own reasons.

**The S3K extract.** `mk_s3kprobe.sh` builds `probes/s3kdollar.asm` (1,010 lines, md5
`18c3c5ba539a9a8fae18ff4e6be8e5f6`) from VERBATIM skdisasm `2fcd861c` text: all 93 `$$`
lines, in ten `sonic3k.asm` ranges (297-351, 486-507, 1010-1222, 1313-1345, 1747-1781,
2003-2116, 2269-2297, 2298-2563, 2667-2790), with `sonic3k.macros.asm` 1-66 and 93-103
(`stopZ80`, whose body writes a `.loop:` between a `$$` definition and its use). Not
verbatim: `cpu 68000`/`supmode on`/`org $2A6` at the top, `s3k-standins.asm` (ports at
their real addresses, ROM names at distinct even addresses, RAM through S3K's own
`ramaddr`), and one inserted `BlueSpheresStartup:` label, reached by a `bra.s`. asl: exit
0, pass loop complete, 0 warnings. `compare.py`: both 2,466 bytes, CRC32 `a3146e4a`, 0
differing bytes; **control**: three planted bytes reported exactly as `[0, 1233, 2465]`.
The base binary refuses the same file with 93 errors, so the match is not vacuous. It is
the test `the_sonic_3_temporary_labels_match_asl`.

## Acceptance (c): aeon, Sonic 1, Sonic 2

**Reachability first.** `grepdollar.sh` counts `$$` in every `.asm`/`.inc`/`.s`/`.emp`/
`.z80` file, comment text included (which can only over-count): aeon (`.aeon-s3k-codepage`,
`ec640bcf`, 205 files) **0**, Sonic 1 gen tree (459 files) **0**, Sonic 2 gen tree (371
files) **0**; the positive control, skdisasm's gen tree, reports `sonic3k.asm:93` and
`s3.asm:83`. So none of the three can reach the new lexer arm or a `$$` key. What they do
reach is `temp_scope` being written (every label, binder, `cpu`, `padding`...), which is
read only for a `$$` name; the identities below attest that writing it moved nothing.

**Aeon.** `AEON_DIR=/home/volence/sonic_hacks/.aeon-s3k-codepage`, re-verified first:
`repin --check` exit 0, last line `pins.rs unchanged` (`logs/repin-check.log`).
`four_shapes.sh` built all four shapes with each binary pair:

| shape | before (`120be609` archive, sigil md5 `54ae38bc...`) | after (`9097415d`, md5 `32288022...`) |
|---|---|---|
| `sonic4` | `91c46c94` / 820,209 | `91c46c94` / 820,209 |
| `sonic4` DEBUG | `8a378de6` / 846,509 | `8a378de6` / 846,509 |
| `demo` | `1c7a34d3` / 96,863 | `1c7a34d3` / 96,863 |
| `demo` DEBUG | `72e405a5` / 103,185 | `72e405a5` / 103,185 |

Each pair compared whole by `compare.py`: 0 differing bytes, each with its planted
three-byte control reported exactly (`logs/identity.log`). The before values equal the
codepage parcel's, and the golden ROMs were copied back into AEON_DIR afterwards.

**Sonic 1 and Sonic 2** (census Q3 roots and arguments, `s12.sh`, gen trees copied from
the census scratch and checked against fresh `git archive`s of `f6ece657` / `e45ebf33`:
0 content differences, only the 10 / 77 pre-step outputs added):

| corpus | before | after |
|---|---|---|
| S1 `sonic.asm -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after` | md5 `09dadb5071eb35050067a32462e39c5f`, CRC32 `afe05eee`, 524,288 | identical |
| S2 `s2.asm -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after` | md5 `9feeb724052c39982d432a7851c98d3e`, CRC32 `7b905383`, 1,048,576 | identical |

Both equal the census references; each after/before pair compared whole with its planted
control; exit 0 both times (S2's one stderr line is the known `shared` warning).

## Tests, and the proof they can fail

`crates/sigil-frontend-as/tests/as_dollar_labels.rs`, 22 integration tests run by
`cargo test --workspace`, over 51 accepted probes (expected bytes: asl's image hex from
`$1200`, emitted from asl's own `.bin` by `gen_tests.py`, which also asserts asl's image
is zero below `$1200` and that asl exited 0 with the pass loop complete) and 34 refused
probes (needles name the built key). Plus `the_sonic_3_temporary_labels_match_asl` over
the extract and asl's image (`tests/vectors/s3k_dollar_labels/`), and the unit test
`lexer::tests::a_temporary_symbol_is_one_identifier_under_both_cpus`. The harness links
the module, so an undefined name is a refusal as it is in the CLI.

**Red before the implementation** (`redfirst_base.sh`, `logs/redfirst-base.log`): the
tests copied into the `120be609` archive tree with its implementation untouched (the log
checks `eval.rs` against the archive): 22 of 22 and 1 of 1 fail, e.g. ``expected asl's
bytes 12044e71..., got diagnostics: ["`$` with no hex digits", ...]``. That red is only
the lexer, so the scoping is proved by mutations of the SUBJECT, applied by `mutate.py`,
shown on disk by `git diff --stat`, run, then restored by `git show HEAD:<file> > <file>`
with an empty diff stat after every one (`mutrun.sh`, `logs/mutrun.log`):

| id | mutation (on disk) | red | quoted |
|---|---|---|---|
| M1 | scope dropped from the key: `format!("{name}@{}", "")` | 14 of 22, incl. S3K | ``expected a refusal naming "unresolved symbol `$$x@A2`", got 4614 bytes`` |
| M2 | `open_temp_scope` a no-op | 3 | ``expected a refusal naming "unresolved symbol `$$x@Str_len`"`` |
| M3 | a plain label does not move the scope | 10, incl. S3K | ``got diagnostics: ["symbol double defined: `$$q@PADDING`"]`` |
| M4 | a body's `$$x:` filed globally | 4 | ``symbol `$$l@A1` redefined by section `sec4608` `` |
| M5 | ownership ignores the previous pass | 1 (`body_ownership_is_by_scope_not_by_spelling`) | ``unresolved symbol `$$q@Lb` for fixup ... offset 16`` (k07) |
| M6 | `ifdef` answers for `$$` like any name | 1 (`a_temporary_symbol_is_never_defined`) | ``sigil's image from $1200 differs from asl's`` |
| M7 | a `$$` binder moves the scope | 2 | ``unresolved symbol `$$v@$$v` `` |
| M8 | `temp_sym_key` not idempotent | 4, incl. S3K | ``unresolved symbol `$$v@Lab1` for fixup ...`` |
| M9 | a value binder does not move the `$$` scope | 4 | ``expected a refusal naming "unresolved symbol `$$x@B1`"`` |
| M10 | lexer arm 68000-only | 2 (Z80 tests) | ``expected mnemonic, directive, or label`` |

The S3K test goes red only under M1, M3 and M8: S3K's `$$` use is plain labels at file
level, so the macro, binder and directive rules are pinned by the synthetic probes, not by
the corpus.

## Full suite

`cargo test --release --workspace --no-fail-fast` at `9097415d` with this parcel's
`CARGO_TARGET_DIR` and `AEON_DIR` (`logs/suite.log`, stamped with pwd, HEAD, branch):
496 test-result lines, **5,657 passed, 1 failed, 2 ignored**. `as_dollar_labels` ran there
(22 passed) and so did the lexer unit test. The one failure is `sigil-harness --test
m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`, stopping on `NO REFERENCE TREE
IS NAMED` (no `ORACLE_DIR`; it declines to derive `oracle-old`), the environmental failure
the brief names and the codepage note measured at its own base. `cargo clippy --release
--workspace --all-targets -- -D warnings`: `CLIPPY_RC=0` (`logs/clippy.log`).

## What this parcel does not change

- `.`-local scoping: asl moves it at `cpu`/`padding`/`supmode`/`listing`/`restore`/
  `endstruct` too (g03, h01); sigil's does not. Ledgered.
- A register spelling defined as a label is accepted in a value position (z01).
  Pre-existing silent over-acceptance. Ledgered.
- `section` (e04, f04, z02): unsupported, loud.
- Unmeasured `$$` corners (enum members in a body, `pushv`/`popv`), `$$.name`, and asl's
  hashed symbol-table spelling. Ledgered.

## Instruments

| Instrument | Identity |
|---|---|
| sigil (after) | `9097415de7602996b2dd2c13103dab2e91a11662`, md5 `32288022c5919cc0cb604840c4f9a5d7`, CRC32 `9e42ffec`; emit_sound_blob md5 `fc77af0d999333326e64a1c4a611479c` |
| sigil (before, aeon/S1/S2/red-first) | `git archive 120be609` (tar md5 `f5c439df7434477e444ce155da0e6607`), md5 `54ae38bc5ef73e8c0fa9eb993a4c8254`, CRC32 `fc9d1471`; emit_sound_blob md5 `ad6e2b8fbaec9a0a63aebc21c160588f` |
| asl, p2bin | `s1disasm/build_tools/Linux-x86_64/`, asl md5 `61e672562465725a8c102288a7da9098` CRC32 `28d3da2f`, p2bin md5 `4f2fff99c3347bafb93b12d5be1db754` |
| aeon | `ec640bcf70e263167223a33d987d4661ff221b7e`, 0 tracked changes |
| corpora | skdisasm `2fcd861c`, s1disasm `f6ece657`, s2disasm `e45ebf33` |

One trap met on the way, for whoever builds a base binary the same way: a `git archive`
extract keeps commit-time mtimes, so building it into a target dir that already holds a
newer build reuses the newer artifacts silently (the first "base" binary accepted `$$`).
Touching the extract's files forced the rebuild; the base binary used above refuses `$$`,
which is checked, and `cargo clean -p sigil-cli` was needed afterwards to restore the
worktree binary's `--version` provenance.
