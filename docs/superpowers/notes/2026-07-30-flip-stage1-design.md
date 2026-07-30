# 2026-07-30 — FLIP STAGE 1 design: the dual native whole-ROM build (DESIGN CHECKPOINT)

Status: **DESIGN NOTE ONLY — answers the brief's §2 six questions + the staged
execution plan. No implementation, no build change, no aeon modification.**
Sigil branch `flip-stage1`, worktree `.worktrees/flip-stage1`, base master `9b7ee04`.
Aeon is READ-ONLY here; all `.asm`/`.inc`/`.emp`/`.bin` cites are aeon `34023be`,
all `.rs`/map/spec cites are sigil `9b7ee04` (or `empyrean` for `SIGIL_*.md`).

**Baseline (verified read-only, no build triggered):** `aeon/s4.bin` =
**eff2396f / 413577**, `aeon/s4.debug.bin` = **1e9097bc / 421579**,
`aeon/demo.bin` = **2b71b37d / 88738**. PRIMARY assembled-ROM provenance
`e5765873` / `dab4f06c` (PROVENANCE.md:1993-2017). All match the brief.

**Binding constraints (Volence rulings, from the brief §0):** everything Stage 1
does is ADDITIVE — asl stays the default and byte-identical (eff2396f/1e9097bc,
PRIMARY e5765873/dab4f06c unmoved on the default path); `sigil-frontend-as` is
PERMANENT (never design toward its deletion); the demo flips IN LOCKSTEP;
checksum folds into the sigil emit path. Rollback = revert (nothing deleted).

---

## §0 — THE LOAD-BEARING FACTS (verified, before the answers)

1. **`sigil build --aeon` today is GATES-OFF.** `run_build` (`sigil-cli/src/main.rs:793`)
   → `assemble_full_rom` (`sigil-harness/src/lib.rs:66,80`) sets ONLY
   `SOUND_DRIVER_ENABLED` (+`__DEBUG__` for the debug entry), **no `SIGIL_EMP_*`
   define**. So the whole `main.asm` tree assembles through `sigil-frontend-as`
   as pure `.asm` — every `ifndef SIGIL_EMP_X` takes the INCLUDE arm, zero `.emp`
   modules are linked. This is the M1 byte-identity result (whole `s4.bin` ==
   asl). With `-o`, `emit_rom(&img, &map)` writes the ROM reading
   `sigil.map.toml` (`main.rs:814-825`).

2. **The checksum fold is ALREADY DONE.** `emit_rom` (`sigil-link/src/lib.rs:725-736`)
   calls `apply_header_checksum` as its final pass — the 16-bit big-endian
   word-sum over `[0x200, EOF)` written at `$18E` (`lib.rs:740-743`), i.e. exactly
   `tools/fixheader`. OQ-4's ruled default is implemented; Stage 1 inherits it for
   free. `tools/fixheader` keeps serving the asl default path during the dual state.

3. **convsym is a VERIFIED NO-OP on the current tree.** Both shipped ROMs contain
   **zero `deb2` bytes** (grep of `s4.bin` and `s4.debug.bin`): each is the pure
   `p2bin`+`fixheader` image, no appended symbol table. `build.sh:158-162` wraps
   `convsym … -a 2>/dev/null || true` and it appends 0 bytes (its `as_lst` parser
   finds no table it will emit for this source). This RESHAPES design-question 3
   entirely (see Q3) — and it is the single most load-bearing verified fact for
   the "full-file debug identity" bar.

4. **The mixed-link machinery is the flip in miniature, ALREADY BUILT to
   near-completeness.** 45 distinct gate-flips are exercised across the
   `assemble_mixed_tranche*`/`assemble_mixed_*` helpers (`sigil-harness/src/lib.rs`),
   consumed by `mixed_dac_rom.rs` (52 `#[test]`s) + `mixed_offcanonical_rom.rs`
   (4). Each gate arm's `else org <resume>` is exercised, the `.emp` module placed
   at its real LMA, one `resolve_layout`+`link` over the union, compared
   whole-ROM to the asl reference via `assert_rom_matches_convsym`
   (`lib.rs:784`). **What is missing is not any single gate — it is a build that
   flips ALL 53 at once through ONE registry.**

---

## §1 — THE GATE CENSUS (design question 1)

### 1.1 The count

