//! `assert` / `raise_error` DESUGAR (diagnostics construct Task 3): the
//! byte-level integration test. The construct's synthesized expansion must
//! reproduce the hand-written transliterations in `aeon/engine/objects/rings.emp`
//! and `core.emp` EXACTLY, so the golden reference is obtained by ASSEMBLING the
//! transliteration block through the same lower→link path (NOT hand-computed
//! instruction bytes): each test lowers a transliteration proc AND a one-line
//! construct proc into the SAME section, links both against a shared stub symbol
//! table (the `MDDBG__*` handlers + `Object_RAM*` externs), and asserts their
//! proc byte ranges are identical.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Module, Section, SymbolTable, SymbolValue};
use sigil_span::Level;

/// Lower `src` with `DEBUG` set to `debug` via `-D`. Asserts a clean parse and
/// returns the module + lowering diagnostic messages.
fn lower_with_debug(src: &str, debug: i128) -> (Module, Vec<String>) {
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let (module, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), debug)],
        },
    );
    (module, diags.into_iter().map(|d| d.message).collect())
}

/// A stub symbol table defining the handler entry points + the `Object_RAM`
/// externs at arbitrary fixed addresses, so both the transliteration and the
/// construct link (they reference the SAME symbols, so the comparison is valid).
fn stubs() -> SymbolTable {
    let mut t = SymbolTable::new();
    for (name, addr) in [
        ("MDDBG__ErrorHandler", 0x0010_0000i64),
        ("MDDBG__ErrorHandler_PagesController", 0x0010_0100),
        ("Object_RAM", 0x00FF_B000),
        ("Object_RAM_End", 0x00FF_D5FF),
        ("NUM_DYNAMIC", 0x60),
    ] {
        t.define(name, SymbolValue::Int(addr));
    }
    t
}

fn section<'a>(m: &'a Module, name: &str) -> &'a Section {
    m.sections.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("no section `{name}`"))
}

fn label_offset(s: &Section, name: &str) -> usize {
    s.labels
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no label `{name}` (have {:?})", s.labels.iter().map(|l| &l.name).collect::<Vec<_>>()))
        .offset as usize
}

fn linked_bytes(m: &Module, sec: &str) -> Vec<u8> {
    let st = stubs();
    let resolved = sigil_link::resolve_layout(&m.sections, &st, true).expect("resolve_layout");
    let linked = sigil_link::link(&resolved, &st).expect("link");
    linked.section(sec).expect("linked section").bytes.clone()
}

/// The bytes of proc `name`: from its exported label to the next label at a
/// higher offset in the section (or section end).
fn proc_bytes(m: &Module, sec: &str, name: &str) -> Vec<u8> {
    let s = section(m, sec);
    let start = label_offset(s, name);
    let bytes = linked_bytes(m, sec);
    // End = the smallest label offset strictly greater than start, else EOF.
    let end = s
        .labels
        .iter()
        .map(|l| l.offset as usize)
        .filter(|&o| o > start)
        .min()
        .unwrap_or(bytes.len());
    bytes[start..end].to_vec()
}

// ---------------------------------------------------------------------------
// 1. DEBUG=0 (and undefined) → ZERO bytes.
// ---------------------------------------------------------------------------

#[test]
fn assert_debug_off_emits_nothing() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        assert.b d4, eq, #0
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 0);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // The proc is just `rts` (2 bytes) — the assert expanded to nothing.
    let bytes = proc_bytes(&m, "s", "p");
    assert_eq!(bytes, vec![0x4E, 0x75], "DEBUG=0: assert emits zero bytes, proc is `rts`");
}

