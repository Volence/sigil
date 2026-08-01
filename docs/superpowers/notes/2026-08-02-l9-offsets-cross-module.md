# 2026-08-02 — L9: the `offsets` construct's cross-module target adoption (close packet)

Status: **Checkpoint for the overseer's countersign + merge.** Branch pair
`lang-l9-offsets` (aeon + sigil). The three player-state jump tables adopt the
§4.7 `offsets` construct with bare cross-module targets; byte-identical across
all six targets. NOT merged — the merge is the overseer's.

## §0 — THE HEADLINE

**The shipped `offsets` construct ALREADY expresses cross-module targets — no
sigil source change was needed.** The `Ref` (by-reference) form takes a target's
symbol NAME by shape (it does not resolve the name locally), so a bare
identifier naming a `pub proc` in a SIBLING module lowers to the exact same
`Cell::RelOffset { base, target }` — and thus the exact same self-relative
`dc.w target - base` word — that the hand `extern(target) - extern(base)` data
form produces. The three player-state tables (`Player_States` /
`PState_EnterHooks` / `PState_ExitHooks`) flip to the construct **byte-
identically**, and the hand `ensure(7 == PSTATE_COUNT)` guards are replaced by
`ensure(Table.count == PSTATE_COUNT)`.

Strict **2915 → 2920** (+5 new cross-module tests, 0 failed, 4 ignored). Six
targets byte-identical; `refreeze --check` OK (chain 18); `repin --check` pins.rs
unchanged. sigil is a **test-and-doc-only** change; aeon is the flip.

## §1 — THE GRAMMAR DECISION (and why no code changed)

The ledger (row 1803 / the t38 correction row 1848) estimated this adoption as
"construct-feature-scale — TDD the cross-module `Ref` declaration path", on the
premise that "the corpus has ZERO `offsets Name {}` DECLARATION-form adoptions …
cross-module `Ref`-to-`.emp`-target adoption is UNEXERCISED". Re-verified against
the implementation, that premise over-scoped the work:

- **The `Ref` form is built and unit-tested** (`OffsetsTarget::Ref`, ast.rs:399;
  parser.rs:944–947; `eval_offsets_with_root`, layout.rs:1710–1780). The existing
  `eval_offsets.rs` tests already exercise bare-identifier targets (`t0, t1, t2`).
- **The `Ref` path takes the target NAME by shape, not by local resolution.** For
  a single-segment bare `Path` that is neither a builtin scalar type nor a known
  `const`, it emits `p.segments.join(".")` verbatim as the `RelOffset` target
  (layout.rs:1750–1754). It never checks that the name is locally defined — so a
  bare cross-module identifier is accepted exactly like a local label and left
  for the linker to resolve, which is precisely the L11 "bare = cross-module link
  ref" idiom.
- The emitted `Cell::RelOffset { base, target }` lowers to a `RelWord16Be`
  fixup = `Sub(Sym(target), Sym(base))` — **identical** to the fixup the hand
  `extern(target) - extern(base)` data form produces (asm.rs:855–864 documents the
  data form uses the same `Cell::RelOffset`).

**Grammar shipped (one construct, two entry kinds — both pre-existing):**

```
pub offsets Player_States {
        Ground:   PState_Ground,     // bare cross-module ref -> dc.w PState_Ground - Player_States
        Roll:     PState_Roll,
        // ...
}
ensure(Player_States.count == PSTATE_COUNT, "…")
```

Decisions and their rationale:
- **Bare identifier targets** (`Ground: PState_Ground`), NOT `extern("…")`, per
  the L11 ruling (bare = cross-module link ref). The `Ref`-by-shape path makes
  the bare form resolve as a raw link symbol; `extern()` is unnecessary in
  `offsets` target position.
