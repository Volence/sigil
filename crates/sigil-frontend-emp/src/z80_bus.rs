//! `[bus.*]` — the Z80-bus machine-state contract lint (item-4 core, §D backlog):
//! the sigil-native absorption of `s4lint.py`'s E006/E007/E008/E011 crash-class
//! checks (`aeon/tools/s4lint.py`). The 68000 can only touch Z80 RAM / the VDP
//! ports safely while it OWNS the Z80 bus (`stopZ80`); a mismatched stop/start or
//! a port write with the Z80 still running is a hardware-tier crash. s4lint caught
//! these with a flat scalar and no CFG joins; sigil sees the full CFG, so the net
//! gets true path-sensitivity.
//!
//! **Lattice (three points):** this is the INFERRED-tier instance of the shared
//! [`Tri`] lattice — `Held` = the 68k provably owns the bus, `NotHeld` = the bus
//! is provably not held, `Unknown` = caller-dependent entry or a join of
//! disagreeing states. The forward MUST dataflow, the meet, and the worklist are
//! [`crate::context::must_in_states`], shared with the DECLARED tier; this module
//! supplies only the transfer function and the firing rules.
//!
//! **Soundness stance — zero false positives (the item-4-rider polarity).** Every
//! check fires ONLY when the IN-state is a DEFINITE value proving the violation;
//! a joined `Unknown` NEVER fires. The DEFAULT entry seed is `Unknown`, not
//! `NotHeld`: a proc's caller might already hold the bus, and that is not locally
//! provable, so a lone unpaired toggle/write at the very top of a proc is
//! deliberately NOT flagged (no reaching definite state ⇒ no fire). This costs
//! the "unpaired at entry" cases s4lint's `Running` seed caught, and buys
//! guaranteed no-FP: every firing is a bus state the code itself made definite.
//!
//! **The DECLARED tier supplies the seed the inference cannot compute.** A proc
//! declaring `requires(<bus ctx>)` / `grants(<bus ctx>)` states that every call
//! site holds the bus — a claim `[context.unsatisfied]` CHECKS at every call
//! site. [`BusEntry::Held`] hands that fact to this net, so a proc that runs
//! under a declared hold is analyzed from a definite entry state and its unpaired
//! toggles fire like any other. Everything else keeps the `Unknown` seed
//! unchanged.
//!
//! **Recognition (the item-4b modeling ruling).** By the time the corpus lint
//! sees an evaluated `CodeBuf`, a context bracket's acquire/release are already
//! instruction bodies, so the net keys off the RESOLVED operand, not a name: a `move` whose destination is `Z80_BUS_REQUEST` ($A11100) is a bus toggle
//! (`#$0100` ⇒ Stopped, `#$0000` ⇒ Running); a `move`/`clr` whose destination is a
//! VDP port (`VDP_CTRL`/`VDP_DATA`, $C00000–$C00007) is a fenced access. The
//! `btst #0, Z80_BUS_REQUEST` READ inside the stop spin is not a `move`, so it is
//! correctly not a toggle.
//!
//! **The E006 caveat (mirrors s4lint's `(a4)` punt, `s4lint.py:1133`).** A VDP
//! write through a register-indirect destination (`move.w d1, (a4)` — the
//! `type_slice`-before-seam addressing the real corpus uses for DMA/VRAM setup) is
//! UNRESOLVABLE here: the address lives in a register, so the destination symbol is
//! unknown. Those are not flagged — the same soundness bailout as the type slice's
//! unverifiable paths. On the real aeon corpus E006 is therefore largely inert;
//! `sound_api` (the sole bus-toggle file) is where E007/E008/E011 have teeth.

use crate::context::{must_in_states, Tri};
use crate::flag_check::Cfg;
use crate::value::{CodeItem, CodeOperand};
use sigil_ir::backend::Cpu;
use sigil_span::Span;
use std::collections::BTreeMap;

/// Which Z80-bus contract violated (the sigil-native names for s4lint's
/// E006/E007/E008/E011).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusFiringKind {
    /// `[bus.double-stop]` (s4lint E011): a bus-REQUEST write reached with the
    /// bus provably already Stopped.
    DoubleStop,
    /// `[bus.start-without-stop]` (s4lint E008): a bus-RELEASE write reached with
    /// the bus provably Running (an unpaired / doubled release).
    StartWithoutStop,
    /// `[bus.stopped-at-return]` (s4lint E007): a return reached with the bus
    /// provably Stopped — the Z80 is left dead after the proc exits.
    StoppedAtReturn,
    /// `[bus.vdp-write-unstopped]` (s4lint E006): a VDP-port write reached with
    /// the bus provably Running (the crash class). Largely inert on the corpus —
    /// see the module `(a4)` caveat.
    VdpWriteUnstopped,
    /// `[bus.released-at-return]`: a proc whose DECLARED entry holds the bus
    /// ([`BusEntry::Held`]) returns with it provably released — it freed its
    /// CALLER's hold. The mirror of [`Self::StoppedAtReturn`], and reachable only
    /// under a declaration: with the inferred `Unknown` entry there is no contract
    /// to break, which is why s4lint — which has no declared tier — has no such
    /// class.
    ReleasedAtReturn,
}

