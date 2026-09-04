# 2026-09-04 — the re-freeze at aeon master, and four figures that did not reproduce

Chain 202 named aeon `5875e60e`, the tip of `parcel/rom-relayout-more-room`. That was the
only honest value at the time — the goldens could be built in no other tree — but it left
the chain pointing at a branch. aeon landed their half. Chain 203 names master.

## Identity

| | |
|---|---|
| sigil branch | `parcel/relayout-refreeze-at-aeon-master`, based on master `bf20b848` |
| aeon revision frozen against | `483b3e128ec4b9efc77a9d2e8a8c7679e961cea8` (master; a merge whose second parent IS `5875e60e`) |
| durability | `git ls-remote origin` at measurement time: `483b3e12…` was `refs/heads/master`. It has since advanced to `f93cb27a…`, from which `483b3e12` remains reachable — measured, not assumed |
| reference tree | `/home/volence/sonic_hacks/.aeon-ref-relayout-master`, clean detached worktree at `483b3e12`, all four shapes BUILT (`REF_BUILD_DEMO=1`), one shape per invocation |
| assembler | built from this branch into `/home/volence/sonic_hacks/.sigil-refreeze-master-target`; `closure-revision 72a7a3554b48154ae132e10731d396988b17afa3`, md5 `de66b9a1096fd30d21295ee015eeb58d` |
| shared `target/release/sigil` | md5 `58db359428e9b38e633836313bf40487` — READ as a control, never relinked; unchanged throughout |

## THE FOUR SHAPES, and the assertion that they are builds

Every artifact below post-dates the run that made it, and each `.bin` sits within
milliseconds of a `.lst` of the same stem. A copied golden has no listing to pair with.

| shape | CRC32 / size | reference-tree mtime (UTC) | paired listing |
|---|---|---|---|
| `s4.bin` | `1c09fbfc` / 819131 | `18:07:01.661` | `s4.lst` `18:07:01.658` |
| `s4.debug.bin` | `e2144057` / 840324 | `18:09:06.913` | `s4.debug.lst` `18:09:06.900` |
| `demo.bin` | `11ebd7ab` / 96602 | `17:58:42.775` | `demo.lst` `17:58:42.772` |
| `demo.debug.bin` | `9b0d2ce7` / 102818 | `18:00:46.639` | `demo.debug.lst` `18:00:46.638` |

The freeze started `17:52:31Z`; all eight post-date it. The three off-canonical goldens the
freeze also re-took: `config_a` `213eee40`/840676, `config_b` `7ad605fc`/617819, `lean`
`3fd246f7`/773120. The committed goldens carry the same CRC32+size as the reference tree's
builds, shape for shape.

## THE FOUR FIGURES THAT DID NOT REPRODUCE

The engine lane reported, for this same revision and (they said) this same assembler:

```
s4.bin 31813fd8 · s4.debug.bin e9b52aa3 · demo.bin 22b62847 · demo.debug 3029fd89
```

**Not one of the four matches what a clean tree at `483b3e12` produces here.** No sizes
were given with those CRCs, so they cannot be checked against the standard the chain uses
(identity is CRC32 + size, never one alone).

**The assembler is NOT the variable, and that is measured rather than argued.** A control
build of the plain shape, in the same clean reference tree, driven by the shared pinned
binary the engine lane names — md5 `58db3594…`, `--version` revision `756c7efd` — produced
`1c09fbfc`/819131, byte-identical to the tree-built one. The two binaries report the SAME
`closure-revision 72a7a355`, and this branch's `golden/offcanonical_sizes/*` (which travel
inside the assembler and decide placement) were byte-identical to master's before the
freeze. Two independent reasons they cannot differ in output, and then the direct
experiment.

What is left is the tree the other figures were taken in. Not proven, but the shape is
visible from here: while this parcel ran, `/home/volence/sonic_hacks/aeon` — the engine
lane's LIVE main checkout — was executing `./build.sh` with seven dirty paths, among them a
177-line modification to `games/sonic4/test/ojz_scroll_test.emp` and an untracked
`games/sonic4/data/generated/bganim_vprobe_banks.bin` that file `embed()`s. Neither exists
at `483b3e12`. That accounts for a DEBUG-shape difference by construction; it does not by
itself account for the plain shape, so the cause is stated as unresolved rather than
narrated.

## (a) THE REACHABILITY CHECKER CLEARS — verified, not assumed

At chain 202's attest, `rev_reachability` reported `aeon_rev 5875e60e` as **DIVERGENT —
PERMANENT**. The verdict was wrong: `GitRevOracle::at` asks `origin/master` and nothing
else, and the revision was alive on `refs/remotes/origin/parcel/rom-relayout-more-room`.

Run here BEFORE the append, against the chain as committed:

```
aeon: 66 revision(s) vs origin/master 483b3e12… — 66 reachable, 0 OBJECT ABSENT,
      0 AHEAD OF REMOTE, 0 DIVERGENT, 0 COULD NOT MEASURE
```

