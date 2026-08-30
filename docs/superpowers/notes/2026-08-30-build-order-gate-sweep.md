# Sigil's multi-artifact gates, swept for defeat-by-build-order

*2026-08-30. A read-only source enumeration, prompted by the aeon lane's finding that
`tools/demo_specialization_witness.py` reads `s4.debug.lst` **and** `demo.debug.lst` — two
listings from two separate build invocations — so an order leaving either stale puts the gate
green with the pin wrong, and nothing ever sees both fresh.*

**Why enumeration and not red-first testing.** A gate defeasible by ordering has no failing
mode to observe: with the wrong order it passes, and with the right order it also passes. There
is no input that makes it red for the right reason. The only instrument is to enumerate each
gate's inputs and ask, per input, **which invocation produces it, and can that invocation be
skipped, repeated, or run out of order relative to the gate.**

No build, `cargo test`, `cargo run` or build script was run for this sweep; none is needed for
a source enumeration, and a paired freeze was live against this repo's goldens throughout. The
one claim that would have needed a build is recorded as BLOCKED in §6.

**Headline.** aeon's exact defect — a gate reading two listings — **does not exist in sigil**,
and not by luck: the asl `.lst` parse was retired from `repin` at Stage-3 P4c, and every
surviving listing consumer pairs a listing with the ROM from its own invocation. What sigil has
instead is the same class one artifact over, and its worst instance is not a gate at all:
**`refreeze --freeze` validates the aeon revision once, at step 0, then re-reads that tree at
each of its next three steps over a run that outlives a ten-minute foreground cap.** A tree that
moves mid-freeze yields a set of artifacts that are mutually consistent, entirely green under
every downstream gate, and attributed to a revision that built none of them.

---

## 1. The parameter enumerated over

Not gate-sounding names, and not test names. **The build products themselves**, and then every
site that opens one.

| | Product | Produced by |
|---|---|---|
| **P1** | `$AEON_DIR/{s4,s4.debug,demo,demo.debug}.{bin,lst}` | aeon `./build.sh <game>` / `DEBUG=1 ./build.sh <game>`; also `sigil build --config-a/--config-b/--lean`, which clobber `s4.bin` / `s4.debug.bin` |
| **P2** | `golden/{s4,s4.debug,demo,demo.debug,config_a,config_b,lean}.bin` | `golden/capture_goldens.sh --write` (which itself drives P1 nine times) |
| **P3** | `golden/offcanonical_sizes/*.txt` ×7 | `golden/derive_offcanonical_sizes.sh` → `derive_offcanon` |
| **P4** | `crates/sigil-harness/src/pins.rs` | `repin` |
| **P5** | `golden/provenance.toml` | `refreeze --freeze` step 4 / `--attest` |
| **P6** | `$CARGO_TARGET_DIR/attest/witness-*.txt`, `suite-*.log` | `refreeze --attest`, read back in the same process |
| **P7** | `golden/.freeze-journal` (gitignored) | `refreeze --freeze`, in flight |
| **P8** | `$AEON_DIR/engine/{sound,debug}/generated/*` | `emit_sound_blob`, aeon `tools/gen_compression_vectors.py`, or in-process `native::ensure_generated` |
| **P9** | `golden/ab/**/*.json`, `*.ram.bin`, `*.png` | one oracle A/B capture run each |

Search went at the read sites, not the names: path joins onto those filenames, the `golden/`
roots reached through `CARGO_MANIFEST_DIR`, `reference_tree(&[…])` argument lists (multi-line,
so single-line greps miss them), `listing_symbol_addr`, `load_frozen_table`,
`provenance::tip_target`, `include_str!`, and `pins::` use.

**The discriminator, because "reads two products" is not yet "exposed."** Staleness produces a
false green only where it can make the asserted relation *hold*. Six mechanisms, and only the
first five are exposed:

* **M1 — a precondition checked once, before a multi-step producer that re-reads its input.**
* **M2 — a cross-product stamp that is written and never read.**
* **M3 — a currency check that regenerates through the same stale input its producer used**
  (agreement is guaranteed rather than earned).
* **M4 — a source-coordinate check standing in for an artifact-freshness check.**
* **M5 — two products compared to each other, where staleness aligns them.**
* **M6 — one product BOUNDS another; a stale bound shrinks the claim rather than breaking it.**
* *(safe)* — the two products sit on **opposite sides** of one comparison, so staleness in
  either makes them disagree. This is the shape of sigil's largest population.

---

## 2. The aggregate

