# 2026-09-15, `AS-STRING-*`: asl's string typing, measured before anything was built

Parcel: the cluster `AS-STRING-PLUS-NUMERIC-CONTEXT`, `AS-STRING-PLUS-INT`,
`AS-STRING-SYMBOL-INT-SLOT` (ledger section 2026-09-13). Branch
`parcel/as-string-numeric-typing`.

Every asl value below is from the pinned reference build, md5
`61e672562465725a8c102288a7da9098`
(`/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`), selected
through `docs/superpowers/notes/asl-reference/asl_ref.sh` and invoked through
its `asl_run`, which prints `ASL_EXIT` and refuses out loud on a non-zero
status. The exact command is

```
asl_run -xx -n -q -A -L -U -i . <probe>.asm
```

run by `probes/run.sh` (whole files) and `probes/one.sh` (one subject per run).
**Every value quoted here comes from a run that exited 0**, except where the
refusal IS the measurement, which is said each time. The probes are committed
beside this note.

`probes/sig.sh` is the same shape for sigil, so a BEFORE/AFTER row is one
command each.

## 1. The law, in three rules

The matrix does NOT need a non-uniform rule. Three rules cover every cell
measured, and the surprise is that they are this clean.

**R1. TYPE. `+` is the only operator that propagates stringness.** Every other
operator, unary `-` and `~` included, packs a string operand to its integer
(big-endian over the code page's bytes) and yields an INTEGER.

```
dc.w "ab"-1     6161      dc.w "ab"&$00FF  0062      dc.w ~"ab"   9E9D
dc.w "ab"*2     C2C4      dc.w "ab">>8     0061      dc.w "ab"/2  30B1
dc.w -"ab"      9E9E      dc.w "ab"|1      6163      dc.w "ab"^1  6162   (`^` is POWER, not xor)
```

Each of those is ONE word, so the value is an integer and not a two-character
string. Parentheses are transparent to the type: `dc.w ("ab")` is `0061 0062`,
still a string.

**R2. RENDERING, and it depends on the SLOT.** One string value has two
renderings, and this is the asymmetry the ledger warned about:

| slot | rendering | measured |
|---|---|---|
| `dc.b` | one byte per character | `dc.b "ab"` -> `61 62` |
| `dc.w` | one WORD per character, zero-extended | `dc.w "ab"` -> `0061 0062` |
| `dc.l` | one LONG per character, zero-extended | `dc.l "ab"` -> `00000061 00000062` |
| immediate `#` | PACKED big-endian into one integer | `move.w #"ab",d0` -> `303C 6162` |
| absolute address | PACKED, identically | `move.w "ab",d0` -> `3038 6162` |

`move.l #"abcd",d0` is `203C 6162 6364`; `lea "abcd",a0` is `41F9 6162 6364`.
So "string in an integer slot" is ONE answer (pack) and "string in a data
directive" is another (per character), and `move.b #"a",d0` is `103C 0061`.

**R3. `+`'s VALUE.**

- **string + string is CONCATENATION**, at any length:
  `dc.b "ab"+"cd"` -> `61 62 63 64`, `dc.b "abcde"+"f"` -> `61 62 63 64 65 66`.
- **string + integer (either order) is PACKED ARITHMETIC**: pack the string
  big-endian, add, and render the sum **in its MINIMAL number of whole bytes**.

The length rule is the one thing this parcel got wrong twice before measuring
it. It is **not** "keep the string's length" (the ledger's wording, which is
right for every one-byte-aligned case and wrong in general) and **not**
`max(len, needed)`. It is purely the minimum bytes the SUM needs:

```
dc.b "a"+1          62              1 byte   (0x62)
dc.b "ab"+1         61 63           2        (0x6163)
dc.b "abc"+1        61 62 64        3
dc.b "abcd"+1       61 62 63 65     4
dc.b "ab"+256       62 62           2        (0x6162 + 0x100)
dc.b "aa"+255       62 60           2        (carry crosses the characters)
dc.b "\xff\xff\xff"+1  01 00 00 00  4        (the carry GROWS it 3 -> 4)
dc.b "\x00a"+1      62              1        (a leading zero byte is DROPPED)
dc.b "\x00ab"+1     61 63           2        (same, and this is the discriminator)
dc.b "a"+(0-97)     <nothing>       0        (the sum is 0: the EMPTY string)
dc.b "a"+(-98)      FF FF FF FF     4        (a NEGATIVE sum takes all four)
```

`"\x00a"+1` is the measurement that settles it: the string is two characters,
the sum is `0x62`, and asl emits ONE byte. A length-preserving rule would emit
`00 62`.

`dc.b "a"+(0-97)` emitting nothing is a genuine EMPTY STRING, not a shape asl
declined. Proof, and it needed one, because "zero bytes, exit 0" is also what a
declined shape looks like: in an integer slot it raises
**`error #1141: expected integer, but got string`**, exactly as `move.w #"",d0`
does, on all four runs.

**Commutativity and the operator tree.** `dc.b 1+"ab"` is `61 63`, the same as
`"ab"+1`. The root of the tree decides the type, with asl's own ladder and
left-association:

```
dc.w "ab"+1-1     6162        root is `-`  -> INTEGER, one word
dc.w "ab"-1+1     6162        root is `+`, both operands INTEGER -> INTEGER
dc.w "ab"+(1-1)   0061 0062   root is `+`, lhs STRING -> STRING
dc.w 1+2+"ab"     0061 0065   ((1+2)+"ab") -> STRING, packed 0x6162+3
dc.w "ab"+1*2     0061 0064   `*` binds tighter, so root is `+` -> STRING
dc.w "ab"+1&$FF   0061 0063   `&` binds TIGHTER than `+` in AS (tier 8 vs 4)
dc.w "ab"+1<<1    0061 0064   `<<` tightest
dc.w "ab"="ab"    0001        a comparison is tier 1 -> INTEGER
dc.w "ab"+1="ab"  0000        and it compares the STRING: "ac" <> "ab"
```

That last row is worth keeping: it shows `"ab"+1` really is the string `"ac"`
and not a number that happens to render as two bytes.

**A string SYMBOL is its literal, in every slot.** With `S1 equ "a"`,
`S2 equ "ab"`, `S4 equ "abcd"`, `S5 equ "abcde"`:

```
dc.b S1  61            dc.w S2  0061 0062      move.w #S1,d0  303C 0061
dc.b S2  61 62         dc.l S2  00000061       move.w #S2,d0  303C 6162
dc.b S5  61 62 63 64 65         00000062       move.l #S4,d0  203C 6162 6364
dc.b S2+1  61 63       dc.w S2+1  0061 0063    move.w #S2+1,d0  303C 6163
dc.b S1+S2  61 61 62   dc.w S2-1  6161         move.w S2,d0   3038 6162
```

Not one cell differs from the same expression written with the literal. That is
what makes `AS-STRING-SYMBOL-INT-SLOT` a resolution gap and not a semantics
gap.

## 2. Where asl STOPS having an answer, and why we must not copy it

**A. A string of length 0, or 5 and longer, in an INTEGER slot: `error #1141`,
exit 2.** `move.w #"abcde",d0`, `move.l #"abcde",d0`, `move.w #"",d0`. Packing
is defined for 1 to 4 characters only. (Sigil's `expr::string_to_int` already
has exactly this rule, `MAX_PACKED_CHARS` and the empty-string `None`, so this
half already agrees.)

A 1-to-4 character string that does not FIT the slot is an ordinary
`error #1320: range overflow`, not #1141: `move.w #"abc",d0`,
`move.b #"ab",d0`.

**B. A `string + integer` whose SUM needs more than four bytes is a shape asl
DECLINES, silently.** This is a new instance of the standing
`ASL-SILENT-WRONG-ON-BOTH-BUILDS` hazard, on the reference build, at exit 0:

```
dc.b "abcde"+1,$EE        emits NOTHING AT ALL, exit 0, no diagnostic
dc.b "abcdefgh"+1,$EE     emits nothing, and swallows the $EE with it
dc.b "\xff\xff\xff\xff"+1,$EE   emits only the EE (the sum needs 5 bytes)
move.w #"abcde"+1,d0      exit 0, no diagnostic, and NOT A VALUE:
```

five consecutive runs of that last line returned

```
303C 5605   303C 0000   303C 564D   303C 5608   303C 55C6
```

and with one accepted `move.w #$1234,d0` above it, three runs all returned
`303C 1234` - the stale-slot echo `asl_ref.sh` documents. **No byte from these
shapes is a value.** Sigil refuses them loudly instead of reproducing either
the silence or the garbage.

Note that string + STRING has no such cap: `dc.b "abcde"+"f"` is fine. The cap
is on the arithmetic, which asl does in 32 bits.

## 3. The `charset` seam, and the one thing left unmeasurable

`charset 'a',$11` then:

```
dc.b "ab"       11 62
dc.b "ab"+1     11 63
```

So the arithmetic runs on the **mapped** bytes (`0x1162 + 1`), not on the
characters' own codes. That agrees with `expr::string_to_int`, which already
maps through the page before packing.

What this probe **cannot** settle is whether the resulting bytes are then
emitted raw or mapped a second time, because `0x11` and `0x63` are both
fixed points of this page. Discriminating it needs a page whose output byte is
itself a mapped input character, and the answer would change no reachable byte:
no corpus writes a string in an arithmetic expression at all. Sigil therefore
**refuses a `string + integer` outright when the active page is not the
identity**, rather than pick one of the two readings and emit a byte it cannot
prove. Booked as what stays open.

## 4. The corpus gate this parcel gets for free

The ledger records, and this parcel re-uses, that **no corpus writes any of
these forms** - a grep of s1disasm, s2disasm, skdisasm, S.C.E. and aeon finds
no `"..."+"` anywhere. So every byte of every corpus output must be identical
before and after, and a single differing byte would be a FINDING (a form
somebody does write) rather than something to repin.
