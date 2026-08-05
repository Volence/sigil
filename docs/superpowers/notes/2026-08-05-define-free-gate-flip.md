# 2026-08-05 — the define-free gate flip

Branch pair: sigil `define-gates` (from master `ea7b1c36`) / aeon `define-gates`
(from master `77d5317`). Merge state lives in the campaign log, not here.

One parcel because it has to be one: the corpus fix is aeon-side, the gate flip is
sigil-side, and either alone turns a gate red.

---

## §0 — Headline

**Every corpus gate walked a define-free corpus, and no shipped shape is
define-free.** Code inside `if SOUND_DRIVER_ENABLED == 1 { }` or `if DEBUG == 1 { }`
comptime-vanishes before the walk begins, so the gate passes on code it never read.
Two live classes shipped through that hole. Both are fixed, byte-neutrally, and the
two gates that missed them now walk all seven shipped shapes with a standing probe
each that fails if the walk ever collapses back to one define set.

Byte-neutral ×7 · `refreeze --check` OK at chain 44 · strict **3240 / 0 / 4 = 3244**,
the branch's exact `#[test]` total · **aeon must merge first**.

The two revert probes are the load-bearing evidence, and each carries a
counter-proof: with the fix reverted, the NEW gate fails naming the site while
MASTER's gate — run against the identical broken corpus — passes green.

---

## §1 — My own re-measurement, against the packet's counts

Run own-run at the branch point, `--report contracts` under each of the seven
shipping profiles. The packet (`2026-08-05-bprime-4-report-and-cc-precision.md`
§1.5) measured only `--game sonic4` and `--game sonic4 --debug`.

| shape | `SOUND_DRIVER_ENABLED` | `DEBUG` | G5 `slot-type-mismatch` | §1 `clobber-undeclared` |
|---|---|---|---|---|
| sonic4 plain | 1 | 0 | **6** | 0 |
| sonic4 debug | 1 | 1 | **6** | **5** |
| demo plain | 0 | 0 | 0 | 0 |
| demo debug | 0 | 1 | 0 | **5** |
| config_a | 1 | 1 | **6** | **5** |
| config_b | 0 | 0 | 0 | 0 |
| lean | 1 | 0 | **6** | 0 |

The packet's two counts hold exactly: 6 and 5, same procs, same registers. **Two
things the packet did not have**, both from measuring the other five shapes:

1. **The 6 slot mismatches ship in FOUR shapes, not one.** Every
   `SOUND_DRIVER_ENABLED = 1` shape carries them — including `lean`, the
   crash-report-off release shape, and `config_a`.
2. **The 5 clobber under-declarations ship in `demo debug` too.** This is the
   interesting one: demo is sound-OFF with `MAX_RING_BUFFER = 16`, a different game
   registry entirely, and the class still fires identically. The defect is gated on
   `DEBUG`, not on anything game-specific, so it reaches every debug shape any game
   builds. A fix validated only against sonic4 would have been validated against a
   third of its blast radius.

Neither difference changes the fix. Both change what "this is a sonic4 problem"
would have meant.

D1c (`[call.live-clobbered]`) is 21 in the four plain shapes and 26 in the three
debug shapes; dead-saves are 3 in all seven; drops, holes, collisions, §6 flag
firings, D1b, `[bus.*]` and every `[context.*]` firing kind are 0 in all seven.

---

## §2 — The corpus fix (aeon)

### §2.1 — Six `as SfxId` blessings

| file | proc | site |
|---|---|---|
| `games/sonic4/player/player_ground.emp:90` | `PState_Ground` | `move.b #SFXID_SPINDASH, d0` |
| `games/sonic4/player/player_ground.emp:162` | `PState_Ground` | `moveq #SFXID_ROLL, d0` |
| `games/sonic4/player/player_ground.emp:803` | `Player_Jump` | `moveq #SFXID_JUMP, d0` |
| `games/sonic4/player/player_spindash.emp:86` | `PState_Spindash` | `move.b #SFXID_SPINDASH, d0` |
| `games/sonic4/player/player_spindash.emp:128` | `PState_Spindash` | `move.b #SFXID_DASH, d0` |
| `games/sonic4/player/player_common.emp:457` | `Player_Animate` | `moveq #SFXID_SKID, d0` |

