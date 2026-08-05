# z80-parity packet (2026-08-05)

Three Z80 checker holes closed + the corpus contracts they ratify corrected +
housekeeping. **Byte-neutral by design** — all seven golden targets byte-identical
before and after; the fixes are contract/checker-level only.

Base: sigil master `e50146bf`, aeon master (b5 branched from master). Strict base =
3350 `#[test]`; this parcel adds 11 (all named below) → 3361.

---

## Fix 1 — the falls_into tail blind spot (4th mnemonic-table defect instance)

**Hole.** `z80_preserves.rs` checkpointed a `falls_into` proc's `FallOff` exit as a
plain return WITHOUT consulting the declared successor's contract. A proc that ends
by falling into a tail which clobbers rN still passed `preserves(rN)`. The explicit
`jr`-tail path (`Edge::Defer`) charged the tail callee correctly; `falls_into` did not.

**Fix (as ruled — thread the successor, apply Defer semantics; NOT a blunt refuse).**
`check_z80_preserves` now passes `proc.falls_into` into `verify_z80_preserved`. The
`Edge::Return | Edge::FallOff` arm is split: `Return` still checkpoints as an exit
(an early `ret` before the tail); `FallOff` in a proc that HAS a `falls_into` applies
the exact `Edge::Defer` logic — rN survives iff it holds its entry value AND the
declared successor itself preserves it. The successor is also folded into
`ever_clobbered` (mirrors the `jp`/`jr` external-transfer marking), so a `falls_into`
proc is semantically identical to an explicit `jr` tail. The blunt refuse-on-falls_into
shape was NOT taken — it would false-fire five currently-true contracts
(`PsgVolEnv_Resolve preserves(c)`, `Snd_AckSlot`/`Tempo_Ramp`/`Seq_RekeySingle`,
SndDrv_Init's chain).

**Regression pins** (`z80_contracts.rs`):
- `z80_falls_into_tail_clobbering_successor_fires` — P preserves(b), never touches b,
  falls into S whose tail writes b (`ld b, a`) → NotPreserved.
- `z80_falls_into_tail_preserving_successor_holds` — positive twin; S also
  preserves(b) → Verified.

**The measured RED window** (new-sigil + old-corpus, the live witness on the real
corpus, captured BEFORE the corpus correction):
```
engine/sound/sound_psg.emp lower errors: [Diagnostic { level: Error,
  message: "[proc.preserves-unverifiable] `Psg_EmitDivisor` declares `preserves(b)`
  but `b` is written and not restored", primary: Span { start: 26112, end: 26607 } }]
ERROR: sigil emit_sound_blob failed — cannot build the resident sound blob.
```

**Corpus correction (aeon).** `Psg_EmitDivisor` at `sound_psg.emp:430`:
`clobbers(a, c, f) preserves(b, de, hl)` → `clobbers(a, b, c, f) preserves(de, hl)`
(b dies in the tail's data-byte split, `ld b, a` in `Psg_EmitDivisorTo`). Header
comment rewritten to a present-tense contract fact (no change-history narration). All
callers already declare clobbers covering bc → no downstream ripple (confirmed: the
harness closure stays quiet, all 5 `z80_clobbers_incomplete` tests green).

**Post-fix confirmation (corrected — see fix-up round I2).** The corpus lowers clean.
The `falls_into` seams the fix GENUINELY exercises (non-empty checked set) prove
through the successor contract: `PsgVolEnv_Resolve→VolEnv_ResolveScan` (checked {c, ix}
— psg's module invariant), `Fm_NoteOn→Fm_NoteOnFreq` (checked {ix} — fm's invariant),
`Snd_AckSlot→Snd_AckBump` (checked {bc, de, hl, ix, iy} — explicit preserves). Two
seams are **checked by nothing** and are NOT a proof of the fix: `SndDrv_Init→SndDrv_Idle`
and `Sequencer_Channel→Sequencer_NextOpcode` — their modules (`engine.z80_sound_driver`,
`engine.sound_sequencer`) carry NO `invariant`, and the procs declare no `preserves`, so
the checked set is empty and `check_z80_preserves` returns before the proof runs. The
synthetic pins are the mechanism's proof; the three invariant/preserves-bearing seams are
its live witnesses.

---

## Fix 2 — Z80 `out(carry:) ∩ preserves(f/af)` overlap (latent hole)

**Hole.** The Z80 branch of `check_out` (`lower/proc.rs`) returned before the
flag-result checks, so `out(carry: x) preserves(f)` (or `af`) compiled silently.

**Fix.** In the Z80 branch, when `!proc.out_flags.is_empty()` and the expanded
preserves set contains the `f` unit (covers both `f` and `af` — `af` expands to
{a, f}), error `[proc.out-preserves-overlap]`, naming the flag result and the
preserves token (`f` or `af`) that covers the flags.

**RULING encoded (not "fixed").** `clobbers(f) + out(carry:)` REMAINS LEGAL on Z80 —
deliberate divergence from the 68k sr.ccr rule. Z80 has no finer-than-`f` token, so
clobbers-covering-`f` is the only honest spelling of "flags are scratch except the
carry result" (9 of the corpus's 10 carry-returning procs). The divergence is stated
at the check site and here.

**Corpus impact: NONE.** No corpus proc declares both a flag result and preserves(f/af)
(the out(carry:) procs preserve `c` or `bc,ix,iy`, never the flags). A pure
latent-hole close.

**Pins** (`z80_contracts.rs`): `z80_out_carry_preserves_f_overlap_fires`,
`z80_out_carry_preserves_af_overlap_fires`, `z80_out_carry_preserves_a_no_overlap`
(negative), `z80_out_carry_clobbers_f_stays_legal` (the ruling).

---

## Fix 3 — z80_writes models no F-writes (preserves(f) false-pass hole)

**Hole.** `z80_writes` returned `f` for NO instruction, so `preserves(f) { scf; ret }`
verified.

**Fix (conservative-invert polarity, as ruled).** `z80_writes` now appends the `f`
unit as the COMPLEMENT of a flag-neutral allowlist (`z80_flag_neutral`): an
instruction writes `f` UNLESS provably flag-neutral (`ld`/`push`/`pop`/`ex`/`exx`/the
control transfers/`set`/`res`/`out`/`nop`/`di`/`ei`/`halt`/`im`; 16-bit `inc`/`dec`
of a PAIR only). Sound direction — a forgotten neutral over-fires visibly rather than
false-passing. Shares nothing with `flag_check.rs`'s CARRY model (carry ⊂ f; a single
carry table would miss `inc`/`dec`/`bit`, reopening the hole). The detector stays
shared between the proof and the closure (`z80_written_registers`), so they never drift.

`pop af`'s `f`-load stays modeled through the slot machinery (pop is neutral in
`z80_writes`), so `SndDrv_ISR preserves(af)` stays green via its push af/pop af bracket;
`ld (ix+d), 0` writes no register and is neutral, so `Psg_EnvCursorReset preserves(af)`
stays green.

