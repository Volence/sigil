# The path to a Sonic 1 ROM — 318 diagnostics, and the 524,288-byte image behind them

Measurement parcel. No sigil source is changed by it.

**The headline: sigil already emits a full-size Sonic 1 ROM that is 98.6% byte-identical
to the retail REV01 cartridge.** With thirteen source-side stubs standing in for the
constructs the frontend refuses, `sigil sonic.asm -o s1.bin` exits 0 with zero
diagnostics and writes 524,288 bytes, of which 517,003 match `s1built.bin` exactly.
Only **seven** regions differ, and every one of them has a named cause.

## Provenance

| | |
|---|---|
| sigil | `7bef76e6`, `sigil 0.1.0 (7bef76e6)`, md5 `242427190b581e5b5776a1874da990d1` |
| target dir | `/home/volence/sonic_hacks/.sigil-s1recon-target` (outside every shared checkout) |
| corpus | `s1disasm` `f6ece657`, two detached worktrees; the live tree is never written |
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, `Macro Assembler 1.42 Beta [Bld 212] (x86_64-unknown-linux)`, md5 `61e672562465725a8c102288a7da9098` |
| reference ROM | built here by `lua build.lua`: **524,288 bytes, md5 `09dadb5071eb35050067a32462e39c5f`**, `chkbitperfect.lua` says `ROM is bit-perfect with REV01` |
| wall clock | measurements run 00:22–00:38 on 2026-09-04, machine `up 9 days, 16:2x` throughout |

The live `s1disasm` checkout carries two modified `.nem` files and an `s1built.bin` of
551,288 bytes with md5 `a081872bd7269e95a7b64d38d81db553` — **it is not a usable
reference.** Every figure here is against the ROM built in this parcel's own pristine
worktree, which the corpus's own hash gate certifies against the retail cartridge.

### The invocation AS is given, read from the build script rather than guessed

`build.lua` → `common.build_rom_and_handle_failure("sonic", "s1built", "", "-p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after", false, …)`,
and `common.assemble_file` expands that to:

```
asl -xx -n -q -A -L -U -E -i .  sonic.asm
p2bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after  sonic.p  s1built.bin
```

then a Lua `fix_header` that writes the end-of-ROM long at `$1A4` and the checksum word
at `$18E`. **Two of those arguments are load-bearing and neither is an assembler
feature** — see causes L2 and L3 below.

## The 318, reproduced

```
cd <pristine s1disasm worktree>
/home/volence/sonic_hacks/.sigil-s1recon-target/release/sigil sonic.asm -o /dev/null
```

Exit 1. **318 lines on stderr, nothing on stdout, every line severity `error`, every
line matching `file(line): error: …` with no unclassified remainder.** Run twice in two
independently created worktrees; the two 318-line sets are identical after sorting. The
capture is `.s1probe/2026-09-04/s1_318.err.gz`.

They land in **9 files at 67 distinct sites**. The brief's "they may be a long tail" is
wrong in the strongest possible way: **two source lines carry 166 of the 318**, and
seven lines carry 250.

| counts | sites | class as sigil prints it |
|---|---|---|
| 166 | 2 | bad word expression |
| 39 | 39 | `X` is not a recognized 68000 mnemonic |
| 36 | 1 | bad operand expression |
| 18 | 1 | unexpected character |
| 18 | 1 | instruction needs an explicit size suffix (.b/.w/.l) |
| 13 | 2 | unresolved long expression |
| 8 | 1 | unresolved rept count |
| 6 | 6 | case needs a string literal |
| 6 | 6 | bad immediate expression |
| 4 | 4 | trailing tokens in operand |
| 2 | 2 | switch needs a string expression |
| 1 | 1 | unknown directive or mnemonic `X` |
| 1 | 1 | org target precedes the current phase base |
| **318** | **67** | |

## The decomposition by CAUSE — eleven causes, and they sum to 318

Every row was reproduced standalone against **both** assemblers. The probe files are
committed under `.s1probe/2026-09-04/probe/`; `cmp.sh <file.asm>` runs `asl` with S1's
own flags and `sigil` on the same input and prints both.

### F1 — floating point: literals, arithmetic, and `INT()`. 166 counts, 2 lines. Shared.

`s1.sounddriver.asm(1796)` and `(2052)`, the bodies of `MakeFMFrequenciesOctave` and
`MakePSGFrequencies`. Behind them: `MacroSetup.asm(218)`
`roundFloatToInteger function float,INT(float+0.5)`.