| | count |
|---|---|
| Gate carriers examined | **360** |
| — integration-test targets | 321 |
| — `sigil-harness` bins / library modules | 5 / 6 |
| — tracked shell scripts / repo python tools / A/B runner scripts | 7 / 2 / 18 |
| — CI workflows | 1 |
| Read at least one build product | **123** |
| Read **two or more** build products | **84** |
| Of those, cross an invocation boundary | **70** |
| **Exposed** — a defeating order exists | **14 carriers, in 6 mechanisms** |
| Safe by construction, or fail-loud on staleness | the remainder |
| Freshness assertions **anywhere in the repo** | **2** |

The 14 exposed carriers, named: `refreeze.rs` (all three of `--freeze`, `--attest`, `--check`),
`derive_offcanon.rs`, `repin_pins.rs`, `provenance_chain.rs`, `offcanon_assembled_bar.rs`,
`m1c_vector_table.rs`, `native_full_rom.rs`, `compression_selftest_port.rs`,
`native_offcanonical_full.rs`, `native_offcanonical_placement.rs`, `provision-aeon-ref.sh`,
`capture_goldens.sh`, `region-hash.sh`, and the five `SKIP_RELOAD` A/B runners as one family.
Three of those (`capture_goldens.sh`, `region-hash.sh`, the runners) sit outside the 84 or read
only one product, so the exposed count is not a subset arithmetic of the row above it.

Per-crate, from read-site tracing rather than name matching:

* `crates/sigil-cli/tests` — 144 targets; **81** read ≥1 product; **72** read ≥2; **60** cross an
  invocation boundary; **4** single assertions consume two products from different invocations.
  The remaining 12 multi-product files are all `golden`-only: several blobs, one capture run.
* `crates/sigil-harness/tests` — 21 targets; **13** read ≥1; **4** read ≥2, all cross-invocation
  (`m1c_vector_table`, `offcanon_assembled_bar`, `provenance_chain`, `repin_pins`).
* the other seven crates — 156 targets, **zero** build-product reads. Only two files outside
  `sigil-cli`/`sigil-harness` touch the aeon tree at all (`cfg_blind_spots.rs`,
  `isa/tests/support/capstone_diff.rs`), and `nightly_source_gates.sh`'s own classifier puts
  the first in the source lane. Everything else reads committed `tests/vectors/` corpora.

**The last row is the most compact result, and it is structural rather than a `grep -c` of a
spelling.** Over the 7 tracked shell scripts, `-nt`/`-ot` occurs exactly twice, both in
`capture_goldens.sh:96` and `:114` (positive control: `echo` matched in all 7 files in the same
pass). Over the 22 tracked python files, zero. Over every `.rs` file in `sigil-harness` and
`sigil-cli/tests`, every `SystemTime::now()` is temp-directory naming or a timestamp string.
**No Rust gate in sigil asserts the freshness of any artifact it reads.**

### A methodological result worth keeping

Grepping `crates/sigil-harness/tests` for the *names* of build products returns **13** files.
Tracing the actual reads returns **13** files. **Ten of them are the same file.** Three name a
product they never open; three open one whose path never appears in their text, because
`load_frozen_table` is called from inside every `GameProfile` constructor
(`native.rs:716,763,824,881,933`). Two methods, the same headline number, a 77% membership
overlap. A count agreeing is not the methods agreeing.

---

## 3. The exposed set

### Tier 1 — a false green about the freeze itself

#### E1 · `refreeze --freeze` checks the aeon revision once and reads the tree four times

*Mechanism M1. The largest hole found.*

`resolve_aeon_rev()` refuses unless `AEON_DIR` is set, is a git repo, resolves `HEAD` to a
40-char SHA, and is **clean**. It is called at exactly two sites — `refreeze.rs:651` (attest)
and `:1199` (freeze) — and in `do_freeze` it runs **before** `freeze_steps` at `:1250` and never
again. Its own comment says why it is early: *"Checked BEFORE the build, since the build itself
writes into this tree."*

The four steps then each re-read `$AEON_DIR` independently:

| step | site | reads | writes |
|---|---|---|---|
| 1 capture | `:1264` | aeon source, nine times over | P2 (and P1) |
| 2 sizes | `:1267` | aeon source + P2 + P3 (its own prior generation) | P3 |
| 3 pins | `:1269` | aeon source + P3 | P4 |
| ledger | `:1272-1278` | P4 + P3 headers, then the seven **fresh** P2 blobs | P5 |

**The defeating order.** Start `refreeze --freeze` at aeon rev N, clean. Step 1 builds and
freezes seven blobs from rev N. During step 1 or 2 — a window the OVERSEER file records as
outliving a ten-minute foreground cap — `$AEON_DIR` moves to rev N+1. Step 2 derives the size
tables from N+1. Step 3 derives `pins.rs` from N+1. Step 4 reads the fresh pins and table
headers for `anchor_end`, CRCs the **rev-N** blobs **at a rev-N+1 anchor**, and appends an entry
recording `aeon_rev = N`.

