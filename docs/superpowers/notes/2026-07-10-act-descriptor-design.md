# A nicer act_descriptor — design note (Volence's bedtime ask, 2026-07-10)

**The ask:** "think about a nicer way to do act_descriptor, because we'll have a
lot of those." Written against the real `games/sonic4/data/levels/ojz/act1/
act_descriptor.asm` (254 lines: generated includes, three hand `if/error`
asserts, the Act struct as raw `dc` rows with an `Act_len` size guard, then
NINE near-identical `Sec` blocks of ~17 rows each).

## What hurts today (and scales linearly with act count)

1. **The descriptor is field-order-fragile.** Raw `dc.l/dc.w/dc.b` rows must
   match the `Act` struct field-for-field; the `* == Act_len` guard catches
   SIZE drift only — swap two same-width fields and every act mis-parses
   silently.
2. **The section table is 9× copy-paste.** Each `Sec` block differs from its
   neighbor only in the `OJZ_SecN_` label stem and the occasional real
   override (a parallax config, a palette). The interesting information —
   "section 3 clones section 0's art with a sky tint" — lives in a comment;
   the 17 rows are noise.
3. **Engine limits are re-asserted per act.** The pool ceiling / grid /
   axis-extent `if/error` blocks will be pasted into every act file and will
   drift.
4. **Naming is hand-threaded.** `OJZ_Act1_*`, `OJZ_SecN_*` stems repeat
   through every row; renaming an act touches dozens of lines.

## The .emp shape (everything below is SHIPPED language — no new constructs)

### Tier 1 — the descriptor becomes a typed struct literal

```
data OJZ_Act1_Descriptor: Act = Act{
    sec_grid_ptr:        OJZ_Act1_Sections,
    grid_w:              3,
    grid_h:              3,
    start_local_x:       $0100,
    start_local_y:       $0100,
    start_sec_x:         0,
    start_sec_y:         0,
    act_bg_layout:       OJZ_Act1_BG_Layout,
    act_bg_tiles:        OJZ_Act1_BG_Tiles,
    act_parallax_config: ParallaxConfig_OJZ_Default,
    act_art_pool_table:  OJZ_Act_Pool_PageTable,      // generated include, AS side
    act_art_pool_pages:  extern_is_fine_here,          // see Tier 3
    edge_mode:           EDGE_CLAMP,
    reserved:            0,
}
```

Every field is NAMED (field-order drift impossible), the type IS the
`Act_len` guard (the emit size check), pointer fields lower to fixups
automatically, and a struct gaining a field makes every act literal error —
the per-act checklist D2.31 designed. **This tier needs the `Act` struct as
an .emp twin** (the vars-era struct story; the struct-equ export already
carries `Act_len`/field offsets the other way).

### Tier 2 — the checks move into the type and a shared constructor

The three hand asserts are facts about RELATIONS between fields, so they
belong in ONE place all acts flow through — a comptime constructor in a
shared game module:

```
pub comptime fn act(grid_w, grid_h, start_sec, start_local, bg, pool, edge) -> Act {
    ensure(grid_w * grid_h <= extern("MAX_ACT_SECTIONS"),
           "act grid {grid_w}x{grid_h} exceeds MAX_ACT_SECTIONS")
    ensure((grid_w << extern("SECTION_SIZE_SHIFT")) <= $8000,
           "axis extent breaks the signed-word camera clamp")
    ensure((grid_h << extern("SECTION_SIZE_SHIFT")) <= $8000,
           "axis extent breaks the signed-word camera clamp")
    Act{ grid_w: grid_w, grid_h: grid_h, /* … */ }
}

data OJZ_Act1_Descriptor: Act = act(grid_w: 3, grid_h: 3, …)
```

Act files stop carrying engine invariants; new acts CANNOT forget them. The
pool-ceiling assert stays near the pool include (it guards generated data,
not the descriptor).

### Tier 3 — the section grid becomes a mapped expression

The 150-line Sec table is a naming convention plus overrides. Comptime
string labels (the D-PP.3 string-label form: a ptr field accepts
`"OJZ_Sec3_Blocks"` as a deferred link symbol) + `map` collapse it:

```
comptime fn ojz_sec(n) -> Sec {
    let stem = "OJZ_Sec" ++ str(n)
    Sec{
        sec_block_index: stem ++ "_Blocks",
        sec_objects:     stem ++ "_Objects",
        sec_rings:       stem ++ "_Rings",
        sec_pal:         OJZ_Palette,
        sec_type_table:  stem ++ "_TypeTable",
        sec_block_dict:  /* LinkExpr: */ stem ++ "_Blocks" + BLOCK_INDEX_SIZE,
        sec_block_dict_len: extern("OJZ_SEC" ++ str(n) ++ "_BLOCK_DICT_LEN"),
        sec_plc: 0, sec_parallax_config: 0, sec_bg_layout: 0, /* defaults */
        …
    }
}

data OJZ_Act1_Sections: [Sec; 9] = range(0, 9) |> map(ojz_sec)
ensure(OJZ_Act1_Sections.len == 3 * 3, "grid table size == grid_w*grid_h")
```

The interesting facts (which sections override parallax, the row-clone
story) become the ONLY hand-written lines — a per-section override match or
a small post-map patch list. `[Sec; 9]` re-derives the whole-table size
check. NOTE the two mechanics this leans on, both shipped: `Cell::Expr`
(S2-D13f — the `Blocks+BLOCK_INDEX_SIZE` residual tree in a ptr cell) and
struct field DEFAULTS (D2.31 — the seven always-zero fields declare
defaults in the Sec twin and each literal writes `field: default`, or the
constructor supplies them). What is NOT yet proven: `extern()` with a
COMPUTED name string (the dict-len read) — today extern takes a literal;
that is a small, recorded increment if Tier 3 is adopted.

### Tier 4 — the direction: acts come from `import()`

The editor already exports JSON the build's python generators consume. The
§6.7 `import()` maps structured data onto comptime values CHECKED against a
declared type. The end state for "a lot of acts":

```
const ACT = import("act1.toml")           // or the editor's JSON, directly
data OJZ_Act1_Descriptor: Act = act(grid_w: ACT.grid.w, grid_h: ACT.grid.h, …)
data OJZ_Act1_Sections: [Sec; 9] = ACT.sections |> map(sec_from_import)
```

One source of truth (the editor file), typed end-to-end, and the
`ojz_act_pool.asm` generator question resolves the same way: the .emp
imports/embeds the generated PAGES (`embed("act_pool_page0.zx0")`) and
builds the page table itself — the generator stops emitting .asm at all.
This is the hermetic-build "reproducibility own session" ledger item worn
as a feature.

## Recommendation

- **Port act_descriptor (tranche 4 #3) at Tier 1+2**: typed `Act`/`Sec`
  literals + the shared `act()` constructor. Byte-gateable today, kills
  pain points 1 and 3, and the constructor module is the seed of the game
  prelude (construct walk #1's neighborhood).
- **Tier 3 rides the same port if the computed-name `extern()` increment is
  cheap** (it likely is — same resolution path, name built before the
  call); otherwise jot it and keep nine explicit-but-typed `sec` literals.
- **Tier 4 is the post-campaign direction**, decided together with the
  ojz_act_pool generator question at the checkpoint — they are the same
  decision ("editor data enters through import/embed, not generated .asm").
