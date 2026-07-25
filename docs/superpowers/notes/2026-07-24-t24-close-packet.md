# t24 — TRANCHE CLOSE PACKET (children.emp: the last unported engine 68k island)

**Sixth tranche under the LEAN amendment.** Scope: `engine/objects/children.asm`
→ `children.emp` (ONE file, no seam split — the descriptor tables stay
game-side), full loop `0 → 1 → 2 → (3→4→5)×3 → panel → wave → re-panel → 6`.
Two panel rounds (one full, one focused-over-the-delta as a recorded
proportionality deviation), three Volence rulings, six overseer rulings.

Branch tips at close: **aeon `679ac4c` / sigil `3e66659`**, bases aeon `1470af2`
/ sigil `b912ffd`.
Branch ROMs at close: **plain `c51342d0`/421041 · debug `992d9e7d`/429102**
(PROVENANCE re-baseline due at merge).
Full paired strict at every byte-changing commit; final **2604/0** (baseline
2588 + 7 t24 probes + 5 children_port + 2 mixed_tranche24 + 2 tranche3
positive controls).

## Scoreboard

| Workstream | Outcome |
|---|---|
| **children.emp** (8 pub procs, 483 L donor) | byte-identical BOTH shapes at the FIRST linking compile — zero demanded features at step 1 (the 5 step-0 probes predicted it) |
| **Step-2 modernize** | bare Bcc / `jbra` / `jbsr` file-wide; **−6 bytes**, the step-0 prediction exactly, with its three named relaxation sites |
| **Step-4 constructs (2 built)** | `pixels_to_coord` (engine/coords.emp, NEW module, 10 sites) + `refresh_piece_count` (frames.emp) — the latter killed an EXACT 12-instruction duplicate between `children` and `animate` that no gate could see |
| **Demanded feature** | struct-field displacement over a SPLICED base register (`Sst.field({reg})`) — the construct pass hit it, TDD'd as probe P6; the sibling of ledger row 1036's spliced-INDEX gap |
| **§19 review mined** | every C/M/micro/bug item adjudicated; M1/M2/M3/M4/M5 + micros TAKEN, C1 + bug(2) escalated, C2/bug(1) documented+asserted, C3 logged |
| **PANEL (A1+B1+B2+C1+C2, C3 non-run)** | DRY did NOT stand — real findings, cycle re-opened |
| **RE-PANEL (C1+C2 over the delta)** | caught a BUG in our own wave (band-inherit), reverted same session |
| **Ownership flips** | NONE (pure-callee port; all three callees already `.emp`-owned) |

## Step 0 (design note `2026-07-24-t24-step0-design.md`, committed before code)

Region CONFIRMED from the real listing, both shapes built in the branch
worktree with editor data seeded (padded-ROM trap avoided) and canonical CRCs
verified BEFORE any code. **5 probes, all at the LOCAL instruction-encoding
binding class** (children carries zero link-time immediates and zero comptime
gating; the three `jsr` targets are the proven load_object class) — **all PASS
as-found**, artifact `tranche24_spelling_probes.rs`.

**Hazard sweep found TWO brief errors that bit** (§"corrections" below). F2
field-store row adjudicated LEDGER (children stores anim memory-to-memory; no
register param carries an anim value, so the reopen condition is not met).

## Step-1 gate list — FILLED (15 rows, every gate named to its artifact)

byte gate plain / debug (`children_port::children{,_debug}_region_matches_reference`)
· region pin + `pins_rs_is_current` · **shared-anchor proof**
(`children_region_pins_share_both_anchors` — entity_window.end == children.base
== load_object.base, both shapes) · drift guards captured+passing · TWO negative
probes (`doctored_sibling_ptr_fires_its_guard`, `doctored_rf_xflip_fires_its_guard`)
· `mixed_tranche24_{rom,debug_rom}` with BOTH in-arm proofs named (org-resume
lands `Load_Object` canonically with children.asm gated out; `test_parent`'s
`jsr CreateChild_Normal`/`jsr DeleteChildren` encode the .emp's own RESOLVED
label addresses) · gate-off CRC neutrality · transitive contract closure (fired
on `DeleteChildren`/a0 until the truthful `preserves(a0)` shipped) · the 5
probes · paired strict.

## Step-2 checklist — FILLED (all seven, explicit outcomes)

1. **Branch conversions + wave**: 7 `jsr→jbsr`, 6 `bsr.w→jbsr`, 7 `bra→jbra`,
   all conditionals bare. Measured −6 both shapes; absorbed by re-pin
   propagation, ROM total unchanged (org-$10000 shield).
2. **Structural width pins**: NONE exist — named not-applicable. The twin's
   surviving `.w` conditionals are resolved widths, not pins, and the twin
   header says so.
3. **Bare-symbol width rule**: not-applicable — zero absolute-EA operands.
4. **Brace-indent**: conformant; no nested `{}` blocks.
5. **Idiom list**: ADOPTED `Sst.field`, movem-RANGE reglists, shared-constant
   import. Not-applicable and named: bareword winptr/bankid, label-in-immediate,
   typed VDP fns, abs-EA-over-link-base, Sec/Act, TILE_CACHE_*.