#[test]
fn assert_debug_undefined_is_explicit_error() {
    // Spec §5: `DEBUG` undefined → a loud, explicit-shape error (NOT a silent
    // zero-emit). Lower WITHOUT the -D define.
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        assert.b d4, eq, #0
        rts
    }
}
";
    let (file, _perrs) = parse_str(src);
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions { initial_cpu: Cpu::M68000, include_root: None, embed_base: None, defines: vec![] },
    );
    let msg = diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains("DEBUG") && msg.to_lowercase().contains("defined"),
        "undefined DEBUG must be an explicit error naming DEBUG: {msg}"
    );
}

// ---------------------------------------------------------------------------
// 2. Full-expansion byte equality vs the assembled transliteration.
// ---------------------------------------------------------------------------

/// rings.emp's `assert.b d4, eq, #0` — cmp form, `.b` arg push, EVEN-parity flag
/// (`$A0,$00`). Golden = the hand-written transliteration, assembled.
#[test]
fn rings_byte_form_matches_transliteration() {
    // The hand-written `golden` claims `preserves(sr.mask)`, its `construct` twin
    // bare `preserves(sr)` — and both emit identical BYTES (contracts emit
    // nothing). This is the noreturn-tail model, not an inconsistency: the
    // desugar's inner `move.w sr, -(sp)` frame push is `AssertDesugar`-authored on
    // a divergent rail, so the CCR-bracket advisory clears `construct`'s CCR half;
    // the SAME instruction hand-written in `golden` is `User`-authored and reads
    // as a nested bracket save, so a linear walk can prove only the mask half.
    let src = "\
module m
section s (cpu: m68000) {
    proc golden () clobbers(d4) preserves(sr.mask) {
        move.w  sr, -(sp)
        cmp.b   #0, d4
        beq.w   .assert_ok
    .raise:
        pea     .raise(pc)
        move.w  sr, -(sp)
        subq.w  #2, sp
        move.b  d4, 1(sp)
        jsr     (MDDBG__ErrorHandler).l
        dc.b    \"Assertion failed:\", $E0, $EC, \"> assert.b \", $E8, \"d4,\", $EC, \"eq\", $E8, \",#0\", $E0, $EA, \"Got: \"
        dc.b    $80, $00
        dc.b    $A0, $00
        jmp     (MDDBG__ErrorHandler_PagesController).l
    .assert_ok:
        move.w  (sp)+, sr
        rts
    }
    proc construct () clobbers(d4) preserves(sr) {
        assert.b d4, eq, #0
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 1);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    // Compare everything up to the trailing `rts` (2 bytes) that both share.
    let golden = proc_bytes(&m, "s", "golden");
    let construct = proc_bytes(&m, "s", "construct");
    assert_eq!(construct, golden, "assert.b construct must reproduce the rings transliteration bytes");
}

/// core.emp's `assert.l a0, hs, #Object_RAM` — `.l` cmp form with a symbol
/// immediate, ODD-parity flag (`$20`, no pad). Golden = the transliteration.
#[test]
fn core_long_form_matches_transliteration() {
    let src = "\
module m
section s (cpu: m68000) {
    // golden = sr.mask, construct = bare sr (byte-identical): the asymmetry is the
    // model, see the rings-form fixture above.
    proc golden () clobbers() preserves(sr.mask) {
        move.w  sr, -(sp)
        cmp.l   #Object_RAM, a0
        bhs.w   .skip1
    .a1raise:
        pea     .a1raise(pc)
        move.w  sr, -(sp)
        move.l  a0, -(sp)
        jsr     (MDDBG__ErrorHandler).l
        dc.b    \"Assertion failed:\", $E0, $EC, \"> assert.l \", $E8, \"a0,\", $EC, \"hs\", $E8, \",#Object_RAM\", $E0, $EA, \"Got: \"
        dc.b    $83, $00
        dc.b    $20
        jmp     (MDDBG__ErrorHandler_PagesController).l
    .skip1:
        move.w  (sp)+, sr
        rts
    }
    proc construct () clobbers() preserves(sr) {
        assert.l a0, hs, #Object_RAM
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 1);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let golden = proc_bytes(&m, "s", "golden");
    let construct = proc_bytes(&m, "s", "construct");
    assert_eq!(construct, golden, "assert.l construct must reproduce the core transliteration bytes");
}

/// core.emp's `assert.w d7, lo, #NUM_DYNAMIC` — `.w` cmp form, ODD-parity flag.
#[test]
fn core_word_form_matches_transliteration() {
    let src = "\
module m
section s (cpu: m68000) {
    // golden = sr.mask, construct = bare sr (byte-identical): the asymmetry is the
    // model, see the rings-form fixture above.
    proc golden () clobbers(d7) preserves(sr.mask) {
        move.w  sr, -(sp)
        cmp.w   #NUM_DYNAMIC, d7
        blo.w   .skip3
    .a3raise:
        pea     .a3raise(pc)
        move.w  sr, -(sp)
        move.w  d7, -(sp)
        jsr     (MDDBG__ErrorHandler).l
        dc.b    \"Assertion failed:\", $E0, $EC, \"> assert.w \", $E8, \"d7,\", $EC, \"lo\", $E8, \",#NUM_DYNAMIC\", $E0, $EA, \"Got: \"
        dc.b    $81, $00
        dc.b    $A0, $00
        jmp     (MDDBG__ErrorHandler_PagesController).l
    .skip3:
        move.w  (sp)+, sr
        rts
    }
    proc construct () clobbers(d7) preserves(sr) {
        assert.w d7, lo, #NUM_DYNAMIC
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 1);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let golden = proc_bytes(&m, "s", "golden");
    let construct = proc_bytes(&m, "s", "construct");
    assert_eq!(construct, golden, "assert.w construct must reproduce the core transliteration bytes");
}

// ---------------------------------------------------------------------------
// 3. tst form emits `tst.<w>`, not `cmp`.
// ---------------------------------------------------------------------------

#[test]
fn tst_form_uses_tst_not_cmp() {
    let src = "\
module m
section s (cpu: m68000) {
    // golden = sr.mask, construct = bare sr (byte-identical): the asymmetry is the
    // model, see the rings-form fixture above.
    proc golden () clobbers(d1) preserves(sr.mask) {
        move.w  sr, -(sp)
        tst.w   d1
        beq.w   .skip
    .raise:
        pea     .raise(pc)
        move.w  sr, -(sp)
        move.w  d1, -(sp)
        jsr     (MDDBG__ErrorHandler).l
        dc.b    \"Assertion failed:\", $E0, $EC, \"> assert.w \", $E8, \"d1,\", $EC, \"eq\", $E0, $EA, \"Got: \"
        dc.b    $81, $00
        dc.b    $A0, $00
        jmp     (MDDBG__ErrorHandler_PagesController).l
    .skip:
        move.w  (sp)+, sr
        rts
    }
    proc construct () clobbers(d1) preserves(sr) {
        assert.w d1, eq
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 1);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let golden = proc_bytes(&m, "s", "golden");
    let construct = proc_bytes(&m, "s", "construct");
    assert_eq!(construct, golden, "tst form must emit tst.w, not cmp, bytes must match the tst transliteration");
}

// ---------------------------------------------------------------------------
// 4. Hygiene: two asserts in one proc do not collide.
// ---------------------------------------------------------------------------

#[test]
fn two_asserts_in_one_proc_do_not_collide() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4, d7) preserves(sr) {
        assert.b d4, eq, #0
        assert.w d7, lo, #NUM_DYNAMIC
        rts
    }
}
";
    let (m, msgs) = lower_with_debug(src, 1);
    // A colliding `.skip`/`.raise` label would surface as a duplicate-symbol /
    // link error; a clean lower + link is the proof of fresh-per-expansion labels.
    assert!(msgs.is_empty(), "two asserts must lower cleanly (fresh hygienic labels): {msgs:?}");
    let _ = linked_bytes(&m, "s"); // link must succeed (no duplicate label symbol).
}

// ---------------------------------------------------------------------------
// 5. raise_error: NO DEBUG gate, NO cmp/branch/CCR wrapper (steps 4-10 only).
// ---------------------------------------------------------------------------

#[test]
fn raise_error_has_no_gate_and_no_compare_wrapper() {
    // path_swap.asm shape. The expansion is JUST the RaiseError tail — no
    // `move.w sr,-(sp)` CCR-save/`cmp`/`b<cond>` prefix and no `.skip` restore.
    // Emitted UNCONDITIONALLY (DEBUG=0 still emits it).
    let src = "\
module m
section s (cpu: m68000) {
    proc golden () clobbers(d0) {
    .raise:
        pea     .raise(pc)
        move.w  sr, -(sp)
        subq.w  #2, sp
        move.b  d0, 1(sp)
        jsr     (MDDBG__ErrorHandler).l
        dc.b    \"Bad path swap!\", $E0, \"Got: \", $80, $00
        dc.b    $A0, $00
        jmp     (MDDBG__ErrorHandler_PagesController).l
        rts
    }
    proc construct () clobbers(d0) {
        raise_error \"Bad path swap!%<endl>Got: %<.b d0>\"
        rts
    }
}
";
    // DEBUG=0 — raise_error must STILL emit (unconditional).
    let (m, msgs) = lower_with_debug(src, 0);
    assert!(msgs.is_empty(), "clean lower: {msgs:?}");
    let golden = proc_bytes(&m, "s", "golden");
    let construct = proc_bytes(&m, "s", "construct");
    assert_eq!(construct, golden, "raise_error must be the bare tail (no gate, no compare wrapper)");
}

// ---------------------------------------------------------------------------
// 6. §5 steering errors at the desugar/eval stage (not just the parser).
// ---------------------------------------------------------------------------

/// `src` must be a register in v1 — a memory operand is a steering error naming
/// the fix ("move to a register first"). This is the EVAL-stage check (the
/// parser accepts the operand shape; the register limit is enforced here).
#[test]
fn assert_src_must_be_a_register() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers() preserves(sr) {
        assert.b Ring_Add_Dropped, eq, #0
        rts
    }
}
";
    let (file, _perrs) = parse_str(src);
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), 1)],
        },
    );
    let msg = diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains("register") && msg.to_lowercase().contains("move"),
        "non-register src must steer to a register: {msg}"
    );
}

