# Design — Spec 2 quality-of-life batch (C1) · post-freeze amendment set

Written 2026-07-11 (Fable), from the ergonomics audit
(`notes/2026-07-11-emp-ergonomics-audit.md` bucket C1) + the gap-ledger rows it consolidates.
Volence delegated the spec ("just spec out your C1"). **DESIGN ONLY.** Six small items, one
implementation tranche; every item is **byte-neutral** (all existing gates stay green
unchanged — that IS the acceptance bar, plus the per-item retrofit demonstrations below).
These are the taxes every FUTURE port pays; the batch compounds across the rest of the
campaign. Per A-Spec2.3 each lands as a recorded decision; all surface additions follow the
S2-D1 contextual/headroom policy.

## Item 1 — Label values in immediate expressions (+ `use`-imported labels as values)

**Problem** (ledger, tranche 6 ×2 — "every future object port hits this once per routine
store"): a bare label name in an instruction immediate is `unknown name`; the D-PP.3
bareword→deferred-link-symbol fallback covers only data initializers and call arguments. So
the objroutine store pays self-extern ceremony:

```emp
equ SOLID_ROUTINE_MAIN = extern("TestSolid_Main") - extern("ObjCodeBase")   // today
move.w  #SOLID_ROUTINE_MAIN, Sst.code_addr(a0)
```

**Surface.** Adopt the call-argument rule in **immediate operand position** (`#expr`) — the
third and last of the deferral positions:

```emp
move.w  #TestSolid_Main - ObjCodeBase, Sst.code_addr(a0)     // after
move.l  #HBlank_Null, (HBlank_Handler_Ptr).w                 // the port-#1 shape, .emp-side
```

- An unresolved bareword in a `#` immediate becomes a deferred link symbol (`Expr::Sym`);
  `label ± const` folds the addend (the tranche-9 pc-rel addend precedent); `label − label`
  lowers as a link-time `Sub` expr (the tranche-6 `dc.w` deferral precedent).
- **Width routing is the shipped imm-link machinery unchanged**: `.l` → `Value32Be`
  (tranche 5), `.w` → `ImmWord16Be` (tranche 10 — the union window is exactly what
  objroutine offsets in `[0x8000,0xFFFF]` need), `.b` stays consumer-gated with its existing
  steering diagnostic (kill-row-4 stage 2's blocker, unchanged by this item).
- **Totality guard**: this opens immediates ONLY. Barewords in `const`/`ensure`/comptime
  expressions keep hard-erroring (the sfx_bank-header rule stands — a typo must not silently
  become a symbol in comptime logic). In immediate position the typo trade is the same one
  `jsr Label` already accepts: resolution failure is loud at link, naming the symbol.
- **`use`-imported labels** (the tranche-6 sibling ask): `use games.sonic4.particle_anims.
  {Ani_Particle}` brings a `data`/`offsets`/`table`/`proc` label into the module's known-name
  set — usable in immediate position, module-resolved (typo = comptime error, not link), and
  self-documenting. The bareword deferral remains the cross-seam (.asm-side) path.
- **Demonstrator**: a prelude-class `comptime fn objroutine(l: Label)` returning the
  `l − ObjCodeBase` link expr becomes writable — the per-port idiom collapses to
  `move.w #objroutine(TestSolid_Main), Sst.code_addr(a0)`.

**Machinery**: eval-context flag on operand-immediate expressions reusing the D-PP.3 fallback;
lowering routes through `lower_m68k_imm_link` (shipped, incl. the one-pinned-abs-operand
composition from t10). No new fixup kinds. **Retrofit proof**: test_solid.emp /
test_particle.emp drop their self-extern `equ`s — byte-identical (same fixups, new spelling).

## Item 2 — `clobbers()` accepts movem reglists

**Problem** (ledger, tranche 6 + 7 — three data points): `preserves(d0-d1/a0)` takes the
movem-reglist grammar; `clobbers(d0-d4/a1-a3)` is a parse error. TouchResponse spelled TWELVE
registers comma-by-comma.

**Surface**: `clobbers(d0-d7/a0-a3)` — one grammar for both attributes (and `out(...)`,
D2.35, gets it too for uniformity). Comma-separated singles stay legal; `sr` composes as
today. **Machinery**: share `preserves`' reglist parser; the attribute's register-set
representation is already a set. **Retrofit proof**: TouchResponse's contract respelled —
byte-neutral (attributes emit nothing).

## Item 3 — `bankid`/`winptr` bareword arguments

**Problem** (audit, data files): `bankid("Sfx_33")` spells the label as a STRING — renames
don't refactor, typos surface only at link, and the file's own data labels are referenced in
two different notations three lines apart.

**Surface**: `bankid(Sfx_33)` / `winptr(Sfx_33)` — a bareword call argument already becomes a
deferred link symbol everywhere else (the very rule the sfx_bank header documents); verify
these two builtins accept it and unify. The string form STAYS (it is the computed-name path —
the act_descriptor Tier-3 `extern("OJZ_Sec{N}_...")` increment will want it). Bareword
preferred in new-style files; a note-tier lint is future polish, not part of this batch.
**Retrofit proof**: one respell in mt_bank/sfx_bank's `ensure`s — byte-neutral.

## Item 4 — equ hygiene (module-local equs mangle like non-pub procs)

**Problem** (ledger, tranche 6): equ names are link-global — two modules declaring
`equ OBJ_CODE_BASE = …` collide at link, so files carry hand prefixes (`SOLID_`/`PARTICLE_`).

**Surface**: none — non-`pub` equs get owner-mangled link names exactly like non-export
labels (`$module$NAME`); `pub equ` keeps the plain name (it IS the cross-seam contract
surface). The collision diagnostic (for two `pub equ`s) says "equ" and names both modules.
**Compat check**: no current `.emp` module reads another module's non-pub equ by bare name
cross-module (they can't — that's the collision being fixed); the harness's `extern()` reads
target `pub equ`s and `.asm` symbols, untouched. **Retrofit**: the SOLID_/PARTICLE_ prefixes
CAN drop, but that renames link symbols consumed by port-test pins — do it (or don't) as a
deliberate follow-up, not silently inside this batch.

## Item 5 — Unexported-label hint diagnostic

**Problem** (ledger, tranche 9): `bra.w AnimateSprite.cc_delete` fails at link with
"unresolved symbol" when the label EXISTS but lacks `export` — the fix is undiscoverable from
the message. **Fix**: when an `Owner.label` reference misses AND `Owner` has a non-exported
`.label`, the diagnostic says so and suggests the `export .label:` marker. Pure
message-quality; the mangled-name machinery already knows both facts.

## Item 6 — `clobbers()`/`preserves()`/`out()` entry validation

**Problem** (ledger, preserves(sr) slice): `clobbers(d9)` or a typo'd name is silently
accepted — it just never matches the lint's lookup, so the contract rots invisibly.
**Fix**: validate every entry against the register vocabulary (+ `sr`) at the same site
`preserves` validates, error on unknowns. Composes with item 2 (validate after reglist
expansion).

## Sequencing & acceptance

One tranche, items independent (1 is the largest; 2/5/6 are hours-class; 3 may be
verify-only; 4 needs the compat check). Gate: full strict suite unchanged (byte-neutral bar)
+ the three retrofit proofs (items 1/2/3) + negative probes per item (typo'd immediate fails
loud at link naming the symbol; comptime bareword still hard-errors; unknown clobber entry
errors; two pub-equ collision names both modules). Not in this batch (stays ledgered):
`.b` imm-link (consumer-gated), computed-name extern (act_descriptor Tier 3), the force-width
byte-lock idiom, `jbcc`.