**Corpus-wide firing enumeration (the mandated check): ZERO new firings.** After
Fix 1's correction unblocked the lower, all 5 `z80_clobbers_incomplete` tests pass and
the corpus lowers clean. The corpus is already flag-honest: every proc with a clobbers
contract that writes flags declares `clobbers(af, …)` / `clobbers(…, f)`, and the only
scratch-writing procs WITHOUT a clobbers contract (`Snd_DacLookup`, `Snd_RouteClassFlags`)
are closure-exempt (`has_clobber_contract = false`). The warning heuristic's Z80 write
detection is 68k-only, so no `proc.clobber-undeclared` warning moved either.

**Pins** (`z80_contracts.rs`): `z80_preserves_f_scf_body_fires`,
`z80_preserves_af_pure_ld_body_holds` (positive, memory-dest ld),
`z80_preserves_af_push_pop_bracket_holds` (positive, SndDrv_ISR shape),
`z80_preserves_f_16bit_inc_holds` + `z80_preserves_f_8bit_inc_fires` (the
operand-sensitive inc/dec split).

---

## Housekeeping (aeon)

- **`Snd_DacLookup`** (`z80_sound_driver.emp:736`): had `out(carry:) preserves(bc, ix, iy)`
  with NO clobbers clause. Honest set derived from the body (`a` + `de` index/offset
  math, `f` the id-validity flags + final `add hl` carry; `hl` is the out result;
  `bc`/`ix`/`iy` untouched): declared `clobbers(a, de, f)`. Verified complete by the
  closure. `clobbers(f) + out(carry:)` is legal per Fix 2's ruling.
- **Stale comment** (`z80_sound_driver.emp:733-735`): the "INBOUND TRUST CONVERSION:
  sound_sequencer.emp:101 declares `extern … preserves(ix)`" claim is stale —
  sound_sequencer.emp:90 now imports the real proc via `use engine.z80_sound_driver.{…}`;
  there is no conservative extern to reconcile. Rewritten to a present-tense contract fact.
- **Ledger** (`campaign-gap-ledger.md`): closed the "Z80 `dc`-label unprobed" half of
  the dc-label row — `PsgVolEnv_Ptrs` (`sound_tables_z80.emp:79`) is a live `dc.w <label>`
  pointer table in the shipped sound blob, byte-covered by the seven-target bar.

---

## Bars (with numbers)

