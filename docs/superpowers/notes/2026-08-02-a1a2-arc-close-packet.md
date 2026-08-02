# The A1/A2 seam-2 arc — close packet (2026-08-02, overnight)

Arc spec: `specs/2026-08-02-a1a2-arc-design.md` (sigil `af78cdb2`). Three parcels,
three Opus porters, sequential merges, every gate own-run countersigned by the
overseer. Masters at close: **aeon `e03aad8` / sigil `ea686380`**, both pushed.
Chain: **22** (tip `a2-mtsyms` — the arc's single sanctioned appendix-only
refreeze; A1 and A3 were provenance-only/byte-identical). Strict at close:
**2990 / 0 / 4** (arc start 2973; +2 A2 reassembly-identity, +8 A1 derivation/
anchors, +7 A3 span tests). Post-merge composition re-proof: all four shapes
rebuilt from merged masters match the chain-22 goldens exactly.

## What the arc did

- **A2 — the mt_syms kill** (aeon `3263231` / sigil `8d4c874d`). The seam-2 emit
  cuts the MT blob at its own lowering's label offsets into
  `mt_bank_body|mt_songtable|mt_songpatchtable` per shape; `mt_bank_blob.emp`
  places them as three contiguous labeled members, so `SongTable`/
  `SongPatchTable` are NATIVE section labels resolved at whole-ROM link. The
  `mt_syms{,_debug}.asm` emission and `game_root.asm`'s gated include are
  deleted. **The last non-native sound residue is gone; the sanctioned AS
  survivor set is 3** (2 five-line game_root stubs + the vendored debugger).
  Identity: anchors IDENTICAL ×6; deb2 appendix +26/+24/+24 B on the sound-on
  shapes (the labels enter the convsym table) → the sanctioned appendix-only
  refreeze (chain 22).
- **A1 — registry unification** (sigil `bdcbc0de`, sigil-only). The K-capstone
  §6 P1 demand moment (met by A2) consumed: `seam2::sound_layout()` derives all
  10 banked LMAs from `map.toml`'s two declared anchors + the emit's own
  measured artifact lengths; the 10 hardcoded literals died; the emit lay-down
  order must be a subsequence of the map's declared `order` (loud desync
  error); the seam1↔seam2 cycle broke via a symbols-only lowering + lazy
  `DacSampleTable` resolve. `pins.rs`/`tests/repin_pins.rs` stay literal as the
  independent drift detectors; the old literals survive as the
  `seam2_layout_derivation` gate (+2 doctored-map negatives).
- **A3 — `span(ProcName)`** (aeon `e03aad8` / sigil `ea686380`). The comptime
  emitted-span primitive (pure query over the real body lowering; loud on
  instructions/unknown procs/unlowerable bodies). Adopted at both demand sites:
  `dac_sample_tab.emp`'s `10*9` hand literal → measured span (now catches
  wrong-descriptor-count emission, not just declared-count drift);
  `FMVOLENV_COUNT`/`PSGVOLENV_COUNT` → pub consts derived from the id-list
  spans, the two ungoverned `seam_emit_config` keys deleted, seam-1 resolving
  authority-first via `sound_tables_authority_consts`. Rider: the deleted AS
  twin's id/ptr `End-Start` count guard REVIVED as in-section ensures — an
  id/ptr desync is now a build error.

## Step-3 findings (language/tooling asks) — arc total

1. **The standalone-lower constraint** (A2): a placement `.emp` module cannot
   `use` the game's const authority because its region gate lowers it in
   isolation — forces the local-const + `ensure(extern(...))` drift-guard
   idiom (shared with `mt_bank.emp`). Existing ledger class; an affordance to
   lower region-gate modules with the game's pub-const authority in scope
   would retire the idiom. Not newly rowed.
2. A1 and A3 produced **no new asks** — A3 *is* the row-1654/1805/1911 ask,
   shipped; rows closed same-branch, row 1879 discharged as the id/ptr rider.

## Step-5 findings (deferred optimizations)

1. **Emit-lowering reuse** (A1): `sound_layout` is memoized, but the full
   `emit_sound_blob` run still re-lowers artifacts the derivation already
   lowered — a caching opportunity, correctness-neutral. Not rowed as a
   priority; note-only.
2. **Vol-env control-byte harvest** (A3): ledger row 1910's 6 `*VolEnvCtl_*`
   bytes stay in the generator — out of the A3 demand; the row stands.

## Neither-bucket headlines

- **The shared-checkout ROM staleness class** (found by the A1 porter, fixed
  in-flight): the shared aeon checkout's built ROMs predated the A2 merge, and
  `demo.bin` is only rebuilt by a `./build.sh demo` invocation (positional
  arg) — a sonic4 build leaves it stale, the capture-script's documented trap.
  All four shapes rebuilt and verified against goldens; any live-ROM-comparing
  test run must check ROM freshness first. `build.sh` now (post-flip) requires
  `SIGIL_BUILD`/`SIGIL_EMIT` env.
- **A2's appendix growth was the predicted middle case** — the two labels
  changing form (AS equ → native section label) enters them into the deb2
  appendix; behavior-inert, post-EndOfRom, proven anchors-identical.
- **The soundness arc of A3 is larger than its size**: two invariants that were
  hand-maintained (or absent — the vol-env counts had NO guard) are now
  measured, and a third (id/ptr sync) that had been silently downgraded at
  conversion time is re-armed. The `span` primitive turned three trust-me
  facts into build errors.
- Overseer errata: the A1 merge-commit message lost its `$`-hex values to
  shell interpolation (cosmetic; messages now single-quoted); one early
  countersign strict run piped output through grep/uniq/head (exit code and
  totals masked) and was re-run with a full captured log before adjudication.

## Ledger/kill-list reconciliation

- Gap-ledger: mt_syms rows closed (A2); P1-unification + gate_blocks rows
  closed (A1 — gate_blocks found already consummated, zero grep hits); rows
  1654/1805/1911 closed, 1879 discharged (A3). Language-round ledger SECTION 3:
  A1/A2/A3 all consumed.
- Kill-list: the mt_syms row closed with an A2 closure blockquote; survivor
  set 4→3 recorded.
- The K packet's "honest 100%" statement updates to: **the AS side is THREE
  files** (2 game contracts... now game_root stubs only + 1 vendored debugger)
  loaded by 2 five-line stubs; no `.asm` emits a ROM byte, declares an org, or
  carries a cross-seam symbol.

## Arc verdict

The arc closed the last architectural residue of the conversion campaign: the
sound stack is 100% native including its cross-seam labels, placement has ONE
authoring site (`map.toml` — the emit tool now derives instead of asserting),
and the language gained the span primitive the data-table corpus had been
asking for since the Z80 rung ports. Next: THE CAMPAIGN RETROSPECTIVE.
