# Scripts name their tree — the shell half of the suite-paths contract

Parcel branch `parcel/scripts-name-their-tree`, off master `9a2f40c6`.

**The contract.** `contract/SUITE_PATHS.md`, empyrean `origin/main` at
`f222f37148da88c19f9ef0f0fc789f7e5274a061` (recorded at read time, 2026-09-02). It has since
gained a clause on the step-3 proof shape, which this parcel already satisfied — **§9** has
the SHAs, all re-verified at write time rather than relayed.
`docs/OVERSEER.md` R7 (line 76) is this lane's migration list; R4 (line 55) is the d-18
refusal that sits on top of the same resolver. Board rows `SUITE-PATHS-MIGRATION` and
`LANDING-RUN-DEFEATS-THE-NEW-GUARD`.

**Precedence, identical in every resolver:** (1) the explicit checkout variable
`<TOOL>_DIR`; (2) `EMPYREAN_SUITE_ROOT` joined with the repo's directory name; (3)
derivation from this repo's own location via `git rev-parse --git-common-dir`, never
`--show-toplevel`; (4) refuse, naming every variable consulted and every path tried. A
variable that is **set but wrong** is a hard error at its own step.

---

## 1. What landed

| commit | what |
|---|---|
| `8fc92808` | `scripts/lib/suite_paths.sh` — the shell resolver |
| `f4c273d1` | `landing-run.sh`, `capture_goldens.sh`, `derive_offcanonical_sizes.sh` |
| `51567415` | the two nightly lanes, `drift-nightly.conf`, `provision-aeon-ref.sh` |
| `f4dcced5` | `golden/ab/suite_paths.py` + the eighteen A/B scripts |
| (this) | `crates/sigil-harness/tests/scripts_name_their_tree.rs`, `corpus_bytediff.sh`, this note, the ledger rows |

### Why shell and not a Rust binary

Three of the callers must be able to REFUSE before a cargo build exists. `landing-run.sh`
builds the assembler it is about to judge, so a resolver needing that build could not
refuse ahead of it. The two nightly units run from bare detached checkouts on a timer,
where the first thing that happens is the resolution and the second is a compile that must
not start when the resolution failed.

### Exit codes (both halves)

| code | meaning |
|---|---|
| 0 | resolved; path on stdout, announce on stderr |
| 3 | step 4 — nothing named the checkout |
| 4 | a variable was set but does not name that checkout |

---

## 2. Every `/home/volence` literal in scope, routed or left with a reason

`grep -rn '/home/volence' scripts crates/sigil-harness/golden --include='*.sh'
--include='*.py' --include='*.conf' --include='*.service' --include='*.timer'
--include='*.md'` — **43 hits before, 2 after**, both deliberate.

### Routed (41)

| site | was | now |
|---|---|---|
| `scripts/landing-run.sh:207` | `AEON_ARG:-AEON_DIR:-<literal>` | `--aeon` is step 1 and announced; else the include; log stamp carries the step |
| `golden/capture_goldens.sh:75` | `AEON_DIR:-<literal>` | include, resolved **after** the write gate |
| `golden/derive_offcanonical_sizes.sh:25` | `AEON_DIR:-<literal>` | include |
| `scripts/nightly_ref_drift.sh:37` | `SIGIL_MAIN=<literal>` | `suite_resolve_checkout sigil SIGIL_DIR` |
| `scripts/nightly_ref_drift.sh:38` | `AEON_MAIN=<literal>` | `suite_resolve_checkout aeon AEON_DIR` |
| `scripts/nightly_ref_drift.sh:41` | `SIGIL_DRIFT=<literal>` | `$SUITE_ROOT/.sigil-ref-drift` |
| `scripts/nightly_ref_drift.sh:47` | `CARGO_TARGET_DIR=<literal>` | `$SUITE_ROOT/.sigil-ref-drift-target` |
| `scripts/nightly_source_gates.sh:31,32` | `SIGIL_MAIN`, `AEON_MAIN` | the two resolvers |
| `scripts/nightly_source_gates.sh:33,34` | `SIGIL_GATES`, `AEON_GATES` | joined onto `$SUITE_ROOT` |
| `scripts/nightly_source_gates.sh:41` | `CARGO_TARGET_DIR` | joined onto `$SUITE_ROOT` |
| `scripts/drift-nightly.conf:47` | `DRIFT_AEON_TREE=<literal>` | `${EMPYREAN_SUITE_ROOT:?…}/.aeon-ref-drift` |
| `scripts/provision-aeon-ref.sh:22` | `AEON_REPO:-$(cd $SIGIL_ROOT/../aeon)` | `AEON_REPO` fed into the include as step 1 |
| `scripts/provision-aeon-ref.sh` (`W` default) | `$SIGIL_ROOT/../.aeon-ref` | `$(suite_resolve_root)/.aeon-ref` |
| `scripts/corpus_bytediff.sh:17` | `MASTER_DIR=<literal>` | `suite_resolve_checkout sigil SIGIL_DIR` — **not on the brief's list; found by the enumeration** |
| 18 × `golden/ab/*/*.py` `sys.path.insert` | `<literal>/empyrean/clients/python` | `suite_paths.add_empyrean_clients()` |
| 6 × `golden/ab/*/*.py` `LST` | `<literal>/aeon/s4.debug.lst` | `suite_paths.debug_listing()` (2 of the 6 had no env override at all) |

