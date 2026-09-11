# Two sigil-side findings routed by aeon's lens-pins parcel, 2026-09-11

**Provenance, stated first because it bounds everything below.** Measurements by the aeon lane's
agent at aeon `cd075f2d`, bisected, **mechanism NOT established**, and NOT re-run by this lane. The
evidence lives on an aeon branch that was unpushed when routed; their note is
`docs/superpowers/notes/2026-09-11-lens-pins-parcel.md` in aeon, and aeon will send the landed SHA.
The tables below are transcribed from aeon's message, not from an artifact this lane could read.
Banked here because a routed finding gets an artifact in the receiving repo when it is accepted.

## 1. `extern()` in a RESIDENT Z80 module is vacuous (queue row EMP-EXTERN-UNKNOWN-GREEN)

Probe builds, `FAST=1 ./build.sh sonic4`, each restored:

| probe | change in `z80_sound_driver.emp` | result |
|---|---|---|
| T1 | `use engine.sound_fm.{YM_ADDR_TO_DATA_MIN_T}` | exit 1, "unknown name" at all 8 consumers (emit_sound_blob fails): a const does not cross a `use` between resident modules |
| T2 | `const X = extern("YM_ADDR_TO_DATA_MIN_T")` | exit 0, ROM identical |
| T3a | `const X = extern("NO_SUCH_SYMBOL_LENS_PIN")` | **exit 0: a name that exists nowhere builds green** |
| T3b | T2 with the authority raised to 100 | only sound_fm's own 2 guards fired; the driver's 8 passed |

Aeon's reading (theirs, not verified here): seam-1 builds extern PROC stubs from `pub proc` only
(`seam1.rs` `import_stub_table` / `use_import_stubs`). Aeon's exposure: the only `ensure(extern(...))`
on their Z80 side is `z80_init.emp:94`, already their LS-16a.

**Split for this lane:** the soundness half (an `extern()` naming a symbol that exists nowhere must
refuse, never evaluate green) changes no spelling and is this lane's to fix. Making a cross-module
constant expressible (a const-carrying `use`, or the name on seam-1's injected list) is `.emp`
surface, so it is proposed to the owner and discussed before it lands (`d-6`).

## 2. A zero-byte const move changed another module's placement (queue row LINK-ZERO-BYTE-MOVE-PLACEMENT)

Moving a module-local const plus its `ensure` to the END of `engine/system/boot_data.emp` (after the
`dc.b` that uses it) turned sonic4 DEBUG red with an overlap between two UNTOUCHED sections:
`ojz_scroll_test [0xBE2D4, 0xC02F2)` and `replay_fixture [0xC02EC, 0xC054C)`. One-file bisect from
that tree: reverting `boot_data.emp` alone gives exit 0; reverting `dma_queue.emp`, `section.emp` or
`plane_buffer.emp` alone keeps the overlap; the control reproduces it. Aeon's tree no longer has the
shape (the const moved to `constants.emp`), so it is a report, not a blocker.

**A hypothesis, this lane's, labelled as one:** placement should not depend on where a zero-byte item
sits. The first candidate to test is the frozen-table seeding of provisional bases
(`native::load_frozen_table`, `native.rs:232`, see `2026-09-11-aeon-source-digest-ask.md` second
amendment). Nothing measured supports it yet; a reproduction at aeon `cd075f2d` with the move applied
comes before any mechanism.
