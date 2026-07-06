# Sigil Spec 2 · Plan 7 — Language-Completion Research (candidate feature set)

> Research half of Plan 7 (language finalization). Produced 2026-07-06 overnight by
> four parallel agents mining **every local disassembly** (s2disasm, skdisasm, S.C.E.,
> sonic_hack, and the active **aeon** target) plus **online** modern-assembler /
> retro-dev sources. Every frequency below is `grep`-grounded in a real tree. This is
> **input to a design decision**, not a spec — Plan 7's finalization half decides
> in/out, spells the syntax, closes the deferred ledger, and freezes
> `SIGIL_SPEC2_LANGUAGE.md`. Companion memory: [[emp-data-table-dsl-candidates]],
> [[jbra-jbsr-auto-reaching-branches]].

## How to read this
Candidates are ranked by **cross-source convergence × frequency × safety win**. A
candidate that showed up independently in multiple engines *and* has high raw
frequency *and* replaces a hand-written correctness guard is a Tier-1 buy. Each entry
notes what `.emp` already covers so we don't rebuild it.

The dominant theme across all four agents: **the recurring idioms are typed
data-table / byte-sequence constructs where a count, an offset, an index, or a size
is computed and a range is checked** — exactly the `.emp` comptime + totality story.
Several apparently-separate candidates (animation scripts, palette scripts, SMPS,
PLC lists) are *the same construct at different scales*.

---

## Tier 1 — buy these (multi-source, high-frequency, safety-critical)

### T1-a. Offset-table type — **bidirectional** (table **+** ID enum)
The #1 idiom in every tree: **14,179** `dc.w Target-Base` lines (S3K), **4,627** entries
(S2), 866 (legacy), and it **blocks the representative aeon data ports** (particle_anims,
test_mappings, sonic_anims) — `.emp` can't express a symbol *difference* in data today.
Crucially, S2 also has the **inverse**: `id()` derives a named constant from a label's
*index* in the table — **778 hand-synced cross-file constants** (217 ObjID, 79 SndID, …),
each requiring the author to set three globals (`offset`/`ptrsize`/`idstart`) before the
block. **SCE promoted both to named `offsetTable`/`offsetTableEntry` + `id` macros** —
independent confirmation this is a deliberate abstraction.
- **Construct:** declare targets once → compiler emits `dc.w t-base`, derives the entry
  count, **and** hands back an ordinal index enum. `> $7FFF` fit checked automatically.
- **Win:** kills the manual `-Base` subtraction, ~all `(End-Start)/n == COUNT` asserts,
  the 4 hand `>$7FFF` guards, **and** the 778 drift-prone ID constants. Inserting a row
  can never silently renumber downstream IDs. Byte-identical emission.
- **Note:** this is the Plan-6 blocker → the natural **first Plan-7 implementation** item.

### T1-b. Typed state machine (`state` / `machine`)
The #2 runtime idiom: **2,917 reads + 3,120 writes** of `routine(a0)` (S3K), dispatched
through offset-table jumps. An object advances state by writing a **raw byte** later
`add.w`'d into a jump index — a wrong constant or a missing table entry is an unbounded
`jmp` into arbitrary code (the classic S3K crash). **S3K and SCE disagree on the
encoding** (S3K routine-byte + dispatch vs SCE `move.l #.label, code_addr(a0)`
continuation) — a strong signal `.emp` should **own** this concept rather than leave it
to convention.
- **Construct:** an object's state field is an enum whose variants bind to code blocks;
  the compiler emits the offset table, the `add.w`/bounds check, and guarantees every
  variant has an entry (exhaustive, totality tenet).
- **Win:** turns "silent jump-to-garbage on a bad byte" into a compile error, at zero
  byte cost (identical dispatch table). Unifies the two engines' encodings behind one
  surface. (Builds on `proc`/`routine` — this is their typed container.)