sigil has no float type at all. `probe/p2.asm`:

```
       4/       0 :                     roundFloatToInteger function float,INT(float+0.5)
       5/       0 :                     MakeFMFrequency function frequency,roundFloatToInteger(frequency*1024*1024*2/FM_Sample_Rate)
       6/       0 : 025E                	dc.w MakeFMFrequency(15.39)
       8/       2 : 0A5E                        dc.w MakeFMFrequency(15.39)+1*$800
       8/       4 : 0A84                        dc.w MakeFMFrequency(16.35)+1*$800
      10/       6 : 0003                	dc.w INT(3.7)
```

sigil answers `bad word expression` for all four. S2 hits the same wall from the other
side — it reports 23 `int(): could not evaluate float expression`.

**This is the only frontend cause that is also a ROM-byte cause**: it owns 330 bytes of
the retail ROM (`FM_Notes` at `$72790`, `PSGFrequencies` at `$729CE`).

### F2 — AS enumeration directives. 28 counts, 28 lines. Shared, at identical counts.

`enum` (6) / `nextenum` (20) / `enumconf` (2), all in `sound/_smps2asm_inc.asm`. S2
reports 9/20/2 for the same shared file.

**This cause hides an undefined-symbol population.** `probe/p3.asm` shows sigil emitting
*nothing at all* for `dc.b zqA,zqB,…` after a failed `enum` — the names are simply never
bound, and the reference to them is deferred to link (see the silent half). AS says
`error #1010: symbol undefined` on the spot.

### F3 — `charset`, and the listing-control directives. 12 counts. Shared, and S2 needs it more.

`charset` 9 (`sonic.asm` 2616–2675); `page` 1 and `listing` 1 (`MacroSetup.asm` 7–8);
and `unknown directive or mnemonic \`purecode\`` at `sound/z80.asm(11)`, which is the
same `listing` directive reported through a different message. S2 has **82** `charset`
sites to S1's 9.

`charset` is a byte-changing feature, not cosmetics: it remaps the code page for
subsequent string literals. `probe/p5.asm` line 20, after `charset ' ', $FF`:

```
      20/      20 : FF                  	dc.b " "
```

It owns 504 bytes of the retail ROM (`$359E`–`$3795`, the level-select and sound-test
text).

### F4 — macro default parameter values. 54 counts, 2 lines. **S1-ONLY** (this heading said *Shared*; see the correction below).

`Macros.asm(12)`, `locVRAM: macro loc,controlport=(vdp_control_port).l`. When a caller
omits the parameter sigil substitutes nothing rather than the default. That produces
both the 18 `instruction needs an explicit size suffix` **and** the 36
`bad operand expression`. `probe/p5.asm` lines 8–12 reproduce both; AS assembles the
call cleanly as `move.l #$1234,(4).l`.

**The 36 are all reported at `sonic.asm(1)`** — line 1 of the root file, which has
nothing to do with them. The misattribution reproduces standalone (`p5.asm(1)` for a
call at `p5.asm(11)`), so it is a span bug in its own right, not a corpus artifact.

### F5 — `dc.ATTRIBUTE [count]value`. 18 counts, 1 line. **S1-only.**

`MacroSetup.asm(98)`, the body of the corpus's `dcb` macro. AS's `[n]value` repetition
in a `dc` operand list:

```
       6/       7 : FFFF FFFF           	dc.b [4]$FF
```

sigil: `unexpected character`. S2's 11 `unexpected character` rows are a different
construct (`zoneanimcount_{"\{zoneanimcur}"} =` at `s2.macros.asm(246)`), so nothing in
S2 exercises this.

### F6 — a label written on the same line as `if`. 13 counts, 2 lines, and a link failure too.

**The most valuable single row in this parcel, and the whole corpus contains exactly one
instance of it:** `sonic.asm:4121`

```
Map_Ring:   if Revision=0
```

sigil drops the label. `Map_Ring` is then never defined anywhere, and the consequences
split by expression shape:

* used non-affinely — `dc.l (frame<<24)|mappings` in
  `_inc/Special Stage Mappings & VRAM Pointers.asm(10)` (5 counts) and
  `dc.l map+(object<<24)` in `_incObj/DebugMode.asm(382)` (8 counts) — the **frontend**
  says `unresolved long expression`;
* used plainly, it is deferred to a link fixup and the **linker** says
  `unresolved symbol \`Map_Ring\` for fixup in section sec752 at offset …`, **16 times**,
  with no file and no line.

