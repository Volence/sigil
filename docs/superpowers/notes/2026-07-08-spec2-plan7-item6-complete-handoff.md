# Completion handoff — Spec 2 · Plan 7 backlog #6: SST overlay + `dispatch`

Date: 2026-07-08 (Fable). Branch `plan7-item6-overlay-dispatch` (worktree
`sigil/.worktrees/plan7-item6-overlay-dispatch`), 17 commits on top of master `d1e4288`.
**Status: implementation complete, all reviews passed, awaiting Volence merge checkpoint.**

## What shipped (by commit)

**Part 0 — #5-merge audit fixes** (from the two post-merge adversarial audits of ba3fb98..d1e4288):
- `91c0228` — nested `section{}` REJECTED loudly (`[section.nested]`); was silently dropping
  items/guards/max_size checks in lowering.
- `a988356` — section-nested `pub` comptime defs now injected cross-module (prelude + use paths).
- `f1a58c8` — section-nested `use` honored in all three flat scans (BFS, rename env, ambient).

**Part A — SST overlay + field-access-as-displacement** (T2-c; the pitcher_plant `timer(a0)` blocker):
- `0a67de2` — dotted window path (`vars X: Sst.sst_custom {}`); `VarsDecl.region: Vec<String>`.
- `740adf9` + `1f35fc9` + `aded86a` — overlay data model: window resolution (bare-unique/dotted,
  `[overlay.unknown-window]`/`[overlay.ambiguous-window]`/`[overlay.window-not-bytes]`/
  `[overlay.bad-window]`), layout by struct rules, always-on `[overlay.window-overflow]` +
  `[overlay.shadows-field]`, `offsetof`/`sizeof` on overlays, odd-field lint on WINDOW-ABSOLUTE
  parity, once-per-compile diagnostic dedup across passes, bare-window scan matches by name
  WITHOUT forcing unrelated struct layouts, `[u8;N]`-only windows.
- `3177201` + `fefa61d` — field access, bare form: `timer(a0)` on param-typed registers
  (`a0: *Sst`) lowers to integer-displacement bytes ($2E(a0)-class), field-space-only resolution
  (`[operand.unknown-field]`, `[operand.ambiguous-field]`), `[operand.field-overrun]` width check,
  untyped registers byte-identical to master, per-proc register-typing scope.
- `504bece` — qualified form `V.timer(aN)`/`Sst.x_pos(aN)` on ANY address register (the
  cross-object idiom); `pub vars` overlays cross-module via use/prelude.
- `4748476` — `examples/sst_overlay.emp` + byte-exact ports test (24-byte image, hand-derived).

