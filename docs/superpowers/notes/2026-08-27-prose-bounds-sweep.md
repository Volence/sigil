# The prose-bounds sweep of sigil's own tree

*2026-08-27. Queue item `PROSE-SWEEP`, branch `sweep/prose-bounds`. The sigil half of the
joint commitment recorded in the PROSE section of
`docs/superpowers/notes/2026-08-27-absent-and-silent-are-one-artifact.md`; the aeon lane
sweeps its own tree.*

The class: **a bound asserted in prose is executed by nothing, so no gate can contradict
it.** A stale bound in code eventually fails something. A stale bound in prose just teaches.

**This sweep found what these queries found. It is not a completeness claim** — see
*What this sweep did NOT cover* at the end, which is the load-bearing section.

---

## The queries, verbatim and re-runnable

Run from the repo root. `git grep` so untracked build output cannot pad a count.

### A — numeric literals on comment lines (the "does a number appear" parameter)

```
A1  git grep -nE '^[[:space:]]*(//|///|//!).*0x[0-9A-Fa-f]+'          -- 'crates/*.rs'   # 1348
A2  git grep -nE '^[[:space:]]*(//|///|//!).*\$[0-9A-Fa-f]+'          -- 'crates/*.rs'   # 1432
A3  git grep -nE '^[[:space:]]*(//|///|//!).*[^0-9A-Za-z_][0-9]{2,}'  -- 'crates/*.rs'   # 4661
A4  git grep -nE '[^:]//[^/].*0x[0-9A-Fa-f]+'                         -- 'crates/*.rs'   # 1309
B1  git grep -nE '^[[:space:]]*//!.*(0x[0-9A-Fa-f]+|\$[0-9A-Fa-f]+)'  -- 'crates/*.rs'   #  285
B3  git grep -nE '^[[:space:]]*///.*(0x[0-9A-Fa-f]+|\$[0-9A-Fa-f]+)'  -- 'crates/*.rs'   #  795
```

### B — bound-language keywords (the "what is being claimed" parameter)

```
K1  git grep -ncEi '^[[:space:]]*(//|///|//!).*(both shapes|same in both|identical in both|shape-invariant|byte-identical in both|unchanged in both)' -- 'crates/*.rs'   # 276
K2  git grep -nE  '(same-LENGTH|same length|identical length|shape-invariant|length-invariant|same-length)' -- 'crates/*.rs' 'docs/OVERSEER.md'                          # 107
K3  git grep -nEi '^[[:space:]]*(//|///|//!).*(exactly|at most|no more than|up to|must be|fits in|capacity of) [0-9$]' -- 'crates/*.rs'                                  #  30
K4  git grep -nEi '"[^"]*(must be|at most|no more than|exceeds|too (large|many|long)|out of range|limit)[^"]*"' -- 'crates/*/src/*.rs'
K5  git grep -nEi '^[[:space:]]*(//|///|//!).*plain `?\$[0-9A-Fa-f]{3,}`?.*(/|,| ) *debug'  -- 'crates/sigil-cli/tests/*.rs' 'crates/sigil-harness/src/*.rs'             #   14
```

### C — shaped forms (the "how is a window spelled" parameter)

```
C1  git grep -nE 's4(\.debug)?\.bin\[0x[0-9A-Fa-f]+\.\.0x[0-9A-Fa-f]+\]'  -- 'crates/*.rs'   # 51 sites / 15 files
C2  git grep -nE 's4(\.debug)?\.(bin|lst)\[\$[0-9A-Fa-f]+'                -- 'crates/*.rs'   #  2 sites
C3  git grep -nE '^//! *(Plain|Debug|2-byte|Length went)'                 -- 'crates/sigil-cli/tests/*.rs'
E1  git grep -nE 's4(\.debug)?\.lst'                       -- 'crates/*.rs' 'docs/OVERSEER.md'   # 36 sites / 20 files
```

### D — the derived instruments (the ones that actually decided anything)

Three throwaway scripts, all reading `crates/sigil-harness/src/pins.rs` as the generator's
own output rather than pattern-matching what a correct value looks like:

- **D1** parse every `Region { plain_base, debug_base, plain_len, debug_len }` out of
  `pins.rs`; extract every `s4[.debug].bin[0xA..0xB]` from comments; report windows that
  match no current region in the right shape.
- **D2** per file, infer the region from the `pins::NAME.{plain,debug}_{base,len}` the file
  actually reads, then check each `$X both shapes` / `shape-invariant` claim against that
  region's real `plain_len == debug_len`.
