# 2026-07-30 — THE GENERATOR / LEVEL-TREE precursor parcel (DESIGN GATE ONLY)

Status: **DESIGN NOTE — answers the brief's six question sets. No implementation, no
twin deletion, no build change, no aeon modification.** Sigil branch `level-gen`,
worktree `.worktrees/level-gen`. This is the seam-2-for-level-data analogue and Spec-5
flip input #5 (`2026-07-30-spec5-flip-design.md` §3.2-G, OQ-3); NOT on the flip
critical path — Stage 2's residual-AS set shrinks by whatever lands here.

Baseline (from the brief, overseer-verified this morning): masters aeon `34023be` /
sigil `a08ec57`; PRIMARY assembled-ROM `e5765873` (plain) / `dab4f06c` (debug);
artifacts `eff2396f`/413577 · `1e9097bc`/421579; strict **2906/0 (1 ignored)**. No
build was triggered for this design gate; every claim below is a static read.

Cites: `.asm`/`.emp`/`.py`/`.gitignore`/`prebuild.sh` are aeon paths at `34023be`
(main checkout, strictly read-only); design/ledger/kill-list are sigil at `de8a124`.

---

## §0 — THE ONE LOAD-BEARING FACT (state it first)

**The generated level tree is NOT determined by the git-tracked sources.** It is a
function of (a) git-IGNORED editor working binaries and (b) TWO external sibling repos
referenced by HARD-CODED ABSOLUTE PATHS that are not in the aeon repo at all. Every
answer below turns on this. The "convert-vs-embed" reader test still applies to the
FORM of each output, but the reproducibility settlement (Q2) is the parcel's spine and
it is a TRACKING decision, not a conversion decision — exactly the shape the sound
generators already took (tracked `.bin`, manual re-bake, drift-gated).

---

## §1 — THE CENSUS (Q1)

### 1.1 The generated tree — `games/sonic4/data/generated/` (ENTIRELY gitignored)

`git ls-files games/sonic4/data/generated` = **0 tracked**. `git check-ignore` fires on
every file. Root cause: `.gitignore:75` `games/sonic4/data/generated/` (whole dir) plus
`.gitignore:2` `*.bin` and `:71` `*.zx0`. Contents: **6 `.asm` + 43 `.bin` + 12 `.zx0` =
61 files**, all `data/generated/ojz/act1/`.

| generated file(s) | emitted by | invoked | AS consumer (include/BINCLUDE site) |
|---|---|---|---|
| `entity_data.asm` | `ojz_entity_gen.py generate` (called at the tail of `ojz_strip_gen.py generate`, `ojz_strip_gen.py:1403-1404`) | prebuild (transitively) | `main.asm:244` `include` |
| `ojz_act_pool_manifest.asm` (page count + per-page VRAM slot/tiles equs) | `ojz_strip_gen.py generate` | prebuild.sh (`ojz_strip_gen.py generate`) | `act_descriptor.asm:7` `include`; ALSO PARSED by `prebuild.sh` (`sed -n 's/^OJZ_ACT_POOL_PAGES = …'`) to drive the ZX0 loop |
| `ojz_act_pool.asm` (page BINCLUDEs + `dc.l` page table) | **inline shell in `prebuild.sh`** (the `POOL_ASM` heredoc, after `salvador` ZX0 packing) | prebuild.sh | `act_descriptor.asm:8` `include`; BINCLUDEs `act_pool_page{k}.zx0` |
| `sec_block_blobs.asm` (per-section `BINCLUDE sec{N}_blocks.bin`) | `ojz_block_gen.py generate` (`ojz_block_gen.py:379`) | prebuild.sh | `act_descriptor.asm:247` `include` |
| `sec_block_dicts.asm` (`OJZ_SEC{N}_BLOCK_DICT_LEN` equs) | `ojz_block_gen.py generate` | prebuild.sh | `act_descriptor.asm:18` `include` |
| `bg_anim.asm` (band table `BgAnim_Table`/`BgAnim_Banks` + `BINCLUDE bg_anim_banks.bin`) | `inject_editor_bg.py` (`:77,:96`) when `editor_bg_override.json` exists (it does, tracked, 122 KB) | prebuild.sh (conditional) | `act_descriptor.asm:263` `include` |
| `act_pool_page{0..2}.bin` / `.zx0` | pool `.bin` = `ojz_strip_gen.py`; `.zx0` = `prebuild.sh` `salvador` wrapper | prebuild.sh | BINCLUDE'd via `ojz_act_pool.asm` |
| `sec{0..8}_blocks.bin` | `ojz_block_gen.py generate` | prebuild.sh | BINCLUDE'd via `sec_block_blobs.asm` |
| `sec{0..8}_strips_a.bin` / `_strips_source.bin` / `_tiles.bin` / `_tiles.zx0` | `ojz_strip_gen.py generate` | prebuild.sh | INTERMEDIATE — strips feed `ojz_block_gen`; not all BINCLUDE'd directly |
| `zone_bg.bin` | `ojz_strip_gen.py:1350` (base) / `inject_editor_bg.py:113` (override) | prebuild.sh | `act_descriptor.asm:257` `BINCLUDE` (`OJZ_Act1_BG_Layout`) |
| `bg_tiles.bin` | `ojz_strip_gen` / `inject_editor_bg.py` | prebuild.sh | `act_descriptor.asm:259` `BINCLUDE` (`OJZ_Act1_BG_Tiles`) |
| `bg_anim_banks.bin` | `inject_editor_bg.py` | prebuild.sh | BINCLUDE'd via `bg_anim.asm` |
| `ojz_palette.bin` | `ojz_strip_gen.py generate` (copied from sonic_hack) | prebuild.sh | `act_descriptor.asm:249` `BINCLUDE` (`OJZ_Palette`) |

