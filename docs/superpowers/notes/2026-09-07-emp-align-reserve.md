# `.emp` `align` reserves by the shared asl rule (`sig-emp-align-fill`)

Branch `parcel/emp-align-reserve`, based on master `8faccab9`. Closes lens
finding `sig-emp-align-fill` (`docs/lens-findings.jsonl`, severity high, from
the 2026-09-06 sweep) and the ledger row `LENS-EMP-ALIGN-FILL`.

## The defect

`crates/sigil-frontend-emp/src/lower/mod.rs`'s `emit_align_pad` computed its
pad with a hand-rolled unsigned round-up and called `emit_fill(pad, 0, span)`.
The AS front-end's `align` directive (`sigil-frontend-as/src/eval.rs`
`directive_align`) moved to `sigil_ir::asl_align_pad` + `reserve` at
`84c48a7b` (2026-09-04) and touched only that side. `.emp`'s REGION `@align(n)`
(`lower/regions.rs` `align_to`) was already on the shared rule. The field path
was the last outlier in the workspace, on BOTH axes: the rule and the effect.

## Alignment-pad sites, enumerated by name

The population that computes how far an `align` advances a position **inside a
section** — all four now on one rule and one effect:

| Site | What it aligns | Rule | Effect |
|---|---|---|---|
| `sigil-ir/src/align.rs:40` `asl_align_pad` | *the rule itself* — 31 asl-measured rows | — | — |
| `sigil-frontend-as/src/eval.rs:6602` `directive_align` | AS `align n` | `asl_align_pad` | `reserve` |
| `sigil-frontend-emp/src/lower/regions.rs:565` `align_to` | `.emp` region `@align(n)` (RAM cursor) | `asl_align_pad` | `reserve` |
| `sigil-frontend-emp/src/lower/mod.rs:1031` `emit_align_pad` | **all three `.emp` forms** | `asl_align_pad` *(was a hand-rolled round-up)* | `reserve` *(was `emit_fill`)* |

**The finding's `where` line is narrower than the defect.** The lens row and
the brief both name this "the per-item `(align: N)` path". `emit_align_pad` is
ONE function with THREE callers, and the per-item form is the least-used of
them in the corpus:

- `lower/mod.rs:932` — an EXPLICIT top-level `align N` item. **This is the one
  the DAC banks use** (`games/sonic4/data/sound/dac_banks.emp:61`, `align $8000`).
