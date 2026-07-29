# Convention-drift audit — the .emp corpus vs the step-2 house format (2026-07-29)

READ-ONLY audit. Dispatched by the overseer (Fable) at Volence's request, motivated by
the bare-abs-EA catch: the house form for absolute RAM EAs lived only as PRECEDENT in
engine files, never as TEXT in the step-2 checklist, so game-side files drifted for three
tranches undetected (byte gates are blind to byte-identical spelling variants). This audit
asks the general question — what ELSE is precedent-only, and where has practice drifted?

Scope: all 68 main-tree `*.emp` under `aeon/engine` + `aeon/games` (the `.worktrees/`
copies of the three in-flight branches were excluded). Nothing was modified. The three
in-flight branches (`fix-test-parent-lifecycle`, `fix-combined-link-locals`,
`style-bare-abs-ea`) were left untouched; findings that those branches already cover are
called out and set aside.

Checklist reference: `sigil/docs/superpowers/notes/campaign-port-loop.md` (step 2 +
register-contract convention). "TEXT" below = already codified there; "PRECEDENT-ONLY" =
the corpus is consistent but nothing in the checklist says so.

---

## Headline counts

- **Dimensions audited: 14**
- **Precedent-only conventions worth codifying: 3** (contract CPU-split ordering/separator;
  the bare-abs-EA imperative as a first-class idiom line; the mem-to-mem width-pin comment string)
- **Drift instances found: 26 concrete** (sonic ×2, sound_debug ×5, uncommented mem-to-mem
  pins ×8, contract range-form ×5, stale-fixed-bug comment ×1, change-history narration ×5),
  **plus one large gated sweep** (68k contract comma→slash normalization, ~40 procs) that
  should not run until its rule is ratified.

---

## (a) Checklist additions — precedent-only conventions worth codifying

### A1. Contract clause ORDER and register-group SEPARATOR are a clean CPU split — but precedent-only

The only contract text in the checklist (step-2 item 5) codifies the movem-*range* form
(`clobbers(d0-d7/a0-a4)` "not comma enumeration") and shows a `/`-separated example, but it
never states the CPU split, so 68k files drift to comma-separated groups freely.

Observed, consistent within each CPU:

- **68k** — `clobbers` first, then `out`, then `preserves`, `falls_into` last; register
  groups separated by `/`. Canonical exemplars: `dma_queue.emp:80`
  `clobbers(d0-d4/a1-a2) out(carry: dropped)`, `section.emp:142`
  `clobbers(d1-d2) out(d0, a0, zero: none)`, `load_art.emp:89`
  `clobbers(d0-d5/d7/a0-a3) preserves(d6/a4-a6)`, `tile_cache.emp` throughout.
- **Z80** — `out` FIRST, then `clobbers`, then `preserves`; named registers separated by
  `,` (af/bc/de/hl are pairs, not rangeable). Universal in `sound_fm.emp` /
  `sound_psg.emp`: `sound_fm.emp:155` `out(b, c) clobbers(af) preserves(de, hl)`,
  `sound_psg.emp:170` `out(hl, carry: found) clobbers(a, b, de, f) preserves(c)`.

No 68k proc uses out-first; no z80 proc uses clobbers-first. This is a real convention that
should be TEXT so the next z80/68k port doesn't guess — and, crucially, so the widespread
68k comma-group spelling (finding B5) is recognized as drift rather than accepted.

**Draft checklist text (append to step-2 item 5, contract-reglist bullet):**

> Contract clause ORDER and register-group SEPARATOR are CPU-split. **68k:**
> `clobbers` → `out` → `preserves`, `falls_into` last; register groups joined by `/`
> (`clobbers(d0-d4/a1-a2)`), never comma. **Z80:** `out` → `clobbers` → `preserves`;
> named registers joined by `,` (`out(a) clobbers(f) preserves(bc, de, hl)`) — af/bc/de/hl
> are pairs, not ranges. A 68k contract with comma-separated groups, or a z80/68k contract
> with the other CPU's clause order, is a step-2 miss.