The live act descriptor is `data/levels/ojz/act1/act_descriptor.asm` (`main.asm:245`),
which is the HUB that `include`s the generated `.asm` and BINCLUDEs the generated
`.bin`. It is TRACKED and already has a `.emp` twin (`act_descriptor.emp`, row-5 A-list
code-flip twin) whose `.emp` side ALSO reaches the generated tree AS-side — e.g.
`act_descriptor.emp:67` `act_art_pool_table: OJZ_Act_Pool_PageTable` and `:68`
`extern("OJZ_ACT_POOL_PAGES")`. So the generated tree is residual-AS in BOTH shapes.

### 1.2 The editor tree — `games/sonic4/data/editor/` (27 tracked, rest ignored)

`git ls-files games/sonic4/data/editor` = **27 tracked**. The tracked set is the
SEMANTIC authoring surface:
- `objects.json` (object library: id→codeLabel/subtype/properties)
- `ojz/act1/section_{0..8}.objects.json` + `.rings.json` (per-section placements),
  `section_0.meta.json`, `ojz/chunks.json`, `ojz_bglib.json`
- `ojz/act1/export/{act_descriptor,entity_data,vram_bases}.asm` (editor's raw export —
  DISTINCT from the live `levels/…/act_descriptor.asm` and the generated
  `generated/…/entity_data.asm`; the export copies are the editor's own emission)
- `bg_src/{ojz_cave_lilypad,ojz_forest_flowers}.png` (procedural-BG source art)

**IGNORED** (present on disk, `git status --ignored` `!!`): every `section_{N}.tiles.bin`,
`.coll.bin`, `.collattr.bin`, `.collattrb.bin`; `export/section_{N}.{art,coll,tiles}.bin`;
`ojz/chunks_tiles.bin`; `ojz_act1_bg*.bin`; the `ojz_bg_deep-forest-*.bin` version
gallery; and four `.*-backup/` dirs. These are the EDITOR'S BINARY WORKING DATA — the
painted section nametables and collision — and they are the load-bearing missing inputs
(§2).

Also relevant (outside `editor/` but in scope): `data/editor_bg_override.json` TRACKED
(122 KB — the `inject_editor_bg` source); `data/mappings/test_mappings.asm` TRACKED
(`main.asm:246`, HAND-authored test sprite mappings, NOT generated); `data/collision/`
= **0 tracked** (all `.bin`, ignored — `import_sk_collision` + bake outputs).

### 1.3 The generators (the full survey)

**In prebuild (`games/sonic4/prebuild.sh`, run EVERY build via `build.sh:106-107`):**

| generator | reads | emits | tracking of output |
|---|---|---|---|
| `import_sk_collision.py` | `../../skdisasm/Levels/Misc/*` (EXTERNAL repo, abs path) | `data/collision/{,base/}{heightmaps,heightmaps_rot,angles,solidity}.bin` | IGNORED |
| `ojz_strip_gen.py generate` | EXTERNAL `sonic_hack` (layouts/art/chunk+block maps, abs path in `ojz_common.py:23`) + IGNORED editor `section_N.tiles.bin` + `chunks_tiles.bin`; calls `collision_pipeline` | strips/tiles/pool `.bin`, `ojz_act_pool_manifest.asm`, `zone_bg.bin`, `ojz_palette.bin`; **tail-calls `ojz_entity_gen.generate()`** | IGNORED |
| `ojz_entity_gen.py generate` | TRACKED `objects.json` + `section_N.{objects,rings}.json` + `project.json` | `entity_data.asm` | IGNORED |
| ZX0 pack (inline `prebuild.sh` shell + `tools/bin/salvador`) | `act_pool_page{k}.bin` | `act_pool_page{k}.zx0` + `ojz_act_pool.asm` | IGNORED |
| `ojz_block_gen.py generate` | strips `.bin` | `sec{N}_blocks.bin`, `sec_block_dicts.asm`, `sec_block_blobs.asm` | IGNORED |
| `inject_editor_bg.py` (conditional on `editor_bg_override.json`) | TRACKED `editor_bg_override.json` | `zone_bg.bin`, `bg_tiles.bin`, `bg_anim.asm`, `bg_anim_banks.bin` | IGNORED |

`ojz_common.py` — shared loader LIBRARY (no outputs; home of the `SONIC_HACK` abs path
and Kosinski/layout loaders imported by strip-gen + collision_pipeline).

**MANUAL authoring tools (NOT in prebuild/build — upstream of the tracked sources):**

| generator | role | output |
|---|---|---|
| `forest_bg_gen.py` | procedural Deep-Forest BG art | editor bg `.bin` / override JSON (hand-run) |
| `gen_multi_band_bg.py` | Plane-B nametable helper LIBRARY | `.bin` layouts (hand-run/imported) |
| `png_to_bg_override.py` | PNG → `editor_bg_override.json` | the override JSON |

### 1.4 Census headline

**6 generated `.asm` + 43 `.bin` + 12 `.zx0` (61 files, 0 tracked, all gitignored);
27 tracked editor sources; 6 prebuild generation steps (5 python + 1 inline-shell) +
1 shared lib + 3 manual authoring tools = 10 generators in the brief's named set.** No
`.emp` exists yet for any generated data head (only `act_descriptor.emp`, the code-flip
twin). `verify_emit_bin.py` is SOUND-ONLY — it covers zero of the level tree (§6).

---

## §2 — THE REPRODUCIBILITY SETTLEMENT (Q2 — row 178)

### 2.1 WHY a fresh worktree builds ~131 KB different — the exact mechanism

`git worktree add` checks out only TRACKED files, so a fresh worktree lacks every
gitignored editor binary. The divergence is **silent, not a build failure**, and its
gate is one predicate:

```
ojz_strip_gen.py:439  editor_data_available():
    sec0 = editor/ojz/act1/section_0.tiles.bin   # GITIGNORED
    return os.path.isfile(sec0) and os.path.isfile(CHUNKS_TILES_PATH)  # chunks_tiles.bin GITIGNORED
```

In a fresh worktree BOTH probes are absent → `editor_data_available()` returns **False**
→ `enumerate_collision_layouts()` (`:172-190`) takes the **FALLBACK branch**: it globs
ALL `OJZ_1_sec*.bin` from the external `sonic_hack` layout dir and bakes them with the
LEGACY priority-bit collision placeholder (the "no-sonic_hack-collision-sources" path,
`write_strips_to_file` docstring). The result is a DIFFERENT, LARGER level bake with a
different section set and air-fallback collision — the ~131 KB divergence, emitted with
NO error. `tools/seed-worktree.sh` exists precisely to paper over this: it `cp`s the
whole gitignored artifact set from a known-good tree before building (its own header
documents the "silently falls back … builds a ROM that diverges ~130 KB with no error"
failure mode).

### 2.2 The deeper hole: TWO external repos at hard-coded absolute paths

Even with the editor binaries present, the generated tree is NOT determined by anything
in the aeon repo:
- `ojz_common.py:23` `SONIC_HACK = "/home/volence/sonic_hacks/sonic_hack"` — layouts,
  Kosinski art, `mappings/128x128/OJZ.bin`, `mappings/16x16/OJZ.bin`.
- `import_sk_collision.py` reads `../../skdisasm/Levels/Misc/*`.

Neither repo is in aeon; both are addressed by absolute/relative sibling paths. On any
machine without those exact checkouts, the generators cannot run at all. **So the tracked
sources do NOT determine the generated tree today — the generated tree is a function of
(gitignored editor binaries) ∘ (two out-of-repo donor projects).** This is strictly
worse than the sound generators, whose external donors (S3K/smps) feed generators whose
OUTPUTS are committed `.bin` (`.gitignore:29-57` un-ignores them) so the build never
needs the donor.

### 2.3 The ruling: mirror the SOUND precedent — TRACK the outputs, generators go MANUAL

The convert-vs-embed test's "generated-deterministic → emission the build reproduces"
clause assumes the generator's INPUTS are in-repo. Here they are not, so the reproducible
end state is the SAME one the sound stack reached (seam-2 close packet; `.gitignore:25-57`;
`sfx_transcode` "moved from prebuild-auto to MANUAL"): **commit the generated artifacts;
move the level generators OUT of prebuild to MANUAL re-bake tools.** Concretely:

1. **Un-ignore and commit the generated tree** (the `.bin`/`.zx0` bulk + the mechanical
   `.asm`, or their `.emp`/`.bin` successors per §3). The build then CONSUMES tracked
   artifacts; a fresh checkout builds byte-identical because everything it needs is
   committed. This is the acceptance bar: **a fresh `git worktree add` builds
   `e5765873`/`dab4f06c` with NO seeding step.**
2. **Move `ojz_strip_gen` / `ojz_block_gen` / `ojz_entity_gen` / `inject_editor_bg` /
   `import_sk_collision` + the inline ZX0 shell out of `prebuild.sh`** into a MANUAL
   `regenerate-level.sh` a developer runs when the editor data or donor changes — the
   exact `sfx_transcode` MANUAL move. `prebuild.sh` shrinks to nothing level-related (or
   just the ZX0-pack-of-tracked-pages if pages stay `.bin`, but recommend the `.zx0` are
   tracked too, so prebuild does nothing here).
3. **Kill the silent fallback.** `editor_data_available()`'s False branch must become a
   HARD ERROR in the MANUAL regenerate tool ("editor tiles absent — cannot re-bake") so
   a re-bake never silently produces the legacy air-collision tree. Post-conversion the
   BUILD never regenerates, so the build path cannot hit the fallback at all.
4. **A drift verifier** (the level analogue of `verify_emit_bin.py`): re-derive each
   committed mechanical head from its `.bin` twins (and/or re-bake into a temp dir and
   diff) so a stale committed artifact fails the build LOUDLY. Optional but recommended —
   it is what makes "fails loudly" real for the tracked artifacts.
5. **`seed-worktree.sh` becomes unnecessary** once (1) lands — retire it, or keep it as
   an error-checked relic that ASSERTS the tracked tree is complete rather than copying
   from a peer tree.

**Does the editor export tree fully determine the generated tree today? NO** (§2.1/2.2).
What must become tracked to close the gap, in priority order: the generated OUTPUTS
themselves (sufficient for build reproducibility — the sound model); and, to make the
RE-BAKE reproducible too, the editor `section_N.tiles.bin`/`.coll.bin`/`collattr*.bin`
+ `chunks_tiles.bin`, plus a vendored snapshot (or pinned commit ref) of the sonic_hack
layouts/art and skdisasm collision the generators consume. Tracking the outputs is the
REQUIRED minimum for the acceptance gate; vendoring the re-bake inputs is the STRONGER
form and is flagged as OQ-2 (how far to go).

---

## §3 — THE EMISSION FORM PER OUTPUT (Q3, argued by the reader test)

Reader test (ratified): human-authored SEMANTIC → typed `.emp`; deterministic-generated
mechanical → emission the build reproduces (generator-emits-`.emp`, precedent
`gen_sound_tables.py`/`zyrinx_player.py`); opaque BULK → `.bin` `embed()`/BINCLUDE.

| output | class | ruling |
|---|---|---|
| `entity_data.asm` | **SEMANTIC** — `OJZ_Sec{N}_Objects` `objentry` lines, `dc.w X,Y` rings, `dc.l ObjDefLabel` type tables; readable placement intent, true source = tracked editor JSONs | **generator-emits-`.emp`** — `ojz_entity_gen.py` emits `entity_data.emp` (the clearest generator-emits-`.emp` candidate: it carries LINK labels a flat blob can't hold — `dc.l ObjDefLabel` — exactly the `sound_tables_z80` argument, seam-2 §1b). `.emp` `table`/`data` rows single-source it; the `.asm` twin dies. |
| `ojz_act_pool_manifest.asm` | **SEMANTIC** small consts (`OJZ_ACT_POOL_PAGES` + per-page slot/tiles) | **`.emp` `const`s** — but `prebuild.sh` PARSES `OJZ_ACT_POOL_PAGES` via `sed`; that coupling moves to the manual re-bake tool (which knows the page count directly). |
| `sec_block_dicts.asm` | **SEMANTIC** small consts (`OJZ_SEC{N}_BLOCK_DICT_LEN`) | **`.emp` `const`s** (fold into the act-descriptor `.emp` neighborhood or a tiny generated-consts `.emp`). |
| `ojz_act_pool.asm` | **MECHANICAL** — a `BINCLUDE`-list + `dc.l` page-address table | **`.emp` `embed()` of the `.zx0` pages + a `data` pointer array** (or: keep as a trivially-regenerated manifest, since it is pure glue over tracked `.zx0`). Reader test says the pointer table is the semantic part → `.emp` `data`; the pages are BULK → `embed()`. |
| `sec_block_blobs.asm` | **MECHANICAL** — per-section `BINCLUDE sec{N}_blocks.bin` | **`.emp` `embed()` list** (the `sec{N}_blocks.bin` are the opaque S4LZ blobs). |
| `bg_anim.asm` | **SEMANTIC table + BULK payload** — `BgAnim_Table` band records (44-byte LOCKSTEP layout, `inject_editor_bg.py` header) + `BINCLUDE bg_anim_banks.bin` | **`.emp` `data`/`table` for the band records + `embed(bg_anim_banks.bin)`** — the dac_samples two-class exemplar. NOTE the record layout is a THREE-WAY lockstep (`inject_editor_bg.py` ↔ `engine/level/bg_anim.emp` `bganim_band` ↔ `engine/level/bg_anim.asm`); a `.emp` data form must match that struct. |
| `*.bin` / `*.zx0` (strips, tiles, blocks, pool pages, zone_bg, bg_tiles, banks, palette) | **OPAQUE BULK** (compressed/packed byte streams, nametable arrays) | **tracked `.bin`/`.zx0`, `embed()`/BINCLUDE** (BINCLUDE-in-phase not relevant here — these are plain `$10000+` object/level bank data, no `phase` window). |
| `test_mappings.asm` (not generated, tracked, `main.asm:246`) | **SEMANTIC** hand table | **typed `.emp` data**, opportunistic (like parallax data, spec5 §3.2-H) — NOT gitignored, not part of the reproducibility problem; port at leisure. |
| `act_descriptor.asm` (levels/, tracked, has `.emp` twin) | **SEMANTIC** act descriptor CODE | **RIDES STAGE 2** as a row-5 code-flip twin — see §5; NOT converted in this parcel. |

**Where the emitted files live:** the RECOMMENDATION is **tracked, like the sound
outputs** — the `.emp` heads committed, the `.bin`/`.zx0` committed and `embed()`'d/
BINCLUDE'd. This is what buys Q2's fresh-checkout reproducibility; a "regenerated-per-
build" form does not, because the inputs are out-of-repo. The generator-emits-`.emp`
files are re-emitted only by the MANUAL re-bake, then committed (the `gen_sound_tables`
model: tracked output, manual regen, drift-gated).

**Borderline argued both ways:**
- `entity_data` — generator-emits-`.emp` vs a `.emp` that reads the JSONs directly.
  *For direct-`.emp`:* the JSONs are tracked+semantic, so a `.emp` `include`ing/parsing
  them would be single-source with no generator. *For generator-emits-`.emp`:*
  `ojz_entity_gen` does non-trivial work (X-sort, per-section type-table minimization,
  bitmask-capacity validation) that is NOT `.emp` data-DSL expressible today, and the
  sound precedent (`gen_sound_tables`) is exactly "generator does the math, emits
  `.emp`". **RULING: generator-emits-`.emp`** — the sort/minimize/validate logic stays in
  Python; the emitted `.emp` is the committed, diffable, link-checked form.
- `ojz_act_pool.asm` — `.emp` conversion vs keep-as-mechanical-manifest. *For keep:* it
  is pure glue that the ZX0 step already emits; converting buys little. *For `.emp`:*
  once the `.zx0` are tracked, an `.emp` `embed()`+`data` form removes the last inline-
  shell heredoc from `prebuild.sh` and single-sources the page table. **RULING: `.emp`
  `embed()`+`data`** (kills the inline-shell emitter — a real simplification), but this
  is the LOWEST-priority conversion and may defer.

---

## §4 — IDENTITY BARS PER CONVERSION (Q4)

Dual-proof discipline (seam-1/seam-2 precedent), applied per `.asm` retire, and the
parcel-level acceptance gate:

1. **Twin-present region proof (BEFORE any deletion).** For each generated `.asm` being
   converted: with BOTH the `.asm` and its `.emp`/`.bin` successor present, prove the
   ASSEMBLED BYTES agree — the level-data slice from the asl-`.asm` path == the slice
   from the sigil `.emp`/`embed()` path == the current committed reference bytes. This is
   a NEW region gate per file (the seam-2 `*_port` region-gate pattern). For the pure
   `.bin`/`.zx0` bulk that only gets TRACKED (no `.asm` change), the proof is trivial:
   the committed `.bin` == the freshly-baked `.bin` (a `cmp`).

2. **Whole-ROM byte gate, both shapes.** After each conversion: `./build.sh` and
   `DEBUG=1 ./build.sh` both reproduce **`e5765873` / `dab4f06c`** (the PRIMARY assembled-
   ROM CRCs — the brief's hard bar). Because the generated tree is residual-AS in BOTH the
   asl build and the sigil build, this must hold through the whole parcel (the asl dual
   build stays live — pre-Stage-2 rule).

3. **Determinism probe.** Run the MANUAL re-bake TWICE into two temp dirs → byte-identical
   outputs (guards against any nondeterministic dict-order/hash-seed in `ojz_block_gen`'s
   `hashlib` dedup or dict-sweep). Then re-bake once and diff against the COMMITTED tree
   → identical (proves the committed artifacts are what the current generators produce).

4. **THE FRESH-WORKTREE ACCEPTANCE GATE (the parcel's headline bar, from Q2).** In a
   worktree created by `git worktree add` with **NO `seed-worktree.sh`**, `./build.sh`
   and `DEBUG=1 ./build.sh` build `e5765873` / `dab4f06c`. If any required input is
   missing, the build FAILS LOUDLY (no silent `editor_data_available()` fallback). This
   gate is the definition of "done" for the reproducibility half of the parcel.

5. **t24 non-vacuity carry-over.** Any new region/whole-ROM gate gets a positive control
   (undoctored == reference) AND a negative probe (doctor a placement → assert divergence)
   so the golden gate cannot go vacuous — the standing t24 discipline.

---

## §5 — THE RETIREMENT SET + STAGE ASSIGNMENT (Q5)

The generated `.asm` are residual AS consumed by BOTH `act_descriptor.asm` and (AS-side)
`act_descriptor.emp` — they do **NOT block the Spec-5 code flip** (spec5 §3.2-G). So the
question is which die in THIS parcel vs ride Stage 2.

**Die HERE (this parcel), once single-sourced + dual-proven:**
- `entity_data.asm` → `entity_data.emp` (generator-emits-`.emp`).
- `ojz_act_pool_manifest.asm`, `sec_block_dicts.asm` → `.emp` consts.
- `sec_block_blobs.asm`, `ojz_act_pool.asm` → `.emp` `embed()` (act_pool lowest priority).
- `bg_anim.asm` → `.emp` data + `embed()`.
- The `.bin`/`.zx0` bulk: not "retired" — TRACKED + `embed()`/BINCLUDE'd (their identity
  is the reproducibility fix, not a deletion).

**RIDES STAGE 2 (argued):** `data/levels/ojz/act1/act_descriptor.asm`. It is a **row-5
code-flip twin** with an existing `act_descriptor.emp`, and it is the HUB that `include`s
the generated heads. *Argument to convert it here:* it is "level data," topically this
parcel. *Argument to defer (WINS):* (a) it is the ACT DESCRIPTOR code/structure, a row-5
gate-collapse twin whose `.emp` becomes sole source only when its `SIGIL_EMP_*` gate flips
at Stage 2 — deleting it here breaks the asl build (the `.emp` isn't the sole source
pre-flip); (b) `act_descriptor.emp` currently REACHES the generated tree via AS-side
`extern`/labels (`:67-68`), so it depends on the generated heads' emission form — this
parcel FEEDS it (making the heads native gives the `.emp` real symbols to resolve instead
of AS externs) but does not delete it. **RULING: `act_descriptor.asm` rides Stage 2; this
parcel makes its generated dependencies native so the Stage-2 flip is cleaner.**

**Kill-list row updates (same-commit at execution):**
- **The reproducibility item is NOT currently a twin-scaffolding kill-list row** — that
  list tops out at **row 92**; "row 178" (brief/seam-2 §2b) is an informal gap-ledger/
  campaign reference, and there is NO level/OJZ row in the twin-scaffolding list today.
  **ADD a new kill-list row: "the gitignored level tree / fresh-worktree non-determinism,"
  kill condition = the fresh-worktree acceptance gate (§4.4) passes with `seed-worktree.sh`
  retired.**
- **ADD per-conversion rows** for each generated `.asm`→`.emp`/`.bin` twin (entity_data,
  manifest, dicts, block_blobs, act_pool, bg_anim), kill condition = "the generated `.asm`
  goes single-sourced (`.emp`/tracked-`.bin`), both shapes reproduce the reference."
- **Reconcile spec5 §3.2-G**: its "residual AS consumed by `sigil-frontend-as`" note for
  the generated tree updates to "converted in the level-gen parcel" for whatever lands.
- The seam-2 close-packet "flip input #5" line updates to reflect what this parcel closed.

---

## §6 — THE FLIP INTERFACE (Q6)

**What Stage 1/Stage 2 inherit — residual-AS after this parcel:**
- `data/levels/ojz/act1/act_descriptor.asm` (rides Stage 2, §5).
- `data/mappings/test_mappings.asm` (hand data, opportunistic `.emp`, spec5 §3.2-H).
- Any generated head this parcel's honest-budget valve leaves unconverted (e.g. if
  `ojz_act_pool.asm` defers) stays residual-AS consumed by `sigil-frontend-as` — it does
  NOT block the flip (the whole point of §3.2-G).
- The tracked `.bin`/`.zx0` bulk is BINCLUDE'd by both shapes — permanent residual data,
  never "ported," only tracked; it retires (as AS BINCLUDE) only at full D5 when the last
  AS reader (`act_descriptor.asm`) flips.

**`verify_emit_bin.py`'s retirement condition (spec5 §1.5):** confirmed **SOUND-ONLY**.
Its module docstring + target list (`tools/verify_emit_bin.py:5-57`) cover only
`song_*`/`sfx_NN{,_patches}`/`hcz2_patches` `.asm`↔`.bin` twins; it touches ZERO of the
level tree. Therefore:
- This parcel does NOT extend `verify_emit_bin.py`. If a level drift-check is wanted
  (§2.3 step 4), it is a NEW verifier (a level analogue), not an addition here.
- `verify_emit_bin.py` retires exactly per spec5 §1.5: when the generated SOUND `.asm`/
  `.bin` twins it checks go single-sourced (the Stage-2 flip deletes the last sound `.asm`
  twins), it retires with its `.asm` half. The level parcel neither advances nor blocks
  that condition.

---

## OPEN QUESTIONS (for the gate)

- **OQ-1 (conversion vs track-only scope).** The reproducibility fix (Q2 — track outputs,
  generators MANUAL) is INDEPENDENT of the emission-form conversions (Q3 — `.asm`→`.emp`).
  Track-only closes the fresh-worktree gate with ZERO `.asm` deletions and is the minimal
  parcel; the `.emp` conversions are the seam-2-style upgrade. Should this parcel do BOTH
  (recommended: track-only FIRST as its own commit — it is the load-bearing fix and is
  byte-neutral — THEN the `.emp` conversions incrementally), or land track-only and defer
  all conversions to the post-flip arc? Recommend: track-only first, then `entity_data`
  (the clean generator-emits-`.emp` win), then the rest as budget allows.

- **OQ-2 (how far to vendor the re-bake inputs).** Tracking the OUTPUTS is sufficient for
  the build's fresh-worktree gate (§2.3). Making the RE-BAKE reproducible additionally
  needs the gitignored editor binaries tracked AND a vendored/pinned snapshot of the
  external `sonic_hack` + `skdisasm` donor data (currently hard-coded abs paths). Full
  vendoring is a large tracked-binary add. Ruling needed: (a) track outputs only (re-bake
  stays a developer-machine concern), (b) also track the editor binaries, (c) also vendor
  the donor snapshots. Recommend (b) as the pragmatic middle — the editor binaries are
  small and in-repo-adjacent; leave the donor projects as a documented developer
  prerequisite for re-baking, since re-bakes are rare and manual.

- **OQ-3 (`act_descriptor.asm` timing — confirm the §5 ruling).** This design rides it to
  Stage 2 as a row-5 twin and has THIS parcel make its generated dependencies native. If
  the gate instead wants `act_descriptor` fully `.emp`-owned earlier, that pulls a row-5
  gate-collapse forward out of Stage 2 — a flip-sequencing change, flagged for an explicit
  call.

- **OQ-4 (drift verifier — build it now or defer).** The level analogue of
  `verify_emit_bin.py` (§2.3 step 4) makes "fails loudly" real for stale committed
  artifacts. Worth building in this parcel, or is the fresh-worktree whole-ROM gate (§4.4)
  sufficient protection? Recommend building it — cheap, and it is what catches a
  hand-edited committed head that the whole-ROM gate would only catch if the ROM moved.

- **OQ-5 (the inline-shell emitter in `prebuild.sh`).** `ojz_act_pool.asm` is emitted by a
  heredoc in `prebuild.sh`, not a python generator. Converting it to `.emp` `embed()`
  (§3) removes that shell — but if `prebuild.sh` is emptied of level work anyway (Q2),
  the heredoc simply moves to the manual re-bake tool. Confirm whether the `.emp`
  conversion of `ojz_act_pool` is worth doing vs leaving it as tracked mechanical glue.

- **OQ-6 (the two `entity_data.asm` / `act_descriptor.asm` copies).** There are editor
  EXPORT copies (`editor/ojz/act1/export/{entity_data,act_descriptor}.asm`, TRACKED)
  distinct from the live `generated/…/entity_data.asm` and `levels/…/act_descriptor.asm`.
  This design treats the export copies as editor artifacts (not build inputs — `main.asm`
  includes the generated/levels copies, not the export ones). Confirm the export copies
  are inert w.r.t. the build (a grep found no `main.asm`/`engine.inc` include of them) and
  are not a second source of truth to reconcile.
```
