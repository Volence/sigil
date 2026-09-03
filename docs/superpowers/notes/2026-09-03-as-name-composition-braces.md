# A `{expr}` group composes the symbol name it sits in

2026-09-03 · branch `parcel/as-macro-variadic-shift` · sigil master base `de527346`

Fourth in the AS-frontend arc for the public Sonic 2 disassembly
(`/home/volence/sonic_hacks/s2disasm`, git `e45ebf3`). The parcel was dispatched
to implement AS's `shift` directive. `shift` is **not** what this landed, and the
determination that redirected it is the substance of the note.

## The brief's hypothesis, and why it was wrong

Two diagnostics dominate `s2.macros.asm` and fire the SAME number of times —
32,351 each:

```
s2.macros.asm(197): error: `shift` is not a recognized 68000 mnemonic
s2.macros.asm(191): error: org needs a constant expression
```

Both sit in one macro body, `zoneTableEntry`, which walks a variable-length
argument list by `shift`-ing and re-invoking itself. The natural reading — and
the dispatching brief's — is one root cause with two faces: `shift` is
unimplemented, so the walk cannot advance, and the `!org` beside it fails as
collateral.

**Two probes separate them.** Each is the corpus shape with one construct
removed:

*Probe A — recursion + `shift`, no brace group in the `org` expression:*

```
zt macro value
	if "value"<>""
	    org base+zone_id_0*2
	    dc.w value
	    shift
	    zt ALLARGS
	endif
    endm
	zt 1,2,3
```
→ 64 × `` `shift` is not a recognized 68000 mnemonic `` + 1 × `expansion too
deep`. **Zero `org` errors.**

*Probe B — one brace group, no recursion, no `shift`:*

```
zt macro value
	org base+zone_id_{cur_str}*2
	dc.w value
    endm
	zt 1
```
→ exactly one `org needs a constant expression`.

So they are **two independent unimplemented constructs that co-occur in one
macro body**. `shift` supplies the MULTIPLIER — the self-call never consumes an
argument, so the expansion recurses to `EXPAND_CAP` (64) and every iteration
re-emits both lines. The brace group supplies the ERROR — `zone_id_{…}` reaches
the evaluator as the truncated name `zone_id_`, so the `org` expression is not
constant. Identical counts are co-location times a runaway, not causation.

The third thread, `macro zoneTableEntry expansion too deep` (532), IS `shift`:
it is the runaway itself, and it is the only one of the three that is.

## What AS actually does, measured

Ground truth throughout is `asl -L` (AS V1.42 Beta Bld 212,
`s2disasm/build_tools/Linux-x86_64/asl`), not inference. Every expected value in
the new tests is a row quoted from a listing.

### Name composition

A `{expr}` group written outside a string literal and outside a comment
evaluates and pastes its value into the surrounding identifier:

```
   7/     100 : 0055                	dc.w zone_id_{cur_str}      ; cur_str := "3", zone_id_3 = $55
   8/     100 : 0022                	dc.w {"n"}{cur}             ; leading group, two groups
   9/     102 : 0055                	dc.w zone_id_{cur}_x        ; interior group
  10/     104 : 0066                	dc.w xx{cur+0}              ; a full expression
   9/     104 : =$77                 zone_id_{cur_str}b = $77     ; the DEFINING side too
  11/     106 : 6272 6163 6520 7B63 7572 7D20 …   dc.b "brace {cur} in string"
```

A string expression contributes its characters, an integer its decimal digits.
Inside a `"…"` literal the braces are literal text (`7B 63 75 72 7D` is `{cur}`),
and a `;` ends the scan — `s2.asm:90240`'s `; struct blockMapElement {` must stay
inert. The closing `}` is found across a literal that itself holds one, which is
what lets `s2.macros.asm:246`'s `zoneanimcount_{"\{zoneanimcur}"}` compose.

### `\{expr}` folds at BINDING time

```
   4/     100 : ="3"                 s := "\{n}"      ; n := 3
   5/     100 : 0001                	dc.w strlen(s)  ; 1, the rendered digit
   6/     102 : 33                  	dc.b s
   7/     103 : =$2A                 n := 42
   8/     103 : 33                  	dc.b s          ; still 3
```

sigil stored the source spelling and folded at each read. It now folds at the
assignment.

## The defect that was invisible in the diagnostic count

Before this parcel, every `zoneID` invocation in `s2.constants.asm:375` —

```
zone_id_{cur_zone_str} = zoneID
```

— defined the same collapsed name `zone_id_`, last write winning, seventeen
times. And `s2.asm:88545` consumed it:

```
	!org LevelArtPointers+zone_id_{cur_zone_str}*12
```

That line produced **no diagnostic**, before or after. It was not failing; it was
silently folding a constant `org` against one zone's id for every zone. A
truncated name is only loud when nothing owns the truncation — here something
did. This is why an unresolvable group is now a hard error rather than a pasted
prefix: the failure mode is a name that exists and is wrong.

## What is BLOCKED, and why

### `shift` — the variadic argument walk (35,167 diagnostics)

asl-verified semantics, `zt 1,2,3,4` with params `a,b,c`:

| after | a | b | c | ALLARGS |
|---|---|---|---|---|
| entry | 1 | 2 | 3 | `1,2,3,4` |
| shift | 2 | 3 | *(empty)* | `2,3,4` |
| shift | 3 | *(empty)* | *(empty)* | `3,4` |
| shift | *(empty)* | *(empty)* | *(empty)* | `4` |

Params are bound ONCE to the first N arguments and that vector shifts left with
empty fill — argument 4 never reaches `c`. `ALLARGS` is the raw argument text
with one leading group dropped per shift. A shift past exhaustion is a no-op.

