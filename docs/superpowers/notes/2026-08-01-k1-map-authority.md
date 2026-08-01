# Parcel K0 + K1 — close-packet: the pre-K deletes + the declared placement map
# (authority swap, VALIDATE stage, ruled R2+R1)

**Porter: Opus (k1-map branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
K0 (pre-K deletes) and K1 (the map authority swap, R2 validate stage) are landed and
proven fold-identical ×6. The K1 order-DRIVE flip + frozen-table full demotion is K5
(post-K4), per Fable's ruling. Spec: `specs/2026-08-01-k-capstone-design.md` (§1 fact-1
two-staged at `6e685f13`; §5 addendum). Evidence base: `notes/2026-08-01-k-capstone-survey.md`,
`notes/2026-07-31-waveb-b0-computed-placement.md`.

---

## §0 — K0: the pre-K deletes (byte-neutral ×6)

### aabb.inc — DELETED (aeon `9ae3323`, tracked → `git rm`)
`engine/objects/aabb.inc` (50 ln). Grep-proof zero consumers: the only `aabb.inc`
hits are COMMENT lines in `aabb.emp`; `collision.emp:6`/`rings.emp:13`
`use engine.objects.aabb` = the `.emp` twin, not the `.inc`. Its AS consumers
(`collision.asm`+`rings.asm`) are already deleted. Kill-list rows 5+13 fired ahead of Spec 5.

### z80_sound_syms.asm — removed from disk (untracked; nothing to commit)
**Premise correction:** the parcel brief said `engine/debug/z80_sound_syms.asm`; it is
`engine/sound/generated/z80_sound_syms.asm` (survey §10.2). **Untracked** (gitignored via
`engine/sound/generated/`), a leftover from a pre-`flip-stage0` build — seam1's
`emit_sound_blob` stopped writing it at flip-stage0 (kill row 92) and nothing includes it.
`rm`'d from disk.

Commits: aeon `9ae3323`. sigil `fb4287aa` (kill-list rows 5/13/92 + census row 11 → DELETED/stale).

---

## §1 — K1: the declared placement map (what landed)

Two committed maps — `games/sonic4/map.toml`, `games/demo/map.toml` (aeon `a737568`) — carry
the reviewed placement contract; the sigil reader + consumption flip (sigil `e4bdb73a`)
make them the authority the chainer's resolved layout must agree with.

### The four map facts (each replacing an implicit authority)
| fact | as-landed | replaced |
|---|---|---|
| **anchors** | `[[anchor]] at=` — `0x0` boot/vector head + `0x10000` object bank [all six] + `0x58000` (vma `0x8000`) MT/SFX phase bank [`when=sound_on`] | the `packed_true_bases` ANCHOR_GAP / phase-bank / run-head inference, now VALIDATED against the declaration (the `[map.undeclared-island]` lint) |
| **hole** | `[[hole]] after="Z80_IdleProgram" at=0x3FE filled_by="engine.z80_init" when="sound_off"` — data | boot_data's `org $3FE` no-sound arm (enforcement rides K2; K1 = data + a presence check) |
| **budget** | `[[budget]] region="object_bank" ceiling=0x20000` | the AS `if * > $20000 / error` object-bank guard (was already `sigil.map.toml`'s `object_bank` region — now per-game) |
| **order** | `order=[…]` — 70 (sonic4) / 41 (demo) byte-emitting section head-labels/module-ids, canonical UNION order | (STAGE 1 / R2) the map VALIDATES the frozen-derived order; the frozen tables remain the per-label provisional-base MEASUREMENT cache the derivation sorts. STAGE 2 (K5): map DRIVES. |

### The consumption flip (`native.rs::validate_placement`, called post-resolve)
Recovers the inferred island set from the RESOLVED layout (lma-sorted; anchor = run head OR
`lma > prev_end + 0x400` OR phase bank `vma!=lma && vma>=0x8000`) and checks it against the
shape-applicable declared anchors both ways (`[map.undeclared-island]` / `[map.anchor-absent]`);
validates the derived byte-emitting id order (min-offset LABEL, image bytes > 0) is a
subsequence of the declared `order` (`[map.order-diverged]` / `[map.order-undeclared]`);
checks each applicable hole's `after` label resolves. **Fold-identical by construction** —
it ADDS checks, changes no placement. The per-game map's regions (mirroring `sigil.map.toml`)
drive `emit_rom` + the budget; `GameProfile::map_path` derives `games/<g>/map.toml`.

### The keying that makes R2 work (the §2 finding's resolution)
Order is keyed by the **min-offset LABEL** (BootData, ObjCodeBase, GameLoop, …) — stable and
in the frozen table — NOT the synthetic `sec{vma}` section name. Two robustness facts, both
empirically established and load-bearing:
1. **Byte-emitting order is shape-invariant.** With zero-byte sections excluded, s4 / s4_debug
   / config_a / config_b share one relative order (and demo / demo_debug another) — so a single
   per-game UNION list validates all its targets via the subsequence check. (The naive
   full-order check failed only on `__BUDGET_DATA`, a zero-byte marker whose tie-position flips
   between sound-on/off — byte-neutral; excluded.)
2. **The island set is small + shape-invariant + label-free-safe.** Exactly `{0x0, 0x10000}`
   (all six) + `{0x58000}` (sound-on). The survey-flagged DAC banks (`0x48000/0x50000`) are NOT
   inferred islands (label-less BINCLUDE blobs that pack by contiguity) — the lint enumerates
   the true set, and it needs no label-less anchor declaration.

### Proof — the bar, met
- **Full fold identity ×6** (byte-for-byte, appendix included, NO re-freeze): all native gates
  green (`native_rom` 2/0, `native_offcanonical_rom` 4/0, `native_full_rom` 3/0,
  `native_offcanonical_full` 7/0, `native_declared_chain` 2/0, `native_offcanonical_placement`
  8/0 — the B-0 fold-identity suite). **Six CRCs = CHAIN-11 tips exactly:** s4 `ff9037f2/412127`,
  s4.debug `06680f0b/421958`, demo `4e446a64/90524`, demo.debug `949e9215/93022`,
  config_a `2485eab3/422297`, config_b `d6d23298/303501`.
- **`refreeze --check`: OK (tip `conv-hdemo`, chain len 11).**
- **`repin` → `pins.rs unchanged`** (diff empty).
- **strict 2885 / 0 / 4** (baseline 2877 + 8 new: 2 reader unit tests + 6 `validate_placement`
  negative probes proving undeclared-island / anchor-absent / order-diverged / order-undeclared
  / shape-gating each FIRE on a doctored map, and the correct map passes).

### Fixture migration census (deliverable 5)
**Zero fixtures migrated — none needed it.** The ~68 `map_toml` port-test fixtures build
region-only maps through `sigil_link::load_map`, which is UNCHANGED. The K1 placement facts are
ADDITIVE and game-map-only (a region-only source parses to an empty `PlacementMap` — unit-tested
`region_only_map_parses_to_empty_placement`), so the anticipated "pins→map ripples every fixture"
(survey §12.6) did not materialize: the region format is stable and the placement layer is
orthogonal. Per Fable's leave ("thin test-only constructor acceptable if unification resists —
say so"): no constructor was needed; `load_map` serves both, and `load_placement_map` is the
thin additional parse for the game maps only.

### repin.toml (deliverable 4) — retire nothing (conservative)
repin.toml's 58 `[[region]]` rows are per-section pin sources (start/end SYMBOLS → `pins.rs`
test snapshots). They are NOT redundant with the map's placement facts: they reference some of
the same symbols (e.g. `ObjCodeBase`) but drive a different mechanism (the cosmetic per-shape
region bases the `ModuleSpec`s carry, IGNORED under Frozen) — retiring any would break `pins.rs`
generation, which Fable ruled untouched. The map's 3 geometry regions live in each `map.toml`
(mirroring `sigil.map.toml`, which retires in K5). So: no repin row derived or retired this
parcel; `pins.rs` role intact, `repin → pins.rs unchanged` confirmed.

