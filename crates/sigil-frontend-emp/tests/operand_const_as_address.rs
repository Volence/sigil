//! `[operand.const-as-address]` (D1 / S2-D12(e)) — the forgotten-`#`.
//!
//! A bare path in operand position is always a SYMBOL, so naming a plain-valued
//! `const`/`equ` there reads MEMORY at the constant's value. An `equ` becomes a
//! real link symbol equal to its value, so the mistake is silently well-formed:
//! `move.w RINGS_MAX, d0` loads a word from address 999. Warn-tier,
//! `@allow`-suppressible, inert under `@as_compat` (ported AS code names plain
//! constants in address position deliberately), and exempt for constants that
//! ARE addresses — a `*T` annotation, a link-time value, or a hardware/RAM
//! address at or above `$A00000`.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::{Diagnostic, Level};

const LINT: &str = "[operand.const-as-address]";

fn lower(src: &str) -> Vec<Diagnostic> {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "unexpected parse diagnostics: {perrs:?}");
    let (_module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        },
    );
    diags
}

fn firings(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.message.contains(LINT)).collect()
}

/// The classic pair, in one file so the ONLY difference is the `#`.
#[test]
fn bare_const_operand_fires_and_the_immediate_form_does_not() {
    let bug = lower(
        "\
module m
const RINGS_MAX = 999
proc p () {
    move.w RINGS_MAX, d0
    rts
}
",
    );
    let f = firings(&bug);
    assert_eq!(f.len(), 1, "the bare form is the bug: {bug:?}");
    assert_eq!(f[0].level, Level::Warning, "warn tier, not an error");
    assert!(f[0].message.contains("`RINGS_MAX`"), "names the const: {}", f[0].message);
    assert!(f[0].message.contains("#RINGS_MAX"), "offers the fix: {}", f[0].message);

    let ok = lower(
        "\
module m
const RINGS_MAX = 999
proc p () {
    move.w #RINGS_MAX, d0
    rts
}
",
    );
    assert!(firings(&ok).is_empty(), "the immediate form is the correct code: {ok:?}");
}

/// An `equ` is the SILENT half of the bug (it becomes a link symbol equal to its
/// value, so the wrong code links and runs), and the message says `equ`.
#[test]
fn bare_equ_operand_fires_and_names_its_kind() {
    let diags = lower(
        "\
module m
equ SPAWN_LIMIT = 12
proc p () {
    move.w SPAWN_LIMIT, d0
    rts
}
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 1, "firings: {diags:?}");
    assert!(f[0].message.contains("an `equ`"), "kind named: {}", f[0].message);
}

/// A `*T`-annotated const IS the address type — the spec's own exemption.
#[test]
fn pointer_typed_const_is_silent() {
    let diags = lower(
        "\
module m
struct Sst { id: u16 }
const SLOT: *Sst = $10
proc p () {
    move.w SLOT, d0
    rts
}
",
    );
    assert!(firings(&diags).is_empty(), "a `*T` const is address-typed: {diags:?}");
}

/// An `equ` OF A LABEL is a link-time value, not a number — naming it in address
/// position is the whole point.
#[test]
fn equ_of_a_label_is_silent() {
    let diags = lower(
        "\
module m
data Table: [u8; 2] = [1, 2]
equ TABLE_BASE = Table
proc p () {
    move.w TABLE_BASE, d0
    rts
}
",
    );
    assert!(firings(&diags).is_empty(), "an equ-of-label is an address: {diags:?}");
}

/// The corpus idiom: an MMIO/Z80/RAM address spelled as a plain untyped const.
/// `$C00004` is the VDP control port — a deliberate absolute address, and the
/// single largest false-positive class the D1 corpus sweep found (67/67).
#[test]
fn hardware_address_const_is_silent() {
    let diags = lower(
        "\
module m
const VDP_CTRL = $C00004
const Z80_BUS_REQUEST = $A11100
proc p () {
    move.w VDP_CTRL, d0
    move.w Z80_BUS_REQUEST, d1
    rts
}
",
    );
    assert!(firings(&diags).is_empty(), "hardware addresses are addresses: {diags:?}");
}

/// Both ends of the range are exact, so the exemption cannot quietly widen:
/// `$9FFFFF` is still cartridge space and still the bug; `$A00000` is the first
/// hardware address; `$FFFFFF` is the last address the 24-bit bus can carry.
#[test]
fn the_hardware_range_is_exact_at_both_ends() {
    for (value, fires) in
        [("$9FFFFF", true), ("$A00000", false), ("$FFFFFF", false), ("$1000000", true)]
    {
        let src = format!(
            "\
module m
const K = {value}
proc p () {{
    move.l K, d0
    rts
}}
"
        );
        let diags = lower(&src);
        assert_eq!(
            firings(&diags).len(),
            usize::from(fires),
            "{value} should {} fire: {diags:?}",
            if fires { "" } else { "NOT" }
        );
    }
}

