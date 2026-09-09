//! A `.emp` `cpu:` attribute naming a processor the front end does not
//! recognize is REFUSED by name, never coerced to a default.
//!
//! ## What was silently wrong
//!
//! `attr_cpu` selected the Z80 when the value's last path segment folded to
//! `z80` and returned the 68000 for **everything else**. It took no diagnostics
//! sink, so it structurally could not report. A typo, or any name we do not
//! know (`6502`, `Z81`, `m6800`), became a 68000 in silence.
//!
//! The consequence is the worst shape this class takes. On a section its author
//! meant for the Z80, the tool then rejects that section's own correct Z80
//! instructions as unrecognized 68000 mnemonics. It speaks, and it points
//! *away* from the cause: the repair it implies to the reader is rewriting
//! working code.
//!
//! ## The half this is, and the half it is not
//!
//! `cpu_undeclared.rs` refuses an assembly unit that declares NO processor.
//! This file refuses one that declares a processor we do not recognize. Both
//! are "the assembler picked a processor for you and said nothing", reached
//! through different doors, and both refuse rather than warn for the reason
//! stated there: a run that reports what it skipped still exits 0.
//!
//! ## Why the accepted set is `m68000` / `m68k` / `z80`
//!
//! Those are exactly the spellings the corpus writes today (aeon's `*.emp`
//! carry `z80` and `m68000` and nothing else; this tree adds `m68k`, in
//! `examples/main.emp` and two design docs). Grandfathering the observed set
//! makes the refusal provably byte-neutral over every source that exists and
//! leaves it refusing only genuine typos. Narrowing to one canonical spelling
//! per processor is a language-surface question for the owner, not one this
//! gate settles.
//!
//! The set is deliberately NOT `sigil_frontend_as::CPU_SPELLINGS`. AS accepts
//! `68000` and `68008`, which the `.emp` grammar cannot even deliver here: a
//! bare-numeric name lexes as an integer literal rather than a path.
//!
//! ## Why the two call sites are gated separately
//!
//! `attr_cpu` is read at a section head and at a module head. A property proven
//! at one consumer is not a property of the other, and the whole defect this
//! file closes is one function's behaviour reaching two places unchecked.

use sigil_frontend_emp::lower::{
    cpu_for_spelling, lower_module, LowerOptions, CPU_SPELLINGS,
};
use sigil_frontend_emp::parse_str;
use sigil_harness::test_support::{aeon_dir, NO_REFERENCE_TREE};
use sigil_ir::backend::Cpu;
use sigil_span::Level;
use std::process::Command;

/// Lower one `.emp` source through the real seam and return its module and
/// every diagnostic. This is the path both `cpu:` call sites sit on, so a
/// spelling that resolves here resolves in a build.
fn lower(src: &str) -> (sigil_ir::Module, Vec<sigil_span::Diagnostic>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.is_empty(), "the fixture must PARSE, so the outcome is about the `cpu:` value: {perrs:?}");
    lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    )
}

/// A one-section source whose section head names `spelling` as its processor.
fn section_head_src(spelling: &str) -> String {
    format!("module m\nsection s (cpu: {spelling}, vma: $0) {{\n  data V: u16 = $1111\n}}\n")
}

/// A source whose MODULE head names `spelling` as its processor. The second
/// call site, and the one an author reaches when the whole file is for one
/// processor.
fn module_head_src(spelling: &str) -> String {
    format!("module m (cpu: {spelling})\n")
}

/// The refusals in a diagnostic list, as their messages.
fn refusals(diags: &[sigil_span::Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.level == Level::Error && d.message.starts_with("unrecognized processor"))
        .map(|d| d.message.clone())
        .collect()
}

// ---- the refusal, at each of the two call sites -----------------------------

/// THE SILENT ONE, at the SECTION head. A processor name the table does not
/// carry must be refused, and the refusal must name the value and list what to
/// write instead.
#[test]
fn an_unrecognized_spelling_at_a_section_head_is_refused_by_name() {
    for name in ["banana", "z81", "m6800", "68020", "gbz80"] {
        // Derived precondition: the table genuinely does not carry this. If a
        // later parcel adds one, this fails loudly rather than asserting on
        // nothing.
        assert!(
            cpu_for_spelling(name).is_none(),
            "precondition: `{name}` is a spelling this front end does not recognize"
        );

        let (_, diags) = lower(&section_head_src(name));
        let found = refusals(&diags);
        assert_eq!(
            found.len(),
            1,
            "`cpu: {name}` must be refused exactly once at a section head. \
             Defaulting it to a processor nobody named then reports that \
             section's own correct instructions as unknown mnemonics. \
             diagnostics: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            found[0].contains(&format!("`{name}`")),
            "the refusal must name the value it refused: {}",
            found[0]
        );
        for (spelling, _) in CPU_SPELLINGS {
            assert!(
                found[0].contains(&format!("`cpu: {spelling}`")),
                "the refusal must print `cpu: {spelling}` as a line the reader \
                 can write, every accepted spelling, listed from the table so \
                 it cannot advertise what the attribute does not accept: {}",
                found[0]
            );
        }
    }
}