and AFTER it, `67 revision(s) … 67 reachable, 0 DIVERGENT`. The warning is gone. Nothing
was changed to make it go: aeon's master now contains the commit, which is precisely the
condition the checker asks about.

**It is a defect deferred, not a defect fixed.** The checker still consults one branch, so
the next freeze taken against a pushed non-master revision will produce the same false
PERMANENT. The booking stands.

**Two DIVERGENT findings remain and they are SIGIL-side**, unrelated to this parcel and
present before it: entry #181 `rebake-after-repaint` and entry #201 `corpus-pin-advance`,
both `strict.sigil_rev` (`bfbedc11…`, `47c97b35…`). `refreeze --reachability` exits 1 on
them; `refreeze --check` is unaffected and reports `OK (tip relayout-at-aeon-master, chain
len 203)`.

## (b) `032b4cff`'s RE-STAMPED GATE FIXTURES ARE NOT PINNED HERE AT ALL

The question was whether our goldens pin those three files by BYTES or by SYMBOLS. Neither.

`git grep` over every tracked file for `instashield_cut`, `loop_crossover_cut` and
`sprite_tilt_cut` returns hits in exactly two documentation files
(`docs/superpowers/notes/2026-08-30-alignment-flip-packet.md` and one lane-log row) and in
no test, no source, no manifest and no golden. `git grep "tools/fixtures"` over the whole
repo returns the same two files.

They cannot reach a golden even indirectly: in aeon at `483b3e12`, the three JSONs are
consumed by `build.sh` and by `tools/{instashield,loop_crossover,sprite_tilt}_gate.py` plus
their unit tests — a grep of every `*.emp` and `*.inc` for those names returns nothing, so
no assembled byte depends on them. The only way a stale fixture reaches this lane is as a
build FAILURE in the reference tree, which is loud and is not a golden diff. Our four
shapes built clean.

## (c) THE PREDICTION HELD IN FORM AND WAS WRONG ABOUT THE REMEDY

Recorded before the measurement: every previous `Editor*` addition to
`effects_scenes.emp` needed a new `[[symbol]]` row in `repin.toml` or
`act_descriptor_port`'s link assert failed; `EditorReelBindings_*` carries two `extern()`s,
so expect a fourth row.

`act_descriptor_port` did go red, in exactly the predicted shape — *"link assertion
condition references symbol(s) … not defined in this link"*. **`repin.toml` needed no new
row and was not touched.** The 2026-09-04 sweep retired that treadmill for this family:
adding `EditorReel` to `act_descriptor_port`'s `GENERATED` prefix list supplies both
`EditorReels_*` and `EditorReelBindings_*` from the listing, and every future generated
member arrives with no edit at all.

The prediction also missed the population. The DEBUG shape failed on
`EditorReelBindings_OJZ_Act1` as predicted; the PLAIN shape failed on something else
entirely, and failed on it alone:

```
link assertion condition references symbol(s)
  `__align$games.sonic4.ojz_effects_editor_act1$0` not defined in this link
```

That is the compiler's own alignment-pad label, minted by an `align 2` the generator
emitted for the first time at this revision, carrying a `[layout.align]` congruence assert.
No `[[symbol]]` row would have covered it and no `Editor*` prefix matches it — see class 3
below for what does. `ojz_act1_sec_patched` (aeon `6248bf12`) needed nothing: it is defined
and used only inside `effects_scenes.emp`, with no cross-module consumer.

## The catch-up, and its three classes

The 138-commit reference jump brought 11 red tests across 6 binaries on the first landing
run. None was a golden divergence — `pins_rs_is_current`, `native_full_rom` and every byte
gate were green throughout.

**Class 1 — the `Sec` record shrank, `$42` → `$22`.** Nine fields and three pads left
`engine/structs.emp`; every per-channel resource arrives through `sec_effects`'s
EffectsPreset. Four gates read that layout (`act_fixture_drift`, `structs_module`'s
harvest spot-checks, `test_support::act_sec_field_equs`, and the harvest-vs-supply name
check). Re-derived field for field from the live declaration.

**Class 2 — two new cross-seam refs, the port-flip rule twice.** `player_common.emp`
publishes `Level_Width`/`Level_Height`; the p1 scope now sweeps the `Level_` family from
the listing RESTRICTED TO WORK RAM, so `Level_LoadArt` (a ROM proc sharing the prefix) is
excluded by address rather than by a name list. `act_descriptor_port` gains the
`EditorReel` prefix.

**Class 3 — an alignment pad's congruence assert.** The `[layout.align]` assert names a
symbol of a section this standalone scope does not link, so the fold poisons and the gate
reports an unresolved symbol rather than a failed invariant. It cannot go through the AS
equ seam the other cross-seam labels use — that seam emits `name = $HEX`, and a name
carrying `$` makes the AS lexer read a hex literal with no digits (`` `$` with no hex
digits ``, measured). It is supplied as a link STUB at the pad's real listing address, so
the congruence is evaluated rather than waived, and the prefix form catches a `$1` pad
whenever the generator emits one.