### T1-c. User-definable typed byte-command DSL (`bytecode` / `format`)
The biggest *meta*-win. Animation scripts (**182** raw `dc.b` blobs, magic
`$FF/$FE/$FD/$FC`, zero macro — S2), palette scripts (negative-word control opcodes),
PLC lists, and **SMPS music (77 hand macros + 8 tempo fns, 19,871 coord lines, FM-operator
scrambling + algorithm-dependent TL masks, driver-versioned)** are **all the same
construct at different scales**: a typed, terminated, self-validating byte-command
sequence. SMPS alone carries **13 of the 40 `fatal` guards** in S2 as compatibility
checks.
- **Construct:** let an author *declare* a byte-command DSL once — opcode → operand
  layout, terminator, per-command validity predicate, comptime-conditional encodings —
  and get a checked emitter. Animation + palette + PLC + SMPS collapse onto it.
- **Win:** ~450 lines of hand-written driver-versioned `dc.b` glue → a declarative
  table; wrong arg-count / out-of-range operand / unterminated script (a real hang bug)
  become type errors. The compatibility invariants live *in the DSL definition*.
- **Scope caution:** this is the largest single feature; Plan 7 should decide how much
  of it to build vs. ship the narrow "animation-script + PLC" special-cases first.

---

## Tier 2 — strong, specific, migration-relevant

### T2-a. Bank / window placement type + a packing linker
**aeon's highest-value gap, currently unrepresentable.** 18 hand-written 32KB-straddle
`fatal`s + the same two derivations copied everywhere: `bank_id = (addr & $7F8000) >> 15`
and `window_ptr = (addr & $7FFF) | $8000` (dac_samples, song_table). The Moving-Trucks
streaming block (tables + 14.9KB song + pitch + 858B patch bank) is held together by
four `if (X>>15) <> ((Y-1)>>15)` fatals + prose begging editors to keep `align $8000`.
Online: ca65 `.SEGMENT`+linker-config and rgbds `SECTION` (BANK/ALIGN, floating
placement) let the linker **pack** objects into free regions instead of hand-ORG.
- **Construct:** a `bank { }` region / `@bank_aligned` blob exposing derived
  `.bank_id`/`.window_ptr`/`.len` with a compiler-enforced "never crosses 32KB"
  invariant; longer-term, a placement policy that fits sections into declared regions.
- **Win:** deletes the 18 fatals + the `align $8000` boilerplate; bit-twiddling becomes
  checked field access. **This is what the Z80/68k sound migration stumbles on hardest**
  — flag as a migration prerequisite for the sound subsystem.

### T2-b. `assert!` / `static_assert!` + capacity-refined regions
By raw frequency this is enormous: **aeon has 195 build guards** (147 `error` + 48
`fatal`); S2 has 40 `fatal`s of which **17 are buffer-fit checks**; S3K/SCE add
`zonewarning`, `finishBank`'s "must fit $8000", a DMA-128KB-cross check. Online: ca65
`.ASSERT` (with error/warn severity) and rgbds `ASSERT`/`STATIC_ASSERT`.
- **Construct:** free-form comptime boolean checks over **symbols/sizes** (distinct from
  refinements, which only bound a single value's domain) with error|warn severity; plus
  a `fits_within(buffer)` / `max_size` invariant attachable to a region/overlay type.
- **Win:** the ubiquitous `if size <> N / error` / `if * > limit / fatal` guards become
  inherent and *always-on* (checked for every table, not just the ones someone
  remembered to guard).

### T2-c. Per-state named overlays over shared SST scratch (union view)
**26,697** `field(aN)` accesses in S3K `Levels/` alone; ~50 generic `objoff_XX` names all
aliasing the *same* $30–$4F scratch bytes because AS has no per-object view of a shared
struct. `.emp` has typed overlays for the *base* SST; the gap is a **union/newtype view**.
- **Construct:** `overlay MyObj over SST.scratch { charge: u16, timer: u8 }`; read
  `charge(a0)`. (This is the Plan-4-pool "SST overlay + field-access-as-displacement"
  item, now quantified.)
- **Win:** eliminates the `objoff_*` magic-number vocabulary; reading another object's
  scratch layout by mistake becomes impossible. Byte-identical (`$3C(a0)`).

### T2-d. First-class counted / sentinel / sparse collection (`list<T>`)
**Six** distinct macros re-implement the identical count/back-patch/sentinel dance
(`plrlistheader`, `zoneanimstart`, `dbglistheader`, `HScroll_Header`,
`titlecardresultsheader`, `watertransheader`), plus boundary sentinels
(`ObjectLayoutBoundary = dc.w -1,0,0,0`). aeon's **SfxTable = 135 slots, 122 literal
`dc.l 0` holes** hand-typed around 9 real entries. S2's **zone-ordered tables** (568
entries, 28 decls) index-by-key with `!org` math + a hand count-mismatch warning.
- **Construct:** a counted collection whose length header (`-1`-adjusted or not), trailing
  sentinel, and *sparse* `{id: ptr}` holes are generated; plus an "indexed by enum K, N
  slots per key" variant with compile-time exhaustiveness.
