# ASL-WARN-PARITY-ALIGN: sigil warns where asl says "address is not properly aligned"

Parcel `ASL-WARN-PARITY-ALIGN`, branch `parcel/as-warn-odd-address-align`, 2026-09-17.

## The premise, and what was true of it

The switch-matrix sweep (`docs/superpowers/notes/2026-09-16-switch-matrix-sweep.md`, section
"Diagnostic parity", commit `6348b831`) found asl raising
`s2.asm(30438): warning #180: address is not properly aligned` on `move.w (1).w,d0` inside
`if gameRevision=0`, with sigil silent and the bytes agreeing. That held. What the premise did not
say, and the probes below measured, is that `#180` is **two rules under one code**, and that the
second one (an instruction starting at an odd address) has nothing to do with operand size.

## Instrument

`s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`, checked by hand
and again by `../asl-reference/asl_ref.sh` inside every run. Flags `-xx -n -q -A -L -U`. Every probe
file assembles with **exit 0 and `ASL_DIAG=complete`** (no error anywhere in the file, pass loop
finished), so no row carries another line's contamination. A line asl refuses was moved out of its
probe rather than left in it: `move.w ($FFFF0001).w,d0` is `error #1340: short addressing not
allowed` and is not a `#180` case at all.

- `run.sh [probe...]` runs asl alone and prints each warning with its source line.
- `compare.py --sigil <bin>` runs both toolchains over every probe and compares the SET of warning
  locations and the images. `compare.py --mint` rewrites `asl_expected.tsv` from asl alone; the
  Rust test reads that file, so the test's expectations are the instrument's output.
- `sigil_refuses.tsv` lists the probes sigil does not assemble for an unrelated reason, with the
  text the refusal must carry. Both tools assert it in both directions.

## Probe table

"asl" is the set of lines `#180` fired on (count per line in brackets where asl repeated itself).
"sigil" is the result of `compare.py` at the implementing commit: **AGREE** means same location set
and byte-identical image.