- **D3** build the set of every numeric value `pins.rs` currently holds (bases, lengths,
  `base+len` ends, `Pin` VMAs, offsets — 955 distinct), then flag backticked/parenthesised
  hex of ≥3 digits in comments **in files that `use sigil_harness::pins`** that is not any
  of them.

---

## Where the two enumerations disagreed — neither is a superset

This is the part worth carrying to the aeon lane.

1. **C1 (window shape) missed the `$`-spelled windows.** `seam1_native_link.rs` writes
   `s4.bin[$3DE..$1BFA]`, not `0x3DE`. Two stale sites, invisible to the pattern that found
   the other 51. **The literal spelling of the radix was a variable of the population and I
   had baked one value of it into the query** — the same failure the companion note records
   for `pgrep -f "refreeze --attest"`.
2. **C1 also missed every bound not spelled as a window.** `math_port.rs`'s
   *"the block's CONTENT (24 bytes of code + the 640-byte embedded sine table = 0x298 bytes
   total) … only its BASE address shifts (plain `$2464`, debug `$25F6`)"* carries three
   stale numbers and no `s4.bin[...]`. K5 and D3 reach these; C1 cannot.
3. **The keyword sweep (K1/K2) missed the numberless-but-false claims' numbers, and the
   numeric sweep missed the numberless claims.** `parallax_port.rs` and
   `seam1.rs` say "shape-invariant" with no number against regions whose `plain_len` and
   `debug_len` differ in `pins.rs`. A numeric sweep sees nothing there.
4. **E1 (`.lst`) is reachable by neither A nor K.** 36 sites still describe resolution
   against `s4.lst` / `s4.debug.lst`, an instrument `repin.rs`'s own header says was
   retired. That is a stale *mechanism* in prose, not a stale number, and only a query
   naming the retired tool finds it.
5. **D3 over-matched badly and that is worth stating.** Unrestricted it flags 337 sites;
   restricted to files where `pins` actually owns the subject, 50. The 287-site difference
   is almost entirely synthetic fixture addresses in unit tests (`lower_data.rs`,
   `operand_const_as_address.rs`, `m68k.rs` opcodes) — which are **code-adjacent**: the test
   constructs the value, so a wrong one goes red. A raw count here would have been a
   dishonest headline.

---

## The decision rule applied to every candidate

> *Is there any check in this repo that would go red if this number were wrong?*

If yes → code-adjacent, **out of scope for this class**. If no → instance.

The port gates are the pure case: every one of them reads its window from
`pins::<REGION>` (or the derived sound layout) at run time and then **restated the same
window in its own doc comment**. The body is checked. The prose is not. So the prose was
free to be wrong, and it was — in every single case.

---

## Instances found, by these queries

Cited by file and heading/symbol. **Not by line number** — line numbers in this workspace
rot within hours and a correction carrying one inherits the defect it is correcting.

### Group 1 — region-length claims that `pins.rs` refutes (fixed, commit `49666597`)

| File | Heading / symbol | Claimed | `pins.rs` plain / debug | Verdict |
|---|---|---|---|---|
| `bg_anim_port.rs` | `## Shape` | `$A0` both shapes | `0x9E` / `0x158` | **stale + claim false** |
| `bg_port.rs` | `## Shape` | `$AE` both shapes | `0xE0` / `0x140` | **stale + claim false** |
| `camera_port.rs` | `## Shape` | `$16A` both shapes | `0x1D0` / `0x1E0` | **stale + claim false** |
| `plane_buffer_port.rs` | `## Shape`, `map_toml` | `$29C` both shapes | `0x328` / `0x378` | **stale + claim false** |
| `section_port.rs` | `## Shape`, `map_toml` | `$3EA` both shapes | `0x42C` / `0x48C` | **stale + claim false** |
| `load_object_port.rs` | header | `$9E` both shapes | `0x88` / `0x88` | invariance true, **number stale** |
| `mt_bank_port.rs` | header | head `$607`, body `0x34E1`/`0x4F33`, LMA `$58607` | `0x630`, `0x34E8`/`0x4F38`, base `0xA0630` | **stale** |
| `error_handler_port.rs` | header, `Shape` comment | `0x10B0` both shapes | `0x10B0` / `0x10B0` | **correct-but-unverifiable** |

`section_port` and `plane_buffer_port` **contradicted themselves inside one screen**: the
doc comment on `map_toml` said *"sized to $3EA (both shapes)"* three lines above
`let len = if debug { pins::SECTION.debug_len } else { pins::SECTION.plain_len };`.

### Group 2 — reference-window blocks (fixed, commit `b60e79cb`)

