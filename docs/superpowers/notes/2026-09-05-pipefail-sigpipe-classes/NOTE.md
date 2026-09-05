# 2026-09-05 — SOURCE-GATE-PIPEFAIL-CLASSES: the class swept, two more live sites, one gate

`3aa83086` fixed `scripts/nightly_source_gates.sh`'s membership test, where
`printf … | grep -qx` under `set -o pipefail` read a MATCH as a NON-match. It fixed
ONE SITE and argued two neighbours safe. This is the sweep: the population derived
by more than one parameter, a verdict per site naming which of the three conditions
is present, the hand-argued sites ruled out by construction rather than by argument,
two more live instances fixed, and a lint so the next one cannot arrive unswept.

---

## 1. The mechanism, re-derived rather than taken on trust

`grep -q` exits the moment it MATCHES. Its writer is then killed by SIGPIPE and
exits 141; `pipefail` hands 141 back as the pipeline's status; an `if` on it takes
the ELSE branch **on a match**.

Three conditions must all hold:

- **(a)** an early-exiting reader as the pipeline's LAST stage;
- **(b)** a writer that can still be writing when it goes;
- **(c)** `pipefail` in effect **and** the pipeline's status consumed as a decision.

`docs/superpowers/notes/2026-09-05-pipefail-sigpipe-classes/repro.sh` measures it
directly, both arms, on a membership question whose answer is YES:

```
runs=9600  workers=64  (150 per worker, 16 cpus)
UNFIXED — `printf ... | grep -qx` under pipefail:
  unfixed: 44 wrong answer(s) of 9600
           44 MISS status=141
FIXED — the same question, no pipe:
  fixed: 0 wrong answer(s) of 9600
```

### 1a. "Serially it cannot appear" is refuted — this part stands

Both the founding commit ("0/4,000 serial") and this parcel's brief ("serially is
exactly the condition under which this defect cannot appear") state that a serial
run cannot show the fault. **It can.** The same script with one worker:

```
serial trial 1:  unfixed: 1 wrong answer(s) of 9600
serial trial 2:  unfixed: 7 wrong answer(s) of 9600
```

`0/4,000` was never evidence of impossibility: at a base rate near `1/9600`, zero
hits in 4,000 trials has probability **0.66** — it was the *modal* outcome. An
under-powered measurement read as a proof.

### 1b. My replacement mechanism was ALSO wrong — CORRECTED, see `boundary.sh`

> **CORRECTED 2026-09-05, same day, after the coordinator's challenge and my own
> re-measurement. The struck sentence below propagates a triage rule that is
> backwards for the sites that matter most, which makes it the more dangerous of
> the two errors. It is struck rather than deleted so nobody re-derives it.**

> ~~So load does not GATE the fault, it AMPLIFIES it — roughly 44× at 64 workers on
> 16 CPUs here.~~ ~~The practical consequence is the opposite of reassuring: a
> nightly lane that runs the audit ONCE, alone, on a machine doing nothing else,
> still has a real per-run chance of going dark.~~

**The variable is the WRITER'S SIZE, not concurrency.** `boundary.sh` sweeps it with
one worker and no concurrency anywhere, needle on the first line so the match is as
early as possible:

```
  writer = bash printf builtin
    bytes=4798      false-non-match=   0/400     0%  IMPOSSIBLE
    bytes=7198      false-non-match=   4/400     1%    <- the racing band
    bytes=9598      false-non-match=  83/400    20%    <- the racing band
    bytes=10798     false-non-match= 287/400    71%    <- the racing band
    bytes=14398     false-non-match= 394/400    98%  NEAR-CERTAIN
    bytes=23998     false-non-match= 400/400   100%  NEAR-CERTAIN
```

If the writer has already handed over everything it will ever emit, it is finished
and no signal is delivered — the fault is **impossible**, not rare. If it must still
issue one more write past the reader's exit, that write lands on a closed pipe — the
fault is **near-certain**, not rare. Concurrency only decides the narrow racing band
between.

**But the boundary is NOT "a pipe buffer", and getting that wrong is dangerous in
the permissive direction.** Three writers, the *same* 65536-byte pipe (read via
`F_GETPIPE_SZ`, not assumed), same reader, same machine, all serial:

