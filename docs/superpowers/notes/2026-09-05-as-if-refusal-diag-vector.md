# AS-IF-REFUSAL-DIAG-VECTOR: asl does not clamp a negative `substr` pos

**Branch** `parcel/as-if-refusal-diag-vector` off master `c8b2ed72`.
**Fix** `2e72c88a`, one file (`crates/sigil-frontend-as/src/eval.rs`).

**Reference** `asl` AS V1.42 Beta Bld 212, md5 `61e672562465725a8c102288a7da9098`, at
`skdisasm/build_tools/Linux-x86_64/asl`. Every value below comes from an `asl -L` listing that
reported `0 errors` and exited 0. `s2disasm`'s asl (`0dee1f98e6480a4783d27ffd8b90896f`) was not
run at any point.

---

## 1. What was broken

`eval_substr` opened with

```rust
if pos < 0 {
    return None;
}
```

`None` is the poison path, so a `substr` with a negative `pos` did not evaluate. That made
`aeon/engine/debug/debugger.asm:572` unassemblable:

```
elseif (strlen(OPERAND)>4)&&(substr(OPERAND, strlen(OPERAND)-4, 4)="(pc)")
```

and four `diag_assert_vector` tests red under the strict gate.

This was an **over-fire, not a correct refusal**: asl decides the condition. Reduced to six lines,
asl takes the else branch and emits `BB` (exit 0), where sigil errored. The goldens were therefore
never in question and were not touched.

---

## 2. asl's out-of-range `substr` semantics

### 2.1 The length law (exact, and fully derivable)

```
avail = max(0, strlen - pos)
count = avail                if len == 0
        0                    if len <  0
        min(len, avail)      if len >  0
```

Probes, all from one exit-0 listing:

| probe | asl | what it establishes |
|---|---|---|
| `substr("abcde",7,2)` | `""` | `pos` past the end is **not an error** |
| `substr("abcde",5,2)` | `""` | `pos` exactly at the end |
| `substr("abcde",3,10)` | `"de"` | `len` past the end clamps to the tail |
| `substr("abcde",1,0)` | `"ello"` | `len == 0` means "to the end" |
| `substr("abcde",1,-1)` | `""` | negative `len` yields empty |
| `substr("abcde",-2,-1)` | `""` | negative `len` beats negative `pos` |

The negative-`len` row is load-bearing rather than academic: `debugger.asm`'s `%<...>` decoder
reaches `substr(string, len, -1)` when a token carries no trailing param (`%<.w d0>`), and the
empty string it yields is what selects the `"hex"` default.

### 2.2 Negative `pos`: asl has behaviour, not semantics

asl does **not** clamp. It copies `count` characters starting at an unchecked offset **below** the
string's buffer, so `avail` **grows**:

| probe | asl | clamp-pos-to-0 would give | yield-empty would give |
|---|---|---|---|
| `strlen(substr("wxyz",-4,0))` | **8** | 4 | 0 |
| `strlen(substr("wxyz",-4,3))` | **3** | 3 | 0 |
| `strlen(substr("wxyz",-4,20))` | **8** | 4 | 0 |

**Why these three values discriminate.** Out-of-range indexing is dense with confounds, and a
single probe here proves nothing: `len == 3` returns `3` under both the real law and a
clamp-to-zero reading, so on its own it separates nothing. The `len == 0` and `len == 20` rows are
what break the tie, and they break it in the one direction no clamping model can produce: a result
**longer than the source string**. `"wxyz"` is 4 characters, and asl returned 8. No reading that
keeps `pos` inside the string can do that. The `len == 3` row is retained precisely because it is
the one that agrees across models: it pins that a positive `len` still clamps against the enlarged
`avail` rather than against `strlen`.

**Content.** The bytes read below the buffer are whatever asl's allocator left there. Measured with
`[`/`]` sentinels, which is what makes an empty result distinguishable from a NUL-only one:

```
dc.b "[", substr("wxyz",-1,2), "]"   ->  5B 00 77 5D
dc.b "[", substr("wxyz",-2,3), "]"   ->  5B 00 00 77 5D
dc.b "[", substr("wxyz",-3,4), "]"   ->  5B 00 00 00 77 5D
dc.b "[", substr("wxyz",-4,5), "]"   ->  5B 00 00 00 00 77 5D
dc.b "[", substr("Q",-4,5), "]"      ->  5B 00 00 00 00 51 5D
dc.b "[", substr("",-4,4), "]"       ->  5B 00 00 00 00 5D
dc.b "[", substr("longer-string-here",-4,6), "]" -> 5B 00 00 00 00 6C 6F 5D
```

