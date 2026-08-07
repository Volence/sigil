# Lane A packet — mulw-parallax

Branches: sigil `lane-a` (3 commits), aeon `lane-a` (1 commit). **Not merged, not
pushed.** Baselines below were all measured this session.

## The correction that comes first

The brief called this site "BYTE-CHANGING and BEHAVIOUR-RELEVANT" and asked for
"an A/B designed to EXHIBIT a difference, not prove identity". **That framing is
wrong for this site, and the parcel is built on the corrected one.**

`band_entry` is 10 bytes today, and the hand-rolled chain
(`lsl.w #3 / add.w d5,d5 / add.w d5,d3`) computed ×10 **correctly**; the copy run
moved exactly 10 bytes. Nothing was mis-indexed. The defect is **latent, not
live** — an 11th field would have produced both a wrong-band read and a truncated
copy, and the byte gate is blind to both because the golden and the fresh build
would agree on the same wrong number.

So the A/B bar inverts on a different axis, and the note says so at the top: the
*instruction encodings* must DIFFER (that is what proves the change is live at
every site rather than the one the author inspected), observable state must
MATCH, and — the part that matters most for this class — the changed code must be
**proven to have executed**, because an all-green identity A/B over code that
never ran is indistinguishable from a correct one.

## What landed, and why it is one parcel and not two

The brief's Lane A is the aeon half. Building it surfaced that `mul_const.w #10`
lowers to a chain ending in `lsl.w #1,d3` (8 cycles) where `add.w d3,d3` (4) is
available at the same two bytes — so the derived spelling would have cost 2
cycles per band. That pessimisation is **already a known, OPEN gap-ledger row**
(ltr-mul panel, 2026-08-06), whose own kill condition reads "a byte-changing
parcel switches the k = 1 word run to `add.w d,d` and re-proves". This parcel is
one.

Taking it inverts the arithmetic: the derived stride becomes 26 cycles against
the hand form's 28. Landing the two apart would have moved the same
instruction's bytes twice and made the aeon commit alone read as a regression, so
they are one parcel with two commits.

* sigil `8e86aded` — `shift_run_word` mirrors `shift_run`: a remaining single
  double is `add.w d,d`. Reaches four corpus lowerings and nothing else.
* aeon `f3fe7a9` — parallax's stride is `mul_const.w d3, #sizeof(band_entry), d5`
  and its copy is a comptime fold whose total pointer advance IS
  `sizeof(band_entry)` — the same fact the following `-sizeof(band_entry)(a4)`
  rebase already depended on.
* sigil `0083a911` — goldens re-frozen to chain 52 with the A/B ref.
* sigil `c5bcd7a2` — ledger.

**Copy spelling, justified on measurement as asked:** a derived LOOP was rejected
without needing a cycle argument — a comptime-generated run is byte-identical to
the hand-written one at the current size (0 cycles, 0 bytes of cost) and grows
with the struct, so a loop would pay cycles for nothing. The "hot path" premise
also does not hold: `configs.emp` declares 1-, 2-, 4- and 5-band configs, so the
loop runs **at most five times per frame**, and neither the +2 nor the −2 cycles
per band was ever the deciding argument.

## The byte delta — complete, and every run named

All four canonical shapes carry **exactly the same seven differing runs** against
chain 51: the header checksum plus five instruction-encoding sites. **Every ROM
size and every golden `anchor_end` is unmoved**, so nothing was re-placed —
`repin` reported `pins.rs unchanged`, and the 5-site ripple therefore needed no
hand edits (`engine.inc` and `mixed_dac_rom.rs` are deleted from both repos;
`repin_pins.rs` tracks region addresses, which did not move).

| site (debug shape) | addr | OLD | NEW | cycles |
|---|---|---|---|---|
| `MigrateMasks.new_loop`+4 | `$004B22` | `E348` | `D040` | ×26: 38 → 30 |
| `MigrateMasks.new_loop`+$C | `$004B2A` | `E348` | `D040` | (same chain) |
| `TileCache_DecompressBlock.rr_ok`+$4A | `$0054C4` | `E34B` | `D643` | ×66: 32 → 28 |
| `Section_GetSecPtrXY`+$24 | `$0066F8` | `E348` | `D040` | ×66: 32 → 28 |
| `Parallax_Step4_Fill.copy_band` | `$006F22` | `3602 E74B 3A02 DA45 D645` | `3602 3A03 E54B D645 D643` | ×10: 28 → 26 |

## Gates — all own-run, none waived

* **Byte bar, seven targets** (list derived from `crates/sigil-harness/golden/`,
  counted this session: `s4`, `s4.debug`, `demo`, `demo.debug`, `config_a`,
  `config_b`, `lean`). Proven byte-identical to chain 51 BEFORE any edit (the
  worktree-seed proof), then re-captured and every delta named above.