### Left, with reason (2)

| site | reason |
|---|---|
| `scripts/systemd/sigil-ref-drift.service:6` | `ExecStart` is by construction an install-time absolute path; systemd has no run-time resolver, and `%h` only trades a full literal for a home-relative one that is equally wrong when the suite moves. This is the ENTRY point — the one place the literal is not a duplicated fact, because the script it names resolves everything else. `scripts/systemd/README.md` already makes reinstalling the documented step after an edit. |
| `scripts/systemd/sigil-source-gates.service:6` | as above |
| (`--include='*.md'` matched nothing) | no documentation in scope named a path |

**Two, not three.** `DRIFT_RECORD_READER` was reported BLOCKED and is now routed — see §2b.
Final count: **43 in scope before, 2 after**, both deliberate.

### 2b. `DRIFT_RECORD_READER` — was blocked, then ruled and fixed

The gate holding it in place is `the_record_seam_is_empty_and_absence_is_not_a_pass`, which
reads the key as RAW TEXT and required absolute-or-empty. The coordinator ruled the
amendment permissible on the substance: **that gate's subject is "the seam is genuinely
configured or genuinely empty; absence is not a pass", and a `${EMPYREAN_SUITE_ROOT}/…`
value IS genuinely configured — it is only not literal.** Blanking the key or renaming the
path would have defeated the subject; teaching the gate the spelling does not.

**An ownership question was raised and is now SETTLED.** The ruling named the gate
`crates/sigil-harness/tests/drift_nightly_harness.rs` and cleared it as outside the
concurrent lane's file set. The file is actually
**`crates/sigil-cli/tests/drift_nightly_harness.rs`** — *inside* that lane's declared
`crates/sigil-cli/tests/**`. That was flagged before the edit was reported rather than
assumed away, on this lane's own evidence: the file contains **no `AEON_DIR`, no
`aeon_dir`, and no `env::var` at all**, so it holds no private resolution copy for that lane
to route, and the hunks here are confined to `seam_verdict` and its unit test.

The hub then checked rather than conceded, and the substance survived the error: the other
lane's **actual** routing population is the files under `crates/sigil-cli/tests` carrying
`env::var("AEON_DIR")`, and this file is not one of them. Re-measured here at write time —
`grep -rl 'env::var("AEON_DIR")' crates/sigil-cli/tests/*.rs | wc -l` → **85**, and the seam
gate is **not** among them. The edit stands; the glob was drawn wider than the work.

Worth keeping as method rather than trivia: **refusing to treat this lane's own evidence as
the authorization is what surfaced the error at all.** Had the evidence been read as
permission, the edit would have been identical and the wrong glob would still be in the
coordinator's model of who owns what.

What the amendment does, and the three ways a spelling is still refused:

- the variable must be one the job exports **before** it sources the config. The exporter
  set is DERIVED from `nightly_ref_drift.sh`, not listed; "before" is part of the rule,
  since an export after the `source` line cannot reach the value. An empty derived set is
  `COULD NOT MEASURE`, because it would make every expansion look unexportable for a reason
  about the scan rather than about the config.
- a variable **set to nothing** is Broken. Set-but-empty is a wrong environment rather than
  a missing one — the same semantic the resolver applies one level up — and would compose a
  reader rooted at `/`.
- a remainder not starting with `/` is Broken: no expansion of it can be absolute, so the
  relative-reader refusal would be evaded by spelling alone.

The variable's value arrives through an **injected lookup**, not `std::env::var` inside the
verdict. The three failure modes are properties of the value, and reaching them by mutating
this process's environment would make one case's setup visible to every other test in the
binary — shared state inside a gate about configuration being unambiguous.

**No coverage was traded away.** The live value now reports `Unprovable` here and says why;
the literal reported `Unprovable` too in an unprovisioned checkout, because
`.aeon-ref-drift` does not exist on a fresh tree. Named, not passed, in both.

Red-first, all three, each mutant reaching a different verdict class than the rule requires:

