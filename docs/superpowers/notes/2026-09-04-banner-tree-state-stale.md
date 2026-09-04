# BANNER-TREE-STATE-STALE — close packet

`sigil --version`'s `tree:` word could report a clean tree over a binary compiled from
uncommitted sources. aeon's assembler-provenance gate (`aeon/build.sh`, the
`SIGIL_VERSION_STRICT` arm) keys on that word with a positive match on
`{clean, clean-sources}`, so it opened on precisely the case it exists to catch.

Branch `fix/banner-tree-state`. Two faults, both reproduced before either was touched.

---

## Fault 1 — the capture was keyed on the revision, not on the sources

### Reproduced, at `feba7521`, on a clean tree

```
$ sed -i 's|usage: sigil parse <file.emp>|... REPRO-MUTATION-IS-ON-DISK|' \
      crates/sigil-cli/src/main.rs
$ git status --porcelain=v1        ->   M crates/sigil-cli/src/main.rs
$ git rev-parse HEAD              ->  feba7521556ba2e12f9f0dad4438ce4cc8c8035b  (unmoved)
$ cargo build --release -p sigil-cli
     Compiling sigil-cli v0.1.0
     md5  c167943da7f6359e40327a362fd553c1  ->  4353055f3b128c96aca3579c71dcb14f
$ ./target/release/sigil parse
     usage: sigil parse <file.emp> REPRO-MUTATION-IS-ON-DISK
$ ./target/release/sigil --version
     sigil 0.1.0 (feba7521)
       tree:      clean at capture — no uncommitted changes
```

aeon's gate arm, run on that word: **PASSES.**

The binary printed the uncommitted edit back at the operator in its own output while the
banner called the tree clean. No CRC, golden or pin downstream could have caught it: the
md5 moved and nothing compared it to anything.

### Mechanism

Emitting any `cargo:rerun-if-changed` replaces cargo's default whole-package tracking for
the build script. Every trigger `build.rs` emitted was revision-shaped — `<git-dir>/HEAD`,
`<common-dir>/refs`, `<common-dir>/packed-refs`, and each closure manifest. None of them
follows file *content*. Cargo tracks sources for **compilation**, so an uncommitted edit to
a closure source recompiles the crate and relinks the binary while the build script keeps
its previous answer.

### The fix

Emit `cargo:rerun-if-changed` for each derived closure path — the set `build.rs` already
walks from `cargo metadata --no-deps`. The rule and its trap live in
`crates/sigil-cli/src/tree_class.rs` (`source_triggers`, `Triggers`) beside the classifier,
so `cargo test` runs them.

Why it escapes both refusals the old module note recorded, each checked rather than assumed:

* **Not the whole repository.** The 29 derived paths are `Cargo.toml`, `Cargo.lock`,
  `.cargo`, `rust-toolchain`, `rust-toolchain.toml`, each closure package's manifest and
  `src` dir, and the whole directory of each package carrying a build script
  (`sigil-cli`, `sigil-salvador-sys`, `sigil-clownlzss-sys`, `sigil-clownnemesis-sys`).
  `target/` is at the workspace root and is under none of them, checked directly. The
  build reaches a fixed point — see M3 below.
* **Not an unconditional rerun.** It fires only when a source the binary is compiled from
  actually changed, which is when cargo was going to recompile this crate anyway. Measured
  below, and the measurement is where the honest cost turned up.

### The trap, and it was live here

The closure deliberately lists `.cargo`, `rust-toolchain` and `rust-toolchain.toml`
whether or not they exist, on the stated ground that *a git pathspec matching nothing is
harmless*. All three are **absent in this workspace**. A rerun trigger naming a missing
path is the opposite of harmless: cargo reads it as dirty on every build. Emitting the
derived list unfiltered would have bought the unconditional-rerun cost the design refuses.

Only existing paths are emitted. The rest are named in the banner rather than hidden:

```
  tree-tracked: 26/29 closure path(s) watched for edits; NOT PRESENT and so unwatchable
                (a trigger on a missing path makes every build dirty):
                .cargo rust-toolchain rust-toolchain.toml
```

---

## Fault 2 — the printed drift recipe returned an answer-shaped nothing

The banner printed `git log -1 --format=%H HEAD -- <closure-paths>` beside a
space-separated path list. Verified firsthand under **zsh 5.9.2**, this machine's shell:

```
PATHS="<the closure-paths line>"          # 733 characters
git log -1 --format=%H HEAD -- $PATHS     # ONE pathspec — zsh does not word-split
  stdout: []
  exit:   0
```

Empty, exit 0, reads as an answer. With splitting forced (`${=PATHS}`) the same command
returns `48313276d3c34315fd7800281cfe8e7b04954e9e` — exactly the reported
`closure-revision`. **The check was sound; only its spelling was not.**

