# 2026-07-30 — level-gen CLOSE PACKET: THE OJZ LEVEL TREE IS REPRODUCIBLE-BY-TRACKING

Status: **CHECKPOINT (b). Stage (1) — the reproducibility fix — COMPLETE and
countersigned. Ruled OPTION 3: the parcel closes at stage (1); the generated-head
`.emp` conversions ride the post-flip arc.** The merge, provenance re-baseline, and
roadmap update are the overseer's. No push, no merge from the porter seat.

Tips: **aeon `level-gen` `d3ebed9`** · **sigil `level-gen` `39e590b`** (this packet +
the gap-ledger rows extend it). Strict **2906 passed / 0 failed / 1 ignored**.

Cites: `.asm`/`.py`/`.gitignore`/`build.sh` are aeon at `d3ebed9`; design/kill-list/
ledger are sigil at `39e590b`.

---

## THE HEADLINE

The OJZ generated level tree + the ROM collision tables were entirely gitignored and
regenerated every build from generators that read TWO out-of-repo donor projects
(`sonic_hack` / `skdisasm`) at hard-coded absolute paths. A fresh worktree lacked them
and either failed or — via `ojz_strip_gen.py:439 editor_data_available()`'s SILENT
fallback — built a **~131 KB wrong ROM with no error** (the row-178 hole;
`tools/seed-worktree.sh` was the standing workaround). **This parcel adopts the
sound-migration model: the generated tree is now COMMITTED bytes the build consumes
directly; generation moved to a MANUAL re-bake; the silent fallback is a HARD ERROR; a
fresh unseeded worktree builds the reference or fails loudly.** Byte-neutral throughout.

The Option-3 reframe that closed the parcel: **post-flip, `sigil-frontend-as` is
PERMANENT (Volence ruling), so the generated-head `.asm` are legitimate bucket-G
residual-AS FOREVER — not debt the flip must clear.** `entity_data.asm` does not need
to go native; it and the other heads stay residual-AS, assembled by `sigil-frontend-as`
in the native driver's single link (flip design §3.2-G). The stage-(2) `.emp`
conversions are a post-flip nicety, not a flip precondition — deferred (ledger below).

---

## §1 THE CENSUS (final)

### Generated tree — `games/sonic4/data/generated/ojz/act1/` (was 0 tracked → now committed)

**6 `.asm` + 43 `.bin` + 12 `.zx0` = 61 files.**

| generated `.asm` | emitted by | invoked (was) | AS consumer | convert-vs-embed class |
|---|---|---|---|---|
| `entity_data.asm` | `ojz_entity_gen.py generate` (tail of `ojz_strip_gen.py:1403`) | prebuild → **manual** | `main.asm:244` include; exports `OJZ_Sec{N}_{Objects,Rings,TypeTable}` to `act_descriptor.asm:77+` | SEMANTIC → generator-emits-`.emp` (post-flip) |
| `ojz_act_pool_manifest.asm` | `ojz_strip_gen.py generate` | prebuild → **manual** | `act_descriptor.asm:7`; parsed by the ZX0 step | SEMANTIC consts → `.emp` consts (post-flip) |
| `sec_block_dicts.asm` | `ojz_block_gen.py generate` | prebuild → **manual** | `act_descriptor.asm:18` | SEMANTIC consts → `.emp` consts (post-flip) |
| `ojz_act_pool.asm` | inline shell heredoc (now `regenerate-level.sh`) | prebuild → **manual** | `act_descriptor.asm:8`; BINCLUDEs the `.zx0` | MECHANICAL glue → `.emp` `embed()` or tracked glue (post-flip) |
| `sec_block_blobs.asm` | `ojz_block_gen.py generate` | prebuild → **manual** | `act_descriptor.asm:247`; BINCLUDEs `sec{N}_blocks.bin` | MECHANICAL → `.emp` `embed()` list (post-flip) |
| `bg_anim.asm` | `inject_editor_bg.py` (override present) | prebuild → **manual** | `act_descriptor.asm:263`; BINCLUDEs `bg_anim_banks.bin` | SEMANTIC table + BULK → `.emp` data + `embed()` (post-flip) |

The `.bin`/`.zx0` bulk (strips, tiles, blocks, pool pages, zone_bg, bg_tiles, banks,
palette) = OPAQUE, tracked + BINCLUDE'd. Hub: `data/levels/ojz/act1/act_descriptor.asm`
(`main.asm:245`, row-5 twin, rides Stage 2).

### Tracked additions (106 artifacts)

