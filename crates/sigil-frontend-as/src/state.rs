//! state: the AS assembler-state unit + the save/restore stack.

use sigil_ir::backend::Cpu;

/// A CPU's default `padding` flag. 68000 defaults to ON (auto-even-pad); Z80 has
/// no alignment concept (byte stream), so the flag is inert there but modelled as
/// ON for a uniform default. asl-verified: `padding off; cpu 68000` -> ON.
fn default_padding(_cpu: Cpu) -> bool {
    true
}

/// A CPU's default `supmode` flag (privileged-instruction mode). Always OFF.
/// Byte-inert — `supmode` only gates a privileged-instruction warning — but
/// modelled for state fidelity.
fn default_supmode(_cpu: Cpu) -> bool {
    false
}

/// The CPU the state runs on while NOTHING has declared one yet.
///
/// It is provisional, never a target choice: the moment the unit reaches a
/// CPU-dependent construct (any emitted byte) with [`AsmState::cpu_declared`]
/// still false, the front-end refuses the whole unit, so no accepted output is
/// ever encoded against this value. It is `Z80` only because that is what the
/// old silent default was — keeping it there means the ONLY behavioural change
/// is the refusal itself.
const PROVISIONAL_CPU: Cpu = Cpu::Z80;

/// The assembler-state unit and the `save`/`restore` stack.
///
/// **`padding`/`supmode` semantics (asl 1.42 Bld 212, live-probe-verified — see
/// `docs/superpowers/notes/2026-07-04-m1d-t0.1-padding-probes.md`):**
///
/// - The `cpu X` **directive** resets `padding`/`supmode` to CPU defaults
///   **unconditionally** — even when `X` is the current CPU (probe d:
///   `padding off; cpu 68000` ends padding ON). See [`AsmState::set_cpu`].
/// - `save` snapshots only the CPU. `restore` re-applies the saved CPU and, **only
///   if it differs from the current one**, resets `padding`/`supmode` to that CPU's
///   default (probe t14). If the CPU is unchanged, they are left as-is (probe t12).
///   `restore` **never** restores a saved `padding`/`supmode` value (probes b, c:
///   the post-`save` value survives, the saved one does not).
///
/// The prior "`save`/`restore` preserve padding/supmode" claim was wrong (F1): it
/// made everything after boot.asm's `save;cpu z80;…;restore` keep `padding off`,
/// but asl's `restore` re-switches to 68000 and resets padding **ON** there.
///
/// `save`/`restore` do NOT touch the phase displacement — `phase`/`dephase` are a
/// SEPARATE, explicitly balanced mechanism. A `save` while phased, then `dephase`,
/// then `restore`, does NOT resurrect the phase. The continuous physical location
/// counter lives in `Asm` (never rewound by `restore`).
#[derive(Clone, Debug)]
pub struct AsmState {
    pub cpu: Cpu,
    /// Whether a CPU was ever DECLARED for this assembly unit — by the caller
    /// (`Options::initial_cpu = Some(..)`) or by a `cpu` directive anywhere in
    /// the unit, root or included file.
    ///
    /// A one-way latch, and deliberately NOT part of the `save`/`restore`
    /// snapshot: declaring a processor is a property of the unit, not a scoped
    /// piece of assembler state, so a `save`/`restore` pair cannot un-declare
    /// it and `restore` re-applying a saved CPU is not itself a declaration.
    pub cpu_declared: bool,
    /// Phase displacement: `$`/labels report `physical + disp`. `phase addr` sets
    /// `disp = addr - physical_at_phase`; `dephase` sets `disp = 0`. Zero when not
    /// phased. NOT saved/restored (asl treats phase as its own balanced pair).
    pub disp: i64,
    /// `padding on/off` (68k auto-even-pad: a `$00` byte before a word/long/
    /// instruction at an odd logical `$`). Reset to the CPU default on every `cpu`
    /// directive and on a CPU-changing `restore`.
    pub padding: bool,
    /// `supmode on/off` (68k privileged-instruction mode; byte-inert).
    pub supmode: bool,
    saved: Vec<Saved>,
}

#[derive(Clone, Debug)]
struct Saved {
    cpu: Cpu,
}

impl AsmState {
    /// New state, no phase, CPU defaults (padding on, supmode off).
    ///
    /// `initial_cpu` is `Some(c)` when the CALLER declares the processor and
    /// `None` when it does not. `None` does not mean "pick one" — it runs on
    /// [`PROVISIONAL_CPU`] with [`cpu_declared`](AsmState::cpu_declared) false,
    /// which the front-end turns into a refusal at the first CPU-dependent
    /// construct.
    pub fn new(initial_cpu: Option<Cpu>) -> Self {
        let cpu = initial_cpu.unwrap_or(PROVISIONAL_CPU);
        AsmState {
            cpu,
            cpu_declared: initial_cpu.is_some(),
            disp: 0,
            padding: default_padding(cpu),
            supmode: default_supmode(cpu),
            saved: Vec::new(),
        }
    }