| mutation | the failing assertion's own wording |
|---|---|
| the exporter check is disabled | `a reader expanding a variable no exporter sets names nothing runnable, so it is broken, not tolerable (got Unprovable)` |
| set-but-empty is treated as a value | ``a reader whose variable is set to nothing would be rooted at `/`, which is broken, not tolerable (got Unprovable)`` |
| the rooted-remainder check is disabled | `a reader whose remainder is not rooted cannot expand to an absolute path, which is the relative case under another spelling (got Unprovable)` |

See §8 for how the first attempt at these three rows nearly went into this packet as a
pass.

---

## 3. Gates added

All seven live in `crates/sigil-harness/tests/scripts_name_their_tree.rs`, which is a
**cargo test target — the house pattern** (`reference_tree_write_guard.rs`,
`skip_marker_lint.rs`, `source_gate_classification.rs`, `golden_freeze_atomicity.rs`), so
it runs in every `cargo test --workspace`, which is what `landing-run.sh` invokes.
Each runs the real resolver in a subprocess in a scrubbed environment (`env_clear()`, then
`PATH` only) against a bed the test builds under the target directory — never `/tmp`.

**Classification for `scripts/nightly_source_gates.sh`:** the file names no built artifact
and calls no harness accessor — every path it uses points into its own bed — so the lane's
classifier buckets it `no-reference`, which is what it is. Verified by running the lane's
own rule: `--audit` → `scanned=132 source=44 artifact=85 no-reference=3 unclassified=0`,
`rc=0`.

**The bed nests its linked worktree INSIDE the checkout**, where this repo's real ones
live. Beside the checkout, a `--show-toplevel` implementation and a `--git-common-dir` one
give the SAME answer, so a bed shaped that way would pass with the wrong code.

| # | gate | derivation of its expectation | red-first mutation | the failing assertion's own wording |
|---|---|---|---|---|
| 1 | `step_one_wins_and_is_announced` | announce format read off the include's `suite_paths_announce` | step-1 announce says step 9 | `step 1 resolved without announcing itself as step 1:` … `(step 9: explicit AEON_DIR)` |
| 2 | `a_set_but_wrong_variable_stops_the_resolution_by_name` | the contract's "hard error at that step, never a null" | the wrong value is blanked instead of refused (both halves) | `a wrong variable was accepted:` … `RC=0` |
| 3 | `step_two_joins_the_suite_root` | the contract's step-2 wording | announce loses `EMPYREAN_SUITE_ROOT/aeon` | `step 2 resolved without announcing itself as step 2:` … `(step 2: the suite root)` |
| 4 | `step_three_derives_the_sibling_from_inside_a_linked_worktree` | the contract's "never `--show-toplevel`" | `--git-common-dir` → `--show-toplevel` | `step 3 refused a sibling that is present — the derivation is answering from the worktree's own root rather than from the common git directory:` … `tried …/sigil/aeon (no such directory)` |
| 5 | `step_four_refuses_naming_every_variable_and_every_path` | the contract's "refusals name the variable(s) consulted and the path(s) tried" | the `consulted EMPYREAN_SUITE_ROOT` line is dropped (both halves) | `the refusal has no line saying it CONSULTED `EMPYREAN_SUITE_ROOT`. Naming it only in the closing advice tells a reader what to set, not what was already looked at:` |
| 6 | `the_announce_is_not_counted_as_a_skipped_gate` | the two spellings `landing-run.sh:369` counts | announce adopts one of them | ``the resolver's own output contains `skipping`, which the landing bar counts as a gate that measured nothing:`` |
| 7 | `no_resolver_caller_regrows_a_home_literal` | population = entry points read out of the include × runnable files that name one | a literal planted in `capture_goldens.sh` | `1 file(s) that use the resolver still name one person's home directory…` naming `capture_goldens.sh:21` |

Three more in `crates/sigil-cli/tests/drift_nightly_harness.rs::the_seam_verdict_separates_absent_from_broken`,
covering the config spelling the same parcel introduced — table in §2b.

### Gate 7 was widened a second time, and again by checking a claim instead of banking it

Writing this parcel's ledger row produced the sentence *"routing the conf brought it into
the lint's population."* Checking it — `grep 'suite_resolve_\|suite_paths_' scripts/drift-nightly.conf`
→ **no match** — showed the opposite. The conf names no entry point, so an
entry-point-only population never judged it, **which is exactly how the literal this parcel
removed from it survived the first pass.**