- **Byte bar (seven targets, all EXACT):** s4.bin `c2d17ee3/411096` · s4.debug
  `6c296656/423480` · demo `4a09314e/91258` · demo.debug `f3e5ed3e/93955` · config_a
  `4e34a38a/423871` · config_b `b8cce891/301132` · lean `b92cb485/379110`.
- **refreeze --check:** OK (tip `sst-fold`, chain len 47). **repin --check:** pins.rs
  unchanged.
- **Strict suite (final, post-fix-up):** 3365 passed / 0 failed / 4 ignored = **3369**
  (= 3350 base + 19 new: 11 round-1 pins + 8 fix-up pins, all named below).
- **Warn tiers (no delta):** plain 19 (path-mismatch 9, undeclared-fallthrough 6,
  out-unwritten 3, clobber-undeclared 1) · s4-DEBUG 18 (fallthrough 5, else same). No
  touched proc appears in any warning; the Z80 write heuristic is 68k-only, so the Z80
  contract edits cannot move these 68k/data-oriented warnings.
- **Clippy (changed files):** z80_preserves.rs + lower/proc.rs clean. (One pre-existing
  `redundant_closure` in `mul_lower.rs:357` from the mul-lowering merge — not this parcel,
  out of scope.)

## Land-order (measured)

- new-sigil + old-corpus → **RED** at `Psg_EmitDivisor preserves(b)` (the witness above).
- old-sigil + new-corpus → **GREEN**: s4.bin builds clean, `c2d17ee3/411096` (identical).

The measurements confirm **AEON MERGES FIRST** (not sigil-first): the aeon corpus
correction is backward-compatible with master sigil, while new sigil rejects the
uncorrected corpus — so landing sigil first would red its strict gate against unpatched
aeon. Land aeon, then sigil.

## Per-pass split

- **Step-3 (language asks): none takeable.** Fix 2's Z80/68k `out(carry:)+clobbers`
  divergence is a documented design fact, not an ask.
- **Step-5 (engine/checker findings):** the F-write model shares one detector across
  proof + closure (no second flag table); the falls_into successor is now folded into
  both the proof's exit arm and `ever_clobbered`, matching explicit-jr-tail semantics
  exactly.
- **Neither-bucket headline:** the panel round turned this parcel's own neither-bucket
  findings into taken work — `Snd_RouteClassFlags` gained `clobbers(f)`, `Psg_EnvCursorReset`
  gained explicit `clobbers()` (closure-exemption → machine-checked proof), and every
  stale "INBOUND TRUST CONVERSION" twin the round-1 sweep left (`Snd_StartSample`,
  `Snd_RouteClassFlags`, the `Psg_EmitDivisor/Mod_ReArm de` comment) is now a present-tense
  contract fact. The recurring shape — a `use`-imported proc whose body-site comment still
  cites a conservative `extern` that the `use` migration deleted — is itself the headline.

## Fix-up round (three-lens panel, 2026-08-05)

A second commit on each b5 branch. Byte-neutral held (seven CRCs unchanged), strict
grew from 3361 → 3369 (8 pins), corpus new-firings for M2/M3/S3/A1 enumerated **zero**.

**MUST-FIX (Lens B):**
- **M1** — the empty-body `falls_into` bypass: `verify_z80_preserved`'s instruction-less
  early return returned Verified for every unit before the successor was consulted
  (`proc P() preserves(b) falls_into S {}` false-passed). Now it applies the successor
  oracle. Pins: `z80_empty_body_falls_into_clobbering_successor_fires` /
  `..._preserving_successor_holds`.
- **M2** — `djnz` writes its counter `b`; no `z80_writes_regs` arm modeled it. One-line
  arm. Pin `z80_djnz_writes_counter_b_fires`. Flag-neutrality of `djnz` left intact.
- **M3** — block transfers write their pointer/counter pairs; `ldir` (live: Snd_LoadSong,
  Snd_StartSample) writes bc, de, hl. `ldir` is the ONLY assemblable block op in the ISA
  `Mnemonic` enum (checked: no ldi/ldd/lddr/cpi/cpd/cpir/cpdr) — modeled exactly, with the
  LD-block/CP-block siblings' write-sets noted for the day they become assemblable
  (CP-family writes bc+hl, not de). Pin `z80_ldir_writes_hl_fires`.
- Post-M2/M3 corpus enumeration: **zero** new firings. `Sfx_SlotPtr` is the witness —
  its `djnz` writes b INSIDE a `push bc`/`pop bc` bracket (preserves(bc) still holds), and
  both its exits (early `ret z` before the push; final `ret` after the balanced pop) see an
  empty stack, so S3 does not bail.