| writer | last size at 0% | first size at ≥95% | vs. the 65536-byte pipe |
|---|---|---|---|
| bash `printf` builtin | 4,798 B | 14,398 B | **turns over BELOW it** |
| `seq` | 23,998 B | 239,998 B | ~4× above |
| `cat` | 479,998 B | 719,998 B | ~10× above |

A boundary that moves ~100× across writers on a fixed pipe is not the pipe's. What
governs is how much the writer must still push out past the match **in units of its
own output buffering**. *(That explanation is inference — no `strace` on this box.
The per-writer turnovers are measured.)*

So the triage question **"can this writer emit more than a pipe buffer past the
earliest match?"** would CLEAR a bash-builtin writer at 9,598 bytes that measures
83/400, and at 14,398 bytes that measures 394/400 — and the shell builtin is the
writer this repo's scripts actually use. **The usable rule is the smallest turnover,
not the pipe: treat a shell-builtin writer with more than a few KB past the earliest
match as near-certain.**

### 1c. The two measurements reconcile — neither replaces the other

The founding site's writer is `printf '%s\n' "${SOURCE_GATES[@]}"`: 46 names, ≈1.1 KB.
That is in the impossible-to-racing band, which is exactly why it took **9,600
concurrent** runs to show 44 and why a serial 4,000 showed none. Small writer → rare,
and scheduling decides. Large writer → near-certain, and nothing decides. My 44/9600
and the coordinator's 9534/9600 are the same mechanism sampled either side of the
turnover.

**The triage question is therefore static and per-site, needing no load harness and
no probability**: *can this writer produce more than a few kilobytes past the
earliest possible match?* A site that runs once, serially, in the nightly lane over
a large corpus is in the **near-certain** regime, not the rare one — which is the
opposite of what the struck sentence above implies.

---

## 2. The population, derived by five parameters and reconciled

**Hosts of shell in this repo** (`git ls-files`, 1836 tracked files):

| parameter | population |
|---|---|
| `*.sh` / `*.bash` / `*.zsh` extension | 79 |
| a `#!…sh` shebang on line 1 | 76 |
| reconciliation | the 3 with no shebang are `scripts/lib/sigil_tool.sh`, `scripts/lib/suite_paths.sh` (sourced libs) and `.s1probe/construct_probe.sh`; no file has a shell shebang without a shell extension |
| other shell hosts | `.github/workflows/ci.yml` (2 `run:` blocks); inline `bash` scripts written by `crates/sigil-harness/tests/{golden_write_gate,golden_freeze_atomicity}.rs` — **checked, and they contain no pipeline at all**, so condition (a) is absent throughout |

**Condition (c), pipefail in effect**: 53 of the 79 `.sh` files, plus both `run:`
blocks of `ci.yml`. Only **five** shell bodies also carry `set -e`, which is what
turns every command's status into a decision: `crates/sigil-harness/golden/{capture_goldens,derive_offcanonical_sizes,ab/region-hash}.sh`,
`scripts/provision-aeon-ref.sh`, and `ci.yml`'s report step. The first four contain
**zero shell pipelines** (`grep -nE '\|'` returns only two Python regex alternations
inside embedded `python3 -` heredocs), so the whole `set -e` half of the population
reduces to the one CI step — which is where the second live defect was.

**Condition (a), early-exiting readers.** Enumerated by class, then reconciled
against a structural pass over every pipeline in every pipefail file (178 pipeline
lines):

| class | raw hits | real |
|---|---|---|
| `grep -q` / `-l` / `-m` as a pipe consumer | 5 | **2** — `stability.sh:58–59` is `grep -c` (reads to EOF); `poscontrol.sh:52` and `nightly_source_gates.sh:410` have grep reading a FILE, not a pipe |
| `head` as a pipe consumer | 33 | 33 |
| `sed` with a `q` command | 0 | 0 |
| `awk` with `exit` | 1 | **0** — `landing-run.sh:211` is a `case` pattern `-h\|--help)`, and awk reads `"$0"`, not a pipe |
| `find -quit` / `grep -m1` / `--max-count` | 4 | **0** — all read files; `tree_state_capture_gate.sh:114`'s `find … -print -quit` has no pipe under it |
| `\| while read` / `\| read` / `\| xargs` | 0 | 0 |
| `tail` | 9 | **0 — `tail` is not an early-exiting reader.** It reads to EOF by construction. Every hit is either `tail` as the WRITER or `tail -c` on a file. This class is empty and stating that is worth more than leaving it unexamined. |

