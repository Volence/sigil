# Assembler drift 0a58f2ec → c041f238, measured at aeon 09d964c7

**Question.** Does the sigil assembler's drift between `0a58f2ec` and master `c041f238`
move any of aeon's four ROM shapes, at aeon revision
`09d964c7326d1cb75290fcb936f7c694943b883f`?

**Answer. No. All four shapes are byte-identical across the two assemblers.**

Three earlier builds had shown this drift byte-neutral at the frozen provenance tip
`4f5ad5a1`. They are ONE result, not three: they shared a tree, a revision and a shape
set, and the enumeration parameter never varied. This run varies exactly that parameter
— same comparison, different aeon revision — and the revision is well past the tip
(`s4.bin` 720803 here against the tip's 719700).

## The instrument, proved non-vacuous BEFORE any result

A comparison that always says "identical" would produce exactly the expected answer,
which is the most dangerous case here. On real files (`golden/s4.bin`, 719700 bytes):

| proof | expected | observed |
|---|---|---|
| ROM vs an exact copy of itself | 0 differing bytes | `differing_bytes=0` → IDENTICAL |
| copy with ONE byte planted at offset 500000 | exactly 1, at 500000 | `differing_bytes=1 first_diff_offset=500000`, crc `14ee2440`→`1d9a7d86` |
| copy truncated by 3 bytes | MOVED | `differing_bytes=3`, size 719700 vs 719697 |
| absent file | refuse, not a verdict | `ABSENT: … 'did not run', NOT a verdict` (exit 1) |
| empty file | refuse, not a verdict | `EMPTY/SHORT: … 0 bytes` (exit 1) |

Cross-checked against an independent tool: `cmp -l` reports 0 and 1 respectively, the
single difference at 1-based 500001 (= 0-based 500000), octal 045→046 = 0x25→0x26 as
planted.

The first run of this proof PASSED FOR THE WRONG REASON — an argv bug made every case
report `ABSENT`, including the two that were supposed to report `ABSENT`. A proof whose
pass and whose failure look identical is not a proof. Fixed and re-run before use.

## The answer

Aeon tree at `09d964c7`, four shapes, one invocation each (`build.sh` makes ONE shape per
invocation), strictly sequential in one tree (`build.sh` rm's its outputs first, so two
builds sharing a tree transiently delete each other's ROMs).

| shape | A — `0a58f2ec` | B — `c041f238` | differing bytes | verdict |
|---|---|---|---|---|
| `s4.bin` | `06a9502e` / 720803 | `06a9502e` / 720803 | 0 | **IDENTICAL** |
| `s4.debug.bin` | `542c7365` / 741993 | `542c7365` / 741993 | 0 | **IDENTICAL** |
| `demo.bin` | `11ebd7ab` / 96602 | `11ebd7ab` / 96602 | 0 | **IDENTICAL** |
| `demo.debug.bin` | `9b0d2ce7` / 102818 | `9b0d2ce7` / 102818 | 0 | **IDENTICAL** |

Identity is CRC32 + size, never SHA1. Independently cross-checked: `cmp -l` reports 0
differing bytes for all four pairs.

## What was held fixed, and what varied

Varied: **the assembler binary, and nothing else.**

Held fixed: the aeon tree (one worktree, detached at `09d964c7`, HEAD asserted at every
arm), the aeon revision, the shape set, the build order within each arm, `NO_LINT=1`,
and `SIGIL_EMIT`.

**The `SIGIL_EMIT` decision.** `build.sh` line 399 runs `SIGIL_EMIT` on EVERY invocation,
regenerating `engine/sound/generated` — it is a live per-build input, not a one-time
provisioning step, so the choice is load-bearing rather than a detail. ONE emitter
(built from `c041f238`) was used for both arms, so that the assembler is the sole
variable; pairing each arm with its own emitter would have varied two inputs and made a
moved byte unattributable.

That choice is also empirically free, which is stronger than the argument for it. The
two emitters are genuinely different artifacts — `emit_sound_blob` at `0a58f2ec` is
5352552 bytes, md5 `65b5cbd8…`; at `c041f238` it is 5480184 bytes, md5 `52c5ba03…` — yet
run against this exact aeon tree they produce **byte-identical output across all 19
sound artifacts** (`diff -r` clean; per-file CRC32 equal). So pairing the emitters would
have fed both arms identical inputs anyway. Holding it fixed costs nothing.

Scope limit, stated rather than implied: this measures the ASSEMBLER's drift. A drift in
`emit_sound_blob` is deliberately not the subject — though on this tree it is measured
above and is nil.

## The binaries

**A — the pinned stale one**, used as given, not rebuilt (a fresh build of `0a58f2ec`
would be a different artifact answering a slightly different question).

```
$ md5sum /home/volence/sonic_hacks/.pinned/sigil-0a58f2ec
6c2378ae8a657e26684d4019a7d976d7
$ /home/volence/sonic_hacks/.pinned/sigil-0a58f2ec --version
sigil 0.1.0 (0a58f2ec)
  revision:  0a58f2ecc8e77c9433bc0ea3f0549c1e0e556f3b
  committed: 2026-09-02T17:35:23-04:00
  tree:      clean at capture — no uncommitted changes
```

**B — current master**, built from a private worktree at `c041f238` into a private
`CARGO_TARGET_DIR` on disk. The shared `sigil/target/` was never written.

```
$ md5sum .measure-drift-09d964c7/target-B/release/sigil
d38f64e64a8d33f0d91e4d2353ca5d2b
$ .../target-B/release/sigil --version
sigil 0.1.0 (c041f238)
  revision:  c041f23898bbfa2b2d33d727afdc6c20e9b3c7f1
  committed: 2026-09-04T07:07:57-04:00
  tree:      clean at capture — no uncommitted changes
  closure-revision: c68ffe872967c6c3394d4459da8f8364562b8bb3
```

Binary B's closure correspondence verifies against its own tree:
`git log -1 --format=%H HEAD -- <closure-paths>` = `c68ffe87…` = the binary's
`closure-revision`. No commit in that tree can have reached this binary unseen.

## Controls

**1. The arms really used different assemblers.** The strongest available answer to "was
the instrument the thing under test", and it comes from the SUBJECT's own build system
rather than from this parcel:

```
arm A       build log:  Assembler: sigil 0a58f2ecc8e7 (clean at capture)
arm B       build log:  Assembler: sigil c041f23898bb (clean at capture)
arm Arepeat build log:  Assembler: sigil 0a58f2ecc8e7 (clean at capture)
```

Without this, four IDENTICAL rows would be equally consistent with "byte-neutral drift"
and "both arms silently ran the same binary" — and on a byte-neutral result those are
the only two worlds there are.

**2. The subject is genuinely past the frozen tip.** Measured shapes vs the frozen-tip
goldens — MOVED is REQUIRED here, and is what makes this a NEW observation rather than a
fourth telling of the old one:

| shape | golden @ tip | measured @ `09d964c7` | differing bytes |
|---|---|---|---|
| `s4.bin` | `14ee2440` / 719700 | `06a9502e` / 720803 | 465092 |
| `s4.debug.bin` | `142294b3` / 737683 | `542c7365` / 741993 | 508867 |
| `demo.bin` | `0c456778` / 96474 | `11ebd7ab` / 96602 | 28307 |
| `demo.debug.bin` | `2e603d53` / 101339 | `9b0d2ce7` / 102818 | 62475 |

**3. Order control.** All four arm-A shapes were rebuilt AFTER arm B (`Arepeat`) and
reproduce arm A's CRCs exactly, for all four shapes. The result is not an artifact of
build order or of state left behind by the preceding arm.

**4. Every artifact positively asserted.** A missing result file is "did not run", never
a verdict. Each of the 12 ROMs was asserted to exist, be non-empty, carry a plausible
size, and have an mtime POST-DATING the build that made it; a build exiting non-zero, or
producing a ROM older than its own start, is recorded as NOT_TAKEN rather than as a
divergence. This was not theoretical here — see below.

## What the brief got wrong, and one near-miss

**The brief said master was `c041f238`; it is `1d5b00c2`.** `c041f238` is its parent and
an ancestor of HEAD; the only commit between them adds one line to `docs/lane-log.jsonl`
and touches nothing in the compile closure. Binary B was built at `c041f238` exactly, as
instructed, so the measurement is unaffected — but "current master" and `c041f238` were
not the same object at dispatch time.

**`provision-aeon-ref.sh` has a SECOND defect beyond the `REF_TARGET` one the brief
names.** The brief's workaround (pass `REF_TARGET` explicitly) is correct and necessary.
But the script also cannot provision a worktree at an arbitrary path: aeon's
`tools/suite_paths.py` climbs to the directory holding both `aeon/` and `empyrean/` and
then requires the repo to sit at exactly `<suite-root>/<repo>/tools/…` or
`<suite-root>/<repo>/.claude/worktrees/<wt>/tools/…`. A worktree at
`<suite-root>/.measure-drift-…/aeon-ref` matches neither, and `build.sh`'s pytest
preflight fails 5 tests with `UNMEASURABLE: … matches none of the layouts this repo
has`. The failure names layouts, not ROMs, and is not an assembler or golden problem.
Fix: place the worktree one level under the suite root (here
`/home/volence/sonic_hacks/aeon-measure-09d964c7`). Relocating and re-provisioning was
the honest repair; disabling aeon's verification to get a number would not have been.

**The near-miss, and it is the exact failure invariant 6 exists for.** When that first
provisioning run died, the worktree already contained `s4.bin`, `s4.debug.bin`,
`demo.bin` and `demo.debug.bin` — all four present, all four non-empty, all four with
plausible ROM sizes. They were the frozen-tip GOLDENS, copied in by provisioning step 3,
and no build had ever run. CRC'ing them would have produced four confident, wrong,
mutually-consistent rows. Two things caught it: their mtimes pre-dated any build, and
their sizes were the tip's (719700) rather than this revision's (720803). Presence,
non-emptiness and plausible size are jointly insufficient; the mtime-post-dates-the-build
assertion is the one that actually discriminates.

**Binary A was destroyed and restored mid-run** (the controller demonstrating that a
file's read-only mode does not stop a rename over it — that is the directory's
permission). No build had used it at that point: only its metadata had been read. Its
md5 was re-verified before first use and captured AT POINT OF USE inside each arm's log
(`6c2378ae8a657e26684d4019a7d976d7`, all three arms). Measurement taken from the
hardened path `/home/volence/sonic_hacks/.pinned/sigil-0a58f2ec`, whose directory is
`0555`. Nothing needed re-running.

**zsh's lack of word-splitting bit twice**, both times exactly as the standing note and
the `--version` banner's own `drift:` block warn. Once in a control loop (`set -- $pair`
passed one word), once avoided by using `${=CP}` for the closure-path check. An unquoted
parameter used as a pathspec list matches nothing, prints nothing, and exits 0 — a tree
never looked at reads as a tree with no drift.

**`ls` (aliased to eza) reported the notes directory MISSING** while `git ls-files`
showed it populated. Emptiness from `ls` here is not a finding.

## Provenance and conditions

- aeon revision: `09d964c7326d1cb75290fcb936f7c694943b883f`, ancestor of aeon
  `origin/master`, verified by the provisioner's own `ls-remote`-backed reachability
  check. NOT "aeon master" — their live tree was at `3a247c92` and moving.
- aeon worktree: `/home/volence/sonic_hacks/aeon-measure-09d964c7`, exclusively this
  parcel's. The owner's live checkout at `/home/volence/sonic_hacks/aeon` was never
  built in and never written to beyond `git fetch` + `git worktree add`, both of which
  touch only `.git`.
- No source changed in any repo. No emulator was touched.
- Golden control at this revision is correctly `not-applicable` (pinned revision is
  `4f5ad5a146b799c13aedabbba9da23fce370b63c`); the goldens describe different source, so
  their CRCs are printed as data and nothing is asserted from them.
- Wall clock: instrument proof 07:17–07:18; arm A 07:29:41–07:38:19; arm B
  07:38:19–07:47:22; arm A-repeat 07:47:22–07:55:57 (2026-09-04).
- Load average was 3.82 at dispatch, peaked at 11.85 during the two concurrent cargo
  builds, and sat between 3.0 and 9.0 across the ROM builds. Contention was real and
  visible; per-shape build times stayed in a narrow 123–159 s band regardless, and the
  order control rules out any effect on the bytes.