- **Entry ordering = declaration order = PSTATE_\* order.** The dispatch indexes
  the table by the `player_state` byte (a word-scaled PSTATE_\* value: 0,2,4,…,12),
  so declaration order is load-bearing and preserved verbatim from the array. The
  construct's own ordinals (`.Ground == 0`, `.Roll == 1`, …) are a byproduct and
  are NOT used for dispatch (they are 0-based sequential, not word-scaled).
- **`.count` replaces `ensure(7 == PSTATE_COUNT)`** — at least as strong: it is
  the machine-derived member count, so it cannot silently drift from the table
  the way a hand-written `7` could.
- **Width = word** (the shipped construct's signed-word offsets, matching the
  `[i16; 7]` the arrays declared). The §4.7 `dc.l` / `base:` / `start:` / Z80
  knobs are UNTOUCHED (see §4).

The only genuinely-unexercised-before-L9 things — a bare cross-module target that
resolves to a real sibling-module `pub proc`, its byte-identity, and `.count` on
a Ref-only table — are now covered by tests (§3) and the byte gate (§2). No
`parse/AST/lower/link` code changed; the parcel is a proof + adoption.

## §2 — BYTE-IDENTITY PROOF (all six vs the pre-flip golden)

Golden CRCs recorded from the PRISTINE worktree (branch tip before any edit),
then re-verified after the flip:

| target | pre-flip golden | after flip | match |
|---|---|---|---|
| s4.bin        | `5f72b9c3` / 412134 | `5f72b9c3` / 412134 | ✓ (direct CRC) |
| s4.debug.bin  | `e6171a80` / 421970 | `e6171a80` / 421970 | ✓ (direct CRC) |
| demo          | frozen provenance | in-proc build | ✓ (`native_offcanonical_full`) |
| demo.debug    | frozen provenance | in-proc build | ✓ (strict-suite) |
| config_a      | frozen provenance | in-proc build | ✓ (`config_a_full_file`) |
| config_b      | frozen provenance | in-proc build | ✓ (`config_b_full_file`) |

s4/s4.debug verified by direct CRC against the pristine golden; demo/config_a/
config_b via the strict suite's whole-ROM provenance tests (they build the FLIPPED
aeon source in-process and compare against master's frozen `golden/provenance.toml`
CRCs — the rigorous cross-check that the flip moved no byte in any shape). Ownership
move at unchanged bytes: no re-freeze (`refreeze --check` OK, chain 18), pins.rs
unchanged (`repin --check`).

## §3 — THE TESTS (sigil, `offsets_cross_module.rs`, +5)

1. `cross_module_ref_lowers_like_the_extern_difference_form` — the `offsets` Ref
   form and the hand `extern(target) - extern(base)` data form emit IDENTICAL
   linked bytes (`[0x00, 0x02, 0xAA]`). The equivalence proof.
2. `cross_module_ref_target_needs_no_local_definition` — a bare target absent from
   the module (the real cross-module case) lowers CLEANLY to one
   `RelOffset { base: "Tbl", target: "PState_Ground" }` per member; NO local-
   resolution diagnostic fires (that would false-flag every legit cross-module ref).
3. `repeated_cross_module_target_emits_a_word_per_member` — the
   `PHook_AirBallEnter × 3` pattern: a repeated target emits its own self-relative
   word per member.
4. `undefined_cross_module_target_fails_loudly_at_link` — a genuine typo lowers
   clean (indistinguishable from a valid ref at lower time) but the linker fails
   loudly naming the target (`unresolved target expression (dangling symbol(s)
   \`PState_Typpo\`)`). §4.7's "fails loudly at link".
5. `count_ordinal_available_for_the_sync_guard` — `.count` is available on a
   Ref-only cross-module table (folds `Table.count == PSTATE_COUNT`).

The pre-existing negatives (`const_alias_target_is_diagnosed_as_const`,
`non_label_target_is_diagnosed`, `builtin_type_without_initializer_names_the_fix`
in `eval_offsets.rs`/`offsets_inline.rs`) already cover the non-label rejection at
lower time, naming the entry. Not duplicated here.

