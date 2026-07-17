# Contract-grammar v2 — register-effect contracts, verified and transitive (design spec)

**Fable, 2026-07-17.** The adjudicated build source for the diagnostics tier's register-contract
net (2026-07-16 review adjudication: "contract-grammar-v2 spec = D1a-d + D2 CCR/must-use +
row-1069 typed reg signatures + extern-contract trust boundary + indirect-call bounds, ONE
grammar, Fable drafts"). The review's D-section is the EVIDENCE BASE; this spec is what gets
built. **Pass-3 (object/render contract surgery) is gated on G1+G2 of this spec.**

Evidence inputs (all on master):

- Review `aeon docs/reviews/2026-07-16-emp-port-optimization-review.md` §D1 (a–d), §D2, and the
  adjudication amendments (extern-proc contracts, indirect-call bounds, scaffolding annotation).
- Census `notes/2026-07-17-diagnostics-contract-census.md` + TSV (126 procs; 13 under-decl
  retrofit targets + 3 SAT-pointer `out(a4)`s; 14 no-contract procs; **3 extern calls**;
  **6 indirect sites / 5 mechanisms** with prose bounds; `preserves()` has **0 adopters**).
- Gap-ledger row 1030 (RE-OPENED: individual-push preservation inexpressible — ≥3 live FPs) and
  the typed-asm-proc-register-signature ask (shared-struct item-5 balloon; row 1054 re-keyed
  onto it).
- Existing machinery: SPEC2 D2.32 (`preserves`, syntactic movem slice), D2.35 (`out` metadata +
  the deferred `out(a1) if cc` sibling), `check_clobbers`/`check_out`/`instr_written_regs`
  (post auto-inc/dec fix, sigil 127409f/c8b0e52).

---

## 1. The model

Every proc has a **register-effect contract** — a partition of the register file plus flag
effects, seen from the caller:

| Clause | Meaning to the caller |
|---|---|
| params `(rN: Type, …)` | **inputs** — registers the proc READS on entry (D1b; typed slots per row-1069) |
| `out(rN[: Type][, …])` | **results** — written, live-out, caller may read (D2.35, extended) |
| `out(carry: name [if cc])` | **flag result** — a status-flag-encoded result the caller MUST consume (D2) |
| `clobbers(…)` | **scratch** — destroyed, caller must not rely on |
| `preserves(…)` | **untouched** — value at return == value at entry, verified (v2: by dataflow, not just movem shape) |

Undeclared registers stay in today's default bucket (heuristic warn on write). A proc with no
contract at all gets an **inferred** contract (its computed effect) so the closure below stays
total — inference is for the graph, never a substitute for the declared+verified tier.

**The v2 upgrade is verification + transitivity.** Today's `check_clobbers` is local and
heuristic. v2 computes each proc's **effective clobber set** over the call-graph closure:

```
effective(P) = localWrites(P)
             ∪ ⋃ { effective(C) | C ∈ directCallees(P) }
             ∪ ⋃ { bound(S).clobbers | S ∈ indirectSites(P) }
             − verifiedPreserved(P)
```

- `directCallees` resolve through the link (Sigil owns the link — D7's same substrate).
- **Indirect sites use their declared bound (§4). An unbounded indirect call is ⊤ (all
  registers).** This is the load-bearing ordering fact: `RunObjects` sits on nearly every
  frame path and dispatches `jsr (a1)` — without a bound there, ⊤ propagates to the whole
  graph and transitivity is useless. Therefore **boundary declarations (extern §3 + indirect
  §4) ship IN G1 with the closure, not as a later phase.**
- Extern callees use their `extern proc` declaration (§3); an undeclared extern call is a hole
  in the closure — error under strict once G1 lands. **G1-checkpoint erratum (2026-07-17): the
  boundary is 5 declarations, not the census's 3** — the closure found `QueueDMA_Important`/
  `QueueDMA_Deferrable` called via a comptime-fn PARAMETER (`perform_dplc(QueueDMA_*)`,
  dplc.emp), invisible to the census's textual call scan. Lesson: calls passed as comptime-fn
  arguments only surface in the lowered closure — the closure, not a text census, is the
  boundary authority. (`Debug_MusicToggle` is SOUND_DEBUG_HOTKEYS-gated: not a plain-build
  hole, still declared for debug-shape analysis.)
- `verifiedPreserved` is subtraction earned by proof (§5): a callee effect the proc provably
  saves/restores around the call does not escape it.
- Recursion/SCCs: fixpoint from ∅ (monotone union on a finite lattice — terminates).
- `sr` participates as today ([proc.sr-undeclared] stays); full CCR dataflow stays S2-D7 —
  the ONLY flag analysis v2 takes is the bounded call-site consumption check in §6.

## 2. Surface grammar

No new clause for inputs — **the param list IS `in()`** (D1b unified with row-1069): it already
exists (`proc wait (a0: *Sst)`), has the right shape, and the census's `// In:` comments
retrofit into it mechanically. Bare-register params (`(d2)`) are legal = declared untyped
input; typed params (`(d2: GridCoord)`) add the §7 slot-type checks. This is deliberate
no-ceremony: nothing new to learn, one mechanism gains teeth.

```
proc  Section_FlatIDXY (d2: GridCoord, d3: GridCoord) out(d0: SectionId) clobbers(d1) { … }
proc  AllocDynamic () clobbers(d0) out(a1) preserves(a0) { … }        // §5 makes this legal
proc  QueueDMA (a1: *DmaSrc, d0, d1) clobbers(d0,d1,a1) out(carry: dropped) { … }   // §6
extern proc VSync_Wait () clobbers(d0)                                 // §3
type  HBlankHandler = proc () clobbers(d0, d1, a0)                     // §4 contract type
@scaffolding("VInt_Lag race fix — forward reset hook")                 // §8
pub proc Plane_Buffer_Reset () { … }
```

Spelling stays house-consistent: `clobbers`/`out` comma lists, `preserves` movem-reglist
(`/`-ranges), attributes in the existing `@` style.

## 3. Extern-proc declarations (the .asm trust boundary)

`extern proc <Name> (params) clobbers(…) preserves(…) out(…)` declares the contract of a
routine defined in no `.emp` — the caller-side statement of the `.asm` header, checkable where
today there is prose or nothing.

- **Placement:** at first use, in the calling module. On a second consumer, hoist to a shared
  home (the row-1073 second-consumer rule; with 3 total calls today, do not pre-build a module).
- **Truth & drift:** the `.asm` header stays the source of truth. The declaration is a MIRROR,
  so it takes the standing mirror discipline: a `// drift-guard:` comment citing the `.asm`
  file:line, and a kill-list row per declaration (kill condition: the callee ports to `.emp`
  and the extern decl is deleted in the same commit — the census note's own recommendation).
  There is no mechanical cross-seam verifier for a HEADER COMMENT; this is the one contract
  tier that rests on review, which is why the boundary must stay small (it is: 3).
- **Closure role:** extern contracts are leaves — `effective(extern P) = declared clobbers`.
- The three retrofits, verbatim from the census: `VSync_Wait () clobbers(d0)`;
  `S4LZ_DecompressDict (a4: *DictBase, d4)` with `clobbers(a3, a4)` + advances-a1 modelled as
  `out(a1)` (in-out cursor, exactly the DrawRings pattern deliverable 2 legalized);
  `Debug_MusicToggle () clobbers(d0-d2, a0, a1)`.

## 4. Indirect-call contract bounds

A **contract type** names the contract every installable target of a dispatch must satisfy:

```
type ObjRoutine    = proc (a0: *Sst) preserves(a0, d7)     // census sites #1/#2/#3
type TouchHandler  = proc (a0: *Sst)                        // #4 — caller saves d6-d7/a2-a4
type GameState     = proc () clobbers(d0-d7/a0-a6)          // #5 — honest ⊤
type HBlankHandler = proc () clobbers(d0, d1, a0)           // #6 — interrupt context
```

- **Dispatch sites** annotate the call: `jsr (a1) as ObjRoutine` / `jsr (a0, d4.w) as
  TouchHandler`. The closure uses the bound's clobbers for that site (§1); the annotation also
  arms the caller-side checks of §6 against the bound.
- **Targets** are checked at the places they become installable, at compile time:
  - a jump/dispatch **table defined in `.emp`** declares its element type
    (`Touch_HandlerTable : [TouchHandler; N]`) — every entry's contract is checked;
  - a **pointer install** (`move.l #Target, (Ptr)`) where `Ptr` carries a contract type checks
    `Target` at the install site. RAM cells still defined in `ram.asm` can't carry the type
    yet — the dispatch-site `as` bound still protects the closure; install-side coverage for
    those arrives when the owning file ports (log per site, don't block).
- **Subcontract relation** (`target ⊑ bound` — what makes a target installable):
  `target.clobbers ⊆ bound.clobbers` · `target.preserves ⊇ bound.preserves` ·
  `target.params ⊆ bound.params` (may use fewer inputs, never more) ·
  `target.out ⊇ bound.out` (must produce everything the caller may read).
- Diagnostics: `[dispatch.unbounded]` (warn in G1, error under strict once the 6 sites are
  annotated), `[dispatch.target-exceeds-bound]` (error; names the offending register and
  clause).
- This formalizes bounds that already exist in prose + a DEBUG rail (`Debug_AssertObjLoop`);
  the runtime rail stays (belt and braces on real hardware).

## 5. Verified `preserves` (the dataflow upgrade — answers row 1030)

D2.32's syntactic movem-pair slice has **zero corpus adopters** because real preservation in
this engine is individual pushes, often branch-straddling (`AllocDynamic`,
`Collected_Park/UnparkSlot` — the census's 3 live FPs). v2 verifies `preserves(rN)` by
**symbolic stack tracking**, not shape:

- Per control-flow path from entry, track sp depth and which stack slot holds which register's
  ENTRY value (writes to a register kill its tracked slots' "entry-value" status only for the
  register copy, not the slot). `preserves(rN)` holds iff on EVERY return path, rN's value at
  `rts` equals its entry value — restored from the matching slot (or never written at all).
- Soundness bailouts (assembly is assembly): computed sp, sp escaping into an address
  calculation, a write through a pointer that could alias tracked slots (any `(aN)`-relative
  store while inside a saved-region window whose base could be sp) ⇒ that path is
  **unverifiable** → `[proc.preserves-unverifiable]`, error-tier for a DECLARED preserves (a
  wrong contract is worse than none — the D2.32 principle, kept).
- The movem pair remains as the trivial fast path of the same analysis. D2.32's diagnostics
  keep their names; `[proc.preserves-missing-pair]` retires (subsumed).
- Interaction with the clobber lint: a verified-preserved register is removed from the
  computed write set (it's how `AllocDynamic` finally declares `preserves(a0)` and the 3 FPs
  die honestly, per the census's retrofit instruction — never with a false `clobbers(a0)`).
- This same analysis powers `verifiedPreserved` in the closure (§1) and D1d's dead-save lint
  (§6): they are one pass, three consumers.

## 6. Caller-side checks (D1b / D1c / D1d / D2)

All keyed off the callee's (declared or bound) contract at each call site; all need only
LOCAL def-use in the caller body — no interprocedural state beyond the contract itself.

- **[call.input-undefined]** (D1b, error): a register param of the callee has no reaching
  definition at the call site on some path. *Evidence: the AnimateSprite d3/DUR_DYNAMIC bug —
  shipped in two templates.*
- **[call.live-clobbered]** (D1c, error): a value defined before the call and read after it,
  held in a register in the callee's effective clobber set. This is the check that makes
  pass-3's "trust the contract" hoists safe — and it's why pass-3 waits for G1+G2.
- **[proc.dead-save]** (D1d, warn/perf tier): a §5-verified save/restore pair for a register
  the callee provably preserves. This lint's firing list IS a chunk of the pass-3 backlog
  (dplc ~575 cyc/frame-change, load_object ~76/spawn, children 44-116/child) — it lands with
  G2 so pass-3 starts from a machine-generated worklist.
- **[call.flag-result-unused]** (D2, error): a callee declaring `out(carry: name)` was called,
  and no path reads the flag (Bcc/Scc/ADDX-class consumer) before an instruction that
  redefines it. Explicit opt-out at the call site: `@discards(name)` — greppable, honest.
  *Evidence: Palette_Dirty cleared-on-drop + load_art's ignored Critical carry — both real
  silent-corruption bugs.* The conditional register form `out(a1 if cc)` (D2.35's deferred
  sibling) rides the same call-site machinery: reading a1 on the path where cc says it's
  invalid = `[call.result-invalid-path]` (error). Scope fence: this is per-call-site flag
  def-use ONLY — full CCR liveness stays S2-D7.

## 7. Typed register slots (row-1069 / re-keyed row 1054)

Params and `out` slots optionally carry newtypes: `(d2: GridCoord)`, `out(d0: SectionId)`.
Check: at every call site, the argument register's reaching definition must be
construction-compatible with the slot's type (same newtype, its base via explicit
construction, or a declared coercion) — the check a documentary `let rN: Type` cannot give
across a `jbsr`. Unlocks: SectionId/GridCoord at the FlatIDXY seam (~4 call sites, the
motivating case), and the prelude newtype backlog ([[emp-sonic-newtype-candidates]]) gains its
enforcement surface. Untyped slots check nothing (adoption-optional, no ceremony tax).

## 8. `@scaffolding` (adjudication amendment, D7 adjunct)

`@scaffolding("reason")` on a decl marks a ratified zero-caller keep (e.g.
`Plane_Buffer_Reset`). Semantics now: inert metadata + the reason string is mandatory.
Semantics when D7 lands: suppresses dead-symbol/zero-caller findings for that decl; unmarked
dead symbols still fail. Shipping the attribute in G1 lets the corpus annotate as it
retrofits, so D7's first run is clean instead of noisy.

## 9. Adoption modes & the retrofit (per the adjudication)

- **Auto-enforcing (error tier, first compile IS the retrofit):** transitive clobber
  verification against DECLARED contracts (§1), extern-hole (§3), declared-preserves
  verification (§5), flag-result must-use (§6). Known corpus debt the first strict compile
  will surface, from the census: the **13 under-declarations** (retrofit: add the register to
  `clobbers` — except the **3 SAT a4s which are `out(a4)`**, and `Section_RedrawPlanes` which
  is `clobbers(sr)`), the **11 Touch stubs + GameState_Idle** (`clobbers()`), the **3 FPs**
  (§5 `preserves(a0)`, never false clobbers). **Tier-timing ruling (G1 checkpoint,
  2026-07-17): the closure firing check ships WARN-tier through G1/G2 and flips to ERROR as
  G3's closing act, once the AllocDynamic-FP residue (the 3 direct FPs + the 2 transitive
  rows they leak into, Load_Object/Sound_PlayRing a0) provably reaches 0 via verified
  `preserves(a0)`.** Rationale: those 5 firings CANNOT be honestly retrofitted before §5
  exists (a false `clobbers(a0)` is forbidden), and building a temporary suppression
  mechanism for a known two-phase window is worse than a documented warn window. The
  error flip is a one-line tier change with a zero-residue precondition.
- **Adoption-requiring (annotation tier):** extern decls (5, per the §3 erratum), contract types + `as` bounds
  (6 sites / 4 types), typed slots, `@scaffolding`. Each ships its retrofit sweep in the same
  commit as its mechanism (the standing retro rule).
- The `.asm` tier mirrors best-effort in s4lint (warning-only, per the W021 pattern): W021
  already approximates §1-local; the growth list's D3-lite and dbf-invariant lints stay a
  separate parcel. Nothing here waits on a port.

## 10. Build order (Opus tranches; each = grammar + check + tests + same-commit retrofit)

| Phase | Contents | Gates |
|---|---|---|
| **G1** | Closure + transitive clobbers; extern-proc decls (3 retrofits); contract types + `as` bounds (4 types, 6 sites); `@scaffolding`; the 13+12 clobbers/out retrofits | **unblocks nothing alone — G1+G2 = the pass-3 gate.** Boundary decls are IN G1 by necessity (§1: unbounded indirect = ⊤ poisons the closure) |
| **G2** | `out(carry:)` + `@discards` + `[call.flag-result-unused]`; conditional `out(rN if cc)` | with G1: **pass-3 unblocks** (adjudication: "waits for D1a/D2") |
| **G3** | Verified preserves (§5) + the 3 FP retrofits + `[proc.dead-save]` (D1d) | closes row 1030; emits pass-3's dead-save worklist |
| **G4** | `[call.input-undefined]` (D1b) + `[call.live-clobbered]` (D1c) + `// In:`→param retrofit sweep | D1c hardens pass-3 mid-flight; order G3→G4 swappable if pass-3 wants dead-saves later |
| **G5** | Typed slots (§7) + GridCoord/SectionId at the FlatIDXY seam | closes row 1054's re-key |

Byte-neutrality: every phase is lint/metadata — **zero codegen change, byte gates stay green
throughout**; the retrofit sweeps are contract-text-only. Strict-suite + failures-first +
5-site ripple discipline apply as everywhere (ripple sites shouldn't move, but VERIFY, don't
assume).

## 11. Open implementation questions (Opus decides, note in the tranche packet)

1. CFG granularity for §5/§6 path checks — the lowerer's basic-block view vs a lightweight
   CFG over the emitted instruction list. (§5 needs joins; straight-line-only is NOT enough —
   that's exactly the stale-1030 mistake.)
2. Where the call graph lives — link layer owns symbol resolution; the closure wants to run
   post-link with diagnostics mapped back to source spans.
3. `@discards` attachment point — trailing attribute on the call instruction vs a standalone
   `discard <name>` statement immediately after; pick whichever the parser takes cleanly,
   both are greppable.
4. Whether `extern proc` participates in module resolution as a real symbol decl (preferred —
   it should collide loudly if the callee later ports and both exist).

**Not in scope** (stay their own rows): D4 cycle budgets, D5 hardware typestate, D6 context
ownership, D7 beyond `@scaffolding`, D8-D11, full CCR liveness (S2-D7), jbcc.
