# 2026-07-30 — THE SPEC-5 FLIP design (DESIGN GATE ONLY — the terminal event's plan)

Status: **DESIGN NOTE — answers the brief's five question sets. No implementation,
no twin deletion, no build change, no aeon modification.** Sigil branch
`spec5-flip-design`, worktree `.worktrees/spec5-flip-design`. Masters: sigil
`77e7ceb` (= the brief's commit) / aeon `409b8ba`.

**Baseline VERIFIED read-only** (no aeon build triggered — existing artifacts CRC'd
in place; aeon tree clean at `409b8ba`, NOTHING modified): `s4.bin` =
**eff2396f / 413577**, `s4.debug.bin` = **1e9097bc / 421579** (both match the brief).
Strict suite (`AEON_DIR=…/aeon cargo test --release`, `emit_sound_blob` built first) =
**2904 passed / 0 failed / 1 ignored** — matches the brief exactly.

Cites: `.asm`/`.emp`/`.inc` are aeon paths at `409b8ba`; `.rs`/ledger/kill-list are
sigil at `77e7ceb`; kill rows are numbered from `twin-scaffolding-kill-list.md`.

---

## §0 — THE LOAD-BEARING FACT THE WHOLE FLIP TURNS ON (two "AS"es, not one)

There are **two distinct things called "asl"** and the flip retires them at
different times. Naming them apart is the prerequisite for every answer below.

1. **`tools/asl`** — the EXTERNAL Macro Assembler binary `build.sh` shells out to
   (`build.sh:~135` `"${TOOLS}/asl" ${ASFLAGS}`). It produces the REAL shipped
   `s4.bin`. This is what "asl retires" means in the everyday sense: `build.sh`
   stops calling it.
2. **`sigil-frontend-as`** — sigil's OWN in-tree reimplementation of the AS
   frontend, byte-identical to `tools/asl` (M1 complete — MEMORY: "full `s4.bin`
   byte-identical to `asl`"). `sigil build --aeon <dir>` ALREADY assembles the whole
   `main.asm` include tree through it (`sigil-cli/src/main.rs:16-18`,
   `sigil_harness::assemble_full_rom` `lib.rs:60-96`). Per `SIGIL_CORE_SPEC.md:63`
   (D5) + `:487`/`:536` (§7.4/§9.1), `sigil-frontend-as` is a **single deletable
   crate with a one-way dependency** — nothing but `sigil-cli`/`sigil-harness`
   depends on it — so its deletion is the D5 CAPSTONE, separable from everything
   else.

The mixed-link machinery that has run the whole campaign already IS the flip in
miniature: set a `SIGIL_EMP_<X>` define → the AS side's `ifndef` arm takes the
`else org <resume>` branch (skips the region) → sigil places the lowered `.emp`
module at the pinned LMA → the two combine byte-exactly (`assemble_mixed_*_as_side`
+ the `mixed_dac_rom` tranches, `lib.rs:98-260`). **The flip = flip EVERY gate ON at
once, drive the whole build through sigil, delete the `.asm` code twins.** No new
build mechanism is invented; the existing one is turned all the way up.

**Consequence for "asl retires":** `tools/asl` leaves the build the moment `build.sh`
calls `sigil build` instead. `sigil-frontend-as` SURVIVES the flip — it is what lets
sigil keep consuming the residual pure-AS DATA (config, generated level tree,
parallax data, the demo game's game-side files) that nobody has ported. Deleting
`sigil-frontend-as` (full D5) is a strictly LATER capstone once that residual is gone.
Conflating the two is the single biggest trap in planning this event.

---

## §1 — THE MANIFEST

### 1.1 What the manifest IS today

The build description is `games/sonic4/main.asm` + its four config includes
(`main.asm:10-14,16-18`: `config/constants.asm`, `config/sound_ids.asm`,
`config/game.asm`, `config/ram.asm`). `main.asm` defines seven `game*Includes`
macros (`SIGIL_AEON_COMPAT_NOTES.md:53`) whose bodies are the ordered include list;
`engine/engine.inc` owns the ROM layout (`org $10000` object bank `engine.inc:653`,
the sound bank window, `EndOfRom`). Every ported module sits behind an
`ifndef SIGIL_EMP_<X> / include <X>.asm / else / org <resume> / endif` arm — 30+ of
them across `main.asm` and `engine.inc` (`engine.inc:80-687`, `main.asm:21-465`).
The banked sound head is bracketed by `save / cpu z80 / phase 08000h / soundBankHead …
/ dephase / restore` (`main.asm:363-368`, macro at `engine/sound/sound_bank.inc:33`).

### 1.2 The form — GROW `sigil.map.toml` + a build entrypoint, NOT a new `.emp`
manifest

**Argue from what exists.** `sigil build --aeon . -o s4.bin` already does the whole
job in three existing pieces (`run_build`, `sigil-cli/src/main.rs`): `assemble_full_rom`
→ `sigil_link::emit_rom(&img, &map)` reading **`sigil.map.toml`** → write bytes. The
map file already declares the ROM region + the Z80 Moving-Trucks bank window
(`sigil.map.toml:6-19`). `SIGIL_SPEC2_LANGUAGE.md:199` states the intended end-state
directly: *"Section placement order is an explicit ordering manifest in the map file
— the current main.asm ordering constraints ('sfx blobs before sfx_table') become
named orderings stated once, and `__BUDGET_*` accounting markers become per-section
size reports for free."*

So the manifest form is: **`sigil.map.toml` GROWS into the placement manifest** (the
region LMAs + the section ordering the `main.asm`/`engine.inc` gate-resume `org`s
encode today), and a **build entrypoint (`sigil build`, already present) replaces
`build.sh`'s asl invocation**. A brand-new `.emp` manifest language is NOT argued for
— the map file is the established placement surface, the CLI already consumes it, and
Spec-2's own design names it as the home. `main.asm`'s role shrinks to a thin
per-game module/section LIST that the driver reads (which `.emp` modules this game
links, in what order); the ordering CONSTRAINTS move to the map.

### 1.3 Include / order / org semantics sigil must own

The gate-resume `org`s are not free-floating — each encodes a **section END** the
next placement resumes at. Sigil must own these as **section ordering + size
accounting**, not literal addresses:

- **Object-bank ordering** (`main.asm:61-98`, player→test objects): a linear run
  inside `org $10000 … if * > $20000 / error` (`engine.inc:653-662`). Sigil owns it
  as an ordered section list bounded by the 64 KB budget assert (which becomes a
  per-section size report, `SPEC2:199`).
- **Per-shape resume `org`s** (kill rows 6/58): e.g. `main.asm:445/447` object_test_state
  `$5E2DA`/`$5C7EC`, `engine.inc:625/627` sound_api `$80E4`/`$6414`. These are LINK
  OUTPUTS in a native build — the placement of one section IS the resume point of the
  next. Sigil owns them by computing them, not by pinning literals; the whole
  kill-row-6 re-pin tax DISAPPEARS (row 6 kill condition: "the pins exist only while
  the dual build exists").
- **The `phase 08000h` LMA≠VMA head** (`main.asm:363-368`, `sound_bank.inc`): the
  banked window (VMA `$8000`, physically in the `$58000` bank) is already expressible
  natively — `sigil.map.toml`'s `z80_moving_trucks_bank` region carries
  `vma_base = 0x8000` / `lma_base = 0x60000` (`sigil.map.toml:14-19`). The `save/cpu
  z80/phase/dephase/restore` bracket is an AS spelling of exactly this; sigil owns the
  window placement through the map region, and the `.emp` sound modules already lower
  `cpu: z80` with phased placement (z80_init.emp precedent, seam-1/seam-2 heads).

### 1.4 The gate-collapse mechanics

Every `ifndef SIGIL_EMP_<X> / include <X>.asm / else / org <resume> / endif`
resolves to **just the native `.emp` placement**:

- Each `SIGIL_EMP_<X>` arm collapses to its `.emp` module link (the `ifndef`/`else`
  vanishes; the `.asm` include line is deleted with the twin).
- **The `else`-`org` resume arms DIE** — the resume point is a link output (§1.3).
- **The `phase 08000h` bracket + `soundBankHead` macro DIE** — the banked head is a
  native `cpu: z80` phased placement via the map region; `soundBankHead`
  (`sound_bank.inc`) and its five BINCLUDEs collapse into the sound modules' own
  placements. (Precondition: `movingtrucks_pitchtable.asm`, the LAST AS member of
  that bracket, ports first — §3, flip input #3.)
- **The `SIGIL_EMP_*_BODY_STUB` fork DIES** (kill row 91): the DSM in-memory
  composition vs BINCLUDE duality (`main.asm:312-334/369-425`) only exists because
  the real build was AS-with-placed-.emp. Native, the composed path IS the build.
- **The numeric `ErrorHandler` / `Game_Entry` equs DIE or resolve natively** (kill
  rows 52/90, `engine.inc:704`, `config/game.asm`): today they are numeric per-shape
  equs because sigil "does not resolve an equ off a link-EXTERNAL base" (row 52). At
  the flip these become the `.emp`-owned labels directly — either the
  link-time-equ-off-external-base capability lands (row 52's kill) or the consumer
  reads the `.emp` symbol by bare link (no external base to fold against once
  everything is one link).

### 1.5 What replaces `build.sh`'s asl + p2bin + fixheader pipeline

Today (`build.sh`): `emit_sound_blob` (sigil, already a HARD dependency — no asl
fallback, `build.sh:~80-100`) → `tools/asl` (assemble) → `tools/p2bin` (`.p`→`.bin`)
→ `tools/convsym` (symbols) → `tools/fixheader` (checksum). Post-flip:

- **asl → `sigil build`** (`assemble_full_rom` over all frontends: `.emp` modules
  lowered natively + residual `.asm` DATA via `sigil-frontend-as`).
- **p2bin → `sigil_link::emit_rom(&img, &map)`** (already the `-o` path in
  `run_build`; the map IS the p2bin layout, `sigil.map.toml:1`).
- **convsym → sigil's own symbol table** (kill row 34: "sigil's own symbol table
  becomes address ground truth"). `SIGIL_CORE_SPEC.md:62` (D3) requires sigil keep
  emitting an AS-`.lst`-compatible listing so `convsym`/`s4budget.py`/oracle
  `load_symbols` keep working — so the LISTING format survives, its asl SOURCE does
  not.
- **fixheader → keep the tool OR fold the checksum into `sigil build`.** The Genesis
  header checksum is a trivial 16-bit sum; recommend folding it into `emit_rom` so
  the pipeline is one command, but keeping `tools/fixheader` is a valid stopgap (it
  is not an assembler — it does not block asl retirement). Rule this an execution
  detail.
- `verify_emit_bin.py` (`build.sh:~120`) — its whole job is checking the generated
  sound `.asm`/`.bin` twins agree (an asl-era coupling); when the generated data goes
  single-sourced (§3, generator parcel) it retires with its `.asm` half.

---

## §2 — THE ORACLE-MODEL SHIFT (the hard question — THIS SECTION GOES VERBATIM TO VOLENCE)

**Read this section as the honest accounting of what the flip costs in verification
power. It is the one part of the plan that trades a real protection away, and you
should decide it with eyes open.**

### 2.1 What the ~60 test files actually prove today

Today every sigil gate — windowed region, whole-ROM, DSM tranche — compares sigil's
output against **the asl-built reference ROM** (`s4.bin`/`s4.debug.bin`, frozen at
eff2396f/1e9097bc). asl is an **independent implementation of the same assembly
semantics, written by different people over decades, with different bugs than
sigil's.** That independence is the deep value: when sigil's `.emp` lowering (or
sigil's own AS frontend) has a bug, asl DOESN'T share it, so the byte comparison
catches it. asl is a **live oracle** — it re-assembles ARBITRARY new/changed source
on demand and tells you the truth about it, not just about a frozen snapshot.

### 2.2 What is genuinely LOST at the flip — stated plainly

When asl leaves the build AND the `.asm` code twins delete, **there is no longer a
second, independent implementation of the whole assemble-and-link pipeline.** The
concrete loss:

- **For UNCHANGED bytes:** nothing is lost. The frozen reference ROM (eff2396f /
  1e9097bc) becomes a tracked golden; sigil must still reproduce it exactly. Full
  regression protection survives.
- **For NEW or CHANGED code after the flip** (optimizations, new objects, act 2, any
  post-conversion step-5 work): the independent witness is GONE. Today, if you change
  engine code, asl re-assembles it and sigil must match asl — asl vouches for the new
  bytes. Post-flip, new bytes have **no independent second opinion**: sigil is
  checked against sigil's own belief. A bug that sigil's single implementation makes
  consistently (lowering AND encoding it wrong the same way) produces "correct-looking"
  bytes that no gate can flag, because the only thing that could disagree — asl — is
  gone. The byte gate degrades from "sigil == an independent truth" to "sigil ==
  itself" for everything the golden doesn't already cover.

This is the real cost: **the flip converts the byte gate from a correctness oracle
(for new code) into a regression oracle (for frozen code).** New code loses its
independent witness.

### 2.3 What replaces it — the three residual protections, honestly rated

1. **Frozen reference ROMs as tracked goldens** — STRONG for regression, ZERO for
   new code. Covers exactly the bytes frozen at flip time. Mitigation: **freeze the
   MAXIMAL golden set at flip** — both shapes AND a few off-canonical configs (the
   `mixed_offcanonical_rom` Config-A/B ROMs, kill rows 55/58) — so the regression
   surface is as wide as possible.
2. **The committed ISA golden-vector corpus** (`README.md:63-74`, "the asl-oracle
   discipline") — THE ONE PIECE OF INDEPENDENT-asl WITNESS THAT SURVIVES. Per-CPU
   snippet corpora were assembled by real asl into committed golden vectors; CI
   byte-checks every encoding against them and NEVER needs asl. This is an
   independent asl witness for the INSTRUCTION ENCODINGS (mnemonic/operand-shape →
   bytes), frozen as vectors rather than a live tool. It does NOT cover whole-program
   LAYOUT/link, but it means a mis-encoded instruction in new code IS still caught
   against asl-derived truth. **This is the residual independence Volence should lean
   on hardest for new code, and it argues for keeping the vector corpus rich (add
   vectors for any new instruction shape a post-flip optimization introduces).**
3. **Emulator A/B (the oracle)** — behavioral, not byte-level. Catches SEMANTIC bugs
   the byte gate is structurally blind to (the whole "byte-gate-blind" class the loop
   already runs oracle A/B for: kill row 35, ledger G9). Post-flip it becomes MORE
   load-bearing, not less, because it is the only check that judges new code by what
   it DOES rather than by bytes sigil vouches for itself.

Two internal-redundancy notes, weaker but real: (a) `sigil-frontend-as` survives the
flip and is an independent CODE PATH from the `.emp` frontend, so residual `.asm` and
`.emp` modules agreeing on shared symbols in one link is a cross-frontend check — but
both share sigil's IR/backend/linker, so it is NOT independent of sigil's encoder
(much weaker than asl). (b) The strict suite's t24 positive-control/negative-probe
discipline (undoctored==golden AND doctored!=golden) keeps every golden gate
NON-VACUOUS — this must be preserved verbatim through the flip.

### 2.4 The recommendation to weigh

**Retiring asl from the BUILD is not the same as deleting the asl binary.** The
cheapest insurance against the §2.2 loss: keep `tools/asl` in the repo (out of the
build path) as a MANUAL re-validation oracle for major new code — assemble a new
module through asl by hand and diff — until confidence in sigil-solo is high. This
costs nothing to keep and preserves the live independent witness for the exact case
(big new code) where it is most valuable. It is orthogonal to the flip and does not
block it.

### 2.5 Per gate-class disposition

| gate class | today's comparand | post-flip |
|---|---|---|
| **windowed region gates** (e.g. `test_p2_player_states_port`, `sound_psg_port`) | reference-ROM slice OR AS-twin re-assembly | the AS-twin-reassembly half **DIES** (no `.asm` to `assemble()`); the region gate SURVIVES transformed → "built region == frozen-golden slice" (the seam-1 precedent, `seam1-design.md §3.2`) |
| **whole-ROM gates** (`mixed_dac_rom`, `seam2_*_rom`) | asl reference ROM | SURVIVE → "sigil-native whole ROM == frozen-golden ROM CRC" (the top provenance gate) |
| **DSM mixed tranches** (`mixed_dac_rom` composing `.emp` in-memory into an AS ROM) | asl ROM with placed `.emp` | **COLLAPSE** — native, the composed path IS the real build, so the ~52 tranches fold into the single whole-ROM golden gate; the BODY_STUB fork (row 91) dies |
| **t24 controls** (positive control + negative probe) | reference window | SURVIVE, re-comparanded to the golden — doctor the `.emp`, assert divergence from golden; undoctored `.emp` == golden. KEEP verbatim (they are what stops the golden gates going vacuous) |

**Strict-count estimate:** the count DROPS. Every windowed oracle's AS-reassembly
half retires (dozens of tests), and the ~52 DSM tranches collapse toward a handful of
whole-ROM golden gates. But per-test COVERAGE rises (each surviving gate is
golden-anchored + t24-controlled). Rough shape: expect the low-2000s to shed several
hundred AS-twin tests, landing meaningfully below 2904 — the exact number is an
execution output, not predictable here. Report it at each stage.

---

## §3 — THE RETIREMENT ENUMERATION (every open kill row + every remaining `.asm`)

**Method:** enumerated from the kill-list ROWS and from `find . -name '*.asm'` in the
aeon main checkout (105 real `.asm`, excluding `tools/` and the stale nested
`.worktrees/sound-perf-budget/` mirror). Each gets a disposition + the flip STAGE
(§4) that executes it.

### 3.1 The open kill-list rows

Already CLOSED (no flip action): rows 3, 13, 16, 29–33, 38, 39, 42 (killed by
port/consolidation); rows 56, 57, 70, 71, 78, 83, 87 (seam-1/seam-2 twin deletions
DONE). Rows 80 (KILLED at P4). These need nothing.

**Rows that the flip executes:**

| row | what | disposition + stage |
|---|---|---|
| **1, 2, 12, 14, 17, 19, 20** | `engine.constants` twin (all blocks) | `engine/constants.asm` ports → **ownership flip**: `engine/system/constants.emp` BECOMES the definition; residual AS readers (data files consumed by `sigil-frontend-as`) take exported equs. **Stage 3** (needs the export-to-residual-AS mechanism). |
| **4** | `ANIM_*` ordinal guards ×12 | stage-2 delete the config block + swap player code to `.emp` exports (`SPEC2 D2.34`); needs the `.b`/`.w` imm-link deferral. **Stage 3** (language dep). |
| **5** | THE 60 AS code twins | each gate collapses → `.asm` twin DELETED. **Stage 2 (point of no return).** Enumerated in §3.2. |
| **6, 58** | per-shape gate pins + inline off-canonical org arms | DIE — resume points become link outputs (§1.3). **Stage 2.** |
| **7, 8, 11, 15, 25** | `Act`/`Sec`/`Sst`/`EntityScanState` struct twins + `interact_off()` | `engine/structs.asm` ports → **ownership flip** (the `.emp` overlays become the definitions). **Stage 3.** |
| **9, 45** | `gameDebugTick` / `gameBootHook` macro-mirrors | need a ratified game-contract-hook mechanism (extern-macro / link-time hook) OR GameLoop migrates game-side. **Stage 3** (language dep) — surfaced as an open question. |
| **10** | sound_api immediate mirrors | 5 untyped retire at the sound_api flip (**Stage 0**); 2 typed (`SFXID_RING_*`) blocked on typed-extern grammar (**Stage 3**/language). |
| **18, 22, 23, 24, 26, 27, 28, 36, 37, 40, 41, 43, 44, 46, 47, 48, 49, 50, 51** | comptime-fn templates / const mirrors / rept-family / `SpawnDesc` / boot cursor | DIE with their `.asm` twins (the AS macro/inline has no reference once the twin is gone). **Stage 2** (with row 5). A few (18/22 game-config mirrors) need a game-constants `.emp` home — **Stage 3.** |
| **21, 53** | diagnostics twin-parity emission + donor-config pin | `debugger.asm` is the SOURCE; when the `.asm` twins die there is no macro-tower to reproduce → the message format + `b<cond>.w` pin are FREED. The POST-TWIN-RETIREMENT `.emp`-native diagnostics runtime owns the format. **Stage 3 (debug-runtime rewrite).** |
| **34** | **asl listings as address ground truth** | THE core flip item: sigil's own symbol table becomes ground truth; repin's `.lst` parsing (`repin.rs:57-58`) + the debug-`.lst` cp/suffix machinery + convsym's asl-listing source are DELETED. **Stage 3.** |
| **35** | OJZ per-frame mode-register force-write | INDEPENDENT of the flip; carried commented into `ojz_scroll_test.emp` (t41 CARRY-AS-IS) OR closed by the parallax-hardening parcel (§5). **Not a flip blocker.** |
| **52, 90** | numeric `ErrorHandler` / `Game_Entry` equs | resolve natively at the flip (link-time-equ-off-external-base OR bare `.emp` symbol). **Stage 2.** |
| **54, 62, 65, 65b** | game-config const surface (buttons / song+sfx ids / `VRAM_TEST_*`) | a game-constants `.emp` module is born (SONG_*/SFXID_*/VRAM_* move there) OR `config/constants.asm`+`config/sound_ids.asm` retire. **Stage 3.** |
| **55, 59** | z80_init / sound_debug gate-off twins | DIE with the twin. **Stage 2.** |
| **60, 61, 63, 64, 66, 67, 68, 69** | G1–G3 test-object twins + overlays + `SpawnDesc`/`vram_bytes`/`DplcV` | gate collapse → `.asm` deleted; overlays flip to `.emp`-owned. **Stage 2** (code); game-config mirrors **Stage 3**. |
| **72, 73, 74, 75, 76, 77** | player keystones (internal-gate) | the CODE half deletes (**Stage 2**); the always-emitted HEADER equates flip to `.emp`-owned `pub const` (**Stage 3** — needs the export-to-residual mechanism). |
| **79, 81, 82** | player state twins + PPHYS mirrors + `abs_w` | code DELETED (**Stage 2**); PPHYS/game-config mirrors to a game-constants `.emp` (**Stage 3**). |
| **84, 85, 86, 88, 89** | test_player / test_enemy / path_swap / object_test_state / ojz_scroll_test twins | gate collapse → `.asm` + per-shape org arms deleted; overlays/consts flip `.emp`-owned. **Stage 2** (code), **Stage 3** (consts). |
| **91** | `SIGIL_EMP_*_BODY_STUB` DSM defines | DIE — the composed path IS the native build; drop the stub arm + the defines. **Stage 0/2.** |
| **92** | vestigial `boot_data.asm` sound scaffolding | drop the `z80_sound_syms.asm` include + the 4 SFX helpers + stop seam-1 emitting the syms; byte-neutral. **Stage 0** (precursor cleanup) or **Stage 3.** |

### 3.2 Every remaining `.asm` — the 105-file walk (bucketed, each with its stage)

**A. The 60 code twins with a `.emp` counterpart (row 5 — DELETE at Stage 2):**
engine/system: `boot`, `boot_data`(partial — keeps the permanent Z80 boot arm),
`buffers`, `controllers`, `dma_queue`, `game_loop`, `hblank`, `math`, `vblank`,
`vdp_init`, `vectors`, `z80_init`. engine/objects: `animate`, `children`, `collision`,
`core`, `dplc`, `entity_window`, `load_object`, `rings`, `sprites`. engine/level:
`bg`, `bg_anim`, `camera`, `collision_lookup`, `load_art`, `parallax`, `plane_buffer`,
`section`, `tile_cache`. engine/compression: `s4lz_decompress`, `zx0_decompress`.
engine/debug: `compression_selftest`, `error_handler`, `sound_debug`. engine/sound:
`sound_api`. engine: `structs` (→ ownership flip, not plain delete — rows 7/11/25).
game player: `player_air`, `player_ground`, `player_sensors`, `player_spindash`,
`sonic`. game objects: `path_swap`, `test_animated`, `test_churn`, `test_emitter`,
`test_parent`, `test_particle`, `test_solid`, `test_static`, `test_stress_emitter`.
game test: `object_test_state`, `ojz_scroll_test`. game debug: `game_debug`. game
data: `data/animations/sonic_anims`, `data/animations/particle_anims`,
`data/objdefs/test_objects`, `data/levels/ojz/act1/act_descriptor`.

**B. The 3 internal-gate keystones (rows 72/84/85 — CODE deletes Stage 2, HEADER
flips Stage 3):** `player/player_common.asm`, `objects/test_player.asm`,
`objects/test_enemy.asm`. Their zero-byte headers feed surviving readers, so the
header equates become `.emp`-owned `pub const` exports at Stage 3.

**C. The 4 config files (rows 54/62/65/9/45/90 — Stage 3):**
`config/constants.asm`, `config/sound_ids.asm`, `config/game.asm`, `config/ram.asm`.
`game.asm` carries the manifest macros + `Game_Entry`/`gameDebugTick`/`gameBootHook`
— its retirement needs the game-contract-hook mechanism. Recommend a game-constants
`.emp` module is born to absorb the mirror truths; the RAM layout (`ram.asm`,
`config/ram.asm`) either ports to `.emp` `vars` or stays residual AS.

**D. Shared truth / macro files:**
- `engine/constants.asm` (row 1 twin) + `engine/structs.asm` (rows 7/11) → **ownership
  flip Stage 3** (the `.emp` becomes the definition; residual AS takes exported equs).
- `engine/macros.asm` — NO `.emp` (its counterparts are comptime fns across modules,
  rows 24/26/40/41/48/63/75). Retires when NO AS invoker of any macro remains — i.e.
  when `sigil-frontend-as` no longer assembles a `.asm` that expands a macro. **Stage
  3/4** (residual-dependent).
- `engine/sound_constants.asm` (row 59 truth), `engine/ram.asm` — flip or residual.

**E. Debug infrastructure (rows 21/52/53 — Stage 3 debug-runtime rewrite):**
`engine/debug/debugger.asm` (the macro tower — zero-emission, the diagnostics
construct's source, POST-TWIN-RETIREMENT home ledgered), `engine/debug/mddbg_symbols.asm`
(derives `MDDBG__` equs off `ErrorHandler`, row 52), `engine/debug/generated/vectors.asm`
(generated — regenerate or residual).

**F. The last AS sound head (flip input #3 — Stage 0 precursor):**
`games/sonic4/data/sound/movingtrucks_pitchtable.asm` (the `SndDefaultPitchTable`, the
LAST surviving member of the `soundBankHead` phase bracket, `main.asm:366`,
`sound_bank.inc:51`). **RULE: it PRECEDES the flip** — a small pitch-table port to
`.emp` (data table), byte-gated vs the golden slice, so the `phase 08000h` bracket can
fully die at Stage 2. It is a natural extension of the seam-2 banked-head work, not
part of the code flip.

**G. Generated level-tree data + the generators (flip input #5 — OWN PRECURSOR
PARCEL, Stage 0-parallel):** `data/generated/ojz/act1/{entity_data, bg_anim,
ojz_act_pool, ojz_act_pool_manifest, sec_block_blobs, sec_block_dicts}.asm`,
`data/levels/ojz/act1/act_descriptor.asm` (has `.emp`), `data/editor/ojz/act1/export/*`,
`data/mappings/test_mappings.asm`. Emitted by `tools/ojz_entity_gen.py` (`--generate`
→ `entity_data.asm`, `ojz_entity_gen.py:10,342`) + the OJZ level pipeline. **RULE: its
OWN precursor parcel** (a DATA seam, the seam-2-for-level-data analogue), NOT bundled
into the code flip: the generators emit `.emp`/`.bin` instead of `.asm` (the
generator-emits-`.emp` realization already proven for sound, seam-2 stage-3), with the
level-tree reproducibility row as its acceptance bar. Until it runs, these files are
**residual AS consumed by `sigil-frontend-as`** — they do NOT block the code flip.

**H. Parallax + sprite DATA (no `.emp` — residual AS, Stage 4/opportunistic):**
`data/parallax/{ojz_default, ojz_windy}.asm`, `data/parallax/effects/{haze,
perspective, rocking, shimmer}.asm`, `data/parallax/scenes/{caves, locked_clouds,
sky_haze, windy_haze}.asm`, `data/sprites/pitcher_plant/anims.asm`. Pure `dc.*` data
tables. Consumed by `sigil-frontend-as` post-flip; port to `.emp` data / a data-DSL
opportunistically. Not blockers.

**I. Generated sound syms (row 92 — Stage 0):** `engine/sound/generated/{mt_syms,
mt_syms_debug, z80_sound_syms}.asm`. `mt_syms` feeds `sound_api.asm`'s
`movea.l #SongTable`; `z80_sound_syms` is vestigial post-seq/sfx. When `sound_api`
flips (Stage 0) these lose their AS consumer → deleted + seam-1 stops emitting them.

**J. The manifests themselves (Stage 2):** `games/sonic4/main.asm` (shrinks to the
per-game module list; the macro/gate scaffolding deletes), `engine/engine.inc` (the
gate arms + org resumes collapse; the layout moves to the map).

**K. THE DEMO GAME — the biggest under-scoped constraint (open question OQ-1):**
`games/demo/{main, demo_state, config/constants, config/game, config/ram,
objects/demo_box, data/demo_data}.asm`. `demo/main.asm:42` `include "engine/engine.inc"`
— **demo builds through asl including the ENGINE `.asm` code twins.** When the row-5
engine twins delete at Stage 2, demo's current build path breaks. Resolution:
**demo must ALSO build through sigil** at Stage 2 (its game-side `.asm` files consumed
by `sigil-frontend-as` as residual, linked against the native `.emp` engine). This is
feasible precisely because `sigil-frontend-as` survives the flip — but it MUST be
designed in lockstep (demo's `build.sh` path flips to `sigil build` at the same
commit, and demo's whole-ROM golden — `demo.bin` — joins the frozen goldens). If demo
is instead PARKED, that is a Volence call. Flagged as OQ-1.

**L. `engine/system/vectors.asm`** — has `vectors.emp` twin (row 5) → **Stage 2**.
`engine/debug/generated/vectors.asm` → generated (regenerate/residual).

### 3.3 sound_api — the deferred parcel: RIDES BEFORE the flip (ruled)

`sound_api.asm` (rows 10/24/36/43) is the 68k sound caller, NOT in any Z80 blob
(`engine.inc:612`, its own `SIGIL_EMP_SOUND_API` gate + org-resume arm already
scaffolded). **RULE: flip it as its own small 68k parcel at Stage 0, BEFORE the main
flip** (`seam1-design.md §3.5` recommends exactly this — "keep the code flip purely
the code link"). Its machinery exists; flipping it early retires row 10's 5 untyped
mirrors + the `mt_syms`/`z80_sound_syms` consumers (§3.2-I) + closes the last banked
sound `.asm` caller cleanly, shrinking the Stage-2 surface.

---

## §4 — SEQUENCING + SAFETY (the staged order, the named point of no return)

The flip CANNOT be one commit. Five stages; each dual-proven where a dual state
exists.

### Stage 0 — PRECURSORS (fully reversible; asl still builds everything)
Independent small parcels, each byte-gated vs the frozen golden, all keeping the dual
build intact:
1. **sound_api flip** (§3.3) — its own 68k parcel; retires rows 10(untyped)/mt_syms/z80_sound_syms consumers.
2. **`movingtrucks_pitchtable.asm` → `.emp`** (§3.2-F) — frees the `phase 08000h` bracket.
3. **Row 91 BODY_STUB collapse** — convert DSM composition to BINCLUDE-only; byte-neutral (both paths already == golden, kill row 91).
4. **Row 92 vestigial cleanup** — drop the dead boot_data sound scaffolding; byte-neutral.
5. **(Parallel, own track) the generator/level-tree parcel** (§3.2-G) — data seam; not on the flip critical path.
- **Proof model:** byte-identity vs golden, both shapes. **Rollback:** trivial `git revert` — nothing deleted that asl needs.

### Stage 1 — STAND UP THE NATIVE WHOLE-ROM BUILD (DUAL; the last asl-witness moment)
- Build the sigil-native whole-ROM driver: flip ALL `SIGIL_EMP_*` ON in a `sigil build`
  path; drive assemble + link + `emit_rom` + checksum through sigil for BOTH games
  (sonic4 + demo). Grow `sigil.map.toml` into the placement manifest (§1.2/1.3).
- Keep the asl build ALSO working (gates OFF). **This is the DUAL state.**
- **Proof model — THE STRONGEST POSSIBLE, and the reason this stage is the gate before
  no-return:** `sigil-native whole ROM == asl whole ROM == frozen golden`, both shapes,
  both games. This is the LAST moment asl is a live independent witness on the full
  program; use it to prove the native build byte-for-byte before deleting anything.
- **Rollback:** trivial — the asl path still exists; the native path is additive.

### Stage 2 — THE FLIP COMMIT === THE POINT OF NO RETURN
**Named explicitly: the commit that switches `build.sh` (both games) to `sigil build`
AND deletes the row-5 `.asm` code twins (+ keystone code halves, sonic_anims/particle_anims,
act_descriptor, test-object twins, main.asm/engine.inc gate scaffolding, the phase
bracket, the BODY_STUB defines, the numeric ErrorHandler/Game_Entry equs).**
- **After this commit, asl (and `sigil-frontend-as`) can no longer build the ROM** —
  the `.asm` code twins are GONE; only the `.emp` exists for those modules. That is the
  no-return line: no live independent assembler can re-derive the code bytes anymore.
- **Proof model — FROZEN GOLDEN:** the sigil-native build == the frozen reference CRCs
  (eff2396f / 1e9097bc + the off-canonical goldens). The independent asl witness is
  gone; the golden is the bar (§2). The DSM tranches collapse into the whole-ROM golden
  gate; the windowed gates re-comparand to golden slices; the t24 controls survive.
- **Rollback:** `git revert` ONLY (the twins live in history) — but the WORKING build
  is now sigil-solo. This is the stage whose plan goes in the MORNING REPORT for
  Volence's eyes before it runs (per the brief §2).

### Stage 3 — CLEANUP (post-flip; normal reversibility)
Row 34 (delete repin `.lst` parsing + cp/suffix machinery — sigil's symbol table is
ground truth); the ownership flips (constants.emp/structs.emp become sole definitions,
export equs to residual AS); the game-constants `.emp` module (rows 54/62/65/76/77/81);
the debug-runtime rewrite (rows 21/52/53 — debugger/mddbg_symbols); rows 4/9/45 as
their language deps land. **Proof:** golden-anchored byte gates + oracle A/B for any
byte-changing ownership flip.

### Stage 4 — FULL D5 CAPSTONE (LATER; optional, not blocking)
Port the residual AS off AS entirely (config, parallax data, generated tree, demo
game-side, macros.asm's last invokers), THEN **delete the `sigil-frontend-as` crate**
(`SIGIL_CORE_SPEC.md:63/536` — the D5 terminal act, structurally a no-op for
IR/backends/link). Until then `sigil-frontend-as` is the residual-data reader. This is
explicitly AFTER the code flip.

### Byte-identity-provable vs frozen-golden-only
- **Stages 0 + 1 are byte-IDENTITY-provable** (asl live → the strongest bar).
- **Stage 2 onward is frozen-GOLDEN** (asl gone → the golden is the only whole-program
  truth; new/changed code leans on the §2.3 residuals + oracle A/B).

**Strict count:** drops from 2904 (§2.5) — the AS-reassembly halves retire, the DSM
tranches collapse; report the exact number at Stage 2.

---

## §5 — THE POST-FLIP ARC HANDOFF

What the optimization sweep + language-ask round + capstone sweep inherit:

- **The master step-5 optimization backlog (§17 = the per-file section index in
  `campaign-gap-ledger.md:1092`)** — the 2026-07-16 emp-port optimization review, 13
  per-file deep reviews + cross-file priority: tile_cache #1/#2, plane_buffer #1–4,
  collision_lookup #1–3, sprites H1–H3, rings R2/R3, animate A2/A3, entity_window
  High #1, core #1. All park with STATUS banners; the port-loop PARCEL-SCOPE amendment
  (`campaign-port-loop.md:609`) says each runs at ~HALF cost after asl retires (one
  source file, no twin lockstep, no re-pin) — the flip is precisely what unlocks the
  cheap sweep.
- **The oracle-A/B items** — G9 (`player_ground` `d7` high-word clear, ledger t35
  `:1785`, byte-changing → needs oracle A/B); the **parallax-hardening parcel** (row 35
  / ledger t41 `:1825` — the engine mode-3 write from `Parallax_Active_Config` + the
  harness force-write deletion, with the two named A/B checks: no render regression +
  no perturbation of the `Debug_Scene_Freeze` cache-fill soak).
- **The language-ask round** — the in-cell `winptr()` ask (seam-2 stand-in shipped);
  the **`.b`/`.w` imm-link deferral** (unblocks rows 4/18/22's reverse-seam ordinal +
  game-config flips — the single most-referenced language dep in the kill list); the
  **typed-extern grammar** (row 10's 2 typed `SFXID_RING_*` mirrors); the
  **link-time-equ-off-external-base** capability (rows 52/90); a **data-table DSL**
  (rows 46/69 boot_data cursor + SpawnDesc, mt_bank dense-conditional — `SPEC2` D2.36
  `table` not-yet-dense); the **game-contract-hook mechanism** (rows 9/45 macro seams).
- **The B5/B6/B7 sweeps** — B5 = the ~40-proc 68k comma-group→`/`-separator contract
  spelling sweep (byte-neutral, now that the CPU-split rule is TEXT,
  `campaign-port-loop.md:260`); B6/B7 = the comment backlogs (the codename-reference
  audit ~40 sites/16 files + present-tense rewrites, port-loop step-3(b)).
- **The census-refresh duty** — re-run the extern census + the newtype-candidate census
  + **the kill-list backfill sweep** (row 5's own noted lag: the tranche13-16 gate-off
  body twins — load_object/entity_window/section/collision/tile_cache — were never
  appended to row 5's enumeration; the flip's §3.2 walk supersedes it but the row
  should be reconciled).
- **The capstone sweep** — the POST-TWIN-RETIREMENT full-corpus retrospect (already a
  ledger row); the debug-runtime rewrite (the `.emp`-native diagnostics runtime owning
  the message format, subsuming rows 21/53); full D5 (§4 Stage 4).

---

## OPEN QUESTIONS (for the overseer / the morning report)

- **OQ-1 (the demo game — the biggest one):** the row-5 engine twins are load-bearing
  for demo's build (`demo/main.asm:42` includes `engine.inc` → the engine `.asm`).
  Stage 2 breaks demo unless demo ALSO flips to `sigil build` (its game-side `.asm`
  consumed by `sigil-frontend-as`, linked against the native `.emp` engine, `demo.bin`
  joining the goldens). Design decision: flip demo in lockstep (recommended, feasible
  via the surviving AS frontend) vs park demo. §3.2-K.
- **OQ-2 (keep asl-the-binary?):** §2.4 — retiring asl from the BUILD ≠ deleting the
  asl binary. Keeping it as a manual re-validation oracle for major new code costs
  nothing and preserves the live independent witness for the exact case §2.2 loses it.
  Volence's call.
- **OQ-3 (generator/level-tree parcel timing):** §3.2-G — own precursor parcel vs
  residual-AS-then-later. Recommend own parcel (a data seam) but it is NOT a flip
  blocker; the residual path works meanwhile.
- **OQ-4 (checksum home):** fold the Genesis header checksum into `emit_rom` vs keep
  `tools/fixheader`. Execution detail; either retires asl fine (§1.5).
- **OQ-5 (ownership-flip export mechanism):** rows 1/7/11 flip the constants/structs
  `.emp` to sole definitions, but residual AS data still READS them — needs the `.emp`
  → residual-AS equ-export path (the reverse seam) working at Stage 3. Confirm it
  covers the residual data files, not just the (now-deleted) code twins.
- **OQ-6 (map-manifest scope):** how much of the per-shape/off-canonical org geometry
  (kill rows 6/58) the grown `sigil.map.toml` expresses declaratively vs computes as
  link outputs — an execution-parcel detail once the native driver stands up (Stage 1).