## §4 — §4.7 DEFERRED KNOBS — what remains untouched

Confirmed UNCHANGED (each still a later item):
- **`base:` override** — a table whose words are relative to a FOREIGN anchor
  (not the table's own base). This is the `code_addr = label - ObjCodeBase`
  family (test_parent/test_emitter/objdef) — see §5's corpus sweep; those stay
  the hand form pending this knob (they also live as struct fields, not bare
  offset tables).
- **`start:` ordinal-origin override** — classic-Sonic `idstart = 1` blocks; Aeon
  never needs it.
- **`dc.l` (long) offsets** — the player tables are word offsets.
- **Z80 offset tables** — `[offsets.non-68k]` still refuses them.
- **inline-target (mixed) blocks** — shipped separately (D2.31, `offsets_inline.rs`);
  the player tables are pure by-reference, no inline bodies.

**Spec-text recommendation (a step-3 item, NOT applied — see §7):** §4.7's
deferred-knobs list currently reads "cross-module/multi-segment targets (folds
into the S2-D3 module-resolution work)". L9 proves **single-segment bare
cross-module targets already ship** via the `Ref` form (§4.7 itself already says
the by-reference form "keeps shared/cross-module targets", line ~365 — the
deferred-knobs line is now in tension with that). The residual deferral is
specifically **MULTI-SEGMENT (dotted `mod.thing`) targets**, which
`eval_offsets_with_root` still joins `a.b` and leaves for S2-D3 (layout.rs:1645–
1647, 1751–1753). The knob should be narrowed to "multi-segment (dotted)
targets". Flagged for the overseer/Fable to apply in a `docs(spec2)` integration
pass — not edited here (§7).

## §5 — CORPUS SWEEP (`- extern(` self-relative offset tables)

Every `- extern(` site in the aeon `.emp` corpus, classified against the
construct's shape (a table of `dc.w Target - <own base>` words):

| site | shape | disposition |
|---|---|---|
| `player_common.emp` Player_States / PState_EnterHooks / PState_ExitHooks | `dc.w Target - <own base>` word tables | **FLIPPED** (this parcel) |
| `objects/test_parent.emp:165-167` `SpawnDesc{ code: extern("TestChildPart") - extern("ObjCodeBase") }` | struct field, base = FOREIGN `ObjCodeBase` | LEFT — `base:` knob (foreign anchor); also a struct field, not a bare offset table |
| `objects/test_emitter.emp:62`, `test_stress_emitter.emp:64` `code: … - extern("ObjCodeBase")` | struct field, foreign base | LEFT — `base:` knob |
| `engine/objects/objdef.emp:71` `code_addr: extern(code) - extern("ObjCodeBase")` | comptime-fn struct field, foreign base | LEFT — `base:` knob (the ObjRoutine-constructor family, ledger item-13) |
| `player/sonic.emp:70` `ensure(8*2 == extern("Player_Phys_End") - extern("Player_Phys"))` | SPAN GUARD (ensure), not a table | LEFT — a size assertion, not an offset table |
| `engine/level/parallax.emp:29` `ensure((extern(…) - extern(…))/4 == …)` | span guard | LEFT — size assertion |
| `engine/system/boot_data.emp:73` `equ Z80_SOUND_SIZE = extern("Z80_Sound_End") - extern("Z80_Sound_Start")` | size EQU | LEFT — a span equ, not a table |
| `engine/system/z80_init.emp:45` `ld bc, extern("Z80_RAM_END") - extern("Z80_RAM") - .code_end - 1` | Z80 instruction immediate | LEFT — Z80 + instruction operand (not a table) |

**Result: 3 flipped, 0 other eligible.** The three player-state tables were the
corpus's ONLY self-relative-to-own-base `dc.w Target - Base` offset TABLES. Every
other `- extern(` is a span guard/equ, a Z80 immediate, or a struct-field
computation against the FOREIGN `ObjCodeBase` anchor (the deferred `base:` knob).

