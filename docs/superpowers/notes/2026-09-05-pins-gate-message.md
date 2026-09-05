# PINS-GATE-MESSAGE-MISLEADS

Branch `parcel/pins-gate-message`, off master `90a3c9ea`.

Yesterday `repin_pins::pins_rs_is_current` went red with

```
src/pins.rs is STALE against the live listings (0 changed pin(s)):

run: cargo run -p sigil-harness --bin repin
```

**The gate was right.** `pins.rs` genuinely was stale, and genuinely no pin value had
moved. It reported two true things whose juxtaposition reads as a self-contradiction,
and the printed remediation was not a command that works. Both halves are fixed
without touching the comparison.

## The two comparisons, and why they differed

| | verdict | count |
| --- | --- | --- |
| came from | `strip_provenance(committed) != strip_provenance(generated)` (`tests/repin_pins.rs:72`) | `diff_pins` (`src/repin.rs:1180`) |
| sees | every line except those containing `[provenance]` (`strip_provenance`, `src/repin.rs:1147`) | `pub const` declarations only, via `const_lines` (`src/repin.rs:1167`) |
| so a comment-only change is | STALE | zero |

All three line references in the brief check out against `90a3c9ea`.

The trigger was a dash sweep that rewrote 38 string literals in `repin.rs`. Those
literals are the ones `render` writes as `pins.rs` COMMENTS, so the generator's output
drifted from the committed copy in 108 comment lines while every pin value stood still.

**The tempting fix is the wrong one, and it is now refused in code and in a test.**
Narrowing the verdict to what `diff_pins` models is the obvious way to make the count
and the verdict agree, and it would have made that real staleness invisible. Weakening
a check to stop it reporting something true is the move this repo rejects elsewhere.
The verdict stays whole-file and strict. The MESSAGE now explains itself.

## What was built

* **`repin::drift_report(committed, generated) -> Option<DriftReport>`** is THE verdict,
  asked in exactly one place. The `pins_rs_is_current` gate and the `repin` binary both
  call it, so a tool and the test that guards it cannot disagree about whether the
  committed file is current. The comparison inside it is unchanged: `strip_provenance`
  equality over the whole text.
* **`DriftReport`** partitions the WHOLE line difference into buckets a reader can tell
  apart: pin values that moved, `pub const` lines that differ without their value
  moving, and surrounding text, each in the file's own order (multiset difference, so a
  single inserted line does not report the whole tail as changed). When every bucket is
  empty and the texts still differ, the headline names the only remaining explanation
  (line ORDER) instead of printing zeroes next to the word STALE.
* **`repin::stale_pins_message`** assembles the exact panic text, which makes the
  message a testable artifact. The gate itself needs a reference tree to reach its
  panic; the new `tests/repin_gate_message.rs` is hermetic and builds both drift cases
  out of the committed `src/pins.rs`.
* **`repin::regenerate_command`** prints the command that actually works, with the paths
  the caller already knows filled in.
* The `repin` BINARY had the identical defect (`0 pin(s) changed:` followed by an empty
  list) and its `--check` path printed the same incomplete hint. Both now use the shared
  report and the shared remediation, and `delta_suffix` moved next to the report so both
  surfaces show the same deltas.

## The message, both cases, verbatim

Produced by `stale_pins_message` against the real committed `pins.rs`, with the build
directory and reference tree stood in for readability.

### A real pin change

```
src/pins.rs is STALE against the live listings.

WHAT DIFFERS: 1 pin value moved; the surrounding text is identical

pin values that moved (name: committed -> regenerated):
  ASSEMBLED_LEN: 0xBDC82 -> 0xDEAD (Δ -0xAFDD5)

TO FIX: regenerate the file. BOTH VARIABLES BELOW ARE PART OF THE COMMAND, not
optional extras. `repin` alone regenerates nothing: with no reference tree it
panics at exit 101 naming AEON_DIR and never reaches its second check, and with
a tree but no SIGIL_EMIT it exits 2 naming SIGIL_EMIT (the resolve builds the
sound-on shape) and writes nothing. From the sigil checkout whose pins.rs is
stale:

  cargo build --release -p sigil-harness --bin emit_sound_blob
  SIGIL_EMIT=/home/volence/sonic_hacks/sigil/.target-land/release/emit_sound_blob \
  AEON_DIR=/home/volence/sonic_hacks/.aeon-ref \
    cargo run --release -p sigil-harness --bin repin
```

