# 2026-08-02 — L5 + L8: the type-layer parcel (close packet)

Status: Merge state lives in the campaign log, not here. Branch pair
`lang-l5l8-types` (aeon + sigil), both off the merged K5 tips (aeon `8516892`,
sigil `7643ee62`). Two stages, committed separately per side. **Six targets
byte-identical to the chain-18 goldens; strict 2904 → 2915 (+11 new tests, 0
retired — the one drift-guard probe was REPURPOSED, not removed).**

Branch tips:
- aeon: `c886f83` (L8) on top of `aedb89e` (L5).
- sigil: `dd1c24a7` (L8) on top of `11e0eaf2` (L5).

Six-target identity (CRC32/size): s4 `5f72b9c3`/412134 · s4.debug `e6171a80`/421970
· demo `55b70266`/90576 · demo.debug `6487a47c`/93073 (all four built directly and
crc-checked); config_a/config_b via the strict `native_offcanonical_full` gates.
`refreeze --check` OK (tip `k4-skeleton`, chain len 18, no re-freeze); `repin
--check` pins.rs unchanged.

---

## §1 — STAGE L5: fold an imported pub const's value at its definition site

### The bug (conv-f #21 demand site)

`games/sonic4/config/constants.emp:107` had `VRAM_TEST_MARKER : VramTile = $03F8`
with the real derivation `= VRAM_TEST_OBJ + $18` demoted to a comment — a base
change silently desynced the marker. Restoring the derivation failed the build:
`unknown name VRAM_TEST_OBJ`.

Root cause (NOT the type annotation — an untyped derivation failed identically):
`player_common.emp` imports only `VRAM_TEST_MARKER` via
`use games.sonic4.constants.{…, VRAM_TEST_MARKER}`. The resolver injects the
imported const's DECL as an ambient (`collect_pub_comptime`), and the consumer's
evaluator RE-EVALUATES its initializer expression in the CONSUMER's scope — where
the sibling `VRAM_TEST_OBJ` (not itself imported) is invisible. A List import
injects only the named consts; prelude/glob inject everything, which is why other
derived pub consts (`SND_REQ_PING = SND_REQ_BASE+$00`, same-module siblings) work —
they are never List-imported by name into a foreign consumer.

### The fix (sigil `crates/sigil-frontend-emp/src/resolve/mod.rs`)

Fold the value at its DEFINITION site — the const-value analogue of the
overlay-window stamp (`stamp_overlay_window`) and the harvest's "resolve from the
file's own items" (`eval_all_pub_consts`). When `collect_pub_comptime` clones a
`pub const` for injection, `fold_const_literal` resolves it against `def_file`
(siblings + the profile's `-D` defines visible) and rewrites the injected clone's
value to the resolved `i64` literal. The consumer reads a self-contained value; no
sibling name leaks into its namespace.

Best-effort, so ZERO regression by construction: a value that does not resolve
cleanly to an `i64` (a cross-module reference `def_file` alone cannot see — e.g.
`COLLECTED_PARK_ENTRY_SIZE = 1 + 2*COLLECTED_MASK_BYTES`; a non-int; an
out-of-range magnitude; a cycle) keeps its ORIGINAL expression, so every const the
consumer already resolved is unchanged. `defines`/`include_root` are threaded
`ambient_items → ambient_from_uses → collect_pub_comptime` so the fold sees the
profile's defines (a define-dependent const like `SONG_COUNT` folds with the same
`DEBUG` the consumer would have used — byte-neutral).

Definition-order/cycle: the fold reuses `resolve_const`'s lazy/memoized/cycle
machinery, so a cyclic initializer is a loud diagnostic (never a hang) exactly as
a same-module cycle is.

### Diagnostics / tests added (sigil `tests/eval_cross_module_const.rs`, +6)

- `typed_const_refs_typed_sibling` — the demand shape (`MARKER: V = BASE + $18`).
- `typed_const_refs_untyped_sibling`; `chain_of_two_siblings_resolves` (two-hop).
- `unimported_sibling_does_not_leak_into_consumer` — a non-imported sibling stays
  `unknown name` in the consumer (leak-free proof).
- `unknown_name_in_initializer_still_errors_cleanly`; `cyclic_initializer_is_a_loud_diagnostic`.

### The aeon change (`games/sonic4/config/constants.emp`)

`VRAM_TEST_MARKER : VramTile = VRAM_TEST_OBJ + $18` restored (folds to $03F8 —
BYTE-NEUTRAL; the intent is machine-checked again). No newtype arithmetic rule was
needed: `resolve_const` evaluates typed consts to bare ints (the annotation is not
applied at resolution), so `VramTile + int` here is plain-int arithmetic → $03F8.

### Corpus sweep result

Grepped every `.emp` for a `pub const` literal whose derivation was demoted to a
comment for this reason. **Exactly one: `VRAM_TEST_MARKER` (sonic4), now restored.**
The demo game's marker (`games/demo/config/constants.emp`) is a game-varying `-D`
(`VRAM_RING_PLACEHOLDER`), structurally not an `.emp` const — nothing to restore.
`COLLECTED_PARK_ENTRY_SIZE` is a LIVE derivation (references cross-module
`COLLECTED_MASK_BYTES`), not demoted; it is not imported by name, so the fold's
graceful fallback leaves it unchanged.

