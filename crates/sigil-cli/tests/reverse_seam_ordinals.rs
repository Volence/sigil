//! The ANIM-ordinal reverse-seam flip — the SCOPED proof (D2.34,
//! Volence-ratified 2026-07-10).
//!
//! End-state (post-Spec-5): `sonic_anims.emp`'s offsets table IS the id
//! definition — the ordinals export as the `ANIM_*` equs, the hand-written
//! config block is deleted, and AS player code (`move.b #ANIM_WALK,
//! SST_anim(a0)`) reads the exports. During the dual-build era the config
//! block STAYS (the gate-off shape can't read .emp exports), so kill-list
//! row 4 carries a two-stage kill: [1] this proof (mechanics real, done),
//! [2] the config deletion at Spec 5.
//!
//! Mechanics: NOTHING NEW — `equ NAME = <ordinal>` (R-T0.3) already lowers
//! to a link-level `EquSym`, and ordinals are plain comptime ints. This
//! test proves the whole flip verbatim: an .emp module defines the table
//! and exports its ordinals; a synthetic AS consumer reads them through
//! the shared link — the .emp side DEFINES, the AS side CONSUMES.

use sigil_frontend_as::{assemble, Options as AsOptions};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{SectionPlacement, SymbolTable};

#[test]
fn emp_offsets_ordinals_export_as_equs_the_as_side_reads() {
    // The .emp side: a miniature Ani table whose ordinals export under the
    // ANIM_* names — the flip's exact shape.
    let emp = "module m\n\
               offsets Ani {\n\
                   Walk: [u8; 2] = [1, $FF],\n\
                   Run:  [u8; 2] = [2, $FF],\n\
                   Roll: [u8; 2] = [3, $FF],\n\
               }\n\
               equ ANIM_WALK = Ani.Walk\n\
               equ ANIM_RUN = Ani.Run\n\
               equ ANIM_ROLL = Ani.Roll\n\
               equ ANIM_COUNT = Ani.count\n";
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pdiags:?}");
    let (module, ldiags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    assert!(ldiags.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ldiags:?}");

    // The AS side, consuming the exports through the deferral shapes that
    // exist today: `move.l #Sym` (R3 imm32) and `dc.b Sym` (the db/dw
    // deferral). NOTE for the FULL flip (kill-list row 4 stage 2): the real
    // player code writes `move.b #ANIM_WALK, SST_anim(a0)` — the imm
    // deferral needs `.b`/`.w` widths before the config block can be
    // deleted (recorded in the ledger).
    let asm = "cpu 68000\n\
               Consumer:\n\
               \tmove.l  #ANIM_ROLL, d0\n\
               \tdc.b    ANIM_WALK, ANIM_RUN, ANIM_COUNT\n";
    let opts = AsOptions { initial_cpu: Cpu::M68000, ..AsOptions::default() };
    let as_module =
        assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble (consumer): {d:?}"));

    let mut sections = module.sections;
    // Pin the .emp table + carrier at a harness-private base; the AS
    // consumer at another.
    for sec in &mut sections {
        sec.lma += 0x0100_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    let mut consumer = as_module.sections;
    for sec in &mut consumer {
        sec.lma = 0x0300_0000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(consumer);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link: {d:?}"));

    let consumer = linked
        .sections
        .iter()
        .find(|s| s.lma == 0x0300_0000)
        .expect("linked image must carry the AS consumer");
    // move.l #imm, d0 = 203C + imm32 — ANIM_ROLL must have resolved to
    // ordinal 2 through the seam.
    assert_eq!(
        &consumer.bytes[0..6],
        &[0x20, 0x3C, 0x00, 0x00, 0x00, 0x02],
        "move.l #ANIM_ROLL, d0 must bake ordinal 2 from the .emp export"
    );
    // dc.b ANIM_WALK, ANIM_RUN, ANIM_COUNT = 00 01 03.
    assert_eq!(
        &consumer.bytes[6..9],
        &[0x00, 0x01, 0x03],
        "the dc.b row must read ordinals 0/1 and count 3 from the .emp exports"
    );
}