Afterwards every gate is green, because step 4 derives its numbers from steps 1-3's *outputs*
rather than from the tree: `refreeze --check` (blobs match the tip at the recorded
`anchor_end`), `provenance_chain_holds`, `offcanon_assembled_bar` (header == `anchor_end`,
pins == table), `repin_pins::pins_rs_is_current` (both sides use the N+1 tables). The ledger
names one revision and the blobs came from another, and **nothing re-checks HEAD or cleanliness
after step 0.**

The operational mitigation exists and is not enforced by the tool: `docs/OVERSEER.md`'s landing
lane requires freezing from a clean checkout of a committed SHA — a dedicated worktree nobody
else touches. `--freeze` accepts any tree that is clean at step 0, including the aeon main tree,
which this lane's own notes record as carrying the owner's live content edits at all times.

**The cheap remedy is a HEAD re-read, not a cleanliness re-check.** A post-step-3
`git status --porcelain` would fire on the build's own writes, which is why the check is early.
`git rev-parse HEAD` would not: re-read it after step 3 and refuse if it moved. That closes the
revision half exactly, with no false positives.

A second, cheaper defeat needs no race at all: run the three steps **by hand, out of order**.
`capture_goldens.sh --write`, `derive_offcanonical_sizes.sh` and `repin` are all documented
standalone entry points. `refreeze` cannot observe that its steps were not its own — a completed
journal is deleted, and a run that never opened one leaves nothing.

#### E2 · `derive_offcanon`'s tie to the golden blob is written and read by nothing

*Mechanism M2.*

Every `golden/offcanonical_sizes/<t>.txt` opens with:

```
# target=s4
# reproduces_golden=s4.bin
# golden_crc32=6e746bb9
# assembled_anchor=c44a97fa
# assembled_end=0xa5c90
# labels=68
```

`derive_offcanon.rs:69-89` opens `golden/<reproduces_golden>` and computes the two CRCs from its
bytes; the comment there reads *"Provenance: tie to the committed golden blob."* Grepped across
all of `crates/` with a positive control: `assembled_end` has six readers (`refreeze.rs:191-200`
and `offcanon_assembled_bar.rs:64-73`); **`golden_crc32`, `assembled_anchor`,
`reproduces_golden` and `labels` have exactly zero, and their only occurrences are the four
`out.push_str` write sites at `derive_offcanon.rs:100-104`.**

**The order that defeats it: step 1 without step 2.** After a hand-run
`capture_goldens.sh --write` — which `freeze_journal.rs` names as its own explicit residual hole,
*"unjournaled and still silent"* — the blobs are new and every table's `golden_crc32` describes
the previous set. `refreeze --check` goes red only if the ledger was skipped too;
`offcanon_assembled_bar` passes, because `assembled_end` is derived from source and does not
move for a byte-neutral parcel. Nothing compares the recorded CRC to the blob beside it.

Two facts make this more than book-keeping:

* **The size table is a BUILD INPUT, not only a frozen expectation.** `load_frozen_table`
  (`native.rs:247-252`) is called from inside every `GameProfile` constructor, and
  `profile.frozen_sizes` feeds `true_bases_by_index` — the declared-order placement walk
  (`native.rs:3617-3622`) that supplies provisional bases, org-island anchors and per-section
  alignment quanta. Every native ROM every gate builds is laid out from it. 26 test files reach
  it without the path appearing anywhere in their text.
* **The OVERSEER file already nominates this artifact as the best freeze witness** — *"a table
  of unmoved labels beside two changed CRCs cannot be produced by a build that did not run.
  Prefer it to the pin file for length-neutral parcels."* Those two changed CRCs are
  `golden_crc32` and `assembled_anchor`. **The repo's best freeze witness is checked by a human
  reading a diff, and by no gate.**

The check is one comparison per shape and it currently passes 7/7 — measured here on the
committed blobs at master `9fd6607d`, so the other lane's in-flight tree cannot colour it:

```
s4 6e746bb9 · s4_debug 839bafaf · demo 3415e3ef · demo_debug fdedb6e4
config_a 39f3f765 · config_b 6c21739d · lean fa81802d      — table == blob, all seven
```

It belongs in `offcanon_assembled_bar`, which is already the file that compares one value across
tools, already source-only, and already never skips.

**A second exposure in the same binary: self-feedback.** `derive_offcanon` reads P3 — *the
previous generation of the file it is about to overwrite* — through the profile constructors at
`:47-56`, and uses it as the placement anchor set for the resolve that produces the new one.
`derive_frozen_table`'s own doc calls this a fixpoint and says so plainly; the consequence is
that a layout error introduced in generation *k* rides into *k+1* unless the delta happens to
dislodge it.

#### E3 · `pins_rs_is_current` regenerates through the same input `repin` used

*Mechanism M3.*