Every one is inside an `if SOUND_DRIVER_ENABLED == 1 { }` block, every one feeds
`Sound_PlaySFX`'s `d0: SfxId` slot, and every constant is a
`pub const SFXID_* : SfxId` in `games/sonic4/config/sound_ids.emp`. The engine's own
call sites (`sound_api.emp:384/387`, `animate.emp:228`, `game_debug.emp:110`) already
bless correctly — game-side drift, not a design hole, exactly as the packet read it.

### §2.2 — Five clobber declarations, and why NOT `preserves`

| proc | register | how |
|---|---|---|
| `Collected_ParkSlot` | d2 | direct — the `if DEBUG == 1` duplicate-id scan |
| `EntityWindow_TrySpawnRing` | d5 | direct — the `if DEBUG == 1` no-dup scan |
| `EntityWindow_RescanRings` | d5 | transitive, via TrySpawnRing |
| `EntityWindow_ScanRingsRight` | d5 | transitive, via TrySpawnRing |
| `EntityWindow_PopulateSectionRings` | d5 | transitive, via TrySpawnRing |

The packet proposed `preserves` + "widen ParkSlot's save to `.l` first". I did not
take that route, and the reason is a measurement, not a preference.

Both sites bracket their DEBUG arm with a HALF-WIDTH save/restore —
`entity_window.emp` `move.w d2,-(sp)` / `move.w (sp)+,d2` around a
`moveq #COLLECTED_PARK_SLOTS-1, d2`, and `move.w d5,-(sp)` / `move.w (sp)+,d5`
around a `moveq #0, d5`. `moveq` writes all 32 bits; the restore returns 16. The
`preserves` verifier is width-correct (`preserves.rs` tracks `.l` saves; a `.w`
round-trip "leaves the high word stale and does NOT verify"), so declaring
`preserves` on the code as written produces `[proc.preserves-unverifiable]`, an
error. Making it verify means widening the saves — `$3F02` → `$2F02` and
`$3F05` → `$2F05`, same instruction length, **different bytes in the three DEBUG
shapes**. That is a golden re-freeze and a chain bump past 44, which this parcel's
bars forbid.

So the honest byte-neutral fix is the other true statement: **absence from
`clobbers` IS a survives-claim** (§7.1), the claim is false in the DEBUG shapes, and
retracting it is a correctness fix in its own right. `clobbers(d2)` / `clobbers(d5)`
are TRUE — the procs do destroy those registers' high words — and over-declaring a
write is the safe polarity.

What neither contract can say is what the code actually guarantees and what all
three callers actually consume: **the low word survives.** `Collected_UpdateCenter`
reads d2 only through `sub.w`/`cmpi.w`; `EntityWindow_Scan` and `EntityWindow_RescanY`
read d5 only through `btst d6, d5` with `d6 < MAX_TRACKED_SECTIONS`. Gap-ledgered as
`preserves.w(rN)`, with the demand gate being a second corpus instance or the first
caller that needs the long.

Also ledgered: a `clobbers` clause cannot be comptime-conditional, so these five
declarations charge d2/d5 in the four PLAIN shapes where nothing writes them. Sound,
measurably free today, and a real precision loss that will grow.

**The safety margin is now pinned, not merely reasoned.** Every d5 reader uses
`btst d6, d5`, which on a data register selects bit (d6 mod 32) of the whole LONG —
so the half-width restore covers the read only while the bit index stays under 16, and
nothing said so. `ensure(MAX_TRACKED_SECTIONS <= 16, …)` now does, and names the fix
(widen the pair to `.l`) rather than just refusing. Comptime-only: zero bytes, all
seven targets identical. Both lenses converged on this independently.