- **Win:** collapses 6 fragile back-patch macro pairs into one; count *provably* matches
  entries; SfxTable's 122 holes generated from a sparse literal.

---

## Tier 3 — smaller, but each removes a real footgun

- **T3-a. Dual-CPU target-polymorphic byte** (`pbyte`/`MOMCPUNAME`). aeon: 8 macro
  redefinitions, a 251-line shared FM-patch bank assembled once as Z80 `db` and once as
  68k `dc.b`. **Migration stumble** — the module system needs a single authored blob
  emitted into two CPU targets; distinct from `embed()`. Needs an explicit spec answer.
- **T3-b. Tail-call + auto operand-sizing.** SCE hand-rolls call polymorphism in 18
  macros / 179 sites (`terminate`→jsr/jmp, `byte`→moveq/move.w). `jbra`/`jbsr` solve
  *reach*, not "this is my last act, tail-call it" or "pick the smallest move for this
  constant." `return f()` → `jmp`; literals auto-`moveq` when they fit.
- **T3-c. Version-parameterized record emission** (one schema → N wire formats).
  `SonicMappingsVer`/`SonicDplcVer`/`SonicDriverVer` branch inside the mapping/DPLC/SMPS
  emitters (15 branch sites in MapMacros). Makes cross-game asset porting a config
  change, not a destructive re-export.
- **T3-d. Charmaps** (rgbds/ca65 `NEWCHARMAP`). Named char→byte tables for menu /
  level-select / credits text; "illegal glyph doesn't compile."
- **T3-e. Per-section fill-byte control** (68k `cnop` defaults to `0x4E71` NOP word).
  The fill byte is in the ROM image — **byte-exactness-relevant** for reproducing AS gaps.
- **T3-f. Cheap / scoped local labels within a routine** (ca65 cheap locals). `.emp`
  scopes globals via modules but not intra-body label reuse.
- **T3-g. `EXPECT`/`ENDEXPECT` negative assertions** (AS) — assert a block *does* raise a
  given diagnostic; valuable for the assembler's own byte-exact self-test suite.

---

## Already covered — do NOT rebuild
Comptime table generators (`deform_table_sine`), VDP/tile math (`vdpComm`,
`make_art_tile`, `tiles_to_bytes`), DMA/state macros (`clearRAM`, `stopZ80`), typed
structs / `ObjDef` (SCE modernized this hard — validated), bitfields + nested-namespace
bit ops, `embed`/`import`/`zx0` assets, `jbra`/`jbsr` reach, `UNION`/`RS`/`.ENUM`/`REPT`
(all have `.emp` equivalents).

## Deferred-ledger items Plan 7 must also close
S2-D3 (prelude + scan/manifest module resolution — already the composition spine), S2-D6
(register/clobber contracts), S2-D7 (machine-state / CCR / stack-delta contracts), S2-D8
(dimensional types). None are on the migration critical path except S2-D3.

