# SIGIL-DECOUPLE — the consuming-end pass: every place a POSITION is inferred from a NAME

**What this is.** `2026-09-05-decouple-aeon-side-inventory.md` §6(a) owed a pass whose parameter
is *not* "what constraints exist" but **"what does this code compute that nothing verifies"** —
specifically, *find every place a position is inferred from a NAME.* The survey found twelve
(Classes F and G) by asking it of the aeon tools it read in full; it did not ask it of
`s4lint.py`, `effects_gen.py`, the `tools/test_*.py` pytest lane, or seven gates it read by
docstring only. This is that pass.

**PIN.** Aeon read ONLY at `305af222` (`merge(item 14): confirm sigil's decouple inventory —
both defects hold, their fix does not`), in a detached worktree
(`git -C /home/volence/sonic_hacks/aeon worktree add --detach <path> 305af222`). The owner's
live checkout `/home/volence/sonic_hacks/aeon` was never read as a tree. **No ROM was built and
no aeon tool was run** — every behavioural claim below is from control flow read out of
`build.sh`, or from the tool's own code, and is labelled as such. **This parcel moved no aeon
bytes and lands nothing in the aeon repo.**

---

## §1 — The population, and how it was derived

The brief named a list and told me not to take it as the population. Derived independently, by
**mechanism** rather than by name, over `tools/*.py` at the pin:

```
axis 1  files naming a listing            grep -lE "\.lst\b"                          → 84
axis 2  files reading the placement map   grep -lE "map\.toml"                        → 13
axis 3  files indexing the ROM by address grep -lE "0xFFFFFF|\brom\[|_rom\["          → 62
        union                                                                          → 92
```

92 of the tree's 218 tracked `tools/*.{py,sh}` touch a listing, a map, or ROM-by-address. Of the
81 `tools/test_*.py` files, **22** are placement-shaped by the same axes (enumerating command in
§4).

### Differences against the note's list, both directions

**The note's list is a subset. It named nothing that turned out not to belong** — `s4lint.py`,
`effects_gen.py`, the pytest lane, and the seven gates are all genuinely in the population, and
the seven are identifiable from §3 without guessing: of the thirteen files listed as "docstring +
targeted grep only", exactly seven are gates —
`editor_palette_golden.py`, `row_remap_gate.py`, `waterline_art_gate.py`, `sprite_tilt_gate.py`,
`instashield_gate.py`, `loop_crossover_gate.py`, `plane_role_swap_gate.py`. The other six
(`collision_consistency.py`, `emp_expect_fail.py`, `gen_compression_vectors.py`,
`suite_paths.py`, plus the two big files) are not gates in that sense.

**What the note's list omits, and what this pass found there:**

| file | in the note's §3 "not read" list? | why it belongs |
|---|---|---|
| `scene_spans.py` | **no — named nowhere in the survey** | owns `lst_proc_sizes`, the tree's fifth and only *corrected* implementation of the Class-G5 extent rule |
| `demo_specialization_witness.py` | **no** | consumes `lst_proc_sizes`; its own test carries a Class-G adjacency difference |
| `state_ram.py` | **no** | carries four hardcoded name→address rows used silently when no listing is passed |
| `demo_drift_classifier.py` | **no** | second, independent instance of F4's un-failable `EndOfRom` NOTE |
| `curve_probe.py`, `deform_own_cost_probe.py`, `parallax_cost_probe.py` | **no** | three copies of one RAM-side adjacency pin |

### One correction to my own method, recorded because it nearly became a finding

My first build-path enumeration used `grep -oE '(\$\{TOOLS\}|tools)/[a-z_0-9]+\.(py|sh)' build.sh`
and returned **30** tools. That number is **wrong**: the regex matched tool names inside
`build.sh`'s prose comments, and `build.sh` is ~1,160 lines of which a large fraction is comment.
It put `effects_gates.py`, `reels_witness.py`, `ojz_block_gen.py`, `repaint_ojz_collision.py` and
two `test_*.py` files on the build path when none of them is invoked from it. Re-derived by
stripping comment lines first and requiring an actual invocation verb:

```
grep -vE '^\s*#' build.sh \
  | grep -oE '(python3|bash|sh|\./)\s+"?\$\{TOOLS\}/[a-z_0-9]+\.(py|sh)' \
  | grep -oE '[a-z_0-9]+\.(py|sh)' | sort -u        →  24
```

**24, which reproduces the survey's own figure exactly.** This matters beyond bookkeeping: it is
the *name / presence / behaviour* distinction inside my own instrument. A tool's name appearing
in `build.sh` is not evidence `build.sh` runs it, and in this tree — where comments outweigh code
— the naive grep is wrong by 25%.

**Consequence for §2's rows:** `scene_spans.py` and `demo_specialization_witness.py` are reached
by `effects_gates.py`, which is an emulator-backed lane run separately and **not** from
`build.sh`. They are on the build path only through the pytest lane, which imports `scene_spans`
but (see §4) cannot observe placement.

---

## §2 — The sites

### S1 — `s4lint.py` · `SST_FIELDS` / `SST_LEN` → `_resolve_sst_offset` → `check_e009`

**Position inferred, from what name.** A byte offset *inside the live object record* is inferred
entirely from the spelling of the symbol in the operand text: `SST_x_pos` **means** `0x02` to
this tool. A 26-row `Dict[str, int]` plus `SST_LEN: int = 0x50` is the whole layout model. Nothing
measures it.

**It is already wrong, verified firsthand** against `engine/objects/sst.emp`, whose own header
declares that file "the SOLE author of the object-record layout":

