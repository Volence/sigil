# The language-round ledger — every Spec-2 campaign ask, evidence-grounded

Read-only survey (2026-08-02). Purpose: collect every LANGUAGE / TOOLING /
ARCHITECTURE ask the Spec-2 conversion campaign has ledgered, with the parcel +
concrete friction that DEMANDED each, so the spec owner (Fable) can build the
language-round agenda without re-reading the whole campaign. **No design
recommendations here beyond an impact ranking — the spec owner designs.**

Sources swept: `campaign-gap-ledger.md` (whole file; primary), every
`2026-08-01-conv-*` / `2026-08-01-k*` / `2026-08-01-sound-*` / `2026-07-31-waveb-*`
close packet's step-3 sections, `2026-07-31-conversion-tail-census.md`,
`twin-scaffolding-kill-list.md`, the K-capstone / embed / item-7 /
sound-constants specs' deferral sections, and `SIGIL_SPEC2_LANGUAGE.md` §4.7.

The canonical agenda seed is **K-capstone spec §4** ("What K explicitly does NOT
do, ledgered for the language round"): *the game-contract-hook construct · the
objdef/objentry human-authoring DSL · relaxation-aware `align` · struct-scope
`@allow` · typed-const-refs-typed-const · `[u8;_]` sugar · the mapping-DSL
candidate.* Every §4 item is below with its demanding evidence, plus the asks §4
did not enumerate.

Legend: scale S/M/L uses the campaign's effort language (S = a focused increment;
M = a spec-able parcel; L = its own spec + multiple parcels / Spec-5-era).
"BN" = byte-neutral. Citations are `note/spec:section` or `ledger:line`.

---

## SECTION 1 — LANGUAGE asks (grammar / semantics)

### L1 · game-contract-hook construct
- **What.** A ratified `.emp` mechanism (extern-macro or link-time hook) letting a
  game define a hook body — `gameBootHook`, `gameDebugTick` — that the engine
  `.emp` calls, so the game's contract lives game-side instead of being MIRRORED
  in engine `.emp`. Spec-5 neighborhood.
- **Demanded by.** tranche-5 H2 ruling / t34 P1 keystone (kill-list rows 9, 45).
  `engine/system/game_loop.emp` and `engine/system/boot.emp` each carry a
  hand-copy of `games/sonic4/config/game.asm`'s macro EXPANSION (row 9's
  `jsr Debug_MusicToggle` combo, row 45's ping+autoplay hotkeys arm). Adjacent:
  kill row 90 (`config/game.asm`'s gate-aware `Game_Entry` numeric equ — a
  cross-seam label equalate that only a Spec-5 flip / link-time-equ-off-external-base
  resolves).
- **Workaround + cost.** `game_loop_port.rs`'s combo matrix and
  `boot_port::boot_hotkeys_shape_twin_parity` re-extract the REAL macro body from
  game.asm every run and byte-diff all four define combos; a LOCKSTEP comment sits
  at each mirror. Cost: two engine files must be edited in lockstep with a game
  file every time the macro body changes; the mirror is a standing drift hazard the
  gate contains but does not remove.
- **Retires.** The game_loop.emp + boot.emp mirrors and their drift matrices; kill
  rows 9 and 45 collapse; frees `games/sonic4/config/game.asm` ×2 (sonic4 + demo).