### A2. The bare-abs-EA rule needs a first-class imperative line (THE motivating catch)

The rule exists in item 5, but it is buried mid-paragraph and framed defensively (as a
*boundary* against the `(sym+const).w` operand-override and the `[lower.imm-link]` kept
widths). There is no crisp positive imperative, which is exactly why porters, panels, and
overseer reads all walked past the game-side drift. Engine files obey it as precedent
(e.g. `camera.emp`, `parallax.emp`, `dma_queue.emp` all write bare `Camera_X` /
`Parallax_State` in code with the width spelled only in a trailing `// (Sym).w` twin-note).

**Draft checklist text (promote to a standalone bullet at the head of the item-5 idiom walk):**

> **Absolute RAM/ROM EA = BARE symbol, auto-width.** Write `move.w Camera_X, d0` /
> `lea DPLC_Sonic, a2` — the width rule picks abs.w/abs.l from the resolved value. An
> explicit `(Sym).w` / `(Sym).l` in CODE is a MISS unless it is one of the named pinned
> exceptions, each with its site comment: (1) mem-to-mem two-symbolic operands (A3 below),
> (2) `[lower.imm-link]` — a link-immediate `#extern(...)` source combined with a relaxable
> symbolic dest (`core.emp` / `boot.emp` / `dma_queue.emp:279`), (3) a movem abs seam
> (`boot.emp:210`), (4) the trailing-`(d16,An)` form with no bare lowering yet
> (`parallax.emp:342`). The width may legitimately be spelled in a trailing `// (Sym).w`
> comment (the twin-note); that comment is NOT a code pin.

### A3. The mem-to-mem two-symbolic width-pin has a canonical comment STRING — make it text

Item 5 names the mem-to-mem exception ("keep the t15 mem-to-mem pinned `.w` spellings") and
item 2 requires every structural pin to carry a site comment, but the *canonical comment
string* is precedent-only. `vblank.emp:97/123/125` spell it
`// widths pinned: mem-to-mem two-symbolic operands`; `section.emp:426-427` and
`tile_cache.emp:1157-1159,1280-1282` keep the same correct `.w` pins with NO comment at all
(finding B3). Codifying the exact string closes the gap.

**Draft checklist text (append to step-2 item 2):**

> A mem-to-mem `move.w (X).w, (Y).w` (two symbolic operands) legitimately keeps both widths;
> its required site comment is the fixed string `// widths pinned: mem-to-mem two-symbolic
> operands` (`vblank.emp:97` is the precedent). A kept `.w` mem-to-mem pin without it is an
> uncommented-pin miss (item-2 class).

---

## (b) Drift instances to fix — one byte-neutral parcel

All are byte-neutral (spelling / contract / comment only). Grouped for a single cleanup
parcel; the large B5 sweep is gated on A1 ratifying first.

**B1. Bare-abs-EA drift MISSED by the in-flight cleanup — `sonic.emp` (2 sites).**
`sonic.emp:59` `lea (DPLC_Sonic).l, a2` and `sonic.emp:60` `lea (Art_Sonic).l, a3` are the
exact class `style-bare-abs-ea` is cleaning in `player_common.emp` (cf. the in-flight
`player_common.emp:287` `lea (PhysTable_Sonic).l`), but `sonic.emp` is NOT in that branch's
named set (game_debug 14 / test_churn 1 / player_common 20). These are ROM art/DPLC pointers
(>$8000 → abs.l); bare resolves to the identical .l, so byte-neutral. → make bare.

**B2. Same class, engine side — `sound_debug.emp` (5 sites), VERIFY FIRST.**
`sound_debug.emp:69` `(SND_MIRROR_DEST).w`, `:70` `(SND_REQ_SRC).l`, `:78` `(SND_STATE_SRC).l`,
`:93` `(SND_SEQ_SRC).l`, `:109` `(SND_TRACE_SRC).l` — explicit widths on abs EAs with no pin
comment. The `.w`/`.l` mix tracks the target region (68k RAM mirror vs the $A0xxxx Z80-bus
window), so IF those symbols carry their full resolved addresses, bare picks the same width
and this is drift → make bare. IF a symbol's value forces a non-default width (e.g. a bare
$1F00 that must be long via the bus mapping), it is a genuine pin → KEEP and add a site
comment instead. Resolve by checking the SND_* definitions before touching.

