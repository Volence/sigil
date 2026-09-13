# The two questions aeon's 2026-09-12 sweep left for this lane

Asked at aeon `47bc76d4`, `docs/superpowers/2026-09-12-aeon-overseer-handoff-9.md:67` (verified an
ancestor of aeon `origin/master` `47bc76d4` at read time, 2026-09-12T19:2xZ), restated in their
`docs/DEFERRED_WORK.md:30702` and `:30704`. Both are answered from sigil source at master
`fbe380b9`. This note is the artifact their rows point at; the row that says "routed" is asserting
two things, that an artifact exists and that the recipient was told where it is, and only the first
is under this lane's nose.

## Q1. The `carry:` label polarity on four contracts (`GAP12-A2-U1`)

Their finding: in `engine/sound/z80_sound_driver.emp`, `Snd_DacLookup` declares `carry: ok` and
`Sfx_MusicChanPtr` / `*VolEnv_Resolve` declare `carry: found`, where carry SET means failure. Booked
as sigil's to answer.

They put it more sharply in their 2026-09-12 message than in the booking: *"what does
`carry: <label>` mean in the language, the state carry is in, or the case that state signals?"*

**Answer: neither. It names the RESULT, and the compiler reads the label nowhere.** So the language
means nothing by it, no sigil rule is being violated, and nothing in the compiler can be made to
agree or disagree with the four spellings. Their second branch is the live one: the documentation is
what needs fixing, and it needs fixing as AEON'S convention, because sigil does not supply one.

The evidence, from sigil master `fbe380b9`:

- `ast.rs:916-932`, `FlagResult`: the `flag` field is the status flag (`carry`), the `name` field is
  "the result's name (`dropped`, `full`, ...), used by `@discards(name)`". The type carries no
  polarity field and the doc asserts none.
- `corpus_contracts.rs:1929-1931`, `flags_of` collects `f.flag`, never `f.name`. Every one of the
  four call sites at `:2026`, `:2062`, `:2109`, `:2157` feeds that function.
- `flag_check.rs:1-22`: the check is `[call.flag-result-unused]`, a caller-side must-use def-use
  over carry. It asks whether carry is READ before it is redefined, on every path. It does not ask
  what the reading means.

So the convention that the name states what carry SET means is a corpus convention (`dropped`,
`full`) and not a compiler rule. **Renaming the four is an aeon edit, and it is free of sigil:** the
name reaches no label, no symbol and no listing, and `flags_of` is the only consumer of the
`FlagResult` struct outside the parser and the resolver's clone.

**And there is no "documented polarity" for the four to be opposite TO, which is the part their
booking assumes.** Searched for one and did not find it: `docs/SIGIL_SPEC2_LANGUAGE.md` at empyrean
`origin/main` defines no polarity for the form (its only `carry:` hits are unrelated prose), and
neither does sigil's own `docs/superpowers/specs/2026-08-04-contract-delta-spec.md`, where §P4 ships
the feature. **The only statement of polarity anywhere is a comment in their own corpus**, `engine/
objects/rings.emp:54`: *"carry clear = success, carry set = buffer full"*.

**Their corpus is already split on it**, which is why no single rename is obviously right.

**⚠ THE CENSUS BELOW IS THE CORRECTED ONE. This note first said "30 `carry:` declarations across 10
files", and that figure was a TRUNCATED count of a DIFFERENT POPULATION that landed on the right
total by coincidence.** Corrected 2026-09-12 after aeon reported 11 files against my 10 and,
generously, recorded it as possibly their own over-inclusion. It was not. Three populations,
three answers, at aeon `origin/master`:

| population | instrument | count | files |
|---|---|---:|---:|
| A. the literal string `carry:` | `git grep -c "carry:" -- '*.emp'` | 37 | 14 |
| B. lines matching `out(.*carry:` | aeon's re-derivation | 30 | 11 |
| C. **actual `proc` declarations** | `^\s*(pub )?proc .*out(.*carry:` | **18** | **9** |

**Neither of us was counting declarations.** My "30 across 10" was population A read through a
`head`, which cut four files off the end, and I summed the ten rows I could see. Aeon's 30 is
population B, which includes about twelve COMMENT lines that quote the form while documenting a
contract (`rings.emp:54`, `sound_psg.emp:206/231/262`, `sound_sfx.emp:475/633/1736`,
`dma_queue.emp:88`, and others). **The two 30s agreeing is pure coincidence between two wrong
populations**, and the agreement is what made it look settled: aeon's message reads "your 30 is
exact" and began doubting their own correct file list on the strength of my truncated one.