**The whole-repo structural scan** (`\|\s*(head\|grep -q…\|sed -n…q\|awk…exit\|read\|while read)`) returns 46 lines, of which 4 are Rust false positives (`\|\|` in
closures, a `head` local, a comment), 2 are comments about this very defect, and 1
is a `head=` variable assignment — leaving **39 real sites**, which is the union of
the class counts above. The two enumerations agree.

---

## 3. Verdict per site — which of the three conditions is present

The decisive question is (c)'s second half: **is the pipeline's status consumed?**
Of the 39 sites, exactly **one** was in a condition context and **one** was a bare
command under `set -e`. Every other one is a value:

- **`v=$(… | head -1)` under `set -uo pipefail`, no `set -e` — 16 sites.** (c) is
  absent: the assignment's status is never read and the decision is
  `[[ -n $v ]]` / `[[ -z $v ]]` on the VALUE. The value is complete by
  construction — `head` writes its lines and only then exits, so the SIGPIPE
  travels upstream, never downstream. `nightly_ref_drift.sh:276,311`,
  `tree_state_capture_gate.sh:83,160`, `diagnostics_sweep.sh:62`, and the probe
  scripts' `$(… | head -1)` are all this shape.
- **display-only pipelines mid-body — 21 sites.** (c) absent for the same reason
  and no `set -e` anywhere near them. Checked mechanically that none is the last
  command of a function or script whose status a caller reads: three are
  `LAST-IN-BLOCK` (`poscontrol.sh:58`, `aslr.sh:45`, `characterise.sh:132`) and all
  three sit inside a `for`/`while` body, not a function tail.
- **`scripts/lib/sigil_tool.sh:291`** — this one deserves naming because it is the
  only pipeline in the repo that runs under an inherited `set -e`
  (`provision-aeon-ref.sh` sources the lib). It is
  `"$(printf '%s' "$version_out" | head -3 | tr '\n' ' ')"` in ARGUMENT position to
  `sigil_tool_refuse`. A command substitution in an argument list does not trigger
  errexit — only one that is the entire right-hand side of an assignment does — so
  (c) is absent. The lib's other two pipelines end in `sed`, which reads to EOF.
- **the two live defects**, below.

---

## 4. The two hand-argued sites, ruled out by construction

`3aa83086` wrote: *"The other two early-exit pipelines in this file were checked and
are not decision-bearing … `accessor_closure`'s status IS consumed, but a spurious
failure there raises the loud UNMEASURABLE refusal, which is the safe direction;
left alone rather than changed blind."* Three claims, all argued. Mechanically:

1. **`reference_env_var` (`nightly_source_gates.sh:233–234`)** —
   `sed … | sed -n … | sort -u | head -1`. Condition (a) is PRESENT (`head -1`),
   and it is the last command of the function, so the function returns 141.
   **Its status is nonetheless never consumed, and that is a property of the
   consuming end, not of the function.** There is exactly one caller — line 334,
   `var=$(reference_env_var "$src")` — the file carries `set -uo pipefail` with no
   `-e`, and the very next line decides on `[[ -n $var ]]`. Verified by
   enumerating callers (`grep -n 'reference_env_var'` → definition at 232, one call
   at 334, one mention in a comment at 364), not by reading the function.
   **VERDICT: (c) absent at the consuming end. Safe. Left alone.**

2. **`REGISTER=$(sed -n … | grep -v '^test ' | head -20)` (line 620)** — (a)
   present. Assignment under no `set -e`; the value is used to print the open-findings
   register. **VERDICT: (c) absent. Safe. Left alone.**

3. **`accessor_closure` (lines 280–316)** — its status IS consumed (`accessors=$(accessor_closure "$src") || { … return 2; }`), so (c) is fully present.
   **Condition (a) is absent, by construction and not by argument: every one of its
   four pipelines ends in `sort -u`, and `sort` cannot emit a byte before EOF, so it
   cannot exit before EOF, so there is no signal for its writer to take.** That is
   the mechanical ruling-out the commit did not have. **VERDICT: safe by the shape
   of `sort`, not by the safe direction of its failure.** The commit's reasoning
   ("a spurious failure raises the loud refusal") was true but was answering the
   wrong question — it conceded the fault could happen and argued the consequence
   was tolerable, when in fact the fault cannot happen.