Proof it is one cause and not two: correcting *only* line 4121 (splitting the label onto
its own line) takes both frontend sites from 13 diagnostics to **zero**, with no other
change. `probe/p7.asm` is the 16-line repro — AS binds `LblOnIf` to `$0C` and
`LblOnRept` to `$0D`; sigil defines neither and fails at link.

### F7 — `abs()`. 8 counts, 1 line. Not seen in S2.

`Macros.asm(353)`, `rept 1+(abs(first-last)/abs(step))` in the `range` macro; 8 call
sites across three object files. `probe/p5.asm` line 3: AS emits 7 bytes, sigil says
`unresolved rept count`.

### F8 — `switch`/`case` on an integer expression. 8 counts, 8 lines. Shared.

`sound/_smps2asm_inc.asm(63)` and `(88)`, `switch SonicDriverVer` / `case 1`. sigil's
`switch` handles string expressions only. AS's listing shows the integer form resolving
(`=>FALSE` / `=>TRUE` per case). S2 reports the same 6+2 split.

### F9 — multi-character literals as immediates. 6 counts, 6 lines. **S1-only.**

`_incObj/82, 83 SBZ Eggman Cutscene and Crumbling Floor.asm`, `move.w #"SW",…`. AS:
`303C 5357`. sigil: `bad immediate expression`. S2 has zero.

### F10 — a function call in a Z80 operand. 4 counts, 4 lines. **S1-only in practice.**

`sound/z80.asm` 51, 55, 188, 197 — `ld de,zmake68kPtr(SegaPCM)`,
`ld b,pcmLoopCounter(16000)`. Under `CPU Z80`, `name(expr)` is being read as a symbol
followed by a memory-indirect operand, so the parser reports `trailing tokens in
operand`. S2's Z80 driver never reaches this because it declares `cpu z80undoc` and
stays on the 68000 path; **S1's really does switch processors**, which is why only S1
can find this.

### F11 — `!org` restarting the counter inside `save`. 1 count, 1 line. **S1-only.**

`sound/z80.asm(9)`, `!org 0`, reached after the root has already `org`'d far into the
ROM. AS restarts the address counter at 0 for the phased Z80 block and later
`restore`s. sigil: `org target precedes the current phase base`. `probe/p9.asm` is a
15-line repro; AS emits `3e 01 32 05 00 00 aa 00 00 05` and resolves `zVar` to `$0005`.

**Sum: F1 166 + F2 28 + F3 12 + F4 54 + F5 18 + F6 13 + F7 8 + F8 8 + F9 6 + F10 4 +
F11 1 = 318, with no remainder.** Eleven causes; F3 and F4 each span more than one
message string, which is why a decomposition by message text (13 rows) and one by cause
(11 rows) do not line up.

## How far past the frontend we get — and the ROM

### Without stubs: nowhere. And that is the ranking answer.

`assemble_root_located` returns `Failure`, which carries **no partial module**;
`main.rs` renders and exits 1, and `sigil_link::link` is never called. There is
therefore **no such thing as a class that "merely produces diagnostics while assembly
proceeds"** — the distinction the brief asked me to draw does not exist in this
codebase. All 318 block a ROM equally. **A single unfixed diagnostic blocks the ROM as
completely as all 318 do**, which is exactly why counting them ranks nothing.

### With stubs: a 524,288-byte ROM, exit 0, zero diagnostics.

Thirteen source-side edits (`.s1probe/2026-09-04/s1-stub.patch.gz`, 1,003 diff lines
over 10 files; regenerated by `stub.py` + `stub_range.py`) stand in for the eleven
causes. Seven of the thirteen are **byte-neutral** — they express the same data another
way, so the bytes they produce are the bytes AS produces:

| stub | byte effect |
|---|---|
| `enum`/`nextenum`/`enumconf` mechanically expanded to `=` assignments | neutral |
| `switch`/`case` → `if`/`elseif` on the same expression | neutral |
| `locVRAM` default arg → explicit `if "controlport"<>""` | neutral |
| `dcb`'s `[count]value` → `rept count` / `dc.ATTRIBUTE value` | neutral |
| each `range` call site → the literal `dc.b` list AS emits | neutral |
| `#"SW"` → `#$5357` (computed, not copied) | neutral |
| `Map_Ring:` split onto its own line | neutral |
| `charset` lines commented out | **changes text bytes** |
| FM/PSG `dc.w MakeXFrequency(op)` → `dc.w 0` | **changes 2 tables** |
| `!org 0` in `sound/z80.asm` commented out | **changes every Z80 label** |
| four Z80 function-call operands → `0` | **changes 4 instructions** |
| `page` / `listing purecode` / `zonewarning` body removed | neutral |

