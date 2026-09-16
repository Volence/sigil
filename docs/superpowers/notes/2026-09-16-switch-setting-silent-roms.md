# The switch-setting silent ROMs, closed: what was measured, what was chosen, and what is still open

2026-09-16, parcel `SWITCH-SETTING-SILENT-ROMS`, branch
`parcel/switch-setting-silent-roms`, base `53c707d9` (master, read from the tree).
The measurement this closes is `2026-09-16-as-corpus-census.md`, Q5, landed at
`3704df29`. Every tree below is a private copy under
`/home/volence/sonic_hacks/.scratch/switch-silent-roms/`; the shared `s1disasm`,
`s2disasm` and `skdisasm` checkouts were only read, through `git archive`.

## Headlines

1. **Both silent faults are closed, and the way they are closed is different for
   each, because sigil can FIX one and can only DECLINE the other.** The header
   fault is a derived field sigil was not deriving, so it derives it. The
   driver-size fault is a number the source baked in before the number existed,
   so sigil refuses to write a ROM built on it.
2. **The census names one header field and there are two.** `fix_header` rewrites
   the end-of-ROM address at `0x1A4` as well as the checksum at `0x18E`, and the
   second can go stale on its own. Measured with a positive control, not
   reasoned: `$100` bytes of `dcb.b` emitted after `sonic.asm`'s `EndOfRom:`
   label make the reference write `000800FF` where sigil wrote `0007FFFF`. Six
   bytes across two fields, all of it silent at exit 0.
3. **sigil CAN read the `<constant>` a `-z` instruction names, which the parcel's
   own brief assumed it could not.** An AS equate reaches the link as an `EquSym`
   on its section: `Size_of_DAC_driver_guess` folds to `Int(5984)` and
   `Size_of_Snd_driver_guess` to `Int(3940)`. No new front-end plumbing was
   needed and none was added.
4. **And the constant is EXACT in both corpora at their shipped settings**
   (`0x1760` stored against `$1760`; `0xF64` against `$F64`), which is what makes
   a refusal on overflow safe rather than a guess.
5. **The refusal is one-directional, and the asymmetry is measured.** `skdisasm`
   ships a stream SMALLER than its constant, so a symmetric check would fire on a
   corpus at its own shipped settings. That direction stays silent and is booked.
6. **Ten of the census's eleven flips now agree or refuse correctly.** The
   eleventh is fault 3, the `move.w (v_limitright2),d0` width-selection gap,
   which is a front-end row and was not touched.

## Provenance

| Instrument | Identity |
|---|---|
| sigil, base | built from `53c707d9` with `CARGO_TARGET_DIR` in scratch, md5 `a2d60855050816be3642f089fdf77cb0`, `--version` `53c707d9`. Every BEFORE figure is this binary. |
| sigil, fixed | built from the parcel tip, md5 `0ca33e3299f56fdfef588ce9cf2639f3`. Every AFTER figure is this binary. |
| corpora | `s1disasm f6ece657`, `s2disasm e45ebf33`, `skdisasm 2fcd861c`, each extracted by `git archive`. |
| references | `s1built.bin` CRC32 `afe05eee` / 524,288 B; `s2built.bin` CRC32 `7b905383` / 1,048,576 B; `skbuilt.bin` 2,097,152 B. Each from the corpus's own `build.lua` run unmodified in a copy. |

CRC32 throughout is IEEE/zlib, computed by `zlib.crc32` in CPython 3, rendered as
eight hex digits, and quoted with the byte size. Every image compare carries the
census's `compare.py` planted-byte control, and no figure below comes from a run
whose control did not pass.

## Fault 1: the header, measured before and after

The BEFORE run is the base binary against the stock toolchain's ROM from the same
tree, whole-image, no window.

| flip | before | after |
|---|---|---|
| s1 `CheatsEnabled = 1` | 2 B differ at `0x18E`: reference `BF37`, sigil `AFC7`, exit 0, **zero lines on stderr** | 0 B, CRC32 `ac362f3d` |
| s1 `Revision = 0` | 0 B (agreed already) | 0 B, `f9394e97` |
| s1 `AllOptimizations = 1` | 2 B at `0x18E` | 0 B, `95dd2ddc` |
| s1 `EnableSRAM = 1` | 0 B (agreed already) | 0 B, `649236ea` |
| s1, `$100` bytes after `EndOfRom:` | **6 B: 2 at `0x18E`, 4 at `0x1A4`** | 0 B, `120b1e4f` |
| s2 `gameRevision = 0` | 2 B at `0x18E` | 0 B, `24ab4c3a` |
| s2 `allOptimizations = 1` | 2 B at `0x18E` | 0 B, `6c8cbd9b` |
| s2 `useFullWaterTables = 1` | 2 B at `0x18E` | 0 B, `bbaffa38` |
| s2 `padToPowerOfTwo = 0` | 0 B (agreed already) | 0 B, `dd0ebd72` |
| **s1, shipped** | 0 B, `afe05eee` | **0 B, `afe05eee`** |
| **s2, shipped** | 0 B, `7b905383` | **0 B, `7b905383`** |

