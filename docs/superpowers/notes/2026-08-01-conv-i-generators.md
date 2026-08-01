# conv-i — the generators emit `.emp` (Parcel I)

**Parcel:** I (Opus porter; overseer countersigns/merges — NOT merged/pushed).
**Repos:** aeon `conv-i-generators` · sigil `conv-i-generators` (worktree `.worktrees/opt-sweep`).
**Date:** 2026-08-01.

Three sub-tracks. Headline: **sub-track (a) mddbg SHIPPED byte-identical** (the
`.emp`-side equ-off-link-external-base surface was BUILT to do it — the census
"already live" premise was AS-side-only); **sub-tracks (b) and (c) STOP with
numbered premise corrections** — (b) is mostly already-retired / gitignored
build-products with one seam-scheduled deferral; (c) rests on a premise that does
NOT hold (**the AS frontend cannot `include` a `.emp` file**), which reclasses every
include-embedded generated file as native-placement work, and the interior-embedded
ojz files as residual-split-capstone-scale.

---

## THE ONE STRUCTURAL FINDING THAT DRIVES THE WHOLE PARCEL

`sigil-frontend-as::directive_include` (eval.rs:1222) reads a file's text and
**execs it as AS source** — there is no `.emp` dispatch. So a generated file that
is `include`d into the AS assembly stream CANNOT simply "become `.emp`": the AS
frontend would try to parse `.emp` syntax as AS and fail. The only ways AS
consumes `.emp`-origin material are (1) **native placement** (a registry module in
a section, linked into a chained region) or (2) **link EquSyms / guarded-defines**
(scalar values, not ROM data). Every generated file that emits ROM DATA
(BINCLUDE wrappers, `dc.w` tables) therefore must become a **natively-placed
`.emp` module** to convert — the same registry/pin/gate/frozen-anchor machinery
`test_mappings` (conv-h #35) and `boot_data` (conv-h2 #12) face — **not** a
mechanical "generator retargets its writer to `.emp`."

This corrects the census's Parcel I framing ("each generator emits `.emp` instead
of `.asm`. Effort S–M each; mechanical once one is done").

---

## SUB-TRACK (a) — mddbg_symbols (#7): SHIPPED, six-target byte-identical

### What moved
`engine/debug/mddbg_symbols.asm` (44 `MDDBG__* = ErrorHandler+off` equates, zero
bytes, `include`d by engine.inc) → a **45-entry `pub equ` table folded into
`engine/debug/error_handler.emp`** — the module that already owns the blob
(`ErrorHandlerBlob`) these symbols index into and already referenced two of them.
Each entry is `pub equ MDDBG__X = extern("ErrorHandlerBlob") + $off`. engine.inc
drops the `include` AND the now-unneeded `ErrorHandler = ErrorHandlerBlob` AS alias
(bare `ErrorHandler` had no other consumer — verified whole-tree).

Folding into the existing native module (not a new module) = zero new registry /
pin / gate / frozen-anchor: the pre-ruling's "less machinery, fewer artifacts, one
authority" applied to sub-track (a) too.

### The census premise correction + the surface that was built
The census said #7 is a "straight equate-flip using the link-external-base
capability (already live — engine.inc:409 uses it for ErrorHandler alias)." That
capability is **AS-side only** (`fold_equ_syms` folds an AS `X: equ Ext+off`). The
`.emp` side had NO equivalent that (1) folds a `label+offset` into an exportable
link symbol and (2) is consumable by a `.emp` `dc.l`. Empirically:

- `pub equ MDDBG__X = ErrorHandlerBlob + $off` → `unknown name ErrorHandlerBlob`
  (the equ evaluator does not resolve a forward link label; 47 errors).
- `pub equ MDDBG__X = extern("ErrorHandlerBlob") + $off` PARSES and registers a
  named `EquSym` (mod.rs `lower_equ_item`), so AS externs + intra-`.emp` code
  resolve it — BUT the blob's own two `dc.l MDDBG__Debugger_AddressRegisters,
  MDDBG__Debugger_Backtrace` cells fail `[dc.comptime-only]`: a `dc` element that
  resolves to a LinkExpr (extern+offset) was rejected (2 errors).

**The surgical unblock (sigil `crates/sigil-frontend-emp/src/eval/asm.rs`):** the
`dc` element match now routes a `Value::LinkExpr` to the general link-expr VALUE
cell (`Cell::Expr`) — a bare `Sym` keeps `Cell::SymRef`; an arithmetic residual
tree emits `Cell::Expr`, the SAME machinery the typed-data emit path
(`lower_link_expr`) and the `offsets`/`dispatch` constructs already use, and the
linker already writes (`sigil-link` general link-expr value cells, S2-D13f). ~15
lines + `tests/dc_link_expr.rs`. This is a genuine capability completion (the `dc`
surface gains the link-expr value the type-layer already had), not a bespoke hack.

### Identity (six-target FULL-CRC, chain-10 anchors)
| target | CRC32 / size | proof |
|---|---|---|
| s4.bin | ff9037f2 / 412127 | direct `./build.sh` |
| s4.debug.bin | 06680f0b / 421958 | direct `DEBUG=1 ./build.sh` |
| demo.bin | 9bb8c993 / 90506 | direct `./build.sh demo` |
| demo.debug.bin | bc7678d0 / 93006 | direct `DEBUG=1 ./build.sh demo` |
| config_a | 2485eab3 / 422297 | golden gate `native_offcanonical_*` |
| config_b | d6d23298 / 303501 | golden gate `native_offcanonical_*` |

FULL identity — NOT appendix-only: the CRCs (which include the deb2 symbol
appendix) are UNCHANGED, so the `MDDBG__*` symbols still appear identically in the
symbol table, now sourced from `.emp` rather than the AS include. No re-freeze.

---

## SUB-TRACK (b) — sigil-emitted syms (#9/#10/#11): drift corrections + one deferral

**Premise correction: all three are gitignored BUILD PRODUCTS, not committed
artifacts** (`git check-ignore` confirms `engine/sound/generated/*.asm`).

- **#11 z80_sound_syms.asm — ALREADY RETIRED.** `emit_sound_blob` no longer emits
  it (its `.asm` output list is `mt_bank{,_debug}.bin + mt_syms{,_debug}.asm`
  only), `seam1.rs:446` documents the retirement ("the old z80_sound_syms.asm
  handler-VMA contract file is no longer …"), and it is referenced NOWHERE in
  either repo (whole-tree grep). The `Seq_Op_*` symbols come from
  `sound_sequencer.emp`'s `pub proc`s directly, resolved same-seam by
  `seq_opcode_tab.emp`. The stale on-disk copy (mtime 2026-07-30, gitignored,
  never committed) is a dead file, not a live artifact. **Nothing to convert.**

- **#9/#10 mt_syms{,_debug}.asm — DEFER to the seam-2-native mt_bank stage.**
  These ARE live: `include`d by main.asm (287/290), defining
  `SongTable`/`SongPatchTable` as absolute equates that `sound_api.emp`'s
  `extern("SongTable")` resolves at link. Converting them:
  - **emit-`.emp`-syms** needs a native NO-SECTION equate module (new registry
    machinery) whose `pub equ`s feed the AS extern — net-MORE machinery, for a
    file the next seam stage deletes.
  - **fold-via-link** cleanly needs the mt_bank BLOB to be natively placed (not
    AS-`BINCLUDE`d): then `SongTable` is a link label directly and the sym file
    vanishes with ZERO generated artifact. main.asm already carries the
    `SIGIL_EMP_MT_BODY_STUB` arm for exactly this native-mt_bank future (seam-2
    stage). The values are shape- AND placement-dependent — hardcoding them as
    comptime `.emp` ints would move the placement-authority assumption into
    `.emp` (fragile), against the current design intent.

  Evidence-grounded decision (per the pre-ruling): **the least-machinery /
  fewest-artifacts / one-authority path is to let #9/#10 ride the scheduled
  seam-2-native mt_bank placement**, which retires the sym file entirely via link.
  Emitting `.emp` now is strictly more machinery for a doomed file. No code
  changed for (b).

---

## SUB-TRACK (c) — the Python generators (#8, #28–33): STOP, native-placement-scale

All committed sub-track-(c) targets live in
`games/sonic4/data/generated/ojz/act1/` (#28 bg_anim, #29 entity_data, #30
ojz_act_pool, #31 ojz_act_pool_manifest, #32 sec_block_blobs, #33
sec_block_dicts). #8 vectors.asm is gitignored (a `gen_compression_vectors.py`
build product, with its 5 golden `.bin` also gitignored).

By the structural finding above, each is `include`d into the AS stream, so
converting it = a natively-placed `.emp` module. Their placement contexts:

- **#28/#30/#31/#32/#33 — INTERIOR-embedded in `act_descriptor.asm`.** The pool
  manifest/pool/dict `include`s sit at act_descriptor.asm:8/9/19 (BEFORE the
  `org $14D9E` that resumes the descriptor region); the block-blobs + bg_anim
  `include`s at :40/:56 (AFTER the native descriptor, interleaved with the
  palette/BG `BINCLUDE`s). Native-placing any of them means a pin at its exact
  interior address + the AS side org-resuming past the hole — **the
  residual-split capstone shape** (interior data-island holes between native and
  AS-residual). conv-h #34 already ruled these "stay AS-side … the generated
  includes / BINCLUDEs + org resume stay AS-side" for exactly this reason. Even
  the 4-line #28 bg_anim stub needs the full per-file machinery (cost is
  per-FILE, not per-complexity). **= Parcel K (residual-split capstone) territory.**

- **#29 entity_data.asm — at an org boundary (main.asm:156, after `org
  $11D7E`)** but the data shape is X-sorted ring lists + `objentry`/`objend`
  object placements + per-section count-prefixed `dc.l ObjDef_*` type tables —
  needs the **objentry/objdef level-data DSL** (not yet a `.emp` construct). A
  native port is a data-DSL build, not mechanical.

- **#8 vectors.asm — gitignored, DEBUG-only, at an org boundary (engine.inc:339,
  `org $72BE`), now `embed`-expressible** (conv-h2 shipped `embed(...)`; the 3
  `CSELF_*` equates → pub consts, the 5 golden `BINCLUDE`s → `embed()`). This is
  the single most-tractable native port, but still a full registry / pin / gate /
  frozen-anchor + `gen_compression_vectors.py` rewrite (Effort M, test_mappings
  scale) — deferred as a stand-alone native-port parcel, not landed here to avoid
  a half-done DEBUG-region port under budget.

No code changed for (c). Recommendation: the ojz interior islands (#28/#30–33)
ride Parcel K's residual-split; #29 waits for the objdef/objentry data DSL; #8
vectors is a clean stand-alone native-port candidate (own parcel) once someone
wants the DEBUG golden vectors `.emp`-owned.

---

## Gates (failures-first)

- **Strict:** `SIGIL_STRICT_GATE=1 AEON_DIR=<aeon> cargo test --workspace` →
  **2875 passed / 0 failed / 4 ignored** (baseline 2874 + 1 new `dc_link_expr`;
  `error_handler_port` stays 4 tests, `vectors_port` stays 5, both updated to the
  new ownership). Zero failures.
- **error_handler_port.rs (updated):** its harness previously injected 4 synthetic
  AS-side `MDDBG__*` labels; those now COLLIDE with error_handler.emp's own pub
  equs, so the synthetic injection + its two helpers + the 4 offset consts were
  removed — the module self-resolves. All 4 tests green (region byte-match both
  shapes + the 12-vector ownership flip + the AS-side derived-equ test).
- **Clippy:** `cargo clippy -p sigil-frontend-emp -p sigil-cli --tests` — no new
  warnings in the four edited files (asm.rs, dc_link_expr.rs, error_handler_port.rs,
  vectors_port.rs); pre-existing workspace warnings untouched.

## step-3 (retrospect) / step-5 (engine)

- **step-3:** the `.emp`-side equ-off-link-external-base gap was real (the census
  premise was AS-side-only). It closed by wiring the `dc` surface to the general
  link-expr value cell the type layer already had — the language stays smaller (no
  bespoke mddbg path; the same `Cell::Expr` machinery). The `MDDBG__*` table found
  its natural owner (error_handler.emp) rather than a standalone module.
- **step-5:** none — no engine bytes changed (sub-track a is a zero-byte ownership
  move; b/c changed nothing). The parcel did NOT invent per-file native placement
  for the interior ojz islands under budget/scope pressure — that is the correct
  STOP (it is Parcel K work).

## Retirements / re-homes

- **Retired:** `engine/debug/mddbg_symbols.asm` (deleted; folded into
  error_handler.emp). #11 z80_sound_syms already retired upstream (no action).
- **Re-homes (sigil):** `eval/asm.rs` dc link-expr arm + `tests/dc_link_expr.rs`
  (new capability); `tests/error_handler_port.rs` updated to the new ownership.
- **Still AS-residual (STOPPED with findings):** #9/#10 mt_syms (seam-2-native
  deferral), #8 + #28/#29/#30/#31/#32/#33 (native-placement / capstone / data-DSL).

## Gap-ledger (→ campaign-gap-ledger.md)

1. **`.emp` native NO-SECTION equate module** — a registry module that emits zero
   ROM bytes but exports `pub equ`s to the link table (the #9/#10 emit-`.emp`
   path, if the seam-2-native route is not taken).
2. **objdef/objentry level-data DSL** (#29 entity_data blocker) — object-placement
   records (`objentry x,y,type[,sub][,oflags]` + `objend` terminator) + count-
   prefixed type tables as a `.emp` data construct.
3. **Interior data-island native placement** (#28/#30–33 blocker) — the
   residual-split capstone's interior-hole mechanism (a native module placed
   BETWEEN two AS-residual data islands within one `include`d file).

## Kill-list (→ twin-scaffolding-kill-list.md)

- #7 `mddbg_symbols.asm` DELETED (no scaffolding survives; error_handler.emp is
  sole source of the `MDDBG__*` table). No new twin scaffolding introduced.
