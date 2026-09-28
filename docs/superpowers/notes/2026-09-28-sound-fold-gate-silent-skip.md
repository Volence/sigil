# SOUND-FOLD-GATE-SILENT-SKIP: the fold gate accounts for every folded head

Sigil branch `worktree-agent-a6b75fff08bbc25ec`, off master `254cf3a6`. Reference tree
`.aeon-sigil-ref` (detached `ec640bcf`). No emulator was used.

## In plain words

seam-2 writes absolute pointers into the sound banks against the base it predicts for
each bank. The fold gate (`validate_sound_fold`) then checks that the chainer placed each
bank where seam-2 predicted. It looked each bank up by its head label, and when a label
was not in the layout it skipped that bank and said nothing. A layout placing neither
head passed with zero comparisons.

It now refuses instead. Every head seam-2 predicts under must be placed, and must be the
first item of its section; a sound-off shape must place none of them. Each case has its
own diagnostic, and the gate returns how many heads it actually compared.

End to end, nothing was silently wrong today: a head renamed in source is refused first
by `[map.order-orphan]` (map row left alone) or by seam-2's own "map.toml `order` is
missing" check (map row renamed too), and an item planted ahead of `Song_MovingTrucks` is
refused by `[layout.undeclared-alignment]`. The gate relied on those; it no longer does.
No ROM byte changes.

## For aeon: what now refuses

The sound-on build refuses if `Song_MovingTrucks` or `Sfx_33` is not placed, or is
placed but is not the first label of its section (a label tied with it at offset 0 and
listed ahead of it counts as ahead). After the song-bank-2 change moves the MT bank into
`mt_bank.emp`, keep `data Song_MovingTrucks` the first item of `section mt_bank`, and put
nothing ahead of it, not even an item that is `Data.empty` in some shape. Renaming either
head means changing sigil's `seam2::MT_BANK_HEAD` / `SFX_BANK_HEAD` in the same landing.
A sound-off shape (demo, config_b) that places either label is refused as well. The
diagnostics are:

- `[sound.fold-head-absent]`: a sound-on layout does not place the head.
- `[sound.fold-head-not-first]`: the head is placed but another label heads its section.
- `[sound.fold-vs-placement]`: unchanged; the placed base differs from the folded one.
- `[sound.fold-head-sound-off]`: a sound-off shape places a head.

## Stage 0

Pre-change `crates/sigil-harness/src/native.rs` (at `254cf3a6`):

- 3333-3334 looped over two hand-typed pairs, `("Song_MovingTrucks", mt_bank_lma)` and
  `("Sfx_33", sfx_predicted)`.
- 3336: `let Some(actual) = placed(label) else { continue }; // not in this shape`.
  Confirmed for both labels. The only shape filter was `profile.sound_on` at 3312, so
  "not in this shape" never applied to a sound-on shape; the skip was pure silence.
- 3319-3326: the comparison used the label's address, `sec.lma + l.offset`. A head sitting
  behind another item read as `[sound.fold-vs-placement]` with a delta (loud, but naming
  the wrong cause), and passed silently when that offset happened to equal the alignment
  slack. A head tied at offset 0 behind another label passed silently while the addresses
  agreed, even though the walk then aligns the section by the other label's declaration.

## What changed

- `seam2.rs`: `MT_BANK_HEAD`, `SFX_BANK_HEAD` and `FOLDED_BANK_HEADS` name the heads;
  `packed_chained_base` and `SOUND_BANK_ORDER` use them, and
  `SoundLayout::folded_heads(debug)` returns `(head, folded base)` for a shape. The gate
  compares exactly that list, so the expected set is seam-2's own prediction list.
- `native.rs`: `validate_sound_fold` returns the number of heads compared and delegates to
  `pub fn check_sound_fold(resolved, folded: Option<&[(&str, u32)]>)`, `None` meaning a
  sound-off shape. Which set is expected comes from `profile.sound_on`.

## Tests

`crates/sigil-harness/tests/sound_fold_heads.rs`, classified in
`scripts/nightly_source_gates.sh` `SOURCE_GATES`:

- every shipped shape passes, comparing 2 heads when sound is on and 0 when off;
- the heads and bases the gate compares are `SoundLayout`'s;
- on the real resolves, planted in memory: a renamed head, a head tied behind a planted
  label, a head behind a planted item, a moved head section, and a head planted into a
  sound-off shape are each refused under their code, naming the head;
- a head renamed in source (module and map row) on a scratch copy of the tree under
  `CARGO_TARGET_TMPDIR` fails the build naming it, in both sonic4 shapes.

Red first, each mutation restored with `git show HEAD:<path> > <path>`:

- inserting `let Some((sec, off)) = placed(label) else { continue }; // RED-FIRST MUTATION`
  ahead of the absent refusal: `renamed_head_is_refused` failed, "a layout planted for
  [sound.fold-head-absent] on `Song_MovingTrucks` passed the fold gate: "Ok(2)"". That
  `Ok(2)` also showed the count was `folded.len()`, not a tally; it is a tally now.
- `if false && head != label { // RED-FIRST MUTATION`: `head_not_first_is_refused` failed,
  the tie at offset 0 passing with `Ok(2)`.