6. **Type-layer walk**: ADOPTED `(a0/a2: *Sst)`, `(a1: *u8)`, `d0: ObjRoutine`
   ×2, and (post-panel) `d2/d3: i8` where `u16` actively misinformed. LEDGERED
   with reasons: pixel offsets (A4-i class), the bare-word slot pointer,
   `let rN: Type` with its exact first sites.
7. **Noticing**: two proposals (twin lockstep-spelling note; mixed-arm proofs
   must read RESOLVED labels — the second was found by the gate catching my own
   step-1 arm).

## Byte-delta table (measured, not predicted)

| change | Δ plain | Δ debug | absorbed by |
|---|---|---|---|
| step 1 (+gates, pins, mixed arm, 2 templates' consumers) | 0 | 0 | additive; gate-off exact |
| **step-2 modernize** | **−6** | **−6** | re-pin (all regions from load_object down); ROM total unchanged |
| pass-1 3(b) comment batch | 0 | 0 | — |
| pass-1 step-4 (both templates + the demanded feature) | 0 | 0 | comptime-only |
| **pass-1 step-5 (§19 wave: M1/M2/M3/M4/M5 + clr.w + 2 asserts)** | **−0x6C** | **+0x46** | re-pin; **first shape-SPLIT region** (opposite directions per shape) |
| pass-2/3 (D-items, comments) | 0 | 0 | — |
| **ruling wave (A/B/C + panel takeables)** | **+0x3C** | **+0x94** | re-pin; **$8000 bar first live run — PASSED** |
| **re-panel: band revert + rulings 1+2** | **−0xA** | **−0xA** | re-pin |
| **NET vs donor** | **−0x3E** ($30E→$2D0) | **+0xCC** ($30E→$3DA) | debug surplus = 2 assert blobs |

**The −90 on both ROM FILES is NOT image shrink** (overseer catch): assembled
`EndOfRom` is unchanged in both shapes (`ASSEMBLED_LEN`/`DEBUG_ASSEMBLED_LEN`
never moved); file-length−image goes 37317→37227 plain and 38486→38396 debug —
the whole −90 is the **convsym symbol-table appendix**, almost certainly M4's
five deleted fail-skip label sets. Same class as t23's "+30 = appendix
re-encoding". **This belongs in the PROVENANCE re-baseline** or the file sizes
read as a shrink no byte table explains.

## CORRECTIONS LIST (single-sourced)

| claimed | true | note |
|---|---|---|
| FlipAware split "60 bytes for **24 cycles**" | **36–42** | the figure the overseer relayed to Volence; **superseded** — the branchless mask replaces the item |
| inline refresh "**+144 B** / **~120 cyc**" | **+108 B** / **86 cyc** | 120 was inherited from the pre-M2 review estimate |
| high-word-add micro "~10 cyc/axis" | that transform is **+2 cyc / −2 B** | the real 10 belongs to register-caching the parent coord |
| M5 hoist "savings" | **12 cyc/child**, **wash at n=1**, **+12 at n=0** | |
| effect parent_ptr "~28 cyc/frame" | **30** (28 is the multisprite case, which is a behaviour change) | |
| ROM files "−90 both shapes" | convsym appendix, image unchanged | |
| brief: "ledger sweep essentially clean" | **rows 1034 and 1085 both bite** | overseer-owned, below |
| brief: "+4 internal Populate calls" | **6** | |
| brief: "−2 step-2 delta" | **−6** | brief costed only the bsr family |
| brief: "8 Coord-idiom sites" | **10** in-file; and the corpus half was wrong twice (§step 6) | |

## PER-PASS: step-3 vs step-5

- **Pass 0-2**: probes PASS as-found; the two ledger-sweep corrections; the
  demanded-feature-free step 1; the −6 wave.
- **Pass 1** — *3(a)*: named-stack-slots ask; the descriptor-format DSL demand;
  escape census **zero** (cleanest file in the campaign). *3(b)*: 5 comment-claim
  fixes + the chain-contract block + the `_Linked` constraint. *step 4*: both
  templates BUILT + the demanded feature. *step 5*: the §19 wave (above).
- **Pass 2** — *3(a)*: the `save_across(call)` ask (6-site demand). *3(b)*: D6
  caught **six** of my own stale claims. *4*: nothing (the ask is verb (c)).
  *5*: inline-vs-call logged with numbers.
- **Pass 3**: EMPTY at all three steps → dry claim → panel.

## PANEL ROUND (A1+B1+B2+C1+C2 — five lenses, synchronous, read-only; C3 a reasoned non-run: children touches no VDP/DMA/interrupt/bus state)

**DRY DID NOT STAND.** Adjudication: 8 REAL (my own false/stale claims), 4
takeable-and-verified, 2 REFUTED with evidence, 4 escalated, ~15 ledgered.
Headlines:

- **Three lenses independently** caught `DeleteChildren`'s prose-vs-declaration
  clobber mismatch.
- **C2** proved the termination proof incomplete (a self-freed chain member +
  LIFO recycle writes `sibling_ptr(C)=C` → renderer hang + double-free) and
  that the "leak until entity-window despawn" claim is FALSE (despawn calls
  `DeleteObject` and skips untagged slots; children are untagged by
  construction).
- **REFUTED**: C1's zero-displacement tax (the listing shows `3482` —
  the assembler folds it) and C2's `preserves(a0)` fix (verified
  `[proc.preserves-unverifiable]`; the blind spot is real, the fix does not
  compile → language ask).
