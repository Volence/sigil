# AS-BACKWARD-SEEK-SPLIT: a section closed behind its own seek puts the next section's bytes where its labels are not

Queue row AS-BACKWARD-SEEK-SPLIT. Measurement parcel, branch
`measure/as-backward-seek-split`, base master `c6651e45`. **Nothing under
`crates/` changed.** The hypothesis came from `2026-09-11-s1-driver-space-stage1.md`
(lines 113-117 and "Left open" item 3), which said it was unmeasured beyond a
reading of the code.

**Verdict: reproduced, and wider than stated.** Whenever a section in the image
is closed with its write cursor behind its extent (an `org` seek pointed back
into it and nothing returned the cursor to the end), sigil binds the next
section's labels where asl does, at `base + cursor`, and places its bytes at
`base + extent`. Every later `Chained` section carries the same drift until the
next `Pinned` one. asl overwrites the stale tail; sigil keeps it and grows the
image by `extent - cursor`. Exit 0, no diagnostic. **Exposure: none.** No
corpus program and no aeon input takes the path, so a fix moves no shipped byte.

## Provenance

| | |
|---|---|
| sigil measured | this worktree at `c6651e45`, `cargo build --release --bin sigil` into the worktree's own `target/`; `sigil` md5 `1dbdaa6da91ae35b497c53881b459a1c`, `--version` `sigil 0.1.0 (c6651e45)` |
| sigil instrumented (census only) | `git archive c6651e45` into an untracked scratch directory, `instrument.py` over the copy, built into `target/instr/`. Two builds of the same source: md5 `4dba3fd2451d2daf220c088f0072f2df` (produced `census/CENSUS.txt`) and, from `census-prepare.sh`'s fresh copy, `afda270a2dd240af64557733d19f36c5` (produced identical counts). The digests differ because the binary embeds build-time provenance read from the ENCLOSING checkout: `strings` on the second finds its own source directory, a build timestamp and this worktree's HEAD, so its `--version` reads `d4f315b9-dirty` although it was compiled from the `c6651e45` archive. The first was overwritten by the second (same target directory) and cannot be compared byte for byte. The instrument adds three warnings and nothing else: for EACH build, over all 15 probes, images and exit statuses equal the measured binary's (`diff` of the `RAW.tsv` hex and exit columns, 18 lines, exit 0) |
| asl | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098`, only through `asl-reference/asl_ref.sh`'s `asl_run`, flags `-xx -n -q -A -L -U -i .` (as `2026-09-12-as-macro-label-leak.md` line 13 states them). Bytes from the `p2bin` beside it, md5 `4f2fff99c3347bafb93b12d5be1db754`, and only from runs reported `ASL_EXIT=0`, `ASL_DIAG=complete` |
| corpora | s1disasm `f6ece65`, s2disasm `e45ebf3`, skdisasm `2fcd861`. Searched with `git -C <repo> grep` on tracked source; the census ran over `git archive HEAD` copies prepared by `scripts/corpus-prepare.sh` (8 / 74 / 100 generated files, the counts `2026-09-06-as-org-backwards-fix.md` recorded). The checkouts were only read |
| aeon | read-only through git objects at `origin/master` `a38ce7c9`; the working tree was never opened |
| emulator | none; nothing here needs runtime confirmation |

Files, all in `2026-09-12-as-backward-seek-split/`: `gen.py` writes `probes/`,
`run.sh` runs both assemblers into `results/` (`RAW.tsv`, one transcript set per
shape), `classify.py` prints `results/TABLE.md` (the table below).
`instrument.py`, `census-prepare.sh` and `census.sh` are the exposure census;
`census/` holds its output. `census-prepare.sh` was re-run from scratch into a
fresh directory and its census `diff`ed against `census/CENSUS.txt`. Every count
is identical. The one differing line is the instrumented binary's md5 (see
Provenance).

## How the front end closes a section

`close_section` (`eval.rs` 6538) is the only place that sets `in_section = false`.
Its five callers are every closer there is:

| closer | site |
|---|---|
| `restore` that changes the CPU | `eval.rs` 6305 |
| `cpu`, any name, including the CPU already selected | 6776 |
| `phase` | 6835 |
| `dephase` | 6855 |
| an `org` that leaves the section | 6993 (then `pin_next_section`) |

`builder.finish` closes the last section at the end of the program, where no
section follows. `directive_org` (6990) is the only producer of an in-section
seek.

What happens after a close with the cursor behind the extent:

1. `close_section` records the seek as the re-base and advances
   `phys_base += current_offset()`, so the counter is at `base + cursor`.
2. The next `open_section_if_needed` (6515) opens the section with
   `vma_base = phys_base + disp` **baked** (`Some`) and `lma = phys_base`. Labels
   bind at that address.
3. The builder stamps it `Chained` (`placement_for_next`, `builder.rs` 208). Only
   the leaving-`org` arm pins, and `assign_address_spaces` pins only a section in
   a second address space (`eval.rs` 1159-1165).
4. `place_pass` (`sigil-link` `relax.rs` 244-301) lands a `Chained` section at its
   group cursor, which is the previous section's base plus
   `max(reserved_span, final_size)`. That is `base + extent`. The move is recorded
   as an `LmaMove`, but the labels were bound from the baked VMA and stay where
   they were.

## Per shape

One shape per probe. Each probe's head table (`$00`) holds `dc.l L_next` and
`dc.l L_after`, so both assemblers report the label addresses in their own
images. The first label after the closer heads `dc.w $1234` / `dc.l *`, and a
label one plain close further on heads `dc.w $5678`. "bytes @" is where that
marker actually sits in the image. The seek section is four words at `$08`
(cursor `$C`, extent `$10` after the seek and one overwrite) unless the shape
says otherwise.

| shape | asl: L_next / bytes @ | asl: L_after / bytes @ | sigil: L_next / bytes @ | sigil: L_after / bytes @ | verdict |
|---|---|---|---|---|---|
| c01 `cpu 68000` (CPU unchanged) | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c02 `phase $8000` | $8000 / $C | $12 / $12 | $8000 / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c03 `dephase` closing a phased section with a seek | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c04 cpu-changing `restore` closing a phased Z80 section with a seek | $A / $A | $10 / $10 | $A / $C | $10 / $12 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 20 bytes |
| c05 `org $40`, forward, leaves the section | $40 / $40 | $46 / $46 | $40 / $40 | $46 / $46 | MATCH (images identical, 72 bytes) |
| c06 `org $10`, backward below the base, leaves | $10 / $10 | $16 / $16 | $10 / $10 | $16 / $16 | MATCH (images identical, 40 bytes) |
| c07 seek back, then `org` to exactly the extent, then `cpu` | $10 / $10 | $16 / $16 | $10 / $10 | $16 / $16 | MATCH (images identical, 24 bytes) |
| c08 two seeks, the second forward but short of the extent | $C / $C | $12 / $12 | $C / $10 | $12 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 20 vs 24 bytes |
| c09 seek back, no bytes, then `cpu` | $A / $A | $10 / $10 | $A / $10 | $10 / $16 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 24 bytes |
| c10 `cpu z80`, no `phase` | $C / $C | $E / $E | sigil exit 1 (refused) | - | SIGIL REFUSES (asl accepts) |
| c11 `cpu z80` + `phase 8000h` | $8000 / $C | $10 / $10 | $8000 / $10 | $10 / $14 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 22 bytes |
| c12 AS `section` / `public` / `endsection` | $C / $C | $12 / $12 | sigil exit 1 (refused) | - | SIGIL REFUSES (asl accepts) |
| c13 a Z80-host program, `cpu z80` unchanged | $6 / $6 | $A / $A | $6 / $8 | $A / $C | DISAGREE: labels agree, bytes land elsewhere, image 12 vs 14 bytes |
| c14 cpu-changing `restore` out of a host section, into phased Z80 | $8000 / $C | $10 / $10 | $8000 / $10 | $10 / $14 | DISAGREE: labels agree, bytes land elsewhere, image 18 vs 22 bytes |
| c15 control: `restore` that keeps the CPU (no close) | $C / $C | $12 / $12 | $C / $C | $12 / $12 | MATCH (images identical, 20 bytes) |

Every asl row is `ASL_EXIT=0`, `ASL_DIAG=complete`, `P2BIN_EXIT=0`, and p2bin
printed nothing about the overlap. Every row where asl's L_next differs from its
own bytes-@ is phase-relative by construction (`phase $8000`), and there the
bytes-@ is the physical placement.

What the table establishes:

- **The front end's labels are asl's in every row.** So is the value each moved
  section computes for itself (`dc.l *`, e.g. c01 `0000000E` in both images).
  Only the placement differs.
- **The drift is not confined to the next section.** L_after, one plain `cpu`
  later, sits at asl's address in sigil's labels and `extent - cursor` bytes
  further on in sigil's image (c01: label `$12`, bytes `$16`). Every `Chained`
  section inherits it until a `Pinned` one resets the cursor.
- **The stale tail survives.** c01, asl: `AAAA EEEE 1234 ...`, the new section
  overwriting `CCCC DDDD`. sigil: `AAAA EEEE CCCC DDDD 1234 ...`. The code at
  `$10` believes it is at `$C`, so any reference to `L_next` reaches `CCCC`.
- **It is not specific to the host CPU.** The split occurs when the section
  closed with its cursor behind is in the image, whatever its CPU: a phased Z80
  section with the seek (c04), a phased Z80 section as the moved one (c11, c14),
  and a program whose image CPU is the Z80 (c13).
- **No split where the next section is pinned or nothing is pending.** An `org`
  that leaves the section pins the next one, forward (c05) or backward (c06). A
  seek that returns to the extent leaves nothing pending (c07). A `restore` that
  keeps the CPU is not a closer (c15).
- **Two refusals, both loud.** c10 is stage 1's second-space rule
  (`sections ... are assembled for the Z80 at origin 0xC, in a second address
  space ...`). In c12 sigil does not implement AS `section` / `public` /
  `endsection` (``c12_section_endsection.asm(9):9: error: `section` is not a recognized 68000 mnemonic``), so it is not
  a closer here at all. asl accepts both. Neither is this row's defect.

## Exposure

### Static: every `org` in the corpora's tracked source

`git -C <repo> grep -n -i -E '^[^;]*[[:space:]]!?org[[:space:]]' -- '*.asm' '*.inc' '*.s'`
finds 16 lines in s1disasm, 22 in s2disasm and 19 in skdisasm. **Positive
control:** the same command over this branch finds the planted seeks in the
committed probes (`probes/c01_cpu_same.asm:7: org Start+2`, and the seek line of
every probe).

Read by hand, every site falls in one of four classes:

| class | sites | why it cannot leave a close pending |
|---|---|---|
| zero-offset insn macros | s1 `MacroSetup.asm` 128, 140, 149, 151, 154, 156, 161; s2 `s2.macrosetup.asm` 116, 128, 137, 139, 142, 144, 149 | each returns the cursor to the end inside the macro (`*-1`/`dc.b 0`; `.start+3`/`dc.b 0`/`.end`; `*-3`/`dc.b 0`/`*+1`/`dc.b 0`) and holds no closer |
| zone-ordered tables | s2 `s2.macros.asm` 191, 208, 227; `s2.asm` 88545, 88577 | slot seeks with no closer between them; each table ends with an `!org` to `table + count * stride` (`zoneTableEnd`, `s2.asm` 88577) |
| the corpus's own `org` / `cnop` macro | the plain `org` sites, e.g. s2 `s2.asm` 1963, sk `LockOn Data.asm` 1317-1327, the align macros | s1 and s2 refuse `address < *` and otherwise `!org` forward; sk's `org` (`sonic3k.macrosetup.asm`, the macro beginning at line 9) pads with `dc.b` under the 68000 and fills with `db 0` under the Z80, and never reaches `!org` |
| `!org` at a section boundary | the three drivers' entries and returns, the RAM `!org 0`s, `org $FFFF8000`, sk `Z80 Sound Driver.asm` 220 | reached with no section open (after `dephase` or a cpu-changing `restore`) or leaving the section, so the next section is pinned |

### Dynamic: what the front end actually did

The static reading leans on arithmetic in macros. The census replaces it with a
measurement. `instrument.py` adds a warning at each seek (`SEEKFEED`), and one at
the seek whenever a section is closed with its cursor behind its extent
(`SEEKSPLIT`, and `SEEKSPLIT-BY-ORG` when the closer is a leaving `org`, which
pins). `census.sh` counts them (`census/CENSUS.txt`, per site in
`census/sites.txt`):

| run | SEEKFEED | SEEKSPLIT | of which BY-ORG | how far the run got |
|---|---|---|---|---|
| probes (positive control) | 17 | **12** | 2 | c01-c04, c08-c11, c13, c14 and the pinned c05, c06 (`census/probes-instrument-lines.txt`) |
| s1disasm `sonic.asm` | 103, at the 7 insn-macro sites | **0** | 0 | front end complete: stops at layout on its one error, stage 1's driver refusal |
| s2disasm `s2.asm` | 620, at the 12 sites above | **0** | 0 | front end complete: stops at layout on its one error, the driver refusal |
| skdisasm `sonic3k.asm`, and wrapper roots with `Sonic3_Complete = 0` / `= 1` | 0 / 0 / 0 | **0** | 0 | front end fails (137 / 120 / 110 errors); coverage partial, so the static reading carries skdisasm |

The zeros for s1 and s2 are measurements: the same instrument returns 12 on the
planted shapes, and every seek site those corpora contain fires `SEEKFEED`. For
skdisasm the zero rests on the static reading: it has no in-section seek to fire.

### aeon

`build.sh` at `origin/master` `a38ce7c9` builds `MAIN_ASM="games/${GAME}/game_root.asm"`
(line 175) through `sigil build` (line 980), and its comment at 735 names
`engine/debug/debugger.asm` as the sole include. aeon tracks exactly three
`.asm` files and no `.inc`: `engine/debug/debugger.asm`,
`games/demo/game_root.asm` and `games/sonic4/game_root.asm`.

`git -C aeon grep -n -i -E '^[^;]*[[:space:]]!?(org|phase|dephase|cpu|save|restore|section|endsection)([[:space:]]|$)' origin/master -- '*.asm' '*.inc'`
finds two lines, `cpu 68000` at `game_root.asm:15` in each game. A broader
`include|org|phase|restore` grep finds `org` only in comments. **No `org` means
no seek**, and `directive_org` is the only producer of one, so aeon cannot reach
this shape. **Positive control:** the same command form
(`git grep <pattern> <rev> -- <path>`) over this branch's `HEAD` finds all nine
directive lines of `probes/c04_restore_z80_phased.asm`.

## Where a fix would go, and whose bytes it would move

Not implemented. The re-base is already recorded where it happens:
`close_section` (`eval.rs` 6546-6554) stores the seek as `rebased_at`. What is
missing is its consequence for a section that stays in the image.

- **A: pin it (recommended).** When `close_section` records a seek as the
  re-base, call `self.builder.pin_next_section()` too (or, equivalently, let
  `assign_address_spaces` stamp `Pinned` on an `Image` section that consumes a
  seek re-base, not only a `Foreign` one). The next section then lands at
  `base + cursor`, where its labels are. Wherever it has bytes it overlaps the
  previous section's `[cursor, extent)`, and the linker's overlap check should
  refuse that by name. That is the same policy as `p5_overlap` in
  `2026-09-06-as-org-backwards-fix.md`: an overlay asl resolves last write wins,
  sigil refuses. Every DISAGREE row becomes a refusal. The doc comment on
  `assign_address_spaces` ("Such a section is always refused at link, so the pin
  changes no accepted image") would need rewriting. Unmeasured: that
  `overlap_diag` names these rows, because no code changed.
- **B: model the overlay.** Let the later section overwrite the tail, as p2bin
  does. The DISAGREE rows would then be byte-identical to asl. This is a linker
  policy change the org-backwards parcel declined.
- **A link-side net is the wrong layer.** `place_pass` already sees the move
  (`LmaMove`), but `.emp` sections move legitimately, so refusing a moved section
  would need a per-section "this lma is authoritative" flag, which is what
  `Pinned` already is.

**Bytes moved:** under A, none in any accepted image of the measured population
(s1, s2, skdisasm, aeon: none takes the path). What changes is that sources shaped
like the DISAGREE probes stop assembling, which today they do silently and
wrongly. Under B, those same sources change bytes to asl's; still nothing in the
corpora or aeon.

## Not measured, and why

- **The fix's behaviour.** Nothing under `crates/` may change here, so neither
  A's refusal nor B's bytes were run.
- **skdisasm's dynamic coverage.** Its front end fails before the end of the
  file in all three roots, so the census cannot speak for its unreached lines. The
  static reading covers them.
- **Build variants.** The census ran each corpus's default root and skdisasm's two
  `Sonic3_Complete` values. Other `-D` shapes (and `s3.asm`) were read
  statically only; their `org` sites are in the grep above.
- **aeon by build.** Not built, by instruction. Its answer is the static one:
  no `org` in any of its three AS inputs.

## Corrections to the brief

1. **"the NEXT section's labels at one address and its bytes at another"
   understates it.** Every later `Chained` section is split by the same amount,
   until the next pin (the L_after columns).
2. **"in a 68000 (host CPU) program" is not the condition.** The condition is a
   section in the image closed with its cursor behind its extent, followed by a
   `Chained` section. Z80 sections qualify both as the seek section (c04) and as
   the moved one (c11, c14), and so does a Z80-host program (c13).
3. **A forward `org` that leaves the section closes it but never splits.** It
   pins the next section (c05), and so does a backward one (c06). Neither belongs
   in the exposure class.
4. **AS `section` is not a closer in sigil.** sigil refuses `section`, `public`
   and `endsection` as unknown mnemonics (c12), which is a separate, loud gap.
5. **The hypothesis itself stands as written:** labels at `base + cursor`, bytes
   at `base + extent`.
