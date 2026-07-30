# 2026-07-30 — seam-2 stage-2b Option Y: the CO-LINK MECHANISM, settled empirically (probe + the enabling fixup arm)

Status: **EXECUTION Y-sequence step 1 — the co-link mechanism PROBED end-to-end
and SETTLED. The recon's TWO anticipated routes are BOTH unbuilt; the mechanism
that satisfies the ruling is a THIRD one, enabled by a single principled fixup-kind
arm (a correctness-guard relaxation mirroring the ratified t27 decision). TDD, GREEN,
zero-regression, provenance UNCHANGED.** Sigil branch `seam2-banked-data`. This is a
VALVE STOP before propagating the (guard-relaxing) mechanism through the DAC deletion
and stages 2c/2d/3 — the guard change wants the overseer's blessing at merge first.

## What the probe empirically settled (every recon open-question, recorded)

The probe (`crates/sigil-cli/tests/seam2_colink_probe.rs`) co-links the REAL
`dac_samples.emp` (placed at the current baseline $48000/$50000) with a synthetic
one-descriptor head module and tests each cross-module cell mechanism:

| cell (dac_sample_tab form) | width | cross-module result | fact |
|---|---|---|---|
| `dc.w SND_KICK_PTR` (winptr equ) | 2 | **RESOLVES** | equ→LinkExpr(winptr) folds to $8000; `Value16Le` |
| `dc.w SND_KICK_LEN` (comptime equ) | 2 | **RESOLVES** | equ→Int(1406); `Value16Le` |
| `dc.b SND_KICK_BANK` (bankid equ) | 1 | **WALLED** (before the fix) | width-1 Z80 SymRef → `[cross-cpu.unwindowed-pointer]` |
| `dc.b bankid(Dac_Kick)` (inline builtin) | 1 | **UNBUILT** | `[dc.operand]` parse; and `lower_dc` has NO `Value::LinkExpr` arm (`[dc.comptime-only]`) |
| `Dac_Kick.len` (cross-module .len) | — | **UNAVAILABLE** | `.len` is a comptime property of a same-module `Value::Data`; a foreign module cannot resolve it |

### The two recon-anticipated routes are BOTH unbuilt
- **Recommended route — `use data.dac_samples.{the 30 SND_*}` (equ import):** the
  lowerer does NOT load `use`d files (seam-1 only synthesizes proc STUBS from a
  hand-built table; there is no cross-file equ-definition import). `use module.{EQU}`
  imports nothing at lower time. NOT the mechanism.
- **Fallback — inline `bankid("Dac_Kick")`/`winptr("Dac_Kick")` in the cells:** in the
  entire `.emp` corpus `bankid`/`winptr` appear ONLY inside `ensure(...)`/`equ` — NEVER
  emitted into a data cell. `dc.b bankid(Dac_Kick)` doesn't parse as a call in
  dc-operand position, and even parsed, `lower_dc` (`eval/asm.rs`) rejects a
  `Value::LinkExpr` element (`[dc.comptime-only]`) — the "emission lowers to Cell::Expr"
  path (builtins.rs:479) is for TYPED `data` ITEMS, not `dc` code cells. NOT the mechanism.

### The mechanism that DOES satisfy the ruling (the third route)
`dac_sample_tab.emp` keeps its descriptor body **byte-identical** (`dc.b SND_KICK_BANK`
/ `dc.w SND_KICK_PTR` / `dc.w SND_KICK_LEN`); the 30 `SND_*` are sourced NOT from `-D`
but as **cross-module link symbols resolved against `dac_samples.emp`'s equs** in the
joint link. The `SND_*` equs fold SAME-MODULE in `dac_samples.emp` (`bankid`/`winptr`
from placement, `.len` comptime), so the head only RESOLVES them — no `-D`, no mirror,
the names live ONCE at the producer. This is exactly the ruling's §2d intent
("descriptor cells fold from placement, no -D, no 30-value mirror"), reached via the
proven cross-module SymRef path rather than the (unbuilt) in-cell builtin fold.

PTR (width-2) and LEN (width-2) resolve AS-IS. The BANK cell (width-1) was the sole
wall — see below.

## The enabling fix (the demanded feature — TDD, one arm)

`crates/sigil-frontend-emp/src/lower/data.rs::fixup_kind` had NO `(Cpu::Z80, 1, false)`
arm, so a width-1 Z80 SymRef fell to `[cross-cpu.unwindowed-pointer]`. Added:

```rust
(Cpu::Z80, 1, false) => Some(FixupKind::Value8),
```

This is the **width-1 sibling of the ratified t27 `(Cpu::Z80, 2, false) => Value16Le`
decision**. Rationale: a `bankid()` fold is a 1-byte bank ORDINAL, not a pointer; a
width-1 Z80 SymRef is the byte VALUE of the resolved symbol. The cross-cpu guard's
protection SURVIVES width-split: the linker's `write_value` does an unsigned u8 range
check, so a genuine un-windowed 68k address (> $FF) that reached here without folding
fires `[value.out-of-range]` at link (verified: SND_KICK_BANK=$0A passes; a $50000
pointer would not). Width-4 Z80 un-windowed pointers STILL error (arm is deliberately
width-split, unit-tested).