**51 windows across 15 files. Zero matched a current `pins.rs` region.** The ROM re-layout
moved every one and nothing went red. Files: `act_descriptor`, `animate`, `collision`,
`controllers`, `core`, `dplc`, `entity_window`, `game_loop`, `math`, `mt`, `rings`, `sfx`,
`sonic_anims`, `sprites`, `test_mappings` (all `_port.rs`).

Three files already carried the correct form and were left alone — `collision_lookup_port`,
`sound_api_port`, `vdp_init_port`, plus `particle_anims_port`'s *"Both windows come from
`pins::PARTICLE_ANIMS` at run time"*, which is the shape every fix here follows.

`sprites_port.rs` is the sigil twin of the aeon instance that prompted this sweep. Its
header already pointed at `pins::SPRITES.{plain,debug}_len` *and still restated
`0x408`/`0x4DA` twelve lines down* — **the pointer was added without removing the copy**,
which is the half-fix this class specifically warns about.

`sfx_port.rs` contradicted itself across 90 lines: `$5BB20`/`$5D570` at the top, and at
`map_toml` *"a re-layout (2026-08-26, `$5BB20`/`$5D570` → `$A3B20`/`$A5570`) moves this with
it"* — naming the correct new values while the top of the file kept the old ones.

### Group 3 — a doc comment refuted by the assertion below it (fixed, `b60e79cb`)

`seam1_native_link.rs`, `blob_lengths_are_canonical`: the doc comment opened *"The debug
blob is **exactly $7E** longer than plain."* Four lines below,
`assert_eq!(BLOB_LEN_DEBUG - BLOB_LEN_PLAIN, 0x82, …)`. The same doc block's own
*"6163 / 6293"* also implies `0x82`. **Of the three statements, the two that execute agreed
and the one that does not was wrong** — and per the note's adjacency rule the wrong sentence
was *edited*, not annotated. The module header's *"(6172 B)"* / *"6300 B = 6172 + $7E"* and
the two `assert_blob_matches` label strings carried the same stale pair; all removed.

### Group 4 — per-shape base/VMA prose (fixed, `b60e79cb`)

`controllers_port` (7 sites), `math_port` (6), `collision_lookup_port` (6), `game_loop_port`
(4), `test_objects_port` (8, incl. a self-labelled *"2026-07-10 pins"*), `particle_anims_port`
(2), `sfx_port` (2). All stale against `pins.rs`; all now name the pins symbol.

### Group 5 — found, deliberately NOT fixed

| Item | Why not |
|---|---|
| `crates/sigil-harness/repin.toml` — the header's *"resolves every entry against BOTH aeon listings (`s4.lst` / `s4.debug.lst`)"* **and its per-region comments** | **BLOCKED by scope.** `repin.toml` is the byte-mover lane's file and this parcel was told not to touch it. The header is already recorded as load-bearing in the companion note — it is the sentence that made the pin gate look like an independent instrument. What this sweep adds: the manifest's **per-region comments carry the identical stale claims that were just fixed in the tests** — *"length (`$29C` both shapes — no asserts, no `__DEBUG__`)"* for plane_buffer (`pins.rs`: `0x328`/`0x378`) and *"Plain `$513E..$5528` = `$3EA` bytes"* for section (`pins.rs`: base `0x42C` wide, different base). So the defect is in the **manifest that feeds the generator**, not only in its consumers, and whoever fixes the header should sweep the region comments in the same pass. Leaving it is a deferral, not a judgement. |
| `crates/sigil-harness/tests/repin_pins.rs` — ~40 `both shapes` deltas | **Code-adjacent and a dated ledger.** Every value there sits inside an `assert_eq!`, so a wrong one goes red; the comments are an append-only per-parcel record, which is a snapshot of its date by design. |
| The other 34 `s4.lst` / `s4.debug.lst` sites | Stale *mechanism*, one enumerated cluster, larger than this parcel. Fixing them means re-describing what each gate resolves against — real work, not a token swap. **Queued, not half-done.** |
| `dplc_negative_probes`, `hblank_negative_probes`, `tranche3_negative_probes` — *"`$2700` instead of the real plain `$26FC`"* | The **wrong** value is a deliberate fixture (fine). Its *"the real X"* companion **is stale** (`pins::DPLC` is `0x2C48`). Fixable, but the sentence's whole point is the contrast, so it needs rewriting rather than deleting. Queued. |
| `header_port.rs` `$18E`/`$1A4`, `ojz_run_a_port.rs` `$FFFF`, `tranche6_negative_probes` `$12345`, `clownlzss` `65535`, `clownnemesis` `0x20`, `provenance.rs` `40-char SHA` | **Genuine documented constants.** Genesis ROM-header field offsets, a data sentinel, a fixture, `u16::MAX`, tile granularity, SHA-1 hex width. These do not rot on this clock. |
| `load_object_port.rs` shape-invariance | Its claim is **true** (`0x88`/`0x88`) but unasserted. The strong fix is an `assert_eq!(pins::LOAD_OBJECT.debug_len, len, "…shape-invariant")` — the in-repo precedent is `sfx_bank_port`, `soundbankhead_port`, `collision_data_port`, `dac_bank_port`. **Recommended, not applied**: adding a gate to a landing lane's test is the owner's call, and a legitimately-diverging future would make the fix a hand-edited baseline — the vulnerability the maintenance-act note names. |

