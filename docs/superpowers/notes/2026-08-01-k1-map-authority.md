# Parcel K0 + K1 — pre-K deletes done; K1 map-authority swap STOPPED at the
# spec-vs-reality collision on the "ordered section list" fact

**Porter: Opus (k1-map branch, aeon + sigil worktree). Overseer/spec owner: Fable.**
Evidence base: `specs/2026-08-01-k-capstone-design.md` (§1/§2/§3),
`notes/2026-08-01-k-capstone-survey.md` (§6/§9/§10-§12),
`notes/2026-07-31-waveb-b0-computed-placement.md` (the mechanism). This note records
K0 (DONE, byte-neutral ×6) and a **STOP finding on K1's central mechanism** — the
"ordered section list drives the packer / frozen tables fully demoted" fact collides
with a concrete, precedented reality obstacle. K1's *declarable* facts (anchors, hole,
budget) are transcribed below as the seed the reader+lint must land TOGETHER, post-ruling.

---

## §0 — K0: the pre-K deletes (COMPLETE, byte-neutral ×6)

### aabb.inc — DELETED (aeon `9ae3323`, tracked → `git rm`)
- `engine/objects/aabb.inc` (50 ln). Grep-proof of zero consumers:
  `grep -rn aabb.inc` across `.asm/.inc/.emp/.sh/.rs/.toml` → the only two hits are
  COMMENT lines in `aabb.emp`. `collision.emp:6` / `rings.emp:13` `use engine.objects.aabb`
  = the `.emp` twin `aabb.emp`, NOT the `.inc`. Its documented AS consumers
  (`collision.asm` + `rings.asm`, the gate-off twins) are already deleted
  (`find engine -name collision.asm -o -name rings.asm` → empty). Kill-list rows 5+13
  condition fired ahead of Spec 5.

### z80_sound_syms.asm — REMOVED FROM DISK (untracked; nothing to commit)
- **Premise correction:** the parcel brief located it at `engine/debug/z80_sound_syms.asm`;
  it is actually `engine/sound/generated/z80_sound_syms.asm` (survey §10.2).
- It was **untracked** (`git ls-files` → no match; gitignored via `engine/sound/generated/`).
  A leftover from a pre-`flip-stage0` build: seam1's `emit_sound_blob` stopped writing it
  at flip-stage0 (kill row 92; `render_syms_asm` removed) and nothing includes it
  (`grep -rn z80_sound_syms` → zero non-comment hits). `rm`'d from disk; nothing to commit.