Replaced with a `drift-check:` line carrying a whole command: the paths are already in it,
`-C <source>` carries the run-at-the-tree-root instruction that the pathspecs depend on,
and nothing is left for a reader to substitute in any shell. Paths are shell-quoted only
when they need it, so a path with a space cannot split into two pathspecs and reintroduce
the same silent-empty answer from the other side.

The only in-repo consumer, `scripts/lib/sigil_tool.sh`, was already correct — it uses
`read -r -a paths` — so nothing downstream changes.

---

## Measurements

Primary metric is **`rustc` invocations under `cargo … -v`**. It is deterministic and
load-independent, and it is the quantity the design decision turns on. `Compiling <pkg>`
is printed once per *package* and undercounts a run that relinks 150 test binaries as `1`;
an earlier pass of this measurement used it and was misled by it.

Wall clock is reported with the 1-minute load average beside every row. **The box was
shared with another lane's `cargo test --workspace` throughout**; load ran 2–35 on 16
cores. No wall-clock figure here should be read without its load column, and the
`rustc`-invocation columns are what the conclusions rest on.

### M1 / M3 — no-op builds, and the fixed point

| run | before | after |
|---|---|---|
| no-op `build --release -p sigil-cli` ×3 | 0 units, 0.038 / 0.071 / 0.086s (load 29) | 0 units, 0.031 / 0.020 / 0.019s (load 9) |
| no-op `test --workspace --no-run` ×3 | 0 units, 0.099 / 0.103 / 0.100s (load 20) | 0 units, 0.099 / 0.101 / 0.100s (load 9) |
| **M3** two consecutive no-op builds | 0 units, 0.038s then 0.071s | 0 units, 0.019s then 0.020s |
| **M3** two consecutive no-op `--no-run` | 0 units, 0.185s then 0.205s | 0 units, 0.105s then 0.105s |

**The build reaches a fixed point.** Two consecutive no-op runs do no work, in both
profiles, with the triggers in place. No accidental always-dirty trigger.

### M2 — after touching one file

Counted in `rustc` invocations, with the triggers removed as the control (PROOF 1's
mutation, so the delta is attributable to the triggers and nothing else):

| touched | with triggers | control (no triggers) |
|---|---|---|
| nothing | 0 | 0 |
| `docs/OVERSEER.md` (outside the closure) | 0 | 0 |
| `crates/sigil-link/src/lib.rs` (a dependency source) | **347** | **347** |
| `crates/sigil-cli/tests/version_provenance.rs` | **155** | **1** |

Wall clock for the last two rows, five runs each, load 10–36 throughout:

```
with triggers,    touch cli test:   4.891  5.002  5.069  6.924  5.952 s
without triggers, touch cli test:   0.300  0.204  0.178  0.176  0.184 s
with triggers,    touch dep src:   10.267 s        control: 12.512 s
```

The build script's own subprocesses are not the cost: `cargo metadata --no-deps --offline`
is 7–9 ms, `git status --porcelain=v1` 2–3 ms, the closure `git log` 1 ms.

### The one real regression, and why it is paid rather than avoided

`sigil-cli` has 2 bins, 151 integration-test targets and 1 build script. Its `tests/`
directory sits inside `crates/sigil-cli`, which is in the closure as a **whole directory**
because the package carries a build script and a build script may read any file in its
package. So editing one of those 151 test files reruns the script, and cargo recompiles
the dependent crate on a rerun even when the script's output is byte-identical — the old
module note's claim, re-measured and confirmed. 1 unit becomes 155; 0.16s becomes 4.9s.

Kept anyway, and the reason is not thrift:

* narrowing the trigger set below the material set would make the tree word **stale for a
  region the classification itself calls material** — a false clean, the one direction
  that must not exist;
* it would put the emitted set and the reported set out of step, and my gate would have to
  be weakened to permit the divergence;
* the cost lands only on the edit-a-CLI-test loop. Never on a no-op, never on a real
  source edit, never on a `docs/` edit.

It is a measured trade rather than an oversight, and the number is here so it can be
re-decided with the number in hand.

---

## Proofs

Every proof is a chain **green → red → green** on the same instrument, with the mutation
quoted back from disk and `git diff --stat` naming the file before the red run. Restores
are `git checkout --` against a committed baseline on an otherwise-clean tree.