**Lens C dissents on this whole choice and the overseer should rule — see §8.2.**

### §2.3 — Byte-neutrality, confirmed

Seven-target `cmp` against the golden blobs, in `capture_goldens.sh` order, canonical
rebuilt afterwards:

```
OK  1 s4          OK  4 demo.debug     OK  7 lean
OK  2 s4.debug    OK  5 config_a       OK  restore s4
OK  3 demo        OK  6 config_b       OK  restore s4.debug
```

A type bless and a clobber clause are metadata, and the bytes agree.

### §2.4 — Comments

Eight comments in `entity_window.emp` stated contract facts the edit made false —
three `// Clobbers:` header blocks, the `Collected_ParkSlot` header, two call-site
annotations (`// preserves a0/d2-d5`, `// clobbers d0-d4/a0/a2`), the TrySpawnRing
header, and the DEBUG arm's `// d5 must survive — push it`. All rewritten to
present-tense contract facts naming the half-width restore as the reason the
register is declared clobbered rather than preserved.

---

## §3 — The gate flip (sigil): which profiles, and why

### §3.1 — All seven, and the reasoning

`warn_tier_corpus.rs` is the house precedent and it walks all seven with a per-shape
row. I took the same shape set and a stronger assertion, because the subject differs:
warn-tier pins a per-shape lint-id SET (shapes legitimately differ), whereas the
closure residue and the slot-type firing set must be **EMPTY in every shape** — there
is no legitimate per-shape divergence to baseline.

Why not fewer:

- **Only `sonic4 plain`** leaves `DEBUG`, `SOUND_DEBUG_HOTKEYS`, `SOUND_DBG_MIRROR`
  and `CRASH_REPORT=0` arms unread — and `DEBUG` is where five of this parcel's seven
  defects lived.
- **Only the sonic4 pair** would have missed that the clobber class also fires in
  `demo debug` (§1). Nothing about the fix changes, but the claim "this is sonic4
  drift" would have been wrong.
- The seven profiles cover BOTH polarities of every toggle they carry:
  `SOUND_DRIVER_ENABLED` (1 in sonic4×2/config_a/lean, 0 in demo×2/config_b),
  `DEBUG` (1 in sonic4 debug/demo debug/config_a), `CRASH_REPORT` (0 only in lean),
  `SOUND_DEBUG_HOTKEYS` and `SOUND_DBG_MIRROR` (1 only in config_a). That is what
  makes "empty in every shape" a statement about the corpus rather than about
  whichever arms one define set happened to keep.

### §3.2 — What actually changed

`crates/sigil-harness/src/native.rs` gains `shipped_shapes()` — the ONE place the
seven-row table is spelled — and `shape_defines(&GameProfile)`, the one conversion
from a profile to a `-D` set. `warn_tier_corpus.rs` and the CLI's `profile_defines`
now read them, so three of the six duplicate spellings the B′-4 lens-B row named are
retired and this parcel added none.

`GAME_CONFIG_DEFINES` — the hand-copied `MAX_RING_BUFFER`/`VRAM_RING_PLACEHOLDER`/
`COLLECTED_WINDOW_SLOTS` table row 103 was named for, cross-checked by nothing — is
**deleted**.

Four gate bodies now assert per shape:

| gate | file | tier |
|---|---|---|
| `corpus_has_zero_dropped_instructions` | `contract_closure_corpus.rs` | substrate |
| `corpus_closure_residue_is_empty_the_error_gate` | `contract_closure_corpus.rs` | **ERROR** |
| `corpus_flag_results_are_all_consumed` | `contract_closure_corpus.rs` | §6 |
| `retrofitted_corpus_has_zero_slot_mismatches` + both pre-existing negative probes | `slot_type_corpus.rs` | **ERROR** |

