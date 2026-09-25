# S3K whole ROM: sigil builds `buildSK.lua`'s image byte-identical

2026-09-25, measurement parcel, branch `measure/s3k-whole-rom`, base `ec860c7e`
(master, read from the tree at start). By the end master was `dd867b80`; `git diff
--stat ec860c7e dd867b80` is `docs/QUEUE.md` only, so no assembler behaviour moved.
**No sigil source was changed**, so no suite run was needed and none was made.
Evidence, scripts, probe sources and run records are in `2026-09-25-s3k-whole-rom/`
beside this note. Scratch: `/home/volence/sonic_hacks/.scratch/s3k-whole-rom/`.

## Headlines

1. **Sonic 3 & Knuckles builds whole and byte-identical.** sigil `ec860c7e`, on the
   census wrapper root with `buildSK.lua`'s own p2bin instruction, exits 0 with no
   diagnostic at all and writes md5 `4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32
   `0658f691`, 2,097,152 bytes: **0 differing bytes**, no window, planted-byte control
   reported exactly. Three sigil runs, identical.
2. **Sonic 3 Complete also builds byte-identical**, and there the header fix is
   load-bearing: the source hardcodes `$DFB3`, the image needs `$E790`, and sigil
   writes `$E790` because its AS route has folded `fix_header` in since `5d72e4a4`.
   So does S&K with `buildSK.lua`'s own `improved_sound_driver_compression = true`
   (`kosinski-optimised`, 4,868 bytes and the checksum away from the shipped image).
3. **Sonic 3 alone does not build**: 6 errors, one class, multi-character
   single-quoted strings in `dc.b`. With those 6 operands rewritten to double quotes
   (neutral under asl, measured), sigil builds it byte-identical too, so that class is
   the whole gap. Booked, not fixed.
4. **The plain root still needs the wrapper**: 17 rows, all tracing to the one
   undefined `Sonic3_Complete`. A `-D` is small, but asl's `-D` defines a SET
   variable, and the front end's existing `Options.defines` does something else. Sized
   below; not built.

## Instruments

| Instrument | Identity |
|---|---|
| sigil | built from `ec860c7e` (clean tree) with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/s3k-whole-rom/target`, md5 `41b5d2676ade740fd1c4e9d6a0869e72`; `--version` reports revision `ec860c7e4cd207bf1fefe9cea361759cdcb01bee`, tree clean. One binary, one target dir; no archive build shared it |
| skdisasm | `git archive 2fcd861c208f342b6d14df694c6422c74f20a4be`, tar md5 `0a4e468053abc841d867d130748629be` (CRC32 `d00ce908`), the census and codepage parcels' value |
| asl, p2bin | only via `asl_ref.sh`'s `asl_run`: `s1disasm/build_tools/Linux-x86_64/asl` md5 `61e672562465725a8c102288a7da9098`, p2bin md5 `4f2fff99c3347bafb93b12d5be1db754` |
| the build scripts' own asl | skdisasm's `build_tools/Linux-x86_64/asl` is md5 `61e672562465725a8c102288a7da9098` too (`find_assembler` picks `build_tools/<os>-<arch>/asl`), so every `buildS*.lua` reference below was made by the pinned build |
| gen tree | pristine + `Sound/DAC/generated` (99 files) + `Sound/PCM/generated` (3 files) from a copy where `buildSK.lua` ran, + `wrapper.asm` (`Sonic3_Complete = 0` then `include "sonic3k.asm"`); `scripts/mk_trees.sh`, `logs/mk_trees.log` |
| lua | `/usr/bin/lua` 5.5 (the census's) |

## Step 0

* **Reference.** `buildSK.lua` run unmodified three times in one copy: md5
  `4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32 `0658f691`, 2,097,152 bytes every run, no
  `sonic3k.log` (asl emitted no diagnostic). The brief's numbers re-derived.
* **0 front-end rows on the wrapper root**: yes, and more: the whole run exits 0.

## Step 2: what `buildSK.lua` passes, and sigil's equivalent

`buildSK.lua` calls `build_rom_and_handle_failure("sonic3k", "skbuilt", "-D
Sonic3_Complete=0", "<p2bin args>", false, ...)`, then `fix_header("skbuilt.bin")`.
`common.lua`'s `assemble_file` expands that to

```text
asl -xx -n -q -A -L -U -E -i .  -D Sonic3_Complete=0 sonic3k.asm
p2bin -p=FF -z=0,kosinski,Size_of_Snd_driver_guess,before -z=1300,kosinski,Size_of_Snd_driver2_guess,before sonic3k.p skbuilt.bin
```

(`create_header_file` is `false`, so there is no `-c` and no share file).

| toolchain flag | what it does | sigil |
|---|---|---|
| `-xx` | most detailed error text | n/a (sigil's diagnostics are its own) |
| `-n` | error numbers in messages | n/a |
| `-q` | quiet | n/a (sigil prints one `built:` line) |
| `-A` | compact symbol table (speed) | n/a, no byte effect |
| `-L` | write a listing | none: sigil writes no listing (census Q4 item 2) |
| `-U` | case-sensitive symbols | always on in sigil |
| `-E` | diagnostics to `sonic3k.log` | n/a, sigil writes stderr |
| `-i .` | include path | sigil resolves includes against the root file's directory, which is `.` here |
| `-D Sonic3_Complete=0` | predefine a symbol | **none on the AS route**; the wrapper root stands in (step 4) |
| p2bin `-p=FF` | gap fill byte | `-p=FF`, same spelling |
| p2bin `-z=0,kosinski,Size_of_Snd_driver_guess,before` | place and compress the first Z80 blob | `-z=...`, same spelling |
| p2bin `-z=1300,kosinski,Size_of_Snd_driver2_guess,before` | the second | same |
| `fix_header` (after) | end-of-ROM at `0x1A4`, checksum at `0x18E` | **folded into sigil's AS route** (`sigil_link::apply_sega_header`, since `5d72e4a4`), idempotent |

**Pre-steps**: only `convert_pcm_files_in_directory("Sound/PCM")` and
`convert_dpcm_files_in_directory("Sound/DAC")`, which the gen tree carries. There is no
separate compression pre-step: the Kosinski compression of both drivers is p2bin's
`-z`, which sigil performs itself. **Post-steps**: only `fix_header`. There is no
`amend_sound_driver_size` in `buildSK.lua` (that was Sonic 2).

**Isolating the assembler.** `scripts/postcheck.sh` hand-runs exactly the asl and p2bin
lines above through `asl_run` (exit 0, `ASL_DIAG=complete`, `2 passes`, no log) and keeps
p2bin's raw output: md5 `4ea493ea...`, the reference. So at the shipped setting
`fix_header` changes zero bytes: the source hardcodes `Checksum: dc.w $DFB3`
(`sonic3k.asm:75`), and `header.py` computes `$DFB3` from the image, end-of-ROM
`$001FFFFF` both ways, with a liveness control (a planted bit moves the required sum to
`$E0B3`). Applying the toolchain's own `fix_header` (the lua, `require
"build_tools.lua.common"`) to a copy of sigil's image changes nothing either. **The
comparison therefore isolates the assembler for this shape: raw asl + p2bin, sigil, and
the fixed reference are one md5.** The same asl line on the wrapper root (no `-D`) also
gives `4ea493ea...`, so the wrapper is neutral under asl.

## Step 3: the compare

`logs/compare-wrapper.txt`: reference and candidate both 2,097,152 bytes, CRC32
`0658f691`; **0 differing bytes**, no length mismatch; control planted at `[0, 1048576,
2097151]`, comparer reported exactly those; the planted copy differs from the reference
in exactly 3. No differing range, so there is nothing to map to asl's listing and no
class to book for this shape.

**Controls that the run was not vacuous** (`runs/ctl-*`):

* pristine tree (no generated PCM/DAC), same arguments: exit 1, 75 rows (29 `int()`
  float, 29 `division by zero`, the `cannot include Sound/.../generated/...` rows and
  their cascades). sigil reads the inputs the reference reads.
* gen tree without the `-z` instructions: exit 1, the two second-address-space
  refusals (`Sound/Z80 Sound Driver.asm(226)` for `[0x0, 0x116B)` and `(4444)` for
  `[0x1300, 0x1B43)`). The two-blob `before` placement is live in the identical image;
  this is the whole-ROM evidence the stage-2 note's `probes/p_s3k.asm` stood in for.

## The two other shapes skdisasm ships (beyond the brief, cheap, and one of them fails)

`scripts/flips.sh`, `logs/flips.log`, `logs/s3c-raw.log`. Each build script run
unmodified in its own pristine copy, twice, identical.

| shape | build script | reference | sigil |
|---|---|---|---|
| S3 Complete | `buildS3Complete.lua`: `-D Sonic3_Complete=1`, same p2bin line | md5 `b23c88b46f5ec7377f1b6f109d97f5a5`, CRC32 `a651623a`, 3,360,160 B | wrapper `Sonic3_Complete = 1`: exit 0, 0 rows, **same md5, 0 bytes** (control OK) |
| S3 alone | `buildS3.lua`: `s3.asm`, no `-D`, `-z=...,uncompressed,...` twice | md5 `d724ea4dd417fe330c9dcfd955c596b2`, CRC32 `9bc192ce`, 2,097,152 B | **exit 1**, 6 errors, 2576 warnings |
| S&K, improved driver compression | `buildSK.lua` with its own setting `improved_sound_driver_compression = true` (edit shown by `diff`), so `-z=...,kosinski-optimised,...` | md5 `296c10571cb0268a43170e6f43769465`, CRC32 `9fe129b9`, 2,097,152 B; differs from the shipped reference in 4,868 bytes over 139 ranges, including the checksum (`$DFB3` to `$F657`) | same `-z` spelling: exit 0, 0 rows, **same md5, 0 bytes** (control OK). `scripts/flip_kosopt.sh`, `logs/flip-kosopt.log` |

### S3 Complete: the folded header fix is load-bearing

asl + p2bin by hand with `-D Sonic3_Complete=1`, raw: md5
`f433c223ef0e770db96b66ad0c66f80d`, which differs from the reference in exactly one
range, `0x00018E +2: dfb3 vs e790`. The source hardcodes `$DFB3` for every setting;
`fix_header` computes `$E790` for this image. sigil's own image already holds `$E790`,
so sigil's output equals the fixed reference and differs from raw asl in those 2 bytes
only. **This is the census Q5 item 1 fault, measured closed on a corpus that did not
exist when it was found**: without the fold, sigil would have written a stale checksum
here at exit 0.

### S3 alone: one class, multi-character single-quoted strings in `dc.b`

All 6 errors are `operand N out of range -128..=255` at `s3.asm` 394, 410, 414, 416,
418, 420: `dc.b 'J',0,'UE'` and `dc.b 6, 'DEVELOPED FOR USE ONLY WITH',0` and its four
siblings (the TMSS-style region message). `sonic3k.asm` has no such line, which is why
the S&K shapes never met it.

**What asl does** (`asl_run`, every quoted value from an exit-0 run, `logs/probe-sq*.log`):

| probe | source | asl bytes | sigil |
|---|---|---|---|
| sq1 | `dc.b 'J',0,'UE'` / `dc.b 6,'ABC',0` / `dc.b 'Q'` / `dc.w 'UE'` / `move.w #'UE',d0` | `4a005545` `0641424300` `51` `5545` `303c5545` | refuses the first two lines |
| sq4 | `dc.b 'ABCDE'` | `4142434445` | refuses (`operand 280284578885`) |
| sq3 | `charset 'A',$11` then `dc.b 'AB','A'` / `dc.b "AB"` | `114211` `1142` | refuses |
| sq2 | `dc.b 'AB'+1` | `4143` | refuses |

So in `dc.b` asl reads a multi-character `'...'` as a string, one translated byte per
character, exactly like `"..."`; elsewhere (`dc.w`, an immediate) it stays the packed
integer. sq2 shows it is a typed value, not a lexer rewrite: `'AB'+1` is `"AC"`.
sigil's lexer folds every `'...'` into one `Tok::Int` at lex time
(`crates/sigil-frontend-as/src/lexer.rs`, the `b'\''` arm), so the `dc.b` string path
never sees it.

**The 2576 warnings are cascades.** Rewriting only those 6 operands to double quotes
(`scripts/s3_requote.sh`, each edit printed with `diff` on disk) is neutral under asl:
`asl_run` + p2bin on the edited tree gives the reference md5 `d724ea4d...` (0 bytes,
control OK). On that tree sigil exits 0 with **0 rows** and writes md5
`d724ea4dd417fe330c9dcfd955c596b2`: 0 differing bytes, control OK. The odd-address
warnings were the layout shift from the 6 refused operands. **This class is the whole
S3-alone gap.**

**Booked, not fixed.** It is loud (sigil refuses, never mis-emits), but it is not a
one-line change: the lexer has to keep the quoted form so that `dc.b` can take it as a
string while `dc.w`/immediates keep the integer, and sq2 says string arithmetic has to
follow. Probes sq1 to sq4 are the red-first material for whoever takes it.

(A regex slip in the first requote attempt rewrote line 394 to `'J",0,"UE'`; asl then
built a different image, 332,802 bytes off, and the neutrality compare caught it. The
anchored rewrite is the one recorded.)

## Step 4: the plain root, and a `-D`

**Rows.** `runs/plain/rows`, sigil on `sonic3k.asm` with the buildSK p2bin line and no
wrapper: exit 1, **17 rows, the census's count**. Their text is not "all
`Sonic3_Complete`" as the census summarised: 10 name `Sonic3_Complete` (`unresolved if
condition` at 81, 289, 1673, 1710, 9548, 10862, 16232, 200210, 200927, 203583), 6 name
`strip_padding` (200194, 200642, 200919, 201931, 202157, 203576), and 1 is `unresolved
symbol LockonHeader` at 315. All 17 have one cause: `strip_padding = 0|Sonic3_Complete`
(`sonic3k.asm:35`), and `LockonHeader` is defined in `Lockon S3/LockOn Pointers.asm`,
which a `Sonic3_Complete` conditional gates. They are still the only difference: the
wrapper, which is exactly one `Sonic3_Complete = 0` line ahead of the include, takes the
run to 0 rows and the reference image.

**asl needs the define too** (`logs/asl-plain.log`): the same asl line without `-D`
exits 2, `ASL_DIAG=INCOMPLETE`, 9 `error #1820: expression must be evaluatable in first
pass` (81, 289, 1673, 9548, 10862, 16232, 200210, 200927, 203583). asl does not flag
the 8 `if ~~Sonic3_Complete` / `if ~~strip_padding` sites sigil reports, nor 315; its
pass loop stopped after pass 1, so its count is a floor. Not pursued: no build reaches
it.

**Would `-D NAME=VALUE` close them?** By construction yes, *predicted not measured*
(nothing was built): the wrapper is the same symbol visible from the first line, and
asl gives the same image from `-D Sonic3_Complete=0 sonic3k.asm` and from
`wrapper.asm` (`logs/postcheck.log`, runs A and B). S3 Complete and S3 need nothing
more: `-D Sonic3_Complete=1` for the one, nothing for the other.

**What asl's `-D` actually is** (`scripts/dprobe.sh`, `logs/dprobe.log`, exit-0 values
only):

| case | asl |
|---|---|
| `-D FOO=5`, `dc.b FOO` | `05` |
| `-D FOO` (no value) | `01` |
| `-D FOO=1,BAR=2` (comma list, one argument) | `0102` |
| `-D FOO=$10` | `10` |
| `-D FOO=5`, use `foo` (with `-U`) | refused, `#1010 symbol undefined`: case sensitive |
| `-D FOO=0` / `=1` under `if FOO` | `02` / `01` |
| `-D FOO=5`, then `FOO set 7` | `07` accepted; `dc.b FOO` / `FOO set 7` / `dc.b FOO` gives `0507` |
| `-D FOO=5`, then `FOO = 7` or `FOO equ 7` | **refused**, `#2035 variables cannot be redefined as constants` |

**asl's `-D` defines a SET variable.** The AS front end already has
`Options.defines` (`crates/sigil-frontend-as/src/lib.rs`), fed today by the aeon
harness, but its documented semantics are different: "an in-file `=`/`equ` of the same
name wins" silently. Wiring the CLI straight to it would accept the two programs asl
refuses. That over-acceptance changes no S3K byte, but it is visible surface.

**Size.** Small. In `run_asm` (`crates/sigil-cli/src/main.rs`): a `-D` arm and
`valued("-D")` in the entry's options, plus a usage line; about 10 lines. The existing
`parse_define` (used by `sigil emp` / `sigil test`) takes `NAME=INT` only, so asl's
grammar needs a small parser of its own: a comma list, an optional value defaulting to
1, `$hex`, and an `i64` range check (about 20 lines). The real decision is the semantics:
either a SET-variable flavour of define (a `set` may rebind; `=`/`equ` refused as #2035),
which is a small front-end change beside `defines`/`guarded_defines`, or reuse `defines`
and record the over-acceptance. Tests: the eight `dprobe` rows as red-first cases (asl
bytes as expected values) plus one S3K plain-root whole-image run. Owner call.

## Booked

1. **S3 alone: multi-character `'...'` in `dc.b`** is a string (6 errors, the whole
   S3-alone gap). Loud, M. Evidence above; red-first material is sq1 to sq4.
2. **A `-D` for the AS route**, sized above, with the SET-variable semantics question.
   Owner call.
3. **`Options.defines` documents silent-override semantics that asl's `-D` does not
   have** (asl refuses `=`/`equ` after `-D`). Recorded for whoever consumes it next;
   not measured against the aeon harness's actual use, which may intend exactly that.

## What this does not show

* **Whether any of the three ROMs plays.** TAGGED for the controller. No emulator was
  touched.
* **Source switches other than `Sonic3_Complete`.** S3K's other conditionals (the
  census's blind region) were not flipped. The one build-script setting,
  `improved_sound_driver_compression`, was (table above).
* **Listing, share file, `-D`**: outputs outside the image, as in census Q4 item 2.

## Things in the brief that turned out wrong

1. **"Apply the toolchain's own post steps to sigil's output the same way the census
   did for S1/S2."** Since `5d72e4a4` sigil's AS route performs `fix_header` itself, so
   its output is already post-step. For `buildSK.lua` it makes no difference (the step
   changes 0 bytes); for S3 Complete it is what makes the image right. Isolating the
   assembler therefore means comparing against raw asl + p2bin as well, which is done
   above. There is no size patch and no compression pre-step in `buildSK.lua`.
2. **"The `Sonic3_Complete`-style rows"** read as all naming that symbol: 6 of the 17
   name `strip_padding` and 1 `LockonHeader`. Same single cause, different text.
3. **"sigil has no `-D`."** The AS route has none, but `sigil emp` and `sigil test`
   have `-D NAME=INT`, and the AS front end has `Options.defines` with semantics that
   differ from asl's `-D`. The sizing depends on that.
4. **"Map each [diff] via asl's listing."** There was no diff for any shape that
   built, so no listing value is quoted. `sonic3k.lst` from the exit-0 run is in
   scratch (`logs/asl-A-sonic3k.lst`, 23 MB, not committed).
5. **Step 0 said "reproduce 0 frontend rows"**: the run does better than that, it
   finishes; and "Nothing has yet compared a full image" is now false for S3K.