| `SST_FIELDS` says | `sst.emp` says |
|---|---|
| `SST_width_pixels 0x18` | `width_pixels @ $16` |
| `SST_anim 0x1A` | `anim @ $18` |
| `SST_subtype 0x24` | `subtype @ $19` |
| `SST_status 0x2C` | `status @ $1E` |
| `SST_mapping_frame 0x1E` | `mapping_frame @ $23` |
| `SST_sst_custom 0x32` | `sst_custom @ $30` |
| `SST_priority`, `SST_respawn_index`, `SST_wait_timer`, `SST_anim_callback` | **absent from the struct** |
| — | `angle`, `sprite_piece_count`, `slot_tag`, `entity_section_id`, `entity_list_index`, `layer`, `frame_off` **absent from the table** |

Only `SST_LEN = 0x50` still agrees (`sst_custom @ $30` + 32 = `$50`).

**Scenario for silent wrongness — live today, not hypothetical.** `SST_mapping_frame+45(a0)`
resolves here to `$1E+45 = $4B < $50` and passes; the real address is `$23+45 = $50`, one byte
past the record — the exact overrun E009 exists to catch. Symmetrically `SST_sst_custom+30`
resolves to `$32+30 = $50` and **errors**, while the real address `$30+30 = $4E` is in bounds.
And `_resolve_sst_offset` returns `None` for any unrecognised `SST_*` name, so `check_e009` does
nothing at all for the seven fields added since — a *rename* silently switches the check off with
no message.

**Failure class: all three at once.** SILENT for the stale-forward fields, SILENT for the
unrecognised ones, and LOUD-BUT-MISATTRIBUTED for `sst_custom` — the message reads
`SST access at offset $50 is past end of struct (SST_len=$50)`, blaming the assembly line's
arithmetic when the cause is that the struct moved and the table did not.

**Blast radius today, measured rather than assumed.** `build.sh` lints
`games/${GAME}/game_root.asm`. Verified firsthand: the tree has **three** tracked `.asm` files
(`engine/debug/debugger.asm`, `games/demo/game_root.asm`, `games/sonic4/game_root.asm`), and
`grep -rn "SST_" --include='*.asm' .` returns **grep exit 1 — a genuine no-match, not an error**.
E009 fires on nothing at present. **This is a loaded gun, not a firing one**, and the tool emits
no signal that its coverage has collapsed to zero.

**Structurally fixable: YES.** The linter already tokenises `struct` / `ds.b` / `endstruct` and
already walks the include graph; the `.emp` side is the sole author and the build already
harvests these offsets. Deriving the table by *measuring* the declared field widths names no
field and pins nothing. **⚠ Hazard on that fix, and it is the reason the staleness survived:**
`tools/test_s4lint.py` consumes the same stale constant and documents it as correct —
`test_sst_custom_base_no_error`'s docstring reads *"SST_sst_custom ($32) with no addition"*.
Correcting the table turns that green test red. **A green gate is currently locking in a false
positive**, which is why the defect looks verified.

### S2 — `state_ram.py` · `FALLBACK_SYMBOLS` → `Ram.addr_of`

**Position inferred, from what name.** Four hardcoded absolute RAM addresses keyed by symbol
name (`Player_1 0xFFFF8ED6`, `Player_2 0xFFFF8F26`, `Camera_X 0xFFFFA604`, `Camera_Y 0xFFFFA608`),
used by `addr_of` whenever the name is not in the listing. `load_symbols` returns an **empty
dict** for a missing or unsupplied `--lst` without complaint, so the fallback path is reached by
simply omitting the flag.

**Scenario.** Any edit to `ram.emp` that moves the player SSTs or the camera block — the ordinary
consequence of adding a RAM variable upstream — leaves these four rows pointing at whatever now
occupies those addresses. The decoder then prints a complete, plausible page of player state read
from the wrong bytes.

**Failure class: SILENT.** The tool's stated contract is that it "REFUSES a state it cannot
justify … never a plausible-looking page of nonsense". The fallback is the one path that violates
its own contract.

**Structurally fixable: YES, by deletion.** Delete `FALLBACK_SYMBOLS` and make a missing `--lst`
a refusal. `addr_of`'s error message already spells the right instruction
(*"pass --lst pointing at the build's listing"*). This is *measure or refuse*, not a new
assertion.

**⚠ Its test cannot catch this.** `tools/test_state_ram.py` reads
`SR.FALLBACK_SYMBOLS["Camera_X"]` and derives its expectations *from the same constant*. The
test consumes the value it would have to falsify, so it stays green at any address.

### S3 — four independent `routine_extent` implementations, one of which learned the lesson

The survey's **G5** ("a routine's ROM extent is `[LMA, next non-local symbol above it)`") names
three sites. There are **four**, and there is a **fifth implementation of the same rule that has
already been corrected and whose correction was not propagated.**

| implementation | filters its own locals | filters RAM | **filters PHASED symbols** |
|---|---|---|---|
| `instashield_gate.routine_extent` | yes (`prefix`) | no | **no** |
| `sprite_tilt_gate.routine_extent` | yes | no | **no** |
| `loop_crossover_gate.routine_extent` | yes (`marker`) | no | **no** |
| `waterline_art_gate.proc_span` | yes (`own`) | yes (`v < 0x400000`) | **no** |
| `scene_spans.lst_proc_sizes` | n/a (head-to-next-head) | yes (`a < 0x800000`) | **YES — `vma_phased_symbol_names()`** |

**Position inferred, from what name.** All five infer "where this routine ends" from "the address
of whatever symbol is listed next". The first four then trust that the next *listed* address is
the next *ROM* address.