Result:

```
$ sigil sonic.asm -o s1stub.bin ; echo $?
0
$ ls -l s1stub.bin
524288
```

**Zero diagnostics. Exit 0. Exactly the retail ROM's size.**

### The divergence map — 7 regions, 7,285 bytes, 98.6% identical

`diffmap.py` against the bit-perfect reference. The instrument was proven non-vacuous
first: reference-vs-itself reports 0 differing bytes, reference-vs-one-flipped-byte
reports exactly 1 at `$040000`.

| region | bytes | cause | mine or sigil's |
|---|---|---|---|
| `$00359E–$003795` | 504 | level-select / sound-test text | **F3 `charset`** |
| `$01E272–$01E3FF` | 398 | inter-section gap: `FF` vs `00` | **L2, real** |
| `$06AF24–$06AFFF` | 220 | inter-section gap: `FF` vs `00` | **L2, real** |
| `$071CB7–$071CB9` | 3 | a 68k reference to a Z80 label | my `!org` stub |
| `$072790–$07284F` | 192 | `FM_Notes` | **F1 floats** |
| `$0729CE–$072A57` | 138 | `PSGFrequencies` | **F1 floats** |
| `$072E7C–$0745DA` | 5983 | the DAC driver blob | **L3, real** (+ my Z80 stubs) |

Everything else — all code, all art, all level layouts, all mappings, all object data,
**and the ROM header including the checksum word at `$18E` and the end-of-ROM long at
`$1A4`** — is byte-identical.

## The three causes that live past the frontend, which no diagnostic count can see

### L1 — undefined symbols are a LINK diagnostic, with no file and no line.

`probe/p4.asm`: `dc.b zqNeverDefinedAnywhere` draws `error #1010: symbol undefined` from
AS at the source line, and from sigil:

```
error: unresolved symbol `zqNeverDefinedAnywhere` for fixup in section sec0 at offset 1
```

**No `file(line)`.** So the entire undefined-symbol population of the corpus is
invisible to the 318 and stays invisible until the frontend goes clean. In this parcel
it turned out to be a population of one root cause (F6, 16 fixups), but *nothing in the
frontend measurement could have told us that*, and the shape of the message means the
first person to hit a real one gets no location.

### L2 — the inter-section gap fill byte. 618 bytes over 2 regions.

`p2bin -p=FF`; `main.rs` calls `sigil_link::flatten(&linked, 0x00)`. The gaps at
`$1E272` and `$6AF24` are `FF FF FF …` in the cartridge and `00 00 00 …` in sigil's
image. This is a one-argument fix and it is a hard ROM-identity blocker; no frontend
run can ever surface it.

### L3 — p2bin's `-z` Kosinski compression is an emit stage sigil does not have.

`-z=0,kosinski,Size_of_DAC_driver_guess,after` compresses the Z80 DAC driver **in the
output image**, after assembly. The build prints `Uncompressed driver size: 1BC6h
bytes.` and the source reserves `$1760` for the compressed result
(`!org (DACDriver+Size_of_DAC_driver_guess)`). At `$72E7C` the cartridge holds a
Kosinski bitstream; sigil holds the raw Z80 driver, whose bytes are plainly visible
inside the reference's framing:

```
ref   $72E7C: e1ff f3ff 31fc 1fdd 2100 40af 32fd ff8f
sigil $72E7C: f3f3 f331 fc1f dd21 0040 af32 fd1f 32ff
```

**5,983 bytes of the ROM — 82% of the total remaining divergence — cannot be produced by
any assembler-side work at all.** This was invisible to every measurement the project
had, and it is the single most expensive thing on the list.

`fix_header`, by contrast, turns out to be free: the header bytes match without it,
because S1's source carries the correct checksum and end-of-ROM values as literals. That
holds only while the ROM stays 512 KB with this content.

## The ranked path to a ROM

Everything blocks. So the ranking is by **cost per byte of ROM unlocked**, and by
whether a class can be measured at all before the ones ahead of it land.

**Tier 1 — the two that own ROM bytes and are cheap.**

