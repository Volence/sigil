# AS-ORG-BACKWARDS: `org` sets the counter absolutely, and one directive had two models

Parcel note. Branch `parcel/as-org-backwards`, base master `e9a9dfa6`, tip
`ee136317` at the time this file was written (this commit moves the tip).
**LANDABLE: no aeon byte moves, on all four shipped shapes.**

## Provenance

| | |
|---|---|
| sigil base | master `e9a9dfa6`; binaries `sigil` md5 `bd32b518e9690b5d8e143887445abede`, `emit_sound_blob` md5 `4d79c9f3dfb21738c0ba7ac4b6289e60` |
| sigil fixed | this branch; `sigil` md5 `94d5c8beea0578f05e80d2d6444230d1`, `emit_sound_blob` md5 `86d696a7ab3c3275d7b730d5f6b463d7` |
| both built | `cargo build --release --bin sigil --bin emit_sound_blob`, `CARGO_TARGET_DIR` inside this worktree. The shared `sigil/target/release/sigil` was never written |
| corpora | `s1disasm` `f6ece657`, `s2disasm` `e45ebf33`, `skdisasm` `2fcd861`, each in its OWN detached worktree under `/home/volence/sonic_hacks/.corpus-orgback/`; the shared live checkouts were never written |
| preparation | `scripts/corpus-prepare.sh` wrote 8 / 74 / 100 generated files; `corpus-baseline.sh` reports `READY (4/4)`, `(39/39)`, `(50/50)` |
| baselines reproduced | **42 / 5,162 / 2,126** at the base binary, matching `2026-09-06-corpus-generated-includes.md` |
| aeon | `/home/volence/sonic_hacks/.aeon-orgback-pin`, a detached worktree at `483b3e12` (the provenance tip), and `/home/volence/sonic_hacks/.aeon-orgback` at `origin/master` `2f6f7e95`. The owner's live checkout at `/home/volence/sonic_hacks/aeon` was never written and `AEON_DIR` never pointed at it |
| oracle | `asl` md5 `61e672562465725a8c102288a7da9098` through `asl_ref.sh`'s `asl_run`. The `s2disasm` build (md5 `0dee1f98...`) was never invoked. Every value below comes from a run reported `ASL_EXIT=0` and `ASL_DIAG=complete`, except `p4_pcsym.asm`, whose whole purpose is to make asl refuse and which is quoted only for its refusals |
| emulator | none. Nothing here wants runtime confirmation |

## What asl's `org` actually is, derived

Probes in `2026-09-06-as-org-backwards-probes/`, each standalone, each stating
before it runs what the two candidate models predict.

**`org` sets the program counter absolutely, in either direction, and asl says
nothing about the direction.** `p1_back.asm` emits four bytes at `$1000`, orgs to
`$10`, and orgs to `$2000`:

```
p1_back.asm(5):  warning: P1A pc=1004h
p1_back.asm(8):  warning: P1B pc=12h low=10h start=1000h
p1_back.asm(11): warning: P1C pc=2001h hi=2000h
ASL_EXIT=0  ASL_DIAG=complete
```

`low` and `start` are both in the symbol table, `10h` and `1000h`, in one file.
That is the discriminating row: the "physical relocation, backward is illegal"
model has to produce either a refusal or a `low` in `$1000` space, and produces
neither. `p2bin` over the same `.p` writes `AA BB` at `$10` with `01 02 03 04`
still at `$1000`, so the image agrees with the symbol table.

**`save`/`restore` do not carry the program counter.** `p2_saverestore.asm`:

```
P2A pc=1004h      (before save)
P2B pc=11h        (after save, org $10, one byte)
P2C pc=11h        (after restore)
P2D pc=12h
```

`P2C` is the row that matters, and it reads the same as `P2B`. This is why the
Sonic 1 driver's tail has an explicit `!org (DACDriver+Size_of_DAC_driver_guess)`
after its `restore`: nothing else would put the counter back.

**There is no section concept in asl for `org` to be relative to.** The corpus's
two sites differ only in whether sigil happened to have a section open, and asl
answers `0` at both.

