//! CPU tag and (Task 4) the Backend / IrStreamer traits.

/// Which instruction set a [`crate::Section`]'s bytes are encoded for.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Cpu {
    Z80,
    M68000,
}