**B3. Uncommented mem-to-mem width pins (8 sites) — correct pins, missing the comment.**
`section.emp:426-427` (2) and `tile_cache.emp:1157-1159,1280-1282` (6) are `move.w (X).w,
(Y).w` two-symbolic mem-to-mem — the `.w` is CORRECT to keep, but they lack the
`// widths pinned: mem-to-mem two-symbolic operands` comment that `vblank.emp` carries
(step-2 item 2 miss). → add the comment (see A3).

**B4. Contract range-form violations (5 sites) — drift against the CODIFIED item-5 rule.**
Contiguous runs spelled enumerated instead of as a range:
- `vdp_init.emp:26` `clobbers(d0, a0, a1)` → `clobbers(d0/a0-a1)`
- `load_object.emp:106` `clobbers(d0-d3, a0, a1, a2, a3)` → `clobbers(d0-d3/a0-a3)`
- `controllers.emp:13` `clobbers(d0, d1, a0)` → `clobbers(d0-d1/a0)`
- `sound_api.emp:140` `clobbers(d0/d1)` → `clobbers(d0-d1)`
- `sound_api.emp:352` `clobbers(d0, d1, a0)` → `clobbers(d0-d1/a0)`

**B5. 68k contract comma-group → slash normalization (~40 procs) — GATED on A1.**
Pervasive 68k comma-separated groups: all of `entity_window.emp` (`:137` `clobbers(d1, a0)`,
`:212` `clobbers(d0-d1, a0)`, `:540` `clobbers(d0, d2, a0)`, ~30 procs), `player_common.emp`
(`:308` `clobbers(d0-d7, a1-a4)` etc.), `load_object.emp:38` `clobbers(d0-d1/d3, a2, a3)`
(mixed `/` and `,` in one clause), `core.emp` (`:205` `clobbers(d0-d1, a1)`),
`animate.emp:51` type decl `clobbers(d0-d2, a1-a2)`, `collision.emp` Touch_* params, plus
comma `out(...)`/`preserves(...)` (`math.emp:21` `out(d0, d1)`, `s4lz:88` `out(a0, a1)`).
Canonical is `/` (dma_queue/parallax/tile_cache/sprites/section). This is the biggest single
stylistic drift, but it is pure churn until A1 ratifies the `/`-on-68k rule — DO NOT sweep it
speculatively; ratify A1, then run it as its own byte-neutral pass (or ledger it).

**B6. Stale fixed-bug comment (1 site).** `z80_init.emp:38`
`ld de, 1 + .code_end    // (== code_end+1) leading-`.`local bug: see gap-ledger` — the t28
P3 parser fix landed (ledger row 1610 closed), and item 5 itself instructs "drop the 'until
the fix lands' site comments at next touch." The commuted spelling stays (house preference),
but the "bug" framing is stale → rewrite to a plain preference note or drop.

**B7. Change-history narration in comments (5 sites) — exhibit-comment rule, at-next-touch.**
Same class as the ~40-site codename audit already ledgered. Comments that narrate what the
code USED to be rather than state its present contract:
- `aabb.emp:38` "Formerly this was an `if`-branch whose `asm{}` yielded..."
- `collision.emp:142` "The old mid-frame clear in player_common sat..."
- `animate.emp:233` "the old movem.l a1/d1 save/restore pair is dead..."
- `sound_api.emp:276` "...less bus contention than the old direct post."
- `game_debug.emp:97` "the old game_loop.emp extern decl TRUSTED that narrow leaf contract..."