    /// The `cpu` **directive**: [`set_cpu`](AsmState::set_cpu), and latch the
    /// unit as having DECLARED its processor. Only a real declaration calls
    /// this — `restore` re-applies a CPU without declaring one.
    pub fn declare_cpu(&mut self, cpu: Cpu) {
        self.cpu_declared = true;
        self.set_cpu(cpu);
    }

    /// The `cpu` **directive**: set the CPU and reset `padding`/`supmode` to that
    /// CPU's defaults, UNCONDITIONALLY (asl resets even to the same CPU).
    pub fn set_cpu(&mut self, cpu: Cpu) {
        self.cpu = cpu;
        self.padding = default_padding(cpu);
        self.supmode = default_supmode(cpu);
    }

    /// `save`: push a snapshot of the CPU (only the CPU matters on `restore`; the
    /// padding/supmode reset is a side effect of the CPU re-application).
    pub fn save(&mut self) {
        self.saved.push(Saved { cpu: self.cpu });
    }

    /// `restore`: pop the last snapshot; Err if empty. Re-apply the saved CPU —
    /// resetting padding/supmode to its default ONLY if that CPU differs from the
    /// current one (a real switch). Same CPU ⇒ padding/supmode untouched. The saved
    /// padding/supmode value is never restored. Phase displacement is untouched.
    pub fn restore(&mut self) -> Result<(), &'static str> {
        let s = self
            .saved
            .pop()
            .ok_or("`restore` with no matching `save`")?;
        if s.cpu != self.cpu {
            self.set_cpu(s.cpu);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AsmState;
    use sigil_ir::backend::Cpu;

    // The three cases below encode the live-asl truth (padding-probes.md). They
    // model the SENSOR-observed padding state, NOT the prior (wrong) round-trip.

    #[test]
    fn cpu_directive_resets_padding_even_to_same_cpu() {
        // Probe d: `padding off; cpu 68000` -> padding ON.
        let mut s = AsmState::new(Some(Cpu::M68000));
        s.padding = false;
        s.set_cpu(Cpu::M68000); // same CPU, still resets
        assert!(s.padding, "the cpu directive resets padding unconditionally");
    }

    #[test]
    fn restore_with_cpu_change_resets_padding_to_default() {
        // Probe t14: `padding off; save; cpu z80; restore` -> padding ON (the
        // restore switches z80->68000, an actual change, so padding resets ON).
        let mut s = AsmState::new(Some(Cpu::M68000));
        s.padding = false;
        s.save(); // saves cpu = 68000
        s.set_cpu(Cpu::Z80); // switch to z80 (resets padding to z80 default)
        s.restore().unwrap(); // z80 -> 68000: a change, reset to 68000 default
        assert_eq!(s.cpu, Cpu::M68000);
        assert!(s.padding, "cpu-changing restore resets padding to default");
    }

    #[test]
    fn restore_same_cpu_preserves_padding_and_never_restores_saved() {
        // Probe t12: `padding off; save; restore` -> stays OFF (no cpu change).
        let mut s = AsmState::new(Some(Cpu::M68000));
        s.padding = false;
        s.save();
        s.restore().unwrap();
        assert!(!s.padding, "same-cpu restore leaves padding as-is");

        // Probe b: `padding off; save; padding on; restore` -> ON (the saved OFF
        // is NOT brought back; the post-save value survives).
        let mut s = AsmState::new(Some(Cpu::M68000));
        s.padding = false;
        s.save();
        s.padding = true;
        s.restore().unwrap();
        assert!(s.padding, "restore never restores the saved padding value");
    }

    #[test]
    fn restore_does_not_touch_phase_displacement() {
        // asl truth: phase/dephase is a separate mechanism; `restore` leaves the
        // displacement exactly as `dephase` (or a live `phase`) left it.
        let mut s = AsmState::new(Some(Cpu::M68000));
        s.save();
        s.disp = 0x1234; // as if `phase` set a displacement after the save
        s.restore().unwrap();
        assert_eq!(s.disp, 0x1234, "restore must not rewind the phase displacement");
    }

    #[test]
    fn restore_without_save_is_an_error() {
        let mut s = AsmState::new(Some(Cpu::Z80));
        assert!(s.restore().is_err());
    }

    #[test]
    fn caller_declaring_the_cpu_counts_as_declared() {
        assert!(AsmState::new(Some(Cpu::M68000)).cpu_declared);
        assert!(AsmState::new(Some(Cpu::Z80)).cpu_declared);
        assert!(!AsmState::new(None).cpu_declared);
    }

    #[test]
    fn only_a_declaration_latches_declared() {
        // `set_cpu` is the mechanical state change (`restore` uses it); it is
        // NOT a declaration. `declare_cpu` is, and the latch is one-way.
        let mut s = AsmState::new(None);
        s.set_cpu(Cpu::M68000);
        assert!(!s.cpu_declared, "restore-style set_cpu must not declare");
        s.declare_cpu(Cpu::M68000);
        assert!(s.cpu_declared);
        s.save();
        s.set_cpu(Cpu::Z80);
        s.restore().unwrap();
        assert!(s.cpu_declared, "save/restore cannot un-declare the unit");
    }
}