| probe | what it varies | asl fires on | sigil |
|---|---|---|---|
| p01_size | `move.b/.w/.l (1).w` and `(2).w` | `.w (1)`, `.l (1)`; never `.b`, never even | AGREE |
| p02_dest | odd address as destination, and both operands odd | every odd operand, source or destination; `(1).w,(3).w` fires twice | AGREE (x2 on that line) |
| p03_abs_forms | `(1).l`, `($FF0001).l`, `(1)`, bare `1`, bare `$FF0001`, `(-1).w`, `($FFFFFFFF).w`, `.b` bare, even `.l` | every odd word spelling; not `.b`, not even | AGREE |
| p04_symbols | `equ`, `=`, expression, forward-referenced `equ` | odd symbols and `even2+1` (x2, two passes), forward `fwd` (x1) | AGREE (x1 each) |
| p05_noaccess | `lea`, `pea`, `jmp`, `jsr` at `(1).w`/`(1).l` | `jmp`, `jsr` only | AGREE |
| p06_pcrel | `move.w oddlbl(pc)`, `lea/jmp/jsr oddlbl(pc)`, indexed PC, `(oddlbl).w/.l` | only the absolute `(oddlbl).w` and `(oddlbl).l` | AGREE |
| p07_disp | `1(a0)`, `1(a0,d0.w)`, `(1,a0)`, `-1(a1)`, `3(sp)` | none | AGREE |
| p08_imm | `#1` in every position, `dc.w 1`, `dc.l 1` | none | AGREE |
| p09_insns | tst/clr/cmpi/addq/btst/bset/movem/movea/cmp/add/not/lsl/scc/divu/mulu/move sr,ccr/tst.b/btst d0/clr.b/tas | every word/long form incl. `movem`, `move <ea>,sr`, `move sr,<ea>`, `move <ea>,ccr`; not btst/bset/scc/tst.b/clr.b/tas; `cmpi.w` x2 | AGREE |
| p10_cpu_68008 | CPU | fires | AGREE |
| p10_cpu_68010 | CPU | fires | sigil refuses `cpu 68010` |
| p10_cpu_68020/68030/68040/68332 | CPU | **none** | sigil refuses the CPU |
| p11_supmode_on | `supmode on` | fires, no effect | AGREE |
| p12_padding_on | `padding on` | fires, no effect on the operand rule | AGREE |
| p13a_oddpc_insn | `dc.b 0` then `move.w (2).w,d0` (even address, odd PC) | fires: **instruction at an odd PC** | AGREE |
| p13b_oddpc_dcw / p13c_oddpc_dcl | `dc.w` / `dc.l` at odd PC, padding off | none | AGREE |
| p13d_oddpc_padding_on | odd PC under `padding on` | none (the pad byte makes it even) | AGREE |
| p14_macro | odd address through a macro argument | at the call site, `p14_macro.asm(7) rd(1)` | AGREE, same spelling |
| p15_include | odd address in an included file | `p15_inc.inc(2)` | AGREE |
| p16_if_arms | false `if` arm, true arm, `else` arm | taken arms only | AGREE |
| p17_byte_forms | `cmpi.b`, `addq.b`, `move.b` both sides, `add.b`, `subi.b`, `move.b #` | none | AGREE |
| p18_imm_to_mem | `move.w #`, `addi/subi/andi/ori/eori/cmpi` to `(1).w`, `adda.w`, `cmpa.l` | all; addi/subi/andi/ori/cmpi x2, eori x1 | AGREE (x1 each) |
| p19_branches | `bra.w`, `bsr.w`, `dbf` to an odd label; `jmp`/`jsr oddlbl` | `jmp oddlbl`, `jsr oddlbl` only | AGREE |
| p20_phase | label in a `phase $FF0001` block, `.l` and masked `.w` | both | AGREE |
| p21_multiplicity | one constant line in a two-pass file | x2 | AGREE (x1) |
| p22_expr_forms | `base+1`, `base+2`, `(3*5)`, `15/2`, `$FF0000\|1` | the odd results | AGREE |
| p23a..p23j | instruction at an odd PC: `nop`, byte insn, 68020, odd PC plus odd address, `org 1`, two in a row, label only, after `ds.b 1`, 68008, `phase 1` | every instruction start at an odd PC, **on every CPU including 68020**; label-only silent; p23d fires twice (PC and operand) | AGREE; p23c sigil refuses `cpu 68020` |
| p24_unsuffixed | `asl (1).w`, `move (1).w,ccr`, `jmp 1`, `jsr 1`, `lea 1,a0`, `pea 1`, `movem.w 1`, `movem.l d0,(1)`, `jmp (1)`, `tas 1` | all but `lea`, `pea`, `tas` | AGREE |
| p25_mnemonics | neg/sub/suba/and/or/eor/muls/divs/subq/roxr/bclr/bchg/move.l to a0/movea.l/neg.b/not.l/clr.l/addi.b/`move.w (1),(2)`/`move.b (1),(2)` | word/long forms; not bclr/bchg/neg.b/addi.b/move.b; `or.l d0,(1).w` x2 | AGREE |
| p26a_phase_phys_odd_vma_even | physical odd, `phase 2` | none: the **VMA** is tested | AGREE |
| p26b_phase_phys_even_vma_odd_data | `phase 1` then `dc.w` | none | AGREE |
| p27_pc_symbol | `(*+1).w`, `(*).w` at PC 0 | `*+1` | AGREE |
| p28_rept | odd address in a `rept 3` | x3, `p28_rept.asm(6) REPT n(1)` | AGREE, same spelling |
| p29_reg_indirect | `jmp (a0)`, `jmp 1(a0)`, `(a1)`, `(sp)+`, `-(sp)` | none | AGREE |
| p30_odd_label_long | `move.w/jsr/lea/move.b (OddData).l` | `move.w`, `jsr` | AGREE |
| s01_sigil_refuses_unsuffixed | unsuffixed `move/tst/clr/addq` to `(1).w` | all four | sigil refuses: needs a size suffix |
| s02_sigil_refuses_chk | `chk.w (1).w,d0` | fires | sigil refuses `chk` |
| s03_sigil_refuses_nbcd | `nbcd (1).w` | none (byte) | sigil refuses `nbcd` |

`compare.py` total at the implementing commit: **51 probes, 42 AGREE, 9 SIGIL-REFUSES, 0 DIFFER**.

## asl's rule, as measured

1. **Instruction start.** Any 68k instruction whose first byte is at an odd VMA (`$` after any
   `padding on` pad) fires, whatever its operands, on every CPU asl accepts. Data directives and bare
   labels never fire.
2. **Odd absolute operand.** An absolute address operand (`(a).w`, `(a).l`, `(a)`, bare `a`, symbol,
   expression, forward reference) whose value is odd fires, once per operand, on 68000/68008/68010
   only, when the instruction reads or writes a word or long through it (size from suffix, special
   register, or default) or is `jmp`/`jsr`. Never for `lea`/`pea`, byte access (including bit
   operations on memory, `Scc`, `tas`, `nbcd`), branches or `DBcc`, displacements, PC-relative
   operands, immediates.
