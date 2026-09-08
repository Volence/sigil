# Closing `WARN-TIER-COUNTS-UNWATCHED`: the warn tier's file gate (2026-09-08)

Closes the gap-ledger row `WARN-TIER-COUNTS-UNWATCHED`, opened the night before by the
`PROBE-CONTENT-SNAPSHOT` sweep while it was refuting a claim of its own.

Reference tree for every measurement here: `/home/volence/sonic_hacks/.aeon-ls12-fix`, aeon at
`ec640bcf`, unmodified. Churn figures are `git log --diff-filter=A --since=<n>.days` over `*.emp`
in that tree, taken 2026-09-08.

---

## THE HOLE, RE-MEASURED RATHER THAN INHERITED

`warn_tier_corpus.rs` had two gates over the corpus warnings:

* `warn_tier_lint_ids_match_the_frozen_baseline` compares the firing id SET per shape against
  `CORPUS_LINTS` + the shape's `WARN_ID_BASELINE` row. An id already admitted cannot fail it by
  firing more often, or in a file it has never fired in.
* `corpus_open_findings_fire_exactly_where_registered` counts firings per `(shape, id, file,
  detail)` and fails on an unregistered site — but it walks
  `warnings.iter().filter(|w| registered_ids.contains(w.id.as_str()))`, and `registered_ids` is
  built from the register's own rows. That was `import.no-names` alone.

Both halves confirmed by running them. The size of what fell between them, measured by walking the
same seven-shape lowering the gates share:

| id | firings, 7 shapes | distinct files | files added ≤14d | ≤30d |
|---|---|---|---|---|
| `import.no-names` | 10 | 2 | 0 | 1 |
| `module.path-mismatch` | 98 | 14 | 4 | 5 |
| `module.unreachable` | 477 | 94 | 21 | 63 |
| `proc.clobber-undeclared` | 486 | 3 | 0 | 0 |
| `proc.out-unwritten` | 7 | 1 | 0 | 0 |
| `proc.undeclared-fallthrough` | 91 | 10 | 0 | 1 |

**1167 of the 1169 firings were watched by nothing below the id set.** Only the two
`import.no-names` sites had a register row.

## THE LEDGER'S PROPOSED FIX IS REFUTED BY THAT TABLE

The row proposed the small move: widen the register walk from `registered_ids` to `CORPUS_LINTS`,
and pay "one adjudication pass to add rows for whatever the five ids currently fire at". The table
is what that pass costs and why it must not be paid:

* **124 rows**, each carrying an owner, an anchor and a kill condition, for 1169 firings.
* The rows would pin **counts that move on ordinary engine work**. `page_cache.emp` fires
  `proc.clobber-undeclared` 66 times on `sonic4 plain` and 70 on `sonic4 debug`;
  `proc.undeclared-fallthrough` totals run 5 to 21 across the seven shapes; `module.unreachable`
  runs 57 to 92. The file's own header already rejects exactly this ("a baseline that churns on
  unrelated work gets rubber-stamped, and a rubber-stamped gate asserts nothing") — the proposal
  reintroduced it one level down.
* **21 of `module.unreachable`'s 94 files were added within 14 days.** A gate over that population
  goes red on correct work more than once a week. That is the always-red trade: it does not make
  the corpus quieter, it makes the gate ignorable.

So the ledger's own "the move, and it is small" is wrong, and it is wrong in the direction the same
ledger row warns about two paragraphs later.

## WHAT LANDED: `SITE_WATCH`, a FILE gate over every admitted id

One new table and two new tests in `crates/sigil-cli/tests/warn_tier_corpus.rs`.

```rust
struct SiteWatch { id, unpinned: &[&str], why_unpinned: &str, files: &[&str] }
```

`files` is the set of files that fire the id, **unioned over the seven shapes** and **count-free**.
That is the choice that separates this from the refuted proposal: the union does not move when a
comptime define pulls a file into or out of one shape's closure, and no count is pinned, so
ordinary engine work inside a file that already fires is not this gate's business. What the file
set moves on is a class reaching code it had never reached — the event nothing was watching.

* `site_watch_rows_are_completely_specified` — every id admitted by `CORPUS_LINTS` **or** by a
  `WARN_ID_BASELINE` shape row has exactly one row; a row for an id nothing admits is stale and
  fails too. This is the part that keeps the hole from reopening: admitting a new id without
  watching its files is now a failure, not a default. It also refuses a row that disarms itself
  (a pinned file underneath one of its own unpinned prefixes) and an unpinned prefix with no
  measurement behind it.
* `warn_tier_firing_files_match_the_pinned_sites` — the gate. Red on a file that fires an admitted
  id and is not pinned; red on a pinned file that fires in no shape any more; red on an admitted id
  that fires zero times, which is this walk going blind rather than a win.

### The two unpinned populations, and why they are named rather than swallowed

Two ids fire on populations that grow with the corpus rather than with its defects. Pinning those
would be the always-red check; pretending they are covered would be the false comfort this parcel
is about. So they are unpinned BY PREFIX, each prefix carrying the measurement that put it there,
and the number of firings each prefix swallows is **rendered on every run, green or red**:

```
warn-tier files: import.no-names               10 firing /   0 unpinned (0 prefix(es)) /  2 pinned
warn-tier files: module.path-mismatch          98 firing /  98 unpinned (3 prefix(es)) /  0 pinned
warn-tier files: module.unreachable           477 firing / 421 unpinned (1 prefix(es)) /  9 pinned
warn-tier files: proc.clobber-undeclared      486 firing /   0 unpinned (0 prefix(es)) /  3 pinned
warn-tier files: proc.out-unwritten             7 firing /   0 unpinned (0 prefix(es)) /  1 pinned
warn-tier files: proc.undeclared-fallthrough   91 firing /   0 unpinned (0 prefix(es)) / 10 pinned
```

