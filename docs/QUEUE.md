# Sigil queue rows, full text

**A ROW'S STATE LIVES IN ITS OWN HEADING, and this rule exists because the file had nowhere to put
a closing reason.** On 2026-09-06 one commit closed four rows and wrote all four closing reasons
into the tail of a fifth row's body. The result: four closed rows whose own headings read `open`,
and one genuinely open row appearing to carry four closure notices. Nobody misread it deliberately;
the file offered no other place to write. The four were rehomed 2026-09-09 and each says where it
had been sitting.

So: when a row closes, mark the HEADING (`## ROW: LANDED` or `## ROW: CLOSED`) and put the reason
directly under it. A reader scanning headings must never have to read a neighbour's body to learn
this row's state.

**And a row can go stale faster than a day.** `PINS-DEAD-TESTS-FIELD-AND-ORPHANS` was written into
the dispatch slot at 19:29 and the parcel it describes landed at 19:59, thirty minutes later, both
by the same seat in one evening. Before dispatching from any row here, ground its claims against the
tree first: a stale open row does not read as stale, it reads as work.

Verbatim row text, moved out of `docs/lane-status.json` on 2026-09-06 under
`empyrean/contract/LANE_STATUS.md` rule 7 (a row states its state; the history has its own file).
The board keeps a short title and this file keeps the argument. Rows closed on that date are kept
here with their closing reason rather than deleted.

## AS-NAMELESS-LABELS-RC1

- state: **LANDED** 2026-09-10 at merge `6b26f7f7`, pushed  size: `L`  project: `SIGIL-AS-REPLACEMENT`
- Landed on `d-29`, answered `land` **by the HUB** under the owner's delegation, not by him; he overturns
  it with one word. Landing run GREEN: 5,017 passed / 0 failed, clippy clean, zero skips. Corpus headline
  reproduced independently at landing (149 rows, `$F9198`, `$989C`, zero unresolved slots).
- history, kept: blockedBy the owner, as decision **d-29** filed 2026-09-09: THE WORK IS NOW BUILT AND UNLANDED on branch
  `parcel/as-nameless-labels` (tip `c22f6050`, six commits, nothing merged). The hold below still stands and was
  correct; a 2026-09-09 session REVERSED IT WITHOUT READING IT, dispatched the parcel, and told the owner the item
  had never been waiting on him. That was wrong. The question put to him is no longer whether to spend the effort,
  which is already spent, but whether the feature lands. Size L sent for his eye, and no owner decision exists on
  the item itself. A hub go is not his go.

THE Sonic 2 item, now measured: AS nameless temporary labels (+, -, /), unimplemented entirely, are the single root cause of 4,985 rows across four message classes, 86.5 percent of the whole run. ACCEPTED BY THE HUB (d-22, 2026-09-03, answered by: hub, under the project declaration) and NOT BY THE OWNER: this item has had no owner decision of any kind, only a size sent to him. On 2026-09-06 the hub sent a go resting on d-22 read as an acceptance in hand; this lane held and the hub withdrew it. Do NOT size the post-fix count by subtraction: closing it reaches code currently abandoned and can add rows as well as remove them.

## AS-MESSAGE-AND-INTERPOLATION: LANDED 2026-09-07, and this row sat open for two days

- state at archive: `LANDED`  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; nothing is left to do

**BOTH halves shipped at `67fdea97`** (2026-09-07 12:16 local, verified an ancestor of master),
*"`message` reaches stdout on the converged pass; `\{expr}` pastes floats and strings"*, 39 files
including asl probe transcripts. `message` now carries a stream of its own out of the front end as
`Assembled::messages` / `Failure::messages`, one line per firing from the converged pass, and the
`a_message_is_dropped_on_every_pass_including_the_final_one` pin was deleted on purpose as this row
asked. The float interpolation half is pinned too: `as_interp_shapes.rs` asserts
`("(2048-1024)/1024.0", "1")` and `as_message_stdout.rs` carries the exact ROM-size idiom the row
named. Reading the row's text against the tree, every clause it states as outstanding is done.

**⚠ THE ROW WAS THE NEXT THING THIS LANE WAS ABOUT TO DISPATCH, AND ONLY GROUNDING CAUGHT IT.** On
2026-09-09 the fallback sequence reached this row, and the brief was one command from being written.
What stopped it was reading the message arm in `eval.rs` before describing it, which is a habit this
lane adopted tonight only because three earlier briefs had each named a wrong location. **A stale
open row does not read as stale; it reads as work.**

This is the boot read's own banked defect arriving a second time: a parcel's completion has to be
written back to the document that DISPATCHES it, and no gate anywhere makes that happen. The first
instance was the R7 block saying *"what remains is the flip itself"* for a week after the flip
landed. **The cost here would have been an agent sent to build what exists**, and the likelier
failure is not wasted effort but a plausible second implementation of a solved problem.

The old row text is kept below, unedited, because a reader who has already carried it away needs to
meet the correction rather than find the row silently rewritten.


Two rows that must land as ONE parcel, per the census. (a) sigil's message arm evaluates its string and discards it on EVERY pass, converged included, so it is not a pass question at all and the old row pairing it with warning was wrong. asl writes it to STDOUT, unprefixed and outside the diagnostic stream, so implementing it moves NO corpus diagnostic count while changing what a runner capturing stdout sees. 39 sites; two print under asl today and sigil prints neither. (b) the one corpus site that would fire needs an interpolation form sigil drops: \{(EndOfRom-StartOfRom)/1024.0} comes back UNINTERPOLATED, and that idiom is how all five s2disasm table-size messages are written. warning shares interp_string and inherits this the day a corpus site fires. Deleting the a_message_is_dropped pin is part of the work and is deliberate, not incidental.

## EMP-Z80-MNEMONIC-TABLE: LANDED

- **REOPENED 2026-09-25 as card `d-34-reopened`:** the hub withdrew its ruling at empyrean `8a8c29ad` (outside the delegation, d-6 reserves spellings to him). Stays LANDED until he answers; a revert moves no ROM byte.
- state: **LANDED** 2026-09-25 at merge `bc88480d`, pushed. Landed on `d-34`, answered `land-all-21` **by the HUB** under the owner's delegation (empyrean `796b508f`), not by him; he overturns it with one word at no cost while no game source uses the `(c)` port spelling. Landing run on the merged tree: 494 suites, 5611 passed, 0 failed.
- history, kept:

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: you: this is language surface, not assembler internals

TAGGED by the Z80 coverage parcel and deliberately not done. The AS side now encodes 21 more instructions; our own .emp language's Z80 mnemonic table was NOT extended to match, because what the game can be written in is language surface and yours rather than mine. Extending it also needs a new operand kind for the in/out port-in-C form. Nothing depends on it today.

## ROWREMAP-HEAD-LABEL-RULED: CLOSED

- state at archive: `open`  size: `S`  project: `EFFECTS-W1`
- blockedBy: the engine lane: the fix is at their generator, not here

SENT to aeon 2026-09-06 with its anchor: row D2 in docs/superpowers/notes/2026-09-05-decouple-aeon-side-inventory.md (b0cf2eeb). RULED, no sigil change: the ojz_effects_editor_act1 order row must key by SECTION NAME, not head label, because the head label is content-derived. Nothing breaks today (SECTION-ROW resolves name to head label at placement); the fix is at the engine lane's generator, and aurora's read is that only determinism AMONG tables is required, so it is probably a one-line move, unproven until a build refuses or does not.

**CLOSED 2026-09-06 as far as this lane goes: ruled, no sigil change, sent to aeon with its anchor. The fix is at their generator.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## AEON-TOOL-DEFECTS-NAME-AS-POSITION: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

SENT to aeon 2026-09-06 with paths and SHAs; they book from the files, not from a restatement. Five findings for the engine lane, all committed in sigil since 2026-09-05 and reachable from origin/master: the lint table already wrong, four mis-measuring routine-extent copies, pointer identity by name-string (these three are sections S1/S3/S9 of docs/superpowers/notes/2026-09-05-consuming-end-name-as-position.md), plus a margin decaying 440 B/day into a forbidden band and Knuckles' ceiling equalling its bar with nothing saying so (2026-09-05-p6-resweep-current-layout.md, section 6, items 2 and 3). RENAMED from AEON-S1-S3-S9-ROUTED: the old id was built from this lane's own internal section numbers and was unreadable to the only lane that needed it, which is how aeon came to believe the findings were lost in a message while they sat committed and unread. Recording was never the failure; routing was.

**CLOSED 2026-09-06. The aeon lane worked all five at aeon 36e3e409: four held and landed, S1 declined on the owner's scope cut with its ruling recorded. One of the five refuted a measurement of this lane's (see the bound-versus-ceiling correction at sigil 4221cfee).**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## STABILITY-RUNNER-MISSING-WHERE-CLAIMED: SWEPT 2026-09-16, and the "five" is retired as unfounded

- state at archive: `open` -> swept  size: `S`  project: `-`
- blockedBy: nothing

Original text, kept verbatim: *"Sweep owed, now with a named population: five notes from the same
week state their ground truth as the UNRELIABLE copy cited by version banner, with no committed
probe directory to re-run."*

**"Now with a named population" was not true: the row names none of the five, and no instrument
returns five.** The 2026-09-08 staleness sweep reached the same conclusion and refused to invent a
population; that refusal is upheld rather than overridden. **The count is retired as an unfounded
figure** of the same family as the three premise-less rows found the same morning.

**What was done instead**, note `docs/superpowers/notes/2026-09-16-asl-citation-form-sweep.md`,
instrument `scripts/sweep_asl_citation_form.sh` (committed, which is this row's own standing fix
applied to itself): all four `asl` binaries here print the same version banner, so a note citing its
oracle by banner has named nothing. **22 notes cite the banner and never an md5.** The script prints
the list and carries its own scope limits, a positive control, and the two workspace facts it depends
on. Figures move when notes discussing the defect are edited, so the script is the authority.

**Left open deliberately:** whether any of the 22 states a claim that actually DEPENDS on which
binary produced it. That needs reading rather than grepping and is a separate parcel. The standing
fix stands: a stability or provenance claim cites a committed script or an md5, or it is not made.

**2026-09-26: the open question is answered.** The sweep at `9212d6e0` returns 22; read against
what is actually known to differ between the four builds, and re-run on all four where the input
survived or could be rebuilt: **3 DEPENDS, 19 DOES-NOT-DEPEND, 0 UNDETERMINED**, and every DEPENDS
row is already settled or superseded. Note
`docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`, probes and the all-builds
runner beside it. The three are booked below.

### ASL-BANNER-DEPENDS-THREE: CLOSED 2026-09-26 at landing (merge `7478f0fe`), no re-run owed

- was: state `open`  size: `S`  project: `-`
- blockedBy: nothing

Three banner-only notes state a claim that depends on which `asl` produced it. Each is settled or
superseded in `docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`:
`2026-09-03-as-macrosetup-three-sites.md` (a whole-Sonic-2 run's silence at 34 lines, on
`0dee1f98`; superseded, `AS-WORD-IMM-RAM-LABEL` closed at `2026-09-04-as-end-probes/README.md`),
`2026-09-07-as-nesting-relex.md` (asl timing, taken through `asl_run`, which accepts only
`61e67256`; settled), `2026-09-08-extra-traversal-measure.md` (asl timing and a whole-`s2.asm`
success; identified as `61e67256` for s1disasm and `0dee1f98` for s2disasm from its surviving
scratch copies; settled). The notes themselves are historical and were not edited.


### AS-REPLACEMENT-STATE: LANDED

- state: **LANDED** 2026-09-27 at merge `6128bb1e`, pushed  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Measurement only: `docs/superpowers/notes/2026-09-27-as-replacement-state.md`. Headline: all five stock entry
  points build byte-identical through sigil's own command; no disassembly's own build script can use sigil as
  `asl` today. The note's section 8 is the source of the five rows below; re-derive before dispatching any.
- Overseer re-check at landing: S1 `afe05eee`/524288 and S3K `0658f691`/2097152 rebuilt both ways from fresh
  archives (s1disasm `f6ece65`, skdisasm `2fcd861`), zlib CRC-32, cmp identical, planted-byte control seen.
- The first run of this parcel died uncommitted in the 2026-09-27T09:14Z lane restart; its worktree
  `agent-a83b3b56407567d30` (branch `parcel/as-replacement-state`) holds only scratch and can be pruned.

### AS-DROPIN-ASL-CONTRACT: LANDED

- state: **LANDED** 2026-09-27 at merge `31c65791`, pushed. Full record: the merge body and
  `docs/superpowers/notes/2026-09-27-as-dropin-asl-contract.md`. Deviations ratified: `-U` required in asl
  mode; the two S2 `ACK_DISAGREE` entries scoped to the direct route rather than removed. Left open, per the
  note: `-L` writes no listing, `-i` takes only the source's own directory, one source per call, a `before`
  blob after a run over 0xFFFF bytes may be placed differently (no corpus has it), no Windows build.
- original row, kept:
- state: **next**  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Let a disassembly's unmodified `build.lua` run sigil in place of `asl`: accept asl's flags
  (`-xx -n -q -A -L -U -E -i . -c -D`), write a `.p` the stock `p2bin` accepts, the `-c` share file and the
  `-E` log. Sized off the shim (5/5 stock ROMs) and the `.p` writer probe (3/3 with p2bin md5 `4f2fff99`),
  note sections 3 and 8. Measured constraints: records keep emission order, and the run just before each Z80
  record stays separate. Should also retire both S2 `ACK_DISAGREE` legs (share file), proved by the option
  sweep through the drop-in.
- Invocation shape is forced by the goal (the script must not be edited), so it is this lane's call under
  `d-2`, logged as a notable call when made, not an owner blocker. No `.emp` surface.

### AS-DROPIN-NIGHTLY

- state: **LANDED** 2026-09-28 at merge `38d6aead`, pushed; landing run GREEN 5846 passed 0 failed (5846 + 0 new, scripts and docs only). First two-route nightly: 02:17 2026-09-29, read `~/.local/state/sigil-switch-sweep/nightly.log`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The nightly switch sweep runs only the direct route; `--route dropin` exists since `31c65791` (40/40, and
  808/808 with `--cross`, on 2026-09-27). Add a drop-in pass so the route a disassembly user takes is watched
  every night. The direct route's S2 share-file gap stays acknowledged there.
- 2026-09-28, on the branch: `nightly_switch_sweep.sh` runs `--cross` twice, `--route direct` (log
  `sweep.log`) then `--route dropin` (`sweep-dropin.log`), one sigil build and one pair of corpus SHAs for
  both; each pass reconciled by the direct pass's rules plus its own `ROUTE` line; the dropin pass runs even
  when the direct one could not; both share a four-hour budget inside the unit's five; one verdict line names
  both routes, exit the worse of the two. Hand run at `f79c0059`, GREEN: direct 808 legs (790 planned + 18
  rescue) 22m23s, dropin 808 legs (790 + 18) 24m38s, 47m19s job wall. Reds proved through a log-replaying
  stub sweep: dropin marker dropped exit 2, dropin finding exit 1, dropin log naming the wrong route exit 2,
  direct abort exit 2 with the dropin pass still run green, budget spent exit 2 with dropin not started.

### LANDING-LOG-CHECKER-READS-THE-WRONG-FILE

- state: **LANDED** 2026-09-28, fast-forward to `17361152` (fix `28e51dc8`), pushed; landing run GREEN at `f37dfc77`, 5852 passed 0 failed  size: `S`
- `scripts/check_landing_log.py <log>` reports NO VERDICT, RED on the log `landing-run.sh` writes (`$LOG`),
  because the script echoes its verdict block to stdout only and writes just the `*_EXIT=` lines into `$LOG`.
  Measured 2026-09-27 on the drop-in parcel's `landing-2.log` and on `.landing-audit-194834.log`; the
  `.stdout` capture passes the checker while it reports every exit gate NOT REPORTED. `landing-run.sh
  --verdict-only <log>` judges `$LOG` correctly. Make one artifact carry both, or point the checker's docs
  at `--verdict-only`; either way the two instruments must agree on one file.