**53 `SIGIL_EMP_*` code/data gates** live in `games/sonic4/main.asm` +
`engine/engine.inc` (unique, excluding the three `*_BODY_STUB` DSM-harness-only
defines). Enumerated:

```
ANIMATE BG BG_ANIM BOOT BUFFERS CAMERA CHILDREN COLLISION COLLISION_LOOKUP
COMPRESSION_SELFTEST CONTROLLERS CORE DMA_QUEUE DPLC ENTITY_WINDOW ERROR_HANDLER
GAME_DEBUG GAME_LOOP HBLANK LOAD_ART LOAD_OBJECT MATH OBJDEFS OBJECT_TEST_STATE
OJZ_SCROLL_TEST PARALLAX PARTICLE_ANIMS PATH_SWAP PLANE_BUFFER PLAYER_AIR
PLAYER_GROUND PLAYER_SENSORS PLAYER_SPINDASH RINGS S4LZ SECTION SONIC SONIC_ANIMS
SOUND_API SOUND_DEBUG SPRITES TEST_ANIMATED TEST_CHURN TEST_EMITTER TEST_OBJECTS
TEST_PARENT TEST_STATIC TEST_STRESS_EMITTER TILE_CACHE VBLANK VDP_INIT VECTORS
```

Plus **two INTERNAL-gate keystones** exercised but not appearing as manifest
`ifndef` arms — `SIGIL_EMP_TEST_PLAYER`, `SIGIL_EMP_TEST_ENEMY` (their zero-byte
headers stay AS-visible; kill rows 72/84/85). And **four sound gates that already
COLLAPSED** off the manifest — `SIGIL_EMP_DAC/MT/SFX/Z80_SOUND` no longer exist as
`ifndef` arms in main.asm/engine.inc: the sound blobs are emitted natively by
`emit_sound_blob` (seam-1/seam-2) and BINCLUDE'd unconditionally. So the true
gate-flip surface Stage 1 must turn ON is **53 manifest gates + the sound stack
(already native) + the 2 internal keystones**.

### 1.2 Per-gate native-placement status — the gap, precisely

Every one of the 53 gates already has (a) a ported `.emp` module (83 `.emp` files
exist under `engine/`+`games/`) and (b) a `*_port.rs` window oracle proving its
ENCODING. They split on whether their REAL-IMAGE placement (gate-flipped in the
full `main.asm` build, `.emp` spliced at the true LMA, whole-ROM diff) is proven:

- **39 of 53 have whole-ROM real-placement gates** — via the
  `assemble_mixed_tranche*` helpers in `mixed_dac_rom.rs` (+ `game_debug`,
  `sound_debug` via `mixed_offcanonical_rom.rs`). These are placement-AND-cross-seam
  proven in the real image.
- **14 of 53 are proven ONLY as isolated window oracles** (encoding, not
  real-image placement): **BG, BG_ANIM, CAMERA, CORE, DPLC, ENTITY_WINDOW,
  LOAD_OBJECT, OBJDEFS, PARALLAX, PLANE_BUFFER, SECTION, SPRITES, TILE_CACHE,
  VECTORS.** Each has a `*_port.rs` (`bg_port.rs`, `tile_cache_port.rs`,
  `sprites_port.rs`, `vectors_port.rs`, …) that assembles the isolated `.asm`
  twin as its own reference and compares a bare-region placement — it never sets
  `SIGIL_EMP_*` in the real build. **Stage 1 flipping every gate ON is the FIRST
  time these 14 modules are placed in the real ROM image** — the highest-value
  new coverage the stage produces, and the likeliest place a cross-seam or
  ordering surprise surfaces.

### 1.3 The three concrete missing pieces

1. **No all-gates-ON entry point.** The tranche helpers hand-list defines
   per-tranche and the cumulative chain broke after the early tranches (t38/t39/t41
   name only their single new gate — `lib.rs:1153,1179,1205`). Nothing sets all 53
   at once. `assemble_full_rom` must gain a gates-ON sibling that defines every
   `SIGIL_EMP_*`.
2. **No module registry.** Each mixed test hand-codes which `.emp` file, which
   section name, and which LMA to place (`placed_emp`, `mixed_offcanonical_rom.rs:86`;
   the ~40 `.emp` module wirings scattered across `mixed_dac_rom.rs`). Stage 1
   needs ONE registry that discovers, lowers (with each module's build-shape
   defines), and places EVERY `.emp` module into the single full-ROM link — the
   `sigil build` driver's new core. `sigil emp --root … --map` already links a
   whole reachable `.emp` PROGRAM through a manifest (`main.rs:569-676`,
   `build_program` + `place_sections` + `link_rom`); that machinery is the seed,
   but it links a pure-`.emp` program, not the MIXED `.emp`+residual-`.asm` union
   the real build is.
