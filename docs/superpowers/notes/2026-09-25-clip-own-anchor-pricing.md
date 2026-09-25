# 2026-09-25: pricing a shape-conditional bank anchor pair for aeon's S2CLIP shape (sigil half)

Research and pricing only. Nothing here is implemented. Asked by the aeon lane under the hub
ruling at empyrean e229e1a3 (docs/OVERSEER-LOG.md, "HUB RULING ... aeon S2 clip DEBUG shape
short of the growth reserve"): the S2CLIP shape gets its OWN `dac_banks`/`sound_bank` pair,
the canonical shapes' anchors and bytes untouched. Question priced: what sigil needs, at what
size, to accept an anchor that applies only when the S2CLIP shape is built.

Sources read: sigil at 65f5337d (this branch's base); aeon through git objects only, at
`origin/master` 7ea4fd37 (`git -C .../aeon show origin/master:<path>`). No build was run.
Every claim is labelled MEASURED (command given, a read of source counts as the measurement
of what the source says) or INFERRED (a consequence reasoned from that source, not run).

## Verdict in one paragraph

Sigil does NOT need nothing. Today sigil cannot tell the S2CLIP build from canonical sonic4:
build.sh passes it the identical flags, and every input that places or predicts the banks is
either compiled into the binary (the frozen island rows) or read from map.toml by anchor name
with no shape context (the whole seam-2 sound derivation). The recommended design (A below) is
a map-declared `variant` key on `[[anchor]]` rows, a `--map-variant <name>` build flag, and
threading that variant through the island provisional bases, `validate_placement` and seam-2.
Size **M** (about 400 to 500 lines including tests, touching 6 source files). No canonical
byte can move and no refreeze is needed, provably (proof plan below). It is owner-facing
surface: one new map.toml key and one new CLI flag.

## 1. Where anchors are declared and consumed today

**Declared (aeon).** `games/sonic4/map.toml` at origin/master (MEASURED:
`git -C /home/volence/sonic_hacks/aeon show origin/master:games/sonic4/map.toml`):

- lines 352-361: `dac_banks at = 0xA8000 when = "sound_on"`, `sound_bank at = 0xB8000
  vma = 0x8000 when = "sound_on"`. Plus `boot_head 0x0` and `object_bank 0x10000`
  (lines 181-187), no `when`.
- lines 189-255: the BANK PLACEMENT RULE. lines 257-333: the file's own statement that an
  island's address is written in two places (this `at` and sigil's frozen row) and both
  must move together.

**Parsed (sigil).** `crates/sigil-harness/src/map_placement.rs` (MEASURED: `sed -n 1,200p`):

- `Anchor { name, at, vma, when }` (18-27); `AnchorDoc` (110-119) has no
  `deny_unknown_fields`, so an unknown key on an anchor row is silently dropped.
- `anchors_for(sound_on)` (64-66) filters by `when_applies` (91-98):
  `None => true, "sound_on" => sound_on, "sound_off" => !sound_on, Some(_) => true`.

**Consumed (sigil), five places, all keyed only on `sound_on` or on the aeon path:**

1. `native.rs:4146` `anchor_addrs = pmap.anchors_for(profile.sound_on)` feeds
   `packed_true_bases` (2914-3080), where a labelled section is an island iff its FROZEN
   provisional base is in that set (`is_anchor_gap`, 2929 and 3024-3031); a phase bank head
   (`SoundTablesZ80_Head`, vma $8000) is held at its frozen base regardless (3006-3015).
2. `native.rs:3689-3745` `validate_placement`: `[map.undeclared-island]` and
   `[map.anchor-absent]`, keyed by ADDRESS (the in-code ledger at 3705-3719 says names are
   decorative here).
3. `native.rs:3813, 3886` holes, same filter.
4. `seam2.rs:181-226` `bank_anchors(aeon)`: reads map.toml from disk and finds
   `dac_banks`/`sound_bank` by NAME via `anchors_for(true).find(..)`, first match wins.
   `sound_layout(aeon)` (272ff.) derives every bank LMA from those two and is memoized per
   aeon PATH (273-279). 15 call sites in seam2.rs, 3 in seam1.rs (191, 1320, 1332: the
   resident Z80 blob's bank id and DacSampleTable window), 1 in native.rs (3637,
   `validate_sound_fold`) (MEASURED: `grep -c 'bank_anchors(aeon)\|sound_layout(aeon)'
   seam2.rs` = 15; `grep -n 'seam2::(banked_head_vmas|sound_bank_id|dac_sample_table_vma|
   sound_layout)(' seam1.rs native.rs`).
5. `emit_generated` (native.rs:1212-1230) runs every sound-on build before the AS residual
   and writes the seam-1/seam-2 bins into `aeon/engine/sound/generated`, all folded against
   `sound_layout(aeon)`.

**Existing conditional mechanisms, all checked:**

- `when` exists but has exactly two live values and both are derived from
  `profile.sound_on`. An unknown value FAILS OPEN (applies to every shape).
- The CLI has no build define. `parse_build_args` (sigil-cli/src/main.rs:2695-2790) accepts
  `--game --debug --config-a/-b --lean --stress-evict --stress-art --extra-entry --check
  --report` and refuses anything else. `-D` exists only on `sigil x` (main.rs:3315).
- `game-defines` (`[defines]` in map.toml, native.rs `shape_defines` at 1016) is a
  per-GAME merge into every shape's define env: it cannot select anything per shape.
- The off-canonical profile precedent exists: `stress_evict_profile`/`stress_art_profile`
  (native.rs:937-966), each a CLI flag, unfrozen, no golden, absent from `shipped_shapes`.
- **S2CLIP reaches sigil as plain canonical sonic4.** aeon build.sh at origin/master:
  1250-1262 sets `NATIVE_FLAGS="--game ${GAME}"` (+`--debug`) for S2CLIP exactly as for
  canonical, and 1289 is the one `sigil build` call (MEASURED: `git ... show
  origin/master:build.sh | sed -n 1240,1300p`). The S2CLIP difference is only aeon's
  throwaway re-bake of the act slot (1039-1074). So sigil builds S2CLIP from the canonical
  `s4.txt`/`s4_debug.txt` frozen rows today.

**Decouple step 2** would not make this aeon-only. It is not started (docs/OVERSEER.md:
698-700, "Not started ... step 2 is aeon's ROM-RELAYOUT"), and even after it, sigil still
needs to be TOLD which shape it is building, because S2CLIP's invocation is byte-identical to
canonical's. (INFERRED from the two facts above.)

## 2. The frozen table, and whether a refreeze is needed

MEASURED (`ls crates/sigil-harness/golden`, `grep` of the named files):

| artifact | knows about banks? | knows shapes? | needs a row for S2CLIP? |
|---|---|---|---|
| `golden/offcanonical_sizes/s4.txt`, `s4_debug.txt` | YES: `Dac_Temp_Blip 0xa8000`, `SoundTablesZ80_Head 0xb8000` are the island PLACEMENT (provisional base = held base) | one table per shipped profile, loaded by a path compiled in via `CARGO_MANIFEST_DIR` (native.rs:232-252) | not under design A (the variant rewrites two rows in memory); yes under design B |
| `src/pins.rs` (from `repin.toml`) | `DAC_BANKS` 0xA8000, `SOUNDBANKHEAD` 0xB8000 (pins.rs:276-286) | plain/debug of the canonical s4 listings only | no: its bank pins are test oracles over canonical listings. The one pin a build path reads is `BOOT_HEAD` (seam1.rs:67, the resident blob LMA), which the clip re-bake of the act slot does not move (INFERRED) |
| `golden/provenance.toml` | no | s4, s4_debug, demo, demo_debug, config_a, config_b | no |
| `src/section_align.rs` | `Dac_Temp_Blip` and `SoundTablesZ80_Head` require 0x8000 (124-125) | shape-independent | no: 0xB8000 and 0xC8000 satisfy it |

No artifact on sigil's side knows about any non-canonical shape except the two stress
profiles, which carry no frozen artifact at all. **Under design A no refreeze is needed and
nothing is byte-moving for the canonical shapes**: their frozen tables, pins, goldens and
provenance are untouched, and their anchor set is unchanged by construction (proof plan in 4).
The variant shape is unfrozen, exactly like `stress_art`. (INFERRED from the table.)

## 3. The arithmetic, verified

The rule (aeon map.toml 199-211, tools/bganim_room.py 247-258, MEASURED by `grep -n`):

    dac_banks  = align_up(packed_end + DATA_GROWTH_RESERVE + DATA_GROWTH_GRACE, BANK_ALIGN)
    sound_bank = dac_banks + 2 * 0x8000
    RESERVE = 0xC000, GRACE = 0x8000, BANK_ALIGN = 0x8000

**The 0x8000 term in `align_up(0xA074C + 0xC000 + 0x8000, 0x8000)` is `DATA_GROWTH_GRACE`**
(one SetBank window, added 2026-09-04), and the trailing 0x8000 is `BANK_ALIGN`: two
different constants that happen to share a value.

Recomputed (MEASURED: `python3 -c` over the rule, shown in the session log):

    0xA074C + 0xC000 + 0x8000 = 0xB474C  ->  align_up = 0xB8000   (dac_banks, bank $17)
    sound_bank = 0xB8000 + 0x10000 = 0xC8000                     (bank $19)
    room today under 0xA8000: 30,900 B, short of the 49,152 B reserve by 18,252 B
    room at the new anchor:   96,436 B
    (reserve-only rule would give 0xB0000; canonical 09-04 check reproduces 0xA8000)

Aeon's 0xB8000 is right. The sound_bank the ruling implies is **0xC8000**, which the brief
did not state. The packed end 0xA074C itself is aeon's measurement; I did not re-measure it
(it needs an S2CLIP build). Note that the clip's `dac_banks` 0xB8000 equals canonical's
`sound_bank` address: harmless per shape, but it is the collision the substitution trap in
design A must handle.

## 4. Candidate designs

### 0. "Sigil needs nothing" (REFUTED)

What aeon alone could do, and why each fails (INFERRED from the source above, not run):

- Add a second pair with `when = "s2clip"`: `when_applies` returns true for an unknown value,
  so the clip pair applies in EVERY shape. Canonical then declares 0xC8000, nothing lands
  there, `[map.anchor-absent]` fails every canonical sound-on build. Loud, not silent, but a
  dead end. seam-2's `.find` would additionally pick whichever `dac_banks` row comes first.
- Rewrite map.toml's two `at` values inside the S2CLIP throwaway re-bake: seam-2 follows the
  file, but `Dac_Temp_Blip`'s frozen provisional base is still 0xA8000, no longer an anchor,
  so it packs to align_up(0xA074C, 0x8000) = 0xA8000 and `validate_placement` infers an
  island there (gap 0x78B4 > ANCHOR_GAP) that is not declared: `[map.undeclared-island]`.
  This is the same failure aeon measured on 2026-09-04 (map.toml 320-322). The frozen rows
  are compiled into the binary and `sigil build` has no override (map.toml 306-311).

Also worth stating plainly: **nothing in sigil blocks the S2CLIP build today.** It builds at
the canonical 0xA8000 with 30,900 B of room (INFERRED: aeon read 0xA074C from that shape's
sigil listing, which exists only if the build succeeded). The failure is aeon's policy gate
`bganim_room.py --gate`. The sigil work exists only to honour the ruling's chosen remedy.

### A. Map `variant` key + `--map-variant` flag (RECOMMENDED), size M

Surface:

- map.toml grammar (aeon's file, sigil's reader): `[[anchor]]` gains optional
  `variant = "<name>"`. Rule: a variant row applies ONLY when the build names that variant,
  and in that build it REPLACES the base row of the same `name`. Base rows keep their text
  exactly, so the canonical pair is untouched literally, not just in bytes. Aeon would add:

      [[anchor]]
      name = "dac_banks"
      at = 0xB8000
      when = "sound_on"
      variant = "s2clip"
      # and sound_bank at = 0xC8000, vma = 0x8000, same when/variant

- CLI: `sigil build --game sonic4 [--debug] --map-variant s2clip`, and the same flag on the
  `emit_sound_blob` binary. Refused when no map row declares that variant (a typo must not
  silently build canonical), refused with `--config-*`/`--lean`/`--stress-*`.
- Recommended hardening, in the same parcel: `when_applies` refuses an unknown value, and
  `AnchorDoc`/`HoleDoc` get `deny_unknown_fields`. Both are byte-neutral today (aeon's two
  maps use only `sound_on`/`sound_off` and only the keys name/at/vma/when and
  after/at/filled_by/when; MEASURED by reading both maps). Without them, a map that gains the
  `variant` rows before a sigil that reads them fails open in the way design 0 describes.

Files (MEASURED line counts where given):

- `crates/sigil-harness/src/map_placement.rs` (248 lines): field, parse, `anchors_for(sound_on,
  variant)` with replace-by-name, refusals (variant row with no base twin, two variant rows of
  one name, unknown `when`), unit tests. About 100-130 lines.
- `crates/sigil-harness/src/native.rs`: `GameProfile.map_variant: Option<String>`, a builder
  like `with_extra_entries`; thread into `anchors_for` at 3724, 3739, 3813, 3886, 4146,
  4244/4312; and **rewrite the frozen island rows in memory for the variant**: for each
  variant anchor that replaces a base anchor, the frozen row whose value equals the base `at`
  (exactly one, else refuse) takes the variant `at`. About 80 lines.
- `crates/sigil-harness/src/seam2.rs` (1,377 lines) and `seam1.rs`: `bank_anchors`/
  `sound_layout` take the variant; the memo key becomes (path, variant). Keep
  `sound_layout(aeon)` as the canonical wrapper so the 14 sigil-cli test files that call it
  (MEASURED: `grep -rln 'sound_layout(' crates/sigil-cli/tests | wc -l` = 14) do not churn.
  About 100 lines.
- `crates/sigil-cli/src/main.rs` flag + parse tests; `crates/sigil-harness/src/bin/
  emit_sound_blob.rs` flag. About 60 lines.

Two traps the tests must pin (INFERRED from the source, and the reason these are named):

1. **Sequential row substitution double-moves `Dac_Temp_Blip`.** Clip `dac_banks` 0xB8000
   equals canonical `sound_bank` 0xB8000. Rewriting dac first (0xA8000 to 0xB8000) and then
   "every row equal to 0xB8000" to 0xC8000 moves BOTH heads to 0xC8000. The substitution
   must be computed against the original table in one pass.
2. **The seam-2 memo is keyed by path only.** A process that derives canonical then variant
   (every test binary that builds both) would silently reuse the first layout and fold every
   sound pointer against the wrong bank. `validate_sound_fold` would catch the MT/SFX bases,
   but not the DAC bank ids baked into the resident blob.

Proof that canonical bytes cannot move, and that the variant row is actually consulted:

- All seven `shipped_shapes` byte-identical to their goldens with a strict paired-tree run
  (the existing golden full-file gates), with aeon's map CARRYING the variant rows.
- Mutation M1: set the variant rows' `at` to 0xD0000. Pass looks like: canonical shapes still
  byte-identical, the variant build fails `[map.anchor-absent]`. If the variant build still
  succeeds, the row is not consulted.
- Mutation M2: make the variant filter always apply. Pass looks like: every canonical sound-on
  build fails. If canonical still builds, the filter is not what protects it.
- Unit: the 0xB8000 collision case gives `Dac_Temp_Blip` 0xB8000 and `SoundTablesZ80_Head`
  0xC8000; a same-process canonical, variant, canonical derivation returns three layouts
  with the first and third equal and the middle different.
- End to end (aeon lane, needs the clip bake): `s4.s2clip.debug.lst` shows `Dac_Temp_Blip`
  at 0xB8000 and `SoundTablesZ80_Head` at 0xC8000, `[sound.fold-vs-placement]` silent,
  `bganim_room.py --gate` green. Sigil cannot run this without building aeon.

What settles the size: the prototype's diffstat and the count of `anchors_for` and
`sound_layout` call sites it had to change; the estimate above assumes 7 and 19.

Expected side effect: the downstream frozen rows (`Song_MovingTrucks` onward) keep their
canonical provisional bases, so they will pack about 0x10000 above them and raise
`[layout.provisional-drift]` warnings in the variant shape. Warnings, never fatal
(native.rs:2982-2994). The clip shape very likely warns already for its grown act data
(INFERRED, not measured).

### B. A dedicated `--s2clip` profile with its own frozen tables, size M+, not recommended

`s4_s2clip.txt`/`s4_s2clip_debug.txt` in `golden/offcanonical_sizes`, a `stress_art`-style
profile. It still needs all the seam-2 threading from A, because seam-2 reads the anchors
from map.toml by name. It re-creates the two-place authority the decouple plan is retiring,
for a shape with no refreeze cadence, behind a table aeon cannot regenerate (the path is
compiled in). It also hard-codes an aeon dev-shape name into sigil.

### C. Islands from the map for a flagged build, map rewritten by aeon's re-bake, size S/M, runner-up

Sigil adds a flag under which `Dac_Temp_Blip` and `SoundTablesZ80_Head` (the heads seam-2
already names in `SOUND_BANK_ORDER`, seam2.rs:63) take their provisional base from the map's
`dac_banks`/`sound_bank` instead of the frozen table. Aeon's S2CLIP bake rewrites the two
`at` values in map.toml under its EXIT trap. seam-2 needs no threading because it reads the
rewritten file. About 150 lines. The cost is that the clip pair never exists as a committed,
reviewable declaration (it lives in a script's sed), and the listing's source digest records
a map.toml that git does not have. It is also a partial first step of decouple step 2, which
belongs to aeon's queue. Choose it only if the pair being committed in map.toml is not what
the ruling meant.

## 5. Recommendation

**Design A.** It is the only one where the clip pair is a committed map.toml declaration, the
canonical rows keep their text, and the canonical build's anchor set is unchanged by
construction rather than by care. Size M. Owner-facing: yes, one map.toml key (`variant`) and
one CLI flag (`--map-variant`), so it should go past the owner or hub before landing. Sequence:
sigil lands first (grammar, fail-closed `when`, `deny_unknown_fields`), aeon adds the rows and
passes the flag from build.sh's S2CLIP branch second. Aeon's half also includes
`bganim_room.py`, which reads the anchor by name (`anchor_addr`, line 338) and would need the
variant too.

## 6. Things in the brief I concluded were wrong or incomplete

1. "Is there ANY existing conditional mechanism": there is one, `when`, but it is fixed to
   `sound_on`, and an unknown value fails OPEN. A naive aeon-only `when = "s2clip"` would
   apply the clip pair to every shape.
2. "Placement authority is moving to aeon's map.toml (decouple step 2), which may make this
   mostly aeon-side": step 2 is not started, and it would not remove the need. S2CLIP invokes
   sigil with flags identical to canonical, so sigil must be told the variant whatever owns
   placement.
3. The ruling frames S2CLIP as a shape whose anchor could be conditioned, but to sigil it is
   not a shape at all today. It is canonical sonic4 over re-baked data, placed from the
   canonical frozen rows.
4. Of the four "frozen table" candidates, only `offcanonical_sizes/*.txt` holds island
   placement. `pins.rs`/`repin.toml` is a test oracle, `provenance.toml` records goldens, and
   `section_align.rs` is shape-independent and already satisfied.
5. The brief gives only the `dac_banks` figure. The pair is 0xB8000/**0xC8000**.
6. Nothing in sigil blocks the S2CLIP build now. Only aeon's reserve gate fails.
7. Incidental, sigil's own: the `stress_art_profile` doc comment (native.rs:955) still
   says the DAC and sound phase banks are at $48000/$58000. They have been at
   0xA8000/0xB8000 since 2026-09-04.