NUL at offsets -1..-7, -9 and -10, but **`$91` at offset -8**, stable across two runs and two
different source files. That single non-NUL byte is the whole point: this is an **out-of-bounds
read, not a defined result**. Confirmed directly, and this is the decisive probe:

```
dc.b strlen(substr("a",-1000000,0))   ->  asl SEGFAULTS, exit 139
```

An assembler that segfaults on an input does not have a semantics for it.

### 2.3 `&&` does not short-circuit

```
dc.b (0)&&(1/0)   ->  error: division by 0        (asl, exit non-zero)
dc.b (0)&&(1/1)   ->  00                          (asl, exit 0, control)
```

The left operand is `0`. "division by 0" is an evaluation-time diagnostic and can only be reported
if the division was actually performed, so the right operand ran anyway. The `(1/1)` control shows
the error is specific to performing the division and not to the `&&` shape itself. The two probes
were run as separate files because a run carrying an error is not a source of values for the lines
that did assemble.

---

## 3. Which hypothesis was fixed, and why

**H1 (negative `substr` pos). H2 is refuted.**

H2 said `&&` might short-circuit, so the negative `substr` would never be evaluated at the failing
site. Section 2.3 refutes it by direct measurement: asl evaluates both operands. The brief already
suspected as much from the observation that asl evaluates the negative `substr` happily on its own,
and that reasoning holds, but it is now a measurement rather than an inference. H2 needed refuting
on its own terms because, had it been true, it would have been a separate compatibility fact with
separate consequences (a right operand with a side effect, or one that legitimately errors).

The fix reproduces the **length law exactly** and models the unreadable prefix as **NUL**. That is
byte-identical to asl at every offset where the measured prefix was NUL, and, more importantly, it
decides every *comparison* the way asl does regardless of what garbage is actually there: an AS
string literal cannot contain a NUL, so a NUL-prefixed substring can never compare equal to one.
At the failing site the left operand `strlen(OPERAND)>4` is exactly what forces `pos` negative when
it is false, and the right operand then folds to `0` whatever the prefix holds.

A count above `MAX_SUBSTR_CHARS` (65536) refuses rather than attempting the allocation, because
asl does not survive that case either (exit 139) and there is nothing to be compatible with. Only
a negative `pos` can drive the count above the source string's own length, so no real source
approaches it.

---

## 4. Does the defect predate `d9f00a3e`? Yes, by two months

Confirmed from history, not taken on report:

- The `if pos < 0 { return None; }` guard was introduced by **`d59bab36` (2026-07-04)**, the
  original string-builtins landing, found with `git log -L` on the function body rather than by
  dating the directory.
- The guard is **present at `742c7366`**, the commit where these four tests passed
  (`git show 742c7366:...eval.rs` shows it at line 1816).

So `d9f00a3e` did not create this. Before it, an unevaluable condition was read as silently false,
which at this site happens to be the answer asl gives, so the output was accidentally correct.
`d9f00a3e` converted an accidentally-right silent answer into a visibly-wrong loud one. That is the
loud-refusal parcel working as designed and surfacing a two-month-old latent defect, which is the
class of thing it was built to surface.

---

## 5. The four tests: red first, then green

Both runs `SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-ref`.

**Red at the committed baseline `c8b2ed72`** (no mutation needed; this is a live regression):

```
test result: FAILED. 11 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    assert_b_no_dest_tst_form
    assert_l_with_dest_odd_parity_no_pad
    assert_w_with_dest_symbol_immediate
    raise_error_with_byte_arg
```

**Green at `2e72c88a`:**

