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
| **B7** | **`Art_Sonic` must be the last packed blob before the banks, and its section must have exactly one embed.** `bganim_room.py` derives `packed_data_end` as `LMA(Art_Sonic) + len(art/optimized/characters/sonic.bin)`. | **1** (tool) + **1** (map prose) | `tools/bganim_room.py`; `games/sonic4/map.toml` — *"`Art_Sonic` is the last packed blob before the banks by `order`"* and *"they go BEFORE collision_data … on purpose: a section with several embeds has no such instrument"* | **nowhere.** Nothing asserts that `Art_Sonic` is the terminus, nor that its section stays single-embed. The map's `order` puts it there; violating that changes the derived end silently. | **This is the sharpest row in the list.** If placement is re-derived and anything lands between `Art_Sonic` and `Dac_Temp_Blip`, `bganim_room.py` measures a `packed_data_end` that is too low, computes MORE room than exists, and the reserve gate goes green while the reserve is gone. An always-green gate on a real breach. It is exactly the prior note's R6 shape — *"the gap between two labels is an allotment"* — reappearing in a live build gate rather than in `repin`. |
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

---

## §2 — Coverage, per source

<!-- FILLED BELOW ONCE AGENT RESULTS LAND -->

## §3 — What this did NOT cover

## §4 — Where the brief I was given was wrong

## §5 — Is the precondition satisfiable from this list?