- 2026-09-28, on the branch: a run appends its verdict block to its own `$LOG` inside a
  `##### VERDICT SPAN` with `LANDING_EXIT=<exit code>`, computed first and appended after, so the verdict
  never reads a growing file; a failed append exits 2. The verdict is one function over the RUN RECORD, the
  log above that span, because the block quotes run-shaped text (a silent binary's own `Running` line) that
  a re-judge reading the span would count again. `--verdict-only` writes nothing. The checker reads RESULT
  from the one span only and the exit lines from the record; exit 0 needs a recorded GREEN, `LANDING_EXIT=0`
  and all three gate exits 0. Refused with exit 2: a pre-span log (naming `--verdict-only <log>`), a captured
  stdout (naming the log), two spans, a truncated span, and a GREEN its exit lines contradict. Header (9a),
  usage and the checker docstring now name the log and both instruments.
- Verified: `landing_verdict.rs` 23 to 29 tests; three run the real script in run mode against a stub
  toolchain (stub cargo on PATH, stub ledger and census). Red-first: pre-fix append (2 red), run record
  reading the whole file (1 red, 2 launched read back as 3), `--verdict-only` appending (1 red), pre-fix
  checker (5 red). Mutation evidence is in the commit bodies.
- Landing run at `f37dfc77` (clean), reference `.aeon-sigil-ref` @ `ec640bcf`: GREEN, 5852 passed 0 failed
  2 ignored, 519 suites, 519 launched = 519 reported = census, 5846 + 6 new, clippy and ledger clean. On
  its `$LOG`: `check_landing_log.py` exit 0 `RESULT GREEN` with `LANDING_EXIT=0` and all three gate exits 0;
  `--verdict-only` exit 0 `RESULT GREEN` twice, byte-identical to each other and to the recorded block, log
  md5 unchanged.

### AS-AUTHOR-FORMS-EXACT

- state: **LANDED** 2026-09-27 at merge `cb5ab42b`, pushed; landing run GREEN 5784 passed 0 failed (5770 + 14 new), clippy clean  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- 31 of 167 common 68000 spellings a hack author writes are refused by sigil and accepted by `asl`; the three
  disassemblies use none, so no corpus test sees it (note section 4). Twenty are plain, plus unquoted
  `binclude` and refusing `swap.w`. Stage 0: check whether refusing the `adda`/`eori` shorthand was a ruling;
  if it was, those rows go to the owner.
- Stage 0 answered: the ISA's refusal of `add Dn,An` is the `01121362` fix for a silent ADDX mis-encode, not
  a ruling against the alias. It stays on the `.emp` route; the AS route rewrites to `adda`/`suba` as asl does.
- 2026-09-27, on the branch: every form byte-identical to the pinned asl (md5 `61e67256`, `asl_run`, exit 0),
  probes in `docs/superpowers/notes/2026-09-27-as-author-forms-exact-probes/`, tests
  `sigil-frontend-as/tests/as_author_forms.rs`.
  - ISA (both routes' encoder, `.emp` spelling unchanged): `chk.w`, `link`, `unlk`, `reset`, `rtr`, `stop`,
    `trapv`, and `eori.b #,ccr`, `andi.w`/`ori.w`/`eori.w #,sr`, which sigil refused even SUFFIXED. Decoder
    and capstone sweep agree, no new exclusion.
  - AS route only: the eight mnemonics; `.b` branch suffix; unsuffixed `andi`/`ori`/`eori` to `ccr`/`sr`;
    `add`/`sub` to An as `adda`/`suba` (every source mode) and `eor #imm,Dn` as `eori`; `$1234.w`/`.l`
    unparenthesised, and `.W`/`.L` in either form; bare `binclude` paths.
  - Refusals added, each an asl refusal sigil accepted: `swap.b`/`.w`/`.s` (asl takes bare and `.l`),
    `andi.w`/`ori.w`/`eori.l` to `ccr`, and suffixes on `reset`/`rtr`/`trapv`/`stop`/`illegal`/`unlk`.
  - A silent-wrong-bytes defect found beside item 6: `binclude "f",off,len` ignored `off` and `len` and
    emitted the whole file. Now sliced as asl slices it. No shipped source or corpus uses the operands.
  - `.emp` unchanged, pinned against master `3ceed19b`: `sigil-frontend-emp/tests/as_author_forms_stay_off_emp.rs`.
- Left open, each recorded not guessed:
  - `chk.l` assembles under asl (`4300`, the 68020 form) and sigil refuses it by name; bare `chk` assembles
    under asl (`4380`) and sigil refuses it with bare `muls`/`divs`, under AS-UNSIZED-DEFAULTS.
  - A `.s`/`.b` branch to the next instruction: asl emits `4E71` (a NOP) and sigil refuses the zero
    displacement. Unchanged here.
  - ~~`jmp Foo.w` with `Foo` a symbol assembles under asl (`4EF8 1234`); sigil lexes `Foo.w` as one name.~~
    Done in `AS-SYMBOL-SIZE-SUFFIX` below.
  - `.emp` spelling of the seven new mnemonics: needs `link`/`unlk` in the clobber model and `rtr` in the
    terminator set, and is an owner-reviewed language change.

### AS-SYMBOL-SIZE-SUFFIX

- state: **LANDED** 2026-09-28, fast-forward to `5bc05325` (fix `88eb2e35`), pushed; landing run GREEN at `94df424e`, 5859 passed 0 failed  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- From AS-AUTHOR-FORMS-EXACT's "Left open": `jmp Foo.w` with `Foo` a symbol. sigil's lexer makes `Foo.w` one
  identifier (`.` is an identifier character, `Parent.local` a local label), so sigil refused it, and read
  the dotted name SILENTLY where one existed (a `Foo.w` equate, or a local `.w` under `Foo`).
- Stage 0, pinned asl (md5 `61e67256`, `asl_run`), 97 probes in
  `docs/superpowers/notes/2026-09-28-as-symbol-size-suffix-probes/` (`asl.out`, `sigil-before.out`,
  `sigil-after.out`). asl peels a trailing `.w`/`.l`/`.b`/`.s` off an effective-address operand BEFORE the
  lookup: `jmp Foo.w` is `Foo` at word width even when `Foo.w` exists; out-of-window values are #1340
  (no silent truncation, forward or not); `.b`/`.s` on an absolute are refused. It reads the name WHOLE in
  `dc`, `#imm`, `(Foo.w).w`, `Foo.w+2`, and in branch and `dbcc` targets (`bra.s T.w` goes to the local
  `T.w`). The peel also covers `(Foo.w)` and displacements (`.w` before `(An)`/`(pc)` and in `(d,An)`, `.b`
  before `(An,Xn)`). The three disassemblies and the aeon reference tree spell none of this (scan: 0 hits
  each, 2 of 2 in a planted control), so their builds could not move.
- Changed (AS route, `operands.rs`): `peel_width_suffix` applied to absolute, `(abs)` and displacement
  operands with the width asl accepts there, every other width refused by name; branch and `dbcc` go
  through `parse_target_operands`, which reads the name whole. Numbers take the same peel, so `4.w(a0)`,
  `(4.w,a0)`, `0.w(pc)`, `4.b(a0,d0.w)` and `($1234.w)` now assemble, each measured. `.emp` unchanged.
- Divergence, recorded not matched: `move.w Foo.b(a0),d0` exits 0 under asl with no diagnostic and NO BYTES
  for the line; sigil refuses it.
- Verified: `sigil-frontend-as/tests/as_symbol_size_suffix.rs`, 6 tests over all 97 probes, bytes from
  `asl.out` only; sigil agrees with asl on 96 of 97 (41 before, with 11 probes exit 0 on bytes or acceptance
  asl does not give), the 97th being the divergence above. `.emp` pinned at `4915361c` in
  `as_author_forms_stay_off_emp.rs`. Red-first: pre-fix `operands.rs`/`eval.rs` (5 of 6 red), branch
  targets through the peel (1 red: `bra_local_w`, `bsr_local_l`, `dbf_local_w`), any displacement width
  accepted (1 red), and an altered `.emp` pin expectation (1 red). Landing run at `94df424e`, reference
  `.aeon-sigil-ref` @ `ec640bcf`: GREEN, 5859 passed 0 failed 2 ignored, 520 suites launched = reported =
  census, 5852 + 7 new, clippy and ledger clean; `check_landing_log.py` and `--verdict-only` both GREEN.
  Switch sweep (s1disasm, s2disasm): 40 legs launched = reported, 0 unacknowledged, SWEEP PASSED.

### AS-UNSIZED-DEFAULTS

- state: **open, owner question**  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- `asl` silently defaults an unsized instruction to `.w`, accepts bare `ds`, and sizes unsized branches itself;
  sigil refuses all three (note section 4). Matching `asl` means accepting a silent default in the AS frontend,
  against this lane's refuse-by-name stance (AS-DEFAULT-CPU ruling). Mechanics exist (`RelaxLadder`); the
  question is direction, so it is put to the owner before any dispatch.

### AS-DIAG-FATAL-FIRST

- state: **landed** (merge `cf520369`, landing run GREEN 5770 passed 0 failed)  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- When the source's own `fatal` fires (S3K, a FixMusicAndSFXDataBugs build), sigil prints 563 error lines and
  the cause is line 468; `asl` prints one (note section 5). Report the `fatal` first and drop the unresolved
  follow-ons it caused.
- 2026-09-27, on branch `parcel/as-diag-fatal-first`, since merged: the `fatal` is printed first; an error
  is dropped, and counted in one note, when a second assembly that does not stop at the `fatal` no longer
  raises it. S3K leg: 563 lines with the cause 468th became the `fatal` plus `562 further errors not
  reported`, exit 1 both. Tests: `sigil-cli/tests/as_fatal_reported_first.rs`.

### AS-PERF-MEASURE

- state: **LANDED 2026-09-28 at merge `4bbc972b`** (tip `e01364c8`), measurement done  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- Answer: `docs/superpowers/notes/2026-09-28-as-perf-measure.md`. Extra front-end passes (S1/S2/S3K 3/4/3 vs asl 2)
  are 94 to 98 percent of wall; layout's second copy of every section sets peak RSS. The candidates below measured
  under 0.5 percent each. The fixes are booked as `AS-PERF-FIX-PEAK` (next) and `AS-PERF-FIX-PASSES`.
- sigil is 2.0x to 2.5x slower than `asl`+`p2bin` and uses 5x to 18x the memory on the `.asm` route (load 9 to
  11 at the time, note section 6). No profiler here; per-phase timing is a code change, so measure first.
- 2026-09-27, CORRECTED 2026-09-28: the first dispatch was NOT dead. Its transcript went quiet for 14 hours while it
  waited on its own backgrounded release build (the brief forbade waiting on a notification; it did anyway), and the
  overseer removed its worktree on the transcript's silence alone, without checking for a live process. It then
  reported the loss. Nothing committed was lost; its read-only findings, below, are the head start.
- Environment facts it measured: no `perf`, `valgrind`, `samply`, `heaptrack`, `hyperfine` or `/usr/bin/time`, so CPU
  seconds come from `getrusage` in-process. The worktree-isolation hook refuses compound shell commands mentioning git;
  single plain commands pass. Tell the next agent both, and to run its build in the FOREGROUND.
- Code candidates (read, NOT measured), `crates/sigil-frontend-as/src/eval.rs`: `one_pass_with_defer` builds a fresh
  `Asm` and clones the whole symbol environment, macro, function and known-label tables every pass; every `include`
  is re-read from disk each pass through `sigil_span::read_set` (CRC-32, canonicalise, mutex), given a fresh
  `SourceMap` entry and re-lexed by `split_src_lines`; `run_passes` keeps a whole `SymbolTable` per pass in `history`
  for the oscillation proof (a peak-RSS candidate growing with pass count); a converged run with `force_relocate` or
  leftover poison runs a bonus pass. CLI phases after the front end (`run_asm`, `sigil-cli/src/main.rs`):
  `resolve_layout_placing`, `link`, `check_image_bounds`, `flatten_placing` (compression), `apply_sega_header`,
  `emit_image`. Its planned instrument: env var `SIGIL_PHASE_TIMING`, read once via `OnceLock` like `SIGIL_CENSUS_*`.

### INOUT-PROOF-INVOKE-HOOK

- state: **LANDED 2026-09-28 at merge `87cdb675`** (tip `f1c7b70a`)  size: `S`  project: `-`  reported by: aeon, 2026-09-27
- Reproduced and real: three checkers read only a bare `Sym` as a call target, and a bound `invoke` lowers to
  `jsr (Proc).l` (`AbsSym`). One shared reader, `flag_check::direct_proc_target`, now takes both. Full record:
  `docs/superpowers/notes/2026-09-28-inout-proof-invoke-hook.md`. aeon's `Rings_AnimTick` split is now optional.
- **Booked from it, open:** `preserves.rs` `call_target` is a fifth `Sym`-only reader feeding the dead-save walk; a
  fix there can move bytes, so it is its own parcel. Also: the worktree tree-walk tests skip only a dir named
  `target`, so a cargo target dir named anything else leaves scratch copies that red `scripts_name_their_tree`.
- *(Report as filed, kept below.)*
- aeon's agent reports that while landing their merge `9496bbb3` (verified on aeon `origin/master` here), our
  contract checker treated `invoke Game.ring_frame` inside `DrawRings` as an unknown callee and failed its
  `inout` proof (d5/a4), so they moved the tick into its own proc. A hook call has a declared clobber bound,
  so an `inout` proof across it should be decidable. **Unreproduced here.** Stage 0: rebuild the pre-workaround
  shape (aeon's `parcel/vram-tier1` history, or a minimal `.emp` with an `invoke` of an interface hook inside a
  proc carrying an `inout` claim) and see whether the refusal is real and whether it is the right answer.
- **Reproduction pointer (aeon, verified here):** the only record is the body of aeon `3d369bc7`, reachable from
  their `origin/master`, lines 16 to 17: the inlined `invoke` fired `[proc.inout-unverified] DrawRings ::
  inout(d5)/inout(a4)`. The failing file was never committed; rebuild it by inlining `Rings_AnimTick`'s body
  (its `invoke Game.ring_frame`) back into `DrawRings` in `engine/objects/rings.emp` at `3d369bc7`.
- Related, not a defect: their `rings_port` byte change reaches us only at the next pin advance
  (`PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS`); the reference tree stays at `ec640bcf`.

### PHASE-ROW-CPU

- state: **open, parked**  size: `S` to `M`  project: `-`  asked by: oracle, 2026-09-27
- **The ask:** a trailing token on each listing `PHASE` row naming the processor, so oracle stops guessing
  whether a phased VMA is a Z80 or a 68000 address (it named real 68000 code at `$8000` as
  `SoundTablesZ80_Head+...`).
- **Why it is parked, found at step 0 and verified by the overseer at the remote tips 2026-09-27:**
  (1) `Section.cpu` names the ENCODER (`crates/sigil-ir/src/lib.rs`, "which CPU this section's instruction
  bytes target"), not the bus the VMA belongs to. All six live rows come from aeon
  `games/sonic4/data/sound/soundbankhead.emp:58`, `section soundbankhead (cpu: m68000, vma: $8000)`, which is
  68000-assembled data in the Z80 bank window, so threading `sec.cpu` prints the wrong answer on every live
  row. What oracle needs is the VMA's ADDRESS SPACE, which no `.emp` section records
  (`AddressSpace::Foreign` is set only by the AS frontend and describes the LMA). Recording it is a new section
  attribute, so language surface: propose, discuss, land.
  (2) Two readers break on a trailing token: oracle `crates/oracle-core/src/symbols.rs:1537` matches exactly six
  tokens (origin/main `980d0d7d`), and aeon `tools/clip_anchors.py:107` is `\s*$`-anchored (origin/master
  `aa9cb67b`). Both loosen first, before any row bytes change.
  (3) Spelling, if it lands: `.emp` says `z80`/`m68000` (d-27, `m68k` refused), the AS frontend and linker
  diagnostics say `Z80`/`68000`. Undecided.
- **Oracle's interim rule** (VMA != LMA means the VMA is a Z80-window address, the 68000 address is the LMA) is
  right for the whole live population. **Kill condition / trigger:** a phased 68000 block whose VMA is a 68000
  address (code copied to RAM) appears in aeon; then this row becomes a language proposal to the owner.

### ASL-CITATION-SWEEP-FOLLOWUPS: LANDED

- **LANDED 2026-09-27 at merge `103c2771`.** All three done: the sweep script sees 8-hex prefixes and
  all four digests and excludes a reviewed mention-only list, printing the old rule as its "historical
  count" (22 at `702b2397`; corrected 20 there, 19 from this parcel on, because the enum README now names
  its oracle by digest). Row 13 of the enum probe README is corrected with two more probes. The
  asl-reference README's two `a8cd8b80` rows now read `(i386-unknown-linux)`, applied after the
  overseer's review re-ran all seven table paths. **The "22" quoted in the row above and in the
  2026-09-16 and 2026-09-26 lane-log entries is the historical rule's figure and stays as recorded**;
  the script's output is the authority for the current count.
- state at landing: `open`  size: `S`  project: `-`
- Three findings from `docs/superpowers/notes/2026-09-26-asl-banner-citation-dependence.md`, none acted on:
  (1) `scripts/sweep_asl_citation_form.sh` misses digests written as 8-character md5 prefixes and counts notes that
  only MENTION the banner, so its banner-only population over-counts (`2026-09-05-asl-silent-decline-regime.md` is in
  it only for that reason); (2) `asl-reference/README.md` gives `a8cd8b80`'s second banner line as x86_64, the binary
  prints `(i386-unknown-linux)`; (3) the enum probe README's row 13 "value folds to 0" is not a rule (one byte above
  the line gives `0101` on all four builds, from a run that exited with errors). Also: banner line 2 separates three
  families, so only `0dee1f98` and `aa6de52f` are banner-indistinguishable.

## S1-BUILD-PROFILE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Why we take about 1.3 seconds on Sonic 1 where the old assembler takes half. One leg answered: there is essentially no fixed startup cost, so what we spend is proportional to input.

## AS-LEADING-DIGIT-IDENTIFIER

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Whether a name may begin with a digit. The one-byte-short defect is closed; what remains splits into a compatibility call (mine) and a language call (yours).

## EMP-USP-CCR-SURFACE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Two of the new instructions are deliberately NOT exposed in our own language - they need a new operand kind, which is language surface and your call rather than mine. Nothing depends on it today.

## EMP-ALIGN-SHARED-RULE: LANDED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Our own language's align still rounds up plainly. That is correct for every ROM section, which is all the engine currently has, so nothing is wrong today - but the moment anything aligns on the RAM side it needs the signed rule the assemble

**LANDED at `3ed600d4` / merge `4f7fc54f`, established by the 2026-09-08 staleness sweep rather than assumed from the merge title: both `.emp` lowering sites call `sigil_ir::asl_align_pad` (`lower/mod.rs:1034`, `regions.rs:565`), and the rule is pinned by `align_as_parity.rs:138`, `a_negative_vma_align_follows_the_shared_signed_rule`. The row's claim that our align "still rounds up plainly" and needs the signed rule the day anything sits on the RAM side is answered on both halves.**

## CLOBBER-UNEXERCISED

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: the payoff measurement, and then the engine lane: the declarations are theirs, not ours

RULED d-26 on 2026-09-06 BY THE HUB IN THE OWNER'S PLACE, overturnable by him, NOT his word. Measure the payoff first as a size S row, AFTER the driver-size parcel, never beside it; the L fix does not start on the unnumbered claim. If the number justifies it, a witness derived from EMITTED code is mandatory before any declaration is removed, because a witness taken from the declaration being corrected shares the frame it exists to check. Hand-editing the frozen baseline is REJECTED. Scope rider verified here rather than relayed: 89 aeon .emp files carry clobbers against 3 in sigil, so the correction is an AEON-side landing serialized behind the byte chain and routed to them, not landed from sigil.

## SIGIL-DECOUPLE

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: nothing

Item 14. Both owed reads done (§6 a and b). Verdict unchanged - NOT YET - but its argument is rewritten: the census was never stale, and the live risk is a decaying margin routed to the engine lane.

## AGENT-WORKTREE-TARGET-DIRS: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

SUPERSEDED BY THE 2026-09-05 SWEEP and restated to what is left. 49 GB reclaimed, 31 worktrees to 13, 246 branches to 17, every removed tree verified 0 tracked-dirty, 0 untracked and tip in master, locks cleared with unlock and never a second -f. WHAT REMAINS: 12 trees hold something, mostly a single untracked build artifact each, and 7 branches are unmerged and kept by name. The residue needs a per-tree judgement rather than a rule, so it is not a sweep.

**CLOSED 2026-09-06 at the root level by the cleanup the owner asked for: 171.9 GB reclaimed, 227 hidden directories to 122, manifest at docs/2026-09-06-workspace-cleanup.md. What remains is 110 aeon-owned directories (routed to that lane) and 6 registered worktrees of mine, which need a per-tree judgement rather than a rule.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## RESOLVER-FALLS-THROUGH-TO-SHARED-TREE: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

**CLOSED 2026-09-08 on `parcel/resolver-fallthrough`.** The read moved behind the door: `test_support::oracle_legacy_dir` resolves `oracle-old` by the SAME precedence the engine tree uses (`ORACLE_DIR`, then `EMPYREAN_SUITE_ROOT/oracle-old`, then a step-3 derivation that `names_a_reference_tree` refuses), and `m1b_gate` now calls it instead of reading the variable itself. One algorithm over a `CheckoutSpec`, not a second resolver: the second tree is a value.

**THE UNFORCED CASE REFUSES, WITH THE DIRECTORY PRESENT, AND IT IS SHOWN.** `/home/volence/sonic_hacks/oracle-old` and its `linux-port/gui/Symbols.cpp` both exist; with every variable unset, `cargo test -p sigil-harness --test m1b_gate -- --exact oracle_loadfromaslisting_resolves_emit_listing` FAILS with *"NO REFERENCE TREE IS NAMED … This run DECLINED to use /home/volence/sonic_hacks/oracle-old, which step 3 derived"*. `ORACLE_DIR=…/oracle-old` on the same command announces *"reference-tree: /home/volence/sonic_hacks/oracle-old (SUITE_PATHS step 1, named by ORACLE_DIR)"* and passes; `SIGIL_ALLOW_PARTIAL=1` skips it against the stand-in path. Three directions, and the third is not a formality: a resolver that refused unconditionally satisfies the first two.

**THE POPULATION, not the one site.** `reference_env_read_is_routed` ruled exactly `AEON_DIR` and `EMPYREAN_SUITE_ROOT`, and named the gap in its own module doc, *"A THIRD tree-naming variable … until then a read of it would pass unseen."* That gap was live: `ORACLE_DIR` was the third variable, it was being read privately, and the gate was green over it for as long as the read existed. The rule now iterates a `RULED_VARIABLES` list that includes it, so a test reading `ORACLE_DIR` for itself is a failing row. The gap for a FOURTH variable is restated rather than deleted, because it is still real.

**The operator names it too.** `scripts/landing-run.sh` resolves `oracle-old` through the shell include, exports `ORACLE_DIR` to the child, and stamps the path and the step into the log header; `suite_paths.sh` and its Python twin gained the matching marker row (`linux-port/gui/Symbols.cpp` + `Devices`) so a value naming the Rust `oracle/` is a set-but-wrong hard error at its own step rather than a tree with no `linux-port/` in it. Without this the landing run would have stopped inside the suite with the resolver's message instead of at its own front door.

**A LIMIT THE FIX CARRIES, named rather than left to be found.** When `landing-run.sh` resolves at STEP 3 it exports a DERIVED tree under the name of an explicitly-chosen one, and from inside the suite those two are indistinguishable: the resolver sees a variable and cannot see who set it. The `AEON_DIR` line beside it has always made the same trade; the mitigation for both is that the log header stamps the STEP and not only the path, so a reader asking which value was in effect can answer it from the record. The suite cannot answer it on its own, and this parcel does not change that.

**WHAT DID NOT CLOSE.** The gate's CURRENCY question is untouched and this parcel does not touch it: `emit_listing` is still pinned against the legacy C++ reader, and the shipping debugger's Rust symbol loader is still not a subject of any gate here. That is the standing gap-ledger row (`campaign-gap-ledger.md`, const-arity lane 2026-08-22), whose own kill condition says *"it runs again" is NOT a kill*; naming the tree does not make the tree current either.

`crates/sigil-harness/tests/m1b_gate.rs:51` resolves `ORACLE_DIR`, then falls through to the fixed path `/home/volence/sonic_hacks/oracle-old` — another lane's checkout. Found 2026-09-07 by turning aurora's fifth Roster C rule (no resolver in a seat's rig may fall through to a shared last resort) on this lane's own rig; the rule is in the lens UX amendment at empyrean `3ad431f` per the hub's relay, NOT read here.

**The existing mitigation covers the wrong failure.** The gate skips when the sibling repo is ABSENT and fails hard on absent under `SIGIL_STRICT_GATE=1`. On this machine the directory is PRESENT, so the unforced case does not refuse: it measures whatever that tree contains and attributes the result to nothing. Fix shape: the value is set explicitly by whoever runs the gate, and the unforced case is PROVEN to refuse once, rather than assumed.

**Checked and clear in the same sweep, named so the zero has an instrument behind it:** `test_support::aeon_dir` (step 3, the derived sibling, excluded from `names_a_reference_tree` by name at `test_support.rs:795`; a run naming no tree refuses or skips by declared intent), `scripts/landing-run.sh` (refuses at the end of its chain), `P2BIN_BIN` in the three vector generators (derives from an already-explicit `ASL_BIN` that exits 2 when unset, then asserts the result is a file). **`refreeze.rs:785`'s `AEON_DIR` default is now audited and is NOT this class.** It is `unwrap_or_default()`, an empty-string default and not a path, and it is unreachable with an empty value: `resolve_aeon_rev()` five lines above it already refuses the whole `--attest` run when `AEON_DIR` is unset or names no directory, and the local's only consumer is the log header at `:866`. Nothing was changed there.

**Its neighbouring PROSE was stale, though, and that is the same defect the `OVERSEER-REFERENCE.md` note below records.** The doc comment at `refreeze.rs:105` described `capture_goldens.sh` as carrying a `${AEON_DIR:-/home/volence/…/aeon}` fallback that "silently builds against the owner's LIVE working tree". That script refutes it: it resolves through `suite_resolve_checkout` and REFUSES in as many words (*"This does NOT fall back to a live"*). A comment that narrates a fall-through the code has since removed reads as a live hazard to anyone auditing for exactly this class. Rewritten to the present-tense contract fact in the same change.

Related and already fixed: `docs/OVERSEER-REFERENCE.md` claimed `aeon_dir` still defaulted to the owner's live checkout, which the code refutes; corrected at `2802d189`. The practice that paragraph teaches survives, its argument did not.

**How the closing sweep enumerated, so the zeros have instruments behind them.** `git grep -n "/home/"` over tracked `*.rs *.sh *.toml *.py` (129 hits) and `git grep -n "sonic_hacks"`, plus `git grep -nE 'env::var(_os)?\([^)]*\)[^;]*unwrap_or'` for the Rust shape and `git grep -nE '\$\{[A-Z_][A-Z0-9_]*:?-[^}]+\}'` for the shell one, and `/usr/bin/grep -rn -A3 "env::var"` for a fallback `rustfmt` had wrapped onto a following line. POSITIVE CONTROL: every one of those instruments returned `m1b_gate.rs:51`, the known instance, so no zero below rests on a query that silently matched nothing. **In shipped `crates/**` test and harness code the set was exactly one: `m1b_gate.rs:51`.** Outside it, and deliberately not converted, are the throwaway probe scripts under `.f1probe/`, `.s1probe/` and `docs/superpowers/notes/*-probes/`, roughly a dozen `SIGIL=${SIGIL:-/home/…}` lines naming per-parcel target directories rather than a peer's checkout, in scripts that are evidence of a finished investigation and are not run by any gate.

## LENS-UX-SEAT-DIAGNOSTICS

- state at archive: `open`  size: `M`  project: `-`
- blockedBy: oracle's pilot notes on the brief, relayed by the hub; and the in-flight lens fixes and the owed shared-pair refresh, both ahead of it

OWNER-RULED 2026-09-07, verified firsthand at empyrean `97cd725` (an ancestor of their `origin/main`; text at `docs/2026-09-07-lens-ux-seat-amendment.md`, protocol lands in aeon's `LENS_PROTOCOL.md`). His words in the artifact, not the relay: asked whether the lens should have a UX seat, *"I think it should have one right?"*, and on scope *"I think we draft it and run on oracle, aurora, and sigil for now"*. So the owner decision exists AND it is this question; the SEQUENCING (oracle first as pilot) is the hub's on his words and the artifact says so itself.

The sigil variant is pointed at DIAGNOSTICS. **UXa, task walk:** the seat writes wrong code on purpose, three to five classes named in the controller's charter (a typo'd mnemonic, a width mismatch, an unresolved label, a macro misuse, a section overflow), as a newcomer who has read the README and nothing else, and judges whether each message gets them out — logging every stall, guess, and moment it had to read our source to proceed, ranked by time burned. **UXb, heuristic audit:** walk the message catalog against the checklist — can the cause be found from the text, does it say what to do next, is it consistent with its neighbours.

Three conditions that are not optional: every finding ships the diagnostic text VERBATIM (no evidence, no finding — the amendment's own guard against a verdict of "nothing found", which is indistinguishable from clean); the panel is LATE, at a new pin, and the packet is amended to name it; and the no-emulator-MCP line stands unchanged.

**⚠ THE BRIEF HAS MOVED AND THIS ROW WAS WRITTEN FROM THE OLDER TEXT.** Everything above was read at empyrean `97cd725`. The amendment is now at empyrean `6a12740` (same file, +25 lines, verified here an ancestor of their `origin/main`), carrying four rules added from oracle's pre-run reading. **This row does NOT restate them, because this lane has not read them** — bumping a citation to a revision you have not read is the verified-at defect wearing a freshness costume. Two were relayed by the hub and are recorded as RELAYED, not verified: that failing to get a newcomer from launch to a first successful assemble is a FINDING and never BLOCKED, and that the charter names the surface each job is walked in. **THE AUTHORITATIVE TEXT IS THE PROTOCOL, NOT THE AMENDMENT, AND THIS ROW DELIBERATELY DOES NOT PIN ITS REVISION.** Roster C lives in aeon's `docs/superpowers/LENS_PROTOCOL.md`. **Resolve `aeon origin/master` at RUN TIME and read it there**; write the charter from the protocol, never from the empyrean amendment (the draft that fed it) and never from this row, since citing a draft over the landed text is how a superseded rule gets enforced.

**Why no SHA: this row's protocol citation went stale TWICE IN EIGHT MINUTES.** Measured here — that file took two commits tonight, `8def2240` at 19:05 (Roster C lands) and `82540fab` at 19:13 (rule five changes from *proves the unforced case refuses* to *proves WHICH VALUE WAS IN EFFECT*). Both were relayed to this lane as the one to cite, and the second REVERSED the obligation the first carried. A pin into a document being rewritten on that cadence is coordinate-rot at document scale (protocol bar 18's corollary: subscribe to the event you depend on, re-read at point of use), and the failure is not that a pin goes stale but that it goes stale while still resolving, so a session writes a charter to a superseded obligation with a verified SHA beside it.

**Known-good floor, for content presence only and NOT as the revision to cite:** at `82540fab` (verified here an ancestor of their `origin/master`) the file carries Roster C, the fall-through rule, and the artifacts'-location proof form with *"absent from the shared default"* verbatim. If the text you resolve at run time lacks any of those, you have gone BACKWARDS and should ask, not proceed. Grep note from the hub: the landed text capitalises WHICH VALUE WAS IN EFFECT.

**THE FALL-THROUGH RULE HAS A DIRECTION AND BOTH ARROWS ARE LIVE HERE** (oracle's refinement, relayed by the hub; their framing, this lane's instances). Ask what is SHARED and which way this tool moves against it, rather than pattern-matching a resolver.

*Reaching the shared thing* — the seat resolves onto something it does not own: booked as `RESOLVER-FALLS-THROUGH-TO-SHARED-TREE`.

*Becoming the shared thing* — the seat's own run turns into what a LATER run resolves to. **This lane already manages three instances and had never seen them as one class**, which is the framing's real yield here; each was learned from its own incident and none of them names the other two:
1. a build in the main checkout relinks `target/release/sigil`, the assembler aeon's `build.sh` resolves to (it cost a peer four legs assembled by two different binaries on 2026-09-06); managed by the hub-window rule;
2. `~/sonic_hacks/.sigil-pin-af35fa56` (was `.sigil-ls12-pin`, retired 2026-09-07) became a path aeon's `build.sh` currency-checks, so sweeping it turns their provenance to `unknown`; managed by a declaration on both boards;
3. `sigil build --aeon <tree>` WRITES into the tree it is handed; managed by the exclusive-tree rule in every brief.

**The one that is NOT yet managed and is the charter's own risk:** `scripts/nightly_ref_drift.sh` and `scripts/nightly_source_gates.sh` write under `${XDG_STATE_HOME:-$HOME/.local/state}/sigil-{ref-drift,source-gates}` — the reaching arrow in the default, and the becoming arrow in the ledger, which a later run appends to and reads. Audited 2026-09-07: no pollution TODAY (the `--selftest-fail` path exits before writing, and `record_unmeasured`'s `unknown` rows are a deliberate record of a quiet night, not an accident). But **a record names its revisions and its time and NOT its writer**, so a seat that ran either script would append observations indistinguishable from the nightly lane's. **The charter gives the seat its own state dir explicitly and proves WHICH VALUE WAS IN EFFECT, read back from the run's own output** (the hub's generalised form of rule five's proof obligation: terminate the chain explicitly and prove the value, a demonstrated refusal being one such proof and not the only one). That generalisation matters here specifically: the older refusal-only form would have had this lane ADD a refusal to a production nightly script purely so a seat could prove something, which is changing the subject to suit the instrument.

**But the readback is NOT where the relay said it was, and a charter written on the relay would look for a line that is not there.** Checked at `9af282ae`: both scripts mention `$STATE` only inside FAILURE notes (*"see $STATE/build.log"*). A clean run prints no such line. The proof is therefore the artifacts' LOCATION, not a printed path: the run WRITES `nightly.log`, `build.log`, `provision.log`, `gates.log` and the ledger under the resolved dir, so the seat asserts those exist under its own dir and are absent from the shared default. Same obligation, available on the green path, and it still needs nothing added to the script.

**DO NOT START until the hub relays oracle's pilot notes.** The brief is the thing under test on the first run, which is the whole reason oracle goes first — starting early would spend this lane's run on a brief the pilot is about to correct.

## S4BUDGET-STALE-ASSUMPTION: CLOSED

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

✅ GROUNDED 2026-09-07 and ROUTED to aeon; retire at the next boundary. The 2026-09-06 non-match was a search of the WRONG TREE: the row is a finding about aeon's tool, so no artifact for it could exist here. Grounded at aeon tools/s4budget.py (the premise appears twice, in load_vram_layout's docstring and in the user-facing VRAM message) against this tree's built listings, which carry 780 EQU rows of which 32 are VRAM_*, in the form `EQU VRAM_PLANE_A = $0000C000`. The row's figure of 17 is not restated; the measurement is 32 by enumeration. Their CONCLUSION (read vram.toml) survives on the union/overlay_with argument; only the premise is stale. Superseded text follows. ⚠ UNGROUNDED, DO NOT ROUTE UNTIL RESOLVED. The row reads 'their budget tool reports a value UNMEASURED while the data it needs sits in the listing it just read', and on 2026-09-06 that wording matched NO artifact in this tree. The instrument was working (s4budget appears in 14 files), so it is a genuine non-match: the row has either lost its source or never had one. Deliberately NOT sent to aeon as bookable. What IS grounded nearby, and is aeon's independently: row F4 in docs/superpowers/notes/2026-09-05-decouple-aeon-side-inventory.md, EndOfRom equals the ROM file size, structurally un-failable because the disagreement prints as a NOTE and never reaches breaches, so padding, a stale file and a real placement error read identically. Next action is to ground this row or retire it, not to work it.

