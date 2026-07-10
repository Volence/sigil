# Tranche 7 step-0 — engine/objects/collision.asm into the typed surface (2026-07-10)

Target: aeon `engine/objects/collision.asm` (232 ln, master `d3cf26b`) →
`engine/objects/collision.emp`. First ENGINE code file to port into the
construct-walk-#3 typed surface (Coord/Velocity/Radius/Angle live in its
hot path), and the ledgered second-consumer moment for the shared
engine-macros templates module (aabb.inc). Its step-5 engine review is the
point of the tranche (hot bug-fix file: the on-object bit lifecycle and the
Touch_Solid contact-face logic both carry recent fix comments).

## Gate: RESOLVED — structural changes retired

Re-asked at kickoff per the handoff. Volence: the "queued structural engine
changes" (tranche-4-close remark) was thinking out loud — nothing queued,
question retired permanently, do NOT carry it forward. collision.asm ports
as-is from aeon master.

## File facts (recon, verified in tree)

- **One pub label**: `TouchResponse` — external `jsr` consumers in
  `games/demo/demo_state.asm`, `games/sonic4/test/ojz_scroll_test.asm`,
  `games/sonic4/test/object_test_state.asm` (profiler wraps it). All other
  labels (`Touch_*`, locals) are file-internal.
- Include site: `engine/engine.inc:175`, between `animate.asm` and
  `rings.asm` — inside the fixed engine block, NOT the object bank.
- `aabb.inc` (41 ln): `aabb_axis_test` macro, guarded by `__AABB_INC__`.
  Second consumer = `engine/objects/rings.asm:223,230` (RingCollision, with
  `(a0)`/`2(a0)` operands) — rings STAYS AS-side this tranche.
- SST fields touched: code_addr, x_pos, y_pos (Coord — `.w` integer-word
  access only), x_vel, y_vel (Velocity), collision_resp, width_pixels,
  height_pixels (Radius), status. ALL already in the sst.emp twin.
- Constants needed at comptime (immediates / bit numbers): NUM_PLAYERS=2,
  NUM_DYNAMIC=40, NUM_SYSTEM=8, NUM_EFFECTS=16, COLLISION_TOUCH=12,
  ST_ON_OBJECT=5, ST_IN_AIR=3, ST_P1_STANDING=3. RAM labels `Player_1`,
  `Dynamic_Slots` are EA-position only → stay AS-owned over the link
  (spelled `(Player_1).w` — the explicit-width AbsSym form, AS byte parity).
- Instruction inventory hazards: `jsr .handler_table(pc, d4.w)` (pc-indexed
  jsr on a LOCAL label), `movea.w d0,a0` reg-stash, the bra.w×13 handler
  table, 11 stub labels aliased onto one `rts`, `dbf` loops, `lea
  SST_len(a3),a3` slot advance.

## Design decisions

**D1 — Module + gate.** `module engine.objects.collision in collision`,
one pinned section at the per-shape reference address (derive at step 1
from both-shape builds; DEBUG=1 overwrites s4.bin — manual `cp` for the
debug reference, per the tranche-6 process note). Gate
`SIGIL_EMP_COLLISION` in engine.inc wrapping the include, org-resume at
the region end per shape (exact `SIGIL_EMP_COLLISION_LOOKUP` pattern,
including the "sonic4-shape addresses, never set for other games" note —
demo builds take the include). `pub proc TouchResponse` is the only
export; `Touch_*` stay module-internal (non-pub, mangled — no namespace
noise). Mixed harness: module needs `types_ambient_items` (sst.emp →
types.emp ambient) like the tranche-6 object modules.

