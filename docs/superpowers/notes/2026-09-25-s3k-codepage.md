# S3K `codepage`: the census re-measured, then the directive implemented

2026-09-25, parcel `S3K-FOUR-CLASSES` (project SIGIL-AS-REPLACEMENT), branch
`parcel/s3k-codepage`, base `e8efbec8` (master, read from the tree at start).
Evidence, scripts and raw row multisets are in `2026-09-25-s3k-codepage/` beside
this note.

## Step 0: the census's 120 rows, re-measured

### Instruments

| Instrument | Identity |
|---|---|
| sigil | built from `e8efbec8` with `CARGO_TARGET_DIR=/home/volence/sonic_hacks/.scratch/s3k-codepage/target`, md5 `04ce076bcd41e5a19a8ecccfad27be1d`; `sigil --version` reports revision `e8efbec8208d863489eef66067c40f80b7d291d5`, tree clean |
| skdisasm | `git archive` of `2fcd861c208f342b6d14df694c6422c74f20a4be` (the census revision) into scratch; tar md5 `0a4e468053abc841d867d130748629be` |
| gen tree | the pristine extract plus `Sound/DAC/generated` (99 files) and `Sound/PCM/generated` (3 files) copied from a copy where the stock `buildSK.lua` ran, plus the census's wrapper root (`Sonic3_Complete = 0` then `include "sonic3k.asm"`); `mk_trees.sh` |
| reference ROM | `buildSK.lua` run unmodified in that copy: md5 `4ea493ea4e9f6c9ebfccbdb15110367e`, CRC32 `0658f691`, 2,097,152 bytes, the census's value |
| pinned asl | `s1disasm/build_tools/Linux-x86_64/asl`, md5 `61e672562465725a8c102288a7da9098` |

The run is `run_sigil.sh` with the census's argument list (`-p=FF` and the two
`before` `-z` instructions). Exit 1, 120 stderr rows.

### The set diff against the census's `rows/sk-gen-wrapper.rows`

`setdiff-base-vs-census.txt` (multiset, `setdiff.py`):

| class | census | now | left | entered | common |
|---|---:|---:|---:|---:|---:|
| `$$name` labels read as a hex prefix | 93 | 93 | 0 | 0 | 93 |
| `codepage` directive | 19 | 19 | 0 | 0 | 19 |
| `abcd` / `subx` | 6 | 6 | 0 | 0 | 6 |
| `(d8,PC,Xn)` | 2 | 2 | 0 | 0 | 2 |
| **total** | **120** | **120** | **0** | **0** | **120** |

Member for member identical: no class moved, so step 1 proceeds. The 19
`codepage` rows are `sonic3k.macros.asm(140)` (the page-building block),
`sonic3k.asm(9951)` (the level-select plane-map code) and 17 at
`sonic3k.asm(10553..10569) levselstr(2)`, one per `levselstr` call. The census
text says the 17 are reported at `sonic3k.asm(10553) levselstr(2)`; they are at
seventeen consecutive call-site lines starting there, which its own row file
shows.

## Step 1: `codepage`, measured on the pinned asl, then implemented

### The oracle

