# The A1/A2 seam-2 arc — design (ruled 2026-08-02)

The post-K architecture arc from the language-round agenda: **A2** (mt_syms emit
split — retires `mt_syms{,_debug}.asm`, the last non-native sound residue),
**A1** (seam-2 registry unification — the emit tool becomes a consumer of the
map authority), **A3** (comptime emitted-span primitive) folded in per the
agenda ruling. Grounding: language-round ledger §SECTION 3, K-capstone design
§6, the K4 dissolution packet (Stage 3 + inc-6B), and the 2026-08-02 as-built
survey (countersigned against sigil `ae5a2324` / aeon `7779d84`).

## §0 — The demand moment, satisfied

K-capstone §6 ledgered P1 with an explicit test: *"its demand moment is when
the emit-tool architecture itself needs changing."* A2 cannot be done without
touching the emit tool (the two labels sit at shape-dependent mid-blob offsets
only the emit's own lowering knows), so A2 **is** the demand moment and A1
rides the same arc. The arc does NOT dismantle `emit_sound_blob` — the
emit-tool architecture is a deliberate shipped design (`build.sh` REQUIRES
`SIGIL_EMIT`); the arc changes what the tool *knows* (map-derived placement)
and what it *emits* (split MT artifacts), not that it exists.

## §1 — Parcel A2: the mt_syms kill (three-way split) — FIRST

**As-built:** `seam2::emit_mt_artifacts` (seam2.rs:805) writes
`mt_bank{,_debug}.bin` + `mt_syms{,_debug}.asm` (SongTable/SongPatchTable as
absolute equs at blob-end − SONG_COUNT*8/4; SONG_COUNT 1 plain / 3 debug).
`mt_bank_blob.emp` embeds the bin as ONE `pub data Song_MovingTrucks`;
`game_root.asm:31-37` includes the syms file (SIGIL_EMP_MT + __DEBUG__ gated);
`sound_api.emp:72-73` externs the two labels. mt_syms is one of the four
sanctioned AS survivors (K packet §0).

**Ruled form — the three-way split** (the ledger's "separate tiny artifacts"
option, executed so the labels become native):

- `emit_mt_bank` already knows the two label offsets from its own lowering
  (seam2.rs:781-794 reads the PLACED section's labels). Change
  `emit_mt_artifacts` to cut the blob there and write THREE artifacts per
  shape: `mt_bank_body{,_debug}.bin` (bytes [0, SongTable)),
  `mt_songtable{,_debug}.bin` (SONG_COUNT*4 bytes),
  `mt_songpatchtable{,_debug}.bin` (SONG_COUNT*4 bytes). The `mt_syms*.asm`
  emission is DELETED.
- `mt_bank_blob.emp` places them as three contiguous labeled members (the
  shipped soundbankhead/dac_banks mechanism):
  `pub data Song_MovingTrucks = <body>` · `pub data SongTable = <st>` ·
  `pub data SongPatchTable = <spt>`, each selecting on DEBUG. Add `.len`
  size walls (`SONG_COUNT*4` each — SONG_COUNT already comptime-reachable via
  the sound-constants authority; if not, drift-guard vs extern).
- `game_root.asm` drops the whole gated include block (sonic4's stub shrinks
  toward the demo stub's shape). `sound_api.emp`'s externs are UNCHANGED —
  they now resolve against native section labels at whole-ROM link.
- Delete stale references to mt_syms in comments/docs at touch sites
  (mt_bank_blob.emp header comment, game_root.asm comment), present-tense
  contract facts only.

**Why not the alternatives (logged):** a single split-embed offset construct =
new language surface for one consumer; full-native inline lowering of
`mt_bank.emp` in the main build = breaks the DAC/MT/SFX emit symmetry, drags
the seam-const environment into the whole-ROM build, and contradicts the ruled
"P2 keeps the emit pipeline" design. Declined.

**Identity bar:** the concatenation of the three artifacts is byte-identical
to the old blob (assert this in the emit gate: body+st+spt == old lowering's
bytes); anchors IDENTICAL ×6 expected. The deb2 appendix MAY shift (the two
labels change form equ→section label in the listing) → the sanctioned
appendix-only re-freeze (`refreeze --freeze a2-mtsyms`, no `--ab`) if full
CRCs move; if fully neutral, no re-freeze. **ANY assembled-anchor move is a
STOP** (the bank-anchor rule stays armed). `pins.rs` SONG_TABLE /
SONG_PATCH_TABLE pins are UNCHANGED (addresses don't move) and now
countersign the native labels. mt_bank_port + mt_negative_probes stay green
(emit-first; update the probes if they touch the syms output). Kill-list:
close the mt_syms row; the K packet's "honest 100%" survivor list drops 4→3
(update the packet's §0 claim where quoted in living docs, not history).

## §2 — Parcel A1: registry unification (emit reads the map) — SECOND

**As-built:** two placement registries state the same facts. Registry 1 (the
authority, post-K5): `games/sonic4/map.toml` anchors (`dac_banks at=0x48000`,
`sound_bank at=0x58000 vma=0x8000`) + declared `order`. Registry 2:
`seam2.rs`'s ~10 hardcoded LMA consts (DAC_BLIP/SHARED 0x48000/0x50000,
SOUND_TABLES_Z80 0x58000, then the pack-result chain: PITCHTABLE 0x58357,
SFX_WIN_TAB 0x5845F, SEQ_OPCODE_TAB 0x5856D, DAC_SAMPLE_TAB 0x585AD,
MT_BANK 0x58607, SFX_BANK 0x5BAE8/0x5D53A). A placement move currently
requires re-pinning both. That is the exact duplication A1 names.

**Ruled form:** the emit tool derives its placement from the map + its own
artifact lengths:

- Parse the two DECLARED anchors from `games/sonic4/map.toml` (the tool
  already takes `--aeon`; parsing scope = just what it needs, not a second
  map engine — reuse the harness's existing map reader if importable).
- Derive the chain: soundbankhead member LMAs = sound_bank anchor + running
  offsets of its own emitted heads (it emits every one of them);
  MT_BANK_LMA = anchor + head span; SFX_BANK_LMA = MT_BANK_LMA + mt body len
  (per shape); DAC_SHARED = dac_banks anchor + $8000 (the declared intra-bank
  align). Every derived value must equal the old literal on master — assert
  equality during the parcel (a transition ensure), then the literals die.
- The derivation order must match the map's declared `order` slice for the
  sound bank — read it or loudly assert against it, so a future reorder
  cannot silently desync the emit.
- `pins.rs` / `tests/repin_pins.rs` stay LITERAL (they are the independent
  drift detectors — the whole point is the emit no longer self-certifies).
- Rider (small, same demand): the vestigial `repin` `gate_blocks()` paste
  blocks (ledgered K4 inc-6B — no destination since engine.inc/main.asm died)
  may be dropped IF the porter confirms no other consumer; else leave.

**Identity bar:** byte-identical ×6 (full CRCs unchanged, no re-freeze);
strict suite green; `repin --check` clean. This parcel changes provenance of
values, never values.

## §3 — Parcel A3: the comptime emitted-span primitive — THIRD

**Targets (the honest inventory, survey §3):**
1. `engine/sound/dac_sample_tab.emp:59` — `ensure(10 * 9 == …)` names this
   primitive as its explicit blocker. Retire the hand literal with a measured
   emitted span.
2. `FMVOLENV_COUNT`=3 / `PSGVOLENV_COUNT`=0x0B — ungoverned
   `seam_emit_config` literals (seam1.rs:772-773) with NO guard at all; the
   data that should drive them (`FmVolEnv_Ptrs` 3 entries /
   `PsgVolEnv_Ptrs` 11 entries) lives in `sound_tables_z80.emp` one module
   away. Derive the counts from the data (pub consts exported from the data
   module, resolved authority-first), delete the two config keys.

**Design latitude (porter designs, mini design note required before code):**
the gap is "emitted byte span of a lowered section/table body" — `.len`
exists on `Value::Data`/arrays/tables but not on lowered sections.
Preferred shape: the cheapest honest mechanism per target — if the vol-env
ptr tables are `table`/`offsets` constructs whose `.count`/row facts already
fold, use those and no new primitive is needed for target 2; target 1 wants
the real span primitive (e.g. a comptime builtin over a same-module pure-data
section). Do not build a general section-length feature beyond what the two
targets + the ledgered span-guard ask (row 1654) justify — scope to the
demand, ledger the rest.

**Identity bar:** byte-identical ×6. Derivations must equal the old literals
(transition ensures prove it); emitted values NEVER change. New compiler
surface gets negative tests (a doctored-length table must fail the ensure).

## §4 — Hazards (binding)

- **Bank-anchor STOP rule** (K spec §6): any assembled-anchor move
  ($48000/$50000/$58000/$58607/$5BAE8/…) is a STOP, not an A/B.
- **Structural exclusivity** stays: native sections unconditional in sound-on
  registries; no BINCLUDE/stub arm may reappear.
- **Golden gates run emit-first** (`ensure_generated` before compare) — keep.
- **Byte-changing ripple doctrine, CURRENT surface** (the memory's five-site
  list is stale): pins.rs + tests/repin_pins.rs + the seam2.rs-derived values
  (post-A1: the map) + golden blobs/provenance.toml. engine.inc /
  mixed_dac_rom.rs / src-repin_pins.rs no longer exist.
- **Verify failures-first**; strict = `SIGIL_STRICT_GATE=1 SIGIL_EMIT=<sigil>/
  target/release/emit_sound_blob AEON_DIR=…/aeon cargo test --workspace
  --release`, expected 2973/0/4 baseline (grows with new tests);
  `refreeze -- --check` (tip m1-budget-fix, chain 21 at arc start);
  `repin -- --check` clean.
- **Shared checkouts:** all work in fresh worktrees (sigil-wt-a2 etc.);
  explicit-path commits only; never `git add -u` in the master checkouts;
  merge queue checks `origin/master` first.

## §5 — Sequencing + gates

A2 → A1 → A3, one porter per parcel, each on its own branch pair
(sigil + aeon where touched), sequential merge queue, overseer own-run
countersign at every gate (strict + ×6 + refreeze --check + repin --check +
the parcel's own identity bar). Each parcel ends with the standard packet:
step-3 vs step-5 findings + neither-bucket headlines; gap-ledger rows closed/
added same-branch; kill-list rows closed same-branch. After A3: the arc
packet, then THE CAMPAIGN RETROSPECTIVE (next on the agenda).
