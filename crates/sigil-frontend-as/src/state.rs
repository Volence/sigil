//! state: the AS assembler-state unit + the save/restore stack.

use crate::charset::CodePage;
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
/// - `save` snapshots the CPU and the undocumented-Z80 mode
///   ([`AsmState::z80_undoc`]). `restore` re-applies both and, **only
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
    /// Whether the Z80 was selected as `z80undoc`, the spelling that enables its
    /// undocumented instructions. Under it asl reads the index-register halves
    /// (`ixl`, `ixu`/`ixh`, `iyl`, `iyu`/`iyh`) as registers and reports
    /// `MOMCPU` as `$80DC`; under plain `z80` those names are ordinary symbols.
    /// Part of the `save`/`restore` snapshot, measured: `cpu z80undoc`, `save`,
    /// `cpu z80`, `restore` gives `ld a,ixl` back as `DD 7D`.
    pub z80_undoc: bool,
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
    /// The `charset` code page: the character-to-byte translation every string
    /// and character literal passes through on its way to a byte.
    ///
    /// Deliberately NOT part of the `save`/`restore` snapshot, and that is a
    /// measurement rather than a convenience. asl:
    ///
    /// ```text
    ///       3/       0 :                     	charset $41,$11
    ///       4/       0 : 11                  	dc.b "A"
    ///       5/       1 :                     	save
    ///       6/       1 :                     	charset $41,$44
    ///       7/       1 : 44                  	dc.b "A"
    ///       8/       2 : ALL                  	restore
    ///       9/       2 : 44                  	dc.b "A"
    /// ```
    ///
    /// The `44` on line 9 is the finding: `restore` does not bring the page
    /// back. The raw `$41` index is what makes the probe conclusive — spelling
    /// it `charset 'A',$44` makes the inner line inert for an unrelated reason
    /// (see [`CodePage`]), and a first attempt at this probe read as
    /// "save/restore brackets the page" for exactly that reason.
    ///
    /// Reset to the identity page at the start of every pass, which happens for
    /// free because `Asm` rebuilds this whole struct per pass.
    // REASON: the doc comment above quotes asl listings verbatim, and asl separates
    // its listing columns with TABS. The tabs ARE the evidence: reflowing them to
    // spaces would silently edit a reference assembler's output that later parcels
    // compare against. Scoped to this item, never crate wide.
    #[allow(clippy::tabs_in_doc_comments)]
    pub charset: CodePage,
    saved: Vec<Saved>,
}

#[derive(Clone, Debug)]
struct Saved {
    cpu: Cpu,
    z80_undoc: bool,
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
            z80_undoc: false,
            cpu_declared: initial_cpu.is_some(),
            disp: 0,
            padding: default_padding(cpu),
            supmode: default_supmode(cpu),
            charset: CodePage::identity(),
            saved: Vec::new(),
        }
    }

    /// The `cpu` **directive**: [`set_cpu`](AsmState::set_cpu), and latch the
    /// unit as having DECLARED its processor. Only a real declaration calls
    /// this; `restore` re-applies a CPU without declaring one. `z80_undoc` is
    /// whether the name was the undocumented Z80's (see [`AsmState::z80_undoc`]);
    /// every `cpu` line sets it, so `cpu z80` after `cpu z80undoc` clears it.
    pub fn declare_cpu(&mut self, cpu: Cpu, z80_undoc: bool) {
        self.cpu_declared = true;
        self.set_cpu(cpu);
        self.z80_undoc = z80_undoc;
    }

    /// The `cpu` **directive**: set the CPU and reset `padding`/`supmode` to that
    /// CPU's defaults, UNCONDITIONALLY (asl resets even to the same CPU).
    pub fn set_cpu(&mut self, cpu: Cpu) {
        self.cpu = cpu;
        self.padding = default_padding(cpu);
        self.supmode = default_supmode(cpu);
    }

    /// `save`: push a snapshot of the CPU and the undocumented-Z80 mode (only
    /// these matter on `restore`; the padding/supmode reset is a side effect of
    /// the CPU re-application).
    pub fn save(&mut self) {
        self.saved.push(Saved { cpu: self.cpu, z80_undoc: self.z80_undoc });
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
        self.z80_undoc = s.z80_undoc;
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

    /// The code page starts at the identity and `save`/`restore` does not
    /// bracket it. Asserted at the STRUCT level as well as end-to-end
    /// (`tests/as_charset.rs::save_and_restore_do_not_bracket_the_page`,
    /// which quotes the asl listing) because the two can fail apart: putting
    /// the page into `Saved` would keep every existing byte test green and
    /// break only a source that changes the page inside a `save` block, which
    /// no corpus in this workspace does.
    #[test]
    fn save_and_restore_do_not_bracket_the_code_page() {
        let mut s = AsmState::new(Some(Cpu::M68000));
        assert!(s.charset.is_identity(), "a new state starts on the identity page");
        s.charset.set(0x41, 0x11);
        s.save();
        s.charset.set(0x41, 0x44);
        s.restore().unwrap();
        assert_eq!(
            s.charset.map_char('A'),
            0x44,
            "restore must not bring the saved code page back (asl: `dc.b \"A\"` reads 44)"
        );
        assert!(!s.charset.is_identity());
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
        s.declare_cpu(Cpu::M68000, false);
        assert!(s.cpu_declared);
        s.save();
        s.set_cpu(Cpu::Z80);
        s.restore().unwrap();
        assert!(s.cpu_declared, "save/restore cannot un-declare the unit");
    }
}
