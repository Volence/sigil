//! A1/A2 arc §3: the comptime emitted-span primitive `span(ProcName)`.
//! Covers both adoption shapes — the DacSampleTable emitted-span guard (target
//! 1) and the vol-env id-list-derived counts + revived id/ptr guard (target 2)
//! — plus the pure-data scope wall and the missing-proc error.
use sigil_frontend_emp::eval::eval_all_pub_consts;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_span::Level;

/// Lower a whole module and return its Error-level diagnostic messages.
fn errs(src: &str) -> Vec<String> {
    let (file, perrs) = parse_str(src);
    if perrs.iter().any(|d| d.level == Level::Error) {
        return perrs
            .iter()
            .filter(|d| d.level == Level::Error)
            .map(|d| format!("PARSE: {}", d.message))
            .collect();
    }
    let (_m, ds) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    ds.iter().filter(|d| d.level == Level::Error).map(|d| d.message.clone()).collect()
}

/// Evaluate a module's pub consts, asserting no error, and return the map.
fn pub_consts(src: &str) -> Vec<(String, i64)> {
    let (file, perrs) = parse_str(src);
    assert!(!perrs.iter().any(|d| d.level == Level::Error), "parse: {perrs:?}");
    let (vals, ds) = eval_all_pub_consts(&file, None, &[]);
    assert!(!ds.iter().any(|d| d.level == Level::Error), "eval: {ds:?}");
    vals
}

// --- Target 1 shape: span() folds an emitted byte span for an ensure ---------

// A pure-data proc of ten 9-byte descriptors spans 90 bytes; the ensure that
// checks the measured span against the declared count × stride passes clean.
#[test]
fn span_measures_emitted_bytes_and_ensure_passes() {
    let body = (0..10)
        .map(|_| "        dc.b 0,0,0\n        dc.w 0,0,0")
        .collect::<Vec<_>>()
        .join("\n");
    let src = format!(
        "module m\n\
         section s (cpu: z80, vma: $8000) {{\n\
         \x20   proc T () clobbers() {{\n{body}\n    }}\n}}\n\
         ensure(span(T) == 90, \"drift\")\n"
    );
    let e = errs(&src);
    assert!(e.is_empty(), "expected clean (span == 90), got {e:?}");
}

// The DACSampleTable-shaped guard fires when the body emits the WRONG NUMBER of
// descriptors: nine descriptors span 81, so `span(T) == 90` fails with its
// message — the drift the old `10*9` hand literal could not catch.
#[test]
fn span_doctored_descriptor_count_fires_the_ensure() {
    let body = (0..9)
        .map(|_| "        dc.b 0,0,0\n        dc.w 0,0,0")
        .collect::<Vec<_>>()
        .join("\n");
    let src = format!(
        "module m\n\
         section s (cpu: z80, vma: $8000) {{\n\
         \x20   proc T () clobbers() {{\n{body}\n    }}\n}}\n\
         ensure(span(T) == 90, \"DacSampleTable emitted span drift\")\n"
    );
    let e = errs(&src);
    assert!(e.iter().any(|m| m.contains("emitted span drift")), "expected the ensure to fire, got {e:?}");
}

// span() over a `dc.w Label` table counts 2 bytes per link-deferred cell — the
// value need not resolve for the span to fold (the vol-env ptr-table shape).
#[test]
fn span_counts_link_deferred_word_cells() {
    let src = "module m\n\
         section s (cpu: z80, vma: $8000) {\n\
         \x20   proc Ptrs () clobbers() {\n        dc.w A, B, C\n    }\n\
         \x20   proc A () clobbers() { dc.b 0 }\n\
         \x20   proc B () clobbers() { dc.b 0 }\n\
         \x20   proc C () clobbers() { dc.b 0 }\n}\n\
         ensure(span(Ptrs) == 6, \"ptr span\")\n";
    let e = errs(src);
    assert!(e.is_empty(), "expected clean (3 word cells = 6 bytes), got {e:?}");
}

// --- Target 2 shape: pub consts derived from the id-list span ----------------

// The vol-env count derives from the id-list emitted span (one `db` per env),
// and the revived id/ptr guard cross-checks the pointer table. Clean + the pub
// const folds to the id count.
#[test]
fn span_derives_pub_const_count_and_ptr_guard_passes() {
    let src = "module m\n\
         section s (cpu: z80, vma: $8000) {\n\
         \x20   proc Ids () clobbers() { dc.b $01, $02, $03 }\n\
         \x20   proc Ptrs () clobbers() { dc.w E01, E02, E03 }\n\
         \x20   proc E01 () clobbers() { dc.b 0 }\n\
         \x20   proc E02 () clobbers() { dc.b 0 }\n\
         \x20   proc E03 () clobbers() { dc.b 0 }\n}\n\
         pub const COUNT = span(Ids)\n\
         ensure(span(Ptrs) == COUNT * 2, \"id/ptr desync\")\n";
    assert!(errs(src).is_empty(), "expected clean, got {:?}", errs(src));
    let c = pub_consts(src);
    assert_eq!(c.iter().find(|(n, _)| n == "COUNT").map(|(_, v)| *v), Some(3));
}

// An id-list of 3 against a ptr-table of only 2 entries fails the revived guard
// (the deleted AS `<> COUNT*2 / error` check, now build-enforced).
#[test]
fn span_doctored_ptr_table_fires_the_revived_guard() {
    let src = "module m\n\
         section s (cpu: z80, vma: $8000) {\n\
         \x20   proc Ids () clobbers() { dc.b $01, $02, $03 }\n\
         \x20   proc Ptrs () clobbers() { dc.w E01, E02 }\n\
         \x20   proc E01 () clobbers() { dc.b 0 }\n\
         \x20   proc E02 () clobbers() { dc.b 0 }\n}\n\
         pub const COUNT = span(Ids)\n\
         ensure(span(Ptrs) == COUNT * 2, \"id/ptr desync\")\n";
    let e = errs(src);
    assert!(e.iter().any(|m| m.contains("id/ptr desync")), "expected the guard to fire, got {e:?}");
}

// --- Scope wall + name errors ------------------------------------------------

// span() over a body holding an INSTRUCTION is the pure-data scope wall: a code
// proc's length is a link-time fact, out of the demand.
#[test]
fn span_on_a_code_body_is_not_data() {
    let src = "module m\n\
         section s (cpu: z80, vma: $0) {\n\
         \x20   proc P () clobbers() { nop }\n}\n\
         ensure(span(P) == 0, \"x\")\n";
    let e = errs(src);
    assert!(e.iter().any(|m| m.contains("[span.not-data]")), "got {e:?}");
}

// A name with no proc in the module is a loud error, never a silent 0.
#[test]
fn span_unknown_proc_errors() {
    let src = "module m\n\
         section s (cpu: z80, vma: $0) {\n\
         \x20   proc P () clobbers() { dc.b 0 }\n}\n\
         ensure(span(Nope) == 1, \"x\")\n";
    let e = errs(src);
    assert!(e.iter().any(|m| m.contains("no proc named `Nope`")), "got {e:?}");
}
