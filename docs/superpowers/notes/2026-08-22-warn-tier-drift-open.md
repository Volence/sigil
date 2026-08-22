# The warn-tier lint set drifted — diagnosed

Status: **CLOSED for the mechanism and for the firing.** Follow-up 1 (the lane) and
follow-up 3 (aeon's re-alignment) both landed on 2026-08-22; follow-up 2 (the language
gap) is open and ledgered. See "Resolution" at the foot.

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

1. **sigil (mine): CLOSED 2026-08-22.** Ungate the corpus run from refreeze. Red-first
   proof required — a source-only aeon change must move it. Landed as
   `scripts/nightly_source_gates.sh` + `sigil-source-gates.timer`; see "Resolution".
2. **sigil (mine), language gap:** a struct that wants even-aligned members can only say so
   by hand-counting bytes into a pad, and the pad goes stale silently when a field is
   inserted above it. An alignment attribute or an even-offset assertion would make the
   class impossible rather than merely catchable. Ledger and design.
3. **aeon (theirs): LANDED.** Re-align the two bridges and state the invariant as
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

## Resolution (2026-08-22)

### The firing is fixed, in aeon, in the form this note asked for

`layout.odd-field` fires **zero** times on the corpus at aeon master `1ee8f8e6`, measured
with `SIGIL_WARNINGS=full` against a clean detached checkout of that SHA. aeon landed
`9a718f74` ("the Scene pad went stale — assert even parity instead of recomputing it"),
merged at `1a794ace`, and it took the `offsetof(Scene, …) % 2 == 0` form recommended above
rather than the withdrawn `@offset 94` pin — the pad widened `u8 -> u16` off the compiler's
own measurement, with the two `ensure`s, not the comment, now holding the authority.

**A cross-session correction worth keeping, because it is this note's own pattern again.**
The finding was reported as still open on the strength of `parcel/scene-even-align-guard`
having *zero unique commits and an empty diff against master*, read as "a branch name with
nothing behind it". Those are the signature of a branch that is **already merged**, and the
same report's own evidence said so: `git merge-base --is-ancestor cd4c8e23 master` returning
true means `cd4c8e23` **is in** master. A true fact about the wrong object, with enough
confidence attached to skip the check — the third instance in two days. The cheap
discriminator, when a branch looks empty: ask whether it is an ancestor of master before
concluding nothing is behind it, and measure the lint rather than reasoning about the branch.

### The trigger changed: a standing source-gate lane

`scripts/nightly_source_gates.sh`, fired by `sigil-source-gates.timer` at 05:17 daily
(an hour behind `aeon-effects-gates.timer`, so the two lanes do not contend). Modelled on
aeon's nightly: detached checkouts of both repos' master tips, both **outside** their repo
roots, exit `1` on a gate failure and `2` on a lane that could not run, `notify-send` on
either, `--selftest-fail` for the notification path.

It runs **33 source-only gates** — everything whose inputs are aeon source plus sigil's own
compilation of it. It deliberately excludes the ~63 region-diff gates, the ~18 golden-CRC
gates and `pins_rs_is_current`, all of which read bytes that exist only after `./build.sh`.
Those already have the right trigger — aeon's byte-identity ritual, which fires exactly when
bytes move. That is the correct trigger for them and the wrong one for these.

Proven red-first against aeon `b1f8a230` — the exact SHA this note recorded the finding at —
where the full lane exits 1 with `NEWLY FIRING: ["layout.odd-field"]` and 144 of 145 gates
still passing. So the ruling's counterfactual is now measured, not asserted: with the lane
running, the finding is caught the night it lands.

### A register for firings the baseline would have blunted

`CORPUS_OPEN_FINDINGS` in `warn_tier_corpus.rs`. The id baseline answers "may this lint fire
at all on this shape" and that file's own header admits it "does NOT catch growth WITHIN an
already-firing class" — so an id admitted there is admitted everywhere, any number of times.
The register pins `(shape, id, file, symbol)` with a per-shape **count**, and carries a
required owner, tracking anchor and kill condition per row.

The two rows today are the hand-written closure edges the baseline's own comment names the
cost of: *"a real accidental bare use in sonic4 now hides behind these two."* They no longer
hide. Measured red-first, each with the id gate staying **green** through it, which is the
whole claim:

- a bare `use` in a third file → `a registered lint id fired at 5 site(s) NO open-findings
  row claims`;
- the same site firing twice → `COUNT MOVED: … pinned 1, measured 2`. Not theoretical: the
  diagnostic does report the identical tuple twice, so set semantics would have passed it;
- a registered site adopting a name list → `RESOLVED: … no longer fires (was 1)`.

`layout.odd-field` is **not** in the register, because it does not fire. A register row is
for a firing that is real and unadjudicated; recording a finding that has been fixed is the
same class of stale artefact as the pad that started this.

Every row renders in the lane's ordinary output with its age in days. Green-with-a-named-open-item
is a different object from a silent green, and silence — not redness — was the failure here.
