# F4 — a macro parameter's declared default, and a diagnostic that named the wrong line

Two separable defects, one commit each, both reached from `s1disasm`'s
`Macros.asm(11)`. Sonic 1's frontend diagnostics go **305 → 251**; Sonic 2's error
stream is **byte-for-byte unchanged**; all four aeon shapes are byte-identical.

## Provenance

| | |
|---|---|
| branch | `parcel/as-macro-default-params`, worktree `/home/volence/sonic_hacks/.sigil-f4` |
| baseline | sigil `2ed1f7cc` (master at parcel start), md5 `ded002b6b858017f0e75452bff3f9a3a` |
| after | sigil `c9a81d7e`, `sigil 0.1.0 (c9a81d7e)`, `clean-sources at capture`, md5 `5e874712a1da98425094503b8037ec25` |
| target dirs | `/home/volence/sonic_hacks/.sigil-f4-target` (after), `…/.sigil-f4-basetarget` (baseline) — both on disk, outside every shared checkout |
| corpora | `s1disasm` `f6ece657` and `s2disasm` `e45ebf33`, in this parcel's own detached worktrees; neither live checkout is written |
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, `Macro Assembler 1.42 Beta [Bld 212]`, run with Sonic 1's own flags `asl -xx -n -q -A -L -U -i .` |
| aeon reference | `/home/volence/sonic_hacks/.aeon-f4ref` @ `4f5ad5a1`, provisioned by `scripts/provision-aeon-ref.sh`; `repin --check` says **`pins.rs unchanged`** |
| wall clock | 05:29–06:20 on 2026-09-04, machine `up 9 days, 21:1x–21:5x` throughout |

The live `s1disasm` checkout carries two modified `.nem` files and is not a usable
reference. A **pristine worktree is not one either until the DAC files are
generated**: `sound/dac/*/generated/*.inc` are build products, and without them
`sonic.asm` reports four extra `cannot include` rows and four
`int(): could not evaluate float expression` rows that are artifacts of the missing
content, not of the assembler. `lua` over `common.convert_pcm_files_in_directory` /
`convert_dpcm_files_in_directory` produces them. Every count below is against a
worktree that has them.

## The baseline is 305, not the brief's 318

The grounding note measured 318 against sigil `7bef76e6`. Master has moved 19
commits since, and `b5c4f83f` (a label sharing its line with a directive now binds)
retired F6 — 13 `unresolved long expression` rows. 318 − 13 = **305**, and the
class table is otherwise identical. Nothing was reconciled to the older figure.

## Defect 1 — an empty operand group reported at line 1 of the root file

`split_commas` hands `classify` a group with no tokens whenever an operand field has
an empty slot: `move.l #1,` splits into `#1` and nothing. A tokenless group has no
span, and the span borrowed in its place was `SourceId(0)` at offset 0 — line 1 of
the ROOT source. All 36 of Sonic 1's `bad operand expression` rows printed at
`sonic.asm(1)`, a comment line, for operands written in two other files.

`parse_operands` now takes the span of the line the operands were written on. Only
the empty-group path changes; a group that has tokens still reports at its own first
token.

`asl` **accepts** every source in the test, so the expectations are not oracle rows —
it reads an empty operand as absolute address zero:

```
     5/       4 : 21FC 0000 1234      	move.l	#$1234,
              A : 0000
```

`move.l #$1234,($0000).w`. sigil refuses instead; that difference is out of this
parcel's scope and logged below. What the test pins is the LINE.

Corpus effect, exactly as pre-declared: total unchanged at 305, class decomposition
unchanged, the 36 rows move from `sonic.asm(1)` to `Macros.asm(12)` where the other
18 already were.

## Defect 2 — the declared default is substituted when the call omits the slot

`Macros.asm(11)`:

```
locVRAM:	macro loc,controlport=(vdp_control_port).l
		move.l	#($40000000+(((loc)&$3FFF)<<16)+(((loc)&$C000)>>14)),controlport
		endm
```