**D2 — The shared AABB template module (the ledgered demand moment).**
NEW zero-byte module `engine/objects/aabb.emp`:
`pub comptime fn aabb_axis_test(...) -> Code`, the .emp twin of
aabb.inc's macro. rings.asm still consumes the .inc AS-side → aabb.inc
becomes a tracked twin — **kill-list row 13**, kill condition = rings.asm
port (same-commit rule). Template shape stays FAITHFUL to the .inc
contract (test + branch-out on fail, single source of truth): args
(apos: Reg, breg: Reg, boff: int, adim: Reg, bdim: Reg, cdim/delt aliases
allowed as in the .inc, stmp: Reg, mlab: Label). The `utag` parameter
DIES — hygienic labels make the `.aov` internal branch fresh per
instantiation (probe-verified; retrospect item: a hygiene win to record
in the empyrean amendment stack). collision.emp calls it twice
(x_pos/y_pos offsets via `offsetof`-style comptime ints); the rings port
later calls it with boff 0/2.

**D3 — constants.emp twin grows a collision block**: the 8 comptime
constants above + one drift-guard `ensure(extern(...))` each, exactly the
RF/AF precedent. (COLLISION_NONE..SOLID_HURT are NOT taken — only
COLLISION_TOUCH appears in code; the jump table is positional. Jot the
full-enum idea to the gap ledger: an `offsets`-style collision-type enum
wants the whole family, but nothing demands it today.)

**D4 — Typed spellings.** Handlers with register contracts take typed
params: `proc Touch_Solid (a2: *Sst, a3: *Sst)` → bare field access off
both regs (`y_pos(a3)`, `status(a2)`; two same-typed pointer params
disambiguate by register). `TouchResponse` itself has NO register-argument
contract (reads Object_RAM) → no params; its field access uses the
qualified form `Sst.x_pos(a2)` (the 9a-era ruling: bare needs a typed
param, qualified always works). `.w` on Coord fields reads/writes the
integer word (BE high word at the field offset) — types erase, no width
police; byte gate proves it. Slot advance = `lea sizeof(Sst)(a3), a3`.