**Part B — `dispatch`, the encoding-agnostic typed state-dispatch table** (T1-b per R1/R2):
- `7caf262` — grammar/AST: `dispatch Name (encoding: word_offsets|long_ptrs) { Member: target }`;
  encoding REQUIRED (R1: enable, don't impose); `Member: { … }` inline bodies RESERVED for #9
  with a specific error.
- `f02b124` — word_offsets: `dc.w target−Name` on the shipped offsets RelOffset machinery
  (offsets byte-for-byte untouched; data.rs zero diff), ordinals `Name.Member` = ordinal×2,
  `Name.count` unscaled, reserved-`count`/dup-member validation, `[dispatch.target-not-code]`
  module-local kind check, `[dispatch.non-68k]`, cross-module targets ride the generic
  Fragment::Data rename (adversarially probed: two modules' private `helper` procs, each table
  points at its OWN helper).
- `200534e` + `0b3f263` — long_ptrs: `dc.l target` via `Cell::SymRef{width:4}` (the `code:"init"`
  pointer cell), ordinals ×4; review ride-alongs (mirror back-refs on the offsets side,
  empty-dispatch + kind-noun + ×4-ordinal tests).
- `c3d9ee3` — `examples/dispatch.emp`: both encodings over the same three procs, byte-exact
  (35-byte image, hand-derived).

## Verification evidence (independently re-run by Fable, 2026-07-08)
- `cargo test --workspace --no-fail-fast`: exactly the 4 allowlisted pre-existing sigil-harness
  reds (aeon sound-driver strlen refactor: `full_build_reproduces_sound_driver_regions`,
  `vector_table_matches_reference_rom_first_256_bytes`, `full_debug_rom_matches_assembled_reference`,
  `full_rom_matches_assembled_reference`) — zero new failures (~1156 passing).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- Both exhibits compiled fresh and hand-verified byte-for-byte against the 68k reference by the
  controller (not test comments).
- Process: TDD per task (RED confirmed each), two-stage reviews on T5/T6/T10 with fix loops
  (5 review findings fixed on-branch), final whole-branch adversarial review incl. master-vs-branch
  byte-divergence probes, feature-interaction matrix, resolver-fix composition, grep sweep — all
  clean; verdict checkpoint-ready.

## Notes for the merge reviewer (sanctioned, not defects)
1. **D6.A3 is a deliberate byte-level meaning change** for `bareName(typedReg)`: on a register
   typed `*S`, a bare displacement identifier resolves ONLY in field space — a const with the
   same name as a field is no longer consulted (master emitted the const; branch emits the field
   offset, or `[operand.unknown-field]` if no field exists). Untyped registers are byte-identical
   to master. No existing source (incl. the full aeon harness) hits this.
2. **Reverse ordinals (`Name.Member`) do not resolve cross-module** — for `dispatch` AND the
   pre-existing `offsets` alike (`pub_comptime_name` injects neither). Base labels DO export
   (targets work cross-module). Consistent parity; ledger candidate for a later increment.
3. MINOR (cosmetic): `offsets Tbl` + `dispatch Tbl` in one module collides at link
   (`symbol redefined` — loud), but `eval_path` would resolve `Tbl.Member` via the offsets arm
   first; unreachable as wrong bytes today. Fold a name-collision diagnostic into a later item.

## Deferred by decision (design doc, for the spec ledger)
- Part A: region-form `vars` allocation (map-file work, #7 territory); `Player_1.x_pos`
  straight-line symbolic operands (gap b6, RelaxAbsSym extension); field arithmetic
  (`timer+1(a0)`); typed-reg field access in `asm{}`/comptime fns; `[operand.const-as-address]`
  lint (natural next increment).
- Part B: ordinal-unscaled ids (Treasure word-index); no-table continuation style (= gap b7
  proc-name-as-value); Z80 dispatch; `start:` ordinal origin; `for Enum` binding; per-member pub;
  `Member: { … }` scripted states (backlog #9 — grammar seam reserved, erroring specifically).
- Amended semantics vs the original design doc (ratified during review): bare-window matching is
  NAME-first (unique name → then byte-array check, for better diagnostics); odd-field lint keys
  on window-ABSOLUTE parity; `[i8;N]` windows rejected.
- Extraction of a shared offsets/dispatch emission core: deliberately deferred (rule of three —
  revisit when a third table-shaped construct lands); mirror back-ref comments are the guard.

## Spec integration (Fable's next action after merge)
Lift into `empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` per the design doc's final section: §4.6 pinned
overlay semantics (D6.A1–A9 as amended above), new `dispatch` section (§5.5 or §4.8), D2.21+
decision rows, deferred-ledger entries (incl. cross-module ordinals), D2.15/S2-D10 cross-ref
updates. Design doc: `specs/2026-07-07-spec2-plan7-item6-overlay-dispatch-design.md`; plan:
`plans/2026-07-07-spec2-plan7-item6-overlay-dispatch.md` (both committed on this branch).

## After #6 (unchanged backlog)
#7 bank/window placement (`no_straddle`) → #8 jbra/jbsr + relaxation → #9 byte-command DSL /
scripted coroutine (dispatch's reserved member-body seam) → #10 compression builtins.
pitcher_plant.emp still needs (by design): #8, statement-position comptime helpers
(spawn/anim/routine grammar), `code: init` bareword proc pointers, `Player_1.x_pos` operands.