- **A1's `save_across` verdict** accepted with its third argument partly
  refuted by measurement (the dead-save worklist reports **0 firings** and did
  not flag the `d0` oversave — it analyses the proc's own body, not caller
  liveness).

## RE-PANEL (C1+C2 over the delta — recorded proportionality deviation)

**It caught a BUG in our own wave.** A priority band is a **3-bit value**, the
inherit composes with `or`, and the child-side idiom `ori.b #N<<RF_PRIORITY_SHIFT`
assumes a zeroed field → or-ing both yields their UNION. Verified against
shipped objects: `test_emitter` band 5 + `test_particle` band 6 → **band 7**.
`engine/constants.asm:189` already documents the rule ("ori.b alone accumulates
stale bits") and notes spawn-time `ori` is safe *only because the slot is
zeroed* — **so the inherit did not break a convention, it invalidated the
precondition the documented convention depends on.** Ten child-side `ori`
sites across the game share it. Band half REVERTED; COORDMODE half kept.

## OVERSEER-ERROR ROW (logged, per the campaign's own rule)

**The band regression is the overseer's, not the porter's.** The instruction
was "ONE masked copy" of a field that includes a 3-bit *value*, which specifies
`or` semantics for data that cannot be or-ed. The porter implemented the ruling
as given; the re-panel caught it; the wrong half was reverted and the right
half kept. **The instructive part is the failure mode: extending a ruling by
analogy (COORDMODE ⇒ "same class as the band") without re-deriving it for the
new data shape.** A flag bit and a packed value are not the same class, and the
difference is invisible until something else ORs into the same byte.

## Step-6 corpus sweep (enumeration; B2's additions folded in)

`pixels_to_coord` — **my enumeration was wrong twice, and a lens caught it both
times**: (1) `load_object.emp:55-59` (2 `.emp` sites) never named; (2)
**camera.emp has FIVE sites, not three** — `:224-226` and `:319-321` spell the
same promotion as `ext.l` + `lsl.l #8` ×2 (6 bytes vs 4), invisible to a
`swap`-shaped grep, so those two are byte-CHANGING (−2B) retrofits. Plus
`object_test_state.asm` ×4, `player_common.asm`'s `distToFix` MACRO (5
expansions — the corpus had independently named the same abstraction),
`test_player.asm:229-231`. **BLOCKED**: two `moveq`-fed sites (+2 bytes without
a `clr:` default param). `refresh_piece_count` — both consumers adopted;
`sprites.emp`'s three frame-table walks BLOCKED on the indexed-vs-displacement
interface (which is why `frames.emp`'s "single source of truth" claim was
corrected). **Method note ledgered**: a sweep must enumerate the SEMANTIC
shape, not the syntactic one.

## NEITHER-BUCKET HEADLINES

- **The $8000 bank-relocation class** — a DEBUG-only engine growth pushed
  engine symbols past `$8000`, widening the object bank's `jsr (Sym).w` to
  abs.l and sliding the whole DEBUG game bank +0xC, which broke four fixture
  families that assumed the two banks coincide. All four are now pins-derived
  per shape; the class is a **standing bar** (gap-ledger + a step-5
  interrogation line), and its **first live run passed** at the ruling wave.
- **The positive-control rule** — `tranche3_negative_probes.rs` had been
  comparing a 36-byte section against a 424-byte slice through hardcoded
  addresses: both `assert_ne!` probes were trivially true and *could not fail*.
  Root cause was a stale RAM fixture (`$FFFFA834` vs the real `$FFFFA836`), not
  the twin. Migrated to pins + **two positive controls** that make the file's
  prose claim executable; falsification verified (both probes now FAIL when
  undoctored). Promoted to a campaign-wide rule in the loop doc.
- **Two panel rounds, two catches in our own work** — the full panel caught six
  stale self-claims and a false leak-bound; the focused re-panel caught a
  shipped regression. The proportionality deviation paid for itself.
- **children.emp closes the engine 68k backlog** except the debug trio (t25).
  Escape-hatch census: **zero** externs, ensures, transliterations.

## OPEN AT MERGE (not mine to close)

Band inheritance (Volence, with three fix shapes + the ten-site convention
fact) · C1 effect-visibility probe + COORDMODE-pop probe (overseer runs them) ·
the despawn-leak BUG row · four language asks (conditional-out-on-clobbered,
never-written-as-proof, caller-side liveness, typed comptime-fn registers) ·
the `ObjRoutine` name collision · C1-N4's XFLIP fold (band-adjacent).