A file does not have to CALL a resolver to consume one: the conf is sourced by the job after
the job has resolved and exported the root, and expands it. `calls_the_resolver` now also
matches a file that expands a variable the resolver ANNOUNCES under a literal name, with the
variable derived from the resolver's own `suite_paths_announce` calls rather than retyped.
Expansion syntax (`${NAME`) is required, so prose naming the variable is not swept in. An
empty announced set is `COULD NOT MEASURE`, for the same reason the empty entry-point set is.

Red-first, with a pre-check that the mutation actually changed the file (`git diff --stat`
→ `1 insertion`) before trusting the run:

```
1 file(s) that use the resolver still name one person's home directory…
  scripts/drift-nightly.conf:11: # STALE_READER=/home/volence/…/drift_record.py
```

That is **three** population defects found in one lint by red-first, all of the same shape:
the population looked right and could not reach a file it was supposed to judge.

### Two gates were WRONG when first written, and the red-first pass is what said so

Both were **green under a mutation that broke the rule they name**, which is the whole
point of proving red before keeping a gate.

- **Gate 5** asserted the refusal `contains` each variable name. Both names also appear in
  the message's closing advice, so a refusal that never said it CONSULTED the suite-root
  variable passed. Now asserted per line (`consulted_line`).
- **Gate 7's population** matched files whose `source` line spelled the include's path —
  and MISSED both capture scripts, which hold that path in a variable first so they can
  check whether the include is reachable at all. A literal planted in one of them left the
  lint green. The population now derives the resolver's entry points from the resolver's
  own source (`suite_*() {` definitions) and matches callers by those names; a function
  name cannot be held in a variable the same way. Population is runnable files only
  (`.sh`, `.py`, `.conf`) — a document describing a path is not a resolver, and **this
  note** quotes refusals full of the literal it is about.

---

## 4. Transcripts, verbatim from real runs

### `landing-run.sh` — step 1 by variable, step 1 by flag, and a set-but-wrong refusal

`SIGIL_BUILD` names a binary that does not exist; that override is checked immediately
after the tree is resolved and BEFORE any build, so the run stops there having announced.

```
=== step 1 via AEON_DIR (the provisioned reference tree)
# AEON_DIR=/home/volence/sonic_hacks/.aeon-ref-scripts (step 1: explicit AEON_DIR)
landing-run: REFUSING — SIGIL_BUILD is set to /nonexistent-sigil-binary, which is not an executable file.
       Unset it to use this run's own build, or point it at a real binary.
rc=2

=== step 1 via --aeon
# AEON_DIR=/home/volence/sonic_hacks/.aeon-ref-scripts (step 1: explicit --aeon)
landing-run: REFUSING — SIGIL_BUILD is set to /nonexistent-sigil-binary, which is not an executable file.
rc=2

=== set-but-wrong AEON_DIR
suite-paths: REFUSING — AEON_DIR=/home/volence/sonic_hacks/empyrean is not the aeon checkout (no build.sh — that is not the aeon checkout).
       A variable that is set but wrong is a wrong environment, not a missing one, so
       this stops here rather than resolving aeon some other way and leaving AEON_DIR
       pointing somewhere else for everything downstream.
landing-run: REFUSING — the reference tree could not be resolved (see the refusal above).
       Pass --aeon <path to a built aeon checkout>, or export AEON_DIR.
rc=2
```

### The two nightly lanes — both refuse before any side effect

Run bare with a set-but-wrong `AEON_DIR`. Neither created a worktree, built anything, or
touched a checkout; the earlier resolutions announce first.

```
=== nightly_source_gates.sh (bare, wrong AEON_DIR)
# EMPYREAN_SUITE_ROOT=/home/volence/sonic_hacks (step 3: derived from this checkout via git --git-common-dir)
# SIGIL_DIR=/home/volence/sonic_hacks/sigil (step 3: sibling of this checkout via git --git-common-dir)
suite-paths: REFUSING — AEON_DIR=/nonexistent-aeon-for-this-probe is not the aeon checkout (no such directory).
rc=2

=== nightly_ref_drift.sh (bare, wrong AEON_DIR)
   [identical three lines]
rc=2
```

`--audit` and `--selftest-fail` still work with no reference tree at all (`rc=0` / `rc=1`).

### Step-4 refusal, in full (from the gate's own red-first run)

```
suite-paths: REFUSING — cannot locate the aeon checkout.
       consulted  AEON_DIR                  (unset)
       consulted  EMPYREAN_SUITE_ROOT   (unset)
       tried      <bed>/sigil/aeon (no such directory)
       Export AEON_DIR to the aeon checkout, or EMPYREAN_SUITE_ROOT to the directory the
       suite's checkouts hang off. This does NOT fall back to a live working tree: a
       run against a tree nobody named is a run nobody can reproduce.
```

### The Python half, scrubbed env, one subprocess per case

