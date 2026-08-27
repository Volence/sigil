# Sigil Overseer

How a Claude session runs **sigil** as its overseer. The shared role — solo-first
posture, delegation discipline, review bars, peer protocol — lives in
`empyrean/docs/OVERSEER-PROTOCOL.md`; read it once, then this file for what is
sigil-specific: the landing-lane division, the worktree/test quirks, and the queue.

## Boot

> You're the overseer for this repo. Read `docs/OVERSEER.md` first, then
> `empyrean/docs/OVERSEER-PROTOCOL.md` if you haven't. Work the queue. Peers may or
> may not be running — check `ListAgents`; coordinate if present, proceed solo if not.

**Read the protocol at a committed revision, never through the filesystem path:**

```sh
git -C ../empyrean fetch -q origin && \
git -C ../empyrean show origin/main:docs/OVERSEER-PROTOCOL.md
```

`../empyrean/docs/OVERSEER-PROTOCOL.md` is one peer's live working tree, so reading it
by path means booting from somebody's uncommitted directory. Carried here because this
is the only file upstream of that read; the empyrean copy governs on any disagreement.
Taken from empyrean `274d26d2`.

**Re-read it mid-session.** That file currently moves faster than a session lasts — on
2026-08-22 it went from `cea2e57c` to `274d26d2` and gained two numbered bars inside
ninety minutes, both of which landed on this lane's in-flight work. Boot-time is the only
read anybody performs unless you make yourself perform another; re-read when a peer cites
a bar you don't recognise, before dispatching a wave, and at any landing.

## Landing-lane division — THE rule for this repo

The aeon↔sigil landing lane has **one owner, and it is the aeon overseer** (owner
directive, 2026-08-19). Aeon-paired byte-movers — golden refreezes, the provenance
chain, pin updates — land through aeon's session. One session sequencing them is why
a 16-refreeze day (chain 134→149) had zero collisions; a parallel refreeze against
shared goldens is exactly the collision this rule prevents.

**The sigil overseer owns everything sigil-internal:**

- the `.emp` language work (Spec 2 — specs in `empyrean/docs/SIGIL_*.md`)
- the CLI (`crates/sigil-cli`), the frontends, the backends, the linker
- the port-test harness (`crates/sigil-harness`) and its test-support seams
- sigil-only parcels: anything whose landing does not move aeon-paired bytes

**COORDINATE with the aeon session before touching any of:**

- `crates/sigil-harness/golden/` (and `golden/provenance.toml`)
- `crates/sigil-harness/src/pins.rs`
- `crates/sigil-harness/repin.toml`

If a sigil-internal change would ripple into those files, message the aeon overseer
(find it via `ListAgents`; address by repo, not session name) and let that session
sequence the landing. A byte-changing parcel ripples past `pins.rs` into
`engine.inc` / `mixed_dac_rom.rs` / `repin_pins.rs` — the repin tool auto-updates
only `pins.rs`; the rest are hand-edited, and `repin.toml` changes only when a
region is added. That whole ripple belongs to the aeon-owned lane.

Provenance identity is **CRC32 + size**, never SHA1 — the campaign standard.

## The autonomy directive — and its scope, which is the part that matters

Every sigil overseer session operates on a banked owner directive: **on assembler
internals the owner defers to the implementer — make the best technical/design call and
proceed; checkpoint at milestone boundaries.** Provenance, stated because it is load-bearing
and was until now unbanked: it lives in this lane's session memory
(`user-defers-sigil-technical-calls`, written ~2026-07-05, origin session
`9ff8029c`), recording the owner's own words — the internals are "out of my wheelhouse",
"make the best decisions". It is a real granting act, not a status field promoted into a
ruling. But **`git log -S` confirms it has never appeared in any sigil doc**, so until this
paragraph every cold boot took the whole posture from a memory file no other seat can audit.

**The scope boundary is narrower than "sigil technical calls", and reading it broadly is the
live risk.** The directive names *technical/design forks and encoding minutiae* — fold-vs-fragment,
where a shared rule lives — and explicitly reserves **direction and priorities** to the owner.
So:

**RULED BY THE OWNER, 2026-08-24, in his own words — and it is none of the three options he
was offered, so read the words rather than a key.** Verbatim: *"The language is yours, but
let's discuss it first and let me agree or not for the most part"*. Put to him as decision
`d-3`; recorded as `d-6`.

**What it means operationally.** The `.emp` language is this lane's to design and drive —
that is a real grant and is broader than the previous parked reading, which sent every new
surface to him as a blocker. It is **not** a licence to land language surface silently. The
shape is **propose, discuss, then land**: sigil does the design work and forms the
recommendation, puts it to him in plain terms before it lands, and he agrees or not. The
authority to design sits here; the agreement is his and is genuine, not a notification.

- **Discuss before landing:** anything that adds or changes a word, spelling or construct
  the game's source will be written in. That is the durable surface — once the game is
  written using it, it is close to unremovable, which is the whole reason he wants a look.
- **Just do it:** everything behind that surface. Implementation strategy, diagnostic
  wording, gate design, which of two sound encodings to use, how the compiler achieves a
  construct that already exists. This is `d-2` territory and gets a lane-log note if the
  call is notable, no discussion required.
- **"For the most part" is load-bearing and cuts toward less ceremony, not more.** He did
  not ask to approve every spelling. A trivial, obvious or purely-mechanical surface change
  does not need a round trip; use judgement, and when in doubt spend the one message. What
  he is buying is a veto on the shape of the language, not a queue of rubber stamps.
- **Do not convert this into a blocker.** A discussion is a message and an answer, not a
  `blockedOnOwner` entry with work stopped behind it — park it as a blocker only if it is
  genuinely holding a landing and he has not answered.

Live consequences of this ruling, both previously stuck: the closure-edge import spelling
that kills the two `CORPUS_OPEN_FINDINGS` rows is now sigil's to design and put to him, and
`pad_to(N)` (queue item 4) is sigil's to draft and put to him rather than a thing that waits
for him to raise it. Neither lands without his nod.

**`pad(N)` / `pad_to(N)` — RULED YES BY THE OWNER, 2026-08-26, directly in this lane's
session.** Put to him as `d-12`; his answer was a bare **yes** against the recommendation, so
the ruling is the **`both`** option: `pad_to(N)` derives the filler width from the next field's
offset, AND the plain `pad(N)` reserve-N-bytes spelling stays for the case where the count is
the author's real intention, with a diagnostic that names the mistake and prints the exact line
to write when the deriving form was clearly meant. Closed as `d-13` (rule 8c supersession).
`(align: N)` stays mandatory and is not the derivation's casualty — that is §1.3 of the draft
and the argument the whole design rests on.

**This retires the provenance caveat the draft carries in its own header.** That document
records its authority as an *empyrean* overseer's ruling made under a delegation reported in
the owner's words, flagged there as *"this lane did not witness that utterance"*. It is now
witnessed firsthand here, so the construct no longer stands on a relayed grant. The draft text
itself is unaffected; only its authority line is.

**The spec text is LANDED and un-DRAFTed at empyrean `2000b5ca03592377ea1881671db53e03ad36f264`**
(reachable from their `origin/main`, verified here after their push, not from the local tracking
ref). D2.37 is `(align: N)`, D2.38 is `pad`/`pad_to`, §4.3.1 carries the construct text lifted
verbatim from this lane's draft. **Cite that SHA from the implementation parcel.**

**What the review of that text bought, and the debt it left here.** The spec had carried, from
this lane's own field-align packet §10, the claim that the six `(align: N)` diagnostic strings are
a cross-repo interface aeon fixtures assert on. **They are not: none of the six appears anywhere in
aeon** — not in the 44-case `tools/emp_expect_fail.py` negative-build lane, not in the poison
modules, nowhere. Checked here and independently by the hub. It was a name doing a behaviour's
work, and it would have frozen six strings against a consumer that does not exist while telling
every reader a gate protected the wording. The producer side is thinner than the spec implied too,
measured string by string at `db2dacce`: #1 well pinned; #2 only on `must be a power of two` (the
parenthetical free); #3 only on ``asserts its alignment with `(align: N)` `` (the leading clause
free); #4, #5 and #6 pinned by nothing in the workspace.
**Owed by this lane, in the `pad`/`pad_to` parcel:** tests for #4-#6, widen #2/#3 to the full
strings, and pin the Scope clause (true by construction — the check walks only this struct's own
fields — but unpinned, and marked as such in the spec). The hub strengthens that clause on a
message from here once it lands.

Sequencing, so nobody lands half of it: the spec text is **empyrean's to land** — sigil does not
land `.emp` language spec, `SIGIL_SPEC2_LANGUAGE.md` is their file — lifted verbatim from
`docs/superpowers/notes/2026-08-26-pad-to-spec-draft.md`. **Nothing is implemented**; no crate
was touched by the parcel that wrote the draft, and `pad_to_cycles` in `t40_cycles.rs` is an
unrelated cycle-padding construct that shares a prefix and nothing else. Implementation is a
separate parcel and is this lane's.

**RE-CONFIRMED BY THE OWNER, 2026-08-24, directly in this lane's session — not relayed.**
The 48-day-old grant was put to him as decision `d-2` in `docs/decisions.jsonl` and he chose
**keep it, and tell me the notable calls as they happen**. So the authority is unchanged and
a fresh boot does NOT need to re-ask it. What is new is the obligation attached to it, below.

**The obligation: a notable call gets written down for him WHEN IT IS MADE.** Before this
ruling nothing was written down for him at all, which meant the first he heard of an internal
design choice was when something built on it went wrong. The vehicle is `docs/lane-log.jsonl`
(`empyrean/contract/LANE_LOG.md`), which is already the owner-facing feed — do not invent a
second channel. The bar for "notable" is **a choice a reasonable person could have made the
other way and would care about having made**, not every fork in the road: a spelling that
will appear across the game's source, a tier decision (error vs warning), a soundness
trade, an option deliberately refused. Routine implementation strategy stays unwritten;
logging everything would restore exactly the volume the lane log exists to replace.
**Write it at the time, not at the landing** — a call reconstructed afterwards is the
confident-guess-wearing-a-record's-clothes defect that ruling forbids for the log generally.
The scope line itself (`d-3`) is still open and is the one that gates work.

**Do not fund work off the broad reading of the scope line while `d-3` is open.**

**PUSHING IS STANDING-APPROVED — owner ruling, 2026-08-24, direct in this lane's session.**
Put to him as decision `d-1` in `docs/decisions.jsonl`; he chose **send finished work up
whenever it is ready**. So a push needs no per-time approval and a fresh boot must not
re-ask. The reasoning he was given, which is also the scope: pushing only ever ADDS, and
until work is at the remote no other lane can reference it at all, so holding it back
protects nothing and only delays consumers. That is the whole grant — **it authorizes
fast-forward pushes of finished work, and nothing else.** A history rewrite (force-push,
rebase or squash of already-pushed commits) is a different act with a different blast
radius, it is not covered here, and it still goes to him. Verify every push against
`git ls-remote origin refs/heads/master`, never the local tracking ref, which is the only
check that distinguishes "pushed" from "looks pushed".

The history this replaces, kept because it is why the question existed: the 2026-08-22 push
to `a70e6644` was banked as "owner-approved" for that ONE push, and the preceding 80-commit
unpushed backlog was described by the aeon lane as "the owner's gate", i.e. deliberate. The
ambiguity was real, and it cost two sessions the same re-derivation before it was asked
plainly.

## Dispatch practice — a stated MECHANISM is more dangerous than a stated FACT