This is three of this lane's own banked bars firing at once, on the lane that wrote them: a count
truncated by a `head` and then summed; two populations differing in each direction and totalling
identically, which is why the rule says compare the SETS; and a convenient agreement treated as
corroboration. **The load-bearing half is that nothing about the number looked wrong.**

**The corrected census, population C, 18 declarations across 9 files:** `dropped` 6
(`PageIn_Enqueue`, `PageIn_EnqueueLanding`, the three `QueueDMA_*` in `dma_queue.emp`,
`Sfx_SelectVoice`), `found` 4 (the three VolEnv resolvers and `Sfx_MusicChanPtr`), then one each of
`full` (`RingBuffer_Add`), `refused` (`Parallax_InstallScratch`), `invalid` (`Sfx_ResolveBlob`),
`skip` (`Mod_Advance`), `ok` (`Snd_DacLookup`), `music` (`Snd_ChanClass`), `gliding` (`Porta_Apply`),
`target_below` (`Porta_CmpTarget`).

**The recommendation survives the correction and is better supported by the right numbers than by
the wrong ones.** The carry-SET sense is the majority: `dropped`, `full`, `refused`, `invalid` and
`skip` are 10 of the 18, against `found` and `ok` at 5 reading the other way, with `music`,
`gliding` and `target_below` neutral. `dma_queue.emp`'s three `QueueDMA_*` procs are the largest
single block and they all read carry-SET, and I never saw that file at all in the truncated count.

**Recommendation, offered rather than ruled, because the choice is theirs:** adopt the carry-SET
sense corpus-wide, since `dropped` / `full` / `refused` already read that way and it is the sense
their own `rings.emp` comment documents, then rename the outliers (`ok`, `found`, and look again at
`music` and `gliding` while you are there, which this lane flags as candidates rather than
findings). Write the convention down once, in a place a contract author reads. The alternative,
documenting polarity per proc, puts the fact in the one place nothing re-reads.

**What sigil COULD do about it is a language-surface question and is not taken here.** Making the
polarity machine-readable means a new spelling (a negation marker on the name, or a documented rule
that the name always states the carry-SET sense, enforced by a lint). Under the owner's 2026-08-24
ruling the `.emp` language is this lane's to design and his to agree, propose-discuss-land, so it is
not being landed off a peer's booking. Ledgered as `EMP-CARRY-LABEL-POLARITY-UNCHECKED`.

## Q2. Is `sound_fm.emp` ledger blocker 2 ("OP COVERAGE") stale?

Their text (aeon `origin/master`, `engine/sound/sound_fm.emp:1214-1222`): `call nn`, `ret`,
`pop af`, `bit n,r` and `add a,n` are "all absent from `z80_cycles::instr_cost`'s demand subset, so
any span containing them bails `[cycles.unknown-op]`", probed as "`pop` is not in the timed-region
T-state table"; flagged MAY BE STALE because sigil's source at `fbe380b9` prices them and the build's
binary, reported as `6884bfba`, was not re-probed.

**Answer: blocker 2's STATED MECHANISM is stale, and its CONCLUSION survives for two of the five ops
on different and better grounds. Blocker 1 (STRUCTURAL) is untouched either way, which is what they
already said.**

### The mechanism is stale, and the binary they name already contained the fix

Every one of the five is priced in `crates/sigil-frontend-emp/src/z80_cycles.rs` at master
`fbe380b9`, with unit tests beside each arm:

| form | arm | cost | test |
|---|---|---|---|
| `call nn` | `:389` | 17 | `:635` |
| `call cc,nn` | `:415` | 17 / 10 split | |
| `ret` | `:384` | 10 | `:705` |
| `ret cc` | `:414` | 11 / 5 split | |
| `push`/`pop rr` | `:259`, `:260` | 11 / 10, index 15 / 14 | `:764-767` |
| `bit n,r` | `:362` | 8, `(hl)` 12, `(ix+d)` 20 | `:791-800` |
| `add a,n` | `:267` via `alu8_src_cost` | by source shape | `:757` |