- **Scale.** L (Spec-5-era; "first game-contract macro with a NON-TRIVIAL body to
  port is the construct's demand moment", row 9).
- **Deps.** Dual-build death (Spec 5); interacts with row 90 (Game_Entry) and the
  kill-list row-4 stage-2 neighborhood.

### L2 · objdef / objentry human-authoring DSL
- **What.** A human-facing declarative surface for object-placement records — the
  packed entity entries K3 now emits as anonymous typed literals.
- **Demanded by.** K3 run A (`entity_data.emp`, census #29,
  `2026-08-01-k3-run-a.md`; K spec §4). The generator replaced the AS
  `objentry`/`objend` macros with `[u16;3] = [x, y, flags|(type<<8)|subtype]` + a
  `$FFFF` terminator — machine-authored and correct, but a human hand-authoring an
  act would be writing raw packed words.
- **Workaround + cost.** The generator (`ojz_entity_gen.py`) bakes the packing;
  there is no ergonomic hand-authoring path. Cost is latent (all current authors
  are generators), realized the moment a human wants to place objects in `.emp`.
- **Retires.** Nothing today (no consumer is blocked); it is a NEW-capability ask,
  not a scaffolding-remover.
- **Scale.** M. **Deps.** None hard; pairs conceptually with L7 (mapping-DSL) as
  the "object-authoring surface" cluster.

### L3 · relaxation-aware `align` / per-data-item alignment attribute
- **What.** Either (a) an `align` that participates in the relaxation fixpoint
  (re-evaluate pad position after branch sizes settle), or (b) a per-item attribute
  `pub data X: align(2) = embed(...)` pinning the item's own base independent of
  what precedes it.
- **Demanded by.** conv-i8 (`2026-08-01-conv-i8-vectors.md`; ledger 1942).
  Folding the five golden-vector `embed()`s into `compression_selftest.emp` after a
  proc holding `bra.s`/`beq.s` relaxables fired `[align.provisional] alignment pad
  at a provisional position` — the align can't resolve until the proc's size pins.
  (SPEC2 D2.29 already records `[align.provisional]` as the v1 loud stop.)
- **Workaround + cost.** The aligns were dropped; every golden blob is even-length
  today, so labels word-align with no padding; five comptime
  `ensure(_Vec_X.len % 2 == 0, …)` guards fire if a future encoding makes a blob
  odd. Cost: any port folding data AFTER relaxable code in one section hits the same
  wall.
- **Retires.** The five ensure guards; unblocks the data-after-relaxable-code shape
  generally.
- **Scale.** S–M. **Deps.** The relaxation fixpoint (option a) or the data-item
  layout path (option b).

### L4 · struct-scope `@allow("layout.odd-field")`
- **What.** Let a struct (or module) silence `[layout.odd-field]` the way the
  DATA-side `[layout.odd-item]` already consults `allows_lint`.
- **Demanded by.** sound-constants E1 (`2026-08-01-sound-e1-flip.md` §8;
  K spec §4). `check_struct_odd_fields` (layout.rs) emits UNCONDITIONALLY; the 5
  Z80 sound structs (DacSample/FmPatch/SfxHeader/SfxChannel/SeqChannel) have
  intentionally-unaligned words and no way to declare that fact.
- **Workaround + cost.** None needed — it is warning-tier only, so E1 shipped
  with the noise. Cost: an unsilenceable lint on intentionally-packed Z80 records.
- **Retires.** The warning noise on every Z80-struct-bearing module.
- **Scale.** S. **Deps.** None (mirrors the existing `layout.odd-item` allow path).

### L5 · typed-const initializer referencing another typed const
- **What.** `pub const X: VramTile = OTHER_TYPED_CONST + n` — resolve a typed const
  named in another typed const's initializer (today: "unknown name" in initializer
  scope).
- **Demanded by.** conv-f #21 (`2026-08-01-conv-f-game-config.md`; ledger 1917).
  `VRAM_TEST_MARKER` lost its `= VRAM_TEST_OBJ + $18` derivation — `config/constants.emp`
  had to use the literal `$03F8` with the derivation demoted to a comment.
- **Workaround + cost.** Literal value + derivation-in-comment. Cost: the intent
  (marker = obj base + $18) is no longer machine-checked; a future base change
  silently desyncs the marker.
- **Retires.** The literal; restores the checked derivation.
- **Scale.** S. **Deps.** The newtype/type-layer (relates to L8's SfxId/VramTile
  newtype work).