1. **F6, label on an `if` line** — one construct, 13 frontend diagnostics *and* 16 link
   failures, and it is the only thing standing between the current tree and reaching
   link on real (non-stubbed) `dc.l` expressions. Smallest fix on the list; largest
   ratio.
2. **L2, the gap fill byte** — one argument. 618 ROM bytes. Do it with F6 so the next
   image comparison is clean.

**Tier 2 — the frontend bulk, in decreasing counts-per-line.**

3. **F1, floating point + `INT()`** — 166 of 318 on two lines, and 330 ROM bytes.
   Needs a real float path through `function` bodies and `dc` operands; it is the
   largest genuine implementation job in the frontend, but it retires 52% of the count.
4. **F4, macro default parameter values** — 54 counts on two lines, and it carries the
   `sonic.asm(1)` span bug with it. **S1-only; the 2,624 figure this line carried is not
   an F4 population** (correction below). LANDED at master `7f6ad19d`.
5. **F2, `enum`/`nextenum`/`enumconf`** — 28 counts; shared with S2 at identical counts;
   unblocks a symbol population that is currently *unmeasurable* (L1).
6. **F3, `charset`** — 11 counts and 504 ROM bytes here, **82 sites in S2**. The best
   cross-corpus value on the list.
7. **F5, `[count]value`** — 18 counts on one line. S1-only, self-contained.
8. **F8 `switch`/`case` on integers** (8), **F7 `abs()`** (8), **F9 multi-char
   immediates** (6) — small, independent, each a day or less.

**Tier 3 — the Z80 sub-assembly, 6 counts and one ROM region.**

9. **F10 + F11** — function calls in Z80 operands, and `!org` restarting the counter
   inside `save`. Six diagnostics; but F11 governs every label in the DAC driver, so
   nothing in that 5,983-byte region can be checked until it lands.

**Tier 4 — the one that is not an assembler problem.**

10. **L3, Kosinski compression at emit.** 5,983 bytes. Either sigil's link grows a
    p2bin-compatible `-z`, or the S1 build keeps p2bin for that step. **This is a
    design decision, not a bug, and it should be made before anyone promises a
    byte-identical S1 ROM.**

## The silent half — what was searched, not only what was found

1. **Every one of the 318 lines was classified**; 0 fail to match `file(line): error:`.
2. **The 318 was reproduced in a second, independently created worktree** — the one that
   also produced the bit-perfect reference ROM — and the sorted sets are identical.
3. **The diff instrument was proven non-vacuous** before any conclusion was drawn from
   it (0 on identical inputs, exactly 1 on one planted byte).
4. **Ten differential probes** against `asl` with S1's own flags, each quoting the
   listing's real byte columns.

Found:

* **F6 is a silent-wrong-answer class in the strict sense.** A label on an `if` line is
  not diagnosed, not bound, and produces no bytes; the failure surfaces far away, at
  link, without a location. Only *one* line in the whole corpus triggers it — a corpus
  with a second one would be that much harder to diagnose.
* **F2 the same, one step removed.** A failed `enum` leaves its names unbound, and every
  reference to them is accepted by the frontend without comment.
* **L2 and L3 are ROM-identity divergences that no diagnostic will ever report.** Both
  were found only by building an image and comparing it.
* Probed and found **correct**, i.e. matching AS byte-for-byte: forward references
  combined non-affinely (`(2<<24)|Fwd`) both within a section and across an `org`
  (`probe/p6.asm`, `p8.asm` — both byte-identical to AS); a bare unknown head under
  `CPU Z80` (`probe/p10.asm` — AS binds it as a label at the PC and so does sigil, `AA
  02 01` from both). The 2026-09-03 note's row (a) is closed.

## What S1 and S2 share, at today's tip

S2 at `e45ebf3` reports **9,432** (down from the 13,109 recorded on 2026-09-03), and it
is a *different* problem: 2,309 of them are the nameless `+`/`-` local labels S1 does
not use anywhere, and 3,384 are `unresolved symbol in operand`. **S1 is by a wide margin
the closer corpus to a ROM, and it is the one with a retail-cartridge byte gate.**

Shared causes worth doing once: **F1** (S2: 23 `int()` rows), **F2** (identical counts,
same shared file), **F3** (S2: 82 `charset` sites), **F8** (S2: 6+2).
S1-only: **F4** (corrected — see below), **F5**, **F9**, **F10**, **F11**, and F7 in practice.

## Booked, not done

