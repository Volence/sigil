//! sigil-cli: the `sigil` command-line assembler binary.

fn main() {
    println!("sigil");
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_toolchain() {
        assert_eq!(2 + 2, 4);
    }
}