**CLOSED. Grounded 2026-09-07 and routed to the engine lane, and they landed the fix at aeon `c834cb66`, whose docstring now quotes this lane's own 780 EQU / 32 VRAM figures. The row's own body already carried its closing reason while its heading read `open`, which is the same defect as the four rehomed above wearing a different shape.**

## CLOBBER-PAYOFF-MEASURE

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; **the blocker LANDED at `ee136317`, verified an ancestor of master by the
  2026-09-08 sweep, so the d-26 ordering is satisfied and this row is runnable now**

The S row d-26 ordered before the L fix may start. Measure what the 76 over-declared clobber lists actually cost across the shipped ROM in bytes and cycles. MEASURE AT THE CONSUMING END: the cost is wasted saves and restores at CALL SITES, so 76 of 387 procs is a producer count and says nothing about how often they are called. A small answer retires the item honestly and that is a real result, not a failure.

## EMP-ORG-TWIN-CHECK: CLOSED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

ANSWERED by the org parcel and kept only so it is not re-opened: the .emp frontend CANNOT have this defect, because it has no org at all. lower/mod.rs always opens with switch_section_lma, so LMA is a monotone chain that can never take a lower target, and vma: is a VMA pin only. That is a DESIGN DIFFERENCE, not twin agreement, which is the distinction that matters: twin agreement would have proved nothing. No parcel site exists. Close this row at the next boundary.