The drop gate was the one site already carrying defines, and flipping it is what let
`GAME_CONFIG_DEFINES` die. The §6 gate came along because leaving one define-free
call among profile-routed siblings in the same file is worse than either state.

Both files also gained `native::ensure_generated(&aeon)` before the walk.
`engine/sound/generated/` is a gitignored build product the corpus `embed`s, and a
missing embed target drops instructions — precisely what the drop gate asserts is
zero. Pre-existing exposure, widened by the debug shapes needing the `_debug.bin`
variants; closed here.

### §3.3 — The anti-vacuity probes, which are the point

A gate walking a profile where its subject is compiled out is the same failure in a
new costume. Each flipped file therefore carries a permanent SHAPE-SENSITIVITY probe
that doctors a **comptime-gated** site and requires a two-sided outcome:

- `a_bless_stripped_inside_a_comptime_gate_fires_in_exactly_the_sound_on_shapes`
  strips `as SfxId` from `Player_Jump`'s `moveq #SFXID_JUMP, d0`. Must fire in the
  four sound-ON shapes; must be SILENT in the three sound-OFF ones.
- `a_clobber_undeclared_inside_a_comptime_gate_fires_in_exactly_the_debug_shapes`
  drops `d2` from `Collected_ParkSlot`'s `clobbers`. Must fire in the three DEBUG
  shapes; must be SILENT in the four plain ones.

The fires-here half proves teeth on comptime-gated code. **The silent-there half is
the one that matters**: it is false the instant the walk stops reading each shape's
defines, because seven labels over one define set would fire everywhere. Both probes
assert on their anchor string before doctoring, so an aeon reformat breaks them
loudly rather than turning them into no-ops.

---

## §4 — The revert probes, with counter-proofs

Both classes, run against the real aeon tree.

**Class 1 — slot type.** Reverted `Player_Jump`'s `as SfxId`:

```
NEW gate  → FAILED
  [call.slot-type-mismatch] firings on the retrofitted corpus in shape `sonic4 plain`:
    SlotTypeMismatch { proc: "Player_Jump", callee: "Sound_PlaySFX", reg: "d0",
                       expected: "SfxId", found: None, … }
MASTER gate (same broken corpus, `git show master:…/slot_type_corpus.rs`)
          → ok. 1 passed; 0 failed
```

**Class 2 — transitive clobber.** Reverted `Collected_ParkSlot`'s `d2`:

```
NEW gate  → FAILED
  shape `sonic4 debug`: closure firing(s) — an undeclared register effect must be
  declared or verified-preserved before it can ship: [("Collected_ParkSlot", "d2")]
MASTER gate (same broken corpus, `git show master:…/contract_closure_corpus.rs`)
          → ok. 1 passed; 0 failed
```

The counter-proof is the half worth having. It is not merely "the new gate catches
this" — it is "the old gate, on the identical corpus, was green," which is the blind
spot demonstrated rather than asserted.

---

## §5 — Every remaining define-free gate

The sweep is over real-corpus gates (those reading `AEON_DIR`). Synthetic-source
tests (`branch_const.rs`, `context_brackets.rs`, `corpus_contracts.rs`,
`type_slice.rs`, `z80_bus.rs`) own their input and have no blind spot; build-driven
gates (`warn_tier_corpus`, `native_rom`, `native_full_rom`, `native_offcanonical_*`)
already run under profiles.

**CLOSED this parcel** — 4 gate bodies + 2 probes, all seven shapes:
`contract_closure_corpus.rs::{corpus_has_zero_dropped_instructions,
corpus_closure_residue_is_empty_the_error_gate, corpus_flag_results_are_all_consumed}`
and all of `slot_type_corpus.rs`.

**STILL BLIND — profile-reachable, cheap, measured:**