### A comment-only drift

This is yesterday's case in miniature: one rendered comment line changed, no pin value
moved, and the file is still stale.

```
src/pins.rs is STALE against the live listings.

WHAT DIFFERS: NO pin value moved; 2 line(s) of surrounding text differ (1 committed-only, 1 regenerated-only)

Every pin value in the committed file is still the value the generator produces, and
the file is STILL stale: the verdict compares the whole rendered file minus the
`[provenance]` stamp lines, so the text the generator writes AROUND the pins counts.
That is not a false alarm and not a gate bug. The committed file is no longer what
`repin` emits, so regenerate it; do NOT narrow the comparison to pin values, which
would hide exactly this case.

surrounding text, committed side only (1 line(s)):
  //! GENERATED FILE, DO NOT EDIT BY HAND.

surrounding text, regenerated side only (1 line(s)):
  //! GENERATED FILE. Regenerate it, never hand edit it.

TO FIX: regenerate the file. BOTH VARIABLES BELOW ARE PART OF THE COMMAND, not
optional extras. `repin` alone regenerates nothing: with no reference tree it
panics at exit 101 naming AEON_DIR and never reaches its second check, and with
a tree but no SIGIL_EMIT it exits 2 naming SIGIL_EMIT (the resolve builds the
sound-on shape) and writes nothing. From the sigil checkout whose pins.rs is
stale:

  cargo build --release -p sigil-harness --bin emit_sound_blob
  SIGIL_EMIT=/home/volence/sonic_hacks/sigil/.target-land/release/emit_sound_blob \
  AEON_DIR=/home/volence/sonic_hacks/.aeon-ref \
    cargo run --release -p sigil-harness --bin repin
```

The pre-parcel message for that same case was one line: `src/pins.rs is STALE against
the live listings (0 changed pin(s)):` followed by an empty list.

## The verdict was NOT weakened

`the_whole_file_verdict_survives_a_comment_only_drift` builds a comment-only drift out
of the committed `pins.rs`, asserts that `diff_pins` finds nothing in it (so the
fixture really is the case under test, not a pin move in disguise), and asserts that
`drift_report` still says STALE. That is the exact case a narrowed verdict would lose,
and it is the only thing that stops a later session from "simplifying" the check.

Three further guards hold the verdict's shape from the other side, so it cannot be
weakened by widening either: `a_provenance_only_difference_is_not_drift` (a rebuild
that moves no pin must not red), `an_identical_render_is_current`, and
`a_reordering_is_named_rather_than_reported_as_zeroes`.

Mutation 1 below is the direct proof: replacing the whole-file comparison with
`diff_pins(...).is_empty()` reds that test by name.

## Red-first evidence

Every mutation was applied to a COMMITTED baseline (`git checkout HEAD -- <path>`
restores; `git diff HEAD --stat` is the witness, since `git checkout <rev> -- <path>`
stages and plain `git diff --stat` would report nothing on an applied mutation). Each
was also confirmed by a content grep, and the anchor assertion in each patch script
fails loudly rather than writing an unchanged file, so a patch that did not land could
not have run as a pass.

### Mutation 1: narrow the verdict to what `diff_pins` models (the tempting fix)

```
 crates/sigil-harness/src/repin.rs | 4 ++--
 1 file changed, 2 insertions(+), 2 deletions(-)

-    if a == b {
+    let pin_changes = diff_pins(committed, generated);
+    if pin_changes.is_empty() {
         return None;
     }
-    let pin_changes = diff_pins(committed, generated);
```

`test result: FAILED. 5 passed; 5 failed`

```
the_whole_file_verdict_survives_a_comment_only_drift
a_comment_only_drift_says_no_pin_value_moved_and_why
a_reformatted_declaration_is_reported_rather_than_lost
a_reordering_is_named_rather_than_reported_as_zeroes
every_drift_case_prints_the_remediation
```

The not-weakened guard reds with its own words: `a comment-only drift is STALE; a
verdict that returns None here is weakened`.

### Mutation 2: keep the strict verdict, put the MESSAGE back to the pin count

`headline()` returns `format!("{} changed pin(s)", self.pin_changes.len())` and the
pins-stood-still explanation is switched off.