4. **`nightly_source_gates.sh:413–414`**, not mentioned by the commit:
   `! grep -qE "$obtains" <<< "$decommented" && ! grep -qF … <<< "$decommented"`.
   `grep -q` in a condition, under pipefail — but `<<<` is a **redirection, not a
   pipeline**, so there is no pipeline for `pipefail` to modify and `$?` is grep's
   own. **VERDICT: (b) and the pipeline itself absent. Safe by construction.**

---

## 5. What was actually wrong — two live sites, both fixed

### R1 · `docs/superpowers/notes/asl-reference/selfcheck.sh:85`

```sh
if printf '%s' "$msg" | grep -q "$VARYING_MD5" && printf '%s' "$msg" | grep -q "$STABLE_MD5"; then
```

All three conditions. The file's own `set -uo pipefail` (line 27); the pipeline's
status IS the `if`. Failure direction: **case 3 reports FAIL on a refusal message
that was correct** — the selfcheck for the asl reference guard turning red on itself.

> **CORRECTED — this verdict's REASON was wrong, its conclusion was not.** The
> struck sentence below prices the site by the mechanism §1b refutes.
>
> ~~`grep -q` matches on the first line of a short message, which is the WIDEST
> window for the writer to still be pending, not the narrowest.~~
>
> Backwards. `$msg` is ~200 bytes, far below the bash-builtin turnover of ~5–14 KB,
> so a short message puts this site in the **racing band** — the *rare* regime, not
> the widest window. That is consistent with what I measured and did not think
> about at the time: 30/9600 needed 64 workers, and the site is one that runs once.
> The verdict is unchanged (all three conditions present; fix it), and it never
> depended on the size: it is a defect because (a) and (c) hold, and the size only
> ever set how often it would bite.

Fixed to `if [[ $msg == *"$VARYING_MD5"* && $msg == *"$STABLE_MD5"* ]]`. Proven by
`selfcheck_case3_proof.sh`, which lifts BOTH constructs out of the two versions of
the file (`git show HEAD:` and the working tree) rather than retyping them:

```
baseline decision: if printf '%s' "$msg" | grep -q "$VARYING_MD5" && printf '%s' "$msg" | grep -q "$STABLE_MD5"; then
tree decision:     if [[ $msg == *"$VARYING_MD5"* && $msg == *"$STABLE_MD5"* ]]; then
BASELINE construct:  base: 30 wrong answer(s) of 9600   (30 MISS status=141)
TREE construct:      tree: 0 wrong answer(s) of 9600
RED on the committed construct (30/9600), GREEN on the tree's (0/9600).
```

### R2 · `.github/workflows/ci.yml`, the "report reference-dependent skips" step

```sh
set -euo pipefail
…
grep '^skip: ' test-output.txt \
  | sed -E … \
  | sort | uniq -c | sort -rn | head -40
```

Conditions (a) and (c) unambiguously.

> **CORRECTED — the pricing of (b) here was the clearest instance of the wrong
> mechanism in this whole note.**
>
> ~~(b) needs `sort -rn`'s output to exceed a pipe buffer, so on today's ~150 skip
> lines it is latent rather than live.~~
>
> "Exceed a pipe buffer" is not the boundary (§1b), and `sort`'s own turnover was
> never measured here — the sweep covered `printf`, `seq` and `cat`, and `sort`
> buffers differently again. What can be said without measuring it: `sort -rn`'s
> output is `uniq -c` groups, which **grows with the corpus**, and the entire
> purpose of this step is to make that growth visible. So the quantity my "latent"
> verdict rested on is one the site exists to let increase, and nobody re-checks it
> when it does. Pricing a defect by a number that is designed to grow is not a
> mitigation.

It is fixed either way, because the cost is zero. Two things were wrong at the site:

- **the SIGPIPE**: `head -40` exits, `sort -rn` takes 141, `set -e` fails the step —
  a red that says nothing about the code under test and arrives only when there was
  MORE to report. Also, `head -40` silently truncates the report this step exists to
  make visible, against this repo's own rule (`landing-run.sh:107`: *"never a tail
  excerpt, never `grep | head` — that has hidden failures behind"*). Replaced with an
  `awk` that reads to EOF and NAMES the tail it dropped.
- **an adjacent defect, same `set -e` reasoning, booked openly as adjacent rather
  than as this class**: when there are ZERO skip lines, `grep '^skip: '` exits 1 and
  `set -e` kills the step *before* the five-line ERROR block that explains exactly
  that case. **The one condition this step exists to shout about produced a silent
  red.** The zero check now runs BEFORE the breakdown.

Both proven by `ci_report_step_proof.sh`, which extracts the step's own `run:` block
from the workflow (baseline via `git show HEAD:`, tree from disk) and runs it against
two beds — one with no skip lines, one with 100,000 distinct ones:

```
================ bedA
  base  exit=1    explains-itself=no   … output_lines=4
  tree  exit=1    explains-itself=yes  … output_lines=9
  OK: baseline exit=1 with no explanation; tree exit=1 WITH it.
================ bedB
  base  exit=141  explains-itself=no   says-it-truncated=no   output_lines=44
  tree  exit=0    explains-itself=no   says-it-truncated=yes  output_lines=49
  OK: baseline exit=141 (SIGPIPE, 141); tree exit=0 and names the tail it dropped.
```

`exit=141` on the committed baseline is the class's own signature, produced by the
file as committed, not by a construct retyped for the occasion.

---

## 5c. All 39 verdicts re-read under the corrected mechanism

The question asked of every site: **did I price this low because it runs serially,
once, or over a small input?** Under the size mechanism none of those is mitigation.

**No verdict changed. Two REASONS did, and both were mine** (§5 R1 and R2 above).
Reading the list again, in the order the population was derived:

| group | count | did the mechanism move it? |
|---|---|---|
| `v=$(… \| head -1)`, status discarded, no `set -e` | 16 | **No.** Ruled out on (c), which the mechanism does not touch. The value is complete for a structural reason — `head` writes its lines *before* it exits — not a probabilistic one. Size is irrelevant to both halves. |
| display-only mid-body, no `set -e` | 21 | **No.** Same: (c) absent. |
| `scripts/lib/sigil_tool.sh:291` (argument position under inherited `set -e`) | 1 | **No.** Ruled out because a command substitution in an argument list does not trip errexit — a fact about bash, not about `head -3`'s input. |
| `accessor_closure`'s four pipelines | (within the 21) | **No.** `sort -u` cannot emit before EOF, so it cannot exit before EOF. Structural, and if anything the size mechanism *strengthens* it: there is no size at which `sort` acquires an early exit. |
| `reference_env_var` | 1 | **No** — but see the sharpened note below. |
| `nightly_source_gates.sh:413` (`<<<` here-string) | 1 | **No.** A redirection is not a pipeline; `pipefail` has nothing to modify. |
| R1 `selfcheck.sh:85` | 1 | **Verdict no, reason yes.** I called a short message the "widest window"; it is the *rare* band. Still a defect, still fixed. |
| R2 `ci.yml` | 1 | **Verdict no, reason yes.** I called it "latent" on a pipe-buffer threshold that is not the boundary, using a number the step exists to let grow. Still a defect, still fixed. |

**One verdict I want to sharpen rather than change, because the corrected mechanism
makes its margin much thinner than I wrote it.** `REGISTER=$(sed -n … | grep -v '^test ' | head -20)`
at `nightly_source_gates.sh:620`: the writer feeding `head -20` is a `grep -v`
streaming a `sed` range over the **whole strict suite log**. Under the size
mechanism that is not a borderline case — it is squarely in the **near-certain**
SIGPIPE regime, and it runs once, serially, in the nightly lane, which is exactly
the shape I would previously have called low-risk. It is safe **only** because (c)
is absent: the script sets `set -uo pipefail` with no `-e`, and the assignment's
status is never read. That is a one-word margin. **If anyone ever adds `-e` to
`scripts/nightly_source_gates.sh`, that line is in the near-certain regime from the
first run.**