3. **Multiplicity** is asl's pass loop: once per pass that evaluates the line, twice per pass for
   `addi/subi/andi/ori/cmpi` and `or.l` to memory. Not reproduced; sigil prints one per offending
   operand or instruction start, which is what the source determines.

## What was implemented

`crates/sigil-frontend-as/src/eval.rs`: `warn_odd_pc_instruction` (after `pad_word_align` in
`lower_m68k`) and `warn_odd_abs_operands` (on the parsed operands of the generic path, the
`jmp`/`jsr` path and `movem`'s memory operand), with the qualifying mnemonics in
`m68k_odd_address_access`. Warn tier, lint id **`[as.odd-address]`** (the `[area.name]` id the warn
tier tallies and the corpus gates key on; the `.emp` counterparts are `[layout.odd-field]` at warn
tier and `[layout.odd-item]`), message naming the address, the reason (a 68000 address error), and
asl's code and text. No byte changes: only a `Level::Warning` diagnostic is pushed. CPU gating is not
needed because this front end accepts only `68000` and `68008`, both of which fire.

## What is not decided, and why

**`AS-ODD-ADDRESS-RELOCATING-UNDECIDED`** (booked in `campaign-gap-ledger.md`). On the front end's
deferral pass, which every relocating assembly returns (every chained aeon build) and a pinned build
with a deferred cross-seam `jsr` reaches, section-label references are kept symbolic because sections
still move after assembly. There the parity of a label- or `$`-derived address and of the location
counter is provisional, so sigil does not warn about them. Uncovered probed cases, on that path only:
p13a, p23a/b/d/e/f/h/i/j (instruction start), p06 and p30 (odd label operand), p19's `jmp/jsr
oddlbl`, p20 (phase label), p27 (`*+1`). Constant operands (p01 to p05, p09, p18, p22, p24, p25) are
still decided there. On the ordinary (pinned, poison-free) path every probed case is decided, which is
the path `sigil <input.asm>` takes on the disassemblies.

## Consumers

- `crates/sigil-frontend-as/tests/as_odd_address.rs`: the probe set in both directions plus named
  fires, does-not-fire and relocating cases. Each shown red first on a mutation printed back with
  `git diff HEAD` and restored from HEAD.
- `scripts/switch_matrix_sweep.py`: `("s2disasm", "asl#180")` left `ACK_WARNING_GAP`; asl#180 now
  pairs with `[as.odd-address]` by location (`CODED_COUNTERPARTS`, `warning_parity_keys`), and
  `EXPECT_WARNING_PARITY` asserts the pairing is observed. Control C8 keeps its fixture (the real
  `s2.asm(30438)` asl line) and gains sigil's rendering of it plus four pairing failure cases.
- **The first `--cross` run with the warning went red, and the cause was the sweep, not the rule.**
  Run at sigil `0446c80b` (script md5 `e8dc48d3`, identical at `d1a163f7`): 808 legs launched and
  reported, `SWEEP FAILED` on one key, `("s2disasm", "sigil-only:[as.odd-address]")` on 1 leg. Per
  leg, with the sweep's own parsers over its logs: 34 legs name `s2.asm(30438)`; 33 agree location
  for location (17 both-built, the pairing the run reported); 32 are asl-only because sigil refused
  the leg's `-z saxman-optimised` before assembling, which parity already excludes; and the one
  both-built divergence is `s2disasm-CONTROL-divergent-source`, the end-to-end control that flips
  `gameRevision` 1 to 0 AFTER the asl reference build. asl built revision 1 (no warning), sigil built
  revision 0 and warned, correctly. That leg hands the toolchains different programs by design, so it
  is now excluded from parity (`diagnostic_parity`, rows marked `control`), with C8 row-level cases
  that go red if the exclusion or the pairing table is removed. Nothing was added to
  `ACK_WARNING_GAP`.
- **Re-run at `fc081ef0`** (`--cross`, sigil and script built and copied at that commit, 14:27Z to
  14:48Z): 808 legs launched and reported, `SWEEP PASSED`. 347 legs where both ran (349 before, less
  the two corpora's control legs); one gap key, the standing `shared` residual on 55 Sonic 2 legs
  (56 before, less the control); pairing `asl#180 = [as.odd-address]` on 17 legs at `s2.asm(30438)`.
- Warn-tier gates on aeon (`sigil-cli/tests/warn_tier_corpus.rs`): measured 0 firings on all seven
  shipped shapes of `.aeon-sigil-ref` at `ec640bcf`, so no baseline moves.
