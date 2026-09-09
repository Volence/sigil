//! `DEFINED(NAME)` is 1 when a symbol of that name has been defined AT THIS
//! POINT IN THIS PASS, and 0 otherwise.
//!
//! The three community disassemblies each spend it in exactly two places, all
//! in `sound/_smps2asm_inc.asm`, all spelled `DEFINED(loc)` on a macro
//! parameter, and all guarding a `fatal`:
//!
//! ```text
//!     if (MOMPASS=1)&&(DEFINED(loc))
//!         fatal "Tracks for Sonic 1 songs must come after the start of the song"
//!     endif
//! ```
//!
//! ## Why the pass matters, and what a naive answer would get wrong
//!
//! `DEFINED` is a question about assembly STATE at a point in a traversal, and
//! sigil converges over several passes with the symbol table CARRIED FORWARD:
//! that is how a forward reference gets a value. Answering `DEFINED` from that
//! carried table would report every symbol in the program as defined from the
//! first line of every pass after the first.
//!
//! asl does not do that, and it is measured rather than assumed. Probe
//! `db.asm`, exit 0, `2 passes`, `0 errors` (the `jmp Later` is what forces the
//! second pass, and it resolves to `Later`'s real address on it):
//!
//! ```text
//!       12/       0 : 4EF8 0006             jmp     Later
//!       13/       4 : 00                    dc.b    DEFINED(Later)
//!       14/       5 : 00                    dc.b    DEFINED(LaterEqu)
//!       15/       6 :                    Later:
//! ```
//!
//! and its `message` arms print `pass1 Later=0` and `pass2 Later=0`. asl carries
//! the VALUE forward and does not carry the DEFINEDNESS forward. sigil answers
//! from [`Asm::defined_this_pass`], which is never seeded, so it agrees on every
//! pass rather than only on the first.
//!
//! ## Provenance
//!
//! Every expected value comes from
//! `/home/volence/sonic_hacks/s1disasm/build_tools/Linux-x86_64/asl`, md5
//! `61e672562465725a8c102288a7da9098`, invoked `-xx -n -q -A -L -U -i .`, exit
//! status checked and quoted per test. The other build in this workspace (md5
//! `0dee1f98e6480a4783d27ffd8b90896f`) was not run for any value here.

use sigil_frontend_as::{assemble_root_located, Options};

fn assemble(body: &str) -> Result<Vec<u8>, Vec<String>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("probe.asm");
    std::fs::write(&path, body).expect("write probe");
    match assemble_root_located(&path, &Options::default()) {
        Ok(m) => {
            let resolved =
                sigil_link::resolve_layout(&m.sections, &sigil_ir::SymbolTable::new(), true)
                    .expect("resolve_layout");
            let linked = sigil_link::link(&resolved, &sigil_ir::SymbolTable::new()).expect("link");
            Ok(sigil_link::flatten(&linked, 0x00).unwrap())
        }
        Err(f) => Err(f.diags.iter().map(|d| d.message.clone()).collect()),
    }
}

fn bytes(body: &str) -> Vec<u8> {
    match assemble(body) {
        Ok(b) => b,
        Err(d) => panic!("expected bytes, got diagnostics: {d:?}"),
    }
}

const HEAD: &str = "\tcpu 68000\n\tpadding off\n\torg 0\n";
/// A forward reference, so the fixpoint runs more than one iteration. Without
/// it a test cannot tell a per-pass answer from a carried-forward one.
const FWD: &str = "\tdc.w Later2-*\nLater2:\n";

