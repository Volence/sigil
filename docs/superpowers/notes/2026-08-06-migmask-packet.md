# migmask packet — the MigrateMasks stride fix

Lane `migmask`. Branches: sigil `migmask` off master `f4d87aae`, aeon `migmask`
off master `077dc7d` (both verified as the tips before branching). Worktrees
created and seeded by the porter.

**No merge-state claims appear in this packet.** Nothing was merged or pushed.

## What changed

One proc, one file. `engine/objects/entity_window.emp`,
`EntityWindow_MigrateMasks`: the hand-rolled ×22 shift chain that indexed
`Entity_Scan_State` is replaced by

```
        move.w  d3, d0
        mul_const.w d0, #sizeof(EntityScanState), d1     // d0 = entry × $1A; d1 trashed
```

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
`mulu.w #26,d0` (4 B / ~70 cy) on the cost model — is 2 B and 6 cy cheaper than
the 7-instruction hand chain it replaced. It is also, by construction, the same
lowering the literal `#26` would produce.

**Pinned, both polarities plus non-vacuity:**
`crates/sigil-frontend-emp/tests/mul_lowering.rs::a_sizeof_multiplier_derives_from_the_struct`
asserts (i) `sizeof` of a 26-byte struct == `#26`, (ii) `sizeof` of the same
struct shrunk to 22 bytes == `#22`, and (iii) the two differ. Assertion (iii) is
the one that matters: without it, a spelling that compiled and silently ignored
the struct would pass. This is the only test added; it accounts for the whole
`#[test]` count delta below.

## The sweep (the 07-22 bar, applied by the porter)

Run over the whole module — every `lsl`/`lsr`/`asl`/`asr`/`mul*` — not only the
site named in the brief. Line numbers re-resolved against the POST-fix file.

* The ×22 chain — fixed. **It was the only unguarded restated stride.**
* Six `lsl.w #5` sites (`:545/:561/:579/:641/:1642/:1646`) restate
  `log2(ENTITY_LOADED_SLOT_SIZE)`. Same restatement class, but covered by a live
  tripwire: `ensure(ENTITY_LOADED_SLOT_SIZE == 32, …)` at `:93`. A size change
  fails the build loudly. **Ledgered, not ridden in.**
* `lsl.w #2` at `:1071` (4-byte ring ROM entries) and `:1236` (4-byte longword
  type-table entries) are ROM-format strides, documented at both sites, derived
  from no struct.
* `lsr.w #3` ×4 is bit-index→byte-offset; `lsl.w #8` is a byte-to-high-half
  placement. Neither is a stride.
* Cross-module: `Entity_Scan_State` has exactly one other consumer,
  `engine/ram.emp:37-39`, which already carries
  `ensure(extern("EntityScanState_len") == ENTITY_SCAN_STATE_LEN, …)`.

**So the sweep's answer is "one more class instance, and it is guarded."** The
real gap it exposed is that a shift count cannot derive from a size: there is no
`log2` builtin (`eval/builtins.rs` holds `map filter fold len find slice val
range array byte dense pad_to_cycles nop jr extern bankid winptr` and nothing
arithmetic), so the corpus pairs a hand-maintained `*_SHIFT` with every `*_SIZE`
— a second constant that can drift from the first. Ledgered with a kill
condition.

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
appendix by design). The 222-vs-320 split in run 3 tracks the shapes that carry
the extra DEBUG code inside the same module, not the release/debug label:

| # | region | what it is |
|---|---|---|
| 1 | `$18E..$18F` (2 B) | ROM header checksum; the header-neutral anchor zeroes it |
| 2 | one byte — s4/lean `$3A75`, s4.debug `$46E9`, demo/config_b `$2309`, demo.debug `$2EF1`, config_a `$4707` | the `bsr.w` displacement of the single external call into `EntityWindow_Slide`, which moved −2 |
| 3 | 222 B in four shapes — s4, demo, config_b, lean (`$3E1E..$3EFB` in s4/lean) — and 320 B in three — s4.debug, demo.debug, config_a (`$4B20..$4C5F` in s4.debug) | the rewritten head, the module tail shifted −2, the intra-module branch displacements that re-tightened with it, and two extra pad bytes at the slot tail |
| 4 | 33 B release / 45 B debug; absent in lean | the deb2 appendix's nine moved intra-module label addresses |

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

## Gates (all own-run this session)

1. **Byte bar, seven targets.** Baseline reproduced pre-edit; post-freeze, a
   SECOND independent `capture_goldens.sh` run reproduced all seven chain-51
   CRCs exactly. `SIGIL_EMIT` and `SIGIL_BUILD` both exported; script order
   untouched. Aeon worktree ROMs rebuilt from the new source before the strict
   run (the `collision_lookup_port.rs` on-disk-reference trap).
2. **Full strict**, foreground, streams SEPARATED (`> out 2> err`), no
   `2>&1`: `SIGIL_STRICT_GATE=1 AEON_DIR=<this lane's aeon worktree> cargo test
   --workspace --release --no-fail-fast`, exit 0. **3511 passed / 0 failed / 4
   ignored.** Failures-first grep over both streams: none.
   * Closing arithmetic: `3511 + 4 = 3515` == this branch's `#[test]` total
     counted today (`git grep -c '^\s*#\[test\]' -- 'crates/**/*.rs'` = 3515).
   * Baseline named and re-derived: master `f4d87aae` counted **today** = 3514.
     `3514 + 1` (the single test added by this parcel) `= 3515`. Closes exactly.
   * Corruption cross-check (the mul2 trap): sum of `running N tests` headers =
     3515; sum of `test result:` lines = 3515. The two independent totals agree,
     so the capture is not spliced.