`repin_pins::pins_rs_is_current` (`repin_pins.rs:34-71`) runs the identical pipeline `repin` runs
— `sigil_native_symbol_listing` for both shapes, `phase_bank_lmas`, `section_extents`,
`section_label_owners`, `resolve(&manifest, …)`, `render` — and compares the result to
`include_str!("../src/pins.rs")`. Verified by reading both: the two code paths match statement
for statement.

Both sides reach `load_frozen_table` through `resolve_canonical_sections` → `sonic4_profile`.
**The defeating order:** run `repin` standalone — the documented invocation at `repin.rs:5-8` —
after an aeon source change, without running `derive_offcanonical_sizes.sh`. `repin` derives
`pins.rs` from a resolve anchored on the **old** size tables; this test re-derives from the
**same old** tables and finds agreement. Green.

Stated precisely, because the overclaim is easy: the test's literal claim — *re-running `repin`
now would produce this file* — is exactly what it verifies, and it verifies it correctly. The
gap is between that and the claim it is read as: *the pins describe the shipped layout*.
`pins.rs` is current relative to (source ∧ `golden/offcanonical_sizes/`), never relative to the
source alone. `refreeze --freeze` gets the order right — sizes at step 2, pins at step 3 —
and nothing outside that one function enforces it.

### Tier 2 — a green about a revision that was never measured

#### E4 · The source-coordinate gates certify git HEAD and never look at a built artifact

*Mechanism M4. Two carriers, one shape.*

* `provenance_chain::aeon_dir_matches_the_provenance_tip` (`provenance_chain.rs:104`) runs
  `git -C $AEON_DIR rev-parse HEAD` and compares it to the tip's `aeon_rev`. Hard under
  `SIGIL_STRICT_GATE=1`. It reads `golden/provenance.toml` and **no aeon build product at all.**
* `refreeze --attest` step 4 (`refreeze.rs:645-660`) applies the same comparison as a hard
  refusal before running the suite.

**The order that defeats both:** build `$AEON_DIR/s4.bin` at rev N+1, then
`git checkout <tip_rev>` in `$AEON_DIR` **without rebuilding**. The rev check passes; the suite
runs; every P1-reading gate inside it — `m1b_gate`'s checksum re-derivation, `m1c_vector_table`,
the 54 region gates — measures the rev-N+1 ROM; and the ledger records *"the strict suite passed
on aeon `<tip_rev>`."* These gates certify the **source tree's coordinates** and say nothing
about the **binaries sitting beside it**.

#### E5 · `provision-aeon-ref.sh`'s REBUILD CONTROL compares a file the same script placed

*Mechanism M4, and the reason E4 has no floor.* This script is the recipe every artifact-lane
`AEON_DIR` comes from.

* **`:76`** — `shutil.copy2(src, w / fn)` copies `golden/{s4,s4.debug,demo,demo.debug}.bin`
  **into** the reference tree.
* **`:118-122`** — builds both canonical shapes in that tree.
* **`:126-141`** — reads `$W/s4.bin` and `$W/s4.debug.bin` back and prints
  `REBUILD CONTROL … MATCHES THE GOLDEN`, against the same `provenance.toml` entry the copy at
  `:76` came from.

**The control cannot distinguish a real rebuild from its own copy.** Its stated claim — *"a ROM
built here from the pinned source must match the golden CRC32 byte for byte"* — is true, and the
instrument does not touch the half that could be false. Under `set -e` a build that *fails* is
caught; a build that exits 0 **without writing** is not, and that is the precise failure mode
this repo already documents for aeon's `gen_compression_vectors.py`, which prints `FAIL:` and
exits 0.

The listing half is weaker still: `:124` is `[ -s "$W/$l" ]` — nonempty, i.e. **presence**. A
leftover `s4.lst` from an earlier provisioning of a different revision satisfies it, and the
script's own header explains at length why a missing listing surfaces three tests away as a
misattributed `unresolved symbol`.

**Nothing downstream closes it.** The script's nominated witness is
`repin --check → "pins.rs unchanged"`, and `repin` reads no ROM and no listing: it resolves both
shapes in-process from source. E4's pairing gate is also source-only. **The whole freshness
argument for `$AEON_DIR/s4.bin`, `s4.debug.bin`, `s4.lst` and `s4.debug.lst` rests on a git-rev
match on the source plus a control that is blind to whether the build ran.**

`capture_goldens.sh` carries the fix in the same repo — a `mktemp` marker plus
`[[ "$path" -nt "$marker" ]]`, described in its own header as the stale-capture guard. The two
producers disagree about whether that guard is needed.

#### E6 · `m1c_vector_table` is green on a revision it did not measure