* Two of the seven divergent regions are contaminated by my own Z80 stubs
  (`$071CB7`, and part of `$072E7C–$0745DA`). Once F11 lands, re-measure that region
  against the reference **before** costing L3 — the compression may not be the only
  thing wrong there.
* The link diagnostic has no `file(line)`. Worth fixing before the frontend goes clean,
  because after that every remaining failure will wear that shape.
* `sonic.asm(1)` span misattribution (F4), still open, still reproducible standalone.
* Nothing here was run under an emulator. **The stub ROM is not a playable ROM and was
  never executed** — it exists only to be compared byte-wise. Any claim that sigil's S1
  output *runs* is TAGGED for foreground follow-up and is not made by this parcel.

## Where this brief was wrong

1. **"The frontend is where the remaining work is (it may all be at link)" — both
   halves are wrong, and the truth is a third thing.** It is neither. The frontend holds
   eleven causes worth 318 diagnostics; the link stage holds one real class (L1, and its
   whole population here was a single frontend cause); and the *largest* remaining item
   by ROM bytes (L3, Kosinski compression at emit) is at neither — it is a post-link
   image transform that AS itself does not perform. A frontend/link dichotomy misses it
   entirely.
2. **"The 318 may be a long tail" — no. It is the opposite of a long tail.** Eleven
   causes, and 250 of 318 sit on seven source lines. Two lines carry 166.
3. **"Classes that BLOCK emitting a ROM at all, versus classes that merely produce
   diagnostics while assembly proceeds" — the second category is empty.** `Failure`
   carries no partial module, so any single diagnostic is as fatal as all of them. The
   distinction the brief called "the whole value of the ranking" cannot be drawn, and
   the ranking has to be built on cost-per-ROM-byte instead. That is what is above.
4. **"A complaint count measures what the frontend refused" — true, and it understated
   the case.** The count also cannot see that the frontend is *nearly done*. 318
   diagnostics reads like a wall; the same tree, stubbed, is 98.6% of a retail
   cartridge. The count was not just blind to danger, it was blind to progress.
5. **The 318 figure held exactly**, twice, in two independently created worktrees — the
   predecessor's landing record was accurate on the number. The *zero unresolvable
   symbol names* half needs one qualification, and it is the brief's own warning coming
   true: that is a property of the **frontend** only. Stub the frontend past its
   refusals and the link stage reports **16** unresolved `Map_Ring` fixups. Zero
   unresolvable names at the frontend did not mean zero unresolvable names.

## CORRECTION 2026-09-04 — F4 is Sonic-1-only, and the 2,624 was never an F4 number

Raised by the agent that implemented F4, which refused this note's framing on measurement
rather than adopting it, and re-derived here by the overseer over a **different enumeration
parameter** (a regex over macro declaration lines, against the agent's parse-based sweep)
before being accepted.

**F4 is S1-only.** Macro declarations carrying a default parameter: **1** in `s1disasm`
(`Macros.asm:11`), **0** in `s2disasm` at `e45ebf33`, **0** in aeon's tracked `.asm`/`.inc`.
The `Shared` label above was an inference, never a measurement, and it is withdrawn.

**The 2,624 stands as arithmetic and falls as attribution — do not carry it forward with a
concession attached.** It is the size of S2's `bad operand expression` class, and that class
is AS's anonymous relative labels (`beq.s +`, `beq.s ++`) over ~2,602 distinct sites. It was
never an F4 population, so the right statement is not *"the number was right and only the story
was wrong"* — it is that **this note attributed a real count to the wrong construct**, and any
sizing that read F4 as the largest cross-corpus item on the list was reading a number that
belongs to a different row. Independent of the attribution: the S2 stderr stream is
byte-for-byte identical across the F4 fix, which is what a zero-defaults corpus predicts.

**Consequence for anyone sizing off this note.** F4 was ranked 4th on the ranked path largely
on that cross-corpus figure. On the corrected reading its whole value was the 54 S1 rows it
did in fact retire. **F3 (`charset`, 82 S2 sites) is now the best genuine cross-corpus item on
the list**, and the ranking above has NOT been rewritten to match — read the ranking through
this correction rather than at face value.

**A third figure in this note is stale rather than wrong.** The 318 baseline was true when
measured and was 305 by the time F4 ran, master having retired F6's 13 rows at `b5c4f83f`
nineteen commits later. The overseer quoted 318 into a dispatch brief from this note instead of
deriving it. **Derive the baseline at use time; this note dates it, it does not maintain it.**