```
=== step 3, nothing set
# EMPYREAN_DIR=/home/volence/sonic_hacks/empyrean (step 3: sibling of this checkout via git --git-common-dir)
=== step 1
# EMPYREAN_DIR=/home/volence/sonic_hacks/empyrean (step 1: explicit EMPYREAN_DIR)
=== step 1 set-but-wrong (names aeon)
EMPYREAN_DIR=/home/volence/sonic_hacks/aeon is not the empyrean checkout (no contract — that is not the empyrean checkout).   [rc=3]
=== step 2
# EMPYREAN_DIR=/home/volence/sonic_hacks/empyrean (step 2: EMPYREAN_SUITE_ROOT/empyrean)
=== step 2 root wrong
EMPYREAN_SUITE_ROOT=/home/volence/sonic_hacks/aeon names a suite root whose empyrean entry (…/aeon/empyrean) is not the empyrean checkout (no such directory).   [rc=3]
```

---

## 5. The shell↔Rust agreement list (for the merge gate)

The concurrent lane is writing the Rust side of the same precedence. Their branch was not
read. These are the behaviours the two must agree on; each is checkable against
`scripts/lib/suite_paths.sh` and `golden/ab/suite_paths.py`, which already agree with each
other and are gated as agreeing by gates 1–5 above (every case asserts both halves).

1. **Announce format, exactly.** `# <VAR>=<path> (step <N>: <reason>)`, on **stderr**, one
   line, before any work against the path. The path is the resolved absolute path.
2. **The four reason strings**, verbatim:
   - step 1 → `explicit <VAR>` (a command-line flag spells it `explicit --<flag>`)
   - step 2 → `EMPYREAN_SUITE_ROOT/<name>`
   - step 3 → `sibling of this checkout via git --git-common-dir`
   - suite-root step 3 → `derived from this checkout via git --git-common-dir`
3. **Refusal prefix** `suite-paths: REFUSING — `, and a refusal carries one line per
   variable containing the word `consulted` and that variable's name, plus a line
   containing the path tried. Gate 5 asserts the per-line form, not a bare `contains`.
4. **Set-but-wrong is defined by a marker check, not by existence.** `.git` (file or
   directory — a linked worktree's is a file) plus per-repo markers: `aeon` →
   `build.sh` + `engine`; `sigil` → `Cargo.toml` + `crates/sigil-harness`; `empyrean` →
   `contract` + `clients`. A name with no marker row is checked for `.git` only. **If the
   Rust side checks only existence, the two disagree** on a variable pointing at a sibling
   checkout, which is the commonest wrong value.
5. **Set-but-wrong applies at step 2 as well as step 1**: a suite root that exists but
   whose `<name>` entry is not that checkout is a hard error, not a fall-through to step 3.
6. **Step-3 semantics**: `git rev-parse --git-common-dir`, asked **from the resolver's own
   file's directory** (not `$PWD`, not the caller's cwd), resolved to absolute by `cd`
   because git may answer relatively; suite root = `dirname(dirname(common))`; candidate =
   `<root>/<name>`. A step-3 candidate that fails its marker check falls to **step 4**, not
   to a hard error — steps 1 and 2 are assertions by a person, step 3 is a guess.
7. **Exit / error codes**: 3 = step-4 refusal, 0 = resolved, 4 = set-but-wrong. The Python
   half raises `SuitePathError` and its callers exit 3; if the Rust side flattens 3 and 4
   into one code, a caller cannot tell "nobody named it" from "somebody named it wrongly".
8. **The announce must contain neither `skip:` nor `skipping`** — `landing-run.sh:369`
   counts both out of its own log. Gate 6.
9. **Resolution is cached per process** in the Python half (one announce per answer). If
   the Rust side announces per call, a transcript's step lines multiply; harmless, but the
   two logs then read differently for the same run.
10. **`AEON_DIR` is the ratified checkout spelling.** Aliases accepted during the
    transition (`AEON_REPO` in `provision-aeon-ref.sh`, `--aeon` in `landing-run.sh`) are
    fed INTO the resolver as step-1 values rather than resolved separately, so a
    set-but-wrong alias produces the same refusal naming the same variable.

---

## 6. Deviations, blocked items, tagged items, re-provisioning

### Deviations from the brief, with evidence

