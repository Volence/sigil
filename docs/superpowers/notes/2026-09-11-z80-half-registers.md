# The Z80 index-register halves: Sonic 2 builds whole, byte-identical

2026-09-11, branch `parcel/z80-half-registers`, base `8d31a003` (contains the Sonic 2
small-features merge `f8052111`). Evidence (every probe source, asl's result tables and a
listing extract, the sigil comparison reports, the scripts, the logs and the mutation
proofs) is in `2026-09-11-z80-half-registers/` beside this note.

| commit | what |
|---|---|
| `f3b63c1f` | the halves under `cpu z80undoc`, the mode in `save`/`restore`, `MOMCPU`/`MOMCPUNAME`; `cpu_spellings.rs` stops listing the halves as refused |
| `23d7d57d` | `as_z80_half_registers.rs` (524 asl probes) and `as_sonic2_whole_rom.rs` |
| `f9505ad6` | `state.rs`'s `save`/`restore` doc names the mode |
| this note | the note and its evidence, and an em dash out of the `declare_cpu` doc line `f3b63c1f` rewrapped (comment only) |

## Headlines

1. **Sonic 2 builds whole, byte-identical, nothing excluded.** On s2disasm `e45ebf33` with
   `build.lua`'s pre-steps run in a copy, `sigil s2.asm -o s2sigil.bin -p=0
   -z=0,saxman-bugged,Size_of_Snd_driver_guess,after` exits 0 and writes md5
   `9feeb724052c39982d432a7851c98d3e`, CRC32 `7b905383`, 1,048,576 bytes: the ROM
   `build.lua` writes, 0 bytes different. `as_sonic2_whole_rom.rs` builds the reference live
   and holds this.
2. **The 12 Sonic 2 rows leave; with the build script's instruction nothing arrives.**
   Without any `-z` instruction one row arrives, and it is the designed one: the front end
   now finishes, so layout reports the driver's second address space (the same row Sonic 1
   gives without its instruction). Sonic 1 and S3K: no row moves.
3. **asl's halves are a MODE, not an alias.** Under `cpu z80` the names are ordinary
   symbols; under `cpu z80undoc` the bare name is the register even when a symbol or label of
   that name exists. The base binary was silently wrong there: `ixl equ 5` then `ld b,ixl`
   assembled as `06 05` where asl gives `DD 45` (the case `OVERSEER-LOG.md` booked).
4. **The AS front end only.** `sigil-frontend-as` (`eval.rs`, `lib.rs`, `state.rs`, one new
   test) and `sigil-cli` (tests only). The Z80 backend's encoder and tables are untouched,
   so no `.emp` or engine byte can move, and the `.emp` language gains nothing.

## Provenance

| instrument | identity |
|---|---|
| asl, every probe | `s1disasm/build_tools/Linux-x86_64/asl`, md5 **`61e672562465725a8c102288a7da9098`**, through `asl_run` (`asl_ref.sh` sourced by `scripts/run_asl.sh`), `-xx -n -q -A -L -U -i .`, one construct per file. Each round's `probes/m*/asl-run.log` records the md5 of the binary it ran and every probe's `ASL_EXIT`. Values are quoted only from `ASL_EXIT=0` runs with a complete footer; refused runs are read for accept-or-refuse and the error number only. |
| asl, the Sonic 2 reference | `s2disasm/build_tools/Linux-x86_64/asl`, md5 `0dee1f98e6480a4783d27ffd8b90896f`, used only by `build.lua` itself (the pinned build cannot assemble Sonic 2: the residual census, Q0). |
| p2bin / saxman | md5 `4f2fff99c3347bafb93b12d5be1db754` / `704e5c8c361b9959f45bac3e803f0033` |
| lua | `/usr/bin/lua`, Lua 5.5.1, md5 `6c07a63a91719d9b42a76b2c342efedc` |
| sigil, before | built from `8d31a003`, md5 `5a5dae03308b46b0321969b674d8a332` |
| sigil, first cut | the uncommitted tree before one doc edit, md5 `8e152f4f0da29e0d5c66adeff83c9ba7` (`logs/diag-base-v1.log`, `logs/s2whole-v1.log`) |
| sigil, after | built from the tree of `f3b63c1f`, md5 **`cec46f181c272451f28a169d9012393b`**. The two later commits change a test file and a doc comment, no compiled path of the binary. Every "after" number below is this binary. |
| corpora | `cp -a` copies in `/home/volence/sonic_hacks/.scratch/z80-half-registers/`: s2disasm `e45ebf33` (status 0 lines before and after), s1disasm `f6ece657` (17 status lines before and after: modified `artnem/GHZ Bridge.nem` and `artnem/Signpost.nem`, untracked `.aurora/`), skdisasm `2fcd861c` (110 before and after). The checkouts were only read. |
| cargo | `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/z80-half-registers/target`; no `AEON_DIR`. |

## asl's rules, as measured

540 probes in four rounds (`probes/m1` to `m4`, `tables/table-m*.tsv`,
`tables/asl-listings.txt`).

**Spellings.** `ixl`, `ixu` and `ixh` (one register, two names), `iyl`, `iyu` and `iyh`,
in any case (`IXL`, `IxL`, `iXu`, `Iyh` all accepted). Not registers (`#1010 symbol
undefined`): `lx`, `hx`, `xl`, `xh`, `ly`, `hy`, `yl`, `yh`, `ixlo`, `ixhi`, `ix.l`,
`ix.h`.

**The mode.** Under `cpu z80` a half's name is a symbol: `ld a,ixl` is `#1010`, and with
`ixl equ 5` it is `3E 05`. Under `cpu z80undoc` (any case of the directive's name) the bare
name is the register even with that symbol defined (`DD 7D`), and even with a label of that
name (`ixl: nop`, then `ld a,ixl` is `DD 7D`). The mode follows every `cpu` line (`cpu
z80undoc` then `cpu z80` makes it a symbol again) and is part of `save`/`restore`: `cpu
z80undoc`, `save`, `cpu z80`, `restore` gives `DD 7D`; `cpu z80`, `save`, `cpu z80undoc`,
`restore` gives `3E 05`; and Sonic 2's own shape (a 68000 root, `save`, `cpu z80undoc`, the
driver, `restore`) assembles both halves on their own processor. No other directive is
needed. `MOMCPU` is `$80DC` (`dw MOMCPU` is `DC 80`) and `MOMCPUNAME` is `"Z80UNDOC"`
under it.