| # | mutation | what MUST FAIL | result |
|---|---|---|---|
| 1 | the `println!` handing cargo the triggers is removed | `every_closure_path_that_exists_is_watched_for_edits` | RED, exit 101, named |
| 2 | the existence filter is dropped (`partition(\|_p\| true)`) | same test, on `.cargo` | RED, exit 101, named |
| 3 | `drift_check` returns `-- $PATHS` again | `the_printed_drift_check_returns_the_reported_revision_in_every_shell` | RED, exit 101, named |
| 4 | the `-C` is dropped — command still runs, answers wrongly | same test, on the VALUE | RED, exit 101: got `a660312a…`, wanted `f0f3893c…` |
| 5 | the trigger `println!` removed **and committed** | `scripts/tree_state_capture_gate.sh` | RED, **exit 1**, reproducing the original defect verbatim |
| 6 | the existence filter is dropped | `tree_class::tests::a_path_that_does_not_exist_is_never_emitted_as_a_trigger` | RED, exit 101, named |

Two corrections made to the proof method partway through, both recorded because the
uncorrected versions read as passes:

* **PROOF 5 first came back exit 2, `COULD NOT RUN`** — the script gate refuses a dirty
  tree (its restore is a `git checkout --`), so the uncommitted mutation could not be
  measured. That is not a red. Re-run with the mutation **committed on a throwaway
  branch**, which is also the honest shape of the defect: a `build.rs` that ships without
  the triggers. It then went red at exit 1.
* **PROOF 6 first used `cargo test -p sigil-cli --lib`, which exits 101 with
  `no library targets found in package sigil-cli`** — the same exit code a real failure
  gives, and it printed 101 *before and after* the restore. It measured nothing: the proof
  ran the wrong program. `tree_class`'s unit tests compile into the bin target under
  `cfg(test)`, so the selector is `--bins`. Under that selector it is green → red (named)
  → green.

Proofs 1–4 were re-established under the tightened chained method after PROOF 6 introduced
it; the first pass showed red-then-restore without re-running green between each, which
leaves "was the instrument healthy at that moment" unasserted.

### The split between the two gates, stated rather than assumed

`crates/sigil-cli/tests/version_provenance.rs` reads **cargo's own recording of the build
script's stdout** (`$OUT_DIR/../output`, the path derived at build time and carried as
`SIGIL_BUILD_SCRIPT_OUTPUT`), so what it asserts is the directive stream cargo *received*.
An unreadable stream fails and names the substitute; it is never skipped and never renders
as green.

The first version of that gate asserted what the **banner claimed** about the trigger set,
and the banner computes that claim beside the emission rather than from it — so deleting
the `println!` left the gate green. The defect would have been reintroducible under a
passing test. That is the vacuous shape this whole feature argues against, appearing in the
gate for it.

What no in-process test can show is that cargo **acted** on the stream — that needs a
tracked source edited, a rebuild and the banner read back, which must not run inside a
shared checkout under a killable suite. `scripts/tree_state_capture_gate.sh` is that half.
It is on no timer and says so: run it before landing, merging, freezing or quoting this
banner, and after any change to the trigger set.

---

## Residual holes, named

`SIGIL_TREE_STATE` is still labelled a snapshot, because the tracking is path-scoped rather
than total. It can only **under-report**, and only where no mtime under a watched path
moves:

1. **Dirt outside the closure** — `clean` may stand where `clean-sources` is now true.
   Both are trusted words in aeon's positive-match arm, so this costs a consumer nothing.
2. **A derived closure path that does not exist yet** — the three named in `tree-tracked`.
   Creating `.cargo/config.toml` or `rust-toolchain.toml` changes rustflags or the compiler
   itself, which invalidates cargo's own fingerprints; that is a **mitigation, not a
   guarantee**, and it is written down here rather than relied on silently. Closing it
   properly would need a trigger on the workspace root, which contains `target/` — the
   option refused for never reaching a fixed point.
3. **An edit landing inside the same cargo invocation** that captured the state.
4. **Any change that alters content without moving an mtime** — cargo's tracking is
   mtime-based. This includes editing the root `.gitignore` so that a previously-ignored
   file inside a closure directory starts being reported by `git status`.

A word beginning `dirty` is therefore trustworthy when it appears.

---

## Cross-repo surface

**The state-word vocabulary is UNCHANGED** — still `{clean, clean-sources, dirty,
unknown}`, pinned by `version_provenance.rs`. aeon's positive-match arm needs no
amendment, and the "a consumer enumerating the trusted words must be told when a word is
added" obligation is not triggered.

Two banner lines are added, `tree-tracked:` and `drift-check:`. Every consumer parses
anchored labels — `^  closure-revision:` and `^  closure-paths: `
(`scripts/lib/sigil_tool.sh`, `scripts/nightly_ref_drift.sh`) and `^ *tree: *`
(`aeon/build.sh`) — and `tree-tracked` cannot match the last of those, which requires the
literal `tree:`. The `tree:` line is still printed first, so `head -1` is unaffected
either way. The `freshness:` first physical line still carries `re-captured` and
`(cargo tracks HEAD,…,manifests,sources)`, which is what three existing disclosure gates
read.