| site | feeds | what moves when flipped |
|---|---|---|
| `contract_closure_corpus.rs::corpus_report()` | **5 gates, 3 of them ERROR**: `corpus_input_undefined_is_empty_the_error_gate` (D1b), `corpus_context_brackets_prove_the_error_gate` (§3.2), `corpus_context_requirements_are_satisfied_the_error_gate` (§3.3), plus `corpus_flag_results_declared_vs_verified_credit_agree` and `corpus_out_residue_is_the_verified_complement` | every firing set is already 0 in all seven, so only ONE pinned census moves: `context_regions.len()` is **17** define-free vs **23** for sonic4 plain/debug/config_a/lean and **20** for demo plain/debug/config_b. `context_claim_sites` (10) and `context_discharged` (12) are shape-invariant. This is a per-shape row on one number — the highest-leverage remaining row by a wide margin. |
| `out_verify_corpus.rs:55::corpus_report()` | `dump_out_unverified_residue`, `cond_out_survives_claims_all_prove`, `d1c_firings_match_the_frozen_baseline` | `D1C_BASELINE` is the 21-row plain-shape set; the debug shapes fire 26. Needs a second row (or a per-shape table), and the 5 extra rows are named in §1. |

**STILL BLIND — STRUCTURALLY unable to flip in place.** All three are in
`crates/sigil-frontend-emp/tests/`, and `sigil-harness` (the sole owner of
`GameProfile`) **depends on** `sigil-frontend-emp`. Those tests cannot see a profile
at all; they must move up a crate to `crates/sigil-cli/tests/`, or the frontend needs
a define-set injection point that does not import the harness.

| site | posture | teeth today |
|---|---|---|
| `movem_restore_guard_corpus.rs::every_stack_movem_restore_has_a_matching_save` | `eval_proc_body(…, &[], …)` | **the one with real teeth** — it pairs `movem` saves against restores corpus-wide, and every `movem` pair inside an `if DEBUG == 1 { }` arm is invisible to it |
| `preserves_corpus.rs::residue_procs_verify_as_predicted` | `eval_proc_body(…, &[], …)` | checkpoint pin over named procs |
| `dead_save_corpus.rs::dead_save_worklist_over_corpus` | `analyze_corpus` | dump only; measured 3 dead-saves in all seven shapes, so shape-invariant today |

One more class, adjacent and worth naming: `contract_closure_corpus.rs` scopes its
DEFINES to a shape but not its MODULE SET, so `--game demo` still analyzes all of
`games/sonic4/`. That is the B′-4 lens-B row (`GameProfile::registry` carries the
per-target module ids) and it is deliberately left alone — every corpus gate walks
`engine/` + `games/` whole, so narrowing one makes it disagree with all of them.

---

## §6 — Merge order: AEON FIRST

**aeon `define-gates` must merge before sigil `define-gates`, and the reason is
mechanical.** The sigil gates assert an EMPTY firing set under every shipped shape.
Against a master aeon tree they see 6 slot mismatches and 5 closure firings and go
RED. In the other order, an aeon master carrying the fixes with a sigil master whose
gates are still define-free is merely a corpus that is stricter than its gate — green,
and the state the tree sits in between the two merges.

There is no window in which aeon-first is red. There is no window in which
sigil-first is green. The `sr` lane reached the same answer for the same reason.

---

## §7 — Bars

- **Byte bar ×7**: `cmp` against the golden blobs in `capture_goldens.sh` order
  (`s4`, `s4.debug`, `demo`, `demo.debug`, `config_a`, `config_b`, `lean`), canonical
  `s4.bin`/`s4.debug.bin` rebuilt and re-compared afterwards. **All nine comparisons
  identical**, before and after the corpus edit.
- **`refreeze --check`**: `OK (tip 'b-jumps', chain len 44)`.
- **Strict**, `SIGIL_STRICT_GATE=1 AEON_DIR=<b3 aeon> cargo test --workspace
  --release`, full capture, failures-first: **0 failures**.
  **3240 passed / 0 failed / 4 ignored = 3244.**