/// A `raise_error` whose arg is a memory/EA operand is a §5 steering error.
#[test]
fn raise_error_memory_arg_is_a_steering_error() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers() {
        raise_error \"bad %<.w (a0)>\"
        rts
    }
}
";
    let (file, _perrs) = parse_str(src);
    let (_m, diags) = lower_module(
        &file,
        &LowerOptions {
            initial_cpu: Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![("DEBUG".into(), 1)],
        },
    );
    let msg = diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n");
    assert!(
        msg.contains("register or immediate") || msg.to_lowercase().contains("memory"),
        "memory raise_error arg must steer to register/immediate: {msg}"
    );
}



// ---------------------------------------------------------------------------
// 5. AUTHORSHIP — the emission-site obligation behind the sr-lint exemption.
// ---------------------------------------------------------------------------
//
// `[proc.sr-undeclared]` exempts `AssertDesugar`-authored SR writes because the
// desugar's push/pop balance is the COMPILER's obligation. These tests ARE that
// obligation's pin, at the emission site: if the desugar ever grows an
// unbalanced path — or stops stamping its items — this fails, not a corpus
// measurement.

use sigil_frontend_emp::value::{CodeItem, CodeOperand, ItemAuthor, Reg};

/// The lowered CodeBuf of the one proc `p` in `src` (DEBUG defined to `debug`).
fn proc_items(src: &str, debug: i128) -> Vec<CodeItem> {
    use sigil_frontend_emp::ast::Item;
    let (file, perrs) = parse_str(src);
    assert!(perrs.iter().all(|d| d.level != Level::Error), "parse: {perrs:?}");
    let proc = file
        .items
        .iter()
        .find_map(|it| match it {
            Item::Section(s) => s.items.iter().find_map(|si| match si {
                Item::Proc(d) if d.name == "p" => Some(d),
                _ => None,
            }),
            Item::Proc(d) if d.name == "p" => Some(d),
            _ => None,
        })
        .expect("proc `p`");
    let (buf, _diags, _counter) = sigil_frontend_emp::eval::eval_proc_body(
        &file,
        &proc.name,
        &proc.params,
        &proc.body,
        proc.span,
        0,
        Cpu::M68000,
        &[("DEBUG".into(), debug)],
        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
    );
    buf.expect("Code").items
}

