# sigil lens sweep — closeout

Companion to `2026-08-13-sigil-lens-sweep.md`. Every finding in that packet is
closed except one, which is ledgered as deliberately deferred.

Landed across four merges on master — `6a6146e5`, `cc01c59b`, `93a389f1`,
`dfdbdb21` — plus `49609913`. Pushed to `origin/master` 2026-08-14.

## Closed

| # | Seat | What landed |
|---|---|---|
| S1/S2 | CGa | EA-class model in `sigil-isa`. Four exact-alias miscompilations (`bset d0,a1`→`movep.l`, `sne a3`→`dbne`, `eor.w d0,a1`→`cmpm.w`, `pea d0`→`swap`) plus ~25 illegal forms, the An-has-no-byte-form rule, `encode_alu_ea` validating its destination as a source, and `movem` applying one direction's modes to both. |
| S4 | Vb | Z80 `clobbers(...)` undeclared-write check implemented. The driver was already honest — 0 firings across all three modules — so aeon's "machine-checked" claim is now true rather than merely asserted. |
| S5 | CGb | The eight half-built Z80 primitives (`rst` + 7 siblings), cross-checked against oracle-next's SingleStepTests-verified CPU core. |
| S6 | RELAX | The seven unpinned exact reach boundaries. |
| S7 | IR | `emit_fragment` derives its cursor advance; cross-crate agreement test. |
| S9 | LINK | `banked_carriers` cross-checked against the seam-2 derivation on every emission. |
| S12 | Va | Dead-save advice no longer rests on the unverified `as Type` bound. Partial by design — see below. |
| S13 | HALF | `patch`/`bind` stopped being a silent no-op; `@scaffolding`'s doc stopped implying a lint reads it. |
| S14 | ERR | The contract gate resolves a new firing to `file:line:col`. |
| S15 | GATE | Empty-window pins closed in all 41 port helpers; CI counts and reports its skips (317) and states that a green badge carries no whole-ROM byte evidence. |
| S16 | B2 | The `Act` supply fixture answers to `harvest_engine_struct_offsets`, on values AND field coverage. |
| S17/S22 | P1a/CACHE | Const-fold memoised. 2.66s → 0.68s, 3.9×. |
| S18 | FUZZ | The zero-progress parse loop fixed structurally, not per-keyword. |
| S19 | SAFE | Both uncatchable SIGABRT aborts closed, plus an array-literal shape the packet did not name. |
| S20 | TEST/A2 | The whole-ROM gate no longer claims to compare against live `asl`. Corrected at all four homes, README included. |
| S8 (part) | ARCH | salvador vendoring provenance recorded, with the unknown upstream hash flagged rather than invented. |

S3 was superseded by S17 in the packet itself; S10/S11 were answers, not defects.

## Not closed, on purpose

**S8, the `sigil-harness` crate split.** A ~9,300-LOC multi-crate move for a
build-time and architecture win, with no byte-level gate that would catch a
subtle mis-split — the ROM would still build. Ledgered in
`docs/superpowers/notes/campaign-gap-ledger.md` with ARCH's measurements. It
wants owner review of the seam choices, not an unattended night.

## Partial, and why

**S12** keeps the trusting closure for the warn-tier analyses. Neither remedy the
packet offered is reachable as-is: wiring `subcontract_violations` at the
dispatch site needs a target set the language cannot express there (every corpus
`targets(...)` is an intra-proc label table), and dropping the narrowing costs
**53 `[proc.clobber-undeclared]` firings** — 53 engine contracts are written
against the narrowed answer. Ledgered with that number so the next attempt starts
from it.

## Things the sweep did not know

- **CI's clippy step was already red**, from a toolchain update, at the sweep's
  own pinned review SHA. Seat B1's "clippy nearly silent across 188k lines" was
  true of the code; the GATE was broken. Fixed.
- **`OJZ_BG_ANIM` was worse than reported**: its plain window is 14 bytes, the
  module emits 2, and both of those are zero — the pin constrained nothing in
  either direction.
- **A second abort shape**: array-literal nesting, alongside the bracket-index
  one S19 named.
- **The aeon Z80 driver was already contract-honest**, which the missing checker
  had made unknowable.

## Corrections to my own work, recorded so they are not repeated

- The first `Act` comparator assumed hex throughout; the fixture spells values in
  BOTH hex (`"$0A"`) and decimal (`"10"`), so it reported `DMAEntry` and six
  `parallax_config` fields as drifted when only `Act_len` had moved.
- The first `CONTROL` EA set forgot that the 68000 manual counts `#imm` among the
  MEMORY modes, leaving `lea #$1000,a0` encodable. The over-strictness guard
  written for the opposite reason is what caught it.
- Two probes looked like proof and were not: deleting both of `bg_anim.emp`'s
  data items fails on master too, for an unrelated reason (no section at all);
  and `emit_sound_blob` reports no warnings at all, so a clean run through it
  says nothing about a lint.

## Verification at closeout

```
sonic4 plain   crc=fedcf197 len=696836   (== frozen golden)
sonic4 debug   crc=3dc20e2c len=711298   (== frozen golden)
refreeze --check                          OK (chain len 111)
workspace suite (AEON_DIR set)            3715 passed / 0 failed
CI simulation (AEON_DIR=/nonexistent)     3715 passed / 0 failed, 317 skips reported
clippy --workspace --all-targets -Dwarn   clean
```

Every new gate was proven to FAIL on the drift it exists to catch — the pin
against a zero-byte module, the carrier against its historical `0x856D`, the
reach boundaries against a one-off widening, the unreachable lint against a false
orphaned guard, the parser guard against its own removal.
