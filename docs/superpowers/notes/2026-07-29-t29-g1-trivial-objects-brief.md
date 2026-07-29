# 2026-07-29 — t29 brief: game-side G1 — the trivial objects (test_static + test_animated)

Status: **DISPATCH BRIEF** (overseer: Fable; porter: Opus subagent, direct-dispatch).
Target = the game-side census's recommended first tranche (`2026-07-29-game-side-census.md`
@ sigil `c37a164` — READ ITS G1 SECTION AND THE PER-FILE ROWS FIRST). Volence's standing
authorization (2026-07-29): continue the cadence, full loop + panel discipline.

## 0. Bars (overseer-verified at dispatch)

- Masters: aeon **`7051b4d`** / sigil **`6cc5920`**, origin==local, clean.
- Canonical: plain **`c51342d0`/421041** (`s4.bin`) · debug **`992d9e7d`/429102**
  (`s4.debug.bin`); EndOfRom `0x5DB60`/`0x5F65A`. Strict baseline **2685/0 (1 ignored)**.
- Branches `port-tranche29` BOTH repos, worktrees `.worktrees/port-tranche29`, editor-dir
  rsync before first build, one shape per invocation, cd-every-call, no `git add -u`,
  explicit paths, never chain two repos' git in one compound.
- **THIS IS A CANONICAL-BYTES TRANCHE** (unlike t27/t28): the two files emit canonical
  bytes in BOTH shapes. Step 1 = byte-identical transliteration (expected delta ZERO).
  Step 2 house-spelling flips (jbsr/jbra etc.) may produce small negative deltas — those
  follow the STANDARD WAVE DISCIPLINE: batch byte-changing flips into ONE wave inside the
  loop when the item set is final, ONE re-pin, the row-1257 upstream-slide sweep, the
  5-site ripple doctrine (engine.inc / mixed_dac_rom / repin_pins are HAND-edited on any
  byte-changing wave; repin auto-does pins.rs only), the $8000 bank-shift bar checked if
  debug-shape bytes move. Panel runs AFTER the wave.
- Checkpoints: (a) steps 0-2 then STOP for countersign; (b) loop+panel; (c) overseer-opened.
- Canonical loop text `notes/campaign-port-loop.md`; kill-list rows same-commit;
  comments describe function; brace-indent; positive controls per the t24 rule.
- Context valve: standing.

## 1. Scope (LEAN, FIRM: two files — no scope growth)

| Lane | File | Census row facts (re-verify at step 0) |
|---|---|---|
| A | `games/sonic4/objects/test_static.asm` → `test_static.emp` | Trivial display object; structurally identical to shipped test_solid.emp/test_particle.emp; all engine callees already `.emp` (module-to-module, NO new externs — the corpus is at ZERO extern procs and MUST STAY THERE); shape-invariant canonical bytes. |
| B | `games/sonic4/objects/test_animated.asm` → `test_animated.emp` | Same class + animation; `AnimId` typed fields in Sst — the type layer may DEMAND blesses at step 1 (t26 precedent: if `slot_type_corpus` fires, take it, not defer it). |

`test_enemy` is NOT in scope (census "±" resolved: it goes to the next game tranche —
scope discipline beats opportunism). The player cluster, test_parent, and the harness
states are explicitly out (the census ordering stands; player_common is the keystone and
is NOT touched here).

## 2. Proof machinery (the census names it; porter verifies at step 0)

- The `test_objects_port` canonical windowed byte-gate class + object-bank region pin +
  `org` resume arm — the EXACT template that proved test_solid/test_particle. Step 0
  derives the region bounds for each file from the listing (both shapes), states the
  neighbour-anchor situation (shared anchors owe the t24-style proof), and adds the
  repin.toml region blocks if the template requires them.
- Gates `SIGIL_EMP_TEST_STATIC` / `SIGIL_EMP_TEST_ANIMATED` (match the existing object
  gate naming — verify against the shipped test_solid/test_particle gates and COPY their
  pattern, do not invent).
- Zero extern procs: any needed engine symbol resolves module-to-module via import. If a
  symbol genuinely cannot resolve module-to-module, that is a STOP-and-report finding,
  not an extern.

## 3. Known type-layer surface (census-derived; verify)

Sst typed fields (`AnimId`, `VramArtTile`, `Coord`, `ObjRoutine`) are live in these
objects' stores. The `pixels_to_coord` construct (kill row 49) MUST be adopted where the
promote idiom appears — re-hand-rolling it is a step-2 checklist violation. `refresh_piece_count`
(t24) likewise if the shape appears. Log-only for anything new (LEAN); take what the gates
demand.

## 4. Panel ruling

**A1 + B1 + C2.** C1 INACTIVE-RECORDED unless step 5 identifies a hot per-frame path worth
cycle-derivation (these are test objects; the census calls them trivial — but if the
animate path runs per-frame per-object, C1 activation is the porter's call to flag at
checkpoint (a), not to decide silently). C3 inactive (no hardware contracts expected; a
VDP-touching surprise = flag it). Lenses synchronous; dry adjudicated by panel.

## 5. Duties

Kill rows same-commit for gate arms/org pins (they join the row-58 class) and any const
mirrors (rows 44/54/59 class — drift-guard + kill-row, the t28 census pattern). Ledger
sweep per pass. Close packet with per-pass findings, neither-bucket headlines, corrections
list. Step-2 checklist feed-forward for anything that generalizes. The census file gets a
STATUS AMENDMENT at close (G1 rows marked ported) — the census is now the game-side
tracking document.

## 6. After t29

G2 (effect/child objects) per the census ordering; the Z80 rung-2 implementation is running
in parallel on `z80-rung2-contracts` (sigil-only; merge queue: t29 FIRST, rung-2 rebases).