/// THE SECOND CONSUMER. The same value at the MODULE head is refused too.
///
/// `module_declared_cpu` is a separate function reading the same helper. A
/// refusal wired into the section path alone would leave a whole file's
/// declared processor silently defaulting, which is the larger blast radius of
/// the two.
#[test]
fn an_unrecognized_spelling_at_a_module_head_is_refused_by_name() {
    for name in ["banana", "z81", "m6800"] {
        assert!(
            cpu_for_spelling(name).is_none(),
            "precondition: `{name}` is a spelling this front end does not recognize"
        );

        let (_, diags) = lower(&module_head_src(name));
        let found = refusals(&diags);
        assert_eq!(
            found.len(),
            1,
            "`module m (cpu: {name})` must be refused. The module head is the \
             second reader of this value and nothing else gates it. \
             diagnostics: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            found[0].contains(&format!("`{name}`")),
            "the refusal must name the value it refused: {}",
            found[0]
        );
    }
}

/// The refusal points at the VALUE, not at the section head.
///
/// A span covering the whole declaration renders a `path:line:col:` that sends
/// the reader to the line but not to the word, and the word is the entire
/// content of this diagnostic.
#[test]
fn the_refusal_points_at_the_offending_value() {
    let src = section_head_src("banana");
    let (_, diags) = lower(&src);
    let d = diags
        .iter()
        .find(|d| d.message.starts_with("unrecognized processor"))
        .expect("the refusal");

    let start = d.primary.start as usize;
    let end = d.primary.end as usize;
    assert!(
        end <= src.len() && start < end,
        "the span must be a real range of the source: {:?}",
        d.primary
    );
    assert_eq!(
        &src[start..end],
        "banana",
        "the span must cover the processor name itself. Source:\n{src}"
    );
}

/// A bare-numeric name reaches this attribute as an INTEGER literal rather than
/// a path, so the shape that resolved spellings never took is exactly the shape
/// the old default swallowed most quietly. It is refused, and named by its
/// digits rather than by its AST shape.
#[test]
fn a_bare_numeric_processor_name_is_refused_and_named() {
    let (_, diags) = lower(&section_head_src("68000"));
    let found = refusals(&diags);
    assert_eq!(
        found.len(),
        1,
        "`cpu: 68000` must be refused. It is AS's spelling, not this front \
         end's, and reading it as the nearest processor is how a source \
         assembles as something it never asked for. diagnostics: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(
        found[0].contains("`68000`"),
        "the refusal must name the digits the author wrote: {}",
        found[0]
    );
}

// ---- what is accepted, and that it lands where the table says ---------------