**CLOSED 2026-09-06. Answered by the org parcel: the .emp frontend cannot have the defect, no parcel site exists. Its own text said to close it at the next boundary.**

**Its closing reason was written into PROSE-STATED-BOUNDS's body by the commit that closed four rows at once, so this row read `open` for three days while its own closure sat under an unrelated heading. Moved here 2026-09-09.**

## PROSE-STATED-BOUNDS

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing

Raised by the aeon lane 2026-09-06 from their own instance: inject_editor_bg's refusal told authors 'the limit is 12 KiB' while enforcing 20,480, stale two days after the owner raised the constant. A BOUND STATED IN PROSE inside a diagnostic is a third population, reached by neither an identifier grep nor a quoted-key grep, and it is read at the exact moment an author hits the limit. NOT YET MEASURED HERE: sigil's diagnostics are this lane's own surface and nobody has asked whether any of them state a numeric bound in words rather than deriving it from the constant. The sweep is the measurement, and a zero needs an instrument that could have returned non-empty. Their fix is the shape to copy: derive the figure from the constant and prove it by poisoning the constant and watching every printed figure follow.





## LENS-PASS (closed 2026-09-06, dropped from the board)

- state at archive: `open`  size: `L`  project: `-`
- blockedBy: nothing

DONE and landed (packet 2570f724, 22 of 22 seats); findings booked same day in docs/superpowers/notes/campaign-gap-ledger.md. What remains is the FIX ORDER, which is yours: 4 byte-changing, 4 silent-acceptance or abort, 5 gates that cannot fail, 3 structural, 3 measure-first. Nothing fixed during the sweep by design, and nothing is authorized by being booked.

## PINS-DEAD-TESTS-FIELD-AND-ORPHANS: LANDED, and it was stale in the DISPATCH SLOT within thirty minutes

- state at archive: `next`  size: `S`  project: `-`
- blockedBy: nothing; the engine lane is TOLD before it lands, not asked

The two low lens findings in the pin manifest, `sig-pins-tests-field` and `sig-orphan-pins`, taken
as one parcel because they are one file's two halves. Grounded 2026-09-08 before briefing, and the
grounding changed what the parcel is:

**Half one is already solved and only the cleanup is left.** The manifest's own header says
`tests` IS NO LONGER READ BY ANYTHING: the rerun hint is DERIVED at print time from which
`crates/*/tests/*.rs` actually reference each constant, proved on a real pin move where doctoring
`MAP_TEST_OBJ` emitted all five consuming binaries against a declared list naming one. What remains
is deleting the 412 dead `tests` lines, which the header itself books as belonging on its own commit
and calls the correct end state, for the stated reason that a field nobody reads and nobody
maintains will be read by a future author as a record of something. The generator stamps that field
into the generated file's doc lines, so `pins.rs` is regenerated in the same change and its diff
must be doc lines only, with no pin VALUE moving.

**Half two needed a mechanism check before it could be briefed, and the check inverted the obvious
fix.** An orphan pin is NOT inert. `repin_pins::pins_rs_is_current` regenerates the whole file from
the manifest against the tree and compares the entire text, so every pin, consumed or not,
participates: a symbol that moves in the engine tree makes the generated text differ, the gate goes
red, and someone regenerates. So a pin no test reads still costs a red on unrelated movement, and
still shows what moved. Deleting the 22 would have looked like tidying and would have removed drift
coverage of 22 symbols. **A brief saying "delete the orphans" would have been acted on.**

**RULED HERE, and it follows this repo's own precedent rather than taste:** the zero-consumer set is
REPORTED by the repin tool on every run, enumerated by name, and GATED BY NOTHING. Same shape as
`PROVENANCE-REV-REACHABILITY`, and for the same reason: a declared exception list is a population to
maintain whose failure mode is green because nobody maintained it. Reporting makes the set visible
and count-free, which is what the finding actually asked for. Nothing is deleted on this parcel.

**Both halves touch `crates/sigil-harness/repin.toml` and `crates/sigil-harness/src/pins.rs`**, the
two files the boot read puts on the coordinate list. Under the 2026-09-02 cut the engine lane
freezes alone, so this is a notification rather than a permission: this seat undertook to the engine
lane on 2026-09-08 that they hear before such a parcel lands, not after, and that undertaking is
what governs.

**LANDED at `53bc85de`, merge `d39b7ce9`. The manifest carries zero `tests` lines, re-adding one is a parse error, and `zero_consumer_report` ships.**

**⚠ THIS ROW IS THE SHARPEST STALENESS INSTANCE THIS LANE HAS.** It was written at `fa9d1e1d`, 19:29 local, in state `next`, which is the dispatch slot. The parcel it describes landed at `53bc85de`, 19:59 local. **Thirty minutes, same evening, same repo, both by this seat.** Staleness here was being modelled as multi-day drift and it is two orders of magnitude faster than that: a row can be stale before the session that wrote it has finished its turn. Found by the 2026-09-08 sweep, not by the seat that wrote and then invalidated it.

## INDIRECT-COST-REPORT-UNCOVERED: covered by `parcel/cost-report-shape-test`, both halves

- state at archive: `open` until that branch merges; nothing further is owed by it  size: `S`  project: `-`
- blockedBy: nothing

Booked at the landing of `parcel/quoted-cost-deferrals` (merge 68351ffc) by the controller, against
that parcel's own delivery, because an unstated gap reads as coverage. The gap was real and was
re-measured before it was worked: at `bfd34618` the only occurrence of the flag or either new field
outside their own source was a comment in `crates/sigil-frontend-emp/tests/contract_closure.rs`
citing the command, and `crates/sigil-cli` had no test naming it at all.

**WHAT CLOSED.** `the_indirect_cost_report_is_wired_and_internally_consistent`, in
`crates/sigil-cli/tests/contract_closure_corpus.rs` beside the `--report contracts` gate it mirrors,
drives the real binary over all three shipped shapes and reads its real stdout. It asserts the report
does not contradict itself and asserts no number: each section's declared count against the rows it
renders, the headline cost against the two policy counts printed above it, the row grammars, and that
the walk ran under the target's defines rather than define-free. Proven red-first with three
mutations of the SUBJECT, each shown applied on disk before its run: swapping the cost operands (red
on the cost relation, 0 against 59), a site header that declares 11 and renders 0 (red on the site
relation), and a firing list one row short of its own count (red on the firing relation). Runner:
`cargo test --release -p sigil-cli --test contract_closure_corpus`, which CI's `cargo test
--workspace` already runs.

The script half closed too, since the row named it and it was cheap in the same parcel:
`crates/sigil-harness/tests/s8_seam_size.rs`, beside the existing script tests in that directory.
Two cases. The arithmetic case asserts the report's totals against the rows they total, the headline
against the seam totals, and THE PARTITION, that the move plus the unassigned remainder is the whole
crate; that last relation is the one the silent-undercount class breaks, and it was proven red-first
by dropping a seam from the headline (16644 against 18257, the same failure re-demonstrated) and
again by claiming a module in neither list (741 lines accounted for nowhere). The refusal case builds
a throwaway fixture tree and runs it twice, whole (the control, which must exit 0) and with one
seam-named module removed (which must exit 2 and name it); without the control the exit 2 would be
evidence only that the fixture was broken.

**WHAT DID NOT CLOSE, and none of it is owed by this parcel.**

1. Nothing here says the reported figure is RIGHT. Both gates assert self-consistency, so a closure
   that computed the wrong firing sets would still render a report that balances. That is the
   deliberate limit of a shape gate and the price of a figure that is free to move.
2. The cost relation is partly vacuous while the trusting count is 0: `cost == forced` and
   `cost == forced - trusting` are the same claim today, so a defect on the trusting operand would
   pass. It discriminates on the forced operand now and on both the day the warn tier is non-empty.
   Stated in the test rather than left to be found.
3. The relation is `forced.saturating_sub(trusting)` and NOT `forced >= trusting`, which was
   considered and rejected as a check that fires on correct code: `check_firings` collapses a proc
   whose effective set is TOP into a SINGLE unbounded firing and returns, where the trusting reading
   of the same proc can fire once per register, and `@allow("clobbers.unanalyzable")` suppresses the
   unbounded case outright. A legitimately smaller forced count is reachable, and with the warn tier
   empty nothing would have noticed the assertion was wrong until it was.
4. The cross-check the report prints (grep the source for `as Type` sites the walk cannot see) is
   still a manual step. Nothing compares the walk's site list against the source text, so a site
   inside an unresolved splice template remains invisible to every check, exactly as the report's
   own note says.
5. The s8 refusal is proven on a fixture, not on the real tree, because proving it there means
   removing a harness module.

## CLOSURE-MISSES-FALLS-INTO-EDGE: LANDED 2026-09-09, and one of its two halves had no instance

- state at archive: `LANDED`  size: `S`  project: `-`
- blockedBy: nothing; it was OUR code

**The edge is modelled.** `ProcNode::falls_into` carries the declaration and the closure fixpoint
charges the successor's whole effect to the falling proc, exactly as it charges a tail transfer,
with the same hole treatment for an unresolvable successor. `seam1`'s Z80 node builder had modelled
the same edge since it was written (by pushing the successor into `direct_callees`) and now uses the
field, so one edge has one spelling. Per-row evidence, the adjudication of every baseline movement,
and the red-first proofs: `docs/superpowers/notes/2026-09-09-falls-into-edge.md`.

**The asymmetry it removes, in the corpus's own words.** `Player_SensorFloor` ends
`jbra Player_SensorSurface`; `Player_SensorCeiling` declares `falls_into Player_SensorSurface`. One
transfer spelled two ways, and the closure read the first as clobbering the shared body's ten
registers and the second as clobbering `d6/d7`.

**WHAT CLOSED.** The census half, in full: over-declaring procs drop 90 to 66 on sonic4 plain (71 to
47 debug, 101 to 77 demo plain, 82 to 58 demo debug, 71 to 47 config_a, 91 to 67 config_b, 90 to 66
lean) and the `falls_into` artifact class goes 16 to 0 on every shape. The row's figure of 11 is the
same population under `clobber_payoff`'s bucket ORDER, which counts an empty-effective proc in the
comptime-empty bucket first. The drop is 24 rather than 16 because callers of a falling proc gain its
successor's registers too.

**WHAT DID NOT CLOSE, and it is the half this row called urgent.**
`[proc.clobber-undeclared]` closure firings are **0 before and 0 after, on all seven shapes**. No
proc in the corpus falls into a successor writing more than the head declares, so the suppressed
under-declaration firing this row was booked for **has no instance in today's corpus**. The
mechanism is real and a test now holds it, so the firing cannot be suppressed the day someone writes
that pair; but the row's claim that the destructive direction was the reason to run now does not
survive the measurement, and anyone re-reading the CLOBBER-PAYOFF note's closing paragraph should
read this sentence beside it.

**WHAT MOVED INSTEAD.** `[call.live-clobbered]` (D1c) moved identically on all seven shapes, one row
relocating and six appearing, all of them the player sensors returning their result in an
undeclared `d0`/`d1`. Three rows of that class were already frozen. Each new row is adjudicated at
its own row in `crates/sigil-harness/src/contract_baseline.rs`; the GONE row
(`Air_Collide @ Air_WallProbeRight :: d1`) did not vanish but moved to the later call on the one
path that reaches the read, and `Air_Collide :: d1` is one row before and one row after.
`[proc.dead-save]` is unchanged by count AND by row on every shape, which is the direction the
baseline's ratchet warns about.

**The follow-on this leaves, not booked by this parcel:** the sensors declare no `out(...)` for
results their own headers document (`Out: d0.w dist, d1.b angle, d2.b attr`). Declaring that
surface in aeon would dissolve nine D1c rows at once, and it is an aeon contract change, not a
sigil one.

Original row text, unedited, because a reader who carried it away needs to meet the correction
rather than find the row silently rewritten:

> Booked 2026-09-09 from the CLOBBER-PAYOFF-MEASURE result, and it is the reason that measurement was
> ordered before the fix rather than beside it.
>
> `corpus_contracts.rs` builds callee edges from call and tail mnemonics only, so **a `falls_into`
> declaration is not modelled as an edge at all.** Consequence, measured: 11 of the 90 procs the census
> reports as over-declaring are not loose, they are correct. `S4LZ_DecompressDict` reads as never
> writing 8 of its 9 declared registers, while `S4LZ_Decompress`, the proc it falls into, writes them.
>
> **⚠ THE DIRECTION THAT MATTERS: had the large fix started on the producer count, a register was on the
> list to be deleted from a contract that needs it, along with its caller's correct save.** The
> over-declaration census is the SAFE direction of this gap. The same missing edge suppresses a real
> under-declaration firing, which is the destructive one, and that half wants its own row rather than
> riding this one.
>
> Fix shape: teach the closure the `falls_into` edge. It removes 11 false census rows and closes the
> missed-firing direction in one change, in our tree.

## AS-Z80-FUNCTION-CALL-IN-OPERAND: LANDED 2026-09-09, and it was TWO defects, not one

- state at archive: `LANDED`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing; nothing is left to do

**Shipped on `parcel/z80-function-operand` at `4cc75c75`**, *"as: a user `function` call in a Z80
operand, and the paren the expansion adds"*. sigil now reports `1BC6h` on s1disasm `f6ece657`, the
figure asl (md5 `61e672562465725a8c102288a7da9098`) reports from the same tree at exit 0.

The row's diagnosis was right about the cause and wrong about the count. Fixing what it describes
gets to `1BC5h`, not `1BC6h`, and raises a diagnostic the row does not predict.

1. `lower_z80` handed its operands straight to `parse_operands`, running none of the
   `expand_operand_builtins` layer that `dc.b`/`dc.w`/`dc.l` and the 68000 path both run. There is no
   call syntax in `parse_expr`, so `zmake68kPtr(SegaPCM)` parsed as the bare symbol `zmake68kPtr`
   with `(SegaPCM)` left over. This is what the row describes, and routing through the existing layer
   closes it.
2. That alone lands on `1BC5h` and a NEW refusal at line 197, `unsupported form: Ld, [Reg(B),
   Mem(11)]`. `expand_calls` wraps every expansion in parens for precedence, and on the Z80 an
   operand that is one whole paren group is an INDIRECTION. So the expansion rewrote
   `ld de,zmake68kPtr(SegaPCM)` (3 bytes) into `ld de,(nn)` (4 bytes) and `ld b,pcmLoopCounter(16000)`
   into `ld b,(nn)`, which is not a Z80 instruction: +4 and +0 where the reference has +3 and +2. The
   68000 never saw this because an immediate there carries a `#`, which `classify` settles before it
   looks at parens. The fix lets the WRITTEN shape decide the addressing mode: a group the programmer
   did not parenthesise stays a value however many parens the expansion added.

The row's "substituting literals for exactly those four operands yields `1BC6h`" was a sound
measurement of the GAP and not of the fix: a literal carries no parens, so it never exercised defect
2. The defect the substitution could not see is the one that cost the ninth byte.

Ripple over all four corpus roots, both directions, nothing newly present anywhere: s1 `sonic.asm`
50 -> 46 (4 absent), s2 `s2.asm` 5229 -> 5227 (2), skdisasm `sonic3k.asm` 2424 -> 2417 (7),
`s3.asm` 1433 -> 1429 (4). The two Sonic 3 roots need their generated PCM includes present first;
without them the newly-reached line 4404 reports `unresolved symbol SEGA_PCM.sample_rate`, which is
a corpus-preparation artefact downstream of an include failure both runs already report, not a
regression. Pinned by `crates/sigil-frontend-as/tests/as_z80_function_operand.rs`, expectations read
out of an asl listing committed beside it, red under three separate mutations of the subject
(no expansion; expansion without the written-shape rule; the written-shape rule over-reaching).

## DEAD-SAVE-DOC-AND-FLOOR-STALE

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing

`dead_save_corpus.rs`'s doc comment says one firing in every shape. Measured 2026-09-09: **1 in the
non-debug shapes and 10 in the three debug ones.** Its floor is `widest >= 1`, which is tolerant in the
direction that matters: it could never notice growth. Same class as the stale-comment rows this file
already carries, with the addition that the gate's own tolerance hides the drift its comment
misdescribes.

## WORDING-LOCKS-LATENT-TWO: OPEN

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing
- moved out of `docs/lane-status.json` 2026-09-12, under rule 7's 20-row bound, to make headroom

Checked: only ONE test pins a sentence for a feature that is not built yet (field assignment,
`eval_control_flow.rs`); the second was a permanent error, not a wording lock. Low value, and the
reason is worth keeping rather than just the verdict: whoever eventually builds that feature has to
rewrite the test anyway, so the lock costs nothing until the day it is removed as a side effect of
the work it would have obstructed. Moved here rather than closed because "low value" is a judgement
that a later seat may reverse, and a closed row does not get re-read.

## PARTIAL-ROWS-PRINT-PLAIN-OK: OPEN

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing
- moved out of `docs/lane-status.json` 2026-09-12, under rule 7's 20-row bound, to make headroom

Narrower than it was first booked. The partial-run marker IS printed and the nightly check reads it,
so the automated backstop is intact. What remains is that a HAND-RUN suite hides it, because cargo
swallows a passing test's stdout unless asked for it. So the exposure is an operator reading a green
scoped run and not seeing which binaries went unmeasured, which is the same hand-run-has-no-second-reader
shape as `AB-PROTOCOL-CART-UNVERIFIED`. Kill: have the partial banner print to stderr, or have the
runner script surface the unmeasured count in its own summary line.

## ASL-WARN-PARITY-ALIGN: PREMISE UNVERIFIED, do not dispatch as written

- state at archive: `open`  size: `S`  project: `-`
- blockedBy: nothing, but see below

