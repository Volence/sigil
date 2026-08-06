# The byte-changing multiply parcel — LTR honesty + loop adoption (2026-08-06)

Status: RULED (Fable, overnight overseer session; Volence's directive authorizes
the parcel — his morning gate is play-test acceptance, and the merge must be
cleanly revertible if he rejects it). Base documents: the 2026-08-05
`mul_const.w`/`mul_bounded.w` ruling and the mul-w packet; this spec states only
what changes. Census verified against sigil `247ae9ef` / aeon `4974bf3` on
2026-08-06 (scout, own-run) — every site below was re-read from the current
tree, not taken from the ledger.

## 1 · What this parcel is

The corpus's first deliberate byte-MOVING optimization pass since conversion.
The mul-w round shipped the word construct under a byte-identity bar, which
required deliberately suppressing the cheaper LTR chain form (gate at
`mul_lower.rs:425`, `n.count_ones() >= 3`) — the hand chains sit 2–10 cycles
from optimal BY DESIGN, recorded as a step-5 finding "recoverable only by a
deliberate byte-changing parcel." This is that parcel. The byte gate flips from
identity to AUDIT: every delta named and explained; behavioral identity proven
on the emulator (overseer-run); the win measured in lag frames.

## 2 · Rulings

**R1 — the LTR gate relaxes to `>= 2` set bits.** Both the two-power arm and
the LTR arm generate at 2 bits; `choose()` picks by the ruled tie-break (fewest
worst-case cycles → fewest bytes → fixed enumeration order). The gate comment
at `mul_lower.rs:416–440` rewrites to a present-tense contract fact (the
byte-identity rationale is history now — it moves to the packet, not the code).
The suppression was load-bearing for the adoption round's bar; this parcel
retires that bar deliberately, with Volence's authorization.

**R2 — every remaining hand multiply site adopts in THIS parcel.** The scout
found exactly seven un-adopted sites (§3 table, groups B–D). The two sites that
would have been byte-identical adoptions under the old gate (group D) become
byte-changing under R1 — adopting them here rather than splitting a "neutral"
commit is the ruling: one parcel, one refreeze, and the end state is that every
multiply in the corpus is construct-spelled and model-priced. No opportunistic
adoption debt survives this parcel.

**R3 — the word loop sheds its dead accumulator seed.** `mul_bounded.w`'s loop
candidate currently seeds the accumulator with `moveq #0` (generator
:517–522) — dead under the word contract (the `add.w` body never reads the
upper word, and the upper word is UNDEFINED by contract). Shedding it is a
generator correctness fix (the model was over-pricing its own loop by 4
cycles), unit-oracle-proven with garbage-upper-word seeds, and it moves the
loop/mulu boundary from M=2 to M=3 exactly as row 2165 derived.