Borderline (each also carries a live "why"); rewrite to present-tense fact or delete, same
bucket/priority as the existing codename backlog — not urgent.

---

## (c) Non-findings — dimensions that are clean and/or already TEXT (the negative result)

- **Header doc-comment (CLEAN).** 68/68 files open with a `// path — description` block.
  Zero exceptions. Not TEXT, but universal; no drift, no action.
- **`module X in Y` vs `module X` (CLEAN, self-consistent).** `in Y` appears exactly on
  section-emitting modules; library/type/const/macro modules (`types`, `structs`,
  `constants`, `vdp`, `irq`, `coords`, `aabb`, `objdef`, `sst`, `z80_bus`) correctly omit it.
  `(cpu: z80)` / `(cpu: m68000, bank:/vma:)` section attributes are consistent. No drift.
- **use-clause style (CLEAN).** Uniform `use module.{A, B}` braced lists, one module per
  line, grouped at file top. No one-per-line-symbol drift, no unbraced singletons.
- **extern() drift-guard pairing (CLEAN).** Mirrored const blocks pair each `const` with an
  `ensure(extern(...)==..., "...drifted...")` — `game_debug.emp:36-71`, `sfx_bank.emp`,
  the mt_bank/dac mirrors. Consistent mirror-comment style.
- **Branch spelling (CLEAN, already TEXT).** `jbra`/`jbsr` universal; bare Bcc universal;
  `jmp`/`jsr` only on computed targets (`core.emp:474` `jsr (a1) as ObjRoutine`,
  `vblank.emp:40`, `player_common.emp:357`) and documented structural jump tables
  (`collision.emp:198-210`, `dma_queue.emp:203/252`, `animate.emp:140-153`), each with its
  bra.w/stride site comment. The single `.s` pin (`animate.emp:65`) is commented.
- **`as Type` bless placement (CLEAN).** 45 sites, all trailing the producing instruction
  (`moveq #SONG_MOVINGTRUCKS, d0 as SongId`, `jsr (a0) as VBlankHandler`). No drift.
- **falls_into placement (CLEAN).** Always the last clause on the header. Consistent.
- **Immediate #hex vs #decimal (NON-FINDING — no enforced convention).** Mixed by intent:
  hex for bit patterns/addresses, decimal for counts and small compares (`andi.w #63` as a
  64-row mask, `cmpi.w #15`). No codified rule; the split is semantic, not drift.
- **commuted `1 + .local` spelling (effectively retired).** Only `z80_init.emp:38` remains,
  now optional post-t28; captured as B6 (stale comment), not a spelling drift.
- **Proc param typing (checked, no survey-level straggler).** Domain params carry newtypes
  where moved/compared (`GetSineCosine(d0: Angle)`, `Section_FlatIDXY(d3: GridY)`,
  `Sound_PlayMusic(d0: SongId)`, `CreateChild_Linked(d0: ObjRoutine)`); the remaining bare
  params are the t33 deliberate-rest-stay-bare class. No obviously-domain param found left
  bare at signature level (a per-proc value-flow audit is out of this audit's scope).

---

## Note for the overseer — the game_debug tension

`game_debug.emp`'s 14 explicit-`(X).w` / `bsr.w` / `.s` sites are DOCUMENTED as a structural
width pin (lines 105-113: the trailing `align 2` after `Dbg_SfxIdTable` needs every preceding
instruction size fixed, so bare Bcc / `jbsr` / bare abs would leave the align provisional →
`[align.provisional]`). That reads as divergence-WITH-REASON, i.e. NOT drift — yet the
motivating catch counts game_debug's 14 among the sites `style-bare-abs-ea` is cleaning.
These cannot both be right. Either the align dependency is load-bearing (keep the 14, they
are legitimate pins and the cleanup should exclude them) or it is not (the pin comment is
wrong and should go with the widths). I did not rule; flagging it because the in-flight parcel
and this audit disagree on the same 14 sites, and the resolution changes whether A2's
exception list should name the trailing-`align` class as a fifth pinned exception.
