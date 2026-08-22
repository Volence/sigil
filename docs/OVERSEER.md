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

## Quality bars

- **Full suite bar:** `cargo test --release --workspace --no-fail-fast` with
  `AEON_DIR` set to the aeon tree — currently **3810 passed / 0 failed / 4 ignored**
  under `SIGIL_STRICT_GATE=1` (sigil master `cba0a0bc`, aeon `1ee8f8e6`, 2026-08-22),
  with **zero `skip:` lines** in the run — check that, not just the
  totals: a reference gate that skips reports nothing and reads as coverage.
  **Reconcile the total against the tree, not against the last remembered bar:**
  `git grep -c '#\[test\]' HEAD -- '*.rs'` summed gives the declared count, and
  `passed + ignored` must equal it (3810 + 4 = 3814 here). Baseline arithmetic carried
  across branches measured on different reference trees does not reconcile and will
  invent a discrepancy that is not there. Never plain
  `cargo test`: without `--release` some gates are impractically slow, without
  `--workspace --no-fail-fast` a wedge or an early failure hides the rest of the
  result set. Report failures-first with explicit pass/fail counts; never
  `grep | head` test output (it buries FAILED lines).
- **Pre-merge:** re-run the suite on the merged tree with `SIGIL_STRICT_GATE=1`, which
  turns reference-tree skips into failures so the port gates cannot silently skip.
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
`sigil-source-gates.timer` at 05:17 daily. It runs the **33 gates whose inputs are aeon
SOURCE** — the warn-tier corpus and its neighbours — against detached master-tip
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

**Current state (2026-08-22, master `cba0a0bc`).** Landed this session, both
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
revision; `~/sonic_hacks/.aeon-sigil-gates` is **source-only by construction** and must
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

1. **A provenance witness for the shared binary** — `sigil --version` reporting the
   revision it was built from. The shared assembler sat three days stale while every
   aeon build used it, and byte identity is silent on that by construction. Class-level
   row in the ledger.
2. **A Capstone differential as a permanent gate** — the only non-circular ISA oracle
   available, already installed; it found the `TST` bug on its first run. Two
   known-benign disagreements must be excluded by name (`6xFF` branch words, `btst
   Dn,#imm` sizing).
3. **An alignment attribute / even-offset assertion** — the class-level fix for the
   odd-field finding: today a struct wanting even-aligned members can only say so by
   hand-counting bytes into a pad, and the pad goes stale silently. Aeon's own fix has
   LANDED (`9a718f74`, `ensure(offsetof(Scene, sc_mask_raw) % 2 == 0, …)`), so this now
   has a live subject to retire rather than a hypothetical one.
4. **Ten remaining ROM-as-sentinel port tests** — they gate on `s4.bin` existing to
   answer "is there an aeon tree here", so a source-only tree makes them panic
   "aeon tree missing" while the tree is fully present. One was fixed this session
   (`native_object_bank_budget`, sentinelled on `map.toml`, proven both directions).

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

**In flight (2026-08-22, dispatched from master `5c75b5b6`):** two sigil-internal parcels,
both in isolated worktrees, neither touching the aeon-paired lane —
`feat/version-provenance` (queue item 1) and `feat/capstone-differential` (queue item 2).
Both parcels landed earlier merged at `cba0a0bc` and the merged tree is verified green;
**master is NOT pushed** — sigil `origin/master` is `40f862e2` and local master is 65
commits ahead (see the local-only anchor warning below).

## Standing cross-session obligations (2026-08-22)

The aeon session owes sigil two things, both triggered by sigil work rather than by
time — **ping them, don't assume they are watching**:

- **On the `game-defines` ship notice:** they re-run T8's three measured contexts
  (data-binding layout, struct harvest, RAM harvest) against a capability-derived
  define and confirm all three see it. Cheap, theirs to run.
- **When the alignment attribute lands:** they migrate the two `offsetof(Scene, …) % 2
  == 0` ensures to it and retire them. Their `ensure`s are a workaround for the missing
  language feature, not a fix for the class. Those ensures now EXIST — aeon landed them
  at `9a718f74` (merged `1a794ace`), live at `engine/level/scene_dsl.emp:1025,1027` —
  so this obligation has a concrete subject.

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

**⚠ Local-only anchors.** Sigil's `origin/master` is `40f862e2` (2026-08-21) while local
master is far ahead, so **every sigil SHA exchanged with aeon on 2026-08-22 is
unreachable from origin** — including the arity fixture their unlock row cites, and the
revision the shared `target/release/sigil` was built from. Aeon recorded those citations
as local-only deliberately. **When sigil is pushed, whoever acts on one of those rows
re-verifies reachability rather than trusting the note** — the note was true when
written and does not stay true by itself. The general form, which composes with the
SHA-class rule: **a SHA has a class, a path has a time, and a revision has a
reachability.**

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