*Mechanism M4.* It asserts that assembling `m1c_root.asm` plus aeon's front-matter include tree
reproduces `$AEON_DIR/s4.bin[0..256]` byte for byte, and it reads three products:
`s4.bin` (`:86`, **P1**), `pins::{ENTRY_POINT, V_BLANK_HANDLER, …}` (`:62-78`, **P4** — these
become the compared `dc.l` bytes), and `golden/offcanonical_sizes/s4.txt` (**P3**, transitively).

Both single-sided orders fail **loud**: repin without rebuilding, or rebuild without repinning,
both go red. The false-green order is neither: build at rev N → `repin` at rev N → check out rev
N+1 whose delta lies past the vector table → run the suite. Every artifact agrees, the gate is
green, and the sentence it claims — *the current source's front-matter reproduces the reference
ROM's vector table* — was tested against a ROM and a pin table from a different revision.

#### E7 · `offcanon_assembled_bar` — the one gate built for this class, and where it stops

Four tests. Two are safe: `size_table_header_agrees_with_its_endofrom_row` (`:113`) compares two
spellings of one number **in the same file**, and `every_frozen_size_table_is_gated` (`:164`)
`read_dir`s the directory against its own `targets()` list. Two are exposed:

* **(a) `assembled_len_matches_provenance_tip_for_every_shape` (`:82`)** — P3 vs P5. The file's
  own header is already honest that for the five off-canonical targets `refreeze` *copies* the
  table header into `anchor_end` (`refreeze.rs:213-215`), so the comparison is **temporal, not
  independent** — it catches a freeze taken against a different table state, and is not a second
  witness to the address.
* **(c) `canonical_pins_agree_with_the_canonical_size_tables` (`:141`)** — P4 vs P3, the only
  genuinely cross-tool comparison in the file.

**The order that defeats all four at once:** move `EndOfRom` in the aeon source, rebuild the aeon
ROM, and run none of P2/P3/P4/P5. Every artifact these tests compare is the previous freeze's,
mutually consistent and collectively stale. All four go green while the shipped source's
assembled bar has moved. **Nothing in this file reads a byte of `$AEON_DIR`** — which is the
property that makes it never skip, and the property that makes it blind here.

### Tier 3 — safe by construction, and therefore blind

#### E8 · `refreeze --check` cannot be red about a stale tree

*Mechanism M5, in its degenerate form.* `--check` reads `golden/.freeze-journal` first
(deliberately — a killed run would make any verdict a true statement about the wrong subject),
then `provenance.toml`, then the seven blobs, and asserts the chain is well-formed and that the
blobs recompute to the tip at the tip's **recorded** `anchor_end`. It never reads `pins.rs`, the
size tables, `$AEON_DIR/*.bin`, or `$AEON_DIR/*.lst`.

Its two products are steps 1 and 4 of one `refreeze --freeze`, and step 4 computes its numbers
*from* step 1's output (`refreeze.rs:1274`). **Tip-match is a tautology over one invocation's own
output.** There is no order of invocations that makes `--check` red about a stale tree, because
there is nothing outside that freeze's output for it to disagree with. A tree where nobody has
run anything for months is permanently green. That is correct behaviour for what it claims and
worth writing down beside it, because "refreeze --check green" is quoted as a landing witness.

The same shape, smaller, in `provenance_chain_holds`: `check` takes `anchor_end` from the chain
(`provenance.rs:819`), never from `pins.rs` or the tables, so a P3/P4 drift *after* a freeze is
structurally invisible to it. `offcanon_assembled_bar` (c) is the only independent cover.

And in `freeze_journal.rs`: `STEPS` (`:87-120`) **names** all 15 artifacts, and `fresh()` /
`stale()` (`:334`, `:348`) partition them purely by recorded step keys. **The module never stats
or reads a single artifact it names.** A journal claiming four completed steps is accepted with
zero filesystem corroboration, and `close()` deletes the file — so a freeze that *completed*
while its steps saw different trees (E1) leaves no trace at all. The journal covers kills, not
incoherence, and says so.

#### E9 · Four assertions where one product BOUNDS another

*Mechanism M6.* These are the only four sites in `sigil-cli/tests` where a single assertion
consumes two products from different invocations; everywhere else the products are split across
independent tests.

| site | bound | bounded | defeating direction |
|---|---|---|---|
| `native_full_rom.rs:258,266-273` | `pins::ASSEMBLED_LEN` (**repin**) | `$AEON_DIR/s4.bin` prefix (**`./build.sh`**) | a stale-SHORT pin compares a shorter prefix and still passes |
| `compression_selftest_port.rs:202-209` | `pins` base/len (**repin**) | `s4.debug.bin` window (**`DEBUG=1 ./build.sh`**), built against `compression_vectors.emp` (**`gen_compression_vectors.py`**) | as above, plus a third producer on the subject side |
| `native_offcanonical_full.rs:130` | `provenance.toml` `want_len` (**refreeze**) | a ROM laid out from the size table (**derive_offcanon**) | a freeze whose ledger was written against a different table state |
| `native_offcanonical_placement.rs:225-237` | `provenance.toml` `anchor_end` (**refreeze**) | `golden/config_b.bin` prefix (**capture_goldens.sh**) | a stale-SHORT `anchor_end` narrows the control window; the guard is `golden.len() >= eor`, which only catches too-LARGE |