### K0 proof — fold identity ×6, exit 0
`SIGIL_STRICT_GATE=1 cargo test --release -p sigil-cli --test native_rom --test
native_offcanonical_rom --test native_full_rom --test native_offcanonical_full --test
native_declared_chain --test native_offcanonical_placement` → **all green, exit 0**
(native_rom 2/0, native_offcanonical_rom 4/0, native_offcanonical_full 7/0,
native_offcanonical_placement 8/0, + native_full_rom / native_declared_chain above the
tail cutoff; cargo's exit 0 ⇒ every binary green). The deleted files are included by
nothing, so byte-neutrality is guaranteed by construction and proven by the suite.
- Commits: aeon `9ae3323` (delete). sigil `fb4287aa` (kill-list rows 5/13/92 +
  census row 11 → DELETED/stale).

---

## §1 — K1 STOP: the "ordered section list" fact collides with reality

### What K1 asks (spec §1)
Four map-owned facts replace implicit authorities: (1) **ordered section list** — "the
declared order the packer walks. Replaces the frozen tables' bootstrapped order";
(2) **island anchors**; (3) **holes**; (4) **budgets**. Mechanics: "the frozen tables
REMAIN as derived per-freeze MEASUREMENT caches (they record what the pack produced;
**they no longer author anything**)." The parcel brief: "flip `packed_true_bases`/the
profile layer to consume the map for ORDER + ANCHORS (+ budgets), with the frozen tables
demoted to derived measurement caches **exactly as the spec says**." And the hazard clause:
"If the frozen-table demotion **fights** the refreeze driver's regeneration flow, that's a
**stop**, not a workaround."

### How ORDER is actually authored today (native.rs, read-only)
`true_bases_by_index` (Frozen arm): each ROM section's **provisional base** =
`min over its labels found in the frozen table (addr − label.offset)`; label-less →
`s.lma` (order-only fallback). `packed_true_bases` then:
- **ORDER** = `order.sort_by_key(|&i| prov[i])` — a **stable sort over the frozen-table-
  derived prov bases**. (Stable ⇒ equal-prov ties resolve by section INSERTION order:
  AS-side sections, then `.emp` sections.)
- **ISLANDS (anchors)** = a section is an island iff it is the run head, OR
  `prov > running_end + ANCHOR_GAP(0x400)`, OR a phase bank (`vma != lma && vma ≥ 0x8000`).

So ORDER is *inextricably* a function of the frozen table's per-label addresses. To make
the frozen table "no longer author" the order, the **map must supply the order** — which
requires stable per-section identifiers.

### The obstacle: AS-residual section names are synthetic and NOT stable
`sigil-frontend-as/src/eval.rs:2036` names every auto-opened AS section
`format!("sec{vma_base}")` — an **LMA-derived** name. `sigil-harness/src/lib.rs:56-64`
(`region_at_lma`) documents it verbatim: *"the front-end's auto-section names (`sec{vma}`)
are disambiguated on collision and so are **not stable identifiers**."* Empirically, the
resolved section names for the AS residual are `text` (the default, appearing **5+ times**
in a single target — not name-unique) and `sec256 / sec936 / sec1022 / sec65536 / sec65542
/ sec65906 / sec383840 / sec361991 / …` (each keyed on the very LMA the map is meant to
REPLACE). A declared "ordered section list" keyed by section name is therefore:
- **not realizable** — the names are non-unique (`text`) and LMA-synthetic; and
- **self-defeating** — it would encode the addresses the map exists to stop authoring, and
  would go stale the instant anything moves (which is the whole point of packed placement).

`.emp` module sections DO have stable names (boot, vdp_init, …). The AS-residual *labeled*
sections have stable HEAD LABELS in the frozen table (BootData, ObjCodeBase, Checksum,
NullInterrupt, HeightMaps, SoundTablesZ80_Head, …). So a declared order keyed by
**module-id | head-label** (with label-less blobs slotting by contiguity) IS stable and
reviewable — but that is a **different keying than section-name**, and adopting it is a
design decision on a RATIFIED spec, i.e. the spec owner's call.

### Precedent — this exact class was already ruled once
The item-7 RAM-regions design names the same end-state "the §3.3 **ordering manifest**"
and **defers it**: §7.2 "cross-module contribution deferred to the map-file ordering
manifest"; and §9 records a porter who "**empirically confirmed §3.3's export claim false
for eager AS references and stopped per brief**" — RULED **Option B** (harvest values),
rejecting **Option A** (teaching the AS frontend to defer absolute-EA operands) because it
"touches the **hot conversion path every residual line rides**." Teaching the AS frontend
to emit *stable section names* is the same class of change against the same hot path, and
was implicitly disfavored by that ruling.

### Why this is a STOP, not a hack-around
Per the parcel's own hazard clause, the frozen-table **demotion fights reality**: the order
is a stable-sort over frozen-derived prov bases, and the AS residual it orders has no stable
declarable section identity while those `.asm` files still exist (they are deleted only in
K2–K4). Any "ordered section list" I ship now would be a brittle LMA-snapshot masquerading
as a declaration — exactly the "hack around" the brief forbids. **The anchors/holes/budgets
facts do NOT have this problem** (they are addresses, not names) and are transcribed below.

---

## §2 — The declarable K1 facts, transcribed (the map seed)

Derived by instrumenting `packed_true_bases` (a throwaway `K1_DUMP` eprintln, since
reverted; branch is clean) and dumping the converged round for all six targets.

### Island anchors (the genuinely-anchored heads)
| anchor | addr | present in | classification today |
|---|---|---|---|
| boot/vector head (run head) | `0x0` | all 6 | run-head island |
| object bank (`ObjCodeBase`) | `0x10000` | all 6 | ANCHOR_GAP island (also `object_bank` region in sigil.map.toml) |
| MT/SFX sound bank (`SoundTablesZ80_Head`) | `0x58000` (vma `0x8000`) | sound-ON only (s4, s4_debug, config_a) | **phase-bank** island (`vma != lma && vma ≥ 0x8000`) |

**Open question the reader's lint must answer (why the map+lint land together):** the
survey §7 asserts the DAC banks (`0x48000` / `0x50000`) are also phase-bracket anchors, but
they carry **no frozen-table label** (the only table label in `0x40000–0x59000` is
`SoundTablesZ80_Head`) — they are label-less BINCLUDE blobs, classified by contiguity /
ANCHOR_GAP at runtime, not hand-declarable from the table. The **complete** anchor set
(which label-less blobs cross the ANCHOR_GAP into island status) is measurement-derived; the
spec's `[map.undeclared-island]` lint is precisely the mechanism that ENUMERATES them (an
ANCHOR_GAP-inferred island absent from the map fails loud → you declare it). Hence a correct,
complete `map.toml` cannot be authored *ahead of* the lint — they are one unit of work.

