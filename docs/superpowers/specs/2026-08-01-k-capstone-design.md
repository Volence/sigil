# Parcel K — the capstone: the declared map + the residual-split (design)

**Status: RATIFIED (overseer/spec owner: Fable).** Evidence base:
`notes/2026-08-01-k-capstone-survey.md` (READ IT FIRST — every number below is
grounded there). Successor to the B-0 packed-placement note (the mechanism this
spec makes declaratively-owned) and closer of rows 6/58's full arc.

## §0 — The end-state (what "done" means)

- Placement authority is a DECLARED, reviewed artifact — the map — not tables
  bootstrapped from a build. Packed placement (B-0) stays the MECHANISM; the map
  replaces the bootstrapped ORDER/ANCHOR authority and repin.toml's region list.
- `games/sonic4/main.asm`, `games/demo/main.asm`, `engine/engine.inc`,
  `engine/macros.asm` are DELETED. Every included module/data island is
  manifest-placed `.emp`.
- The 106 inert `org` lines die with the files that carry them (they are
  already placement-dead; the deletion is the proof made visible).
- **Named survivors (the honest 100%)**: `games/*/config/game.asm` ×2 (the
  contract macros — pending the ratified game-contract-hook construct, a
  language-round item; kill-row-9/45 combo matrices stay as their guard) and
  the vendored debugger tree (Volence ruling, own debugger planned). Everything
  else is `.emp`.

## §1 — The map (K1): declared constraints, not baked addresses