Board text, reproduced verbatim so it is greppable: *"The old assembler warns about a misaligned
address and we say nothing, on every leg of one option. Bytes agree, so it is a missing warning
rather than a bad ROM. We already have this check on the other language."*

**Nothing in the tree records the measurement behind this.** Not the option whose legs were walked,
not which language already carries the check, not the comparison that showed `asl` warning where we
are silent. Measured 2026-09-16 with a positive control. **It was the `next` row**, so a session
taking the board at its word would have briefed an agent to add a warning with no artifact showing
one is missing. Re-derive against `asl` before working it, under this file's own opening rule that a
stale open row does not read as stale, it reads as work.

**Correction, 2026-09-17: the premise WAS in the tree; it was found by content, not by this row's id.**
"Nothing in the tree records the measurement behind this" is wrong. The measurement is
`docs/superpowers/notes/2026-09-16-switch-matrix-sweep.md`, section "Diagnostic parity: sigil is silent
where asl warns, once", commit `6348b831`: asl's `warning #180: address is not properly aligned` on
`move.w (1).w,d0` at `s2.asm:30438` over every `gameRevision=0` leg, sigil silent, bytes agreeing; the
option is `gameRevision`, the other language is `.emp` (`[layout.odd-field]`), and the gap ledger row
"sigil has no odd-address lint on the AS route" carried it. None of those name `ASL-WARN-PARITY-ALIGN`,
which is why a search by row id found nothing. Worked on `parcel/as-warn-odd-address-align`: asl's rule
re-derived by 51 probes, `[as.odd-address]` implemented on the AS front end, notes and probe table in
`docs/superpowers/notes/2026-09-17-asl-warn-180/asl-warn-180.md`.

## S3K-FOUR-CLASSES: CLOSED, Sonic 3 & Knuckles builds whole and byte-identical

- **CLOSED 2026-09-25.** All four classes landed (codepage `8de29c7f`, `$$` labels `f535d779`, bcd and PC-indexed `cbc81b99`); the whole S3K image equals the reference, CRC32 `0658f691`, 0 bytes differ (`docs/superpowers/notes/2026-09-25-s3k-whole-rom.md`). Sonic 3 Complete also identical.
- history, kept:


- state at archive: `open`  size: `M`  project: `-`
- blockedBy: nothing, but see below

Board text: *"Sonic 3 and Knuckles is the next game that does not build: 120 rows in four classes.
Start with the one where our tool accepts a directive and then ignores it, which is the fault shape
we just measured."*

**The only 120 in the tree counts Sonic 1 BUILD CORNERS** (`docs/superpowers/notes/2026-09-16-as-width-suffix-bare-expr.md`),
a different subject entirely. This row's 120 is a count of S3K source rows in four fault classes and
has no artifact. The S3K hits under `docs/superpowers/notes/` are July handoffs about something else.
**Whether a real census was run and never written down, or the figure was absorbed from the adjacent
one, is not decidable from here, and that is the finding.** Re-measure before sizing anything off it.

**CORRECTED 2026-09-25: the premise IS in the tree, and the paragraph above is wrong about that.** The
census is `docs/superpowers/notes/2026-09-16-as-corpus-census.md`, headline 5 and section *Q6. S3K, the
next loud corpus*: 120 rows at sigil `72aca2e2`, skdisasm `2fcd861c`, in four classes (93 `$$name`
labels, 19 `codepage`, 6 `abcd`/`subx`, 2 `(d8,PC,Xn)`), with its raw row multisets beside the note.
The sweep that booked this looked under the right directory and missed it. Re-derive:
`grep -n "120 rows in four" docs/superpowers/notes/2026-09-16-as-corpus-census.md`. The re-measure on
current master is still owed and is step 0 of the parcel now working the row.

## OVER-ACCEPTANCE-THREE-SHAPES: PREMISE UNVERIFIED

- state at archive: `open`  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- blockedBy: nothing, but see below

Board text: *"We accept three shapes the old assembler refuses. Your decision puts them in the
settle-everything-else half, so closing them would be a second call, not part of this one."*

**The three shapes are named nowhere in the tree**, so the row cannot be worked without re-deriving
them, and its claim about which half of the owner's decision they fall in cannot be checked against
the decision either.

**CORRECTED 2026-09-25: the three shapes ARE in the tree.** `docs/superpowers/notes/2026-09-10-as-circular-split.md`,
section 6: `c4` (a `rept`/`ds` count naming a forward constant), `c5` (one naming a forward label past
an `org`) and `d2` (an `if` naming a forward label that settles), each refused by asl with *expression
must be evaluatable in first pass* and left accepted because the circular-split ruling's own words put
them in "everything else settles". They are watched: probes `firstpass_rept_forward_constant`,
`firstpass_rept_org_separated`, `firstpass_if_settles_false` in `crates/sigil-frontend-as/tests/as_over_acceptance.rs`,
rows of `over_acceptance/ledger.txt`. Closing them is a second decision, as the row says. Re-derive:
`grep -n "Sigil accepts three shapes" docs/superpowers/notes/2026-09-10-as-circular-split.md`.

## Where these three came from, and the correction to where they were first filed

Found 2026-09-16 by trying to start the `next` row and failing to locate its premise. Swept all 20
board rows with `git grep -l <id> -- docs/` and a positive control: 17 traceable, these 3 not.

**They were first written into `docs/OVERSEER-ROW-HISTORY.md`, which was the wrong file, and the
reasoning that put them there was wrong in an instructive way.** That file's header promises every
board row is reproduced verbatim, so it looked like the home for an untraceable row. **It is a
HISTORY file** — rows land there when they leave the board — **and THIS file is the live tracked
queue.** The seat hand-certified the first artifact it found carrying the right-sounding promise and
never swept for a sibling, which is the same defect the same seat had spent the hour correcting in
two other places. The `ROW-HISTORY` section is kept, since the sweep and its lesson are real, and now
points here for the live rows.

## The 2026-09-18 durability census, and the seven rows it moved here

`docs/lane-status.json` is gitignored in this repo, so a row that lives only on the board dies with
the session. The hub raised it after aurora measured its own board and found 8 of 20 rows in no
committed file at all. Census run here the same hour, one `git grep` per row id against
`origin/master`: **16 rows, 9 already booked in a file that records OPEN work** (this file, or
`docs/superpowers/notes/campaign-gap-ledger.md`), **7 not.** The seven are below.

**The question is not "does the id appear" and the census is wrong in both directions if you ask
it that way.** `docs/lane-log.jsonl` records what HAPPENED and `docs/decisions.jsonl` records a
QUESTION; an id in either looks booked to a naive grep and books no work. In the other direction,
`EDITOR-BINDING-COUPLING` has a whole committed note on its subject
(`docs/superpowers/notes/2026-09-10-editor-binding-cross-repo-coupling.md`) that never says the id,
so a grep calls it an orphan while the argument is durable. **Classify by what the FILE is for, then
check the subject separately.**

Rows keep their board state. Where a row's argument already lives in a committed note, the row
points at it rather than restating it, so there is one copy to keep true.

### SWEEP-OPTIMISED-COMPRESSORS: LANDED

- state: **LANDED** 2026-09-18 at merge `ac314f5c` plus lint fix `a1d0975f`, pushed  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Landing gate on the merged tree against `.aeon-sigil-ref` (aeon `ec640bcf`): 492 suites, 5,591
  passed, 0 failed, 0 skips, 492 of 492 binaries launched and reported, reconciling 5,581 baseline
  plus 10 new, clippy 0, GREEN, stamped tree `a1d0975f`.
- **It was identification, not implementation.** Both optimised formats are clownlzss's optimal
  parser, already vendored here, so they were byte-identical to p2bin on all 14 vectors first try
  with no tie-break tail. Plain `saxman` fell out free. `kosinskiplus` is now the only refused
  format and no corpus here selects it.
- Sonic 1 under `improved_dac_driver_compression` AGREES end to end (crc32 `faa36f4d`, 524,288 B);
  its cross product goes from 288 of 576 corners comparing nothing to 528 built by both, 528 agree.
- **What the refusal was hiding, now the leg compares bytes:** Sonic 2 under
  `improved_sound_driver_compression` differs by two, and it is not the compressor. Booked as
  campaign-gap-ledger **Open 5**, offsets pinned. Deliberately not refused; the argument is there.
- **The lint the parcel's own run never exercised:** implementing four of the five formats left a
  single-element loop that clippy rejects under `-D warnings`. Population sized without that flag
  at zero other sites, so one site was all of it. The agent reported no clippy run at all, which is
  the gap worth remembering: a returned green that never ran the lint bar is not a landing gate.
- Sonic 1 and Sonic 2 each ship a setting that squeezes data harder
  (`s1disasm/build.lua:10` `improved_dac_driver_compression`, `s2disasm/build.lua:10`
  `improved_sound_driver_compression`), selecting p2bin's `kosinski-optimised` and
  `saxman-optimised`. sigil implements 3 of p2bin's 7 formats and refuses both by name, so half of
  each corpus's swept corners record a refusal instead of comparing a byte.
- Argument and kill condition: `docs/superpowers/notes/campaign-gap-ledger.md`, the 2026-09-17
  `SWEEP-NIGHTLY` section, **Open 1**. Only the row ID was absent from a committed file; the work
  was booked there all along.
- Oracle: the corpora's own committed `build_tools/Linux-x86_64/p2bin`, which runs here.

### BRA-W-RANGE-UNCHECKED

- state: **open (watch)**  size: `S`
- ANSWERED and good: where the old assembler refuses a too-far jump, we refuse too, same file and
  line. No silent wrong ROM. Kept only until the standing check that now guards it
  (`scripts/switch_matrix_sweep.py`) has run a few nights.
- **Kill:** delete the row once the nightly has carried the check for several consecutive greens.
  Nothing announces that; it is a date check, not a red.

### SOUND-BLOB-REFREEZE-SP6

- state: **blocked**  size: `M`  blockedBy: aeon, then the owner (their card `SP6-MODULATION-DIVERGENCE`)
- Not a deadlock, though two boards read like one: the engine lane can merge on his ear alone, and
  our pin follows and repairs their build. Three sites plus a refreeze, not one line.
- Was booked ONLY in `docs/lane-log.jsonl`, which records landings and books nothing open.
- The wait is aeon's to carry and the hub has the card in front of him as the suite's oldest; this
  lane chases neither.

### EDITOR-BINDING-COUPLING

- state: **open**  size: `M`
- Four places in our tool are keyed to a name the engine's own tools invent. Rename a scene binding
  there and our checks break, with nothing in either repo saying why. Booked, not fixed.
- Full argument, committed and durable:
  `docs/superpowers/notes/2026-09-10-editor-binding-cross-repo-coupling.md`. That note does not
  carry this id, which is why the census flagged it; the subject was never at risk.

### EMP-RESIDENT-CONST-USE: CLOSED

- **CLOSED 2026-09-25, nothing built:** `d-31` answered `by-name` **by the HUB** in the owner's place (empyrean `796b508f`), the route that already works. He can reopen it with one word.
- history, kept:

- state: **blocked**  size: `M`  blockedBy: the owner, card `d-31` (filed 2026-09-12)
- Should a shared constant be importable between sound modules the way it already is elsewhere? A
  language question for him: four options costed, one already works today.
- Was booked ONLY in `docs/decisions.jsonl`. **A decision entry books the QUESTION; it does not book
  the work that lands behind the answer**, and this row is where his go arrives.
- Language surface, so it is propose-discuss-land under `d-6`: the design is this lane's, the
  agreement is his.

### DEB2-Z80-LABEL-IN-68K-TABLE

- state: **open**  size: `S`
- The debugger's name list files a sound-processor label at main-CPU address 0x8000, so a real label
  there loses its name. Needs a check against the old assembler and a talk with the engine lane first.
- Argument, and the reasons it was ranked out of `next` on 2026-09-12:
  `docs/superpowers/notes/2026-09-12-deb2-minus-20-bytes.md`, which names this id and carries the
  per-tree check (`grep -E ' : 8000 [A-Z] \|' <shape>.lst`). Booked in a dated note rather than
  here, so nothing swept it.

### PUB-STRUCT-SIZE-MANDATORY

- state: **open**  size: unmeasured (absent rather than guessed)
- The language spec says shared records must state their size, but nothing enforces it and 42 engine
  records do not. Enforce it or drop the word: ours to propose to him once something depends on it.
  Spec marks it open, empyrean `b5fe6efa`.
- **The only row of the seven booked NOWHERE**, under any name. Adjacent but not the same subject:
  `d-32` (answered) is about the size check running only when something else in the build happens to
  use the record. Re-derive the 42 before quoting it; it is a snapshot.

### AS-OVERACCEPT-SILENT-THREE: LANDED

- **LANDED** 2026-09-25 at gated merge `e5b6ee06`, pushed; 5640 passed, 0 failed. Function arity was already fixed at `1b8cb9ae`.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The three over-acceptances the 2026-09-10 gate found that no ruling chose to keep:
  `dir_padding_not_on_off` (`padding maybe` silently turns padding ON; `supmode` shares the helper),
  `expr_function_arg_count` (no arity check on user `function` calls) and `save_missing_restore`
  (the save stack is never checked at end of unit). Rows 44 to 46 of
  `crates/sigil-frontend-as/tests/over_acceptance/ledger.txt`. Unlike `c4`/`c5`/`d2` these are not in
  any ruling's keep-settling half: each is a typo or mistake asl refuses, and the first builds a ROM
  with a layout flag set the wrong way. Refusing them the way asl does is assembler internals under
  the autonomy directive, with a lane-log note.

### OVERACCEPT-LEDGER-STALE-ROW-SILENT: LANDED

- **LANDED** 2026-09-26 at merge `538382a0` (tip `53749139`), pushed. Ruled: a ledger row that no longer diverges from asl FAILS the gate (`no_stale_ledger_row`), both directions, naming the row. First run retired `dir_unknown_codepage`. 1046 passed, 0 failed.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The over-acceptance gate REPORTS a ledgered row that sigil now refuses and does not FAIL on it,
  so a fixed over-acceptance can stay listed as open indefinitely. Measured 2026-09-25:
  `expr_function_arg_count` was fixed at `1b8cb9ae` on 2026-09-11 and sat in
  `crates/sigil-frontend-as/tests/over_acceptance/ledger.txt` for two weeks, and a parcel was
  dispatched to fix it. Decide whether a retirable row should fail the gate (forcing retirement in
  the fixing commit) or whether reporting is enough; the evidence is the two weeks. Source:
  `docs/superpowers/notes/2026-09-25-as-overaccept-silent-three.md`.

### CLIP-OWN-ANCHOR-PRICING: LANDED

- **LANDED** 2026-09-25 at merge `011fcc5e`; merged-tree gate GREEN, 503 suites, 5723 passed, 0 failed
  (5707 + 16 new). Aeon's end-to-end verdict at `a1ff8796`: the format works. Their half lands after
  the shared binary is refreshed in the hub's window.
- was: state **open**, unblocked 2026-09-25: the owner answered `d-35-revised` = `clip-overlay-file` (record
  `d-35-revised-answered` in `docs/decisions.jsonl`; heard directly by aeon, relayed ~14:20Z, and on
  the console card). Build: a new `sigil build` switch naming a small positions file in the clip's own
  folder, written the way `map.toml` writes anchors, applied only when passed; `map.toml` untouched.
  Sigil lands first (switch + file format contract, and the emit must read the same positions),
  then aeon adds the file and passes the switch. Aeon re-derives the values as the act grows (their
  `7d409cdb` costing: one more CPZ section already needs 0xC0000/0xD0000 in DEBUG), so nothing here
  pins them. The companion check is `BANK-ID-EMIT-VS-PLACE-UNCHECKED`.
- landed on branch `parcel/clip-overlay` (not merged): `--anchor-overlay <path>` on `sigil build`
  and `emit_sound_blob`, one-pass apply over the map anchors and the frozen island rows, every
  refusal with its id; contract for aeon in
  `docs/superpowers/notes/2026-09-25-clip-overlay-contract.md`. Aeon passes the switch to
  `sigil build` only: on the preflight emit it reds
  `test_check_does_not_perturb_generated_sound_artifacts` (measured, see the contract).
  Canonical byte-neutral (s4, s4_debug, demo, demo_debug equal the provenance tip); landing run at
  `eb73887d` GREEN, 5723 passed, 0 failed (baseline 5707 + 16 new).
- was: state **blocked** on card `d-35`  size: `M` to build  project: `-`
- Priced 2026-09-25 at merge `603087e0`: `docs/superpowers/notes/2026-09-25-clip-own-anchor-pricing.md`.
  Sigil cannot tell the S2CLIP build from canonical sonic4, so the hub's own-anchor ruling (empyrean
  `e229e1a3`) needs a new map.toml key plus a build flag (design A, recommended) or a narrower flag
  (C). That is surface the engine's layout file is written in, so it is `d-35`. Gates aeon's row 8
  zone work, nothing tonight. Sigil lands first; aeon then adds the rows and passes the flag.

### MAP-WHEN-UNKNOWN-VALUE-SILENT: LANDED

- **LANDED** 2026-09-25 with CLIP-OWN-ANCHOR-PRICING, merge `011fcc5e`.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- `when_applies` in `crates/sigil-harness/src/map_placement.rs` treats any `when` value other than
  `sound_on`/`sound_off` as applying to every build (`Some(_) => true`). A typo in map.toml's `when`
  silently places a row in every shape. Both aeon maps use only the two known values (per the pricing
  note, inferred from a read, not a build), so refusing unknown values should move no bytes. Folded
  into design A of `d-35`; worth doing alone if `d-35` waits.
- landed on branch `parcel/clip-overlay` (not merged, commit `0a383b11`): `when` is a closed enum
  (`[map.when-unknown]`), and `[[anchor]]`/`[[hole]]` rows refuse an unknown key. Byte-neutral:
  the four canonical shapes equal the provenance tip.

