# SIGIL-DECOUPLE — the constraint inventory taken from AEON'S side

**What this is.** `SIGIL-DECOUPLE`'s precondition, in aeon's words: *"Every constraint the
frozen tables encode today must be recaptured as an explicit rule BEFORE the tables stop being
authority, or it silently stops being enforced."* Two prior inventories
(`2026-08-26-placement-constraint-inventory.md`, `2026-08-27-constraint-recheck.md`) enumerated
from **sigil's own crates**, and the second says in writing that a pass from **aeon's build
side** *"is still owed and would very likely find more"*. This is that pass.

**The enumeration parameter, stated so a reader can tell what it cannot contain.** Three
sources, all on aeon's side:

1. **`build.sh` and the tools it invokes** — gate flags, budget checks, address arithmetic.
2. **`ensure(...)` statements inside `.emp` sources** — constraints asserted in the engine's
   own language.
3. **Commit prose in aeon's history** — constraints that exist only as a sentence someone
   wrote when they made the rule.

Every row below names which of the three it came from. A list complete against one source and
silent about the other two looks identical to a complete one, so the per-source coverage and
per-source positive control are reported separately in §Coverage.

**PIN. Aeon was read ONLY at `origin/master` = `9e3d28614cbee78ffeec74eab6e2bcd2ffc301b3`**
(*witness(loop): row-35 twin works; crown contact does not — booked and PARKED*), confirmed two
ways before work started (`git ls-remote origin master` and the fetched ref agree) and again
in a detached worktree at that SHA. The owner's live checkout `/home/volence/sonic_hacks/aeon`
was never read as a tree. Sigil was read at this branch's base, master `311ded5a`.

**Why the prior pass's basis was stale in the dimension it measures.** The 2026-08-27 re-check
read `games/sonic4/map.toml` from `.aeon-freeze-slope` at `9bba8700`. At that tree the sound
anchors were `dac_banks 0x90000` / `sound_bank 0xA0000` and the hole was `at = 0x3F8`. **At the
pin they are `dac_banks 0xA8000` / `sound_bank 0xB8000` and `at = 0x3F0`** — the 2026-09-04
re-layout (`DATA_GROWTH_RESERVE` 0x4000 → 0xC000, plus a new `DATA_GROWTH_GRACE` term) and
sigil chain 196's hole-row move. Every address in the prior note's map-derived reasoning is one
re-layout out of date.

---

## §1 — The rows

`WHERE CHECKED` is the strongest thing that fails the build. `nowhere` means no build path
refuses a violation; a dashboard that prints a number is not a check.

### Class A — the Z80 bank window ($8000)