**Where the name is the register, and where it is still a symbol.** With `ixl equ 5`
defined:

| position | asl | reading |
|---|---|---|
| any operand of `ld` (`ld a,ixl`, `ld hl,ixl`, `ld (hl),ixl`, `ld (ix+1),ixl`, `ld (1234h),ixl`, `ld i,ixl`) | `DD 7D` or `#1350` | register |
| the accumulator ops, `inc`/`dec` (`add a,ixl` `DD 85`, `sub ixl` `DD 95`, `inc ixl` `DD 2C`), `push`, `ex (sp),ixl`, `add hl,ixl` | bytes or `#1350` | register |
| the target of `bit`/`res`/`set` (`bit 0,ixl` is `#1350`) | `#1350` | register |
| the bit number (`bit ixl,b` `CB 68`, `res ixl,b` `CB A8`, `set ixl,(hl)` `CB EE`) | bytes | symbol |
| inside an expression (`ld a,ixl+1` `3E 06`, `ld ixl,ixl+1` `DD 2E 06`) | bytes | symbol |
| inside parentheses (`ld a,(ixl)` `3A 34 12` with `ixl equ 1234h`, `ld hl,(ixl)` `2A 05 00`, `ld a,(ix+ixl)` `DD 7E 05`, `in a,(ixl)` `DB 05`, `out (ixl),a` `D3 05`) | bytes | symbol |
| `jp`, `call`, `jp nz`, `call nz` (`CD 05 00`...), `jr`/`djnz` to a label of that name, `rst ixl` (`#1905`, a value), `im ixl` (`#1320`, a value) | bytes or a value's error | symbol |
| `db ixl` | `05` | symbol |

**The encoding.** The documented instruction on `h` (for `ixu`/`iyu`) or `l` (for
`ixl`/`iyl`) behind `DD` (IX) or `FD` (IY): `ld a,iyl` `FD 7D`, `ld ixl,ixu` `DD 6C`, `ld
iyu,5` `FD 26 05`.

