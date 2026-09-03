# A processor spelling is accepted only when sigil encodes its instruction set

2026-09-03 · branch `parcel/as-cpu-variant-spellings` · sigil master base `de527346`

Third in the `cpu`-directive arc. The first refused an UNDECLARED processor; the
second folded the case of a declared one; this one rules on what a declared name
is allowed to mean.

## The rule

**A spelling earns an arm only when it names an instruction set sigil already
encodes.** Two shapes qualify:

- *The same instruction set, different packaging.* `68008` is a 68000 core on an
  8-bit data bus — identical instructions, so the same target. This was the
  existing precedent.
- *A strict superset whose extra instructions are refused BY NAME.* `z80undoc`
  is the Z80 with its undocumented instructions enabled. Accepting it widens
  *where the refusal is reported*, never what assembles — but only while the
  refusal half holds, which is why it was measured before the arm was written.

A spelling naming an instruction set sigil does not encode gets no arm. `68020`,
`z180`, `gbz80` add instructions; `6502`, `8051` are unrelated processors.
Aliasing any of them is how a source silently assembles as something it never
asked for. **Do not bulk-add variant spellings to make a corpus go green.**

## The two defects, and the one that mattered more

### The loud one: `cpu z80undoc` was refused

`s2.sounddriver.asm:250` declares `CPU Z80UNDOC`. `directive_cpu` rejected it,
so the one file in that corpus genuinely asking to change processor assembled as
a 68000 for its entire length.

**It is a MID-UNIT switch, not a unit declaration.** `s2.asm:90859` reads
`include "s2.sounddriver.asm"` between a `save` and a `restore`, so the driver is
part of the 68000 root's assembly unit and its `cpu` line is a switch inside it.
The corpus has exactly three `cpu` directives and only ONE of them is a
declaration:

| site | spelling | role |
|---|---|---|
| `s2.asm:55` | `CPU 68000` | the unit's declaration |
| `s2.asm:319` | `CPU Z80` | mid-unit switch, inside `save`/`restore` (the VDP-init Z80 startup blob) |
| `s2.sounddriver.asm:250` | `CPU Z80UNDOC` | mid-unit switch, inside the `save`/`restore` around the include |

### The silent one, and the reason this is a parcel rather than an arm

A processor name that begins with a digit reaches the lexer as `Tok::Int`. The
match **discarded the value** and read every numeric name as `68000`:

```rust
Some(Tok::Int(_)) => "68000".to_string(),
```

Measured on the shipped command, not inferred:

```
cpu 6502  / move.w #$1234,d0   ->  exit 0, wrote 303c1234
cpu 68020 / move.w #$1234,d0   ->  exit 0, wrote 303c1234
```

That is the same silent-wrong-target class the previous two parcels closed from
the undeclared and the mis-cased side, reached through a third door: a spelling
nobody checked. The `"68008"` string arm was dead code for the same reason —
`68008` reaches the match as an integer and was collapsed to `"68000"` before it
could be seen.

`68020` is the sharp case rather than `6502`: its instruction set is a superset
of the 68000's, so a source written for it assembles *almost* correctly right up
to the first 68020-only instruction, which is then reported as an unknown
mnemonic on a target the source never named.

## The soundness measurement that licenses `z80undoc`

The condition: **an undocumented instruction sigil does not encode must be
refused by name, never dropped and never mis-assembled.** If that failed, the
right answer was to keep refusing `z80undoc`, corpus count be damned.

It holds. Every probe below fails, exits 1 and writes no binary:

| form | what happens |
|---|---|
| `sll a` | `unknown directive or mnemonic` |
| `in a,(c)` | `unknown directive or mnemonic` (documented, but outside sigil's table) |
| `rlc (ix+1),b` | `unsupported form: CB shift/rotate expects one operand` |
| `ld a,ixh` / `ld a,iyl` / `add a,ixl` / `adc a,ixu` | `unresolved symbol \`ixh\` in operand` |
| `ld ixh,5` | `unsupported form` + `unresolved symbol` |

The last row is what the corpus actually uses: `s2.sounddriver.asm` reads and
writes the index-register halves twelve times (`ixl` 4, `ixu` 4, `iyl` 2,
`iyu` 2), which is what it declares `Z80UNDOC` for.

**The caveat, stated as a caveat.** That refusal is *contingent*: `ixl` is not a
register name to the operand parser, so it becomes a symbol, and the refusal is
"this symbol is undefined". A source that DEFINED a symbol called `ixl` would
have `ld a,ixl` assemble as `ld a,n` with that value. Nothing in `s2disasm`
defines one (checked), and no `cpu` spelling makes it more or less likely — but
the property would be unconditional if `is_reg_or_cond_word` knew the six
index-half names. Booked below rather than done: `parse_operands` has no CPU
context, so adding them there also changes the 68000 path, and that is byte-risk
to a shipping build for a hazard nobody has met.

## The mechanism: one table

`CPU_SPELLINGS` in `lib.rs` carries the four accepted spellings and the target
each selects. `cpu_for_spelling` resolves against it; `unsupported_cpu` lists it
back to the reader. The refusal therefore cannot advertise a spelling the
directive does not accept, and the integer's VALUE is carried to the table as
the spelling it is.

The refusal names the fault and prints the remedy, matching `CPU_UNDECLARED`'s
register:

```
unsupported processor `68020`: sigil's AS-compatibility front end does not encode
this instruction set, and will not assemble the source as a different processor
instead. Write one of `cpu 68000`, `cpu 68008`, `cpu z80`, `cpu z80undoc`. A
spelling is accepted only when it names an instruction set sigil encodes, so a
wider processor is refused here rather than aliased onto a narrower one and
silently mis-assembled.
```

## The corpus: 85,335 → 89,120

Same command both times, `sigil s2.asm` from `~/sonic_hacks/s2disasm` at
`e45ebf3`. Distinct `file(line)` sites 8,486 → 8,701. Both re-derived this pass.

Per file:

| file | before | after |
|---|---|---|
| `s2.macros.asm` | 66,136 | 66,138 |
| `s2.asm` | 11,877 | **11,691** |
| `mappings/MapMacros.asm` | 4,568 | 4,568 |
| `sound/_smps2asm_inc.asm` | 36 | **3,520** |
| `s2.macrosetup.asm` | 2,676 | 2,731 |
| `s2.sounddriver.asm` | 35 | **465** |
| `s2.constants.asm` | 7 | 7 |

The rise is one cause. The driver now assembles as a Z80 from line 250, so two
regions that had never been reached are reached:

- **`s2.sounddriver.asm` +430** — the driver's own Z80 body, reporting its real
  gaps for the first time. 366 `unresolved symbol` (`STRUCT` field references:
  `zTrack.PlaybackControl` 66, `zTrack.VoiceControl` 20, …), 14 unknown heads
  (`op` 7, `label` 4, `endm` 2, `purecode` 1), 30 `bad byte expression`, 17
  `int()`. Its first 28 diagnostics are unchanged: they sit *before* line 250,
  where the CPU is still legitimately 68000.
- **`sound/_smps2asm_inc.asm` +3,484** — 3,417 of them one unimplemented AS
  directive, `eval`, in the SMPS music macros, plus 67 `operand out of range`.

**The win is the fall, not the rise.** `s2.asm` dropped 186, and its distinct
unresolved symbols went 302 → 207: **95 symbols the game code references and
could not resolve are now defined**, because the driver that defines them
(`MusID_2PResult`, `MusID_Boss`, `MusID_Continue`, … the whole sound-ID block)
finally assembles. One refused `cpu` line was withholding a sound driver's
symbol table from the program that calls it.

`unsupported cpu` — 1 diagnostic before — is gone.

## Aeon: no bytes moved

`sigil-frontend-as` is in the shipping build path (`build.sh` routes
`engine/debug/debugger.asm` and both `game_root.asm` through it), so this parcel
could have moved ROM bytes. All four shapes rebuilt at aeon `4f5ad5a1` with a
binary whose own witness reads `02d58e0b334d (clean at capture)`:

| shape | CRC32 / size | expected |
|---|---|---|
| `s4.bin` | `14ee2440` / 719700 | same |
| `s4.debug.bin` | `142294b3` / 737683 | same |
| `demo.bin` | `0c456778` / 96474 | same |
| `demo.debug.bin` | `2e603d53` / 101339 | same |

The `Tok::Int` change is the one that could have. Aeon's roots write
`cpu 68000`, which now depends on the integer's value being 68000 rather than on
the arm that ignored it; the four identical shapes are what proves it.

## Gates

`crates/sigil-cli/tests/cpu_spellings.rs`, six tests, all driving the shipped
`sigil <file.asm>` process — the caller whose input is a foreign tree, and so the
only one that meets an unknown processor name.

Red-first, every mutation applied from a committed baseline and read back from
disk before the run:

| mutation | predicted | actual |
|---|---|---|
| A: restore `Tok::Int(_) => "68000"` | `a_numeric_spelling_sigil_does_not_encode_is_refused` | **1 red, as predicted** |
| B: `("z80undoc", Cpu::M68000)` in the table | the alias gate | **2 red — but NOT the alias gate.** See below |
| C: special-case `z80undoc` → M68000 in `directive_cpu`, table unchanged | the alias gate | **3 red, incl. the alias gate** |
| D: unknown head under `Cpu::Z80` becomes a silent no-op | the soundness gate | **1 red, as predicted** |
| E′: `unsupported_cpu` stops listing the table | the refusal gate | **1 red, as predicted** |

### Two things the mutations corrected, recorded because they were corrections

**Mutation B refuted the alias gate's stated reach.** It was documented as
catching a row pointed at the wrong target. It does not: `directive_cpu`
resolves through the same table the test derives its expectations from, so a
wrong row picks the body that agrees with it and the assertion moves with the
defect. **A table cannot audit itself.** What went red on B were the two gates
whose expectations come from the Z80 ISA rather than from `CPU_SPELLINGS` —
`the_undocumented_forms_z80undoc_adds_are_refused_and_emit_nothing`'s control
and `a_mid_unit_switch_to_z80undoc_assembles_both_halves`. Those are what pin
`z80undoc` to the Z80. The alias gate's real reach — the table and the lowering
DISAGREEING — was then proven by mutation C, which is the mutation that shape
actually needs.

**Mutation E ran against a looser gate and was re-run.** The first version read
the accepted spellings out of the whole stderr stream. A unit whose only `cpu`
line was refused has also declared nothing, so `CPU_UNDECLARED` is reported
alongside — and its text names `cpu 68000` and `cpu z80` itself. Two of the four
spellings were being found in a neighbouring diagnostic no matter what the
refusal said. The gate now reads the refusal's own line, and E′ is the re-run
that establishes the claim under the tightened method; it fails on the FIRST
spelling now instead of the second.

## Suite

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-as-fold cargo test
--release --workspace --no-fail-fast`, run in
`sigil/.claude/worktrees/agent-adac56d3cc7ead057` on
`parcel/as-cpu-variant-spellings` at `02d58e0b`: **4259 passed / 0 failed / 2
ignored**, exit 0, across 376 result lines. The log carries all six
`cpu_spellings` test names. The diff against master adds exactly six `#[test]`
and modifies no existing test file, so it reconciles as master + 6.

`cargo clippy --all-targets -- -D warnings`: **exit 0**, no warnings.

## Left open

- **The contingent index-half refusal.** `ld a,ixl` is refused as an unresolved
  SYMBOL, not as an unimplemented register. Correct today and misleading; a
  source defining a symbol named `ixl`/`ixu`/`iyl`/`iyu` would silently get
  `ld a,n`. The unconditional form is six names in
  `operands.rs::is_reg_or_cond_word`, but that function is shared with the
  68000 path and has no CPU context, so it is byte-risk to a shipping build for
  a hazard nobody has met. Wants the CPU threaded into `parse_operands` first.
- **The version witness can OVER-report dirt.** `sigil --version` read
  `dirty at capture — 1 modified` against a tree `git status --porcelain`
  reported clean: the capture is a build-time snapshot and cargo did not re-run
  `build.rs` when the working tree was restored. Its own caveat names the
  under-reporting direction only. Over-reporting is the worse direction for a
  banner — a warning that fires on a clean tree is one people learn to scroll
  past. Touching `crates/sigil-cli/build.rs` forces a fresh capture.
- **AS directives the driver's Z80 region now names:** `op`, `label`,
  `purecode`, and `endm` reaching dispatch from a nested block. Plus `eval`
  (3,417 sites in `_smps2asm_inc.asm`) and the `STRUCT` field references the
  driver leans on. All inventory, none in this parcel's scope.
- **Nothing here was run on the emulator.** No runtime confirmation was
  attempted or is implied; the byte identity of the four aeon shapes is the
  whole behavioural claim.