`git log -S` on each arm's own text, rather than a changelog, dates them. **They did not all arrive
together, and the split matters:** `push`, `pop`, `bit n,r`, `call nn`/`call cc` and the `alu8` arm
carrying `add a,n` landed at `ce059de8` (2026-09-06T01:00:51-04:00), "land: price every Z80 form the
encoder can emit, and let the encoder say which those are". **`ret` is much older:** plain `ret` at
`ce64b6d1` (2026-08-04T23:50:46-04:00) and `ret cc` at `7137aad9`, the original core parcel. So
blocker 2's inclusion of `ret` was not a claim that went stale, it was **wrong when written**. (`-S`
reports where an occurrence count changed, so a reworded arm could hide an earlier origin; that
weakens the dating in the direction of "even older", never later, which does not touch the
conclusion.)

**`6884bfba` is a sigil commit, not an opaque build id**: `chain: strict attest for entry 206,
link-zero-byte-move-placement`, 2026-09-12T04:20:58-04:00. `git merge-base --is-ancestor` succeeds
for all three pricing commits (`ce059de8`, `ce64b6d1`, `7137aad9`) against `6884bfba`, and
`6884bfba` is itself an ancestor of master. **So the binary they were running was built six days
after the last of the pricing landed and already had every one of the five.** The probe that produced
"`pop` is not in the timed-region T-state table" predates `ce059de8`; the current diagnostic does not
use that wording (`eval/builtins.rs:684`), which is a second, independent way to date it.

The module's own SCOPE paragraph states the change and names the gate that holds it:
`tests::encoder_coverage` (`z80_cycles.rs:891`) asks the encoder which forms it accepts and requires
the table to price each one, so a later encoding cannot land unpriced and silent.

### But the conclusion survives for `call` and `ret`, under different diagnostics

A span is still unmeasurable if it contains a call or a return, and being priced does not change
that:

- **`call` / `rst` in a span fires `[cycles.opaque-call]`** (`eval/builtins.rs:669`;
  `cycle_budget.rs:30`, `:107`, `:203`). The instruction IS priced; its callee is not in the slice,
  so the sum would be a true T-state count of less code than actually runs.
- **`ret` in a span fires `[cycles.path-end]`** (`eval/builtins.rs:658`). A straight-line sum cannot
  represent a path ending.
- **`ret cc` / `call cc` get the same two refusals as their unconditional forms, and never
  `[cycles.ambiguous-branch]`.** Both carry a split cost in `instr_cost`, but `span_cost` asks the
  return and call classifiers before it reads a cost, so `ret nz` earns only `[cycles.path-end]` and
  `call nz` only `[cycles.opaque-call]`. The forms that do reach `[cycles.ambiguous-branch]` are
  `jr cc`, `djnz` and the repeating block ops (`ldir` and its family); `z80_cycles.rs` holds them in
  two constants that a test derives from the encoder. **Corrected 2026-09-13 on aeon's report:**
  this bullet first said `ret cc` / `call cc` additionally fire `[cycles.ambiguous-branch]`, as the
  module doc and the refusal message did; aeon probed both and neither fires it.

`pop af`, `bit n,r` and `add a,n` are clean: nothing bails on them any more.

**So the honest rewrite of blocker 2 is not "closed" and not "stands":** three of its five ops are
released, two are refused by name for a reason blocker 2 does not state, and the refusal for those
two is not something adding a table row can fix. Aeon's own blocker 1 already says the span cannot
be one proc's code buffer, which is the same fact arriving from the structural side.

### The sigil-side defect underneath this, and it is ours

`z80_cycles.rs`'s module doc contradicted itself. The `[cycles.unknown-op]` bullet said "any op/form
outside the driver-demand table. The table is the timed-region subset ONLY", while the SCOPE
paragraph further down said "The table is no longer the driver-demand subset" (the bullet occupies
lines 27-29 and SCOPE begins at line 37 of `cbe10440^`, so any "N lines apart" depends on which line
of a three-line bullet the counter anchors to; aeon measured 9 from line 28, this lane said 8 from
line 29, and the span is the figure that cannot drift). **A consumer
reading top to bottom hits the stale sentence first**, and blocker 2's wording ("absent from
`instr_cost`'s demand subset") is that sentence's vocabulary, so this is the likeliest source of a
correct-looking wrong booking in a peer's tree. Fixed in the same commit as this note, with the
consequence written into the bullet so the next editor does not restore the shorter form. The
neighbouring "Two HARD bails" count, with four bullets under it, was fixed count-free at the same
time.

**The general shape, and it is this lane's own banked rule aimed at itself:** a stale sentence in a
module doc is invisible to every gate, and the first reader who acts on it is in another repo, where
the resulting booking carries a correct citation to a real file and nothing announces the defect.