* `module.unreachable`, unpinned under `games/`: 85 of its 94 files are game content and 63 of
  those were added within 30 days (45 are poison-test fixtures). The **engine side is pinned** —
  9 files, and 6 engine `.emp` modules were added in the same 30 days without one of them landing
  in the firing set, because an engine module normally IS in the profile closure. So an engine
  module falling out of it, its `ensure` guards going dark with nothing to say so, now fails by
  name. Those nine are the S21 adjudication's own list.
* `module.path-mismatch`, unpinned under `games/sonic4/data/generated/`,
  `games/sonic4/data/levels/` and `tools/fixtures/`: all 14 firings are generated level modules or
  anchor-sweep fixtures, 4 of them added within 30 days, every one inside those prefixes. Its
  pinned set is **empty**, and that is the assertion: no hand-written module in the corpus has a
  header disagreeing with its file, and 188 of the 202 `.emp` files lie outside those prefixes.

The 519 firings under an unpinned prefix are the residual, re-booked in the gap ledger. They are
smaller than the 1167 that were unwatched, they are counted in ordinary green output, and narrowing
a prefix is a one-line diff.

### The coverage that was lost is back

`898a97b1` deleted the suite's only assertion that `engine/objects/collision.emp` lowers with no
`[proc.undeclared-fallthrough]`. That file is absent from the `proc.undeclared-fallthrough` row, so
its first fallthrough fails the new gate by name — and generally, not for one hand-picked file.

## THE REGISTER'S FAILURE TEXT OVERCLAIMED, AND IT MATTERED

The old message read:

> The id is admitted in `WARN_ID_BASELINE`, so the id-set gate stays green and **this is the only
> thing that sees it, which is the whole reason the register is site-pinned.**

True only for an id that already has a row — the filter one screen above narrows the walk to those.
A reader meeting that sentence concludes they are covered, which is how the same claim reached a
committed sweep note the night before. It now states its scope, and says which gate holds the rest:

> What this register is the only thing to see, exactly: the SYMBOL and the COUNT, and only for an id
> that already has a row here — an id with no row is not watched by this walk at all. Its FILES are
> watched for every admitted id by `warn_tier_firing_files_match_the_pinned_sites`, which fails
> alongside this one when the file is new too.

The register's doc comment gained the same scope paragraph, and the module header now states which
of the three gates catches a new id, a new file, and a new symbol or extra firing.

## RED-FIRST EVIDENCE

Six mutations. Every one applied to disk and shown applied before its run; every restore from the
committed branch tip, never `git checkout --` on a dirty tree.

**M1 — the case the gate exists for, mutating the INPUT, not the baseline.** The reference tree is
read-only, so the corpus was copied to `.target-warntier/aeon-red` (`cp -a`, 68M) and the copy
first proven green (9/9). Then one line of `engine/objects/collision.emp`:

```
-proc Touch_SolidHurt () clobbers() falls_into Touch_Touch {}
+proc Touch_SolidHurt () clobbers() {}
```

A non-`import.no-names` id firing at a file with no register row and no `SITE_WATCH` row.

* On this branch: `warn_tier_firing_files_match_the_pinned_sites` **FAILED**, naming
  `engine/objects/collision.emp` in all 7 shapes with the lint's own message.
* On **master's** version of the same file, same mutated tree, same command: **7 passed, 0 failed.**
  That is the hole, demonstrated rather than argued: the class reached a file it had never reached
  and the whole test binary was green.

The remaining five, all against the unmodified reference tree:

| # | mutation | result |
|---|---|---|
| M2 | drop `engine/system/vectors.emp` from the pinned set | RED, "fired in 7 place(s) no SITE_WATCH row lists", file named |
| M3 | pin `engine/objects/collision.emp`, which never fires | RED, "fires in no shape any more" |
| M4 | admit `proc.mutation-m4-unwatched` in `CORPUS_LINTS` | RED, "admitted lint id(s) with no SITE_WATCH row" (and the id-set gate, separately) |
| M5 | widen `proc.out-unwritten`'s prefix to `engine/`, swallowing its own pin | RED twice: the self-disarm check and the vanished check |
| M6 | blind the walk to `module.path-mismatch` (the id whose pinned set is empty) | RED, "fired zero times across all seven shapes" |

M6 is the non-vacuity proof that matters: an empty pinned set cannot fail the appeared or vanished
checks, so without it a walk that stopped seeing that id would have read as coverage.

Restored from the committed tip after each: 9 passed, 0 failed.

## WHAT THIS PARCEL DID NOT DO

* **No re-baselining of anything that existed.** `CORPUS_LINTS`, `WARN_ID_BASELINE` and
  `CORPUS_OPEN_FINDINGS` are byte-identical to master. The new table is a dimension nothing
  watched, and its initial value can only be a measurement of what fires today; it is strictly
  stronger everywhere and weaker nowhere.
* **No register rows added.** The 1167 previously-unwatched firings are not adjudicated, and this
  parcel does not claim they are fine — it claims their FILES are now fixed, which is the property
  whose absence the ledger row was about.
* **The `games/` and generated-module populations stay unwatched at file granularity**, by
  measurement, with the count printed on every run.

The first run after the change was green, as a measured baseline must be, which is exactly why the
red-first proofs above are the evidence and the green is not.
