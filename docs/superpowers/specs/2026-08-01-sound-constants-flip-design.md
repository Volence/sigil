# The sound-constants flip (census row-59, reclassed CAPSTONE) — design

**Status: RATIFIED (overseer, path-1 ruling under Volence's standing "do it
properly" precedent — conv-C/item-7 shape, applied here without a re-ask since
no language feature is missing; composition scale only).** Spec owner: Fable.
Basis: the Parcel-E inspection stop
(notes/2026-08-01-conv-e-sound-constants.md) — read it first; its numbers are
the premise of record.

## §1 — What this is and why

`engine/sound_constants.asm` (1481 ln) is the last AS-authored definition
carrier in the engine: 321 `=` (flat + struct-derived aliases), the FIVE Z80
structs (DacSample/FmPatch/SfxHeader/SfxChannel/SeqChannel, with the 13-field
SeqChannel↔SfxChannel shared-prefix invariant), a derived Z80 RAM layout
(~40 computed addresses = flat bases × `sizeof(struct)`), ~40 `error`/`fatal`
walls, and 5 comptime `function`s. The REAL prize is not the file: it is the
**seam1/seam2 hardcoded mirror** — 399 hand-maintained `(name,value)` entries
across five tables feeding `emit_sound_blob`'s resident-Z80 module builds, plus
seam2's DAC-head carriers — TRIPLE-copied with the .asm and the 33 68k-`.emp`
link-externs, and cross-checked by NOTHING. One drifted entry is a silent
wrong-byte in the sound driver. The flip makes the `.emp` the single authority
and deletes the mirror.

## §2 — Decomposition (by CONSUMER SEAM, two parcels, sequential)

The Parcel-E finding is right that no CONSTANT-CLASS sub-seam exists (every
seam1 table interleaves flat/struct-offset/derived-RAM classes). The clean
split is by consumer:

### E1 — the ownership flip (authority created; .asm deleted)
- Author `engine/sound/sound_constants.emp`: the 5 Z80 structs as `.emp`
  structs (dense byte layout, the existing struct machinery — the conv-A
  `layout_struct_ambient` path; these are the first Z80-consumed struct twins
  to flip, but layout is CPU-agnostic dense bytes); the shared-prefix invariant
  as per-field `ensure(offsetof(SeqChannel,f) == offsetof(SfxChannel,f))`; the
  flat consts as `pub const`; the struct-derived aliases as derivations
  (`pub const sc_x = offsetof(SeqChannel, sc_x)`-class — never baked values);
  the derived Z80 RAM layout as computed `pub const`s (bases × `sizeof`, the
  derivation visible); the walls as `ensure`s; the 5 comptime functions ported.
- 68k side: the 33 link-externs (sound_api 24 / sound_debug 7 /
  dac_sample_tab 2) swap to `use`.
- AS residual: extend the harvest family (`harvest_sound_constants`, sibling
  of P5/conv-A/#7b) ONLY for whatever the residual actually still reads
  (~0 real sites per the finding — verify; the harvest may legitimately be
  near-empty, in which case say so rather than building dead machinery).
- Delete `engine/sound_constants.asm`.
- **E1 explicitly KEEPS seam1/seam2 as-is** (they are sigil-side tables,
  independent of the .asm) — their retirement is E2. E1's walls-to-ensures
  must not weaken anything seam1 depended on.
- BAR: all six ROM targets byte-identical to the current tips **+ the
  generated Z80 sound blob byte-identical explicitly** (engine/sound/generated
  — compare the emitted artifact, not just the final ROMs); strict green with
  every retirement enumerated/re-homed; kill-list/census bookkeeping.

### E2 — the mirror dissolution (the payoff)
- seam1's five `*_consts` tables (399 entries) stop being hardcoded: the values
  come FROM `sound_constants.emp` through the same evaluation machinery the
  build uses (ONE authority — the item-7 §9 rule: shared entry point, drift
  structurally impossible; a focused module-eval reuse, not a re-declared
  table). seam2's DAC-head carriers likewise re-point.
- The five tables and the carriers are DELETED, not wrapped: after E2 there is
  no hand-maintained copy anywhere.
- BAR: Z80 blob byte-identical + six ROMs + strict; the deletion diff IS the
  deliverable.

## §3 — Hazards (binding)

1. **Z80-blob-precedes-engine**: every E1/E2 change must keep the blob
   byte-neutral; rebuild `emit_sound_blob` before every aeon build; compare the
   blob artifact directly at every step.
2. The 13-field shared-prefix invariant must survive as compiler-checked
   `ensure`s — it is load-bearing driver behavior, not documentation.
3. Comment discipline: present-tense contract facts; the walls' WHY text
   (where load-bearing) moves into the ensure messages, not comments.
4. Inspection-first still applies inside each parcel; stops reported, never
   hacked around.

## §4 — Non-goals

- No sound-driver behavior change of any kind.
- No new language surface (everything needed shipped by conv-A/#7).
- The `sound_tables_z80` generator path (settled ledger 1615/1619/1620) is out
  of scope.