/// One Z80-bus contract firing: in `proc`, the instruction at `span` violates the
/// bus contract in the way named by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusFiring {
    pub proc: String,
    pub kind: BusFiringKind,
    pub span: Span,
}

/// The bus-ownership lattice value at a program point — the shared three-point
/// [`Tri`] read in this net's vocabulary: `Held` = the 68k owns the bus (a bus
/// request dominates on every path), `NotHeld` = provably released, `Unknown`
/// = caller-dependent or a join of disagreeing states.
type BusState = Tri;
const STOPPED: BusState = Tri::Held;
const RUNNING: BusState = Tri::NotHeld;

/// The bus state a proc's ENTRY is known to be in — the seam where the DECLARED
/// tier feeds the inferred one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BusEntry {
    /// Caller-dependent: the zero-false-positive default (nothing fires purely
    /// off entry).
    Unknown,
    /// The proc declares `requires(<bus ctx>)`, so every call site holds the bus
    /// — a definite entry state, CHECKED at each call site by
    /// `[context.unsatisfied]`. Deliberately not fed by `grants`, which is an
    /// unverified trust root: this seed gates a crash-class check.
    Held,
}

/// The 68k address of the Z80 bus-request register (`engine/constants.asm`).
const Z80_BUS_REQUEST_ADDR: i128 = 0xA1_1100;
/// The VDP port address window: `VDP_DATA` ($C00000) … `VDP_CTRL` ($C00004) and
/// their high-word aliases, all fenced behind the Z80 stop.
const VDP_PORT_LO: i128 = 0xC0_0000;
const VDP_PORT_HI: i128 = 0xC0_0007;

const RETURN_MNEMONICS: [&str; 4] = ["rts", "rte", "rtr", "rtd"];
/// Store-class mnemonics whose LAST operand is the write destination.
const STORE_MNEMONICS: [&str; 3] = ["move", "movem", "clr"];

/// Does `op` name the Z80 bus-request register — as a bare link symbol
/// (`Z80_BUS_REQUEST`, the `.emp` spelling), an explicit-width absolute of that
/// symbol, or the raw address ($A11100, the macros.asm `(...).l` spelling)?
fn is_z80_bus_request(op: &CodeOperand) -> bool {
    match op {
        CodeOperand::Sym(name) => name == "Z80_BUS_REQUEST",
        CodeOperand::AbsSym { target, .. } => target == "Z80_BUS_REQUEST",
        CodeOperand::AbsInt { addr, .. } => *addr == Z80_BUS_REQUEST_ADDR,
        _ => false,
    }
}

/// Does `op` name a VDP port — a resolvable destination only. A register-indirect
/// form (`(a4)`, `4(a6)`, `(a0)+`) is UNRESOLVABLE (the address is in a register)
/// and returns `false`: the s4lint `(a4)` punt.
fn is_vdp_port(op: &CodeOperand) -> bool {
    match op {
        CodeOperand::Sym(name) => name == "VDP_CTRL" || name == "VDP_DATA",
        CodeOperand::AbsSym { target, .. } => target == "VDP_CTRL" || target == "VDP_DATA",
        CodeOperand::AbsInt { addr, .. } => (VDP_PORT_LO..=VDP_PORT_HI).contains(addr),
        _ => false,
    }
}

/// If `instr` toggles the Z80 bus, the state it establishes. A `move` whose
/// destination is `Z80_BUS_REQUEST`: `#$0100` requests the bus (Stopped), `#$0000`
/// releases it (Running). Any other value written to that register is an unmodeled
/// bus event ⇒ `Unknown` (sound: we stop trusting the tracked state). A non-`move`
/// touching the register (the `btst` spin READ) is not a toggle → `None`.
fn bus_toggle(mnem: &str, ops: &[CodeOperand]) -> Option<BusState> {
    if mnem != "move" {
        return None;
    }
    let dst = ops.last()?;
    if !is_z80_bus_request(dst) {
        return None;
    }
    match ops.first() {
        Some(CodeOperand::Imm(0x0100)) => Some(STOPPED),
        Some(CodeOperand::Imm(0x0000)) => Some(RUNNING),
        _ => Some(BusState::Unknown),
    }
}