### BANK-ID-EMIT-VS-PLACE-UNCHECKED

- state: **open**  size: `S`  project: `-`
- `emit_sound_blob` bakes sound-bank ids into the resident Z80 blob and `dac_sample_tab.bin` (aeon
  measured `ld a,$17 ; call SetBank` at blob 0x235, 0xD5E, 0xED4, 0x122C, `ld a,$15` at 0x41F, table
  bytes 0x15/0x16) before `sigil build` places the banks. Nothing is known to check that the baked ids
  equal the placed banks' ids, so a mismatch would play sound from the wrong bank silently. Our pricing
  note flagged the cache half (the sound fold check covers MT/SFX addresses, not DAC bank ids). Add a
  link-time check, proven red by moving a bank. Needed by `d-35-revised` either way; worth doing
  alone. Source: aeon 3f208336, docs/research/2026-09-25-clip-own-anchor-pricing.md.
- landed on branch `parcel/bank-id-emit-vs-place` (not merged): `[sound.bank-id-vs-placement]` in
  `crates/sigil-harness/src/sound_bank_ids.rs`, run after link in every full chained build. It reads every baked
  id out of the linked image, finding each site by re-emitting with the id moved, and compares it with
  `bank_id_of` of the placed bank label. It measured a hole the existing checks miss: DAC relocated to a
  consistent anchor plus an extra window in `dac_banks.emp` builds green at master with drums baked one bank
  low; the branch refuses it. Four shapes byte-identical to the provenance tip. The note's `ld a,$15` at 0x41F
  is the YM timer rearm, not a bank id.

### AS-REGISTER-SPELLED-LABEL-SILENT: LANDED

- **LANDED** 2026-09-25, fast-forward to `09af6350` (gate run at `29f64ad8`: 504 suites, 5728 passed, 0
  failed, 5723 + 5 new). 47 over-accepts refused, one rule; aeon four shapes and five corpus ROMs unchanged.
- was: state **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- With a label named like a register (`A1:`), sigil assembles `#A1+Lab2` where the pinned asl refuses
  it as a register, so a program asl rejects builds here with a value. Pre-existing, measured by the
  `$$` parcel with a `$$`-free control: `docs/superpowers/notes/2026-09-25-s3k-dollar-labels.md` and
  its gap-ledger rows (which also carry the `.`-local scope not following `cpu`/`padding`/`supmode`/
  `listing`/`restore`/`endstruct` as asl's does).
- On branch `parcel/as-register-label` (measurement `a2080de5` and `4d18bacc`, fix `63a678a5`): asl's rule
  measured over 127 probes (`docs/superpowers/notes/2026-09-25-as-register-spelled-label.md`): on the 68000
  `d0`-`d7`/`a0`-`a7`/`sp` in any case is the register in every expression whatever symbol exists. sigil now
  reads it so: 47 over-accepts to 0 (i09, j07, j08, z01 all refused with a sentence naming the shadowed
  symbol); the 58 accepted shapes keep asl's bytes; S1, S2, S3K, S3C, S3 byte-identical and refusal sets
  unchanged. The 6 former BYTES-DIFFER rows are now named refusals (silent-empty `dc` ruling, and
  `AS-REGISTER-ALIAS-SYMBOL` below).
  Aeon's four shapes at `ec640bcf` match the provenance tip (s4 `91c46c94/820209`, s4.debug
  `8a378de6/846509`, demo `1c7a34d3/96863`, demo.debug `72e405a5/103185`). Landing run at `29f64ad8`:
  GREEN, 504 suites, 5728 passed (5723 + 5 new), 0 failed, 2 ignored, clippy and ledger clean.

### AS-REGISTER-ALIAS-SYMBOL

- state: **open**  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- asl lets `X equ d3` and `X reg d3` name a register: `move.w X,d0` is `3003` (exit 0). sigil refuses both
  (`d3` is a register, not a value; `reg` is not a recognized 68000 mnemonic). With `A1:` defined,
  `X equ A1` is the alias of `a1` to asl (`3009`); sigil wrote `3038 1200` there before
  AS-REGISTER-SPELLED-LABEL-SILENT and refuses the line by name after it. Probes `alias_equ_d3`,
  `alias_reg_d3`, `alias_equ_A1_label` in `docs/superpowers/notes/2026-09-25-as-register-spelled-label/`.
  An over-refusal, never a wrong byte; no corpus or aeon source uses it (all build unchanged).

### PORT-TESTS-RED-AT-AEON-TIP

- state: **landed on branch** `parcel/port-tests-aeon-tip` (awaiting merge)  size: `M`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-25 (`docs/superpowers/notes/2026-09-25-port-tests-aeon-tip.md`), pin `ec640bcf`,
  tip held at aeon `8a6f92c4`: all 26 measured failures and the predicted `ojz_run_b_port` failure
  were repaired. The `ojz_run_b_port` one was MASKED behind an earlier section's byte failure;
  the gate now lowers every section first. None of the 14 port binaries fails on a name at the tip
  (26 name-shaped lines on master@tip, 0 on the branch). All 14 are green at the pin, and the full
  suite at the pin fails only the environmental `m1b_gate` row. Against pins regenerated at the tip
  (scratch tree, committed `pins.rs` untouched) 38 of the 40 tests are byte-identical.
  `test_objects_port` was a real bug, proven red-first: embeds joined the module directory instead
  of the aeon root. Values are derived from the tree under test: `sonic4_shape_defines`, zero-byte
  declaring modules, `listing_labels_if_defined`, family sweeps, and per-shape
  `mddbg_entry_labels`.
- OPEN for the pin advance: `ojz_run_b_port`'s declared `ojz_act_assets` shape delta (0xF) is
  stale at the tip. The actual delta is 0x5EC2, the three `OJZ_DEBUG_TEST_BGS` backgrounds. Derive
  it from the module; do not raise it. Masking in the other multi-section byte gates is unprobed.
- Measured 2026-09-25 (`docs/superpowers/notes/2026-09-25-aeon-481ac02e-port-impact.md`): against aeon
  at `b2820dc3`/`481ac02e`, 167 sigil tests fail that pass at the pin `ec640bcf`. 141 are expected
  (bytes and pins moved past the pin). **26 are unresolved-name failures in `*_port` tests** from aeon
  commits after the pin (`OJZ_Preset_Night`, `BG_VSCROLL_*`, `BG_Bands_Hold`, `GAME_SCANLINE_CAPS`,
  `CRASH_REPORT`, `MDDBG__ErrorHandler`/`DMA_Queue_End`, `Region_Resolve`/`Plane_Buffer_Peak`, and a
  doubled `embed` path in `test_objects_port`). None is from `481ac02e`. Also re-check `parallax_port` for aeon `6e1a4f80`'s new `pub equ band_record_len = sizeof(band_record)` when the pin moves (inferred same-module, not run). These must be repaired
  before the pin next advances; they are the port tests' hand-assembled dependency seams drifting.
- Predicted by aeon 2026-09-25, not run here: aeon `7c0baa73` (verified reachable from their
  `origin/master`) adds `use games.sonic4.ojz_clip_act_act1.{OJZ_CLIP_ACT}` to
  `games/sonic4/data/levels/ojz/act1/act_assets.emp`. `ojz_run_b_port` compiles that file standalone
  supplying only `BG_LAYOUT_SIZE`, so at the next pin advance it should report `unknown name
  OJZ_CLIP_ACT` unless the port supplies it, derived from aeon's neutral
  `games/sonic4/data/generated/ojz/act1/clip_act.emp` (value 0 there), never a copied literal.

### PORT-TESTS-HIDDEN-NAME-FAILURES

- state: **done in branch** `parcel/port-hidden-names` (awaiting merge)  size: `M`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-26 (`docs/superpowers/notes/2026-09-26-port-hidden-names.md`), pin `ec640bcf`,
  tip held at aeon `9caa1368`: a static scan of all 4511 tests (`mask_scan.py`, banked beside the
  note; positive control: it flags `ojz_run_b_port` at `78f5c7ca^` and clears it at HEAD) flagged
  66 aeon-reading tests; hand triage found NINE, in seven files, where a comparison that can fail
  on aeon drift ran before another module, shape or link was produced: `ojz_run_a_port` (2),
  `seam2_seq_colink` and `seam2_sfx_head_colink` both-shapes gates, `soundbankhead_port`'s pin
  test, `game_debug_port`'s flip, `seam2_phased_head` (2), `section_row_fixture`'s both-spellings
  gate. All nine now produce every part first (`a6691120`, `79c1cd94`), green at the pin, and
  red-first proven: the same mutations (earlier part wrong, a real unknown name in the later part)
  hide the name in the old structure and report it in the new one.
- At `9caa1368` nothing was hidden: every restructured gate reports the same first failure as
  before, or stays green. `game_debug`'s link stage is unmeasured there (resolve stops first on
  a visible name). The other 57 flags are single-unit, same-source reruns, or invariant checks;
  the note classifies each.
- OPEN: 12 tests fail on VISIBLE names at `9caa1368` (`Music_Service` in `game_loop`,
  `Music_Want` in `parallax`/`sound_api`), from aeon commits after `8a6f92c4`. They need the
  derive-from-the-tree repair before the pin advances past them.
- Suites at `6cd988ea`: pin 504 binaries, 5732 passed, 0 failed, 2 ignored; `repin --check`
  says `pins.rs unchanged`. Tip: the same 176 failures as before the parcel, 12 name lines both.

### MODULE-REGISTRY-HARDCODES-AEON-FILES

