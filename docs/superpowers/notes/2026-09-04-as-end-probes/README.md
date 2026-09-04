# asl probes: `END`, word-sized immediates, and `cnop`

The differential oracle for these three booked rows. Every probe here is run
against the **committed** `asl` 1.42 Beta Bld 212 at
`s2disasm/build_tools/Linux-x86_64/asl`, whose answer does not pass through
sigil — which is the whole reason the answers are worth anything.

```
./run.sh end1.asm        # listing + p2bin image for one probe
```

`run.sh` assembles with `-L -q -i <this dir>` and then runs `p2bin`, printing
the listing and an `xxd` of the image. The golden snippet generator
(`gen_snippet_vectors`) uses `-cpu 68000 -q -L -U`; `-U` (case-sensitive
symbols) does not change any answer below, verified on `end5.asm` — asl
recognises the uppercase `END` under `-U` exactly as under the default.

## `END` ends the assembly (row `AS-END-DIRECTIVE`)

asl stops reading source at the `END` line and says nothing about what it
skipped: 0 errors, 0 warnings. Reading past it emits extra bytes silently.

| probe | shape | asl image |
|---|---|---|
| `end1.asm` | `dc.b $11,$22` / `end` / `dc.b $33,$44` | `11 22` |
| `end2.asm` + `endinc.asm` | `end` in a FALSE `if` arm does not stop; `end` in an INCLUDED file stops the WHOLE unit, so the parent's post-`include` line is dropped too | `11 22 33` |
| `end3.asm` | `end` inside a MACRO EXPANSION stops the unit | `11 55` |
| `end4.asm` | `end` inside a `rept` body stops on the first iteration | `11 77` |
| `end5.asm` | `END <label>` stops exactly like the bare form | `11` |
| `incroot.asm` + `part.asm` | the committed test fixture's exact text (`tests/vectors/as_end_include/`) | `11 22 33` |

## A word-sized immediate against a `$FFFF….` RAM label (row `AS-WORD-IMM-RAM-LABEL`)

`wrange.asm` walks the boundary. asl accepts `-32768..=65535` — the signed
floor and the unsigned ceiling — and reports `range overflow` on either side:

```
       4/       0 : 303C FFFF           	move.w	#65535,d0
       5/       4 : 303C 8000           	move.w	#-32768,d0
> > > wrange.asm(6):10: error: range overflow	move.w	#-32769,d0
> > > wrange.asm(7):10: error: range overflow	move.w	#65536,d0
> > > wrange.asm(8):10: error: range overflow	move.w	#-65536,d0
> > > wrange.asm(9):10: error: range overflow	move.w	#$FFFFF700,d0
```

sigil reports the same four and accepts the same two. `wimm.asm` is the
narrower question the row's own wording asked — a `.w` immediate naming a
`$FFFF….` label directly — and asl **refuses** it (`range overflow` on all of
`#v_ram`, `#v_ctl`, `dc.w v_ram`, `dc.w v_ctl`), so the row's premise that "asl
accepts and sigil refuses" is refuted.

`wimm2.asm` is the shape `s2.asm` actually writes at :400/:413 — arithmetic over
a DIFFERENCE of two such labels, which is small and in range — plus
`lea (RamLabel).w,a6` and `dc.l RAM_Start&$FFFFFF`:

```
       7/       0 : 3C3C 1EFF           	move.w	#((CrossResetRAM_End-CrossResetRAM)/4)-1,d6
       8/       4 : 3C3C 1FFF           	move.w	#((CrossResetRAM-RAM_Start)/4)-1,d6
       9/       8 : 303C FFFF           	move.w	#(RAM_Start>>16)&$FFFF,d0
      10/       C : 4DF8 8000           	lea	(CrossResetRAM).w,a6
      11/      10 : 00FF 0000           	dc.l	RAM_Start&$FFFFFF
```

## `cnop` off the program counter (row `AS-CNOP-ORG-CONST`)

`cnop.asm` carries `s2.macrosetup.asm`'s own `cnop`/`align` macro pair verbatim
in shape — `org (*-1+(alignment)-((*-1+(-(offset)))#(alignment)))`, the
`macrosetup:40` line the row named. asl image:

```
00000000: 1100 0000 2200 0000 0000 3300 0000 0000  ....".....3.....
00000010: 44                                       D
```

Both of these are pinned as asl-minted golden blocks in
`crates/sigil-frontend-as/tests/snippets_golden.txt`
(`as_word_immediate_against_ffff_ram_labels`,
`as_cnop_org_off_the_program_counter`), so the shapes carry a byte assertion
rather than the absence of a complaint.