Every value is from `asl_run` in `docs/superpowers/notes/asl-reference/asl_ref.sh`
(pinned build md5 `61e672562465725a8c102288a7da9098`), invocation `-xx -n -q -A -L -U -i .`
(skdisasm's own flags minus `-E`), then that directory's `p2bin -p=0`. Bytes are quoted
only from runs that exited 0 with `ASL_DIAG=complete`; a refusal probe quotes asl's
diagnostic and nothing else. Every accepted probe uses several changed characters with
non-identity targets, so the plausible readings emit different bytes. Probe sources are
in `2026-09-25-s3k-codepage/probes/`, runner `probe.sh`, full transcript
`probes-final.log` (sigil `79e30fa6`, md5 `6041728b9aaddbd3ecf04f5b05d2a641`).

### What asl does

| rule | probe(s) |
|---|---|
| The default page is `STANDARD`, spelled exactly: `codepage standard` makes a new page | cp01, cp17 |
| A NEW page with no base is a copy of the page selected at that moment (not the identity, not `STANDARD`) | cp01, cp05 |
| A NEW page with a base is a copy of the base | cp02 |
| Reselecting an EXISTING page brings back its own contents; a base given then copies nothing | cp01, cp09 |
| ...but the base is still looked up: an unknown base is `#1610 unknown codepage` even on an existing page | cp28 |
| `charset` edits, and the bare `charset` reset, touch the selected page only | cp01, cp06 |
| Character constants and strings in expressions translate through the selected page | cp08 |
| Names are case sensitive (`pg1`, `Pg1` are new pages) | cp03 |
| Names are not symbols (`PG1 equ 5` coexists) and are plain text (`.loc` is one page under any label); `A.B`, `_x`, `.loc`, `d0` are all names | cp18, cp23, cp24, cp27, cp29 |
| `save`/`restore` bracket the SELECTION, nested; the contents are not bracketed (the existing `charset` finding) | cp04, cp19 |
| The selection leaks out of a macro body; a macro argument translates where it is used | cp07, cp30 |
| Every pass starts on `STANDARD` with no other page; both probes are `2 passes` | cp10, cp11 |
| A label on a `codepage` line is an ordinary address label | cp20 |
| Refusals: 0 or 3 operands `#1110`; `"PG1"`, `1`, `PG1+1`, `$$x` `#1020 invalid symbol name`; empty base, unknown base, `PG1,PG1` for a new PG1 `#1610 unknown codepage` | cp12 to cp16, cp21, cp22, cp25, cp26, cp28 |

### The probe table (input, asl bytes, sigil bytes)

sigil at `79e30fa6`. Every accepted probe matches asl byte for byte.

| probe | asl (exit 0) | sigil | |
|---|---|---|---|
| cp01 | `114233114233112233114233112233` | `114233114233112233114233112233` | same |
| cp02 | `112243114243` | `112243114243` | same |
| cp03 | `414241424142` | `414241424142` | same |
| cp04 | `11221142` | `11221142` | same |
| cp05 | `112243112243114243112233` | `112243112243114243112233` | same |
| cp06 | `112241421142` | `112241421142` | same |
| cp07 | `11421122` | `11421122` | same |
| cp08 | `1100303c0022323c112200124100303c0042` | `1100303c0022323c112200124100303c0042` | same |
| cp09 | `112243` | `112243` | same |
| cp10 | `4142414200064122` | `4142414200064122` | same |
| cp11 | `114200041122` | `114200041122` | same |
| cp17 | `11221142` | `11221142` | same |
| cp18 | `1122051142` | `1122051142` | same |
| cp19 | `112233112243114243` | `112233112243114243` | same |
| cp20 | `11220000` | `11220000` | same |
| cp23 | `11221122` | `11221122` | same |
| cp24 | `1142` | `1142` | same |
| cp27 | `1122` | `1122` | same |
| cp29 | `11221142` | `11221142` | same |
| cp30 | `111122414142414142` | `111122414142414142` | same |
| s3klevsel | 257 bytes, CRC32 `4b984279` | 257 bytes, CRC32 `4b984279` | same |

| refusal probe | asl (exit 2) | sigil (exit 1) |
|---|---|---|
| cp12 `codepage` | `#1110 wrong number of operands` | `codepage takes a page name and an optional base page, not 0 operands` |
| cp13 `codepage PG1,STANDARD,PG2` | `#1110` | `... not 3 operands` |
| cp14 `codepage PG2,NOPE` | `#1610 unknown codepage` | ``codepage base `NOPE` is an unknown codepage: no page of that name exists`` |
| cp15 `codepage "PG1"` | `#1020 invalid symbol name` | `codepage name is not a valid symbol name (asl: invalid symbol name)` |
| cp16 `codepage 1` | `#1020` | same as cp15 |
| cp21 `codepage PG1+1` | `#1020` | same as cp15 |
| cp22 `codepage PG1,` | `#1610` | `codepage base is empty, which names no page (asl: unknown codepage)` |
| cp25 `codepage $$x` | `#1020` | ``` `$` with no hex digits ``` (the lexer: the `$$name` class, not this mechanism) |
| cp26 `codepage PG1,PG1` | `#1610` | ``codepage base `PG1` is an unknown codepage ...`` |
| cp28 `codepage STANDARD,NOPE` (existing) | `#1610` | ``codepage base `NOPE` is an unknown codepage ...`` |

### The design

`AsmState.charset` stays the one field every consumer reads; it is now the SELECTED
page. `AsmState` adds the selected page's name and a list of the pages not selected;
`select_code_page(name, base)` swaps pages in and out and carries every rule above, and
`Saved` carries the page name so `restore` reselects it. No consumer changed, so no
consumer can have been missed. The per-pass reset is free for the same reason the
`charset` reset is: `Asm` rebuilds `AsmState` every pass. The directive
(`eval.rs::directive_codepage`) is operand shape only. A name is one identifier token
spelled with letters, digits, `_` and `.`, not starting with a digit; anything else is
refused by name, so nothing is accepted and ignored.

### Acceptance (a): the S3K run

Same tree, wrapper and arguments as step 0, sigil `79e30fa6` md5
`6041728b9aaddbd3ecf04f5b05d2a641`. `setdiff-after-vs-base.txt`:

| class | base | after | left | entered |
|---|---:|---:|---:|---:|
| `$$name` | 93 | 93 | 0 | 0 |
| `codepage` | 19 | 0 | 19 | 0 |
| `abcd`/`subx` | 6 | 6 | 0 | 0 |
| `(d8,PC,Xn)` | 2 | 2 | 0 | 0 |
| **total** | **120** | **101** | **19** | **0** |

All 19 codepage rows left, nothing entered, no class rose. The run still stops at the
front end on the other three classes, so what the level-select text emits inside the
whole ROM is not measurable yet; acceptance (b) measures it on an extract.

### Acceptance (b): the level-select bytes

`mk_s3kprobe.sh` builds `probes/s3klevsel.asm` from VERBATIM skdisasm `2fcd861c` text:
`levselstr` and the `LEVELSELECT` block (`sonic3k.macros.asm` 130-150), `make_art_tile`
(114), the character-constant lines of the plane-map code (`sonic3k.asm` 9950-9995 with
branches and loads filtered out, `save`/`codepage LEVELSELECT`/`restore` kept), and
`LevelSelectText` (10552-10569, all 17 `levselstr` calls). Two `dc.b "*AZaz09:. "`
STANDARD-page controls bracket the menu; they read as ASCII in asl's image, which shows
both `restore`s reselected `STANDARD`. Three equates and `planeLocH28` stand in for
definitions elsewhere in the source and touch no character.

`compare.py`, whole image: asl and sigil both 257 bytes, CRC32 `4b984279`, 0 differing
bytes. **Control:** three bytes planted into a copy of sigil's image at offsets 0, 128
and 256; the comparer reported exactly `[0, 128, 256]`, and the planted copy differs
from asl's in exactly 3 bytes. The same probe on the base sigil drew `codepage is not a
recognized 68000 mnemonic` refusals, so the match is not vacuous.

One property the S3K source cannot see: it creates `LEVELSELECT` while `STANDARD` is the
identity, so "a new page copies the selected page" and "a new page starts as the
identity" emit identical S3K bytes. Mutation M1 below shows it: the S3K test stays green
under it while 15 synthetic tests go red. The rule is pinned by cp01/cp05, not by the
corpus.

### Acceptance (c): aeon

`AEON_DIR=/home/volence/sonic_hacks/.aeon-s3k-codepage`, provisioned by
`scripts/provision-aeon-ref.sh` at aeon `ec640bcf70e263167223a33d987d4661ff221b7e` with
`REF_BUILD_DEMO=1`: `REBUILD CONTROL s4.bin 91c46c94/820209 MATCHES THE GOLDEN`, same for
`s4.debug.bin 8a378de6/846509`, and `repin --check` printed `pins.rs unchanged`.

`four_shapes.sh` built all four shapes twice in that tree, `SIGIL_BUILD`/`SIGIL_EMIT`
from each target:

| shape | before (base `e8efbec8`, md5 `c1bc9407...`) | after (`79e30fa6`, md5 `6041728b...`) |
|---|---|---|
| `sonic4` | `91c46c94` / 820,209 | `91c46c94` / 820,209 |
| `sonic4` DEBUG | `8a378de6` / 846,509 | `8a378de6` / 846,509 |
| `demo` | `1c7a34d3` / 96,863 | `1c7a34d3` / 96,863 |
| `demo` DEBUG | `72e405a5` / 103,185 | `72e405a5` / 103,185 |

Each pair compared whole with `compare.py`: 0 differing bytes, each with its own
three-byte planted control reported exactly. The base binary reports
`revision-unknown` because it was built from a `git archive` of `e8efbec8` (tar md5
`fde4aa50cb5196fe0ffb5cdde17d2050`), which is its provenance.

**Reachability, and what the identity therefore attests.** Aeon's AS-frontend input is
three `.asm` files (`engine/debug/debugger.asm`, `games/demo/game_root.asm`,
`games/sonic4/game_root.asm`); after the builds a `find` over every `.asm`/`.inc`/`.s`
in the tree still finds only those three. A grep for `codepage` or `charset` outside a
comment finds 0 of them (and 0 `.emp` files). The same grep on skdisasm's `*.asm` finds
3 files, so the grep can fire. **Aeon never reaches `codepage` or `charset`, so the
four-shape identity attests only that nothing else moved, not that the new code is
right.** That is (a) and (b)'s job.

### Tests, and the proof they can fail

`crates/sigil-frontend-as/tests/as_codepage.rs`, 23 tests, run by `cargo test
--workspace` (an integration test of `sigil-frontend-as`). Every expected byte string is
asl's image hex, emitted from the probe files and asl's image by `gen_tests.py`, never
computed from sigil. Plus a struct-level unit test,
`state::tests::save_and_restore_bracket_the_selection_not_the_contents`.

Red before the implementation: all 23 failed at base `e8efbec8` (`codepage` is not a
recognized mnemonic). The refusal tests were red too, because their needles name this
mechanism (`operand`, `unknown`, `name`) and the mnemonic refusal names none of them.

Red on mutations of the SUBJECT, each applied by `mutate.py`, shown on disk by `git
diff`, run, then restored by `git show HEAD:<file> > <file>` with an empty diff stat
after every one (`mutrun.sh`):

| id | mutation (on disk) | red |
|---|---|---|
| M1 | new page from `CodePage::identity` instead of a copy of the selected page | 15 of 23 plus the unit test; cp05 got `414243412243114243414233`, want `112243112243114243112233`. The S3K test stays green (above) |
| M2 | `restore` does not reselect (`let _ = &s.page;`) | 4 of 23 including the S3K test, plus the unit test; cp19 got `112233112233112233` |
| M3 | a base on an existing page is copied again | `a_base_on_an_existing_page_copies_nothing`: got `114233`, want `112243` |
| M4 | the base looked up only for a new page | `an_unknown_base_is_refused`: `expected a refusal, got bytes: [11, 42]` |
| M5 | accept and ignore (`if !rest.is_empty() { return; }`) | 19 of 23 including the S3K test |
| M6 | names upper-cased | `page_names_are_case_sensitive`, `the_default_page_is_named_standard_exactly` |
| M7 | an integer token accepted as a name | `a_name_that_is_not_a_symbol_name_is_refused`: `expected a refusal, got bytes: [11, 42]` |

### Full suite

`cargo test --release --workspace --no-fail-fast` at `79e30fa6` with this parcel's
`CARGO_TARGET_DIR` and `AEON_DIR`: 496 test-result lines, **5,625 passed, 1 failed, 2
ignored**. `as_codepage` ran there (23 passed) and so did the new unit test. The one
failure is `sigil-harness --test m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`,
which stops on `NO REFERENCE TREE IS NAMED` (no `ORACLE_DIR` was set; it declines to
derive `oracle-old`). The same test fails the same way at base `e8efbec8` (archive
build, same `AEON_DIR`), so it is environmental and pre-existing. `cargo clippy --release
--workspace --all-targets -- -D warnings`: exit 0.

### Instruments, step 1

| Instrument | Identity |
|---|---|
| sigil (after) | `79e30fa682950760f665d5836f9814c6dadceb13`, md5 `6041728b9aaddbd3ecf04f5b05d2a641`, tree clean |
| sigil (before, aeon) | `git archive e8efbec8`, md5 `c1bc94078252e27a9bad9dbf3e5beda3` |
| asl, p2bin | `s1disasm/build_tools/Linux-x86_64/`, asl md5 `61e672562465725a8c102288a7da9098` |
| aeon | `ec640bcf70e263167223a33d987d4661ff221b7e` |
