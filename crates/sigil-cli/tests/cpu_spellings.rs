//! A `cpu` directive's processor name is accepted only when sigil encodes the
//! instruction set it names, and every accepted spelling lands on the target the
//! table says it does.
//!
//! ## The two failure modes this file stands on
//!
//! **A spelling silently aliased onto the wrong target.** A processor name that
//! begins with a digit reaches the lexer as an integer literal, and reading
//! every numeric name as one fixed processor makes `cpu 6502` select the 68000,
//! emit bytes and exit 0. That is the same silent-wrong-target class as an
//! undeclared unit (`cpu_undeclared.rs`) and a mis-cased directive, reached
//! through the third door: a name nobody checked. Nothing downstream can catch
//! it — the bytes are a valid program for a processor the source never asked
//! for.
//!
//! **A spelling refused that names an instruction set sigil DOES encode.** The
//! public Sonic 2 disassembly's sound driver declares `CPU Z80UNDOC` — the Z80
//! with its undocumented instructions enabled — as a mid-unit switch under
//! `save`/`restore` from a 68000 root. Refusing it left the one file in that
//! corpus genuinely asking to change processor assembling as a 68000, and left
//! every symbol it defines undefined for the game code that references them.
//!
//! ## Why accepting a SUPERSET is safe here, and what keeps it safe
//!
//! `z80undoc` is not the same instruction set as `z80`; it is strictly wider.
//! It is safe to alias only because sigil refuses the extra instructions BY
//! NAME rather than dropping or mis-encoding them, so the alias widens where
//! the refusal is reported and never what assembles.
//!
//! That is a property of the Z80 lowering, not of the `cpu` directive, and
//! nothing else in the suite asserts it. If a later change made an unknown
//! mnemonic a skip or a fallback, the alias would silently become the defect it
//! was cleared of — so the property is gated here, next to the decision that
//! rests on it.
//!
//! ## Why these gates drive the CLI process
//!
//! The shipped `sigil <file.asm>` command is the caller whose input is a foreign
//! tree, and therefore the one that meets an unknown processor name at all. Its
//! output binary is also the only place "assembled clean as the wrong
//! processor" is visible as a fact rather than an inference.

use sigil_frontend_as::{cpu_for_spelling, unsupported_cpu, CPU_SPELLINGS};
use sigil_ir::Cpu;
use std::path::Path;
use std::process::Command;

/// What one assembly of a source produced: whether the process succeeded, its
/// stderr, and the output bytes if it wrote any.
struct Run {
    ok: bool,
    stderr: String,
    bytes: Option<Vec<u8>>,
}