**R4 — the two loop sites adopt `mul_bounded.w` with the bound derived from
the module ensures, and the MODEL decides loop vs mulu.** If the honest bound
at real grid heights prices mulu as the winner, mulu is the adoption — that is
not a failure, it is the construct doing its job (worst-case 70 ceiling vs the
hand loop's unbounded 28+14·M). The porter pins the chosen lowering and the
boundary at each site. Row 2165 closes either way; the kill condition names
both exits.

**R5 — one refreeze, at the end.** Chain entry 49, anchor-primary doctrine,
real `--ab` refs for every moved anchor. No per-step freezes. Canonical aeon
ROMs rebuilt after capture; both build shapes verified separately (one shape
per invocation).

**R6 — expected size behavior, verified not assumed.** The stride/chain
re-derivations (groups A, C, D) are expected SAME-SIZE (4×2-byte instruction
forms either side) — opcode content changes, layout does not. The loop sites
(group B) change size. The porter verifies actual sizes; any layout shift
triggers the FULL 5-site ripple (repin → pins.rs auto; engine.inc,
mixed_dac_rom.rs, repin_pins.rs by hand; repin.toml only if a region was
added). Z80 stays byte-neutral (blob-precedes-engine).

## 3 · Ordered changes (nothing else moves)

| Group | Site(s) | Change | Expected Δcycles |
|---|---|---|---|
| A | section.emp:154 ×66 · tile_cache.emp:233 ×66 · tile_cache.emp:77 ×80 (comptime fn → both splice sites) · plane_buffer.emp:76 ×160 | already `mul_const.w`; re-derive to LTR under R1 (no .emp edit — generator output changes) | −2 · −2 · −8 · −10 |
| B | section.emp:142–147 `.gxy_mul` · tile_cache.emp:225–231 `.mul_loop` | adopt `mul_bounded.w` (R3+R4) | model-decided; pinned |
| C | tile_cache.emp:326–329 · :374–377 · :1355–1359 (single-temp ×80) | adopt `mul_const.w …, #80` | model-decided (ties at 32 are legal; pin the tie-break) |
| D | plane_buffer.emp:247–250 ×80 · section.emp:281–283 ×160 | adopt `mul_const.w` (byte-changing under R1) | −8 · −10 |

Sigil side: `mul_lower.rs` (gate :425, word-loop seed :517–522, comment
rewrites), `tests/mul_lowering.rs` (re-pin chosen lowerings, both-polarity
probes for the new gate boundary: a 2-bit n now yields LTR-or-two-power by
cost; a 1-bit n still yields the shift arm; the ×66/×80/×160 pins update to
the LTR encodings), unit oracle extended per R3.

## 4 · Bars (the byte gate is an AUDIT, not identity)

1. **Every byte delta named.** Porter produces a delta table: per golden
   target, `cmp -l` against the chain-48 goldens, each run of differing bytes
   mapped to a §3 site (symbol-range mapping via the convsym output or map
   file). Any delta NOT in the table = STOP. `scripts/corpus_bytediff.sh`
   additionally runs as the compiler-level probe (it is sigil-only and
   example-scoped — it does not replace the whole-ROM audit).
2. **Behavioral identity, overseer-run.** The porter does NOT touch the
   emulator (oracle MCP deadlocks under subagents). Porter delivers: built
   ROMs both shapes, the delta table, per-site cycle expectations. The
   overseer runs deterministic A/B: Frame_Counter-anchored input scripts,
   state_hash + memory_hash OLD vs NEW, the Debug_Scene_Freeze(0xFF8A10) +
   camera-poke cache-fill identity technique, ObjectTest soak via the
   Game_Entry flip. Divergence is adjudicated explicitly (a legitimate
   timing shift from the cycle win is the only acceptable class) — never
   waved through.
3. **Lag frames, before and after, overseer-run.** Trailing lag indicator
   (the beam-position gate is dead — Tile_Cache_Fill runs in VBlank). The
   stride wins land in the tile-cache hot path; the win must show up there or
   the packet says why not. No scene may regress.
4. Full strict with closing arithmetic; refreeze `--check`; warn-tier ID sets
   ×7 (no deliberate lint deltas in this parcel); negative probes both
   polarities for the gate boundary; the R6 ripple if layout shifts.
5. Panel (A/B/C, read-only) before merge; packet with per-pass step-3/step-5
   findings; NO merge-state claims in the packet.

## 5 · Ledger and doc actions (same parcel)

- Row 2165: CLOSE with the R4 outcome (adopted; name the chosen lowering).
- Open NO new row for the LTR suppression — it dies here; the mul-w packet's
  step-5 item 3 gains a one-line "taken by this parcel" pointer, and the
  mul-const-w design §3's suppressed-LTR paragraph gains the same pointer.
- The generator gate comment rewrite (R1) — present-tense only.
- If R4 lands mulu at a loop site, record the actual-vs-ceiling honesty note
  at the site (the hand loop was cheaper for small sec_y; the construct prices
  worst-case — that is the contract, stated once, at the site).
