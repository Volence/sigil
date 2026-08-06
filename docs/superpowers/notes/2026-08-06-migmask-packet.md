# migmask packet — the MigrateMasks stride fix

Lane `migmask`. Branches: sigil `migmask` off master `f4d87aae`, aeon `migmask`
off master `077dc7d` (both verified as the tips before branching). Worktrees
created and seeded by the porter. Sigil master has since moved to `8dce8ef6`
(docs only, an unrelated write-up); the `#[test]` baseline below is re-derived
against `8dce8ef6`, counted this session.

**No merge-state claims appear in this packet.** Nothing was merged or pushed.

**This packet has been through a three-lens panel and a fixup round.** The panel
found the shipped code change SOUND — one lens decoded it out of the golden
bytes, verified register liveness instruction by instruction, and constructed and
refuted two independent failure hypotheses; another reproduced all 28 golden
numbers independently. Almost nothing in the fixup touches the fix. What it
touches is the **A/B specification** (rebuilt — it could not have been executed
as written, and two of its three defects would have produced a wrong conclusion),
one comment, one test-coverage gap, and a set of record errors. Every correction
below is marked where it changed a previously-recorded number or claim, rather
than silently overwritten.

## What changed

One proc, one file. `engine/objects/entity_window.emp`,
`EntityWindow_MigrateMasks`: the hand-rolled ×22 shift chain that indexed
`Entity_Scan_State` is replaced by

```
        move.w  d3, d0
        mul_const.w d0, #sizeof(EntityScanState), d1     // d0 = entry × sizeof(EntityScanState); d1 trashed
```

The comment names the derivation, not the number. This proc has broken TWICE on
a hand-carried size, and both times a stale size written next to the code was the
tell; a comment that restates `$1A` re-creates exactly the artefact the fix
exists to retire.

and the KNOWN-BUG comment block is replaced by a present-tense contract comment.
Nothing else in the proc, the file, or either repo's engine code was touched.

Register choice checked against the proc's real contract: it declares
`clobbers(d0-d3, a0-a1)`; d3 is the live loop counter and is not touched; the
existing leading `move.w d3, d0` already serves as the construct's preload; d1
is declared, and is dead from here to its reload at `.match` (verified by reading
every instruction between). `d0.w` is the only half consumed
(`move.b (a0, d0.w), d2`), so the `.w` contract's undefined upper word is safe.

## The `sizeof` question — ANSWERED YES

**`mul_const.w d0, #sizeof(EntityScanState), d1` is accepted.** This was a
measured-unknown handed to the lane, and it was measured, not predicted: the
corpus builds. The mechanism is ordinary — the immediate operand mapper
evaluates the expression and folds it through `Value::as_stored_int` into
`CodeOperand::Imm` (`crates/sigil-frontend-emp/src/eval/asm.rs:1748-1749`), so
any comptime integer expression reaches the multiplier position.

The fallback (`#26` plus a retained `ensure`) was therefore **not** taken, and
no `ensure` was added: the multiplier now derives from the struct, so there is
nothing left for a guard to catch at this site.

The lowering chosen for 26 — a 6-instruction LTR chain, 12 B / 38 cy, taken over
`mulu.w #26,d0` (4 B / **48 cy**) on the cost model — is 2 B and 6 cy cheaper
than the 7-instruction hand chain it replaced. It is also, by construction, the
same lowering the literal `#26` would produce.

**The rejected candidate's price, re-derived against the table this session:**
`sigil-isa/src/m68k_cycles.rs:303-305` charges a KNOWN-IMMEDIATE `mulu.w` exactly
`38 + 2·ones(n) + ea_time(Imm)`; `ones(26) = 3` (26 = `%11010`) and the word
immediate fetch is 4, so `38 + 6 + 4` = **48 cycles**. 70 is the *ceiling for the
register-source form* (`at_most(70 + ea)`, the row immediately below), and citing
it here overstated the margin by 22 cycles. The decision is unaffected — 38 < 48
either way — but the number was written as a measurement and is corrected
everywhere it appeared.

Also measured this session, from the lowering oracle's own failure output: the
scratch is what buys the chain. Without a scratch register the `.w` candidate set
for 26 collapses and the construct picks `mulu.w #26,d0`; the 6-instruction LTR
chain is only available because this site can grant `d1`.