48 call sites omit the second argument. sigil substituted nothing, and additionally
read the parameter list by harvesting every `Ident` token in it — so this declared
**four** parameters: `loc`, `controlport`, and the `vdp_control_port` and `.l` read
out of the default expression. The first fault produced the 54 diagnostics; the
second is latent and would have substituted a body's mention of `vdp_control_port`
away to nothing.

### The rule, off the oracle

`ac macro p1,p2=DEF2,p3` emitting `dc.b ARGCOUNT` / `"<p1|p2|p3>"` / `"[ALLARGS]"`:

```
    9/  0 : 00               dc.b 0            ; ac
    9/  1 : 3C7C 4445 4632   dc.b "<|DEF2|>"
    9/  9 : 5B5D             dc.b "[]"
   10/  B : 01               dc.b 1            ; ac 11
   10/  C : 3C31 317C 4445   dc.b "<11|DEF2|>"
   10/ 16 : 5B31 315D        dc.b "[11]"
   11/ 1B : 3C31 317C 3232   dc.b "<11|22|>"    ; ac 11,22
   13/ 3F : 03               dc.b 3            ; ac 11,,33
   13/ 40 : 3C31 317C 4445   dc.b "<11|DEF2|33>"
   14/ 55 : 3C7C 4445 4632   dc.b "<|DEF2|99>"  ; ac p3=99
```

* The trigger is **no text supplied**, not "no slot written": `ac 11,,33` writes the
  middle slot empty and still takes the default.
* A keyword naming a LATER parameter (`ac p3=99`) leaves the skipped defaultless
  slot empty and the skipped defaulted one defaulted.
* `ARGCOUNT` and an unshifted `ALLARGS` are read off the CALL and see no default.
* The default is substituted as TEXT: `b1 macro q=(1,2),r=ZZ` gives `<(1,2)|ZZ>`, so
  the list splits on top-level commas only.

### Byte proof at corpus scale

Two worktrees of `s1disasm f6ece657`, both carrying the measurement parcel's
thirteen stubs, differing in **one macro only** (`diff -rq` over the trees confirms
it; the DAC `.inc` files are identical and only a hash-cache key order differs):

* `a` — the stub's hand-written `if "controlport"<>""` / `else` pair;
* `b` — the upstream `controlport=(vdp_control_port).l` declaration.

Both assemble **and link** with **zero** diagnostics to **524,288 bytes, crc32
`db808de7`** — identical. Against the corpus's own asl+p2bin ROM built in this
parcel's pristine worktree (`chkbitperfect.lua`: *ROM is bit-perfect with REV01*,
crc32 `afe05eee`), `b` matches **517,003 / 524,288 bytes = 98.61%**, reproducing the
measurement parcel's figure with the real construct in place of its stub.

### What is NOT settled

`ALLARGS` **after a `shift`** in a macro that declares a default. asl puts defaults
into the store the shifted `ALLARGS` renders from, by a rule four probe rows do not
pin — no join of the bound vector reproduces all four, although the PARAMETERS after
a shift do follow the plain slide-left rule:

```
  n1,n2=DD,n3     shift → ALLARGS "DD,"    params <DD||>
  n1,n2,n3=LL     shift → ALLARGS "LL"     params <|LL|>     shift → "LL"  <LL||>
  p1,p2,p3=CC,p4  shift → ALLARGS "CC,"    params <|CC||>    shift → "CC,"
  n1,n2=DD,n3=EE  shift → ALLARGS "DD,EE"  params <DD|EE|>   shift → "EE"
```

Nothing in `s1disasm`, `s2disasm` or aeon both declares a default and reads
`ALLARGS`/`shift`, so the corner is unreachable today. It is left as it was and
written down at the site rather than guessed at.

A second, smaller divergence: asl preserves whitespace inside a default
(`macro s = 5 , t = 6` binds `s` to `" 5"`, listing `< 5| 6>`), while sigil renders
the default from tokens and produces `5`. It is observable only where a default is
pasted into a string literal; no corpus default has interior whitespace.