- **Test delta accounted exactly**: `git grep -c '^\s*#\[test\]'` over
  `crates/**/*.rs` is **3239 at master**, **3244 at `define-gates`**, delta **+5**,
  and `passed + ignored = 3244` matches. Every one is new; none deleted or renamed,
  and no other file's count moved:
  | file | master → branch | added |
  |---|---|---|
  | `contract_closure_corpus.rs` | 9 → 11 | `a_clobber_undeclared_inside_a_comptime_gate_fires_in_exactly_the_debug_shapes`, `an_undeclared_clobber_in_ungated_code_fires_in_every_shape` |
  | `slot_type_corpus.rs` | 3 → 4 | `a_bless_stripped_inside_a_comptime_gate_fires_in_exactly_the_sound_on_shapes` |
  | `sigil-harness/tests/shipped_shapes.rs` | 0 → 2 (new file) | `the_shipped_shape_set_is_the_seven_the_byte_bar_builds`, `every_comptime_toggle_is_walked_in_both_polarities` |
- **Cost**: the two flipped test binaries run in 1.03s and 0.90s wall in the RELEASE
  build the strict suite uses; lens C measured ~26s and ~20s in a debug build, so the
  figure depends entirely on the profile. Seven analyses over one parse is cheap
  because the parse is the expensive half, and it is correctly hoisted.
- **Every probe proven to have teeth**, not merely to pass: the two revert probes
  with their master counter-proofs (§4), and the `MAX_TRACKED_SECTIONS <= 16` ensure
  checked by temporarily tightening it to `<= 3` and confirming the build fails
  naming it.

---

## §8 — Lens panel

Three fresh read-only lenses over `git diff master...define-gates` in both repos:
A ceremony/style, B corpus-pattern with the VACUITY question pointed at it
explicitly, C correctness/hazard. All three found real things. **The panel changed
the parcel materially** — the flip as first written had a genuine vacuity hole of its
own, which is the outcome the standing lens rule exists to produce.

### §8.1 — Lens B: the flip's own vacuity holes (ACCEPTED, all three)

**B1 — the closure gate had a liveness witness in only 3 of the 7 shapes it walked.**
Its one probe PARTITIONS: it demands a firing in the three `DEBUG` shapes and SILENCE
in the four plain ones. Silence is also what a walk that analyzed nothing produces —
so `sonic4 plain`, `demo plain`, `config_b` and `lean` had no proof their half of the
ERROR gate read anything at all. `slot_type_corpus.rs` already had this covered (its
two pre-existing negative probes doctor UNGATED code and now assert per shape); the
closure gate had no analogue. **Fixed**:
`an_undeclared_clobber_in_ungated_code_fires_in_every_shape` drops `d1` from
`Collected_UnparkSlot`'s `clobbers` — ungated code, shape-invariant firing — and
requires all seven to show it. This is the single most valuable thing the panel
produced: the parcel existed to close a vacuity hole and had opened a smaller one.

**B2 — removing a shape from `shipped_shapes()` was SILENT.** Adding one is loud
(`warn_tier_corpus` panics on a shape with no baseline row); removing one just
narrows every gate with everything green. It matters concretely: `config_a` is the
ONLY carrier of `SOUND_DEBUG_HOTKEYS=1`/`SOUND_DBG_MIRROR=1`, and `lean` is the ONLY
carrier of `CRASH_REPORT=0`. **Fixed**: `crates/sigil-harness/tests/shipped_shapes.rs`
pins the label set AND that all five toggles are walked in BOTH polarities — so the
coverage argument in §3.1 is now executable rather than prose.

