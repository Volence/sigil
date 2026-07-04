//! state: the AS assembler-state unit + the save/restore stack.

// `AsmState` is consumed by the eval loop (later task); unused until then.

use sigil_ir::backend::Cpu;

/// The whole assembler-state unit `save`/`restore` preserve.
#[derive(Clone, Debug)]
pub struct AsmState {
    pub cpu: Cpu,
    /// Phase VMA base (`None` after `dephase`).
    pub vma_base: Option<u32>,
    /// `padding on/off` (68k auto-even-pad; inert for Z80 bytes in M0).
    pub padding: bool,
    /// `supmode on/off` (68k privileged-instruction mode; inert for Z80 bytes).
    pub supmode: bool,
    saved: Vec<Saved>,
}

#[derive(Clone, Debug)]
struct Saved {
    cpu: Cpu,
    vma_base: Option<u32>,
    padding: bool,
    supmode: bool,
}

impl AsmState {
    /// New state with `initial_cpu`, no phase, 68k defaults (padding on, supmode off).
    pub fn new(initial_cpu: Cpu) -> Self {
        AsmState {
            cpu: initial_cpu,
            vma_base: None,
            padding: true,
            supmode: false,
            saved: Vec::new(),
        }
    }

    /// `save`: push a snapshot of the whole unit.
    pub fn save(&mut self) {
        self.saved.push(Saved {
            cpu: self.cpu,
            vma_base: self.vma_base,
            padding: self.padding,
            supmode: self.supmode,
        });
    }

    /// `restore`: pop the last snapshot. Err if the stack is empty.
    pub fn restore(&mut self) -> Result<(), &'static str> {
        let s = self
            .saved
            .pop()
            .ok_or("`restore` with no matching `save`")?;
        self.cpu = s.cpu;
        self.vma_base = s.vma_base;
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
    fn save_restore_round_trips_the_whole_unit() {
        let mut s = AsmState::new(Cpu::M68000);
        s.padding = false;
        s.supmode = true;
        s.save();
        s.cpu = Cpu::Z80;
        s.vma_base = Some(0x8000);
        s.padding = true;
        assert_eq!(s.cpu, Cpu::Z80);
        s.restore().unwrap();
        // restore pops back the whole unit.
        assert_eq!(s.cpu, Cpu::M68000);
        assert_eq!(s.vma_base, None);
        assert!(!s.padding);
        assert!(s.supmode);
    }

    #[test]
    fn restore_without_save_is_an_error() {
        let mut s = AsmState::new(Cpu::Z80);
        assert!(s.restore().is_err());
    }
}
