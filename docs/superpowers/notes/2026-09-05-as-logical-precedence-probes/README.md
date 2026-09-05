# AS operator-precedence probes, 2026-09-05

What asl's binary-operator ladder actually is, measured rather than recalled.

## How to re-run

```
./run.sh golden.asm      # the listing every expectation in
                         # crates/sigil-frontend-as/tests/as_operator_precedence.rs comes from
./run.sh ladder.asm  ; python3 digest.py ladder.lst
./run.sh ladder2.asm ; python3 digest.py ladder2.lst
./run.sh ladder3.asm ; python3 digest.py ladder3.lst
./run.sh lxor.asm        # `!!` and `~~`, which sigil does not lex
```

`run.sh` sources `../asl-reference/asl_ref.sh`, which refuses any build but md5
`61e672562465725a8c102288a7da9098`. Every listing here exited 0 with `0 errors`
and `0 warnings`.

## The probe shape, and why the listings are committed

`gen.py`, `gen2.py` and `gen3.py` emit each probe as THREE `dc.b` lines: the bare
expression `a op1 b op2 c`, and both candidate parenthesisations `(a op1 b) op2 c`
and `a op1 (b op2 c)`. `digest.py` reads which candidate the bare form matched.

The two candidates being in the SAME listing is what makes a probe honest. Where
they emit the same byte the probe cannot distinguish the two parses, and
`digest.py` prints `CONFOUNDED` rather than a verdict. Four probes came back
confounded and are excluded from the ladder derivation:

| probe | why it cannot distinguish |
|---|---|
| `3*1&&0` | both parses fold to 0 |
| `1<5&&0` | both parses fold to 0 |
| `0<1&&0` | both parses fold to 0 |
| `1&&0&&1`, `0||1||0` | logical and/or are associative, so grouping cannot show |

`A>4&&B<5` is the same failure mode with a live consequence: it folds to `1`
under asl's ladder AND under the C ladder sigil shipped, so it agrees with both
and is evidence for neither. It is kept in the test as a labelled control.

Unlike the neighbouring probe directories this one commits its `.lst` files. The
derivation rests on 73 pairwise verdicts and the confound analysis above, and a
reader who cannot see the listings has to take the digest tables on trust. The
`.p` intermediates are not committed.

## The derived ladder, tightest first

```
  <<  >>
  &
  |
  !                       (bitwise xor)
  *  /  #
  +  -
  &&
  ||
  !!                      (logical xor; sigil does not lex it)
  =  <>  <  >  <=  >=
```

`&&`, `||` and `!!` are normalising: `6&&3`=1, `4||2`=1, `5!!3`=0. `~~` is
prefix logical not: `~~0`=1, `~~7`=0.

Three tiers of that are not where a C reader would put them, and all three were
wrong in sigil before this directory existed: the shifts and the bitwise
operators bind TIGHTER than `*`, `!` is looser than `|` instead of sharing its
tier, and the comparisons are the loosest tier rather than a middle one.
