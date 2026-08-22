# The warn-tier lint set drifted — diagnosed

Status: **DIAGNOSED.** Culprit named and mechanism pinned. Two follow-ups remain, one per
repo. The hazard is latent, not live.

## The firing

`crates/sigil-cli/tests/warn_tier_corpus.rs:166`, on sigil master `a32fee7f`:

```
the warn-tier lint id set moved for `sonic4 plain`.
  NEWLY FIRING (a lint fires on the corpus and nobody decided that): ["layout.odd-field"]
  NO LONGER FIRING: []
```

Measured inside a full-suite baseline: **3695 passed / 57 failed / 4 ignored**. The other
56 failures are environmental — the aeon main checkout carries the owner's uncommitted
content edits and builds `s4.bin = c7b9d10d` against a pinned `060401e4`, so every region
diff, golden CRC and the 37 shifted pins in `pins_rs_is_current` read that noise. This one
does not belong to that family: no content blob decides a field offset.

## The verdict

Measured against the clean aeon worktree at committed tip `b1f8a230` (all four ROMs
verified against the provenance tip: `060401e4` / `0dbaa80f` / `c708b114` / `dec88cc1`),
using the existing `target/release/sigil` with `SIGIL_WARNINGS=full`:

```
engine/level/scene_dsl.emp:1006:5  [layout.odd-field] struct Scene: field sc_mask_raw (2-byte) at odd offset 119
engine/level/scene_dsl.emp:1007:5  [layout.odd-field] struct Scene: field sc_v_deform_shift_raw (2-byte) at odd offset 121
```

Two firings inside 135 total warnings (proc.clobber-undeclared 68, module.unreachable 44,
module.path-mismatch 9, proc.undeclared-fallthrough 9, import.no-names 2,
layout.odd-field 2, proc.out-unwritten 1).

## Mechanism: a hand-computed pad that a later field insertion invalidated

`scene_dsl.emp` pads deliberately, and says so:

> Pad so the two i16 bridges land on EVEN offsets (94, 96). `Scene` is comptime-only — it
> never reaches ROM, so an odd 2-byte field could not fault a 68000 today, and sigil's
> `[layout.odd-field]` is a false positive here in the strict sense. Padded anyway, for one
> cheap reason: the day anything emits a Scene, an odd word field is an address error, and
> a warning that has been explained away in a baseline is not there to catch it.

**The comment asserts even offsets 94 and 96; the compiler measures odd 119 and 121.** The
pad no longer does what it claims. Timeline, from `git log -S` in the aeon tree:

- `120180ac` 2026-08-18 15:07:31 — "fix(scene_dsl): even-align the two i16 bridges in Scene
  (layout.odd-field)". The pad was added for this exact lint, and it worked.
- `37732afe` 2026-08-18 15:06:11 — the sigil baseline adjudication, 80 seconds earlier.
  Consistent: the lint fired, aeon padded, the baseline froze around the fixed state.
- Between that fix and tip, **17 commits** touched `scene_dsl.emp`
  (`git log 120180ac..b1f8a230 -- engine/level/scene_dsl.emp`), several of them adding
  struct fields above the bridges: `022b961f` (SceneDeform gains Own()), `a1d66b51`
  (capability-shaped band records), `59e29b68` (per-layer vertical depth), `ba335e08`
  (left_column_mask), `806bed57`. The offsets moved 94 → 119, **+25 bytes**.

**What is established: the pad went stale somewhere in that 17-commit window, and no gate
could see it.** The comment's own author predicted the failure mode — "a warning that has
been explained away in a baseline is not there to catch it" — and what actually hid it was
a baseline gate not being *run*.

### RETRACTED: "T12 re-broke it"

An earlier revision named `ba335e08` (P3 Task 12) as the commit that broke the alignment.
**That attribution is not established and is withdrawn.** It came from
`git log -S 'sc_left_col_mask'`, which answers *"which commit introduced this field"* — not
*"which commit broke parity"*. Those are different objects. Evenness is a **parity**
property, so each insertion flips or preserves it according to its width; across 17 commits
the offsets may have gone odd → even → odd more than once. Naming the breaking commit
requires walking parity across all 17, which nobody has done, and the fix does not depend
on it.

Retracted rather than deleted, so the retraction is as citable as the claim was: a clean
causal sentence is exactly what gets quoted forward and never re-derived. (Caught by the
aeon overseer, in the same exchange where this note named the pattern it is an instance of.)

**Severity: latent, not live.** `Scene` is comptime-only and emits nothing, so no shipped
ROM performs an odd-address word access; all four CRCs held byte-identical across T11-T16,
which is consistent. What is broken is the guard, in exactly the scenario it was installed
to cover.

## Correction to an earlier claim in this note