### L6 · `[u8; _]` inferred-length sugar
- **What.** `_` in array-length position as a genuine inference hole.
- **Demanded by.** conv-h2 embed (`2026-08-01-conv-h2-embed.md`; ledger 1935;
  embed spec §1). `_` currently parses as an ordinary path (not a hole) in
  `layout.rs::resolve_type`.
- **Workaround + cost.** OMIT the annotation entirely (`pub data X = embed(...)`)
  — omit-to-infer works and the embed spec §1 blesses it as the v1 whole. So the
  `_` form is purely COSMETIC readability parity with the spec text. Near-zero cost.
- **Retires.** Nothing functional. **Scale.** S (cosmetic, low priority).
  **Deps.** None.

### L7 · shared mapping-DSL / mapping-struct home
- **What.** A shared engine module (e.g. `engine.objects.mappings`) exporting the
  VDP sprite-mapping frame structs (MapPiece / MapFrame1 / spr_size / centered) so
  both games `use` it instead of authoring locally.
- **Demanded by.** conv-hdemo (`2026-08-01-conv-hdemo.md`; ledger 1939; K spec §4
  "the mapping-DSL candidate"). MapPiece/MapFrame1/spr_size/centered are DUPLICATED
  between `games/sonic4/data/mappings/test_mappings.emp` and
  `games/demo/data/demo_data.emp`.
- **Workaround + cost.** Each authors its own copy (the demo keeps one so the
  template stays self-contained). Cost: two hand-maintained copies of the mapping
  frame format.
- **Retires.** The duplication (re-home both copies).
- **Scale.** M — `engine.objects.frames` today holds only code-emitting comptime
  fns, not data structs, so the home is a new engine-module addition. **Deps.**
  None hard; clusters with L2 (object-authoring surface).

### L8 · typed sound-id newtypes (SFXID_RING_* : SfxId, SONG_* : SongId)
- **What.** Promote the sound-id families to newtypes (SfxId / SongId / MusicId) so
  the typed mirrors become the definition rather than a hand-synced block.
- **Demanded by.** conv-f2 #24 (`2026-08-01-conv-f2-sound-ids.md`; ledger 1920;
  census #24; kill row 81). Explicitly PRE-RULED deferred to the language round —
  `sound_api.emp` keeps its typed `SFXID_RING_*`/`SONG_*: SfxId/SongId` mirror.
- **Workaround + cost.** The typed mirrors survive alongside the untyped
  `games.sonic4.sound_ids` values. Cost: a hand-maintained typed mirror per family.
- **Retires.** The typed sound-id mirrors in sound_api.emp.
- **Scale.** M. **Deps.** The **T1 newtype roadmap** (MEMORY: "T1 item-13 =
  MusicId/SfxId · AnimId/MappingFrame · VramTile/VramAddr" already ruled; rule =
  "moved+compared pays"). This ask IS the campaign's concrete demand for that
  ruling. Interacts with L5 (typed-const arithmetic).

### L9 · offsets-DSL cross-module `Ref` path (player-state offset tables)
- **What.** Extend the `offsets` construct so a table's targets can be cross-module
  — the §4.7 deferred knob "cross-module/multi-segment targets (folds into the
  S2-D3 module-resolution work)".
- **Demanded by.** conv-d (`2026-08-01-conv-d-gated-twins.md`; ledger 1767, 1906;
  census OQ4/Parcel D). The 3 player-state offset tables
  (`Player_States`/`EnterHooks`/`ExitHooks`) keep the
  `extern("Label") - extern("Player_States")` difference form; overseer pre-ruled
  they stay byte-identical, DSL adoption deferred to its own parcel / Spec 5.
- **Workaround + cost.** The extern-difference form (byte-identical). Cost: the
  self-relative-offset idiom is spelled by hand as a subtraction rather than
  declared as an `offsets` table.
- **Retires.** The extern-difference hand-form in player_common.emp.
- **Scale.** M (its own parcel). **Deps.** S2-D3 module-resolution; note SPEC2 §4.7
  also defers `dc.l` offsets, `base:`/`start:` overrides, Z80 offset tables — the
  same construct's knob backlog.