- state: **done in branch** (`parcel/module-registry-derive`)  size: `M`  project: `SIGIL-DECOUPLE`  asked by: aeon (HOLDING its deletion on us)
- 2026-09-26 DONE IN BRANCH: `registry()` / `demo_registry()` and the config_a/config_b push/filter edits are deleted.
  The build's module roots derive from `games/<g>/map.toml` `order` over the scanned tree
  (`crates/sigil-harness/src/module_roots.rs`): a label row names the in-scope module defining it (private heads
  included), `section:<name>` the module declaring it, `__align$<module>$N` its module; a row nothing defines is
  `[map.order-orphan]`, a doubly-defined row is settled by the `use` graph or `[map.order-ambiguous]`. Enumeration
  table + measurements: `docs/superpowers/notes/2026-09-26-module-registry-derivation.md`. RESIDUE kept in sigil and
  named: `SHAPE_GATES` (21 modules only some shapes place; the map's union `order` cannot say which; proposed home a
  per-row `when` on `order`), `ENGINE_TERMINUS` (`engine.epilogue`: the demo map has no `EndOfRom` row), and
  `SIGIL_PROBE_FIXTURES` (sigil's own `sigil_objroutine_probe`, which duplicates `TestSolid_Init`). `DECLARED` is left
  as is: a stale row is inert (measured), but a NEW section still needs a row by the R7 ruling; deriving it needs a
  ruling. Pinned tree ec640bcf, branch sigil f94cc693, all four shapes at the provenance tip: s4 91c46c94/820209,
  s4.debug 8a378de6/846509, demo 1c7a34d3/96863, demo.debug 72e405a5/103185. Aeon origin/master d3b98868 with
  path_swap deleted (the `.emp`, its `order` row, its objects.json entry, plus the two stamp files
  `tools/regenerate-level.sh` rewrites; generated level data unchanged): master sigil refuses (`no module
  games.sonic4.path_swap found under the scan root`); this branch builds s4 cf8606bb/829140, s4.debug
  08e41ec3/856276, demo 259fb22b/99127, demo.debug 8f135bcb/106273 (demo equals the undeleted control). Full landing
  run at f94cc693 GREEN: 506 suites, 5742 passed, 0 failed, 2 ignored. Aeon may delete once this lands and the
  shared pair is rebuilt.
- 2026-09-26: aeon wants to delete `games/sonic4/objects/path_swap.emp` (owner ruling: layer lines are the only layer-switch
  mechanism; aeon confirmed the path is `games/sonic4/objects/path_swap.emp`; aeon's half is booked blocked on this row) plus its `map.toml` row and
  `objects.json` entry. The build refuses, because sigil's whole-program module list is hard-coded:
  `crates/sigil-harness/src/native.rs` `registry()` (the `m!("games.sonic4.path_swap", "path_swap")` row) and
  `section_align.rs` `DECLARED` (`d("ObjDef_PathSwap", 2, WORD)`). Checked here 2026-09-26. `pins.rs` `PATH_SWAP`,
  `repin.toml`'s path_swap region and anchors, and `test_g4_final_objects_port.rs` describe the PINNED tree (ec640bcf,
  which has the file) and do not block aeon; they move with PIN-ADVANCE.
- The trap: deleting the registry row breaks builds of the pinned tree, which still ships the module. So the list must be
  derived from the tree being built, most likely from `map.toml`'s sections, which aeon edits anyway; never from a
  file's mere presence. Same class as BLOB-LEN-PIN-OFF-EMIT-PATH: a pinned-corpus fact sitting on the build path.
  First step: enumerate every other hard-coded aeon module or anchor on the build path, not just this one. Order agreed
  with aeon: sigil lands first and rebuilds the shared pair (from a durable path, then LOCK it, see
  INSTALLED-BINARY-READS-BUILD-TREE), then aeon deletes.

### INSTALLED-BINARY-READS-BUILD-TREE

- state: **done in branch**  size: `S`  project: `SIGIL-DECOUPLE`
- 2026-09-26, branch `parcel/installed-binary-selfcontained` (tip in the parcel report). The seven
  `golden/offcanonical_sizes/*.txt` tables are compiled in (`native::FROZEN_TABLES`, `include_str!`); rejected:
  resolving them from a named location at run time, which moves the dependency instead of removing it. Enumeration
  (binary strings + source grep, classified), consumers and evidence: `docs/superpowers/notes/2026-09-26-installed-binary-selfcontained.md`.
  The only run-time read of the build tree on either binary's path was `load_frozen_table`, and only `sigil` reached
  it: master's `emit_sound_blob` emits byte-identically with its tree removed. Falsifier, binaries built in a
  throwaway worktree that was then removed: master `c024aba8` fails all four aeon shapes with `read frozen table ...
  No such file`; the parcel builds all four at the provenance tip's CRC32/size. Consequences: the `.lst` digest loses
  its one `root=sigil` row (the table is now part of the assembler, covered by `DIGEST-ASSEMBLER revision=`), so
  aeon's `artifact_provenance.py` no longer opens anything in the sigil tree either; the `--version` closure now
  follows include/path references to their files (38 paths, was 29); a prebuilt `REPIN_BIN` in `refreeze` step 3 now
  keeps the tables it was compiled with (default `cargo run` recompiles). Gates: `installed_binary_needs_no_build_tree`
  (sigil-cli and sigil-harness, `bwrap` hides the checkout), `frozen_tables_embedded`, the extended
  `version_provenance` closure mirror; each red first.
- The standing rule below ("lock the build tree at swap time") is UNNECESSARY for a pair built from this change or
  later. It still binds the pair installed today (built at `4ce2509d`): keep `.worktrees/land-4ce2509d` locked until
  a pair built from this change is swapped in. The unlock is the overseer's. Heads-up for aeon at that swap: the
  digest's `reads=` drops by one (the `root=sigil` row).
- 2026-09-26: the shared `target/release/sigil` reads `<build tree>/crates/sigil-harness/golden/offcanonical_sizes/*.txt`
  at RUNTIME through a baked `CARGO_MANIFEST_DIR`-style path. The pair was built at `.worktrees/land-4ce2509d`; removing
  that worktree after the swap broke every aeon demo build ("read frozen table ... demo.txt: No such file", exit 101)
  until it was recreated at the same path and commit. It is now `git worktree lock`ed with a reason. STANDING until
  fixed: the tree an installed pair was built from is part of the install; lock it at swap time and remove it only
  after a later pair is swapped in from elsewhere. The fix: embed the frozen tables (`include_str!`) or resolve them
  from a named reference tree at run time, and grep the binary for every other baked build path (`strings | grep
  .worktrees`) before choosing. Found by aeon.
- Correction from aeon, same day: sonic4 shapes broke too while the tree was missing (a panic reading
  `offcanonical_sizes/s4.txt`), not only demo. The frozen tables of every shape are read at run time.

### BLOB-LEN-PIN-OFF-EMIT-PATH: LANDED

- **LANDED** 2026-09-26 at merge `4ce2509d` (tip `4fc0ccfa`), pushed. The emit writes the resident blob at the tree's
  length (no length refusal, `SIGIL_BLOB_LEN_DRIFT` removed); `BLOB_LEN_*` are pinned-corpus test assertions
  (`emitted_blob_lengths_are_the_pinned_corpus_lengths`, sigil-cli); `seam1_emit_length` (nightly source-gate list)
  proves a grown blob emits. No ceiling in the emit: aeon's `ensure(Z80_SOUND_SIZE <= SND_STATE_BASE)` is in
  `engine/system/boot_data.emp` (not `sound_constants.emp`, which holds only the comment). Merged-tree re-gate 23/23.
  Shared pair rebuilt from a clean worktree at `4ce2509d` and swapped by rename: sigil `1edd31eb`, emit_sound_blob
  `3c3bd0ba`; previous pair (ab0a5fe1) kept at `~/sonic_hacks/.sigil-pair-archive/ab0a5fe1/`. Old vs new emit on
  ec640bcf: 19/19 files byte-identical; with +17 B in `Psg_HwCh` old refuses, new writes 6193/6323. Aeon told.
- For PIN-ADVANCE: aeon `e6a00773` landed the PSG envelope fix on this pair: blob 6194 B plain / 6324 B debug,
  `Z80_SOUND_SIZE` `$1832` / `$18B4` (aeon's figures, not measured here), no new cross-seam names. `BLOB_LEN_*` and the
  Z80_SOUND_SIZE mirrors move with the pin when it passes that commit.
- was: state **doing** from 2026-09-26  size: `S`  project: `SIGIL-DECOUPLE`
- Asked by aeon 2026-09-26: a PSG envelope fix (`PsgEnvUpdate` once in `Seq_HookNoteOn`, +17 B resident; evidence
  aeon `67a7a374` `docs/research/2026-09-26-s2-music-volume.md`) is refused by `emit_sound_blob`: "plain blob is 6193
  bytes, expected 6176". `seam1::BLOB_LEN_PLAIN`/`BLOB_LEN_DEBUG` is both the pinned-corpus length assertion and a
  hard refusal in the emit path aeon's build runs, so a re-pin reds our pinned tests. Ruled here: the refusal leaves
  the emit path (a ceiling only if derived from the tree), the exact length stays a test assertion against the pinned
  tree. Aeon guards the real hazard itself (`Z80_SOUND_SIZE <= SND_STATE_BASE`). After landing: rebuild the shared
  `emit_sound_blob` and tell aeon the SHA. Branch `parcel/blob-len-off-emit-path`; agent tree
  `/home/volence/sonic_hacks/.aeon-sigil-ref-blob` (remove after).

### OJZ-RUN-B-SHAPE-DELTA-BOUND: LANDED

- **LANDED** 2026-09-26 at merge `0631e755` (tip `a45cb079`), pushed. `crates/sigil-cli/tests/ojz_run_b_port.rs`:
  `ojz_act_assets`' plain/debug span allowance is `ALIGN_PAD` plus the debug-only bytes `act_assets.emp` declares
  through `[u8; LEN]` types (evaluated both shapes). Pin: 0 extra. Aeon origin/master: 24,258 extra; the real delta
  there is estimated, not measured, until the pin advances. Red-first proven.

### CART-CHECK-CITES-STALE-PEER-BEHAVIOUR: LANDED

- **LANDED** 2026-09-26 at `375ad832`, pushed. The comment now states our own reasons (one round trip, whole-image coverage) and cites the contract's `memory_hash` entry (section 6) and 11.48 by section, naming no peer tool. `ab_cart_check` 24 passed, 0 failed. Announced to aeon.
- was: state **open**  size: `S`  project: `-`
- `crates/sigil-harness/golden/ab/cart_check.py:53` says aeon's `tools/evict_witness.py` "already runs
  exactly this shape" (a whole-image `memory_hash` as its stale-binary guard). At aeon `origin/master`
  that file no longer mentions `memory_hash` at all: `git -C ../aeon log -S memory_hash origin/master --
  tools/evict_witness.py` names `8c83475a` as the commit that removed it. The claim decayed with no edit
  here. Reword it to state our own reason without citing a peer's behaviour. The file sits under
  `golden/`, so it is a comment-only edit announced to aeon, not a silent one. Found 2026-09-25 while
  checking aeon's consumer notice for `27684931`, which itself changes nothing we execute.

### AS-MULTICHAR-SQUOTE-STRING: LANDED

- **LANDED** 2026-09-25 at `485fbd95`, pushed; Sonic 3 alone byte-identical (`9bc192ce`). 5699 passed, 0 failed.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- The whole remaining gap for Sonic 3 alone (`buildS3.lua`): 6 rows, all a multi-character `'...'`
  in `dc.b`, which asl treats as a string (one charset-translated byte per character; `'AB'+1` gives
  `4143`) and sigil's lexer turns into one integer. Requoting those 6 operands gives the reference md5
  under sigil. Probes sq1 to sq4 in `docs/superpowers/notes/2026-09-25-s3k-whole-rom/`.

### AS-CLI-DEFINE: LANDED

- **LANDED** 2026-09-25 at `ae50976a`, pushed; 5694 passed, 0 failed.
- was: state **doing** from 2026-09-25  size: `S`  project: `SIGIL-AS-REPLACEMENT`
- `-D NAME[=VALUE]` on the assembler path, asl-exact: a SET variable (later `set` may change it, `=`
  and `equ` refused as asl #2035). Removes the S3K wrapper root. Ruled here, logged: NOT wired to
  `Options.defines`, whose documented let-the-file-win semantics would accept two programs asl
  refuses. Measured in `docs/superpowers/notes/2026-09-25-s3k-whole-rom.md`.

### SEAM2-SONG-COUNT-LITERAL

- state: **done in branch** `parcel/seam2-song-count`  size: `S`  project: `SIGIL-DECOUPLE`
- Asked by aeon (Sonic 2 zone music): the `mt_bank` cross-seam carrier restated `SONG_COUNT` as
  `if debug { 3 } else { 1 }` (and `SONG_MOVINGTRUCKS = 1`), so an aeon-only song addition fired
  `mt_bank.emp`'s drift guard in sigil's emit. Now `seam2::song_id_carrier` evaluates
  `games/sonic4/config/sound_ids.emp`'s `pub const`s through `eval_all_pub_consts` under the shape's
  `DEBUG`; `seam2::mt_bank_carrier_asm` is the one carrier source. Same fix at every other consumer:
  `mt_port.rs`, `mt_negative_probes.rs` (both carriers), `sound_api_port.rs` (`SONG_COUNT = 3` in both
  shapes, now per shape).
- `size = 0x79F9` was the room from the ORIGINAL `mt_bank` LMA `$58607` to its bank top `$60000`, never
  re-derived when the LMA became map-derived: at the pin (LMA `0xB8630`) the region ended at `0xC0029`,
  `0x29` past the `$8000` window. Now `(sound_bank + 0x8000) - mt_bank_lma` (`0x79D0` at the pin), the
  bound `mt_port.rs` already used. Placement-only; no byte change.
- Positive control, scratch copy of the pin tree, `emit_sound_blob` master vs branch: authority-only
  debug arm 3 -> 5: master exit 0 (blind), branch fires `SONG_COUNT drifted ...: 3` (the local mirror
  lags). Full aeon-side change (authority 5, mt_bank local 5, two more table rows): master fires
  `SONG_COUNT drifted from games.sonic4.sound_ids: 5` (aeon's reported failure), branch exit 0 with
  20-byte debug SongTable/SongPatchTable. Unmutated: master and branch emits identical (19 files).
- Adding a song is still a TWO-file aeon change (`sound_ids.emp` and `mt_bank.emp`'s local
  `SONG_COUNT` + table rows): the local mirror and its drift guard are aeon's design, not sigil's.
- Not fixed, same class: `sound_api_port.rs` supplies `SFXID_RING_RIGHT/LEFT = $33/$34` as literals.
- Proof at pin `ec640bcf` (`.aeon-songcount-pin`), code tip `8193dcaf`: `repin --check` "pins.rs
  unchanged"; full strict suite 504 binaries launched / 504 reported, 5729 passed, 1 failed (the
  environmental `m1b_gate::oracle_loadfromaslisting_resolves_emit_listing`), 2 ignored. Red-first:
  both a formatter literal and a core literal turn `mt_bank_carrier_song_count_follows_the_authority` red.

### SHARED-PAIR-SWAP-F52609FE

- state: **done 2026-09-25T23:07:11Z** (outgoing pair frozen at `.sigil-outgoing-1d19e60b`, md5s matched; the six steps were issued as one parallel batch rather than read one by one, outcome verified correct)
- original state: **waiting**  size: `S`  project: `-`
- Asked by the hub 2026-09-25 for aeon's music step 4. The candidate pair is built ASIDE from origin/master
  `f52609fe` in the durable detached worktree `/home/volence/sonic_hacks/.sigil-pin-f52609fe`
  (`target/release/sigil` md5 `cdf3ec1abe0d33d8042b7cf5092e5e64`, `emit_sound_blob` md5
  `ed45bedb7ab47972d570536b8d215d22`; `--version` clean, closure-revision `f52609fe`). The outgoing shared
  pair at `/home/volence/sonic_hacks/sigil/target/release/` is `1d19e60b` (sigil `fed84c1967b3b92a57402e86d494cc89`,
  emit_sound_blob `3f2e7322d1cdb1e41dec32ed1637d5e8`). Re-measure all four md5s before acting.
- **Swap ONLY when both are true:** aeon reports its two running builds done and its byte-identical control
  against the candidate clean, AND the hub opens the window. A control difference is a finding: stop.
- **Each step is its own tool call, read before the next; never a `set -e` chain** (zsh does not stop on it):
  1. `mkdir /home/volence/sonic_hacks/.sigil-outgoing-1d19e60b` and copy the live pair into it.
  2. md5 the copies and compare with the live pair. Do not continue unless both match.
  3. Copy each candidate binary into `target/release/` under a temporary name, md5 it against the candidate,
     then `mv` it over the live name (a rename, so a running reader keeps the old inode).
  4. md5 the live pair against the candidate, run `target/release/sigil --version` (expect `f52609fe`,
     source `.sigil-pin-f52609fe`), freeze the outgoing directory, tell aeon and the hub.

### SEAM1-BANKED-CARRIERS-DERIVE

- state: **done in branch** `parcel/seam1-banked-derive`  size: `M`  project: `-`
- Asked by aeon 2026-09-25; blocks their S2CLIP-REGION-MUSIC step 2 (Sonic 2 PSG envelopes). Same family as
  SEAM2-SONG-COUNT-LITERAL.
- Premise verified here at master `7c081077`: `crates/sigil-harness/src/seam1.rs` `banked_carriers()` hand-pins
  ELEVEN `$8000`-window VMAs. `check_banked_carrier_drift` compares only the three HEAD-level ones
  (`SeqOpcodeTable`, `SfxBlobWinTab`, `SndDefaultPitchTable`, derived by `seam2::banked_head_vmas`); its own doc
  comment says the rest (`FmPitchTableZ`, `LogVolumeLutZ`, `CarrierMaskTableZ`, `PsgDivisorTableZ`,
  `PsgVolEnv_Ids`, `PsgVolEnv_Ptrs`, `FmVolEnv_Ids`, `FmVolEnv_Ptrs`) are offsets inside `SoundTablesZ80_Head`
  and "stay hand-maintained and unchecked". These literals are baked into the shipped resident driver's operand
  bytes, so a stale one is a WRONG blob that a refreeze would bless. Also `seam2.rs` `SOUND_TABLES_Z80_LEN = 0x357`.
- Aeon's measurement (theirs, not re-run here): their branch `parcel/s2-music-envelopes-table` at `f7dda3e2`
  (pushed to aeon origin) appends 5 envelope bodies, table 855 -> 985 B; the drift check refuses on the three
  head members, while `PsgVolEnv_Ptrs` 0x828F -> 0x8294, `FmVolEnv_Ids` -> 0x83B7, `FmVolEnv_Ptrs` -> 0x83BA move
  silently. Updating only the three reported numbers would go green with every envelope read from a wrong address.
- Ask: derive ALL eleven and the length from the lowered `sound_tables_z80.emp` (its own labels), so a table
  change is aeon-only; retire or widen the drift check accordingly. Byte-neutral at the pin by construction.
  Positive control: aeon `f7dda3e2` builds under the new code with correct envelope addresses; master refuses it.
- Outcome: all eleven carriers and the table length are derived. `seam2` lowers and links
  `sound_tables_z80.emp` once and reads each label's window VMA off the RESOLVED section (section `vma:` plus
  final label offset); `seam2::banked_carrier_vmas_in` = the three head members from `sound_layout` then the
  eight table labels (`SOUND_TABLES_BANKED_LABELS`, names only). `seam1::banked_carriers(aeon, ov)` is that
  derivation on every emit path, under the caller's anchor overlay. An absent label is an error naming it; a
  table `vma:` that disagrees with the map's `sound_bank` window is refused. The size-only path (run from
  inside `sound_layout`) links every carrier at a `$8000` placeholder (`BANKED_CARRIER_SIZE_PROBE`, same
  argument as `DAC_SAMPLE_TABLE_SIZE_PROBE`). `SOUND_TABLES_Z80_LEN` is gone; the colink gate slices by the
  emitted length.
- Drift check RETIRED, not widened: with the carriers derived from the authority it compared against, it
  could only compare a derivation with itself. Replaced by `seam2` unit tests (the real table with one id and
  one pointer appended in memory: each carrier moves by exactly what precedes it; absent label and window
  mismatch refused by name) and `tests/banked_carrier_derivation.rs` (renamed from `banked_carrier_drift.rs`,
  `nightly_source_gates.sh` follows): all eleven produced in order, table labels inside the table, and each of
  the eleven moves the linked resident blob when doctored (every one is read by the driver).
- Byte-neutral by construction at the pin `ec640bcf`: the derivation yields exactly the eleven retired literals
  and 855; `emit_sound_blob` master vs branch, 19 artifacts byte-identical.
- Positive control at aeon `f7dda3e2`: master `emit_sound_blob` refuses naming only the head three; the branch
  derives `PsgVolEnv_Ptrs` 0x8294, `FmVolEnv_Ids` 0x83B7, `FmVolEnv_Ptrs` 0x83BA, head 0x83D9/0x84E1/0x85F3,
  length 985 (0x3D9): aeon's figures, and an independent Python count of the `.emp` agrees. The emitted blob
  carries each at its `ld hl`/`ld de` operand (plain blob `+0x16A1`: `11 94 82`), and the `ld b` scan count
  `PSGVOLENV_COUNT` reads 16.
- FOR AEON, f7dda3e2 still does not build: aeon restates the table length itself.
  `soundbankhead.emp` walls `_sound_tables.len == $357` and `_dacsamp.len == $7F`, and `dac_sample_tab.emp`
  has `DAC_HEAD_PREFIX = $357 + ...` (which sizes `DacHeadPad`, 7 -> 5 at 985 B). All loud (the walls fire).
  With those three edited in a scratch copy, `sigil build --native --game sonic4` builds (crc 8b9761e2, 822231
  B), the listing places the heads at 83D9/84E1/85F3/8633, and ROM `$B8000` holds the 985-byte table.
- Not fixed, same class: `seam2_layout_derivation.rs` pins the layout as frozen literals by design (its
  `pitchtable_lma` encodes 855) and refreshes with the pin; `PITCHTABLE_LEN = 264` and the `size = 0x400`
  sound-tables region are hand values; the 2026-08-26 note in `tests/repin_pins.rs` still describes the
  carriers as unchecked (a dated record, left as written).
- Proof at pin `ec640bcf` (`.aeon-sigil-ref`), full strict suite, `ORACLE_DIR` named, target dir outside the
  checkout: before (code `ec0a6106`) 504 result lines, 5730 passed / 0 failed / 2 ignored, exit 0; after (tip
  `d2fe50b9`) 504 result lines, 5732 / 0 / 2, exit 0 (two drift tests retired, two derivation tests and two unit
  tests added). `repin --check` "pins.rs unchanged" before and after; clippy `-D warnings` exit 0. Red-first,
  each mutation shown then restored from the commit: a restated `FmVolEnv_Ptrs` turns
  `banked_table_carriers_follow_the_table_labels` red while the pin's blob byte gate stays green (the blind
  spot); an absent label mapped to 0, a `PsgVolEnv_Ptrs` off by one (byte gate red at `ld de` `+0x16A1`), a
  doctor that skips `FmVolEnv_Ids`, and a head member off by one each turn their gate red.

### SHARED-PAIR-SWAP-AB0A5FE1

- state: **done 2026-09-26T03:20:12Z** on the hub's go, each step its own call (outgoing pair frozen at `.sigil-outgoing-f52609fe`, file and directory, refusal observed; md5s matched)
- original state: **waiting**  size: `S`  project: `-`
- For aeon's Sonic 2 envelopes (their S2CLIP-REGION-MUSIC step 2), which need SEAM1-BANKED-CARRIERS-DERIVE in the
  shared pair. The candidate is built ASIDE from origin/master `ab0a5fe1` in the durable detached worktree
  `/home/volence/sonic_hacks/.sigil-pin-ab0a5fe1` (`target/release/sigil` md5 `3fb7c0b35df4ce3e5cd4dfdbb83710d8`,
  `emit_sound_blob` md5 `f0fedab42d72ad1b5cf734591c18a57f`; `--version` clean, revision `ab0a5fe1`). The outgoing
  shared pair is `f52609fe` (sigil `cdf3ec1abe0d33d8042b7cf5092e5e64`, emit_sound_blob
  `ed45bedb7ab47972d570536b8d215d22`). Re-measure all four md5s before acting.
- **Swap ONLY when both are true:** aeon reports its byte-identical four-shape control against the candidate clean
  with no aeon build running, AND the hub opens the window (it polls aurora and oracle, then says go). Aeon's report
  alone is not a go. A control difference is a finding: stop.
- Same steps as SHARED-PAIR-SWAP-F52609FE above, with `1d19e60b` read as `f52609fe` and `f52609fe` as `ab0a5fe1`.
  Each step is its own tool call, read before the next.

### PIN-ADVANCE-S2-ENVELOPE-RESTATEMENTS

- state: **open**  size: `S`  project: `SIGIL-DECOUPLE`
- **Also red at the next pin advance (measured 2026-09-28 by the SONG-BANK-2-EMBED-ROOT agent against aeon
  `478fa091`, not re-run here):** `mt_negative_probes.rs` wrong-bank probe expects 5 co-residency ensures to fire
  (aeon master has 9, 0 once aeon deletes them); `mt_port.rs` guard count expects 7 (aeon has 11, 2 after). Once
  aeon respells `mt_bank.emp`'s embeds root-relative, the probes that lower it with the sound dir themselves also
  break (`mt_negative_probes` straddle and wrong-bank, two `mt_port` region tests, `sfx_negative_probes` (e));
  `mt_bank_port.rs` reads `mt_bank_blob.emp`, which aeon deletes. Record:
  `docs/superpowers/notes/2026-09-28-song-bank-2-embed-root.md`.