### The confounder these probes had to avoid

`p4_pcsym.asm` establishes which token IS the program counter, because the
answer is not the same under both CPUs and a probe that gets it wrong measures
that instead:

| CPU | `\{*}` | `\{$}` |
|---|---|---|
| `68000` | the PC | `#1020 invalid symbol name` |
| `z80` | `#1110 wrong number of operands` (`*` is multiplication) | the PC |

**This contradicts the previous note's claim that `*` is the PC on both CPUs in
both assemblers.** That claim is false for asl under `cpu z80`, measured here.
The previous note's own PROBE-2 was nevertheless sound: `probe_s1.py` inserts it
between `!org 0` and `CPU Z80`, so it is still under the 68000 CPU where `*` is
correct. The conclusion stands; the reason given for it does not.

## The change

`crates/sigil-frontend-as/src/eval.rs`, `directive_org`. One `if` and its
diagnostic go:

```rust
-        if target_abs < base {
-            self.err(span, "org target precedes the current phase base");
-            return;
-        }
-        let rel = target_abs - base;
-        if rel <= self.builder.extent() {
-            self.builder.seek(rel, 0, span);
+        if target_abs >= base && target_abs - base <= self.builder.extent() {
+            self.builder.seek(target_abs - base, 0, span);
         } else {
```

A target inside `[base, base + extent]` is still the in-place back-patch seek
(`parallax_section_end`). Every other target, forward or backward, closes the
section and re-bases the physical counter, so the next emit opens a `Pinned`
section there. Backward, the pin is load-bearing: a `Chained` section would be
compacted back into sequence at link time and the org would vanish.

**Both arms of the directive now hold the same model, which is the actual
repair.** The defect was never that one arm refused and the other accepted; it
was that `org` had two models and the file it appeared in chose between them.
The no-section arm was already right, and is untouched.

## The regression, and that it can fail

`org_backwards_new_section` in `crates/sigil-frontend-as/tests/snippets_golden.txt`,
run by `tests/asl_snippets.rs`. Its bytes are minted from asl by
`gen_snippet_vectors`, not written by hand:

```
        org 16
        dc.b 1,2,3,4
        org 0
        dc.b 5,6
--- bytes ---
05 06 00 00 00 00 00 00 00 00 00 00 00 00 00 00 01 02 03 04
```

The mint churned ONLY this block; the other 227 vectors and the provenance header
re-derived byte-identically, which is the non-circularity invariant that file
carries. Red-first, with the mutation shown on disk (`git stash push` of
`eval.rs` alone, `grep` confirming the refusal string was back at `:5243`):

```
thread 'snippets_match_golden' panicked at tests/asl_snippets.rs:11:53:
assemble: [Diagnostic { level: Error, message: "org target precedes the current phase base", ... }]
test result: FAILED. 0 passed; 1 failed
```

Restored with `git stash pop`, from a committed baseline.

## The aeon A/B: what each arm produced

Four shipped shapes, both arms, in a detached worktree at the provenance tip
`483b3e12`. Built with `build.sh` (one shape per invocation: plain, `DEBUG=1`,
each for `sonic4` and `demo`), the ROMs deleted before each arm.

| shape | base `bd32b518` | fixed `94d5c8be` | |
|---|---|---|---|
| `s4.bin` | `31813fd8069d80100410cac374bdced7`, 819,131 B | `31813fd8069d80100410cac374bdced7`, 819,131 B | `cmp` clean |
| `s4.debug.bin` | `e9b52aa37a7a00152764806f51bb1d43`, 840,324 B | `e9b52aa37a7a00152764806f51bb1d43`, 840,324 B | `cmp` clean |
| `demo.bin` | `22b62847dabee49a64165baa6a6dff4b`, 96,602 B | `22b62847dabee49a64165baa6a6dff4b`, 96,602 B | `cmp` clean |
| `demo.debug.bin` | `3029fd89fa8c625ba63f4397eefd637e`, 102,818 B | `3029fd89fa8c625ba63f4397eefd637e`, 102,818 B | `cmp` clean |