/// Every instruction the `assert` desugar emits is `AssertDesugar`-authored,
/// and its SR traffic is balanced BY CONSTRUCTION: the first instruction is the
/// `move.w sr, -(sp)` save, the last is the `move.w (sp)+, sr` restore, and
/// the restore is the expansion's ONLY SR-destination write.
#[test]
fn the_assert_expansion_is_desugar_authored_and_sr_balanced() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        assert.b d4, eq, #0
    }
}
";
    let items = proc_items(src, 1);
    let instrs: Vec<&CodeItem> =
        items.iter().filter(|i| matches!(i, CodeItem::Instr { .. })).collect();
    assert!(!instrs.is_empty(), "the DEBUG expansion emits instructions");
    for it in &instrs {
        let CodeItem::Instr { author, mnemonic, .. } = it else { unreachable!() };
        assert_eq!(
            *author,
            ItemAuthor::AssertDesugar,
            "every expansion instruction is the desugar's (`{mnemonic}` is not)"
        );
    }
    let CodeItem::Instr { mnemonic, ops, .. } = instrs[0] else { unreachable!() };
    assert_eq!(mnemonic, "move", "the expansion opens with the CCR save");
    assert_eq!(
        ops.as_slice(),
        &[CodeOperand::Sr, CodeOperand::PreDec(Reg::A7)],
        "first instruction is `move.w sr, -(sp)`"
    );
    let CodeItem::Instr { ops: last_ops, .. } = instrs[instrs.len() - 1] else { unreachable!() };
    assert_eq!(
        last_ops.as_slice(),
        &[CodeOperand::PostInc(Reg::A7), CodeOperand::Sr],
        "last instruction is the `move.w (sp)+, sr` restore"
    );
    let sr_dest_writes = instrs
        .iter()
        .filter(|i| {
            let CodeItem::Instr { ops, .. } = i else { return false };
            matches!(ops.last(), Some(CodeOperand::Sr))
        })
        .count();
    assert_eq!(sr_dest_writes, 1, "the restore is the expansion's only SR write");
}