```
 crates/sigil-harness/src/repin.rs | 4 +++-
 1 file changed, 3 insertions(+), 1 deletion(-)

content witness: line 1259  return format!("{} changed pin(s)", self.pin_changes.len());
                 line 1304  if false {
```

`test result: FAILED. 6 passed; 4 failed`

```
a_comment_only_drift_says_no_pin_value_moved_and_why
a_moved_pin_value_is_counted_and_named
a_reformatted_declaration_is_reported_rather_than_lost
a_reordering_is_named_rather_than_reported_as_zeroes
```

The failure output reproduces the original defect exactly, which is the point of this
mutation: `WHAT DIFFERS: 0 changed pin(s)` printed under `src/pins.rs is STALE`.

### Mutation 3: put the remediation back to the bare one-line hint

```
 crates/sigil-harness/src/repin.rs | 3 +++
 1 file changed, 3 insertions(+)

content witness: line 1521  return "run: cargo run -p sigil-harness --bin repin".to_string();
```

`test result: FAILED. 7 passed; 3 failed`

```
the_remediation_carries_both_variables_and_the_emitter_build
the_remediation_falls_back_to_named_placeholders
every_drift_case_prints_the_remediation
```

### Why these are three distinct applied patches, not one

The red SETS are different and each is the set its own mutation predicts. Mutation 2
leaves `the_whole_file_verdict_survives_a_comment_only_drift` GREEN (it does not touch
the verdict) and mutation 1 leaves the two remediation tests GREEN (it does not touch
the remediation). A patch that had landed on the wrong anchor, or not landed at all,
could not produce three different red sets from three different files' worth of change.

After restoring from HEAD: `git status --porcelain` clean, `git diff HEAD --stat`
empty, `repin_gate_message` back to `10 passed; 0 failed`.

**And the corollary, which the brief half-states.** The brief says a test that only
checks the verdict is vacuous here. The inverse is equally true and mutation 2 measures
it: a test that only checks the MESSAGE would have left the verdict guard green. Both
classes are needed and neither implies the other.

## The printed remediation, measured

Measured in all three conditions on 2026-09-05 rather than reasoned about, from this
worktree with `CARGO_TARGET_DIR` set to its own `.target-land`.

| condition | exit | what the reader sees |
| --- | --- | --- |
| neither `AEON_DIR` nor `SIGIL_EMIT` | **101** | a panic from `test_support::aeon_dir`: NO REFERENCE TREE IS NAMED. **The string `SIGIL_EMIT` does not appear in the output at all.** |
| `AEON_DIR` set, `SIGIL_EMIT` unset | **2** | `repin: set SIGIL_EMIT to <sigil>/target/release/emit_sound_blob (the resolve builds sound-on).` Nothing written. |
| both set, emitter built | **0** | two `player_climb` allotment warnings, then `pins.rs unchanged`. `git status` confirms `pins.rs` was not rewritten. |

`main` resolves the aeon tree (`src/bin/repin.rs:91`) BEFORE it checks `SIGIL_EMIT`
(line 94), which is why the second check is unreachable from a bare shell.

So the SEVERITY correction in the brief holds: every failure is loud and names something
actionable, so this was a papercut and never a silent no-op or a trap. The reader who
copies the printed command is not misled into a bad state. What they get is one missing
variable per round trip, and the first one is not the one either version of the brief
named.

## Strict run

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-ref cargo test --release
--workspace --no-fail-fast`, segmented by package after the earlier monolithic runs were
killed under load, each segment with its own end marker, and the log stamped with `pwd`,
`HEAD` and branch so it names its own tree.

**Failures first: 0 failed. 4,627 passed, 2 ignored.** No `test result: FAILED` line
anywhere in either log, and no failing test names.

| segment | passed | failed | ignored | rc |
| --- | --- | --- | --- | --- |
| `-p sigil-cli` (rerun, see below) | 685 | 0 | 1 | 0 |
| `-p sigil-harness` + `--workspace --exclude` both | 3,942 | 0 | 1 | 0 |
| **total** | **4,627** | **0** | **2** | |

Reconciles against the master bar of 4,617 passed / 0 failed: +10, which is exactly the
ten `#[test]` in the new `tests/repin_gate_message.rs`. Nothing else moved.