An earlier revision stated the lint fires on a **region** declaration and therefore that
searching struct definitions could not find it. **That was wrong**, and it was passed to
the aeon overseer confidently enough that they withdrew a correct finding on the strength
of it. The lint has three message forms:

- `lower/regions.rs:598` — region fields
- `layout.rs:835` — overlay fields
- `layout.rs:1018` — **struct fields** ← the form that fired here

Only the first was read. The general shape of the error: a lookup returned something true
about the wrong object, and the confidence attached to it steered another session's
scrutiny away from the place that held the answer.

## The drift window — derived, and wider than either overseer first assumed

The baseline only moves when someone edits it, so the window opens at the array's last
modification, not at the last refreeze. `WARN_ID_BASELINE` is
`crates/sigil-cli/tests/warn_tier_corpus.rs:74-149`; `git log -L 74,149:<file>` dates it to
`37732afe`, 2026-08-18 15:06. **Window: 2026-08-18 15:06 -> aeon tip `b1f8a230`** = 25 aeon
merges, 44 changed `.emp` files, +8869/-738.

Two wrong windows were proposed first, both by a competent lookup returning a clean answer:
`b0b85f47..b1f8a230` (sigil — `b0b85f47` **is** the T11 merge, so the range starts after
it), and T10's refreeze `40f862e2` as the freeze point (aeon — that commit touched the
tests *directory*, not the baseline array).

**A SHA's class is what it CHANGED, not what it touched.** "Last commit under this
directory" and "last commit to this declaration" are different questions;
`git log -L <range>:<file>` answers the second. In-repo form of the protocol's cross-repo
SHA-class rule.

Also worth recording: the prime candidate reasoned from construct-adjacency (`3e4e5cfc`,
capability-selected record shapes) was **wrong**, and so was its replacement (see the
retraction above). Adjacency of subject matter is not evidence, and neither is "the commit
that introduced the field named in the diagnostic".

**The pattern all four of today's instances share:** a lookup returns something true about
the wrong object, and the confidence attached to the answer suppresses the check that would
catch it. Three were the sigil overseer's (the `b0b85f47..` window, the region-vs-struct
message form, the T12 attribution) and one the aeon overseer's (dating the freeze from a
directory-touch). Two of them occurred *after* the pattern had been named in writing, which
is the argument for making the check mechanical rather than adding another bar to remember.

## Ruling — the corpus run stops being gated on refreeze

Root cause of the six-parcel blind spot, diagnosed by the aeon overseer: the warn-tier
baseline moves only on a **refreeze**, a refreeze happens only when **bytes move**, and
aeon's T11-T16 were every one of them zero-byte (all four CRCs held). A ritual triggered by
byte movement is structurally blind to a source-derived lint set moving.

**Ruled (sigil overseer, 2026-08-22): the fix is sigil-side and the trigger changes.** The
warn-tier corpus already builds every shipped shape against `AEON_DIR`; it becomes a
standing check that runs against aeon tip regardless of whether bytes moved. That covers
every future source-derived check without aeon's ritual having to enumerate them. Aeon's
byte-identity check is unchanged. The aeon side keeps the *principle* booked (aeon
`bc5239b5`) and will point at this mechanism once it lands.

This instance validates the ruling independently: with the corpus ungated, T12 would have
been caught at landing on 08-21 rather than in a boot baseline the next day.

## Follow-ups

1. **sigil (mine):** ungate the corpus run from refreeze. Red-first proof required — a
   source-only aeon change must move it.
2. **sigil (mine), language gap:** a struct that wants even-aligned members can only say so
   by hand-counting bytes into a pad, and the pad goes stale silently when a field is
   inserted above it. An alignment attribute or an even-offset assertion would make the
   class impossible rather than merely catchable. Ledger and design.
3. **aeon (theirs):** re-align the two bridges and state the invariant as
   `ensure(offsetof(Scene, sc_mask_raw) % 2 == 0, "...")`. An `@offset 94` assertion was
   proposed here first and **withdrawn** — it pins a hand-maintained magic number that must
   be updated on every legitimate field insertion, which re-arms the exact failure being
   fixed. The `offsetof` form states the property, survives legitimate insertions, and
   fires only on real parity breakage. House pattern already shipped at
   `engine/system/replay.emp:96-102` (`% 2` and `% 4` span checks whose messages name the
   instruction that would fault, not the offset that moved); ~15 more at
   `sound_constants.emp:673-756`, `parallax.emp:254,309`. Supported end to end:
   `parser.rs:3884` (`Tok::Percent` → `BinOp::Mod`), folded at `eval/expr.rs:812,836`.

Do **not** measure any of this against `aeon/.worktrees/defines-verify`: its ROMs match the
pins but its source is an ancestor's, and these gates read aeon source — a pass there means
"aeon satisfied this as of an ancestor" while reading as "aeon satisfies this".
