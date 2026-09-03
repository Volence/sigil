//! asl's `align n` — the pad it inserts at a given program counter. Shared by
//! the AS front-end's `align` directive and the `.emp` front-end's region
//! `@align(n)` field placement. Single source of truth: the two used to carry
//! separate copies of a rule described as "mirror the other exactly", and the
//! copies encoded a rule neither of them held.

/// The number of bytes `align n` advances the program counter by, as asl 1.42
/// Bld 212 computes it.
///
/// asl does **not** round the address up. It computes the aligned target on the
/// **low 32 bits of the PC read as a signed `i32`**, using C's truncating
/// remainder (whose sign follows the dividend), and then advances by the
/// unsigned 32-bit difference:
///
/// ```text
/// t     = (i32) (pc + n - 1)
/// a     = t - (t % n)            // C '%': truncates toward zero
/// pad   = (u32) (a - (i32) pc)
/// ```
///
/// For a PC whose low 32 bits are **non-negative** (every ROM address) this is
/// exactly the plain round-up, and an already-aligned PC gets a zero pad. For a
/// PC whose low 32 bits are **negative** — which is every 68k RAM address,
/// `$FF_0000`-aliased or otherwise — truncation rounds toward zero instead of
/// down, so the result usually lands one block high and an already-aligned
/// address advances a full `n`. This is not a quirk we can round away: it
/// decides where every `@align`ed RAM field sits.
///
/// Measured against asl over 30 listing rows spanning both signs, both sides of
/// `$8000_0000`, non-power-of-two `n`, and `n = 2`; the probes and their listing
/// rows are in `docs/superpowers/probes/2026-09-03-align/`.
///
/// Two asl behaviours are deliberately NOT modelled, both outside the corpus:
/// asl truncates `n` to a 16-bit `Word` (so `align -256` acts as `align $FF00`,
/// and `align 0` aborts asl with SIGFPE); and asl carries the PC wider than 32
/// bits, so an align off the top of the address space lands at `$1_0000_0000`
/// where this returns the pad that wraps to `$0000_0000`.
///
/// `n` must be non-zero; callers reject `n <= 0` before reaching here.
pub fn asl_align_pad(pc: u32, n: u32) -> u32 {
    debug_assert!(n != 0, "align 0 has no pad (asl divides by zero and aborts)");
    let n_i = n as i32;
    let t = pc.wrapping_add(n).wrapping_sub(1) as i32;
    // `t % n_i` is Rust's truncating remainder — the same operator C uses, with
    // the same sign-of-dividend behaviour that produces the RAM-side overshoot.
    // `n_i` is `i32::MIN` only for `n == $8000_0000`, where `%` is still defined.
    let a = t.wrapping_sub(t.wrapping_rem(n_i));
    (a as u32).wrapping_sub(pc)
}

#[cfg(test)]
mod tests {
    use super::asl_align_pad;

    /// Every row is a `(pc, n, asl's answer)` triple read off an asl 1.42 Bld 212
    /// listing under the corpus flags `-xx -n -q -A -L -U -i .`. The probe
    /// sources and full listings are in `docs/superpowers/probes/2026-09-03-align/`.
    #[test]
    fn reproduces_every_measured_asl_row() {
        // (pc, n, resulting pc)
        let rows: &[(u32, u32, u32)] = &[
            // ── ROM side: the plain round-up, no-op when already aligned ──
            (0x0000_0000, 256, 0x0000_0000),
            (0x0000_B000, 256, 0x0000_B000),
            (0x0000_B001, 256, 0x0000_B100),
            (0x0000_B02A, 256, 0x0000_B100),
            (0x0000_B02A, 100, 0x0000_B02C),
            (0x0000_B02A, 2, 0x0000_B02A),
            (0x0000_B02B, 2, 0x0000_B02C),
            (0x0000_B000, 2, 0x0000_B000),
            (0x7FFF_B02A, 256, 0x7FFF_B100),
            // Phased ROM: `phase $B000` + `ds.b 5` then `align 256` -> $B100,
            // NOT $B200. The whole reason this file exists.
            (0x0000_B005, 256, 0x0000_B100),
            (0x0000_B040, 256, 0x0000_B100),
            (0x0000_B045, 256, 0x0000_B100),
            (0x0000_B102, 256, 0x0000_B200),
            // ── RAM side: truncation toward zero, so an aligned PC still moves ──
            (0xFFFF_B000, 256, 0xFFFF_B100),
            (0xFFFF_B001, 256, 0xFFFF_B100),
            (0xFFFF_B002, 256, 0xFFFF_B200),
            (0xFFFF_B003, 256, 0xFFFF_B200),
            (0xFFFF_B0FF, 256, 0xFFFF_B200),
            (0xFFFF_B02A, 256, 0xFFFF_B200),
            (0xFFFF_B100, 256, 0xFFFF_B200),
            (0xFFFF_B101, 256, 0xFFFF_B200),
            (0xFFFF_B102, 256, 0xFFFF_B300),
            (0xFFFF_B000, 100, 0xFFFF_B0B4),
            (0xFFFF_B02A, 100, 0xFFFF_B0B4),
            (0xFFFF_B02A, 3, 0xFFFF_B02C),
            (0xFFFF_B02A, 2, 0xFFFF_B02C),
            (0xFFFF_B02B, 2, 0xFFFF_B02C),
            (0xFFFF_0000, 2, 0xFFFF_0002),
            (0xFFFF_FE00, 256, 0xFFFF_FF00),
            // Just under and just over the sign boundary.
            (0x8000_0000, 256, 0x8000_0100),
            (0x8000_0001, 256, 0x8000_0100),
        ];
        let mut wrong = Vec::new();
        for &(pc, n, want) in rows {
            let got = pc.wrapping_add(asl_align_pad(pc, n));
            if got != want {
                wrong.push(format!("align {n} at ${pc:08X}: got ${got:08X}, asl says ${want:08X}"));
            }
        }
        assert!(wrong.is_empty(), "{} of {} rows disagree with asl:\n{}", wrong.len(), rows.len(), wrong.join("\n"));
    }

    /// The three rows where asl's PC leaves the 32-bit space (`$1_0000_0000`).
    /// The pad is still the right unsigned-32 delta; only the wide sum is
    /// unmodelled, and it wraps to zero here.
    #[test]
    fn the_rows_that_leave_the_32_bit_space_wrap() {
        for (pc, n) in [(0xFFFF_FF00u32, 256u32), (0xFFFF_FF01, 256), (0xFFFF_FFFF, 256), (0xFFFF_B000, 0x8000)] {
            assert_eq!(pc.wrapping_add(asl_align_pad(pc, n)), 0, "align {n} at ${pc:08X}");
        }
    }

    /// `align 1` never moves anything, on either side of the sign boundary.
    #[test]
    fn align_one_is_always_a_noop() {
        for pc in [0u32, 1, 0x0000_B02A, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_B02A, 0xFFFF_FFFF] {
            assert_eq!(asl_align_pad(pc, 1), 0, "align 1 at ${pc:08X}");
        }
    }
}