**Pinned, both polarities plus non-vacuity:**
`crates/sigil-frontend-emp/tests/mul_lowering.rs::a_sizeof_multiplier_derives_from_the_struct`
asserts (i) `sizeof` of a 26-byte struct == `#26`, (ii) `sizeof` of the same
struct shrunk to 22 bytes == `#22`, and (iii) the two differ. Assertion (iii) is
the one that matters: without it, a spelling that compiled and silently ignored
the struct would pass. This is the only test added; it accounts for the whole
`#[test]` count delta below.

## The sweep (the 07-22 bar, applied by the porter)

Run over the whole module — every `lsl`/`lsr`/`asl`/`asr`/`mul*` — not only the
site named in the brief. **Re-counted line by line in the fixup round against the
POST-fix file; the first pass's enumeration was incomplete and two buckets were
wrong.** The conclusion held (two further independent re-censuses agree the ×22
chain was the module's only unguarded restated STRUCT stride) but the evidence
first offered for it did not enumerate the module, so it could not have
established that conclusion. Stated plainly rather than quietly repaired.

The complete census — 23 shift/multiply sites, every one placed:

* **The ×22 chain — fixed.** The only unguarded restated struct stride.
* **Six `lsl.w #5`** (`:545`, `:561`, `:579`, `:641`, `:1642`, `:1646`) restate
  `log2(ENTITY_LOADED_SLOT_SIZE)`. Same restatement class, covered by a live
  tripwire: `ensure(ENTITY_LOADED_SLOT_SIZE == 32, …)` at `:93`. **Ledgered, not
  ridden in.**
* **Seven `lsr.w #3`** (`:188`, `:213`, `:235`, `:264`, `:550`, `:566`, `:584`)
  — bit-index → byte-offset. Not a stride. (First pass said "×4"; it is seven.)
* **Two `lsl.w #2`** (`:1071` 4-byte ring ROM entries, `:1236` 4-byte longword
  type-table entries) — ROM-format strides, documented at both sites, derived
  from no struct.
* **One `lsl.w #8`** (`:1015`) — byte-to-high-half placement. Not a stride.
* **One `lsr.w #OEF_TYPE_SHIFT`** (`:1233`) — a named bitfield extract. Not a
  stride, and it names its shift.
* **Eight register shifts by `SECTION_SIZE_SHIFT`** (`:691`, `:697`, `:729`,
  `:731`, `:837`, `:840`, `:1693`, `:1696`) — world-px ↔ section-coordinate.
  Guarded: `ensure(SECTION_SIZE == 1 << SECTION_SIZE_SHIFT, …)` at `:97`.
* **One `lsr.w #1`** (`:748`) — **the site the first pass filed in no bucket at
  all.** It decodes an entry index into (col, row): `d0 = entry & 1` at `:745`,
  `d1 = entry >> 1` at `:748`. That pair is an unguarded restatement of
  `MAX_TRACKED_SECTIONS == 4` *being a 2×2 window*. The module's only guard on
  that constant is `ensure(MAX_TRACKED_SECTIONS <= 16, …)` at `:45`, written for
  an unrelated reason (a `btst` bit index), and it admits 6, 8, 9 and 16 — every
  one of which silently breaks the decode into garbage geometry. **Disposition:
  ledgered, not fixed here.** It is not a struct stride, so it is outside this
  parcel's class and outside its byte proof; and unlike the ×22 chain the correct
  spelling is not obvious (the window is 2×2 by the *envelope* argument in
  `DeriveWindow`'s header, not by arithmetic on `MAX_TRACKED_SECTIONS`). The
  cheap, honest repair is a second `ensure(MAX_TRACKED_SECTIONS == 4, …)` naming
  the 2×2 decode, and it belongs in the same parcel that revisits the window
  geometry.
* Cross-module: `Entity_Scan_State` has exactly one other consumer,
  `engine/ram.emp:37-39`, which already carries
  `ensure(extern("EntityScanState_len") == ENTITY_SCAN_STATE_LEN, …)`.

**So the sweep's answer is "one more struct-stride class instance, and it is
guarded — plus one unguarded restatement of a different constant."** The
language gap it exposed is that a shift count cannot derive from a size: there is
no `log2` builtin (`eval/builtins.rs` holds `map filter fold len find slice val
range array byte dense pad_to_cycles nop jr extern bankid winptr` and nothing
arithmetic), so the corpus pairs a hand-maintained `*_SHIFT` with every `*_SIZE`
— a second constant that can drift from the first. Ledgered with a kill
condition.

**Widened past the module** (fixup round): the one restated stride outside
`entity_window.emp` with no `ensure` in its own file is
`engine/level/plane_buffer.emp:244` (`mul_const.w d0, #80, d3`) — and its
neighbour `:76` (`#160`). `plane_buffer.emp` contains **zero** `ensure(` of any
kind. Both sites are already enumerated in the deferred stride-naming ledger row;
the file-scoped fact is the increment, and it is cross-referenced there rather
than duplicated. By contrast the two `sizeof(Sec)` = 66 multiplies
(`section.emp:151`, `tile_cache.emp:233`) — the closest analogue to the ×22 bug,
a struct size restated as a literal — ARE guarded, by
`ensure(sizeof(Sec) == 66, …)` in both files.

## Byte accounting — seven targets, region ladder RE-DERIVED

Baseline proven BEFORE any edit: a full seven-target `capture_goldens.sh` on the
freshly seeded aeon worktree reproduced every chain-50 CRC exactly (`s4
4f43c8d9/411167` · `s4.debug 159b152f/423571` · `demo fbf40075/91330` ·
`demo.debug 8c17cd39/94031` · `config_a c6e8cd87/423949` · `config_b
48ef0d0b/301205` · `lean 29d49dc6/379110`). The seed is therefore proven, not
assumed. Target list derived from `crates/sigil-harness/golden/` — seven blobs.

**Chain 50 → 51.** Every new golden diffed against its committed predecessor
this session; the ladder below is that diff, not an assumption.

| target | full CRC32 / size | anchor CRC32 / EndOfRom |
|---|---|---|
| `s4` | `84c33dfc` / 411167 | `0471cb2a` / `0x5d968` |
| `s4.debug` | `7e273b14` / 423571 | `6e6dcfec` / `0x5f758` |
| `demo` | `b3ceaac5` / 91330 | `d697f872` / `0x11224` |
| `demo.debug` | `f36cd2b4` / 94031 | `d88fac6c` / `0x11224` |
| `config_a` | `10b1cd9e` / 423949 | `19b3212e` / `0x5f758` |
| `config_b` | `30d4e243` / 301205 | `55385fc6` / `0x42d90` |
| `lean` | `7dcc7cef` / 379110 | `bcd81885` / `0x5c8e6` |

CRC32 + size throughout; no SHA1 anywhere.

**Every `full_size` and every `anchor_end` is UNCHANGED from chain 50.** The
2-byte shrink never left the module's placement slot.

Each shape's diff has the same four runs (lean has three — it carries no deb2
appendix by design). **The 222-vs-320 split in run 3 IS exactly the non-DEBUG /
DEBUG partition** — an earlier draft disclaimed that ("tracks the shapes carrying
extra DEBUG code, not the release/debug label"); the disclaimer was false and is
withdrawn. Four non-DEBUG shapes (`s4`, `demo`, `config_b`, `lean`) carry 222 B;
three DEBUG shapes (`s4.debug`, `demo.debug`, `config_a`) carry 320 B. Every
number below was re-derived this session by a byte-level diff of each chain-51
golden against its chain-50 predecessor.

| # | region | what it is |
|---|---|---|
| 1 | `$18E..$18F` (2 B) | ROM header checksum; the header-neutral anchor zeroes it |
| 2 | one byte — s4/lean `$3A75`, s4.debug `$46E9`, demo/config_b `$2309`, demo.debug `$2EF1`, config_a `$4707` | the `bsr.w` displacement of the single external call into `EntityWindow_Slide` (inside `EntityWindow_Scan`), which moved −2 |
| 3 | **222 B in the four non-DEBUG shapes** — s4, demo, config_b, lean (`$3E1E..$3EFB` in s4/lean) — and **320 B in the three DEBUG shapes** — s4.debug, demo.debug, config_a (`$4B20..$4C5F` in s4.debug) | the rewritten head, the module tail shifted −2, the intra-module branch displacements that re-tightened with it, and two extra pad bytes at the slot tail |
| 4 | **nine** moved label addresses in the four non-DEBUG shapes (33 B span; absent in lean, which has no appendix — so three shapes actually carry it) and **twelve** in the three DEBUG shapes (45 B span in s4.debug) | the deb2 appendix's intra-module label addresses, every one −2 (decoded longword by longword this session) |

**Where the 2 bytes went — the mechanism is a FIXED PLACEMENT SLOT, not
pre-existing slack.** The ledger first said the slot "had ≥2 bytes of tail
padding"; measured, that is true in some shapes and false in others, and it is
not the mechanism either way. Tail padding before → after, per shape:

| shape | OLD pad | NEW pad | next section head |
|---|---|---|---|
| `s4` / `lean` | 4 | 6 | `$003F00` (`PopulateSpawnedPieceCount`) |
| `s4.debug` | **0** | 2 | `$004C60` |
| `demo` | **0** | 2 | `$002790` |
| `demo.debug` | 8 | 10 | `$003470` |
| `config_a` | 2 | 4 | `$004C80` |
| `config_b` | **0** | 2 | `$002790` |

Three of the seven shapes — `s4.debug`, `demo`, `config_b` — had **zero** tail
padding beforehand, so no pre-existing slack absorbed anything there. In all
seven the pad grew by exactly 2 because the next section head sits at a fixed
org: the freed bytes BECOME padding rather than being consumed by it. That is
why nothing downstream moved.

The instruction-level delta, read out of a hexdump:

```
OLD $3E1E: e948 3203 e549 d041 3203 d241 d041          (14 B, 44 cy)
NEW $3E1E: 3200 e348 d041 e548 d041 e348               (12 B, 38 cy)
```

`2·((x·2 + x)·4 + x)` = `26x`. The absorbing boundary is the next section head,
`PopulateSpawnedPieceCount` (`$3F00` plain / `$4C60` debug).

**Ripple — THREE sites, and the hand-edited two were empty.** `engine.inc` and
`mixed_dac_rom.rs` confirmed deleted from both repos (`git ls-files`, no hits).
`repin` reported `pins.rs unchanged`; `crates/sigil-harness/tests/repin_pins.rs`
needed no edit (no pinned symbol moved); `repin.toml` needed none (no region
added). The seven `golden/offcanonical_sizes/*.txt` tables changed **only** in
their two CRC header lines — not one of the 62/74/41/43/78/60/62 boundary labels
moved. Negative sweep run over `*.rs` / `*.toml` / `*.sh` for all fourteen
chain-50 CRCs (outside the append-only `provenance.toml` history) and for
hardcoded addresses in the moved windows: **zero hits.**

## Gates — RE-RUN IN FULL IN THE FIXUP ROUND (all own-run, all foreground)

Every number below was produced this session, after the fixup edits, against
sigil master `8dce8ef6` and aeon master `077dc7d`.

1. **Byte bar, seven targets — UNCHANGED at chain 51.** Target list derived from
   `crates/sigil-harness/golden/` (seven blobs). A full `capture_goldens.sh` run
   with `SIGIL_EMIT` and `SIGIL_BUILD` exported reproduced every chain-51 CRC
   exactly, and a second independent run reproduced them again:
   `s4 84c33dfc/411167` · `s4.debug 7e273b14/423571` · `demo b3ceaac5/91330` ·
   `demo.debug f36cd2b4/94031` · `config_a 10b1cd9e/423949` ·
   `config_b 30d4e243/301205` · `lean 7dcc7cef/379110`. **Not one byte moved** —
   the only aeon-side fixup edit is a comment.
   * The chain-50 comparands were re-derived this session too, from the goldens
     at master `f4d87aae`: `s4 4f43c8d9` · `s4.debug 159b152f` ·
     `demo fbf40075` · `demo.debug 8c17cd39` · `config_a c6e8cd87` ·
     `config_b 48ef0d0b` · `lean 29d49dc6`.
   * Aeon worktree ROMs left rebuilt from the branch source by the capture's
     restore step, before the strict run (the `collision_lookup_port.rs`
     on-disk-reference trap).
2. **Full strict**, foreground, streams SEPARATED (`> out 2> err`), never
   `2>&1`: `SIGIL_STRICT_GATE=1 SIGIL_EMIT=… AEON_DIR=<this lane's aeon
   worktree> cargo test --workspace --release --no-fail-fast`, exit 0.
   **3511 passed / 0 failed / 4 ignored.** Failures-first grep over both streams
   for `FAILED` / `panicked` / `^failures:` / `error[`: **no hits.**
   * Closing arithmetic: `3511 + 4 = 3515` == this branch's `#[test]` total
     counted today (`git grep -c '^\s*#\[test\]' -- 'crates/**/*.rs'` = 3515).
   * Baseline named and re-derived: sigil master **`8dce8ef6`** counted today =
     3514. `3514 + 1` (the single test this parcel added) `= 3515`. Closes
     exactly. The fixup round added no test — it widened an existing test's
     sample set — so the total is unchanged from the pre-fixup run.
   * Corruption cross-check (the mul2 trap): sum of `running N tests` headers =
     3515; sum of `test result:` lines = 3515. Two independent totals agree, so
     the capture is not spliced.
3. **`refreeze --check`: OK (tip `migmask`, chain len 51).** `repin`:
   **`pins.rs unchanged`.**
4. **Warn tiers.** Identical lint-id SET across all seven shapes:
   `module.path-mismatch 9`, `proc.out-unwritten 3`, `proc.clobber-undeclared 1`,
   `proc.undeclared-fallthrough` 6 in s4, demo and config_b (19 total) / 5 in
   s4.debug, demo.debug, config_a and lean (18 total). Identical before and
   after the fixup. **No deliberate delta is claimed, and none occurred.**
5. **Clippy** `--workspace --all-targets --release -- -D warnings`: exit 0. The
   19 warning lines in the stderr stream are all `sigil-clownlzss-sys` vendored
   C++ build-script output (`enigma.h` `-Wmaybe-uninitialized`), pre-existing and
   not Rust findings.
6. **Negative probes / non-vacuity.**
   * The parcel's one added test (`a_sizeof_multiplier_derives_from_the_struct`)
     carries both polarities plus the non-vacuity guard, as above.
   * The fixup round's coverage addition — `26` into `mul_lower.rs`'s `NS`
     sample set — was proven **non-vacuous by a mutation probe**: temporarily
     asserting `×27` for `n == 26` in
     `word_lowering_matches_low_word_and_leaves_upper_free` made that test FAIL
     (`n=26 x=0x1 scratch=false shape=["mulu.w #26,d0"]`, left 26 / right 27);
     the probe was reverted and the test passes. That failure output is also
     where the scratch-free-picks-`mulu` fact above was measured.

## Behavioural evidence — the bar is INVERTED here

This parcel must EXHIBIT a difference, so no identity check counts as evidence.
The porter did not run the emulator (overseer-only). The A/B is **specified**, in
executable detail, at `docs/superpowers/notes/2026-08-06-migmask-ab.md`, and
referenced as this chain entry's `ab`. **The note was REBUILT in the fixup round
— a lens panel found three defects that would each have wasted a live run, two of
which would have produced a WRONG conclusion.** Since the overseer executes it,
the note, not this packet, is the primary artefact. In summary:

* **The drive is the anchored replay stream, not `Debug_Scene_Freeze`.** The
  freeze REMOVES the proc under test: `ojz_scroll_test.emp:256-263` gates
  `jbsr EntityWindow_Scan` behind it, and `Scan` is the only caller of `Slide`,
  which is the only caller of `MigrateMasks`. A freeze-driven run reports "no
  difference" while executing nothing. Replaced with the standing input-replay
  net (`Input_Source = INPUT_PLAYBACK`, `Replay_Ptr = Replay_OJZ_Fixture + 20`),
  poked at a persistent breakpoint on `GameState_OJZScroll_Init` **before**
  `reload_rom` — the anchor aeon `b014865` requires, cited verbatim in the note,
  with the companion cells (`Replay_Hold`/`Replay_Prev`/`Replay_Done`/
  `Replay_Exit_Request`, and DEBUG `Replay_Check_Idx`) named as init-value
  preconditions. One drive, all three probes, DEBUG cart.
* **Probe 1, the mechanism, guaranteed to differ.** Break at the id-read
  instruction (debug `$4B32` OLD / `$4B30` NEW), hit 4× per slide before any
  `SEC_VOID` skip. `d0.w` reads `$0000/$0016/$002C/$0042` OLD and
  `$0000/$001A/$0034/$004E` NEW.
* **Probe 2, the consequence, controlled — the primary evidence.** Break at
  `EntityWindow_MigrateMasks` entry, `$4B1C` on both carts (`EntityWindow_Slide`
  moved and is NOT a valid shared anchor). Three corrections from the panel:
  * **A content precondition, or the probe can pass while measuring nothing.**
    Entry 0 is `0 × stride` and is correct at ×22 AND ×26, so a slide whose only
    mask-carrying survivor lands there is byte-identical on both carts. The probe
    now requires, at the anchor, some entry k in 1..3 that is non-void, whose id
    appears in the snapshot ids, and whose matching snapshot mask block is
    non-zero. Failing that, the finding is "no qualifying slide found" — never
    "no difference".
  * **The control binds ONE slide.** Requiring byte-identical proc inputs and
    "run several consecutive slides" cannot both hold: once slide N writes
    different masks, slide N+1's inputs differ BY CONSTRUCTION, and that
    divergence IS the parcel working. The earlier draft listed exactly that
    observation under "what would refute the parcel", so a correct run read as a
    refutation on the second slide. The control now binds only the first
    qualifying slide; later slides need independent runs from a common state.
    `a4` (the snapshot base, an argument) is now read and required equal too.
  * **The exhibit is deterministic by slide DIRECTION.** Entries are assigned
    absolutely from the anchor (`BuildEntries:742-749`), so survivors land at
    {0,2} rightward, {0,1} downward, {1,3} leftward, {2,3} upward. Destination 0
    is correct even under the bug ⇒ right/down show **one** differing slot,
    left/up show **two**. The ROM boots scrolling right, so the default run shows
    the weaker exhibit; the note tells the executor to take a left/up slide if
    the stream offers one, and gives a deterministic camera poke at
    `EntityWindow_Scan` (`$46A8`, same address on both carts) to force one
    otherwise — flagged as poisoning probe 3, because `Camera_X` is inside
    `Replay_Hash`.
* **The consequence model is rewritten — three directions, and the reachable
  second one was missing.** Because entries are absolutely anchored and
  `MigrateMasks` runs only when the anchor MOVED, **no valid entry can keep its
  section across the call**; the old bucket (b) ("entry kept its section, foreign
  mask copied over a correct one") is satisfiable only by `SEC_VOID`↔`SEC_VOID`,
  where it is harmless. The reachable second mode is worse: a garbage id
  chance-matches a snapshot id, so a foreign NON-ZERO mask lands on an entry whose
  section is genuinely NEW and already correctly zeroed — those entities are
  marked already-loaded and **NEVER SPAWN**. That is missing rings and objects,
  not duplicates, and it is probabilistic, so its absence refutes nothing. A third
  direction is now named too: the `cmpi.b #SEC_VOID` guard tests the GARBAGE byte,
  so a genuinely void entry is not skipped and can receive a foreign mask —
  benign, but it fits neither bucket and the executor must be able to place it.
* **Probe 3, the shipped replay fixture.** See the finding below. Clean,
  un-poked run; DEBUG cart (`replay.emp` compares only under `DEBUG == 1`).

Cycle-changing-parcel hygiene is stated in the note: a `pause`-anchored A/B is
**invalid** here (the two runs go out of phase after the first slide); every
probe anchors on a PC breakpoint, and the cart is identified by ROM hash, not
by the reload diagnostic. The note states what would refute the parcel **and** a
separate list of what would NOT — every item on that second list was a defect in
the first draft.

**One address correction of fact.** The first draft said the fixture "sits after
the code that moved" and told the executor to re-read it from the OLD listing.
It did not move: the 340-byte blob at `$5E568` in the NEW debug ROM is found at
exactly `$5E568` in the OLD debug ROM, and at `$5C778` in both plain ROMs
(verified by locating the blob in the chain-50 goldens this session). The claim
also contradicted this parcel's own relocation-free headline. Separately, the
draft's RAM table listed the DEBUG `Camera_X`/`Camera_Y` addresses
(`$FFFFA152`/`$FFFFA156`) in the *plain* column and said debug was "same"; the
plain shape's are `$FFFFA12E`/`$FFFFA132`. Both corrected.

## Per-pass findings

### Pass 1 (byte gate as verifier only)

Not applicable in the usual direction — this parcel is deliberately
byte-changing, and the byte gate's role here was to prove the **seed** (baseline
reproduced pre-edit) and then to bound the change (the four-run ladder above).
The gate is structurally blind to the defect itself: both the golden and the
fresh build agreed on the wrong stride, which is the whole reason the bug
survived nine days.

### Pass 2 / step 3 — retrospect, language asks

* **`sizeof` in an immediate works, and the corpus never knew.** Closed as a
  measurement with a pin. The mechanism generalises to any comptime integer
  expression, which is a stronger fact than the lane needed; the stride-naming
  row's `#(TILE_CACHE_STRIDE*2)` sub-question is *implied* by it but is
  deliberately recorded as still-unmeasured, because implied is not measured.
* **There is no `log2`, so a shift count cannot derive from a size.** The
  corpus's workaround is a hand-maintained `*_SHIFT` beside a `*_SIZE`, a second
  constant free to drift from the first. **The exemplar list was re-checked in
  the fixup round and only one member survived:**
  * `BLOCK_TILE_SHIFT` = 4 / `BLOCK_TILE_SIZE` = 16
    (`engine/system/constants.emp:440-441`) — a genuine pair, and **unguarded**:
    no `ensure` anywhere relates the two, and `BLOCK_TILE_SHIFT` has 10 shift
    sites in `tile_cache.emp` (`:56`, `:59`, `:511`, `:515`, `:519`, `:523`,
    `:534`, `:537`, `:553`, `:555`). This is the row's real find.
  * `SECTION_SIZE_SHIFT` **is** guarded —
    `ensure(SECTION_SIZE == 1 << SECTION_SIZE_SHIFT, …)` at
    `entity_window.emp:97`, plus a range `ensure` at `camera.emp:19`. Not a gap.
  * `PARALLAX_LERP_SHIFT` = 4 is a lerp *rate*. There is no
    `PARALLAX_LERP_SIZE` and there could not be — **not a member**, dropped.
  * `TILE_BYTES_SHIFT` = 5 (`bg_anim.emp:76`) has no `*_SIZE` partner either.
    That is a *different* defect — a shift with no size at all, where the size
    (32 bytes per 8×8 tile) exists only in a comment — and is re-filed as such.

  Ledgered with a kill condition. This is the same disease as the ×22 chain, one
  instrument short of the same cure.
* **A `mul_const` is the wrong instrument for a power of two.** 12 B / 38 cy
  against a 2-byte `lsl`. Worth stating because "adopt the construct everywhere"
  is the obvious wrong generalisation from this parcel.

### Pass 2 / step 5 — engine optimize

* The fix is incidentally 2 B and 6 cy/iteration cheaper than the code it
  replaces (24 cy per slide, cold path). This was not the goal and is not
  claimed as a win; it is reported so the cycle delta in the A/B is expected
  rather than surprising.
* No other optimization was taken. The six `lsl.w #5` sites were deliberately
  left alone — ledgered, not ridden in, per the brief and per the mul2 fixup
  panel's three rider criteria (this change was not in the brief and was not in
  the ledger before today, so it does not ride this byte proof).

### Neither bucket — the headline

**The shipped OJZ replay fixture was recorded from a build that carried the
bug.** `games/sonic4/data/replays/ojz_fixture.bin` carries 33 curated state
checkpoints over 2059 ticks and desyncs loudly in DEBUG. Its last
content-changing commit is aeon `806a0de` (2026-08-05) — **after** `4ad9f9b`
(2026-07-31) introduced the stride bug.

**What the dating proves, precisely:** the bug was PRESENT in the recording
build. It does **not** prove the bug ever FIRED during those 2059 ticks — that
needs a section-boundary crossing with entities collected in a surviving section,
and whether the stream contains one is exactly what probe 3 measures. An earlier
draft of this packet wrote the stronger claim ("i.e. with duplicate spawns and/or
foreign-mask copies already inside the hashed state"); the ledger row was hedged
correctly and the prose was not, so the prose is now aligned to the ledger.

**What `Replay_Hash` covers** (read out of `engine/system/replay.emp:267-334`
this session), because it bounds how loud a desync can be: `Logic_Tick`;
`Player_1`'s **address-free SST spans only** (motion `$02`, display `$14`,
status/anim `$1E`, entity `$2A`, `sst_custom` `$30`, plus four word folds);
`Camera_X`/`Camera_Y`; the section-streaming cells `Section_Top_Row_Written`,
`Section_Right_Col_Written`, `Section_Fwd_Neighbor_Data`,
`Section_Bwd_Neighbor_Data` and `Section_Plane_Dirty`; and three object-system
counters, `Dynamic_Live_Count`, `Dynamic_Free_SP`, `Effect_Free_SP`. **It does
NOT cover the SSTs of spawned entities.** A duplicate spawn or a never-spawned
ring reaches the hash only through those three counters (and, once the player
touches the changed world, through `Player_1`) — so a desync is a real signal,
but a difference that changes which entities exist without moving a counter at a
checkpoint tick can hide from it. "SST (object) state", the earlier draft's
phrase, overstated the coverage.

Nothing the porter can run detects this: the fixture is played only on the
emulator, and no consumer of it exists anywhere under `crates/*/tests` or
`crates/*/src`. Probe 3 of the A/B specifies the run with its own non-vacuity
control (OLD must first reach `Replay_Done = $FF` with zero desyncs, which is
what `b014865` recorded). A desync on NEW is the sharpest available exhibit that
shipped behaviour changed **and** means the fixture must be re-recorded on the
fixed ROM before it can serve as a regression net again; a clean run on NEW
instead bounds the fix's blast radius. Either way the general defect is worth
carrying: **a recorded regression net captured from a buggy build silently
promotes the bug to the spec, and re-recording is the only repair.**

Second, smaller headline: **a byte-changing edit was relocation-free.** Zero
boundary labels moved in any of seven shapes, `pins.rs` was unchanged, and the
two hand-edited ripple sites were empty. The campaign's default expectation for
a size-changing parcel is a relocation ladder plus a pins ripple; here the
correct answer was neither, and the proof is the seven size tables rather than an
assertion.

## Ledger rows — 4 added by the parcel, 2 by the fixup, 4 corrected

All in `docs/superpowers/notes/campaign-gap-ledger.md`.

Added by the parcel:

1. `sizeof(...)` accepted as a `mul_const` immediate — CLOSED for the `sizeof`
   half of the stride-naming row's open questions; the expression half stays
   open. **Corrected in the fixup:** the rejected `mulu.w #26,d0` prices at
   **48 cy**, not "~70"; and the scratch is what makes the chain available.
2. The relocation-free measurement (seven shapes, zero boundary labels).
   **Corrected in the fixup:** four non-DEBUG / three DEBUG (not "three
   release / three debug", which accounts for 6 of 7); nine moved appendix
   labels in the non-DEBUG shapes and **twelve** in the DEBUG shapes; and the
   mechanism is a fixed placement slot, not pre-existing slack — three shapes
   had ZERO tail padding beforehand.
3. The replay fixture was recorded from a build carrying the bug — OPEN, with a
   kill condition.
4. The 07-22 bar sweep result over the module, and the missing `log2` — OPEN,
   with a kill condition. **Corrected in the fixup:** the module census is now
   complete (23 sites, `lsr.w #3` is ×7 not ×4, and `:748`'s `lsr.w #1` gets a
   disposition); and the `log2` row's exemplar list is down to its one genuine
   member, `BLOCK_TILE_SHIFT`/`BLOCK_TILE_SIZE`.

Added by the fixup round:

5. `plane_buffer.emp` has zero `ensure(` and two literal stride multiplies —
   RECORDED as a **cross-reference** to the deferred stride-naming row, which
   already enumerates both sites and already records that six of its eight are
   unguarded. The increment is the file scope, not a new site count.
6. An A/B SPECIFICATION needs the same adversarial review as code — the three
   design defects found here, and why an inverted-bar A/B has no "same = pass"
   fallback to catch a broken design. RECORDED, with a bar.

The mul2 row that recorded the defect is left **OPEN**: its kill condition is
"a behaviour parcel corrects the stride … with its own A/B", and the A/B is
specified but not executed. Flipping it is not the porter's call. A clause was
added so its closing sentence is not false on this branch — the KNOWN-BUG marker
it says "is already in the tree" is gone here, replaced by the contract comment.

`docs/superpowers/notes/emp-idioms.md`, `mul_const` entry, two touches:
the multiplier-may-DERIVE bullet (the parcel's headline language finding, which
had landed only in the gap ledger), and a rewrite of the stale "When NOT to use
them" paragraph — it still said the two repeated-add loop sites awaited a
byte-changing parcel, and both were adopted at ltr-mul; neither `.gxy_mul` nor
`.mul_loop` exists in the corpus any more.

## Residue and things deliberately NOT done

* `crates/sigil-frontend-emp/src/branch_const.rs:12` and `src/type_slice.rs:9`
  cite "the MigrateMasks stride bug" as the exemplar of the
  silent-wrong-answer family. Those remain accurate as references to a ledgered
  past defect and were left alone; they are not markers at the code site.
* The six guarded `lsl.w #5` restatements — ledgered, untouched.
* `entity_window.emp:745/:748`, the unguarded 2×2 window decode — ledgered with
  a disposition, not fixed. Not a struct stride, so outside this parcel's class
  and its byte proof.
* `plane_buffer.emp`'s missing `ensure` — cross-referenced to the stride-naming
  row that owns it; not added here (it would be a rider on this byte proof, and
  it fails the three rider criteria).
* `BLOCK_TILE_SHIFT`/`BLOCK_TILE_SIZE`'s missing `ensure` — same reasoning,
  ledgered in the `log2` row.
* The `#(EXPR*2)` immediate question — ledgered as still unmeasured, untouched.
* The replay fixture — not re-recorded (needs the emulator).
* No merge. No push. No oracle/emulator tool used.