## THE HAND-TYPED BASELINE THE FREEZE MOVED, and where its delta comes from

`DEBUG_ASSEMBLED_LEN` `0xC052C → 0xC055E`, `+0x32`. DERIVED from the two listings rather
than back-read from the total: walking the 2,963 symbols common to `.aeon-relayout-freeze`'s
`s4.debug.lst` (`5875e60e`) and this tree's, in address order, the delta transitions are

| at | old → new | delta |
|---|---|---|
| `Player_LevelBound` | `01068A → 010692` | `+0x8` |
| `EditorRaster_OJZ_Act1_aurora_ramp_witness` | `013FC6 → 013FE0` | `+0x1A` |
| `OJZ_Reels_Fill$advance` | `014762 → 014792` | `+0x30` |
| `OJZ_Sec0_Blocks` | `017AC2 → 0179D2` | `-0xF0` |
| `OJZ_Sec5_LocalMap` | `0238C8 → 02370E` | `-0x1BA` |
| `Dac_Temp_Blip` | `0A8000 → 0A8000` | `+0` — the bank anchor absorbs all of the above |
| `Debug_PresetReadout_Show$verdict_live` | `0BEDA4 → 0BEDB6` | `+0x12` |
| `OJZ_SectionMarkerColors` | `0BEE8A → 0BEEBC` | `+0x32` → carries to `EndOfRom` |

Both post-anchor points are inside `games/sonic4/test/ojz_scroll_test.emp`'s DEBUG-only
region. `ASSEMBLED_LEN` HOLDS at `0xBDC82`, and the plain walk says why directly: its only
transitions are `+0x8` at `Player_LevelBound`, `-0x118` at `OJZ_Sec0_Blocks` and `-0x1E2`
at `OJZ_Sec5_LocalMap`, every one of them before the anchor and absorbed by it. The plain
FILE grew 8 bytes (819123 → 819131) of deb2 appendix while its assembled length did not
move at all.

## `aeon_dir_matches_the_provenance_tip`, both directions

A gate that has only been seen to pass has not been seen to work.

| AEON_DIR | aeon rev | result |
|---|---|---|
| `.aeon-ref-relayout-master` | `483b3e12` (= the tip's `aeon_rev`) | `ok`, exit 0 — and again inside the landing run |
| `.aeon-relayout-freeze` | `5875e60e` (the PREVIOUS tip's) | `FAILED`, exit 101 |

The refusal names both revisions, the entry and the remedy: *"is at aeon 5875e60e…, but the
goldens were frozen from aeon 483b3e12… (provenance tip `relayout-at-aeon-master`, entry
#203)."*

## The landing run

`scripts/landing-run.sh --aeon /home/volence/sonic_hacks/.aeon-ref-relayout-master`,
`2026-09-04T18:23:07Z → 18:27:13Z`, tree `@ a4a0ad92 (parcel/relayout-refreeze-at-aeon-master,
clean)`, reference `@ 483b3e12 (HEAD, clean) — all four present`.

```
CARGO_EXIT 0    suites 386   passed 4463   failed 0   ignored 2   skip lines 0    GREEN
```

Failures first: **zero**. `grep -c '^test .* FAILED'` on the raw log returns 0, and the
totals were re-derived independently from the 386 `test result:` lines — 4463 / 0 / 2,
agreeing exactly with the wrapper. `aeon_dir_matches_the_provenance_tip … ok` appears in
that log. No `--baseline` was passed, so the wrapper's reconciliation arm reports NOT
CHECKED; the count equals chain 202's recorded 4463, which is the expected outcome of a
catch-up that added no tests.

The first landing run (`@ dc4cc841`, before the catch-up) recorded 4452 / 11 / 2 and cargo
exit 101 — the eleven named above. Its log sits beside this one in the worktree's
gitignored `.target-land/`, so it is local evidence rather than a committed artifact; the
eleven names and their causes are transcribed here for that reason.

## What this parcel did NOT do

**No `--attest`.** The tip carries `[entry.targets.*]` and no `[entry.strict]`. Attesting
is the merge owner's step and must run after the freeze is committed to master.

**The two sigil-side orphans are untouched.** They are permanent by construction and a
re-attestation would record a different tree's run under an existing entry's name.

**`provision-aeon-ref.sh` still refuses a legitimately pushed non-master revision** — its
comment says `ls-remote`, its code says `merge-base --is-ancestor $REV origin/master`. It
did not bite this time because the revision IS on master. Booked at chain 202 and still
open.

## Trees

`/home/volence/sonic_hacks/.aeon-ref-relayout-master` (`483b3e12`, clean) is kept: it is the
tree the tip names and the only one in which these goldens reproduce.
`/home/volence/sonic_hacks/.aeon-relayout-freeze` (`5875e60e`) was read for the delta
derivation and for the negative direction of the tip gate; it was not written — same HEAD,
`git status` clean, ROM mtimes unchanged.