**Accepted:**

- `ld` between a half and one of `a`, `b`, `c`, `d`, `e`, in both directions (40 forms);
- `ld` between the two halves of ONE index register (`ld ixl,ixl` `DD 6D`, `ld ixl,ixu`
  `DD 6C`, `ld ixu,ixl` `DD 65`, `ld ixu,ixu` `DD 64`, and `FD` for IY);
- `ld <half>,n`, `n` in `-128..255` (`-1` `FF`, `-128` `80`, `'A'` `41`, a symbol, a
  forward reference); `-129` and `256` are `#1320 range overflow`, the same window as `ld
  l,n`;
- `add`, `adc`, `sub`, `sbc`, `and`, `xor`, `or`, `cp` `a,<half>`;
- the one-operand `sub`, `and`, `xor`, `or`, `cp <half>` (`sub ixl` `DD 95`);
- `inc`/`dec <half>`.

**Refused:**

- `h` or `l` beside a half, in either position (`ld h,ixl`, `ld ixl,l`: `#1350`). Under
  the prefix `h` and `l` ARE the halves, so accepting these is not a harmless extra: `ld
  ixl,h` would assemble as `DD 6C`, which is `ld ixl,ixu`;
- a half of IX with a half of IY (`ld ixl,iyl`: `#1350`), since one prefix is emitted;
- `(hl)`, `(ix+d)`, `(iy+d)`, `(nn)` beside a half, `i`/`r`, a 16-bit pair (`ld hl,ixl`,
  `ld ix,ixl`, `ld sp,ixl`), `add hl,<half>`, `add <half>,a`, `add b,<half>`, `cp
  <half>,a`;
- the CB shifts (`rlc` to `sll`) and `bit`/`res`/`set` on a half, `push`/`pop`, `in <half>,(c)`,
  `out (c),<half>`, `ex de,<half>`: all `#1350`;
- the one-operand `add`, `adc`, `sbc <half>` (`#1110 wrong number of operands`), and wrong
  operand counts (`ld ixl`, `ld ixl,a,b`, `inc ixl,a`).

## What changed in sigil

`AsmState` carries `z80_undoc`, set by every `cpu` line from its spelling
(`z80_undocumented` beside `cpu_for_spelling` in `lib.rs`) and part of the `save` snapshot.
`lower_z80` calls `index_halves` when the mode is on: an operand that is exactly a half's
name, in a register position (every operand of the mnemonics above, the second of
`bit`/`res`/`set`), is a half. The instruction is checked against the accepted list, each
half is rewritten to `h` or `l`, the documented form lowers through the unchanged Z80
backend, and the prefix goes in front (any fixup offset moves with it). Everything outside
the list is refused with a message that names the half; the `h`/`l` and IX/IY refusals say
why. `MOMCPU` and `MOMCPUNAME` read the mode.

**The `MOMCPU` change is a decision, flagged.** It is not the halves, but it is the same
mode and the old value was a silent difference (`dw MOMCPU` `80 00` against asl's `DC 80`,
and `if MOMCPUNAME="Z80UNDOC"` taking the other branch, both at exit 0). Sonic 2 tests
`notZ80(MOMCPU)`, which accepts both values, so its bytes do not move (proven: the whole
ROM). No corpus compares `MOMCPUNAME`, and no aeon `.asm`/`.inc`/`.s` at aeon
`origin/master` `6d4b5656` names `z80undoc` or `MOMCPU` (`git grep` of that revision's
objects; no working file read).

**The probe matrix, whole images compared** (`scripts/check_sigil.py`,
`tables/check-m*-{base,c1}.txt`):

| round | probes | before: match / both refuse / sigil refuses / sigil accepts / differ | after |
|---|---:|---|---|
| m1 forms | 340 | 14 / 177 / 144 / 0 / 5 | 161 / 177 / 2 / 0 / 0 |
| m2 positions, mode, ranges | 108 | 36 / 38 / 15 / 9 / 10 | 57 / 37 / 4 / 10 / 0 |
| m3 symbol positions | 82 | 29 / 27 / 6 / 12 / 8 | 43 / 39 / 0 / 0 / 0 |
| m4 Sonic 2 shape and lines | 10 | 1 / 1 / 8 / 0 / 0 | 9 / 1 / 0 / 0 / 0 |

