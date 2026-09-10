# S1-LAYOUT-OVERLAP: asl never resolves this, and neither should sigil's front end

2026-09-10, investigation parcel, branch `investigate/s1-phased-overlap`, worktree
`/home/volence/sonic_hacks/.wt-s1-phased-overlap` off `master` `4ebfb950`.
**INVESTIGATE-AND-PROPOSE. No behaviour change is proposed for landing in this
note; the two code edits on this branch are MEASUREMENT SCAFFOLD, marked as such
at both sites, and must not merge.**

The headline is that the row is misnamed. Nothing here is phased. Sonic 1 has two
`save`/`restore` Z80 blocks and they are different mechanisms: the one at
`sonic.asm(324)` IS phased and **sigil already gets it exactly right**; the one at
`sound/z80.asm(9)` is not phased at all and is a **second address space** whose
ROM placement asl does not perform, does not check, and does not record. It is
performed by `p2bin`, from a flag in `build.lua`.

Second headline, and it changes what the row is worth: **this is the last blocker
on the AS route.** With the overlap silenced and nothing else changed, sigil
assembles the whole Sonic 1 corpus — exit 0, zero diagnostics, a 524,288-byte ROM.
Silence the overlap *and* leave the driver's bytes out (the two-probe form below,
a deliberately wrong ROM) and 98.65% of that image is byte-identical to the
reference toolchain's, with every remaining difference accounted for by name.

## Provenance

| Instrument | Identity |
|---|---|
| sigil (clean) | built from `4ebfb950` (= `origin/master`), md5 `42cb2877b1e1ad3a1526979479b22bab`. Reproduces the diagnostic verbatim. |
| sigil (scaffolded) | the same tree + this branch's two probes, md5 `8b4d6923b3bd747ef4e5f8137dc5762b`. Every number below marked *probe* comes from this one. |
| build | `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.s1phase-2026-09-09/target`. The shared `target/` was never used, so no md5-pinned freeze was relinked. |
| corpus | `/home/volence/sonic_hacks/.s1phase-2026-09-09/corpus`, a `cp -a` of `/home/volence/sonic_hacks/s1disasm` restored to `f6ece657c1cf253404312137dfcb8ec15fa42318` (`git status --porcelain` = 0 paths in the copy). |
| corpus, not written | `s1disasm` itself was read only. Verified after the run: same 4 dirty paths as before, same HEAD, no new `*.p`/`*.lst`/`out.bin`. |
| oracle | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`** — verified before use, and distinguished from `s2disasm`'s `0dee1f98e6480a4783d27ffd8b90896f`, which is a different binary and is not the reference. |
| oracle run | `asl -xx -n -q -A -L -U -E -i . sonic.asm` → exit 0, empty stderr, no `sonic.log`, `sonic.p` 529,364 B md5 `4f54eb0a983bee02aefe8c3348695a67`. A clean run, so its listing is a source of values. |
| oracle ROM | `p2bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after sonic.p out.bin` → 524,288 B, md5 `09dadb5071eb35050067a32462e39c5f`. This is the reference image every byte figure below is measured against. |
| NOT an oracle | `s1disasm/s1built.bin` is UNTRACKED, 551,288 B, dated 2026-08-20. It is a stale artifact of somebody's modified build, not a reference, and 323,324 of its bytes differ from a clean build of `f6ece657`. It was not used. |

## Q1. What does asl actually do here? It punts, and the punt is the answer

### The two blocks are not the same construct

| site | directives | asl's object record | sigil's section |
|---|---|---|---|
| `sonic.asm` 322-353 | `save` / `CPU Z80` / **`phase 0`** / … / `dephase` / `restore` | cpu **Z80**, address **`0x2CA`**, len `0x26` | `sec0#1`, cpu Z80, **lma `0x2CA`**, `vma_base Some(0)` |
| `sound/z80.asm` 8-237 | `save` / **`!org 0`** / `CPU Z80` / … / `restore` / `!org (DACDriver+$1760)` | cpu **Z80**, address **`0x0`**, len `0x1BC6` | `sec0#2`, cpu Z80, **lma `0x0`**, `vma_base Some(0)` |

