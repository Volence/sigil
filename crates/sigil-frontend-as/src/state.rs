//! state: the AS assembler-state unit + the save/restore stack.

use sigil_ir::backend::Cpu;

/// The assembler-state unit `save`/`restore` preserve.
///
/// asl-faithful (Bld 212, probe-verified): `save`/`restore` preserve the CPU
/// type, `padding`, and `supmode` (the privileged/listing-style flags). They do
/// NOT touch the phase displacement — `phase`/`dephase` are a SEPARATE, explicitly
/// balanced mechanism. A `save` taken while phased, followed by `dephase`, then
/// `restore`, does NOT resurrect the phase (probe: label after such a `restore`
/// reads the physical location, not the window VMA). The continuous physical
/// location counter itself lives in `Asm` (never rewound by `restore`).
#[derive(Clone, Debug)]
pub struct AsmState {
    pub cpu: Cpu,
    /// Phase displacement: `$`/labels report `physical + disp`. `phase addr` sets
    /// `disp = addr - physical_at_phase`; `dephase` sets `disp = 0`. Zero when not
    /// phased. NOT saved/restored (asl treats phase as its own balanced pair).
    pub disp: i64,
    /// `padding on/off` (68k auto-even-pad; inert for Z80 bytes in M0).
    pub padding: bool,
    /// `supmode on/off` (68k privileged-instruction mode; inert for Z80 bytes).
    pub supmode: bool,
    saved: Vec<Saved>,
}

#[derive(Clone, Debug)]
struct Saved {
    cpu: Cpu,
    padding: bool,
    supmode: bool,
}

impl AsmState {
    /// New state with `initial_cpu`, no phase, 68k defaults (padding on, supmode off).
    pub fn new(initial_cpu: Cpu) -> Self {
        AsmState {
            cpu: initial_cpu,
            disp: 0,
            padding: true,
            supmode: false,
            saved: Vec::new(),
        }
    }

    /// `save`: push a snapshot of the cpu/padding/supmode unit (NOT the phase).
    pub fn save(&mut self) {
        self.saved.push(Saved {
            cpu: self.cpu,
            padding: self.padding,
            supmode: self.supmode,
        });
    }

    /// `restore`: pop the last snapshot. Err if the stack is empty. Restores
    /// cpu/padding/supmode only — the phase displacement is untouched.
    pub fn restore(&mut self) -> Result<(), &'static str> {
        let s = self
            .saved
            .pop()
            .ok_or("`restore` with no matching `save`")?;
        self.cpu = s.cpu;
        self.padding = s.padding;
        self.supmode = s.supmode;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AsmState;
    use sigil_ir::backend::Cpu;

    #[test]
    fn save_restore_round_trips_cpu_padding_supmode() {
        let mut s = AsmState::new(Cpu::M68000);
        s.padding = false;
        s.supmode = true;
        s.save();
        s.cpu = Cpu::Z80;
        s.padding = true;
        assert_eq!(s.cpu, Cpu::Z80);
        s.restore().unwrap();
        // restore pops back cpu/padding/supmode.
        assert_eq!(s.cpu, Cpu::M68000);
        assert!(!s.padding);
        assert!(s.supmode);
    }

    #[test]
    fn restore_does_not_touch_phase_displacement() {
        // asl truth: phase/dephase is a separate mechanism; `restore` leaves the
        // displacement exactly as `dephase` (or a live `phase`) left it.
        let mut s = AsmState::new(Cpu::M68000);
        s.save();
        s.disp = 0x1234; // as if `phase` set a displacement after the save
        s.restore().unwrap();
        assert_eq!(s.disp, 0x1234, "restore must not rewind the phase displacement");
    }

    #[test]
    fn restore_without_save_is_an_error() {
        let mut s = AsmState::new(Cpu::Z80);
        assert!(s.restore().is_err());
    }
}
