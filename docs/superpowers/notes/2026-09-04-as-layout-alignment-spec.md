# AS layout alignment: the asl-minted specification

Every row below is a value asl printed, not a value derived from reading AS's
semantics. Probes were run against **both** assembler builds, which agree on
every case:

| build | path | md5 | banner |
|---|---|---|---|
| v1 (vanilla Arnold) | `s1disasm/build_tools/Linux-x86_64/asl` | `61e672562465725a8c102288a7da9098` | `Macro Assembler 1.42 Beta [Bld 212]` / `(x86_64-unknown-linux)` |
| v2 (flamewing fork) | `s2disasm/build_tools/Linux-x86_64/asl` | `0dee1f98e6480a4783d27ffd8b90896f` | `Macro Assembler 1.42 Beta [Bld 212]` / `(x86_64-Linux)` |

Invocation: `asl -xx -n -q -A -L -U -i . <file>`, then
`p2bin -p=0xFF <file>.p <file>.bin`.

**The `-p=0xFF` fill is the instrument that makes this measurable.** A pad byte
asl really emits reads `00`; an address asl merely skips reads `ff`. Under the
`-p=0` the disassemblies use, the two are indistinguishable — which is how a
"verified against asl: fill byte is 0x00" claim was recorded for a construct
that emits no byte at all.

Symbol values are read from the listing's **symbol table**, never from the
listing's PC column. The two disagree, and the PC column is the decoy: for a
lone label the PC column shows the pre-pad address while the symbol table shows
the post-pad one.

## Rule 1 — `align` never emits image bytes

`align <n>` advances the location counter and contributes nothing to the image.

```
	cpu	68000
	padding	off
	org	0
	dc.b	$11
	align	2
```

asl object file (`a2` variant, with a `dc.b $22` after the align) contains
exactly two records — `11` at address 0 and `22` at address 2. There is no
record for address 1:

```
89 14 81 01 01 01 00 00 00 00 01 00 11 81 01 01
01 02 00 00 00 01 00 22 ...
       ^addr=2      ^len=1 ^byte
```

and the fill choice decides the gap byte, proving it is a hole:

```
p2bin -p=0x0  ->  11 00 22
p2bin -p=0xFF ->  11 ff 22
```

| probe | source tail | asl image | note |
|---|---|---|---|
| `a1` | `dc.b $11` / `align 2` | `11` (len 1) | trailing align adds no bytes |
| `a4` | `dc.b $11` / `align 4` | `11` (len 1) | likewise, whatever the boundary |
| `a2` | `dc.b $11` / `align 2` / `dc.b $22` | `11 <hole> 22` (len 3) | gap is a hole, not a `$00` |
| `a5` | `dc.b $11,$22` / `align 2` | `11 22` (len 2) | no-op when already aligned |

`align` is independent of `padding`: `a1` (`padding off`) and `a3`
(`padding` defaulted on) both give length 1.

## Rule 2 — the implicit even-pad, and what it applies to