## §6 — PER-PASS BREAKDOWN

**Step-3 (retrospect — new language/tooling asks):**
- **The `offsets` `Ref` form already handles single-segment cross-module targets
  — the ledger's "construct-feature-scale" estimate (row 1803/1848) was too
  pessimistic.** The by-shape target extraction means no declaration-form-specific
  cross-module machinery was ever missing; the deferral was really a lack of an
  in-tree ADOPTION + acceptance test, both now supplied. Correction logged up the
  chain (the estimate was the overseer's, per the row-1848 discipline).
- **SPEC §4.7's deferred-knob list is stale** — it lists "cross-module … targets"
  as deferred while the same section's design paragraph already says the
  by-reference form "keeps shared/cross-module targets". Recommend narrowing to
  "multi-segment (dotted)" (§4). Spec-home question flagged (§7).
- **The one true residual is MULTI-SEGMENT (`mod.thing`) resolution** (S2-D3). Not
  demanded by any current corpus site (every cross-module target is a single
  bare symbol — procs export unmangled global labels), so still consumer-gated.
- **Lower-time "unknown target names the entry" is structurally impossible for a
  cross-module ref** — a typo and a valid sibling-module target are the same shape
  (a bare name absent from this module). The link-time error (which names the
  target symbol) is the correct and only place to catch it; §4.7 already sanctions
  this. No ask.

**Step-5 (engine optimization observed, out of scope):** none. Pure readability
adoption at unchanged bytes; no lowering changed, no bytes moved.

**Neither-bucket headlines:**
- **`extern()` is now absent from `player_common.emp` entirely** — the three tables
  were its only remaining users in that file; the flip removes 21 `extern("…")`
  occurrences (7 per table) in favor of 21 bare cross-module refs.
- **The construct's ordinals are unused-but-free** — the dispatch keys on the
  word-scaled PSTATE_\* bytes, not the 0-based `.Ground/.Roll/…` ordinals. They
  are available (and named after the state, index-relatable) should any future
  reader want `Player_States.Roll` for a comment/assert; harmless.
- **`base:` (foreign-anchor) knob has concrete demand** — the ObjCodeBase struct-
  field family (test_parent/test_emitter/objdef) is the next natural offsets
  consumer, but it wants both the `base:` override AND struct-field emission; it
  folds into the ObjRoutine-constructor item (ledger item-13), not a bare `offsets`
  block.

## §7 — SPEC-HOME QUESTION (flagged, not acted on)

`empyrean/docs/SIGIL_SPEC2_LANGUAGE.md` is the ratified home for construct
semantics (D2.x decisions integrated via `docs(spec2)` commits). But its last
content edit is the July-2026 v1-freeze/D2.36 pass — the ENTIRE conversion
campaign and the language round (L5/L8/…) documented in sigil notes + the
gap-ledger, never editing empyrean unilaterally. Per the L9 brief's guidance, the
§4.7 knob-narrowing (§4) is therefore RECOMMENDED here and left for the
overseer/Fable to apply in a spec-integration pass, rather than edited on this
porter branch. If the overseer rules that porters may touch SPEC2 §4.7 directly,
the one-line narrowing is ready.

## §8 — FILE MANIFEST

**aeon (`lang-l9-offsets`), 1 commit:**
- `games/sonic4/player/player_common.emp` (M) — the three tables flipped.

**sigil (`lang-l9-offsets`), 2 commits:**
- `crates/sigil-frontend-emp/tests/offsets_cross_module.rs` (A) — +5 tests.
- `docs/superpowers/notes/2026-08-02-l9-offsets-cross-module.md` (A) — this note.
- `docs/superpowers/notes/campaign-gap-ledger.md` (M) — the adoption-landed row.
