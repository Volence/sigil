# L1 — the game contract (interface / implement) — design

**Status: RATIFIED (Volence, 2026-08-01) — all seven §7 decisions ruled as
recommended.** D1 `interface`/`implement` · D2 explicit manifest ("declarative
is better — I hate having things less explicit") · D3 bound-proc `jsr`
("if code's not called how can we potentially not include it") · D4 entry IN
the interface, with the ruling's clarification recorded: the manifest is a
GAME-SIDE `.emp` module (`games/<game>/config/game.emp`) — game work never
touches engine files; Volence's TOML question resolved as the suite rule
"TOML for placement/build data, `.emp` for anything that names a symbol"
(the bindings are checked symbol references + a comptime conditional, which
TOML would demote to strings resolved by tooling) · D5 qualified `Game.`
refs · D6 as proposed, parcel-verified · D7 Config-A re-freeze accepted.
Parcels P1/P2 (§8) are GO.

**Original status: DRAFT — for Volence's review.** The language round's headliner
(agenda `2026-08-02-language-round-agenda.md` Tier 1, ratified BUILD with the
direction blessed: *the ENGINE declares the hook SIGNATURES as a typed
interface; a GAME provides the implementations as ordinary `.emp` procs bound
by declaration — no macro seam, no text extraction*). Evidence base: ledger
`notes/2026-08-02-language-round-ledger.md` §L1; kill-list rows 9, 45, 90;
K-capstone spec §0 (the named survivors) and §4 (the deferral that created
this item). Every claim below re-verified against the repos at masters
aeon `8516892` / sigil `3d57dba8`.

## §0 — The end-state (what "done" means)

- `games/sonic4/config/game.asm` and `games/demo/config/game.asm` are
  **DELETED** — the last game-authored `.asm` carrying semantics. The
  `game_root.asm` stubs lose their game.asm include (they keep debugger.asm,
  and sonic4's mt_syms include, per their own survivorship rulings).
- The engine is **literally game-agnostic**: `engine/system/boot.emp` and
  `engine/system/game_loop.emp` contain no sonic4 symbol names, no mirrored
  macro expansions, and no LOCKSTEP comments. Kill-rows 9 and 45 close;
  row 90 (Game_Entry) closes with them.
- The game contract is a **typed, engine-declared interface**: a game that
  implements a hook with a wrong signature, binds an unknown member, or omits
  a required member **fails to lower** — the drift class the row-9/45 combo
  matrices existed to contain becomes unrepresentable. (The matrices
  themselves were already retired at flip Stage-2; today's guard is the
  Config-A/canonical golden set. After L1 the goldens still gate bytes, but
  there is no mirror left to drift.)
- The `-D GAME_CAMERA_JUMP_LOCK` duplication (declared in game.asm AND
  hard-coded per-profile in `native.rs`) collapses to one declared source:
  the game's manifest.

## §1 — The contract surface, as evidenced

What `config/game.asm` ×2 actually carries today (everything else in the file
is documentation pointing at already-native homes — header.emp, sound_ids.emp,
sfx_bank.emp):

| item | sonic4 | demo | engine consumer |
|---|---|---|---|
| `GAME_CAMERA_JUMP_LOCK` | 1 | 0 | `camera.emp` comptime-selects code arms (×4 sites) |
| `Game_Entry` | `= GameState_OJZScroll_Init` | `= GameState_Demo_Init` | `boot.emp:301` `move.l #Game_Entry, (Game_State).w` |
| `GAME_ENTRY_ID` | `GS_OJZ_SCROLL_TEST` | `GS_DEMO` | `boot.emp:302` `move.b #GAME_ENTRY_ID, (Game_State_ID).w` |
| `gameBootHook` macro | ping + autoplay + `Dbg_Music_On` under `(SOUND_DEBUG_HOTKEYS, SOUND_DRIVER_ENABLED)` | empty | `boot.emp:286-294` — the row-45 MIRROR of the expansion |
| `gameDebugTick` macro | `jsr Debug_MusicToggle` under the same defines | empty | `game_loop.emp:37-44` — the row-9 MIRROR |

Two structural facts the design must honor:

1. **The mirrors are the engine impersonating the game.** boot.emp/game_loop.emp
   carry hand-copies of a *game* macro's expansion, each with an edit-both-together
   comment. Worse, `game_loop.emp` names `Debug_MusicToggle` — a
   `games.sonic4.game_debug` proc — directly from engine code, and boot.emp
   names `SONG_MOVINGTRUCKS` and `Dbg_Music_On`. The engine knows sonic4's
   debug internals by name.
2. **Empty must cost zero bytes.** Both hooks are empty in every canonical
   shape (hotkeys is an env opt-in) and in the demo always. Today's empty
   macro expansion emits nothing; the construct's empty/unbound case must
   also emit nothing, or five of six golden targets move for no reason.

## §2 — The construct

Two new item forms. Names are working names — see D1 in §7 for the keyword
question.

### Engine side — the declaration

```
// engine/system/game_contract.emp — emits nothing; pure declaration
module engine.game_contract

use engine.system.game_loop.{GameState}    // pub-hoisted by the parcel

pub interface Game {
    // comptime values — the engine's lowering consumes these
    const CAMERA_JUMP_LOCK: bool

    // the boot handoff
    const ENTRY_ID: u8
    proc entry: GameState

    // hooks: engine-invoked, game-implemented. `= empty` declares the
    // default — an unbound hook's call site emits NOTHING.
    hook boot_hook () clobbers(d0-d1/a0-a1) = empty
    hook debug_tick () clobbers(d0-d7/a0-a6) = empty
}
```

Member kinds, deliberately minimal (v1): `const` (typed comptime value),
`proc` (a reference to a game proc, typed by a declared proc type), and
`hook` (a proc signature the ENGINE calls, with an optional `empty` default).
The difference between `proc` and `hook`: a `proc` member is a value the
engine *takes the address of* (`#Game.entry`); a `hook` member is a call
site the engine *invokes* (`invoke Game.boot_hook`). No data members, no RAM
members, no multiple instances — see §9.

### Game side — the implementation

```
// games/sonic4/config/game.emp
module games.sonic4.game

use engine.game_contract.{Game}
use games.sonic4.constants.{GS_OJZ_SCROLL_TEST}

pub implement Game {
    const CAMERA_JUMP_LOCK = true
    const ENTRY_ID = GS_OJZ_SCROLL_TEST
    proc entry = GameState_OJZScroll_Init

    // conditional binding: the item-7a comptime-if-over-members precedent.
    // In every other shape both hooks stay `empty` — zero bytes, as today.
    if SOUND_DEBUG_HOTKEYS == 1 && SOUND_DRIVER_ENABLED == 1 {
        hook boot_hook = SoundTest_BootPing        // games.sonic4.game_debug
        hook debug_tick = Debug_MusicToggle        // games.sonic4.game_debug
    }
}
```

The demo's manifest is the whole file: three value bindings, no hook binds —
the empty defaults carry it, and the module documents at a glance that the
demo installs nothing.

`SoundTest_BootPing` is new game-side code: the ping+autoplay body moves
VERBATIM out of boot.emp's mirror into a `games.sonic4.game_debug` proc
(where `SONG_MOVINGTRUCKS` and `Dbg_Music_On` are in-family names instead of
cross-seam leaks). `Debug_MusicToggle` binds directly — no wrapper proc; a
hook binds to any proc satisfying the signature.

### Engine call/reference sites

```
// boot.emp — replaces the row-45 mirror block:
        invoke  Game.boot_hook          // jsr abs.l when bound; NOTHING when empty

// boot.emp — the handoff, unchanged shape:
        move.l  #Game.entry, (Game_State).w
        move.b  #Game.ENTRY_ID, (Game_State_ID).w

// game_loop.emp — replaces the row-9 mirror block:
        invoke  Game.debug_tick

// camera.emp — comptime, replaces the -D define:
        if Game.CAMERA_JUMP_LOCK { ... }
```

`invoke` lowers to **absolute `jsr <impl>`** when the hook is bound and to
**nothing** when it is `empty`. Absolute-always is by construction, not by
comment: a hook impl is game-side by definition, and an engine→game call must
stay placement-independent (the rule game_loop.emp:38-43 currently carries as
prose becomes the lowering rule).

## §3 — Binding model and semantics

- **One instance per build.** The build's `GameProfile` names the manifest
  module; the registry requires EXACTLY ONE `implement` per declared
  interface in the module set. Zero = error listing the unimplemented
  interface; two = error naming both sites. This is the ML-functor shape —
  the engine is parameterized over one `Game` structure — not typeclass
  dispatch.
- **Members resolve at bind time, before lowering.** `Game.CAMERA_JUMP_LOCK`
  is a comptime value in every engine module (mechanically it feeds the same
  comptime environment `-D` feeds today, so camera.emp's arms comptime-select
  identically). `Game.entry` / hook impls resolve to module symbols; the
  existing module-to-module link path carries them.
- **Conditional binding** uses comptime `if` over members inside `implement`
  (precedent: item-7a's `if DEBUG == 1 { }` over vars fields). Build-shape
  defines (`SOUND_DRIVER_ENABLED`, `SOUND_DEBUG_HOTKEYS`, `DEBUG`) remain
  ordinary `-D` inputs — they are config dimensions, orthogonal to the game
  contract; the manifest may consult them.
- **Required vs defaulted.** A member without a default is required
  (missing = bind error). Only `hook` members may declare `= empty` in v1;
  a defaulted-const variant is grammar-compatible but has no demand — add on
  demand.

## §4 — Typing and diagnostics (the correctness payoff)

The hook signature is a full contract-grammar v2 proc signature; at bind
time the impl must SATISFY it:

- **clobbers**: impl clobbers ⊆ declared hook clobbers. Today the row-9
  mirror's safety rests on a comment; after L1, binding `debug_tick` to a
  proc that clobbers more than the declared bound is a lower error with both
  sites named.
- **params/preserves**: exact agreement on declared registers, same rule as
  `jsr (a0) as T` typed dispatch.
- **kind/type**: `const` binds require the declared type (the L5 typed-const
  work this arc ships is the natural checker); `proc` binds require the
  declared proc type.

New diagnostics (each with a negative probe in the parcel):
`[contract.unimplemented]`, `[contract.duplicate-impl]`,
`[contract.unknown-member]` (binding a member the interface doesn't
declare — the reverse-drift guard), `[contract.member-kind]`,
`[contract.hook-signature]` (clobber/param excess, both sites cited),
`[contract.missing-member]` (required member unbound).

## §5 — Byte-identity expectations (the honest ledger)

- **Five of six targets byte-identical**: both canonical sonic4 shapes and
  Config-B run hotkeys-off → hooks unbound → `invoke` emits nothing; the
  handoff stores emit the same imm32/imm8 for the same values; camera's
  comptime arms select identically. The demo pair binds no hooks at all.
- **Config-A (hotkeys ON) moves, deliberately.** Today's boot carries the
  22-byte ping+autoplay INLINE; after L1 it carries a 6-byte `jsr
  SoundTest_BootPing` and the body lives in a game section (+rts). The
  game_loop site stays `jsr Debug_MusicToggle` byte-for-byte (direct bind, no
  wrapper). Net: one golden re-freeze of Config-A, anchors-verified per the
  standing procedure; boot-time single call, zero per-frame cost change.
- The alternative that avoids even that — an `inline` splice hook — is
  REJECTED as the recommendation (D3, §7): splice-at-call-site is the macro
  seam reborn with types on it, and "no macro seam" is the blessed direction.
  One routine re-freeze of a non-canonical golden is the cheaper price.

## §6 — What it retires

| artifact | fate |
|---|---|
| `games/sonic4/config/game.asm`, `games/demo/config/game.asm` | DELETED (the K spec §0 liberation) |
| boot.emp:283-294 mirror + LOCKSTEP comment | `invoke Game.boot_hook` (row 45 CLOSES) |
| game_loop.emp:34-44 mirror + LOCKSTEP comment | `invoke Game.debug_tick` (row 9 CLOSES) |
| `Game_Entry`/`GAME_ENTRY_ID` AS equalates + link externs via residual | `Game.entry`/`Game.ENTRY_ID` module refs (row 90 CLOSES — the numeric-fold arm already died in the K era; this removes the equalate itself) |
| `native.rs` per-profile `("GAME_CAMERA_JUMP_LOCK", n)` hardcodes (×4 sites) | derived from the manifest (single source) |
| engine's by-name knowledge of `Debug_MusicToggle` / `SONG_MOVINGTRUCKS` / `Dbg_Music_On` | gone — bound or moved game-side |
| `game_root.asm` game.asm include + its combo-matrix caveat comment | dropped; stubs shrink to debugger (+ sonic4 mt_syms) |

Explicitly NOT retired: `debugger.asm` (vendored, own ruling), `mt_syms`
(A2's item), the build-shape `-D` defines (not game contract), and the
`SFX_BLOB_BANK` Z80-seam value (already .emp-declared game-side; its engine
consumers are Z80 sections fed through the sound seam — absorbing it into
the interface is a possible follow-up, not v1).

## §7 — Decisions for review (each with my recommendation)

- **D1 — keywords.** `interface` / `implement` / `hook` / `invoke` / `empty`
  as spelled above. Considered: `contract` (collides with contract-grammar
  v2's register-effect meaning), `trait`/`impl` (imports Rust typeclass
  connotations we don't deliver — single instance, no dispatch), a bare
  `game { }` block (special-cases the one interface; the construct is worth
  having for future engine↔game seams — e.g. a `Debugger` interface when the
  own-debugger work lands). **Recommend `interface`/`implement` as the
  general form.**
- **D2 — binding mechanism: explicit manifest vs link-name convention.**
  Link-name convention (game just exports `pub proc gameBootHook` and the
  engine calls it) is less grammar, but it is inference — nothing declares
  which game module is *the* manifest, absence silently means empty, a typo'd
  name silently unbinds, and completeness is uncheckable. Declared-over-
  inferred IS the K philosophy, freshly re-proven at K5. **Recommend the
  explicit `implement` block.**
- **D3 — call semantics: bound-proc `jsr` vs inline splice.** Recommend
  bound-proc `jsr` with `empty`-elision (§5 rationale; Config-A re-freeze
  accepted).
- **D4 — does `entry` belong in the interface,** or stay a plain cross-module
  export the engine names? Keeping it in the interface makes boot.emp
  game-name-free and gives ENTRY_ID a type home; the alternative leaves boot
  naming a per-game symbol forever. **Recommend in.**
- **D5 — `Game.CAMERA_JUMP_LOCK` spelling at consumer sites.** Qualified
  member ref (as written) vs re-exported bare const. Qualified reads as what
  it is — a game-supplied fact — and greps cleanly. **Recommend qualified.**
- **D6 — hook clobber bounds.** `boot_hook` declared `clobbers(d0-d1/a0-a1)`
  (boot-time, registers free — generous is honest); `debug_tick` declared ⊤
  (`d0-d7/a0-a6`) matching the GameState dispatch bound it runs beside.
  Parcel verifies against the real impl contracts.
- **D7 — Config-A golden re-freeze** accepted as the parcel's expected byte
  motion (anchors must hold; ANY anchor move = STOP, standing rule).

## §8 — Parcel plan (after ratification)

- **P1 — the construct (sigil-only).** Parse `interface`/`implement`/`invoke`
  + member forms; bind pass (instance resolution, member typing, hook
  signature satisfaction); comptime-env feed for const members; lowering for
  `invoke` (jsr-or-nothing) and member refs; the §4 diagnostic set with
  negative probes; unit + lower tests. No aeon bytes move. Bars: strict
  suite green with new tests, no golden motion, repin untouched.
- **P2 — the conversion (paired aeon+sigil).** game_contract.emp declaration
  (+ GameState pub-hoist); manifests ×2; `SoundTest_BootPing` moves the
  ping+autoplay body game-side; boot/game_loop/camera flips; game.asm ×2
  DELETE + game_root.asm shrink; native.rs profiles read the manifest (the
  -D hardcodes die); kill-rows 9/45/90 closed with artifacts named. Bars:
  five-target byte identity vs chain-18, Config-A re-freeze with independent
  anchor verification, strict suite, six-target native gates green,
  `refreeze --check` chain intact, repin per byte-motion doctrine (5-site
  ripple sweep for the Config-A shape).
- Scale: P1 = M, P2 = M. One porter chain, P2 gated on P1's countersign.

## §9 — Non-goals / deferred

- **Multiple instances / dispatch** — one game per build is the model;
  Spec-5 dual-build death changes the BUILD count, not the instance count.
- **L2/L7 authoring DSLs** — separately ruled (defer to first content).
- **Data/RAM interface members** — no demand; the vars/RAM system already
  has its own declared surface.
- **`SFX_BLOB_BANK` and the Z80 sound seam** — stays with the sound seam
  (A1/A2's neighborhood), noted in §6.
- **Interface inheritance/composition** — YAGNI until a second real
  interface exists.