**Why the three that agreed before agree for a reason rather than by luck**, and
this is the mechanism predicting its own exceptions: they are exactly the flips
that change no byte at or after `0x200` and no byte of the image's length, so
neither derived field moves.

### The design call, and what was rejected

The test applied is aeon's: does the fix REMOVE the need to remember or ADD one.

* **A `--fix-header` option.** Rejected. An option nobody passes is the same
  silence one step further away, and the person who would have to remember to
  pass it is the same person who would have to remember to run a checker.
* **A checker that verifies the literal.** Rejected for the same reason, one
  layer out: an obligation that must be remembered is the same class of thing as
  a list that must be maintained, and it hides better.
* **Refusing when the literal is stale.** Rejected, and this one is refuted by a
  shipped use rather than by taste. The documented integration for sigil on this
  route is as a drop-in for asl plus p2bin inside a disassembly's own
  `build.lua`, whose `fix_header` runs AFTER it. In that integration a stale
  literal at sigil's exit is expected and correct, and a refusal would break the
  build for a ROM that was about to be made right.
* **Folding it, guarded on the content.** Chosen. `apply_sega_header` runs
  unconditionally on the route that writes an image and is guarded by
  `has_sega_header`, the console's own TMSS mark at `0x100`. It is idempotent, so
  a `build.lua` that still runs `fix_header` writes the same two fields again.

**The guard is not decoration.** `s2disasm`'s `build.lua` assembles a generated
`song.asm` into a music binary through the same assembler entry point, once per
song. Those are longer than `0x200` and carry no header, so an unguarded header
pass would silently overwrite four bytes of somebody's data: the fault this
parcel closes, pointed the other way.

**It is silent when it changes something**, like `fix_header`. The value is
self-referential, so a reader can do nothing with the news except see it on every
build of a tree whose author does not maintain a field sigil now maintains for
them.

## Fault 2: the driver-size immediate

BEFORE, s2 `fixBugs = 1`: 3 bytes differ, `0x18E` (`CAA9` against `D951`) and
`0xED051` (`88` against `64`), exit 0, one warning and it is about `shared`. The
image sums to `CA85` and the reference to `CAA9`, and `CAA9 - CA85 = 0x24 =
0xF88 - 0xF64`: the two faults are one arithmetic showing up twice.

AFTER: exit 1, no image written, and

```text
s2.sounddriver.asm(248):6: error: `-z=0,saxman-bugged,Size_of_Snd_driver_guess,after`:
the blob [0x0, 0x134F) is 0xF88 bytes compressed as saxman-bugged, and
`Size_of_Snd_driver_guess`, the size the source declares for it, is $F64; every byte
the source computed from that name is short by 0x24, and p2bin's own message for this
is to raise it, so set `Size_of_Snd_driver_guess` to $F88
```

### What the existing reservation check could not see

`flatten_placing` already refused a stream larger than the physical GAP after it,
which is the most p2bin can measure. Measured gaps: s1 `0x1760` (equal to the
constant), s2 shipped `0x1018`, s2 `fixBugs = 1` `0x8018`. For the `after` form
the gap is not the reservation the source wrote: Sonic 2 reserves `$F64` with an
`!org` and the filler after `Snd_Driver_End` makes the gap far larger, so a
`$F88` stream fits the gap and overflows the declaration. The new check is the
same refusal moved from the gap to the constant.

### The design call

* **Patch the immediate.** Rejected as unsound. `build.lua` finds the site
  through asl's share file; sigil writes none, and hardcoding `movewZ80CompSize`
  would put a Sonic 2 label name in a general assembler. There is also no
  relocation record for a comptime integer folded into an arbitrary expression,
  so there is no general set of bytes to patch.
* **Re-assemble with the constant rebound to the measured size.** Considered and
  rejected. It would converge (the guess is already exact on both corpora) but it
  changes the reservation as well as the immediate, so the resulting ROM is no
  longer the one the stock toolchain builds, and it is a fixpoint loop in a route
  whose whole contract is "what p2bin does".