`SetupValues_Z80` is at `$2CA` and `SetupValues_Z80_End` at `$2F0` in the listing,
`$26` apart, while `zStartupCodeEndLoc` inside reads `$26` — bytes inline in the
68000 stream, labels 0-based. That is the classic load-vs-run split, and **sigil
reproduces it exactly**, down to the same record boundary asl chose.

`!org 0` is a different thing. `disp` is 0 at that point (no `phase` is open), so
it is not a displacement; it re-bases the physical counter. The corpus says so in
its own words at `MacroSetup.asm:15`, where a wrapper macro named `org` exists
precisely to avoid the builtin — "make it work in Z80 code **without creating a
new segment**" — and `!` is the escape that reaches the builtin anyway.

### The object file contains BOTH address spaces, flat, undistinguished

`sonic.p` decomposes into 442 records carrying a CPU tag (1 = 68000, 81 = Z80) and
an absolute address. Grouped into contiguous runs, the whole file is 7 non-empty
regions, and sigil's section table is **structurally identical to it**:

```
asl record run              sigil section
cpu 1  [0x0,     0x2CA)     sec0        M68000 [0x0,     0x2CA)
cpu 81 [0x2CA,   0x2F0)     sec0#1      Z80    [0x2CA,   0x2F0)
cpu 1  [0x2F0,   0x1E272)   sec752      M68000 [0x2F0,   0x1E272)
cpu 1  [0x1E400, 0x6AF24)   sec123904   M68000 [0x1E400, 0x6AF24)
cpu 1  [0x6B000, 0x72E7C)   sec438272   M68000 [0x6B000, 0x72E7C)
cpu 81 [0x0,     0x1BC6)    sec0#2      Z80    [0x0,     0x1BC6)
cpu 1  [0x745DC, 0x80000)   sec476636   M68000 [0x745DC, 0x80000)
```

The Z80 driver's record sits *positionally* where `DACDriver` is (between the run
ending at `0x72E7C` and the run starting at `0x745DC`) but is *addressed* at 0.
**asl records no relationship between the two.** It exits 0 and warns about
nothing. There is no overlap check in asl at all.

### The proof that asl punts: run p2bin without the flag

```
$ p2bin -p=FF sonic.p plain.bin        # no -z
$ xxd -l 16 plain.bin
00000000: f3f3 f331 fc1f dd21 0040 af32 fd1f 32ff
$ xxd -s 0x72E7C -l 16 plain.bin
00072e7c: ffff ffff ffff ffff ffff ffff ffff ffff
```

The Z80 driver lands **on the vector table** and the ROM hole at `DACDriver` is
pad. That is sigil's diagnostic, produced by the reference toolchain, silently.

With the flag the same object file produces a correct ROM:

```
$ p2bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after sonic.p out.bin
$ xxd -l 8 out.bin        ; 00fffe00 00000206   <- vector table intact
$ xxd -s 0x72E7C -l 8 out.bin ; e1fff3ff 31fc1fdd  <- KOSINSKI stream, not Z80 code
```

`e1ff` is a Kosinski descriptor word; the raw driver begins `f3 f3 f3 31 fc 1f`.
So the bytes stored at `DACDriver` are **not the assembled bytes**: a compression
pass runs at the address-space boundary. `Size_of_DAC_driver_guess = $1760`
(`_Constants.asm:7`) is the *reserved* size; the *uncompressed* size is `$1BC6`,
larger, which is why the stored form must be compressed and why the reservation
is a budget the compressed form has to fit.

**The load-address-versus-run-address split for this driver is not performed by
the assembler.** It is performed by a post-processor, from a policy written in
`build.lua`:

```lua
common.build_rom_and_handle_failure("sonic", "s1built", "",
  "-p=FF -z=0," .. compression .. ",Size_of_DAC_driver_guess,after", false, ...)
```

`-z=<address>,<compression>,<constant>,<type>` = "the consecutive Z80 records
starting at address 0 are one blob; compress it thus; reserve `<constant>` bytes
for it; insert it **after** the preceding segment." Every part of the answer —
which records, what transform, how big a hole, where it goes — is in that string
and nowhere in the source.

## Q2. What does sigil model? The split, correctly. The address space, not at all

sigil **has** the LMA/VMA split, in the IR and on both front ends:

- `sigil_ir::Section { vma_base: Option<u32>, lma: u32 }`, `vma_origin() =
  vma_base.unwrap_or(lma)`; `link` defines every label at `vma_origin + offset`
  (`sigil-link/src/lib.rs:57-73`).
- The AS front end reaches it. `state.rs:64` carries `disp`; `directive_phase`
  sets it; `open_section_if_needed` (`eval.rs:5711`) opens the section with
  `vma_base = phys_base + disp` and `lma = phys_base`. `sec0#1` above is that
  code working on the real corpus.
- The `.emp` front end reaches it through the `vma:` section attribute
  (`sigil-frontend-emp/tests/lower_sections.rs:80-135`).

So the answer to "why does the AS front end not reach it" is: **it does.** The row
is not blocked on a phase mechanism.

What sigil does not have is an **address space**. `Section` has `cpu`, but `cpu`
is not a space — `sec0#1` and `sec0#2` are both `Cpu::Z80` and only one of them
belongs outside the ROM image. Every placed section is scanned against every
other in one flat `[lma, lma+size)` universe (`relax.rs::overlap_diag`), and
`flatten` writes them all into one buffer. `directive_org`'s own doc comment
(`eval.rs:6080-6092`) already anticipated `sound/z80.asm`'s `save`/`!org 0` by
name and deliberately chose to close the section, re-base, and let the collision
be a link-time matter — which is the right call given the model, and the model is
what is missing.

The nearest thing to the concept is `RegionKind::Z80Bank` in `sigil-ir/src/map.rs:9`.
It is **declared and never consumed**: three hits in the whole tree, all of them
the declaration, the label string, and the TOML parse. A name with no behaviour.

## Q3. The true collision population: 3 pairs, 4 sections, not 1

Measured, not inferred. Scaffold **1 of 2** on this branch: `SIGIL_OVERLAP_DUMP=1`
in `overlap_diag` dumps every placed section before the early-returning scan. The
run is the corpus + the previous parcel's B/E stubs (`scripts/lib/s1_stub_bc.py`,
9 + 12 lines, its own warning attached: stub B is not byte-neutral).

11 sections placed, 4 empty and correctly skipped, 7 scanned:

```
COLLIDING PAIRS: 3
  sec0   [0x0,0x2CA)     M68000  X  sec0#2 [0x0,0x1BC6) Z80   intersection 0x2CA  bytes
  sec0#1 [0x2CA,0x2F0)   Z80     X  sec0#2 [0x0,0x1BC6) Z80   intersection 0x26   bytes
  sec752 [0x2F0,0x1E272) M68000  X  sec0#2 [0x0,0x1BC6) Z80   intersection 0x18D6 bytes
DISTINCT SECTIONS INVOLVED: 4 -> sec0, sec0#1, sec0#2, sec752
```

One offender (`sec0#2`), three victims, `0x1BC6` bytes of clobber — which is
exactly what `plain.bin` shows the reference toolchain doing when its flag is
withheld. The shipped scan reports the first pair and returns, so `1` was a floor
and `3` is the population **for this corpus at this revision**; it is not a
constant, it is however many placed sections the driver's extent happens to cross.

## Q4. Designs

Four things are missing, and they are separable:

1. a way to say *this section is not in the ROM address space*;
2. a policy mapping that space's contents into the ROM at a derived offset;
3. a transform (Kosinski) at that boundary, plus a budget check against `$1760`;
4. the pad byte. `p2bin -p=FF`; sigil's AS route hardcodes `flatten(&linked, 0x00)`
   (`main.rs:364`). Worth 618 bytes on this corpus and unrelated to the row.

### Design A — mirror p2bin: an address-space field plus a CLI flag

`Section` gains a space discriminator, set by the AS front end when `org` re-bases
under a CPU other than the module's host. `overlap_diag` scans per space. A new
link stage consumes a flag that mirrors p2bin's grammar:

```
sigil sonic.asm -o s1.bin --pad FF --blob 0,kosinski,Size_of_DAC_driver_guess,after
```

- Costs: a CLI surface that is a foreign tool's flag string; a link stage that
  re-implements p2bin's `after`/`before` rule; `Size_of_DAC_driver_guess` must be
  read back from the symbol table at link time (it is an `equ`, and sigil already
  carries `equ_syms` through `resolve_layout`).