One map file per target profile: `games/sonic4/map.toml`, `games/demo/map.toml`
(+ the off-canonical configs reuse them with overrides as the profiles do
today). TOML, not a new grammar — the repin.toml precedent: build tooling
config, boring, diffable; `region` items (item #7) stay in-source for RAM; the
ROM map is the build's placement contract. Owned facts, each replacing an
implicit authority:

1. **Ordered section list** (the §3.3 ordering manifest) — TWO-STAGE (amended
   at the K1 stop, ruled R2+R1): AS-residual section names are synthetic and
   LMA-derived (`sec{vma_base}`, documented non-stable, non-unique), so a
   name-keyed declared order is unrealizable while the residual exists — and
   the residual is exactly what K2–K4 delete. STAGE 1 (K1): the map VALIDATES
   the derived order, keyed by stable head-labels/module-ids where they exist
   — fold-identical by construction, and any derivation change fails loud
   against the declaration. STAGE 2 (post-K4, the flip's completion): with
   every section a stable-named .emp module, the map DRIVES order and the
   frozen tables fully demote. R3 (AS-frontend name surgery) rejected per the
   item-7 §9 hot-path precedent.
2. **Island anchors**: sections genuinely anchored (`anchor = 0x10000` object
   bank, the sound banks, boot/vector head). Replaces ANCHOR_GAP inference
   for the declared ones; the >0x400 inference stays as a lint that flags an
   undeclared island (`[map.undeclared-island]` — loud, then declare it).
   NOTE (K1 finding): the DAC banks (0x48000/0x50000) are LABEL-LESS islands —
   the complete anchor set is measurement-derived, so the map, reader, and
   lint must land TOGETHER (the lint enumerates what the map must declare; a
   complete map.toml cannot be authored ahead of the reader).
3. **Holes**: `[[hole]] after = "<section>" at = <addr> filled_by = "<module>"`
   — the boot_data $3FE/z80_init-idle relationship, declared (§3 below).
4. **Budgets**: per-region byte ceilings (the spec §7 line 722 promise —
   object-bank 64KB guard etc.), enforced at pack time with over-by-N.
5. **Bank groups** ride the existing `bank:` section property (shipped D2.25) —
   the map references, does not redefine.

Mechanics: `resolve_layout`/`packed_true_bases` consume the map for
order+anchors+holes+budgets; the frozen tables REMAIN as derived per-freeze
MEASUREMENT caches (they record what the pack produced; they no longer author
anything). repin.toml's `[[region]]`/`[[symbol]]` lists derive from the map +
listings or retire where redundant; `pins.rs` keeps its role (repin-generated
TEST SNAPSHOTS). The ~68 `map_toml` test fixtures migrate mechanically to the
same reader. Fold-identity bar: with the map transcribing the current layout,
all six targets byte-identical BEFORE any file moves — the map lands as a
pure authority swap (its own parcel, its own proof).

## §2 — Sub-parcel plan (sequential; each with the standard bar)

- **K0 — pre-K deletes** (rides K1's branch, first commit): `aabb.inc` (kill
  condition fired — its gated twins are deleted) + `z80_sound_syms.asm` (stale,
  emitted by nothing, included by nobody). Byte-neutral. Census/kill-list rows.
- **K1 — the map authority swap** (§1). Sigil-side; aeon untouched except
  the two map.toml files landing in the game trees. Fold identity ×6.
- **K2 — boot_data + the hole** (#12): port boot_data.asm to `.emp` (embed for
  the blob — shipped; vdpComm/vdpReg/bytesToLcnt become comptime fns in the
  ported module, taking 3 of macros.asm's 5 live consumers with them); the
  $3FE relationship becomes a declared map hole with z80_init as the filler.
  The conditional demo/sound arms are `if` groups (the shipped conventions).
  Live on all six targets — full identity bar.
- **K3 — the interior islands**: the ojz generated data (#28/30–33) becomes
  generator-emitted `.emp` modules, manifest-placed (the Parcel-I root finding
  is resolved by K1 — native placement IS the include replacement). RULING
  (pre-made): **generated files do NOT wait for the objdef/objentry DSL** — a
  generator emits verbose typed struct literals (machines don't pay ceremony;
  the DSL remains a language-round item for HUMAN-authored files). #29
  entity_data ports this way, taking objentry/objend (macros.asm's last 2 live
  consumers). act_descriptor.asm's residue (61 ln) dissolves into its `.emp`.
- **K4 — the skeleton dissolution**: main.asm ×2 + engine.inc delete — every
  surviving include becomes a manifest entry; `header.inc` + `sound_bank.inc`
  (the survey's census-missing, load-bearing data-emitting contract macros)
  port as `.emp` data + comptime with their 14 fatal walls as ensures;
  macros.asm deletes (consumers all gone by K2/K3; the 20 dead helpers die
  unmourned). The `-D` interface names stay build-profile config (ruled at F).
  game.asm ×2 survive as the named exception with a README-class comment
  pointing at the game-contract-hook ledger row.

## §3 — Hazards the parcels must respect (from the survey §12, binding)

1. The $3FE hole is the ONE non-mechanical sub-problem — K2 does not proceed
   past a surprise there; it stops (the boot path is the highest-blast-radius
   code in the tree).
2. The kill-row-9/45 combo matrices keep byte-diffing the REAL game.asm every
   run — K4 must not break them; game.asm survives K by design.
3. Appendix drift from label renames is expected in K3/K4 — the H/H-demo
   protocol applies (anchors proven unchanged, flagged re-freeze); genuine
   assembled-byte changes are NOT expected anywhere — any anchor move is a
   STOP, not an A/B (nothing in K changes behavior).
4. Every sub-parcel: six-target identity + strict green + fixpoint refreeze
   discipline + census/kill-list/gap-ledger same-commit + the main-checkout
   sweep. Inspection-first; STOP over hack; honest partials welcome.

## §4 — What K explicitly does NOT do (ledgered for the language round)

The game-contract-hook construct (frees game.asm) · the objdef/objentry
human-authoring DSL · relaxation-aware `align` · struct-scope `@allow` ·
typed-const-refs-typed-const · `[u8;_]` sugar · the mapping-DSL candidate.
These are the language-ask round's agenda, scheduled after K.

## §5 — K1 stop addendum (ruled R2+R1)

The K1 porter's finding and this ruling are the §1-fact-1 amendment above.
K1's landed scope is therefore: the map file (anchors + holes + budgets +
order-validation entries), the reader, the consumption flip for
anchors/holes/budgets, the order VALIDATION (not drive), the
`[map.undeclared-island]` lint, and the fixture migration — all together,
fold-identity ×6. The order-DRIVE flip is K5 (post-K4), a small parcel once
names are stable.