- **`repin --check` warning count: two measurements disagree, and the disagreement is
  UNRESOLVED — not resolved in this parcel's favour.** This run printed **ten**
  `declared allotment` warnings (five regions × two shapes: `entity_window`, `children`,
  `objdefs`, `dust_spindash`, `player_climb`) from the tree provisioned here. The brief said
  four; the coordinator re-checked that source and it is **not** a truncated tail —
  `grep -c 'declared allotment'` over the whole provisioning log of the **other** reference
  tree returns 4 (`dust_spindash` and `player_climb`, plain and debug). Both trees are at
  the same engine revision `027ec162`.
  So this is not a stale number in the brief. It is **two `repin --check` runs against the
  same revision warning about different region sets**, which is a determinism question; and
  since `repin.toml`'s own header says ~80 region/shape pairs declare `allotment` today,
  **both 4 and 10 are small subsets and the selection is unexplained by either of us.**
  NOT CHASED — out of this parcel's scope, and the machine belongs to the engine lane's
  freeze. Recorded here so the next reader inherits the open question rather than one
  lane's number. `pins.rs unchanged` printed last, and that is the witness that mattered.
- **`provision-aeon-ref.sh` had a second literal the brief did not list**, the default
  worktree path `$SIGIL_ROOT/../.aeon-ref`, wrong from a linked worktree in exactly the way
  `../aeon` was. Routed to `$(suite_resolve_root)/.aeon-ref`.
  **`../aeon` did not merely *look* wrong — it KILLED this parcel's own first provisioning
  run**, before a line of the parcel was written: `SIGIL_ROOT` from a linked worktree is
  `<sigil>/.claude/worktrees/<agent>`, so `../aeon` named a directory that does not exist.
  Every sigil agent runs in a linked worktree, so that spelling was failing for the majority
  of its callers. Measured on this session, not supposed about someone else's.