## Suggested Plan 7 implementation order (after the research is ratified)
1. **Offset-table (bidirectional)** — unblocks Plan-6-class data ports; #1 by every measure.
2. **Scan/manifest module resolution + prelude (S2-D3)** — unblocks code ports at all.
3. **`assert!`/capacity refinements** — 195 aeon guards; cheap, high-coverage.
4. **State machine + SST overlay view** — the object-code migration pair.
5. **Bank/window placement** — gates the *sound* subsystem migration.
6. **Counted/sparse collections**, then the **byte-command DSL** (largest; may stage).
7. Tier 3 as encountered during migration (`pbyte`, tail-call, fill-byte, charmaps…).

---

# Part II — Addendum (2026-07-06): non-Sonic engines + demoscene/modern passes

Two further passes after the first six agents: **Agent E** mined six **non-Sonic** Genesis
disassemblies (Treasure's Gunstar Heroes & Alien Soldier, Technosoft's Thunder Force IV,
BlueSky's Vectorman, Sonic Team's Ristar, and Batman & Robin), and **Agent F** covered the
**68k/Genesis demoscene** + modern low-level languages (Zig/Odin/Terra/Kaitai) online. The
non-Sonic pass is the most consequential: it **challenges the Sonic-derived candidate set**,
because these are different studios' engines. Claims verified against raw assembly.

## The two findings that REVISE Part I

### R1. The offset-table's `dc.w Target-Base` encoding does NOT generalize to *state dispatch*
Indexed jump-table dispatch is **universal** (all six engines), so the *concept* (T1-a/T1-b)
stands. But Sonic's **self-relative word-offset encoding** — Part I's #1 idiom — is a Sonic-ism
for *state dispatch*: **Vectorman** stores raw absolute 32-bit code pointers
(`move.l #NewState, $4(a4)` — no table at all), **Ristar** stores IDs pre-shifted ×4, **Treasure**
uses word-index tables. So the offset-table stays the right answer for **data pointer tables**
(mappings, DPLC, art — where it's #1 by volume), but the **dispatch/state construct must be
encoding-agnostic** (offset-relative *or* absolute-pointer *or* pre-shifted-index, chosen per
target) **and admit states as first-class function values**, not only enum indices. Baking
Sonic's offset form into state dispatch would misfit 3 of 6 engines. → **split T1-a (data offset
table) from T1-b (state machine); make T1-b encoding-agnostic.**

### R2. Byte-command DSL + state machine should MERGE into a scripted coroutine construct
Sonic revealed byte-command DSLs only for **leaf** subsystems (animation/palette/sound).
**Batman & Robin's entire object model is a two-level threaded-code bytecode interpreter** —
every object's behavior is a script of handler addresses walked by `movea.w (a2)+,a0; jmp (a0)`
(across 8 files), and the `$0820` **`yield`** opcode saves the script PC *as the state* (a
coroutine on the 68000; Ristar echoes it with yield-on-IRQ decompression). → **promote T1-b +
T1-c into one first-class *scripted state-machine / coroutine* construct** that lowers to
threaded dispatch, with **`yield` as a language primitive** and a compiler-tracked resume-point
type. This is the single feature that most distinguishes a Treasure/Batman-class engine from
Sonic, and it subsumes both prior candidates into a more powerful one.

## New candidates the non-Sonic + demoscene passes add

### T2-e. Hot-swappable / RAM-patched interrupt handlers — **near-universal (5/5)**
Every HBlank-effect engine patches its handler at runtime: `$FFFFEE00` (Treasure), `$F0F8`
(TF4), `$FFFF9D2E` (Vectorman), `$FFEA70` (Ristar — writes an inline `4EF9 jmp.l` opcode into
RAM to kill per-scanline indirection). Sonic used fixed ROM handlers, so this never surfaced.
→ a **hot-swappable interrupt-handler construct** (declare/install/replace per level) with the
inline-JMP thunk as a codegen option and a **verified vector write**. Converges with Agent F's
**safe operand-field labels for self-modifying code** (name the immediate/displacement slot so
runtime patches address by name, not `+2` offset math) — the same underlying need: *typed,
safe runtime code patching*.