---

## §2 — STAGE L8: sound-id newtypes + typed extern

### The types (aeon `games/sonic4/config/sound_ids.emp`)

`engine.types` already exports `SongId = u8` and `SfxId = u8` (no `MusicId` — see
below). The authority now types its families: `SONG_MOVINGTRUCKS/DRUMTEST/HCZ2 :
SongId`, the nine `SFXID_* : SfxId`, and `SFXID_REV_LOOP : SfxId = SFXID_SPINDASH`
(a derived id — rides L5's initializer fix). `SONG_COUNT` stays a raw count (a
bound, not an id); the `SFXPRI_*` ladder stays untyped (priorities, not ids).
Byte-neutral: `eval_all_pub_consts` (the harvest) and `sound_authority_consts`
(the seam-1 sound eval) both read the VALUE through `resolve_const`, which drops
the annotation, so the harvested equs are the same numeric values.

**MusicId vs SongId:** the roadmap's "MusicId" is realized as `SongId` — that is
the name `engine.types` ships and every consumer uses. No second name for the same
family was invented; noting it here per the parcel's instruction.

### The typed-extern grammar (sigil)

A new declaration item `extern NAME: Type` — a typed reference to a value symbol
defined outside the module and resolved at link (a harvested game constant's
EquSym, an AS/emp equ). Referencing the name yields `Typed{ty, Label(name)}`: in
an immediate it erases to the same `ImmLink` fixup a bare symbol / `extern(..)`
takes (byte-identical to the untyped link name it replaces — the `boot.emp`
`moveq #SONG_MOVINGTRUCKS` precedent), while carrying the newtype for enforcement
in typed positions (an `SfxId` extern into a `SongId` data slot is `[emit.type]`).

Surface, spelled to fit the existing grammar (one-way-to-do-it):
- **parser.rs** — `extern IDENT :` opener, distinct from `extern proc` (the
  two-token peek already there) and the expression-position `extern("Sym")`
  builtin (peek2 is an `Ident`, never `(` or `proc`). Type annotation mandatory.
- **ast.rs** — `Item::ExternConst(ExternConstDecl { public, name, ty, span })`.
- **eval/mod.rs** — `extern_consts` index (populated in `index_items`).
- **eval/expr.rs** — `eval_path` resolves the name to `Typed{resolve_type(ty),
  Label(name)}`, after defines/consts and before the D-PP.3 link fallback.
- **eval/asm.rs** — immediate lowering unwraps `Typed{_, Label|LinkExpr}` to an
  `ImmLink` (the type erases in imm position; byte-identical).
- The name is a FOREIGN link symbol — deliberately NOT in `collect_defined`, so it
  stays raw (unrenamed), resolved by the harvest EquSym exactly like `extern proc`.

### The retirement (aeon `engine/sound/sound_api.emp`)

The two local `const SFXID_RING_RIGHT/LEFT: SfxId` mirrors AND their two
`ensure(extern(..)==..)` drift guards are DELETED, replaced by `extern
SFXID_RING_RIGHT: SfxId` / `extern SFXID_RING_LEFT: SfxId`. The `moveq #…, d0 as
SfxId` use sites are UNCHANGED and emit the same bytes (the value resolves through
the same harvest/link path). Value desync is now **impossible by construction**:
`games.sonic4.sound_ids` is the single authority, harvested; sound_api holds no
copy, so there is nothing to drift and no guard to keep. The engine stays
game-agnostic — no `use games.*`.

### Harvest / consumer verification

- `harvest_game_constants` reads `sound_ids.emp`'s pub consts via
  `eval_all_pub_consts` → `resolve_const` (type dropped) → same numeric equs. The
  seam-1 `sound_authority_consts` / seam_emit_config hardcode of `SFXID_REV_LOOP`
  (0xAB, gap-ledgered) is untouched — it does not read the typed authority.
- Every consumer of `SONG_*`/`SFXID_*` still lowers: the player files
  (`moveq #SFXID_ROLL/JUMP`, `move.b #SFXID_SPINDASH`), `game_debug`
  (`moveq #SONG_* … as SongId`, the SFX ordinal table), `boot.emp`
  (`moveq #SONG_MOVINGTRUCKS`), and `mt_bank.emp`'s local typed mirrors (an
  emit-context constraint, out of scope per conv-f2 §6) all build byte-identical.
- `mt_bank.emp` is NOT touched: it lowers in seam-2 isolation (no game module in
  scope), so its `SONG_*: SongId` mirrors are an emit-context constraint, not a
  removable mirror — the same ruling conv-f2 §6 made.

### Negative probes (sigil `tests/typed_extern.rs`, +5)

- `extern_const_parses_with_a_type`; `extern_const_requires_a_type` (type
  mandatory); `extern_disambiguates_from_the_extern_call_and_extern_proc`.
