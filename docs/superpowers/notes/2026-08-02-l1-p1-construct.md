# L1 P1 — the game-contract construct (sigil-only) — close packet

Branch `l1-p1-contract`. Implements the L1 construct per the ratified spec
`specs/2026-08-02-l1-game-contract-design.md` §2–§4. No aeon bytes move; the
conversion (game.asm deletes, boot/game_loop/camera flips, native.rs) is P2.

Strict gate **2938 / 0 / 4** (2920 baseline + 18 new). `refreeze --check` OK
(tip `k4-skeleton`, chain 18). `repin --check`: pins.rs unchanged. Clippy: no new
warnings in the L1 code (the one pre-existing `sigil-ir/symbols.rs` and Z80
indexed-operand nits are untouched).

## Grammar as shipped (matches spec §2 — no shape changes needed)

Engine side:

```
pub interface Game {
    const CAMERA_JUMP_LOCK: bool          // typed comptime value
    const ENTRY_ID: u8
    proc entry: GameState                 // ref, typed by a `type = proc` contract
    hook boot_hook () clobbers(d0-d1/a0-a1) = empty   // engine-invoked; empty default
    hook debug_tick () clobbers(d0-d7/a0-a6) = empty
}
```

Game side:

```
pub implement Game {
    const CAMERA_JUMP_LOCK = true
    const ENTRY_ID = GS_OJZ_SCROLL_TEST
    proc entry = GameState_OJZScroll_Init
    if SOUND_DEBUG_HOTKEYS == 1 {          // comptime-if group over bindings
        hook boot_hook = SoundTest_BootPing
        hook debug_tick = Debug_MusicToggle
    }
}
```

Consumer sites (engine code): `invoke Game.boot_hook` (statement),
`move.l #Game.entry, (…)` and `move.b #Game.ENTRY_ID, (…)` (immediates),
`if Game.CAMERA_JUMP_LOCK { … }` (comptime-if).

- Keywords `interface`/`implement`/`invoke` are **contextual openers** (D2.36):
  an item/statement only when the keyword is immediately followed by the naming
  identifier. `hook`/`empty` are keywords only inside the interface/implement
  block parsers (a hook member opener; the `= empty` RHS). Verified against the
  aeon corpus — no collision (`hook`/`empty` occur only in comments and
  `Data.empty`, which stays its own special case). `contract_keywords_do_not_
  break_ordinary_identifiers` pins `const hook`/`const empty`/`const interface`.

## Bind-pass architecture (where it lives and why)

- **`crate::contract`** (crate root, Core-free): the resolved vocabulary —
  `InterfaceEnv` → `ResolvedInterface` → `ResolvedMember::{Const(Value),
  Proc(sym), Hook(Option<sym>)}`. Core-free so the evaluator (D-P4.1) can carry
  it; the producer (`resolve`) may use the evaluator, so the types live below it.

- **`crate::resolve::contract::bind(&[ContractModule], defines)`**: the §3 bind
  pass. Collects every `interface` + `implement` across the module set (one level
  into `section {}`). Enforces exactly-one-implement (0 → unimplemented, ≥2 →
  duplicate-impl). Flattens comptime-`if` binding groups against the build-shape
  defines (via the new `eval::eval_expr_in_file`). Types each member: const →
  fold + kind sanity; proc → link symbol + declared proc-contract-type check;
  hook → link symbol + the §4 subcontract relation via
  `closure::subcontract_violations` (impl clobbers ⊆ declared, preserves ⊇,
  params ⊆, out ⊇), building each side's `Contract` from its reglists with
  `regfile::expand_reglist`. A bound proc that is not a declared `proc`/`extern
  proc` in scope, or that declares no clobber contract, is an unbounded boundary
  and the signature check skips it (opt-in contracts, exactly like the closure's
  unknown-callee leaf).

- **Evaluator consumption**: `Evaluator::seed_interfaces(&InterfaceEnv)` (paired
  with `seed_defines`). `eval_path`'s 2-segment branch resolves `Iface.MEMBER`
  AFTER a genuine value receiver (so an interface never shadows a local/const/
  data) — const → value, proc → `Value::Label(sym)` (the same link-deferred
  imm32 a bare symbol takes), hook → member-kind error. `lower_asm_stmt`'s new
  `Invoke` arm synthesizes `jsr (sym).l` (the `abs_l` shape) and lowers it
  recursively when bound, emits **nothing** when `Hook(None)`.