**SHOULD-FIX (Lens B):**
- **S1** — the Z80 out-flag rule gated only on `out_flags`; a conditional register result
  `out(rN if cc)` reads its guard from the flags too. Added `|| !proc.out_cond.is_empty()`
  (68k precedent). Pins `z80_out_cond_preserves_f_overlap_fires` (+ negative
  `z80_out_cond_preserves_b_no_overlap`). Ledger row added: Z80 has no cc vocabulary —
  `VALID_CCS` accepts 68k spellings regardless of CPU (deferred).
- **S2** — three round-1 assertions used `d.contains("b")`/`contains("f")`, which the word
  "unverifia**b**le"/"unveri**f**iable" satisfies vacuously. Retargeted to the delimited
  `"preserves(b)"`/`"preserves(f)"`/`"preserves(h)"` text. (Pre-existing `contains("h")` at
  :242 is NOT vacuous — "unverifiable" has no `h` — left untouched.)
- **S3** — an exit reached with a live tracked push slot (cross-seam push-here/pop-in-
  successor) is now a conservative BAIL (Unverifiable — the local model cannot pair the
  push). Corpus enumeration first: **zero** procs fire (verified — every corpus proc
  balances its stack before every exit; Sfx_SlotPtr above is the closest shape and is
  clean). Pins `z80_exit_with_live_slot_bails` (+ positive `z80_exit_with_balanced_stack_holds`).

**Lens A + C (all taken):**
- **A1** (aeon micro-fixes, byte-neutral): `Snd_RouteClassFlags` gained `clobbers(f)` (its
  two `cp`s write flags); `Psg_EnvCursorReset` gained explicit `clobbers()` (closure
  exemption → machine-checked "touches nothing"); three stale/false comments rewritten —
  `Snd_StartSample`'s "INBOUND TRUST CONVERSION" twin, `Snd_RouteClassFlags`'s (citing a
  `sound_sfx.emp:85` extern that is actually a `const` line, contradicting the file header),
  and `sound_psg.emp:330` ("Psg_EmitDivisor/Mod_ReArm clobbered de" — both PRESERVE de).
- **A2** (sigil comment narration): dropped the "old/was never consulted" history voice at
  `z80_writes`'s `f` comment and the FallOff-arm comment; dropped the "(adjudicated)"
  process tag at the check_out divergence comment (kept the design fact). The pre-existing
  Defer/gap-3 comment left untouched.
- **A3** — reworded the `ld a,i`/`ld a,r` comment: the ISA enum has only `LdIA`/`LdRA`
  (write i/r, neutral); the flag-writing READ forms are unassemblable today and MUST join
  the F-writer set the day `LdAI`/`LdAR` land. Ledger row carries that kill condition.

## Lens findings and dispositions

| Lens | Finding | Disposition |
|---|---|---|
| B | M1 empty-body falls_into bypass | FIXED (early-return successor oracle) + 2 pins |
| B | M2 djnz unmodeled b-write | FIXED (arm) + pin; corpus zero-firing |
| B | M3 block-transfer unmodeled writes | FIXED (ldir, the sole assemblable) + pin; corpus zero-firing |
| B | S1 out_cond bypasses flag rule | FIXED + 2 pins + ledger row |
| B | S2 vacuous register-specificity asserts | FIXED (delimited text) |
| B | S3 exit with live slot | FIXED (conservative bail) + 2 pins; corpus zero-firing |
| B | I2 round-1 "genuinely proven" overclaim | CORRECTED (checked-by-nothing list; see Fix 1 confirmation) |
| A | A1 corpus clobbers/comment micro-fixes | FIXED (byte-neutral) |
| A | A2 comment history-voice | FIXED |
| A | A3 ld a,i/ld a,r comment/code mismatch | FIXED + ledger kill-condition |
| C | (no separate C-only finding beyond A/B above) | — |

## Base note

The aeon `z80-parity` branch is based on aeon master `9a5df9e` (which includes the
`docs: §9.7 deferred-work research corpus` rider on top of the `5b736a5` mul-lowering
merge). The sigil `z80-parity` branch is based on sigil master `e50146bf`.

## Commits (merge queue — AEON FIRST, then sigil)

- aeon (branch `z80-parity`, base `9a5df9e`):
  - `18c1d35` sound contracts: Psg_EmitDivisor charges b to its tail, Snd_DacLookup
    names its scratch.
  - `768940b` sound contracts fix-up: Snd_RouteClassFlags names its flag scratch,
    Psg_EnvCursorReset proves it touches nothing.
- sigil (branch `z80-parity`, base `e50146bf`):
  - `fe935e12` z80 parity: the falls_into tail is charged, F-writes modeled,
    out(carry:) refuses preserved flags.
  - `c7109635` z80 parity fix-up: the empty-body tail is charged, djnz/ldir write
    their registers, out_cond demands the flags, a live slot at exit bails. (This hash
    refreshes when this commit-list edit is amended in.)