The same four shapes were also built at aeon `origin/master` `2f6f7e95` before
the pinned tree was found, both arms, and agreed there too (`s4.bin`
`d52dd49d113edf7131df79348a5c92f1` 820,207 B; `s4.debug.bin`
`8a76f1fc7fb9939bd1f0cf98e0811583` 846,388 B; `demo.bin`
`117c3fa8bb9131ecdd7cfa2a76c917af` 96,827 B; `demo.debug.bin`
`ae7a91dea5014dde812082aafc5603f7` 103,044 B).

### Why the arms agree, measured rather than argued

**`directive_org` is never reached in an aeon build.** A third binary,
identical to the fixed one plus an `eprintln!` at the top of `directive_org`
(`sigil` md5 `cb310003732dccd96e3e0a50baa6dae7`), was used to build all four
shapes: **zero** `ORGFEED` lines in any of them. sigil's stderr does reach the
build log, proved by the `warning:` lines that sit beside the count, and the
instrument prints normally when handed a file that has an `org`.

The static side agrees: aeon at `2f6f7e95` holds **3** `.asm` files and **0**
`.inc` files (`games/demo/game_root.asm`, `games/sonic4/game_root.asm`,
`engine/debug/debugger.asm`), 198 `.emp`, and no `org`/`phase`/`dephase`/`save`/
`restore` in directive position in any of them. **The dispatch's figure of 378
`.asm` files is stale for this revision**, and `engine.inc`, which
`directive_equate`'s comment cited as a real aeon `org 0` site, no longer exists.

### And that the A/B could have come out the other way

An A/B whose arms agree is worth nothing until the instrument is shown to move.
A one-byte mutation to `apply_header_checksum` (`rom[0x18F] ^= 1`, in
`sigil-link`, nothing to do with this parcel), built and run through the same
`build.sh` invocation on the same tree, produced

```
s4.bin  52543f7bf3001eca107703ff58c2043b  819131 bytes
```

against the A/B's `31813fd8...` at the same length. So the comparison detects a
single-byte sigil-side change in exactly this pipeline. The mutation was reverted
from the committed baseline before anything else ran.

## Corpus after-counts, measured

Both binaries over the same three prepared trees. Feed counts are `org`
directives EVALUATED across the whole run, from the instrumented binary, so each
count has the size of its own input beside it.

| corpus | before | after | org feed (no section / seek / leaves section, of which backward) |
|---|---|---|---|
| s1disasm `f6ece657` | 42 | **50** | 327 (3 / 309 / 15, of which **3 backward**) |
| s2disasm `e45ebf33` | 5,162 | **5,162** | 3,431 (10 / 3,070 / 351, of which **0 backward**) |
| skdisasm `2fcd861` | 2,126 | **2,448** | 24 (12 / 0 / 12, of which **4 backward**) |

Populations: every count is the full non-empty stderr stream, every line parsed
as `file(line): level:`.

**s1disasm, +8.** Both z80 rows go (`org target precedes the current phase base`
and the bogus `It currently takes 73DFDh bytes` fatal). Ten rows arrive, all in
`sound/_smps2asm_inc.asm`, which the fatal used to abandon: 6 `case needs a string
literal`, 2 `switch needs a string expression`, 2 `unresolved if condition`.

**s2disasm, unchanged, and the feed row says why.** Its `!org 0` is one of the 10
no-section evaluations, and **no** s2 org is a backward-with-section-open. Zero
lines differ in either direction. This is the arm the dispatch called the silent
one, and at the front end it was already asl's answer: `$` becomes 0, exactly as
asl makes it 0, with no diagnostic on either side.

**skdisasm, +322,** which is coverage arriving. `+897 unexpected character` as
roughly 5,000 lines of Z80 driver become reachable for the first time;
`-467 unresolved long expression`, `-80 unresolved symbol` and `-38 float` as the
pass loop stops aborting at line 345 and the symbols those rows wanted get
defined. The `rst`-alignment fatal and the org row both go. 38 symbol names left
the unresolved set and none joined it.