### The positive result, which is also a finding

**The diagnostic and refusal message surface is clean.** K4 returns essentially only
*derived* messages — `{max}`, `{len}`, `{lo}..={hi}`, `operand {v} out of range {lo}..={hi}`,
`module_size {requested:#x} exceeds the maximum {max:#x}`. sigil's error text interpolates
its bounds from the values it just checked rather than hardcoding them, so **this class
mostly cannot arise there**. There is no `clap` in the workspace; the nine `usage:` strings
carry no numeric bounds.

`docs/OVERSEER.md` is also clean *for this class*: its counts (`3990 passed / 0 failed / 4
ignored`) are stamped with the measuring SHA, the aeon SHA, the chain entry and the date —
which is the correct pattern, and the file already records an incident where one un-stamped
half went stale for a day.

---

## Verification

`CARGO_TARGET_DIR=/home/volence/sonic_hacks/.sigil-prose-target` — on nvme, **not** tmpfs,
**not** the shared `target/`.

```
SIGIL_STRICT_GATE=1 AEON_DIR=/home/volence/sonic_hacks/aeon \
  cargo test --release --workspace --no-fail-fast
```

| | passed | failed | ignored |
|---|---|---|---|
| `sweep/prose-bounds` @ `b60e79cb` | 4008 | 15 | 4 |
| baseline `master` @ `2b26419c`, same command, same target dir | 4008 | 15 | 4 |

**The failing test-name SETS are identical** (`diff` of the sorted `FAILED` lines is empty)
— checked as sets, not as counts, because two different fifteens produce the same number.

All 15 are environmental, and the repo's own guard says so:

```
aeon_dir_matches_the_provenance_tip:
AEON_DIR ... is at aeon 4b43bdda..., but the goldens were frozen from aeon 33d905b8...
(provenance tip `sprite-owner`, entry #173).
```

Not one of the 15 is in a file this parcel touched. **Not investigated and not fixed**, per
the coordinator's standing instruction on the peer lane's in-flight freeze.

---

## What this sweep did NOT cover

Stated because an empty result and a query that never matched produce the same artifact, and
so do a partial sweep and a complete one.

- **`.emp` and `.asm` sources.** Zero coverage. The queries are `crates/*.rs` and
  `docs/OVERSEER.md` only.
- **`docs/superpowers/**` dated notes, plans and specs.** Out of scope by design: a dated
  note asserting what was true on its date is a record, not a defect. **But the specs are a
  real gap** — `SIGIL_*.md` under `empyrean/docs/` is read as current and was not swept, by
  either lane, as far as this note knows.
- **Decimal bounds.** A3 has 4661 hits and was never triaged; every decimal instance in this
  note was found incidentally via a neighbouring hex. `mt_port`'s *"13,544 bytes"* against a
  `0x34F0`-wide window in the same sentence (`0x34F0` = 13552) was caught only because the
  hex was there too. **A decimal-only stale bound would have survived this sweep.**
- **Prose bounds whose subject `pins.rs` does not own.** D3's restriction to
  pins-consuming files is what made it decidable; it also means a stale bound about
  anything else was not checked.
- **Non-numeric prose rot** other than the `.lst` cluster: stale mechanism descriptions,
  retired flags, superseded procedures. The companion note's *booking* class
  (a row that invents a blocker) was not swept for here at all.
- **Whether the fixes' pointers are themselves correct.** Each replacement names a
  `pins::<REGION>` the file already reads, so a wrong pointer would be visible next to the
  code — but no gate asserts that a doc comment names the right symbol either. **This fix
  reduces the rot surface; it does not close the class.** The only thing that closes it is a
  bound that is *generated* into prose or asserted, and nothing here does that.

## The one that would close it

Every fix above is still prose. The structural version is a test that renders the pinned
geometry and diffs it against the doc block — i.e. make the doc comment a **generated**
artifact of `repin`, the way `pins.rs` itself is. Queued as `PROSE-GENERATED-WINDOWS`.
Until then this class is *reduced in surface*, not *made unwritable*, and a future parcel
will re-author a number here unless something refuses it.
