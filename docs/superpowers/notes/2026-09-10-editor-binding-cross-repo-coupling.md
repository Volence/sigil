# The editor-binding symbols are a cross-repo coupling nobody had booked

**Routed by aeon via the hub, 2026-09-10. Verified here firsthand before booking; the
population below is WIDER than the one row that was routed.**

## What the coupling is

`EditorSceneBinding_OJZ_Act1_Sec{i}` is **minted by aeon's `tools/effects_gen.py`**, from
`binding_sec(i)` -> `..._Sec{i}`. Sigil depends on that spelling in four places, none of
which says so:

| site | what depends on it |
|---|---|
| `crates/sigil-harness/src/section_align.rs` | one `d("EditorSceneBinding_OJZ_Act1_Sec0", 2, WORD)` row, module `ojz_effects_editor_act1` |
| `crates/sigil-cli/tests/act_descriptor_port.rs` | `Sec0` as the **END label of the pinned `SCENE_REGISTRY` region** - its expected address is `SCENE_REGISTRY.base + len` in each shape |
| same file | `Sec4` asserted against `pins::EDITOR_SCENE_BINDING_OJZ_ACT1_SEC4` |
| `crates/sigil-harness/repin.toml` | `Sec4` as a `[[symbol]]` pin |

**The routed row named only the first.** The second is the tighter one: a rename does not
merely drop an alignment row, it moves the arithmetic a pinned region's end label supplies.

## The mechanism, which is aeon's finding and corrects two earlier framings

It depends on **NEITHER section ordering NOR section identity**. It depends on the
**SPELLING of the module's lowest-offset label**, which ordering picks and naming supplies.
Aurora had filed it as a claim about ORDER and the hub ratified that reading; both were
reasoning about a file neither had opened. That is why the row survives regions step 4 (whose
file list excludes `effects_gen.py` and its output, and whose one removed consumer is a
`comptime fn`, which cannot move a head label) and breaks instead at the tools parcel, §3.3.

## The vacuous-gate twin: ASKED AND ANSWERED NO

Aeon booked `test_effects_gen.py:1140` in their tree as green-because-the-bed-is-the-runner -
it writes the section-0 sidecar in all three of its tests, so it cannot see the binding become
unbound, while its name reads `..._still_the_section_0_binding`. **Sigil has no twin.**
`native::emit_generated` writes **only sound artifacts** into `engine/sound/generated`
(blob, dac, mt, sfx, seq opcodes, sound tables, pitchtable - seven emitters, none of them
editor bindings). The binding is supplied by the aeon tree, so sigil asserts a symbol it does
not create, which is the non-vacuous shape. Re-check this if `emit_generated` ever grows an
editor emitter.

## Why it is SIGIL-DECOUPLE and not a fix

This is precisely the class that project exists to retire: a hand-maintained sigil table keyed
on a symbol aeon's codegen names, invisible from either side alone. It is booked, not fixed.
Fixing it means deriving these rows rather than transcribing them, which is decouple work.
