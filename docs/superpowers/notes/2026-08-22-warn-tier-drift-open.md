# The warn-tier lint set drifted — open investigation

Status: **OPEN**. The firing is confirmed; the culprit is not yet named. This note records
the method and the rulings so the verdict can be dropped in without re-deriving anything.

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
does not belong to that family: no content blob decides a field address.

## What the lint means

`crates/sigil-frontend-emp/src/lower/regions.rs:592-598`:

```
[layout.odd-field] field `{name}` needs an even address but lands at {addr}
```

It fires on a **word-or-wider REGION field at an odd address** — `layout.rs:449` names it
"AS's silent address-error trap". `ty_needs_even` (`layout.rs:453`) is true for any prim of
width >= 2, any pointer, and recursively any array/struct/tuple containing one; `u8` and
`[u8; N]` are exempt.

So this is not a tidiness lint. A word-or-wider field at an odd address on a 68000 is the
silent address-error class, and it wants adjudication on its merits, not a baseline bump.

Note the construct: it is the **region** declaration, not a `struct` definition. A search
for struct-definition changes correctly finds nothing and correctly rules nothing out.

## The drift window — derived, and it is wider than either overseer first assumed

The baseline only moves when someone edits it, so the window opens at the array's last
modification, not at the last refreeze.

- `WARN_ID_BASELINE` is `crates/sigil-cli/tests/warn_tier_corpus.rs:74-149`.
- `git log -L 74,149:crates/sigil-cli/tests/warn_tier_corpus.rs` → last modified by
  **`37732afe`, 2026-08-18 15:06** ("re-home the OJZ parallax block"), which also carries
  the array's most recent `ADJUDICATED 2026-08-18` comment.

**Window: 2026-08-18 15:06 -> aeon tip `b1f8a230`** = 25 aeon merges, 44 changed `.emp`
files, +8869/-738. Leading candidate by construct: `3e4e5cfc` (p3/t8-extended-record,
"capability-selected record shapes").

Two wrong windows were proposed on the way here, both by a competent lookup returning a
clean answer:

- `b0b85f47..b1f8a230` (sigil overseer) — `b0b85f47` **is** the T11 merge, so the range
  starts after T11 and cannot see it.
- T10's refreeze `40f862e2` as the freeze point (aeon overseer) — that commit touched
  `crates/sigil-cli/tests/`, the directory, but not the baseline array.

**The trap, stated generally: a SHA's class is what it CHANGED, not what it touched.**
"Last commit under this directory" and "last commit to this declaration" are different
questions with different answers, and `git log -L <range>:<file>` is the one that answers
the second. This is the same family as the protocol's cross-repo SHA-class rule, applied
within a repo.

## Ruling — the corpus run stops being gated on refreeze

Root cause of the six-parcel blind spot, diagnosed by the aeon overseer: the warn-tier
baseline moves only on a **refreeze**, a refreeze happens only when **bytes move**, and
aeon's T11-T16 were every one of them zero-byte parcels (all four CRCs held at
`060401e4` / `0dbaa80f` / `c708b114` / `dec88cc1`). A ritual triggered by byte movement is
structurally blind to a source-derived lint set moving.

**Ruled (sigil overseer, 2026-08-22): the fix is sigil-side and the trigger changes.** The
warn-tier corpus already builds every shipped shape against `AEON_DIR`; it becomes a
standing check that runs against aeon tip regardless of whether bytes moved. Putting the
trigger on the thing that actually changed (source) instead of a proxy (bytes) also covers
every future source-derived check without aeon's ritual having to enumerate them. Aeon's
byte-identity check is unchanged and stays where it is.

Kill condition for this row: the corpus check runs on a trigger independent of refreeze,
with a red-first proof that a source-only aeon change moves it.

## Next step

Run the warn-tier corpus against the clean aeon worktree at committed tip `b1f8a230`
(`aeon/.worktrees/constlen`, built by the aeon session, CRCs to be verified against the
provenance tip first) and read the field name out of the diagnostic. Then adjudicate the
field on its merits.

Do **not** measure this against `aeon/.worktrees/defines-verify`: its ROMs match the pins
but its source is an ancestor's, and these gates read aeon source — a pass there means
"aeon satisfied this as of an ancestor" while reading as "aeon satisfies this".
