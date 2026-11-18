//! The span-to-source seam the build export reads (G0a Task 4): `ChainSources` behind
//! `SpanLocator`. A line is the OUTERMOST line a person wrote, so the bytes of a macro
//! body locate to the invocation, never to the body line inside the macro.

use sigil_frontend_emp::resolve::manifest::{Manifest, SourceIndex};
use sigil_harness::native::ChainSources;
use sigil_ir::{Fragment, Section};
use sigil_link::export::locate::{SourcePos, SpanLocator};
use sigil_span::{SourceId, Span};
use std::collections::HashMap;

const EMP_TEXT: &str = "module m\n\nproc P () {\n}\n";

fn line_of(text: &str, needle: &str) -> u32 {
    let at = text.find(needle).unwrap_or_else(|| panic!("fixture lacks `{needle}`"));
    text[..at].matches('\n').count() as u32 + 1
}

fn frag_span(f: &Fragment) -> Span {
    match f {
        Fragment::Data(d) => d.span,
        other => panic!("fixture emits only data, got {other:?}"),
    }
}

fn emp_index(dir: &std::path::Path, names: &[&str]) -> SourceIndex {
    let mut sources = HashMap::new();
    for (id, name) in names.iter().enumerate() {
        let p = dir.join(name);
        std::fs::write(&p, EMP_TEXT).unwrap();
        sources.insert(SourceId(id as u32), p);
    }
    SourceIndex::new(&Manifest { modules: Vec::new(), by_id: HashMap::new(), sources })
}

#[test]
fn locator_resolves_an_as_macro_body_to_its_call_line() {
    let dir = tempfile::tempdir().unwrap();
    let src = "\tcpu 68000\nmymac\tmacro\n\tdc.w\t$1111\n\tdc.w\t$2222\n\tendm\n\tdc.w\t$0000\n\tmymac\n\tdc.w\t$3333\n";
    let root = dir.path().join("fixture.asm");
    std::fs::write(&root, src).unwrap();
    let asm = sigil_frontend_as::assemble_root_located_warned(&root, &sigil_frontend_as::Options::default())
        .unwrap_or_else(|_| panic!("the fixture must assemble"));
    let (chain, sections) =
        ChainSources::join(emp_index(dir.path(), &["a.emp", "b.emp"]), asm.sources, asm.module.sections, Vec::new())
            .unwrap();
    let frags: Vec<&Fragment> = sections.iter().flat_map(|s: &Section| s.fragments.iter()).collect();
    let call_line = line_of(src, "\tmymac\n");
    let body_line = line_of(src, "$1111");
    assert_ne!(call_line, body_line);
    let mut in_macro = 0;
    for f in frags {
        let span = frag_span(f);
        if !span.source.is_expansion() {
            continue;
        }
        in_macro += 1;
        let pos = chain.outermost(span).expect("a macro body locates through its call");
        assert_eq!(pos.line, call_line, "expansion bytes belong to the invocation line");
        assert!(chain.file(pos.file).path.ends_with("fixture.asm"), "{:?}", chain.file(pos.file));
    }
    assert_eq!(in_macro, 2, "both body lines of the one expansion were checked");
    // A plain AS line (outside the macro) locates to itself, in the same file.
    let plain = sections
        .iter()
        .flat_map(|s| s.fragments.iter())
        .map(frag_span)
        .find(|s| !s.source.is_expansion())
        .expect("fixture has plain lines");
    let pos = chain.outermost(plain).unwrap();
    assert!(pos.line == line_of(src, "$0000") || pos.line == line_of(src, "$3333"));
}

#[test]
fn locator_resolves_plain_emp_line() {
    let dir = tempfile::tempdir().unwrap();
    let chain = ChainSources::join(
        emp_index(dir.path(), &["first.emp", "second.emp"]),
        sigil_span::SourceMap::new(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
    .0;
    let at = EMP_TEXT.find("proc").unwrap() as u32;
    let span = Span { source: SourceId(1), start: at, end: at + 4 };
    let pos = chain.outermost(span).unwrap();
    assert_eq!(pos, SourcePos { file: 1, line: line_of(EMP_TEXT, "proc") });
    assert!(chain.file(pos.file).path.ends_with("second.emp"));
    // The no-source id answers None, never a guess.
    assert_eq!(chain.outermost(Span { source: SourceId(u32::MAX), start: 0, end: 0 }), None);
}