**Scenario for silent wrongness — measured, by aeon, on 2026-09-03.** A phased section head
carries its **VMA** in the listing, not its LMA. `scene_spans.vma_phased_symbol_names`'s own
docstring records two real truncations: `SoundTablesZ80_Head` listed at `$8000` cut
`Parallax_Step5_Vscroll` to 64 bytes, and `SfxBlobWinTab` listed at `$845F` cut `Raster_HInt` to
21 bytes. Both come from one section, `soundbankhead.emp`'s
`section soundbankhead (cpu: m68000, vma: $8000)`. **The hazard is not theoretical and the
address neighbourhood is populated** — `test_plane_role_swap_gate.py` records
`Parallax_Set_Roles_Swapped` at `$007BD6`, below `$8000`.

Concretely: any re-layout that moves one of the four gates' subjects below `$85AD` puts a phased
symbol inside its address run and truncates its extent.

**Failure class: LOUD-BUT-MISATTRIBUTED, which is the survey's own worst category and it is
correct here.** `sprite_tilt_gate` and `loop_crossover_gate` compare a normalised decoded
instruction stream against a committed cut; a truncated extent yields a shorter stream and the
gates report that the routine's code changed. `instashield_gate` and `waterline_art_gate`
decode the extent and report that the instruction run is not clean. In every case the message
names a code defect; the cause is a layout change.

**Structurally fixable: YES, and the fix already exists in this tree.**
`scene_spans.vma_phased_symbol_names()` derives the phased set **from source**, by scanning every
`section … (…, vma: $HEX)` declaration and collecting its top-level declarations. It names no
symbol and enumerates no population. The work is to make the other four consume it. Its docstring
even states why the listing cannot answer this alone — *"sigil-link's `emit_listing` writes one
`(0) N/HEXADDR : Name:` shape for every symbol, phased or not, with no field saying which"* —
which is a **sigil-side ask**: a listing that marked phased rows would retire this whole class.

**This is the "gate every consumer of a value" shape at n=5.** The property was established at
one consumer and left unestablished at four.

### S4 — `demo_drift_classifier.py` · the code/appendix boundary from `EndOfRom`

**Position inferred, from what name.** `appendix = min(so["EndOfRom"], sn["EndOfRom"])` — the
boundary between ROM content and the deb2 symbol appendix is taken to be the `EndOfRom` symbol.

**This is a SECOND, independent instance of the survey's F4.** F4 is `s4budget.py`
`format_rom_report()` printing `NOTE: EndOfRom and the ROM file differ by …` and never appending
it to `breaches`. Here, when the two listings disagree, the tool prints
`NOTE: EndOfRom itself moved $X -> $Y — the code/data region changed SIZE, which a pure
RAM-growth parcel must not do.` and **continues with `min()`**. The sentence states a violated
requirement and the code does not act on it.

