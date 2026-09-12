# AS-MACRO-LABEL-LEAK: a label made inside a macro, used outside it

Queue row, verbatim: *"Silent: a label made inside a macro can be used outside
it here, which the old assembler forbids. A program could build here that is
wrong there. Needs a reproduction first."*

Branch `parcel/as-macro-label-leak`, base `3bc0d81a`.

## Provenance

* Oracle: `s1disasm/build_tools/Linux-x86_64/asl`, md5
  `61e672562465725a8c102288a7da9098`, selected and run through
  `asl-reference/asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .`
  (the corpus's own minus `-E`/`-c`). Bytes come from the `p2bin` beside it, and
  only from runs that exited 0.
* sigil: built from this worktree into its own target dir, `sigil <f> --hex`.
* Every probe is placed at `org $100` behind a `$1111` filler word, so a bound
  label reads as a non-zero address and a zero cannot pass for an answer.
* One shape per file, one suspect line per file (marked `; REF`), because an
  asl run carrying any error is not a source of values.

Probes and runners are in `2026-09-12-as-macro-label-leak/`:
`gen.py` (84 shapes, `probes/`), `gen2.py` (54 shapes, `probes2/`), `genx.py`
(9 counter probes, `xprobes/`), `matrix.sh` (both assemblers, one row per shape),
`classify.py` (the verdict column), `xrun.sh` (asl's nameless symbol names).

## Stage 0: reproduced

The row has a committed source after all: `src/nameless.rs` lines 77-90 and
`2026-09-09-as-nameless-labels.md` section "Measured and deliberately NOT
implemented: macro-body scoping" record exactly this, as a known divergence
left unmodelled. Two more open items in `2026-09-05-as-macro-body-label.md`
("Left open": `enum` members; names produced by substitution) are the same
row in other spellings.

At base `3bc0d81a`, 138 shapes:

| batch | shapes | MATCH | LEAK (asl refuses, sigil builds) | OVER-REFUSE (asl builds, sigil refuses) |
|---|---|---|---|---|
| `probes/` | 84 | 51 | 25 | 8 |
| `probes2/` | 54 | 36 | 10 | 8 |

Full per-shape tables: `compact-base1.txt`, `compact-base2.txt`.

The leaks, by mechanism:

* **Nameless labels** (`+`, `-`, `/`): c01 c02 c03 c04 c05 c08 c10 c11 c12
  c14 c15 i03 in6 n02 n03 n10 n13 n20 n21. asl keeps one nameless counter per
  pass but files each definition in the namespace of the expansion instance (a
  macro expansion or one loop iteration) that is innermost when it is written.
  sigil files every one globally.
* **A plain label whose name the body text does not spell**: g01 g02
  (`__LABEL__`), g04 g05 (`{expr}`), g06 g07 g08 s06 s07 (the name arrives as
  argument text), g14 (`ALLARGS`). The body scan reads raw text, misses the
  name, and the fallback files it globally.
* **`enum` / `nextenum` members** written in a body: h01 h04.
* **A label in a file `include`d from a body**: i01. The include cleared the
  namespace stack.
* **A `.x` under a plain label of the body**: a10 (`Lp.x` read from outside).
* **A `{GLOBALSYMBOLS}` body's `.dl:` defined twice**: gs7 (asl `#1000`).
* a12 (a `.b :=` written after the call, read as `Base.b`) is the maclocal
  parcel's known open scope-change divergence, not this row; see "Left open".

The over-refusals are mostly the other face of the same leaks: a name filed
globally collides when a second expansion writes it again (g09 g10 i02 in1 in2
in4 dx2), and the include clearing the stack also strands a read of the body's
own label from inside the included file (in3). The rest are `{GLOBALSYMBOLS}`,
which sigil did not recognise at all (f_globalsymbols, _dot, _forward, _lower,
gs1 gs6), and the a11/gs5 scope-change pair.

### asl's nameless counters are not what sigil models, and not only in macros

`xprobes/` read asl's own symbol names (`__forwN`, `__backN`, zero-based):

```text
x1  file: +, ++, +        __forw0 = 102   __forw2 = 104   __forw1 = 106
x5  file: ++, +           __forw1 = 102   __forw0 = 104
x2  body ++ ; file +, +   __forw0 = 104   __forw1 = 106
x3  body +  ; file +, +   __forw1 = 104   __forw2 = 106
```

A `+` run of length m >= 2 defines `__forw(c + m - 1)` and does NOT advance the
forward counter; only a single `+` advances it. sigil advances by m. The two
agree on every reference the 2026-09-09 probes made (`q8` has no `+` after its
`++`), and disagree on a `+` defined after a `++` with a reference spanning
both. This is a separate row, recorded here and not changed; it is why n21
stays divergent after the scope fix. Also measured and not modelled: a `/`
written in a macro body advances asl's forward counter and not its backward
one (x7: after body `/`, file `+` is `__forw1`, file `-` is `__back0`).