### Hole (spec §1 fact #3, declared-as-data now; K2 consumes)
`boot_data.asm`'s no-sound arm: `[[hole]] after = "Z80_IdleProgram" at = 0x3FE
filled_by = "engine.z80_init"` — the 38-byte idle occupies `0x3d8..0x3fe`; the post-hole AS
data resumes at `0x3fe`. LIVE for sound-OFF (demo, demo_debug, config_b); in sound-ON the
Z80 sound blob fills the region (no hole). Enforcement/consumption is K2 (survey §5/§7).

### Budget (spec §1 fact #4)
`object_bank`: base `0x10000`, size `0x10000` (the `__BUDGET_DATA` cursor must stay
`≤ 0x20000`) — already the `object_bank` region + `check_object_bank_budget` in
`sigil.map.toml`. No other ROM ceiling is asserted in the residual today beyond this and the
`EndOfRom > 0x3FFFFF` / geometry walls (survey §9); the brief says "do not invent new ceilings."

### Proposed schema (for Fable's review — R2, see §3)
Per-game `games/<g>/map.toml` extending today's `sigil.map.toml` region format
(backward-compatible so the ~68 `map_toml` region-only fixtures keep parsing through the
same `load_map`):
```toml
# regions: unchanged from sigil.map.toml (rom / object_bank / z80_moving_trucks_bank)
[[anchor]] name = "boot_head"    at = 0x0
[[anchor]] name = "object_bank"  at = 0x10000
[[anchor]] name = "sound_bank"   at = 0x58000  vma = 0x8000  when = "sound_on"
# ...plus every label-less ANCHOR_GAP island the lint enumerates (DAC banks, tails)
[[hole]]   after = "Z80_IdleProgram" at = 0x3FE filled_by = "engine.z80_init" when = "sound_off"
[[budget]] region = "object_bank" ceiling = 0x20000
[[order]]  by = "module_id | head_label"  list = [ /* R2: stable ids, contiguity for blobs */ ]
```

---

## §3 — Recommended resolution (for the spec owner)

Three options; recommend **R2 now + R1 for the full drive**:

- **R1 (sequence):** the "ordered section list DRIVES the packer / frozen fully demoted"
  becomes cleanly realizable only once the synthetic-named AS residual is ported/deleted
  (K2–K4). Do K1's **anchors + hole + budget** now (address-keyed, no name dependency);
  defer full order-drive to after the residual shrinks. Matches the item-7 §7.2 deferral.
- **R2 (validate, keyed by stable id):** land the map NOW owning anchors+hole+budget, plus a
  declared order keyed by **module-id / head-label** (label-less blobs by contiguity), and
  make the map AUTHORITATIVE by **gate**: the derived (measurement-cache) order/anchor set is
  VALIDATED against the declared map and **fails loud on divergence** (`[map.undeclared-island]`
  + an order-mismatch lint). This is fold-identical by construction (derivation unchanged; a
  check is ADDED), is how repin/pins already work (generate + assert-matches), and makes the
  map the reviewed authority without the unrealizable section-name drive. Upgrade to "map
  drives" under R1 when names stabilize.
- **R3 (frontend surgery):** teach the AS relocating frontend to name sections by head label.
  **Disfavored** — same hot-path blast radius the item-7 §9 ruling rejected.

Under R2/R1, K1 lands its declarable facts fold-identically this session's successor;
the `[map.undeclared-island]` lint enumerates the complete anchor set (resolving the DAC-bank
open question); the ~68 region-only `map_toml` fixtures need no migration (backward-compatible
`load_map`); repin.toml's region rows stay (they mirror the same object_bank/z80 geometry the
map already owns) — "derive-or-retire" becomes a same-work follow-on, not a blocker.

---

## §4 — Ledger / kill-list / gap-ledger

- **Kill-list:** row 13 (aabb.inc) → DELETED (K0); row 5 aabb token struck; row 92
  (z80_sound_syms.asm on-disk leftover) → fully closed. Census row 11 → DELETED/stale.
- **Gap-ledger:** the K1 order-drive is deferred pending Fable's R1/R2/R3 ruling (this note).
  The DAC-bank label-less-island enumeration is a reader/lint deliverable, not a hand-declared
  map fact.
- **Step-3 (retrospect):** the spec's "ordered section list" fact silently assumed
  stable section identifiers; reality (eval.rs:2036 / lib.rs:56-64) + the item-7 §9 precedent
  say otherwise while the AS residual lives. Surface this in the spec.
- **Step-5 (optimize):** none taken — K1 is a pure authority swap by contract; no
  behavior/byte change is in scope, and none was made.

## §5 — Status
- **K0: DONE**, byte-neutral ×6, committed (aeon `9ae3323`, sigil `fb4287aa`).
- **K1: STOPPED** at the spec-vs-reality collision above; awaiting the spec owner's ruling on
  R1/R2/R3 before the reader + consumption flip + map files land (they must land together so
  the map is provably complete + correct). Branch clean; strict baseline intact (no sigil code
  changed beyond the reverted instrumentation; K0 doc rows only).