I first wrote here that the lint already flags that transition. **I tested it
instead of asserting it, and I was wrong** — the mutation is worth recording because
it makes the `$(…)` residual concrete rather than theoretical. Adding `-e` to line
29 and running the lint:

```
1 pipeline(s) let SIGPIPE decide, out of 18 …
  scripts/nightly_source_gates.sh:233: sed … | sed -n … | sort -u | head -1
      `set -e` consumes every command's status, so a 141 from SIGPIPE aborts the script
```

Line **233** is caught. Line **620 is not** — it is `REGISTER=$(… | head -20)`, a
pipeline inside a command substitution, and §6 records that the lint does not read
inside those at all. So the transition is not silent (the script would go red on 233
and never ship), but **anyone who "fixed" 233 alone would leave 620 invisible and
believe the file was clear** — which is this parcel's founding failure shape,
one level up. That raises the `$(…)` hole from a documented residual to the item I
would close first (§9).

Same shape, same reasoning, lower stakes: `reference_env_var`'s `head -1` is fed by
`sort -u` over a handful of environment-variable names, so it is small *and* (c) is
absent at its single enumerated caller. Two independent reasons, either sufficient.

---

## 6. The gate — `crates/sigil-harness/tests/pipefail_sigpipe_lint.rs`

A sweep closes the sites that exist; it does nothing about the next one. The lint
reads every `.sh` in the tree plus every `run:` block of every workflow, folds
continuations, drops heredoc bodies, and flags a pipeline that ends in an
early-exiting reader when `pipefail` is in effect AND the status is consumed —
`if`/`elif`/`while`/`until`/`!`, an `&&` that gates what follows, or a bare command
under `set -e`.

**Red-first, four mutations, each shown on disk before the run:**

| mutation | result |
|---|---|
| restore `printf … \| grep -qx` in `nightly_source_gates.sh` (`git diff --stat` → 1 insertion, 5 deletions) | RED — 1 finding, "the pipeline's status IS the condition" |
| restore both fixed files to their committed defective form (`git diff --stat` empty ⇒ both AT baseline) | RED — 2 findings, naming `ci.yml` ("`set -e` consumes every command's status") and `selfcheck.sh:85` |
| `last_stage` returns no pipes (the reader parses nothing) | RED — `COULD NOT MEASURE: 84 shell unit(s) scanned and NOT ONE pipeline ending in an early-exiting reader was seen` |
| `"sh"` → `"sh_MUTATED"` (no `.sh` file enters the corpus) | RED — `COULD NOT MEASURE: only 2 shell unit(s) found` |

Each restore was from a byte-verified copy (md5 `1457df5d…` for `ci.yml`,
`dd0a87a0…` for `selfcheck.sh`, `d24743b5…` for the lint) or from the committed
baseline, never `git checkout --` over a dirty tree.

**The census reconciles exactly.** The lint counts 18 pipelines ending in an
early-exiting reader on the clean tree; 19 with the `nightly_source_gates` mutation;
21 with both baseline defects (`selfcheck.sh:85` is two `&&`-joined segments, so it
contributes 2). Against the shell-side scan: 40 candidate lines, 16 of them inside a
`$(…)` (no top-level pipeline) = 24, minus `diagnostics_sweep.sh:63` (folded into a
`$(…)` head), `tree_state_capture_gate.sh:83` (last stage `awk '{print $1}'`, no
`exit`), `gen_org_both.sh:20` and `poscontrol.sh:58` (last stage `sed` with no `q`),
and the two now-fixed sites = **18**. Both numbers were enumerated, not inferred.

**What the lint cannot see**, stated because the assertion of completeness and the
check that would establish it are separable:

- a pipeline inside a heredoc body — skipped wholesale, because the evidence files
  for this very class quote the defective construct inside one;
- **a pipeline inside `$(…)` — not read at all.** Under `set -uo pipefail` that is
  correct (status discarded); under `set -e`, `x=$(… | head -1)` IS a defect and the
  lint would miss it. Currently ZERO instances, because the only `set -e` bodies in
  the repo are the four pipeline-free golden scripts, `provision-aeon-ref.sh`, and
  the CI step. **It stops being zero the moment anyone adds `-e` to a script that
  has a `$(… | head)` in it** — which is most of `scripts/`. Booked as open.