- Forecloses: little, but it enshrines p2bin's weakest property — the blob's ROM
  offset is "wherever the previous record happened to end", which is stated
  nowhere in the source and therefore cannot be checked against intent. If a
  future edit moves the preceding data, the blob moves silently.
- Buys: byte-exact reproduction of every AS-toolchain Sonic disassembly with zero
  source edits, which is the AS front end's entire purpose. S1, S2 and S3K all use
  the same `-z` shape with different compressors.

### Design B — sub-assembly as a comptime value (`.emp`, the aeon model)

Aeon already solves this outside the language: `emit_sound_blob` is a separate
sigil invocation producing a byte blob that the 68000 build places as data
(`sigil-harness/src/seam1.rs::native_sound_blob`). Design B makes that a value:

```
let driver = assemble("sound/z80.emp", cpu: z80, org: $0000)

section dac_driver at DACDriver {
    reserve $1760 {
        bytes kosinski(driver)
    }
}
```

`kosinski()` already exists (`eval/classic_compress.rs::eval_kosinski`, over the
vendored `clownlzss`), so item 3 is free. `reserve N { … }` makes the budget a
language construct rather than a flag, and `driver.size` feeds the corpus's own
`$ > z80_stack` guard.

- Costs: a comptime `assemble(...)` builtin — a re-entrant nested front-end
  invocation returning data plus a symbol table, with caching. Real work.
- Forecloses: nothing; purely additive.
- Buys: the whole story becomes explicit and testable with no corpus. It is the
  `.emp` thesis applied to exactly the case that motivates it.
- **Does not unblock Sonic 1**, which is written in AS.

### Design C — address space in the IR, policy in the map file

C is A's mechanism with B's declaration site. sigil already loads a map
(`--map`, `sigil-link/src/map_load.rs`) that already owns regions, kinds, sizes,
budgets and the gap-fill byte, and already has an unused `z80_bank` kind:

```toml
[[region]]
name       = "z80_driver"
kind       = "z80_bank"
vma_base   = 0
size       = 0x2000            # the Z80 address space, not the ROM
place_in   = "rom"
place_at   = "DACDriver"       # a SYMBOL, not a number
budget     = "Size_of_DAC_driver_guess"
transform  = "kosinski"
```

- Costs: map schema growth, and symbol-valued map fields impose an ordering
  constraint (the map's placement must resolve after the symbol table, which
  `resolve_layout` → `link` already sequences). Sonic 1 now needs a map file.
- Forecloses: the "drop-in for asl, no config" story — but that is already gone,
  because the pad byte (item 4) is `-p=FF` on the reference command line and
  sigil's AS route has no way to say it. A Sonic 1 build needs *some* out-of-band
  policy no matter which design wins.
- Buys: `place_at` is a **symbol**, so the fact p2bin cannot state — "the blob
  belongs at `DACDriver`" — becomes checkable, and a future edit that moves the
  preceding data gets caught instead of silently relocating the driver. Items
  1-4 all land in one artifact. Both front ends can use it.

### Recommended: C, in two stages, with A available as sugar over it

**Stage 1, small and needing no ruling on surface.** Give `Section` an
address-space discriminator, scan overlaps **per space**, and turn a
non-host-space section with no declared placement into its own diagnostic. Sonic 1
then fails with a message that names the real problem at the real line —
`sound/z80.asm(9): error: section 'sec0#2' is a Z80-space section at org 0 with no
ROM placement` — instead of an overlap reported at `sonic.asm(81)`, which is a
`dc.l v_systemstack&$FFFFFF` with nothing to do with it. This is strictly a
sharpening: it removes a wrong-location diagnostic and adds a right-located one,
and it changes no bytes.

**Stage 2, needs the ruling.** Where the placement/transform/budget is declared:
map file (C), CLI flag (A), or `.emp` value (B). I would take C. B should be built
anyway — it is the right long-term `.emp` surface — but it cannot unblock a corpus
written in AS, and this row exists to unblock that corpus.

**Do not take the option of relaxing the overlap check.** asl has no such check;
this is a place where sigil is legitimately stronger than its oracle, and the
check is what makes the next section correct.