**This is a correctness-guard RELAXATION.** It is safe (see below) and precedent-
mirroring, but it is exactly the class of change the campaign ratifies at the gate —
flagged for the overseer's explicit blessing at merge.

## Proof

- **Probe** (`seam2_colink_probe.rs`, 2 tests, GREEN): the co-linked `SND_*` fold to
  the correct bytes in all three cell widths — kick $0A/$8000/$057E, blip $09/$8000/$0B40;
  and cross-module `.len` correctly errors (documents WHY LEN references the equ).
- **Unit** (`lower/data.rs`, 2 new tests, GREEN): width-1 Z80 SymRef → `Value8`
  (not the cross-cpu error); width-4 Z80 SymRef → still the cross-cpu error.
- **Zero regression:** the arm made a previously-hard error into a valid emission; NO
  existing test relied on the width-1 Z80 error (grep-confirmed + full-gate green). The
  current `dac_sample_tab` build still uses `-D` (Cell::Scalar, not SymRef), so this arm
  is **byte-neutral to the real build** — `dac_sample_tab_matches_as_twin` unchanged.
- **Strict gate: 2888 passed / 0 failed / 1 ignored** (baseline 2884 + 2 probe + 2 unit),
  `SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/target/release/emit_sound_blob AEON_DIR=<aeon>`
  from the sigil worktree. Provenance UNCHANGED: assembled ROM e5765873/dab4f06c proven
  by `mixed_seam1_rom_matches_reference_{plain,debug}` + `mixed_dac_rom` (all green);
  blob c7534c84/fd2a845d, syms 87b87b1b, artifacts 22f69f77/414414 · d4e8d043/422466.

## The cascade (why settling this first matters for 2c/2d/3)

The same cross-module SymRef mechanism is what the rest of Option Y / stage 3 needs:
- **`dac_sample_tab.emp` (this stage):** drop the `-D`, co-link with `dac_samples.emp`;
  the head folds `SND_*` cross-module. The `dac_sample_tab_port.rs` `-D` oracle retires
  in favor of the co-link byte gate (the kickoff's `emit_dac_body_and_head` substrate).
- **`sfx_blob_win_tab` (stage 3):** `dc.w winptr(Sfx_NN)` — a width-2 case. Note it is
  `winptr` INLINE in a cell (the unbuilt in-cell-builtin path), NOT an equ ref, so it
  needs a DIFFERENT enabling step than the DAC head (either an sfx-side `SFX_WIN_*` equ
  layer to reference, mirroring the DAC's `SND_*`, or building the in-cell-builtin path).
  RE-PROBE before stage 3 — do not assume the DAC mechanism transfers.
- **`seq_opcode_tab` (stage 3):** already proven — width-2 `Value16Le` (OQ-5 / t27), no
  new mechanism.
- **`mt_bank`/`sfx_bank` (2c/2d):** bank BODIES (`bank: $8000`, m68000 data), not the
  Z80 head — the DAC-bank emit pattern (`emit_dac_banks`), unaffected by this arm.

## Next (the finisher's remaining Y-sequence — mechanism now known)

1. **(pending the guard blessing)** Convert `dac_sample_tab.emp`: remove the `-D` seam
   from `dac_sample_tab_port.rs`; extend the harness (`emit_dac_body_and_head`) to
   co-link `dac_samples.emp` + `dac_sample_tab.emp` and byte-gate the head against the
   reference `dac_sample_tab` slice (re-derive its exact ROM offset from `s4.lst` —
   `DacSampleTable` phase VMA → head bank $58000). Un-suppress `dac_samples.emp`'s
   per-sample start labels (OQ-4 — byte-neutral; the co-link is their first consumer).
   Dual-prove both shapes (the head is `-D`-shape-invariant, t24 control).
2. **The wire** (build.sh + BINCLUDE, per `2026-07-30-seam2-stage1-rebaseline.md`
   §"STAGE 2b READINESS") — DAC body + head together.
3. **THE DELETION COMMIT** — `dac_samples.asm` + `dac_sample_tab.asm` as a unit (rows
   5-dac + 57 same-commit), post-deletion assembled-ROM unchanged.
4. **2c/2d/3** as above → loop + C3-heavy dry panel → checkpoint (b).

## VALVE STOP — clean boundary

Stopped here: the mechanism is settled with a proven minimal fix, but that fix is a
correctness-guard relaxation that deviates from both routes the recon planned. Landing
it as a discrete, TDD'd, zero-regression committed step (the campaign's small-steps
pattern) and stopping for the overseer to bless the guard change is the disciplined
move — building the irreversible DAC deletion + 4 more stages on an un-ratified guard
relaxation is the rushed multi-round the kickoff's own valve-stop warned against. The
handoff is complete: every recon open-question is empirically answered, the mechanism
and its one enabling change are proven, and the cascade (incl. the sfx_blob_win_tab
re-probe caveat) is mapped. No push; the merge is the overseer's.