- a pipeline assembled at runtime from a variable;
- `set -e` inherited from a sourcer rather than set in the file.

---

## 7. SKIP-TEXT-HOLE — the brief's second question, and the answer is no

The brief asks whether it is *still true* that "27 sites print `skipping …` while 146
use the `skip:` form" and that the nightly script "greps the same blind pattern".
**It is not still true. That defect was closed on 2026-08-27** (`c1718cb2` via
`df21c18f`; the lint's own last touch is `0ce85540`). Measured now:

- `crates/sigil-harness/tests/skip_marker_lint.rs` holds EVERY announced early
  return in `crates/*/tests/**` plus `test_support.rs` to the one `SKIP_MARKER`
  (`"skip: "`, defined once in `test_support.rs:1340`), via two independent
  detectors — structural (a print with a `return` within five lines) and lexical (a
  print using skip vocabulary) — whose UNION must carry the marker.
- `scripts/nightly_source_gates.sh:603` EXTRACTS the marker from `test_support.rs`
  rather than retyping it, and exits 2 if it is unextractable.
- `scripts/landing-run.sh:554` and `crates/sigil-harness/src/bin/refreeze.rs:575`
  count BOTH spellings (`/skip:|skipping/`).
- The 30 remaining `skipping` occurrences in `crates/` are doc comments, prose, and
  the assertions that forbid the spelling — not announcements. Enumerated, not
  sampled.

The lane log's own 2026-08-27 entry already corrected half of the original claim
("true for the SPELLING and false for CAPTURE"), and the spelling half was then
fixed too. **The two defects do not compound**, because the surviving anchored grep
(`ci.yml`'s `'^skip: '`) is consistent BY CONSTRUCTION with a lint that requires
every announcement to START with that marker. Nothing to fix; the booking is stale
and should be closed.

---

## 8. Verification, with the tree named

Run from `/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-aa6cafeb749d800b3`,
branch `parcel/source-gate-pipefail-classes`, at `dd7026f8` — and the log itself
carries that path, so it cannot be a green from somewhere else:

| bar | result |
|---|---|
| `cargo test --workspace` (`SIGIL_ALLOW_PARTIAL=1`) | exit 0 — **4551 passed, 0 failed, 2 ignored**, and `no_pipefail_decision_rests_on_an_early_exiting_reader ... ok` is IN that log |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, zero `error:`/`warning:` lines |
| `scripts/nightly_source_gates.sh --audit` | exit 0 — `SOURCE_GATES=46 scanned=138 source=45 artifact=87 no-reference=6 unclassified=0`. The new lint names no reference tree, so it does not enter that population at all. |

## 9. Left open

- **`$(…)` pipelines under `set -e` are outside the lint's reach** (§6) — **and
  this is now the item I would close first, demoted from "residual" by a measurement
  rather than a guess.** Zero instances today, but §5c shows the failure mode
  concretely: mutating `-e` into `nightly_source_gates.sh` reds line 233 and leaves
  line 620 — `REGISTER=$(… | head -20)`, whose writer is a `grep -v` streaming the
  whole suite log, i.e. squarely near-certain — completely invisible. Anyone who
  fixed the flagged line alone would believe the file was clear, which is the
  founding defect's shape one level up. Closing it needs the reader to recurse into
  command substitutions and decide whether the substitution is a whole assignment
  RHS — real work, and not this parcel.
- **Condition (b) is unjudgeable from source.** The lint flags (a)+(c) and says so.
  A site with (a) and (c) but a tiny writer is not currently faulty; it is one input
  size from being faulty, which is why it is flagged rather than excused.
- **The `sed -E 's#(set AEON_DIR)#\1#'` in the CI step is a no-op substitution**
  (it replaces a capture group with itself). Noticed while editing the line, not
  touched — it changes nothing today and diagnosing what it was meant to do is
  somebody's else's five minutes.
- **TAGGED FOR FOREGROUND**: nothing here needs the emulator. `selfcheck.sh` itself
  was not executed end-to-end (it needs two pinned `asl` builds at absolute paths
  outside this worktree); its case-3 construct was proven in isolation instead, and
  the file's other cases are untouched by this change.
