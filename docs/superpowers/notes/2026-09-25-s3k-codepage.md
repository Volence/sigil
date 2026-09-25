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