- `lower/mod.rs:1354` — a per-DATA-item `(align: N)` (`lower_data_align`).
- `lower/mod.rs:1481` — a `table item_align:` pad, one after every emitted part
  (`emit_table_part`; the corpus use is `sfx_bank.emp`'s `SfxTable`).

Sites that compute an alignment but are NOT in-section pads, enumerated so the
set is closed rather than filtered:

- `sigil-link/src/relax.rs:196` — `base.next_multiple_of(n)`, the D7.2 bank
  bump on a section BASE. Deliberately a plain unsigned round-up on a map-level
  address: the invariant it enforces is no-straddle, and alignment is one
  strategy for it, not the rule being modelled. No asl equivalent.
- `sigil-harness/src/native.rs:2421` + `section_align.rs:247` `required_for` —
  `align_up(running, required_for(head_label))`, the packing walk's section-base
  alignment from the DECLARED table. A different layer; chain 196 replaced the
  inferred `packed_align_of` with it.
- `sigil-harness/src/native.rs:2975` `trim_trailing_align_overshoot` and
  `:3039` `recompute_bank_aligns` — post-hoc REWRITES of an already-baked pad
  when a section relocates. They match zero-`Fill` only; see below.
- `sigil-frontend-emp/src/layout.rs:1201` `check_struct_field_align` — computes
  `align - (offset % align)` for a struct field's `(align: N)`, but only to
  QUOTE it in a diagnostic. It asserts an offset, never inserts a pad.

Instrument note: the hand-rolled-round-up sweep
(`git grep -nE '\(\s*[a-z_0-9]+\s*-\s*\(?[a-z_0-9.]+\s*%'` over `crates/**/*.rs`)
was validated against the live instance — it returned `lower/mod.rs:1008`, the
defect, before the fix. Re-running it after the fix returns nothing, and that
emptiness only means something because the instrument had already been shown to
fire on this exact class.

## What changed

```rust
-    let pad = (n - (pos % n)) % n;
-    if pad > 0 { builder.emit_fill(pad, 0, span); }
+    let pad = sigil_ir::asl_align_pad(pos, n);
+    if pad > 0 { builder.reserve(pad, span); }
```

**The effect half** is where the bytes are. asl's object file carries no record
for the addresses an `align` steps over. Mid-image that is indistinguishable
from `$00` fill — `Section::image_bytes` skips a `Reserve` with the write cursor
and the next datum's `resize(end, 0)` zeroes the gap — so the two part only at a
section's TAIL, where a `Fill` lengthens the image by up to `n-1` bytes asl
never emits.

**The rule half is INERT TODAY, and is recorded as inert rather than as
impossible.** `emit_align_pad`'s position is `origin + builder.current_offset()`,
and `origin` is `vma.unwrap_or(next_lma)` (`lower/mod.rs:507`). `next_lma` starts
at 0 per module and accumulates image lengths, so it is a small non-negative
number; the only way the position goes negative as `i32` is an explicit
`section (vma: $FFFF….)` carrying an align. **All 20 `section` declarations in
aeon `ec640bcf` were enumerated: the largest `vma:` is `$8357`
(`movingtrucks_pitchtable.emp`), and no section declares a RAM vma at all.**

That enumeration is backed by an instrument that does not depend on it:
`/usr/bin/grep -rnE 'vma:\s*\$F' --include='*.emp'` over the WHOLE aeon tree
returns nothing, and the same pattern fires on a planted
`section canary (cpu: m68000, vma: $FFFF8000)` — so the emptiness is a finding
rather than a grep that could not have returned anything. (`grep -r` is a shell
function here that skips gitignored paths; the absolute path is the reason the
generated `.emp` under `games/sonic4/data/generated/` is in scope.) So
the unsigned round-up and the signed asl rule agree on every byte the corpus
reaches, and unifying them was byte-neutral by construction — which is what the
brief's "STOP if it would move a field" condition asked to be measured.

Two things about that measurement worth keeping:

1. The position `emit_align_pad` aligns is a LOWERING BASELINE, not the final
   address (the link-time congruence assert backstops the final one). So even a
   `.emp` section eventually PLACED in RAM would not reach the negative regime
   unless it declared the RAM address as its `vma:`. The signed rule becomes
   live on a narrower trigger than "a RAM section", and the trigger is a source
   construct, which is why the new vector states it as one.
2. This is a survey of aeon `ec640bcf`, not a property of the language. A
   `section (vma: $FFFF8000) { align 256 }` is legal `.emp` today and would have
   silently picked the wrong rule before this change.

## Hypothesis 3 — the DAC coupling. REFUTED as stated, and the reason matters.

The sweep says: *"the DAC intra-bank recompute only still fires because `.emp`
align emits `Fill`, so fixing this alone lands the drum bank off its `$8000`
boundary."* The ledger row repeats it as a **do-not-land-alone**. Measured:

**The drum bank does not move.** Both arms place `Dac_Temp_Blip` at `$A8000`
and `Dac_SharedBank_Start` / `Dac_Kick` at `$B0000` — one `$8000` window apart,
both bank-aligned — read off `s4.lst` in each arm. All four listings are
byte-identical between the arms, so nothing else moved either.

The mechanism, re-derived rather than taken:

- `recompute_bank_aligns` (`native.rs:3039`) rewrites a zero-`Fill` bank pad so
  the following content resumes on the SAME absolute boundary after relocation:
  `new_pad = baked_after - true_pos`. It is guarded by `if is_bank_align &&
  baked_after >= tru`.
- Every `.emp` section enters the chainer at the COSMETIC base `emp_map_frozen`
  gives it — `lma_base = 0x0` for every distinct section name
  (`native.rs:1558`). So for `dac_banks`, `baked_base = 0` and
  `true_base = $A8000`, and `baked_after` (a small multiple of `$8000` measured
  from 0) is far BELOW `tru` (`$A8000` + the blip length). **The `baked_after >=
  tru` guard fails, the else branch runs, and both cursors simply advance by the
  pad. `recompute_bank_aligns` performs no rewrite on this section under `Fill`
  either.**
- The pad is nonetheless correct, and for a reason independent of the recompute:
  the lowering origin is `0` and the true base `$A8000` is itself `$8000`-aligned,
  so `pos ≡ final address (mod $8000)` and the baseline pad is the final pad.
  The link-time congruence assert `emit_align_pad` records is what actually
  guarantees this, and it holds in both arms.
- Under `Reserve` the pad lands in `recompute_bank_aligns`' combined
  `Fill | Reserve` arm, which advances both cursors by `count` — the identical
  outcome to the else branch it took before.

So the coupling the sweep describes is real in SHAPE (both harness passes match
zero-`Fill` and are blind to a `Reserve`) but is not LIVE: the pass they name
was already inert on this section, for a reason the sweep did not have — the
cosmetic base of `0` that every `.emp` section is chained from. The correct
reading of the do-not-land-alone is that it was a hazard about the AS BINCLUDE
arm, which `dac_banks.emp`'s own header records as DELETED; `directive_align`
stopped emitting `Fill` at `84c48a7b`, so **`recompute_bank_aligns` and
`trim_trailing_align_overshoot` now have no producer of zero-`Fill` align pads
at all.** That is a separate finding, ledgered below, and it is dead code rather
than a live risk — which is why nothing is changed there in this parcel.

## Where the bytes actually move

Not the DAC. The `.emp` files with a TRAILING `align`, where a `Fill` emitted
padding asl would not:

- `games/sonic4/data/animations/particle_anims.emp:19` — last line. One
  `offsets` table (1 word) + a 5-byte body = position 7, odd, so `align 2` had
  been emitting one trailing `$00`.
- `games/sonic4/data/animations/dust_anims.emp:24` — last line, but at an EVEN
  position (11 → pad 1 → 12, then 8 more = 20), so its pad was already zero.
- `games/sonic4/debug/game_debug.emp:193` — last line, DEBUG-shape only.

Everything else (`sonic_anims`, `tails_anims`, `knuckles_anims`,
`ojz_effects`, `effects_scenes`, `boot_data`, and every per-item `(align: 2)`)
has a datum after the align, where fill and reserve render identically.

## The byte differential

One aeon tree (`/home/volence/sonic_hacks/.aeon-align-parcel`, detached at
`ec640bcf`, ROMs deleted before every leg so existence proves freshness), one
`build.sh`, one variable: the sigil pair. Control = master `8faccab9` unchanged.

| Shape | Control (master `8faccab9`) | Treatment (`f1a9e4f7`) | Chain tip (entry 205) | Moved |
|---|---|---|---|---|
| `s4.bin` | `b09ccd65` / 820229 | `b09ccd65` / 820229 | `b09ccd65` / 820229 | no |
| `s4.debug.bin` | `1b7fe316` / 846529 | `1b7fe316` / 846529 | `1b7fe316` / 846529 | no |
| `demo.bin` | `0ad17404` / 96863 | `0ad17404` / 96863 | `0ad17404` / 96863 | no |
| `demo.debug.bin` | `2565ece2` / 103185 | `2565ece2` / 103185 | `2565ece2` / 103185 | no |

**The control reconciles with the chain tip on all four shapes**, so the
differential means something. `LEGS_COMPLETED=4` and an `END_MARKER` in each
arm's log; every leg exited 0; the ROMs were `rm`ed before each leg.

**The two arms genuinely ran different assemblers** — the vacuity this null
result would otherwise be indistinguishable from:

| | `sigil` md5 | `--version` | `emit_sound_blob` md5 |
|---|---|---|---|
| control | `c8842222833cb671f72b3325b204e309` | `sigil 0.1.0 (8faccab9)` | `06b21cac4b96e3103df91fd52acb9622` |
| treatment | `cec28c0eb4ea882cf7046620cf939a5b` | `sigil 0.1.0 (f1a9e4f7)` | `e44adc1c9bd7995374a652ea94c5fb90` |

**All four `.lst` listings are byte-identical between the arms too** — not one
symbol moved, which is a stronger statement than equal CRCs (a compensating
pair of moves cannot hide in it).

### Why byte-neutral, when a trailing align really did stop emitting a byte

It did stop, and the byte is still there — written by a different writer. In
`s4.debug.lst`, `Ani_Particle` is at `$2BCC8`, its 5-byte body at `$2BCCA`, and
the align anchor `__align$games.sonic4.particle_anims$0` at `$2BCD0`; so the
`align 2` pad was one byte, at `$2BCCF`. **The next section, `dust_anims`
(`Ani_DustSpindash`), also begins at `$2BCD0`** — the packing walk puts it
there independently, via `align_up(running, required_for(head_label))`
(`native.rs:2421`, `section_align.rs:247`). So under `Fill` the byte at `$2BCCF`
was particle_anims' own trailing `$00`; under `Reserve` it is the flatten gap
fill `$00` between a 7-byte section image and a successor based at `$2BCD0`.
Same address, same value, different writer.

**This is contingent, not free**, and the condition is worth writing down: it
holds because every one of the 104 rows in `section_align.rs`'s `DECLARED`
table asks for alignment ≥ 2 (census: 102 rows at 2, 2 rows at 8, none at 1).
A head declared with alignment 1 would pack contiguously at `$2BCCF` and shift
everything after it by a byte. The same argument covers
`__align$games.sonic4.knuckles_anims$0` at `$2BCC8`, the other trailing align in
the shipped shapes.

## The DAC bank occupancy, re-derived

The aeon lane measured `engine/sound/generated/dac_shared_bank.bin` at
**25,754 B occupied of 32,768, free tail 7,014 B**, in their working tree at
aeon `ec640bcf`.

Re-derived here from this lane's own emitted image, control arm, plain-s4 leg:
**25,754 B**, crc32 `aded015c`, mtime `2026-09-07 18:30:01 -0400`, tree
`/home/volence/sonic_hacks/.aeon-align-parcel` at `ec640bcf` clean. Free tail
`32768 - 25754 = 7,014 B`. **The two agree exactly**, so there is no staleness
question to adjudicate.

## The gate

`crates/sigil-cli/tests/align_as_parity.rs` — runner
`cargo test -p sigil-cli --test align_as_parity` (and the workspace run).

The sweep's reading of the old gate was VERIFIED before being relied on: all
three pre-existing vectors place a datum AFTER the align and flatten with
`0x00`, so fill and reserve render identically. They stay green under every
mutation below, which measures that claim rather than repeating it.

Three vectors added:

- `a_trailing_align_does_not_lengthen_the_image` — the EFFECT, via image
  length. The AS arm re-derives asl's answer rather than trusting the prose at
  `directive_align`; `vec![0x11]` is the independent hand-derivation.
- `an_align_pad_is_reserved_not_filled` — the EFFECT at the fragment, so a
  future change that keeps the bytes by accident still reddens. This is the
  shape `recompute_bank_aligns` / `trim_trailing_align_overshoot` discriminate
  on, so the two front-ends must hand them the same fragment kind.
- `a_negative_vma_align_follows_the_shared_signed_rule` — the RULE. The
  expectation is derived by CALLING `sigil_ir::asl_align_pad`, not transcribed,
  so it cannot drift from the rule it guards; it is deliberately agnostic about
  the fragment kind so it reds for the rule and nothing else.

### Red-first, mutations shown applied from disk

Each mutation was applied to a tree restored from the committed baseline
(`git checkout HEAD -- <path>`, `git status --porcelain` empty before each), the
patched lines were read back off disk and `git diff --stat` named the file, and
the mutation script `assert`s its match count so an unapplied patch cannot print
`ok`.

| Mutation | `let pad =` / pad call on disk | Result |
|---|---|---|
| M1 rule+effect reverted | `(n - (pos % n)) % n` / `emit_fill(pad, 0, span)` | 3 passed, **3 failed** — all three new vectors |
| M2 effect only | `asl_align_pad(pos, n)` / `emit_fill(pad, 0, span)` | 4 passed, **2 failed** — the two effect vectors; the rule vector green |
| M3 rule only | `(n - (pos % n)) % n` / `reserve(pad, span)` | 5 passed, **1 failed** — the rule vector alone |
| restored | `asl_align_pad(pos, n)` / `reserve(pad, span)` | 6 passed, 0 failed |

The matrix is orthogonal: each half reds exactly its own vectors, so a future
regression names which half broke.

RETROACTIVE (invariant 8(e)): the rule vector originally collected only
`Reserve` fragments and therefore reddened under M2 as well — it was measuring
both halves and could not say which broke. It was rewritten at `f1a9e4f7` to
read the pad size out of whichever pad fragment carries it, and the WHOLE matrix
above was re-run from the committed baseline afterwards. No claim in this note
rests on the earlier two-mutation run.

## Suite

`SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/.aeon-align-parcel
cargo test --release --workspace --no-fail-fast`, from
`/home/volence/sonic_hacks/sigil/.claude/worktrees/agent-ae0641a8b5b55d882` at
`0749e1a4`, branch `parcel/emp-align-reserve` (the log is stamped with pwd,
HEAD, branch and the aeon revision). Ran 19:12:38 → 19:19:23, `RUNNER_EXIT=101`,
`END_MARKER_suite` present.

**429 result lines: 4,849 passed / 5 failed / 2 ignored.** The five, by name:

| Failure | Cause |
|---|---|
| `pins_rs_is_current` | **the parcel's ripple** — see below |
| `particle_anims_debug_region_matches_reference` | the same pin (`pins::PARTICLE_ANIMS.debug_len`) |
| `doctored_af_delete_produces_different_bytes` | the same pin (its control arm reads the same window) |
| `config_a_size_table_rederives_native` | the same shrink in `golden/offcanonical_sizes/config_a.txt` |
| `the_published_line_states_this_revision_s_position_against_a_named_remote_ref` | **environmental, refuted** — see below |

An earlier run of the same command returned 4,686 / 168 / 2. **That was this
lane's own harness artifact, not a result**: the arm script deletes all four
ROMs before each leg, so after the last leg only `demo.debug.bin` existed and
163 of the 168 panicked with `SIGIL_STRICT_GATE set but reference missing`. The
four shapes were rebuilt to coexist (deleted once up front, each leg producing
only its own, `LEGS_COMPLETED=4`, all four CRCs reproduced a second time) and
the suite re-run. The first run is kept at `logs/suite.run1.log` and is not a
verdict on anything.

### The four real failures are ONE ripple, and it is in DERIVED tables only

No ROM byte moved. What moved is the recorded LENGTH of two sections, each by
the one trailing pad byte that stopped being emitted — and no BASE moved, which
is the same fact from the other side:

```
KNUCKLES_ANIMS: plain_len 0x16C -> 0x16B, debug_len 0x16C -> 0x16B   (bases unchanged)
PARTICLE_ANIMS: debug_len 0x8   -> 0x7                               (bases unchanged, plain_len already 0)
config_a.txt:15  Ani_Particle_End  0x2bcd0 -> 0x2bccf                (the ONLY key of 86 that differs)
```

`particle_anims_port` and `tranche4_negative_probes` both take their ROM window
from `pins::PARTICLE_ANIMS` at run time, so both follow the pin: the compiled
module is 7 bytes and the 8-byte window still holds the ROM's 8th byte, which
is now the flatten gap fill. Regenerating `pins.rs` closes all three.

**NOT REGENERATED HERE, on instruction.** The parcel brief's step 5 reserves the
pins and the chain for the controller to sequence (a refreeze landed an hour
before this parcel started). The two artifacts that need it:

- `crates/sigil-harness/src/pins.rs` — via `repin` with BOTH `AEON_DIR` and
  `SIGIL_EMIT` set, per `pins_rs_is_current`'s own instructions.
- `crates/sigil-harness/golden/offcanonical_sizes/config_a.txt` line 15 — via
  `derive_offcanon` / `golden/derive_offcanonical_sizes.sh`. `s4_debug.txt`
  carries `Ani_Particle` but no `_End` row and is unaffected.

The other three of the five-site ripple are expected to be untouched and are
NOT asserted here: no region was added (`repin.toml`), no section base moved
(`engine.inc` orgs), and the DAC blob lengths are unchanged
(`mixed_dac_rom.rs`, `emit_sound_blob` pins) — the emitted `dac_shared_bank.bin`
is `aded015c`/25754 in both arms. Whoever repins should verify rather than
inherit that expectation.

### The fifth failure is environmental, and that was measured, not assumed

`the_published_line_...` compares the banner's baked `origin/master` tip against
what git resolves at test time. **Master moved 12 commits under the running
suite** (`8faccab9` → `edff4932`, all of them `docs/OVERSEER.md`,
`docs/OVERSEER-REFERENCE.md`, `docs/QUEUE.md` — `git diff --stat 8faccab9
edff4932 -- crates/` is empty), so the compiled banner and the live ref
disagreed. Re-running `cargo test --release -p sigil-cli --test
version_provenance` with **no source change** returns **19 passed / 0 failed**.
That master motion also means the byte differential taken against `8faccab9`
holds against current master: not one source file differs.

## Open / TAGGED

- **No emulator ran.** Nothing here was confirmed at runtime. If the byte table
  shows movement in a shipped shape, a foreground boot check is the follow-up.
- **Chain and pins NOT touched, so the branch lands RED on four gates.** Per the
  brief's step 5, movement is reported and stopped there. The four are named
  above with their exact deltas; they are all one ripple in derived tables and
  all close with one `repin` plus one `config_a.txt` line.
- **"No ROM byte moved" is not the same claim as "nothing moved."** Two section
  images are a byte shorter, and only `pins_rs_is_current` and the config_a
  size table could see it — the four-shape CRC differential could not, because
  the successor sections' own declared alignment reabsorbs the byte. A parcel
  that had only run the CRC differential would have reported this as fully
  byte-neutral and been wrong in a way nothing downstream would have caught
  until the next repin.
- `recompute_bank_aligns` / `trim_trailing_align_overshoot` (`native.rs`) now
  have NO producer of zero-`Fill` align pads in either front-end. Ledgered as
  `LENS-BANK-ALIGN-RECOMPUTE-ORPHANED`; deliberately not removed here, because
  deleting a relocation pass is a placement change and belongs to its own parcel
  with its own differential.