/// Assemble `src` through the shipped command, in its own directory.
fn assemble(src: &str) -> Run {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("root.asm");
    let out = dir.path().join("out.bin");
    std::fs::write(&root, src).expect("write root.asm");

    let res = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args([root.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .output()
        .expect("spawn sigil");

    Run {
        ok: res.status.success(),
        stderr: String::from_utf8_lossy(&res.stderr).into_owned(),
        bytes: read_if_present(&out),
    }
}

fn read_if_present(p: &Path) -> Option<Vec<u8>> {
    std::fs::read(p).ok()
}

/// A source body this target can assemble. Kept separate from the spelling so a
/// test can vary the `cpu` line alone and attribute any difference to it.
fn body_for(cpu: Cpu) -> &'static str {
    match cpu {
        Cpu::M68000 => "\tmove.w\t#$1234,d0\n",
        Cpu::Z80 => "\tld\ta,5\n\tld\tbc,1234h\n",
    }
}

/// THE SILENT ONE. A numeric processor name sigil does not encode must be
/// refused, not read as the nearest numeric processor it does.
///
/// `68020` is the sharp case: it is a real Motorola part whose instruction set
/// is a superset of the 68000's, so a source written for it assembles *almost*
/// correctly as a 68000 — right up to the first instruction that only exists on
/// the 68020, which is reported as an unknown mnemonic on a target the source
/// never named. `6502` is the same hole with no relation between the two
/// processors at all.
#[test]
fn a_numeric_spelling_sigil_does_not_encode_is_refused() {
    for name in ["68020", "6502"] {
        // The precondition, derived rather than assumed: the table genuinely
        // does not carry this spelling. If a later parcel implements one of
        // these, this fails loudly instead of asserting on nothing.
        assert!(
            cpu_for_spelling(name).is_none(),
            "precondition: `{name}` is a spelling this front end does not encode"
        );

        let run = assemble(&format!("\tcpu {name}\n{}", body_for(Cpu::M68000)));
        assert!(
            !run.ok,
            "`cpu {name}` must be REFUSED. Assembling it as some other processor \
             emits a valid program for a machine the source never asked for, and \
             nothing downstream can tell. stderr:\n{}",
            run.stderr
        );
        assert!(
            run.stderr.contains(&format!("`{name}`")),
            "the refusal must name the processor it refused. stderr:\n{}",
            run.stderr
        );
        assert!(
            run.bytes.is_none(),
            "a refused processor must produce no output binary — bytes on disk \
             are the whole damage of this class"
        );
    }
}

/// The control that makes the test above about the NAME rather than about the
/// body: the identical source under a spelling the table does carry assembles
/// and emits.
#[test]
fn the_same_source_under_an_accepted_spelling_assembles() {
    let run = assemble(&format!("\tcpu 68000\n{}", body_for(Cpu::M68000)));
    assert!(run.ok, "the control source must assemble. stderr:\n{}", run.stderr);
    assert_eq!(
        run.bytes.as_deref(),
        Some([0x30, 0x3C, 0x12, 0x34].as_slice()),
        "the control must emit the 68000 encoding of `move.w #$1234,d0`"
    );
}

/// Every spelling in the table lands on the target the table names it for, and
/// on no other.
///
/// Byte-identity against the first spelling recorded for that target proves an
/// alias is the SAME processor rather than merely an accepted word — that is
/// what makes `68008` and `z80undoc` aliases and not new targets. The other
/// target's body must be REFUSED under this spelling: `move.w` does not assemble
/// on a Z80 and `ld` is not a 68000 mnemonic, so the pair of outcomes says which
/// processor was actually selected.
///
/// **What this cannot check, and what does.** `directive_cpu` resolves through
/// the same table these expectations are derived from, so a row pointed at the
/// WRONG target is self-consistent here: the body chosen for the claimed target
/// is exactly the body that claim makes assemble. A table cannot audit itself.
/// The gates that pin `z80undoc` to the Z80 independently of the table are the
/// two whose expectations come from the Z80 ISA —
/// `the_undocumented_forms_z80undoc_adds_are_refused_and_emit_nothing`'s control
/// and `a_mid_unit_switch_to_z80undoc_assembles_both_halves`. What this test
/// catches is the table and the lowering DISAGREEING: a spelling special-cased
/// somewhere on the way to a target other than the one its row names.
#[test]
fn every_accepted_spelling_selects_the_target_the_table_names() {
    assert!(
        !CPU_SPELLINGS.is_empty(),
        "precondition: the table has rows to check"
    );
    let mut targets: Vec<Cpu> = Vec::new();
    for (_, cpu) in CPU_SPELLINGS {
        if !targets.contains(cpu) {
            targets.push(*cpu);
        }
    }
    assert!(
        targets.len() >= 2,
        "this gate distinguishes targets by refusing the OTHER target's body; \
         with fewer than two targets in the table there is nothing to \
         distinguish and the assertions below prove nothing. Targets: {targets:?}"
    );

    for (spelling, cpu) in CPU_SPELLINGS {
        let canonical = CPU_SPELLINGS
            .iter()
            .find(|(_, c)| c == cpu)
            .map(|(s, _)| *s)
            .expect("the target's own spelling is in the table it came from");

        let body = body_for(*cpu);
        let under_spelling = assemble(&format!("\tcpu {spelling}\n{body}"));
        let under_canonical = assemble(&format!("\tcpu {canonical}\n{body}"));

        assert!(
            under_spelling.ok,
            "`cpu {spelling}` is in the table and must assemble. stderr:\n{}",
            under_spelling.stderr
        );
        assert!(
            under_canonical.bytes.is_some(),
            "the canonical spelling `{canonical}` must emit bytes for this \
             comparison to mean anything. stderr:\n{}",
            under_canonical.stderr
        );
        assert_eq!(
            under_spelling.bytes, under_canonical.bytes,
            "`cpu {spelling}` must encode exactly as `cpu {canonical}` — the \
             table says they are the same target, and a spelling that assembles \
             to different bytes is a different processor wearing an alias"
        );

        for other in &targets {
            if other == cpu {
                continue;
            }
            let wrong = assemble(&format!("\tcpu {spelling}\n{}", body_for(*other)));
            assert!(
                !wrong.ok,
                "`cpu {spelling}` assembled a {other:?} body. The table says it \
                 selects {cpu:?}; a spelling that accepts another processor's \
                 instructions is not pointing where the table says. stderr:\n{}",
                wrong.stderr
            );
        }
    }
}

/// The refusal names the fault and prints the remedy, and the remedy is the
/// table — not a transcription of it that can drift out of date.
///
/// Read from the refusal's OWN line, not from stderr as a whole. A unit whose
/// only `cpu` line was refused has also declared nothing, so `CPU_UNDECLARED`
/// is reported alongside — and that text names `cpu 68000` and `cpu z80` itself.
/// Searching the whole stream would find two of the four spellings no matter
/// what this refusal said.
#[test]
fn the_refusal_lists_every_accepted_line() {
    let run = assemble(&format!("\tcpu z180\n{}", body_for(Cpu::Z80)));
    assert!(!run.ok, "precondition: `z180` is refused. stderr:\n{}", run.stderr);

    let line = run
        .stderr
        .lines()
        .find(|l| l.contains("unsupported processor"))
        .unwrap_or_else(|| panic!("no refusal line for `z180`. stderr:\n{}", run.stderr));

    for (spelling, _) in CPU_SPELLINGS {
        assert!(
            line.contains(&format!("`cpu {spelling}`")),
            "the refusal must print `cpu {spelling}` as a line the reader can \
             write — every accepted spelling, listed from the table. \
             refusal:\n{line}"
        );
    }
    assert!(
        line.contains(&unsupported_cpu("z180")),
        "the shipped refusal text must be the one the constructor builds. \
         refusal:\n{line}"
    );
}

/// THE PROPERTY THE `z80undoc` ARM RESTS ON. The undocumented instructions
/// sigil does not encode are refused by name and write no bytes.
///
/// Each row below is a form AS accepts under `Z80UNDOC` and sigil does not
/// implement. What is asserted is not the wording of any one diagnostic — that
/// is free to improve — but that the run FAILS and leaves no binary. A skip, a
/// warning, or a fallback encoding would make accepting the wider processor
/// exactly the silent-wrong-output defect it was cleared of.
#[test]
fn the_undocumented_forms_z80undoc_adds_are_refused_and_emit_nothing() {
    // The corpus's own uses are the last three: `s2.sounddriver.asm` reads and
    // writes the index registers' halves, which is what it declares Z80UNDOC
    // for in the first place.
    let undocumented = [
        "\tsll\ta\n",         // undocumented shift, no mnemonic for it here
        "\trlc\t(ix+1),b\n",  // undocumented CB form with a result register
        "\tld\ta,iyl\n",      // index-register half as a source
        "\tadd\ta,ixl\n",
        "\tadc\ta,ixu\n",
    ];
    for form in undocumented {
        let run = assemble(&format!("\tcpu z80undoc\n{form}"));
        assert!(
            !run.ok,
            "sigil does not encode `{}` — under `cpu z80undoc` it must be \
             REFUSED, never dropped or assembled as something else. Accepting \
             the wider processor is sound only while this holds. stderr:\n{}",
            form.trim(),
            run.stderr
        );
        assert!(
            run.bytes.is_none(),
            "`{}` produced an output binary. Whatever bytes those are, they are \
             not this instruction.",
            form.trim()
        );
    }

    // The control: the documented Z80 subset still assembles under the same
    // spelling, so the assertions above are about the undocumented forms and
    // not about `z80undoc` failing wholesale.
    let ok = assemble(&format!("\tcpu z80undoc\n{}", body_for(Cpu::Z80)));
    assert!(
        ok.ok,
        "the documented subset must still assemble under `cpu z80undoc`. \
         stderr:\n{}",
        ok.stderr
    );
}

/// The corpus's real shape: a 68000 root that switches to the undocumented Z80
/// mid-unit under `save`/`restore`, assembles the Z80 body, and comes back.
///
/// `s2.sounddriver.asm` is not a separate assembly unit — `s2.asm:90859`
/// `include`s it between `save` and `restore`, so its `CPU Z80UNDOC` at line 250
/// is a switch inside an already-declared 68000 unit, not a declaration of its
/// own. The bytes assert that both halves encoded on their own processor.
#[test]
fn a_mid_unit_switch_to_z80undoc_assembles_both_halves() {
    let run = assemble(
        "\tcpu 68000\n\
         \tmove.w\t#$1234,d0\n\
         \tsave\n\
         \tcpu z80undoc\n\
         \tld\ta,5\n\
         \trestore\n\
         \tmove.w\t#$5678,d1\n",
    );
    assert!(run.ok, "the mid-unit switch must assemble. stderr:\n{}", run.stderr);
    assert_eq!(
        run.bytes.as_deref(),
        Some([0x30, 0x3C, 0x12, 0x34, 0x3E, 0x05, 0x32, 0x3C, 0x56, 0x78].as_slice()),
        "each half must be encoded on its own processor: 68000 `move.w` \
         immediates around a Z80 `ld a,5` (3E 05)"
    );
}
