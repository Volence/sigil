//! sigil-isa: instruction-set encoders/decoders. Plan 1 targets the Z80 subset.

pub mod z80 {}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_toolchain() {
        assert_eq!(2 + 2, 4);
    }
}