3. **`refreeze --check`: OK (tip `migmask`, chain len 51).**
4. **Warn tiers.** Identical lint-id SET and identical counts across all seven
   shapes, before and after: `module.path-mismatch 9`, `proc.out-unwritten 3`,
   `proc.clobber-undeclared 1`, and `proc.undeclared-fallthrough` 6 in s4, demo and
   config_b (19 total) / 5 in s4.debug, demo.debug, config_a and lean (18 total).
   **No deliberate delta is claimed, and none occurred.**
5. **Clippy** `--workspace --all-targets --release -- -D warnings`: exit 0.
6. **Negative probes / non-vacuity**: the one new test carries both polarities
   and a non-vacuity guard (above). No other check was added.

## Behavioural evidence — the bar is INVERTED here

This parcel must EXHIBIT a difference, so no identity check counts as evidence.
The porter did not run the emulator (overseer-only). The A/B is **specified**, in
executable detail, at
`docs/superpowers/notes/2026-08-06-migmask-ab.md`, and referenced as this chain
entry's `ab`. In summary:

* **Probe 1, the mechanism, guaranteed to differ.** Break at the id-read
  instruction (plain `$3E30` OLD / `$3E2E` NEW; debug `$4B32` / `$4B30`), hit 4×
  per slide before any `SEC_VOID` skip. `d0.w` reads `$0000/$0016/$002C/$0042`
  OLD and `$0000/$001A/$0034/$004E` NEW.
* **Probe 2, the consequence, controlled — the primary evidence.** Break at
  `EntityWindow_MigrateMasks` entry, whose address is **the same in both ROMs**
  (`$3E1A` plain / `$4B1C` debug — chosen for exactly that reason;
  `EntityWindow_Slide` moved and is NOT a valid shared anchor). Dump the proc's
  entire input (`Entity_Scan_State` 104 B, `Entity_Mask_Scratch` 132 B,
  `Entity_Loaded_Masks` 128 B, `Entity_Window_Active`, `Entity_Window_Anchor`)
  and require it byte-identical across carts — that is the non-vacuity control,
  and a mismatch voids the comparison. Then `step_out` and dump
  `Entity_Loaded_Masks` again. Identical inputs, different outputs, same anchor
  ⇒ the difference is this proc and nothing else. Step 3 of the probe names
  which of the two directions fired at each slide: **(a)** an entry whose section
  changed reads all-zero in OLD (the compare-clear at `:637-647` ran, then the
  identity match failed on a garbage id → duplicate spawns) and correct in NEW;
  **(b)** an entry that kept its section holds, in OLD, a copy of a different
  entry's snapshot mask. Both directions are claimed; the A/B asks for at least
  one instance of each across several consecutive slides.
* **Probe 3, the shipped replay fixture.** See the finding below.

Cycle-changing-parcel hygiene is stated in the note: a `pause`-anchored A/B is
**invalid** here (the two runs go out of phase after the first slide); every
probe anchors on a PC breakpoint, and the cart is identified by ROM hash, not
by the reload diagnostic. The note also states what would REFUTE the parcel.

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
  corpus's workaround is a hand-maintained `*_SHIFT` beside every `*_SIZE`
  (`BLOCK_TILE_SHIFT`, `SECTION_SIZE_SHIFT`, `TILE_BYTES_SHIFT`,
  `PARALLAX_LERP_SHIFT`) — a second constant free to drift from the first,
  guarded only where someone remembered an `ensure`. Ledgered with a kill
  condition. This is the same disease as the ×22 chain, one instrument short of
  the same cure.
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

**The shipped OJZ replay fixture is a recording of the bug.**
`games/sonic4/data/replays/ojz_fixture.bin` carries 33 curated state checkpoints
over 2059 ticks, hashing SST (object) state, and desyncs loudly in DEBUG. Its
last content-changing commit is aeon `806a0de` (2026-08-05) — **after** `4ad9f9b`
(2026-07-31) introduced the stride bug. Every checkpoint hash in the shipped
regression net was therefore captured with `MigrateMasks` mis-indexing, i.e. with
duplicate spawns and/or foreign-mask copies already inside the hashed state.

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

## Ledger rows added (4)

All in `docs/superpowers/notes/campaign-gap-ledger.md`, same commits as the code:

1. `sizeof(...)` accepted as a `mul_const` immediate — CLOSED for the `sizeof`
   half of the stride-naming row's open questions; the expression half stays
   open.
2. The relocation-free measurement (seven shapes, zero boundary labels).
3. The replay fixture is a recording of the bug — OPEN, with a kill condition.
4. The 07-22 bar sweep result over the module, and the missing `log2` — OPEN,
   with a kill condition.

The mul2 row that recorded the defect is left **OPEN and untouched**: its kill
condition is "a behaviour parcel corrects the stride … with its own A/B", and the
A/B has been specified but not executed. Flipping it is not the porter's call.

## Residue and things deliberately NOT done

* `crates/sigil-frontend-emp/src/branch_const.rs:12` and `src/type_slice.rs:9`
  cite "the MigrateMasks stride bug" as the exemplar of the
  silent-wrong-answer family. Those remain accurate as references to a ledgered
  past defect and were left alone; they are not markers at the code site.
* The six guarded `lsl.w #5` restatements — ledgered, untouched.
* The `#(EXPR*2)` immediate question — ledgered as still unmeasured, untouched.
* The replay fixture — not re-recorded (needs the emulator).
* No merge. No push. No oracle/emulator tool used.