/// Every spelling in the table is accepted at BOTH call sites and selects the
/// target the table names, in any case.
///
/// The case arm is not decoration: the old resolver folded case, and dropping
/// that while grandfathering the set would refuse sources that assemble today.
#[test]
fn every_accepted_spelling_selects_the_target_the_table_names() {
    assert!(!CPU_SPELLINGS.is_empty(), "precondition: the table has rows to check");
    let mut targets: Vec<Cpu> = Vec::new();
    for (_, cpu) in CPU_SPELLINGS {
        if !targets.contains(cpu) {
            targets.push(*cpu);
        }
    }
    assert!(
        targets.len() >= 2,
        "this gate distinguishes targets; with fewer than two in the table \
         there is nothing to distinguish. Targets: {targets:?}"
    );

    for (spelling, cpu) in CPU_SPELLINGS {
        for written in [spelling.to_string(), spelling.to_uppercase()] {
            let (module, diags) = lower(&section_head_src(&written));
            assert!(
                refusals(&diags).is_empty(),
                "`cpu: {written}` is in the table and must be accepted. \
                 diagnostics: {:?}",
                diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
            let s = module
                .sections
                .iter()
                .find(|s| s.name == "s")
                .expect("the fixture's section `s`");
            assert_eq!(
                s.cpu, *cpu,
                "`cpu: {written}` must select the target its row names. A row \
                 the lowering disagrees with is a spelling special-cased on its \
                 way somewhere else."
            );
        }
    }
}

/// A multi-segment path is not a processor name.
///
/// The old resolver read only the LAST segment, so `foo.z80` selected the Z80
/// and `foo.bar` selected the 68000 by default. Neither spelling exists in any
/// corpus, and accepting a dotted path here would mean a module path could
/// silently name a processor.
#[test]
fn a_dotted_path_is_not_a_processor_name() {
    for name in ["foo.z80", "a.b.m68000"] {
        let (_, diags) = lower(&section_head_src(name));
        let found = refusals(&diags);
        assert_eq!(
            found.len(),
            1,
            "`cpu: {name}` must be refused: reading only the last segment made \
             a path's tail decide the processor. diagnostics: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            found[0].contains(&format!("`{name}`")),
            "the refusal must echo the whole path the author wrote: {}",
            found[0]
        );
    }
}

/// A diagnostic that NAMES a processor must name one the reader can write.
///
/// `[module.cpu-mismatch]` renders both processors through a separate
/// `Cpu -> &str` helper, so the table is not the only place a spelling is
/// produced. Narrow the table without touching that helper and the message
/// starts telling people to write a `cpu:` value the attribute would refuse,
/// which is the drift the one-table design exists to prevent, arriving through
/// the one direction the table does not resolve.
#[test]
fn a_diagnostic_that_names_a_processor_names_an_acceptable_spelling() {
    // A module declared Z80 opening a 68000 section: the mismatch names both.
    let src = "module m (cpu: z80)\nsection s (cpu: m68000, vma: $0) {\n  data V: u16 = $1111\n}\n";
    let (_, diags) = lower(src);
    let msg = diags
        .iter()
        .find(|d| d.message.contains("[module.cpu-mismatch]"))
        .map(|d| d.message.clone())
        .unwrap_or_else(|| {
            panic!(
                "this gate needs the mismatch diagnostic to exist; without it \
                 it asserts on nothing. diagnostics: {:?}",
                diags.iter().map(|d| &d.message).collect::<Vec<_>>()
            )
        });

    // BOTH sides. The message renders the module's processor and the section's
    // through the same helper, and only one of the two is written `(cpu: X)`,
    // so scanning for that spelling alone would check half of it.
    for target in [Cpu::Z80, Cpu::M68000] {
        let accepted: Vec<&str> = CPU_SPELLINGS
            .iter()
            .filter(|(_, c)| *c == target)
            .map(|(s, _)| *s)
            .collect();
        assert!(
            !accepted.is_empty(),
            "precondition: the table names {target:?} at all"
        );
        assert!(
            accepted.iter().any(|s| msg.contains(*s)),
            "the mismatch message names the {target:?} side with something that \
             is not a spelling the attribute accepts, so it is telling the \
             reader to write a value that would be refused. Accepted for \
             {target:?}: {accepted:?}. message: {msg}"
        );
    }
}

// ---- the promise made to aeon -----------------------------------------------

/// THE GATE ON THE PROMISE, DERIVED. Every distinct `cpu:` spelling written in
/// the aeon tree must be accepted here.
///
/// The failure this stands in front of is total rather than partial: aeon's
/// nine `m68000` sites and twenty-four `z80` sites are the whole of its `.emp`
/// processor declarations, so a table that dropped either spelling stops every
/// aeon build at once. Deriving the expectation from that tree at run time
/// means a spelling aeon adds and we do not carry goes red HERE, at the end
/// that can fix it, rather than in their build.
///
/// Asserted through `lower_module`, the seam both call sites sit on, not
/// through a helper written for this test: a spelling accepted by a
/// test-local comparison and refused by the real one proves nothing.
#[test]
fn every_cpu_spelling_written_in_the_aeon_tree_is_accepted() {
    let dir = aeon_dir();
    if dir == std::path::Path::new(NO_REFERENCE_TREE) {
        eprintln!(
            "skip: no reference tree named, this gate's DERIVED half cannot \
             measure. `the_spellings_aeon_writes_today_are_accepted` covers \
             the same promise without one."
        );
        return;
    }

    let Census { spellings, files } = cpu_spellings_written_under(&dir);
    // What was measured, on the record. A census that reports only its verdict
    // cannot be checked for over-reach, which is this one's failure mode.
    eprintln!(
        "cpu census: {} `.emp` files under {}, spellings {spellings:?}",
        files,
        dir.display()
    );
    assert!(
        files > 0 && !spellings.is_empty(),
        "the census found {files} `.emp` files and no `cpu:` value under {}. An \
         empty result here is a BROKEN INSTRUMENT, not a clean tree: aeon \
         declares processors in its `.emp` sources and always has. Refusing to \
         read that as a pass.",
        dir.display()
    );

    for spelling in &spellings {
        assert!(
            cpu_for_spelling(spelling).is_some(),
            "aeon writes `cpu: {spelling}` and this front end does not accept \
             it. Every site using it stops assembling. Spellings found under \
             {}: {spellings:?}",
            dir.display()
        );
        let (_, diags) = lower(&section_head_src(spelling));
        assert!(
            refusals(&diags).is_empty(),
            "`cpu: {spelling}` is written in aeon and must lower. Accepting it \
             in the table while the real seam refuses it is the same outage. \
             diagnostics: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// THE SAME PROMISE, WITHOUT THE TREE. `m68000` and `z80` are the two spellings
/// aeon's sources carry, and this gate cannot skip.
///
/// The derived gate above is the one that catches aeon ADDING a spelling. This
/// one catches us DROPPING one, and it holds on a machine with no reference
/// tree, where the derived gate has nothing to read. Its literals are the cost
/// of never rendering "could not measure" as a pass.
#[test]
fn the_spellings_aeon_writes_today_are_accepted() {
    for (spelling, expected) in [("m68000", Cpu::M68000), ("z80", Cpu::Z80)] {
        let (module, diags) = lower(&section_head_src(spelling));
        assert!(
            refusals(&diags).is_empty(),
            "aeon's sources declare `cpu: {spelling}`. Refusing it stops every \
             build that uses it, and that is all of them for this processor. \
             diagnostics: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        let s = module.sections.iter().find(|s| s.name == "s").expect("section `s`");
        assert_eq!(
            s.cpu, expected,
            "`cpu: {spelling}` must still select {expected:?}, accepting the \
             word while lowering it as the other processor is the defect this \
             file closes wearing an accepted spelling"
        );
    }
}

/// Subtrees the census does not descend into. Each would attribute somebody
/// else's sources to the reference tree.
///
/// `.claude/worktrees` and `.worktrees` are the sharp ones and they are not
/// hypothetical: both hold SYMLINKS INTO OTHER REPOSITORIES (this one included),
/// so a walk that follows them censuses sigil's own `examples/` and reports the
/// result as aeon's. That is how this gate first passed, with a spelling in its
/// census that appears nowhere in aeon.
const CENSUS_SKIPS: &[&str] = &[".git", ".claude", ".worktrees", "target"];

/// The census population: every `*.emp` file under `root`, not following
/// symlinks and not descending into [`CENSUS_SKIPS`].
///
/// Returned alongside the spellings so a reader can see WHAT was measured. A
/// census is only as good as its population, and this one's failure mode is
/// silent over-reach rather than emptiness.
struct Census {
    spellings: Vec<String>,
    files: usize,
}

/// Every distinct `cpu:` value written in `*.emp` under `root`, folded.
///
/// Reads the attribute out of the source text rather than parsing: the census
/// must see a spelling this front end may not be able to lower, which is the
/// case the derived gate exists to catch.
///
/// Symlinks are not followed, in either direction. `entry.file_type()` reports
/// the LINK rather than its target, unlike `Path::is_dir`, so a link out of the
/// tree is neither descended into nor read.
fn cpu_spellings_written_under(root: &std::path::Path) -> Census {
    let mut spellings: Vec<String> = Vec::new();
    let mut files = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() {
                continue;
            }
            let path = entry.path();
            if kind.is_dir() {
                if path.file_name().is_some_and(|n| CENSUS_SKIPS.iter().any(|s| n == *s)) {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "emp") {
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                files += 1;
                for spelling in cpu_values_in(&text) {
                    if !spellings.contains(&spelling) {
                        spellings.push(spelling);
                    }
                }
            }
        }
    }
    spellings.sort();
    Census { spellings, files }
}

/// The `cpu:` attribute values in one `.emp` source's text.
fn cpu_values_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in text.match_indices("cpu:") {
        let rest = text[i + "cpu:".len()..].trim_start();
        let value: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !value.is_empty() {
            out.push(value.to_ascii_lowercase());
        }
    }
    out
}

// ---- the shipped command ----------------------------------------------------

/// THE PROCESS. The shipped `sigil emp` command refuses the file and writes no
/// output.
///
/// A diagnostic in a `Vec` is not a refusal. What decides whether wrong bytes
/// reach a disk is the exit status of the command a build script runs, and
/// that is the caller a library-level test cannot speak for.
#[test]
fn the_shipped_command_refuses_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("m.emp");
    let out = dir.path().join("out.bin");
    std::fs::write(&src, section_head_src("banana")).expect("write m.emp");

    let res = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", src.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .output()
        .expect("spawn sigil");

    let stderr = String::from_utf8_lossy(&res.stderr);
    let stdout = String::from_utf8_lossy(&res.stdout);
    assert!(
        !res.status.success(),
        "an unrecognized processor must FAIL the command, not warn. A run that \
         reports what it defaulted still exits 0. status: {:?}\nstderr:\n{stderr}\nstdout:\n{stdout}",
        res.status
    );
    assert!(
        stderr.contains("unrecognized processor `banana`") || stdout.contains("unrecognized processor `banana`"),
        "the command must report the refusal it failed on.\nstderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(
        !out.exists(),
        "a refused file must produce no output binary, bytes on disk are the \
         whole damage of this class"
    );
}

/// THE CONTROL, and the byte-neutrality claim as a process fact. The identical
/// fixture under an accepted spelling assembles and writes its bytes, so the
/// refusal above is attributable to the NAME and not to the fixture.
#[test]
fn the_same_fixture_under_an_accepted_spelling_assembles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("m.emp");
    let out = dir.path().join("out.bin");
    std::fs::write(&src, section_head_src("m68k")).expect("write m.emp");

    let res = Command::new(env!("CARGO_BIN_EXE_sigil"))
        .args(["emp", src.to_str().unwrap(), "-o", out.to_str().unwrap()])
        .output()
        .expect("spawn sigil");

    assert!(
        res.status.success(),
        "the control must assemble. stderr:\n{}",
        String::from_utf8_lossy(&res.stderr)
    );
    assert_eq!(
        std::fs::read(&out).expect("the control's output binary"),
        vec![0x11, 0x11],
        "the control must emit its 68000 big-endian u16"
    );
}

/// `examples/main.emp` is the newcomer's first-contact file and it declares
/// `cpu: m68k` five times. Every one of its processor declarations still
/// resolves, to the target its spelling names.
///
/// This is the concrete cost of narrowing the accepted set without asking, and
/// gating it means a later narrowing meets this file rather than discovers it.
///
/// Asserted at the lowering seam rather than through `sigil emp
/// examples/main.emp --root examples --hex`. That command does not succeed on
/// this tree for reasons that have nothing to do with processors
/// (`examples/game/badniks/pitcher_plant.emp` names types and prelude symbols
/// that do not resolve), and a gate whose green depends on unrelated example
/// code going green is a red waiting to be silenced rather than read.
///
/// What keeps it from passing vacuously is the second half: the section
/// targets are checked, so the file must have lowered far enough for its
/// attributes to resolve at all.
#[test]
fn the_first_contact_examples_processor_declarations_all_resolve() {
    let main = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("the workspace root")
        .join("examples")
        .join("main.emp");
    let text = std::fs::read_to_string(&main)
        .unwrap_or_else(|e| panic!("read {}: {e}", main.display()));

    // The precondition that makes this gate about `m68k` rather than about
    // nothing: if the spelling is ever cleaned out of the file, this fails
    // loudly instead of passing for the wrong reason.
    let m68k_sites = text.matches("cpu: m68k").count();
    assert!(
        m68k_sites >= 5,
        "precondition: {} declares `cpu: m68k` (found {m68k_sites} sites)",
        main.display()
    );

    let (file, perrs) = parse_str(&text);
    assert!(perrs.is_empty(), "the example must parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    assert!(
        refusals(&diags).is_empty(),
        "the newcomer's first-contact file must not meet this refusal. \
         Refusing `m68k` breaks it on the first command anyone types. \
         diagnostics: {:?}",
        refusals(&diags)
    );

    // Non-vacuity, and the property itself: each declared section carries the
    // target its spelling names.
    for (name, expected) in [
        ("vectors", Cpu::M68000),
        ("header", Cpu::M68000),
        ("engine", Cpu::M68000),
        ("obj_bank", Cpu::M68000),
        ("data", Cpu::M68000),
        ("z80drv", Cpu::Z80),
    ] {
        let s = module
            .sections
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "section `{name}` did not lower, so this gate would have \
                     passed without measuring anything. Sections: {:?}",
                    module.sections.iter().map(|s| &s.name).collect::<Vec<_>>()
                )
            });
        assert_eq!(s.cpu, expected, "section `{name}` must select {expected:?}");
    }
}