**Why it is not a bolt-on.** `expand_macro_inner` substitutes the ENTIRE body
text up front and then `exec`s it. Both corpus uses (`zoneTableEntry`,
`creditsPtrs`) read `ALLARGS` on a line AFTER the `shift`, so the substitution
must be lazy — performed as each line is reached. That means every site that
consumes a body line's text: `exec`'s keyword dispatch, `exec_if` (arm scan and
condition), `exec_switch`, `exec_rept`, `exec_while`, `find_block_end`,
`capture_macro`, `capture_struct`, `def_function`, `exec_one`. Roughly ten sites
in the evaluator that ships aeon's bytes. `capture_macro` is the semantically
sharp one: today a nested macro definition captures text with the outer
expansion's parameters already baked in, and lazy substitution would capture raw
text unless the capture is made to substitute explicitly.

Sizing: one parcel of its own, with the full four-shape aeon re-verification, not
an addendum to this one.

### `.`-local `set` variables are scoped per expansion, and AS scopes them to the caller

**This is the reason this parcel moves no corpus number.** Every unresolved brace
group left in the corpus — all 32,385 of them — is the same one:

```
s2.macros.asm(191): error: `{.cur_zone_str}` in a symbol name did not resolve
s2.macros.asm(208): error: `{.cur_zone_str}` in a symbol name did not resolve
```

`.cur_zone_str` is assigned in `zoneOrderedTable`'s expansion and read in
`zoneTableEntry`'s. sigil gives each expansion a private scope for its `.`-names,
so the read misses.

asl distinguishes two cases that sigil currently treats as one:

```
setit macro v
.shared := v
    endm
Outer:
	setit $11
	setit $22
	dc.w .shared        →  0022    (symbol table: OUTER.SHARED = 22)
Second:
	dc.w .shared        →  0033    (SECOND.SHARED — a different symbol)
```

A `.`-name bound by `set` inside a macro qualifies against the CALLER's enclosing
global-label scope and is shared across expansions. A `.`-LABEL does not:

```
lbl macro
.here:
	nop
	bra.s	.here
    endm
Outer:
	lbl
	lbl
	dc.w .here-Outer    →  error: symbol undefined
```

Each expansion owns its own `.here`, and it is not visible to the caller — which
is exactly sigil's present model, and correct for labels. So sigil is right for
one half of the rule and wrong for the other.

Fixing it needs a second namespace (an expansion scope for labels, the enclosing
label scope for `set`/`equ`) and a reference-resolution rule that picks between
them. The obvious shortcut — try the expansion scope, fall back to the caller's —
has a silent-wrong failure mode: a macro body's forward branch to a `.`-label not
yet defined would fall through to a same-named `.`-label in the caller's scope.
That needs designing, not patching. **BLOCKED. One parcel, aeon-byte-risky.**

### The rest of the corpus, sized

| Construct | Diagnostics | Note |
|---|---|---|
| `label` + `{INTLABEL}`/`__LABEL__` | 5,353 | Inseparable — every `label` use in the corpus is `__LABEL__ label *`. `{INTLABEL}` is a macro-param attribute the lexer discards, so `capture_macro` cannot currently see it. One parcel. |
| `eval` | 3,417 | `sound/_smps2asm_inc.asm`. Unexamined. |
| Z80 `$` as the program counter | 752 (+749 cascade) | `s2.macrosetup.asm:59`'s `if ($)&1`; the following `endif` errors are its cascade. |
| `strlen()` on a `function` parameter | 996 | `s2.macrosetup.asm:104`'s `chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)`. |
| `irpc` | 370 | Character-wise repeat. |
| `charset` | 82 | |

## Corpus movement

Same command both times, `sigil s2.asm` from `s2disasm`:

| | before | after |
|---|---|---|
| diagnostics | 89,120 | 121,505 |
| distinct unresolved symbols in `s2.asm` | 207 | 207 |
| distinct unresolved symbols, all files | 298 | 298 |

The rise is 32,385 new `` `{…}` in a symbol name did not resolve `` — one per
expansion of the runaway, naming the real defect where `org needs a constant
expression` named a symptom. No previously-unresolved symbol resolved and none
regressed; the composition sites that DO now work (`s2.constants.asm:375`,
`s2.asm:88545`, `s2.macros.asm:246`) are definitions and a silent fold, none of
which was ever counted.

**Read the count rise as diagnostic precision, not regression, and read the flat
symbol count as the honest measure of what a name-composition fix can reach while
the `.`-local scope rule is unfixed.**

## Verification

- aeon four shapes rebuilt from `/home/volence/sonic_hacks/.aeon-as-fold`
  (detached at aeon `4f5ad5a1`) with this parcel's binary, all CRC32+size
  unchanged against `crates/sigil-harness/golden/provenance.toml`: s4
  `14ee2440`/719700, s4.debug `142294b3`/737683, demo `0c456778`/96474,
  demo.debug `2e603d53`/101339. mtimes moved on every one — a matching CRC on a
  byte-neutral parcel cannot witness that the build ran.
- Full suite, `SIGIL_STRICT_GATE=1`: 4265 passed / 0 failed / 2 ignored
  (master 4259/0/2, +6 = this parcel's tests).
- `cargo clippy --release --all-targets -- -D warnings` exit 0.
- Each of the six new tests was proven red by a mutation shown applied from disk,
  restored from the committed baseline between runs: disabling the `exec_one`
  hook reds four; disabling the literal/comment skip arms reds the inertness
  test; removing the bind-time `interp_text` reds exactly the binding-time test
  and nothing else.