### L10 · typed comptime `Data` carrying relocations (`to_data(typed_value)`)
- **What.** A comptime `Data` value that carries pointer relocations, so a
  typed-struct-plus-array can be concatenated (`Struct ++ Array.map(...)`) and
  lowered literally.
- **Demanded by.** conv-g parallax (`2026-08-01-conv-g-parallax.md`; ledger 1927).
  The spec's Appendix-A `ParallaxConfig{…} ++ bands.map(band_entry)` sketch does
  NOT lower — `++` needs matching operand kinds and pointer relocations flow through
  a data-item's declared type, not a comptime `Data`.
- **Workaround + cost.** Nested `ParallaxCfgN` wrapper structs — one per shipping
  band count (1/2/4/5) — exist ONLY to carry the header+bands byte shape for typed
  emission. Cost: N near-identical wrapper structs instead of one function.
- **Retires.** The `ParallaxCfgN` wrapper structs collapse to a single
  `parallax_section(cfg, bands) -> Data` fn.
- **Scale.** M. **Deps.** Relocation-in-comptime-Data machinery.

### L11 · same-module data-label ref in a data initializer — DECIDE: ask or idiom?
- **What.** In a data initializer, a SAME-MODULE data label used as a bare
  identifier fails "unknown name"; it must be `extern("Label")`. The observed rule:
  "bare = cross-module (resolves), `extern()` = same-module forward/back ref in
  data".
- **Demanded by.** K3 run A (`2026-08-01-k3-run-a.md`; ledger 1951). Hit twice:
  the `OJZ_Act_Pool_PageTable` page pointers and the block-blobs dedup alias.
- **Workaround + cost.** `extern("…")` for same-module refs (shipped, byte-clean).
  Cost: an asymmetry an author must learn (bare works across modules but not
  within one).
- **Retires.** The asymmetry (if bare same-module refs are made to resolve).
- **Scale.** S. **Deps.** None. **Flag for the spec owner:** decide whether this is
  a grammar ask (make bare same-module data refs resolve) or merely an idiom to
  document.

### L12 · `use` multi-line / trailing-comma braced form
- **What.** A braced multi-name `use { A, B, C }` (and trailing-comma) import form.
- **Demanded by.** K3 islands (`2026-08-01-k3-islands.md:168`). A braced `use`
  gives `parse error: expected imported name, found RBrace`; folded to a single-line
  `use`.
- **Workaround + cost.** Single-line `use`. Cost: cosmetic only.
- **Retires.** Nothing functional. **Scale.** S (cosmetic). **Deps.** None.

### L13 · parallax_combine / parallax_combine_split authoring sugar
- **What.** Convenience `configs.emp` comptime fns for uniform or split
  multi-effect band stacks.
- **Demanded by.** conv-g (ledger 1925). The two macros of `parallax_macros.inc`
  had 0 / 1 consumers; the one use inlined more clearly as a direct 2-band config.
- **Workaround + cost.** Hand-author bands / inline `hdr()`+`band()`. Cost: none
  today (dropped by design).
- **Retires.** Nothing. **Scale.** S (re-add on demand). **Deps.** None. Low
  priority — listed for completeness; "re-add if a game wants uniform/split stacks".

### L14 · cross-module `sizeof`/derived-const scope ergonomics
- **What.** Two small ergonomics: (a) `sizeof` of a struct whose fields carry
  newtypes needs the type vocabulary in the consuming module's scope
  (`use engine.types.*`); (b) importing a DERIVED `pub const` evaluates its RHS in
  the CONSUMER's scope, so its base consts must also be in scope.
- **Demanded by.** item-7b engine RAM port (`2026-08-01-item7b-engine-ram-port.md`
  §8 step-3).