None can be made to pass on *wrong* bytes; each can be made to pass on *fewer*.
`native_offcanonical_placement.rs:206-216` already carries a long comment on why `eor` comes from
the provenance tip rather than from a literal or from `golden.len()` — both cheaper sources
rotted once each. The residual asymmetry is that it guards one direction of two.

### Tier 4 — the producers and the A/B protocol

#### E10 · A `FAST=1` inherited from the operator's shell silences the capture's own banner

`capture_goldens.sh`'s `capture()` runs `env "$@" SIGIL_EMIT=… ./build.sh "$game" >/dev/null`.
`env` does not clear the environment, and aeon's `build.sh` takes `FAST="${FAST:-0}"` from it.
`FAST=1` skips every aeon verification lane — including build.sh's own
`${ROM_NAME}.lst`-was-produced check at `build.sh:632` — and prints a loud banner saying so at
both ends. **That banner goes to `/dev/null`.**

The bytes are unaffected (aeon states a FAST ROM is byte-identical, and the golden CRC bar would
catch it otherwise), so this does not corrupt a golden. It corrupts the record: a freeze can be
captured with every aeon verification lane skipped and nothing anywhere says so. `report()` then
computes `EndOfRom` from whatever `s4.lst` is present, and `capture()` deletes the ROM before
building but **not** the listing — so on a repeat run that listing is the one `lean` wrote last.
The script already knows the hazard: its restore block does
`rm -f "$AEON"/{s4,s4.debug}.{bin,lst}` and explains that a leftover off-canonical listing is
read as the canonical shape's. The same reasoning is not applied before each capture.

#### E11 · `region-hash.sh --diff` reports IDENTICAL as the PS class's passing verdict

*Mechanism M5.* `golden/ab/region-hash.sh --diff <old> <new>` compares two capture files, each
from a separate oracle run against a separately-loaded cart. **Nothing in the script or in
`AB_PROTOCOL.md` step 3 witnesses that the second run used a different ROM.** If the cart swap
did not take, both captures come from one cart, the diff says `IDENTICAL`, and for the **PS**
class — pure-size / value-identical — `IDENTICAL` is exactly the evidence the bar asks for.

#### E12 · The A/B cart-identity check is a SIZE check, on the class where sizes are equal

The later runners (`ab_wavec_scroll.py`, `ab_wavec_ramdiff.py`, `ab_wavec_state.py`,
`ab_collision_state.py`, `ab_g9_state.py`) are documented as *"already booted via
`ab_current.bin` content-swap or an MCP reload; SKIP_RELOAD identity-checks the booted cart
against the target size."* The check is:

```python
want = os.path.getsize(ROM); want += want % 2
r = await call(bus, "read_memory", {"addr": "0x1A0", "len": 8})
assert int(r["bytes"][8:], 16) + 1 in (want, want - 1), "wrong ROM booted"
```

It compares the booted cart's ROM-end pointer to the file's size. **The parcel class it guards
is the one where OLD and NEW have the same size** — the A3 pathfinder was `bsr.w`→`bra.w`,
explicitly size-neutral. The instrument does not touch the property. E11 and E12 compound: the
swap is out-of-band, the swap witness is vacuous for the class, and the verdict for that class
is "no difference."

Remedy is cheap and already in the bus surface: hash the ROM region (`emulator_memory_hash`), or
read the parcel's own known-changed offsets, which every byte-changing parcel already produces
from `cmp`. **Not attempted here — it needs the emulator, which is off-limits from a background
agent. TAGGED for foreground follow-up.**

---

## 4. Safe by construction — the results that came back clean

These are measurements, not absences of findings.

**`repin` does not read a listing file at all, and derives both shapes in one process.** This is
the direct analogue of aeon's defect and it is structurally absent. `repin.rs:104-125` resolves
`native::sigil_native_symbol_listing(&aeon, false)` and `(&aeon, true)` — plus phase-bank LMAs,
section extents and label owners per shape — in a single invocation from aeon SOURCE, the asl
`.lst` parse having been retired at Stage-3 P4c. Two shapes, one process, one source snapshot.
No ordering can leave one shape's input stale relative to the other's, because neither is a file.