* **Warn.** Rejected. It leaves a ROM that is wrong at runtime on disk at exit 0,
  and the thing it is wrong about is the byte count a decompressor reads.
* **Refuse, one direction.** Chosen. See the asymmetry below.

### The asymmetry, and why it is not a symmetric check

`skdisasm`'s `Size_of_Snd_driver_guess` is `$E00` and the Kosinski stream in the
ROM `buildSK.lua` itself writes is `$DFC` (read off `skbuilt.bin` in the
reservation between `Z80_SoundDriver` at `0xF6960` and `Z80_SoundDriverData` at
`0xF7760`; its end is located by Kosinski's own `00 F0 00` terminator at
`+0xDF9`, not by guessing from trailing zeros, and four pad bytes follow it).
**Refusing or even warning on a stream smaller than its constant would fire on a
corpus at its shipped settings**, which is the always-red shape: a check that
fires on correct code trains people to weaken the check that works. So the
under-run direction stays silent, and it is booked in the gap ledger as
`Z80-DRIVER-SIZE-UNDER-RUN-STAYS-SILENT` with `-c` named as its kill.

## The ripple: aeon cannot move, by reachability

Both new behaviours live on `run_asm`, the `sigil <root>.asm -o` route. Enumerated
rather than sampled: `apply_sega_header` has exactly one non-test call site
(`crates/sigil-cli/src/main.rs`, in `run_asm`) and `flatten_placing` exactly one
(the same function). Aeon's build enters through `sigil build --aeon`, which is
`run_build`, and reaches neither. The only edit to code aeon executes is lifting
`apply_header_checksum`'s sum loop into `header_checksum` verbatim, same
arithmetic and same order, with `emit_rom` unchanged.

Verified anyway, four shapes, `AEON_DIR=/home/volence/sonic_hacks/.aeon-sigil-ref`
at `ec640bcf` (the tree the golden `provenance.toml` tip names): see the parcel's
report for the run.

## What this parcel did NOT do

* **Fault 3**, s1 `FixBugs = 1` and `move.w (v_limitright2),d0`, is a front-end
  width-selection gap and is booked separately. Untouched here.
* **`-c`**, the share file, is out of scope and is the kill for the residual.
* **The switch matrix.** Eleven flips is a sample of 557 + 698 conditional sites.
  `SWITCH-MATRIX-SWEEP` is the row that turns the executed domain into the
  accepted one; the three rows added here are its first expected results.
* **Runtime.** Whether any ROM plays is not a byte question. No emulator was
  touched. TAGGED for the controller.

## Reproducing

```text
CARGO_TARGET_DIR=<on disk> cargo build --release --bin sigil
git -C <corpus> archive --format=tar HEAD | tar -x -C <scratch>/trees/<corpus>-pristine
cp -a <scratch>/trees/<corpus>-pristine <scratch>/trees/<corpus>-luaref
(cd <scratch>/trees/<corpus>-luaref && lua build.lua)   # the reference ROM
```

**⚠ THE TWO SCRIPT LINES THAT SAT HERE NAMED PATHS THAT DO NOT EXIST, and the block was
never runnable as written.** Corrected 2026-09-16 on the `SWITCH-MATRIX-SWEEP` agent's
catch, verified here: `scripts/mk_gen_trees.sh` is not under `scripts/` (the only tracked
copy is `docs/superpowers/notes/2026-09-16-as-corpus-census/scripts/mk_gen_trees.sh`, the
census's own directory), and `scripts/flip.sh` **exists nowhere in the tree** under any
path; the census's nearest script is `switchflip.sh`, which is a different thing. Check
with `git ls-files | grep -E 'mk_gen_trees|flip'`.

**Use `scripts/switch_matrix_sweep.py` instead**, landed at `f06dfa5a`, which supersedes
both: it derives the option set from the corpus rather than taking a flip on the command
line, and it reproduces all nine of this note's flip CRC32s. The rest of the block above
stands.

*(The general rule this broke is already banked as CITE THE ARTEFACT THAT CAN BREAK: a
path written into a record is a claim about the tree, and this one was never checked
against it. A citation costs one `ls` to verify and rots in silence otherwise.)*

and in the suite, which needs no scratch at all:

```text
cargo test --release -p sigil-cli --test as_switch_setting_roms
cargo test --release -p sigil-cli --test as_second_address_space
cargo test --release -p sigil-link --lib
```