```
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 5.1 What their green does and does not prove

`as_reference()` in `diag_assert_vector.rs` assembles through **sigil's own AS front end**, not
through asl. These four tests are an EMP-versus-AS-frontend equivalence check, and they were red
because the AS side could not assemble the macro tower at all. **Their green is not by itself
evidence that sigil's `substr` matches asl.** That grounding comes from the three new unit tests in
section 5.2, whose expectations are derived from asl listings. Recorded because the opposite is an
easy over-claim to make from a green vector.

### 5.2 Three new asl-derived regression tests

Added to the T9.1 block in `eval.rs`, in the house style of the existing
`substr_len_zero_means_to_the_end`:

- `substr_negative_pos_counts_below_the_buffer` pins `8 / 3 / 8` (section 2.2).
- `substr_negative_pos_prefix_is_nul` pins the sentinel-bracketed bytes.
- `substr_negative_pos_decides_a_guarded_if` pins the reduced `debugger.asm:572` site emitting `BB`.

**Red-first, mutation shown applied on disk.** Restored the `d59bab36` guard into the committed
file, verified by content grep rather than `git diff --stat` (a `git checkout <rev> -- <path>`
stages, so a plain `git diff` reports nothing):

```
2073:        } // MUTATION: baseline d59bab36 guard restored for the red-first proof
```

```
test result: FAILED. 0 passed; 3 failed; 0 ignored
failures:
    eval::tests::substr_negative_pos_counts_below_the_buffer
    eval::tests::substr_negative_pos_decides_a_guarded_if
    eval::tests::substr_negative_pos_prefix_is_nul
```

`substr_negative_pos_decides_a_guarded_if` failed with the exact production message
(`unresolved if condition: it does not evaluate...`), so the proof exercised the real path and not
a lookalike. Restored, then green: `3 passed; 0 failed`.

---

## 6. Strict gate

```
SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-ref \
  cargo test --release --workspace --no-fail-fast
```

The log is stamped with its own tree, because a suite log does not otherwise name one:

```
pwd=/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae3b7086eb428aba3
HEAD=2e72c88ad86eeba64cb2132455ea9faa7042a873
branch=parcel/as-if-refusal-diag-vector
```

**Failures first: 1 failed, 4616 passed, 2 ignored.**

| test | verdict |
|---|---|
| `repin_pins::pins_rs_is_current` | **KNOWN, not mine.** Declares `src/pins.rs` STALE while reporting **`0 changed pin(s)`**. Booked separately; fails on master independent of this work. Not touched here. |

Exactly one `test result: FAILED` line in the whole log, so nothing else hid behind the known row.

**⚠ CORRECTED 2026-09-05: the sentence below reads a number that does not describe the failing verdict.** That gate's verdict came from a WHOLE-FILE comparison and its `0 changed pin(s)` count from a values-only one, so on that very run the same output declared the file STALE. The conclusion happened to be true and the evidence offered for it was the wrong number. Byte identity here rests on the strict run and the golden gates, not on that count. Original text follows.

The `0 changed pin(s)` is also positive evidence in its own right: this parcel moved no pinned
aeon bytes.

**Reconciliation.** 4616 passed + 1 failed = 4617 tests, against master's 4614, and the diff adds
exactly three `#[test]`. Master's own strict totals would therefore be 4609 passed / 5 failed
(the four `diag_assert_vector` rows plus the pins row). That last figure is **arithmetic, not
measured**: the only baseline run made here was `diag_assert_vector` alone (11/4).

**The reference tree was read, never written.** Its ROMs were not rebuilt.

---

## 7. Corpus effect on `s2disasm`

Run from a detached worktree at `e45ebf33` under a run-unique path
(`/home/volence/sonic_hacks/.s2corpus-ae3b7086`); the owner's checkout was never written to. Both
binaries built from this worktree into `.target-land`, and they are distinct
(`edd43c82...` before, `80d5fa9e...` after).

**Result: zero delta. The output is byte-identical, `cmp`-clean, 5247 diagnostics either side.**

Per-class decomposition, both directions, over 59 distinct message classes each side:

```
APPEARED (after minus before):  (empty)
VANISHED (before minus after):  (empty)
```

**No class rose. No class appeared. No class fell.** Counts are identical class by class because
the files are byte-identical.

Top classes, unchanged either side: `bad operand expression` 2624, `expected mnemonic, directive,
or label` 2309, `is not a recognized 68000 mnemonic` 89, `bad word expression` 49, `bad byte
expression` 30, `unresolved symbol in operand` 24.

### 7.1 The zero is real, and here is why it is zero