3. **Placement LMAs live in test code, not the map.** The real LMAs (region bases,
   per-shape resume points) are literals in Rust test bodies + inline `org`s in
   `main.asm`/`engine.inc`. They must move into `sigil.map.toml` as named section
   placements / computed link outputs (Q2).

### 1.4 Emitted-blob BINCLUDEs vs native linking of the same `.emp`

The sound stack is the resolved precedent. `emit_sound_blob` (`seam1::emit_sound_blob`
+ `seam2::emit_dac_artifacts`) lowers+links the five resident sound `.emp` +
DAC/MT/SFX `.emp` to `.bin`s that asl BINCLUDEs (`build.sh:77-91`). In a full
NATIVE build those same `.emp` modules are lowered+placed DIRECTLY into the one
image — the `.bin` round-trip is purely an artifact of the dual state (asl cannot
lower `.emp`, so it consumes the pre-emitted bytes). seam-1's native link
(`seam1_native_link.rs`) and the `seam2_*_rom.rs` gates already prove the native
link yields the exact BINCLUDE'd bytes; the full driver just places those
sections in-line instead of via BINCLUDE. **No new mechanism — the sound gates
are the completed template for the other 53.**

---

## §2 — THE MAP-MANIFEST GROWTH PLAN + OQ-6 RULING (design question 2)

### 2.1 What `sigil.map.toml` is today

Two regions (`sigil.map.toml:5-17`): `rom` (lma_base 0, size 0x400000, fill 0x00)
and `z80_moving_trucks_bank` (lma_base 0x60000, size 0x8000, vma_base 0x8000 —
the LMA≠VMA banked window). `emit_rom` validates each section's region
containment/budget against this (§7.3).

### 2.2 What grows in

The gate-resume `org`s encode section ENDS (spec5-flip-design §1.3). They move
into the map as three kinds of fact:

- **Fixed region geometry (DECLARE):** the object-code bank base + budget
  (`org $10000 … if * > $20000 / error`, `engine.inc:653-662`), the banked
  `$8000`/`$58000`/`$60000` sound windows (already declared), any other
  hardware-fixed base. These are TRUE constraints, not dual-build artifacts —
  they stay declarative region entries.