/// The `raise_error` rail is `AssertDesugar`-authored end to end — its terminal
/// `jmp (pages).l` is the compiler's authored divergent transfer (spec §1(b)),
/// so the noreturn-tail consumers treat it as a divergent terminal, and a
/// hand-written `jmp` to the same blob stays a plain `TailOut`.
#[test]
fn the_raise_error_rail_is_desugar_authored_with_a_divergent_jmp() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers() {
        raise_error \"bad\"
    }
}
";
    let items = proc_items(src, 1);
    let instrs: Vec<&CodeItem> =
        items.iter().filter(|i| matches!(i, CodeItem::Instr { .. })).collect();
    assert!(!instrs.is_empty(), "the raise rail emits instructions");
    for it in &instrs {
        let CodeItem::Instr { author, mnemonic, .. } = it else { unreachable!() };
        assert_eq!(*author, ItemAuthor::AssertDesugar, "`{mnemonic}` must be the desugar's");
    }
    let CodeItem::Instr { mnemonic, author, .. } = instrs[instrs.len() - 1] else { unreachable!() };
    assert_eq!(mnemonic, "jmp", "the rail ends in the divergent `jmp (pages)`");
    assert_eq!(*author, ItemAuthor::AssertDesugar, "the terminal transfer is authored");
}