61 generated + **8 collision** (`data/collision/{,base/}*.bin`; the 4 non-`base`
BINCLUDE'd by `main.asm:282-291`) + **37 editor re-bake inputs** (`section_{0..8}.{tiles,
coll,collattr,collattrb}.bin` = 36, + `chunks_tiles.bin`). Scoped per OQ-2(b): `.*-backup/`
dirs, `ojz_bg_deep-forest-*` experiments, and the inert editor `export/*.bin` stay ignored.

### Generators

6 prebuild steps (5 python + 1 inline shell) + `ojz_common.py` shared lib + 3 MANUAL
authoring tools (`forest_bg_gen`, `gen_multi_band_bg`, `png_to_bg_override`). All 6
prebuild steps moved to the MANUAL `tools/regenerate-level.sh`; `prebuild.sh` is now a
documented no-op.

---

## §2 STAGE-(1) PROOF MATRIX

| proof | result |
|---|---|
| **both shapes @ commit A** (track) | `s4.bin` **eff2396f / 413577** · `s4.debug.bin` **1e9097bc / 421579** |
| **both shapes @ commit B** (decouple) | eff2396f / 413577 · 1e9097bc / 421579 (unmoved) |
| **PRIMARY assembled-ROM** | **e5765873 / dab4f06c** (proven by strict `mixed_seam1_rom_matches_reference_{plain,debug}`) — unmoved |
| **determinism (in-build)** | 0 tracked artifacts changed after a rebuild (prebuild's regen reproduced the committed bytes; now prebuild is a no-op) |
| **determinism (full re-bake)** | `tools/regenerate-level.sh` reproduces the committed tree exactly but for the `ojz_act_pool.asm` header comment (an assembler comment; zero ROM bytes) — synced in commit B |
| **FRESH-WORKTREE GATE (the headline)** | UNSEEDED detached worktree at `d3ebed9`, only `SIGIL_EMIT` set, NO seed step → both shapes build **eff2396f / 1e9097bc** directly |
| **fail-loudly** | absent donor/editor data ⇒ `ojz_strip_gen.require_donor()` hard error (no silent fallback); missing build-consumed file ⇒ asl include failure; internal drift ⇒ `verify_level_bin.py` preflight |
| **t24 non-vacuity of `verify_level_bin.py`** | control PASS; doctored `.zx0` wrapper size FAIL (clear msg); doctored manifest page count FAIL (3 msgs); restore PASS |
| **strict suite** (from sigil worktree, AEON_DIR→aeon worktree, own-built emitter) | **2906 / 0 / 1** — matches baseline exactly |

### Commits

- aeon `b8f9fdb` (1/n TRACK) · `d3ebed9` (2/n DECOUPLE: `regenerate-level.sh`,
  no-op prebuild, `require_donor()` hard error, env-overridable donor paths,
  `verify_level_bin.py` wired into build.sh preflight, `seed-worktree.sh` retired to
  an error-checked relic).
- sigil `fd4da32` (design note) · `39e590b` (kill-list row 93) · this packet + ledger.

---

## §3 KILL-LIST ROW 93 (DONE)

**Row 93 — the gitignored OJZ level tree + the fresh-worktree non-determinism.** Kill
condition = an unseeded `git worktree add` builds the reference or fails loudly.
**DONE (level-gen 1-2/n, aeon `d3ebed9`):** tree committed; generation manual;
`require_donor()` hard error; `verify_level_bin.py` preflight; `seed-worktree.sh` relic.
Per-conversion rows (entity_data → `.emp` etc.) are NOT added as kill rows — Option 3
makes them post-flip niceties over PERMANENT residual-AS, not scaffolding with a kill
condition; they live as gap-ledger rows (§5) instead.

---

## §4 RESIDUAL LIST — bucket-G residual-AS (through AND past the flip)

Per the Option-3 reframe (`sigil-frontend-as` permanent), these stay residual-AS,
assembled by the native driver's single link. NONE block the flip.

- **The 6 generated heads:** `entity_data.asm`, `bg_anim.asm`, `ojz_act_pool.asm`,
  `ojz_act_pool_manifest.asm`, `sec_block_blobs.asm`, `sec_block_dicts.asm` — committed,
  residual-AS, forever-OK (flip design §3.2-G).
- **`data/mappings/test_mappings.asm`** — hand-authored test sprite mappings
  (`main.asm:246`); tracked, not generated; residual-AS, opportunistic `.emp` (§3.2-H).
- **`data/levels/ojz/act1/act_descriptor.asm`** — the row-5 code-flip twin (has
  `act_descriptor.emp`); DELETES at **Stage 2**. Its Stage-2 deletion works fine against
  residual-AS `entity_data` (the `.emp`/`.asm` cross-frontend link in one image is
  exactly what the native driver does) — the retirement cluster needs NO entity_data
  conversion.
- **The `.bin`/`.zx0` bulk** — permanent BINCLUDE'd data.
- `verify_emit_bin.py` — confirmed SOUND-ONLY; untouched; retires with its sound `.asm`
  half at the flip (§1.5). `verify_level_bin.py` is its independent level analogue.

---

## §5 POST-FLIP LEDGER ROWS (deferred — gap-ledger, this commit)

Added to `campaign-gap-ledger.md`:

**(i) Generator-emits-`.emp` conversions of the generated heads (entity_data first).**
Ride LATE, at post-flip cost (one source, no twin lockstep, no dual build — the port-loop
PARCEL-SCOPE amendment). The act-cluster coupling is the reason they wait: `entity_data`
EXPORTS `OJZ_Sec{N}_*` to `act_descriptor.asm` (Stage-2 twin) and IMPORTS `ObjDef_*`
from `test_objects`/`path_swap` (Stage-2 row-5 twins, unpinned), so a pre-flip emit-to-
`.bin` deletion is blocked (the row-1620 SND_* class) and a native-`.emp` twin would add
a gate mid-flip. Post-flip, in the single native link, `entity_data.emp` resolves
`ObjDef_*` cross-module natively and its exports feed `act_descriptor` (or its successor)
directly — no syms bridge, no pin. Emission form per head is settled in the design note
§3 (entity_data → generator-emits-`.emp`; manifest/dicts → `.emp` consts; blobs → `.emp`
`embed()`; bg_anim → data + `embed()`).

**(ii) `ojz_act_pool.asm` tracked-glue note.** It is a mechanical BINCLUDE-list + `dc.l`
page table, now emitted by `tools/regenerate-level.sh` (the last heredoc). Converting it
to `.emp` `embed()` removes that heredoc, but tracked mechanical glue over the committed
`.zx0` is an acceptable resting state; lowest-priority of the §5(i) set.

---

## PER-PASS STEP-3 vs STEP-5 FINDINGS

**Step-3 (ceremony / faithful-transliteration):**
- The reproducibility hole was pure BUILD-HYGIENE ceremony, not code: a gitignored
  generated tree + a peer-copy seed script + a silent generator fallback standing in for
  "commit the artifacts." The sound stack had already retired the identical ceremony
  (tracked `.bin`, manual regen, `verify_emit_bin`); this parcel applied the SAME pattern
  to the level tree verbatim — zero new mechanism, one established model reused.
- The `ojz_act_pool.asm` header comment ("by build.sh") was a stale narration of the
  emitter's home; corrected to "by tools/regenerate-level.sh" at the move (byte-neutral).

**Step-5 (perf / hazard / correctness-hardening — the campaign's favorite):**
- **The silent-fallback hazard is closed STRUCTURALLY, not by a hand-fix.**
  `require_donor()` makes "re-bake without editor data" UNREPRESENTABLE — the exact
  pending-mechanism-marker practice (a compiler/tool net making the bug impossible rather
  than patching one instance). A re-bake can no longer emit the ~131 KB wrong tree.
- **`verify_level_bin.py` adds an independent, donor-free drift oracle** wired into the
  build preflight — referential integrity (page count == pages present == pool-include
  references; every BINCLUDE target present; a dict-len per section) + a `.zx0` wrapper
  roundtrip-lite (the wrapper's uncompressed-size header == its `.bin`). It catches a
  hand-edited head / missing blob / stale page BEFORE the ROM moves (t24-controlled,
  non-vacuous), which the whole-ROM byte gate only catches AFTER.
- **The donor absolute paths became env-overridable** (`AEON_SONIC_HACK_DIR` /
  `AEON_SKDISASM_DIR`) with loud errors — no silent machine-specific behavior.

**Neither-bucket headlines:**
- **The fresh-worktree reproducibility gate is now a first-class, provable acceptance
  bar** — the parcel's headline deliverable and a permanent guard against the class of
  bug that historically shipped a wrong ROM. `seed-worktree.sh` is retired; a checkout
  builds from tracked bytes.
- **Scope discipline held at a design fork:** the stage-(2) deletion was correctly
  identified as Stage-2-coupled (entity_data ↔ act_descriptor/objdef cluster) and
  escalated rather than forced; the Option-3 ruling (residual-AS permanent) made the
  whole conversion set a post-flip nicety, keeping this parcel's deliverable clean,
  self-contained, and byte-neutral.