**B3 — "zero dropped instructions proves the profile reached the analysis" is FALSE**,
and it was the only thing checking. Lens B measured it: a statement-`if` whose
condition does not resolve discards BOTH arms and drops nothing. A profile that lost
`SOUND_DRIVER_ENABLED` or `DEBUG` outright reproduces this parcel's exact bug at zero
drops. The claim is corrected in place; the structural fix (`[comptime.unresolved]`)
is ledgered, not built — `CRASH_REPORT`/`SOUND_DEBUG_HOTKEYS`/`SOUND_DBG_MIRROR` still
have no proof they reach the analysis, and a per-toggle probe will never scale to
"the next toggle someone adds."

Lens B also independently reproduced §1's counts on master and §5's `context_regions`
17/23/20 re-baseline, and decomposed the delta exactly: the six/three elided brackets
are ALL `z80_stopped` — so the §3.2 bracket ERROR gate, still define-free, examines
**none** of the Z80-bus fences in any shipped shape. That is a sharper statement than
row 103 carried and it is now in the row.

### §8.2 — Lens C: my `ensure_generated` was wrong (ACCEPTED)

I had added `native::ensure_generated` to both walks on the theory that a missing
`embed` target drops instructions. **Lens C proved the rationale false**: `Embed`
appears nowhere in `sigil-frontend-emp` — `embed(...)` is a `data` item the contract
walk never resolves — and it deleted BOTH generated directories and watched the gates
pass anyway. Worse, it is a WRITE into `AEON_DIR` from a read-only analysis gate,
which can race a concurrent build.

What actually shrinks the walk is a missing GENERATED MODULE:
`engine/debug/generated/vectors.emp` is gitignored (`tools/gen_compression_vectors.py`
writes it, from `build.sh`), so a cold tree walks **121** files where a warm tree
walks **122**, with `engine.compression_vectors` and its six instruction sites
silently absent — and nothing noticed. **Fixed**: `ensure_generated` removed from
both, replaced by a corpus FLOOR plus that named witness. Verified own-run: 122 total,
exactly one untracked.

**Lens C DISSENTS on the aeon fix, and the overseer should rule.** It argues the
`.l` + `preserves` route was the correct one on correctness grounds, and its evidence
is stronger than the packet's: the two widenings are the SAME instruction length, so
no address shifts and **the frozen size tables do not move** — the cost is recapturing
three DEBUG goldens, nothing else. It also checked the mechanical safety (neither
DEBUG bracket contains an sp-relative read; `TrySpawnRing`'s `movea.l 8(sp),a0`
precedes the block), and names `Sound_PlaySFX`'s own `movem.l d1/a0` as the in-corpus
precedent. Its point: the shipped variant leaves three call sites the corpus's own
D1c reports as contract violations, where the other variant makes callee contract and
caller reliance simultaneously true. **I did not take it**, because the parcel's bars
are explicit (byte-neutral; `refreeze --check` at chain 44) and my brief says to STOP
and report if bytes move rather than to decide. It is a live decision, not a closed
one, and it is cheap either way.

Lens C otherwise confirmed: no runtime bug (§9), no interior mutability so the
seven analyses cannot contaminate each other (`ast.rs` has zero `Cell`/`RefCell`;
the frontend has zero `thread_local`/`static mut`/`OnceLock`), all six blesses genuine
with the right widths (`$AB`/`$B6` correctly use `move.b` — a `moveq` would
sign-extend), and D1c byte-identical between master and branch at 26 rows because the
closure's `effective` map is BODY-derived, so the declarations merely caught up to
what was already computed.

It also **independently confirmed the merge order empirically**: sigil `define-gates`
against aeon master is `8 passed; 2 failed`. §6 is not a prediction.

### §8.3 — Lens A: ceremony (ACCEPTED, all)

Change-history narration in both module headers and three probe docs (rewritten to
present-tense contract facts); a false `a0` claim in `TrySpawnRing`'s header; two
call sites holding d5 across a now-`clobbers(d5)` call with no note about it; one
mechanical error (`the push restores` — the POP restores); asserts missing their shape
label; `shipped_shapes`' doc overclaiming to be the only target table in the tree.