/// Is `instr` a WRITE to a resolvable VDP port (an E006 candidate)? A store-class
/// mnemonic whose destination operand is a VDP port. Reads (`move VDP_CTRL, d0` —
/// port in SOURCE position) and non-store touches are excluded.
fn is_vdp_write(mnem: &str, ops: &[CodeOperand]) -> bool {
    STORE_MNEMONICS.contains(&mnem) && ops.last().is_some_and(is_vdp_port)
}

/// Apply instruction `idx`'s effect to the bus state. Only the two recognised bus
/// toggles change ownership; every other instruction leaves it unchanged.
fn step(st: BusState, mnem: &str, ops: &[CodeOperand]) -> BusState {
    bus_toggle(mnem, ops).unwrap_or(st)
}

/// Does this `with` region's spliced ACQUIRE take the Z80 bus? Read off the
/// RESOLVED operand of the instructions the acquire splices — the same "key off
/// the resolved operand, never the macro name" discipline the net's own
/// recognition uses, so a context is identified as bus-holding by what its
/// bracket EMITS, not by what it is called.
///
/// The range is `Enter..AcquireEnd`, NOT `Enter..BodyEnd`: the corpus nests
/// `with ints_off { with z80_stopped { … } }`, so the outer region's BODY
/// contains the inner acquire's bus request. Reading the body would identify
/// `ints_off` as a bus context, and the first `requires(ints_off)` written would
/// then be analyzed from a bogus held entry — blinding the `[bus.*]` crash-class
/// check for that whole proc.
///
/// This is how the DECLARED tier tells [`check_bus_state`] which contexts a
/// `requires(...)` makes [`BusEntry::Held`] for.
pub fn region_acquires_bus(items: &[CodeItem], region: &crate::context::Region) -> bool {
    items[region.enter..region.acquire_end].iter().any(|it| {
        matches!(it, CodeItem::Instr { mnemonic, ops, .. }
            if bus_toggle(mnemonic.as_str(), ops) == Some(STOPPED))
    })
}

/// Run the `[bus.*]` machine-state lint over one proc's evaluated CodeBuf. Fires
/// once per instruction whose IN-state is a definite bus-contract violation.
/// `entry` is the proc's declared entry state (see [`BusEntry`]).
pub fn check_bus_state(proc: &str, items: &[CodeItem], entry: BusEntry) -> Vec<BusFiring> {
    let mut firings = Vec::new();
    let cfg = Cfg::build(items);
    let Some(entry_idx) = items.iter().position(|it| matches!(it, CodeItem::Instr { .. })) else {
        return firings;
    };
    let seed = match entry {
        BusEntry::Unknown => BusState::Unknown,
        BusEntry::Held => STOPPED,
    };
    let in_state: BTreeMap<usize, BusState> = must_in_states(
        &cfg,
        Cpu::M68000,
        &[(entry_idx, seed)],
        &|st, mnem, ops| step(st, mnem, ops),
        &|_| true,
    );

    let mut fire = |kind: BusFiringKind, span: Span| {
        firings.push(BusFiring { proc: proc.to_string(), kind, span });
    };

    for (idx, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, ops, span, .. } = it else { continue };
        let Some(st) = in_state.get(&idx).copied() else { continue };
        let mnem = mnemonic.as_str();

        match bus_toggle(mnem, ops) {
            // A stop while provably Stopped — a double-stop (E011).
            Some(STOPPED) if st == STOPPED => {
                fire(BusFiringKind::DoubleStop, *span);
            }
            // A release while provably Running — an unpaired / doubled start (E008).
            Some(RUNNING) if st == RUNNING => {
                fire(BusFiringKind::StartWithoutStop, *span);
            }
            _ => {}
        }

        // A resolvable VDP write while provably Running — the crash class (E006).
        if is_vdp_write(mnem, ops) && st == RUNNING {
            fire(BusFiringKind::VdpWriteUnstopped, *span);
        }

        // A return, judged against the proc's ENTRY contract.
        //
        // With an inferred (`Unknown`) entry the rule is E007: returning provably
        // Stopped leaves the Z80 dead. With a DECLARED held entry that rule
        // INVERTS — the caller holds the bus across this call, so returning held
        // is the contract and returning RELEASED is the violation. Keeping E007
        // under a declared entry would fire on every correct requiring proc,
        // which is the false-positive polarity the net does not take.
        if RETURN_MNEMONICS.contains(&mnem) {
            match entry {
                BusEntry::Unknown if st == STOPPED => fire(BusFiringKind::StoppedAtReturn, *span),
                BusEntry::Held if st == RUNNING => fire(BusFiringKind::ReleasedAtReturn, *span),
                _ => {}
            }
        }
    }
    firings
}
