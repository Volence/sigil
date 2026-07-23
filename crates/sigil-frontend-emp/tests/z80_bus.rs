//! `[bus.*]` — the Z80-bus machine-state contract lint (item-4 core, §D backlog):
//! the sigil-native absorption of s4lint's E006/E007/E008/E011 crash-class checks.
//!
//! A three-point MUST lattice `{Stopped, Running, Unknown}` tracked forward over
//! `flag_check::Cfg`; each check fires ONLY on a DEFINITE violation (the rider's
//! zero-FP polarity — a joined `Unknown` never fires). Each test pins one rule:
//! - a second `stop_z80` while provably Stopped fires `[bus.double-stop]` (E011);
//! - a `start_z80` while provably Running fires `[bus.start-without-stop]` (E008);
//! - a return reached provably Stopped fires `[bus.stopped-at-return]` (E007);
//! - a VDP-port write while provably Running fires `[bus.vdp-write-unstopped]`
//!   (E006) — inert on the real corpus (indirect `(a4)` VDP writes are punted);
//! - a balanced stop/write/start does NOT fire (the happy path);
//! - a VDP write under a stop does NOT fire (the stop is respected);
//! - a write whose bus-state DISAGREES across a join (`Unknown`) does NOT fire
//!   (the zero-FP meet — the load-bearing soundness rule);
//! - the `btst #0, Z80_BUS_REQUEST` READ inside the stop spin is NOT a toggle.

use sigil_frontend_emp::corpus_contracts::{analyze_corpus, ContractReport};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::z80_bus::BusFiringKind;

fn analyze(src: &str) -> ContractReport {
    let (f, diags) = parse_str(src);
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    analyze_corpus(&[f])
}

fn count(r: &ContractReport, proc: &str, kind: BusFiringKind) -> usize {
    r.bus_firings.iter().filter(|f| f.proc == proc && f.kind == kind).count()
}

/// Two `stop_z80` writes with no intervening release: the second stops a bus that
/// is provably already held → `[bus.double-stop]` (E011). The FIRST stop, reached
/// with `Unknown` entry state, does NOT fire (not a provable double).
#[test]
fn double_stop_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::DoubleStop), 1, "{:?}", r.bus_firings);
}

/// A `start_z80` reached while the bus is provably Running (a prior release with
/// no re-stop) → `[bus.start-without-stop]` (E008). The stop makes the first start
/// legitimate; the second start is the unpaired one.
#[test]
fn start_without_stop_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::StartWithoutStop), 1, "{:?}", r.bus_firings);
}

/// A `rts` reached while the bus is provably Stopped (stopped and never
/// restarted) → `[bus.stopped-at-return]` (E007). The Z80 is left dead.
#[test]
fn stopped_at_return_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::StoppedAtReturn), 1, "{:?}", r.bus_firings);
}

/// A VDP-port write reached while the bus is provably Running → the crash class
/// `[bus.vdp-write-unstopped]` (E006). The write follows an explicit release.
#[test]
fn vdp_write_while_running_fires() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers(d0) {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             move.w  d0, VDP_CTRL\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::VdpWriteUnstopped), 1, "{:?}", r.bus_firings);
}

/// The happy path: `stop / write-Z80-RAM / start / rts`. Every check is satisfied
/// — no firing of any kind.
#[test]
fn balanced_pair_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P (d0: u16, a0: *u8) clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.b  d0, (a0)\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(r.bus_firings.iter().filter(|f| f.proc == "P").count(), 0, "{:?}", r.bus_firings);
}

/// A VDP write UNDER the bus hold: provably Stopped at the write → does NOT fire.
/// Proves the stop is respected (the check is not blindly firing on all writes).
#[test]
fn vdp_write_while_stopped_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P (d0: u16) clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
             move.w  d0, VDP_CTRL\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::VdpWriteUnstopped), 0, "{:?}", r.bus_firings);
}

/// Two paths reach a VDP write with DISAGREEING bus state (one stops, one does
/// not) → the meet is `Unknown` → the write does NOT fire. The zero-FP soundness
/// rule: a violation must hold on EVERY reaching path to be definite.
#[test]
fn disagreeing_join_does_not_fire() {
    let r = analyze(
        "module m\n\
         pub proc P (d0: u16) clobbers() {\n\
             tst.b   d0\n\
             beq     .skip\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
         .skip:\n\
             move.w  d0, VDP_CTRL\n\
             rts\n\
         }\n",
    );
    assert_eq!(count(&r, "P", BusFiringKind::VdpWriteUnstopped), 0, "{:?}", r.bus_firings);
}

/// The `btst #0, Z80_BUS_REQUEST` inside the real stop-spin READS the bus-grant
/// bit — it is NOT a bus toggle and must not perturb the tracked state. A full
/// `stop-spin / start / rts` must stay clean.
#[test]
fn btst_read_is_not_a_toggle() {
    let r = analyze(
        "module m\n\
         pub proc P () clobbers() {\n\
             move.w  #$0100, Z80_BUS_REQUEST\n\
         .wait:\n\
             btst    #0, Z80_BUS_REQUEST\n\
             bne     .wait\n\
             move.w  #$0000, Z80_BUS_REQUEST\n\
             rts\n\
         }\n",
    );
    assert_eq!(r.bus_firings.iter().filter(|f| f.proc == "P").count(), 0, "{:?}", r.bus_firings);
}