The 16 that still differ after are left out of the test table by name and are not the
halves: `sll a`/`sll b` (sigil has no `sll`, loud); one-operand `add`/`adc`/`sbc` on a plain
register or an immediate, 9 probes (sigil accepts, asl refuses `#1110`: pre-existing
accept-more, unchanged by this parcel); upper-case documented registers, 4 probes (`ld
A,ixl`, `ld IXU,B`: sigil does not fold `A`/`B` and refuses, loud; for `ld A,ixl` the
message blames the half, because `A` reads as an immediate); and a `save` with no
`restore` (asl `#1460`, sigil accepts). m2's `sigil accepts` count moved from 9 to 10
because `mode_68k_save_undoc` (the missing `restore`) was refused before only by the
unresolved `ixl` it contained.

## The half-fix matrix: red-first proofs

`scripts/mutproof.py`: each mutation reads its files' committed baseline (`git show
HEAD:`), refuses unless the old text occurs exactly once, applies it, quotes the mutated
lines back FROM DISK (and checks the old text is gone), runs the test binary, then restores
every file from the committed baseline and requires the tracked tree clean. All 15 went
red; every restore left the tree clean (`logs/mutations/*.log`, each with the HEAD it ran
at).

| # | half-fix | mutated line, read back from disk | red |
|---|---|---|---|
| m1 | `ixl`/`iyl` only (high-half arms deleted) | `eval.rs:11780 "ixl" => ...`, `:11781 "iyl" => ...`, `:11782 "iyu" \| "iyh" => ...` then, after the second deletion, `"iyl"` followed by `_ => None` | 7 of 10: spellings, ld, arithmetic, forms, symbol, Sonic 2's lines, the reason test |
| m2 | the high halves only (low-half arms deleted) | `eval.rs:11780 "ixu" \| "ixh" => ...` directly under `match ...`, then `"iyu" \| "iyh"` directly after it | 8 of 10 (adds the mode test) |
| m3 | IY written, IX's prefix encoded | `eval.rs:8480 IndexReg::Iy => 0xDD,` | 7 of 10 |
| m4 | `h`/`l` accepted beside a half | `eval.rs:8431 if false && atoms.iter().any(\|a\| reg_word(a, &["h", "l"])) {` and `:8443 const PLAIN: &[&str] = &["a", "b", "c", "d", "e", "h", "l"];` | ld, the reason test |
| m5 | IX and IY halves accepted together | `eval.rs:8418 ... halves.iter().find(\|h\| false && h.1 != reg) {` | ld, forms, the reason test |
| m6 | only `ld` (the accumulator and `inc`/`dec` arms deleted) | `eval.rs:8452 },` then `:8453 _ => false,` (the Ld arm's close followed directly by the fallback) | arithmetic, symbol, Sonic 2's lines |
| m7 | the mode ignored (halves under plain `z80` too) | `eval.rs:8320 let prefix = if true {` | the mode test |
| m8 | the half a register in every operand position | `eval.rs:8404 _ => 0, // mutation m8: every operand position` | the symbol test |
| m9 | `restore` loses the mode | `state.rs:179 }` then `:180 Ok(())` (the assignment between them deleted) | the mode test |
| m10 | `MOMCPU` still `$80` | `eval.rs:2514 Cpu::M68000 => 0x68000,` then `:2515 Cpu::Z80 => 0x80,` | the `MOMCPU` test |
| m11 | one-operand `add`/`adc`/`sbc` with a half accepted | `eval.rs:8456 (Add \| Adc \| Sbc \| Sub \| And \| Xor \| Or \| Cp \| Inc \| Dec, 1) => is_half(0),` | arithmetic |
| m12 | no halves at all | `eval.rs:8320 let prefix = if false {` | 8 of 10 |
| m13 | `ld <half>,n` refused | `eval.rs:8448 reg_word(&atoms[1], PLAIN)` | ld, symbol |
| s2a | the Sonic 2 whole-ROM test, wrong prefix | as m3 | `sonic_2_builds_to_the_reference_rom_byte_for_byte`: "695 bytes differ in 18 runs, the first: [0x0EC898, 0x0EC899) ..." (inside the stored driver) |
| s2b | the Sonic 2 whole-ROM test, no halves | as m12 | same test: "sigil failed" |

**A correction to the proof harness's own output.** For a DELETION the harness quotes the
lines around the deletion site, derived from the baseline's offset, not the deleted text
(it is gone); the "old text now absent" check is what proves the deletion landed, and the
quoted neighbours show the gap. m8's first run quoted the wrong line: its replacement `_ =>
0,` also occurs at `eval.rs:3249`, and the harness quoted the first occurrence. m8 was run
again with a replacement unique in the file; the table quotes that run, and it went red on
the same test.

## Diagnostic multiset diffs, exact lines

`scripts/run_diag.sh`, stderr sorted and diffed, before `5a5dae03` and after `cec46f18`
(`logs/diag-base-c1.log`):

**Sonic 2, with `build.lua`'s instruction** (`-p=0 -z=0,saxman-bugged,...`): 13 rows
before, 1 after. Left, all `error: unresolved symbol ... in operand`:
`s2.sounddriver.asm` 1863 `iyl`, 1866 `iyu`, 1976 `iyl`, 1979 `iyu`, 2258 `ixl`, 2259
`ixu`, 3475 `ixl`, 3478 `ixu`, 3650 `ixl`, 3651 `ixu`, 3687 `ixl`, 3689 `ixu`. Arrived:
none. What remains is `s2.asm(91275):2: warning: \`shared\` is ignored ...`, by the small
features' decision. Exit 1 before, exit 0 after, image the reference ROM.

**Sonic 2, no instruction** (the configuration the earlier notes counted): 13 before, 2
after. Left: the same 12. Arrived: `s2.sounddriver.asm(248):6: error: section \`sec0#2\`
[0x0, 0x1308) is assembled for the Z80 at origin 0x0, in a second address space this org
opens outside the ROM image, and no -z instruction places it into the ROM`. It is not new
behaviour: before, the front end's 12 errors stopped sigil before layout ("sigil stopped
at the front end"), and now layout runs and gives stage 1's designed refusal, the row Sonic
1 gives at `sound/z80.asm(9)` in the same configuration.

**Sonic 1** (`sonic.asm`), without and with its instruction: rows identical (1 and 0),
stdout identical but for the output path, image identical before and after (md5
`31b56ea47d5ce1b9a6c70dc15d2ec985`, 551,288 bytes: the shared checkout's two modified art
files make it not the `f6ece657` ROM; the corpus test builds the clean revision).

**S3K** (`sonic3k.asm`): 137 rows before, 137 after, identical; stdout identical.

## Sonic 2, whole

`scripts/run_luaref.sh`: `build.lua` run unmodified in a `cp -a` copy gives
`s2built.bin` md5 **`9feeb724052c39982d432a7851c98d3e`**, CRC32 **`7b905383`**,
**1,048,576** bytes (re-derived; equal to the census).

`scripts/setup_corpora.sh`: a second copy with `build.lua`'s pre-steps only (the text of
`build.lua` up to `-- Build the ROM.`: song compression and the PCM/DPCM conversions; 80
generated files).

`scripts/run_s2_whole.sh`, in that copy:

```text
sigil-c1 s2.asm -o s2sigil.bin -p=0 -z=0,saxman-bugged,Size_of_Snd_driver_guess,after
SIGIL_EXIT=0 seconds=4 stderr-rows=1   (the `shared` warning)
ROM size is $100000 bytes (1024 KiB). About $661 bytes are padding.
REF   md5 9feeb724052c39982d432a7851c98d3e crc32 7b905383 size 1048576
SIGIL md5 9feeb724052c39982d432a7851c98d3e crc32 7b905383 size 1048576
DIFF 0 bytes in 0 runs; sizes equal: True
```

**The post-p2bin patch changes zero bytes, confirmed rather than assumed**
(`scripts/run_nopost.sh`): `build.lua` with its two post-p2bin calls removed
(`amend_sound_driver_size()` and `common.fix_header(...)`) writes the same md5
`9feeb724...`. The share file it leaves reads `comp_z80_size 0xF64` and `movewZ80CompSize
0xEC04E`; the word at `0xEC050` is already `0F64`, the header checksum at `0x18E` already
`D951`, the end at `0x1A4` already `000FFFFF`.

**The whole-ROM test** (`crates/sigil-cli/tests/as_sonic2_whole_rom.rs`), in the style of
the Sonic 1 corpus test: `git archive` of s2disasm `e45ebf33` into the target tmp dir, the
three tool md5s checked, `build.lua` run for the reference (pinned by CRC32 and size), then
sigil with the instruction on the same tree, compared whole. The suite root comes from
`test_support::unnamed_default_tree`, the absent-root path through `suite_root_absent`; it
reads no reference variable (`reference_env_read_is_routed` green below). Red-first: s2a
and s2b above.

**Whether the driver plays** is a runtime question. TAGGED for the controller; no emulator
was touched. (The image is byte-identical to the retail-CRC ROM `build.lua` writes, which
is the strongest static answer available.)

## Scoped suites and clippy

`SIGIL_ALLOW_PARTIAL=1 cargo test --release -p <crate> --no-fail-fast -- --nocapture`,
target in the scratch directory, no `AEON_DIR`, each run stamped with its tree
(`logs/suite-*.summary.txt`): HEAD `f9505ad6`, branch `parcel/z80-half-registers`,
tracked-changes 0. No commit was made while a run was in flight.

| run | binaries | passed | failed | ignored |
|---|---:|---:|---:|---:|
| `-p sigil-frontend-as` | 75 | 835 | 0 | 0 |
| `-p sigil-cli` | 170 | 809 | 0 | 1 |
| `-p sigil-harness --test reference_env_read_is_routed` | 1 | 1 | 0 | 0 |

The `sigil-cli` run printed the partial-run banner (131 reference-dependent test binaries
UNMEASURED, no reference tree named). Both corpus tests ran in it and passed:
`sonic_1_builds_to_the_reference_rom_byte_for_byte` and
`sonic_2_builds_to_the_reference_rom_byte_for_byte`. `cargo clippy --release -p
sigil-frontend-as -p sigil-cli --all-targets -- -D warnings` exited 0 (`logs/clippy.log`).

## Open, and why

- **`sll` and the other undocumented forms** (`rlc (ix+1),b`, `in (c)`, `out (c),0`):
  refused by name, loud. No corpus uses them; `cpu_spellings.rs` keeps two of them as its
  refusal rows.
- **One-operand `add`/`adc`/`sbc` on a plain register or immediate**: sigil accepts
  (`add b` is `80`), asl refuses with `#1110`. Pre-existing accept-more in both `cpu z80`
  and `cpu z80undoc`, silent in the dangerous direction only in that a source asl rejects
  assembles. Worth its own row.
- **Upper-case documented registers** (`ld A,b`, `LD A,IXL`): sigil does not fold them and
  refuses, loud; asl accepts. With a symbol named `A` defined, sigil would read it as the
  symbol: unmeasured here, worth a probe of its own.
- **A `save` with no `restore`**: asl `#1460 missing RESTORE`, sigil accepts.
- **Whether the driver plays**: TAGGED above.

## Things in the brief that turned out wrong

1. **"asl ... may accept `ixh`/`iyh` too"** was right, and so was the rest of the
   controller's hypothesis (DD/FD on the 8-bit form; no `h`/`l` mix). What it did not
   predict: the halves are a MODE (symbols under `cpu z80`, registers under `z80undoc`
   even over a defined symbol), which the `z80undoc`-is-plain-`Cpu::Z80` mapping could not
   express, and the name stays a symbol in several operand positions (bit numbers, jump
   targets, inside parentheses and expressions).
2. **"only `ld` handled, not ... `add adc sub sbc ...`"**: the arithmetic forms have a
   trap of their own. asl takes the one-operand `sub`/`and`/`xor`/`or`/`cp <half>` and
   REFUSES one-operand `add`/`adc`/`sbc <half>`.
3. **"Expected: Sonic 2's 12 rows leave, and nothing arrives anywhere"**: true with the
   build script's instruction. Without one, one row arrives (the driver-placement
   refusal), because the front end now completes; it is the designed row, not a defect.
4. **The census's plan, "front end and Z80 backend"**: only the front end was needed; the
   backend encodes the documented forms and the prefix is the whole difference.
