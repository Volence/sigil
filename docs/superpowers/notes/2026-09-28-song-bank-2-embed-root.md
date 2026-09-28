# SONG-BANK-2-EMBED-ROOT: seam-2 reads mt_bank.emp's embeds from either root

Sigil branch `worktree-agent-a39c42047c611ee73`, off master `c97924b1`. Reference tree
`.aeon-sigil-ref` (detached `ec640bcf`). Probe tree `.aeon-embedroot-probe`, a detached
copy of aeon `origin/master` at `478fa091`. No emulator was used.

## For aeon, in plain words

- **You can respell `mt_bank.emp`'s embeds relative to the repo root now.** seam-2's
  `emit_mt_bank_at` picks the base from the module's own `embed("...")` literals: if they
  name files under the aeon root, it lowers from the root (as the ROM build does); if they
  name files under `games/sonic4/data/sound`, it lowers from there (today's module). Both
  spellings emit the same bytes. Your change can land on its own after this one.
- **It will not guess.** It refuses, by name, three cases:
  - `[sound.mt-embed-ambiguous]`: one path names a file under BOTH the root and the sound
    dir (for example a stray `song_movingtrucks.bin` at the repo root while the module is
    sound-dir relative). Equal bytes in two files are still refused; the same file
    reached through a symlink is accepted.
  - `[sound.mt-embed-mixed]`: some paths are root-relative and others sound-dir relative.
    Respell all of them at once.
  - `[sound.mt-embed-computed]`: an `embed`/`import` whose path is not a plain string
    literal (a `const` holding the path, an interpolated string). Keep the paths literal.
- **What I reproduced of your change:** the embed-path rewrite only (all ten literals in
  aeon master's `mt_bank.emp` prefixed with `games/sonic4/data/sound/`), then the same
  plus deleting the nine co-residency ensures. I did NOT make `mt_bank.emp` the placed ROM
  module, delete `mt_bank_blob.emp`, or move `SongTable`/`SongPatchTable` to 68k. Every
  build below is byte-identical to the unmodified tree.
- **Your python audits read the old spelling.** With the rewrite and the lint lanes on,
  `./build.sh` fails in the pre-build pytest lane: `tools/test_smps_import.py`
  (`mt_bank.emp does not embed song_s2_ehz.bin`) and three
  `tools/test_song_packer.py::TestCommittedSongHeaders` tests (`no embed("song_*.bin") in
  .../mt_bank.emp`). They match the literal `embed("song_*.bin")`; they need to accept
  the root-relative spelling as part of your change. The builds below ran with
  `NO_LINT=1` for that reason, and the failure is in those audits, not in sigil.
- **Deleting the co-residency ensures:** sigil's production code does not depend on them.
  Two sigil test files (`mt_negative_probes.rs`, `mt_port.rs`) read them from the
  reference tree and WILL go red at the next reference-pin advance, loudly, not
  vacuously; both already go red against your current master because of the four S2
  EHZ/CPZ ensures. The third you named, `sfx_negative_probes.rs`, reads only
  `sfx_bank.emp`'s own ensure. Details below. The three substitutes
  you named do cover the invariant, together, with one caveat on the fold gate.

## The fix

`crates/sigil-harness/src/seam2.rs`, `mt_bank_embed_root` (called from `emit_mt_bank_at`
before lowering). It lexes the module and collects every `embed`/`import` call whose
argument is a single string literal. For each literal it asks whether `<aeon>/<path>` and
`<sound_dir>/<path>` are files:

| result | action |
|---|---|
| both, different canonical files | refuse `[sound.mt-embed-ambiguous]`, naming both |
| both, one canonical file | counts for both; no vote |
| root only / sound only | a vote for that base |
| neither | no vote; the lowerer reports `[embed.not-found]` against the chosen base |

Votes for both bases: `[sound.mt-embed-mixed]`. Votes only for the sound dir: the sound dir.
Otherwise (root votes, or no votes at all): the aeon root, so a missing file is reported at
the path the ROM build would read. A non-literal path: `[sound.mt-embed-computed]`.

The decision is per path, before anything lowers. A "lower twice, keep whichever succeeds"
design was rejected: with a stray same-named file at the root and the rest of the module
sound-dir relative, the root lowering fails on the other files, the sound-dir lowering
succeeds, and the stray file's ambiguity is never seen. Per-path classification refuses it.

`include_root` is set to the chosen base and `embed_base` stays `None`, so on today's tree
the lowering options are exactly the ones used before (include root = the sound dir):
byte-neutral by construction, and measured below. On the root spelling they equal the ROM
build's (`native.rs` sets include root and embed base to the aeon root).

The existence probes (`is_file`, `canonicalize`) are metadata reads, not file reads, so
they do not enter the build's read set. Their only effect on output is success versus a
named refusal, never a different byte.

## Tests

`crates/sigil-cli/tests/seam2_mt_embed_root.rs`, runner
`cargo test -p sigil-cli --test seam2_mt_embed_root` with `AEON_DIR` and
`SIGIL_STRICT_GATE=1`. Each test works on a scratch copy of the reference tree's `engine/`
and `games/sonic4/` and respells that copy's literals from the reference's own, reading each
literal as whichever spelling it is, so the test holds before and after aeon's respelling:

- `both_spellings_emit_the_reference_bank`: a module-relative copy and a root-relative copy
  each emit the unmodified reference's bank bytes and table offsets, both shapes, and the
  same `sound_layout`.
- `a_path_naming_two_files_is_refused`: a different file and an equal-bytes copy at the
  root spelling of the first path are refused as ambiguous; a symlink to the same file is
  accepted and emits the reference bytes.
- `mixed_spellings_are_refused`: one literal respelled, both directions.
- `a_computed_embed_path_is_refused`: the first path moved into a `const`.

Green 4/4 against `.aeon-sigil-ref` and against the root-relative probe. Classified as a
source gate in `scripts/nightly_source_gates.sh` (it reads aeon source and committed `.bin`
inputs, no built ROM, listing or golden).

Red-first: with `let embed_root = mt_bank_embed_root(aeon, &dir, &src)?;` mutated on disk to
`let embed_root = dir.clone();` (the old base), 0 passed, 4 failed against `.aeon-sigil-ref`,
the root-relative case failing with
`[embed.not-found] cannot read .../games/sonic4/data/sound/games/sonic4/data/sound/song_movingtrucks.bin`
(aeon's measured refusal). With only the ambiguity branch disabled (`if !same && false`),
exactly `a_path_naming_two_files_is_refused` failed. Both restored from the committed file.

The first version of the test (step-1 commit) assumed the reference module is
module-relative and went red, 3 of 4, against the root-relative probe: it would have broken
at the pin advance after aeon lands. Step 2 fixed that.

## Probe builds (zlib CRC-32 / size)

Release `sigil` and `emit_sound_blob` built from this branch (the seam-2 source is the same
at both commits). Probe = aeon `478fa091`.

| probe state | binaries | shape | rc | `s4.bin` / `s4.debug.bin` |
|---|---|---|---|---|
| unmodified | this branch | plain | 0 | `d2c5842a` / 829,987 |
| unmodified | this branch | debug | 0 | `1000eded` / 856,982 |
| embeds root-relative | shared pair `7051ff7a` (control) | plain | 1 | `[embed.not-found] cannot read .../sound/games/sonic4/data/sound/song_movingtrucks.bin` in emit_sound_blob |
| embeds root-relative | this branch, lint lanes on | plain | 1 | seam-2 emit OK; aeon pytest audits fail (above) |
| embeds root-relative | this branch, `NO_LINT=1` | plain | 0 | `d2c5842a` / 829,987 |
| embeds root-relative | this branch, `NO_LINT=1` | debug | 0 | `1000eded` / 856,982 |
| root-relative + 9 ensures deleted | this branch, `NO_LINT=1` | plain | 0 | `d2c5842a` / 829,987 |
| root-relative + 9 ensures deleted | this branch, `NO_LINT=1` | debug | 0 | `1000eded` / 856,982 |

## Other seam-2 emit paths

None has this defect today, because none of the modules they lower is also placed by the
ROM build; each is lowered only by seam-2 and reaches the ROM through a generated `.bin`
that a `*_blob.emp` / `soundbankhead.emp` / `dac_banks.emp` embeds with root-relative
paths. So there is no second resolver to disagree with. They would get the same split if
aeon made one of them the placed module:

- `emit_dac_banks_at` and `emit_dac_body_and_head_at` lower `dac_samples.emp` with include
  root `games/sonic4/data/sound` (`embed("temp_blip.bin")`, `embed("dac/kick.pcm")`, ...).
- `emit_sfx_body_and_head_at` lowers `sfx/sfx_bank.emp` with include root
  `games/sonic4/data/sound/sfx` (`embed("sfx_33.bin")`, ...) and `sfx_blob_win_tab.emp`
  with the sound dir (no embeds).
- `emit_pitchtable_in` lowers `movingtrucks_pitchtable.emp` with the sound dir (no embeds).
- The second song bank (`games.sonic4.song_bank2`) is not seam-2 lowered; it is a ROM-build
  module and already resolves from the root.

Not fixed here: not the same defect today. If aeon moves one of them, the same
`mt_bank_embed_root` shape applies (it takes the module's own directory as a parameter).

## Deleting the nine co-residency ensures

aeon master's `mt_bank.emp` has nine `ensure(bankid(X) == bankid("MovingTrucks_Bank_Start"), ...)`
lines (`.aeon-sigil-ref` at `ec640bcf` has five; the S2 EHZ/CPZ songs added four).

### Does sigil rely on them?

Production code: no. seam-2 supplies the `MovingTrucks_Bank_Start` carrier label
(`mt_bank_carrier_asm`) and fails if any link assert fails, but counts none. With the
ensures gone the carrier label is unused and harmless.

Tests, each measured against the probe (`AEON_DIR=.aeon-embedroot-probe`,
`SIGIL_STRICT_GATE=1`, `--no-fail-fast`) in three states:

| test | reads | aeon master as is | + embeds root-relative | + ensures deleted (sound-dir embeds) |
|---|---|---|---|---|
| `mt_negative_probes.rs` `wrong_bank_cross_seam_label_fires_all_five_co_residency_ensures` | the REAL `mt_bank.emp` from `AEON_DIR` | FAIL, 9 != 5 | FAIL, `[embed.not-found]` | FAIL, 0 != 5 |
| `mt_negative_probes.rs` `straddle_doctored_map_base_...` | real module, include root = sound dir | pass | FAIL, `[embed.not-found]` | pass |
| `mt_port.rs` both region tests | real module; `guard_assert_count == 7` | FAIL, 11 != 7 | FAIL, `[embed.not-found]` | FAIL, 2 != 7 |
| `sfx_negative_probes.rs` `wrong_bank_cross_seam_label_fires_the_co_residency_ensure` | the real `sfx/sfx_bank.emp`'s OWN ensure | pass | pass | pass |
| `sfx_negative_probes.rs` `grown_mt_section_past_the_sfx_base_...` | real `mt_bank.emp`, include root = sound dir, overlap only | pass | FAIL, `[embed.not-found]` | pass |

So:

- **`mt_negative_probes.rs`** reads the ensures from the aeon tree. Deletion makes the
  wrong-bank probe fail loudly (0 fired, 5 expected), not go vacuous. It is already red
  against aeon master (9 fired). It needs to be retired or re-aimed at the substitutes
  (a wrong-window `mt_bank_lma` refused by `mt_bank_room`; a fold-gate mismatch). The
  root-relative respelling separately breaks its straddle probe and the wrong-bank probe,
  because they lower the module with include root = sound dir themselves rather than
  through `emit_mt_bank_at`; they need the same base choice.
- **`mt_port.rs`** reads the ensures from the aeon tree through its guard-count check.
  Deletion makes it fail loudly (2 counted, 7 expected); it is already red against aeon
  master (11). It needs the count re-derived (2: the two extern drift guards) or derived
  from the module, and the "co-residency ensures must all PASS" assertion goes vacuous for
  co-residency and should be dropped with it. It too lowers with include root = sound dir
  and breaks on the respelling.
- **`sfx_negative_probes.rs`** does not read `mt_bank.emp`'s ensures: its ensure probe uses
  `sfx_bank.emp`'s own `bankid(Sfx_33)` ensure, which aeon is not deleting. Deletion changes
  nothing for it. The respelling breaks probe (e), which lowers the real `mt_bank.emp` with
  include root = sound dir.

Not changed in this parcel, as asked.

### Do the three substitutes cover what the ensures checked?

The ensures checked: every stream and patch label in the `mt_bank` section is in the same
32 KB bank as the engine-table head. Together, yes:

1. **The head is bank-aligned.** `section_align.rs:136` declares `SoundTablesZ80_Head`
   requires `0x8000`, and `native::validate_resolved_alignment` (`native.rs:2538`, called
   at `native.rs:3943`) checks it against the resolved ROM layout. Without this the others
   do not add up: the window `[bank_start, bank_start + 0x8000)` would span two banks.
2. **The fold gate** (`native::validate_sound_fold`, `native.rs:3307`, called at
   `native.rs:3944`, always on for sound-on shapes) requires the ROM's placed
   `Song_MovingTrucks` to equal seam-2's predicted `mt_bank_lma`.
3. **seam-2's window-range check** (`seam2::mt_bank_room`, `seam2.rs:1764`, called at
   `seam2.rs:1530`) refuses a predicted `mt_bank_lma` outside
   `[bank_start, bank_start + 0x8000)` and sizes seam-2's `mt_bank` region to end at the
   window top.
4. **No-straddle**: the section's `bank: $8000` makes `resolve_layout` refuse a section
   crossing a bank boundary (`sigil-link/src/relax.rs:340-385`), in the ROM link once
   `mt_bank.emp` is the placed module.

(2) and (3) put the section's first byte in the head's bank; (4) keeps every later byte in
the bank of the first; (1) makes "the head's window" and "the head's bank" the same thing.

Caveat on (2): `validate_sound_fold` skips a label it cannot find
(`let Some(actual) = placed(label) else { continue }`). If `Song_MovingTrucks` is ever
renamed, or stops being the section's first item, the gate goes silent for the MT bank and
the chain above loses its link to the ROM. Keep that label, first in the section, or tell
sigil so the gate's label moves with it.

## Out of scope, listed only

- `mt_bank_port.rs` reads `mt_bank_blob.emp`, which aeon's change deletes.
- After the ensures go, `mt_bank_carrier_asm`'s `MovingTrucks_Bank_Start` label is dead
  scaffolding; kill it when `mt_bank.emp` no longer names it.
- `section_align.rs:140`'s comment names `mt_bank_blob` as `Song_MovingTrucks`'s module.
- The `mt_negative_probes`/`mt_port`/`sfx_negative_probes` tests that lower the module
  with include root = sound dir could reuse `mt_bank_embed_root` if it were made public.