## Suite

`scripts/landing-run.sh --baseline 4177 --aeon <pinned tree>`, run TWICE against
the same reference tree, once at this branch and once with `eval.rs` and the
golden file reverted to `e9a9dfa6` on disk:

| | passed | failed | ignored | clippy | cargo |
|---|---|---|---|---|---|
| base `e9a9dfa6` content | 4,704 | **0** | 2 | 0 | 0 |
| this branch | 4,704 | **0** | 2 | 0 | 0 |

Identical, so the parcel moves no test in either direction. The `4177` figure is
an older overseer-log number and is NOT a verified baseline for `e9a9dfa6`; the
load-bearing comparison is the pair of runs above, which share everything but the
two reverted files.

**A first landing run against aeon `origin/master` `2f6f7e95` came back with 150
failures, and they were provisioning, not this parcel.**
`aeon_dir_matches_the_provenance_tip` named the cause in its own message: the
goldens are frozen from aeon `483b3e12`, entry #203 `relayout-at-aeon-master`,
and every `*_region_matches_reference` row downstream was comparing against a
different revision than the record describes. The dispatch's instruction to use
`origin/master` is wrong for a landing run; it is fine for the A/B, where both
arms see the same tree, and that A/B was run at both revisions.

## Where sigil and asl still differ, deliberately

`p5_overlap.asm` orgs backward onto ground a CLOSED region already owns. asl
accepts it and `p2bin` resolves it last-write-wins:

```
00001000: 0102 e0e1 e2e3 0708      (the four E-bytes overwrote 03 04 05 06)
```

sigil refuses, at link, by name:

```
sections `sec4096` [0x1000, 0x1008) and `sec4098` [0x1002, 0x1006) overlap in the image (colliding pins)
```

That is `relax`'s R7p.4 `overlap_diag`, and it is not this directive's doing. It
is the difference between a flat single-image linker and AS's chunked `.p` file,
and it is why `directive_org` must NOT pre-judge a backward target: the target is
just as likely to be untouched ground, which is the case `p1_back.asm` covers and
where sigil now emits an image byte-identical to asl's
(`b979d01428cfdc48ed0d84e7c5c95d05`, 8,193 bytes, both tools).

**It also means a byte-identical end-to-end s1disasm build needs more than this
parcel.** `build.lua` runs `p2bin -p=FF -z=0,kosinski,Size_of_DAC_driver_guess,after`:
the `-z` takes the chunk at address 0 OUT of the image, Kosinski-compresses it,
and pastes it after a named symbol. sigil has no counterpart, so the driver's
section at LMA 0 collides with the vector table rather than being extracted. That
is a linker-stage gap, sized and named here, and it is in the gap ledger.

## The `.emp` twin: a finding, not fixed here

**It does not share this defect, and the reason is that it has no counterpart
construct.** There is no `org` in the `.emp` frontend: `lower/mod.rs` always
opens a section with `switch_section_lma(&sec.name, cpu, vma, next_lma)`, so the
LMA is the monotone chain `next_lma` and can never be handed a lower target. An
explicit `vma:` is a VMA pin only (R7p.5) and does not move the physical counter.
So there is nothing there to seek backward, and a parcel to "fix the twin" would
have no site to work on. That is a fact about the two designs, not twin
agreement, and it is the reason it is safe to say so from a read rather than a
run.

## What could not be run, named

* **No emulator.** Nothing here wants runtime confirmation and none was
  attempted.
* **The landing run at aeon `origin/master` is reported as a refusal, not as a
  result.** It ran and came back red for the provisioning reason above; the
  verdict quoted in this note is the one at the pinned tree.
* **No end-to-end corpus ROM.** Every corpus run still ends with errors, so
  nothing links, and no claim here rests on a corpus ROM's bytes.
* **The `4177` suite baseline is unverified for `e9a9dfa6`.** It was taken from
  an overseer-log entry, and the base/branch pair above is what the "no test
  moved" claim rests on instead.