## Measurement

### Sonic 1 — 305 → 251, and nothing rose

| class | 305 | 251 |
|---|---:|---:|
| bad word expression | 166 | 166 |
| `X` is not a recognized 68000 mnemonic | 39 | 39 |
| **bad operand expression** | **36** | **0** |
| unexpected character | 18 | 18 |
| **instruction needs an explicit size suffix** | **18** | **0** |
| unresolved rept count | 8 | 8 |
| case needs a string literal | 6 | 6 |
| bad immediate expression | 6 | 6 |
| trailing tokens in operand | 4 | 4 |
| switch needs a string expression | 2 | 2 |
| unknown directive or mnemonic `X` | 1 | 1 |
| org target precedes the current phase base | 1 | 1 |

No class rose and no class appeared. The `Macros.asm(12)` site is gone entirely.

### Sonic 2 — unchanged, and F4 was never there

`s2.asm`: **9432 diagnostics before and after, the two stderr streams byte-for-byte
identical** (`cmp`). The unresolved-symbol sets are 200 names on both sides, and
BOTH directions of the comparison are empty — nothing newly unresolved, nothing
newly resolved. Sonic 1 has no unresolved-symbol rows on either side.

### Does the corpus reach link

Unstubbed, no: the frontend still refuses 251 rows. **Stubbed, yes** — and with the
default declaration in place rather than stubbed out, which is the new part: exit 0,
zero diagnostics, a full 524,288-byte image.

### The four aeon shapes — no movement

`/home/volence/sonic_hacks/.aeon-f4ref` @ `4f5ad5a1`, `SIGIL_BUILD` naming each
binary explicitly, `build.sh` invoked once per shape:

| shape | size | crc32 (baseline `2ed1f7cc`) | crc32 (after `c9a81d7e`) |
|---|---:|---|---|
| `./build.sh` → `s4.bin` | 719700 | `14ee2440` | `14ee2440` |
| `DEBUG=1 ./build.sh` → `s4.debug.bin` | 737683 | `142294b3` | `142294b3` |
| `./build.sh demo` → `demo.bin` | 96474 | `0c456778` | `0c456778` |
| `DEBUG=1 ./build.sh demo` → `demo.debug.bin` | 101339 | `2e603d53` | `2e603d53` |

All eight builds exited 0, and **each of the eight artifacts was asserted to exist
with a non-zero size before its CRC was read** — an absent artifact is "did not
run", never a divergence and never a match. The three residual `.asm` files aeon
routes through `sigil-frontend-as` declare no macro default, so the result is what
the construct's absence predicts.

## Two corrections to the parcel brief

1. **F4 is Sonic-1-only.** Neither the brief's nor the grounding note's "Shared" is
   right: a sweep of every tracked file in `s2disasm e45ebf33` finds **zero** macro
   declarations with a default parameter, and the same sweep over aeon's tracked
   `.asm`/`.inc` finds zero. `s1disasm` has exactly one, `Macros.asm:11`.
2. **Sonic 2's 2,624 `bad operand expression` rows are a different construct.**
   They are AS's anonymous relative labels — `beq.s +`, `beq.s ++`, `bpl.ATTRIBUTE x`
   — spread over **2,602 distinct sites**, and they are untouched by this parcel
   (the S2 stream is byte-identical before and after). The figure is the class
   total for S2, not an F4 population.

## Logged, not fixed

**asl reads a missing operand as absolute address zero.** `move.l #$1234,` assembles
to `21FC 0000 1234 0000` and `move.l ,d0` to `2038 0000`, both silently. sigil
refuses both. That is the safer direction and no corpus line depends on it, so the
refusal stands; it is recorded because it is a silent-wrong-answer shape in the
reference assembler, and a future parcel arguing for asl fidelity here should know
what it would be buying.