*(2026-08-22; the oracle lane's formulation, from an episode this lane caused.)* A brief's
factual claims compete with the agent's evidence and lose when wrong. A brief's
**explanations do not compete — they absorb.** An agent that measures something
inconsistent with its controller's stated mechanism will tend to reconcile the measurement
to the story rather than report the conflict, and **an agent has almost none of the standing
a peer overseer has to push back.** So label mechanisms in a brief as hypotheses, and say
outright that the agent's own command output outranks anything the brief asserts.

**The cheap mechanism, with this lane's measured hit rate.** Every dispatch's deliverable
section ends with a required line: *"and anything in this brief you concluded was wrong."*
On 2026-08-22 that produced a correction in **3 of 3** dispatches:

- `feat/field-align` overturned the brief's **central design ruling** — the auto-padding
  question was already settled by spec §4.3, dispatched as open because the controller had
  not read the spec.
- `fix/rom-sentinel-port-tests` refuted the brief's implied **cost** argument for excluding
  gates from the nightly lane (cost is not the obstacle; committed-artifact oracles are) and
  corrected the booked count from ten to eleven.
- `docs/readme-refresh` corrected a flat **factual** error (`SIGIL_BUILD`/`SIGIL_EMIT` do not
  live in `test_support.rs`) and declined to document an env var the brief implied was
  required, having verified it is not.

Pair it with an explicit invitation where the brief carries a design position — the
field-align brief said a contradicting delivery is worth more than a complying one, and got
the sharpest pushback of the three.

**Honest limits, so this is not read as a solved problem:** n=3, one day, one lane, and all
three briefs were checkable against a tree, which is the easy case. A self-report line
cannot surface what the agent never thought to question — it catches conflicts the agent
noticed and would otherwise have swallowed, which is a real but bounded win.

## Quality bars

**⚠ THE REFERENCE TREE IS WHATEVER `golden/provenance.toml`'s TIP PAIRS WITH — and every
CRC written into THIS file is a snapshot that a refreeze silently invalidates**
*(2026-08-26, and this overseer caused it at a dispatch)*. The bar paragraph below used to
carry four CRCs and an aeon SHA. A refreeze landed at `029868e5` (chain entry 165, paired
with aeon `b08b35c0`), moving the two s4 goldens — and nothing updated the paragraph, because
nothing can: a prose CRC has no trigger. The next dispatch verified `.aeon-landing`'s four
artifacts **against the numbers in this file**, found them matching, and declared the tree
good. It was a real check with a stale oracle, so it returned green while pointing the parcel
at the wrong aeon revision; the agent lost a run to **71 failures reading exactly like a
byte-moving regression** (`Ground_Move_Cap resolved to 0x10902, expected 0x10912`) and settled
it only by re-running the same gate against master's own sources and getting the identical
failure. Note the shape: this is the *derived-never-copied* bar (#1) aimed at the reference
tree rather than at a test expectation, and it is the failure mode this document already warns
about one section down — *"Read the TIP of `golden/provenance.toml`, never a number in this
file"* — which did not save the dispatch that wrote it.

**So: derive the reference tree, never read it off a note.** The demo shapes matched through
the whole incident, so a partial match is not a witness:

```sh
tail -40 crates/sigil-harness/golden/provenance.toml   # the TIP entry = the four expected CRC32+size
git -C <aeon-tree> rev-parse --short HEAD              # must be the SHA that tip entry pairs with
```

All four must match on **CRC32 + size** (never SHA1 — the campaign standard) before a run is
worth anything.

**Which reference worktree is current is NOT recorded here, deliberately — ask the disk.** This
paragraph twice carried a named worktree and a SHA, and both rotted at the next refreeze while
reading as fact. The candidates are `~/sonic_hacks/.aeon-landing` and
`~/sonic_hacks/.sigil-portfix-aeon`; which of them pairs with the tip changes every freeze, so
derive it:

```sh
for d in ~/sonic_hacks/.aeon-landing ~/sonic_hacks/.sigil-portfix-aeon; do
  printf '%s %s\n' "$d" "$(git -C "$d" rev-parse --short HEAD)"
done                                                   # then CRC32+size all four ROMs in it
```

`~/sonic_hacks/.aeon-sigil-gates` is never a candidate — it is source-only by construction and
deletes built ROMs, per the source-gate lane section.

**This is now gated, as of 2026-08-26 — and it was not the one-afternoon gate it looked
like.** Historically no gate witnessed that `AEON_DIR` matched the provenance tip, because
`provenance.toml` had **no field naming the aeon revision a freeze pairs with**: at chain 166
the schema was `name` / `ab` / `note` plus the per-target CRC rows, and the aeon SHA appeared
only inside the free text of `ab`/`note` (16 of the 166 entries carry no `note` at all). Every
check of the pairing to date was a human reading prose, and a parcel lost a run to exactly
that. What shipped:

- **`Entry.aeon_rev`** — a full 40-character SHA (never abbreviated), typed `Option<String>`
  with `#[serde(default)]`, so the 166 historical entries keep parsing; they are
  **deliberately not backfilled**, since a prose-derived SHA is a reconstructed record. The
  `Option` is load-bearing: `None` means the KEY IS ABSENT (an older `refreeze` wrote the
  entry) while `Some("")` means somebody blanked it, and only the first is legitimate.
- **`refreeze --freeze` refuses unless it can name the revision honestly** — `AEON_DIR` unset,
  not a directory, not a git repo, HEAD unresolvable, or the tree **DIRTY** all refuse, before
  anything is built, each naming the variable, the path and the fault. This is not new policy:
  the landing lane below already required freezing from a clean checkout of a committed SHA.
  It was simply unenforceable, and unset silently fell back to the owner's live tree via
  `capture_goldens.sh`'s `${AEON_DIR:-…/aeon}`. **`--check` is unaffected and still takes no
  aeon tree.**
- **`provenance_chain::aeon_dir_matches_the_provenance_tip`** — compares `AEON_DIR`'s HEAD
  against the tip's `aeon_rev`. **Hard under `SIGIL_STRICT_GATE=1`; a loud `notice:` line
  otherwise.** Deliberately not hard in both modes: aeon master routinely runs ahead of the
  frozen tip for byte-neutral reasons (at the time this shipped it was two commits ahead, both
  documentation-only, and four by the time it was reviewed), so an unconditional assertion
  would be red on trees that are byte-correct in every way. Strict is where the bar belongs —
  it *is* the pre-merge run.
- **AEON-REV-WELL-FORMED** in `provenance::check` — an entry carrying an `aeon_rev` at all must
  carry a full 40-char SHA, **wherever it sits in the chain**. Hard in every mode.
- **AEON-REV-MONOTONIC** in `provenance::check` — once any entry names its aeon revision, no
  later entry may omit it. Hard in every mode. These two are the rules with teeth today.

**⚠ THE BOUNDARY IS DERIVED FROM THE CHAIN, NEVER PINNED TO AN ENTRY NUMBER — and this was a
real defect caught in review, not a style preference.** The first implementation used a
`AEON_REV_FROM_ENTRY = 166` constant. The aeon lane refreezes by running sigil's `refreeze` out
of **sigil master**, so a byte-moving refreeze landing before the field shipped would append a
field-less entry #167 that was entirely legitimate when written — and the pinned rule then
turned master red on somebody else's correct work the moment the branch merged. Measured, not
reasoned: on a chain carrying exactly that entry the pinned form failed **two** tests
(`provenance_chain_holds` and the pairing gate) while the derived form passes the identical
file. That is the failure this whole mechanism exists to prevent, aimed at ourselves. If you
ever find yourself wanting a number here, that is the trap.

**The one tolerance, and its disarm condition:** while the tip carries no `aeon_rev` the
pairing gate prints a `ratchet:` line and passes *in both modes*. `refreeze` appends nothing
when nothing moved, so no parcel can force the field onto the tip; failing closed would put
master red under the full-suite bar indefinitely. **The ratchet disarms permanently at the
next refreeze that names a revision** — the condition is the field's absence, never an entry
number. It prints `ratchet:`, never `skip:` — the bar
below requires zero `skip:` lines and this is not a missing reference.

- **Full suite bar:**
  `SIGIL_STRICT_GATE=1 AEON_DIR=<clean> cargo test --release --workspace --no-fail-fast`,
  with `AEON_DIR` a tree matching the provenance tip (derive it — see the warning above) —
  **3943 passed / 0 failed / 4 ignored** (3947 declared), **zero `skip:` lines**, exit 0,
  clippy `-D warnings` exit 0.
  **The env vars belong INSIDE the command span, and this paragraph had them outside it
  until 2026-08-27.** That is not formatting: a reader copies the backticked command and
  gets the prose-stated requirement only if they read two lines further. It is the exact
  defect that cost aeon two unverified chains from their own landing lane, found in their
  file and swept here immediately after.
  Last measured at master `2e284c4d` against aeon `ac57991d` (chain 171), 2026-08-27, by the
  aeon lane's landing run of the seam1 re-pin.
  **This paragraph said `3939 / 3943` for a day while the RELAYOUT-REVIEW section below
  recorded `3943 / 3947` from the same tree, and the stale half is the one a cold boot quotes
  — this overseer quoted it to a peer, who reconciled against `git grep -c '#\[test\]'` and
  refused it.** That is this document's own trigger-less-prose defect, one section below the
  paragraph warning about it. **Reconcile against the declaration, never against this
  sentence:** `git grep -c '#[test]' HEAD -- '*.rs'` summed is the count, and
  `passed + ignored` must equal it.
  **The run now emits ZERO `ratchet:` lines and that is the correct state** — the pairing
  gate's self-disarming tolerance ended when chain 167 became the first tip to carry an
  `aeon_rev`. A `ratchet:` line reappearing means a tip was written without the field, which
  `check`'s monotonic rule should already have refused; investigate rather than tolerate.
  **⚠ DO NOT COMMIT WHILE A LANDING RUN IS IN FLIGHT.**
  `version_reports_the_head_of_the_tree_it_was_built_from` compares the binary's baked-in
  revision against the checkout's HEAD *at assertion time*, so a commit landed mid-run fails
  it — the binary is honest, the tree moved. Cost here: one full re-run at a landing. The
  diagnostic names both causes and the command that separates them, and the log stamp is what
  makes it legible; without `head=` in the header it reads as a mysterious one-test red.
  **The count is the bar; the pairing is a
  timestamp, not an instruction** — a later freeze moves the aeon SHA and leaves the count
  alone, so reconcile a parcel's delta against `git grep -c '#\[test\]' HEAD -- '*.rs'` and
  read the pairing only as "this number was last seen on that pair".
  **The previously recorded bar of 3881/3885 was 4 LOW and had been for some time** — measured
  at `67575c3c`, master's own sources run 3885/0/4 against 3889 declared, and
  `git grep -c '#\[test\]'` agrees with the run, not with the old note. A parcel's delta is
  therefore reconciled against `git grep -c '#\[test\]' HEAD`, never against the number in this
  paragraph; the pad parcel's +43 matches the declared-count delta exactly, where the stale bar
  implied +47 and would have manufactured a four-test discrepancy that does not exist.
  **⚠ THE REFERENCE CHECKOUT MUST CARRY NO `sigil`/`skdisasm` SYMLINKS.** The aeon main tree
  has none, and adding them makes `section_row_fixture`'s tree mirror die with
  *"the source path is neither a regular file nor a symlink to a regular file"* — three gates
  red for a reason that has nothing to do with the parcel under test. The old worktree-seeding
  note that says a fresh aeon worktree needs them is STALE; `./build.sh` works without them.
  Ledgered: the mirror should skip or name a non-regular entry instead of `unwrap`ing.
  *(History 2026-08-24→26: 3844/5/4 → §4 → 3852/5/4 → closure 3854/3/4 → first real scene
  3852/5/4 → SECTION-ROW → CLOSURE-2 3866/3/4 → REPIN-END 3869/1/4 → FIVE-REG 3870/0/4 →
  NIGHTLY-GAP 3872/0/4 → aeon ring-sparkle 3870/2/4 → BGROOM-3 + RINGS-ENV 3881/0/4.)*
  **BGROOM-3 landed** (`fix/measure-at-packed-base`): every measuring round is exact at its own
  bases (`resolve_layout_measuring` in sigil-link, fixpoint; scratch/spread fallbacks deleted,
  which also removes the ~0x400 growth cap — 5000 B of growth now builds with drift warnings),
  and non-convergence names the width-flipping sites file:line with both encodings. **The aeon
  lane's reported MECHANISM was refuted while their measurement stood:** not "abs.w because the
  provisional base is unknown" (an unresolved operand is a hard error) but the collision-fallback
  scratch slot aliasing zero — `collision_data` at slot 41 = `0x300_0000`, masked to 24 bits by
  `asl_width_rule` → `0x0`, where `abs.w` is legitimate. Their `lea (X).l` pins are a SUPERSEDED
  WORKAROUND, not a style rule; they un-pin once the shared binary reports a master containing it.
  **CONFIRMED IN THE FIELD 2026-08-26** (aeon `bc95e32e` and its parent): they un-pinned
  `SolidityTable`/`AngleTable` back to unsized `lea` and all four shapes stayed byte-identical
  (`b96319e3` / `7be32302` / `bf2cdb42` / `62a0019e`), after verifying the shared binary
  themselves rather than taking a quoted SHA. Their own finding, kept because it is a trap for
  the next reader: the THIRD site, `movea.l #{ptable}`, was never a width pin — `({ptable}).l`
  does not parse in a template arg, so it was always an immediate, and `movea.l`-immediate vs
  `lea`-abs.l are both 6 bytes but different opcodes, so "finishing" the un-pin moves the CRC at
  identical size for no gain. It stays `movea.l` with a comment saying why.
  **RINGS-ENV landed** (`fix/rings-contract-env`): the port rigs bind the game's real contract via
  `test_support::game_contract_env_from_aeon` (interface + the profile's own `config/game.emp`,
  path derived from `GameProfile::game_root_rel`), replacing hand-written stub strings that could
  not see a new hook. `game_contract_env_coverage` parses the member COUNT from the contract, so
  the next hook cannot repeat it. `tranche5_negative_probes` keeps its synthetic stub by intent
  (it probes the binder over a define matrix no manifest spans); `camera_port` is split.

  **FIVE-REG landed:** the last red was `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma`
  on the **PinnedBaked/registry** path (`build_emp(sonic4_pinned_profile)` → `emp_map_toml`
  mints one region per REGISTRY pin; a content-derived section declared by a `section:` row
  has no `pins::Region` by design and `repin` derives pins from the shipped resolve, so that
  path can never learn it). Every PinnedBaked reference enumerated: two `#[ignore]`d archaeology
  tests, `derive_offcanon --bootstrap-canonical` (documented unavailable since 2026-08-01), and
  an unreachable arm behind the Frozen early-return. The catch was RE-HOMED, not deleted:
  `soundbankhead_pin_is_the_lma_not_the_vma` asserts `pins::SOUNDBANKHEAD` bases == the shipped
  resolve's lma with a distinct vma present (literals removed, red-first on a sabotaged pin).
  Test-file `LOCK` is poison-tolerant (3/8 repeats had poisoned a sibling; 8/8 clean after).
  Ledgered: retire the whole PinnedBaked path (kill list in the gap ledger).

  **REPIN-END landed:** `repin.toml` `start`/`end` accept `section:<name>` (LMA / own
  `lma + image_len`, one derivation `native::section_end` shared with the `<Base>_End` sites);
  `act_descriptor` and `scene_registry` ends re-spelled; `ACT_DESCRIPTOR` plain_len 0x27C→0x27A,
  debug_len 0x280→0x27A, bases unchanged, no other constant moved, `repin --check` a fixed point.
  A bare `end` that sweeps placer pad is now WARNED per shape by `repin` — **79 region/shape
  pairs** carry pad by convention (boot … player_climb, 0x2..0x20), green under the align-pad
  tolerance, ledgered for region-by-region conversion. `scene_registry` was NOT affected (0xACE
  flush both shapes; the 0xB1C was pre-repoint history).

  **CORRECTED 2026-08-26 (aeon's closure agent, reproduced in worktrees; branch verified here as
  2 test files, +20/-1): "cause is aeon-side, the RESERVED SLOT" is WRONG for four of the five.**
  (A) `unknown function ojz_act1_act_default / ojz_act1_sec_scene` is a SIGIL HARNESS gap: the
  four act tests build the descriptor with single-file `lower_module` + a hand-listed
  `with_ambient` set that resolves no `use`; every other cross-seam name rides as an AS equ but
  these two are `pub comptime fn` and cannot. `sigil build` never saw it. Fix on sigil branch
  `parcel/sigil-red-closure` @ `75802f6a` (worktree `~/sonic_hacks/sigil-wt/sigil-red`, based on
  `7bc50e41`, local-only): parse the generated `effects_scenes.emp` and ride its items ambient.
  Queued as `CLOSURE-FIX`, lands after the §4 parcel (shares the landing checkout).
  (C) UNMASKED by (A): the `act_descriptor_port` pair then fail
  `section.bytes.len() == pins::ACT_DESCRIPTOR.plain_len` — emitted 0x27A, pinned 0x27C. The
  805370b1 refreeze put `OJZ_Sec0_Blocks` 2 bytes past the descriptor's end on an alignment
  boundary and `repin` measures start..next-label, so the SUCCESSOR'S PAD entered the pin.
  Same "gap between labels is an allotment" family as the bganim 2-byte slot, now in the pin
  generator. Queued as `REPIN-END`: repin measures the section's own end (preferred) or the
  gate tolerates trailing fill. `pins.rs` is not to be hand-edited for it.
  With (A) applied: 3846 / 3 / 4 (their run, clean aeon 415e0b6a); the three = the pair on (C)
  and `soundbankhead` on (B) below. Aeon CRCs unchanged in all four shapes.
  **Also: this correction has itself gone stale twice since it was written, which is the
  point.** It named a fresher tip in prose; two refreezes later (chains 165 and 166) that
  tip was wrong again, and a fresher-looking number reads as more trustworthy than the one
  it replaced. **No CRC is quoted here any more.** Read the TIP of `golden/provenance.toml`,
  never a number in this file — including a number in this file that presents itself as a
  correction to another number in this file.
  **Re-split 2026-08-26 with the aeon lane:** the two messages are two owners. `unknown
  function ojz_act1_act_default / ojz_act1_sec_scene` is aeon's (closure of their generated
  module under this harness). `section ojz_effects_editor_act1 has no region in the map`
  (`sigil-frontend-emp resolve/mod.rs:849`) cannot arise on the Frozen path `sigil build`
  takes — `emp_map_frozen` mints a region per present section — so it comes from whichever
  of the five tests walks the **PinnedBaked/registry** path, and is a SIGIL registry question
  (a registry row, or the test's synthetic entry not reaching the module). Aeon will NOT land
  a map row for it; do not ask them to. Queued here as `FIVE-REG`.
  **§3.5 confirmed live 2026-08-26:** aurora's first real scene save made `ojz_effects_editor_act1`
  emit, and `[map.order-undeclared]` named `EditorSceneBinding_OJZ_Act1_Sec0` — the BINDING TABLE,
  which changes with content. Aeon wrote that label as an interim `order` row (uncommitted, lands
  through their lane), to be replaced by the `section:ojz_effects_editor_act1` row. Queued as
  `SECTION-ROW` (S, after the §4 parcel); sigil specs it, aeon lands the map side.
  **The nightly source-gate lane does NOT cover these** — none of the five is in
  `SOURCE_GATES` (they are artifact-oracle gates, deliberately excluded), so the lane ran
  green at 05:17 while master was red. That exclusion was correct for byte-divergence red;
  it is wrong for a HARD ERROR like a missing map region, which no refreeze clears — check that, not just the
  totals: a reference gate that skips reports nothing and reads as coverage.
  **Reconcile the total against the tree, not against the last remembered bar:**
  `git grep -c '#\[test\]' HEAD -- '*.rs'` summed gives the declared count, and
  `passed + ignored` must equal it (3839 + 4 = 3843 here). Baseline arithmetic carried
  across branches measured on different reference trees does not reconcile and will
  invent a discrepancy that is not there. Never plain
  `cargo test`: without `--release` some gates are impractically slow, without
  `--workspace --no-fail-fast` a wedge or an early failure hides the rest of the
  result set. Report failures-first with explicit pass/fail counts; never
  `grep | head` test output (it buries FAILED lines).
  **SECTION-ROW LANDED both sides 2026-08-26:** sigil `8566f962` (`"section:<name>"` order rows;
  packet `2026-08-26-section-row-packet.md`), aeon `058ad606` (map.toml:118 now
  `"section:ojz_effects_editor_act1"`, byte-neutral, verified at aeon origin/master). The
  sigil fixture gate `both_spellings_of_the_section_row_build_the_same_rom` is direction-agnostic
  and ran 3/3 green against the landed map. `.aeon-landing` moved to `058ad606` (same ROM bytes
  as `0e34408d`).
- **THE PAIRING GATE IS ARMED — the ratchet disarmed at chain 167 and will not re-arm.**
  `aeon_dir_matches_the_provenance_tip` is now a live assertion in both modes, and its
  first real execution was exercised here both directions rather than assumed: pointed at
  `.aeon-landing` (aeon `893747f7`, the tip's own `aeon_rev`) it passes with **no ratchet
  line**; pointed at the genuinely stale `~/sonic_hacks/.sigil-portfix-aeon` (aeon
  `b08b35c0`) it FAILS, naming both revisions, the entry number, the parcel and the exact
  remedy. That second case is this morning's incident — a real check run against a stale
  reference tree, returning green — and it is now impossible to have silently. **A stale
  reference worktree is a hard failure from here, not a thing to notice**, so a run that
  refuses is telling you `AEON_DIR` is wrong, not that the parcel is.
- **`aeon_rev` answers WHICH TREE BUILT THESE BYTES — never WHICH PARCEL MOVED THEM.**
  `name` and `ab` are the attribution; `aeon_rev` is the provenance, and the two point at
  different commits whenever a freeze is retried on top of a later aeon revision. Chain
  entry #167 is the first where they diverge by construction: the showcase parcel moved the
  bytes at aeon `9dd52471`, but two freeze attempts failed on latent defects and the
  succeeding freeze runs from a zero-byte gate-fix commit stacked above it, which is what
  `aeon_rev` honestly records. **A reader who takes `aeon_rev` as attribution finds the
  bytes moving at a commit whose diff moves nothing**, and the record looks self-
  contradictory while being exactly right. The entry carries the attribution in words for
  that reason. Do not "simplify" these into one field, and do not backfill one from the
  other.
- **Pre-merge:** re-run the suite on the merged tree with `SIGIL_STRICT_GATE=1`, which
  turns reference-tree skips into failures so the port gates cannot silently skip.
- **A suite log does not name the tree it ran in — stamp it, and prove the landed code
  is IN it** *(2026-08-22, caught at a landing)*. Cargo prints no cwd, no branch and no
  HEAD, so a run launched from the wrong directory produces a log that is green,
  plausible, and about somebody else's branch. Precedent, this repo, this lane: a
  landing run of master reported **3857 passed / 0 failed / 2 ignored, exit 0** — a
  *higher* number than the bar, which reads as strictly better news. It was another
  agent's worktree: the log contained that agent's in-flight `m68k_capstone_differential`
  and contained **zero** occurrences of `version_provenance`, the parcel actually being
  landed. Nothing in the output said so.
  Two mechanisms, both cheap, and the second is the one that cannot be fooled:
  **(1)** stamp the log before cargo writes to it —
  `{ echo "### pwd=$(pwd)"; echo "### head=$(git rev-parse HEAD)"; echo "### branch=$(git branch --show-current)"; } > "$LOG"` then append the run;
  **(2)** `grep -c` the log for a test name **unique to the parcel being landed** and
  require ≥ 1. A landing whose own new tests do not appear in its own green log did not
  happen. This is why the reconcile-against-the-tree rule above is load-bearing rather
  than bookkeeping: `passed + ignored` equalling the declared count was the *only*
  signal that separated the bogus run from the real one (3857 + 2 = 3859 ≠ 3823; the
  correct re-run gave 3819 + 4 = 3823 exactly). Aggregate greens do not self-attribute.
  **Second precedent, 2026-08-22, and it is the nastier face: the numbers were EXACTLY
  RIGHT.** A parcel agent's first suite run came back `3821 / 0 / 4` — the bar of the day,
  to the test — stamped `branch=worktree-agent-a19f801c9ae459301 head=8884e255`. The work was
  still uncommitted and the worktree carried its auto-generated branch name, so the run had
  measured *master*, faithfully, and reported a number that reconciled perfectly against the
  declared count. Where the first precedent announced itself as suspiciously good news, this
  one announces nothing at all: a bar-matching green on a parcel that changes no test count
  is exactly what a correct run looks like. **The stamp is the only thing that caught it**,
  and it caught it on the `branch=` line, not the totals — which is the argument for stamping
  branch and head rather than just cwd. The agent renamed the branch, committed, and re-ran.
  Note the trigger: this is a hazard of the *deliver-on-a-named-branch* workflow itself, since
  an agent that has not yet renamed its worktree branch is in this state by default.
- **GENERAL-GROUNDS DOUBT IS NOT A FINDING — confidence is contagious and that is a
  defect, not calibration** *(2026-08-26; the aeon lane's own naming of something it did,
  kept because the honest half is the useful half)*. Having had two of its claims fail in
  one evening, that lane pre-emptively lowered its confidence in a **third, unrelated,
  sound** claim — "44 rows moved, 0 quantum changes" — on no evidence about that claim at
  all, and asked this lane not to spend an agent refuting it. The line was correct, and
  re-derivation upheld it. The cost of the discount is real and asymmetric: a *raised*
  doubt sends someone to re-measure something already true, and a lowered one lets a
  wrong thing through. **Discount a claim on evidence about THAT claim, never on the
  batting average of the claimant — including when the claimant is you.** The
  countermeasure is the same one that settled it: re-derive over a **different
  enumeration parameter** (here, sigil's own frozen tables vs. their measured deltas,
  each computed before seeing the other's number). Two derivations that share a parameter
  are one derivation run twice; two that do not are corroboration, and corroboration is
  what should move a confidence, not mood.
  **Its companion, from the same episode:** "every delta is a multiple of 16" is
  **strictly stronger** than "the addresses are 16-aligned", and the non-vacuity is
  carried entirely by the counterexamples (`Art_Tails` %16==10,
  `GameState_OJZScroll_Init` %16==4). A reader who skims it as *"well, everything is
  aligned anyway"* takes a proof for a truism. When a quantified claim's force lives in
  its counterexamples, state them beside it or the claim decays into a platitude the
  next time it is read.
- **Port work** follows the port loop (canonical:
  `docs/superpowers/notes/campaign-port-loop.md` — byte gate is step-1 only, then
  modernize/retrospect/back-prop/optimize until dry; dry is panel-adjudicated, not
  self-declared). New-era work takes the A/B/C lens panels
  (`docs/superpowers/notes/2026-08-03-era-lens-loop.md`).
- Unimplemented nice-to-haves go to `docs/superpowers/notes/campaign-gap-ledger.md`;
  twin scaffolding gets a kill condition in
  `docs/superpowers/notes/twin-scaffolding-kill-list.md`, same commit.
- Comments describe present-tense function, never change history. Commit with
  explicit paths only — never `git add -u` in this shared checkout — and check the
  branch before every commit.

### THE ZERO-`skip:` BAR CANNOT SEE 27 OF THIS REPO'S SKIPS (2026-08-27)

**Found by the aeon lane, off a stale pin they hit at their chain-171 landing.** The
full-suite bar above requires **zero `skip:` lines**. Twenty-seven early-return sites across
**nine** test files announce themselves as `eprintln!("skipping <gate> …")` instead, and
**`skipping` does not match `skip:`** — so those gates can no-op and still clear the bar,
which reads back as coverage. Measured, not reasoned: `grep -rn 'eprintln!("skipping' crates
--include='*.rs' | wc -l` = 27 across 9 files, against 146 sites using the `skip:` form.
Queued as `SKIP-TEXT-HOLE`; the fix is one spelling, and it is the bar that moves, not the
gates. **It is not only the hand-run bar: `scripts/nightly_source_gates.sh` gates on the same
blind `grep -q 'skip:'` and exits 2 on a hit**, so all 27 are invisible to the automated lane
as well — whose whole purpose is to notice what a byte-triggered ritual cannot.

**Their hypothesis for the specific incident was WRONG, and the correction matters more than
the hole.** They proposed the silent skip as the reason a stale `SFX_BODY_LEN` survived chain
170. It cannot be: `colinked_sfx_head_matches_the_reference_rom_slice_both_shapes` guards on
`if !strict_gate()`, and `strict_gate()` is `std::env::var("SIGIL_STRICT_GATE").is_ok()` —
under the landing bar that test **always runs and must have gone red**. So the skip text is a
real defect with a different victim, and this one needs its own answer.

**The actual answer, and it is a landing-lane gap rather than a false green: THIS LANE
PRODUCED NO CHAIN-170 GREEN.** No strict full-suite run was logged at chain 169 (`c8e87ecb`)
or chain 170 (`174f4300`); the newest suite log on this machine is
`~/sonic_hacks/.sigil-verify-d8748933.log`, stamped `head=d8748933`, which **predates both**.
Both chains were refreezes landed through the aeon-owned lane. Verified rather than inferred:
`SFX_BODY_LEN` was `2046` = `0x7FE` = `SFX_BANK_BLOB.plain_len` **at chain 169**, chain 170
grew that region to `0x8DA` (2266) and left the constant behind, and the assertion is
`out.body.len() == SFX_BODY_LEN` — 2266 against 2046, which no strict run survives.

**So the structural finding is: a refreeze can land in this repo without sigil's strict suite
ever running.** That is what let a pin go stale for a whole chain with nothing red anywhere.
Queued as `REFREEZE-NEEDS-STRICT`; aeon has accepted it as a standing commitment banked in
their own repo.

**CORRECTED the same night, by the aeon lane, and the corrected cause is sharper than the
one first written here.** This section originally said the landing rule named *who lands* but
not *that the bar runs*. **That was wrong: their rule did require the full sigil suite.** It
spelled the command as a bare `cargo test --release --workspace --no-fail-fast` with **no
`SIGIL_STRICT_GATE=1`** — so a session following it faithfully ran a suite in which every
`strict_gate()`-guarded port and co-link gate early-returns. The rule was present and the one
token that gave it force was absent. Fixed on their side at aeon `origin/master`, verified
here by reading their file at that revision rather than taking the report.
**The generalisation, and it is why this is banked rather than closed:** a procedure can be
complete in its steps and inert in its spelling, and the inert version passes every review a
reader gives it — nobody audits a command line for a missing env var. Note the two findings
compound: the missing run is why nothing was red, and the skip-text hole is why a future run
might not be red either.

**Their `+0x10` vs `+0x12` derivation was re-checked here against the pins themselves and
holds.** `SFX_BANK_BLOB` grew `0x12` while the assembled total moved `0x10`, because the
16-alignment pad before the next region absorbed 2: `0xA3B20 + 0x8DA = 0xA43FA` (6 to
`0xA4400`), `0xA3B20 + 0x8EC = 0xA440C` (4 to `0xA4410`). Corroborated over a different
parameter than their arithmetic — `EPILOGUE` moved `0xA5C80 → 0xA5C90`, exactly `+0x10`.


## The source-gate lane

`scripts/nightly_source_gates.sh`, fired by the `systemd --user` timer
`sigil-source-gates.timer` at 05:17 daily. It runs the **gates whose inputs are aeon
SOURCE** — the warn-tier corpus and its neighbours, named one per line in the
hand-maintained `SOURCE_GATES` array; read the count out of the array rather than out of
this sentence, which is how it went stale the first time — against detached master-tip
checkouts of *both* repos, at `~/sonic_hacks/.sigil-source-gates` and
`~/sonic_hacks/.aeon-sigil-gates`. Both live outside their repo roots: a worktree under
the aeon root double-counts every module in that repo's `tools/emp_helper_closure.py`
tree scan.

**Why it exists.** These gates read aeon source, but nothing *ran* them against a fresh
aeon tip except a refreeze — and a refreeze happens only when ROM bytes move. Six
consecutive zero-byte aeon parcels hid a real `layout.odd-field` finding for a day
(`docs/superpowers/notes/2026-08-22-warn-tier-drift-open.md`). A trigger keyed to byte
movement is structurally blind to a source-derived lint set moving, so this one is a
clock.

- It compares **no byte against a committed artifact**. The region-diff gates, the
  golden-CRC gates and `pins_rs_is_current` read a built ROM or compare against sigil's
  frozen goldens, and they already have the right trigger — aeon's byte-identity ritual,
  which fires exactly when bytes move. The exclusion is named in the script, not silent,
  and it is **counted**: the verdict line prints how many aeon-reading gates were skipped
  as artifact-lane (82 at `a886fd2b`; read the line, not this number) so "skipped" cannot
  be read as "green".
- It **does build every shipped shape from source**: `corpus_builds`
  (`crates/sigil-cli/tests/corpus_builds.rs`) is the lane's **brick witness**. For each
  shape in `native::shipped_shapes()` — the one table the byte gates enumerate — the entry
  `sigil build` reaches (`build_rom_chained_with_listing`) must return `Ok` with zero
  error-level diagnostics; no byte is compared to anything, so a byte-moving aeon parcel
  leaves it green while a BRICK (`[map.order-undeclared]`, `section … has no region in
  the map`, colliding pins, an unknown function in a reached module) turns it red naming
  every bricked shape. The two failure kinds the skipped artifact gates conflate — CRC
  drift (refreeze owns it) and a brick (nobody's ritual clears it) — are therefore split:
  the verdict line names `corpus_builds` as the brick witness, and a red whose output
  carries its phrase is announced as `BUILD BRICK`. Its second test injects the
  2026-08-26 live brick into a `shadow_aeon_tree` copy (the `section:ojz_effects_editor_act1`
  map row deleted) and requires the same checker to name the shape and section, so the
  detector is proven live on every run. Two other `SOURCE_GATES` entries
  (`m68k_roundtrip_stream`, `m68k_capstone_stream`) also build all seven shapes and would
  panic on a brick — at the first bricked shape, under a roundtrip test's name; the witness
  exists so the brick is measured as itself, on every shape, and read back by the verdict.
- **Every new `crates/*/tests/*.rs` that reads the aeon tree must be classified the day it
  lands.** `derived_layout` (master `4f303b0d`) was source-only, not in `SOURCE_GATES`, and
  named no artifact: the audit would have exited 2 — the whole lane dark — at the next
  05:17. Replaying the audit against the branch is one loop and it is the only thing that
  sees this before the timer does.
- Exit `1` = a gate failed, `2` = the lane could not run; both `notify-send`. "Could not
  run" includes fewer gates executing than were named, zero tests executed, and any
  `skip:` line surviving `SIGIL_STRICT_GATE=1`.
- `SIGIL_SOURCE_GATES_REF` / `AEON_SOURCE_GATES_REF` put a branch or an old SHA through
  the real lane. The timer never sets them.
- **`~/sonic_hacks/.aeon-sigil-gates` is source-only by construction — never point an
  artifact-dependent run at it.** Each lane run deletes the generated inputs *and* any
  built ROM or listing, so a full-suite run sharing that tree loses its ROMs mid-flight
  and reports ~127 `reference missing: …/s4.bin` failures that read exactly like a
  golden divergence. Build a separate checkout for the artifact gates.
- The units are committed at `scripts/systemd/` and installed by copying them to
  `~/.config/systemd/user/` — a `--user` unit lives outside every repo, so an
  enabled-but-uncommitted timer is invisible to every session that did not install it.
- Do **not** touch `aeon-effects-gates.{service,timer}`; that lane is aeon's, and it
  fires at 04:17 so the two do not contend.

**⚠ The lane's self-audit reads PROSE, and a doc comment can take the whole lane down.**
Before running, the script classifies every `crates/*/tests/*.rs` matching
`AEON_DIR|aeon_dir|reference_tree|--aeon` as either in `SOURCE_GATES` or derivably
artifact-dependent, and exits `2` — the entire nightly backstop dark, reporting nothing —
if any file is neither. That grep cannot tell a *use* from a *mention*: a new test whose
header says "takes no `AEON_DIR`" matches on the disclaimer and is unclassifiable by
construction. Caught on `feat/version-provenance` before landing (replaying the audit gave
`unclassified=1 [version_provenance]` at the first delivery, `0` after); the fix was to
describe those inputs without naming the identifiers, and to say why in the file so the
next author does not re-arm it. **Replay the audit against any branch adding a
`crates/*/tests/*.rs` file** — it costs one loop and the failure is otherwise invisible
until 05:17.
**⚠ `gates=N` is NOT a number this script prints — it is a reconstruction, and two lanes
reconstructed different quantities under the same label** *(caught 2026-08-22, at a landing)*.
The script emits no `gates=` line at all. This document recorded `gates=35 unclassified=0`
(the size of the `SOURCE_GATES` array); a parcel agent replaying the same audit reported
`gates=116 unclassified=0` (the count of `crates/*/tests/*.rs` files the selector SCANS).
**Both measurements were correct and they are not the same quantity** — on the merged tree:
`SOURCE_GATES=35 scanned=116 unclassified=0`. Nothing was wrong except the label, which is
precisely why it survived: two different numbers agreeing on the part that matters look like
a discrepancy in the part that does not. **The load-bearing figure is `unclassified`** — it
is the only one the script acts on (non-zero ⇒ exit 2, whole lane dark). When reporting an
audit replay, name the quantity: `SOURCE_GATES=<n> scanned=<n> unclassified=<n>`, never a
bare `gates=`.

**Left open, deliberately:** tightening the detector to match code uses rather than any
occurrence would disarm the trap, but it also loosens what the lane considers
unclassified, and a genuinely aeon-reading gate escaping silently is the failure this
audit exists to prevent. Soundness-reducing, so it is not a parcel-local call — it needs
its own ruling.

**Adjudicating a warn-tier firing.** A new firing goes into `CORPUS_OPEN_FINDINGS`
(`crates/sigil-cli/tests/warn_tier_corpus.rs`), **not** into `WARN_ID_BASELINE`. The
baseline admits an id everywhere and any number of times; the register pins
`(shape, id, file, symbol)` with a count, and requires an owner, an anchor and a kill
condition per row. Anchors are symbols and paths, never line numbers — a register entry
outlives what it points at, so a coordinate in one rots. A row leaves the register in
either direction: fixed, or ruled deliberate and promoted into the baseline citing the
ruling. Each row's age prints on every lane run.

## Worktree and environment quirks

- **Worktrees are agent-isolated but the registry is repo-global.** Every session's
  worktrees live under `.worktrees/` off this main checkout and share one
  `git worktree list`. Check the list before adding (names collide across sessions),
  and prune stale entries — ~18 accumulate fast on a busy day.
- **Port tests need the aeon tree.** `AEON_DIR` selects it
  (`crates/sigil-harness/src/test_support.rs::aeon_dir`, default
  `/home/volence/sonic_hacks/aeon`); when the referenced paths are absent the gates
  skip green unless `SIGIL_STRICT_GATE=1`. A worktree agent running port tests must
  be told which aeon tree to point at — and beware the aeon-side worktree trap: a
  fresh aeon worktree missing its gitignored `games/sonic4/data/editor/` builds a
  padded, wrong ROM.
- **The `*_port` cross-seam trap:** the standalone port oracles lower ONE aeon module
  against a hand-picked dep list, so they see neither the whole-program contract bind
  (which resolves `Game.MEMBER`) nor `scene_dsl.emp` (which declares the `CAP_*`
  bits). A new `Game.*` / `CAP_*` reference in an engine module therefore breaks ~9
  port tests silently (`unknown name Game.SCANLINE_CAPS` at lower time). The fix
  pattern is `crates/sigil-harness/src/test_support.rs` §4: synthesize the interface
  + `implement` and the `pub const CAP_*` block, both **derived from the aeon tree at
  test runtime** via `emp_const_rhs` / `emp_const_literal` — never a copied literal.
- **Never build in the scratchpad/tmp** — `/tmp` is tmpfs; a cargo build there wedges
  the shell. Set `CARGO_TARGET_DIR` to disk for any out-of-tree build.
- **A LANDING RUN AGAINST THE OWNER'S LIVE AEON TREE PRODUCES PHANTOM FAILURES, and this
  overseer proved it the expensive way (2026-08-24).** The rule below is not advice. A
  full-suite landing run pointed at `/home/volence/sonic_hacks/aeon` returned **six**
  failures where the parcel's own agent had reported five; the extra was
  `soundbankhead_matches_reference`, and the parcel had touched `seam1.rs` — the sound
  seam — so the coincidence looked damning. It reproduced twice, including with the test
  binary run alone. A control at unmodified master showed it passing, which read as
  attribution. **It was transient**: four consecutive re-runs on the merged tree passed,
  single-threaded included, and a clean re-run of the whole suite returned the agent's
  5 exactly. Nothing was wrong with the parcel; the tree had artifacts in flux underneath
  a run that takes minutes. **The cost was an hour and a nearly-shipped wrong attribution
  against an agent that had done nothing wrong.** Steady-state disagreement between two
  runs over the same tree is the tell — re-run before attributing, and prefer a clean
  worktree of a committed SHA, which is what the next rule already said.
- **The main aeon checkout carries the owner's live editor content edits** (collision
  bins, regenerated act-pool pages under `games/sonic4/data/`) for hours or days at a
  time — never gate a green on that tree cleaning up. A strict-gate or landing run
  that matters points `AEON_DIR` at a CLEAN WORKTREE of a committed aeon SHA, with all
  four shapes built there first (repin resolves but does not generate). File seeding is
  RETIRED (`aeon/tools/seed-worktree.sh` is a copies-nothing stub; the OJZ tree and
  collision tables are committed, generated dirs rebuild via build.sh). **A fresh aeon
  reference worktree needs NO `sigil`/`skdisasm` symlinks and no paired sigil worktree
  — and this paragraph asserted the opposite for some time while the quality-bar section
  above asserted the truth, so a cold boot could read either.** Settled in the field
  2026-08-26: the aeon lane rebuilt `.aeon-landing` at `c3f5cbe0` with no symlinks of any
  kind and all four shapes built and reproduced their chain-168 CRCs exactly. The symlink
  requirement is real only for a worktree that must resolve the emp-helper closure — an
  agent's *development* tree — never for a reference checkout that only has to build.
  Seeding them into a reference tree is actively harmful: it kills `section_row_fixture`'s
  tree mirror with *"the source path is neither a regular file nor a symlink to a regular
  file"*, three gates red for a reason unrelated to whatever is under test.
  Verify the built ROMs against `golden/provenance.toml` (CRC32+size) before trusting
  the worktree. Mid-brushstroke aeon
  edits flipping sigil port-gate results is environmental, not signal; the tell is
  broad `*_port` region-diff failures at embedded addresses plus
  `repin_pins::pins_rs_is_current` failing identically on sigil master.

## Queue

The standing sigil-native arc is the **`.emp` language work (Spec 2)** — specs in
`empyrean/docs/SIGIL_*.md`. The whole sound stack is sigil-native, the language round
+ §17 optimization arc + conversion tail are done, and the map drives the build.

### SIGIL-DECOUPLE — the owner ruled 2026-08-26, in this lane's session: follow aeon's plan

Asked whether to adopt the shape aeon nominated, his answer was to follow it. **The
authoritative text is aeon's, not a paraphrase here**: `docs/DEFERRED_WORK.md` at aeon
`822c382a`, verified reachable from their `origin/master` at read time. Four steps in
order — (1) cut the golden cord, sigil vendors a PINNED aeon source snapshot as its corpus
and drift detection moves to a nightly non-blocking job; (2) placement authority comes home
to aeon, anchors declared in `map.toml` and everything else placed fresh; (3) retire
`repin`/`pins.rs` from the landing path into an internal regression tool over sigil's own
corpus; (4) archive the byte-identical certification as a dated historical result.
**Sequencing is theirs and it is step 2 FIRST**, after the showcase — which has landed
(chain 167).

**What the coupling actually buys, measured before the ruling rather than assumed**, because
step 4 will want it and because a future session will otherwise re-litigate this from scratch.
Two families, and only one of them is liveness:

- **The goldens (byte-identity) catch things nothing else does, and they SURVIVE step 1** —
  they simply describe a pinned corpus. Instance with teeth: the one-sided
  `Player_SensorPair` push/pop narrowing is a real runtime bug (wrong angle delivered) that
  left contract closure green, fired no warn tier, and is invisible to every static analysis
  here because nothing models 68k stack byte lanes; the ROM diverging from the frozen golden
  was the only signal. Also the `test_emitter` end-anchor error (caught as a window length
  mismatch) and the `boot.asm` −0xA base slide, which surfaced hardcoded-address fixtures the
  design-gate census had not listed.
- **Liveness catches exactly one family: aeon's newest source hitting a sigil capability or
  measurement gap** — a brick or a mis-measure, never a byte regression. All three 2026-08-26
  instances are that shape: the missing map region for `ojz_effects_editor_act1`; the `repin`
  pin defect where the successor's alignment pad entered `ACT_DESCRIPTOR` (0x27C pinned vs
  0x27A real); and BGROOM-3's `abs.w`, whose real cause was a collision-fallback scratch slot
  aliasing zero. **That family is already covered without the coupling** — the nightly
  source-gate lane and `corpus_builds`' brick witness build every shipped shape from live
  aeon tip and block nothing, which is the same shape step 1's nightly drift job takes.
- **And the coupling is not a general safety net, so do not price it as one:** the `×26`
  stride bug sat green because both twins carried the identical wrong shift decomposition
  (the gate proves twin-agreement, not correctness), and `[layout.odd-field]` drifted four
  days through six zero-byte parcels whose CRCs were verified at every landing.

**The residual cost, stated so nobody discovers it later:** byte coverage AGES. A construct
aeon writes after the snapshot has no golden, so mis-placing it surfaces as a brick rather
than a byte diff, until the snapshot is bumped on sigil's cadence. Aeon stated the matching
cost to the owner before his yes — assembler regressions surface nightly rather than at the
next aeon landing.

**The gate this lane holds on step 1, and it is aeon's own sentence rather than an addition:**
*"Every constraint the frozen tables encode today must be recaptured as an explicit rule
BEFORE the tables stop being authority, or it silently stops being enforced."* Treat that as a
precondition with a deliverable, not a caution. **Enumerate by what TOUCHES placement, never
by what the frozen table lists** (bar 8) — the table is the artifact whose authority is being
retired, so taking it as the enumeration is the shared-frame failure aimed at ourselves.
BGROOM-2's leftovers are the same territory and should be read as part of this, not beside it.

**STATE AS OF 2026-08-26 EVENING — a fresh boot inherits exactly this.**

- **The constraint inventory is COMPLETE and pushed**:
  `docs/superpowers/notes/2026-08-26-placement-constraint-inventory.md`. Eight rows, no
  row left named-but-unlocated. R7 is the one to read first — alignment is *inferred* from
  each pin's own address (`native::packed_align_of` = largest power of two dividing the
  frozen base), so a repin changes a section's alignment with no alignment code changing,
  and only two labels are guarded against the silent-audio consequence.
- **Aeon's `parcel/rom-relayout` is IN FLIGHT and holds a sigil branch of the same name.**
  Their agent, their landing, through the aeon lane. **This lane holds no refreeze until
  that pair lands**, and owes it a review of the sigil half when they send the pushed SHAs.
  Their agreed landing requirement includes a per-shape old/new base + old/new quantum
  table for every moved row, and seam2 predicted-vs-`.lst` bases for every sound label.
- **Island-order piece (1) LANDED** at `1a03c75c`: the MDDBG blob-end guard is proven to
  fire, red-first against two mutants. **Piece (2) — the per-shape non-vacuous-arm
  assertion that closes the fail-open on a missing `ErrorHandlerBlob` label — is QUEUED
  BEHIND THAT PAIR** because it touches `native.rs`. Bar moved 3939 → 3943 declared.
- **The 79 pad-sweeping region ends convert before the flip** and are this lane's to
  sequence; they touch the aeon-owned lane files, so they wait for the pair too.
- **Ruled jointly with aeon and NOT to be re-opened unilaterally:** when the rules are
  written, declare each section's alignment as what its CONTENT needs; never transcribe
  the accidental quanta. Anything that held only by accident stops holding once, visibly.

**Not started, and two triggers are not this lane's:** the hub declares the project id (it has
said one word from the owner is enough, the yes being banked at aeon `822c382a`), and step 2 is
aeon's ROM-RELAYOUT, which their queue holds for the owner's go.

**Current state (2026-08-22, master `8884e255`, pushed — `origin/master` verified equal by
`git ls-remote origin refs/heads/master`, not by the local tracking ref).** The two parcels
below merged at `cba0a0bc`, which master has since moved past; read this heading for where
master IS, and the per-parcel SHAs for where each landed. Both were
sigil-internal (neither touches `golden/`, `pins.rs` or `repin.toml`, so neither needed
aeon-lane sequencing):

- **`feat/game-defines`** — the aeon `emp_defines` ask, with both lens latents closed. A
  game-declared `[defines]` row now joins the blind-arm polarity net via
  `audit_game_declared_polarity` (exempted-and-named, with reason-less and stale
  exemptions both rejected), and `shape_defines`' NotFound tolerance is gone outright.
  Value-only enforcement at parse time was considered and **rejected**: it would refuse
  the one safe game-declared toggle (a row the two games declare at opposite values —
  exactly what the polarity net wants) while still missing `if CAPS == 20` on a
  non-boolean row. The shipped CLI's failure surface moves: `--report ram` and
  `--report contracts` now exit on a missing `games/<g>/map.toml` where they proceeded on
  built-in rows; a full `sigil build` was already fatal there, the diagnostic just moves
  earlier and names the config instead of the placement.
- **`feat/warn-tier-ungate`** — the source-gate lane (see that section above), the
  `CORPUS_OPEN_FINDINGS` register, and `sigil-source-gates.timer`.

**The reference tree for any artifact-dependent run is aeon master, built** — at the
revision the `provenance.toml` **tip** pairs with, derived at read time per the quality-bar
section, never from a CRC written in this file. A non-tip entry read as the tip once cost this
session a false "corroborated" in a review, and there are 166 entries to pick the wrong one
from. `~/sonic_hacks/.aeon-landing` is a built checkout at that
revision **(it was found ABSENT on 2026-08-26 and re-created as an aeon worktree at
`415e0b6a`; re-creating one has two lived hazards, from the aeon lane: a fresh checkout gives
`project.json` a new mtime so `tools/level_staleness.py` hard-fails BEFORE any ROM is emitted —
run `tools/regenerate-level.sh` first and discard its `DONOR_PROVENANCE.json` churn; and `rm -f`
all four ROMs before EACH build, because a build that stops at the staleness gate leaves
leftovers whose CRCs match the pins perfectly. Export `AEON_SKDISASM_DIR`)**; `~/sonic_hacks/.aeon-sigil-gates` is **source-only by construction** and must
never be pointed at an artifact-dependent run.

Previously landed the same day: **const-arity** — a
typed comptime `const` literal is now array-arity checked at elaboration (the length
check lived only in the byte-emission path, which a `const` never reaches, so a
wrong-shaped constant compiled clean); and the **Oracle listing gate's `ORACLE_DIR`
default**, which had gone stale at the oracle/oracle-old rename and made
`SIGIL_STRICT_GATE=1` — the pre-merge bar this very file names — unsatisfiable.

Also landed today: **`feat/m68k-roundtrip`** (an m68k decoder mirror, a round-trip pass
over every shipped shape's emitted stream, and an opcode sweep) — carrying the real ISA
fix it surfaced: `TST` takes `DATA_ALTERABLE` on the MC68000, not `DATA`, so the old row
admitted nine words the hardware traps. Latent, never emitted, derived four ways. Its
packet's soundness claim was corrected rather than kept — the sweep's oracle is
`encode()`, so it proves decoder-subset-of-encoder and **cannot see a defect both halves
share**; `TST` was exactly that, live.

**Front of the queue, in order:**

1. ~~**A provenance witness for the shared binary**~~ — **LANDED** at `9c08f2a5`
   (`feat/version-provenance`). `sigil --version` / `-V` reports the revision, branch,
   commit date, tree state and source dir the binary was built from. Verified firsthand at
   review: an empty commit touching zero source files re-runs `build.rs` and the binary
   reports the new revision (1.25s); a detached checkout reports `branch: detached`, not a
   branch called `HEAD`. Cargo tracks `HEAD`, `<common-dir>/refs` and `packed-refs` — the
   git-dir/common-dir split is load-bearing, since from a linked worktree cargo must be
   given the *main* checkout's common dir. **Tree dirtiness is NOT trackable** — cargo has
   no trigger for uncommitted edits, and the alternatives were priced and refused (a
   repo-root directory trigger never reaches a fixed point because `target/` is at the
   root; an unconditional rerun costs 13.2 s wall / 167 s CPU per invocation to refresh
   one boolean). The banner discloses that limit in place and names the command that
   settles it, and a test asserts the disclosure so it cannot be quietly tidied away.
   **The consumer side is still open and is where the original incident actually bit:**
   aeon's build invokes `sigil` without asking what it is. Making the build refuse or warn
   when the assembler's revision ≠ the tree being assembled is an **aeon-side** parcel.
   Not covered by this witness at all: rustc version and build profile.
2. ~~**A Capstone differential as a permanent gate**~~ — **LANDED** at `aafe612a`
   (`feat/capstone-differential`). Two gates: `m68k_capstone_differential` (the 65,536-word
   space, 0.5 s, no aeon dependency, default suite) and `m68k_capstone_stream` (all seven
   shipped shapes' emitted stream, default suite **and** the nightly source-gate lane).
   **Non-circularity verified firsthand, not asserted:** under MUTANT T — `Tst` widened to
   `DATA` on encoder *and* decoder together — the pre-existing `m68k_opcode_sweep` stays
   GREEN while the capstone gate goes RED naming `4A3A`/`4A7A`/`4ABA` by word. Loud on
   unmeasurable both directions: capstone absent skips and passes; capstone absent under
   `SIGIL_STRICT_GATE=1` fails naming the reason.
   Comparison is on a normalised abstract form (legality, consumed length, mnemonic family,
   size where capstone reports one, ordered canonical operands) — **never text**, which
   measures spelling. Capstone's *structured* operand detail is deliberately unused: in
   5.0.7 its m68k backend leaves `mem.base_reg` invalid for `(An)/(An)+/-(An)` and
   `mem.disp` zero for absolutes, so a structural compare would measure capstone's own bugs.
   Both named hypotheses re-derived and both turned out **broader than stated**: the `$FF`
   long-displacement escape is a 68020 addition capstone applies inside its 000 mode (16
   words), and the bit-op sizing class is *every* bit-op form, not just dynamic `btst` —
   capstone is wrong in both directions and it costs a length disagreement too. Five
   exclusions total, each value-aware, each asserting its derived class size exactly; that
   size assertion **fired twice during development and was right both times**. Residual
   after normalisation and exclusion: zero, on all three corpora.
   Three findings went to the gap ledger rather than being excluded or papered over — see
   its tail: `encode_bit`'s bit-number field (152 words, a defect **both halves share**,
   byte-neutral to fix and provably so), the odd `.s` branch displacement latent, and the
   unpinned oracle version.
3. ~~**An alignment attribute / even-offset assertion**~~ — **LANDED** at `6fae4d6a`
   (`feat/field-align`). Shipped as `sc_mask_raw: i16 (align: 2)` — error-tier, per-field,
   opt-in, `N` restricted to powers of two, `(align: 1)` the identity and a per-*field*
   `[layout.odd-field]` opt-out (finer than the existing module-wide `@allow`).
   **The spelling is `(align: N)` and NOT `@align(N)`, and this is the parcel's real
   finding:** `@align(N)` is already taken on `vars` region fields, where it *moves* the
   allocation cursor (`parser.rs::opt_align` → `lower/regions.rs::align_to`); D2.29 says so
   outright — *"`vars` regions keep `@align(N)` on fields (reserved space, no bytes) —
   different mechanism, deliberately different spelling."* One spelling with two opposite
   meanings (assert vs move) is a worse trap than the one being closed. `(align: N)` instead
   joins the paren-attribute family, every existing member of which verifies a placement
   rather than choosing one. `@align(N)` written on a struct field is refused **by name**
   with a teaching diagnostic, not mis-parsed as an offset expr opening with the identifier
   `align`.
   **The auto-padding tier was never an open call, and this overseer was wrong to dispatch
   it as one.** Spec 2 §4.3 already rules it, verbatim: *"The compiler never inserts
   alignment or padding — Aeon runs `padding off` globally and hand-pads; an auto-aligning
   struct would silently break byte-exact ports."* Shipping it is a spec amendment, not a
   feature. Read the spec before framing settled law as a design question.
   Verified at landing, not accepted: declared `#[test]` = **3839** on the merged tree,
   run gave **3835 passed / 0 failed / 4 ignored**, `3835 + 4 = 3839` reconciling exactly;
   zero `skip:` lines under `SIGIL_STRICT_GATE=1`; log stamped `head=6fae4d6a branch=master
   AEON_DIR=.aeon-landing@1ee8f8e6`; source-gate self-audit `gates=35 unclassified=0`.
   Delta vs the old 3821/0/4 bar is exactly +14, the parcel's own tests — note that only 13
   carry the `field_align` prefix; the 14th is `vars_form_align_on_a_struct_field_is_refused_by_name`,
   so a `grep -c field_align` under-counts by one and looks like a discrepancy that is not there.
   **The bar moves to 3835 / 0 / 4.**
   **Why error tier and not another warning — and this travels as a POST-MORTEM, not as
   foresight** *(framing at the aeon lane's explicit request, and they are right that it
   matters)*: `[layout.odd-field]` **did** fire across the 2026-08-18→08-22 drift and was
   swallowed by a warning baseline nobody re-read, through six consecutive zero-byte parcels
   whose CRCs were verified and cited at every landing. A lint that fires into a baseline is
   not a guard. That lesson was paid for by the lane that MISSED it, not spotted by the lane
   that saw it — record it that way, because the next person needs to know it is easy to miss.
   Its packet is `docs/superpowers/notes/2026-08-22-field-align-packet.md`; six new
   diagnostic strings are a cross-repo interface (aeon fixtures assert exact text),
   enumerated in its §10.
   The superseded queue text, kept because it is still the best statement of WHY: the
   class-level fix for the
   odd-field finding: today a struct wanting even-aligned members can only say so by
   hand-counting bytes into a pad, and the pad goes stale silently. Aeon's own fix has
   LANDED (`9a718f74`, `ensure(offsetof(Scene, sc_mask_raw) % 2 == 0, …)`), so this now
   has a live subject to retire rather than a hypothetical one.
4. ~~**`pad_to(N)` — a derived-width struct pad**~~ — **LANDED** at `ffa7bdb8`
   (`feat/struct-pad`), implementing Spec 2 §4.3.1 / D2.38 at empyrean `2000b5ca`. Both
   spellings ship per the owner's `both` ruling (`d-13`): `pad_to(N)` derives the filler width
   from the next field's target offset, `pad(N)` reserves exactly N, and
   `[layout.pad-hand-counted]` (default-on warning) names the mistake and prints the exact
   `pad_to` line when a fixed pad stands immediately before a field asserting `(align: N)`.
   **The design call:** pads are held BESIDE the fields (`StructDecl::pads`, each anchored by
   the index of the field it precedes), not interleaved into one member list. A pad has no
   name, so struct literals, `offsetof`, the emitted-shape check, `resolve_bare_window` and the
   harness's offset harvest all read `fields` unchanged and are correct **by construction**
   rather than by remembering to filter — and forgetting to filter is silent. Only the two
   consumers of a pad's WIDTH change: the layout walk, and the emission that follows its
   offsets. Emission derives the `$00` runs from the layout's offsets, never from what the
   fields emitted, so a field that lowers to nothing cannot slide the pad.
   **Byte-identity is by construction too, and worth stating so nobody re-derives it:** a gap
   between two consecutive fields can ONLY be a pad, because `@ offset` on a struct field is an
   *assertion* checked against the declaration-order offset (`check_struct_field_offsets`) and
   never a placement. No corpus struct carries a pad line, so zero ROM bytes moved in all four
   shapes; nothing under `golden/`, `pins.rs` or `repin.toml` was touched and the parcel stayed
   sigil-internal.
   **Four deviations from the spec's literal rendering, all RATIFIED by this overseer** (bar 7 —
   ruled explicitly, not absorbed): (1) the lint tag is inline in the message, because
   `sigil_span::Diagnostic` has no code field and every other tagged diagnostic here embeds it —
   the spec's sentence survives verbatim as a substring; (2) `[layout.pad-count]` tags both
   spellings, only the noun varying, per the derivation table; (3) the non-int case interpolates
   the value's type name, a rendering the spec does not spell; (4) **`[layout.pad-hand-counted]`
   withdraws when the neighbouring `(align:)` is itself refused** — the fix-it promises "the
   assertion still proves it" and a refused assertion proves nothing, so firing there is false
   advice, and the lint must evaluate the same expression `check_struct_field_align` does, which
   without the withdrawal reports an unresolvable name twice.
   **(4) IS NOW SPEC** — the hub appended it to §4.3.1's lint paragraph in this lane's reasoning
   and recorded the other three as implementation's, at empyrean `08ce4c1` (verified reachable
   from their `origin/main` here at read time, and the clause read back rather than taken).
   D2.38 is marked shipped at `ffa7bdb8`.
   **The owed align debt is CLOSED in the same parcel:** the six `(align: N)` coverage claims
   were re-measured at `67575c3c` rather than taken (all six held); #2 and #3 now assert the
   whole string, #4/#5/#6 gained tests, and §4.3's Scope clause gained
   `field_align_does_not_propagate_into_a_nested_structs_own_fields`, built so the nested struct
   is internally clean and lands at outer offset 1 — a propagating check must report, and must
   not. **`ALIGN-STRINGS` is CLOSED**: the hub verified the four test names on this repo's origin
   and re-grepped aeon for the three pad tags themselves rather than taking the report, then
   strengthened D2.37's "what actually pins them" clause to say all six strings are pinned
   (empyrean `08ce4c1`).
   **§4.3.1 shipped carrying the same false "cross-repo interface" claim D2.37 had just been
   corrected of** — the sentence rode in on a verbatim lift from this lane's draft. Now fixed to
   *"these strings are sigil's; no consumer outside sigil asserts them"*, with a parenthetical
   recording how it happened. **The durable lesson, and the hub named it against itself:** a
   verbatim lift is reviewed for FIDELITY TO THE SOURCE, and that review cannot see a claim that
   is false in the source. Both lanes read it twice and neither was reading for truth. When
   lifting text that carries factual claims, the claims need their own pass.
   **22 poisons, 22 red**, one of which caught a green leak in the parcel's OWN new assertion:
   `final_pad_to_below_the_cursor_names_the_end_of_the_struct` asserted the sentence but not the
   `[layout.pad-overflow]` tag, so a poison left it green while its sibling went red. That is bar
   2 firing on the parcel that wrote it.
   Packet: `docs/superpowers/notes/2026-08-26-pad-packet.md`. **The bar moves to 3928 / 0 / 4.**
   *(Superseded queue text, kept for the reasoning — NOT a queue item; the ratification
   history that led to the landing above.)*
   **RULED ADOPT, 2026-08-22 — but read the provenance before acting on it.** The ruling is the **empyrean overseer's**, made under a
   delegation from the owner they report in his words (*"Sure I'll go with what your decision
   is."*). **This lane did not witness that utterance**; it is a peer's report of an owner
   grant, recorded as such rather than as an owner ruling, and it is reversible by him or by
   evidence. Verdict: **adopt `pad_to(N)`, and `(align: N)` stays MANDATORY alongside it —
   never made redundant by the derived width.** The reasoning, which matters more than the
   verdict: the decisive objection is **silence, not authorship** — auto-derivation converts
   a *detected* defect class into an *absorbed* one, so the assertion is what makes derivation
   safe and therefore cannot be derivation's casualty. That argument bites `pad_to(N)` too and
   not only blanket auto-padding, which is why this is a **pair, not a replacement**.
   **Falsifier they stated, so it is not a preference:** if keeping both makes the common case
   actively worse to write — an author supplying an alignment *and* a pad marker where one
   number used to do — the ergonomics argument wins and the cost should be brought back.
   **SEQUENCING, and this lane is holding it: SPEC TEXT FIRST.** Not built yet, deliberately.
   `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` is the language contract and empyrean's to land;
   implementing before the text exists means the implementation *defines* the spec, which is
   the protocol's own "a spec ratifying whatever shipped" failure. Sigil has offered to draft
   the §4.3 text for them to land.
   **Scope note that outlives this item:** the ruling authorizes **this construct**, not a
   general licence to add `.emp` surface. The next one parks the same way — see the autonomy-
   scope section.
   *(Superseded framing, kept for the reasoning:)* ratified in
   principle by this overseer, not built. The `(align: N)` assertion above **guards** the
   stale constant; it does not remove it. `sc_pad_5D`'s width is still a function of every
   field above it and a human still re-counts it by hand each time the guard fires. The
   proposal (packet §4): give structs the `pad(N)` field form that `vars` region bodies
   already have (`parser.rs::region_field`), plus a `pad_to(N)` sibling whose width the
   engine computes. **This does not need a §4.3 amendment** — the bytes sit on a line the
   author wrote, in declaration order; the compiler sizes a declared pad rather than
   inserting an undeclared one. Nothing appears in a struct lacking such a line, so no
   existing struct changes size and byte-identity is untouched. It kills the staling
   constant outright, with `(align: N)` left as the independent proof that the derivation
   did what was intended. Needs spec text in `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` first.
   **The strongest argument for keeping it parked is not the one this overseer led with**
   (*empyrean lane, 2026-08-22*). "It would be the first construct moving bytes the author
   did not write" is true but second-order. The real cost: **auto-pad makes the defect class
   SILENT.** Today a stale hand-counted pad is *detected* — loudly, by the guard. After
   auto-pad a wrong pad is *absorbed*, the ROM changes size, and nothing notices. That is
   trading a loud failure for an invisible one, worth doing only once the layout rules are
   settled enough that the invisible case is genuinely impossible. Note this argument bites
   `pad_to(N)` too, not just blanket auto-padding — which is why `(align: N)` must stay
   mandatory alongside it rather than becoming redundant once the width is derived.
5. ~~**The remaining ROM-as-sentinel port tests**~~ — **LANDED** at `c75c2ffa`
   (`fix/rom-sentinel-port-tests`). Eleven files / 35 tests now sentinel on the source they
   actually read, via `test_support::reference_tree_for_profile` — derived from the
   `GameProfile` each gate builds (`profile.game_root_rel`, with the `map.toml` sibling by
   path arithmetic), not a copied path.
   **The booked count of ten was wrong, and HOW it was wrong is the finding.** The eleventh,
   `compression_selftest_port`, sentinels on **`s4.debug.bin`** — which an `s4\.bin` regex
   does not match, so re-running the original enumeration returns ten forever and looks
   complete. It was found by a third pass enumerating the presence-check **operation**
   (`.exists()`), which contains no filename at all. Neither of the first two passes was a
   superset of the other: five targets spell the check inline with no named guard (invisible
   to an identifier pass), and the name pass also throws false positives it cannot resolve
   (`golden("s4.bin")` reads *sigil's* golden dir, not aeon's). This is the enumeration-
   parameter bar with a live instance in this repo — **a count is only as good as the
   attribute it enumerated over, and re-running the same pass is not a second check.**
   Classification: **A = 71 files** where the ROM is the subject (untouched), **B = 11 files
   / 35 tests** (fixed), **C = 2** (ledgered — the tree-root `.exists()` probe shape, and the
   lane classifier's use-vs-mention weakness, whose kill condition is a declared
   `ARTIFACT_ORACLE_GATES` array).
   Proven three directions per test: source-only tree (35 pass, doing real work — one built
   five ROM shapes in 18s with no ROM in the tree; all 35 panic there on master); absent tree
   under strict (all 35 FAIL, exit 101, naming the missing path); full built tree (green).
   **Nothing was added to `SOURCE_GATES`, and the brief's implied reason was wrong** — cost
   is not the obstacle (all eleven run source-only in ~50s). The real one: each is measured
   against a *committed sigil artifact* (golden blob, `provenance.toml`, `pins.rs`), so
   between an aeon parcel that legitimately moves bytes and sigil's refreeze they are red by
   design. `repin_pins` argues hardest for inclusion (3.72s, wholly source-only now) and is
   excluded on exactly that ground.
   Verified at landing: **3835 / 0 / 4**, `3835 + 4 = 3839` = declared, zero `skip:` lines,
   log stamped `head=c75c2ffa branch=master`, source-gate audit `gates=35 unclassified=0`.
   The parcel adds no tests, so the bar is unchanged from item 3.
   Packet: `docs/superpowers/notes/2026-08-22-rom-sentinel-packet.md`.

**`feat/arity-cli-fixture` is LANDED, not queued** — `crates/sigil-cli/tests/const_arity_cli.rs`
is on master (added by `a24a1b4f`; the branch tip is an ancestor of master, confirmed by
`git log <branch>` rather than by an empty commit range — see the empty-range trap below).
It is a CLI-level regression test for const arity, driving the built binary via
`CARGO_BIN_EXE_sigil` over a committed poison/control pair, taking no `AEON_DIR` at all.
It exists because the enforcement was covered only at the frontend-unit level while aeon
invokes the **binary** — which is how a three-day-stale shared assembler went unnoticed.
Red-first on both arms: the poison arm against the enforcement reverted, the control arm
against the check made unconditional, because a reject-everything compiler satisfies the
poison arm and is caught only by the control. It also pins a cross-repo interface: aeon
fixtures assert on the exact diagnostic wording `array length mismatch: expected N
element(s), got M`, so rephrasing that diagnostic breaks them.

*(This paragraph sat under "front of the queue" opening with "And", which reads as a fifth
queue item. Prose adjacency is not queue membership — state landed/queued in the sentence
itself, since the next boot's only source is this file.)*

Queue items 1 and 2 both landed —
`feat/version-provenance` at `9c08f2a5`, `feat/capstone-differential` at `aafe612a`. The
merged tree is verified green at the bar above: **3821 passed / 0 failed / 4 ignored**,
`3821 + 4 = 3825` reconciling exactly against the declared `#[test]` count, zero `skip:`
lines under `SIGIL_STRICT_GATE=1`, log stamped and all three new test binaries confirmed
present in it by name. The nightly lane's self-audit is `gates=35 unclassified=0` on the
merged tree. Master is pushed — see the heading of the "current state" paragraph above for
the tip and how it was verified.

**In flight (2026-08-22, dispatched by this session).** Two worktree agents, both
sigil-internal, neither touching `golden/` / `pins.rs` / `repin.toml`:

- **`feat/field-align`** — LANDED at `6fae4d6a`, see queue item 3 above.
- **`fix/rom-sentinel-port-tests`** — LANDED at `c75c2ffa`, see queue item 5 above.

6. ~~**`pub equ` does not export**~~ and ~~**clippy fails on master**~~ — both **LANDED** at
   `1a984bd2` (`fix/pub-equ-export`). Reported by the aeon lane; ruled a **compiler bug, not a
   spec error**, on a fact neither lane had at report time: `parser.rs:546` accepts `pub` and
   threads it into `ast::EquDecl::is_pub`, a field whose own doc comment says it means
   exported, while `item_pub_name()` dropped it. **A modifier that parses and does nothing is
   worse than one that is rejected**, so §7.5 (*"`pub equ` adds module visibility like every
   other `pub` item"*, read at empyrean `origin/main`) stands and the compiler moved.
   **The one-line fix this was dispatched as would have been WORSE THAN THE BUG, and the
   agent measured that rather than arguing it.** Every other exported item renames to its
   module-qualified canonical; a `pub equ`'s definition keeps the **bare** name — the
   construct's purpose. Exporting without changing how imports bind makes `use m.a.{X}`
   resolve to a symbol nothing defines: applying exactly the specified change yields
   `unresolved symbol m.a.WIDTH`. Shipped fix also records which exports are equs and binds
   those to the bare name on the `use{}`, glob **and** prelude paths (all three called
   `canonical()` directly). Sound only because `build_program`'s `[equ.collision]` check
   already makes a `pub equ` name program-unique — its own diagnostic states that invariant.
   `collect_exported` needed no change (the `Vars` special case is for items exporting MANY
   names; an equ exports one), and `collect_defined` must NOT gain an `Equ` arm — that
   reintroduces the dangling symbol.
   **Still open, and worth knowing before writing any new consumer:** a QUALIFIED reference to
   an imported `pub equ` (`m.a.X`) still fails `unknown symbol` — `canonicalize_name`'s
   last-dot path assumes the canonical is `<module>.<item>`, which a pub equ's never is.
   **`equ` is now the only item kind whose bare and qualified spellings disagree.** Bare names
   and `use{}` imports both work. Ledgered OPEN with a kill condition.
   Verified at landing: **3839 / 0 / 4**, `3839 + 4 = 3843` = declared, zero `skip:` lines,
   log stamped `head=1a984bd2 branch=master`, all four new tests present by name, and
   `cargo clippy --workspace --all-targets -- -D warnings` **now exits 0** — that bar was
   failing on master before this parcel. **The bar moves to 3839 / 0 / 4.**
   Packet: `docs/superpowers/notes/2026-08-22-pub-equ-export-packet.md`.
   *(Superseded queue text, kept for the reasoning:)* clippy failed on master with one error,
   pre-existing, surfaced by the README pass (which is docs-only and did not cause it):
   `crates/sigil-frontend-emp/src/eval/const_arity.rs:153`, `clippy::collapsible_match` —
   *"this `if` can be collapsed into the outer `match`"*. Reproduced firsthand on master with
   `cargo clippy -p sigil-frontend-emp --all-targets -- -D warnings`; rustc 1.97.1.
   Almost certainly a lint that tightened under a toolchain bump rather than a regression.
   **Why it is queued rather than hand-fixed at the desk:** collapsing a match arm is a
   control-flow edit in shipping frontend code, and the const-arity enforcement it sits in
   landed only today — a "one-line" refactor there wants the suite behind it, and the CLI
   fixture (`const_arity_cli.rs`) pins a diagnostic aeon fixtures assert on by exact text.
   Small parcel, not a desk edit.

**README, landed** at `a4476e53` (`docs/readme-refresh`), per the owner's relayed directive
(*"let's quickly have everything update their readmes correctly. Doesn't have to be super in
depth."*). The finding worth keeping: **every CLI invocation the README documented was
broken, and none had ever worked as written** — `cargo run -p sigil-cli --` cannot run at all
(two binaries, no `default-run`), `sigil diff` does not exist, `parse`/`build` had the wrong
argument shapes, and a generator bin name was misspelled (`gen-snippet-vectors` vs the real
`gen_snippet_vectors`; the two `sigil-isa` generators ARE hyphenated, so the inconsistency is
in the tree, not the doc). Also corrected: `sigil-frontend-emp` was described as a *future*
track while being what the shipped ROM is lowered from; five workspace members were missing
from the crate table; three named harness gates no longer exist (`assemble_full_rom`,
`m0_regions`, `m1d_rom`/`m1d_debug_rom` — the last three survive only as names in
`repin.toml`/`pins.rs` comments); and the claim that the `convsym` deb2 appendix is out of
scope is false, `sigil build` produces it. **A README's commands are its most-used and
least-checked content — run them, do not read them.**

**Nothing in flight; no agents running.** Master `f02fb22b`, pushed — verified against
`git ls-remote origin refs/heads/master`, not the tracking ref. *(Superseded heading:
`ffa7bdb8`, the pad landing.)*

### RELAYOUT-REVIEW — LANDED 2026-08-26, and the sweep outranks the parcel

The aeon lane's cartridge re-layout merged here as chain-168 (`6e4f2533`, pairing aeon
`c3f5cbe0`). Their landing went red at 3941/2 on `boot_port.rs`'s hand-typed
`GameState_OJZScroll_Init`, which they re-typed. **`d8748933` derives it instead** —
`pins::OJZ_SCROLL_TEST.plain_base`/`.debug_base`, sound because `repin.toml` declares that
region's `start = "GameState_OJZScroll_Init"`, so the pin's base IS the symbol's address by
construction and `repin` regenerates it every freeze. The site stops being a ripple site
rather than becoming a sixth row in a doctrine. The aeon lane **withdrew their doc-row
proposal** in favour of this. Suite on the merged tree 3943/0/4 (3947 declared, reconciled),
zero `skip:`, zero `ratchet:`, clippy `-D warnings` exit 0, log
`~/sonic_hacks/.sigil-verify-d8748933.log`. Bar unchanged — the parcel adds no tests.

**Deriving does not weaken the gate, and the reasoning is worth keeping:** the value is an
INPUT to boot's imm-link, not the oracle. The oracle is the frozen golden window, sliced
from a *different* pin (`pins::BOOT`). A wrong `OJZ_SCROLL_TEST` now emits wrong bytes and
fails here — so the derivation ADDS an alarm. The old literal was never independent either;
its own comment said it was copied from the pin.

**THE FINDING OF RECORD is the sweep, not the fix:**
`docs/superpowers/notes/2026-08-26-hand-typed-rom-address-inventory.md`. Enumerated by the
PROPERTY (a literal address fed into or compared against shipped output) rather than by the
five known filenames — which is the enumeration that let this one hide, and a second pass
over the same five would have agreed with itself (bar 8, aimed at ourselves).

- **Nine region bases across `tranche{2,4,5,6}_negative_probes.rs` are ALREADY STALE** and
  rotted at some earlier re-layout with nothing noticing. The precise claim, measured and
  deliberately weaker than the first draft: repointing tranche4 at its current pin leaves
  all six probes green, so the base is **not load-bearing** for an `assert_ne!` — not
  "vacuous", not "red and unnoticed". The cost is lost intent and a lying comment, plus
  proof that **nothing in the tree can tell you when one of these rots.** Queued as
  `STALE-PROBE-BASES`; convert the `refrom`-slicing ones first, following
  `core_negative_probes.rs`. `tranche5:240` already uses `pins::GAME_LOOP.plain_len`, so
  the idiom is in the file.
- **Rotted independently of any doctrine:** all three off-canonical `assembled_len` values
  in `native.rs` disagree with their own generated frozen tables (config_b says `0x434d0`,
  its table `0x8b6f0`); `lib.rs:69 REGION_A_LMA` is a dead ROM-address const whose sibling
  was already deleted for that reason. Neither has a test reading it, which is why. Queued
  as `OFFCANON-ROT`.
- **`Z80_SOUND_SIZE` stays literal, ruled:** a derivation exists (`seam1::BLOB_LEN_*`) but
  `BLOB_LEN_*` is itself hand-pinned, so it trades two re-type sites for one.
- **`native_full_rom.rs:180` (`Ground_Move_Cap`) must NOT derive** — the file argues at
  `:70-75` that `repin` generates pins from the same listing `convsym` consumes, so a
  pin-derived expectation is circular. The agent listed it, tested the argument, and
  corrected itself. It is an ORACLE literal needing a doctrine entry, not a derivation.

### CONFIG-B +2 BYTES — ANSWERED 2026-08-26; R7 REFUTED HERE AND NARROWED

`docs/superpowers/notes/2026-08-26-config-b-two-byte-growth.md` (+ an explicitly
non-gate investigation artifact, `assets/deb2_appendix.py`, wired into no runner).

**The 2 bytes are not in the assembled image at all**, so the overseer's R7 hypothesis —
that a move silently changed an inferred alignment quantum — is refuted rather than
unproven. `anchor_end` is `0x8b6f0` in **both** chain 167 and 168 (verified here directly
in `provenance.toml`), and `EndOfRom` is where `convsym` writes the `de b2` magic, so the
assembled length did not move and there is no room in the image for a placement pad to have
acted. The bytes are **one 64 KB chunk-block header in the `deb2` debug-symbol appendix at
ROM offset `0x93d66`, value `00 0a`** — verified firsthand in `golden/config_b.bin`, and
the two records immediately following it are `1870`/`21b0`, the in-window halves of
`DPLC_Sonic` (`0x71870`) and `Art_Sonic` (`0x721b0`), the symbols that moved into a
`$70000–$7FFFF` window previously holding none. Non-empty chunks 7 → 8, 2 bytes of header
each. The size model derived for it predicts **all twelve** shipped appendices byte-exactly.

- **Aeon's "44 rows moved, 0 quantum changes" SURVIVES**, re-derived independently from
  sigil's own tables before comparing. It is a proof rather than a sample because
  `packed_align_of` is read only in the `labeled[i]` arm plus seam2's two frozen rows
  (frozen-table rows are the whole population), `derive_frozen_table` writes
  `s.lma + l.offset` so a base moves by exactly its row's delta, and **all 44 deltas are
  multiples of 16** — stronger than "the addresses are 16-aligned", which matters because
  several moved symbols are not (`Art_Tails` %16==10, `GameState_OJZScroll_Init` %16==4).
- **R7 is ONE-DIRECTIONAL, which narrows the inventory's row:** for a packed section
  `tb = align_up(running, a)` is divisible by `a` and the refreeze records `tb`, so an
  inferred quantum **ratchets up and can never silently fall**. The hazard is therefore not
  "needs 16, silently gets 8"; it is a quantum silently *rising*, inserting pad and
  invalidating structural pads. **Code-derived, not measured** — flagged as such in the note,
  and it wants a red-first probe that was deliberately NOT built inside this task (bar 6).
- **The appendix moves `full_size`, the `$1A4` header field and the checksum while being
  invisible to every layout-side gate** — which is exactly what made 2 bytes look alarming.
  The only guard is the coarse `min_appendix..=0x10000` band. `lean` carries no appendix at
  all. Whether that band is the right bar belongs to the appendix contract's owner.
- **The `0x2a3c0` exact landing is by construction, not coincidence:** zero symbols below it
  moved (appendix chunks 00/01 byte-identical), and `Map_Tails`' section took the order slot
  `HeightMaps` vacated, both taking `align_up(running, 16)` from an unchanged cursor.
- **TAGGED for foreground:** nothing here is runtime-confirmed; the "debugger-only data"
  claim is read off the code.

**This paragraph used to say `pad_to(N)` was PARKED ON THE OWNER and must not be
dispatched. That is no longer true and the contradiction was live for a day** — the
autonomy-scope section above recorded the owner's `d-6` ruling while this tail still told
a fresh boot to stop. A cold boot reads the tail. **The ruling governs: the `.emp`
language is this lane's to design and drive; the obligation is to put a new surface to
him before it lands, not to wait for him to raise it.**

**LANDED 2026-08-24 — the blank-import spelling (`use base._`).** Queue item `WARN-2`.
Pulls a module into the use closure while binding none of its names: the closure-edge
idiom that until now could only be spelled as a bare whole-path `use`, which
`[import.no-names]` cannot tell from an unfinished import. Owner-agreed spelling, put to
him with two alternatives (a `require` keyword; an `@allow` opt-out with no new surface)
and chosen by him. Merged at `4e262006`.
- **`_` stays an ordinary identifier to the lexer.** It is reserved BY NAME at the two
  places a module path is written — `use` and the `module` header — so one spelling never
  carries two meanings, and a module named `_` is impossible rather than merely
  unreachable. Reserving the token outright would have rippled into expressions and types
  for no gain: zero standalone `_` identifiers exist in the aeon corpus.
- **Byte-identical to the bare form, proven red-first** against a reached module that
  emits, so placement and relocation are in scope. Nothing under `golden/`, `pins.rs` or
  `repin.toml` moved, so this stayed sigil-internal and needed no aeon-lane sequencing.
- **The agent's first poison came back GREEN and it caught its own gate, not the guard.**
  A `use` binds names by TWO independent mechanisms — `resolve::ambient_from_uses` injects
  comptime names, `resolve::imports` builds the rename map for link names — and a gate on
  one says nothing about the other. This is the gate-every-consumer bar arriving in the
  frontend: the brief specified the proof as a single check and was wrong to.
- **The dispatch brief was wrong about the cross-repo interface.** It asserted aeon
  fixtures assert this diagnostic's exact text; they do not. `git grep "imports no names"`
  at aeon `origin/master` returns nothing — the id appears only as prose in four docs. The
  claim was over-generalised from the `array length mismatch` case, which IS asserted.
  The wording change is additive anyway, so every existing prefix matcher survives.
- **Still open, deliberately: aeon has not adopted it.** Sequencing — aeon writing `._`
  before sigil accepts it breaks their build. The two `CORPUS_OPEN_FINDINGS` rows stay,
  correctly, and now track the ADOPTION; their `kill` clauses and the register header were
  rewritten, because both asserted the edges "cannot be spelled another way", which is
  false as of this parcel.
- **Ledgered, not done:** `native.rs::synthetic_entry_src` writes ~88 bare `use` lines
  that are blank imports in every sense, suppressed today by a `SourceId` filter. Adopting
  `._` there would let that suppression narrow. Adjacent to the golden lane, so not taken
  incidentally.
- **Owed to empyrean:** `SIGIL_SPEC2_LANGUAGE.md` documents the `use` grammar and does not
  carry the fourth suffix. Sigil has offered to draft it; the spec is theirs to land.

A note on the `?? sigil` in `git status`: it is a self-symlink `sigil -> /home/volence/sonic_hacks/sigil`
created 2026-08-20, untracked and harmless. It is NOT a stray nested checkout — resolve it
before reading it as one.

## Standing cross-session obligations (2026-08-22)

The aeon session owes sigil two things, both triggered by sigil work rather than by
time — **ping them, don't assume they are watching**:

- **On the `game-defines` ship notice:** they re-run T8's three measured contexts
  (data-binding layout, struct harvest, RAM harvest) against a capability-derived
  define and confirm all three see it. Cheap, theirs to run.
- **When the alignment attribute lands:** they migrate the two `offsetof(Scene, …) % 2
  == 0` ensures to it and retire them. Their `ensure`s are a workaround for the missing
  language feature, not a fix for the class. Those ensures now EXIST — aeon landed them
  at `9a718f74` (merged `1a794ace`), live in `engine/level/scene_dsl.emp` (cited by symbol,
  not by line: the `:1025,1027` this row carried is exactly the coordinate-rot the protocol
  bars) — so this obligation has a concrete subject.
  **TRIGGERED 2026-08-22:** `(align: N)` landed at sigil `6fae4d6a`. The migration is
  `sc_mask_raw: i16 (align: 2)` / `sc_v_deform_shift_raw: i16 (align: 2)`, deleting both
  trailing `ensure`s. Two cautions for them: the spelling is **`(align: N)`, not
  `@align(N)`** — the latter is the `vars`-region cursor-mover and is refused by name on a
  struct field; and `sc_pad_5D`'s width is still hand-computed after the migration, since
  the assertion guards the constant rather than deriving it (that is queue item 4,
  `pad_to(N)`, parked). Push before citing `6fae4d6a` to them — verify against the remote.
  **CLOSED 2026-08-26:** aeon landed the migration (merge of `parcel/field-align` `1c3dd0cf`,
  verified reachable from aeon origin/master): both fields carry `(align: 2)`, both `ensure`s
  are gone, zero-byte on all four shapes. Their follow-up claim that the `@align(N)`-on-a-struct-
  field refusal had no test was REFUTED: `eval_layout.rs::vars_form_align_on_a_struct_field_is_refused_by_name`
  asserts on the second half of the message; they grepped the first half.

**Sigil owes aeon a warning before changing either of these** *(registered 2026-08-22)*:

- **`pub equ` is zero-byte and listing-visible — aeon depends on exactly that.** It reaches
  the `.lst` and **not** the deb2 appendix; aeon measured it (two added, ROM length
  unchanged) and is using the property as a reachability witness. So it is a contract, not
  an implementation detail: if a change would put equates into deb2, tell them **before**
  it lands rather than letting a ROM length move be their notification.
- **`[map.order-undeclared]` keys on BYTE-EMITTING sections** (`sigil-harness/src/native.rs`,
  verified by them firsthand). That scoping is load-bearing for aeon: it let them ship a
  zero-byte generated section with **no** `order` row, plus a guarantee the build stops by
  name the moment there is content to place. A row would have been inert *and* unverifiable —
  a gate that cannot measure its subject. A well-meant "require `order` rows everywhere"
  tightening would break the honest option; do not take it without talking to them.

**⚠ An empty commit range does not mean an empty branch.** Both overseers read
`git rev-list --count master..parcel/scene-even-align-guard` = 0, an empty three-dot
diff, and `--is-ancestor <tip> master` = true as proof the branch held no work. That
triple is the signature of a branch **already merged** — its commits are in master, so
the range is necessarily empty. The fix had landed hours earlier and the lint fired zero
times. This matters beyond the one incident: those are exactly the commands protocol bar
16 prescribes for converting a name into behaviour, so **the bar's mechanism is
necessary and not sufficient** — the output is two-valued and reads as one-valued.
Disambiguate with `git log <branch>` for its own history, or `--is-ancestor` on a commit
you expect the branch to CONTAIN. Two lanes cross-verified each other and shared the
frame, which is the one thing mutual verification cannot catch.

Their side is banked at aeon `1ee8f8e6` (handoff) and `ba189b40` (the `br_ext` unlock
row, cuttable cold) — both verified reachable from aeon's `origin/master`.

**Local-only anchors — RESOLVED 2026-08-22.** Sigil's `origin/master` sat at `40f862e2`
(2026-08-21) while local master ran 80 commits ahead, so every sigil SHA exchanged with
aeon that day was unreachable from origin, including the arity fixture their unlock row
cites. Master was pushed to `a70e6644` (owner-approved), fast-forward, no history
rewritten — verified by `git ls-remote origin refs/heads/master` against the remote
itself rather than by the local tracking ref, which is the only check that can distinguish
"pushed" from "looks pushed locally". Reachability from `origin/master` then re-verified
per SHA: `a24a1b4f` (the arity fixture), `cba0a0bc`, `5c75b5b6`, `9c08f2a5`, `aafe612a`,
`a70e6644` — all reachable.

**The same class with the sign flipped: a stale CAUTION is a false negative, and it wears
caution as a costume** *(aeon lane, 2026-08-22, from their own doc)*. Everything above
guards against a stale "this is reachable". The inverse is a stale **"you cannot cite
these"** — aeon's handoff carried, as standing hazard #1, that every sigil SHA was
local-only with `origin/master` at `40f862e2`. Once sigil pushed, that row made their lane
refuse a perfectly good anchor. **Nothing fails, nobody is told, and the lane looks rigorous
while being wrong** — which is why this direction outlives the positive one. Retracted on
their side at aeon `8ccef438`.
**Checked here, so the next boot does not re-check it:** this document carries **no**
mirrored row. `grep -n 'local-only\|unpushed\|not pushed'` returns exactly one hit, the
general two-directions rule immediately below, which is the durable form rather than a
snapshot. The rule generalises past reachability: **a caution is a claim, and it expires
like one.**

**The rule that outlives the incident, and it did NOT stay true by itself:** a note saying
"this is local-only" is true when written and rots on the next push, exactly as a note
saying "this is reachable" rots on the next rewrite. **Whoever acts on a cross-repo row
re-verifies reachability at read time rather than trusting either kind of note.** The
general form, which composes with the SHA-class rule: **a SHA has a class, a path has a
time, and a revision has a reachability.** Nothing about sigil being pushed today makes a
*future* citation fetchable — push before you cite, every time, and verify against the
remote.

For anything else read newest-first: the dated notes in `docs/superpowers/notes/`
(start with `2026-08-22-warn-tier-drift-open.md`), then
`docs/superpowers/notes/campaign-gap-ledger.md`, whose tail carries eleven rows added
2026-08-22 — refinement bounds unchecked on every binding form, interface `const`
members getting a shape-only check, the `emp_const_rhs` scraper that breaks on any
const gaining a type, a Capstone differential as the only non-circular ISA oracle,
and the `--extra-entry` liveness hazard (a red obtained there proves an assertion's
logic, never that it is reached).

For live next-work, read newest-first: the most recent dated `HANDOFF`/packet notes
in `docs/superpowers/notes/`, then `docs/superpowers/notes/campaign-gap-ledger.md`
for banked nice-to-haves. Keep this section's "current state" paragraph fresh when
landing an arc — a stale queue snapshot misleads the next boot more than no snapshot.