**D5 — Handler table hoists to a module-level proc.** `.handler_table`
(local label after TouchResponse's rts) becomes
`proc Touch_HandlerTable ()` — 13 `bra.w` entries, dispatched via
`jsr Touch_HandlerTable(pc, d4.w)`. Probe-verified end-to-end (4E BB 40 xx
+ 60 00 disp words decode exactly); labels emit no bytes, the pc-rel
displacement is identical → byte-neutral vs the local-label spelling.
(The local-label pc-idx form `.lbl(pc,dN)` does NOT parse today — see the
features ledger note below; the hoist is the house shape anyway: the
table is a standalone artifact, not TouchResponse control flow.)

**D6 — Stub aliasing = empty falls_into chain.**
`proc Touch_None () falls_into Touch_Enemy {}` × 10 → `Touch_Touch { rts }`
(+ separate `Touch_Hurt { rts }`, full `Touch_Solid`). Probe-verified:
empty procs emit zero bytes, all chain labels alias the single rts —
byte-identical to the AS 11-labels-one-rts shape, and the falls_into
declarations make the aliasing EXPLICIT (totality: a stub gaining a body
without a terminator fails the fallthrough lint instead of silently
running into its neighbor).

## Demanded features (step 1, the demanded-features law)

Probed against `sigil emp` at master; each gets negative probes + AS
byte-diff parity through the port gates.

**F1 — splice in displacement position: `{off}({reg})`.** Today
`2({breg})` parses (inner reg splice OK — probe c) but `{boff}(a3)` does
NOT (parse error at the splice-then-paren). The aabb template cannot
carry its bpos operand without it. Smallest possible grammar extension:
the displacement expression slot accepts a splice; disp evaluates to a
comptime int and range-checks exactly like a literal. Every future
engine-macro template with a memory operand hits this same wall — this is
the general fix, not a collision special.

**F2 — local-label values in call arguments: `axis_test(..., .next_object)`.**
D-PP.3 label-value fallback is bareword-only (module-level names —
probe c's `v3(Stub_A, d2)` works). The aabb call site's fail target
`.next_object` is proc-local, so `.name` must be accepted in call-arg
position and resolve through the ENCLOSING proc's hygienic label space
(the spliced Code lands in the same body, so the mangling context
matches; forward refs fine — resolution is at link). Fallback shape if
this turns out deeper than it looks: the template drops its trailing
`bhs.s` and call sites write the branch (byte-identical output, but the
.inc/.emp twin shapes diverge and the "single source of truth" claim
weakens — prefer F2).

**F3 — cross-module `pub comptime fn` import (found during step-1 prep).**
`pub comptime fn` parses (ComptimeFnDecl.public exists) but
resolve/imports.rs has no ComptimeFn arm — a shared template module
cannot export its templates. The aabb.emp module is the demand:
collision.emp must `use engine.objects.aabb.{aabb_axis_test}`. Mirror
Const's import treatment (comptime-only item, no bytes/link symbol).

NOT demanded here (stay ledgered): label values in imm exprs (no
objroutine store in this file — engine block, not object bank), `.b`
ImmLink, equ hygiene, clobbers() ranges.

## Byte gates + probes (step 1 checklist)

- Region pin: both shapes (plain + debug), collision region start = old
  TouchResponse address, org-resume = rings.asm start; re-derive from
  fresh master builds, manual debug copy (PROCESS: `DEBUG=1 ./build.sh`
  overwrites `s4.bin`/`s4.lst`).
- Byte gates both shapes + mixed-build acceptance + gate-off neutrality
  (18/18-class mixed gate run, now 16 modules).
- Negative probes: (1) doctored sst.emp offset fires the drift ensure;
  (2) doctored constants twin fires its ensure; (3) F1 wrong-kind splice
  in disp position → `[asm.splice-kind]`; (4) F2 unknown local label →
  loud error, not silent Sym; (5) falls_into-broken stub chain fails the
  fallthrough lint; (6) template stmp-aliasing misuse (stmp == cdim)
  still assembles to WRONG bytes in AS too — document, don't lint (the
  .inc has the same sharp edge; jot a `distinct-regs` template-contract
  ask to the ledger instead).
- clobbers: TouchResponse header documents d0-d7/a0-a3 — spell
  `clobbers(d0,d1,d2,d3,d4,d5,d6,d7,a0,a1,a2,a3)` (the reglist-range ask
  stays ledgered; this is its third long spelling — note it in retrospect).

## Step-5 preview (the point of this tranche — engine review, live-verified)

Candidates spotted at recon, to be cycle-argued and oracle-verified at
step 5, lockstep + re-pin as needed:

1. **Hoist player dims out of the object loop**: `width_pixels(a2)`/
   `height_pixels(a2)` are re-read (moveq+move.b ×2) per OBJECT, but only
   change per PLAYER. ~64 hot-loop iterations/frame × 2 players. Register
   pressure is the question (d0-d7 all live; a0/a1 free before the stash).
2. **`.overlap_done` reload runs even when no handler was dispatched**
   (`bhi.s .overlap_done` for types > COLLISION_TOUCH) — wasted 8 cycles
   on that path; reload only after the jsr.
3. `move.w #0, SST_y_vel(a2)` ×2 / `SST_x_vel(a2)` — `clr.w` wins 2 bytes
   each (tranche-6 clr precedent; 68000 clr RMW quirk is RAM-safe).
4. The bra.w table costs 4B/entry + double jump per dispatch; a word
   offset table (`dc.w Touch_*-Touch_HandlerTable` + `adda`/`jmp`) is
   2B/entry — but changes the dispatch instruction sequence; weigh at
   step 5, not before.
5. Profiler note: `Prof_TouchResponse` exists in ram.asm — use it for
   before/after cycle evidence in the oracle run (object_test_state
   already wraps TouchResponse with it).

## Retrospect seeds (carry into step 3)

- utag-parameter death = hygiene win over AS macros (empyrean amendment
  candidate).
- Register OUT contracts ask gets a THIRD data point: the d0-d3/a2-a3
  handler-entry convention is a register CONTRACT the language can't
  state (ledger row exists from the GetSineCosine ruling).
- Local typed reg binding inside a body (`TouchResponse`'s a2/a3 are
  *Sst for the whole body but only the qualified spelling is available) —
  possible `let a2: *Sst` ask; see how noisy the qualified form reads.