/// The `raise_exception` rail (the exception-vector shape, no frame push) is
/// likewise `AssertDesugar`-authored end to end, terminal `jmp` included.
#[test]
fn the_raise_exception_rail_is_desugar_authored_with_a_divergent_jmp() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers() {
        raise_exception \"BUS ERROR\"
    }
}
";
    let items = proc_items(src, 1);
    let instrs: Vec<&CodeItem> =
        items.iter().filter(|i| matches!(i, CodeItem::Instr { .. })).collect();
    assert!(!instrs.is_empty(), "the raise_exception rail emits instructions");
    for it in &instrs {
        let CodeItem::Instr { author, mnemonic, .. } = it else { unreachable!() };
        assert_eq!(*author, ItemAuthor::AssertDesugar, "`{mnemonic}` must be the desugar's");
    }
    let CodeItem::Instr { mnemonic, author, .. } = instrs[instrs.len() - 1] else { unreachable!() };
    assert_eq!(mnemonic, "jmp", "the rail ends in the divergent `jmp (pages)`");
    assert_eq!(*author, ItemAuthor::AssertDesugar, "the terminal transfer is authored");
}

/// The user's own lines around an `assert` stay `User`-authored — the stamp is
/// scoped to the expansion, so the exemption can never leak onto hand-written
/// code before or after it.
#[test]
fn lines_around_an_assert_stay_user_authored() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        moveq   #0, d4
        assert.b d4, eq, #0
        rts
    }
}
";
    let items = proc_items(src, 1);
    let user_mnemonics: Vec<&str> = items
        .iter()
        .filter_map(|i| match i {
            CodeItem::Instr { mnemonic, author: ItemAuthor::User, .. } => {
                Some(mnemonic.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        user_mnemonics,
        ["moveq", "rts"],
        "exactly the hand-written lines are User-authored"
    );
}

/// The STRUCTURAL INVARIANT the closure's authorship exclusion rests on. That
/// exclusion (corpus_contracts: an `ItemAuthor::AssertDesugar` instruction
/// contributes no callee edge) is keyed on authorship, justified by authorship ∧
/// DIVERGENCE. Authorship is enforced at the emission-site stamp; this pins the
/// divergence half over the actual lowered items, so a future desugar that
/// emitted a RETURNING helper call on a surviving path would FAIL here instead of
/// being silently dropped from the closure — a case the counterfactual censuses
/// cannot catch (an AssertDesugar call to a real-proc node lands in NEITHER
/// `abs_call_edges` NOR `authored_rail_holes`). Two properties over all three
/// desugars:
///   (a) every AssertDesugar-authored call/tail targets exactly a rail boundary
///       ({MDDBG__ErrorHandler, MDDBG__ErrorHandler_PagesController}) — never a
///       returning helper into a real proc; and
///   (b) for `assert` (the only shape with a surviving path), the `$skip` label
///       is reached ONLY by the pass branch: every rail call/tail precedes it, and
///       the instruction that would fall INTO it is the divergent `jmp (pages).l`
///       — so the surviving suffix at/after `$skip` carries no rail call/tail.
#[test]
fn the_assert_desugar_rails_diverge_the_exclusions_structural_pin() {
    const CALL_TAIL: [&str; 6] = ["jsr", "bsr", "jbsr", "jmp", "bra", "jbra"];
    const HANDLER: &str = "MDDBG__ErrorHandler";
    const PAGES: &str = "MDDBG__ErrorHandler_PagesController";

    fn abs_target(ops: &[CodeOperand]) -> Option<&str> {
        ops.iter().find_map(|o| match o {
            CodeOperand::AbsSym { target, .. } => Some(target.as_str()),
            _ => None,
        })
    }

    // (a) across all three desugars.
    let shapes = [
        ("assert.b d4, eq, #0", "clobbers(d4)"),
        ("raise_error \"bad\"", "clobbers()"),
        ("raise_exception \"BUS ERROR\"", "clobbers()"),
    ];
    let mut rail_calls = 0usize;
    for (stmt, clob) in shapes {
        let src = format!(
            "module m\nsection s (cpu: m68000) {{\n    proc p () {clob} {{\n        {stmt}\n    }}\n}}\n"
        );
        for it in proc_items(&src, 1) {
            let CodeItem::Instr { mnemonic, ops, author, .. } = &it else { continue };
            if !CALL_TAIL.contains(&mnemonic.as_str()) || *author != ItemAuthor::AssertDesugar {
                continue;
            }
            let target = abs_target(ops).unwrap_or_else(|| {
                panic!(
                    "`{stmt}`: an AssertDesugar `{mnemonic}` names no abs boundary \
                     (a returning register-indirect helper?): ops={ops:?}"
                )
            });
            assert!(
                target == HANDLER || target == PAGES,
                "`{stmt}`: an AssertDesugar `{mnemonic}` targets `{target}`, not a rail \
                 boundary, a returning helper call into a real proc would be silently \
                 dropped from the closure"
            );
            rail_calls += 1;
        }
    }
    assert!(rail_calls >= 3, "non-vacuity: each rail emits its call+terminal, saw only {rail_calls}");

    // (b) assert only: the surviving path is bounded by the divergent terminal.
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        assert.b d4, eq, #0
    }
}
";
    let items = proc_items(src, 1);
    let skip_idx = items
        .iter()
        .position(|it| matches!(it, CodeItem::Label { name, .. } if name.contains("$skip")))
        .expect("the assert expansion mints a `$skip` label");
    for (i, it) in items.iter().enumerate() {
        let CodeItem::Instr { mnemonic, author, .. } = it else { continue };
        if CALL_TAIL.contains(&mnemonic.as_str()) && *author == ItemAuthor::AssertDesugar {
            assert!(
                i < skip_idx,
                "an AssertDesugar `{mnemonic}` at index {i} is at/after `$skip` (index \
                 {skip_idx}), a rail call/tail leaked onto the surviving path"
            );
        }
    }
    let before = items[..skip_idx]
        .iter()
        .rev()
        .find(|it| matches!(it, CodeItem::Instr { .. }))
        .expect("an instruction precedes `$skip`");
    let CodeItem::Instr { mnemonic, ops, author, .. } = before else { unreachable!() };
    assert_eq!(
        mnemonic, "jmp",
        "the instruction reaching `$skip` by fall-through must be the divergent terminal, \
         so `$skip` is entered only by the pass branch"
    );
    assert_eq!(*author, ItemAuthor::AssertDesugar, "the divergent terminal is authored");
    assert_eq!(abs_target(ops), Some(PAGES), "the divergent terminal is `jmp (pages).l`");
}

/// The consumer-side effect of the emission-site obligation: an `assert` in a
/// `clobbers()` proc does not fire `[proc.sr-undeclared]` — the desugar's
/// restore is `AssertDesugar`-authored and its balance is proven above, so the
/// lint has nothing to charge the containing proc with.
#[test]
fn an_assert_does_not_fire_sr_undeclared_on_its_proc() {
    let src = "\
module m
section s (cpu: m68000) {
    proc p () clobbers(d4) {
        assert.b d4, eq, #0
        rts
    }
}
";
    let (_m, msgs) = lower_with_debug(src, 1);
    assert!(
        !msgs.iter().any(|m| m.contains("[proc.sr-undeclared]")),
        "the desugar's SR restore must not be charged to `p`: {msgs:?}"
    );
}