## The half-fix question, answered with both half-fixes on disk

Scaffold **2 of 2**: `SIGIL_PROBE_IGNORE_Z80_LMA0=1` (skip the pair) and
`SIGIL_PROBE_DROP_Z80_LMA0=1` (also drop the section's bytes). **Control: with both
unset the rebuilt binary still prints the overlap error**, so neither probe is on
by default. Both produce a 524,288-byte ROM at exit 0 with **zero diagnostics**.

| | half-fix (a) `IGNORE` | half-fix (b) `DROP` |
|---|---|---|
| exit / diagnostics | 0 / none | 0 / none |
| ROM size | 524,288 — correct | 524,288 — correct |
| bytes wrong vs oracle | 13,954 (2.66%) | **7,078 (1.35%)** |
| vector table `[0,0x2CA)` | destroyed | **byte-identical** |
| `[0x72E7C,0x745DC)` | pad | all-zero — driver absent |
| md5 | `76d866087f9c892bce4dd691ccf7ad0f` | `087ea44f8f24b5b7fd8bc09d09dd23f3` |

(b) is the maximally reassuring wrong answer and it is not hypothetical, it is a
file. Its 7,078 wrong bytes decompose as: **504** the charset stub's own doing
(`[0x359E,0x3796)`, void under stub B by construction), **618** the pad-byte
difference (item 4 above, two inter-section gaps sigil fills `0x00` where p2bin
fills `0xFF`), **75** an unrelated sigil defect found by this run (below), and
**5,881** the actual defect — a hole where the compressed driver belongs. Every
other byte of the ROM matches the reference toolchain exactly.

A ROM like that boots. The vectors are right, the code is right, the level data is
right; the DAC driver is simply not there.

### What distinguishes success from (b), and can we get it today

**Yes, two instruments, both available now.**

1. **Byte compare against the oracle image**, `out.bin` md5
   `09dadb5071eb35050067a32462e39c5f`, produced today from `f6ece657` by the
   verified `asl` + `p2bin`. Both probes fail it at named offsets. The compare must
   be **windowed** until B lands — stub B voids `[0x359E,0x3796)` and the pad byte
   voids `[0x1E272,0x1E400)` and `[0x6AF24,0x6B000)` — and the window must be
   *asserted* (its extent printed and checked), never assumed, or the exclusion
   silently grows to cover the defect.
2. **A property of the ROM alone, needing no reference**: take the bytes at
   `DACDriver`, decompress them with sigil's own vendored `clownlzss`, and require
   the result to equal the assembled Z80 section byte-for-byte, and the compressed
   length to be `<= Size_of_DAC_driver_guess`. This is the better gate: it cannot
   pass on an absent driver, an uncompressed one, a truncated one, or one placed
   at the wrong offset, and it does not depend on a reference image that a future
   corpus revision will invalidate. Recommend it as the acceptance gate for
   whichever design lands.

**Not available today, and not attempted: any runtime confirmation.** Whether the
driver actually plays is an emulator question and this parcel touched no emulator
tool. TAGGED for the owner.

The two instruments differ in what they can catch. (1) catches the wrong Kosinski
variant (authentic vs optimised — `build.lua` selects authentic and both
decompress correctly, so only a byte compare sees it). (2) catches a correct-size
blob at the wrong offset, which a windowed compare would catch only if the window
happened to include both offsets. Run both.

## A separate defect this run uncovered, silent and byte-wrong

75 of the drop-probe's wrong bytes are nothing to do with this row. In four
places sigil emits the wrong bytes with **no diagnostic**:

```
[0x8BF8,0x8C14)  28 B   sigil 21 21 21 21 …   oracle 22 22 23 23 24 24 25 25
[0xB7B1,0xB7BF)  14 B   sigil 21 21 21 21 …   oracle 22 23 24 25 26 27 28 29
[0xB821,0xB83F)  30 B   sigil 21 21 21 21 …   oracle 22 23 24 25 26 27 28 29
[0x1201B,0x1201E)  3 B  sigil 26 26 26        oracle 28 2a 2c
```

All four are the `range` macro (`Macros.asm:346`), whose body is a `set` following
an inner `rept … endr` inside an outer `rept`. Minimal repro, both assemblers, no
stub, no corpus:

```asm
	cpu 68000
	padding off
	org 0
	set .val, $21
	rept 4
		rept 2
			dc.b .val
		endr
	set .val, .val+(+1)
	endr
	end
```

```
asl:    21 21 22 22 23 23 24 24
sigil:  21 21 21 21 21 21 21 21
```

Two controls narrow it: moving the `set` **before** the inner `rept` is correct
(`22 22 23 23 24 24 25 25` both), and replacing the inner `rept` with an `if`
block is correct (`21 22 23 24` both). The byte count is 4×2, so the outer body
appears to be captured only up to the **inner** `endr` and the trailing `set`
falls outside the loop — a nesting-depth bug in `rept` body capture, not a `set`
bug. Files: `.s1phase-2026-09-09/probe/{nested,n2,n3}.asm`.

**This needs its own row.** It is invisible to the census (no diagnostic) and was
invisible to every previous parcel (the run never reached `flatten`). It is the
class the campaign cares most about: a wrong answer that looks like a right one.

## Corrections to the brief and to the standing note

1. **"The Z80 sub-assembly's phased block"** — the driver's block is not phased.
   `disp` is 0 at `sound/z80.asm(9)`; `!org` re-bases the physical counter. The
   phased block is the *other* one, `sonic.asm(324)`, and sigil handles it
   correctly. Naming the driver "phased" points a fix at machinery that works.
2. **"AS supports a phased block: `save` / `!org <addr>` / `restore`"** conflates
   three unrelated directives. `save`/`restore` stack assembler *state* (cpu,
   padding, listing) and say nothing about addresses — sigil's own `state.rs:48-50`
   states this correctly. `phase`/`dephase` is the displacement pair. `org` is a
   third thing. The corpus uses `save`/`restore` around both blocks precisely
   because it needs `CPU Z80` restored, not because of any address behaviour.
3. **"Code inside is assembled as though it sits at one address while being
   emitted somewhere else in the image."** True of `phase`; **false of the
   driver**. asl emits the driver's bytes at address 0 in the object file and
   records no relationship to the ROM at all. The split is performed by `p2bin`
   from a `build.lua` flag. Withhold the flag and asl's own output puts the driver
   on the vector table, silently, exit 0.
4. **"How many sections actually collide is UNMEASURED"** — measured here: 3 pairs
   over 4 sections, one offender. The `1` was a floor, as the note said.
5. **The note's "cannot be sized until B and E land, because until then it is
   unreachable"** is now overtaken: with B and E stubbed and the overlap silenced,
   **nothing else stands behind it**. `resolve_layout`, `link`,
   `check_image_bounds` and `flatten` all pass, and the resulting ROM matches the
   reference over 98.65% of its bytes. This row is the last AS-route blocker, and
   what remains after it is a 618-byte pad-byte policy question and the `rept`
   defect above.
6. **`!org 0` is load-bearing** — the note's finding, not re-derived and not
   contradicted. The refuted probe was not repeated.

## Reproducing

```
CARGO_TARGET_DIR=<on disk, NOT the shared target/> cargo build --release --bin sigil
cp -a /home/volence/sonic_hacks/s1disasm  <work>/corpus     # never build IN s1disasm
cd <work>/corpus && git checkout -- . && git clean -fd      # the copy only
./build_tools/Linux-x86_64/asl -xx -n -q -A -L -U -E -i . sonic.asm
./build_tools/Linux-x86_64/p2bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after sonic.p out.bin
./build_tools/Linux-x86_64/p2bin -p=FF sonic.p plain.bin    # the punt, visible
cp -a <work>/corpus <work>/stub && python3 scripts/lib/s1_stub_bc.py <work>/stub
cd <work>/stub && SIGIL_OVERLAP_DUMP=1 sigil sonic.asm            # population
              SIGIL_PROBE_IGNORE_Z80_LMA0=1 sigil sonic.asm -o a.bin
              SIGIL_PROBE_IGNORE_Z80_LMA0=1 SIGIL_PROBE_DROP_Z80_LMA0=1 sigil sonic.asm -o b.bin
```

The scaffold's control is that `sigil sonic.asm` with no variable set still prints
the overlap error; check it before believing either probe's silence.