**Failure class: SILENT** for the classification that follows (a wrong boundary re-attributes
appendix bytes as code diffs — the tool's own comment records a prior mis-boundary reporting
*"615 unclassified bytes that were symbol-table content all along, i.e. a confident false
finding"*).

**Partial credit, and it is real.** Unlike `s4budget`, this tool **refuses** outright when
`EndOfRom` is absent from either listing (`raise SystemExit`), and it derives its relocation
deltas and region floor from the symbol tables rather than from a density heuristic — its own
comment: *"Density heuristics do not get a vote when the linker emits the answer."* The defect is
narrowly the `min()`-and-continue on a stated violation.

**Structurally fixable: YES.** Turn the NOTE into the refusal its own text describes.

### S5 — three copies of one RAM-side adjacency pin

`curve_probe.py`, `deform_own_cost_probe.py` and `parallax_cost_probe.py` each spell
`span = sym["Parallax_Shadow_Scroll_A"] - sym["Parallax_Shadow_Bands"]` — a region's *size*
derived from the address of a named neighbour. Same shape as Class G, on the RAM side.

**Failure class: SILENT** (a wrong span reads too few or too many bands and the probe reports a
number). **Structurally fixable: YES** — declare an explicit `Parallax_Shadow_Bands_End` and
subtract that, the way `sprite_owner_probe.py` already does with
`SST_SPAN_SYMBOLS = ("Object_RAM", "Object_RAM_End")`. **Presence-only on invocation:** none of
the three is invoked from `build.sh`; they are emulator-driven probes, so this is a correctness
risk for measurements, not for the shipped gate surface.

### S6 — `test_demo_specialization_witness.py` · a Class-G difference inside a test

`collision_signature = addrs["SfxBlobWinTab"] - addrs["Raster_HInt"]`, asserted `!=`
`sizes["Raster_HInt"]`. The quantity is only a "collision signature" if `SfxBlobWinTab` is the
symbol that would have terminated `Raster_HInt` — an adjacency assumption, in the one test in the
lane that reads a real listing.

**Failure class: SILENT, in the direction of a vacuous pass.** A re-layout that separates the two
symbols makes the difference large, the `assertNotEqual` trivially true, and the regression the
test exists to catch invisible. **Structurally fixable: YES** — assert the positive property
(that `Raster_HInt`'s measured size equals its real extent with phased symbols excluded) rather
than the negative one.

### S7 — `effects_gen.py` · `walk_patch_sites`: a 400-character window over UNSTRIPPED comments

**Position inferred, from what name.** The extent of one `patchable(...)` call is taken to be
`min(start + 400, the next literal "patchable(")`, and `ch:` / `lo:` / `hi:` are regex-searched
inside it. The starts come from `re.finditer(r"patchable\s*\(", src)` over the **raw file** —
`src = f.read()` with no comment stripping — while its two siblings in the same file
(`preset_parallax_bindings`, `section_preset_symbols`) both call `_strip_line_comments` first,
with a docstring saying prose in this tree contains the words being searched for.

**Verified firsthand:** `games/sonic4/data/effects/ojz_effects.emp` contains **12** literal
`patchable(` occurrences, of which **8 are on comment lines**. Two thirds of this walk's anchors
are prose.

**Scenario.** The same file carries `lo:` / `hi:` numbers *in a comment* below a comment-borne
`patchable(` mention whose window currently closes before reaching them. Deleting an intervening
banner brings a comment's numbers into the window and the walk mints a channel from prose. In the
other direction, a real call whose first argument spans enough lines to push `lo:` past 400
characters drops out.

**Failure class: split, and two of the three are bad.** `ch:` present but `lo:`/`hi:` missing →
explicit `_refuse` (LOUD, correct). `ch:` itself outside the window → `continue`, and the site
disappears from both consumers: `_check_patch_context` then refuses a legal authored sweep with
*"NOTHING in games/sonic4/data/effects consumes that channel"* (**LOUD-BUT-MISATTRIBUTED** — it
blames the author's document for a scanner-window problem), and `render_channel_bands` drops the
channel from the aurora sidecar (**SILENT** — the drift gate compares only against the
*committed* file, so one bake plus one commit makes the hole permanent and green).

**Structurally fixable: YES** — match balanced parentheses instead of counting characters, and
strip comments the way the function two definitions above already does.

### S8 — `effects_gen.py` · source LINE NUMBERS baked into a byte-compared committed artifact

`render_channel_bands` publishes `"source": f"…/{site['file']}:{site['line']}"` and
`_edge_behaviour` publishes `"engine": f"engine/effects/raster.emp:{hits[0]}"`. **Verified
firsthand** in the committed `games/sonic4/data/generated/effects_channel_bands.json`:
`ojz_effects.emp:1809/1811/1995/1998` and `raster.emp:1980/1983`.

These are *measured*, which is right — but they land in an artifact `effects_gen.py check`
compares byte-for-byte on every canonical build. **Inserting one comment line above line 1809 of
`ojz_effects.emp` fails the build** with `DRIFT — … A patchable( band moved, or the sidecar was
hand-edited.` Neither statement is true.

**Failure class: LOUD-BUT-MISATTRIBUTED.** The remedy is a genuine re-bake, so it is annoying
rather than dangerous — but it is an *always-red-on-correct-work* shape, which trains people to
re-bake reflexively. **Structurally fixable: YES** — drop `:line` from the published record, or
key on a stable identifier.

Adjacent and worse because it is silent: `render_channel_bands`'s `how_to_use` string hardcodes
`engine/effects/raster.emp:397`. It is correct today. Unlike the derived refs it does not move
with the file, so nothing notices when it stops pointing at the ensure it names — inside a
document whose own `edges` block exists because *"a sidecar that hardcoded the same two facts
would rot the same way."* **SILENT.**

### S9 — `effects_gen.py` · POINTER IDENTITY decided by string-comparing symbol NAMES

`render_module`'s reels arm builds `emitted = {names.binding_sec(i) for i in rung1}` and then
`hits = [a for a in aliases if a["target"] in emitted]`. The question it is answering is *"does
any hand preset's `parallax:` point at the same ROM object as this section's lowered record"* —
an **address** question — and it answers by **name equality on whatever identifier the argument
happens to spell.**

**This is the purest instance of the parcel's parameter found anywhere in this pass.**

**Scenario.** A hand preset writes `parallax: SomeOtherSpelling` for the same record — an `equ`, a
re-export through `scene_registry.emp`, any second name for one address. The name-set reports no
hit, the `reels` key is accepted, and two sections resolve to one `Parallax_Current_Config`
pointer. That is exactly what the rung model exists to prevent, and the file states the symptom
itself: *"nothing errors, nothing is missing, the wrong strips simply scroll."*

Compounding it: `_PRESET_DECL` and `_PARALLAX_ARG` are spelling recognisers whose **non-match is
read as "no rung-2 binding"** — the permissive answer. The same file's `reel_band_count` and
`game_scanline_caps` both refuse outright when their declaration is not found, on the stated
argument that *a check that cannot run must not pass*. The two preset walks do not carry that
guard.

**Failure class: SILENT.** **Fixable structurally only in half.** The "zero `preset(`
declarations found in a directory of `.emp` files" case can be made a refusal like
`reel_band_count`'s — that half is free. **Proving address identity across aliases cannot be done
from source text at all**, and the honest fix moves that half to something reading *linked*
addresses (a listing). It must not be closed by writing an assertion that the names agree.

### S10 — `effects_gen.py` · three lower-severity sites, stated so they are not re-found

- **`render_channel_bands`, duplicate-channel arm.** `if ch in channels: continue  # guard 11 …
  first wins` — and "first" is `sorted(os.listdir(lib))`, i.e. **alphabetical filename order**. A
  second `patchable(ch: 0, …)` landing in `ojz_scenes.emp` publishes `ojz_effects.emp`'s band to
  aurora instead. SILENT. Structurally fixable: refuse the duplicate here rather than deferring to
  an engine-side "guard 11" this file neither reads nor names.
- **`section_preset_symbols`.** Section↔preset attribution by *textual adjacency* — the nearest
  numeric `sec:` before an `effects:`. Blast radius is bounded (it only decorates an error message
  with "bound by section(s) N"; the refusal still fires). SILENT, structurally fixable by parsing
  the call.
- **`render_table_ref`.** `full = os.path.join(REPO, *TABLE_BIN_ROOT, rel)` uses the **module-level**
  `REPO` for its `isfile` and 256-byte `getsize` checks, while the emitted
  `embed("games/sonic4/data/editor/effects/<rel>")` resolves against the **build's** repo. Run
  with a different repo root — which the test suite does — and it validates one tree's table while
  embedding another's. SILENT. Structurally fixable, though note `render_table_ref`'s signature
  carries no `repo` parameter to thread; the caller has to supply one, so the fix is slightly
  wider than a one-line change.

### S11 — `effects_gen.py` · the emission-order dependency, and the placement rule that lives only in a comment

Two ROM-placement facts this file *depends on* and does not check:

- Its module header asserts the block's ROM position is declared **by SECTION NAME** "because this
  block's head label is content-derived". **Verified:** `games/sonic4/map.toml` carries
  `"section:ojz_effects_editor_act1"`. This is the survey's **D2** and it is mitigated today.
  What would break it silently: changing that row to any *label* row. The first byte-emitting item
  is the deform-table block, emitted only `if used`, which vanishes entirely if the last bound
  scene loses its deform attachments — so the anchor would become a different symbol at a
  different offset with nothing noticing. The header paragraph is documentation, not a check, and
  the file's own record shows it was **false for days** before a "CORRECTED 2026-08-29" in-place fix.
- Its `REELS_KEY` banner records that reel tables are emitted into the *generated* module rather
  than beside the hand table **specifically so as not to disarm `reels_gate.py`**, whose
  `SPEED_SYM`/`FILL_SYM`/`NEXT_SYM` measure the table and proc as address gaps (the survey's
  **G1**). This is a real placement constraint whose only home is a comment in a different file
  from the gate it protects. **And the failure is silent in the way that matters:** `reels_gate`
  raises `Unmeasurable`, which is **not** `FAIL` — move the emission and the gate goes *quiet*,
  not red.

**Neither is this file's to fix.** They are recorded because S11's second item is a **new
source-3-shaped row**: a placement rule that exists only as the reason a natural refactor was not
taken — the survey's **P3** shape, found again, in a second file, four days later.

### S12 — `s4lint.py` · `check_e006`'s VDP port identification (priced low, recorded anyway)

`check_e006` identifies the VDP ports by substring-matching the *names* `"VDP_CTRL"` /
`"VDP_DATA"` and by the literal addresses `0xC00000` / `0xC00004`. The addresses are Genesis
hardware and cannot move, so this is not a layout risk. The **name** half can go stale: a rename
or an alias makes E006 stop firing SILENTLY. Structurally fixable by resolving the symbol to its
value. Also noted: `_VDP_WRITE_TARGETS` is defined and **never read** — `check_e006` uses its own
inline tuple. A dead constant shaped like the check's configuration; editing it changes nothing.

---

## §3 — Where the honest fix would be a hand-edited assertion

**One site is only half-fixable structurally, and it is named: S9.** Proving that two `.emp`
identifiers denote the same ROM object cannot be done from source text. The free half (refusing
when a directory of `.emp` files yields zero `preset(` declarations, the way the same file's
`reel_band_count` already refuses) should be taken; the identity half must move to something that
reads **linked addresses**, not to an assertion that the two spellings agree. **An assertion that
the names agree is exactly the enshrining move and is refused here.**

Every other site — S1–S8, S10, S12 — has a measuring fix. That is a better result than the
survey's F-class, where `bganim_room.py`'s `LAST_PACKED_LABEL` needed §6(c)'s max-LMA
construction to avoid enshrining the accident.

**The one place the maintenance-act hazard appears as a precondition rather than a fix:**
correcting `s4lint.py`'s `SST_FIELDS` requires editing `tools/test_s4lint.py`, whose docstrings
assert the stale offsets as expected behaviour. That edit — changing a test's expectations to
match a changed tool — is shape-indistinguishable from weakening a gate to hide a defect.
**Flagged rather than proposed.** It is discharged the right way by deriving the table and letting
the test derive its expectations the same way, so neither side carries a typed offset; a hand-edit
of the expected numbers is refused. The same applies verbatim to S2 and `test_state_ram.py`.

**S11's two items are not fixable in the file that carries them at all** — they are placement
rules living in a comment, about a `map.toml` row and a sibling gate respectively. Recorded, not
proposed.

---

## §4 — The pytest-lane verdict

Aeon reported ~20 placement-shaped files reading committed cuts of past listings, so that no
re-derivation can redden any of them. **Verified, quantified, and the mechanism is stronger and
more deliberate than the relay conveyed.**

### The enumeration

```
grep -lE '\.lst\b|\brom\[|0xFFFFFF|routine_extent|NEXT_SYM|syms\[|labels\[|_addr\b|lma' \
     tools/test_*.py                                                        →  22
grep -lE 'fixtures/' tools/test_*.py                                        →   9
git ls-files 'tools/test_*.py'                                              →  81
```

22 placement-shaped of 81 total. Aeon's "20" and my 22 are the same finding at a different
threshold.

### The structural proof, which is stronger than the count

Established firsthand from `build.sh`'s control flow (it defines **no shell functions** — grep
for `^[a-z_]+\(\) *\{` returns nothing — so line order is execution order):

- the pytest lane is invoked at **line 612** (`python3 -m pytest "${TOOLS}"`);
- `"${SIGIL_BUILD}" build --aeon .` is invoked at **line 775**.

**The pytest lane runs strictly before the ROM and listing this invocation produces.** No test in
the lane can observe this build's placement, whatever it reads. That is not an enumeration and it
cannot be defeated by a file I missed.

### It is a deliberate design decision, not an oversight — and it was made after a real defect

`build.sh`'s own comment block records why, and it corrects the framing the relay carried:

> LISTING-READING GATES RUN POST-SIGIL ONLY (2026-08-26). A gate that reads a `.lst` must read
> the one THIS invocation emitted: the pre-build pytest lane used to re-derive the BG-animation
> ceiling from whatever `s4*.lst` a prior build had left on disk, and that listing was twice not
> the subject (another sigil profile's; then absent on a fresh tree, so a first canonical build
> failed its own pre-build lane). The pytest lane now tests the DERIVATION over a committed
> listing cut.

Four separate test files repeat the rule in their own docstrings (`test_dplc_straddle.py`,
`test_plane_role_swap_gate.py`, `test_reels_witness.py`, `test_instashield_gate.py`,
`test_sprite_tilt.py`). So the blindness is **designed, documented, and compensated** — the
listing-reading half of each gate runs post-sigil from `build.sh`, with `--built-after` and
`--fixture` provenance checks.

### Is there a re-deriving test? One, and it does not change the conclusion

`test_demo_specialization_witness.py::TestAgainstARealListing` opens the tree's `s4.debug.lst`
and re-derives from it. It does **not** change the verdict, for three independent reasons:

1. It is `@unittest.skipUnless(os.path.isfile(...))` — on a clean tree it skips.
2. Because the lane runs at line 612 and the build at line 775, the listing it finds is a
   **previous** build's, which is the exact defect the 2026-08-26 comment was written about.
3. Its assertion is `assertNotEqual` on an adjacency-derived difference (S6), which a re-layout
   makes vacuously true rather than red.

`test_effects_gates_segments.py` also opens real artifacts, but under `pytest.skip` and about
JSON transport, not placement.

**Verdict: the pytest lane cannot redden on a re-layout, the reason is structural rather than
incidental, and it is the deliberate consequence of a fix to a worse defect.** The correct ask is
not "make the pytest lane placement-aware" — that would re-create the stale-listing bug — but
"confirm every placement derivation has a post-sigil arm in `build.sh`". Two of the four
`routine_extent` consumers (S3) do; `waterline_art_gate.py` and `instashield_gate.py` are both
invoked post-sigil, so all four are in fact on the post-build side. The gap is the phased-symbol
filter, not the lane.

### The one test-lane defect that is real

**`tools/test_s4lint.py` and `tools/test_state_ram.py` each consume the very constant they would
have to falsify** (S1, S2). These are not blind-by-design; they are green gates locking in wrong
values. That is a different and worse condition than "reads a committed cut", and it is the
pytest lane finding that matters.

---

## §5 — Behaviour established vs presence only

| finding | invocation established how | class |
|---|---|---|
| S1 `s4lint.py` | read `build.sh`: invoked under `if [[ "${NO_LINT:-0}" == "0" ]]`, hard `exit 1` on failure; skipped by `-nl`/`--no-lint` and by `FAST=1` | **behaviour** (control flow read; the tool was not run) |
| S1 blast radius | `git ls-files '*.asm'` → 3 files; `grep -rn "SST_" --include='*.asm' .` → **exit 1, genuine no-match** | **behaviour** |
| S2 `state_ram.py` | **not** in the 24 invoked from `build.sh`; reached by the pytest lane via `test_state_ram.py` | **behaviour** for the lane; **presence** for any other caller |
| S3 the four `routine_extent`s | all four host gates are among the 24 invoked from `build.sh`, post-sigil | **behaviour** |
| S3 the two measured truncations | `scene_spans.vma_phased_symbol_names`'s docstring, dated 2026-09-03 | **relayed from aeon's own source comment** — not reproduced here (no build) |
| S4 `demo_drift_classifier.py` | **not** invoked from `build.sh` | **presence only** |
| S5 the three probes | **not** invoked from `build.sh`; emulator-driven | **presence only** |
| S6 the witness test | in the pytest lane sweep (`find "${TOOLS}" -maxdepth 1 -name 'test_*.py'`) | **behaviour** for collection; **skip-gated** for the arm itself |
| S7–S11 `effects_gen.py` | read `build.sh`: `python3 "${TOOLS}/effects_gen.py" check`, inside `if [[ "$FAST" == "0" ]]`, `exit 1` on failure. **Not** gated on `GAME` — `generate()` hardcodes `"sonic4"`, so `./build.sh demo` still checks the sonic4 tree. Output is committed and byte-compared in memory; only `regenerate-level.sh` runs the writing verb | **behaviour** (control flow read; the tool was not run) |
| S7 the prose-anchor count | `grep -c "patchable(" ojz_effects.emp` → **12**; of those, `grep -c "//"` → **8** | **behaviour** |
| S8 the baked line numbers | read out of the committed `games/sonic4/data/generated/effects_channel_bands.json` | **behaviour** |
| S11 the `map.toml` mitigation | `"section:ojz_effects_editor_act1"` present in `games/sonic4/map.toml` | **behaviour** |
| pytest lane ordering | `build.sh` line 612 vs 775, no shell functions in the file | **behaviour** |

**Nothing in this note was confirmed by running an aeon tool or building a ROM.** Two claims want
runtime confirmation and are **TAGGED for foreground follow-up**: (i) where the four
`routine_extent` subjects actually land relative to `$85AD` in a current listing, which decides
whether S3 is live today or latent; (ii) whether `s4lint.py`'s E009 arm has any subject in any
non-canonical shape.

**Two of the s4lint and effects_gen sites were read by delegated subagents; the load-bearing
claims of each were re-verified at this seat before being written down** — S1's stale offsets
against `sst.emp` field by field, S1's blast radius by `git ls-files` + an exit-checked grep, S7's
12-vs-8 count, S8's committed line refs, S10's `REPO`-not-`repo`, S11's `map.toml` row. One
subagent claim was **corrected in the writing**: S10's fix is not "thread `repo`" —
`render_table_ref`'s signature carries no `repo` parameter, so the caller has to supply one.

---

## §6 — What I concluded was wrong in the brief

**1. "~20 placement-shaped test files that read committed cuts of past listings" understates the
mechanism and mis-locates the fault.** The count is right (22 by my axes, 9 touching a fixture).
But framing it as *"a whole gate lane that cannot fail for the reason it exists"* is not what the
tree shows. The lane never existed to catch re-layouts; it exists to test derivations, and it was
**deliberately moved to that footing on 2026-08-26 after the listing-reading version produced two
false results**. The listing-reading arm of each gate lives post-sigil in `build.sh` and does
fire. Reported as a corroboration with a corrected mechanism, per *a peer's measurement vs their
mechanism*.

**2. The brief's framing that Class F/G "may mis-describe what you find" — it does not; but it is
incomplete in one direction.** F (terminus proxy) and G (adjacency pin) both describe a position
inferred from a name. This pass found a third member of the family the survey has no class for:
**a hardcoded name→VALUE table standing in for a layout a different file authoritatively owns**
(S1's `SST_FIELDS`, S2's `FALLBACK_SYMBOLS`). It is neither a terminus nor an adjacency; it is a
*mirror*, and its distinguishing property is that it is **already wrong** rather than
conditionally wrong. F and G go wrong when the layout moves; a mirror is wrong the moment the
authority changes and nobody notices, which has already happened at S1. Suggest **Class M** if
this list is extended.

**3. The seven-gates read yielded less than the survey predicted, and the survey said it would
yield more.** §3 says *"a full read would very likely surface more of that class."* It surfaced
exactly one new G5 site (`waterline_art_gate.proc_span`) across the seven. The three
`_golden`/`row_remap`/`editor_palette` gates are clean of name-as-position: `row_remap_gate`
re-derives its ladder from the generator module and refuses (`Unmeasurable`) rather than skipping
when it cannot, which is the shape §6(c) asks for. The productive files were the ones the survey
never named at all.

**4. The parcel's parameter needs one axis it did not state, and half of `effects_gen.py`'s yield
falls on the other side of it.** "A position inferred from a name" turns out to cover **two
different kinds of position**, and only one of them is what DECOUPLE is about:

- **ROM/RAM-layout position** — what a re-layout moves. S1–S6, S9, S11 are these.
- **Source-text position** — a character window, a line number, a textual adjacency in an `.emp`
  file. S7, S8, S10's `section_preset_symbols` are these.

The second class is a real defect class and shares the shape exactly (an unverified inference
about where something sits), but **a decouple of the frozen tables cannot trigger any of it.**
Reported in full because the survey's question was *"what does this code compute that nothing
verifies"* and these answer it — but they should be routed as ordinary tool defects, not as
DECOUPLE preconditions, and a future pass should state which axis it is enumerating. Getting this
wrong in the other direction would inflate the precondition's cost with work that does not
discharge it.

**5. One brief-level framing I would restate.** The brief says the twelve F/G rows were found "in
the aeon tools that were read in full" and that "the tools read only by docstring are the same
kind of code". Half true: the seven docstring-only gates yielded one new site, while the four
files the survey **never named at all** (`scene_spans.py`, `state_ram.py`,
`demo_specialization_witness.py`, `demo_drift_classifier.py`) yielded four. **The predictor of
yield was not "how thoroughly was it read" but "was it in the enumerated set at all."** A list of
what a survey skimmed is a much weaker guide to where the residue is than the complement of what
it enumerated.

---

## §7 — What I left open, and why

- **No ROM built, no tool run.** Deliberate — the brief scoped this as a source read and barred
  the emulator. The two runtime questions are tagged in §5.
- **`effects_gen.py` was read in full (4,632 lines) and is covered by S7–S11.** What it does
  **not** contain, established by that read plus exit-checked greps: no terminus proxy, no
  adjacency pin, no symbol table or listing read of any kind, no hardcoded ROM address, no
  `& 0xFFFFFF`, no emitted `Target − Base` offset table, and no generated symbol name that encodes
  a position. Its yield is the source-text class of §6.4 plus the two comment-only placement rules
  of S11.
- **The 92-file population was filtered by mechanism, then read selectively.** The emulator
  witnesses and probes (roughly 45 of the 92) were swept with the differencing and
  literal-name-lookup greps in §2/S5 but not read in full. A witness that infers a position some
  other way would not have surfaced. The greps' exit statuses were checked; none returned an
  error, so the empties are genuine no-matches.
- **`suite_paths.py`, `collision_consistency.py`, `emp_expect_fail.py`,
  `gen_compression_vectors.py`** — the four non-gate files in the survey's docstring-only list —
  were not read in full. Only `gen_compression_vectors.py` carries an address-shaped constant
  (`POISON_WORD = 0xA5A5`, a fill pattern, not an address).
- **`zyrinx_player.py` carries three hardcoded ROM addresses** (`SEQ_PTR_TABLE = 0x1F0F37`,
  `SONG_ADDR = 0x1F0BDF`, `VOICE_PTR_TABLE = 0x1F49A8`). Read and set aside: they address a
  **donor** ROM, not aeon's, so no aeon re-layout can move them. Stated so a future sweep does
  not re-find them as a hit.
- **RAM-layout name-as-position was surveyed but not enumerated.** S2 and S5 are RAM-side and
  surfaced by ROM-shaped queries. A dedicated RAM pass would likely find more; the decouple
  precondition is about ROM placement, so it was not run.

---

## §8 — The routing summary

| # | site | build path? | failure | structurally fixable | fix owner |
|---|---|---|---|---|---|
| S1 | `s4lint.py` `SST_FIELDS`/`SST_LEN` | **yes**, hard gate (skipped by FAST/`--no-lint`) | silent + loud-misattributed; **already wrong**; latent (0 subjects) | yes — derive from `sst.emp` | aeon |
| S2 | `state_ram.py` `FALLBACK_SYMBOLS` | pytest lane only | silent | yes — delete, refuse without `--lst` | aeon |
| S3 | four `routine_extent`s missing the phased filter | **yes**, all four post-sigil | **loud-but-misattributed** | yes — reuse `scene_spans.vma_phased_symbol_names()` | aeon; **sigil ask**: mark phased rows in `emit_listing` |
| S4 | `demo_drift_classifier.py` `EndOfRom` NOTE | no | silent | yes — raise instead of note | aeon |
| S5 | 3× `Parallax_Shadow_*` span pin | no | silent | yes — declare an `_End` symbol | aeon |
| S6 | `test_demo_specialization_witness.py` collision signature | pytest lane, skip-gated | silent (vacuous pass) | yes — assert the positive property | aeon |
| S7 | `effects_gen.walk_patch_sites` 400-char window over unstripped comments (8 of 12 anchors are prose) | **yes**, `FAST=0` | loud-misattributed **and** silent | yes — balanced parens + strip comments | aeon |
| S8 | `effects_gen` source line numbers in a byte-compared artifact; `raster.emp:397` hardcoded | **yes**, `FAST=0` | loud-misattributed; the `:397` is silent | yes — drop `:line`, derive the marker | aeon |
| S9 | `effects_gen` reels rung-2: **pointer identity by name-string equality** | **yes**, `FAST=0` | silent | **HALF ONLY** — the identity half needs linked addresses; see §3 | aeon |
| S10 | `effects_gen` × 3: alphabetical duplicate-channel winner; textual `sec:` adjacency; `REPO`-not-`repo` | **yes**, `FAST=0` | silent | yes (all three) | aeon |
| S11 | `effects_gen` comment-only placement rules: the `map.toml` section-row dependency, and reel emission sited to keep `reels_gate` measurable | **yes**, `FAST=0` | silent — `reels_gate` goes **`Unmeasurable`, not FAIL** | **no**, not in this file | aeon (route to the map / the gate) |
| S12 | `s4lint.py` `check_e006` name match + dead `_VDP_WRITE_TARGETS` | yes | silent | yes — resolve, don't match | aeon |

**Twelve sites. Which of them DECOUPLE actually needs:** S1–S6, S9 and S11 are layout-position
(§6.4's first axis); S7, S8 and S10's two source-text members are not, and should be routed as
ordinary tool defects rather than as preconditions.

**S3 is the one to route first.** It is the only finding that is on the shipped gate surface, has
a demonstrated failure (two measured truncations, dated in aeon's own source), fails in the worst
category, and has its fix **already written in the same tree** waiting to be consumed by four
callers. It also carries the pass's one **sigil-side ask**: `emit_listing` writes one
`(0) N/HEXADDR : Name:` row for every symbol with no field saying whether the address is a VMA or
an LMA, which is why five separate aeon tools each had to re-derive the distinction from source
and only one of them did. A phased-row marker in the listing retires the whole class.

**S9 is second, and for a different reason:** it is the only site whose fix is not wholly aeon's,
and the only one where the tempting fix is the forbidden one.

## Verified at the landing — what the overseer checked firsthand, and one wrong doubt

Three claims were re-derived at this seat before this note was accepted, because two of them
correct a mechanism that reached this lane through a relay and one asserts a live defect in
another repo's tool.

**1. The pytest-lane mechanism — CONFIRMED, and it corrects the relayed framing.** At aeon
`305af222`, `build.sh` runs `python3 -m pytest "${TOOLS}"` at **line 612** and the post-build
listing gates under `--built-after` at **line 772+**. So no test in that lane can observe this
build's placement whatever it reads. And it is deliberate: `build.sh`'s own header (lines 64-74)
records that the pre-build listing-reading version *"was twice not the subject"* and was moved to
committed listing cuts for that reason. **Aeon's "structurally blind to placement" is right about
the reading and wrong about the implication** — this is a documented design decision with the
real gates elsewhere, not an accidental blindness.

**2. `s4lint.py`'s `SST_FIELDS` is already wrong — CONFIRMED, after a wrong doubt worth
recording.** The first check ran against the head of the table and found six offsets *matching*
`engine/objects/sst.emp` exactly, which read as a refutation. It was not: **the table agrees
through `art_tile @$14` and diverges from `$16` onward.** `SST_priority @$16` names a field the
struct does not have; the struct's `$16`/`$17`/`$18` are `width_pixels`/`height_pixels`/`anim`,
each of which `s4lint` places two bytes late. **The sample was drawn from the agreeing prefix** —
a table that is right for nine rows and then shifts is exactly the shape a head-sample clears.
Sample the tail, or diff the whole table, and never accept a prefix as the population.

**3. The latency argument — CONFIRMED, with the exit status checked.**
`git grep -l "SST_" -- '*.asm'` at `305af222` **exits 1 with zero matching files**, against
**3** tracked `.asm` files in the whole tree. The defect is real and has no live subject today.
*(First attempt piped that grep into `head` and read `$?` from the pipeline — measuring `head`,
not the grep. The rule this lane already carries: an absence is only a finding once the exit
status came from the command you meant.)*

**Not verified here, and left as the agent's measurement alone:** the four-versus-three
`routine_extent` count, the field-by-field totals in S1 (six stale / four phantom / seven
unlisted — the shape is confirmed, the counts are not), and the S9 name-equality reading.