- `typed_extern_into_a_wrong_newtype_data_slot_is_an_error` — an `SfxId` extern
  into a `SongId` slot is `[emit.type]` (the enforcement a bare link name cannot
  give); `a_plain_int_still_fills_a_matching_newtype_slot` (the control).

The retired drift-guard's job was re-homed (not deleted): the tranche5
`doctored_immediate_mirror_fails_its_drift_guard` probe — whose premise (a
doctorable local mirror) no longer exists — became
`typed_extern_has_no_mirror_so_a_missing_authority_is_loud`: with the AS-side
truth OMITTING the two SFX ids, resolution fails naming them. A missing authority
is LOUD (no stale local fallback), which is the single-authority property that
replaces the runtime guard. `sound_api_port::assert_drift_guards` now asserts the
capture count is 0 (was 2).

### Test-harness note (byte-neutral)

The isolated `sound_api_port` region gate and the tranche5 `lower_sound_api` helper
now prepend `engine/system/types.emp` (a zero-byte pure-types module) so the
extern's newtype resolves at lowering — the old local const's `: SfxId` annotation
was never resolved (lazy, dropped), so the isolated builds did not need it before.
Region-neutral.

---

## §3 — STEP-3 (retrospect) vs STEP-5 (engine optimization)

**Step-3 (language/tooling asks discovered):**
- L5's fold is a general capability, not a one-site patch: any `use`-imported
  derived `pub const` now resolves its same-module siblings. It matches the
  harvest/overlay "resolve at home" philosophy and removes a real footgun (a
  derivation demoted to a literal loses machine-checking).
- **Could NOT close in this parcel (typed-extern's broader home):** the typed
  extern's ENFORCEMENT is strongest at a DATA slot (`[emit.type]`). In an
  immediate or a comptime comparison, the value wraps a `Label`, so
  `as_stored_int` is `None` and typed arithmetic/comparison degrade to a silent
  `Value::Poison` rather than a loud `[cross-type mix]`. sound_api does not
  exercise those positions, but a future consumer that compares two typed externs
  would want a loud diagnostic — a language ask: teach `eval_typed_binary` to
  report a cross-type error for a `Typed{_, Label/LinkExpr}` operand instead of
  silently poisoning. Gap-ledger candidate.
- **Test-harness provisioning:** the minimal `build_program` test harness cannot
  provision an external link symbol the way the real reachable-program build does
  (`report_unresolved` flags an un-provisioned foreign symbol), so the byte-level
  link-path coverage lives in the real build (ROM identity + the region gate). A
  tooling nicety would be a build_program test seam that injects a stub symbol
  table (the harvest analog), so typed-extern link resolution can be unit-tested
  in isolation. Ledger candidate.

**Step-5 (engine optimization observed, out of scope):** none. Both stages are
authority/ownership moves; no lowering changed, no ROM byte moved.

## §4 — NEITHER-BUCKET HEADLINES

- **A derived pub const that references a CROSS-MODULE name (not a same-module
  sibling) still cannot be List-imported and resolved** — the fold falls back to
  the raw expression, which fails in the consumer unless it also imports the base.
  L5 closes the same-module-sibling case (the demand); the cross-module case is a
  separate, harder problem the harvest already solves via `-D` seeding. Not a
  regression (the fallback preserves today's behavior exactly).
- **`resolve_const` drops a const's type annotation.** A `const X: T = v` resolves
  to a bare `Int(v)`, not `Typed{T, v}` — so a bare use of a typed const carries no
  enforcement at the use site (only the annotation-site range/documentation
  intent). The typed EXTERN is strictly stronger here: its reference carries the
  type in the value. If the campaign wants typed consts to enforce at use sites
  too, that is a distinct, larger change (apply-the-annotation-at-resolution) with
  its own byte-risk — a language ask, not this parcel.
- **The one authority that L8 could not fully consolidate is `SFXID_REV_LOOP`'s
  seam-1 hardcode (0xAB).** It stays as conv-f2 left it (gap-ledgered); typing the
  authority does not touch it (the seam does not read the typed module).

## §5 — FILE MANIFEST

**aeon (`lang-l5l8-types`):** L5 `aedb89e` M `games/sonic4/config/constants.emp`.
L8 `c886f83` M `engine/sound/sound_api.emp`, `games/sonic4/config/sound_ids.emp`.

**sigil (`lang-l5l8-types`):** L5 `11e0eaf2` M
`crates/sigil-frontend-emp/src/resolve/mod.rs`, +
`crates/sigil-frontend-emp/tests/eval_cross_module_const.rs`. L8 `dd1c24a7` M
`crates/sigil-frontend-emp/src/{ast.rs, parser.rs, eval/mod.rs, eval/expr.rs,
eval/asm.rs}`, `crates/sigil-cli/tests/{sound_api_port.rs,
tranche5_negative_probes.rs}`, `docs/superpowers/notes/twin-scaffolding-kill-list.md`
(row 10 closed), + `crates/sigil-frontend-emp/tests/typed_extern.rs`, this note.
