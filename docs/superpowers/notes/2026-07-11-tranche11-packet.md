# Tranche 11 packet — engine/objects/sprites.asm → sprites.emp

Branch: sigil `port-tranche11`, aeon worktree `sigil-emp-tranche11` (NOT
merged — awaiting Volence's gate). Design note:
`notes/2026-07-11-tranche11-sprites-design.md`.

**Status: loop DRY, at the merge gate.** Full workspace strict **2140/0**,
clippy clean, both full ROM builds green.

## What shipped

**Step 0** — design note (recon + hazards + the row-17 forced-flip analysis).

**Step 1 — transcribe (byte-exact both shapes).** sprites.emp, 6 items in
decl order (InitSpriteSystem, Draw_Sprite, Render_Sprites, `data
CellOffsets_XFlip`, Emit_ObjectPieces, InsertSpriteMasks). Gate
`SIGIL_EMP_SPRITES`, region `$2954/$2C0E` len `$420` both shapes.
`sprites_port.rs`: region byte gate both shapes + AS-twin lockstep oracle +
outbound `jsr InitSpriteSystem` relaxation + doctored `SCREEN_WIDTH` drift
probe.
- **DEMANDED FEATURE (step-1 law): `Sym ± const` absolute-address operands.**
  `btst #0, (Sprite_Cycle_Counter+1).w` (the odd byte of a word RAM cell)
  needs a bare symbol + constant byte offset. The bare-label idiom lowered
  fine but `Sym+1` comptime-folded the link symbol and failed "unknown name".
  Fix (`eval/asm.rs map_plain`): route `sym ± const` (either order for `+`,
  sym-left for `-`) to `CodeOperand::SymOff`, riding the existing
  `RelaxAbsSym` width-rule seam (same as `Item.field`). 3 byte-level tests in
  `field_operands.rs`.
- **FORCED row-17 flip (demanded structural change):** the gate removes
  sprites.asm's `MAX_VDP_SPRITES`/`VDP_SPRITE_{X,Y}_OFFSET` defs, but the
  gate-off rings.asm twin reads them in immediates (reverse-seam-in-immediate
  is the unshipped `.b`/`.w` imm-link deferral). Hoisted the three `=` defs
  sprites.asm → `engine/constants.asm`; row 17 → row-1 class.
- **Constants twin grew 34→49:** 15 render-flag/band/screen/frame constants
  (sprites is their first comptime consumer). Kill-list row 20.

**Step 2 — modernize (−8 both shapes).** sprites.emp: all Bcc bare, bra→jbra,
bsr.w→jbsr. sprites.asm twin: 4 branches asl width-selected `.w`→`.s`
(bra `.next_object`, beq `.multi_done`, child bsr `Emit_ObjectPieces`, beq
`.pieces_xflip`), site-commented; the rest keep original explicit widths (the
sigil AS front-end pins `.asm` width — bare `.asm` is a hard error). Region
`$420`→`$418`; engine.inc resume orgs re-pinned −8 for the 6 downstream gated
regions; `org $10000` shield keeps the object bank + game data fixed.
Downstream re-pins: pins.rs regenerated, repin_pins baseline, mixed_dac_rom
game_loop disp `$3A24`→`$3A1C` / `$4E54`→`$4E4C`.

## What each pass added

**Loop pass 1 (steps 3-5, then dry):**

- **Step 3 (retrospect):**
  - *Language/format ask (shipped in step 1):* `Sym ± const` operands — the
    one real language gap this file hit. Now general (any `Label+N` absolute).
  - *Reads-wrong:* none — the transcribe reads faithfully; the four
    Emit_ObjectPieces variants are repetitive but that is a construct
    question (step 4), not a reads-wrong defect.
  - *Kill-list:* row 17 flipped to row-1 class; row 20 added (the 15-const
    sprites block). No new mirrors of a *drift* kind (sprites.emp carries
    zero module-local mirrors; SPRITE_MASK_{SIZE,HEIGHT} are unguarded
    module consts, "NOT on the list" class).

- **Step 4 (construct pass):** the headline — **Emit_ObjectPieces's four
  inline flip-variant loops** (unflipped / xflip / yflip / xyflip, ~40 lines
  each). They share the SAT-write skeleton and differ only in: Y transform
  (simple vs neg+height), X transform (simple vs neg+width-LUT), the tile
  `eori` mask (`0/$0800/$1000/$1800` = `xflip<<11 | yflip<<12`), and whether
  the size byte is read from `d1` or re-read `-6(a3)` (yflip clobbers d1).
  - Verdict: **BUILD candidate, DEFERRED to a Volence decision (step-3(a)
    ask).** A `comptime fn emit_piece_loop(xflip: bool, yflip: bool) -> Code`
    with flip-conditional `asm{}` composition would collapse ~160 lines → ~50,
    **byte-neutral** (still fully unrolled — the zero-JSR-per-piece perf
    intent is preserved). NOT built this pass because: (1) it is
    **file-specific** (no other file emits VDP SAT piece loops — low
    corpus-reuse, unlike `clear_longs`/`rep`), so it is a dedup-within-one-file
    win, not a toolbox addition; (2) byte-exactness across four variants is
    delicate and deserves focused work, not a rushed in-port build. Design
    sketch above; recommend as a fast follow-up if Volence wants the
    collapse. The `CellOffsets_XFlip` LUT stays as-is (consumed by the x/xy
    variants).
  - Other verbs: no `adopt` (offsets/table/dispatch don't fit sprites'
    shapes); no `delete` (no dead code — every proc has a live caller:
    Draw_Sprite←core.emp, Init/Render←game states, Emit/InsertSpriteMasks
    ←Render_Sprites internal).

- **Step 5 (optimize):** **no changes, recorded why.** Render_Sprites +
  Emit_ObjectPieces are already hand-unrolled for zero-JSR-per-piece (the
  four inline variants ARE the optimization). The band walk (7→0), link-order
  cycling, scanline budget, and multi-sprite sibling walk are all already
  lean. No algorithmic improvement is identifiable without a live oracle
  profile; that profile is a deferred follow-up (the code is not on the
  hot-path frontier the way RunObjects' 66-slot loop was in tranche 10). The
  step-4 Emit unification, if taken, is byte-neutral → no perf delta.

**Step 6 (corpus sweep):** the one new thing prior files could use is the
`Sym ± const` operand feature. Sweep of all prior `.emp` files: **zero
retrofit sites** — no ported file has a `Label+N` memory operand or an
`extern()+N` EA workaround (sprites is the sole user so far). Ledgered, no
retrofit. (The −8 re-pin already propagated to every downstream test; the
constants-twin growth is additive, not a retrofit trigger.)

## Neither-bucket (step-1 demanded features / probe outcomes)

- `Sym ± const` operands (demanded feature) — shipped + tested, byte-verified.
- Row-17 forced hoist — byte-neutral, verified (rebuilt s4.bin IDENTICAL to
  master pre-modernization).
- The `.asm` twin cannot go bare (sigil AS front-end pins branch width) —
  confirmed by the AS-twin oracle; explicit `.s` on the 4 relaxed branches.

## Merge checklist (Volence's gate)

- New reference pins (post-modernization): the worktree listings drive them;
  `cargo run -p sigil-harness --bin repin` regenerates. Master s4.bin/debug
  will be rebuilt post-merge; provenance hashes updated then.
- `--no-ff` merge both sides + push; update spec2-progress provenance.
- Open decision for Volence: take the Emit_ObjectPieces `emit_piece_loop`
  BUILD now (fast follow-up) or leave the four variants inline.
</content>