- **Workaround + cost.** Glob `use engine.constants.*` / `use engine.types.*` (the
  clean answer, matches tile_cache.emp). Cost: an author must import the type/base
  vocabulary transitively; no diagnostic points at it.
- **Retires.** Nothing (workaround is clean). **Scale.** S. **Deps.** Module
  resolution. Low priority — a DX/diagnostic note, not a blocker.

---

## SECTION 2 — TOOLING asks (repin / harness / oracle)

### T1 · RAM map report (sigil-emitted per-region address map)
- **What.** A `sigil`-emitted per-region map: name, address, size, padding,
  headroom vs budget. Pure tooling, no language surface.
- **Demanded by.** ram.asm audit (ledger 206–210) — "never know what their real
  number is". This is the ONLY still-OPEN row of the (otherwise realized)
  vars/RAM-regions section.
- **Workaround + cost.** None; the address is currently invisible on the page.
- **Retires.** Closes the last vars/RAM section row. **Scale.** S (cheap; "could
  ride any tranche"). **Deps.** Spec-3 inlay hints are the eventual in-editor form.

### T2 · `emulator_memory_hash(addr, len)` — PARTIALLY superseded, VERIFY
- **What.** A CRC32/md5 over a memory range computed emulator-side (no bytes cross
  the agent). Oracle backlog row 21.
- **Demanded by.** pass-2 step-1 (ledger 1136) — the 14400-byte cache
  byte-identity bar is impractical with only `read_memory` (context + output-token
  blowup; a false nametable diff from a hand-transcribed hex drop cost a diagnosis
  cycle). Re-cited by opt-sweep design §3.2 ("the single highest-value tool …
  recommend building before the first tile_cache/plane_buffer PF parcel").
- **Status — verify closure.** A whole-state **`state_hash`** bus method HAS
  shipped and is in use: the collision-lookup wave
  (`2026-08-01-waveb-collision-lookup.md:125`) reports the emulator-side
  `state_hash` seeing "zero run-to-run variance" over the framebuffer + full VDP
  register file. BUT the parametric addr/len range hash the ledger asks for is not
  clearly closed — the same wave still compared 64 KB RAM byte-by-byte ("exactly
  ONE differing byte per anchor"), i.e. it did NOT range-hash RAM. **Verdict:
  whole-state identity is answered; the parametric `memory_hash(addr,len)` for
  sub-region PS bars is likely still open — confirm with the oracle owner before
  the agenda lists it.**
- **Scale.** S–M. **Deps.** Oracle core.

### T3 · A/B runner script (paired capture harness)
- **What.** A thin runner: `OLD.bin + NEW.bin + scene script` (reset → poke →
  run_to frame N → capture) → the paired screenshot + region-hash set for a packet.
- **Demanded by.** opt-sweep design §3.2 — codifies frame-anchored determinism so
  each optimization parcel doesn't re-improvise the A/B capture.
- **Workaround + cost.** Each parcel hand-scripts its A/B captures. Cost: repeated
  boilerplate + determinism-drift risk per parcel.
- **Retires.** Per-parcel capture boilerplate. **Scale.** S. **Deps.** Pairs with
  T2.

### T4 · phase-aware repin (pin LMA for `bank:`/`vma:` sections)
- **What.** repin should pin the LMA for phase-bank sections instead of deriving
  the pin as the phase VMA (which forces a +$50000 fixup in the golden gate).
- **Demanded by.** K4 inc-5 Stage 4b (`soundbankhead.emp`; ledger 1966). The pin
  `SOUNDBANKHEAD` derives as the phase VMA ($8000), so the gate reads
  LMA = VMA+$50000; cosmetic under `Frozen` (shipped) but would MISPLACE under a
  `PinnedBaked` re-bootstrap (untested).
- **Workaround + cost.** The frozen key drives the shipped path; the +$50000 fixup
  is manual in the gate. Cost: a latent misplacement hazard on any future
  PinnedBaked re-bootstrap of a phase bank.
- **Retires.** The +$50000 gate fixup; de-risks phase-bank re-bootstrap.
- **Scale.** S. **Deps.** repin internals. Low priority (frozen key covers the
  live path).

### T5 · repin's asl-`.lst`-parse retirement (row 34 P4c)
- **What.** Retire repin's parse of the asl `.lst` for placement facts.
- **Demanded by.** kill-list row 34 P4c; explicitly named at K5 (ledger 1977) as
  "what K5 did NOT close — the only open sub-item of row 95".
- **Workaround + cost.** `pins.rs` / `repin.toml` SURVIVE by the pins ruling as
  repin-generated test snapshots (orthogonal to the K5 placement-authority flip).
  Cost: repin still depends on the asl `.lst` format.
- **Retires.** The `.lst`-parse dependency (pins.rs/repin.toml survive as
  snapshots). **Scale.** M. **Deps.** The pins ruling (they stay as snapshots).

### T6 · vestigial repin `gate_blocks()` cleanup — ✅ ALREADY CLOSED (A1 rider)
- **What.** Drop `bin/repin.rs`'s `gate_blocks()` rendering + its two print sites +
  two unit tests.
- **Demanded by.** K4 inc-6B (ledger 1971). Those `org`-snippet paste blocks
  existed to be pasted into `SIGIL_EMP_*` `else`-arms in engine.inc / main.asm —
  all deleted at K4, so the blocks have no destination.
- **✅ CLOSED (A1, 2026-08-02 — the rider confirmation).** The A1 porter grepped
  the whole workspace (`grep -rn 'gate_blocks' . --include='*.rs'` → zero matches):
  `gate_blocks()`, its print sites, and its unit tests are ALREADY GONE from
  `bin/repin.rs` / `repin.rs` (removed by a prior cleanup, not this parcel). The
  rider had nothing to drop — confirmed no other consumer reads the rendered orgs.
  Ledger 1971's "Kill:" clause is consummated.

### T7 · SIGIL_EMP_* pure-identifier gate cull
- **What.** A sweep to distinguish gates that still gate something (the sound-on
  selectors: MT/DAC/SFX) from pure-identifier leftovers, and drop the AS `-D`
  emission of the latter (~40 gates in `native.rs code_gates`).
- **Demanded by.** K4 inc-6B (ledger 1972). With every `.asm` twin + engine.inc /
  main.asm deleted, the gates arm no `ifndef` in the residual (game_root.asm has
  none).
- **Workaround + cost.** Harmless unused `-D` defines. Cost: naming/clarity noise.
- **Retires.** The pure-identifier `-D` emissions. **Scale.** S. **Deps.** None.
  Low priority.

---

## SECTION 3 — ARCHITECTURE items

### A1 · P1 — seam-2 registry unification — ✅ CLOSED (A1, 2026-08-02)
- **What.** Unify the seam-2 registry (the emit-tool architecture's ROM-data
  placement registry). LEDGERED as a post-K consolidation.
- **Demanded by.** K-capstone spec §6. The K4 sound-bank pass ran as **P2** (native
  sections `embed()` the seam-2-emitted `.bin` at declared anchors; the emit-tool
  architecture left untouched by design — `build.sh` REQUIRES `SIGIL_EMIT`). "P1
  (seam-2 registry unification) is LEDGERED … its demand moment is when the
  emit-tool architecture itself needs changing."
- **✅ CLOSED (Parcel A1, arc spec §2).** The demand moment arrived: A2 changed the
  emit tool (the mt_syms split), and A1 rode the same arc. `seam2.rs` no longer
  hard-codes the ~10 LMA consts (registry 2) — `sound_layout()` DERIVES every
  banked LMA from `games/sonic4/map.toml`'s two declared anchors (`dac_banks` /
  `sound_bank`) + the emit's own measured artifact lengths, validated against the
  map's declared `order`. The consts are DELETED (consumers now read
  `SoundLayout`); `seam1`'s `DacSampleTable` window VMA flows from the same
  derivation. `pins.rs` / `tests/repin_pins.rs` stay LITERAL as the independent
  drift detectors (the emit no longer self-certifies). A doctored map anchor moves
  the derivation; a reordered `order` fails loud. Provenance-only: byte-identical
  ×6, no re-freeze (chain 22), `repin --check` clean.
- **Retires.** The dual placement registry (map + seam2 hardcodes). **Scale.** L.

### A2 · mt_syms emit split (the one non-full-native sound residue)
- **What.** Emit `SongTable` / `SongPatchTable` as separate tiny artifacts (or a
  split-`embed()` carrying the emitted offset) so the `.emp` owns them and
  `mt_syms{,_debug}.asm` drops.
- **Demanded by.** K4 inc-5 Stage 3 + inc-6B (ledger 1960, 1970; census #9/#10).
  The two labels sit at mid-blob offsets (len − SONG_COUNT*8/4) a single `embed()`
  can't label; `sound_api.emp` externs them, so the `include` re-homed to
  `game_root.asm` (SIGIL_EMP_MT-gated) rather than dying.
- **Workaround + cost.** The `mt_syms` include survives as the one non-full-native
  sound residue. Cost: two AS `include`s block "100% .emp" for the sound bank.
- **Retires.** `mt_syms.asm` + `mt_syms_debug.asm`; closes the last sound residue.
- **Scale.** M (touches the emit pipeline — "P2 keeps it untouched by design", so
  this is deliberately post-P2 / A1-adjacent). **Deps.** A1 (emit architecture).

### A3 · comptime section-length primitive
- **What.** A comptime primitive that reads a data section's / table body's emitted
  LENGTH, so counts/spans derive from the data instead of hand literals. Gap-ledger
  row 1805.
- **Demanded by.** sound-constants E2 (ledger 1910–1911) — `FMVOLENV_COUNT` /
  `PSGVOLENV_COUNT` have NO comptime source (only seam1 `-D` values); ideally
  derived from the vol-env DATA length in `sound_tables_z80.emp`. Same blocker as
  `dac_sample_tab.emp`'s `10*9` hand-literal LHS (sound E1 §8 item 2) and the
  vol-env span guards.
- **Workaround + cost.** The counts stay honest emit-config in `seam_emit_config`;
  the `10*9` stays a hand literal. Cost: a length invariant maintained by hand
  rather than derived; drift possible if the data grows.
- **Retires.** The hand-literal counts/spans; enables the vol-env control-byte
  harvest (ledger 1910, which additionally touches `gen_sound_tables.py`).
- **Scale.** M. **Deps.** Could be classed language (a comptime builtin) or
  tooling; listed here as architecture because it spans the generated-data seam.

---

## SECTION 4 — ALREADY ANSWERED (resolved in flight — do not re-litigate)

- **binary-embed / incbin construct → SHIPPED as `embed(...)`.** conv-h #12 asked
  for it (ledger 1930); conv-h2 shipped `embed("root/rel/path.bin")` →
  `Value::Data` (ledger 1934, embed spec). The `[u8;_]` sugar (L6) is the only
  cosmetic remainder.
- **absolute-`org` / reserved-hole data surface → NOT NEEDED (K2).** conv-h2 feared
  boot_data's `org $3FE` hole needed a new surface (ledger 1934); K2 showed the
  hole is where the separately-chained `z80_init` idle physically LANDS, so
  CONTIGUOUS declared-order packing + a `BootData_End` frozen key solved it with no
  new surface. Row closed; "no other consumer wants them".
- **the `offsets` construct → SHIPPED (D2.15, §4.7).** Bidirectional self-relative
  table; used by conv-h #35 (`test_mappings.emp`). The cross-module `Ref` path
  (L9) and the §4.7 knob backlog (dc.l, base:/start:, Z80) remain deferred.
- **conditional `vars` fields + the whole vars/RAM-regions feature (item #7) →
  SHIPPED.** ram.asm audit blocked on comptime-`if`-over-fields (ledger 180–188);
  item #7a built it (`if DEBUG == 1 { … }` + `@shape_divergent`), #7b/#7c ported
  all three ram.asm files. `alias()`+`ensure` buffer-reuse, `@align(N)`,
  `[vars.shape-divergent]` all shipped. Only the RAM map report (T1) stays open.
- **the map manifest ask (declared ORDER + island anchors + budgets) → REALIZED by
  K1/K5.** The waveb close packet's packing walk "wants the map manifest to own
  DECLARED ORDER + ISLAND anchors" (`2026-07-31-waveb-close-packet.md:49`); K1
  built the map (keying by min-offset label, excluding zero-byte markers —
  `2026-08-01-k1-map-authority.md:122`), K5 flipped it to the placement AUTHORITY
  (`packed_true_bases` drives from the declared `order`; `sigil.map.toml` retired;
  frozen tables demoted to a measurement cache). Kill row 95 all-but-closed (only
  row 34 P4c / T5 remains).
- **object-bank budget as a map region → DONE (Stage-3 / K5).** kill row 95;
  `RegionKind::ObjectBank` + `MemoryMap::check_budget`; engine.inc's `if * >$20000`
  deleted.
- **game-constants `.emp` module (buttons + SONG_*/SFXID_*) → CLOSED.** kill row 54
  / row 81: BUTTON_A/B/C/START hoisted to `engine.constants` (conv-b); SONG_* /
  SFXID_* moved to `games.sonic4.sound_ids` (conv-f2); the 16 mirror consts + their
  `ensure(extern)` guards deleted.
- **SFX_ID_BASE / SFX_COUNT / SFX_TABLE_LEN family → DISSOLVED (conv-f2).** The
  quadruple mirror collapsed into `sfx_bank.emp`'s derivation via
  `eval_all_pub_consts` on a data module.
- **whole-state identity hashing → SHIPPED (`state_hash`).** The framebuffer + VDP
  reg-file hash used in collision-lookup. (The parametric range hash T2 is the part
  to verify.)

---

## SECTION 5 — TOP 5 BY IMPACT (ranking only; the spec owner designs)

1. **L1 game-contract-hook construct** — the single largest structural remainder:
   it frees `game.asm` on BOTH games, retires two engine-side lockstep mirrors
   (game_loop + boot) and their drift matrices, and is the gateway to Spec-5
   dual-build death. Named in K spec §4 first.
2. **L8 typed sound-id newtypes (+ L5 typed-const arithmetic)** — the campaign's
   concrete demand for the already-ruled T1 newtype roadmap (MusicId/SfxId/
   VramTile); retires standing typed mirrors in sound_api.emp and restores checked
   derivations (VRAM_TEST_MARKER). Two asks that share the type-layer.
3. **A1 seam-2 registry unification (with A2 mt_syms split)** — the last barrier to
   a truly "100% .emp" sound bank; A2's `mt_syms{,_debug}.asm` is the one surviving
   non-native sound residue, and it is A1-gated.
4. **L3 relaxation-aware `align` / per-item align** — a real, hit-in-flight
   language wall (conv-i8) that any future data-after-relaxable-code port re-hits;
   currently patched with an even-length invariant + guards.
5. **T2 `emulator_memory_hash(addr,len)`** — "the single highest-value tool"
   (opt-sweep §3.2) for the §17 optimization arc's PS-class identity bars; a
   whole-state `state_hash` shipped but the parametric sub-region hash needs
   verification and is recommended before the next tile_cache/plane_buffer parcel.

Honorable mentions (high value / low cost): **L4 struct-scope `@allow`** and
**T1 RAM map report** are both S-scale and close standing lint/visibility gaps;
**L2 objdef DSL** + **L7 mapping-DSL** are the "human authors a new game in `.emp`"
cluster whose demand arrives with the first hand-authored act.
