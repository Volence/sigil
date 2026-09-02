# EMBED-BASE-SKEW — parcel brief for the engine lane

**Trigger: hand this over once the engine lane's item 3 lands** (hub, 2026-09-02; watch their
lane-log). Written ahead so the handover is a message rather than an investigation.

**It is the engine lane's parcel, not this one's.** It moves engine bytes, and since the paired
freeze ended that lane certifies alone — so it re-derives its own goldens. Sigil's half is one
deletion.

## What is wrong

The aeon tree carries **two `embed(...)` path conventions at once**, and sigil hardcodes an
exception to paper over it. `crates/sigil-harness/src/native.rs:1930-1932`:

```rust
let embed_base_for = move |id: &str| -> Option<std::path::PathBuf> {
    if id == "engine.math" { Some(math_dir.clone()) } else { Some(aeon_root.clone()) }
};
```

`engine.math` is given `engine/system` as its base; everything else is given the repo root. That
exists because `engine/system/math.emp` writes module-relative paths — line 37
`embed("../data/sine.bin")` and line 164 `embed("../data/arctan.bin")` — while every other
`embed` in the tree is repo-root-relative.

## The fix

One module's spelling changes and the exception dies with it:

- **aeon:** `engine/system/math.emp:37,164` → `embed("engine/data/sine.bin")` /
  `embed("engine/data/arctan.bin")`. Both files are already at `engine/data/`, verified.
- **sigil:** delete the `if id == "engine.math"` branch, leaving `Some(aeon_root.clone())` for
  every module.

## CORRECTION: THE LOCKSTEP WAS AN ARTEFACT OF MY OWN TWO-STATE DESIGN

**The section below is wrong and is kept so the reasoning is visible.** It concluded "neither
side can go first", which held only because I had posited exactly two states: the exception, or
its absence. The hub's three-step shape dissolves it by adding a **tolerant middle state** —
resolve root-relative first, fall back to module-relative, announce the fallback — and every step
is then independently valid. Step 1 landed 2026-09-02 and is byte-neutral. *A lockstep that comes
from the design rather than from the problem disappears when the design does; check for a middle
state before declaring two halves inseparable.*

**The three steps as ruled:**
1. **sigil, DONE:** uniform transition rule for every module, no named exception. `pins.rs
   unchanged` and the whole-ROM golden gates pass, so no byte moved — measured, not assumed. The
   fallback fires on exactly the two known sites and says so: `embed.module-relative 2`, at
   `math.emp:37` and `:164`, each with file and line.
2. **aeon, whenever it fits:** re-spell those two to `engine/data/...`. Works under either sigil,
   so it is an ordinary landing with aeon's own goldens. **This is the handover.**
3. **sigil, after step 2 is on aeon's origin/master:** delete the fallback. A build at that tip
   with it removed proves no module-relative embed remains — and the warning count is the
   progress meter until then.

## (SUPERSEDED) THE HALVES ARE NOT INDEPENDENTLY VALID — land them together

This is the part that would bite silently, and it is sharper now than it was a day ago because
landings are no longer paired:

| state | `engine.math`'s base | `"../data/sine.bin"` resolves to | |
|---|---|---|---|
| today | `engine/system` | `engine/data/sine.bin` | works |
| aeon re-spells first, exception still present | `engine/system` | `engine/system/engine/data/sine.bin` | **fails** |
| sigil deletes first, old spelling still present | repo root | `<root>/../data/sine.bin` | **fails, and escapes the tree** |

So neither side can go first. **Whoever lands it must carry both halves in one change**, which
means the engine lane takes sigil's deletion into its own landing or sequences the two within one
window. That is a coordination cost the unpaired arrangement no longer provides for free, and it
is worth naming rather than discovering: the ordinary assumption after the cut is that either
lane can land its own half whenever it likes, and here that assumption is false.

## Why bother at all

Two conventions in one tree is a trap for whoever writes the next `embed`: they will copy
whichever neighbour they happen to open, and only one of the two spellings works from any given
module. The exception also means sigil's resolver behaves differently for one named module, which
is the kind of special case that quietly acquires a second entry.

**Byte accounting is the engine lane's**, and the row's claim that it moves bytes should be
measured rather than assumed — re-spelling a path that resolves to the same file need not change
output, and if the four shapes come back byte-identical that is the cheapest possible outcome
and the parcel is nearly free.