* **Chain 52 frozen** with `--ab docs/superpowers/notes/2026-08-07-mulw-parallax-ab.md`;
  `refreeze --check: OK (tip mulw-parallax, chain len 52)`; `provenance_chain_holds` passes.
* **Full strict**, foreground, streams separated: **3512 passed / 0 failed /
  4 ignored = 3516**, and the branch's own `#[test]` total counted this session is
  **3516**. Closes exactly. Delta against master's 3515: **+1**, chased to the
  named function — `a_struct_sized_copy_run_grows_with_the_struct`.
* **Warn tiers**: `warn_tier_lint_ids_match_the_frozen_baseline` passes; the
  firing set across all seven shapes is unchanged (`module.path-mismatch`,
  `proc.undeclared-fallthrough`, `proc.out-unwritten`, `proc.clobber-undeclared`).
  This parcel deliberately changes no lint.
* **Negative probe, both polarities**, in-tree rather than on the emulator
  because that is where the claim lives: `a_struct_sized_copy_run_grows_with_the_struct`
  asserts the derived run is byte-identical to the restated one at 10 bytes (so
  the byte gate provably cannot tell them apart) and that only the derived run
  changes at 11.
* **Oracle domain**: `n = 10` added to `NS`, so the new stride is executed over 9
  sampled/boundary `x` × 3 upper-word garbage seeds × with/without scratch.

## A/B — four probes, all pass

`docs/superpowers/notes/2026-08-07-mulw-parallax-ab.md` (spec written before the
run; execution appended after).

1. **Five ROM sites, 10/10 reads exact** — every site differs as predicted, so
   the change is live at all five, not just the inspected one.
2. **Parallax arithmetic** — `d3` identical at three hits, two of them
   non-vacuous (`d2` = 3 → 30, `d2` = 1 → 10). `d5` differs at exactly the
   predicted values and is the ONLY differing register; it is written once in the
   whole proc and declared clobbered.
3. **`MigrateMasks` across a forced leftward slide** — inputs byte-identical
   (three region hashes + the whole register file), sites 1-2 stepped through on
   BOTH carts, output `0x9CFA99EF` on both. Non-vacuous by construction: that
   output requires the ×26 to have produced 26 for entry 1.
4. **The shipped 2059-tick fixture** — completes on both with the same end
   `Replay_Ptr` `$0005E6A0`, the same `Entity_Loaded_Masks` `0x25913C7E` and zero
   desyncs; site 3 fires on both at the **same `Logic_Tick` `$54E`**.

## Per-pass findings

**Step 3 (retrospect / language asks)** — nothing new demanded of the language.
The two constructs this needed (`mul_const.w` with a `sizeof` immediate, and a
`fold`-generated move run) both already exist and both worked first try; the
`0..n |> fold(asm {}, ...)` idiom has two prior corpus users (`clear_longs`,
`fill_slot_markers`).

**Step 5 (engine optimize)** — one proposal, measured and NOT taken, so it does
not ride this proof: the `.copy_band` source pointer could walk with the copy's
own post-increments instead of being recomputed each iteration. `.find_k` leaves
`a4 = bands_base + (k+1)·sizeof(band_entry)` — the invariant is exact for both
loop exits — so `band[k]` is `a4 − sizeof(band_entry)`, and `a2`/`a3` are free
inside the loop. That removes the multiply, the `lea` and the `adda` entirely:
**−44 cycles per band** against the −2 this parcel banks, and it makes the stride
structurally unrepresentable rather than merely derived. Not taken because it is
a control-flow restructure of a proc being touched for a latent defect, and
mixing the two would put a behaviour-shaped change inside an identity proof.

**Neither bucket — the headline** — a prior ruling in this ledger said the
parallax ×10 was "correct as written; **re-open only if the cost table
changes**". This parcel changed the cost table. The condition fired, the ruling
inverted, and the site adopted. Two things follow: a cost-grounded ruling that
names its premise is re-checkable and one that does not is a landmine; and that
row had measured only the STRIDE — it never noticed the same block restated the
entry size a **second** time as a hardcoded ten-byte copy, which no cycle
argument protected at all.

## Honest residue

* `Section_GetSecPtrXY`'s ×66 is **not exercised behaviourally by any drive
  available here** — armed for the whole fixture on both carts and never fired;
  fired once in the camera walk with flat index 0, where the change cannot show.
  Ledgered, and it is lane D's case from a second direction.
* The A/B surfaced a new instrument trap (the halt landing BEFORE the breakpoint
  address while the log reports it triggered) — ledgered with its bar. Every
  register read in this A/B was taken after verifying the PC.