---

## §2 — Decision record: the K1 stop that produced the R2+R1 ruling (evidence)

The "ordered section list drives the packer / frozen fully demoted" fact (spec §1 fact-1)
collided with reality: ORDER today = a stable sort over frozen-table-derived provisional bases
(`true_bases_by_index`/`packed_true_bases`); to stop the frozen table authoring order the map
must supply it via stable per-section identifiers, but AS-residual section names are synthetic +
LMA-derived + non-stable (`sigil-frontend-as/src/eval.rs:2036` `format!("sec{vma_base}")`;
`sigil-harness/src/lib.rs:56-64` "not stable identifiers"; `text` non-unique) — and that residual
is exactly what K2–K4 delete. Precedent: item-7 design §7.2 defers the "§3.3 ordering manifest";
§9 rejected AS-frontend surgery for this class (hot-path blast radius). **RULED R2+R1** (spec
`6e685f13`): STAGE 1 (K1, this packet) the map VALIDATES the derived order keyed by stable
head-labels; STAGE 2 (K5, post-K4) the map DRIVES and the frozen tables fully demote; R3
(frontend name surgery) rejected. The DAC-bank label-less finding is why map+reader+lint landed
together (the lint enumerates the true island set).

## §3 — step-3 / step-5 / ledgers

- **step-3 (retrospect):** the "ordered section list" fact assumed stable section identifiers;
  keying by min-offset LABEL + excluding zero-byte markers is the realization that makes it
  fold-identical today. Recorded as the §1 keying note + the spec amendment.
- **step-5 (optimize):** none — K1 is a pure authority swap by contract; no behavior/byte change
  in scope, none made.
- **Gap-ledger:** the order-DRIVE flip (frozen tables fully demoted) is K5 (post-K4), tracked in
  the spec §1-fact-1 / §5. `sigil.map.toml` retirement (its 3 geometry regions now duplicated in
  the per-game maps) is a K5 cleanup. Both are conscious, ledgered deferrals, not gaps.
- **Kill-list:** K0 rows 5/13 (aabb.inc) + 92 (z80_sound_syms.asm) closed; census row 11 stale.
  No new kill rows (the map reader is permanent, not scaffolding).

## §4 — Status
- **K0 + K1 (VALIDATE stage): DONE**, fold-identity ×6, strict green, refreeze/repin discipline intact.
- Commits (branches unmerged for Fable's countersign): aeon `9ae3323` (K0), `a737568` (map files);
  sigil `fb4287aa` (K0 rows), `e4bdb73a` (reader + flip), + this note.
- Next in the K arc: K2 (boot_data + the $3FE hole enforcement), then K3/K4, then K5 (order-DRIVE).