Its sharpest finding was structural: **the parcel deleted one hand-copied table and
had replaced it with two.** `SOUND_ON` and `DEBUG_SHAPES` restated as string literals
what `GameProfile` already carries as `sound_on` and `debug`. Both probes now
partition on the profile's own flag, and each asserts BOTH halves actually ran — a
partition that lands every shape on one side asserts nothing on the other.

### §8.4 — One process note

Lens C ran `git stash` / `git checkout master` inside my sigil worktree while I was
editing it, which silently reverted two of my edits (I re-applied them twice before
its report explained why). Its own disclosure is what identified the cause. The stash
it left is superseded and contains nothing not now committed. A read-only lens should
not mutate the tree it reviews; if a lens needs to build master it wants its own
worktree.

### §8.5 — Declined

`retrofitted_corpus_has_zero_slot_mismatches` → a name saying the property: declined
as orthogonal churn on a pre-existing name that carries real project vocabulary.
A shared `tests/common/` module for the four near-identical `corpus_sources`/
`emp_files` copies: declined as a refactor wider than this parcel, ledgered instead.
`OnceLock` caching of the per-shape walk: declined — the release-build cost is 1.0s
per file, and a cache shared across probes that DOCTOR their input is a footgun.

## §9 — Step-3 (language / tooling asks) vs step-5 (engine) findings

**Step 3 — the language could not say the true thing.** The headline finding is not
either defect; it is that when the contracts were wrong, the vocabulary offered no
way to make them right without either over-charging or moving ROM bytes.
`preserves.w(rN)` (the low word survives) and a comptime-conditional `clobbers`
clause (this write exists only under `DEBUG`) are both ledgered. The third ask is
structural rather than syntactic: the crate graph puts `GameProfile` downstream of
the frontend, so three frontend corpus gates cannot reach a real define set at all.

**Step 5 — the engine is fine, and that is the finding.** No runtime bug, verified
register-by-register at all four live call sites. It is even tighter than "accidentally
sufficient": in `Collected_ParkSlot`, d2 arrives with its high word already 0
(`moveq #0,d2` + `move.b d0,d2` in the caller) and the DEBUG arm's
`moveq #COLLECTED_PARK_SLOTS-1, d2` sign-extends a POSITIVE 3, so the high word is 0
both before and after — the `.w` restore reproduces d2 bit-exactly, and the only
post-call read is a `sub.w` anyway. For d5 the readers are `btst d6, d5` with
`d6 < MAX_TRACKED_SECTIONS = 4`, inside the restored low word, and the third caller
does not hold d5 at all (it is dead, killed by a preceding `clobbers(d0-d5)` call).
The engine change is four characters of declaration, one `ensure`, and eight
corrected comments. What moved is not the code but what the code is allowed to claim.

**Neither bucket — the headline, and the panel's turn on it.** The parcel's value is
not the 11 fixed defects. It is that a gate which walks a shape nobody ships is
indistinguishable from a gate that passes, and the only durable defence is a probe
whose SILENT half fails when the shapes collapse.

The panel then made the same point back at me, which is the part worth carrying
forward. A two-sided probe is necessary and NOT sufficient: because one of its halves
asserts silence, it is itself satisfied by a walk that analyzed nothing (§8.1 B1). The
full pattern a flipped gate needs is three pins, and it took a lens to see the third:

1. the property, asserted per shape;
2. a SHAPE-SENSITIVITY probe over comptime-gated code — fires in exactly the shapes
   that assemble it, silent in the rest;
3. a LIVENESS probe over UNGATED code — fires in every shape, so no shape's silence
   can mean "saw nothing".

Plus, one level up, a pin on the shape SET itself (§8.1 B2), because all three are
vacuous for a shape that quietly stopped being walked. That is the template for the
five remaining blind gates, and it is worth more than either defect this parcel fixed.