/// A name defined ABOVE is 1; a name defined BELOW and a name defined nowhere
/// are both 0.
///
/// asl, probe `da.asm`, exit 0, `1 pass`, `0 errors`:
///
/// ```text
///        9/       0 : 00                   Early:  dc.b    0
///       10/       1 : 01                           dc.b    DEFINED(Early)
///       11/       2 : 00                           dc.b    DEFINED(Later)
///       12/       3 : 00                           dc.b    DEFINED(Nowhere)
/// ```
#[test]
fn defined_answers_from_the_position_in_the_file() {
    let src = format!(
        "{HEAD}Early:\tdc.b 0\n\tdc.b DEFINED(Early)\n\tdc.b DEFINED(Later)\n\
         \tdc.b DEFINED(Nowhere)\nLater:\tdc.b 0\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x00, 0x01, 0x00, 0x00, 0x00]);
}

/// THE PASS PROPERTY. A label defined LATER in the file is 0 on the pass that
/// emits, in a file that provably takes more than one pass, and it is 0 for the
/// same reason on every earlier one.
///
/// asl, probe `db.asm`, exit 0, `2 passes`, `0 errors`, quoted in the module
/// header: `dc.b DEFINED(Later)` is `00` while the `jmp Later` two lines above
/// it has resolved to `4EF8 0006`.
///
/// An implementation that answered `DEFINED` from the carried symbol table
/// would emit `01` here while emitting the same resolved `jmp`. That is the
/// other answer this test can produce, and it is the one a naive implementation
/// gives.
#[test]
fn defined_of_a_forward_label_is_zero_on_the_pass_that_emits() {
    let src = format!(
        "{HEAD}\tjmp Later\n\tdc.b DEFINED(Later)\n\tdc.b DEFINED(LaterEqu)\n\
         Later:\n\tnop\nLaterEqu\tequ 7\n{FWD}\tend\n"
    );
    let b = bytes(&src);
    // `jmp Later` assembled with Later's real address, so the pass that emitted
    // these bytes had the forward reference resolved.
    assert_eq!(&b[..4], &[0x4E, 0xF8, 0x00, 0x06], "jmp must be resolved");
    assert_eq!(&b[4..6], &[0x00, 0x00], "both forward names must read as undefined");
}

/// The corpus's own guard: `(MOMPASS=1)&&(DEFINED(loc))` over a name defined
/// nowhere yet does not fire, so the `fatal` it guards never runs and the
/// following bytes are assembled.
///
/// This is the shape at `sound/_smps2asm_inc.asm` lines 238 and 282 in all
/// three disassemblies, reduced to the macro-parameter substitution it performs.
#[test]
fn the_corpus_guard_shape_does_not_fire() {
    let src = format!(
        "{HEAD}Chk\tmacro loc\n\tif (MOMPASS=1)&&(DEFINED(loc))\n\
         \tfatal \"came after the start of the song\"\n\tendif\n\tdc.b $77\n\tendm\n\
         \tChk\tVoicesLater\n{FWD}VoicesLater:\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x77, 0x00, 0x02]);
}

/// The FUNCTION NAME folds case; the ARGUMENT does not. asl, probe `da.asm`,
/// exit 0: `dc.b defined(Early)` is `01` and `dc.b Defined(Nowhere)` is `00`,
/// and the symbol table it prints carries `CASESENSITIVE : 1`.
#[test]
fn the_builtin_name_folds_case_but_the_argument_does_not() {
    let src = format!(
        "{HEAD}Early:\tdc.b 0\n\tdc.b defined(Early)\n\tdc.b Defined(Early)\n\
         \tdc.b DEFINED(EARLY)\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x00, 0x01, 0x01, 0x00]);
}

/// Every symbol-defining form counts, and a builtin counts too. asl, probe
/// `dc.asm`, exit 0, `1 pass`, `0 errors`:
///
/// ```text
///        8/       0 : 01                   dc.b    DEFINED(V)         ; V set 1
///        9/       1 : 01                   dc.b    DEFINED(E)         ; E equ 2
///       10/       2 : 01                   dc.b    DEFINED(MOMCPU)
///       11/       3 : 01                   dc.b    DEFINED(TRUE)
///       12/       4 : 00                   dc.b    DEFINED(Mac)       ; a MACRO
/// ```
#[test]
fn set_equ_and_builtins_count_but_a_macro_name_does_not() {
    let src = format!(
        "{HEAD}V\tset 1\nE\tequ 2\nMac\tmacro loc\n\tdc.b 0\n\tendm\n\
         \tdc.b DEFINED(V)\n\tdc.b DEFINED(E)\n\tdc.b DEFINED(MOMCPU)\n\
         \tdc.b DEFINED(TRUE)\n\tdc.b DEFINED(Mac)\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x01, 0x01, 0x01, 0x01, 0x00]);
}

/// The argument is read as a NAME, never evaluated, so anything that is not a
/// bare name is 0 rather than an error. asl, exit 0 on all three, with `E equ 2`
/// in scope: `DEFINED((E))` is `00` (probe `dd.asm`), `DEFINED(1)` is `00`
/// (probe `de.asm`), `DEFINED(E+1)` is `00` (probe `dg.asm`).
#[test]
fn a_non_name_argument_is_zero_and_not_an_error() {
    let src = format!(
        "{HEAD}E\tequ 2\n\tdc.b DEFINED((E))\n\tdc.b DEFINED(1)\n\
         \tdc.b DEFINED(E+1)\n\tdc.b DEFINED(E.x)\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x00, 0x00, 0x00, 0x00]);
}

/// The verdict is an ordinary integer and composes with every operator. asl,
/// probe `dc.asm`, exit 0: `dc.b DEFINED(E)+DEFINED(Nowhere)*4` is `01` and
/// `dc.b 1-DEFINED(E)` is `00`.
#[test]
fn the_verdict_composes_in_arithmetic() {
    let src = format!(
        "{HEAD}E\tequ 2\n\tdc.b DEFINED(E)+DEFINED(Nowhere)*4\n\
         \tdc.b 1-DEFINED(E)\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x01, 0x00]);
}

/// Only the CALL shape is the builtin. A bare `DEFINED` with no parenthesis is
/// an ordinary symbol reference, and a program may define one. asl, probe
/// `df.asm`, exit 0: `DEFINED equ 2` then `dc.b DEFINED` is `02`.
#[test]
fn a_bare_defined_is_an_ordinary_symbol() {
    let src = format!("{HEAD}DEFINED\tequ 2\n\tdc.b DEFINED\n\tend\n");
    assert_eq!(bytes(&src), vec![0x02]);
}

/// A USER `function` of that name WINS over the builtin, which is measured
/// rather than chosen. asl, probe `dh.asm`, exit 0, `1 pass`, `0 errors`:
///
/// ```text
///        4/       0 :                    DEFINED function x,x+100
///        5/       0 : 66                          dc.b    DEFINED(E)
/// ```
///
/// `$66` is 102, which is `E`'s 2 plus the function's 100, so the call went to
/// the user's function and not to the predicate.
#[test]
fn a_user_function_of_that_name_wins() {
    let src = format!(
        "{HEAD}E\tequ 2\nDEFINED\tfunction x,x+100\n\tdc.b DEFINED(E)\n\tend\n"
    );
    assert_eq!(bytes(&src), vec![0x66]);
}