Log canary, because a suite log does not name its own tree: both logs are stamped with
`pwd`, `HEAD` and branch, and `repin_gate_message` is grepped for by name in the harness
log. It is there, `10 passed; 0 failed`. `repin_pins::pins_rs_is_current` is there too,
`ok`, against `AEON_DIR=/home/volence/sonic_hacks/.aeon-ref`, which is the gate this
parcel rewired and the one that was red yesterday.

**Segment 1 was rerun, and why is a finding.** The first pass of `-p sigil-cli` returned
`rc=101` on one test, `version_provenance::version_reports_the_head_of_the_tree_it_was_
built_from`: the `sigil` binary reported revision `72f464dd` while the checkout's HEAD
read `6ab45fe1`. Cause: I committed the note WHILE the run was in flight, so HEAD moved
under a suite that measures HEAD. The test names both possible causes itself and says
`re-run to distinguish`, which is what distinguished it. The rerun holds HEAD still and
stamps it before and after (`HEAD_BEFORE` and `HEAD_AFTER` both `6ab45fe1`): `rc=0`,
`685 passed; 0 failed`, and the version row green. Segments 2 and 3 both ran entirely
after that commit and so were never exposed to it.

The general form, which is a standing trap rather than a one-off: **"commit before a
long run" and "do not commit during a long run" are both true, and the second is the one
no invariant here states.** Every delta calculation holds something fixed; this suite
holds HEAD fixed, and I moved it.

## Anything in this brief you concluded was wrong

1. **The half-2 MECHANISM, which the brief had already corrected once, is still not
   right.** The brief's corrected claim is that `repin` "exits 2 and prints a hint naming
   the missing variable". Measured, that holds ONLY when `AEON_DIR` is already set. In a
   bare shell, which is exactly what a reader who copies `run: cargo run -p sigil-harness
   --bin repin` has, it exits **101** on the missing reference tree and never mentions
   `SIGIL_EMIT`. There are TWO missing variables and the one the reader hears about
   first is not the one the brief names. The severity ruling survives intact; the story
   under it does not. Both retractions of this claim have been in the direction of the
   mechanism rather than the severity, which is the pattern worth carrying forward: the
   check `what would the run have to print for my claim to be false` was applied to
   "is it silent" and never to "which check fires first".

2. **The brief located the defect at the gate's message; it was at BOTH surfaces.**
   `src/bin/repin.rs` printed `0 pin(s) changed:` followed by an empty list for the same
   input, and its `--check` path printed the same incomplete one-line hint. That matters
   more than it sounds: the binary is the surface the remediation tells the reader to run
   next, so fixing only the test message would have sent a confused reader to a second
   copy of the same contradiction. The brief's framing held the binary fixed without
   saying so, which is the shape of `an exclusion stated as a location`.

3. **A smaller one, offered so nobody re-derives it.** `strip_provenance` joins lines
   with `\n`, so it also normalizes a trailing-newline difference away. A render that
   differs from the committed file only in its final newline is NOT drift. Nothing in
   the brief claimed otherwise; it is simply a property of the comparison that is easy
   to assume the other way when reading `it compares essentially the whole file`.

4. **The confusion had already propagated, which the brief treats as a cost to one
   seat.** `docs/superpowers/notes/2026-09-05-as-if-refusal-diag-vector.md` books the
   red as a known row and then reads its own output as reassurance twice: "The `0 changed
   pin(s)` is also positive evidence in its own right: this parcel moved no pinned aeon
   bytes" (line 249) and "its own `0 changed pin(s)` output is consistent with it and
   shows this parcel moved no pins" (line 361). The conclusion drawn there is true, but
   it is drawn from a number that does not describe the failing verdict, on a run where
   the file was declared STALE. A message that has to be reasoned around is a message
   that will eventually be reasoned around wrongly. That note is another lane's and is
   not edited here.

5. **Nothing else.** The three line references (`repin_pins.rs:72`, `repin.rs:1147`,
   `repin.rs:1180`), the claim that `strip_provenance` drops only `[provenance]` lines,
   the claim that `diff_pins` builds from `const_lines` and is blind to comments, and the
   108-comment-line account of yesterday's drift all check out against `90a3c9ea`. The
   instruction to refuse the narrowing fix was correct and is now enforced by a test
   rather than by a comment.
