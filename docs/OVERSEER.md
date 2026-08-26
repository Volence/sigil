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

- **Full suite bar:** `cargo test --release --workspace --no-fail-fast` with
  `AEON_DIR` set to the aeon tree — **3844 passed / 5 failed / 4 ignored** under
  `SIGIL_STRICT_GATE=1` (sigil master `e36debf8`, aeon master `415e0b6a`, 2026-08-24),
  with **zero `skip:` lines** in the run.
  **THE FIVE FAILURES ARE REAL AND ARE NOT SIGIL'S** — recorded as the bar rather than
  as an aspiration, because a bar nobody can hit is a bar nobody checks against.
  `act_descriptor_region_matches_reference`, `act_descriptor_debug_region_matches_reference`,
  `soundbankhead_pinned_bootstrap_lands_at_lma_not_vma`,
  `act_wrong_base_map_places_the_section_at_a_different_address`,
  `swapped_sec_fields_produce_different_bytes`. Two messages: `unknown function
  ojz_act1_act_default` / `ojz_act1_sec_scene`, and `section ojz_effects_editor_act1 has
  no region in the map`. Cause is aeon-side and verified there: `effects_scenes.emp`
  declares `module games.sonic4.ojz_effects_editor_act1 in ojz_effects_editor_act1` while
  `games/sonic4/map.toml` still carries that block as a **RESERVED SLOT** with
  deliberately no row. Reproduced on UNMODIFIED sigil master (`bc05f446`) in a detached
  worktree against the same aeon tree, which is the only thing that separates
  "pre-existing" from "the parcel I am landing". Relayed to the aeon lane 2026-08-24.
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

- It builds **no aeon ROM**. The region-diff gates, the golden-CRC gates and
  `pins_rs_is_current` read bytes that exist only after `./build.sh`, and they already
  have the right trigger — aeon's byte-identity ritual, which fires exactly when bytes
  move. The exclusion is named in the script, not silent.
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
  collision tables are committed, generated dirs rebuild via build.sh). What a fresh
  aeon worktree DOES need: the `.worktrees/sigil` and `skdisasm` symlinks, plus a
  PAIRED sigil worktree of the same name at `sigil/.worktrees/<name>` — the
  emp-helper-closure locator hard-asserts that pairing and fails the build without it.
  Verify the built ROMs against `golden/provenance.toml` (CRC32+size) before trusting
  the worktree. Mid-brushstroke aeon
  edits flipping sigil port-gate results is environmental, not signal; the tell is
  broad `*_port` region-diff failures at embedded addresses plus
  `repin_pins::pins_rs_is_current` failing identically on sigil master.

## Queue

The standing sigil-native arc is the **`.emp` language work (Spec 2)** — specs in
`empyrean/docs/SIGIL_*.md`. The whole sound stack is sigil-native, the language round
+ §17 optimization arc + conversion tail are done, and the map drives the build.

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

**The reference tree for any artifact-dependent run is aeon master, built.** The goldens'
`provenance.toml` **tip** pins `s4 060401e4/699106 · s4.debug 0dbaa80f/715010 · demo
c708b114 · demo.debug dec88cc1`, and building all four shapes at aeon `1ee8f8e6`
reproduces them exactly. `07d19c54`/`445092a7` appear only in the `ab` prose of a
mid-file historical entry — reading a non-tip entry as the tip cost this session a false
"corroborated" in a review. `~/sonic_hacks/.aeon-landing` is a built checkout at that
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
4. **`pad_to(N)` — a derived-width struct pad.** **RULED ADOPT, 2026-08-22 — but read the
   provenance before acting on it.** The ruling is the **empyrean overseer's**, made under a
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

**Nothing in flight; no agents running.** Master `e36debf8`, pushed (verified against
`git ls-remote origin refs/heads/master`, not the tracking ref).

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