### T2-f. Split update/render pipeline + bounded VBlank transfer queue
Three engines (Vectorman, Ristar, Alien Soldier) separate an **update** phase (logic) from a
**render** phase (priority-sorted display-list build, capacity-capped), then drain a
**pre-computed queue** in VBlank with *zero* math. Vectorman double-buffers with a per-entry
byte budget (`cmpi.w #$B40`); Ristar bails gracefully at the 80-sprite cap; nullable render
pointers give free culling. → entity types with distinct **`update`/`render` phases** + a
**deferred-transfer/display-list queue type** with a compiler-known **capacity + byte budget**
(static assert where possible, checked overflow-policy otherwise). Converges with Agent F's
**DMA/FIFO cost model** (`words*2.4+5.6`) — the queue's budget is exactly that closed form.

### T2-g. Cycle-budget & DMA-budget assertions — empirical demand for **S2-D7**
Demoscene raster/DMA effects live or die on cycles (~480/line; "twisters break without exact
FIFO/DMA timing"). Demos hand-count cycles today; external tools (68kcounter) exist. Folding a
68k instruction-timing table into comptime enables `assert cycles(hblank_handler) <= 480` and
`assert dma_cycles(queue) < vblank_budget`. **This is the deferred S2-D7 (`@budget`/
`@cycles_exact`) — the research shows it is less optional than it looked.** Plan 7 should
re-weight S2-D7 from "post-v1" toward "in scope," at least the cycle/DMA-budget slice.

### T2-h. Multiple collection *kinds* — flat-array is the outlier
Sonic's flat single-table SST scan is the *minority*: Batman uses an intrusive **doubly-linked
list** (O(1) alloc), Vectorman a **terminator dispatch list** + separate data blocks, TF4
**type-segregated pools** (32-byte stride, AI-free loop), Ristar a **pool feeding priority-banded
linked lists**. → the collection model must offer **linked-list / pooled-segregated /
priority-banded** kinds, not hardwire "flat array of fixed-stride slots." (Generalizes T2-d.)
Plus **typed `ref<Entity>` relation fields** (Treasure's `$58`/`$5C` parent/child/sibling links,
447/305 refs) with a relation kind + optional engine-maintained list membership.

### T3-h..T3-k (smaller, from Agent F)
- **Bit-level packed structs + layout-introspection asserts** (Zig `@offsetOf`/`@bitSizeOf`):
  extend bitfields to bit-granularity for `art_tile`/sprite-attr/VDP-command words + the 64-byte
  object map; `comptime assert offsetof(Object.subtype)==0x21` guards layout drift.
- **Struct-of-Arrays layout (`#soa`)** (Odin): Genesis tables are column-major (per-line scroll
  pairs, sprite-attr columns) — author AoS, emit columns.
- **Typed VDP/DMA command builder**: `vdp_reg(n,v) => 0x8000|(n<<8)|v` with `n: 0..24` refinement
  — a concrete instance of the byte-command DSL for the two APIs every Genesis program touches.
- **Declarative binary-format DSL** (Kaitai/DFDL): describe a ROM table's shape once → emit +
  typecheck. Validates the format-DSL thesis; the "emit code, not just data" frontier (Terra)
  pushes comptime to generate instruction sequences (unrolled speedcode), not only constants.

## Revised takeaways for the Plan 7 finalization decision
1. **Offset-table** remains the #1 *data-table* buy and the Plan-6 unblocker — but **decouple it
   from state dispatch**, which becomes its own encoding-agnostic + function-value construct.
2. The **scripted state-machine / coroutine (`yield`)** is now the marquee object-model feature —
   bigger and more differentiating than the "byte-command DSL" framing suggested.
3. **Hot-swap interrupts, the update/render+VBlank-queue pipeline, and cycle/DMA budgets** are new
   Tier-2 buys with strong cross-engine + demoscene convergence; the last **revives S2-D7**.
4. `.emp`'s **collection model** should be multi-kind (linked-list/pool/priority), not flat-array.
5. These are engine-architecture features — larger than data-table constructs. Plan 7 finalization
   must decide how much object-model opinion `.emp` should hold vs. leave to each engine (the
   Sonic-vs-Treasure-vs-Vectorman divergence is real: `.emp` should *enable* all three encodings,
   not *impose* one).