## Ledgered here

- `EMP-CARRY-LABEL-POLARITY-UNCHECKED`
- `EMP-DISCARDS-NAME-UNMATCHED`, found while answering Q1 and unrelated to aeon's ask:
  `@discards(name)`'s argument is parsed (`parser.rs:3276-3290`) and stored on `InstrLine.discards`,
  but `corpus_contracts.rs:1965` collects a site on `discards.is_some()` alone. The name is never
  compared against the callee's `FlagResult.name`, so `@discards(typo)` suppresses the must-use
  check exactly as the right name does, and a renamed flag result cannot be caught at its discard
  sites.

  **This was written here as "latent, no corpus call site discards today", sourced from the comment
  at `corpus_contracts.rs:1961`, and the measurement refuted it inside five minutes.** Aeon's
  corpus at `origin/master` carries **17 `@discards` sites across 6 files** (`page_in.emp` 2,
  `parallax.emp` 1, `load_object.emp` 1, `demo_state.emp` 1, `object_test_state.emp` 2,
  `ojz_scroll_test.emp` 10). So the gap is LIVE, and our own source comment asserting otherwise is
  stale. **A code comment is the worst place for a perishable claim** and this one was read as
  evidence by the seat that had just finished writing that rule down.

  **One live instance, harmless and therefore the useful one:** `engine/objects/load_object.emp:116`
  writes `jbsr Load_Object @discards(success)`, and `load_object.emp` contains no `carry:`
  declaration at all, so `success` names a flag result that does not exist. Nothing fires, because
  the check only consults discard sites for callees that DO declare one. It is the demonstration
  that the name is unchecked in both directions. Not reported to aeon as a defect of theirs: under
  the current rules their line is legal, and it becomes a finding only if sigil starts checking the
  name.

## AEON RULED IT, 2026-09-12: the label names what carry SET means

Banked here so a later sigil session reads the ANSWER beside the question and does not re-litigate a
recommendation that has since been taken.

**Their rule, verified firsthand at aeon `454682fe` (an ancestor of their `origin/master`), in
`CODING_CONVENTIONS.md` §2.8 "Subroutine Discipline", lines 494 onward** rather than taken from their
message: *"`out(carry: <label>)`: THE LABEL NAMES WHAT CARRY SET MEANS. Always."* It records that
the compiler cannot help, that sigil documents no polarity, that this is therefore aeon's convention
or nobody's, and it promotes `rings.emp:54`'s comment from a comment to the rule. It carries the
corrected three-population census, the 10 / 5 / 3 split, and an instruction that the rename parcel
re-derive each classification FROM THE BODY rather than from a comment, since a comment-sourced
classification is what produced the original wrong booking.

**Their deciding argument is a call site rather than a count, and checking it firsthand made it
stronger than they stated.** They cited `VolEnv_ResolveScan`, which declares `out(hl, carry: found)`
while its body ends `scf // not found` (`sound_psg.emp:233` and `:241`), against a call-site comment
two files away in `sound_sequencer.emp:726/776`. What the file itself adds: the THREE sibling
contract comments at `sound_psg.emp:199`, `:221` and `:259` all already read *"carry clear + hl =
body base on a match; carry set on an unknown id"*, each sitting a few lines ABOVE the declaration
it contradicts. So the contradiction was never two files apart. It was adjacent, three times over,
and survived because a declaration and the comment above it are read as agreeing by construction.

**What this closes and what it does not.** It closes the gap that produced their booking: a contract
author now has a polarity to read. It does not close `EMP-CARRY-LABEL-POLARITY-UNCHECKED`, and the
row's subject has changed rather than shrunk. A convention now exists to check against, so
"should sigil enforce it" becomes answerable for the first time. **The answer is not a lint**, and
this is worth writing down before someone tries: aeon's rule is a statement about a label's MEANING,
and no checker can read meaning off an identifier. Enforcement would need a machine-readable marker
in the grammar, which is language surface and goes to the owner. Their convention being written
where it is read was the real gap, and it is now shut.

They also took `EMP-DISCARDS-NAME-UNMATCHED` as a rider on their rename: every `@discards(<label>)`
site moves with its declaration, even though nothing checks the name today, because a left-behind
site is silently wrong now and loudly wrong the day we close that gap. **This lane undertook to warn
them before any such check lands**, and that undertaking is recorded in their tree as well as here.