**Every listing consumer pairs a listing with the ROM from its own invocation.** There are
exactly two `listing_symbol_addr` call sites in the tree (`rings_port.rs:366`,
`test_p1_player_port.rs:335`) and both take the shape-matched sibling. `test_p1_player_port`'s
`SonicShape` struct says so in the type — *"the reference ROM and its sibling sigil-canonical
listing — the same build, so a seam symbol's address here and the operand encoded in the window
cannot disagree."* `m1b_gate` reads `s4.lst` and `s4.bin` together, both plain. Every other
`.lst` spelling in the test tree is prose. **No gate in sigil reads two listings.**

**The dominant port-gate shape is fail-loud on staleness.** 54 files share one form: one test per
shape, reading exactly one aeon ROM, with `pins::<REGION>.{plain,debug}_{base,len}` locating the
window (`hblank_port.rs:307-311` is the template). That is two products across an invocation
boundary and the largest multi-product population in the repo — but the products sit on
**opposite sides** of the comparison. The subject is compiled from today's source; the reference
is the ROM. Staleness in either makes them disagree. Across all 54, no single assertion mixes the
plain and debug shapes.

The residual, stated because it is real: if the *whole* reference tree is one consistent old
snapshot — source, ROM and pins all from the same past — everything is green and the claim
"sigil reproduces aeon's current bytes" is false. That is E4/E5.

**The golden-blob gates read one capture run.** Ten files (`boot_port`, `math_port`, `mt_port`,
`sfx_port`, `seam1_native_link` and the five `seam2_*` co-links) read `golden/s4.bin` and
`golden/s4.debug.bin`; both come from one `capture_goldens.sh --write`, so they cannot be stale
relative to each other. `seam1_native_link.rs:47-49` states why the golden and not `$AEON_DIR`:
*"post-flip `aeon/s4.bin` is itself sigil-built, so composing `.emp` and comparing to it would
be circular."*

**`native::ensure_generated` regenerates the sound blobs in-process before every native build.**
`native.rs:1148-1158` re-emits the seam-1 resident blob and all six seam-2 artifacts into
`$AEON_DIR/engine/sound/generated` at the start of the build that consumes them, so a stale P8
blob cannot survive into a ROM comparison. Every apparent P8 "read" in the test tree
(`mt_bank_port.rs:136-138`, `dac_port.rs:194,202`, the `seam2_*` co-link temp reads) is reading a
file the same process just wrote. `nightly_source_gates.sh` deletes both generated trees for the
same reason and says why: *a stale generated file is byte-indistinguishable from a correct one.*

**`capture_goldens.sh`'s `report()` pairs one build's ROM with the same build's listing.** aeon's
`build.sh:621` emits `${ROM_NAME}.bin` and `${ROM_NAME}.lst` from ONE `sigil build` invocation
and hard-fails at `:632` if the listing is missing. One invocation, both products. The asymmetry
— the ROM gets an mtime guard, the listing gets nothing — is exploitable only through E10.

**`refreeze --attest`'s witness files are same-process.** P6 (`witness-*.txt`, `suite-*.log`) is
written at `refreeze.rs:723-724` and read back at `:776`/`:478` inside the same run. And
`strict_census` derives its expectation from **source** (`strict_census.rs:312-340`), so the
population it diffs the witness against cannot be edited into agreement.

**Nine of the 21 harness tests, and four of the six harness modules, read no build product at
all** — `act_fixture_drift`, `banked_carrier_drift`, `error_handler_island_order`,
`golden_freeze_atomicity`, `harness_root_handover`, `skip_marker_lint`, `strict_census_lint`,
`supersede_fixpoint`, and the `rev_reachability` / `strict_census` / `contract_baseline` /
`emit_sound_blob` code. `golden_freeze_atomicity` drives the real `atomic_freeze.sh` against
scratch blobs and never touches a committed one; `supersede_fixpoint` drives the real
`provenance::freeze_into` against a scratch chain. Both are the correct construction for a gate
whose subject is a ritual.

**`golden/PROVENANCE.md` and `golden/ab/**` have no reader in the crate.** The `ab` field is run
only through `provenance::fault_in_prose` (`provenance.rs:891`), which checks for control
characters and never resolves a path. E11/E12 are therefore findings about the *protocol*, which
is what the chain's `--ab` reference points at, not about an automated gate.

### Two doc-vs-behaviour mismatches found in passing

* `shipped_shapes.rs:16-17` says *"Reference-free: this reads profile literals, not the aeon
  tree, so it runs everywhere."* True of the aeon tree, **false of the golden tree** —
  constructing the seven profiles reads seven committed build products, and a missing one panics
  with `read frozen table …: No such file`, a hard failure unrelated to the claim.
* `harness_root.rs:50`'s `ROOT_MARKERS` probe on `golden/provenance.toml` is
  **existence-only, never content**. A zero-byte or corrupt ledger is accepted as a valid harness
  root, so `refreeze`/`repin` adopt the tree and fail later with a message about the ledger
  rather than about the root.