- **Threading**: a new public `lower::lower_module_with_contracts(file, opts,
  &InterfaceEnv)` threads the env through a `contracts` field on `ProcCtx` and a
  `contracts` arg on `eval_proc_body(_env)`. `lower_module` /
  `lower_module_with_region_ends` / `build_program` are UNCHANGED — they call
  through with `InterfaceEnv::empty()`, so the whole existing corpus and every
  golden build is byte-identical. This is why **`LowerOptions` gained no field**
  (that would have broken all 151 construction sites); the seam is a sibling
  entry point, which P2 will call from the registry/harness.

## Diagnostic set (each with a negative probe)

| diagnostic | site | probe test |
|---|---|---|
| `[contract.unimplemented]` | bind | `probe_unimplemented` |
| `[contract.duplicate-impl]` | bind (also a duplicate interface decl) | `probe_duplicate_impl` |
| `[contract.unknown-member]` | bind (binding an undeclared member; also invoke-site unknown) | `probe_unknown_member` |
| `[contract.member-kind]` | bind (kind mismatch; also value-ref/invoke on a hook) | `probe_member_kind` |
| `[contract.hook-signature]` | bind (subcontract violation, both sites cited) | `probe_hook_signature` |
| `[contract.missing-member]` | bind (required member/hook unbound) | `probe_missing_member`, `required_hook_without_empty_default_must_be_bound` |

`well_formed_contract_binds_clean` is the no-diagnostics control. The lower
tests (`bound_hook_emits_absolute_jsr`, `empty_hook_emits_nothing`,
`empty_and_bound_differ_by_exactly_the_jsr`, `conditional_binding_flips_on_a_
define`, `const_member_feeds_a_comptime_if`, `proc_member_lowers_as_a_link_
imm32`) byte-compare the emitted image (jsr abs.l = `4E B9 …`, imm32 =
`20 3C …`, empty = the bare `rts`).

## Spec deviations (spec is ratified — each flagged; none semantic)

None change the grammar or the ruled semantics. Scope boundaries within P1:

1. **Const-member type enforcement is LIGHT in P1** (a kind sanity check — a
   non-value bound to a const is rejected; a full type/range check is not run).
   Spec §4 ties const typing to "the L5 typed-const work this arc ships is the
   natural checker", and the enforcement rides the USE site exactly as `extern
   NAME: Type` does (typed_extern's model). The folded value flows to consumers;
   full typed-const binding is the natural L5 follow-up.

2. **Interface-member references resolve in code (proc-body) position.** Every
   real consumer is a proc body (boot.emp `invoke`/`#Game.entry`/`#Game.ENTRY_ID`,
   game_loop.emp `invoke`, camera.emp `if Game.CAMERA_JUMP_LOCK`), so contracts
   are threaded to proc-body eval. Top-level `const`/`data` items that name an
   interface member are not wired in P1 (that would thread `contracts` to the
   const/data eval entries too — a mechanical extension); no P1/P2 consumer needs
   it. Dispatch-inline-body and `script` bodies also pass the empty env (not
   contract consumers today); an `invoke` there reports unknown-member rather
   than silently vanishing.

3. **Bind evaluates binding values/conditions in the impl module's own scope +
   defines** (no cross-module ambient injection). P1 tests keep referenced consts
   local or use `-D`. The `build_program` ambient-prepend (P2's registry
   integration) supplies cross-module `use`d game consts.

4. **Invoke-site + duplicate-interface reuse existing diagnostic names.** Invoke
   on a non-hook → `[contract.member-kind]`; invoke on an unknown member/unimpl
   interface → `[contract.unknown-member]`. A duplicate interface DECLARATION
   (same name twice) → `[contract.duplicate-impl]`. These extend the spec's named
   set to the natural use/decl sites rather than inventing new ids.

## step-3 / step-5 / neither breakdown

- **step-3 (retrospect / language asks)**: none new — L1 *is* the language ask
  this parcel builds. One observation for the ledger: the const-member type
  layer (deviation 1) and top-level const/data member refs (deviation 2) are the
  two natural L5/P2 follow-ups; both are additive, neither blocks P2.
- **step-5 (engine optimize)**: n/a — sigil-only, no engine code touched.
- **neither-bucket**: the `lower_module_with_contracts` sibling-entry pattern
  (vs. a `LowerOptions` field) is the load-bearing architecture choice — it keeps
  the 151 `LowerOptions` sites and `build_program` untouched, which is what makes
  "no golden byte moves" structurally true for P1 rather than test-verified. P2
  should call `lower_module_with_contracts` from the whole-program path (feeding
  it `bind`'s env over the reachable module set) and provide the ambient for
  cross-module binding values.