Under `padding on` (AS's 68000 default) asl advances the PC to the next **even**
address before a word-or-larger item at an odd address. The listing marks this
with a `<padding>` pseudo-line.

Two independent axes decide what happens:

**(a) which items trigger it** — element size > 1 byte, whether the item emits
or merely reserves:

| probe | item at odd `$` | pads? | resulting label |
|---|---|---|---|
| `c05` | `dc.b` | no | 1 |
| `c04` | `ds.b 1` | no | 1 |
| `c06` | `dc.w` | yes | 2 |
| `c02` | `ds.w 1` | yes | 2 |
| `c10` | `ds.l 1` | yes | **2** |
| `c12` | `move.w #1,d0` | yes | 2 |
| `c11` | `ds.w 1`, no label at all | yes | — |

`c10` is the first thing a reader guesses wrong: `ds.l` at address 1 pads to
**2, not 4**. The implicit pad is always to a 2-byte boundary regardless of the
element's own size. `c14` (`ds.l` at address 3) gives 4, which is consistent —
it is the next *even* address, and 4 only looks like a longword boundary by
coincidence.

`c11` is the second: the pad is **not** label-driven. It fires with no label
present.

**(b) whether the pad byte is materialized** — emitting items produce a real
`$00`; reserving items leave a hole:

| probe | item | image under `-p=0xFF` |
|---|---|---|
| `c06` | `dc.w $3344` | `11 `**`00`**` 33 44` — real emitted pad |
| `c12` | `move.w #1,d0` | `11 `**`00`**` 30 3c 00 01` — real emitted pad |
| `c02` | `ds.w 1` | `11 `**`ff ff ff`**` 33` — pad and reservation both holes |
| `c10` | `ds.l 1` | `11 `**`ff ff ff ff ff`**` 33` |

**The whole of rule 2 is gated on `padding`.** Under `padding off` no implicit
pad happens for any item:

| probe | source | asl `Lbl` |
|---|---|---|
| `e06` | `padding off`, lone `Lbl:` then `dc.w` at odd | 1 |
| `e03` | `padding off`, lone `Lbl:` then `ds.w` at odd | 1 |

## Rule 3 — exactly one label absorbs the pad

**The rule is not the one this was filed under.** It is not about `ds`, and it
is not about lone labels specifically. The single **most recently defined**
label takes the address after the pad — whether it sits in the padding line's
own label column or alone on the line above. Never two, never zero.

The listing's PC column actively hides it.

`c03`: `dc.b $11` / `Lbl:` / `ds.w 1`. The listing reads

```
       3/       0 : 11                  	dc.b	$11
       4/       1 :                     Lbl:
       5/       1 :                     <padding>
       5/       2 :                     	ds.w	1
```

— PC column says `Lbl` is at 1 — while the symbol table says

```
*Lbl :                            2 C
```

The exceptions, each an asl-minted value:

1. **Only the LAST label of a run defers.** `c09`/`e08`: `Lbl:` / `Lb2:` /
   `ds.w 1` gives `Lbl : 1`, `Lb2 : 2`. Two labels on consecutive lines pointing
   at the same place end up with *different* values.
2. **`align` does not participate.** `c07`/`d07`: lone `Lbl:` then `align 4`
   gives `Lbl : 1`, not 4. The deferral belongs to the implicit even-pad only;
   an explicit `align` never back-propagates to a label above it.
3. **A comment line is transparent.** `c13`: `Lbl:` / `; comment` / `ds.w 1`
   gives `Lbl : 2`. The deferral survives non-advancing lines.
4. **It is not a `ds` phenomenon.** `e07`: lone `Lbl:` then `dc.w $3344` at odd
   gives `Lbl : 2` as well. The deferral applies to every item that triggers the
   implicit pad — emitting or reserving.

Exception 4 matters for scoping: the deferral is a defect in its own right,
independent of whether `ds` pads, and it is reachable through `dc.w`/`dc.l`/
instructions today.

At end of file with nothing after it, a lone label simply takes the current
address — `c08` gives `Lbl : 1` and an image of length 1.

### Which label, when there are two candidates

A label in the padding line's OWN label column outranks one carried from the
line above, and the carried one is then left where it was:

| probe | source | asl |
|---|---|---|
| `g1` | `Lbl:` / `Lb2: ds.w 1` at odd | `Lbl : 1`, `Lb2 : 2` |
| `g2` | `Lbl: dc.w $3344` at odd | `Lbl : 2` |
| `g3` | `Lbl: ds.b 1` at odd | `Lbl : 1` (line does not pad) |

`g1` is the case that collapses the rule into one sentence: two labels written
at the same place end with **different** values, and the one that moves is the
later one. That is the same "only the last of a run" behaviour `c09` shows, so
both are one rule about the most recent label — not two rules about lines.

### What ends a deferral

Strictly: **any line that dispatches anything**, including lines that emit
nothing at all.

| probe | line between the lone label and the padded item | asl `Lbl` |
|---|---|---|
| `c13` | a comment | **2** — transparent |
| `f1` | `align 3` (lands odd, so the `dc.w` still pads) | 1 |
| `f2` | `Other: equ 7` | 1 |
| `f3` | `ds.b 0` | 1 |
| `f4` | `dc.b $99` | 1 |

`f2` and `f3` are the sharp ones: neither emits a byte or moves `$`, and both
still end the deferral. Only blank and comment-only lines are transparent.

## Reachability: none of this is exercised by anything we build

Stated plainly because a fix nothing exercises is still worth landing, and worth
saying so. All three consumers are clear of both constructs, and for reasons
stronger than "the addresses happen to come out even".

**AS's builtin `align` is never reached by either disassembly.** Both shadow it
with a macro that lowers to `cnop` → `org`:

- `s1disasm/MacroSetup.asm:63` — *"redefine align in terms of cnop, because the
  built-in align can be stupid sometimes"*
- `s2disasm/s2.macrosetup.asm:46` — *"redefine align in terms of cnop, for the
  padding counter"*

There is no `!align` (the builtin-escape spelling) anywhere in either tree, and
zero trailing `align` at end of file among the 4 (s1) and 14 (s2) occurrences.

**The implicit even-pad is never reached by either disassembly.** Both set
`padding off` before the first emitted byte and never turn it back on —
`s1disasm/MacroSetup.asm:6` and `s2disasm/s2.macrosetup.asm:2`, each the first
directive in the file. The two further `padding off` lines in each corpus are
re-assertions after a `restore` clobbered the flag, not toggles to `on`.

**No `ds.w`/`ds.l` sits at an odd address in either corpus anyway.** All 138 (s1)
and 253 (s2) live in the RAM-map files, and a parity walk honouring every
`phase`/`org`/`dephase` finds zero at an odd offset. The sources are visibly
hand-maintained to guarantee it — singleton `ds.b 1 ; unused` fillers appear
exactly where a run of byte flags would otherwise leave the counter odd, which
is what a source written under `padding off` has to do for itself.

**Aeon reaches neither.** Its shipping build routes three tracked `.asm` files
through this frontend (`engine/debug/debugger.asm`, `games/demo/game_root.asm`,
`games/sonic4/game_root.asm`), and they contain no `align`, no `ds.b`/`ds.w`/
`ds.l` and no `even` between them; both game roots declare `padding off`.
Measured, not assumed: all four ROM shapes are byte-identical across this
change.

So the corpus diagnostic counts cannot move, and did not — Sonic 1 stays at 65
and Sonic 2 at 6,035, with the two diagnostic sets identical line for line.
Note that a corpus BYTE comparison is not available as evidence here: neither
corpus assembles to completion under those diagnostics, so sigil emits no image
for either and `--hex` returns zero bytes. The byte evidence is the 33 probes
and aeon's four shapes.

## Probe corpus

`.sigil-layoutdiv-probe/` (untracked scratch, regenerable from this note).
`mint.sh` dumps one probe against both builds with the full listing; `sym.sh`
is the compact form used for the tables above (both builds' bytes under
`-p=0xFF` plus every user symbol's value).
