//! Sound-migration T2 — Task 2, P5 (link-level): TWO separately-lowered
//! `.emp` modules, EACH with top-level `equ` items (so each emits its OWN
//! zero-byte `text` carrier section — R-T0.3, confirmed in `equ_link.rs`'s
//! `plain_comptime_int_equ_folds_to_int`), placed via ONE map and linked
//! TOGETHER into a single image. This is the exact shape the mixed
//! DAC+MT harness needs (T1's `dac_samples.emp` + T2's `mt_bank.emp`, each
//! contributing its own equ carrier), mirrored here with synthetic modules
//! so the probe has no aeon dependency.
//!
//! Technique: `place_sections` (`crates/sigil-cli/tests/dac_port.rs`) placing
//! sections BY NAME against map regions, and the multi-module compose+link
//! pattern from `crates/sigil-cli/tests/ports.rs`
//! (`mixed_build_cross_seam_symbol_resolves`).
//!
//! R7's open question — "if the dual zero-byte `text` carrier pair trips a
//! duplicate-name/overlap diagnostic, fix it HARNESS-SIDE (map/naming/stagger),
//! NOT in the linker" — turned out to be MOOT: `place_sections` places
//! sections by NAME against a region, tracking a per-region CUMULATIVE cursor
//! (`crates/sigil-frontend-emp/src/resolve/mod.rs`'s `used` map), so two
//! independently-lowered modules whose default carrier is BOTH literally named
//! `text` place cleanly — the second `text` section is simply `Chained` right
//! after the first in the SAME region (both zero bytes, so they land at the
//! identical address, which is harmless: nothing ever reads a "carrier
//! address", only the `equ_syms` it carries). `two_modules_sharing_the_same_text_carrier_name_place_and_link_cleanly`
//! below pins this directly. No linker change, no harness rename was actually
//! required — recorded here as the negative result the plan asked to record
//! either way. `two_modules_with_harness_renamed_carriers_also_work` pins the
//! alternative (give each its own map region) as an equally-valid harness
//! option, in case a later task wants per-module carrier addressability.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Section, SymbolTable};

/// Lower a single `.emp` source (no `include_root` needed — these probe
/// modules use no `embed`), asserting a clean parse + lower.
fn emp_sections(src: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != sigil_span::Level::Error), "parse: {pdiags:?}");
    let (module, ldiags) =
        lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, defines: vec![] });
    assert!(ldiags.iter().all(|d| d.level != sigil_span::Level::Error), "lower: {ldiags:?}");
    module.sections
}

const MODULE_A: &str = "module a\n\
    section bank_a (vma: $8000) {\n\
      data Alpha: [u8;4] = [$11,$22,$33,$44]\n\
    }\n\
    equ AlphaBank = bankid(\"Alpha\")\n";

const MODULE_B: &str = "module b\n\
    section bank_b (vma: $28000) {\n\
      data Beta: [u8;4] = [$55,$66,$77,$88]\n\
    }\n\
    equ BetaBank = bankid(\"Beta\")\n";

/// Read a folded equ value from the resolved sections (mirrors dac_port.rs's
/// `equ_value`).
fn equ_value(sections: &[Section], name: &str) -> i64 {
    for sec in sections {
        for eq in &sec.equ_syms {
            if eq.name == name {
                match &eq.expr {
                    Expr::Int(v) => return *v,
                    other => panic!("equ `{name}` did not fold to Int post-resolve: {other:?}"),
                }
            }
        }
    }
    panic!("equ `{name}` not found in any resolved section");
}

/// The core probe, matching the REAL dac+mt shape byte-for-byte: both modules'
/// default carrier is named `text` (unmodified, un-renamed) and the map has
/// exactly ONE `text` region — the same map shape `dac_port.rs` already uses
/// for a single module. Link must succeed and BOTH equ sets resolve.
#[test]
fn two_modules_sharing_the_same_text_carrier_name_place_and_link_cleanly() {
    let mut sections = emp_sections(MODULE_A);
    sections.extend(emp_sections(MODULE_B));
    assert_eq!(
        sections.iter().filter(|s| s.name == "text").count(),
        2,
        "sanity: both modules' equ attaches to a section literally named `text`"
    );

    let map = sigil_link::load_map(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"bank_a\"\n\
         lma_base = 0x8000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"bank_b\"\n\
         lma_base = 0x28000\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections must not error on the dual zero-byte text carrier pair sharing one region: {pdiags:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (dual carrier, shared name) failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (dual carrier, shared name) failed: {d:?}"));

    // Both data sections carry their real bytes at their placed LMAs.
    let bank_a = linked.section("bank_a").expect("bank_a in linked image");
    assert_eq!(bank_a.bytes, vec![0x11, 0x22, 0x33, 0x44]);
    let bank_b = linked.section("bank_b").expect("bank_b in linked image");
    assert_eq!(bank_b.bytes, vec![0x55, 0x66, 0x77, 0x88]);

    // Both equ sets resolve to their OWN module's placed bank: bankid($8000)=1,
    // bankid($28000)=5 — distinct banks, so the fold proves per-module
    // resolution, not a coincidental shared value.
    assert_eq!(equ_value(&resolved, "AlphaBank"), 1, "bankid($8000) == 1");
    assert_eq!(equ_value(&resolved, "BetaBank"), 5, "bankid($28000) == 5");
}

/// The alternative harness option R7 anticipated (per-module carrier rename
/// before placement, giving each its own map region) ALSO works — recorded so
/// a later task that wants per-module carrier addressability has a pinned
/// example, even though the plain shared-name shape above needed no fix.
#[test]
fn two_modules_with_harness_renamed_carriers_also_work() {
    let mut sections_a = emp_sections(MODULE_A);
    let mut sections_b = emp_sections(MODULE_B);
    for sec in sections_a.iter_mut() {
        if sec.name == "text" {
            sec.name = "text_a".to_string();
        }
    }
    for sec in sections_b.iter_mut() {
        if sec.name == "text" {
            sec.name = "text_b".to_string();
        }
    }
    let mut sections = sections_a;
    sections.extend(sections_b);

    let map = sigil_link::load_map(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text_a\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"text_b\"\n\
         lma_base = 0x0010\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"bank_a\"\n\
         lma_base = 0x8000\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"bank_b\"\n\
         lma_base = 0x28000\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
    )
    .expect("map must load");
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections must not error on renamed per-module carriers: {pdiags:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout (renamed carriers) failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link (renamed carriers) failed: {d:?}"));

    let text_a = linked.section("text_a").expect("text_a carrier placed");
    assert_eq!(text_a.bytes.len(), 0, "an equ-only carrier emits zero bytes");
    let text_b = linked.section("text_b").expect("text_b carrier placed");
    assert_eq!(text_b.bytes.len(), 0, "an equ-only carrier emits zero bytes");
    assert_eq!(equ_value(&resolved, "AlphaBank"), 1);
    assert_eq!(equ_value(&resolved, "BetaBank"), 5);
}