- **Section ORDERING (DECLARE):** the ordered section list within each region
  (object bank: player→test objects, `main.asm:21-98`; the "sfx blobs before
  sfx_table" class). Per `SIGIL_SPEC2_LANGUAGE.md:§3.3`, ordering is "an explicit
  ordering manifest in the map file" and `__BUDGET_*` markers "become per-section
  size reports for free." Declare the order; the sizes report out.
- **Per-shape / off-canonical resume points (COMPUTE):** the kill-row-6/58
  literals — `main.asm:445/447` object_test_state `$5E2DA`/`$5C7EC`,
  `engine.inc` sound_api `$80E4`/`$6414`, the `mixed_offcanonical_rom` Config-A/B
  arm orgs (`game_debug $6408`, `sound_debug $827C`, `z80_init $3FE`). In a native
  build the placement of one section IS the resume point of the next — these are
  LINK OUTPUTS, not inputs.

### 2.3 OQ-6 RULED — declarative geometry, COMPUTED resume points

**Ruling: declare region geometry + section ordering; COMPUTE every per-shape /
off-canonical resume `org` as a link output. Do not pin any resume address the
map can derive from placement.**

Reasons: (1) The resume-orgs exist ONLY because the dual build needs asl to skip
a region and land the next include at the right spot — kill rows 6/58 both die at
Stage 2 regardless. Pinning them into the map would import the exact re-pin tax
(`repin.rs`, `pins.rs`) the flip is meant to end; computing them makes the whole
per-shape geometry fall out of one placement pass. (2) It matches SPEC2 §3.3's
stated end-state (ordering manifest + size reports, not address literals).
(3) It is strictly SAFER: a computed resume can never drift out of sync with the
section it resumes after, whereas a pinned literal can (the entire kill-row-58
"re-derive on any reference re-baseline" hazard). The ONLY literals that remain
are the genuinely-fixed hardware/budget bases in 2.2's first bullet.

Consequence: the map after Stage 1 = the 2 current regions + the object-code-bank
region (base `$10000`, budget `$20000`) + a declared section-ordering list per
region. Every kill-row-6/58 address becomes a computed output the whole-ROM gate
validates by byte-identity, not a pin.

---

## §3 — LISTING / CONVSYM (design question 3)

### 3.1 The reshaping finding

**convsym appends nothing.** Verified: `s4.bin` and `s4.debug.bin` both contain
zero `deb2` bytes; each is the pure `p2bin`+`fixheader` image (413577 / 421579).
`SIGIL_CORE_SPEC.md:83,92,106` (D7) states this as the contract: "Core appends
nothing to the ROM (convsym is a verified no-op)." The debug artifact
`1e9097bc`/421579 has **no appendix** — its extra bytes over the plain ROM are
`__DEBUG__` code, not a symbol table.

**Therefore full-file debug identity does NOT depend on the listing/convsym path
at all.** `native s4.debug.bin == asl s4.debug.bin == 1e9097bc/421579` reduces to
`emit_rom(debug-shape) + apply_header_checksum == golden`, full-file, because
there is nothing after the assembled ROM. The checksum fold is already in
`emit_rom` (§0.2). The brief §1.4's "sigil listing → convsym → appendix
byte-identical" concern is **moot for the ROM bytes on the current tree** — the
appendix is empty. This is a materially easier bar than the brief assumed, and it
should be stated to Volence plainly.

### 3.2 What the listing IS still for, and the real (small) gap

`emit_listing` EXISTS (`sigil-link/src/listing.rs:18`) and emits an
AS-`.lst`-compatible file: address-sorted `[*]NAME : HEX C|- |` rows under the
`Symbol Table (* = unused):` header (parseable by `s4budget.py::parse_symbol_table`),
PLUS Oracle `ParseLineHeader` body lines `(0) N/HEXADDR :        Name:` that
`Symbols.cpp::LoadFromAsListing` reads (unit-proven, `listing.rs:56-105`). Its job
is the `.lst` FILE consumers — `build.sh:159-177` (`convsym` input + `s4budget.py`
summary) and Oracle `emulator/load_symbols` — **not ROM bytes**. Per
`SIGIL_CORE_SPEC.md:61-62` (D4), the requirement is that the listing be
FUNCTIONALLY parseable by those two consumers, NOT byte-identical to asl's
3.2 MB / 8.4 MB `.lst` (which sigil neither can nor must reproduce).

Gap for Stage 1: `emit_listing` is not WIRED to WRITE a `.lst` file from any build
entrypoint (`run_build` writes only the ROM). Stage 1's native driver should emit
`<ROM_NAME>.lst` alongside the ROM so `s4budget.py`/Oracle keep working when the
native path drives the build. Acceptance = `s4budget.py` produces a budget summary
and Oracle loads symbols from the sigil `.lst`; **not** a byte diff against asl's
`.lst`.

### 3.3 The one watch item

IF a future debug build ever grows enough symbols that `convsym`'s `as_lst` parser
STARTS emitting a `deb2` table (today it emits nothing), THEN the ROM WOULD gain an
appendix and native-`.lst` → convsym → appendix WOULD need a byte gate. On the
`34023be` tree it does not. Flag as a standing check: the Stage-1 full-file debug
gate must assert `deb2`-absence in the golden (a positive control that convsym is
still a no-op), so this assumption cannot silently rot. **OQ-A1 below.**

---

## §4 — THE DEMO CENSUS + LOCKSTEP (design question 4)

### 4.1 How demo builds today

`games/demo/main.asm` defines the seven game-contract macros
(`gameConfigIncludes`, `gameRamIncludes`, `gameEngineBlockIncludes` (EMPTY),
`gameObjectBankIncludes` → `demo_box.asm`, `gameDataIncludes` → `demo_data.asm`,
`gameSoundDataIncludes` (EMPTY), `gameStatesIncludes` → `demo_state.asm`) then
`include "engine/engine.inc"` (`demo/main.asm:42`). **The engine's ROM layout AND
every `SIGIL_EMP_*` engine gate live in `engine.inc`, so all 53 engine gates apply
to demo through that include.** `build.sh GAME=demo` (`build.sh:4,13`) drives it;
`ROM_NAME=demo`. demo builds **sound-OFF** (`games/demo/build.conf`:
`SOUND_DRIVER_ENABLED:=0`), so the DAC/MT/SFX/sound_api/z80_sound stack is
excluded and the no-sound `z80_init` off-canonical arm applies. `demo.bin` exists
(**2b71b37d / 88738**, `PAD_TO_POWER_OF_TWO=1`).

### 4.2 Why Stage 2 breaks demo, and the lockstep

At Stage 2 the row-5 engine `.asm` code twins DELETE. demo currently includes
`engine.inc` → those `.asm`; deleting them breaks demo's build unless demo ALSO
builds through `sigil build` (its game-side `.asm` — `demo_box`, `demo_data`,
`demo_state`, `config/*` — consumed by `sigil-frontend-as` as residual, linked
against the native `.emp` engine). This is feasible precisely because
`sigil-frontend-as` is PERMANENT. **Ruling in force (brief §0): demo flips in
lockstep.** So Stage 1 must:

- Stand up `sigil build --aeon . --game demo` (or equivalent) — the gates-ON
  native path for the demo manifest, engine gates all ON, sound stack OFF, demo
  game-side residual AS.
- Freeze `demo.bin` (2b71b37d/88738) as a committed golden and prove the native
  demo build reproduces it full-file. Demo has NO game-specific `.emp` today
  (demo objects stay residual AS) — so demo's native build is the pure "native
  `.emp` engine + residual-AS game" configuration, a valuable independent
  exercise of the engine gates in a DIFFERENT surrounding program than sonic4.

### 4.3 Demo shapes

demo has no `SOUND_DEBUG_HOTKEYS`/mirror surface. Its shapes are plain and
`__DEBUG__` (the engine's debug code still gates on `__DEBUG__`). Freeze both if
a `demo.debug.bin` is a meaningful build; the design defaults to freezing at least
plain `demo.bin` (present, CRC'd) and capturing `demo.debug.bin` at execution if
`DEBUG=1 ./build.sh demo` produces one. **OQ-A2.**

---

## §5 — THE OFF-CANONICAL GOLDEN FREEZE (design question 5)

From `mixed_offcanonical_rom.rs:148-267`, the two off-canonical configs are the
canonically-empty consumers that no shipped ROM contains:

- **Config A (debug):** `__DEBUG__` + `SOUND_DRIVER_ENABLED` + `SOUND_DEBUG_HOTKEYS`
  + `SOUND_DBG_MIRROR` — one combined build serving `game_debug` (region
  `[0x6356,0x6408)`) AND `sound_debug` (region `[0x81B0,0x827C)`). One shape (it is
  inherently debug).
- **Config B (no-sound):** `SOUND_DRIVER_ENABLED` off, plain — serves `z80_init`
  (the `boot_data.asm` no-sound `else` arm, region `[0x3D8,0x3FE)`, 38 B phased
  Z80). One shape.

**The frozen set Stage 1 commits (the maximal golden freeze, brief §1.6 /
spec5-flip-design §2.3 mitigation 1):**

| # | golden | config | provenance to pin |
|---|---|---|---|
| 1 | sonic4 `s4.bin` plain | canonical, sound-on | eff2396f / 413577 (assembled e5765873) |
| 2 | sonic4 `s4.debug.bin` | canonical, `__DEBUG__` | 1e9097bc / 421579 (assembled dab4f06c) |
| 3 | sonic4 Config-A ROM | debug+hotkeys+mirror | capture at execution (no shipped file today) |
| 4 | sonic4 Config-B ROM | no-sound plain | capture at execution |
| 5 | demo `demo.bin` plain | sound-off | 2b71b37d / 88738 |
| 6 | demo `demo.debug.bin` | sound-off `__DEBUG__` | capture at execution (OQ-A2) |

Kill rows **55** (z80_init twin) and **58** (off-canonical gate-arm org pins) die
at Stage 2 — their `mixed_offcanonical_rom` gates re-comparand from "pure-asl
reference == mixed placement" to "native == frozen Config-A/B golden."

**Freeze form:** the campaign standard is CRC32+size provenance, and today's gates
diff against the LIVE `aeon/s4.bin` FILE (no committed blob — PROVENANCE.md M0/M1
notes). But asl LEAVES at Stage 2, so the Config-A/B ROMs (which have no shipped
file and are only reproducible while asl can assemble them) MUST be captured as
COMMITTED byte-for-byte artifacts during Stage 1, while asl is still live to
vouch for them. Recommendation: commit the six ROMs as tracked golden blobs under
`crates/sigil-harness/golden/` (or an aeon-side frozen dir) WITH a
CRC32+size+provenance note, captured under the t24 positive-control /
negative-probe discipline (undoctored == golden AND a doctored `.emp` != golden,
kept verbatim). This is the one place Stage 1 must produce durable artifacts, not
live-rebuild gates — it is the regression oracle Stage 2+ leans on.

---

## §6 — THE PROOF MATRIX + STRICT-SUITE GATES (design question 6)

### 6.1 The Stage-1 exit bar (named gates)

For every (game × shape × config), one NEW whole-ROM native gate asserting
`sigil-native whole ROM == asl whole ROM == frozen golden`, full-file including
the (empty) appendix:

| gate (proposed name) | asserts |
|---|---|
| `native_rom_sonic4_plain` | native gates-ON full build == eff2396f == asl |
| `native_rom_sonic4_debug` | native gates-ON `__DEBUG__` == 1e9097bc == asl (full-file; deb2-absent positive control, §3.3) |
| `native_rom_config_a` | native gates-ON Config-A == captured golden == asl |
| `native_rom_config_b` | native gates-ON Config-B == captured golden == asl |
| `native_rom_demo_plain` | native demo (engine gates ON, sound off, game residual AS) == 2b71b37d == asl |
| `native_rom_demo_debug` | native demo `__DEBUG__` == captured golden == asl (OQ-A2) |
| `native_listing_consumers` | sigil `.lst` parses under `s4budget.py` + Oracle `load_symbols` (functional, not byte) |

Each keeps its t24 controls (positive control + a doctored-`.emp`-diverges
negative probe) so the golden gates never go vacuous. These ADD to the strict
suite; the 39 tranche gates + 14 window oracles + `mixed_offcanonical_rom` stay
green and the asl default path stays at eff2396f/1e9097bc, PRIMARY
e5765873/dab4f06c.

### 6.2 What Stage 2 re-comparands them to

At Stage 2 asl leaves the build and the `.asm` twins delete. The `== asl` clause
DROPS from every gate above (no live asl to re-assemble); each becomes
`native == frozen golden` (the CRC-pinned artifacts §5 commits). Concretely: the
~39 tranche whole-ROM gates + the ~52 DSM tranches in `mixed_dac_rom.rs` COLLAPSE
(their AS-side `assemble_mixed_*` half has no `.asm` to assemble once the twins are
gone) into the handful of §6.1 native-vs-golden gates; the 14 window oracles lose
their isolated-`.asm` reference and either retire or re-comparand to a golden
slice; the `mixed_offcanonical_rom` gates re-point to the committed Config-A/B
goldens. The strict COUNT drops (the AS-reassembly halves retire) — the exact
number is a Stage-2 execution output. **This is why Stage 1 is the point-before-
no-return: it is the last moment asl is a live independent witness on the full
program, and the §6.1 gates capture that witness as frozen goldens before it goes.**

---

## §7 — STAGED EXECUTION PLAN (on endorsement)

Each aeon-touching commit is dual-proven (asl default path byte-identical +
native path added). All additive; rollback = `git revert`.

**S1.1 — the gates-ON native driver (sigil-only, no aeon change).**
Add `assemble_full_rom_gates_on` (all 53 `SIGIL_EMP_*` + the sound stack) + the
`.emp` module registry that discovers/lowers/places every module into one
`resolve_layout`+`link`+`emit_rom`, generalizing `placed_emp` + the tranche
composition into ONE driver. Wire it behind a non-default `sigil build` flag/env
(default stays gates-OFF, untouched). *Identity bar:* native gates-ON `s4.bin` ==
eff2396f == the gates-OFF native build == asl, both shapes. *Rollback:* revert;
the new path is additive.

**S1.2 — grow `sigil.map.toml` (§2).** Add the object-code-bank region + declared
section ordering; make resume points computed link outputs. *Identity bar:*
unchanged — S1.1's native ROMs still == eff2396f/1e9097bc; the map change is a
refactor of where placement facts live, byte-neutral. *Rollback:* revert map.

**S1.3 — the 14 first-time real-image placements (§1.2).** Flip BG/CAMERA/
TILE_CACHE/SPRITES/… ON in the native driver (they enter via S1.1's all-gates-ON
set — this step is the PROOF, not new code): confirm each places + cross-seam
links in the real image. *Identity bar:* whole-ROM native == golden holds with
these 14 ON (the highest-risk step). *Rollback:* if one misplaces, isolate it OFF
in the native set (still additive; asl unaffected).

**S1.4 — wire the native `.lst` emit (§3.2).** Have the native driver write
`<ROM_NAME>.lst` via `emit_listing`. *Bar:* `s4budget.py` + Oracle `load_symbols`
consume it. *Rollback:* revert; the asl `.lst` still serves the default path.

**S1.5 — demo lockstep native build (§4).** Stand up the demo native path (engine
gates ON, sound off, game-side residual AS). *Bar:* native `demo.bin` == 2b71b37d
== asl. *Rollback:* revert; asl builds demo.

**S1.6 — capture + commit the six frozen goldens (§5)** under t24 discipline,
with CRC32+size provenance. *Bar:* each == its live asl build at capture time; the
Config-A/B ROMs deb2-absent. *Rollback:* revert the golden dir.

**S1.7 — the seven Stage-1 gates (§6.1)** added to the strict suite. *Bar:* strict
suite green, asl default path unmoved at eff2396f/1e9097bc + PRIMARY
e5765873/dab4f06c. *Close checkpoint:* the full proof matrix, overseer own-run,
merge.

### Workload estimate

**Medium-large — roughly 5–8 focused porter-days**, dominated by S1.1 (the module
registry + all-gates-ON driver: the one genuinely new mechanism, though every
piece exists in the mixed harness to lift from) and S1.3 (the 14 first-time
real-image placements — the empirical risk, may surface cross-seam/ordering
surprises needing per-module diagnosis). S1.2/S1.4/S1.6/S1.7 are mechanical
(1 day total). S1.5 demo is ~1 day (a second, simpler manifest through the same
driver). Add contingency for whatever the 14 placements turn up — the honest range
is 5 days if they place cleanly, 8 if two or three fight.

---

## OPEN QUESTIONS (for the gate)

- **OQ-A1 (deb2 no-op durability):** §3.3 — the current tree's convsym appends 0
  bytes (verified: no `deb2` in either ROM), so full-file debug identity needs no
  appendix reproduction. The Stage-1 debug gate should assert deb2-ABSENCE as a
  positive control so this can't silently rot. Confirm this is the intended bar
  (vs. proactively building a native deb2 emitter now — D7 says Core must NOT
  append, so I recommend the assert-absence control, not an emitter). LOAD-BEARING:
  it is the difference between "Q3 is nearly free" and "Q3 needs a convsym
  reimplementation."
- **OQ-A2 (demo debug shape):** §4.3/§5 — does `DEBUG=1 ./build.sh demo` produce a
  meaningful `demo.debug.bin` to freeze, or is demo plain-only for the golden set?
  No `demo.debug.bin` exists in the tree today; capturing it is cheap if wanted.
- **OQ-A3 (golden freeze location + form):** §5 — commit the six ROMs as tracked
  binary blobs under `crates/sigil-harness/golden/` (departing from the current
  live-file/CRC-pin discipline, which cannot survive asl's departure), vs. an
  aeon-side frozen dir. Recommend committed blobs in sigil with CRC provenance —
  they must be durable byte-for-byte artifacts for Stage 2+, not live rebuilds.
  Confirm the home.
- **OQ-A4 (registry scope — residual-AS boundary):** §1.3 — the native driver must
  link native `.emp` modules AND residual `.asm` DATA (config, generated level
  tree, parallax data, demo game-side) via `sigil-frontend-as` in ONE image. The
  mixed harness proves this union for the ported set; confirm the residual-AS
  files enumerated in spec5-flip-design §3.2 (buckets G/H — generated level tree,
  parallax, sprite data) all assemble clean through `sigil-frontend-as` in the
  full native build, since Stage 1 is the first time they ride the native driver
  rather than asl. If any residual file uses an AS construct `sigil-frontend-as`
  doesn't yet cover, that is a Stage-1 blocker to surface early (not a guess to
  paper over).
- **OQ-A5 (Config-A/B capture reproducibility):** §5 — the Config-A/B ROMs have no
  shipped file and are reproducible only while asl is live. Confirm capturing them
  during Stage 1 (not Stage 2) is acceptable, and that the capture config strings
  (`CONFIG_A`/`CONFIG_B`, `mixed_offcanonical_rom.rs:152-263`) are the canonical
  off-canonical set to freeze — vs. a wider off-canonical sweep Volence wants.
