//! G0b final review C-I6: an `include` / `BINCLUDE` path the named-root rule refuses
//! surfaces as an AS diagnostic carrying the rule's own message (`source_path`'s `Err`
//! arm), never a read of whatever the path happens to name. CONTROL: the rooted spelling
//! of the same file assembles.
//!
//! Runner: `cargo test -p sigil-frontend-as` (the landing bar's full suite).

use sigil_frontend_as::{assemble, Options};
use sigil_ir::Cpu;
use std::sync::Arc;

fn opts(roots: sigil_span::roots::SourceRoots) -> Options {
    Options {
        roots: Some(Arc::new(roots)),
        initial_cpu: Some(Cpu::M68000),
        defines: vec![],
        guarded_defines: vec![],
        cli_defines: vec![],
        share_file: false,
        include_root: None,
    }
}

#[test]
fn a_refused_include_path_is_an_as_error_naming_the_rule() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, game) = (dir.path().join("engine"), dir.path().join("game"));
    std::fs::create_dir_all(&engine).unwrap();
    std::fs::create_dir_all(&game).unwrap();
    std::fs::write(game.join("part.asm"), "\tdc.w\t$1234\n").unwrap();
    std::fs::write(game.join("blob.bin"), [1u8, 2]).unwrap();
    // A game in its own repository: no transitional tree, so an unrooted path is refused.
    let roots = || sigil_span::roots::SourceRoots::new(engine.clone(), game.clone());
    assemble("\tinclude \"game/part.asm\"\n\tbinclude \"game/blob.bin\"\n", &opts(roots()))
        .unwrap_or_else(|d| panic!("CONTROL: the rooted spellings assemble: {d:?}"));
    for (src, code) in [
        ("\tinclude \"part.asm\"\n", "[root.ambiguous] `part.asm`"),
        ("\tbinclude \"nowhere.bin\"\n", "[root.unrooted] `nowhere.bin`"),
        ("\tinclude \"game/../../x.asm\"\n", "[root.escape]"),
    ] {
        let diags = match assemble(src, &opts(roots())) {
            Ok(_) => panic!("{src:?} must be refused"),
            Err(d) => d,
        };
        assert!(diags.iter().any(|d| d.message.contains(code)), "{src:?}: want {code}, got {diags:?}");
    }
}