A zero that would be a finding does not get believed here without a canary, and this one is a
finding in both directions the brief warned about.

**Why zero:** `s2disasm` contains exactly two `substr` call sites, and **neither can reach a
negative `pos`**:

```
s2.macrosetup.asm:104  chkop function op,ref,(substr(lowstring(op),0,strlen(ref))<>ref)
s2.macrosetup.asm:280  extractJmpToName function name,val(substr(name, strstr(name,"_")+1, strlen(name)))
```

The first is a literal `0`. The second is `strstr(...)+1`, and `strstr` returns `-1` at worst, so
the floor is `0`. The corpus **structurally cannot exercise the fixed path**, which is the correct
explanation for a zero delta and is not the same statement as "the fix is inert".

**Canary, because the above is an argument and not a measurement.** Planting one negative-`pos`
site into the corpus worktree and re-running both binaries:

```
before: 5248 lines   after: 5247 lines
< s2.macrosetup.asm(346): error: unresolved if condition: it does not evaluate, ...
```

The corpus harness **does** register this fix when the path is reachable. The zero is therefore a
true zero and not a dead instrument. The worktree was restored (`0 modified`).

A separate canary covered the set-difference tooling itself: a planted class in the `after` set is
correctly reported by `comm` as APPEARED, so the two empty lists above are real emptiness rather
than a broken pipeline.

### 7.2 Two unresolved-`if` rows survive, and they are not this defect

`s2.macrosetup.asm(20)` and `(22)` still report unresolved `if`/`elseif`, identically before and
after. They are the `org` macro's `if address < *` / `elseif address > *`, a macro-parameter and
`*` resolution gap with no `substr` in it. Observed, out of scope, not claimed as fixed.

---

## 8. Anything in this brief I concluded was wrong

**1. "Establishing asl's exact out-of-range `substr` semantics ... derive each from an exit-0
listing" is not fully achievable as written, and the part that is not achievable is the
interesting part.** The *length* law is exact and derivable, and section 2.1/2.2 derives it. The
*content* under a negative `pos` is not a semantics at all: it is an unchecked read of asl's own
heap, which returned `$91` at offset -8 while returning NUL at -1..-7. There is no exit-0 listing
that can establish "the correct value" of a byte asl never owned. The brief's framing implies a
defined behaviour waiting to be measured, and the honest answer is that a defined behaviour does
not exist there. This changes the shape of the fix from "match asl" to "match asl's length law and
choose the prefix", and the choice needs its own justification, which section 3 gives.

**2. H1 as stated bundles two cases that are not the same defect.** The brief reads "`substr` with
an out-of-range **or negative** position" as one hypothesis. Out-of-range **high** (`pos` past the
end, `len` past the end) was **already correct** in sigil before this parcel and is a properly
defined asl behaviour. Only the **negative** case was broken. Anyone reading the brief would expect
to find and fix a family of out-of-range bugs; there was one, and the other half of the family was
already right. Worth separating because "we fixed out-of-range substr" would overstate what
changed.

**3. The four tests going green is weaker evidence than it looks, and the brief treats it as the
headline.** `diag_assert_vector`'s "AS reference" is sigil's own AS front end, not asl (section
5.1). Those four tests would have gone green for *any* change that let the AS front end get through
`debugger.asm`, including a wrong one, such as clamping `pos` to `0`, which also assembles and also
yields a false condition at this site but disagrees with asl on the length law. **The four tests
cannot distinguish the correct fix from that wrong one.** The asl grounding lives entirely in the
three new unit tests, which is why they were written rather than treating the vector's green as
sufficient.

**4. Minor, on the brief's own uncertainty.** The brief hedged H2 as "evidence against H2 being the
explanation, not proof". That hedge was appropriate and the direction was right, but H2 is now
positively refuted rather than merely unsupported, which is a stronger result than the brief
expected to be available.

**5. Not checked.** The claim that `SIGIL_ALLOW_PARTIAL=1` skips 127 reference-dependent binaries
was taken as background and not verified; nothing here depends on the number. The claim that
`repin_pins::pins_rs_is_current` fails on master independently was likewise not re-measured, though
its own `0 changed pin(s)` output is consistent with it. **CORRECTED: that count does NOT show the parcel moved no pins, because it describes a different comparison from the verdict that was failing beside it.**
