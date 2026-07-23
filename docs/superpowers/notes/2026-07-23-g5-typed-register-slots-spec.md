# G5 — typed register slots (§7 tier 5): domain newtypes at the FlatIDXY seam

**Author:** Opus (Phase-3 lead). **Status:** SPEC to the design gate — *no implementation until ruled.*
**Grounding:** SIGIL_SPEC2_LANGUAGE.md §7 register-contract tiers + gap-ledger rows **1054** (SectionId/
GridCoord cross-file newtype) & **1069** (typed asm-proc register signatures). Design seed: the item-5
sketch `2026-07-16-item5-sectionid-gridcoord-sketch.md` (commit b487f83). Baseline canonical (settled
by the Phase-2.5 merge): plain `406c773b`/421122 · debug `5752c2e3`/429107.

---

## 0 · Stage-0 existence check — the sketch's premise is PARTLY RETIRED (read this first)

The item-5 sketch (2026-07-16) said the seam typing "balloons" because **no register-typing mechanism
exists** ("every proc is `proc Foo ()` with an EMPTY param list; `out(dN)` names the register not a type;
`let rN: Type` has zero corpus usage"). **Existence-checked at current master `033865f` — that is no
longer true.** Port modernization shipped most of the mechanism ([[stage-0-every-pass3-parcel]] again):

| Capability | State at master | Evidence |
|---|---|---|
| Typed register **input** params | **SHIPPED** — 43 / 122 procs use them | `proc Section_FlatIDXY (d2: u8, d3: u8, a2: *Act)` (section.emp:109); the D2.22 "param-typed registers" that enable `Act.grid_w(a2)` field access |
| Typed **flag** output `out(flag: Name)` | **SHIPPED** — 4 sites | `out(carry: full)` (rings.emp:50), `out(carry: dropped)` (dplc.emp:30/32) |
| Typed **data-register** output `out(dN: Type)` | **GAP** | zero corpus hits for `out(dN: Type)`; `Section_FlatIDXY … out(d0)` is untyped |
| Domain newtypes `GridCoord`/`SectionId` | **GAP** | not in types.emp (which has Coord/Angle/VramArtTile/… — the precedent) |
| Cross-`jbsr` call-site type **checking** | **GAP / design-open** | callers are bare `jbsr Section_FlatIDXY` with d2/d3 pre-set — no call-site coercion today |

**So G5 is a much smaller, well-bounded increment than the sketch implied.** The register-typing
substrate exists; G5 adds (1) the two domain newtypes, (2) typed data-register output, and (3) the
call-site checking story — the last is the only real design question.

### The seam (confirmed live at master, entity_window ↔ section)

```
pub proc Section_FlatIDXY   (d2: u8, d3: u8, a2: *Act) clobbers(d1)    out(d0)      // section.emp:109
pub proc Section_GetSecPtrXY(d2: u8, d3: u8, a2: *Act) clobbers(d1-d2) out(d0, a0)  // section.emp:131
```
Callers (all `entity_window.emp`): FlatIDXY ×3 (`:744`, `:832`, `:1630`) + GetSecPtrXY ×1 (`:741`).
Construction site example (`:824-832`): `Camera_X → +½screen → asr #SECTION_SIZE_SHIFT → d2 (= sec_x)`,
same for `d3 (= sec_y)`, then `jbsr Section_FlatIDXY → d0 (= flat id)`. The GridCoord "birth" is the
`asr`; the SectionId "birth" is FlatIDXY's `out`. **d2/d3 want `GridCoord`, out d0 wants `SectionId`.**

### Row-1069 demand census (RUN — the demand-quantifying step, done)

Aeon `.emp` corpus at master: **122 procs · 91 `// In:` · 85 `// Out:` register-doc comments · 43 typed-param
procs · 79 still-`()` procs · 4 `out(carry:…)`.** ~70-75% of procs document their register convention in a
COMMENT — the "comment-as-compensation" class (the language failing to say it in code). The FlatIDXY seam
is the **pilot**; those 85 `// Out:` / 79 `()` procs are the **staged retrofit surface** (opportunistic,
rides port modernization — NOT a G5 obligation).

---

## 1 · Byte-neutrality — HARD constraint AND the point

G5 is **byte-neutral by construction**: newtypes erase to their raw widths (`GridCoord`→u8, `SectionId`→u16),
and all checks are comptime. It lands on the settled `406c773b` canonical and **proves pass-3 changed
nothing typeable underneath it** (the roadmap's "type the final layout, not a moving target" rationale).

**Verification plan (acceptance):** rebuild both shapes BOTH invocations (`./build.sh`, `DEBUG=1 ./build.sh`
— the one-shape-per-invocation trap); CRCs UNCHANGED (`406c773b`/421122 · `5752c2e3`/429107); full paired
strict green (currently 2457/0/1); zero new clippy. Any byte delta = a bug in the erasure, stop.

---

## 2 · Section A — MECHANICAL (the gate rules; no Volence-taste needed)

**A1 · Type homes** — `engine/system/types.emp` (trivial; Coord/Angle precedent):
```
pub newtype GridCoord = u8    // a section grid index (sec_x / sec_y)
pub newtype SectionId = u16   // flat section id = sec_y * grid_w + sec_x
```

**A2 · Typed data-register output — the one grammar addition.** Extend `out()` from `out(dN)` /
`out(carry: Name)` to also accept `out(dN: Type)`. The flag form `out(carry: full)` is the precedent —
this is the data-register analogue. Checks mirror the existing `out()` tiers (D2.35): `[proc.out-*]`
overlaps/unwritten still apply; the type is metadata on the already-declared output. **Byte-neutral.**

**A3 · Apply at the seam** — the pilot retrofit:
```
pub proc Section_FlatIDXY   (d2: GridCoord, d3: GridCoord, a2: *Act) clobbers(d1)    out(d0: SectionId)
pub proc Section_GetSecPtrXY(d2: GridCoord, d3: GridCoord, a2: *Act) clobbers(d1-d2) out(d0: SectionId, a0)
```
plus the `.asm` twins' `; In:`/`; Out:` comments updated to name the types. Body field access
(`Act.grid_w(a2)`) is unaffected — `a2: *Act` already types the pointer.

**A4 · THE CALL-SITE CHECK — the mechanical crux (needs a ruling, see also B4).** Callers use bare
`jbsr` with the register pre-set, so a check must know d2/d3's TYPE at the jbsr. Three mechanisms:

- **(i) Signature-obligation dataflow** — the callee's `d2: GridCoord` param declares "the caller must
  hold a GridCoord in d2 at the jbsr"; a register-type dataflow tracks each register's domain type from
  its construction to the call and flags a mismatch. *Strongest check (catches a real sec_x/sec_y swap),
  heaviest build (a register-type dataflow pass — but it composes with the S2-D6 clobbers/preserves
  dataflow the tier already has).*
- **(ii) Body-level `let rN: Type` at construction** — `let d2: GridCoord = <asr result>` at the birth
  site; the type flows to the jbsr locally. *Debuts the specced-but-unused `let rN: Type`; the check is
  local (construction-site), lighter than (i), but only as good as adoption at every construction site.*
- **(iii) Documentary-only** — typed param/out name intent, no cross-call enforcement. *Cheapest;
  thinnest — "naming without checking," which adoption-over-cleverness warns against.*

**Recommendation:** pilot with **(ii)** at the 4 seam call sites (debut `let rN: Type`, bounded, real
construction-site checking) and BANK **(i)** as the general unblock once a second consumer appears —
mirrors how `preserves`/`out` shipped as syntactic slices before the full S2-D6 dataflow. **(iii)
rejected** (thin). *This is the primary gate question; it also has a taste dimension — B4.*

**A5 · Error surface** — a coercion mismatch is `[reg.type-mismatch]` (names expected domain type vs
supplied); raw→domain construction is an explicit coercion (see B3) so an *un-annotated* raw value
reaching a typed param/`let` is the loud site, never a silent accept.

**A6 · Corpus retrofit cost** — G5 proper = **2 procs + 4 call sites + 2 newtype defs + 1 grammar
addition** (bounded pilot). The 85 `// Out:` / 79 `()` procs are OUT of G5 (staged; each rides its
file's next port/modernization touch, byte-neutral).

---

## 3 · Section B — VOLENCE-TASTE (domain-type design questions — options + trade-offs for you)

*Volence drives the prelude domain-type pass (roadmap item 13); these are the calls that shape how the
.emp domain types FEEL. Each is framed as options.*

**B1 · Newtype family scope.** Does G5 define ONLY the two seam types, or seed the fuller prelude family?
- *(a) Minimal:* just `GridCoord`/`SectionId`. Smallest, proves the mechanism, defers taste.
- *(b) Seed the family:* add the [[emp-sonic-newtype-candidates]] set now (SubPixel/Speed fixed pair,
  VramTile+conversion, Tile/Block/Chunk, palette/collision/sound ids) as type homes, adopt at the seam
  only. Front-loads the naming/taste work; risks dangling types with no adopter (the campaign usually
  avoids). *Lean: (a) for G5; (b) is the item-13 prelude pass you drive next, on G5's proven mechanism.*

**B2 · Naming.** `GridCoord` vs `SecCoord` vs `GridIndex`? `SectionId` vs `FlatSecId` vs `SectionIndex`?
The `// In:` vocabulary is `sec_x`/`sec_y`/`flat id`. (My default: `GridCoord`/`SectionId` per the sketch.)

**B3 · Conversion ergonomics — how a raw value BECOMES a domain type.** At `asr … → sec_x`:
- *(a) Explicit `as`:* `let d2: GridCoord = raw as GridCoord` — loud, Rust-like, greppable coercion points.
- *(b) Constructor:* `GridCoord(raw)` — call-like, matches `Reg`/`Label` value idioms.
- *(c) Inferred at the typed boundary:* raw flows into `d2: GridCoord` and coerces silently at the seam.
*Trade-off: (a)/(b) make every raw→domain crossing visible (the safety G5 is FOR); (c) is ergonomic but
re-introduces silent crossings. Lean (a) or (b) — what should the crossing FEEL like?*

**B4 · Where does the check live — signature boundary or body?** (couples to A4.) Do you want G5 to
debut **body-level `let rN: Type`** (construction-site typing, option A4-ii), or keep register types at
the **signature boundary only** and defer body typing? This is both a build-cost call (mechanical) and a
taste call (does `.emp` gain per-statement register types, or stay signature-typed?).

**B5 · Refinement.** `GridCoord` is grid-dimension-bounded (`0..grid_w`), a RUNTIME bound (needs the
act-descriptor grid). Options: *(a)* plain `newtype GridCoord = u8` (naming only, bound unchecked — the
existing `Section_GetSecPtrXY` unsigned range check already guards out-of-grid at runtime); *(b)* carry a
`where`/refinement that emits a runtime check (heavier, overlaps GetSecPtrXY's existing guard). *Lean (a)
— the runtime guard exists; a refinement would duplicate it.*

---

## 4 · Carried riders (context, per the gate's ask)

- **Live slide mask-migration observation** (Phase-2.5 non-blocking rider) — observe one real
  `EntityWindow_MigrateMasks` window slide with correct section ids next runtime session. Unrelated to
  G5's mechanism; tracked so it isn't lost. (G5's SectionId types the very value that migration reads.)
- **s4lint W026 (width-discipline dataflow) pairing** — the s4lint-absorption list #2 pairs W026 with
  "G5 width typing." **Assessment: they are ORTHOGONAL axes.** W026 checks `.b/.w/.l` WIDTH discipline;
  G5 types DOMAIN semantics (a `GridCoord` is still byte-width). Coupling them balloons G5 and mixes a
  dataflow-lint build into a byte-neutral newtype pilot. **Recommend: W026 stays backlog (its own
  dataflow pass, alongside the S2-D6 register dataflow the A4-i unblock would share); G5 does not ride
  it.** Gate to confirm.

---

## 5 · Gate questions (what I need ruled before implementing)

1. **A2/A4 (mechanical):** approve `out(dN: Type)` grammar + the **A4-ii pilot** (`let rN: Type` checking
   at the 4 seam sites) with A4-i banked? Or a different call-site mechanism?
2. **B1 scope:** G5 = minimal (GridCoord/SectionId only), item-13 prelude pass seeds the family? (my lean)
3. **B3 conversion ergonomics:** explicit `as` / constructor / inferred? (Volence taste)
4. **B4:** debut body-level `let rN: Type`, or signature-boundary only? (Volence taste + build cost)
5. **B5 refinement:** plain newtype (lean) vs runtime-checked `where 0..`?
6. **§4 rider:** confirm W026 stays backlog (G5 orthogonal, does not ride).

**Byte-neutral, bounded (2 procs + 4 sites + 2 newtypes + 1 grammar add). No code until the gate rules
1–6 — and 3/4/5 wait on Volence's taste calls, separated here from the mechanical 1/2/6.**

---

## GATE + VOLENCE RULINGS (2026-07-23) — SPEC RATIFIED, IMPLEMENTATION AUTHORIZED

All six questions ruled. Volence's taste answers were taken directly at the gate (four
option-sets, all resolved); mechanical rulings are the overseer's. This section is the
authoritative delta over the sections above where they differ.

**Q1 (mechanical) — RULED, with one shape change.** `out(dN: Type)` grammar ships (flag-out
`out(carry: …)` is the precedent; this is the first DATA-register out-typing, flipping the
construct-walk #3 ledgered ruling as pre-planned there). The CALL-SITE CHECK SHIPS NOW — not
banked: it reuses the existing reaching-definition infrastructure (D1b/out-verify) as a
**strict-degrade slice**, which IS the "slice before full dataflow" shape Q1 wanted:
- Per-register type state: `Untyped | T`. A plain register copy (`move`/`movea` rX→rY)
  propagates `T`. ANY other write — arithmetic, logic, shifts, memory loads — degrades to
  `Untyped` (re-bless with `as` where the result is semantically still a `T`). Control-flow
  join: both edges same `T` → `T`; disagreement → `Untyped`.
- BANKED for a future consumer (A4-i): arithmetic preservation, declared coercions (§7's
  coercion clause ships NO syntax in G5). A4-ii `let rN: Type` is ALSO banked — see Q4.
- New diagnostic: `[call.slot-type-mismatch]`, ERROR tier from day one (Volence ruling below);
  message names call site, slot, expected newtype, found state, and producing site.

**Q2 (mechanical) — minimal scope RATIFIED, with the newtype set corrected to THREE.**
Volence ruled AXIS-SPLIT coordinates: `pub newtype GridX = u8` / `pub newtype GridY = u8` /
`pub newtype SectionId = u16` (in `engine/system/types.emp`, beside Coord/Velocity). Two axis
types close the x/y argument-swap class at compile time — the same silent-wrong-answer family
as the MigrateMasks stride bug. The wider family (MusicId/SfxId, Tile/Block/Chunk, VramTile,
PaletteLine, Coord/Velocity out-typing retrofits) is the item-13 prelude domain-type pass
(Volence-driven), which now has its enforcement surface.

**Q6 (mechanical) — RATIFIED.** W026 width-discipline stays §D backlog (domain-typing ≠
width-discipline; a GridX is still byte-width).

**Q3 (Volence) — `as`-cast on the producing instruction.** `move.w d3, d2 as GridX` — reuses
the existing `as` precedent (`jsr (a1) as ObjRoutine`). Constructor-call form rejected
(operands are addressing modes, not expressions); inferred coercion rejected (raw→domain
crossings must be visible). The NORMAL path is born-typed via out slots; `as` is the boundary
escape hatch.

**Q4 (Volence + cost) — signature-boundary + `as` ONLY in G5.** Body-level `let rN: Type`
(A4-ii) is BANKED to item-13: one blessing syntax at debut; revisit if `as` ceremony proves
awkward in the domain pass. Ledger the banked row.

**Q5 (Volence) — plain newtype.** No runtime `where 0..grid_w` refinement (GetSecPtrXY's
existing unsigned range guard stands; Volence chose plain nominal newtypes over refinement
style generally — refinements remain the separate vram_art-style param-refinement track).

**Strictness (Volence) — ERROR from day one.** Untyped/base value into a typed slot = ERROR;
mismatched newtype = ERROR; untyped slots check NOTHING (no ceremony tax, §7 verbatim). No
warn window — the seam's producers retrofit in the same commit (standing retro rule).

**Same-commit retrofit set:** the three newtypes in types.emp; `Section_FlatIDXY` →
`(d2: GridX, d3: GridY, a2: *Act) clobbers(d1) out(d0: SectionId)`; `Section_GetSecPtrXY` →
typed d2/d3 params + `out(d0: SectionId, a0)`; the 4 entity_window.emp call sites' producers
`as`-blessed (or upstream out-typed where a producer genuinely returns a grid coord —
implementer's judgment, smallest honest set); SectionId consumers (ess_section_id stores,
compares) get NO ceremony — struct fields are not slots (typed fields = frontier).
The game-side `.asm` caller (ojz_scroll_test) is outside the net — s4lint mirror stays §D.

**Tests + acceptance floor:** unit — GridX-into-GridY-slot fires (the swap pin); untyped-into-
typed fires; `as` accepted; out-born accepted; copy propagates; arithmetic degrades; join
disagreement degrades; untyped slots free. Corpus — retrofitted seam green under strict, PLUS
an injected NEGATIVE test (swap d2/d3 at one call site → build fails naming that site; the
class-closure pin, `struct_field_disp_plus_n.rs` precedent). Byte-neutral HARD bar —
dual-invocation builds (`./build.sh` AND `DEBUG=1 ./build.sh`) reproduce plain
`406c773b`/421122 · debug `5752c2e3`/429107 EXACTLY; full paired strict green; repin zero
drift. Overseer gate at close: own builds + seam-diff review + the swap-pin test demonstrated.