- **`scripts/corpus_bytediff.sh:17` was not on the brief's list.** The enumeration found it;
  it is a resolver site (`MASTER_DIR` is precisely step 3's answer) and is routed.
- **The provisioning script's own first run failed here**, at
  `SIGIL_BIN=$SIGIL_ROOT/target/release/sigil` — the shared-target assumption, not a path
  literal, so it is out of this parcel's scope. Worked around by passing `SIGIL_BIN` and
  `REF_TARGET` explicitly. Worth a follow-up: the script's `SIGIL_BIN` default assumes a
  `target/` inside the checkout, which the landing rules forbid a landing run from using.

### BLOCKED

- **None, and none outstanding.** `DRIFT_RECORD_READER` was reported blocked, ruled on by the
  coordinator, and closed here (§2b). The escape hatch worked as intended: stopped on the
  item, recorded exactly why, continued with the rest, and the item was unblocked by someone
  with the authority to weigh the gate's subject rather than by this lane deciding its own
  exception.
- The crate-path flag raised alongside it is also settled (§2b): checked by the hub, glob
  wider than the work, edit stands.

### TAGGED for the controller — nothing here was attempted

- **No emulator was used and none is needed for this parcel.** The eighteen A/B scripts are
  hand-run against the emulator and cannot be executed here; the helper they now import was
  proved in isolation instead (section 4). What is NOT proved is that a full A/B run still
  works end to end — that needs one hand-run of any of the eighteen with the emulator up.
- **`refreeze`, `repin --write` and a real `landing-run.sh` were not run.** Only resolution
  paths were exercised (dry runs, early exits, an override that refuses before any build).

### Re-provisioning the nightly timers needs after landing

`sigil-source-gates.timer` and `sigil-ref-drift.timer` fire
`/home/volence/sonic_hacks/sigil/scripts/nightly_*.sh` — the **main** checkout, so the
timers pick these changes up as soon as master carries them. **This branch changes nothing
about what the timers run until it lands.** After landing:

1. Nothing to reinstall. The `.service`/`.timer` files are unchanged, so no
   `cp` + `daemon-reload` is needed.
2. **The lanes' own detached checkouts do need one thing**: `nightly_source_gates.sh` runs
   its gates inside `$SUITE_ROOT/.sigil-source-gates`, which it re-checks out to master
   every run, so it picks the new script up on the first fire after landing. The same holds
   for `.sigil-ref-drift`. No manual step.
3. **The one thing to watch on the first fire after landing** is that both lanes now
   announce three resolution lines before doing anything, and both now exit 2 with a
   `COULD NOT RUN` notification if the suite cannot be resolved where previously they would
   have proceeded against a literal. A first-fire check of
   `${XDG_STATE_HOME:-~/.local/state}/sigil-source-gates/nightly.log` and the ref-drift
   equivalent confirms the resolution announced and the lane ran.
4. `.aeon-ref-drift` is created by `provision-aeon-ref.sh` from the drift lane, which now
   passes `AEON_REPO` explicitly — unchanged behaviour, since that is step 1.

---

## 7. Suite

Held at the coordinator's instruction: the engine lane is running a golden freeze on this
machine and a release workspace sweep alongside their ROM builds is the load that has
produced a killed freeze twice before. Everything else in this note was completed first.

Targeted single-binary runs that were made (all green, real output):

| target | result |
|---|---|
| `sigil-harness --test scripts_name_their_tree` | 7 passed; 0 failed; 0 ignored |
| `sigil-harness --test golden_write_gate` | 6 passed; 0 failed; 0 ignored |
| `sigil-harness --test source_gate_classification` | 2 passed; 0 failed; 0 ignored |
| `sigil-harness --test golden_freeze_atomicity` | 9 passed; 0 failed; 0 ignored |
| `sigil-harness --test reference_tree_write_guard` | 2 passed; 0 failed; 0 ignored |
| `sigil-harness --test reference_tree_named_write` | 1 passed; 0 failed; 0 ignored |
| `sigil-harness --test skip_marker_lint` | 2 passed; 0 failed; 0 ignored |
| `sigil-harness --test harness_root_handover` | 4 passed; 0 failed; 0 ignored |
| `sigil-harness --test strict_census_lint` | 5 passed; 0 failed; 0 ignored |
| `sigil-cli --test drift_nightly_harness` (after the §2b amendment) | 7 passed; 0 failed; 0 ignored |

`scripts/nightly_source_gates.sh --audit` after every change: `scanned=132 source=44
artifact=85 no-reference=3 unclassified=0`, `rc=0` — the lane still classifies this tree.

`golden_write_gate` is the one that matters most for the capture-script change: it runs the
shipped script from a COPY planted outside the repo, which is the branch where the include
is unreachable and only step 1 applies.

`bash -n` passes on all nine touched shell/conf files. **`shellcheck` is not installed on
this machine** (`which shellcheck` → not found), so no lint beyond `bash -n` was run.

The reference tree was provisioned at the pinned `027ec162` and PROVEN, before the hold:
both rebuild controls matched the frozen images (`s4.bin fdd1cf81/719387`,
`s4.debug.bin 0f6b1359/736391`) and `repin --check` ended `pins.rs unchanged`.

---

## 8. Two findings about the METHOD, not the code

Both came out of this parcel by accident, both are about how a proof can be false while
reading green, and the hub has taken both as standing rules. They are here in full because
the interesting part of each is the moment it nearly got away.

### 8a. A mutation that fails to apply is indistinguishable from a correctly restored baseline

**What happened.** The three red-first proofs for the seam amendment (§2b) ran from a script
that mutated the gate, ran the one test, and restored with `git checkout -- <file>` between
cases. Case A went red. Cases B and C printed `ok`.

**What I nearly did.** I had a ready explanation for `ok` — the same explanation that had
been *correct* twice already that hour, since gates 5 and 7 had genuinely been too weak and
had genuinely printed `ok` under a real mutation. So a third and fourth `ok` looked like
more of the same class of finding, and I was one step from writing "cases B and C reveal the
verdict is insensitive to those inputs" into this packet. That would have been a fabricated
finding on top of an unrun test.

**What was actually true.** The amendment was **not committed yet**. The first
`git checkout --` therefore did not restore a baseline — it *deleted the work*. Mutations B
and C then searched a file that no longer contained their anchors, replaced nothing, wrote
the file back unchanged, and ran the **original** gate, which passes because it is the
original gate. Only case A ever tested anything.

**Why it is invisible.** Both my mutation scripts used `str.replace`, which returns the
input unchanged when the anchor is absent, and writes it back without complaint. A
no-op mutation and a correct restore produce byte-identical files, so the *only* difference
between "the rule survived being broken" and "the rule was never broken" is a `git diff` I
had not run. Nothing in the output distinguishes them: both print `ok`.

**Why the invariant did not catch it.** The bar says *prove it red-first*. It does not say
*prove the mutation applied*, and a red-first proof is only worth the mutation actually
landing. Every proof of this shape has the hole, mine and anyone else's.

**The fix**, now used in this parcel's later red-first runs: assert the mutation changed the
file before believing the run. The conf lint proof (§3) opens with

```
### PRE-CHECK: the mutation must actually change the file, or the run proves nothing
 scripts/drift-nightly.conf | 1 +
 1 file changed, 1 insertion(+)
```

and only then runs the gate. A `1 insertion` line before a red is a proof; a red with no
such line is an anecdote.

**Retroactive audit of this parcel under the new rule.** Asked of every red-first claim in
this note, since a rule that only applies going forward would leave the existing rows
resting on the method it just condemned:

- Gates 1, 2, 3, 4, 6 (§3) each went **red**. A mutation that did not apply cannot produce a
  red, so the mutation demonstrably landed in all five. Sound.
- Gates 5 and 7 (§3) printed `ok`, which is the ambiguous outcome — but each was
  **independently confirmed afterwards**: the assertion was tightened, the same class of
  mutation was re-run, and it then went red (§2b table's siblings, and the
  `capture_goldens.sh:21` row). A weakness diagnosed from an `ok` and then reproduced as a
  red is established by the red, not by the `ok`. Sound, by the second measurement rather
  than the first.
- The seam amendment's three (§2b): first run unsound and discarded; re-run after committing
  the amendment, all three red, and each mutant lands in a *different* verdict class than the
  rule requires, which is a second signal that the mutation bit.
- The conf lint proof (§3): carries the pre-check. Sound.

So no row in this packet rests on an unapplied mutation. The audit is recorded because
"I checked the old ones too" is exactly the claim a reader cannot verify from a rule change
alone.

### 8b. A derived population must be derived from the CONSUMPTION relation, not the call relation

Gate 7's population was wrong three times, each time in the same direction — it looked
principled and could not reach a file it existed to judge:

1. **Hand-written** would have gone stale at the twenty-fifth caller. Never shipped; the
   brief forbade it.
2. **"Sources the include by path"** missed both capture scripts, because they hold the
   include's path in a variable first *precisely so they can check whether it is reachable*.
   Caught by a planted literal that stayed green.
3. **"Names an entry point"** missed `drift-nightly.conf`, because the conf never calls
   anything: it is *sourced by the job after the job has resolved and exported the root*, and
   expands it.

The third is the general lesson and the other two are its special cases. **A file does not
have to CALL a resolver to CONSUME one.** The conf holds a fully resolved path and names no
function, so any population built on the call relation is blind to it by construction — and
that blindness is not hypothetical: **it is how the conf's own home literal survived this
parcel's first pass**, when the parcel's whole subject was removing home literals from files
that consume resolutions.

The population now matches the consumption relation: entry points named, the Python module
imported, **or a variable the resolver announces under a literal name being expanded**, with
the variable read out of the resolver's own `suite_paths_announce` calls. Expansion syntax
(`${NAME`) is required so prose is not swept in, and an empty announced set is
`COULD NOT MEASURE` — otherwise a scan failure would silently return the population to the
shape that missed the conf.

**How it was found is the part worth keeping.** Not by testing, and not by review: by
**writing the ledger row and then checking its own sentence.** The row said "routing the conf
brought it into the lint's population." One `grep` said otherwise. The claim was mine, about
my own work, written minutes earlier, and it was wrong — which is the memory note *own repo
state asserted from memory* arriving in a new costume, and the reason the row is now a
`CLOSED` with a red-first proof instead of a plausible sentence.

---

## 9. Contract SHAs, re-read at write time

| what | SHA | checked |
|---|---|---|
| contract as this parcel was built against | `f222f37148da88c19f9ef0f0fc789f7e5274a061` | reachable from `origin/main` |
| contract clause on the step-3 proof shape | `08dd3f6` | **reachable**, verified here |
| tip the hub named as "identical at" | `f96fbf6` | reachable |
| **empyrean `origin/main` at this write** | **`889062f0f3c8f09e30ef7bb60ba739074566694b`** | fetched and read here |

The tip had already moved past the one the hub named — expected, and the reason the check is
worth doing rather than the relay worth trusting (*a peer's status is a snapshot*). What
matters is settled independently of it: `git diff 08dd3f6 origin/main -- contract/SUITE_PATHS.md`
is **empty**, so the clause this gate answers to is byte-identical at the live tip.

**The new clause** (added 2026-09-02 from aurora's O68, found on their own merged resolver):
the step-3 proof runs from a linked worktree, or says in the run's own output that it did
not — because in the main checkout `--show-toplevel` and `--git-common-dir` agree, so a test
asserting the property there proves nothing, and a test that skips there never runs where the
suite normally runs.

**Already satisfied, before the clause existed** (§3): the bed builds a real repo, an engine
checkout beside it, and a linked worktree **nested inside** the checkout, then runs the
resolver with `current_dir` inside that worktree under `env_clear()`. So the property is
proved wherever `cargo test` is invoked from, with no skip and no vacuity. The nesting is the
load-bearing part and was chosen for the reason the clause gives: a worktree that happens to
sit *beside* the suite root hands the wrong method the right answer by accident.

**The one thing the clause asked for that this did not do**: remove the worktree after. Forty-
three beds had accumulated under the target directory, each a `git worktree` registration in a
repository that exists only while the scratch tree does. `impl Drop for Bed` now removes it,
on a panic as well; nothing is lost, because every assertion here quotes the subprocess's
whole merged output, so the evidence is in the panic message rather than on disk. Verified: a
full green run now leaves **zero**.