| # | Constraint | Source | Anchor | Where checked | What lapses |
|---|---|---|---|---|---|
| **A1** | The five MT-bank stream/patch labels (`Song_MovingTrucks`, `MovingTrucks_Patches`, `Song_DrumTest`, `Song_HCZ2`, `HCZ2_Patches`) must land in the same `$8000` window as `MovingTrucks_Bank_Start` — the sequencer reads FM pitch / log-volume / opcode tables window-relative from the bank head. | **2** | `games/sonic4/data/sound/mt_bank.emp`, the five `ensure(bankid(X) == bankid("MovingTrucks_Bank_Start"), …)` | **`ensure`** — deferred to a `LinkAssert` (`sigil-link` D-H.4), so it fails the build | Nothing — this one survives decouple intact. It is stated in aeon's language and evaluated against the *placed* address, so it is indifferent to where the placement came from. |
| **A2** | `Sfx_33` must land in that same window. | **2** | `games/sonic4/data/sound/sfx/sfx_bank.emp`, `ensure(bankid(Sfx_33) == bankid("MovingTrucks_Bank_Start"), …)` | **`ensure`** (same mechanism) | Nothing. |
| **A3** | The SFX block's base must be `≡ 0 (mod 8)` — seam-2 folds the embedded `SfxTable` pointer cells and the head's `SfxBlobWinTab` window pointers under a contiguous-pack model while the chainer aligns the section; at a base `≢ 0 (mod 8)` every folded pointer lands short and the SFX go silent with no other symptom. | **2** | `games/sonic4/data/sound/sfx_bank_blob.emp`, `ensure((winptr(Sfx_33) & 7) == 0, …)` | **`ensure`** | Nothing directly — but see A4/A5: this `ensure` checks the **placed** base only, and the structural pads it depends on are separately asserted. |
| **A4** | The engine-table head's five embeds must total `≡ 0 (mod 8)` — A3's structural precondition, held by a comptime-sized pad at the head tail. | **2** | `games/sonic4/data/sound/soundbankhead.emp`, `ensure((_sound_tables.len + _pitchtab.len + _sfxwin.len + _seqop.len + _dacsamp.len) % 8 == 0, …)` | **`ensure`** (comptime) | Nothing. |
| **A5** | Each of the five engine-table head embeds must be byte-exact (`$357`, `$108`, `$112`, `$40`, `$7F`) — the resident Z80 driver holds banked carriers at fixed VMAs `$8000/$8357/$845F/$856D/$85AD`, so a size drift slides a downstream head off its carrier VMA and desyncs the blob. | **2** | `games/sonic4/data/sound/soundbankhead.emp`, the five `ensure(_X.len == …)` walls | **`ensure`** (comptime) | Nothing. |
| **A6** | **The `soundbankhead` section is a HARD org at VMA `$8000` and NEVER repacks** — it is a labelled `$8000`-window head whose contents the resident Z80 blob addresses absolutely. | **2** (declaration) + **1** (map) | `games/sonic4/data/sound/soundbankhead.emp` `section soundbankhead (cpu: m68000, vma: $8000)`; `games/sonic4/map.toml` `[[anchor]] name = "sound_bank" … vma = 0x8000` | sigil `native::validate_placement` (phase-bank island must be a declared anchor) + `native::validate_resolved_alignment` (`SoundTablesZ80_Head` declares `0x8000` in `section_align::DECLARED`) | The **LMA** is held by the anchor and its alignment is now declared. What is **not** checked is that the declared `vma` is in window phase with the declared `at` — see **A7**. |
| **A7** | The `sound_bank` anchor's declared `vma` must be congruent to its `at` modulo the `SetBank` window (`vma == winptr(at)`). | **1** (map declaration) | `games/sonic4/map.toml` `[[anchor]] name = "sound_bank" at = 0xB8000 vma = 0x8000` | **nowhere.** `seam2::bank_anchors_from_str` requires the `vma` to be *declared*, then derives every head's VMA as `sound_bank_vma + (lma − sound_tables_z80_lma)`; nothing compares `vma` to `at`. | An anchor pair out of phase yields window pointers the Z80 latch cannot reach, silently — no build symptom, garbled or silent sound at runtime. (This is the prior note's `SOUND_BANK_WINDOW_PHASE`, still unimplemented at sigil master, and the `at` value has moved twice since it was written.) |
| **A8** | The two DAC banks must sit exactly one `$8000` window apart, blip first, so a single Z80 bank id covers each and FILL never re-banks mid-sample. | **2** (declaration) | `games/sonic4/data/sound/dac_banks.emp` — `pub data Dac_Temp_Blip`, then a bare `align $8000`, then `Dac_SharedBank_Start` / `Dac_Kick`, all inside `module games.sonic4.dac_banks in dac_banks` | **partial.** The section HEAD's `0x8000` alignment is now checked (`Dac_Temp_Blip` declares `0x8000` in `section_align::DECLARED`, enforced by `native::validate_resolved_alignment`). The intra-section `align $8000` then holds the shared bank by construction. | The relation now rests on a declared rule rather than on the frozen row — **this is the one the 2026-08-27 note flagged as `SOUND_BANK_ALIGN` and it has since LANDED.** Residual: `dac_banks` still carries **no `bank:` attribute** (it is `module … in dac_banks`, not a `section (… bank: $8000)`), so `relax::bank_diag`'s fit/no-straddle checks never look at it. Its two-window span is deliberate and would trip a naive no-straddle rule, which is why. |
| **A9** | Bank ids must stay **derivable** from the placed LMA (`bankid()` = `(lma & $7F8000) >> 15`, folded at link) — nothing hand-typed. | **1** (map prose) + **2** (usage) | `games/sonic4/map.toml`, BANK PLACEMENT RULE block; `dac_samples.emp` / `dac_sample_tab.emp` fold the per-sample `{bank,ptr,len}` triples through `bankid()`/`winptr()` at emit | **structural** — there is no hand-typed bank id left to drift. Not a check; an absence of the thing a check would guard. | Nothing, as long as no future parcel reintroduces a literal. Worth a lint, not a rule. |
| **A10** | Bank LMA `< 0x800000` (the driver writes the 9th `SetBank` latch bit as a literal `0`, so the id rides in 8 bits) **and** `≤ 0x3F8000` (the cartridge address space ends at `0x3FFFFF`). | **1** (map prose) + **2** (mechanism) | `games/sonic4/map.toml`, "WHAT BOUNDS HOW FAR THE BANKS CAN MOVE"; `engine/sound/z80_sound_driver.emp` `SndDrv_SetBank` | **nowhere as a bank-id bound.** `epilogue.emp`'s `ensure(extern("EndOfRom") <= $3FFFFF)` bounds the *image*, which today implies it — the highest bank is `0xB8000` and the ROM ends under `0x110000`. | If the banks ever move above `0x3F8000` while the image still fits 4 MB (possible: the banks are placed by rule from `packed_data_end`, not by image size), the latch silently truncates. Headroom today is 104 windows, so this is a *stated bound with no instrument*, not a live risk. |

### Class B — region room, ceilings and the ROM image

| # | Constraint | Source | Anchor | Where checked | What lapses |
|---|---|---|---|---|---|
| **B1** | `EndOfRom ≤ $3FFFFF` — the image must fit 4 MB without banking. | **2** | `engine/system/epilogue.emp`, `ensure(extern("EndOfRom") <= $3FFFFF, "ROM exceeds 4MB without banking")` | **`ensure`** (link-time, `LinkAssert`) | Nothing. |
| **B2** | `EndOfRom` must be even — word-aligned tail. | **2** | `engine/system/epilogue.emp`, `ensure((extern("EndOfRom") & 1) == 0, "ROM size is odd")` | **`ensure`** (link-time) | Nothing. |
| **B3** | `EndOfRom` is the LAST placed section, abutting `error_handler`'s tail; the header's `rom_end` and convsym's deb2 appendix both key off it. | **2** (prose in the `.emp` that owns the label) | `engine/system/epilogue.emp` header — *"a zero-length `pub data` label placed LAST … the packer's last section is exactly this one, abutting error_handler's tail"* | sigil `native::check_error_handler_is_last` (called from `append_deb2_appendix`) asserts the appendix begins at `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`, which is the same fact from the other end | Nothing — this is the prior note's R8, already asserted with both directions of membership refusal. **Do not re-rule it.** |
| **B4** | The object bank's used cursor (the resolved LMA of `DeformTable_Zero`, the first section past the bank) must not exceed `0x20000`. | **1** | `games/sonic4/map.toml` `[[budget]] region = "object_bank" ceiling = 0x20000 cursor = "DeformTable_Zero"`; enforced sigil-side by `native::check_object_bank_budget` → `map.check_budget`, and reported build-side by `tools/s4budget.py --map` | **sigil gate** (pack time, on the shipped path) **+ an aeon-side dashboard** | Nothing — the ceiling and the cursor are both *declared in the map*, not read off a frozen row. This one is already decoupled. |
| **B5** | `ObjCodeBase` requires a 64 KB-aligned base (`0x10000` itself is a kept design choice, not a hardware fact) — every object's SST `code_addr` is a 16-bit `label − ObjCodeBase`. | **1** (map anchor) + aeon's R1 ruling | `games/sonic4/map.toml` and `games/demo/map.toml` `[[anchor]] name = "object_bank" at = 0x10000` | **sigil gate** — `section_align::DECLARED` row `d("ObjCodeBase", 0x10000, OBJ_BANK_64K)`, enforced by `native::validate_resolved_alignment` on the shipped path | Nothing. **This is the prior note's `OBJ_BANK_ALIGN` and it has since LANDED** (see §4 — the brief I was given says it has not). |
| **B6** | The free room under the `dac_banks` anchor must stay at or above `DATA_GROWTH_RESERVE`; the anchors are derived, not fixed — `dac_banks = align_up(max over sound-on shapes of packed_data_end + DATA_GROWTH_RESERVE + DATA_GROWTH_GRACE, 0x8000)`, `sound_bank = dac_banks + 2 × 0x8000`. | **1** | `tools/bganim_room.py` constants `DATA_GROWTH_RESERVE = 0xC000`, `DATA_GROWTH_GRACE = 0x8000`, `BANK_ALIGN = 0x8000`, run as `--gate` from `build.sh`; declared in `games/sonic4/map.toml`'s BANK PLACEMENT RULE | **aeon build-side gate**, post-sigil, both canonical shapes, `sonic4` only, **skipped under `FAST=1`** | See **B7** — the gate is real but its *instrument* is an unasserted placement assumption. |
| **B7** | *(not a separate row — the instrument B6 and B8 both rest on)* | | | **see F1 and F2** | **This is the sharpest thing in the list and it is stated once, in Class F.** In short: nothing asserts `Art_Sonic` is the terminus. If placement is re-derived and anything lands between `Art_Sonic` and `Dac_Temp_Blip`, `bganim_room.py` measures a `packed_data_end` that is too low, computes MORE room than exists, and **both** B6 and B8 go green while the reserve is gone. An always-green gate on a real breach — exactly the prior note's R6 shape (*"the gap between two labels is an allotment"*), reappearing in a live build gate instead of in `repin`. |
| **B8** | `BGANIM_SECTION_CEILINGS` — the ruled 12,288 B BG-animation budget, re-derived from each build's listing rather than pinned. | **1** | `tools/bganim_room.py --gate`, invoked from `build.sh` | **aeon build-side gate**; `sonic4` only, skipped under `FAST=1` | It rides B7's instrument, so it inherits B7's failure mode. |
| **B9** | The ROM total must fit its declared `[[region]] rom` size (`0x400000`), cross-checked by `EndOfRom`. | **1** | `games/*/map.toml` `[[region]] name = "rom" size = 0x400000`; `tools/s4budget.py` `_MAX_ROM_ADDR = 0x400000` | **`ensure`** (B1) is the hard one; `s4budget` is the dashboard | Nothing. |

### Class C — DMA / hardware source boundaries ($20000)

| # | Constraint | Source | Anchor | Where checked | What lapses |
|---|---|---|---|---|---|
| **C1** | A DPLC transfer whose ROM source crosses a **128 KB** boundary is split into two DMA queue entries, so a frame costs `entries + straddles` slots — the measured peak slot cost must stay at or under the committed `dplc_peak_entries(_dplc_sonic) + DPLC_ENTRY_RESERVE <= DMA_IMPORTANT_SLOTS` ratchet. **Whether an entry straddles depends on where the art LANDED.** | **1** (the placement half) + **2** (the count half) | `tools/dplc_straddle.py --gate` (its `boundary_from_source()` re-derives the period from `engine/system/dma_queue.emp`'s `blo .split`, rather than typing `0x20000`); the ratchet is READ from `games/sonic4/data/collision/collision_data.emp`'s `ensure(dplc_peak_entries(_dplc_sonic) + DPLC_ENTRY_RESERVE <= DMA_IMPORTANT_SLOTS, …)` | **aeon build-side gate**, post-sigil; `sonic4` only (`demo` has no player), skipped under `FAST=1` | **The constraint is SPLIT across two sources and neither half is sufficient.** The `ensure` bounds the entry *count* and, in build.sh's own words, *"no comptime `ensure` can see"* the straddle — `dplc_peak_entries` parses the blob and never learns the base. The gate covers the placement half. If placement is re-derived, the gate still measures the real ROM, so **C1 survives** — it is the model row for how a placement constraint should be written. Recorded here because it is the only one that is already shaped right. |

### Class D — order, islands and holes

| # | Constraint | Source | Anchor | Where checked | What lapses |
|---|---|---|---|---|---|
| **D1** | Each shape's byte-emitting section order must be a subsequence of the map's declared union `order`, and every byte-emitting section must be declared. | **1** | `games/sonic4/map.toml` / `games/demo/map.toml` `order = [...]` | sigil `native::validate_placement` — `[map.order-undeclared]` completeness + the drive confirmation | Nothing. The map already drives; this is the prior note's R1a/R5 territory and is settled. |
| **D2** | The `ojz_effects_editor_act1` block's `order` row must be keyed by **section name**, not head label — its head label is content-derived (whatever the generator emits first). | **1** | `games/sonic4/map.toml`, the `"section:ojz_effects_editor_act1"` row and its comment | **structural** — sigil's SECTION-ROW spelling resolves the name to the head label at placement | Nothing today. Named because it is a live example of the general rule that **a label row rots when content decides the label**, which is the same defect class as B7. |
| **D3** | The declared `[[hole]]`'s interior — `[end of the section named by `after`, `at`)` — must hold nothing but the module named in `filled_by`. | **1** | `games/sonic4/map.toml` and `games/demo/map.toml` `[[hole]] after = "Z80_IdleProgram" at = 0x3F0 filled_by = "engine.z80_init" when = "sound_off"` | **sigil gate** — `native::hole_interior_faults`, called from `validate_placement` | Nothing for the declared span. But see **D4**. |
| **D4** | **The hole's `at` is shape-varying and the maps declare only the LOWER bound.** `games/demo/map.toml` says so in its own comment: post-hole data resumes at `0x3F0` plain / `0x3F4` debug, *"this row is the LOWER bound across the sound-off shapes"*. | **1** | `games/demo/map.toml`, the `[[hole]] at` comment | **nowhere for the residue.** `hole_interior_faults` reserves `[…, 0x3F0)`; the four bytes `0x3F0..0x3F4` that are still hole in the debug shape are unreserved. | An emitter placed at `0x3F0` passes the hole gate and displaces real boot data in `demo.debug` only. The under-reservation is silent and shape-specific — the direction a single-value declaration cannot express. A per-shape `at`, or an `at` that is the MAX with a `when`, would close it. |
| **D5** | The set of org islands in the resolved layout must be exactly the set of declared anchors, both directions. | **1** | `games/*/map.toml` `[[anchor]]` rows | sigil `native::validate_placement` — `[map.undeclared-island]` / `[map.anchor-absent]` | Nothing. Already asserted and demonstrated. |
| **D6** | An anchor must hold **that particular section** — anchor identity is address-only. | **1** (the declaration carries a `name` that means nothing) | `games/*/map.toml` `[[anchor]] name = …` | **nowhere.** `validate_placement` keys anchors by `a.at` in a `HashMap<u32, &str>`; the name is carried only into diagnostic text. | Prior note's `ANCHOR_BINDS_SECTION`, still unimplemented. A re-layout that lands a different section on an anchor address satisfies every existing check. **The pressure on this rose since the prior note**: `seam2::bank_anchors_from_str` now looks up `dac_banks` and `sound_bank` **by name**, so a name-keyed consumer already exists on the shipped path. |
| **D7** | The error_handler island must be the FINAL byte-emitting section in every shape carrying it — the vendored MDDBG blob locates its deb2 appendix through PC-relative displacements baked into opaque bytes pointing at `ErrorHandlerBlob + $F56`. | **1** (map prose) + **2** (`.emp` header) | `games/sonic4/map.toml`, the INVARIANT block; `engine/debug/error_handler.emp` header WARNING | sigil `native::check_error_handler_is_last` | Nothing. R8 again. |

### Class F — TERMINUS PROXIES: a named label standing in for "the end of the region"

**This is the class the sigil-side inventories could not see, and it is the reason this pass was
owed.** Each row computes a region's high-water mark from *one hardcoded label*, and each then
feeds a gate that passes. Violate the proxy and the number is wrong-but-plausible; the gate goes
green on a real breach.

| # | Proxy | Source | Anchor | Status | What lapses |
|---|---|---|---|---|---|
| **F1** | `Art_Sonic` stands for "the end of the packed data run". | **1** | `tools/bganim_room.py` `LAST_PACKED_LABEL` (a bare module constant), consumed by `rom_room()` | **ASSUMPTION** — nothing verifies it. See B7. | B6 and B8 both pass while the reserve is gone. |
| **F2** | `Art_Sonic`'s ROM extent equals `len(art/optimized/characters/sonic.bin)` exactly — i.e. its section holds exactly one embed and nothing (pad, trailing content) follows it inside the section. | **1** | `tools/bganim_room.py` `rom_room()` (`end = LMA + blob_len`), length via `art_sonic_bytes()` parsing `const _art_sonic = embed(...)` out of `collision_data.emp` | **ASSUMPTION** | Understates `packed_end` by the pad; same green-on-breach as F1. `map.toml` concedes the fragility in its own words — *"a section with several embeds has no such instrument"* — which is why the character-data sections were ordered before `collision_data`. |
| **F3** | `DeformTable_Zero` stands for "the object bank's high-water mark" — the bank and the data region pack contiguously, so the head of the first section *past* the bank is the bank terminus. | **1** | `games/sonic4/map.toml` `[[budget]] cursor`, consumed by `tools/s4budget.py` `resolve_budgets()` (`used = addr − region.lma_base`) | **ASSUMPTION**, and an explicit one — the map comment names its ancestry: this proxy replaced the AS-era `if * > $20000 / error` guard and the retired `__BUDGET_DATA` sentinel. A real terminus was traded for a proxy label. | If the bank and data stop packing contiguously, or any bank content lands after the cursor, `used` is wrong and the `$20000` ceiling is silently unenforced. **Note this is the same value B4 reports as already-decoupled: the *ceiling* is declared, the *measurement of what is under it* is not.** |
| **F4** | `EndOfRom` equals the ROM file size. | **1** | `tools/s4budget.py` `format_rom_report()` | **ASSUMPTION, and structurally un-failable** — the disagreement is printed as `NOTE: EndOfRom and the ROM file differ by …` and is **never appended to `breaches`**, so it cannot fail a build. Verified firsthand: the note is emitted in `format_rom_report`; `breaches` is built separately and takes only the ROM-limit and budget rows. | Padding, a stale file, and a real placement error read identically. |
| **F5** | The three sound-bank art regions' extents equal their embed lengths (`Art_Sonic`, `Art_Tails`, `Art_TailsAppendage`, `Art_Knuckles`). | **1** | `tools/dplc_straddle.py` `SUBJECTS` / `load_subjects()` | **ASSUMPTION** for the extent (a missing label is a loud `Unmeasurable`) | Feeds the straddle calculation C1, so a pad silently shifts which frames are judged to cross a boundary. |
| **F6** | `sound_bank == dac_banks + 0x10000`. | **1** | `tools/bganim_room.py` `SOUND_BANK_OFFSET = 2 * BANK_ALIGN` | **ASSUMPTION, and not even that** — verified firsthand: the constant appears at its definition and **twice inside one f-string in the failure message**, and nowhere else. There is no comparison against the declared `sound_bank` anchor. | The two anchors can drift apart with nothing on aeon's side noticing. The relation is stated in `map.toml`'s BANK PLACEMENT RULE and asserted by no one. |
| **F7** | Growth in `ojz_bg_anim` shifts the whole run `Map_TestObj .. Art_Sonic` downstream into the room under `dac_banks`. | **1** | docstrings of `tools/bganim_room.py` ("WHAT LIMITS THE SECTION") and `tools/inject_editor_bg.py` | **ASSUMPTION** — the ordering premise that makes the ceiling arithmetic mean anything | If the section stops being upstream of `Art_Sonic`, or a gap absorbs its growth, B8 measures the wrong room. |

### Class G — ADJACENCY PINS: a hardcoded neighbour name standing in for "the next thing emitted"

Seven instances across five gates. Each derives a symbol's **size** from the *address of a
named neighbour*, which is an assertion about emission order that nothing verifies. All are
source **1**, all **ASSUMPTION**, all post-sigil, all `sonic4`-only, all skipped under `FAST=1`.

| # | Pinned adjacency | Anchor |
|---|---|---|
| **G1** | `ObjDef_Static` immediately follows `OJZ_Reels_Fill`; `OJZ_Reel_Speed` immediately precedes it. | `tools/reels_gate.py` `NEXT_SYM` / `SPEED_SYM` / `FILL_SYM`. Its own comment grounds the claim in `map.toml`'s `order`, and records that it *"measured red the first time this file was drafted"* when a block landed between two symbols another gate depended on. |
| **G2** | `OJZ_TestPal` immediately follows `OJZ_BaseSwap`. | `tools/plane_base_swap_gate.py` `SYM` / `NEXT_SYM`. Its comment explicitly **rejects** "the next label in address order" as a derivation and pins the name instead. |
| **G3** | `Parallax_Update` immediately follows the routine under test. | `tools/plane_role_swap_gate.py` `NEXT_SYM` |
| **G4** | `ParallaxConfig_OJZ_Default` → `ParallaxConfig_OJZ_Underwater` → `DeformTable_OJZ_Calm` are three consecutively emitted symbols, in that order. | `tools/band_drift_golden.py` `CFG_SYM` / `NEXT_SYM` / `CHECK_SYM` — `sizeof(parallax_config)` is **derived** from `(nxt − cfg) − count*stride`. Partially self-defending: two independent pairs must agree, so a re-order usually surfaces as a loud `Unmeasurable`. **Only usually** — a re-derivation moving all three by equal amounts agrees and is wrong. |
| **G5** | A routine's ROM extent is `[LMA, next non-local symbol above it)`. | `routine_extent()` in `tools/instashield_gate.py`, `tools/sprite_tilt_gate.py`, `tools/loop_crossover_gate.py`. **Its violation is LOUD but MISATTRIBUTED**: alignment inserted by a re-derivation makes capstone decode fill bytes, and these gates' own comments describe the symptom (*"the extent is not a clean instruction run"*, *"execution left the extent"*) as a code defect, not a layout change. |

### Class H — declared on aeon's side, consumed by nothing on aeon's side

**This is the structural finding, and it is what makes the precondition answerable.** Verified
firsthand: `grep -rnE '\["order"\]|get\("order"' tools/*.py build.sh` returns **zero hits**.

| # | Declaration | Read by any aeon tool? | Enforced where |
|---|---|---|---|
| **H1** | `map.toml`'s root `order` array | **no** | sigil `native::validate_placement` only |
| **H2** | `[[hole]]` after `Z80_IdleProgram` | **no** | sigil `native::hole_interior_faults` only |
| **H3** | the `object_bank` (`0x10000`) and `boot_head` (`0x0`) anchors | **no** — `bganim_room.anchor_addr()` takes a `name` defaulting to `"dac_banks"` and is only ever called with the default | sigil `validate_placement` + `validate_resolved_alignment` |
| **H4** | the `sound_bank` anchor | **no** (F6: named only in a message string) | sigil `validate_placement` + `validate_resolved_alignment`; its `vma`↔`at` phase: **nowhere** (A7) |
| **H5** | the fault-handler-island-is-last INVARIANT block | **no** — grep of `tools/` for `deb2`/`convsym`/`EndOfRom` finds no consumer | sigil `native::check_error_handler_is_last` only (verified firsthand at sigil master) |
| **H6** | the `z80_sound_bank` region (`lma_base = 0xB8000`, `vma_base = 0x8000`) | **no** | **nothing** — the map marks it DESCRIPTIVE ONLY and records that it sat wrong (mid-`Map_Tails`) for months with nothing noticing. Not a constraint today; named because **retiring the frozen tables is exactly when a descriptive-only region becomes either load-bearing or a lie again.** |

**Seven aeon-side assumptions (F1, F3, F7, G1–G4) cite `order` as their justification. Nothing
on aeon's side reads it.** Today that is harmless because the frozen tables hold the order still;
the moment they stop, the justification and the mechanism are in different repos with no link.

### Class P — from the history walk

The headline for source 3 is a **negative with a caveat**: across the whole reachable ancestry
(4,434 commits) and ~45 query shapes, **nothing was found whose only home is a commit message.**
That is a real property of this tree, not an empty result — aeon's K1 parcel
(`a7375682`, *"K1: the declared placement maps"*, 2026-08-01) deliberately moved the implicit
placement contract into declared files, and the sweep confirms it took. Every rule the history
states is also in `map.toml`, an `ensure`, a gate tool, or a `.emp` header. Coverage limits in §2.

Two rows survive anyway, in the weaker but still-fatal sense: stated in a doc plus its commit,
executed by nothing.

| # | Constraint | Source | Anchor | Where checked | What lapses |
|---|---|---|---|---|---|
| **P1** | **Something in the sound region sits hard against ROM `0x8000`, the 68000 `abs.w` ceiling.** A parcel that *shrinks* the image upstream drags it below the line, every absolute reference in that neighbourhood re-encodes `abs.l`→`abs.w`, and lengths cascade — presenting as a test failure in a region nobody edited. **Growth is safe; shrink is dangerous.** | **3** | commit `9c8117ed` *"book: the sound tables sit exactly on the abs.w ceiling, and a one-byte shrink is enough"*; text lives in `docs/DEFERRED_WORK.md` §*"A LANDING HAZARD FOR EVERY BYTE-MOVER…"* | **nowhere.** No gate, no `ensure`, no `map.toml` row, no tool reads it. Its own stated "durable half" is a `grep` a human is asked to run by hand. | Today the frozen per-label LMAs are the only thing holding `Sound_PlaySFX` and its neighbourhood on their side of `0x8000`. Retire them, let those bases float, and one shrink upstream flips 14 real transfer sites' encoding with no diagnostic. **Direct hit on the precondition.** |
| **P1-b** | ⚠ **P1's prescribed check is VACUOUS, and its binding symbol is misidentified.** Verified firsthand. | **3** | same booking | — | The booking says *"`SoundTablesZ80_Head` at `0x8000` exactly — margin ZERO … A shrink of ONE byte upstream is enough"* and prescribes *"`SoundTablesZ80_Head` at `8000` or above = clear."* **`SoundTablesZ80_Head` is a PHASED section head** — `soundbankhead.emp` declares `section soundbankhead (cpu: m68000, vma: $8000)`, and `map.toml`'s own anchor row reads `at = 0xB8000, vma = 0x8000`. So `0x8000` is its declared **VMA**, not its ROM LMA; **no upstream shrink can move it and the prescribed grep always prints `8000`.** The symbol with a real ROM address is `Sound_PlaySFX` (`engine/sound/sound_api.emp`, `module engine.sound_api in sound_api` — **no `vma:`**), at `0x8024`: **the margin is 36 bytes, not zero.** This is the exact class aeon itself named four days later in `9d7cefd5` *"merge: a phased section's VMA was being read as a ROM address"*, which removed a by-name allowlist that had been patching the same confusion one symbol at a time. **Verified the booking stands unamended at the pin**: `git log -S` on its heading returns only `9c8117ed`, so the 09-03 correction was never back-applied. **Take P1's shape; re-derive its coordinate from a non-phased symbol.** |
| **P2** | Only **DATA** tables may live in the Z80 `$8000` banked window; **all in-frame code must be resident**, because a banked opcode fetch traverses the 68k bus and corrupts under DMA/BUSREQ contention. | **3** | commit `3b186623` *"fix(sound): relocate Fm_FnumApplyDelta resident; delete banked-code file (T0.3)"* | **prose only**, but in four live `.emp` headers (`z80_sound_driver.emp`, `sound_sequencer.emp`, `sound_fm.emp`, `seq_opcode_tab.emp`) | **A near-miss worth its own line.** The commit says the rule was written into `main.asm`'s window block. **`main.asm` was deleted 2026-08-01 (`f7405d63`).** The rule survived only because someone had also copied it into the `.emp` headers — nothing structural preserved it. That is the failure mode this whole inventory exists to prevent, and it already happened once and was caught by luck. |
| **P3** | A page-aligned optimisation in the Z80 DAC lookup was deliberately **NOT TAKEN** because `ensure((DacSampleTable & $FF) == 0)` would fail — the table sits at `$85AD`, after the other engine-head tables, not at the `$8000` window head. | **3** (booked in `e4e173eb` *"book: two placement findings from sigil's alignment parcel"*) | verified firsthand: the `// NOT TAKEN —` block in `engine/sound/z80_sound_driver.emp` | **n/a — it is a constraint that was declined rather than incurred** | **This is the named counterexample to source 2's completeness, and it is worth more to the decouple lane than most rows above.** It is a placement requirement that **an `ensure` sweep cannot find, because it has no comptime wall to find**: the requirement exists only as the reason a faster form was rejected. If someone later takes the optimisation without writing the ensure alongside it, a silent alignment requirement is created that no source enumerates. |
| **P4** | The replay fixture is placed after **all** gameplay content so a re-record shifts zero gameplay addresses; `EndOfRom` is named last so the terminus encloses it. | **3** | `a74b7427`, `806bc2de` | also in `games/sonic4/test/replay_fixture.emp` header + the `map.toml` order tail; enforced by D1/D7 | Nothing beyond D1/D7. |
| **P5** | The parallax/scene block's **emission order is load-bearing** — the shipped block interleaves the six deform tables with the twenty records that attach them; grouping them elsewhere rewrites the table pointer inside all twenty. | **3** | `d673bad0`, `0634e79f` | also in `map.toml`'s `DeformTable_Zero` comment, `scene_registry.emp`, `ojz_scenes.emp`; enforced by D1 (the map drives order) | Nothing today. Named because it is a case where `order` is load-bearing for *content correctness*, not just for a proxy's arithmetic — so D1's enforcement is doing real work here, not bookkeeping. |
| **P6** | **How constrained the packer actually is, measured.** Sweeping `Art_Sonic`'s base one byte at a time across ±64 KB: **2,773 of 131,073 shifts FAIL, in 43 forbidden bands**, worst peak 17 slots. Current margin **5,188 B**. An append is *not* tail-only — `DPLC_Sonic` sits immediately before `Art_Sonic`, so a shrink moves the art base and every DMA source with it. Two parcels ruled independent are **coupled**: both land safe only inside the combined band `[−29,796, −15,300]`. | **3** | `15cb42f7` *"measure: the d-47 append condition, discharged with an instrument"*, corrected by `9f82779a` *"correct: three subjects' art straddles, not one"* | `tools/dplc_straddle.py --gate` (C1) is the instrument and it is on the build path | Nothing lapses — but **this is the number §5 turns on**, and its commit states the source-2 finding independently and in aeon's own words: *"`dplc_peak_entries` parses the blob and never learns the base address, so **every ensure in this tree is structurally blind to it**."* |

**Also confirmed retired rather than missing** (so a future reader does not go looking): the
`SIGIL_EMP_*` mixed-build resume-org constraints (`ae1de4d1` + ~30 siblings) died with
`main.asm`/`engine.inc` on 2026-08-01 (`f7405d63`); the 64 KB object-bank overflow guard
`if * > $20000 / error` migrated to the map `[[budget]]` in `8fb3a85f` (it is B4/F3 now).

### Class E — the ROM-only-answerable gates that a re-derivation must keep working

These are not placement *constraints*; they are placement-*dependent instruments*. They are here
because retiring the tables changes what they measure, and a gate that quietly measures the
wrong thing is worse than no gate.

| # | Instrument | Source | Anchor | Assumption it rests on |
|---|---|---|---|---|
| **E1** | `tools/bganim_room.py` | 1 | its `--lst` / `--rom` / `--built-after` / `--fixture` arguments | B7: `Art_Sonic` is the terminus and single-embed. Also `--fixture` pins a committed listing excerpt, so an emitter format change is a named staleness failure rather than a silent green. |
| **E2** | `tools/s4budget.py` | 1 | `parse_listing`'s `ListingFormatError` | The listing format. Its docstring records the prior defect: a dead parser reported `RAM: 0KB/64KB (0%)` for a long time. It now validates and refuses; UNMEASURED is never rendered as a number. |
| **E3** | `tools/dplc_straddle.py` | 1 | `boundary_from_source()` | That `dma_queue.emp` still spells the split with `blo .split`. Deliberately derives rather than pins — the pattern the other tools should follow. |
| **E4** | the listing-presence guard in `build.sh` | 1 | the `[[ "$FAST" == "0" && ! -f "${ROM_NAME}.lst" ]]` refusal | Named as a build bug, with *"Do not convert this to a skip"* in the source. It exists because a `-f` guard once made a missing listing a silent skip of `s4budget`, the seam gate and the BG-anim ceiling together. |
| **E5** | every ROM-reading gate's `rom[sym & 0xFFFFFF]` | 1 | 43 files under `tools/` spell `& 0xFFFFFF` (measured); the gates among them are `row_remap_gate.py`, `sprite_tilt_gate.py`, `instashield_gate.py`, `loop_crossover_gate.py`, `reels_gate.py`, `band_drift_golden.py`, `editor_palette_golden.py`, `plane_base_swap_gate.py`, `waterline_art_gate.py` | **A symbol's LMA masked to 24 bits is its byte offset in the ROM file.** The mask itself is correct — the 68000 address bus is 24 bits — so this is *not* a defect. It is a **latent** assumption: it is already conditionally false in this tree, because `map.toml`'s `sound_bank` anchor declares `vma = 0x8000` ≠ its LMA. No gate reads a banked symbol today, so it holds by luck of subject choice rather than by construction. Recorded so a future gate over a banked symbol does not inherit it silently. |
| **E6** | `tools/gen_sound_tables.py` `emit_emp_z80()`, `tools/song_packer.py` | 1, **off the build path** | the emitted `section sound_tables_z80 (cpu: z80, vma: $8000)` and its *"the byte-exact $8000-based layout is load-bearing"*; `song_packer`'s 16-bit stream offsets the loader adds the `$8000`-window pointer to | The Z80 bank's one-window co-residency (A1/A2's runtime premise). These are **authoring-time generators whose output is committed** — `build.sh` never re-runs them, so nothing on the build path re-checks that the bank still fits one window or that `bankid()` folding still lands. The `ensure`s A1–A5 are what catch it, which is the right place; noted so nobody mistakes the generators for a gate. |

**Read and genuinely clean of ROM-placement constraints** (stated rather than omitted, because
"I found nothing" and "I could not look" are different sentences): `art_rom_report.py` — a byte
*footprint* budget (`ART_ROM_SOFT_KB`/`ART_ROM_HARD_KB`), no address ever appears;
`effects_budget_check.py` — `.emp` constants against `effects_budget_model.toml`, cycle budgets;
`effects_seam_gate.py` — uses the listing for symbol *presence* as reachability evidence, never
for addresses; `verify_level_bin.py` — referential integrity of the generated tree, and its
`(align: N)` appears only in a regex that *tolerates* the annotation, never parses it;
`level_staleness.py` — mtime + sha256, no addresses at all; `build.sh` itself — `stat`s the ROM
and prints a percentage of 4 MB with no comparison. Also read and clean: `dac_encode.py`,
`dac_verify.py`, `sfx_transcode.py`, `dplc_layout.py`, `vram_map.py` (format/encode/VRAM tools),
and `dma_straddle_reading.py` (an emulator-driven measurement *session*, not a gate — it records
the reading that only Knuckles straddles, one boundary at `0x60000`, frame `$88`).

---

## §2 — Coverage, per source, with the positive control for each

Aeon will confirm this list and needs to check coverage **per source**, because a list complete
against one source and silent about the other two reads identically to a complete one.

### Source 1 — `build.sh` and the tools it invokes

**Rows from this source: 24** (B4, B6–B9, C1, D1–D7, E1–E6, F1–F7, G1–G5, H1–H6 — counting each
table row once; the classes overlap where a constraint is declared in the map and checked by a
tool).

**Population, measured not assumed.** `grep -oE '\$\{TOOLS\}/[a-z_0-9]+\.(py|sh)' build.sh | sort -u`
→ 24 tools, plus `tools/regenerate-level.sh` (FAST / `STRESS_ART` lanes only) and
`games/<game>/prebuild.sh` (a documented no-op for sonic4). Eight further address-arithmetic
tools were skimmed and confirmed **not** on the build path (zero hits in `build.sh` and
`regenerate-level.sh`).

**Positive control: PASSED, and the numbers match.** Target chosen before looking: `bganim_room.py --gate`
enforcing that free room under the `dac_banks` anchor stays at or above `DATA_GROWTH_RESERVE`,
with `packed_data_end` derived as `LMA(Art_Sonic) + len(sonic.bin)`. Re-found in the tool's own
code: `rom_room()` computes `packed_end = labels[LAST_PACKED_LABEL] + blob_len`;
`art_sonic_bytes()` parses `const _art_sonic = embed("art/optimized/characters/sonic.bin")` out
of `collision_data.emp` and `getsize`s it (101,056 B present); the gate arm in `report()`
compares the declared anchor against `rule_anchor(packed_end)` =
`align_up(packed_end + DATA_GROWTH_RESERVE + DATA_GROWTH_GRACE, BANK_ALIGN)` with
`DATA_GROWTH_RESERVE = 0xC000`, `DATA_GROWTH_GRACE = 0x8000`, `BANK_ALIGN = 0x8000`. Those match
`map.toml`'s own worked derivation exactly: `align_up(0x8C01E + 0xC000 + 0x8000) = 0xA8000`,
which is the declared `[[anchor]] dac_banks at = 0xA8000`. **The method reaches the build side
and the arithmetic reconciles across two independently-maintained artifacts.**

**A second, sharper control on the same source.** Aeon's own plan doc
(`docs/superpowers/plans/2026-08-27-sigil-decouple-steps-1-4.md`) states the enumeration it did
from this source **and its axis**: *"searched `room|reserve|guarantee|_CEILING` across `tools/`,
`map.toml`, `engine/` and `config/`. That is ONE spelling axis — a guarantee phrased in none of
those words would not appear. Three found and all enforced, NOT proof there are exactly three.
Given the day's record, treat it as a floor."* This pass ran a **different axis** — hardcoded
address constants (`grep -rnE '^[A-Z_]+ *= *0x[0-9A-Fa-f]{4,}' tools/*.py`), adjacency-pin
spellings (`NEXT_SYM`, `routine_extent`), terminus proxies, and `rom[… & 0xFFFFFF]` — and it
recovers all three of aeon's rows **and** classes F, G and H, which the word axis could not
reach because none of them is spelled `room`, `reserve`, `guarantee` or `_CEILING`. **Aeon's own
floor was correct and its stated limit was the right limit.**

### Source 2 — `ensure(...)` inside `.emp` sources

**Rows from this source: 8** (A1–A5, A8's declaration half, B1, B2).

**Population, measured.** `1,324` `ensure(` / `ensure_fatal(` call sites tree-wide; `1,246`
outside `games/sonic4/test/poison/`. Of these, **exactly 7 constrain ROM placement**: the six
`bankid`/`winptr` bank walls (A1×5, A2, A3 — seven statements across three files) and the two
`extern("EndOfRom")` walls (B1, B2). Every other `ensure` in the tree is a RAM-layout, Z80-RAM,
VRAM, struct-offset, opcode-encoding, cycle-count or gameplay-constant fact.

**That number is itself a finding.** `build.sh`'s own comment states the mechanism —
*"a link-time fact no comptime `ensure` can see, because `dplc_peak_entries` parses the blob and
never learns the base"* — and the seven exceptions are exactly the statements that reach a
placed address through a link-deferred builtin (`bankid`, `winptr`, `extern`), which sigil defers
to a `LinkAssert` (`sigil-link/src/lib.rs`, D-H.4). **Source 2 is a narrow but perfectly
trustworthy source: every constraint it carries is evaluated against the placed address, so
every one of them survives decouple untouched.**

**Positive control: PASSED with a measured method failure, which is the useful half.** Two
independent queries were run over the population:

- **Q1** (placement-term filter: `bankid|winptr|span|align|$8000|$20000|straddle|boundary|window|lma`)
  → found A1–A5 and A8's neighbourhood. **It missed B1 and B2.**
- **Q2** (a differently-shaped blind query: identifiers containing `BASE|ADDR|LMA|BANK|ALIGN|ORG`,
  plus `align|straddl|boundar|contiguous|adjacen|EndOfRom|_END|LAST|precede|follow`)
  → found B1 and B2, the `EndOfRom` walls.

**Q1 alone would have under-counted this source by 2 of 7 — 29%.** So a third query shape could
find more, and this row of the coverage table should be read as a floor, not a total. Reported
because the brief asked for a control's *result*, and a control that only ever confirms is not
an instrument.

**A control whose target was named by an outside source before I looked.** Sigil's own
2026-08-27 note says commit `2c49f538` *"invalidated aeon's mod-8 pads"* — a claim about a repo
this pass enumerated independently. The sweep surfaced **both** pads without being pointed at
them: `ensure((winptr(Sfx_33) & 7) == 0, …)` in `sfx_bank_blob.emp` and
`ensure((… + _dacsamp.len) % 8 == 0, …)` in `soundbankhead.emp`. That control could have failed
and did not.

### Source 3 — commit prose in aeon's history

**Rows from this source: 7** (P1, P1-b, P2, P3, P4, P5, P6).

**Range walked, and it is the whole ancestry.** `git log --since=2026-06-01` = **4,005**
commits; `git log --until=2026-06-01` = **429** (the repo starts 2026-04-24). Total reachable
from the pin: **4,434**, walked in two passes over **bodies as well as subjects**, merge commits
included. Plus `git log --follow` on `games/sonic4/map.toml` (52 commits) and
`games/demo/map.toml` (17). ~45 distinct query shapes.

**Positive control: PASSED.** Target chosen before looking: the BANK PLACEMENT RULE introduced
by the 2026-08-26 and 2026-09-04 re-layouts. Three independent queries (`--grep='BANK PLACEMENT RULE'`,
`--grep='align_up'`, `--grep='DATA_GROWTH_RESERVE'`) each surfaced it, at
`446a27d9` *"relayout(rom): the banks move to 0xA8000/0xB8000, the reserve triples, and the rule
grows the term that makes 'extra room' a guarantee"*, `0cddcaa9` (the 08-26 predecessor at
`0x90000`/`0xA0000`) and `dddfbf0a` (*"bganim_room enforces the bank placement rule"*).

**The result, in the sentence that means what it says.** *Searched 4,434 commits with ~45 query
shapes and found nothing whose only home is a commit message.* That is **not** "there is nothing
of this shape". The residual risk is concentrated in two places, both stated:

- **High-population terms were narrowed, not exhausted** — `boundary` (271 hits), `reserve`
  (364), `placement` (232), `align` (163), `load-bearing` (132), `headroom` (103). A constraint
  buried in one of those and phrased outside the narrowing patterns would be missed.
- **A rule stated as a bare address relation with no relational word** (*"`X` at `$5BB10`, `Y`
  right after"*) matches none of the ~45 vocabulary terms and would not surface. Three long-context
  regexes hit `ugrep` complexity limits and had to be simplified, costing some recall.
- Unmerged branch tips were not swept. `--grep` over `--all` for the three control queries
  returned nothing outside the pin's ancestry, but that is three queries, not the sweep.

**The interesting part of a null result is what it implies about the other sources.** Source 3
being empty of *sole-home* constraints is aeon's K1 parcel working as designed. But the walk
surfaced **P3** — a placement requirement whose only trace is *the comment explaining why a
faster form was rejected* — which is a shape **none of the three sources can enumerate**: it has
no gate, no `ensure`, and no commit that states it as a rule. That is source 3's real yield.

## §3 — What this did NOT cover

Stated rather than implied, in the prior note's spirit, and **none of it should be read as
clean**.

- **No ROM was built and no tool was run.** There is no listing in the detached pin worktree, so
  every source-1 row is confirmed from the tool's code and from `map.toml`'s committed worked
  example, never from a live measurement. B6/B8/C1's *firing* is unexercised here.
- **`scripts/landing-run.sh` was not run.** This branch's whole diff is one new markdown file
  under `docs/superpowers/notes/`; nothing under `crates/` was touched, so there is no path by
  which it could move. That is a reason not to spend the run, not evidence that it is green.
- **Not read in full, only docstring + targeted grep:** `s4lint.py` (2,000+ lines),
  `effects_gen.py` (3,000+ lines), `collision_consistency.py`, `emp_expect_fail.py`,
  `gen_compression_vectors.py`, `suite_paths.py`, `editor_palette_golden.py`,
  `row_remap_gate.py`, `waterline_art_gate.py`, `sprite_tilt_gate.py`, `instashield_gate.py`,
  `loop_crossover_gate.py`, `plane_role_swap_gate.py`. The grep axes were address arithmetic,
  `NEXT_SYM`/adjacency, `align`, boundary constants and `rom[…]` indexing — which is how G1–G5
  were found, so **a full read would very likely surface more of that class.**
- **The pytest lane was not enumerated.** `python3 -m pytest ${TOOLS}` sweeps `tools/test_*.py`
  (skipped under `FAST=1` and when pytest is absent); what placement facts those pin is unmeasured.
  `tools/test_bg_emit.py` at minimum asserts on `rom_room()`'s outputs.
- **The off-canonical shapes were not read shape-by-shape.** They did not need separate map
  reads — verified firsthand that `GameProfile::map_path` derives the map from `game_root_rel`'s
  parent, so `s4`, `s4_debug`, `config_a`, `config_b` and `lean` all read
  `games/sonic4/map.toml` and only `demo`/`demo_debug` read `games/demo/map.toml`. **Both maps
  were read in full at the pin** — which is a coverage improvement on the prior note, and is not
  the same as checking what each shape *places*.
- **`when`-gated facts were reasoned about, not exercised.** The four sound-off arms and the
  `sonic4`-only / `DEBUG`-only gate conditions were read out of `build.sh`'s nesting
  structurally, not traced line-by-line through all 1,162 lines.
- **Sigil-side claims are firsthand where stated as such.** `check_error_handler_is_last`,
  `validate_placement`, `hole_interior_faults`, `validate_resolved_alignment`,
  `section_align::DECLARED`, `check_object_bank_budget` and `frozen_sizes`' doc were read
  directly at sigil master `311ded5a`. Nothing about sigil in this note is relayed.
- **`docs/DEFERRED_WORK.md` (17,500+ lines) was searched, not read.** Only the entries the
  history walk's commit hits pointed at were opened. P1 was found that way; **there may be
  further prose-only placement rules booked in that file that no commit message names.** Given
  P1 is one of exactly two prose-only rows and it was found by following a commit, a direct
  sweep of that file is a cheap and probably productive next query.
- **Unmerged branch tips were not swept.** The history walk covered the pin's ancestry;
  `--grep` over `--all` for the three control queries returned nothing outside it, which is
  three queries, not a sweep.
- **`AEON_DIR = /home/volence/sonic_hacks/.aeon-verify-483` was not used at all.** It is 164
  commits behind the pin, and no fact in this note comes from it. Stated because the brief
  offered it and a reader will want to know which tree each fact came from: **every aeon fact
  here comes from a detached worktree at `9e3d2861`, and nothing else.** That worktree was
  created for this pass and **removed afterwards** — reproduce it with
  `git -C /home/volence/sonic_hacks/aeon worktree add --detach <path> 9e3d2861`. It is not a
  tree anyone should go looking for.

## §4 — Where the brief this parcel was dispatched with was wrong

Four items. None of them changes the parcel's shape; two change its ledger.

### 4.1 — Two of the five "not implemented" predicates HAVE landed, so the residue is three, not five

The brief says `HOLE_INTERIOR_RESERVED` and `SECTION_ALIGN_DECLARED` are the two predicates from
the 2026-08-26 inventory that are implemented in sigil, and that *"the other five it derived
(`ANCHOR_BINDS_SECTION`, `OBJ_BANK_ALIGN`, `SOUND_BANK_ALIGN`, `SOUND_BANK_WINDOW_PHASE`,
`REGION_END_IS_OWN_SECTION`) are not."* It also said to verify rather than trust, and that is
what caught this.

**`OBJ_BANK_ALIGN` and `SOUND_BANK_ALIGN` are implemented**, and by the same landing that
implemented `SECTION_ALIGN_DECLARED`. Verified firsthand at sigil master `311ded5a`:
`crates/sigil-harness/src/section_align.rs` holds a 107-row `DECLARED` table keyed on head
label, of which five rows require more than the `WORD` baseline —

```
d("Dac_Temp_Blip",       0x8000,  Z80_BANK_WINDOW)
d("SoundTablesZ80_Head", 0x8000,  Z80_BANK_WINDOW)
d("Sfx_33",              8,       SFX_MOD8)
d("Song_MovingTrucks",   8,       MT_MOD8)
d("ObjCodeBase",         0x10000, OBJ_BANK_64K)
```

— and `native::validate_resolved_alignment` asserts `required` divides every section's
**resolved** LMA, called from `build_rom_chained_with_listing` on the shipped path (alongside
`validate_placement`, `validate_sound_fold` and `check_object_bank_budget`). The module's own
doc says the anchor rows exist precisely so this pass *"measures the anchors against what the
sections actually need."* `native::packed_align_of` — the residue-of-address inference the
2026-08-27 note's R7 was about — **no longer exists**; the walk reads `required_for(head_label)`.
Sigil's provenance ledger dates the flip to chain 196.

So: **four of the seven old predicates are now in, three remain** — `ANCHOR_BINDS_SECTION`,
`SOUND_BANK_WINDOW_PHASE`, `REGION_END_IS_OWN_SECTION`. Rows D6, A7 and (in its build-side
reincarnation) F1/F3 are their live instances.

Two further notes on the same landing, because they matter to this list. First, `Sfx_33` and
`Song_MovingTrucks` are declared at 8 with named reasons — the sigil-side counterpart of aeon's
A3/A4 mod-8 `ensure`s, so that requirement is now stated on both sides of the seam. Second, the
predicate **names** in the brief appear nowhere in sigil's crates; they are the prior note's
proposed spellings, and `hole_interior_faults` / `section_align::DECLARED` are the
implementations. Searching for the names finds nothing and reads as "not implemented".

### 4.2 — The three-source population omits the source that carries the most unrecaptured constraint

The stated population is `build.sh` + its tools, `.emp` `ensure`s, and commit prose. **The
densest single source of stated-but-unenforced placement constraint in aeon is
`games/sonic4/map.toml`'s PROSE**, which is none of the three: not `build.sh`, not a tool, not
an `ensure`, not a commit message. It carries the BANK PLACEMENT RULE and its derivation, the
`DATA_GROWTH_RESERVE`/`GRACE` reasoning, the fault-handler-island-is-last INVARIANT, the
`[[budget]] cursor` rationale, the `Art_Sonic`-is-the-last-packed-blob statement, and the
bank-id upper bounds (A9, A10, B7, F1–F3, F6, F7, D7 all trace back to it).

It reached this list only because the tools read the *tables* in that file, so I read the file.
That is an accident of my route, not a property of the stated population. The brief is
internally ambiguous here — it names the prior note's "only read `games/sonic4/map.toml`" as a
*limit*, which implies the map is in scope, while the numbered list excludes it — and a stricter
reading by the next agent would drop those rows. **If this list is re-run or extended, name
`games/*/map.toml` prose as a fourth source explicitly.**

### 4.3 — "Don't re-enumerate sigil" is right for the rows and wrong for the WHERE-CHECKED column

The brief's reasoning — *"two enumerations sharing a parameter are one enumeration run twice"* —
is correct about which constraints exist. But the deliverable also asks **"whether a check exists
today, and WHERE"**, and that column cannot be answered from aeon at all. Taken literally, the
instruction would have produced a list whose where-checked column was wrong on four rows (A8,
B5, and the two alignment predicates in 4.1), because the prior note's account of sigil is a
snapshot that has since moved.

The general form, which is the useful half: **the ledger of what is already CHECKED ages faster
than the ledger of what EXISTS.** A constraint's existence changes when someone writes a new
rule; its enforcement changes every time either side lands a gate. So a "don't re-do the other
side" rule needs a standing exception for the enforcement column, and this parcel needed roughly
an hour of sigil reading it was told it did not need.

### 4.4 — The staleness figure is understated, in the direction that strengthens the brief

The brief says the prior note's map read was from *"a tree 814 commits behind current aeon
master."* Measured at the pin: `git rev-list --count 9bba8700..9e3d2861` = **978**. The
companion figure is exact — `games/sonic4/map.toml` changed **four** times in that range
(`af4f5098`, `4c62294e`, `446a27d9`, `5875e60e`), and `games/demo/map.toml` twice, which the
prior note never read at all.

Flagged rather than silently corrected because the error is in the direction that never gets
caught: a stale-basis argument overstated is challenged, understated is banked. The conclusion
is unchanged and stronger than stated.

## §5 — Counts, and where the checks live

**45 distinct constraint rows** (Class E's six are instruments, not constraints, and are excluded;
H1–H5 are a cross-cutting view of rows counted elsewhere; P1-b is a defect *on* P1, not a row).

| | count |
|---|---|
| Rows with a check that **fails the build** | **22** |
| Rows that are structural, declined, or dashboard-only-but-covered-elsewhere (A9, B9, D2, P3) | 4 |
| **Rows with NO check anywhere** | **19** |

**Where the 22 checks live — this distribution is the finding.**

| Enforcer | rows | which |
|---|---|---|
| an `ensure` in aeon's own `.emp`, deferred to a `LinkAssert` and evaluated by **sigil's linker** | 7 | A1–A5, B1, B2 |
| a **sigil** gate on the shipped build path | 9 | A6, A8, B3, B4, B5, D1, D3, D5, D7 |
| an **aeon build-side** gate on `build.sh` | 3 | B6, B8, C1 |
| covered transitively by one of the above | 3 | P4, P5, P6 |

**Only three hard placement checks live on aeon's build side.** All three are post-sigil listing
readers; all three are `sonic4`-only; all three are skipped under `FAST=1`. And two of those
three — B6 (the `DATA_GROWTH_RESERVE` floor) and B8 (the BG-animation ceiling) — **rest on F1
and F2, which nothing checks.** So the aeon-side enforcement surface is effectively *one*
independently-grounded gate: C1, `dplc_straddle.py`, which derives its boundary from source and
reads the built ROM.

**The 19 unchecked rows are not evenly distributed.** Twelve of them (F1–F7, G1–G5) are the same
two mistakes made twelve times: **a named label standing in for a position** — either "the end of
a region" (terminus proxies) or "the next thing emitted" (adjacency pins). They are unchecked
*because they are not constraints anyone wrote down*; they are assumptions that fell out of the
frozen order being stable, and they are load-bearing for gates that then pass.

## §6 — Is the precondition satisfiable from this list?

**No. Not from this list, and the reason is structural rather than a matter of length.**

The precondition asks that *every constraint the frozen tables encode be recaptured as an
explicit rule before the tables stop being authority.* This list is a good-faith enumeration
from aeon's side with per-source controls, and it still cannot discharge that, for three
reasons — in increasing order of how much they matter.

**1. Two of the three sources are near-empty, and their emptiness is structural, so the
enumeration's confidence should not be spread evenly across them.** Source 2 carries 8 rows out
of 1,246 `ensure` sites, and all of them are already safe: they evaluate against the *placed*
address. Source 3 carries no sole-home constraints at all. Almost the entire result is source 1,
and within source 1 almost the entire *risk* is the F/G/H classes. A future pass should not
budget three equal source sweeps; it should budget one deep source-1 read and two shallow
confirmations.

**2. The most dangerous class cannot be enumerated by asking "what constraints exist".** P3 is
the proof: a real alignment requirement whose only trace is a comment explaining why a faster
form was **rejected**. It has no gate, no `ensure`, and no commit stating it as a rule; none of
the three sources contains it, and it surfaced only as a by-catch of a history walk looking for
something else. F1–F7 and G1–G5 are the same shape one step further along — **requirements that
were never stated because nothing ever made anyone state them.** The question that finds this
class is not *"what constraints exist"* but **"what does this code compute that nothing
verifies"**, and that question has to be asked file by file over the consuming end. This
inventory found twelve by asking it of `tools/`; it has not been asked of the `test_*.py` lane,
of `s4lint.py`, or of `effects_gen.py`.

**3. The measured answer to "how free is the packer, really" is that it is not very free, and
nobody has re-measured it since.** P6: sweeping `Art_Sonic`'s base ±64 KB, **2,773 of 131,073
one-byte positions fail, in 43 forbidden bands.** That is 2.1% of positions, and the tree
currently sits **5,188 B** from a band edge. Retiring the frozen tables means handing the packer
a freedom it has been measured, once, not to have — and that measurement was taken at a
different layout, before the 09-04 re-layout moved the banks 96 KB. **Whatever else DECOUPLE
does, that sweep needs re-running at the current layout before the tables come out.**

### What is actually needed, and from what parameter

Three passes, none of which is another *"enumerate constraints from repo X"*:

- **(a) A consuming-end pass over the aeon tools that this one only skimmed** — `s4lint.py`,
  `effects_gen.py`, the `tools/test_*.py` pytest lane, and the seven gates read by docstring
  only. Parameter: **not** "find constraints" but *"find every place a position is inferred from
  a name"* — the F/G shapes. Twelve were found in the tools that were read in full; the tools
  read only by docstring are the same kind of code.
- **(b) Re-run P6's sweep at the current layout**, and extend it from `Art_Sonic` to the other
  three straddle subjects. Until that number exists for `0xA8000`, "the packer may float" is
  unpriced.
- **(c) Close F1/F3 structurally rather than by writing a rule.** Both are terminus proxies, and
  a proxy cannot be fixed by declaring the proxy correct — that is the maintenance-act shape:
  the fix would be a hand-edited assertion indistinguishable from enshrining the accident.
  `bganim_room.py` should take `packed_data_end` from **the maximum LMA+extent over every
  section below the anchor in the listing**, which needs no label named and no population
  enumerated; `s4budget.py` should take the object bank's used cursor the same way. Both are
  small changes to tools aeon owns, and both convert an assumption into a measurement that
  cannot go quietly wrong.

Do (c) first: it is the cheapest, it is entirely aeon-side, it retires seven of the nineteen
unchecked rows at a stroke, and unlike the rest of this list it does not depend on anyone
agreeing with an enumeration.

### The three old predicates that still stand

Of the 2026-08-26 inventory's seven, four have landed (§4.1). The residue maps onto this list:

- **`ANCHOR_BINDS_SECTION`** → D6, and the pressure on it has risen: `seam2::bank_anchors_from_str`
  now looks up `dac_banks` and `sound_bank` **by name** on the shipped path, so the "nothing keys
  anchors by name" premise the old ledger comment rests on is spent.
- **`SOUND_BANK_WINDOW_PHASE`** → A7, unchanged, and its `at` has moved twice since it was written.
- **`REGION_END_IS_OWN_SECTION`** → F1/F3, and this is the one that got *worse*. The 2026-08-27
  note found it in `repin`, a maintenance binary no ROM build consults, and could reasonably
  price it low. It is now the instrument under two live `build.sh` gates.


## §7 — CORRECTIONS from aeon's confirmation, 2026-09-05

Aeon confirmed this inventory and landed its reading at aeon `305af222` (verified here an
ancestor of their `origin/master`, and the text found by `git show 305af222 | grep -n nt_row`).
Appended as a dated correction rather than folded into the rows above, so a reader can see what
this survey got wrong and in which direction.

### 7.1 — Both defects CONFIRMED, and my replacement coordinate for the second was wrong

Defect 1 (`bganim_room.py:266` computes an end from one label plus a blob length and compares it
to nothing) holds, reproduced independently by aeon across all twelve rows of that shape. Their
figures: `Art_Sonic` 0x72F72, packed_end 0x8BA32, anchor 0xA8000, room 0x1C5CE — and their
framing is sharper than mine: `Art_Sonic` is today the max-LMA label below the anchor with zero
labels between it and the anchor, so the assumption holds **by luck of declared order, not by
construction.**

Defect 2 holds and is corrected in place — **and my replacement coordinate was wrong in exactly
the way this document's own booking warns about.** I restated the 2026-08-30 figures
(`Sound_PlaySFX` at 0x8024, margin 36 B) instead of re-measuring them. From a real listing the
binding symbol is **`nt_row` inside `BG_Init` at 0x801A, margin 26 B**; `Sound_PlaySFX` sits at
0x825A with 602 B and is not binding at all; `BG_Init`'s body straddles 0x8000; and the ceiling
neighbourhood is `bg.emp` / `bg_anim.emp`, **not the sound region**. So the section of this note
that criticised a stale named-symbol grep reproduced a stale figure while doing it. Aeon replaced
the grep with an awk that measures.

**The rule this cost, stated because the survey broke it while enumerating others breaking it:**
a figure carried forward from a dated note is a claim about a tree that has moved, and citing it
inside a correction does not refresh it. The direction matters too — 36 B read as more headroom
than the real 26 B, which is the favourable direction, the one that never gets caught.

### 7.2 — `falls_into` is the shipped answer to Class G, and this survey credited it nothing

Aeon's finding, verified here firsthand rather than taken from the relay: `.emp` has a
`falls_into` proc contract that machine-checks intra-section adjacency. **47 occurrences across
20 aeon `.emp` files** (`git grep -c falls_into -- '*.emp'`), enforced in **sigil's**
`crates/sigil-frontend-emp/src/corpus_contracts.rs` (`falls_into_succ`, built from the proc
buffers and checked at lower time). Class G above says these adjacencies are "an assertion about
emission order that nothing verifies" and lists no mechanism that could express them. That was a
gap in the survey.

**⚠ AND IT DOES NOT MOVE THE COUNT — refuse that reading explicitly, because the summary of this
finding invites it.** "Zero rows in your distribution" reads as *you undercounted the checked
rows*. Measured here: of the nine symbols named in G1–G5 — `ObjDef_Static`, `OJZ_Reels_Fill`,
`OJZ_Reel_Speed`, `OJZ_TestPal`, `OJZ_BaseSwap`, `Parallax_Update`,
`ParallaxConfig_OJZ_Default`, `ParallaxConfig_OJZ_Underwater`, `DeformTable_OJZ_Calm` — **not
one appears in any `falls_into` declaration.** The two constructs address the same *shape* and
not the same *sites*: `falls_into` is a source-side declaration about control-flow fallthrough
between procs; G1–G5 are gate-side assumptions about the layout of data and routine symbols.
**All five Class G rows remain checked by nothing, and §5's 19 stands.**

What changes is §6's recommendation, and it changes in the cheaper direction: **fixing Class G
needs no new mechanism designed.** The language already has a machine-checked adjacency
construct in daily use; the work is expressing these five as it, or as its data-side analogue if
one is needed, rather than inventing a scheme. That is a smaller and better-grounded ask than
this note made.

### 7.3 — Aeon's pytest lane is structurally blind to placement (corroborates, does not correct)

Aeon reports 20 placement-shaped files in their pytest lane that read **committed cuts of past
listings**, so no re-derivation can redden any of them. That is §2's finding arriving from their
side and by a different parameter: this survey enumerated by what touches placement, theirs by
what their test lane actually reads. Independent corroboration, not echo.

### 7.4 — Three more sources, and one constraint that is unexpressible

Beyond §3's admitted omission of `map.toml` prose: **A9 and A10 live only in `map.toml` prose**
(`map.toml:285-288`); **intra-section emission order is unexpressible in `map.toml`** at all
(`page_cache.emp:926` is live and depends on it); and **`decisions.jsonl` is a fifth source**,
with 50 cards carrying placement text. The three-source scope was narrower than the population
by more than the one source §4.2 conceded.

### 7.5 — P6 is UNCHECKED by aeon, and should not be read as confirmed

Aeon did not re-run the Art_Sonic sweep at the current layout — the commit exists and the gate is
on the build path, but re-running was outside their parcel. **So the 2,773-of-131,073 figure and
the 5,188 B margin carry this survey's measurement alone**, at the revision named in §1, and no
second party has reproduced them. Stated here because a confirmation that covers most of a
document gets read as covering all of it.

### 7.6 — P6 RE-SWEPT: the counts could not have moved, and §6(b) was wrong about why

Measured at aeon `305af222`; full result in `2026-09-05-p6-resweep-current-layout.md` and the
band data in `2026-09-05-p6-resweep-305af222.json`, re-runnable via `scripts/p6_resweep.py`.

**§6 point 3 and item (b) said this sweep needed re-running because the layout had moved. That
is true of ONE of the five figures they quote.** The straddle predicate depends on the base only
through `(base + offset + d) mod 0x20000`, so the failing set is **periodic with period
131,072** and a base move merely *translates* it. P6's ±64 KB window is **131,073 positions —
exactly one period plus one — so it is a COMPLETE CENSUS of every residue class**, not a
neighbourhood sample. Verified at this seat: the arithmetic holds, and the landed band data sums
to exactly **2,773 over 43 bands with 43 distinct starts mod the period**, so nothing is
double-counted across the wrap.

So `2,773 / 131,073`, the 43 bands, the 31/31/415 widths and the peak of 17 are **invariant by
construction**. Only the **margin** is layout-dependent, and it moved: **+3,102 B** in the
shrink direction (5,188 → 8,290) and **−3,102 B** in the growth direction (36,092 → 32,990).

**This also makes the unfreedom a STRONGER claim than §6 made, not a weaker one.** 2.1% of
positions forbidden is a property of `DPLC_Sonic` mod 128 KB — **permanent, not contingent**. No
re-layout can improve it.

**Extending to the other three subjects is a null result and a structural one.** Slot cost is
≤ 2× entry count, putting the ceilings at Tails 4, appendage 2, Knuckles 10 against a bar of 10:
**three of four cannot fail at any base in the address space.** The enumeration was carrying
priced risk that does not exist. *(Knuckles' ceiling equals the bar exactly, and nothing states
it — one 6-entry frame added to `DPLC_Knuckles` changes that. Routed to aeon.)*

**And the 09-04 re-layout is not what moved the margin** — this document implied it did. The
anchors are `0xA8000`/`0xB8000`, `Art_Sonic` *ends* at `0x8C402` below both, and `map.toml`
derives the anchors *from* `packed_data_end`: **the banks moved because the data grew, not the
reverse.** What moved the phase is seven days of ordinary upstream growth, ~440 B/day, unwatched.

**THE VERDICT SURVIVES AND THE ARGUMENT FOR IT DOES NOT.** §5's "not yet" stands, but not on
this row's reasoning. The live risk here is not a stale census — it is a **decaying margin**:
`BLOCK-STREAM-DEDUP`'s slack above its safe band fell **5,686 → 2,584 B in seven days**, because
`Art_Sonic` is the terminus of the packed region and every byte added anywhere below it moves
the base. At the observed drift, that parcel's approved **−20,986 B** shift walks into a
forbidden band in **under a week of unrelated data growth**, with no gate that says so before
the build reds. Routed to aeon; it wants a monitor, not a survey row.

**New since P6, and not a budget question:** one 31 B band at `[+45,567, +45,597]` where a
*reachable* Sonic frame splits seven ways and `.split_reject` drops the transfer — **art that
would not load.**

**Not swept and not sweepable by this instrument:** VERDICT C is a property of every subject's
base *at once*, and this sweep moves one at a time. 0 of 2 today, unpriced under a re-layout.