## No emitted byte moves — proved, not assumed

A binary built at the merge-base (`feba7521`, in a detached worktree with its own target
dir) and one built at this tip were handed the same inputs and their output compared.

```
OLD 6471ecf0bce170a9c9b78c4b7e5f52fa  sigil 0.1.0 (feba7521)
NEW 6d8c24f94b4b79ea1711897658ad2dc6  sigil 0.1.0 (8b6c90c8)

  asm m68k.asm    IDENTICAL   30 3C 12 34 72 07 D0 81 41 FA 00 28 24 18 61 00 00 04 60 EC
                              48 E7 FF FE 4A 40 67 10 53 40 66 F4 …    (191 bytes of hex)
  asm z80.asm     IDENTICAL   3E 3F 21 16 00 11 FF 1F 01 10 00 ED B0 23 1B 09 EB C3 00 00
                              18 EA 00 11 22 33 00 00
  emp prog.emp                IDENTICAL   DE AD BE EF
  emp const_arity_control     IDENTICAL   (0 bytes)
  emp defines -D DEBUG=1      IDENTICAL   2A
  emp defines -D DEBUG=0      IDENTICAL   07

  6 compared, 6 identical, 0 differing
```

Covers the `.asm` front end with the 68k backend (`movem`, `lea` pc-relative, `bsr.w`,
`cmpi.l`, `swap`, `dc.w`/`dc.l`/`dc.b`), the Z80 backend (`ldir`, `jr`, `jp`, `db`, `dw`),
and the `.emp` front end including `-D` define selection.

**Two earlier attempts at this proof measured nothing and are not quoted as evidence:**

1. `examples/*.emp` — every one refused at exit 2, because `sigil build` requires
   `--aeon <dir>` and this worktree has no reference tree. Zero bytes compared; the script
   printed `MEASURED NOTHING`.
2. The `.emp` vectors compared clean, but all three hand-written `.asm` fixtures *also*
   refused (`even` is not a 68000 mnemonic in this dialect; Z80 hex needs a trailing `h`),
   so the asm front end and both backends were never exercised — **while the verdict line
   claimed they had been.** The fixtures were fixed rather than the verdict softened.

One fixture still refuses and is named rather than dropped: `relax.asm` gets
*"branch needs an explicit size suffix (.s or .w) — Aeon pins branch width, no
relaxation."* That is the assembler being right and the fixture being wrong; branch
relaxation is not a thing this assembler does, so there is nothing there to compare.

Consistent with the source surface: `git diff master..HEAD` touches shipped code only in
`run_version()`. `tree_class.rs` is compiled into the binary under `cfg(test)` and is
reached by `build.rs` through `#[path]`, so it is not linked into the release executable.

## Suite

`cargo test --workspace`, `SIGIL_ALLOW_PARTIAL=1`, from this worktree at `8b6c90c8` on
`fix/banner-tree-state`: **4381 passed, 0 failed, 2 ignored over 381 legs, exit 0.** All
six gates added here ran and passed.

`SIGIL_ALLOW_PARTIAL=1` is a **declared partial run**: no reference tree is provisioned in
this worktree, and pointing `AEON_DIR` at the owner's live checkout would produce phantom
failures and let `sigil build --aeon` write into it. **126 of the 366 test binaries are
reference-dependent and every row in them was left UNMEASURED.** The strict landing gate
against a provisioned tree is the overseer's to run.

`cargo clippy --workspace --all-targets`: exit 0, 0 warnings.

`cargo fmt --all --check` is red — 6438 diffs across ~350 files, including many this branch
never touched (`crates/sigil-ir/`, `crates/sigil-isa/`, `crates/sigil-frontend-emp/`…).
There is no `rustfmt.toml`; this workspace is simply not rustfmt-formatted and `fmt --check`
is not a gate here. Stated so a later reader does not mistake it for something this branch
introduced.

## Incidental, not introduced here

With `SIGIL_ALLOW_PARTIAL=1` the partial-run disclosure — *"126 test binaries are
reference-dependent and every row in them is left UNMEASURED"* — is printed by the test
that produces it, and libtest **captures** per-test stdout unless `--nocapture` is given.
So a captured suite log from a partial run contains no trace of the disclosure and reads
as unqualified green, while the two reference-dependent rows report `ok`. The mechanism
works; its legibility in a log does not. Pre-existing, and d-18 already rules that a run
which only prints how much it did not measure still exits 0.