/// The 24-bit ceiling in its real form: a `vdp_comm()` command long is a
/// QUANTITY the 68000 cannot address, and `move.l VRAM_FILL_CMD, VDP_CTRL`
/// meaning `#VRAM_FILL_CMD` is the classic Mega Drive form of this exact bug. A
/// floor alone would exempt it (`$40000080 >= $A00000`); the range catches it,
/// while the genuine port address in the same instruction stays silent.
#[test]
fn a_32_bit_vdp_command_long_fires_but_the_port_beside_it_does_not() {
    let diags = lower(
        "\
module m
const VDP_CTRL = $C00004
const VRAM_FILL_CMD = $40000080
const CRAM_WRITE_CMD = $C0000000
const LOW_WORD_MASK = $FFFF0000
proc p () {
    move.l VRAM_FILL_CMD, VDP_CTRL
    move.l CRAM_WRITE_CMD, d0
    and.l LOW_WORD_MASK, d1
    rts
}
",
    );
    let f = firings(&diags);
    assert_eq!(f.len(), 3, "each 32-bit quantity fires once: {diags:?}");
    for name in ["VRAM_FILL_CMD", "CRAM_WRITE_CMD", "LOW_WORD_MASK"] {
        assert!(
            f.iter().any(|d| d.message.contains(name)),
            "`{name}` must fire: {:?}",
            f.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
    assert!(
        !f.iter().any(|d| d.message.contains("VDP_CTRL")),
        "the destination IS an address and must stay silent"
    );
}

/// The probe must not SWALLOW a real diagnostic. `resolve_const` memoizes
/// unconditionally (including a `Poison`), so a probe that truncated its
/// diagnostics and left the memo behind would silence the const's genuine
/// failure at every later use site in the same evaluator too.
///
/// Non-vacuity: this asserts the SPECIFIC "unknown name" message, not merely
/// that some error exists. Without the un-memoize the only survivor is the
/// generic "instruction dropped" floor, which names neither the const nor the
/// missing symbol — measured by reverting the fix.
#[test]
fn the_value_probe_does_not_swallow_a_real_const_error() {
    let diags = lower(
        "\
module m
const BAD = NOT_DECLARED_ANYWHERE + 1
proc p () {
    move.w BAD, d0
    move.w #BAD, d1
    rts
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.level == Level::Error
                && d.message.contains("NOT_DECLARED_ANYWHERE")),
        "the broken const must still name the missing symbol: {diags:?}"
    );
}

/// `@allow("operand.const-as-address")` suppresses it at module scope, and the
/// UNQUOTED spelling does not (it parses as arithmetic — `[attr.allow-form]`).
#[test]
fn allow_attr_suppresses_only_in_string_form() {
    let allowed = lower(
        "\
module m
@allow(\"operand.const-as-address\")
const RINGS_MAX = 999
proc p () {
    move.w RINGS_MAX, d0
    rts
}
",
    );
    assert!(firings(&allowed).is_empty(), "@allow must silence it: {allowed:?}");

    let bare = lower(
        "\
module m
@allow(operand.const-as-address)
const RINGS_MAX = 999
proc p () {
    move.w RINGS_MAX, d0
    rts
}
",
    );
    assert_eq!(firings(&bare).len(), 1, "the unquoted form registers no opt-out: {bare:?}");
}

/// `@as_compat` makes the lint inert: ported AS code writes this deliberately in
/// known idioms, and this first slice is new-style-only.
#[test]
fn as_compat_module_is_inert() {
    let diags = lower(
        "\
module m
@as_compat
const RINGS_MAX = 999
proc p () {
    move.w RINGS_MAX, d0
    rts
}
",
    );
    assert!(firings(&diags).is_empty(), "@as_compat is new-style-only's escape: {diags:?}");
}

/// A bare LABEL operand (a data item, a proc) is the normal absolute-address
/// idiom and must stay silent — the lint keys on const/equ declarations only.
#[test]
fn bare_label_operand_is_silent() {
    let diags = lower(
        "\
module m
data Counter: [u8; 2] = [0, 0]
proc p () {
    move.w Counter, d0
    rts
}
",
    );
    assert!(firings(&diags).is_empty(), "a label operand is an address: {diags:?}");
}