---

## 5. What enforces ordering today

| Mechanism | Covers | Does not cover |
|---|---|---|
| `refreeze --freeze`'s literal step sequence (`freeze_steps`, `refreeze.rs:1250`) | order **within** one full-ritual run | a tree that moves between steps (E1); any step run standalone |
| `refreeze.rs:1554-1590` — a **source-text** gate asserting the join spellings and that `authoritative_anchor_ends` contains `join("src/pins.rs")` | the step wiring not being silently rerouted | anything about the artifacts themselves |
| `golden/.freeze-journal` + `--check`/`--attest` refusing over a leftover | a *killed* `refreeze` | a hand-run `capture_goldens.sh --write` (unjournaled — a stated residual hole); a *completed* incoherent freeze, whose journal is deleted |
| `atomic_freeze.sh` staged commit + `.committing` marker | a kill inside the seven-blob commit loop | ordering between steps |
| `resolve_aeon_rev` (set, git, HEAD, **clean**) | the tree's state at step 0 | its state at steps 1-3 (E1) |
| `capture_goldens.sh` internal order, with `rm -f` of both ROMs *and* both listings before the restore | clobbering between shapes within one capture | the listing state before the FIRST capture of a run (E10) |
| `capture_goldens.sh:96,114` mtime markers | the ROM being stale-captured | the listing; every other producer in the repo |
| `provision-aeon-ref.sh` sequence | placing then building the reference tree | whether the build ran (E5) |
| `provenance_chain::aeon_dir_matches_the_provenance_tip` (strict only) | the tree's git revision vs the frozen tip | whether that tree's ROMs were built from it (E4) |
| `nightly_source_gates.sh` — scrubs `*.bin`/`*.lst` and both generated trees, and refuses to run if any aeon-reading test is unclassified | keeping the source lane provably source-only | the artifact lane, by design and by an explicit written argument |
| `offcanon_assembled_bar` | `assembled_end` across three tools | `golden_crc32` / `assembled_anchor` (E2) |
| git tracking of P2/P3/P4/P5 | a partial regeneration shows in `git status` | a byte-neutral freeze, indistinguishable from one that never ran |

---

## 6. Open, and what would settle each

* **Whether aeon's `./build.sh` can exit 0 without writing its ROM.** E5's severity turns on it.
  Settling it means running a build in a doctored tree. **BLOCKED — a live freeze holds this
  repo's goldens and the brief forbids building or relinking.** The finding stands regardless:
  the control is circular *by construction*, which is a property of the script rather than of
  how often the failure fires.
* **Whether the oracle's cart swap can silently not take (E11/E12).** Needs the emulator.
  **TAGGED for foreground**, never attempted from here.
* **How wide the E1 window actually is in practice.** It depends on whether freezes are in fact
  run from a dedicated worktree, which is a ritual question for the owner and the aeon lane, not
  a source question. The source answer — the check is taken once — is settled.

---

## 7. Where sigil's shape differs from the brief

The brief relayed aeon's pattern as *"a gate reading two listings produced by separate build
invocations"* and asked whether sigil's freeze rituals lean on gates of that shape.

**They do not, and the reason is specific rather than lucky** — see §4's first two paragraphs.
Sigil has no two-listing gate to defeat.

Three ways the real shape differs from the description, each worth stating so a second pass does
not go looking for the wrong thing:

1. **The worst instance is a PRODUCER, not a gate.** E1 is `refreeze --freeze` itself. No gate
   is defeated by an ordering there; the artifacts are made incoherent *at production*, and
   every gate afterwards is correctly green about an incoherent set. Sweeping gates alone would
   have missed it.
2. **The second-worst instance is a MISSING gate whose input already exists.** E2's evidence —
   `golden_crc32` beside every table — has been written on every freeze for as long as the tool
   has existed, and read by nobody. An enumeration keyed on "gates that read two products" finds
   this only if it also asks what each product *carries* and who consumes it.
3. **E5 is not two products stale relative to each other; it is a control comparing a product to
   a copy of itself.** Same family — a clean result that does not distinguish *the property
   holds* from *the property was not exercised* — but the defeating condition is not an ordering
   between two builds. It is one build not happening, inside a script that pre-populated its own
   comparand.

One thing the brief's frame got exactly right and which the sweep confirms from the other side:
the safe cases are safe for a *stateable* reason, and stating it is worth as much as the
findings. Every safe verdict above names the mechanism — one invocation, opposite sides of a
comparison, or in-process regeneration — rather than reporting an absence.

---

*Related, and this note is the build-order specialisation of both:
`docs/superpowers/notes/2026-08-27-absent-and-silent-are-one-artifact.md`, and the two
2026-08-30 sections in `docs/OVERSEER.md`.*