- **Aeon landing notices to fold in at the next pin advance (2026-09-28, their words, not verified here):**
  `d33fa8d0` (+8 B inside `Parallax_State`, every later RAM address moves; new names `Parallax_Shadow_Key_Config`,
  `_VS`, `_K`); `2d006c4c` (new `Cache_H_Pfx_Run: u16` in ram.emp, `H_PFX_ARM = 128` plus an ensure in
  constants.emp; canonical s4 `80dc4338/830244`, debug `3a8a8e5a/857227`). `*_port` lists may need the new names.
  `ac17f3a6` (song-bank-2 step 2: new driver proc `SndDrv_SongBankIn`; the Z80 blob grows +20 B so labels from the
  blob end through `ObjCodeBase` move +20, and one debug jump in `GameState_OJZScroll_Init` goes long, +2; s4
  `76835839/830244`, debug `dd3cf064/857247`). A Z80 blob size change touches seam-2's predictions and the sound
  pins: re-derive them at the advance, never copy these figures.
- Aeon's heads-up 2026-09-26 (their agent's reading, NOT verified here): once aeon lands
  `parcel/s2-music-envelopes-table-2` (tip `6c3cd4cf`), these sigil-side restatements go red when our pin reaches
  it: `seam2.rs` `DAC_SAMPLE_TAB_LEN = 127` and `seam2_dac_head_colink.rs` (the table is now 120 B; the 8-byte pad
  moved into `soundbankhead.emp`, sized from the heads' `.len`); `seam2_layout_derivation.rs` pins `pitchtable_lma
  0xB8357` and the later heads; `pins::SOUNDBANKHEAD` length used by `soundbankhead_port` (head now `$6B0`); the
  "VMA $8357" comment in `seam2_pitchtable.rs`. Also aeon `movingtrucks_pitchtable.emp` vma moved `$8357` -> `$8000`
  window base (emitted .bin unchanged per their cmp).
- Second heads-up 2026-09-26 (aeon's reading, NOT verified here): aeon `b90cd650` adds the two Sonic 2 songs and
  renumbers song ids (`SONG_S2_EHZ=2`, `SONG_S2_CPZ=3`, DrumTest 2 -> 4, HCZ2 3 -> 5; `SONG_COUNT` 3 plain / 5 debug).
  Our `SONG_COUNT` derivation took it unchanged, but any sigil test or off-canonical golden (`--config-a`/`-b`, the
  hotkeys profile) naming song ids by number, or pinning song-bank offsets or sizes, moves when the pin reaches it.
- Ask when it is worked: derive what can be derived (the SEAM1 pattern), leave the pin-frozen ones to the refreeze,
  and re-derive the list from the tree at the commit rather than from this row.
- Measured at aeon `d7103d30` (with committed AND tip-regenerated pins): `docs/superpowers/notes/2026-09-26-port-music-names.md`,
  section PIN-ADVANCE. All five items confirmed; `soundbankhead_port` stays red after a repin (its length is a `repin.toml` literal).

- Sequencing, 2026-09-26: aeon says the advance (including the `repin.toml` `soundbankhead` `len` 0x630 -> 0x6B0 edit)
  is sigil's to schedule under the retired paired-freeze ruling, holds nothing for it, and will measure any aeon-side
  fact on request. That settles WHO sequences it, not WHETHER: advancing the pin is a decision, tested against the
  hub's 2026-09-02 ruling that the pin exists so the corpus does NOT track tip (`docs/OVERSEER-REFERENCE.md`, *A TASK
  RELAYED AS ROUTINE*). Run `refreeze --check` first to know whether it is a repair or a decision.
- Asked aeon 2026-09-28 (sigil session message, on the hub's steer): name a quiet aeon revision for sound and RAM
  layout (suggested after their PPB4 and woven sizing) and confirm the sound bank head span there. We advance to THAT
  SHA, never to tip, and edit `repin.toml` `soundbankhead` `len` to match it. Aeon sets the timing; nothing is held.
  Resume when their reply names the revision.
- Third heads-up 2026-09-26 (aeon's agent's reading, NOT verified here): aeon `f136c486` (slope landing fix) adds a
  cross-module name `Player_SensorLand` (`games/sonic4/player/player_sensors.emp`, used from `player_air.emp`); the
  port tests run against aeon master report it as the only unresolved name, 39 failed / 24 passed, same at their base
  and tip. Needs the derive-from-the-tree repair before the pin passes it. Possibly coming, only if the owner picks it:
  `LayerLine`, `Act.act_layer_lines`, `Player_LayerLines` (geometry constants in `engine/system/constants.emp`).
- Fourth heads-up 2026-09-26 (aeon's reading, NOT verified here): that branch LANDED at aeon `332cc1ba`. New cross-module
  names: `LayerLine` (engine/structs.emp), `Act.act_layer_lines` (Act 46 -> 50 B), `LL_*` (engine/system/constants.emp),
  `PlayerBlock.ll_prev`/`ll_cursor` (games/sonic4/config/ram.emp), `Player_LayerLines` (player_common.emp),
  `OJZ_CLIP_LAYER_LINE_ROWS`/`ojz_clip_act_layer_lines` (generated clip act). A struct-size move touches every port
  that lays out `Act`. Coming: `Player_LoopCrossover`, `CrossoverTable`, `XOVER_*` retire in favour of lines (owner
  ruling, per aeon).
- Fifth heads-up 2026-09-26 (aeon's reading, NOT verified here): aeon `19978b00` REMOVES `Player_LoopCrossover`,
  `CrossoverTable` (named by our `test_p1_player_port.rs`, `repin.toml`, `pins.rs`), and `XOVER_NONE`/`XOVER_TO_A`/
  `XOVER_TO_B`/`XOVER_LAYER_BIAS`; `PlayerBlock` is 8 B shorter (`ll_prev` @20, `ll_cursor` @24); ADDS `OJZ_Act1_LayerLines`
  and module `games.sonic4.ojz_layer_lines_act1` (`OJZ_ACT1_LAYER_LINE_ROWS`).
- Sixth heads-up 2026-09-26 (aeon's reading, NOT verified here; "nothing owed"): aeon `b62f559c` renames the page cache.
  ADDS RAM `Page_Live_Masks`, `Page_Live_RowPtr`, `Page_Live_Unit`, `Page_Live_Seen`, `Page_Live_Forced` (DEBUG only);
  constants `PAGE_LIVE_UNITS`, `PAGE_LIVE_COL_OFFSET`; procs `PageCache_PickVictim`, `LiveSweep`, `LiveSweepAll`,
  `LiveReset`, `LiveUnion`, `ReleaseHolds`. CHANGES `PatchRun_Col` (new `d1` input). REMOVES `PageCache_ResetRefcounts`,
  `PF_EVICTABLE`, `PF_EVICTABLE_BIT`, `Page_Audit_Scratch`; `pf_refcount` renamed `pf_pad`. Their agent notes our
  `tile_cache_port` name list lacks these, as it already lacked GPL-2's `PageCache_DemandHoldTick`; untested.

### PORT-TESTS-MUSIC-NAMES-AT-TIP

- state: **done in branch** `parcel/port-music-names` (awaiting merge)  size: `S`  project: `SIGIL-DECOUPLE`
- Outcome 2026-09-26 (`docs/superpowers/notes/2026-09-26-port-music-names.md`), pin `ec640bcf`, tip held at aeon
  `d7103d30` (`.aeon-music-tip`): THIRTEEN tests stopped on a name, not twelve. The thirteenth,
  `tranche5::misspelled_extern_slot_is_loud`, hid `Music_Want` behind a bool control (it failed the same way at
  `9caa1368`); its control now reports the messages, proven red-first. Repaired from the tree under test:
  `Music_Service` from each shape's listing (`game_loop_port`), `Music_Want` and `Music_Current` (the second was behind
  the first) from the listing (`parallax_port`, `sound_api_port`), harness-private carriers in the synthetic probes
  (`game_debug` flip, `tranche5`). `drain_define_is_load_bearing` asserted a literal 4-byte delta; the tip gates two
  calls, so the delta is now counted from `game_loop.emp`. `game_debug`'s flip link stage, measured at the tip: green.
- Name-shaped lines at the tip 12 -> 0; failing set 191 -> 186, none left fails on a name (byte, length, golden,
  pin or provenance drift, plus the S2 restatements the PIN-ADVANCE row covers, not all of which a repin clears).
  Against pins regenerated at the tip (scratch copy) all 15 tests in the five binaries pass.
  Suites at `9d38e63d`: pin 504 binaries, 5732 passed, 0 failed, 2 ignored, `repin --check` says `pins.rs unchanged`.

### SONG-BANK-2-STUB-PROBE

- state: **LANDED 2026-09-28 at merge `599008e8`** (parcel tip `4fe49de3`), sent to aeon  size: `M`  project: `-`
- Results: `docs/superpowers/notes/2026-09-28-song-bank-2-probe.md`. Without the anchor plain/debug refuse
  `[map.undeclared-island]` at 0xC0000; with it every shape builds, head at 0xC0000; a refreeze does not pin it.
  Debug has 1,508 B of bank-1 growth before `[map.anchor-absent]` (aeon quoted 1,509: the test is strict).
- *(The ask as filed, kept below.)*
- The ask, from aeon's overseer, full reasoning at aeon `f49dd2bd` `docs/research/2026-09-27-song-bank.md`, section
  "Our side: which shape". Shape (a): a plain-address bank (vma == lma), `$8000`-aligned, placed in `map.toml` order
  right after `Sfx_33` and before `GameState_ObjectTest_Init`, so seam-2's predicted chain is untouched.
  1. `section_align.rs` DECLARED: `d("SongBank2_Head", 0x8000, Z80_BANK_WINDOW)`. The label is a placeholder until they
     write the module, so check whether anything refuses a row whose label no shape defines.
  2. `module_roots.rs` SHAPE_GATES: `("games.sonic4.song_bank2", ShapeGate::SoundOn)` (the hand list
     `ORDER-WHEN-SHAPE-GATES` would retire; a 22nd row is acceptable meanwhile).
  3. seam-2 stops owning the song tables: aeon makes `SongTable`/`SongPatchTable` ordinary linker-resolved label arrays;
     seam-2 drops the `SongTable` export requirement (`seam2.rs:1562`) and the three-way split, and keeps measuring the
     MT length for the `Sfx_33` prediction. Land a TOLERANT version first (accepts either), so their byte-identical step
     lands alone. Expected movers: `mt_bank_port`, `mt_dual_carrier`, `seam2_layout_derivation`.
  4. Confirm a refreeze will not pin `SongBank2_Head` (no frozen row; the walk places it by alignment).
  5. Then the stub-bank probe: a COPY of their tree, stub bank 2 after `Sfx_33`, with and without the anchor.
- Verified by this lane 2026-09-27 (source reading at `b1c9bb58`): their two corrections both HOLD. (i) A gap over
  `ANCHOR_GAP` (0x400) before a section makes it an inferred island that needs a declared `[[anchor]]`
  (`native.rs:3406`, `[map.undeclared-island]`), so shape (a) needs their anchor at 0xC0000 and no frozen row. (ii)
  seam-2 refuses an `mt_bank` without `SongTable` (`seam2.rs:1562`).
- **Hazard to put in the brief and to aeon: a gap-inferred anchor is content-dependent in both directions.** Their
  measured gaps are 9,277 B plain and 2,533 B debug. If bank-1 content grows to within 0x400 of 0xC0000, no island is
  inferred and the declared anchor fires `[map.anchor-absent]`; past 0xC0000 the bank jumps to 0xC8000 and fires too.
  Both are loud, never silent, but a routine song edit can break the build with an error about placement.
- `section_alignment_declared.rs::the_requirements_above_16_are_declared_for_the_anchored_sections` and the header of
  `section_align.rs` say every row above 16 is an anchored island. The new row is the first `$8000` section the
  walk ROUNDS rather than holds, so both texts need amending in the same parcel.

### PPB-1-DECODE-FACTOR-PIN

- state: **open**, scheduled with aeon when they want it  size: `S`  project: `-`  asked by: aeon, 2026-09-28
- aeon's PPB-1: inlining `Parallax_Update`'s `Decode_Factor_A`/`Decode_Factor_B` calls saves about 0.5k cycles a
  tick, but `crates/sigil-harness/src/contract_baseline.rs` pins those call sites (`Parallax_Update` @
  `Decode_Factor_A` :: d2 in the invariant list, the `_B` twin in `D1C_DEMO_EXTRA`). The ratchet is two-way BY
  DESIGN (module header: a vanished row can mean the closure narrowed and a live save now reads dead), so their
  inline is refused until the rows go. Not a tolerant-form fix: remove both rows in a landing paired with theirs,
  after confirming the rows vanish because the calls are gone, not because the analysis narrowed.
- Their notice the same day: aeon `d33fa8d0` adds 8 B inside `Parallax_State` (every later RAM address moves) and
  three cross-module RAM names (`Parallax_Shadow_Key_Config`, `_VS`, `_K`); relevant at our next reference-tree pin
  advance, not before (reference tree stays at `ec640bcf`).

### AS-PERF-FIX-PEAK

- state: **LANDED 2026-09-28 at merge `a1bfb6fb`** (tip `809b2a35`), with fixes 5 and 6  size: `S`  project: `SIGIL-AS-REPLACEMENT`  from: AS-PERF-MEASURE fix 3
- Peak RSS S1/S2/S3K 72->49, 110->94, 244->185 MB; wall ratios 0.842 / 0.835 / 0.867 (load 10.9 to 16.8). Record:
  the note's "After AS-PERF-FIX-PEAK" section. Left open there: `sigil emp` single-file link still lends sections;
  the front end's per-pass growth (S3K 141 -> 163 -> 190 MB) has no owner measurement.
- `run_asm` keeps the front end's whole module alive while layout builds a second copy of every section. Drop or
  consume it first. Estimated peak cut 15 to 30 percent (S3K 243 to about 190 MB). Low risk; byte identity on
  S1/S2/S3K and aeon is the bar. Then fixes 5 and 6 (substitution fast path, builtin-scan skip), both small.

### AS-PERF-FIX-PASSES

- state: **LANDED 2026-09-28 at merge `b7d80a2b`** (tip `9664f887`; review record `8772017f`), fixes 1 and 2; fix 4
  (macro block structure once) NOT done, still open  size: `M`  project: `SIGIL-AS-REPLACEMENT`
- Two passes on S1/S2/S3K like asl; ifdef now positional as in asl. An adversarial review broke the first version
  (silent wrong bytes); the rework passed a second round. Record: the perf note's "Passes" section and
  `docs/superpowers/notes/2026-09-28-passes-adversarial-review.md`. Pre-existing sigil-vs-asl differences it lists
  (abs.w stand-in, forward values accepted in if/rept/org, case-insensitive ifdef, string equates) are unbooked.
- (1) converge when no value a pass READ changed (S2/S3K third/fourth passes follow only ADDED names, each assigned
  before read); needs a complete read log including `defined`/`ifdef`; about 25 to 30 percent. (2) why pass 1 lays
  out differently (S1 first move `EEgg_Wait`, S2 `paddingSoFar`); investigate first, size S. (4) macro block
  structure computed once. High risk: a pass stopped early is a silent wrong ROM, so every step needs byte identity
  on all three corpora and aeon plus a planted control that an early stop is caught.

### SOUND-FOLD-GATE-SILENT-SKIP

- state: **LANDED 2026-09-28 at merge `a8b1d9a5`** (tip `e5f2a458`); record
  `docs/superpowers/notes/2026-09-28-sound-fold-gate-silent-skip.md`. Open from it: a head label defined in two
  sections takes the first match, not refused.  size: `S`  project: `-`  found: SONG-BANK-2-EMBED-ROOT review, 2026-09-28 (agent's reading, not
  re-verified here)
- `validate_sound_fold` (`crates/sigil-harness/src/native.rs`, about line 3307) silently skips a label it cannot
  find. If `Song_MovingTrucks` is renamed or stops being its section's first item, the check that the placed MT bank
  equals seam-2's prediction goes quiet with nothing red. After aeon's song-bank-2 change the fold gate is one of
  the four checks standing in for the deleted co-residency ensures, so a vacuous pass there matters more. Stage 0:
  confirm the skip, then make a missing expected label loud (a planted rename must go red).

### SONG-BANK-2-EMBED-ROOT

- state: **LANDED 2026-09-28 at merge `116aa23e`** (tip `6b878e3e`; onto master as `5c483786`)  size: `S`  asked by: aeon
- seam-2 lowers `mt_bank.emp` with its embeds relative to the aeon root or the sound dir; ambiguous, mixed and
  computed paths refuse loudly. Record: `docs/superpowers/notes/2026-09-28-song-bank-2-embed-root.md`. Aeon owes
  its own pytest audits a respelling (`tools/test_smps_import.py`, three in `tools/test_song_packer.py` look for
  the literal `embed("song_*.bin")`).

### PAIR-REBUILD-AFTER-AEON-STEP1

- state: **DONE 2026-09-28T11:51:59Z**, pair `1173bb31`, after aeon `cd262846`; pre-checked on their tree  size: `S`  owed to: aeon, 2026-09-28
- Rebuild the shared pair at our then-current master once aeon's step 1 (native song tables, root-relative embeds,
  ensures deleted) lands, so they build under the loud fold gate (`a8b1d9a5`). Held back on purpose so the stricter
  gate did not land mid-step. Their shape: `Song_MovingTrucks` first in `mt_bank`; the tables at the tail of
  `sfx_bank_blob` after `Sfx_33`; neither head renamed. If the gate then refuses their tree, it is a finding on
  their side. Method: the pair paragraph in `docs/OVERSEER-REFERENCE.md` (locked build tree, outgoing copy kept,
  /proc scan, stage and `mv`).

### SONG-BANK-2-OVERLAY-UNFROZEN-ANCHOR

- state: **LANDED 2026-09-28 at merge `bced52d4`** (tip `468f557b`)  size: `S`  asked by: aeon
- A clip overlay may move an anchor that holds no frozen row (bank 2 is walk-placed); the overlaid map is what
  `validate_placement` checks the placed head against, so a wrong overlay address refuses
  `[map.undeclared-island]`. Record: `docs/superpowers/notes/2026-09-28-overlay-unfrozen-anchor.md`.
